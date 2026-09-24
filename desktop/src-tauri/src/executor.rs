//! R1 job-executor command/state model and production control boundary.
//!
//! The module owns the bounded control worker, single active runner,
//! persistence ports, recovery contracts, execution-spec fencing, and the
//! `ExecutorRuntime` resource boundary used by RuntimeState. Production
//! execution loads immutable request specs by Job ID and creates its own
//! Database/FileStore/Sidecar context; there is no synchronous archive command
//! fallback.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};

use xarchive_core::{JobEvent, JobState};
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{ArchiveService, Database, FileStore, JobSummary};

const DEFAULT_QUEUE_CAPACITY: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveJobRequest {
    pub job_id: String,
    pub tweet_id: String,
    pub request_id: String,
    pub request_json: String,
}

/// Adapter that validates a browser archive request and derives the durable
/// executor Job identity from it. It performs request validation and Job
/// identity derivation only; it does not own I/O.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArchiveJobSubmissionAdapter;

impl ArchiveJobSubmissionAdapter {
    pub fn prepare(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        xarchive_protocol::BrowserRequest::ArchiveRequest {
            protocol_version: xarchive_protocol::PROTOCOL_VERSION,
            request_id: "archive-validation".to_owned(),
            tweet: request.tweet.clone(),
        }
        .validate()
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;

        Ok(ArchiveJobRequest {
            job_id: format!("archive-{}-{timestamp}", request.tweet.tweet_id),
            tweet_id: request.tweet.tweet_id.clone(),
            request_id: format!("desktop-archive-{}", request.tweet.tweet_id),
            request_json: serde_json::to_string(request)
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
        })
    }

    pub fn submit<P: JobPersistence>(
        &self,
        service: &ArchiveApplicationService,
        persistence: &mut P,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
    ) -> Result<SubmitResult, ExecutorError> {
        let job = self.prepare(request, timestamp)?;
        service.submit_persisted(persistence, job)
    }

    pub fn query<P: JobPersistence>(
        &self,
        service: &ArchiveApplicationService,
        persistence: &P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        service.query_persisted(persistence, job_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSnapshot {
    pub job_id: String,
    pub tweet_id: String,
    pub state: JobState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitRecoveryDirectory {
    Missing,
    Present,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecoveryFacts {
    pub job_id: String,
    pub state: JobState,
    pub final_directory: CommitRecoveryDirectory,
    pub staging_directory: CommitRecoveryDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitRecoveryDecision {
    ResumeStaging,
    Complete {
        archive_directory: String,
    },
    MarkFailed {
        error_code: String,
        error_message: String,
    },
    Skip,
    NotApplicable,
}

pub trait CommitRecoveryFactsProvider {
    fn facts_for(&self, job_id: &str) -> Result<CommitRecoveryFacts, ExecutorError>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommitRecoveryBatchResult {
    pub job_id: String,
    pub result: Result<JobSnapshot, ExecutorError>,
}

#[derive(Debug, Default)]
pub struct InMemoryCommitRecoveryFactsProvider {
    facts: HashMap<String, CommitRecoveryFacts>,
}

impl InMemoryCommitRecoveryFactsProvider {
    pub fn insert(&mut self, facts: CommitRecoveryFacts) {
        self.facts.insert(facts.job_id.clone(), facts);
    }
}

impl CommitRecoveryFactsProvider for InMemoryCommitRecoveryFactsProvider {
    fn facts_for(&self, job_id: &str) -> Result<CommitRecoveryFacts, ExecutorError> {
        self.facts
            .get(job_id)
            .cloned()
            .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))
    }
}

impl CommitRecoveryFacts {
    pub fn decide(&self, archive_directory: &str) -> CommitRecoveryDecision {
        match (self.state, self.final_directory, self.staging_directory) {
            (
                JobState::Downloaded,
                CommitRecoveryDirectory::Missing,
                CommitRecoveryDirectory::Present,
            ) => CommitRecoveryDecision::ResumeStaging,
            (JobState::Downloaded, CommitRecoveryDirectory::Present, _) => {
                CommitRecoveryDecision::Complete {
                    archive_directory: archive_directory.to_owned(),
                }
            }
            (
                JobState::Downloaded,
                CommitRecoveryDirectory::Missing,
                CommitRecoveryDirectory::Missing,
            ) => CommitRecoveryDecision::MarkFailed {
                error_code: "ARCHIVE_COMMIT_INCOMPLETE".to_owned(),
                error_message: format!(
                    "job {} is DOWNLOADED but has neither final archive nor staging directory",
                    self.job_id
                ),
            },
            (JobState::Complete, CommitRecoveryDirectory::Present, _) => {
                CommitRecoveryDecision::Skip
            }
            (JobState::Complete, CommitRecoveryDirectory::Missing, _) => {
                CommitRecoveryDecision::MarkFailed {
                    error_code: "ARCHIVE_COMMIT_MISSING".to_owned(),
                    error_message: format!(
                        "job {} is COMPLETE but its final archive directory is missing",
                        self.job_id
                    ),
                }
            }
            _ => CommitRecoveryDecision::NotApplicable,
        }
    }
}

impl JobSnapshot {
    /// Project only executor-owned fields from the storage-facing JobSummary.
    /// Timestamps, tweet type, and persisted error details remain query-layer
    /// data and are intentionally not synthesized by the executor.
    pub fn from_job_summary(summary: &JobSummary) -> Self {
        Self {
            job_id: summary.job_id.clone(),
            tweet_id: summary.tweet_id.clone(),
            state: summary.state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitResult {
    pub job: JobSnapshot,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecutionResult {
    pub files_copied: u32,
    pub archive_directory: String,
    pub download_event_recorded: bool,
    pub state_already_updated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecutionError {
    pub error_code: String,
    pub error_message: String,
    pub persistence_already_updated: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

pub trait JobExecution: Send {
    fn execute(
        &mut self,
        job: &JobSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError>;
}

pub trait JobExecutionFactory: Send + Sync {
    fn create(
        &self,
        job_id: &str,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError>;
}

#[derive(Clone, Debug)]
pub struct ExecutorConfig {
    pub archive_root: PathBuf,
    pub staging_root: PathBuf,
    pub database_path: PathBuf,
    pub sidecar_program: Option<String>,
    pub sidecar_args: Vec<String>,
    pub aria2_program: Option<String>,
    pub network: ExecutorNetworkConfig,
}

/// Network values carried into production execution.
///
/// Timeouts and retry budgets stay numeric here; the optional proxy may carry
/// credentials, so it is kept off `Debug` output and off process command lines
/// (aria2 and the Sidecar receive it through the environment instead).
#[derive(Clone, Default)]
pub struct ExecutorNetworkConfig {
    pub transfer_timeout: std::time::Duration,
    pub telegram_timeout: std::time::Duration,
    pub proxy: Option<String>,
    pub aria2_connect_timeout: std::time::Duration,
    pub aria2_idle_timeout: std::time::Duration,
    pub aria2_max_tries: u32,
    pub extraction_timeout_secs: f64,
    pub discovery_timeout_secs: f64,
}

impl std::fmt::Debug for ExecutorNetworkConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExecutorNetworkConfig")
            .field("transfer_timeout", &self.transfer_timeout)
            .field("telegram_timeout", &self.telegram_timeout)
            .field("proxy", &self.proxy.as_ref().map(|_| "[REDACTED]"))
            .field("aria2_connect_timeout", &self.aria2_connect_timeout)
            .field("aria2_idle_timeout", &self.aria2_idle_timeout)
            .field("aria2_max_tries", &self.aria2_max_tries)
            .field("extraction_timeout_secs", &self.extraction_timeout_secs)
            .field("discovery_timeout_secs", &self.discovery_timeout_secs)
            .finish()
    }
}

impl ExecutorNetworkConfig {
    pub(crate) fn from_seconds(
        transfer_timeout_seconds: u64,
        telegram_timeout_seconds: u64,
        aria2_connect_timeout_seconds: u64,
        aria2_idle_timeout_seconds: u64,
        aria2_max_tries: u32,
        extraction_timeout_seconds: u64,
        discovery_timeout_seconds: u64,
    ) -> Self {
        Self {
            transfer_timeout: std::time::Duration::from_secs(transfer_timeout_seconds.max(1)),
            telegram_timeout: std::time::Duration::from_secs(telegram_timeout_seconds.max(1)),
            proxy: None,
            aria2_connect_timeout: std::time::Duration::from_secs(
                aria2_connect_timeout_seconds.max(1),
            ),
            aria2_idle_timeout: std::time::Duration::from_secs(aria2_idle_timeout_seconds.max(1)),
            aria2_max_tries: aria2_max_tries.max(1),
            extraction_timeout_secs: extraction_timeout_seconds.max(1) as f64,
            discovery_timeout_secs: discovery_timeout_seconds.max(1) as f64,
        }
    }

    pub(crate) fn with_proxy(mut self, proxy: Option<String>) -> Self {
        self.proxy = proxy
            .map(|proxy| proxy.trim().to_owned())
            .filter(|proxy| !proxy.is_empty());
        self
    }
}

pub struct ProductionExecutionFactory {
    config: ExecutorConfig,
}

impl ProductionExecutionFactory {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }
}

impl JobExecutionFactory for ProductionExecutionFactory {
    fn create(
        &self,
        job_id: &str,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        let database = Database::open(&self.config.database_path)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let (_schema_version, request_id, request_json) = database
            .archive_job_request(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .ok_or_else(|| {
                ExecutorError::Persistence("archive execution spec is missing".to_owned())
            })?;
        let request: crate::ArchiveTweetRequest =
            serde_json::from_str(&request_json).map_err(|error| {
                ExecutorError::Persistence(format!("invalid archive execution spec: {error}"))
            })?;
        if request.tweet.tweet_id != snapshot.tweet_id {
            return Err(ExecutorError::Persistence(
                "archive execution spec tweet identity does not match Job".to_owned(),
            ));
        }
        let tweet_row_id = database
            .tweet_row_id(&request.tweet.tweet_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let files = xarchive_storage::FileStore::with_staging_root(
            self.config.archive_root.clone(),
            self.config.staging_root.clone(),
        )
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let program =
            self.config
                .sidecar_program
                .as_deref()
                .ok_or_else(|| ExecutorError::Execution {
                    error_code: "SIDECAR_NOT_CONFIGURED".to_owned(),
                    error_message: "XARCHIVE_SIDECAR_PROGRAM is not configured".to_owned(),
                    persistence_already_updated: false,
                })?;
        let args = self
            .config
            .sidecar_args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let env = vec![(
            "XARCHIVE_PROXY".to_owned(),
            self.config.network.proxy.clone(),
        )]
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect::<Vec<_>>();
        let supervisor = SidecarSupervisor::spawn_ready_v2_with_env(
            program,
            &args,
            &env,
            std::time::Duration::from_secs(5),
        )
        .map_err(|error| ExecutorError::Execution {
            error_code: "SIDECAR_START_FAILED".to_owned(),
            error_message: error.to_string(),
            persistence_already_updated: false,
        })?;
        let context = crate::archive::ArchiveExecutionContext::with_aria2_and_network(
            database,
            files,
            supervisor,
            self.config.aria2_program.clone(),
            self.config.network.clone(),
        );
        let (execution, _lease) = crate::archive::ArchiveExecutionJob::new(
            context,
            request,
            tweet_row_id,
            request_id,
            crate::runtime::timestamp_marker(),
        );
        Ok(Box::new(execution))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorEvent {
    Submitted {
        job_id: String,
    },
    Reused {
        job_id: String,
    },
    StateChanged {
        from: JobState,
        to: JobState,
    },
    Recovered {
        from: JobState,
        to: JobState,
    },
    ShutdownInterrupted {
        from: JobState,
        to: JobState,
    },
    DownloadStarted {
        backend: String,
    },
    DownloadCompleted {
        files_copied: u32,
    },
    DownloadFailed {
        error_code: String,
        error_message: String,
    },
    Completed {
        archive_directory: String,
    },
    SidecarCrashed {
        error_message: String,
    },
}

impl ExecutorEvent {
    pub fn to_job_event(&self, job_id: &str) -> Option<JobEvent> {
        match self {
            Self::Submitted { .. } => Some(JobEvent::Created {
                job_id: job_id.to_owned(),
            }),
            Self::Reused { .. } => None,
            Self::StateChanged { from, to } | Self::Recovered { from, to } => {
                Some(JobEvent::StateChanged {
                    from: *from,
                    to: *to,
                })
            }
            Self::ShutdownInterrupted { from, to } => Some(JobEvent::StateChanged {
                from: *from,
                to: *to,
            }),
            Self::DownloadStarted { backend } => Some(JobEvent::DownloadStarted {
                backend: backend.clone(),
            }),
            Self::DownloadCompleted { files_copied } => Some(JobEvent::DownloadCompleted {
                files_copied: *files_copied,
            }),
            Self::DownloadFailed {
                error_code,
                error_message,
            } => Some(JobEvent::DownloadFailed {
                error_code: error_code.clone(),
                error_message: error_message.clone(),
            }),
            Self::Completed { archive_directory } => Some(JobEvent::Completed {
                archive_directory: archive_directory.clone(),
            }),
            Self::SidecarCrashed { error_message } => Some(JobEvent::DownloadFailed {
                error_code: "SIDECAR_INTERNAL_ERROR".to_owned(),
                error_message: error_message.clone(),
            }),
        }
    }
}

pub trait JobPersistence {
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError>;
    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError>;
    fn list_recovery_candidates(&self) -> Vec<JobSnapshot>;
    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError>;
    fn fail(
        &mut self,
        job_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError>;
    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError>;
    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError>;

    fn has_execution_spec(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(true)
    }

    fn begin_attempt(&mut self, _job_id: &str) -> Result<u32, ExecutorError> {
        Ok(0)
    }

    fn attempt_is_current(&self, _job_id: &str, _attempt: u32) -> Result<bool, ExecutorError> {
        Ok(true)
    }

    /// Resolve the most recently updated Job for a Tweet, when one exists.
    ///
    /// Default returns `None`; storage-backed adapters override this so the
    /// browser `query_status` boundary can answer by Tweet ID.
    fn snapshot_for_tweet(&self, _tweet_id: &str) -> Result<Option<JobSnapshot>, ExecutorError> {
        Ok(None)
    }
}

/// Connection factory contract for isolated executor Job contexts.
pub trait JobDatabaseFactory {
    fn open_job_database(&self, job_id: &str) -> Result<Database, ExecutorError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct InMemoryJobDatabaseFactory;

impl JobDatabaseFactory for InMemoryJobDatabaseFactory {
    fn open_job_database(&self, _job_id: &str) -> Result<Database, ExecutorError> {
        Database::open_in_memory().map_err(|error| ExecutorError::Persistence(error.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct InMemoryJobPersistence {
    jobs: HashMap<String, JobSnapshot>,
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
    database: Database,
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

    fn now() -> &'static str {
        "2026-09-13T00:00:00Z"
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
                    Self::now(),
                )
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
        };
        self.tweet_rows.insert(tweet_id.to_owned(), row_id);
        Ok(row_id)
    }

    fn stored_events(&self, job_id: &str) -> Result<Vec<(String, Option<String>)>, ExecutorError> {
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

    fn stored_job_summary(&self, job_id: &str) -> Result<JobSummary, ExecutorError> {
        self.database
            .job_summary(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
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
            aria2_program: std::env::var("XARCHIVE_ARIA2_PROGRAM").ok(),
            network: ExecutorNetworkConfig::default(),
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
            .create_archive_job(&request.job_id, tweet_row_id, Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if created {
            self.database
                .save_archive_job_request(
                    &request.job_id,
                    1,
                    &request.request_id,
                    &request.request_json,
                    Self::now(),
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
            .active_job_for_tweet(&request.tweet_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
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
            .list_recovery_candidate_jobs()
            .unwrap_or_default()
            .into_iter()
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
                .transition_job(&snapshot.job_id, snapshot.state, Self::now())
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
                Self::now(),
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
                    .record_event(job_id, &job_event, Self::now())
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
            .begin_archive_attempt(job_id, Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        self.database
            .archive_attempt(job_id)
            .map(|current| current == attempt)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn snapshot_for_tweet(&self, tweet_id: &str) -> Result<Option<JobSnapshot>, ExecutorError> {
        Ok(self
            .database
            .active_job_for_tweet(tweet_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .map(|job| JobSnapshot::from_job_summary(&job)))
    }
}

#[derive(Clone)]
pub struct ArchiveApplicationService {
    executor: JobExecutorHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    Closed,
    QueueFull,
    ResponseClosed,
    UnknownJob(String),
    InvalidTransition {
        from: JobState,
        to: JobState,
    },
    Persistence(String),
    Execution {
        error_code: String,
        error_message: String,
        persistence_already_updated: bool,
    },
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("job executor is closed"),
            Self::QueueFull => formatter.write_str("job executor command queue is full"),
            Self::ResponseClosed => formatter.write_str("job executor response channel is closed"),
            Self::UnknownJob(job_id) => write!(formatter, "unknown job: {job_id}"),
            Self::InvalidTransition { from, to } => {
                write!(formatter, "invalid executor transition: {from:?} -> {to:?}")
            }
            Self::Persistence(message) => {
                write!(formatter, "executor persistence error: {message}")
            }
            Self::Execution {
                error_code,
                error_message,
                persistence_already_updated: _,
            } => write!(
                formatter,
                "job execution error [{error_code}]: {error_message}"
            ),
        }
    }
}

impl std::error::Error for ExecutorError {}

#[derive(Debug)]
struct JobRecord {
    snapshot: JobSnapshot,
    events: Vec<ExecutorEvent>,
    cancellation: CancellationToken,
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

#[derive(Clone)]
pub struct JobExecutorHandle {
    sender: SyncSender<Command>,
    runner_sender: SyncSender<RunnerCommand>,
    factory: Arc<dyn JobExecutionFactory>,
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
        let (runner_sender, runner_receiver) = mpsc::sync_channel(1);
        let handle = JobExecutorHandle {
            sender,
            runner_sender: runner_sender.clone(),
            factory: factory.clone(),
        };
        let runner = thread::Builder::new()
            .name("xarchive-job-runner".to_owned())
            .spawn(move || run_runner(runner_receiver, factory))
            .expect("job runner must spawn");
        let worker = thread::Builder::new()
            .name("xarchive-job-executor".to_owned())
            .spawn(move || run_worker(receiver, runner_sender))
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

struct UnconfiguredExecutionFactory;

impl JobExecutionFactory for UnconfiguredExecutionFactory {
    fn create(
        &self,
        _job_id: &str,
        _snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        Err(ExecutorError::Execution {
            error_code: "EXECUTOR_FACTORY_UNAVAILABLE".to_owned(),
            error_message: "production execution factory is not configured".to_owned(),
            persistence_already_updated: false,
        })
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
        // opened. This gives the caller a chance to persist an explicit
        // scheduling failure against the already-created Job.
        StorageJobPersistence::open(&database_path).map_err(|error| ExecutorError::Execution {
            error_code: "EXECUTOR_SCHEDULE_FAILED".to_owned(),
            error_message: format!("failed to open persistence for Job {job_id}: {error}"),
            persistence_already_updated: false,
        })?;

        let service = self.clone();
        thread::Builder::new()
            .name(format!("xarchive-job-schedule-{job_id}"))
            .spawn(move || {
                let mut persistence = match StorageJobPersistence::open(&database_path) {
                    Ok(persistence) => persistence,
                    Err(_) => return,
                };
                let _ = service.execute_persisted_from_factory(&mut persistence, &job_id);
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
        match execution.execute(&running, &cancellation) {
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
        let attempt = persistence.begin_attempt(job_id)?;
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

        match self.executor.execute(job_id, execution) {
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
                let downloaded = JobSnapshot {
                    state: JobState::Downloaded,
                    ..running
                };
                persistence.persist_state(&downloaded)?;
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
        let attempt = persistence.begin_attempt(job_id)?;
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

        match self.executor.run_job(job_id, running.clone()) {
            Ok(result) => {
                let current = persistence.snapshot(job_id)?;
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(current);
                }
                if result.state_already_updated && current.state == JobState::Complete {
                    return Ok(current);
                }
                let downloaded = JobSnapshot {
                    state: JobState::Downloaded,
                    ..running
                };
                persistence.persist_state(&downloaded)?;
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
        let candidates = persistence.list_recovery_candidates();
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
        let cancelled = self.executor.cancel(job_id)?;
        persistence.persist_state(&cancelled)?;
        persistence.record_event(
            job_id,
            ExecutorEvent::StateChanged {
                from: current.state,
                to: cancelled.state,
            },
        )?;
        Ok(cancelled)
    }

    pub fn interrupt_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
    ) -> Result<(), ExecutorError> {
        let active_jobs = persistence
            .list_recovery_candidates()
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
        let mut candidates = persistence.list_recovery_candidates();
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
            RunnerCommand::Execute {
                snapshot,
                mut execution,
                cancellation,
                response,
                ..
            } => {
                let result = execution
                    .execute(&snapshot, &cancellation)
                    .map_err(|error| ExecutorError::Execution {
                        error_code: error.error_code,
                        error_message: error.error_message,
                        persistence_already_updated: error.persistence_already_updated,
                    });
                let _ = response.send(result);
            }
            RunnerCommand::RunJob {
                job_id,
                snapshot,
                cancellation,
                response,
            } => {
                let result = factory
                    .create(&job_id, &snapshot)
                    .and_then(|mut execution| {
                        execution
                            .execute(&snapshot, &cancellation)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: error.error_code,
                                error_message: error.error_message,
                                persistence_already_updated: error.persistence_already_updated,
                            })
                    });
                let _ = response.send(result);
            }
            RunnerCommand::Shutdown => break,
        }
    }
}

fn run_worker(receiver: Receiver<Command>, runner_sender: SyncSender<RunnerCommand>) {
    let mut jobs = HashMap::<String, JobRecord>::new();
    while let Ok(command) = receiver.recv() {
        let shutdown = matches!(command, Command::Shutdown { .. });
        handle_command(command, &mut jobs, &runner_sender);
        if shutdown {
            break;
        }
    }
}

fn handle_command(
    command: Command,
    jobs: &mut HashMap<String, JobRecord>,
    runner_sender: &SyncSender<RunnerCommand>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    fn request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
        ArchiveJobRequest {
            job_id: job_id.to_owned(),
            tweet_id: tweet_id.to_owned(),
            request_id: format!("test-request-{job_id}"),
            request_json: "{}".to_owned(),
        }
    }

    fn archive_request(tweet_id: &str) -> crate::ArchiveTweetRequest {
        crate::ArchiveTweetRequest {
            tweet: xarchive_protocol::BrowserTweet {
                tweet_id: tweet_id.to_owned(),
                url: format!("https://x.com/alice/status/{tweet_id}"),
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
                text: Some("archive me".to_owned()),
                created_at: Some("2026-09-12T00:00:00Z".to_owned()),
                tweet_type: "post".to_owned(),
                reply_to: None,
                quoted_tweet: None,
            },
            browser: None,
            profile: None,
        }
    }

    fn insert_downloaded_job<P: JobPersistence>(persistence: &mut P, job_id: &str, tweet_id: &str) {
        persistence
            .create_or_reuse(&request(job_id, tweet_id))
            .expect("create job");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
            JobState::Downloaded,
        ] {
            persistence
                .persist_state(&JobSnapshot {
                    job_id: job_id.into(),
                    tweet_id: tweet_id.into(),
                    state,
                })
                .expect("advance job state");
        }
    }

    fn temporary_database_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "xarchive-executor-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system clock")
                    .as_nanos()
            ))
            .join("config")
            .join("archive.sqlite3")
    }

    fn wait_for_persisted_state(
        database_path: &Path,
        job_id: &str,
        expected: JobState,
    ) -> JobSnapshot {
        for _ in 0..100 {
            if let Ok(persistence) = StorageJobPersistence::open(database_path)
                && let Ok(snapshot) = persistence.snapshot(job_id)
                && snapshot.state == expected
            {
                return snapshot;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("Job {job_id} did not reach {expected:?}");
    }

    #[derive(Debug)]
    struct FakeJobExecution {
        result: Result<JobExecutionResult, JobExecutionError>,
        calls: Vec<String>,
    }

    impl FakeJobExecution {
        fn success(files_copied: u32, archive_directory: &str) -> Self {
            Self {
                result: Ok(JobExecutionResult {
                    files_copied,
                    archive_directory: archive_directory.to_owned(),
                    download_event_recorded: false,
                    state_already_updated: false,
                }),
                calls: Vec::new(),
            }
        }

        fn failure(error_code: &str, error_message: &str) -> Self {
            Self {
                result: Err(JobExecutionError {
                    error_code: error_code.to_owned(),
                    error_message: error_message.to_owned(),
                    persistence_already_updated: false,
                }),
                calls: Vec::new(),
            }
        }
    }

    impl JobExecution for FakeJobExecution {
        fn execute(
            &mut self,
            job: &JobSnapshot,
            _cancellation: &CancellationToken,
        ) -> Result<JobExecutionResult, JobExecutionError> {
            self.calls.push(job.job_id.clone());
            self.result.clone()
        }
    }

    struct CancellationAwareExecution;

    impl JobExecution for CancellationAwareExecution {
        fn execute(
            &mut self,
            _job: &JobSnapshot,
            cancellation: &CancellationToken,
        ) -> Result<JobExecutionResult, JobExecutionError> {
            while !cancellation.is_cancelled() {
                thread::sleep(std::time::Duration::from_millis(2));
            }
            Ok(JobExecutionResult {
                files_copied: 1,
                archive_directory: "archives/cancelled-late-result".to_owned(),
                download_event_recorded: false,
                state_already_updated: false,
            })
        }
    }

    #[test]
    fn submits_and_reuses_an_active_job_for_duplicate_tweets() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        let first = handle.submit(request("job-1", "tweet-1")).unwrap();
        let second = handle.submit(request("job-2", "tweet-1")).unwrap();

        assert!(first.created);
        assert!(!second.created);
        assert_eq!(second.job.job_id, "job-1");
        assert_eq!(second.job.state, JobState::Queued);
    }

    #[test]
    fn cancel_is_idempotent_and_does_not_change_terminal_state() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        handle.submit(request("job-1", "tweet-1")).unwrap();

        let cancelled = handle.cancel("job-1").unwrap();
        assert_eq!(cancelled.state, JobState::Cancelled);
        assert_eq!(handle.cancel("job-1").unwrap(), cancelled);
    }

    #[test]
    fn running_cancel_signals_execution_and_fences_late_success() {
        let executor = Arc::new(JobExecutor::new());
        let handle = executor.handle();
        handle
            .submit(request("job-cancel-running", "tweet-cancel-running"))
            .unwrap();
        let worker_handle = handle.clone();
        let execution = thread::spawn(move || {
            worker_handle
                .execute("job-cancel-running", Box::new(CancellationAwareExecution))
                .expect("execution response")
        });

        thread::sleep(std::time::Duration::from_millis(10));
        let cancelled = handle.cancel("job-cancel-running").expect("cancel");
        let late_result = execution.join().expect("execution thread");

        assert_eq!(cancelled.state, JobState::Cancelled);
        assert_eq!(
            late_result.archive_directory,
            "archives/cancelled-late-result"
        );
        assert_eq!(
            handle.snapshot("job-cancel-running").unwrap().state,
            JobState::Cancelled
        );
        drop(executor);
    }

    #[test]
    fn shutdown_interrupts_active_jobs_and_rejects_later_commands() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        handle.submit(request("job-1", "tweet-1")).unwrap();
        handle.submit(request("job-2", "tweet-2")).unwrap();

        executor.shutdown().unwrap();
        assert_eq!(handle.snapshot("job-1").unwrap_err(), ExecutorError::Closed);
    }

    #[test]
    fn concurrent_submissions_keep_one_active_worker_per_tweet() {
        let executor = Arc::new(JobExecutor::with_capacity(16));
        let mut workers = Vec::new();
        for index in 0..8 {
            let executor = Arc::clone(&executor);
            workers.push(thread::spawn(move || {
                executor
                    .handle()
                    .submit(request(&format!("job-{index}"), "tweet-1"))
                    .unwrap()
            }));
        }
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.created).count(), 1);
    }

    #[test]
    fn start_preserves_job_state_machine_boundaries() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        handle.submit(request("job-1", "tweet-1")).unwrap();
        assert_eq!(handle.start("job-1").unwrap().state, JobState::Validating);
        assert_eq!(
            handle.start("job-1").unwrap_err(),
            ExecutorError::InvalidTransition {
                from: JobState::Validating,
                to: JobState::Validating,
            }
        );
    }

    #[test]
    fn application_service_delegates_submit_query_cancel_and_shutdown() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let submitted = service.submit(request("job-1", "tweet-1")).expect("submit");
        assert!(submitted.created);
        assert_eq!(service.query("job-1").unwrap(), submitted.job);
        assert_eq!(service.cancel("job-1").unwrap().state, JobState::Cancelled);
        service.shutdown().expect("shutdown");
    }

    #[test]
    fn recovery_skips_terminal_and_duplicate_jobs_and_restarts_interrupted_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service.submit(request("job-1", "tweet-1")).unwrap();

        let recovered = service
            .recover(vec![
                JobSnapshot {
                    job_id: "job-1".into(),
                    tweet_id: "tweet-1".into(),
                    state: JobState::Interrupted,
                },
                JobSnapshot {
                    job_id: "job-2".into(),
                    tweet_id: "tweet-2".into(),
                    state: JobState::Interrupted,
                },
                JobSnapshot {
                    job_id: "job-3".into(),
                    tweet_id: "tweet-3".into(),
                    state: JobState::Complete,
                },
            ])
            .unwrap();

        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].job_id, "job-2");
        assert_eq!(recovered[0].state, JobState::Validating);
        assert_eq!(
            service.query("job-3"),
            Err(ExecutorError::UnknownJob("job-3".into()))
        );
    }

    #[test]
    fn persisted_submit_reuses_database_like_active_job_semantics() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();

        let first = service
            .submit_persisted(&mut persistence, request("job-1", "tweet-1"))
            .unwrap();
        let duplicate = service
            .submit_persisted(&mut persistence, request("job-2", "tweet-1"))
            .unwrap();

        assert!(first.created);
        assert!(!duplicate.created);
        assert_eq!(duplicate.job.job_id, "job-1");
        assert_eq!(
            service.query_persisted(&persistence, "job-1").unwrap(),
            first.job
        );
        assert_eq!(
            persistence.events("job-1").unwrap(),
            vec![
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
                ExecutorEvent::Reused {
                    job_id: "job-1".into(),
                },
            ]
        );
    }

    #[test]
    fn execute_persisted_records_successful_lifecycle_and_completion_order() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-execute", "tweet-execute"))
            .unwrap();
        let mut execution = FakeJobExecution::success(3, "archives/tweet-execute");

        let completed = service
            .execute_persisted(&mut persistence, &mut execution, "job-execute")
            .unwrap();

        assert_eq!(completed.state, JobState::Complete);
        assert_eq!(execution.calls, vec!["job-execute"]);
        assert_eq!(
            persistence.events("job-execute").unwrap(),
            vec![
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Validating,
                },
                ExecutorEvent::StateChanged {
                    from: JobState::Validating,
                    to: JobState::MetadataReady,
                },
                ExecutorEvent::StateChanged {
                    from: JobState::MetadataReady,
                    to: JobState::Downloading,
                },
                ExecutorEvent::StateChanged {
                    from: JobState::Downloading,
                    to: JobState::Downloaded,
                },
                ExecutorEvent::DownloadCompleted { files_copied: 3 },
                ExecutorEvent::Completed {
                    archive_directory: "archives/tweet-execute".into(),
                },
            ]
        );
    }

    #[test]
    fn execute_persisted_records_failure_without_polluting_other_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-failed", "tweet-failed"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-other", "tweet-other"))
            .unwrap();
        let mut execution = FakeJobExecution::failure("AUTH_REQUIRED", "login required");

        let failed = service
            .execute_persisted(&mut persistence, &mut execution, "job-failed")
            .unwrap();

        assert_eq!(failed.state, JobState::Failed);
        assert_eq!(execution.calls, vec!["job-failed"]);
        assert_eq!(
            persistence.snapshot("job-other").unwrap().state,
            JobState::Queued
        );
        assert!(matches!(
            persistence.events("job-failed").unwrap().last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "AUTH_REQUIRED"
        ));
    }

    #[test]
    fn execute_persisted_skips_terminal_jobs_without_calling_execution_port() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-terminal", "tweet-terminal"))
            .unwrap();
        insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let before = persistence.events("job-terminal").unwrap();
        let mut execution = FakeJobExecution::success(1, "archives/terminal");

        let result = service
            .execute_persisted(&mut persistence, &mut execution, "job-terminal")
            .unwrap();

        assert_eq!(result.state, JobState::Complete);
        assert!(execution.calls.is_empty());
        assert_eq!(persistence.events("job-terminal").unwrap(), before);
    }

    #[test]
    fn persisted_submit_marks_created_job_failed_when_executor_is_closed() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        executor.shutdown().unwrap();

        let error = service
            .submit_persisted(&mut persistence, request("job-closed", "tweet-closed"))
            .expect_err("closed executor");

        assert_eq!(error, ExecutorError::Closed);
        assert_eq!(
            persistence.snapshot("job-closed").unwrap().state,
            JobState::Failed
        );
        assert!(persistence.list_recovery_candidates().is_empty());
        assert!(matches!(
            persistence.events("job-closed").unwrap().as_slice(),
            [ExecutorEvent::DownloadFailed { error_code, .. }] if error_code == "EXECUTOR_UNAVAILABLE"
        ));
    }

    #[test]
    fn submit_and_schedule_persists_background_executor_failure() {
        let database_path = temporary_database_path("background-failure");
        std::fs::create_dir_all(database_path.parent().expect("database parent")).expect("parent");
        let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);

        let submitted = service
            .submit_and_schedule_persisted(
                &mut persistence,
                request("job-background-failure", "tweet-background-failure"),
                database_path.clone(),
            )
            .expect("submit and schedule");

        assert!(submitted.created);
        assert_eq!(submitted.job.state, JobState::Queued);

        let failed =
            wait_for_persisted_state(&database_path, "job-background-failure", JobState::Failed);
        assert_eq!(failed.tweet_id, "tweet-background-failure");

        let persisted = StorageJobPersistence::open(&database_path).expect("reopen database");
        assert!(matches!(
            persisted
                .stored_events("job-background-failure")
                .expect("events")
                .last(),
            Some((event_type, payload))
                if event_type == "DOWNLOAD_FAILED"
                    && payload.as_deref().is_some_and(|payload| {
                        payload.contains("EXECUTOR_WORKER_FAILED")
                    })
        ));

        let _ = std::fs::remove_dir_all(database_path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn submit_and_schedule_marks_job_failed_when_database_path_is_invalid() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        let invalid_database_path = std::env::temp_dir()
            .join(format!("xarchive-missing-parent-{}", std::process::id()))
            .join("missing")
            .join("archive.sqlite3");

        let error = service
            .submit_and_schedule_persisted(
                &mut persistence,
                request("job-invalid-database", "tweet-invalid-database"),
                invalid_database_path,
            )
            .expect_err("invalid database path");

        assert!(matches!(
            error,
            ExecutorError::Execution { error_code, .. }
                if error_code == "EXECUTOR_SCHEDULE_FAILED"
        ));
        assert_eq!(
            persistence
                .snapshot("job-invalid-database")
                .expect("snapshot")
                .state,
            JobState::Failed
        );
        assert!(matches!(
            persistence.events("job-invalid-database").expect("events").last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "EXECUTOR_SCHEDULE_FAILED"
        ));
    }

    #[test]
    fn archive_submission_adapter_preserves_sync_job_identity_and_reuse_contract() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let adapter = ArchiveJobSubmissionAdapter;
        let mut persistence = InMemoryJobPersistence::default();
        let request = archive_request("123");

        let first = adapter
            .submit(&service, &mut persistence, &request, "2026-09-13T00:00:00Z")
            .expect("submit");
        let duplicate = adapter
            .submit(&service, &mut persistence, &request, "2026-09-13T00:00:01Z")
            .expect("reuse");

        assert!(first.created);
        assert_eq!(first.job.job_id, "archive-123-2026-09-13T00:00:00Z");
        assert!(!duplicate.created);
        assert_eq!(duplicate.job, first.job);
        assert_eq!(
            adapter
                .query(&service, &persistence, &first.job.job_id)
                .expect("query"),
            first.job
        );
    }

    #[test]
    fn archive_submission_adapter_rejects_invalid_browser_requests_before_submit() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let adapter = ArchiveJobSubmissionAdapter;
        let mut persistence = InMemoryJobPersistence::default();
        let mut request = archive_request("123");
        request.tweet.tweet_id = "not-a-tweet-id".to_owned();
        request.tweet.url = "https://example.com/not-x".to_owned();

        let error = adapter
            .submit(&service, &mut persistence, &request, "2026-09-13T00:00:00Z")
            .expect_err("invalid request");
        assert!(
            matches!(error, ExecutorError::Persistence(message) if message.contains("tweet_id"))
        );
        assert!(persistence.list_recovery_candidates().is_empty());
    }

    #[test]
    fn persisted_recovery_updates_only_recoverable_candidates() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-2", "tweet-2"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-2".into(),
                tweet_id: "tweet-2".into(),
                state: JobState::Interrupted,
            })
            .unwrap();

        let recovered = service.recover_persisted(&mut persistence).unwrap();
        assert_eq!(recovered.len(), 2);
        assert!(
            recovered
                .iter()
                .all(|job| job.state == JobState::Validating)
        );
        assert_eq!(
            service
                .query_persisted(&persistence, "job-2")
                .unwrap()
                .state,
            JobState::Validating
        );
        assert_eq!(
            persistence.events("job-2").unwrap(),
            vec![ExecutorEvent::Recovered {
                from: JobState::Interrupted,
                to: JobState::Validating,
            }]
        );
    }

    #[test]
    fn persisted_cancel_transitions_queued_to_cancelled_and_is_idempotent() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        executor
            .handle()
            .submit(request("job-1", "tweet-1"))
            .unwrap();

        let cancelled = service.cancel_persisted(&mut persistence, "job-1").unwrap();
        assert_eq!(cancelled.state, JobState::Cancelled);
        assert_eq!(persistence.snapshot("job-1").unwrap(), cancelled);
        assert_eq!(
            persistence.events("job-1").unwrap(),
            vec![ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Cancelled,
            }]
        );

        assert_eq!(
            service.cancel_persisted(&mut persistence, "job-1").unwrap(),
            cancelled
        );
        assert_eq!(persistence.events("job-1").unwrap().len(), 1);
    }

    #[test]
    fn persisted_cancel_does_not_change_terminal_job_or_record_an_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let before = persistence.events("job-1").unwrap();

        let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
        assert_eq!(result.state, JobState::Complete);
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    #[test]
    fn persisted_shutdown_interrupts_active_jobs_once_and_skips_terminal_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-active", "tweet-active"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-terminal", "tweet-terminal"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let terminal_events = persistence.events("job-terminal").unwrap();

        service.shutdown_persisted(&mut persistence).unwrap();

        assert_eq!(
            persistence.snapshot("job-active").unwrap().state,
            JobState::Interrupted
        );
        assert_eq!(
            persistence.events("job-active").unwrap(),
            vec![
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Interrupted,
                },
                ExecutorEvent::ShutdownInterrupted {
                    from: JobState::Queued,
                    to: JobState::Interrupted,
                },
            ]
        );
        assert_eq!(persistence.events("job-terminal").unwrap(), terminal_events);
    }

    #[test]
    fn interrupt_persisted_is_separate_from_worker_shutdown() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        service
            .submit(request("job-active", "tweet-active"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-active", "tweet-active"))
            .unwrap();

        service.interrupt_persisted(&mut persistence).unwrap();

        assert_eq!(
            persistence.snapshot("job-active").unwrap().state,
            JobState::Interrupted
        );
        assert!(service.query("job-active").is_ok());
        service.shutdown().unwrap();
    }

    #[test]
    fn persisted_completion_requires_downloaded_state_and_records_completion_once() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let completed = service
            .complete_persisted(&mut persistence, "job-1", "archives/123")
            .unwrap();
        assert_eq!(completed.state, JobState::Complete);
        assert_eq!(persistence.snapshot("job-1").unwrap(), completed);
        assert_eq!(
            persistence.events("job-1").unwrap().last(),
            Some(&ExecutorEvent::Completed {
                archive_directory: "archives/123".into(),
            })
        );

        let events_before_retry = persistence.events("job-1").unwrap();
        assert_eq!(
            service
                .complete_persisted(&mut persistence, "job-1", "archives/123")
                .unwrap(),
            completed
        );
        assert_eq!(persistence.events("job-1").unwrap(), events_before_retry);
    }

    #[test]
    fn persisted_completion_rejects_queued_job_without_writing_state_or_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        let before = persistence.events("job-1").unwrap();

        assert_eq!(
            service.complete_persisted(&mut persistence, "job-1", "archives/123"),
            Err(ExecutorError::InvalidTransition {
                from: JobState::Queued,
                to: JobState::Complete,
            })
        );
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued
        );
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    #[test]
    fn commit_recovery_decision_distinguishes_resume_complete_failure_and_skip() {
        let resume = CommitRecoveryFacts {
            job_id: "job-resume".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        };
        assert_eq!(
            resume.decide("archives/123"),
            CommitRecoveryDecision::ResumeStaging
        );

        let complete = CommitRecoveryFacts {
            job_id: "job-complete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert_eq!(
            complete.decide("archives/123"),
            CommitRecoveryDecision::Complete {
                archive_directory: "archives/123".into(),
            }
        );

        let incomplete = CommitRecoveryFacts {
            job_id: "job-incomplete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert!(matches!(
            incomplete.decide("archives/123"),
            CommitRecoveryDecision::MarkFailed { error_code, .. }
                if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
        ));

        let already_complete = CommitRecoveryFacts {
            job_id: "job-done".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert_eq!(
            already_complete.decide("archives/123"),
            CommitRecoveryDecision::Skip
        );
    }

    #[test]
    fn commit_recovery_decision_detects_missing_archive_for_complete_job() {
        let facts = CommitRecoveryFacts {
            job_id: "job-done".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert!(matches!(
            facts.decide("archives/123"),
            CommitRecoveryDecision::MarkFailed { error_code, error_message }
                if error_code == "ARCHIVE_COMMIT_MISSING"
                    && error_message.contains("job-done")
        ));
    }

    #[test]
    fn commit_recovery_does_not_rewrite_unrelated_job_states() {
        let facts = CommitRecoveryFacts {
            job_id: "job-active".into(),
            state: JobState::Downloading,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Present,
        };
        assert_eq!(
            facts.decide("archives/123"),
            CommitRecoveryDecision::NotApplicable
        );
    }

    #[test]
    fn apply_commit_recovery_resumes_staging_without_fabricating_completion() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        let before = persistence.events("job-1").unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Downloaded);
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    #[test]
    fn apply_commit_recovery_completes_existing_final_directory() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Complete);
        assert!(matches!(
            persistence.events("job-1").unwrap().last(),
            Some(ExecutorEvent::Completed { archive_directory })
                if archive_directory == "archives/123"
        ));
    }

    #[test]
    fn apply_commit_recovery_marks_missing_commit_as_failed_and_records_error() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Failed);
        let summary = persistence.stored_job_summary("job-1").unwrap();
        assert_eq!(
            summary.last_error_code.as_deref(),
            Some("ARCHIVE_COMMIT_INCOMPLETE")
        );
        assert!(matches!(
            persistence.events("job-1").unwrap().last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
        ));
    }

    #[test]
    fn apply_commit_recovery_skips_complete_job_without_duplicate_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let before = persistence.events("job-1").unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Complete);
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    #[test]
    fn recover_commit_persisted_uses_injected_directory_facts_and_matches_snapshot() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let result = service
            .recover_commit_persisted(&mut persistence, &provider, "job-1", "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Complete);
    }

    #[test]
    fn recover_commit_persisted_rejects_facts_that_do_not_match_snapshot() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let error = service
            .recover_commit_persisted(&mut persistence, &provider, "job-1", "archives/123")
            .expect_err("mismatched facts");
        assert!(
            matches!(error, ExecutorError::Persistence(message) if message.contains("do not match"))
        );
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued
        );
    }

    #[test]
    fn storage_recovery_facts_preserve_job_summary_state_and_injected_directories() {
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let facts = persistence
            .recovery_facts(
                "job-1",
                CommitRecoveryDirectory::Present,
                CommitRecoveryDirectory::Missing,
            )
            .unwrap();
        assert_eq!(facts.job_id, "job-1");
        assert_eq!(facts.state, JobState::Downloaded);
        assert_eq!(facts.final_directory, CommitRecoveryDirectory::Present);
        assert_eq!(facts.staging_directory, CommitRecoveryDirectory::Missing);
    }

    #[test]
    fn recover_commits_persisted_keeps_results_sorted_and_isolates_fact_errors() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        for job_id in ["job-b", "job-a"] {
            persistence
                .create_or_reuse(&request(job_id, &format!("tweet-{job_id}")))
                .unwrap();
            persistence
                .persist_state(&JobSnapshot {
                    job_id: job_id.into(),
                    tweet_id: format!("tweet-{job_id}"),
                    state: JobState::Downloaded,
                })
                .unwrap();
        }

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-a".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert_eq!(
            results
                .iter()
                .map(|result| result.job_id.as_str())
                .collect::<Vec<_>>(),
            vec!["job-a", "job-b"]
        );
        assert_eq!(
            results[0].result.as_ref().unwrap().state,
            JobState::Complete
        );
        assert!(matches!(
            &results[1].result,
            Err(ExecutorError::UnknownJob(job_id)) if job_id == "job-b"
        ));
        assert_eq!(
            persistence.snapshot("job-a").unwrap().state,
            JobState::Complete
        );
        assert_eq!(
            persistence.snapshot("job-b").unwrap().state,
            JobState::Downloaded
        );
    }

    #[test]
    fn recover_commits_persisted_skips_terminal_jobs_from_candidate_scan() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-complete", "tweet-complete"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Complete,
            })
            .unwrap();

        let provider = InMemoryCommitRecoveryFactsProvider::default();
        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert!(results.is_empty());
    }

    #[test]
    fn recover_commits_persisted_mixes_actions_and_preserves_each_job_event_order() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        insert_downloaded_job(&mut persistence, "job-resume", "tweet-resume");
        insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
        insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
        insert_downloaded_job(&mut persistence, "job-missing", "tweet-missing");
        insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-resume".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-complete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-failed".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-terminal".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert_eq!(
            results
                .iter()
                .map(|result| result.job_id.as_str())
                .collect::<Vec<_>>(),
            vec!["job-complete", "job-failed", "job-missing", "job-resume"]
        );
        assert_eq!(
            results[0].result.as_ref().unwrap().state,
            JobState::Complete
        );
        assert_eq!(results[1].result.as_ref().unwrap().state, JobState::Failed);
        assert!(matches!(
            &results[2].result,
            Err(ExecutorError::UnknownJob(job_id)) if job_id == "job-missing"
        ));
        assert_eq!(
            results[3].result.as_ref().unwrap().state,
            JobState::Downloaded
        );

        assert!(matches!(
            persistence.events("job-complete").unwrap().last(),
            Some(ExecutorEvent::Completed { archive_directory })
                if archive_directory == "archives"
        ));
        assert!(matches!(
            persistence.events("job-failed").unwrap().last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
        ));
        assert!(persistence.events("job-resume").unwrap().is_empty());
        assert_eq!(
            persistence.snapshot("job-terminal").unwrap().state,
            JobState::Complete
        );
    }

    #[test]
    fn storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        insert_downloaded_job(&mut persistence, "job-resume", "tweet-resume");
        insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
        insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
        insert_downloaded_job(&mut persistence, "job-missing", "tweet-missing");
        insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-resume".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-complete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-failed".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-terminal".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert_eq!(
            results
                .iter()
                .map(|result| result.job_id.as_str())
                .collect::<Vec<_>>(),
            vec!["job-complete", "job-failed", "job-missing", "job-resume"]
        );

        let complete_events = persistence.stored_events("job-complete").unwrap();
        assert_eq!(complete_events.last().unwrap().0, "JOB_COMPLETED");
        assert_eq!(
            persistence.snapshot("job-complete").unwrap().state,
            JobState::Complete
        );

        let failed_events = persistence.stored_events("job-failed").unwrap();
        assert_eq!(failed_events.last().unwrap().0, "DOWNLOAD_FAILED");
        let failed_summary = persistence.stored_job_summary("job-failed").unwrap();
        assert_eq!(failed_summary.state, JobState::Failed);
        assert_eq!(
            failed_summary.last_error_code.as_deref(),
            Some("ARCHIVE_COMMIT_INCOMPLETE")
        );
        assert!(
            failed_summary
                .last_error_message
                .as_deref()
                .is_some_and(|message| message.contains("job-failed"))
        );

        assert!(persistence.stored_events("job-resume").unwrap().len() >= 4);
        assert_eq!(
            persistence.snapshot("job-resume").unwrap().state,
            JobState::Downloaded
        );
        let missing_events = persistence.stored_events("job-missing").unwrap();
        assert!(missing_events.len() >= 4);
        assert!(
            missing_events
                .iter()
                .all(|(event_type, _)| event_type == "JOB_STATE_CHANGED")
        );
        assert_eq!(
            persistence.snapshot("job-missing").unwrap().state,
            JobState::Downloaded
        );
        assert_eq!(
            persistence.snapshot("job-terminal").unwrap().state,
            JobState::Complete
        );

        let terminal_events = persistence.stored_events("job-terminal").unwrap();
        assert!(terminal_events.len() >= 5);
        assert!(
            terminal_events
                .iter()
                .all(|(event_type, _)| event_type != "JOB_COMPLETED")
        );
    }

    #[test]
    fn storage_completion_persists_state_then_job_completed_event() {
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service
            .complete_persisted(&mut persistence, "job-1", "archives/123")
            .unwrap();
        let events = persistence.stored_events("job-1").unwrap();
        assert_eq!(events.last().unwrap().0, "JOB_COMPLETED");
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Complete
        );
    }

    #[test]
    fn storage_persistence_adapter_matches_job_repository_contract() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        let first = persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        let duplicate = persistence
            .create_or_reuse(&request("job-2", "tweet-1"))
            .expect("reuse");

        assert!(first.created);
        assert!(!duplicate.created);
        assert_eq!(duplicate.job.job_id, "job-1");
        assert_eq!(
            persistence.snapshot("job-1").expect("snapshot").state,
            JobState::Queued
        );

        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .expect("event");
        assert_eq!(
            persistence.events("job-1").expect("events"),
            vec![ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            }]
        );
    }

    #[test]
    fn database_factory_opens_a_job_context_without_runtime_state() {
        let factory = InMemoryJobDatabaseFactory;
        let mut persistence = StorageJobPersistence::from_factory(&factory, "job-1").unwrap();
        let result = persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        assert!(result.created);
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued
        );
    }

    #[test]
    fn storage_persistence_preserves_state_transition_and_created_event() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .expect("created event");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .expect("transition");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Validating,
                },
            )
            .expect("state event");

        let events = persistence.stored_events("job-1").expect("stored events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "JOB_CREATED");
        assert_eq!(events[1].0, "JOB_STATE_CHANGED");
        assert!(events[0].1.is_some());
        assert!(events[1].1.is_none());
        assert_eq!(
            persistence.snapshot("job-1").expect("snapshot").state,
            JobState::Validating
        );
    }

    #[test]
    fn executor_lifecycle_events_match_existing_archive_job_event_types() {
        let cases = [
            (
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
                "JOB_CREATED",
            ),
            (
                ExecutorEvent::DownloadStarted {
                    backend: "gallery-dl".into(),
                },
                "DOWNLOAD_STARTED",
            ),
            (
                ExecutorEvent::DownloadCompleted { files_copied: 2 },
                "DOWNLOAD_COMPLETED",
            ),
            (
                ExecutorEvent::DownloadFailed {
                    error_code: "DOWNLOAD_FAILED".into(),
                    error_message: "network error".into(),
                },
                "DOWNLOAD_FAILED",
            ),
            (
                ExecutorEvent::Completed {
                    archive_directory: "archives/123".into(),
                },
                "JOB_COMPLETED",
            ),
        ];

        for (event, expected_type) in cases {
            let job_event = event.to_job_event("job-1").expect("mapped event");
            assert_eq!(job_event.event_type(), expected_type);
        }
        assert!(
            ExecutorEvent::Reused {
                job_id: "job-1".into()
            }
            .to_job_event("job-1")
            .is_none()
        );
    }

    #[test]
    fn storage_persistence_does_not_duplicate_transactional_state_events() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .expect("created event");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .expect("transition");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Validating,
                },
            )
            .expect("lifecycle event");

        let events = persistence.stored_events("job-1").expect("events");
        assert_eq!(
            events
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            vec!["JOB_CREATED", "JOB_STATE_CHANGED"]
        );
    }

    #[test]
    fn job_summary_projection_preserves_executor_fields_without_synthesizing_storage_fields() {
        let summary = JobSummary {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            tweet_type: "quote".into(),
            state: JobState::Failed,
            created_at: "2026-09-13T00:00:00Z".into(),
            updated_at: "2026-09-13T00:01:00Z".into(),
            last_error_code: Some("DOWNLOAD_FAILED".into()),
            last_error_message: Some("network error".into()),
        };

        assert_eq!(
            JobSnapshot::from_job_summary(&summary),
            JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Failed,
            }
        );
        assert_eq!(summary.tweet_type, "quote");
        assert_eq!(summary.created_at, "2026-09-13T00:00:00Z");
        assert_eq!(summary.updated_at, "2026-09-13T00:01:00Z");
        assert_eq!(summary.last_error_code.as_deref(), Some("DOWNLOAD_FAILED"));
        assert_eq!(summary.last_error_message.as_deref(), Some("network error"));
    }

    #[test]
    fn storage_summary_and_event_queries_preserve_error_and_nullable_payload_contracts() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        persistence
            .database
            .fail_job(
                "job-1",
                JobState::Failed,
                "DOWNLOAD_FAILED",
                "network error",
                "2026-09-13T00:01:00Z",
            )
            .expect("fail");

        let summary = persistence.stored_job_summary("job-1").expect("summary");
        assert_eq!(summary.state, JobState::Failed);
        assert_eq!(summary.last_error_code.as_deref(), Some("DOWNLOAD_FAILED"));
        assert_eq!(summary.last_error_message.as_deref(), Some("network error"));

        let events = persistence.stored_events("job-1").expect("events");
        assert_eq!(events[0].0, "JOB_STATE_CHANGED");
        assert!(events[0].1.is_none());
    }

    #[test]
    fn storage_persistence_recovery_scans_active_jobs_and_skips_terminal_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");

        persistence
            .create_or_reuse(&request("job-queued", "tweet-queued"))
            .expect("queued job");
        persistence
            .create_or_reuse(&request("job-interrupted", "tweet-interrupted"))
            .expect("interrupted job");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-interrupted".into(),
                tweet_id: "tweet-interrupted".into(),
                state: JobState::Interrupted,
            })
            .expect("interrupt job");
        persistence
            .create_or_reuse(&request("job-failed", "tweet-failed"))
            .expect("failed job");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-failed".into(),
                tweet_id: "tweet-failed".into(),
                state: JobState::Validating,
            })
            .expect("validate failed job");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-failed".into(),
                tweet_id: "tweet-failed".into(),
                state: JobState::Failed,
            })
            .expect("fail job");

        let candidates = persistence.list_recovery_candidates();
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|job| job.job_id != "job-failed"));

        let recovered = service.recover_persisted(&mut persistence).unwrap();
        assert_eq!(recovered.len(), 2);
        assert!(
            recovered
                .iter()
                .all(|job| job.state == JobState::Validating)
        );
        assert_eq!(
            persistence.snapshot("job-queued").unwrap().state,
            JobState::Validating
        );
        assert_eq!(
            persistence.snapshot("job-interrupted").unwrap().state,
            JobState::Validating
        );
        assert_eq!(
            persistence.snapshot("job-failed").unwrap().state,
            JobState::Failed
        );
        let interrupted_events = persistence.stored_events("job-interrupted").unwrap();
        assert_eq!(
            interrupted_events
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            vec!["JOB_STATE_CHANGED", "JOB_STATE_CHANGED"]
        );
    }

    #[test]
    fn persisted_recovery_records_the_actual_source_state_for_queued_and_interrupted_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-queued", "tweet-queued"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-interrupted", "tweet-interrupted"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-interrupted".into(),
                tweet_id: "tweet-interrupted".into(),
                state: JobState::Interrupted,
            })
            .unwrap();

        service.recover_persisted(&mut persistence).unwrap();
        assert_eq!(
            persistence.events("job-queued").unwrap(),
            vec![ExecutorEvent::Recovered {
                from: JobState::Queued,
                to: JobState::Validating,
            }]
        );
        assert_eq!(
            persistence.events("job-interrupted").unwrap(),
            vec![ExecutorEvent::Recovered {
                from: JobState::Interrupted,
                to: JobState::Validating,
            }]
        );
    }

    #[test]
    fn executor_events_preserve_state_transition_order() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service.submit(request("job-1", "tweet-1")).unwrap();
        service.cancel("job-1").unwrap();

        assert_eq!(
            service.events("job-1").unwrap(),
            vec![
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Cancelled,
                },
            ]
        );
    }

    #[test]
    fn sidecar_crash_maps_to_safe_failure_and_core_job_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service.submit(request("job-1", "tweet-1")).unwrap();
        service.executor.start("job-1").unwrap();
        let snapshot = service
            .executor
            .sidecar_crashed("job-1", "worker exited")
            .unwrap();

        assert_eq!(snapshot.state, JobState::Failed);
        let events = service.events("job-1").unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutorEvent::SidecarCrashed { error_message } if error_message == "worker exited"
        )));
        assert_eq!(
            events.last().and_then(|event| event.to_job_event("job-1")),
            Some(JobEvent::DownloadFailed {
                error_code: "SIDECAR_INTERNAL_ERROR".into(),
                error_message: "worker exited".into(),
            })
        );
    }
}
