//! Executor data contracts: request, snapshot, result, error, cancellation and
//! recovery-fact types.
//!
//! Pure data plus their inherent `impl`s. No I/O, no transaction and no ownership
//! of a writer, permit, phase or resource scope, so moving them cannot change the
//! ADR-015 execution order. Crate-facing paths stay available through the
//! `executor` re-export.

use std::collections::HashMap;
use std::path::PathBuf;

use xarchive_core::JobState;
use xarchive_storage::JobSummary;

use crate::config::ProxyMode;
use crate::executor::{CommitRecoveryFactsProvider, ExecutorError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveJobRequest {
    pub job_id: String,
    pub tweet_id: String,
    pub request_id: String,
    pub request_json: String,
    pub schema_version: u32,
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

#[derive(Debug, PartialEq, Eq)]
pub struct CommitRecoveryBatchResult {
    pub job_id: String,
    pub result: Result<JobSnapshot, ExecutorError>,
}

#[derive(Debug, Default, PartialEq, Eq)]
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

/// Move-only execution result: a completed attempt's publication facts are
/// consumed once by the session boundary and never cloned or replayed for
/// another attempt. (`JobExecutionError` is likewise non-`Clone`.)
#[derive(Debug)]
pub struct JobExecutionResult {
    pub files_copied: u32,
    pub archive_directory: String,
    pub download_event_recorded: bool,
    pub state_already_updated: bool,
    pub pending_publication: Option<Box<crate::archive::PendingArchivePublication>>,
}

impl JobExecutionResult {
    /// Legacy/non-session dispatch must never turn pending archive data into a
    /// successful download or terminal state. Dropping it returns its owner.
    pub(crate) fn require_finalized(self) -> Result<Self, JobExecutionError> {
        if self.pending_publication.is_some() {
            Err(JobExecutionError {
                error_code: "PUBLICATION_SESSION_REQUIRED".to_owned(),
                error_message:
                    "pending archive publication requires runner-local session finalization"
                        .to_owned(),
                persistence_already_updated: false,
            })
        } else {
            Ok(self)
        }
    }
}

/// Move-only execution error: the failure payload is consumed once by the
/// session boundary and never cloned or replayed for another attempt.
#[derive(Debug)]
pub struct JobExecutionError {
    pub error_code: String,
    pub error_message: String,
    pub persistence_already_updated: bool,
}

/// CROSS_PLATFORM_CHANGE_REQUIRED: attempt creation needs a move-only
/// failure-owner return channel so a retained Sidecar startup-cleanup owner
/// crosses the factory/session boundary instead of being flattened into a
/// string error.
///
/// `StartupCleanupPending` is intentionally non-`Clone`: its
/// [`StartupFailureOwner`](xarchive_sidecar_supervisor::StartupFailureOwner)
/// must be consumed by `retry_cleanup`, and a failed retry returns it intact.
/// Ordinary failures stay `Failed(ExecutorError)` with unchanged semantics.
#[derive(Debug)]
pub enum AttemptCreationError {
    /// Ordinary creation failure with no retained resource.
    Failed(ExecutorError),
    /// Sidecar startup failed and the move-only cleanup owner is retained for
    /// a same-owner retry.
    StartupCleanupPending(xarchive_sidecar_supervisor::StartupFailureOwner),
}

impl From<ExecutorError> for AttemptCreationError {
    fn from(error: ExecutorError) -> Self {
        Self::Failed(error)
    }
}

#[derive(Clone, Debug)]
pub struct ExecutorConfig {
    pub telegram_config_file: Option<PathBuf>,
    pub telegram: crate::config::TelegramConfig,
    pub archive_root: PathBuf,
    pub staging_root: PathBuf,
    pub database_path: PathBuf,
    pub sidecar_program: Option<String>,
    pub gallery_dl_program: Option<String>,
    pub sidecar_args: Vec<String>,
    pub aria2_program: Option<String>,
    /// Whether this execution may spawn aria2. When false, gallery-dl
    /// performs byte downloads and aria2 must remain untouched.
    pub use_aria2: bool,
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
    /// The routing mode. It travels with the proxy because `System` and `Direct`
    /// are decisions, not values: a stored proxy is inert unless the mode says
    /// otherwise.
    pub proxy_mode: ProxyMode,
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
            .field("proxy_mode", &self.proxy_mode)
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
            proxy_mode: ProxyMode::System,
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

    /// Apply the routing mode and the configured value together.
    ///
    /// The value is normalized but kept even for `System` and `Direct`, because
    /// a later switch back to `Manual` must not silently lose the setting. Only
    /// the boundary decides whether a stored value is in effect.
    pub(crate) fn with_proxy(mut self, mode: ProxyMode, proxy: Option<String>) -> Self {
        self.proxy_mode = mode;
        self.proxy = proxy
            .map(|proxy| proxy.trim().to_owned())
            .filter(|proxy| !proxy.is_empty());
        self
    }

    /// The proxy value that is actually in effect for this mode.
    pub(crate) fn active_proxy(&self) -> Option<String> {
        match self.proxy_mode {
            ProxyMode::Manual => self.proxy.clone(),
            ProxyMode::System | ProxyMode::Direct => None,
        }
    }
}
