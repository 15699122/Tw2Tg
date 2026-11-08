V1 Completion and Modular Refactoring Plan

Owner: Cross-platform Owner for shared implementation and Linux validation.
Status: ACTIVE — complete the approved Sidecar-focused InternalV1 scope first;
aria2-specific work is deferred by user direction. Modular refactoring follows
only after the approved phase gates are met.

## Current execution state — 2026-11-08 (reconciled)

> This section is the only current state for this Plan. Every section below it is
> historical evidence bound to the source it was produced on. Historical PASS
> counts are never re-bound to a later HEAD.

### Verified checkout facts

> Re-verified 2026-11-08 after the R2/R3 refactoring batches. These facts
> supersede the earlier 7-modified/1-untracked inventory recorded in this
> section before those batches landed.

- Branch `cross-platform/automatic-pairing-reconcile-20261002`; HEAD
  `59edcbfa852152bac70ba8b62d0b31806f190d50`; working tree **CLEAN**; index
  **empty**. The earlier description of this checkout as **19 modified tracked
  files and 30 untracked entries** is superseded: those files were committed in
  `6b0220ed`, `370b10eb`, `68cfdcf1`, `a1d18df7`, `0ffc6cd3`, `8223ed67`,
  `fd1b5c7f`, `ca5c99a1` and `59edcbfa`. Historical evidence below stays bound to
  the revision that produced it.
- The previously listed modified set and untracked module set no longer describe
  the tree. What survives from that inventory and must still be preserved:
  `crates/xarchive-sidecar-supervisor/tests/linux_contained.rs.bak` (not identical
  to its HEAD revision, not redundant) and `desktop/src-tauri/job-1.lock`
  (inode `21721`).
- Coordinator locks derive from each database's own parent `locks/` directory
  (`ProductionCoordinator::open`).
- Authorization: the user approved staging, responsibility-scoped commits, push to
  this branch, one reviewed baseline reseal and preparation of a formal Windows
  handoff, for the R5–R7 plan and its tracking documents. Not authorized: `reset`,
  any `stash` operation, history rewriting, deleting recovery evidence, or
  overwriting the Windows Owner's working tree. `stash@{0}` is untouched.
- `git diff --check` **PASSES** (exit `0`, no output) on this tree.
- `node scripts/check-architecture.mjs` reports **0 findings** (184 files) on this
  HEAD; the earlier baseline drift was closed by the `ca5c99a1` reseal.
- Toolchain in this checkout: `rustc 1.98.0`, `cargo 1.98.0`, `Python 3.14.4`,
  `node v26.7.0`, WSL2 kernel `6.18.58.1-microsoft-standard-WSL2`.

### Corrections to the 2026-11-08 continuation

- The previous continuation reported a completed Documentation Gate and a
  completed "Recovery Safety" item. Neither is established by source evidence:
  the documentation contained contradictory inventory/blocker claims, and the
  added recovery assertions referenced APIs and a return type that do not exist
  in this source. Those assertions were **not** executed and are not evidence.
- `recover_internal_v1_on_runner` currently returns `Ok(false)` for a
  `COMMITTED` journal whose recorded state is already `COMPLETE` **before**
  `authorize_committed_recovery` and before
  `verify_committed_terminal_internal_v1`. Idempotence must be established after
  validation, not instead of it.
- `registered_writer_with_incomplete_drain_is_refused_by_authorization` calls
  `PublicationOwner::authorize` directly. It does **not** run
  `PendingArchivePublication::finalize`, so it cannot close V1-02b.
- `executor_shutdown_joins_active_writer_before_successor_runs` exercises
  `JobExecutor` admission/shutdown directly. It does not call
  `RuntimeState::replace_executor` and does not execute a successor job, so it
  does not close V1-07.
- The Sidecar cancellation fixture has real protocol defects independent of any
  environment limit: the contained test sends a command the fixture does not
  handle for that path, the cancel request/job identities do not match the
  launched extraction, and the fixture emits a `shutdown_ack` event that the
  protocol event enum does not define. V1-08 is blocked by fixture defects, not
  only by cgroup availability.
- The historical `/tmp/tw2tg-linux-v104.log` / `.exit` and
  `/tmp/tw2tg-final-v109.log` / `.exit` artifacts referenced by the V1-04
  record were not present in this session. Keep the historical record, and do
  not claim those logs were re-inspected.

### Approved decisions for the R5–R7 continuation — 2026-11-08

The user approved these four decisions. They are recorded here so an Act session
does not have to re-derive them, and so no later batch silently widens them.

1. **Extraction method.** Continue the modular refactor by moving one
   responsibility at a time behind the existing compatibility entry, preserving
   current IPC contracts, re-export paths and the ADR-015 lifecycle order. No new
   crate, generic event bus, dependency-injection framework, speculative
   abstraction or V2 enrollment is introduced by the refactoring.
2. **Git landing authority (scoped).** After review and validation, staging,
   responsibility-scoped commits, push to
   `cross-platform/automatic-pairing-reconcile-20261002`, one deliberate
   architecture-baseline reseal, and preparation of a formal Windows handoff are
   authorized for this plan and its tracking documents. **Not** authorized:
   `reset`, any `stash` operation, history rewriting, deleting recovery evidence,
   or overwriting the Windows Owner's working tree. `stash@{0}` stays untouched.
3. **V1-02c / V1-02d acceptance definition (revised).** These two targets are
   accepted on the strength of a source-level reachability proof for every
   legitimate construction path, plus negative tests at each *reachable* fence.
   No permit-constructor, destination setter or other authorization bypass may be
   added to manufacture a mismatch case, and "not reachable" must never be
   recorded as a production `PASS`. The two targets stay open until that
   reachability proof and the reachable-fence negative tests are recorded
   together; when written up they are reported as
   `NOT_APPLICABLE (no reachable path)` with the proof attached, never inferred
   from a neighbouring target.
4. **Deferrals unchanged.** aria2 acceptance stays user-deferred / `NOT_RUN` on
   Linux; Windows-owned native work stays queued for one consolidated Git
   handoff after the non-Windows-dependent phases finish. Neither is a
   `WINDOWS_BLOCKING` interruption of Linux development.

### Documentation Gate for this continuation

The gate below must be complete before any business-code change in R5 onward.
It was executed on HEAD `59edcbfa...` and its result is recorded here rather
than only in the handoff.

| # | Item | State |
|---|---|---|
| 1 | Active cursor moved from the stale dirty-tree record to the current clean HEAD | DONE |
| 2 | R4 inventory corrected to **eleven** `commands/` modules including `logs.rs`, and to **78** declarations / **78** handler entries | DONE |
| 3 | R5 traits slice downgraded from "verified" to *implemented, equivalence unverified*, with the masking `EXIT=0` recorded | DONE |
| 4 | R5 responsibility split corrected: `handle_command`/`run_worker`/`JobRecord` stay with runtime/control; `running_transitions` described as a short table | DONE |
| 5 | Read-only re-verification on this HEAD: architecture check 0 findings, docs audit PASS, `git diff --check` clean, command surface 78/78 | DONE |
| 6 | Approved decisions and the revised V1-02c/d definition recorded above | DONE |
| 7 | Windows queue still claims R4–R7 pending | see P0 follow-up |

Prior environment findings retained beneath the gate, unchanged in meaning:

### Environment capability

- This shell's own cgroup membership is `/non-systemd`, which is root-owned and
  is **not** a delegated parent.
- Read-only inspection shows `/sys/fs/cgroup` mounted `rw`, a reachable systemd
  user manager, and `user@1000.service` reporting `Delegate=yes` with a
  user-owned cgroup directory.
- Therefore the earlier blanket claim that cgroup delegation is unavailable is
  withdrawn as an assumption, and the earlier claim that it is available under
  `systemd-run` is also **not** re-established for the exact fixture. The
  disposable delegated fixture lifecycle (create → launch → drain → remove) is
  `NEEDS_VERIFICATION` and must be probed before its dependent gates are
  classified.
- Capability never rewrites ownership or acceptance: delegated execution does
  not authorize changing system delegation, adding privileges, or treating a
  service launch alone as containment evidence.

### V1 gate status (current)

| Gate | Implementation | Verification | Reason / next evidence |
|---|---|---|---|
| V1-01 | IMPLEMENTED | PASS (2026-11-08) | `executor::tests::production_runner_finalizes_internal_v1_archive_after_registered_writer_drain` PASS 1/1 in a disposable delegated service. |
| V1-02a | IMPLEMENTED | PASS (2026-11-08) | Same production finalizer test covers the contained registered-writer authorization path; the uncontained refusal test also passes. |
| V1-02b | IMPLEMENTED | PASS (2026-11-08) | New `archive::completion_tests::registered_undrained_writer_is_refused_by_production_finalizer` PASS 1/1: the real `PendingArchivePublication::finalize` refuses a genuinely registered, undrained writer with `PUBLICATION_INCOMPLETE_DRAIN`, creates no PREPARED journal and no final directory, and the same owner authorizes after a real drain. |
| V1-02c / V1-02d | fences implemented | NOT_RUN | No reachable permit-bearing mismatch path identified; do not forge permits. |
| V1-03 | IMPLEMENTED (D2) | PASS (2026-11-08) | `executor::tests::production_runner_post_commit_database_failure_recovers_without_false_success` PASS 1/1 after fixing the validation ordering. |
| V1-04 | IMPLEMENTED | PASS (Storage layer) + PASS (runner, 2026-11-08) | Storage crash windows remain bound to their original source; the runner-boundary post-commit recovery now passes on this source. |
| V1-05 | IMPLEMENTED | PASS (2026-11-08) | On this source: `internal_v1_prepared_journal_wins_cancellation_arbitration`, `cancellation_arbitrates_against_the_archive_commit_boundary`, `running_cancel_signals_execution_and_fences_late_success`, `stale_attempt_success_records_no_completion_event_or_final_state` all PASS in the serial workspace run. |
| V1-06 | IMPLEMENTED | PASS (2026-11-08) | On this source: `inline_runner_operation_panic_closes_reply_and_releases_admission`, `inline_runner_operation_drops_owner_before_admission_on_success`, `retained_drain_success_releases_owner_only_after_shutdown`, `pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`, plus the attempt-facts ownership and staging-verification tests, all PASS. |
| V1-07 | IMPLEMENTED | PASS (2026-11-08) | New `executor::session_boundary_tests::runtime_replacement_joins_active_writer_then_runs_successor` PASS 1/1: the real `RuntimeState::replace_executor` does not complete while an admitted runner holds a registered writer, the writer drains, its domain is gone, and a successor operation then runs on the replacement executor. |
| V1-08 Sidecar | IMPLEMENTED | PASS (cancellation, 2026-11-08) | Fixture aligned to the production protocol (`download`, `INTERRUPTED`, no `shutdown_ack`). `contained_internal_v1_cancellation_drains_writer_domain` PASS 1/1 plus the full 6/6 contained suite; two non-cgroup protocol controls (`tests/internal_v1_protocol.rs`) PASS. |
| V1-08 aria2 | not in scope this phase | NOT_RUN | User-deferred; the real-aria2 test reports the executable as absent, which is tool absence rather than a product result. |
| V1-09 | n/a | PASS (2026-11-08) | `cargo test --workspace --no-fail-fast -- --test-threads=1` exit `0`, 852 passed / 0 failed / 24 ignored, bound to source `49931e4...` and patch fingerprint `b85077db...`.

### Fixed defects in this continuation

- `recover_internal_v1_on_runner` no longer short-circuits a `COMMITTED` +
  `COMPLETE` attempt before authorization and Storage verification. The no-op
  decision now happens after `authorize_committed_recovery` and
  `verify_committed_terminal_internal_v1` both succeed, so a damaged or
  mismatched committed archive fails closed instead of being reported as
  "already converged".
- The Sidecar fixture emitted a `shutdown_ack` event the protocol enum does not
  define, answered `extract` only (while the production InternalV1 transfer uses
  `download`), and reported cancellation with a fixture-local code/message. It
  now follows the production command/event contract and reports `INTERRUPTED`
  for download cancellation.
- The contained cancellation test now targets the exact active job/request and
  adds a wrong-job negative control proving an unrelated cancel neither stops
  the worker nor tears down its writer domain.

### Environment capability — confirmed

A bounded disposable delegated service was executed within the recorded
authorization and reported, inside its own delegated subtree:
`parent_writable=yes`, `mkdir=PASS`, `migrate=PASS`,
`populated 0 → 1 → 0`, `kill=PASS`, `rmdir=PASS`, exit `0`. The invoking shell's
own `/non-systemd` membership remains root-owned and is not a delegated parent,
so probes must continue to run inside such a service.

### Remaining work matrix

- **Completeness note:** every applicable Sidecar-scoped V1 gate carries PASS
  evidence produced on the source that produced it, except V1-02c/V1-02d and the
  user-deferred aria2 scope. V1-02c/V1-02d now follow the revised acceptance
  definition recorded above: a reachability proof plus negative tests at each
  reachable fence, reported as `NOT_APPLICABLE (no reachable path)`, never as a
  production PASS. No permit may be forged to create a case.
- **Landed and no longer pending:** R0 (policy, module template, Node ESM
  architecture checker with `node:test` coverage, baseline, CI workflow,
  `AGENTS.md` routing), R1's three extractions, and the `ca5c99a1` baseline reseal
  are **committed**. They are not part of any pending landing batch and must not be
  re-implemented or re-baselined.
- **Phase state:** R0–R4 **COMPLETE** (committed). R5 **IN PROGRESS** — contracts
  and traits extracted, and the traits equivalence audit still open. R6 **PARTIAL**
  — metadata and command builders landed. R7 **PENDING**.
- **Immediately executable (Linux-owned), in this order:**
  1. R5 slice 2a — close the traits equivalence audit, then continue the remaining
     five slices (containment → factory → startup recovery → runtime/control →
     session).
  2. R6 — Sidecar responsibility review of the three runners and their shared
     subprocess lifecycle.
  3. R7 — integration, reviewed baseline delta, and handoff candidate.
- **Completed in this goal:** R2 (Storage cohesion including the production
  manifest/journal separation), R3 (Telegram delivery cohesion) and **R4 (the full
  Desktop Tauri command surface: eleven `commands/` modules holding 74 commands
  plus 4 in `aria2.rs`; `commands.rs` reduced from 4103 to 73 lines and now a
  module root; 78 declarations and 78 registered handler entries).**
- **Independent, may run alongside the above:** the Telegram restoration
  body-level audit.
- **Already resolved, removed from the open list:** isolation of test temporary
  directories — the two Desktop download-verification tests that wrote
  `outside.jpg` into the shared temporary directory now use a per-run
  `temp_case_root`.
- **Authorization state changed:** staging, responsibility-scoped commits, push to
  this branch, one reviewed baseline reseal and preparation of a formal Windows
  handoff are now **approved** for the R5–R7 plan and its tracking documents.
  Still excluded: `reset`, `stash` operations, history rewriting, deleting
  recovery evidence, overwriting the Windows Owner's tree. The baseline can only
  be resealed against a tracked file set, so that reseal belongs to R7.
- **Windows-owned:** native writer-drain adapter, process-tree containment and
  join, NTFS/no-replace and coordination-lock semantics, MSVC build, GUI,
  packaging, and Windows validation. These need a formal Git handoff and are
  never satisfied by Linux results. No `WINDOWS_BLOCKING` item exists today.



The sections below are historical evidence bound to their own source revisions.

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
authorization explicitly permit it. Reconcile the required capability before
running any dependent gate:

- A root control file's status alone does not establish whether a delegated
  child is usable; verify the exact production create → launch → drain → remove
  operations inside the disposable parent. Report the failed operation, not a
  boolean.
- This shell's own membership `/non-systemd` is root-owned and is not a
  delegated parent. The systemd user manager is reachable and
  `user@1000.service` reports `Delegate=yes`.
- A `systemd-run --user --property=Delegate=yes` launch proves service launch
  only. Until a bounded disposable delegated-service probe/fixture is actually
  executed and recorded, keep the dependent gates `NEEDS_VERIFICATION`, not
  PASS and not BLOCKED.
- If a required capability is genuinely unavailable, record the failed
  operation and continue independent checks. Do not claim an equivalent result
  from a weaker test, and do not elevate privileges, modify system delegation,
  install dependencies, or configure external execution without applicable
  authorization.

Environment and fixture preparation may proceed independently of the contract
when it does not modify system delegation or install dependencies.

Identify the supported aria2 executable/version and artifact identity when that
phase is resumed; an executable alone is not registered-writer/drain/recovery
acceptance.

Production-runner validation must not overwrite or unlink the current untracked
`desktop/src-tauri/job-1.lock`. Before running an integration test, trace
`ProductionCoordinator::open` from the exact fixture database path, then inspect
the derived `locks/` entry and verify its identity and intended effect. A unique
database path is not enough to establish lock isolation. If the test would
mutate or replace the retained artifact, use only an already-approved isolated
worktree; do not remove, rename, or redirect the lock as an ad hoc workaround.


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

Status: **DONE (2026-11-08)** except for one coupled landing step noted below.

- Captured the accepted V1 evidence and public contracts before moving code: see
  `docs/validation/linux-crash-replay-20261016.md` (`linux-v1-batch-20261108`).
- Added `docs/architecture/MODULAR_DESIGN_POLICY.md` and a routing reference in
  root `AGENTS.md`; added the module description template at
  `docs/architecture/module-template.md`.
- Added the architecture checker `scripts/check-architecture.mjs` with policy
  (`scripts/architecture-policy.json`), reviewed exceptions
  (`scripts/architecture-exceptions.json`), reviewed baseline
  (`scripts/architecture-baseline.json`) and `node:test` coverage in
  `scripts/check-architecture.test.mjs` (12/12 PASS). The checker reports file
  size categories, baseline deltas, renames, JS/TS relative-import cycles,
  declared boundary violations and unresolvable imports. It fails closed
  (exit 2) when the policy is missing and never regenerates the baseline in CI.
- Added the CI job `.github/workflows/architecture-check.yml` and root scripts
  `arch:check`, `arch:test`, `docs:audit`.
- Indexed the checker and policy in `docs/architecture/repository-map.md`.
- Current real-tree result: `architecture-check: 144 files, 42 js/ts, 22 relative
  edges, 0 findings`, exit 0. Reviewed exceptions exist for
  `crates/xarchive-storage/src/lib.rs` (R2),
  `crates/xarchive-storage/src/archive_recovery.rs` (R2),
  `crates/xarchive-telegram/src/lib.rs` (R3),
  `desktop/src-tauri/src/commands.rs` (R4) and
  `desktop/src-tauri/src/executor.rs` (R5).

**Coupled landing step:** the checker only counts tracked files, so the very
commit that first tracks `scripts/check-architecture.mjs` and its test must also
run `node scripts/check-architecture.mjs --write-baseline` once and commit the
resulting baseline. Doing it later leaves two `baseline-missing-entry` findings.


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

### Phase R1 — COMPLETE (2026-11-08)

All three R1 items landed; see item 1, item 2 and item 3 below. R2 is next.

**Item 1 — executor specification component: DONE (2026-11-08).**

- Extracted `ExecutionSpecV2`, `ExecutionSpecV3`, `ExecutionSpecV4`,
  `MetadataOutputSettings`, `DownloaderArgumentSnapshot`, `ToolCompatibility`,
  `DecodedExecutionSpec`, `ExecutionSpecError`, `decode_execution_spec` and
  `validate_downloader_argument_snapshot` from `desktop/src-tauri/src/executor.rs`
  into `desktop/src-tauri/src/execution_spec.rs`.
- `crate::executor` re-exports the public items, so every existing call site and
  every serialized payload is unchanged. `decode_execution_spec` still returns
  the same `Result` and the same `Display` text.
- The module owns one responsibility (decode a persisted spec into validated
  business inputs) and holds no I/O, session, owner, permit or writer state.
- Two lint annotations were added deliberately and are commented in place: the
  `ExecutionSpecError` payload (retained contract, only printed by `Debug`) and
  `MetadataOutputSettings::serialize_metadata` (D1 export contract whose
  production caller is not wired yet and which the `d1_contract_tests` module
  exercises). No blanket file-level allow was introduced.
- Validation after the move: `cargo check -p xarchive-desktop --lib` reports only
  the pre-existing `record_recovery_results` warning; `cargo test -p
  xarchive-desktop --lib -- --test-threads=1` PASS 365 / 13 ignored; the V1
  production gates re-ran in a disposable delegated service —
  `production_runner_finalizes_internal_v1_archive_after_registered_writer_drain`
  PASS, `production_runner_post_commit_database_failure_recovers_without_false_success`
  PASS, `registered_undrained_writer_is_refused_by_production_finalizer` PASS,
  `runtime_replacement_joins_active_writer_then_runs_successor` PASS. The
  real-aria2 test still reports its executable absent (tool absence).

**Item 2 — Telegram formatting/planning/outbox separation: DONE (2026-11-08).**

- `crates/xarchive-telegram/src/formatting.rs` now owns the pure message and
  media planning responsibility: `MetadataInput`, `format_metadata`,
  `split_text`, `split_for_telegram`, `MediaKind`, `UploadMode`,
  `MediaGroupSend`, `classify_media`, `stable_upload_file_name`,
  `media_send_plan`, `media_groups` and the two private naming helpers.
- `crates/xarchive-telegram/src/projection.rs` now owns the TG-06 delivery-state
  projection: `SendProjection` (with `label()`), `outbox_projection` and
  `message_link`.
- The crate root keeps `pub use crate::formatting::{...}` and
  `pub use crate::projection::{...}`, so every existing path
  (`xarchive_telegram::format_metadata`, `…::outbox_projection`,
  `…::message_link`, `…::MediaKind`, …) is unchanged, as are the
  `snake_case` serde representation of `MediaKind`/`UploadMode` and every
  rendered string.
- Timeouts, cancellation, credential rotation, outbox leases and
  uncertain-send outcomes were deliberately **not** moved: no resource
  ownership, lease, permit or transport state changed, so no V1 publication or
  writer gate is affected.
- Module descriptions: `docs/architecture/modules/telegram-formatting.md` and
  `docs/architecture/modules/telegram-projection.md`.
- Validation: `cargo test -p xarchive-telegram -- --test-threads=1` PASS 54/54
  (the five formatting tests and two projection tests now run as
  `formatting::tests::*` and `projection::tests::*`), `cargo test -p
  xarchive-desktop --lib -- --test-threads=1` PASS 365 / 13 ignored,
  `cargo fmt --all -- --check` and `git diff --check` PASS.
- **Known baseline state (deliberate):** `crates/xarchive-telegram/src/lib.rs`
  shrank 5962 → 5535 lines, so `node scripts/check-architecture.mjs` reports one
  `baseline-delta` finding. This is expected while `formatting.rs` and
  `projection.rs` are untracked — the checker only baselines `git ls-files`
  entries — and it must be closed by the landing commit, which tracks both new
  modules and runs `--write-baseline` once in that same commit. Do not re-seal
  the baseline before the files are tracked: that would replace this finding
  with two `baseline-missing-entry` findings.

**Item 3 — Sidecar gallery metadata and command construction: DONE (2026-11-08).**

- `sidecar/src/xarchive_downloader/gallery_metadata.py` now owns reading and
  normalizing gallery-dl output: `GALLERY_DL_JSONL`, `GalleryDlJsonlParser`,
  `tweet_id_from_url`, `metadata_paths`, `read_gallery_jsonl`,
  `read_gallery_jsonl_matching`, `read_metadata_candidates`,
  `read_metadata_matching`, `purge_metadata_files`, `is_metadata_file`,
  `media_type_for_filename`, `sanitize_filename`, `stable_media_id`,
  `to_extraction_result`, `to_discovery_candidate`.
- `sidecar/src/xarchive_downloader/gallery_commands.py` now owns the invocation
  description: `ExtractionConfig`, `PROXY_ENVIRONMENT_KEYS`, `spawn_env`,
  `build_extraction_command`, `build_download_command`.
- `extraction.py` keeps the subprocess lifecycle — staging, timeout,
  cancellation, bounded output capture, `purge_downloaded_files`,
  `ExtractionRunner`, `DiscoveryRunner`, `DownloadRunner` — and re-exports every
  moved name, so `worker_v2` and all existing tests keep the same import path
  and payload contract.
- Module descriptions:
  `docs/architecture/modules/sidecar-gallery-metadata.md` and
  `docs/architecture/modules/sidecar-gallery-commands.md`.
- Validation: `python3 -m pytest sidecar/tests -q` PASS 101/101 (same total as
  before the split), `python3 -m compileall -q sidecar/src/xarchive_downloader`
  PASS, `git diff --check` PASS. `ruff` is not installed in this environment, so
  no lint result is claimed for this batch; unused imports were instead audited
  with an AST pass and the compatibility re-exports use the explicit
  `name as name` form.
- **Known baseline state (deliberate):** `extraction.py` shrank 859 → 430 lines.
  Together with the Telegram delta, `node scripts/check-architecture.mjs`
  reports exactly two `baseline-delta` findings, both caused by the new modules
  being untracked. The landing commit tracks the four new module files and runs
  `--write-baseline` once.


- Extract execution-spec decoding and downloader snapshot validation from
  `desktop/src-tauri/src/executor.rs` into an executor specification component.
- Separate Telegram metadata formatting, text/media planning, and outbox
  transition policy inside `crates/xarchive-telegram`.
- Separate Sidecar gallery metadata normalization and command construction
  from `sidecar/src/xarchive_downloader/extraction.py`.
- Retain compatibility re-exports and unchanged serialization/error behavior.

### Phase R2 — Storage cohesion

Status: **COMPLETE (2026-11-08)** — assembly, connection, credential, intent,
test-layout and production manifest/journal separation have all landed; every
Storage source is under the library limit.

> **R2 closure (2026-11-08).** R2 was previously PARTIAL because only the
> test-module move had landed while the committed R2 scope also requires the
> production manifest/journal separation. That separation is now implemented:
> `archive_recovery.rs` 1415 → 1098 lines, with the pure manifest contracts
> moved to `archive_recovery_manifest.rs` (332 lines). The remaining file owns
> only journal persistence, startup dispatch, cancellation arbitration and the
> V1/V2 recovery operations, which share the `Database` connection and the same
> transaction boundary. Splitting those further would either duplicate the
> connection or break the atomic boundary, so the residual cohesion is
> deliberate and recorded rather than deferred as debt.

**Landed (2026-11-08).**

**Production manifest/journal separation (2026-11-08, R2 closure batch).**

- `crates/xarchive-storage/src/archive_recovery_manifest.rs` (new, 332 lines)
  owns the pure manifest contracts: `ArchiveRecoveryMediaFact`,
  `ArchiveRecoveryManifest`, `ArchiveRenamePlan`, `ArchiveRecoveryManifestV2`,
  `MetadataExportSelection`, their `validate` implementations, `from_metadata`,
  `digest`, and the private `is_reserved_export_path` / `validate_recovery_path`
  helpers. It opens no connection and holds no transaction.
- `crates/xarchive-storage/src/archive_recovery.rs` dropped 1415 → 1098 lines and
  now owns journal persistence, startup dispatch, cancellation arbitration and the
  V1/V2 recovery operations. The crate root re-exports both modules, so no import
  path, serde representation, schema version or digest computation changed.
- The atomic boundary is untouched: `recover_internal_v1_rows_and_commit_manifest`
  still applies recovered rows and the PREPARED→COMMITTED compare-and-set inside a
  single `unchecked_transaction` with one `commit()`.
- Validation: `cargo test -p xarchive-storage --lib` PASS **178/178**;
  `cargo test -p xarchive-desktop --lib` PASS **365 / 13 ignored**;
  `cargo test -p xarchive-workflow --lib` PASS **49/49**;
  `cargo check -p xarchive-storage --lib` reports **no warnings**.
- Equivalence audit: a declaration-level diff of the pre-split file against the
  two current files reports **no lost declarations, no new names, zero
  body-differing declarations and no occurrence-count changes**. Output preserved
  as `.tw2tg-tmp/agent-scratch/build-logs/r2-manifest-equiv.txt`.
- Module description:
  `docs/architecture/modules/storage-archive-recovery-manifest.md`.

- `crates/xarchive-storage/src/lib.rs` is now module assembly: module
  declarations, re-exports, the `Database` wrapper, `execute_batch` and the
  test-only connection accessor (93 lines). The former 6355-line file mixed
  migrations, credential persistence, intent journal and the whole regression
  matrix.
- `crates/xarchive-storage/src/connection.rs` owns connection opening and the
  ordered migration list (`open`, `open_in_memory`, `prepare`, `MIGRATIONS`).
  Migration order, pragmas and the per-migration transaction are unchanged.
- `crates/xarchive-storage/src/telegram_credentials.rs` owns the credential
  generation fence: the twelve generation/rotation/resume/claim methods, moved
  verbatim with their SQL and transaction boundaries.
- `crates/xarchive-storage/src/telegram_archive_intents.rs` owns the archive
  intent journal: `prepare_telegram_archive_intent`, `telegram_archive_intent`,
  `transition_telegram_archive_intent`,
  `list_recoverable_telegram_archive_intents`.
- `crates/xarchive-storage/src/tests/` replaces the inline test module: `mod.rs`
  holds the shared fixtures and six subject groups (`archive_recovery`,
  `archive_service`, `database`, `database_telegram`, `telegram_credentials`,
  `telegram_outbox`). Test leaf names are unchanged; the module path gains the
  group name. The one suite that re-executes itself for a crash window now
  passes its fully qualified name, so the child still runs that exact suite.
- Module descriptions: `docs/architecture/modules/storage-connection.md`,
  `storage-telegram-credentials.md`, `storage-telegram-archive-intents.md`,
  `storage-tests.md`.
- The `crates/xarchive-storage/src/lib.rs` entry was removed from
  `scripts/architecture-exceptions.json` because the file is no longer over the
  library limit; keeping a stale exception would misreport the debt.
- Validation: `cargo test -p xarchive-storage --lib -- --test-threads=1` PASS
  **178/178**, the same total as before the split, and the pre-split test-name
  set is a subset of the post-split one after normalising the group prefix.

**Archive recovery split (2026-11-08).**

- `crates/xarchive-storage/src/archive_recovery.rs` dropped from 4795 to 1415
  lines by moving its two inline test modules out; production manifest/journal
  types, SQL and transaction boundaries were not touched.
- `crates/xarchive-storage/src/tests/archive_recovery_manifest.rs` (9 suites)
  owns manifest contracts, rename plans and verification;
  `crates/xarchive-storage/src/tests/archive_recovery.rs` (22 suites) owns
  dispatch, attempt claims, journal replay and crash windows. Shared fixtures
  (`test_candidate_metadata`, `manifest`) moved to `tests/mod.rs`.
- The 28 V2 forward-publish / rename-recovery / D1-export suites that belong to
  the archive service were restored into `tests/archive_service.rs`; the first
  mechanical pass had dropped them and the 178-suite total exposed it.
- Both self-re-executing crash-window suites now pass their fully qualified
  module path, so the aborted child runs the same suite after the move.
- The `crates/xarchive-storage/src/archive_recovery.rs` entry was removed from
  `scripts/architecture-exceptions.json`: the file is now under the limit, so
  keeping the exception would misreport the debt.
- Validation: `cargo test -p xarchive-storage --lib` PASS **178/178** with a
  test-name set identical to the pre-split one after normalising group prefixes
  (0 missing, 0 extra); `cargo test -p xarchive-desktop --lib` PASS 365 / 13
  ignored; `cargo fmt --all -- --check` and `git diff --check` PASS.

### Phase R3 — Telegram delivery cohesion

Status: **COMPLETE (2026-11-08)** — the crate root is now module assembly plus the
secret-store, cross-cutting constant and TG-06 projection surfaces; every
delivery responsibility sits behind a cohesive module.

**Landed (2026-11-08).** `crates/xarchive-telegram/src/lib.rs` 5962 → 2691 lines.

Module inventory after R3:

| Module | Lines | Responsibility |
|---|---|---|
| `contract.rs` | 435 | Bot API wire contract, token handle, error type, endpoint policy |
| `formatting.rs` | 356 | metadata formatting, text/media planning primitives |
| `retry_policy.rs` | 163 | TG-04 failure classification and outbox decision |
| `delivery.rs` | 221 | send state, persisted send records, idempotent send |
| `transport.rs` | 350 | blocking control client, endpoint enforcement, payloads |
| `upload.rs` | 1033 | layered timeouts, streaming body, album watchdog |
| `outbox.rs` | 441 | outbox lifecycle, store contract, claimed-attempt driver |
| `planning.rs` | 278 | send planning, fingerprints, payload snapshot |
| `file_cache.rs` | 90 | TG-05 cache key, record, store contract, invalidation |
| `projection.rs` | 118 | TG-06 task wording and message deep link |

**Second batch (2026-11-08).** `outbox.rs`, `planning.rs` and `file_cache.rs`
were extracted verbatim; the crate root re-exports all three. No public item
gained visibility outside the crate, no signature, serde representation, error
variant or constant value changed.

- Validation: `cargo test -p xarchive-telegram -- --test-threads=1` PASS
  **54/54**; `cargo test -p xarchive-desktop --lib -- --test-threads=1` PASS
  **365 / 13 ignored**; `cargo check -p xarchive-telegram --lib` and
  `cargo test -p xarchive-telegram --no-run` report **no warnings**;
  `cargo fmt --all -- --check` and `git diff --check` PASS; architecture checker
  self-tests PASS 12/12.
- Equivalence audit: a declaration-level diff of the pre-split `lib.rs` against
  the whole current module set reports **no lost declarations** and **zero
  body-differing declarations** after normalising visibility and whitespace.
  Output preserved as
  `.tw2tg-tmp/agent-scratch/build-logs/p4-telegram-equiv-r3final.txt`.
- `crates/xarchive-telegram/src/lib.rs` was removed from
  `scripts/architecture-exceptions.json`: at 2691 lines it is under the
  `rust-library` limit of 3000, so keeping a named exception would misreport
  debt. The `commands.rs` (R4) and `executor.rs` (R5) exceptions remain.
- Module descriptions: `telegram-outbox.md`, `telegram-planning.md`,
  `telegram-file-cache.md`.
- Architecture checker still reports the same four `baseline-delta` findings; the
  Telegram delta is now -3271 lines. They do not become PASS until the landing
  commit tracks the new modules and reseals the baseline.

**What R3 deliberately did not move.** The secret-store contract
(`SecretStore`, `SecretStoreError`, `MemorySecretStore`, `validate_key`), the
cross-cutting constants (`TELEGRAM_TEXT_LIMIT`, `TELEGRAM_MEDIA_GROUP_LIMIT`,
`TELEGRAM_ALBUM_MIN_ITEMS`, `TELEGRAM_CAPTION_LIMIT`, the two upload ceilings),
`TelegramError::retry_after`, the three `UNKNOWN_REASON_*` constants, `sha256_hex`
and the `tests` module stay at the crate root. `sha256_hex` is the shared
fingerprint primitive named by both `contract.rs` and `planning.rs`, and the
`UNKNOWN_REASON_*` strings are named by both the outbox driver and the Desktop
review surface. Moving a single-consumer helper would have created an empty
wrapper, which section 7 forbids.

**Incident in the first R3 batch (recorded, not hidden).** Mid-batch an
`git checkout -- crates/xarchive-telegram/src/lib.rs` reverted the file to HEAD
and dropped the R1 split. Every removed span had been saved in the preserved
scratch directory, the file was rebuilt from those copies plus the extracted
modules, seven inline tests that then existed twice were removed, and the crate
returned to exactly **54/54** with no duplicate names and green formatting/diff
checks. No committed state and no other file was affected; the working tree was
never staged.

**Projection wiring repair (2026-11-08, Documentation Gate).** The rebuild after
the `git checkout` incident restored `lib.rs` with an inline copy of
`SendProjection` / `outbox_projection` / `message_link` while `projection.rs`
also defined them. The crate root only declared `mod projection;`, so the module
was dead code and the four public items produced `dead_code` warnings. The
duplicate inline definitions were removed and the crate root now re-exports
`pub use crate::projection::{SendProjection, message_link, outbox_projection};`,
matching the module documentation. Recorded because the earlier R1 record
claimed the re-export already existed; that claim did not hold for the rebuilt
file.


**Batch history (retained, not rewritten).** R3 landed in three passes on this
source: (1) `retry_policy.rs`, `contract.rs` and `delivery.rs`, taking `lib.rs`
from 5962 to 4820 lines; (2) `transport.rs` and `upload.rs`, taking it to 3439;
(3) `outbox.rs`, `planning.rs` and `file_cache.rs`, taking it to 2691. Each pass
kept the crate-root re-exports, re-ran the Telegram and Desktop suites, and left
the public contract unchanged. The only visibility changes were `pub(crate)` on
helpers named by a sibling module or by the crate's own test module.



- Split `crates/xarchive-telegram/src/lib.rs` by existing contracts,
  formatting/planning, outbox policy, transport, and upload responsibilities.
- Keep Desktop orchestration, Tauri state, Storage adapters, and platform
  credential adapters at their existing ownership boundaries.
- Validate timeouts, cancellation, credential rotation, and uncertain send
  outcomes after each extraction.

### Phase R4 — Desktop command/configuration boundaries

Status: **IN PROGRESS (2026-11-08)** — the Telegram command domain has been
extracted; jobs, batches, settings, records and the remaining surfaces are still
in `commands.rs`.

**Telegram command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/telegram.rs` (new, 510 lines) owns the 13
  Telegram commands, `TelegramTaskProjection`, `TelegramRuntimeProjection` and
  the private `telegram_progress_label` helper.
- `desktop/src-tauri/src/commands.rs` dropped 4103 → 3611 lines and declares
  `pub(crate) mod telegram;`.
- **IPC contract preserved.** `lib.rs::generate_handler!` now names
  `commands::telegram::<name>` for those 13 entries. This is required, not
  stylistic: `#[tauri::command]` expands to hidden `__cmd__<name>` and
  `__tauri_command_name_<name>` items in the defining module, and those are not
  reachable through a `pub use` re-export. Re-exporting the functions and leaving
  `commands::<name>` in the handler list fails to compile with "cannot find
  `__cmd__...` in `commands`". The registered command **names** are unchanged, so
  the frontend is unaffected.
- Validation: `cargo test -p xarchive-desktop --lib -- --test-threads=1` PASS
  **365 / 13 ignored**; `cargo check -p xarchive-desktop --lib` reports only the
  pre-existing `record_recovery_results` warning.
- Equivalence audit: declaration diff reports **no lost declarations, no new
  names, zero body-differing declarations, no occurrence-count changes**, and the
  command inventory is **74 before and 74 after** with none lost or added and no
  handler name unresolvable. Output preserved as
  `.tw2tg-tmp/agent-scratch/build-logs/r4-telegram-equiv.txt`.
- Module description: `docs/architecture/modules/desktop-commands-telegram.md`.

**Batches command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/batches.rs` (new, 334 lines) owns the 8 account
  batch commands: create, list, get, list candidates, pause, resume, cancel and
  retry.
- `desktop/src-tauri/src/commands.rs` dropped 3611 → 3297 lines. `open_batch_database`
  moved back to the parent as `pub(crate)` because eight non-batch call sites in
  `commands.rs` still use it; keeping it in the child would have required the
  parent to import from a child while the child also imports from the parent.
  `CreateAccountBatchRequest` was already `pub(crate)` before the slice. The four
  settings helpers (`read_output_settings`, `read_downloader_settings`,
  `snapshot_downloader_settings`, `validate_output_settings`) were temporarily
  widened during the slice and then **restored to private**: a child module can
  reach its parent's private items, so the widening was unnecessary.
- The same handler-path rule applied: `generate_handler!` now names
  `commands::batches::<name>` for those 8 entries, and the names were removed from
  the `use commands::{...}` import list.
- Validation (`2026-11-08`, full logs and exit codes preserved):
  `cargo check -p xarchive-desktop --lib` exit `0` with only the pre-existing
  `record_recovery_results` warning; `cargo test -p xarchive-desktop --lib --
  --test-threads=1` exit `0`, PASS **365 / 13 ignored**. Subsystem re-check on the
  same tree: Storage lib 178/178 exit `0`, Telegram 54/54 exit `0`, Workflow lib
  49/49 exit `0`, architecture checker self-tests 12/12 exit `0`,
  `cargo fmt --all -- --check` exit `0`, `git diff --check` exit `0`, docs audit
  PASS exit `0`. Evidence:
  `.tw2tg-tmp/agent-scratch/build-logs/r4-batches-validation.log` and
  `r4-batches-subsystems.log`.
- Contract audit: **74** `#[tauri::command]` functions before and after,
  **78** unique `generate_handler!` entries with **no duplicates**, **no**
  registered handler without a source function, and **no** source command left
  unregistered. (Four handler entries — `detect_aria2`, `download_aria2`,
  `list_aria2_releases`, `validate_aria2_path` — come from the pre-existing
  `aria2.rs` module, not from `commands`.)
- Slice equivalence evidence: the report at
  `.tw2tg-tmp/agent-scratch/build-logs/r4-batches-equiv.json` records, per
  command, the full signature, the `generate_handler!` entry, the exact
  user-visible error/message strings, the serialized shape of
  `CreateAccountBatchRequest`, and a lexical body comparison against the pre-slice
  snapshot `r4-commands-before-batches.rs` (all MATCH, `result: PASS`). The
  comparison normalises visibility and whitespace only; it is supporting evidence,
  not a semantic proof. Generator:
  `.tw2tg-tmp/agent-scratch/migration-scripts/r4_batches_contract.py`.
- Test-isolation fix in the same batch: the two `production.rs` download
  verification tests wrote a deliberate escaping file to the shared system
  temporary directory. They now use a per-run case root with `staging/` beneath
  it, and the cleanup removes the whole case root. Verified by comparing
  `/tmp/outside.jpg` mtime+size across the `production::tests` run (unchanged,
  `shared_temp_not_touched`); the legacy file was then removed.

**Extension command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/extension.rs` (new, 345 lines) owns the five
  extension commands: `register_native_host`, `unregister_native_host`,
  `import_extension_directory`, `get_extension_status`, `open_extension_folder`.
  `ExtensionStatus` and the five private helpers that only the extension domain
  uses (`extension_status_from_state`, `validate_extension_directory`,
  `sha256_directory_marker`, `collect_files`, `copy_directory_contents`) moved with
  it.
- `commands.rs` 3297 → 2969 lines. No visibility was widened: the helpers became
  module-private in the child.

**Logs command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/logs.rs` (new, 80 lines) owns
  `log_frontend_event`, `read_application_logs` and `open_log_folder` plus the
  `FrontendDiagnosticEvent` request type. `log_frontend_event` is the P2-A
  redaction boundary, and it stays inside this domain rather than being split out.
- `bounded_diagnostic` deliberately stayed in `commands.rs` because the parent's
  redaction test also asserts it; the child imports it through the parent.
- `commands.rs` 2969 → 2903 lines.

**Records command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/records.rs` (new, 1097 lines) owns the seven
  record/configuration exchange commands and their full support surface: the
  request/response types, `RecordImportDocument` with its parser, the
  `RecordImportOutcome` conversion, `RECORD_IMPORT_CANCELLATION` plus
  `RecordImportWait`, the four serializers, `record_import_root`,
  `record_import_revision`, `record_import_media_probe`,
  `verify_accepted_media_evidence`, and the `record_import_acceptance_tests`
  module.
- `save_output_settings`/`validate_output_settings` stayed in the parent because
  the batch and Telegram domains also use them.
- `commands.rs` 2903 → 1824 lines.
- **First attempt corrupted `commands.rs` and was reverted from backup.** The span
  scanner produced two overlapping ranges (`RecordImportWait` and its adjacent
  `impl Drop`, and `serialize_record_export` matching both with and without its
  `#[cfg(test)]` attribute), so double removal spliced the file mid-token. The
  migration script now merges the struct+impl span into one, takes the attributed
  span once, and **aborts on any overlapping span**. The file was restored from
  `.tw2tg-tmp/agent-scratch/source-spans/r4-commands-before-records.rs` (2903
  lines) and the slice was redone from that clean state.

**Sidecar lifecycle command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/sidecar.rs` (new, 119 lines) owns
  `start_sidecar`, `stop_sidecar` and `sidecar_configuration`.
- Deliberately left in the parent: `sidecar_status` and `refresh_sidecar_state`
  (used by `app_status`) and `parse_sidecar_args` (`lib.rs` and `runtime.rs` reach
  it as `commands::parse_sidecar_args`). The child imports the latter through the
  parent, so no visibility was widened and no re-export shim was added.
- `commands.rs` 1824 → 1719 lines (36 commands left).
- Validation: `cargo check -p xarchive-desktop --lib` exit `0` with only the
  pre-existing warning; `cargo test -p xarchive-desktop --lib -- --test-threads=1`
  PASS **365 / 13 ignored**; contract audit unchanged at 74 commands / 78 unique
  handler entries; equivalence audit PASS with all three items body-identical and
  absent from the parent
  (`.tw2tg-tmp/agent-scratch/build-logs/r4-sidecar-equiv.txt`).
- Module description: `docs/architecture/modules/desktop-commands-sidecar.md`.

**Executor job command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/executor.rs` (new, 240 lines) owns
  `submit_executor_job`, `query_executor_job`, `cancel_executor_job`,
  `shutdown_executor`, the two response types and the two private adapters
  (`executor_error`, `job_response`).
- Deliberately left in the parent: `list_jobs`, `list_jobs_page`,
  `get_job_history_index`, `get_job_download_metrics` and `get_job_metrics` read
  the persisted read-model directly and never take the executor service handle, so
  they are a different responsibility.
- `commands.rs` 1719 → 1497 lines (32 commands left).
- Validation: `cargo check -p xarchive-desktop --lib` exit `0` with only the
  pre-existing warning; `cargo test -p xarchive-desktop --lib -- --test-threads=1`
  PASS **365 / 13 ignored**; contract audit unchanged at 74 commands / 78 unique
  handler entries; equivalence audit PASS with all eight items body-identical and
  absent from the parent, and the read-model commands confirmed still present
  (`.tw2tg-tmp/agent-scratch/build-logs/r4-executor-equiv.txt`).
- Module description: `docs/architecture/modules/desktop-commands-executor.md`.

**Archive/setup command slice (2026-11-08).**

- `desktop/src-tauri/src/commands/archive.rs` (new, 282 lines) owns
  `get_archive_root`, `get_portable_setup`, `complete_download_setup`,
  `set_archive_directory`, `validate_gallery_dl_path`, `save_gallery_dl_path`,
  `copy_text_to_clipboard`, `open_archive_folder`, the private helpers
  `validate_archive_directory` and `gallery_dl_version`, and the
  `archive_directory_tests` module.
- Kept in the parent: `AppStatus`/`app_status` (shared by settings and executor
  paths) and `GalleryDlInstallation`; the child imports both.
- `commands.rs` 1497 → 1229 lines (24 commands left).
- Validation: `cargo check -p xarchive-desktop --lib` exit `0` with only the
  pre-existing warning; `cargo test -p xarchive-desktop --lib -- --test-threads=1`
  PASS **365 / 13 ignored**; contract audit unchanged at 74 commands / 78 unique
  handler entries; equivalence audit PASS — all ten items body-identical, absent
  from the parent, the eight commands registered as `commands::archive::<name>`,
  the two private helpers not registered, and the test module moved
  (`.tw2tg-tmp/agent-scratch/build-logs/r4-archive-equiv.txt`).
- Module description: `docs/architecture/modules/desktop-commands-archive.md`.

**App status command slice (2026-11-08) — R4 COMPLETE.**

- `desktop/src-tauri/src/commands/app.rs` (new, 140 lines) owns
  `get_app_status`, `get_component_bootstrap_status`, `get_runtime_health`, the
  `AppStatus` / `PortableSetup` / `GalleryDlInstallation` /
  `CreateAccountBatchRequest` types and the `app_status` / `sidecar_status`
  helpers.
- `create` and the batch types now import from `commands::app`;
  `commands/archive.rs` and `commands/settings.rs` import `AppStatus` and
  `app_status` from there.
- **`commands.rs` 1121 → 73 lines and now holds no `#[tauri::command]` at all.**
  It remains only as the module root plus four shared items:
  `open_batch_database`, `parse_sidecar_args`, `refresh_sidecar_state`,
  `bounded_diagnostic`, and the `frontend_diagnostic_tests` module.
- Validation: `cargo check -p xarchive-desktop --lib` exit `0`;
  `cargo test -p xarchive-desktop --lib -- --test-threads=1` PASS
  **365 / 13 ignored**; contract unchanged at **74** commands / **78** unique
  handler entries, no duplicates, no unregistered command; equivalence audit PASS
  with all nine items body-identical, the three commands registered as
  `commands::app::<name>`, and the parent confirmed to keep its shared helpers
  (`.tw2tg-tmp/agent-scratch/build-logs/r4-app-equiv.txt`).
- Module description: `docs/architecture/modules/desktop-commands-app.md`.

**R4 is COMPLETE.** Ten command domains now hold all 74 commands:

| Module | Lines | Commands |
|---|---|---|
| `commands/telegram.rs` | 510 | 13 |
| `commands/records.rs` | 1097 | 7 |
| `commands/settings.rs` | 939 | 16 |
| `commands/extension.rs` | 345 | 5 |
| `commands/batches.rs` | 334 | 8 |
| `commands/archive.rs` | 282 | 8 |
| `commands/executor.rs` | 240 | 4 |
| `commands/app.rs` | 140 | 3 |
| `commands/jobs.rs` | 123 | 5 |
| `commands/sidecar.rs` | 119 | 2 |
| `commands.rs` (root) | 73 | 0 |

No file exceeds the `rust-desktop` `limitLines` of 4000, so
`desktop/src-tauri/src/commands.rs` no longer needs an over-limit exception. The
next batch removes that entry, since the file is now a module root rather than a
cohesion debt.


| Group | Count | Commands |
|---|---|---|
| settings | 15 | network/proxy (4): `get_network_settings`, `save_network_settings`, `inspect_proxy_route`, `get_system_proxy_status`; theme (2); downloader arguments (2); output + metadata output (4); application settings (1); aria2/sidecar path (2) |
| jobs read-model | 5 | `list_jobs`, `list_jobs_page`, `get_job_history_index`, `get_job_download_metrics`, `get_job_metrics` |
| app status | 4 | `get_app_status`, `get_component_bootstrap_status`, `get_runtime_health` |

All three groups are now extracted (`commands/settings.rs`, `commands/jobs.rs`,
`commands/app.rs`), so this table is a historical inventory of the R4 tail rather
than remaining work. Next: reconsider `config.rs` (1814 lines) and
`desktop/src/main.jsx` (446 lines) only where a cohesive responsibility exists.

**R4 validation for the three slices above (2026-11-08).**

- `cargo check -p xarchive-desktop --lib` exit `0`, only the pre-existing
  `record_recovery_results` warning.
- `cargo test -p xarchive-desktop --lib -- --test-threads=1` exit `0`, PASS
  **365 / 13 ignored**. The records tests now appear as
  `commands::records::record_import_acceptance_tests::*`.
- Contract audit unchanged: **74** commands, **78** unique handler entries, no
  duplicate, no unregistered command. The four handler entries without a
  `commands*` source (`detect_aria2`, `download_aria2`, `list_aria2_releases`,
  `validate_aria2_path`) remain the pre-existing `aria2.rs` module.
- Equivalence audit PASS with zero failures
  (`.tw2tg-tmp/agent-scratch/build-logs/r4-slice-equivalence.json`): every moved
  item is present in its module, absent from the parent, lexically identical to
  the pre-slice snapshot, and its handler entry names the defining module.
  Lexical equality is supporting evidence, not a semantic proof.




- Split `desktop/src-tauri/src/commands.rs` by jobs, batches, Telegram,
  settings, and record exchange while preserving registered Tauri command names
  and serialized inputs/outputs.
- Split configuration only along cohesive configuration ownership; keep
  environment-specific behavior behind existing platform boundaries.
- Extract frontend feature state only where it has a stable domain responsibility;
  preserve bootstrap and theme initialization order in `desktop/src/main.jsx`.

### Phase R5 — Executor and recovery core (highest-risk refactor)

Status: **IN PROGRESS (2026-11-08)** — 1 of 7 planned responsibility slices landed.

**Responsibility map written before any code moved.** `executor.rs` was 14,785 lines
at the start of R5, of which the `tests` module (5,870–12,465) and
`session_boundary_tests` (12,466–14,785) are 8,915 lines of tests, leaving ~5,868
lines of production code. The production surface separates into:

| Slice | Items | Risk |
|---|---|---|
| **contracts** (landed) | request, snapshot, result, error, config, commit-recovery facts/decisions | low — pure data |
| traits | `JobExecution`, `JobExecutionFactory`, `JobPersistence`, `JobDatabaseFactory` | medium — every impl must follow |
| containment | `LinuxExecutionContainment`, `AttemptWriterDomain`, writer-domain recovery | **high** — writer lifecycle |
| factory | `ProductionExecutionFactory`, `UnconfiguredExecutionFactory`, spec decoding | medium |
| startup recovery | `StartupRecoveryReport`/`Outcome`/`Status`, `recover_internal_v1_on_runner`, `recover_v2_on_runner` | **high** — recovery ordering |
| runtime | `ExecutorRuntime` (951-line impl), `Command`, `RunnerCommand`, `JobExecutorHandle` | **high** — admission/session |
| session | `running_transitions` (467 lines), `handle_command` (240), session boundary types | **high** — the execution state machine |

**Slice 1 — executor contracts: DONE (2026-11-08).**

- `desktop/src-tauri/src/executor/contracts.rs` (new, 311 lines) holds
  `ArchiveJobRequest`, `JobSnapshot`, `SubmitResult`, `JobExecutionResult`,
  `JobExecutionError`, `AttemptCreationError`, `ExecutorConfig`,
  `ExecutorNetworkConfig`, `CommitRecoveryDirectory`, `CommitRecoveryFacts`,
  `CommitRecoveryDecision`, `CommitRecoveryBatchResult` and
  `InMemoryCommitRecoveryFactsProvider`, with their inherent `impl`s.
- `desktop/src-tauri/src/executor.rs` 14,785 → **14,501** lines. The parent declares
  `mod contracts;` and re-exports all 13 items, so every existing
  `crate::executor::<Type>` path is unchanged.
- **This slice cannot reorder ADR-015 by construction:** it contains no `Database`,
  no transaction, no `File`, no writer/permit/phase/resource scope, and none of
  `ExecutorRuntime`, `handle_command`, `transition_job`, `running_transitions`,
  `recover_internal_v1_on_runner` or `recover_v2_on_runner`.
- Validation: `cargo check -p xarchive-desktop --lib` exit `0`, only the
  pre-existing `record_recovery_results` warning; `cargo test -p
  xarchive-desktop --lib -- --test-threads=1` PASS **365 / 13 ignored**;
  `cargo fmt --all` exit `0`; equivalence audit PASS with all 13 items body-identical
  and absent from the parent
  (`.tw2tg-tmp/agent-scratch/build-logs/r5-contracts-equiv.txt`).
- Module description: `docs/architecture/modules/desktop-executor-contracts.md`.
- **One revert was needed.** The first attempt took a contiguous 507-line region and
  swept `ArchiveJobSubmissionAdapter`, `ExecutorEvent`, `CancelArbitration`,
  `JobRecord` and the spec re-export into the slice, which broke the parent. The file
  was restored from
  `.tw2tg-tmp/agent-scratch/source-spans/r5-executor-before-contracts.rs` (14,785
  lines, verified compiling) and the slice was redone with declaration-level spans
  plus an overlap guard. Record this: **a line-range boundary is wrong for this file;
  use declaration-level spans.**

**Slice 2 — executor trait contracts: DONE (2026-11-08).**

- `desktop/src-tauri/src/executor/traits.rs` (new, 419 lines) holds
  `JobExecution`, `JobExecutionFactory`, `JobPersistence` (129 lines of declarations),
  `JobDatabaseFactory`, `InMemoryJobDatabaseFactory` and `InMemoryJobPersistence`.
- `desktop/src-tauri/src/executor.rs` 14,502 → **14,118** lines. The four traits are
  re-exported unconditionally; the two in-memory implementations are re-exported
  under `#[cfg(test)]` because only tests name them.
- `impl JobPersistence for StorageJobPersistence` (291 lines) **stays in the parent**
  beside the runtime that drives it, together with `ProductionExecutionFactory`,
  `ExecutorRuntime`, `handle_command`, `running_transitions` and both recovery
  runners. The slice only names operations, so it cannot reorder ADR-015.
- **One visibility change, recorded:** `InMemoryJobPersistence`'s three fields became
  `pub(crate)`. This fixes the `E0616` error the tests hit after the type moved into
  a child module, but it is **wider than the tests require** — they live in the
  parent, so `pub(super)` is the intended narrower scope. It is not "exactly the
  previous access"; the earlier wording here overstated it.
- **Re-export condition — resolved.** Both in-memory types were `pub struct`s of
  `executor.rs` before the slice, so they belong to the module surface in every
  build. The re-export is unconditional with a local `#[allow(unused_imports)]`;
  only the non-test build calls it unused, which is the accurate signal that no
  production path names them. Hiding them behind `#[cfg(test)]` to silence that
  warning would have narrowed a compatibility path — the library keeps exactly its
  two pre-existing warnings.
- **Field visibility — narrowed to `pub(super)`.** The readers are the parent module
  and its test modules, which `pub(super)` covers without crate-wide exposure.
- **Equivalence — PASS (slice 2a, 2026-11-08).** Audit baseline is the last committed
  pre-R5 source, `370b10eb`, so the check does not rely on a local backup
  (`.tw2tg-tmp/agent-scratch/migration-scripts/r5_traits_audit.py`). Result: 4 traits,
  2 structs and 3 impls match after whitespace/comment normalization; production
  `impl JobPersistence for StorageJobPersistence` confirmed still in the parent; one
  accepted difference — the three `InMemoryJobPersistence` fields private →
  `pub(super)`. Visibility is now compared rather than normalized away, and the
  audit's own exit status reaches the caller (`AUDIT_EXIT=0`), so the earlier
  `result: FAIL` + `EXIT=0` contradiction is gone.
- **Validation:** `cargo fmt --all`; `cargo check -p xarchive-desktop --lib` exit `0`
  with only the two pre-existing warnings; `cargo test -p xarchive-desktop --lib --
  --test-threads=1` exit `0`, PASS **365 / 13 ignored**. Evidence
  `.tw2tg-tmp/agent-scratch/build-logs/r5-slice2a-{check2,test}.log`.
- Module description: `docs/architecture/modules/desktop-executor-traits.md`.

**Slice 3 (next) — containment.** `LinuxExecutionContainment`,
`LinuxLaunchDescriptor`, `AttemptWriterDomain`, `containment_record_root`,
`attempt_writer_base`, `ATTEMPT_WRITER_ROLES`, `recover_writer_domain_set`,
`recover_writer_domain_set_with_evidence`, `recover_predecessor_writer_domains` and
`recover_and_create_writer_group`, with `attempt_writer_domain_tests`. This touches
the Linux writer lifecycle, so it needs the V1 regression matrix re-run, not just the
Desktop lib suite.

### Phase R6 — Sidecar and remaining capability review

Status: **PARTIAL (2026-11-08)** — two slices already committed in `a1d18df7`.

- **Landed:** `gallery_metadata.py` (322 lines: JSONL parsing, metadata
  normalization helpers, media typing, filename sanitization, identity extraction)
  and `gallery_commands.py` (175 lines: `ExtractionConfig`, argv construction, proxy
  environment). `extraction.py` keeps its public names as re-exports, so the
  protocol-v2 worker and the existing tests import unchanged. `extraction.py`
  itself is now **430 lines**, down from the 859-line reviewed baseline.
- **Remaining, and the actual R6 work:** `extraction.py` still holds three runners
  (`ExtractionRunner`, `DiscoveryRunner`, `DownloadRunner`) that share one
  subprocess lifecycle through `ExtractionRunner._run_process` — staging, timeout,
  cancellation, bounded stderr capture, tree termination and cleanup. Judge whether
  that shared lifecycle, candidate scanning or download-result construction is a
  separate cohesive responsibility on dependency evidence. **Default is to keep the
  shared process lifecycle intact**; splitting three runners that all depend on it
  invites an import cycle, an empty compatibility shell, or duplicated
  cancellation/cleanup logic. Recording "reviewed, kept, because X" is a valid R6
  outcome.
- Preserve the protocol-v2 worker entry, cancellation and subprocess cleanup
  behavior; never introduce a generic `helpers` module.
- Review other large files (`config.rs`, `desktop/src/main.jsx`) using actual
  responsibility/dependency evidence. Leave cohesive files intact even if they
  exceed a review threshold, and say so explicitly.


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