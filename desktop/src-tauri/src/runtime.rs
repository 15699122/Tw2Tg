use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{Database, FileStore};

use crate::config::AppConfig;
use crate::executor::ExecutorRuntime;
use crate::logging::LogFile;
use crate::portable::{PortablePaths, portable_root};
#[cfg(unix)]
use crate::transport::DesktopTransportServer;

pub struct RuntimeState {
    pub(crate) portable_root: PathBuf,
    pub(crate) cache_root: PathBuf,
    pub(crate) staging_root: PathBuf,
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
    pub(crate) sidecar: Option<SidecarSupervisor>,
    pub(crate) sidecar_error: Option<String>,
}

pub(crate) fn timestamp_marker() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

impl RuntimeState {
    pub(crate) fn initialize() -> Self {
        let portable_root = portable_root();
        let paths = PortablePaths::from_root(portable_root.clone());
        let (config, config_error) = AppConfig::load(&paths);
        let database_path = config.database_path(&paths);
        let cache_root = config.cache_path(&paths);
        let staging_root = cache_root.join("staging");
        let download_root = config.download_path(&paths);
        let logs_root = config.logs_path(&paths);
        let download_setup_required = !download_root.is_dir();
        let _ = paths.ensure_runtime_dirs();
        let log_file =
            LogFile::open(&logs_root, config.logging.level, config.logging.max_files).ok();
        if let Some(log) = log_file.as_ref() {
            let _ = log.append(
                crate::config::LogLevel::Info,
                "application runtime initialized",
            );
        }
        let database = (|| {
            if download_setup_required {
                return None;
            }
            FileStore::with_staging_root(download_root.clone(), staging_root.clone()).ok()?;
            std::fs::create_dir_all(database_path.parent()?).ok()?;
            Database::open(&database_path).ok()
        })();
        let database_ready = database.is_some();
        let database_error = if let Some(error) = config_error {
            Some(error)
        } else if database_ready || download_setup_required {
            None
        } else {
            Some("failed to initialize archive database".to_owned())
        };

        let mut state = Self {
            portable_root,
            cache_root,
            staging_root: staging_root.clone(),
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
                sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok(),
                sidecar_args: std::env::var("XARCHIVE_SIDECAR_ARGS")
                    .ok()
                    .and_then(|raw| serde_json::from_str(&raw).ok())
                    .unwrap_or_default(),
            }),
            #[cfg(unix)]
            transport_server: None,
            sidecar: None,
            sidecar_error: None,
        };
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
        let _ = state.executor.recover_startup();
        state
    }
}

impl Drop for RuntimeState {
    fn drop(&mut self) {
        // Keep the executor worker lifetime bounded by the application
        // runtime. The synchronous archive fallback still owns its current
        // resources independently; this only shuts down the idle R1 worker
        // boundary during application teardown.
        let _ = self.executor.shutdown_in_place();
    }
}
