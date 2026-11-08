//! Delivery-state projection policy (plan TG-06).
//!
//! This module turns a persisted [`OutboxState`] into the wording a task surface
//! shows, and builds a message deep link only when Telegram's own rules allow
//! one. Both functions are pure: they read no lease, claim no send and never
//! upgrade an uncertain outcome into "sent".
//!
//! The crate root re-exports every public item here, so existing call sites are
//! unchanged.

use crate::OutboxState;

/// How an outbox state is presented on a task. The Windows GUI renders these
/// labels; the wording rules live here so every surface stays consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendProjection {
    Queued,
    Sending,
    SendConfirmed,
    WaitingToRetry,
    Failed,
    NeedsReview,
    Cancelled,
}

pub fn outbox_projection(state: OutboxState) -> SendProjection {
    match state {
        OutboxState::Queued => SendProjection::Queued,
        OutboxState::InFlight => SendProjection::Sending,
        OutboxState::Sent => SendProjection::SendConfirmed,
        OutboxState::RetryWait => SendProjection::WaitingToRetry,
        OutboxState::FailedPermanent => SendProjection::Failed,
        OutboxState::Unknown => SendProjection::NeedsReview,
        OutboxState::Cancelled => SendProjection::Cancelled,
    }
}

impl SendProjection {
    /// Stable task wording (plan TG-06). Only a confirmed Bot API result is
    /// shown as sent; nothing here ever claims the client received or read
    /// the message, because no separate reliable evidence for that exists.
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Telegram: queued",
            Self::Sending => "Telegram: sending",
            Self::SendConfirmed => "Telegram: send confirmed",
            Self::WaitingToRetry => "Telegram: waiting to retry",
            Self::Failed => "Telegram: send failed",
            Self::NeedsReview => "Telegram: outcome unknown, review required",
            Self::Cancelled => "Telegram: cancelled",
        }
    }
}

/// A deep link to a delivered message when one can be built by Telegram's
/// own rules, otherwise `None` — callers then keep the target and message id
/// instead of concatenating unvalidated input (plan TG-06).
///
/// Only the documented `https://t.me/c/<internal id>/<message id>` form for
/// channel/supergroup chats (`chat_id` starting with `-100`) is produced;
/// private chats and basic groups yield `None`.
pub fn message_link(chat_id: &str, message_id: &str) -> Option<String> {
    let internal = chat_id.strip_prefix("-100")?.parse::<i64>().ok()?;
    let message = message_id.parse::<i64>().ok()?;
    if internal <= 0 || message <= 0 {
        return None;
    }
    Some(format!("https://t.me/c/{internal}/{message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_projection_labels_never_claim_receipt() {
        for state in [
            OutboxState::Queued,
            OutboxState::InFlight,
            OutboxState::Sent,
            OutboxState::RetryWait,
            OutboxState::FailedPermanent,
            OutboxState::Unknown,
            OutboxState::Cancelled,
        ] {
            let label = outbox_projection(state).label();
            let lowered = label.to_lowercase();
            assert!(
                !lowered.contains("received") && !lowered.contains("read"),
                "label {label:?} must not claim client receipt or read state"
            );
        }
        assert_eq!(
            outbox_projection(OutboxState::Sent).label(),
            "Telegram: send confirmed"
        );
        assert_eq!(
            outbox_projection(OutboxState::Unknown).label(),
            "Telegram: outcome unknown, review required"
        );
    }

    #[test]
    fn message_link_follows_the_official_form_only() {
        assert_eq!(
            message_link("-1001234567890", "42"),
            Some("https://t.me/c/1234567890/42".to_owned())
        );
        // Basic groups, private chats, malformed ids and zero ids never get a
        // concatenated link — callers keep target + message id instead.
        assert_eq!(message_link("-12345", "42"), None);
        assert_eq!(message_link("12345", "42"), None);
        assert_eq!(message_link("-100abc", "42"), None);
        assert_eq!(message_link("-1001234567890", "x"), None);
        assert_eq!(message_link("-1000", "42"), None);
        assert_eq!(message_link("-1001234567890", "0"), None);
    }
}
