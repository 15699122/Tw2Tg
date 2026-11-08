## telegram-retry-policy

- **Path / entry:** `crates/xarchive-telegram/src/retry_policy.rs`; entered
  through the crate-root `pub use crate::retry_policy::{...}`.
- **Responsibility:** Decide what a failed send means and which durable
  transition must be recorded for it (plan TG-04).
- **Owner:** Cross-platform
- **Depends on:** `crate::TelegramError`, `crate::{UNKNOWN_REASON_*}`.
- **Depended on by:** the sender loop and the Desktop/Storage outbox paths that
  persist a transition after an attempt.
- **Public contract:** `RequestProgress`, `SendFailure`, `OutboxDecision`,
  `TELEGRAM_DEFAULT_RETRY_AFTER`, `bounded_backoff`, `classify_send_failure`,
  `decide_outbox_transition`. The 5 s→30 min bounded backoff, the 429
  `retry_after` preference and every `UNKNOWN_REASON_*` string are unchanged.
- **Compatibility entry:** crate-root re-export only.
- **State and ownership:** none. The module reads a classified error and an
  attempt number; the sender loop owns the lease and the persisted transition.
- **Failure behaviour:** classification is never optimistic — a request that may
  have been accepted becomes `SendFailure::Unknown` (persisted as `UNKNOWN`, not
  retried), and `RequestProgress` must come from the transport's own tracking
  rather than an assumption.
- **Tests:** `classifies_send_failures_into_the_retry_policy`,
  `bounded_backoff_grows_then_caps`,
  `attempt_driver_persists_the_retry_policy_of_each_failure_class` and the
  outbox-transition suites in the crate's inline test module.
- **Gates to re-run:** `cargo test -p xarchive-telegram`, `cargo test -p
  xarchive-storage --lib` (retry/UNKNOWN persistence).
- **Exceptions:** none.
- **Open questions:** none.