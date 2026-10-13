# Desktop Download Output and Settings Plan

## Linux bootstrap progress (2026-10-13)

The sidecar now has a trusted Linux cgroup launcher which joins and verifies an
existing delegated group before exec, without Rust unsafe or post-spawn migration.
Eleven launcher tests pass, and the real delegated diagnostic uses this launcher
for positive late-write and kill/drain controls. This does not prevent same-user
adversarial migration. Production supervisor ownership, packaging and lifecycle
wiring remain pending; this evidence does not complete B4 or enable v2.


Owner: Cross-platform Owner for shared contracts, Rust/Storage/Sidecar integration,
and shared UI; Windows Platform Owner for Windows-specific integration, native GUI,
runtime behavior, packaging, and Windows validation.
Status: `IN_PROGRESS` — current owner is the Cross-platform Owner on branch
`cross-platform/automatic-pairing-reconcile-20261002`, starting HEAD
`6567691d26f1f6b33f8153edcc95013bb79012d0`, equal to upstream. Three pre-existing
uncommitted documentation edits were preserved; this continuation updates the
Plan and Windows queue. No production code changed, no commit or formal Windows
handoff is claimed, and Windows acceptance is not implied.

**Enablement boundary.** The absent Windows coordination adapter does not block
shared cross-platform implementation or Linux verification. It blocks exactly
two things: enabling the new coordinated lifecycle on Windows, and any
Windows/NTFS platform acceptance. Accordingly the shared batches below are
allowed to proceed, and they must not (a) silently omit the session behind a
platform `cfg`, (b) substitute a no-op coordinator for real cross-process
advisory locking, or (c) make the existing supported download path fail on
Windows. The new path stays gated on the platform acceptance gate until the
Windows Owner implements and validates the same `Coordinator` port; the legacy
path keeps its current behavior and is never presented as having C1
lifecycle guarantees. No `WINDOWS_BLOCKING` is declared for B3–B7 as a whole.
This is shared WIP,
not a Windows handoff. E3 settings, v3 task/batch snapshots, selected-tool policy
and compatibility checks are implemented. Linux actual-spawn acceptance is
verified (aria2 supervisor and gallery-dl DownloadRunner argv paths); gallery-dl
user arguments remain disabled by the empty allowlist. F1 shared preference,
pre-render application, and token implementation are present; Windows visual and
native acceptance remains pending. The overall Plan is not complete: C1.4
production recovery, D1, F2/F3 shared implementation/design gates and integrated
G remain open. Production C1 v2 remains fail-closed. Windows acceptance requires
formal Git handoff and a fresh artifact. Historical commit references below
describe prior checkpoints, not the current tree.

Supervisor shutdown follow-up (2026-10-13): `Aria2Supervisor::shutdown` now
returns an error when child waiting or short-lived secret-file removal fails;
`transfer_once` propagates shutdown and cancellation-monitor failures rather
than treating transfer success alone as sufficient. `SidecarSupervisor::shutdown`
reports child-wait, output-reader timeout, and reader-panic failures, and the
explicit stop command returns those failures. These local lifecycle checks do
not prove escaped descendants exited, authorize publication, or replace Linux
cgroup B4 evidence. B4/B5 remain incomplete and v2 remains fail-closed.

`/goal` capability correction (2026-10-13, source `ec11434`): a disposable user
systemd service with `Delegate=yes` provides writable cgroup v2 and cgroup.kill,
despite the non-writable root hierarchy. `scripts/validate-cgroup-containment.py`
passes a real setsid descendant/parent-exit kill/drain/late-write diagnostic.
Repeat using `systemd-run --user --wait --pipe -p Delegate=yes python3` followed
by the absolute script path. This is kernel/environment capability evidence,
not production containment integration or B4 completion. Continue B3–B7 then
D1/F2/F3 and integrated G; Windows-exclusive checks remain separately queued.

The diagnostic additionally verifies a positive delayed-write control and uses
only its exclusively created cgroup for cleanup, avoiding cached-PID signaling.
Both controls PASS in the delegated service. Production containment is still
`IN_PROGRESS`: existing supervisors call `Command::spawn`; moving a running child
after spawn is racy, while `pre_exec` conflicts with workspace `unsafe_code=forbid`.
Before integration, define a trusted bootstrap/spawn boundary that joins before
any tool code executes, propagates launch/readiness failures, kills and drains
before publication, and states its migration-out threat model. The diagnostic
bootstrap is test code, not an approved production launcher or sandbox.

This plan formalizes the user's exploratory outline only where the explicit
scope and constraints below are more precise. Current implementation facts remain
in [`status.md`](status.md), current batch ownership/revision in
[`../status/platform-handoff.md`](../status/platform-handoff.md), and platform
validation evidence in [`../validation/windows-queue.md`](../validation/windows-queue.md).

## 1. Goal and scope

Add task-stable output settings and safe customization of archived media names,
user-facing metadata, supported downloader options, application theme, and local
configuration exchange. Preserve the existing `Tweets/<tweet_id>` archive
identity and current Telegram completion-time configuration sampling.

Screenshots are interaction examples only. They do not define layout, colors,
control styling, or product fields. Reuse the project's existing visual system
and expose only fields supported by verified data paths.

Out of scope: changing archive-root or Tweet-directory identity; rewriting old
archives; persisting signed URLs, request headers, cookies, or browser secrets in
execution specs/recovery records; shell evaluation; silently merging/overwriting
an existing archive; and treating Linux checks or older artifacts as Windows GUI
acceptance.

## 2. Confirmed baseline and constraints

- `archive_job_requests` is the existing durable execution-spec table. It stores a
  positive `schema_version`, request ID, and JSON. The current E3 implementation
  writes v3 snapshots and dispatches historical versions; v1/v2 decode with empty
  custom-argument arrays, and unknown versions must fail closed. Reconfirm this
  behavior against the current diff before changing execution-spec code; the former
  v1-only statement is a historical baseline, not the current implementation.
- The runner loads the persisted request by Job ID. Reused active jobs do not
  create a new spec. Missing specs already have explicit recovery handling.
- Archive output currently uses `Tweets/<tweet_id>`. Rust validates staging files,
  computes size/hash, writes `tweet.json` and `tweet.txt`, then commits the staging
  directory; an existing destination is rejected.
- Production currently captures Telegram intent before archive completion. A
  naming feature must first finalize the media-relative-path mapping and update
  the unified result; only then may intent be captured. Telegram settings continue
  to be sampled at archive completion, not pinned to task submission.
- `sidecar_args` launch the worker; Sidecar `executable_args` are part of the
  trusted gallery-dl process configuration. Neither is a user custom-arguments
  interface; task arguments travel only on the v2 download command's `user_args`.
- Existing config persistence and selected settings are implemented. This does
  not establish theme support or safe versioned import/export.

## 3. Batches and dependencies

| Batch | Scope | Depends on | Main verification |
|---|---|---|---|
| A | Existing page split and project-style UI | Current handoff | Frontend wiring; Windows fresh-artifact GUI remains pending |
| B1 | Explicit execution-spec version decoding, historical v1 compatibility, unknown-version rejection | None | Executor tests: v1 valid/invalid, unknown version, identity mismatch, reject before process startup |
| B2 | v2 envelope and immutable output-settings snapshot, wired to one-off and batch submission | B1 | Executor/Storage/batch tests: defaults, snapshot persistence, reuse, changed settings, retry and restart |
| C0/D0 | Internal archive recovery journal/manifest independent of optional user exports | B2 | Storage/archive integration and injected crash boundaries; old/new/corrupt recovery records |
| C1.2 | `xarchive-workflow` crate and same-host multi-process coordination | C0/D0 | Shared contracts and Unix `flock` adapter implemented; production lifecycle wiring remains open; Windows adapter is Windows-owned |
| C1.3 | Filename renderer, attempt-isolated staging, replayable file operations and attempt fencing | C0/D0, C1.2 | Staging/replay helpers implemented and targeted-tested; production attempt ownership/writer integration open; non-Unix no-replace adapter is Windows-owned |
| C1.4 | Attempt-fenced archive finalization and production recovery wiring | C1.2/C1.3 and D0 | Cancellation/PREPARED and attempt-fenced state-write primitives tested; transactional production finalization and service restart matrix remain open |
| D1 | Optional user JSON/YAML outputs, separate from recovery facts | C0/D0, C1 | JSON/YAML contract approved; four combinations remain blocked until the C1 shared gate passes; historical TXT snapshots and archives retain their meaning and are not rewritten |
| E | v3 task snapshots and user downloader arguments through actual tool argv | B2 | E0–E3 shared implementation and actual spawn acceptance are present; Windows Named Pipe and packaged-artifact checks remain queued |
| F | Theme, configuration exchange and download/archive record exchange | Settings schema decisions; three scoped streams after contract review | F1 shared preference/pre-render/theme-token implementation is present; Windows appearance checks queued. F2 implementation/design freeze open; F3 JSON/YAML format decision confirmed; shared schema/dependency/transaction implementation gates remain open |
| G | Integrated Linux verification and Git handoff | Applicable batches | Risk-based module/subsystem checks, docs audit, diff review; exact source revision recorded |
| H | Windows implementation/validation batch | Git handoff from G | Fresh artifact and worker provenance; native no-replace/coordination, NTFS path, process, recovery, GUI, DPI and configuration checks |

### Current execution order and status vocabulary

Implementation and verification are separate dimensions. For work items below,
`PLANNED` means implementation has not started, `IN_PROGRESS` means implementation
is being changed, `COMPLETED` means implementation scope is complete, and
`VERIFIED` is reserved for a completed scope with the required evidence. Do not
use `COMPLETED` or `VERIFIED` as synonyms for code merely present in the worktree;
validation results additionally use the states defined in
[`../validation/validation-policy.md`](../validation/validation-policy.md).

The user goal for the 2026-10-12 continuation is to complete every Plan item
that does not require Windows execution, then consolidate Windows-only manual
validation. This is a target, not a claim of completion: the current source still
has shared implementation gates listed below. Execute in dependency order and
keep production v2 fail-closed until the full C1 gate passes. F2/F3 remain subject
to their explicit shared design gates in §11; do not invent unresolved decisions
to make the checklist appear complete. The 2026-10-12 continuation hardened
attempt-staging directory creation and verified its Storage/Workflow/Desktop
modules, but did not close the production lifecycle, F2/F3, or integrated G. The
2026-10-12 resumed review resolved F3's activity policy: apply uses the shared
maintenance barrier and cancellable drain, not immediate refusal while tasks run.

The goal does not authorize treating Windows ownership as a reason to stop shared
work, nor does it authorize choosing unspecified data contracts on behalf of the
product owner. If a shared design gate is still genuinely undecided, record the
exact decision needed and keep its implementation `PLANNED`; proceed with
independent shared work and prepare reproducible Windows manual steps for the
implemented revision. Do not mark an unimplemented Windows feature `NOT_RUN` as
if it were runnable.

The current non-Windows order is:

1. **P0 — baseline and documentation synchronization (`VERIFIED` for this
   resumption inventory only):** branch
   `cross-platform/automatic-pairing-reconcile-20261002`, HEAD
   `6567691d26f1f6b33f8153edcc95013bb79012d0`, equal to upstream before this
   uncommitted documentation batch. Three pre-existing modified documentation
   files were retained. Environment is Ubuntu x86_64 under WSL2; Cargo/Rust,
   Node, `/mnt/c`, `powershell.exe`, and `DISPLAY=:0` are available. This
   establishes neither a native Windows Desktop runtime nor artifact-bound
   Windows evidence. P0 does not claim C1 or other implementation/validation
   gates complete; historical test counts bind only to their recorded revisions.
2. **C1.1 — durable recovery contract (`COMPLETED`; Linux module verification PASS):**
   the v2 manifest is an immutable plan (`rename_phase` remains `PLANNED`); mutable
   progress is stored in `rename_progress`/`phase`. Creation relies on migration
   0017's `rename_progress` default and does not name that column in the INSERT.
3. **C1.2 — `xarchive-workflow` foundation and multi-process coordination (`IN_PROGRESS`; `CROSS_PLATFORM_CHANGE_REQUIRED`):** the crate defines bounded identity/phase/path contracts and Linux now has a Unix `flock` adapter (replacing the retired per-process `fcntl` record locks) with a process-shared local lock registry, `O_NOFOLLOW` symlink rejection and group-lock rollback. `cargo test -p xarchive-workflow --test coordination` passes 11/11, covering cross-process and same-process contention, crash release, guard drop/release, partial-group rollback, fork/exec descriptor residue, parallel jobs and destination scope. Remaining shared work is wiring Job/destination ownership into production lifecycle; no production lifecycle holds its guards yet. Windows adapter remains Windows-owned.
4. **C1.3 — replayable file operations and attempt fencing (`IN_PROGRESS`; `CROSS_PLATFORM_CHANGE_REQUIRED`):** Storage replay accepts only immutable `RenameEdge` plus `ArchiveRenamePlan` identities, validates lexical containment and link/reparse points, distinguishes missing from I/O errors, and checks size/hash for both pending and already-completed edges. Linux moves use atomic `renameat2(RENAME_NOREPLACE)`; non-Unix remains fail-closed pending the Windows Owner adapter. `create_attempt_file` performs NOFOLLOW create-new creation, and the attempt staging/replay helpers are implemented and targeted-tested, but they are not wired into Sidecar/Executor. A non-behavioral guard now deprecates permissive `FileStore::attempt_staging_dir` so future fresh-attempt creation cannot silently reuse an existing payload; only tests used that helper, and production still uses Job-level staging. Persistent fencing at all file writes remains unimplemented.
5. **C1.4 — C1 finalization and production wiring (`IN_PROGRESS`; production remains fail-closed):** `recover_internal_rename_v2` still returns a fail-closed error, the service failpoint parameter is present but not wired to operation boundaries, and Desktop startup still returns `ARCHIVE_RECOVERY_V2_NOT_READY`. Service-level process restart coverage and read-only COMMITTED filesystem+DB verification are absent. The cancellation/PREPARED linearization boundary is implemented and targeted-tested (`archive_commit_started` advisory probe plus `decide_archive_cancellation` re-checking both journal generations inside one transaction, exposed to Desktop as `persist_cancellation`/`CancelArbitration`); `begin_archive_attempt` also rejects terminal Jobs transactionally. These primitives are not yet bound to a production lifecycle and do not provide atomic Job claim or full attempt ownership: attempt-isolated writes for both download paths, replay and no-replace directory commit, complete finalize/replay wiring, and the service crash/restart matrix remain shared gates. Index handling also needs an end-to-end contract audit: inspected Sidecar extraction/model code enumerates from 1, core rejects index 0, and SQLite requires positive indexes. The existing storage finalizer also persists `tweets.archive_path` from the caller-provided directory without a dedicated COMMITTED read-only verifier; both require attempt/destination-bound integration coverage before v2 enablement.
6. **D1 (`PLANNED`, gated on C1):** activate the four JSON/YAML output combinations
   without making internal recovery depend on user exports or rewriting legacy
   archives.
7. **E (`IN_PROGRESS`; `CROSS_PLATFORM_CHANGE_REQUIRED`):** preserve v1/v2 task-spec behavior; add v3 for typed argument snapshots and policy/tool compatibility identity; make Rust the sole authority for user-option policy checks at task creation and immediately before execution; and connect gallery-dl and aria2 argv end-to-end. Python retains protocol shape and argv-boundary handling but does not duplicate the user-option allowlist/value policy. Share the parsed option contract with the pinned tool versions; unknown user options fail in Rust before process startup. **E0 (Rust argument-policy authority) is now implemented and corrected:** `crates/xarchive-download/src/downloader_args.rs` defines a versioned `ARGUMENT_POLICY_VERSION`, a narrow reviewed `DOWNLOADER_ARG_ALLOWLIST` (retry/timeout/concurrency knobs only), an explicit `PROTECTED_ARIA2_OPTIONS` set (output/proxy/credential/config/RPC/hook/reporting) that classifies rejections without ever permitting an option, JSON-string-array parsing that preserves argv boundaries and rejects shell metacharacters/`@file` indirection/`--`/attached/short forms, and `build_aria2_argv` with per-option value ranges. The pre-integration prototype's contract gap (it accepted `--out`/`--all-proxy` overrides) is fixed; the former "gap-recorded" tests are replaced by protected-option acceptance assertions, and policy-data tests lock the allowlist/protected sets against duplicates and overlap. The module remains a self-contained policy authority with **no production caller yet**. **E1 (v3 execution-spec snapshot) is now implemented:** `desktop/src-tauri/src/executor.rs` defines `ToolCompatibility` (per-engine pinned version, `None` when an engine carries no arguments), `DownloaderArgumentSnapshot` (validated argv arrays plus `policy_version` and `tool_compatibility`, `deny_unknown_fields`), the `ExecutionSpecV3` envelope, a `DecodedExecutionSpec.downloader_arguments` field, and version-3 dispatch in `decode_execution_spec`. v1/v2 continue to decode to an empty `DownloaderArgumentSnapshot::historical()` (never resampled from current settings); the argument-policy version is validated independently of the execution-spec version via `validate_downloader_argument_snapshot`, and a foreign policy version is refused rather than reinterpreted. Linux unit tests (5 new, then 277/277 in the desktop lib) cover v3 request/output/snapshot decode, v1/v2 empty-historical decode, policy-version mismatch refusal, invalid JSON, unknown snapshot field, and byte-for-byte argv round-trip. The argument-policy version is dispatched separately from the schema version (the DB `schema_version` column stays authoritative and no duplicate version is added to JSON). **E2 (both engines wired) is now implemented:** the decoded `downloader_arguments` snapshot flows from `ProductionExecutionFactory::create` into `ArchiveExecutionContext.with_downloader_arguments` and survives every error-recovery context rebuild. The gallery-dl direct-download path forwards `snapshot.gallery_dl` as the `SidecarV2Command::download` `user_args`, which the Python worker passes to gallery-dl's argv (structure/argv-boundary only, no duplicate allowlist). The aria2 path threads `snapshot.aria2` through `transfer_with_one_refresh` → `transfer_once` → `aria2_config` → `Aria2SupervisorConfig::with_user_args`, and `command_args()` appends them after the application-owned options so a user value wins for a shared option while every protected option was already rejected upstream; the RPC `--conf-path` secret is still appended last by the spawner. The internal worker `executable_args` (Python-worker launch) is kept separate from user args. New targeted tests: `xarchive-download` 46/46 (user_args appended after application options; empty user_args leaves the argv shape unchanged) and desktop `aria2_config_forwards_validated_user_args_into_the_spawned_config`. Still absent: E3 (no submit path writes a v3 spec yet — every production submit still encodes `schema_version: 2`; there is no UI JSON-array input, no export redaction, no fake-process acceptance), and re-verifying the persisted snapshot in Rust immediately before spawn. Until a submit path and UI emit v3 with user args, E is wired but not yet reachable as a production user feature.
8. **F (`IN_PROGRESS` at plan level; streams individually tracked):** F1 typed preference, pre-render application, fallback and shared token audit are implemented; Windows appearance/native checks remain queued. F2 versioned local configuration exchange and F3 download/archive record exchange remain `PLANNED`; their shared implementation contracts and acceptance details are recorded in §11, with concrete schemas, JSON/YAML compatibility mapping, and failure-injection cases still required before coding. Neither stream is a Windows-only blocker.
9. **G (`IN_PROGRESS`):** after remaining shared development is implemented, run
   the risk-based integrated Linux regression, audit the final diff/docs, then
   commit and formally hand off to Windows. H is the subsequent Windows-owned
   implementation/validation phase; its blocked/skipped checks do not replace
   unfinished shared development or count as G completion.

The C1 subphases are `CROSS_PLATFORM_CHANGE_REQUIRED`: they change shared recovery,
workflow architecture, coordination and persistence semantics. E protocol/config
changes and F exchange schemas are also `CROSS_PLATFORM_CHANGE_REQUIRED`; Windows-native process, filesystem, GUI,
packaging and artifact acceptance remain Windows-owned and must not block these
shared batches unless a specific Windows behavior is a demonstrated prerequisite.

Batch A is implemented for the narrow page relocation only. It is not evidence that
all intended screens, controls, fields, or Windows acceptance are complete.

P0/P1 execution decision (2026-10-07): global output settings are persisted in
`settings_meta` and captured when one-off tasks or account batches are created;
Browser Native Host, Named Pipe and WebSocket submissions capture the validated
settings snapshot at runtime generation startup. Replacing the executor also
refreshes the snapshot captured by those listeners. The settings UI explicitly explains
that naming and optional-export effects remain inactive until independent D0
recovery facts are implemented. The user requested moving C1/D1 earlier, but
the ordering is rejected as unsafe: current recovery validates and reconstructs
commits from `tweet.json`, so disabling that user export before D0 would make
recovery depend on a switch-controlled file. Preserve D0 → C1 → D1.

## 4. Confirmed contract decisions

The following choices were confirmed by the user on 2026-10-06/07 and are the
implementation baseline. Screenshots remain interaction examples, not a visual
specification.

Privacy/recovery decision confirmed by the user on 2026-10-07: the internal
recovery manifest stores only the minimum facts needed to verify/recover archive
identity and media (Tweet/Job identity, final relative paths, sizes, hashes, and
Telegram intent linkage). It must not copy Tweet text, author metadata, or
reply/quote relationships. The Download Config UI warns that these minimum facts
remain stored internally even when optional user metadata exports are off.
Template tokens are limited to the flat `{username}`, `{tweet_id}`, and
`{index}` example; extension remains application-controlled.

1. Preserve the independent v2 execution-spec envelope for output settings. Read
   historical v1/v2 rows without rewriting them; reject unknown versions
   explicitly. Add v3 for downloader-option snapshots. The database
   `schema_version` column is authoritative; do not duplicate it in JSON.
2. Snapshot output settings and typed downloader options/policy identity only. Do
   not snapshot the entire runtime environment. Telegram configuration continues
   to be sampled at archive completion.
3. The current D1 output contract has naming mode/template and independent JSON/YAML
   switches (`export_json`, `export_yaml`). Both formats default on. They serialize
   the same explicit metadata DTO and schema version; their parsed records must be
   semantically equivalent. Do not expose field selection or custom text formatting
   in the first implementation. Preserve historical execution-spec versions and
   their `export_text` meaning exactly; new D1 snapshots require explicit version
   dispatch and must never reinterpret TXT as YAML.
4. Keep `Tweets/<tweet_id>` fixed. First naming template is a flat media filename;
   extension is application-controlled. Reject conflicts; never overwrite or add
   an implicit random suffix. Rust is the single renderer for preview and commit.
5. Keep internal recovery facts in a database journal plus an internal manifest,
   separate from user JSON/YAML files. A present corrupt/unknown internal record is
   an explicit recovery error, not a reason to fall back to stale user exports.
6. Capture one output-settings snapshot when an account batch is created; all its
   child jobs inherit it, including after pause/resume. One-off jobs snapshot at
   submission. Reusing an active job never replaces its persisted snapshot.
7. Complete and verify independent recovery facts before exposing JSON/YAML output
   disable switches. Existing archives are not rewritten.
8. Downloader arguments use a controlled allowlist in the first release, not
   arbitrary argv or shell parsing.

## 5. C1 transaction/workflow and ownership decisions

The user confirmed the following implementation choices on 2026-10-09:

1. Add a dedicated `xarchive-workflow` workspace crate for reusable workflow
   policy/contracts; C1 archive recovery is its first consumer. Keep the initial
   framework bounded to typed workflow steps, idempotency, recovery coordination,
   error classification and failpoints needed by C1. Do not migrate unrelated
   workflows or build a general distributed scheduler. The workflow crate must not
   depend on Desktop/Tauri or create a dependency cycle with Storage. Put port
   contracts inward and infrastructure adapters outward; keep SQLite transaction
   implementation in `xarchive-storage` and runtime scheduling in Desktop.
2. SQLite transaction atomicity covers database facts only. File rename and
   directory commit are separate durable workflow steps recovered by persisted
   state and file-identity checks. Telegram network sending is not part of local
   finalization.
3. Support multiple processes on the same host concurrently operating on one
   supported local data workspace. Different Jobs may proceed concurrently;
   same-Job execution, recovery, retry/attempt increment, cancellation and cleanup
   must use cross-process coordination. Protect final archive destinations too,
   because distinct Jobs can contend for one path. Do not claim cross-host or
   network-filesystem distributed execution. Select and validate OS-backed
   coordination adapters; an in-process mutex or an expiring DB lease alone is
   insufficient to fence live workers or their child processes. Attempt fencing
   must be checked in transactions that write durable Job, journal and archive
   facts, and stale workers must be prevented from mutating files.
4. Bind each attempt to isolated staging and explicit execution identity. Recovery
   resumes the journaled attempt; retry creates a new attempt only after the old
   attempt no longer owns commit rights. Do not let an old worker write into a new
   attempt's staging tree.
5. Persistent `PREPARED` is the irreversible commit boundary. Before it, a
   coordinated cancellation may win and prevent commit. After it, finish or recover
   the commit; cancellation must not rewrite the Job as cancelled or remove
   committed files. Persist all required candidate facts and local intent linkage
   before entering this boundary.
6. Do not enlarge the immutable recovery manifest with Tweet text, author data or
   relationship metadata. Reuse stable, attempt-bound durable business facts where
   sufficient; otherwise add a separate, minimum attempt-bound durable candidate
   for recovery, independent of user JSON/TXT exports, with defined lifecycle and
   cleanup rules. Never persist signed URLs, request headers, cookies or browser
   secrets in it.
7. Keep the current whole-phase journal rather than adding per-file progress rows
   initially. Replay must reason over the complete immutable path mapping and
   actual verified file identities; ambiguous or conflicting states fail closed.
8. Use an atomic platform no-replace filesystem operation for protected moves.
   Do not silently fall back to an overwrite-capable rename. Windows-native
   implementation and NTFS evidence remain Windows Owner responsibilities. The
   initial shared acceptance target is process interruption/restart, not sudden
   power-loss durability. Do not silently replace atomic moves with copying.
9. A `COMMITTED` replay is read-only verification of the final directory, every
   planned media identity, database media/path set, completed Job and applicable
   local intent linkage. Mismatches are reported without automatic repair.
10. **Implementation analysis / selected architecture:** archive service and DB
    facts are in `xarchive-storage`, while scheduling/startup entry points are in
    Desktop. The new workflow crate owns workflow contracts and coordination policy;
    Storage implements persistence/file adapters and atomic DB finalization; Desktop
    invokes workflows and owns runtime scheduling. Implement this API as C1 work,
    not as a reason to defer it. Windows-native behavior and NTFS acceptance remain
    Windows Owner responsibilities and do not block shared Linux implementation.

### C1 execution sequence and acceptance gates

The following sequence is the active implementation plan for the current
uncommitted continuation. It does not mark any item completed or verified merely
because related helper code exists.

1. **B0 — documentation and baseline gate (`VERIFIED` for this N0 slice):** preserve all existing
   worktree changes; reconcile this Plan, architecture, handoff and Windows queue
   against inspected source. `claim_archive_job` is a legacy conditional state
   transition only; the current WIP also adds `claim_archive_attempt`, which
   atomically advances attempt/state and records the transition event, but is not
   yet the complete production admission/session contract. Keep Windows-specific
   adapters/NTFS/native-process work in the Windows queue; do not route shared
   implementation gaps there. Repair the current Desktop persistence trait/impl
   placement and claim fixtures, then run compile, focused tests, `git diff --check`
   and the repository docs audit. The bounded N0 slice passed; this does not
   validate production admission or close B1–B7.
2. **B1 — admission and backpressure (`IN_PROGRESS`; local reservation exists, lifecycle boundary incomplete):**
   `runner_try_admit`/`RunnerReservation` currently reserves local runner capacity
   in `execute_persisted_from_factory` and `execute_persisted_on_worker`, but the
   claim and state writes still happen on the caller before work is dispatched to
   the runner. The reservation is released when the runner returns, before
   post-run persistence and archive finalization. Therefore the current mechanism
   does not yet prove admission-before-claim at the actual execution boundary or
   keep capacity through finalization. Preserve bounded/reschedulable rejection;
   complete B1 as part of the runner-owned lifecycle rather than describing the
   existing local counter as end-to-end admission. Do not mask backpressure with
   an unbounded queue.
3. **B2 — atomic state-plus-attempt claim (`IMPLEMENTED` as a persistence primitive and caller-path wiring; runner/session ownership open):** the Storage `claim_archive_attempt` transaction already checks
   expected state/attempt and PREPARED facts, increments the attempt with checked
   arithmetic, and writes the initial running state plus its event atomically.
   Both `execute_persisted_from_factory` and `execute_persisted_on_worker` use
   `claim_attempt` for the first running transition (instead of a separate
   `begin_attempt` followed by an unfenced first write), so a losing claimant
   returns the winner's snapshot with no side effects and a Job already in its
   running state still obtains an attempt identity via `begin_attempt`. This does
   not establish a production `ExecutionSession`: the durable claim is still
   outside the runner-owned factory/execution scope. PREPARED/COMMITTED routing to
   recovery stays fail-closed.
4. **B3 — runner-owned execution session (`IN_PROGRESS`; two-phase shared contract
   implemented and proven, production wiring open):** once a runner has
   accepted and dequeued work, acquire the Job and destination scopes, perform
   the atomic claim *under those scopes*, and create an immutable attempt
   identity/session using the existing Workflow coordinator port. Pass it
   through the execution factory and retain its guard for the full writer and
   publication lifecycle. Use deterministic lock ordering fixed by the adapter.
   Test one winner for concurrent same-Job claims, parallel distinct Jobs,
   same-destination serialization, and guard release on every exit path. A
   session-owned result must not be overwritten by a second generic persistence
   path.
   - Implemented: `ExecutionReservation::acquire` takes the Job *and*
     destination scopes with no attempt known yet, then
     `ExecutionReservation::claim` runs the caller's durable claim transaction
     under that guard and binds the session to the attempt the claim returned.
     The ordering is a callback rather than a parameter, so it cannot be
     inverted: an earlier single-step `ExecutionSession::begin` that accepted a
     pre-built `AttemptIdentity` was removed precisely because it permitted the
     claim-then-lock sequence. `ExecutionSession` is generic over the
     `Coordinator` port and therefore compiles on Windows; `SessionError`
     separates `Contended` and `ClaimLost` (both reschedulable, both reported
     through `is_reschedulable`) from `InvalidScope`/`Io`/`StaleAttempt`/
     `ClaimRejected`, so contention and a lost race can never be persisted as a
     download failure. A session also binds its final destination, which
     `authorize` alone does not fence. A lost, failed or foreign-job claim drops
     the reservation, so a Job that never starts is never left locked.
   - Proven on Linux with a real second OS process
     (`xarchive-lock-probe` in `session` mode, which itself claims through the
     two-phase entry point): a live peer session makes the same job contended
     and releases with the process; a peer on another job does not block this
     one; two different jobs sharing a destination serialise; a crashed peer
     does not block a successor; a stale attempt is refused. Two mutations fail
     the suite (dropping the destination scope, accepting a foreign-job
     identity); a third that inverted the ordering was rejected by the borrow
     checker.
    - Still open for B3: the Desktop executor does not yet construct a session,
      so no production path holds a Job/destination guard across claim, writer
      shutdown, publication or DB finalization. **Chosen architecture:** use a
      concrete generic execution wrapper, keep the Executor/factory object-safe,
      and let the runner own an independent persistence context and local session.
      Current-call-graph review shows no safe standalone production insertion
      point: the entry claims before factory creation, and the factory creates a
      Job-scoped staging writer. Do not add a partial session around that writer;
      combine production wiring with B4/B5 attempt-owned staging, proven writer
      exit and fenced publication/finalization. This avoids propagating
      Coordinator generics or changing Workflow to an owned/shared coordinator.
      The Windows adapter implements the same port and remains Windows Owner work;
      do not infer NTFS behaviour from Unix `flock` tests.
5. **B4 — attempt staging and writer lifetime (`IN_PROGRESS`):** wire both gallery-dl
   and aria2, archive metadata and temporary files to the session's unique attempt
   directory. Validate every reported path is within that attempt. Prove all child
   writers/process trees have exited before prepare, publication, guard release or
   cleanup; a completion message alone is insufficient. Cancellation must stop and
   join writers. Define restart recovery and conservative orphan cleanup so a stale
   attempt can neither publish nor delete newer files. Exercise both backends with
   fake processes, including late writes and parent/child termination. Windows
   coordination adapter and NTFS/native process validation remain Windows Owner work.

   **Git publication checkpoint (2026-10-13):** independent batches committed as
   `6d1f70a` (shutdown outcomes), `ccf2375` (execution teardown), `5917198`
   (Sidecar descendant cleanup), and `b5e5ab2` (writer-domain primitives).
   Publication regression: Download 52/52, Sidecar supervisor 16/16, Workflow
   33/33 plus integration/doctests, Desktop 298/298, Python protocol-v2 26/26,
   fmt and documentation audit PASS. Ownership remains Cross-platform Owner;
   this progress sync does not complete B4/B5 or enable v2.

   **2026-10-13 directory durability increment:** Unix fresh creation syncs each
   newly created ancestor and its descriptor-held parent, then the attempt and
   its parent before success. Sync failure returns `Io` and may leave partial
   directories; exclusive attempt creation still refuses reuse. Workflow 32/32,
   doctests 7/7 and strict Clippy/fmt pass. No power-loss simulation or directory
   sync fault injection; existing ancestor durability, mkdir/open replacement,
   writer containment and production integration remain unproven. B4/B5 remain
   incomplete and v2 stays fail-closed.

   **Directory-sync error verification follow-up:** private fault injection now
   covers all six newly-created staging/job/attempt child-parent sync positions,
   verifies descriptor identity/order, immediate failure and partial-directory
   state, and checks that retry succeeds only before an attempt already exists.
   Production remains fixed to `File::sync_all`. Workflow 33/33, doctests 7/7 and
   strict Clippy/fmt pass; this replaces the missing fault-injection evidence
   above, not the missing power-loss, existing-ancestor or containment evidence.

   **Retry durability correction:** fresh creation now also syncs existing
   traversed ancestors and parents: `EXIST` does not establish that a previous
   failed creation synced them. Regression verifies descriptor sync order on
   retries as well as initial failure and attempt refusal. Workflow 33/33,
   doctests 7/7 and strict Clippy/fmt pass. This supersedes new-directory-only
   syncing, adds sync cost and fails closed if directory sync is unsupported;
   power-loss, namespace isolation and containment acceptance remain open.

   **2026-10-12 progress checkpoint:** `ProductionExecutionFactory::create_for_attempt`
   now carries the claimed `AttemptIdentity` into `ArchiveExecutionContext`; the
   legacy factory entry rejects execution without that identity. Sidecar staging
   resolution creates only `<job>/attempt-<n>` and refuses a missing or mismatched
   identity. Desktop regression tests cover both outcomes. Production now resolves
   downloader media staging beneath the claimed attempt; this does not yet make
   attempt staging the full production commit source: metadata/temp-file placement,
   reported-path validation as an attempt-wide set, writer/process-tree join,
   and fenced publication/finalization remain open. `InternalRenameV2` remains
   rejected before Sidecar execution, so this progress does not open B5/B7 or
   production release gates.

   **2026-10-13 progress checkpoint — `CROSS_PLATFORM_CHANGE_REQUIRED`:**
   `xarchive-workflow::WriterDomain` now binds a validated relative-path
   authorization to one `AttemptIdentity`; the Unix adapter exclusively creates
   a fresh `<staging>/<job>/attempt-N` root and supports read-only lookup of an
   existing root. Tests cover attempt identity, path traversal/absolute-path
   rejection, duplicate creation refusal, retry after removal of a partial
   fixture, and idempotent existing-root lookup. This is a shared path/attempt
   contract only: it does **not** revoke raw filesystem access, contain Sidecar
   or aria2 processes, prove descendant exit, or authorize publication. It is
   not wired into production yet. The environment is WSL2 with cgroup v2
   controllers visible but `/sys/fs/cgroup` not writable, so cgroup writer
   containment acceptance is `BLOCKED`; v2 remains fail-closed.
   Follow-up boundary review rejects unsafe Job path components and validates
   all existing-root ancestors. This object is a path binding, not a live
   session authorization; its constructors do not require a held session.
   Lexical path checks and snapshot link checks cannot replace descriptor-relative
   I/O or prevent concurrent directory replacement. Those gates remain open.
   `WriterDomain::bind_session` now creates a borrowed, non-cloneable
   `SessionWriterDomain` only for the held session's matching attempt; relative
   resolution checks that session again. Workflow unit tests pass 28/28 with
   captured exit 0. Returned paths are not durable I/O permits; production
   containment, descriptor-relative writes and B5 integration remain open.
   Follow-up adds Unix `SessionWriterDomain::create_new_file`: the operation
   borrows the held session, opens every existing directory component with
   descriptor-relative NOFOLLOW, exclusively creates and syncs the file, and
   returns no writable handle. Tests reject root/intermediate/final links and
   overwrite. Directory creation, namespace rename attacks, crash durability
   of directory entries, subprocess containment and production wiring remain
   open; failed writes may leave partial files that retries must not reuse.
6. **B5 — fenced publication and durable finalization (`PLANNED`):** require the
   live attempt identity and held Job/destination authorization at final destination
   no-replace publication and at transactional DB finalization. Keep cancel/PREPARED
   arbitration: cancellation may win only before durable PREPARED. A counter check
   before a publish call is not a fence. Test stale-attempt denial, destination
   collision, prepare/cancel races and complete rollback of DB facts/events.
6. **B6 — shared commit/replay and restart matrix (`PLANNED`):** route normal
   completion and recovery through the same journaled rename/directory-commit
   workflow. Connect failpoints to real operation boundaries; use on-disk SQLite,
   real files and child processes, reopen the service, and prove replay idempotency.
   Verify COMMITTED files/hashes, DB paths/media, Job completion and local Telegram
   intent read-only; mismatches fail closed. Cover malformed/missing records, every
   rename/commit/finalize crash point, conflicts and stale attempts.
8. **B7 — Desktop recovery integration (`PLANNED`, gated on B1–B6):** wire startup
   recovery and normal production execution while preserving legacy/InternalV1
   compatibility. Keep v2 fail-closed until an integration test proves claim,
   writer shutdown, stale-attempt denial and destination commit, and applicable
   Linux target gates pass. Keep D1 disabled until the shared C1 gates pass.
9. **Windows implementation and acceptance (`PLANNED`, Windows Owner):** implement
   Windows-specific coordination and NTFS no-replace adapters, then validate native
   process-tree cancellation and the integrated recovery matrix on a fresh artifact
   bound to the formal handoff SHA. These Windows-owned tasks follow shared Git
   handoff and do not block the shared Linux implementation.

### C1-A implementation checkpoint — 2026-10-13

- **Status: shared implementation remains `IN_PROGRESS`; production session
  wiring is not safe to enable.** The persisted entry reserves local runner
  capacity and atomically claims the first running transition before dispatching
  to the runner's factory/execution path. The reservation is released when the
  runner returns, before post-run persistence/finalization. Neither this local
  reservation nor the atomic claim is a production `ExecutionReservation` /
  `ExecutionSession`; the cross-process Job/destination guard is not held through
  writer shutdown and finalization.
- **Verified design sequence:** B1 admission/backpressure, B2 atomic state-plus-
  attempt claim, B3 runner-owned session/guard, B4 attempt staging and confirmed
  writer exit, B5 fenced publication/finalization, B6 shared replay/restart matrix,
  then B7 Desktop recovery integration. The sequence above remains actionable;
  this checkpoint records an interface constraint, not a blocker on further
  shared design or implementation.
- Concrete integration boundary verified in source: `ExecutionSession` is
  `Send` (compile-time assertion in `crates/xarchive-workflow/src/execution.rs`),
  but borrows a concrete coordinator and is not object-safe. The executor's
  factory/runner boundary is trait-object based, while `JobPersistence` is
  borrowed by the scheduling caller and cannot be moved into its `'static` runner
  queue. The shared design must therefore explicitly own both a concrete
  coordinator and an isolated persistence context inside a generic runner-owned
  wrapper; that wrapper must acquire Job/destination scope before claim and retain
  the session through writer exit and finalization. Do not use unsafe lifetime
  extension, a lock created after claim, Unix-only fallback, or a no-op Windows
  adapter.

Validation escalates by impact: targeted concurrency/recovery tests, affected
module suites, subsystem restart matrix, then full regression when justified by
the cross-module/security-sensitive change. Each result records exact source SHA,
commands, environment and status. Windows checks remain `NOT_RUN` until actually
executed; no Linux result substitutes for NTFS/native acceptance.

## 6. E validation ownership decision

Rust is the sole authority for user downloader-option policy validation. Validate
at task/settings acceptance and again immediately before execution, including
contract and tool-version compatibility. Python validates protocol structure and
preserves argv boundaries but does not maintain a duplicate allowlist or value
policy. Trusted internal `executable_args` remain a distinct trust domain. Python
must not filter, rewrite or shell-parse validated arrays. The Sidecar v2 model and
worker accept optional `download.user_args`; Batch E2 also wires the decoded
snapshot through the gallery-dl download command and aria2 argv construction.
Production task and batch paths now persist v3 snapshots with Rust validation.
Rust and Python contract tests must remain synchronized. Shared E3 Linux
acceptance has passed, but this is not Windows acceptance or a formal handoff.
The gallery-dl user-argument allowlist remains empty, so those arguments continue
to be rejected; aria2's reviewed options are bounded by the Rust policy.
Rust constructs aria2 argv directly from the same task snapshot. Bind accepted
options to an explicit policy/tool compatibility matrix; initially accept only
reviewed exact tool versions. Unknown or incompatible versions fail before spawn,
without silently dropping or rewriting saved arguments. Python may retain checks
for protocol structure and application-owned execution invariants, but not a
duplicate user-option allowlist/value policy.

The execution-spec decoder rejects unknown fields and explicitly dispatches v1,
v2 and v3. Preserve v1 and v2 historical meanings; v3 carries user
downloader-option snapshots, policy version and required tool compatibility
identity. Historical v1/v2 tasks decode with empty custom arrays and are not
resampled from current settings. Execution-spec version, recovery-contract
version and argument-policy version are independent; dispatch each explicitly.

The historical v2 envelope is `{ "request": <historical ArchiveTweetRequest>,
"output_settings": { "naming_mode": "original" | "template",
"filename_template": string, "export_json": boolean, "export_text": boolean } }`.
This is a frozen legacy contract: preserve its TXT behavior and stored meaning.
The request JSON keeps its v1 field shape; v2 adds only the historical typed output
snapshot. The new D1 JSON/YAML contract must use a separately dispatched version
or equivalently explicit versioned representation; it uses `export_json` and
`export_yaml`, never aliases `export_text`, and preserves the database version as
the authority. New-format unknown fields are rejected rather than ignored. Defaults
for new D1 settings are `original`, an empty template, and both formats enabled.

These are shared-contract decisions: implementation escalation is
`CROSS_PLATFORM_CHANGE_REQUIRED`.

## 7. Batch B — Versioned execution specification
Keep the internal typed specification out of the Browser request protocol and
reuse `archive_job_requests`. Explicitly dispatch historical v1, output-snapshot
v2 and downloader-arguments v3; unknown versions fail with a stable explicit
error before creating a Sidecar process. The DB row version is authoritative.
Do not rewrite historical rows. Existing rows remain on their stored schema;
current E3 submissions capture downloader arguments in v3 specs.

One-off tasks snapshot validated current settings at submission. Batch tasks
snapshot once at batch creation and persist those settings with the batch; every
child job inherits that exact snapshot. Active-job reuse and retry retain the
stored snapshot. Telegram configuration remains completion-time sampled.

Before implementation, verify that job creation and execution-spec persistence
are atomic or add a narrow transaction so no newly created Job can be left without
its required spec. Avoid a migration if the existing row can carry v2 JSON and
the current positive version column unchanged; a batch snapshot does require a
durable batch field/migration unless an equivalent existing storage slot is
identified and validated.
historical version-1 request behavior. Add version dispatch or migration only if
the agreed new snapshot representation requires it; do not duplicate or replace
the existing request-version handling without evidence of a compatibility gap.

### Acceptance

- Version-1 jobs retain their existing output behavior and are not rewritten.
- Version 2 persists and reloads the exact agreed typed output snapshot.
- Unknown versions are rejected before external process startup.
- All child jobs of a batch use the batch-creation snapshot despite later global
  settings changes, pause/resume, or restart.
- Unknown/corrupt versions fail explicitly and cannot execute with guessed
  settings.
- Reused active jobs retain their original request and output snapshot.
- Retry/startup recovery use the persisted spec, not changed global output values.
- Telegram completion-time sampling behavior remains unchanged.

Escalation: `CROSS_PLATFORM_CHANGE_REQUIRED`.

### C1 continuation disposition — 2026-10-08

`CROSS_PLATFORM_CHANGE_REQUIRED`: before C1 can be accepted or production v2
enabled, the shared implementation must complete and test (a) transactional
finalization of Tweet/media rows, Job completion, v2 journal state, and Telegram
intent lifecycle; (b) v2-aware startup/replay dispatch for legacy, InternalV1,
InternalRenameV2, missing, corrupt, and unknown records; (c) atomic attempt fencing;
and (d) service-level failpoints and replay across durable write, every rename,
directory commit, and finalization. A COMMITTED record must be verified against
the final directory and durable DB facts on replay. Keep production v2 fail-closed
until this gate passes. These are shared development requirements, not Windows
manual checks.

## 8. Batch C — Filename rendering and recoverable path mapping

Keep `Tweets/<tweet_id>` fixed. A template produces media-relative paths only;
`/` may denote an archive-internal subdirectory if the validated design permits
it. One Rust renderer/validator must serve preview and actual output. Do not
reimplement the naming algorithm in JavaScript. Custom text and tag-editor modes
must preserve templates losslessly or leave the user in custom mode with an
explicit explanation. Keyboard-accessible tag reordering is required; drag/drop
is optional.

Before changing files, compute and validate the complete mapping (safe relative
paths, then persist a recovery record). The planned `xarchive-workflow` crate and
its cross-process coordination contract are prerequisites to production C1 wiring.
Renames must be idempotently recoverable across crashes, verify source/destination
identity and hash/size, and never overwrite.
Update the unified archive result to final relative paths before capturing
Telegram intent. Keep existing commit destination refusal semantics.

The first template is flat and may use only fields whose provenance is verified
end-to-end. In particular, protocol `filename` is not assumed to mean original
source filename. Missing, zero, and unknown values must have explicit semantics.
Subdirectories, arbitrary field selection, and extension substitution are out of
scope for this first naming implementation.

Acceptance includes preview/output equivalence, both transfer modes, invalid and
duplicate names, path traversal/platform restrictions, and injected failures
before/during/after rename and before/after directory commit. Verify database
media rows, user metadata, Telegram payload references, and recovery all use the
same final path mapping. Windows path behavior remains pending until tested on a
fresh Windows artifact.

Escalation: `CROSS_PLATFORM_CHANGE_REQUIRED`.

## 9. Batch D — User metadata and internal recovery facts

Separate durable internal recovery facts (identity, final media mapping, size,
hash, phase, and required intent linkage) from optional user exports (`tweet.json`
and `tweet.yaml`). Internal recovery must not depend on user export switches and
must not store signed URLs, cookies, or browser credentials. New D1 jobs default
to both formats enabled. Do not rewrite historical metadata or reinterpret old
`export_text` snapshots; legacy jobs retain their stored TXT behavior, and existing
`tweet.txt` files are not renamed or rewritten. The
internal journal/manifest foundation is D0 and must be completed before C1 rename
operations depend on it; user-facing output switches are D1 after C1 acceptance.
Field selection and custom text formatting are deferred beyond this first release.

Recovery precedence: valid new internal record; otherwise explicitly recognized
legacy format; a present but corrupt/unsupported new record is a clear failure,
not a fallback to stale legacy files. Coordinate its schema and path map with
B/C; a sidecar JSON file alone does not establish crash consistency.

Acceptance covers all four JSON/YAML combinations; semantic equivalence of both
serializations from the same metadata DTO; serialization/write failure before
publication; legacy/new/missing/corrupt/unknown-version recovery; preservation of
historical TXT snapshots and files; and crashes with user exports disabled. Recovery
uses internal journal facts and never depends on either user-facing export.

Escalation: `CROSS_PLATFORM_CHANGE_REQUIRED`.

## 10. Batch E — Downloader arguments

Desktop Download Config is the proposed single settings authority for both
ordinary tasks and Extension-originated archive requests. Browser request
protocols carry archive metadata only; they do not carry downloader settings.
Read and validate current settings in Rust at submission, snapshot once for a
one-off task, and snapshot once at batch creation. Every batch child, retry,
resume and active-job reuse retains the persisted snapshot; dispatch must never
sample current global settings. This settings authority and snapshot contract are
`CROSS_PLATFORM_CHANGE_REQUIRED`.

Provide separate gallery-dl and aria2 settings as JSON string arrays, preserving
argv boundaries without shell-like parsing. Persist the typed arrays in the task
spec and validate again immediately before execution. Never invoke a shell. Do not
conflate tool argv with Sidecar worker program/args. The arrays are passed as
individual process arguments; they are not a command-line string.

The UI and persisted representation are JSON string arrays; there is no
shell-like tokenization. The official aria2 1.37.0 manual and gallery-dl
configuration documentation are the reference sources for reviewed options.
The supported option set must be versioned and intentionally narrower than
either tool's full CLI/configuration surface. Before enabling the feature, pin
the packaged tool versions and add the exact accepted option names/value types
as shared fixtures. Reject application-owned input/output, config-file loading,
RPC, proxy, credentials, hooks, daemonization, metadata/reporting and other
indirect override routes. Attached, short-option, `--`, and configuration
indirection forms are rejected unless a specific allowlisted form is explicitly
represented by the parser contract. Official references: aria2 manual
(`https://aria2.github.io/manual/en/html/index.html`) and gallery-dl
configuration (`https://gdl-org.github.io/docs/configuration.html`).

Preserve historical execution spec v1/v2 and add v3 for argument snapshots,
policy version and tool compatibility identity. v1/v2 decode to empty custom
arrays; active reuse, retry and recovery never resample settings. The database row
version is authoritative, and execution-spec, recovery-contract and argument-policy
versions are dispatched independently.

Trace and test both paths end-to-end:

```text
execution spec -> explicit Sidecar config -> gallery-dl argv
execution spec -> Rust aria2 command construction -> aria2 argv
```
Reject options that replace application-controlled input/output/protocol,
disable required metadata, execute hooks/commands, or bypass shared proxy policy;
check indirect config-file routes too. Rust rejects unknown user options before
process startup; do not pass them through to the tool or silently drop them. Do not log full argv.
No execution-spec export API was found during the E3 documentation gate. Do not
describe export redaction as an existing feature or add an export surface in E3.
If Batch F later introduces configuration exchange, its default contract must
exclude both custom-argument arrays. UI accepts JSON arrays only; no shell
tokenization, escape processing, pipes, or redirection semantics.

Acceptance covers quoting, escaping, Unicode, ordering, protected/indirect
overrides, inactive-engine save/apply behavior, cancellation/failure/timeout,
and redaction. Windows child-process semantics remain a Windows-owned check.

Escalation: `CROSS_PLATFORM_CHANGE_REQUIRED` if Sidecar/config protocol or shared
execution semantics change; use `CROSS_PLATFORM_REVIEW_REQUIRED` only for a small
implementation preserving an existing contract.

## 11. Batch F — Theme, configuration exchange, and record exchange

Theme authority is persisted `system | light | dark`. Audit all pages, fields,
warning tokens, previews, labels, logs, and status badges. WebView theme evidence
does not prove native titlebar/dialog appearance.

Configuration exchange and download/archive record exchange are separate formats,
commands, previews, and import flows. Configuration exchange uses a dedicated
versioned local JSON file, never a URL or blind replacement of `config.yaml`.
The user confirmed that archive-directory and Sidecar executable paths may be
included, but when an imported path does not exist on the target machine the
current local value is preserved and the mismatch is shown in preview. Never
create directories, install/launch tools, or restart automatically. Secrets,
tokens, cookies, and credentials are excluded. Preview changed/preserved/excluded
fields and restart requirements; strictly validate a candidate before writing;
save atomically and report success only after persistence. Do not force-restart.
- **F3 application decision (confirmed 2026-10-12):** preview archive-record
to preserve queued work without silently interrupting active work. Preview is
available while jobs run; applying requires explicit confirmation to wait. Then
close admission across the shared workspace, let active executions finish through
archive finalization, and apply only when quiescent. Waiting is cancellable; cancel
reopens admission and leaves configuration unchanged. After quiescence, recheck
the preview's configuration revision and all path/validation assumptions; stale
preview requires a new preview/confirmation. Persist the validated candidate
atomically before changing runtime configuration. On write failure retain the old
runtime and durable configuration and always release maintenance mode. Show
restart requirements explicitly; never restart or launch tools automatically.
Maintenance admission must be coordinated across processes, not inferred only from
the current GUI's active-job list. Define crash-safe recovery so a process exit
cannot leave durable maintenance mode permanently blocking the queue.

Download records and archive records are separately exported/imported as distinct
files with their own state and hash metadata. The user confirmed optional import
of both files from ZIP or 7z; other archive formats are deferred. Each may also
be imported individually, with an explicit GUI warning describing the behavior
when the counterpart record file is absent. Import previews first and merges
idempotently; path/size/hash conflicts reject the affected archive record. Import
never re-downloads, overwrites archive files, or marks a Job COMPLETE without
verified matching facts. Each export GUI displays a warning banner about possible
sensitive information. Configuration files and record files are never combined.

**F3 application decision (confirmed 2026-10-12):** preview archive-record
conflicts record-by-record and allow partial acceptance; the user-confirmed
accepted set is committed in one transaction. Applying requires explicit
confirmation to enter the same cross-process maintenance admission barrier as F2,
drain active work through archive finalization, and then recheck preview revisions
and assumptions before committing. Waiting is cancellable; cancellation releases
the guard and leaves records unchanged. Import supports individual JSON and YAML
files first; ZIP and 7z are staged follow-up containers using the same validated
import pipeline. Keep configuration exchange separate from record exchange. A
failed validation/write leaves existing configuration/records intact and must not
use startup's fallback-to-default path as import success. Do not expand shell or
filesystem permissions. ZIP/7z
extraction must reject traversal, links, duplicate/ambiguous entries, and
unexpected payloads. TAR, gzip and other formats are future work.

**Shared F2/F3 contract baseline (recommended implementation defaults, approved
for this batch on 2026-10-12; `CROSS_PLATFORM_CHANGE_REQUIRED`):** F2 uses a strict,
versioned JSON envelope and explicit field allowlist. Unknown versions or fields
are rejected with a field-level preview error; secrets, tokens, cookies and
credentials are never serialized. Target-local path fields are accepted only
when they validate on the importing machine; otherwise preserve the existing
local value and report the mismatch. Apply is bound to a revision of the exact
persisted configuration snapshot, and stale previews must be regenerated.
Maintenance ownership is a process-lifetime coordinator guard over the same
workspace admission domain as execution; do not use a GUI-local active-job count
or a durable boolean as the authority. Admission closes before drain, active work
drains through finalization, waiting is cancellable, and guard release on owner
death resumes admission. Durable configuration is committed before runtime
replacement; restart-required fields are reported but never auto-restarted.

For F3, separate versioned download-record and archive-record schemas define one
canonical data model shared by independent JSON and YAML import/export files.
Both formats use the same validation, preview, and merge pipeline; conversion
between them preserves the canonical records. JSON and YAML imports bind to a
user-selected existing local archive root and relative paths, and require local
file type/size/hash evidence before accepting archive facts; source-machine absolute
paths are informational only. Idempotency uses a stable record identity plus schema
version; importing the same records in either format is idempotent, and conflicting
duplicate identities reject the affected records. F3 apply must use the same
process-lifetime maintenance coordinator and workspace admission domain as F2, so
no task can start between drain, revision/evidence revalidation, and the single
transaction committing the user-selected accepted set. First ship
individual JSON/YAML input/output. ZIP/7z remain a later independently approved batch;
before selection, compare maintained libraries, license compatibility, security
history, resource limits and required traversal/link/duplicate/unexpected-entry
rejection tests. These defaults do not authorize adding a container dependency.

The behavior decisions above do not freeze the shared implementation contract.
Before F2/F3 coding, translate the approved defaults above into concrete schema
definitions, compatibility policy, exact included/excluded fields and redaction
rules, configuration path merge/unknown-field behavior, and failure-injection
cases. For F3, additionally specify the versioned JSON/YAML representation and
round-trip compatibility rules, existing-root and on-disk evidence rules,
partial-acceptance transaction boundary, duplicate and idempotency keys, and
active-task admission barrier. Acceptance must verify JSON↔YAML round-trip
equivalence and idempotency when equivalent records are imported across formats.
ZIP/7z dependency selection
and license/security/resource review are required before any container dependency
is added. The F2 process-lifetime maintenance guard must prove crash-safe release
without a durable stale admission-closed state. These shared implementation gates
are not Windows blockers; do not implement guessed schemas, dependencies or
recovery protocols.

Acceptance covers round-trip, unknown/invalid schema, failure atomicity, F2
maintenance admission, graceful task drain, cancellable wait, revision recheck,
crash-safe release, explicit restart confirmation, theme token coverage, separate
record-file/ZIP/7z flows, incomplete single-file import warnings, conflict
handling, and Windows native UI separately.

## 12. Ownership, evidence, and progress

Shared contracts and cross-platform behavior belong to the Cross-platform Owner.
Windows-specific APIs/runtime/GUI/packaging and native acceptance belong to the
Windows Platform Owner. Do not interrupt non-Windows-dependent implementation for
ordinary Windows checks. Add Windows queue/manual items only when an implementation
revision and test target exist; keep planned features `IMPLEMENTATION_NOT_READY`
and distinguish them from implemented-but-`NOT_RUN` checks.

For each completed batch, record changed modules, tests selected/skipped and why,
results, known warnings, Windows work, and exact source/handoff revision. Batch C
and D require at least Storage/archive Subsystem validation; protocol/argv changes
cover both Rust and Python sides. Do not run Full suite automatically; escalate
based on actual blast radius. Final Windows results bind the source SHA, worker
build provenance, and tested artifact identity.

### Non-Windows continuation scope and decision

The 2026-10-09 uncommitted C1 continuation adds attempt-scoped Storage staging, identity-checked
rename replay, a Storage/Workflow bridge and Linux no-replace movement. Targeted
Storage and Workflow tests cover hash mismatch, missing files, symlinks, atomic
moves, and process-level locking. These helpers are still not wired into archive
creation or Desktop startup recovery. C1.2/C1.3/C1.4 are not accepted: production
v2 remains fail-closed until two-phase replay, atomic Tweet/media/Job/journal/
Telegram-intent finalization, persistent attempt fencing, COMMITTED filesystem+DB
verification and the service restart crash matrix are integrated. The Windows lock
and NTFS no-replace adapters remain Windows-owner implementation/validation work.
The statements in this historical checkpoint that typed task persistence and
gallery-dl/aria2 argv wiring were absent predate the current E3 implementation and
must not be used as current status. Current Plan status at the top of this document
records the v3 snapshots and Linux actual-spawn evidence. F2/F3 remain unimplemented
unless a later dated checkpoint says otherwise; F3 decisions are recorded in §11.
These are shared development items,
not Windows blockers.

### Next implementation batch — C1-A runner-owned claim/session boundary

N0 remains a baseline gate with targeted checks recorded as `VERIFIED` for that
bounded slice; it is not a prerequisite that blocks C1-A. C1-A is the current
shared design/validation checkpoint; production lifecycle integration is gated
on a safe combined B4/B5 insertion point. Keep production v2 fail-closed while
its lifecycle gates are incomplete.

### 2026-10-12 approved implementation scope and historical checkpoint

The user approved this delivery order: C1 production lifecycle/replay/Desktop
  then F2 and F3, then integrated G. D1 is confirmed as
independent JSON/YAML metadata exports from one versioned DTO and immutable task
snapshot; legacy TXT snapshots/files retain their historical meaning and are never
silently converted. F3 is confirmed as
record-by-record conflict preview with partial acceptance committed as one
transaction; apply through the shared maintenance admission barrier after a
cancellable drain and revision/evidence recheck; deliver individual JSON/YAML
files before ZIP/7z containers. These decisions do not mark any feature
implemented.

This continuation adds a Storage API that previews metadata from an existing
attempt staging tree and never falls back to legacy Job-level staging. Targeted
tests prove the attempt payload is used and a legacy-only payload is rejected.
It is an isolated safety primitive only: ProductionExecutionFactory,
`download_v2`, archive commit, writer shutdown, session fencing, and startup
recovery still use/expect the legacy lifecycle or fail closed. C1 B3–B7,
production enablement, D1, F2/F3, and integrated G remain open. Do not describe
this helper as production attempt-staging integration.

Chosen C1-A architecture: keep Executor/factory object-safe, but instantiate a
concrete generic runner whose execution method owns the coordinator, opens its
own persistence context, and locally acquires the Job/destination reservation
before performing the durable claim. The same runner scope owns the session and
attempt-sensitive claim result; return only a result/snapshot to the erased outer
boundary. Do not pass borrowed persistence or session values through `'static`
queues, do not use `unsafe` lifetime extension, and do not introduce a no-op
coordinator. Keep lock identity derived from the same workspace as the archive
root. **Plan correction from current call-graph inspection:** the existing
`execute_persisted_from_factory` path durably claims before factory creation;
the factory then creates the archive job with Job-scoped staging. There is no
safe insertion point for a production session in this path without moving the
claim and making the subsequent writer/staging/finalization lifecycle safe as one
change. Therefore this batch records the architecture and adds regression
coverage for the ordering constraint, but does not claim production session
wiring. Do not publish, finalize or clean staging under a partial session. Wire
the production lifecycle only together with B4/B5 attempt-owned staging,
confirmed writer/process-tree exit and fenced publication/finalization.

Before editing production lifecycle code, trace claim, executor dispatch, both
gallery-dl/aria2 writer lifetimes, staging creation, archive completion, failure,
cancellation and startup recovery. Current bounded C1-A work is design freeze and
targeted regression coverage for the required reservation-before-claim order and
the absence of a safe standalone production insertion point. Do not add a
decorative/unowned session or change production claim behavior in isolation.
Continue in dependency order: runner-owned admission/claim plus (1) B4 attempt
staging and confirmed writer/process-tree exit, (2) B5 session-fenced publication
and transactional DB/Telegram local finalization, (3) B6 shared replay/restart
crash matrix, and (4) B7 Desktop recovery integration. Preserve contention as
reschedulable, not terminal failure. Do not enable v2 recovery or D1 until all
required C1 gates pass.

Acceptance for this design checkpoint: targeted regression tests establish that
the current production entry claims before factory creation/runner execution, so
a factory-local coordinator cannot satisfy reservation-before-claim. Do not
claim contention/session production acceptance from Workflow unit tests alone.
When the combined B4/B5 production change is ready, acceptance must prove one
durable same-Job claimant under Job/destination reservation, unrelated-Job
admission, destination serialization, non-failing contention/claim loss,
attempt-owned staging, writer exit, stale-attempt publication denial and
transactional finalization. Record Linux evidence separately from Windows
lock/NTFS implementation and validation, which remain Windows-owned.

## 13. Progress

### 2026-10-12 `/goal` staging hardening and validation (historical checkpoint; source `7927452`)

- Preserved the existing uncommitted Plan, architecture, handoff, Windows-queue,
  and Storage changes. Added `FileStore::create_attempt_staging_dir` regression
  coverage for duplicate attempt refusal and symlinked job/staging parents. The
  staging-root test exposed that validating only the child path followed a
  symlinked staging root; production code now verifies the root with
  `symlink_metadata` before creating attempt content.
- Validation on Ubuntu x86_64 under WSL2, Rust/Cargo 1.98.0: Storage library
  122/122 PASS; Workflow all-targets PASS when run serially (see exact test
  summary in current handoff); Desktop library 289/289 PASS; formatter, docs
  audit, and `git diff --check` PASS. An earlier parallel Cargo run had five
  Workflow cross-process coordination failures while suites competed for shared
  test resources; the serialized rerun passed. This tooling/scheduling result is
  retained and is not classified as a product regression.
- C1 B3 production session ownership, B4 writer-exit integration, B5 fenced
  publication/finalization, B6 service crash/replay matrix and B7 Desktop
  recovery remain open shared work. v2 remains fail-closed; D1 remains gated.
  F2 implementation and F3 shared design/implementation also remain open. No
  Windows behavior is inferred from WSL; exact manual follow-up is in the
  Windows Validation Queue. Full workspace regression and Windows validation
  were not run.

### 2026-10-12 `/goal` continuation — source review and validation (current WIP)

- **Identity/ownership:** branch `cross-platform/automatic-pairing-reconcile-20261002`,
  starting HEAD `6567691d26f1f6b33f8153edcc95013bb79012d0`, equal to upstream at
  inspection. The pre-existing uncommitted edits to this Plan and
  `docs/status/platform-handoff.md` were retained. This WIP is owned by the
  Cross-platform Owner; it is not a commit, push, or Windows handoff.
- **Capability:** Ubuntu x86_64 under WSL2; Rust/Cargo 1.98.0, Node 26.7.0,
  Python 3.14.4. This session did not have a native Windows Desktop runtime,
  artifact bound to a formal handoff SHA, verified disposable NTFS fixture, or
  artifact-bound Windows evidence capture. No Windows check was run or inferred.
- **Source review:** traced `execute_persisted_from_factory`,
  `ProductionExecutionFactory`, `ArchiveExecutionJob`, both production download
  paths, Storage finalize/replay, and startup recovery. Durable claim still occurs
  before factory creation; production context uses Job-level staging; publication/
  finalization occurs inside the execution job; and the local runner reservation
  is released before post-run persistence. The Storage failpoint parameter is
  currently unused. These are shared lifecycle gaps, not Windows blockers. C1
  B3–B7 and v2 production remain open/fail-closed; D1 remains gated. F2/F3 remain
  shared implementation work, with concrete schema/privacy/merge semantics and
  strict TXT grammar still unresolved in §11. ZIP/7z have not been approved as an
  implementation dependency.
- **Implementation disposition:** no production code changed. A local extension
  of `RunnerReservation` cannot satisfy the required lifecycle: each scheduling
  thread owns a borrowed persistence context, while the runner factory independently
  creates execution resources. The implementation needs an owned generic runner
  scope that creates isolated persistence and coordinator resources, reserves Job
  and destination before atomic claim, and keeps the session through writer exit,
  publication, and finalization. Avoid unsafe lifetime extension or no-op platform
  adapters. The Plan's B1/B2 status text was corrected to distinguish local
  admission/claim wiring from this still-open contract.
- **Focused Linux evidence:** source review only; no production-target tests were
  run because no production code changed. Documentation checks for this updated
  batch are recorded after completion below.
- **Focused Linux evidence on the starting source:** Workflow coordination 11/11,
  execution-session integration 5/5, and no-replace integration 2/2 PASS;
  Storage attempt-preview selection 2/2 PASS; Desktop library 289/289 PASS.
  A Storage test filter for `archive_recovery_v2` selected zero tests and is not
  counted as coverage. These checks validate existing primitives, not production
  C1 lifecycle integration. The targeted commands were run serially after the
  earlier concurrent Workflow run showed five shared-resource failures; the
  serial coordination suite passed. Tool scheduling is not classified as a
  product defect.
- **Documentation delta:** preserve the requirement not to invent unresolved
  product contracts, correct B1/B2 implementation wording, and clarify the safe
  C1 runner-owned ownership boundary. Documentation audit and `git diff --check`
  are run for this continuation; no production test/build or Full regression is
  claimed for a documentation-only change.
- **Remaining non-Windows work:** close C1 B3–B7 with the reviewed reservation →
  atomic claim → attempt-owned writers/process exit → fenced no-replace publish →
  transactional finalization → restart/replay lifecycle and failure matrix;
  then enable D1 only after C1 passes. D1 uses independent JSON/YAML exports from
  one versioned DTO and task snapshot; preserve historical `export_text` semantics
  and existing TXT archives without reinterpretation or rewriting. Freeze F2/F3
  exact schemas, privacy/path merge policy, stable identities, transaction/admission boundary,
  and failure-injection cases before implementation. Perform applicable risk-based
  integrated Linux verification afterward. These shared items remain
  `IN_PROGRESS`/`PLANNED`, not Windows `BLOCKED` work.
- **Windows follow-up:** use the consolidated manual sequence in
  `docs/validation/windows-queue.md` after shared implementation is ready and a
  formal Git handoff provides an exact source SHA. Windows adapter/NTFS/native
  process, packaged-artifact, visual/DPI, and native GUI checks remain separately
  `NOT_RUN`; integrated C1/F2/F3 acceptance remains `BLOCKED`
  (`IMPLEMENTATION_NOT_READY`). No Windows PASS is claimed.
- **Regression scope:** full workspace regression was not run because no
  production code changed in this continuation. Run risk-based module/subsystem
  verification when the next production implementation batch changes those
  targets.

### 2026-10-11 cross-platform continuation (superseded starting state; no Windows handoff)

- Historical starting source: branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `035278e296797564d84c5dc8ff012568f556d2ff`. This checkpoint predates the
  committed persistence implementation and the current F1 continuation below.
- F1 implemented slice: strict Rust `ThemePreference` enum (`system`, `light`,
  `dark`), `get_theme_preference`/`save_theme_preference` Tauri commands backed by
  `settings_meta` key `ui.theme`, missing-value default to `system`, settings UI
  selector, renderer/wiring assertions and Rust parse/persistence tests. This
  does **not** apply the preference to document/CSS state or complete the shared
  all-page token audit. F1 remains `IN_PROGRESS`, not COMPLETED. The root
  stylesheet remains unchanged from HEAD. Windows native chrome/DPI checks remain
  queued and cannot establish the missing shared WebView behavior.
- Historical checkpoint only: C1 remained shared implementation work, not blocked
  as a whole by that host; the current call-graph disposition is in the dated
  Documentation Synchronization entry above. At that checkpoint:
  production does not inject `ExecutionSession`; attempt-specific staging/writer
  exit and publication/finalization are not joined into one fenced lifecycle;
  v2 recovery remains fail-closed and D1 exports remain inactive. The Windows
  `Coordinator`/NTFS no-replace adapter is nevertheless a Windows-owner
  implementation prerequisite to enabling this lifecycle on Windows. Do not
  use `cfg` omission or a no-op coordinator; retain supported legacy behavior.
- At that checkpoint F2/F3 remained shared `PLANNED` work. Before implementation,
  resolve/archive in
  a dedicated design review: permitted ZIP/7z parser dependencies and licenses;
  exact versioned config/record schemas and redaction inventory; record merge
  transaction/partial-conflict semantics; and active-task/restart interaction.
  The existing Plan §11 constraints remain binding; this is not a Windows-only
  deferral.
- Verification at that historical checkpoint: Desktop Rust `output_settings_tests`
  5/5 PASS; full Desktop Node suite 221/221 PASS; full Desktop Rust library suite
  288/288 PASS; Desktop all-target Clippy with `-D warnings`, `cargo fmt --all --
  --check`, `git diff --check`, Vite production build and docs audit PASS. One
  full Node run initially failed because sidebar CSS assertions were temporarily
  changed while exploring a token edit; the assertions were restored to match
  unchanged CSS and the complete suite then passed. Vite emitted its existing
  mixed static/dynamic Tauri API import warning. No full workspace regression,
  GUI/browser visual validation or dark-theme behavior is claimed.
- Windows status: `NOT_RUN` on Linux x86_64/WSL2; no NTFS target, fresh Windows
  artifact, formal handoff SHA or Windows GUI/runtime was available. WQ-PLAN-C1-
  P2-01 consolidates C1/NTFS/process/recovery/D1 steps; WQ-PLAN-C-F-01/02 remain
  unimplemented-feature follow-up only. Never record unexecuted cases as PASS.

### 2026-10-11 F1 pre-render theme and token continuation (historical; not handed off)

- Input identity: branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `4558670848c733e7605746e84ccba062ec04493b`; upstream
  `166837426366ebc9a84afeb306a49140d124bd35` (ahead 3). The three documentation
  edits from the preceding task were present and uncommitted before this code
  batch; no commit, push, or formal Windows handoff was created. Linux is the
  Cross-platform Owner; current working tree includes those edits plus this WIP.
- F1 shared implementation: `desktop/src/main.jsx` now reads the Rust-backed
  `get_theme_preference` before `createRoot`, applies the result to the document
  root, and bounds the read to five seconds. IPC rejection/timeout and unknown
  values fall back to `system`; a failure message is retained for Settings.
  Startup markers report `theme_preference_loading`/`theme_preference_loaded`
  while the existing 15-second non-React fallback remains active. The new pure
  `desktop/src/lib/theme.js` owns normalization/document application and has
  coverage for light, dark, system, invalid-value fallback and rejected reads.
- CSS audit covers the Desktop shell, shared cards (including inherited
  `.settings-panel` card styling), nav/hover, inputs, status surfaces, skeleton,
  logs, and startup fallback. Semantic tokens now have explicit light/dark
  palettes; the log viewport has its own themed surface token. Intentional
  brand/status glyph colors remain fixed. Extension popup/options are separate
  documents and remain outside Desktop preference scope. This is a source/token
  audit, not visual acceptance; no broad hardcoded-color rejection is used.
- Checks on this WIP: initial `node --test desktop/test/ui-wiring.test.mjs`
  against current HEAD passed 52/52. Final targeted theme, UI wiring and
  startup-contract tests pass 60/60; full `npm test` in `desktop` passes 226/226.
  `npm run check` (Vite production build), CSS parsing, `git diff --check`,
  `node scripts/docs-audit.mjs`, and docs audit tests (16/16) pass. Vite retains
  its existing mixed static/dynamic Tauri API import warning; the transient CSS
  parser warning found during review was traced to an accidentally truncated log
  rule, restored, and is absent from the final build. Intermediate test attempts
  also exposed stale CSS assertions, including a nonexistent standalone
  `.settings-panel` rule; assertions now verify `.ui-card` inheritance and actual
  shared selectors. No Windows GUI/WebView2/DPI validation was run.
- Reassessment: C1.4 remains incomplete because production v2 startup recovery,
  transactional finalization, COMMITTED filesystem+DB verification and restart
  crash matrix have not all passed; keep C1 production fail-closed. D1 remains
  gated on C1 acceptance. F2 remains `PLANNED` with its maintenance-mode,
  wait-and-apply decision confirmed; its schema/dependency implementation work is
  open. F3 remains `PLANNED` pending its §11 schema/dependency/license, merge and
  active-task/revision decisions. Integrated G remains open until every gated batch
  and cross-platform integration acceptance is complete. No new work in these
  areas meets its gate in this continuation.
- Windows acceptance remains `NOT_RUN`: no fresh Windows artifact or formal
  handoff SHA. Source build and Node tests do not establish WebView2 appearance,
  native chrome/DPI behavior or OS system-theme following.

### 2026-10-12 cross-session goal reconciliation (historical checkpoint; superseded by current state above)

- User goal: complete all Plan development and tests that do not depend on Windows,
  then consolidate Windows-only validation and provide manual steps for checks
  unavailable in this Linux/WSL2 environment. This goal is not yet complete.
- Current source identity at inspection: branch
`cross-platform/automatic-pairing-reconcile-20261002`, HEAD
`344322ba7cd1a75588f5def746eb061451114c87`, equal to upstream before the later
  Storage and documentation commits. No production files were changed in that
  continuation; Windows runtime, NTFS target and fresh Windows artifact were absent.
- F1's current shared source includes preference persistence, pre-render theme
  application, bounded IPC fallback, and light/dark token coverage. This corrects
  older historical F1 status rows; Windows
  WebView2 appearance, OS preference following, native chrome/dialog and DPI remain
  Windows-owner checks.
- At that historical checkpoint the shared goal remained open: C1 B3–B7 production
  lifecycle/recovery and D1 stayed fail-closed/gated; F2 remained unimplemented,
  and F3's shared schema/dependency/transaction decisions remained open. These
  were not Windows blockers. Do not use this checkpoint as current status.
- Next shared implementation remains C1 in dependency order: runner-owned
  reservation-before-claim combined with attempt-isolated staging and proven
  writer/process-tree exit; attempt-fenced file publication and transactional
  DB/Telegram finalization; restart/replay crash matrix; then Desktop recovery
  integration. Only after these gates pass may v2/D1 be enabled. F2 is `PLANNED`
  with maintenance-mode application decision confirmed; F3 remains `PLANNED`
  pending its remaining §11 schema/dependency/transaction gates.
- Windows-only checks remain `NOT_RUN` pending a formal Git handoff and fresh
  artifact. Use the active WQ-PLAN-C1-P2-01 and WQ-PLAN-C-F-01/02 procedures;
  their preconditions, manual steps and evidence requirements are listed in the
  Windows queue. No formal handoff is made by this reconciliation.

### 2026-10-13 non-Windows development continuation (superseded source-review checkpoint)

- Starting revision: `6567691d26f1f6b33f8153edcc95013bb79012d0`, equal to upstream. The three pre-existing uncommitted documentation edits were retained. Current owner: Cross-platform Owner; no commit or Windows handoff.
- Reconciled historical status with current call graph: C1 B3–B7 production lifecycle is still not safely integrated; claim precedes factory execution, production uses Job-level staging, archive publication/finalization occurs inside the execution object, and runner admission is released before post-run persistence. Storage v2 recovery remains fail-closed and its service failpoint is not wired. These shared prerequisites keep D1 gated; they are not Windows blockers.
- F2/F3 behavior decisions are approved, including F3's cross-process maintenance admission, drain through finalization, cancellable wait, revision/evidence recheck, and one atomic accepted-set transaction. Concrete schemas, field/privacy rules, JSON/YAML canonical mapping, path evidence rules, merge identities and failure cases remain implementation gates. Per the approved C1 → D1 → F2 → F3 → G order, F2/F3 implementation does not leapfrog C1/D1. ZIP/7z remain deferred.
- Linux capability: Ubuntu x86_64 under WSL2; Cargo/Rust and Node available. `cargo test -p xarchive-workflow --features __lock_probe --all-targets -- --test-threads=1` PASS (22 unit, 11 coordination, 5 execution-session, 2 no-replace); `cargo test -p xarchive-storage --lib` PASS (124); `cargo test -p xarchive-desktop --lib` PASS (289). These verify existing shared primitives and modules, not integrated C1/D1/F2/F3 acceptance.
- Documentation checks: `node scripts/docs-audit.mjs` PASS; `git diff --check` PASS. No production code changed, so no production build or Full workspace regression was run. Windows-native adapter, NTFS, process, packaged artifact and GUI validation were not run; see the consolidated manual queue in `docs/validation/windows-queue.md`.

### 2026-10-13 `/goal` continuation result

- Rechecked the current call graph before implementation. Durable claim still
  occurs on the scheduling caller before `ProductionExecutionFactory::create`;
  the runner receives only a snapshot and constructs the execution object itself.
  `JobPersistence` is borrowed and cannot be sent through the runner's `'static`
  queue, while `ExecutionSession` borrows a concrete coordinator. Production
  download still uses Job-scoped staging, archive publication/finalization occurs
  inside the execution object, and the local runner reservation is released before
  post-run state/event persistence. Storage's recovery failpoint argument remains
  unused.
- No production code was changed: extending the local reservation lifetime alone
  would not establish atomic claim-under-cross-process-guard, attempt-owned writer
  lifecycle, confirmed child exit, or fenced publication/finalization. Production
  v2 remains fail-closed; D1 remains gated. F2/F3 remain after D1 in the approved
  order; unresolved schema/privacy/identity/evidence contracts were not guessed.
  ZIP/7z remain deferred pending a separately approved dependency/security/license
  decision.
- Linux documentation validation: `node scripts/docs-audit.mjs` PASS and
  `git diff --check` PASS. No production tests/build or Full regression was run
  because no production code changed. Previously recorded Workflow/Storage/Desktop
  module tests remain primitive/module evidence, not integrated C1 acceptance.
- Windows adapter/NTFS/native-process, fresh-artifact, and GUI checks remain
  `NOT_RUN` pending formal Git handoff and artifact prerequisites. Integrated
  C1/D1/F2/F3 are gated by shared implementation readiness (`IMPLEMENTATION_NOT_READY`),
  not blocked by the Windows environment; manual follow-up is consolidated in
  `docs/validation/windows-queue.md`.

### 2026-10-12 Documentation Synchronization — historical pre-implementation checkpoint

- **Implementation: `PLANNED` for production C1-A; documentation: `COMPLETED`;
  documentation verification: `VERIFIED`.** Refreshed
  the current C1-A design boundary and F2 application decision in this Plan,
  architecture overview, current platform handoff, and Windows validation queue.
- C1-A architecture remains runner-owned reservation-before-durable-claim with an
  owned persistence context and a local concrete-generic session. Call-graph
  inspection found no safe standalone production insertion point: the current path
  claims before factory creation and then uses Job-scoped staging. No production
  C1-A code or ordering regression test was added because the existing contracts
  provide no test seam that can prove this property without an API/production-flow
  change; creating a hook solely for that test would be misleading. Production
  integration remains gated on a combined safe B4/B5 design. No feature completion
  is claimed.
- F2 wait-and-apply maintenance-mode behavior is a confirmed design decision;
  F2 implementation remains `PLANNED`. F3 remains `PLANNED` with its remaining
  schema/dependency/transaction decisions open.
- Fresh validation at that historical checkpoint: `cargo fmt --all -- --check`
  PASS; `cargo test -p xarchive-workflow --features __lock_probe --all-targets`
  PASS (22 unit + 11 coordination + 5 execution-session + 2 no-replace);
  `cargo test -p xarchive-storage --lib` PASS (119); and
  `cargo test -p xarchive-desktop --lib` PASS (289). `node
  scripts/docs-audit.mjs` and `git diff --check` PASS. These establish module/helper
  behavior only; none proves Desktop production session wiring or integrated C1.
  Windows native/NTFS/runtime checks remain `NOT_RUN` pending formal Git handoff
  and the Windows Owner's adapter/artifact prerequisites.
- The `/goal` is not complete: C1 production wiring/B4–B7 and D1 remain gated;
  F2 implementation remains `PLANNED`, and F3's shared design gates remain open.
  Windows-only cases are consolidated in the current Windows queue with explicit
  implementation prerequisites and manual execution steps.

### 2026-10-11 P2 runner-owned wiring investigation

- Audited the uncommitted P2 experiment. Its incomplete Workflow dependency,
  config/import scaffolding and coordination error mapping were removed; no
  production coordinator/session wiring is retained.
- Confirmed the canonical destination from existing archive and recovery
  contracts is the workspace-relative `Tweets/<tweet_id>`; archive root comes
  from the configured `FileStore`/runtime archive directory. No user-selected
  per-job destination exists in the persisted execution spec. A future lock-root
  setting must be derived from the same workspace identity as the archive root,
  not independently guessed from the database filename.
- P2 cannot safely close in isolation: production currently claims before
  factory creation and runner dispatch; the writer uses Job-level staging;
  `complete_sidecar_archive` and post-run Job completion are not attempt-fenced;
  startup v2 recovery is intentionally fail-closed. Simply acquiring a session
  in the current factory would be after the durable claim and would release its
  guard before post-run finalization. Therefore no coordinator is enabled and no
  legacy download path is changed. Continue shared B3–B5 work before production
  session construction; this is `IN_PROGRESS`, not blocked by Windows.
- Final validation after cleanup: Workflow tests 22 + 11 + 5 + 2 PASS;
  Storage library tests 119/119 PASS; Desktop library tests 286/286 PASS;
  workspace Clippy `-D warnings`, `cargo fmt --all -- --check`, `git diff
  --check`, and docs audit PASS. Earlier checks on the incomplete draft reported
  unused imports, a missing `SessionError` reference, and rustfmt import ordering;
  those draft-only errors were removed and the final checks above passed. No full
  workspace suite, Windows target runtime, NTFS, or GUI
  validation was run. Full regression was not run because this batch has not
  changed lifecycle behavior and targeted suites cover the inspected modules.
- Windows follow-up is `NOT_RUN` on this Linux/WSL2 host and remains with the
  Windows Platform Owner after a formal handoff and fresh artifact. See the
  Plan coordination entry in `docs/validation/windows-queue.md`; it is not a
  blocker on remaining shared B3–B7 work.

### 2026-10-11 P1 two-phase execution authorization

- Corrected the shared-state record first: the handoff file still described the
  lint and B3 batches as uncommitted and still stated that no `ExecutionSession`
  helper existed, all of which contradicted the tree. Those statements are now
  marked as their own historical checkpoints against a single current-state
  section pinned to `ebeaaf8…`.
- Reclassified the Windows adapter. It blocks enabling the coordinated lifecycle
  on Windows and NTFS acceptance; it does not block shared implementation or
  Linux verification, so no `WINDOWS_BLOCKING` is declared for B3–B7. The Plan
  header now records the three things shared work must not do instead: omit the
  session behind a platform `cfg`, substitute a no-op coordinator, or make the
  existing supported download path fail on Windows.
- Added `ExecutionReservation` in `xarchive-workflow` and removed
  `ExecutionSession::begin`. The old constructor accepted an `AttemptIdentity`
  the caller had already produced, which permitted exactly the claim-then-lock
  ordering this contract exists to forbid: a claim made outside the lock
  increments the attempt and only then discovers a peer owns the job. The
  reservation takes both scopes first, then `claim` runs the caller's durable
  claim under that guard and binds the session to whatever attempt the claim
  actually returned. `SessionError` gained `ClaimLost` and `ClaimRejected` plus
  `is_reschedulable`, and the session now binds its final destination.
- Verification on the current tree: `cargo fmt --all -- --check` PASS;
  `cargo test -p xarchive-workflow --features __lock_probe` 22 lib + 11
  coordination + 5 execution-session + 2 no-replace PASS;
  `cargo clippy -p xarchive-workflow --features __lock_probe --all-targets`
  clean; `cargo check -p xarchive-workflow --target x86_64-pc-windows-msvc
  --all-targets` clean both with and without `__lock_probe`;
  `cargo test --workspace --no-fail-fast` 26/26 binaries ok, 0 failed.
- Mutation evidence: dropping the destination scope and accepting a foreign-job
  identity each fail the suite; a third mutation that inverted the ordering was
  rejected by the borrow checker rather than by a test.
- This closes P1 only. P2 runner-owned wiring, B4–B7, v2 startup recovery and D1
  remain open; v2 stays fail-closed. No production path constructs a session
  yet, and no Windows/NTFS validation or formal handoff is claimed.

### 2026-10-11 release clippy gate cleared

- Cleared every workspace clippy warning, which is what gates the release job:
  `.github/workflows/pre-release.yml` runs
  `cargo clippy --workspace --all-targets -- -D warnings`, so any warning fails
  the pipeline. Before this batch there were 4 in `xarchive-desktop` and 6 in
  `xarchive-storage`; that command now exits 0.
- Fixed properly rather than silenced, because each was a real defect:
  - `desktop/src-tauri/src/transport.rs`: removed `let use_aria2 = use_aria2;`,
    a redundant self-binding that shadowed the parameter.
  - `desktop/src-tauri/src/websocket_transport.rs`: removed a
    `.map(|frame| frame.into())` that converted a `tungstenite` `CloseFrame` to
    its own type, so the RFC 6455 close reply now passes the frame through
    unchanged.
  - `crates/xarchive-storage/src/archive_recovery.rs`: converted two
    `assert_eq!(..., true/false)` to `assert!`/`assert!(!...)` and gave each a
    message naming the invariant, so a failure says which claim outcome broke
    instead of printing a bare `left: true, right: false`.
- The remaining four warnings were `too_many_arguments`. They are **not**
  fixed, because the fix is to bundle parameters into a struct, which changes
  shared Storage and Desktop signatures that are part of the persistence and
  transport contracts. That is shared-API work requiring its own reviewed
  batch, so each site carries a scoped
  `#[allow(clippy::too_many_arguments)]` with a comment stating why, rather
  than an unrelated refactor hidden inside a lint cleanup.
- Verification: `cargo clippy --workspace --all-targets -- -D warnings` exit 0;
  `cargo fmt --all -- --check` PASS; `cargo test --workspace --no-fail-fast`
  26/26 binaries ok, 0 failed; Storage claim tests 10/10 and Desktop WebSocket
  tests 13/13 re-run after the edits.
- This closes only the lint gate. It is not Plan progress: B3 production
  wiring, B4–B7, v2 startup recovery and D1 remain open and v2 stays
  fail-closed.

### 2026-10-11 B3 shared execution-session contract

- Added `xarchive-workflow::execution::ExecutionSession`, the shared
  runner-owned authorization token for one attempt. It holds the claimed
  `AttemptIdentity` and a coordinator guard acquired over the Job *and*
  destination scopes together, so the adapter's canonical ordering removes
  deadlock instead of trusting caller order.
- It is generic over the `Coordinator` port and must stay that way. This
  closes the risk the previous batch flagged: a session gated on the Unix
  adapter would be compiled out of existence on Windows and silently turn the
  shared lifecycle into an unsynchronised path there. Verified by
  `cargo check -p xarchive-workflow --target x86_64-pc-windows-msvc
  --all-targets` and the same command with `--features __lock_probe`, both
  clean. The probe binary was found ungated and would have failed that second
  command, so each of its items is now individually `cfg(unix)` with a refusing
  `main` stub for other targets.
- `SessionError` separates `Contended` from `InvalidScope`, `Io` and
  `StaleAttempt`. That separation is the point: contention must reach the
  caller as a reschedulable outcome and must never be persisted as a download
  failure, which is the defect B1 fixed for runner saturation and which would
  otherwise reappear at the coordination layer.
- The guard is owned by the session and exposed only as `&C::Guard`, so
  `CoordinationGuard::release` cannot be called through the accessor: a caller
  cannot drop exclusivity under a live session or double-release it.
- Proven against a real second OS process via a new `session` mode on
  `xarchive-lock-probe`, which prints what it actually holds so the test asserts
  the peer established the exact session requested: a live peer session makes
  the same job contended and releases with the process; a peer on another job
  does not block this one; two different jobs sharing one destination
  serialise; a crashed peer does not block a successor; a stale attempt is
  refused. Two mutation checks (dropping the destination scope, removing
  attempt fencing) each fail the suite, so the tests are not vacuous.
- Verification on the current tree: `cargo fmt --all -- --check` PASS;
  `cargo test -p xarchive-workflow --features __lock_probe` 16 lib +
  11 coordination + 5 execution-session + 2 no-replace PASS;
  `cargo test --workspace --no-fail-fast` 0 failed;
  `cargo clippy -p xarchive-workflow --features __lock_probe --all-targets`
  clean; Windows `x86_64-pc-windows-msvc` cross-check clean.
- Not done: no production path constructs a session yet, so nothing holds a
  Job/destination guard across claim, writer shutdown, publication or DB
  finalization. That wiring needs the destination resolved before the claim and
  remains shared `CROSS_PLATFORM_CHANGE_REQUIRED` work with B4/B5. B4–B7, v2
  startup recovery and D1 stay open; v2 stays fail-closed. No Windows/NTFS
  validation or formal Windows handoff was performed, and NTFS session
  behaviour must not be inferred from the Unix `flock` evidence above.

### 2026-10-11 documentation gate reconciliation

- Historical checkpoint note: this gate reconciliation predates the later
  committed shared two-phase `ExecutionSession` contract and must not be read as
  the current Workflow state. In the current P2 worktree, only incomplete Desktop
  error-mapping edits were removed; the committed Workflow contract remains.
- Removed undocumented `ExecutionSession` residue that contradicted the committed
  gate record: untracked `crates/xarchive-workflow/src/execution.rs`,
  `pub mod execution;` plus a `coordination`/`execution` re-export block in
  `crates/xarchive-workflow/src/lib.rs`, a new `xarchive-workflow` dependency in
  `desktop/src-tauri/Cargo.toml` and `Cargo.lock`, and an unused
  `xarchive_workflow::{AttemptIdentity, Coordinator, ExecutionSession}` import in
  `desktop/src-tauri/src/executor.rs` that would have failed the release clippy
  gate. Tracked files were restored with `git checkout --`; the untracked file was
  deleted. No other production file changed.
- B3 stays `PLANNED` with no code retained. The session must arrive with real
  Job/destination authorization across claim, writer shutdown, publication and DB
  finalization as a shared `CROSS_PLATFORM_CHANGE_REQUIRED` contract; a
  `Noop`-coordinator type proves no lifecycle behavior and must not be mistaken
  for ownership.
- Gate re-verified on the cleaned tree: `cargo fmt --all -- --check`,
  `git diff --check`, `node scripts/docs-audit.mjs` PASS;
  `cargo test -p xarchive-workflow --lib` 12/12, `--test coordination
  -- --test-threads=1` 11/11, `--test no_replace` 2/2;
  `cargo test -p xarchive-desktop --lib` 286/286; Storage attempt-scoped 14/14 and
  staging 6/6; `cargo clippy -p xarchive-workflow --all-targets` clean.
- Pre-existing and out of scope: `cargo clippy -p xarchive-desktop --lib` reports
  4 warnings at HEAD (`useless_conversion` and similar). They gate the release
  clippy job and need a dedicated small change; unrelated refactoring is
  prohibited in this batch.
- B3–B7, v2 startup recovery and D1 remain open; v2 stays fail-closed. No
  Windows/NTFS validation or formal Windows handoff was performed.

### 2026-10-11 N2 atomic state-plus-attempt claim

- Rewired both production attempt paths (`execute_persisted_from_factory`,
  `execute_persisted_on_worker`) to acquire the initial running transition through
  the atomic `JobPersistence::claim_attempt` (backed by
  `Database::claim_archive_attempt`) instead of a separate `begin_attempt`
  followed by an unfenced first state write. The claim binds expected
  state+attempt, increments with checked arithmetic, and writes the initial state
  plus its event in one transaction; a losing claimant returns the winner's
  snapshot with no side effects. A Job already in its running state (resume) still
  obtains an attempt identity via `begin_attempt` because there is no transition
  to claim.
- This is behavior-preserving: the final state/event sequence is identical, which
  the unchanged 286-test Desktop suite confirms. PREPARED/COMMITTED routing to
  recovery remains fail-closed.
- Verification at that historical checkpoint: `cargo fmt --all` clean;
  `cargo check -p xarchive-desktop --lib` PASS; `cargo test -p xarchive-desktop
  --lib` 286/286 PASS. This closes the B2 production claim wiring. Typed
  loss/contention outcomes surfaced to callers and cross-process admission remain
  with B3+; the B3–B7 session, staging/writer fencing, finalize, replay and crash
  matrix remain open.

### 2026-10-11 N1 admission and backpressure

- Fixed the confirmed `CROSS_PLATFORM_CHANGE_REQUIRED` defect where a saturated
  single-active runner turned an accepted Job into a persisted `FAILED`
  (`EXECUTOR_WORKER_FAILED: queue is full`). `JobExecutorHandle` now carries a
  runner reservation counter mirroring the runner channel capacity;
  `execute_persisted_from_factory` and `execute_persisted_on_worker` reserve a
  slot *before* `begin_attempt`, so a `QueueFull` rejection no longer increments
  an attempt or writes `FAILED` and the Job stays queued/reschedulable.
- Added regression `saturated_runner_rejection_is_non_failing_and_reschedulable`
  (blocking factory execution saturates the runner; a second Job is rejected
  non-destructively then runs once the slot frees).
- Verification at that historical checkpoint: `cargo fmt --all -- --check`
  PASS; `cargo test -p xarchive-desktop --lib` 286/286 PASS; focused Storage
  claim tests 2/2 PASS. This closes the B1 runner-admission core only. B2 typed
  claim-loss/contention outcomes, B3 session ownership, and the B4–B7 lifecycle,
  staging, replay/finalize and crash matrix remain open; v2 stays fail-closed.

### 2026-10-11 N0 baseline repair

- Repaired `JobPersistence` so attempt lifecycle methods are declared on the
  trait and implemented by the in-memory, storage-backed, and commit-boundary
  test adapters. Removed accidentally nested method declarations from
  `StorageJobPersistence::attempt_is_current`.
- Corrected the PREPARED claim fixture to use the durable manifest's attempt
  identity (`attempt_count = 1`) when asserting that a fresh claim is rejected.
- The uncommitted source at that historical checkpoint passed `cargo fmt --all -- --check`,
  `cargo check -p xarchive-desktop --lib`, the two focused Storage claim tests,
  the Desktop stale-attempt fencing test, `git diff --check`, and
  `node scripts/docs-audit.mjs`.
- This verifies only the N0 compile/fixture slice. Saturated-runner rescheduling,
  production use of atomic claim, execution-session/writer fencing, replay and
  finalization remain open; v2 remains fail-closed and D1 remains gated.

### 2026-10-11 non-Windows continuation closeout

- This session started from `bf050ccc5559d1b791bfe335413373fd4ad8b078` on branch
  `cross-platform/automatic-pairing-reconcile-20261002`. E3 was committed as
  `d846ef9`; N1/N2 admission and atomic claim work was committed separately as
  `6fc79a2`. The documentation synchronization is the current third batch; none
  of these commits constitutes a formal Windows handoff.
- The current request did not close every non-Windows Plan item. C1 production
  wiring remains incomplete: production writer fencing/coordinator ownership,
  service replay/finalization wiring and the process-restart matrix are open. D1
  remains gated on C1. F1 shared implementation is present; F2 remains planned.
  F3's scope is specified in §11 and
  its implementation is not started. These are shared implementation gaps, not
  Windows blockers.
- B1 runner admission/backpressure and B2 atomic attempt/state claim are
  implemented and committed in `6fc79a2`. Queue saturation leaves the Job
  queued with no attempt increment; atomic claim binds expected state+attempt and
  first running state. B3–B7 lifecycle/recovery integration remains incomplete.
- E3's Linux actual-spawn acceptance is verified and committed in `d846ef9`:
  `cargo test -p xarchive-download --lib -- --test-threads=1` 50/50 and Sidecar
  downloader/protocol tests 30/30. `cargo fmt --all -- --check`, `git diff --check`
  and `node scripts/docs-audit.mjs` pass. These do not constitute integrated G or
  Windows validation.
- Windows-specific and artifact-bound checks are skipped in this Linux/WSL
  session, remain `NOT_RUN` (and `BLOCKED` only where the queue records an
  implementation prerequisite), and have manual steps in the Windows queue.
- Current source includes E3 `d846ef9` and N1/N2 `6fc79a2`; this documentation
  batch will record its own committed revision. No formal Windows handoff is made.

| Batch | Implementation | Verification / next action |
|---|---|---|
| A — page split | Implemented at `d406099` | Linux evidence recorded at `57edd18`; Windows fresh-artifact GUI pending |
| B1 — v1 compatibility / version dispatch | Implemented | v1 compatibility, v2 envelope, unknown-version and identity checks have targeted coverage; full module tests pass |
| B2 — v2 output snapshot | Implemented; Linux verification passed, Windows pending | Typed settings read/save commands and Download Config UI; one-off and batch submissions capture snapshots. Verify active reuse, retry/recovery and restart against immutable snapshots on Windows. |
| D0 — InternalV1 recovery | IMPLEMENTED; Linux safety acceptance complete | Existing InternalV1 recovery behavior and recorded crash-boundary coverage remain in place. New rename-capable v2 work is separate and incomplete; do not treat D0 evidence as proof of v2 rename recovery. Windows NTFS interruption evidence remains `NOT_RUN` (`WQ-PLAN-C-D0-01`). |
| C1.1 — durable journal | COMPLETED; Linux Storage module tests PASS on `b9ac571` | `transition_archive_recovery_v2` reads the immutable plan from `manifest_json` and advances only `rename_progress`/`phase`; creation retains migration 0017's default. This phase-machine repair is unit-tested, not production integration-verified. |
| C1.2 — workflow coordination | Linux adapter implemented and targeted-verified; production wiring open | `cargo test -p xarchive-workflow --test coordination` 11/11 PASS, covering same-process/coordinator contention, guard lifecycle, partial-group rollback, process crash, fork/exec residue, parallel Jobs and destination scope. Job/destination acquisition is not wired into SQLite/Desktop production scheduling. Windows adapter and NTFS validation remain Windows-owned. |
| C1.3 — attempt staging and rename replay | IMPLEMENTED (shared helpers); Windows pending | Attempt-scoped staging and size/hash-checked replay helpers PASS targeted Linux tests; platform no-replace mover is Linux-only and non-Unix fails closed. Path helpers are not wired to the production lifecycle; durable fencing remains incomplete. |
- Existing current-tree Linux regression results: Storage 117/117 and Desktop 285/285 PASS (not rerun in the 2026-10-11 E3 continuation). A transactional Storage finalization primitive, cancellation/PREPARED arbitration and attempt-fenced state writes have unit/regression coverage, but are not connected to the complete archive service/startup workflow. Lifecycle Job/destination locks, attempt-owned file-write fencing, PREPARED rename replay/no-replace directory commit, transactional DB/Telegram finalization wiring, read-only COMMITTED filesystem+DB verification, operation-boundary failpoints, service process-restart matrix, and end-to-end media-index contract evidence remain open. v2 stays fail-closed; D1 remains gated. Windows NTFS validation is additional, not a substitute for these shared gates. |
| D1 — metadata outputs | BLOCKED pending C1 shared gate | Historical output snapshots retain their stored meaning; the new D1 contract is JSON/YAML and is not yet operative. Add explicit version dispatch rather than reinterpret `export_text`. Do not make recovery depend on user exports. Acceptance matrix: JSON/YAML on/on, on/off, off/on, off/off; legacy TXT compatibility; semantic equivalence; and exports-off recovery. |
| E — tool arguments | Shared E3 implementation and Linux acceptance verified; Windows acceptance pending | E0 policy/protected-option handling, E1 v3 decoding/snapshots, E2 argv wiring and E3 settings/persistence/batch/tool-compatibility/redaction are implemented. Linux actual-spawn acceptance: `cargo test -p xarchive-download --lib` 50/50, including readiness-driven `Aria2Supervisor::spawn` argv order, final `--conf-path`, secret absence and cleanup; Sidecar downloader/protocol tests 30/30, including actual `DownloadRunner` spawn argv boundaries. Formatting, diff-check and docs audit PASS. gallery-dl user arguments remain rejected by the empty allowlist. Windows Named Pipe/native process/packaged-artifact checks remain `BLOCKED` or `NOT_RUN` pending formal handoff and fresh artifact. This closes only shared E3, not C1/D/F or the overall Plan. No execution-spec export API exists; do not claim export-redaction support. |
| F1 — theme authority/token audit | Shared implementation implemented; Windows visual/native verification pending | `ui.theme` typed `system | light | dark` authority, SQLite settings commands, pre-render WebView preference application with bounded system fallback, and light/dark Desktop tokens are implemented and covered by Node/Rust tests and Vite build (see 2026-10-11 evidence above). Windows WebView2 appearance, OS preference changes, native titlebar/dialog and DPI are separate Windows-owner checks. |
| F2 — versioned local configuration exchange | PLANNED; maintenance-mode apply decision confirmed | No shared implementation started. Before coding, freeze the versioned JSON schema/compatibility and unknown-field policy, exact export allowlist/secret exclusion, target-path validation and preserve-local mismatch semantics, revision identity, and crash-safe cross-process maintenance ownership/release. Then implement preview/import/export with atomic persistence and runtime update ordering, and test cancel/drain/revision/failure boundaries. |
| F3 — download/archive record exchange | PLANNED; behavior decisions confirmed, shared contract open | No shared implementation started. Before coding, freeze download/archive record schemas and redaction, identities and idempotency keys, archive-root/path/hash evidence requirements, partial acceptance transaction boundary and active-task admission race. Review and select a maintained ZIP/7z dependency and license before container work; implement standalone files first, then containers through the same validator. |
| G — integrated Linux verification | IN_PROGRESS | Targeted E3/F1 and prior C1 helper suites have recorded results at their exact historical revisions; these do not constitute integrated Plan acceptance. After all shared gates close, run the risk-based module/subsystem/restart matrix, full regression as justified, final diff/docs audit, then record exact source SHA for formal Git handoff. |
| H — Windows implementation/validation | NOT_STARTED | Windows Owner work requires formal Git handoff and fresh artifact. Native no-replace/coordination, NTFS, GUI/DPI and packaged-process checks remain manual queue items. |

### 2026-10-08 Linux continuation evidence

Source baseline: branch `cross-platform/automatic-pairing-reconcile-20261002`,
commit `8b2b544206e62e1356ef582e46d62b92be14cb04`; initial worktree clean. Re-ran
`cargo test -p xarchive-storage --lib` (106/106),
`cargo test -p xarchive-desktop --lib` (269/269), and
`cargo test -p xarchive-download --lib` (35/35): PASS. Later in this continuation,
`cargo test --workspace --no-fail-fast` passed. Strict workspace Clippy failed
on existing `too_many_arguments` lints in `database/jobs.rs` and
`archive.rs::execute_archive_context`, plus `useless_conversion` in
`websocket_transport.rs`; no production code was changed to suppress them.
Formatting, diff checks and docs audit passed. Sidecar 63/63, Desktop Node
220/220, Extension Node 52/52 and Desktop
Vite build passed. Windows-native validation was not performed. Production v2
remains fail-closed. Current final verification is recorded in
`docs/status/platform-handoff.md`.

### 2026-10-09 Linux continuation evidence

Source commit: `b9ac571448c9e093c1639ca0586718dc9dd7bcc4`; current owner remains
Cross-platform Owner; the worktree was clean immediately after the commit. The
prior uncommitted C1.1 repair was committed only after current-tree validation:
Storage 106/106, Desktop Rust library 269/269, Download 35/35, Storage build,
format check, diff check and docs audit all PASS. Full Rust workspace, Sidecar,
Desktop/Extension Node and Windows validation were not run for this source. C1.2,
C1.3, C1.4 and production E were not implemented in this continuation. Windows
items remain NOT_RUN/BLOCKED and must bind to a later formal handoff and fresh
artifact. The strict workspace Clippy failure recorded in the 2026-10-09 handoff
is pre-existing and was not re-run here.

### 2026-10-07 Linux continuation evidence (historical; not current validation)

At that checkpoint, the worktree was uncommitted on branch
`cross-platform/automatic-pairing-reconcile-20261002`, based on
`773fe64a9f8fe813a489fc4ddb675b32e0b9ddca`. The recorded Linux results (Storage
104/104, Desktop Rust 268/268, Download 35/35, Sidecar pytest 63/63, Desktop Node
220/220, Extension Node 52/52, formatting, diff check and docs audit) apply only to
that checkpoint and are not validation of the current dirty tree. This incremental
change added a strict v2 source/final disjointness invariant and regression test; it
does not implement the v2 production pipeline. Full workspace regression was not
run. C1/D1 recovery/finalization is still incomplete and v2 remains fail-closed.
Windows native validation is unavailable in this WSL2 session; see the consolidated
Windows queue/manual steps.

### C1 continuation attempt — 2026-10-07 (historical checkpoint)

The requested Linux-only continuation did not complete the remaining shared gate.
At that checkpoint, the working tree contained v2 finalization and startup-dispatch
prototypes, but service replay, connected failpoints, and COMMITTED final-directory
verification were absent. Production v2 therefore remained fail-closed; C1 was
`IN_PROGRESS`, D1 remained gated, and E/F remained shared implementation work rather
than Windows-only blockers. Existing targeted results (Storage 106/106, Desktop
Rust 269/269, formatting, diff check and docs audit) applied to that checkpoint,
not validation of a completed pipeline or the current dirty tree. The current dirty
tree now contains startup dispatch and file-operation prototypes, but production v2
remains fail-closed and current-tree validation is pending. The earlier attempted
implementation did not compile and was reverted at that time; no completed product
pipeline was claimed. Full workspace regression was not run. Windows-only procedures are consolidated in
`docs/validation/windows-manual-steps.md` and remain separate from these shared
implementation gaps.

### 2026-10-13 `/goal` continuation result

- The current call graph was rechecked before implementation. The durable claim
  still occurs on the scheduling caller before `ProductionExecutionFactory::create`;
  the runner receives only a snapshot and constructs the execution object itself.
  `JobPersistence` is borrowed and cannot be sent through the runner's `'static`
  queue, while `ExecutionSession` borrows a concrete coordinator. Production
  download still uses Job-scoped staging, and archive publication/finalization
  occurs inside the execution object. The local runner reservation is released
  immediately after the runner interaction, before post-run state/event writes and
  final persistence. Storage's recovery failpoint argument remains unused.
- No production code was changed: changing only the local reservation lifetime
  would not establish the required atomic claim-under-cross-process-guard,
  attempt-owned writer lifecycle, confirmed child exit, or fenced publication and
  finalization. Production v2 remains fail-closed; D1 remains gated. F2/F3 remain
  after D1 in the approved order, and unresolved schema/privacy/identity/evidence
  contracts were not guessed. ZIP/7z remain deferred pending a separately approved
  dependency/security/license decision.
- Linux documentation validation: `node scripts/docs-audit.mjs` PASS and
  `git diff --check` PASS. No production tests/build or Full regression was run
  because no production code changed. Existing recorded Workflow/Storage/Desktop
  module tests remain primitive/module evidence, not integrated C1 acceptance.
- Windows-only manual follow-up is consolidated in
  `docs/validation/windows-queue.md`. Windows adapter/NTFS/native-process, fresh
  artifact and GUI checks remain `NOT_RUN` pending formal Git handoff and artifact
  prerequisites. Integrated C1/D1/F2/F3 are `BLOCKED`
  (`IMPLEMENTATION_NOT_READY`), not Windows-environment blockers.

### 2026-10-13 cross-session handoff closeout

- The session performed read-only confirmation of the current call graph and the
  dirty worktree, then updated handoff/plan/validation documentation only; no
  production code was changed. The six dirty files are the `file_store.rs`
  deprecation guard plus five documentation files (Plan, roadmap,
  platform-handoff, windows-manual-steps, windows-queue); none are committed and
  no formal Windows handoff exists.
- Current-tree validation: `cargo fmt --all -- --check`, `cargo check` for
  `xarchive-storage` and `xarchive-desktop`, Storage lib 124/124, Workflow lib
  22/22, Desktop lib 289/289, `git diff --check` and `node scripts/docs-audit.mjs`
  all PASS. Full workspace regression, strict workspace Clippy, Node/Sidecar
  suites and all Windows-native checks were NOT_RUN; Windows items remain
  consolidated in `docs/validation/windows-queue.md`.
- Open cross-platform work for the next continuation: C1 B3–B7 (owned
  persistence/coordinator/session boundary), E's persisted tool arguments and
  actual Sidecar/aria2 argv wiring, and the F2/F3 shared contracts. D1 remains
  gated on C1 and F2/F3 on C1+D1 per the approved order. The reverted
  service-recovery attempt in `archive_service.rs`/`archive_recovery.rs` (failed
  `cargo check`) must not be repeated as a partial change.

### C1 B3 runner-owned session boundary — 2026-10-13 continuation

- **What was implemented (shared, cross-platform).** A concrete generic
  runner-owned execution-session boundary in `desktop/src-tauri/src/executor.rs`:
  `SessionExecutionBoundary<P, C>` owns a `JobPersistence` context `P` and a
  `Coordinator` `C` and exposes `execute_session(job_id)`. This is the "concrete
  generic execution wrapper" the C1-A checkpoint chose: the Executor/factory
  boundary stays object-safe (`JobExecution`), while the boundary owns the
  context and the session so neither is borrowed across the runner's `'static`
  queue. It consumes the already-proven two-phase Workflow contract
  (`ExecutionReservation::acquire` for Job *and* destination scope, then a
  callback `claim` that runs the durable claim under the guard and binds the
  attempt the claim returned) plus `ExecutionSession`, `AttemptIdentity`,
  `ClaimOutcome` and `SessionError`. No local `JobPersistence` borrow crosses
  the queue and no coordinator is created after the claim.
- **`CROSS_PLATFORM_CHANGE_REQUIRED`: new dependency edge and shared lifecycle.**
  `desktop/src-tauri` now depends on the shared crate
  `crates/xarchive-workflow` (`desktop/src-tauri/Cargo.toml`), so the Desktop
  executor session boundary sits on the shared `Coordinator`/`ExecutionSession`
  contract. The boundary is generic over the port and compiles on Windows; the
  Windows `Coordinator` + NTFS no-replace adapters remain Windows Owner work and
  no platform `cfg` removes the session. This is an architecture/protocol change
  to the shared download pipeline and must route through the Windows Owner by
  formal Git handoff before it is treated as cross-platform accepted.
- **Ordering contract (proven on Linux, fail-closed by construction).**
  `execute_session` runs: runner admission (`runner_try_admit`) → Job/destination
  scope acquisition → durable attempt claim *under the guard* (`reservation.claim`
  callback) → attempt-fenced running-state advances → factory `create` *after*
  the claim → dispatch to the single-active runner with the boundary's owned
  running snapshot (`execute_owned`, never the worker's `jobs` map) →
  attempt-fenced terminal `Downloaded` write → `complete_persisted` finalization
  → session dropped last. The runner reservation is held through finalization;
  a new `SessionRunOutcome::{Finished, Reschedulable}` distinguishes a durable
  decision from contention/lost-claim. Contention (`SessionError::Contended`),
  ClaimLost and a saturated runner (`ExecutorError::QueueFull`) are all
  non-failing: no attempt is incremented and no `FAILED` is persisted, so the
  Job stays queued/reschedulable. Non-reschedulable session-establishment
  failures map to `EXECUTION_SESSION_FAILED` without persisting `FAILED`
  (the Job never obtained a legitimate attempt).
- **Attempt fencing reuses the committed `JobPersistence` contract.** The
  boundary fences every running and terminal write through the existing
  `persist_state_for_attempt` / `record_claimed_attempt_state` / `attempt_is_current`
  trait methods (established by B2) and reports the winning writer's state when
  a newer attempt or a durable cancellation moved the row; it never overwrites a
  session-owned result. The Storage recovery failpoint argument is still unused
  and production recovery is unchanged.
- **Regression coverage (5 new tests, all PASS).** In
  `executor::session_boundary_tests`: `a_session_holds_both_scopes_across_execution_and_finalization`
  (every durable state write, the execution object and the final `Complete`
  finalization observe a peer holding the Job scope, proving the session spans
  execution and finalization), `a_live_peer_makes_the_run_reschedulable_without_any_durable_write`,
  `a_lost_claim_is_reschedulable_releases_scopes_and_persists_no_failure`
  (also proves scopes are released so a successor can claim),
  `a_saturated_runner_is_rejected_before_any_claim` (job stays `Queued`, attempt
  0, no events), and a compile-time `assert_send` proving the boundary can move
  into the scheduler's `'static` thread.
- **Status: B3 shared boundary `IMPLEMENTED` and tested; production wiring still
  gated.** This does **not** switch the production download entry
  (`execute_persisted_from_factory` / `execute_persisted_on_worker`) onto the
  boundary. Production download still uses Job-scoped staging and archive
  publication/finalization inside the execution object are not yet attempt-fenced
  (B4/B5); switching the production entry before B4 (attempt-owned staging,
  proven writer/process-tree exit) and B5 (fenced publication and transactional
  finalization) exist would be exactly the partial session insertion the Plan
  forbids. Production v2 stays fail-closed; B6 and B7 recovery integration stay
  open. D1 remains gated on C1 and F2/F3 on C1+D1 per the approved order — not
  started here.
- **Current-dirty-tree validation (Linux):** `cargo fmt --all -- --check` PASS;
  `cargo check -p xarchive-desktop --lib` PASS; `cargo test -p xarchive-storage
  --lib -- --test-threads=1` 124/124 PASS; `cargo test -p xarchive-workflow
  --lib -- --test-threads=1` 22/22 PASS; `cargo test -p xarchive-desktop --lib
  -- --test-threads=1` 294/294 PASS; `git diff --check` PASS;
  `node scripts/docs-audit.mjs` PASS. NOT_RUN: full workspace regression,
  strict workspace Clippy (pre-existing failures recorded earlier),
  Node/Sidecar suites and all Windows-native checks — none executed here and no
  Linux result substitutes for NTFS/native acceptance. This is not
  `READY_FOR_WINDOWS`: nothing is committed, no formal Git handoff exists, and
  the Windows `Coordinator` adapter has not been implemented or validated.

### Resumption evidence — local date 2026-10-12

- B3 test probes now require explicit contention for both Job and destination
  scopes during factory creation, execution and durable state writes. I/O errors
  are not accepted as evidence that a scope is held.
- PASS: targeted `executor::session_boundary_tests` 5/5, Rust formatting,
  `git diff --check`. The complete test log was reread after a terminal integration
  closure report; this was not a product test failure.
- NOT RUN: B4/B5 production lifecycle implementation and its acceptance matrix;
  full module/workspace reruns for this test-only increment; Windows validation.
  The all-non-Windows goal remains incomplete. C1-dependent D1 and F2/F3 are
  sequencing-gated, not blocked by the absence of Windows. Existing 2026-10-13
  checkpoint dates are retained historical labels, not this session's local date.

### Prerequisite factory-failure fix — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: the B3 boundary now feeds factory creation
  errors into its existing session failure-finalization path. Previously `?`
  escaped after the durable claim and could leave a Job `DOWNLOADING`.
- Added regression for durable failure/event recording, scope release and runner
  admission release after factory failure. PASS: targeted session tests 6/6;
  the independently read log records Cargo exit 0. Terminal integration failed
  to observe completion; that tool issue is separate from the test result.
- Remaining shared prerequisites are development work, not Windows blockers.
  B4–B7, D1, F2/F3 and integrated acceptance are not complete. No production v2
  cutover or Windows validation is claimed by this prerequisite correction.

### Attempt-aware factory prerequisite — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: added object-safe `create_for_attempt` to
  `JobExecutionFactory`. The session passes the actual claimed `AttemptIdentity`
  rather than expecting a factory to infer an attempt from a Job-only snapshot.
- The default refuses unsupported attempt execution; it never delegates to the
  legacy Job writer. Probe factories explicitly implement it and assert identity.
  Production remains fail-closed until B4/B5 writer/publication integration.
- PASS: session tests 6/6; Desktop module 295/295, captured Cargo exit 0;
  fmt-check and diff-check. NOT RUN: integrated B4–B7 acceptance, full workspace
  regression and Windows-native checks. Remaining shared prerequisites must be
  implemented before downstream D1/F2/F3 activation; they are not Windows blockers.

### B4 supervisor ownership prerequisite — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: stdout/stderr reader handles are owned by
  `SidecarSupervisor`, rather than detached at spawn. Shutdown joins completed
  readers and retains pending handles after a bounded wait; `readers_finished`
  reports reader completion only, never full process-tree exit authorization.
- Natural parent exit retains the process-group identity through existing Unix
  termination cleanup, before discarding Child. Added shell fixtures for repeat
  shutdown and a parent exiting with a descendant retaining output pipes.
- PASS: Supervisor module 13/13 and final Desktop module 295/295, Cargo exit 0;
  fmt-check, diff-check and docs-audit. Production B4/B5 integration,
  escaped-descendant containment and final publication authorization remain open;
  this prerequisite must not be used as the complete writer-exit proof.
- Windows native process-tree handling remains Windows Owner work; no Windows
  behavior or native validation is claimed. The Plan is not yet complete.

### B4 incomplete-stop refusal — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: `wait_for_exit` requires direct-child exit
  and finished pipe readers before returning true; it remains explicitly not
  process-tree authorization. Repeat calls after child removal can finish joins.
- Desktop stop retains the supervisor and reports incomplete shutdown rather
  than clearing its error and returning `stopped` while readers are running.
- PASS: Supervisor 15/15, including unfinished-reader refusal/retry and a timeout
  preserving the running child; background Desktop rerun 295/295 with exit 0;
  fmt-check, diff-check and docs-audit. The initial foreground run was interrupted
  by terminal integration, not a diagnosed product failure. Production B4/B5
  integration remains open.

### B4 aria2 process-group prerequisite — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: Unix aria2 now has a dedicated process group.
  Startup failure, explicit shutdown and natural parent exit signal descendants
  through the group before direct-child cleanup. Natural exit removes RPC secret.
- Reuses the existing nix 0.29 dependency family through a Unix-only declaration.
  Added a real shell fixture: parent exits with a pending descendant file writer;
  after cleanup its delayed file must not appear and the secret must be absent.
- PASS: Download 51/51 and Desktop 295/295, captured Cargo exits 0. This is not
  escaped-descendant containment or complete B4/B5 acceptance; production v2
  stays fail-closed. Windows native process-group/tree handling is unchanged.

### Aria2 polling terminal cleanup — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: observing natural exit through
  `Aria2Supervisor::is_running` now invokes existing process-group/secret cleanup;
  callers need not subsequently wait to trigger it. Waiting after cleanup is
  idempotent. The Unix descendant fixture now covers this polling path.
- PASS: Download 51/51; Desktop 295/295; captured Cargo exit 0 for both.
  This does not establish complete writer-tree exit authorization. B4/B5
  production staging/publication integration and subsequent B6/B7/D1/F2/F3
  remain unfinished shared development, to be continued in dependency order.
- NOT RUN: Windows-native validation and full workspace regression. No
  production cutover, commit, push or formal platform handoff is claimed.

### B4 production writer-lifetime boundary review — 2026-10-13

- Reviewed the current production call graph before changing completion or
  publication. The runner's production factory starts one Sidecar per execution
  object; the shared executor owns a single active runner, while Sidecar remains
  alive until the execution context is dropped/replaced. Within that process,
  `download_completed` is emitted after the Python runner returns, but this
  protocol event alone does not establish native descendant exit or reader
  completion. Supervisor shutdown kills the owned process group, waits for the
  direct child, and boundedly joins readers; on timeout it intentionally retains
  unfinished readers and does not prove escaped descendants have stopped.
- A proposed normal-completion shortcut was rejected after tracing
  `handle_v2_command("shutdown")`: while a download is active it sets the
  shutdown flag, which causes the active gallery-dl operation to terminate and
  report interruption. Sending shutdown after `download_completed` therefore
  cannot be described as a graceful drain protocol. Forceful process-group
  cleanup is also not an OS-backed proof for descendants that escaped the group.
- A Sidecar `idle`/`command_finished` protocol event alone would still be only
  worker-level evidence, not native process-tree exit. The existing
  `ExecutionSession` owns the Job/destination guard, but production scheduling
  still dispatches an execution object through a `'static` runner channel and
  performs recovery dispatch and terminal persistence outside the session
  boundary. Wiring only the event or only the session into `ArchiveExecutionJob`
  would not retain both writer-exit evidence and attempt authorization through
  staging verification, cleanup, and finalization.
- aria2's process-group supervisor has a Linux delayed-writer regression and
  terminal polling cleanup, but the current production transfer path does not
  expose a typed, successful writer-tree-exit authorization to archive completion.
  Re-verification after cleanup alone would still be insufficient while the
  Sidecar writer or escaped descendants may retain access to attempt staging.
- Therefore no production metadata/Telegram-intent/completion root was switched
  to attempt staging in this review: doing so without a reliable lifetime barrier
  would turn an attempt-scoped path into an unproven shared mutable input. B4
  remains `IN_PROGRESS`; v2 remains fail-closed and B5 remains open.
- Next implementation gate: define a Sidecar drain/idle-and-exit contract that
  does not cancel active work (or provide equivalent OS-backed supervision),
  and an attempt-scoped authorization value for both Sidecar and aria2 that can
  remain held through staging verification and cleanup. Integrate runner
  ownership, recovery dispatch, archive completion, and terminal persistence
  around that same session before enabling the production path. Then add
  delayed-writer regressions through the production archive path. Existing
  supervisor fixtures are prerequisite evidence only. Escaped-writer containment
  and native Windows process-tree behavior must not be inferred from Linux
  process-group tests.
- No production behavior or Windows result changed in this continuation. The
  pre-existing uncommitted `commands.rs` change remains separate and untouched;
  do not include it in a B4 handoff without its own lifecycle review.

#### Idempotent terminal polling follow-up — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: repeated `is_running()` after natural-exit
  cleanup now consistently returns `false`; it no longer returns
  `SupervisorNotRunning`. The regression exercises poll → wait → poll.
- PASS: targeted test 1/1, Download 51/51, fmt-check, diff-check and docs-audit.
- This remains a supervisor lifecycle refinement, not complete process-tree
  authorization or B4/B5 production integration. Windows checks remain NOT RUN.

### Existing attempt staging ancestor containment — 2026-10-12

- `CROSS_PLATFORM_CHANGE_REQUIRED`: resolving an existing attempt staging tree
  now validates the staging root and Job ancestor with `symlink_metadata`,
  refusing symlink/reparse-point replacement just as fresh attempt creation
  does. This protects recovery and attempt preview from ancestor redirection.
- Added Unix regression for a replaced staging root and replaced Job directory.
  PASS: corrected targeted test 1/1 and Storage module 125/125; fmt-check,
  diff-check and docs-audit PASS. An initial `--exact` invocation selected zero
  tests and is explicitly not counted.
- This is a containment primitive, not B4 production writer wiring or B5
  publication authorization. Windows validation and integrated C1 tests remain
  NOT RUN; no Windows result is inferred.

### B4 session writer immediate-parent durability — 2026-10-13

- Input validation ordering correction: session/relative-path checks precede
  root pinning again, preserving invalid-path classification even with a
  replaced root. Traversal/absolute/empty-path regression PASS; valid paths
  still enforce directory identity. Workflow 32/32, doctests 7/7, strict
  Clippy/fmt PASS, exit 0. No containment or production enablement claimed.

- Root identity fence (`CROSS_PLATFORM_CHANGE_REQUIRED`): adapters record
  descriptor-observed device/inode; pin and ordinary session writes reject
  mismatched roots. Same-name replacement regression PASS; already pinned
  writes continue to the original directory. Workflow 32/32, doctests 7/7,
  strict Clippy/fmt PASS, exit 0. Snapshot cannot exclude inode reuse after
  deletion or replacement between mkdir and first open; retained creation-time
  capability, descendant isolation and production containment remain open.

- Pinned boundary verification: compile-fail tests reject early bound-writer
  consumption and Clone requirements, with a compiling positive example.
  Runtime tests reject directory/final-file links and missing intermediates
  after root replacement, with no outside creation. Doctests 7/7, Workflow
  32/32, strict Clippy and fmt-check PASS, exit 0. Test/documentation-only
  change; native containment, production integration and B5 remain open.

- Pinned-root follow-up (`CROSS_PLATFORM_CHANGE_REQUIRED`): session writer can
  retain a private Unix root descriptor; writes after root rename/same-name
  replacement stay in the pinned original directory. Regression also checks
  traversal and overwrite refusal. Workflow 32/32, doctests 4/4 and strict
  Clippy PASS, exit 0. Pin-time identity is not creation-time identity; descendant
  replacement and moving the root outside staging remain possible. This is
  not filesystem isolation/containment or B5 publication authorization.

- Lifetime verification follow-up: three compile-fail examples reject session
  consumption before writer use, domain consumption before writer use, and
  Clone requirements; one positive generic resolution example compiles.
  Doctests 4/4, Workflow 32/32, strict Clippy and fmt-check PASS, exit 0.
  This adds type-contract evidence only, not pinned directory identity,
  external writer containment or publication authorization. No runtime change.

- Existing-query follow-up (`CROSS_PLATFORM_CHANGE_REQUIRED`): recovery lookup
  now opens all attempt-path components descriptor-relatively with `DIRECTORY`
  and `NOFOLLOW`, without creating missing paths. Valid lookup, staging link,
  missing/file/linked attempt and existing linked-Job regressions PASS.
  Workflow 32/32, strict Clippy and fmt-check PASS, aggregate exit 0.
  Returned domain is still an unpinned path binding, not a recovery permit.
  Later namespace stability, ancestor durability, production containment and
  B5 remain open. Full/Windows/cgroup acceptance not run in this isolated batch.

- Fresh-creation follow-up (`CROSS_PLATFORM_CHANGE_REQUIRED`): staging/Job/
  attempt directories use descriptor-relative mkdir/open with `NOFOLLOW`,
  replacing check-then-path-create. New directories request owner-only access;
  existing attempts remain refused. Linked-ancestor/no-outside-create and
  private nested-attempt regression PASS. Workflow 31/31, strict Clippy and
  fmt-check PASS, captured exit 0. Returned paths are not pinned capabilities;
  rename stability, ancestor durability, path-based recovery inspection and
  production containment remain open. No B4/B5 completion or v2 enablement.

- Follow-up: private deterministic sync-failure tests cover file-sync and
  parent-sync failure, immediate error propagation, retained payload and
  create-new retry refusal. Production always uses `File::sync_all`.
  Workflow 30/30, strict Clippy and fmt-check PASS with captured aggregate
  exit 0. Initial Clippy module placement error corrected; terminal close-state
  reporting failure recorded separately. This is not a power-loss simulation.

- `CROSS_PLATFORM_CHANGE_REQUIRED`: create-new session writes sync the retained
  immediate parent directory after the file. Either sync failure returns an
  error, and failure can leave a created file. No writable handle escapes.
- Extended session-write coverage for nested payloads and empty files.
  Workflow 29/29, strict module Clippy and fmt-check PASS on Linux WSL2.
  These tests do not simulate power loss or inject sync failure.
- Previously created ancestors are not synced by this operation; directory
  rename stability, OS-backed writer containment, production lifecycle wiring
  and B5 publication/recovery remain open. v2 stays fail-closed. Windows and
  cgroup acceptance NOT_RUN in this continuation. Full regression not run:
  isolated workflow primitive change; prior uncommitted changes preserved.
