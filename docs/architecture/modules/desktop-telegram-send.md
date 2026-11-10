# desktop-telegram-send

- **Path / entry:** `desktop/src-tauri/src/telegram_send.rs` plus child modules
  `telegram_send/{credentials,intent_planning,payload,delivery,runner}.rs`.
- **Responsibility:** turn a finished archive into queued Telegram sends and execute
  due outbox entries under a bounded claim loop.
- **Owner:** Cross-platform Owner.
- **Depends on:** `xarchive_telegram` (planning, transport, outbox store, file cache),
  `xarchive_storage` (`Database`, `FileStore`), and `crate::config`/`crate::clock`.
- **Depended on by:** `archive.rs` (queueing behind `auto_send_enabled`) and
  `telegram_worker.rs` (active archived sends).
- **Public contract:** unchanged. Every previously public item still resolves as
  `crate::telegram_send::<name>` through parent re-exports; no item was promoted.
- **Compatibility entry:** `mod telegram_send;` is crate-private and the parent
  re-exports `pub use credentials::*`, `intent_planning::*`, `payload::*`,
  `runner::*` for the pre-existing public surface, plus `pub(crate) use` for
  `run_active_archived_sends_cooperative` (called by `telegram_worker.rs`).
- **State and ownership:** the credential generation is captured and re-checked
  after the secret is fetched; a queued row keeps its immutable persisted snapshot;
  `UNKNOWN` rows are never picked up automatically.
- **Failure behaviour:** disabled or migration-pending sending reads no secret and
  claims nothing; a failed activation deletes the newly stored secret; the file
  cache is written only from a confirmed success and evicted only on an explicit
  dead-identifier answer.
- **Visibility:** all parent-to-child bridges are `pub(in crate::telegram_send)`,
  the narrowest scope that lets the parent's own tests and sibling modules reach
  them. Nothing became `pub` or `pub(crate)`.
- **Tests:** `telegram_send::tests` (30 tests, 0 ignored), unmoved, so every path is
  unchanged.
- **Gates to re-run:** Telegram send module regression; Desktop lib regression.
- **Exceptions:** none.
- **Related:** [telegram planning](telegram-planning.md),
  [telegram delivery](telegram-delivery.md),
  [telegram upload](telegram-upload.md).
