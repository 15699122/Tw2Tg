//! Telegram outbox: the persisted send lifecycle, its store contract and the
//! claimed-attempt driver (plan TG-04).
//!
//! This module owns one responsibility: decide and record what happened to a
//! planned send. It performs no network I/O — the caller hands in a future that
//! does the attempt — and it never rewrites a row whose claim it no longer owns.
//!
//! The crate root re-exports every public item, so call sites are unchanged.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::contract::TelegramError;
use crate::delivery::SendStateError;
use crate::retry_policy::{
    OutboxDecision, RequestProgress, SendFailure, classify_send_failure, decide_outbox_transition,
};

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
    ///
    /// Entries belonging to one finished archive are a planned sequence: a row
    /// that names an earlier `plan_order` of the same `(bot_identity,
    /// target_scope, tweet_id)` and has not reached `SENT` blocks a later row,
    /// so a paused or `UNKNOWN` unit never lets the rest of the archive jump
    /// ahead (plan §7, Scheme B). Rows without a `tweet_id` are independent.
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

    /// Extend the lease of an in-flight claim while a long upload continues, so
    /// a slow but healthy send is not recovered out from under its worker.
    ///
    /// Returns `false` when the token no longer owns an `IN_FLIGHT` entry (the
    /// lease already expired, another worker reclaimed it, or it reached a
    /// terminal state). The caller must then drop its write instead of
    /// overwriting newer facts.
    fn renew_outbox_claim(
        &self,
        claim_token: &str,
        lease_until: &str,
        now: &str,
    ) -> Result<bool, SendStateError>;

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

    /// Crash recovery: resolve every expired claim. Returns how many entries
    /// were resolved (to `RETRY_WAIT` if the request never started, otherwise
    /// to `UNKNOWN` with `claim_lease_expired`).
    fn recover_outbox_claims(&self, now: &str) -> Result<u64, SendStateError>;

    /// Entries this bot may send now: `QUEUED`/`RETRY_WAIT`, due, ordered by
    /// plan position. Never includes `UNKNOWN`.
    fn list_due_outbox(
        &self,
        bot_identity: &str,
        now: &str,
    ) -> Result<Vec<OutboxEntry>, SendStateError>;
}

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
    run_claimed_attempt_with_clock(
        store,
        claim_token,
        attempt,
        || now.to_owned(),
        schedule,
        execute,
    )
    .await
}

/// Clock-aware driver: read time again after I/O before writing an outcome.
/// The legacy wrapper retains deterministic fixed-time behavior for callers
/// that explicitly provide a captured timestamp.
pub async fn run_claimed_attempt_with_clock<F, S, C>(
    store: &dyn TelegramOutboxStore,
    claim_token: &str,
    attempt: u32,
    clock: C,
    schedule: S,
    execute: F,
) -> Result<SendAttemptSuccess, RunAttemptError>
where
    F: std::future::Future<Output = Result<SendAttemptSuccess, SendAttemptError>>,
    S: FnOnce(Duration) -> String,
    C: Fn() -> String,
{
    let started_at = clock();
    store
        .mark_request_started(claim_token, &started_at)
        .map_err(store_error)?;
    let outcome = execute.await;
    let completed_at = clock();
    let now = completed_at.as_str();
    let failure = match outcome {
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
