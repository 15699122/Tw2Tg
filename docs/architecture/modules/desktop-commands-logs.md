# desktop-commands-logs

- **Path / entry:** `desktop/src-tauri/src/commands/logs.rs`; reached through
  `crate::commands::logs`, declared by `commands.rs` as `pub(crate) mod logs;`.
- **Responsibility:** The application-log surface — frontend diagnostic events, log
  reading and log folder opening.
- **Owner:** Cross-platform.
- **Depends on:** `crate::RuntimeState`, `crate::logging::LogFile`,
  `crate::logging::redact_line`, `crate::config::LogLevel`,
  `crate::platform::open_path_command`, and `crate::commands::bounded_diagnostic`
  (kept in the parent because the parent's redaction test also asserts it).
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, which names
  each command as `commands::logs::<name>`.
- **Public contract:** `log_frontend_event`, `read_application_logs`,
  `open_log_folder`, the `FrontendDiagnosticEvent` field names and defaults, and
  the error strings. Unchanged by the move.
- **Security note:** `log_frontend_event` is the P2-A redaction boundary. Every
  field is length-bounded and newlines are stripped before `redact_line` applies
  the configured log secrets; nothing reaches disk unredacted. This is why it
  belongs to the log domain rather than a generic diagnostics module.
- **State and ownership:** reads `RuntimeState.log_file` and `RuntimeState.logs_root`
  only; it never creates or rotates the log file.
- **Failure behaviour:** an unavailable log file returns
  "application log is unavailable"; an empty event name returns
  "frontend diagnostic event is empty"; a failed folder open returns
  "failed to open log folder: …".
- **Tests:** the parent's redaction/bounding tests exercise `bounded_diagnostic`
  and `redact_job_errors`; log reading is covered through `crate::logging`.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`.
- **Exceptions:** none; 80 lines.
- **Open questions:** none.
