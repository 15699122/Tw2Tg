# desktop-runtime

- **Path / entry:** `desktop/src-tauri/src/runtime.rs` plus test modules
  `runtime/tests/{startup_recovery,telegram_provider,executor_transport,sidecar_args}.rs`.
- **Responsibility:** own the process-level runtime state object — configuration
  snapshot, database, executor handle, transport, Telegram sender/worker lifecycle,
  secret store adapter, and orderly shutdown.
- **Owner:** Cross-platform Owner.
- **Depends on:** `AppConfig`, `Database`, the executor, the websocket transport,
  `xarchive_telegram` sender and secret store.
- **Depended on by:** application bootstrap and every Tauri command.
- **Public contract:** unchanged.
- **Boundary that must not move:** `RuntimeState` is the composition root. Its
  lifecycle (build → start → replace executor → stop worker → drop) is one control
  boundary, so it is not split; only inline tests were classified.
- **State and ownership:** one `RuntimeState` owns the executor handle, transport and
  Telegram worker; replacement waits for in-flight work and `Drop` performs shutdown.
- **Failure behaviour:** unchanged. Recovery refusals still surface in application
  logs without raw errors, and fatal startup recovery still propagates.
- **Tests:** 11 tests moved into four groups (startup recovery 2, telegram provider 5,
  executor transport 2, sidecar args 2). A separate module,
  `browser_tool_compatibility_tests`, sits mid-file and was left in place because it
  is already a standalone `#[cfg(test)]` module adjacent to the functions it covers.
- **Gates to re-run:** Desktop lib regression (365/0/13).
- **Exceptions:** none.
- **Open questions:** none.
