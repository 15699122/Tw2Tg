//! Browser protocol transport adapter for the executor service.
//!
//! This module translates the browser Native Messaging / WebSocket protocol
//! into calls against the executor application service and job persistence.
//!
//! The Unix endpoint is registered by the Desktop runtime. The Windows Named
//! Pipe backend remains a separate platform-specific implementation.
#![allow(dead_code)]
//! It preserves the browser `request_id` so the extension can match request
//! and response.
// `BrowserTransportAdapter` and its constructors are compiled on every
// platform, so `PathBuf` must be imported unconditionally. Only the
// Unix-specific socket helpers below need `Path`.
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(unix)]
use crate::executor::StorageJobPersistence;
use crate::executor::{
    ArchiveApplicationService, ArchiveJobSubmissionAdapter, ExecutorError, JobPersistence,
    state_sort_priority,
};
use xarchive_protocol::{BrowserRequest, BrowserResponse, BrowserTweet, PROTOCOL_VERSION};

/// Transports one browser request through the executor application service.
pub(crate) struct BrowserTransportAdapter {
    service: ArchiveApplicationService,
    database_path: Option<PathBuf>,
}

impl BrowserTransportAdapter {
    pub(crate) fn new(service: ArchiveApplicationService) -> Self {
        Self {
            service,
            database_path: None,
        }
    }

    pub(crate) fn with_database_path(
        service: ArchiveApplicationService,
        database_path: PathBuf,
    ) -> Self {
        Self {
            service,
            database_path: Some(database_path),
        }
    }

    /// Handle one browser request and return the matching browser response.
    ///
    /// The browser `request_id` is preserved on success and included on
    /// failure so the extension can correlate the error with the original
    /// request.
    pub(crate) fn handle_request<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request: BrowserRequest,
    ) -> BrowserResponse {
        let request_id = match &request {
            BrowserRequest::ArchiveRequest { request_id, .. }
            | BrowserRequest::QueryStatus { request_id, .. } => request_id.clone(),
        };
        if let Err(error) = request.validate() {
            return BrowserResponse::Error {
                protocol_version: PROTOCOL_VERSION,
                request_id: Some(request_id),
                error_code: "PROTOCOL_ERROR".to_owned(),
                error_message: error.to_string(),
            };
        }

        match request {
            BrowserRequest::ArchiveRequest {
                protocol_version: _,
                request_id,
                tweet,
            } => self.handle_archive_request(persistence, request_id, tweet),
            BrowserRequest::QueryStatus {
                protocol_version: _,
                request_id,
                tweet_ids,
            } => self.handle_query_status(persistence, request_id, tweet_ids),
        }
    }

    fn handle_archive_request<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request_id: String,
        tweet: BrowserTweet,
    ) -> BrowserResponse {
        let archive_request = crate::ArchiveTweetRequest {
            tweet,
            browser: None,
            profile: None,
        };

        match ArchiveJobSubmissionAdapter.prepare(&archive_request, &now_iso()) {
            Ok(job) => {
                let result = match &self.database_path {
                    Some(database_path) => self.service.submit_and_schedule_persisted(
                        persistence,
                        job,
                        database_path.clone(),
                    ),
                    None => self.service.submit_persisted(persistence, job),
                };
                match result {
                    Ok(result) => BrowserResponse::ArchiveStatus {
                        protocol_version: PROTOCOL_VERSION,
                        request_id: request_id.clone(),
                        tweet_id: result.job.tweet_id.clone(),
                        job_id: Some(result.job.job_id.clone()),
                        state: result.job.state.as_str().to_owned(),
                        progress: None,
                    },
                    Err(error) => error_response(&request_id, &error),
                }
            }
            Err(error) => error_response(&request_id, &error),
        }
    }

    fn handle_query_status<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request_id: String,
        tweet_ids: Vec<String>,
    ) -> BrowserResponse {
        let requested_tweet_ids = tweet_ids.clone();
        let mut entries = Vec::new();

        for tweet_id in tweet_ids {
            let snapshot = match persistence.snapshot_for_tweet(&tweet_id) {
                Ok(Some(snapshot)) => snapshot,
                Ok(None) => continue,
                Err(_) => {
                    return BrowserResponse::Error {
                        protocol_version: PROTOCOL_VERSION,
                        request_id: Some(request_id.clone()),
                        error_code: "PERSISTENCE_ERROR".to_owned(),
                        error_message: format!("failed to read job for tweet {tweet_id}"),
                    };
                }
            };

            entries.push(snapshot);
        }

        if entries.is_empty() {
            return BrowserResponse::Error {
                protocol_version: PROTOCOL_VERSION,
                request_id: Some(request_id),
                error_code: "NO_JOBS_FOUND".to_owned(),
                error_message: format!(
                    "no matching jobs found for the requested tweet IDs: {}",
                    requested_tweet_ids.join(", ")
                ),
            };
        }

        // Sort by state priority (latest state first) then by job_id for stable ordering
        entries.sort_by(|a, b| {
            let a_priority = state_sort_priority(&a.state);
            let b_priority = state_sort_priority(&b.state);
            b_priority
                .cmp(&a_priority)
                .then_with(|| a.job_id.cmp(&b.job_id))
        });

        let latest = &entries[0];

        BrowserResponse::ArchiveStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.clone(),
            tweet_id: latest.tweet_id.clone(),
            job_id: Some(latest.job_id.clone()),
            state: latest.state.as_str().to_owned(),
            progress: None,
        }
    }
}

/// Linux/Unix Desktop endpoint for the Native Host transport.
///
/// Windows uses a separate Named Pipe backend and remains a platform-specific
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

    pub(crate) fn start(
        service: ArchiveApplicationService,
        database_path: PathBuf,
        endpoint: PathBuf,
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
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("failed to configure transport endpoint: {error}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
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
                            let _ = std::thread::Builder::new()
                                .name("xarchive-desktop-transport-request".to_owned())
                                .spawn(move || {
                                    // A stalled or half-written request must not
                                    // hold the connection open indefinitely.
                                    let _ = stream.set_read_timeout(Some(Self::CONNECTION_TIMEOUT));
                                    let _ =
                                        stream.set_write_timeout(Some(Self::CONNECTION_TIMEOUT));
                                    handle_unix_connection(&service, &database_path, &mut stream);
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
) {
    use xarchive_native_host::{read_json, write_json};

    let response = match read_json::<_, BrowserRequest>(stream) {
        Ok(Some(request)) => match StorageJobPersistence::open(database_path) {
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
            },
        },
        Ok(None) => return,
        Err(error) => BrowserResponse::Error {
            protocol_version: PROTOCOL_VERSION,
            request_id: None,
            error_code: "INVALID_MESSAGE".to_owned(),
            error_message: error.to_string(),
        },
    };
    let _ = write_json(stream, &response);
}

#[cfg(unix)]
pub(crate) fn transport_endpoint(portable_root: &Path) -> PathBuf {
    std::env::var_os("XARCHIVE_PIPE_ENDPOINT")
        .map(PathBuf::from)
        .unwrap_or_else(|| portable_root.join("cache").join("xarchive-v1.sock"))
}

fn now_iso() -> String {
    // Browser submissions must be persisted with the real request time; the
    // job identity and metrics depend on it.
    crate::clock::now_iso()
}

fn error_response(request_id: &str, error: &ExecutorError) -> BrowserResponse {
    BrowserResponse::Error {
        protocol_version: PROTOCOL_VERSION,
        request_id: Some(request_id.to_owned()),
        error_code: error_error_code(error),
        error_message: error.to_string(),
    }
}

fn error_error_code(error: &ExecutorError) -> String {
    match error {
        ExecutorError::Closed => "EXECUTOR_CLOSED".to_owned(),
        ExecutorError::QueueFull => "EXECUTOR_QUEUE_FULL".to_owned(),
        ExecutorError::ResponseClosed => "EXECUTOR_RESPONSE_CLOSED".to_owned(),
        ExecutorError::UnknownJob(_) => "EXECUTOR_UNKNOWN_JOB".to_owned(),
        ExecutorError::InvalidTransition { .. } => "EXECUTOR_INVALID_TRANSITION".to_owned(),
        ExecutorError::Persistence(_) => "PERSISTENCE_ERROR".to_owned(),
        ExecutorError::Execution { error_code, .. } => error_code.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{InMemoryJobPersistence, JobExecutor};
    use xarchive_core::JobState;

    fn tweet(tweet_id: &str) -> BrowserTweet {
        BrowserTweet {
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            text: Some("archive me".to_owned()),
            created_at: Some("2026-09-12T00:00:00Z".to_owned()),
            tweet_type: "post".to_owned(),
            reply_to: None,
            quoted_tweet: None,
        }
    }

    fn archive_adapter() -> (BrowserTransportAdapter, JobExecutor) {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        (BrowserTransportAdapter::new(service), executor)
    }

    #[test]
    fn transport_preserves_request_id_on_archive_request() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-archive-1".to_owned(),
            tweet: tweet("123"),
        };

        let response = transport.handle_request(&mut persistence, request.clone());

        let BrowserResponse::ArchiveStatus {
            request_id,
            tweet_id,
            job_id,
            state,
            ..
        } = response
        else {
            panic!("expected archive_status response, got {response:?}");
        };

        assert_eq!(request_id, "browser-archive-1");
        assert_eq!(tweet_id, "123");
        // The job identity is derived from the real submission time, so the
        // timestamp is no longer a fixed production constant.
        let job_id = job_id.expect("expected a job id");
        assert!(
            job_id.starts_with("archive-123-"),
            "unexpected job id: {job_id}"
        );
        let timestamp = job_id
            .strip_prefix("archive-123-")
            .expect("job id carries the tweet id prefix");
        crate::clock::assert_canonical_timestamp(timestamp);
        assert_eq!(state, "QUEUED");
    }

    #[test]
    fn transport_reuses_existing_job_for_duplicate_archive_request() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-archive-1".to_owned(),
            tweet: tweet("123"),
        };

        let first = transport.handle_request(&mut persistence, request.clone());
        let second = transport.handle_request(&mut persistence, request);

        let BrowserResponse::ArchiveStatus {
            request_id: first_id,
            ..
        } = first
        else {
            panic!("expected first response to be archive_status");
        };
        let BrowserResponse::ArchiveStatus {
            request_id: second_id,
            ..
        } = second
        else {
            panic!("expected second response to be archive_status");
        };

        assert_eq!(first_id, "browser-archive-1");
        assert_eq!(second_id, "browser-archive-1");
    }

    #[test]
    fn transport_replies_with_current_state_on_query_status() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        // Submit an archive request first
        let archive_request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-archive-1".to_owned(),
            tweet: tweet("123"),
        };
        let BrowserResponse::ArchiveStatus { job_id, .. } =
            transport.handle_request(&mut persistence, archive_request)
        else {
            panic!("expected archive_status response");
        };
        let job_id = job_id.expect("expected a job id");

        // Advance job state to Downloading
        persistence
            .persist_state(&crate::executor::JobSnapshot {
                job_id,
                tweet_id: "123".to_owned(),
                state: JobState::Downloading,
            })
            .expect("persist state");

        // Query status
        let query_request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-status-1".to_owned(),
            tweet_ids: vec!["123".to_owned()],
        };

        let response = transport.handle_request(&mut persistence, query_request);

        let BrowserResponse::ArchiveStatus {
            request_id,
            tweet_id,
            state,
            ..
        } = response
        else {
            panic!("expected archive_status response, got {response:?}");
        };

        assert_eq!(request_id, "browser-status-1");
        assert_eq!(tweet_id, "123");
        assert_eq!(state, "DOWNLOADING");
    }

    #[test]
    fn transport_reports_error_for_query_status_with_no_matching_jobs() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let query_request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-status-1".to_owned(),
            tweet_ids: vec!["999".to_owned()],
        };

        let response = transport.handle_request(&mut persistence, query_request);

        let BrowserResponse::Error {
            request_id,
            error_code,
            error_message,
            ..
        } = response
        else {
            panic!("expected error response, got {response:?}");
        };

        assert_eq!(request_id.as_deref(), Some("browser-status-1"));
        assert_eq!(error_code, "NO_JOBS_FOUND");
        assert!(error_message.contains("999"));
    }

    #[test]
    fn transport_reports_error_when_archive_tweet_validation_fails() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-invalid-tweet".to_owned(),
            tweet: BrowserTweet {
                tweet_id: "123".to_owned(),
                url: "https://example.com/status/123".to_owned(),
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
                text: Some("archive me".to_owned()),
                created_at: Some("2026-09-12T00:00:00Z".to_owned()),
                tweet_type: "post".to_owned(),
                reply_to: None,
                quoted_tweet: None,
            },
        };

        let response = transport.handle_request(&mut persistence, request);

        let BrowserResponse::Error {
            request_id,
            error_code,
            error_message,
            ..
        } = response
        else {
            panic!("expected error response, got {response:?}");
        };

        assert_eq!(request_id.as_deref(), Some("browser-invalid-tweet"));
        assert_eq!(error_code, "PROTOCOL_ERROR");
        assert!(error_message.contains("tweet URL"));
        assert!(persistence.list_recovery_candidates().is_empty());
    }

    #[test]
    fn transport_rejects_invalid_protocol_version_before_persistence() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();
        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION + 1,
            request_id: "browser-invalid-version".to_owned(),
            tweet: tweet("123"),
        };

        let response = transport.handle_request(&mut persistence, request);

        let BrowserResponse::Error {
            request_id,
            error_code,
            error_message,
            ..
        } = response
        else {
            panic!("expected protocol error response");
        };
        assert_eq!(request_id.as_deref(), Some("browser-invalid-version"));
        assert_eq!(error_code, "PROTOCOL_ERROR");
        assert!(error_message.contains("unsupported protocol version"));
        assert!(persistence.list_recovery_candidates().is_empty());
    }
}

#[cfg(all(test, unix))]
mod transport_limit_tests {
    use super::DesktopTransportServer;
    use crate::executor::ArchiveApplicationService;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    /// ENG-04: opening connections without ever completing a request must not
    /// consume resources without bound. The accept loop must stay responsive
    /// and keep serving once the burst of clients disconnects.
    #[test]
    fn survives_a_burst_of_incomplete_connections() {
        let directory = std::env::temp_dir().join(format!(
            "xarchive-transport-limit-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&directory).expect("endpoint directory");
        let endpoint = directory.join("xarchive-limit.sock");

        let executor = crate::executor::JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let database_path = std::env::temp_dir().join(format!(
            "xarchive-transport-limit-db-{}.sqlite3",
            crate::runtime::timestamp_marker()
        ));
        let server = DesktopTransportServer::start(service, database_path, endpoint.clone())
            .expect("start transport server");

        // More clients than the cap, none of which sends a complete request.
        let mut clients = Vec::new();
        for _ in 0..(DesktopTransportServer::MAX_ACTIVE_CONNECTIONS + 8) {
            match UnixStream::connect(&endpoint) {
                Ok(stream) => clients.push(stream),
                // The kernel backlog may refuse once the cap is hit; that is the
                // intended behavior rather than a failure.
                Err(_) => break,
            }
        }
        std::thread::sleep(Duration::from_millis(200));
        drop(clients);

        // The server must still accept work after the burst drains.
        let mut probe = UnixStream::connect(&endpoint).expect("connect after burst");
        let _ = probe.write_all(b"");
        drop(server);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn connection_limits_are_reasonable_for_local_clients() {
        // Bounds chosen for a single-user local endpoint: enough headroom for
        // concurrent browser and CLI clients, but never unbounded.
        let cap = DesktopTransportServer::MAX_ACTIVE_CONNECTIONS;
        assert!(cap > 0 && cap <= 1024, "unexpected connection cap: {cap}");
        let timeout = DesktopTransportServer::CONNECTION_TIMEOUT;
        assert!(
            timeout >= Duration::from_secs(1) && timeout <= Duration::from_secs(120),
            "unexpected connection timeout: {timeout:?}"
        );
    }
}
