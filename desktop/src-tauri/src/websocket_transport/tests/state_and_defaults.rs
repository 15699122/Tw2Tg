use super::*;

#[test]
fn the_connection_state_follows_the_live_socket_not_a_recent_request() {
    let session = WebSocketSessionState::default();
    assert_eq!(session.browser_connection(), "not_loaded");
    assert!(!session.authenticated());
    assert_eq!(session.last_request_age_seconds(), None);

    // An authenticated connection with an open socket.
    session.connected_once.store(true, Ordering::Relaxed);
    session.authenticated_once.store(true, Ordering::Relaxed);
    session.active.store(1, Ordering::Relaxed);
    assert_eq!(session.browser_connection(), "connected");
    assert!(session.authenticated());

    // A request arrived, then the socket went away. The state must follow the
    // socket: claiming "connected" here would tell the user a connection
    // that no longer exists.
    if let Ok(mut last) = session.last_request.lock() {
        *last = Some(Instant::now());
    }
    session.active.store(0, Ordering::Relaxed);
    assert_eq!(
        session.browser_connection(),
        "disconnected",
        "a recent request never keeps a closed connection alive"
    );
    assert!(
        !session.authenticated(),
        "a dead socket is never reported as authenticated"
    );
    // The evidence an idle-duration question needs is still available.
    assert_eq!(session.last_request_age_seconds(), Some(0));
    assert_eq!(
        session.diagnostic_snapshot().last_request_age_seconds,
        Some(0)
    );
}

#[test]
fn pins_websocket_transport_defaults() {
    assert_eq!(xarchive_protocol::WEBSOCKET_DEFAULT_PORT, 17321);
    assert_eq!(
        xarchive_protocol::WEBSOCKET_PORT_ENV,
        "XARCHIVE_WEBSOCKET_PORT"
    );
    assert_eq!(
        xarchive_protocol::WEBSOCKET_TOKEN_ENV,
        "XARCHIVE_WEBSOCKET_TOKEN"
    );
}
