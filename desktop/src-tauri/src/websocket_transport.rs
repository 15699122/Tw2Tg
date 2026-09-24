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

#[derive(Default)]
pub(crate) struct WebSocketSessionState {
    active: AtomicUsize,
    connected_once: AtomicBool,
    authenticated_once: AtomicBool,
    last_request: std::sync::Mutex<Option<Instant>>,
}

impl WebSocketSessionState {
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
    stream.set_read_timeout(Some(AUTH_TIMEOUT)).ok();
    let Ok(mut socket) = accept(stream) else {
        return;
    };
    if !authenticate(&mut socket, expected_token, stop) {
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
}

fn authenticate(
    socket: &mut WebSocket<TcpStream>,
    expected_token: &str,
    stop: &AtomicBool,
) -> bool {
    if stop.load(Ordering::Relaxed) {
        return false;
    }
    let Ok(Message::Text(text)) = socket.read() else {
        return false;
    };
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
    let sent = socket
        .send(Message::Text(
            serde_json::to_string(&response).unwrap_or_default().into(),
        ))
        .is_ok();
    valid && sent
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn authentication_rejects_a_wrong_token() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
        let port = listener.local_addr().expect("local address").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut socket = accept(stream).expect("websocket handshake");
            assert!(!authenticate(&mut socket, "expected", &stop_for_thread));
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
}
