# telegram-outbox

- **Path / entry:** `crates/xarchive-telegram/src/outbox.rs`; entered through the
  crate-root `pub use crate::outbox::{NewOutboxEntry, OutboxEntry, OutboxState,
  RunAttemptError, SendAttemptError, SendAttemptSuccess, TelegramOutboxStore,
  run_claimed_attempt, run_claimed_attempt_with_clock}`.
- **Responsibility:** Own the persisted send lifecycle and its store contract
  (TG-04), plus the claimed-attempt driver that fences, classifies and records
  one attempt's outcome. The claim/lease/`UNKNOWN` semantics live here.
- **Owner:** Cross-platform
- **Depends on:** `crate::contract::TelegramError`, `crate::delivery::SendStateError`,
  and `crate::retry_policy` (`RequestProgress`, `SendFailure`, `OutboxDecision`,
  `classify_send_failure`, `decide_outbox_transition`).
- **Depended on by:** `planning` (`NewOutboxEntry` is what a plan produces), the
  Storage outbox implementation, and Desktop `telegram_send.rs` /
  `telegram_worker.rs` which drive claims and attempts.
- **Public contract:** `OutboxState` (with `as_str`, `parse`, `auto_sendable`,
  `is_terminal`), `OutboxEntry`, `NewOutboxEntry`, `TelegramOutboxStore`,
  `SendAttemptSuccess`, `SendAttemptError` (with `new`, `classification`),
  `RunAttemptError`, `run_claimed_attempt`, `run_claimed_attempt_with_clock`.
  The `SCREAMING_SNAKE_CASE` serde representation of `OutboxState`, every trait
  method signature and every error variant are unchanged by the move.
- **Compatibility entry:** crate-root re-export; no path changed for callers.
  Nothing gained visibility outside the crate.
- **State and ownership:** no connection and no network I/O. The store trait is
  the only persistence boundary, and the driver never writes without the claim
  token it was handed, so a worker whose lease expired cannot overwrite newer
  facts.
- **Failure behaviour:** fail closed. A lost claim surfaces as
  `RunAttemptError::StaleClaim` and the caller's result must be discarded. A
  failure whose request may have reached Telegram resolves to `UNKNOWN`, never to
  an automatic re-send. `auto_sendable` deliberately excludes `UNKNOWN`.
- **Tests:** the outbox-state, claim/lease, retry-transition and
  attempt-driver suites in the crate's inline test module, plus the Storage
  outbox suites that implement this trait.
- **Gates to re-run:** `cargo test -p xarchive-telegram`;
  `cargo test -p xarchive-storage --lib`; Desktop `--lib` (worker and sender).
- **Exceptions:** none. Any change to claim or `UNKNOWN` semantics is a shared
  behaviour change and must be marked `CROSS_PLATFORM_CHANGE_REQUIRED`.
- **Open questions:** none.
