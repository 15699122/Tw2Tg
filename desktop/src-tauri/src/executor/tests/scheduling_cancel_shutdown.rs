//! Admission, scheduling, cancel-wins-commit races and shutdown draining.

use super::*;

#[test]
fn shutdown_interrupts_active_jobs_and_rejects_later_commands() {
    let executor = JobExecutor::new();
    let handle = executor.handle();
    handle.submit(request("job-1", "tweet-1")).unwrap();
    handle.submit(request("job-2", "tweet-2")).unwrap();

    executor.shutdown().unwrap();
    assert_eq!(handle.snapshot("job-1").unwrap_err(), ExecutorError::Closed);
}

/// B1 admission regression: a saturated runner must reject a queued Job
/// without incrementing an attempt or persisting FAILED, and the Job must
/// stay reschedulable once the runner frees up.
#[test]
fn saturated_runner_rejection_is_non_failing_and_reschedulable() {
    let release = Arc::new(AtomicBool::new(false));
    let started = Arc::new(AtomicBool::new(false));
    let factory = Arc::new(N1AdmissionFactory {
        release: release.clone(),
        started: started.clone(),
    });
    let executor = JobExecutor::with_capacity_and_factory(DEFAULT_QUEUE_CAPACITY, factory);
    let service = ArchiveApplicationService::new(&executor);

    let database_path = temporary_database_path("n1-admission");
    std::fs::create_dir_all(database_path.parent().expect("config dir")).expect("parent");
    {
        let mut setup = StorageJobPersistence::open(&database_path).expect("setup persistence");
        setup
            .create_or_reuse(&request("job-blocker", "tweet-blocker"))
            .expect("create blocker");
        setup
            .create_or_reuse(&request("job-second", "tweet-second"))
            .expect("create second");
    }

    // Occupy the single runner slot with a blocking Job on its own thread.
    let blocker_service = service.clone();
    let blocker_db = database_path.clone();
    let blocker = thread::spawn(move || {
        let mut persistence =
            StorageJobPersistence::open(&blocker_db).expect("blocker persistence");
        // The blocker only needs to occupy the runner slot; its final
        // archive result is irrelevant to the admission assertion.
        let _ = blocker_service.execute_persisted_from_factory(&mut persistence, "job-blocker");
    });
    while !started.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(2));
    }
    // From here on the blocker occupies the runner slot. If any assertion
    // below panics, this guard still releases the slot so the blocker
    // thread unblocks and the test binary cannot hang.
    let _release_guard = ReleaseOnDrop(release.clone());

    // The saturated runner must reject the second Job before any durable
    // claim: no attempt increment and no FAILED.
    let mut main_persistence =
        StorageJobPersistence::open(&database_path).expect("main persistence");
    let rejected = service.execute_persisted_from_factory(&mut main_persistence, "job-second");
    assert!(
        matches!(rejected, Err(ExecutorError::QueueFull)),
        "a saturated runner must report QueueFull, got {rejected:?}"
    );
    let second = main_persistence
        .snapshot("job-second")
        .expect("second snapshot");
    assert_eq!(
        second.state,
        JobState::Queued,
        "a QueueFull rejection must leave the Job queued, not failed"
    );
    assert_eq!(
        main_persistence
            .current_attempt("job-second")
            .expect("attempt"),
        0,
        "a QueueFull rejection must not increment the durable attempt"
    );

    // Release the blocker; the runner frees its slot. A drop guard also
    // releases it if any assertion above panics, so the blocker thread can
    // never hang the test binary.
    release.store(true, Ordering::SeqCst);
    blocker.join().expect("blocker thread");

    // The previously rejected Job is now admitted and runs (failing only
    // because the test factory returns a failure execution).
    let rerun = service.execute_persisted_from_factory(&mut main_persistence, "job-second");
    assert!(
        rerun.is_ok(),
        "the rescheduled Job must be admitted once the runner frees up, got {rerun:?}"
    );
    assert_eq!(
        main_persistence
            .current_attempt("job-second")
            .expect("attempt"),
        1,
        "an admitted Job claims exactly one attempt"
    );

    let _ = std::fs::remove_dir_all(database_path.parent().expect("root"));
}

#[cfg(unix)]
#[test]
fn persisted_scheduler_binds_attempt_before_successful_completion() {
    struct AttemptFactory;
    impl JobExecutionFactory for AttemptFactory {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("scheduler must use attempt-owned creation");
        }
        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(identity.job_id(), snapshot.job_id);
            assert_eq!(identity.attempt(), 1);
            assert_eq!(snapshot.state, JobState::Downloading);
            Ok(Box::new(FakeJobExecution::success(1, "Tweets/scheduled")))
        }
    }
    let database_path = temporary_database_path("scheduled-success");
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    let executor = JobExecutor::new();
    let mut handle = executor.handle();
    handle.factory = Arc::new(AttemptFactory);
    let service = ArchiveApplicationService {
        executor: handle,
        v2_enrollment: false,
    };
    service
        .submit_and_schedule_persisted(
            &mut persistence,
            request("job-scheduled", "tweet-scheduled"),
            database_path.clone(),
        )
        .expect("schedule");
    wait_for_persisted_state(&database_path, "job-scheduled", JobState::Complete);
    assert_eq!(persistence.current_attempt("job-scheduled").unwrap(), 1);
    let events = persistence.stored_events("job-scheduled").unwrap();
    for expected in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
        assert_eq!(
            events.iter().filter(|(kind, _)| kind == expected).count(),
            1
        );
    }
    executor.shutdown().unwrap();
    drop(persistence);
    let _ = std::fs::remove_dir_all(database_path.parent().unwrap().parent().unwrap());
}

#[test]
fn persisted_cancel_transitions_queued_to_cancelled_and_is_idempotent() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    executor
        .handle()
        .submit(request("job-1", "tweet-1"))
        .unwrap();

    let cancelled = service.cancel_persisted(&mut persistence, "job-1").unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled);
    assert_eq!(persistence.snapshot("job-1").unwrap(), cancelled);
    assert_eq!(
        persistence.events("job-1").unwrap(),
        vec![ExecutorEvent::StateChanged {
            from: JobState::Queued,
            to: JobState::Cancelled,
        }]
    );

    assert_eq!(
        service.cancel_persisted(&mut persistence, "job-1").unwrap(),
        cancelled
    );
    assert_eq!(persistence.events("job-1").unwrap().len(), 1);
}

#[test]
fn persisted_cancel_does_not_change_terminal_job_or_record_an_event() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Complete,
        })
        .unwrap();
    let before = persistence.events("job-1").unwrap();

    let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
    assert_eq!(result.state, JobState::Complete);
    assert_eq!(persistence.events("job-1").unwrap(), before);
}

#[cfg(target_os = "linux")]
#[test]
fn maintenance_blocks_production_and_startup_recovery_before_mutation() {
    let path = temporary_database_path("maintenance-admission");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let runtime = ExecutorRuntime::new(path.clone());
    let database = Database::open(&path).unwrap();
    let now = crate::clock::now_iso();
    let tweet_row_id = database
        .insert_tweet(
            "maintenance-tweet",
            "https://x.com/test/status/maintenance-tweet",
            "post",
            "",
            &now,
        )
        .unwrap();
    database
        .create_archive_job_with_optional_recovery_contract(
            "maintenance-job",
            tweet_row_id,
            1,
            "maintenance-request",
            "{}",
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            &now,
        )
        .unwrap();
    drop(database);
    let coordinator = ProductionCoordinator::open(&path).unwrap();
    let locks = coordinator.inner.lock_directory().to_owned();
    drop(coordinator);
    let maintenance =
        xarchive_workflow::maintenance::WorkspaceGuard::drain(&locks, || false).unwrap();
    assert!(runtime.production_session_boundary().is_err());
    let results = runtime.recover_startup().unwrap().join().unwrap();
    assert!(results.outcomes.is_empty());
    assert!(results.fatal_error.is_none());
    let persistence = StorageJobPersistence::open(&path).unwrap();
    assert_eq!(
        persistence.snapshot("maintenance-job").unwrap().state,
        JobState::Queued,
        "startup with no recovery candidate must leave the queued job unchanged"
    );
    assert_eq!(
        persistence.current_attempt("maintenance-job").unwrap(),
        0,
        "startup with no recovery candidate must not claim an attempt"
    );
    drop(persistence);
    drop(maintenance);
    assert!(runtime.production_session_boundary().is_ok());
    drop(runtime);
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}

/// A live production session (holding an active shared guard) must block the
/// maintenance drain until it finalizes; only then does drain complete. This
/// reconciles the apply path against a real session, not a raw guard.
#[cfg(target_os = "linux")]
#[test]
fn live_production_session_blocks_maintenance_drain_until_finalize() {
    let path = temporary_database_path("live-writer-drain");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let session = ProductionCoordinator::open(&path).unwrap();
    let locks = session.inner.lock_directory().to_owned();

    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let drain_locks = locks.clone();
    let drain_thread = std::thread::spawn(move || {
        // Signal that drain is about to block, then wait (cancellable) up to
        // a generous deadline for the live session to release.
        ready_tx.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let guard = xarchive_workflow::maintenance::WorkspaceGuard::drain(&drain_locks, || {
            std::time::Instant::now() >= deadline
        });
        done_tx.send(guard.is_ok()).unwrap();
    });
    ready_rx.recv().unwrap();
    // Give the drain thread time to close admission and start waiting.
    std::thread::sleep(std::time::Duration::from_millis(200));
    // The live session still holds active shared, so drain cannot have completed.
    assert!(
        done_rx.try_recv().is_err(),
        "drain must not complete while a live session holds active shared"
    );
    // New production admission is now refused because the turnstile is held.
    assert!(
        ProductionCoordinator::open(&path).is_err(),
        "turnstile must refuse new admission during maintenance drain"
    );
    // Finalize the live session: release its active shared guard.
    drop(session);
    // Drain now completes within the deadline.
    let completed = done_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("drain must complete after the live session finalizes");
    assert!(
        completed,
        "drain should acquire active exclusive once drained"
    );
    drain_thread.join().unwrap();
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}

/// When a durable archive commit boundary exists before cancellation is
/// attempted, the commit wins: the writer keeps running, no cancelled
/// state is persisted, and no state-change event is recorded.
#[test]
fn persisted_cancel_refuses_before_stopping_when_commit_started() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = CommitBoundaryPersistence {
        inner: InMemoryJobPersistence::default(),
        report_at_precheck: true,
        report_at_decision: false,
    };
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    executor
        .handle()
        .submit(request("job-1", "tweet-1"))
        .unwrap();

    let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
    assert_eq!(result.state, JobState::Queued);
    assert_eq!(
        persistence.snapshot("job-1").unwrap().state,
        JobState::Queued
    );
    assert!(
        persistence
            .events("job-1")
            .unwrap()
            .iter()
            .all(|event| !matches!(event, ExecutorEvent::StateChanged { .. })),
        "a refused cancellation records no state change"
    );
    // The writer was never stopped.
    assert_eq!(
        executor.handle().snapshot("job-1").unwrap().state,
        JobState::Queued
    );
}

/// A prepare that lands between the pre-check and the decision still wins:
/// the decision path reports CommitWins and no cancelled state or event is
/// written over the journal.
#[test]
fn persisted_cancel_commit_wins_race_and_writes_nothing() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = CommitBoundaryPersistence {
        inner: InMemoryJobPersistence::default(),
        report_at_precheck: false,
        report_at_decision: true,
    };
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    executor
        .handle()
        .submit(request("job-1", "tweet-1"))
        .unwrap();

    let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
    assert_eq!(result.state, JobState::Queued);
    assert_eq!(
        persistence.snapshot("job-1").unwrap().state,
        JobState::Queued,
        "the durable job stays non-cancelled when the commit wins"
    );
    assert!(
        persistence
            .events("job-1")
            .unwrap()
            .iter()
            .all(|event| !matches!(event, ExecutorEvent::StateChanged { .. })),
        "no cancelled state-change event is written when the commit wins"
    );
}

#[test]
fn persisted_shutdown_interrupts_active_jobs_once_and_skips_terminal_jobs() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-active", "tweet-active"))
        .unwrap();
    persistence
        .create_or_reuse(&request("job-terminal", "tweet-terminal"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Complete,
        })
        .unwrap();
    let terminal_events = persistence.events("job-terminal").unwrap();

    service.shutdown_persisted(&mut persistence).unwrap();

    assert_eq!(
        persistence.snapshot("job-active").unwrap().state,
        JobState::Interrupted
    );
    assert_eq!(
        persistence.events("job-active").unwrap(),
        vec![
            ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Interrupted,
            },
            ExecutorEvent::ShutdownInterrupted {
                from: JobState::Queued,
                to: JobState::Interrupted,
            },
        ]
    );
    assert_eq!(persistence.events("job-terminal").unwrap(), terminal_events);
}

#[test]
fn interrupt_persisted_is_separate_from_worker_shutdown() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    service
        .submit(request("job-active", "tweet-active"))
        .unwrap();
    persistence
        .create_or_reuse(&request("job-active", "tweet-active"))
        .unwrap();

    service.interrupt_persisted(&mut persistence).unwrap();

    assert_eq!(
        persistence.snapshot("job-active").unwrap().state,
        JobState::Interrupted
    );
    assert!(service.query("job-active").is_ok());
    service.shutdown().unwrap();
}
