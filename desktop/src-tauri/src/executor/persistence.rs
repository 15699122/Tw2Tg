//! Persistence adapters for the executor: the in-memory fake, the
//! storage-backed adapter and the Runtime-owned `ExecutorRuntime` boundary.
//!
//! ENG-15: separated from the application service so a persistence change no
//! longer forces a review of scheduling and control-loop code.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use xarchive_core::JobState;
use xarchive_storage::{ArchiveService, Database, FileStore, JobSummary};

use super::DEFAULT_QUEUE_CAPACITY;
use super::model::*;
use super::runtime::JobExecutor;
use super::service::ArchiveApplicationService;
#[derive(Debug, Default, Clone, Copy)]
pub struct InMemoryJobDatabaseFactory;

impl JobDatabaseFactory for InMemoryJobDatabaseFactory {
    fn open_job_database(&self, _job_id: &str) -> Result<Database, ExecutorError> {
        Database::open_in_memory().map_err(|error| ExecutorError::Persistence(error.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct InMemoryJobPersistence {
    pub(crate) jobs: HashMap<String, JobSnapshot>,
    events: HashMap<String, Vec<ExecutorEvent>>,
    attempts: HashMap<String, u32>,
}

impl JobPersistence for InMemoryJobPersistence {
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        if let Some(existing) = self
            .jobs
            .values()
            .find(|job| job.tweet_id == request.tweet_id && job.state.is_active())
        {
            return Ok(SubmitResult {
                job: existing.clone(),
                created: false,
            });
        }
        let snapshot = JobSnapshot {
            job_id: request.job_id.clone(),
            tweet_id: request.tweet_id.clone(),
            state: JobState::Queued,
        };
        self.jobs.insert(request.job_id.clone(), snapshot.clone());
        Ok(SubmitResult {
            job: snapshot,
            created: true,
        })
    }

    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.jobs
            .get(job_id)
            .cloned()
            .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))
    }

    fn list_recovery_candidates(&self) -> Vec<JobSnapshot> {
        self.jobs
            .values()
            .filter(|job| job.state.is_active() || job.state == JobState::Interrupted)
            .cloned()
            .collect()
    }

    fn snapshot_for_tweet(&self, tweet_id: &str) -> Result<Option<JobSnapshot>, ExecutorError> {
        let mut matches = self
            .jobs
            .values()
            .filter(|job| job.tweet_id == tweet_id)
            .collect::<Vec<_>>();
        matches.sort_by_key(|job| job.job_id.clone());
        Ok(matches.pop().cloned())
    }

    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
        let stored = self
            .jobs
            .get_mut(&snapshot.job_id)
            .ok_or_else(|| ExecutorError::UnknownJob(snapshot.job_id.clone()))?;
        stored.state = snapshot.state;
        Ok(())
    }

    fn fail(
        &mut self,
        job_id: &str,
        _error_code: &str,
        _error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let mut snapshot = self.snapshot(job_id)?;
        if snapshot.state == JobState::Queued {
            let validating = JobSnapshot {
                state: JobState::Validating,
                ..snapshot.clone()
            };
            self.persist_state(&validating)?;
            snapshot = validating;
        }
        if snapshot.state != JobState::Failed {
            let next = snapshot
                .state
                .transition_to(JobState::Failed)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: snapshot.state,
                    to: JobState::Failed,
                })?;
            let updated = JobSnapshot {
                state: next,
                ..snapshot
            };
            self.persist_state(&updated)?;
            return Ok(updated);
        }
        Ok(snapshot)
    }

    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError> {
        if !self.jobs.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(event);
        Ok(())
    }

    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        if !self.jobs.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        Ok(self.events.get(job_id).cloned().unwrap_or_default())
    }

    fn has_execution_spec(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(true)
    }

    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        if !self.jobs.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        let attempt = self.attempts.entry(job_id.to_owned()).or_default();
        *attempt = attempt.saturating_add(1);
        Ok(*attempt)
    }

    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        Ok(self.attempts.get(job_id).copied().unwrap_or_default() == attempt)
    }
}

/// SQLite persistence context used by one executor/application operation.
///
/// The connection is intentionally owned by the operation context rather than
/// by `RuntimeState`, so opening or using it never requires holding the global
/// runtime mutex across worker I/O.
pub struct StorageJobPersistence {
    pub(crate) database: Database,
    tweet_rows: HashMap<String, i64>,
    events: HashMap<String, Vec<ExecutorEvent>>,
}

impl StorageJobPersistence {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        Self::from_database(Database::open(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    pub fn open_in_memory() -> Result<Self, String> {
        Self::from_database(Database::open_in_memory().map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    pub fn from_factory<F: JobDatabaseFactory>(
        factory: &F,
        job_id: &str,
    ) -> Result<Self, ExecutorError> {
        Self::from_database(factory.open_job_database(job_id)?)
    }

    fn from_database(database: Database) -> Result<Self, ExecutorError> {
        Ok(Self {
            database,
            tweet_rows: HashMap::new(),
            events: HashMap::new(),
        })
    }

    fn now() -> String {
        // Production persistence must record the real event time. Tests that
        // need a deterministic timestamp construct a `StorageJobPersistence`
        // with an explicit clock instead of relying on a shared fixed value.
        crate::clock::now_iso()
    }

    fn remember_tweet(&mut self, tweet_id: &str) -> Result<i64, ExecutorError> {
        if let Some(row_id) = self.tweet_rows.get(tweet_id) {
            return Ok(*row_id);
        }
        let row_id = match self.database.tweet_row_id(tweet_id) {
            Ok(row_id) => row_id,
            Err(_) => self
                .database
                .insert_tweet(
                    tweet_id,
                    &format!("https://x.com/test/status/{tweet_id}"),
                    "post",
                    "",
                    &Self::now(),
                )
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
        };
        self.tweet_rows.insert(tweet_id.to_owned(), row_id);
        Ok(row_id)
    }

    pub(crate) fn stored_events(
        &self,
        job_id: &str,
    ) -> Result<Vec<(String, Option<String>)>, ExecutorError> {
        self.database
            .list_events_for_job(job_id, 100)
            .map(|events| {
                events
                    .into_iter()
                    .rev()
                    .map(|event| (event.event_type, event.payload_json))
                    .collect()
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    pub(crate) fn stored_job_summary(&self, job_id: &str) -> Result<JobSummary, ExecutorError> {
        self.database
            .list_recent_jobs(100)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .into_iter()
            .find(|job| job.job_id == job_id)
            .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))
    }

    pub fn recovery_facts(
        &self,
        job_id: &str,
        final_directory: CommitRecoveryDirectory,
        staging_directory: CommitRecoveryDirectory,
    ) -> Result<CommitRecoveryFacts, ExecutorError> {
        let summary = self.stored_job_summary(job_id)?;
        Ok(CommitRecoveryFacts {
            job_id: summary.job_id,
            state: summary.state,
            final_directory,
            staging_directory,
        })
    }
}

/// Runtime-owned executor configuration and lifecycle boundary for the R1 worker.
/// Production runner resources are created from immutable Job execution specs.
pub struct ExecutorRuntime {
    executor: JobExecutor,
    database_path: PathBuf,
    config: ExecutorConfig,
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
            archive_root,
            staging_root: database_path
                .parent()
                .map(|path| path.join("_staging"))
                .unwrap_or_else(|| PathBuf::from("_staging")),
            database_path: database_path.clone(),
            sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok(),
            sidecar_args: std::env::var("XARCHIVE_SIDECAR_ARGS")
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
        };
        Self::with_config(config)
    }

    pub fn with_config(config: ExecutorConfig) -> Self {
        Self {
            executor: JobExecutor::with_capacity_and_factory(
                DEFAULT_QUEUE_CAPACITY,
                Arc::new(ProductionExecutionFactory::new(config.clone())),
            ),
            database_path: config.database_path.clone(),
            config,
        }
    }

    pub fn service(&self) -> ArchiveApplicationService {
        ArchiveApplicationService::new(&self.executor)
    }

    pub fn open_persistence(&self) -> Result<StorageJobPersistence, String> {
        StorageJobPersistence::open(&self.database_path)
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }

    pub fn recover_startup(&self) -> Result<(), ExecutorError> {
        let database_path = self.database_path.clone();
        let archive_root = self.config.archive_root.clone();
        let staging_root = self.config.staging_root.clone();
        let service = self.service();
        thread::Builder::new()
            .name("xarchive-startup-recovery".to_owned())
            .spawn(move || {
                let mut persistence = match StorageJobPersistence::open(&database_path) {
                    Ok(persistence) => persistence,
                    Err(_) => return,
                };
                let candidates = persistence.list_recovery_candidates();
                for candidate in candidates {
                    if !persistence.has_execution_spec(&candidate.job_id).unwrap_or(false) {
                        let _ = persistence.fail(
                            &candidate.job_id,
                            "EXECUTION_SPEC_MISSING",
                            "archive execution spec is missing; recovery cannot reconstruct the request",
                        );
                        continue;
                    }

                    if candidate.state != JobState::Downloaded {
                        let _ = service.execute_persisted_from_factory(
                            &mut persistence,
                            &candidate.job_id,
                        );
                        continue;
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
            })
            .map(|_| ())
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

pub(crate) fn state_sort_priority(state: &JobState) -> u8 {
    match state {
        JobState::Complete => 7,
        JobState::Downloaded => 6,
        JobState::Downloading => 5,
        JobState::TgMediaUploading => 5,
        JobState::TgMetadataSent => 4,
        JobState::TgMetadataSending => 4,
        JobState::MetadataReady => 4,
        JobState::Validating => 3,
        JobState::Queued => 2,
        JobState::Interrupted => 1,
        JobState::AuthRequired => 1,
        JobState::Failed | JobState::Cancelled => 0,
    }
}

impl JobPersistence for StorageJobPersistence {
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        let tweet_row_id = self.remember_tweet(&request.tweet_id)?;
        let created = self
            .database
            .create_archive_job(&request.job_id, tweet_row_id, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if created {
            self.database
                .save_archive_job_request(
                    &request.job_id,
                    1,
                    &request.request_id,
                    &request.request_json,
                    &Self::now(),
                )
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
            let summary = self.stored_job_summary(&request.job_id)?;
            return Ok(SubmitResult {
                job: JobSnapshot::from_job_summary(&summary),
                created: true,
            });
        }
        let job = self
            .database
            .list_recent_jobs(100)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .into_iter()
            .find(|job| job.tweet_id == request.tweet_id && job.state.is_active())
            .ok_or_else(|| ExecutorError::UnknownJob(request.job_id.clone()))?;
        Ok(SubmitResult {
            job: JobSnapshot::from_job_summary(&job),
            created: false,
        })
    }

    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        let summary = self.stored_job_summary(job_id)?;
        Ok(JobSnapshot::from_job_summary(&summary))
    }

    fn has_execution_spec(&self, job_id: &str) -> Result<bool, ExecutorError> {
        self.database
            .archive_job_request(job_id)
            .map(|request| request.is_some())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn list_recovery_candidates(&self) -> Vec<JobSnapshot> {
        self.database
            .list_recent_jobs(100)
            .unwrap_or_default()
            .into_iter()
            .filter(|job| job.state.is_active() || job.state == JobState::Interrupted)
            .map(|job| JobSnapshot::from_job_summary(&job))
            .collect()
    }

    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
        let current = self
            .database
            .job_state(&snapshot.job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if current != snapshot.state {
            self.database
                .transition_job(&snapshot.job_id, snapshot.state, &Self::now())
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        }
        Ok(())
    }

    fn fail(
        &mut self,
        job_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.database
            .fail_job(
                job_id,
                JobState::Failed,
                error_code,
                error_message,
                &Self::now(),
            )
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        self.snapshot(job_id)
    }

    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError> {
        let job_event = event.to_job_event(job_id);
        if let Some(job_event) = job_event {
            // Database::transition_job persists StateChanged atomically with
            // the state update. Do not insert a duplicate event here.
            if !matches!(
                event,
                ExecutorEvent::StateChanged { .. }
                    | ExecutorEvent::Recovered { .. }
                    | ExecutorEvent::ShutdownInterrupted { .. }
            ) {
                self.database
                    .record_event(job_id, &job_event, &Self::now())
                    .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
            }
        }
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(event);
        Ok(())
    }

    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        if !self.events.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        Ok(self.events.get(job_id).cloned().unwrap_or_default())
    }

    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        self.database
            .begin_archive_attempt(job_id, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        self.database
            .archive_attempt(job_id)
            .map(|current| current == attempt)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn snapshot_for_tweet(&self, tweet_id: &str) -> Result<Option<JobSnapshot>, ExecutorError> {
        let jobs = self
            .database
            .list_recent_jobs(100)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let matched = jobs
            .into_iter()
            .filter(|job| job.tweet_id == tweet_id && job.state.is_active())
            .collect::<Vec<_>>();
        if matched.is_empty() {
            return Ok(None);
        }
        let latest = matched
            .into_iter()
            .max_by(|a, b| {
                let a_priority = state_sort_priority(&a.state);
                let b_priority = state_sort_priority(&b.state);
                a_priority
                    .cmp(&b_priority)
                    .then_with(|| a.job_id.cmp(&b.job_id))
            })
            .expect("matched is non-empty");
        Ok(Some(JobSnapshot::from_job_summary(&latest)))
    }
}
