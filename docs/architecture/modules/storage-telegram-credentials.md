## storage-telegram-credentials

- **Path / entry:** `crates/xarchive-storage/src/telegram_credentials.rs`;
  inherent `impl Database` block reached through the public `Database` API.
- **Responsibility:** Own the Telegram credential-generation fence: rotation,
  resume authorization, generation-scoped claiming and the writes that record
  each decision.
- **Owner:** Cross-platform
- **Depends on:** `crate::{Database, StorageError}`, `rusqlite`
  (`OptionalExtension`, `params!`), `serde_json`,
  `xarchive_telegram` store traits.
- **Depended on by:** Desktop Telegram control/delivery paths that prepare,
  activate, retire or review a credential generation, and the outbox worker that
  must be fenced to the active generation.
- **Public contract:** `claim_due_outbox_for_generation`,
  `prepare_telegram_credential_generation`,
  `activate_telegram_credential_generation`,
  `active_telegram_credential_generation`,
  `activate_telegram_credential_with_resume`,
  `retire_telegram_credential_generation`, `record_telegram_rotation_decision`,
  `telegram_rotation_decision`, `telegram_resume_candidates`,
  `authorize_telegram_resume`, `telegram_resume_authorized`,
  `review_telegram_unknown_for_resend`. Method names, signatures, SQL, error
  mapping and the `xarchive_telegram::SendStateError` conversions are identical
  to the pre-move implementation.
- **Compatibility entry:** the methods remain inherent methods on `Database`, so
  every call site and trait implementation is unchanged.
- **State and ownership:** persists the generation fence only. No worker lease
  is held in memory here; a competing worker is excluded by the SQLite write
  transaction that the caller's claim already used.
- **Failure behaviour:** fail closed. A stale or retired generation claims
  nothing, a conflicting rotation decision is refused rather than overwritten,
  and an `UNKNOWN` send outcome is never picked up automatically.
- **Tests:** `tests::telegram_credentials::*` (11 suites covering atomic
  activation, cross-connection races, rotation eligibility, resume grants,
  unknown-outcome review and generation-fenced claims).
- **Gates to re-run:** `cargo test -p xarchive-storage --lib`, plus the Desktop
  Telegram delivery tests.
- **Exceptions:** none.
- **Open questions:** none.
