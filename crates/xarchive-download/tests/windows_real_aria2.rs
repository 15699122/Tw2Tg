#![cfg(windows)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use xarchive_download::{
    Aria2Supervisor, Aria2SupervisorConfig, Aria2TransferDriver, MediaTransferPlan,
    TransferControl, TransferDriver, TransferDriverConfig, TransferPlanItem,
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
        loop {
            match server.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut request = [0u8; 4096];
                    let received = stream.read(&mut request).unwrap();
                    assert!(request[..received].starts_with(b"GET /fixture HTTP/1."));
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
            url: format!("http://127.0.0.1:{port}/fixture"),
            directory: root.to_string_lossy().into_owned(),
            filename: "payload-测试.bin".into(),
            headers: vec![],
            media_type: "image".into(),
            mime_type: Some("application/octet-stream".into()),
        }],
    };
    let outcomes = driver
        .execute(&plan, &TransferControl::new(), &mut |_| {})
        .unwrap();
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
