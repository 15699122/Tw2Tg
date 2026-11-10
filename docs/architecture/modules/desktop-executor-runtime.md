# desktop-executor-runtime

- **Path / entry:** `desktop/src-tauri/src/executor/runtime.rs` plus its child
  modules `runtime/startup.rs` and `runtime/application_service.rs`. `runtime` stays
  a private module; the public parent re-exports for `ExecutorRuntime`,
  `JobExecutor`, `JobExecutorHandle` and `ArchiveApplicationService` are unchanged.
- **Responsibility:** queue/control-plane state, worker and runner thread lifetime,
  command dispatch, cancellation registry, admission ownership and service entry
  points. `JobRecord`, `handle_command` and `run_worker` stay together; this is not
  the attempt-owned session state machine.
- **Owner:** Cross-platform Owner.
- **Depends on:** existing executor contracts and ports, factory, recovery
  entry points and session boundary. Concrete production factory provisioning is
  preserved, not replaced by a new dependency-injection framework.
- **Depended on by:** `executor.rs` (public re-exports), `executor::session`,
  `transport`/`commands` callers, and the executor subtree tests.
- **Public contract:** unchanged. Public names, IPC callers, command payloads,
  response handling and queue sizes are the same; nothing was promoted to `pub`.
- **Compatibility entry:** `executor.rs` keeps
  `pub use runtime::{ArchiveApplicationService, ExecutorRuntime, JobExecutor, JobExecutorHandle};`
  so every existing path resolves. `runtime.rs` re-exports
  `ArchiveApplicationService` from its child module.
- **State and ownership:** `JobRecord`, `Command`/`RunnerCommand`,
  `CancellationRegistry`, `OwnedRunnerAdmission`, `JobExecutor`/`JobExecutorHandle`
  and the free functions `run_runner`, `run_worker`, `handle_command`,
  `transition_job`, `cancel_job`, `sidecar_crash` remain in `runtime.rs`. Their
  `impl` blocks (`impl JobExecutor`, `impl Drop for JobExecutor`,
  `impl JobExecutorHandle`) also stay there, so worker/runner lifetime, admission
  and shutdown/join are not split across modules.
- **Failure behaviour:** unchanged. Startup only *calls* the existing recovery
  entry points. Containment provisioning performs no environment inference and has
  no uncontained fallback. Shutdown drains and joins worker and runner before it
  returns. V2 enrollment stays closed unless explicitly provisioned.
- **Visibility:** `ArchiveApplicationService`'s `executor` and `v2_enrollment`
  fields were `pub(super)` where `super` means `executor` — not `runtime`. They are
  `pub(in crate::executor)` in the child module, which *preserves* the original
  ancestor scope rather than widening it. `ExecutorRuntime::with_production_factory`
  (a construction helper, not an entry point) and
  `ExecutorRuntime::recover_startup_with_v2` (the recovery entry point) were briefly
  widened to `pub(crate)` during the move and have since been narrowed back to
  `pub(in crate::executor)`, matching pre-move reachability.
- **Tests:** `executor/tests/` (six responsibility groups plus the two self-exec
  selectors) and `executor/session_boundary_tests.rs`. Test paths are unchanged.
- **Gates to re-run:** executor lib regression, plus the delegated production
  acceptance listed below.
- **Verification:** original bodies, strings, attributes and comments match after
  only the enumerated visibility changes and rustfmt. Historical Desktop result at
  commit `1cd1692e` is 365 passed / 13 ignored. **Specialised delegated acceptance
  is closed** (2026-11-09): registered-writer drain publication, post-commit DB
  failure recovery, uncontained-writer refusal, startup drain-before-decision and
  runtime replacement were each executed in a real delegated scope and passed,
  together with shutdown/join and SIGKILL recovery. Real aria2/gallery-dl tool
  acceptance remains deferred; Windows-native equivalence stays Windows-owned.
- **Evidence:** [large-file capability refactor evidence](../../validation/large-file-capability-refactor-evidence.md)
  §P9. Earlier logs `r5-runtime-session-equiv.log`,
  `r5-runtime-verified2-summary.json`, `r5-session-verified2-summary.json` are bound
  to older sources and are not reused as current results.
- **Exceptions:** none — the former `executor.rs` size exception was removed in
  `9947de9c`.
- **Open questions:** none.
- **Related:** [session](desktop-executor-session.md),
  [recovery](desktop-executor-recovery.md),
  [approved plan](../../development/internal-v1-completion-and-refactor-plan.md).
