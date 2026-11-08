# desktop-commands-sidecar

- **Path / entry:** `desktop/src-tauri/src/commands/sidecar.rs`; reached through
  `crate::commands::sidecar`, declared by `commands.rs` as
  `pub(crate) mod sidecar;`.
- **Responsibility:** Starting and stopping the protocol-v2 Sidecar worker.
- **Owner:** Cross-platform (process containment details of the supervisor remain
  platform-owned).
- **Depends on:** `crate::RuntimeState`, `xarchive_sidecar_supervisor`
  (`SidecarSupervisor`, `ChildProcessEnvironment`), `crate::config::AppConfig`,
  `crate::portable::PortablePaths`, and `crate::commands::parse_sidecar_args`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, naming
  `commands::sidecar::start_sidecar` and `commands::sidecar::stop_sidecar`.
- **Public contract:** the command names, the `"ready"` / `"stopped"` status
  strings, every error string, and the environment-variable overrides
  (`XARCHIVE_SIDECAR_PROGRAM`, `XARCHIVE_SIDECAR_ARGS`). Unchanged by the move.
- **What stayed in the parent and why:** `sidecar_status` and
  `refresh_sidecar_state` stay in `commands.rs` because `app_status` uses them;
  `parse_sidecar_args` stays there because `lib.rs` and `runtime.rs` reach it as
  `commands::parse_sidecar_args`. The child imports `parse_sidecar_args` through
  the parent, so no visibility was widened.
- **State and ownership:** owns no state. It mutates `RuntimeState.sidecar`,
  `sidecar_error` and logs diagnostics through the existing debug/error hooks.
- **Failure behaviour:** a missing worker returns
  "XArchive Sidecar worker was not found"; a handshake failure stores
  `sidecar_error` and returns the same message. A stop that cannot terminate the
  worker restores the supervisor in state and returns the error instead of
  pretending it stopped.
- **Ordering guarantees:** `Direct` proxy mode also removes the inherited proxy
  environment variables, so a stopped proxy is not silently re-introduced through
  gallery-dl.
- **Tests:** the shell-argument and lifecycle behaviour is covered by the Desktop
  lib suite and the supervisor crate's own tests.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`; supervisor unit and protocol suites.
- **Exceptions:** none; 119 lines.
- **Open questions:** none.
