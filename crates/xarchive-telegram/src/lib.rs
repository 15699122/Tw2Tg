//! Telegram Bot API contracts, formatting primitives, and HTTPS transport.
//!
//! The Windows Credential Manager adapter remains outside this crate. The
//! network transport uses blocking reqwest with Rustls and keeps the token out
//! of error messages and debug output.

use std::collections::HashMap;
use std::time::Duration;
pub use tokio_util::sync::CancellationToken;

mod contract;
mod delivery;
mod file_cache;
mod formatting;
mod outbox;
mod planning;
mod projection;
mod retry_policy;
mod transport;
mod upload;

pub use crate::file_cache::{
    CachedFileId, FileCacheKey, FileIdCacheStore, TELEGRAM_FILE_CACHE_VERSION,
    is_invalid_file_id_error,
};
pub use crate::outbox::{
    NewOutboxEntry, OutboxEntry, OutboxState, RunAttemptError, SendAttemptError,
    SendAttemptSuccess, TelegramOutboxStore, run_claimed_attempt, run_claimed_attempt_with_clock,
};
pub use crate::planning::{
    PlannedMediaItem, PlannedSend, SendPayload, plan_media_sends, plan_text_send,
};
pub use crate::transport::{
    ReqwestTelegramTransport, TELEGRAM_MAX_RESPONSE_BYTES, validate_message,
};
pub use crate::upload::{
    AlbumItemUpload, TELEGRAM_UPLOAD_CHUNK_BYTES, UploadRequest, UploadStage, UploadTimeouts,
    send_media_group_attempt,
};

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

pub use crate::contract::{
    BotToken, EndpointMode, MediaGroupItem, SendMediaGroupRequest, SendMessageRequest,
    SendPhotoRequest, SendVideoRequest, TELEGRAM_CLOUD_API_BASE, TelegramEndpoint, TelegramError,
    TelegramRequest, TelegramResponse, TelegramTransport, verify_bot_identity,
};

pub use crate::delivery::{
    DeliveredSend, IdempotentSendError, IdempotentSendOutcome, PendingSendRecord, SendState,
    SendStateError, SendStateStore, SentSendRecord, send_idempotently,
};

pub use crate::formatting::{
    MediaGroupSend, MediaKind, MetadataInput, UploadMode, classify_media, format_metadata,
    media_groups, media_send_plan, split_for_telegram, split_text, stable_upload_file_name,
};

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

// ---------------------------------------------------------------------------
// TG-04 — error classification and retry policy
// ---------------------------------------------------------------------------

pub use crate::retry_policy::{
    OutboxDecision, RequestProgress, SendFailure, TELEGRAM_DEFAULT_RETRY_AFTER, bounded_backoff,
    classify_send_failure, decide_outbox_transition,
};

/// `UNKNOWN` reason recorded when the request was fully sent but its response
/// was lost. Persisted so the review UI can explain the uncertainty.
pub const UNKNOWN_REASON_RESPONSE_LOST: &str = "response_lost";
/// `UNKNOWN` reason recorded when a user cancellation raced a request that
/// was already on the wire: the intent is kept, remote undo is never promised.
pub const UNKNOWN_REASON_CANCELLED_AFTER_SEND: &str = "cancelled_after_send";
/// `UNKNOWN` reason recorded by lease recovery when the crash happened after
/// the request had started (see `recover_outbox_claims`).
pub const UNKNOWN_REASON_LEASE_EXPIRED: &str = "claim_lease_expired";

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

// ---------------------------------------------------------------------------
// TG-06 — task-level send projection (shared business model)
// ---------------------------------------------------------------------------

/// How an outbox state is presented on a task. The Windows GUI renders these
/// labels; the wording rules live here so every surface stays consistent.
pub use crate::projection::{SendProjection, message_link, outbox_projection};

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::transport::{json_error, request_payload};
    use crate::upload::{UploadTracker, upload_method, upload_watchdog, validate_upload_request};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
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
    fn validates_message_boundaries() {
        let valid = SendMessageRequest {
            chat_id: "-100".into(),
            message_thread_id: Some(42),
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
            let request = read_http_request(&mut stream);
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
                    message_thread_id: Some(42),
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
        let body = request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("HTTP body");
        let json: serde_json::Value = serde_json::from_str(body).expect("JSON request");
        assert_eq!(json["message_thread_id"], 42);
    }

    #[test]
    fn single_upload_contains_exactly_one_topic_field() {
        let path = temp_upload_file("single-topic", b"image-bytes");
        let (address, server) = upload_http_server(
            None,
            br#"{"ok":true,"result":{"message_id":7,"photo":[{"file_id":"p7"}]}}"#.to_vec(),
        );
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");
        let request = album_upload_request(&path, "topic.jpg");
        let outcome = block_on(transport.send_upload(
            &token,
            &request,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            |_| {},
        ));
        assert!(outcome.expect("upload").ok);
        let received = server.join().expect("server");
        let parsed = parse_multipart_request(&received).unwrap_or_else(|error| {
            panic!(
                "{error}; raw request: {}",
                String::from_utf8_lossy(&received)
            )
        });
        assert_eq!(parsed.fields["message_thread_id"], "42");
        assert_eq!(
            String::from_utf8_lossy(&received)
                .matches("name=\"message_thread_id\"")
                .count(),
            1
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_control_request_with_a_lost_response_is_not_safe_to_retry() {
        let (endpoint, server) = fake_server("not-json");
        let transport = ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
        let token = BotToken::new("123:secret").expect("token");
        let error = transport
            .send(
                &token,
                TelegramRequest::Message(SendMessageRequest {
                    chat_id: "-100".into(),
                    message_thread_id: Some(42),
                    text: "send once".into(),
                    disable_web_page_preview: true,
                }),
            )
            .expect_err("invalid response after request is sent");
        let request = server.join().expect("server");
        assert!(request.contains("send once"));
        assert_eq!(
            classify_send_failure(&error, RequestProgress::Sent),
            SendFailure::Unknown
        );
    }

    #[test]
    fn sends_photo_video_and_media_group_methods() {
        for (request, method) in [
            (
                TelegramRequest::Photo(SendPhotoRequest {
                    chat_id: "-100".into(),
                    message_thread_id: Some(42),
                    photo: "photo-id".into(),
                    caption: Some("caption".into()),
                }),
                "sendPhoto",
            ),
            (
                TelegramRequest::Video(SendVideoRequest {
                    chat_id: "-100".into(),
                    message_thread_id: Some(42),
                    video: "video-id".into(),
                    caption: None,
                }),
                "sendVideo",
            ),
            (
                TelegramRequest::MediaGroup(SendMediaGroupRequest {
                    chat_id: "-100".into(),
                    message_thread_id: Some(42),
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
                    message_thread_id: None,
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
                    message_thread_id: None,
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
                message_thread_id: None,
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
            message_thread_id: Some(42),
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
    fn verified_bot_identity_is_stable_across_credentials_and_rejects_non_bots() {
        struct IdentityTransport(serde_json::Value);
        impl TelegramTransport for IdentityTransport {
            fn send(
                &self,
                _token: &BotToken,
                request: TelegramRequest,
            ) -> Result<TelegramResponse, TelegramError> {
                assert_eq!(request, TelegramRequest::GetMe);
                serde_json::from_value(serde_json::json!({"ok":true,"result":self.0}))
                    .map_err(json_error)
            }
        }
        let transport =
            IdentityTransport(serde_json::json!({"id":1234567890123_i64,"is_bot":true}));
        let first = BotToken::new("first-token").expect("token");
        let second = BotToken::new("second-token").expect("token");
        assert_eq!(
            verify_bot_identity(&transport, &first).expect("identity"),
            "telegram-bot:1234567890123"
        );
        assert_eq!(
            verify_bot_identity(&transport, &first).unwrap(),
            verify_bot_identity(&transport, &second).unwrap()
        );
        for result in [
            serde_json::json!({"id":1,"is_bot":false}),
            serde_json::json!({"id":0,"is_bot":true}),
            serde_json::json!({"id":-1,"is_bot":true}),
            serde_json::json!({"id":"1","is_bot":true}),
            serde_json::json!({"is_bot":true}),
        ] {
            assert!(verify_bot_identity(&IdentityTransport(result), &first).is_err());
        }
        let (method, payload) = request_payload(TelegramRequest::GetMe).expect("request");
        assert_eq!(method, "getMe");
        assert_eq!(payload, serde_json::json!({}));
        let (method, payload) = request_payload(TelegramRequest::GetChat {
            chat_id: "-100123".into(),
        })
        .unwrap();
        assert_eq!(method, "getChat");
        assert_eq!(payload, serde_json::json!({"chat_id":"-100123"}));
        assert!(request_payload(TelegramRequest::GetChat { chat_id: "".into() }).is_err());
        assert_eq!(
            request_payload(TelegramRequest::LogOut).unwrap().0,
            "logOut"
        );
        assert_eq!(request_payload(TelegramRequest::Close).unwrap().0, "close");
        assert_eq!(
            request_payload(TelegramRequest::DeleteWebhook).unwrap(),
            (
                "deleteWebhook",
                serde_json::json!({"drop_pending_updates": false})
            )
        );
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

    #[test]
    fn migration_source_and_target_apply_endpoint_specific_proxy_policy() {
        // A Cloud→Local migration builds two transports from the same
        // resolved proxy value. The local leg addresses the operator's own
        // loopback server, so a configured proxy must be dropped there and
        // kept for cloud; otherwise the token would travel through a proxy
        // that has no reason to see it, or the local call would leave the
        // machine.
        let proxy = Some("http://user:secret@127.0.0.1:3128".to_owned());
        let source = ReqwestTelegramTransport::with_api_endpoint(
            TelegramEndpoint::cloud(),
            Duration::from_secs(5),
            proxy.clone(),
        )
        .expect("cloud source transport");
        let target = ReqwestTelegramTransport::with_api_endpoint(
            TelegramEndpoint::parse(EndpointMode::Local, "http://127.0.0.1:8081/").expect("local"),
            Duration::from_secs(5),
            proxy.clone(),
        )
        .expect("local target transport");

        let cloud_debug = format!("{source:?}");
        let local_debug = format!("{target:?}");
        assert!(
            cloud_debug.contains("disable_proxy: false"),
            "cloud leg must keep the configured proxy: {cloud_debug}"
        );
        assert!(
            local_debug.contains("disable_proxy: true"),
            "local leg must pin a direct connection: {local_debug}"
        );
        // The local leg must not retain the proxy value at all, and neither
        // leg may expose proxy credentials through Debug.
        assert!(
            !local_debug.contains("3128") && !cloud_debug.contains("secret"),
            "proxy credentials must never reach Debug output: {cloud_debug} / {local_debug}"
        );
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

    struct ParsedMultipartFile {
        file_name: String,
        bytes: Vec<u8>,
    }

    struct ParsedMultipartRequest {
        fields: std::collections::HashMap<String, String>,
        files: std::collections::HashMap<String, ParsedMultipartFile>,
    }

    fn parse_multipart_request(request: &[u8]) -> Result<ParsedMultipartRequest, String> {
        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| "request headers are incomplete".to_owned())?;
        let headers =
            std::str::from_utf8(&request[..header_end]).map_err(|error| error.to_string())?;
        let content_type = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-type")
                    .then_some(value.trim())
            })
            .ok_or_else(|| "missing content-type".to_owned())?;
        let boundary = content_type
            .split(';')
            .map(str::trim)
            .find_map(|parameter| parameter.strip_prefix("boundary="))
            .ok_or_else(|| "missing multipart boundary".to_owned())?;
        let marker = format!("--{boundary}").into_bytes();
        let body = &request[header_end + 4..];
        let mut fields = std::collections::HashMap::new();
        let mut files = std::collections::HashMap::new();

        let mut cursor = 0;
        while let Some(relative_start) = body[cursor..]
            .windows(marker.len())
            .position(|window| window == marker)
        {
            let part_start = cursor + relative_start + marker.len();
            if body.get(part_start..part_start + 2) == Some(b"--") {
                break;
            }
            let part_start = part_start
                .checked_add(2)
                .ok_or_else(|| "invalid multipart boundary offset".to_owned())?;
            let header_offset = body[part_start..]
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .ok_or_else(|| "multipart part headers are incomplete".to_owned())?;
            let part_headers = std::str::from_utf8(&body[part_start..part_start + header_offset])
                .map_err(|error| error.to_string())?;
            let content_start = part_start + header_offset + 4;
            let next = body[content_start..]
                .windows(marker.len() + 2)
                .position(|window| window.starts_with(b"\r\n") && window[2..] == *marker)
                .map(|offset| content_start + offset)
                .ok_or_else(|| "multipart part terminator is missing".to_owned())?;
            let content = &body[content_start..next];
            let disposition = part_headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-disposition")
                        .then_some(value.trim())
                })
                .ok_or_else(|| "missing content-disposition".to_owned())?;
            let parameter = |name: &str| -> Option<String> {
                let prefix = format!("{name}=\"");
                disposition.split(';').map(str::trim).find_map(|item| {
                    let (key, value) = item.split_once('=')?;
                    if key.eq_ignore_ascii_case(name) {
                        return Some(value.trim().trim_matches('"').to_owned());
                    }
                    item.strip_prefix(&prefix)
                        .map(|value| value.trim_end_matches('"').to_owned())
                })
            };
            let name = parameter("name").ok_or_else(|| "missing part name".to_owned())?;
            if let Some(file_name) = parameter("filename") {
                files.insert(
                    name,
                    ParsedMultipartFile {
                        file_name,
                        bytes: content.to_vec(),
                    },
                );
            } else {
                fields.insert(
                    name,
                    String::from_utf8(content.to_vec()).map_err(|error| error.to_string())?,
                );
            }
            cursor = next + 2;
        }
        Ok(ParsedMultipartRequest { fields, files })
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

        fn renew_outbox_claim(
            &self,
            _claim_token: &str,
            _lease_until: &str,
            _now: &str,
        ) -> Result<bool, SendStateError> {
            Ok(true)
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
    fn clock_aware_attempt_reads_time_before_and_after_io() {
        let store = RecordingOutbox::default();
        let reads = std::cell::Cell::new(0);
        block_on(run_claimed_attempt_with_clock(
            &store,
            "claim-1",
            0,
            || {
                reads.set(reads.get() + 1);
                format!("time-{}", reads.get())
            },
            schedule_label,
            async {
                assert_eq!(reads.get(), 1);
                Ok(SendAttemptSuccess {
                    telegram_message_id: "88".into(),
                    results_json: None,
                })
            },
        ))
        .unwrap();
        assert_eq!(reads.get(), 2);
        assert_eq!(store.writes()[0], "started");
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

    #[test]
    fn media_planner_splits_document_runs_without_reordering() {
        let mut items: Vec<_> = (0..5).map(planned_item).collect();
        items[1].media_kind = MediaKind::Video;
        items[2].media_kind = MediaKind::Document;
        items[3].media_kind = MediaKind::Document;
        let plans = plan_media_sends("-100", Some(42), items.clone(), "mixed");
        assert_eq!(plans.len(), 3);
        let groups: Vec<_> = plans
            .iter()
            .map(|plan| match &plan.payload {
                SendPayload::Media { items, .. } => items.clone(),
                _ => panic!("media plan expected"),
            })
            .collect();
        assert_eq!(
            groups.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![2, 2, 1]
        );
        assert_eq!(groups.into_iter().flatten().collect::<Vec<_>>(), items);
        assert_eq!(plans[2].message_kind, "media_single");
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
            message_thread_id: Some(42),
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
        let body = br#"{"ok":true,"result":[{"message_id":11,"photo":[{"file_id":"a1"}]},{"message_id":12,"photo":[{"file_id":"a2"}]}]}"#
            .to_vec();
        let (address, server) = upload_http_server(None, body);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let token = BotToken::new("1234:TEST").expect("token");

        let items = vec![
            AlbumItemUpload::File(Box::new(album_upload_request(&first, "album-a.jpg"))),
            // The second item reuses an identifier already cached for this bot.
            AlbumItemUpload::CachedFileId {
                media_kind: MediaKind::Photo,
                file_id: "cached-file-id".to_owned(),
                caption: Some("cached caption".to_owned()),
            },
        ];
        let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&stages);
        let success = block_on(send_media_group_attempt(
            &transport,
            &token,
            "-1001",
            Some(42),
            &items,
            &UploadTimeouts::default(),
            &CancellationToken::new(),
            move |stage| observed.lock().expect("stages").push(stage),
        ))
        .expect("album attempt");
        assert_eq!(success.telegram_message_id, "11", "first confirmed message");
        let results = success.results_json.expect("album results");
        let parsed_results: serde_json::Value =
            serde_json::from_str(&results).expect("structured album results");
        assert_eq!(
            parsed_results["message_ids"],
            serde_json::json!(["11", "12"])
        );
        assert_eq!(parsed_results["file_ids"], serde_json::json!(["a1", "a2"]));

        let received = server.join().expect("server");
        let request_text = String::from_utf8_lossy(&received).to_ascii_lowercase();
        assert!(request_text.contains("/bot1234:test/sendmediagroup"));
        let request = parse_multipart_request(&received).unwrap_or_else(|error| {
            panic!(
                "{error}; raw request: {}",
                String::from_utf8_lossy(&received)
            )
        });
        assert_eq!(request.fields["chat_id"], "-1001");
        assert_eq!(request.fields["message_thread_id"], "42");
        let media: serde_json::Value =
            serde_json::from_str(&request.fields["media"]).expect("media JSON field");
        assert_eq!(media.as_array().expect("media array").len(), 2);
        assert_eq!(media[0]["media"], "attach://media_0");
        assert_eq!(media[1]["media"], "cached-file-id");
        assert_eq!(media[1]["caption"], "cached caption");
        assert_eq!(request.files["media_0"].file_name, "album-a.jpg");
        assert_eq!(request.files["media_0"].bytes, b"first-item-bytes");

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
            None,
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
            None,
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
            None,
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
