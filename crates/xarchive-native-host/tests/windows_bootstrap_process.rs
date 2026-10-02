#![cfg(windows)]
use interprocess::os::windows::named_pipe::{PipeListenerOptions, pipe_mode};
use std::io::Cursor;
use std::process::{Command, Stdio};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::{Duration, Instant};
use xarchive_native_host::{read_json, windows_pipe::BoundedPipe, write_json};
use xarchive_protocol::{BrowserPairingRequest, BrowserPairingResponse};

#[test]
fn native_process_forwards_windows_bootstrap_frames() {
    let endpoint = format!(r"\\.\pipe\xarchive-host-test-{}", std::process::id());
    let listener = PipeListenerOptions::new()
        .path(endpoint.as_str())
        .accept_remote(false)
        .nonblocking(true)
        .create_duplex::<pipe_mode::Bytes>()
        .unwrap();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let stream = loop {
            match listener.accept() {
                Ok(stream) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(error) => panic!("pipe accept: {error}"),
            }
        };
        let mut stream =
            BoundedPipe::new(stream, Arc::new(AtomicBool::new(false)), deadline).unwrap();
        let request: BrowserPairingRequest = read_json(&mut stream).unwrap().unwrap();
        request.validate().unwrap();
        write_json(
            &mut stream,
            &BrowserPairingResponse::Bootstrap {
                protocol_version: 1,
                request_id: "process-bootstrap".into(),
                host: "127.0.0.1".into(),
                port: 45678,
                path: "/".into(),
                runtime_instance_id: "windows-runtime".into(),
                ticket: "a".repeat(64),
                expires_in_ms: 30000,
            },
        )
        .unwrap();
        stream.finish_response();
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_xarchive-native-host"))
        .arg("chrome-extension://iaajefkoanbkleojofoadeakelihbjne/")
        .env("XARCHIVE_PIPE_ENDPOINT", endpoint)
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
            request_id: "process-bootstrap".into(),
        },
    )
    .unwrap();
    drop(input);
    let deadline = Instant::now() + Duration::from_secs(8);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("Host process deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let mut frames = Cursor::new(output.stdout);
    let response: BrowserPairingResponse = read_json(&mut frames).unwrap().unwrap();
    response.validate().unwrap();
    assert!(
        matches!(response, BrowserPairingResponse::Bootstrap {request_id,port:45678,..} if request_id == "process-bootstrap")
    );
    assert!(
        read_json::<_, serde_json::Value>(&mut frames)
            .unwrap()
            .is_none()
    );
    worker.join().unwrap();
}
