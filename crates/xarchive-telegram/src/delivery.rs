//! Send state machine and idempotent delivery.
//!
//! A send is recorded before it is attempted and its persisted outcome is read
//! after, so a crashed or restarted sender never silently duplicates a delivery.
//! Persisted states, keys, error variants and the "skip when already sent"
//! behaviour are unchanged by the move.

use crate::TelegramError;

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
