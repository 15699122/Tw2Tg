//! Bot API wire contracts: token handle, request payloads, response envelope,
//! error type and endpoint policy.
//!
//! Nothing here performs I/O. `TelegramTransport` is a trait that the transport
//! module implements; the payload structs and their `serde` field names are
//! persisted/over-the-wire contracts and must not change without an ADR.
//!
//! The crate root re-exports every public item, so call sites are unchanged.

use serde::{Deserialize, Serialize};

use crate::{SecretStoreError, sha256_hex};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_thread_id: Option<i64>,
    pub text: String,
    pub disable_web_page_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendPhotoRequest {
    pub chat_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_thread_id: Option<i64>,
    pub photo: String,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendVideoRequest {
    pub chat_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_thread_id: Option<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_thread_id: Option<i64>,
    pub media: Vec<MediaGroupItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramRequest {
    GetMe,
    GetChat { chat_id: String },
    LogOut,
    Close,
    DeleteWebhook,
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

/// Authenticate a candidate without publishing content. Only a verified bot
/// user id may become a stable identity; token fingerprints are not bot ids.
pub fn verify_bot_identity(
    transport: &dyn TelegramTransport,
    token: &BotToken,
) -> Result<String, TelegramError> {
    let response = transport.send(token, TelegramRequest::GetMe)?;
    let result = response.result.as_ref();
    let id = result
        .and_then(|value| value.get("id"))
        .and_then(serde_json::Value::as_i64);
    if !response.ok
        || result
            .and_then(|value| value.get("is_bot"))
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || !id.is_some_and(|id| id > 0)
    {
        return Err(TelegramError::InvalidUploadRequest(
            "invalid bot identity response".into(),
        ));
    }
    Ok(format!("telegram-bot:{}", id.expect("validated id")))
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
                        .and_then(|media| {
                            media
                                .get("file_id")
                                .or_else(|| media.as_array()?.first()?.get("file_id"))
                        })
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
