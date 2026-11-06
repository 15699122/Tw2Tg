V1 Completion and Modular Refactoring Plan

Owner: Cross-platform Owner for shared implementation and Linux validation.
Status: ACTIVE — complete the approved Sidecar-focused InternalV1 scope first;
aria2-specific work is deferred by user direction. Modular refactoring follows
only after the approved phase gates are met.

## Current execution state — 2026-11-06

### Reconciled source baseline and dependency matrix

- Current checkout: branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `214d077b1f740253bb3e6de8f1d447e8ccbbd585`; 19 tracked paths modified,
  two untracked paths (`desktop/src-tauri/job-1.lock` and
  `desktop/src-tauri/src/executor.rs.bak`). Existing `stash@{0}` is preserved.
  No formal handoff exists. Prior test totals in this Plan are historical and
  are not evidence for this checkout unless reproducibly tied to this source.
- **Verified this continuation (2026-11-06):** The production no-journal
  `DOWNLOADING` successor path is wired through `SessionExecutionBoundary` and
  startup dispatch. It holds runner admission and both scopes, drains the
  predecessor through adapter evidence, performs Storage's expected-attempt /
  no-journal CAS, consumes proof for Workflow session takeover, and executes
  the successor without ordinary `begin_attempt`. Desktop successor success,
  malformed-record drain refusal, missing-containment refusal, Storage CAS
  positive/negative cases, and Workflow evidence-consuming takeover tests pass.
  Desktop `cargo check --lib` passes. The maintenance-test failure was an
  incorrect expectation: a `QUEUED` job is not a recovery candidate, so
  `fatal_error` correctly remains `None`; the test now asserts unchanged state
  and attempt. Four earlier focused Desktop regressions, two Storage CAS tests,
  and the Workflow takeover test pass. A full serial workspace run after the
  initial regressions exited 0: Desktop 364 passed / 8 ignored, Storage 176 passed,
  Workflow unit 49 passed, coordination 11, execution-session 5, recovery
  integration 3, Linux containment 7 passed / 2 ignored, plus the other package,
  binary, transfer, bootstrap, and doc-test suites recorded in
  `/tmp/tw2tg-goal-workspace-final2.log` with exit record
  `/tmp/tw2tg-goal-workspace-final2.exit` (`0`). The startup-recovery filter
  passes 11 / 1 delegated-cgroup test ignored. `cargo check -p xarchive-desktop
  --lib` passes (one existing `record_recovery_results` dead-code warning), as
  do formatting, docs audit, and diff checks. These checks do not close the
  remaining lifecycle gates below.
- **Verified follow-up (2026-11-06):** Added and passed
  `no_journal_internal_v1_successor_cas_loss_does_not_start_or_fail_successor`.
  The runner-boundary fixture reaches the no-journal predecessor-drain and
  Workflow takeover path, injects a lost successor claim at the persistence
  port, and proves no successor factory/writer starts, the predecessor remains
  `DOWNLOADING` at its original attempt, and no failure event is written. This
  is boundary behavior coverage with deterministic CAS-loss injection, not a
  real concurrent SQLite CAS race. Added and passed
  `startup_recovery_retries_successor_after_cas_owner_is_sigkilled`: a real
  subprocess is SIGKILLed after the durable successor CAS at the attempt-owned
  factory boundary; a fresh production startup-recovery runtime opens the same
  SQLite database, executes the next successor once, and a repeated recovery
  does not execute it again. This proves CAS-after-process-exit recovery and
  terminal idempotence, but it does not launch a real writer or replace the
  delegated live-descendant drain acceptance. Also added and passed
  `startup_recovery_sqlite_successor_cas_loss_fails_closed_at_runner_boundary`:
  real `StorageJobPersistence`/SQLite and the production startup boundary use a
  fixture-local trigger to make the successor UPDATE affect zero rows. Startup
  reports `Refused`, launches no execution factory, leaves the predecessor at
  `DOWNLOADING`/attempt 1, and records no failure/completion event. This proves
  the actual Storage CAS-loss result through the runner boundary, but models
  the competing durable winner with a trigger rather than a concurrently
  scheduled second connection. The port-level test remains complementary.
  These tests do not launch a real writer or replace delegated live-descendant
  drain acceptance. Audit
  result: Unix/Linux persisted scheduling and startup recovery enter the owned
  `SessionExecutionBoundary`; the legacy factory path is retained only under
  `cfg(not(unix))`, so Linux has no second generic persisted execution route to
  wire. V1-02b reachability recheck confirms production `execute_session`
  receives publication facts only after `execution.execute` returns (which
  drains/joins registered writers), then synchronously finalizes. No existing
  production-faithful control pauses a genuinely registered writer at the
  authorization point. Keep V1-02b `NOT_RUN`; do not fabricate registry state,
  add a production pause hook, or change authorization semantics. The successor
  delegated live-descendant acceptance and the remaining dependent V1 gates
  still need review;
  do not add ordinary-claim fallback or unfenced predecessor failure.
- **Other remaining Linux gates:** V1-02b registered-but-undrained writer
  refusal is **CLOSED** (2026-11-06, delegated scope; see below). V1-03 Storage
  trigger rollback/replay is covered and the production runner-boundary
  post-commit failure now runs and passes; its retry leg exposed a separate
  open defect. V1-04 has production startup PREPARED/COMMITTED recovery and
  process-crash window coverage for all four V1 publish boundaries, but the full
  phase gate remains open. V1-05 cancellation-before-PREPARED and stale-result
  fencing, and V1-06 panic/drop boundaries have focused PASS evidence rerun on
  this dirty source (2026-11-06); their complete production resource-lifetime
  matrices remain open. V1-07 active registered-writer replacement and V1-08
  Sidecar containment/publication evidence are **PASS for the Sidecar backend**
  (2026-11-06: containment, drain, descendant kill, cancellation inside a real
  domain, and publication all pass under a delegated scope; aria2 remains
  `NOT_RUN`). V1-09's serial workspace suite passed and its evidence binding is
  recorded. Delegated live-descendant successor drain now PASSES under a
  delegated scope.
- **Delegated cgroup is available — earlier blocker withdrawn — 2026-11-06:** An
  earlier entry claimed V1-02b/V1-03 were blocked because `/sys/fs/cgroup` is not
  writable. That probe only tested the cgroup root; the adapter operates below
  the invoking unit. Under `systemd-run --user --property=Delegate=yes` all
  required operations succeed (`mkdir`, `cgroup.events` = `populated 0`,
  `cgroup.kill`, `rmdir`). Previously ignored tests now pass: the production
  finalizer with a real registered writer (1/1, stable across 3 runs), the new
  V1-03 post-commit failure test (1/1), the new V1-02b refusal test (1/1),
  `startup_recovery_drains_live_descendant_after_owner_sigkill` (1/1),
  `production_factory_contained_handshake_and_drop_remove_writer_group` (1/1),
  workflow `linux_containment` (2/2), sidecar-supervisor `linux_contained` (4/4),
  and download `linux_contained` (3/3). Remaining ignored failures are tool
  absence (`aria2c`, `gallery-dl`) or the separately deferred V2 gate, never
  PASS.
- **V1-02b CLOSED:** `registered_writer_with_incomplete_drain_is_refused_by_authorization`
  acquires a real session, registers a genuine `sidecar` role via
  `WriterRegistration::register_linux` on a real recorded domain, and launches a
  live member. `authorize` returns `IncompleteDrain`; after a real
  `stop_and_drain` plus `close_admission` the same owner authorizes, proving the
  refusal came from the incomplete drain alone. No PREPARED journal and no final
  directory exist.
- **V1-03 runner boundary PASS, retry defect found:** The new
  `production_runner_post_commit_database_failure_recovers_without_false_success`
  injects a SQLite failure after `commit_attempt_staging` and proves no
  `JOB_COMPLETED`, no `Complete`, a `PREPARED` journal, and a durable media file.
  However `recover_internal_v1_on_runner` accepts only `DOWNLOADED`/`COMPLETE`
  (`executor.rs:3153`), so a `DOWNLOADING` + `PREPARED` attempt is refused on
  retry. The refusal is fail-closed but not recoverable. **Not patched**:
  widening accepted states is `CROSS_PLATFORM_CHANGE_REQUIRED` and needs review
  before it changes Windows acceptance.
- **V1-04 storage-layer crash windows closed — 2026-11-06:** Added
  `archive_recovery::tests::v1_publish_crash_windows_converge_through_production_prepared_replay`
  (Storage 177 passed vs 176 before; full serial workspace run afterwards:
  849 passed / 0 failed, exit 0, `/tmp/tw2tg-linux-v104.log` and
  `/tmp/tw2tg-linux-v104.exit`). Windows covered: `prepared` (journal durable, no rename yet), `rename`
  (`AfterFinalRename(0)`), `directory` (staging committed, database not
  converged), and `committed` (full publish, no replay needed). Each parent
  process then recovers through the real lock-protected production path
  (`ExecutionReservation::bind_existing_attempt_checked` →
  `LinuxContainment::recover_production_attempt` →
  `authorize_prepared_recovery` → `replay_prepared_internal_v1` →
  `converge_committed_internal_v1`) and proves converged media bytes,
  `COMMITTED` journal, terminal state, and idempotent repeat convergence.
  This is process-crash evidence, not a returned-error assertion, so it
  satisfies the V1-04 storage-layer evidence requirement that the earlier
  same-process test could not.
- **V1 `AfterDirectoryCommit` gap closed — 2026-11-06:** `complete_sidecar_archive_attempt_inner`
  previously consumed only `AfterFinalRename`, so the committed-directory /
  database-not-converged boundary had no V1 failpoint (V2 already had one at
  `archive_service.rs`). The V1 path now returns an injected crash after
  `FileStore::commit_attempt_staging` and before `update_tweet_metadata`, which
  is a symmetric use of the existing `ArchiveRecoveryFailpoint` enum rather
  than new product behavior. Both additions are test-side: the new
  `#[cfg(test)] complete_sidecar_archive_attempt_for_test` entry point mirrors
  the existing `publish_prepared_internal_rename_v2_for_test` and still runs
  every production attempt/contract/destination/current-attempt check.
- **V1-09 source binding complete — 2026-11-06:** Recorded a full
  validation entry in
  [`../validation/linux-crash-replay-20261016.md`](../validation/linux-crash-replay-20261016.md)
  using every required field from `../validation/validation-policy.md`. It
  binds the results to `source_sha 214d077b1f740253bb3e6de8f1d447e8ccbbd585`
  and a dirty-tree fingerprint (`git diff` sha256
  `e61dfb46a9e6c391adeda1f016c4ddf2ada35c4bd75b6f7b0b4f874d4a794893`, 19 tracked
  modifications, 2 untracked) rather than to a bare test count. Environment,
  tool versions (`rustc`/`cargo` 1.98.0, Python 3.14.4), commands, exit codes,
  and evidence paths are recorded, and the entry states explicitly which gates
  it does not close. Because the tree is uncommitted, the entry is marked
  `REVALIDATION_REQUIRED`. The V1-09 evidence-binding portion is therefore
  `PASS`; the gate as a whole still depends on the unclosed V1 gates.
- **Runner-seam dependency proof — 2026-11-06:** Source trace shows the
  unignored production runner entry
  (`executor::tests::production_runner_rejects_uncontained_internal_v1_writer_before_publish`)
  reaches the real `SessionExecutionBoundary::execute_session` →
  `PendingArchivePublication::finalize` → `PublicationOwner::authorize`.
  With no containment provisioned, `authorize` fails `IncompleteDrain` before
  `ArchiveService::publish_internal_v1_attempt`, so no PREPARED journal, staging
  `commit_attempt_staging`, or final directory exists. Conversely,
  `ProductionExecutionFactory::create_attempt_execution` only builds
  `AttemptWriterDomain`/`recover_and_create_writer_group` when
  `LinuxExecutionContainment` is `Some`; that path requires a writable delegated
  cgroup parent (`LinuxContainment::create` → `open_parent_chain`), which this
  environment does not provide. Therefore V1-03's post-`commit_attempt_staging`
  database-failure acceptance and V1-02b's registered-but-undrained refusal share
  one seam dependency: a fixture-only lifecycle control that lets a genuinely
  registered, drained writer reach the real finalizer, or an authorized
  disposable cgroup. Neither a forged permit nor an unregistered writer may
  substitute. V1-03 remains `NOT_RUN` at the runner boundary.
- **Continuation verification — 2026-11-06:** Storage archive-recovery filter
  PASS 29/29; Workflow suite PASS (unit 49, coordination 11, execution-session
  5, InternalV1 recovery integration 3, Linux containment 7/2 ignored, no-replace
  2, and doctests); Desktop `internal_v1` filter PASS 6/2 ignored. Focused
  Desktop production no-containment rejection, PREPARED/COMMITTED startup
  recovery/idempotence, SQLite-trigger successor CAS loss, and SIGKILL-after-CAS
  retry each PASS. Storage Sidecar filter PASS 5/5 and prepared-replay SQL-fault
  test PASS 1/1. Formatting, diff check, and docs audit PASS. The actual
  containment-backed production finalizer test was explicitly attempted with
  `cargo test -p xarchive-desktop --lib
  production_runner_finalizes_internal_v1_archive_after_registered_writer_drain
  -- --ignored --nocapture`; it fails before the product assertions because
  `LinuxExecutionContainment::from_launch_arguments` returns `Linux execution
  cgroup delegation unavailable` in this environment (`/sys/fs/cgroup` is not
  writable). This is an environment-blocked test, not a product failure or
  acceptance pass. Do not infer runner-level V1-03 from Storage tests.
- Current dependency matrix: production successor wiring `IMPLEMENTED`; its
  SQLite-trigger runner CAS-loss acceptance and crash-after-CAS subprocess
  recovery `IMPLEMENTED` (a concurrent-connection CAS race, live-writer crash
  recovery, and V1-02b drain-boundary acceptance remain);
  generic entry-point audit `DONE`; V1-02b reachability audit `DONE`, seam
  `REVIEW-BLOCKED`; V1-03 runner acceptance `DEPENDENCY-WAITING` on the reviewed
  fixture seam while Storage rollback/replay is covered; V1-04–06 have focused
  tests that do not close their complete acceptance targets; V1-07 and the
  production V1-08 Sidecar gate remain capability/seam-dependent; V1-09 workspace
  regression `PASS` (2026-11-06), with final source-bound reporting outstanding;
  V1-02b acceptance
  revision `DECISION-REQUIRED ONLY IF` no safe production-faithful test seam
  exists; Windows native adapter/validation `WINDOWS-OWNED`; aria2
  `USER-DEFERRED`. No current user decision blocks Linux implementation.
- **Blocked/deferred:** delegated live-descendant cgroup validation is still
  ignored/`NOT_RUN` because disposable delegation is unavailable; Windows-native
  work belongs to Windows Owner and awaits formal Git handoff; aria2-specific
  scope remains user-deferred/`NOT_RUN`. These do not block the independent
  shared successor integration.
- **Linux work is exhausted short of the seam — 2026-11-06:** Source proof
  confirms there is no reachable non-cgroup state for V1-02b: production
  containment cannot even be constructed here
  (`LinuxExecutionContainment::from_launch_arguments` capability probe at
  `executor.rs:986-990` fails with `Linux execution cgroup delegation
  unavailable`), and `ProductionWriters::mark_drained` is only reachable after
  `stop_and_drain` observes a real `cgroup.events` value of `populated 0`
  (`linux_containment.rs:763-773`). V1-01 and V1-03 likewise need a real
  `PublicationPermit`, which only `PublicationOwner::authorize` may mint.
  A fake, failed, or cloned registration therefore cannot substitute. Every
  remaining independently executable Linux gate is now closed; the residual
  gates require either an approved fixture-only lifecycle control or an
  authorized delegated cgroup.
- **Decision-required only if reached:** a V1-02b acceptance revision, and only
  if production call-chain evidence proves there is no safe test-only
  reachability control. No decision is needed for the already-approved bounded
  successor direction.
- **Windows-owned:** native writer-drain adapter, Windows process/filesystem
  semantics, NTFS/no-replace behavior, packaging, GUI and Windows validation.
  These do not block independent Linux/shared implementation. aria2-specific
  acceptance remains user-deferred/`NOT_RUN`.

The active cursor is cross-platform review of the fixture-only V1-02b
factory/runner lifecycle seam and continuation of individually executable Linux
gates. The same containment-backed runner seam also prevents current Linux
execution of V1-03's production-finalizer failure/replay acceptance. A barrier
inside the existing drain is not viable: it prevents `execute` from returning,
so the production runner cannot enter the finalizer. Do not implement that
design or claim a Storage-only test closes runner acceptance.
CAS-after-process-crash/restart recovery is covered by a subprocess test. The
runner CAS-loss tests include the production startup boundary and SQLite CAS
result, with a fixture-local SQLite trigger modeling the concurrent winner; a
two-connection stress/race test has not been added. The generic execution/
startup audit is complete for this batch. V1-02b has no
safe reachable negative-case seam in the current synchronous production path;
the smallest safe next step is a fixture-local barrier at the real adapter drain
boundary, reviewed before introduction because it must not become production
authorization or a production pause hook. The following dated sections are
historical evidence, not current checkout verification.

### Historical execution state — 2026-11-05

The approved Option B recovery authority and adapter-minted evidence boundary
remain in force. The current dirty source implements session-derived drain
evidence bound to operation, workspace, attempt, and the fixed writer-role set;
this continuation did not change that trust contract. Startup candidate
enumeration/reporting and journaled InternalV1 runner recovery are present.
Source inspection confirms journaled InternalV1 recovery obtains runner
admission, acquires both scopes, validates the existing attempt, drains through
the session-derived binding, revalidates durable identity, then performs
phase-specific authorization while retaining the session. Historical notes
below that describe a startup pre-drain followed by a second evidence drain
are stale relative to this implementation.

### Next batch — journaled refusal and bounded no-journal successor

- The next implementation batch is approved to proceed within the existing
  bounded successor direction and ADR-015 ownership constraints; do not create
  a redundant user-approval gate. The successor changes shared execution
  semantics and must be marked `CROSS_PLATFORM_CHANGE_REQUIRED`. ADR-015 is not
  authorization for a general Workflow redesign or any change to publication
  authority.
- First add and fix the journaled drain-refusal regression. A drain refusal must
  report `Refused`, preserve the existing state, attempt and journal, and remain
  retryable. `DeferredByContention` is reserved for actual scope contention;
  do not persist `FAILED` for a writer that may retain commit rights. Verify a
  later retry can recover the same journaled attempt.
- Then audit every reachable InternalV1 entry and implement the no-journal
  `DOWNLOADING` chain only as predecessor validation → adapter-minted drain
  evidence → Storage no-journal expected-predecessor successor CAS → one-shot
  Workflow session takeover under the same runner admission and Job/destination
  scopes. Ordinary `begin_attempt`/claim must not become a fallback. Preserve
  PREPARED/COMMITTED recovery of the existing attempt and Legacy behavior.
- A drain, wait, or process shutdown must not run inside the SQLite write
  transaction. CAS failure, journal/cancellation race, or drain refusal must not
  start a writer or write predecessor `FAILED`. If the process stops after CAS
  but before writer start, retain the successor attempt for startup recovery.
- V1-02b remains `NOT_RUN` pending cross-platform review of a fixture-only
  factory/runner lifecycle seam. The 2026-11-06 reachability audit shows that a
  barrier inside the current synchronous drain prevents `execute` from returning
  and therefore cannot reach the real finalizer. Do not fabricate facts, alter
  the live registry, reorder drain, or add a production pause hook. Any change to
  the acceptance target still requires user decision.
- **2026-11-05 validation update:**
  - Storage lib tests: 176/176 PASS (InternalV1 successor CAS checks, drain refusal regression,
    no-journal DOWNLOADING chain assertions across QUEUED/DOWNLOADED/VALIDATING/METADATA_READY/INTERRUPTED states)
  - Workflow lib tests: 49/49 PASS (drained predecessor takeover, binding tests, session recovery)
  - Desktop lib tests: 360/362 PASS (2 failures: `startup_recovery_fails_closed_when_internal_manifest_is_missing`
    [regression from successor recovery changes], `startup_recovery_drains_crashed_writer_domains_before_decisions`)
  - `generic_execution_entry_points_refuse_internal_v1_downloading_without_mutation`: PASS
    (fail-closed for InternalV1 DOWNLOADING at generic execution entry points, no writer starts on refusal)
  - V1-02b: `NOT_RUN` - current synchronous execute/drain/return order has not established a legal
    production path where a registered writer is still undrained at the real finalizer. Do not fabricate
    facts, alter the live registry, reorder drain, or add a production pause hook. If no test-only control
    point preserves the production call chain, provide the reachability evidence and a minimal acceptance
    revision proposal for user decision. The existing PREPARED replay regression was extended with a
    scoped SQLite trigger at the V1 journal COMMITTED update; after twice reproducing a split-transaction
    failure, Storage now applies recovered rows and the PREPARED→COMMITTED compare-and-set in one SQLite
    transaction in both authorized V1 replay entry paths. This is Storage replay evidence, not V1-02b or
    runner-level V1-03 acceptance.
- The historical delegated-service authorization only permits a bounded
  disposable user-level probe/fixture within its recorded scope. It does not
  establish that the current environment can perform the exact registered
  writer containment test. Recheck current capability and authorization before
  any such probe; request narrowly scoped authorization if needed.
- Existing validation counts below and in the active handoff are historical
  evidence for their recorded dirty source, not results for this documentation
  or future implementation batch. Full workspace regression was not run.

### JDR-01 execution update — 2026-11-05

- Added a regression to
  `startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`:
  a malformed recorded containment entry first produced the expected red
  failure (`DeferredByContention` instead of `Refused`, exit 101), while
  asserting the actual startup report.
- Fixed the result classification with an InternalV1-specific outcome that
  distinguishes completed recovery from actual admission/scope contention.
  Adapter drain errors now return the existing per-job refusal path without
  persisting `FAILED`; after the malformed record is removed, recovery succeeds
  on the same journaled attempt and existing replay/idempotence assertions pass.
- Exact targeted test passed 1/1. Startup filter passed 9, ignored 1
  (`startup_recovery_drains_live_descendant_after_owner_sigkill`, requiring a
  delegated disposable cgroup), exit 0. A prior `--exact` invocation selected
  zero tests and is not counted as evidence. One intermediate compilation
  failed from a misplaced enum brace while editing; corrected before the
  passing runs. Existing unused-variable warning at `executor.rs:8617` remains.
- Startup-filter command:
  `CARGO_TARGET_DIR=.tw2tg-tmp/c1-target cargo test -p xarchive-desktop --lib startup_recovery_ -- --nocapture --test-threads=1`
  (exit 0). `git diff --check` and `node scripts/docs-audit.mjs` passed after
  the implementation/cursor update. No Windows/cgroup test, commit, or push
  was performed.
- Direct execution entry audit is still in progress. Generic
  `execute_persisted_on_worker` and `execute_persisted_from_factory` can call
  `begin_attempt` for a running-state job; production Unix scheduling uses
  `SessionExecutionBoundary`, whose `DOWNLOADING` branch also currently calls
  `begin_attempt`. Do not route production InternalV1 `DOWNLOADING` through
  these ordinary claims before the successor primitive exists.

This continuation repaired the shared non-Linux cfg gap: Workflow recovery
authorization remains explicitly fail-closed off Linux, and Linux-only recovery
fixtures are not compiled on Windows. The Windows-target Workflow library check
passed. Linux Workflow tests passed 49/49; Storage library tests passed 174/174;
the Storage candidate-query matrix passed 1/1; Desktop startup recovery passed
9 tests with one delegated-cgroup test ignored. One isolated missing-manifest
run unexpectedly observed `EXECUTOR_WORKER_FAILED` instead of the expected
`ARCHIVE_COMMIT_RECOVERY_FAILED`; the exact rerun and complete startup filter
passed. Preserve the first observation as a superseded diagnostic. Its fixture
is `DOWNLOADED`, outside no-journal normal-execution dispatch. Source review
found that `SessionExecutionBoundary` uses `begin_attempt` for `DOWNLOADING`,
which increments the durable attempt; adding that state to normal resume is not
a safe local routing change. This requires a shared attempt/recovery decision,
not a state-list-only change. The reviewed job-state
contract only describes repeated submission as returning/reporting the existing
active job; the archive ownership decision says journaled recovery retains its
attempt and a retry may advance only after the prior attempt loses commit rights.
Neither document decides the crash case of a no-journal `DOWNLOADING` row. Keep
the route fail-closed/unmodified until that shared policy is decided; do not infer
either same-attempt resume or a fresh-attempt retry. Formatting, diff-check, and
docs audit passed.

### Continuation — 2026-11-05 (drain refusal and non-Windows regression pass)

- Corrected the InternalV1 no-journal startup drain-refusal path: a failed
  predecessor writer drain now reports recovery refusal without persisting
  `FAILED` or changing the durable state/attempt, because the old writer may
  still retain commit rights. The exact Desktop regression passes 1/1 after
  correcting its expected fixture state from DOWNLOADED to DOWNLOADING; its
  first assertion failure was a test expectation error and is retained in the
  evidence log.
- Current-source validation: Workflow lib 49/49; Storage archive recovery
  27/27; Workflow InternalV1 recovery integration 2/2; Desktop startup filter
  9 PASS / 1 ignored; Desktop lib 361 PASS / 8 ignored; Workflow session
  boundary module 21/21; pending-finalizer cancellation/incomplete-drain 1/1;
  PREPARED/COMMITTED startup replay 1/1; executor transport-generation
  replacement 1/1. `cargo fmt --all -- --check`, `git diff --check`, and
  documentation audit passed on the recorded source snapshot. Live-descendant
  and positive real registered-writer publication checks remain ignored for
  the unavailable authorized disposable delegated cgroup; they are not PASS.
- This correction preserves retryability only. It does not implement the
  no-journal DOWNLOADING successor contract. That production route still must
  atomically retire/drain the predecessor and claim a successor under retained
  runner admission and scopes before execution. V1-02b, V1-03, remaining
  V1-04–07, V1-08/09, and Windows-owned follow-up remain open; see the active
  platform cursor for the current dependency matrix. No commit/push.

### Continuation — 2026-11-05 (successor boundary and V1-02b reachability)

- Rechecked the canonical branch/HEAD, dirty paths and existing validation logs
  before continuing. No product source was changed in this continuation.
  Workflow 49/49, Storage archive-recovery 27/27, Workflow recovery integration
  2/2, Desktop startup 9 passed/1 ignored, and Desktop library 361 passed/8
  ignored remain the recorded results in `/tmp/tw2tg-goal-20261105/`; the
  corrected drain-refusal regression passed in its recorded rerun and was not
  repeated. Its initial exit 101 is retained as a fixture expectation error.
  Full workspace regression is NOT_RUN. The WSL2 process is in `/non-systemd`
  on a cgroup-v2 mount; no authorized disposable delegated cgroup was
  established. Windows PowerShell availability from WSL is not Windows Owner
  validation; Windows-native work remains deferred to that owner/formal Git
  handoff.
- **Successor remains NOT_RUN; shared contract is `CROSS_PLATFORM_CHANGE_REQUIRED`.**
  Source confirms a no-journal `DOWNLOADING` execution reaches the
  `SessionExecutionBoundary` no-transition path and calls `begin_attempt` before
  any predecessor drain. Storage's current `claim_archive_attempt` rejects
  same-state transitions, while the session attempt and adapter-minted
  `VerifiedRecoveryDrain` are bound to an immutable predecessor identity. The
  journaled Option B path does not itself approve transferring predecessor
  retirement evidence into successor identity. The minimum safe shared seam is
  a move-only, evidence-consuming Workflow takeover that retains the already
  held runner admission and Job/destination scopes, validates durable
  `(job_id, predecessor_attempt, DOWNLOADING, no journal)`, drains the exact
  predecessor through the platform adapter, performs a Storage-owned atomic
  expected-predecessor/no-journal successor CAS, and returns a successor-bound
  session under the same guard. Publication permits remain independent. Do not
  weaken attempt fencing, make the identity mutable, or route `DOWNLOADING` to
  ordinary `begin_attempt`. Implement within the bounded direction now
  authorized; retain `CROSS_PLATFORM_CHANGE_REQUIRED` classification.
- **V1-02b remains `NOT_RUN`; reachability audit completed 2026-11-06.** The
  production path is `ArchiveExecutionJob::execute` →
  `execute_archive_context` → `collect_attempt_facts` →
  `ArchiveExecutionContext::shutdown_sidecar` →
  `AttemptArchiveFacts` → `PendingArchivePublication::from_verified_facts`.
  `collect_attempt_facts` synchronously shuts down the Sidecar and rejects a
  retained undrained aria2 owner before facts can be created. Only after
  `execute` returns does `SessionExecutionBoundary::execute_session` receive
  the pending value and synchronously invoke `finalize` while retaining the
  same session and `PublicationOwner`. In addition, the production Linux
  containment owner is local to writer startup/transfer and is drained before
  registration can authorize publication. Thus the real writer cannot remain
  undrained at the real finalizer through any existing control point.
  `AttemptArchiveFacts::after_sidecar_drain` and the lifecycle fixture are
  separate unit-level checks; neither exercises the actual registered-writer
  finalizer boundary.

  **Seam decision:** do not add a pause hook or alter the facts/authorization
  contract. A barrier inside the current drain cannot satisfy the target because
  it blocks `execute` from returning, so the runner cannot reach `finalize`.
  Making facts while a writer is undrained would weaken the current
  drain-gated constructor; mutating `PublicationOwner` registry state or
  fabricating a pending object would not be production-faithful. The smallest
  reviewable direction is a fixture-only construction/execution seam that
  supplies an actual started, registered adapter owner to the same runner
  finalization boundary and can hold its drain independently, with all seam
  inputs unavailable in non-test/release builds and no permit-minting access.
  This requires cross-platform review before implementation because it changes
  the shared runner/factory lifecycle boundary. Until that review authorizes a
  concrete seam, retain `NOT_RUN`; do not claim the synthetic lifecycle test as
  acceptance or modify production authority.

- **Focused Linux revalidation — 2026-11-06:** The following previously
  implemented, non-Windows tests were rerun individually on this dirty source;
  all passed 1/1: `startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`,
  `cancellation_that_wins_before_prepare_is_not_replaced_by_execution_failure`,
  `stale_attempt_success_records_no_completion_event_or_final_state`,
  `inline_runner_operation_panic_closes_reply_and_releases_admission`,
  `pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`,
  and `prepared_replay_is_atomic_under_sql_fault_and_terminal_convergence_is_idempotent`.
  The full serial `cargo test --workspace -- --test-threads=1` completed with
  exit 0 at `/tmp/tw2tg-linux-workspace-verification.log`; Desktop 365 passed / 8
  ignored, Storage 176 passed, Workflow 49 passed, and all other workspace and
  doc suites passed. Delegated-cgroup tests remain ignored, so this does not
  establish live-descendant drain, registered-writer production publication, or
  V1-02b. Final targeted Linux acceptance gates remain as classified above.
  A foreground attempt timed out before the successful background rerun; it is
  not counted separately.

### Cross-session handoff — 2026-11-05 (status reconciliation)

- **Current Git state:** branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `214d077b1f740253bb3e6de8f1d447e8ccbbd585`, equal to the local origin
  tracking ref. Dirty: 13 modified tracked paths and two untracked paths
  (`desktop/src-tauri/job-1.lock`, empty; `desktop/src-tauri/src/executor.rs.bak`).
  Preserve all; nothing staged, committed, or pushed. Recent commits: HEAD
  `docs: record InternalV1 recovery authority and current handoff`, parent
  `8a79d758 feat: add runner-owned InternalV1 recovery authority and startup replay`,
  followed by `1baefa34 docs: sync P4 handoff, status, and validation records`.
- **PASS:** on current dirty source Workflow lib 49/49; Storage archive recovery
  27/27; Workflow recovery integration 2/2; Desktop startup 9/1 ignored; Desktop
  library 361/8 ignored; session-boundary module 21/21; exact corrected
  drain-refusal test 1/1. Formatter, `git diff --check`, and docs audit PASS.
  Evidence and exit files: `/tmp/tw2tg-goal-20261105/`. Full workspace suite:
  **NOT RUN** in this continuation. Never count ignored cgroup checks or filters
  with zero selected tests as PASS.
- **FAIL, superseded:** first exact drain-refusal run exit 101 reflected a
  fixture assertion expecting `DOWNLOADED` instead of the actual `DOWNLOADING`;
  corrected and subsequent runs passed. The final test build reports unused
  variable `recovery` at `desktop/src-tauri/src/executor.rs:8613`.
- **NOT RUN / actionable:** production no-journal `DOWNLOADING` fenced successor
  retry; V1-02b real registered-but-undrained writer at the production finalizer;
  remaining V1-03–09 acceptance. Next Linux task should inspect the active
  handoff and dirty call-site diffs, then implement only the bounded successor
  chain: retain runner admission and Job/destination scopes, validate the durable
  predecessor attempt/no-journal state, drain and retire that predecessor with
  bound adapter evidence, atomically claim the successor in Storage, and take
  over within the same session. Keep publication and recovery authority distinct.
- **BLOCKED / requires capability or external owner:** live-descendant and
  positive real writer publication evidence requires an authorized disposable
  delegated cgroup; next task must reprobe, then mark BLOCKED if no permitted
  path exists. Windows-native implementation/validation needs Windows Owner and
  formal Git handoff. Manual Windows GUI/package checks remain queued; aria2 is
  deferred and `NOT_RUN`.
- **V1-02b caution:** current synchronous execute/drain/return chain cannot leave
  a registered but undrained writer at the real finalizer. First inspect the
  actual `PendingArchivePublication` construction/finalizer chain; if there is
  no production-faithful control point, record the blocker and smallest safe
  shared seam proposal. Do not fabricate facts, alter live registry state, or
  add production pause hooks. Do not change shared authorization semantics to
  satisfy a synthetic fixture.
- **Historical discrepancy:** prior entries contain older descriptions of
  journaled recovery's pre-drain/order and contradictory transient failures.
  For current state follow this dated handoff plus active
  `docs/status/platform-handoff.md`; historical entries remain for traceability
  and are not current status. A missing-manifest isolated observation of
  `EXECUTOR_WORKER_FAILED` was superseded by exact and startup-filter PASS.

Open gates include V1-02b registered-but-undrained refusal at the production
finalizer; V1-03 post-publication SQL failure/replay (sequenced after authorized
recovery); remaining cancellation/stale-result and panic/Drop arrangements
(V1-04–06); V1-07 active-writer executor replacement; V1-08 Sidecar gate
completion; and V1-09 final source-bound validation. This continuation reran
V1-02a, cancellation-before-PREPARED, stale-success fencing, pending-finalizer
cancel/incomplete-drain, and PREPARED/COMMITTED startup regressions (each 1/1
PASS), Workflow InternalV1 recovery integration (2/2 PASS), and the full Desktop
Linux library suite (361 PASS, 8 ignored). These results do not close V1-02b or
the Sidecar phase. Workspace `cargo test --workspace --no-fail-fast` completed
with one Desktop library failure (360 passed, 1 failed, 8 ignored):
`attempt_staging_resolution_requires_matching_identity_and_isolates_attempts`
failed spawning its temporary shell Sidecar with `ETXTBSY` (“Text file busy”).
That exact test passed in isolation immediately afterward, as did the complete
Desktop library suite (361/0/8); classify the workspace observation as
intermittent/unresolved, not as a clean workspace PASS or a confirmed product
regression. Other workspace crates, integrations, and doctests reported passing
in that run. A subsequent serial workspace rerun,
`cargo test --workspace --no-fail-fast -- --test-threads=1`, passed. Retain both
workspace observations; the serial PASS does not erase the parallel ETXTBSY
failure. User-deferred aria2 remains NOT_RUN. No formal Git handoff exists; do
not start Windows native implementation or acceptance. Windows-native
validation remains Windows Owner work after formal Git handoff.

### Continuation — 2026-11-04 (recovery evidence binding)

- Implemented `RecoveryDrainBinding` derived from the live `ExecutionSession`;
  Linux adapter-minted evidence now binds operation identity, workspace, attempt,
  and the fixed production writer-role set. Workflow authorization validates this
  binding before issuing recovery authority. Desktop recovery passes the
  session-derived binding; housekeeping retains its non-evidence drain path.
- Corrected stale test expectations to reflect binding-first rejection order.
  Workflow lifecycle tests PASS 6/6; full Workflow crate PASS (49 unit tests,
  integration tests, and doctests; two delegated-cgroup tests remain ignored).
  Storage `archive_recovery` filter PASS 27/27. `node scripts/docs-audit.mjs`
  PASS. Desktop test compilation had passed `--no-run` in the preceding run;
  Desktop runtime tests were not run in this continuation.
- Revalidated on the current dirty tree: Desktop startup PREPARED/COMMITTED
  replay/idempotence PASS 1/1 and the uncontained-writer production rejection
  PASS 1/1. The latter is not V1-02b's live registered-writer negative case.
- Production call-chain audit found startup's journaled branch still performs a
  pre-session drain and discards its evidence, then enters
  `recover_internal_v1_on_runner`, which acquires runner admission/scopes and
  drains again before authorization. This does not satisfy approved ordering
  (admission/scopes → journal/attempt validation → drain → revalidation → permit).
  Correcting the caller/evidence flow is a shared authorization integration
  change: `CROSS_PLATFORM_CHANGE_REQUIRED`; do not claim the production chain
  complete or start dependent V1-03 acceptance until corrected and retested.
- Next concrete task: route journaled startup recovery directly through one
  runner-owned operation that retains admission and both scopes, validates the
  original attempt before drain, then passes that exact adapter-minted evidence
  to PREPARED/COMMITTED authorization. Keep this within approved Option B; do not
  use recovery evidence as ordinary publication authority. V1-02b's live writer
  negative finalizer control remains independently open.
- The first lifecycle reruns exposed test expectation drift only; after precise
  assertion corrections, the targeted module and full Workflow suite passed.
  V1-02b/V1-03 acceptance remains open: this component validation does not prove
  the full production recovery chain or post-file-commit failure/replay behavior.
- No commit or push. Current working tree remains dirty; Windows-native
  validation is not performed here.

### Continuation — 2026-11-04

- Reconciled current HEAD and found the prior reported undefined `evidence`
  compile error in `authorize_committed_recovery`, not in `PublicationOwner`.
  Corrected the parameter indentation/declaration only; no authorization
  behavior changed.
- Added Workflow tests rejecting recovery authorization for mismatched attempt,
  mismatched workspace, invalid journal-relative path, and PREPARED/COMMITTED
  phase mismatch. `cargo test -p xarchive-workflow writer_lifecycle::tests --
  --nocapture` initially found an invalid attempted timeout test: it used an
  empty record root, for which `NoRecord` is an accepted success. Removed that
  invalid test without changing product behavior. Re-run focused tests after
  that removal.
- A real active-domain timeout test was not established; it needs a live
  delegated-cgroup fixture. Do not claim timeout rejection validation from the
  no-record path. Per requester direction, ordinary live-publication evidence
  bridge/finalizer work is deferred pending cross-platform contract confirmation;
  recovery-only proof must not be reused as live-writer proof.

### Continuation reconciliation — 2026-11-04

- Git/handoff reconciliation: branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `214d077b1f740253bb3e6de8f1d447e8ccbbd585`; three modified tracked paths,
  no untracked paths. The earlier cursor's 18+6 inventory and embedded `1baefa…`
  source identity were stale. Preserved source and ledger edits; no commit/push.
- Verified previous recovery-auth fix: `cargo test -p xarchive-workflow --lib`
  PASS 49/49; `cargo test -p xarchive-desktop --lib --no-run` PASS. Tightened
  tests so both PREPARED and COMMITTED authorization reject wrong attempt,
  workspace and invalid journal path; PREPARED/COMMITTED phase mismatch also
  rejects. Renamed the case so it no longer falsely claims to exercise
  “missing evidence”; absence is excluded by the required move-only parameter.
- Continued independent validation: `cargo test -p xarchive-storage --lib`
  PASS 174/174; `cargo test -p xarchive-workflow --test internal_v1_recovery_tests`
  PASS 2/2; `cargo test -p xarchive-desktop --lib startup_recovery_` PASS 9,
  1 ignored delegated-cgroup case. The ignored test was not re-executed here.
  `git diff --check` and `node scripts/docs-audit.mjs` PASS.
- Capability probe for a fresh active-domain timeout/fixture run did not produce
  captured completion evidence through the terminal integration. No fixture was
  started; mark that check NOT_RUN, not PASS or product FAIL. Prior desktop test
  output confirms the expected delegated-cgroup ignore requirement.
- Corrected the obsolete next-task wording: the approved recovery evidence
  boundary is not being redesigned. Next executable source-bound task is to
  complete the authorization rejection matrix without synthetic minting and
  continue non-cgroup Sidecar production coverage; live-writer publication
  evidence remains the separately identified shared-contract gate. Active-writer
  replacement remains unverified and Windows work remains Windows Owner-owned.
- Continued V1-02 with the existing production rejection test
  `executor::tests::production_runner_rejects_uncontained_internal_v1_writer_before_publish`:
  fully qualified exact filter PASS 1/1. A prior exact invocation without its
  module prefix selected 0 tests and is not evidence. V1-02b (registered
  production writer with incomplete drain) remains unproven; do not mutate
  registry booleans or mint a synthetic permit to force that branch.
- Environment capability was checked: Linux x86_64 WSL2, direct cgroup
  membership `/non-systemd`, root cgroup not writable, `/usr/bin/systemd-run`
  and user manager available. A bounded `Delegate=yes` probe exited 0. The exact
  ignored descendant-SIGKILL fixture then PASSed 1/1 under
  `run-p439486-i441789.service`; a repeat with corrected parent-path quoting
  PASSed 1/1 under `run-p440816-i443123.service`. Both used the isolated
  `/tmp/tw2tg-resume-cursor` fixture root, which was confirmed owner-only before
  execution. This validates the specific persisted orphan-domain recovery case
  only, not active live-session replacement or publication authorization.
- Focused `rustfmt --check` reports formatting differences in both this new
  test block and pre-existing code in the same dirty Workflow file, plus
  unrelated Desktop formatting differences. Avoid rewriting unrelated source;
  format the newly added block only and rerun targeted validation.
- Final rustfmt comparison confirms the newly added test module is byte-for-byte
  equal to rustfmt output; the whole file still differs because of pre-existing
  formatting in the Recovery helpers and publication owner. `git diff --check`
  and docs audit pass after ledger updates.
- V1-02b remains concretely blocked by missing production test control: the
  current production execution owns registered writer creation/drain/finalize
  synchronously and exposes no parent-controlled pause/replacement seam. Do not
  turn lifecycle registry bits into drain proof. The separately scoped
  real-session replacement barrier is the next dependency: introduce a bounded
  parent-controlled test seam without changing production authority semantics,
  then test admission close, actual drain, finalization, join and generation
  order. Preserve the approved evidence contract; keep ordinary live-writer
  publication bridge gated as recorded above.


### Immediate execution cursor

- **2026-11-04 continuation reconciliation:** handoff and plan differed on whether the next V1-02b action was adding a replacement seam or completing publication evidence integration. Source audit confirms `PendingArchivePublication::finalize` closes admission and calls `PublicationOwner::authorize`; that shared owner currently decides from role-state booleans. The existing delegated production fixture proves the positive registered-writer drain/finalize path only. V1-02b requires a genuinely registered writer to remain undrained at the production authorization boundary. Preserve approved Option B; do not change authorization semantics or fabricate registry state. A fixture-local control must pause the real writer after registration and before adapter drain, run the actual production finalizer, prove refusal and absence of PREPARED/final publication, then release and verify actual drain and join ordering. No safe seam for that ordering is established in this continuation; keep V1-02b NOT_RUN and active-writer replacement open. Any shared authorization contract change remains `CROSS_PLATFORM_CHANGE_REQUIRED`. Transport-generation replacement is not writer replacement evidence. Independent Startup/candidate-state checks have been revalidated. Windows-native validation remains Windows Owner work.
- **Continuation recheck — 2026-11-05:** Reviewed the approved sequence and current production path; no safe deterministic control point exists between actual Linux writer registration/drain and `PendingArchivePublication::finalize`. V1-02b remains NOT_RUN; do not fabricate registry state. The existing PREPARED replay regression was extended with a scoped SQLite trigger at the V1 journal COMMITTED update. It reproduced a split-transaction failure: replay moved the directory and committed archive rows, then failed to advance the journal, leaving PREPARED. After independently reproducing twice, Storage now applies recovered rows and the PREPARED→COMMITTED compare-and-set in one SQLite transaction in both authorized V1 replay entry paths. The regression asserts transaction rollback preserves the PREPARED journal and prior row/state/event facts while retaining the moved directory, then drops the trigger, obtains fresh adapter drain evidence/permit, and converges the same attempt to COMMITTED/COMPLETE idempotently. Exact test PASS 1/1; Storage library 174/174; Workflow library 49/49; Workflow recovery integration 2/2; Desktop startup 9 passed/1 ignored; existing production uncontained-writer refusal 1/1. This is Storage replay evidence, not V1-02b or runner-level V1-03 acceptance. Attempt-ownership docs still leave no-journal `DOWNLOADING` crash semantics undecided; route/attempt behavior remains unchanged pending shared policy. Full workspace regression is pending; formatting/diff/docs audit pass.
- **Continuation recheck — 2026-11-05:** Reviewed the approved sequence and current production path; no safe deterministic control point exists between actual Linux writer registration/drain and `PendingArchivePublication::finalize`. V1-02b remains NOT_RUN; do not fabricate registry state. The existing PREPARED replay regression was extended with a scoped SQLite trigger at the V1 journal COMMITTED update. It reproduced a split-transaction failure: replay moved the directory and committed archive rows, then failed to advance the journal, leaving PREPARED. After independently reproducing twice, Storage now applies recovered rows and the PREPARED→COMMITTED compare-and-set in one SQLite transaction in both authorized V1 replay entry paths. The regression asserts transaction rollback preserves the PREPARED journal and prior row/state/event facts while retaining the moved directory, then drops the trigger, obtains fresh adapter drain evidence/permit, and converges the same attempt to COMMITTED/COMPLETE idempotently. Exact test PASS 1/1; Storage library 174/174; Workflow library 49/49; Workflow recovery integration 2/2; Desktop startup 9 passed/1 ignored; existing production uncontained-writer refusal 1/1. Full serial workspace regression `cargo test --workspace --no-fail-fast -- --test-threads=1` PASS (exit 0), including doctests; the declared delegated-cgroup tests remain ignored. Formatting, diff-check, and docs audit pass. This is Storage replay evidence, not V1-02b or runner-level V1-03 acceptance. Attempt-ownership docs still leave no-journal `DOWNLOADING` crash semantics undecided; route/attempt behavior remains unchanged pending shared policy.
- **Revalidation in this continuation:** Workflow lifecycle filter PASS 6/6; Storage `recovery_candidate_query_includes_only_nonterminal_recovery_states` PASS 1/1 (the attempted `--exact` filter first selected 0 and was not counted); Desktop startup filter was attempted and failed one missing-manifest assertion, so the previous PASS 9/1-ignored result is not current evidence for the present source. The earlier V1-02a `production_runner_rejects_uncontained_internal_v1_writer_before_publish` PASS 1/1 remains scoped to its recorded source/environment. Two earlier malformed exact filters selected 0 and are not evidence. Existing compiler warnings unchanged.
- **Current batch task review — 2026-11-04:** Inspected the 13-state `JobState` enum against `list_recovery_candidate_jobs`; the existing Storage matrix constructs all states and verifies exactly the eight active states plus INTERRUPTED, excluding COMPLETE/FAILED/CANCELLED/AUTH_REQUIRED. No independent gap found. Existing delegated registered-writer fixture proves only drain-before-finalize success; it cannot establish V1-02b's incomplete-drain refusal at the real finalizer boundary without a controlled writer lifetime seam. V1-03's trigger-only Storage failure is explicitly sequenced after the approved recovery-authority integration, so it is not promoted to runner acceptance or started ahead of that dependency. No additional independent executable production gate was identified in the approved plan; continue after the shared authorization boundary/seam is available. No product source changed in this continuation.
- **Resumption reconciliation — 2026-11-04:** Re-read continuous-goal procedure, Git and top handoff. Branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD `214d077b1f740253bb3e6de8f1d447e8ccbbd585`; three tracked modified files, no untracked paths. `cargo test -p xarchive-workflow writer_lifecycle::tests -- --nocapture --test-threads=1` completed with EXIT_CODE=0 (6/6); log `/tmp/tw2tg-writer-lifecycle-resume.log`. Tool search timed out and is not evidence. Source review confirms Desktop's PREPARED path obtains a session, drains, rechecks, derives `PreparedRecoveryPermit`, consumes it in Storage replay, converges and retains session through the operation. However the Linux `VerifiedAttemptDrain` currently carries only workflow attempt and workspace; the adapter loops the fixed three role names but does not embed operation/domain-set identity, contrary to approved proposal §§173-211. This is the next production-chain gap; do not mark recovery-authority integration complete or run dependent V1-03 acceptance until the witness binds and Workflow validates those identities. Existing evidence tests and current dirty-source test are component evidence only.
- **Resumption implementation — 2026-11-04:** Began closing the approved evidence-binding gap. `ExecutionSession` has a private per-session source identity, but no exported recovery-operation binding yet. Do not pass a raw caller string/nonce from Desktop: that would be self-asserted evidence. Next concrete implementation is an opaque Workflow-issued binding derived from the live session and exact fixed-role set, consumed by the Linux adapter when minting `VerifiedAttemptDrain`, then checked by Workflow authorization. Preserve the existing runner/session/Storage call chain and phase-specific authorization; run focused Workflow tests before touching Desktop call sites.

1. The startup dispatch/report and candidate query are implemented. A production
   fixture run exposed and this continuation corrected the pre-claim InternalV1
   routing defect; regression proves the new job claims attempt 1 through the
   session-owned boundary and refuses publication without trusted drain evidence.
   Startup recovery filter and full Desktop lib suite pass on this dirty source.
   The exact registered-writer production case subsequently PASSed twice in
   previously approved bounded disposable `Delegate=yes` services; direct host
   membership remains WSL2 `/non-systemd`. Do not generalize that PASS to the
   separate descendant-SIGKILL case. On 2026-11-04 direct non-delegated
   execution failed at `create_recorded` with PermissionDenied (expected for
   this WSL2 membership); rerun under the already-authorized disposable
   `Delegate=yes` service passed 1/1 (unit `run-p359903-i361747.service`,
   0.23s). This now proves the exact fixture's recovery path can drain the
   orphaned descendant in that bounded delegated environment only; it does not
   prove active-writer replacement or Windows behavior.
2. The latest Storage recovery-module run is 27/27 PASS; earlier fixture failures
   came from tests still invoking the deliberately refused legacy no-permit API.
   The current tests were updated to assert fail-closed/no-mutation and no longer
   claim successful replay through that entry point. Recheck this module after
   any authorization-contract change. Post-directory-commit SQL fault injection
   is fixture-local feasible: Storage
   already has `#[cfg(test)] connection_for_test()`, so a Storage-crate test can
   install a trigger without a production hook. Do not proceed to call that a
   runner acceptance yet: audit found `PublicationOwner::authorize` currently
   derives drained flags from lifecycle registration, not `VerifiedAttemptDrain`.
   The missing evidence binding is a shared authorization-contract issue;
   preserve fail-closed behavior and obtain the required cross-platform review
   before changing that boundary. Meanwhile continue independent active-writer
   active-writer replacement feasibility. Keep Windows-native validation
   with the Windows Owner after formal Git handoff.

3. **Startup report follow-up (2026-11-04):** the real startup recovery regression
   now requires exactly one `Recovered` outcome for its PREPARED replay and
   exactly one for the subsequent COMMITTED convergence, rather than merely
   checking absence of refusal and sleeping. Targeted Desktop test PASS 1/1;
   Desktop test PASS 1/1; candidate-query matrix PASS 1/1; Storage recovery
   filter PASS 27/27.
4. **Active-writer replacement feasibility result (2026-11-04):** the only
   existing replacement test is `runtime_replaces_executor_without_stale_worker_events`
   and operates on transport generations; it has no live `ExecutionSession`,
   registered containment writer, or drain/admission assertions. No current
   test seam can safely create a second startup-recovery caller while a live
   production runner owns an InternalV1 attempt. Do not treat the transport
   test as writer replacement evidence. Keep this specific coverage deferred
   until a production runner/session control seam exists; pursue candidate-state
   matrix coverage independently. Storage candidate query matrix passes 1/1;
   dispatch coverage now also verifies QUEUED, VALIDATING, METADATA_READY,
   DOWNLOADING and INTERRUPTED no-journal InternalV1 jobs route to session
   execution, while DOWNLOADED without a journal refuses without mutation.
   The exact dispatch test passes 1/1 and Desktop startup filter passes 9/9
   applicable with 1 ignored. The existing tests exercise those dispatch
   branches individually but did not assert every active-state candidate was
   enumerated exactly once. Added Storage query matrix regression for all 12
   persisted states; exact test PASS 1/1. It proves the query includes the 8
   active states plus INTERRUPTED and excludes COMPLETE, FAILED, CANCELLED,
   while COMPLETE, FAILED, CANCELLED and AUTH_REQUIRED are excluded. The
   Desktop startup filter now passes 9/9 applicable tests, 1 ignored. Root cause
   fixed: journaled published candidates now take the evidence-backed runner
   recovery path; missing-journal DOWNLOADED and malformed writer-record cases
   retain their distinct fail-closed error codes. Current active state remains
   `CROSS_PLATFORM_IN_PROGRESS`.

The typed startup report and fallible recovery-candidate query are the selected
implementation direction for this phase. They are shared behavior changes and
must be marked `CROSS_PLATFORM_CHANGE_REQUIRED`. Do not add a public capability
constructor or production fault-injection API; prefer fixture-local controls and
database triggers where they prove the exact boundary. R0–R7 refactoring remains
deferred until applicable InternalV1 gates are closed.

## 1. Purpose and scope

This document defines the concrete completion criteria for the in-progress
InternalV1 production execution, publication, and recovery validation, followed
by the approved sequence for modular refactoring. It is a plan, not validation
evidence: no gate below is implied to have passed merely because it is listed.

The current approved phase scope is to demonstrate, using the production runner,
real session authorization, registered writer ownership, and durable state,
that success, rejection, failure, cancellation, process restart, and executor
replacement preserve the existing publication and recovery invariants for the
Sidecar path. The user explicitly deferred aria2-specific implementation and
acceptance for this phase; record those applicable checks as NOT_RUN with this
approval and follow-up, never PASS or NOT_APPLICABLE. Recovery-authority Option
B was explicitly approved on 2026-11-01 as recorded in
`../architecture/internal-v1-recovery-authority-proposal.md`. The trusted
platform-adapter drain-evidence boundary was explicitly approved on
2026-11-02: a private, move-only, identity-bound proof from the adapter after
the complete fixed writer-role set is drained; Workflow validates and consumes
it to derive the one-shot Storage permit. Recovery implementation is now
authorized shared work, marked `CROSS_PLATFORM_CHANGE_REQUIRED`. A production
call chain exists in the dirty source, but integration and validation remain
unverified until targeted tests establish the end-to-end behavior. This
approval does not authorize cgroup configuration or operations. Production
delegated-cgroup acceptance still requires an explicitly authorized disposable
environment. Independent Sidecar work continues where safe.
After the approved phase gates are complete, remind the user to begin the
modular refactoring plan. Windows-native acceptance remains separately owned
and must not be inferred from Linux evidence.
boundary. Historical entries below that describe the boundary as unapproved or
The 2026-11-02 user approval resolved the trusted adapter-minted drain-evidence
boundary. Historical entries below that describe the boundary as unapproved or
this decision. Production integration and validation remain outstanding.
this decision. Production integration and validation remain outstanding.

### Act continuation — evidence-boundary contract review — 2026-11-04 (historical)

This historical entry predates the subsequent explicit user direction to
preserve the approved architecture and limit work to recovery authorization
rejection tests. The Option B adapter proof and Storage's independent durable
journal checks remain approved. Recovery evidence proves the recovered fixed
writer-role set for a persisted attempt; it is not evidence that the current
live writer set has drained. Ordinary publication evidence integration is a
distinct shared-contract question and remains gated. Do not reinterpret this
historical “Next: design” wording as authorization to redesign the approved
boundary. Current executable cursor is maintained at the top of
`docs/status/platform-handoff.md`.

### Act continuation baseline — historical snapshot, 2026-11-01

At this historical continuation's read-only start, branch was
`cross-platform/automatic-pairing-reconcile-20261002`, HEAD and local upstream
were `1baefa345053204e0eb252213fdcdf4544494277`, and the tree contained nine
modified tracked paths and two untracked plan/proposal documents. Preserve all
pre-existing edits. First complete the documentation gate, then restore the
existing cross-crate `WriterRegistration::register_linux` visibility only if
the current build reproduces the recorded E0624. This is a normal registration
integration fix, not recovery authorization. Re-run the targeted Desktop build
and tests and bind outcomes to the resulting dirty source. Do not claim historic
component-test results as newly run. The ignored delegated-cgroup production
fixture remains NOT_RUN unless separately authorized in a specifically bounded
environment. See the current platform handoff and Linux validation ledger.

The plan preserves existing public contracts except for separately approved
recovery-authority changes, IPC names, execution-spec formats, SQLite schema,
error behavior, platform ownership, and the fail-closed InternalRenameV2 gate.
It does not authorize forged publication permits, weakened writer drains, a
Job-scoped publication fallback, or unrelated feature work.

## 2. Initial baseline and current-state routing

At the plan's creation, the repository was on branch
`cross-platform/automatic-pairing-reconcile-20261002`, source HEAD
`1baefa345053204e0eb252213fdcdf4544494277`, with six pre-existing tracked
modifications. This is historical baseline evidence, not the current inventory.
Preserve all current edits; inspect exact status before each batch and never
overwrite or attribute prior changes to this plan.

At plan creation, the handoff reported that the inline `python -c` worker did
not reach its barrier; `__file__` is not a usable barrier-path mechanism for a
`-c` program. The
deterministic storage failure window is after `commit_attempt_staging` and
before `update_tweet_metadata` in
`crates/xarchive-storage/src/archive_service.rs`. No PREPARED replay or B1/B2
acceptance is claimed. Existing executor replacement coverage checks transport
routing, not continuity of a genuinely active registered writer. The
registered aria2 drain/retry check is NOT_RUN because `aria2c` is absent in the
recorded environment. Windows-native acceptance remains a separate owner task.
Reconcile all of these statements against the live source and handoff before
execution; this plan is not a substitute for that reconciliation.

The creation-time inventory above is historical, not the live dirty-file count.
The [current handoff](../status/platform-handoff.md) owns the live cursor and
evidence links. The earlier Storage V2 fixture compilation blocker is no longer
an active task: the handoff records a later Storage library run of 172/172.
Do not erase the earlier failure or represent that recorded run as newly rerun.
The recovery authorization contract blocks its gated PREPARED replay work, not
every independent Linux-owned milestone. Do not add a permit-minting test
workaround. The current handoff owns the live inventory and next executable
milestone.

## 3. InternalV1 completion gates

### Approved phase scope — 2026-11-01

The user approved the remaining plan and Option B recovery-authority contract,
directed that aria2-specific tasks be skipped for this phase, and prioritized
modular refactoring after Sidecar-scoped V1 completion. Applicable aria2 checks
remain `NOT_RUN` with this user-directed deferral and follow-up; they are never
PASS or NOT_APPLICABLE by virtue of being skipped. This phase can be reported
complete only as the approved Sidecar-scoped phase, not as all-backend
InternalV1 completion. Windows-native work remains Windows Owner work.

Every applicable gate must be recorded as PASS, FAIL, BLOCKED, NOT_RUN, or
NOT_APPLICABLE using the evidence requirements in
`../validation/validation-policy.md`. A BLOCKED or NOT_RUN gate does not count
as complete. Record source SHA, dirty-state description, platform/environment,
exact command or procedure, exit code, evidence location, and follow-up.

| ID | Acceptance target | Required evidence |
|---|---|---|
| V1-01 | Production success path | A production runner claims and executes a persisted Job, drains its registered writer, publishes under the live session permit, and reaches the expected terminal state. Assert final file bytes and paths, tweet/media database facts, committed recovery journal, and exactly-once completion facts/events. |
| V1-02 | Authorization and isolation | With the production call path, reject an unregistered writer, incomplete drain, mismatched attempt, and mismatched destination. Assert fail-closed behavior and absence of an unauthorized PREPARED journal or final publication. Never mint or forge a `PublicationPermit`. |
| V1-03 | Post-file-commit database failure | Deterministically fail the applicable Sidecar publication database operation after `commit_attempt_staging` and before metadata/database convergence. Confirm no false terminal success, preserve durable recovery facts, and prove expected retry/recovery. Keep production entry and authorization ownership unchanged. |
| V1-04 | PREPARED crash and replay | Exercise Sidecar interruption after PREPARED and during/after persisted file renames, including the committed-directory/database-not-converged boundary. Restart through the authorized production recovery path; prove convergence, idempotence, and no duplicate media, completion events, or Telegram intent. A returned error alone is not process-crash evidence. aria2-specific writer recovery is deferred/NOT_RUN. |
| V1-05 | Cancellation and stale results | Exercise cancellation around the PREPARED arbitration boundary and a late result from a superseded attempt. Verify that the winning state is preserved, stale writes are fenced, and recovery does not publish for the wrong attempt. Targeted dirty-source reruns on 2026-11-02: cancel-before-PREPARED, stale-success suppression, and running-cancel late-success fence each PASS 1/1; full V1-05 recovery coverage remains open. |
| V1-06 | Panic, Drop, and resource lifetime | Exercise relevant panic/error/drop paths with the real session and registered owners. Show that Storage, Sidecar, writer registrations, and permit lifetime cannot be released in an order that allows unauthorized publication or an untracked writer to continue. Targeted dirty-source reruns on 2026-11-02: owner-before-admission drop, panic cleanup, startup cleanup-owner retry, attempt facts, and pending-finalizer/incomplete-drain each PASS 1/1; this is partial coverage, not V1-06 completion. |
| V1-07 | Executor replacement with an active writer | Pause a real registered writer inside its registered lifetime, replace the executor, and verify old-worker/runner shutdown and join before successor operation. Prove no overlapping publication authority or premature owner release. A transport-only restart test is insufficient. The 2026-11-02 runtime transport-generation replacement test PASS 1/1 but does not satisfy this production target; active-writer replacement remains open. |
| V1-08 | Supported transfer backends | Sidecar: verify registration, drain, cancellation/retry behavior, and its applicable publication integration. aria2-specific work is user-deferred for this phase and remains NOT_RUN with follow-up; Sidecar evidence does not substitute for aria2 evidence, and the phase must not be called all-backend complete. |
| V1-09 | Regression and evidence integrity | Run affected Storage, Workflow, Desktop executor/archive, protocol, and relevant subprocess integration tests. Bind results to the exact source SHA and dirty state. Record unrun full-suite and Windows checks accurately; do not promote compilation, mocks, or smoke tests into production-path acceptance. |

### V1-02 subtargets and evidence limits

| Subtarget | Production acceptance target | Evidence limit |
|---|---|---|
| V1-02a | Refuse publication without containment/registered writer authority | Independent of cgroup delegation. `executor::tests::production_runner_rejects_uncontained_internal_v1_writer_before_publish` was rerun on the current dirty source on 2026-11-02: PASS 1/1, exit 0. It asserts no PREPARED journal or final directory. The persisted outer error code is `EXECUTOR_WORKER_FAILED`; its message contains `PUBLICATION_INCOMPLETE_DRAIN`. |
| V1-02b | Refuse publication for a registered writer whose drain is incomplete | V1-02a and lifecycle component tests do not prove this registered production case. |
| V1-02c | Refuse an attempt mismatch at the relevant authorization/publication boundary | Source audit: current legitimate `PublicationOwner::authorize(session)` requires owner/session attempt identity and constructs the permit borrowing that session; finalizer checks pending attempt against that session. No reachable permit-bearing mismatch path was identified. Retain the reachable pending/session and Storage durable-attempt fences; do not forge a permit or claim synthetic permit-mismatch acceptance. |
| V1-02d | Refuse a destination mismatch at the relevant authorization/publication boundary | Source audit: the permit borrows the same session destination; finalizer checks it against the Storage-derived archive path, and Storage compares permit destination against its normalized request path. No reachable permit-bearing mismatch path was identified. Retain these reachable fences and test reachable tampering if found; do not forge a permit or claim synthetic permit-mismatch acceptance. |

Map each subtarget to source-bound evidence in the handoff/validation record.
One passing subtarget does not close V1-02. The table specifies coverage, not
a new parallel test-results ledger.

The production boundary audit must account for the current construction path:
`PublicationOwner::authorize(session)` creates a `PublicationPermit` whose
attempt and destination borrow that same session; `PendingArchivePublication`
also compares its identity and destination with the session before calling
`authorize`; Storage compares the consumed permit against the durable attempt
and normalized request destination. Therefore, a test cannot claim a
permit-bearing mismatch merely by passing a fixture-supplied claim identity to
an earlier session test. If no legitimate live state can make the permit
mismatch the publication request, record that reachability proof and retain
negative tests for each reachable fence; do not add a permit constructor,
mutable destination setter, or other authorization bypass to satisfy a
synthetic branch.

### Completion decision

The approved Sidecar-scoped phase is complete only when applicable V1-01,
V1-02, V1-05, V1-06, V1-07, V1-08 Sidecar, V1-09 and production recovery
acceptance gates are satisfied. Recovery targets are applicable under the approved Option B
contract; any recovery scenario that exercises the aria2 writer specifically
remains deferred, not inferred from Sidecar evidence. The drain-proof minting
boundary is approved but not implemented; production recovery remains open until
the adapter-evidence → Workflow-consumption → Storage-permit production chain is
implemented and validated as recorded in the recovery proposal. aria2-specific
V1-08 checks remain user-deferred/NOT_RUN; this phase
must not be described as all-backend InternalV1 completion. After truthful
completion of this approved phase, begin modular refactoring while retaining
the aria2 follow-up. NOT_APPLICABLE requires a source-backed reason; user
deferral is NOT_RUN. Any other BLOCKED/NOT_RUN applicable gate means the
Sidecar phase is not complete, although independent permitted work may continue.

Windows-specific cgroup/process containment, NTFS/no-replace behavior, native
credential or transport integration, GUI, packaging, and other Windows-only
checks remain Windows Owner work. Record their queue status separately; Linux
completion does not close them.

## 4. Fixture and failure-injection constraints

### Approved recovery-authority contract

V1-03/V1-04 require the explicitly approved Option B recovery-authority
contract and its production integration before adding the post-commit
failure/replay seam.
Mark this shared boundary `CROSS_PLATFORM_CHANGE_REQUIRED`. PREPARED replay
binds the immutable manifest's existing attempt; it must not masquerade as a
new durable claim via `ExecutionReservation::claim`.

The decision must define runner admission, held Job/destination scopes,
lock-protected journal/attempt/destination revalidation, recorded writer-domain
drain proof, and the authority accepted by Storage. Authorization must remain
valid through file replay, database convergence, attempt-fenced terminal
persistence, and final scope release. Define cancellation, contention, stale
attempt, failed drain, repeat recovery, panic and Drop behavior. Preserve legacy
Describe implementation from current source rather than this target contract:
startup currently holds workspace admission, conditionally drains recorded
writer domains when containment is present, then calls
`ArchiveService::recover_internal_archive` for InternalV1; that call does not
yet consume the approved recovery permit.

Option B was approved by the user on 2026-11-01, with its authority limited to
the contract in `../architecture/internal-v1-recovery-authority-proposal.md`.
The non-claiming existing-attempt session shape and phase-discriminated PREPARED
replay versus COMMITTED verification remain the approved design. The trusted
adapter-minted drain-evidence boundary was explicitly approved on 2026-11-02;
the historical approval gap is resolved. Implement the approved move-only
evidence and Workflow consumption path, then connect Storage and the production
InternalV1 recovery branch. Production recovery remains incomplete until that
chain is implemented and validated. Neither a fresh empty publication owner
nor a forged permit authorizes recovery of already-published files.

### Fixture and environment prerequisites

Prefer a real fixture script at
`desktop/src-tauri/tests/fixtures/internal_v1_worker.py` over extending a
`python -c` command contract. The fixture must implement the actual Sidecar
JSONL handshake and production argument shape, and provide a deterministic
barrier through an explicitly supplied fixture configuration or a
production-compatible coordination channel. Do not rely on `__file__` for a
`-c` program.

For a precise database failure, first assess whether a temporary SQLite trigger
can deterministically fail the target write without adding production hooks.
If not, propose a narrowly scoped test seam. Any feature-gated failpoint must
be disabled in default/release builds and must not bypass session authorization,
permit ownership, writer registration, or drain. Record tooling failures
separately from product failures. Do not change production behavior merely to

Use a disposable delegated cgroup only when the validation environment and
Use a disposable delegated cgroup only when the validation environment and
authorization explicitly permit it. Reconcile the ignored production-runner
required capability is unavailable, record the failed operation and continue
status (0 means writable; 1 means not writable); do not report the status value
as a capability boolean. A root control file's status alone does not establish
whether a delegated child is usable: verify the exact production
create/drain/remove operations in the disposable parent. The current WSL2
membership `/non-systemd` is not an authorized disposable delegation. A
`systemd-run --user --property=Delegate=yes` launch proves only service launch,
not fixture capability or authorization. Until the user separately authorizes
a bounded disposable delegated-service probe/fixture or provides an approved
delegation, keep those operations NOT_RUN. If a required capability is
unavailable, record the failed operation and continue independent checks; do
not claim an equivalent result from a weaker test.
independent checks; do not claim an equivalent result from a weaker test.
Environment and fixture preparation may proceed independently of the contract
proposal when they do not modify system delegation or install dependencies.
Inspect authorized disposable delegation paths and equivalent Linux execution
channels before declaring the target blocked. Do not elevate privileges, modify
system delegation, install dependencies, or configure external execution
without applicable authorization. Identify the supported aria2 executable/
version and artifact identity; an executable alone is not registered-writer/
drain/recovery acceptance. Production-runner validation must not overwrite or
unlink the current untracked `desktop/src-tauri/job-1.lock`. Before running an
integration test, trace `ProductionCoordinator::open` from the exact fixture
database path, then inspect the derived `locks/` entry and verify its identity
and intended effect. A unique database path is not enough to establish lock
isolation. If the test would mutate or replace the retained artifact, use only
an already-approved isolated worktree; do not remove, rename, or redirect the
lock as an ad hoc workaround.
untracked coordination artifact or change its inode. The current
`desktop/src-tauri/job-1.lock` must be retained; first verify that an isolated
checkout or another already-approved lock-directory configuration prevents the
test from touching it. Lock existence itself is not a global blocker, but do
not treat a custom database path as isolation until `ProductionCoordinator`
source confirms the derived lock path is isolated too.

### Evidence identity and formatting

When HEAD is unchanged but the tree is dirty, bind evidence to the dirty-file
inventory and a patch or relevant source-content hash as well as HEAD. Preserve
exact test filters, actual executed counts, exit records, environment and tool
identity. Log filenames do not establish execution dates: reconcile discrepant
names against log contents and independent execution records, mark unresolved
dates as unverified, and do not rename historical evidence to manufacture parity.

The recorded Desktop format check remains FAIL until that check actually passes.
Review ownership of format differences, fix only justified task-owned regions,
and treat any necessary whole-file formatting as an explicit reviewed scope.
Do not silently format unrelated dirty content or classify formatting failure as
a product-behavior defect. Product tests are not rerun merely to update this plan.

## 5. Work sequence for completing InternalV1

1. Reconcile the live branch, HEAD, dirty files, current cursor, ignored test
   prerequisites, and available runtime capabilities. Preserve all existing
   uncommitted work. (Completed for this Act resumption; recheck before each
   fixture or source batch.)
2. Pass the Documentation Gate: reconcile this plan, the current handoff,
   validation evidence, platform ownership, and Windows queue before business-
   code edits. Record exact acceptance and blocker classifications; run docs audit
   and `git diff --check` for the documentation batch. Reconcile all current
   status claims before production implementation.
3. Recovery-authority Option B and the adapter-minted trusted drain-evidence
   boundary are approved as recorded in the decision record. Implement only
   within that scope and return for renewed approval if semantics materially
   change. Do not treat approval as verification or as delegated-cgroup
   authorization.
4. Implement Sidecar production-fixture and normal-path work; verify exact
   delegated-parent provisioning operations when safely available. Keep aria2
   out of this phase and record its user-approved NOT_RUN deferral.
5. Implement runner-owned PREPARED recovery with the
   original attempt, both scopes, recorded-domain drain, Storage authority and
   attempt-fenced terminal persistence. Do not enable V2 or mint publication
   permits; implement only the approved, distinct recovery capability.
6. Establish a deterministic production-faithful registered-writer fixture and
   prove its barrier. Run the positive production path and V1-02 subtargets;
   reuse prior evidence only when its source/environment requirements match.
7. Only on top of the authorized recovery path, add the narrowest post-file-commit
   database failure injection, first assessing temporary SQLite triggers.
8. Exercise real process crash/restart, PREPARED replay, repeated recovery,
   cancellation and stale-result fencing (V1-03 through V1-05). Returned errors,
   thread panics and process crashes are distinct evidence targets.
9. Prove relevant panic/Drop resource retention and cleanup (V1-06).
10. Validate active registered-writer executor replacement and shutdown/join
   continuity (V1-07), not merely transport replacement.
11. Validate Sidecar scope under V1-08. Keep aria2-specific acceptance NOT_RUN
    as explicitly user-deferred; do not claim all-backend completion.
12. Run affected module/integration regression and format checks, review the final
    diff, and record exact source-bound evidence and Windows follow-up (V1-09).
13. Reconcile every applicable Sidecar-phase gate against source-bound evidence.
   Upon truthful completion, report the Sidecar-scoped phase and immediately
   begin the modular refactoring plan, retaining aria2 as a deferred follow-up.

## 6. Modular refactoring plan after V1 completion

Refactoring starts only after the completion decision above. Preserve behavior
and public contracts; move existing responsibilities behind stable module
interfaces rather than slicing files mechanically. Keep the current crate
boundaries unless dependency evidence justifies a deliberate boundary change.

### Phase R0 — baseline and governance

- Capture V1 regression evidence and public contracts before moving code.
- Add `docs/architecture/MODULAR_DESIGN_POLICY.md` and a concise reference in
  root `AGENTS.md`.
- Add a module documentation template and document the principal capabilities:
  archive execution/publication, archive storage/recovery, Telegram delivery,
  downloader extraction, and configuration/commands.
- Add a Node ESM architecture checker and `node:test` tests for file-size
  categories, exceptions, baseline deltas, dependency edges/cycles, and renamed
  files. Count physical source lines, including inline tests; use file size as a
  review signal, not a decomposition algorithm.
- Establish a reviewed historical baseline and explicit exceptions; do not
  regenerate or expand the baseline automatically. Add a dedicated CI check.

Candidate governance files are `scripts/check-architecture.mjs`,
`scripts/check-architecture.test.mjs`,
`scripts/architecture-policy.json`,
`scripts/architecture-baseline.json`,
`scripts/architecture-exceptions.json`, and
`.github/workflows/architecture-check.yml`. Update package scripts and index
the policy in `docs/architecture/repository-map.md`. Reuse the existing
`docs/architecture/overview.md`; do not add a competing architecture index.

### Phase R1 — low-coupling contracts and pure logic

- Extract execution-spec decoding and downloader snapshot validation from
  `desktop/src-tauri/src/executor.rs` into an executor specification component.
- Separate Telegram metadata formatting, text/media planning, and outbox
  transition policy inside `crates/xarchive-telegram`.
- Separate Sidecar gallery metadata normalization and command construction
  from `sidecar/src/xarchive_downloader/extraction.py`.
- Retain compatibility re-exports and unchanged serialization/error behavior.

### Phase R2 — Storage cohesion

- Reduce `crates/xarchive-storage/src/lib.rs` to module assembly, migration
  wiring, and stable exports.
- Move database connection/migration assembly into a focused database module;
  move credential-generation/rotation persistence into a focused repository.
- Split archive recovery manifest and journal responsibilities only where the
  current call graph and transaction boundary permit. Keep SQL semantics,
  schema, and recovery ordering unchanged.

### Phase R3 — Telegram delivery cohesion

- Split `crates/xarchive-telegram/src/lib.rs` by existing contracts,
  formatting/planning, outbox policy, transport, and upload responsibilities.
- Keep Desktop orchestration, Tauri state, Storage adapters, and platform
  credential adapters at their existing ownership boundaries.
- Validate timeouts, cancellation, credential rotation, and uncertain send
  outcomes after each extraction.

### Phase R4 — Desktop command/configuration boundaries

- Split `desktop/src-tauri/src/commands.rs` by jobs, batches, Telegram,
  settings, and record exchange while preserving registered Tauri command names
  and serialized inputs/outputs.
- Split configuration only along cohesive configuration ownership; keep
  environment-specific behavior behind existing platform boundaries.
- Extract frontend feature state only where it has a stable domain responsibility;
  preserve bootstrap and theme initialization order in `desktop/src/main.jsx`.

### Phase R5 — Executor and recovery core (highest-risk refactor)

- Keep `desktop/src-tauri/src/executor.rs` as the compatibility/module entry
  while extracting specification, contracts, persistence, worker, session,
  production factory, and recovery responsibilities.
- Move one responsibility at a time; preserve admission → scopes → claim →
  execute → drain → authorize → finalize → terminal persistence → release order.
- Re-run the V1 regression matrix after every change affecting resource
  ownership, recovery, publication, or executor lifecycle.
- Do not introduce a generic event bus, DI framework, speculative abstraction,
  or V2 enrollment as a by-product of extraction.

### Phase R6 — Sidecar and remaining capability review

- Separate extraction metadata, discovery, download, and gallery command
  construction where each has a cohesive contract.
- Preserve the existing protocol-v2 worker entry, cancellation and subprocess
  cleanup behavior; avoid a generic `helpers` module.
- Review other large files using actual responsibility/dependency evidence.
  Leave cohesive files intact even if they exceed a review threshold.

### Phase R7 — integration and handoff

- Run architecture checks, affected unit/contract/integration tests, formatting,
  lint/build checks, documentation audit, and final diff review at risk-appropriate
  scope.
- Record residual baseline debt, exceptions, Windows Owner work, and any
  NOT_RUN/BLOCKED checks. Do not represent Linux checks as Windows acceptance.
- Formal cross-platform handoff must use Git history and state branch, source
  commit, handoff commit when applicable, dirty state, and current Owner.

## 7. Refactoring acceptance and stop conditions

An extraction is accepted only when its business responsibility and dependency
direction are clearer, its public contract is explicit or intentionally
unchanged, existing behavior is regression-tested, and it does not create a
cycle, generic dumping ground, empty wrapper, or test-only reduction in line
count. Every new hard-limit violation requires a reviewed, visible exception.

Stop an affected batch if correctness depends on weakening authorization or
drain, changing a shared contract without the required cross-platform review,
rewriting unrelated behavior, or treating an environment/tooling failure as a
product defect. Preserve working state, record evidence and blocker, and
continue only independent permitted work.

## 8. Completion reminder and next action

The completion reminder is an explicit execution requirement, not a timer or
background notification. On the first response that truthfully declares the
InternalV1 shared Linux goal complete, immediately tell the user:

> InternalV1 共享 Linux 验证已完成，现在应开始已批准的模块化重构计划。

In that same response, summarize the evidence-backed gate status, list
remaining Windows Owner checks, and identify Phase R0 as the next action. Do
not issue this reminder for a passing sub-batch while any applicable gate is
BLOCKED, NOT_RUN, or unverified.