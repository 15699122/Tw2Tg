## telegram-projection

- **Path / entry:** `crates/xarchive-telegram/src/projection.rs`; entered through
  the crate root re-export
  `pub use crate::projection::{SendProjection, message_link, outbox_projection}`.
- **Responsibility:** Map a persisted outbox state to task wording and build a
  message deep link only when Telegram's documented form applies.
- **Owner:** Cross-platform
- **Depends on:** `crate::OutboxState` (persisted `SCREAMING_SNAKE_CASE`
  contract, unchanged).
- **Depended on by:** Desktop `commands.rs` via
  `xarchive_telegram::outbox_projection` / `xarchive_telegram::message_link`.
- **Public contract:** `SendProjection` (with `label()`),
  `outbox_projection(OutboxState) -> SendProjection`,
  `message_link(&str, &str) -> Option<String>`. Label wording is a TG-06
  product contract: no surface may claim client receipt or read state.
- **Compatibility entry:** same names re-exported from the crate root.
- **State and ownership:** none. The module reads a value and returns a value;
  it never advances an outbox row, claim, lease or attempt.
- **Failure behaviour:** `message_link` returns `None` for private chats, basic
  groups, non-`-100` ids, malformed ids and non-positive ids, so callers keep
  the target plus message id instead of concatenating unvalidated input.
- **Tests:** `projection::tests` (`send_projection_labels_never_claim_receipt`,
  `message_link_follows_the_official_form_only`).
- **Gates to re-run:** `cargo test -p xarchive-telegram`,
  `cargo test -p xarchive-desktop --lib`.
- **Exceptions:** none in `architecture-exceptions.json` for this file.
- **Open questions:** none.
