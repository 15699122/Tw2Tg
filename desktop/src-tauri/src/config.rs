use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::portable::{PortablePaths, resolve_config_path};

pub const DEFAULT_LOG_MAX_FILES: usize = 5;
pub const MIN_LOG_MAX_FILES: usize = 1;
pub const MAX_LOG_MAX_FILES: usize = 100;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warning,
    #[default]
    Info,
    Debug,
    Silent,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Silent => "silent",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_logs_directory")]
    pub directory: String,
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_logs_directory() -> String {
    "./logs".to_owned()
}
fn default_max_files() -> usize {
    DEFAULT_LOG_MAX_FILES
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StorageConfig {
    #[serde(default = "default_database_path")]
    pub database: String,
    #[serde(default = "default_cache_path")]
    pub cache: String,
}

fn default_database_path() -> String {
    "./config/archive.sqlite3".to_owned()
}
fn default_cache_path() -> String {
    "./cache".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DownloadConfig {
    #[serde(default = "default_download_mode")]
    pub mode: String,
    #[serde(default = "default_download_path")]
    pub directory: String,
    #[serde(default)]
    pub initialized: bool,
}

fn default_download_mode() -> String {
    "portable".to_owned()
}
fn default_download_path() -> String {
    "./download".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SidecarConfig {
    #[serde(default = "default_sidecar_worker_path")]
    pub worker: String,
    #[serde(default = "default_gallery_dl_path")]
    pub gallery_dl: String,
    #[serde(default = "default_aria2_path")]
    pub aria2: String,
}

fn default_sidecar_worker_path() -> String {
    "./sidecar/xarchive-downloader/xarchive-downloader.exe".to_owned()
}

fn default_gallery_dl_path() -> String {
    "./sidecar/gallery-dl/gallery-dl.exe".to_owned()
}
fn default_aria2_path() -> String {
    "./sidecar/aria2/aria2c.exe".to_owned()
}

pub const MIN_NETWORK_TIMEOUT_SECONDS: u64 = 1;
pub const MAX_NETWORK_TIMEOUT_SECONDS: u64 = 86_400;
pub const MIN_ARIA2_MAX_TRIES: u32 = 1;
pub const MAX_ARIA2_MAX_TRIES: u32 = 100;

/// Network configuration shared by extraction, aria2 transfer and Telegram.
///
/// One section feeds every network boundary so a deployment behind a proxy is
/// configured once and diagnosed in one place. `proxy` may carry credentials:
/// it is handed to child processes (through the environment where the tool
/// supports it) and is only ever exposed through [`NetworkConfig::redacted_proxy`]
/// and [`NetworkConfig::diagnostics`].
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default = "default_extraction_timeout_seconds")]
    pub extraction_timeout_seconds: u64,
    #[serde(default = "default_discovery_timeout_seconds")]
    pub discovery_timeout_seconds: u64,
    #[serde(default = "default_aria2_connect_timeout_seconds")]
    pub aria2_connect_timeout_seconds: u64,
    #[serde(default = "default_aria2_idle_timeout_seconds")]
    pub aria2_idle_timeout_seconds: u64,
    #[serde(default = "default_aria2_max_tries")]
    pub aria2_max_tries: u32,
    #[serde(default = "default_transfer_timeout_seconds")]
    pub transfer_timeout_seconds: u64,
    #[serde(default = "default_telegram_timeout_seconds")]
    pub telegram_timeout_seconds: u64,
}

fn default_extraction_timeout_seconds() -> u64 {
    300
}
fn default_discovery_timeout_seconds() -> u64 {
    600
}
fn default_aria2_connect_timeout_seconds() -> u64 {
    30
}
fn default_aria2_idle_timeout_seconds() -> u64 {
    60
}
fn default_aria2_max_tries() -> u32 {
    3
}
fn default_transfer_timeout_seconds() -> u64 {
    1800
}
fn default_telegram_timeout_seconds() -> u64 {
    30
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            proxy: None,
            extraction_timeout_seconds: default_extraction_timeout_seconds(),
            discovery_timeout_seconds: default_discovery_timeout_seconds(),
            aria2_connect_timeout_seconds: default_aria2_connect_timeout_seconds(),
            aria2_idle_timeout_seconds: default_aria2_idle_timeout_seconds(),
            aria2_max_tries: default_aria2_max_tries(),
            transfer_timeout_seconds: default_transfer_timeout_seconds(),
            telegram_timeout_seconds: default_telegram_timeout_seconds(),
        }
    }
}

/// Redacted network view exposed to the settings UI and diagnostics.
#[allow(dead_code)] // Reserved for the platform settings/diagnostic surface.
#[derive(Clone, Debug, Serialize)]
pub struct NetworkDiagnostics {
    pub proxy: Option<String>,
    pub proxy_configured: bool,
    pub extraction_timeout_seconds: u64,
    pub discovery_timeout_seconds: u64,
    pub aria2_connect_timeout_seconds: u64,
    pub aria2_idle_timeout_seconds: u64,
    pub aria2_max_tries: u32,
    pub transfer_timeout_seconds: u64,
    pub telegram_timeout_seconds: u64,
}

impl NetworkConfig {
    /// The configured proxy with surrounding whitespace removed.
    pub fn normalized_proxy(&self) -> Option<String> {
        self.proxy
            .as_deref()
            .map(str::trim)
            .filter(|proxy| !proxy.is_empty())
            .map(str::to_owned)
    }

    /// The configured proxy in a form that is safe to display or log.
    #[allow(dead_code)] // Consumed by the platform settings/diagnostic surface.
    pub fn redacted_proxy(&self) -> Option<String> {
        self.normalized_proxy()
            .map(|proxy| xarchive_core::redact_url_credentials(&proxy))
    }

    /// Literal secrets that must never reach SQLite, logs or browser messages.
    pub fn secrets(&self) -> Vec<String> {
        self.normalized_proxy().into_iter().collect()
    }

    /// Environment handed to the Sidecar worker process.
    ///
    /// The proxy travels as an environment variable rather than a command-line
    /// argument, so credentials stay out of process listings and command echoes.
    pub fn sidecar_env(&self) -> Vec<(String, String)> {
        self.normalized_proxy()
            .map(|proxy| vec![("XARCHIVE_PROXY".to_owned(), proxy)])
            .unwrap_or_default()
    }

    /// Sidecar arguments for the unified timeouts (non-secret values only).
    pub fn sidecar_args(&self) -> Vec<String> {
        vec![
            "--timeout-seconds".to_owned(),
            self.extraction_timeout_seconds.to_string(),
            "--discovery-timeout-seconds".to_owned(),
            self.discovery_timeout_seconds.to_string(),
        ]
    }

    #[allow(dead_code)] // Consumed by the platform settings/diagnostic surface.
    pub fn diagnostics(&self) -> NetworkDiagnostics {
        NetworkDiagnostics {
            proxy: self.redacted_proxy(),
            proxy_configured: self.normalized_proxy().is_some(),
            extraction_timeout_seconds: self.extraction_timeout_seconds,
            discovery_timeout_seconds: self.discovery_timeout_seconds,
            aria2_connect_timeout_seconds: self.aria2_connect_timeout_seconds,
            aria2_idle_timeout_seconds: self.aria2_idle_timeout_seconds,
            aria2_max_tries: self.aria2_max_tries,
            transfer_timeout_seconds: self.transfer_timeout_seconds,
            telegram_timeout_seconds: self.telegram_timeout_seconds,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            (
                "extraction_timeout_seconds",
                self.extraction_timeout_seconds,
            ),
            ("discovery_timeout_seconds", self.discovery_timeout_seconds),
            (
                "aria2_connect_timeout_seconds",
                self.aria2_connect_timeout_seconds,
            ),
            (
                "aria2_idle_timeout_seconds",
                self.aria2_idle_timeout_seconds,
            ),
            ("transfer_timeout_seconds", self.transfer_timeout_seconds),
            ("telegram_timeout_seconds", self.telegram_timeout_seconds),
        ] {
            if !(MIN_NETWORK_TIMEOUT_SECONDS..=MAX_NETWORK_TIMEOUT_SECONDS).contains(&value) {
                return Err(format!(
                    "network.{field} must be between {MIN_NETWORK_TIMEOUT_SECONDS} and {MAX_NETWORK_TIMEOUT_SECONDS} seconds"
                ));
            }
        }
        if !(MIN_ARIA2_MAX_TRIES..=MAX_ARIA2_MAX_TRIES).contains(&self.aria2_max_tries) {
            return Err(format!(
                "network.aria2_max_tries must be between {MIN_ARIA2_MAX_TRIES} and {MAX_ARIA2_MAX_TRIES}"
            ));
        }
        if let Some(proxy) = self.normalized_proxy() {
            validate_proxy(&proxy)?;
        }
        Ok(())
    }
}

/// Reject proxy values that cannot be handed to a child process safely.
fn validate_proxy(proxy: &str) -> Result<(), String> {
    if proxy
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err("network.proxy must not contain whitespace or control characters".to_owned());
    }
    let rest = match proxy.split_once("://") {
        Some((scheme, rest)) => {
            let scheme = scheme.to_ascii_lowercase();
            if !matches!(
                scheme.as_str(),
                "http" | "https" | "socks4" | "socks5" | "socks5h" | "ftp"
            ) {
                return Err(format!("network.proxy scheme '{scheme}' is not supported"));
            }
            rest
        }
        None => proxy,
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();
    if host.trim().is_empty() {
        return Err("network.proxy must include a host".to_owned());
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExtensionConfig {
    #[serde(default = "default_extension_path")]
    pub directory: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

fn default_extension_path() -> String {
    "./extension".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub download: DownloadConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub sidecar: SidecarConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub extension: ExtensionConfig,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database: default_database_path(),
            cache: default_cache_path(),
        }
    }
}
impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            mode: default_download_mode(),
            directory: default_download_path(),
            initialized: false,
        }
    }
}
impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            directory: default_logs_directory(),
            level: default_log_level(),
            max_files: DEFAULT_LOG_MAX_FILES,
        }
    }
}

#[cfg(debug_assertions)]
fn default_log_level() -> LogLevel {
    LogLevel::Debug
}

#[cfg(not(debug_assertions))]
fn default_log_level() -> LogLevel {
    LogLevel::Info
}
impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            worker: default_sidecar_worker_path(),
            gallery_dl: default_gallery_dl_path(),
            aria2: default_aria2_path(),
        }
    }
}
impl Default for ExtensionConfig {
    fn default() -> Self {
        Self {
            directory: default_extension_path(),
            source: "bundled".to_owned(),
            version: None,
            sha256: None,
        }
    }
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            storage: StorageConfig::default(),
            download: DownloadConfig::default(),
            logging: LoggingConfig::default(),
            sidecar: SidecarConfig::default(),
            network: NetworkConfig::default(),
            extension: ExtensionConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn validate(&mut self) -> Result<(), String> {
        if !(MIN_LOG_MAX_FILES..=MAX_LOG_MAX_FILES).contains(&self.logging.max_files) {
            return Err(format!(
                "logging.max_files must be between {MIN_LOG_MAX_FILES} and {MAX_LOG_MAX_FILES}"
            ));
        }
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported config schema_version {}",
                self.schema_version
            ));
        }
        self.network.validate()?;
        Ok(())
    }

    /// Literal secrets that must never be written to SQLite, logs or browser
    /// messages (P2-A). Kept in one place so every sink redacts the same values.
    pub fn log_secrets(&self) -> Vec<String> {
        self.network.secrets()
    }

    pub fn database_path(&self, paths: &PortablePaths) -> PathBuf {
        resolve_config_path(&paths.root, &self.storage.database)
    }
    pub fn cache_path(&self, paths: &PortablePaths) -> PathBuf {
        resolve_config_path(&paths.root, &self.storage.cache)
    }
    pub fn download_path(&self, paths: &PortablePaths) -> PathBuf {
        paths.resolve_download_directory(Some(&self.download.directory))
    }
    pub fn logs_path(&self, paths: &PortablePaths) -> PathBuf {
        resolve_config_path(&paths.root, &self.logging.directory)
    }

    pub fn load(paths: &PortablePaths) -> (Self, Option<String>) {
        let Ok(raw) = fs::read_to_string(&paths.config_file) else {
            return (Self::default(), None);
        };
        match serde_yaml::from_str::<Self>(&raw) {
            Ok(mut config) => match config.validate() {
                Ok(()) => (config, None),
                Err(error) => (Self::default(), Some(error)),
            },
            Err(error) => (
                Self::default(),
                Some(format!("invalid config.yaml: {error}")),
            ),
        }
    }

    pub fn save(&mut self, paths: &PortablePaths) -> Result<(), String> {
        self.validate()?;
        fs::create_dir_all(&paths.config_dir).map_err(|error| error.to_string())?;
        let content = serde_yaml::to_string(self).map_err(|error| error.to_string())?;
        let temporary = paths.config_file.with_extension("yaml.tmp");
        fs::write(&temporary, content).map_err(|error| error.to_string())?;
        fs::rename(&temporary, &paths.config_file).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable::PortablePaths;

    #[test]
    fn defaults_to_info_and_five_log_files() {
        let config = AppConfig::default();
        #[cfg(debug_assertions)]
        assert_eq!(config.logging.level, LogLevel::Debug);
        #[cfg(not(debug_assertions))]
        assert_eq!(config.logging.level, LogLevel::Info);
        assert_eq!(config.logging.max_files, 5);
    }

    #[test]
    fn validates_log_file_range() {
        let mut config = AppConfig::default();
        config.logging.max_files = 0;
        assert!(config.validate().is_err());
        config.logging.max_files = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn resolves_database_and_download_paths() {
        let paths = PortablePaths::from_root("/tmp/xarchive");
        let config = AppConfig::default();
        assert_eq!(
            config.database_path(&paths),
            PathBuf::from("/tmp/xarchive/config/archive.sqlite3")
        );
        assert_eq!(
            config.download_path(&paths),
            PathBuf::from("/tmp/xarchive/download")
        );
        assert_eq!(
            config.logs_path(&paths),
            PathBuf::from("/tmp/xarchive/logs")
        );
    }

    #[test]
    fn separates_worker_gallery_dl_and_extension_defaults() {
        let config = AppConfig::default();
        assert_ne!(config.sidecar.worker, config.sidecar.gallery_dl);
        assert!(config.sidecar.worker.ends_with("xarchive-downloader.exe"));
        assert!(config.sidecar.gallery_dl.ends_with("gallery-dl.exe"));
        assert_eq!(config.extension.directory, "./extension");
        assert_eq!(config.extension.source, "bundled");
    }

    #[test]
    fn worker_default_path_matches_portable_layout() {
        // Locks the contract between PyInstaller one-dir output,
        // windows-worker-artifact.yml and build-portable-windows.mjs:
        // worker must live at sidecar/xarchive-downloader/xarchive-downloader.exe
        let config = AppConfig::default();
        assert_eq!(
            config.sidecar.worker,
            "./sidecar/xarchive-downloader/xarchive-downloader.exe"
        );
    }

    #[test]
    fn network_defaults_are_bounded_and_proxy_free() {
        let network = NetworkConfig::default();
        assert!(network.proxy.is_none());
        assert!(network.normalized_proxy().is_none());
        assert!(network.sidecar_env().is_empty());
        assert_eq!(network.extraction_timeout_seconds, 300);
        assert_eq!(network.discovery_timeout_seconds, 600);
        assert_eq!(network.aria2_connect_timeout_seconds, 30);
        assert_eq!(network.aria2_idle_timeout_seconds, 60);
        assert_eq!(network.aria2_max_tries, 3);
        assert_eq!(network.transfer_timeout_seconds, 1800);
        assert_eq!(network.telegram_timeout_seconds, 30);
        assert!(network.validate().is_ok());
    }

    #[test]
    fn network_proxy_credentials_never_appear_in_diagnostics() {
        let network = NetworkConfig {
            proxy: Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            ..NetworkConfig::default()
        };
        assert!(network.validate().is_ok());
        assert_eq!(
            network.redacted_proxy().as_deref(),
            Some("http://[REDACTED]@proxy.example:8080")
        );
        let diagnostics = serde_json::to_string(&network.diagnostics()).expect("diagnostics");
        assert!(!diagnostics.contains("s3cret"));
        assert!(diagnostics.contains("proxy.example"));
        assert_eq!(
            network.secrets(),
            vec!["http://alice:s3cret@proxy.example:8080"]
        );
        assert_eq!(
            network.sidecar_env(),
            vec![(
                "XARCHIVE_PROXY".to_owned(),
                "http://alice:s3cret@proxy.example:8080".to_owned()
            )]
        );
        // The proxy is never part of the process command line.
        assert!(
            !network
                .sidecar_args()
                .iter()
                .any(|argument| argument.contains("s3cret"))
        );
        assert_eq!(
            network.sidecar_args(),
            vec![
                "--timeout-seconds",
                "300",
                "--discovery-timeout-seconds",
                "600"
            ]
        );
    }

    #[test]
    fn rejects_malformed_proxy_and_out_of_range_network_values() {
        let mut network = NetworkConfig::default();
        for proxy in [
            "http://alice:s3cret@proxy example:8080",
            "file:///etc/passwd",
            "http://",
            "http://@",
        ] {
            network.proxy = Some(proxy.to_owned());
            assert!(
                network.validate().is_err(),
                "proxy must be rejected: {proxy}"
            );
        }
        network.proxy = Some("socks5://127.0.0.1:1080".to_owned());
        assert!(network.validate().is_ok());

        network.extraction_timeout_seconds = 0;
        assert!(network.validate().is_err());
        network.extraction_timeout_seconds = default_extraction_timeout_seconds();
        network.aria2_max_tries = 0;
        assert!(network.validate().is_err());
        network.aria2_max_tries = MAX_ARIA2_MAX_TRIES + 1;
        assert!(network.validate().is_err());
    }

    #[test]
    fn app_config_validation_checks_the_network_section() {
        let mut config = AppConfig::default();
        config.network.proxy = Some("ftp://proxy.example:21".to_owned());
        assert!(config.validate().is_ok());
        config.network.transfer_timeout_seconds = 0;
        assert!(config.validate().is_err());
    }
}
