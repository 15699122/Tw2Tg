//! Telegram Bot API contracts, formatting primitives, and HTTPS transport.
//!
//! The Windows Credential Manager adapter remains outside this crate. The
//! network transport uses blocking reqwest with Rustls and keeps the token out
//! of error messages and debug output.

use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
pub use tokio_util::sync::CancellationToken;

/// One-way SHA-256 over `parts`, hex encoded.
///
/// Used for the bot identity and the request fingerprint. It is a
/// non-reversible fingerprint of high-entropy input, never an encoding of the
/// secret itself.
fn sha256_hex(parts: &[&[u8]]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

pub const TELEGRAM_TEXT_LIMIT: usize = 4096;
pub const TELEGRAM_MEDIA_GROUP_LIMIT: usize = 10;

pub trait SecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError>;
    fn set(&mut self, key: &str, value: &str) -> Result<(), SecretStoreError>;
    fn delete(&mut self, key: &str) -> Result<(), SecretStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretStoreError {
    InvalidKey,
    EmptyValue,
    /// The platform secret store could not be reached (for example the Windows
    /// Credential Manager is unavailable). The payload must describe the
    /// failure, never the secret value.
    Unavailable(String),
    /// The platform secret store refused access (for example a policy or ACL
    /// denial). The payload must describe the failure, never the secret value.
    AccessDenied(String),
}

impl std::fmt::Display for SecretStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKey => formatter.write_str("secret key must not be empty"),
            Self::EmptyValue => formatter.write_str("secret value must not be empty"),
            Self::Unavailable(reason) => write!(formatter, "secret store unavailable: {reason}"),
            Self::AccessDenied(reason) => write!(formatter, "secret store access denied: {reason}"),
        }
    }
}

impl std::error::Error for SecretStoreError {}

#[derive(Default, Clone)]
pub struct MemorySecretStore {
    values: HashMap<String, String>,
}

impl std::fmt::Debug for MemorySecretStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MemorySecretStore")
            .field("keys", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl SecretStore for MemorySecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
        validate_key(key)?;
        Ok(self.values.get(key).cloned())
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        validate_key(key)?;
        if value.is_empty() {
            return Err(SecretStoreError::EmptyValue);
        }
        self.values.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), SecretStoreError> {
        validate_key(key)?;
        self.values.remove(key);
        Ok(())
    }
}

fn validate_key(key: &str) -> Result<(), SecretStoreError> {
    if key.trim().is_empty() {
        Err(SecretStoreError::InvalidKey)
    } else {
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct BotToken(String);

impl std::fmt::Debug for BotToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BotToken")
            .field("value", &"[REDACTED]")
            .finish()
    }
}

impl BotToken {
    pub fn new(value: impl Into<String>) -> Result<Self, SecretStoreError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SecretStoreError::EmptyValue);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A non-reversible fingerprint of this token, safe to store in SQLite
    /// and to use as the bot identity for outbox rows and the `file_id`
    /// cache (plan TG-04/TG-05). One-way SHA-256: the token cannot be
    /// recovered from it, and it changes whenever the token is replaced, so
    /// a token rotation naturally isolates the previous bot's entries.
    pub fn identity(&self) -> String {
        sha256_hex(&[self.0.as_bytes()])
    }
}

impl std::fmt::Display for BotToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub chat_id: String,
    pub text: String,
    pub disable_web_page_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendPhotoRequest {
    pub chat_id: String,
    pub photo: String,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendVideoRequest {
    pub chat_id: String,
    pub video: String,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaGroupItem {
    #[serde(rename = "type")]
    pub media_type: String,
    pub media: String,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMediaGroupRequest {
    pub chat_id: String,
    pub media: Vec<MediaGroupItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramRequest {
    Message(SendMessageRequest),
    Photo(SendPhotoRequest),
    Video(SendVideoRequest),
    MediaGroup(SendMediaGroupRequest),
}

pub trait TelegramTransport {
    fn send(
        &self,
        token: &BotToken,
        request: TelegramRequest,
    ) -> Result<TelegramResponse, TelegramError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramResponse {
    pub ok: bool,
    #[serde(default)]
    pub error_code: Option<i64>,
    #[serde(default)]
    pub description: Option<String>,
    /// Bot API error `parameters` (`retry_after`, migration targets, ...).
    /// The transport retains them so a 429 never collapses to a bare status.
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
}

impl TelegramResponse {
    /// Extracts `result.message_id` from a successful Bot API response.
    pub fn result_message_id(&self) -> Option<String> {
        self.result
            .as_ref()
            .and_then(|result| result.get("message_id"))
            .and_then(serde_json::Value::as_i64)
            .map(|value| value.to_string())
    }

    /// All message ids in the success result, in Telegram's order.
    ///
    /// A single send returns one message object and an album returns an array
    /// of message objects; anything else yields no ids.
    pub fn result_message_ids(&self) -> Vec<String> {
        self.result_items()
            .into_iter()
            .filter_map(|item| {
                item.get("message_id")
                    .and_then(serde_json::Value::as_i64)
                    .map(|value| value.to_string())
            })
            .collect()
    }

    /// Attached file ids in the success result, in Telegram's order.
    ///
    /// Reads `photo.file_id`, `video.file_id` or `document.file_id` from each
    /// returned message, so single media sends and album results share one
    /// extraction path (TG-02: extract message/media/album results instead of
    /// only checking the HTTP status).
    pub fn result_file_ids(&self) -> Vec<String> {
        self.result_items()
            .into_iter()
            .filter_map(|item| {
                ["photo", "video", "document"].into_iter().find_map(|key| {
                    item.get(key)
                        .and_then(|media| media.get("file_id"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
            })
            .collect()
    }

    /// The messages the result carries: one object, or each element of an
    /// album array.
    fn result_items(&self) -> Vec<&serde_json::Value> {
        match &self.result {
            Some(serde_json::Value::Array(items)) => items.iter().collect(),
            Some(value) => vec![value],
            None => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramError {
    InvalidChatId,
    EmptyText,
    InvalidEndpoint,
    /// A cloud endpoint was not HTTPS.
    EndpointNotSecure,
    /// A local endpoint was not an explicit loopback HTTP address.
    EndpointNotLoopback,
    /// The endpoint string could not be parsed or carried userinfo/query/fragment.
    EndpointMalformed,
    /// The upload was cancelled before Telegram confirmed the outcome.
    UploadCancelled,
    /// The connection or DNS lookup for an upload timed out.
    UploadConnectTimeout,
    /// The request body stopped making progress while the file was streaming.
    UploadBodyStalled,
    /// The file finished uploading but the server did not answer in time.
    UploadServerTimeout,
    /// The optional overall deadline for an upload elapsed.
    UploadDeadlineExceeded,
    /// The media file could not be opened or read.
    FileUnreadable(String),
    /// The media file no longer matches the size it was planned with.
    FileChanged(String),
    /// A Telegram response exceeded the size bound.
    ResponseTooLarge,
    /// The upload request carries an invalid file name, MIME type or timeout.
    InvalidUploadRequest(String),
    /// The request reached Telegram but its response was lost or unreadable,
    /// so the outcome is unknown and the send must not be retried blindly
    /// (plan TG-04: "request may have been accepted but the response was lost").
    ResponseLost,
    Transport(String),
    Api {
        code: i64,
        description: String,
        /// Raw Bot API `parameters` object: `retry_after`, migration targets,
        /// usage limits. Retained verbatim so retry policy can read them.
        parameters: Option<serde_json::Value>,
    },
}

impl std::fmt::Display for TelegramError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidChatId => formatter.write_str("Telegram chat_id must not be empty"),
            Self::EmptyText => formatter.write_str("Telegram text must not be empty"),
            Self::InvalidEndpoint => formatter.write_str("Telegram endpoint must use HTTPS"),
            Self::EndpointNotSecure => {
                formatter.write_str("Telegram cloud endpoint must use HTTPS")
            }
            Self::EndpointNotLoopback => formatter
                .write_str("Telegram local endpoint must be an explicit loopback HTTP address"),
            Self::EndpointMalformed => formatter.write_str("Telegram endpoint is malformed"),
            Self::UploadCancelled => {
                formatter.write_str("upload cancelled before the outcome was confirmed")
            }
            Self::UploadConnectTimeout => formatter.write_str("upload connection timed out"),
            Self::UploadBodyStalled => {
                formatter.write_str("upload body stalled while sending the file")
            }
            Self::UploadServerTimeout => formatter.write_str(
                "Telegram did not answer within the server-processing timeout after the upload finished",
            ),
            Self::UploadDeadlineExceeded => {
                formatter.write_str("upload exceeded the overall deadline")
            }
            Self::FileUnreadable(reason) => write!(formatter, "media file is not readable: {reason}"),
            Self::FileChanged(reason) => {
                write!(formatter, "media file changed since it was planned: {reason}")
            }
            Self::ResponseTooLarge => {
                formatter.write_str("Telegram response exceeded the configured size bound")
            }
            Self::InvalidUploadRequest(reason) => {
                write!(formatter, "invalid upload request: {reason}")
            }
            Self::ResponseLost => formatter.write_str(
                "Telegram request was sent but the response was lost or unreadable",
            ),
            Self::Transport(message) => write!(formatter, "Telegram transport error: {message}"),
            Self::Api {
                code,
                description,
                parameters: _,
            } => {
                write!(formatter, "Telegram API error {code}: {description}")
            }
        }
    }
}

impl std::error::Error for TelegramError {}

/// Default Telegram cloud Bot API base URL.
pub const TELEGRAM_CLOUD_API_BASE: &str = "https://api.telegram.org";

/// Which Bot API the transport talks to.
///
/// `Local` exists so an operator-run official `telegram-bot-api --local`
/// server can be used without weakening the cloud HTTPS requirement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointMode {
    /// The public cloud Bot API over HTTPS.
    #[default]
    Cloud,
    /// An operator-run official server on an explicit loopback address.
    Local,
}

impl EndpointMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "cloud" => Some(Self::Cloud),
            "local" => Some(Self::Local),
            _ => None,
        }
    }
}

/// A validated Bot API base URL.
///
/// Construction rejects anything the mode does not allow, so a later request
/// cannot silently carry a token to an unexpected origin. See
/// `docs/development/telegram-local-bot-api-plan.md` TG-01.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramEndpoint {
    base: String,
    mode: EndpointMode,
}

impl TelegramEndpoint {
    /// The Telegram cloud Bot API over HTTPS.
    pub fn cloud() -> Self {
        Self {
            base: TELEGRAM_CLOUD_API_BASE.to_owned(),
            mode: EndpointMode::Cloud,
        }
    }

    /// Validate `base` against `mode` and build an endpoint.
    ///
    /// Cloud requires HTTPS. Local requires HTTP on an explicit loopback host
    /// with a port. Both reject embedded credentials, a query, a fragment and
    /// any path other than `/`, because those turn a token-bearing URL into a
    /// redirect or a different-origin target.
    pub fn parse(mode: EndpointMode, base: &str) -> Result<Self, TelegramError> {
        let trimmed = base.trim().trim_end_matches('/');
        let parsed = reqwest::Url::parse(trimmed).map_err(|_| TelegramError::EndpointMalformed)?;
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err(TelegramError::EndpointMalformed);
        }
        match mode {
            EndpointMode::Cloud => {
                if parsed.scheme() != "https" || parsed.host_str().is_none() {
                    return Err(TelegramError::EndpointNotSecure);
                }
            }
            EndpointMode::Local => {
                if parsed.scheme() != "http" || parsed.port().is_none() {
                    return Err(TelegramError::EndpointNotLoopback);
                }
                let host = parsed
                    .host_str()
                    .ok_or(TelegramError::EndpointNotLoopback)?;
                if !is_loopback_host(host) {
                    return Err(TelegramError::EndpointNotLoopback);
                }
            }
        }
        Ok(Self {
            base: parsed.as_str().trim_end_matches('/').to_owned(),
            mode,
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn mode(&self) -> EndpointMode {
        self.mode
    }

    /// The request URL for `method`, keeping the token out of any log or error
    /// by construction: callers format it at the last moment.
    pub fn method_url(&self, token: &BotToken, method: &str) -> String {
        format!("{}/bot{}/{}", self.base, token.as_str(), method)
    }
}

fn is_loopback_host(host: &str) -> bool {
    let unbracketed = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    if unbracketed.eq_ignore_ascii_case("localhost") {
        return true;
    }
    unbracketed
        .parse::<std::net::IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or(false)
}

/// Delivery state of a persisted Telegram send attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendState {
    Pending,
    Sent,
    Failed,
}

impl SendState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Sent => "SENT",
            Self::Failed => "FAILED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "PENDING" => Some(Self::Pending),
            "SENT" => Some(Self::Sent),
            "FAILED" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Persisted completed send used for idempotency checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentSendRecord {
    pub chat_id: String,
    pub idempotency_key: String,
    pub message_kind: String,
    pub telegram_message_id: String,
    pub telegram_file_id: Option<String>,
}

/// Persisted send that still needs delivery (PENDING or FAILED).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSendRecord {
    pub chat_id: String,
    pub idempotency_key: String,
    pub message_kind: String,
    pub state: SendState,
    pub attempt_count: u32,
}

/// Successful delivery result handed back by the send closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveredSend {
    pub telegram_message_id: String,
    pub telegram_file_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendStateError {
    Store(String),
    NotFound,
    InvalidPayloadPath,
    /// The same logical send identity was queued with a different plan.
    IdempotencyConflict,
    /// The claim token no longer owns the row: the lease expired, another
    /// worker reclaimed it, or the entry reached a terminal state. The
    /// caller must drop its write instead of overwriting newer facts.
    StaleClaim,
}

impl std::fmt::Display for SendStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(message) => write!(formatter, "send state store error: {message}"),
            Self::NotFound => formatter.write_str("send state record not found"),
            Self::InvalidPayloadPath => {
                formatter.write_str("send payload media path must be archive-relative")
            }
            Self::IdempotencyConflict => {
                formatter.write_str("send idempotency key conflicts with a different plan")
            }
            Self::StaleClaim => formatter.write_str("send state claim is stale or expired"),
        }
    }
}

impl std::error::Error for SendStateError {}

/// Persistence contract for Telegram send state. The storage layer
/// implements this on top of SQLite so delivery state survives restarts.
pub trait SendStateStore {
    fn find_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<SentSendRecord>, SendStateError>;

    /// Records (or re-opens) a send attempt; repeated calls for the same
    /// `(chat_id, idempotency_key)` increment the attempt counter.
    fn record_pending(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        message_kind: &str,
        updated_at: &str,
    ) -> Result<(), SendStateError>;

    fn record_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        telegram_message_id: &str,
        telegram_file_id: Option<&str>,
        updated_at: &str,
    ) -> Result<(), SendStateError>;

    fn record_failed(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        error_code: Option<i64>,
        error_message: &str,
        updated_at: &str,
    ) -> Result<(), SendStateError>;

    /// Lists sends that still need delivery (PENDING or FAILED), oldest first.
    fn list_unsent(&self) -> Result<Vec<PendingSendRecord>, SendStateError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotentSendOutcome {
    AlreadySent { telegram_message_id: String },
    Delivered(DeliveredSend),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotentSendError {
    Store(SendStateError),
    Telegram(TelegramError),
}

impl std::fmt::Display for IdempotentSendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "{error}"),
            Self::Telegram(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for IdempotentSendError {}

impl From<SendStateError> for IdempotentSendError {
    fn from(value: SendStateError) -> Self {
        Self::Store(value)
    }
}

impl From<TelegramError> for IdempotentSendError {
    fn from(value: TelegramError) -> Self {
        Self::Telegram(value)
    }
}

/// Sends with idempotent retry semantics: the `deliver` closure runs at most
/// once per `(chat_id, idempotency_key)` even across process restarts. If the
/// send was already completed, `AlreadySent` is returned without invoking the
/// closure; failures are persisted so `list_unsent` can drive later re-send.
pub fn send_idempotently<F>(
    store: &dyn SendStateStore,
    chat_id: &str,
    idempotency_key: &str,
    message_kind: &str,
    updated_at: &str,
    deliver: F,
) -> Result<IdempotentSendOutcome, IdempotentSendError>
where
    F: FnOnce() -> Result<DeliveredSend, TelegramError>,
{
    if let Some(sent) = store.find_sent(chat_id, idempotency_key)? {
        return Ok(IdempotentSendOutcome::AlreadySent {
            telegram_message_id: sent.telegram_message_id,
        });
    }
    store.record_pending(chat_id, idempotency_key, message_kind, updated_at)?;
    match deliver() {
        Ok(delivered) => {
            store.record_sent(
                chat_id,
                idempotency_key,
                &delivered.telegram_message_id,
                delivered.telegram_file_id.as_deref(),
                updated_at,
            )?;
            Ok(IdempotentSendOutcome::Delivered(delivered))
        }
        Err(error) => {
            let (error_code, error_message) = match &error {
                TelegramError::Api {
                    code,
                    description,
                    parameters: _,
                } => (Some(*code), description.clone()),
                other => (None, other.to_string()),
            };
            store.record_failed(
                chat_id,
                idempotency_key,
                error_code,
                &error_message,
                updated_at,
            )?;
            Err(IdempotentSendError::Telegram(error))
        }
    }
}

/// Blocking control-request transport (JSON body, plan TG-01/TG-02).
///
/// Construction and the blocking `send()` must happen **outside a Tokio
/// runtime context**: reqwest 0.13.4 refuses to build its blocking client
/// inside one and panics in debug builds. The async upload path
/// ([`ReqwestTelegramTransport::send_upload`]) has no such restriction — it
/// never touches the blocking client — but the transport *object* still has
/// to be created in synchronous code (or a `spawn_blocking` context) first.
#[derive(Clone)]
pub struct ReqwestTelegramTransport {
    client: reqwest::blocking::Client,
    endpoint: String,
    disable_proxy: bool,
    /// May carry credentials: never formatted, logged or persisted. The
    /// derived-from-nothing `Debug` below reports presence only.
    proxy: Option<String>,
}

impl std::fmt::Debug for ReqwestTelegramTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReqwestTelegramTransport")
            .field("endpoint", &self.endpoint)
            .field("proxy", &self.proxy.as_ref().map(|_| "[REDACTED]"))
            .field("disable_proxy", &self.disable_proxy)
            .finish()
    }
}

impl ReqwestTelegramTransport {
    pub fn new() -> Result<Self, TelegramError> {
        Self::with_endpoint("https://api.telegram.org")
    }

    pub fn with_endpoint(endpoint: impl Into<String>) -> Result<Self, TelegramError> {
        let endpoint = normalize_endpoint(endpoint.into());
        validate_endpoint(&endpoint, false)?;
        Self::build(endpoint, false, std::time::Duration::from_secs(30), None)
    }

    /// Build a transport with the unified network timeout and proxy (P2-A).
    ///
    /// The proxy is applied to the client rather than left to the process
    /// environment, so a configured value actually reaches Telegram. The value
    /// may carry credentials: it is never logged or persisted here.
    pub fn with_network(
        endpoint: impl Into<String>,
        timeout: std::time::Duration,
        disable_proxy: bool,
    ) -> Result<Self, TelegramError> {
        let endpoint = normalize_endpoint(endpoint.into());
        validate_endpoint(&endpoint, false)?;
        Self::build(endpoint, disable_proxy, timeout, None)
    }

    /// Build a transport that routes through an explicit proxy.
    ///
    /// `proxy` is the value the caller resolved for this endpoint. Passing
    /// `Some` pins the route; `None` leaves reqwest's own environment matcher in
    /// place, which is what the `System` mode means.
    pub fn with_resolved_proxy(
        endpoint: impl Into<String>,
        timeout: std::time::Duration,
        proxy: Option<String>,
    ) -> Result<Self, TelegramError> {
        let endpoint = normalize_endpoint(endpoint.into());
        validate_endpoint(&endpoint, false)?;
        Self::build(endpoint, false, timeout, proxy)
    }

    #[doc(hidden)]
    pub fn with_test_endpoint(endpoint: impl Into<String>) -> Result<Self, TelegramError> {
        let endpoint = normalize_endpoint(endpoint.into());
        validate_endpoint(&endpoint, true)?;
        Self::build(endpoint, true, std::time::Duration::from_secs(30), None)
    }

    /// Build a transport from the validated endpoint contract (TG-01).
    ///
    /// This is the production consumer of [`TelegramEndpoint`]: mode-specific
    /// policy (cloud HTTPS, local loopback HTTP with a port) was already
    /// enforced when the endpoint was parsed, and the endpoint's fields are
    /// private, so no raw-string re-check runs here. `EndpointMode::Local`
    /// addresses the operator-run loopback server and always pins the
    /// connection direct — a proxy must never sit between the token and the
    /// local server; `proxy` only applies to cloud endpoints.
    pub fn with_api_endpoint(
        endpoint: TelegramEndpoint,
        timeout: std::time::Duration,
        proxy: Option<String>,
    ) -> Result<Self, TelegramError> {
        let disable_proxy = endpoint.mode() == EndpointMode::Local;
        let proxy = if disable_proxy { None } else { proxy };
        Self::build(endpoint.base().to_owned(), disable_proxy, timeout, proxy)
    }

    fn build(
        endpoint: String,
        disable_proxy: bool,
        timeout: std::time::Duration,
        proxy: Option<String>,
    ) -> Result<Self, TelegramError> {
        // Redirects are disabled: the request URL carries the token, so a 3xx
        // must never forward it to another origin (plan §6 TG-01 policy).
        let mut builder = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none());
        if disable_proxy {
            // `no_proxy()` is the only way to keep a direct promise when the
            // process environment exports proxy variables.
            builder = builder.no_proxy();
        } else if let Some(proxy) = proxy.as_deref() {
            // The value may carry credentials, so it is applied here and never
            // logged, persisted, or placed on a command line.
            let proxy = reqwest::Proxy::all(proxy).map_err(|_| {
                TelegramError::Transport("failed to apply the configured proxy".to_owned())
            })?;
            builder = builder.proxy(proxy);
        }
        let client = builder
            .build()
            .map_err(|_| TelegramError::Transport("failed to build HTTP client".to_owned()))?;
        Ok(Self {
            client,
            endpoint,
            disable_proxy,
            proxy,
        })
    }

    fn method_url(&self, token: &BotToken, method: &str) -> String {
        format!("{}/bot{}/{method}", self.endpoint, token.as_str())
    }
}

impl TelegramTransport for ReqwestTelegramTransport {
    fn send(
        &self,
        token: &BotToken,
        request: TelegramRequest,
    ) -> Result<TelegramResponse, TelegramError> {
        let (method, payload) = request_payload(request)?;
        let mut response = self
            .client
            .post(self.method_url(token, method))
            .json(&payload)
            .send()
            .map_err(map_control_send_error)?;
        let status = response.status();
        // Bound the response size before parsing (TG-02), then parse the JSON
        // ourselves so the bound applies to both paths uniformly.
        if let Some(length) = response.content_length()
            && length > TELEGRAM_MAX_RESPONSE_BYTES
        {
            return Err(TelegramError::ResponseTooLarge);
        }
        use std::io::Read;
        let mut body = Vec::new();
        (&mut response)
            .take(TELEGRAM_MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|_| TelegramError::ResponseLost)?;
        if body.len() as u64 > TELEGRAM_MAX_RESPONSE_BYTES {
            return Err(TelegramError::ResponseTooLarge);
        }
        // The Bot API answers with JSON both for HTTP 200 + `ok: false` and
        // for error statuses (429 with `parameters.retry_after`, 400, ...), so
        // parse first and keep the full error payload instead of collapsing a
        // non-2xx into a bare "HTTP status" transport error (plan TG-04).
        match serde_json::from_slice::<TelegramResponse>(&body) {
            Ok(telegram_response) => {
                if !telegram_response.ok {
                    return Err(TelegramError::Api {
                        code: telegram_response
                            .error_code
                            .unwrap_or_else(|| i64::from(status.as_u16())),
                        description: telegram_response
                            .description
                            .clone()
                            .unwrap_or_else(|| "Telegram API request failed".to_owned()),
                        parameters: telegram_response.parameters.clone(),
                    });
                }
                if !status.is_success() {
                    return Err(TelegramError::Transport(format!(
                        "HTTP status {}",
                        status.as_u16()
                    )));
                }
                Ok(telegram_response)
            }
            Err(_) if status.is_success() => {
                // A 2xx whose body is not the Bot API's JSON: the request may
                // already have been processed, so the outcome is unknown.
                Err(TelegramError::ResponseLost)
            }
            Err(_) => Err(TelegramError::Transport(format!(
                "HTTP status {}",
                status.as_u16()
            ))),
        }
    }
}

/// Maps a control-request send failure. A connection failure never carried
/// the request; any other failure happened after it was handed to the wire,
/// so the caller must treat the outcome as unknown (plan TG-04).
fn map_control_send_error(error: reqwest::Error) -> TelegramError {
    if error.is_connect() || error.is_request() {
        TelegramError::Transport("connection failed".to_owned())
    } else {
        TelegramError::ResponseLost
    }
}

fn normalize_endpoint(endpoint: String) -> String {
    endpoint.trim_end_matches('/').to_owned()
}

fn validate_endpoint(endpoint: &str, test_only: bool) -> Result<(), TelegramError> {
    let parsed = reqwest::Url::parse(endpoint).map_err(|_| TelegramError::InvalidEndpoint)?;
    let allowed = if test_only {
        parsed.scheme() == "http"
            && matches!(parsed.host_str(), Some("127.0.0.1" | "localhost"))
            && parsed.port().is_some()
    } else {
        parsed.scheme() == "https"
            && parsed.host_str().is_some()
            && parsed.username().is_empty()
            && parsed.password().is_none()
    };
    if !allowed || parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(TelegramError::InvalidEndpoint);
    }
    Ok(())
}

fn request_payload(
    request: TelegramRequest,
) -> Result<(&'static str, serde_json::Value), TelegramError> {
    match request {
        TelegramRequest::Message(request) => {
            validate_message(&request)?;
            Ok((
                "sendMessage",
                serde_json::to_value(request).map_err(json_error)?,
            ))
        }
        TelegramRequest::Photo(request) => {
            validate_chat_id(&request.chat_id)?;
            Ok((
                "sendPhoto",
                serde_json::to_value(request).map_err(json_error)?,
            ))
        }
        TelegramRequest::Video(request) => {
            validate_chat_id(&request.chat_id)?;
            Ok((
                "sendVideo",
                serde_json::to_value(request).map_err(json_error)?,
            ))
        }
        TelegramRequest::MediaGroup(request) => {
            validate_chat_id(&request.chat_id)?;
            Ok((
                "sendMediaGroup",
                serde_json::to_value(request).map_err(json_error)?,
            ))
        }
    }
}

fn json_error(error: serde_json::Error) -> TelegramError {
    TelegramError::Transport(error.to_string())
}

fn validate_chat_id(chat_id: &str) -> Result<(), TelegramError> {
    if chat_id.trim().is_empty() {
        Err(TelegramError::InvalidChatId)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataInput<'a> {
    pub tweet_id: &'a str,
    pub url: &'a str,
    pub username: Option<&'a str>,
    pub display_name: Option<&'a str>,
    pub text: &'a str,
    pub tags: &'a [&'a str],
}

pub fn format_metadata(input: &MetadataInput<'_>) -> String {
    let author = match (input.display_name, input.username) {
        (Some(name), Some(username)) => format!("{name} (@{username})"),
        (Some(name), None) => name.to_owned(),
        (None, Some(username)) => format!("@{username}"),
        (None, None) => "Unknown author".to_owned(),
    };
    let mut output = format!(
        "{author}\nhttps://x.com/i/status/{}\n\n{}",
        input.tweet_id,
        input.text.trim()
    );
    if !input.tags.is_empty() {
        output.push_str("\n\n");
        output.push_str(
            &input
                .tags
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    if !input.url.is_empty() && !output.contains(input.url) {
        output.push('\n');
        output.push_str(input.url);
    }
    output
}

pub fn split_text(text: &str, limit: usize) -> Vec<String> {
    if limit == 0 || text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(limit)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

pub fn split_for_telegram(text: &str) -> Vec<String> {
    split_text(text, TELEGRAM_TEXT_LIMIT)
}

/// The Bot API requires an album to hold between 2 and 10 items, so a group of
/// exactly one item is not a valid album and must be sent as a single item.
pub const TELEGRAM_ALBUM_MIN_ITEMS: usize = 2;

/// Caption length unit used by the Bot API.
pub const TELEGRAM_CAPTION_LIMIT: usize = 1024;

/// Documented cloud Bot API upload ceiling for the classic methods.
pub const TELEGRAM_CLOUD_UPLOAD_MAX_BYTES: u64 = 50 * 1024 * 1024;

/// Official server `--local` mode upload ceiling.
///
/// This is a **server capability ceiling**, not a per-media-type guarantee; see
/// `docs/development/telegram-local-bot-api-plan.md` §4.1.
pub const TELEGRAM_LOCAL_UPLOAD_MAX_BYTES: u64 = 2000 * 1024 * 1024;

/// How a media item is classified for delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Photo,
    Video,
    Document,
}

/// Delivery intent for a media item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadMode {
    /// Browse in the client: photo/video/album methods. No byte-identical
    /// download promise.
    #[default]
    Display,
    /// Preserve the exact bytes: a file message, hash-verified on download.
    OriginalFile,
}

/// One unit of a send plan: either an album or a standalone item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaGroupSend {
    Album(Vec<MediaGroupItem>),
    Single(MediaGroupItem),
}

/// Classify a media item for display purposes.
///
/// The declared MIME type wins, with the file extension as a fallback.
/// Anything unrecognised becomes a `Document`, which is the safe choice: it
/// never claims the client can render or play the file inline.
pub fn classify_media(mime_type: Option<&str>, file_name: &str) -> MediaKind {
    let mime = mime_type.unwrap_or("").trim().to_ascii_lowercase();
    // `image/gif` is an animation, not a still photo, so it falls through to
    // `Document` rather than being claimed as a photo.
    if mime.starts_with("image/") && mime != "image/gif" {
        return MediaKind::Photo;
    }
    if mime.starts_with("video/") {
        return MediaKind::Video;
    }
    let extension = normalized_extension(file_name);
    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" => MediaKind::Photo,
        "mp4" | "mov" | "m4v" | "webm" => MediaKind::Video,
        _ => MediaKind::Document,
    }
}

/// Build a deterministic, collision-resistant upload file name (TG-03).
///
/// Shape: `x_<tweet_id>_<media_index>_<hash_prefix>.<ext>`. It is never a
/// generic name such as `video.mp4`, never contains an absolute local path, and
/// never renames the archived file on disk.
pub fn stable_upload_file_name(
    tweet_id: &str,
    media_index: u32,
    content_hash_hex: &str,
    original_file_name: &str,
) -> String {
    let tweet = sanitize_token(tweet_id, 32);
    let mut hash_prefix: String = content_hash_hex
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(12)
        .collect();
    hash_prefix = hash_prefix.to_ascii_lowercase();
    if hash_prefix.is_empty() {
        hash_prefix = "nohash".to_owned();
    }
    let extension = normalized_extension(original_file_name);
    format!("x_{tweet}_{media_index:02}_{hash_prefix}.{extension}")
}

fn normalized_extension(file_name: &str) -> String {
    let raw = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or("");
    let cleaned: String = raw
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect();
    let cleaned = cleaned.to_ascii_lowercase();
    if cleaned.is_empty() {
        "bin".to_owned()
    } else {
        cleaned
    }
}

fn sanitize_token(value: &str, max: usize) -> String {
    let cleaned: String = value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(max)
        .collect();
    if cleaned.is_empty() {
        "unknown".to_owned()
    } else {
        cleaned
    }
}

/// Group items into sendable units.
///
/// [`media_groups`] chunks by the album maximum, which can leave a trailing
/// group of one item. Because an album requires 2–10 items, any such group is
/// promoted to a standalone single send instead of an invalid one-item album.
pub fn media_send_plan(items: Vec<MediaGroupItem>) -> Vec<MediaGroupSend> {
    media_groups(items)
        .into_iter()
        .map(|mut group| {
            if group.len() < TELEGRAM_ALBUM_MIN_ITEMS
                && let Some(single) = group.pop()
            {
                return MediaGroupSend::Single(single);
            }
            MediaGroupSend::Album(group)
        })
        .collect()
}

pub fn media_groups(items: Vec<MediaGroupItem>) -> Vec<Vec<MediaGroupItem>> {
    items
        .chunks(TELEGRAM_MEDIA_GROUP_LIMIT)
        .map(|group| group.to_vec())
        .collect()
}

pub fn validate_message(request: &SendMessageRequest) -> Result<(), TelegramError> {
    validate_chat_id(&request.chat_id)?;
    if request.text.is_empty() {
        return Err(TelegramError::EmptyText);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// TG-02 — localised async streaming upload transport
// ---------------------------------------------------------------------------

/// Upper bound for any Telegram response body.
///
/// Bot API JSON responses are small; the bound keeps a hostile or broken
/// server from exhausting memory (plan TG-02: "Bound the response size").
pub const TELEGRAM_MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

/// Bytes read from disk per body-stream step.
///
/// The upload body is pull-based, so at most one chunk is in flight: memory
/// does not grow with the file size and a whole video is never read at once.
pub const TELEGRAM_UPLOAD_CHUNK_BYTES: usize = 64 * 1024;

/// Layered timeouts for a streaming upload (plan TG-02).
///
/// Each layer covers a different failure mode, so a slow-but-progressing
/// transfer is not killed by the control-request timeout while a dead server
/// is still detected:
///
/// - `connect`: TCP/TLS/DNS connection setup.
/// - `body_stall`: the longest allowed gap with no bytes produced while the
///   file is streaming. Once the body has finished, this layer no longer
///   applies — a slow *server* is covered by `server_processing`.
/// - `server_processing`: the wait from the last body byte being handed over
///   until the response arrives.
/// - `total`: optional overall deadline covering the whole send.
///
/// The transport's control-request timeout (30 s) never governs large sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadTimeouts {
    pub connect: Duration,
    pub body_stall: Duration,
    pub server_processing: Duration,
    pub total: Option<Duration>,
}

impl UploadTimeouts {
    /// Each fixed layer must be positive, so a misconfiguration fails loudly
    /// at validation time instead of firing instantly at runtime.
    fn validate(&self) -> Result<(), TelegramError> {
        if self.connect.is_zero() {
            return Err(TelegramError::InvalidUploadRequest(
                "connect timeout must be greater than zero".to_owned(),
            ));
        }
        if self.body_stall.is_zero() {
            return Err(TelegramError::InvalidUploadRequest(
                "body stall timeout must be greater than zero".to_owned(),
            ));
        }
        if self.server_processing.is_zero() {
            return Err(TelegramError::InvalidUploadRequest(
                "server processing timeout must be greater than zero".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Default for UploadTimeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            body_stall: Duration::from_secs(60),
            server_processing: Duration::from_secs(300),
            total: None,
        }
    }
}

/// One step of an upload, in emission order.
///
/// `Uploading { sent_bytes == total_bytes }` means only that the request body
/// reached the Bot API connection — it is never a "message sent" claim. Only
/// `Confirmed`, which follows a parsed `ok: true` result, means Telegram
/// accepted the message (plan TG-02 progress semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadStage {
    Queued,
    CheckingFile,
    Uploading { sent_bytes: u64, total_bytes: u64 },
    AwaitingResult,
    Confirmed,
}

/// One media file to stream to the Bot API as `multipart/form-data`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadRequest {
    pub chat_id: String,
    /// Selects the method (`sendPhoto`/`sendVideo`/`sendDocument`) and the
    /// multipart field name (`photo`/`video`/`document`).
    pub media_kind: MediaKind,
    pub file_path: PathBuf,
    /// Stable display name from [`stable_upload_file_name`]; never an
    /// absolute local path.
    pub file_name: String,
    pub mime_type: Option<String>,
    pub caption: Option<String>,
    /// Re-verification hint: when present, the file size must still match at
    /// send time. Every call re-opens the file from disk, so a rebuilt
    /// request re-reads and re-verifies it (plan TG-02).
    pub expected_size: Option<u64>,
}

fn validate_upload_request(request: &UploadRequest) -> Result<(), TelegramError> {
    validate_chat_id(&request.chat_id)?;
    if request.file_name.trim().is_empty() {
        return Err(TelegramError::InvalidUploadRequest(
            "file name must not be empty".to_owned(),
        ));
    }
    // The name lands verbatim in a `Content-Disposition` header: quoting,
    // CR/LF and path separators would corrupt or redirect the part.
    if request
        .file_name
        .chars()
        .any(|character| matches!(character, '"' | '\r' | '\n' | '/' | '\\'))
    {
        return Err(TelegramError::InvalidUploadRequest(
            "file name contains unsupported characters".to_owned(),
        ));
    }
    if let Some(mime_type) = request.mime_type.as_deref() {
        let trimmed = mime_type.trim();
        if trimmed.is_empty() || !trimmed.contains('/') || trimmed.contains(['\r', '\n']) {
            return Err(TelegramError::InvalidUploadRequest(
                "content type must be a single-line MIME type".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Bot API method and multipart field name for a media kind.
fn upload_method(media_kind: MediaKind) -> (&'static str, &'static str) {
    match media_kind {
        MediaKind::Photo => ("sendPhoto", "photo"),
        MediaKind::Video => ("sendVideo", "video"),
        MediaKind::Document => ("sendDocument", "document"),
    }
}

/// Body phase used by the watchdog to pick the right layered deadline.
const UPLOAD_PHASE_PRE_BODY: u8 = 0;
const UPLOAD_PHASE_STREAMING: u8 = 1;
const UPLOAD_PHASE_FINISHED: u8 = 2;

/// Watchdog re-check interval. Short enough that a phase transition (for
/// example the body finishing while a long stall deadline is pending) is
/// noticed promptly, and cheap because it only reads two atomics.
const UPLOAD_WATCHDOG_TICK: Duration = Duration::from_millis(25);

/// Progress state shared between the body stream and the watchdog.
/// Bytes accumulated by the shared body progress sink.
struct UploadTracker {
    started: Instant,
    phase: AtomicU8,
    /// Milliseconds since `started` of the last body chunk (or of start).
    last_progress_ms: AtomicU64,
    /// Milliseconds since `started` when the body finished; `u64::MAX` while
    /// the body has not finished.
    finished_at_ms: AtomicU64,
    /// Cumulative body bytes and the expected total, reported as progress.
    bytes_sent: AtomicU64,
    total_bytes: AtomicU64,
}

impl UploadTracker {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            phase: AtomicU8::new(UPLOAD_PHASE_PRE_BODY),
            last_progress_ms: AtomicU64::new(0),
            finished_at_ms: AtomicU64::new(u64::MAX),
            bytes_sent: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
        }
    }

    /// Record the expected body size once it is known (the file was opened
    /// and verified). Only the total changes; no progress is implied.
    fn set_total_bytes(&self, total: u64) {
        self.total_bytes.store(total, Ordering::Release);
    }

    /// The body produced bytes: the stall clock restarts and streaming is
    /// considered started.
    fn record_body_progress(&self, bytes: u64) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.last_progress_ms.store(elapsed, Ordering::Release);
        self.bytes_sent.fetch_add(bytes, Ordering::AcqRel);
        if self.phase.load(Ordering::Acquire) == UPLOAD_PHASE_PRE_BODY {
            // Only the body streams perform this transition, so a benign
            // check-then-set is enough; the atomics provide visibility.
            self.phase.store(UPLOAD_PHASE_STREAMING, Ordering::Release);
        }
    }

    /// The whole body was handed to the transport: the stall rule stops and
    /// the server-processing clock starts.
    fn mark_body_finished(&self) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.finished_at_ms.store(elapsed, Ordering::Release);
        self.phase.store(UPLOAD_PHASE_FINISHED, Ordering::Release);
    }

    fn phase(&self) -> u8 {
        self.phase.load(Ordering::Acquire)
    }

    fn last_progress(&self) -> Duration {
        Duration::from_millis(self.last_progress_ms.load(Ordering::Acquire))
    }

    fn finished_at(&self) -> Duration {
        let value = self.finished_at_ms.load(Ordering::Acquire);
        if value == u64::MAX {
            self.started.elapsed()
        } else {
            Duration::from_millis(value)
        }
    }

    fn snapshot(&self) -> (u64, u64) {
        (
            self.bytes_sent.load(Ordering::Acquire),
            self.total_bytes.load(Ordering::Acquire),
        )
    }
}

/// Receives body progress from the tracked streams: bytes produced, and the
/// end of one item's body. A single-file send reports the end of the whole
/// request; an album reports it per item and finishes when the last one ends.
trait BodyProgressSink: Send + Sync {
    fn on_chunk(&self, bytes: u64);
    fn on_item_finished(&self);
    /// Cumulative `(sent_bytes, total_bytes)` for `UploadStage::Uploading`.
    fn snapshot(&self) -> (u64, u64);
}

impl BodyProgressSink for UploadTracker {
    fn on_chunk(&self, bytes: u64) {
        self.record_body_progress(bytes);
    }

    fn on_item_finished(&self) {
        self.mark_body_finished();
    }

    fn snapshot(&self) -> (u64, u64) {
        UploadTracker::snapshot(self)
    }
}

/// Aggregate progress of an album body: the request is only finished when
/// every item's body has been handed over.
struct AlbumProgress {
    started: Instant,
    total_items: u64,
    total_bytes: u64,
    finished_items: AtomicU64,
    bytes_sent: AtomicU64,
    /// Milliseconds since `started` of the last chunk, and of the last
    /// finished item, so the stall and server-processing rules work exactly as
    /// they do for a single-file body.
    last_progress_ms: AtomicU64,
    last_finish_ms: AtomicU64,
}

impl AlbumProgress {
    fn new(total_items: u64, total_bytes: u64) -> Self {
        Self {
            started: Instant::now(),
            total_items,
            total_bytes,
            finished_items: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            last_progress_ms: AtomicU64::new(0),
            last_finish_ms: AtomicU64::new(u64::MAX),
        }
    }

    fn phase(&self) -> u8 {
        if self.finished_items.load(Ordering::Acquire) >= self.total_items {
            UPLOAD_PHASE_FINISHED
        } else if self.bytes_sent.load(Ordering::Acquire) > 0 {
            UPLOAD_PHASE_STREAMING
        } else {
            UPLOAD_PHASE_PRE_BODY
        }
    }

    fn last_progress(&self) -> Duration {
        Duration::from_millis(self.last_progress_ms.load(Ordering::Acquire))
    }

    fn last_finish(&self) -> Duration {
        let value = self.last_finish_ms.load(Ordering::Acquire);
        if value == u64::MAX {
            self.started.elapsed()
        } else {
            Duration::from_millis(value)
        }
    }
}

impl BodyProgressSink for AlbumProgress {
    fn on_chunk(&self, bytes: u64) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.last_progress_ms.store(elapsed, Ordering::Release);
        self.bytes_sent.fetch_add(bytes, Ordering::AcqRel);
    }

    fn on_item_finished(&self) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.last_finish_ms.store(elapsed, Ordering::Release);
        self.finished_items.fetch_add(1, Ordering::AcqRel);
    }

    fn snapshot(&self) -> (u64, u64) {
        (self.bytes_sent.load(Ordering::Acquire), self.total_bytes)
    }
}

/// The album counterpart of [`upload_watchdog`]: the same layered deadlines,
/// driven by the aggregate album phase.
async fn album_watchdog(progress: Arc<AlbumProgress>, timeouts: UploadTimeouts) -> TelegramError {
    loop {
        let elapsed = progress.started.elapsed();
        let mut wake: Option<Duration> = None;
        match progress.phase() {
            UPLOAD_PHASE_FINISHED => {
                let deadline = progress.last_finish() + timeouts.server_processing;
                if elapsed >= deadline {
                    return TelegramError::UploadServerTimeout;
                }
                wake = Some(deadline);
            }
            UPLOAD_PHASE_STREAMING => {
                let deadline = progress.last_progress() + timeouts.body_stall;
                if elapsed >= deadline {
                    return TelegramError::UploadBodyStalled;
                }
                wake = Some(deadline);
            }
            _ => {}
        }
        if let Some(total) = timeouts.total {
            if elapsed >= total {
                return TelegramError::UploadDeadlineExceeded;
            }
            wake = Some(wake.map_or(total, |existing| existing.min(total)));
        }
        let sleep_for = match wake {
            Some(deadline) => deadline.saturating_sub(elapsed).min(UPLOAD_WATCHDOG_TICK),
            None => UPLOAD_WATCHDOG_TICK,
        };
        tokio::time::sleep(sleep_for).await;
    }
}

/// One item of an album body: either a file to stream, or an already cached
/// `file_id` Telegram can reuse for this bot (plan TG-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlbumItemUpload {
    File(Box<UploadRequest>),
    CachedFileId(String),
}

/// Streams a whole album (`sendMediaGroup`) as `multipart/form-data`.
///
/// Every file part is length-known and pull-based, so an album of videos is
/// never buffered whole. An item whose `file_id` is already cached for this
/// bot is sent as that value instead of being uploaded again.
pub async fn send_media_group_attempt<P>(
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    chat_id: &str,
    items: &[AlbumItemUpload],
    timeouts: &UploadTimeouts,
    cancellation: &CancellationToken,
    on_progress: P,
) -> Result<SendAttemptSuccess, SendAttemptError>
where
    P: FnMut(UploadStage) + Send + 'static,
{
    if chat_id.trim().is_empty() {
        return Err(SendAttemptError::new(
            TelegramError::InvalidChatId,
            RequestProgress::NotSent,
        ));
    }
    if !(TELEGRAM_ALBUM_MIN_ITEMS..=TELEGRAM_MEDIA_GROUP_LIMIT).contains(&items.len()) {
        // One item is a single-media send, and more than the limit is refused
        // before any request is built.
        return Err(SendAttemptError::new(
            TelegramError::InvalidUploadRequest(format!(
                "an album needs {TELEGRAM_ALBUM_MIN_ITEMS}..={TELEGRAM_MEDIA_GROUP_LIMIT} items, got {}",
                items.len()
            )),
            RequestProgress::NotSent,
        ));
    }
    let progress: Arc<Mutex<UploadProgressCallback>> = Arc::new(Mutex::new(Box::new({
        let mut moved = on_progress;
        move |stage: UploadStage| moved(stage)
    })));
    emit_upload_progress(&progress, UploadStage::Queued);
    emit_upload_progress(&progress, UploadStage::CheckingFile);

    // Prepare every file before anything reaches the wire: an album must never
    // be half-sent because a later file turned out to be unreadable.
    let mut prepared: Vec<Option<(tokio::fs::File, UploadRequest, u64)>> =
        Vec::with_capacity(items.len());
    let mut total_bytes = 0_u64;
    for item in items {
        match item {
            AlbumItemUpload::CachedFileId(file_id) => {
                if file_id.trim().is_empty() {
                    return Err(SendAttemptError::new(
                        TelegramError::InvalidUploadRequest(
                            "cached file id must not be empty".to_owned(),
                        ),
                        RequestProgress::NotSent,
                    ));
                }
                prepared.push(None);
            }
            AlbumItemUpload::File(request) => {
                validate_upload_request(request)
                    .map_err(|error| SendAttemptError::new(error, RequestProgress::NotSent))?;
                let file = tokio::fs::File::open(&request.file_path)
                    .await
                    .map_err(|error| {
                        SendAttemptError::new(
                            TelegramError::FileUnreadable(format!(
                                "{}: {error}",
                                request.file_name
                            )),
                            RequestProgress::NotSent,
                        )
                    })?;
                let metadata = file.metadata().await.map_err(|error| {
                    SendAttemptError::new(
                        TelegramError::FileUnreadable(format!("{}: {error}", request.file_name)),
                        RequestProgress::NotSent,
                    )
                })?;
                if metadata.is_dir() {
                    return Err(SendAttemptError::new(
                        TelegramError::FileUnreadable(format!(
                            "{}: path is a directory",
                            request.file_name
                        )),
                        RequestProgress::NotSent,
                    ));
                }
                let size = metadata.len();
                if let Some(expected_size) = request.expected_size
                    && expected_size != size
                {
                    return Err(SendAttemptError::new(
                        TelegramError::FileChanged(format!(
                            "expected {expected_size} bytes, found {size}"
                        )),
                        RequestProgress::NotSent,
                    ));
                }
                total_bytes = total_bytes.saturating_add(size);
                prepared.push(Some((file, (**request).clone(), size)));
            }
        }
    }

    let album_progress = Arc::new(AlbumProgress::new(items.len() as u64, total_bytes));
    let mut form = reqwest::multipart::Form::new().text("chat_id", chat_id.to_owned());
    for (item, entry) in items.iter().zip(prepared.iter()) {
        match (item, entry) {
            (AlbumItemUpload::CachedFileId(file_id), _) => {
                // A cached identifier is sent as a value, not as an upload.
                form = form.text("media", file_id.clone());
            }
            (_, Some((file, request, size))) => {
                let reader = file.try_clone().await.map_err(|error| {
                    SendAttemptError::new(
                        TelegramError::FileUnreadable(format!("{}: {error}", request.file_name)),
                        RequestProgress::NotSent,
                    )
                })?;
                let body = TrackedUploadStream {
                    inner: tokio_util::io::ReaderStream::with_capacity(
                        reader,
                        TELEGRAM_UPLOAD_CHUNK_BYTES,
                    ),
                    progress_sink: album_progress.clone(),
                    progress: Arc::clone(&progress),
                    finished: false,
                };
                let mut part = reqwest::multipart::Part::stream_with_length(
                    reqwest::Body::wrap_stream(body),
                    *size,
                )
                .file_name(request.file_name.clone());
                if let Some(mime_type) = request.mime_type.as_deref() {
                    part = part.mime_str(mime_type).map_err(|_| {
                        SendAttemptError::new(
                            TelegramError::InvalidUploadRequest(
                                "content type could not be parsed".to_owned(),
                            ),
                            RequestProgress::NotSent,
                        )
                    })?;
                }
                form = form.part("media", part);
            }
            _ => {
                return Err(SendAttemptError::new(
                    TelegramError::InvalidUploadRequest("album item was not prepared".to_owned()),
                    RequestProgress::NotSent,
                ));
            }
        }
    }

    let client = transport
        .upload_client(timeouts)
        .map_err(|error| SendAttemptError::new(error, RequestProgress::NotSent))?;
    let url = transport.method_url(token, "sendMediaGroup");
    let upload = async {
        let response = client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(map_upload_send_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(TelegramError::Transport(format!(
                "HTTP status {}",
                status.as_u16()
            )));
        }
        read_bounded_upload_response(response).await
    };
    let outcome: Result<TelegramResponse, TelegramError> = tokio::select! {
        result = async {
            tokio::select! {
                result = upload => result,
                error = album_watchdog(Arc::clone(&album_progress), timeouts.clone()) => Err(error),
            }
        } => result,
        () = cancellation.cancelled() => Err(TelegramError::UploadCancelled),
    };
    // The phase is read after the send, so a failure reports what was really
    // known about the request rather than a guess.
    let phase = album_progress.phase();
    let response = match outcome {
        Ok(response) => {
            emit_upload_progress(&progress, UploadStage::AwaitingResult);
            emit_upload_progress(&progress, UploadStage::Confirmed);
            response
        }
        Err(error) => return Err(SendAttemptError::new(error, progress_from_phase(phase))),
    };

    let message_ids = response.result_message_ids();
    let telegram_message_id = message_ids.first().cloned().ok_or_else(|| {
        SendAttemptError::new(
            TelegramError::Transport(
                "Telegram confirmed the request but returned no message id".to_owned(),
            ),
            RequestProgress::Sent,
        )
    })?;
    Ok(SendAttemptSuccess {
        telegram_message_id,
        results_json: Some(
            serde_json::json!({
                "message_ids": message_ids,
                "file_ids": response.result_file_ids(),
            })
            .to_string(),
        ),
    })
}

/// Waits until the first layered deadline fires and classifies it exactly.
///
/// Cancellation is handled by the caller's `select!`, so this future only
/// ever returns timeout classifications. Before the body starts streaming
/// only the overall deadline applies — connection setup is bounded by the
/// client's connect timeout instead.
async fn upload_watchdog(tracker: Arc<UploadTracker>, timeouts: UploadTimeouts) -> TelegramError {
    loop {
        let elapsed = tracker.started.elapsed();
        let mut wake: Option<Duration> = None;
        match tracker.phase() {
            UPLOAD_PHASE_STREAMING => {
                let deadline = tracker.last_progress() + timeouts.body_stall;
                if elapsed >= deadline {
                    return TelegramError::UploadBodyStalled;
                }
                wake = Some(deadline);
            }
            UPLOAD_PHASE_FINISHED => {
                let deadline = tracker.finished_at() + timeouts.server_processing;
                if elapsed >= deadline {
                    return TelegramError::UploadServerTimeout;
                }
                wake = Some(deadline);
            }
            _ => {}
        }
        if let Some(total) = timeouts.total {
            if elapsed >= total {
                return TelegramError::UploadDeadlineExceeded;
            }
            wake = Some(wake.map_or(total, |existing| existing.min(total)));
        }
        // Never sleep past a phase transition: a long stall deadline must not
        // hide an earlier server-processing or overall deadline.
        let sleep_for = match wake {
            Some(deadline) => deadline.saturating_sub(elapsed).min(UPLOAD_WATCHDOG_TICK),
            None => UPLOAD_WATCHDOG_TICK,
        };
        tokio::time::sleep(sleep_for).await;
    }
}

type UploadProgressCallback = Box<dyn FnMut(UploadStage) + Send>;

fn emit_upload_progress(progress: &Mutex<UploadProgressCallback>, stage: UploadStage) {
    let mut callback = progress.lock().expect("upload progress callback");
    callback(stage);
}

/// Wraps the file reader so the transport observes byte-level progress (the
/// stall clock) and can report the `Uploading`/`AwaitingResult` stages.
///
/// The wrapper is pull-based: exactly one chunk is read per poll, so memory
/// stays bounded regardless of file size.
struct TrackedUploadStream {
    inner: tokio_util::io::ReaderStream<tokio::fs::File>,
    progress_sink: Arc<dyn BodyProgressSink>,
    progress: Arc<Mutex<UploadProgressCallback>>,
    finished: bool,
}

impl Stream for TrackedUploadStream {
    type Item = std::io::Result<bytes::Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(context) {
            Poll::Ready(Some(Ok(bytes))) => {
                this.progress_sink.on_chunk(bytes.len() as u64);
                let (sent_bytes, total_bytes) = this.progress_sink.snapshot();
                emit_upload_progress(
                    &this.progress,
                    UploadStage::Uploading {
                        sent_bytes,
                        total_bytes,
                    },
                );
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => {
                if !this.finished {
                    this.finished = true;
                    this.progress_sink.on_item_finished();
                    emit_upload_progress(&this.progress, UploadStage::AwaitingResult);
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Maps a `send()` failure without ever formatting the `reqwest::Error`
/// itself, because its `Display` includes the token-bearing URL.
fn map_upload_send_error(error: reqwest::Error) -> TelegramError {
    if error.is_timeout() {
        // The upload client only configures a connect timeout, so a reqwest
        // timeout here is connection setup rather than the transfer.
        TelegramError::UploadConnectTimeout
    } else if error.is_body() {
        // The media stream failed mid-flight (file removed or truncated
        // after it was verified).
        TelegramError::FileUnreadable("media stream ended early".to_owned())
    } else {
        TelegramError::Transport("upload request failed".to_owned())
    }
}

/// Reads a Telegram response under the size bound, then parses and checks it
/// (`ok: true`, not just the HTTP status).
async fn read_bounded_upload_response(
    mut response: reqwest::Response,
) -> Result<TelegramResponse, TelegramError> {
    if let Some(length) = response.content_length()
        && length > TELEGRAM_MAX_RESPONSE_BYTES
    {
        return Err(TelegramError::ResponseTooLarge);
    }
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| TelegramError::Transport("upload response read failed".to_owned()))?
    {
        if body.len() as u64 + chunk.len() as u64 > TELEGRAM_MAX_RESPONSE_BYTES {
            return Err(TelegramError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    let parsed: TelegramResponse = serde_json::from_slice(&body)
        .map_err(|_| TelegramError::Transport("invalid Telegram JSON response".to_owned()))?;
    if !parsed.ok {
        return Err(TelegramError::Api {
            code: parsed.error_code.unwrap_or(0),
            description: parsed
                .description
                .unwrap_or_else(|| "Telegram API request failed".to_owned()),
            parameters: parsed.parameters,
        });
    }
    Ok(parsed)
}

/// Maps a body tracker phase to what is known about the request when it ends.
///
/// This is the only honest source for that value: guessing it would either
/// re-send an accepted message or treat an unsent one as lost (plan TG-04).
fn progress_from_phase(phase: u8) -> RequestProgress {
    match phase {
        UPLOAD_PHASE_FINISHED => RequestProgress::Sent,
        UPLOAD_PHASE_STREAMING => RequestProgress::Partial,
        _ => RequestProgress::NotSent,
    }
}

impl ReqwestTelegramTransport {
    /// The async client used for uploads. Built per upload so the layered
    /// connect timeout belongs to this send, not to the control-request
    /// client. Redirects are disabled: the URL carries the token.
    fn upload_client(&self, timeouts: &UploadTimeouts) -> Result<reqwest::Client, TelegramError> {
        let mut builder = reqwest::Client::builder()
            .connect_timeout(timeouts.connect)
            .redirect(reqwest::redirect::Policy::none());
        if self.disable_proxy {
            builder = builder.no_proxy();
        } else if let Some(proxy) = self.proxy.as_deref() {
            let proxy = reqwest::Proxy::all(proxy).map_err(|_| {
                TelegramError::Transport("failed to apply the configured proxy".to_owned())
            })?;
            builder = builder.proxy(proxy);
        }
        builder
            .build()
            .map_err(|_| TelegramError::Transport("failed to build HTTP client".to_owned()))
    }

    /// Streams one media file to the Bot API over `multipart/form-data`
    /// (plan TG-02: localised async upload transport).
    ///
    /// The file is re-opened from disk, verified against `expected_size`,
    /// read in bounded chunks and handed to reqwest as a length-known part;
    /// it is never read into memory as a whole. The caller races this future
    /// against `cancellation` with `tokio::select!`; the future itself
    /// settles `UploadCancelled` when the token fires, and a request that was
    /// already sent must not be auto-retried — the caller reports `UNKNOWN`
    /// instead (plan TG-02 cancellation semantics).
    ///
    /// `on_progress` receives ordered [`UploadStage`] events. `Confirmed` is
    /// emitted only after a parsed `ok: true` result, so "uploaded" is never
    /// presented as "sent" before Telegram confirms it.
    async fn send_upload_outcome<P>(
        &self,
        token: &BotToken,
        request: &UploadRequest,
        timeouts: &UploadTimeouts,
        cancellation: &CancellationToken,
        mut on_progress: P,
    ) -> (Result<TelegramResponse, TelegramError>, RequestProgress)
    where
        P: FnMut(UploadStage) + Send + 'static,
    {
        // The tracker lives outside the send so the caller can learn what is
        // known about the request once the attempt ends.
        let tracker = Arc::new(UploadTracker::new());
        let outcome: Result<TelegramResponse, TelegramError> = async {
        on_progress(UploadStage::Queued);
        validate_upload_request(request)?;
        timeouts.validate()?;
        if cancellation.is_cancelled() {
            return Err(TelegramError::UploadCancelled);
        }

        on_progress(UploadStage::CheckingFile);
        let file = tokio::fs::File::open(&request.file_path)
            .await
            .map_err(|error| {
                TelegramError::FileUnreadable(format!("{}: {error}", request.file_name))
            })?;
        let metadata = file.metadata().await.map_err(|error| {
            TelegramError::FileUnreadable(format!("{}: {error}", request.file_name))
        })?;
        if metadata.is_dir() {
            return Err(TelegramError::FileUnreadable(format!(
                "{}: path is a directory",
                request.file_name
            )));
        }
        let total_bytes = metadata.len();
        if let Some(expected_size) = request.expected_size
            && expected_size != total_bytes
        {
            return Err(TelegramError::FileChanged(format!(
                "expected {expected_size} bytes, found {total_bytes}"
            )));
        }

        let (method, field) = upload_method(request.media_kind);
        tracker.set_total_bytes(total_bytes);
        // The caller's callback is moved behind the shared handle the body
        // stream and the final `Confirmed` event both use.
        let progress: Arc<Mutex<UploadProgressCallback>> = Arc::new(Mutex::new(Box::new({
            let mut moved = on_progress;
            move |stage: UploadStage| moved(stage)
        })));
        let body = TrackedUploadStream {
            inner: tokio_util::io::ReaderStream::with_capacity(file, TELEGRAM_UPLOAD_CHUNK_BYTES),
            progress_sink: tracker.clone(),
            progress: Arc::clone(&progress),
            finished: false,
        };
        let mut part = reqwest::multipart::Part::stream_with_length(
            reqwest::Body::wrap_stream(body),
            total_bytes,
        )
        .file_name(request.file_name.clone());
        if let Some(mime_type) = request.mime_type.as_deref() {
            part = part.mime_str(mime_type).map_err(|_| {
                TelegramError::InvalidUploadRequest("content type could not be parsed".to_owned())
            })?;
        }
        let mut form = reqwest::multipart::Form::new()
            .text("chat_id", request.chat_id.clone())
            .part(field, part);
        if let Some(caption) = request.caption.as_deref() {
            form = form.text("caption", caption.to_owned());
        }

        let client = self.upload_client(timeouts)?;
        let url = self.method_url(token, method);
        let upload = async {
            let response = client
                .post(url)
                .multipart(form)
                .send()
                .await
                .map_err(map_upload_send_error)?;
            let status = response.status();
            if !status.is_success() {
                return Err(TelegramError::Transport(format!(
                    "HTTP status {}",
                    status.as_u16()
                )));
            }
            read_bounded_upload_response(response).await
        };
        // Layers: the send itself, the watchdog's classification of the first
        // deadline, and the caller's cancellation token.
        let outcome: Result<TelegramResponse, TelegramError> = tokio::select! {
            result = async {
                tokio::select! {
                    result = upload => result,
                    error = upload_watchdog(Arc::clone(&tracker), timeouts.clone()) => Err(error),
                }
            } => result,
            () = cancellation.cancelled() => Err(TelegramError::UploadCancelled),
        };
        match outcome {
                Ok(response) => {
                    emit_upload_progress(&progress, UploadStage::Confirmed);
                    Ok(response)
                }
                Err(error) => Err(error),
            }
        }
        .await;
        (outcome, progress_from_phase(tracker.phase()))
    }

    /// Streams one media file to the Bot API over `multipart/form-data`
    /// (plan TG-02) and returns the parsed response.
    ///
    /// Use [`ReqwestTelegramTransport::send_upload_attempt`] when the send is
    /// driven by the outbox: that variant also reports what is known about the
    /// request when it fails.
    pub async fn send_upload<P>(
        &self,
        token: &BotToken,
        request: &UploadRequest,
        timeouts: &UploadTimeouts,
        cancellation: &CancellationToken,
        on_progress: P,
    ) -> Result<TelegramResponse, TelegramError>
    where
        P: FnMut(UploadStage) + Send + 'static,
    {
        self.send_upload_outcome(token, request, timeouts, cancellation, on_progress)
            .await
            .0
    }

    /// One media attempt as the outbox driver needs it.
    ///
    /// On success the confirmed message id (and any per-item file ids) are
    /// extracted; on failure the request progress comes from the body tracker,
    /// so [`run_claimed_attempt`] never has to guess it.
    pub async fn send_upload_attempt<P>(
        &self,
        token: &BotToken,
        request: &UploadRequest,
        timeouts: &UploadTimeouts,
        cancellation: &CancellationToken,
        on_progress: P,
    ) -> Result<SendAttemptSuccess, SendAttemptError>
    where
        P: FnMut(UploadStage) + Send + 'static,
    {
        let (outcome, progress) = self
            .send_upload_outcome(token, request, timeouts, cancellation, on_progress)
            .await;
        let response = outcome.map_err(|error| SendAttemptError::new(error, progress))?;
        let telegram_message_id = response.result_message_id().ok_or_else(|| {
            // `ok: true` without a message id cannot confirm which message was
            // created, so the outcome stays unknown instead of being guessed.
            SendAttemptError::new(
                TelegramError::Transport(
                    "Telegram confirmed the request but returned no message id".to_owned(),
                ),
                RequestProgress::Sent,
            )
        })?;
        let file_ids = response.result_file_ids();
        let results_json = if file_ids.is_empty() {
            None
        } else {
            Some(
                serde_json::json!({
                    "message_ids": response.result_message_ids(),
                    "file_ids": file_ids,
                })
                .to_string(),
            )
        };
        Ok(SendAttemptSuccess {
            telegram_message_id,
            results_json,
        })
    }
}

// ---------------------------------------------------------------------------
// TG-04 — outbox state machine, atomic claim and retry contract
// ---------------------------------------------------------------------------

/// Persisted lifecycle of one outbox entry (plan TG-04).
///
/// ```text
/// QUEUED → IN_FLIGHT → SENT
///                   ├─ RETRY_WAIT
///                   ├─ FAILED_PERMANENT
///                   └─ UNKNOWN
/// QUEUED → CANCELLED
/// ```
///
/// `UNKNOWN` means the request may have been accepted while the response was
/// lost. It is never re-sent automatically: the operator reviews it first
/// (local idempotency does not equal remote exactly-once).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutboxState {
    Queued,
    InFlight,
    Sent,
    RetryWait,
    FailedPermanent,
    Unknown,
    Cancelled,
}

impl OutboxState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::InFlight => "IN_FLIGHT",
            Self::Sent => "SENT",
            Self::RetryWait => "RETRY_WAIT",
            Self::FailedPermanent => "FAILED_PERMANENT",
            Self::Unknown => "UNKNOWN",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "QUEUED" => Some(Self::Queued),
            "IN_FLIGHT" => Some(Self::InFlight),
            "SENT" => Some(Self::Sent),
            "RETRY_WAIT" => Some(Self::RetryWait),
            "FAILED_PERMANENT" => Some(Self::FailedPermanent),
            "UNKNOWN" => Some(Self::Unknown),
            "CANCELLED" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// States a sender may pick up automatically. `UNKNOWN` is excluded on
    /// purpose: it requires human review before any re-send.
    pub fn auto_sendable(self) -> bool {
        matches!(self, Self::Queued | Self::RetryWait)
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Sent | Self::FailedPermanent | Self::Cancelled)
    }
}

/// One outbox row as the shared contract sees it (plan TG-04: bot identity,
/// target/topic, archive reference, plan position, claim, retry, results,
/// redacted error and the `UNKNOWN` reason all travel together).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxEntry {
    pub id: i64,
    pub bot_identity: String,
    /// Explicit target scope, independent of nullable topic SQL semantics.
    pub target_scope: String,
    pub chat_id: String,
    pub message_thread_id: Option<i64>,
    pub idempotency_key: String,
    /// Fingerprint of the planned request, so a changed plan never silently
    /// reuses a result recorded for different content.
    pub request_fingerprint: String,
    pub message_kind: String,
    /// Settings version this entry was queued under; later configuration
    /// edits affect only new items (plan TG-06).
    pub config_version: i64,
    pub plan_version: i64,
    pub plan_order: i64,
    pub tweet_id: Option<i64>,
    pub media_reference: Option<String>,
    pub content_sha256: Option<String>,
    /// Versioned, immutable serialized SendPayload. NULL is reserved for
    /// historical rows that cannot be safely reconstructed.
    pub payload_schema_version: Option<i64>,
    pub payload_json: Option<String>,
    pub state: OutboxState,
    pub attempt_count: u32,
    pub claim_token: Option<String>,
    pub claim_expires_at: Option<String>,
    pub request_started: bool,
    pub next_retry_at: Option<String>,
    pub telegram_message_id: Option<String>,
    /// Per-item results for albums: JSON array of `{message_id, file_id}`.
    pub results_json: Option<String>,
    pub last_error_code: Option<i64>,
    /// Already redacted by the caller; must never carry a token or URL with
    /// credentials.
    pub last_error_message: Option<String>,
    pub unknown_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Values supplied when queueing an entry; the store assigns the id and the
/// initial `QUEUED` state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOutboxEntry {
    pub bot_identity: String,
    pub target_scope: String,
    pub chat_id: String,
    pub message_thread_id: Option<i64>,
    pub idempotency_key: String,
    pub request_fingerprint: String,
    pub message_kind: String,
    pub config_version: i64,
    pub plan_version: i64,
    pub plan_order: i64,
    pub tweet_id: Option<i64>,
    pub media_reference: Option<String>,
    pub content_sha256: Option<String>,
    pub payload_schema_version: i64,
    pub payload_json: String,
    pub created_at: String,
}

/// Persistence contract for the Telegram outbox (plan TG-04).
///
/// Claim semantics: `claim_*` atomically moves one due entry to `IN_FLIGHT`
/// under a caller-chosen `claim_token` and a lease. Every later transition
/// must present the same token, so a worker whose lease expired can never
/// overwrite facts recorded by the worker that reclaimed the entry.
/// `recover_outbox_claims` resolves crashed claims: an entry whose request
/// never started goes back to `RETRY_WAIT`, one whose request started
/// becomes `UNKNOWN` (never an automatic re-send).
pub trait TelegramOutboxStore {
    /// Queue an entry. Idempotent on `(bot_identity, target_scope,
    /// idempotency_key)` only when the fingerprint matches. A changed plan
    /// under the same logical identity is rejected.
    fn enqueue_outbox(&self, entry: NewOutboxEntry) -> Result<i64, SendStateError>;

    /// Atomically claim the oldest due entry for `bot_identity`
    /// (`QUEUED`/`RETRY_WAIT`, `next_retry_at` elapsed or unset, no active
    /// lease). Returns `None` when nothing is due.
    fn claim_due_outbox(
        &self,
        bot_identity: &str,
        claim_token: &str,
        now: &str,
        lease_until: &str,
    ) -> Result<Option<OutboxEntry>, SendStateError>;

    /// Atomically claim one specific entry. This is the deliberate re-send
    /// path: it also accepts an `UNKNOWN` entry (the operator reviewed a
    /// possible duplicate) and a `RETRY_WAIT` entry that is not due yet.
    /// Entries of a different bot identity, or in a terminal state, yield
    /// `None`.
    fn claim_outbox(
        &self,
        bot_identity: &str,
        idempotency_key: &str,
        claim_token: &str,
        now: &str,
        lease_until: &str,
    ) -> Result<Option<OutboxEntry>, SendStateError>;

    /// Fencing point immediately before the network attempt: from here a
    /// lease expiry can only resolve to `UNKNOWN`, never to a re-send.
    fn mark_request_started(&self, claim_token: &str, now: &str) -> Result<(), SendStateError>;

    fn record_outbox_sent(
        &self,
        claim_token: &str,
        telegram_message_id: &str,
        results_json: Option<&str>,
        now: &str,
    ) -> Result<(), SendStateError>;

    /// Schedule the next attempt. `next_retry_at` must already encode the
    /// retry policy (bounded backoff or the server's `retry_after`).
    fn record_outbox_retry(
        &self,
        claim_token: &str,
        next_retry_at: &str,
        error_code: Option<i64>,
        error_message: &str,
        now: &str,
    ) -> Result<(), SendStateError>;

    fn record_outbox_unknown(
        &self,
        claim_token: &str,
        reason: &str,
        error_code: Option<i64>,
        error_message: &str,
        now: &str,
    ) -> Result<(), SendStateError>;

    fn record_outbox_failed(
        &self,
        claim_token: &str,
        error_code: Option<i64>,
        error_message: &str,
        now: &str,
    ) -> Result<(), SendStateError>;

    /// Record a clean cancellation of a claimed entry: the request never
    /// started, so nothing was sent and no uncertainty is recorded
    /// (plan TG-04).
    fn record_outbox_cancelled(&self, claim_token: &str, now: &str) -> Result<(), SendStateError>;

    /// Cancel a not-yet-sent entry (`QUEUED` or `RETRY_WAIT`) for this bot.
    /// Returns `false` when the entry does not exist or is already claimed
    /// or terminal — cancelling an `IN_FLIGHT` entry after the request was
    /// sent must go through the `UNKNOWN` path instead (plan TG-04).
    fn cancel_outbox(
        &self,
        bot_identity: &str,
        idempotency_key: &str,
        now: &str,
    ) -> Result<bool, SendStateError>;

    /// Crash recovery: resolve every expired claim. Returns how many
    /// entries were resolved (to `RETRY_WAIT` if the request never
    /// started, otherwise to `UNKNOWN` with `claim_lease_expired`).
    fn recover_outbox_claims(&self, now: &str) -> Result<u64, SendStateError>;

    /// Entries this bot may send now: `QUEUED`/`RETRY_WAIT`, due, ordered by
    /// plan position. Never includes `UNKNOWN`.
    fn list_due_outbox(
        &self,
        bot_identity: &str,
        now: &str,
    ) -> Result<Vec<OutboxEntry>, SendStateError>;
}

// ---------------------------------------------------------------------------
// TG-04 — error classification and retry policy
// ---------------------------------------------------------------------------

/// What is known about the request when a failure happened. The transport
/// determines this from its own phase tracking; the classifier never guesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestProgress {
    /// Nothing reached the wire: safe to retry after backoff.
    NotSent,
    /// Body bytes were written but the request never completed. The server
    /// discards an incomplete multipart body, so a retry is still safe.
    Partial,
    /// The complete request was handed over: the outcome may be `UNKNOWN`.
    Sent,
}

/// Retry-policy decision for a failed send (plan TG-04 table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendFailure {
    /// Retry with the server-supplied `retry_after`, or with bounded
    /// backoff when it is absent.
    Retry { retry_after: Option<Duration> },
    /// Auth/permission failure or an invalid target: stop until the user
    /// corrects the configuration.
    Permanent,
    /// Explicit media-parameter error: correct the input or apply a
    /// deterministic fallback; never a blind retry.
    MediaCorrection,
    /// The request may have been accepted: record `UNKNOWN` and never
    /// re-send automatically.
    Unknown,
    /// The user cancelled. `after_send` records the uncertainty that must
    /// accompany a cancel that raced an already-sent request.
    Cancelled { after_send: bool },
}

/// Fallback when Telegram answers 429 without a `retry_after` parameter.
pub const TELEGRAM_DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(30);

const BACKOFF_BASE: Duration = Duration::from_secs(5);
const BACKOFF_MAX: Duration = Duration::from_secs(30 * 60);

/// Deterministic bounded backoff: 5 s doubled per attempt, capped at
/// 30 minutes. Used for connection failures before the request was sent.
pub fn bounded_backoff(attempt: u32) -> Duration {
    let exponent = attempt.min(10);
    BACKOFF_BASE
        .saturating_mul(1_u32 << exponent)
        .min(BACKOFF_MAX)
}

impl TelegramError {
    /// The server-supplied `retry_after` carried in an API error's
    /// `parameters`, if present (plan TG-04: retain `parameters` instead of
    /// collapsing a 429 to a bare status).
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Api {
                parameters: Some(parameters),
                ..
            } => parameters
                .get("retry_after")
                .and_then(serde_json::Value::as_u64)
                .map(Duration::from_secs),
            _ => None,
        }
    }
}

/// Classifies a send failure into the plan's retry-policy rows.
///
/// `progress` comes from the transport's own phase tracking; supplying the
/// wrong progress invalidates the result, so callers must derive it rather
/// than assume.
pub fn classify_send_failure(error: &TelegramError, progress: RequestProgress) -> SendFailure {
    match error {
        TelegramError::Api {
            code,
            description,
            parameters: _,
        } => match *code {
            429 => SendFailure::Retry {
                retry_after: error.retry_after().or(Some(TELEGRAM_DEFAULT_RETRY_AFTER)),
            },
            401 | 403 => SendFailure::Permanent,
            400 | 404 if is_target_error(description) => SendFailure::Permanent,
            400..=499 => SendFailure::MediaCorrection,
            // 5xx and anything else: bounded backoff.
            _ => SendFailure::Retry { retry_after: None },
        },
        TelegramError::UploadCancelled => SendFailure::Cancelled {
            after_send: progress == RequestProgress::Sent,
        },
        TelegramError::ResponseLost
        | TelegramError::UploadServerTimeout
        | TelegramError::ResponseTooLarge => SendFailure::Unknown,
        TelegramError::UploadDeadlineExceeded => match progress {
            RequestProgress::Sent => SendFailure::Unknown,
            RequestProgress::NotSent | RequestProgress::Partial => {
                SendFailure::Retry { retry_after: None }
            }
        },
        // Configuration faults never resolve by retrying.
        TelegramError::InvalidChatId
        | TelegramError::EmptyText
        | TelegramError::InvalidEndpoint
        | TelegramError::EndpointNotSecure
        | TelegramError::EndpointNotLoopback
        | TelegramError::EndpointMalformed => SendFailure::Permanent,
        // Input-side faults need correction before another attempt.
        TelegramError::FileUnreadable(_)
        | TelegramError::FileChanged(_)
        | TelegramError::InvalidUploadRequest(_) => SendFailure::MediaCorrection,
        // Everything else is transport-level: before (or without) a complete
        // request it retries with bounded backoff; once the full request was
        // sent it becomes UNKNOWN.
        _ => match progress {
            RequestProgress::NotSent | RequestProgress::Partial => {
                SendFailure::Retry { retry_after: None }
            }
            RequestProgress::Sent => SendFailure::Unknown,
        },
    }
}

/// True for errors that say the configured target itself is unusable
/// (chat/peer/user lookup failed), as opposed to a media-parameter problem.
fn is_target_error(description: &str) -> bool {
    let description = description.to_ascii_lowercase();
    ["chat", "channel", "peer", "user", "group"]
        .iter()
        .any(|needle| description.contains(needle))
}

/// `UNKNOWN` reason recorded when the request was fully sent but its response
/// was lost. Persisted so the review UI can explain the uncertainty.
pub const UNKNOWN_REASON_RESPONSE_LOST: &str = "response_lost";

/// `UNKNOWN` reason recorded when a user cancellation raced a request that
/// was already on the wire: the intent is kept, remote undo is never promised.
pub const UNKNOWN_REASON_CANCELLED_AFTER_SEND: &str = "cancelled_after_send";

/// `UNKNOWN` reason recorded by lease recovery when the crash happened after
/// the request had started (see `recover_outbox_claims`).
pub const UNKNOWN_REASON_LEASE_EXPIRED: &str = "claim_lease_expired";

/// The durable transition a failed attempt must be recorded as.
///
/// Keeping this decision in the shared contract means the sender loop only
/// performs I/O and then persists what the contract decided; it cannot invent
/// its own retry semantics (plan TG-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxDecision {
    /// Schedule another attempt after this delay (bounded backoff when the
    /// server did not name one).
    RetryAfter(Duration),
    /// Record `UNKNOWN` with this reason.
    Unknown(&'static str),
    /// Record `FAILED_PERMANENT`.
    FailedPermanent,
    /// Record a clean `CANCELLED`: nothing was sent.
    Cancelled,
    /// Record `CANCELLED` with the media/plan needing correction first.
    NeedsMediaCorrection,
}

/// Maps a classified failure plus the attempt number onto the next durable
/// transition. `attempt` is 0-based and only feeds the bounded backoff.
pub fn decide_outbox_transition(failure: &SendFailure, attempt: u32) -> OutboxDecision {
    match failure {
        SendFailure::Retry { retry_after } => {
            OutboxDecision::RetryAfter(retry_after.unwrap_or_else(|| bounded_backoff(attempt)))
        }
        SendFailure::Unknown => OutboxDecision::Unknown(UNKNOWN_REASON_RESPONSE_LOST),
        SendFailure::Cancelled { after_send: true } => {
            OutboxDecision::Unknown(UNKNOWN_REASON_CANCELLED_AFTER_SEND)
        }
        SendFailure::Cancelled { after_send: false } => OutboxDecision::Cancelled,
        SendFailure::Permanent => OutboxDecision::FailedPermanent,
        SendFailure::MediaCorrection => OutboxDecision::NeedsMediaCorrection,
    }
}

// ---------------------------------------------------------------------------
// TG-04 / TG-06 — attempt driver and send plans
// ---------------------------------------------------------------------------

/// What one confirmed attempt achieved, as the durable layer needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendAttemptSuccess {
    pub telegram_message_id: String,
    /// Per-item results for an album: JSON with `message_ids` and `file_ids`.
    pub results_json: Option<String>,
}

/// A failed attempt plus what is known about the request.
///
/// `progress` always comes from the transport's own phase tracking: it decides
/// whether another attempt is safe at all, so it must never be guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendAttemptError {
    pub error: TelegramError,
    pub progress: RequestProgress,
}

impl SendAttemptError {
    pub fn new(error: TelegramError, progress: RequestProgress) -> Self {
        Self { error, progress }
    }

    /// The retry-policy class of this failure.
    pub fn classification(&self) -> SendFailure {
        classify_send_failure(&self.error, self.progress)
    }
}

/// Why an attempt driver stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunAttemptError {
    /// The claim was lost (lease expired or another worker owns the row);
    /// nothing was written and the caller's result must be discarded.
    StaleClaim,
    /// The durable store failed while writing the transition.
    Store(SendStateError),
    /// The attempt failed. `classification` and `transition` are what was
    /// decided and persisted; `message` is the redacted text that was stored.
    Failed {
        classification: SendFailure,
        transition: OutboxDecision,
        message: String,
    },
}

/// Runs one claimed attempt end to end and persists its outcome.
///
/// The protocol is fixed here so no caller can take a shortcut (plan TG-04):
/// fence first (`mark_request_started`), then perform the I/O, then let the
/// shared classification choose the durable transition. A lost claim surfaces
/// as [`RunAttemptError::StaleClaim`] and never overwrites a newer row.
///
/// `schedule` turns a retry delay into the timestamp format the store uses; the
/// caller owns its clock, so this crate stays free of date formatting.
pub async fn run_claimed_attempt<F, S>(
    store: &dyn TelegramOutboxStore,
    claim_token: &str,
    attempt: u32,
    now: &str,
    schedule: S,
    execute: F,
) -> Result<SendAttemptSuccess, RunAttemptError>
where
    F: std::future::Future<Output = Result<SendAttemptSuccess, SendAttemptError>>,
    S: FnOnce(Duration) -> String,
{
    store
        .mark_request_started(claim_token, now)
        .map_err(store_error)?;
    let failure = match execute.await {
        Ok(success) => {
            store
                .record_outbox_sent(
                    claim_token,
                    &success.telegram_message_id,
                    success.results_json.as_deref(),
                    now,
                )
                .map_err(store_error)?;
            return Ok(success);
        }
        Err(failure) => failure,
    };

    let classification = failure.classification();
    let transition = decide_outbox_transition(&classification, attempt);
    let message = failure.error.to_string();
    let error_code = match &failure.error {
        TelegramError::Api { code, .. } => Some(*code),
        _ => None,
    };
    let written = match &transition {
        OutboxDecision::RetryAfter(delay) => {
            store.record_outbox_retry(claim_token, &schedule(*delay), error_code, &message, now)
        }
        OutboxDecision::Unknown(reason) => {
            store.record_outbox_unknown(claim_token, reason, error_code, &message, now)
        }
        OutboxDecision::FailedPermanent => {
            store.record_outbox_failed(claim_token, error_code, &message, now)
        }
        // Nothing was sent, so a clean cancel is recorded with no uncertainty.
        OutboxDecision::Cancelled => store.record_outbox_cancelled(claim_token, now),
        // A media-parameter error needs a corrected plan before another
        // attempt, so it leaves the automatic queue as a terminal row carrying
        // the exact Bot API message.
        OutboxDecision::NeedsMediaCorrection => {
            store.record_outbox_failed(claim_token, error_code, &message, now)
        }
    };
    written.map_err(store_error)?;
    Err(RunAttemptError::Failed {
        classification,
        transition,
        message,
    })
}

fn store_error(error: SendStateError) -> RunAttemptError {
    match error {
        SendStateError::StaleClaim => RunAttemptError::StaleClaim,
        other => RunAttemptError::Store(other),
    }
}

/// One media item of a planned send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedMediaItem {
    pub media_kind: MediaKind,
    /// Archive-root-relative path, not an absolute machine-local location.
    pub file_path: PathBuf,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub caption: Option<String>,
    /// SHA-256 already computed while archiving. It is reused as the `file_id`
    /// cache key (plan TG-05) instead of hashing the file again.
    pub content_sha256: Option<String>,
    pub size_bytes: Option<u64>,
}

/// What one outbox entry transmits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SendPayload {
    Message {
        chat_id: String,
        message_thread_id: Option<i64>,
        text: String,
    },
    Media {
        chat_id: String,
        message_thread_id: Option<i64>,
        items: Vec<PlannedMediaItem>,
    },
}

/// A deterministic unit of Telegram work: one text message, one album or one
/// standalone media item (plan TG-03/TG-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedSend {
    /// Stable across restarts, so the same planned content is never queued twice.
    pub idempotency_key: String,
    /// `message`, `media_album` or `media_single`.
    pub message_kind: String,
    pub payload: SendPayload,
}

impl PlannedSend {
    /// Deterministic content fingerprint stored as `request_fingerprint`: a
    /// changed plan can then never reuse a result recorded for other content.
    pub fn fingerprint(&self) -> String {
        // File paths are runtime locations, not part of the logical payload.
        // Hash the portable media facts so moving an archive does not create a
        // different send intent.
        let canonical = match &self.payload {
            SendPayload::Message {
                chat_id,
                message_thread_id,
                text,
            } => serde_json::json!({
                "kind": "message",
                "chat_id": chat_id,
                "message_thread_id": message_thread_id,
                "text": text,
            })
            .to_string(),
            SendPayload::Media {
                chat_id,
                message_thread_id,
                items,
            } => {
                let items: Vec<_> = items
                    .iter()
                    .map(|item| {
                        serde_json::json!({
                            "media_kind": item.media_kind,
                            "file_name": item.file_name,
                            "mime_type": item.mime_type,
                            "caption": item.caption,
                            "content_sha256": item.content_sha256,
                            "size_bytes": item.size_bytes,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "kind": "media",
                    "chat_id": chat_id,
                    "message_thread_id": message_thread_id,
                    "items": items,
                })
                .to_string()
            }
        };
        sha256_hex(&[self.message_kind.as_bytes(), b"\x1f", canonical.as_bytes()])
    }

    /// The outbox row this plan becomes. A versioned payload snapshot is stored
    /// with archive-root-relative media references so restart recovery never
    /// depends on the current settings.
    ///
    /// `archive_directory` is the Tweet's committed archive directory. Every
    /// media reference must be an archive-relative path that stays inside it;
    /// otherwise the plan is rejected with `InvalidPayloadPath` rather than
    /// persisting an unsafe or machine-local snapshot.
    ///
    /// A text message references no file, so it is unaffected.
    #[allow(clippy::too_many_arguments)]
    pub fn to_outbox_entry(
        &self,
        bot_identity: &str,
        archive_directory: &str,
        tweet_id: Option<i64>,
        config_version: i64,
        plan_version: i64,
        plan_order: i64,
        created_at: &str,
    ) -> Result<NewOutboxEntry, SendStateError> {
        if let SendPayload::Media { items, .. } = &self.payload {
            let directory = Path::new(archive_directory);
            if !is_safe_archive_relative_path(directory) {
                return Err(SendStateError::InvalidPayloadPath);
            }
            for item in items {
                // `starts_with` is component-wise, so `archive-x` can never
                // masquerade as a child of `archive`, and the equality guard
                // requires a file *inside* the directory, not the directory.
                if !is_safe_archive_relative_path(&item.file_path)
                    || !item.file_path.starts_with(directory)
                    || item.file_path.as_path() == directory
                {
                    return Err(SendStateError::InvalidPayloadPath);
                }
            }
        }
        let payload_json = serde_json::to_string(&self.payload)
            .map_err(|error| SendStateError::Store(error.to_string()))?;
        let target_scope = match &self.payload {
            SendPayload::Message {
                chat_id,
                message_thread_id,
                ..
            }
            | SendPayload::Media {
                chat_id,
                message_thread_id,
                ..
            } => format!(
                "{chat_id}:{}",
                message_thread_id.map_or_else(|| "none".to_owned(), |id| id.to_string())
            ),
        };
        let (chat_id, message_thread_id, media_reference, content_sha256) = match &self.payload {
            SendPayload::Message {
                chat_id,
                message_thread_id,
                ..
            } => (chat_id.clone(), *message_thread_id, None, None),
            SendPayload::Media {
                chat_id,
                message_thread_id,
                items,
            } => (
                chat_id.clone(),
                *message_thread_id,
                Some(
                    items
                        .iter()
                        .map(|item| item.file_path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(";"),
                ),
                items.first().and_then(|item| item.content_sha256.clone()),
            ),
        };
        Ok(NewOutboxEntry {
            bot_identity: bot_identity.to_owned(),
            target_scope,
            chat_id,
            message_thread_id,
            idempotency_key: self.idempotency_key.clone(),
            request_fingerprint: self.fingerprint(),
            message_kind: self.message_kind.clone(),
            config_version,
            plan_version,
            plan_order,
            tweet_id,
            media_reference,
            content_sha256,
            payload_schema_version: 1,
            payload_json,
            created_at: created_at.to_owned(),
        })
    }
}

fn is_safe_archive_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// One text message as a planned send.
pub fn plan_text_send(
    chat_id: impl Into<String>,
    message_thread_id: Option<i64>,
    text: impl Into<String>,
    idempotency_key: impl Into<String>,
) -> PlannedSend {
    PlannedSend {
        idempotency_key: idempotency_key.into(),
        message_kind: "message".to_owned(),
        payload: SendPayload::Message {
            chat_id: chat_id.into(),
            message_thread_id,
            text: text.into(),
        },
    }
}

/// Turn an ordered media list into sendable units: albums of 2–10 items, with a
/// trailing group of one promoted to a standalone send (plan TG-03, mirroring
/// [`media_send_plan`] for the richer media items).
pub fn plan_media_sends(
    chat_id: impl Into<String>,
    message_thread_id: Option<i64>,
    items: Vec<PlannedMediaItem>,
    idempotency_prefix: &str,
) -> Vec<PlannedSend> {
    let chat_id = chat_id.into();
    let total = items.len();
    let mut units = Vec::new();
    let mut index = 0;
    while index < total {
        let remaining = total - index;
        let take = if remaining >= TELEGRAM_ALBUM_MIN_ITEMS {
            remaining.min(TELEGRAM_MEDIA_GROUP_LIMIT)
        } else {
            1
        };
        units.push(PlannedSend {
            idempotency_key: format!("{idempotency_prefix}:{index:02}"),
            message_kind: if take == 1 {
                "media_single".to_owned()
            } else {
                "media_album".to_owned()
            },
            payload: SendPayload::Media {
                chat_id: chat_id.clone(),
                message_thread_id,
                items: items[index..index + take].to_vec(),
            },
        });
        index += take;
    }
    units
}

// ---------------------------------------------------------------------------
// TG-05 — bot-isolated `file_id` cache contract
// ---------------------------------------------------------------------------

/// Representation version baked into every cache key. Bump it when the way
/// media is rendered or sent changes in a way that makes an older `file_id`
/// representation unusable for the new expectations.
pub const TELEGRAM_FILE_CACHE_VERSION: u32 = 1;

/// Cache key: `bot identity + SHA-256 + media kind + representation
/// version` (plan TG-05). `file_unique_id` is never part of the key — it is
/// identification only and never a send parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCacheKey {
    pub bot_identity: String,
    pub content_sha256: String,
    pub media_kind: MediaKind,
    pub representation_version: u32,
}

/// A confirmed cached upload result. Written only after the Bot API
/// confirmed the send (plan TG-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedFileId {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_size: u64,
    pub confirmed_at: String,
}

/// Persistence contract for the bot-isolated `file_id` cache.
pub trait FileIdCacheStore {
    /// Look up a file id for this exact bot and representation. A miss must
    /// fall back to uploading the original file, never to another bot's id.
    fn lookup_file_id(&self, key: &FileCacheKey) -> Result<Option<CachedFileId>, SendStateError>;

    /// Store a confirmed result. The caller only invokes this after a parsed
    /// `ok: true` response (plan TG-05).
    fn store_file_id(&self, key: &FileCacheKey, value: &CachedFileId)
    -> Result<(), SendStateError>;

    /// Remove one entry, used when Telegram explicitly reports the identifier
    /// as unusable so the next attempt uploads the original file again
    /// (plan TG-05). No other failure may call this.
    fn delete_file_id(&self, key: &FileCacheKey) -> Result<u64, SendStateError>;

    /// Drop every entry of exactly one bot identity (the chosen policy when
    /// the operator decides to purge after a token change). Entries of other
    /// identities are never touched.
    fn drop_file_cache_for_bot(&self, bot_identity: &str) -> Result<u64, SendStateError>;
}

/// True only for Telegram's explicit "this `file_id` is no longer usable"
/// answers. Permission, network or generic server failures must never be
/// treated as cache invalidation (plan TG-05): the cache entry stays until
/// Telegram itself says the identifier is dead.
pub fn is_invalid_file_id_error(error: &TelegramError) -> bool {
    let TelegramError::Api {
        code: 400 | 404,
        description,
        ..
    } = error
    else {
        return false;
    };
    let description = description.to_ascii_lowercase();
    [
        "invalid file id",
        "wrong file identifier",
        "file reference expired",
        "file reference empty",
    ]
    .iter()
    .any(|needle| description.contains(needle))
}

// ---------------------------------------------------------------------------
// TG-06 — task-level send projection (shared business model)
// ---------------------------------------------------------------------------

/// How an outbox state is presented on a task. The Windows GUI renders these
/// labels; the wording rules live here so every surface stays consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendProjection {
    Queued,
    Sending,
    SendConfirmed,
    WaitingToRetry,
    Failed,
    NeedsReview,
    Cancelled,
}

pub fn outbox_projection(state: OutboxState) -> SendProjection {
    match state {
        OutboxState::Queued => SendProjection::Queued,
        OutboxState::InFlight => SendProjection::Sending,
        OutboxState::Sent => SendProjection::SendConfirmed,
        OutboxState::RetryWait => SendProjection::WaitingToRetry,
        OutboxState::FailedPermanent => SendProjection::Failed,
        OutboxState::Unknown => SendProjection::NeedsReview,
        OutboxState::Cancelled => SendProjection::Cancelled,
    }
}

impl SendProjection {
    /// Stable task wording (plan TG-06). Only a confirmed Bot API result is
    /// shown as sent; nothing here ever claims the client received or read
    /// the message, because no separate reliable evidence for that exists.
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Telegram: queued",
            Self::Sending => "Telegram: sending",
            Self::SendConfirmed => "Telegram: send confirmed",
            Self::WaitingToRetry => "Telegram: waiting to retry",
            Self::Failed => "Telegram: send failed",
            Self::NeedsReview => "Telegram: outcome unknown, review required",
            Self::Cancelled => "Telegram: cancelled",
        }
    }
}

/// A deep link to a delivered message when one can be built by Telegram's
/// own rules, otherwise `None` — callers then keep the target and message id
/// instead of concatenating unvalidated input (plan TG-06).
///
/// Only the documented `https://t.me/c/<internal id>/<message id>` form for
/// channel/supergroup chats (`chat_id` starting with `-100`) is produced;
/// private chats and basic groups yield `None`.
pub fn message_link(chat_id: &str, message_id: &str) -> Option<String> {
    let internal = chat_id.strip_prefix("-100")?.parse::<i64>().ok()?;
    let message = message_id.parse::<i64>().ok()?;
    if internal <= 0 || message <= 0 {
        return None;
    }
    Some(format!("https://t.me/c/{internal}/{message}"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn stores_and_deletes_secrets_without_exposing_values() {
        let mut store = MemorySecretStore::default();
        store.set("telegram.bot_token", "123:secret").expect("set");
        assert_eq!(
            store.get("telegram.bot_token").expect("get"),
            Some("123:secret".into())
        );
        let token =
            BotToken::new(store.get("telegram.bot_token").expect("get").unwrap()).expect("token");
        assert_eq!(token.to_string(), "[REDACTED]");
        assert!(!format!("{token:?}").contains("123:secret"));
        assert!(!format!("{store:?}").contains("123:secret"));
        store.delete("telegram.bot_token").expect("delete");
        assert_eq!(store.get("telegram.bot_token").expect("get"), None);
    }

    #[test]
    fn formats_metadata_and_continuations() {
        let input = MetadataInput {
            tweet_id: "123",
            url: "https://x.com/alice/status/123",
            username: Some("alice"),
            display_name: Some("Alice"),
            text: "hello",
            tags: &["person", "launch"],
        };
        let text = format_metadata(&input);
        assert!(text.contains("Alice (@alice)"));
        assert!(text.contains("#person #launch"));
        assert_eq!(split_text("你好世界", 2), vec!["你好", "世界"]);
    }

    #[test]
    fn groups_media_in_telegram_limit() {
        let items = (0..21)
            .map(|index| MediaGroupItem {
                media_type: "photo".into(),
                media: format!("file-{index}"),
                caption: None,
            })
            .collect();
        let groups = media_groups(items);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 10);
        assert_eq!(groups[2].len(), 1);
    }

    #[test]
    fn validates_message_boundaries() {
        let valid = SendMessageRequest {
            chat_id: "-100".into(),
            text: "hello".into(),
            disable_web_page_preview: true,
        };
        assert!(validate_message(&valid).is_ok());
        assert_eq!(
            validate_message(&SendMessageRequest {
                chat_id: "".into(),
                ..valid.clone()
            }),
            Err(TelegramError::InvalidChatId)
        );
        assert_eq!(
            validate_message(&SendMessageRequest {
                text: "".into(),
                ..valid
            }),
            Err(TelegramError::EmptyText)
        );
    }

    fn fake_server(response: &'static str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = format!("http://{}", listener.local_addr().expect("address"));
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read timeout");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            let bytes = stream.read(&mut buffer).expect("read request");
            request.extend_from_slice(&buffer[..bytes]);
            let request = String::from_utf8_lossy(&request).into_owned();
            let body = response.as_bytes();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                response
            )
            .expect("write response");
            request
        });
        (address, handle)
    }

    #[test]
    fn sends_message_over_https_transport_contract() {
        let (endpoint, server) = fake_server(r#"{"ok":true}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
        let token = BotToken::new("123:secret").expect("token");
        let response = transport
            .send(
                &token,
                TelegramRequest::Message(SendMessageRequest {
                    chat_id: "-100".into(),
                    text: "hello".into(),
                    disable_web_page_preview: true,
                }),
            )
            .expect("send");
        assert!(response.ok);
        let request = server.join().expect("server");
        assert!(request.starts_with("POST /bot123:secret/sendMessage HTTP/1.1"));
        assert!(request.contains(r#""chat_id":"-100""#));
        assert!(request.contains(r#""text":"hello""#));
    }

    #[test]
    fn sends_photo_video_and_media_group_methods() {
        for (request, method) in [
            (
                TelegramRequest::Photo(SendPhotoRequest {
                    chat_id: "-100".into(),
                    photo: "photo-id".into(),
                    caption: Some("caption".into()),
                }),
                "sendPhoto",
            ),
            (
                TelegramRequest::Video(SendVideoRequest {
                    chat_id: "-100".into(),
                    video: "video-id".into(),
                    caption: None,
                }),
                "sendVideo",
            ),
            (
                TelegramRequest::MediaGroup(SendMediaGroupRequest {
                    chat_id: "-100".into(),
                    media: vec![MediaGroupItem {
                        media_type: "photo".into(),
                        media: "photo-id".into(),
                        caption: None,
                    }],
                }),
                "sendMediaGroup",
            ),
        ] {
            let (endpoint, server) = fake_server(r#"{"ok":true}"#);
            let transport =
                ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
            let token = BotToken::new("123:secret").expect("token");
            transport.send(&token, request).expect("send");
            let request = server.join().expect("server");
            assert!(request.starts_with(&format!("POST /bot123:secret/{method} HTTP/1.1")));
        }
    }

    #[test]
    fn maps_telegram_api_and_http_errors_without_exposing_token() {
        let (endpoint, server) =
            fake_server(r#"{"ok":false,"error_code":429,"description":"Too Many Requests"}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
        let token = BotToken::new("123:secret").expect("token");
        let error = transport
            .send(
                &token,
                TelegramRequest::Message(SendMessageRequest {
                    chat_id: "-100".into(),
                    text: "hello".into(),
                    disable_web_page_preview: false,
                }),
            )
            .expect_err("api error");
        assert_eq!(
            error,
            TelegramError::Api {
                code: 429,
                description: "Too Many Requests".into(),
                parameters: None
            }
        );
        assert!(!error.to_string().contains("123:secret"));
        server.join().expect("server");

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = format!("http://{}", listener.local_addr().expect("address"));
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .expect("write");
        });
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let error = transport
            .send(
                &token,
                TelegramRequest::Message(SendMessageRequest {
                    chat_id: "-100".into(),
                    text: "hello".into(),
                    disable_web_page_preview: false,
                }),
            )
            .expect_err("http error");
        assert_eq!(error, TelegramError::Transport("HTTP status 503".into()));
        server.join().expect("server");
    }

    #[test]
    fn rejects_non_https_production_endpoints() {
        assert!(matches!(
            ReqwestTelegramTransport::with_endpoint("http://localhost:8080"),
            Err(TelegramError::InvalidEndpoint)
        ));
        assert!(ReqwestTelegramTransport::new().is_ok());
    }

    #[test]
    fn a_resolved_proxy_actually_receives_the_request() {
        // A proxy receives an absolute-form request line. If the transport
        // discarded the proxy, the request would go straight to the target and
        // the proxy would never be contacted.
        let proxy_listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy");
        let proxy_address = proxy_listener.local_addr().expect("proxy address");
        let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let recorded = seen.clone();
        let proxy = thread::spawn(move || {
            let (mut stream, _) = proxy_listener.accept().expect("accept");
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).unwrap_or(0);
            recorded
                .lock()
                .expect("record")
                .push_str(&String::from_utf8_lossy(&request[..read]));
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"ok\":true,\"x\":1}",
            );
        });

        let transport = ReqwestTelegramTransport::with_resolved_proxy(
            "https://api.telegram.org",
            std::time::Duration::from_secs(10),
            Some(format!("http://{proxy_address}")),
        )
        .expect("transport");
        // The call must fail at the proxy stage, which is enough to prove the
        // request was routed there rather than to the real API.
        let _ = transport.send(
            &BotToken::new("123456:ABCDEF").expect("token"),
            TelegramRequest::Message(SendMessageRequest {
                chat_id: "1".into(),
                text: "hello".into(),
                disable_web_page_preview: false,
            }),
        );
        proxy.join().expect("proxy thread");
        let observed = seen.lock().expect("read").clone();
        assert!(
            observed.contains("api.telegram.org"),
            "the proxy must receive the request, got: {observed}"
        );
    }

    #[test]
    fn a_disabled_proxy_still_builds_a_direct_client() {
        // `with_network(..., disable_proxy = true)` must still build; the
        // no_proxy path is what keeps a direct promise.
        assert!(
            ReqwestTelegramTransport::with_network(
                "https://api.telegram.org",
                std::time::Duration::from_secs(5),
                true
            )
            .is_ok()
        );
    }

    #[test]
    fn a_malformed_proxy_is_rejected_rather_than_silently_ignored() {
        let result = ReqwestTelegramTransport::with_resolved_proxy(
            "https://api.telegram.org",
            std::time::Duration::from_secs(5),
            Some("not a url".to_owned()),
        );
        assert!(matches!(result, Err(TelegramError::Transport(_))));
    }

    #[derive(Default)]
    struct MemorySendStateStore {
        records: std::sync::Mutex<std::collections::HashMap<(String, String), MemorySendRecord>>,
    }

    #[derive(Clone)]
    struct MemorySendRecord {
        message_kind: String,
        state: SendState,
        attempt_count: u32,
        telegram_message_id: Option<String>,
        telegram_file_id: Option<String>,
    }

    impl SendStateStore for MemorySendStateStore {
        fn find_sent(
            &self,
            chat_id: &str,
            idempotency_key: &str,
        ) -> Result<Option<SentSendRecord>, SendStateError> {
            let records = self.records.lock().expect("lock");
            Ok(records
                .get(&(chat_id.to_owned(), idempotency_key.to_owned()))
                .filter(|record| record.state == SendState::Sent)
                .map(|record| SentSendRecord {
                    chat_id: chat_id.to_owned(),
                    idempotency_key: idempotency_key.to_owned(),
                    message_kind: record.message_kind.clone(),
                    telegram_message_id: record.telegram_message_id.clone().expect("sent id"),
                    telegram_file_id: record.telegram_file_id.clone(),
                }))
        }

        fn record_pending(
            &self,
            chat_id: &str,
            idempotency_key: &str,
            message_kind: &str,
            _updated_at: &str,
        ) -> Result<(), SendStateError> {
            let mut records = self.records.lock().expect("lock");
            let record = records
                .entry((chat_id.to_owned(), idempotency_key.to_owned()))
                .or_insert_with(|| MemorySendRecord {
                    message_kind: message_kind.to_owned(),
                    state: SendState::Pending,
                    attempt_count: 0,
                    telegram_message_id: None,
                    telegram_file_id: None,
                });
            record.state = SendState::Pending;
            record.attempt_count += 1;
            Ok(())
        }

        fn record_sent(
            &self,
            chat_id: &str,
            idempotency_key: &str,
            telegram_message_id: &str,
            telegram_file_id: Option<&str>,
            _updated_at: &str,
        ) -> Result<(), SendStateError> {
            let mut records = self.records.lock().expect("lock");
            let record = records
                .get_mut(&(chat_id.to_owned(), idempotency_key.to_owned()))
                .ok_or(SendStateError::NotFound)?;
            record.state = SendState::Sent;
            record.telegram_message_id = Some(telegram_message_id.to_owned());
            record.telegram_file_id = telegram_file_id.map(str::to_owned);
            Ok(())
        }

        fn record_failed(
            &self,
            chat_id: &str,
            idempotency_key: &str,
            _error_code: Option<i64>,
            _error_message: &str,
            _updated_at: &str,
        ) -> Result<(), SendStateError> {
            let mut records = self.records.lock().expect("lock");
            let record = records
                .get_mut(&(chat_id.to_owned(), idempotency_key.to_owned()))
                .ok_or(SendStateError::NotFound)?;
            record.state = SendState::Failed;
            Ok(())
        }

        fn list_unsent(&self) -> Result<Vec<PendingSendRecord>, SendStateError> {
            let records = self.records.lock().expect("lock");
            let mut pending: Vec<PendingSendRecord> = records
                .iter()
                .filter(|(_, record)| record.state != SendState::Sent)
                .map(|((chat_id, idempotency_key), record)| PendingSendRecord {
                    chat_id: chat_id.clone(),
                    idempotency_key: idempotency_key.clone(),
                    message_kind: record.message_kind.clone(),
                    state: record.state,
                    attempt_count: record.attempt_count,
                })
                .collect();
            pending.sort_by(|left, right| left.idempotency_key.cmp(&right.idempotency_key));
            Ok(pending)
        }
    }

    #[test]
    fn parses_result_message_id_from_bot_api_response() {
        let response: TelegramResponse =
            serde_json::from_str(r#"{"ok":true,"result":{"message_id":4242,"chat":{"id":-100}}}"#)
                .expect("response");
        assert_eq!(response.result_message_id(), Some("4242".into()));
        let plain: TelegramResponse = serde_json::from_str(r#"{"ok":true}"#).expect("plain");
        assert_eq!(plain.result_message_id(), None);
    }

    #[test]
    fn idempotent_send_skips_transport_when_already_sent() {
        let store = MemorySendStateStore::default();
        store
            .record_pending("-100", "tweet-1:metadata", "metadata", "now")
            .expect("pending");
        store
            .record_sent("-100", "tweet-1:metadata", "77", Some("file-id-1"), "now")
            .expect("sent");
        let outcome = send_idempotently(
            &store,
            "-100",
            "tweet-1:metadata",
            "metadata",
            "now",
            || panic!("deliver must not run for already-sent messages"),
        )
        .expect("outcome");
        assert_eq!(
            outcome,
            IdempotentSendOutcome::AlreadySent {
                telegram_message_id: "77".into()
            }
        );
    }

    #[test]
    fn idempotent_send_records_delivered_state_and_stays_stable() {
        let store = MemorySendStateStore::default();
        let outcome = send_idempotently(&store, "-100", "tweet-1:media:1", "media", "now", || {
            Ok(DeliveredSend {
                telegram_message_id: "42".into(),
                telegram_file_id: Some("photo-file-id".into()),
            })
        })
        .expect("outcome");
        assert_eq!(
            outcome,
            IdempotentSendOutcome::Delivered(DeliveredSend {
                telegram_message_id: "42".into(),
                telegram_file_id: Some("photo-file-id".into()),
            })
        );
        let sent = store
            .find_sent("-100", "tweet-1:media:1")
            .expect("find")
            .expect("sent record");
        assert_eq!(sent.telegram_message_id, "42");
        assert_eq!(sent.telegram_file_id.as_deref(), Some("photo-file-id"));
        assert!(store.list_unsent().expect("unsent").is_empty());
        let outcome = send_idempotently(&store, "-100", "tweet-1:media:1", "media", "now", || {
            panic!("deliver must not run twice for the same key")
        })
        .expect("outcome");
        assert_eq!(
            outcome,
            IdempotentSendOutcome::AlreadySent {
                telegram_message_id: "42".into()
            }
        );
    }

    #[test]
    fn idempotent_send_persists_failure_and_retries_with_same_key() {
        let store = MemorySendStateStore::default();
        let error = send_idempotently(
            &store,
            "-100",
            "tweet-2:metadata",
            "metadata",
            "now",
            || {
                Err(TelegramError::Api {
                    code: 429,
                    description: "Too Many Requests".into(),
                    parameters: None,
                })
            },
        )
        .expect_err("first attempt fails");
        assert_eq!(
            error,
            IdempotentSendError::Telegram(TelegramError::Api {
                code: 429,
                description: "Too Many Requests".into(),
                parameters: None,
            })
        );
        let unsent = store.list_unsent().expect("unsent");
        assert_eq!(unsent.len(), 1);
        assert_eq!(unsent[0].state, SendState::Failed);
        assert_eq!(unsent[0].attempt_count, 1);
        assert_eq!(unsent[0].message_kind, "metadata");

        let outcome = send_idempotently(
            &store,
            "-100",
            "tweet-2:metadata",
            "metadata",
            "now",
            || {
                Ok(DeliveredSend {
                    telegram_message_id: "9001".into(),
                    telegram_file_id: None,
                })
            },
        )
        .expect("retry succeeds");
        assert_eq!(
            outcome,
            IdempotentSendOutcome::Delivered(DeliveredSend {
                telegram_message_id: "9001".into(),
                telegram_file_id: None,
            })
        );
        assert!(store.list_unsent().expect("unsent").is_empty());
        let sent = store
            .find_sent("-100", "tweet-2:metadata")
            .expect("find")
            .expect("sent record");
        assert_eq!(sent.telegram_message_id, "9001");
    }

    fn photo_item(index: usize) -> MediaGroupItem {
        MediaGroupItem {
            media_type: "photo".into(),
            media: format!("file-{index}"),
            caption: None,
        }
    }

    #[test]
    fn accepts_cloud_and_loopback_local_endpoints() {
        let cloud = TelegramEndpoint::parse(EndpointMode::Cloud, "https://api.telegram.org")
            .expect("cloud endpoint");
        assert_eq!(cloud.base(), "https://api.telegram.org");
        assert_eq!(cloud.mode(), EndpointMode::Cloud);
        assert_eq!(TelegramEndpoint::cloud().base(), cloud.base());

        // A trailing slash is normalised away rather than rejected.
        let with_slash = TelegramEndpoint::parse(EndpointMode::Cloud, "https://api.telegram.org/")
            .expect("trailing slash");
        assert_eq!(with_slash.base(), "https://api.telegram.org");

        for base in [
            "http://127.0.0.1:8081",
            "http://localhost:8081",
            "http://[::1]:8081",
        ] {
            let local =
                TelegramEndpoint::parse(EndpointMode::Local, base).expect("loopback endpoint");
            assert_eq!(local.mode(), EndpointMode::Local);
            assert_eq!(local.base(), base);
        }

        assert_eq!(EndpointMode::parse("cloud"), Some(EndpointMode::Cloud));
        assert_eq!(EndpointMode::parse("local"), Some(EndpointMode::Local));
        assert_eq!(EndpointMode::parse("Local"), None);
        assert_eq!(EndpointMode::Cloud.as_str(), "cloud");
    }

    #[test]
    fn rejects_endpoints_that_could_redirect_or_leak_the_token() {
        // Cloud must be HTTPS.
        assert_eq!(
            TelegramEndpoint::parse(EndpointMode::Cloud, "http://api.telegram.org"),
            Err(TelegramError::EndpointNotSecure)
        );
        // Local must be an explicit loopback HTTP address with a port.
        for base in [
            "http://192.168.1.5:8081",
            "http://10.0.0.1:8081",
            "http://127.0.0.1",
            "https://127.0.0.1:8081",
        ] {
            assert_eq!(
                TelegramEndpoint::parse(EndpointMode::Local, base),
                Err(TelegramError::EndpointNotLoopback),
                "expected rejection for {base}"
            );
        }
        // Embedded credentials, query, fragment and extra path are malformed in
        // either mode: they turn a token-bearing URL into another target.
        for base in [
            "https://user:pass@api.telegram.org",
            "https://api.telegram.org/?token=x",
            "https://api.telegram.org/#frag",
            "https://api.telegram.org/extra",
            "not a url",
        ] {
            assert_eq!(
                TelegramEndpoint::parse(EndpointMode::Cloud, base),
                Err(TelegramError::EndpointMalformed),
                "expected malformed for {base}"
            );
        }
    }

    #[test]
    fn classifies_media_by_mime_then_extension() {
        assert_eq!(
            classify_media(Some("image/jpeg"), "a.jpg"),
            MediaKind::Photo
        );
        assert_eq!(classify_media(Some("IMAGE/PNG"), "a.png"), MediaKind::Photo);
        assert_eq!(classify_media(Some("video/mp4"), "a.mp4"), MediaKind::Video);
        // A GIF is an animation, not a still photo; document is the safe claim.
        assert_eq!(
            classify_media(Some("image/gif"), "a.gif"),
            MediaKind::Document
        );
        // The extension is only a fallback when the MIME type is absent/unknown.
        assert_eq!(classify_media(None, "PHOTO.PNG"), MediaKind::Photo);
        assert_eq!(classify_media(None, "clip.MOV"), MediaKind::Video);
        assert_eq!(
            classify_media(Some("application/octet-stream"), "archive.zip"),
            MediaKind::Document
        );
        assert_eq!(classify_media(None, "noext"), MediaKind::Document);
    }

    #[test]
    fn builds_stable_upload_file_names() {
        let name = stable_upload_file_name(
            "1234567890123456789",
            2,
            "A1B2C3D4E5F60708112233",
            "IMG_0001.JPG",
        );
        assert_eq!(name, "x_1234567890123456789_02_a1b2c3d4e5f6.jpg");

        // Deterministic: identical inputs give an identical name.
        assert_eq!(
            name,
            stable_upload_file_name(
                "1234567890123456789",
                2,
                "A1B2C3D4E5F60708112233",
                "IMG_0001.JPG"
            )
        );
        // Two-digit index keeps names sortable and distinct.
        assert!(stable_upload_file_name("9", 12, "aa", "a.mp4").contains("_12_"));
        // Never a generic name and never a local path fragment.
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
        assert!(!name.contains(':'));
        assert_ne!(name, "video.mp4");
        // A missing or non-hex hash still yields a usable, distinct name.
        assert!(stable_upload_file_name("9", 1, "", "a.mp4").contains("nohash"));
        assert!(stable_upload_file_name("9", 1, "zzz", "a.mp4").contains("nohash"));
        // Path-hostile inputs are sanitised, not passed through.
        assert_eq!(
            stable_upload_file_name("../../etc/passwd", 1, "ab", "../../a.mp4"),
            "x_etcpasswd_01_ab.mp4"
        );
        // A missing extension falls back to `bin` rather than an empty suffix.
        assert!(stable_upload_file_name("9", 1, "ab", "noext").ends_with(".bin"));
    }

    #[test]
    fn promotes_a_trailing_single_item_out_of_an_album() {
        assert!(media_send_plan(vec![]).is_empty());

        assert_eq!(
            media_send_plan(vec![photo_item(0)]),
            vec![MediaGroupSend::Single(photo_item(0))]
        );

        let pair = media_send_plan((0..2).map(photo_item).collect());
        assert!(matches!(&pair[..], [MediaGroupSend::Album(items)] if items.len() == 2));

        // Whole multiples of ten never yield a single-item album.
        for total in [10usize, 20] {
            let plan = media_send_plan((0..total).map(photo_item).collect());
            assert!(
                plan.iter()
                    .all(|group| matches!(group, MediaGroupSend::Album(_))),
                "{total} items must not produce a single-item album"
            );
        }

        // 11 and 21 leave a trailing remainder of one: it becomes a Single,
        // never an invalid one-item album.
        for total in [11usize, 21] {
            let plan = media_send_plan((0..total).map(photo_item).collect());
            assert_eq!(plan.len(), if total == 11 { 2 } else { 3 });
            assert!(matches!(plan.last(), Some(MediaGroupSend::Single(_))));
            assert!(
                plan[..plan.len() - 1].iter().all(
                    |group| matches!(group, MediaGroupSend::Album(items) if items.len() == 10)
                )
            );
        }
    }

    #[test]
    fn secret_store_error_variants_do_not_carry_the_secret() {
        let unavailable = SecretStoreError::Unavailable("credential manager offline".into());
        assert_eq!(
            unavailable.to_string(),
            "secret store unavailable: credential manager offline"
        );
        let denied = SecretStoreError::AccessDenied("policy blocked read".into());
        assert_eq!(
            denied.to_string(),
            "secret store access denied: policy blocked read"
        );
        assert_ne!(unavailable, denied);
    }

    fn sample_upload_request() -> UploadRequest {
        UploadRequest {
            chat_id: "-100123".into(),
            media_kind: MediaKind::Photo,
            file_path: std::path::PathBuf::from("/tmp/x.jpg"),
            file_name: "x_1_ab.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            caption: Some("#tag".into()),
            expected_size: Some(10),
        }
    }

    #[test]
    fn validates_upload_request_boundaries() {
        assert!(validate_upload_request(&sample_upload_request()).is_ok());

        let empty_chat = UploadRequest {
            chat_id: " ".into(),
            ..sample_upload_request()
        };
        assert_eq!(
            validate_upload_request(&empty_chat),
            Err(TelegramError::InvalidChatId)
        );

        // The name lands in a Content-Disposition header: emptiness and
        // quoting/newline/path characters must all be rejected.
        for bad_name in [
            "",
            "  ",
            "with/slash.jpg",
            "with\"quote.jpg",
            "line\nbreak.jpg",
            "back\\slash.jpg",
        ] {
            let request = UploadRequest {
                file_name: bad_name.into(),
                ..sample_upload_request()
            };
            assert!(
                matches!(
                    validate_upload_request(&request),
                    Err(TelegramError::InvalidUploadRequest(_))
                ),
                "{bad_name:?} must be rejected"
            );
        }

        for bad_mime in ["", "image", "image/png\rX: y"] {
            let request = UploadRequest {
                mime_type: Some(bad_mime.into()),
                ..sample_upload_request()
            };
            assert!(
                matches!(
                    validate_upload_request(&request),
                    Err(TelegramError::InvalidUploadRequest(_))
                ),
                "{bad_mime:?} must be rejected"
            );
        }
        let no_mime = UploadRequest {
            mime_type: None,
            ..sample_upload_request()
        };
        assert!(validate_upload_request(&no_mime).is_ok());
    }

    #[test]
    fn validates_upload_timeout_layers() {
        assert!(UploadTimeouts::default().validate().is_ok());
        let zero = Duration::ZERO;
        let broken = [
            UploadTimeouts {
                connect: zero,
                ..UploadTimeouts::default()
            },
            UploadTimeouts {
                body_stall: zero,
                ..UploadTimeouts::default()
            },
            UploadTimeouts {
                server_processing: zero,
                ..UploadTimeouts::default()
            },
        ];
        for timeouts in broken {
            assert!(
                matches!(
                    timeouts.validate(),
                    Err(TelegramError::InvalidUploadRequest(_))
                ),
                "{timeouts:?} must be rejected"
            );
        }
        // The overall deadline is optional; omitting it stays valid.
        let no_total = UploadTimeouts {
            total: None,
            ..UploadTimeouts::default()
        };
        assert!(no_total.validate().is_ok());
    }

    #[test]
    fn maps_media_kinds_to_upload_methods_and_fields() {
        assert_eq!(upload_method(MediaKind::Photo), ("sendPhoto", "photo"));
        assert_eq!(upload_method(MediaKind::Video), ("sendVideo", "video"));
        assert_eq!(
            upload_method(MediaKind::Document),
            ("sendDocument", "document")
        );
    }

    #[test]
    fn extracts_message_and_file_ids_from_success_results() {
        let album: TelegramResponse = serde_json::from_str(
            r#"{"ok":true,"result":[{"message_id":1,"photo":{"file_id":"p1"}},{"message_id":2,"photo":{"file_id":"p2"}}]}"#,
        )
        .expect("album response");
        assert_eq!(album.result_message_ids(), ["1", "2"]);
        assert_eq!(album.result_file_ids(), ["p1", "p2"]);

        let single: TelegramResponse = serde_json::from_str(
            r#"{"ok":true,"result":{"message_id":9,"video":{"file_id":"v1"}}}"#,
        )
        .expect("single response");
        assert_eq!(single.result_message_ids(), ["9"]);
        assert_eq!(single.result_file_ids(), ["v1"]);
        assert_eq!(single.result_message_id(), Some("9".into()));

        let plain: TelegramResponse =
            serde_json::from_str(r#"{"ok":true,"result":true}"#).expect("plain response");
        assert!(plain.result_message_ids().is_empty());
        assert!(plain.result_file_ids().is_empty());
    }

    #[test]
    fn api_endpoint_contract_gates_local_http_production_transport() {
        // Local mode is the production consumer of the TG-01 contract: an
        // explicit loopback HTTP server is accepted...
        let local =
            TelegramEndpoint::parse(EndpointMode::Local, "http://127.0.0.1:8081/").expect("local");
        assert_eq!(local.base(), "http://127.0.0.1:8081");
        let transport =
            ReqwestTelegramTransport::with_api_endpoint(local, Duration::from_secs(5), None)
                .expect("local transport");
        assert!(format!("{transport:?}").contains("http://127.0.0.1:8081"));

        // ...while non-loopback HTTP, a port-less local target and local
        // HTTPS are refused by the contract itself, before any transport
        // exists, and the raw-string constructor still demands HTTPS.
        assert!(matches!(
            TelegramEndpoint::parse(EndpointMode::Local, "http://93.184.216.34:8080"),
            Err(TelegramError::EndpointNotLoopback)
        ));
        assert!(matches!(
            TelegramEndpoint::parse(EndpointMode::Local, "http://127.0.0.1"),
            Err(TelegramError::EndpointNotLoopback)
        ));
        assert!(matches!(
            TelegramEndpoint::parse(EndpointMode::Local, "https://127.0.0.1:8081"),
            Err(TelegramError::EndpointNotLoopback)
        ));
        assert!(matches!(
            TelegramEndpoint::parse(EndpointMode::Cloud, "http://api.telegram.org"),
            Err(TelegramError::EndpointNotSecure)
        ));

        let cloud = ReqwestTelegramTransport::with_api_endpoint(
            TelegramEndpoint::cloud(),
            Duration::from_secs(5),
            None,
        )
        .expect("cloud transport");
        assert!(format!("{cloud:?}").contains("https://api.telegram.org"));
    }

    // ---------------------------------------------------------------
    // TG-02 upload test helpers
    // ---------------------------------------------------------------

    fn temp_upload_file(tag: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "xarchive-tg-upload-{tag}-{}.bin",
            std::process::id()
        ));
        std::fs::write(&path, bytes).expect("write temp upload file");
        path
    }

    /// Reads one HTTP request: headers first, then exactly `Content-Length`
    /// body bytes (or, without a length, until the peer stops sending).
    fn read_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let mut received = Vec::new();
        let mut buffer = [0_u8; 4096];
        let header_end = loop {
            let bytes = stream.read(&mut buffer).expect("read request");
            if bytes == 0 {
                break None;
            }
            received.extend_from_slice(&buffer[..bytes]);
            if let Some(index) = received.windows(4).position(|window| window == b"\r\n\r\n") {
                break Some(index + 4);
            }
        };
        let Some(header_end) = header_end else {
            return received;
        };
        let headers = String::from_utf8_lossy(&received[..header_end]).to_ascii_lowercase();
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse::<usize>().ok());
        if let Some(length) = content_length {
            while received.len() < header_end + length {
                let bytes = stream.read(&mut buffer).expect("read body");
                if bytes == 0 {
                    break;
                }
                received.extend_from_slice(&buffer[..bytes]);
            }
        }
        received
    }

    /// Serves exactly one request and answers with a 200 JSON response of
    /// `body`. `claimed_length` overrides the announced `Content-Length`
    /// (which may exceed the real body) so the size bound can be tested.
    fn upload_http_server(
        claimed_length: Option<u64>,
        body: Vec<u8>,
    ) -> (String, thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind upload server");
        let address = format!("http://{}", listener.local_addr().expect("address"));
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept upload request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("read timeout");
            let received = read_http_request(&mut stream);
            let length = claimed_length.unwrap_or(body.len() as u64);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
            // Keep the socket open briefly so the client can read the answer.
            thread::sleep(Duration::from_millis(50));
            received
        });
        (address, handle)
    }

    /// Runs one upload future on its own runtime.
    ///
    /// The transport (a blocking reqwest client) is always constructed
    /// outside any runtime context — reqwest 0.13.4 refuses to build it
    /// inside one — so the runtime exists only around the async send.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("upload test runtime")
            .block_on(future)
    }

    #[tokio::test]
    async fn watchdog_classifies_the_first_layered_deadline() {
        let layer = Duration::from_millis(40);
        let slow = Duration::from_secs(60);
        let base = UploadTimeouts {
            connect: Duration::from_secs(5),
            body_stall: slow,
            server_processing: slow,
            total: None,
        };

        // Streaming with no byte progress for `body_stall` -> body stall.
        let streaming = Arc::new(UploadTracker::new());
        streaming.record_body_progress(1);
        let error = upload_watchdog(
            Arc::clone(&streaming),
            UploadTimeouts {
                body_stall: layer,
                ..base.clone()
            },
        )
        .await;
        assert_eq!(error, TelegramError::UploadBodyStalled);

        // Body finished, response slower than `server_processing` -> server timeout.
        let finished = Arc::new(UploadTracker::new());
        finished.mark_body_finished();
        let error = upload_watchdog(
            finished,
            UploadTimeouts {
                server_processing: layer,
                ..base.clone()
            },
        )
        .await;
        assert_eq!(error, TelegramError::UploadServerTimeout);

        // Before the body starts, only the overall deadline applies.
        let queued = Arc::new(UploadTracker::new());
        let error = upload_watchdog(
            queued,
            UploadTimeouts {
                total: Some(layer),
                ..base
            },
        )
        .await;
        assert_eq!(error, TelegramError::UploadDeadlineExceeded);
    }

    #[test]
    fn upload_short_circuits_before_the_network_when_cancelled() {
        let transport =
            ReqwestTelegramTransport::with_test_endpoint("http://127.0.0.1:9").expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&stages);
        let error = block_on(transport.send_upload(
            &token,
            &sample_upload_request(),
            &UploadTimeouts::default(),
            &cancellation,
            move |stage| observed.lock().expect("stages").push(stage),
        ))
        .expect_err("cancelled upload must not run");
        assert_eq!(error, TelegramError::UploadCancelled);
        // Only the queued stage was reported: no file check, no upload, and
        // no fabricated progress.
        assert_eq!(*stages.lock().expect("stages"), vec![UploadStage::Queued]);
    }

    #[test]
    fn upload_reports_missing_and_changed_files_before_any_network() {
        let transport =
            ReqwestTelegramTransport::with_test_endpoint("http://127.0.0.1:9").expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let cancellation = CancellationToken::new();

        let mut missing = sample_upload_request();
        missing.file_path = std::env::temp_dir().join("xarchive-tg-missing-upload-file.bin");
        let error = block_on(transport.send_upload(
            &token,
            &missing,
            &UploadTimeouts::default(),
            &cancellation,
            |_| {},
        ))
        .expect_err("missing file must fail");
        assert!(matches!(error, TelegramError::FileUnreadable(_)));

        let path = temp_upload_file("changed", b"1234");
        let mut changed = sample_upload_request();
        changed.file_path = path.clone();
        changed.expected_size = Some(999);
        let error = block_on(transport.send_upload(
            &token,
            &changed,
            &UploadTimeouts::default(),
            &cancellation,
            |_| {},
        ))
        .expect_err("size mismatch must fail");
        assert!(matches!(error, TelegramError::FileChanged(_)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn send_upload_streams_the_file_and_reports_ordered_progress() {
        let payload: Vec<u8> = (0..100_000u32).map(|index| (index % 251) as u8).collect();
        let path = temp_upload_file("stream", &payload);
        let expected_size = payload.len() as u64;

        let response_body = br#"{"ok":true,"result":{"message_id":77,"chat":{"id":-100},"photo":{"file_id":"photo-file-1"}}}"#
            .to_vec();
        let (address, server) = upload_http_server(None, response_body);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let cancellation = CancellationToken::new();

        let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&stages);
        let mut request = sample_upload_request();
        request.file_path = path.clone();
        request.expected_size = Some(expected_size);

        let response = block_on(transport.send_upload(
            &token,
            &request,
            &UploadTimeouts::default(),
            &cancellation,
            move |stage| observed.lock().expect("stages").push(stage),
        ))
        .expect("upload succeeds");

        // The success result carries message and media ids, not just a status.
        assert_eq!(response.result_message_id(), Some("77".into()));
        assert_eq!(response.result_file_ids(), ["photo-file-1"]);

        let received = server.join().expect("server thread");
        let received = String::from_utf8_lossy(&received).to_ascii_lowercase();
        assert!(received.contains("/bot1234:test/sendphoto"));
        assert!(received.contains("content-type: multipart/form-data"));
        assert!(received.contains("name=\"photo\""));
        assert!(
            received.contains("x_1_ab.jpg"),
            "file name reaches Telegram"
        );
        assert!(received.contains("content-length:"), "length-known body");

        // Progress order: queued -> checking -> uploading* -> awaiting -> confirmed.
        let stages = stages.lock().expect("stages").clone();
        assert_eq!(stages.first(), Some(&UploadStage::Queued));
        assert_eq!(stages.get(1), Some(&UploadStage::CheckingFile));
        let uploads: Vec<&UploadStage> = stages
            .iter()
            .filter(|stage| matches!(stage, UploadStage::Uploading { .. }))
            .collect();
        assert!(!uploads.is_empty());
        assert_eq!(
            uploads.last().copied(),
            Some(&UploadStage::Uploading {
                sent_bytes: expected_size,
                total_bytes: expected_size,
            })
        );
        assert_eq!(
            stages.get(stages.len() - 2),
            Some(&UploadStage::AwaitingResult)
        );
        assert_eq!(stages.last(), Some(&UploadStage::Confirmed));
        // Confirmed follows AwaitingResult: "body sent" is never "sent".
        let awaiting = stages
            .iter()
            .position(|stage| *stage == UploadStage::AwaitingResult)
            .expect("awaiting stage");
        let confirmed = stages
            .iter()
            .position(|stage| *stage == UploadStage::Confirmed)
            .expect("confirmed stage");
        assert!(awaiting < confirmed);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn send_upload_rejects_an_oversized_response_before_reading_it() {
        let path = temp_upload_file("oversize", b"tiny");
        // Announce more than the bound without ever sending the body: the
        // header check must refuse the response before it is read.
        let (address, server) =
            upload_http_server(Some(TELEGRAM_MAX_RESPONSE_BYTES + 1), Vec::new());
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let cancellation = CancellationToken::new();

        let mut request = sample_upload_request();
        request.file_path = path.clone();
        request.expected_size = None;
        let error = block_on(transport.send_upload(
            &token,
            &request,
            &UploadTimeouts::default(),
            &cancellation,
            |_| {},
        ))
        .expect_err("oversized response must be refused");
        assert_eq!(error, TelegramError::ResponseTooLarge);

        let _ = server.join();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn send_upload_classifies_a_bot_api_error_result() {
        let path = temp_upload_file("apierror", b"payload");
        let body = br#"{"ok":false,"error_code":400,"description":"chat not found"}"#.to_vec();
        let (address, server) = upload_http_server(None, body);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let cancellation = CancellationToken::new();

        let mut request = sample_upload_request();
        request.file_path = path.clone();
        request.expected_size = None;
        let error = block_on(transport.send_upload(
            &token,
            &request,
            &UploadTimeouts::default(),
            &cancellation,
            |_| {},
        ))
        .expect_err("ok:false must surface as an API error");
        assert_eq!(
            error,
            TelegramError::Api {
                code: 400,
                description: "chat not found".into(),
                parameters: None,
            }
        );

        let _ = server.join();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn classifies_send_failures_into_the_retry_policy() {
        let rate_limited = TelegramError::Api {
            code: 429,
            description: "Too Many Requests: retry after 7".into(),
            parameters: Some(serde_json::json!({ "retry_after": 7 })),
        };
        assert_eq!(
            classify_send_failure(&rate_limited, RequestProgress::NotSent),
            SendFailure::Retry {
                retry_after: Some(Duration::from_secs(7))
            }
        );
        let no_parameters = TelegramError::Api {
            code: 429,
            description: "Too Many Requests".into(),
            parameters: None,
        };
        assert_eq!(
            classify_send_failure(&no_parameters, RequestProgress::Sent),
            SendFailure::Retry {
                retry_after: Some(TELEGRAM_DEFAULT_RETRY_AFTER)
            }
        );

        for code in [401_i64, 403] {
            let error = TelegramError::Api {
                code,
                description: "unauthorized".into(),
                parameters: None,
            };
            assert_eq!(
                classify_send_failure(&error, RequestProgress::NotSent),
                SendFailure::Permanent,
                "auth failure {code} must stop"
            );
        }
        let target = TelegramError::Api {
            code: 400,
            description: "Bad Request: chat not found".into(),
            parameters: None,
        };
        assert_eq!(
            classify_send_failure(&target, RequestProgress::Sent),
            SendFailure::Permanent
        );
        let media = TelegramError::Api {
            code: 400,
            description: "Bad Request: PHOTO_INVALID_DIMENSIONS".into(),
            parameters: None,
        };
        assert_eq!(
            classify_send_failure(&media, RequestProgress::NotSent),
            SendFailure::MediaCorrection
        );
        let server = TelegramError::Api {
            code: 500,
            description: "Internal Server Error".into(),
            parameters: None,
        };
        assert_eq!(
            classify_send_failure(&server, RequestProgress::NotSent),
            SendFailure::Retry { retry_after: None }
        );

        // Response-loss and server-timeout rows are UNKNOWN regardless of
        // progress: never an automatic re-send.
        for error in [
            TelegramError::ResponseLost,
            TelegramError::UploadServerTimeout,
            TelegramError::ResponseTooLarge,
        ] {
            for progress in [
                RequestProgress::NotSent,
                RequestProgress::Partial,
                RequestProgress::Sent,
            ] {
                assert_eq!(
                    classify_send_failure(&error, progress),
                    SendFailure::Unknown
                );
            }
        }

        // Transport rows depend on whether the full request went out.
        let transport = TelegramError::Transport("boom".into());
        assert_eq!(
            classify_send_failure(&transport, RequestProgress::NotSent),
            SendFailure::Retry { retry_after: None }
        );
        assert_eq!(
            classify_send_failure(&transport, RequestProgress::Sent),
            SendFailure::Unknown
        );
        assert_eq!(
            classify_send_failure(&TelegramError::UploadBodyStalled, RequestProgress::Partial),
            SendFailure::Retry { retry_after: None }
        );
        assert_eq!(
            classify_send_failure(
                &TelegramError::UploadConnectTimeout,
                RequestProgress::NotSent
            ),
            SendFailure::Retry { retry_after: None }
        );
        assert_eq!(
            classify_send_failure(
                &TelegramError::UploadDeadlineExceeded,
                RequestProgress::Sent
            ),
            SendFailure::Unknown
        );

        // Cancellation records whether it raced a sent request.
        assert_eq!(
            classify_send_failure(&TelegramError::UploadCancelled, RequestProgress::NotSent),
            SendFailure::Cancelled { after_send: false }
        );
        assert_eq!(
            classify_send_failure(&TelegramError::UploadCancelled, RequestProgress::Sent),
            SendFailure::Cancelled { after_send: true }
        );

        // Config faults are permanent; input faults need correction.
        assert_eq!(
            classify_send_failure(&TelegramError::InvalidEndpoint, RequestProgress::NotSent),
            SendFailure::Permanent
        );
        assert_eq!(
            classify_send_failure(
                &TelegramError::FileChanged("size".into()),
                RequestProgress::NotSent
            ),
            SendFailure::MediaCorrection
        );
    }

    #[test]
    fn bounded_backoff_grows_then_caps() {
        assert_eq!(bounded_backoff(0), Duration::from_secs(5));
        assert_eq!(bounded_backoff(1), Duration::from_secs(10));
        assert_eq!(bounded_backoff(2), Duration::from_secs(20));
        assert!(bounded_backoff(4) > bounded_backoff(2));
        assert_eq!(bounded_backoff(20), Duration::from_secs(30 * 60));
        assert_eq!(bounded_backoff(u32::MAX), Duration::from_secs(30 * 60));
    }

    #[test]
    fn outbox_states_roundtrip_and_gate_automatic_sending() {
        for state in [
            OutboxState::Queued,
            OutboxState::InFlight,
            OutboxState::Sent,
            OutboxState::RetryWait,
            OutboxState::FailedPermanent,
            OutboxState::Unknown,
            OutboxState::Cancelled,
        ] {
            assert_eq!(OutboxState::parse(state.as_str()), Some(state));
        }
        assert_eq!(OutboxState::parse("WEIRD"), None);
        // Only queued/retry entries may be picked up automatically; UNKNOWN
        // requires human review and must never be re-sent by default.
        assert!(OutboxState::Queued.auto_sendable());
        assert!(OutboxState::RetryWait.auto_sendable());
        assert!(!OutboxState::Unknown.auto_sendable());
        assert!(!OutboxState::InFlight.auto_sendable());
        assert!(!OutboxState::Sent.auto_sendable());
        assert!(OutboxState::Sent.is_terminal());
        assert!(OutboxState::FailedPermanent.is_terminal());
        assert!(OutboxState::Cancelled.is_terminal());
        assert!(!OutboxState::Unknown.is_terminal());
    }

    #[test]
    fn bot_identity_is_stable_reversible_free_and_token_sized() {
        let token = BotToken::new("1234567:AAFakeSecretValue").expect("token");
        let identity = token.identity();
        assert_eq!(identity.len(), 64, "full SHA-256 hex");
        assert!(
            identity
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        );
        assert_eq!(identity, token.identity(), "identity is stable");
        assert!(!identity.contains("1234567"));
        assert!(!identity.contains("AAFakeSecret"));
        let other = BotToken::new("9999999:OtherSecret").expect("token");
        assert_ne!(identity, other.identity(), "rotation isolates the bot");
        // Debug/Display stay redacted with the new method present.
        assert!(!format!("{token:?}").contains("AAFakeSecretValue"));
        assert_eq!(token.to_string(), "[REDACTED]");
    }

    #[test]
    fn invalid_file_id_detection_is_conservative() {
        for description in [
            "Bad Request: invalid file id",
            "Bad Request: wrong file identifier specified",
            "Bad Request: file reference expired",
        ] {
            let error = TelegramError::Api {
                code: 400,
                description: description.into(),
                parameters: None,
            };
            assert!(
                is_invalid_file_id_error(&error),
                "{description} must trigger the upload fallback"
            );
        }
        // Everything else keeps the cache: permission, network, target and
        // generic server failures are never cache invalidation.
        for error in [
            TelegramError::Api {
                code: 400,
                description: "Bad Request: chat not found".into(),
                parameters: None,
            },
            TelegramError::Api {
                code: 403,
                description: "Forbidden: bot was blocked".into(),
                parameters: None,
            },
            TelegramError::Api {
                code: 429,
                description: "Too Many Requests".into(),
                parameters: None,
            },
            TelegramError::Transport("connection failed".into()),
            TelegramError::ResponseLost,
        ] {
            assert!(
                !is_invalid_file_id_error(&error),
                "{error:?} must keep the cache"
            );
        }
    }

    #[test]
    fn send_projection_labels_never_claim_receipt() {
        for state in [
            OutboxState::Queued,
            OutboxState::InFlight,
            OutboxState::Sent,
            OutboxState::RetryWait,
            OutboxState::FailedPermanent,
            OutboxState::Unknown,
            OutboxState::Cancelled,
        ] {
            let label = outbox_projection(state).label();
            let lowered = label.to_lowercase();
            assert!(
                !lowered.contains("received") && !lowered.contains("read"),
                "label {label:?} must not claim client receipt or read state"
            );
        }
        assert_eq!(
            outbox_projection(OutboxState::Sent).label(),
            "Telegram: send confirmed"
        );
        assert_eq!(
            outbox_projection(OutboxState::Unknown).label(),
            "Telegram: outcome unknown, review required"
        );
    }

    #[test]
    fn message_link_follows_the_official_form_only() {
        assert_eq!(
            message_link("-1001234567890", "42"),
            Some("https://t.me/c/1234567890/42".to_owned())
        );
        // Basic groups, private chats, malformed ids and zero ids never get a
        // concatenated link — callers keep target + message id instead.
        assert_eq!(message_link("-12345", "42"), None);
        assert_eq!(message_link("12345", "42"), None);
        assert_eq!(message_link("-100abc", "42"), None);
        assert_eq!(message_link("-1001234567890", "x"), None);
        assert_eq!(message_link("-1000", "42"), None);
        assert_eq!(message_link("-1001234567890", "0"), None);
    }

    #[test]
    fn failed_attempts_map_onto_the_next_durable_transition() {
        // A rate limit uses the server's own delay; anything else without one
        // uses bounded backoff from the attempt number.
        let rate_limited = SendFailure::Retry {
            retry_after: Some(Duration::from_secs(42)),
        };
        assert_eq!(
            decide_outbox_transition(&rate_limited, 3),
            OutboxDecision::RetryAfter(Duration::from_secs(42))
        );
        let backoff = SendFailure::Retry { retry_after: None };
        assert_eq!(
            decide_outbox_transition(&backoff, 0),
            OutboxDecision::RetryAfter(bounded_backoff(0))
        );
        assert_eq!(
            decide_outbox_transition(&backoff, 2),
            OutboxDecision::RetryAfter(bounded_backoff(2))
        );

        // Response loss and a cancel that raced a sent request both become
        // UNKNOWN, with distinct reasons so the review UI can explain them.
        assert_eq!(
            decide_outbox_transition(&SendFailure::Unknown, 0),
            OutboxDecision::Unknown(UNKNOWN_REASON_RESPONSE_LOST)
        );
        assert_eq!(
            decide_outbox_transition(&SendFailure::Cancelled { after_send: true }, 0),
            OutboxDecision::Unknown(UNKNOWN_REASON_CANCELLED_AFTER_SEND)
        );
        assert_eq!(
            decide_outbox_transition(&SendFailure::Cancelled { after_send: false }, 0),
            OutboxDecision::Cancelled
        );

        assert_eq!(
            decide_outbox_transition(&SendFailure::Permanent, 0),
            OutboxDecision::FailedPermanent
        );
        assert_eq!(
            decide_outbox_transition(&SendFailure::MediaCorrection, 0),
            OutboxDecision::NeedsMediaCorrection
        );
        // Only a plain retry ever schedules another automatic attempt.
        for failure in [
            SendFailure::Unknown,
            SendFailure::Permanent,
            SendFailure::MediaCorrection,
            SendFailure::Cancelled { after_send: true },
            SendFailure::Cancelled { after_send: false },
        ] {
            assert!(
                !matches!(
                    decide_outbox_transition(&failure, 0),
                    OutboxDecision::RetryAfter(_)
                ),
                "{failure:?} must not schedule an automatic retry"
            );
        }
    }

    // ---------------------------------------------------------------------
    // Attempt driver and send plans
    // ---------------------------------------------------------------------

    /// In-memory `TelegramOutboxStore` for driver tests: it records every
    /// transition the driver persists so the protocol can be asserted without
    /// a database.
    #[derive(Default)]
    struct RecordingOutbox {
        state: Mutex<RecordingOutboxState>,
    }

    #[derive(Default)]
    struct RecordingOutboxState {
        writes: Vec<String>,
        fail_with: Option<SendStateError>,
    }

    impl RecordingOutbox {
        fn writes(&self) -> Vec<String> {
            self.state.lock().expect("lock").writes.clone()
        }

        fn fail_next_with(&self, error: SendStateError) {
            self.state.lock().expect("lock").fail_with = Some(error);
        }

        fn record(&self, entry: String) -> Result<(), SendStateError> {
            let mut state = self.state.lock().expect("lock");
            if let Some(error) = state.fail_with.take() {
                return Err(error);
            }
            state.writes.push(entry);
            Ok(())
        }
    }

    impl TelegramOutboxStore for RecordingOutbox {
        fn enqueue_outbox(&self, _entry: NewOutboxEntry) -> Result<i64, SendStateError> {
            Ok(1)
        }

        fn claim_due_outbox(
            &self,
            _bot_identity: &str,
            _claim_token: &str,
            _now: &str,
            _lease_until: &str,
        ) -> Result<Option<OutboxEntry>, SendStateError> {
            Ok(None)
        }

        fn claim_outbox(
            &self,
            _bot_identity: &str,
            _idempotency_key: &str,
            _claim_token: &str,
            _now: &str,
            _lease_until: &str,
        ) -> Result<Option<OutboxEntry>, SendStateError> {
            Ok(None)
        }

        fn mark_request_started(
            &self,
            claim_token: &str,
            _now: &str,
        ) -> Result<(), SendStateError> {
            assert_eq!(claim_token, "claim-1");
            self.record("started".to_owned())
        }

        fn record_outbox_sent(
            &self,
            _claim_token: &str,
            telegram_message_id: &str,
            results_json: Option<&str>,
            _now: &str,
        ) -> Result<(), SendStateError> {
            self.record(format!("sent:{telegram_message_id}:{results_json:?}"))
        }

        fn record_outbox_retry(
            &self,
            _claim_token: &str,
            next_retry_at: &str,
            error_code: Option<i64>,
            error_message: &str,
            _now: &str,
        ) -> Result<(), SendStateError> {
            self.record(format!(
                "retry:{next_retry_at}:{error_code:?}:{error_message}"
            ))
        }

        fn record_outbox_unknown(
            &self,
            _claim_token: &str,
            reason: &str,
            error_code: Option<i64>,
            error_message: &str,
            _now: &str,
        ) -> Result<(), SendStateError> {
            self.record(format!("unknown:{reason}:{error_code:?}:{error_message}"))
        }

        fn record_outbox_failed(
            &self,
            _claim_token: &str,
            error_code: Option<i64>,
            error_message: &str,
            _now: &str,
        ) -> Result<(), SendStateError> {
            self.record(format!("failed:{error_code:?}:{error_message}"))
        }

        fn record_outbox_cancelled(
            &self,
            _claim_token: &str,
            _now: &str,
        ) -> Result<(), SendStateError> {
            self.record("cancelled".to_owned())
        }

        fn cancel_outbox(
            &self,
            _bot_identity: &str,
            _idempotency_key: &str,
            _now: &str,
        ) -> Result<bool, SendStateError> {
            Ok(false)
        }

        fn recover_outbox_claims(&self, _now: &str) -> Result<u64, SendStateError> {
            Ok(0)
        }

        fn list_due_outbox(
            &self,
            _bot_identity: &str,
            _now: &str,
        ) -> Result<Vec<OutboxEntry>, SendStateError> {
            Ok(Vec::new())
        }
    }

    fn schedule_label(delay: Duration) -> String {
        format!("due+{}s", delay.as_secs())
    }

    #[test]
    fn attempt_driver_fences_before_sending_and_records_a_confirmed_result() {
        let store = RecordingOutbox::default();
        let success = block_on(run_claimed_attempt(
            &store,
            "claim-1",
            0,
            "2026-10-01T00:00:00Z",
            schedule_label,
            async {
                Ok(SendAttemptSuccess {
                    telegram_message_id: "77".to_owned(),
                    results_json: Some("{\"file_ids\":[\"p1\"]}".to_owned()),
                })
            },
        ))
        .expect("attempt");
        assert_eq!(success.telegram_message_id, "77");
        // The fence is written before anything else, so a crash after it can
        // only ever resolve to UNKNOWN, never to a silent re-send.
        let writes = store.writes();
        assert_eq!(writes[0], "started");
        assert!(writes[1].starts_with("sent:77:"));
    }

    #[test]
    fn attempt_driver_persists_the_retry_policy_of_each_failure_class() {
        // Rate limit: the server's own delay wins over the backoff.
        let store = RecordingOutbox::default();
        let error = block_on(run_claimed_attempt(
            &store,
            "claim-1",
            2,
            "2026-10-01T00:00:00Z",
            schedule_label,
            async {
                Err(SendAttemptError::new(
                    TelegramError::Api {
                        code: 429,
                        description: "Too Many Requests".to_owned(),
                        parameters: Some(serde_json::json!({ "retry_after": 7 })),
                    },
                    RequestProgress::Sent,
                ))
            },
        ))
        .expect_err("failed attempt");
        assert_eq!(
            error,
            RunAttemptError::Failed {
                classification: SendFailure::Retry {
                    retry_after: Some(Duration::from_secs(7))
                },
                transition: OutboxDecision::RetryAfter(Duration::from_secs(7)),
                message: "Telegram API error 429: Too Many Requests".to_owned(),
            }
        );
        assert_eq!(
            store.writes(),
            vec![
                "started".to_owned(),
                "retry:due+7s:Some(429):Telegram API error 429: Too Many Requests".to_owned()
            ]
        );

        // Response loss: UNKNOWN with its reason, never a scheduled re-send.
        let store = RecordingOutbox::default();
        let error = block_on(run_claimed_attempt(
            &store,
            "claim-1",
            0,
            "2026-10-01T00:00:00Z",
            schedule_label,
            async {
                Err(SendAttemptError::new(
                    TelegramError::ResponseLost,
                    RequestProgress::Sent,
                ))
            },
        ))
        .expect_err("failed attempt");
        assert!(matches!(
            error,
            RunAttemptError::Failed {
                transition: OutboxDecision::Unknown(UNKNOWN_REASON_RESPONSE_LOST),
                ..
            }
        ));
        assert!(
            store.writes()[1].starts_with("unknown:response_lost:"),
            "{:?}",
            store.writes()
        );

        // Auth failure: permanent, so no automatic attempt is scheduled.
        let store = RecordingOutbox::default();
        let error = block_on(run_claimed_attempt(
            &store,
            "claim-1",
            0,
            "2026-10-01T00:00:00Z",
            schedule_label,
            async {
                Err(SendAttemptError::new(
                    TelegramError::Api {
                        code: 401,
                        description: "Unauthorized".to_owned(),
                        parameters: None,
                    },
                    RequestProgress::Sent,
                ))
            },
        ))
        .expect_err("failed attempt");
        assert!(matches!(
            error,
            RunAttemptError::Failed {
                transition: OutboxDecision::FailedPermanent,
                ..
            }
        ));
        assert!(
            store.writes()[1].starts_with("failed:Some(401):"),
            "{:?}",
            store.writes()
        );

        // Cancel before the send: a clean cancel with no uncertainty.
        let store = RecordingOutbox::default();
        let error = block_on(run_claimed_attempt(
            &store,
            "claim-1",
            0,
            "2026-10-01T00:00:00Z",
            schedule_label,
            async {
                Err(SendAttemptError::new(
                    TelegramError::UploadCancelled,
                    RequestProgress::NotSent,
                ))
            },
        ))
        .expect_err("cancelled attempt");
        assert!(matches!(
            error,
            RunAttemptError::Failed {
                transition: OutboxDecision::Cancelled,
                ..
            }
        ));
        assert_eq!(store.writes(), vec!["started", "cancelled"]);

        // Media-parameter error: leaves the automatic queue for re-planning.
        let store = RecordingOutbox::default();
        let error = block_on(run_claimed_attempt(
            &store,
            "claim-1",
            0,
            "2026-10-01T00:00:00Z",
            schedule_label,
            async {
                Err(SendAttemptError::new(
                    TelegramError::Api {
                        code: 400,
                        description: "Bad Request: PHOTO_INVALID_DIMENSIONS".to_owned(),
                        parameters: None,
                    },
                    RequestProgress::Sent,
                ))
            },
        ))
        .expect_err("failed attempt");
        assert!(matches!(
            error,
            RunAttemptError::Failed {
                transition: OutboxDecision::NeedsMediaCorrection,
                ..
            }
        ));
        assert!(
            store.writes()[1].contains("PHOTO_INVALID_DIMENSIONS"),
            "{:?}",
            store.writes()
        );
    }

    #[test]
    fn attempt_driver_drops_a_lost_claim_without_sending_or_writing() {
        let store = RecordingOutbox::default();
        store.fail_next_with(SendStateError::StaleClaim);
        let sent = std::sync::atomic::AtomicBool::new(false);
        let error = block_on(run_claimed_attempt(
            &store,
            "claim-1",
            0,
            "2026-10-01T00:00:00Z",
            schedule_label,
            async {
                sent.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(SendAttemptSuccess {
                    telegram_message_id: "77".to_owned(),
                    results_json: None,
                })
            },
        ))
        .expect_err("stale claim");
        assert_eq!(error, RunAttemptError::StaleClaim);
        assert!(
            !sent.load(std::sync::atomic::Ordering::SeqCst),
            "a lost claim must stop before the request is sent"
        );
        assert!(store.writes().is_empty(), "no transition may be written");
    }

    fn planned_item(index: usize) -> PlannedMediaItem {
        PlannedMediaItem {
            media_kind: MediaKind::Photo,
            file_path: PathBuf::from(format!("archive/media/{index}.jpg")),
            file_name: format!("x_00_{index}.jpg"),
            mime_type: Some("image/jpeg".to_owned()),
            caption: None,
            content_sha256: Some(format!("sha-{index}")),
            size_bytes: Some(1024),
        }
    }

    #[test]
    fn media_plans_promote_a_trailing_single_and_keep_stable_keys() {
        // One item: a single send, never a one-item album.
        let single = plan_media_sends("-1001", None, vec![planned_item(0)], "tweet-1:media");
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].message_kind, "media_single");
        assert_eq!(single[0].idempotency_key, "tweet-1:media:00");

        // Two items: one album with both, in order.
        let pair = plan_media_sends(
            "-1001",
            None,
            (0..2).map(planned_item).collect(),
            "tweet-1:media",
        );
        assert_eq!(pair.len(), 1);
        assert_eq!(pair[0].message_kind, "media_album");
        match &pair[0].payload {
            SendPayload::Media { items, .. } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].file_name, "x_00_0.jpg");
                assert_eq!(items[1].file_name, "x_00_1.jpg");
            }
            other => panic!("expected a media payload, got {other:?}"),
        }

        // Eleven items: an album of ten plus a promoted single.
        let eleven = plan_media_sends(
            "-1001",
            Some(7),
            (0..11).map(planned_item).collect(),
            "tweet-1:media",
        );
        assert_eq!(eleven.len(), 2);
        assert_eq!(eleven[0].message_kind, "media_album");
        assert_eq!(eleven[1].message_kind, "media_single");
        assert_eq!(eleven[0].idempotency_key, "tweet-1:media:00");
        assert_eq!(eleven[1].idempotency_key, "tweet-1:media:10");
        match &eleven[1].payload {
            SendPayload::Media {
                message_thread_id,
                items,
                ..
            } => {
                assert_eq!(*message_thread_id, Some(7));
                assert_eq!(items.len(), 1);
            }
            other => panic!("expected a media payload, got {other:?}"),
        }

        // Twenty-one items: two albums plus a trailing single.
        assert_eq!(
            plan_media_sends(
                "-1001",
                None,
                (0..21).map(planned_item).collect(),
                "t:media"
            )
            .len(),
            3
        );
        // The same input always produces the same plan, so a restart never
        // queues the same content twice.
        assert_eq!(
            plan_media_sends(
                "-1001",
                Some(7),
                (0..11).map(planned_item).collect(),
                "tweet-1:media"
            ),
            eleven
        );
    }
    #[test]
    fn a_planned_send_fingerprints_its_content_and_becomes_an_outbox_entry() {
        let plan = plan_media_sends(
            "-1001",
            None,
            (0..2).map(planned_item).collect(),
            "tweet-1:media",
        )
        .remove(0);
        let fingerprint = plan.fingerprint();
        assert_eq!(fingerprint.len(), 64, "SHA-256 hex");
        assert_eq!(fingerprint, plan.fingerprint(), "fingerprints are stable");

        // Changing the content changes the fingerprint, so a recorded result
        // can never be reused for different media.
        let mut changed = plan.clone();
        if let SendPayload::Media { items, .. } = &mut changed.payload {
            items[1].content_sha256 = Some("sha-other".to_owned());
        }
        assert_ne!(changed.fingerprint(), fingerprint);

        let mut moved = plan.clone();
        if let SendPayload::Media { items, .. } = &mut moved.payload {
            items[0].file_path = PathBuf::from("relocated/0.jpg");
        }
        assert_eq!(moved.fingerprint(), fingerprint);

        let entry = plan
            .to_outbox_entry(
                "bot-identity",
                "archive",
                Some(42),
                7,
                1,
                0,
                "2026-10-01T00:00:00Z",
            )
            .expect("safe relative media reference");
        assert_eq!(entry.target_scope, "-1001:none");
        assert_eq!(entry.payload_schema_version, 1);
        assert!(!entry.payload_json.contains("/tmp/"));
        assert!(entry.payload_json.contains("media/0.jpg"));
        assert_eq!(entry.bot_identity, "bot-identity");
        assert_eq!(entry.chat_id, "-1001");
        assert_eq!(entry.idempotency_key, "tweet-1:media:00");
        assert_eq!(entry.request_fingerprint, fingerprint);
        assert_eq!(entry.message_kind, "media_album");
        assert_eq!(
            entry.config_version, 7,
            "queued items bind to a settings version"
        );
        assert_eq!(entry.plan_order, 0);
        assert_eq!(entry.tweet_id, Some(42));
        assert_eq!(
            entry.media_reference.as_deref(),
            Some("archive/media/0.jpg;archive/media/1.jpg")
        );
        assert_eq!(entry.content_sha256.as_deref(), Some("sha-0"));

        // A text plan carries no media reference and keeps its own kind.
        let text = plan_text_send("-1001", Some(9), "hello", "tweet-1:metadata");
        assert_eq!(text.message_kind, "message");
        let text_entry = text
            .to_outbox_entry(
                "bot-identity",
                "archive",
                None,
                7,
                1,
                1,
                "2026-10-01T00:00:00Z",
            )
            .expect("text plan");
        assert_eq!(text_entry.media_reference, None);
        assert_eq!(text_entry.content_sha256, None);
        assert_eq!(text_entry.message_thread_id, Some(9));
        assert_eq!(text_entry.idempotency_key, "tweet-1:metadata");
    }

    #[test]
    fn send_upload_attempt_reports_confirmed_ids_and_honest_progress() {
        let payload = b"media-bytes";
        let path = temp_upload_file("attempt", payload);
        let expected_size = payload.len() as u64;

        // Success: the confirmed message id and per-item file ids are extracted.
        let body = br#"{"ok":true,"result":{"message_id":77,"photo":{"file_id":"p1"}}}"#.to_vec();
        let (address, server) = upload_http_server(None, body);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let mut request = sample_upload_request();
        request.file_path = path.clone();
        request.expected_size = Some(expected_size);
        let success = block_on(transport.send_upload_attempt(
            &token,
            &request,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            |_| {},
        ))
        .expect("attempt");
        assert_eq!(success.telegram_message_id, "77");
        let results = success.results_json.expect("file results");
        assert!(results.contains("p1"), "{results}");
        assert!(results.contains("77"), "{results}");
        let _ = server.join();

        // A Bot API rejection arrives after the body was fully sent, so the
        // progress must say so instead of looking unsent.
        let body = br#"{"ok":false,"error_code":400,"description":"chat not found"}"#.to_vec();
        let (address, server) = upload_http_server(None, body);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let failure = block_on(transport.send_upload_attempt(
            &token,
            &request,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            |_| {},
        ))
        .expect_err("api error");
        assert_eq!(failure.progress, RequestProgress::Sent);
        assert!(matches!(
            failure.error,
            TelegramError::Api { code: 400, .. }
        ));
        let _ = server.join();
        let _ = std::fs::remove_file(path);
    }
    fn album_upload_request(path: &std::path::Path, file_name: &str) -> UploadRequest {
        UploadRequest {
            chat_id: "-1001".to_owned(),
            media_kind: MediaKind::Photo,
            file_path: path.to_path_buf(),
            file_name: file_name.to_owned(),
            mime_type: Some("image/jpeg".to_owned()),
            caption: None,
            expected_size: std::fs::metadata(path).map(|meta| meta.len()).ok(),
        }
    }

    #[test]
    fn an_album_streams_every_file_and_reuses_a_cached_file_id() {
        let first = temp_upload_file("album-a", b"first-item-bytes");
        let second = temp_upload_file("album-b", b"second-item-bytes");
        let body = br#"{"ok":true,"result":[{"message_id":11,"photo":{"file_id":"a1"}},{"message_id":12,"photo":{"file_id":"a2"}}]}"#
            .to_vec();
        let (address, server) = upload_http_server(None, body);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");

        let items = vec![
            AlbumItemUpload::File(Box::new(album_upload_request(&first, "album-a.jpg"))),
            // The second item reuses an identifier already cached for this bot.
            AlbumItemUpload::CachedFileId("cached-file-id".to_owned()),
        ];
        let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&stages);
        let success = block_on(send_media_group_attempt(
            &transport,
            &token,
            "-1001",
            &items,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            move |stage| observed.lock().expect("stages").push(stage),
        ))
        .expect("album attempt");
        assert_eq!(success.telegram_message_id, "11", "first confirmed message");
        let results = success.results_json.expect("album results");
        assert!(
            results.contains("11") && results.contains("12"),
            "{results}"
        );
        assert!(
            results.contains("a1") && results.contains("a2"),
            "{results}"
        );

        let received =
            String::from_utf8_lossy(&server.join().expect("server")).to_ascii_lowercase();
        assert!(received.contains("/bot1234:test/sendmediagroup"));
        assert!(received.contains("name=\"media\""));
        assert!(
            received.contains("cached-file-id"),
            "cached id sent as a value"
        );
        assert!(received.contains("album-a.jpg"), "file part keeps its name");
        assert!(
            received.contains("content-length:"),
            "length-known album body"
        );

        // Progress is reported for the whole album, ending in Confirmed.
        let stages = stages.lock().expect("stages").clone();
        assert_eq!(stages.first(), Some(&UploadStage::Queued));
        assert_eq!(stages.last(), Some(&UploadStage::Confirmed));
        assert!(
            stages.iter().any(
                |stage| matches!(stage, UploadStage::Uploading { sent_bytes, .. } if *sent_bytes > 0)
            ),
            "the album body reported progress: {stages:?}"
        );

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn an_album_outside_the_legal_range_is_refused_before_any_request() {
        let path = temp_upload_file("album-range", b"bytes");
        let transport =
            ReqwestTelegramTransport::with_test_endpoint("http://127.0.0.1:9").expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let request = album_upload_request(&path, "album.jpg");

        // One item is a single-media send, not an album.
        let single = vec![AlbumItemUpload::File(Box::new(request.clone()))];
        let error = block_on(send_media_group_attempt(
            &transport,
            &token,
            "-1001",
            &single,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            |_| {},
        ))
        .expect_err("one item is not an album");
        assert!(matches!(
            error.error,
            TelegramError::InvalidUploadRequest(_)
        ));
        assert_eq!(error.progress, RequestProgress::NotSent);

        // More than the album limit is refused as well.
        let too_many = (0..11)
            .map(|_| AlbumItemUpload::File(Box::new(request.clone())))
            .collect::<Vec<_>>();
        let error = block_on(send_media_group_attempt(
            &transport,
            &token,
            "-1001",
            &too_many,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            |_| {},
        ))
        .expect_err("eleven items is not an album");
        assert!(matches!(
            error.error,
            TelegramError::InvalidUploadRequest(_)
        ));

        // An item that cannot be read fails the album before any bytes move.
        let missing = album_upload_request(
            std::path::Path::new("/nonexistent/xarchive-tg-missing-album-item.jpg"),
            "missing.jpg",
        );
        let pair = vec![
            AlbumItemUpload::File(Box::new(request.clone())),
            AlbumItemUpload::File(Box::new(missing)),
        ];
        let error = block_on(send_media_group_attempt(
            &transport,
            &token,
            "-1001",
            &pair,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            |_| {},
        ))
        .expect_err("an unreadable item must fail the album");
        assert!(matches!(error.error, TelegramError::FileUnreadable(_)));
        assert_eq!(error.progress, RequestProgress::NotSent);
        let _ = std::fs::remove_file(path);
    }
}
