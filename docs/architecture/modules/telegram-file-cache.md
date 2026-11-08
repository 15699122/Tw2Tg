# telegram-file-cache

- **Path / entry:** `crates/xarchive-telegram/src/file_cache.rs`; entered through
  the crate-root `pub use crate::file_cache::{CachedFileId, FileCacheKey,
  FileIdCacheStore, TELEGRAM_FILE_CACHE_VERSION, is_invalid_file_id_error}`.
- **Responsibility:** Own the bot-isolated `file_id` cache contract (TG-05): the
  key shape, the confirmed-value record, the store contract and the rule that
  decides when a cached identifier may be invalidated.
- **Owner:** Cross-platform
- **Depends on:** `crate::contract::TelegramError`, `crate::delivery::SendStateError`,
  `crate::formatting::MediaKind`, `serde`.
- **Depended on by:** Desktop `telegram_send.rs` (cache lookup before upload,
  store after confirmation, delete on an explicit invalid-id answer) and the
  Storage cache implementation.
- **Public contract:** `TELEGRAM_FILE_CACHE_VERSION`, `FileCacheKey`,
  `CachedFileId`, `FileIdCacheStore`, `is_invalid_file_id_error`. The serde
  representation, the key composition (`bot_identity` + `content_sha256` +
  `media_kind` + `representation_version`) and every trait method signature are
  unchanged by the move.
- **Compatibility entry:** crate-root re-export; no path changed for callers.
- **State and ownership:** no I/O and no connection. The store trait is the only
  persistence boundary; the key is bot-scoped, so one bot's identifier is never
  handed to another.
- **Failure behaviour:** conservative on purpose. `is_invalid_file_id_error`
  returns true only for Telegram's explicit "this `file_id` is unusable" answers
  on 400/404; a permission, network or generic server failure never invalidates a
  cache entry. A cache miss falls back to uploading the original file, never to
  reusing another bot's id. `file_unique_id` is identification only and is never
  a send parameter.
- **Tests:** the cache-key, bot-isolation and invalidation-rule suites in the
  crate's inline test module, plus the Storage cache suites.
- **Gates to re-run:** `cargo test -p xarchive-telegram`;
  `cargo test -p xarchive-storage --lib`; Desktop `--lib` (upload reuse path).
- **Exceptions:** none.
- **Open questions:** none.
