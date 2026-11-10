use super::*;

#[test]
fn oversized_first_frame_is_rejected_before_business() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let worker = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut socket =
            accept_hdr_with_config(stream, validate_handshake, Some(socket_config())).unwrap();
        assert!(matches!(
            socket.read(),
            Err(tungstenite::Error::Capacity(_))
        ));
    });
    let (mut socket, _) = client(port);
    let _ = socket.send(Message::Text("x".repeat(MAX_MESSAGE_BYTES + 1).into()));
    worker.join().unwrap();
}

#[test]
fn preauth_connections_are_capped_and_reclaimed() {
    let executor = JobExecutor::new();
    let server = DesktopWebSocketServer::start(
        ArchiveApplicationService::new(&executor),
        std::env::temp_dir().join("unused-websocket-cap.sqlite3"),
        xarchive_storage::BatchOutputSettings::default(),
        false,
        crate::executor::ToolCompatibility::default(),
    )
    .unwrap();
    let streams: Vec<_> = (0..MAX_CONNECTIONS + 8)
        .map(|_| TcpStream::connect(("127.0.0.1", server.port())).unwrap())
        .collect();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        server.session.diagnostic_snapshot().accepted,
        MAX_CONNECTIONS
    );
    drop(streams);
    std::thread::sleep(Duration::from_millis(50));
    let (socket, _) = client(server.port());
    assert_eq!(
        server.session.diagnostic_snapshot().accepted,
        MAX_CONNECTIONS + 1
    );
    drop(socket);
    drop(server);
}

#[test]
fn preauth_byte_budget_is_total_across_reads() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (stream, _) = listener.accept().unwrap();
    let mut stream = DeadlineStream {
        stream,
        deadline: Some(Instant::now() + Duration::from_secs(1)),
        remaining_bytes: Some(2),
        stop: None,
    };
    client.write_all(b"abc").unwrap();
    let mut buffer = [0; 8];
    assert_eq!(stream.read(&mut buffer).unwrap(), 2);
    assert_eq!(
        stream.read(&mut buffer).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}
