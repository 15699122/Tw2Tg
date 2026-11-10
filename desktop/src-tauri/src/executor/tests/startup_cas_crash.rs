//! Startup redispatch, CAS ownership, crashed-writer drain and crash replay.

use super::*;

/// Non-cgroup Sidecar lifecycle: hello/ready handshake, extract then
/// cancel, and shutdown-driven exit. Uses the in-repo JSONL fixture at
/// `desktop/src-tauri/tests/fixtures/internal_v1_worker.py`; no aria2,
/// no gallery-dl, no real network, no cgroup delegation required.
#[cfg(target_os = "linux")]
#[test]
fn sidecar_internal_v1_hello_ready_and_shutdown_exits() {
    use std::time::Duration;
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_owned());
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixture = fixture_dir.join("internal_v1_worker.py");
    assert!(fixture.is_file(), "fixture must exist: {fixture:?}");
    let mut supervisor = SidecarSupervisor::spawn_ready_v2(
        &python,
        &[
            "-u",
            "-c",
            &format!(
                "import sys; sys.path.insert(0, {fixture_dir:?}); from internal_v1_worker import main; main()"
            ),
        ],
        Duration::from_secs(5),
    )
    .expect("fixture must start and complete v2 handshake");
    assert!(
        supervisor
            .is_running()
            .expect("sidecar running after handshake")
    );
    supervisor
        .send_v2(&xarchive_protocol::SidecarV2Command {
            protocol_version: 2,
            request_id: "extract-active".to_owned(),
            cmd: xarchive_protocol::SidecarV2CommandType::Extract,
            job_id: "job-active".to_owned(),
            url: Some("http://127.0.0.1/fixture".to_owned()),
            browser: None,
            profile: None,
            staging_dir: None,
            user_args: Vec::new(),
        })
        .expect("send active extraction");
    let started = supervisor
        .recv_timeout(Duration::from_secs(5))
        .expect("receive extraction_started")
        .expect("extraction event");
    let xarchive_sidecar_supervisor::SupervisorEvent::V2(started) = started else {
        panic!("expected extraction_started event");
    };
    assert_eq!(
        started.event,
        xarchive_protocol::SidecarV2EventType::ExtractionStarted
    );
    supervisor
        .send_v2_shutdown("desktop-shutdown")
        .expect("send shutdown");
    supervisor.close_stdin();
    assert!(
        supervisor
            .wait_for_exit(Duration::from_secs(5))
            .expect("wait for exit"),
        "sidecar must exit after shutdown"
    );
}

/// Non-cgroup lifecycle: extract starts, then cancel aborts the extraction
/// and the worker remains able to shut down cleanly.
///
#[cfg(target_os = "linux")]
#[test]
fn sidecar_internal_v1_extract_then_cancel_aborts_cleanly() {
    use std::time::Duration;
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_owned());
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/internal_v1_worker.py");
    use std::io::{BufRead, Read, Write};
    use std::process::{Command, Stdio};

    let mut child = Command::new(&python)
        .arg("-u")
        .arg(&fixture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("fixture must start");
    let stdout = child.stdout.take().expect("stdout must be piped");
    let mut reader = std::io::BufReader::new(stdout);
    let mut stderr = child.stderr.take().expect("stderr must be piped");
    let mut stdin = child.stdin.take().expect("stdin must be piped");

    fn send_event(
        stdin: &mut std::process::ChildStdin,
        reader: &mut std::io::BufReader<std::process::ChildStdout>,
        stderr: &mut std::process::ChildStderr,
        cmd: &str,
        job_id: &str,
        request_id: &str,
    ) -> serde_json::Value {
        let command = serde_json::json!({
            "protocol_version": 2u32, "request_id": request_id, "cmd": cmd,
            "job_id": job_id, "url": null, "browser": null, "profile": null,
            "staging_dir": null, "user_args": [],
        });
        let payload = serde_json::to_string(&command).unwrap();
        stdin.write_all(payload.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let mut err = String::new();
                stderr.read_to_string(&mut err).ok();
                panic!("fixture EOF after {cmd}; sent={payload}; stderr={err}");
            }
            Ok(_) => serde_json::from_str(line.trim()).expect("valid JSON event"),
            Err(error) => panic!("read fixture event after {cmd}: {error}"),
        }
    }

    let ready = send_event(
        &mut stdin,
        &mut reader,
        &mut stderr,
        "hello",
        "system",
        "hello-1",
    );
    assert_eq!(ready["event"], "ready");
    let started = send_event(
        &mut stdin,
        &mut reader,
        &mut stderr,
        "extract",
        "job-cancel",
        "extract-1",
    );
    assert_eq!(started["event"], "extraction_started");
    let cancelled = send_event(
        &mut stdin,
        &mut reader,
        &mut stderr,
        "cancel",
        "job-cancel",
        "cancel-1",
    );
    assert_eq!(cancelled["event"], "failed");
    assert_eq!(cancelled["error_code"], "CANCELLED");
    // Production exits on `shutdown` without emitting an event: the protocol
    // event enum in `xarchive-protocol` defines no shutdown acknowledgement,
    // so the fixture and this assertion follow the real contract instead of a
    // fixture-only `shutdown_ack`.
    let shutdown_payload = serde_json::to_string(&serde_json::json!({
        "protocol_version": 2u32, "request_id": "shutdown-1", "cmd": "shutdown",
        "job_id": "system", "url": null, "browser": null, "profile": null,
        "staging_dir": null, "user_args": [],
    }))
    .unwrap();
    stdin.write_all(shutdown_payload.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "fixture child exit timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success());
}

#[cfg(unix)]
#[test]
fn startup_redispatch_binds_attempt_before_successful_completion() {
    struct RecoveryFactory;
    impl JobExecutionFactory for RecoveryFactory {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("startup recovery must use attempt-owned creation");
        }
        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(identity.job_id(), snapshot.job_id);
            assert_eq!(identity.attempt(), 1);
            Ok(Box::new(FakeJobExecution::success(1, "Tweets/recovered")))
        }
    }
    let database_path = temporary_database_path("startup-success");
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    persistence
        .create_or_reuse(&request("job-recovered", "tweet-recovered"))
        .unwrap();
    let mut runtime = ExecutorRuntime::new(database_path.clone());
    runtime.executor =
        JobExecutor::with_capacity_and_factory(DEFAULT_QUEUE_CAPACITY, Arc::new(RecoveryFactory));
    runtime.recover_startup().expect("startup redispatch");
    wait_for_persisted_state(&database_path, "job-recovered", JobState::Complete);
    assert_eq!(persistence.current_attempt("job-recovered").unwrap(), 1);
    let events = persistence.stored_events("job-recovered").unwrap();
    for expected in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
        assert_eq!(
            events.iter().filter(|(kind, _)| kind == expected).count(),
            1
        );
    }
    runtime.executor.shutdown().unwrap();
    drop(persistence);
    let _ = std::fs::remove_dir_all(database_path.parent().unwrap().parent().unwrap());
}

/// A no-journal DOWNLOADING InternalV1 row is eligible only for the
/// successor takeover. Without containment, startup must leave it
/// unchanged and refuse rather than invent a failure.
#[test]
fn startup_recovery_fails_closed_when_internal_manifest_is_missing() {
    let database_path = temporary_database_path("startup-missing-manifest");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
    let job_id = "job-startup-missing-manifest";
    let tweet_id = "900001";
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    insert_downloaded_job(&mut persistence, job_id, tweet_id);
    persistence
        .database
        .execute_batch(&format!(
            "UPDATE jobs SET recovery_contract_version = 1 WHERE id = '{job_id}'"
        ))
        .expect("declare the durable InternalV1 contract");
    drop(persistence);

    let mut runtime = ExecutorRuntime::new(database_path.clone());
    struct RefusingFactory;
    impl JobExecutionFactory for RefusingFactory {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("recovery must use a claimed attempt identity");
        }
        fn create_for_attempt(
            &self,
            _: &AttemptIdentity,
            _: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            panic!("missing containment must refuse before factory creation");
        }
    }
    runtime.executor =
        JobExecutor::with_capacity_and_factory(DEFAULT_QUEUE_CAPACITY, Arc::new(RefusingFactory));
    let report = runtime
        .recover_startup()
        .expect("startup recovery spawns")
        .join()
        .expect("startup recovery thread");
    assert!(report.fatal_error.is_none());
    assert_eq!(
        report.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Refused,
        }]
    );
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
    assert_eq!(
        reopened.snapshot(job_id).expect("snapshot").state,
        JobState::Downloading
    );
    assert_eq!(reopened.current_attempt(job_id).expect("attempt"), 0);
    assert_eq!(
        reopened.internal_v1_attempt_phase(job_id).expect("phase"),
        Some(None)
    );
    assert!(
        !reopened
            .stored_events(job_id)
            .expect("events")
            .iter()
            .any(|(event_type, _)| event_type == "JOB_COMPLETED")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(target_os = "linux")]
#[test]
fn startup_recovery_takes_over_no_journal_internal_v1_downloading_attempt() {
    struct SuccessorFactory;
    impl JobExecutionFactory for SuccessorFactory {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("successor recovery must use attempt-owned creation");
        }

        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(identity.job_id(), snapshot.job_id);
            assert_eq!(
                identity.attempt(),
                2,
                "the drained predecessor advances once"
            );
            assert_eq!(snapshot.state, JobState::Downloading);
            Ok(Box::new(FakeJobExecution::success(1, "Tweets/900042")))
        }

        fn create_registered_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
            _registration: xarchive_workflow::WriterRegistration,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            self.create_for_attempt(identity, snapshot)
        }
    }

    let database_path = temporary_database_path("startup-v1-successor");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("database parent");
    let job_id = "job-startup-v1-successor";
    let tweet_id = "900042";
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    persistence
        .create_or_reuse(&request(job_id, tweet_id))
        .expect("create persisted job with execution spec");
    persistence
        .database
        .execute_batch(&format!(
            "UPDATE jobs SET recovery_contract_version = 1 WHERE id = '{job_id}'"
        ))
        .expect("declare InternalV1 contract");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        persistence
            .persist_state(&JobSnapshot {
                job_id: job_id.to_owned(),
                tweet_id: tweet_id.to_owned(),
                state,
            })
            .expect("advance to crashed download state");
    }
    assert_eq!(
        persistence
            .database
            .begin_archive_attempt(job_id, "predecessor")
            .expect("persist predecessor attempt"),
        1
    );
    drop(persistence);

    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: database_path.parent().unwrap().join("_staging"),
        database_path: database_path.clone(),
        sidecar_program: None,
        gallery_dl_program: None,
        sidecar_args: Vec::new(),
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig::default(),
    };
    let mut runtime = ExecutorRuntime::with_linux_containment(
        config,
        LinuxExecutionContainment {
            delegated_parent: PathBuf::from("/sys/fs/cgroup"),
            python: PathBuf::from("/usr/bin/python3"),
            launcher: PathBuf::from("/missing-launcher.py"),
        },
    );
    runtime.executor =
        JobExecutor::with_capacity_and_factory(DEFAULT_QUEUE_CAPACITY, Arc::new(SuccessorFactory));

    let report = runtime
        .recover_startup()
        .expect("startup recovery dispatch")
        .join()
        .expect("startup recovery thread");
    assert!(report.fatal_error.is_none());
    assert_eq!(
        report.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Recovered,
        }]
    );
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
    assert_eq!(
        reopened.snapshot(job_id).expect("snapshot").state,
        JobState::Complete
    );
    assert_eq!(reopened.current_attempt(job_id).expect("attempt"), 2);
    let events = reopened.stored_events(job_id).expect("events");
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "DOWNLOAD_COMPLETED")
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "JOB_COMPLETED")
            .count(),
        1
    );
    runtime.executor.shutdown().expect("shutdown runner");
    drop(reopened);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(target_os = "linux")]
#[test]
fn startup_recovery_sqlite_successor_cas_loss_fails_closed_at_runner_boundary() {
    struct MustNotStart(Arc<AtomicBool>);
    impl JobExecutionFactory for MustNotStart {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            self.0.store(true, Ordering::Release);
            panic!("ordinary execution path must not run after successor CAS loss");
        }

        fn create_for_attempt(
            &self,
            _: &AttemptIdentity,
            _: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            self.0.store(true, Ordering::Release);
            panic!("successor writer must not start after SQLite CAS loss");
        }

        fn create_registered_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
            _: xarchive_workflow::WriterRegistration,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            self.create_for_attempt(identity, snapshot)
        }
    }

    let database_path = temporary_database_path("startup-v1-sqlite-cas-loss");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("database parent");
    let job_id = "job-startup-v1-sqlite-cas-loss";
    let tweet_id = "900044";
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    persistence
        .create_or_reuse(&request(job_id, tweet_id))
        .expect("create persisted job");
    persistence
        .database
        .execute_batch(&format!(
            "UPDATE jobs SET recovery_contract_version = 1 WHERE id = '{job_id}'"
        ))
        .expect("declare InternalV1");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        persistence
            .persist_state(&JobSnapshot {
                job_id: job_id.to_owned(),
                tweet_id: tweet_id.to_owned(),
                state,
            })
            .expect("advance job state");
    }
    assert_eq!(
        persistence
            .database
            .begin_archive_attempt(job_id, "predecessor")
            .expect("persist predecessor attempt"),
        1
    );
    // Fixture-local SQLite trigger deterministically models a competing
    // durable writer winning the successor UPDATE: SQLite reports zero
    // changed rows, exactly the CAS-loss result handled by production.
    persistence
        .database
        .execute_batch(&format!(
            "CREATE TRIGGER fixture_win_successor_cas BEFORE UPDATE OF attempt_count ON jobs
             WHEN OLD.id = '{job_id}' AND NEW.attempt_count = OLD.attempt_count + 1
                AND OLD.state = 'DOWNLOADING' AND OLD.recovery_contract_version = 1
             BEGIN SELECT RAISE(IGNORE); END;"
        ))
        .expect("install fixture-local CAS arbitration trigger");
    drop(persistence);

    let runtime = ExecutorRuntime::with_linux_containment(
        ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root: root.join("archive"),
            staging_root: database_path.parent().unwrap().join("_staging"),
            database_path: database_path.clone(),
            sidecar_program: None,
            gallery_dl_program: None,
            sidecar_args: Vec::new(),
            aria2_program: None,
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        },
        LinuxExecutionContainment {
            delegated_parent: PathBuf::from("/sys/fs/cgroup"),
            python: PathBuf::from("/usr/bin/python3"),
            launcher: PathBuf::from("/missing-launcher.py"),
        },
    );
    let started = Arc::new(AtomicBool::new(false));
    let mut runtime = runtime;
    runtime.executor = JobExecutor::with_capacity_and_factory(
        DEFAULT_QUEUE_CAPACITY,
        Arc::new(MustNotStart(started.clone())),
    );
    let report = runtime.recover_startup().unwrap().join().unwrap();
    assert!(
        report.fatal_error.is_none(),
        "CAS loss is per-job refusal: {report:?}"
    );
    assert_eq!(
        report.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Refused,
        }]
    );
    assert!(!started.load(Ordering::Acquire));
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen database");
    assert_eq!(
        reopened.snapshot(job_id).unwrap().state,
        JobState::Downloading
    );
    assert_eq!(reopened.current_attempt(job_id).unwrap(), 1);
    assert!(
        reopened
            .stored_events(job_id)
            .unwrap()
            .iter()
            .all(|(event_type, _)| event_type != "JOB_COMPLETED"
                && event_type != "DOWNLOAD_FAILED")
    );
    runtime.executor.shutdown().expect("shutdown runner");
    drop(reopened);
    std::fs::remove_dir_all(root).expect("remove CAS-loss fixture");
}

/// A malformed containment record refuses takeover without mutating the
/// predecessor state or attempt; refusal is retryable after remediation.
#[cfg(target_os = "linux")]
#[test]
fn startup_recovery_drains_crashed_writer_domains_before_decisions() {
    let database_path = temporary_database_path("startup-writer-drain");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
    let job_id = "job-startup-writer-drain";
    let tweet_id = "900041";
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    insert_downloaded_job(&mut persistence, job_id, tweet_id);
    persistence
        .database
        .execute_batch(&format!(
            "UPDATE jobs SET recovery_contract_version = 1 WHERE id = '{job_id}'"
        ))
        .expect("declare the durable InternalV1 contract");
    drop(persistence);
    let mut database = Database::open(&database_path).expect("database");
    database
        .begin_archive_attempt(job_id, "startup-recovery-test")
        .expect("begin attempt");
    drop(database);
    let staging_root = database_path.parent().unwrap().join("_staging");
    let record_root = database_path
        .parent()
        .unwrap()
        .join("tw2tg-containment-records");
    let malformed =
        xarchive_workflow::linux_containment::AttemptIdentity::new(&staging_root, job_id, 1)
            .unwrap()
            .with_writer("sidecar")
            .unwrap();
    std::fs::create_dir_all(&record_root).expect("record root");
    std::fs::write(
        malformed.record_path(&record_root),
        "not a containment record\n",
    )
    .expect("malformed record");

    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root,
        database_path: database_path.clone(),
        sidecar_program: None,
        gallery_dl_program: None,
        sidecar_args: Vec::new(),
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig::default(),
    };
    let mut runtime = ExecutorRuntime::with_linux_containment(
        config,
        LinuxExecutionContainment {
            delegated_parent: std::path::PathBuf::from("/sys/fs/cgroup"),
            python: std::path::PathBuf::from("/usr/bin/python3"),
            launcher: std::path::PathBuf::from("/missing-launcher.py"),
        },
    );
    struct RefusingFactory;
    impl JobExecutionFactory for RefusingFactory {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("recovery must use a claimed attempt identity");
        }
        fn create_for_attempt(
            &self,
            _: &AttemptIdentity,
            _: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            panic!("drain refusal must stop before factory creation");
        }
    }
    runtime.executor =
        JobExecutor::with_capacity_and_factory(DEFAULT_QUEUE_CAPACITY, Arc::new(RefusingFactory));
    let report = runtime
        .recover_startup()
        .expect("startup recovery spawns")
        .join()
        .expect("startup recovery thread");
    assert!(report.fatal_error.is_none());
    assert_eq!(
        report.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Refused,
        }]
    );
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
    assert_eq!(
        reopened.snapshot(job_id).expect("snapshot").state,
        JobState::Downloading
    );
    assert_eq!(reopened.current_attempt(job_id).expect("attempt"), 1);
    assert_eq!(
        reopened.internal_v1_attempt_phase(job_id).expect("phase"),
        Some(None)
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// D0 acceptance: a non-NULL, unsupported recovery contract version fails
/// explicitly before any state re-execution is attempted.
#[test]
fn startup_recovery_fails_closed_on_unknown_contract_version() {
    let database_path = temporary_database_path("startup-unknown-version");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
    let job_id = "job-startup-unknown-version";
    let tweet_id = "900002";
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    insert_downloaded_job(&mut persistence, job_id, tweet_id);
    drop(persistence);

    let database = Database::open(&database_path).expect("database");
    database
        .execute_batch(&format!(
            "UPDATE jobs SET recovery_contract_version = 99 WHERE id = '{job_id}'"
        ))
        .expect("install unknown contract version");
    drop(database);

    let runtime = ExecutorRuntime::new(database_path.clone());
    runtime.recover_startup().expect("startup recovery spawns");

    let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
    assert_eq!(failed.tweet_id, tweet_id);
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
    assert_eq!(
        reopened
            .stored_job_summary(job_id)
            .expect("summary")
            .last_error_code
            .as_deref(),
        Some("ARCHIVE_RECOVERY_UNSUPPORTED_VERSION")
    );
    assert!(
        !reopened
            .stored_events(job_id)
            .expect("events")
            .iter()
            .any(|(event_type, _)| event_type == "JOB_COMPLETED")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn startup_v2_missing_journal_remains_fail_closed() {
    let path = temporary_database_path("startup-v2-gate");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut persistence = StorageJobPersistence::open(&path).unwrap();
    insert_v2_downloaded_job(&mut persistence, "v2-gated", "900003");
    let database = Database::open(&path).unwrap();
    database
        .execute_batch("UPDATE jobs SET recovery_contract_version = 2 WHERE id = 'v2-gated'")
        .unwrap();
    let runtime = ExecutorRuntime::new(path.clone());
    let before = persistence.stored_events("v2-gated").unwrap();
    for _ in 0..2 {
        let results = runtime
            .recover_startup_with_v2(true)
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(results.outcomes.len(), 1);
        assert_eq!(results.outcomes[0].job_id, "v2-gated");
        assert!(results.outcomes[0].status == StartupRecoveryStatus::Refused);
        assert_eq!(
            persistence.snapshot("v2-gated").unwrap().state,
            JobState::Downloaded
        );
        assert_eq!(persistence.stored_events("v2-gated").unwrap(), before);
    }
    runtime.recover_startup().unwrap();
    wait_for_persisted_state(&path, "v2-gated", JobState::Failed);
    assert_eq!(
        persistence
            .stored_job_summary("v2-gated")
            .unwrap()
            .last_error_code
            .as_deref(),
        Some("ARCHIVE_COMMIT_RECOVERY_FAILED")
    );
    assert!(
        !persistence
            .stored_events("v2-gated")
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "JOB_COMPLETED")
    );
    runtime.executor.shutdown().unwrap();
    drop(database);
    drop(persistence);
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}

#[cfg(target_os = "linux")]
#[test]
fn startup_recovery_runs_new_internal_v1_job_before_first_attempt() {
    let database_path = temporary_database_path("startup-new-internal-v1");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec![
            "-u".into(),
            "-c".into(),
            r#"import sys,json,pathlib
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
c=json.loads(sys.stdin.readline())
p=pathlib.Path(c['staging_dir']); p.mkdir(parents=True,exist_ok=True); (p/'media.jpg').write_bytes(b'fixture-media')
r={'tweet_id':'12091','url':c['url'],'tweet_type':'post','media':[]}
d={'tweet_id':'12091','url':c['url'],'media':[{'index':1,'media_type':'photo','filename':'media.jpg','relative_path':'media.jpg','size_bytes':13}],'result':r}
print(json.dumps({'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':d,'result':r}),flush=True)
sys.stdin.read()"#
                .into(),
        ],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    };
    let runtime = ExecutorRuntime::with_production_factory(
        config.clone(),
        ProductionExecutionFactory::new(config),
        None,
    );
    let submission = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "startup-new-internal-v1-job",
            OutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                ..OutputSettings::default()
            },
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let job_id = submission.job_id.clone();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    runtime
        .service()
        .submit_persisted(&mut persistence, submission)
        .unwrap();
    assert_eq!(persistence.current_attempt(&job_id).unwrap(), 0);
    runtime.recover_startup().unwrap().join().unwrap();
    wait_for_persisted_state(&database_path, &job_id, JobState::Failed);
    assert_eq!(persistence.current_attempt(&job_id).unwrap(), 1);
    let summary = persistence.stored_job_summary(&job_id).unwrap();
    assert_eq!(
        summary.last_error_code.as_deref(),
        Some("EXECUTOR_WORKER_FAILED")
    );
    assert!(
        summary
            .last_error_message
            .as_deref()
            .is_some_and(|message| { message.contains("PUBLICATION_INCOMPLETE_DRAIN") })
    );
    assert!(
        Database::open(&database_path)
            .unwrap()
            .archive_recovery_journal(&job_id)
            .unwrap()
            .is_none()
    );
    assert!(!root.join("archive/Tweets/12091").exists());
    drop(runtime);
    drop(persistence);
    std::fs::remove_dir_all(root).unwrap();
}

/// A legacy job without an execution spec and without archive recovery
/// facts is classified as incomplete, not as a new-contract spec failure.
#[test]
fn startup_recovery_classifies_legacy_without_recovery_facts_as_incomplete() {
    let database_path = temporary_database_path("startup-missing-spec");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
    let job_id = "job-startup-missing-spec";
    let tweet_id = "900003";

    let database = Database::open(&database_path).expect("database");
    let tweet_row_id = database
        .insert_tweet(
            tweet_id,
            &format!("https://x.com/test/status/{tweet_id}"),
            "post",
            "",
            "now",
        )
        .expect("tweet");
    database
        .create_archive_job(job_id, tweet_row_id, "now")
        .expect("job without execution spec");
    drop(database);

    let runtime = ExecutorRuntime::new(database_path.clone());
    runtime.recover_startup().expect("startup recovery spawns");

    let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
    assert_eq!(failed.tweet_id, tweet_id);
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
    assert_eq!(
        reopened
            .stored_job_summary(job_id)
            .expect("summary")
            .last_error_code
            .as_deref(),
        Some("ARCHIVE_RECOVERY_LEGACY_INCOMPLETE")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn startup_recovery_classifies_legacy_without_execution_spec_consistently() {
    let database_path = temporary_database_path("startup-legacy-missing-spec");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
    let job_id = "job-startup-legacy-no-spec";
    let tweet_id = "900005";
    let database = Database::open(&database_path).expect("database");
    let tweet_row_id = database
        .insert_tweet(
            tweet_id,
            &format!("https://x.com/test/status/{tweet_id}"),
            "post",
            "",
            "now",
        )
        .expect("tweet");
    database
        .create_archive_job(job_id, tweet_row_id, "now")
        .expect("legacy job without execution spec");
    drop(database);

    let runtime = ExecutorRuntime::new(database_path.clone());
    runtime.recover_startup().expect("startup recovery spawns");
    let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
    assert_eq!(failed.tweet_id, tweet_id);
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
    assert_eq!(
        reopened
            .stored_job_summary(job_id)
            .expect("summary")
            .last_error_code
            .as_deref(),
        Some("ARCHIVE_RECOVERY_LEGACY_INCOMPLETE")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn startup_candidate_enumeration_failure_is_fatal_and_does_not_mutate_jobs() {
    let database_path = temporary_database_path("startup-enumeration-failure");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
    let job_id = "job-enumeration-failure";
    let tweet_id = "900006";
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    insert_downloaded_job(&mut persistence, job_id, tweet_id);
    let database = Database::open(&database_path).expect("reopen database");
    database
        .execute_batch("DROP TABLE jobs")
        .expect("break candidate enumeration");
    drop(database);

    let runtime = ExecutorRuntime::new(database_path.clone());
    let report = runtime
        .recover_startup()
        .expect("spawn recovery")
        .join()
        .expect("report");
    assert!(report.outcomes.is_empty());
    assert!(report.fatal_error.is_some());
    drop(runtime);
    drop(persistence);
    std::fs::remove_dir_all(root).unwrap();
}

/// The happy path through the production startup boundary: a valid
/// InternalV1 journal + on-disk manifest reconstructs the archive, completes
/// the job exactly once, and a repeated startup leaves it untouched.
#[test]
fn startup_recovery_completes_valid_internal_archive_and_repeats_idempotently() {
    let database_path = temporary_database_path("startup-valid-manifest");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    let config_dir = database_path.parent().unwrap().to_owned();
    std::fs::create_dir_all(&config_dir).expect("parent");
    let job_id = "job-startup-valid";
    let tweet_id = "900004";
    let final_directory = format!("Tweets/{tweet_id}");

    let mut database = Database::open(&database_path).expect("database");
    let tweet_row_id = database
        .insert_tweet(
            tweet_id,
            &format!("https://x.com/test/status/{tweet_id}"),
            "post",
            "",
            "now",
        )
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            job_id,
            tweet_row_id,
            2,
            &format!("test-request-{job_id}"),
            &serde_json::to_string(&ExecutionSpecV2 {
                request: archive_request(tweet_id),
                output_settings: OutputSettings::default(),
            })
            .expect("v2 spec"),
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("job with contract");
    database
        .begin_archive_attempt(job_id, "now")
        .expect("attempt");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        database
            .transition_job(job_id, state, "now")
            .expect("advance state");
    }

    let files =
        FileStore::with_staging_root(&root, config_dir.join("_staging")).expect("file store");
    let staging = files
        .create_attempt_staging_dir(job_id, 1)
        .expect("attempt-scoped staging");
    std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
    let sha256 = FileStore::sha256(staging.join("01.jpg")).expect("hash");
    let metadata = xarchive_core::ArchiveMetadata {
        schema_version: 1,
        tweet_id: tweet_id.to_owned(),
        url: format!("https://x.com/test/status/{tweet_id}"),
        tweet_type: "post".to_owned(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
        },
        created_at: None,
        text: "startup recovery".to_owned(),
        media: vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: Some("m1".to_owned()),
            media_type: "photo".to_owned(),
            file: "01.jpg".to_owned(),
            mime_type: Some("image/jpeg".to_owned()),
            size_bytes: 3,
            sha256,
        }],
        archived_at: "now".to_owned(),
        reply_to: None,
        quoted_tweet: None,
    };
    let manifest = xarchive_storage::ArchiveRecoveryManifest::from_metadata(
        job_id,
        1,
        tweet_row_id,
        Path::new(&final_directory),
        &metadata,
        None,
    )
    .expect("manifest");
    std::fs::write(
        staging.join(".xarchive-recovery.json"),
        serde_json::to_vec(&manifest).expect("manifest json"),
    )
    .expect("manifest file");
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("journal");
    database
        .transition_job(job_id, JobState::Downloaded, "now")
        .expect("simulate archive rows committed before terminal recovery");
    drop(database);

    let runtime = ExecutorRuntime::new(database_path.clone());
    let first_recovery = runtime
        .recover_startup()
        .expect("first startup recovery")
        .join()
        .expect("startup recovery thread");
    assert_eq!(
        first_recovery.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Recovered,
        }],
        "the PREPARED startup candidate is reported as recovered exactly once"
    );
    let completed = wait_for_persisted_state(&database_path, job_id, JobState::Complete);
    assert_eq!(completed.tweet_id, tweet_id);
    assert!(root.join(format!("{final_directory}/01.jpg")).is_file());
    assert!(
        !config_dir
            .join("_staging")
            .join(job_id)
            .join("attempt-1")
            .exists()
    );

    let database = Database::open(&database_path).expect("reopen");
    let journal = database
        .archive_recovery_journal(job_id)
        .expect("journal")
        .expect("journal row");
    assert_eq!(
        journal.phase,
        xarchive_storage::ArchiveRecoveryPhase::Committed
    );
    // Simulate a crash after the InternalV1 Storage journal committed but
    // before the runner's terminal transaction recorded DownloadCompleted.
    database
        .execute_batch(&format!(
            "DELETE FROM events WHERE job_id = '{job_id}' AND (event_type IN ('DOWNLOAD_COMPLETED','JOB_COMPLETED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state = 'COMPLETE')); UPDATE jobs SET state = 'DOWNLOADED', finished_at = NULL WHERE id = '{job_id}'"
        ))
        .expect("model committed archive before terminal transaction");
    drop(database);

    // A committed InternalV1 archive with a pending runner terminal
    // transaction is verified read-only, then its terminal events are
    // completed atomically from the immutable journal.
    let second_recovery = runtime
        .recover_startup()
        .expect("second startup recovery")
        .join()
        .expect("second startup recovery report");
    assert_eq!(
        second_recovery.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Recovered,
        }],
        "the COMMITTED startup candidate is reported as recovered exactly once"
    );
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
    assert_eq!(
        reopened.snapshot(job_id).expect("snapshot").state,
        JobState::Complete
    );
    let completion_events = reopened
        .stored_events(job_id)
        .expect("events")
        .iter()
        .filter(|(event_type, _)| event_type == "JOB_COMPLETED")
        .count();
    assert_eq!(completion_events, 1, "completion must stay single-shot");
    let download_events = reopened
        .stored_events(job_id)
        .expect("events")
        .iter()
        .filter(|(event_type, _)| event_type == "DOWNLOAD_COMPLETED")
        .count();
    assert_eq!(
        download_events, 1,
        "recovery must restore the transfer event once"
    );
    // A repeated startup after terminal repair is idempotent.
    runtime.recover_startup().expect("third startup recovery");
    thread::sleep(std::time::Duration::from_millis(300));
    let reopened = StorageJobPersistence::open(&database_path).expect("reopen after repeat");
    assert_eq!(
        reopened
            .stored_events(job_id)
            .unwrap()
            .iter()
            .filter(|(event_type, _)| event_type == "DOWNLOAD_COMPLETED")
            .count(),
        1,
    );
    let _ = std::fs::remove_dir_all(&root);
}
