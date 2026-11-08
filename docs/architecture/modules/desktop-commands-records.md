# desktop-commands-records

- **Path / entry:** `desktop/src-tauri/src/commands/records.rs`; reached through
  `crate::commands::records`, declared by `commands.rs` as
  `pub(crate) mod records;`.
- **Responsibility:** The record and configuration exchange surface — exporting,
  previewing and applying a versioned exchange document, and previewing,
  cancelling and accepting a record import.
- **Owner:** Cross-platform. The Windows-specific archive apply path remains
  Windows-owned.
- **Depends on:** `crate::RuntimeState`, `crate::portable::PortablePaths`,
  `crate::executor::CancellationToken`, `xarchive_core::DownloadRecord` and
  `ArchiveMetadata`, `xarchive_storage` (`Database`,
  `record_import::{AcceptedRecord, RecordImportOutcome}`,
  `record_evidence::{RecordEvidenceVerdict, preview_download_records}`), and
  `xarchive_workflow::maintenance::WorkspaceGuard`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, which names
  each command as `commands::records::<name>`.
- **Public contract:** the 7 registered command names, the serialized fields of
  `ConfigurationExchangePreviewResponse`, `ConfigurationExchangeApplyResponse`,
  `RecordImportOutcomeResponse`, `RecordImportPreviewEntry` and
  `RecordImportPreviewResponse`, the argument names and every error string.
  Unchanged by the move, including the flat serialized outcome shape that the
  frontend depends on.
- **Compatibility entry:** `generate_handler!` must name
  `commands::records::<name>`; the hidden `#[tauri::command]` items are not
  re-exportable, so the names are absent from the `use commands::{...}` list.
- **State and ownership:** `RECORD_IMPORT_CANCELLATION` is the single in-flight
  import cancellation token, guarded by `RecordImportWait`, whose `Drop` clears the
  registration so a cancelled or panicked import never leaves a stale token.
  Export reads published facts; import commits through Storage.
- **Transaction and safety boundaries:** the import path revalidates the exchange
  document, re-derives the input revision, probes local media evidence, and only
  then applies. A changed media set rejects the whole selection rather than a
  partial import, and the Git-backed maintenance drain wraps the apply so a
  concurrent workspace operation cannot interleave.
- **Failure behaviour:** fail closed with a redacted `String` error. Unknown
  versions, duplicate identities, revision drift, changed or missing media and
  invalid selections all return an error before any write.
- **Tests:** `record_import_acceptance_tests` moved with the module and runs as
  `commands::records::record_import_acceptance_tests::*`; it covers the parser,
  cancellable wait, invalid selections, whole-selection rejection, revision
  invalidation, both export formats and the flat serialized outcome shape.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`; the Storage record-import and record-evidence suites.
- **Exceptions:** none; 1097 lines.
- **Migration note:** the first extraction attempt produced overlapping spans and
  corrupted `commands.rs`; it was restored from
  `.tw2tg-tmp/agent-scratch/source-spans/r4-commands-before-records.rs` and redone
  with an overlap guard in the migration script.
- **Open questions:** none.
