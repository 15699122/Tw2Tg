use super::*;

#[cfg(unix)]
#[test]
fn natural_parent_exit_stops_descendant_writer_and_removes_secret() {
    use std::io::{BufRead, BufReader};
    let root = std::env::temp_dir().join(format!(
        "aria2-tree-{}-{}",
        std::process::id(),
        SECRET_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).expect("fixture directory");
    let late = root.join("late-write");
    let secret = root.join("secret.conf");
    std::fs::write(&secret, "fixture-secret").expect("secret");
    let mut command = Command::new("/bin/sh");
    command
        .args([
            "-c",
            "(sleep 0.3; printf late > \"$1\") & echo ready; exit 0",
            "fixture",
        ])
        .arg(&late)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    configure_process_group(&mut command);
    let mut child = command.spawn().expect("parent fixture");
    let mut ready = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut ready)
        .expect("read readiness");
    assert_eq!(ready.trim(), "ready");
    let mut supervisor = Aria2Supervisor {
        child: Some(child),
        #[cfg(target_os = "linux")]
        containment: None,
        client: Aria2HttpClient::new("127.0.0.1", 6800, "fixture").expect("client"),
        secret_file: Some(secret.clone()),
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    while supervisor.is_running().expect("poll parent") {
        assert!(Instant::now() < deadline, "parent must exit");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        supervisor.child.is_none(),
        "polling performs terminal cleanup"
    );
    assert!(
        supervisor
            .wait_for_exit(Duration::ZERO)
            .expect("repeat wait")
    );
    assert!(!supervisor.is_running().expect("repeat poll"));
    assert!(!secret.exists(), "secret removed on natural exit");
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        !late.exists(),
        "descendant must not write after parent exit cleanup"
    );
    supervisor.shutdown().expect("shutdown cleanup");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn startup_failure_owner_can_retry_real_secret_cleanup() {
    let root = std::env::temp_dir().join(format!(
        "aria2-startup-owner-retry-{}-{}",
        std::process::id(),
        SECRET_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).expect("fixture directory");
    let blocked_secret = root.join("secret.conf");
    std::fs::create_dir(&blocked_secret).expect("cleanup blocker");
    let client = Aria2HttpClient::new("127.0.0.1", 6800, "fixture").expect("client");
    let mut failure = Aria2StartupFailure {
        error: DownloadError::Process("startup fixture failure".into()),
        owner: Some(supervisor_with_secret_file(client, blocked_secret.clone())),
    };

    let first = failure
        .retry_cleanup()
        .expect_err("blocked cleanup must fail");
    assert!(first.to_string().contains("remove aria2 secret file"));
    assert!(failure.owner.is_some(), "owner must survive failed retry");
    std::fs::remove_dir(&blocked_secret).expect("remove cleanup blocker");
    std::fs::write(&blocked_secret, "fixture secret").expect("restore file");
    assert!(failure.retry_cleanup().expect("retry same owner"));
    assert!(failure.owner.is_none());
    assert!(!blocked_secret.exists());
    std::fs::remove_dir_all(root).expect("cleanup fixture");
}

#[test]
fn shutdown_reports_secret_file_removal_failure_and_can_retry() {
    let root = std::env::temp_dir().join(format!(
        "aria2-secret-cleanup-{}-{}",
        std::process::id(),
        SECRET_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).expect("fixture directory");
    let secret = root.join("secret.conf");
    let config_path = root.join("config");
    std::fs::create_dir(&secret).expect("directory at secret path");
    let client = Aria2HttpClient::new("127.0.0.1", 6800, "fixture").expect("client");
    let mut supervisor = supervisor_with_secret_file(client, secret.clone());

    let error = supervisor
        .shutdown()
        .expect_err("directory cannot be removed as file");
    assert!(error.to_string().contains("remove aria2 secret file"));
    assert!(
        supervisor.secret_file.is_some(),
        "failed cleanup remains retryable"
    );

    std::fs::rename(&secret, &config_path).expect("move blocking directory");
    std::fs::write(&secret, "secret").expect("replace with file");
    supervisor.shutdown().expect("retry secret cleanup");
    assert!(!secret.exists());
    std::fs::remove_dir_all(root).expect("cleanup fixture");
}
