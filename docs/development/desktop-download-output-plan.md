# Desktop Download Output and Settings Plan

Owner: Cross-platform Owner for shared contracts, Rust/Storage/Sidecar integration,
and shared UI; Windows Platform Owner for Windows-specific integration, native GUI,
Status: `IN_PROGRESS` — current owner is the Cross-platform Owner on branch
`cross-platform/automatic-pairing-reconcile-20261002`, source HEAD
`41f11a504c92a20155ac8ebca1a98411c19491f6` (documentation gate for B3–B7);
this reconciliation note is uncommitted. The working tree carries no production
change beyond that documentation batch: undocumented `ExecutionSession` residue
was removed so the tree matches the gate record.
This is shared WIP, not a Windows handoff. E3 settings,
v3 task/batch snapshots, selected-tool policy and compatibility checks are
implemented. Linux actual-spawn acceptance is verified (aria2 supervisor and
gallery-dl DownloadRunner argv paths); gallery-dl user arguments remain disabled
by the empty allowlist. Windows acceptance remains pending. The overall Plan is
not complete: C1.4 production recovery, D1, F1–F3 and integrated G remain open.
The C1 continuation remains separate shared work and
production C1 v2 remains fail-closed. Windows acceptance requires formal Git
handoff and a fresh artifact. Historical commit references below describe prior
checkpoints, not the current tree.

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
| C1.2 | `xarchive-workflow` crate and same-host multi-process coordination | C0/D0 | Shared implementation of crate contracts, Unix `flock` adapter and process-shared registry, with cross-process and same-process contention tests; Windows adapter not implemented |
| C1.3 | Filename renderer, attempt-isolated staging, replayable file operations and attempt fencing | C0/D0, C1.2 | Shared staging/replay helpers implemented and targeted-tested; full attempt-ownership fencing and production wiring open; native no-replace mover is Linux-only |
| C1.4 | Attempt-fenced archive finalization and production recovery wiring | C1.2/C1.3 and D0 | Cancellation/PREPARED and attempt-fenced state-write primitives are tested; production recovery wiring and service restart matrix remain open |
| D1 | Optional user JSON/TXT outputs, separate from recovery facts | C0/D0, C1 | Contract finalized; four JSON/TXT combinations blocked until the C1 shared gate passes; legacy archives are not rewritten |
| E | v3 task snapshots and user downloader arguments through actual tool argv | B2 | E0–E3 shared implementation and actual spawn acceptance are present; Windows Named Pipe and packaged-artifact checks remain queued |
| F | Theme, configuration exchange and download/archive record exchange | Settings schema decisions; three scoped streams after contract review | F1/F2 implementation not started; F3 contract is confirmed in §11, implementation not started |
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

The current non-Windows order is:

1. **P0 — baseline and documentation synchronization (`COMPLETED`):** confirmed
   clean pushed baseline `8b2b544206e62e1356ef582e46d62b92be14cb04`; re-ran
   Storage 106/106, Desktop 269/269 and Download 35/35 on that source. These are
   targeted results, not full-workspace or Windows acceptance.
2. **C1.1 — durable recovery contract (`COMPLETED`; Linux module verification PASS):**
   the v2 manifest is an immutable plan (`rename_phase` remains `PLANNED`); mutable
   progress is stored in `rename_progress`/`phase`. Creation relies on migration
   0017's `rename_progress` default and does not name that column in the INSERT.
3. **C1.2 — `xarchive-workflow` foundation and multi-process coordination (`IN_PROGRESS`; `CROSS_PLATFORM_CHANGE_REQUIRED`):** the crate defines bounded identity/phase/path contracts and Linux now has a Unix `flock` adapter (replacing the retired per-process `fcntl` record locks) with a process-shared local lock registry, `O_NOFOLLOW` symlink rejection and group-lock rollback. `cargo test -p xarchive-workflow --test coordination` passes 11/11, covering cross-process and same-process contention, crash release, guard drop/release, partial-group rollback, fork/exec descriptor residue, parallel jobs and destination scope. Remaining shared work is wiring Job/destination ownership into production lifecycle; no production lifecycle holds its guards yet. Windows adapter remains Windows-owned.
4. **C1.3 — replayable file operations and attempt fencing (`IN_PROGRESS`; `CROSS_PLATFORM_CHANGE_REQUIRED`):** Storage replay accepts only immutable `RenameEdge` plus `ArchiveRenamePlan` identities, validates lexical containment and link/reparse points, distinguishes missing from I/O errors, and checks size/hash for both pending and already-completed edges. Linux moves use atomic `renameat2(RENAME_NOREPLACE)`; non-Unix remains fail-closed pending the Windows Owner adapter. `create_attempt_file` performs NOFOLLOW create-new creation, and the attempt staging/replay helpers are implemented and targeted-tested, but they are not wired into Sidecar/Executor. Persistent fencing at all file writes remains unimplemented.
4. **C1.4 — C1 finalization and production wiring (`IN_PROGRESS`; production remains fail-closed):** `recover_internal_rename_v2` still returns a fail-closed error, the service failpoint parameter is present but not wired to operation boundaries, and Desktop startup still returns `ARCHIVE_RECOVERY_V2_NOT_READY`. Service-level process restart coverage and read-only COMMITTED filesystem+DB verification are absent. The cancellation/PREPARED linearization boundary is implemented and targeted-tested (`archive_commit_started` advisory probe plus `decide_archive_cancellation` re-checking both journal generations inside one transaction, exposed to Desktop as `persist_cancellation`/`CancelArbitration`); `begin_archive_attempt` also rejects terminal Jobs transactionally. These primitives are not yet bound to a production lifecycle and do not provide atomic Job claim or full attempt ownership: attempt-isolated writes for both download paths, replay and no-replace directory commit, complete finalize/replay wiring, and the service crash/restart matrix remain shared gates. Index handling also needs an end-to-end contract audit: current inspected Sidecar extraction/model code enumerates from 1, core rejects index 0, and SQLite requires positive indices; do not characterize this as a confirmed production zero-based Sidecar mismatch without evidence from every conversion boundary. Windows coordination/no-replace adapters and NTFS acceptance remain Windows-owned. D1 exports-off stays disabled until all shared gates pass; E and F are separate shared development streams, not Windows manual checks.
6. **D1 (`PLANNED`, gated on C1):** activate the four JSON/TXT output combinations
   without making internal recovery depend on user exports or rewriting legacy
   archives.
7. **E (`IN_PROGRESS`; `CROSS_PLATFORM_CHANGE_REQUIRED`):** preserve v1/v2 task-spec behavior; add v3 for typed argument snapshots and policy/tool compatibility identity; make Rust the sole authority for user-option policy checks at task creation and immediately before execution; and connect gallery-dl and aria2 argv end-to-end. Python retains protocol shape and argv-boundary handling but does not duplicate the user-option allowlist/value policy. Share the parsed option contract with the pinned tool versions; unknown user options fail in Rust before process startup. **E0 (Rust argument-policy authority) is now implemented and corrected:** `crates/xarchive-download/src/downloader_args.rs` defines a versioned `ARGUMENT_POLICY_VERSION`, a narrow reviewed `DOWNLOADER_ARG_ALLOWLIST` (retry/timeout/concurrency knobs only), an explicit `PROTECTED_ARIA2_OPTIONS` set (output/proxy/credential/config/RPC/hook/reporting) that classifies rejections without ever permitting an option, JSON-string-array parsing that preserves argv boundaries and rejects shell metacharacters/`@file` indirection/`--`/attached/short forms, and `build_aria2_argv` with per-option value ranges. The pre-integration prototype's contract gap (it accepted `--out`/`--all-proxy` overrides) is fixed; the former "gap-recorded" tests are replaced by protected-option acceptance assertions, and policy-data tests lock the allowlist/protected sets against duplicates and overlap. The module remains a self-contained policy authority with **no production caller yet**. **E1 (v3 execution-spec snapshot) is now implemented:** `desktop/src-tauri/src/executor.rs` defines `ToolCompatibility` (per-engine pinned version, `None` when an engine carries no arguments), `DownloaderArgumentSnapshot` (validated argv arrays plus `policy_version` and `tool_compatibility`, `deny_unknown_fields`), the `ExecutionSpecV3` envelope, a `DecodedExecutionSpec.downloader_arguments` field, and version-3 dispatch in `decode_execution_spec`. v1/v2 continue to decode to an empty `DownloaderArgumentSnapshot::historical()` (never resampled from current settings); the argument-policy version is validated independently of the execution-spec version via `validate_downloader_argument_snapshot`, and a foreign policy version is refused rather than reinterpreted. Linux unit tests (5 new, then 277/277 in the desktop lib) cover v3 request/output/snapshot decode, v1/v2 empty-historical decode, policy-version mismatch refusal, invalid JSON, unknown snapshot field, and byte-for-byte argv round-trip. The argument-policy version is dispatched separately from the schema version (the DB `schema_version` column stays authoritative and no duplicate version is added to JSON). **E2 (both engines wired) is now implemented:** the decoded `downloader_arguments` snapshot flows from `ProductionExecutionFactory::create` into `ArchiveExecutionContext.with_downloader_arguments` and survives every error-recovery context rebuild. The gallery-dl direct-download path forwards `snapshot.gallery_dl` as the `SidecarV2Command::download` `user_args`, which the Python worker passes to gallery-dl's argv (structure/argv-boundary only, no duplicate allowlist). The aria2 path threads `snapshot.aria2` through `transfer_with_one_refresh` → `transfer_once` → `aria2_config` → `Aria2SupervisorConfig::with_user_args`, and `command_args()` appends them after the application-owned options so a user value wins for a shared option while every protected option was already rejected upstream; the RPC `--conf-path` secret is still appended last by the spawner. The internal worker `executable_args` (Python-worker launch) is kept separate from user args. New targeted tests: `xarchive-download` 46/46 (user_args appended after application options; empty user_args leaves the argv shape unchanged) and desktop `aria2_config_forwards_validated_user_args_into_the_spawned_config`. Still absent: E3 (no submit path writes a v3 spec yet — every production submit still encodes `schema_version: 2`; there is no UI JSON-array input, no export redaction, no fake-process acceptance), and re-verifying the persisted snapshot in Rust immediately before spawn. Until a submit path and UI emit v3 with user args, E is wired but not yet reachable as a production user feature.
8. **F (`IN_PROGRESS` at plan level; streams individually tracked):** F1 theme authority and token audit plus F2 versioned local configuration exchange remain `PLANNED`; F3 download/archive record exchange has a confirmed contract in §11 but implementation remains `PLANNED`. No further scope expansion is authorized without an explicit decision. None is a Windows-only blocker.
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
remain stored internally even when `tweet.json` and `tweet.txt` exports are off.
Template tokens are limited to the flat `{username}`, `{tweet_id}`, and
`{index}` example; extension remains application-controlled.

1. Preserve the independent v2 execution-spec envelope for output settings. Read
   historical v1/v2 rows without rewriting them; reject unknown versions
   explicitly. Add v3 for downloader-option snapshots. The database
   `schema_version` column is authoritative; do not duplicate it in JSON.
2. Snapshot output settings and typed downloader options/policy identity only. Do
   not snapshot the entire runtime environment. Telegram configuration continues
   to be sampled at archive completion.
3. Output settings initially contain naming mode/template and independent JSON/TXT
   output switches. Both outputs default on to preserve current behavior. Do not
   expose field selection or custom TXT formatting in the first implementation.
4. Keep `Tweets/<tweet_id>` fixed. First naming template is a flat media filename;
   extension is application-controlled. Reject conflicts; never overwrite or add
   an implicit random suffix. Rust is the single renderer for preview and commit.
5. Keep internal recovery facts in a database journal plus an internal manifest,
   separate from user JSON/TXT files. A present corrupt/unknown internal record is
   an explicit recovery error, not a reason to fall back to stale user exports.
6. Capture one output-settings snapshot when an account batch is created; all its
   child jobs inherit it, including after pause/resume. One-off jobs snapshot at
   submission. Reusing an active job never replaces its persisted snapshot.
7. Complete and verify independent recovery facts before exposing JSON/TXT output
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
2. **B1 — admission and backpressure (`IMPLEMENTED` for the runner-admission core):**
   move queue/capacity rejection before any durable execution claim. The runner
   now exposes a reservation (`runner_try_admit`/`RunnerReservation`) that both
   `execute_persisted_from_factory` and `execute_persisted_on_worker` acquire
   *before* the durable attempt claim; a saturated single-active runner returns `QueueFull`
   without incrementing an attempt or persisting `FAILED`, and the Job stays
   queued/reschedulable. Regression
   `saturated_runner_rejection_is_non_failing_and_reschedulable` proves rejection
   is non-destructive and the Job runs once the runner frees up. Remaining with
   B2/B3: typed coordination-contention and claim-loss outcomes and cross-process
   admission; `Closed` still follows the established unavailable path. Do not
   mask backpressure with an unbounded queue.
3. **B2 — atomic state-plus-attempt claim (`IMPLEMENTED` for the production
   claim wiring):** the Storage `claim_archive_attempt` transaction already checks
   expected state/attempt and PREPARED facts, increments the attempt with checked
   arithmetic, and writes the initial running state plus its event atomically.
   Both `execute_persisted_from_factory` and `execute_persisted_on_worker` now use
   `claim_attempt` for the first running transition (instead of a separate
   `begin_attempt` followed by an unfenced first write), so a losing claimant
   returns the winner's snapshot with no side effects and a Job already in its
   running state still obtains an attempt identity via `begin_attempt`. The full
   286-test Desktop suite passes unchanged, proving the state/event sequence is
   preserved. Remaining with B3+: typed loss/contention outcomes surfaced to the
   caller and cross-process admission; PREPARED/COMMITTED routing to recovery
   stays fail-closed.
4. **B3 — runner-owned execution session (`PLANNED`, no code retained):** once a
   runner has accepted and dequeued work, acquire the Job guard, perform the
   atomic claim, and create an immutable attempt identity/session using the
   existing Workflow coordinator port. Pass it through the execution factory and
   retain its guard for the full writer and publication lifecycle. Use
   deterministic Job-then-destination lock ordering. Test one winner for
   concurrent same-Job claims, parallel distinct Jobs, same-destination
   serialization, and guard release on every exit path. A session-owned result
   must not be overwritten by a second generic persistence path. An earlier
   isolated `ExecutionSession` helper backed by a `Noop` coordinator was removed
   during the documentation gate reconciliation because it proved no lifecycle
   behavior; the session must be introduced together with real
   Job/destination authorization as a shared `CROSS_PLATFORM_CHANGE_REQUIRED`
   contract, not as a stand-alone type. The Windows adapter implements that
   shared port and remains Windows Owner work.
5. **B4 — attempt staging and writer lifetime (`PLANNED`):** wire both gallery-dl
   and aria2, archive metadata and temporary files to the session's unique attempt
   directory. Validate every reported path is within that attempt. Prove all child
   writers/process trees have exited before prepare, publication, guard release or
   cleanup; a completion message alone is insufficient. Cancellation must stop and
   join writers. Define restart recovery and conservative orphan cleanup so a stale
   attempt can neither publish nor delete newer files. Exercise both backends with
   fake processes, including late writes and parent/child termination. Windows
   coordination adapter and NTFS/native process validation remain Windows Owner work.
6. **B5 — fenced publication and durable finalization (`PLANNED`):** require the
   live attempt identity and held Job/destination authorization at final destination
   no-replace publication and at transactional DB finalization. Keep cancel/PREPARED
   arbitration: cancellation may win only before durable PREPARED. A counter check
   before a publish call is not a fence. Test stale-attempt denial, destination
   collision, prepare/cancel races and complete rollback of DB facts/events.
7. **B6 — shared commit/replay and restart matrix (`PLANNED`):** route normal
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

### C1-A initial analysis checkpoint — 2026-10-11

- **Status at initial analysis: `IMPLEMENTATION_NOT_READY`;** the subsequent
  targeted B1 dispatch distinction is recorded in §13 and does not close C1-A.
  Re-traced the production executor, Storage claim/state APIs, FileStore,
  Sidecar request and archive completion. The existing claim primitive is not
  called by production; the production path separately increments attempt,
  persists state, then schedules a bounded runner. Queue rejection is currently
  reported as an execution error and persisted as `FAILED`.
- **Verified design sequence:** B1 admission/backpressure, B2 atomic state-plus-
  attempt claim, B3 runner-owned session/guard, B4 attempt staging and confirmed
  writer exit, B5 fenced publication/finalization, B6 shared replay/restart matrix,
  then B7 Desktop recovery integration. The sequence above is the actionable
  C1-A/C1 implementation plan; this checkpoint records prior analysis, not a
  blocker on further shared design or implementation.
- A safe C1-A slice cannot be closed by adding a lock at the runner: both download
  paths still receive Job-level staging, the child writer must remain fenced until
  process-tree exit, and `complete_sidecar_archive`/directory publication does
  not check the attempt. A newer attempt could therefore revoke a counter check
  while an old writer still has a path to mutate/publish. The factory API also
  creates execution resources on the runner thread after dispatch, while durable
  attempt ownership is held by a separate persistence path.
- Required next implementation boundary: change dispatch so capacity/backpressure
  is resolved before the durable claim, atomically claim state+attempt in Storage,
  pass immutable identity and a held coordinator guard into the production
  execution, use attempt staging for Sidecar and archive metadata, retain the guard
  until all downloader child processes have exited, and fence final publication
  plus durable finalization. Contention must reschedule/remain non-failed; it must
  not flow through generic `EXECUTOR_WORKER_FAILED`. Add one integration test
  spanning claim → writer → stale-attempt denial → destination commit before
  removing the v2 fail-closed boundary.
- Linux capability detected: WSL2/Linux x86_64; Unix `flock` adapter can be tested.
  Windows coordinator/no-replace implementations and NTFS/native process evidence
  remain Windows Owner work and are not inferred from Linux. No attempt was made
  to enable v2 recovery or D1.

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

The v2 envelope is `{ "request": <historical ArchiveTweetRequest>,
"output_settings": { "naming_mode": "original" | "template",
"filename_template": string, "export_json": boolean, "export_text": boolean } }`.
The request JSON keeps its v1 field shape; v2 adds only the typed output snapshot.
Defaults are `original`, an empty template, and both exports enabled. These
defaults preserve the current filename and metadata behavior. The version remains
authoritative in the database column, and unknown fields in v2 settings are
rejected rather than ignored.

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
then persist a recovery record. The planned `xarchive-workflow` crate and its
cross-process coordination contract are prerequisites to production C1 wiring.
Renames must be idempotently recoverable across
then persist a recovery record. The planned `xarchive-workflow` crate and its
cross-process coordination contract are prerequisites to production C1 wiring.
Renames must be idempotently recoverable across
crashes, verify source/destination identity and hash/size, and never overwrite.
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
hash, phase, and required intent linkage) from optional user exports (JSON and
text). Internal recovery must not depend on user export switches and must not
store signed URLs, cookies, or browser credentials. Preserve current
`tweet.json`/`tweet.txt` defaults. Do not rewrite historical metadata. The
internal journal/manifest foundation is D0 and must be completed before C1 rename
operations depend on it; user-facing output switches are D1 after C1 acceptance.
Field selection and custom TXT formatting are deferred beyond this first release.

Recovery precedence: valid new internal record; otherwise explicitly recognized
legacy format; a present but corrupt/unsupported new record is a clear failure,
not a fallback to stale legacy files. Coordinate its schema and path map with
B/C; a sidecar JSON file alone does not establish crash consistency.

Acceptance covers all JSON/TXT combinations, field selection and text formatting,
legacy/new/missing/corrupt/unknown-version recovery, and crashes with user exports
disabled.

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

Download records and archive records are separately exported/imported as distinct
files with their own state and hash metadata. The user confirmed optional import
of both files from ZIP or 7z; other archive formats are deferred. Each may also
be imported individually, with an explicit GUI warning describing the behavior
when the counterpart record file is absent. Import previews first and merges
idempotently; path/size/hash conflicts reject the affected archive record. Import
never re-downloads, overwrites archive files, or marks a Job COMPLETE without
verified matching facts. Each export GUI displays a warning banner about possible
sensitive information. Configuration files and record files are never combined.

With active tasks, do not interrupt implicitly; task handling must be explicit.
A failed validation/write leaves existing configuration/records intact and must
not use startup's fallback-to-default path as import success. Do not expand shell
or filesystem permissions. ZIP/7z extraction must reject traversal, links,
duplicate/ambiguous entries, and unexpected payloads. TAR, gzip and other formats
are future work.

Acceptance covers round-trip, unknown/invalid schema, failure atomicity, active
task handling, explicit restart confirmation, theme token coverage, separate
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
records the v3 snapshots and Linux actual-spawn evidence. F remains unimplemented
unless a later dated checkpoint says otherwise. These are shared development items,
not Windows blockers.

### Next implementation batch — N0 baseline, then C1 production coordination

N0 is the next shared implementation batch; C1-A follows only after its baseline
gate passes. N0's Desktop persistence trait/impl placement and claim fixtures have
been repaired and the targeted baseline checks passed on the current uncommitted
tree. N0 is `VERIFIED` for this bounded slice; neither that status nor helper
presence closes production admission or C1-A.
Before editing production lifecycle code, inspect the production task claim,
executor dispatch, both gallery-dl/aria2 writer lifetimes, staging creation and
archive completion call paths. Then wire Job and destination guards with a fixed
lock order into the existing production lifecycle; atomically claim the Job and
attempt before spawning a downloader; give each attempt an isolated staging root;
ensure stale workers cannot publish, clean, or finalize a newer attempt; and retain
the guard until all spawned writers have exited. Preserve contention as a
reschedulable outcome, not a terminal download failure. Do not enable v2 startup
recovery or D1 during this batch.

Acceptance for C1-A: targeted tests prove one downloader starts for concurrent
claims of the same Job, different Jobs remain concurrent, destination collisions
serialize, both backends write only to attempt-owned staging, a stale attempt
cannot publish/finalize, cancellation and child shutdown do not race publication,
and contention does not create a false FAILED state. Run Workflow coordination,
Storage job/staging and Desktop executor tests, then the affected subsystem suites
as required by the observed call graph. Record Linux evidence separately from the
Windows lock/NTFS implementation and validation, which remain Windows-owned.

## 13. Progress

### 2026-10-11 documentation gate reconciliation

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
- Verification on the current uncommitted tree: `cargo fmt --all` clean;
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
- Verification on the current uncommitted tree: `cargo fmt --all -- --check`
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
- Current uncommitted source passed `cargo fmt --all -- --check`,
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
  remains gated on C1. F1/F2 remain planned. F3's scope is specified in §11 and
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
| D1 — metadata outputs | BLOCKED pending C1 shared gate | JSON/TXT switches remain stored/snapshotted but non-operative. The v2 recovery manifest carries the switches, but production must not omit either export until the recovery dispatcher and final-path DB/Telegram consistency are implemented and tested. State matrix: on/on, on/off, off/on, off/off, with legacy/new recovery and exports-off recovery checks. |
| E — tool arguments | Shared E3 implementation and Linux acceptance verified; Windows acceptance pending | E0 policy/protected-option handling, E1 v3 decoding/snapshots, E2 argv wiring and E3 settings/persistence/batch/tool-compatibility/redaction are implemented. Linux actual-spawn acceptance: `cargo test -p xarchive-download --lib` 50/50, including readiness-driven `Aria2Supervisor::spawn` argv order, final `--conf-path`, secret absence and cleanup; Sidecar downloader/protocol tests 30/30, including actual `DownloadRunner` spawn argv boundaries. Formatting, diff-check and docs audit PASS. gallery-dl user arguments remain rejected by the empty allowlist. Windows Named Pipe/native process/packaged-artifact checks remain `BLOCKED` or `NOT_RUN` pending formal handoff and fresh artifact. This closes only shared E3, not C1/D/F or the overall Plan. No execution-spec export API exists; do not claim export-redaction support. |
| F1 — theme authority/token audit | PLANNED | No persisted theme authority or token audit implementation in current tree; implement and test shared WebView behavior, then queue native titlebar/dialog/DPI for Windows. |
| F2 — versioned local configuration exchange | PLANNED | No exchange schema, preview, path-preservation merge, atomic import/export commands or UI; include approved local path fields, exclude secrets, and test unknown schemas/failure atomicity before claiming implementation. |
| F3 — download/archive record exchange | PLANNED | Contract is confirmed in §11 (separate files, optional ZIP/7z import, preview, idempotent merge, conflict rejection, no overwrite/re-download/false COMPLETE); implementation and contract-level tests remain open. Do not expand formats beyond ZIP/7z without an explicit decision. |
| G — integrated Linux verification | IN_PROGRESS | Targeted E3 suites, formatting, diff check and docs audit pass. Full workspace regression and integrated Plan acceptance are not complete; run them after shared implementation closes. |
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
