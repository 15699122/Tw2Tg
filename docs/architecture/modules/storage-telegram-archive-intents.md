## storage-telegram-archive-intents

- **Path / entry:** `crates/xarchive-storage/src/telegram_archive_intents.rs`;
  inherent `impl Database` block reached through the public `Database` API.
- **Responsibility:** Persist a Telegram archive intent and advance it through
  its explicit state machine.
- **Owner:** Cross-platform
- **Depends on:** `crate::{Database, StorageError, TelegramArchiveIntentRecord}`,
  `rusqlite` (`OptionalExtension`, `params!`).
- **Depended on by:** the archive publishing path that records planning facts
  before a directory rename, and the recovery path that lists intents that still
  need to be sent or replayed.
- **Public contract:** `prepare_telegram_archive_intent`,
  `telegram_archive_intent`, `transition_telegram_archive_intent`,
  `list_recoverable_telegram_archive_intents`. Signatures, SQL, permitted
  transitions and error mapping are unchanged by the move.
- **Compatibility entry:** unchanged inherent methods on `Database`.
- **State and ownership:** journal only. The record carries the planning facts;
  the module holds no session, permit or writer state and never renames a
  directory itself.
- **Failure behaviour:** fail closed. Repeating an identical write is idempotent,
  a changed planning fact is refused instead of redirecting queued work, and an
  invalid transition is an error rather than a silent overwrite.
- **Tests:** `tests::archive_service::archive_intent_journal_recovers_staging_after_database_reopen`,
  `…_renamed_archive_after_database_reopen`,
  `archive_intent_journal_rejects_changed_facts_and_invalid_transitions`,
  `internal_v1_telegram_intent_refuses_recovery_without_runner_permit`.
- **Gates to re-run:** `cargo test -p xarchive-storage --lib` and the V1 intent
  recovery checks in the Linux ledger.
- **Exceptions:** none.
- **Open questions:** none.
