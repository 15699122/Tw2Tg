//! Send-failure classification and the durable retry decision (plan TG-04).
//!
//! This module is the single place that decides what a failed send means. It
//! performs no I/O, holds no lease and never re-sends: the sender loop reports
//! the transport's own progress, then persists exactly the transition decided
//! here.
//!
//! The crate root re-exports every public item, so call sites, persisted
//! reason strings and error mapping are unchanged.

use std::time::Duration;

use crate::{TelegramError, UNKNOWN_REASON_CANCELLED_AFTER_SEND, UNKNOWN_REASON_RESPONSE_LOST};

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
