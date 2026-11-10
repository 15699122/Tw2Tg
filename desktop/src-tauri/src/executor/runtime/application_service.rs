//! `ArchiveApplicationService`: the archive-facing application service.
//!
//! Split out of `executor/runtime.rs` without changing behaviour, the module path
//! or the visibility of any item. `executor.rs` re-exports
//! `ArchiveApplicationService` through the parent, so every existing path keeps
//! resolving.
//!
//! Invariants that must not move (Plan §3):
//!
//! * the service delegates admission and cancellation to `JobExecutorHandle`; it
//!   never queues work itself and never admits twice for one submission.
//! * a durable attempt/state claim happens inside the handle, so a response here
//!   never precedes the claim or the teardown it implies.
//! * V2 enrollment stays closed unless it was explicitly provisioned; this module
//!   does not infer it from the environment.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;

use xarchive_core::JobState;

#[cfg(target_os = "linux")]
use super::super::ProductionCoordinator;
use super::super::{
    ArchiveJobRequest, CancelArbitration, CancellationToken, CommitRecoveryBatchResult,
    CommitRecoveryDecision, CommitRecoveryFacts, CommitRecoveryFactsProvider, ExecutorError,
    ExecutorEvent, JobExecution, JobExecutionResult, JobPersistence, JobSnapshot,
    SessionExecutionBoundary, StorageJobPersistence, SubmitResult, resolve_session_run,
};
use super::{JobExecutor, JobExecutorHandle};

#[derive(Clone)]
pub struct ArchiveApplicationService {
    pub(in crate::executor) executor: JobExecutorHandle,
    pub(in crate::executor) v2_enrollment: bool,
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
