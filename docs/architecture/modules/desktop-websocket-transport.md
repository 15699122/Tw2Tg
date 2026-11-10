# desktop-websocket-transport

- **Path / entry:** `desktop/src-tauri/src/websocket_transport.rs` plus test
  modules
  `websocket_transport/tests/{state_and_defaults,authentication,deadline_and_lifecycle,preauth_limits}.rs`.
- **Responsibility:** accept loopback WebSocket connections from the browser
  extension, authenticate them with a one-time ticket, enforce handshake and read
  deadlines, bound pre-authentication resource use, and report connection state.
- **Owner:** Cross-platform Owner for the shared state machine; the Windows
  named-pipe transport remains Windows-owned and is a separate module.
- **Depends on:** the pairing ticket source, `tokio`/`tungstenite` and the shared
  IPC request handler.
- **Depended on by:** the browser transport adapter and application bootstrap.
- **Public contract:** unchanged.
- **Boundary that must not move:** authentication, deadline accounting, connection
  state and stop semantics share one state machine; splitting them would spread a
  single protocol invariant across modules. Only tests were classified.
- **State and ownership:** the server owns the listener and its live connection set;
  shutdown interrupts both handshake and authenticated idle connections.
- **Failure behaviour:** unchanged. Wrong tickets, oversized first frames, exceeded
  pre-auth connection caps and exceeded byte budgets are still rejected before
  business handling.
- **Tests:** 12 tests moved into four groups (state/defaults 2, authentication 5,
  deadline/lifecycle 3, pre-auth limits 3). Two helpers (`client`, `credentials`)
  stayed in the parent `mod tests`.
- **Gates to re-run:** Desktop lib regression (365/0/13).
- **Exceptions:** none.
- **Open questions:** none.
