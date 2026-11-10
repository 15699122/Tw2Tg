use super::*;

#[test]
fn deadline_cannot_be_extended_by_partial_reads() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (stream, _) = listener.accept().unwrap();
    let mut stream = DeadlineStream {
        stream,
        deadline: Some(Instant::now() + Duration::from_millis(40)),
        remaining_bytes: Some(4096),
        stop: None,
    };
    client.write_all(b"x").unwrap();
    let mut buffer = [0; 1];
    assert_eq!(stream.read(&mut buffer).unwrap(), 1);
    std::thread::sleep(Duration::from_millis(60));
    client.write_all(b"y").unwrap();
    assert_eq!(
        stream.read(&mut buffer).unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
}

#[test]
fn server_shutdown_interrupts_handshake_and_authenticated_idle_connections() {
    let executor = JobExecutor::new();
    let server = DesktopWebSocketServer::start(
        ArchiveApplicationService::new(&executor),
        std::env::temp_dir().join("unused-websocket-shutdown.sqlite3"),
        xarchive_storage::BatchOutputSettings::default(),
        false,
        crate::executor::ToolCompatibility::default(),
    )
    .unwrap();
    let response =
        server
            .pairing()
            .bootstrap(xarchive_protocol::BrowserPairingRequest::Bootstrap {
                protocol_version: 1,
                request_id: "r1".into(),
            });
    let xarchive_protocol::BrowserPairingResponse::Bootstrap { ticket, .. } = response else {
        panic!("bootstrap");
    };
    let (mut socket, _) = client(server.port());
    socket
        .send(Message::Text(
            serde_json::to_string(&AuthenticationEnvelope {
                protocol_version: 1,
                message_type: "authenticate".into(),
                ticket,
            })
            .unwrap()
            .into(),
        ))
        .unwrap();
    socket.read().unwrap();
    let _stalled = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
    std::thread::sleep(Duration::from_millis(40));
    let started = Instant::now();
    drop(server);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[cfg(unix)]
#[test]
fn unix_bootstrap_yields_a_ticket_for_the_current_listener() {
    use crate::transport::DesktopTransportServer;
    use std::os::unix::net::UnixStream;
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let database_path =
        std::env::temp_dir().join(format!("pairing-ipc-{}.sqlite3", std::process::id()));
    let endpoint = std::env::temp_dir().join(format!("pairing-ipc-{}.sock", std::process::id()));
    let server = DesktopWebSocketServer::start(
        service.clone(),
        database_path.clone(),
        xarchive_storage::BatchOutputSettings::default(),
        false,
        crate::executor::ToolCompatibility::default(),
    )
    .unwrap();
    let ipc = DesktopTransportServer::start_with_pairing(
        service,
        database_path,
        endpoint.clone(),
        Some(server.pairing()),
        xarchive_storage::BatchOutputSettings::default(),
        false,
        crate::executor::ToolCompatibility::default(),
    )
    .unwrap();
    let mut transport = UnixStream::connect(endpoint).unwrap();
    transport
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let response = xarchive_native_host::forward_bootstrap(
        &mut transport,
        xarchive_protocol::BrowserPairingRequest::Bootstrap {
            protocol_version: 1,
            request_id: "r1".into(),
        },
    )
    .unwrap();
    let xarchive_protocol::BrowserPairingResponse::Bootstrap {
        port,
        ticket,
        runtime_instance_id,
        ..
    } = response
    else {
        panic!("bootstrap");
    };
    assert_eq!(port, server.port());
    assert_eq!(runtime_instance_id, server.runtime_instance_id());
    let (mut socket, _) = client(port);
    socket
        .send(Message::Text(
            serde_json::to_string(&AuthenticationEnvelope {
                protocol_version: 1,
                message_type: "authenticate".into(),
                ticket,
            })
            .unwrap()
            .into(),
        ))
        .unwrap();
    let Message::Text(response) = socket.read().unwrap() else {
        panic!("authentication");
    };
    let response: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(response["authenticated"], true);
    drop(ipc);
    drop(server);
}
