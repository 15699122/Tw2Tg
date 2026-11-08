# desktop-executor-contracts

- **Path / entry:** `desktop/src-tauri/src/executor/contracts.rs`; declared by
  `executor.rs` as `mod contracts;`. The parent re-exports every item, so
  `crate::executor::JobSnapshot`, `…::ArchiveJobRequest`, `…::CommitRecoveryFacts`
  and the rest keep their existing paths.
- **Responsibility:** The executor's data contracts — the archive request, the Job
  snapshot, submit/execution results, execution and attempt errors, the executor and
  network configuration, and the commit-recovery fact/decision types with their
  in-memory provider.
- **Owner:** Cross-platform.
- **Depends on:** `xarchive_core::{JobState, ProxyMode}`,
  `xarchive_storage::JobSummary`, `crate::config::ProxyMode` and
  `crate::executor::{CommitRecoveryFactsProvider, ExecutorError}`.
- **Depended on by:** the whole executor module, `crate::commands::executor`, and the
  Desktop tests. All of them reach the types through the parent re-export, so no
  call site changed.
- **Public contract:** every field name, `Serialize`/`Deserialize` shape, derived
  trait set, enum variant and `Display` string is byte-identical to the pre-slice
  source. The equivalence audit compares each moved item against the pre-slice
  snapshot and reports MATCH for all 13 types.
- **Why this slice is safe:** it moves pure data plus inherent `impl`s. It contains
  no `Database`, no transaction, no `File`, no writer/permit/phase/resource scope,
  and none of `ExecutorRuntime`, `handle_command`, `transition_job`,
  `running_transitions`, `recover_internal_v1_on_runner` or
  `recover_v2_on_runner`. The ADR-015 order is therefore untouched by construction,
  not merely by inspection.
- **Deliberately left in the parent:** `JobExecution`, `JobExecutionFactory`,
  `JobPersistence` (trait contracts that the runner implementations satisfy),
  `ExecutorEvent`, `CancelArbitration`, `JobRecord`, `CancellationToken`,
  `ArchiveJobSubmissionAdapter` and the containment/startup-recovery items. They are
  the next R5 slices and each has its own dependency question.
- **Test-only import:** `xarchive_core::ProxyMode` is imported under `#[cfg(test)]`
  in the parent because the parent's tests name it directly once the contract types
  moved. A `#[cfg(test)]` import keeps the runtime import surface unchanged instead
  of widening it for a test.
- **Validation:** `cargo check -p xarchive-desktop --lib` exit `0` with only the
  pre-existing `record_recovery_results` warning; `cargo test -p
  xarchive-desktop --lib -- --test-threads=1` PASS **365 / 13 ignored**. Evidence:
  `.tw2tg-tmp/agent-scratch/build-logs/r5-contracts-equiv.txt`.
- **Exceptions:** none; 311 lines.
- **Open questions:** none.

