# telegram-upload

- **Path / entry:** `crates/xarchive-telegram/src/upload.rs`; entered through the
  crate-root `pub use crate::upload::{AlbumItemUpload, TELEGRAM_UPLOAD_CHUNK_BYTES,
  UploadRequest, UploadStage, UploadTimeouts, send_media_group_attempt}`.
- **Responsibility:** Own the async streaming upload half of the Bot API
  transport: layered upload timeouts, ordered phase reporting, the pull-based
  body stream, the album watchdog, progress tracking and the per-media and album
  send calls.
- **Owner:** Cross-platform
- **Depends on:** `reqwest` (async client), `tokio_util` reader stream,
  `futures_core`, `crate::transport` (the transport type, `method_url`, the
  `pub(crate)` proxy fields, `TELEGRAM_MAX_RESPONSE_BYTES`), `crate::formatting`
  (`MediaKind`), `crate::retry_policy` (`RequestProgress`), and the crate-root
  `SendAttemptSuccess` / `SendAttemptError` / `CancellationToken` /
  `TELEGRAM_MEDIA_GROUP_LIMIT` / `TELEGRAM_ALBUM_MIN_ITEMS` contract items.
- **Depended on by:** Desktop `telegram_send.rs` (which drives the album send and
  collects `UploadStage` events) and `runtime.rs` (which stores a stage per job).
- **Public contract:** `UploadRequest`, `UploadTimeouts`, `UploadStage`,
  `AlbumItemUpload`, `TELEGRAM_UPLOAD_CHUNK_BYTES` and
  `send_media_group_attempt`. Field names, the `UploadStage` variants and their
  ordering, and the progress callback signature are unchanged by the move.
- **Compatibility entry:** crate-root re-export; no path changed for callers.
  The internal validators, tracker, watchdog and method-resolution helpers became
  `pub(crate)` so the crate's own test module can keep exercising them; nothing
  became visible outside the crate.
- **State and ownership:** the upload owns per-send phase state, byte counters and
  the streaming body. It holds no persisted outbox row, no lease and no
  credential; the token is borrowed for the call only.
- **Failure behaviour:** fail closed and never optimistic. `Confirmed` is emitted
  only after a parsed `ok: true` result, so "uploaded" is never presented as
  "sent". A cancellation after the request was handed to the wire does not
  produce a retry decision here and is reported as progress-aware failure so the
  caller can record `UNKNOWN`. Response bodies stay bounded by
  `TELEGRAM_MAX_RESPONSE_BYTES`.
- **Tests:** the upload-validation, phase/timeout, tracker/watchdog and album
  suites in the crate's inline test module, which reach the `pub(crate)` helpers
  through explicit test-only imports.
- **Gates to re-run:** `cargo test -p xarchive-telegram`; Desktop
  `cargo test -p xarchive-desktop --lib` (cancellation, progress and album paths).
- **Exceptions:** none. The layered timeouts and the album watchdog are the
  cancellation-sensitive parts of this module and must be re-validated whenever
  the send ordering changes.
- **Open questions:** none.
