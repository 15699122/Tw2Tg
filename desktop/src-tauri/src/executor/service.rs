//! Application service and control loops: maps executor results onto the
//! archive flow and owns the bounded worker/runner scheduling boundary.
//!
//! ENG-15: separated from persistence and lifecycle so scheduling policy
//! changes are reviewed independently of storage and handle ownership.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread;

use xarchive_core::JobState;

use super::model::*;
use super::persistence::*;
use super::runtime::{JobExecutor, JobExecutorHandle};
/// Test-only adapter for comparing the future executor submit/query boundary
/// with the current synchronous `archive_tweet` command. It performs request
/// validation and Job identity derivation only; it does not own I/O.
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
#[derive(Clone)]
pub struct ArchiveApplicationService {
    pub(crate) executor: JobExecutorHandle,
}
#[derive(Debug)]
pub(super) struct JobRecord {
    pub(super) snapshot: JobSnapshot,
    pub(super) events: Vec<ExecutorEvent>,
    pub(super) cancellation: CancellationToken,
}

pub(super) enum Command {
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

pub(super) enum RunnerCommand {
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
pub(super) struct UnconfiguredExecutionFactory;

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
                if matches!(error, ExecutorError::Closed | ExecutorError::QueueFull) {
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

pub(super) fn run_runner(receiver: Receiver<RunnerCommand>, factory: Arc<dyn JobExecutionFactory>) {
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

pub(super) fn run_worker(receiver: Receiver<Command>, runner_sender: SyncSender<RunnerCommand>) {
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
    if current == JobState::Interrupted {
        return Ok(record.snapshot.clone());
    }
    current
        .transition_to(JobState::Interrupted)
        .map_err(|_| ExecutorError::InvalidTransition {
            from: current,
            to: JobState::Interrupted,
        })?;
    record.snapshot.state = JobState::Interrupted;
    record.events.push(ExecutorEvent::StateChanged {
        from: current,
        to: JobState::Interrupted,
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
