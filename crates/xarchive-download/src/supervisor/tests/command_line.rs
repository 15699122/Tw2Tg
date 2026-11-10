use super::*;

#[cfg(unix)]
#[test]
fn spawn_preserves_argv_order_and_keeps_rpc_secret_out_of_argv() {
    use std::os::unix::fs::PermissionsExt;

    let _temp_dir_guard = super::super::super::tests::lock_aria2_secret_temp_dir_for_test();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind readiness server");
    let port = listener.local_addr().expect("server address").port();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let requests_for_server = Arc::clone(&requests);
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept readiness request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set server read timeout");
        let mut request = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let count = stream.read(&mut buffer).expect("read readiness request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                let header_end = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .expect("HTTP header terminator")
                    + 4;
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().expect("content length"))
                    })
                    .expect("content length header");
                if request.len() >= header_end + content_length {
                    break;
                }
            }
        }
        requests_for_server
            .lock()
            .expect("request lock")
            .push(request.clone());
        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("HTTP header terminator")
            + 4;
        let body: serde_json::Value =
            serde_json::from_slice(&request[header_end..]).expect("JSON-RPC request");
        assert_eq!(body["method"], "aria2.getVersion");
        assert_eq!(body["params"][0], "token:spawn-rpc-secret");
        let response = json!({
            "jsonrpc": "2.0",
            "id": body["id"],
            "result": {"version": "fake-aria2"}
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.len(),
            response
        )
        .expect("write readiness response");
    });

    let capture_dir = std::env::temp_dir().join(format!(
        "aria2-spawn-argv-{}-{}",
        std::process::id(),
        SECRET_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&capture_dir).expect("isolated capture directory");
    let argv_path = capture_dir.join("argv.json");
    let fake_executable = capture_dir.join("fake-aria2");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nexec sleep 10\n",
        argv_path.display()
    );
    std::fs::write(&fake_executable, script).expect("write fake executable");
    let mut permissions = std::fs::metadata(&fake_executable)
        .expect("fake executable metadata")
        .permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&fake_executable, permissions).expect("make executable");

    let config = Aria2SupervisorConfig::new(
        fake_executable.to_string_lossy(),
        "127.0.0.1",
        port,
        "spawn-rpc-secret",
    )
    .expect("supervisor config")
    .with_startup_timeout(Duration::from_secs(2))
    .with_user_args(vec!["--max-tries".into(), "7".into()]);
    let mut supervisor = Aria2Supervisor::spawn(config).expect("fake aria2 starts");
    server.join().expect("readiness server completes");

    let captured: Vec<String> = std::fs::read_to_string(&argv_path)
        .expect("captured child argv")
        .lines()
        .map(str::to_owned)
        .collect();
    let secret_path = captured
        .last()
        .and_then(|arg| arg.strip_prefix("--conf-path="))
        .expect("secret-file option is the final argv entry");
    assert_eq!(captured[0], "--enable-rpc=true");
    assert_eq!(captured[1], "--rpc-listen-all=false");
    assert!(captured.windows(2).any(|pair| pair == ["--max-tries", "7"]));
    assert_eq!(captured.last(), Some(&format!("--conf-path={secret_path}")));
    assert!(
        !requests
            .lock()
            .expect("request lock")
            .iter()
            .any(|request| String::from_utf8_lossy(request).contains(secret_path))
    );
    assert!(!captured.iter().any(|arg| arg.contains("spawn-rpc-secret")));
    assert_eq!(
        std::fs::read_to_string(secret_path).expect("secret config contents"),
        "rpc-secret=spawn-rpc-secret\n"
    );
    assert_eq!(
        std::fs::metadata(secret_path)
            .expect("secret config metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let request_bytes = requests.lock().expect("request lock");
    let request_text = String::from_utf8_lossy(&request_bytes[0]);
    assert!(request_text.contains("token:spawn-rpc-secret"));
    drop(request_bytes);

    supervisor.shutdown().expect("shutdown cleanup");
    assert!(!PathBuf::from(secret_path).exists());
    assert!(
        !std::fs::read_to_string(&argv_path)
            .expect("captured argv")
            .contains("rpc-secret=spawn-rpc-secret")
    );
    std::fs::remove_file(argv_path).expect("remove argv capture");
    std::fs::remove_file(fake_executable).expect("remove fake executable");
    std::fs::remove_dir(capture_dir).expect("remove capture directory");
}

#[test]
fn random_rpc_secrets_are_fresh_hex_values() {
    let first = Aria2SupervisorConfig::new_with_random_secret("aria2c", "127.0.0.1", 6800)
        .expect("first random configuration");
    let second = Aria2SupervisorConfig::new_with_random_secret("aria2c", "127.0.0.1", 6800)
        .expect("second random configuration");
    assert_eq!(first.rpc_secret.len(), 64);
    assert!(
        first
            .rpc_secret
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    );
    assert_ne!(first.rpc_secret, second.rpc_secret);
    assert!(!format!("{first:?}").contains(&first.rpc_secret));
}

#[test]
fn user_args_are_appended_after_the_application_options() {
    // Batch E: validated per-task user options are appended after the
    // application-owned options so a user value wins over an application
    // default for a shared option. They are already allowlist/value vetted
    // upstream; a protected option never reaches this point.
    let config = config()
        .with_network(Duration::from_secs(30), Duration::from_secs(60), 3)
        .with_user_args(vec![
            "--max-tries".to_owned(),
            "9".to_owned(),
            "--split".to_owned(),
            "4".to_owned(),
        ]);
    let args = config.command_args();
    // The user's --max-tries overrides the application default of 3 because
    // it appears later on the argv; both are present, the last one wins.
    assert_eq!(
        args.iter()
            .filter(|a| a.as_str() == "--max-tries=3")
            .count(),
        1
    );
    assert!(args.contains(&"--max-tries".to_owned()));
    assert!(args.contains(&"9".to_owned()));
    assert!(args.contains(&"--split".to_owned()));
    assert!(args.contains(&"4".to_owned()));
    // The RPC secret is never placed on the argv by user args.
    assert!(!args.iter().any(|a| a.contains("rpc-secret")));
    // The application-owned output/proxy/RPC options are still intact and
    // the user options are last.
    assert_eq!(args.last(), Some(&"4".to_owned()));
}

#[test]
fn command_args_without_user_args_matches_the_previous_shape() {
    // No user args means the argv is unchanged from before Batch E wiring.
    let args = config().command_args();
    assert!(args.contains(&"--connect-timeout=30".to_owned()));
    assert!(args.contains(&"--timeout=60".to_owned()));
    assert!(args.contains(&"--max-tries=3".to_owned()));
    assert_eq!(args.last(), Some(&"--quiet=true".to_owned()));
}
