#![cfg(unix)]
use std::io::{Cursor, Write};
use std::os::unix::net::UnixListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use xarchive_native_host::{read_json, write_json};
use xarchive_protocol::{
    BrowserPairingRequest, BrowserPairingResponse, BrowserRequest, BrowserResponse,
};

#[test]
fn native_process_keeps_bootstrap_and_business_frames_separate() {
    let endpoint =
        std::env::temp_dir().join(format!("native-bootstrap-{}.sock", std::process::id()));
    let listener = UnixListener::bind(&endpoint).unwrap();
    listener.set_nonblocking(true).unwrap();
    let worker = std::thread::spawn(move || {
        for control in [true, false] {
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("IPC connection unavailable: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let request: serde_json::Value = read_json(&mut stream).unwrap().unwrap();
            if control {
                assert_eq!(request["message_type"], "bootstrap");
                write_json(
                    &mut stream,
                    &BrowserPairingResponse::Bootstrap {
                        protocol_version: 1,
                        request_id: "bootstrap-1".into(),
                        host: "127.0.0.1".into(),
                        port: 43127,
                        path: "/".into(),
                        runtime_instance_id: "runtime".into(),
                        ticket: "a".repeat(64),
                        expires_in_ms: 30000,
                    },
                )
                .unwrap();
            } else {
                assert_eq!(request["message_type"], "query_status");
                write_json(
                    &mut stream,
                    &BrowserResponse::ArchiveStatusBatch {
                        protocol_version: 1,
                        request_id: "query-1".into(),
                        statuses: Vec::new(),
                    },
                )
                .unwrap();
            }
        }
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_xarchive-native-host"))
        .arg("chrome-extension://iaajefkoanbkleojofoadeakelihbjne/")
        .env("XARCHIVE_PIPE_ENDPOINT", &endpoint)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    write_json(
        &mut input,
        &BrowserPairingRequest::Bootstrap {
            protocol_version: 1,
            request_id: "bootstrap-1".into(),
        },
    )
    .unwrap();
    write_json(
        &mut input,
        &BrowserRequest::QueryStatus {
            protocol_version: 1,
            request_id: "query-1".into(),
            tweet_ids: vec!["123".into()],
        },
    )
    .unwrap();
    input.flush().unwrap();
    drop(input);
    let output = child.wait_with_output().unwrap();
    worker.join().unwrap();
    std::fs::remove_file(endpoint).unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let mut framed = Cursor::new(output.stdout);
    let control: BrowserPairingResponse = read_json(&mut framed).unwrap().unwrap();
    assert_eq!(control.validate(), Ok(()));
    let business: BrowserResponse = read_json(&mut framed).unwrap().unwrap();
    assert!(
        matches!(business, BrowserResponse::ArchiveStatusBatch { request_id, .. } if request_id == "query-1")
    );
    assert_eq!(
        read_json::<_, serde_json::Value>(&mut framed).unwrap(),
        None
    );
}
