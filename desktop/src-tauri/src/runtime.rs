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
    pub(crate) telegram_worker: Option<crate::telegram_worker::TelegramWorker>,
    pub(crate) telegram_secrets: Option<SharedTelegramSecrets>,
    pub(crate) telegram_sender_error: Option<String>,
    pub(crate) telegram_migration_busy: bool,
    pub(crate) telegram_batch_error: Arc<StdMutex<Option<String>>>,
    pub(crate) telegram_progress: Arc<StdMutex<HashMap<String, xarchive_telegram::UploadStage>>>,
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

/// Shared platform adapter handle. The wrapper sanitizes native errors before
/// they can reach commands, logs or the sender. It never stores a token itself.
#[derive(Clone)]
pub(crate) struct SharedTelegramSecrets(
    Arc<StdMutex<Box<dyn xarchive_telegram::SecretStore + Send>>>,
);

impl xarchive_telegram::SecretStore for SharedTelegramSecrets {
    fn get(&self, key: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
        self.0
            .lock()
            .map_err(|_| secret_unavailable())?
            .get(key)
            .map_err(|_| secret_unavailable())
    }
    fn set(&mut self, key: &str, value: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
        self.0
            .lock()
            .map_err(|_| secret_unavailable())?
            .set(key, value)
            .map_err(|_| secret_unavailable())
    }
    fn delete(&mut self, key: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
        self.0
            .lock()
            .map_err(|_| secret_unavailable())?
            .delete(key)
            .map_err(|_| secret_unavailable())
    }
}

fn secret_unavailable() -> xarchive_telegram::SecretStoreError {
    xarchive_telegram::SecretStoreError::Unavailable("platform credential access failed".into())
}

pub(crate) fn timestamp_marker() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

impl RuntimeState {
    /// Windows Owner calls this with its native adapter during setup. Installing
    /// the adapter also starts enabled sending; no adapter means fail closed.
    #[allow(dead_code)]
    pub(crate) fn install_telegram_secrets(
        &mut self,
        store: Box<dyn xarchive_telegram::SecretStore + Send>,
    ) -> Result<(), String> {
        if !self.stop_telegram_worker(std::time::Duration::from_secs(2)) {
            return Err("telegram worker still stopping".into());
        }
        self.telegram_secrets = Some(SharedTelegramSecrets(Arc::new(StdMutex::new(store))));
        self.restart_telegram_sender()
    }

    pub(crate) fn telegram_proxy(&self) -> Result<Option<String>, String> {
        if self.config.telegram.endpoint_mode == xarchive_telegram::EndpointMode::Local {
            return Ok(None);
        }
        match crate::proxy::decide(
            self.config.network.proxy_mode,
            self.config.network.normalized_proxy(),
            crate::proxy::platform_resolver().as_ref(),
            &self.config.telegram.api_base,
        ) {
            xarchive_core::ProxyDecision::Direct => Ok(None),
            xarchive_core::ProxyDecision::Proxy(value) => Ok(Some(value)),
            _ => Err("telegram proxy resolution unavailable".into()),
        }
    }

    pub(crate) fn restart_telegram_sender(&mut self) -> Result<(), String> {
        let result = (|| {
            if !self.stop_telegram_worker(std::time::Duration::from_secs(2)) {
                return Err("telegram worker still stopping".into());
            }
            if self.config.telegram.migration_pending {
                return Err("endpoint migration pending; sender remains paused".into());
            }
            if !self.config.telegram.enabled {
                return Ok(());
            }
            let secrets = self
                .telegram_secrets
                .clone()
                .ok_or("platform credential provider unavailable")?;
            let error_state = self.telegram_batch_error.clone();
            self.start_telegram_sender(secrets, self.telegram_proxy()?, move |result| {
                if let Ok(mut error) = error_state.lock() {
                    *error = result
                        .err()
                        .map(|_| "telegram batch failed; durable state retained".into());
                }
            })
        })();
        self.telegram_sender_error = result.as_ref().err().cloned();
        result
    }

    pub(crate) fn delete_telegram_secret(&mut self) -> Result<(), String> {
        use xarchive_telegram::SecretStore;
        if !self.stop_telegram_worker(std::time::Duration::from_secs(2)) {
            return Err("telegram worker still stopping".into());
        }
        let mut store = self
            .telegram_secrets
            .clone()
            .ok_or("platform credential provider unavailable")?;
        let database = self.database.as_ref().ok_or("database unavailable")?;
        if let Some((generation, _, reference)) = database
            .active_telegram_credential_generation()
            .map_err(|_| "credential state unavailable")?
        {
            database
                .retire_telegram_credential_generation(generation)
                .map_err(|_| "credential revocation failed")?;
            store
                .delete(&reference)
                .map_err(|_| "inactive credential cleanup failed")?;
        }
        Ok(())
    }
    /// Platform entry point: inject credentials, never synthesize a fallback.
    #[allow(dead_code)] // Windows Owner supplies the production adapter.
    pub(crate) fn start_telegram_sender<S, E>(
        &mut self,
        secrets: S,
        cloud_proxy: Option<String>,
        report: E,
    ) -> Result<(), String>
    where
        S: xarchive_telegram::SecretStore + Send + 'static,
        E: FnMut(Result<crate::telegram_send::SendRunSummary, String>) + Send + 'static,
    {
        if !self.stop_telegram_worker(std::time::Duration::from_secs(2)) {
            return Err("telegram worker still stopping".into());
        }
        if !self.config.telegram.enabled {
            return Ok(());
        }
        if !self.database_ready || self.download_setup_required {
            return Err("telegram runtime prerequisites unavailable".into());
        }
        self.telegram_worker = Some(crate::telegram_worker::TelegramWorker::start_sender(
            std::time::Duration::from_secs(5),
            self.executor.database_path().to_owned(),
            self.download_root.clone(),
            self.config.telegram.clone(),
            cloud_proxy,
            secrets,
            {
                self.telegram_progress
                    .lock()
                    .map_err(|_| "progress unavailable")?
                    .clear();
                self.telegram_progress.clone()
            },
            report,
        )?);
        Ok(())
    }

    /// Attach an injected sender only after its dependencies are available.
    /// Never replace a worker which still has an in-flight batch.
    #[allow(dead_code)] // Production caller awaits credential-provider injection.
    pub(crate) fn install_telegram_worker<F>(
        &mut self,
        interval: std::time::Duration,
        batch: F,
    ) -> Result<(), String>
    where
        F: FnMut(&crate::telegram_worker::WorkerStopSignal) + Send + 'static,
    {
        if !self.stop_telegram_worker(std::time::Duration::from_secs(2)) {
            return Err("telegram worker still stopping".into());
        }
        self.telegram_worker = Some(
            crate::telegram_worker::TelegramWorker::start_cooperative(interval, batch)
                .map_err(|_| "telegram worker start failed".to_owned())?,
        );
        Ok(())
    }

    pub(crate) fn stop_telegram_worker(&mut self, timeout: std::time::Duration) -> bool {
        if let Some(worker) = self.telegram_worker.as_mut()
            && !worker.stop_and_wait(timeout)
        {
            return false;
        }
        self.telegram_worker = None;
        true
    }

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
            self.transport_server = Some(
                crate::windows_transport::DesktopTransportServer::start_with_pairing(
                    self.executor.service(),
                    self.executor.database_path().to_owned(),
                    endpoint,
                    self.websocket_server
                        .as_ref()
                        .map(|server| server.pairing()),
                )?,
            );
        }
        Ok(())
    }

    fn stop_transport(&mut self) {
        self.transport_server.take();
        self.websocket_server.take();
    }

    /// Materialize captured archived plans without granting sending authority.
    /// A malformed intent is isolated and remains available for later review.
    fn reconcile_telegram_plans(&self) {
        let Some(database) = self.database.as_ref().filter(|_| self.database_ready) else {
            return;
        };
        match crate::telegram_send::reconcile_archived_send_intents(
            database,
            &crate::clock::now_iso(),
        ) {
            Ok(results) => {
                for (tweet_id, result) in results {
                    match result {
                        Ok(ids) => self.debug(
                            "telegram",
                            &format!("archive intent {tweet_id}: queued {} units", ids.len()),
                        ),
                        Err(_) => self.warn(
                            "telegram",
                            &format!(
                                "archive intent {tweet_id}: planning failed; retained for review"
                            ),
                        ),
                    }
                }
            }
            Err(_) => self.warn("telegram", "archive intent reconciliation unavailable"),
        }
    }

    /// Recover journaled file commits before executor startup examines jobs.
    /// Use a separate connection; never replace the runtime's database handle.
    fn recover_telegram_archives(&self) {
        if !self.database_ready {
            return;
        }
        let config = self.executor.config();
        let recovery = (|| -> Result<(), xarchive_storage::StorageError> {
            let database = Database::open(&config.database_path)?;
            let records = database.list_recoverable_telegram_archive_intents()?;
            if !records.iter().any(|record| record.state == "PREPARED") {
                return Ok(());
            }
            let files = xarchive_storage::FileStore::with_staging_root(
                config.archive_root.clone(),
                config.staging_root.clone(),
            )?;
            let mut service = xarchive_storage::ArchiveService::new(database, files);
            for record in records
                .into_iter()
                .filter(|record| record.state == "PREPARED")
            {
                if service
                    .recover_telegram_archive_intent(&record, &crate::clock::now_iso())
                    .is_err()
                {
                    self.warn(
                        "telegram",
                        &format!(
                            "archive intent {}: file recovery failed; retained for review",
                            record.tweet_row_id
                        ),
                    );
                }
            }
            Ok(())
        })();
        if recovery.is_err() {
            self.warn("telegram", "archive intent file recovery unavailable");
        }
    }

    /// Replace the executor while keeping every Browser transport entry point on
    /// the same service generation. A transport server captures an executor
    /// handle when it starts, so replacing only `RuntimeState.executor` would
    /// leave Native Host requests pointing at the closed generation.
    pub(crate) fn replace_executor(
        &mut self,
        config: crate::executor::ExecutorConfig,
    ) -> Result<(), String> {
        if !self.stop_telegram_worker(std::time::Duration::from_secs(2)) {
            return Err("telegram worker still stopping".into());
        }
        self.stop_transport();
        self.executor
            .shutdown_in_place()
            .map_err(|error| error.to_string())?;
        self.executor = ExecutorRuntime::with_config(config);
        self.start_transport()?;
        self.recover_telegram_archives();
        self.executor
            .recover_startup()
            .map_err(|error| error.to_string())?;
        self.reconcile_telegram_plans();
        self.restart_telegram_sender()?;
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
            telegram_worker: None,
            telegram_secrets: None,
            telegram_sender_error: None,
            telegram_batch_error: Arc::new(StdMutex::new(None)),
            telegram_migration_busy: false,
            telegram_progress: Arc::new(StdMutex::new(HashMap::new())),
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
        state.recover_telegram_archives();
        match state.executor.recover_startup() {
            Ok(()) => {
                state.debug("executor", "startup recovery completed");
                state.reconcile_telegram_plans();
            }
            Err(error) => state.warn("executor", &format!("startup recovery failed: {error}")),
        }
        let _ = state.restart_telegram_sender();
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
        telegram_config_file: Some(crate::portable::PortablePaths::from_root(root).config_file),
        telegram: config.telegram.clone(),
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
        self.stop_telegram_worker(std::time::Duration::from_secs(2));
        // Keep the executor worker lifetime bounded by the application runtime.
        let _ = self.executor.shutdown_in_place();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_lifecycle_fails_closed_and_deletion_revokes_before_native_failure() {
        struct FailingDelete;
        impl xarchive_telegram::SecretStore for FailingDelete {
            fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
                Ok(None)
            }
            fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                Ok(())
            }
            fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                Err(xarchive_telegram::SecretStoreError::Unavailable(
                    "sensitive-native-detail".into(),
                ))
            }
        }
        let root = std::env::temp_dir().join(format!(
            "tg-provider-{}-{}",
            std::process::id(),
            timestamp_marker()
        ));
        let mut state = RuntimeState::initialize_at(root.clone());
        state.config.telegram.enabled = true;
        assert_eq!(
            state.restart_telegram_sender().unwrap_err(),
            "platform credential provider unavailable"
        );
        assert!(state.telegram_worker.is_none());
        state.config.telegram.enabled = false;
        state
            .install_telegram_secrets(Box::new(FailingDelete))
            .unwrap();
        let database = state.database.as_ref().unwrap();
        let generation = database
            .prepare_telegram_credential_generation("telegram-bot:123", "native-ref", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(generation, None)
            .unwrap();
        assert_eq!(
            state.delete_telegram_secret().unwrap_err(),
            "inactive credential cleanup failed"
        );
        assert!(
            state
                .database
                .as_ref()
                .unwrap()
                .active_telegram_credential_generation()
                .unwrap()
                .is_none()
        );
        state.telegram_secrets.as_mut().unwrap();
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn injected_provider_starts_and_restarts_exactly_one_worker_with_loopback_direct() {
        let root = std::env::temp_dir().join(format!(
            "tg-restart-{}-{}",
            std::process::id(),
            timestamp_marker()
        ));
        let mut state = RuntimeState::initialize_at(root.clone());
        state.download_setup_required = false;
        state.config.telegram.enabled = true;
        state.config.telegram.endpoint_mode = xarchive_telegram::EndpointMode::Local;
        state.config.telegram.api_base = "http://127.0.0.1:9".into();
        state.config.network.proxy_mode = crate::config::ProxyMode::Manual;
        state.config.network.proxy = Some("http://127.0.0.1:1".into());
        assert_eq!(state.telegram_proxy().unwrap(), None);
        state
            .install_telegram_secrets(Box::new(xarchive_telegram::MemorySecretStore::default()))
            .unwrap();
        assert!(state.telegram_worker.is_some());
        state.restart_telegram_sender().unwrap();
        assert!(state.telegram_worker.is_some());
        state.config.telegram.enabled = false;
        state.restart_telegram_sender().unwrap();
        assert!(state.telegram_worker.is_none());
        drop(state);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disabled_runtime_does_not_start_sender_or_read_credentials() {
        struct ForbiddenSecrets;
        impl xarchive_telegram::SecretStore for ForbiddenSecrets {
            fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
                panic!("disabled runtime must not read credentials");
            }
            fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("disabled runtime must not write credentials");
            }
            fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("disabled runtime must not delete credentials");
            }
        }
        let root = std::env::temp_dir().join(format!(
            "telegram-runtime-disabled-{}-{}",
            std::process::id(),
            timestamp_marker()
        ));
        let mut state = RuntimeState::initialize_at(root.clone());
        assert!(!state.config.telegram.enabled);
        state
            .start_telegram_sender(ForbiddenSecrets, None, |_| panic!("no batch expected"))
            .unwrap();
        assert!(state.telegram_worker.is_none());
        drop(state);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_refuses_replacement_until_inflight_worker_exits() {
        let root = std::env::temp_dir().join(format!(
            "telegram-runtime-replace-{}-{}",
            std::process::id(),
            timestamp_marker()
        ));
        let mut state = RuntimeState::initialize_at(root.clone());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        state
            .install_telegram_worker(std::time::Duration::from_secs(60), move |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
            .unwrap();
        state.telegram_worker.as_ref().unwrap().wake();
        started_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!state.stop_telegram_worker(std::time::Duration::ZERO));
        assert!(state.telegram_worker.is_some());
        let (replacement_tx, replacement_rx) = std::sync::mpsc::channel();
        assert_eq!(
            state
                .install_telegram_worker(std::time::Duration::from_secs(60), move |_| {
                    replacement_tx.send(()).unwrap();
                })
                .unwrap_err(),
            "telegram worker still stopping"
        );
        assert!(state.telegram_worker.is_some());
        assert!(replacement_rx.try_recv().is_err());
        release_tx.send(()).unwrap();
        assert!(state.stop_telegram_worker(std::time::Duration::from_secs(2)));
        let (tx, rx) = std::sync::mpsc::channel();
        state
            .install_telegram_worker(std::time::Duration::from_secs(60), move |_| {
                tx.send(()).unwrap();
            })
            .unwrap();
        state.telegram_worker.as_ref().unwrap().wake();
        rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert!(state.stop_telegram_worker(std::time::Duration::from_secs(2)));
        drop(state);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_owns_and_stops_injected_telegram_worker() {
        let root = std::env::temp_dir().join(format!(
            "telegram-runtime-{}-{}",
            std::process::id(),
            timestamp_marker()
        ));
        let mut state = RuntimeState::initialize_at(root.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        state
            .install_telegram_worker(std::time::Duration::from_secs(60), move |signal| {
                assert!(!signal.is_stopping());
                tx.send(()).unwrap();
            })
            .unwrap();
        state.telegram_worker.as_ref().unwrap().wake();
        rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert!(state.stop_telegram_worker(std::time::Duration::from_secs(2)));
        assert!(state.telegram_worker.is_none());
        drop(state);
        let _ = std::fs::remove_dir_all(root);
    }

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
