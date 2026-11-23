#![cfg(windows)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use xarchive_download::{
    Aria2Supervisor, Aria2SupervisorConfig, Aria2TransferDriver, MediaTransferPlan,
    RefreshCoordinator, TransferControl, TransferDriver, TransferDriverConfig, TransferPlanItem,
};

#[test]
#[ignore = "requires an explicitly selected Windows aria2 executable and isolated output directory"]
fn supervisor_transfers_loopback_payload_and_removes_secret() {
    let program = std::path::PathBuf::from(std::env::var_os("XARCHIVE_TEST_ARIA2").unwrap());
    let root = std::path::PathBuf::from(std::env::var_os("XARCHIVE_TEST_ARIA2_ROOT").unwrap());
    assert!(program.is_absolute() && program.is_file());
    assert!(root.is_absolute() && root.is_dir());
    assert_eq!(
        std::fs::read(root.join("w6-aria2-test.marker")).unwrap(),
        b"isolated-aria2"
    );
    let output = root.join("payload-测试.bin");
    let secret_file = root.join("supervisor-secret.conf");
    assert!(!output.exists() && !secret_file.exists());

    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = server.local_addr().unwrap().port();
    server.set_nonblocking(true).unwrap();
    let fixture = b"xarchive-contained-aria2-fixture\n".repeat(32768);
    let expected = fixture.clone();
    let serving = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut requests = 0;
        loop {
            match server.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut chunk = [0u8; 1024];
                    while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        let received = stream.read(&mut chunk).unwrap();
                        assert!(received != 0 && request.len() + received <= 8192);
                        request.extend_from_slice(&chunk[..received]);
                    }
                    if requests == 0 {
                        assert!(request.starts_with(b"GET /expired HTTP/1."));
                        stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                        requests += 1;
                        continue;
                    }
                    assert!(request.starts_with(b"GET /fresh HTTP/1."));
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        fixture.len()
                    )
                    .unwrap();
                    stream.write_all(&fixture).unwrap();
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "fixture server timed out");
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("fixture server: {error}"),
            }
        }
    });

    let rpc_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let rpc_port = rpc_listener.local_addr().unwrap().port();
    drop(rpc_listener);
    let config = Aria2SupervisorConfig::new_with_random_secret(
        program.to_string_lossy(),
        "127.0.0.1",
        rpc_port,
    )
    .unwrap()
    .with_secret_file_path(secret_file.clone())
    .with_proxy_mode(xarchive_core::ProxyMode::Direct);
    let mut supervisor = Aria2Supervisor::spawn(config).unwrap();
    assert!(supervisor.is_running().unwrap());
    assert!(!supervisor.get_version().unwrap().is_empty());
    let driver = Aria2TransferDriver::new(
        supervisor.backend().clone(),
        TransferDriverConfig {
            poll_interval: Duration::from_millis(25),
            timeout: Duration::from_secs(20),
        },
    );
    let plan = MediaTransferPlan {
        items: vec![TransferPlanItem {
            media_id: "synthetic-media".into(),
            url: format!("http://127.0.0.1:{port}/expired"),
            directory: root.to_string_lossy().into_owned(),
            filename: "payload-测试.bin".into(),
            headers: vec![],
            media_type: "image".into(),
            mime_type: Some("application/octet-stream".into()),
        }],
    };
    let refresh_count = std::cell::Cell::new(0);
    let mut coordinator = RefreshCoordinator::new(
        |original: &MediaTransferPlan| {
            refresh_count.set(refresh_count.get() + 1);
            assert!(!output.exists(), "failed transfer left an output file");
            let mut refreshed = original.clone();
            refreshed.items[0].url = format!("http://127.0.0.1:{port}/fresh");
            Ok::<_, &'static str>(refreshed)
        },
        |candidate: &MediaTransferPlan| {
            driver.execute(candidate, &TransferControl::new(), &mut |_| {})
        },
    );
    let result = coordinator.execute(&plan).unwrap();
    assert!(result.refreshed);
    assert_eq!(refresh_count.get(), 1);
    let outcomes = result.outcomes;
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].media_id, "synthetic-media");
    assert_eq!(outcomes[0].total_bytes, Some(expected.len() as u64));
    assert_eq!(
        std::path::Path::new(outcomes[0].path.as_ref().unwrap()),
        output
    );
    serving.join().unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), expected);
    assert!(secret_file.is_file());
    supervisor.shutdown().unwrap();
    assert!(!supervisor.is_running().unwrap());
    assert!(!secret_file.exists());
}
