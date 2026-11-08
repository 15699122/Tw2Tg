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
- **One deliberate visibility change, recorded:** `InMemoryJobPersistence`'s three
  fields went from private to `pub(crate)`. While the type lived in `executor.rs`,
  the sibling `tests` module could read those fields directly; once the type moved
  into a child module, that access became a hard error (`E0616`). `pub(crate)` fixes
  the compile error but **does widen visibility beyond what the tests need** — the
  tests live in the parent module, so `pub(super)` is the narrower sufficient scope
  and is the intended final shape. This is not described as "restoring exactly the
  previous access"; the earlier wording overstated it.
- **Re-export condition — resolved.** The two in-memory types were `pub struct`s of
  `executor.rs` before the slice, so they belong to the module's surface in every
  build. Their re-export is now unconditional with a local
  `#[allow(unused_imports)]`, because only the non-test build reports it unused and
  that is the accurate signal (no production path names these types) — suppressing
  it by hiding the items behind `#[cfg(test)]` would have narrowed a compatibility
  path to silence a warning. The library keeps exactly its pre-existing two warnings.
- **Field visibility — narrowed.** The three fields are `pub(super)`, not
  `pub(crate)`: the readers are the parent module and its test modules, which
  `pub(super)` covers without exposing the fields to the whole crate.
- **Equivalence — PASS.** `.tw2tg-tmp/agent-scratch/build-logs/r5-traits-audit3.txt`
  (baseline: the last committed pre-R5 source, `370b10eb`) reports 4 traits and 3
  structs/impls matching after whitespace/comment normalization, the production
  `impl JobPersistence for StorageJobPersistence` confirmed still in the parent, and
  one accepted difference: the three `InMemoryJobPersistence` fields go private →
  `pub(super)`. Visibility is compared, not normalized away, so a widening cannot
  pass as noise. The audit's own exit code reaches the caller (`AUDIT_EXIT=0`); the
  earlier `FAIL`/`EXIT=0` contradiction is not repeated.
- **Why the ADR-015 order is untouched:** the traits only *name* operations. The
  production `impl JobPersistence for StorageJobPersistence` (291 lines) deliberately
  stays in `executor.rs` beside the runtime that drives it, and none of
  `ExecutorRuntime`, `ProductionExecutionFactory`, `handle_command`,
  `running_transitions`, `recover_internal_v1_on_runner` or `recover_v2_on_runner`
  moved. Admission → scopes → claim → execute → drain → authorize → finalize →
  terminal persistence → release is therefore unchanged by construction.
- **Validation:** `cargo fmt --all` applied; `cargo check -p xarchive-desktop --lib`
  exit `0` with only the two pre-existing warnings
  (`InMemoryCommitRecoveryFactsProvider` unused import, `record_recovery_results`
  never used); `cargo test -p xarchive-desktop --lib -- --test-threads=1` exit `0`,
  PASS **365 / 13 ignored**. Evidence:
  `.tw2tg-tmp/agent-scratch/build-logs/r5-slice2a-{check2,test}.log`.
- **Exceptions:** none; 419 lines.
- **Open questions:** none.
