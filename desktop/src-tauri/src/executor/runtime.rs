//! Executor runtime/control plane: queue admission, commands and worker ownership.

#[cfg(target_os = "linux")]
use super::LinuxExecutionContainment;
use super::{
    ArchiveJobRequest, CancellationToken, DEFAULT_QUEUE_CAPACITY, ExecutorConfig, ExecutorError,
    ExecutorEvent, JobExecution, JobExecutionFactory, JobExecutionResult, JobSnapshot,
    RUNNER_CAPACITY, SubmitResult, UnconfiguredExecutionFactory,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use xarchive_core::JobState;

mod application_service;
mod startup;

// Compatibility re-exports: both halves of the runtime moved to child modules,
// but `executor.rs` and the executor subtree keep using `runtime::<name>`.
pub use application_service::ArchiveApplicationService;

/// Runtime-owned executor configuration and lifecycle boundary for the R1 worker.
/// Production runner resources are created from immutable Job execution specs.
pub struct ExecutorRuntime {
    pub(super) executor: JobExecutor,
    database_path: PathBuf,
    config: ExecutorConfig,
    #[cfg(target_os = "linux")]
    pub(super) containment: Option<LinuxExecutionContainment>,
}

#[derive(Debug)]
struct JobRecord {
    snapshot: JobSnapshot,
    events: Vec<ExecutorEvent>,
    cancellation: CancellationToken,
}

#[derive(Clone, Default)]
struct CancellationRegistry(Arc<Mutex<HashMap<String, CancellationToken>>>);

impl CancellationRegistry {
    fn token(&self, job_id: &str) -> Result<CancellationToken, ExecutorError> {
        self.0
            .lock()
            .map_err(|_| ExecutorError::Execution {
                error_code: "CANCELLATION_REGISTRY_POISONED".to_owned(),
                error_message: "job cancellation registry was poisoned".to_owned(),
                persistence_already_updated: false,
            })
            .map(|mut tokens| tokens.entry(job_id.to_owned()).or_default().clone())
    }
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
    /// Run one owned lifecycle operation inline on the runner. The closure
    /// deliberately has no access to a dispatch handle, preventing recursive
    /// submission to the runner that is currently executing it.
    Operation {
        operation: Box<dyn RunnerOperation>,
    },
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

trait RunnerOperation: Send {
    fn run(self: Box<Self>);
}

struct RunnerOperationTask<F, T> {
    operation: Option<Box<F>>,
    response: mpsc::Sender<Result<T, ExecutorError>>,
    admission: Option<OwnedRunnerAdmission>,
}

impl<F, T> RunnerOperation for RunnerOperationTask<F, T>
where
    F: FnOnce() -> Result<T, ExecutorError> + Send + 'static,
    T: Send + 'static,
{
    fn run(mut self: Box<Self>) {
        let operation = self.operation.take().expect("one-shot runner operation");
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
        // Drop the boxed FnOnce explicitly: closure environments are not
        // guaranteed to be destroyed before a caller's response is observed.
        drop(self.operation.take());
        let result = match outcome {
            Ok(result) => result,
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("runner operation panicked")
                    .to_owned();
                // Destroy the panic payload before publishing the error or
                // releasing admission; panic payloads may own lifecycle data.
                drop(payload);
                Err(ExecutorError::Execution {
                    error_code: "RUNNER_OPERATION_PANICKED".to_owned(),
                    error_message: message,
                    persistence_already_updated: false,
                })
            }
        };
        // The boxed closure and any panic payload are gone before admission is
        // released. Release before replying so observing the result also
        // observes completed admission cleanup.
        drop(self.admission.take());
        let _ = self.response.send(result);
    }
}

/// Owned counterpart to `RunnerReservation`: it can cross the runner's
/// `'static` queue and releases admission only after the inline operation and
/// all values captured by that operation have been dropped.
pub(super) struct OwnedRunnerAdmission(Arc<AtomicUsize>);

impl Drop for OwnedRunnerAdmission {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct JobExecutorHandle {
    sender: SyncSender<Command>,
    runner_sender: SyncSender<RunnerCommand>,
    pub(super) factory: Arc<dyn JobExecutionFactory>,
    /// Tracks how many callers currently hold a runner slot. Mirrors the
    /// capacity of `runner_sender` so admission can be decided *before* a
    /// durable attempt/state claim instead of after it.
    pub(super) runner_in_flight: Arc<AtomicUsize>,
    cancellations: CancellationRegistry,
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
        let (runner_sender, runner_receiver) = mpsc::sync_channel(RUNNER_CAPACITY);
        let handle = JobExecutorHandle {
            sender,
            runner_sender: runner_sender.clone(),
            factory: factory.clone(),
            runner_in_flight: Arc::new(AtomicUsize::new(0)),
            cancellations: CancellationRegistry::default(),
        };
        let runner = thread::Builder::new()
            .name("xarchive-job-runner".to_owned())
            .spawn(move || run_runner(runner_receiver, factory))
            .expect("job runner must spawn");
        let worker_cancellations = handle.cancellations.clone();
        let worker = thread::Builder::new()
            .name("xarchive-job-executor".to_owned())
            .spawn(move || run_worker(receiver, runner_sender, worker_cancellations))
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
    pub(super) fn cancellation_for(
        &self,
        job_id: &str,
    ) -> Result<CancellationToken, ExecutorError> {
        self.cancellations.token(job_id)
    }

    /// Reserve a runner slot before a durable attempt/state claim.
    ///
    /// Returns `false` when the single-active runner is already saturated. A
    /// caller that cannot reserve must leave the Job queued and reschedulable
    /// rather than claiming an attempt and later persisting a failure.
    pub(super) fn runner_try_admit(&self) -> bool {
        self.runner_in_flight
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |in_flight| {
                (in_flight < RUNNER_CAPACITY).then_some(in_flight + 1)
            })
            .is_ok()
    }

    /// Release a previously reserved runner slot.
    pub(super) fn runner_release(&self) {
        self.runner_in_flight.fetch_sub(1, Ordering::AcqRel);
    }

    /// Convert an already-reserved slot into an owned token suitable for a
    /// single inline runner operation. The caller must not release the slot
    /// separately after this succeeds.
    pub(super) fn owned_runner_admission(&self) -> OwnedRunnerAdmission {
        OwnedRunnerAdmission(self.runner_in_flight.clone())
    }

    /// Submit one `'static` operation to run directly on the existing runner
    /// thread, with its admission token owned by that task. The task cannot
    /// recursively submit another runner command because it receives no handle.
    pub(super) fn run_operation<T: Send + 'static>(
        &self,
        admission: OwnedRunnerAdmission,
        operation: impl FnOnce() -> Result<T, ExecutorError> + Send + 'static,
    ) -> Result<T, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        let task: Box<dyn RunnerOperation> = Box::new(RunnerOperationTask {
            operation: Some(Box::new(operation)),
            response: response_sender,
            admission: Some(admission),
        });
        if let Err(error) = self
            .runner_sender
            .try_send(RunnerCommand::Operation { operation: task })
        {
            return match error {
                TrySendError::Full(RunnerCommand::Operation { operation }) => {
                    drop(operation);
                    Err(ExecutorError::QueueFull)
                }
                TrySendError::Disconnected(RunnerCommand::Operation { operation }) => {
                    drop(operation);
                    Err(ExecutorError::Closed)
                }
                _ => unreachable!("only an operation command was sent"),
            };
        }
        let result = response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?;
        result
    }

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

    /// Dispatch an execution object directly to the single-active runner,
    /// carrying the caller's *owned* running snapshot.
    ///
    /// This is the dispatch path for the runner-owned session boundary, which
    /// owns its persistence context and therefore cannot depend on the
    /// worker's `jobs` map: a borrowed `JobPersistence` cannot cross the
    /// runner's `'static` queue, so the snapshot must be supplied by the
    /// caller rather than looked up by the worker. The caller holds a
    /// [`RunnerReservation`] for the whole call, so the single runner slot is
    /// guaranteed available and `try_send` cannot spuriously report
    /// [`ExecutorError::QueueFull`]. Mid-execution cancellation is a later
    /// boundary concern (B4/B5); the token is never signalled here because the
    /// boundary fences every terminal write on the durable attempt and reports
    /// a durable `Interrupted`/newer-attempt decision after the run.
    pub(crate) fn execute_owned(
        &self,
        job_id: &str,
        snapshot: JobSnapshot,
        execution: Box<dyn JobExecution>,
    ) -> Result<JobExecutionResult, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        self.runner_sender
            .try_send(RunnerCommand::Execute {
                job_id: job_id.to_owned(),
                snapshot,
                execution,
                cancellation: CancellationToken::new(),
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

fn run_runner(receiver: Receiver<RunnerCommand>, factory: Arc<dyn JobExecutionFactory>) {
    while let Ok(command) = receiver.recv() {
        match command {
            RunnerCommand::Operation { operation } => operation.run(),
            RunnerCommand::Execute {
                snapshot,
                mut execution,
                cancellation,
                response,
                ..
            } => {
                let result = execution
                    .execute(&snapshot, &cancellation)
                    .and_then(JobExecutionResult::require_finalized)
                    .map_err(|error| ExecutorError::Execution {
                        error_code: error.error_code,
                        error_message: error.error_message,
                        persistence_already_updated: error.persistence_already_updated,
                    });
                // The execution owns its process supervisors and session-bound
                // resources. Tear them down before the caller can proceed to
                // finalization or release its attempt authorization.
                drop(execution);
                let _ = response.send(result);
            }
            RunnerCommand::RunJob {
                job_id,
                snapshot,
                cancellation,
                response,
            } => {
                // The scheduling caller (`execute_persisted_from_factory`)
                // already reserved the single runner slot and claimed the
                // attempt before dispatching here. This arm runs *on* the
                // runner thread, so it must invoke the factory and execution
                // directly: re-entering `run_operation` would enqueue to this
                // same runner and self-deadlock, and acquiring a second
                // admission would double-release the caller's reservation.
                let result = factory
                    .create(&job_id, &snapshot)
                    .and_then(|mut execution| {
                        let result = execution
                            .execute(&snapshot, &cancellation)
                            .and_then(JobExecutionResult::require_finalized)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: error.error_code,
                                error_message: error.error_message,
                                persistence_already_updated: error.persistence_already_updated,
                            });
                        // The execution owns its process supervisors and
                        // session-bound resources; tear them down before the
                        // caller can proceed to finalization or release its
                        // attempt authorization.
                        drop(execution);
                        result
                    });
                let _ = response.send(result);
            }
            RunnerCommand::Shutdown => break,
        }
    }
}

fn run_worker(
    receiver: Receiver<Command>,
    runner_sender: SyncSender<RunnerCommand>,
    cancellations: CancellationRegistry,
) {
    let mut jobs = HashMap::<String, JobRecord>::new();
    while let Ok(command) = receiver.recv() {
        let shutdown = matches!(command, Command::Shutdown { .. });
        handle_command(command, &mut jobs, &runner_sender, &cancellations);
        if shutdown {
            break;
        }
    }
}

fn handle_command(
    command: Command,
    jobs: &mut HashMap<String, JobRecord>,
    runner_sender: &SyncSender<RunnerCommand>,
    cancellations: &CancellationRegistry,
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
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
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
            if let Ok(tokens) = cancellations.0.lock()
                && let Some(token) = tokens.get(&job_id)
            {
                token.cancel();
            }
            if let Ok(tokens) = cancellations.0.lock()
                && let Some(token) = tokens.get(&job_id)
            {
                token.cancel();
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
                            TrySendError::Full(RunnerCommand::Operation { .. })
                            | TrySendError::Disconnected(RunnerCommand::Operation { .. }) => {
                                unreachable!("inline operations bypass the executor worker")
                            }
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
                    TrySendError::Full(RunnerCommand::Operation { .. })
                    | TrySendError::Disconnected(RunnerCommand::Operation { .. }) => {
                        unreachable!("inline operations bypass the executor worker")
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
