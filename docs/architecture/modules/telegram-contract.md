## telegram-contract

- **Path / entry:** `crates/xarchive-telegram/src/contract.rs`; entered through
  the crate-root `pub use crate::contract::{...}`.
- **Responsibility:** Define the Bot API wire contract — the redacting token
  handle, request payloads, response envelope, error type and endpoint policy.
- **Owner:** Cross-platform
- **Depends on:** `serde`, `crate::{SecretStoreError, sha256_hex}`.
- **Depended on by:** the transport/upload code in `lib.rs`, `delivery.rs` and
  every Desktop caller that names a request, response or error variant.
- **Public contract:** `BotToken`, `SendMessageRequest`, `SendPhotoRequest`,
  `SendVideoRequest`, `MediaGroupItem`, `SendMediaGroupRequest`,
  `TelegramRequest`, `TelegramTransport`, `verify_bot_identity`,
  `TelegramResponse`, `TelegramError`, `TELEGRAM_CLOUD_API_BASE`,
  `EndpointMode`, `TelegramEndpoint`. All `serde` field names, the
  `SCREAMING_SNAKE_CASE` / `lowercase` representations and every
  `Display`/`Debug` string are unchanged by the move.
- **Compatibility entry:** crate-root re-export only; no path moved for callers.
- **State and ownership:** none. `BotToken` holds the secret but always renders
  `[REDACTED]`; no credential is persisted or logged here.
- **Failure behaviour:** fail closed. `TelegramEndpoint` refuses anything its
  mode does not allow (cloud requires HTTPS; local requires an explicit loopback
  address; userinfo/query/fragment is rejected), so a later request cannot carry
  the token to an unexpected origin.
- **Tests:** the endpoint, token-redaction, transport-contract and API-error
  suites in the crate's inline test module, which import through the root.
- **Gates to re-run:** `cargo test -p xarchive-telegram`; Desktop Telegram
  delivery tests.
- **Exceptions:** none.
- **Open questions:** none.