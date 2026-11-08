//! Executor runtime/control plane: queue admission, commands and worker ownership.

use super::{
    ArchiveJobRequest, CancelArbitration, CancellationToken, CommitRecoveryBatchResult,
    CommitRecoveryDecision, CommitRecoveryDirectory, CommitRecoveryFacts,
    CommitRecoveryFactsProvider, DEFAULT_QUEUE_CAPACITY, ExecutorConfig, ExecutorError,
    ExecutorEvent, ExecutorNetworkConfig, JobExecution, JobExecutionFactory, JobExecutionResult,
    JobPersistence, JobSnapshot, ProductionExecutionFactory, RUNNER_CAPACITY,
    SessionExecutionBoundary, SessionRunOutcome, StartupRecoveryOutcome, StartupRecoveryReport,
    StartupRecoveryStatus, StorageJobPersistence, SubmitResult, UnconfiguredExecutionFactory,
    resolve_session_run,
};
#[cfg(target_os = "linux")]
use super::{
    LinuxExecutionContainment, ProductionCoordinator, containment_record_root,
    recover_internal_v1_on_runner, recover_v2_on_runner, recover_writer_domain_set,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use xarchive_core::JobState;
use xarchive_storage::{ArchiveRecoveryDispatch, ArchiveService, Database, FileStore};

/// Runtime-owned executor configuration and lifecycle boundary for the R1 worker.
/// Production runner resources are created from immutable Job execution specs.
pub struct ExecutorRuntime {
    pub(super) executor: JobExecutor,
    database_path: PathBuf,
    config: ExecutorConfig,
    #[cfg(target_os = "linux")]
    pub(super) containment: Option<LinuxExecutionContainment>,
}

impl ExecutorRuntime {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        let database_path = database_path.into();
        let archive_root = database_path
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("."))
            .to_owned();
        let config = ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root,
            staging_root: database_path
                .parent()
                .map(|path| path.join("_staging"))
                .unwrap_or_else(|| PathBuf::from("_staging")),
            database_path: database_path.clone(),
            sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok(),
            gallery_dl_program: std::env::var("XARCHIVE_GALLERY_DL_PROGRAM").ok(),
            sidecar_args: std::env::var("XARCHIVE_SIDECAR_ARGS")
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
            aria2_program: std::env::var("XARCHIVE_ARIA2_PROGRAM").ok(),
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        };
        Self::with_config(config)
    }

    pub fn with_config(config: ExecutorConfig) -> Self {
        let factory = ProductionExecutionFactory::new(config.clone());
        Self::with_production_factory(
            config,
            factory,
            #[cfg(target_os = "linux")]
            None,
        )
    }

    // Explicit Linux runtime provisioning. No environment inference or
    // uncontained fallback is performed. Containment does not enroll jobs in
    // production v2; ADR-015 keeps that contract fail-closed until C1 passes.
    /// CROSS_PLATFORM_CHANGE_REQUIRED: runtime launch configuration boundary.
    #[cfg(target_os = "linux")]
    pub fn with_linux_containment(
        config: ExecutorConfig,
        containment: LinuxExecutionContainment,
    ) -> Self {
        let factory = ProductionExecutionFactory::new(config.clone())
            .with_linux_containment(containment.clone());
        Self::with_production_factory(config, factory, Some(containment))
    }

    pub(super) fn with_production_factory(
        config: ExecutorConfig,
        factory: ProductionExecutionFactory,
        #[cfg(target_os = "linux")] containment: Option<LinuxExecutionContainment>,
    ) -> Self {
        Self {
            executor: JobExecutor::with_capacity_and_factory(
                DEFAULT_QUEUE_CAPACITY,
                Arc::new(factory),
            ),
            database_path: config.database_path.clone(),
            config,
            #[cfg(target_os = "linux")]
            containment,
        }
    }

    pub fn service(&self) -> ArchiveApplicationService {
        // CROSS_PLATFORM_CHANGE_REQUIRED (ADR-015 clause 8): provisioned
        // containment does not imply production v2 enrollment approval.
        ArchiveApplicationService::new(&self.executor)
    }

    pub fn open_persistence(&self) -> Result<StorageJobPersistence, String> {
        StorageJobPersistence::open(&self.database_path)
    }

    /// Build an owned production session boundary for a single persisted Job.
    ///
    /// The lock records live beside the durable database, under a stable
    /// workspace identity shared by every process using this configured
    /// workspace. This does not opt the Job into v2 publication or recovery.
    #[cfg(unix)]
    pub(crate) fn production_session_boundary(
        &self,
    ) -> Result<SessionExecutionBoundary<StorageJobPersistence, ProductionCoordinator>, ExecutorError>
    {
        let persistence = StorageJobPersistence::open(&self.database_path)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let coordinator = ProductionCoordinator::open(&self.database_path)?;
        Ok(SessionExecutionBoundary::new(
            self.service(),
            persistence,
            coordinator,
            self.config.archive_root.clone(),
        )
        .with_optional_recovery_containment(
            self.containment.clone(),
            self.config.staging_root.clone(),
        ))
    }

    /// Execute one persisted Job through the owned `SessionExecutionBoundary`.
    ///
    /// Routes through `SessionExecutionBoundary::execute_on_runner`, which
    /// performs runner admission → Job/destination scope acquisition → durable
    /// claim under the guard → factory creation → runner execution →
    /// attempt-fenced terminal writes and finalization → session release →
    /// admission release. The live session is retained through finalization,
    /// so no detached result-only seam can be mistaken for lifecycle progress.
    ///
    /// Production scheduling remains fail-closed: this does not switch the
    /// legacy `execute_persisted_from_factory` path until B4/B5 acceptance
    /// gates pass. It is available for callers that opt into the owned
    /// session boundary (e.g. startup recovery via `recover_startup`).
    #[cfg(unix)]
    pub(crate) fn execute_persisted_on_runner(
        &self,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let boundary = self.production_session_boundary()?;
        let outcome = resolve_session_run(boundary.execute_on_runner(job_id.to_owned())?)?;
        match outcome {
            SessionRunOutcome::Finished(snapshot) => Ok(snapshot),
            SessionRunOutcome::Reschedulable { job_id, .. } => Err(ExecutorError::Execution {
                error_code: "EXECUTION_RESCHEDULABLE".to_owned(),
                error_message: format!("job {job_id} is reschedulable"),
                persistence_already_updated: false,
            }),
            // Defensive only: `resolve_session_run` above consumes every
            // pending owner, and `execute_session` never nests one, so no
            // live owner can be observed here. Fail closed instead of
            // routing an unresolved disposition.
            SessionRunOutcome::StartupCleanupPending { job_id, .. } => {
                Err(ExecutorError::Execution {
                    error_code: "UNRESOLVED_STARTUP_CLEANUP".to_owned(),
                    error_message: format!("unresolved startup cleanup decision for {job_id}"),
                    persistence_already_updated: false,
                })
            }
        }
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn linux_containment(&self) -> Option<LinuxExecutionContainment> {
        self.containment.clone()
    }

    pub fn aria2_program(&self) -> Option<&str> {
        self.config.aria2_program.as_deref()
    }

    pub fn gallery_dl_program(&self) -> Option<&str> {
        self.config.gallery_dl_program.as_deref()
    }

    pub fn recover_startup(
        &self,
    ) -> Result<thread::JoinHandle<StartupRecoveryReport>, ExecutorError> {
        // Linux containment is a writer-lifecycle prerequisite, not approval
        // to enroll or recover production InternalRenameV2 jobs.
        // CROSS_PLATFORM_CHANGE_REQUIRED (ADR-015 clause 8): keep production v2
        // fail-closed until the complete C1 lifecycle gate is accepted.
        self.recover_startup_with_v2(false)
    }

    // Kept parameterized for isolated recovery fixtures. Production entry
    // points always pass false until the complete C1 gate is accepted.
    pub(super) fn recover_startup_with_v2(
        &self,
        enable_v2: bool,
    ) -> Result<thread::JoinHandle<StartupRecoveryReport>, ExecutorError> {
        let database_path = self.database_path.clone();
        let archive_root = self.config.archive_root.clone();
        let staging_root = self.config.staging_root.clone();
        #[cfg(target_os = "linux")]
        let containment = self.containment.clone();
        let service = ArchiveApplicationService {
            v2_enrollment: enable_v2,
            ..self.service()
        };
        thread::Builder::new()
            .name("xarchive-startup-recovery".to_owned())
            .spawn(move || {
                let mut v2_results = Vec::new();
                let mut persistence = match StorageJobPersistence::open(&database_path) {
                    Ok(persistence) => persistence,
                    Err(error) => return StartupRecoveryReport::failed(error),
                };
                let mut candidates = match persistence.list_recovery_candidates() {
                    Ok(candidates) => candidates,
                    Err(error) => return StartupRecoveryReport::failed(error.to_string()),
                };
                #[cfg(target_os = "linux")]
                if containment.is_some() {
                    let downloading_internal_v1 = match Database::open(&database_path)
                        .and_then(|database| database.internal_v1_recovery_job_ids())
                    {
                        Ok(job_ids) => job_ids,
                        Err(error) => return StartupRecoveryReport::failed(error.to_string()),
                    };
                    for job_id in downloading_internal_v1 {
                        if candidates.iter().any(|candidate| candidate.job_id == job_id) {
                            continue;
                        }
                        let Ok(candidate) = persistence.snapshot(&job_id) else {
                            continue;
                        };
                        if candidate.state == JobState::Downloading
                            && matches!(persistence.internal_v1_attempt_phase(&job_id), Ok(Some(None)))
                        {
                            candidates.push(candidate);
                        }
                    }
                }
                #[cfg(target_os = "linux")]
                {
                    let job_ids = match Database::open(&database_path)
                        .and_then(|database| database.internal_v1_recovery_job_ids())
                    {
                        Ok(job_ids) => job_ids,
                        Err(error) => return StartupRecoveryReport::failed(error.to_string()),
                    };
                    for job_id in job_ids {
                        if candidates.iter().any(|candidate| candidate.job_id == job_id) {
                            continue;
                        }
                        match persistence.snapshot(&job_id) {
                            Ok(candidate) => candidates.push(candidate),
                            Err(error) => return StartupRecoveryReport::failed(error.to_string()),
                        }
                    }
                }
                candidates.sort_by(|left, right| left.job_id.cmp(&right.job_id));
                for candidate in candidates {
                    #[cfg(target_os = "linux")]
                    // Gated v2 validates journals only after taking scopes. In
                    // particular corrupt journals must not reach the historical
                    // unfenced failure path below.
                    #[cfg(target_os = "linux")]
                    if enable_v2 && matches!(
                        Database::open(&database_path).and_then(|database| database.archive_recovery_contract(&candidate.job_id)),
                        Ok(Some(2))
                    ) {
                        let result = recover_v2_on_runner(
                            &service, database_path.clone(), archive_root.clone(),
                            staging_root.clone(), containment.clone(),
                            candidate.job_id.clone(), candidate.tweet_id.clone(),
                        );
                        v2_results.push((candidate.job_id.clone(), result.map_err(|error| error.to_string())));
                        continue;
                    }
                    let recovery_dispatch_result = Database::open(&database_path)
                        .and_then(|database| database.startup_recovery_dispatch(&candidate.job_id));
                    #[cfg(target_os = "linux")]
                    let recovery_dispatch_result = if candidate.state == JobState::Downloading {
                        match Database::open(&database_path)
                            .and_then(|database| database.archive_recovery_contract(&candidate.job_id))
                        {
                            Ok(Some(1))
                                if matches!(
                                    persistence.internal_v1_attempt_phase(&candidate.job_id),
                                    Ok(Some(None))
                                ) =>
                            {
                                Ok(ArchiveRecoveryDispatch::InternalV1)
                            }
                            _ => recovery_dispatch_result,
                        }
                    } else {
                        recovery_dispatch_result
                    };
                    #[cfg(target_os = "linux")]
                    if candidate.state == JobState::Downloading
                        && matches!(recovery_dispatch_result, Ok(ArchiveRecoveryDispatch::InternalV1))
                        && matches!(persistence.internal_v1_attempt_phase(&candidate.job_id), Ok(Some(None)))
                    {
                        let (Some(containment), Ok(boundary_persistence), Ok(coordinator)) = (
                            containment.clone(),
                            StorageJobPersistence::open(&database_path),
                            ProductionCoordinator::open(&database_path),
                        ) else {
                            v2_results.push((candidate.job_id.clone(), Err(
                                "InternalV1 successor recovery unavailable".to_owned(),
                            )));
                            continue;
                        };
                        if !persistence.has_execution_spec(&candidate.job_id).unwrap_or(false) {
                            v2_results.push((candidate.job_id.clone(), Err(
                                "InternalV1 successor execution spec is unavailable".to_owned(),
                            )));
                            continue;
                        }
                        let boundary = SessionExecutionBoundary::new(
                            service.clone(),
                            boundary_persistence,
                            coordinator,
                            archive_root.clone(),
                        )
                        .with_recovery_containment(containment, staging_root.clone());
                        let result = boundary
                            .execute_on_runner(candidate.job_id.clone())
                            .and_then(resolve_session_run)
                            .map(|outcome| matches!(
                                outcome,
                                SessionRunOutcome::Finished(JobSnapshot { state: JobState::Complete, .. })
                            ))
                            .map_err(|error| error.to_string());
                        v2_results.push((candidate.job_id.clone(), result));
                        continue;
                    }
                    let recovery_contract = match recovery_dispatch_result {
                        Ok(contract) => contract,
                        Err(error) => {
                            #[cfg(target_os = "linux")]
                            let is_internal_v1 = Database::open(&database_path)
                                .and_then(|database| database.archive_recovery_contract(&candidate.job_id))
                                .ok()
                                .flatten()
                                == Some(1);
                            #[cfg(target_os = "linux")]
                            let message = if is_internal_v1 {
                                "InternalV1 recovery classification failed"
                            } else {
                                "startup recovery classification failed"
                            };
                            #[cfg(not(target_os = "linux"))]
                            let message = "startup recovery classification failed";
                            #[cfg(target_os = "linux")]
                            if is_internal_v1 {
                                if candidate.state == JobState::Downloading
                                    && matches!(
                                        persistence.internal_v1_attempt_phase(&candidate.job_id),
                                        Ok(Some(None))
                                    )
                                {
                                    v2_results.push((
                                        candidate.job_id.clone(),
                                        Err(message.to_owned()),
                                    ));
                                    continue;
                                }
                                if candidate.state == JobState::Downloaded
                                    && matches!(
                                        persistence.internal_v1_attempt_phase(&candidate.job_id),
                                        Ok(Some(None))
                                    )
                                {
                                    let record_root = containment
                                        .as_ref()
                                        .and_then(|_| containment_record_root(&staging_root).ok());
                                    let drained = record_root.is_some_and(|record_root| {
                                        Database::open(&database_path)
                                            .and_then(|database| database.archive_attempt(&candidate.job_id))
                                            .and_then(|attempt| {
                                                recover_writer_domain_set(
                                                    &record_root,
                                                    &staging_root,
                                                    &candidate.job_id,
                                                    u64::from(attempt),
                                                )
                                                .map_err(xarchive_storage::StorageError::Io)
                                            })
                                            .is_ok()
                                    });
                                    if drained {
                                        let _ = persistence.fail(
                                            &candidate.job_id,
                                            "ARCHIVE_COMMIT_RECOVERY_FAILED",
                                            "InternalV1 downloaded attempt is missing its recovery journal",
                                        );
                                    } else {
                                        v2_results.push((
                                            candidate.job_id.clone(),
                                            Err(message.to_owned()),
                                        ));
                                    }
                                } else {
                                    v2_results.push((candidate.job_id.clone(), Err(message.to_owned())));
                                }
                            } else if candidate.state == JobState::Downloaded {
                                let error_code = match error {
                                    xarchive_storage::StorageError::InvalidMetadata(_) => {
                                        "ARCHIVE_COMMIT_RECOVERY_FAILED"
                                    }
                                    _ => "ARCHIVE_RECOVERY_DATABASE_ERROR",
                                };
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    error_code,
                                    "archive recovery classification failed",
                                );
                            }
                            #[cfg(not(target_os = "linux"))]
                            let _ = message;
                            continue;
                        }
                    };
                    #[cfg(target_os = "linux")]
                    let internal_v1_commit_started = if recovery_contract == ArchiveRecoveryDispatch::InternalV1 {
                        match persistence.archive_commit_started(&candidate.job_id) {
                            Ok(true) if candidate.state == JobState::Downloaded && containment.is_none() => {
                                let result = recover_internal_v1_on_runner(
                                    &service,
                                    database_path.clone(),
                                    archive_root.clone(),
                                    staging_root.clone(),
                                    candidate.job_id.clone(),
                                    candidate.tweet_id.clone(),
                                );
                                match result {
                                    Ok(true) => v2_results.push((candidate.job_id.clone(), Ok(true))),
                                    Ok(false) => v2_results.push((candidate.job_id.clone(), Ok(false))),
                                    Err(_) => v2_results.push((candidate.job_id.clone(), Err("InternalV1 recovery refused".to_owned()))),
                                }
                                continue;
                            }
                            Err(error) if candidate.state == JobState::Downloaded => {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "ARCHIVE_COMMIT_RECOVERY_FAILED",
                                    &error.to_string(),
                                );
                                continue;
                            }
                            Ok(started) => started,
                            Err(_) => {
                                v2_results.push((candidate.job_id.clone(), Err("InternalV1 recovery refused".to_owned())));
                                continue;
                            }
                        }
                    } else {
                        false
                    };
                    #[cfg(target_os = "linux")]
                    let internal_v1_published_candidate = recovery_contract
                        == ArchiveRecoveryDispatch::InternalV1
                        && candidate.state == JobState::Downloaded
                        && !internal_v1_commit_started;
                    #[cfg(target_os = "linux")]
                    if recovery_contract == ArchiveRecoveryDispatch::InternalV1
                        && internal_v1_commit_started
                    {
                        if containment.is_some() {
                            let drain = (|| {
                                let record_root = containment_record_root(&staging_root)
                                    .map_err(|error| error.to_string())?;
                                let attempt = Database::open(&database_path)
                                    .and_then(|database| database.archive_attempt(&candidate.job_id))
                                    .map_err(|error| error.to_string())?;
                                if attempt == 0 {
                                    return Err("InternalV1 recovery has no durable attempt".to_owned());
                                }
                                recover_writer_domain_set(
                                    &record_root,
                                    &staging_root,
                                    &candidate.job_id,
                                    u64::from(attempt),
                                )
                                .map_err(|error| error.to_string())
                            })();
                            if let Err(error) = drain {
                                let _ = error;
                                v2_results.push((candidate.job_id.clone(), Err("writer-domain recovery refused".to_owned())));
                                continue;
                            }
                            let result = recover_internal_v1_on_runner(
                                &service,
                                database_path.clone(),
                                archive_root.clone(),
                                staging_root.clone(),
                                candidate.job_id.clone(),
                                candidate.tweet_id.clone(),
                            );
                            match result {
                                Ok(true) => v2_results.push((candidate.job_id.clone(), Ok(true))),
                                Ok(false) => v2_results.push((candidate.job_id.clone(), Ok(false))),
                                Err(_) => v2_results.push((
                                    candidate.job_id.clone(),
                                    Err("InternalV1 recovery refused".to_owned()),
                                )),
                            }
                            continue;
                        }
                        let result = recover_internal_v1_on_runner(
                            &service,
                            database_path.clone(),
                            archive_root.clone(),
                            staging_root.clone(),
                            candidate.job_id.clone(),
                            candidate.tweet_id.clone(),
                        );
                        match result {
                            Ok(true) => v2_results.push((candidate.job_id.clone(), Ok(true))),
                            Ok(false) => v2_results.push((candidate.job_id.clone(), Ok(false))),
                            Err(ExecutorError::Execution { error_code, .. })
                                if error_code == "WORKSPACE_ADMISSION_UNAVAILABLE" =>
                            {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "WRITER_DOMAIN_RECOVERY_FAILED",
                                    "InternalV1 recovery workspace admission was unavailable",
                                );
                                v2_results.push((candidate.job_id.clone(), Err("writer-domain recovery refused".to_owned())));
                            }
                            Err(_) => v2_results.push((
                                candidate.job_id.clone(),
                                Err("InternalV1 recovery refused".to_owned()),
                            )),
                        }
                        continue;
                    }
                    #[cfg(target_os = "linux")]
                    if internal_v1_published_candidate && containment.is_some() {
                        let drain = (|| {
                            let record_root = containment_record_root(&staging_root)
                                .map_err(|error| error.to_string())?;
                            let attempt = Database::open(&database_path)
                                .and_then(|database| database.archive_attempt(&candidate.job_id))
                                .map_err(|error| error.to_string())?;
                            if attempt == 0 {
                                return Err("InternalV1 recovery has no durable attempt".to_owned());
                            }
                            recover_writer_domain_set(
                                &record_root,
                                &staging_root,
                                &candidate.job_id,
                                u64::from(attempt),
                            )
                            .map_err(|error| error.to_string())
                        })();
                        if let Err(error) = drain {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                "WRITER_DOMAIN_RECOVERY_FAILED",
                                &error,
                            );
                            continue;
                        }
                        let boundary_persistence = match StorageJobPersistence::open(&database_path) {
                            Ok(value) => value,
                            Err(_) => continue,
                        };
                        let coordinator = match ProductionCoordinator::open(&database_path) {
                            Ok(value) => value,
                            Err(_) => continue,
                        };
                        let boundary = SessionExecutionBoundary::new(
                            service.clone(),
                            boundary_persistence,
                            coordinator,
                            archive_root.clone(),
                        )
                        .with_optional_recovery_containment(
                            containment.clone(),
                            staging_root.clone(),
                        );
                        let _ = boundary
                            .execute_on_runner(candidate.job_id.clone())
                            .and_then(resolve_session_run);
                        continue;
                    }
                    if recovery_contract == ArchiveRecoveryDispatch::InternalV1 {
                        let commit_started = internal_v1_commit_started;
                        if !commit_started {
                            if !matches!(candidate.state, JobState::Queued | JobState::Validating)
                                && candidate.state != JobState::Downloading
                            {
                                if containment.is_some() {
                                    let drained = (|| {
                                        let record_root = containment_record_root(&staging_root)
                                            .map_err(|error| error.to_string())?;
                                        let attempt = Database::open(&database_path)
                                            .and_then(|database| database.archive_attempt(&candidate.job_id))
                                            .map_err(|error| error.to_string())?;
                                        recover_writer_domain_set(
                                            &record_root,
                                            &staging_root,
                                            &candidate.job_id,
                                            u64::from(attempt),
                                        )
                                        .map_err(|error| error.to_string())
                                    })();
                                    if let Err(error) = drained {
                                        let _ = error;
                                        continue;
                                    }
                                }
                                if candidate.state == JobState::Downloading {
                                    v2_results.push((candidate.job_id.clone(), Err(
                                        "InternalV1 successor takeover requires an available containment-backed session".to_owned(),
                                    )));
                                    continue;
                                }
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "ARCHIVE_COMMIT_RECOVERY_FAILED",
                                    "InternalV1 recovery requires a journaled publication attempt",
                                );
                                continue;
                            }
                            let boundary_persistence = match StorageJobPersistence::open(&database_path) {
                                Ok(value) => value,
                                Err(_) => continue,
                            };
                            let coordinator = match ProductionCoordinator::open(&database_path) {
                                Ok(value) => value,
                                Err(_) => continue,
                            };
                            let boundary = SessionExecutionBoundary::new(
                                service.clone(),
                                boundary_persistence,
                                coordinator,
                                archive_root.clone(),
                            )
                            .with_optional_recovery_containment(
                                containment.clone(),
                                staging_root.clone(),
                            );
                            let _ = boundary
                                .execute_on_runner(candidate.job_id.clone())
                                .and_then(resolve_session_run);
                            continue;
                        }
                        #[cfg(target_os = "linux")]
                        if containment.is_some() {
                            let drained = (|| {
                                let record_root = containment_record_root(&staging_root)
                                    .map_err(|error| error.to_string())?;
                                let attempt = Database::open(&database_path)
                                    .and_then(|database| database.archive_attempt(&candidate.job_id))
                                    .map_err(|error| error.to_string())?;
                                if attempt == 0 {
                                    return Err("InternalV1 contract has no durable attempt".to_owned());
                                }
                                recover_writer_domain_set(
                                    &record_root,
                                    &staging_root,
                                    &candidate.job_id,
                                    u64::from(attempt),
                                )
                                .map_err(|error| error.to_string())
                            })();
                            if let Err(error) = drained {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "WRITER_DOMAIN_RECOVERY_FAILED",
                                    &error,
                                );
                                v2_results.push((
                                    candidate.job_id.clone(),
                                    Err("writer-domain recovery refused".to_owned()),
                                ));
                                continue;
                            }
                        }
                        let result = recover_internal_v1_on_runner(
                            &service,
                            database_path.clone(),
                            archive_root.clone(),
                            staging_root.clone(),
                            candidate.job_id.clone(),
                            candidate.tweet_id.clone(),
                        );
                        match result {
                            Ok(true) => v2_results.push((candidate.job_id.clone(), Ok(true))),
                            Ok(false) => v2_results.push((candidate.job_id.clone(), Ok(false))),
                            Err(ExecutorError::Execution { error_code, .. })
                                if error_code == "WORKSPACE_ADMISSION_UNAVAILABLE" =>
                            {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "WRITER_DOMAIN_RECOVERY_FAILED",
                                    "InternalV1 recovery workspace admission was unavailable",
                                );
                            }
                            Err(_) => v2_results.push((
                                candidate.job_id.clone(),
                                Err("InternalV1 recovery refused".to_owned()),
                            )),
                        }
                        continue;
                    }
                    if let ArchiveRecoveryDispatch::Unknown(_) = recovery_contract {
                        let _ = persistence.fail(&candidate.job_id, "ARCHIVE_RECOVERY_UNSUPPORTED_VERSION", "unsupported internal archive recovery contract version");
                        continue;
                    }

                    let has_execution_spec =
                        persistence.has_execution_spec(&candidate.job_id).unwrap_or(false);
                    if !has_execution_spec && recovery_contract != ArchiveRecoveryDispatch::Legacy {
                        let _ = persistence.fail(
                            &candidate.job_id,
                            "EXECUTION_SPEC_MISSING",
                            "archive execution spec is missing; recovery cannot reconstruct the request",
                        );
                        continue;
                    }

                    if recovery_contract == ArchiveRecoveryDispatch::InternalRenameV2 {
                        #[cfg(not(target_os = "linux"))]
                        let _ = enable_v2;
                        // V2 execution and finalization remain gated until the
                        // service-level transaction/replay contract is enabled.
                        // Do not restart an incomplete v2 job as a fresh run.
                        let _ = persistence.fail(
                            &candidate.job_id,
                            "ARCHIVE_RECOVERY_V2_NOT_READY",
                            "v2 archive recovery is not yet enabled",
                        );
                        continue;
                    }

                    let internal_v1_commit_started =
                        if recovery_contract == ArchiveRecoveryDispatch::InternalV1 {
                            match persistence.archive_commit_started(&candidate.job_id) {
                                Ok(started) => started,
                                Err(error) => {
                                    let _ = persistence.fail(
                                        &candidate.job_id,
                                        "ARCHIVE_RECOVERY_DATABASE_ERROR",
                                        &error.to_string(),
                                    );
                                    continue;
                                }
                            }
                        } else {
                    #[cfg(target_os = "linux")]
                    if recovery_contract == ArchiveRecoveryDispatch::InternalV1
                        && internal_v1_commit_started
                    {
                        let result = recover_internal_v1_on_runner(
                            &service,
                            database_path.clone(),
                            archive_root.clone(),
                            staging_root.clone(),
                            candidate.job_id.clone(),
                            candidate.tweet_id.clone(),
                        );
                        match result {
                            Ok(true) => v2_results.push((candidate.job_id.clone(), Ok(true))),
                            Ok(false) => v2_results.push((candidate.job_id.clone(), Ok(false))),
                            Err(_) => v2_results.push((
                                candidate.job_id.clone(),
                                Err("InternalV1 recovery refused".to_owned()),
                            )),
                        }
                        continue;
                    }
                            false
                        };
                    if candidate.state != JobState::Downloaded && !internal_v1_commit_started {
                        if !has_execution_spec {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                "ARCHIVE_RECOVERY_LEGACY_INCOMPLETE",
                                "legacy archive has no execution spec or committed recovery facts",
                            );
                            continue;
                        }
                        // Route through the owned SessionExecutionBoundary so the
                        // session is retained through drain and fenced finalization
                        // (A4). The boundary owns persistence and coordinator, so
                        // they are moved into the runner operation; no caller-side
                        // claim or legacy dispatch remains. Open a fresh
                        // persistence for the boundary so the loop's `persistence`
                        // remains available to later recovery branches.
                        let boundary_persistence = match StorageJobPersistence::open(&database_path) {
                            Ok(p) => p,
                            Err(_) => continue,
                        };
                        let coordinator = match ProductionCoordinator::open(&database_path) {
                            Ok(c) => c,
                            Err(_) => continue,
                        };
                        let boundary = SessionExecutionBoundary::new(
                            service.clone(),
                            boundary_persistence,
                            coordinator,
                            archive_root.clone(),
                        )
                        .with_optional_recovery_containment(
                            containment.clone(),
                            staging_root.clone(),
                        );
                        match boundary
                            .execute_on_runner(candidate.job_id.clone())
                            .and_then(resolve_session_run)
                        {
                            Ok(SessionRunOutcome::Finished(_)) => continue,
                            Ok(SessionRunOutcome::Reschedulable { .. }) => continue,
                            // Defensive only: `resolve_session_run` consumed
                            // the pending owner; its decision is already
                            // durable, so this behaves like `Finished`.
                            Ok(SessionRunOutcome::StartupCleanupPending { .. }) => continue,
                            Err(_) => continue,
                        }
                    }

                    let final_directory = PathBuf::from("Tweets").join(&candidate.tweet_id);
                    let files = match FileStore::with_staging_root(&archive_root, &staging_root) {
                        Ok(files) => files,
                        Err(error) => {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                "ARCHIVE_RECOVERY_FILESYSTEM_ERROR",
                                &error.to_string(),
                            );
                            continue;
                        }
                    };
                    let facts = CommitRecoveryFacts {
                        job_id: candidate.job_id.clone(),
                        state: candidate.state,
                        final_directory: if files
                            .recovery_directory_exists(&candidate.job_id, &final_directory)
                            .unwrap_or(false)
                        {
                            CommitRecoveryDirectory::Present
                        } else {
                            CommitRecoveryDirectory::Missing
                        },
                        staging_directory: if files
                            .recovery_directory_exists(&candidate.job_id, Path::new("_staging"))
                            .unwrap_or(false)
                        {
                            CommitRecoveryDirectory::Present
                        } else {
                            CommitRecoveryDirectory::Missing
                        },
                    };

                    match facts.decide(&final_directory.to_string_lossy()) {
                        CommitRecoveryDecision::ResumeStaging => {
                            let database = match Database::open(&database_path) {
                                Ok(database) => database,
                                Err(error) => {
                                    let _ = persistence.fail(
                                        &candidate.job_id,
                                        "ARCHIVE_RECOVERY_DATABASE_ERROR",
                                        &error.to_string(),
                                    );
                                    continue;
                                }
                            };
                            let tweet_row_id = match database.tweet_row_id(&candidate.tweet_id) {
                                Ok(row_id) => row_id,
                                Err(error) => {
                                    let _ = persistence.fail(
                                        &candidate.job_id,
                                        "ARCHIVE_RECOVERY_DATABASE_ERROR",
                                        &error.to_string(),
                                    );
                                    continue;
                                }
                            };
                            let mut archive = ArchiveService::new(database, files);
                            match archive.recover_staging_archive(
                                &candidate.job_id,
                                tweet_row_id,
                                &final_directory,
                            ) {
                                Ok(_) => {
                                    let _ = service.complete_persisted(
                                        &mut persistence,
                                        &candidate.job_id,
                                        &final_directory.to_string_lossy(),
                                    );
                                }
                                Err(error) => {
                                    let _ = persistence.fail(
                                        &candidate.job_id,
                                        "ARCHIVE_COMMIT_RECOVERY_FAILED",
                                        &error.to_string(),
                                    );
                                }
                            }
                        }
                        CommitRecoveryDecision::Complete { archive_directory } => {
                            let _ = service.complete_persisted(
                                &mut persistence,
                                &candidate.job_id,
                                &archive_directory,
                            );
                        }
                        CommitRecoveryDecision::MarkFailed {
                            error_code,
                            error_message,
                        } => {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                &error_code,
                                &error_message,
                            );
                        }
                        CommitRecoveryDecision::Skip | CommitRecoveryDecision::NotApplicable => {}
                    }
                }
                v2_results.sort_by(|left: &(String, Result<bool, String>), right| {
                    left.0.cmp(&right.0)
                });
                let outcomes = v2_results
                    .into_iter()
                    .map(|(job_id, result)| StartupRecoveryOutcome {
                        job_id,
                        status: match result {
                            Ok(true) => StartupRecoveryStatus::Recovered,
                            Ok(false) => StartupRecoveryStatus::DeferredByContention,
                            Err(_) => StartupRecoveryStatus::Refused,
                        },
                    })
                    .collect();
                StartupRecoveryReport { outcomes, fatal_error: None }
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    pub fn is_running(&self) -> bool {
        self.executor.is_running()
    }

    pub fn shutdown(self) -> Result<(), ExecutorError> {
        self.executor.shutdown()
    }

    pub fn shutdown_in_place(&mut self) -> Result<(), ExecutorError> {
        self.executor.shutdown_in_place()
    }
}

#[derive(Clone)]
pub struct ArchiveApplicationService {
    pub(super) executor: JobExecutorHandle,
    pub(super) v2_enrollment: bool,
}

#[derive(Debug)]
struct JobRecord {
    snapshot: JobSnapshot,
    events: Vec<ExecutorEvent>,
    cancellation: CancellationToken,
}

#[derive(Clone, Default)]
struct CancellationRegistry(Arc<Mutex<HashMap<String, CancellationToken>>>);

impl CancellationRegistry {
    fn token(&self, job_id: &str) -> Result<CancellationToken, ExecutorError> {
        self.0
            .lock()
            .map_err(|_| ExecutorError::Execution {
                error_code: "CANCELLATION_REGISTRY_POISONED".to_owned(),
                error_message: "job cancellation registry was poisoned".to_owned(),
                persistence_already_updated: false,
            })
            .map(|mut tokens| tokens.entry(job_id.to_owned()).or_default().clone())
    }
}

enum Command {
    Submit {
        request: ArchiveJobRequest,
        response: mpsc::Sender<Result<SubmitResult, ExecutorError>>,
    },
    Start {
        job_id: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Cancel {
        job_id: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Snapshot {
        job_id: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Shutdown {
        response: mpsc::Sender<Result<(), ExecutorError>>,
    },
    Recover {
        jobs: Vec<JobSnapshot>,
        response: mpsc::Sender<Result<Vec<JobSnapshot>, ExecutorError>>,
    },
    Events {
        job_id: String,
        response: mpsc::Sender<Result<Vec<ExecutorEvent>, ExecutorError>>,
    },
    SidecarCrashed {
        job_id: String,
        error_message: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Execute {
        job_id: String,
        execution: Box<dyn JobExecution>,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
    RunJob {
        job_id: String,
        snapshot: JobSnapshot,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
}

enum RunnerCommand {
    /// Run one owned lifecycle operation inline on the runner. The closure
    /// deliberately has no access to a dispatch handle, preventing recursive
    /// submission to the runner that is currently executing it.
    Operation {
        operation: Box<dyn RunnerOperation>,
    },
    Execute {
        job_id: String,
        snapshot: JobSnapshot,
        execution: Box<dyn JobExecution>,
        cancellation: CancellationToken,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
    RunJob {
        job_id: String,
        snapshot: JobSnapshot,
        cancellation: CancellationToken,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
    Shutdown,
}

trait RunnerOperation: Send {
    fn run(self: Box<Self>);
}

struct RunnerOperationTask<F, T> {
    operation: Option<Box<F>>,
    response: mpsc::Sender<Result<T, ExecutorError>>,
    admission: Option<OwnedRunnerAdmission>,
}

impl<F, T> RunnerOperation for RunnerOperationTask<F, T>
where
    F: FnOnce() -> Result<T, ExecutorError> + Send + 'static,
    T: Send + 'static,
{
    fn run(mut self: Box<Self>) {
        let operation = self.operation.take().expect("one-shot runner operation");
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
        // Drop the boxed FnOnce explicitly: closure environments are not
        // guaranteed to be destroyed before a caller's response is observed.
        drop(self.operation.take());
        let result = match outcome {
            Ok(result) => result,
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("runner operation panicked")
                    .to_owned();
                // Destroy the panic payload before publishing the error or
                // releasing admission; panic payloads may own lifecycle data.
                drop(payload);
                Err(ExecutorError::Execution {
                    error_code: "RUNNER_OPERATION_PANICKED".to_owned(),
                    error_message: message,
                    persistence_already_updated: false,
                })
            }
        };
        // The boxed closure and any panic payload are gone before admission is
        // released. Release before replying so observing the result also
        // observes completed admission cleanup.
        drop(self.admission.take());
        let _ = self.response.send(result);
    }
}

/// Owned counterpart to `RunnerReservation`: it can cross the runner's
/// `'static` queue and releases admission only after the inline operation and
/// all values captured by that operation have been dropped.
pub(super) struct OwnedRunnerAdmission(Arc<AtomicUsize>);

impl Drop for OwnedRunnerAdmission {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct JobExecutorHandle {
    sender: SyncSender<Command>,
    runner_sender: SyncSender<RunnerCommand>,
    pub(super) factory: Arc<dyn JobExecutionFactory>,
    /// Tracks how many callers currently hold a runner slot. Mirrors the
    /// capacity of `runner_sender` so admission can be decided *before* a
    /// durable attempt/state claim instead of after it.
    pub(super) runner_in_flight: Arc<AtomicUsize>,
    cancellations: CancellationRegistry,
}

pub struct JobExecutor {
    handle: JobExecutorHandle,
    worker: Option<JoinHandle<()>>,
    runner: Option<JoinHandle<()>>,
}

impl JobExecutor {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_QUEUE_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_factory(capacity, Arc::new(UnconfiguredExecutionFactory))
    }

    pub fn with_capacity_and_factory(
        capacity: usize,
        factory: Arc<dyn JobExecutionFactory>,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
        let (runner_sender, runner_receiver) = mpsc::sync_channel(RUNNER_CAPACITY);
        let handle = JobExecutorHandle {
            sender,
            runner_sender: runner_sender.clone(),
            factory: factory.clone(),
            runner_in_flight: Arc::new(AtomicUsize::new(0)),
            cancellations: CancellationRegistry::default(),
        };
        let runner = thread::Builder::new()
            .name("xarchive-job-runner".to_owned())
            .spawn(move || run_runner(runner_receiver, factory))
            .expect("job runner must spawn");
        let worker_cancellations = handle.cancellations.clone();
        let worker = thread::Builder::new()
            .name("xarchive-job-executor".to_owned())
            .spawn(move || run_worker(receiver, runner_sender, worker_cancellations))
            .expect("job executor worker must spawn");
        Self {
            handle,
            worker: Some(worker),
            runner: Some(runner),
        }
    }

    pub fn handle(&self) -> JobExecutorHandle {
        self.handle.clone()
    }

    pub fn is_running(&self) -> bool {
        self.worker.is_some()
    }

    pub fn shutdown(mut self) -> Result<(), ExecutorError> {
        let result = self.handle.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = self.handle.runner_sender.send(RunnerCommand::Shutdown);
        if let Some(runner) = self.runner.take() {
            let _ = runner.join();
        }
        result
    }

    pub fn shutdown_in_place(&mut self) -> Result<(), ExecutorError> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let result = self.handle.shutdown();
        let _ = worker.join();
        let _ = self.handle.runner_sender.send(RunnerCommand::Shutdown);
        if let Some(runner) = self.runner.take() {
            let _ = runner.join();
        }
        result
    }
}

impl Drop for JobExecutor {
    fn drop(&mut self) {
        if self.worker.is_some() {
            let _ = self.handle.shutdown();
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
            let _ = self.handle.runner_sender.send(RunnerCommand::Shutdown);
            if let Some(runner) = self.runner.take() {
                let _ = runner.join();
            }
        }
    }
}

impl JobExecutorHandle {
    pub(super) fn cancellation_for(
        &self,
        job_id: &str,
    ) -> Result<CancellationToken, ExecutorError> {
        self.cancellations.token(job_id)
    }

    /// Reserve a runner slot before a durable attempt/state claim.
    ///
    /// Returns `false` when the single-active runner is already saturated. A
    /// caller that cannot reserve must leave the Job queued and reschedulable
    /// rather than claiming an attempt and later persisting a failure.
    pub(super) fn runner_try_admit(&self) -> bool {
        self.runner_in_flight
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |in_flight| {
                (in_flight < RUNNER_CAPACITY).then_some(in_flight + 1)
            })
            .is_ok()
    }

    /// Release a previously reserved runner slot.
    pub(super) fn runner_release(&self) {
        self.runner_in_flight.fetch_sub(1, Ordering::AcqRel);
    }

    /// Convert an already-reserved slot into an owned token suitable for a
    /// single inline runner operation. The caller must not release the slot
    /// separately after this succeeds.
    pub(super) fn owned_runner_admission(&self) -> OwnedRunnerAdmission {
        OwnedRunnerAdmission(self.runner_in_flight.clone())
    }

    /// Submit one `'static` operation to run directly on the existing runner
    /// thread, with its admission token owned by that task. The task cannot
    /// recursively submit another runner command because it receives no handle.
    pub(super) fn run_operation<T: Send + 'static>(
        &self,
        admission: OwnedRunnerAdmission,
        operation: impl FnOnce() -> Result<T, ExecutorError> + Send + 'static,
    ) -> Result<T, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        let task: Box<dyn RunnerOperation> = Box::new(RunnerOperationTask {
            operation: Some(Box::new(operation)),
            response: response_sender,
            admission: Some(admission),
        });
        if let Err(error) = self
            .runner_sender
            .try_send(RunnerCommand::Operation { operation: task })
        {
            return match error {
                TrySendError::Full(RunnerCommand::Operation { operation }) => {
                    drop(operation);
                    Err(ExecutorError::QueueFull)
                }
                TrySendError::Disconnected(RunnerCommand::Operation { operation }) => {
                    drop(operation);
                    Err(ExecutorError::Closed)
                }
                _ => unreachable!("only an operation command was sent"),
            };
        }
        let result = response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?;
        result
    }

    pub fn submit(&self, request: ArchiveJobRequest) -> Result<SubmitResult, ExecutorError> {
        self.request(|response| Command::Submit { request, response })
    }

    pub fn start(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::Start {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn cancel(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::Cancel {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::Snapshot {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn shutdown(&self) -> Result<(), ExecutorError> {
        self.request(|response| Command::Shutdown { response })
    }

    pub fn recover(&self, jobs: Vec<JobSnapshot>) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.request(|response| Command::Recover { jobs, response })
    }

    pub fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        self.request(|response| Command::Events {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn sidecar_crashed(
        &self,
        job_id: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::SidecarCrashed {
            job_id: job_id.to_owned(),
            error_message: error_message.to_owned(),
            response,
        })
    }

    pub fn execute(
        &self,
        job_id: &str,
        execution: Box<dyn JobExecution>,
    ) -> Result<JobExecutionResult, ExecutorError> {
        self.request(|response| Command::Execute {
            job_id: job_id.to_owned(),
            execution,
            response,
        })
    }

    /// Dispatch an execution object directly to the single-active runner,
    /// carrying the caller's *owned* running snapshot.
    ///
    /// This is the dispatch path for the runner-owned session boundary, which
    /// owns its persistence context and therefore cannot depend on the
    /// worker's `jobs` map: a borrowed `JobPersistence` cannot cross the
    /// runner's `'static` queue, so the snapshot must be supplied by the
    /// caller rather than looked up by the worker. The caller holds a
    /// [`RunnerReservation`] for the whole call, so the single runner slot is
    /// guaranteed available and `try_send` cannot spuriously report
    /// [`ExecutorError::QueueFull`]. Mid-execution cancellation is a later
    /// boundary concern (B4/B5); the token is never signalled here because the
    /// boundary fences every terminal write on the durable attempt and reports
    /// a durable `Interrupted`/newer-attempt decision after the run.
    pub(crate) fn execute_owned(
        &self,
        job_id: &str,
        snapshot: JobSnapshot,
        execution: Box<dyn JobExecution>,
    ) -> Result<JobExecutionResult, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        self.runner_sender
            .try_send(RunnerCommand::Execute {
                job_id: job_id.to_owned(),
                snapshot,
                execution,
                cancellation: CancellationToken::new(),
                response: response_sender,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => ExecutorError::QueueFull,
                TrySendError::Disconnected(_) => ExecutorError::Closed,
            })?;
        response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?
    }

    pub fn run_job(
        &self,
        job_id: &str,
        snapshot: JobSnapshot,
    ) -> Result<JobExecutionResult, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        self.sender
            .try_send(Command::RunJob {
                job_id: job_id.to_owned(),
                snapshot,
                response: response_sender,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => ExecutorError::QueueFull,
                TrySendError::Disconnected(_) => ExecutorError::Closed,
            })?;
        response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?
    }

    fn request<T>(
        &self,
        build: impl FnOnce(mpsc::Sender<Result<T, ExecutorError>>) -> Command,
    ) -> Result<T, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        self.sender
            .try_send(build(response_sender))
            .map_err(|error| match error {
                TrySendError::Full(_) => ExecutorError::QueueFull,
                TrySendError::Disconnected(_) => ExecutorError::Closed,
            })?;
        response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?
    }
}

impl ArchiveApplicationService {
    pub fn new(executor: &JobExecutor) -> Self {
        Self {
            executor: executor.handle(),
            v2_enrollment: false,
        }
    }

    pub fn submit(&self, request: ArchiveJobRequest) -> Result<SubmitResult, ExecutorError> {
        self.executor.submit(request)
    }

    pub fn query(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.executor.snapshot(job_id)
    }

    pub fn cancel(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.executor.cancel(job_id)
    }

    pub fn recover(&self, jobs: Vec<JobSnapshot>) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.executor.recover(jobs)
    }

    pub fn shutdown(&self) -> Result<(), ExecutorError> {
        self.executor.shutdown()
    }

    pub fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        self.executor.events(job_id)
    }

    pub fn run_job(
        &self,
        job_id: &str,
        snapshot: JobSnapshot,
    ) -> Result<JobExecutionResult, ExecutorError> {
        self.executor.run_job(job_id, snapshot)
    }

    pub fn submit_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request: ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        persistence.configure_v2_enrollment(self.v2_enrollment);
        let result = persistence.create_or_reuse(&request)?;
        if result.created {
            if let Err(error) = self.executor.submit(request) {
                if matches!(error, ExecutorError::Closed) {
                    let failure_message = error.to_string();
                    persistence.fail(
                        &result.job.job_id,
                        "EXECUTOR_UNAVAILABLE",
                        &failure_message,
                    )?;
                    persistence.record_event(
                        &result.job.job_id,
                        ExecutorEvent::DownloadFailed {
                            error_code: "EXECUTOR_UNAVAILABLE".to_owned(),
                            error_message: failure_message,
                        },
                    )?;
                }
                // QueueFull is normal bounded backpressure. The durable Job stays
                // QUEUED so a later batch pass can retry the same submission;
                // only a closed executor is an unavailable/failed condition.
                return Err(error);
            }
            persistence.record_event(
                &result.job.job_id,
                ExecutorEvent::Submitted {
                    job_id: result.job.job_id.clone(),
                },
            )?;
        } else {
            persistence.record_event(
                &result.job.job_id,
                ExecutorEvent::Reused {
                    job_id: result.job.job_id.clone(),
                },
            )?;
        }
        Ok(result)
    }

    /// Submit a persisted Job and schedule its production execution without
    /// waiting for Sidecar, filesystem, or commit I/O to finish.
    ///
    /// The caller owns the short-lived persistence transaction. The scheduled
    /// execution opens its own SQLite context and re-loads the immutable
    /// execution spec by Job ID, so transport and Tauri command callers can
    /// return the initial Job snapshot immediately.
    pub fn submit_and_schedule_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request: ArchiveJobRequest,
        database_path: PathBuf,
    ) -> Result<SubmitResult, ExecutorError> {
        let result = self.submit_persisted(persistence, request)?;
        if result.created
            && let Err(error) = self.schedule_persisted(database_path, result.job.job_id.clone())
        {
            // Do not disguise an orchestration/storage failure as a download
            // failure. QueueFull remains non-failing bounded backpressure in
            // submit_persisted. A detached-thread setup error has no retry
            // owner yet, so persist its established explicit failure result.
            let failure_message = error.to_string();
            persistence.fail(
                &result.job.job_id,
                "EXECUTOR_SCHEDULE_FAILED",
                &failure_message,
            )?;
            persistence.record_event(
                &result.job.job_id,
                ExecutorEvent::DownloadFailed {
                    error_code: "EXECUTOR_SCHEDULE_FAILED".to_owned(),
                    error_message: failure_message,
                },
            )?;
            return Err(error);
        }
        Ok(result)
    }

    /// Schedule one persisted Job on a detached orchestration thread.
    ///
    /// The thread only coordinates persistence and the existing bounded
    /// executor/runner. Long-running archive I/O remains owned by the
    /// executor's execution port and never runs under RuntimeState's mutex.
    pub fn schedule_persisted(
        &self,
        database_path: PathBuf,
        job_id: String,
    ) -> Result<(), ExecutorError> {
        // Fail before detaching the thread when the target database cannot be
        // opened. This is an orchestration failure, distinct from bounded queue
        // rejection (handled before any execution claim).
        StorageJobPersistence::open(&database_path).map_err(|error| ExecutorError::Execution {
            error_code: "EXECUTOR_SCHEDULE_FAILED".to_owned(),
            error_message: format!("failed to open persistence for Job {job_id}: {error}"),
            persistence_already_updated: false,
        })?;

        let service = self.clone();
        thread::Builder::new()
            .name(format!("xarchive-job-schedule-{job_id}"))
            .spawn(move || {
                #[cfg(unix)]
                {
                    // Route ordinary submission through the owned SessionExecutionBoundary
                    // (A4): runner admission → scopes → durable claim → factory → execute
                    // → fenced finalize → session release → admission release.
                    let Ok(persistence) = StorageJobPersistence::open(&database_path) else {
                        return;
                    };
                    let Ok(coordinator) = ProductionCoordinator::open(&database_path) else {
                        return;
                    };
                    let archive_root = database_path
                        .parent()
                        .and_then(Path::parent)
                        .unwrap_or(Path::new("."))
                        .to_owned();
                    let boundary = SessionExecutionBoundary::new(
                        service,
                        persistence,
                        coordinator,
                        archive_root,
                    );
                    let _ = boundary
                        .execute_on_runner(job_id)
                        .and_then(resolve_session_run);
                }
                #[cfg(not(unix))]
                {
                    let Ok(mut persistence) = StorageJobPersistence::open(&database_path) else {
                        return;
                    };
                    let _ = service.execute_persisted_from_factory(&mut persistence, &job_id);
                }
            })
            .map(|_| ())
            .map_err(|error| ExecutorError::Execution {
                error_code: "EXECUTOR_SCHEDULE_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            })
    }

    pub fn query_persisted<P: JobPersistence>(
        &self,
        persistence: &P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        persistence.snapshot(job_id)
    }

    pub fn execute_persisted<P, E>(
        &self,
        persistence: &mut P,
        execution: &mut E,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError>
    where
        P: JobPersistence,
        E: JobExecution,
    {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() {
            return Ok(current);
        }
        if current.state == JobState::Interrupted {
            persistence.persist_state(&JobSnapshot {
                state: JobState::Validating,
                ..current.clone()
            })?;
        }

        let mut running = current.clone();
        let required_states = match running.state {
            JobState::Queued => vec![
                JobState::Validating,
                JobState::MetadataReady,
                JobState::Downloading,
            ],
            JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
            JobState::MetadataReady => vec![JobState::Downloading],
            JobState::Downloading => Vec::new(),
            _ => Vec::new(),
        };
        for next in required_states {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            running.state = next;
            persistence.persist_state(&running)?;
            persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }

        let cancellation = CancellationToken::new();
        match execution
            .execute(&running, &cancellation)
            .and_then(JobExecutionResult::require_finalized)
        {
            Ok(result) => {
                let current = persistence.snapshot(job_id)?;
                if current.state == JobState::Interrupted || cancellation.is_cancelled() {
                    return Ok(current);
                }
                if result.state_already_updated && current.state == JobState::Complete {
                    return Ok(current);
                }
                let downloaded = JobSnapshot {
                    state: JobState::Downloaded,
                    ..running
                };
                if !result.state_already_updated || current.state != JobState::Downloaded {
                    persistence.persist_state(&downloaded)?;
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::StateChanged {
                            from: current.state,
                            to: JobState::Downloaded,
                        },
                    )?;
                }
                if !result.download_event_recorded {
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::DownloadCompleted {
                            files_copied: result.files_copied,
                        },
                    )?;
                }
                self.complete_persisted(persistence, job_id, &result.archive_directory)
            }
            Err(error) => {
                persistence.fail(job_id, &error.error_code, &error.error_message)?;
                persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: error.error_code,
                        error_message: error.error_message,
                    },
                )?;
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn execute_persisted_on_worker<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
        execution: Box<dyn JobExecution>,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() || current.state == JobState::Interrupted {
            return Ok(current);
        }
        // Runner admission must precede the durable attempt/state claim. When
        // the single-active runner is saturated we leave the Job queued and
        // reschedulable: no attempt is incremented and no FAILED is persisted.
        let _reservation = if self.executor.runner_try_admit() {
            self.executor.owned_runner_admission()
        } else {
            return Err(ExecutorError::QueueFull);
        };
        let mut running = current.clone();
        let mut transitions = match running.state {
            JobState::Queued => vec![
                JobState::Validating,
                JobState::MetadataReady,
                JobState::Downloading,
            ],
            JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
            JobState::MetadataReady => vec![JobState::Downloading],
            JobState::Downloading => Vec::new(),
            _ => Vec::new(),
        }
        .into_iter();
        // B2: atomically claim the first running transition together with the
        // attempt increment. A losing claimant returns the winner's snapshot
        // without side effects. A Job already in its running state has no
        // transition to claim, so a plain attempt begin provides the identity
        // used by later fenced writes.
        let attempt = if let Some(first) = transitions.next() {
            let expected_attempt = persistence.current_attempt(job_id)?;
            match persistence.claim_attempt(job_id, expected_attempt, current.state, first)? {
                Some(attempt) => {
                    running.state = first;
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::StateChanged {
                            from: current.state,
                            to: first,
                        },
                    )?;
                    attempt
                }
                None => return persistence.snapshot(job_id),
            }
        } else {
            persistence.begin_attempt(job_id)?
        };
        for next in transitions {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            if !persistence.persist_state_for_attempt(job_id, attempt, previous, next)? {
                // A newer attempt, a durable cancellation, or any other writer
                // already moved the row. This attempt lost the arbitration, so
                // it must stop writing states and events and report the state
                // the winning writer actually left behind.
                return persistence.snapshot(job_id);
            }
            running.state = next;
            persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }

        let run_result = self.executor.execute(job_id, execution);
        // Release the runner slot once the runner interaction is complete. The
        // post-run persistence/archive work below does not occupy the runner.
        drop(_reservation);
        match run_result {
            Ok(result) => {
                let current = persistence.snapshot(job_id)?;
                if current.state == JobState::Interrupted {
                    return Ok(current);
                }
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(current);
                }
                if result.state_already_updated && current.state == JobState::Complete {
                    return Ok(current);
                }
                // Fence the terminal download write on this attempt too: a
                // concurrent cancellation or newer attempt must not be
                // overwritten by `DOWNLOADING -> DOWNLOADED`.
                if current.state != JobState::Downloaded
                    && !persistence.persist_state_for_attempt(
                        job_id,
                        attempt,
                        JobState::Downloading,
                        JobState::Downloaded,
                    )?
                {
                    return persistence.snapshot(job_id);
                }
                persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: JobState::Downloading,
                        to: JobState::Downloaded,
                    },
                )?;
                if !result.download_event_recorded {
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::DownloadCompleted {
                            files_copied: result.files_copied,
                        },
                    )?;
                }
                self.complete_persisted(persistence, job_id, &result.archive_directory)
            }
            Err(error) => {
                if persistence.snapshot(job_id)?.state == JobState::Interrupted {
                    return persistence.snapshot(job_id);
                }
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return persistence.snapshot(job_id);
                }
                if let ExecutorError::Execution {
                    persistence_already_updated: true,
                    ..
                } = &error
                {
                    return persistence.snapshot(job_id);
                }
                let message = error.to_string();
                persistence.fail(job_id, "EXECUTOR_WORKER_FAILED", &message)?;
                persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTOR_WORKER_FAILED".to_owned(),
                        error_message: message,
                    },
                )?;
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn execute_persisted_from_factory<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() || current.state == JobState::Interrupted {
            return Ok(current);
        }
        // Runner admission must precede the durable attempt/state claim. When
        // the single-active runner is saturated we leave the Job queued and
        // reschedulable: no attempt is incremented and no FAILED is persisted.
        let _reservation = if self.executor.runner_try_admit() {
            self.executor.owned_runner_admission()
        } else {
            return Err(ExecutorError::QueueFull);
        };
        let mut running = current.clone();
        let mut transitions = match running.state {
            JobState::Queued => vec![
                JobState::Validating,
                JobState::MetadataReady,
                JobState::Downloading,
            ],
            JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
            JobState::MetadataReady => vec![JobState::Downloading],
            JobState::Downloading => Vec::new(),
            _ => Vec::new(),
        }
        .into_iter();
        // B2: atomically claim the first running transition together with the
        // attempt increment. A losing claimant returns the winner's snapshot
        // without side effects. A Job already in its running state has no
        // transition to claim, so a plain attempt begin provides the identity
        // used by later fenced writes.
        let attempt = if let Some(first) = transitions.next() {
            let expected_attempt = persistence.current_attempt(job_id)?;
            match persistence.claim_attempt(job_id, expected_attempt, current.state, first)? {
                Some(attempt) => {
                    running.state = first;
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::StateChanged {
                            from: current.state,
                            to: first,
                        },
                    )?;
                    attempt
                }
                None => return persistence.snapshot(job_id),
            }
        } else {
            persistence.begin_attempt(job_id)?
        };
        for next in transitions {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            if !persistence.persist_state_for_attempt(job_id, attempt, previous, next)? {
                // A newer attempt, a durable cancellation, or any other writer
                // already moved the row. This attempt lost the arbitration, so
                // it must stop writing states and events and report the state
                // the winning writer actually left behind.
                return persistence.snapshot(job_id);
            }
            running.state = next;
            persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }

        let run_result = self.executor.run_job(job_id, running.clone());
        // Release the runner slot once the runner interaction is complete. The
        // post-run persistence/archive work below does not occupy the runner,
        // so the next Job may start downloading while this one finalizes.
        drop(_reservation);
        match run_result {
            Ok(result) => {
                let current = persistence.snapshot(job_id)?;
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(current);
                }
                if result.state_already_updated && current.state == JobState::Complete {
                    return Ok(current);
                }
                // Fence the terminal download write on this attempt too: a
                // concurrent cancellation or newer attempt must not be
                // overwritten by `DOWNLOADING -> DOWNLOADED`.
                if current.state != JobState::Downloaded
                    && !persistence.persist_state_for_attempt(
                        job_id,
                        attempt,
                        JobState::Downloading,
                        JobState::Downloaded,
                    )?
                {
                    return persistence.snapshot(job_id);
                }
                persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: current.state,
                        to: JobState::Downloaded,
                    },
                )?;
                if !result.download_event_recorded {
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::DownloadCompleted {
                            files_copied: result.files_copied,
                        },
                    )?;
                }
                self.complete_persisted(persistence, job_id, &result.archive_directory)
            }
            Err(error) => {
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return persistence.snapshot(job_id);
                }
                if let ExecutorError::Execution {
                    persistence_already_updated: true,
                    ..
                } = &error
                {
                    return persistence.snapshot(job_id);
                }
                let message = error.to_string();
                persistence.fail(job_id, "EXECUTOR_WORKER_FAILED", &message)?;
                persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTOR_WORKER_FAILED".to_owned(),
                        error_message: message,
                    },
                )?;
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn recover_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
    ) -> Result<Vec<JobSnapshot>, ExecutorError> {
        let candidates = persistence.list_recovery_candidates()?;
        for candidate in &candidates {
            if !persistence.has_execution_spec(&candidate.job_id)? {
                persistence.fail(
                    &candidate.job_id,
                    "EXECUTION_SPEC_MISSING",
                    "archive execution spec is missing; recovery cannot reconstruct the request",
                )?;
                persistence.record_event(
                    &candidate.job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTION_SPEC_MISSING".to_owned(),
                        error_message: "archive execution spec is missing; recovery cannot reconstruct the request".to_owned(),
                    },
                )?;
            }
        }
        let previous_states = candidates
            .iter()
            .map(|snapshot| (snapshot.job_id.clone(), snapshot.state))
            .collect::<HashMap<_, _>>();
        let recovered = self.executor.recover(candidates)?;
        for snapshot in &recovered {
            persistence.persist_state(snapshot)?;
            persistence.record_event(
                &snapshot.job_id,
                ExecutorEvent::Recovered {
                    from: previous_states
                        .get(&snapshot.job_id)
                        .copied()
                        .unwrap_or(JobState::Interrupted),
                    to: snapshot.state,
                },
            )?;
        }
        Ok(recovered)
    }

    pub fn cancel_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() || current.state == JobState::Interrupted {
            return Ok(current);
        }
        // A durable PREPARED/COMMITTED archive boundary means the archive
        // commit owns the job: refuse the cancellation without stopping the
        // writer, so the commit can finish.
        if persistence.archive_commit_started(job_id)? {
            return Ok(current);
        }
        // Stop the writer before the durable decision so nothing keeps writing
        // once cancellation is decided. The decision itself re-checks the
        // commit boundary inside its transaction, so a prepare that landed in
        // the meantime is never overwritten by a cancelled state.
        let stopped = self.executor.cancel(job_id)?;
        match persistence.persist_cancellation(job_id)? {
            CancelArbitration::Cancelled => {
                persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: current.state,
                        to: JobState::Cancelled,
                    },
                )?;
                Ok(stopped)
            }
            // The commit boundary won the race. The durable journal keeps the
            // job non-cancelled and replay completes the commit; no cancelled
            // state or event is written.
            CancelArbitration::CommitWins | CancelArbitration::AlreadyDecided => {
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn interrupt_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
    ) -> Result<(), ExecutorError> {
        let active_jobs = persistence
            .list_recovery_candidates()?
            .into_iter()
            .filter(|snapshot| snapshot.state.is_active())
            .collect::<Vec<_>>();
        for current in active_jobs {
            let interrupted = JobSnapshot {
                job_id: current.job_id.clone(),
                tweet_id: current.tweet_id.clone(),
                state: JobState::Interrupted,
            };
            persistence.persist_state(&interrupted)?;
            persistence.record_event(
                &current.job_id,
                ExecutorEvent::StateChanged {
                    from: current.state,
                    to: JobState::Interrupted,
                },
            )?;
            persistence.record_event(
                &current.job_id,
                ExecutorEvent::ShutdownInterrupted {
                    from: current.state,
                    to: JobState::Interrupted,
                },
            )?;
        }
        Ok(())
    }

    pub fn shutdown_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
    ) -> Result<(), ExecutorError> {
        self.interrupt_persisted(persistence)?;
        self.executor.shutdown()
    }

    pub fn complete_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
        archive_directory: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state == JobState::Complete {
            return Ok(current);
        }
        current
            .state
            .transition_to(JobState::Complete)
            .map_err(|_| ExecutorError::InvalidTransition {
                from: current.state,
                to: JobState::Complete,
            })?;
        let completed = JobSnapshot {
            job_id: current.job_id,
            tweet_id: current.tweet_id,
            state: JobState::Complete,
        };
        persistence.persist_state(&completed)?;
        persistence.record_event(
            job_id,
            ExecutorEvent::Completed {
                archive_directory: archive_directory.to_owned(),
            },
        )?;
        Ok(completed)
    }

    pub fn apply_commit_recovery<P: JobPersistence>(
        &self,
        persistence: &mut P,
        facts: &CommitRecoveryFacts,
        archive_directory: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        match facts.decide(archive_directory) {
            CommitRecoveryDecision::ResumeStaging
            | CommitRecoveryDecision::Skip
            | CommitRecoveryDecision::NotApplicable => persistence.snapshot(&facts.job_id),
            CommitRecoveryDecision::Complete { archive_directory } => {
                self.complete_persisted(persistence, &facts.job_id, &archive_directory)
            }
            CommitRecoveryDecision::MarkFailed {
                error_code,
                error_message,
            } => {
                let failed = persistence.fail(&facts.job_id, &error_code, &error_message)?;
                persistence.record_event(
                    &facts.job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code,
                        error_message,
                    },
                )?;
                Ok(failed)
            }
        }
    }

    pub fn recover_commit_persisted<P, F>(
        &self,
        persistence: &mut P,
        facts_provider: &F,
        job_id: &str,
        archive_directory: &str,
    ) -> Result<JobSnapshot, ExecutorError>
    where
        P: JobPersistence,
        F: CommitRecoveryFactsProvider,
    {
        let snapshot = persistence.snapshot(job_id)?;
        let facts = facts_provider.facts_for(job_id)?;
        if facts.state != snapshot.state || facts.job_id != snapshot.job_id {
            return Err(ExecutorError::Persistence(
                "commit recovery facts do not match persisted Job snapshot".to_owned(),
            ));
        }
        self.apply_commit_recovery(persistence, &facts, archive_directory)
    }

    pub fn recover_commits_persisted<P, F>(
        &self,
        persistence: &mut P,
        facts_provider: &F,
        archive_directory: &str,
    ) -> Vec<CommitRecoveryBatchResult>
    where
        P: JobPersistence,
        F: CommitRecoveryFactsProvider,
    {
        let mut candidates = match persistence.list_recovery_candidates() {
            Ok(candidates) => candidates,
            Err(error) => {
                return vec![CommitRecoveryBatchResult {
                    job_id: String::new(),
                    result: Err(error),
                }];
            }
        };
        candidates.sort_by(|left, right| left.job_id.cmp(&right.job_id));
        candidates
            .into_iter()
            .map(|candidate| {
                let job_id = candidate.job_id.clone();
                let result = self.recover_commit_persisted(
                    persistence,
                    facts_provider,
                    &job_id,
                    archive_directory,
                );
                CommitRecoveryBatchResult { job_id, result }
            })
            .collect()
    }
}

fn run_runner(receiver: Receiver<RunnerCommand>, factory: Arc<dyn JobExecutionFactory>) {
    while let Ok(command) = receiver.recv() {
        match command {
            RunnerCommand::Operation { operation } => operation.run(),
            RunnerCommand::Execute {
                snapshot,
                mut execution,
                cancellation,
                response,
                ..
            } => {
                let result = execution
                    .execute(&snapshot, &cancellation)
                    .and_then(JobExecutionResult::require_finalized)
                    .map_err(|error| ExecutorError::Execution {
                        error_code: error.error_code,
                        error_message: error.error_message,
                        persistence_already_updated: error.persistence_already_updated,
                    });
                // The execution owns its process supervisors and session-bound
                // resources. Tear them down before the caller can proceed to
                // finalization or release its attempt authorization.
                drop(execution);
                let _ = response.send(result);
            }
            RunnerCommand::RunJob {
                job_id,
                snapshot,
                cancellation,
                response,
            } => {
                // The scheduling caller (`execute_persisted_from_factory`)
                // already reserved the single runner slot and claimed the
                // attempt before dispatching here. This arm runs *on* the
                // runner thread, so it must invoke the factory and execution
                // directly: re-entering `run_operation` would enqueue to this
                // same runner and self-deadlock, and acquiring a second
                // admission would double-release the caller's reservation.
                let result = factory
                    .create(&job_id, &snapshot)
                    .and_then(|mut execution| {
                        let result = execution
                            .execute(&snapshot, &cancellation)
                            .and_then(JobExecutionResult::require_finalized)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: error.error_code,
                                error_message: error.error_message,
                                persistence_already_updated: error.persistence_already_updated,
                            });
                        // The execution owns its process supervisors and
                        // session-bound resources; tear them down before the
                        // caller can proceed to finalization or release its
                        // attempt authorization.
                        drop(execution);
                        result
                    });
                let _ = response.send(result);
            }
            RunnerCommand::Shutdown => break,
        }
    }
}

fn run_worker(
    receiver: Receiver<Command>,
    runner_sender: SyncSender<RunnerCommand>,
    cancellations: CancellationRegistry,
) {
    let mut jobs = HashMap::<String, JobRecord>::new();
    while let Ok(command) = receiver.recv() {
        let shutdown = matches!(command, Command::Shutdown { .. });
        handle_command(command, &mut jobs, &runner_sender, &cancellations);
        if shutdown {
            break;
        }
    }
}

fn handle_command(
    command: Command,
    jobs: &mut HashMap<String, JobRecord>,
    runner_sender: &SyncSender<RunnerCommand>,
    cancellations: &CancellationRegistry,
) {
    match command {
        Command::Submit { request, response } => {
            let existing = jobs.values().find(|record| {
                record.snapshot.tweet_id == request.tweet_id && record.snapshot.state.is_active()
            });
            if let Some(existing) = existing {
                let job = existing.snapshot.clone();
                let job_id = job.job_id.clone();
                if let Some(record) = jobs.get_mut(&job_id) {
                    record.events.push(ExecutorEvent::Reused {
                        job_id: job_id.clone(),
                    });
                }
                let _ = response.send(Ok(SubmitResult {
                    job,
                    created: false,
                }));
                return;
            }
            let snapshot = JobSnapshot {
                job_id: request.job_id.clone(),
                tweet_id: request.tweet_id,
                state: JobState::Queued,
            };
            let cancellation = CancellationToken::new();
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            jobs.insert(
                snapshot.job_id.clone(),
                JobRecord {
                    snapshot: snapshot.clone(),
                    events: vec![ExecutorEvent::Submitted {
                        job_id: snapshot.job_id.clone(),
                    }],
                    cancellation,
                },
            );
            let _ = response.send(Ok(SubmitResult {
                job: snapshot,
                created: true,
            }));
        }
        Command::Start { job_id, response } => {
            let result = transition_job(jobs, &job_id, JobState::Validating);
            let _ = response.send(result);
        }
        Command::Cancel { job_id, response } => {
            if let Some(record) = jobs.get(&job_id) {
                record.cancellation.cancel();
            }
            if let Ok(tokens) = cancellations.0.lock()
                && let Some(token) = tokens.get(&job_id)
            {
                token.cancel();
            }
            if let Ok(tokens) = cancellations.0.lock()
                && let Some(token) = tokens.get(&job_id)
            {
                token.cancel();
            }
            let result = cancel_job(jobs, &job_id);
            let _ = response.send(result);
        }
        Command::Snapshot { job_id, response } => {
            let result = jobs
                .get(&job_id)
                .map(|record| record.snapshot.clone())
                .ok_or(ExecutorError::UnknownJob(job_id));
            let _ = response.send(result);
        }
        Command::Events { job_id, response } => {
            let result = jobs
                .get(&job_id)
                .map(|record| record.events.clone())
                .ok_or(ExecutorError::UnknownJob(job_id));
            let _ = response.send(result);
        }
        Command::Shutdown { response } => {
            for record in jobs.values_mut() {
                if record.snapshot.state.is_active() {
                    record.cancellation.cancel();
                    let from = record.snapshot.state;
                    record.snapshot.state = JobState::Interrupted;
                    record.events.push(ExecutorEvent::StateChanged {
                        from,
                        to: JobState::Interrupted,
                    });
                    record.events.push(ExecutorEvent::ShutdownInterrupted {
                        from,
                        to: JobState::Interrupted,
                    });
                }
            }
            let _ = response.send(Ok(()));
        }
        Command::Recover {
            jobs: candidates,
            response,
        } => {
            let mut recovered = Vec::new();
            for candidate in candidates {
                if candidate.state.is_terminal() || jobs.contains_key(&candidate.job_id) {
                    continue;
                }
                let from_state = candidate.state;
                let mut snapshot = candidate;
                if snapshot.state == JobState::Queued || snapshot.state == JobState::Interrupted {
                    snapshot.state = JobState::Validating;
                }
                jobs.insert(
                    snapshot.job_id.clone(),
                    JobRecord {
                        snapshot: snapshot.clone(),
                        events: vec![ExecutorEvent::Recovered {
                            from: from_state,
                            to: snapshot.state,
                        }],
                        cancellation: CancellationToken::new(),
                    },
                );
                recovered.push(snapshot);
            }
            let _ = response.send(Ok(recovered));
        }
        Command::SidecarCrashed {
            job_id,
            error_message,
            response,
        } => {
            let result = sidecar_crash(jobs, &job_id, error_message);
            let _ = response.send(result);
        }
        Command::Execute {
            job_id,
            execution,
            response,
        } => {
            let snapshot = jobs
                .get(&job_id)
                .map(|record| record.snapshot.clone())
                .ok_or_else(|| ExecutorError::UnknownJob(job_id.clone()));
            match snapshot {
                Ok(snapshot) => {
                    let cancellation = jobs
                        .get(&job_id)
                        .map(|record| record.cancellation.clone())
                        .unwrap_or_default();
                    let result = runner_sender.try_send(RunnerCommand::Execute {
                        job_id,
                        snapshot,
                        execution,
                        cancellation,
                        response,
                    });
                    if let Err(error) = result {
                        let (response, executor_error) = match error {
                            TrySendError::Full(RunnerCommand::Execute { response, .. }) => {
                                (response, ExecutorError::QueueFull)
                            }
                            TrySendError::Disconnected(RunnerCommand::Execute {
                                response, ..
                            }) => (response, ExecutorError::Closed),
                            TrySendError::Full(RunnerCommand::Operation { .. })
                            | TrySendError::Disconnected(RunnerCommand::Operation { .. }) => {
                                unreachable!("inline operations bypass the executor worker")
                            }
                            TrySendError::Full(RunnerCommand::RunJob { response, .. }) => {
                                (response, ExecutorError::QueueFull)
                            }
                            TrySendError::Disconnected(RunnerCommand::RunJob {
                                response, ..
                            }) => (response, ExecutorError::Closed),
                            TrySendError::Full(RunnerCommand::Shutdown)
                            | TrySendError::Disconnected(RunnerCommand::Shutdown) => {
                                unreachable!("shutdown is only sent by the executor owner")
                            }
                        };
                        let _ = response.send(Err(executor_error));
                    }
                }
                Err(error) => {
                    let _ = response.send(Err(error));
                }
            }
        }
        Command::RunJob {
            job_id,
            snapshot,
            response,
        } => {
            let cancellation = jobs
                .get(&job_id)
                .map(|record| record.cancellation.clone())
                .unwrap_or_default();
            let result = runner_sender.try_send(RunnerCommand::RunJob {
                job_id,
                snapshot,
                cancellation,
                response,
            });
            if let Err(error) = result {
                let (response, executor_error) = match error {
                    TrySendError::Full(RunnerCommand::RunJob { response, .. }) => {
                        (response, ExecutorError::QueueFull)
                    }
                    TrySendError::Disconnected(RunnerCommand::RunJob { response, .. }) => {
                        (response, ExecutorError::Closed)
                    }
                    TrySendError::Full(RunnerCommand::Execute { response, .. })
                    | TrySendError::Disconnected(RunnerCommand::Execute { response, .. }) => {
                        (response, ExecutorError::Closed)
                    }
                    TrySendError::Full(RunnerCommand::Operation { .. })
                    | TrySendError::Disconnected(RunnerCommand::Operation { .. }) => {
                        unreachable!("inline operations bypass the executor worker")
                    }
                    TrySendError::Full(RunnerCommand::Shutdown)
                    | TrySendError::Disconnected(RunnerCommand::Shutdown) => {
                        unreachable!("shutdown is only sent by the executor owner")
                    }
                };
                let _ = response.send(Err(executor_error));
            }
        }
    }
}

fn transition_job(
    jobs: &mut HashMap<String, JobRecord>,
    job_id: &str,
    next: JobState,
) -> Result<JobSnapshot, ExecutorError> {
    let record = jobs
        .get_mut(job_id)
        .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))?;
    let current = record.snapshot.state;
    current
        .transition_to(next)
        .map_err(|_| ExecutorError::InvalidTransition {
            from: current,
            to: next,
        })?;
    record.snapshot.state = next;
    record.events.push(ExecutorEvent::StateChanged {
        from: current,
        to: next,
    });
    Ok(record.snapshot.clone())
}

fn cancel_job(
    jobs: &mut HashMap<String, JobRecord>,
    job_id: &str,
) -> Result<JobSnapshot, ExecutorError> {
    let record = jobs
        .get_mut(job_id)
        .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))?;
    if record.snapshot.state.is_terminal() {
        return Ok(record.snapshot.clone());
    }
    let current = record.snapshot.state;
    if current == JobState::Cancelled || current == JobState::Interrupted {
        return Ok(record.snapshot.clone());
    }
    current
        .transition_to(JobState::Cancelled)
        .map_err(|_| ExecutorError::InvalidTransition {
            from: current,
            to: JobState::Cancelled,
        })?;
    record.snapshot.state = JobState::Cancelled;
    record.events.push(ExecutorEvent::StateChanged {
        from: current,
        to: JobState::Cancelled,
    });
    Ok(record.snapshot.clone())
}

fn sidecar_crash(
    jobs: &mut HashMap<String, JobRecord>,
    job_id: &str,
    error_message: String,
) -> Result<JobSnapshot, ExecutorError> {
    let record = jobs
        .get_mut(job_id)
        .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))?;
    let current = record.snapshot.state;
    let next = if current.can_transition_to(JobState::Failed) {
        JobState::Failed
    } else if current.can_transition_to(JobState::Interrupted) {
        JobState::Interrupted
    } else {
        return Err(ExecutorError::InvalidTransition {
            from: current,
            to: JobState::Failed,
        });
    };
    record.snapshot.state = next;
    record.events.push(ExecutorEvent::StateChanged {
        from: current,
        to: next,
    });
    record
        .events
        .push(ExecutorEvent::SidecarCrashed { error_message });
    Ok(record.snapshot.clone())
}
