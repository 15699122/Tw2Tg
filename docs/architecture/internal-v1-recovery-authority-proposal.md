# InternalV1 PREPARED Recovery Authority Proposal

**Status:** Option B recovery authority and the adapter-minted trusted
drain-evidence boundary were approved by the user on 2026-11-02. Recovery
permit/evidence APIs and Linux startup recovery paths are present in the dirty
source tree. Startup candidate enumeration, single-path InternalV1 dispatch,
and typed reporting have been implemented and compile; targeted replay/idempotence
passed, while candidate-query failure injection, the full startup state matrix,
and real registered-writer production acceptance remain open. Implementation is
`CROSS_PLATFORM_CHANGE_REQUIRED`; Linux adapter implementation is shared
cross-platform work, while the Windows-native adapter remains Windows Owner
work after formal handoff.

The user's 2026-11-02 approval authorizes implementing the adapter-minted
evidence boundary described in the decision record below, within the approved
Option B contract. A bounded user-level disposable delegated-service probe was
subsequently authorized and launched successfully. This does not authorize
changing system delegation/configuration and does not prove the exact
registered-writer fixture's controller or containment requirements; production
acceptance remains open pending that evidence.

**Scope:** Define who may recover an existing InternalV1 archive under the
PREPARED replay and COMMITTED terminal-convergence rules below. This proposal is subordinate to
[ADR-015](decisions.md#adr-015-desktop-归档生产执行的-runner-owned-session-与所有权边界)
and the [InternalV1 completion plan](../development/internal-v1-completion-and-refactor-plan.md).
It does not enable InternalRenameV2, change the schema, or authorize forward
failure injection before recovery is connected.

### Separate bounded successor work — 2026-11-05

The approved Option B contract in this proposal governs journaled PREPARED
replay and COMMITTED convergence; it does not itself grant a journaled recovery
permit to create a successor attempt. Separately, the user approved proceeding
with the bounded no-journal `DOWNLOADING` predecessor-retirement/successor chain
recorded in the active completion plan, within ADR-015's existing ownership
limits. That shared semantic change is `CROSS_PLATFORM_CHANGE_REQUIRED`, but it
does not require a duplicate approval gate. Keep its one-shot retirement
evidence and session takeover separate from publication authority and from the
phase-bound Option B recovery permit. If implementation requires broader
authority, a general Workflow redesign, or changed publication semantics, stop
and seek new approval.

## Context and production call chain

### 2026-11-01 contract clarification

User approval covers Option B, but source review found that ordinary
`ExecutionReservation::claim` cannot authorize recovery: it is intentionally
the only constructor for a *new durable claim*, which would advance the
attempt. The recovery authority must instead bind an immutable existing
attempt after both scopes are acquired, without claiming or incrementing it.
Production Linux admission is represented by the retained
`ProductionCoordinator` workspace guard plus an owned runner operation; Job
and destination scopes must be acquired under that admission and retained
through drain, replay and terminal convergence.

The implementation must model the journal phase explicitly. `PREPARED` replay
may apply the persisted rename plan, move the exact staging tree without
replacement, and recover manifest facts. A `COMMITTED` journal must only
validate the already-final tree and converge a missing terminal state; it must
not republish, reapply a rename plan, or create archive rows/intent again.
The permit must therefore authorize a phase-bound recovery operation, not
unconditionally call the existing mutating `recover_internal_archive` API.
These are clarifications of the approved invariants, not approval to broaden
the contract. If implementation requires a materially different authority or
state transition, stop and ask for renewed approval.

The current shared `ExecutionReservation::acquire` obtains Job and destination
scopes before `claim`; `claim` turns the durable claim result into an
`ExecutionSession` whose attempt is the identity returned by that claim. The
session owns the coordination guard, and `PublicationOwner::for_session` binds
writer registration to that exact session. `PublicationOwner::authorize` only
produces a `PublicationPermit` after admission is closed and every registered
writer role is drained.

Desktop startup recovery is different from a new execution. `recover_startup`
keeps production V2 disabled. The startup loop runs under runner admission;
the InternalV1 branch discovers a recovery candidate, takes the stable
workspace admission and Job/destination scopes, examines the durable journal,
and invokes `ArchiveService::recover_internal_archive`. The Storage replay
re-reads and validates the manifest, requires the current durable attempt to
equal the manifest attempt, checks the journal phase and staging/final tree,
replays only persisted rename facts, validates file facts, and commits recovery
facts. Desktop then persists terminal completion with
`complete_terminal_for_attempt` using the manifest attempt.

The recovery target is the manifest's already-published attempt, not a new
execution. A fresh durable claim is wrong: it would advance attempt/state,
create successor identity, and potentially route the candidate through normal
execution instead of finishing the immutable PREPARED decision. The old attempt
must remain the fence for file replay and terminal persistence. Lock acquisition
alone is insufficient because a crashed parent may leave descendant writers;
the recorded writer domains for the manifest attempt must be drained and that
drain must be established before replay.

An empty `PublicationOwner` is also wrong. It is created from a live execution
session before factory startup, its registry is scoped to that execution, and
an empty role map is not evidence that persistent domains recorded by a prior
process are drained. A fabricated or reused `PublicationPermit` is wrong for
the same reason: permits are constructed only by the exact live owner's
successful authorization and borrow both that owner and session. Recovery must
not bypass that invariant or treat already-published files as a new publication.

## Options reviewed before approval

### A. Controlled recovery session is the Storage authority

Introduce an unforgeable runner-owned `RecoverySession<'a, C>` carrying the
original `AttemptIdentity`, validated destination, Job/destination guard and
successful drain proof. Storage accepts `&RecoverySession` and checks that its
job, attempt and destination match the locked journal.

Advantages: fewer authority types; the control object directly represents the
full serialized operation. Risks: Storage must depend on a broad workflow
session type and its coordination/lifetime structure; it may gain methods that
let it inspect or retain coordination authority beyond the narrow replay need.
It also makes API evolution harder if Desktop and Storage ownership boundaries
change independently.

### B. Derived, narrow Storage recovery capability — approved

The controlled runner session remains the sole authority to establish
admission, scopes, original attempt, destination and writer drain. While that
session is live, it derives one non-Clone, non-Serializable
`PreparedRecoveryPermit<'session>` borrowing the session and a private,
validated recovery-drain witness. Only this capability crosses into Storage.
It is not a publication permit and cannot be constructed from a job id, a
counter, a bool or a caller-supplied `AttemptIdentity`.

This keeps lock/admission ownership in Desktop/workflow and gives Storage the
smallest explicit proof it needs. It makes the authority crossing visible in
the type signature without handing Storage the coordinator or lock guard.
The borrow prevents it from outliving the session; single-use consumption
prevents replay authority from being retained or cloned.

## Option B contract sketch (approved boundary; illustrative API only)

The following signature sketch describes the intended narrow capability under
the approved evidence-minting boundary. It is illustrative, not a frozen
production API; implementation must preserve the approved invariants while
checking actual crate dependencies, lifetimes and visibility.

```rust
// xarchive-workflow: shared contract consumed by Desktop and Storage.
// This capability does NOT depend on ExecutionReservation::claim: recovery
// must never issue a new durable claim for the immutable journal attempt.
pub struct PreparedRecoveryPermit<'session, C: Coordinator> {
    // private, non-Clone/non-Serialize; carries validated job, attempt,
    // canonical destination and a borrow of the recovery session.
}

impl<C: Coordinator> PreparedRecoverySession<'_, C> {
    pub fn job_id(&self) -> &str;
    pub fn attempt(&self) -> &AttemptIdentity;
    pub fn destination(&self) -> &Path;
    pub fn authorize_prepared_replay(
        &self,
        validated_journal: &ValidatedPreparedJournal,
        drain: &RecoveryDrainProof<'_>,
    ) -> Result<PreparedRecoveryPermit<'_, C>, RecoveryError>;
}

// xarchive-storage: phase-discriminated so committed convergence cannot replay.
pub enum InternalV1RecoveryOutcome {
    PreparedReplayed { facts: TweetArchiveFacts },
    CommittedVerified { facts: TweetArchiveFacts },
}

impl ArchiveService {
    pub fn recover_internal_v1(
        &mut self,
        request: InternalV1RecoveryRequest<'_>,
        permit: PreparedRecoveryPermit<'_, impl Coordinator>,
    ) -> Result<InternalV1RecoveryOutcome, StorageError>;
}
```

`PreparedRecoverySession` is a distinct, non-claiming session constructed only
inside the runner after the live Job/destination reservation and validation of
the immutable manifest identity, without performing a claim. `TweetArchiveFacts`
is the existing concrete Storage type needed for terminal persistence. The
sketch is illustrative only and is not authority to create production Rust
signatures before the evidence-boundary decision. This decision was resolved
by the user's explicit 2026-11-02 approval recorded below; this paragraph is a
historical source-review observation, not the current implementation status.

### Approved evidence boundary — 2026-11-02

The user approved a platform-neutral, move-only `VerifiedAttemptDrain` returned
through a private construction path by the trusted platform adapter only after
the complete fixed writer-role set has been successfully drained. The evidence
must bind workspace/job/attempt and the controlled recovery operation; Workflow
alone consumes and validates it to derive the one-shot `PreparedRecoveryPermit`.
Desktop must not mint evidence from a successful `Result<()>` or caller
assertion. Shared Workflow must not inspect Linux cgroups. This approval defines
the trust boundary but does not claim implementation or validation, grant
delegated-cgroup permissions, or transfer Windows adapter ownership.

## Approved construction and required validation

1. Runner first obtains lifecycle admission. A refused/replaced runner does no
   candidate mutation.
2. Under that admission, acquire the canonical Job and destination scopes in the
   same global ordering used for execution. Contention returns a reschedulable
   outcome and does not persist `FAILED`.
3. While holding both scopes, read and validate the journal and manifest. Require
   `phase == PREPARED`, contract `InternalV1`, valid manifest/hash/path facts,
   and equality of candidate Job, manifest Job, Tweet row/id, canonical final
   destination and current durable attempt. The attempt comes from the existing
   journal/durable state; no claim/increment is performed.
4. Derive the recovery session only from this validated locked snapshot. It
   records the existing `AttemptIdentity(job_id, manifest.attempt_count)` and
   exact destination. Re-read/fence these durable facts immediately before
   granting the capability; any changed attempt/journal/target is stale and
   fails closed.
5. Close admission to new starts for the affected writer domains. Recover each
   persisted writer-domain record for that exact job/attempt, stop remaining
   processes/descendants and verify the domain is empty. `NoRecord`,
   `AlreadyAbsent` or confirmed `Drained` may count only as specified by the
   existing adapter contract; unknown, corrupt, busy or failed drain yields no
   proof. The witness records the exact attempt and domain-set identity, not a
   boolean supplied by Desktop.
6. Re-read the journal, durable attempt and destination while still holding both
   scopes. Require the same PREPARED manifest identity/hash and destination.
   Only then derive the one-shot `PreparedRecoveryPermit`.

### Required Storage and durable convergence

Storage consumes the permit by value and independently validates that its
private identity equals the request and the locked PREPARED journal. It must
not expose a public test constructor or a permit-minting hook. It performs the
existing no-follow/path/hash checks and replays only the immutable manifest and
persisted rename plan. It may be idempotent when the same PREPARED attempt has
already reached the final directory, but must not re-plan names or create a new
attempt. It commits the Storage journal/archive facts transactionally and
returns the exact validated attempt, path and file count.

Desktop persists terminal events/state using
`complete_terminal_for_attempt(job_id, recovered.attempt, ...)` while runner
admission and both scopes remain held. A stale attempt fence is an error and
must never fall back to an unfenced `complete`/`fail`. A durable archive with a
terminal-persistence gap remains recoverable from its committed manifest; it is
not converted into a successor attempt.

### Ownership, cancellation, errors and release order

The permit's borrow is tied to the live recovery session. Neither Storage nor a
worker thread may retain it after the session ends. The proposed session owns
the guard and runner lease; the caller owns the returned recovery facts. Keep
the session alive through Storage replay and terminal transaction. Release in
this order: finish Storage transaction; persist/fence terminal Job and events;
drop permit (consumed by Storage); drop writer-drain witness/adapter handles;
drop recovery session (releasing destination then Job scopes through the
adapter's established reverse/global order); release runner admission last.
Exact guard-release mechanics must follow the coordinator's existing contract.

- Cancellation before journal validation/acquisition can cancel startup work.
  Once `PREPARED` is validated and replay authorization granted, PREPARED wins:
  cancellation cannot abandon the durable transaction halfway or create a new
  attempt. It may be observed after convergence.
- Scope contention is reschedulable, with no state/error mutation. A competing
  recovery can win once; the loser re-reads after acquiring scopes and observes
  COMMITTED/terminal state or a changed journal, then returns idempotent/no-op or
  stale without duplicate events.
- A changed attempt, journal hash/phase, Job/Tweet mapping or destination is
  `StaleRecovery`/integrity failure, no permit, no file mutation, no terminal
  write. An unknown/corrupt journal is fail-closed.
- Drain failure or timeout grants no capability and performs no replay. Preserve
  enough durable state for retry; do not report a successful drain based on
  process-parent exit, Drop, or scope ownership.
- Panic unwinding drops the permit/session and releases process-local guards,
  but Drop is not proof that writers drained or replay committed. The PREPARED
  journal remains the recovery source for a later admitted runner. Process crash
  behavior remains covered by the external writer-domain drain contract.
- Repeated recovery verifies/finalizes the already committed exact attempt
  without duplicate terminal events. Missing or contradictory durable facts
  remain errors; no best-effort success.

Proposed error families: `Contended` (reschedulable), `RunnerStopping`,
`JournalMissing`, `UnsupportedContract`, `InvalidJournal`, `StaleRecovery`,
`DestinationMismatch`, `DrainIncomplete`, `StorageReplay`, and
`TerminalFence`. Public error mapping must preserve machine-readable distinctions
where the current API already exposes them; never conflate a Storage replay
failure with a successful terminal update.

## Routing, compatibility and platform impact

- Legacy/no-manifest routing remains unchanged. Recovery authority is only used
  for a validated InternalV1 PREPARED journal. Do not route legacy candidates
  through an invented manifest or this permit.
- Production V2 stays fail-closed. The fixture-only
  `recover_startup_with_v2(true)` must not become a production switch through
  this work. V2 needs its own accepted completion gate.
- The recovery permit is distinct from `PublicationPermit`; no weakening of
  normal publication authorization, no test-only permit minting, and no bypass
  of registered-writer drain.
- This is a shared Workflow/Storage contract change and requires
  `CROSS_PLATFORM_CHANGE_REQUIRED`. Linux implementation must keep the shared
  API platform-neutral. Windows Owner must implement/validate equivalent runner
  admission, Job/destination locking and writer-domain drain semantics in its
  adapter before claiming Windows acceptance; Linux evidence does not transfer
  ownership or establish Windows PASS.

## Required validation after approval

1. Contract-level tests: private construction/non-Clone semantics, identity and
   destination mismatch, scope contention, stale attempt/journal, no new claim,
   incomplete/corrupt/unknown writer-domain drain, repeated recovery, duplicate
   terminal-event prevention, cancellation boundary, panic/Drop/restart behavior.
2. Production runner PREPARED replay using the exact Storage DB and registered
   writer-domain records. Keep the journal attempt unchanged and prove that
   runner admission, both scopes, drain, Storage replay and attempt-fenced
   terminal persistence occur in order.
3. Only after that passes, add the narrow post-file-commit DB failure seam,
   exercise recovery/retry, then run the V1-02b/c/d and V1-03/V1-04 acceptance
   subtargets. Do not enable production V2.
4. Risk-based Targeted → Module → Subsystem validation, final diff review,
   `git diff --check` and docs audit. Run Desktop formatting only if the exact
   check passes; do not format unrelated dirty `executor.rs` content.
5. Windows-native adapter and behavior remain Windows Owner validation; queue
   them for formal Git handoff.

## Environment prerequisite snapshot (2026-11-01)

This is capability discovery only, not product validation or a delegation
change. On Linux x86_64 WSL2, the root cgroup subtree control is not writable,
but the user systemd manager is running and an authorized disposable scope
successfully launched with `Delegate=yes`:

```text
systemd-run --user --wait --pipe --collect --property=Delegate=yes \
  --property=Description=tw2tg-delegation-probe-20261101 /bin/sh -c ...
unit: run-p189702-i190683.service
invocation: 31cfee92bee949fdbcf65556eb6472e3
scope: /user.slice/user-1000.slice/user@1000.service/app.slice/run-p189702-i190683.service
controllers visible to parent: cpu memory pids
exit: 0
```

This proves only that a disposable delegated scope can launch. A later
re-probe also launched `Delegate=yes` scope `run-p4587-i4927.service`
(invocation `06ca0ae0428f4665ab5a78bf9f9e81e0`, exit 0), with `cpu` and
`memory` visible in the scope controller list; the cgroup root's `cgroup.procs`
was not writable. These probes do not prove the registered-writer fixture can
perform its required containment operations. Do not call the production
delegated-cgroup prerequisite available until the exact fixture verifies its
controller setup and containment operations. They also do not prove source
identity, writer registration, drain barrier or publication behavior; verify
those at execution time. `aria2c` was not found on Linux
PATH or in `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, `~/.cargo/bin`, or the
checked Windows System32 path. The repository records official aria2 1.37.0 as
the expected supported Windows artifact, but no executable/source artifact
identity is available in this Linux environment. Do not install, provision or
copy one without authorization. Real aria2 acceptance remains NOT_RUN.

## Decision record

**Decision recorded — 2026-11-01 and clarified 2026-11-02:** The user approved
Option B on 2026-11-01 and explicitly approved its trusted evidence-minting
boundary on 2026-11-02. The approval covers:

1. A controlled runner-owned recovery session for the
   immutable existing attempt derives a one-shot, non-forgeable,
   non-Clone `PreparedRecoveryPermit` borrowing that session and verified drain
   proof; Storage consumes it for InternalV1 PREPARED replay only.
2. The required production ordering: runner admission → Job and
   destination scopes → lock-protected journal/original-attempt/destination
   validation → exact recorded writer-domain drain → revalidation → permit →
   Storage replay → attempt-fenced terminal persistence → release scopes and
   admission.
3. A private, move-only `VerifiedAttemptDrain` minted only by the trusted
   platform adapter after complete fixed-role drain; Workflow validates and
   consumes it to derive the permit. No Desktop assertion or public constructor
   may mint the evidence.
4. The stated fail-closed semantics for contention, stale identity,
   missing/corrupt journal, incomplete drain, replay failure, cancellation
   after PREPARED wins, repeated recovery, panic and Drop; no new claim, forged
   publication permit, empty owner, weakened drain or unfenced terminal write.
5. Authorization limited to InternalV1 PREPARED replay and COMMITTED
   terminal convergence, does not enable production V2, and leaves Windows
   adapter/native acceptance to the Windows Owner.

The user separately directed that aria2-specific tasks be skipped for this
phase and that Sidecar-scoped V1 completion be prioritized before modular
refactoring. This does not mark aria2 acceptance PASS or NOT_APPLICABLE. The
2026-11-02 approval resolves the evidence-boundary decision and authorizes its
implementation within the stated limits. Production recovery and its dependent
crash-replay acceptance remain incomplete until implemented and validated;
delegated-cgroup acceptance remains NOT_RUN until an authorized disposable
environment is available. Windows adapter implementation and validation remain
with the Windows Owner after formal Git handoff.
