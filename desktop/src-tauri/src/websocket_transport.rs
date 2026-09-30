//! Authenticated loopback WebSocket transport for the browser Extension.
//!
//! WebSocket framing and handshake are provided by `tungstenite`; this module
//! owns only the XArchive transport envelope and delegates business messages
//! to the existing `BrowserTransportAdapter`.

use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tungstenite::{Message, WebSocket, accept};
use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

use crate::executor::{ArchiveApplicationService, StorageJobPersistence};
use crate::transport::BrowserTransportAdapter;

const AUTH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthenticationEnvelope {
    protocol_version: u32,
    message_type: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct AuthenticationResponse {
    protocol_version: u32,
    message_type: &'static str,
    authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<&'static str>,
}

#[derive(Debug, Default)]
struct WebSocketDiagnostics {
    accepted: AtomicUsize,
    handshake_failed: AtomicUsize,
    auth_read_failed: AtomicUsize,
    auth_received: AtomicUsize,
    auth_succeeded: AtomicUsize,
    auth_failed: AtomicUsize,
    auth_response_failed: AtomicUsize,
    close_before_auth: AtomicUsize,
    close_after_auth: AtomicUsize,
}

#[derive(Default)]
pub(crate) struct WebSocketSessionState {
    active: AtomicUsize,
    connected_once: AtomicBool,
    authenticated_once: AtomicBool,
    last_request: std::sync::Mutex<Option<Instant>>,
    diagnostics: WebSocketDiagnostics,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct WebSocketDiagnosticSnapshot {
    pub accepted: usize,
    pub handshake_failed: usize,
    pub auth_read_failed: usize,
    pub auth_received: usize,
    pub auth_succeeded: usize,
    pub auth_failed: usize,
    pub auth_response_failed: usize,
    pub close_before_auth: usize,
    pub close_after_auth: usize,
}

impl WebSocketSessionState {
    pub(crate) fn diagnostic_snapshot(&self) -> WebSocketDiagnosticSnapshot {
        WebSocketDiagnosticSnapshot {
            accepted: self.diagnostics.accepted.load(Ordering::Relaxed),
            handshake_failed: self.diagnostics.handshake_failed.load(Ordering::Relaxed),
            auth_read_failed: self.diagnostics.auth_read_failed.load(Ordering::Relaxed),
            auth_received: self.diagnostics.auth_received.load(Ordering::Relaxed),
            auth_succeeded: self.diagnostics.auth_succeeded.load(Ordering::Relaxed),
            auth_failed: self.diagnostics.auth_failed.load(Ordering::Relaxed),
            auth_response_failed: self
                .diagnostics
                .auth_response_failed
                .load(Ordering::Relaxed),
            close_before_auth: self.diagnostics.close_before_auth.load(Ordering::Relaxed),
            close_after_auth: self.diagnostics.close_after_auth.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn browser_connection(&self) -> &'static str {
        if self.active.load(Ordering::Relaxed) > 0
            || self
                .last_request
                .lock()
                .ok()
                .and_then(|last| *last)
                .is_some_and(|last| last.elapsed() < Duration::from_secs(30))
        {
            "connected"
        } else if self.connected_once.load(Ordering::Relaxed) {
            "disconnected"
        } else {
            "not_loaded"
        }
    }

    pub(crate) fn authenticated(&self) -> bool {
        self.authenticated_once.load(Ordering::Relaxed) && self.active.load(Ordering::Relaxed) > 0
    }
}

pub(crate) struct DesktopWebSocketServer {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    port: u16,
    token: String,
    pub(crate) session: Arc<WebSocketSessionState>,
}

impl DesktopWebSocketServer {
    pub(crate) fn start(
        service: ArchiveApplicationService,
        database_path: std::path::PathBuf,
    ) -> Result<Self, String> {
        let port = configured_port()?;
        let token = configured_token()?;
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
            format!("failed to bind WebSocket listener on 127.0.0.1:{port}: {error}")
        })?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("failed to configure WebSocket listener: {error}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let session = Arc::new(WebSocketSessionState::default());
        let session_for_thread = session.clone();
        let token_for_thread = token.clone();
        let thread = std::thread::Builder::new()
            .name("xarchive-desktop-websocket".to_owned())
            .spawn(move || {
                while !stop_for_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            session_for_thread
                                .diagnostics
                                .accepted
                                .fetch_add(1, Ordering::Relaxed);
                            let service = service.clone();
                            let database_path = database_path.clone();
                            let session = session_for_thread.clone();
                            let token = token_for_thread.clone();
                            let stop = stop_for_thread.clone();
                            let _ = std::thread::Builder::new()
                                .name("xarchive-desktop-websocket-request".to_owned())
                                .spawn(move || {
                                    handle_websocket_connection(
                                        &service,
                                        &database_path,
                                        stream,
                                        &token,
                                        &session,
                                        &stop,
                                    );
                                });
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(25));
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("failed to start WebSocket listener: {error}"))?;
        Ok(Self {
            stop,
            thread: Some(thread),
            port,
            token,
            session,
        })
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }
}

impl Drop for DesktopWebSocketServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn configured_port() -> Result<u16, String> {
    match std::env::var(xarchive_protocol::WEBSOCKET_PORT_ENV) {
        Ok(value) => value
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| "XARCHIVE_WEBSOCKET_PORT must be a valid TCP port".to_owned()),
        Err(_) => Ok(xarchive_protocol::WEBSOCKET_DEFAULT_PORT),
    }
}

fn configured_token() -> Result<String, String> {
    if let Ok(value) = std::env::var(xarchive_protocol::WEBSOCKET_TOKEN_ENV)
        && !value.trim().is_empty()
    {
        return Ok(value);
    }
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| "failed to generate WebSocket pairing token".to_owned())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn handle_websocket_connection(
    service: &ArchiveApplicationService,
    database_path: &Path,
    stream: TcpStream,
    expected_token: &str,
    session: &WebSocketSessionState,
    stop: &AtomicBool,
) {
    // The accept loop uses a non-blocking listener only so it can poll the stop
    // flag. The accepted stream must be switched back to blocking mode: POSIX
    // `accept` does not inherit `O_NONBLOCK`, but Winsock does, which would make
    // the handshake and the authentication read fail immediately with
    // `WouldBlock` instead of waiting for the peer's frames.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(AUTH_TIMEOUT));
    let Ok(mut socket) = accept(stream) else {
        session
            .diagnostics
            .handshake_failed
            .fetch_add(1, Ordering::Relaxed);
        return;
    };
    if !authenticate(&mut socket, expected_token, stop, session) {
        session
            .diagnostics
            .close_before_auth
            .fetch_add(1, Ordering::Relaxed);
        return;
    }
    session.connected_once.store(true, Ordering::Relaxed);
    session.authenticated_once.store(true, Ordering::Relaxed);
    session.active.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut last) = session.last_request.lock() {
        *last = Some(Instant::now());
    }
    let _ = socket.get_ref().set_read_timeout(None);
    while !stop.load(Ordering::Relaxed) {
        let message = match socket.read() {
            Ok(message) => message,
            Err(_) => break,
        };
        match message {
            Message::Text(text) => {
                let response = match serde_json::from_str::<BrowserRequest>(&text) {
                    Ok(request) => match StorageJobPersistence::open(database_path) {
                        Ok(mut persistence) => BrowserTransportAdapter::with_database_path(
                            service.clone(),
                            database_path.to_owned(),
                        )
                        .handle_request(&mut persistence, request),
                        Err(error) => BrowserResponse::Error {
                            protocol_version: PROTOCOL_VERSION,
                            request_id: None,
                            error_code: "PERSISTENCE_ERROR".to_owned(),
                            error_message: error,
                            retryable: true,
                        },
                    },
                    Err(error) => BrowserResponse::Error {
                        protocol_version: PROTOCOL_VERSION,
                        request_id: None,
                        error_code: "INVALID_MESSAGE".to_owned(),
                        error_message: error.to_string(),
                        retryable: false,
                    },
                };
                if socket
                    .send(Message::Text(
                        serde_json::to_string(&response).unwrap_or_default().into(),
                    ))
                    .is_err()
                {
                    break;
                }
            }
            Message::Ping(payload) => {
                if socket.send(Message::Pong(payload)).is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
    session.active.fetch_sub(1, Ordering::Relaxed);
    session
        .diagnostics
        .close_after_auth
        .fetch_add(1, Ordering::Relaxed);
}

fn authenticate(
    socket: &mut WebSocket<TcpStream>,
    expected_token: &str,
    stop: &AtomicBool,
    session: &WebSocketSessionState,
) -> bool {
    if stop.load(Ordering::Relaxed) {
        return false;
    }
    let Ok(message) = socket.read() else {
        // A read error here means the peer went away before delivering a
        // complete authentication frame. Counting it separately from
        // `close_before_auth` lets a target environment tell "no frame arrived"
        // apart from "a frame arrived but was not a usable envelope".
        session
            .diagnostics
            .auth_read_failed
            .fetch_add(1, Ordering::Relaxed);
        return false;
    };
    let Message::Text(text) = message else {
        session
            .diagnostics
            .auth_read_failed
            .fetch_add(1, Ordering::Relaxed);
        return false;
    };
    session
        .diagnostics
        .auth_received
        .fetch_add(1, Ordering::Relaxed);
    let envelope = serde_json::from_str::<AuthenticationEnvelope>(&text).ok();
    let valid = envelope.is_some_and(|envelope| {
        envelope.protocol_version == PROTOCOL_VERSION
            && envelope.message_type == "authenticate"
            && envelope.token == expected_token
    });
    let response = AuthenticationResponse {
        protocol_version: PROTOCOL_VERSION,
        message_type: "authentication_response",
        authenticated: valid,
        error_code: (!valid).then_some("AUTHENTICATION_FAILED"),
    };
    if valid {
        session
            .diagnostics
            .auth_succeeded
            .fetch_add(1, Ordering::Relaxed);
    } else {
        session
            .diagnostics
            .auth_failed
            .fetch_add(1, Ordering::Relaxed);
    }
    let sent = socket
        .send(Message::Text(
            serde_json::to_string(&response).unwrap_or_default().into(),
        ))
        .is_ok();
    if !sent {
        session
            .diagnostics
            .auth_response_failed
            .fetch_add(1, Ordering::Relaxed);
    }
    valid && sent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::JobExecutor;
    use tungstenite::connect;

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

    #[test]
    fn authentication_accepts_the_configured_token_and_returns_a_response() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
        let port = listener.local_addr().expect("local address").port();
        let stop = Arc::new(AtomicBool::new(false));
        let session = Arc::new(WebSocketSessionState::default());
        let stop_for_thread = stop.clone();
        let session_for_thread = session.clone();
        let thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut socket = accept(stream).expect("websocket handshake");
            assert!(authenticate(
                &mut socket,
                "expected",
                &stop_for_thread,
                &session_for_thread
            ));
        });
        let (mut socket, _response) =
            connect(format!("ws://127.0.0.1:{port}")).expect("client handshake");
        socket
            .send(Message::Text(
                serde_json::to_string(&AuthenticationEnvelope {
                    protocol_version: PROTOCOL_VERSION,
                    message_type: "authenticate".to_owned(),
                    token: "expected".to_owned(),
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
        let thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            handle_websocket_connection(
                &service,
                &database_path_for_thread,
                stream,
                "expected",
                &session_for_thread,
                &stop_for_thread,
            );
        });
        let (mut socket, _response) =
            connect(format!("ws://127.0.0.1:{port}")).expect("client handshake");
        socket
            .send(Message::Text(
                serde_json::to_string(&AuthenticationEnvelope {
                    protocol_version: PROTOCOL_VERSION,
                    message_type: "authenticate".to_owned(),
                    token: "expected".to_owned(),
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
        thread.join().expect("server thread");
        let diagnostics = session.diagnostic_snapshot();
        assert_eq!(diagnostics.accepted, 0);
        assert_eq!(diagnostics.auth_received, 1);
        assert_eq!(diagnostics.auth_succeeded, 1);
        assert_eq!(diagnostics.close_after_auth, 1);
        let _ = std::fs::remove_file(database_path);
    }

    #[test]
    fn authentication_rejects_a_wrong_token() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
        let port = listener.local_addr().expect("local address").port();
        let stop = Arc::new(AtomicBool::new(false));
        let session = Arc::new(WebSocketSessionState::default());
        let stop_for_thread = stop.clone();
        let session_for_thread = session.clone();
        let thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut socket = accept(stream).expect("websocket handshake");
            assert!(!authenticate(
                &mut socket,
                "expected",
                &stop_for_thread,
                &session_for_thread
            ));
        });
        let (mut socket, _response) =
            connect(format!("ws://127.0.0.1:{port}")).expect("client handshake");
        socket
            .send(Message::Text(
                serde_json::to_string(&AuthenticationEnvelope {
                    protocol_version: PROTOCOL_VERSION,
                    message_type: "authenticate".to_owned(),
                    token: "wrong".to_owned(),
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
                "expected",
                &session_for_thread,
                &stop_for_thread,
            );
        });
        let (mut socket, _response) =
            connect(format!("ws://127.0.0.1:{port}")).expect("client handshake");
        // Delay the authentication frame so a non-blocking read would fail.
        std::thread::sleep(Duration::from_millis(50));
        socket
            .send(Message::Text(
                serde_json::to_string(&AuthenticationEnvelope {
                    protocol_version: PROTOCOL_VERSION,
                    message_type: "authenticate".to_owned(),
                    token: "expected".to_owned(),
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
}
