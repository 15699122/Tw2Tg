use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::portable::{PortablePaths, resolve_config_path};

use xarchive_telegram::{
    EndpointMode, SecretStore, SecretStoreError, TELEGRAM_CLOUD_API_BASE, TelegramEndpoint,
    UploadMode,
};

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

/// Logging behavior for the application.
///
/// `level` is optional on purpose. `None` means "follow the build channel",
/// so a fresh pre-release installation defaults to `debug` while a fresh
/// stable release defaults to `info`. `Some(..)` is an explicit user choice and
/// always wins over the channel default, which is what keeps an existing
/// installation's stored level from being silently rewritten on upgrade.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_logs_directory")]
    pub directory: String,
    #[serde(default)]
    pub level: Option<LogLevel>,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

impl LoggingConfig {
    /// The level the application actually applies.
    ///
    /// This is the only value the runtime, the status command, and the log
    /// page should read; reading `level` directly would confuse "not chosen"
    /// with "chosen as the default".
    pub fn effective_level(&self) -> LogLevel {
        self.level
            .unwrap_or_else(crate::build_channel::channel_default_log_level)
    }
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
    /// Whether the desktop may use the configured aria2 executable.
    ///
    /// `false` is the historical and current default: gallery-dl performs
    /// media byte downloads and aria2 is not started. `true` keeps the
    /// legacy gallery-dl-extraction/aria2-transfer split.
    #[serde(default)]
    pub use_aria2: bool,
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

/// How the application routes outbound network traffic.
///
/// The three states and their meaning are owned by the shared layer, so the
/// Desktop configuration, the child-process rules, and the settings surface
/// cannot drift apart.
pub use xarchive_core::ProxyMode;

/// Network configuration shared by extraction, aria2 transfer and Telegram.
///
/// One section feeds every network boundary so a deployment behind a proxy is
/// configured once and diagnosed in one place. `proxy` may carry credentials:
/// it is handed to child processes (through the environment where the tool
/// supports it) and is only ever exposed through [`NetworkConfig::redacted_proxy`]
/// and [`NetworkConfig::diagnostics`].
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    /// The routing mode.
    ///
    /// A configuration written before this field existed deserializes as
    /// `System`, which is what the application did at the time. `load` then
    /// consults [`NetworkConfig::proxy_mode_declared`] to tell that case apart
    /// from a document that deliberately states `system`, because the two must
    /// not be migrated the same way.
    #[serde(default)]
    pub proxy_mode: ProxyMode,
    /// Whether the loaded document actually carried a `proxy_mode` key.
    ///
    /// This is derived from the raw document at load time and is never written
    /// back, so a save always records the current mode explicitly.
    #[serde(skip)]
    pub proxy_mode_declared: bool,
    /// The manual proxy value. Only meaningful for [`ProxyMode::Manual`].
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
            proxy_mode: ProxyMode::System,
            // A default-constructed value was never read from a document, so it
            // carries no legacy claim.
            proxy_mode_declared: false,
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
    pub proxy_mode: String,
    pub proxy: Option<String>,
    pub proxy_configured: bool,
    /// The manual value in use. A configured value that the current mode
    /// ignores is reported as not configured, so the UI cannot imply that a
    /// stored value is in effect.
    pub proxy_active: bool,
    pub extraction_timeout_seconds: u64,
    pub discovery_timeout_seconds: u64,
    pub aria2_connect_timeout_seconds: u64,
    pub aria2_idle_timeout_seconds: u64,
    pub aria2_max_tries: u32,
    pub transfer_timeout_seconds: u64,
    pub telegram_timeout_seconds: u64,
}

impl NetworkConfig {
    /// Reconcile a freshly loaded configuration with the proxy mode rules.
    ///
    /// A configuration written before the mode existed carried only `proxy`. A
    /// non-empty value therefore means the user had configured a proxy by hand,
    /// so it loads as `Manual` rather than silently becoming `System` and
    /// changing the route. An empty value loads as `System`, which reproduces
    /// the previous inherit-the-environment behavior.
    ///
    /// A document that states its mode is never rewritten: a user who selected
    /// `system` while a value was still stored must keep that choice.
    pub fn apply_legacy_migration(&mut self) {
        if self.proxy_mode_declared {
            return;
        }
        if self.normalized_proxy().is_some() {
            self.proxy_mode = ProxyMode::Manual;
        }
    }

    /// The configured proxy with surrounding whitespace removed.
    pub fn normalized_proxy(&self) -> Option<String> {
        self.proxy
            .as_deref()
            .map(str::trim)
            .filter(|proxy| !proxy.is_empty())
            .map(str::to_owned)
    }

    /// The manual proxy, but only when `Manual` is the active mode.
    ///
    /// A stored value under `System` or `Direct` is not in effect and must not
    /// be handed to any client or child process.
    pub fn active_manual_proxy(&self) -> Option<String> {
        match self.proxy_mode {
            ProxyMode::Manual => self.normalized_proxy(),
            ProxyMode::System | ProxyMode::Direct => None,
        }
    }

    /// The configured proxy in a form that is safe to display or log.
    #[allow(dead_code)] // Consumed by the platform settings/diagnostic surface.
    pub fn redacted_proxy(&self) -> Option<String> {
        self.active_manual_proxy()
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
    /// The mode travels with it so the Sidecar can suppress the variables it
    /// inherits when the mode is `Direct`.
    pub fn sidecar_env(&self) -> Vec<(String, String)> {
        let mut environment = vec![(
            "XARCHIVE_PROXY_MODE".to_owned(),
            self.proxy_mode.as_str().to_owned(),
        )];
        environment.extend(
            xarchive_core::ChildEnvironment::for_mode(self.proxy_mode, self.normalized_proxy()).set,
        );
        environment
    }

    /// The variables the Sidecar process must drop so the mode is honored.
    ///
    /// `Direct` has to actively remove inherited variables; setting an empty
    /// value is not equivalent, because a child that sees an empty string may
    /// still treat it as configured.
    pub fn sidecar_env_remove(&self) -> Vec<String> {
        xarchive_core::ChildEnvironment::for_mode(self.proxy_mode, self.normalized_proxy()).remove
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
            proxy_mode: self.proxy_mode.as_str().to_owned(),
            proxy: self.redacted_proxy(),
            proxy_configured: self.normalized_proxy().is_some(),
            proxy_active: self.active_manual_proxy().is_some(),
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
        if let Some(proxy) = self.active_manual_proxy() {
            validate_proxy(&proxy)?;
        }
        if self.proxy_mode == ProxyMode::Manual && self.normalized_proxy().is_none() {
            return Err("network.proxy is required when proxy_mode is manual".to_owned());
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

/// Secret-store key of the bot token. The value never enters the config
/// document, SQLite, logs or frontend state (plan TG-01).
#[allow(dead_code)] // Consumed by the Telegram settings surface (Batch B).
pub const TELEGRAM_BOT_TOKEN_KEY: &str = "telegram.bot_token";

/// Last verified Bot API server capability record (plan TG-01/TG-06).
///
/// It is only valid for the exact endpoint it was verified against, so any
/// endpoint, bot or configuration change invalidates it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedTelegramCapability {
    pub endpoint_mode: EndpointMode,
    pub api_base: String,
    pub verified_at: String,
    pub server_version: Option<String>,
    /// Server-reported upload ceiling in bytes: a capability ceiling, never a
    /// per-media-type guarantee.
    pub max_upload_bytes: Option<u64>,
}

/// Planned authorization policy for pending plans after a verified same-bot
/// rotation. Runtime credential activation/resume behavior is not wired yet.
/// This does not enable Telegram or authorize retrying unknown outcomes.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialRotationResumePolicy {
    #[default]
    Automatic,
    Confirm,
}

/// Non-sensitive Telegram configuration (plan TG-01).
///
/// There is deliberately no token field: the bot token lives in the
/// `SecretStore` and the UI only ever sees a presence flag.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TelegramConfig {
    /// Durable safety latch: only the migration command may clear it.
    #[serde(default)]
    pub migration_pending: bool,
    /// Off by default: automatic sending requires an explicit opt-in.
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint_mode: EndpointMode,
    #[serde(default = "default_telegram_api_base")]
    pub api_base: String,
    #[serde(default)]
    pub chat_id: String,
    #[serde(default)]
    pub message_thread_id: Option<i64>,
    #[serde(default)]
    pub auto_send_on_archive: bool,
    #[serde(default)]
    pub credential_rotation_resume_policy: CredentialRotationResumePolicy,
    #[serde(default)]
    pub upload_mode: UploadMode,
    #[serde(default = "default_telegram_connect_timeout_seconds")]
    pub connect_timeout_seconds: u64,
    #[serde(default = "default_telegram_upload_processing_timeout_seconds")]
    pub upload_processing_timeout_seconds: u64,
    #[serde(default)]
    pub capability: Option<VerifiedTelegramCapability>,
    /// Incremented on every settings change; queued outbox items bind to the
    /// value they were queued under so an edit cannot silently redirect them
    /// (plan TG-06).
    #[serde(default = "default_telegram_revision")]
    pub revision: i64,
}

/// Frontend-facing Telegram settings (plan TG-06): a presence flag instead of
/// the token, so no surface can read the secret back.
#[allow(dead_code)] // Serialized by the Telegram settings command (Batch B).
#[derive(Clone, Debug, Serialize)]
pub struct TelegramSettings {
    pub migration_pending: bool,
    pub enabled: bool,
    pub endpoint_mode: EndpointMode,
    pub api_base: String,
    pub chat_id: String,
    pub message_thread_id: Option<i64>,
    pub auto_send_on_archive: bool,
    pub credential_rotation_resume_policy: CredentialRotationResumePolicy,
    pub upload_mode: UploadMode,
    pub bot_token_present: bool,
    pub capability_verified: bool,
    pub connect_timeout_seconds: u64,
    pub upload_processing_timeout_seconds: u64,
    pub revision: i64,
}

fn default_telegram_api_base() -> String {
    TELEGRAM_CLOUD_API_BASE.to_owned()
}

fn default_telegram_connect_timeout_seconds() -> u64 {
    10
}

fn default_telegram_upload_processing_timeout_seconds() -> u64 {
    300
}

fn default_telegram_revision() -> i64 {
    1
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            migration_pending: false,
            enabled: false,
            endpoint_mode: EndpointMode::Cloud,
            api_base: default_telegram_api_base(),
            chat_id: String::new(),
            message_thread_id: None,
            auto_send_on_archive: false,
            credential_rotation_resume_policy: CredentialRotationResumePolicy::Automatic,
            upload_mode: UploadMode::Display,
            connect_timeout_seconds: default_telegram_connect_timeout_seconds(),
            upload_processing_timeout_seconds: default_telegram_upload_processing_timeout_seconds(),
            capability: None,
            revision: default_telegram_revision(),
        }
    }
}

#[allow(dead_code)] // Consumed by the Telegram settings surface (Batch B).
impl TelegramConfig {
    /// The validated endpoint contract. Cloud requires HTTPS; `local` requires
    /// an explicit loopback HTTP address with a port (plan TG-01).
    pub fn endpoint(&self) -> Result<TelegramEndpoint, String> {
        TelegramEndpoint::parse(self.endpoint_mode, &self.api_base)
            .map_err(|error| format!("telegram.api_base: {error}"))
    }

    /// Whether a verified capability record still applies to this endpoint.
    pub fn capability_matches(&self) -> bool {
        self.capability.as_ref().is_some_and(|capability| {
            capability.endpoint_mode == self.endpoint_mode
                && capability.api_base.trim_end_matches('/') == self.api_base.trim_end_matches('/')
        })
    }

    /// Record a settings change: the revision advances, which binds newly
    /// queued items to the new settings, and the capability record is dropped
    /// because it described the previous configuration.
    pub fn record_settings_change(&mut self) {
        self.revision = self.revision.saturating_add(1);
        self.capability = None;
    }

    /// Validate a proposed token, then replace the stored credential without
    /// exposing either value to configuration or frontend state. A failed
    /// verification leaves the current token untouched; the platform adapter
    /// must provide an atomic replacement for store-level failure guarantees.
    pub fn replace_bot_token(
        &self,
        store: &mut dyn SecretStore,
        candidate: &str,
        verify: impl FnOnce(&str) -> Result<String, String>,
    ) -> Result<String, String> {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            return Err("telegram bot token must not be empty".to_owned());
        }
        let verified_identity = verify(candidate)?;
        if verified_identity.trim().is_empty() {
            return Err("telegram bot identity verification returned an empty identity".to_owned());
        }
        store
            .set(TELEGRAM_BOT_TOKEN_KEY, candidate)
            .map_err(|error| match error {
                SecretStoreError::Unavailable(reason) => {
                    format!("telegram secret store unavailable: {reason}")
                }
                SecretStoreError::AccessDenied(reason) => {
                    format!("telegram secret store access denied: {reason}")
                }
                other => other.to_string(),
            })?;
        Ok(verified_identity)
    }

    /// Remove the stored bot token. No plaintext fallback is written.
    pub fn delete_bot_token(&self, store: &mut dyn SecretStore) -> Result<(), String> {
        store
            .delete(TELEGRAM_BOT_TOKEN_KEY)
            .map_err(|error| match error {
                SecretStoreError::Unavailable(reason) => {
                    format!("telegram secret store unavailable: {reason}")
                }
                SecretStoreError::AccessDenied(reason) => {
                    format!("telegram secret store access denied: {reason}")
                }
                other => other.to_string(),
            })
    }

    /// Whether a bot token is currently stored, without reading its value.
    pub fn bot_token_present(&self, store: &mut dyn SecretStore) -> Result<bool, String> {
        match store.get(TELEGRAM_BOT_TOKEN_KEY) {
            Ok(Some(value)) => Ok(!value.is_empty()),
            Ok(None) => Ok(false),
            Err(SecretStoreError::Unavailable(reason)) => {
                Err(format!("telegram secret store unavailable: {reason}"))
            }
            Err(SecretStoreError::AccessDenied(reason)) => {
                Err(format!("telegram secret store access denied: {reason}"))
            }
            Err(other) => Err(other.to_string()),
        }
    }

    /// The frontend-facing projection of these settings.
    pub fn settings(&self, bot_token_present: bool) -> TelegramSettings {
        TelegramSettings {
            migration_pending: self.migration_pending,
            enabled: self.enabled,
            endpoint_mode: self.endpoint_mode,
            api_base: self.api_base.clone(),
            chat_id: self.chat_id.clone(),
            message_thread_id: self.message_thread_id,
            auto_send_on_archive: self.auto_send_on_archive,
            credential_rotation_resume_policy: self.credential_rotation_resume_policy,
            upload_mode: self.upload_mode,
            bot_token_present,
            capability_verified: self.capability_matches(),
            connect_timeout_seconds: self.connect_timeout_seconds,
            upload_processing_timeout_seconds: self.upload_processing_timeout_seconds,
            revision: self.revision,
        }
    }

    /// Validate the Telegram section. A disabled section is always valid — a
    /// document written before these keys existed must keep loading as
    /// disabled rather than fail.
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("connect_timeout_seconds", self.connect_timeout_seconds),
            (
                "upload_processing_timeout_seconds",
                self.upload_processing_timeout_seconds,
            ),
        ] {
            if !(MIN_NETWORK_TIMEOUT_SECONDS..=MAX_NETWORK_TIMEOUT_SECONDS).contains(&value) {
                return Err(format!(
                    "telegram.{field} must be between {MIN_NETWORK_TIMEOUT_SECONDS} and {MAX_NETWORK_TIMEOUT_SECONDS} seconds"
                ));
            }
        }
        if !self.enabled {
            return Ok(());
        }
        if self.chat_id.trim().is_empty() {
            return Err("telegram.chat_id is required when Telegram is enabled".to_owned());
        }
        self.endpoint().map(|_| ())
    }
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
    /// Absent in documents written before Telegram configuration existed; it
    /// then loads as disabled rather than failing (plan TG-01).
    #[serde(default)]
    pub telegram: TelegramConfig,
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
            use_aria2: false,
        }
    }
}
impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            directory: default_logs_directory(),
            // No level here: the channel decides until the user picks one.
            level: None,
            max_files: DEFAULT_LOG_MAX_FILES,
        }
    }
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
            telegram: TelegramConfig::default(),
        }
    }
}

/// Whether a raw configuration document states a proxy mode explicitly.
///
/// The distinction matters for migration: a document that never mentions the
/// mode predates the field and may carry a legacy proxy value, while a document
/// that states `system` made a deliberate choice that must be preserved.
///
/// A document that cannot be inspected returns `None` so the caller keeps the
/// deserialized value instead of guessing.
fn declares_proxy_mode(raw: &str) -> Option<bool> {
    let document = serde_yaml::from_str::<serde_yaml::Value>(raw).ok()?;
    let Some(network) = document.get("network") else {
        return Some(false);
    };
    Some(network.get("proxy_mode").is_some())
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
        self.telegram.validate()?;
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
            // The migration runs before validation: a configuration written
            // before the mode existed carries a proxy but no mode, and it must
            // become a valid `Manual` rather than fail as an empty `Manual`.
            Ok(mut config) => {
                config.network.proxy_mode_declared =
                    declares_proxy_mode(&raw).unwrap_or(config.network.proxy_mode_declared);
                config.network.apply_legacy_migration();
                match config.validate() {
                    Ok(()) => (config, None),
                    Err(error) => (Self::default(), Some(error)),
                }
            }
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

/// Dedicated local exchange DTO, intentionally not AppConfig serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigurationExchange {
    pub schema_version: u32,
    pub kind: String,
    pub settings: ConfigurationExchangeSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigurationExchangeSettings {
    pub archive_directory: Option<String>,
    pub sidecar_worker: Option<String>,
    pub gallery_dl: Option<String>,
    pub aria2: Option<String>,
    pub use_aria2: Option<bool>,
    pub log_level: Option<LogLevel>,
    pub log_max_files: Option<usize>,
}

impl ConfigurationExchange {
    /// Linux transaction core. Caller must own the workspace maintenance guard.
    /// No runtime mutation is performed until the rename has succeeded.
    #[cfg(target_os = "linux")]
    pub fn persist_preview(
        &self,
        local: &mut AppConfig,
        paths: &PortablePaths,
        expected_revision: &str,
        _maintenance: &xarchive_workflow::maintenance::MaintenanceGuard,
    ) -> Result<ConfigurationExchangePreview, String> {
        use std::io::Write;
        let bytes = match fs::read(&paths.config_file) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        let preview = self.preview(local, &paths.root, bytes.as_deref())?;
        if preview.revision != expected_revision {
            return Err("configuration preview is stale; preview again".into());
        }
        // The directory must already exist; preview/apply never initializes it.
        let content =
            serde_yaml::to_string(&preview.candidate).map_err(|error| error.to_string())?;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temporary = paths
            .config_file
            .with_extension(format!("yaml.import-{}-{sequence}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let result = (|| {
            file.write_all(content.as_bytes())
                .map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())?;
            fs::rename(&temporary, &paths.config_file).map_err(|error| error.to_string())?;
            Ok::<(), String>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
        *local = preview.candidate.clone();
        Ok(preview)
    }
    pub fn preview(
        &self,
        local: &AppConfig,
        root: &std::path::Path,
        persisted: Option<&[u8]>,
    ) -> Result<ConfigurationExchangePreview, String> {
        use sha2::Digest;
        let revision = persisted
            .map(|bytes| format!("sha256:{:x}", sha2::Sha256::digest(bytes)))
            .unwrap_or_else(|| "missing".into());
        let mut candidate = local.clone();
        let mut preserved = Vec::new();
        for (name, value, target, directory) in [
            (
                "archive_directory",
                &self.settings.archive_directory,
                &mut candidate.download.directory,
                true,
            ),
            (
                "sidecar_worker",
                &self.settings.sidecar_worker,
                &mut candidate.sidecar.worker,
                false,
            ),
            (
                "gallery_dl",
                &self.settings.gallery_dl,
                &mut candidate.sidecar.gallery_dl,
                false,
            ),
            (
                "aria2",
                &self.settings.aria2,
                &mut candidate.sidecar.aria2,
                false,
            ),
        ] {
            if let Some(value) = value {
                let path = std::path::Path::new(value);
                let resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    root.join(path)
                };
                let valid = !value.trim().is_empty()
                    && !value.chars().any(char::is_control)
                    && std::fs::symlink_metadata(&resolved).is_ok_and(|metadata| {
                        !metadata.file_type().is_symlink()
                            && if directory {
                                metadata.is_dir()
                            } else {
                                metadata.is_file()
                            }
                    });
                if valid {
                    *target = value.clone();
                } else {
                    preserved.push(format!(
                        "{name}: target path is missing or has incompatible type"
                    ));
                }
            }
        }
        if let Some(value) = self.settings.use_aria2 {
            candidate.download.use_aria2 = value;
        }
        if let Some(value) = self.settings.log_level {
            candidate.logging.level = Some(value);
        }
        if let Some(value) = self.settings.log_max_files {
            candidate.logging.max_files = value;
        }
        candidate.validate()?;
        let restart_required = candidate.download.directory != local.download.directory
            || candidate.download.use_aria2 != local.download.use_aria2
            || candidate.sidecar.worker != local.sidecar.worker
            || candidate.sidecar.gallery_dl != local.sidecar.gallery_dl
            || candidate.sidecar.aria2 != local.sidecar.aria2;
        Ok(ConfigurationExchangePreview {
            candidate,
            revision,
            preserved,
            restart_required,
        })
    }
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() > 64 * 1024 {
            return Err("configuration exchange exceeds 64 KiB".into());
        }
        let document: Self = serde_json::from_str(raw).map_err(|error| error.to_string())?;
        if document.schema_version != 1 || document.kind != "tw2tg.configuration" {
            return Err("unsupported configuration exchange schema or kind".into());
        }
        Ok(document)
    }

    pub fn export(config: &AppConfig) -> Result<String, String> {
        serde_json::to_string_pretty(&Self {
            schema_version: 1,
            kind: "tw2tg.configuration".into(),
            settings: ConfigurationExchangeSettings {
                archive_directory: Some(config.download.directory.clone()),
                sidecar_worker: Some(config.sidecar.worker.clone()),
                gallery_dl: Some(config.sidecar.gallery_dl.clone()),
                aria2: Some(config.sidecar.aria2.clone()),
                use_aria2: Some(config.download.use_aria2),
                log_level: config.logging.level,
                log_max_files: Some(config.logging.max_files),
            },
        })
        .map_err(|error| error.to_string())
    }
}

pub(crate) struct ConfigurationExchangePreview {
    pub candidate: AppConfig,
    pub revision: String,
    pub preserved: Vec<String>,
    pub restart_required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable::PortablePaths;

    mod exchange;
    mod logging;
    mod network_proxy;
    mod paths;
    mod telegram;

    /// Load a full document the way [`AppConfig::load`] does, so the migration
    /// sees the same `proxy_mode_declared` signal a real load derives from the
    /// raw document rather than from the deserialized value.
    fn load_document(raw: &str) -> AppConfig {
        let paths = crate::portable::PortablePaths::from_root(std::env::temp_dir().join(format!(
            "xarchive-config-doc-{}-{}",
            std::process::id(),
            raw.len()
        )));
        let _ = std::fs::remove_dir_all(&paths.config_dir);
        fs::create_dir_all(&paths.config_dir).expect("config dir");
        fs::write(&paths.config_file, raw).expect("write config");
        let (config, error) = AppConfig::load(&paths);
        let _ = std::fs::remove_dir_all(&paths.config_dir);
        assert_eq!(error, None, "the document must load cleanly");
        config
    }
}
