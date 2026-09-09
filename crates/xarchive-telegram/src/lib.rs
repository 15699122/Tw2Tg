//! Telegram Bot API contracts, formatting primitives, and HTTPS transport.
//!
//! The Windows Credential Manager adapter remains outside this crate. The
//! network transport uses blocking reqwest with Rustls and keeps the token out
//! of error messages and debug output.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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
}

impl std::fmt::Display for SecretStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKey => formatter.write_str("secret key must not be empty"),
            Self::EmptyValue => formatter.write_str("secret value must not be empty"),
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
    Transport(String),
    Api { code: i64, description: String },
}

impl std::fmt::Display for TelegramError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidChatId => formatter.write_str("Telegram chat_id must not be empty"),
            Self::EmptyText => formatter.write_str("Telegram text must not be empty"),
            Self::InvalidEndpoint => formatter.write_str("Telegram endpoint must use HTTPS"),
            Self::Transport(message) => write!(formatter, "Telegram transport error: {message}"),
            Self::Api { code, description } => {
                write!(formatter, "Telegram API error {code}: {description}")
            }
        }
    }
}

impl std::error::Error for TelegramError {}

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
        Self::build(endpoint, false)
    }

    #[doc(hidden)]
    pub fn with_test_endpoint(endpoint: impl Into<String>) -> Result<Self, TelegramError> {
        let endpoint = normalize_endpoint(endpoint.into());
        validate_endpoint(&endpoint, true)?;
        Self::build(endpoint, true)
    }

    fn build(endpoint: String, disable_proxy: bool) -> Result<Self, TelegramError> {
        let mut builder = reqwest::blocking::Client::builder().timeout(Duration::from_secs(30));
        if disable_proxy {
            builder = builder.no_proxy();
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
}
