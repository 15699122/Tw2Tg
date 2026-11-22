//! Windows end-to-end Job Object containment integration tests.

use super::*;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn windows_desktop_keeper_exe() -> PathBuf {
    if let Some(path) = std::env::var_os("TW2TG_DESKTOP_KEEPER_EXE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return path;
        }
    }
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    for profile in ["release", "debug"] {
        let candidate = workspace_root
            .join("target")
            .join(profile)
            .join("xarchive-desktop.exe");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("xarchive-desktop.exe not found; build it or set TW2TG_DESKTOP_KEEPER_EXE");
}

fn windows_python_exe() -> PathBuf {
    if let Some(path) = std::env::var_os("PYTHON") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return path;
        }
    }
    let output = Command::new("where.exe")
        .arg("python")
        .output()
        .expect("run where.exe python");
    assert!(output.status.success(), "python not found on PATH");
    let first = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .expect("python path line")
        .to_owned();
    PathBuf::from(first)
}

fn find_windows_aria2_exe() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XARCHIVE_ARIA2_PROGRAM") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    for candidate in [
        workspace_root.join("dist-portable/aria2c.exe"),
        workspace_root.join("dist-portable/bin/aria2c.exe"),
        workspace_root.join("sidecar/xarchive-downloader/aria2c.exe"),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let output = Command::new("where.exe").arg("aria2c").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

fn is_windows_pid_alive(python_exe: &Path, pid: u32) -> bool {
    let status = Command::new(python_exe)
        .args([
            "-c",
            "import ctypes,sys; h=ctypes.windll.kernel32.OpenProcess(0x1000,0,int(sys.argv[1])); c=ctypes.c_ulong(); ok=bool(h and ctypes.windll.kernel32.GetExitCodeProcess(h,ctypes.byref(c)) and c.value==259); (ctypes.windll.kernel32.CloseHandle(h) if h else None); sys.exit(0 if ok else 1)",
            &pid.to_string(),
        ])
        .status()
        .expect("check pid via kernel32");
    status.success()
}

#[test]
fn windows_production_factory_contained_sidecar_drains_grandchild_and_authorizes_publication() {
    use xarchive_workflow::coordination::windows::WindowsCoordinator;
    use xarchive_workflow::{ExecutionReservation, PublicationOwner};

    let keeper_exe = windows_desktop_keeper_exe();
    let python_exe = windows_python_exe();
    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-e2e-sidecar-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let archive_root = root.join("archive");
    let staging_root = root.join("staging");
    let database_path = root.join("db.sqlite3");
    let marker = root.join("grandchild.pid");
    std::fs::create_dir_all(&archive_root).unwrap();
    std::fs::create_dir_all(&staging_root).unwrap();

    let database = Database::open(&database_path).unwrap();
    let tweet = database
        .insert_tweet(
            "12091",
            "https://x.com/alice/status/12091",
            "post",
            "",
            "now",
        )
        .unwrap();
    let spec_json = serde_json::to_string(&ExecutionSpecV2 {
        request: archive_request("12091"),
        output_settings: OutputSettings::default(),
    })
    .unwrap();
    database
        .create_archive_job_with_optional_recovery_contract(
            "win-e2e-job",
            tweet,
            2,
            "win-e2e-request",
            &spec_json,
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .unwrap();
    drop(database);

    let coordinator = WindowsCoordinator::new(root.join("locks")).unwrap();
    let identity = AttemptIdentity::new("win-e2e-job", 1).unwrap();
    let session =
        ExecutionReservation::acquire(&coordinator, "win-e2e-job", &archive_root.join("12091"))
            .unwrap()
            .bind_existing_attempt(identity.clone())
            .unwrap();
    let owner = PublicationOwner::for_session(&session);

    let script = concat!(
        "import json, pathlib, subprocess, sys;",
        "child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)']);",
        "pathlib.Path(sys.argv[1]).write_text(str(child.pid), encoding='utf-8');",
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
        staging_root: staging_root.clone(),
        database_path,
        sidecar_program: Some(python_exe.to_string_lossy().into_owned()),
        gallery_dl_program: None,
        sidecar_args: vec![
            "-u".into(),
            "-c".into(),
            script.into(),
            marker.to_string_lossy().into_owned(),
        ],
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
                job_id: "win-e2e-job".into(),
                tweet_id: "12091".into(),
                state: JobState::Downloading,
            },
            owner.registration(),
        )
        .expect("create registered Windows attempt");

    let grandchild_pid: u32 = std::fs::read_to_string(&marker)
        .expect("grandchild pid marker")
        .trim()
        .parse()
        .expect("parse grandchild pid");
    assert!(
        is_windows_pid_alive(&python_exe, grandchild_pid),
        "grandchild process {grandchild_pid} must be alive before drain"
    );

    let record_root = crate::executor::containment_record_root(&staging_root).unwrap();
    let record_count_before = std::fs::read_dir(&record_root).unwrap().count();
    assert!(
        record_count_before > 0,
        "containment record must exist while writer job is active"
    );

    owner.close_admission().unwrap();
    assert!(
        owner.authorize(&session).is_err(),
        "publication must be refused while contained writer is still active"
    );

    drop(execution);

    let deadline = Instant::now() + Duration::from_secs(5);
    while is_windows_pid_alive(&python_exe, grandchild_pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !is_windows_pid_alive(&python_exe, grandchild_pid),
        "grandchild process {grandchild_pid} must be terminated after Job Object IOCP drain"
    );
    assert!(
        owner.authorize(&session).is_ok(),
        "publication must succeed once Windows writer job is proven drained"
    );

    drop(owner);
    drop(session);
    drop(coordinator);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn windows_contained_aria2_supervisor_drains_and_authorizes_publication() {
    use xarchive_download::{Aria2Supervisor, Aria2SupervisorConfig};
    use xarchive_workflow::coordination::windows::WindowsCoordinator;
    use xarchive_workflow::{ExecutionReservation, PublicationOwner};

    let keeper_exe = windows_desktop_keeper_exe();
    let python_exe = windows_python_exe();
    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-e2e-aria2-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let archive_root = root.join("archive");
    let staging_root = root.join("staging");
    std::fs::create_dir_all(&archive_root).unwrap();
    std::fs::create_dir_all(&staging_root).unwrap();

    let coordinator = WindowsCoordinator::new(root.join("locks")).unwrap();

    // Unconditional silent-writer path check matching aria2-0 containment ownership.
    {
        let identity = AttemptIdentity::new("win-e2e-silent", 1).unwrap();
        let session = ExecutionReservation::acquire(
            &coordinator,
            "win-e2e-silent",
            &archive_root.join("12092"),
        )
        .unwrap()
        .bind_existing_attempt(identity.clone())
        .unwrap();
        let owner = PublicationOwner::for_session(&session);
        let record_root = crate::executor::containment_record_root(&staging_root).unwrap();
        let domain = crate::executor::WindowsAttemptWriterDomain {
            registration: Some(owner.registration()),
            record_root,
            workspace: staging_root.clone(),
            attempt: identity,
            keeper_executable: Some(keeper_exe.clone()),
        };
        let mut sidecar_job =
            crate::executor::recover_and_create_windows_writer_job(&domain, "sidecar")
                .expect("create sidecar windows writer job");
        sidecar_job
            .stop_and_drain(Duration::from_secs(10))
            .expect("drain sidecar job");
        let mut job = crate::executor::recover_and_create_windows_writer_job(&domain, "aria2-0")
            .expect("create aria2-0 windows writer job");
        job.start_desktop_keeper().expect("start keeper");
        job.spawn_silent_with_environment(
            &python_exe,
            &["-c", "import time; time.sleep(30)"],
            &[],
            &[],
        )
        .expect("spawn silent contained writer");
        owner.close_admission().unwrap();
        assert!(owner.authorize(&session).is_err());
        job.stop_and_drain(Duration::from_secs(10))
            .expect("stop and drain silent writer");
        assert!(owner.authorize(&session).is_ok());
    }

    // Optional real aria2c.exe path check when aria2c is present on the host.
    if let Some(aria2_exe) = find_windows_aria2_exe() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let identity = AttemptIdentity::new("win-e2e-aria2", 1).unwrap();
        let session = ExecutionReservation::acquire(
            &coordinator,
            "win-e2e-aria2",
            &archive_root.join("12093"),
        )
        .unwrap()
        .bind_existing_attempt(identity.clone())
        .unwrap();
        let owner = PublicationOwner::for_session(&session);
        let record_root = crate::executor::containment_record_root(&staging_root).unwrap();
        let domain = crate::executor::WindowsAttemptWriterDomain {
            registration: Some(owner.registration()),
            record_root,
            workspace: staging_root.clone(),
            attempt: identity,
            keeper_executable: Some(keeper_exe),
        };
        let mut sidecar_job =
            crate::executor::recover_and_create_windows_writer_job(&domain, "sidecar")
                .expect("create sidecar job");
        sidecar_job
            .stop_and_drain(Duration::from_secs(10))
            .expect("drain sidecar job");
        let job = crate::executor::recover_and_create_windows_writer_job(&domain, "aria2-0")
            .expect("create aria2-0 job");
        let config = Aria2SupervisorConfig::new(
            aria2_exe.to_string_lossy().into_owned(),
            "127.0.0.1",
            port,
            "win-e2e-secret",
        )
        .expect("aria2 config")
        .with_proxy_mode(ProxyMode::Direct);
        let mut supervisor = Aria2Supervisor::spawn_windows_contained_owned(config, job)
            .expect("spawn contained aria2");
        owner.close_admission().unwrap();
        assert!(owner.authorize(&session).is_err());
        supervisor.shutdown().expect("shutdown contained aria2");
        assert!(owner.authorize(&session).is_ok());
    }

    drop(coordinator);
    std::fs::remove_dir_all(root).unwrap();
}

fn prepare_internal_v1_prepared_job(
    root: &Path,
    job_id: &str,
    tweet_id: &str,
) -> (PathBuf, PathBuf) {
    let config_dir = root.join("config");
    let database_path = config_dir.join("archive.sqlite3");
    let staging_root = config_dir.join("_staging");
    std::fs::create_dir_all(&config_dir).unwrap();
    let mut database = Database::open(&database_path).unwrap();
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

    let files = FileStore::with_staging_root(root, &staging_root).expect("file store");
    let staging = files
        .create_attempt_staging_dir(job_id, 1)
        .expect("attempt staging");
    std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
    let sha256 = FileStore::sha256(staging.join("01.jpg")).expect("hash");
    let final_directory = format!("Tweets/{tweet_id}");
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
        .expect("downloaded state");
    drop(database);
    (database_path, staging_root)
}

#[test]
fn windows_startup_recovery_drains_live_crashed_predecessor_before_prepared_replay() {
    use xarchive_workflow::coordination::windows::WindowsCoordinator;
    use xarchive_workflow::{ExecutionReservation, PublicationOwner};

    let keeper_exe = windows_desktop_keeper_exe();
    let python_exe = windows_python_exe();
    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-e2e-crash-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let job_id = "win-crash-job";
    let tweet_id = "12094";
    let (database_path, staging_root) = prepare_internal_v1_prepared_job(&root, job_id, tweet_id);
    let locks_dir = database_path.parent().unwrap().join("locks");
    let pid_marker = root.join("crashed-writer.pid");

    {
        let coordinator = WindowsCoordinator::new(&locks_dir).unwrap();
        let identity = AttemptIdentity::new(job_id, 1).unwrap();
        let session = ExecutionReservation::acquire(
            &coordinator,
            job_id,
            &root.join("Tweets").join(tweet_id),
        )
        .unwrap()
        .bind_existing_attempt(identity.clone())
        .unwrap();
        let owner = PublicationOwner::for_session(&session);
        let record_root = crate::executor::containment_record_root(&staging_root).unwrap();
        let domain = crate::executor::WindowsAttemptWriterDomain {
            registration: Some(owner.registration()),
            record_root,
            workspace: staging_root.clone(),
            attempt: identity,
            keeper_executable: Some(keeper_exe),
        };
        let mut job = crate::executor::recover_and_create_windows_writer_job(&domain, "sidecar")
            .expect("create sidecar job");
        job.start_desktop_keeper().expect("start keeper");
        job.spawn_silent_with_environment(
            &python_exe,
            &[
                "-c",
                "import os, pathlib, sys, time; pathlib.Path(sys.argv[1]).write_text(str(os.getpid()), encoding='utf-8'); time.sleep(60)",
                pid_marker.to_str().unwrap(),
            ],
            &[],
            &[],
        )
        .expect("spawn silent writer");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pid_marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        // Drop job/owner/session without draining: keeper's inherited handle keeps the job alive.
    }

    let writer_pid: u32 = std::fs::read_to_string(&pid_marker)
        .expect("writer pid marker")
        .trim()
        .parse()
        .expect("parse writer pid");
    assert!(
        is_windows_pid_alive(&python_exe, writer_pid),
        "writer {writer_pid} must remain alive across owner drop via keeper"
    );

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
        "refusals: {:?}",
        first_recovery.refusals
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while is_windows_pid_alive(&python_exe, writer_pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !is_windows_pid_alive(&python_exe, writer_pid),
        "startup recovery must drain live predecessor writer {writer_pid}"
    );
    assert!(root.join(format!("Tweets/{tweet_id}/01.jpg")).is_file());
    let persisted = Database::open(&database_path)
        .unwrap()
        .job_state(job_id)
        .unwrap();
    assert_eq!(persisted, JobState::Complete);

    let second_recovery = runtime
        .recover_startup()
        .expect("second startup recovery")
        .join()
        .expect("second startup recovery thread");
    assert!(
        second_recovery.refusals.is_empty(),
        "idempotent recovery must not refuse: {:?}",
        second_recovery.refusals
    );
    assert!(root.join(format!("Tweets/{tweet_id}/01.jpg")).is_file());
    assert_eq!(
        Database::open(&database_path)
            .unwrap()
            .job_state(job_id)
            .unwrap(),
        JobState::Complete
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn windows_startup_recovery_refuses_corrupt_containment_record_without_publishing() {
    use xarchive_workflow::coordination::windows::WindowsCoordinator;
    use xarchive_workflow::{ExecutionReservation, PublicationOwner};

    let keeper_exe = windows_desktop_keeper_exe();
    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-e2e-corrupt-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let job_id = "win-corrupt-job";
    let tweet_id = "12095";
    let (database_path, staging_root) = prepare_internal_v1_prepared_job(&root, job_id, tweet_id);
    let locks_dir = database_path.parent().unwrap().join("locks");
    let record_root = crate::executor::containment_record_root(&staging_root).unwrap();

    {
        let coordinator = WindowsCoordinator::new(&locks_dir).unwrap();
        let identity = AttemptIdentity::new(job_id, 1).unwrap();
        let session = ExecutionReservation::acquire(
            &coordinator,
            job_id,
            &root.join("Tweets").join(tweet_id),
        )
        .unwrap()
        .bind_existing_attempt(identity.clone())
        .unwrap();
        let owner = PublicationOwner::for_session(&session);
        let domain = crate::executor::WindowsAttemptWriterDomain {
            registration: Some(owner.registration()),
            record_root: record_root.clone(),
            workspace: staging_root,
            attempt: identity,
            keeper_executable: Some(keeper_exe),
        };
        let _job = crate::executor::recover_and_create_windows_writer_job(&domain, "sidecar")
            .expect("create sidecar job");
    }

    let entries: Vec<PathBuf> = std::fs::read_dir(&record_root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1, "expected single containment record");
    std::fs::write(&entries[0], b"corrupted-partial-record").unwrap();

    let runtime = ExecutorRuntime::new(database_path.clone());
    let recovery = runtime
        .recover_startup()
        .expect("startup recovery")
        .join()
        .expect("startup recovery thread");
    assert_eq!(
        recovery.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Refused,
        }]
    );
    assert!(!root.join(format!("Tweets/{tweet_id}/01.jpg")).is_file());
    let persisted = Database::open(&database_path)
        .unwrap()
        .job_state(job_id)
        .unwrap();
    assert_ne!(persisted, JobState::Complete);

    std::fs::remove_dir_all(root).unwrap();
}
