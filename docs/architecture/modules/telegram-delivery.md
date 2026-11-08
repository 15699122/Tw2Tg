## telegram-delivery

- **Path / entry:** `crates/xarchive-telegram/src/delivery.rs`; entered through
  the crate-root `pub use crate::delivery::{...}`.
- **Responsibility:** Record a send before it is attempted, persist its outcome
  afterwards and never duplicate a delivery.
- **Owner:** Cross-platform
- **Depends on:** `crate::TelegramError` and the `TelegramError` variants the
  idempotent send maps onto its own error type.
- **Depended on by:** the sender loop in the Desktop delivery path and the
  Storage `SendStateStore` implementation.
- **Public contract:** `SendState`, `SentSendRecord`, `PendingSendRecord`,
  `DeliveredSend`, `SendStateError`, `SendStateStore`,
  `IdempotentSendOutcome`, `IdempotentSendError`, `send_idempotently`. The
  `SCREAMING_SNAKE_CASE` persisted states, the store method signatures and the
  "already sent returns without invoking the closure" behaviour are unchanged.
- **Compatibility entry:** crate-root re-export only.
- **State and ownership:** the durable send record is owned by the caller's
  `SendStateStore`; this module holds no in-memory lease and performs no I/O
  itself beyond calling the store.
- **Failure behaviour:** a failed send persists its failure so `list_unsent`
  can drive a later retry; an already-recorded send is never re-sent; a store
  error is surfaced instead of being treated as success.
- **Tests:** the idempotent-send and send-state suites in the crate's inline
  test module.
- **Gates to re-run:** `cargo test -p xarchive-telegram`, `cargo test -p
  xarchive-storage --lib` (the outbox store uses this contract), Desktop lib.
- **Exceptions:** none.
- **Open questions:** the blocking transport and the upload path still live in
  `lib.rs` and are the next R3 slices.