# storage-archive-recovery-manifest

- **Path / entry:** `crates/xarchive-storage/src/archive_recovery_manifest.rs`;
  entered through the crate-root `pub use archive_recovery_manifest::{...}`.
- **Responsibility:** Define and validate the immutable archive-recovery manifest:
  the media facts, the recoverable two-phase rename plan, the D1 metadata export
  selection, the V1 manifest and the V2 manifest, plus `validate`,
  `from_metadata` and `digest`.
- **Owner:** Cross-platform
- **Depends on:** `crate::StorageError`, `serde`, `sha2`, `std`,
  `xarchive_core::ArchiveMetadata`.
- **Depended on by:** `archive_recovery` (journal persistence, phase transitions
  and the V1/V2 recovery operations all name these types), `archive_service`, and
  the Desktop recovery paths that read or verify a committed manifest.
- **Public contract:** `ArchiveRecoveryMediaFact`, `ArchiveRecoveryManifest`,
  `ArchiveRenamePlan`, `ArchiveRecoveryManifestV2`, `MetadataExportSelection`.
  The `deny_unknown_fields` attribute, every serde field name, the
  `metadata_output` skip rule, the schema-version values and the SHA-256 digest
  computation are unchanged by the move.
- **Compatibility entry:** crate-root re-export; no import path changed for
  callers.
- **State and ownership:** pure. No connection, no transaction, no journal row and
  no filesystem access. It only defines and validates the payload that the
  journal persists, which is why it can be reasoned about and tested without a
  database.
- **Failure behaviour:** fail closed with `StorageError::InvalidMetadata`. A
  manifest is rejected if its schema version, job/attempt/tweet identity, archive
  directory shape, rename phase or media facts are inconsistent, if any path is
  absolute, escapes upward, contains a backslash or a drive colon, if two media
  entries share an index or collide case-insensitively, if any SHA-256 is not 64
  hex characters, or if a rename/export path collides with a reserved metadata
  file name. `from_metadata` validates before returning.
- **Tests:** `crates/xarchive-storage/src/tests/archive_recovery_manifest.rs`
  (9 suites) exercises these contracts directly.
- **Gates to re-run:** `cargo test -p xarchive-storage --lib`;
  `cargo test -p xarchive-desktop --lib`; the crash-window suites that re-execute
  themselves.
- **Exceptions:** none. The SQL, phase transitions and transaction boundaries
  deliberately stay in `archive_recovery`; do not move them here.
- **Open questions:** none.
