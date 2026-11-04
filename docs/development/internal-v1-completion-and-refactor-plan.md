V1 Completion and Modular Refactoring Plan

Owner: Cross-platform Owner for shared implementation and Linux validation.
Status: ACTIVE — complete the approved Sidecar-focused InternalV1 scope first;
aria2-specific work is deferred by user direction. Modular refactoring follows
only after the approved phase gates are met.

## Current execution state — 2026-11-04

The approved Option B recovery authority and adapter-minted evidence boundary
remain in force. Startup candidate enumeration now returns a fallible result;
   startup builds one deterministically ordered union of generic active candidates
   and declared recovery-contract jobs; journaled InternalV1 jobs route through
   runner-owned recovery after exact writer-domain drain, while declared
   no-journal jobs follow state-checked fail-closed or normal session execution.
   A typed internal report separates fatal enumeration/setup
errors from per-job `Recovered`, `DeferredByContention`, and `Refused` outcomes.
Targeted Linux validation and two bounded delegated registered-writer production
passes are recorded in the active handoff and validation ledger. Descendant-SIGKILL
recovery is verified for the exact ignored fixture only in the approved
delegated scope, active-writer replacement remains unverified, and full phase
acceptance remains open. The Storage recovery module has 27/27 passing tests in
the latest final run; earlier runs had three stale-fixture failures, which were
corrected before the recorded final PASS. Do not treat those superseded failures
as current failures.

InternalV1 candidate over-inclusion and startup double enumeration have been
removed in the current dirty implementation; terminal journal rows remain eligible
for COMMITTED convergence. Recovery refusals are reported
without an unfenced failure write; raw underlying errors remain excluded from
user logs. The current InternalV1 no-journal dispatch accepts QUEUED,
VALIDATING, METADATA_READY, DOWNLOADING, and INTERRUPTED; DOWNLOADED without a
journal fails closed, while any journaled state proceeds to lock-protected
phase/identity validation. Startup tests cover first-attempt QUEUED, missing
manifest after DOWNLOADED, unknown contract, valid COMMITTED convergence,
malformed writer record, and enumeration failure. Full per-state candidate
enumeration coverage remains incomplete; add it without bypassing authorization.

Current Git snapshot (inspected 2026-11-04): branch
`cross-platform/automatic-pairing-reconcile-20261002`, HEAD
`1baefa345053204e0eb252213fdcdf4544494277`; 18 modified tracked paths and 6
untracked paths, including this plan, recovery proposal, fixture and retained lock
artifacts. Preserve all existing edits. No commit or formal handoff exists. The
corrected registered-writer production fixture passed twice in bounded disposable
`Delegate=yes` scopes (latest recorded log has `EXIT_CODE=0`). The separate
descendant-SIGKILL exact test passed only in the bounded delegated service noted
below; direct non-delegated execution is unavailable due to cgroup PermissionDenied.
aria2 remains user-deferred/NOT_RUN. Windows-native acceptance remains Windows
Owner work after formal Git handoff.


### Immediate execution cursor

- **2026-11-04 continuation:** The startup PREPARED/COMMITTED report regression now asserts exactly one `Recovered` outcome for each phase; targeted Desktop test PASS 1/1, candidate-query state matrix PASS 1/1, Storage recovery filter PASS 27/27, Desktop startup filter PASS 9 applicable / 1 ignored. `git diff --check` and docs audit PASS. The recovery authority review remains an active `CROSS_PLATFORM_CHANGE_REQUIRED` gate: `PublicationOwner::authorize` does not consume adapter-minted `VerifiedAttemptDrain`. Continue that approved shared-boundary integration only; do not replace the authority with lifecycle registry assertions. Live active-writer replacement and Windows-native validation retain their current blockers/ownership.

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

### Act continuation — evidence-boundary contract review — 2026-11-04

- Reconciled current dirty tree at HEAD `1baefa345053204e0eb252213fdcdf4544494277`;
  preserved the pre-existing dirty paths and untracked artifacts. No product
  changes were retained in this continuation.
- Confirmed the Option B approval authorizes a narrow, move-only adapter proof
  bound to workspace/job/attempt and operation, consumed by Workflow, with
  Storage's independent durable journal checks unchanged. Publication permit
  extension is separately recorded as `CROSS_PLATFORM_CHANGE_REQUIRED`.
- Contract review found the current Linux recovery proof is only bound to
  workspace/job/attempt; it does not carry a controlled-operation/owner token,
  and its construction lives under a Linux-only module. `PublicationOwner`'s
  registration state is per live session and cannot be populated by a recovery
  proof without an explicit reviewed bridge. Recovery evidence is not evidence
  that the current live writer set has drained.
- A trial API edit was reverted after review showed it would either accept
  mismatched evidence or require an unapproved/public mint seam. A Workflow-only
  test run during the trial passed 49/49 but does **not** validate the final
  source state. No Desktop/product tests were run for this continuation.
- Next: design a crate-private adapter-to-Workflow bridge binding the fixed
  writer-role set and controlled operation; test missing, wrong-attempt,
  wrong-owner, incomplete-role, timeout and partial-failure rejection. Then
  thread the proof through finalizer into Workflow permit authorization while
  leaving Storage's journal fence independent. Do not add a public mint hook.
- Separately build a bounded parent-controlled barrier around a production
  `RuntimeState::replace_executor` operation with a real active writer, and
  verify admission close, drain, finalize, join and generation ordering. The
  existing transport/coordinator tests do not establish that acceptance.
- Before Linux fixture execution, verify the actual fixture's database lock
  directory and current delegated-cgroup capability. Windows-native adapter and
  validation remain Windows Owner responsibilities after formal Git handoff.

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
Recovery targets are applicable under the approved Option B
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