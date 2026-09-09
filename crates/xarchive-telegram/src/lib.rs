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
}
