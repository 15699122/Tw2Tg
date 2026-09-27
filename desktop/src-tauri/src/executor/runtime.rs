//! Executor handles and lifecycle: the worker/runner owner, its cloneable
//! handle, and the drop/shutdown boundary.
//!
//! ENG-15: separated so handle ownership and shutdown semantics are reviewed
//! without the scheduling and persistence code in the same file.

use std::sync::Arc;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};

use super::DEFAULT_QUEUE_CAPACITY;
use super::model::*;
use super::service::{
    Command, RunnerCommand, UnconfiguredExecutionFactory, run_runner, run_worker,
};
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
