use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{SystemTime, UNIX_EPOCH};

use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::Database;

use crate::config::AppConfig;
use crate::executor::{CancellationToken, ExecutorRuntime};
use crate::logging::LogFile;
use crate::portable::{PortablePaths, portable_root};
#[cfg(unix)]
use crate::transport::DesktopTransportServer;
use crate::websocket_transport::DesktopWebSocketServer;
#[cfg(windows)]
use crate::windows_transport::DesktopTransportServer;

pub struct RuntimeState {
    pub(crate) portable_root: PathBuf,
    pub(crate) cache_root: PathBuf,
    pub(crate) download_root: PathBuf,
    pub(crate) logs_root: PathBuf,
    pub(crate) download_setup_required: bool,
    pub(crate) config: AppConfig,
    pub(crate) log_file: Option<LogFile>,
    pub(crate) database: Option<Database>,
    pub(crate) database_ready: bool,
    pub(crate) database_error: Option<String>,
    pub(crate) executor: ExecutorRuntime,
    #[cfg(unix)]
    pub(crate) transport_server: Option<DesktopTransportServer>,
    #[cfg(windows)]
    pub(crate) transport_server: Option<DesktopTransportServer>,
    #[cfg(windows)]
    pub(crate) transport_error: Option<String>,
    pub(crate) websocket_server: Option<DesktopWebSocketServer>,
    pub(crate) websocket_error: Option<String>,
    pub(crate) sidecar: Option<SidecarSupervisor>,
    pub(crate) sidecar_error: Option<String>,
    pub(crate) batch_cancellations: Arc<StdMutex<HashMap<String, CancellationToken>>>,
}

pub(crate) fn timestamp_marker() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

impl RuntimeState {
    /// Write one diagnostic line through the configured redaction boundary.
    ///
    /// Every module logs through this helper so the level filter, the secret
    /// redaction, and the rotation/size caps apply uniformly. A pre-release
    /// therefore reports detail across modules, while a stable release filters
    /// `Debug` lines and keeps progress, warnings, and errors.
    pub(crate) fn record(&self, level: crate::config::LogLevel, module: &str, message: &str) {
        let Some(log) = self.log_file.as_ref() else {
            return;
        };
        let _ = log.append(level, &format!("{module}: {message}"));
    }

    /// Record a `Debug` diagnostic line for `module`.
    pub(crate) fn debug(&self, module: &str, message: &str) {
        self.record(crate::config::LogLevel::Debug, module, message);
    }

    /// Record a `Warning` diagnostic line for `module`.
    pub(crate) fn warn(&self, module: &str, message: &str) {
        self.record(crate::config::LogLevel::Warning, module, message);
    }

    /// Record an `Error` diagnostic line for `module`.
    pub(crate) fn error(&self, module: &str, message: &str) {
        self.record(crate::config::LogLevel::Error, module, message);
    }

    fn start_transport(&mut self) -> Result<(), String> {
        self.websocket_error = None;
        match DesktopWebSocketServer::start(
            self.executor.service(),
            self.executor.database_path().to_owned(),
        ) {
            Ok(server) => self.websocket_server = Some(server),
            Err(error) => self.websocket_error = Some(error),
        }
        #[cfg(unix)]
        {
            let endpoint = crate::transport::transport_endpoint(&self.portable_root);
            self.transport_server = Some(
                crate::transport::DesktopTransportServer::start_with_pairing(
                    self.executor.service(),
                    self.executor.database_path().to_owned(),
                    endpoint,
                    self.websocket_server
                        .as_ref()
                        .map(|server| server.pairing()),
                )?,
            );
        }
        #[cfg(windows)]
        {
            self.transport_error = None;
            let endpoint = crate::windows_transport::transport_endpoint();
            self.transport_server = Some(crate::windows_transport::DesktopTransportServer::start(
                self.executor.service(),
                self.executor.database_path().to_owned(),
                endpoint,
            )?);
        }
        Ok(())
    }

    fn stop_transport(&mut self) {
        self.transport_server.take();
        self.websocket_server.take();
    }

    /// Replace the executor while keeping every Browser transport entry point on
    /// the same service generation. A transport server captures an executor
    /// handle when it starts, so replacing only `RuntimeState.executor` would
    /// leave Native Host requests pointing at the closed generation.
    pub(crate) fn replace_executor(
        &mut self,
        config: crate::executor::ExecutorConfig,
    ) -> Result<(), String> {
        self.stop_transport();
        self.executor
            .shutdown_in_place()
            .map_err(|error| error.to_string())?;
        self.executor = ExecutorRuntime::with_config(config);
        self.start_transport()?;
        self.executor
            .recover_startup()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(crate) fn initialize() -> Self {
        Self::initialize_at(portable_root())
    }

    /// Initialize runtime state for an explicit portable root.
    ///
    /// The archive database is intentionally decoupled from the download
    /// directory setup: `config/archive.sqlite3` is created on every launch so
    /// the job list and SQLite status work before the user has selected a
    /// download directory. Archiving itself remains gated on
    /// `download_setup_required` at the command boundary.
    pub(crate) fn initialize_at(portable_root: PathBuf) -> Self {
        let paths = PortablePaths::from_root(portable_root.clone());
        let (config, config_error) = AppConfig::load(&paths);
        let database_path = config.database_path(&paths);
        let cache_root = config.cache_path(&paths);
        let staging_root = cache_root.join("staging");
        let download_root = config.download_path(&paths);
        let logs_root = config.logs_path(&paths);
        let download_setup_required = !download_root.is_dir();
        let _ = paths.ensure_runtime_dirs();
        let log_file = LogFile::open(
            &logs_root,
            config.logging.effective_level(),
            config.logging.max_files,
        )
        .map(|log| log.with_secrets(config.log_secrets()))
        .ok();
        if let Some(log) = log_file.as_ref() {
            // Record how the effective level was chosen, otherwise a user who
            // wonders why a pre-release is verbose has no way to tell the
            // channel default apart from an inherited setting.
            let _ = log.append(
                crate::config::LogLevel::Info,
                &format!(
                    "application runtime initialized (channel={} channel_default={} effective_level={} user_override={})",
                    crate::build_channel::release_channel().as_str(),
                    crate::build_channel::channel_default_log_level().as_str(),
                    config.logging.effective_level().as_str(),
                    config
                        .logging
                        .level
                        .map(|level| level.as_str())
                        .unwrap_or("none"),
                ),
            );
        }
        let database = (|| {
            std::fs::create_dir_all(database_path.parent()?).ok()?;
            Database::open(&database_path).ok()
        })();
        let database_ready = database.is_some();
        // The configuration error is consumed by `database_error` below, so the
        // diagnostic copy is taken before that happens.
        let config_error_diagnostic = config_error.clone();
        let database_error = if let Some(error) = config_error {
            Some(error)
        } else if database_ready {
            None
        } else {
            Some("failed to initialize archive database".to_owned())
        };

        let state = Self {
            portable_root,
            cache_root,
            download_root: download_root.clone(),
            logs_root,
            download_setup_required,
            config: config.clone(),
            log_file,
            database,
            database_ready,
            database_error,
            executor: ExecutorRuntime::with_config(executor_config(
                &paths.root,
                &config,
                database_path,
                staging_root.clone(),
                download_root.clone(),
            )),
            #[cfg(unix)]
            transport_server: None,
            #[cfg(windows)]
            transport_server: None,
            #[cfg(windows)]
            transport_error: None,
            websocket_server: None,
            websocket_error: None,
            sidecar: None,
            sidecar_error: None,
            batch_cancellations: Arc::new(StdMutex::new(HashMap::new())),
        };
        let mut state = state;
        state.debug(
            "runtime",
            &format!(
                "root={} database={} ready={} download_root={} setup_required={} staging={} cache={}",
                state.portable_root.display(),
                state.config.database_path(&paths).display(),
                database_ready,
                state.download_root.display(),
                state.download_setup_required,
                state.cache_root.join("staging").display(),
                state.cache_root.display(),
            ),
        );
        if let Some(error) = &state.database_error {
            state.warn("database", &format!("unavailable: {error}"));
        }
        if let Some(error) = config_error_diagnostic {
            state.warn("config", &error);
        }
        state.debug(
            "network",
            &format!(
                "proxy_mode={} summary={:?} timeouts=extraction:{}s/discovery:{}s/aria2:{}s+tries:{}",
                state.config.network.proxy_mode.as_str(),
                state.config.network.diagnostics(),
                state.config.network.extraction_timeout_seconds,
                state.config.network.discovery_timeout_seconds,
                state.config.network.aria2_idle_timeout_seconds,
                state.config.network.aria2_max_tries,
            ),
        );
        if let Err(error) = state.start_transport() {
            #[cfg(windows)]
            {
                state.transport_error = Some(error.clone());
                state.warn("transport", &format!("start failed: {error}"));
            }
            #[cfg(unix)]
            {
                state.debug("transport", &format!("start failed: {error}"));
            }
        } else {
            state.debug("transport", "browser transport server started");
        }
        match state.executor.recover_startup() {
            Ok(()) => state.debug("executor", "startup recovery completed"),
            Err(error) => state.warn("executor", &format!("startup recovery failed: {error}")),
        }
        state
    }
}

pub(crate) fn executor_config(
    root: &std::path::Path,
    config: &AppConfig,
    database_path: PathBuf,
    staging_root: PathBuf,
    archive_root: PathBuf,
) -> crate::executor::ExecutorConfig {
    crate::executor::ExecutorConfig {
        archive_root,
        staging_root,
        database_path,
        sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok().or_else(|| {
            let worker = crate::portable::resolve_config_path(root, &config.sidecar.worker);
            worker.is_file().then(|| worker.display().to_string())
        }),
        sidecar_args: sidecar_runtime_args(root, config),
        aria2_program: Some(
            std::env::var("XARCHIVE_ARIA2_PROGRAM")
                .ok()
                .unwrap_or_else(|| {
                    crate::portable::resolve_config_path(root, &config.sidecar.aria2)
                        .display()
                        .to_string()
                }),
        ),
        network: crate::executor::ExecutorNetworkConfig::from_seconds(
            config.network.transfer_timeout_seconds,
            config.network.telegram_timeout_seconds,
            config.network.aria2_connect_timeout_seconds,
            config.network.aria2_idle_timeout_seconds,
            config.network.aria2_max_tries,
            config.network.extraction_timeout_seconds,
            config.network.discovery_timeout_seconds,
        )
        .with_proxy(config.network.proxy_mode, config.network.normalized_proxy()),
    }
}

pub(crate) fn configured_sidecar_args(root: &std::path::Path, config: &AppConfig) -> Vec<String> {
    if let Ok(raw) = std::env::var("XARCHIVE_SIDECAR_ARGS") {
        return serde_json::from_str(&raw).unwrap_or_default();
    }
    let gallery = crate::portable::resolve_config_path(root, &config.sidecar.gallery_dl);
    vec!["--gallery-dl".to_owned(), gallery.display().to_string()]
}

pub(crate) fn sidecar_runtime_args(root: &std::path::Path, config: &AppConfig) -> Vec<String> {
    let mut args = configured_sidecar_args(root, config);
    args.extend(config.network.sidecar_args());
    args
}

impl Drop for RuntimeState {
    fn drop(&mut self) {
        // Keep the executor worker lifetime bounded by the application runtime.
        let _ = self.executor.shutdown_in_place();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_sidecar_args_passes_portable_gallery_dl_path_to_worker() {
        // Build the expected path via PathBuf::join so the assertion is
        // separator-agnostic and valid on Windows (backslash) as well as Unix.
        // Use a relative fixture so the root itself is valid on both Unix and
        // Windows; only the path construction is under test here.
        let root = std::path::Path::new("xarchive with spaces");
        let expected_gallery_dl = std::path::PathBuf::from(root)
            .join("sidecar")
            .join("gallery-dl")
            .join("gallery-dl.exe")
            .display()
            .to_string();
        let config = AppConfig::default();
        assert_eq!(
            configured_sidecar_args(root, &config),
            vec!["--gallery-dl".to_owned(), expected_gallery_dl]
        );
    }

    #[test]
    fn executor_config_preserves_gallery_dl_and_network_arguments() {
        let root = std::path::Path::new("portable root");
        let mut config = AppConfig::default();
        config.network.discovery_timeout_seconds = 123;
        let executor = executor_config(
            root,
            &config,
            PathBuf::from("config/archive.sqlite3"),
            PathBuf::from("cache/staging"),
            PathBuf::from("download"),
        );
        let gallery = crate::portable::resolve_config_path(root, &config.sidecar.gallery_dl)
            .display()
            .to_string();
        let expected_gallery = ["--gallery-dl".to_owned(), gallery];
        let gallery_index = executor
            .sidecar_args
            .windows(2)
            .position(|pair| pair == expected_gallery)
            .expect("portable gallery-dl arguments");
        let timeout_index = executor
            .sidecar_args
            .windows(2)
            .position(|pair| pair == ["--discovery-timeout-seconds", "123"])
            .expect("network discovery timeout");
        assert!(gallery_index < timeout_index);
    }

    #[cfg(unix)]
    #[test]
    fn replacing_executor_restarts_transport_on_the_new_service_generation() {
        use xarchive_native_host::{read_json, write_json};
        use xarchive_protocol::{BrowserRequest, BrowserResponse, BrowserTweet, PROTOCOL_VERSION};

        let root = std::env::temp_dir().join(format!(
            "xarchive-runtime-replace-{}-{}",
            std::process::id(),
            timestamp_marker()
        ));
        let mut state = RuntimeState::initialize_at(root.clone());
        let old_service = state.executor.service();
        let config = state.executor.config().clone();
        state
            .replace_executor(config)
            .expect("replace executor and transport");

        assert_eq!(
            old_service.query("old-job").unwrap_err(),
            crate::executor::ExecutorError::Closed
        );

        let endpoint = crate::transport::transport_endpoint(&root);
        let mut stream =
            std::os::unix::net::UnixStream::connect(&endpoint).expect("connect socket");
        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "after-replace".to_owned(),
            tweet: BrowserTweet {
                tweet_id: "123".to_owned(),
                url: "https://x.com/alice/status/123".to_owned(),
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
                text: Some("archive".to_owned()),
                created_at: Some("2026-09-24T00:00:00Z".to_owned()),
                tweet_type: "post".to_owned(),
                reply_to: None,
                quoted_tweet: None,
            },
        };
        write_json(&mut stream, &request).expect("write request");
        let response: BrowserResponse = read_json(&mut stream)
            .expect("read response")
            .expect("browser response");
        assert!(matches!(
            response,
            BrowserResponse::ArchiveStatus {
                request_id,
                state,
                ..
            } if request_id == "after-replace" && state == "QUEUED"
        ));
        drop(state);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parse_sidecar_args_accepts_explicit_json_array() {
        assert_eq!(
            crate::commands::parse_sidecar_args(r#"["-m","xarchive_downloader"]"#).unwrap(),
            vec!["-m", "xarchive_downloader"]
        );
    }
}
