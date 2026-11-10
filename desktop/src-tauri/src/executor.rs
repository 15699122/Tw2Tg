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
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::mpsc;

use xarchive_core::{JobEvent, JobState};
// ProxyMode is used by the moved contract types and by this module's tests, so
// it is imported for the test build instead of widening the runtime imports.
#[cfg(test)]
use xarchive_core::ProxyMode;

// The supervisor is named only by this module's regression tests, so it is
// imported for the test build instead of the production import list.
#[cfg(test)]
use xarchive_sidecar_supervisor::SidecarSupervisor;
#[cfg(test)]
pub use xarchive_storage::BatchNamingMode as NamingMode;
pub use xarchive_storage::BatchOutputSettings as OutputSettings;
#[cfg(test)]
use xarchive_storage::{ArchiveService, FileStore};
use xarchive_storage::{Database, JobSummary};
use xarchive_workflow::coordination::Coordinator;
#[cfg(unix)]
use xarchive_workflow::coordination::unix::{LinuxCoordinator, LinuxGuard};
#[cfg(unix)]
use xarchive_workflow::coordination::{LockError, LockScope};
#[cfg(test)]
use xarchive_workflow::{AttemptIdentity, ExecutionReservation};

mod containment;
mod contracts;
mod factory;
mod session;
#[allow(unused_imports)]
pub(crate) use session::{
    SessionExecutionBoundary, SessionRescheduleReason, SessionRunOutcome, resolve_session_run,
};
mod runtime;
#[allow(unused_imports)]
pub use runtime::{ArchiveApplicationService, ExecutorRuntime, JobExecutor, JobExecutorHandle};
mod recovery;
pub(crate) use recovery::{StartupRecoveryOutcome, StartupRecoveryReport, StartupRecoveryStatus};
#[cfg(target_os = "linux")]
use recovery::{recover_internal_v1_on_runner, recover_v2_on_runner};
mod traits;

pub use crate::executor::factory::ProductionExecutionFactory;
use crate::executor::factory::UnconfiguredExecutionFactory;

#[cfg(target_os = "linux")]
pub use crate::executor::containment::{AttemptWriterDomain, LinuxExecutionContainment};

// These were `pub(crate)` items of this module before the slice, so they keep
// exactly that reach through the parent rather than being widened to `pub`.
// `ATTEMPT_WRITER_ROLES` is deliberately absent: only the containment module
// enumerates the roles, and re-exporting it produced an unused import.
#[cfg(target_os = "linux")]
pub(crate) use crate::executor::containment::{
    attempt_writer_base, containment_record_root, recover_and_create_writer_group,
    recover_predecessor_writer_domains, recover_writer_domain_set,
    recover_writer_domain_set_with_evidence,
};

// The in-memory implementations were `pub struct`s of this module before the
// slice, so they are reachable as `crate::executor::<Type>` in every build. The
// re-export stays unconditional to preserve that surface; only the non-test build
// reports it as unused, and that is the accurate signal (no production path names
// these types), not a reason to hide them behind `#[cfg(test)]`.
#[allow(unused_imports)]
pub use crate::executor::traits::{
    InMemoryJobDatabaseFactory, InMemoryJobPersistence, JobDatabaseFactory, JobExecution,
    JobExecutionFactory, JobPersistence,
};

pub use crate::executor::contracts::{
    ArchiveJobRequest, AttemptCreationError, CommitRecoveryBatchResult, CommitRecoveryDecision,
    CommitRecoveryDirectory, CommitRecoveryFacts, ExecutorConfig, ExecutorNetworkConfig,
    InMemoryCommitRecoveryFactsProvider, JobExecutionError, JobExecutionResult, JobSnapshot,
    SubmitResult,
};

const DEFAULT_QUEUE_CAPACITY: usize = 32;
/// Capacity of the single-active-runner channel. Runner admission must be
/// checked before any durable attempt/state claim so a saturated runner never
/// converts an accepted Job into a persisted `FAILED`.
const RUNNER_CAPACITY: usize = 1;

// Execution-spec types and decoders live in `crate::execution_spec`. They are
// re-exported here so existing call sites, serialized payloads and error
// behaviour are unchanged by the move. `decode_execution_spec` is named only by
// other modules' test code (`batch.rs`, `transport.rs`) and by this module's
// tests, so the non-test build reports it unused; the re-export stays because
// those `crate::executor::decode_execution_spec` paths are part of the surface.
#[allow(unused_imports)]
pub use crate::execution_spec::{
    DecodedExecutionSpec, DownloaderArgumentSnapshot, ExecutionSpecV2, ExecutionSpecV3,
    ExecutionSpecV4, MetadataOutputSettings, ToolCompatibility, decode_execution_spec,
    validate_downloader_argument_snapshot,
};
// The spec error type is named only by this module's regression tests, so it is
// imported for the test build instead of widening the re-export surface.
#[cfg(test)]
use crate::execution_spec::ExecutionSpecError;

fn validate_execution_spec_identity(
    decoded: DecodedExecutionSpec,
    job_tweet_id: &str,
) -> Result<DecodedExecutionSpec, ExecutorError> {
    if decoded.request.tweet.tweet_id != job_tweet_id {
        return Err(ExecutorError::Persistence(
            "archive execution spec tweet identity does not match Job".to_owned(),
        ));
    }
    Ok(decoded)
}

/// Adapter that validates a browser archive request and derives the durable
/// executor Job identity from it. It performs request validation and Job
/// identity derivation only; it does not own I/O.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArchiveJobSubmissionAdapter;

impl ArchiveJobSubmissionAdapter {
    pub fn prepare_with_metadata_settings(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
        output_settings: MetadataOutputSettings,
        downloader_arguments: DownloaderArgumentSnapshot,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        let mut prepared = self.prepare(request, timestamp)?;
        validate_downloader_argument_snapshot(&downloader_arguments)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        prepared.request_json = serde_json::to_string(&ExecutionSpecV4 {
            request: request.clone(),
            output_settings,
            downloader_arguments,
        })
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        prepared.schema_version = 4;
        Ok(prepared)
    }
    pub fn prepare_with_output_settings(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
        output_settings: OutputSettings,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        let mut prepared = self.prepare(request, timestamp)?;
        prepared.request_json = serde_json::to_string(&ExecutionSpecV2 {
            request: request.clone(),
            output_settings,
        })
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        Ok(prepared)
    }

    pub fn prepare_with_downloader_settings(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
        output_settings: OutputSettings,
        downloader_arguments: DownloaderArgumentSnapshot,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        validate_downloader_argument_snapshot(&downloader_arguments)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        xarchive_protocol::BrowserRequest::ArchiveRequest {
            protocol_version: xarchive_protocol::PROTOCOL_VERSION,
            request_id: "archive-validation".to_owned(),
            tweet: request.tweet.clone(),
        }
        .validate()
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let spec = ExecutionSpecV3 {
            request: request.clone(),
            output_settings,
            downloader_arguments,
        };
        Ok(ArchiveJobRequest {
            job_id: format!("archive-{}-{timestamp}", request.tweet.tweet_id),
            tweet_id: request.tweet.tweet_id.clone(),
            request_id: format!("desktop-archive-{}", request.tweet.tweet_id),
            request_json: serde_json::to_string(&spec)
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
            schema_version: 3,
        })
    }

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
            request_json: serde_json::to_string(&ExecutionSpecV2 {
                request: request.clone(),
                output_settings: OutputSettings::default(),
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
            schema_version: 2,
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

pub trait CommitRecoveryFactsProvider {
    fn facts_for(&self, job_id: &str) -> Result<CommitRecoveryFacts, ExecutorError>;
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
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

/// Outcome of the durable cancellation decision taken by
/// [`JobPersistence::persist_cancellation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelArbitration {
    /// The job is durably cancelled and its state-change event is written.
    Cancelled,
    /// A durable PREPARED/COMMITTED archive boundary exists: the commit wins
    /// and no cancellation state was written.
    CommitWins,
    /// The job was already terminal; nothing was written.
    AlreadyDecided,
}

/// Connection factory contract for isolated executor Job contexts.
/// SQLite persistence context used by one executor/application operation.
///
/// The connection is intentionally owned by the operation context rather than
/// by `RuntimeState`, so opening or using it never requires holding the global
/// runtime mutex across worker I/O.
pub struct StorageJobPersistence {
    database: Database,
    tweet_rows: HashMap<String, i64>,
    events: HashMap<String, Vec<ExecutorEvent>>,
    v2_enrollment: bool,
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
            v2_enrollment: false,
        })
    }

    fn now() -> String {
        // Production persistence must record the real event time. Tests that
        // need a deterministic timestamp pass it explicitly (for example into
        // `submit`) instead of relying on a shared fixed value.
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

/// Production coordinator backed by the Linux advisory-lock adapter.
///
/// Lives beside the durable database under a stable workspace lock
/// directory. The kernel releases held records when the owner process
/// exits (including SIGKILL), so a crashed owner cannot keep a successor
/// out. Windows has no adapter here: the Windows Owner implements and
/// validates the NTFS coordinator separately.
#[cfg(unix)]
pub(crate) struct ProductionCoordinator {
    inner: LinuxCoordinator,
    #[cfg(target_os = "linux")]
    _workspace: xarchive_workflow::maintenance::WorkspaceGuard,
}

#[cfg(target_os = "linux")]
impl ProductionCoordinator {
    fn workspace_path(&self) -> &Path {
        self.inner.lock_directory()
    }
}

#[cfg(unix)]
impl ProductionCoordinator {
    /// Open a production coordinator whose lock files live beside the
    /// durable database.
    pub(crate) fn open(database_path: &Path) -> Result<Self, ExecutorError> {
        let lock_dir = database_path
            .parent()
            .ok_or_else(|| ExecutorError::Execution {
                error_code: "NO_DATABASE_PARENT".to_owned(),
                error_message: "database path has no parent directory for coordination locks"
                    .to_owned(),
                persistence_already_updated: false,
            })?
            .join("locks");
        let inner = LinuxCoordinator::new(lock_dir).map_err(|error| ExecutorError::Execution {
            error_code: "COORDINATOR_OPEN_FAILED".to_owned(),
            error_message: format!("failed to open production coordinator: {error}"),
            persistence_already_updated: false,
        })?;
        #[cfg(target_os = "linux")]
        let workspace =
            xarchive_workflow::maintenance::WorkspaceGuard::try_enter(inner.lock_directory())
                .map_err(|error| ExecutorError::Execution {
                    error_code: "WORKSPACE_ADMISSION_UNAVAILABLE".to_owned(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                })?;
        Ok(Self {
            inner,
            #[cfg(target_os = "linux")]
            _workspace: workspace,
        })
    }
}

#[cfg(unix)]
impl Coordinator for ProductionCoordinator {
    type Guard = LinuxGuard;

    fn acquire(&self, scope: &LockScope) -> Result<Self::Guard, LockError> {
        self.inner.acquire(scope)
    }

    fn acquire_all(&self, scopes: &[LockScope]) -> Result<Self::Guard, LockError> {
        self.inner.acquire_all(scopes)
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
    fn configure_v2_enrollment(&mut self, enabled: bool) {
        self.v2_enrollment = enabled;
    }
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        let tweet_row_id = self.remember_tweet(&request.tweet_id)?;
        let recovery_contract =
            matches!(request.schema_version, 2..=4).then_some(if self.v2_enrollment {
                xarchive_storage::ArchiveRecoveryContract::InternalRenameV2
            } else {
                xarchive_storage::ArchiveRecoveryContract::InternalV1
            });
        let created = self
            .database
            .create_archive_job_with_optional_recovery_contract(
                &request.job_id,
                tweet_row_id,
                request.schema_version,
                &request.request_id,
                &request.request_json,
                recovery_contract,
                &Self::now(),
            )
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if created {
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

    fn list_recovery_candidates(&self) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.database
            .list_recovery_candidate_jobs()
            .map(|jobs| {
                jobs.into_iter()
                    .map(|job| JobSnapshot::from_job_summary(&job))
                    .collect()
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
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

    /// Attempt-fenced state transition backed by one conditional SQL update.
    ///
    /// `persist_state_if_attempt` is the existing atomic primitive: the update
    /// only applies while the row is still in `expected` with
    /// `attempt_count = attempt`, and it writes the state-change event inside
    /// the same transaction. Returning `false` therefore means a newer
    /// attempt, a durable cancellation, or another writer won the race.
    fn persist_state_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        expected: JobState,
        next: JobState,
    ) -> Result<bool, ExecutorError> {
        self.database
            .persist_state_if_attempt(job_id, attempt, expected, next, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
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

    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError> {
        let applied = self
            .database
            .complete_terminal_for_attempt(
                job_id,
                attempt,
                archive_directory,
                files_copied,
                &Self::now(),
            )
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if !applied {
            // Already Complete (idempotent) or attempt lost: return current snapshot.
            return self.snapshot(job_id);
        }
        // Mirror into the in-memory event cache so `events()` stays consistent.
        let snapshot = self.snapshot(job_id)?;
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(ExecutorEvent::DownloadCompleted { files_copied });
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(ExecutorEvent::Completed {
                archive_directory: archive_directory.to_owned(),
            });
        Ok(snapshot)
    }

    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        self.database
            .begin_archive_attempt(job_id, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
        self.database
            .archive_attempt(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError> {
        self.database
            .claim_archive_attempt(job_id, expected_attempt, from, to, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn claim_internal_v1_successor(
        &mut self,
        job_id: &str,
        expected_predecessor_attempt: u32,
    ) -> Result<Option<u32>, ExecutorError> {
        self.database
            .claim_internal_v1_successor(job_id, expected_predecessor_attempt, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn internal_v1_attempt_phase(&self, job_id: &str) -> Result<Option<Option<u8>>, ExecutorError> {
        self.database
            .internal_v1_recovery_identity(job_id)
            .map(|identity| {
                identity.map(|(phase, _, _, _, _)| {
                    phase.map(|phase| match phase {
                        xarchive_storage::ArchiveRecoveryPhase::Prepared => 1,
                        xarchive_storage::ArchiveRecoveryPhase::Committed => 2,
                    })
                })
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn is_internal_v1_contract(&self, job_id: &str) -> Result<bool, ExecutorError> {
        self.database
            .archive_recovery_contract(job_id)
            .map(|contract| contract == Some(1))
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError> {
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(ExecutorEvent::StateChanged { from, to });
        Ok(())
    }

    fn archive_commit_started(&self, job_id: &str) -> Result<bool, ExecutorError> {
        self.database
            .archive_commit_started(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn persist_cancellation(&mut self, job_id: &str) -> Result<CancelArbitration, ExecutorError> {
        let decision = self
            .database
            .decide_archive_cancellation(job_id, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        Ok(match decision {
            xarchive_storage::ArchiveCancellationDecision::Cancelled => {
                CancelArbitration::Cancelled
            }
            xarchive_storage::ArchiveCancellationDecision::CommitWins => {
                CancelArbitration::CommitWins
            }
            xarchive_storage::ArchiveCancellationDecision::AlreadyDecided => {
                CancelArbitration::AlreadyDecided
            }
        })
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

#[cfg(test)]
mod tests;

/// Linux-only: the boundary tests exercise the real Unix coordination
/// adapter, because the point of the boundary is cross-process-equivalent
/// scope exclusivity. The boundary itself stays generic and compiles on
/// every target; only these probes are gated.
#[cfg(all(test, unix))]
mod session_boundary_tests;
