//! Executor domain model: requests, snapshots, events, errors and the
//! persistence and execution ports that the sibling submodules implement.
//!
//! ENG-15: extracted from the former single `executor.rs` without behaviour
//! change. This submodule has the fewest reasons to change, so the persistence
//! adapters, the application service and the lifecycle handles can evolve
//! without touching the shared contract.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xarchive_core::{JobEvent, JobState};
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{Database, JobSummary};
pub(crate) const DEFAULT_QUEUE_CAPACITY: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveJobRequest {
    pub job_id: String,
    pub tweet_id: String,
    pub request_id: String,
    pub request_json: String,
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
        let supervisor =
            SidecarSupervisor::spawn_ready(program, &args, std::time::Duration::from_secs(5))
                .map_err(|error| ExecutorError::Execution {
                    error_code: "SIDECAR_START_FAILED".to_owned(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                })?;
        let context = crate::archive::ArchiveExecutionContext::new(database, files, supervisor);
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
