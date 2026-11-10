//! `ExecutorRuntime` startup, recovery and shutdown.
//!
//! Split out of `executor/runtime.rs` without changing behaviour, the module path
//! or the visibility of any item. `executor.rs` re-exports `ExecutorRuntime` from
//! the parent, so every existing path keeps resolving.
//!
//! Invariants that must not move (Plan §3):
//!
//! * startup only *calls* the existing recovery (`recover_v2_on_runner`,
//!   `recover_internal_v1_on_runner`, `recover_writer_domain_set`). It does not
//!   implement a second recovery algorithm here.
//! * provisioning the Linux containment boundary has no environment inference and
//!   no uncontained fallback: an unusable containment configuration is refused
//!   rather than silently downgraded.
//! * shutdown drains and joins the worker and runner before returning, so a caller
//!   that observes the result also observes completed teardown.
//! * the runtime never queues work for itself and never re-admits an already
//!   admitted job.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use xarchive_core::JobState;
use xarchive_storage::{ArchiveRecoveryDispatch, ArchiveService, Database, FileStore};

use super::super::{
    CommitRecoveryDecision, CommitRecoveryDirectory, CommitRecoveryFacts, ExecutorConfig,
    ExecutorError, ExecutorNetworkConfig, JobPersistence, JobSnapshot, ProductionExecutionFactory,
    SessionExecutionBoundary, SessionRunOutcome, StartupRecoveryOutcome, StartupRecoveryReport,
    StartupRecoveryStatus, resolve_session_run,
};
use super::super::{DEFAULT_QUEUE_CAPACITY, StorageJobPersistence};
#[cfg(target_os = "linux")]
use super::super::{
    LinuxExecutionContainment, ProductionCoordinator, containment_record_root,
    recover_internal_v1_on_runner, recover_v2_on_runner, recover_writer_domain_set,
};
use super::{ArchiveApplicationService, ExecutorRuntime, JobExecutor};

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

    pub(crate) fn with_production_factory(
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
    pub(crate) fn recover_startup_with_v2(
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
