//! Job state-machine transitions, cancellation fencing and lifecycle events.

use super::*;

#[test]
fn execute_reply_waits_until_execution_resources_are_dropped() {
    let executor = JobExecutor::new();
    let handle = executor.handle();
    handle
        .submit(request("job-drop-order", "tweet-drop-order"))
        .unwrap();
    let (dropped_sender, dropped_receiver) = mpsc::channel();
    let result = handle
        .execute(
            "job-drop-order",
            Box::new(DropNotifiesExecution {
                dropped: dropped_sender,
            }),
        )
        .expect("execution result");

    assert_eq!(result.archive_directory, "archives/drop-probe");
    dropped_receiver
        .try_recv()
        .expect("execution must be torn down before the caller receives its result");
    drop(executor);
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
