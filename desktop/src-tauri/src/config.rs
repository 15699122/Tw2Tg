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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable::PortablePaths;

    #[test]
    fn telegram_rotation_defaults_do_not_enable_sending() {
        let config: TelegramConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(
            config.credential_rotation_resume_policy,
            CredentialRotationResumePolicy::Automatic
        );
        assert!(!config.enabled);
        assert!(!config.auto_send_on_archive);
        assert_eq!(
            config.settings(false).credential_rotation_resume_policy,
            CredentialRotationResumePolicy::Automatic
        );
    }

    #[test]
    fn telegram_rotation_confirmation_survives_serialization() {
        let config = TelegramConfig {
            credential_rotation_resume_policy: CredentialRotationResumePolicy::Confirm,
            ..TelegramConfig::default()
        };
        let loaded: TelegramConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(
            loaded.credential_rotation_resume_policy,
            CredentialRotationResumePolicy::Confirm
        );
        assert_eq!(
            loaded.settings(false).credential_rotation_resume_policy,
            CredentialRotationResumePolicy::Confirm
        );
        assert!(
            serde_json::from_str::<TelegramConfig>(
                r#"{"credential_rotation_resume_policy":"invalid"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn token_replacement_verifies_before_mutating_secret_store() {
        use xarchive_telegram::{MemorySecretStore, SecretStore};

        let config = TelegramConfig::default();
        let mut store = MemorySecretStore::default();
        store
            .set(TELEGRAM_BOT_TOKEN_KEY, "old-token")
            .expect("seed existing token");

        let error = config
            .replace_bot_token(&mut store, " new-token ", |_| {
                Err("token verification failed".to_owned())
            })
            .expect_err("invalid candidate must be rejected");
        assert_eq!(error, "token verification failed");
        assert_eq!(
            store.get(TELEGRAM_BOT_TOKEN_KEY).expect("read old token"),
            Some("old-token".to_owned())
        );

        let identity = config
            .replace_bot_token(&mut store, " new-token ", |_| Ok("bot-identity".to_owned()))
            .expect("verified token replacement");
        assert_eq!(identity, "bot-identity");
        assert_eq!(
            store.get(TELEGRAM_BOT_TOKEN_KEY).expect("read new token"),
            Some("new-token".to_owned())
        );
    }

    #[test]
    fn token_delete_removes_only_the_secret_store_value() {
        use xarchive_telegram::{MemorySecretStore, SecretStore};

        let config = TelegramConfig::default();
        let mut store = MemorySecretStore::default();
        store
            .set(TELEGRAM_BOT_TOKEN_KEY, "token")
            .expect("seed token");

        config
            .delete_bot_token(&mut store)
            .expect("delete stored token");
        assert_eq!(store.get(TELEGRAM_BOT_TOKEN_KEY).expect("read token"), None);
        assert!(!config.settings(false).bot_token_present);
    }

    #[test]
    fn a_fresh_config_defers_the_level_to_the_build_channel() {
        let config = AppConfig::default();
        // No stored level: the channel decides, so a pre-release starts at
        // debug and a stable release starts at info.
        assert_eq!(config.logging.level, None);
        assert_eq!(
            config.logging.effective_level(),
            crate::build_channel::channel_default_log_level()
        );
        assert_eq!(config.logging.max_files, 5);
    }

    #[test]
    fn an_explicit_user_level_outranks_the_channel_default() {
        let mut config = AppConfig::default();
        config.logging.level = Some(LogLevel::Error);
        assert_eq!(config.logging.effective_level(), LogLevel::Error);
    }

    #[test]
    fn a_stored_level_survives_a_save_and_load_cycle() {
        // An existing installation keeps its level across an upgrade instead of
        // being silently rewritten to the new channel default.
        let paths = PortablePaths::from_root("/tmp/xarchive-logging");
        let _ = fs::remove_file(&paths.config_file);
        let mut config = AppConfig::default();
        config.logging.level = Some(LogLevel::Warning);
        config.save(&paths).expect("save config");

        let (loaded, error) = AppConfig::load(&paths);
        assert_eq!(error, None);
        assert_eq!(loaded.logging.level, Some(LogLevel::Warning));
        assert_eq!(loaded.logging.effective_level(), LogLevel::Warning);
        let _ = fs::remove_file(&paths.config_file);
    }

    #[test]
    fn a_config_without_a_level_follows_the_channel() {
        let paths = PortablePaths::from_root("/tmp/xarchive-logging-absent");
        let _ = fs::remove_file(&paths.config_file);
        let (loaded, error) = AppConfig::load(&paths);
        assert_eq!(error, None);
        assert_eq!(loaded.logging.level, None);
        assert_eq!(
            loaded.logging.effective_level(),
            crate::build_channel::channel_default_log_level()
        );
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
    fn network_defaults_are_bounded_and_follow_the_system_proxy() {
        let network = NetworkConfig::default();
        assert_eq!(network.proxy_mode, ProxyMode::System);
        assert!(network.proxy.is_none());
        assert!(network.normalized_proxy().is_none());
        assert!(network.active_manual_proxy().is_none());
        // The default inherits, so a child must be left alone and the Sidecar
        // is told the mode instead of being handed a value.
        assert_eq!(
            network.sidecar_env(),
            vec![("XARCHIVE_PROXY_MODE".to_owned(), "system".to_owned())]
        );
        assert!(network.sidecar_env_remove().is_empty());
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
            proxy_mode: ProxyMode::Manual,
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
        assert!(diagnostics.contains(r#""proxy_mode":"manual""#));
        assert_eq!(
            network.secrets(),
            vec!["http://alice:s3cret@proxy.example:8080"]
        );
        assert_eq!(
            network
                .sidecar_env()
                .into_iter()
                .filter(|(key, _)| key == "XARCHIVE_PROXY")
                .collect::<Vec<_>>(),
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
        let mut network = NetworkConfig {
            proxy_mode: ProxyMode::Manual,
            ..NetworkConfig::default()
        };
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
        config.network.proxy_mode = ProxyMode::Manual;
        config.network.proxy = Some("ftp://proxy.example:21".to_owned());
        assert!(config.validate().is_ok());
        config.network.transfer_timeout_seconds = 0;
        assert!(config.validate().is_err());
    }

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

    #[test]
    fn a_configuration_without_a_mode_loads_as_manual_and_keeps_its_value() {
        // This is the exact shape written before the mode existed.
        let network =
            load_document("network:\n  proxy: http://alice:s3cret@proxy.example:8080\n").network;
        assert_eq!(network.proxy_mode, ProxyMode::Manual);
        assert_eq!(
            network.active_manual_proxy().as_deref(),
            Some("http://alice:s3cret@proxy.example:8080")
        );
        assert!(network.validate().is_ok());
        assert_eq!(
            network.redacted_proxy().as_deref(),
            Some("http://[REDACTED]@proxy.example:8080")
        );
    }

    #[test]
    fn a_configuration_without_a_proxy_loads_as_system() {
        let network = load_document("network:\n  extraction_timeout_seconds: 120\n").network;
        assert_eq!(network.proxy_mode, ProxyMode::System);
        assert!(network.active_manual_proxy().is_none());
        assert_eq!(network.extraction_timeout_seconds, 120);
        assert!(network.validate().is_ok());
    }

    #[test]
    fn an_explicit_system_mode_is_not_migrated_away_from_a_stored_value() {
        let network = load_document(
            "network:\n  proxy_mode: system\n  proxy: http://alice:s3cret@proxy.example:8080\n",
        )
        .network;
        assert_eq!(
            network.proxy_mode,
            ProxyMode::System,
            "a deliberate system choice must survive the migration"
        );
        assert!(network.active_manual_proxy().is_none());
    }

    #[test]
    fn an_explicit_mode_is_never_overwritten_by_the_migration() {
        for mode in [ProxyMode::System, ProxyMode::Direct, ProxyMode::Manual] {
            let mut network = NetworkConfig {
                proxy_mode: mode,
                proxy_mode_declared: true,
                proxy: Some("http://proxy.example:8080".to_owned()),
                ..NetworkConfig::default()
            };
            network.apply_legacy_migration();
            assert_eq!(
                network.proxy_mode, mode,
                "the migration must not change an explicit mode"
            );
        }
    }

    #[test]
    fn a_pending_telegram_migration_survives_a_configuration_round_trip() {
        let mut telegram = TelegramConfig {
            enabled: true,
            auto_send_on_archive: true,
            chat_id: "-100777".to_owned(),
            ..TelegramConfig::default()
        };
        assert!(!telegram.migration_pending);
        telegram.migration_pending = true;
        let yaml = serde_yaml::to_string(&telegram).expect("serialize");
        let restored: TelegramConfig = serde_yaml::from_str(&yaml).expect("deserialize");
        assert!(restored.migration_pending);
        assert!(
            restored.enabled,
            "the stored intent is preserved for review"
        );
        // A configuration file without the field must not resume sending.
        let legacy: TelegramConfig =
            serde_yaml::from_str("enabled: true\nauto_send_on_archive: true\n").expect("legacy");
        assert!(!legacy.migration_pending);
    }

    #[test]
    fn the_settings_projection_reports_the_pause_without_exposing_a_token() {
        let mut telegram = TelegramConfig {
            enabled: true,
            chat_id: "-100777".to_owned(),
            ..TelegramConfig::default()
        };
        telegram.migration_pending = true;
        let view = serde_json::to_string(&telegram.settings(true)).expect("projection");
        assert!(view.contains("\"migration_pending\":true"));
        assert!(view.contains("\"bot_token_present\":true"), "{view}");
        // Presence is a boolean fact; no secret material may appear.
        assert!(!view.contains("1234:TEST"), "{view}");
        assert!(!view.contains("\"bot_token\":"), "{view}");
    }

    #[test]
    fn only_the_manual_mode_hands_its_value_to_a_client_or_child() {
        let manual = NetworkConfig {
            proxy_mode: ProxyMode::Manual,
            proxy: Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            ..NetworkConfig::default()
        };
        assert!(manual.active_manual_proxy().is_some());

        for mode in [ProxyMode::System, ProxyMode::Direct] {
            let inactive = NetworkConfig {
                proxy_mode: mode,
                proxy: manual.proxy.clone(),
                ..NetworkConfig::default()
            };
            assert!(
                inactive.active_manual_proxy().is_none(),
                "{mode:?} must not use a stored value"
            );
            assert!(
                inactive.redacted_proxy().is_none(),
                "{mode:?} must not report a proxy as active"
            );
            assert!(
                !inactive
                    .sidecar_env()
                    .iter()
                    .any(|(key, _)| key == "XARCHIVE_PROXY"),
                "{mode:?} must not pass a stored value to the Sidecar"
            );
            let diagnostics = inactive.diagnostics();
            assert!(
                diagnostics.proxy_configured,
                "{mode:?} must still report that a value is stored"
            );
            assert!(
                !diagnostics.proxy_active,
                "{mode:?} must report that the stored value is not in effect"
            );
        }
    }

    #[test]
    fn direct_removes_the_inherited_proxy_environment_of_child_processes() {
        let network = NetworkConfig {
            proxy_mode: ProxyMode::Direct,
            proxy: Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            ..NetworkConfig::default()
        };
        assert!(network.validate().is_ok());
        assert_eq!(
            network.sidecar_env(),
            vec![("XARCHIVE_PROXY_MODE".to_owned(), "direct".to_owned())]
        );
        let removed = network.sidecar_env_remove();
        for key in [
            "http_proxy",
            "https_proxy",
            "all_proxy",
            "HTTP_PROXY",
            "HTTPS_PROXY",
        ] {
            assert!(
                removed.iter().any(|name| name == key),
                "Direct must remove {key}, got {removed:?}"
            );
        }
    }

    #[test]
    fn manual_requires_a_value_and_blank_values_do_not_count() {
        let mut network = NetworkConfig {
            proxy_mode: ProxyMode::Manual,
            ..NetworkConfig::default()
        };
        assert!(network.validate().is_err());
        network.proxy = Some("   ".to_owned());
        assert!(network.validate().is_err());
        network.proxy = Some("http://proxy.example:8080".to_owned());
        assert!(network.validate().is_ok());
    }

    #[test]
    fn the_mode_survives_a_save_and_load_round_trip() {
        let paths = crate::portable::PortablePaths::from_root(
            std::env::temp_dir().join(format!("xarchive-config-{}", std::process::id())),
        );
        let _ = std::fs::remove_dir_all(&paths.config_dir);
        for mode in [ProxyMode::System, ProxyMode::Direct, ProxyMode::Manual] {
            let mut config = AppConfig::default();
            config.network.proxy_mode = mode;
            config.network.proxy = Some("http://alice:s3cret@proxy.example:8080".to_owned());
            config.save(&paths).expect("save");

            let (loaded, error) = AppConfig::load(&paths);
            assert_eq!(error, None, "{mode:?} must load without an error");
            assert_eq!(
                loaded.network.proxy_mode, mode,
                "{mode:?} must survive a restart"
            );
            assert_eq!(
                loaded.network.normalized_proxy().as_deref(),
                Some("http://alice:s3cret@proxy.example:8080")
            );
        }
        let _ = std::fs::remove_dir_all(&paths.config_dir);
    }

    #[test]
    fn a_document_without_telegram_keys_loads_disabled() {
        // A configuration written before Telegram settings existed must keep
        // loading, and it must load as disabled (plan TG-01).
        let config: AppConfig =
            serde_yaml::from_str("schema_version: 1\nlogging:\n  max_files: 5\n").expect("legacy");
        assert!(!config.telegram.enabled);
        assert!(!config.telegram.auto_send_on_archive);
        assert_eq!(config.telegram.api_base, "https://api.telegram.org");
        assert!(config.telegram.capability.is_none());
        assert!(config.telegram.validate().is_ok());
        let mut config = config;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn an_enabled_telegram_section_must_be_complete_and_secure() {
        let mut config = AppConfig::default();
        // Enabled without a target is rejected.
        config.telegram.enabled = true;
        assert!(config.validate().is_err());
        config.telegram.chat_id = "-1001234567890".to_owned();
        assert!(config.validate().is_ok());

        // Cloud stays HTTPS-only; the local mode accepts explicit loopback HTTP.
        config.telegram.api_base = "http://api.telegram.org".to_owned();
        assert!(config.validate().is_err());
        config.telegram.endpoint_mode = EndpointMode::Local;
        config.telegram.api_base = "http://127.0.0.1:8081".to_owned();
        assert!(config.validate().is_ok());
        config.telegram.api_base = "http://93.184.216.34:8080".to_owned();
        assert!(config.validate().is_err(), "cleartext remote is refused");
        config.telegram.api_base = "http://127.0.0.1".to_owned();
        assert!(config.validate().is_err(), "a port is required");

        // Timeout bounds are shared with the rest of the network settings.
        config.telegram.api_base = "http://127.0.0.1:8081".to_owned();
        config.telegram.connect_timeout_seconds = 0;
        assert!(config.validate().is_err());
        config.telegram.connect_timeout_seconds = 10;
        config.telegram.upload_processing_timeout_seconds = MAX_NETWORK_TIMEOUT_SECONDS + 1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn a_settings_change_advances_the_revision_and_drops_the_capability_record() {
        let mut telegram = TelegramConfig {
            enabled: true,
            chat_id: "-1001234567890".to_owned(),
            capability: Some(VerifiedTelegramCapability {
                endpoint_mode: EndpointMode::Cloud,
                api_base: "https://api.telegram.org".to_owned(),
                verified_at: "2026-10-01T00:00:00Z".to_owned(),
                server_version: Some("8.0".to_owned()),
                max_upload_bytes: Some(50 * 1024 * 1024),
            }),
            revision: 4,
            ..TelegramConfig::default()
        };
        assert!(telegram.capability_matches());

        // A trailing slash is the same endpoint, not a change.
        telegram.api_base = "https://api.telegram.org/".to_owned();
        assert!(telegram.capability_matches());
        // Switching to the local server invalidates the cloud record.
        telegram.endpoint_mode = EndpointMode::Local;
        assert!(!telegram.capability_matches());
        assert!(!telegram.settings(false).capability_verified);

        telegram.record_settings_change();
        assert_eq!(telegram.revision, 5, "queued items bind to a new revision");
        assert!(telegram.capability.is_none());
    }

    #[test]
    fn telegram_configuration_never_carries_the_bot_token() {
        use xarchive_telegram::MemorySecretStore;
        let telegram = TelegramConfig {
            enabled: true,
            chat_id: "-1001234567890".to_owned(),
            ..TelegramConfig::default()
        };
        let mut store = MemorySecretStore::default();
        assert!(!telegram.bot_token_present(&mut store).expect("absent"));
        store
            .set(TELEGRAM_BOT_TOKEN_KEY, "123:secret")
            .expect("store");
        assert!(telegram.bot_token_present(&mut store).expect("present"));

        // Neither the persisted document nor the frontend projection carries
        // the secret itself.
        let document = serde_yaml::to_string(&telegram).expect("serialize");
        assert!(!document.contains("123:secret"));
        assert!(!document.to_lowercase().contains("token"));
        let projection = telegram.settings(true);
        let payload = serde_json::to_string(&projection).expect("settings json");
        assert!(!payload.contains("123:secret"));
        assert!(payload.contains("\"bot_token_present\":true"));
    }
}
