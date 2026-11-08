# desktop-executor-session

- **Path / entry:** `desktop/src-tauri/src/executor/session.rs`; crate-visible
  session types and `resolve_session_run` retain their parent compatibility paths.
- **Responsibility:** one attempt's reservation, claim, registered factory creation,
  execution, drain, authorization, finalization and terminal persistence. Includes
  the short `running_transitions` table and move-only deferred startup-cleanup
  owner; this is separate from queue/control-plane transitions.
- **Owner:** Cross-platform Owner.
- **Dependencies:** Workflow coordinator/session/reservation, persistence/factory
  ports, service admission and Linux containment recovery evidence. No event bus,
  new protocol, schema or lifecycle contract is introduced.
- **Contract:** original complete state-machine region moved. Only `service`,
  `persistence` and `execute_session` gain executor-subtree visibility; the tests
  retain their direct scope-release and claim-finalization assertions.
- **ADR-015:** admission → scopes → claim → execute → drain → authorize → finalize
  → terminal persistence → release. The retained cleanup owner remains move-only;
  Drop is still damage control, never cleanup proof.
- **Verification:** complete body equivalence with exact visibility exceptions;
  parent residual byte comparison; Desktop 365 passed / 13 ignored and delegated
  finalizer/recovery/refusal/drain/replacement matrix 5/5, actual child exits 0.
  The existing `factory_failure_finalizes_the_claim_and_releases_admission_and_scopes`
  test asserts FAILED decision, attempt 1, failure event, peer scope reacquisition
  and admission reacquisition; its actual execution is present in the Desktop log.
- **Evidence:** `.tw2tg-tmp/agent-scratch/build-logs/r5-runtime-session-equiv.log`,
  `r5-session-verified2-desktop.log`, `r5-final-delegated-summary.json`.
- **Related:** [runtime/control](desktop-executor-runtime.md),
  [recovery](desktop-executor-recovery.md),
  [approved plan](../../development/internal-v1-completion-and-refactor-plan.md).