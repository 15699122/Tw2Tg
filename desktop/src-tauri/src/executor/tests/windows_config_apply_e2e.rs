//! Windows W3 maintenance-drain E2E tests against a real contained writer:
//! - Cancel while contained writer holds workspace admission -> Contended + admission reopened
//! - Timeout while contained writer holds workspace admission -> Contended + admission reopened
//! - Full drain completion after contained writer IOCP drain and session release
//! - Real I/O failure (`LockError::Io`) distinguished from Contended (cancel/timeout)

use super::windows_containment_e2e::{windows_desktop_keeper_exe, windows_python_exe};
use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[test]
fn windows_config_apply_cancel_and_timeout_against_real_contained_writer_reopen_admission() {
    use xarchive_workflow::coordination::LockError;
    use xarchive_workflow::maintenance::WorkspaceGuard;
    use xarchive_workflow::{ExecutionReservation, PublicationOwner};

    let keeper_exe = windows_desktop_keeper_exe();
    let python_exe = windows_python_exe();
    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-w3-drain-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let config_dir = root.join("config");
    let database_path = config_dir.join("archive.sqlite3");
    let archive_root = root.join("archive");
    let staging_root = config_dir.join("_staging");
    let locks_dir = config_dir.join("locks");
    std::fs::create_dir_all(&archive_root).unwrap();
    std::fs::create_dir_all(&staging_root).unwrap();
    std::fs::create_dir_all(&locks_dir).unwrap();

    let mut database = Database::open(&database_path).unwrap();
    let tweet = database
        .insert_tweet(
            "12201",
            "https://x.com/alice/status/12201",
            "post",
            "",
            "now",
        )
        .unwrap();
    let spec_json = serde_json::to_string(&ExecutionSpecV2 {
        request: archive_request("12201"),
        output_settings: OutputSettings::default(),
    })
    .unwrap();
    database
        .create_archive_job_with_optional_recovery_contract(
            "win-w3-job",
            tweet,
            2,
            "win-w3-req",
            &spec_json,
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .unwrap();
    drop(database);

    let coordinator = crate::executor::ProductionCoordinator::open_from_workspace(&database_path)
        .expect("open windows coordinator from workspace");
    let identity = AttemptIdentity::new("win-w3-job", 1).unwrap();
    let session =
        ExecutionReservation::acquire(&coordinator, "win-w3-job", &archive_root.join("12201"))
            .unwrap()
            .bind_existing_attempt(identity.clone())
            .unwrap();
    let owner = PublicationOwner::for_session(&session);

    let script = concat!(
        "import json, sys;",
        "c = json.loads(sys.stdin.readline());",
        "print(json.dumps({",
        "'protocol_version': 2,",
        "'event': 'ready',",
        "'job_id': 'system',",
        "'request_id': c['request_id'],",
        "'capabilities': ['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']",
        "}), flush=True);",
        "sys.stdin.read()"
    );
    let factory = ProductionExecutionFactory::new(ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: archive_root.clone(),
        staging_root,
        database_path,
        sidecar_program: Some(python_exe.to_string_lossy().into_owned()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script.into()],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    })
    .with_windows_keeper_executable(keeper_exe);

    let execution = factory
        .create_registered_attempt(
            &identity,
            &JobSnapshot {
                job_id: "win-w3-job".into(),
                tweet_id: "12201".into(),
                state: JobState::Downloading,
            },
            owner.registration(),
        )
        .expect("create registered Windows attempt");

    // 1. Cancel case: drain aborts with Contended and immediately reopens admission.
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_bg = Arc::clone(&cancel_flag);
    let locks_dir_cancel = locks_dir.clone();
    let cancel_thread = std::thread::spawn(move || {
        WorkspaceGuard::drain(&locks_dir_cancel, || cancel_flag_bg.load(Ordering::Acquire))
    });
    std::thread::sleep(Duration::from_millis(100));
    cancel_flag.store(true, Ordering::Release);
    assert!(matches!(
        cancel_thread.join().unwrap(),
        Err(LockError::Contended)
    ));
    WorkspaceGuard::try_enter(&locks_dir)
        .expect("admission must reopen immediately after cancelled drain");

    // 2. Timeout case: drain aborts with Contended on deadline and reopens admission.
    let deadline = Instant::now() + Duration::from_millis(150);
    assert!(matches!(
        WorkspaceGuard::drain(&locks_dir, || Instant::now() >= deadline),
        Err(LockError::Contended)
    ));
    WorkspaceGuard::try_enter(&locks_dir)
        .expect("admission must reopen immediately after timed-out drain");

    // 3. Completion case: drain blocks while writer/workspace is active, then acquires
    //    MaintenanceGuard once IOCP drain completes and session/coordinator release.
    let (tx, rx) = std::sync::mpsc::channel();
    let locks_dir_drain = locks_dir.clone();
    let drain_thread = std::thread::spawn(move || {
        let result = WorkspaceGuard::drain(&locks_dir_drain, || false);
        let _ = tx.send(());
        result
    });
    assert!(
        rx.recv_timeout(Duration::from_millis(120)).is_err(),
        "maintenance drain must wait while contained writer and coordinator hold admission"
    );

    owner.close_admission().unwrap();
    drop(execution);
    assert!(owner.authorize(&session).is_ok());
    drop(owner);
    drop(session);
    drop(coordinator);

    rx.recv_timeout(Duration::from_secs(5))
        .expect("maintenance drain must complete after IOCP drain and workspace release");
    let maintenance = drain_thread
        .join()
        .unwrap()
        .expect("maintenance guard acquired after drain");
    assert!(matches!(
        WorkspaceGuard::try_enter(&locks_dir),
        Err(LockError::Contended)
    ));
    drop(maintenance);
    WorkspaceGuard::try_enter(&locks_dir).expect("admission must reopen after maintenance guard");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn windows_config_apply_io_failure_distinguished_from_cancel_and_timeout() {
    use xarchive_workflow::coordination::LockError;
    use xarchive_workflow::maintenance::WorkspaceGuard;

    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-w3-io-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let not_a_dir = root.join("not-a-dir");
    std::fs::write(&not_a_dir, b"occupied").unwrap();

    assert!(matches!(
        WorkspaceGuard::drain(&not_a_dir, || false),
        Err(LockError::Io)
    ));

    std::fs::remove_dir_all(root).unwrap();
}
