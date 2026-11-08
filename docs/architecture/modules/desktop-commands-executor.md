# desktop-commands-executor

- **Path / entry:** `desktop/src-tauri/src/commands/executor.rs`; reached through
  `crate::commands::executor`, declared by `commands.rs` as
  `pub(crate) mod executor;`.
- **Responsibility:** Submitting a Job to the real executor service, and querying,
  cancelling and shutting it down.
- **Owner:** Cross-platform.
- **Depends on:** `crate::RuntimeState`, `crate::archive::upsert_browser_user`,
  `crate::executor` (`ArchiveJobSubmissionAdapter`, `StorageJobPersistence`,
  `ExecutorError`, `JobSnapshot`, `ToolCompatibility`),
  `crate::production::resolved_tool_version`, `xarchive_storage::Database`, and the
  parent's `read_output_settings` / `read_downloader_settings` /
  `snapshot_downloader_settings`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, naming
  `commands::executor::<name>`.
- **Public contract:** the 4 registered command names, `ExecutorSubmitResponse` and
  `ExecutorJobResponse` serialized fields, the argument names and every error
  string (including `"download directory setup is required before archiving"`).
  Unchanged by the move.
- **State and ownership:** the command takes only immutable paths plus the executor
  handle from `RuntimeState` under a short lock, then releases it. The executor
  opens its own persistence and resource context for long-running SQLite,
  FileStore, Sidecar and ArchiveService I/O, so no lock is held across that work
  and no admission/drain/finalize ordering is altered by this module.
- **Why the read-model commands stayed in the parent:** `list_jobs`,
  `list_jobs_page`, `get_job_history_index`, `get_job_download_metrics` and
  `get_job_metrics` read the persisted read-model directly and never take the
  executor service handle, so they are not part of this responsibility.
- **Failure behaviour:** fail closed with the executor's own error text; a failed
  handshake or cancel surfaces as an error rather than a fabricated snapshot.
- **Tests:** covered by the Desktop lib suite; the executor's own state machine and
  the V1 production gates live in `desktop/src-tauri/src/executor.rs` and are
  unaffected by this move.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`; the V1 production runner gates if this module's call
  sequence ever changes.
- **Exceptions:** none; 240 lines.
- **Open questions:** none.
