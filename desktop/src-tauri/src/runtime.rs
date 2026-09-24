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
        let log_file = LogFile::open(&logs_root, config.logging.level, config.logging.max_files)
            .map(|log| log.with_secrets(config.log_secrets()))
            .ok();
        if let Some(log) = log_file.as_ref() {
            let _ = log.append(
                crate::config::LogLevel::Info,
                "application runtime initialized",
            );
        }
        let database = (|| {
            std::fs::create_dir_all(database_path.parent()?).ok()?;
            Database::open(&database_path).ok()
        })();
        let database_ready = database.is_some();
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
            executor: ExecutorRuntime::with_config(crate::executor::ExecutorConfig {
                archive_root: download_root.clone(),
                staging_root: staging_root.clone(),
                database_path,
                sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok().or_else(|| {
                    let worker =
                        crate::portable::resolve_config_path(&paths.root, &config.sidecar.worker);
                    worker.is_file().then(|| worker.display().to_string())
                }),
                sidecar_args: crate::runtime::sidecar_runtime_args(&paths.root, &config),
                aria2_program: Some(std::env::var("XARCHIVE_ARIA2_PROGRAM").ok().unwrap_or_else(
                    || {
                        crate::portable::resolve_config_path(&paths.root, &config.sidecar.aria2)
                            .display()
                            .to_string()
                    },
                )),
                network: crate::executor::ExecutorNetworkConfig::from_seconds(
                    config.network.transfer_timeout_seconds,
                    config.network.telegram_timeout_seconds,
                    config.network.aria2_connect_timeout_seconds,
                    config.network.aria2_idle_timeout_seconds,
                    config.network.aria2_max_tries,
                    config.network.extraction_timeout_seconds,
                    config.network.discovery_timeout_seconds,
                )
                .with_proxy(config.network.normalized_proxy()),
            }),
            #[cfg(unix)]
            transport_server: None,
            #[cfg(windows)]
            transport_server: None,
            #[cfg(windows)]
            transport_error: None,
            sidecar: None,
            sidecar_error: None,
            batch_cancellations: Arc::new(StdMutex::new(HashMap::new())),
        };
        #[cfg(unix)]
        let mut state = state;
        #[cfg(unix)]
        {
            let endpoint = crate::transport::transport_endpoint(&state.portable_root);
            state.transport_server = crate::transport::DesktopTransportServer::start(
                state.executor.service(),
                state.executor.database_path().to_owned(),
                endpoint,
            )
            .ok();
        }
        #[cfg(windows)]
        let mut state = state;
        #[cfg(windows)]
        {
            let endpoint = crate::windows_transport::transport_endpoint();
            match crate::windows_transport::DesktopTransportServer::start(
                state.executor.service(),
                state.executor.database_path().to_owned(),
                endpoint,
            ) {
                Ok(server) => state.transport_server = Some(server),
                Err(error) => state.transport_error = Some(error),
            }
        }
        let _ = state.executor.recover_startup();
        state
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
    fn parse_sidecar_args_accepts_explicit_json_array() {
        assert_eq!(
            crate::commands::parse_sidecar_args(r#"["-m","xarchive_downloader"]"#).unwrap(),
            vec!["-m", "xarchive_downloader"]
        );
    }
}
