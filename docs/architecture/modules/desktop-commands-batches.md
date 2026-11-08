# desktop-commands-batches

- **Path / entry:** `desktop/src-tauri/src/commands/batches.rs`; reached through
  `crate::commands::batches`, declared by `commands.rs` as
  `pub(crate) mod batches;`.
- **Responsibility:** The account-batch surface of the Tauri command layer —
  create, list, get, list candidates, pause, resume, cancel and retry.
- **Owner:** Cross-platform
- **Depends on:** `crate::RuntimeState`, `crate::batch`
  (`spawn_account_batch`, `spawn_batch_dispatch`),
  `crate::executor::CancellationToken`, and the shared helpers that remain in
  `commands.rs` (`open_batch_database`, `CreateAccountBatchRequest`,
  `read_output_settings`, `read_downloader_settings`,
  `snapshot_downloader_settings`, `validate_output_settings`). Only
  `open_batch_database` is `pub(crate)`; the rest stay private because a child
  module can reach its parent's private items.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, which
  names each command as `commands::batches::<name>`.
- **Public contract:** the 8 registered command names (`create_account_batch`,
  `list_account_batches`, `get_account_batch`, `list_account_batch_candidates`,
  `pause_account_batch`, `resume_account_batch`, `cancel_account_batch`,
  `retry_account_batch`), their argument names, return types and error strings.
  Unchanged by the move.
- **Compatibility entry and a constraint worth recording:** `#[tauri::command]`
  expands to hidden `__cmd__<name>` items in the defining module, which cannot be
  reached through a `pub use` re-export. `generate_handler!` must therefore name
  `commands::batches::<name>`, and those names must **not** also appear in the
  `use commands::{...}` import list.
- **State and ownership:** no batch state of its own. Each command opens the
  database through `open_batch_database`, reads the configuration from
  `RuntimeState`, and delegates scheduling to `crate::batch`.
- **Failure behaviour:** fail closed with a redacted `String` error. A missing
  database, an unavailable schedule slot or an invalid output setting returns a
  message rather than starting a batch.
- **Tests:** covered by the Desktop lib suite; the batch scheduling behaviour is
  exercised by `crate::batch` tests.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`.
- **Exceptions:** none specific to this module; `commands.rs` keeps its R4
  exception for the remaining domains.
- **Open questions:** none.
