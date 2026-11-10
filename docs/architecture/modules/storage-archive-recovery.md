# storage-archive-recovery

- **Path / entry:** `crates/xarchive-storage/src/archive_recovery.rs` plus child
  modules `archive_recovery/{dispatch,v1_journal,v2_journal,finalization}.rs`.
- **Responsibility:** provide the durable journal, manifest and finalization
  database capability that archival recovery reads and advances.
- **Owner:** Cross-platform Owner.
- **Depends on:** the crate `Database` connection, recovery contract/phase enums
  and `StorageError`.
- **Depended on by:** `ArchiveService` recovery paths and `ArchiveRecoveryDispatch`
  consumers.
- **Public contract:** unchanged. The public `Database` methods keep their names,
  signatures and the `archive_recovery` module path; the inherent `impl Database`
  is distributed across child modules.
- **Compatibility entry:** `mod archive_recovery;` and the `lib.rs` re-exports of
  `ArchiveRecoveryContract`, `ArchiveRecoveryDispatch`, `ArchiveRecoveryJournalRecord`,
  `ArchiveRecoveryManifestV2Record` and `ArchiveRecoveryPhase` are untouched.
- **State and ownership:** this module owns durable rows only. It never performs
  admission and never mints a `PublicationPermit`.
- **Failure behaviour:** unchanged. Transactions stay whole per call; CAS comparisons,
  attempt fencing and phase monotonicity are still inside the same method bodies.
- **Visibility:** `v2_journal_state` is `pub(super)` so the sibling `v2_journal` and
  `finalization` files can call it; that matches its previous private `impl` reach
  inside the module. Everything else stays `pub` as before.
- **Tests:** `crates/xarchive-storage/src/tests/archive_recovery/` (31 suites),
  unchanged.
- **Gates to re-run:** Storage lib regression; Desktop lib regression.
- **Exceptions:** none.
- **Open questions:** the architecture baseline still records the pre-split line
  count; resealing requires separate authorization.
- **Related:** [storage archive recovery manifest](storage-archive-recovery-manifest.md),
  [storage archive service](storage-archive-service.md),
  [storage tests](storage-tests.md).
