//! Authenticated loopback WebSocket transport for the browser Extension.
//!
//! WebSocket framing and handshake are provided by `tungstenite`; this module
//! owns only the XArchive transport envelope and delegates business messages
//! to the existing `BrowserTransportAdapter`.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::browser_pairing::EXTENSION_ORIGIN;
use serde::Serialize;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket, accept_hdr_with_config};
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
// tungstenite's Callback requires its unboxed HTTP ErrorResponse.
#[allow(clippy::result_large_err)]
fn validate_handshake(request: &Request, response: Response) -> Result<Response, ErrorResponse> {
    let origins: Vec<_> = request.headers().get_all("origin").iter().collect();
    if request.uri().path() != "/"
        || request.uri().query().is_some()
        || origins.len() != 1
        || origins[0].to_str().ok() != Some(EXTENSION_ORIGIN)
    {
        return Err(tungstenite::http::Response::builder()
            .status(403)
            .body(Some("forbidden".to_owned()))
            .expect("static rejection"));
    }
    Ok(response)
}
fn socket_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .read_buffer_size(4096)
        .write_buffer_size(4096)
        .max_write_buffer_size(2 * MAX_MESSAGE_BYTES)
        .max_message_size(Some(MAX_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_MESSAGE_BYTES))
}
use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

use crate::browser_pairing::PairingCoordinator;
use crate::executor::{ArchiveApplicationService, StorageJobPersistence};
use crate::transport::BrowserTransportAdapter;

const MAX_CONNECTIONS: usize = 32;
const AUTH_TIMEOUT: Duration = Duration::from_secs(3);
struct DeadlineStream<'a> {
    stream: TcpStream,
    deadline: Option<Instant>,
    remaining_bytes: Option<usize>,
    stop: Option<&'a AtomicBool>,
}
impl Read for DeadlineStream<'_> {
    // Only Winsock needs retry polling; Unix performs one blocking read.
    #[cfg_attr(not(windows), allow(clippy::never_loop))]
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.stop.is_some_and(|stop| stop.load(Ordering::Relaxed)) {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "listener stopping",
                ));
            }
            if let Some(deadline) = self.deadline {
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .filter(|remaining| !remaining.is_zero())
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::TimedOut, "authentication deadline")
                    })?;
                #[cfg(windows)]
                let remaining = remaining.min(Duration::from_millis(100));
                self.stream.set_read_timeout(Some(remaining))?;
            }
            let limit = self
                .remaining_bytes
                .unwrap_or(buffer.len())
                .min(buffer.len());
            if limit == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "pre-auth read limit",
                ));
            }
            let read = match self.stream.read(&mut buffer[..limit]) {
                #[cfg(windows)]
                Err(error)
                    if self.deadline.is_some()
                        && matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                {
                    continue;
                }
                result => result?,
            };
            if let Some(remaining) = self.remaining_bytes.as_mut() {
                *remaining -= read;
            }
            return Ok(read);
        }
    }
}
impl Write for DeadlineStream<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.stream.write(buffer)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

use xarchive_protocol::BrowserPairingAuthentication as AuthenticationEnvelope;

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
    /// Seconds since the last browser request, `None` when none arrived.
    ///
    /// Evidence for an idle-duration question; deliberately not folded into
    /// the connection state.
    pub last_request_age_seconds: Option<u64>,
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
            last_request_age_seconds: self.last_request_age_seconds(),
        }
    }

    /// Whether a browser socket is open right now.
    ///
    /// The answer comes from the live socket count only. An earlier version also
    /// reported "connected" for 30 seconds after the last request, which claims
    /// a connection that no longer exists — the exact kind of disagreement this
    /// status exists to prevent. `last_request_age_seconds` is reported instead,
    /// so an idle-time question can be answered from evidence rather than from a
    /// status that talks in the connection's favour.
    pub(crate) fn browser_connection(&self) -> &'static str {
        if self.active.load(Ordering::Relaxed) > 0 {
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

    /// Seconds since the last browser request, or `None` when none arrived.
    ///
    /// Diagnostics only: it separates "never used the connection" from "used
    /// it and then went silent", which is what an idle-duration investigation
    /// needs and what a status flag must not answer for it.
    pub(crate) fn last_request_age_seconds(&self) -> Option<u64> {
        self.last_request
            .lock()
            .ok()
            .and_then(|last| *last)
            .map(|last| last.elapsed().as_secs())
    }
}

pub(crate) struct DesktopWebSocketServer {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    port: u16,
    pairing: Arc<PairingCoordinator>,
    pub(crate) session: Arc<WebSocketSessionState>,
}

impl DesktopWebSocketServer {
    pub(crate) fn start(
        service: ArchiveApplicationService,
        database_path: std::path::PathBuf,
        output_settings: xarchive_storage::BatchOutputSettings,
    ) -> Result<Self, String> {
        let port = configured_port()?;
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
            format!("failed to bind WebSocket listener on 127.0.0.1:{port}: {error}")
        })?;
        let port = listener
            .local_addr()
            .map_err(|_| "listener address unavailable")?
            .port();
        let pairing = Arc::new(PairingCoordinator::new(port)?);
        let output_settings = Arc::new(output_settings);
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("failed to configure WebSocket listener: {error}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let session = Arc::new(WebSocketSessionState::default());
        let session_for_thread = session.clone();
        let pairing_for_thread = pairing.clone();
        let thread = std::thread::Builder::new()
            .name("xarchive-desktop-websocket".to_owned())
            .spawn(move || {
                let mut workers: Vec<(JoinHandle<()>, TcpStream)> = Vec::new();
                let mut admission_window = Instant::now();
                let mut admissions = 0usize;
                while !stop_for_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let mut index = 0;
                            while index < workers.len() {
                                if workers[index].0.is_finished() {
                                    let (worker, _) = workers.swap_remove(index);
                                    let _ = worker.join();
                                } else {
                                    index += 1;
                                }
                            }
                            if admission_window.elapsed() >= Duration::from_secs(1) {
                                admission_window = Instant::now();
                                admissions = 0;
                            }
                            if workers.len() >= MAX_CONNECTIONS || admissions >= 64 {
                                continue;
                            }
                            admissions += 1;
                            let Ok(tracked) = stream.try_clone() else {
                                continue;
                            };
                            session_for_thread
                                .diagnostics
                                .accepted
                                .fetch_add(1, Ordering::Relaxed);
                            let service = service.clone();
                            let database_path = database_path.clone();
                            let session = session_for_thread.clone();
                            let pairing = pairing_for_thread.clone();
                            let stop = stop_for_thread.clone();
                            let output_settings = output_settings.clone();
                            if let Ok(worker) = std::thread::Builder::new()
                                .name("xarchive-desktop-websocket-request".to_owned())
                                .spawn(move || {
                                    handle_websocket_connection(
                                        &service,
                                        &database_path,
                                        stream,
                                        &pairing,
                                        &session,
                                        &stop,
                                        &output_settings,
                                    );
                                })
                            {
                                workers.push((worker, tracked));
                            }
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(25));
                        }
                        Err(_) => break,
                    }
                }
                for (_, stream) in &workers {
                    let _ = stream.shutdown(Shutdown::Both);
                }
                for (worker, _) in workers {
                    let _ = worker.join();
                }
            })
            .map_err(|error| format!("failed to start WebSocket listener: {error}"))?;
        Ok(Self {
            stop,
            thread: Some(thread),
            port,
            pairing,
            session,
        })
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn runtime_instance_id(&self) -> &str {
        self.pairing.runtime_instance_id()
    }

    pub(crate) fn pairing(&self) -> Arc<PairingCoordinator> {
        self.pairing.clone()
    }
}

impl Drop for DesktopWebSocketServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.pairing.stop();
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
        Err(_) => Ok(0),
    }
}

fn handle_websocket_connection(
    service: &ArchiveApplicationService,
    database_path: &Path,
    stream: TcpStream,
    pairing: &PairingCoordinator,
    session: &WebSocketSessionState,
    stop: &AtomicBool,
    output_settings: &xarchive_storage::BatchOutputSettings,
) {
    // The accept loop uses a non-blocking listener only so it can poll the stop
    // flag. The accepted stream must be switched back to blocking mode: POSIX
    // `accept` does not inherit `O_NONBLOCK`, but Winsock does, which would make
    // the handshake and the authentication read fail immediately with
    // `WouldBlock` instead of waiting for the peer's frames.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let stream = DeadlineStream {
        stream,
        deadline: Some(Instant::now() + AUTH_TIMEOUT),
        remaining_bytes: Some(16 * 1024),
        stop: Some(stop),
    };
    let Ok(mut socket) = accept_hdr_with_config(stream, validate_handshake, Some(socket_config()))
    else {
        session
            .diagnostics
            .handshake_failed
            .fetch_add(1, Ordering::Relaxed);
        return;
    };
    socket.get_mut().remaining_bytes = Some(4096);
    if !authenticate(&mut socket, pairing, stop, session) {
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
    socket.get_mut().deadline = None;
    socket.get_mut().remaining_bytes = None;
    #[cfg(not(windows))]
    let _ = socket.get_ref().stream.set_read_timeout(None);
    // Winsock shutdown from a cloned handle does not reliably interrupt an
    // in-flight recv. Poll stop without expiring the authenticated session.
    #[cfg(windows)]
    let _ = socket
        .get_ref()
        .stream
        .set_read_timeout(Some(Duration::from_millis(100)));
    while !stop.load(Ordering::Relaxed) {
        let message = match socket.read() {
            Ok(message) => message,
            #[cfg(windows)]
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
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
                        .with_output_settings(output_settings.clone())
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
            // RFC 6455 §7.1.2: a server that receives a close frame must reply
            // with a close frame and then close. tungstenite queues the reply
            // on `read`; the next `read`/`flush` drives it out. Dropping the
            // socket here would abort the handshake before the peer sees the
            // reply, so the client never observes a graceful close.
            Message::Close(frame) => {
                let _ = socket.close(frame.map(|frame| frame.into()));
                let _ = socket.flush();
                break;
            }
            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
    session.active.fetch_sub(1, Ordering::Relaxed);
    session
        .diagnostics
        .close_after_auth
        .fetch_add(1, Ordering::Relaxed);
}

fn authenticate<S: Read + Write>(
    socket: &mut WebSocket<S>,
    pairing: &PairingCoordinator,
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
    if text.len() > 4096 {
        return false;
    }
    let envelope = serde_json::from_str::<AuthenticationEnvelope>(&text).ok();
    let valid = envelope.is_some_and(|envelope| {
        envelope.validate().is_ok()
            && pairing.consume(&envelope.ticket, crate::browser_pairing::EXTENSION_ORIGIN)
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
    use tungstenite::client::IntoClientRequest;
    use tungstenite::{accept, connect};
    fn client(
        port: u16,
    ) -> (
        WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
        tungstenite::handshake::client::Response,
    ) {
        let mut request = format!("ws://127.0.0.1:{port}/")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("Origin", EXTENSION_ORIGIN.parse().unwrap());
        connect(request).expect("client handshake")
    }
    fn credentials() -> (Arc<PairingCoordinator>, String) {
        let pairing = Arc::new(PairingCoordinator::new(43127).unwrap());
        let response = pairing.bootstrap(xarchive_protocol::BrowserPairingRequest::Bootstrap {
            protocol_version: 1,
            request_id: "r1".into(),
        });
        let xarchive_protocol::BrowserPairingResponse::Bootstrap { ticket, .. } = response else {
            panic!("ticket");
        };
        (pairing, ticket)
    }

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
        let endpoint =
            std::env::temp_dir().join(format!("pairing-ipc-{}.sock", std::process::id()));
        let server = DesktopWebSocketServer::start(
            service.clone(),
            database_path.clone(),
            xarchive_storage::BatchOutputSettings::default(),
        )
        .unwrap();
        let ipc = DesktopTransportServer::start_with_pairing(
            service,
            database_path,
            endpoint.clone(),
            Some(server.pairing()),
            xarchive_storage::BatchOutputSettings::default(),
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
}
