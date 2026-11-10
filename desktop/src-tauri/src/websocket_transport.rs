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
        use_aria2: bool,
        tool_compatibility: crate::executor::ToolCompatibility,
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
                                    let (finished_worker, _) = workers.swap_remove(index);
                                    let _ = finished_worker.join();
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
                            let tool_compatibility = tool_compatibility.clone();
                            let request_use_aria2 = use_aria2;
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
                                        request_use_aria2,
                                        &tool_compatibility,
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

/// Serve one authenticated WebSocket connection for its whole lifetime.
///
/// The argument count is above clippy's threshold because each value is a
/// distinct per-connection input; grouping them would change this shared
/// transport signature and needs its own reviewed batch, so the lint is scoped
/// here rather than refactored as a drive-by.
#[allow(clippy::too_many_arguments)]
fn handle_websocket_connection(
    service: &ArchiveApplicationService,
    database_path: &Path,
    stream: TcpStream,
    pairing: &PairingCoordinator,
    session: &WebSocketSessionState,
    stop: &AtomicBool,
    output_settings: &xarchive_storage::BatchOutputSettings,
    use_aria2: bool,
    tool_compatibility: &crate::executor::ToolCompatibility,
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
                        .with_use_aria2(use_aria2)
                        .with_tool_compatibility(tool_compatibility.clone())
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
                let _ = socket.close(frame);
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

    mod authentication;
    mod deadline_and_lifecycle;
    mod preauth_limits;
    mod state_and_defaults;

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
}
