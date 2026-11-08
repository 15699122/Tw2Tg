# telegram-planning

- **Path / entry:** `crates/xarchive-telegram/src/planning.rs`; entered through the
  crate-root `pub use crate::planning::{PlannedMediaItem, PlannedSend, SendPayload,
  plan_media_sends, plan_text_send}`.
- **Responsibility:** Turn archived content into the deterministic units of
  Telegram work (TG-03/TG-04) and shape the persisted payload snapshot. Grouping
  rules (albums of 2–10, document-vs-media compatibility, trailing single item
  promoted out of an album) live here.
- **Owner:** Cross-platform
- **Depends on:** `crate::delivery::SendStateError`, `crate::formatting::MediaKind`,
  `crate::outbox::NewOutboxEntry`, and the crate-root constants
  `TELEGRAM_ALBUM_MIN_ITEMS`, `TELEGRAM_MEDIA_GROUP_LIMIT` plus the private
  `sha256_hex` fingerprint helper.
- **Depended on by:** Desktop `telegram_send.rs` (which builds the plan before
  queueing) and the Storage outbox via the `NewOutboxEntry` it produces.
- **Public contract:** `PlannedMediaItem`, `SendPayload`, `PlannedSend` (with
  `fingerprint`, `to_outbox_entry`), `plan_text_send`, `plan_media_sends`. The
  `snake_case` / `kind`-tagged serde representation of `SendPayload` and the
  idempotency-key shape are unchanged by the move.
- **Compatibility entry:** crate-root re-export; no path changed for callers.
- **State and ownership:** pure. No clock, no I/O, no transport, no lease. It
  computes a fingerprint and a snapshot; it never mutates an outbox row.
- **Failure behaviour:** fail closed. `to_outbox_entry` rejects an unsafe
  archive-relative path or a media reference outside the committed archive
  directory with `SendStateError::InvalidPayloadPath`, so a machine-local or
  escaping path is never persisted. The fingerprint covers portable media facts
  only, not runtime file paths, so moving an archive does not create a new send
  intent.
- **Tests:** the planning, fingerprint, album-grouping and payload-path suites in
  the crate's inline test module.
- **Gates to re-run:** `cargo test -p xarchive-telegram`; Desktop `--lib`
  (planning and queueing paths).
- **Exceptions:** none.
- **Open questions:** none.
