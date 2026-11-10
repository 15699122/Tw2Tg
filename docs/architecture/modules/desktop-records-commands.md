# desktop-records-commands

- **Path / entry:** `desktop/src-tauri/src/commands/records.rs` plus child modules
  `records/{export,import_document,import_evidence}.rs`.
- **Responsibility:** expose the record and configuration exchange IPC commands
  (export, preview, apply, import, cancel) with their unchanged wire shapes.
- **Owner:** Cross-platform Owner.
- **Depends on:** `xarchive_core` record documents, `xarchive_storage`
  (`Database`, record evidence, record import), `xarchive_workflow::maintenance`.
- **Depended on by:** `lib.rs` `generate_handler!` registration.
- **Public contract:** unchanged. All seven `#[tauri::command]` functions stay in
  `records.rs`; command names, argument shapes, serialized fields and error strings
  are the registered IPC contract and were not altered.
- **Compatibility entry:** `lib.rs` keeps the seven
  `commands::records::<name>` registrations.
- **State and ownership:** `RecordImportWait` and the
  `RECORD_IMPORT_CANCELLATION` slot stay in the same module, so the cancellation
  registration keeps exactly one owner.
- **Failure behaviour:** export refuses a record with a private URL or a
  non-portable media path instead of rewriting it; a changed revision invalidates an
  earlier preview; the whole accepted selection is rechecked after the maintenance
  drain, so a valid prefix is never imported past a changed record.
- **Tests:** `commands::records::record_import_acceptance_tests` (8 tests),
  unmoved.
- **Gates to re-run:** records command tests; Desktop lib regression; IPC
  registration check.
- **Exceptions:** none.
- **Related:** [desktop commands records](desktop-commands-records.md).
