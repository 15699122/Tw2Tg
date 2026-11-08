# telegram-transport

- **Path / entry:** `crates/xarchive-telegram/src/transport.rs`; entered through
  the crate-root `pub use crate::transport::{ReqwestTelegramTransport,
  TELEGRAM_MAX_RESPONSE_BYTES, validate_message}`.
- **Responsibility:** Own the blocking control-request transport. It builds the
  concrete Bot API client, enforces the endpoint policy, turns a
  `TelegramRequest` into a method plus JSON payload, sends it and classifies the
  outcome. It also owns the response-size bound shared by both send paths.
- **Owner:** Cross-platform
- **Depends on:** `reqwest` (blocking client, Rustls), `serde_json`,
  `crate::contract` (`BotToken`, `TelegramEndpoint`, `EndpointMode`,
  `TelegramRequest`, `TelegramResponse`, `TelegramError`, `TelegramTransport`).
- **Depended on by:** the upload module (`method_url`, the `pub(crate)` proxy
  fields), `retry_policy` consumers that name `TelegramError`, and every Desktop
  caller that constructs `ReqwestTelegramTransport`.
- **Public contract:** `ReqwestTelegramTransport` and its constructors
  (`new`, `with_endpoint`, `with_network`, `with_resolved_proxy`,
  `with_test_endpoint`, `with_api_endpoint`), `TELEGRAM_MAX_RESPONSE_BYTES`,
  `validate_message`. Every constructor signature, the `Debug` output that only
  reports proxy *presence*, and every error variant are unchanged by the move.
- **Compatibility entry:** crate-root re-export; no path changed for callers.
  `method_url`, the two proxy fields, and the payload/validation helpers became
  `pub(crate)` because the upload module and the crate's own tests name them;
  no item gained wider visibility than the crate.
- **State and ownership:** the client, endpoint, proxy decision and proxy value
  live here. The proxy value may carry credentials and is never logged,
  formatted or persisted; `Debug` renders `[REDACTED]`.
- **Failure behaviour:** fail closed. A non-cloud endpoint is refused before a
  client exists (`cloud` requires HTTPS with no userinfo/query/fragment; the
  test-only path requires an explicit loopback address and a port). Redirects
  are disabled because the request URL carries the token. A 2xx with a
  non-Bot-API body becomes `ResponseLost`, never a success.
- **Construction constraint:** the blocking client and `send()` must run outside
  a Tokio runtime context, because reqwest refuses to build its blocking client
  inside one. This is unchanged and is why the transport object is still created
  by callers in synchronous code.
- **Tests:** the endpoint-policy, token-redaction, payload-shape and
  control-transport suites in the crate's inline test module, which reach the
  helpers through the crate root and the test-only imports.
- **Gates to re-run:** `cargo test -p xarchive-telegram`; Desktop
  `--lib` regression, since Desktop constructs this transport directly.
- **Exceptions:** none.
- **Open questions:** none.
