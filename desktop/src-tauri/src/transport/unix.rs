//! Linux/Unix Desktop transport endpoint.
//!
//! Split out of `transport.rs` without changing behaviour, the module path or the
//! protocol contract. `crate::transport::DesktopTransportServer` and
//! `crate::transport::transport_endpoint` keep resolving through the parent's
//! re-export.
//!
//! Invariants that must not move:
//!
//! * the endpoint is current-user only: the socket is created `0600` because the
//!   bootstrap request issues credentials;
//! * a stale endpoint file is replaced, and `Drop` stops the accept loop, joins
//!   the thread and removes the socket, so a restart never binds a dead path;
//! * the accept loop stops admitting work at `MAX_ACTIVE_CONNECTIONS`, and every
//!   connection runs under `CONNECTION_TIMEOUT`, so one silent client cannot hold
//!   a thread forever;
//! * a bootstrap frame is answered before any Job work, and an unusable pairing
//!   coordinator fails closed with `DESKTOP_NOT_READY` instead of falling back;
//! * each request gets its own persistence context and is handled briefly, so the
//!   runtime lock is never held across transport I/O.
//!
//! The Windows Named Pipe backend is a separate platform implementation
//! (`windows_transport.rs`, Windows Owner) and is not changed by this split.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

use crate::executor::ArchiveApplicationService;
#[cfg(unix)]
use crate::executor::StorageJobPersistence;

use super::BrowserTransportAdapter;

/// Linux/Unix Desktop endpoint for the Native Host transport.
///
/// The Windows backend (roadmap E5, Windows Owner) serves the same
/// BrowserRequest/BrowserResponse contract over a Named Pipe listening on
/// `xarchive_protocol::WINDOWS_PIPE_ENDPOINT`. It remains a platform-specific
/// validation item. The Unix implementation exists to make the production
/// request boundary executable and testable without pretending to validate
/// Windows ACL or Named Pipe behavior.
#[cfg(unix)]
pub(crate) struct DesktopTransportServer {
    stop: Arc<AtomicBool>,
    endpoint: PathBuf,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(unix)]
impl DesktopTransportServer {
    /// Maximum number of concurrent request connections.
    ///
    /// A client that opens a connection and never sends a complete request
    /// would otherwise hold a thread forever, so the accept loop stops
    /// admitting work once the cap is reached.
    pub(crate) const MAX_ACTIVE_CONNECTIONS: usize = 64;
    /// Read/write deadline applied to every accepted connection.
    pub(crate) const CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

    #[cfg(test)]
    pub(crate) fn start(
        service: ArchiveApplicationService,
        database_path: PathBuf,
        endpoint: PathBuf,
    ) -> Result<Self, String> {
        Self::start_with_pairing(
            service,
            database_path,
            endpoint,
            None,
            xarchive_storage::BatchOutputSettings::default(),
            false,
            crate::executor::ToolCompatibility::default(),
        )
    }
    pub(crate) fn start_with_pairing(
        service: ArchiveApplicationService,
        database_path: PathBuf,
        endpoint: PathBuf,
        pairing: Option<Arc<crate::browser_pairing::PairingCoordinator>>,
        output_settings: xarchive_storage::BatchOutputSettings,
        use_aria2: bool,
        tool_compatibility: crate::executor::ToolCompatibility,
    ) -> Result<Self, String> {
        use std::os::unix::net::UnixListener;
        use std::sync::atomic::AtomicUsize;
        use std::time::Duration;

        if endpoint.exists() {
            std::fs::remove_file(&endpoint)
                .map_err(|error| format!("failed to replace transport endpoint: {error}"))?;
        }
        if let Some(parent) = endpoint.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!("failed to create transport endpoint directory: {error}")
            })?;
        }
        let listener = UnixListener::bind(&endpoint)
            .map_err(|error| format!("failed to bind transport endpoint: {error}"))?;
        // Bootstrap issues credentials: the Unix socket is current-user only.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "failed to restrict Unix transport permissions".to_owned())?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("failed to configure transport endpoint: {error}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let output_settings = Arc::new(output_settings);
        let tool_compatibility = Arc::new(tool_compatibility);
        let active = Arc::new(AtomicUsize::new(0));
        let active_for_thread = active.clone();
        let thread = std::thread::Builder::new()
            .name("xarchive-desktop-transport".to_owned())
            .spawn(move || {
                while !stop_for_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            // Refuse new work instead of spawning without bound.
                            if active_for_thread.load(Ordering::Relaxed)
                                >= Self::MAX_ACTIVE_CONNECTIONS
                            {
                                continue;
                            }
                            active_for_thread.fetch_add(1, Ordering::Relaxed);
                            let service = service.clone();
                            let database_path = database_path.clone();
                            let active_for_request = active_for_thread.clone();
                            let pairing = pairing.clone();
                            let output_settings = output_settings.clone();
                            let tool_compatibility = tool_compatibility.clone();
                            let use_aria2 = use_aria2;
                            let _ = std::thread::Builder::new()
                                .name("xarchive-desktop-transport-request".to_owned())
                                .spawn(move || {
                                    // A stalled or half-written request must not
                                    // hold the connection open indefinitely.
                                    let _ = stream.set_read_timeout(Some(Self::CONNECTION_TIMEOUT));
                                    let _ =
                                        stream.set_write_timeout(Some(Self::CONNECTION_TIMEOUT));
                                    handle_unix_connection(
                                        &service,
                                        &database_path,
                                        &mut stream,
                                        pairing.as_deref(),
                                        &output_settings,
                                        use_aria2,
                                        &tool_compatibility,
                                    );
                                    active_for_request.fetch_sub(1, Ordering::Relaxed);
                                });
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(25));
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("failed to start transport endpoint: {error}"))?;
        Ok(Self {
            stop,
            endpoint,
            thread: Some(thread),
        })
    }
}

#[cfg(unix)]
impl Drop for DesktopTransportServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(&self.endpoint);
    }
}

#[cfg(unix)]
fn handle_unix_connection(
    service: &ArchiveApplicationService,
    database_path: &Path,
    stream: &mut std::os::unix::net::UnixStream,
    pairing: Option<&crate::browser_pairing::PairingCoordinator>,
    output_settings: &xarchive_storage::BatchOutputSettings,
    use_aria2: bool,
    tool_compatibility: &crate::executor::ToolCompatibility,
) {
    use xarchive_native_host::{read_json, write_json};

    let payload = read_json::<_, serde_json::Value>(stream);
    if let Ok(Some(value)) = &payload
        && value.get("message_type").and_then(|field| field.as_str()) == Some("bootstrap")
    {
        let response =
            match serde_json::from_value::<xarchive_protocol::BrowserPairingRequest>(value.clone())
            {
                Ok(request) => match pairing {
                    Some(pairing) => pairing.bootstrap(request),
                    None => xarchive_protocol::BrowserPairingResponse::Error {
                        protocol_version: 1,
                        request_id: match request {
                            xarchive_protocol::BrowserPairingRequest::Bootstrap {
                                request_id,
                                ..
                            } => Some(request_id),
                        },
                        error_code: "DESKTOP_NOT_READY".into(),
                        error_message: "Desktop listener unavailable".into(),
                        retryable: true,
                    },
                },
                Err(_) => xarchive_protocol::BrowserPairingResponse::Error {
                    protocol_version: 1,
                    request_id: None,
                    error_code: "INVALID_REQUEST".into(),
                    error_message: "invalid bootstrap request".into(),
                    retryable: false,
                },
            };
        let _ = write_json(stream, &response);
        return;
    }
    let request = payload.and_then(|value| {
        value
            .map(serde_json::from_value::<BrowserRequest>)
            .transpose()
            .map_err(|error| {
                xarchive_native_host::NativeMessagingError::InvalidJson(error.to_string())
            })
    });
    let response = match request {
        Ok(Some(request)) => match StorageJobPersistence::open(database_path) {
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
        Ok(None) => return,
        Err(error) => BrowserResponse::Error {
            protocol_version: PROTOCOL_VERSION,
            request_id: None,
            error_code: "INVALID_MESSAGE".to_owned(),
            error_message: error.to_string(),
            retryable: false,
        },
    };
    let _ = write_json(stream, &response);
}

#[cfg(unix)]
pub(crate) fn transport_endpoint(portable_root: &Path) -> PathBuf {
    std::env::var_os(xarchive_protocol::PIPE_ENDPOINT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| portable_root.join("cache").join("xarchive-v1.sock"))
}
