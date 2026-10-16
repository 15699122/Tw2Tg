# Current Platform Handoff

## Current execution cursor (2026-10-15 continuation; no handoff)

### Desktop download output — remaining non-Windows work

- **Identity:** branch `cross-platform/automatic-pairing-reconcile-20261002`; HEAD/upstream `9368fbdaa66c502e00d32101b1a1daaf7fac020e`. Working tree clean except two untracked runtime `.lock` files (not committed). Owner: Cross-platform Owner; Linux x86_64 under WSL2. Five logical commits pushed to origin; no Windows handoff or Windows validation yet.
- **Batch outcome (R1: atomic terminal completion):** `complete_persisted` previously wrote `Complete` state and `Completed` event as two separate persistence calls, non-atomic. Added `JobPersistence::complete_terminal_for_attempt` + `Database::complete_terminal_for_attempt` that atomically transitions `Downloaded → Complete` and records `DownloadCompleted` + `Completed` events in one SQLite transaction, fenced by attempt. `execute_session` now calls `complete_terminal_for_attempt` instead of separate `record_event(DownloadCompleted)` + `complete_persisted`. Removed the `state_already_updated` early-skip; idempotency is now provided by the attempt fence (state already `Complete` ⇒ no-op). `state_already_updated` field retained on `JobExecutionResult` for call-site compatibility but no longer gates completion.
- **Validation:** `cargo test --workspace` all PASS (0 failed); `cargo fmt --all -- --check` PASS; `git diff --check` PASS. `cargo clippy` clean for `xarchive-storage` and `xarchive-desktop` (pre-existing `xarchive-workflow` `collapsible-if` lint unrelated to this change). Storage tests 148/148 PASS; Desktop tests 312/312 PASS.
- **Current continuation outcome (2026-10-16 closeout):** `ProductionCoordinator` undefined-type compile error fixed: defined `ProductionCoordinator` backed by `LinuxCoordinator` with `#[cfg(unix)]` `Coordinator` impl, and gated `production_session_boundary` with `#[cfg(unix)]`. `archive_recovery.rs` cross-role path-collision validation bug fixed (final_path now correctly rejected when colliding with an existing source). `cargo check --workspace` PASS, `cargo test --workspace` all PASS (0 failed across all crates), `cargo fmt --all -- --check` PASS, `git diff --check` PASS. Remaining blocker: production lifecycle integration (B3/B4/B5) stays fail-closed pending crash-surviving workspace+attempt containment identity and successor-drain/no-late-write proof.
- **2026-10-16 A0-A4 closeout (plan A continuation):** reconciled handoff cursor and evidence (A0); replaced empty `!Clone` test with `Send`-based contract assertion (A0); split `attempt_staging_directory` into `create_attempt_staging_directory` (claim-time, exclusive create) and `existing_attempt_staging_directory` (read-only resolve) so `download_v2` consumes an already-established staging path instead of creating it twice (A1); bound `AttemptArchiveFacts` to an explicit `AttemptIdentity` and verify staging via `existing_attempt_staging_dir(job_id, identity.attempt())` instead of parsing the directory name (A1); preserved `retained_aria2` owner through download-failure and Sidecar-shutdown-failure paths by storing it back into `context.retained_aria2` instead of dropping it (A2). A3: replaced legacy `complete_sidecar_archive` in `execute_archive_context` with fenced v2 publication via `Storage::publish_prepared_internal_rename_v2` — build `ArchiveRecoveryManifestV2`+`ArchiveMetadata` from verified attempt facts, hash each staging file before commit, atomic prepare/rename/finalize under the live attempt identity; removed the `ARCHIVE_RECOVERY_V2_NOT_READY` early return; preserved `retained_aria2` owner through publication. A4: `execute_session` skips legacy `complete_persisted` when `result.state_already_updated` is true (eliminating double publication); added `ExecutorRuntime::execute_persisted_on_runner` routing through `production_session_boundary()` + `execute_on_runner`; `recover_startup` now routes recovery execution through `SessionExecutionBoundary::execute_on_runner` (runner admission → scopes → durable claim → factory → execute → fenced finalize → session release → admission release). `archive::completion_tests` 6/6 and `executor::session_boundary_tests` 13/13 PASS; `cargo test --workspace` all PASS (0 failed); `cargo fmt --all -- --check` PASS; `git diff --check` PASS; `docs-audit` PASS. Production scheduling/recovery/v2 remain fail-closed; legacy `execute_persisted_from_factory` kept as compatibility fallback until B4/B5 acceptance gates pass.
- **Scheduling reconciliation (2026-10-15, updated):** the previously-observed defect is now fixed at the dispatch layer (see batch outcome above): the `RunJob` arm no longer self-enqueues or double-acquires admission. However `execute_persisted_from_factory` still claims on its caller and then dispatches legacy execution that performs Job-scoped publication; it is not yet routed through `SessionExecutionBoundary`. That boundary reserves once and runs an inline operation, but is not wired to production and still has a cloneable result/legacy completion contract. Do not treat the dispatch fix as attempt-owned lifecycle integration.
- **Unique next action (方案 A stage 1→2):** A0–A4 constructs exist and compile: non-Clone attempt-owned outcome (A0), composed writer drain with retained-aria2 preservation (A2), split archive work from fenced publication via `Storage::publish_prepared_internal_rename_v2` with `ArchiveRecoveryManifestV2`+`ArchiveMetadata` built from verified attempt facts and per-file sha256 hashing (A3), immutable attempt staging verified via `existing_attempt_staging_dir(job_id, identity.attempt())` (A1), `execute_session` skips legacy `complete_persisted` when fenced publication already committed (A4), `ExecutorRuntime::execute_persisted_on_runner` added, `recover_startup` and `schedule_persisted` (ordinary submission) both route through `SessionExecutionBoundary::execute_on_runner` (A4). P0 fixes applied: aria2 owner retained unconditionally on Sidecar shutdown failure (archive.rs); `schedule_persisted` uses `#[cfg(unix)]`-gated `ProductionCoordinator`/`SessionExecutionBoundary` with legacy fallback on non-Unix (executor.rs). P1 applied: `output_settings.export_json`/`export_text` restored instead of hardcoded `true`; `telegram_tweet_row_id` now bound to captured `TelegramArchiveIntentRecord` instead of `None` (archive.rs). P2 applied: `record_event(DownloadCompleted)` after durable publication is best-effort — a failed completion event no longer turns a successful archive into a failed Job (archive.rs). Remaining shared work (P1b–P4): owner retention across the `ArchiveExecutionJob` Drop boundary (execution seam releases `retained_aria2` before caller cleanup — needs `JobExecutionResult` owner carry or equivalent, CROSS_PLATFORM_CHANGE_REQUIRED if trait changes); atomic completion/state/event finalization in one transaction; trusted recovery identity and PREPARED replay path; production-path ordering/stale-attempt suppression tests. Production scheduling/recovery/v2 remain fail-closed pending B4/B5 acceptance gates (real cgroup drain, crash-orphan successor-drain/no-late-write evidence). Do not report shared implementation as complete until remaining P1b–P4 pass.
- **Independent blocker:** successor admission after parent crash still lacks trusted persisted workspace/attempt containment identity and parent-SIGKILL/live-descendant/successor-drain/no-late-write proof. The prior WSL delegated cgroup probe failed; this is recovery/capability evidence, not a product result. No PID/time/name inference. Windows Coordinator/NTFS/native validation remains Windows-owned and is not inferred.
- **Validation status for this continuation:** `cargo test -p xarchive-desktop --lib archive::completion_tests -- --test-threads=1` PASS (6/6); `cargo test -p xarchive-desktop --lib executor::session_boundary_tests -- --test-threads=1` PASS (15/15, includes `fenced_publication_records_download_completed_and_skips_duplicate_complete` and `non_fenced_execution_records_download_completed_and_completes`); `cargo test --workspace` PASS (312 passed; 0 failed); `cargo check -p xarchive-desktop --lib`, `cargo fmt --all`, `git diff --check`, and `node scripts/docs-audit.mjs` PASS. These validate the non-Clone execution result living across the completion decision, archive/publication split with `DownloadCompleted` moved to `execute_session`, no-duplicate-completion when `state_already_updated`, and completion when not; production scheduling/recovery/v2 remain fail-closed and unwired. Full Windows validation: NOT_RUN.
- **Git synchronization (2026-10-15):** commit `f21c978484e6e1127e31f61fa32f6cb9b782b14a` (`fix(download): retain aria2 owner after shutdown failures`) contains the five-file Download/Workflow supervisor and containment batch. Push to `origin/cross-platform/automatic-pairing-reconcile-20261002` succeeded; remote was at `f21c978` after push. Six Desktop/plan/handoff files remain uncommitted because typed owner propagation and lifecycle protocol are incomplete. The remote reported 3 Dependabot vulnerabilities on the default branch (2 high, 1 moderate); this was not investigated in this batch.
- **Validation follow-up / exact test harness correction:** the attempted delegated invocation ran no test because the test remained `#[ignore]`; inspection showed its fixture passed `/tmp` to `LinuxContainment::create`, which correctly rejects any parent outside `/sys/fs/cgroup`. The invalid test was removed rather than reported PASS or left as a misleading ignored check. A valid delegated unit can derive its parent from `/proc/self/cgroup` as `crates/xarchive-workflow/tests/linux_containment.rs::real_owner_launches_and_drains` does, but owner-retry semantics still need a protocol-valid fixture that reaches a retained owner without fabricating a startup failure.
- **Full-suite diagnosis:** independently reran existing HEAD tests `executor::tests::saturated_runner_rejection_is_non_failing_and_reschedulable` and `executor::tests::submit_and_schedule_persists_background_executor_failure` with `--exact --nocapture --test-threads=1`; the first exceeded a 25-second timeout (exit 124), and the second panicked because the Job did not reach `FAILED`, then also remained alive until timeout (exit 124). The test implementations are unchanged by this dirty batch. Source tracing shows `schedule_persisted` discards the detached `execute_persisted_from_factory` result and the production default factory can leave the Job queued; this is a pre-existing executor/scheduling defect requiring a separate shared production fix, not a reason to alter this lifecycle acceptance. Full Desktop module result remains BLOCKED/inconclusive, not product FAIL.
- **Dependency matrix (current):** (a) runner admission-before-claim/reschedulable behavior — source includes the expanded saturation/re-admission regression, and the 12-test session-boundary group passes; this does not prove production-factory owner lifecycle; (b) owner-bearing transfer/archive error propagation — present in dirty source, but successful drain-then-refresh evidence is absent; (c) one attempt-scoped owner for Sidecar+aria2+Rust writers and owner-through-session/finalization — executable next shared integration; (d) attempt-owned staging and verified manifest/candidate — helpers/storage primitives exist, not composed in production; (e) B5 no-replace publication plus attempt-fenced atomic state/event/archive finalize — Storage API exists, Desktop production call path is absent; dependent on (c,d); (f) B6 real process-crash/replay and trusted successor drain — depends on an approved persisted containment identity/takeover protocol and delegated crash fixture; (g) B7 scheduling/startup recovery/production enablement — depends on C1 and crash-orphan gates; keep fail-closed; (h) Windows coordinator/NTFS/native artifact acceptance — Windows-owner implementation/validation, independent of Linux shared implementation.
- **Next concrete action (shared, approved architecture; no new decision):** DONE: P2b atomic completion/state/event finalization — removed the `state_already_updated` early return in `execute_session`; `complete_persisted` is now called unconditionally and is idempotent (returns current snapshot if already `Complete`, no double publication, no duplicate `Completed`). Updated `fenced_publication_records_download_completed_and_skips_duplicate_complete` to assert state→`Complete` and exactly one `Completed` event. P4 deterministic recovery tests already added: `recover_internal_rename_v2_committed_is_read_only` (COMMITTED → read-only verify) and `recover_internal_rename_v2_rejects_identity_mismatch` (fail closed, journal stays PREPARED). 15/15 session_boundary_tests PASS, 3/3 database_tests PASS, full workspace `cargo test --workspace` PASS (0 failed). `cargo fmt --all -- --check`, `git diff --check`, `node scripts/docs-audit.mjs` PASS. REMAINING: P5 real cgroup process-tree successor-drain/no-late-write acceptance is environment-blocked (WSL2 cgroup delegation failure); no shared-code task remains executable. Keep production scheduling, startup recovery and v2 fail-closed until drain, staging verification, atomic finalization, replay and crash-orphan successor-drain gates pass. Independent prerequisite: establish a protocol-correct delegated first-drain-failure/same-owner-retry fixture; do not infer cgroup failures or recovery identity. Windows Coordinator/NTFS implementation and native acceptance remain Windows-owned, not blockers to this shared task.

- **Identity / ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`; source HEAD and upstream are both `f9d79291cce3418b5347636ea8f2e2e93ba69775`; working tree has pre-existing and current uncommitted source/documentation changes. Current owner: Cross-platform Owner. No commit, push, or Windows handoff is made. Runtime detected: Linux x86_64 under WSL2. A delegated disposable `systemd-run --user --wait --pipe -p Delegate=yes` run is currently available and successfully executed the real owner success-path test; this does not establish failure injection or Windows acceptance.
- **Dependency matrix:** (1) implement one non-recursive runner-owned production operation that claims, creates execution on the runner, retains/retries typed startup/transfer/Sidecar cleanup owners and finalizes under the live session; (2) split `ArchiveExecutionJob` execution from legacy Job-scoped publication and compose Storage attempt-fenced prepare/publish/finalize with verified transfer-derived file count; (3) independently validate Linux containment with a real delegated first-drain failure and same-owner retry; (4) recovery remains blocked on trusted crash-surviving containment identity and parent-SIGKILL/live-descendant/successor-drain/no-late-write acceptance. Keep production scheduling, startup recovery and v2 fail-closed until their acceptance gates pass. Windows Coordinator/NTFS/no-replace adapter and native acceptance remain Windows-owned and are not blockers for shared work.
- **Current result (2026-10-15 continuation):** verified branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD/upstream `f9d79291cce3418b5347636ea8f2e2e93ba69775` (equal), Linux x86_64 under WSL2, eight dirty tracked files; no commit/push/handoff. `continuous-goal-execution.md` and the current Plan both direct continuation at the approved ADR-015 runner/session operation, not at another independent transfer-only task. The prior 53/53 crate result and formatting/desktop checks are recorded, but this continuation has not independently rerun those commands; prior evidence is bound only to the earlier dirty tree. Current source confirms `transfer_once` maps a failed typed startup owner to an ignored second shutdown attempt, then drops `Aria2Supervisor` on transfer shutdown failure; `transfer_with_one_refresh` refreshes only on `ExpiredUrl`, while shutdown failures currently become `ShutdownIncomplete` and suppress refresh, but neither path returns a retained owner. `SessionExecutionBoundary::execute_session` still claims and finalizes on caller and calls synchronous `execute_owned`; the existing `run_operation` is an inline `'static` runner closure and already has panic/drop/admission tests, but its synchronous API cannot be called from that runner closure. A real delegated cgroup first-failure/same-owner retry remains unverified (`EACCES`); no Windows validation. `docs-audit` passed; initial combined status command had shell integration report exit 1 despite complete-looking output, so only the captured docs-audit and `git diff --check` are accepted from that invocation.
- **Next concrete implementation step:** implement the approved non-recursive runner-owned production operation: move the stable `SessionExecutionBoundary`/coordinator and owned persistence into the `'static` inline runner task, perform admission then scope/claim and invoke object-safe `create_for_attempt` inside that task, and execute the returned `JobExecution` directly on that same runner thread (not through `execute_owned`/`run_job`). Preserve the live `ExecutionSession` until writer drain and fenced finalization. This is the Plan's executable integration seam and removes the false premise that an additional generic borrowed-session queue wrapper is needed: the coordinator can be moved as an owned boundary, while the borrowed session is created and dropped within the inline closure. Carry typed cleanup ownership through the production factory and execution result before allowing refresh or publication; then compose attempt-fenced prepare/publish/finalize. Keep production v2 and recovery fail-closed pending their gates. ADR-015 remains approved; no new architecture decision is requested.
- **Latest continuation (2026-10-15):** source batch adds public `Aria2StartupFailure { error, owner }`, `spawn_contained_owned`, and same-owner `retry_cleanup`; the old error-only `spawn_contained` is deprecated. Production still converts the new typed failure back to legacy `TransferFailure`, and the old `transfer_once` shutdown-error branches still discard the local supervisor on return; owner propagation to refresh/finalization remains unimplemented. A proposed real-cgroup removal failure/retry fixture could not create its cgroup (`EACCES`, before exercising drain); do not report delegated retry acceptance. The synthetic startup-owner/secret-file cleanup retry test is not cgroup evidence. Download crate tests pass 53/53, download test targets compile, Desktop lib checks, `cargo fmt --all -- --check`, and `git diff --check` pass (final exit record `/tmp/tw2tg-final-validation.exit`). No production scheduling/finalization integration, recovery acceptance, Windows validation, commit, push, or formal handoff occurred. Source/upstream remain `f9d79291cce3418b5347636ea8f2e2e93ba69775`; eight tracked files are dirty, including five already dirty at session start. **Next:** change the transfer error/result contract so shutdown failure returns the retained supervisor together with its transfer diagnostic and cannot enter refresh until same-owner retry succeeds; add a protocol-correct transfer seam test for that behavior, then continue runner/session composition. Maintain production v2 fail-closed.
- **Latest continuation (2026-10-15, this session):** revalidated branch/source/upstream at `cross-platform/automatic-pairing-reconcile-20261002` / `f9d79291cce3418b5347636ea8f2e2e93ba69775` (equal), with three pre-existing dirty files. Reviewed the typed-result seam: `transfer_with_one_refresh` and `execute_v2_archive` return `String`, and `ArchiveExecutionJob` restores only `ArchiveExecutionContext`; preserving a failed aria2 owner safely therefore requires a reviewed shared execution-result/context contract, not a local wrapper. `Aria2Supervisor::spawn_contained` also consumes its containment owner and on cleanup failure returns only `DownloadError`, so startup cleanup failure has the same owner-return gap. A first patch attempt was malformed, detected by immediate file read, and reverted; `production.rs` is confirmed unchanged. `rg` is unavailable (exit 127); grep inspection succeeded. Runtime is Linux under WSL2. Delegated cgroup capability is established for a disposable `systemd-run --user --wait --pipe -p Delegate=yes` unit: `scripts/validate-cgroup-containment.py` passed bootstrap failure, positive delayed-write control, descendant kill/drain and absent late write; the ignored `xarchive-workflow` `real_owner_launches_and_drains` integration passed under the same delegated service (exit 0). These prove diagnostics and the existing Linux owner launch/drain path, not containment failure/retry retention, Desktop owner propagation, attempt-wide drain or publication. Next: add a delegated test that causes a real first `stop_and_drain` failure while retaining the `LinuxContainment` value, then retries that same owner and verifies empty-group removal; keep production fail-closed. In parallel, prepare the explicit shared contract change required to return startup/transfer cleanup owners through Desktop execution instead of formatting them into `String`/`DownloadError`. No Windows validation; no commit/push/handoff. Additional code-path audit: `LinuxContainment::stop_and_drain` sets `self.child = None` immediately after launcher reap, before `cgroup.kill` open/write and the populated-state loop succeed. Thus a later drain error retains the cgroup descriptor but not the complete launcher owner; same-owner retry/failure recovery is not yet established. Grep/source search found no existing containment fault-injection seam. Do not claim retry PASS or owner retention across this window; next code task is a narrowly scoped lifecycle correction plus a real delegated failure/retry fixture, followed by re-evaluation of `Aria2Supervisor` and `spawn_contained` owner-return errors.
- **P1 source audit (2026-10-15, corrected scope):** `Command::RunJob` forwards to `RunnerCommand::RunJob`; that runner branch calls `handle.run_operation(...)`, which sends `RunnerCommand::Operation` to the same runner and synchronously waits. If invoked, that path self-deadlocks. Repository call-site search found `run_job` only in the legacy `execute_persisted` path; `schedule_persisted` instead calls `execute_persisted_from_factory`, whose execution dispatch is `RunnerCommand::Execute`, so the self-deadlock is not claimed as a defect in the current scheduled production path. The broader P1 gap remains: `SessionExecutionBoundary` is only called by tests and its `execute_owned` would recursively dispatch if moved into the runner unchanged.
- **P1/B4 source audit (2026-10-15):** `RunnerCommand::Operation` is already an inline, runner-thread `Send + 'static` seam; primitive tests cover thread identity, panic/error owner destruction and admission release. It is not yet composed with the real session or production owners. `SessionExecutionBoundary::execute_session` runs on its caller and `execute_owned` dispatches `JobExecution`; moving the boundary unchanged into an operation would self-dispatch. The production factory starts Sidecar before dispatch and `ArchiveExecutionJob::execute` performs legacy Job-scoped publication internally, so neither generic runner tests nor that execution result proves drain-gated finalization. `production.rs::transfer_once` locally shuts down aria2 before return and refresh reopens only after this local path returns, but there is no typed attempt owner covering Sidecar, aria2 and Rust writers.
- **Inspected containment constraint:** on Linux the factory allocates separate per-process sequence-named cgroups for Sidecar/aria2, not crash-surviving attempt identity. aria2 retains its containment for cleanup retry; Sidecar shutdown may report incomplete reader/child cleanup and does not expose a typed attempt drain. Do not join these local shutdown results into a copied success flag, fabricate cgroup identity, or authorize publication. The next call sites are the production transfer/refresh owner and tests for expiration, drain-before-refresh, cleanup failure suppressing refresh, and destruction ordering; then integrate Sidecar/Rust ownership and Storage finalization. Preserve production scheduling/startup/v2 fail-closed.
- **Continuation reconciliation (2026-10-15):** branch/HEAD/upstream remain `cross-platform/automatic-pairing-reconcile-20261002` / `f9d79291cce3418b5347636ea8f2e2e93ba69775` (equal); five tracked files are dirty, including the prior supervisor and containment edits. Re-read Plan, ADR-015, production factory, runner queue and session boundary. `execute_persisted_from_factory` still claims on its caller and routes through `run_job`; `SessionExecutionBoundary` is a test-owned boundary, not the production scheduler path. Its current `execute_session` calls `execute_owned`, so moving it into `RunnerCommand::Operation` would recursively wait on the same runner. The object-safe factory can be captured and `create_for_attempt` deferred to runner execution, which would move Sidecar startup onto the runner, but that alone does not move claim/finalization or preserve typed cleanup owners and is not attempt-scoped integration. No production change was made. A proposed delegated test using `Duration::MAX` was rejected and removed because it only fails deadline preflight before any cgroup operation; no real first-drain-failure/same-owner-retry evidence exists. `cargo fmt`/focused validation are pending this reconciliation. **Next concrete source step:** add one non-recursive runner execution command that owns the execution factory and accepts an attempt operation; wire production scheduling to create execution only after claim on that runner while preserving existing stable coordinator/persistence owner values; then extend that same operation to retain/retry typed startup/transfer/Sidecar cleanup owners and perform attempt-fenced finalization. Do not create a new coordinator per callback or publish using copied drain status. Production scheduling/startup recovery/v2 remain fail-closed. No Windows validation or Git handoff.
- **Independent recovery blocker:** Plan/ADR require crash-surviving writer-domain identity and trusted successor discovery before successor writer admission. Linux WSL2 has cgroup v2 and user systemd, but the latest explicit delegated `Delegate=yes` probe failed assigning the disposable child (`cgroup.procs` I/O error; recorded prior cursor unit `tw2tg-cgroup-capability-probe-delegated-1791978133.service`, exit 1). No trusted persisted workspace/attempt-to-containment discovery protocol exists. This is capability/recovery `BLOCKED`, not a product PASS/FAIL; do not infer ownership from PID/time/name or treat historical delegated-owner tests as parent-SIGKILL successor-drain acceptance. Retry only as a purpose-built acceptance fixture once a trusted identity protocol exists.
- **Classification:** immediately executable: implement the ADR-015-bounded P1 owner/result seam and its lifecycle tests. Dependency-waiting: P2–P6 above. Decision-required: none; Plan A / ADR-015 is approved and no new architecture decision is requested. Windows-owned: Coordinator/NTFS/no-replace adapter and artifact-bound native validation. The present environment failure blocks crash-orphan acceptance, not shared P1 implementation.
- **This continuation’s evidence:** rechecked current call sites, exact trait bounds, production constructors, Storage finalizer and writer lifecycle. `RunJob` self-deadlocks only on its legacy call path; current `schedule_persisted` uses `execute_persisted_from_factory`/`RunnerCommand::Execute`. `cargo test -p xarchive-desktop --lib executor::session_boundary_tests -- --test-threads=1` PASS 11/11, including existing inline-operation success/error/panic, thread identity and drop-order tests; `cargo fmt --all -- --check` and `git diff --check` PASS. These tests exercise the runner primitive and test-only session boundary, not a production-factory-owned lifecycle. No production source/test changed; scheduling, startup recovery and v2 remain fail-closed. Recovery successor acceptance remains blocked by absent trusted persisted workspace/attempt containment identity.
- **Next concrete action (2026-10-15):** at `/home/shiraishi/VSCode Workspace/Tw2Tg/crates/xarchive-download/src/supervisor.rs`, fix the owner-preservation contract so any `Aria2Supervisor::shutdown` failure retains the direct Child and containment for explicit retry/reap; current code takes/drops Child even when cgroup drain fails. Then change `/home/shiraishi/VSCode Workspace/Tw2Tg/desktop/src-tauri/src/production.rs::transfer_once` to return an owner-bearing typed outcome, and permit `ExpiredUrl`/refresh only after successful drain. Test expiration + successful drain + refresh, drain failure suppressing refresh while retaining the owner, retry cleanup and owner destruction order through protocol-correct transfer seams. Current source already maps aria2 shutdown failure to `ShutdownIncomplete` and thus suppresses refresh; this behavior is not a new implementation claim, and current `production::e3_argument_tests` do not exercise transfer lifecycle. This local transfer owner still does not prove full Sidecar/aria2/Rust attempt drain. Next combine those writers, split `ArchiveExecutionJob` execution from legacy Job-scoped publication, and compose live owner/session with Storage's attempt-fenced prepare/publish/finalize and verified transfer file count. Preserve object-safe factory dispatch, admission-before-claim, session-through-finalize, and production fail-closed. Recovery remains blocked pending trusted persisted containment identity and parent-SIGKILL/live-descendant/successor-drain/no-late-write evidence; ADR-015 remains approved and no new architecture decision is requested.

- **Continuation reconciliation (2026-10-15):** confirmed the previous cursor was stale: the repository is at `f21c978078b4d358b33752fc43299051a7a35deb`, equal to upstream, with six dirty tracked files (not the prior cursor's eight at `f9d7929`). Re-ran Desktop lib check, session-boundary 12/12, and production argument tests 5/5 successfully. Source inspection confirms the dirty owner-bearing transfer changes still do not create a single attempt-scoped owner. The runner boundary executes its inline operation on the one runner thread; calling synchronous `execute_owned` from there would self-deadlock. `ArchiveExecutionJob` still performs legacy Job-scoped publication and `DownloadCompleted` inside `execute`, while attempt staging is distinct; preview-only wiring would not make publication consistent. Storage v2 publishing requires fully verified attempt staging, a matching candidate/manifest, and prior drain; production enrollment remains fail-closed. No production edit was safe without violating these prerequisites. No commit/push/handoff. The next work remains the attempt-owned result + archive split + writer composition and fenced transaction as one ordered lifecycle, with the independent delegated retry and crash-orphan gates retained separately.
- **Independent recovery gate recheck (2026-10-15):** current WSL2 exposes cgroup v2, controllers and `systemd-run`, but the repo's `LinuxContainment` only creates a unique child cgroup under an explicitly supplied delegated parent, retains its directory descriptor in-process, and documents crash-orphan recovery as unimplemented. Search found no persisted workspace/attempt-to-cgroup identity or successor discovery/adoption API. The one-shot environment inspection command's wrapper did not capture a final exit code; its printed output is capability/source inspection only, not validation. No recovery fixture is safe or complete in absence of trusted identity. Do not guess a cgroup name/PID or launch descendants from an unverified test harness. This acceptance remains blocked on the required approved identity protocol, not merely the host's cgroup capability.

### Latest resumed batch — drain-failure finalization evidence (history)

- **Identity / state:** branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD and upstream `0c118f184ce35062b49935b0619c07fa33a85155`; four prior tracked files remain modified, plus `desktop/src-tauri/src/executor.rs`. No commit, push, or Windows handoff. Current owner: Cross-platform Owner.
- **Source reconciliation:** the approved ADR-015 runner move is not a mechanical dispatch change. `ExecutionSession<'coordinator, C>` borrows its coordinator and `RunnerReservation<'a>` borrows `JobExecutorHandle`; the session boundary currently claims and finalizes on its caller while only `JobExecution` runs on the dedicated runner. A full runner-local claim/execution/drain/finalization operation cannot be queued as `'static` without changing the bounded ownership seam. No unsafe lifetime extension, shared mutable execution wrapper, or borrowed `DrainedWriters` transfer was introduced. Existing attempt preview remains unconnected to legacy Job-staging completion; Storage's attempt-fenced prepare/publish/finalize is not yet composed with the writer lifecycle or verified transfer count.
- **New regression:** `archive_writer_drain_failure_is_terminal_and_never_completes_archive` exercises the session failure-finalization branch with `ARCHIVE_WRITER_DRAIN_FAILED`. It proves one claimed attempt reaches `FAILED`, records `DownloadFailed` with the drain diagnostic preserved in its message, and emits neither `DownloadCompleted` nor `Completed`. It proves no retry/success path in this boundary; it does not execute a real Sidecar or prove process-tree drain. An initial test run failed because the test factory inherited the default fail-closed `create_for_attempt`; the fixture was corrected and the targeted test then passed.
- **Validation:** `cargo test -p xarchive-desktop --lib executor::session_boundary_tests -- --test-threads=1` PASS 7/7. The initial `cargo fmt --all -- --check` caught formatting in the new regression; formatting correction is pending/rechecked below. The first two test attempts failed at assertion/setup, then the final targeted regression passed 1/1. Linux environment and WSL2/cgroup recovery status are unchanged from the prior cursor; no Windows validation.
- **2026-10-14 continuation reconciliation:** branch/HEAD/upstream and five-file dirty scope match the preceding cursor. The targeted session suite output printed 7/7 in `/tmp/tw2tg-session-probe-test.log`, but no captured exit-code record exists and the terminal wrapper reported early close/failure; per validation policy this is not a new PASS. No lasting source instrumentation was retained. ADR-015 remains controlling: `ExecutionSession` borrows its coordinator; no unsafe lifetime extension, owned/shared coordinator conversion, second runner channel, or copied drain permit is acceptable. Existing runner owns `JobExecution` only, while boundary currently runs claim/finalization on caller and `execute_owned` synchronously dispatches execution, so sending the whole boundary through that same API would recurse. Storage has `publish_prepared_internal_rename_v2` (requires already verified manifest/metadata and writer drain) plus attempt-fenced DB prepare/finalize primitives; the missing link is a Desktop production completion adapter that derives the verified candidate/count and invokes that pipeline under the live attempt session. The attempt-staging completion event remains correctly withdrawn; do not substitute legacy Job-scoped completion.
- **2026-10-14 environment probe:** WSL2 Linux x86_64, PID 1 systemd, cgroup v2, user systemd active. Re-ran the disposable delegated cgroup child-assignment capability probe: service reported success and emitted `DELEGATED_CGROUP_CHILD_ASSIGNMENT_PASS`; shell log recorded probe exit 0 and systemd reported `Result=success`, `ExecMainStatus=0`. Tool wrapper still reported terminal early-close/nonzero after printing this evidence, so this is capability evidence only, not a formal validation PASS and not product evidence. It does establish an approved route to attempt future delegated crash-recovery fixtures. It does not prove trusted attempt-to-cgroup discovery or parent-SIGKILL/live-descendant successor drain/no-late-write.
- **Historical next-action note (2026-10-14; superseded by current continuation above):** source inspection established why a whole-boundary `'static` callback could not simply be sent through `execute_owned`: `RunnerReservation<'a>` borrows `JobExecutorHandle`, while `execute_owned` synchronously waits on the same runner and consumes/drops only `JobExecution`. The current actionable step is the P1 wrapper/factory construction specified above, not another standalone callback test. The earlier delegated-cgroup probe output is capability-only, not crash-recovery acceptance; no unverified identity or guessed cgroup name may be used. Production scheduling/startup recovery and v2 remain fail-closed.
- **Gates:** production scheduling/startup recovery and v2 remain fail-closed. Linux crash-orphan successor drain is still BLOCKED on trusted crash-surviving workspace+attempt containment identity and parent-SIGKILL/live-descendant/successor-drain/no-late-write proof. This environment gate does not block the shared owner-seam implementation.

### B3/B4/B5 continuous-goal reconciliation

- **Identity / state:** branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD and upstream both `0c118f184ce35062b49935b0619c07fa33a85155`. Four tracked files are modified: Plan, handoff, Storage test, and Desktop archive source. No commit, push, or Windows handoff. Current owner: Cross-platform Owner.
- **Source reconciliation:** `SessionExecutionBoundary` is still unwired to production scheduling; it holds the borrowed session on its caller thread while `execute_owned` dispatches and drops `JobExecution` on the dedicated runner. `ArchiveExecutionJob` still performs legacy archive completion internally and retains its context through `Arc<Mutex<Option<_>>>`. Production transfer emits attempt-relative file paths, while legacy `complete_sidecar_archive` resolves Job-scoped staging; attempt-scoped preview exists but is not wired into a fenced finalizer. New behavior: `execute_archive_context` explicitly shuts down its per-execution Sidecar before metadata/intent/archive completion, and also shuts it down on download-error returns. Shutdown failure maps to `ARCHIVE_WRITER_DRAIN_FAILED` with the original download error retained; the call returns an execution failure and skips archive completion. This proves only direct-child/owned-reader cleanup; it does not prove escaped-descendant drain or combine aria2 with Sidecar. Storage v2 finalizer still lacks atomic `DownloadCompleted`; production scheduling remains unchanged and v2 remains fail-closed.
- **Validation:** new attempt-relative metadata/hash regression passes 1/1; Desktop completion tests pass 3/3, session-boundary tests 6/6, and `archive_context_shutdown_joins_sidecar_before_completion` 1/1 (included in completion tests). Sidecar supervisor library passes 16/16, including blocked-reader `ShutdownIncomplete` and parent/descendant pipe behavior. Storage library passes 142/142 and targeted v2 finalization passes 2/2. `cargo fmt --all -- --check`, `git diff --check`, and `node scripts/docs-audit.mjs` pass. Two exploratory Sidecar-held-pipe fixtures were discarded after they did not establish the intended condition; no such fixture remains. Linux environment is x86_64 under WSL2; prior cgroup delegated-service probe I/O failure remains a capability blocker. No Windows validation.
- **Historical next batch (superseded by the current execution cursor):** implement the approved runner-owned boundary on the dedicated runner thread (coordinator/persistence/claim/execution/drain/finalization co-located), first with a thread-identity/owner-order behavior test and no recursive runner dispatch. Then reconcile attempt-relative transfer paths with drain-gated, attempt-fenced prepare/publish/finalize and atomic verified transfer-derived `DownloadCompleted`. Current session test proves only scope retention, not owner co-location. Keep production/startup recovery gated until the lifecycle and Linux crash-orphan successor-drain gates pass.
- **Recovery gate:** Linux x86_64 under WSL2, cgroup v2 and user systemd available. The plain and explicit `Delegate=yes` disposable service probes failed writing `cgroup.procs` with I/O error (delegated unit `tw2tg-cgroup-capability-probe-delegated-1791978133.service`, exit 1). Earlier 2026-10-13 delegated owner/diagnostic PASS remains historical and is not crash-orphan acceptance. No trusted successor discovery/takeover protocol is present. Production scheduling/startup recovery and writer admission remain BLOCKED pending crash-surviving workspace+attempt containment identity and parent-SIGKILL/live-descendant/successor-drain/no-late-write proof. Never infer ownership from PID/time/name. This is an environment/recovery gate, not a product FAIL.
- **Validation provenance:** prior Workflow/Desktop and delegated-owner counts belong to their recorded historical revisions. This continuation ran Storage targeted tests (2/2), Storage library tests (141/141), and `git diff --check`; Desktop/Workflow suites and Linux crash-orphan acceptance were not run. Current production scheduling and v2 remain unchanged/fail-closed.
- **This continuation:** see the current execution cursor above. Windows coordinator/NTFS/native acceptance remains Windows-owned and independent.
- **Historical dependency matrix:** executable shared lifecycle work is the combined runner-owned resource/result contract, attempt staging, and attempt-fenced terminal state plus completion-event transaction. Production writer admission and startup recovery are dependency-waiting on trusted crash-surviving containment identity and successor-drain/no-late-write evidence. Windows coordinator/NTFS/native acceptance remains Windows-owner work and independent.

### Continuous-goal continuation checkpoint (2026-10-14)

- **Capability / gate probe:** environment is Linux x86_64 under WSL2, cgroup v2 mounted, user systemd running, `systemd-run` available, Rust 1.98.0 / Cargo 1.98.0. A disposable user service could not create/move a probe process into its child cgroup (`cgroup.procs` write returned I/O error; unit `tw2tg-cgroup-capability-probe-1791977973.service`, invocation `647e421cb1924e04b350bbb43f4d552c`). This is an environment capability failure, not a product FAIL. It does not establish parent-SIGKILL recovery and cannot be promoted to PASS. No repository fixture/protocol was found that supplies a trusted successor discovery mechanism. Recovery gate remains open; no PID/time/name inference is authorized.
- **Call-chain reconciliation:** `ArchiveExecutionJob::execute` still calls `complete_sidecar_archive` and records `DownloadCompleted` before returning; `SessionExecutionBoundary` only receives the result afterwards and calls unfenced generic terminal/finalize APIs. This is a concrete shared B3/B4/B5 contract gap, not a safe wrapper-only opportunity. Existing v2 prepare/publish/finalize and PREPARED fencing remain in Storage and must be reused. Production v2 rejection and legacy scheduling are unchanged.
- **This continuation:** the preceding Storage completion-event patch was withdrawn after review found that it supplied manifest media count without proving equivalence to the transfer result's file count. Storage source/tests are restored to their prior state. Runner resource/session ownership, attempt staging integration, real transfer/refresh lifecycle, production wiring, and Linux crash-orphan recovery remain incomplete. Current-source Storage tests passed 141/141 and targeted finalization tests passed 2/2; docs audit and diff check passed. No Desktop/Workflow tests or crash-orphan acceptance. Exact next batch is recorded in the current cursor. Current owner: Cross-platform Owner; no commit/push/handoff.

Historical continuation source (superseded by current cursor above): branch
`cross-platform/automatic-pairing-reconcile-20261002`, HEAD
`640d75ad7cee4ae8510d5a114cc9f160a3a84058` (`docs: advance P0 decision/cursor and remaining gates`).
Current source is `0c118f184ce35062b49935b0619c07fa33a85155`; worktree had the two
uncommitted documentation changes before the present update. Current owner remains
Cross-platform Owner. No commit, push or Windows handoff is made by this continuation.
Environment re-detected: Ubuntu x86_64 under WSL2; `/proc/self/cgroup` is
`/init.scope` and the hierarchy root is not writable, but an explicitly delegated
disposable user service is available. A fresh `systemd-run --user --wait --pipe -p
Delegate=yes` real diagnostic PASS: joined-bootstrap exec failure cleaned up,
positive setsid writer made its delayed write, and the killed setsid writer drained
with no late write; `/tmp/tw2tg-delegated-diagnostic-20261013.{log,exit}` (exit 0).
The actual ignored Rust `LinuxContainment` owner acceptance also PASS 1/1 inside
that service; `/tmp/tw2tg-linux-owner-delegated-20261013-retry.{log,exit}` (exit 0).
An independent `Delegate=yes /bin/true` capability probe also exited 0. These
results supersede the earlier general cgroup-unavailable statement, but are not
Desktop transfer/refresh or archive-lifecycle acceptance.

Re-read the production factory, `SessionExecutionBoundary`, Sidecar/aria2 ownership,
Storage v2 prepare/finalize APIs, and approved C1/B3/B4/B5 sequence. Confirmed
that `SessionExecutionBoundary` has test call sites only; persisted scheduling and
startup recovery still use `execute_persisted_from_factory`. The executor has no
attempt-fenced completion operation combining terminal state and
`DownloadCompleted`, and no production attempt-wide typed writer-drain owner. A
Desktop contained transfer/refresh integration fixture was explored and then fully removed:
two delegated attempts exposed test-harness protocol/startup errors (ready identity,
then fake aria2 exiting before RPC-ready); no production defect was established.
Existing `Aria2Supervisor` tests prove contained startup/readiness/shutdown only,
not the production `transfer_once` / `transfer_with_one_refresh` path. The
combined lifecycle remains unimplemented; wiring only the session boundary would
not prove writer quiescence or atomic archive completion and must not be reported
as a completed B3/B4/B5 integration. The aria2 config intentionally has no
arbitrary test-environment injection; adding a seam
that changes the launch boundary would not be equivalent evidence. No production
change or misleading mock/plain-spawn fixture was retained. The production transfer,
refresh and archive-completion gate is therefore still open, but the original
environment blocker is no longer accurate.

Current call-chain blocker remains architectural and safety-relevant: Sidecar
shutdown is not yet typed writer-quiescence authority at publication, aria2 owner
is local to `transfer_once`, and refresh must not begin until the prior writer is
proven drained. `SessionExecutionBoundary` exists but production entry points do
not use it; Storage v2 prepare/finalize and recovery are not composed into the
production archive commit. B3 reservation/claim, B4 attempt staging and typed
drain, and B5 fenced publication plus terminal DB finalization must be integrated
together; no reservation-only or primitive-only production milestone is
authorized. Storage's current PREPARED transaction already checks the durable
attempt counter against the manifest identity; do not report that check as
missing. D1 is dependency-waiting on C1 acceptance; F2 waits on C1→D1; F3 waits
for F2 and its concrete schema/transaction gates. Windows coordinator/NTFS
implementation and acceptance remain Windows-owner work, not a blocker to shared
design/implementation.
Historical next-action snapshot (superseded by the current execution cursor at
the top of this file): refactor the production execution-object/factory seam
so a runner-owned attempt session retains typed owners for both Sidecar and aria2
writer domains through refresh and archive completion. Compose that ownership
with the existing v2 Storage prepare/publish/finalize path and atomic
attempt-fenced terminal event/state; the former proposal to switch the real
persisted scheduling and startup-recovery paths at this point is superseded by
the current recovery gate. Only after the shared lifecycle is composed, add
protocol-correct delegated production transfer/refresh tests. Keep v2
fail-closed; Windows coordinator, NTFS and native acceptance remain Windows
Owner work and `NOT_RUN`.

**Historical verification snapshot:** source/ownership call-chain inspection only;
no product Rust tests were run in that earlier continuation. Its documentation
audit and diff-check results do not describe the current continuation. Current
validation and implementation status are recorded in the execution cursor above.

Decision update (2026-10-13): user approved Plan A, retaining the generic
runner-owned session and object-safe factory while allowing bounded shared
ownership/lifecycle changes for C1 B3–B7. ADR-015 and the Plan now record that
contract. The call-chain observations above remain implementation evidence.

**Historical validation snapshot (source `1f4e656`; not run in this continuation):**
Storage 141/141, Workflow 35/35, Desktop 299 passed + 1 ignored;
`cargo fmt --all -- --check`, `git diff --check`, and `node scripts/docs-audit.mjs`
PASS. These historical checks do not include the current source revision.

**Historical owner/resource observations:** `ProductionExecutionFactory::create_attempt_execution`
spawns a ready Sidecar before returning the `JobExecution`, so the execution
object owns that supervisor before runner dispatch. `ArchiveExecutionJob::execute`
performs transfer and Storage publication internally, while aria2 containment is
created and destroyed per `transfer_once`. `WriterLifecycle` owns exactly one
`WriterContainment`; its borrowed `DrainedWriters` cannot outlive that owner.
Consequently factory-level allocation of attempt staging would still precede the
required runner-owned lifecycle and cannot prove Sidecar-plus-aria2 quiescence.
Do not insert a synthetic drain token or enable InternalRenameV2 until execution
resource ownership and publish/finalize are composed under the concrete runner.

**Historical batch record (source `1f4e656`; not current checkout):**
`ArchiveService::publish_prepared_internal_rename_v2` composes
prepare → journaled two-phase rename → no-replace directory commit → fenced
finalize in one Storage method, with `ArchiveRecoveryFailpoint` wired at all
three filesystem boundaries. Exit-gate tests (7 new `v2_forward_publish_*`):
full composition with event consistency, all three crash windows converging
through shared recovery replay, destination collision without finalization,
stale attempt fail-closed, PREPARED-beats-cancellation arbitration, candidate
rollback rejection, and duplicate-completion read-only replay. Evidence:
`cargo test -p xarchive-storage --lib` 141/141 PASS (7 forward + 8 recovery),
`cargo test -p xarchive-workflow --lib` 35/35 PASS, `cargo fmt --all -- --check`
PASS, `git diff --check` PASS, non-strict Clippy clean except the pre-existing
`too_many_arguments` lint on `preview_sidecar_archive_attempt` (present in HEAD
`1f4e656`, untouched by that batch). That historical record claimed P4 delivered
the fenced terminal completion via `complete_persisted_for_attempt` and that
`SessionExecutionBoundary::execute_session` used it; current source does not.
`cargo test -p xarchive-desktop --lib` was 299 passed + 1 ignored PASS.
Historical changed-file record (that batch):
`crates/xarchive-storage/src/archive_service.rs`,
`crates/xarchive-storage/src/lib.rs`, `desktop/src-tauri/src/executor.rs`
(code) plus the three P0 documentation files. No commit, push or Windows
handoff. **This does NOT enable v2 acceptance:** `complete_sidecar_archive`
keeps rejecting `InternalRenameV2`, production jobs keep enrolling
`InternalV1`, and `execute_persisted_from_factory` still bypasses the session
boundary. This historical next-action text is superseded by the current cursor
above. Next: integrate B3 reservation/claim/session with attempt-owned staging,
typed writer drain and B5 finalization as one production lifecycle; then add
production transfer/refresh evidence. Do not implement a completion-only fence
or session-only cutover as a substitute for that lifecycle.
P3 real-transfer evidence stays BLOCKED (no aria2c/gallery-dl); keep v2
fail-closed.

The remaining continuation/checkpoint entries below are historical snapshots.
Where they describe cgroup unavailability or the next transfer fixture, this
current cursor supersedes that status: delegated cgroup capability and owner
primitive are now proven, while production transfer/refresh and archive lifecycle
acceptance remain open.

Latest confirmed prior milestone: delegated production-factory fixture using
persisted specs, actual attempt factory, ready-v2 and target-side membership;
execution destruction removed the observed writer group, PASS 1/1
(`/tmp/tw2tg-factory-real.exit` = 0). This is not transfer, refresh or explicit
publication-quiescence evidence; Drop remains best-effort.

This historical continuation revalidated branch `cross-platform/automatic-pairing-reconcile-20261002`,
HEAD `94f533e1ed0de5cf818e1a9b5e6e703ccad38c02` (upstream-equal before this batch).
The runtime provisioning continuation is committed as `7eac279`. The ignored
production factory handshake/membership/drop-cleanup acceptance passes 1/1 in an
explicitly delegated disposable user service; Desktop `production::` tests pass
12/12. Direct shell execution is denied by the WSL cgroup boundary, but the
delegated service provides equivalent evidence for this factory acceptance.
A delegated production-transfer fixture was prototyped and removed after
fixture-only startup/protocol errors; no production defect was established.
Refresh still requires proof that the old writer is drained before Sidecar refresh.

Next gate: resolve whether committed architecture records approve the production
Coordinator, staging, and finalization ownership contract; if not, obtain that
precise decision before shared API changes or production wiring. Then integrate
B3/B4/B5 together and test drain-before-refresh/publication plus actual
`transfer_once` and `transfer_with_one_refresh` worker membership and cleanup.
This is shared lifecycle work, not Windows blocking. v2 remains fail-closed.
Windows/full workspace NOT_RUN; current owner Cross-platform Owner.

Publication checkpoint: implementation is committed in `daf3e0c` (probe
synchronization), `9fbe69d` (Linux owner), `3842a86` (dual supervisors), and
`42c5884` (Desktop opt-in provisioning/transfer wiring). Current owner remains
Cross-platform Owner; this is progress publication, not Windows handoff.
The documentation commit containing this checkpoint identifies the publication
revision. No implementation dirty state remains. Latest publication validation:
Download 52 unit + 7 driver tests, Sidecar supervisor 16 unit tests, Workflow
35 unit tests plus integration/doctests, Desktop 299/299 PASS. Delegated owner
1/1 and dual supervisor 7/7 PASS. Affected crates' strict Clippy and Desktop
no-deps strict Clippy PASS; fmt/diff/docs-audit PASS. Evidence:
`/tmp/tw2tg-sync-validation.exit`, `/tmp/tw2tg-sync-real.exit`, and
`/tmp/tw2tg-sync-owner-real.exit` all 0. Transitive Desktop strict Clippy still
has the previously recorded Storage argument-count failure; not relabeled PASS.
Runtime provisioning, production-path refresh/factory fixtures, B3/B5/recovery
remain incomplete. v2 fail-closed; full workspace and Windows NOT_RUN.

Continuation threads explicit Linux provisioning from production factory into
ArchiveExecutionContext and both initial/refreshed aria2 transfers. Each transfer
creates an exclusive group and uses contained spawn; provisioning failure never
falls back. Cleanup/monitor failures now change expired-URL failure to
ShutdownIncomplete, preventing refresh while old writer shutdown is unconfirmed.
Provisioning is configuration only (Clone), not cloned owner/authorization.
Runtime does not yet supply it. Targeted rejection tests 5/5 PASS; initial chain
Desktop regression 298/298 PASS. Strict transitive Clippy FAIL on pre-existing
Storage preview too-many-arguments; no assertion/lint suppression added.
Final no-deps Desktop Clippy and regression evidence:
`/tmp/tw2tg-chain-final.{log,exit}`. Next: delegated production transfer/refresh
and factory fixtures, then runtime admission/B3/B5 integration. All uncommitted;
v2 fail-closed, Windows and full workspace NOT_RUN.

Current continuation adds explicit Linux provisioning to ProductionExecutionFactory
(`with_linux_containment`): delegated parent, Python and launcher are supplied
by the caller, not inferred from environment. Opt-in Sidecar creation uses the
retained-owner contained start and shared ready-v2 validation; failure does not
fall back to uncontained startup. Default/runtime wiring remains unchanged.
Exclusive pid/sequence group creation refuses orphan collisions rather than
reusing them. CROSS_PLATFORM_CHANGE_REQUIRED applies. Desktop check and existing
factory regression PASS (`/tmp/tw2tg-production-optin.exit` = 0); no delegated
factory integration test yet. Contained RPC fixture now exercises authenticated
addUri, actual fixture payload write and tellStatus complete before shutdown;
delegated 3/3 and strict Download Clippy PASS (`/tmp/tw2tg-transfer-contained.exit`
= 0). This is a fixture, not real aria2 transfer or refresh verification.
Next: factory delegated integration test, thread explicit provisioning into
aria2 transfer/refresh, then runtime admission and B3/B5 finalization. Production
v2 remains fail-closed; no commit/push. Full regression and Windows NOT_RUN.

Continuation: contained aria2 initialization now explicitly drains/removes the
already-created owner on config/client/secret initialization failure, preserving
cleanup diagnostics. Delegated tests cover invalid config before spawn, bounded
startup exit, and successful authenticated getVersion from a target asserting
its own cgroup membership, followed by repeated shutdown and group removal
(3/3 PASS, `/tmp/tw2tg-rpc-ready-real.exit` = 0). Download module tests and
strict Clippy PASS; Desktop lib check PASS (`/tmp/tw2tg-init-owner.exit` and
`/tmp/tw2tg-rpc-ready-check.exit` = 0). RPC readiness uses a Python fixture,
not real aria2 or a completed transfer. No system aria2c found on PATH.
Next: contained transfer/refresh fixture and Desktop attempt-scoped provisioning;
then session-bound prepare/publish/finalize and restart integration. These remain
shared development, not Windows BLOCKED. CROSS_PLATFORM_CHANGE_REQUIRED remains
applicable. No commit/push; v2 remains fail-closed.

Latest verified cursor: Sidecar delegated fixtures now cover valid handshake /
graceful exit and timeout with a ready setsid descendant, kill/drain, reader
join and no delayed write (4/4 PASS). Download has opt-in Linux
`Aria2Supervisor::spawn_contained`, retained owner, secret/argv/environment
startup and explicit stop/drain/remove on startup failure and terminal paths.
Its delegated early-exit cleanup fixture PASS. Startup initially inherited a
15s RPC timeout despite a 1s startup deadline; bounded startup RPC now uses
the remaining deadline (fixture 1.01s, explicit <5s regression PASS).
Affected three crates' tests, strict Clippy and Desktop check PASS; logs and
exit 0: `/tmp/tw2tg-dual-bounded{,-real}.exit`. This is opt-in supervisor wiring,
not Desktop production integration, successful real aria2 transfer or recovery.
`CROSS_PLATFORM_CHANGE_REQUIRED` includes Download->Workflow Linux dependency.
Next: aria2 successful RPC/transfer and partial-start cleanup ownership,
then Desktop attempt-bound owner provisioning and B3/B5 commit integration.
All changes uncommitted; v2 fail-closed; Windows and full workspace NOT_RUN.

Latest continuation: contained Sidecar startup now carries environment set/remove
with removals winning, including the existing proxy-removal policy. A shared
`require_ready_v2` handshake wrapper explicitly shuts down on every handshake
failure and preserves cleanup diagnostics; contained and legacy starts use the
same validation. Delegated tests 2/2 PASS, including environment assertions and
invalid-capability rejection followed by group removal. Affected crates' tests,
strict Clippy and Desktop check PASS; exit files
`/tmp/tw2tg-handshake-final.exit` and `/tmp/tw2tg-handshake-real.exit` both 0.
`CROSS_PLATFORM_CHANGE_REQUIRED` applies to these API changes. Desktop production
still does not opt in. Next: valid handshake, startup timeout/descendant fixtures
and aria2 owner integration, followed by production B3/B5 lifecycle wiring.
Windows NOT_RUN, full workspace NOT_RUN; v2 fail-closed, no commit/push.

Current continuation adds owned piped startup and drain-gated, idempotent group
removal to LinuxContainment. The delegated target reports its own matching
cgroup membership before shutdown (PASS). SidecarSupervisor now has an opt-in
Linux `spawn_contained` path retaining the owner; startup failures attempt
cleanup, shutdown propagates containment failures, joins readers and removes
only drained groups. Its delegated lifecycle fixture PASS (not a protocol
handshake/descendant fixture). Existing uncontained paths remain unchanged.
`CROSS_PLATFORM_CHANGE_REQUIRED`: Linux-only dependency edge from Sidecar
supervisor to Workflow and contained startup API. Both affected crates' tests
and strict Clippy PASS, exit 0 in `/tmp/tw2tg-contained-final.exit`; delegated
tests exit 0 in `/tmp/tw2tg-piped-real.exit` and
`/tmp/tw2tg-contained-real.exit`. Desktop production does not use this yet.
Next: environment additions/removals, handshake and partial-start evidence,
then aria2 startup integration; crash-orphan recovery and publication remain
unfinished. No commit/push, Windows tests NOT_RUN, v2 remains fail-closed.

Latest verified continuation: `LinuxContainment::spawn` now retains its Child
instead of returning it. Stop closes admission, kills/reaps that launcher with
the same bounded deadline used for group drain, and retains the handle on
failure for retry. No supervisor integration or publication authority yet.
`CROSS_PLATFORM_CHANGE_REQUIRED`: provisional spawn API now returns `()`.
Workflow 35 unit, 19 integration (one delegated test separately executed), and
7 doctests PASS; strict Clippy and diff-check PASS, exit 0 recorded in
`/tmp/tw2tg-owner-fixed.exit`. Delegated owner smoke PASS with exit 0 in
`/tmp/tw2tg-owner-delegated.exit`; still no joined-target readiness proof.
The previously observed process-lock test failed even serially: the helper
closed stdout after READY before the probe printed HELD, allowing broken-pipe
exit and premature lock release. It now consumes HELD before closing stdout;
the parallel Workflow run passes. Production lock code was not changed.
Next: add owned pipe/argument startup integration for Sidecar, explicit group
removal and joined-target readiness; then aria2 integration. Overall goal
incomplete, v2 fail-closed, changes uncommitted; no Windows validation.

Latest continuation from `42790547fce6cc99d30a8e3b491382a444817b53`:
uncommitted Linux `LinuxContainment` retains an exclusively created directory
descriptor. It now owns launcher start/reap through [`Self::spawn`], permanently
closes command admission on stop, and performs descriptor-relative kill/drain
with bounded, strict populated-state parsing. No supervisor or Desktop path
uses it yet. Workflow 35 unit tests, adapter path rejection, strict Clippy/fmt/diff
checks PASS; explicit delegated owner smoke test PASS (`/tmp/tw2tg-owner-real.log`, exit file 0).
The smoke test reaps a potentially not-yet-joined launcher before drain; it does
not prove joined target execution, escaped descendants or production cleanup.
Adapter start/reap is now behind the contained cgroup boundary; the caller no
longer constructs or spawns an unrestricted `Command`. Next: wire Sidecar and
aria2 shutdown/startup together, starting with start failure, partial-start and
shutdown/cleanup evidence on one supervisor before production use.
Pending commands can join after an early drain, so this provisional API must
not authorize publication. Readiness and partial-start regressions are still
required before production use. Group removal and crash orphan ownership remain
unfinished. Repository-map registration remains required when the adapter
interface settles.
Overall goal remains incomplete; no commit/push or Windows acceptance.

This cursor supersedes older publication/continuation snapshots below for current
routing. Follow [continuous execution](../development/continuous-goal-execution.md)
and the [remaining Plan gates](../development/desktop-download-output-plan.md#current-remaining-gates-2026-10-13).

- Goal: all permitted non-Windows Plan implementation and verification; incomplete.
- Owner: Cross-platform Owner; `CROSS_PLATFORM_CHANGE_REQUIRED` for lifecycle contracts.
- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source HEAD: `f6fb4c92ab3ee7ab404009439de44a14ae67a925`.
- Publication: Linux primitives/diagnostic committed as `a43e390`; Workflow
  contract committed as `86dc550`. This documentation commit is the publication
  revision (resolve through Git history). All intended changes are included;
  successful push must be verified separately. No ownership transfer is implied.
- Approved route: trusted tools, Python pre-exec launcher, outer session-owned
  orchestration; no malicious same-user migration sandbox claim.
- Current milestone: production B4 owner/adapter and supervisor integration,
  followed by combined B3/B5 wiring. Existing session boundary must be reused.
- Latest evidence: Workflow 35/35, strict Clippy/fmt/diff checks, exit 0 in
  `/tmp/tw2tg-writer-lifecycle.exit` and log; Sidecar 101/101, exit 0 in
  `/tmp/tw2tg-cleanup-regression.exit` and log. Earlier real cgroup checks are
  diagnostic evidence only, not validation of the new production contract.
- Pre-publication validation: Sidecar 101/101 and real delegated cgroup three
  checks PASS. Serial Workflow run passed 35 unit, 18 integration and 7 doctests,
  strict Clippy and docs audit. Logs: `/tmp/tw2tg-publish-serial.log` and
  `/tmp/tw2tg-publish-cgroup.log`, corresponding exit files both 0.
  Initial parallel coordination run failed
  `acquire_all_contention_releases_partial_process_locks`; serial rerun passed.
  Cause is unconfirmed; retain as a test-stability follow-up, not a proven
  environment defect or erased failure. Full workspace/Windows checks not run.
- Next action: review `writer_lifecycle.rs` against Sidecar/aria2 supervisor
  call chains; implement a Rust-owned Linux containment adapter with explicit
  startup/stop-drain errors and launcher resolution. Prove actual supervisor
  cleanup before introducing publication authorization. Do not create a second
  independent Python owner or keep adding diagnostic-only batches.
- Blockers: production integration is unfinished, not environment-BLOCKED.
  F2/F3 concrete contract decisions must be listed before their implementation;
  Windows work cannot be treated as a shared-development blocker.
- Independent work: review Storage prepare/finalize interfaces and D1 compatibility
  fixtures while preserving the approved C1 → D1 → F2 → F3 → G order.
- Prior stops: micro-batch completion, not evidence of a runtime budget limit.
  This request is documentation-only; no production gate or environment config
  changes are claimed. v2 remains fail-closed.

## Current GitHub publication (2026-10-13)

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source: `ec114343e56dc8f7689e750c447a81f5ed1742ea`.
- Launcher and diagnostic are separate implementation commits; this documentation
  commit is the publication revision, resolved through Git history.
- Working state at publication: all current changes included, no intended
  uncommitted remainder. Current owner remains Cross-platform Owner.
- Progress sync only, no Windows ownership transfer or native acceptance.
- Validation: Sidecar 84/84, Desktop session boundary 6/6, real delegated cgroup
  positive/kill-drain controls PASS. Full workspace not run; v2 stays fail-closed.
- Earlier uncommitted statements below describe prior checkpoints and are
  superseded by this publication.

## Runner boundary reconciliation (2026-10-13)

- Source inspection confirms `SessionExecutionBoundary` already owns generic
  coordination and persistence, and retains session scopes across execution and
  finalization. Do not describe B3 as wholly unimplemented or duplicate this type.
- Six existing Desktop session-boundary tests PASS: Send, scope lifetime,
  admission saturation, live-peer contention, claim loss and factory failure.
  Command: `cargo test -p xarchive-desktop --lib session_boundary -- --test-threads=1`.
- Production entry remains unwired pending B4/B5. Success uses existing
  `complete_persisted` and failure uses existing `fail`; this isolated test result
  does not prove atomic attempt-fenced publication/finalization or writer exit.
- Linux launcher coverage is now 11/11 PASS, including short membership write,
  mismatched membership, descriptor closure and exact argv forwarding.
- Overall non-Windows goal remains incomplete; v2 stays fail-closed.

## Linux pre-exec bootstrap continuation (2026-10-13)

- Added a Linux Python cgroup launcher that traverses an existing delegated
  directory with no-follow descriptors, joins and verifies membership before
  executing an absolute target. Join failure exits 125 without starting target.
- Launcher tests: 5/5 PASS. Real delegated cgroup diagnostic now uses the launcher:
  positive delayed-write control and kill/drain negative control both PASS.
- This is a trusted-tool lifecycle primitive, not a same-user adversarial sandbox.
  Supervisor ownership, packaging, cancellation and publication wiring remain
  incomplete. B4/B5 and the overall goal remain incomplete; v2 stays fail-closed.
- Current Cross-platform Owner retains ownership. Changes are uncommitted;
  no formal handoff or Windows acceptance is claimed.

## Published development checkpoint (2026-10-13)

- **New `/goal` capability correction (uncommitted continuation):** source
  `ec114343e56dc8f7689e750c447a81f5ed1742ea`, same branch and Cross-platform
  Owner. WSL2 root cgroup is not writable, but user systemd `Delegate=yes`
  supplies a writable disposable cgroup with `cgroup.kill`. The diagnostic
  `scripts/validate-cgroup-containment.py` PASS: a child calls setsid after
  its bootstrap enters the cgroup, its parent exits, group kill drains the
  cgroup, and its delayed write remains absent. This supersedes the general
  environment-unavailable cgroup claim, not the missing production B4 adapter.
  Python compile and diff checks PASS. PowerShell executes on Windows build
  29683; no artifact-bound native validation or ownership transfer is claimed.
  The all-non-Windows goal remains in progress: B3–B7, D1, F2/F3 and integrated
  G are still open and must not be classified as Windows-only blockers.

- **Diagnostic hardening:** the cgroup probe now runs a positive control proving
  an un-killed setsid writer performs its delayed write, then the killed control
  proving no write after kill/drain. Cleanup uses the exclusively owned cgroup,
  not cached-PID signals; bootstrap exit and bounded cleanup drain are checked.
  Both real delegated-service cases PASS, Python compile/diff checks PASS.
  Production launch remains open: both supervisors currently call Command::spawn;
  post-spawn migration has a writer race, and workspace unsafe_code=forbid rules
  out adding pre_exec. A trusted pre-exec bootstrap or reviewed kernel-backed
  spawn integration is required, including readiness/error propagation, explicit
  kill/drain before publication and protection against migration out of the group.

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source before this publication: `41636217fc0246751aec6a79fe8880dffc1827b1`.
- Implementation commits: `6d1f70a` supervisor shutdown results; `ccf2375`
  execution teardown before response; `5917198` Sidecar descendant cleanup;
  `b5e5ab2` session-bound writer-domain primitives.
- This checkpoint's documentation commit is the publication revision; resolve
  its hash through Git history. Earlier uncommitted/source-revision statements
  below describe prior checkpoints and are superseded by this publication.
- Current owner remains Cross-platform Owner. This is a GitHub progress sync,
  not a transfer of ownership or a request to start Windows implementation.
  All current changes are included in the five publication commits.
- Validation: Download 52/52, Sidecar supervisor 16/16, Workflow 33/33 unit
  tests plus crate integration/doctests PASS; Desktop 298/298; Python protocol-v2
  26/26; fmt, diff-check and documentation audit PASS. Captured validation exit
  0 in `/tmp/tw2tg-publish-validation.log` (local diagnostic log, not a portable
  evidence artifact). No full-workspace, Windows-native or cgroup acceptance.
- B4/B5 remain incomplete and v2 fail-closed. Directory sync/identity checks
  and process-group cleanup do not prove escaped-writer containment or grant
  publication authorization.

## Continuation — aria2 terminal polling cleanup (2026-10-12)

- `CROSS_PLATFORM_CHANGE_REQUIRED`: aria2 `is_running` now performs existing
  shutdown cleanup when polling observes natural child exit. Previously this
  observation could leave descendant writers and the RPC secret until another
  shutdown path ran. `wait_for_exit` is repeatable after cleanup.
- PASS: Download 51/51 and Desktop 295/295, both captured Cargo exit 0.
  The Unix shell fixture exercises polling, delayed descendant-write refusal,
  secret removal and repeated waiting. This is process-group cleanup evidence,
  not escaped-descendant containment or publication authorization.
- Branch/HEAD remain `cross-platform/automatic-pairing-reconcile-20261002` /
  `6567691`; changes remain uncommitted. B4/B5 integration and subsequent
  B6/B7/D1/F2/F3 remain development work, not Windows blockers. Windows native
  checks are NOT RUN; existing manual queue remains applicable.


## Current continuation — pinned session writer root (2026-10-13)

- **Retry durability correction:** fresh creation now syncs every traversed ancestor and retained parent, including existing directories; `EXIST` can represent an ancestor left unsynced by a previous failed call. Failure regression verifies descriptor order on initial creation and retry, all six staging/job/attempt failure positions, and exclusive attempt refusal. Workflow 33/33, doctests 7/7, strict Clippy/fmt/diff-check PASS. Earlier new-directory-only durability limitations are superseded for this traversal, not a power-loss or namespace isolation proof. This adds sync operations on existing ancestors and may fail closed on filesystems refusing directory sync. B4/B5 remain incomplete, v2 fail-closed; dirty state preserved, no commit/push/formal handoff or native Windows/cgroup/full-workspace validation.

- **Directory-sync failure verification:** private sync injection covers all six staging/job/attempt child-parent sync positions, checks descriptor identities and immediate stop on error, verifies partial-directory state and successful fresh retry before attempt creation versus refusal after attempt creation. Public creation remains fixed to `File::sync_all`; no bypass API exposed. Workflow 33/33, doctests 7/7, strict Clippy/fmt/diff-check PASS. No power-loss evidence or retroactive existing-ancestor durability claimed. Prior dirty state preserved; B4/B5 incomplete, v2 fail-closed, no commit/push/formal handoff or native Windows/cgroup/full-workspace validation.

- **Fresh directory sync follow-up:** Unix fresh creation now fsyncs each newly created ancestor and its retained parent, then the attempt directory and its parent, before returning success. Sync errors return `Io`; partial directories may remain and existing attempts are still refused. Workflow 32/32 and doctests 7/7, strict Clippy/fmt and diff-check PASS. No power-loss simulation or directory-sync fault injection performed; existing ancestors are not retroactively proven durable. This does not close namespace replacement or containment gates. Dirty state preserved; no commit/push/formal handoff, B4/B5 incomplete, v2 fail-closed, native Windows/cgroup/full-workspace checks not run.

- **Input-order correction:** ordinary session writes again validate session/relative path before root filesystem access; the identity-fence follow-up had moved pinning ahead of input validation. Regression checks traversal, absolute and empty paths still return `InvalidRelativePath` with a replaced root; valid paths still hit the identity fence. Workflow 32/32, doctests 7/7, strict Clippy/fmt PASS, captured exit 0, log `/tmp/tw2tg-input-order.log`. Full/native Windows/cgroup checks not run for this isolated ordering fix. Prior dirty state preserved, no commit/push/handoff, B4/B5 incomplete and v2 fail-closed.

- **Identity fence follow-up (`CROSS_PLATFORM_CHANGE_REQUIRED`):** fresh/existing adapters record device/inode from the descriptor they observed; `pin()` compares the opened descriptor against that snapshot and refuses mismatches. Ordinary session writes now use the same pin fence. Regression confirms same-name directory replacement blocks both new pin and ordinary writes, while an already pinned writer still targets the original directory. Workflow 32/32, doctests 7/7, strict Clippy/fmt PASS, captured exit 0, log `/tmp/tw2tg-root-identity.log`. This is not a deletion/inode-reuse-resistant identity: descriptors are not retained from adapter observation, and fresh mkdir/open still has a replacement interval. Full/Windows/cgroup validation not run for this isolated Unix identity fence; B4/B5 remain incomplete, v2 fail-closed. Branch/source HEAD and uncommitted ownership unchanged.

- **Boundary verification follow-up:** pinned-writer compile-fail examples reject consuming the borrowed writer before later pinned use and requiring Clone; a positive example compiles. Runtime regression now refuses directory/final-file links and missing intermediate directories after root rename/replacement, confirming no outside payload creation. No runtime API or product behavior changed. Doctests 7/7, Workflow 32/32, strict Clippy and fmt-check PASS, captured `VALIDATION_EXIT=0`, log `/tmp/tw2tg-pinned-boundary.log`. Full regression not run for isolated capability tests; native Windows and cgroup checks NOT_RUN, no containment acceptance claimed.

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD `41636217fc0246751aec6a79fe8880dffc1827b1`, Cross-platform Owner. Prior dirty changes preserved; uncommitted/not pushed, no formal handoff.
- **CROSS_PLATFORM_CHANGE_REQUIRED:** `SessionWriterDomain::pin` returns a non-cloneable Unix `PinnedSessionWriter` borrowing the bound writer and retaining a private root descriptor. Writes recheck session/path authorization, use descriptor-relative create-new, and sync file/parent. No descriptor escapes. Regression renames the pinned root and creates a same-name replacement: writes reach only the original directory; traversal/overwrite remain refused.
- **Validation:** Workflow 32/32, doctests 4/4, strict Clippy PASS, captured `CHECK_EXIT=0`, log `/tmp/tw2tg-pinned-writer.log`. Full regression not run for isolated workflow capability; Windows/cgroup acceptance NOT_RUN because this batch tests Unix filesystem behavior only.
- **Limits:** pinning captures the directory at pin time, not necessarily the original adapter-created directory. Descendant directory replacement/rename and moving the pinned root outside staging are not prevented. No external writer isolation, ancestor durability, production lifecycle or B5 publication proof; v2 remains fail-closed.

## Current continuation — session writer compile-time lifetime evidence (2026-10-13)

- **Identity/ownership:** same branch `cross-platform/automatic-pairing-reconcile-20261002` and source HEAD `41636217fc0246751aec6a79fe8880dffc1827b1`; Cross-platform Owner, prior dirty edits preserved, uncommitted/not pushed, no formal handoff.
- **Verification only:** three compile-fail doctests cover consuming the session before later writer use, consuming the domain before later writer use, and requiring Clone on the bound writer. A positive generic example proves ordinary resolution compiles. No runtime/API behavior changed.
- **Validation:** doctests 4/4 (positive 1, compile-fail 3), Workflow units 32/32, strict Clippy and fmt-check PASS; captured exit 0, log `/tmp/tw2tg-writer-lifetime.log`. Full regression not run for documentation/type-contract tests only. Native Windows and cgroup containment NOT_RUN; no native behavior changed or acceptance claimed.
- **Limits:** Rust borrow evidence does not pin directory identity or constrain external writers. B4 pinned capability/containment/production lifecycle and B5 remain open; v2 fail-closed.

## Current continuation — descriptor-relative existing attempt query (2026-10-13)

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD `41636217fc0246751aec6a79fe8880dffc1827b1`, current owner Cross-platform Owner. Prior dirty edits preserved; uncommitted, not pushed, no formal handoff.
- **CROSS_PLATFORM_CHANGE_REQUIRED:** existing attempt inspection now traverses each component with retained parent descriptors and `openat` using `DIRECTORY`, `NOFOLLOW`, and `CLOEXEC`, removing path-based metadata checks. It never creates missing directories. Regression covers valid lookup, linked staging root, missing attempt, regular-file attempt and linked attempt; existing linked-Job regression remains passing.
- **Validation:** Workflow 32/32, strict Clippy and fmt-check PASS; aggregate exit 0, log `/tmp/tw2tg-recovery-descriptor.log`. Full workspace regression not run for this isolated workflow adapter change. Windows native and cgroup checks NOT_RUN this batch because it only verifies the Unix lookup primitive, not native containment; those acceptance gates remain open.
- **Limits:** descriptors are not retained in the returned path binding. This lookup is not a recovery/publication permit and does not guarantee later namespace stability or exclude concurrent writers. Ancestor durability, pinned writer capability, production containment/lifecycle and B5 remain open; v2 stays fail-closed.

## Current continuation — descriptor-relative fresh attempt creation (2026-10-13)

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD `41636217fc0246751aec6a79fe8880dffc1827b1`, Cross-platform Owner; prior dirty edits preserved. Uncommitted, not pushed, no formal handoff.
- **CROSS_PLATFORM_CHANGE_REQUIRED:** fresh staging/Job/attempt creation now uses retained parent descriptors with `mkdirat` and `openat(NO FOLLOW)` (implemented as `NOFOLLOW`). New directories request owner-only permissions; existing directories are not chmodded. Existing attempts remain exclusive/refused. Regression checks linked staging ancestor refusal, no outside creation, nested creation and private attempt permissions.
- **Validation:** Workflow 31/31, strict Clippy and fmt-check PASS; captured `VALIDATION_EXIT=0`, log `/tmp/tw2tg-mkdir-final.log`. Terminal integration completion reporting failed separately. Full regression not run for this isolated workflow adapter change. Windows/cgroup checks NOT_RUN this batch; follow-up remains with their native owners/capable environment.
- **Limits:** returned domain remains a path binding, not a pinned directory capability; concurrent rename/replacement stability and ancestor crash durability are not established. Recovery inspection still uses path-based checks. B4 production containment/lifecycle and B5 remain open, v2 fail-closed.

## Current continuation — B4 parent-directory sync (2026-10-13)

- **Sync-failure follow-up:** a private sync seam now exercises the production write sequence with deterministic file-sync and parent-sync errors. Both return `Io`, stop at the failed sync, preserve the created payload and reject overwrite on retry. The production entry always uses `File::sync_all`; no public injection API was added. Workflow 30/30, strict Clippy and fmt-check PASS, captured `VALIDATION_EXIT=0` in terminal output. Terminal integration separately reported an erroneous close state; this is not product failure. Initial Clippy test-module placement failure was corrected. Power-loss durability, Windows and containment acceptance are not claimed.

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD `41636217fc0246751aec6a79fe8880dffc1827b1`; current owner Cross-platform Owner. Prior dirty changes preserved; this continuation is uncommitted, not pushed, and not a formal handoff.
- **CROSS_PLATFORM_CHANGE_REQUIRED:** Unix session-bound create-new writes now sync the retained immediate parent descriptor after syncing the file. Either sync failure returns an error; a created file can remain after failure. Tests cover nested payload and empty-file writes. Ancestor durability, namespace stability, process containment and publication authorization are not established.
- **Validation:** Workflow unit tests 29/29 PASS; strict Workflow Clippy and fmt-check PASS. Linux WSL2 only; Windows native and cgroup containment checks NOT_RUN in this continuation. B4/B5 remain incomplete and v2 remains fail-closed. Full regression not run because this change is confined to the workflow file-write primitive.

## Current batch — explicit supervisor shutdown results (2026-10-13)

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, starting HEAD `6567691d26f1f6b33f8153edcc95013bb79012d0`; prior uncommitted changes were preserved. Current owner: Cross-platform Owner. This work is uncommitted and is not a formal Windows handoff.
- **Implementation:** aria2 and Sidecar shutdown now return bounded cleanup outcomes, and the Sidecar stop command plus aria2 transfer caller consume failures. A successful transfer with failed cleanup is now a transfer failure. Reader panic/timeout, secret-file removal retry, and shutdown retry behavior have targeted tests. B4 attempt-bound writer authorization, B5 publication/recovery integration, and fail-closed v2 enablement were not changed.
- **Validation:** `cargo test -p xarchive-sidecar-supervisor -p xarchive-download` PASS (16 and 52 unit tests, plus 7 transfer-driver tests); `cargo check --manifest-path desktop/src-tauri/Cargo.toml` PASS; `cargo fmt --all` PASS. No cgroup containment or Windows-native validation was performed. These checks are local lifecycle evidence only, not B4/B5 acceptance.

## Continuation — attempt-bound writer-domain contract (2026-10-13)

- **Descriptor-relative create follow-up (2026-10-13, `CROSS_PLATFORM_CHANGE_REQUIRED`):** Unix `SessionWriterDomain::create_new_file` borrows the held session and uses existing rustix open/openat APIs with DIRECTORY/NOFOLLOW/CLOEXEC and create-new EXCL. Every root and relative-directory component is opened separately; bytes are written and file-synced without returning a writable handle. Root/intermediate/final symlink and overwrite regressions pass. This is not containment or publication: all directories must exist, rename/namespace stability and directory-entry crash durability are not guaranteed, and partial failures are never silently reused. Production v2 stays fail-closed; Windows adapter remains Windows Owner work. Source HEAD/branch and uncommitted ownership unchanged. Validation log: `/tmp/tw2tg-descriptor-validation.log`; Workflow 29/29 and crate strict Clippy PASS, aggregate exit 0. Full regression and native platform containment not run for this isolated primitive.

- **Live-session binding follow-up (2026-10-13, `CROSS_PLATFORM_CHANGE_REQUIRED`):** added `WriterDomain::bind_session` and non-cloneable `SessionWriterDomain`, borrowing both the domain and held `ExecutionSession`. Binding rejects stale/foreign identities and relative resolution rechecks the held session. Returned paths remain ordinary path bindings, not transferable I/O or publication permits. Production wiring, descriptor-relative I/O, containment and B5 remain open; v2 stays fail-closed. Branch/source HEAD remain `cross-platform/automatic-pairing-reconcile-20261002` / `41636217fc0246751aec6a79fe8880dffc1827b1`; all prior edits preserved, current owner Cross-platform, no commit/push/handoff. PASS: Workflow unit tests 28/28, formatter and diff check, aggregate exit 0 captured in `/tmp/tw2tg-writer-session-validation.log`. Full workspace/Windows/cgroup validation not run for this isolated shared contract change; no native acceptance inferred.

- **Boundary correction (2026-10-13):** writer-domain adapters now reject unsafe Job path components and non-absolute/traversing roots before creation; existing-root inspection checks all ancestors for links and missing directories. Added unsafe-identity and linked-ancestor/missing-payload regressions. `WriterDomain` is a path binding, **not** a live session authorization: its public adapters accept an identity without a held session. Lexical relative-path resolution does not protect against symlinks below the root or concurrent directory replacement; descriptor-relative I/O and OS containment remain open. Workflow unit-test output reports 27/27 passed, but terminal integration returned contradictory completion status, so process exit is not claimed from that run. No production enablement or formal handoff.

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, starting HEAD `41636217fc0246751aec6a79fe8880dffc1827b1`; prior working-tree edits were preserved. Current owner: Cross-platform Owner. This batch is uncommitted and is not a formal Windows handoff.
- **Implementation (`CROSS_PLATFORM_CHANGE_REQUIRED`):** added `xarchive-workflow::WriterDomain` and `WriterPath`, binding safe relative-path resolution to an `AttemptIdentity`, plus a Unix adapter for exclusive fresh attempt-root creation and existing-root inspection. Production paths are not yet wired to this API. The contract does not contain arbitrary writers, revoke raw filesystem access, prove process-tree exit, or authorize publication; no B4/B5 completion is claimed.
- **Validation:** `cargo fmt --all -- --check` PASS; `cargo test -p xarchive-workflow --lib writer_domain -- --test-threads=1` PASS (3/3); `cargo check -p xarchive-workflow --lib` PASS; `git diff --check` PASS. Full regression not run because this is an isolated new workflow module and prior dirty changes are retained.
- **Environment:** Linux x86_64 WSL2; cgroup v2 controllers are visible, but `/sys/fs/cgroup` is not writable. Writable-cgroup writer-containment acceptance is `BLOCKED` by environment capability; no Windows-native validation was performed (`NOT_RUN`). Continue to keep v2 fail-closed. Next gate is a stronger OS-backed writer containment/exit proof integrated with attempt ownership, followed by delayed-writer production-path tests and only then B5 publication/finalization work.

## Current batch — B4 writer-lifetime boundary review (2026-10-13)

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD `41636217fc0246751aec6a79fe8880dffc1827b1`, equal to origin before this continuation. Current owner: Cross-platform Owner. Worktree also contains pre-existing uncommitted edits to `desktop/src-tauri/src/commands.rs` and the two B4 documents; all are preserved. No commit, push, or formal Windows handoff is claimed.
- **B4 finding:** reviewed `ProductionExecutionFactory::create_attempt_execution`, `ArchiveExecutionContext::download_v2`, both production download backends, `SidecarSupervisor`, `Aria2Supervisor`, and `ArchiveService`. Media downloader staging is attempt-scoped; metadata, Telegram intent, and completion still use the Job-scoped `ArchiveService` path. Sidecar `download_completed` is not proof of descendant exit. Sidecar reader shutdown is bounded and explicitly does not prove escaped descendants exited. aria2 process-group cleanup has Linux prerequisite tests, but no production-path writer-exit authorization currently spans through archive verification/completion. An additional read after cleanup would not establish that no writer remains.
- **Decision:** no completion-path or recovery/publication code was changed. Tracing the Sidecar shutdown handler showed that shutdown during an active download sets the interruption flag; it is not a non-cancelling drain/idle protocol and cannot be used as a normal-completion barrier. A future worker-level idle event alone would still not prove native process-tree exit. `SessionExecutionBoundary` already owns attempt and Job/destination authorization, but the production execution object crosses a `'static` runner channel and recovery/terminal persistence are not yet integrated into that boundary. B4 remains `IN_PROGRESS`, B5 remains `PLANNED`, and `InternalRenameV2` remains fail-closed. The next implementation gate is a non-cancelling Sidecar drain/idle-and-exit contract (or equivalent OS-backed supervision), shared attempt-scoped exit authorization for Sidecar and aria2, and cohesive runner/recovery/completion/finalization ownership before enabling production; then add production-path delayed-writer tests. Do not treat existing supervisor fixtures as end-to-end B4 evidence.
- **Existing uncommitted shutdown change:** `commands.rs` retains the Sidecar and reports shutdown incomplete when output reader threads remain unfinished. This avoids reporting `stopped` while owned readers still run; it is not process-tree proof, attempt staging wiring, or publication authorization. It remains uncommitted and was not changed in this batch.
- **Validation/review:** inspected production factory/archive/download paths, the session-boundary execution/finalization lifecycle, Sidecar v2 shutdown handling and gallery-dl process lifecycle, Sidecar supervisor exit/shutdown semantics, and aria2 supervisor plus its Unix delayed-writer fixture. The fixture proves only its Linux process-group prerequisite, not production authorization. No code tests were run because no code changed; Windows behavior remains `NOT_RUN`. Existing prior-round validation is recorded separately below and is not claimed as new evidence for this continuation.

## Continuation — existing attempt staging containment (2026-10-12)

- `CROSS_PLATFORM_CHANGE_REQUIRED`: the production factory now rejects the
  legacy unfenced `create` route and carries the claimed `AttemptIdentity` from
  `create_for_attempt` into the archive context. Sidecar staging resolution uses
  only the matching Job's `attempt-N` directory; missing/mismatched identity
  fails closed. Tests cover attempt 7 isolation and legacy rejection.
- This is partial B4 wiring, not full production commit integration: metadata
  and temporary files, complete reported-path validation, writer/process-tree
  join, and B5 fenced publication/finalization remain open. `InternalRenameV2`
  still fails before Sidecar execution. Windows not run; worktree uncommitted.

- `CROSS_PLATFORM_CHANGE_REQUIRED`: `FileStore::existing_attempt_staging_dir`
  now rejects a staging root or Job ancestor that is a symlink/reparse point,
  matching the fresh-attempt creation boundary. This closes an ancestor-swap
  escape in attempt recovery/preview path resolution.
- Added Unix regression replacing both staging root and Job ancestor with
  symlinks; both are refused. PASS: targeted 1/1, Storage 125/125,
  fmt-check, diff-check and docs-audit.
- The first targeted invocation selected zero tests due to an over-specific
  `--exact` filter; it was not counted as evidence. The corrected invocation
  selected and passed the regression. This is containment hardening only, not
  B4 production wiring or B5 publication authorization. Windows not run.
- Branch/HEAD unchanged (`cross-platform/automatic-pairing-reconcile-20261002` /
  `6567691`); all modifications remain uncommitted.

## Continuation — idempotent aria2 terminal polling (2026-10-12)

- `CROSS_PLATFORM_CHANGE_REQUIRED`: after natural-exit cleanup, repeated
  `is_running()` now returns `false` instead of `SupervisorNotRunning`; terminal
  polling and waiting are both idempotent. Regression covers poll, wait, poll.
- PASS: targeted regression 1/1; Download 51/51; fmt-check, diff-check and
  docs-audit. Windows validation NOT RUN. This does not upgrade best-effort Unix
  process-group cleanup to full process-tree authorization or complete B4/B5.
- State remains uncommitted on branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `6567691`. Remaining shared Plan work is not classified as Windows-blocked.

## Current batch — cross-session handoff closeout (2026-10-13)

- **B4 aria2 process-group prerequisite (2026-10-12, CROSS_PLATFORM_CHANGE_REQUIRED):** Unix aria2 now starts in an isolated process group and uses group termination on startup failure, shutdown and natural parent exit; natural exit also removes its RPC secret. Reuses repository nix 0.29 via a Unix-only dependency. Real shell late-write regression PASS, Download 51/51 PASS, Desktop 295/295 PASS, captured exits 0. Windows process adapter unchanged; full escaped-descendant containment and production attempt publication remain open. Adds dirty Download Cargo.toml/supervisor.rs files; old dirty counts are historical.

- **B4 incomplete-stop refusal (2026-10-12, CROSS_PLATFORM_CHANGE_REQUIRED):** `wait_for_exit` no longer reports success when owned readers remain unfinished, and supports retry after the direct child has exited. Desktop `stop_sidecar` retains the supervisor and records an error instead of returning `stopped` when readers remain. Added blocked-reader/retry and timeout-preserves-child regressions. Supervisor 15/15 PASS. Desktop rerun evidence is recorded in the Plan checkpoint; no complete writer-tree or production B4/B5 acceptance is claimed. This update also modifies `desktop/src-tauri/src/commands.rs`; prior dirty-file counts are historical.

- **B4 supervisor prerequisite (2026-10-12, CROSS_PLATFORM_CHANGE_REQUIRED):** supervisor owns stdout/stderr reader handles, joins finished readers with bounded shutdown waiting, and exposes reader completion without claiming process-tree proof. Natural parent exit now invokes existing process-group cleanup before discarding the Child identity. Unix shell fixtures cover idempotent shutdown and a parent exiting while a descendant holds pipes. Supervisor 13/13 PASS; final Desktop rerun 295/295 PASS, both Cargo exit 0; fmt/diff-check/docs-audit PASS. Production attempt staging/publication and full writer-tree proof remain open. Worktree now additionally modifies the two supervisor source files; the earlier nine-file count is historical.

- **Attempt-aware factory prerequisite (2026-10-12):** `JobExecutionFactory::create_for_attempt` is an object-safe, fail-closed shared port (`CROSS_PLATFORM_CHANGE_REQUIRED`). The session passes its claimed `AttemptIdentity`; factories without explicit attempt support must not silently create Job-level writers. Probe factories assert the Job and claimed attempt. Production still refuses this port until cohesive B4/B5 integration. PASS: session tests 6/6, Desktop module 295/295 (Cargo exit 0), fmt-check and diff-check. B4–B7/D1/F2/F3 remain implementation work, not Windows blockers; this does not complete the Plan.

- **Prerequisite defect fix (2026-10-12):** factory errors after the B3 durable claim now enter the session's existing failure-finalization branch instead of escaping through `?` and leaving the Job `DOWNLOADING`. A regression proves persisted `FAILED` plus a failure event, scope release and runner admission release. Targeted session tests 6/6 PASS with captured Cargo exit 0. This is shared lifecycle behavior (`CROSS_PLATFORM_CHANGE_REQUIRED`). C1 B4–B7, D1 and F2/F3 still require implementation; they are not Windows-only blockers and the all-non-Windows request is not complete.

- **Latest resumption evidence (local date 2026-10-12):** B3 probes now require explicit `LockError::Contended` for both Job and destination scopes at factory creation, execution and durable state writes; arbitrary lock errors no longer count as proof of ownership. Targeted `executor::session_boundary_tests` 5/5 PASS, formatting PASS, diff-check PASS. The terminal integration reported closure, but the separately reread complete test log confirms the test result. Earlier module totals below are retained historical evidence, not newly rerun totals. B4/B5 remain NOT RUN/not implemented; the all-non-Windows goal is incomplete. Implementation sequencing gates are not Windows environment blockers. The existing 2026-10-13 heading is retained from the supplied checkpoint; it is later than this environment's local date.

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD `6567691d26f1f6b33f8153edcc95013bb79012d0`, equal to upstream. The worktree has **nine** dirty files and none are committed (the earlier closeout recorded six; the B3 session added three — see the difference note below): `crates/xarchive-storage/src/file_store.rs` (non-behavioral `#[deprecated]` guard on `attempt_staging_dir` plus internal test migration to `create_attempt_staging_dir`), `desktop/src-tauri/Cargo.toml` (new `xarchive-workflow` dependency), `desktop/src-tauri/src/executor.rs` (C1 B3 `SessionExecutionBoundary` + 5 tests), `Cargo.lock`, and five documentation files (`docs/development/desktop-download-output-plan.md`, `docs/development/roadmap.md`, `docs/status/platform-handoff.md`, `docs/validation/windows-manual-steps.md`, `docs/validation/windows-queue.md`). Current owner is Cross-platform Owner. This is not `READY_FOR_WINDOWS`: no commit, push, formal handoff, or Windows acceptance is claimed.
- **This session's work:** implemented the C1 B3 runner-owned execution-session boundary — the missing owned persistence/coordinator/session surface the C1-A checkpoint identified. `SessionExecutionBoundary<P, C>` owns a `JobPersistence` context and a `Coordinator` (generic over the shared port) and runs runner-admission → Job+destination scope → durable claim-under-guard → factory-after-claim → single-active-runner dispatch with the boundary's owned snapshot → attempt-fenced terminal write + finalization → session released last, with the runner reservation held through finalization. Contention, a lost claim, and a saturated runner are all non-failing/reschedulable (no attempt, no `FAILED`). Five regression tests prove it, including a compile-time `Send` assertion for the `'static` scheduler thread. This is a `CROSS_PLATFORM_CHANGE_REQUIRED` shared change (new `desktop/src-tauri → xarchive-workflow` dependency; boundary generic over the shared `Coordinator`); the Windows `Coordinator`/NTFS adapters remain Windows Owner work. Production download entry is **not** yet switched onto the boundary (B4/B5 gating), so production v2 stays fail-closed.
- **Difference from the earlier closeout (repo is authoritative):** the task's opening note described six dirty files. The repo actually carries nine: the three additional files (`Cargo.lock`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/src/executor.rs`) are the B3 implementation added by this continuation and are uncommitted. Desktop module tests are now 294/294 (was 289/289) because of the five new `session_boundary_tests`.
- **Validation on the current dirty tree:** `cargo fmt --all -- --check` PASS; `cargo check -p xarchive-desktop --lib` PASS; `cargo test -p xarchive-storage --lib -- --test-threads=1` 124/124 PASS; `cargo test -p xarchive-workflow --lib -- --test-threads=1` 22/22 PASS; `cargo test -p xarchive-desktop --lib -- --test-threads=1` 294/294 PASS; `git diff --check` PASS; `node scripts/docs-audit.mjs` PASS. NOT_RUN: full workspace regression, strict workspace Clippy (pre-existing lint failures recorded earlier), Node/Sidecar suites — no full cross-module regression was required for this targeted session-boundary addition. Windows-native checks remain `NOT_RUN`/`BLOCKED`; no handoff-bound artifact exists.
- **Remaining-work disposition:** C1 B4 (attempt-owned staging + proven writer/process-tree exit), B5 (fenced publication and transactional finalization) and switching the production download entry onto the boundary, then B6 (replay/restart matrix) and B7 (Desktop recovery integration) are cross-platform work the next task continues — B4/B5 must land before the production entry moves onto the boundary, and B6/B7 depend on B1–B6. E's persisted tool arguments and Sidecar/aria2 argv wiring and the F2/F3 shared contracts remain open. D1 stays gated on C1 and F2/F3 stay gated on C1+D1 per the approved order — do not start them. The reverted service-recovery attempt in `archive_service.rs`/`archive_recovery.rs` (failed `cargo check -p xarchive-storage --lib`) must not be repeated as a partial change; a complete owned persistence/coordinator/session boundary is required first (now built and tested). Windows-only acceptance stays consolidated in `docs/validation/windows-queue.md` and `docs/validation/windows-manual-steps.md` and requires a formal Git handoff plus a fresh artifact (Windows Platform Owner). ZIP/7z remain deferred pending a separate dependency/security/license decision.


## Current batch — `/goal` non-Windows continuation (2026-10-13 update)

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`, starting HEAD `6567691d26f1f6b33f8153edcc95013bb79012d0`, equal to upstream. Three pre-existing uncommitted documentation edits were retained; this update changes only documentation, and the worktree additionally carries the non-behavioral `FileStore::attempt_staging_dir` deprecation guard in `crates/xarchive-storage/src/file_store.rs` (no behavior change). Current owner is Cross-platform Owner. This is not `READY_FOR_WINDOWS`: no commit, push, formal handoff, or Windows acceptance is claimed.
- **Scope/result:** C1 B3–B7 remain unsafe to integrate independently: claim precedes factory execution, production uses Job-level staging, publication and finalization are inside the execution object, runner admission ends before post-run persistence, and the Storage recovery failpoint is unused. A caller-local reservation change cannot establish the required owned persistence/coordinator/session boundary. A new non-behavioral guard now marks `FileStore::attempt_staging_dir` deprecated so fresh-attempt creation is directed to the exclusive `create_attempt_staging_dir` helper; all internal callers/tests have been migrated off the deprecated path. v2 remains fail-closed; D1 remains gated on C1. F2/F3 remain after D1 in the approved sequence; unresolved concrete schemas and failure contracts are not guessed. No production behavior changed and no feature completion is claimed.
- **Linux validation:** Rust formatting and Storage file-store tests were run for this batch. `cargo fmt --all -- --check` PASS; `cargo test -p xarchive-storage --lib file_store -- --test-threads=1` 12/12 PASS; `git diff --check` PASS; `node scripts/docs-audit.mjs` PASS. The previously recorded Workflow/Storage/Desktop module PASS results remain primitive/module evidence only and do not establish integrated C1 acceptance. Full regression was not run.
- **Environment/Windows:** Ubuntu x86_64 under WSL2; `/mnt/c`, `powershell.exe` and `DISPLAY=:0` exist, but no native Windows Desktop runtime, handoff-bound artifact, verified disposable NTFS fixture, or artifact-bound evidence capture was established. No Windows check was run or inferred. Integrated C1/D1/F2/F3 remain implementation-gated; Windows adapter/NTFS/process/GUI checks remain `NOT_RUN`. Manual steps are consolidated in `docs/validation/windows-queue.md`.

## `/goal` continuation disposition — 2026-10-13

The Plan's B1/B2 statuses were reconciled with the inspected call graph; C1 B3–B7 remain shared implementation work and v2 remains fail-closed. F2/F3 were not started because the approved dependency sequence gates them on C1 and D1, and their precise shared contracts remain open. No production files changed. Documentation audit and `git diff --check` pass; full regression was not run. Windows-native work remains queued for a formal Git handoff and fresh artifact.

## 2026-10-12 approved-scope continuation — attempt preview primitive only

- Current branch: `cross-platform/automatic-pairing-reconcile-20261002`;
  source HEAD remains `344322ba7cd1a75588f5def746eb061451114c87` during this
  uncommitted batch. Current owner: Cross-platform Owner. Existing five dirty
  files from the previous continuation were reviewed and retained; this batch
  additionally changes Storage source/tests and the Plan. No commit, push, or
  formal Windows handoff is claimed.
- The user approved the sequence C1 (B3–B7) → D1 → F2 → F3 → integrated G.
  F3 decisions are now explicit in the Plan: per-record conflict preview and
  partial acceptance committed atomically; reject apply when tasks are active;
  ship individual record files before ZIP/7z. These are product decisions, not
  implementation claims.
- Implemented one bounded Storage primitive: `ArchiveService::preview_sidecar_archive_attempt`
  requires an attempt number and reads only that existing attempt staging tree.
  Added tests proving a distinct attempt payload is used rather than legacy
  staging and that a legacy-only payload does not permit fallback. This does not
  wire Desktop execution or enable D1/C1 v2.
- The C1 lifecycle remains unsafe to enable: production execution still claims
  before factory creation, `download_v2` writes to Job-level staging, the
  archive commits inside the execution job, v2 recovery explicitly fails closed,
  and runner admission currently releases before post-run persistence. B3–B7
  require a combined runner-owned claim/session, writer-exit, fenced publication
  and transactional finalization/recovery design. D1, F2, F3 implementation and
  integrated G are not complete. No Windows behavior is inferred.
- Linux validation: initial red run failed at compile time because the API was
  not yet implemented, as expected; after implementation, the targeted tests
  pass (2/2) and the Storage library passes (124/124). `cargo fmt --all --
  --check`, `node scripts/docs-audit.mjs`, and `git diff --check` pass. Full
  workspace, Desktop/Workflow module suites, and Windows validation are not run
  for this bounded Storage API change.

## Current cross-session status — 2026-10-12 (attempt staging hardening; no Windows handoff)

- **Current identity:** branch `cross-platform/automatic-pairing-reconcile-20261002`,
  source HEAD `344322ba7cd1a75588f5def746eb061451114c87`, equal to upstream at
  inspection; working tree contained three pre-existing uncommitted documentation
  edits and now has five modified files, including the Storage staging helper and
  regression tests. Current owner is Cross-platform Owner. No formal Windows
  handoff has occurred. Refresh
  source SHA and dirty-state after the documentation batch is committed; this
  working tree is not itself a handoff.
- **Goal status:** the user's `/goal` is to complete all Plan development/tests
  independent of Windows and then consolidate Windows-specific manual checks.
  This is **not complete**. B1/B2 are implemented, B3's two-phase Workflow
  contract is tested, but Desktop production does not create/hold a session.
  B4 attempt staging/writer-exit proof, B5 fenced publication/finalization, B6
  replay/restart matrix and B7 Desktop recovery integration remain shared gates;
  v2 recovery and D1 remain fail-closed/gated. F1 shared preference/pre-render
  theme/token implementation is present; F2 remains planned but its maintenance-mode
  wait-and-apply decision is confirmed; F3 has open shared
  schema/dependency/transaction design gates. E3 shared actual-spawn evidence is
  recorded in the Plan. Integrated G is still open.
- **Next shared sequence:** C1-A call-graph review established that the current
  entry durably claims before factory creation and the factory prepares Job-scoped
  staging; there is no safe standalone session insertion point. No regression
  hook was added because current contracts cannot prove lock-before-claim without
  changing production interfaces. Combine runner-owned reservation-before-claim
  with attempt-isolated staging and proven writer/process-tree exit; add
  attempt-fenced publication and transactional DB/Telegram finalization; complete
  service replay/restart matrix; integrate Desktop recovery. F2's
  design is confirmed: explicit maintenance mode blocks new cross-process admission,
  drains active tasks through finalization, allows cancellation before apply, rechecks
  preview revision, persists before updating runtime state, and releases maintenance
  on every outcome. F2 implementation remains planned; F3's remaining §11 shared
  schema/dependency/transaction questions remain open. No Windows adapter absence is
  a blocker to these shared tasks.
- **Ownership and platform status:** lack of the Windows coordinator is not a
  blocker to shared Linux design/implementation. Linux owns the shared contract and
  Linux adapter verification; Windows Owner owns its coordinator, NTFS/native
  process semantics, and Windows acceptance. Do not infer these from Unix tests.
  Current environment is Ubuntu under WSL/Linux; no Windows validation or fresh
  Windows artifact was used in this batch.
- **Environment and validation:** Ubuntu x86_64 under WSL2; Rust/Cargo 1.98.0,
  Node 26.7.0 and Python 3.14.4 detected. `/mnt/c`, `powershell.exe` and
  `DISPLAY=:0` are available, but no native Windows Desktop runtime, verified NTFS
  archive target, fresh handoff-bound Windows artifact or evidence capture tied to
  that artifact was available; no Windows check is inferred from WSL capability.
  Fresh Linux validation on this uncommitted continuation: `cargo fmt --all
  -- --check` PASS; Workflow `cargo test -p xarchive-workflow --all-targets --
  -- --test-threads=1` PASS (40 tests: 22 unit + 11 coordination + 5 session +
  2 no-replace); Storage library 122/122 PASS;
  Desktop library 289/289 PASS; docs audit and `git diff --check` PASS. A prior
  parallel Cargo run produced five Workflow cross-process failures; the serial
  rerun passed, so those failures are retained as test-resource interference,
  not product evidence.
  These module tests do not verify production session wiring or integrated C1. Windows GUI,
  NTFS/native coordinator/process and artifact-bound checks remain `NOT_RUN` with
  manual steps in `docs/validation/windows-queue.md`; no formal handoff is claimed.

## 2026-10-12 C1-A continuation — design boundary and shared docs reconciliation

- Current source remains HEAD `344322ba7cd1a75588f5def746eb061451114c87`;
  documentation synchronization is uncommitted (four modified documentation
  files). No formal handoff or production
  source change has been made in this continuation.
- Confirmed C1-A architecture: preserve object-safe Executor/factory boundaries;
  a concrete generic runner owns coordinator, session and independent persistence
  context, performing reservation before durable claim. Current call graph claims
  before factory creation and then uses Job-scoped staging, so no safe standalone
  production insertion point exists. The current implementation checkpoint is
  limited to design review; no production seam/test hook was added because it would
  not establish cross-process lock-before-claim without changing production
  interfaces. Combine production session wiring with B4/B5. B4/B5/B6/B7 remain
  gated and v2/D1 remain fail-closed.
- F2 maintenance-mode wait-and-apply behavior is confirmed: cross-process
  admission closure, graceful active-task drain, cancellable wait, revision
  recheck, durable-before-runtime apply, and maintenance release/recovery on every
  outcome. Implementation remains `PLANNED`; F3's remaining shared design gates
  remain open.
- Plan, architecture overview and Windows queue were synchronized without changing
  unrelated ownership or historical evidence. The first documentation-only
  checkpoint ran `node scripts/docs-audit.mjs` and `git diff --check` (both PASS);
  the expanded Linux module sweep is recorded in the following `/goal` checkpoint.
   No Windows runtime, NTFS, native process, or artifact-bound acceptance was run.

## 2026-10-12 `/goal` continuation — attempt staging hardening and validation

- Branch `cross-platform/automatic-pairing-reconcile-20261002`; source HEAD
  `344322ba7cd1a75588f5def746eb061451114c87` (equal to upstream at inspection);
  uncommitted state in `crates/xarchive-storage/src/file_store.rs` and the four
  documentation files listed above.
  Current owner remains Cross-platform Owner; no commit/push or formal Windows
  handoff is claimed.
- Added exclusive attempt staging creation so an existing attempt directory is
  never silently reused. The regression suite covers duplicate creation, an
  existing attempt's data preservation, retry isolation, and symlinked job and
  staging parents. It caught and fixed a real gap: `resolve_within` checked
  descendants but not the staging root itself, allowing a symlinked root to
  redirect directory creation.
- Re-reviewed the live executor/factory, ArchiveService, FileStore, Workflow and
  Storage paths. Production still claims before factory creation, uses Job-level
  staging, and commits inside the execution job; no single safe integration point
  proves writer/process-tree exit, attempt fencing, and finalize ownership. B3–B7
  therefore remain open; v2 stays fail-closed. F2/F3 shared work and integrated G
  remain incomplete. This is not a Windows-only development block.
- Fresh Linux evidence: `cargo test -p xarchive-storage --lib` PASS (122);
  `cargo test -p xarchive-workflow --all-targets -- --test-threads=1` PASS (40);
  `cargo test -p xarchive-desktop --lib` PASS (289); `cargo fmt --all --
  --check`, `node scripts/docs-audit.mjs` and `git diff --check` PASS. The
  Workflow cross-process suite initially failed when run concurrently with other
  Cargo tests (5 failures), then passed serially. Full workspace regression was
  not run: the bounded change touched Storage staging and documentation, while
  the affected Storage, Workflow and Desktop module suites passed.
- The requested goal is not complete: C1 production lifecycle/B4–B7, D1, F2/F3
  shared work and integrated G remain open. Their shared work is not classified as
  Windows-blocked. Windows-specific adapter/NTFS, native process, WebView2/GUI,
  packaging and fresh-artifact checks remain `NOT_RUN` (and integrated cases with
  unmet shared prerequisites remain `BLOCKED/IMPLEMENTATION_NOT_READY`) with
  consolidated manual steps in the Windows queue. Windows validation history is
  unchanged because no Windows result was executed.

## Repository & Documentation Refactoring — 2026-10-11 (shared docs batch; no Windows handoff)

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`; starting HEAD:
  `166837426366ebc9a84afeb306a49140d124bd35`; working tree was clean before this
  documentation batch. Current owner: Cross-platform Owner. This is not a formal
  Windows handoff; no Windows implementation or validation is claimed.
- Updated the Documentation Gate plan, corrected repository-map executor and
  validation-history routes, and retained the old Windows validation archive and
  WDIO handoff paths because they contain unique evidence or active queue
  references. Physical archive movement and queue/history splitting remain
  `NEEDS_REVIEW` pending complete revision/Queue-ID/inbound-reference reconciliation.
- No source code or public API changed in this documentation batch. The executor
  submodule split is already present at current HEAD; any further code split is
  outside scope and requires a separate review/validation batch.
- Follow-up archive work: moved the historical bodies of `testing-strategy.md`
  and `non-windows-completion.md` under `docs/development/archive/`; retained
  their historical prose/evidence and adjusted only relative Markdown links for
  the new directory depth. Kept short compatibility entries at both old paths.
  Other current handoff/queue/Windows evidence files remain unmoved.
- Current follow-up gates are recorded in
  `docs/development/documentation-governance-plan.md`: queue/history and
  handoff/status splitting, WDIO migration, legacy Windows evidence migration,
  release-plan disposition and additional source splitting remain
  `NEEDS_REVIEW`. No Windows validation was run or handed off.
- Validation: `node scripts/docs-audit.mjs` PASS; `node --test
  scripts/docs-audit.test.mjs` 16/16 PASS; tracked Markdown relative-link scan,
  executor-map path check and `git diff --check` PASS. Windows validation is
  `NOT_RUN` and is not needed to claim these documentation-only checks.

## Current state — 2026-10-11 (shared; F1 partial, no Windows handoff)

- Branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD
  `035278e296797564d84c5dc8ff012568f556d2ff`, with uncommitted F1 changes in
  `desktop/src-tauri/src/commands.rs`, `desktop/src-tauri/src/lib.rs`,
  `desktop/src/main.jsx`, `desktop/src/pages/settings-page.jsx`, and their two
  Desktop Node test files, plus the Plan, queue and this handoff. No source CSS
  change remains. Current owner is Cross-platform Owner. This is not a formal
  Windows handoff and no Windows implementation or validation is claimed.
- Committed in this batch: `41f11a5` documentation gate reconciliation,
  `4f07069` shared `ExecutionSession` contract, `ebeaaf8` workspace clippy gate.
  Five commits ahead of `origin` before this continuation is committed and pushed.
  Older sections below that say
  "uncommitted" describe their own historical checkpoint, not the current tree.
- Implementation state: B1 admission/backpressure and B2 atomic
  state-plus-attempt claim are wired into the shared Desktop executor. B3's
  `ExecutionSession` component contract is implemented and proven on Linux with a
  real second OS process, but no production path constructs a session, so no
  Job/destination guard is held across claim, writer shutdown, publication or DB
  finalization. B4–B7 are `PLANNED`. v2 startup recovery and D1 stay disabled;
  v2 remains fail-closed. F1 has an uncommitted typed preference/persistence/UI
  slice; WebView application and token audit remain open. See Plan §13.
- **Windows adapter is not a shared-development blocker.** The absent NTFS
  coordination adapter blocks only (1) enabling the new coordinated lifecycle on
  Windows and (2) Windows/NTFS platform acceptance. It does not block shared
  implementation or Linux verification, so shared B3–B7 work continues on Linux
  and no `WINDOWS_BLOCKING` is declared. The shared batches must not omit the
  session behind a platform `cfg`, must not substitute a no-op coordinator, and
  must not make the existing supported download path fail on Windows.
- C1 destination remains workspace-relative `Tweets/<tweet_id>`; lock-root
  identity must derive from that archive workspace, not an independently guessed
  DB path. Do not enable the production path until attempt staging, child-writer
  shutdown, publication and durable writes are fenced. B3 production wiring,
  B4–B7, C1.4, v2 startup recovery and D1 remain open.
- This continuation verification: Desktop `output_settings_tests` 5/5 PASS;
  full Desktop Node suite 221/221 PASS; full Desktop Rust library suite 288/288
  PASS; Desktop all-target Clippy with `-D warnings`, `cargo fmt --all --
  --check`, `git diff --check`, Vite production build and docs audit PASS. Vite
  emitted the existing mixed static/dynamic Tauri API import warning. Full
  workspace regression was not run. Windows validation is `NOT_RUN`; manual
  steps and exact deferral reasons are in `docs/validation/windows-queue.md`.

## P1 two-phase authorization — 2026-10-11 (shared; uncommitted)

- Branch `cross-platform/automatic-pairing-reconcile-20261002`, source
  `ebeaaf8e85679f24169b7be393e1f24df23b49a1`. This batch is uncommitted and not
  pushed. Current owner is Cross-platform Owner. This is not a formal Windows
  handoff and no Windows validation is claimed.
- Changed shared modules: `crates/xarchive-workflow/src/execution.rs` (new
  `ExecutionReservation`, `ClaimOutcome`, `SessionError::ClaimLost`/
  `ClaimRejected`/`is_reschedulable`, destination-bound session),
  `crates/xarchive-workflow/src/lib.rs` (re-exports and crate docs),
  `crates/xarchive-workflow/src/bin/lock_probe.rs` (claims through the two-phase
  entry point), and `crates/xarchive-workflow/tests/execution_session.rs`
  (`claim_session` helper). No Desktop, Storage, Sidecar or shared-UI file
  changed, and no other shared API was altered.
- Why `ExecutionSession::begin` was removed rather than extended: it accepted an
  `AttemptIdentity` the caller had already produced, so a caller could claim
  first and lock second. That ordering increments the attempt and only then
  discovers a peer owns the job, leaving the attempt advanced with no writer
  ever serialized. Expressing the ordering as a callback makes it impossible to
  invert, which a parameter cannot do.
- What this does and does not establish: the shared contract now enforces
  lock-then-claim and binds a session to the attempt the claim returned and to
  its final destination. No production path constructs a session yet, so no
  Job/destination guard is held across claim, writer shutdown, publication or DB
  finalization. B4–B7, v2 startup recovery and D1 remain open and v2 stays
  fail-closed.
- Open design question recorded for P2: the executor currently claims in the
  caller and then sends work to the runner, so B3's runner-owned claim is not
  yet where the Plan places it. P2 must settle that ownership rather than add a
  second lock layer.
- Verification: `cargo fmt --all -- --check` PASS; `cargo test -p
  xarchive-workflow --features __lock_probe` 22 lib + 11 coordination + 5
  execution-session + 2 no-replace PASS; `cargo clippy -p xarchive-workflow
  --features __lock_probe --all-targets` clean; `cargo check -p
  xarchive-workflow --target x86_64-pc-windows-msvc --all-targets` clean with and
  without `__lock_probe`; `cargo test --workspace --no-fail-fast` 26/26 binaries
  ok, 0 failed. Mutation checks: dropping the destination scope and accepting a
  foreign-job identity each fail the suite; inverting the ordering was rejected
  by the borrow checker.
- Windows work required: unchanged — implement the NTFS coordination adapter
  against the same `Coordinator` port, then validate native session behaviour on
  a fresh artifact bound to the formal handoff SHA. Do not infer NTFS behaviour
  from the Unix `flock` evidence.

## Release clippy gate cleared — 2026-10-11 (shared; historical checkpoint)

- Branch `cross-platform/automatic-pairing-reconcile-20261002`, source
  `4f070691af3d8bf810fe613cd7652714709470f6` (B3 shared execution-session
  contract). This lint batch was uncommitted at that checkpoint and is now
  committed as `ebeaaf8`. Current owner is Cross-platform Owner; this is not a
  formal Windows handoff.
- `.github/workflows/pre-release.yml` runs
  `cargo clippy --workspace --all-targets -- -D warnings`, so any workspace
  warning failed the release pipeline. There were 4 in `xarchive-desktop` and 6
  in `xarchive-storage`; that command now exits 0.
- Changed shared modules: `desktop/src-tauri/src/transport.rs` (removed a
  redundant self-binding), `desktop/src-tauri/src/websocket_transport.rs`
  (removed a same-type `CloseFrame` conversion and scoped one lint),
  `desktop/src-tauri/src/archive.rs` (scoped lint plus docs),
  `crates/xarchive-storage/src/archive_recovery.rs` (two assertions converted
  to `assert!` with invariant messages), and
  `crates/xarchive-storage/src/database/jobs.rs` (scoped lint plus docs). No
  shared API, signature or behavior changed.
- Deliberately not done: the four `too_many_arguments` warnings are scoped with
  `#[allow]` and a comment, not fixed. The fix is to bundle parameters into a
  struct, which changes shared Storage/Desktop signatures that are part of the
  persistence and transport contracts. That needs its own reviewed batch rather
  than an unrelated refactor hidden in a lint cleanup.
- Verification: `cargo clippy --workspace --all-targets -- -D warnings` exit 0;
  `cargo fmt --all -- --check` PASS; `cargo test --workspace --no-fail-fast`
  26/26 binaries ok, 0 failed; Storage claim tests 10/10 and Desktop WebSocket
  tests 13/13 re-run after the edits. Windows/NTFS remains `NOT_RUN`.
- This clears the lint gate only. It is not Plan progress: B3 production
  wiring, B4–B7, v2 startup recovery and D1 remain open and v2 stays
  fail-closed.

## Current GitHub progress — 2026-10-11 (shared commits; no Windows handoff)

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`. E3 downloader
  actual-spawn acceptance: `d846ef9`; N1 runner admission and N2 atomic
  state-plus-attempt claim: `6fc79a2`; initial documentation sync: `a8138ab`.
  These commits are pushed. Current owner remains Cross-platform Owner; this is
  not a formal Windows handoff and Windows implementation/validation is not
  claimed.
- Validation: formatting PASS; Download 50/50, Desktop 286/286, Sidecar
  downloader/protocol 30/30, runner saturation 1/1, Storage claim tests 2/2;
  docs audit and diff check PASS. Windows/NTFS remains `NOT_RUN`.
- Next: Cross-platform Owner continues shared B3–B7 lifecycle implementation;
  Windows coordination/NTFS adapters and native validation remain Windows Owner
  work and do not block shared Linux development.

- Documentation review continuation: corrected the Plan baseline and B1/B2/B7
  wording, synchronized the architecture summary, and clarified Windows adapter
  ownership and `NOT_RUN` status. Final `git diff --check` and docs audit passed;
  no production code changed.
- Current state after the later reconciliation below: that checkpoint's tree was
  clean at `41f11a504c92a20155ac8ebca1a98411c19491f6` plus a documentation-only
  note, and at that moment no `ExecutionSession` helper, Desktop
  `xarchive-workflow` dependency or `xarchive_workflow` executor import remained.
  That is no longer the current state: `4f07069` deliberately reintroduced the
  `ExecutionSession` contract in `xarchive-workflow` as a shared component. See
  the current-state section above.

## B3 shared execution-session contract — 2026-10-11 (shared; committed, historical checkpoint)

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`; source
  `1e7f8c049dbc0f353b76f7babcb399c28bd99b54` (documentation gate
  reconciliation). This work was uncommitted at that checkpoint and is now
  committed as `4f07069`; it is not pushed. Current owner is Cross-platform
  Owner. This is not a formal Windows handoff and no Windows validation is
  claimed.
- Changed shared modules: `crates/xarchive-workflow/src/execution.rs` (new),
  `crates/xarchive-workflow/src/lib.rs` (module declaration and crate docs),
  `crates/xarchive-workflow/src/bin/lock_probe.rs` (new `session` mode plus
  per-item `cfg(unix)` gating), and
  `crates/xarchive-workflow/tests/execution_session.rs` (new). No Desktop,
  Storage, Sidecar or shared-UI file changed, and no shared API was altered.
- What this does and does not establish: `ExecutionSession` is the shared
  authorization token for one attempt, holding the claimed attempt identity and
  a guard over the Job and destination scopes acquired together, with
  `SessionError` separating `Contended` from `InvalidScope`, `Io` and
  `StaleAttempt`. It is generic over the `Coordinator` port, so the shared
  session still exists on Windows. No production path constructs a session yet,
  so no Job/destination guard is held across claim, writer shutdown,
  publication or DB finalization; B3 production wiring, B4–B7, v2 startup
  recovery and D1 remain open and v2 stays fail-closed.
- Windows work required: implement the NTFS coordination adapter against the
  same `Coordinator` port, then validate native session behaviour on a fresh
  artifact bound to the formal handoff SHA. Do not infer NTFS behaviour from the
  Unix `flock` evidence, and do not treat the Linux session tests as Windows
  acceptance.
- Validation required and known risks: the shared contract is fail-closed by
  construction (no adapter means no session means no authorized mutation), and
  the destination must be non-empty so distinct destinations cannot collapse
  onto one lock identity. Contention must remain reschedulable rather than
  becoming a persisted `FAILED`.
- Relevant tests: `cargo test -p xarchive-workflow --features __lock_probe`
  16 lib + 11 coordination + 5 execution-session + 2 no-replace PASS;
  `cargo test --workspace --no-fail-fast` 0 failed; formatting, docs audit and
  diff check PASS; `cargo clippy -p xarchive-workflow --features __lock_probe
  --all-targets` clean; `cargo check -p xarchive-workflow --target
  x86_64-pc-windows-msvc --all-targets` and the same with `--features
  __lock_probe` both clean. Pre-existing and out of scope:
  `cargo clippy -p xarchive-desktop --lib` reports 4 warnings at HEAD.
- Priority and deferred work: B3 production wiring follows, gated on resolving
  the final destination before the durable claim; GUI and packaging checks stay
  in the Windows queue.


## Documentation Gate — 2026-10-11 (passed)

- Status: `PASS` for documentation only. This gate confirms final review and
  mechanical validation of documentation; it does not mark B3–B7 as implemented
  or Windows/NTFS as verified.
- Checks passed:
  - `git --no-pager diff --check` PASS.
  - `node scripts/docs-audit.mjs` PASS:
    - dead relative links: 0
    - unlinked documents: 0
    - stale branch links: 0
    - skills without frontmatter: 0
    - release notes missing from index: 0
    - tracked markdown: 107
  - No production code changed.
- Documentation state: B1/B2 `IMPLEMENTED`; B3–B7 `PLANNED` and not started;
  v2 fail-closed; D1 gated.
- Recording caution: `docs/development/desktop-download-output-plan.md` header
  shows an earlier `bf050...` baseline; that header is corrected to
  `f5e049ef...` with an explicit note that this documentation batch is
  uncommitted. No helper or test from the earlier undocumented attempt was
  retained in code.

## Documentation Gate reconciliation — 2026-10-11 (residue removal; gate re-verified)

- Historical checkpoint note: this gate reconciliation predates the later
  committed shared two-phase `ExecutionSession` contract. It records that
  checkpoint only; the current P2 investigation retains the committed Workflow
  contract and removes incomplete Desktop wiring rather than reverting it.
- Found the committed gate record contradicted by the working tree: an
  undocumented `crates/xarchive-workflow/src/execution.rs` (`ExecutionSession`
  plus a `Noop`-coordinator unit test) remained untracked, with
  `pub mod execution;`, a `coordination`/`execution` re-export block in
  `crates/xarchive-workflow/src/lib.rs`, a new `xarchive-workflow` dependency in
  `desktop/src-tauri/Cargo.toml`, an unused
  `use xarchive_workflow::{AttemptIdentity, Coordinator, ExecutionSession};` in
  `desktop/src-tauri/src/executor.rs`, and a matching `Cargo.lock` entry. The
  unused import would have failed the release clippy gate
  (`cargo clippy --workspace --all-targets -- -D warnings`).
- Removed that residue rather than documenting it. An isolated
  `ExecutionSession` type with a `Noop` coordinator proves nothing about the
  shared lifecycle and would imply session ownership that production does not
  have; the gate record correctly states no helper was retained, so the tree was
  brought back to match the record. The tracked files were restored with
  `git checkout --` and the untracked file deleted; no other production file was
  touched. B3 remains `PLANNED` and the correct contract stays a shared
  `CROSS_PLATFORM_CHANGE_REQUIRED` change carrying Job/destination authorization
  across claim, writer shutdown, publication and DB finalization.
- Gate re-verified on the resulting tree, which is now clean at
  `41f11a504c92a20155ac8ebca1a98411c19491f6` plus this documentation note:
  `cargo fmt --all -- --check` PASS; `git diff --check` PASS;
  `node scripts/docs-audit.mjs` PASS (0 dead relative links, 0 unlinked
  documents, 0 stale branch links, 0 skills without frontmatter, 0 release
  notes missing from index); `cargo test -p xarchive-workflow --lib` 12/12;
  `cargo test -p xarchive-workflow --test coordination -- --test-threads=1`
  11/11; `cargo test -p xarchive-workflow --test no_replace` 2/2;
  `cargo test -p xarchive-desktop --lib` 286/286 including
  `saturated_runner_rejection_is_non_failing_and_reschedulable`; Storage
  attempt-scoped tests 14/14 and staging tests 6/6;
  `cargo clippy -p xarchive-workflow --all-targets` clean.
- Pre-existing, not introduced here: `cargo clippy -p xarchive-desktop --lib`
  reports 4 `useless_conversion`/`needless_borrow`-class warnings at HEAD. They
  are outside this documentation batch and are recorded as a separate finding,
  not fixed, because unrelated refactoring is prohibited. They do gate the
  release clippy job, so they need a dedicated small change.
- This closes the documentation gate only. B3–B7, v2 startup recovery and D1
  remain open; no Windows/NTFS validation or formal Windows handoff was
  performed. Current branch remains
  `cross-platform/automatic-pairing-reconcile-20261002`; current owner is
  Cross-platform Owner.

## Plan B3–B7 lifecycle continuation — 2026-10-11 (analysis; no production change)

- Inspected the current `execute_persisted_from_factory` and
  `execute_persisted_on_worker` paths, `run_runner`/`ProductionExecutionFactory`,
  `ArchiveExecutionJob`, `execute_archive_context`, Sidecar/aria2 orchestration,
  Storage attempt-staging/replay helpers, v2 PREPARED/finalize transactions, and
  startup recovery dispatch. Production Sidecar and archive completion still use
  Job-level staging; the archive completion request has no attempt identity, and
  `JobExecutionResult` returns only after the archive service call.
- Confirmed the in-process runner reservation is deliberately dropped after the
  runner interaction and before persisted terminal-state/archive completion.
  That reservation is a capacity counter, not a cross-process Job/destination
  guard. A current-attempt counter check cannot authorize publication while an
  older writer may still be alive.
- The existing Unix coordinator is intentionally unavailable on non-Unix
  targets. Wiring it directly into shared Desktop lifecycle would either leave
  Windows unprotected or make the shared path unavailable there. A correct B3
  session contract must carry Job/destination authorization across claim,
  factory-created execution, writer shutdown, archive publication and DB
  finalization; this is a shared architecture/contract change
  (`CROSS_PLATFORM_CHANGE_REQUIRED`). The Windows adapter remains Windows Owner
  implementation work and must implement that shared port rather than invent a
  separate contract.
- No production change was retained because an isolated Unix-only guard would
  imply safety without covering the shared production lifecycle. The attempted
  helper/test was removed. Existing checks rerun: workflow unit tests 12/12,
  Unix coordination integration tests 11/11, Desktop runner saturation test
  1/1, Storage attempt-staging isolation test 1/1, `cargo fmt --all -- --check`
  PASS, and `git diff --check` PASS. These establish existing component behavior,
  not B3–B7 integration acceptance.
- B3–B7 remain open; attempt staging/writer join, publication authorization,
  finalize/replay crash matrix and Desktop v2 startup integration are not
  implemented by this continuation. v2 remains fail-closed and D1 gated. No
  Windows/NTFS validation or formal Windows handoff was performed. Current branch
  remains `cross-platform/automatic-pairing-reconcile-20261002`, source
  `f5e049ef3ea21d3f4226245812a96642d4d3ae5b`; worktree has this uncommitted
  handoff note; current owner is Cross-platform Owner.

## C1-A implementation sequence — 2026-10-11 (shared work; docs sync complete)

> **Superseded status note:** this heading and the historical bullets below describe
> the earlier analysis checkpoint. The current documentation synchronization and
> next implementation gate are recorded in the dated section that follows.

- Historical checkpoint source: `bf050ccc5559d1b791bfe335413373fd4ad8b078`.
  The E3 and N1/N2 changes described below have since been committed separately;
  see the current GitHub progress sync section for exact revisions. This is not a
  formal Windows handoff.
- Re-traced production scheduling (`execute_persisted_from_factory` → bounded
  `run_job` runner → `ProductionExecutionFactory`), Storage claim/attempt/state
  methods, Sidecar `staging_dir`, and `ArchiveService::complete_sidecar_archive`.
  `Database::claim_archive_job` exists but has only a sequential Storage test and
  no production caller. It conditionally transitions state but does not increment
  the attempt. Production still increments attempt separately, writes lifecycle
  states, then sends to a capacity-one runner; queue-full errors reach generic
  failure persistence and may mark accepted Jobs `FAILED`.
- Attempt staging and no-replace helper APIs exist, but production Sidecar and
  archive metadata/commit paths still use Job-level staging. Archive completion
  has no attempt identity parameter. A current-attempt check before the call is
  not a publication fence: the attempt can change after the check while the old
  Sidecar/aria2 writer remains alive. No safe partial wiring was applied.
- C1-A is `IN_PROGRESS` as a shared implementation plan; the initial analysis
  batch changed no production code. The active Plan now sequences admission/
  backpressure before durable claim; atomic state+attempt claim; runner-owned
  immutable attempt session and coordinator guard; attempt staging for both
  download engines; confirmed process-tree exit; and attempt-authorized destination
  publication/finalization. Contention and capacity rejection must remain
  non-failing and reschedulable. Required behavior tests: one winner for competing
  claims, parallel distinct Jobs, serialized same destination, stale attempts
  denied publication and cleanup of newer files, cancellation waits for writer
  exit before releasing the guard, and saturation does not fail accepted Jobs.
- Linux/WSL2 x86_64 capability was detected; Unix coordination adapter suite
  `cargo test -p xarchive-workflow --test coordination -- --test-threads=1`
  passed 11/11. This proves adapter behavior only, not production lifecycle
  coordination. Windows implementation, Windows tests and NTFS acceptance remain
  `NOT_RUN`; v2 recovery and D1 remain disabled. Full regression was not run.
- At that checkpoint, Plan §5 recorded the ordered B0–B7 C1 batches. Its status
  wording has since been superseded: B0 is not complete until current shared WIP
  compiles and focused claim tests, diff check, docs audit, and final diff review
  are recorded.

## Documentation synchronization and N0 baseline — 2026-10-11 (historical)

- At this checkpoint the source HEAD was
  `bf050ccc5559d1b791bfe335413373fd4ad8b078` and the worktree was uncommitted.
  This checkpoint is superseded by the current GitHub progress sync below; it was
  not a formal Windows handoff.
- Documentation synchronization updated Plan §2/§5/continuation, the architecture
  transaction/workflow description, and this status record to distinguish the
  legacy `claim_archive_job` from the WIP `claim_archive_attempt`. The latter
  atomically advances attempt/state and records an event, but does not establish
  production admission or full lifecycle ownership. E3 current status is retained;
  older statements that E persistence/argv wiring were absent are historical.
- Source inspection still finds `JobPersistence` lacking the attempt methods that
  appear in its implementations, and `executor.rs` requires repair of method
  placement. Claim fixtures also require a current-source check. Therefore N0 is
  `IN_PROGRESS`, not `COMPLETED` or `VERIFIED`; no business-code implementation or
  tests were run in this documentation phase.
- Documentation gate: Plan and architecture now describe the observed WIP and
  keep v2 fail-closed; Windows queue remains a separate owner/evidence surface.
  The documentation diff was reviewed, `git diff --check` passed, and
  `node scripts/docs-audit.mjs` passed before business-code modification began.

## N0 baseline repair — 2026-10-11 (current; shared WIP)

- Repaired `JobPersistence` declarations and implementations for in-memory,
  storage-backed, and commit-boundary test adapters; removed nested function
  declarations from `StorageJobPersistence::attempt_is_current`.
- Corrected the PREPARED claim fixture to match the journal attempt identity.
- Verified on the current uncommitted tree: `cargo fmt --all -- --check`,
  `cargo check -p xarchive-desktop --lib`, two focused Storage claim tests,
  Desktop stale-attempt fencing regression, `git diff --check`, and
  `node scripts/docs-audit.mjs` all pass.
- N0 is verified only for this bounded compile/fixture slice. N1 admission and
  rescheduling plus C1 production ownership, staging, writer exit, publication,
  replay and finalize remain open. v2 remains fail-closed; D1 remains gated.
- Branch and source base remain `cross-platform/automatic-pairing-reconcile-20261002`
  and `bf050ccc5559d1b791bfe335413373fd4ad8b078`; worktree remains uncommitted.
  This is not a formal Windows handoff. Windows implementation and acceptance
  remain assigned to the Windows Platform Owner and are not claimed as run.

## N1 admission and backpressure — 2026-10-11 (committed shared change)

- Fixed the confirmed `CROSS_PLATFORM_CHANGE_REQUIRED` scheduling defect
  (`CROSS_PLATFORM_CHANGE_REQUIRED` in `platform-handoff-history.md`): a
  saturated single-active runner previously marked an accepted Job `FAILED` with
  `EXECUTOR_WORKER_FAILED: queue is full` after it had already been advanced to
  `DOWNLOADING`.
- `JobExecutorHandle` now carries a `runner_in_flight` counter mirroring the
  runner channel capacity, with `runner_try_admit`/`runner_release` and a
  `RunnerReservation` RAII guard. Both `execute_persisted_from_factory` and
  `execute_persisted_on_worker` reserve a runner slot before `begin_attempt`, so
  a `QueueFull` rejection leaves the Job queued/reschedulable, increments no
  attempt, and writes no `FAILED`. The reservation is released right after the
  runner interaction so download and finalize still overlap.
- Regression `saturated_runner_rejection_is_non_failing_and_reschedulable`
  proves: rejection is non-destructive (state stays QUEUED, attempt stays 0) and
  the Job is admitted and runs once the blocker releases the slot.
- Verification before commit: `cargo fmt --all -- --check`
  PASS; `cargo test -p xarchive-desktop --lib` 286/286 PASS (includes the new
  test); focused Storage claim tests 2/2 PASS; `git diff --check` PASS.
- At the time of this entry, this closed the B1 runner-admission core only. B2 typed claim-loss/contention
  outcomes, B3 runner-owned session, and B4–B7 attempt staging/writer fencing,
  fenced publication/finalize, shared replay and the crash matrix remain open.
  Production v2 stays fail-closed; D1 stays gated. Windows implementation and
  NTFS/native-process acceptance remain `NOT_RUN`/`BLOCKED` and are not claimed.

## N2 atomic state-plus-attempt claim — 2026-10-11 (committed shared change)

- Rewired `execute_persisted_from_factory` and `execute_persisted_on_worker` so
  the initial running transition is acquired through the atomic
  `JobPersistence::claim_attempt` (→ `Database::claim_archive_attempt`) rather
  than a separate `begin_attempt` plus an unfenced first state write. The claim
  binds expected state+attempt, increments the attempt with checked arithmetic,
  and writes the first running state plus its event in one transaction; a losing
  claimant returns the winner's snapshot with no side effects. A Job already in
  its running state (resume) still uses `begin_attempt` for an attempt identity.
- Behavior-preserving: final state/event sequence unchanged, confirmed by the
  unchanged 286-test Desktop suite. PREPARED/COMMITTED routing to recovery stays
  fail-closed.
- Verification before commit: `cargo fmt --all -- --check` clean;
  `cargo check -p xarchive-desktop --lib` PASS; `cargo test -p xarchive-desktop
  --lib` 286/286 PASS; `git diff --check` PASS.
- At the time of this entry, this closed the B2 production claim wiring. B3 runner-owned session, B4 attempt
  staging/writer fencing, B5 fenced publication/finalize, B6 shared replay and
  B7 Desktop recovery integration remain open. Windows items remain
  `NOT_RUN`/`BLOCKED` and are not claimed.

## Plan non-Windows continuation — 2026-10-11 (historical checkpoint; superseded below)

- Follow-up implementation result: audited the production dispatch entry points.
  `submit_and_schedule_persisted` still marks an invalid persistence path as
  `EXECUTOR_SCHEDULE_FAILED`/FAILED because detached orchestration cannot retry
  it, while bounded `QueueFull` is handled earlier by `submit_persisted` and
  remains non-failing. Regression tests for invalid-database scheduling and
  background execution failure pass. This does not establish saturated-runner
  rescheduling or close B1; atomic claim and the B2–B7 production lifecycle remain
  open.
- Historical source HEAD before this documentation update:
  `bf050ccc5559d1b791bfe335413373fd4ad8b078`. Current implementation commits and
  handoff status are recorded in the GitHub progress sync section below.
- Capability observed: Linux x86_64 under WSL2; Rust/Cargo 1.98.0, Python 3.14.4,
  Node 26.7.0. No native Windows/NTFS artifact or Windows GUI evidence is bound
  to this tree.
- Existing current-tree results recorded before this continuation: Workflow
  coordination 11/11, Storage 117/117, Desktop 285/285 and Sidecar downloader/
  protocol 30/30 PASS. They establish Linux module/adapter behavior only and
  were not all rerun in this continuation.
- No C1.4 production gate was closed. The v2 archive service and startup
  fail-closed guards remain in force; D1 output switches remain non-operative.
  Next batch C1-A is production coordination and attempt/writer fencing; later C1
  batches cover PREPARED replay/no-replace commit, atomic finalization, COMMITTED
  verification and process-restart matrix. D1, F1–F3 and integrated G are also
  shared work; none is reclassified as a Windows blocker. F3 implementation is
  planned against the confirmed contract in Plan §11 (separate records, optional
  ZIP/7z import, preview, idempotent merge and conflict rejection); no format or
  behavior beyond that scope is implied.
- Windows execution is `NOT_RUN` because this host is WSL2/Linux and no exact
  formal handoff revision or fresh Windows artifact exists. Windows Owner must
  use the consolidated manual checklist in `docs/validation/windows-queue.md`
  only after shared implementation is committed and formally handed off.
- Full workspace regression was not run; targeted Storage/Desktop/Workflow and
  Sidecar suites were selected for the inspected recovery/coordination and
  downloader-acceptance scope. This is not a final G integration result.
- Revalidated in this historical continuation:
  `cargo test -p xarchive-download --lib -- --test-threads=1` 50/50 PASS;
  Sidecar downloader/protocol pytest 30/30 PASS; `cargo fmt --all -- --check`,
  `git diff --check`, and `node scripts/docs-audit.mjs` PASS. An initial
  parallel Cargo invocation produced complete 50/50 output but terminal
  integration returned a nonzero wrapper status; the deterministic serial run
  above is the product result. No F1/F2 product code was retained in this
  continuation; the incomplete exploratory prototype was removed before closeout.
- F1/F2 remain `PLANNED`; F3's contract is confirmed but implementation remains
  `PLANNED`. The
  Plan progress table now distinguishes these streams and marks integrated G
  `IN_PROGRESS`; no claim is made that all non-Windows Plan work is complete.

## Plan E3 shared acceptance continuation — 2026-10-11 (Linux verified; not Windows handoff)

- Current owner: Cross-platform Owner. Branch:
  `cross-platform/automatic-pairing-reconcile-20261002`; E3 implementation commit
  `d846ef9`. Its initial source base was
  `bf050ccc5559d1b791bfe335413373fd4ad8b078`. This is not a formal Windows
  handoff.
- E3 Linux actual-spawn evidence:
  - `cargo test -p xarchive-download --lib`: 50/50 PASS. A readiness-capable
    local JSON-RPC server and fake executable exercise `Aria2Supervisor::spawn`.
    Captured child argv proves application options precede the task pair,
    `--conf-path` is last, and the RPC secret is absent from argv and the path;
    the secret config is mode 0600 and is removed on shutdown.
  - From `/home/shiraishi/VSCode Workspace/Tw2Tg/sidecar`,
    `/home/shiraishi/VSCode Workspace/Tw2Tg/.venv/bin/python -m pytest
    tests/test_downloader_args.py tests/test_protocol_v2.py -q`: 30/30 PASS.
    The added test exercises `DownloadRunner.run` through the actual
    `_run_process`/`subprocess.Popen` route, captures argv boundaries including
    Unicode, spaces and shell-like characters, and checks application staging
    options follow task arguments.
  - Formatting, diff check and documentation audit passed in this batch.
- The first full Download crate run exposed a race with the existing test that
  snapshots the process-wide temp directory. The new test now takes that test
  module's shared lock; a subsequent clean serial run passed.
  `cargo test -p xarchive-desktop --lib` also passed 285/285 and the focused
  `cargo test -p xarchive-desktop --lib e3_ -- --nocapture` passed 4/4 on this
  source.
- gallery-dl user arguments remain rejected by the empty reviewed allowlist.
  The runner fixture proves argv transport only, not an enabled production option.
- This closes only the previously missing E3 Linux spawn gates. The overall
  non-Windows Plan remains open: C1.4 production recovery/restart matrix, D1
  (gated on C1), F1–F3, and integrated G verification are unfinished. See the
  Plan for the complete shared backlog.
- Windows checks remain `NOT_RUN` / `BLOCKED`: this host is WSL2/Linux and has no
  bound formal Windows working tree, fresh artifact or NTFS target. Windows
  Owner execution must follow formal Git handoff and the manual steps in
  `docs/validation/windows-queue.md`.

## Plan E3 shared implementation continuation — 2026-10-10 (current; WIP, not Windows handoff)

- Current owner: Cross-platform Owner. Branch:
  `cross-platform/automatic-pairing-reconcile-20261002`; base HEAD
  `083b3974bebf6475ba97c289c5848a89c5a9bf05`, with uncommitted E3 acceptance
  tests and documentation. This remains shared WIP, not a Windows handoff.
- E2 has both-engine argument plumbing: decoded snapshot arrays reach the
  gallery-dl download command and aria2 supervisor argv; trusted Sidecar
  `executable_args` remain separate. E3 now writes v3 task/batch snapshots, but
  gallery-dl user arguments remain disabled because its reviewed allowlist is
  empty.
- E3 implementation has advanced: Desktop JSON-array settings UI and
  validated persistence exist; one-off and batch submissions write v3 snapshots;
  v3 validation checks policy/engine arguments/tool identity; batch children
  inherit the persisted batch snapshot; production checks selected executable
  compatibility before downloader execution. Browser transport now receives
  the selected executable identity from Desktop runtime, using the same default
  gallery-dl path as Sidecar worker args. `Aria2SupervisorConfig::Debug` reports
  only argument count. No execution-spec export API was found.
- Recommended contract: Desktop Download Config is the settings authority for
  one-off, Extension-originated and batch jobs. Browser requests remain metadata
  only. Rust validates settings and captures immutable task/batch snapshots;
  retries, resume and active-job reuse never resample global settings. Changes to
  shared snapshot/schema or execution semantics remain
  `CROSS_PLATFORM_CHANGE_REQUIRED`.
- This continuation wires Browser transport to the selected Desktop tool identity,
  shares the default gallery-dl executable resolution with Sidecar worker args,
  avoids probing tool identity when the selected argument array is empty, and
  fixes WebSocket worker reclamation so completed request threads are removed.
- Earlier targeted/shared validation on the base source: Desktop library 284/284;
  WebSocket 13/13; Download 48/48; Storage 117/117; Sidecar 67/67; Desktop Node
  220/220; all-targets check, fmt, diff-check and docs audit PASS. This continuation
  adds argv redaction of task-snapshot values from direct-download diagnostics.
  The initial probe-marker test expectation was corrected. Linux verification
  on this WIP: Download supervisor 10/10; Desktop E3 targeted
  tests 4/4; Sidecar argument/protocol tests 29/29; `cargo fmt --all -- --check`
  and `git diff --check` PASS. The Sidecar diagnostic test exposed and now covers
  task-argv echo redaction. These checks do not establish real aria2 supervisor
  spawn argv ordering or full DownloadRunner spawn acceptance: the attempted
  aria2 fixture reconstructed argv rather than exercising production spawn and
  was removed. E3 shared acceptance remains open; full workspace integration was
  not run.
- Windows-native checks remain `NOT_RUN`. A formal Windows handoff requires the
  exact pushed source revision after shared acceptance gates pass.

### 2026-10-14 test round — runner + CLI avalanche evidence

- **Runner 测试组** (`crates/xarchive-workflow`, `--locked`):
  - `--lib`: 35 passed, 0 failed.
  - `execution_session`: 5 passed.
  - `coordination`: 11 passed.
  - `linux_containment`: 1 passed + 1 ignored (delegated cgroup skipped; not a failure).
  - `no_replace`: 2 passed.
  - Subtotal: 54 passed, 0 failed, 1 ignored.
- **CLI 雪崩测试组** (CLI / multi-process / recovery / transfer / protocol support):
  - `xarchive-download`: lib 52 passed + 0 failed; `transfer_driver` 7 passed; `linux_contained` 3 ignored (cgroup delegation).
  - `xarchive-native-host`: lib 10 passed + `main` 1 passed + `bootstrap_process` 1 passed.
  - `xarchive-storage --lib`: 142 passed, 0 failed.
  - `xarchive-core --lib`: 44 passed, 0 failed.
  - `xarchive-protocol --lib`: 31 passed, 0 failed.
  - Subtotal: 341 passed, 0 failed, 4 ignored.
- **Workspace measurement**: 395 passed, 0 failed, 5 ignored; no blocking test.
- **`cargo fmt --all -- --check`**: PASS.
- **`git diff --check`**: PASS (clean tree; HEAD is `ea02b6b`).
- **`node scripts/docs-audit.mjs`**: PASS.
- **Session-boundary order fix (恢复/攀升顺序) confirmed in HEAD**: `ExecutionReservation::claim` drops the guard and returns `ClaimLost`/`ClaimRejected` before binding a session; only a winning claim binds the session, and the session retains the guard through drain. `execute_archive_context` shuts down the Sidecar after a successful download and again on a download error, surfacing `ARCHIVE_WRITER_DRAIN_FAILED`; no production call site was switched.
- **`Istanbul` 提交内容**: not present in this tree/branch — no `Istanbul` marker and no submission patch. The prior two fixes (`047568e` drain-Sidecar-before-archive-completion; `32f932c` own-runner-admission-tokens) are already in HEAD and exercised by the tests above.
- **Next**: P1 runner-owned seam (see `desktop-download-output-plan.md` `### Remaining implementation and acceptance gates`).
### E3 remaining acceptance sequence

1. Still required: implement a controlled readiness-capable fake aria2 server
   and test `Aria2Supervisor::spawn` end-to-end (including secret-file path last
   and RPC secret absent); exercise gallery-dl through DownloadRunner's actual
   spawn path; then run affected suites. Current Linux targeted results do not
   close these gates.
2. Retain the empty gallery-dl user-argument allowlist until a separate reviewed
   policy change; do not infer gallery-dl option support from argv plumbing tests.
3. After successful Linux checks, commit and formally hand off the exact Git
   revision for Windows-owned Named Pipe and packaged-artifact checks.

Current WIP is not a formal handoff. Windows E checks are `BLOCKED` in this
session (`WINDOWS_ENVIRONMENT_UNAVAILABLE`); manual steps are in
`docs/validation/windows-queue.md`. No Windows result is claimed.

---

## Plan C1 non-Windows continuation — 2026-10-10 (current; WIP, not Windows handoff)

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; source HEAD before this
batch `af71c69793301370d92a9ac62e0c4a1420ac47c0`. This batch is a WIP continuation
closing the remaining P2 lifecycle gates (explicit attempt overflow failure,
PREPARED journals blocking new attempts, atomic Job claim); it is not a Windows
handoff. The batch includes
shared changes in `crates/xarchive-workflow/src/coordination/unix.rs`,
`crates/xarchive-workflow/src/coordination.rs` and
`crates/xarchive-workflow/tests/coordination.rs` (process-shared local lock
- Resulting commit: `af71c69793301370d92a9ac62e0c4a1420ac47c0` (current HEAD; WIP,
  shared batch, not a Windows handoff).
- Surface state: branch `cross-platform/automatic-pairing-reconcile-20261002`; HEAD `af71c69793301370d92a9ac62e0c4a1420ac47c0`; working tree clean after this sync; remote head unchanged. This is a GitHub progress sync only, not a Windows handoff.

### Batch contents

- `crates/xarchive-workflow/src/coordination/unix.rs` and `crates/xarchive-workflow/src/coordination.rs` (process-shared local lock registry so same-process coordinator instances contend; `flock` adapter; containment and fork-residual fallbacks; `flock` error classification: only kernel `EAGAIN` is `Contended`; permission/I/O errors map to I/O).
- `crates/xarchive-workflow/tests/coordination.rs` and `crates/xarchive-workflow/src/coordination/unix.rs` (acceptance tests, including lock lifecycle, partial and failed multi-lock cleanup, fork-residual repulsion and destination exclusivity).
- `crates/xarchive-storage/src/file_store.rs` and `crates/xarchive-workflow/src/...` (attempt-scoped file staging, replay tied to immutable `RenameEdge` and `ArchiveRenamePlan` facts, regular-file/reparse checks, size/SHA-256 verification for pending and already-completed rename edges, and propagation of permission/I/O errors rather than treating them as missing paths).
- `crates/xarchive-storage/src/archive_recovery.rs`, `crates/xarchive-storage/src/database/jobs.rs` and `crates/xarchive-storage/src/lib.rs` (cancellation/PREPARED arbitration and atomic terminal-state attempt fencing).
- `desktop/src-tauri/src/executor.rs` (`JobPersistence` cancellation ports and `cancel_persisted` arbitration).
- `crates/xarchive-download/src/supervisor.rs`, `desktop/src-tauri/src/archive.rs`, `desktop/src-tauri/src/production.rs` and `desktop/src-tauri/src/executor.rs` (Plan E2 both-engine wiring: the decoded `downloader_arguments` snapshot flows from `ProductionExecutionFactory::create` into `ArchiveExecutionContext.with_downloader_arguments` and survives every error-recovery context rebuild; the gallery-dl path forwards `snapshot.gallery_dl` as the `SidecarV2Command::download` `user_args`; the aria2 path threads `snapshot.aria2` through `transfer_with_one_refresh` → `transfer_once` → `aria2_config` → `Aria2SupervisorConfig::with_user_args`, appended after the application-owned options in `command_args()`; the internal worker `executable_args` stays separate from user args).
- `docs/development/desktop-download-output-plan.md`, `docs/status/platform-handoff.md`, `docs/validation/windows-queue.md` and this file (state and handoff documentation).
registry so same-process coordinator instances contend),
`crates/xarchive-storage/src/file_store.rs` (`create_attempt_file` NOFOLLOW
creation tests), `crates/xarchive-storage/src/archive_recovery.rs` plus
`crates/xarchive-storage/src/database/jobs.rs` and `crates/xarchive-storage/src/lib.rs`
(cancellation/PREPARED arbitration and atomic terminal-state attempt fencing),
`desktop/src-tauri/src/executor.rs` (`JobPersistence` cancellation ports and
`cancel_persisted` arbitration), plus architecture, plan, and Windows queue/manual
validation documentation. No Windows result is claimed.

- Confirmed non-windows order (Documentation Gate first, then implementation):
  Documentation Synchronization → coordination acceptance (Plan P1) →
  attempt/writer lifecycle and cancellation/PREPARED arbitration (P2/P3) →
  prepare/finalize contract and unified commit/replay workflow (P4/P5) → service
  crash/restart matrix (P6) → Desktop production wiring (P7) → final status and
  handoff update. `PLANNED`/`IN_PROGRESS`/`COMPLETED`/`VERIFIED` stay distinct;
  D0 → C1 → D1 ordering is preserved and D1 stays gated. P2 is `IN_PROGRESS`:
  its cancellation/PREPARED linearization boundary is implemented and
  targeted-tested; `begin_archive_attempt` now rejects COMPLETE/CANCELLED inside
  its transaction and its regression test verifies `attempt_count` is unchanged.
  Conditional attempt-fenced state writes are wired into both production attempt
  paths (`execute_persisted_on_worker` and `execute_persisted_from_factory`)
  through the new `JobPersistence::persist_state_for_attempt` port, backed by the
  existing atomic `Database::persist_state_if_attempt`, with a stale-attempt
  regression test. Full attempt-ownership/file-writer fencing remains open.
- Current P1 adapter result: `cargo test -p xarchive-workflow --test
  coordination` passed 11/11 on 2026-10-11. The tests cover guard drop and
  explicit release, partial-group rollback and fork/exec descriptor residue in
  addition to contention/crash/destination cases. This closes that adapter-test
  gap only; it does not close production Job/destination lifecycle wiring.
- Production v2 stays fail-closed until that sequence passes:
  `recover_internal_rename_v2` still returns the fail-closed error, the service
  failpoint parameter is unused, Desktop startup still returns
  `ARCHIVE_RECOVERY_V2_NOT_READY`, and no production lifecycle holds the
  coordinator guard across the download path.
- Verification results observed for this continuation are recorded in the final
  batch section below; counts in superseded sections are historical checkpoints,
  not current acceptance.
- Windows native coordinator, NTFS no-replace, packaging, GUI and artifact-bound
  checks remain `NOT_RUN`/`BLOCKED` (Windows Owner), tracked in
  `docs/validation/windows-queue.md`; they do not block this shared batch.

## Cancellation/PREPARED arbitration and terminal attempt fencing — 2026-10-10 (WIP progress sync)

This section records the batch prepared for GitHub progress sync. It is not a
formal Windows handoff or a Windows result.
- Batch verification: `cargo fmt --all -- --check` PASS; `git diff --check` PASS; docs audit PASS; `cargo test -p xarchive-storage --lib` 114/114 PASS; `cargo test -p xarchive-desktop --lib` 271/271 PASS; `cargo test -p xarchive-workflow --test coordination` 11/11 PASS. Targeted Clippy on `xarchive-workflow --all-targets`, `xarchive-storage --lib` and `xarchive-desktop --lib` produced no warning pointing at files changed in this batch. `xarchive-storage` still reports two pre-existing `too_many_arguments` lints in `database/jobs.rs:183`/`:204`.

### Production boundary — still open

Normal v2 prepare/commit production flow, service-level restart/replay and crash/failpoint matrix, durable cancellation fencing, and Desktop lifecycle / production coordinator wiring remain incomplete. Production v2 stays fail-closed: `recover_internal_rename_v2` returns the fail-closed error, the service failpoint parameter is unused, Desktop startup still returns `ARCHIVE_RECOVERY_V2_NOT_READY`, and no production lifecycle holds the coordinator guard across the download path. D1 remains gated.

## Conditional attempt-fenced state writes — 2026-10-10 (WIP progress sync)

WIP shared batch; not a Windows handoff and not a Windows result. The batch adds `JobPersistence::persist_state_for_attempt(job_id, attempt, expected, next)` and routes the lifecycle state transitions of both production attempt paths (`execute_persisted_on_worker`, `execute_persisted_from_factory`) plus the terminal `DOWNLOADING → DOWNLOADED` write through it. A losing attempt stops writing states/events and returns the winner's snapshot. `StorageJobPersistence` implements the port over the pre-existing atomic `Database::persist_state_if_attempt`; the trait default always writes and serves only in-memory/stub persistence.

- Batch verification: `cargo fmt --all -- --check` PASS; `git diff --check` PASS; `cargo test -p xarchive-desktop --lib` 272/272 PASS (includes the new `stale_attempt_cannot_overwrite_a_state_written_by_a_newer_attempt` regression test). Targeted Clippy on `xarchive-desktop --lib --all-targets` reports no warning on `executor.rs`; the remaining `needless_question_mark` occurrences introduced during wiring were removed. Pre-existing lints elsewhere (`too_many_arguments` in `xarchive-storage/src/database/jobs.rs`, `useless_conversion` in `websocket_transport.rs`) are unchanged and were not touched.
- Still open: atomic Job claim in the production submit/schedule path, attempt-ownership file-writer fencing (P3), PREPARED replay, read-only COMMITTED verification, service failpoint/restart matrix, and Desktop lifecycle wiring. D1 remains gated.

### Windows status — blocked / NOT_RUN

Windows native coordinator, NTFS no-replace, packaging, GUI and artifact-bound checks remain `NOT_RUN`/`BLOCKED` (Windows Owner), tracked in `docs/validation/windows-queue.md`; they do not block this shared batch. Full manual verification steps for this batch are recorded in `docs/validation/windows-manual-steps.md`.

### Windows manual verification steps — 2026-10-10 (not executed here)

The following Windows manual steps are recorded; none is claimed as executed or PASS:

**W1. Advisory lock compatibility.** Confirm the shared advisory-locking primitive used on Windows is equivalent to the C1 Linux `flock`-based adapter: same reservation, same contention classification, same granted state, same release and same failure classification. Do not reuse the Linux `flock` file-description semantics as an equivalence claim on NTFS.

**W2. Path identity and reparse guards.** Provide a Windows `LockName` adapter with the same rejection rules as Linux: parent-directory escapes, absolute paths outside the staging root, path-length limits and null bytes. Confirm reparse points are rejected consistently with the Linux guard and that no lock path is overwritten by an existing reparse point.

**W3. Lock-file lifecycle and recovery.** Verify that a stale lock file can only be removed by an explicit independent cleanup action and that a new owner cannot open a reset lock file while an old owner still holds the logical reservation. Remove any path where a stale lock file can be silently reset by a second process.

**W4. No-replace move semantics.** Confirm the Windows native no-replace move primitive for staging-to-final and directory commits: destination-exists must be refused without overwrite, missing source must fail clearly, and unsupported platforms must fail closed. The Linux `renameat2(RENAME_NOREPLACE)` adapter result is evidence only for the Linux adapter, not for NTFS.

**W5. Production v2 gating.** Confirm the C1 production gate applies on Windows: `recover_internal_rename_v2` remains fail-closed, `complete_sidecar_archive_with_failpoint` does not change the outcome, and Desktop startup dispatch does not route `InternalRenameV2`. The Windows batch must not advance a job through v2 until the C1 non-Windows production gate is closed.

**W6. Fresh artifact provenance.** All Windows verification in this batch must use a fresh artifact whose provenance is recorded in the artifact manifest, including tool versions and dependency lock contents. Do not reuse results from the Linux build or a non-fresh checkout.

**W7. Repository and documentation cleanup before handoff.** Before the Windows native work and artifact-bound handoff, close the current state: commit or revert the tracks and docs, record source commit and uncommitted state in handoff/queue, and remove this batch from the active window until artifact provenance and the C1 production gate are closed.

**W8. Build and package parity.** Rebuild the Windows package from the same source revision, confirm identical dependency versions, confirm the build did not silently switch to an older dependency source, and record the build hash. If the manifest or package hash differs from the artifact record above, record the difference as a defect.

**W8. Coordination-adapter behavioral parity.** Use the Windows adapter in a controlled isolated directory. Create one `Job`, start one owner, then start a second owner and confirm the second owner is refused with the same contention classification and that the first owner still holds the reservation.

**W8. Restart and cancellation parity.** Allocate a fresh workspace, submit a job, interrupt it at a defined point, and confirm the Windows adapter releases the lock on actual process exit and does not leave a stale ownership state that blocks the next owner.

**W8. NTFS no-overwrite parity.** In a clean directory, stage two files, confirm a second attempt to move the same staging source to the same final destination is refused, then repeat with a different destination to confirm a legal move succeeds.

**W8. GitHub handoff.** Record branch, source commit, handoff commit, uncommitted-state status and current owner in handoff state. Do not claim ownership changes or handoff completion until artifact-bound evidence is produced.

**W9. Plan C1 non-Windows status and Windows handoff summary.** Record this batch as a GitHub progress sync; do not record it as a Windows handoff or as a finished C1. Windows items remain NOT_RUN/BLOCKED until manual artifacts are verified. No Windows PASS is claimed in this Linux session.

- Shared implementation: `ArchiveService::archive_commit_started` reports an
  advisory commit boundary (any v1 or v2 journal row);
  `ArchiveService::decide_archive_cancellation` re-checks both journal
  generations inside one `unchecked_transaction`, returns `CommitWins` with no
  write when a PREPARED/COMMITTED journal exists, `AlreadyDecided` for a
  terminal Job, and otherwise validates the transition and writes the cancelled
  state with its `JOB_STATE_CHANGED` event. `ArchiveCancellationDecision` is
  exported from `xarchive-storage`.
- Desktop: `JobPersistence` gained `archive_commit_started` and
  `persist_cancellation` with fail-open defaults, the `CancelArbitration`
  outcome type, a `Database`-backed override delegating to Storage, and
  `cancel_persisted` now refuses before stopping the writer when the boundary is
  already durable and writes no cancelled state or event when the concurrent
  prepare won.
- `Database::begin_archive_attempt` reads Job state and attempt count in the same
  transaction, rejects terminal states using `JobState::is_terminal`, and leaves
  the attempt count untouched. Regression coverage checks both COMPLETE and
  CANCELLED; FAILED/INTERRUPTED remain retryable by this fence.
- Targeted verification observed on this working tree: Storage library tests
  114/114 PASS (including `beginning_archive_attempt_rejects_terminal_jobs_without_incrementing_attempt` and `cancellation_arbitrates_against_the_archive_commit_boundary`),
  Desktop library tests 271/271 PASS (4 cancellation-arbitration tests),
  Workflow coordination integration tests 11/11 PASS,
  `cargo fmt --all -- --check`, `git diff --check` and
  `node scripts/docs-audit.mjs` PASS. Targeted Clippy on
  `xarchive-workflow --all-targets`, `xarchive-storage --lib` and
  `xarchive-desktop --lib` produced no warning pointing at any file changed in
  this batch; `xarchive-storage` still reports only the two pre-existing
  `too_many_arguments` lints in `database/jobs.rs:183`/`:204`.
- Not done: full Rust workspace regression, Sidecar pytest, Desktop/Extension
  Node suites, Windows native/NTFS/GUI/artifact validation, and the remaining
  P2/P3/P6 gates. Production v2 remains fail-closed and D1 stays gated.

## Previous GitHub progress sync — 2026-10-10 (superseded; WIP, not Windows handoff)

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; base/source commit
`346e538fce32a630b120bb4570159f358885cea0`. This checkpoint records the current
working-tree batch for GitHub visibility; it does not claim the batch is complete
or ready for Windows. Windows implementation and validation were not performed.

- Shared changes in this WIP include Unix `flock` coordination and contention /
  release / rollback tests, a Linux `RENAME_NOREPLACE` mover, attempt-scoped file
  staging and replay helpers, SQLite v2 archive finalization/journal validation,
  workflow bridge scaffolding, and Desktop integration scaffolding. Journal reads
  validate the attempt-bound metadata candidate; tampered, missing, and mismatched
  candidates are rejected.
- Targeted verification observed on this worktree: `cargo fmt --all -- --check`,
  `git diff --check`, `cargo check -p xarchive-storage --lib`, Workflow
  coordination integration tests (10/10), no-replace integration tests (2/2),
  Storage library tests (111/111), and `node scripts/docs-audit.mjs` PASS. One
  combined validation invocation did not return a reliable terminal completion
  status; no result beyond the individually observed commands is claimed.
- Normal v2 prepare/commit production flow, service-level restart/replay and
  crash/failpoint matrix, durable cancellation fencing, and Desktop lifecycle /
  production coordinator wiring remain incomplete. Preserve the existing
  fail-closed v2 recovery guards and keep D1 gated until the required integrated
  evidence passes.
- Windows native coordinator, NTFS no-replace, packaging, GUI and artifact-bound
  checks are `NOT_RUN`; no Windows PASS is claimed. They remain downstream
  Windows-owner work after shared implementation reaches its acceptance gate.
- Working tree was dirty before this sync and included shared Rust code, a Storage
  migration, workflow bridge/tests, and Plan/architecture/validation documentation;
  these pre-existing changes are being preserved together in a clearly labeled
  WIP progress commit, not represented as an accepted feature batch.

## Plan C1 non-Windows continuation — superseded current-tree snapshot

Historical snapshot only; the active current-tree status is recorded in the
`Documentation Synchronization and implementation start — 2026-10-09` section
below. Current owner was Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; base HEAD
`346e538` (`feat(workflow): add Unix fcntl coordination adapter and cross-process tests`).
Working tree is uncommitted; no handoff commit, remote push, Windows working tree,
NTFS target, or fresh artifact is available. This is not a formal Windows handoff.

- Added Storage attempt-scoped staging, replay tied to immutable `RenameEdge` and
  `ArchiveRenamePlan` facts, regular-file/reparse checks, size/SHA-256 verification
  for pending and already-completed rename edges, and propagation of permission/I/O
  errors rather than treating them as missing paths. Linux uses atomic no-replace;
  non-Unix stays fail-closed pending Windows-owner adapter implementation.
- Tightened Unix lock error mapping so only the kernel's `EAGAIN` is reported as
  contention; permission denial and other OS errors map to I/O failure. A unit
  test covers `EAGAIN`, permission denial and generic I/O classification. Existing
  tests prove real cross-process contention, crash release, parallel jobs and
  destination exclusivity.
- Targeted verification: Storage 108/108 PASS; Workflow unit tests 5/5, real
  cross-process tests 4/4 and no-replace tests 2/2 PASS. `cargo fmt --all`,
  `git diff --check` and `node scripts/docs-audit.mjs` PASS. Strict Workflow
  Clippy passes. Storage strict Clippy remains blocked by two existing
  `too_many_arguments` lints in `database/jobs.rs:183` and `:204`. The first
  compile attempt exposed incorrect accessor assumptions; a later permission test
  also initially violated the workspace unsafe-code prohibition. Both were
  corrected/removed; final targeted tests pass. Full workspace tests and Windows
  target compilation/validation remain NOT_RUN.
- This is still helper-level work only. Attempt staging and replay are not wired
  through `archive.rs`, `executor.rs`, or startup recovery; v2 journal transaction,
  durable fencing, atomic Tweet/media/Job/journal/Telegram-intent finalization,
  read-only COMMITTED verification and service crash/restart matrix remain
  incomplete. Both existing v2 fail-closed guards remain unchanged; do not enable
  D1. C1.2–C1.4 are not accepted, and this shared implementation gap is not a
  Windows-only blocker.
- Windows: `NOT_RUN` / `BLOCKED` for lack of NTFS/native Windows environment and
  fresh artifact. Manual steps are consolidated in
  `docs/validation/windows-manual-steps.md` item 3 and the authoritative queue.
  The queue item cannot close until the shared C1.4 gate passes and a formal Git
  handoff provides the exact source SHA/artifact provenance.

## Plan C1 continuation — 2026-10-09 (transactional DB primitive; historical)

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; base HEAD
`346e538fce32a630b120bb4570159f358885cea0`. Working tree remains uncommitted;
there is no handoff commit. This update supplements the entry above and does not
claim C1 completion.

- Added `Database::finalize_archive_recovery_v2`, which validates the immutable
  manifest/journal and updates Tweet metadata/path, media rows, Job state/events,
  Telegram intent state, and journal COMMITTED in one SQLite transaction with
  attempt checks. Success and stale-attempt/rollback unit tests pass. The unsafe
  standalone v2 phase-to-COMMITTED API was removed so callers cannot bypass the
  finalization transaction.
- Tightened v2 manifest validation to reject media index 0 because migration
  0001 requires `media_index > 0`. This does not by itself prove a production
  Sidecar mismatch: the inspected extraction/model conversions enumerate media
  from 1, and core also rejects zero. Audit all protocol and persistence
  boundaries and keep the positive-index contract explicit; change a boundary
  only if a zero-based producer is demonstrated.
- Linux verification on the current working tree: Storage lib 109/109 PASS;
  `cargo fmt --all` and `cargo fmt --all -- --check` PASS; `git diff --check` and
  `node scripts/docs-audit.mjs` PASS. A first test exposed the zero-index schema
  constraint and was corrected in its fixture; no existing assertion was relaxed.
- Still incomplete: service/file replay and startup wiring; production-safe
  same-process plus cross-process coordination; atomic claim/attempt ownership;
  fencing for file and DB publication; PREPARED/cancellation arbitration;
  read-only COMMITTED directory+DB verification; service failpoints and process
  restart matrix. Consequently production v2 remains fail-closed and D1 remains
  gated.
- Historical capability observation: WSL2/Linux, PowerShell 5.1 interop available,
  but `E:` is ReFS, no formal Windows project tree or NTFS scratch target/fresh
  artifact was found, and only `x86_64-unknown-linux-gnu` Rust target was
  installed at that checkpoint. Re-detect before making a current capability
  claim. Windows adapter/build/NTFS acceptance remains Windows Owner work.

## C1 execution checkpoint — 2026-10-10 (superseded by the top section above)

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; base HEAD
`346e538fce32a630b120bb4570159f358885cea0`. The worktree contains pre-existing
uncommitted shared C1 implementation changes plus this documentation update; this
is not a formal handoff. Environment detected as WSL2/Linux with only the Linux
Rust target and an ext4 workspace. No Windows validation is claimed.

- Documentation Gate confirms the media-index assertion: inspected Sidecar
  extraction and model conversion enumerate from 1, core rejects zero, and
  migration 0001 requires positive indices. Full conversion-boundary audit is
  still required; no schema migration is currently justified by the evidence.
- Shared status remains `IN_PROGRESS`; C1.2–C1.4 are not `COMPLETED` or
  `VERIFIED`. At this checkpoint the then-current `fcntl` adapter opened a
  descriptor per acquisition and relied on process-scoped record locks;
  same-process and descriptor-lifecycle exclusion still required direct tests
  (since addressed by the later `flock` adapter, see the top section). The
  archive service recovery entry
  currently rejects v2 replay, and Desktop startup still fails v2 candidates
  closed. The existing service failpoint parameter is unused. Keep v2 fail-closed
  and D1 gated.
- Active implementation order is recorded in
  `docs/development/desktop-download-output-plan.md`: (P0) baseline/documentation
  gate; (P1) coordinator semantics; (P2) atomic lifecycle and PREPARED/cancel
  arbitration; (P3) attempt-isolated file writes; (P4–P5) shared commit/replay,
  finalization and read-only COMMITTED verification; (P6) service process
  interruption/restart matrix; (P7) Desktop wiring only after shared gates pass.
- Windows Owner remains responsible for the Windows coordination adapter, NTFS
  no-replace behavior and artifact-bound native validation after formal Git
  handoff. These are downstream platform gates, not a reason to stop shared Linux
  work. Windows evidence is `NOT_RUN`; no fresh artifact exists for this tree.
- Documentation Gate review: Plan, architecture, handoff and Windows queue must
  agree that the index mismatch is unproven, same-process lock semantics are an
  open shared gate, production v2 remains fail-closed, and Windows validation is
  not run. This checkpoint updates status only; it does not claim C1 behavior was
  implemented or verified.

## Plan C continuation — confirmed architecture decisions 2026-10-09

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; current HEAD
`260dfc6ad2419dffff7914f71fe61986a29e207c`, three commits ahead of origin.
The tree was clean at documentation-sync time and now carries uncommitted shared
implementation. This is not a formal Windows handoff; no Windows validation is
claimed by this continuation.

- Confirmed design: add a dedicated `xarchive-workflow` crate, with C1 archive
  recovery as its first consumer. Keep workflow policy/contracts independent of
  Desktop/Tauri and avoid a dependency cycle with Storage. Storage implements
  SQLite transactions and persistence/file adapters; Desktop retains scheduling
  and runtime recovery entry points. Keep the initial crate scoped to C1; do not
  migrate unrelated workflows or build a general distributed scheduler. Durable
  database facts are atomic within SQLite transactions; file operations and
  Telegram network sends remain separate effects coordinated by recoverable steps.
- Confirmed concurrency contract: support same-host multi-process concurrent
  operation on one supported local data workspace. Different Jobs may run
  concurrently; same-Job operations and final-destination conflicts require
  cross-process coordination and fencing. Network filesystems and cross-host
  distributed execution are out of scope. Attempt-isolated staging is required;
  recovery resumes its recorded attempt, while retry creates a new attempt only
  after the old one loses commit rights. Persistent PREPARED is the irreversible
  commit boundary; cancellation may win only before it.
- Confirmed recovery facts: retain the immutable manifest boundary and persist a
  separate minimal attempt-bound candidate if existing durable facts are
  insufficient. The candidate is independent of optional user JSON/TXT exports and
  excludes signed URLs, headers, cookies and browser secrets. Keep whole-phase
  two-phase replay, atomic platform no-replace moves, fail-closed conflicts and
  read-only COMMITTED verification. Initial crash acceptance is process
  interruption/restart, not sudden power-loss durability.
- Confirmed E policy: Rust alone validates user downloader-option policy at task
  acceptance and before execution. Python retains protocol/argv-boundary handling
  without duplicating the option allowlist. This depends on all supported user
  argument routes passing through Rust; trusted internal Sidecar arguments remain
  distinct.
- This uncommitted continuation implements initial `xarchive-workflow` pure
  attempt/phase logic, the Unix `fcntl` record-lock coordination adapter
  (`LinuxCoordinator`) and Sidecar v2 `download.user_args` structural/argv
  support. Targeted Rust tests pass: 31 protocol, 11 sidecar-supervisor, 3
  workflow unit plus 4 true cross-process coordination integration tests
  (second-process contention, crash recovery, parallel jobs, destination
  exclusivity), plus the unaffected Storage 106 and Download 36 module
  suites; all 67 Sidecar Python tests pass; `cargo fmt --all -- --check`,
  `git diff --check` and the docs audit pass. Strict
  `cargo clippy --workspace --all-targets -- -D warnings` still FAILS
  on the pre-existing baseline only: `xarchive-storage/src/database/jobs.rs:183`
  and `:204` (`too_many_arguments`), `desktop/src-tauri/src/archive.rs:166`
  (`too_many_arguments`) and `desktop/src-tauri/src/websocket_transport.rs:488`
  (`useless_conversion`). None of those functions are touched by this batch, and
  no production code was changed to silence them. `xarchive-workflow`,
  `xarchive-sidecar-supervisor` and `xarchive-protocol` each pass strict Clippy
  cleanly, so this batch introduces no new lint. E has no Rust option policy, v3
  snapshot or Desktop task wiring; C1 has no OS adapter, persistence, filesystem
  replay or production caller.
- Still PLANNED: OS-backed coordination
  adapters and multi-process race/recovery tests; attempt-isolated staging;
  attempt-fenced transactional finalization; complete rename replay and COMMITTED
  verification; E v3 task-spec persistence, Desktop task wiring,
  Rust aria2 argv wiring and explicit tool/policy compatibility matrix.
- E compatibility decision: preserve v1/v2 historical specs and add v3 for
  downloader-argument snapshots. Historical tasks use empty custom arrays and are
  never resampled. Execution-spec, recovery-contract and option-policy versions
  are dispatched independently. gallery-dl user options use an explicit typed
  Sidecar v2 `download.user_args` command field (separate from trusted
  `executable_args`); aria2
  argv is built by Rust. Python retains protocol and application-invariant checks
  but no duplicate user-option allowlist. Unknown/incompatible options or tool
  versions fail before spawn.
Documentation synchronization for these decisions is recorded here; do not infer
implementation completion or acceptance from them.

## Plan C continuation — 2026-10-08 (historical; Linux continuation recheck)

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; source baseline
`8b2b544206e62e1356ef582e46d62b92be14cb04`. At session start the local branch
matched origin and the worktree was clean. This is not a Windows handoff.

Linux targeted validation was rerun on that exact source: Storage 106/106,
Desktop 269/269, Download 35/35 — PASS. The Full Rust workspace tests/doc-tests,
Sidecar pytest 63/63, Desktop Node 220/220, Extension Node 52/52, Desktop Vite
build, formatting and docs audit also passed. These results do not establish C1
production acceptance. A tentative E typed-args prototype was not completed:
protocol validation failed to compile before the remaining suites ran, and the
prototype was removed. The full workspace regression subsequently passed; Windows
native validation was not performed.

`CROSS_PLATFORM_CHANGE_REQUIRED` remains open for C1: production v2 dispatch and
replay, atomic DB/Tweet/media/Job/journal plus optional Telegram finalization,
attempt fencing, COMMITTED filesystem/DB verification, and service-level crash
matrix must be implemented and validated before production v2 can be enabled.
D1 remains gated. E remains shared work: its existing allowlist conflicts with
the protected-options contract; task-spec persistence, Sidecar/gallery-dl and
aria2 argv wiring, shared fixtures and failure tests are absent. F remains shared
work. These are not Windows blockers and must not be substituted with manual
validation.

No Windows validation or formal platform handoff is claimed. Windows-specific
work is accumulated in `docs/validation/windows-queue.md` and the consolidated
manual checklist at the beginning of `docs/validation/windows-manual-steps.md`.
Windows results must bind to a future formal handoff SHA and fresh artifact.

### Validation ledger — 2026-10-08

- Source bound for the targeted Linux module results: `8b2b544206e62e1356ef582e46d62b92be14cb04`.
- PASS: Storage 106/106; Desktop Rust library 269/269; Download 35/35; Full Rust workspace tests and doc-tests; Sidecar compileall and pytest 63/63; Desktop Vite build and Node tests 220/220; Extension tests 52/52; `cargo fmt --all -- --check`; `git diff --check`; `node scripts/docs-audit.mjs`.

- 2026-10-09 编译与共享库测试：修复 `transition_archive_recovery_v2` 对 `ArchiveRecoveryManifestV2Record` 的 `manifest_json`/`rename_progress` 访问，移除 `current.manifest.rename_phase` 等旧字段引用；`manifest_json` 保持不可变计划，阶段推进改为 `rename_progress`/`phase` 列。
- Strict `cargo clippy --workspace --all-targets -- -D warnings`: FAIL. It reports existing `too_many_arguments` lints in `crates/xarchive-storage/src/database/jobs.rs` and `desktop/src-tauri/src/archive.rs::execute_archive_context`, and an existing `useless_conversion` in `desktop/src-tauri/src/websocket_transport.rs:488`. No production code was altered to suppress these findings.
- Full workspace tests passed before the final documentation-only edits; the final diff contains documentation only.

### 跨会话交接 — 2026-10-09（Linux 收尾；Windows 转手工队列）

> 本节记录当日交接时的历史状态；后续续作的权威状态见下方
> “Batch C1 continuation — 2026-10-09”新节。此节原 HEAD/工作树字段仅适用于该交接时点。

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; HEAD
`7135c87a587f590ce05d5beea44d3d7e01f08e39` is the docs-only reconcile commit.
Planned baseline for this batch: `8b2b544206e62e1356ef582e46d62b92be14cb04`.

#### Worktree state at handoff (verified)

- Modified, uncommitted: `crates/xarchive-storage/src/archive_recovery.rs`
  (+48/−14), `docs/status/platform-handoff.md`, `docs/validation/windows-queue.md`.
- Nothing staged, no untracked files. **Not yet committed and not pushed** —
  the next Task must commit this batch before requesting formal Windows handoff.

#### Resolved in this batch — PASS

- Fixed the compile failure in `transition_archive_recovery_v2`: it now
  deserializes the immutable plan from `current.manifest_json` and advances the
  mutable phase through the `rename_progress`/`phase` columns. All references to
  the non-existent `current.manifest.rename_phase` are removed. The
  `archive_recovery_v2.rename_progress` column already exists in
  `migrations/0017_archive_recovery_v2.sql:9` with a
  `CHECK (rename_progress IN ('PLANNED','TEMPORARY','FINAL'))` constraint, so
  **no migration change was needed**.
- Verified commands and results:
  - `cargo build -p xarchive-storage` — PASS.
  - `cargo test -p xarchive-storage --lib` — PASS, 106/106.
  - `cargo test -p xarchive-desktop --lib` — PASS, 269/269.
  - `cargo test -p xarchive-download --lib` — PASS, 35/35.
  - `cargo fmt --all -- --check` — PASS (exit 0).
  - `git diff --check` — PASS.
  - `node scripts/docs-audit.mjs` — PASS (0 dead links, 0 unlinked docs).

#### Still failing — FAIL (pre-existing, not introduced here)

- `cargo clippy --workspace --all-targets -- -D warnings` — FAIL on pre-existing
  `too_many_arguments` in `crates/xarchive-storage/src/database/jobs.rs`
  (`create_archive_job_with_optional_recovery_contract` and
  `create_archive_job_with_request_contract`, 8/7 each) and in
  `desktop/src-tauri/src/archive.rs::execute_archive_context` (8/7), plus an
  existing `useless_conversion` in
  `desktop/src-tauri/src/websocket_transport.rs:488`. No production code was
  altered to suppress these findings; the v2 diff does not touch those files.

#### Shared implementation still incomplete — NOT RUN / BLOCKED

- v2 two-phase rename execution, v2 startup recovery dispatch, transactional
  DB finalization of media rows and Telegram final-path update remain
  unfinished. C1 stays **fail-closed** in production: both
  `complete_local_archive` and `complete_sidecar_archive_with_failpoint` still
  reject an `InternalRenameV2` contract, and the executor still fails a v2
  recovery candidate with `ARCHIVE_RECOVERY_V2_NOT_READY`. C1/D1 rejection
  semantics are unchanged and D1 must not be enabled before this gate passes.
- `WQ-PLAN-C-C1-01` and `WQ-PLAN-C-D1-01` remain `BLOCKED` on that shared gate.
- E remains unimplemented for production (`downloader_args.rs` aria2 prototype
  allowlist still conflicts with the protected-options contract; persistence and
  argv wiring absent). F remains unimplemented. Both are shared work, **not**
  Windows-only manual work, and must not be relabeled as Windows checks.
- Full Rust workspace regression, Sidecar pytest and Desktop/Extension Node
  suites were **not** re-run for this uncommitted change; only the three targeted
  module suites above were executed.

#### Windows — NOT_RUN / BLOCKED (no PASS claimed)

- This session is WSL2/Linux with no bound Windows working tree, fresh artifact,
  NTFS target or authorized Windows Owner session. No native Windows evidence was
  produced and none may be inferred from the Linux results above.
- All `WQ-PLAN-C-*` items stay `NOT_RUN`/`BLOCKED`. Consolidated manual
  execution steps already exist at the top of
  `docs/validation/windows-manual-steps.md` (items 1–6) and must be executed only
  after the formal Git handoff against the exact source SHA, a fresh Full
  artifact, and recorded build/worker provenance, EXE/package SHA-256,
  Windows/WebView2, NTFS, DPI and tool versions.

#### Next-Task context that must be preserved

- Next-Task verification gate (exact, re-checked at 2026-10-09): the entire v2
  journal layer is **library-only with no production caller**. Verified by
  `grep -rn` across `crates/` and `desktop/`, excluding the defining and test
  files, `create_archive_recovery_manifest_v2`,
  `archive_recovery_manifest_v2` and `transition_archive_recovery_v2` have **zero**
  call sites; `transition_archive_recovery_v2` is exercised only by the unit test
  at `archive_recovery.rs:1055` (lines 1114–1129). So the phase-machine fix in
  this batch is compile- and test-verified but **not yet integration-verified**.
  By contrast the naming renderer *is* production-wired:
  `plan_archive_media_renames` is called by `archive_service.rs:63`
  (`preview_sidecar_archive`) and re-exported at `lib.rs:47`.
- Invariant that must hold at creation time: `create_archive_recovery_manifest_v2`
  omits `rename_progress` from its INSERT, relying on the
  `migrations/0017_archive_recovery_v2.sql:9-11` default `'PLANNED'`, so the
  `archive_recovery_manifest_v2` read-back check `manifest.rename_phase == "PLANNED"`
  holds and the digest covers the immutable plan only. Do not add
  `rename_progress` to the INSERT or the digest/re-plan ambiguity returns.
- Key file: `crates/xarchive-storage/src/archive_recovery.rs` — types
  `ArchiveRecoveryContract`, `ArchiveRecoveryDispatch`, `ArchiveRecoveryPhase`,
  `ArchiveRenamePlan`, `ArchiveRecoveryManifestV2`,
  `ArchiveRecoveryManifestV2Record`; DB methods
  `startup_recovery_dispatch`, `archive_recovery_contract`, `v2_journal_state`,
  `create_archive_recovery_manifest_v2`, `archive_recovery_manifest_v2`,
  `transition_archive_recovery_v2`; row mapper
  `archive_recovery_v2_record_from_row` (column order is now
  `job_id, attempt_count, tweet_row_id, archive_directory, manifest_json,
  manifest_sha256, rename_progress, phase, created_at, updated_at`).
- Design constraint: `manifest_json` is the **immutable** plan whose embedded
  `rename_phase` is always `PLANNED`; progress advances only via
  `rename_progress`/`phase`, and a digest mismatch is corruption, never a
  silent re-plan.
- Fail-closed call sites to change together: `archive_service.rs`
  (`complete_local_archive`, `complete_sidecar_archive_with_failpoint`) and
  `desktop/src-tauri/src/executor.rs` (`recover_startup` v2 branch).
- Do not retry the reverted E typed-args prototype: it failed to compile at the
  protocol boundary and was fully reverted. Do not enable D1 exports-off
  behaviour before the v2 recovery gate passes.
- Ordering constraint that stays: D0 → C1 → D1.
- The tentative E contract/argv prototype did not complete and was fully reverted; its protocol compile failure is retained here as a reverted experiment, not a final-source failure. E remains unimplemented for production.
- Windows native/runtime/NTFS/GUI checks: `NOT_RUN` in this Linux/WSL2 continuation; no fresh Windows artifact or formal handoff. Manual actions remain in `docs/validation/windows-manual-steps.md` and the authoritative queue.

## Batch C1 continuation — 2026-10-09

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Source commit:
`b9ac571448c9e093c1639ca0586718dc9dd7bcc4` (includes local commits through the
immutable-manifest phase-machine repair); origin remains at
`8b2b544206e62e1356ef582e46d62b92be14cb04`. The worktree was clean immediately
after the source commit. No formal Windows handoff or push is claimed.

### Scope and result

- Committed prerequisite repair: `transition_archive_recovery_v2` reads the
  immutable plan from `manifest_json` and advances only `rename_progress`/`phase`.
  Manifest JSON/digest remain immutable; v2 INSERT intentionally omits
  `rename_progress` and relies on migration 0017's `'PLANNED'` default. No
  migration change was made.
- Current-tree validation before commit: `cargo build -p xarchive-storage` PASS;
  Storage 106/106, Desktop Rust library 269/269, Download 35/35 PASS;
  `cargo fmt --all -- --check`, `git diff --check`, and docs audit PASS. These
  prove the compile/unit/module scope only.
- C1.1 representation/compiler repair is committed; **C1.2/C1.3/C1.4 remain
  IN_PROGRESS**. The v2 journal APIs remain without production callers. No
  two-phase rename service replay, transaction combining Tweet/media/Job/v2
  journal/Telegram intent, COMMITTED replay verification, attempt fencing, or
  service-level restart crash matrix has been completed. Both archive service
  fail-closed guards and executor `ARCHIVE_RECOVERY_V2_NOT_READY` remain in force.
  Do not enable D1; preserve D0 → C1 → D1.
- E remains **IN_PROGRESS / not production-ready**. The old typed-args prototype
  must not be retried. Current allowlist/protected-options conflict, and shared
  typed fixtures, task-spec persistence, gallery-dl validation, and both actual
  argv paths remain outstanding. E/F are shared implementation, not Windows
  manual work.
- E contract disposition: `CROSS_PLATFORM_CHANGE_REQUIRED`. A targeted regression
  test records that the current aria2 prototype accepts `--out` and `--all-proxy`,
  both forbidden by the protected-options contract. The test intentionally
  documents the known gap; it is not a security acceptance test and does not
  make E production-ready.
- Full workspace, Sidecar, Desktop/Extension Node, and Windows checks were not
  run for this source. Strict Clippy's previously recorded findings are
  pre-existing and were not re-run.

### Environment and validation ledger

- Detected Linux x86_64 under WSL2 (`6.18.33.2-microsoft-standard-WSL2`), Cargo
  available. WSL interop exists, but no bound Windows canonical working tree,
  fresh artifact, NTFS acceptance target, or authorized Windows Owner session
  was established. Therefore all applicable Windows checks remain `NOT_RUN` or
  `BLOCKED`; no Windows PASS is claimed. Manual Windows steps remain at the top
  of `docs/validation/windows-manual-steps.md`.
- Source-bound PASS: Storage build; Storage 106/106; Desktop library 269/269;
  Download 35/35; formatting; diff check; docs audit.
- NOT RUN: full Rust workspace, Sidecar pytest, Desktop/Extension Node suites,
  Windows native/NTFS/GUI/artifact validation.
- FAIL (known prior result, not rerun): strict workspace Clippy reports existing
  `too_many_arguments` in Storage `database/jobs.rs` and Desktop
  `archive.rs::execute_archive_context`, and `useless_conversion` in
  `websocket_transport.rs:488`.

## D0 Linux safety acceptance — 2026-10-07

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; source baseline
`980123ac31e1c709b2fd4878ad9e20ccc73549d8`. This batch completes the D0 Linux
safety acceptance: crash-boundary fault injection at the Storage service
boundary (manifest journal write retry/conflict, post-rename DB-row transaction
rollback leaving journal PREPARED and no partial archive facts, Telegram intent
linkage transition PREPARED→ARCHIVED plus missing-intent fail-closed
verification) and executor startup fail-closed integration tests (missing
journal → `ARCHIVE_COMMIT_RECOVERY_FAILED`, unknown contract version →
`ARCHIVE_RECOVERY_UNSUPPORTED_VERSION`, missing execution spec →
`EXECUTION_SPEC_MISSING`, none producing a false COMPLETE), plus the happy path
recovering a valid InternalV1 archive to exactly one COMPLETE across a repeated
startup.

Validation record: target D0 crash boundaries and executor startup recovery;
priority P1; owner Cross-platform Owner; implementation IMPLEMENTED; status PASS
for `cargo test -p xarchive-storage --lib` 94/94 and
`cargo test -p xarchive-desktop --lib` 266/266; `cargo fmt --all -- --check`,
`git diff --check` and `node scripts/docs-audit.mjs` PASS; source baseline
`980123a`, batch implementation commit
`e8c2622d93883c52f4041b3e0a09449d63c7f792`; local Cargo test origin;
artifact SHA not applicable; Linux x86_64 under WSL2; automated; bundled SQLite;
evidence from this session. Windows NTFS interruption/rename and fresh-artifact
checks remain `NOT_RUN` under `WQ-PLAN-C-D0-01`. D0 Linux safety gate is
satisfied; C1 implementation may start. This entry is not a Windows result or an
ownership transfer.

## Progress sync — 2026-10-07

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Implementation commit
`e4a0bb1575381a29c014266313703eb230f8bbae`; handoff/status documentation
follow-ups `dd1ffbbeba3874fe69fd39cf4d278c8696a3a916` and
`0d590fc0edee239034e8b27a07be60b1b44a7666` (pushed tip). D0 remains `IN_PROGRESS`,
C1/D1 remain gated, and Windows validation
remains `NOT_RUN`. This is not a completed Windows handoff.

## Plan C cross-platform implementation — 2026-10-07

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; source baseline
`7e8fe9297409d0ec72b8594b6d0c2e5b137dff41`. This entry describes the initial
pre-sync state; the continuation entry below records the current pushed source
revision and ownership state. This session is Linux under WSL2/Ubuntu (kernel
`6.18.33.2-microsoft-standard-WSL2`); Windows-native filesystem/runtime and
artifact-bound GUI validation were not available.

C1 and D1 remain **not implemented** and gated on finishing D0 acceptance.
This batch implements the independent D0 manifest/journal and production recovery
path, but C1 filename mapping and D1 export controls have not started. Their
shared contract remains `CROSS_PLATFORM_CHANGE_REQUIRED`.

Plan and Windows queue/manual procedures were updated. D0 Linux Storage and
Desktop library tests, formatting, diff check and docs audit pass (evidence below).
`WQ-PLAN-C-D0-01` is now implemented but Windows `NOT_RUN`; `WQ-PLAN-C-C1-01` and
`WQ-PLAN-C-D1-01` remain `BLOCKED` until their shared implementation exists.
No Windows PASS is claimed. Full workspace regression was not run because this
batch changes Storage/archive recovery and its Desktop executor integration;
Storage plus Desktop library tests cover the selected subsystem scope. No Windows
/goal 完成 Plan 中所有不依赖 Windows 环境的开发与测试，最后统一汇总 Windows 专属验证项目。对于 BLOCKED 的 Windows 验证跳过并生成手工验证步骤。/goal 完成 Plan 中所有不依赖 Windows 环境的开发与测试，最后统一汇总 Windows 专属验证项目。对于 BLOCKED 的 Windows 验证跳过并生成手工验证步骤。PASS is claimed; later progress-sync details are recorded below.

## D0 recovery contract — 2026-10-07 continuation

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; source baseline:
`7e8fe9297409d0ec72b8594b6d0c2e5b137dff41`. The implementation tree includes Storage
`archive_recovery.rs`, `archive_service.rs`, `database/jobs.rs`,
`database/tweets.rs`, migration `0016`, and `lib.rs`; Desktop executor; the
pre-existing Download Config UI/test edits; Plan, queue, manual-steps, and
handoff documentation. The progress-sync continuation below records their Git
commit and remote status.

Implementation is **IN_PROGRESS**; Linux verification is **PASS** for the selected
Storage and Desktop library suites, formatting, diff check and docs audit. New v2
production jobs opt into recovery contract v1; the compatibility creation API
leaves the marker NULL. The journal stores a strict versioned manifest with
Tweet/Job/attempt identity, final relative media paths/index/ID/size/hash and
Telegram intent linkage. Startup recovery validates both the journal and the
on-disk manifest, checks actual files, then reconstructs only manifest-proven
archive/media rows. It does not read user exports. Rich Tweet metadata remains
preserved/updated only from the normal trusted result path, not inferred from the
minimum recovery record.

Legacy recovery remains available only when the explicit contract marker is NULL;
unknown non-NULL versions fail explicitly. Migration 0016 follows 0015 and leaves
existing jobs on legacy recovery. Added targeted tests cover strict/unknown
manifest fields, traversal, duplicate paths, hash validation, manifest tampering,
and staging-only/final-only recovery. Crash injection across every file/SQLite
boundary and transaction rollback/retry behavior still needs further coverage.

Linux validation on this working tree: `cargo test -p xarchive-storage --lib`
PASS (91/91); `cargo test -p xarchive-desktop --lib` PASS (262/262);
`cargo fmt --all -- --check` PASS; `git diff --check` PASS; and
`node scripts/docs-audit.mjs` PASS. Full workspace regression was not run; this
batch's selected blast radius is Storage plus Desktop library recovery integration.

D0 manifest contract and initial production integration are implemented, but
acceptance is not complete: missing/corrupt manifest and changed-media rejection,
unknown-contract fail-closed behavior (including no exported-metadata fallback),
and post-commit replay are covered at the Storage service boundary. Fault injection
for manifest write, rename, DB row recovery, intent transition and repeated startup
recovery remains incomplete. Executor startup's async fail-closed integration is
not yet directly tested. Therefore
D0 stays `IN_PROGRESS`; do not begin C1/D1 until these safety tests pass.
Validation record: target Storage archive recovery and Desktop executor recovery;
priority P1; owner Cross-platform Owner; implementation IN_PROGRESS; status PASS
for 91 Storage tests, 262 Desktop tests, formatting, diff check and docs audit;
source baseline `7e8fe92` plus the progress-sync commit recorded below; local Cargo test origin;
artifact SHA not applicable; Linux x86_64 under WSL2; automated/static; bundled
SQLite; evidence is command output from this session; blocks release: yes for
C1/D1; Windows revalidation required on exact handoff source and fresh artifact.
The progress-sync commit and push state are recorded below.

External/manual work: Windows native NTFS interruption/rename behavior and
fresh-artifact checks are **NOT_RUN** for this source; this WSL2
session cannot provide artifact-bound native evidence. After formal Git handoff,
the Windows Platform Owner executes `WQ-PLAN-C-D0-01`; the check is verification
pending and does not block further Linux safety-test development.

Known broader status: B1 and initial B2 wiring are implemented; B2 remains
partial; D0 implementation is in progress pending remaining fault-injection
coverage; C1/D1 are gated; E/F contracts are recorded but their implementation
has not started. Windows-specific checks remain pending/not run. See the
progress-sync continuation below for current HEAD and handoff state.

## Plan C P0/P1 continuation — 2026-10-07

State: `CROSS_PLATFORM_IN_PROGRESS`; branch
`cross-platform/automatic-pairing-reconcile-20261002`; source baseline
`3744e8880b7a44537ff7acf177b3c145dcc22772`. Implementation and verification
are committed at source revision
`575e2a1006ecf704d8fb88324baf17b0cf278010` and pushed to the configured GitHub remote. The
Cross-platform Owner retains ownership for the next shared batch; this is not a
Windows validation result or ownership transfer.

Implemented in this working tree: output settings get/save through `settings_meta`,
Download Config controls, one-off Tauri/browser submission snapshot capture,
account-batch creation snapshot capture, and child inheritance. Tests cover typed
settings persistence, defaults/corruption validation and execution-spec snapshot
encoding. The UI states clearly that rename/export effects are not active yet.
`Tweets/<tweet_id>` identity and historical execution-spec v1/default behavior
remain unchanged. D0 is a prerequisite for enabling C1/D1 because current recovery
reads `tweet.json`; therefore the request to include effective C1/D1 now is
deferred rather than weakening recovery safety. E remains draft-only.

Escalation: `CROSS_PLATFORM_CHANGE_REQUIRED` for shared settings persistence,
execution snapshot wiring and recovery/output semantics. Windows implementation or
acceptance is not requested before this Linux batch is complete.

Validation: Storage 83/83; Desktop library 262/262 (including
`transport::tests::persisted_output_settings_are_used_by_default_browser_adapter`);
Desktop UI targeted test command 34/34; Vite production build; `cargo fmt --all
-- --check`; Desktop all-target `cargo check` passed; `git diff --check` and
`node scripts/docs-audit.mjs` passed. Strict Desktop Clippy
is not clean due to the existing unrelated `clippy::useless_conversion` at
`desktop/src-tauri/src/websocket_transport.rs:488`; no unrelated code change was
made. An initial Desktop check caught the regression test attempting to read a
nonexistent in-memory spec accessor; the test now uses `StorageJobPersistence`
and reopens the SQLite execution-spec row. An earlier focused Node run exposed an
assertion mistakenly checking the batch request field in `main.jsx`; it was
corrected to check `batches-page.jsx`, and the focused tests passed. Working-tree
Windows native/runtime/GUI checks are `NOT_RUN`; no Windows PASS is claimed.

## Plan C continuation — 2026-10-07 Linux working-tree review

State: `CROSS_PLATFORM_IN_PROGRESS`; branch
`cross-platform/automatic-pairing-reconcile-20261002`; source baseline
`44b8e77`. This continuation is **uncommitted** and remains owned by the
Cross-platform Owner. No formal handoff or Windows artifact is created by this
record.

The interrupted Batch E working tree was inspected before further work. Its Rust
parser/helper and Python tests are local, uncommitted changes, not an end-to-end
feature: there is no persisted downloader-argument field in the v2 execution spec,
no Sidecar v2 per-task config field, and no aria2 launch integration. The draft
allows option/value pairs without binding or validating their values and stops
allowlist checking after `--`; it must not be treated as production-ready or
accepted as Batch E. Progress in
[`desktop-download-output-plan.md`](../development/desktop-download-output-plan.md)
was corrected accordingly.

Plan C scope review confirmed B1 and initial B2 wiring, existing batch snapshot
migration, and existing Telegram intent / `tweet.json` staging recovery. B2 remains
partial because batch creation supplies defaults without a settings source; D0
remains partial because existing archive recovery is not the independent complete
internal manifest/facts contract; C1, D1, and F remain unimplemented. These are
cross-platform contract batches and need implementation before their Windows
acceptance is testable. No claims of Plan C completion are made.

Linux evidence for this continuation: `cargo test -p xarchive-download --lib`
33/33 PASS (includes seven local parser-draft unit tests); `cargo test
-p xarchive-storage --lib` 82/82 PASS; `python -m pytest sidecar/tests -q` 63/63
PASS; `npm test --workspace desktop` 220/220 PASS; `cargo fmt --check` and
`git diff --check` PASS after correcting the interrupted Rust formatting/module
closure. An invocation with Jest-only `--runInBand` was rejected by Node (tool
command misuse) and rerun correctly; no product assertion failed. Full workspace
regression was not run because the E code is an unaccepted local draft and C1/D1/F
are not implemented; the tested crate/module suites are scope-limited evidence.

Windows status: native Windows process/ACL, packaged Sidecar, archive rename/recovery,
GUI, and DPI checks are `BLOCKED` in the current Linux/WSL2 execution environment
and `NOT_RUN` for this source revision. Windows Owner must wait for a committed
implementation handoff, build a fresh artifact from its exact source SHA, and then
execute the new Plan C checklist in [`../validation/windows-queue.md`](../validation/windows-queue.md).
Do not validate this uncommitted draft as a production artifact or reuse older
artifact PASS results.

## Plan C Windows validation summary — 2026-10-08

State: `WINDOWS_VERIFICATION_PENDING`. Cross-platform implementation is recorded
in source commit `09e5bff` on branch
`cross-platform/automatic-pairing-reconcile-20261002`. The handoff documentation
follow-up commit is `ad2ee67`. Current Owner: Cross-platform Owner until both
commits are pushed; Next Owner:
Windows Platform Owner for the queued native/runtime checks after fetching that
revision.

B1 execution-spec version dispatch and the initial B2 typed snapshot path are
implemented. Storage persists batch output snapshots with a migration; one-off
tasks persist v2 specs, and batch dispatch inherits the persisted snapshot.
Active-job reuse is checked before Tweet/User writes so reuse does not replace
the stored task input. Batch creation currently passes default settings because
there is not yet a configurable UI/request source; B2 remains partial. Archive
journal/recoverable naming, metadata export behavior, downloader allowlist, and
configuration import constraints remain planned. See
[`desktop-download-output-plan.md`](../development/desktop-download-output-plan.md).

Linux evidence: Storage tests 82/82, Desktop tests 257/257, Sidecar pytest
61/61, Python compileall, formatting, diff check, and documentation audit
passed. Strict Clippy did not pass because of the existing unrelated
`clippy::useless_conversion` in `desktop/src-tauri/src/websocket_transport.rs:482`;
no unrelated production change was made. Windows native GUI/runtime/package
validation is `NOT_RUN`; no Windows PASS is claimed.

Windows follow-up: fetch this handoff revision, build a fresh artifact from that exact revision, and run the manual Windows queue in
[`../validation/windows-queue.md`](../validation/windows-queue.md) (section `2026-10-06 Desktop download output snapshot B1/B2 — 当前批次`):
fresh-artifact task submission and reuse (`MANUAL-WQ-B1-B2-01/02`), batch pause/resume/retry snapshot inheritance (`MANUAL-WQ-B1-B2-03`),
migration from an existing database (`MANUAL-WQ-B1-B2-04`), output settings behavior (`MANUAL-WQ-B1-B2-05`), and native filesystem/runtime
behavior (`MANUAL-WQ-B1-B2-06`). Record evidence against the exact tested source and artifact; no Windows PASS is claimed in this Linux session.


## Desktop UI closeout — 2026-10-06

State: `WINDOWS_VERIFICATION_PENDING`. This batch resumes the existing desktop
UI handoff. Implementation and verification changes are committed at
`d406099`; this document-only handoff update will identify the final source
revision. The combined download screen is split into
`任务记录`, `下载配置`, and `存储`; `运行日志` is in secondary navigation.
No unrelated plan was included. Windows must fetch the final Git handoff head
and build a fresh artifact from that exact revision.

Local evidence: `npm run check --workspace desktop` passed (Vite production
build; existing Tauri API mixed-import warning remains). The complete Desktop
Node suite passed 220/220. Focused Node UI checks
passed 54/54, including split-page wiring, settings rendering, and Sidebar event
regression coverage. A final
pagination review found that a stale page index could outlive a result-count
change; the displayed page number is now bounded to the current page count, with
a regression assertion. `node scripts/docs-audit.mjs` and `git diff --check` passed.
An initial test invocation passed unsupported `--runInBand` to Node and exited 9;
it was a command-line mistake, not a product failure. The focused checks were
rerun with the repository's Node test runner. Full workspace regression was not
run because this is a frontend-only UI relocation with targeted page/wiring
coverage.

Environment detected: Linux under WSL2; PowerShell interop and display variables
are present, but this session has no artifact-bound Windows Owner GUI/runtime
evidence. No native Windows UI validation was performed. Ordinary/maximized and
narrow layouts, 21-item pagination, keyboard/focus, aria2 controls and persistence,
storage chooser persistence, and 100/125/150/200% DPI remain
`WINDOWS_VERIFICATION_PENDING` and are consolidated in
[`windows-manual-steps.md`](../validation/windows-manual-steps.md#desktop-ui-closeout--2026-10-06-linux-batch)
and `WQ-UI-CONSISTENCY-01` in the Windows queue. No Windows PASS is claimed.

Current Owner: Cross-platform Owner until the final handoff document is committed
and pushed. Next Owner: Windows Platform Owner for fresh-artifact manual
validation. Implementation commit: `d406099`; formal handoff source revision is
the final documentation commit containing this record.

## Reconciled Windows 40858835 return — 2026-10-06

State: `WINDOWS_VERIFICATION_PENDING`. Cross-platform branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Fetched Windows return
branch: `codex/windows-validation-40858835`; requested target
`eaca6f810f8c3a43be4ec52ca7c0c8bde4ff8d88`, based on the Linux handoff
`40858835d249ead37ad1f3efb2dc1571821f9f8f`. Return documentation records input
UI implementation `8a8fa8ab3cb4725282fd10d15aa178deda6fcbfd`, handoff
`a3b7870b12eb5df5ee86139c89f23a2e8a671818`, and Windows implementation/validation
`60b76f357acbcf3510fa9bfcdc32309a35ca97cf`.

The Windows return is Git-integrated. Its `60b76f3` shared callback repair is
already an ancestor of the received Linux handoff `4085883`; the return added the
Sidebar prop wiring and event regression test, which are now present on this
branch. Targeted Linux Node checks passed: Sidebar event + UI wiring 51/51 and
UI-state + Telegram-render 32/32. These source checks do not replace Windows GUI
acceptance. Reconciliation/source revision: `d7d9c23`; subsequent changes are
handoff-record documentation only. Formal record source is the latest commit on
`cross-platform/automatic-pairing-reconcile-20261002`. Current Owner: Cross-platform
Owner until the final record is pushed; next Owner: Windows Platform Owner. Fetch
that branch and use its latest commit. No implementation changes remain
uncommitted.

Windows evidence remains scoped: frontend 83 PASS, Storage 74 PASS, Desktop
257 PASS/1 ignored, and fresh Full ZIP/component integrity checks PASS within the
recorded limits. Worker source-build provenance is unconfirmed. Native empty /
history / 20+1 / index-0, ordinary/maximized layout, targeted Tab/Enter anchors,
Extension click-through and backend warning copy passed scoped checks. Summary
index 19 and date-entry automation were BLOCKED; full DPI/narrow/five-state/error/
focus matrix, real downloads/browser/Telegram, worker provenance and archive
completion remain pending. No whole-GUI, release or archive-completion PASS is
claimed. Preserve prior FAILs and runtime-metrics `IMPLEMENTATION_NOT_READY`.
See [batch plan](../development/windows-40858835-validation-plan.md),
[exact results and manual queue](../validation/windows-40858835-results.md),
and [current queue](../validation/windows-queue.md).

## Historical input: Desktop GUI consistency continuation — 2026-10-06

State: `READY_FOR_WINDOWS`. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Source commit:
`a3a022286d79de7d8eac290618cdf4b7fbd6afb1`; `git fetch --all --prune` found
the branch aligned with origin at start. Earlier UI commits `2398b01` and
`4bfc04b` are superseded by the current shared paging-location correction;
neither is a Windows validation artifact for this batch. Formal handoff source
implementation commit: `8a8fa8ab3cb4725282fd10d15aa178deda6fcbfd`; formal
handoff commit: `a3b7870b12eb5df5ee86139c89f23a2e8a671818`, pushed to
`origin/cross-platform/automatic-pairing-reconcile-20261002`; the tracked tree
was clean after that commit. Current Owner: Windows Platform Owner. The previous
download-history handoff at `72762cd` is historical and is not the artifact
revision for this continuation.

### Cross-platform changes in progress

- Fixed the dashboard five-item job row with responsive grid columns, visible
  focus treatment, and retained the existing plain button and Badge components.
- Added a card-like, responsive download-history layout with clear “任务记录” /
  “下载设置” anchors, an explicit page title, and accessible section names. The
  settings remain a single source of configuration state; no framework or
  dependency was added.
- A final audit found that dashboard jobs sort by `updated_at` while history pages
  sort by `created_at`; a summary-list index alone can target the wrong history
  item. **CROSS_PLATFORM_CHANGE_REQUIRED**: added a storage query and Tauri command
  that return the job's zero-based index in the exact history ordering, then route
  the detail link through that index. A SQLite regression fixture changes the two
  orderings deliberately and asserts the returned history positions.
- Centralized the dashboard Extension connection label mapping, preserved
  unknown as “状态未知”, and made its status action open the matching disclosure.
  Extension diagnostics are labeled as cumulative history, not current-session
  counts. Batch dates share field styling and constrain start/end dates.
- Replaced download-backend help that claimed a configuration change would not
  affect active jobs. It now warns that executor/transport rebuild can interrupt
  active tasks; no lifecycle behavior or shared runtime contract was changed.
- Existing shared React/CSS adjustments preserve their abstractions and use
  `CROSS_PLATFORM_REVIEW_REQUIRED`. The history-position command changes the
  shared Desktop command surface, so this correction is explicitly
  `CROSS_PLATFORM_CHANGE_REQUIRED` and is completed by the Cross-platform Owner.

### Verification and remaining work

- Targeted Desktop Node tests: PASS, 82/82 (`ui-state`, `ui-wiring`,
  `telegram-render`); wiring verifies Tauri registration and shared SQL query.
- `cargo test -p xarchive-storage --lib --no-fail-fast`: PASS, 77 tests,
  including the created-order versus updated-order page-position regression.
- `cargo test -p xarchive-desktop --lib --no-fail-fast`: PASS, 251 tests.
- `cargo fmt --all -- --check`: PASS. The initial check reported only rustfmt
  differences in the newly edited Rust files; `cargo fmt --all` was applied and
  the check rerun successfully.
- `npm run check --workspace desktop`: PASS; the existing mixed static/dynamic
  Tauri API import warning remains.
- `node scripts/docs-audit.mjs`: PASS. `git diff --check`: PASS. Full workspace
  regression was not run because the targeted storage, Desktop module and
  frontend lanes cover the affected query/command/UI path.
- WSL2 environment: Edge executable is visible through `/mnt/c`, but this session
  did not launch it. No artifact-bound Windows-owner GUI session, target build,
  or authorized account is available. Browser rendering, ordinary/maximized and
  minimum window sizes, long content, empty state, 21-task page transition,
  20+1 pagination, date picker, and DPI behavior remain `WINDOWS_VERIFICATION_PENDING`.
- Triage: the first full storage run exposed two incorrect expectations in an
  existing fixture with intentionally different created/updated ordering; they
  were corrected to the verified `created_at DESC, id DESC` positions, and the
  rerun passed. No product assertion was weakened.

### Windows handoff requirements

Windows Owner must checkout handoff commit `a3b7870`, build a fresh Full artifact
from that exact revision, and record artifact SHA-256. Execute the current
WQ-DL-01..08 prerequisites plus the new
`WQ-UI-CONSISTENCY-01` row in `docs/validation/windows-queue.md`. Validate date
entry with a controlled fixture only; do not enter real account credentials,
send Telegram test messages, or delete saved credentials for this UI pass.
Existing historical Windows evidence does not close this changed UI scope.

Next Owner: Windows Platform Owner for fresh-artifact rendering and native
interaction validation.
Do not mark the feature complete until the exact-revision Windows items are
reconciled.

## Download history and dashboard continuation — 2026-10-06

State: `CROSS_PLATFORM_IN_PROGRESS`. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Source commit:
`2e3d738eb6833d2c9b88f5f64b911a342d9470e5`. Formal handoff revision for this
Linux batch: `72762cd97247b881180a853b76714d29a543c28d`, pushed to
`origin/cross-platform/automatic-pairing-reconcile-20261002`. Current Owner:
Cross-platform Owner. The tracked tree is clean at the handoff revision; no
formal Windows acceptance is made by this record. The Windows Owner must build
from `72762cd`, never from the pre-fix baseline `2e3d738` or a copied working
tree.

### Implemented in this increment

- Added migration `0014_download_task_metrics.sql`, typed download metric and
  paginated-job models, paginated storage queries, metric persistence with
  stale-attempt fencing, and storage tests for pagination and fencing.
- Added Tauri commands for paginated task history and per-job download metrics.
- Wired the Downloads page with 20-item pagination, archive-directory controls,
  persisted-metric display, and navigation from dashboard task summaries;
  removed the storage panel from Settings while preserving its other controls.
- Runtime executor instrumentation is **not implemented**. UI must treat
  unavailable historical or runtime metrics as unavailable; do not infer
  duration or speed from unrelated task timestamps.
- Added explicit source-contract regression coverage for 20-item pagination,
  page offsets, dashboard "show all" navigation, and job-detail metric lookup.

### Verification on the pre-handoff working tree

- `cargo test -p xarchive-storage --lib --no-fail-fast`: PASS, 76 tests.
- `cargo check -p xarchive-desktop --all-targets`: PASS.
- `npm run check --workspace desktop`: PASS, with the existing Tauri API
  mixed static/dynamic import warning.
- `npm test --workspace desktop`: PASS, 216 tests after updating the UI wiring
  assertions to follow the storage/archive-directory controls to Downloads.
- `cargo fmt --all -- --check`: PASS. Rustfmt formatting changes affect the
  five listed Rust files only; no executor metrics contract was implemented.
  formatting.
- `git diff --check`: PASS. Full repository regression was not run; the change
  is limited to focused UI assertions and handoff/validation documentation.
- Windows native GUI, directory picker, and real download backend validation
  remain `BLOCKED` in this WSL2 session and `WINDOWS_VERIFICATION_PENDING` for
  the Windows Owner; see WQ-DL-01..08 and manual steps below.

### Goal continuation — 2026-10-06

- Final Linux checks on the handoff source: Desktop Node tests PASS (216/216);
  Desktop Vite production build PASS (exit 0; existing mixed static/dynamic
  Tauri API import warning); `cargo test --workspace --lib --no-fail-fast` PASS
  (see recorded workspace log for package-level counts); and
  `cargo fmt --all -- --check` PASS. `node scripts/docs-audit.mjs` reports PASS.
- `cargo clippy --workspace --all-targets -- -D warnings` FAIL: the
  `xarchive-desktop` target triggers `clippy::useless_conversion` under
  `-D warnings` at `desktop/src-tauri/src/websocket_transport.rs:482` on the
  existing `CloseFrame` conversion, outside the files changed in this batch.
  No unrelated product edit was made; track this as a separate Linux follow-up.
  Full repository regression was not run.
- In WSL2, `cargo test -p xarchive-storage --lib --no-fail-fast` passed (76
  tests), `cargo check -p xarchive-desktop --all-targets` passed,
  `cargo fmt --all -- --check` passed, and `npm run check --workspace desktop`
  passed with the existing mixed static/dynamic Tauri API import warning.
- The desktop Node suite was rerun after moving the stale storage and
  archive-directory source assertions to the Downloads page: PASS (216 tests).
  Full repository regression was not run.
- `git diff --check` passed. No Windows acceptance was attempted: this is WSL2
  with GUI/Windows command bridges, but no artifact-bound Windows-owner session
  or authorized test account. WQ-DL-01..08 remain pending and are explicitly
  covered by P1–P8 manual steps in `windows-manual-steps.md`.
- Executor attempt metric wiring remains deferred pending an actual shared
  contract review. The current event surface does not define enough approved
  semantics to safely populate attempt-scoped timestamps/bytes; no metrics are
  inferred and no executor contract was changed in this continuation.
- The Linux-owned implementation and verification batch was committed and pushed
  as `72762cd97247b881180a853b76714d29a543c28d` on the branch above. The tracked
  working tree was clean at that handoff revision. Windows native acceptance
  remains pending and must use a fresh artifact built from that exact revision.

### Remaining work and ownership

1. Cross-platform Owner: keep executor attempt metric instrumentation deferred
   until the shared event/metrics contract is reviewed. It is not a prerequisite
   for the completed download-history UI handoff; do not infer timestamps,
   duration, bytes or speed from job timestamps.
2. Windows Platform Owner: perform WQ-DL-01..08 on a fresh artifact built from
   `72762cd97247b881180a853b76714d29a543c28d`, recording a separate result for
   every queue row. This Linux batch makes no Windows acceptance claim.

Current Owner: Windows Platform Owner for native artifact validation. Windows
implementation/GUI verification remains pending; do not mark this feature
complete until the shared implementation, Linux checks and subsequent Windows
queue are closed.

## Cross-platform batch — fd7d834 integrated + 4 non-Windows fixes — 2026-10-06

State: `READY_FOR_WINDOWS`. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Merge revision:
`6afb351` (merge of `fd7d834ceaae2fa72bd3eb7d17987d0e02622cf6` into
`5c3efb79d2c7ffe3c84fe1f33821cd94b6c77a49`); this handoff commit on top adds
the cross-platform follow-ups below. Current Owner: Cross-platform Owner.
Next Owner: **Windows Platform Owner**.

### Integrated Windows return (fd7d834, no conflicts)

- Merged `origin/codex/windows-validation-5c3efb79` (`fd7d834`) with `--no-ff`.
  Brings Windows-only PAC adapter repair `44e60e3`, Full `2b98952` build +
  validation docs, and user manual feedback with two scoped FAILs.
- `44e60e3` review: touches only
  `desktop/src-tauri/src/system_proxy_resolver.rs` (a `#[cfg(windows)]`
  module). No shared abstraction, schema, protocol, helper, or dependency
  changed — no `CROSS_PLATFORM_REVIEW_REQUIRED` approval needed for it.
- Requested reconcile per `docs/development/cross-platform-validation.md`
  §16 remains not applicable: that file is navigation-only and has no §16;
  ownership/Git-handoff/validation-policy sources apply instead
  (`git-platform-handoff.md` §16 revision template used).

### Cross-platform follow-ups completed (no Windows environment needed)

All three feedback items triaged; narrow Linux-owned repairs implemented:

1. **WQ-FULL-STATUS-01** (sidebar/panel disagreement + stale port):
   `browser_connection` combines live WebSocket socket state with a 30s
   Named Pipe freshness window, while the panel shows `websocket_*` live
   state — the same snapshot can legitimately disagree after bootstrap
   activity ages out. `save_network_settings`/`set_use_aria2` rebuild the
   executor + transport (possibly a new port) without refreshing the panel.
   Fixes: status copy now names both sources and points at `websocket_*`
   (`commands.rs`); both save handlers call `refreshExtension()` after
   success (`main.jsx`); wiring regression added (`ui-wiring.test.mjs`).
   Full semantic unification (single canonical source) is deferred as
   `CROSS_PLATFORM_CHANGE_REQUIRED` design — needs Windows input on which
   source is authoritative for bootstrap-only activity.
2. **WebSocket close-within-3s FAIL** (`CROSS_PLATFORM_REVIEW_REQUIRED`):
   `handle_websocket_connection` broke out of the loop on `Message::Close`
   without replying; tungstenite queues the close echo on `read` and only
   sends it on the next `read`/`flush`. The peer therefore never saw a
   graceful close. Fix: reply + flush before break, per RFC 6455 §7.1.2.
   Regression: `authenticated_connection_routes_a_browser_request` now
   asserts the server echoes `Message::Close`.
3. **WQ-DL-01/02 cancel attribution**: `production.rs::download_v2` used one
   `"archive download cancelled"` string for both an executor-token trip
   (settings-change rebuild counts) and a worker `Cancelled` event, so triage
   could not separate them. Split into `"interrupted by executor
   cancellation"` (our shutdown) vs `"cancelled by worker"` (worker decided).

### Linux verification on the merged + fixed tree (new-tree evidence only)

- Environment: Linux WSL2 Ubuntu 26.04.1, Rust 1.98.0, Node v26.7.0.
- `cargo test -p xarchive-desktop --lib`: 251/251 PASS (incl. 13/13
  websocket_transport with the new close-echo assertion).
- `cargo test -p xarchive-protocol --lib`: 28/28 PASS.
- `desktop` Node suite: 215/215 PASS (incl. new save-refresh wiring test).
- `node scripts/docs-audit.mjs`: PASS; `git diff --check`: PASS.
- Windows-native build/package/GUI/DPI/keyboard, Full workspace regression,
  installers/releases, real external services: NOT_RUN (incremental scope;
  old-tree results not promoted). `44e60e3`'s 6 extra module tests are
  `#[cfg(windows)]`-gated native fixtures and cannot run on Linux; the 251
  vs 257 delta is expected, not a regression.

### Windows work required (all BLOCKED/NOT_RUN items, none claimed PASS)

- Rebuild Full from this handoff revision; revalidate WQ-FULL-STATUS-01 with
  the new refresh behavior (dedicated browser profile, exact rebuilt Full;
  connect, wait >30s, compare browser/sidebar/WebSocket panel; change
  settings with no active job; verify port + refreshed UI).
- Re-run the controlled WebSocket close probe against the fixed transport;
  confirm close completes within 3s.
- Reproduce WQ-DL-01/02 per mode with the split cancel messages; record job
  ID, mode, worker stderr/event code, cancellation source, final state.
- Remaining: isolated Registry/PAC/WPAD actual-egress M13 (WQ-PROXY-15/16,
  incl. loopback implicit-bypass risk); full DPI/keyboard/busy matrix
  (M10/P6, WQ-PROXY-11/17, WQ-DL-06); fresh Full/real-download/browser/
  Telegram acceptance; Credential Manager ignored test; MSI/NSIS/signing
  NOT_APPLICABLE (portable Full only). Release remains blocked by the
  user-observed FAILs until repaired + revalidated.

---

## Full 2b98952 user manual return — 2026-10-06

[Exact manual results and diagnosis](../validation/windows-full-2b98952-feedback.md): user reports first automatic browser connection/task creation PASS scoped; re-detect/sidebar/current-port consistency FAIL; gallery-dl and aria2 download completion FAIL user-observed. Running Full executable hash verified. Restart/reconnect and cause-isolated download reproduction remain NOT_RUN; existing historical and scoped build results preserved.

Next Owner **Cross-platform Owner** for status-source semantics and configuration-change/executor/transport lifecycle policy (`CROSS_PLATFORM_CHANGE_REQUIRED`, narrow repairs may instead require review). Windows retains native reproduction and exact-build revalidation. State `WINDOWS_VERIFICATION_PENDING`; release blocked by these failures. Documentation input `a104000`; build source `2b98952`, implementation `44e60e3`, Linux handoff `5c3efb79`, return branch `codex/windows-validation-5c3efb79`; handoff revision is the commit containing this entry. Tracked tree clean after commit; local untracked/user data preserved.


## Full package continuation — 2026-10-06

State: `WINDOWS_VERIFICATION_PENDING`. Current Owner Windows Platform Owner; next **Cross-platform Owner** for Git integration/shared WebSocket close review. Source/build/validation `2b98952aeaa717899ebcd13c09f058a28e998e73`; implementation `44e60e369b5c57d3ed66b46fb610dc906a45780f`; return branch `codex/windows-validation-5c3efb79`. Documentation handoff is the commit containing this entry; tracked tree clean after it, local untracked data/artifacts preserved.

Canonical fresh Full portable ZIP built, SHA `06ab9d7973a413f2bbd28d7c81dd83d24daa2e77384f751714c7a4f7affa1e11`; dev candidate, not release or MSI/NSIS. Rust 512 PASS/1 ignored, Node 214+52 PASS, Python 61 PASS; package/ZIP integrity and real bundled component/startup/Sidecar/bootstrap/auth/current-size GUI/download keyboard/restart subsets passed. Controlled WebSocket close-within-3s FAIL retained; investigate shared close/drain behavior under `CROSS_PLATFORM_REVIEW_REQUIRED`, no shared code changed. Real archive/browser automatic reconnect/Telegram/PAC/DPI and remaining lifecycle acceptance NOT_RUN with prerequisites in [exact Full results](../validation/windows-full-2b98952-results.md). [Install/manual continuation](../validation/windows-full-2b98952-manual.md). Windows retains manual/native acceptance after reconciliation.

---

The previous return below identifies the implementation ancestry; this Full continuation is the active artifact state.

## Windows 5c3efb79 return — 2026-10-06

State: `WINDOWS_VERIFICATION_PENDING`. Current Owner: Windows Platform Owner.
Next Owner: **Cross-platform Owner** for Git integration/review; Windows retains native acceptance.

- Input branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Cross-platform input/handoff: `5c3efb79d2c7ffe3c84fe1f33821cd94b6c77a49` (includes `344ca10`, reviewed `8f7fde6` / `8e0169a`). Fetch preceded checkout; no direct sync.
- Return branch: `codex/windows-validation-5c3efb79`.
- Windows implementation/validation: `44e60e369b5c57d3ed66b46fb610dc906a45780f`.
- Documentation handoff: the commit containing this entry / return branch HEAD. Tracked tree clean after commit; existing local untracked dependencies/data/artifacts preserved.

Windows-only adapter now distinguishes configured/failed PAC and WPAD from loaded scripts, applies scheme-specific environment precedence and uses explicit WinHTTP PAC execution. Shared predicate/contracts/dependencies unchanged.

Results: Desktop module 257 PASS / 1 ignored; native WinHTTP PAC fixtures and client/child boundaries PASS scoped; fresh debug native build/startup/Settings/proxy summary PASS scoped. Initial module FAIL retained with Python prerequisite and loopback assertion details. WinHTTP implicit loopback DIRECT despite throwing PAC remains an integration risk; fixture PASS does not close WQ-PROXY-15/16.

Fresh executable SHA-256: `660388A7F828E771B8B8B851A238177F3C6CA6210A9CF7852D5616278C4174E8` (debug, not Full).

[Exact results, capability and manual queue](../validation/windows-5c3efb79-results.md).
Current queue: [windows-queue.md](../validation/windows-queue.md).
History: [windows-validation-history.md](../validation/windows-validation-history.md).

Cross-platform follow-up: integrate/review Windows adapter; decide/document implicit-bypass acceptance. A shared contract change, if needed, requires `CROSS_PLATFORM_CHANGE_REQUIRED`; no shared change occurred here.
Windows follow-up: isolated Registry/PAC/WPAD actual-egress M13; remaining DPI/keyboard/busy matrix; fresh Full/real-download/browser/external-service acceptance. All remain NOT_RUN/pending with explicit prerequisites in the batch record. No whole-row Windows PASS or WINDOWS_BLOCKING claim.

Requested cross-platform-validation §16 is absent (navigation-only); used linked ownership/validation rules and git-platform-handoff §16 revision template.

---

Earlier checkpoints below are historical; only the current batch above defines active ownership/status.

## Windows 130affaf validation return

State: `CROSS_PLATFORM_REVIEW_REQUIRED`; full platform acceptance remains `WINDOWS_VERIFICATION_PENDING`.
Input branch `cross-platform/automatic-pairing-reconcile-20261002`, exact input/handoff `130affafdff512934d7c135b3eb61a2344d27b10`.
Return branch `codex/windows-validation-130affaf`. Shared fixture repair `8f7fde61b4fd2a16c9f96444990cf8fcb57302a2`; shared Toggle Enter repair/final build source `8e0169a83313eec98b57a793d71254689c4fcc1d`. Return revision is the documentation commit containing this entry.
Current Owner at execution/return: Windows Platform Owner; next Owner: Cross-platform Owner for Git reconciliation and review. Windows retains remaining native/Full/manual acceptance. Tracked tree clean before Git alignment; untracked local Windows data preserved; no direct sync.

Native proxy 37/37, core proxy 27/27, Python protocol/entrypoint 27/27 and supervisor 11/11 PASS. Rust protocol at input FAIL 27/28: shared JSONL loop still used a POSIX path; narrow fixture instantiation repair -> 28/28 PASS. Frontend targeted 79/79 at input, 80/80 after Enter repair. Fresh Windows build/startup and independent packaged worker five-capability/download_started probe PASS scoped; Full package/actual download completion NOT_RUN.

New download page navigation, Windows aria2 control presentation and Space persistence PASS on initial 8f7fde6 artifact; Enter FAIL against P6. 8e0169a final artifact retains true after restart and Enter switches/persists false PASS scoped. Remaining Settings navigation/GUI checks BLOCKED / COMPUTER_USE_UNAVAILABLE after two activation failures; full DPI/busy/keyboard/real aria2 acceptance remains open.
Final Desktop SHA256 `3A8CC8AFD3EEA82259F001B62754E01FB97B4C950BDFE7F745AABCE86D77EC46`; worker SHA256 `2098750E0F61A8BD1122FDD1CB22DE8CF5EA4EAE8BD378ED1A54C99E9CB15993`. Detailed commands, identities, original failures, environment and manual queue: [batch record](../validation/windows-130affaf-results.md), local evidence `validation-artifacts/windows-130affaf/`.

CROSS_PLATFORM_REVIEW_REQUIRED: integrate/review both small shared repairs. Native PAC static finding: adapter tests config.pac.is_some(), but official failed download yields pac=None while retaining pac_url/ErrorDownload, so the false boolean defeats the new fail-closed guard; WPAD states/evaluation distinction need adapter-level review. Actual failed-PAC/egress test remains NOT_RUN; do not promote helper PASS to runtime acceptance. Prior timeout/ACL/reconnect/advisory items remain open at original identities. Next Windows checks use reviewed source, controlled isolated PAC/WPAD fixtures and P6/M recipes.

---


## Download settings page and proxy UI continuation — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`, on top of handoff
`fa46a53`. Working tree is committed; there is no uncommitted state. Current
Owner: Cross-platform Owner. Next Owner: **Windows Platform Owner**.

### Cross-platform implementation

- Completed the settings render repair and UI polish carried in the working tree:
  proxy diagnosis form now uses the shared form wrapper, long proxy values wrap
  safely, explanatory status is not presented as a successful route, backend
  coverage clauses are separated, and the Bootstrap panel has local spacing.
- Added the dedicated `内容下载` page and sidebar navigation. The existing
  `use_aria2` setting and Tauri command remain the source of truth; aria2 manager
  controls are shown only on Windows, while other platforms receive an explicit
  explanation. Settings retains the Sidecar gallery-dl path controls.
- Added regressions for page render/platform branches and sidebar/prop wiring.
  `icon.jsx` was not changed: the proposed Extension path was not geometrically
  verified and must not be inferred correct.

### Linux verification (this batch, on the merged and committed tree)

- Desktop targeted render/wiring/state tests: 79/79 PASS.
- Desktop Node suite: 213/213 PASS.
- Desktop Vite production build: PASS; existing mixed static/dynamic Tauri API
  chunk warning remains informational.
- `cargo test -p xarchive-desktop --lib`: 251/251 PASS.
- `node scripts/docs-audit.mjs` and `git diff --check`: PASS.
- Windows-native build, packaged artifact validation, WebView2 GUI/DPI/keyboard,
  persistence across Windows restart, and real aria2/process behavior: **NOT_RUN**.

Environment: Linux WSL2 (Ubuntu), Linux Rust target `x86_64-unknown-linux-gnu`;
Windows PowerShell interop is present, but no Windows Tauri artifact or project
WDIO/Tauri driver process was available to bind to this batch's source. This does
not establish Windows product failure. No Windows execution is claimed PASS. Follow
`WQ-DL-01..08`; WQ-DL-06 and its reproducible steps in
[`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md)
now explicitly cover navigation, aria2 platform UI, DPI, keyboard and persistence.

### Next owner / handoff

Windows Platform Owner: build from the committed source SHA of this handoff and
execute the Windows queue, beginning with WQ-DL-06 UI/navigation acceptance and
the applicable runtime rows. No `WINDOWS_BLOCKING` is identified; remaining
checks are deferred Windows-owned validation and do not block independent
cross-platform work.

---

## Bypass wildcard parity and non-Windows completion — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`,
on top of the reconciled 6cf4cc8 handoff `0fec592`. Current Owner: Cross-platform Owner.
Next Owner: **Windows Platform Owner**.

One further product defect was found and fixed while completing the Plan's
non-Windows scope. `ProxyBypass::matches` documented `*.domain` support, but
`matches_host` only stripped a leading dot, so `*.example.com` degraded to a
literal comparison and **silently matched nothing**: a user who typed the
wildcard spelling into `NO_PROXY` or a bypass list got traffic proxied that they
believed was excluded, with no error anywhere. The two spellings are now
distinct rather than normalized — `.domain` covers the apex and its subdomains,
`*.domain` covers subdomains only — and the doc comment was corrected, since it
also claimed a bare hostname matches its subdomains, which the code never did.

The Plan's remaining completion criteria were audited rather than assumed. Both
validators use the host's own `is_absolute`; credential redaction is applied on
every proxy-bearing surface and aria2 logs no proxy value; `Manual` reaches the
child boundary through the `Set` path and `Direct` through `ClearProxyEnvironment`.

Linux validation: `cargo test --workspace` (21 targets, xarchive-core proxy 27),
workspace all-target Clippy with `-D warnings`, `cargo fmt --check`, the Sidecar
pytest suite, the Desktop Node suite, the Vite build, the docs audit and
`git diff --check` all PASS. The wildcard tests are mutation-checked: disabling
the `*.` branch fails them.

`WQ-PROXY-18` and manual step M15 cover the wildcard behavior, which needs a real
Windows bypass list. Nothing in this entry was validated on Windows.

## Cross-platform reconciliation of the 6cf4cc8 return — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
Input/handoff `6cf4cc8`; reconciled return `d201deb2f69974025285b8068797ba31b2c714dd` from
`codex/windows-validation-6cf4cc8`, merged through Git. This entry is the handoff revision;
Windows must build from the exact SHA of the commit containing it.
Current Owner: Cross-platform Owner. Next Owner: **Windows Platform Owner** for native
runtime, manual and Full acceptance. Platform acceptance remains
`WINDOWS_VERIFICATION_PENDING`; nothing in the returned record was upgraded.

Both returned repairs are integrated and reviewed:

- `dc5ef1f` — the Windows adapter compile repair. Accepted: these are the compile
  errors Linux could not detect, and the Windows evidence is the only source for
  them. The module still cannot be compiled on this host.
- `297a237` — the shared Settings render repair. Accepted, and it fixes a **real defect
  introduced by the proxy batch**: `proxySystem`, `proxyDiagnoseUrl` and
  `setProxyDiagnoseUrl` were never passed from `main.jsx`, so the page could not render
  the resolver summary or hold the diagnosis URL. Its React render regression test is
  kept. This also shows the Linux gap that let it through: only pure `ui-state.js`
  helpers were covered, never an actual render.

Follow-ups closed here:

- **Platform-valid protocol fixtures** (`CROSS_PLATFORM_REVIEW_REQUIRED`). Both validators
  use the host's own `is_absolute`; the hard-coded POSIX `/tmp/job-1` was the fixture's
  defect, not the validator's. Rust and Python now derive a host-absolute staging
  directory. Verified directly that `/tmp/job-1` is *not* absolute under
  `PureWindowsPath` while the new fixture is, and that a relative path is still rejected
  on both flavours. No validator relaxed. A positive "host-absolute is accepted"
  assertion was added, because a validator rejecting everything would have satisfied every
  original assertion.
- **Fail-closed PAC fall-through** (`CROSS_PLATFORM_CHANGE_REQUIRED`). The official
  resolver falls through to `DIRECT` when a script cannot be obtained or evaluated; that
  is indistinguishable from a deliberate `DIRECT` and would send corporate traffic out
  unproxied. The adapter now consults the PAC state and refuses that case. The rule is a
  platform-independent function so it is Linux-testable, and an unrecognized future state
  name is refused rather than assumed safe. **This is the one behavior change in the
  batch and it is shared, not Windows-only.**
- **Capability copy.** `system_proxy_supported()` no longer hard-codes `false` and its
  note no longer claims the native resolver is still to come. The settings test asserts
  the platform invariant instead of the stale literal.
- **WQ-PROXY-08 wording.** It demanded per-URL child routing while `WQ-PROXY-12`
  deliberately refuses a PAC child; it now scopes to static/environment/manual and
  cross-references the refusal.

Linux validation for this reconciliation: `cargo test --workspace` (21 targets ok,
Desktop 251, protocol 28), workspace all-target Clippy with `-D warnings`, `cargo fmt
--check`, the Sidecar pytest suite (61), the Desktop Node suite (209), the Vite build,
the documentation audit and `git diff --check` all PASS. The PAC refusal rule was
mutation-checked: reverting the allow-list fails its tests.

Not performed and not claimed: any Windows check. `WQ-PROXY-01`..`17`, the protocol
fixture rerun and M0..M14 all remain `WINDOWS_VERIFICATION_PENDING`, including the
controlled failing-PAC run the new rule needs. Prior extraction-timeout, confirmation
ACL and reconnect items stay open at their original artifact identities.

## Windows 6cf4cc8 validation return — 2026-10-05

State: `CROSS_PLATFORM_REVIEW_REQUIRED`; platform acceptance remains `WINDOWS_VERIFICATION_PENDING`.
Branch: `codex/windows-validation-6cf4cc8`. Input/handoff `6cf4cc8da25e0925c555cba2aaeb7667ea353414`.
Windows implementation repair `dc5ef1f2503dbbcadcdae5bec6bb169480e2e25b`; shared UI repair/validation build source `297a237aa1f91339c63870f93b1bb2e350b9c2a0`.
Return revision: Git documentation commit containing this entry. Current Owner at return: Windows Platform Owner; next Owner: Cross-platform Owner for Git integration/review and shared fixture/policy/copy follow-up. Windows retains native/manual acceptance.
Tracked tree clean before alignment; existing untracked Windows data retained. Git fetch and exact checkout completed; no direct sync. Old optional-download uncommitted wording below is superseded by committed 3f9cd1f in this handoff.

Input native compile FAIL (three Windows adapter compile errors), repaired on Windows. Native proxy 33/33, core proxy 24/24, production integrity 6/6, supervisor 11/11 after explicit Python path PASS. Rust protocol 27/28 and Python protocol 20/21: Linux absolute-path fixtures FAIL on native Windows; no validators weakened. Frontend targeted 75/75 PASS after Settings missing proxy props repair. Actual React test reproduces proxySystem ReferenceError before fix and passes afterward. Fresh native build and scoped Settings/expanded proxy GUI PASS at 297a237; full DPI/keyboard/persistence NOT_RUN. Fresh independent packaged worker five-capability/download_started PASS scoped; actual download completion/Full package NOT_RUN.

Desktop SHA256 `995930EE5B49345D2CBBC2C5CF28825EE1154F854E166E06595886F85909D56A`. Full evidence, earlier artifact hashes, tooling failures, commands and manual prerequisites: [batch result](../validation/windows-6cf4cc8-results.md). Local evidence `validation-artifacts/windows-6cf4cc8/`.

Cross-platform follow-up: review shared UI repair, make protocol fixtures platform-valid; review PAC/WPAD failure fall-through DIRECT against fail-closed policy; reconcile legacy PAC unsupported copy and per-URL child queue wording with deliberate PAC child refusal. Static policy concern is not a runtime PAC FAIL. Prior archive timeout/confirm ACL/reconnect/advisories remain open at their original identities. No full-regression, actual proxy-route, real-download or Telegram acceptance claimed. Manual continuation: current proxy M recipes and download queue, with fresh reviewed Full artifact, controlled proxy/VM fixtures and authorized external services.

---


## System proxy Batch B — ordered per-URL resolution and the Windows OS resolver — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
Source revision `f57c4b2` is the previous handoff; this batch is the commit
`fbe3116b4fc1e64233a2185ae3ef2d1e8da60f1e` on top of `3f9cd1f`. Owner:
**Cross-platform Owner (Linux)**; the next owner is the **Windows Platform
Owner**. Windows must build from this exact commit.

### Scope

- `crates/xarchive-core/src/proxy.rs` gains the ordered contract:
  `ProxyCandidate` (`Http` / `Socks` / `Direct`), `ProxySource`, and
  `ProxyResolution` with a `reason` that keeps a failure cause instead of
  degrading it to a generic message. An empty candidate list is a policy
  failure, never a direct route. `ProxyBypass` implements the usual `no_proxy`
  matching rules.
- `desktop/src-tauri/src/system_proxy_resolver.rs` (new, `#[cfg(windows)]`) wraps
  `microsoft/os-proxy-resolver`, pinned to commit `796b027c9361bb407f2a8d9d79c56b2dc4a42ee2`
  because the crate is **not published on crates.io**. A backend-less build
  (`default-features = false`) is used so PAC/WPAD evaluation is delegated to
  WinHTTP and no embedded PAC engine or `unsafe` code enters the build.
- The environment resolver now honors `NO_PROXY` and separates the HTTP and HTTPS
  variables per URL instead of taking the first variable for every destination.
- `ProxyHttpClient` caches per candidate and per resolver generation, so a system
  proxy change invalidates cached clients. `candidates_for` returns the ordered
  list; the aria2 release download walks it because that GET is verified against
  the release digest. Telegram takes the first candidate only, because retrying a
  send through another proxy could duplicate it.
- The settings page reports the resolver, the PAC state, the static and bypass
  values, and the ordered candidate list, all redacted, and accepts a target URL
  for route diagnosis.
- `reqwest` gained the `socks` feature so a PAC result naming SOCKS is usable
  rather than a reason to refuse the policy.
- **Child-process coverage is now enforced, not assumed.** gallery-dl and aria2
  read proxy environment variables once, so they cannot evaluate a per-URL PAC or
  WPAD policy. Before the Sidecar starts, the executor asks the resolver which
  policy is active: an environment or static system policy is handed over as an
  inherited environment, while a PAC/WPAD policy (or an unresolved one) is
  **refused with `PROXY_POLICY_NOT_APPLICABLE`**. Without this the child would
  have silently connected direct, bypassing the policy. The settings page and the
  route diagnostic both report the answer as `child_coverage`.

### Validation performed here

`cargo test --workspace`, workspace all-target Clippy with `-D warnings`,
`cargo fmt --all --check`, the Desktop Node suite, the Desktop Vite build, the
documentation audit, and `git diff --check` all pass. See the commit message for
the counts.

### Explicitly not validated here

The Windows adapter **was not compiled**: only `x86_64-unknown-linux-gnu` is
installed, and the official resolver cannot build on a non-Windows target without
a PAC engine. The PAC/WPAD behaviour, the per-URL routes of the real transports,
the DPI and keyboard matrix, and the credential-leak sweep are all
`WINDOWS_VERIFICATION_PENDING`; see the Batch B table in
`docs/validation/windows-queue.md` and the manual steps.

## Optional download mode (`use_aria2`) — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`,
based on `16beade`. This batch is uncommitted in the Linux working tree; no commit
has been created yet, so the branch head is unchanged. Owner: **Cross-platform
Owner (Linux)**; after the commit and push the next owner is the **Windows Platform
Owner**.

### Scope

`use_aria2` becomes an optional configuration flag on `DownloadConfig` (default
`false`). When it is off, gallery-dl performs the media byte download itself and
aria2 never starts; when it is on, the existing extraction → `MediaTransferPlan` →
aria2 path is used unchanged. The two backends are never mixed within one Job.

This is a shared cross-platform protocol change (`CROSS_PLATFORM_CHANGE_REQUIRED`
applies to the contract; Windows retains native runtime validation):

- v2 `SidecarV2Command` gained an optional `staging_dir` field that only the new
  `download` command may carry, and `validate` requires it to be a non-empty
  absolute path; any other command carrying it is rejected.
- New `DownloadStarted` / `DownloadCompleted` events and the
  `DownloadedMediaFile` / `DownloadResult` payloads. `download_completed` must
  carry `download`; the payload may not ride on any other event.
- `download_media` is now a **required** capability (5 total). A worker that does
  not advertise it fails the handshake; there is no fallback path.
- Rust, Python `worker_v2`/`extraction`/`protocol_v2`, both JSON Schemas, the
  shared fixtures, and the supervisor handshake fixture were updated together.

### Rust behavior

`execute_v2_archive` routes on the flag: `true` → `execute_aria2_archive`
(unchanged), `false` → `execute_gallery_dl_archive`. The latter sends `download`,
waits for `download_completed`, cross-checks the tweet identity, and then
`verify_downloaded_files` re-anchors every reported relative path under the job
staging directory, rejects non-regular files, and compares the on-disk length to
the reported size before producing `DownloadFile`s. An extraction that lists media
but produced no file is an error rather than a metadata-only archive. When the flag
is off, `runtime::executor_config` resolves `aria2_program` to `None`, so the aria2
process path is unreachable.

### Python behavior

`build_download_command` runs gallery-dl for real (`--directory`, deterministic
`{num:>02}.{extension}`, `--write-info-json`) and defensively refuses
`--skip-download`/`--dump-json`/`--resolve-json`/`--simulate`.
`DownloadRunner` empties the staging directory first, runs the child with the
existing cancellation/timeout handling, recovers the extraction result from the
`info.json` gallery-dl wrote, pairs sorted media files with the extracted media
indexes, removes the metadata before reporting, and validates the result before
emission.

### Frontend

`AppStatus` exposes `use_aria2`; a new `set_use_aria2` command persists the choice
and rebuilds the executor configuration. The Settings page gained a
"媒体传输方式" section with a toggle.

### Linux validation (all executed on this tree)

- `cargo test --workspace`: 21 test targets, 0 failures (desktop 234,
  `xarchive-protocol` 28).
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 issues.
- `cargo fmt --all -- --check`: clean.
- Sidecar `pytest`: 61/61 (includes new `download` command validation, download
  result validation, command-shape, runner, and worker-event tests).
- Sidecar `compileall`: clean.
- Desktop Node tests: 203/203. Extension Node tests: 52/52.
- Desktop `vite build`: success. `scripts/docs-audit.mjs`: PASS.
- `git diff --check`: clean.

### Windows-only work (not performed here)

Eight queue rows `WQ-DL-01..WQ-DL-08` were added to
[`../validation/windows-queue.md`](../validation/windows-queue.md) with manual
steps in [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md).
They cover the packaged-worker handshake, a real gallery-dl download and archive,
proving aria2 does not start, cancel/timeout process-tree behavior, Windows
staging containment and reparse protection, Settings rendering/persistence across
DPI, the aria2 path regression, and restart recovery. **None may be marked PASS from
Linux evidence, from the command shape, or from the unit/fixture tests.**

### Next steps for the Windows Platform Owner

1. Commit and push this batch; build fresh artifacts from that exact commit.
2. Execute `WQ-DL-01` first: a stale packaged worker without `download_media` now
   fails the handshake, so verify the shipped worker advertises all five
   capabilities before attempting a download.
3. Run `WQ-DL-02`/`WQ-DL-03` together, then the remaining rows in order.
4. Record redacted worker logs only; keep cookies and tokens out of shared evidence.

---

## Cross-platform reconciliation and handoff — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
Reconciled Windows return: `0aa8d14be1f3d4d3e5aac0886def850e9536f1b7`, based on
the prior UI source/handoff `ca25e5379302431a3130436896735b8be2bb4744`. This handoff
commit records Linux reconciliation and validation; no uncommitted state is included.
Current Owner after push: **Windows Platform Owner**. No shared production code was
changed in this reconciliation.

### Reconciled results and triage

- The return is documentation-only. Windows evidence is scoped: targeted UI tests
  47/47 and a fresh native `--no-bundle` build passed; current-size/maximized visual
  inspection, Sidecar Space/Enter, Extension navigation and refresh independence
  passed. It does **not** pass the full DPI, narrow-window, focus/hidden-Tab,
  busy/error or backend-effect matrix.
- The separate fresh Full artifact has package inventory/component-startup PASS
  only. On it, automatic browser connection, task creation, Telegram authentication
  and explicit Channel test receipt were reported PASS; the real archive attempt
  failed with `DOWNLOAD_TIMEOUT`, and a confirmation dialog reported ACL errors
  with action-unknown. No completed archive is evidenced.
- The Owner reports standalone use of the same packaged gallery-dl binary succeeded
  for the same URL and produced a readable image. This narrows investigation to
  invocation context, but does not prove whether arguments, working directory,
  environment/configuration or child-process handling caused the timeout. No
  task-scoped gallery stderr/progress was provided. Do not infer a shared defect or
  modify production behavior without independent reproduction.
- Automatic browser pairing and a real archive attempt in this Full package are
  **not the same acceptance** as the UI build. Keep the scoped PASS, archive FAIL,
  and queue-level Telegram delivery state distinct; in particular, an explicit
  Channel test receipt does not prove automatic archive/media delivery.
- Reconnect after Desktop restart required manual Extension “Reconnect”; manual
  reconnect passed, automatic recovery was observed failed without a measured
  recovery deadline. A stale displayed/actual port mismatch was observed separately
  and does not establish concurrent corruption.
- Route shared extraction diagnostics and confirmation call/capability review under
  `CROSS_PLATFORM_REVIEW_REQUIRED`; Windows Owner retains native reproduction and
  platform-specific queue execution. No `CROSS_PLATFORM_CHANGE_REQUIRED` is
  established.

### Linux reconciliation

The Windows return merge is documentation-only and preserves the original evidence
and artifact identities. Linux targeted UI regression, Desktop build, docs audit,
and diff hygiene are run on the reconciled tree and recorded with the final commit.
No Windows GUI/build/runtime result is claimed by this Linux batch.

### Next Windows handoff

Use this exact handoff commit and build fresh artifacts from it. Priority follow-up:
capture task-scoped redacted extractor arguments/stderr/progress and compare the
standalone versus application invocation; reproduce the confirmation-dialog ACL
failure and inspect the relevant capability path; verify successful archive/output
integrity, duplicate handling, automatic post-restart reconnect with timestamps,
and automatic Telegram archive/media delivery where authorized prerequisites exist.
Continue the remaining 100/125/150/200% DPI, narrow/wide, keyboard/focus/hidden-Tab,
busy/error/backend-effect settings matrix. Do not expose cookies, tokens, or other
credentials in shared evidence. Exact instructions and queue IDs are in
[`windows-full-ca25e53-manual.md`](../validation/windows-full-ca25e53-manual.md),
[`windows-manual-steps.md`](../validation/windows-manual-steps.md), and
[`windows-queue.md`](../validation/windows-queue.md).

---

## Windows ca25e53 visual validation return — 2026-10-05

State: WINDOWS_VERIFICATION_PENDING. Branch: codex/windows-validation-ca25e53.
Input/handoff/validation source: ca25e5379302431a3130436896735b8be2bb4744;
Cross-platform source: 4f119ca76694d4ec1063435d88bd33d66d9fae00.
Return revision: the Git documentation commit containing this entry.
Current Owner: Windows Platform Owner; next Owner: Windows Platform Owner for
remaining GUI acceptance; Cross-platform Owner reconciles this Git result and
continues existing advisory work. No production code or shared contract changed.
Tracked tree clean before checkout; local untracked dependencies/data retained.

Fresh Full follow-up: dist-portable/XArchive-full-validation-ca25e53-20261005; Desktop SHA256 37344cb7777152af1baf76eec85480e432e43ceaf44baae8ee31f8c598fe1b5e. Package inventory and component startup PASS scoped, real integrations NOT_RUN. See [Full manual recipe](../validation/windows-full-ca25e53-manual.md). This distinct Full build does not inherit the earlier GUI artifact PASS.

Scope: Targeted presentation tests and fresh native Desktop build/rendering.
PASS: node --test desktop/test/ui-wiring.test.mjs desktop/test/telegram-render.test.mjs
47/47; node desktop/scripts/build-tauri.mjs --no-bundle exit 0, including Vite
production build. Existing mixed static/dynamic Tauri import warning and MSVC
linker informational warnings retained. Full regression not run: only React/CSS
presentation/copy changed; Sidecar, shared Rust, lockfiles and workflows unchanged.

Artifact: target/release/xarchive-desktop.exe; SHA-256 d9457efe8154f165650d1bd18a49844aed9d7d19a37f8d6d6e12350980e67ccc.
Native environment: Windows 10.0.29680 x64, Node 24.19.0, Cargo 1.98.0;
local dev release channel. This is a fresh --no-bundle executable, not a Full package.
Evidence: validation-artifacts/windows-ca25e53/targeted-tests.log, native-build.log,
evidence.json, gui-disclosure-observation.txt, extension-refresh.png (local),
and inline Computer Use screenshots.

Scoped GUI PASS: current 1082x790 and maximized 1280x752 logical window captures;
concise collapsed summaries, shared Bootstrap/Telegram header spacing and
monochrome icon treatment, Extension puzzle icon; Sidecar and Extension expanded
status omit duplicate icons; detailed Sidecar/Telegram help remains in expanded
content; mouse expansion/collapse and Sidecar Space expand/Enter collapse;
Extension sidebar opens/expands/scrolls to matching panel; refresh leaves it open.
SQLite navigation also observed. Multiple Telegram/storage/Extension panels stayed
open independently. No credential/settings save or real message send performed.
DPI was not independently queried/changed (Owner previously reported 200%); these
window captures do not prove the full 100/125/150/200% matrix or a narrow breakpoint.

NOT_RUN/manual: full DPI/narrow-width matrix, all-panel Enter/Space and hidden Tab
order, focus visibility/navigation focus, fake-token masking/busy/error states,
and independent backend-effect checks for folding. Prior older-artifact GUI results
are not promoted to this changed presentation. Automation reported document-level
focus only, so rendered toggle behavior does not prove full focus acceptance.
Tooling: initial UIA Extension click landed on SQLite after maximize; first
coordinate attempt selected a 13x13 auxiliary screenshot and was rejected.
Recovered using the returned 1280x752 main screenshot; intended Extension jump
and refresh then succeeded. No product defect established or acceptance weakened.

Inherited remaining queue: cancel/shutdown fake-descendant subtree cleanup,
isolated upload=false release transfer/rehearsal and default-branch CodeQL, real
Telegram/Channel/archive acceptance. Deferred as unrelated to this presentation
change and existing Owner local/CI scope. Their prior states/evidence are retained.
No new CROSS_PLATFORM_CHANGE_REQUIRED or CROSS_PLATFORM_REVIEW_REQUIRED.
Cross-platform follow-up: reconcile this documentation return through Git and
continue unresolved Linux glib/WDIO advisory work; no shared implementation fix
requested by this batch.


---

## Cross-platform reconciliation — Windows return 52d8bc5 (2026-10-05)

Reconciled `origin/codex/windows-validation-aa8dabd` at
`52d8bc5a6c996d986c43e90cc574f81b8edf767c` into
`cross-platform/automatic-pairing-reconcile-20261002`. Input handoff:
`30bc96d1c5f69e0337c5928547ead131a6ea5aee` (containing
`aa8dabda5be8050c68ff472eb1090fca160a9082`); Windows implementation:
`39f54e5011b626827ee38b3ab06fadaf316b51b6`. Integration commit: `b937b5b`.
The Windows branch shared the earlier `aa8dabd` parent and did not contain the
later `30bc96d` handoff wording correction; merge preserved both histories.
No shared production implementation/contract change was made and no
`CROSS_PLATFORM_CHANGE_REQUIRED` is open.

### Reconciled Windows results (bounded to exact evidence)

- Release workflow hardening in `39f54e5`: trusted digests gate external asset
  execution/extraction; build/validation jobs have read-only contents and
  publication has a separate write-scoped job; inventory and hashes are rechecked
  before publishing, with no clobber. Isolated Windows Actions run 37255273960
  and native asset checks are recorded in
  [Windows history](../validation/windows-validation-history.md). This is not
  full production release acceptance.
- Sidecar packaged checks cover bounded burst/backpressure, matching cancel, full
  shutdown, EOF and worker exit PASS. Fake-descendant startup/subtree reclamation
  remains NOT_RUN because the fixture marker did not appear; no product defect
  was established.
- Settings GUI evidence is scoped PASS only for current-size mouse disclosure/form,
  hidden collapsed form, and SQLite/Sidecar navigation. Full DPI/keyboard/focus/
  Extension/busy-error/backend-effect matrix remains NOT_RUN.
- Windows native builds and module tests passed on exact `aa8dabd` implementation
  artifacts. Linux did not re-run or claim Windows evidence. Exact artifact hashes
  and test/tool details remain in Windows validation history.

### Remaining queue and ownership

Current Owner at Windows return: Windows Platform Owner. The return is now
integrated by the Cross-platform Owner; current Owner for this handoff is Windows
Platform Owner. Continue `WQ-SEC-SIDECAR-QUEUE-01`
subtree-cleanup fixture; `WQ-SEC-RELEASE-PERMISSIONS-01` isolated production-like
`upload=false` artifact transfer/rehearsal and default-branch CodeQL rescan;
`WQ-SETTINGS-PANELS-01` remaining DPI/keyboard/accessibility/side-effect acceptance.
Real Telegram/channel and archive acceptance remain NOT_RUN per recorded scope.
Linux advisory follow-up for `glib` and WDIO dependencies remains open and is not
resolved by this return. See [Windows queue](../validation/windows-queue.md) and
[manual steps §N](../validation/windows-manual-steps.md#n-current-continuation-follow-up--2026-10-05).

Linux reconciliation checks on the merged tree: Sidecar protocol tests 15/15
PASS; supervisor tests 11/11 PASS; Desktop Node tests 199/199 PASS after updating
the release-workflow wiring assertion; Desktop Vite build PASS (existing mixed
static/dynamic Tauri API chunk warning); docs audit PASS; `git diff --check` PASS.
Windows-only build, GUI, process-tree behavior and production Actions rehearsal
were not run on Linux.

Handoff revision: tracked in Git history on `cross-platform/automatic-pairing-reconcile-20261002`; pushed with a clean working tree. Current Owner after push: Windows Platform Owner, starting from the branch tip.

---

## Windows security implementation and native follow-up — 2026-10-05

This continuation supersedes the earlier aa8dabd pending implementation summary.
Branch: codex/windows-validation-aa8dabd. Input return: 3c77a1341bca3ffe7cd51b3b21622f6081b68e8d.
Implementation: 39f54e5011b626827ee38b3ab06fadaf316b51b6. Return/handoff revision:
the documentation commit containing this entry. State: WINDOWS_VERIFICATION_PENDING.
Current Owner at Windows return: Windows Platform Owner. This return is now
integrated by the Cross-platform Owner; Windows Owner performs remaining platform
acceptance. No shared
production code or contract changed, and no new shared-defect marker is asserted.
Tracked worktree clean at start; existing untracked local data preserved.

Implementation: windows-release.yml now defaults to contents:read; checkout does
not persist credentials; the build reads only the public Extension ID variable;
publication is a separate contents:write job requiring successful build. All
seven verified files are copied into a flat staging directory before upload;
publication rechecks source/tag, inventory, sizes, manifest and SHA256SUMS. Existing
asset names fail before upload; clobber was removed. Dispatch upload defaults to
false. Existing tag/source parity and independent non-blocking WDIO remain intact.
No Release was created, overwritten, or dispatched during this continuation.

External gallery-dl 2026.09.20 and aria2 1.37.0 are pinned in
 desktop/scripts/windows-external-assets.json. gallery digest was independently
read from the official upstream GitHub Release API; aria digest retains the
existing reviewed Windows aria2.rs pin. Verification rejects missing/invalid pins,
missing/empty files and altered bytes before execution/extraction. Pin changes
require independent source review; downloaded bytes never establish their own pin.

PASS: local release/security module tests 9/9; all 31 workflow PowerShell scripts
parsed; both real upstream downloads matched fixed pins; after verification,
gallery-dl --version returned 1.32.13:2026.09.20 and aria2 verified extraction and
--version returned 1.37.0. Isolated read-only Windows Actions PASS:
https://github.com/15699122/Tw2Tg/actions/runs/37255273960 , exact implementation
39f54e5, including negative integrity/policy tests and real asset downloads.
Full production build-to-publish transfer/rehearsal and default-branch CodeQL
rescan remain NOT_RUN; this isolated run is not production release acceptance.

Packaged Sidecar controls PASS on the unchanged aa8dabd worker: 256 metadata
commands (16 KiB each), producer blocked in all cases; matching extraction cancel
CANCELLED / exit 0 in 0.359s; full shutdown INTERRUPTED / exit 0 in 0.313s;
EOF / exit 0 in 1.782s, without synthesizing a shutdown event. Source queue remains
32, read count 33 while busy. EOF fixture marker PID 92060 was observed exited.
Cancel/shutdown fake descendant startup/whole-tree reclamation is NOT_RUN:
marker was not observed before controls; worker exit does not prove subtree cleanup.
Evidence/probes: validation-artifacts/windows-aa8dabd/queue-probe.json and
queue_probe_controls.py; no network/media accessed. Earlier probe timeout included
a fixture shutdown command missing required job_id; fixed, not a product defect.
Original pre-control child-marker waits and independent source diagnosis remain
recorded as fixture limitations, not a shared implementation failure.

Scoped native GUI PASS on unchanged aa8dabd Desktop hash
CA02189CFAB3CBEED5E017010A85B5DB6D2CD895BD91BEC208BEE7281E49A8FA:
current-size collapsed panels; mouse Telegram expand/collapse and form layout;
unconfigured credential checks disabled; hidden form absent from accessibility
text after collapse; SQLite opens storage and Sidecar opens its own section.
Computer Use produced actual snapshots. Current scaling was Owner-reported 200%,
not independently measured or changed. Keyboard focus automation did not establish
web control focus, so keyboard/hidden Tab acceptance remains NOT_RUN. Other DPI,
narrow/wide matrix, Extension jump/refresh independence, masking with entered fake
credential, busy/error and backend-side-effect checks remain NOT_RUN. No settings
or credentials saved; no Telegram send. Worker SHA-256 remains
11CFE182B2C1389F5813466D7B69747AC940F61C8E237EDA2F7016749D3902BE.

Owner explicitly selected local/CI scope for this continuation; real Telegram and
Channel validation remain in the manual queue. Cross-platform follow-up: integrate
Windows-owned workflow/tooling and review evidence; continue existing Linux advisory
work. Windows follow-up: production upload=false rehearsal/CodeQL evidence, fake
child reclamation and the remaining settings GUI matrix. Existing WQ-SEC-PERMS-01
closure and old icon/keyboard artifact results are preserved at their original identity.


## Windows aa8dabd validation return — 2026-10-05

State: WINDOWS_VERIFICATION_PENDING. Current/next Owner: Windows Platform Owner.
Branch: codex/windows-validation-aa8dabd. Input/handoff/validation revision:
aa8dabda5be8050c68ff472eb1090fca160a9082; Sidecar implementation:
2da530c2165125649ca47d4996908464dd32a696; settings implementation:
94fbfd403f8053d5e5584001b7e242e5321ec456. Exact target fetched from origin
and checked out through Git. Return revision is the commit containing this entry.
No production implementation changes in this return. Tracked tree was clean at
checkout; Windows-local untracked dependencies/artifacts were preserved. Tauri
build produced only a non-semantic Cargo.toml line-ending change, restored before commit.

PASS on native Windows 10.0.29680.0, Python 3.12.14, Rust/Cargo 1.98,
Node 24.19: protocol pytest 15/15; supervisor cargo tests 11/11;
Desktop Node tests 197/197 (elevated rerun); fresh PyInstaller worker build;
fresh Tauri optimized --no-bundle build. Independent instrumented queue probe:
capacity/queued 32, reader consumed 33 while extraction blocked, clean EOF after
release. This source probe is not packaged-process acceptance.

Worker SHA-256: 11CFE182B2C1389F5813466D7B69747AC940F61C8E237EDA2F7016749D3902BE.
Desktop SHA-256: CA02189CFAB3CBEED5E017010A85B5DB6D2CD895BD91BEC208BEE7281E49A8FA.
Desktop path: target/release/xarchive-desktop.exe. Worker path:
validation-artifacts/windows-aa8dabd/worker-dist/xarchive-downloader/xarchive-downloader.exe.
Logs and local probes: validation-artifacts/windows-aa8dabd/ (local artifacts,
not committed). Commands: pytest sidecar/tests/test_protocol_v2.py;
cargo test -p xarchive-sidecar-supervisor --locked;
node --test desktop/test/*.test.mjs; PyInstaller sidecar/pyinstaller/xarchive-downloader.spec;
node desktop/scripts/build-tauri.mjs --no-bundle.

Tooling limitations: initial sandbox Node killTree attempt was denied; elevated
rerun passed all 197. Packaged queue probe emitted extraction_started but its
controlled fake child marker never appeared (3-second attempts and a bounded
10-second reset-environment retry). Packaged cancel/full shutdown/EOF/child-exit
acceptance is NOT_RUN: fixture startup blocker, no product defect established.
Later shell channel lost valid cwd/FileSystem access; node filesystem/process
channel remained available and was used for reading evidence and Git write-back.

Scoped static review FAIL against release-hardening criteria: windows-release.yml
runs gallery-dl before trusted digest verification and expands aria2 before such
verification; workflow contents:write is inherited by build and publication is
in the build job. WQ-SEC-RELEASE-ASSET-01 and WQ-SEC-RELEASE-PERMISSIONS-01 remain
implementation pending; isolated Actions and negative digest tests NOT_RUN.
No release dispatched. WQ-SEC-PERMS-01 remains closed.

New settings native GUI/DPI/accessibility matrix NOT_RUN on this artifact;
previous Full-package screenshots/keyboard results do not close this changed UI.
Real Telegram/channel and real archive/duplicate tests remain NOT_RUN; bot/channel
creation reported by Owner is not credential/service acceptance.

Next Windows Owner: repair/independently verify fake fixture, complete packaged
queue cases; implement trusted asset pins and separate read-only build/publish
jobs, then isolated Actions; execute new settings matrix. Cross-platform follow-up:
review this shared queue source evidence and retain unresolved Linux glib/WDIO
advisory work; no new shared-contract defect or shared-code change claimed.



---

## Security remediation — Linux batch handed off (2026-10-05)

State: `READY_FOR_WINDOWS` for Windows-owned follow-up. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Source revision before this
handoff commit:
`2da530c2165125649ca47d4996908464dd32a696`. This follow-up state correction is
committed and pushed as `aa8dabda5be8050c68ff472eb1090fca160a9082`; working tree
is clean. Current Owner: Windows Platform Owner.

Linux shared change: `sidecar/src/xarchive_downloader/worker_v2.py` now bounds
the stdin command queue to 32, backpressures the reader and stops it when the EOF
sentinel is drained. Regression coverage is in
`sidecar/tests/test_protocol_v2.py`. No protocol schema/version change. This is a
small shared implementation change preserving the abstraction:
`CROSS_PLATFORM_REVIEW_REQUIRED`.

Linux checks: initial focused regression reproduced that EOF drained all 128
metadata events but did not emit a protocol `shutdown` event. EOF is distinct
from an explicit shutdown command, so the regression now asserts complete drain
and clean EOF termination without expecting a shutdown event. Final focused
regression 1/1, `test_protocol_v2.py` 15/15, and complete `sidecar/tests` 55/55
PASS. The earlier `55/55` draft claim is superseded by these rerun results.
`cargo test -p xarchive-sidecar-supervisor --locked` 11/11; `cargo fmt --all --
--check`, `git diff --check`, and docs audit PASS. Network-isolated pytest attempt
was a tooling failure because bubblewrap omitted a writable `/tmp`; no product
conclusion was drawn from it. `cargo audit --no-fetch` used the local advisory DB
and is not a fresh online audit. Online `npm audit` on this worktree reported 20
HIGH entries including the existing `extract-zip` path and additional current
dependency findings; no Node dependencies were changed. See the security plan for
scope and caveats.

Windows work: execute `WQ-SEC-SIDECAR-QUEUE-01` against the exact pushed SHA and
follow manual steps §M.3 for bounded stdin burst/backpressure, cancel, shutdown
while full, EOF and process exit. `WQ-SEC-RELEASE-ASSET-01` and
`WQ-SEC-RELEASE-PERMISSIONS-01` require Windows Owner implementation/review and
isolated Actions validation; steps are in §M.1–M.2. These are `NOT_RUN` here, not
product failures or PASS. Existing `WQ-SEC-PERMS-01` is already closed; do not
reopen it. No release job or Windows workflow was modified in this Linux batch.

Open advisory results: `glib 0.18.5` remains Linux-target reachable through GTK3;
Windows target excludes it. No compatible locked upgrade was found. `extract-zip
2.0.1` remains in the WDIO tree with no patch indicated by the current npm audit;
do not infer GitHub alert state from local audit. Linux optimized release/GUI and
Windows runtime/Actions checks remain pending. Details in
`docs/development/security-remediation-plan.md`.

## Desktop settings visual batch — 2026-10-04

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`. Source revision: `c8f63c91ab0f8915e9a61713e2af65d98d7ab5df`; implementation revision: `94fbfd403f8053d5e5584001b7e242e5321ec456`; final corrected handoff revision: recorded below. Uncommitted state: none at handoff. Current/next Owner: Windows Platform Owner.

Implementing the screenshot-requested settings panels, Telegram presentation/icon, corrected Sidecar JSONL explanation and Extension footer spacing. Shared scope: `desktop/src/main.jsx`, `desktop/src/pages/settings-page.jsx`, `desktop/src/components/telegram-settings.jsx`, new `desktop/src/components/settings-section.jsx`, shared icon/button/CSS, and Desktop tests. No Rust, persistence, secret-store, Telegram command or protocol behavior is changed.

The initial resumption run found `desktop/test/ui-wiring.test.mjs` failed to parse; that stale assertion/syntax issue has been fixed. Current Linux validation: Desktop Node tests 197/197 PASS; `npm run check` PASS with the existing mixed static/dynamic `@tauri-apps/api/core.js` chunk warning; documentation audit PASS (96 Markdown files; zero dead links/unindexed docs); `git diff --check` clean. Windows validation is `NOT_RUN` in this Linux environment and is required on the exact handoff build: WebView2 rendered layout at 100/125/150/200% DPI and narrow/wide sizes; keyboard disclosure/focus and hidden-control tab order; SQLite/Sidecar/Extension navigation; Extension separator gap; Telegram form, credential masking, busy/error states; refresh action independence; and no backend effect from folding. UI evidence only; unrelated `WQ-TG-*` external-service acceptance remains governed by the existing queue. The Windows Platform Owner should start from the final correction handoff revision recorded below (implementation: `94fbfd403f8053d5e5584001b7e242e5321ec456`).

## Revision correction — 2026-10-04

The settings implementation is `94fbfd403f8053d5e5584001b7e242e5321ec456`. The earlier handoff documentation commit `fba84c070100403de6669208cc76c3206a491ccc` contained an incorrect implementation SHA reference. The correction is committed and pushed as `4a858eaae8185ff967f1ea4d7bd0faa230d0d7cf`; use `4a858eaae8185ff967f1ea4d7bd0faa230d0d7cf` as the handoff revision for checkout.

## Cross-platform reconciliation of the Windows f57c4b2 return — 2026-10-04

State: `WINDOWS_VERIFICATION_PENDING` after reconcile; next handoff state
`READY_FOR_WINDOWS`. Current and next Owner: **Windows Platform Owner**.
Reviewed incoming branch `codex/windows-validation-f57c4b2`, target
`3734f87c32dd3ab9eaabfec76f55cbdd16503019` (Windows implementation/build
`b5dd8c95f2685e8d91adf2bb9c24f7d7caf50012`) against the formal handoff `f57c4b2`
and source `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff`. Reconciled into the
Cross-platform branch as merge `8e87e19`; this record's own commit is the next
handoff revision and supersedes the Windows local return section below.

### Reconciled result

- Integration was clean and stayed inside the Windows boundary. No shared crate,
  storage migration, schema or cross-platform contract changed: `crates/` and the
  shared Desktop Telegram modules are byte-identical to the handoff source.
- The native provider is a Windows-only module behind
  `#[cfg(windows)]`, depends on a pinned Windows-only `keyring = "=3.6.3"`
  (`windows-native`, explicit `WinCredential`), sanitizes native errors, maps
  `NoEntry` to absence, and installs through the existing production provider
  seam at startup. There is no default, mock or plaintext fallback. This matches
  the shared contract, so this is canonical integration, not
  `CROSS_PLATFORM_CHANGE_REQUIRED`.
- The Windows-only dependency and its licence obligation were registered in
  `docs/references/external-sources.md` and `THIRD_PARTY_NOTICES.md`. Reviewed and
  accepted: MIT selected, redistribution notice recorded, final release
  SBOM/notice inclusion still open.
- Product versus tooling classification is correct. Schannel fetch failure,
  incremental-cache finalization access denial, denied OS CIM query and linker
  informational warnings are recorded as tooling/environment conditions, and no
  assertion or product behavior was weakened. I found no product defect.
- Windows evidence is honestly bounded and is **not** promoted: native synthetic
  credential CRUD, module/mock Telegram and storage tests, UI tests/build, a
  native development-channel build and a scoped read-only settings smoke are
  `PASS`. Real sends, large files, crash recovery, proxy integration,
  WQ-TG-008 GUI acceptance, `WQ-TG-UNI-*`, the Scheme B integrated-source
  remainder and Full packaging remain `NOT_RUN`/`PLANNED`. Earlier pairing
  failures are not closed by this return.

### Linux validation on the reconciled tree

Run by the Cross-platform Owner after the merge, not carried over from the
pre-merge tree: `cargo build --workspace` PASS; `cargo test --workspace` PASS
(Desktop 231, Storage 75, Telegram 54, plus other workspace suites, 0 failures);
strict workspace all-target Clippy PASS; Node Desktop 195 PASS; docs audit PASS
(96 tracked Markdown); `git diff --check` clean. `cargo tree` confirms `keyring`
is not linked on Linux. Logs: `/tmp/tg-recon-build.log`,
`/tmp/tg-recon-test.log`, `/tmp/tg-recon-clippy.log`, `/tmp/tg-recon-node.log`,
`/tmp/tg-recon-docs.log`. Full regression was not escalated: the change set is a
Windows-only adapter plus documentation and the merged shared tree is
byte-identical to the tree that already passed the workspace suite.

### Next Windows work and blocking prerequisites

1. `WQ-TG-001` full acceptance — isolated-account GUI credential rotation,
   restart, presence-only projection and config/SQLite/log/diagnostic-export
   leakage matrix. This is the immediate next executable step and needs only
   Windows capability.
2. User-supplied prerequisites before any real-service queue can run: an isolated
   test bot token, a controlled chat/topic, and `api_id`/`api_hash` for Local Bot
   API Server deployment (`WQ-TG-007`, currently `PLANNED`). Until these exist,
   `WQ-TG-002`–`006`, `009` and `WQ-TG-UNI-*` cannot be executed by anyone.
3. `WQ-TG-008` full GUI matrix and the Scheme B integrated-source acceptance
   remain deferred to a subsequent Windows batch.

No cross-platform defect is open, so nothing blocks Windows from continuing, and
no `CROSS_PLATFORM_CHANGE_REQUIRED` or `CROSS_PLATFORM_REVIEW_REQUIRED` is
outstanding. Reuse of the Windows `PASS` results requires the native adapter,
pinned dependency, runtime setup and account capabilities to be unchanged;
otherwise record `REVALIDATION_REQUIRED`.

## Windows local return — 2026-10-04 / f57c4b2

State: `WINDOWS_VERIFICATION_PENDING`. Current and next Owner: **Windows Platform Owner**.
Source `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff`; incoming documentation handoff
`f57c4b2090f02df3ba55c58a660e2eee143a9660`; Windows implementation/tested build
`b5dd8c95f2685e8d91adf2bb9c24f7d7caf50012` on `codex/windows-validation-f57c4b2`.
Return revision is the Git commit containing this section. No tracked uncommitted
state is required; pre-existing local caches, tools and validation artifacts remain untracked.
This current return supersedes the READY_FOR_WINDOWS state below.

Windows-native Credential Manager adapter and production provider injection are
implemented. Synthetic native CRUD, sanitized errors, Telegram/storage modules,
Desktop Telegram tests, UI tests/build and native build pass. Computer Use opened
the rebuilt application and settings showed credential service available, token
not saved and sender stopped. This is a scoped smoke, not real-send acceptance.
See [history](../validation/windows-validation-history.md#2026-10-04--telegram-windows-local-return-f57c4b2).

User confirmed no prepared bot/chat/api_id/api_hash and requested local validation
only. Local Bot API deployment remains PLANNED; real sends, large files, recovery,
proxy integration and Unigram receiving acceptance remain NOT_RUN. An open Unigram
window was detected, but no version/account/receiver matrix was executed.
Scheme B integrated-source acceptance is NOT_RUN this round: the minimum scope
selected was Telegram credential integration; earlier pairing FAIL/NOT_RUN is not closed.

Cross-platform follow-up: no shared contract change or product defect found.
Cross-platform Owner should reconcile the Windows commit through Git and review
the Windows-only pinned dependency/license record; this is canonical integration,
not `CROSS_PLATFORM_CHANGE_REQUIRED`. Next executable work is Windows GUI credential
presence/restart/leakage checks, then deployment and real-service queue after the
user prepares an isolated bot and target. Full regression/package publication was
not run: no release is being produced and the scoped tests cover this native adapter.

## Formal handoff to Windows Platform Owner — 2026-10-04

State: `READY_FOR_WINDOWS`. Ownership transfers from Cross-platform Owner to
Windows Platform Owner for this batch. This supersedes every
`CROSS_PLATFORM_IN_PROGRESS` checkpoint below; they remain as the development
history that produced this handoff, not as the current state.

| Field | Value |
|---|---|
| Batch | Telegram TG-06 shared integration, Scheme B remainder, plus the Windows Telegram acceptance queue |
| Branch | `cross-platform/automatic-pairing-reconcile-20261002` |
| Handoff source commit | `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff` |
| Handoff commit | the commit containing this table; read its exact SHA from Git history |
| Parent baseline | `f4ec469f0ca3d657b9fe1b954a224bdb433e210d` |
| Uncommitted state | none — the Linux working tree was clean at handoff, and no uncommitted state is required |
| Previous owner | Cross-platform Owner (shared contracts, storage, transport, Desktop business logic, shared docs) |
| Current owner | **Windows Platform Owner** |

### Cross-platform work completed

- Completion-time Telegram configuration sampling in the archive commit path,
  with fail-closed behaviour on missing or invalid configuration.
- Immutable archive-intent capture, a durable journal, and idempotent
  ARCHIVED-to-outbox materialization with atomic new-row authorization.
- Credential rotation decisions and claim-generation fencing
  (migrations `0012_telegram_rotation_decisions.sql` and
  `0013_telegram_claim_generation.sql`), verified bot identity, and exact-row
  resume authorization for both `automatic` and `confirm` policy.
- Cooperative sender worker with claim heartbeat, startup recovery, and
  start/stop/settings-restart lifecycle; it never fabricates a credential
  provider.
- Registered Tauri commands: settings projection, credential replace/delete with
  revocation before native cleanup, rotation-candidate confirmation, explicit
  reviewed `UNKNOWN` resend, task/job state projection with live upload stages,
  bounded cancellation and sender stop.
- Explicit never-retried Cloud/Local endpoint-migration control path plus a
  durable `migration_pending` latch that only that command clears.
- Shared test coverage including concurrent materialization, restart-then-single
  send, permanent-failure isolation from the local archive, provider lifecycle,
  and mutation-checked bounded storage-error handling.

### Changed shared modules

`crates/xarchive-telegram`, `crates/xarchive-storage` (including migrations),
`desktop/src-tauri` (`archive`, `commands`, `config`, `executor`, `runtime`,
`telegram_send`, new `telegram_worker`, new `telegram_control`), the React
settings/task surfaces, and shared documentation.

### Windows work required before dependent checks can run

1. `WQ-TG-001` — implement the Windows Credential Manager adapter and inject it
   through the provider seam. It must be the only production `SecretStore`; no
   plaintext or in-memory fallback may be introduced.
2. `WQ-TG-007` — deploy a version-pinned Local Bot API Server bound to loopback.

### Windows validation required

Run the queue in [`../validation/windows-queue.md`](../validation/windows-queue.md)
and manual steps §K in
[`windows-manual-steps.md`](../validation/windows-manual-steps.md). Start from
the exact source SHA above: fetch, verify a clean Windows working tree, check
out that revision, and record Windows build/architecture/tool versions plus the
rebuilt artifact SHA-256. Linux workspace, Clippy, Node and Sidecar results are
shared evidence only and are not Windows PASS. `WQ-TG-002`–`006`, `008`, `009`
and `WQ-TG-UNI-01`–`08` remain `NOT_RUN` until executed.

### Known risks and expected behavior

- `UNKNOWN` is never resent automatically; re-send requires a deliberate
  reviewed action and may produce a duplicate.
- After an endpoint-mode migration, sending stays disabled until explicitly
  enabled. A failed or unknown migration outcome keeps it paused; do not blindly
  retry.
- Loopback endpoints always connect direct, never through a configured proxy;
  cloud endpoints honour the configured proxy.
- A confirmed send means only that the Bot API answered. It never implies the
  client received, displayed or read the message.

### Deferred GUI and manual items

Keyboard traversal, 100/125/150% DPI, narrow-window layout, cancel/retry/
`UNKNOWN` review interactions, Credential Manager presence checks, and the
Unigram receiving-side matrix. See manual steps §K for the numbered procedures.

### Cross-platform follow-up

None outstanding for this batch. Any shared-contract, schema or cross-platform
behavior finding on Windows returns to the Cross-platform Owner as
`CROSS_PLATFORM_CHANGE_REQUIRED`; a small fix that preserves an existing
abstraction uses `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Documentation reconciliation and proxy-policy regression — 2026-10-04

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`. Parent revision
  `f4ec469f0ca3d657b9fe1b954a224bdb433e210d`. This increment is a development
  checkpoint, not a formal handoff, release approval or ownership transfer.
- Closed the Linux/cross-platform work items that remained after `f4ec469`. The
  shared Telegram production path was already complete; what remained was
  documentation that still described it as missing, which would have made the
  Windows Owner misread `IMPLEMENTATION_NOT_READY` blockers as absent features.
- Reconciled stale claims against the code: `status.md` capability matrix and
  Telegram capability line, `architecture/overview.md` Native Host and Telegram
  entries (Windows Named Pipe server, ACL and browser install wiring are
  implemented; only real Windows registration and browser acceptance are
  outstanding), and the Telegram Plan continuation plus round-boundary sections.
  The round-boundary text is preserved as the state at that point, with an
  explicit note that its four items were subsequently delivered.
- `windows-queue.md` TG table now carries an explicit Implementation column.
  `WQ-TG-002`–`006`, `008`, `009` are `IMPLEMENTED`; `WQ-TG-001` and `WQ-TG-007`
  are shared-`IMPLEMENTED` with the Windows-specific adapter/server deployment
  still `PLANNED`. Every TG row remains `NOT_RUN`, but the deferral reason is now
  the missing Windows/real-environment capability instead of "no production
  entry point". `WQ-TG-UNI-*` rows are unchanged.
- Renamed manual steps §K from "no production send entry point" to "shared
  production path ready, awaiting Windows/real-environment acceptance", updated
  its prerequisites to name only the two genuinely Windows-blocked items, and
  repointed the queue's §K anchor at the new heading.
- Reviewed the outstanding migration proxy question and found the behavior
  correct rather than defective: `with_api_endpoint` pins `no_proxy` for
  `EndpointMode::Local` and keeps the resolved proxy for cloud, so a Cloud→Local
  migration keeps the proxy on the cloud leg and bypasses it on the loopback leg.
  Added `migration_source_and_target_apply_endpoint_specific_proxy_policy` to lock
  that contract in, including that proxy credentials never reach `Debug` output.
- Heartbeat coverage was re-checked rather than assumed missing: accelerated-tick
  renewal and heartbeat-loss tests already exist in `telegram_send.rs`. The Plan's
  "accelerated heartbeat/fault tests and bounded storage-error retry remain
  pending" wording is now stale and was corrected.
- Added the two missing storage-error tests. `heartbeat_storage_error_stops_the_
  attempt_and_is_not_retried` removes the outbox table so the renewal fails with
  a real SQL error, and requires the driver to end on the first failure rather
  than retry the renewal. `an_unusable_file_cache_stops_the_row_without_claiming_
  it_sent` removes the `file_id` cache table and requires the media row to be
  deferred with no request reaching the Bot API.
- Both new tests were mutation-checked rather than trusted: making the renewal
  return `Poll::Pending` instead of the store error fails the first test by
  timeout, and downgrading the cache read to `unwrap_or(None)` fails the second
  test with the raw SQL error. An earlier draft of the second test asserted a
  batch-level abort after a confirmed send; that premise was wrong, because the
  unreachable cache stops the row before the request. The test was rewritten to
  assert the behavior the code actually and correctly guarantees, instead of
  changing product behavior to fit the test.
- No product code changed in this increment; both tests lock in existing
  behavior.
- Final combined results on this tree: `cargo test --workspace` PASS (Desktop
  231, Telegram 54, plus other workspace suites); workspace all-target Clippy
  with `-D warnings` PASS. Logs: `/tmp/tg-final4-workspace.log`,
  `/tmp/tg-final4-clippy.log`.
- Windows Credential Manager, native runtime/packaging, packaged Local Bot API
  deployment, GUI acceptance, controlled real-account send and Unigram acceptance
  remain `NOT_RUN` and are Windows Platform Owner work. No Windows PASS is
  claimed. No shared Telegram implementation or test scope is known to remain;
  the next step is a formal handoff naming an exact SHA.

## Shared-development closure checkpoint — 2026-10-04

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`. Branch
  `cross-platform/automatic-pairing-reconcile-20261002`; parent revision
  `48312827a898e3854cb4ff17371987d9a16ed705`. This increment is a development
  checkpoint, not a formal handoff, release approval or ownership transfer. Its
  exact SHA is the commit carrying this section; read it from Git history rather
  than from this text.
- Closed the remaining non-Windows TG-06 scope: an explicit never-retried
  Cloud/Local endpoint-migration control path (`telegram_control.rs`), a durable
  `migration_pending` latch that only that command clears, and live upload-stage
  projection on every send path.
- The migration flow persists the pause before any control request, requires
  explicit confirmation, uses `logOut` for a Cloud source and `deleteWebhook`
  then `close` for a Local source, verifies the target bot identity, and leaves
  sending paused on any failure or unknown outcome. It cannot be bypassed by an
  ordinary settings save or by sender startup.
- Real defect found and fixed while testing: text-only sends reported no progress
  stage, so the task projection was silently empty for text archives. The text
  path now reports queued/awaiting/confirmed. Byte-level `Uploading` progress is
  still only emitted when a body is actually streamed.
- Tests added: migration latch blocking automatic sending and the batch runner;
  the latch surviving a config round trip; the settings projection exposing the
  pause without secret material; reported stage ordering; frontend command wiring
  per surface, and confirmation-gated migration.
- Final combined results on this tree: `cargo test --workspace` PASS (Desktop
  229, Storage 75, Telegram 53); workspace all-target Clippy with `-D warnings`
  PASS; Node Desktop 195 and Extension 52 PASS; frontend build, `cargo fmt
  --check`, `git diff --check` and docs audit (96 tracked Markdown) PASS. Logs:
  `/tmp/tg-final2-workspace.log`, `/tmp/tg-final2-clippy.log`,
  `/tmp/tg-final2-node.log`, `/tmp/tg-final2-ext.log`, `/tmp/tg-final2-build.log`,
  `/tmp/tg-final2-docs.log`.
- Documentation reconciled: the Plan, the capability status paragraphs and the
  Windows queue no longer describe production enqueue, commands or UI as missing.
- Windows Credential Manager, native runtime/packaging, packaged Local Bot API
  deployment, GUI acceptance, controlled real-account send and Unigram acceptance
  remain `NOT_RUN` and are Windows Platform Owner work. No Windows PASS is
  claimed. The next step is a formal handoff naming an exact SHA.

## GitHub progress synchronization — 2026-10-04

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`.
- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source baseline: `fece4381a1bce47c8b7aaebd342b19fb15165afc`.
- Synchronization revision: the commit containing this section; obtain its exact
  SHA from Git history. This commits all retained Telegram implementation,
  migrations 0012/0013, worker, UI, tests and checkpoint documentation.
- This is a progress checkpoint, not `READY_FOR_WINDOWS`, release approval or
  ownership transfer. Shared remaining work listed below still applies.
- Earlier uncommitted-state statements describe their historical checkpoints;
  after this synchronization, verify local cleanliness and remote branch SHA.
- Validation is the completed combined-tree evidence listed below; this
  synchronization changes documentation only and claims no new Windows results.

## Goal continuation — 2026-10-04 (provider and UI integration)

- Current Owner: Cross-platform Owner; state `CROSS_PLATFORM_IN_PROGRESS`.
  Branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD
  `fece4381a1bce47c8b7aaebd342b19fb15165afc`. Changes remain uncommitted;
  this is not a formal handoff and no commit or push was performed.
- Shared provider injection, startup/settings sender restart, sanitized batch
  errors, native deletion revocation-before-cleanup, random secret references,
  credential presence and task projections are wired. React settings and job
  surfaces are mounted. Production has no MemorySecretStore/plaintext fallback;
  Windows Owner must inject the native provider.
- Sender batches reconcile completed journals and recover expired claims before
  sending. Original-file mode captures document media kinds at completion.
  Tests cover concurrent materialization, disk reopen followed by one send,
  permanent API failure preserving local archive, injected lifecycle restart,
  missing provider and native-delete failure revoking authority.
- Final completed checks: Rust workspace PASS (Desktop 220, Storage 75,
  Telegram 53); strict workspace all-target Clippy PASS; Node Desktop 193 and
  Extension 52 PASS; Sidecar 54 PASS; frontend build, format, diff check and docs
  audit PASS. Logs: `/tmp/tg-reviewed-workspace.log`,
  `/tmp/tg-reviewed-clippy.log`, `/tmp/tg-reviewed-node.log`,
  `/tmp/tg-reviewed-build.log`, `/tmp/tg-goal-sidecar.log`,
  `/tmp/tg-goal-docs.log`.
- Reviewed UNKNOWN resend is now an explicit selected-row, active-generation
  transaction gated by duplicate-risk acknowledgement; automatic claiming of
  UNKNOWN remains forbidden. A regression proves missing confirmation, stale
  generation and repeated review are rejected.
- Remaining shared work is NOT a Windows blocker: complete explicit
  Cloud/Local migration wizard, diagnostic/control-request
  fault tests, progress/reason projection, and final Plan/document reconciliation.
  Current endpoint-mode changes with active credentials are rejected rather
  than silently performing logOut. GetChat/logOut/close contract variants exist;
  no completed migration flow is claimed.
- Windows credentials, native GUI, external Local Server, real Telegram and
  Unigram acceptance remain NOT_RUN. Queue and manual steps remain the authority;
  do not promote existing IMPLEMENTATION_NOT_READY entries to PASS.

## Continuation checkpoint — 2026-10-04 (completion-time capture)

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`; NOT a formal Windows handoff. Branch remains `cross-platform/automatic-pairing-reconcile-20261002`, HEAD `fece4381a1bce47c8b7aaebd342b19fb15165afc`; all implementation remains uncommitted, including migrations 0012/0013 and `telegram_worker.rs`. No restore, commit, push or ownership transfer occurred.
- This checkpoint supersedes the receipt's production `telegram_intent: None` statement: production now samples persisted Telegram configuration after extraction/transfer completion, captures verified active bot identity and immutable archive facts, journals the intent through ArchiveService, and attempts idempotent materialization after local commit. Missing/invalid configuration fails closed. Startup recovery remains journal-only. Error-return contexts preserve the sampling source.
- Atomic materialization now grants only newly inserted rows to the currently active matching bot generation in the same transaction. Existing rows receive no grant on replay; rotation confirmation remains exact-row and UNKNOWN remains excluded. Full bot/target/key uniqueness is used for detecting existing rows.
- Registered Tauri commands: task-state projection, captured-candidate resume confirmation, bounded sender stop, non-secret settings save (server-owned revision/capability), and clean queued/retry cancellation. Saving settings stops the old worker before persisting; it does NOT restart a sender without a native provider.
- New regressions cover completion-time target/revision changes, disable/malformed/missing configuration, initial new-row authority and no reauthorization on post-rotation replay.
- Validation evidence: `/tmp/tg-final-workspace.log` (`cargo test --workspace`, successful completed run: Desktop 216, Storage 74, Telegram 53 plus other workspace tests); `/tmp/tg-combined-clippy.log` (strict all-target Clippy for Desktop/Storage/Telegram). Initial compilation errors from incorrect method signatures and initial Clippy test-module placement failure were corrected before the successful runs. Final checks are rerun after the error-context sampling-source preservation change; consult the completed logs rather than terminal integration status.
- Capability detected this continuation: WSL2 Linux x86_64, Linux-only Rust target, no Wine executable; DISPLAY/Wayland exported. No Windows artifact or native credential/real Telegram acceptance was executed. Terminal completion reporting failed intermittently; completed logs/process inspection were used instead of duplicating still-running tests.
- Remaining non-Windows scope is NOT complete: production provider injection/startup/settings restart and credential commands/presence projection, frontend settings/task integration, and expanded end-to-end crash/rotation/heartbeat tests remain. Capture failures currently fail closed without a dedicated Telegram diagnostic projection and need follow-up. Do not label this READY_FOR_WINDOWS or complete TG-06.
- Windows queue remains `NOT_RUN`; see `../validation/windows-queue.md` and manual steps §K. Native Credential Manager adapter, Windows runtime/GUI/packaging and real-service/Unigram acceptance belong to Windows Owner. No plaintext or in-memory production credential fallback was introduced.

## Cross-session receipt — 2026-10-04

This receipt supersedes stale present-tense statements below; older increments remain historical evidence, not validation of the entire current working tree. This is a local cross-session transfer, NOT a formal Windows handoff.

- Owner: Cross-platform Owner. Branch: `cross-platform/automatic-pairing-reconcile-20261002`. Committed baseline/local tracking revision: `fece4381a1bce47c8b7aaebd342b19fb15165afc`. No remote fetch was performed for this receipt.
- Uncommitted implementation includes rotation decisions (migration 0012), claim-generation fencing (0013), planner versions 1/2/3, real-time Clock, heartbeat, cooperative sender worker, RuntimeState ownership, archive metadata capture helper and executor Telegram configuration snapshots. Preserve all modified and untracked files; do not restore entire files to the baseline.
- Code truth: `archive.rs` still passes `telegram_intent: None`. Runtime has `start_telegram_sender`, but production credential-provider/startup/settings commands are not connected. Existing helpers are not a usable automatic-send product.
- Next: decide and test configuration capture timing (executor/context snapshot currently differs from the approved completion-time sampling); connect immutable intent capture, atomic materialization and exact-row authorization for new plans; then connect provider/startup/settings lifecycle, Tauri business commands and state projection. Complete crash/rotation/stop integration tests before platform delivery.
- PASS (existing local logs, not newly rerun): Desktop library 214/214 and Desktop strict all-target Clippy in `/tmp/tg-executor-snapshot.log` and `/tmp/tg-executor-snapshot-clippy.log`. Historical Storage 74/74 and Telegram 53/53 are older incremental results and require revalidation against the final combined tree.
- NOT_RUN: final combined subsystem/full-workspace regression, production archive-to-send integration, Windows/native GUI/Credential Manager, real Telegram/Local Server/Unigram acceptance. Shared implementation gaps are NOT Windows blockers.
- Tool diagnostic: terminal completion reporting intermittently fails; read completed logs and inspect processes rather than repeatedly restarting tests. Do not treat that tooling failure as a product defect.
- Windows execution remains in `../validation/windows-queue.md` and manual steps §K of `../validation/windows-manual-steps.md`. Implementation-not-ready checks remain NOT_RUN; missing platform capabilities may be BLOCKED only after detection. No Windows PASS or release approval is claimed.


Status: `CROSS_PLATFORM_IN_PROGRESS` — 2026-10-04 development checkpoint. This batch adds archive-intent recovery and atomic outbox materialization, verified bot identity, credential generations and exact-row resume grants. Runtime startup coordinates persisted intents; production capture, policy-driven authorization and sender execution remain incomplete. Windows acceptance remains `NOT_RUN`. Current Owner: Cross-platform Owner. No Windows or real-send PASS is claimed; this checkpoint is not a formal Windows handoff.

- Checkpoint branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Parent/source baseline: `ddc533658860d28b60e50e86e8e1ce4687f02e39`.
- Checkpoint revision: the Git commit containing this update; identify its exact SHA from Git history.
- Working-tree scope: all listed Telegram implementation and documentation changes are included in this checkpoint; post-commit cleanliness is verified separately during synchronization.
- Latest validation: Telegram 52/52, Storage 68/68, Desktop 197/197; relevant strict all-target Clippy, format and whitespace checks PASS. No full workspace or Windows acceptance is claimed.

## Active Telegram TG-06 resumed batch — 2026-10-04

- Post-checkpoint policy increment (uncommitted, based on fece438): candidate
  selection is an exact eligible-row snapshot for the ACTIVE generation.
  Same-bot automatic policy grants that snapshot; confirmation policy returns
  candidates without granting; different-bot identity does not resume old rows.
  Storage 68/68, Desktop 197/197 and strict all-target Clippy PASS. Production
  credential commands, durable rotation decision recovery and sender remain incomplete.

- Claim authorization increment: automatic and keyed manual claim SQL now requires
  an exact grant from the ACTIVE generation for telegram-bot:<id> identities.
  Stable-bot UNKNOWN rows cannot be claimed through the generic manual API.
  Legacy identities retain their historical behavior pending explicit migration;
  they are not converted or granted production authority here. Storage 68/68,
  Desktop 197/197 and strict all-target Clippy PASS. Policy-driven grants and
  production sender integration remain incomplete.

- Resume authorization storage: migration 0011 persists grants for exact outbox ids
  and active credential generations. Grants reject other bots, UNKNOWN and unavailable
  snapshots; invalid selections roll back all grants. New rows are not implicitly
  authorized and retired generations cannot grant authority. Storage 67/67 and strict
  storage/Desktop all-target Clippy PASS. These APIs are not yet enforced by production
  claim SQL or invoked by automatic/confirmation credential commands.

- Candidate activation coordination: verified credentials are stored under isolated
  SecretStore references before database CAS activation. Stale activation cleans up
  its inactive candidate without overwriting the active secret. Cross-connection
  stale-generation regression and disk reopen tests PASS (Storage 66/66); candidate
  coordination regression PASS (Desktop 197/197). Strict all-target Clippy PASS.
  This helper is not yet a production credential command or queue resume authorization.

- Credential persistence increment: migration 0010 stores verified bot identity and
  opaque SecretStore references with CANDIDATE/ACTIVE/RETIRED generations, never tokens.
  Transactional compare-and-set activation rejects stale verification; a partial unique
  index enforces one active generation. Disk reopen regression PASS. Storage 65/65,
  Desktop 196/196 and strict storage/Desktop all-target Clippy PASS. SecretStore/SQLite
  activation coordination and outbox resume authorization are still not connected.

- Verified identity increment: shared transport now supports non-publishing getMe;
  verification accepts only a positive numeric bot id with is_bot=true and produces
  telegram-bot:<id>, independent of token rotation. Credential replacement returns
  the verified identity after successful secret storage instead of discarding it.
  Telegram 52/52 and Desktop 196/196 PASS; strict all-target Clippy PASS.
  Durable generation activation and queue authorization remain unimplemented.

- Runtime continuation: PREPARED archive file recovery now runs before executor startup
  recovery, then ARCHIVED intents are materialized without network requests. Individual
  failures are isolated and retained for review. Desktop 196/196 PASS; strict all-target
  Clippy PASS. The media planner now splits contiguous document-only and photo/video runs
  without reordering (Telegram 51/51 and Desktop send tests 13/13 PASS). No production
  intent capture, credential authorization or sender loop is claimed by this increment.

- Latest incremental evidence: disk journal recovery covers staging and renamed final across
  database reopen (Storage 64/64). Desktop persisted-snapshot planner now uses the atomic
  journal/outbox API and preserves captured bot/chat/topic/config facts (Desktop 195/195).
  Strict storage/Desktop all-target Clippy PASS. Production wiring, authorization, scheduler,
  commands/UI and full integration remain incomplete; Windows checks are not executed.

- Goal continuation: complete-plan outbox insertion is now transactional; Desktop prepares all
  entries before insertion. Conflict rollback and replay regression PASS. Storage 63/63,
  Desktop 194/194 and strict storage/Desktop all-target Clippy PASS. Shared implementation is
  still incomplete (journal advancement, credentials/authorization, production enqueue, scheduler,
  commands/UI); no Windows validation or formal handoff occurred.

- Batch A continuation: fixed final-directory recovery after rename, prevented accidental staging creation,
  added persisted intent/job/tweet and media-integrity validation, and corrected immutable-intent comparison.
  Regression covers rename before archive database completion and damaged metadata. Batch A remains
  CROSS_PLATFORM_IN_PROGRESS; the exhaustive failure/reopen/concurrency matrix and journal identity design
  remain pending. All changes are local and uncommitted; no ownership transfer or Windows evidence.

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source commit: `ddc533658860d28b60e50e86e8e1ce4687f02e39` (branch tip at round start; all current working-tree edits remain uncommitted).
- Handoff commit: none — this is **not** a Windows handoff; status stays `CROSS_PLATFORM_IN_PROGRESS`.
- Uncommitted state: retained edits in `crates/xarchive-telegram/src/lib.rs`; `desktop/src-tauri/src/config.rs` (rotation policy plus token replacement/deletion orchestration and tests); `desktop/src-tauri/src/telegram_send.rs` (topic propagation, immutable `ArchiveSendIntent` planner input and topic regression); Telegram Plan/handoff/queue/manual updates; indexes and new `docs/user-guide/telegram.md`. These changes remain uncommitted; nothing was committed or pushed.
- Current Owner: Cross-platform Owner. PowerShell interop capability was probed; no Windows validation or platform evidence was produced.
- Audited boundary: production archive completion is in `/home/shiraishi/VSCode Workspace/Tw2Tg/desktop/src-tauri/src/archive.rs`; `queue_archive_sends()` still has no production caller. `renew_outbox_claim()` has no live sender caller; runtime has no Telegram scheduler. `RuntimeState` has no injected SecretStore or Telegram commands. `complete_sidecar_archive()` and the filesystem rename/SQLite writes are multi-step and have no tested archive/outbox reconciliation protocol. Remaining non-Windows work is itemized in the Telegram Plan's current-round boundary.
- 2026-10-04 continuation audit: migration `0009` and the PREPARED/ARCHIVED journal plus staging-recovery operation are implemented and covered by two storage tests. The database+filesystem protocol is not complete: crash-boundary injection, missing-stage/manual-review resolution, ARCHIVED-to-outbox materialization, and production callers remain pending. This continuation's final checks: Storage 62/62; Desktop library 194/194; storage+desktop strict all-target Clippy, format, diff check, docs audit PASS. Telegram suite 50/50 is prior same-day evidence and was not rerun in this storage increment.
- Implementation this round: added `TelegramConfig::replace_bot_token()` and `delete_bot_token()` over the existing SecretStore abstraction; candidate verification happens before secret mutation and the token is never included in config or UI projection. Added tests for failed verification retaining the prior token, verified replacement, and deletion. Preserved the existing topic propagation work in the sender and added an end-to-end fake-server regression asserting text posts include the persisted topic. These helpers do not implement credential generations, atomic platform-store failure guarantees, queue authorization, Tauri commands or a Windows adapter.
- Continuation: storage migration `0009_telegram_archive_intents.sql` records immutable archive/target planning facts before archive rename when an intent is supplied. `ArchiveService` can recover the PREPARED journal from staging and advance it to ARCHIVED; CAS transitions and immutable-fact conflict behavior are tested. Production archive callers still pass no Telegram intent; there is no ARCHIVED-to-outbox recovery runner, scheduler, heartbeat, or settings/send UI. Linux checks: Storage 62/62, Desktop all-target compile check, fmt and diff check PASS. This is a journal foundation only, not proof of all crash boundaries or production coordination.
- Validation in this round: WSL2/Linux x86_64; starting source `ddc533658860d28b60e50e86e8e1ce4687f02e39`; working tree remains uncommitted. PASS: `cargo test -p xarchive-telegram` 50/50; `cargo test -p xarchive-storage` 60/60; `cargo test -p xarchive-desktop --lib` 194/194; strict Clippy for the three crates; `cargo fmt --all -- --check`; `git diff --check`; docs audit (95 tracked Markdown). Node/Sidecar checks were from the retained 2026-10-04 baseline, not rerun for this incremental Rust/doc change. Full workspace regression not run. Windows native build/GUI/Credential Manager/Local Bot API/real-send/Unigram were not run; queue remains `NOT_RUN`, with manual procedures in §K. No commit/push/handoff was created.

- Formal Windows test source: `9920566ef1df115effc3ab5df5d9a12fd42210ed` (includes `5ff0a22` Windows integration).
- Handoff documentation branch: `cross-platform/automatic-pairing-reconcile-20261002`. This commit records the documentation checkpoint authorizing Windows retest of the formal source.
- Incoming source reconciled in this batch: `6060f5a2d554d32fa8400f7f5084319b19fabeb1`; Windows return `ebdc44db5ae17ef4189532d06dbb6f6a22a629ae` on `codex/windows-validation-6060f5a`.
- Earlier Windows implementation `d65a01bfe2c6f93a071322bfef154fd947b0b70b` and evidence `30fc57588065b73677473d411368219eda3313a7` are based on `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e`, not on the tested source. Integration commit `5ff0a22` carries identical `crates/` + `desktop/src-tauri/` code to d65a01b (verified by empty diff on those trees); d65a01b Windows PASS remains bound to its own source/artifact and is not claimed for the integrated source.

Exact `6060f5a` Windows evidence: targeted package/UI Node tests 27/27 PASS; Native Host release build and canonical Full build PASS; Extension directory inventory PASS scoped (13 files including helper); generated Full `installation.files` omitted the shipped helper (FAIL scoped); packaged Host returned `BOOTSTRAP_NOT_IMPLEMENTED`. Automatic pairing, UI reproduction, E2E and lifecycle were NOT_RUN / IMPLEMENTATION_NOT_READY on that source. Build success is not GUI or integration acceptance.

CROSS_PLATFORM_CHANGE_REQUIRED was addressed by integrating the Windows bootstrap through Git; Windows validation of that integration remains pending. Full installation inventory correction is committed in the formal test source above. `CROSS_PLATFORM_REVIEW_REQUIRED` for the small inventory correction has been reviewed and targeted Linux validation is complete. Shared stop-aware WebSocket source review and Linux targeted tests are complete; this is not Winsock validation. Shared UI authoritative state / automatic-options behavior remain follow-ups; preserve historical UI FAIL until reproduced on the exact Windows artifact with redacted runtime fields.

Prior Linux validation for the integrated source and this documentation reconciliation: Desktop Node workspace tests, Extension tests/check/build, Cargo format, protocol/Host tests, WebSocket targeted tests, docs audit and whitespace checks PASS (docs audit and targeted tests re-run in this batch). Full regression was not run; no Windows checks are claimed for this follow-up.

After formal handoff, Windows Owner fetches the pushed exact SHA, rebuilds an identified Full artifact, then performs registration, sidebar/detail + runtime field capture, 5s/45s idle, status query, authorized archive/duplicate/recovery, worker/restart/sleep, profiles/ACL, install/upgrade and redaction checks. This Linux session did not have Windows GUI/browser/Registry or Windows Rust-target capability: those checks are blocked here, not product failures. Manual steps are consolidated in [Windows manual steps §L](../validation/windows-manual-steps.md#l-browser-automatic-pairing--current-integrated-source-handoff-pending); queue IDs and prerequisites are in [current queue](../validation/windows-queue.md). Prior failures and automation blockers remain bound to their original artifacts.

Commands, exact-source Windows receipt and artifact hashes: [Windows validation history](../validation/windows-validation-history.md#2026-10-02--exact-6060f5a-windows-validation-return). Manual steps and current queue are linked above.

### Full manual evidence update — 2026-10-05

On the ca25e53 Full artifact above, Owner manual evidence establishes PASS for initial automatic browser connection, task creation, Telegram authentication and explicit Channel test receipt. Real download FAIL: gallery-dl extraction DOWNLOAD_TIMEOUT; confirmation dialog ACL errors also observed, triggering action unknown. Entire integration acceptance remains pending. See the latest Full manual integration record in Windows history and queue. Remaining archive/duplicate/automatic Telegram media and lifecycle checks stay NOT_RUN. Next Owner: Cross-platform Owner for CROSS_PLATFORM_REVIEW_REQUIRED extraction diagnostics and confirm authorization review; Windows Platform Owner retains native reproduction and manual acceptance. This documentation commit is the return revision; no production code changed.

### Reconnect and standalone extractor follow-up — 2026-10-05

Owner confirmed standalone gallery-dl succeeds with the same Full-bundled executable and authorized X URL, while XArchive remains timed out. This narrows but does not establish the invocation-context cause. Empty `_staging` confirms directory setup only. Owner also observed no automatic Extension reconnection after Desktop restart; clicking Reconnect restored authenticated/connected without entering a token. Timed recovery remains FAIL observed; manual reconnect PASS. Captures show stale form port versus active port in one session; investigate as a separate display issue, without assuming simultaneous-state corruption. See Windows history/queue for exact evidence. Next: Cross-platform Owner reviews extraction diagnostics and confirm ACL path; Windows Platform Owner reproduces and completes manual acceptance.
### Plan C Windows items summary — 2026-10-08 (historical validation snapshot)

**Owner at this checkpoint: Windows Platform Owner (native/manual acceptance). Cross-platform Owner retains implementation ownership.** Branch: `cross-platform/automatic-pairing-reconcile-20261002`; source baseline (uncommitted at the time): `e93ba72cb415241cc5fcd3afcb48ca8ce7dfc37e`. The results below are historical and do not validate the current dirty tree.

#### Linux validation completed (2026-10-08)

| Component | Result |
|---|---|
| `cargo test -p xarchive-storage --lib` | **103/103 PASS** |
| `cargo test -p xarchive-desktop --lib` | **268/268 PASS** |
| `cargo test -p xarchive-download --lib` | **33/33 PASS** |
| Sidecar tests | **63/63 PASS** |
| Desktop Node tests | **220/220 PASS** |
| Extension Node tests | **52/52 PASS** |
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS |
| `node scripts/docs-audit.mjs` | PASS |
| Full workspace regression | Not run (D0/C1/D1/F partial) |

#### Windows-specific queue items

| Queue ID | Target | Priority / owner | Status | Deferred reason |
|---|---|---|---|---|
| `WQ-PLAN-C-D0-01` | Internal recovery facts, archive commit interruption and corrupt-record precedence | P1 / Windows Platform Owner | `NOT_RUN` | D0 Linux safety acceptance is complete; exact-source fresh Windows artifact and NTFS evidence are pending. Manual procedures: §D0 recovery in `windows-manual-steps.md`. |
| `WQ-PLAN-C-C1-01` | Filename template preview/commit equivalence, collision/traversal rejection and rename recovery | P1 / Windows Platform Owner | `BLOCKED` | Shared v2 production recovery/finalization remains incomplete; Windows path behavior additionally requires NTFS. Proceed after C1 implementation and formal Git handoff. |
| `WQ-PLAN-C-D1-01` | Independent JSON/YAML export switches, legacy TXT snapshot compatibility, and recovery when exports are disabled | P1 / Windows Platform Owner | `BLOCKED` | D1 remains intentionally inactive until v2 startup recovery and final-path consistency are proven; Windows artifact required. |
| `WQ-PLAN-C-E-01` | Per-tool downloader args via persisted spec → Sidecar/gallery-dl argv and Rust aria2 argv | P1 / Windows Platform Owner | `NOT_RUN` | Shared parser unit coverage exists; production allowlist/wiring not implemented. |
| `WQ-PLAN-C-F-01` | Theme persistence and versioned configuration exchange | P1 / Windows Platform Owner | `NOT_RUN` | Configuration/theme implementation not ready. |
| `WQ-PLAN-C-F-02` | Download/archive record export/import; individual JSON, ZIP and 7z | P1 / Windows Platform Owner | `NOT_RUN` | Record exchange implementation not ready. |
| `WQ-PLAN-C-B1B2-01` | v1/v2 execution-spec compatibility, unknown schema rejection, task reuse/retry/restart snapshot | P1 / Windows Platform Owner | `NOT_RUN` | Exact-source fresh artifact required. |

#### Manual verification steps to be executed on Windows after handoff

1. **D0 recovery (implemented; Windows NOT_RUN):** use a disposable NTFS archive root and synthetic media. Interrupt the process at each documented journal/manifest write, staging-to-final rename, and database-finalization boundary. Restart the exact artifact; verify the manifest version/hash, Tweet/Job identity, every final relative media path/size/SHA-256 and Telegram-intent reference. Repeat missing, corrupt and unknown-version records; each must fail explicitly without guessing or falling back to exports.
2. **C1 mapping (BLOCKED):** enter synthetic username/Tweet/media fixtures and compare the Rust-generated preview with committed names. Include Unicode, Windows reserved names, case-insensitive duplicates, invalid/missing template fields, extension preservation and traversal attempts. Force a commit interruption and restart; verify no overwrite and identical final paths in disk, SQLite, metadata and any Telegram payload.
3. **D1 exports (BLOCKED):** on separate fresh jobs run JSON/YAML combinations on/on, on/off, off/on and off/off. When both are enabled, compare the parsed canonical metadata DTO; preserve historical TXT snapshot semantics. Inspect archive files and confirm internal recovery remains readable and verified in every case. Interrupt and restart one exports-off job; verify no recovery dependence on user exports. Confirm an existing legacy archive is not rewritten.
4. **F configuration exchange (NOT_RUN):** export the dedicated config JSON and verify the warning banner. Import in a disposable profile; verify preview lists changed, preserved and excluded fields. Test a missing archive-directory/Sidecar path: the current local value must be preserved. Reject unknown schema and invalid values without changing the active config; verify no automatic process launch or restart.
5. **F record exchange (NOT_RUN):** separately export download and archive record files and confirm the warning banner. Import only one file and verify the UI explains which counterpart records are absent. Import both standalone files, then a ZIP and a 7z bundle containing the pair. Check preview and idempotent merge; inject path/size/hash conflict and confirm only the conflicting archive record is rejected. Verify archive files are never overwritten, no download is started, and no task becomes COMPLETE without matching verified facts. Try traversal, duplicate names, links and unexpected bundle entries; reject safely.

> Item 1 is implemented but Windows NTFS evidence is NOT_RUN. Items 2–5 are blocked or not ready and must not be marked complete until manual verification is executed on a Windows artifact built from the exact handoff source SHA.

> Authoritative queue: `docs/validation/windows-queue.md`. Windows items remain `NOT_RUN`/`BLOCKED` until manual verification is executed on a Windows artifact built from the exact handoff source SHA. No Windows PASS is claimed in this Linux session.

## Current working-tree status — 2026-10-08 (active)

- Branch: cross-platform/automatic-pairing-reconcile-20261002
- HEAD: 773fe64a9f8fe813a489fc4ddb675b32e0b9ddca
- Uncommitted files: crates/xarchive-storage/migrations/0017_archive_recovery_v2.sql; crates/xarchive-storage/src/archive_recovery.rs, file_store.rs, and lib.rs; desktop/src-tauri/src/executor.rs; docs/development/desktop-download-output-plan.md; docs/status/platform-handoff.md; docs/validation/windows-manual-steps.md and windows-queue.md.
- Resolved: `transition_archive_recovery_v2` now deserializes immutable plan data from `current.manifest_json` before reading/updating `rename_phase`; no schema or C1/D1 rejection semantics changed. `cargo test -p xarchive-storage --lib` PASS (103 passed, 0 failed); `cargo test -p xarchive-desktop --lib` PASS (268 passed, 0 failed).
- Startup dispatch now distinguishes legacy, InternalV1, InternalRenameV2 and unknown versions; v1 startup journal verification and unknown-version fail-closed are covered. Legacy jobs lacking both a spec and committed recovery facts fail as `ARCHIVE_RECOVERY_LEGACY_INCOMPLETE`, not as a new-contract missing-spec failure. Focused Linux results: Storage 106/106; Desktop startup recovery 5/5; format and diff checks PASS. This is only dispatch/classification work; v2 two-phase service replay, transactional finalization, attempt fencing and Telegram final-path updates remain incomplete. C1 remains fail-closed; do not allow D1 first. Windows checks are all `NOT_RUN`/`BLOCKED`; no Windows PASS is claimed.
- Direct Linux checks on this worktree: Download 35/35, Sidecar 63/63, Desktop Node selected 22/22. Full workspace regression NOT_RUN.
- C1 stays fail-closed; D1 gated. Windows items remain NOT_RUN/BLOCKED until manual artifacts are verified. Authoritative queue: docs/validation/windows-queue.md.

## Plan C continuation — 2026-10-08 (no accepted implementation)

- Historical checkpoint: Cross-platform Owner on branch `cross-platform/automatic-pairing-reconcile-20261002`, source baseline `773fe64a9f8fe813a489fc4ddb675b32e0b9ddca`; the worktree was uncommitted at that time. This is not the current worktree status; see the current record at the beginning of this file.
- Scope requested: complete Plan work not dependent on Windows, then consolidate Windows-only acceptance. The C1 service-recovery attempt was not accepted: `cargo check -p xarchive-storage --lib` failed on incomplete code; the attempted changes in `archive_service.rs` and `archive_recovery.rs` were reverted. All other pre-existing working-tree modifications were preserved. No commit or formal handoff was made.
- C1 shared blockers remain: transactional finalization contract and tests, PREPARED/COMMITTED v2 filesystem replay, final-directory and DB fact verification, atomic attempt fencing across service transitions, usable failpoints covering persistence/rename/commit/finalize, and startup invocation of v2 replay. Production v2 remains fail-closed. E's persisted tool arguments and actual Sidecar/aria2 argv wiring and F's theme/config/record exchange also remain shared implementation work; they are not Windows-only blockers.
- No new product validation is claimed for this continuation. Earlier Storage 106/106 and Desktop Rust 269/269 results belong to the prior working-tree check. Windows native/NTFS/GUI acceptance was not run; there is no formal handoff SHA or fresh Windows artifact. Run the consolidated manual procedures after shared implementation is complete, committed, and handed off.

## `/goal` continuation disposition — 2026-10-13

The Plan's B1/B2 statuses were reconciled with the inspected call graph; C1 B3–B7 remain shared implementation work and v2 remains fail-closed. F2/F3 were not started because the approved dependency sequence gates them on C1 and D1, and their precise shared contracts remain open. No production files changed. Documentation audit and `git diff --check` pass; full regression was not run. Windows-native work remains queued for a formal Git handoff and fresh artifact.
