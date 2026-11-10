# workflow-writer-lifecycle

- **Path / entry:** `crates/xarchive-workflow/src/writer_lifecycle.rs` plus
  externally stored test modules `writer_lifecycle/lifecycle_tests.rs` and
  `writer_lifecycle/tests.rs`.
- **Responsibility:** govern writer admission and drain: register a writer role,
  close admission, require verified drain evidence before publication, and keep the
  lifecycle owner retained until the adapter's drain completes.
- **Owner:** Cross-platform Owner.
- **Depends on:** `WriterContainment` implementors and the coordination guards used
  by the Linux tests.
- **Depended on by:** archive publication authorization.
- **Public contract:** unchanged. `WriterLifecycle`, `WriterRegistration`,
  `PublicationOwner`, `PublicationPermit`, `ProductionWriters` and the
  `WriterContainment` trait keep their names, constructors and visibility.
- **Boundary that must not move:** permit/evidence construction authority and the
  borrow/consume semantics of `PublicationPermit` are the security property of this
  module; the file was not split along those lines, only its two test modules were
  externalised.
- **State and ownership:** the lifecycle owner is retained until drain completes; a
  failed drain never proves exit and keeps admission closed on retry.
- **Failure behaviour:** unchanged.
- **Tests:** two modules were externalised verbatim:
  `lifecycle_tests` (4 tests plus a `Fake` containment double, `#[cfg(test)]`) and
  `tests` (6 tests plus its fixtures, `#[cfg(all(test, target_os = "linux"))]`).
  Both modules already contained **identically named** test functions
  (`failed_start_requires_cleanup_and_cannot_restart`,
  `failed_drain_never_proves_exit_and_retry_keeps_admission_closed`,
  `production_drain_update_requires_a_registered_role`,
  `admission_close_does_not_erase_a_later_verified_drain`); they were kept as-is
  rather than merged or de-duplicated, so the run still reports 10 tests.
- **Gates to re-run:** `cargo test -p xarchive-workflow --lib` (49/0/0).
- **Exceptions:** none.
- **Open questions:** the `tests` module is Linux-only by `cfg`, so its 6 tests do
  not exist on Windows; that was already true before this change.
