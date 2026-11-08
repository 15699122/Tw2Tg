# desktop-commands-extension

- **Path / entry:** `desktop/src-tauri/src/commands/extension.rs`; reached through
  `crate::commands::extension`, declared by `commands.rs` as
  `pub(crate) mod extension;`.
- **Responsibility:** The extension surface of the Tauri command layer — Native
  Host registration, extension directory import, status projection and folder
  opening.
- **Owner:** Cross-platform (the Windows Native Host registration itself stays in
  `crate::windows_transport`, which is Windows-owned).
- **Depends on:** `crate::RuntimeState`, `crate::portable::PortablePaths`,
  `crate::platform::open_path_command`, `crate::websocket_transport::*` for the
  status projection, and `crate::windows_transport` behind `#[cfg(windows)]`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, which
  names each command as `commands::extension::<name>`.
- **Public contract:** the 5 registered command names, `ExtensionStatus`'s
  serialized fields, the argument names and every error string (including the
  Chinese-language import messages). Unchanged by the move.
- **Compatibility entry:** `generate_handler!` must name
  `commands::extension::<name>`; `#[tauri::command]` hidden items are not
  re-exportable. The names are therefore absent from the `use commands::{...}`
  list.
- **State and ownership:** no state of its own. Import writes a temporary
  directory under the portable cache, validates it, swaps it into place with a
  backup, and persists the resulting version/digest to `state.config`.
- **Failure behaviour:** fail closed with a redacted `String` error. A bad
  manifest, a missing `background.js`/`content.js`, an import source equal to the
  current target, or a failed swap restores the backup and returns an error. On
  non-Windows, registration returns "only available on Windows".
- **Tests:** covered by the Desktop lib suite; extension import/status behaviour is
  exercised through the runtime and platform fakes.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`. Windows registration needs a Windows Owner check.
- **Exceptions:** none; 345 lines is well under the `rust-desktop` limit.
- **Open questions:** none.
