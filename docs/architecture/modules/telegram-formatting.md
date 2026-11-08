## telegram-formatting

- **Path / entry:** `crates/xarchive-telegram/src/formatting.rs`; entered through
  the crate root re-export
  `pub use crate::formatting::{...}` in `crates/xarchive-telegram/src/lib.rs`.
- **Responsibility:** Turn validated message input into deterministic Telegram
  text and plan media items into sendable album/single units.
- **Owner:** Cross-platform
- **Depends on:** `serde` (derive for `MediaKind` / `UploadMode`), and the
  crate-root protocol constants `TELEGRAM_TEXT_LIMIT`,
  `TELEGRAM_MEDIA_GROUP_LIMIT`, `TELEGRAM_ALBUM_MIN_ITEMS`, plus
  `MediaGroupItem`.
- **Depended on by:** `crate::lib` transport and upload code, and Desktop
  callers via the crate root.
- **Public contract:** `MetadataInput`, `format_metadata`, `split_text`,
  `split_for_telegram`, `MediaKind`, `UploadMode`, `MediaGroupSend`,
  `classify_media`, `stable_upload_file_name`, `media_send_plan`,
  `media_groups`. The `MediaKind` / `UploadMode` `snake_case` serde
  representation and every rendered string (metadata layout, upload file-name
  shape, projection labels) are unchanged by the R1 move.
- **Compatibility entry:** the crate root keeps the same item names, so
  `xarchive_telegram::format_metadata` and friends are unchanged for callers.
- **State and ownership:** none. No credential, lease, permit, writer, session
  or transport state is read or produced here.
- **Failure behaviour:** total functions only. Unrecognised MIME types and
  extensions fall back to `MediaKind::Document`; host-pathile input is
  sanitised into a stable file name rather than rejected or passed through.
- **Tests:** `formatting::tests` (`formats_metadata_and_continuations`,
  `groups_media_in_telegram_limit`, `classifies_media_by_mime_then_extension`,
  `builds_stable_upload_file_names`,
  `promotes_a_trailing_single_item_out_of_an_album`).
- **Gates to re-run:** `cargo test -p xarchive-telegram`,
  `cargo test -p xarchive-desktop --lib`. No V1 publication or writer gate is
  affected because no resource ownership moves.
- **Exceptions:** none in `architecture-exceptions.json` for this file.
- **Open questions:** none.
