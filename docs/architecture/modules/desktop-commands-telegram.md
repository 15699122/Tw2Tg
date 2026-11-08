# desktop-commands-telegram

- **Path / entry:** `desktop/src-tauri/src/commands/telegram.rs`; reached through
  `crate::commands::telegram`, which `commands.rs` declares as
  `pub(crate) mod telegram;`.
- **Responsibility:** The Telegram delivery surface of the Tauri command layer —
  task and job projections, resume/review decisions, sender start/stop,
  credential replacement and deletion, endpoint inspection and migration, and
  send cancellation.
- **Owner:** Cross-platform (the command surface and its serialized contract are
  shared; the Windows Credential Manager adapter remains Windows-owned and lives
  in `windows_credentials.rs`).
- **Depends on:** `crate::RuntimeState`, `xarchive_storage::{Database, ...}`,
  `xarchive_telegram` (projections, outbox state, transport), `tauri::State`,
  `serde`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, which
  names each command as `commands::telegram::<name>`. The frontend calls these by
  their **registered command name**, not by Rust path.
- **Public contract:** the 13 registered command names
  (`get_telegram_task_state`, `get_telegram_job_state`, `confirm_telegram_resume`,
  `review_telegram_unknown`, `stop_telegram_sender`, `save_telegram_settings`,
  `get_telegram_settings`, `inspect_telegram_connection`,
  `migrate_telegram_endpoint`, `start_telegram_sender`,
  `replace_telegram_credential`, `delete_telegram_credential`,
  `cancel_telegram_send`) plus the `TelegramTaskProjection` and
  `TelegramRuntimeProjection` payload shapes. Argument names, return types,
  `Serialize` field names and every error string are unchanged by the move.
- **Compatibility entry and a constraint worth recording:** the
  `#[tauri::command]` attribute expands into hidden `__cmd__<name>` and
  `__tauri_command_name_<name>` items in the module where the function is
  defined. Those hidden items are **not** reachable through a `pub use`
  re-export, so `generate_handler!` must name the defining module path
  (`commands::telegram::<name>`). Re-exporting the functions from `commands.rs`
  and keeping `commands::<name>` in the handler list fails to compile with
  "cannot find `__cmd__...` in `commands`". Any future split must therefore
  update the handler path, not just the module.
- **State and ownership:** no delivery state of its own. Every command reads or
  mutates `RuntimeState` under its mutex and delegates to storage or the Telegram
  crate. Credentials are never returned to the frontend.
- **Failure behaviour:** fail closed with a redacted `String` error. A missing
  database, an unreachable runtime lock or a refused credential operation returns
  a message that never contains a token or a credential value.
- **Tests:** `desktop/src-tauri/src/commands.rs` keeps the inline test modules;
  the Telegram-related behaviour is covered by the Desktop lib suite and by
  `telegram_send.rs` tests.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`; a frontend smoke check if an argument name changes.
- **Exceptions:** `desktop/src-tauri/src/commands.rs` still carries its named R4
  exception because the remaining domains are not yet split.
- **Open questions:** none.
