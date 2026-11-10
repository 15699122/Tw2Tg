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
mod tests;
