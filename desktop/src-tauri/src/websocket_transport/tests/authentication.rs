use super::*;

#[test]
fn authentication_consumes_ticket_and_returns_a_response() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let port = listener.local_addr().expect("local address").port();
    let stop = Arc::new(AtomicBool::new(false));
    let session = Arc::new(WebSocketSessionState::default());
    let stop_for_thread = stop.clone();
    let session_for_thread = session.clone();
    let (pairing, ticket) = credentials();
    let thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut socket = accept(stream).expect("websocket handshake");
        assert!(authenticate(
            &mut socket,
            &pairing,
            &stop_for_thread,
            &session_for_thread
        ));
    });
    let (mut socket, _response) = client(port);
    socket
        .send(Message::Text(
            serde_json::to_string(&AuthenticationEnvelope {
                protocol_version: PROTOCOL_VERSION,
                message_type: "authenticate".to_owned(),
                ticket,
            })
            .unwrap()
            .into(),
        ))
        .expect("send authentication");
    let response = socket.read().expect("read authentication response");
    let Message::Text(response) = response else {
        panic!("expected text authentication response");
    };
    let response: serde_json::Value =
        serde_json::from_str(&response).expect("decode authentication response");
    assert_eq!(response["authenticated"], serde_json::Value::Bool(true));
    assert_eq!(response["message_type"], "authentication_response");
    thread.join().expect("server thread");
    let diagnostics = session.diagnostic_snapshot();
    assert_eq!(diagnostics.auth_received, 1);
    assert_eq!(diagnostics.auth_succeeded, 1);
    assert_eq!(diagnostics.auth_failed, 0);
    assert_eq!(diagnostics.auth_response_failed, 0);
}

#[test]
fn authenticated_connection_routes_a_browser_request() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let port = listener.local_addr().expect("local address").port();
    let database_path = std::env::temp_dir().join(format!(
        "xarchive-websocket-test-{}.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&database_path);
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop.clone();
    let session = Arc::new(WebSocketSessionState::default());
    let session_for_thread = session.clone();
    let database_path_for_thread = database_path.clone();
    let (pairing, ticket) = credentials();
    let thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        handle_websocket_connection(
            &service,
            &database_path_for_thread,
            stream,
            &pairing,
            &session_for_thread,
            &stop_for_thread,
            &xarchive_storage::BatchOutputSettings::default(),
            false,
            &crate::executor::ToolCompatibility::default(),
        );
    });
    let (mut socket, _response) = client(port);
    socket
        .send(Message::Text(
            serde_json::to_string(&AuthenticationEnvelope {
                protocol_version: PROTOCOL_VERSION,
                message_type: "authenticate".to_owned(),
                ticket,
            })
            .unwrap()
            .into(),
        ))
        .expect("send authentication");
    assert!(matches!(
        socket.read().expect("auth response"),
        Message::Text(_)
    ));
    let request = BrowserRequest::QueryStatus {
        protocol_version: PROTOCOL_VERSION,
        request_id: "ws-query-1".to_owned(),
        tweet_ids: vec!["123".to_owned()],
    };
    socket
        .send(Message::Text(
            serde_json::to_string(&request).unwrap().into(),
        ))
        .expect("send query");
    let Message::Text(response) = socket.read().expect("query response") else {
        panic!("expected text response");
    };
    let response: BrowserResponse = serde_json::from_str(&response).expect("decode response");
    assert!(
        matches!(response, BrowserResponse::ArchiveStatusBatch { ref request_id, .. } if request_id == "ws-query-1")
    );
    socket.close(None).expect("close client");
    // A graceful client close must be answered with a server close frame:
    // dropping the socket without replying leaves the peer waiting and
    // surfaces as "close did not complete in 3s" in controlled probes.
    let replied = socket.read().expect("server close reply");
    assert!(
        matches!(replied, Message::Close(_)),
        "server must echo the close handshake, got {replied:?}"
    );
    thread.join().expect("server thread");
    let diagnostics = session.diagnostic_snapshot();
    assert_eq!(diagnostics.accepted, 0);
    assert_eq!(diagnostics.auth_received, 1);
    assert_eq!(diagnostics.auth_succeeded, 1);
    assert_eq!(diagnostics.close_after_auth, 1);
    let _ = std::fs::remove_file(database_path);
}

#[test]
fn authentication_rejects_a_wrong_ticket() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let port = listener.local_addr().expect("local address").port();
    let stop = Arc::new(AtomicBool::new(false));
    let session = Arc::new(WebSocketSessionState::default());
    let stop_for_thread = stop.clone();
    let session_for_thread = session.clone();
    let (pairing, ticket) = credentials();
    let thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut socket = accept(stream).expect("websocket handshake");
        assert!(!authenticate(
            &mut socket,
            &pairing,
            &stop_for_thread,
            &session_for_thread
        ));
    });
    let (mut socket, _response) = client(port);
    socket
        .send(Message::Text(
            serde_json::to_string(&AuthenticationEnvelope {
                protocol_version: PROTOCOL_VERSION,
                message_type: "authenticate".to_owned(),
                ticket: if ticket == "b".repeat(64) {
                    "c".repeat(64)
                } else {
                    "b".repeat(64)
                },
            })
            .unwrap()
            .into(),
        ))
        .expect("send authentication");
    thread.join().expect("server thread");
}

#[test]
fn authenticates_when_the_accepted_stream_starts_non_blocking() {
    // Winsock propagates the listener's non-blocking mode onto the accepted
    // socket, unlike POSIX. A client that connects and then sends its
    // authentication frame slightly later must still be served, so the
    // connection handler has to restore blocking mode itself. This test
    // reproduces that inheritance explicitly.
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let port = listener.local_addr().expect("local address").port();
    let database_path = std::env::temp_dir().join(format!(
        "xarchive-websocket-nonblocking-{}.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&database_path);
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop.clone();
    let session = Arc::new(WebSocketSessionState::default());
    let session_for_thread = session.clone();
    let database_path_for_thread = database_path.clone();
    let (pairing, ticket) = credentials();
    let thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        // Simulate the Winsock inheritance that POSIX does not perform.
        stream
            .set_nonblocking(true)
            .expect("simulate inherited non-blocking mode");
        handle_websocket_connection(
            &service,
            &database_path_for_thread,
            stream,
            &pairing,
            &session_for_thread,
            &stop_for_thread,
            &xarchive_storage::BatchOutputSettings::default(),
            false,
            &crate::executor::ToolCompatibility::default(),
        );
    });
    let (mut socket, _response) = client(port);
    // Delay the authentication frame so a non-blocking read would fail.
    std::thread::sleep(Duration::from_millis(50));
    socket
        .send(Message::Text(
            serde_json::to_string(&AuthenticationEnvelope {
                protocol_version: PROTOCOL_VERSION,
                message_type: "authenticate".to_owned(),
                ticket,
            })
            .unwrap()
            .into(),
        ))
        .expect("send authentication");
    assert!(matches!(
        socket.read().expect("auth response"),
        Message::Text(_)
    ));
    socket.close(None).expect("close client");
    thread.join().expect("server thread");
    let diagnostics = session.diagnostic_snapshot();
    assert_eq!(diagnostics.handshake_failed, 0);
    assert_eq!(diagnostics.auth_read_failed, 0);
    assert_eq!(diagnostics.auth_received, 1);
    assert_eq!(diagnostics.auth_succeeded, 1);
    let _ = std::fs::remove_file(database_path);
}

#[test]
fn handshake_requires_exact_single_origin_and_resource() {
    for origin in [
        None,
        Some("https://x.com"),
        Some("null"),
        Some("chrome-extension://iaajefkoanbkleojofoadeakelihbjne/"),
    ] {
        let mut builder = Request::builder().uri("/");
        if let Some(origin) = origin {
            builder = builder.header("Origin", origin);
        }
        assert!(validate_handshake(&builder.body(()).unwrap(), Response::new(())).is_err());
    }
    for resource in ["/other", "/?ticket=secret"] {
        let request = Request::builder()
            .uri(resource)
            .header("Origin", EXTENSION_ORIGIN)
            .body(())
            .unwrap();
        assert!(validate_handshake(&request, Response::new(())).is_err());
    }
    let request = Request::builder()
        .uri("/")
        .header("Origin", EXTENSION_ORIGIN)
        .body(())
        .unwrap();
    assert!(validate_handshake(&request, Response::new(())).is_ok());
    let request = Request::builder()
        .uri("/")
        .header("Origin", EXTENSION_ORIGIN)
        .header("Origin", EXTENSION_ORIGIN)
        .body(())
        .unwrap();
    assert!(validate_handshake(&request, Response::new(())).is_err());
}
