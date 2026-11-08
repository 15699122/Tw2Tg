//! Executor trait contracts and their in-memory implementations.
//!
//! `JobExecution`/`JobExecutionFactory` describe how one Job is executed;
//! `JobPersistence`/`JobDatabaseFactory` describe how executor state is persisted.
//! The in-memory implementations live here because they exist only to satisfy these
//! traits in tests and in the unconfigured factory.
//!
//! No admission, claim, drain, authorize or finalize ordering is expressed here: the
//! traits only name the operations, so moving them cannot reorder ADR-015. The
//! production `StorageJobPersistence` impl deliberately stays in `executor.rs` next
//! to the runtime that drives it.

use std::collections::HashMap;

use xarchive_core::JobState;
use xarchive_storage::Database;
use xarchive_workflow::AttemptIdentity;

use crate::executor::{
    ArchiveJobRequest, AttemptCreationError, CancelArbitration, CancellationToken, ExecutorError,
    ExecutorEvent, JobExecutionError, JobExecutionResult, JobSnapshot, SubmitResult,
};

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

    fn create_registered_attempt(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
        _registration: xarchive_workflow::WriterRegistration,
    ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
        self.create_for_attempt(identity, snapshot)
    }

    /// CROSS_PLATFORM_CHANGE_REQUIRED: session execution must explicitly bind
    /// its writer to the claimed attempt. Never fall back to a Job-only writer.
    fn create_for_attempt(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
        let _ = (identity, snapshot);
        Err(ExecutorError::Execution {
            error_code: "ATTEMPT_EXECUTION_NOT_CONFIGURED".to_owned(),
            error_message: "factory does not support attempt-owned execution".to_owned(),
            persistence_already_updated: false,
        }
        .into())
    }
}

pub trait JobPersistence {
    fn configure_v2_enrollment(&mut self, _enabled: bool) {}
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError>;
    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError>;
    fn list_recovery_candidates(&self) -> Result<Vec<JobSnapshot>, ExecutorError>;
    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError>;
    fn fail(
        &mut self,
        job_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError>;
    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError>;
    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError>;

    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError>;
    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError>;
    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError>;
    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError>;
    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError>;

    /// Atomic no-journal InternalV1 predecessor retirement. Non-Storage
    /// persistence implementations do not authorize successor takeover.
    fn claim_internal_v1_successor(
        &mut self,
        _job_id: &str,
        _expected_predecessor_attempt: u32,
    ) -> Result<Option<u32>, ExecutorError> {
        Ok(None)
    }

    fn internal_v1_attempt_phase(
        &self,
        _job_id: &str,
    ) -> Result<Option<Option<u8>>, ExecutorError> {
        Ok(None)
    }

    fn is_internal_v1_contract(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(false)
    }

    fn has_execution_spec(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(true)
    }

    /// Persist one state transition only while `attempt` still owns the Job.
    ///
    /// `expected` is the durable state the caller believes it is leaving. The
    /// write is a single conditional update of
    /// `(state = expected, attempt_count = attempt)`; it returns `false` when
    /// the row already moved (a newer attempt, a cancellation, or any other
    /// writer won), meaning this caller lost the arbitration and must stop
    /// writing states and events. The default implementation has no
    /// attempt-identity knowledge and always writes, matching in-memory and
    /// stub persistence whose callers do not claim attempts.
    fn persist_state_for_attempt(
        &mut self,
        job_id: &str,
        _attempt: u32,
        _expected: JobState,
        next: JobState,
    ) -> Result<bool, ExecutorError> {
        let mut snapshot = self.snapshot(job_id)?;
        snapshot.state = next;
        self.persist_state(&snapshot)?;
        Ok(true)
    }

    /// Atomically complete a job from `Downloaded` to `Complete`, recording
    /// `DownloadCompleted` and `Completed` events in the same transaction,
    /// fenced by `attempt`. Returns `Ok(false)` (idempotent no-op) when the
    /// job is already `Complete`. Returns `Err` when the attempt no longer
    /// owns the job or the job is in a state other than `Downloaded` or
    /// `Complete`.
    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError>;

    /// Whether a durable archive commit boundary (a PREPARED/COMMITTED
    /// recovery journal) already owns this job.
    ///
    /// The default implementation has no journal, so cancellation is never
    /// refused. Storage-backed persistence overrides this with the real
    /// journal check.
    fn archive_commit_started(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(false)
    }

    /// Atomically persist the cancellation decision against any durable
    /// archive commit boundary.
    ///
    /// The default implementation has no journal knowledge and persists
    /// unconditionally, matching pre-v2 behaviour. Storage-backed
    /// persistence re-checks the journal inside the same transaction that
    /// writes the cancelled state, so a PREPARED commit is never overwritten.
    fn persist_cancellation(&mut self, job_id: &str) -> Result<CancelArbitration, ExecutorError> {
        let mut snapshot = self.snapshot(job_id)?;
        snapshot.state = JobState::Cancelled;
        self.persist_state(&snapshot)?;
        Ok(CancelArbitration::Cancelled)
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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct InMemoryJobPersistence {
    // `pub(super)` keeps these readable exactly where they were readable before
    // the type moved out of `executor.rs` — the parent module and its test
    // modules — without widening them to the whole crate.
    pub(super) jobs: HashMap<String, JobSnapshot>,
    pub(super) events: HashMap<String, Vec<ExecutorEvent>>,
    pub(super) attempts: HashMap<String, u32>,
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

    fn list_recovery_candidates(&self) -> Result<Vec<JobSnapshot>, ExecutorError> {
        Ok(self
            .jobs
            .values()
            .filter(|job| job.state.is_active() || job.state == JobState::Interrupted)
            .cloned()
            .collect())
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

    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError> {
        self.record_event(job_id, ExecutorEvent::StateChanged { from, to })
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

    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
        Ok(self.attempts.get(job_id).copied().unwrap_or_default())
    }

    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError> {
        let current = self.snapshot(job_id)?;
        if current.state != from || self.current_attempt(job_id)? != expected_attempt {
            return Ok(None);
        }
        let attempt = self.begin_attempt(job_id)?;
        if !self.persist_state_for_attempt(job_id, attempt, from, to)? {
            return Ok(None);
        }
        Ok(Some(attempt))
    }

    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        Ok(self.attempts.get(job_id).copied().unwrap_or_default() == attempt)
    }

    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError> {
        let snapshot = self.snapshot(job_id)?;
        if snapshot.state == JobState::Complete {
            return Ok(snapshot);
        }
        if snapshot.state != JobState::Downloaded {
            return Err(ExecutorError::Persistence(
                "cannot complete job in non-Downloaded state".into(),
            ));
        }
        if !self.attempt_is_current(job_id, attempt)? {
            return Err(ExecutorError::Persistence("stale attempt".into()));
        }
        let next = snapshot
            .state
            .transition_to(JobState::Complete)
            .map_err(|_| ExecutorError::InvalidTransition {
                from: snapshot.state,
                to: JobState::Complete,
            })?;
        let updated = JobSnapshot {
            state: next,
            ..snapshot.clone()
        };
        self.persist_state(&updated)?;
        self.record_event(job_id, ExecutorEvent::DownloadCompleted { files_copied })?;
        self.record_event(
            job_id,
            ExecutorEvent::Completed {
                archive_directory: archive_directory.to_owned(),
            },
        )?;
        Ok(updated)
    }
}
