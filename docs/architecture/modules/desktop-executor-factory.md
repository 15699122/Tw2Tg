# desktop-executor-factory

- **Path / entry:** `desktop/src-tauri/src/executor/factory.rs`; declared by
  `executor.rs` as `mod factory;`. `ProductionExecutionFactory` is a `pub` re-export
  (it was a `pub struct` of the parent); `UnconfiguredExecutionFactory` is
  `pub(super)` with a private parent import, sufficient for the parent and its
  descendants to construct the fail-closed default without crate-wide exposure.
- **Responsibility:** Turning a persisted Job snapshot into an executable
  `JobExecution` bound to the claimed attempt — immutable spec decoding, the
  attempt-owned creation path, and the unconfigured fallback that refuses to run.
- **Owner:** Cross-platform.
- **Depends on:** `crate::execution_spec` (`DecodedExecutionSpec`,
  `decode_execution_spec`), `crate::executor::validate_execution_spec_identity`
  (still in the parent, shared with its tests), the executor contracts, the Linux
  containment helper entry points, `xarchive_core::ChildEnvironment`,
  `xarchive_sidecar_supervisor`, `xarchive_storage::Database` and
  `xarchive_workflow::AttemptIdentity`.
- **Depended on by:** `ExecutorRuntime`, `JobExecutor::with_capacity`,
  `SessionExecutionBoundary` and the factory regression tests.
- **Public contract:** unchanged. Spec decoding still maps a decoding failure to
  `ExecutorError::Persistence`, identity validation still rejects a spec whose
  tweet differs from the Job, and `create_registered_attempt` /
  `create_for_attempt` keep their "never fall back to a Job-only writer" refusal
  (`ATTEMPT_EXECUTION_NOT_CONFIGURED`). `UnconfiguredExecutionFactory` still fails
  with `EXECUTOR_FACTORY_UNAVAILABLE` instead of degrading to uncontained
  execution.
- **Why ADR-015 is untouched:** the factory creates an execution object and does
  nothing else. It performs no admission, scope acquisition, durable claim, drain,
  authorization or finalization; those all stay in the parent, which calls the
  factory at the point the claim already exists.
- **Equivalence:** the earlier selected-block `r5-factory-equiv2.txt` PASS was
  insufficient: it omitted decoder/fallback/second-inherent-impl bodies and accepted
  arbitrary block differences. Corrected complete-region review passes against
  Git `4c208585 → f183e0f5`, covering all declarations/attributes/bodies plus every
  residual parent byte. Only exact test imports and fallback visibility changes
  are allowed; decoder and parent-lifecycle negative controls are rejected.
  Evidence: `.tw2tg-tmp/agent-scratch/build-logs/r5-strict-equivalence.log`.
- **Split-caused cleanups in the parent:** removed `ChildEnvironment` (now only used
  by the factory), moved the supervisor import under `#[cfg(test)]` because only
  tests name it, and marked the `crate::execution_spec` re-export
  `#[allow(unused_imports)]` because `decode_execution_spec` is named only from
  `batch.rs` / `transport.rs` test modules and this module's tests — the re-export
  stays part of the surface rather than being narrowed to silence a warning. The
  library keeps exactly its two pre-existing warnings.
- **Validation:** `cargo check -p xarchive-desktop --lib` exit `0`; Desktop lib exit
  `0`, **365 / 13 ignored**. Delegated V1 subset re-run on this slice, **5/5 PASS**:
  V1-01/02a registered-writer finalization, V1-03 post-commit DB-failure recovery,
  uncontained-publication refusal, crashed-writer drain, and V1-07 replacement.
  Evidence `.tw2tg-tmp/agent-scratch/build-logs/r5-factory-v1.log`.
- **Exceptions:** none; 358 lines.
- **Follow-up:** R7 integration; factory-failure scope/admission release is covered
  by the executed session-boundary regression. Earlier pipeline logs are superseded
  by `r5-session-verified2-summary.json` and `r5-final-delegated-summary.json`, which
  record actual Cargo/systemd child exits and nonzero selected-test counts.