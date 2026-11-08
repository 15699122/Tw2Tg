# desktop-executor-containment

- **Path / entry:** `desktop/src-tauri/src/executor/containment.rs`; declared by
  `executor.rs` as `mod containment;`. Parent re-exports `AttemptWriterDomain` and
  `LinuxExecutionContainment` as `pub`, and the seven `pub(crate)` helpers as
  `pub(crate)`, so every previous path and reach is unchanged. `ATTEMPT_WRITER_ROLES`
  is intentionally not re-exported: only this module enumerates the roles.
- **Responsibility:** Linux attempt-bound writer containment — how a production
  execution owns a cgroup writer domain, and how a successor discovers and drains
  the domains a crashed attempt left behind. Entirely `target_os = "linux"`.
- **Owner:** Cross-platform (Linux behaviour, shared contract).
- **Depends on:** `xarchive_workflow::{AttemptIdentity, linux_containment
  ::{LinuxContainment, RecoveryDrainBinding, RecoveryOutcome, AttemptIdentity},
  writer_lifecycle::VerifiedRecoveryDrain}`.
- **Depended on by:** `ProductionExecutionFactory`, `ExecutorRuntime`,
  `SessionExecutionBoundary`, both recovery runners and the containment regression
  tests — all still in `executor.rs`.
- **Public contract:** `LinuxExecutionContainment`'s launch-descriptor parsing and
  path validation are byte-for-byte the same, including the 16 KiB descriptor
  limit, the `schema_version != 1` rejection, the `/sys/fs/cgroup` dedicated-parent
  requirement, the symlinked-descriptor rejection and the disposable probe that
  proves delegation without starting a target process. The 10-second drain timeout
  and the "only `NoRecord` / `AlreadyAbsent` / `Drained` admit, any error fails
  closed" rule are unchanged.
- **Why ADR-015 is untouched:** the module supplies containment primitives that the
  executor *calls* at its admission, drain, authorize and release points. It never
  admits, claims, authorizes or finalizes on its own, and
  `ExecutorRuntime`, `handle_command`, `running_transitions`,
  `recover_internal_v1_on_runner`, `recover_v2_on_runner` and
  `impl JobPersistence for StorageJobPersistence` all remain in the parent.
- **Equivalence:** PASS. The 317-line moved region is token-identical to the
  pre-slice source after comment/whitespace normalization, none of its ten items
  remain defined in the parent, and `attempt_writer_domain_tests` travelled with it
  (`.tw2tg-tmp/agent-scratch/build-logs/r5-containment-equiv.txt`).
- **Validation:** `cargo check -p xarchive-desktop --lib` exit `0` with only the two
  pre-existing warnings; Desktop lib exit `0` **365 / 13 ignored**; Workflow 49;
  Sidecar Supervisor 17 + 2. Delegated cgroup matrix, run inside
  `systemd-run --user --property=Delegate=yes` against a disposable parent:
  `production_runner_finalizes_internal_v1_archive_after_registered_writer_drain`
  (V1-01/02a), `production_runner_post_commit_database_failure_recovers_without_false_success`
  (V1-03), `production_runner_rejects_uncontained_internal_v1_writer_before_publish`
  (V1-02a), `startup_recovery_drains_crashed_writer_domains_before_decisions`,
  `startup_recovery_drains_live_descendant_after_owner_sigkill`,
  `executor_shutdown_joins_active_writer_before_successor_runs` and
  `runtime_replacement_joins_active_writer_then_runs_successor` (V1-07) — **7/7
  PASS**. Evidence `.tw2tg-tmp/agent-scratch/build-logs/r5-containment-v1{c,d,e}.log`.
- **NOT_RUN, unchanged:** `production_runner_registers_and_drains_real_aria2_before_internal_v1_publish`
  fails on `TW2TG_TEST_ARIA2` being absent (`aria2c` is not installed here). This is
  the long-standing user-deferred aria2 scope and a tool-absence result, not a
  containment regression. It is **not** recorded as PASS.
- **Environment correction:** `systemd-run --user --property=Delegate=yes` works in
  this session, but the delegation must set both `--working-directory` and
  `--setenv=PATH`, because the transient unit starts in `$HOME` with a minimal PATH
  and otherwise cannot find `Cargo.toml` or `cargo`. `user@1000.service` itself
  reports `Delegate=no`, yet its `app.slice` is user-writable and accepts child
  groups, so the earlier "delegation is available" and "delegation is unavailable"
  claims are both too coarse: the capability belongs to the transient delegated
  unit, not to the user manager's own property.
- **Exceptions:** none; 332 lines.
- **Open questions:** none.