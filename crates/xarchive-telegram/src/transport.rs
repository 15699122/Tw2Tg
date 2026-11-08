//! Blocking control-request transport: the concrete Bot API client, endpoint
//! policy enforcement and the control-request payload/error mapping.
//!
//! This module owns exactly one responsibility: take a [`TelegramRequest`],
//! build the corresponding Bot API call, send it over the blocking client and
//! classify the outcome. It holds no outbox state, no lease and no upload
//! progress; the async streaming upload lives in the upload module.
//!
//! The crate root re-exports every public item, so call sites are unchanged.

use crate::contract::{
    BotToken, EndpointMode, SendMessageRequest, TelegramEndpoint, TelegramError, TelegramRequest,
    TelegramResponse, TelegramTransport,
};

/// Upper bound for any Telegram response body.
///
/// Bot API JSON responses are small; the bound keeps a hostile or broken
/// server from exhausting memory (plan TG-02: "Bound the response size").
///
/// Both the blocking control path and the streamed upload path enforce this
/// bound, so it lives with the transport contract rather than with either
/// sender.
pub const TELEGRAM_MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

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
    pub(crate) disable_proxy: bool,
    /// May carry credentials: never formatted, logged or persisted. The
    /// derived-from-nothing `Debug` below reports presence only.
    pub(crate) proxy: Option<String>,
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

    pub(crate) fn method_url(&self, token: &BotToken, method: &str) -> String {
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

pub(crate) fn request_payload(
    request: TelegramRequest,
) -> Result<(&'static str, serde_json::Value), TelegramError> {
    match request {
        TelegramRequest::GetMe => Ok(("getMe", serde_json::json!({}))),
        TelegramRequest::GetChat { chat_id } => {
            validate_chat_id(&chat_id)?;
            Ok(("getChat", serde_json::json!({"chat_id": chat_id})))
        }
        TelegramRequest::LogOut => Ok(("logOut", serde_json::json!({}))),
        TelegramRequest::Close => Ok(("close", serde_json::json!({}))),
        TelegramRequest::DeleteWebhook => Ok((
            "deleteWebhook",
            serde_json::json!({"drop_pending_updates": false}),
        )),
        TelegramRequest::Message(request) => {
            validate_message(&request)?;
            Ok((
                "sendMessage",
                serde_json::to_value(request).map_err(json_error)?,
            ))
        }
        TelegramRequest::Photo(request) => {
            validate_chat_id(&request.chat_id)?;
            validate_thread_id(request.message_thread_id)?;
            Ok((
                "sendPhoto",
                serde_json::to_value(request).map_err(json_error)?,
            ))
        }
        TelegramRequest::Video(request) => {
            validate_chat_id(&request.chat_id)?;
            validate_thread_id(request.message_thread_id)?;
            Ok((
                "sendVideo",
                serde_json::to_value(request).map_err(json_error)?,
            ))
        }
        TelegramRequest::MediaGroup(request) => {
            validate_chat_id(&request.chat_id)?;
            validate_thread_id(request.message_thread_id)?;
            let thread_id = request.message_thread_id;
            let mut payload = serde_json::to_value(request).map_err(json_error)?;
            if let Some(object) = payload.as_object_mut() {
                object.remove("message_thread_id");
                if let Some(thread_id) = thread_id {
                    object.insert(
                        "message_thread_id".to_owned(),
                        serde_json::Value::Number(thread_id.into()),
                    );
                }
            }
            Ok(("sendMediaGroup", payload))
        }
    }
}

pub(crate) fn validate_thread_id(thread_id: Option<i64>) -> Result<(), TelegramError> {
    if thread_id.is_some_and(|value| value <= 0) {
        Err(TelegramError::InvalidUploadRequest(
            "message thread id must be positive".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn json_error(error: serde_json::Error) -> TelegramError {
    TelegramError::Transport(error.to_string())
}

pub(crate) fn validate_chat_id(chat_id: &str) -> Result<(), TelegramError> {
    if chat_id.trim().is_empty() {
        Err(TelegramError::InvalidChatId)
    } else {
        Ok(())
    }
}

/// Bot API message validation, kept beside the transport that enforces it.
pub fn validate_message(request: &SendMessageRequest) -> Result<(), TelegramError> {
    validate_chat_id(&request.chat_id)?;
    validate_thread_id(request.message_thread_id)?;
    if request.text.is_empty() {
        return Err(TelegramError::EmptyText);
    }
    Ok(())
}
