# desktop-executor-traits

- **Path / entry:** `desktop/src-tauri/src/executor/traits.rs`; declared by
  `executor.rs` as `mod traits;`. The parent re-exports `JobExecution`,
  `JobExecutionFactory`, `JobPersistence` and `JobDatabaseFactory` unconditionally,
  and `InMemoryJobDatabaseFactory` / `InMemoryJobPersistence` under `#[cfg(test)]`
  because only tests name them.
- **Responsibility:** The executor's trait contracts and their in-memory
  implementations — how one Job is executed and how executor state is persisted.
- **Owner:** Cross-platform.
- **Depends on:** `xarchive_core::JobState`, `xarchive_storage::Database`,
  `xarchive_workflow::AttemptIdentity`, and the contract types re-exported from
  `crate::executor` (`ArchiveJobRequest`, `JobSnapshot`, `SubmitResult`,
  `JobExecutionResult`, `JobExecutionError`, `AttemptCreationError`,
  `CancelArbitration`, `CancellationToken`, `ExecutorError`, `ExecutorEvent`).
- **Depended on by:** `ProductionExecutionFactory` and `UnconfiguredExecutionFactory`
  (both still in `executor.rs`), `ArchiveApplicationService`, the runner, and the
  executor regression tests.
- **Public contract:** every method signature, default method body, associated type
  and doc comment of the four traits is unchanged. `trait JobPersistence`'s 129 lines
  of method declarations moved verbatim.
- **One deliberate visibility change:** `InMemoryJobPersistence`'s three fields went
  from private to `pub(crate)`. While the type lived in `executor.rs`, the sibling
  `tests` module could read those fields directly; once the type moved into a child
  module, that access became a hard error (`E0616`). `pub(crate)` restores exactly the
  access the tests already relied on and does not widen anything public. This is the
  only body difference the equivalence audit reports, and it is recorded rather than
  suppressed.
- **Why the ADR-015 order is untouched:** the traits only *name* operations. The
  production `impl JobPersistence for StorageJobPersistence` (291 lines) deliberately
  stays in `executor.rs` beside the runtime that drives it, and none of
  `ExecutorRuntime`, `ProductionExecutionFactory`, `handle_command`,
  `running_transitions`, `recover_internal_v1_on_runner` or `recover_v2_on_runner`
  moved. Admission → scopes → claim → execute → drain → authorize → finalize →
  terminal persistence → release is therefore unchanged by construction.
- **Validation:** `cargo check -p xarchive-desktop --lib` exit `0` with only the
  pre-existing `record_recovery_results` warning; `cargo test -p
  xarchive-desktop --lib -- --test-threads=1` PASS **365 / 13 ignored**. Equivalence
  audit: 5 of 6 items body-identical, the sixth differing only by the documented
  field visibility. Evidence:
  `.tw2tg-tmp/agent-scratch/build-logs/r5-traits-equiv.txt`.
- **Exceptions:** none; 419 lines.
- **Open questions:** none.
