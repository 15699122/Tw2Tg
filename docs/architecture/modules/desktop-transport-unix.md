# desktop-transport-unix

- **Path / entry:** `desktop/src-tauri/src/transport/unix.rs`, declared as
  `#[cfg(unix)] mod unix;` from `transport.rs`.
- **Responsibility:** serve the browser `BrowserRequest`/`BrowserResponse` contract
  over a current-user Unix socket endpoint.
- **Owner:** Cross-platform Owner (protocol and shared adapter). The Windows Named
  Pipe backend in `windows_transport.rs` is Windows-owned and untouched.
- **Depends on:** `BrowserTransportAdapter`, `xarchive_native_host` framing,
  `xarchive_protocol`, `crate::browser_pairing`.
- **Depended on by:** `runtime.rs` (server construction and endpoint derivation) and
  `websocket_transport.rs` tests.
- **Public contract:** unchanged.
  `crate::transport::DesktopTransportServer` and
  `crate::transport::transport_endpoint` keep resolving with their original
  `pub(crate)` reach through `#[cfg(unix)] pub(crate) use`.
- **Compatibility entry:** `transport.rs` keeps `BrowserTransportAdapter` and the
  protocol error mapping (`error_response`, `error_error_code`) in place.
- **State and ownership:** the server owns the accept thread, the stop flag and the
  endpoint path; `Drop` stops the loop, joins the thread and removes the socket.
- **Failure behaviour:** the socket is created `0600` because bootstrap issues
  credentials; a stale endpoint file is replaced before binding; an unusable pairing
  coordinator fails closed with `DESKTOP_NOT_READY`; the accept loop stops admitting
  at `MAX_ACTIVE_CONNECTIONS` and every connection runs under `CONNECTION_TIMEOUT`.
- **Tests:** `transport::*` (9 tests, 0 ignored) stay in `transport.rs`.
- **Gates to re-run:** transport module regression; Desktop lib regression. Unix
  socket results are **not** Windows Named Pipe/ACL acceptance.
- **Exceptions:** none.
- **Related:** [desktop executor runtime](desktop-executor-runtime.md); Windows backend
  work is queued in the Windows validation queue.
