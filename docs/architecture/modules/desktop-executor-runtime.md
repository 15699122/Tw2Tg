# desktop-executor-runtime

- **Path / entry:** `desktop/src-tauri/src/executor/runtime.rs`, private module
  with unchanged public parent re-exports for `ExecutorRuntime`, `JobExecutor`,
  `JobExecutorHandle` and `ArchiveApplicationService`.
- **Responsibility:** queue/control-plane state, worker and runner thread lifetime,
  command dispatch, cancellation registry, admission ownership and service entry
  points. `JobRecord`, `handle_command` and `run_worker` stay together; this is not
  the attempt-owned session state machine.
- **Owner:** Cross-platform Owner.
- **Dependencies:** existing executor contracts and ports, factory, recovery
  entry points and session boundary. Concrete production factory provisioning is
  preserved, not replaced by a new dependency-injection framework.
- **Contract:** four complete original regions moved. Exact subtree-only
  `pub(super)` bridges permit the same executor parent/tests/siblings to reach
  runtime fields and admission operations; no new crate-wide control API exists.
  Public names, IPC callers, command payloads, response handling and queue sizes
  are unchanged. Production imports are explicit.
- **Verification:** original bodies, strings, attributes and comments match after
  only the enumerated visibility changes and rustfmt. Parent residual persistence,
  coordinator and tests match byte-for-byte. Desktop 365 passed / 13 ignored,
  actual Cargo exit 0. Initial test-compilation failures from private test access
  were retained in logs and corrected with subtree visibility, not weaker tests.
- **Evidence:** `.tw2tg-tmp/agent-scratch/build-logs/r5-runtime-session-equiv.log`,
  `r5-runtime-verified2-summary.json`, `r5-session-verified2-summary.json`.
- **Related:** [session](desktop-executor-session.md),
  [recovery](desktop-executor-recovery.md),
  [approved plan](../../development/internal-v1-completion-and-refactor-plan.md).