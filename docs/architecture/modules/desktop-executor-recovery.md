# desktop-executor-recovery

- **Path / entry:** `desktop/src-tauri/src/executor/recovery.rs`, declared privately
  by `executor.rs`; report types keep their `pub(crate)` compatibility paths.
- **Responsibility:** startup-recovery reporting and Linux journal replay under
  the existing runner admission and reservation. InternalV1 and v2 remain distinct
  paths; extraction does not enroll jobs into v2.
- **Owner:** Cross-platform Owner. Native Windows adapters remain Windows-owned.
- **Dependencies:** executor service, persistence/session ports, containment drain
  evidence, Storage archive integrity verification and Workflow recovery authority.
  `ProductionCoordinator` stays in the parent because both session setup and replay
  use that shared coordination adapter; moving it solely for naming would obscure
  its ownership.
- **Contract:** complete original bodies and cfg attributes preserved. Only the
  report's `failed` constructor and two replay functions gain `pub(super)` so
  siblings/parent can call them. Existing crate-visible types/fields are unchanged.
- **Ordering:** a damaged/mismatched archive is refused. `COMMITTED + COMPLETE`
  becomes a no-op only after committed-recovery authorization and Storage integrity
  verification. Drain evidence and durable identity rechecks remain in place.
- **Verification:** exact complete types/impl and both replay functions compared to
  `f183e0f5`; every residual parent byte matched before later slices. Three audit
  negative controls reject decoder, runtime and integrity-bypass changes.
  Desktop 365 passed / 13 ignored and delegated production/recovery subset 5/5,
  each Cargo/systemd child exit 0, bound to source hashes in the JSON summaries.
- **Evidence:** `.tw2tg-tmp/agent-scratch/build-logs/r5-strict-equivalence.log`,
  `r5-recovery-verified-summary.json`, `r5-recovery-delegated-summary.json`.
- **Related:** [runtime/control](desktop-executor-runtime.md),
  [session](desktop-executor-session.md),
  [approved plan](../../development/internal-v1-completion-and-refactor-plan.md).