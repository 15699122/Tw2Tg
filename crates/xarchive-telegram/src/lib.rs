//! Telegram Bot API contracts, formatting primitives, and HTTPS transport.
//!
//! The Windows Credential Manager adapter remains outside this crate. The
//! network transport uses blocking reqwest with Rustls and keeps the token out
//! of error messages and debug output.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    Transport(String),
    Api {
        code: i64,
        description: String,
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
            Self::Transport(message) => write!(formatter, "Telegram transport error: {message}"),
            Self::Api { code, description } => {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointMode {
    Cloud,
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
}

impl std::fmt::Display for SendStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(message) => write!(formatter, "send state store error: {message}"),
            Self::NotFound => formatter.write_str("send state record not found"),
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
                TelegramError::Api { code, description } => (Some(*code), description.clone()),
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

#[derive(Debug, Clone)]
pub struct ReqwestTelegramTransport {
    client: reqwest::blocking::Client,
    endpoint: String,
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

    fn build(
        endpoint: String,
        disable_proxy: bool,
        timeout: std::time::Duration,
        proxy: Option<String>,
    ) -> Result<Self, TelegramError> {
        let mut builder = reqwest::blocking::Client::builder().timeout(timeout);
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
        Ok(Self { client, endpoint })
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
        let response = self
            .client
            .post(self.method_url(token, method))
            .json(&payload)
            .send()
            .map_err(|_| TelegramError::Transport("HTTP request failed".to_owned()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(TelegramError::Transport(format!(
                "HTTP status {}",
                status.as_u16()
            )));
        }
        let telegram_response = response
            .json::<TelegramResponse>()
            .map_err(|_| TelegramError::Transport("invalid Telegram JSON response".to_owned()))?;
        if !telegram_response.ok {
            return Err(TelegramError::Api {
                code: telegram_response.error_code.unwrap_or(0),
                description: telegram_response
                    .description
                    .unwrap_or_else(|| "Telegram API request failed".to_owned()),
            });
        }
        Ok(telegram_response)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadMode {
    /// Browse in the client: photo/video/album methods. No byte-identical
    /// download promise.
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
            if group.len() < TELEGRAM_ALBUM_MIN_ITEMS {
                if let Some(single) = group.pop() {
                    return MediaGroupSend::Single(single);
                }
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
                description: "Too Many Requests".into()
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
                })
            },
        )
        .expect_err("first attempt fails");
        assert_eq!(
            error,
            IdempotentSendError::Telegram(TelegramError::Api {
                code: 429,
                description: "Too Many Requests".into(),
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
}
