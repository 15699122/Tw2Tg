# desktop-archive

- **Path / entry:** `desktop/src-tauri/src/archive.rs` plus test modules
  `archive/completion_tests/{completion_settings,attempt_staging,writer_drain}.rs`.
- **Responsibility:** turn a verified Sidecar archive execution into a durable
  publication: collect attempt facts, freeze them into a move-only pending value,
  and call the crash-safe Storage publication once.
- **Owner:** Cross-platform Owner.
- **Depends on:** `xarchive_storage::ArchiveService`/`FileStore`, `SidecarSupervisor`,
  the executor's `JobExecution` contract, `crate::telegram_send` intent planning.
- **Depended on by:** the executor's archive job path.
- **Public contract:** unchanged. `PendingArchivePublication` is still a move-only
  value with private fields, consumed by `finalize`; no field or method was
  widened, and no publication entry point was added.
- **Compatibility entry:** `mod archive;` and the existing `pub(crate)` exports
  (`upsert_browser_user`, `SidecarArchiveResult`, `AttemptArchiveFacts`,
  `ArchiveExecutionContext`, `ArchiveExecutionJob`, `merge_browser_relationships`,
  `capture_completed_intent`) are untouched.
- **Boundary that must not move:** the pending value carries the Storage half plus
  the process owners so publication facts can never be replayed. This is why the
  production code stays in one file and only tests were classified.
- **State and ownership:** the value owns one `ArchiveService` and one
  `SidecarSupervisor`; it is not cloneable and not reconstructible from facts.
- **Failure behaviour:** unchanged. `PUBLICATION_ALREADY_PUBLISHED`,
  `PUBLICATION_AUTHORIZATION_REJECTED`, `ARCHIVE_PREVIEW_FAILED` and
  `TELEGRAM_INTENT_PREPARE_FAILED` keep their codes and `persistence_already_updated`
  values.
- **Tests:** 9 tests moved into three groups (completion settings 1, attempt
  staging 3, writer drain 5). Two `#[ignore = "requires an explicitly delegated
  disposable cgroup"]` plus their `#[cfg(target_os = "linux")]` moved with the
  writer-drain tests; Desktop ignored stays 13.
- **Gates to re-run:** Desktop lib regression (365/0/13).
- **Exceptions:** none.
- **Open questions:** the two ignored writer-drain tests still require a delegated
  disposable cgroup and were not executed in this round.
