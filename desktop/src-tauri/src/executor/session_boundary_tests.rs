//! Linux-only session-boundary probes.
//!
//! These exercise the real Unix coordination adapter, because the point of the
//! boundary is cross-process-equivalent scope exclusivity. The boundary itself
//! stays generic and compiles on every target; only these probes are gated.

use super::*;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use xarchive_workflow::coordination::LockScope;
use xarchive_workflow::coordination::unix::LinuxCoordinator;

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "tg-session-boundary-{}-{}-{}",
        label,
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
    ArchiveJobRequest {
        job_id: job_id.to_owned(),
        tweet_id: tweet_id.to_owned(),
        request_id: format!("session-request-{job_id}"),
        request_json: "{}".to_owned(),
        schema_version: 1,
    }
}

/// One-shot start/release channel pair a blocking probe execution waits on.
struct ExecutionGate {
    started: Sender<()>,
    release: Receiver<()>,
}

struct ProbeExecution {
    lock_dir: PathBuf,
    job_id: String,
    execute_scope_contended: Arc<Mutex<Vec<bool>>>,
    started: Option<Sender<()>>,
    release: Option<Receiver<()>>,
    failure: Option<(String, String)>,
    state_already_updated: bool,
}

struct CancelAfterExecute;

impl JobExecution for CancelAfterExecute {
    fn execute(
        &mut self,
        _job: &JobSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        cancellation.cancel();
        Err(JobExecutionError {
            error_code: "TRANSFER_CANCELLED".to_owned(),
            error_message: "cancel won before publication prepare".to_owned(),
            persistence_already_updated: false,
        })
    }
}

struct CancelAwarePersistence {
    inner: InMemoryJobPersistence,
    fail_called: Arc<AtomicBool>,
}

impl JobPersistence for CancelAwarePersistence {
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        self.inner.create_or_reuse(request)
    }
    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.inner.snapshot(job_id)
    }
    fn list_recovery_candidates(&self) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.inner.list_recovery_candidates()
    }
    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
        self.inner.persist_state(snapshot)
    }
    fn fail(
        &mut self,
        job_id: &str,
        code: &str,
        message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.fail_called.store(true, Ordering::Release);
        self.inner.fail(job_id, code, message)
    }
    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError> {
        self.inner.record_event(job_id, event)
    }
    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        self.inner.events(job_id)
    }
    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        self.inner.begin_attempt(job_id)
    }
    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
        self.inner.current_attempt(job_id)
    }
    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError> {
        self.inner.claim_attempt(job_id, expected, from, to)
    }
    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError> {
        self.inner.record_claimed_attempt_state(job_id, from, to)
    }
    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        self.inner.attempt_is_current(job_id, attempt)
    }
    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.inner
            .complete_terminal_for_attempt(job_id, attempt, directory, files_copied)
    }
    fn persist_cancellation(&mut self, job_id: &str) -> Result<CancelArbitration, ExecutorError> {
        let current = self.inner.snapshot(job_id)?;
        if current.state.is_terminal() {
            return Ok(CancelArbitration::AlreadyDecided);
        }
        let cancelled = JobSnapshot {
            state: JobState::Cancelled,
            ..current
        };
        self.inner.persist_state(&cancelled)?;
        Ok(CancelArbitration::Cancelled)
    }
}

impl JobExecution for ProbeExecution {
    fn execute(
        &mut self,
        job: &JobSnapshot,
        _cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        if let Some(release) = self.release.take() {
            let _ = release.recv();
        }
        if let Some((error_code, error_message)) = self.failure.take() {
            return Err(JobExecutionError {
                error_code,
                error_message,
                persistence_already_updated: false,
            });
        }
        // While the execution object runs (writers and publication are
        // inside it in production), a peer must see the Job scope as
        // contended: the session guard is held by the boundary thread.
        let peer = LinuxCoordinator::new(&self.lock_dir).map_err(|error| JobExecutionError {
            error_code: "PROBE_COORDINATOR".to_owned(),
            error_message: error.to_string(),
            persistence_already_updated: false,
        })?;
        let contended = matches!(
            peer.acquire(&LockScope::Job(self.job_id.clone())),
            Err(xarchive_workflow::coordination::LockError::Contended)
        ) && matches!(
            peer.acquire(&LockScope::Destination(
                PathBuf::from("/archive/Tweets").join(&job.tweet_id)
            )),
            Err(xarchive_workflow::coordination::LockError::Contended)
        );
        self.execute_scope_contended
            .lock()
            .expect("probe contention log")
            .push(contended);
        Ok(JobExecutionResult {
            files_copied: 1,
            archive_directory: format!("Tweets/{}", job.tweet_id),
            state_already_updated: self.state_already_updated,
            pending_publication: None,
            download_event_recorded: false,
        })
    }
}

struct ProbeFactory {
    lock_dir: PathBuf,
    create_scope_contended: Arc<Mutex<Vec<bool>>>,
    execute_scope_contended: Arc<Mutex<Vec<bool>>>,
    gate: Mutex<Option<ExecutionGate>>,
}

impl ProbeFactory {
    fn new(lock_dir: PathBuf) -> Self {
        Self {
            lock_dir,
            create_scope_contended: Arc::new(Mutex::new(Vec::new())),
            execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
            gate: Mutex::new(None),
        }
    }
}

impl JobExecutionFactory for ProbeFactory {
    fn create_for_attempt(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
        assert_eq!(identity.job_id(), snapshot.job_id);
        assert_eq!(identity.attempt(), 1, "factory sees the claimed attempt");
        self.create(identity.job_id(), snapshot).map_err(Into::into)
    }

    fn create(
        &self,
        job_id: &str,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        // Factory creation must happen after the durable claim, inside
        // the live session: a peer must already see the Job scope held.
        let peer =
            LinuxCoordinator::new(&self.lock_dir).map_err(|error| ExecutorError::Execution {
                error_code: "PROBE_COORDINATOR".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            })?;
        self.create_scope_contended
            .lock()
            .expect("probe contention log")
            .push(
                matches!(
                    peer.acquire(&LockScope::Job(job_id.to_owned())),
                    Err(xarchive_workflow::coordination::LockError::Contended)
                ) && matches!(
                    peer.acquire(&LockScope::Destination(
                        PathBuf::from("/archive/Tweets").join(&snapshot.tweet_id)
                    )),
                    Err(xarchive_workflow::coordination::LockError::Contended)
                ),
            );
        let gate = self.gate.lock().expect("probe gate").take();
        let (started, release) = match gate {
            Some(gate) => (Some(gate.started), Some(gate.release)),
            None => (None, None),
        };
        Ok(Box::new(ProbeExecution {
            lock_dir: self.lock_dir.clone(),
            job_id: job_id.to_owned(),
            execute_scope_contended: self.execute_scope_contended.clone(),
            started,
            release,
            failure: None,
            state_already_updated: false,
        }))
    }
}

/// In-memory persistence that logs, at every durable state write,
/// whether a peer coordinator sees the Job scope as contended, and that
/// can force the durable claim to report a lost race.
struct BoundaryProbePersistence {
    inner: InMemoryJobPersistence,
    lock_dir: PathBuf,
    persist_scope_contended: Arc<Mutex<Vec<(JobState, bool)>>>,
    force_claim_lost: bool,
    internal_v1_no_journal: bool,
    force_successor_claim_lost: bool,
}

impl BoundaryProbePersistence {
    fn new(lock_dir: PathBuf) -> Self {
        Self {
            inner: InMemoryJobPersistence::default(),
            lock_dir,
            persist_scope_contended: Arc::new(Mutex::new(Vec::new())),
            force_claim_lost: false,
            internal_v1_no_journal: false,
            force_successor_claim_lost: false,
        }
    }
}

impl JobPersistence for BoundaryProbePersistence {
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        self.inner.create_or_reuse(request)
    }
    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.inner.snapshot(job_id)
    }
    fn list_recovery_candidates(&self) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.inner.list_recovery_candidates()
    }
    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
        let peer = LinuxCoordinator::new(&self.lock_dir)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let contended = matches!(
            peer.acquire(&LockScope::Job(snapshot.job_id.clone())),
            Err(xarchive_workflow::coordination::LockError::Contended)
        ) && matches!(
            peer.acquire(&LockScope::Destination(
                PathBuf::from("/archive/Tweets").join(&snapshot.tweet_id)
            )),
            Err(xarchive_workflow::coordination::LockError::Contended)
        );
        self.persist_scope_contended
            .lock()
            .expect("probe contention log")
            .push((snapshot.state, contended));
        self.inner.persist_state(snapshot)
    }
    fn fail(
        &mut self,
        job_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.inner.fail(job_id, error_code, error_message)
    }
    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError> {
        self.inner.record_event(job_id, event)
    }
    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        self.inner.events(job_id)
    }
    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        self.inner.begin_attempt(job_id)
    }
    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
        self.inner.current_attempt(job_id)
    }
    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError> {
        if self.force_claim_lost {
            return Ok(None);
        }
        self.inner.claim_attempt(job_id, expected_attempt, from, to)
    }
    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError> {
        self.inner.record_claimed_attempt_state(job_id, from, to)
    }
    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        self.inner.attempt_is_current(job_id, attempt)
    }
    fn claim_internal_v1_successor(
        &mut self,
        job_id: &str,
        expected_predecessor_attempt: u32,
    ) -> Result<Option<u32>, ExecutorError> {
        if self.force_successor_claim_lost {
            return Ok(None);
        }
        self.inner
            .claim_internal_v1_successor(job_id, expected_predecessor_attempt)
    }
    fn internal_v1_attempt_phase(&self, job_id: &str) -> Result<Option<Option<u8>>, ExecutorError> {
        if self.internal_v1_no_journal {
            self.inner.snapshot(job_id)?;
            Ok(Some(None))
        } else {
            self.inner.internal_v1_attempt_phase(job_id)
        }
    }
    fn is_internal_v1_contract(&self, job_id: &str) -> Result<bool, ExecutorError> {
        if self.internal_v1_no_journal {
            self.inner.snapshot(job_id)?;
            Ok(true)
        } else {
            self.inner.is_internal_v1_contract(job_id)
        }
    }
    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError> {
        let snapshot = self.inner.snapshot(job_id)?;
        if snapshot.state == JobState::Complete {
            return Ok(snapshot);
        }
        if snapshot.state != JobState::Downloaded {
            return Err(ExecutorError::Persistence(
                "cannot complete job in non-Downloaded state".into(),
            ));
        }
        if !self.inner.attempt_is_current(job_id, attempt)? {
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
        // Write through `Self::persist_state` so the contention probe
        // logs the final Complete write under the held Job scope.
        self.persist_state(&updated)?;
        self.inner
            .record_event(job_id, ExecutorEvent::DownloadCompleted { files_copied })?;
        self.inner.record_event(
            job_id,
            ExecutorEvent::Completed {
                archive_directory: archive_directory.to_owned(),
            },
        )?;
        Ok(updated)
    }
}

/// Compile-time claim: the boundary must be `Send` so it can be moved
/// into the scheduler's `'static` thread together with its owned
/// persistence context and coordinator.
#[test]
fn the_boundary_is_send_so_it_can_move_into_the_scheduler_thread() {
    fn assert_send<T: Send>() {}
    assert_send::<SessionExecutionBoundary<InMemoryJobPersistence, LinuxCoordinator>>();
}

#[test]
fn cancellation_that_wins_before_prepare_is_not_replaced_by_execution_failure() {
    struct Factory {}
    impl JobExecutionFactory for Factory {
        fn create(
            &self,
            _job_id: &str,
            _snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            Err(ExecutorError::Execution {
                error_code: "ATTEMPT_REQUIRED".to_owned(),
                error_message: "attempt required".to_owned(),
                persistence_already_updated: false,
            })
        }
        fn create_for_attempt(
            &self,
            _identity: &AttemptIdentity,
            _snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            Ok(Box::new(CancelAfterExecute))
        }
    }

    let root = scratch("cancel-before-prepare");
    let lock_dir = root.join("locks");
    let executor = JobExecutor::with_capacity(4);
    let fail_called = Arc::new(AtomicBool::new(false));
    let mut inner = InMemoryJobPersistence::default();
    inner.create_or_reuse(&request("job-1", "tweet-1")).unwrap();
    let persistence = CancelAwarePersistence {
        inner,
        fail_called: fail_called.clone(),
    };
    let mut handle = executor.handle();
    handle.factory = Arc::new(Factory {});
    let mut boundary = SessionExecutionBoundary::new(
        ArchiveApplicationService {
            executor: handle,
            v2_enrollment: false,
        },
        persistence,
        LinuxCoordinator::new(&lock_dir).unwrap(),
        PathBuf::from("/archive"),
    );

    let outcome = boundary.execute_session("job-1").unwrap();
    assert!(
        matches!(outcome, SessionRunOutcome::Finished(ref job) if job.state == JobState::Cancelled)
    );
    assert!(
        !fail_called.load(Ordering::Acquire),
        "cancellation must not be overwritten by FAILED"
    );
    let events = boundary.persistence.inner.events("job-1").unwrap();
    assert!(!events.iter().any(|event| matches!(
        event,
        ExecutorEvent::DownloadFailed { .. }
            | ExecutorEvent::DownloadCompleted { .. }
            | ExecutorEvent::Completed { .. }
    )));
    drop(boundary);
    drop(executor);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn factory_failure_finalizes_the_claim_and_releases_admission_and_scopes() {
    let root = scratch("factory-failure");
    let lock_dir = root.join("locks");
    let executor = JobExecutor::with_capacity(4);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");
    let mut boundary = SessionExecutionBoundary::new(
        ArchiveApplicationService::new(&executor),
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    let outcome = boundary
        .execute_session("job-1")
        .expect("failure finalized");
    assert!(
        matches!(outcome, SessionRunOutcome::Finished(ref job) if job.state == JobState::Failed)
    );
    assert_eq!(
        boundary
            .persistence
            .current_attempt("job-1")
            .expect("attempt"),
        1
    );
    assert!(
        boundary
            .persistence
            .events("job-1")
            .expect("events")
            .iter()
            .any(
                |event| matches!(event, ExecutorEvent::DownloadFailed { error_message, .. }
            if error_message.contains("ATTEMPT_EXECUTION_NOT_CONFIGURED"))
            )
    );
    let peer = LinuxCoordinator::new(&lock_dir).expect("peer");
    let reservation =
        ExecutionReservation::acquire(&peer, "job-1", Path::new("/archive/Tweets/tweet-1"))
            .expect("failed factory released scopes");
    drop(reservation);
    assert!(
        boundary.service.executor.runner_try_admit(),
        "runner admission released"
    );
    boundary.service.executor.runner_release();
    drop(boundary);
    drop(executor);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_writer_drain_failure_is_terminal_and_never_completes_archive() {
    struct DrainFailureFactory;
    impl JobExecutionFactory for DrainFailureFactory {
        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(identity.job_id(), snapshot.job_id);
            assert_eq!(identity.attempt(), 1);
            self.create(&snapshot.job_id, snapshot).map_err(Into::into)
        }

        fn create(
            &self,
            job_id: &str,
            _snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            Ok(Box::new(ProbeExecution {
                lock_dir: PathBuf::new(),
                job_id: job_id.to_owned(),
                execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                started: None,
                release: None,
                failure: Some((
                    "ARCHIVE_WRITER_DRAIN_FAILED".to_owned(),
                    "Sidecar shutdown incomplete: child still running".to_owned(),
                )),
                state_already_updated: false,
            }))
        }
    }

    let root = scratch("writer-drain-failure");
    let lock_dir = root.join("locks");
    let executor = JobExecutor::with_capacity(4);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");
    let mut handle = executor.handle();
    handle.factory = Arc::new(DrainFailureFactory);
    let mut boundary = SessionExecutionBoundary::new(
        ArchiveApplicationService {
            executor: handle,
            v2_enrollment: false,
        },
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );

    let outcome = boundary.execute_session("job-1").expect("terminal result");
    let SessionRunOutcome::Finished(snapshot) = outcome else {
        panic!("drain failure must be terminal, not rescheduled: {outcome:?}");
    };
    assert_eq!(snapshot.state, JobState::Failed);
    let events = boundary.persistence.events("job-1").expect("events");
    assert!(
        events.iter().any(|event| matches!(
            event,
            ExecutorEvent::DownloadFailed { error_code, error_message }
                if error_code == "EXECUTOR_WORKER_FAILED"
                    && error_message.contains("job execution error [ARCHIVE_WRITER_DRAIN_FAILED]")
                    && error_message.contains("Sidecar shutdown incomplete")
        )),
        "drain failure must be terminal and preserve diagnostic detail: {events:?}"
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            ExecutorEvent::DownloadCompleted { .. } | ExecutorEvent::Completed { .. }
        )),
        "drain failure must never enter completion: {events:?}"
    );
    assert_eq!(
        boundary
            .persistence
            .current_attempt("job-1")
            .expect("attempt"),
        1
    );
    drop(boundary);
    drop(executor);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn stale_attempt_success_records_no_completion_event_or_final_state() {
    struct StaleSuccessFactory;
    impl JobExecutionFactory for StaleSuccessFactory {
        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(identity.attempt(), 1);
            self.create(&snapshot.job_id, snapshot).map_err(Into::into)
        }

        fn create(
            &self,
            job_id: &str,
            _snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            Ok(Box::new(ProbeExecution {
                lock_dir: PathBuf::new(),
                job_id: job_id.to_owned(),
                execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                started: None,
                release: None,
                failure: None,
                state_already_updated: false,
            }))
        }
    }

    struct StaleAfterExecutePersistence {
        inner: InMemoryJobPersistence,
        job_id: String,
    }
    impl JobPersistence for StaleAfterExecutePersistence {
        fn create_or_reuse(
            &mut self,
            request: &ArchiveJobRequest,
        ) -> Result<SubmitResult, ExecutorError> {
            self.inner.create_or_reuse(request)
        }
        fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
            self.inner.snapshot(job_id)
        }
        fn list_recovery_candidates(&self) -> Result<Vec<JobSnapshot>, ExecutorError> {
            self.inner.list_recovery_candidates()
        }
        fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
            self.inner.persist_state(snapshot)
        }
        fn fail(
            &mut self,
            job_id: &str,
            error_code: &str,
            error_message: &str,
        ) -> Result<JobSnapshot, ExecutorError> {
            self.inner.fail(job_id, error_code, error_message)
        }
        fn record_event(
            &mut self,
            job_id: &str,
            event: ExecutorEvent,
        ) -> Result<(), ExecutorError> {
            self.inner.record_event(job_id, event)
        }
        fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
            self.inner.events(job_id)
        }
        fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
            self.inner.begin_attempt(job_id)
        }
        fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
            self.inner.current_attempt(job_id)
        }
        fn claim_attempt(
            &mut self,
            job_id: &str,
            expected_attempt: u32,
            from: JobState,
            to: JobState,
        ) -> Result<Option<u32>, ExecutorError> {
            self.inner.claim_attempt(job_id, expected_attempt, from, to)
        }
        fn record_claimed_attempt_state(
            &mut self,
            job_id: &str,
            from: JobState,
            to: JobState,
        ) -> Result<(), ExecutorError> {
            self.inner.record_claimed_attempt_state(job_id, from, to)
        }
        fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
            // The attempt was current while its execution ran; a newer
            // attempt won before terminal writes, so the stale result must
            // be suppressed without persisting completion.
            if job_id == self.job_id && attempt == 1 {
                return Ok(false);
            }
            self.inner.attempt_is_current(job_id, attempt)
        }
        fn complete_terminal_for_attempt(
            &mut self,
            job_id: &str,
            attempt: u32,
            archive_directory: &str,
            files_copied: u32,
        ) -> Result<JobSnapshot, ExecutorError> {
            self.inner.complete_terminal_for_attempt(
                job_id,
                attempt,
                archive_directory,
                files_copied,
            )
        }
    }

    let root = scratch("stale-success-suppression");
    let lock_dir = root.join("locks");
    let executor = JobExecutor::with_capacity(4);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");
    let mut handle = executor.handle();
    handle.factory = Arc::new(StaleSuccessFactory);
    let stale_persistence = StaleAfterExecutePersistence {
        inner: persistence,
        job_id: "job-1".to_owned(),
    };
    let mut boundary = SessionExecutionBoundary::new(
        ArchiveApplicationService {
            executor: handle,
            v2_enrollment: false,
        },
        stale_persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    let outcome = boundary.execute_session("job-1").expect("stale suppressed");
    let SessionRunOutcome::Finished(snapshot) = outcome else {
        panic!("stale success must not reschedule: {outcome:?}");
    };
    assert_ne!(snapshot.state, JobState::Complete);
    let events = boundary.persistence.inner.events("job-1").expect("events");
    assert!(
        !events.iter().any(|event| matches!(
            event,
            ExecutorEvent::DownloadCompleted { .. } | ExecutorEvent::Completed { .. }
        )),
        "stale success must record no completion: {events:?}"
    );
    drop(boundary);
    drop(executor);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn fenced_publication_records_download_completed_and_skips_duplicate_complete() {
    /// Execution reports `state_already_updated: true`, meaning fenced
    /// publication already committed inside `execute_archive_context`. The
    /// session must record `DownloadCompleted` (best-effort, after the
    /// durable publication commit) but must not call `complete_persisted`
    /// again, avoiding a double publication and a duplicate `Completed`.
    struct FencedFactory;
    impl JobExecutionFactory for FencedFactory {
        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(identity.job_id(), snapshot.job_id);
            assert_eq!(identity.attempt(), 1, "factory sees the claimed attempt");
            self.create(&snapshot.job_id, snapshot).map_err(Into::into)
        }
        fn create(
            &self,
            job_id: &str,
            _snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            Ok(Box::new(ProbeExecution {
                lock_dir: PathBuf::new(),
                job_id: job_id.to_owned(),
                execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                started: None,
                release: None,
                failure: None,
                state_already_updated: true,
            }))
        }
    }

    let root = scratch("fenced-publication");
    let lock_dir = root.join("locks");
    let executor = JobExecutor::with_capacity(4);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");
    let mut handle = executor.handle();
    handle.factory = Arc::new(FencedFactory);
    let mut boundary = SessionExecutionBoundary::new(
        ArchiveApplicationService {
            executor: handle,
            v2_enrollment: false,
        },
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    let outcome = boundary.execute_session("job-1").expect("terminal result");
    let SessionRunOutcome::Finished(snapshot) = outcome else {
        panic!("fenced publication must be terminal: {outcome:?}");
    };
    assert_eq!(
        snapshot.state,
        JobState::Complete,
        "complete_persisted must advance state even when state_already_updated"
    );
    let events = boundary.persistence.events("job-1").expect("events");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ExecutorEvent::DownloadCompleted { .. })),
        "DownloadCompleted must be recorded after fenced publication: {events:?}"
    );
    let completed_events: Vec<_> = events
        .iter()
        .filter(|event| matches!(event, ExecutorEvent::Completed { .. }))
        .collect();
    assert_eq!(
        completed_events.len(),
        1,
        "Completed must be recorded exactly once (no duplicate): {events:?}"
    );
    assert_eq!(
        boundary
            .persistence
            .current_attempt("job-1")
            .expect("attempt"),
        1
    );
    drop(boundary);
    drop(executor);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn non_fenced_execution_records_download_completed_and_completes() {
    /// Execution reports `state_already_updated: false`, meaning no fenced
    /// publication has committed. The session must record `DownloadCompleted`
    /// and then call `complete_persisted`, advancing the Job to `Complete`.
    struct NonFencedFactory;
    impl JobExecutionFactory for NonFencedFactory {
        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(identity.job_id(), snapshot.job_id);
            assert_eq!(identity.attempt(), 1, "factory sees the claimed attempt");
            self.create(&snapshot.job_id, snapshot).map_err(Into::into)
        }
        fn create(
            &self,
            job_id: &str,
            _snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            Ok(Box::new(ProbeExecution {
                lock_dir: PathBuf::new(),
                job_id: job_id.to_owned(),
                execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                started: None,
                release: None,
                failure: None,
                state_already_updated: false,
            }))
        }
    }

    let root = scratch("non-fenced-completion");
    let lock_dir = root.join("locks");
    let executor = JobExecutor::with_capacity(4);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");
    let mut handle = executor.handle();
    handle.factory = Arc::new(NonFencedFactory);
    let mut boundary = SessionExecutionBoundary::new(
        ArchiveApplicationService {
            executor: handle,
            v2_enrollment: false,
        },
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    let outcome = boundary.execute_session("job-1").expect("terminal result");
    let SessionRunOutcome::Finished(snapshot) = outcome else {
        panic!("non-fenced execution must be terminal: {outcome:?}");
    };
    assert_eq!(
        snapshot.state,
        JobState::Complete,
        "complete_persisted must advance state when not state_already_updated"
    );
    let events = boundary.persistence.events("job-1").expect("events");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ExecutorEvent::DownloadCompleted { .. })),
        "DownloadCompleted must be recorded before complete_persisted: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ExecutorEvent::Completed { .. })),
        "Completed must be recorded after complete_persisted: {events:?}"
    );
    assert_eq!(
        boundary
            .persistence
            .current_attempt("job-1")
            .expect("attempt"),
        1
    );
    drop(boundary);
    drop(executor);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn inline_runner_operation_is_static_runs_on_runner_and_drops_before_admission() {
    use std::sync::atomic::AtomicUsize;

    fn assert_send_static<T: Send + 'static>() {}
    assert_send_static::<Box<dyn FnOnce() + Send + 'static>>();

    let executor = JobExecutor::with_capacity(4);
    let handle = executor.handle();
    assert!(handle.runner_try_admit());
    let admission = handle.owned_runner_admission();
    let observed_thread = Arc::new(Mutex::new(None::<thread::ThreadId>));
    let operation_thread = observed_thread.clone();
    let captured_dropped = Arc::new(AtomicUsize::new(0));
    let captured_counter = captured_dropped.clone();
    let local_dropped = Arc::new(AtomicUsize::new(0));
    let local_counter = local_dropped.clone();
    let operation_handle = handle.clone();
    struct CapturedDropProbe(Arc<AtomicUsize>);
    impl Drop for CapturedDropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::AcqRel);
        }
    }
    let capture = CapturedDropProbe(captured_counter);
    let result = handle
        .run_operation(admission, move || {
            let _capture = capture;
            *operation_thread.lock().expect("thread observation") = Some(thread::current().id());
            struct DropProbe(Arc<AtomicUsize>);
            impl Drop for DropProbe {
                fn drop(&mut self) {
                    self.0.fetch_add(1, Ordering::AcqRel);
                }
            }
            let probe = DropProbe(local_counter);
            let in_flight = operation_handle.runner_in_flight.load(Ordering::Acquire);
            drop(probe);
            Ok((in_flight, thread::current().name().map(str::to_owned)))
        })
        .expect("runner operation");

    assert_eq!(result.0, 1, "admission remains held during the task");
    assert_eq!(result.1.as_deref(), Some("xarchive-job-runner"));
    assert_eq!(local_dropped.load(Ordering::Acquire), 1);
    assert_eq!(
        captured_dropped.load(Ordering::Acquire),
        1,
        "FnOnce captures must be destroyed before the result is returned"
    );
    assert_ne!(
        *observed_thread.lock().expect("thread observation"),
        Some(thread::current().id()),
        "operation must execute on the runner, not the caller"
    );
    assert_eq!(
        handle.runner_in_flight.load(Ordering::Acquire),
        0,
        "admission releases after operation completion"
    );
    drop(handle);
    executor.shutdown().expect("shutdown");
}

/// V1-07: replacing an executor must not overlap with an active writer.
///
/// `shutdown_in_place` joins the runner thread, and the runner is what holds
/// the live session and its registered writer. While a contained worker is
/// running inside an admitted runner operation, a concurrent shutdown must
/// block until that operation finishes, and a successor must not be
/// constructed before the old runner has exited.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn executor_shutdown_joins_active_writer_before_successor_runs() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));

    let executor = JobExecutor::with_capacity(4);
    let handle = executor.handle();
    // Admit the runner operation; the single runner slot is now occupied.
    assert!(handle.runner_try_admit(), "the runner admits one operation");
    let admission = handle.owned_runner_admission();
    assert!(
        !handle.runner_try_admit(),
        "a successor operation must not be admitted while the runner is occupied"
    );

    // Signals between threads.
    let writer_started = Arc::new(AtomicBool::new(false));
    let writer_finished = Arc::new(AtomicBool::new(false));
    let shutdown_complete = Arc::new(AtomicBool::new(false));
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let started_flag = writer_started.clone();
    let finished_flag = writer_finished.clone();
    let shutdown_flag = shutdown_complete.clone();

    let group_name = format!("tw2tg-v1-07-{}", std::process::id());
    let group_path = parent.join(&group_name);
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let parent_owned = parent.clone();
    let group_owned = group_name.clone();

    let handle_for_op = handle.clone();
    // Thread 1: the writer runs inside an admitted runner operation, so the
    // admission is held for the writer's entire lifetime.
    let operation_thread = std::thread::spawn(move || {
        let result = handle_for_op.run_operation(admission, move || {
            let owner = xarchive_workflow::linux_containment::LinuxContainment::create(
                &parent_owned,
                &group_owned,
            )
            .expect("create the writer domain inside the delegated cgroup");
            let mut supervisor = xarchive_sidecar_supervisor::SidecarSupervisor::spawn_contained(
                owner,
                Path::new("/usr/bin/python3"),
                &launcher,
                Path::new("/usr/bin/python3"),
                &["-u", "-c", "import sys; sys.stdin.read()"],
            )
            .expect("contained writer starts");
            started_flag.store(true, Ordering::Release);
            // Hold the writer open until the test releases it.
            let _ = release_rx.recv_timeout(Duration::from_secs(20));
            supervisor
                .shutdown()
                .expect("the writer drains before the join");
            finished_flag.store(true, Ordering::Release);
            Ok(())
        });
        result.expect("runner operation completed");
    });

    // Wait for the writer to start inside the runner operation.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !writer_started.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline, "the writer never started");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        group_path.exists(),
        "the active writer holds a real cgroup domain"
    );

    // Thread 2: shutdown must block on runner.join() while the writer is active.
    let shutdown = std::thread::spawn(move || {
        let mut executor = executor;
        let result = executor.shutdown_in_place();
        shutdown_flag.store(true, Ordering::Release);
        result
    });

    // Shutdown must not complete while the writer is still active.
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !writer_finished.load(Ordering::Acquire),
        "the writer must still be running until the test releases it"
    );
    assert!(
        !shutdown_complete.load(Ordering::Acquire),
        "shutdown must not complete while the runner holds the active writer"
    );

    // A successor must not be admitted while the runner holds the active writer.
    assert!(
        !handle.runner_try_admit(),
        "no successor may be admitted while the runner holds the active writer"
    );

    // Release the writer and let the operation complete.
    release_tx.send(()).expect("release the writer");
    operation_thread.join().expect("runner operation joins");

    // Shutdown must now complete.
    let shutdown_result = shutdown.join().expect("shutdown thread joins");
    assert!(
        shutdown_result.is_ok(),
        "shutdown must complete after writer drains"
    );
    assert!(
        shutdown_complete.load(Ordering::Acquire),
        "shutdown completion flag must be set"
    );

    assert!(
        writer_finished.load(Ordering::Acquire),
        "the writer must drain before the runner exits"
    );
    assert!(
        !group_path.exists(),
        "the old writer domain must be gone before a successor runs"
    );

    // A successor can now be admitted after the old runner has exited.
    assert!(
        handle.runner_try_admit(),
        "a successor operation may be admitted after the old runner exits"
    );
}

/// V1-07 production acceptance: replacing the *runtime* executor must not
/// complete while an admitted runner holds a live registered writer, and a
/// successor operation must run on the replacement generation only after the
/// old writer has drained and its domain is gone.
///
/// Unlike `executor_shutdown_joins_active_writer_before_successor_runs`, this
/// drives the real `RuntimeState::replace_executor` path and executes a real
/// successor operation on the new executor.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn runtime_replacement_joins_active_writer_then_runs_successor() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let delegated_parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));

    let root = std::env::temp_dir().join(format!(
        "xarchive-v1-07-runtime-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    std::fs::create_dir_all(&root).unwrap();
    // `replace_executor` re-validates launch paths, which reject `..`
    // segments by design, so the fixture passes a canonical path.
    let launcher = std::fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
    )
    .expect("the cgroup launcher must resolve to a canonical path");
    let containment = LinuxExecutionContainment {
        delegated_parent: delegated_parent.clone(),
        python: PathBuf::from("/usr/bin/python3"),
        launcher: launcher.clone(),
    };
    let mut state =
        crate::runtime::RuntimeState::initialize_with_containment(root.clone(), Some(containment));
    assert!(
        state.database_ready,
        "runtime replacement acceptance needs a usable database"
    );

    let replacement_config = state.executor.config().clone();
    let handle = state.executor.executor.handle();
    assert!(
        handle.runner_try_admit(),
        "the old runner admits one operation"
    );
    let admission = handle.owned_runner_admission();

    let writer_started = Arc::new(AtomicBool::new(false));
    let writer_finished = Arc::new(AtomicBool::new(false));
    let replacement_done = Arc::new(AtomicBool::new(false));
    let successor_ran = Arc::new(AtomicBool::new(false));
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();

    let group_name = format!("tw2tg-v1-07-runtime-{}", std::process::id());
    let group_path = delegated_parent.join(&group_name);
    let started_flag = writer_started.clone();
    let finished_flag = writer_finished.clone();
    let group_owned = group_name.clone();
    let parent_owned = delegated_parent.clone();

    let operation_thread = std::thread::spawn(move || {
        handle
            .run_operation(admission, move || {
                let owner = xarchive_workflow::linux_containment::LinuxContainment::create(
                    &parent_owned,
                    &group_owned,
                )
                .expect("create the registered writer domain");
                let mut supervisor =
                    xarchive_sidecar_supervisor::SidecarSupervisor::spawn_contained(
                        owner,
                        Path::new("/usr/bin/python3"),
                        &launcher,
                        Path::new("/usr/bin/python3"),
                        &["-u", "-c", "import sys; sys.stdin.read()"],
                    )
                    .expect("contained writer starts");
                started_flag.store(true, Ordering::Release);
                let _ = release_rx.recv_timeout(Duration::from_secs(30));
                supervisor.shutdown().expect("the writer drains");
                finished_flag.store(true, Ordering::Release);
                Ok(())
            })
            .expect("the old runner operation completes")
    });

    let deadline = Instant::now() + Duration::from_secs(15);
    while !writer_started.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "the registered writer never started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(group_path.exists(), "the writer holds a real cgroup domain");

    let done_flag = replacement_done.clone();
    let successor_flag = successor_ran.clone();
    let replacement_thread = std::thread::spawn(move || {
        let result = state.replace_executor(replacement_config);
        if result.is_err() {
            // A failed replacement must not be reported as "completed while
            // the writer was active"; the error is surfaced after the join.
            return result;
        }
        done_flag.store(true, Ordering::Release);
        // Successor work must run on the replacement generation.
        let successor_handle = state.executor.executor.handle();
        assert!(
            successor_handle.runner_try_admit(),
            "the replacement runner admits successor work"
        );
        let successor_admission = successor_handle.owned_runner_admission();
        successor_handle
            .run_operation(successor_admission, move || {
                successor_flag.store(true, Ordering::Release);
                Ok(())
            })
            .expect("successor operation completes on the replacement executor");
        result
    });

    std::thread::sleep(Duration::from_millis(400));
    assert!(
        !writer_finished.load(Ordering::Acquire),
        "the writer must still be alive until the test releases it"
    );
    assert!(
        !replacement_done.load(Ordering::Acquire),
        "replacement must not complete while the runner holds the active writer"
    );
    assert!(
        !successor_ran.load(Ordering::Acquire),
        "no successor may run before the old writer drains"
    );

    release_tx.send(()).expect("release the writer");
    operation_thread.join().expect("old runner operation joins");
    let replacement_result = replacement_thread.join().expect("replacement thread joins");
    replacement_result.expect("runtime replacement succeeds after the drain");

    assert!(
        writer_finished.load(Ordering::Acquire),
        "the old writer must drain before replacement completes"
    );
    assert!(
        !group_path.exists(),
        "the old writer domain must be gone before the successor runs"
    );
    assert!(
        successor_ran.load(Ordering::Acquire),
        "a successor operation must run on the replacement executor"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn inline_runner_operation_error_drops_capture_before_releasing_admission() {
    use std::sync::atomic::AtomicUsize;

    let executor = JobExecutor::with_capacity(4);
    let handle = executor.handle();
    assert!(handle.runner_try_admit());
    let admission = handle.owned_runner_admission();
    let in_flight_during_drop = Arc::new(AtomicUsize::new(usize::MAX));
    let observed = in_flight_during_drop.clone();
    let operation_handle = handle.clone();
    struct OwnerProbe {
        counter: Arc<AtomicUsize>,
        admission: Arc<AtomicUsize>,
    }
    impl Drop for OwnerProbe {
        fn drop(&mut self) {
            self.counter
                .store(self.admission.load(Ordering::Acquire), Ordering::Release);
        }
    }
    let captured_owner = OwnerProbe {
        counter: observed,
        admission: operation_handle.runner_in_flight.clone(),
    };
    let result = handle.run_operation(admission, move || {
        let _owner = captured_owner;
        Err::<(), _>(ExecutorError::Persistence("probe error".to_owned()))
    });
    assert!(matches!(
        result,
        Err(ExecutorError::Persistence(message)) if message == "probe error"
    ));
    assert_eq!(in_flight_during_drop.load(Ordering::Acquire), 1);
    assert_eq!(handle.runner_in_flight.load(Ordering::Acquire), 0);
    drop(handle);
    executor.shutdown().expect("shutdown");
}

#[test]
fn session_boundary_execute_on_runner_runs_session_on_named_runner() {
    let root = scratch("boundary-on-runner");
    let lock_dir = root.join("locks");
    let executor =
        JobExecutor::with_capacity_and_factory(4, Arc::new(ProbeFactory::new(lock_dir.clone())));
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
    persistence
        .inner
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create job");
    let boundary = SessionExecutionBoundary::new(
        service,
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );

    let outcome = boundary
        .execute_on_runner("job-1".to_owned())
        .expect("runner-owned boundary");
    assert!(matches!(
        outcome,
        SessionRunOutcome::Finished(JobSnapshot {
            state: JobState::Complete,
            ..
        })
    ));
    assert_eq!(
        executor.handle().runner_in_flight.load(Ordering::Acquire),
        0,
        "runner admission releases after session finalization"
    );
    executor.shutdown().expect("shutdown");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn startup_cleanup_pending_owner_crosses_the_boundary_and_resolves_same_owner() {
    use xarchive_sidecar_supervisor::SidecarSupervisor;
    use xarchive_sidecar_supervisor::StartupFailureOwner;
    use xarchive_sidecar_supervisor::SupervisorError;

    // The factory hands the admitted attempt a move-only cleanup owner
    // instead of a flattened string error; only one attempt may take it.
    struct PendingStartupFactory(std::sync::Mutex<Option<SidecarSupervisor>>);

    impl JobExecutionFactory for PendingStartupFactory {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("session boundary must use attempt-owned creation");
        }

        fn create_for_attempt(
            &self,
            _: &AttemptIdentity,
            _: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            let supervisor = self
                .0
                .lock()
                .expect("factory lock")
                .take()
                .expect("single attempt");
            Err(AttemptCreationError::StartupCleanupPending(
                StartupFailureOwner::new(
                    supervisor,
                    SupervisorError::HandshakeFailed("sidecar startup fixture".to_owned()),
                ),
            ))
        }
    }

    let root = scratch("boundary-startup-owner");
    let lock_dir = root.join("locks");
    let supervisor =
        SidecarSupervisor::spawn("/bin/sh", &["-c", "exit 0"]).expect("fixture supervisor");
    let executor = JobExecutor::with_capacity_and_factory(
        4,
        Arc::new(PendingStartupFactory(std::sync::Mutex::new(Some(
            supervisor,
        )))),
    );
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
    persistence
        .inner
        .create_or_reuse(&request("job-startup-owner", "tweet-startup-owner"))
        .expect("create job");
    let boundary = SessionExecutionBoundary::new(
        service,
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );

    let outcome = boundary
        .execute_on_runner("job-startup-owner".to_owned())
        .expect("runner-owned boundary");
    // The retained owner carries the original startup failure instead of
    // a flattened string, and the durable decision stays attached to it.
    let SessionRunOutcome::StartupCleanupPending { job_id, .. } = &outcome else {
        panic!("factory owner must cross the boundary, got {outcome:?}");
    };
    assert_eq!(job_id, "job-startup-owner");
    assert!(
        format!("{outcome:?}").contains("sidecar startup fixture"),
        "the retained owner keeps the original startup failure"
    );

    // Crossing the outcome boundary retries cleanup on the same
    // supervisor; the attached FAILED decision is only returned after
    // that retry proves local cleanup.
    let resolved = resolve_session_run(outcome).expect("same-owner cleanup retry");
    assert!(matches!(
        resolved,
        SessionRunOutcome::Finished(JobSnapshot {
            state: JobState::Failed,
            ..
        })
    ));
    assert_eq!(
        executor.handle().runner_in_flight.load(Ordering::Acquire),
        0,
        "runner admission releases after session finalization"
    );
    executor.shutdown().expect("shutdown");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn inline_runner_operation_drops_owner_before_admission_on_success() {
    let executor = JobExecutor::with_capacity(4);
    let handle = executor.handle();
    assert!(handle.runner_try_admit());
    let admission = handle.owned_runner_admission();
    let observed = Arc::new(AtomicUsize::new(usize::MAX));
    let counter = observed.clone();
    let in_flight = handle.runner_in_flight.clone();
    struct Owner(Arc<AtomicUsize>, Arc<AtomicUsize>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0
                .store(self.1.load(Ordering::Acquire), Ordering::Release);
        }
    }
    let owner = Owner(counter, in_flight);
    handle
        .run_operation(admission, move || {
            let _owner = owner;
            Ok(())
        })
        .expect("runner operation");
    assert_eq!(observed.load(Ordering::Acquire), 1);
    assert_eq!(handle.runner_in_flight.load(Ordering::Acquire), 0);
    executor.shutdown().expect("shutdown");
}

#[test]
fn inline_runner_operation_panic_closes_reply_and_releases_admission() {
    let executor = JobExecutor::with_capacity(4);
    let handle = executor.handle();
    assert!(handle.runner_try_admit());
    let admission = handle.owned_runner_admission();
    let caller_handle = handle.clone();
    let panic_owner_dropped = Arc::new(AtomicBool::new(false));
    let owner_flag = panic_owner_dropped.clone();
    let caller = thread::spawn(move || {
        caller_handle.run_operation(admission, move || -> Result<(), ExecutorError> {
            struct PanicOwner(Arc<AtomicBool>);
            impl Drop for PanicOwner {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::Release);
                }
            }
            let _owner = PanicOwner(owner_flag);
            panic!("runner operation panic probe");
        })
    });
    let result = caller.join().expect("caller thread survives runner panic");
    assert!(matches!(
        result,
        Err(ExecutorError::Execution {
            error_code,
            error_message,
            ..
        }) if error_code == "RUNNER_OPERATION_PANICKED"
            && error_message == "runner operation panic probe"
    ));
    assert!(panic_owner_dropped.load(Ordering::Acquire));
    assert_eq!(handle.runner_in_flight.load(Ordering::Acquire), 0);
    assert!(
        handle.runner_try_admit(),
        "runner remains usable after task panic"
    );
    let admission = handle.owned_runner_admission();
    assert_eq!(
        handle
            .run_operation(admission, || Ok::<_, ExecutorError>(()))
            .unwrap(),
        ()
    );
    drop(handle);
    executor.shutdown().expect("shutdown");
}

#[test]
fn a_session_holds_both_scopes_across_execution_and_finalization() {
    let root = scratch("happy");
    let lock_dir = root.join("locks");
    let factory = ProbeFactory::new(lock_dir.clone());
    let create_contended = factory.create_scope_contended.clone();
    let execute_contended = factory.execute_scope_contended.clone();
    let executor = JobExecutor::with_capacity_and_factory(4, Arc::new(factory));
    let service = ArchiveApplicationService::new(&executor);

    let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
    persistence
        .inner
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create job");
    let persist_contended = persistence.persist_scope_contended.clone();

    let mut boundary = SessionExecutionBoundary::new(
        service,
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    let outcome = boundary.execute_session("job-1").expect("session run");
    let SessionRunOutcome::Finished(snapshot) = outcome else {
        panic!("expected a finished session run, got {outcome:?}");
    };
    assert_eq!(snapshot.state, JobState::Complete);

    // Every durable state write — including the final `Complete`
    // finalization write — happened while a peer saw the Job scope held.
    let persisted = persist_contended.lock().expect("probe log");
    assert!(!persisted.is_empty(), "the run must persist durable state");
    assert!(
        persisted.iter().all(|(_, contended)| *contended),
        "every state write must occur under the held Job scope: {persisted:?}"
    );
    assert!(
        persisted
            .iter()
            .any(|(state, contended)| *state == JobState::Complete && *contended),
        "finalization must occur under the held Job scope: {persisted:?}"
    );
    drop(persisted);
    // The execution object observed the guard, and the factory was
    // invoked only after the claim established the session.
    let executed = execute_contended.lock().expect("probe log");
    assert!(
        !executed.is_empty() && executed.iter().all(|contended| *contended),
        "the execution object must run under the held Job scope: {executed:?}"
    );
    drop(executed);
    let created = create_contended.lock().expect("probe log");
    assert!(
        !created.is_empty() && created.iter().all(|contended| *contended),
        "factory creation must happen under the held Job scope: {created:?}"
    );
    drop(created);
    let events = boundary.persistence.events("job-1").expect("events");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ExecutorEvent::DownloadFailed { .. })),
        "a successful run must not record a failure event: {events:?}"
    );
    // After the run both scopes are released for a successor.
    drop(boundary);
    let peer = LinuxCoordinator::new(&lock_dir).expect("peer coordinator");
    ExecutionReservation::acquire(&peer, "job-1", Path::new("/archive/Tweets/tweet-1"))
        .expect("scopes released after the session run");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_live_peer_makes_the_run_reschedulable_without_any_durable_write() {
    let root = scratch("contended");
    let lock_dir = root.join("locks");
    let executor =
        JobExecutor::with_capacity_and_factory(4, Arc::new(ProbeFactory::new(lock_dir.clone())));
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
    persistence
        .inner
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create job");
    let persist_contended = persistence.persist_scope_contended.clone();

    let peer_coordinator = LinuxCoordinator::new(&lock_dir).expect("peer coordinator");
    let peer_guard = peer_coordinator
        .acquire(&LockScope::Job("job-1".to_owned()))
        .expect("peer holds the job scope");

    let mut boundary = SessionExecutionBoundary::new(
        service,
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    let outcome = boundary
        .execute_session("job-1")
        .expect("contention is an outcome, not an error");
    assert!(
        matches!(
            outcome,
            SessionRunOutcome::Reschedulable {
                ref job_id,
                reason: SessionRescheduleReason::Contended,
            } if job_id == "job-1"
        ),
        "expected contended reschedule, got {outcome:?}"
    );
    let snapshot = boundary.persistence.snapshot("job-1").expect("snapshot");
    assert_eq!(
        snapshot.state,
        JobState::Queued,
        "a contended run must not advance the Job"
    );
    assert_eq!(
        boundary
            .persistence
            .current_attempt("job-1")
            .expect("attempt"),
        0,
        "a contended run must not claim an attempt"
    );
    assert!(
        boundary
            .persistence
            .events("job-1")
            .expect("events")
            .is_empty(),
        "a contended run must not persist events"
    );
    assert!(
        persist_contended.lock().expect("probe log").is_empty(),
        "a contended run must not reach any durable state write"
    );
    drop(peer_guard);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_lost_claim_is_reschedulable_releases_scopes_and_persists_no_failure() {
    let root = scratch("lost-claim");
    let lock_dir = root.join("locks");
    let executor =
        JobExecutor::with_capacity_and_factory(4, Arc::new(ProbeFactory::new(lock_dir.clone())));
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
    persistence.force_claim_lost = true;
    persistence
        .inner
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create job");

    let mut boundary = SessionExecutionBoundary::new(
        service,
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    let outcome = boundary
        .execute_session("job-1")
        .expect("a lost claim is an outcome, not an error");
    assert!(
        matches!(
            outcome,
            SessionRunOutcome::Reschedulable {
                ref job_id,
                reason: SessionRescheduleReason::ClaimLost,
            } if job_id == "job-1"
        ),
        "expected lost-claim reschedule, got {outcome:?}"
    );
    let snapshot = boundary.persistence.snapshot("job-1").expect("snapshot");
    assert_eq!(
        snapshot.state,
        JobState::Queued,
        "a lost claim must not advance the Job"
    );
    assert!(
        boundary
            .persistence
            .events("job-1")
            .expect("events")
            .is_empty(),
        "a lost claim must not persist a failure or any event"
    );
    // The failed claim must leave no scope held: a successor can claim.
    drop(boundary);
    let peer = LinuxCoordinator::new(&lock_dir).expect("peer coordinator");
    ExecutionReservation::acquire(&peer, "job-1", Path::new("/archive/Tweets/tweet-1"))
        .expect("scopes released after the lost claim");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(target_os = "linux")]
#[test]
fn no_journal_internal_v1_successor_cas_loss_does_not_start_or_fail_successor() {
    struct NoSuccessorOnCasLossFactory(Arc<AtomicBool>);
    impl JobExecutionFactory for NoSuccessorOnCasLossFactory {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            self.0.store(true, Ordering::Release);
            panic!("factory must not run when successor CAS is lost");
        }

        fn create_for_attempt(
            &self,
            _: &AttemptIdentity,
            _: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            self.0.store(true, Ordering::Release);
            panic!("successor factory must not run when successor CAS is lost");
        }
    }

    let root = scratch("v1-successor-cas-lost");
    let lock_dir = root.join("locks");
    let staging_root = root.join("staging");
    std::fs::create_dir_all(&staging_root).expect("staging root");
    let called = Arc::new(AtomicBool::new(false));
    let executor = JobExecutor::with_capacity_and_factory(
        4,
        Arc::new(NoSuccessorOnCasLossFactory(called.clone())),
    );
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
    persistence.internal_v1_no_journal = true;
    persistence.force_successor_claim_lost = true;
    persistence
        .inner
        .create_or_reuse(&request("job-v1-cas-lost", "tweet-v1-cas-lost"))
        .expect("create job");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        persistence
            .inner
            .persist_state(&JobSnapshot {
                job_id: "job-v1-cas-lost".to_owned(),
                tweet_id: "tweet-v1-cas-lost".to_owned(),
                state,
            })
            .expect("advance job state");
    }
    assert_eq!(
        persistence
            .inner
            .begin_attempt("job-v1-cas-lost")
            .expect("predecessor attempt"),
        1
    );
    let mut boundary = SessionExecutionBoundary::new(
        service,
        persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    )
    .with_recovery_containment(
        LinuxExecutionContainment {
            delegated_parent: PathBuf::from("/sys/fs/cgroup"),
            python: PathBuf::from("/usr/bin/python3"),
            launcher: PathBuf::from("/missing-launcher.py"),
        },
        staging_root,
    );

    let outcome = boundary.execute_session("job-v1-cas-lost");
    assert!(
        matches!(outcome, Err(ExecutorError::Execution { .. })),
        "lost successor CAS must fail closed, got {outcome:?}"
    );
    assert!(
        !called.load(Ordering::Acquire),
        "no successor factory/writer starts"
    );
    let persistence = &boundary.persistence;
    assert_eq!(
        persistence.inner.snapshot("job-v1-cas-lost").unwrap().state,
        JobState::Downloading
    );
    assert_eq!(
        persistence
            .inner
            .current_attempt("job-v1-cas-lost")
            .unwrap(),
        1
    );
    assert!(
        persistence
            .inner
            .events("job-v1-cas-lost")
            .unwrap()
            .is_empty(),
        "CAS loss must not persist predecessor FAILED or failure events"
    );
    drop(boundary);
    drop(executor);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn a_saturated_runner_is_rejected_before_any_claim() {
    let root = scratch("saturation");
    let lock_dir = root.join("locks");
    let factory = ProbeFactory::new(lock_dir.clone());
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    *factory.gate.lock().expect("probe gate") = Some(ExecutionGate {
        started: started_tx,
        release: release_rx,
    });
    let executor = JobExecutor::with_capacity_and_factory(4, Arc::new(factory));
    let service = ArchiveApplicationService::new(&executor);

    let mut running_persistence = BoundaryProbePersistence::new(lock_dir.clone());
    running_persistence
        .inner
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create running job");
    let mut running_boundary = SessionExecutionBoundary::new(
        service.clone(),
        running_persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    assert!(executor.handle().runner_try_admit());
    let reservation = executor.handle().owned_runner_admission();
    let runner = thread::spawn(move || {
        let _reservation = reservation;
        running_boundary.execute_session("job-1")
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("job-1 execution started");

    // While job-1 still owns the single runner slot, a second boundary must
    // refuse completely: no attempt claimed, no persistence read, no
    // durable write.
    let queued_job2 = {
        let mut waiting_persistence = BoundaryProbePersistence::new(lock_dir.clone());
        waiting_persistence
            .inner
            .create_or_reuse(&request("job-2", "tweet-2"))
            .expect("create waiting job");
        let waiting_boundary = SessionExecutionBoundary::new(
            service.clone(),
            waiting_persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let first_snapshot = waiting_boundary
            .persistence
            .snapshot("job-2")
            .expect("snapshot");
        let first_current_attempt = waiting_boundary
            .persistence
            .current_attempt("job-2")
            .expect("attempt");
        let first_events = waiting_boundary
            .persistence
            .events("job-2")
            .expect("events");
        let rejected = waiting_boundary.execute_on_runner("job-2".to_owned());
        assert_eq!(
            rejected.expect_err("a saturated runner must refuse before claiming"),
            ExecutorError::QueueFull
        );
        assert_eq!(first_snapshot.state, JobState::Queued);
        assert_eq!(first_current_attempt, 0);
        assert!(first_events.is_empty());
        first_snapshot
    };

    // Release the runner, then run the same saturated-boundary command on a
    // fresh ownership: it must complete job-2 through the runner operation.
    release_tx.send(()).expect("release job-1");
    let finished = runner.join().expect("job-1 thread").expect("job-1 run");
    assert!(
        matches!(
            finished,
            SessionRunOutcome::Finished(ref snapshot)
                if snapshot.state == JobState::Complete
        ),
        "job-1 must complete once the runner is released: {finished:?}"
    );

    let mut waiting_persistence = BoundaryProbePersistence::new(lock_dir.clone());
    waiting_persistence
        .inner
        .jobs
        .insert("job-2".to_owned(), queued_job2);
    let waiting_boundary = SessionExecutionBoundary::new(
        service,
        waiting_persistence,
        LinuxCoordinator::new(&lock_dir).expect("coordinator"),
        PathBuf::from("/archive"),
    );
    assert!(executor.handle().runner_try_admit());
    let reservation = executor.handle().owned_runner_admission();
    let runner = thread::spawn(move || {
        let _reservation = reservation;
        let mut boundary = waiting_boundary;
        let outcome = boundary.execute_session("job-2");
        (boundary, outcome)
    });
    let (waiting_boundary, completed) = runner.join().expect("job-2 thread");
    assert!(
        matches!(completed, Ok(SessionRunOutcome::Finished(ref out))
        if out.state == JobState::Complete && out.job_id == "job-2"),
        "job-2 must complete after the runner is released: {completed:?}"
    );
    let final_snapshot = waiting_boundary
        .persistence
        .snapshot("job-2")
        .expect("snapshot");
    assert_eq!(final_snapshot.state, JobState::Complete);
    let final_current_attempt = waiting_boundary
        .persistence
        .current_attempt("job-2")
        .expect("attempt");
    assert_eq!(final_current_attempt, 1);
    let final_events = waiting_boundary
        .persistence
        .events("job-2")
        .expect("events");
    assert!(!final_events.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

/// `complete_terminal_for_attempt` writes `Complete` state, `DownloadCompleted`,
/// and `Completed` event in one persistence transaction.
#[test]
fn complete_terminal_for_attempt_is_atomic() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");

    let attempt = persistence.current_attempt("job-1").expect("attempt");
    assert!(
        persistence
            .persist_state_for_attempt("job-1", attempt, JobState::Queued, JobState::Validating)
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt,
                JobState::Validating,
                JobState::MetadataReady
            )
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt,
                JobState::MetadataReady,
                JobState::Downloading
            )
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt,
                JobState::Downloading,
                JobState::Downloaded
            )
            .expect("transition"),
    );
    assert_eq!(
        persistence.snapshot("job-1").expect("snapshot").state,
        JobState::Downloaded
    );

    let completed = persistence
        .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
        .expect("complete");
    assert_eq!(completed.state, JobState::Complete);

    let events = persistence.events("job-1").expect("events");
    let download_completed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, ExecutorEvent::DownloadCompleted { .. }))
        .collect();
    let completed_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, ExecutorEvent::Completed { .. }))
        .collect();
    assert_eq!(download_completed.len(), 1, "exactly one DownloadCompleted");
    assert_eq!(completed_events.len(), 1, "exactly one Completed");
}

/// `complete_terminal_for_attempt` is idempotent: re-completing an already
/// `Complete` job writes nothing and returns the current snapshot.
#[test]
fn complete_terminal_for_attempt_is_idempotent() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");

    let attempt = persistence.current_attempt("job-1").expect("attempt");
    assert!(
        persistence
            .persist_state_for_attempt("job-1", attempt, JobState::Queued, JobState::Validating)
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt,
                JobState::Validating,
                JobState::MetadataReady
            )
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt,
                JobState::MetadataReady,
                JobState::Downloading
            )
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt,
                JobState::Downloading,
                JobState::Downloaded
            )
            .expect("transition"),
    );

    let first = persistence
        .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
        .expect("first complete");
    assert_eq!(first.state, JobState::Complete);
    let events_after_first = persistence.events("job-1").expect("events");
    let completed_count = events_after_first
        .iter()
        .filter(|e| matches!(e, ExecutorEvent::Completed { .. }))
        .count();
    assert_eq!(completed_count, 1, "one Completed after first completion");

    let second = persistence
        .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
        .expect("second complete");
    assert_eq!(second.state, JobState::Complete);
    let events_after_second = persistence.events("job-1").expect("events");
    let completed_count_2 = events_after_second
        .iter()
        .filter(|e| matches!(e, ExecutorEvent::Completed { .. }))
        .count();
    assert_eq!(
        completed_count_2, 1,
        "still one Completed after idempotent re-completion: {events_after_second:?}"
    );
}

/// `complete_terminal_for_attempt` rejects a stale attempt: a newer attempt
/// must not be able to complete an older job's terminal state.
#[test]
fn complete_terminal_for_attempt_rejects_stale_attempt() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");

    let attempt1 = persistence.current_attempt("job-1").expect("attempt");
    assert!(
        persistence
            .persist_state_for_attempt("job-1", attempt1, JobState::Queued, JobState::Validating)
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt1,
                JobState::Validating,
                JobState::MetadataReady
            )
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt1,
                JobState::MetadataReady,
                JobState::Downloading
            )
            .expect("transition"),
    );
    assert!(
        persistence
            .persist_state_for_attempt(
                "job-1",
                attempt1,
                JobState::Downloading,
                JobState::Downloaded
            )
            .expect("transition"),
    );

    // A newer attempt claims the job and moves state away from Downloaded.
    let attempt2 = attempt1 + 1;
    persistence.begin_attempt("job-1").expect("begin attempt 2");
    assert!(
        persistence
            .persist_state_for_attempt("job-1", attempt2, JobState::Downloaded, JobState::Failed)
            .expect("transition"),
    );

    let err = persistence
        .complete_terminal_for_attempt("job-1", attempt1, "Tweets/12345", 3)
        .unwrap_err();
    assert!(
        matches!(err, ExecutorError::Persistence(_)),
        "stale attempt must be rejected: {err:?}"
    );
    assert_eq!(
        persistence.snapshot("job-1").expect("snapshot").state,
        JobState::Failed,
        "stale attempt must not change state"
    );
}

/// `complete_terminal_for_attempt` fails closed when the job is not in
/// `Downloaded` state (e.g. `Queued`, `Failed`, `Cancelled`).
#[test]
fn complete_terminal_for_attempt_requires_downloaded_state() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("job");

    let attempt = persistence.current_attempt("job-1").expect("attempt");
    // Job is still Queued; attempting to complete must fail.
    let err = persistence
        .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
        .unwrap_err();
    assert!(
        matches!(err, ExecutorError::Persistence(_)),
        "non-Downloaded state must be rejected: {err:?}"
    );
    assert_eq!(
        persistence.snapshot("job-1").expect("snapshot").state,
        JobState::Queued,
        "state must not change on rejected completion"
    );
}
