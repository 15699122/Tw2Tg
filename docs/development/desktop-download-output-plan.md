# Desktop Download Output and Settings Plan

Owner: Cross-platform Owner for shared contracts, Rust/Storage/Sidecar integration,
and shared UI; Windows Platform Owner for Windows-specific integration, native GUI,
packaging, and Windows validation.

Status: `IN_PROGRESS` — current owner is the Cross-platform Owner on branch
`cross-platform/automatic-pairing-reconcile-20261002`, current revision
`b9ac571448c9e093c1639ca0586718dc9dd7bcc4`, two local commits ahead of origin.
The 2026-10-09 continuation committed and module-tested C1.1's immutable-manifest
phase-machine repair; C1.2/C1.3/C1.4 remain incomplete and production v2 remains
fail-closed. E/F remain shared implementation work. This is not a Windows handoff.
Windows acceptance requires the formal Git handoff and a fresh artifact. Historical
commit references below describe prior checkpoints, not the current tree.

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
  positive `schema_version`, request ID, and JSON. New tasks currently save version
  1; `ProductionExecutionFactory` reads the version but currently ignores it when
  decoding request JSON. The implementation must add explicit v1/v2 dispatch and
  reject unknown versions. This is a confirmed compatibility gap, not an assumption.
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
  gallery-dl extraction process configuration. Neither is automatically a user
  custom-arguments interface.
- Existing config persistence and selected settings are implemented. This does
  not establish theme support or safe versioned import/export.

## 3. Batches and dependencies

| Batch | Scope | Depends on | Main verification |
|---|---|---|---|
| A | Existing page split and project-style UI | Current handoff | Frontend wiring; Windows fresh-artifact GUI remains pending |
| B1 | Explicit execution-spec version decoding, historical v1 compatibility, unknown-version rejection | None | Executor tests: v1 valid/invalid, unknown version, identity mismatch, reject before process startup |
| B2 | v2 envelope and immutable output-settings snapshot, wired to one-off and batch submission | B1 | Executor/Storage/batch tests: defaults, snapshot persistence, reuse, changed settings, retry and restart |
| C0/D0 | Internal archive recovery journal/manifest independent of optional user exports | B2 | Storage/archive integration and injected crash boundaries; old/new/corrupt recovery records |
| C1 | Shared filename renderer, complete path plan, recoverable rename | C0/D0 | Storage/archive integration; path, collision, rename interruption and final-path consistency |
| D1 | Optional user JSON/TXT outputs, separate from recovery facts | C0/D0, C1 | JSON/TXT option matrix, legacy/new recovery, disabled-output recovery |
| E | User downloader arguments through actual tool argv | B2 | Rust/Python contract, argv ordering, rejected protected options, redaction and failure behavior |
| F | Theme and versioned local config import/export | Settings schema decisions; independent UI work may proceed after contract review | Frontend/config round-trip, strict validation, failure atomicity and theme token coverage |
| G | Integrated Linux verification and Git handoff | Applicable batches | Risk-based module/subsystem checks, docs audit, diff review; exact source revision recorded |
| H | Windows implementation/validation batch | Git handoff from G | Fresh artifact and worker provenance; native path, process, recovery, GUI, DPI and configuration checks |

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
   This completes the representation decision, not production integration.
3. **C1.2 — replayable file operations (`IN_PROGRESS`):** implement deterministic source,
   temporary and final path handling, identity checks, no-overwrite behavior,
   link/reparse rejection, and interruption replay.
4. **C1.3 — atomic finalization (`IN_PROGRESS`):** transactionally commit Tweet/media/
   Job/journal facts and optional Telegram linkage; validate final facts on
   `COMMITTED` replay.
5. **C1.4 — production wiring (`IN_PROGRESS`; production remains fail-closed):** connect startup recovery, attempt
   fencing, rename execution, actual service failpoints and restart-based crash
   matrix. Keep v2 fail-closed until this gate is verified.
6. **D1 (`PLANNED`, gated on C1):** activate the four JSON/TXT output combinations
   without making internal recovery depend on user exports or rewriting legacy
   archives.
7. **E (`IN_PROGRESS`; `CROSS_PLATFORM_CHANGE_REQUIRED`):** current parser's
   proxy/output allowlist conflicts with the protected-options contract. Define
   versioned tool-specific option fixtures, persist typed per-task snapshots,
   and connect gallery-dl and aria2 argv end-to-end before considering
   implementation ready.
8. **F (`PLANNED`):** theme, versioned configuration exchange, separate download and
   archive record formats, then bounded ZIP/7z import with strict preview/merge
   behavior.
9. **G/H:** run risk-based Linux integration/regression, inspect the final diff,
   commit and formally hand off; Windows-owned integration and validation follow.

The C1 subphases are `CROSS_PLATFORM_CHANGE_REQUIRED`: they change shared recovery
and persistence semantics. E protocol/config changes and F exchange schemas are
also `CROSS_PLATFORM_CHANGE_REQUIRED`; Windows-native process, filesystem, GUI,
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

1. Use an independent v2 execution-spec envelope. Read historical v1 rows without
   rewriting them; reject unknown versions explicitly. The database
   `schema_version` column is authoritative; do not duplicate it in JSON.
2. Snapshot output settings and future user downloader arguments only. Do not
   snapshot the entire runtime environment. Telegram configuration continues to
   be sampled at archive completion.
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

The minimum v2 envelope is `{ "request": <historical ArchiveTweetRequest>,
"output_settings": { "naming_mode": "original" | "template",
"filename_template": string, "export_json": boolean, "export_text": boolean } }`.
The request JSON keeps its v1 field shape; v2 adds only the typed output snapshot.
Defaults are `original`, an empty template, and both exports enabled. These
defaults preserve the current filename and metadata behavior. The version remains
authoritative in the database column, and unknown fields in v2 settings are
rejected rather than ignored.

These are shared-contract decisions: implementation escalation is
`CROSS_PLATFORM_CHANGE_REQUIRED`.

## 5. Batch B — Versioned execution specification
Keep the internal typed specification out of the Browser request protocol and
reuse `archive_job_requests`. B1 adds explicit version dispatch: v1 decodes the
historical `ArchiveTweetRequest`; unknown versions fail with a stable explicit
error before creating a Sidecar process. B2 stores a v2 envelope containing the
request and the confirmed output settings. The DB row version is authoritative.
Do not rewrite v1 rows.

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

## 6. Batch C — Filename rendering and recoverable path mapping

Keep `Tweets/<tweet_id>` fixed. A template produces media-relative paths only;
`/` may denote an archive-internal subdirectory if the validated design permits
it. One Rust renderer/validator must serve preview and actual output. Do not
reimplement the naming algorithm in JavaScript. Custom text and tag-editor modes
must preserve templates losslessly or leave the user in custom mode with an
explicit explanation. Keyboard-accessible tag reordering is required; drag/drop
is optional.

Before changing files, compute and validate the complete mapping (safe relative
paths, platform-reserved names, lengths, extensions, duplicates, and conflicts),
then persist a recovery record. Renames must be idempotently recoverable across
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

## 7. Batch D — User metadata and internal recovery facts

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

## 8. Batch E — Downloader arguments

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

Trace and test both paths end-to-end:

```text
execution spec -> explicit Sidecar config -> gallery-dl argv
execution spec -> Rust aria2 command construction -> aria2 argv
```

Reject options that replace application-controlled input/output/protocol,
disable required metadata, execute hooks/commands, or bypass shared proxy policy;
check indirect config-file routes too. Ordinary unknown tool flags may be
reported by the tool, but must not be silently dropped. Do not log full argv.
Config export excludes the entire custom-argument arrays by default; any explicit
include flow requires a separate warning/confirmation. UI accepts JSON arrays
only; no shell tokenization, escape processing, pipes, or redirection semantics.

Acceptance covers quoting, escaping, Unicode, ordering, protected/indirect
overrides, inactive-engine save/apply behavior, cancellation/failure/timeout,
and redaction. Windows child-process semantics remain a Windows-owned check.

Escalation: `CROSS_PLATFORM_CHANGE_REQUIRED` if Sidecar/config protocol or shared
execution semantics change; use `CROSS_PLATFORM_REVIEW_REQUIRED` only for a small
implementation preserving an existing contract.

## 9. Batch F — Theme, configuration exchange, and record exchange

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

## 10. Ownership, evidence, and progress

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

The 2026-10-09 continuation committed the immutable v2 manifest phase-machine
repair as `b9ac571448c9e093c1639ca0586718dc9dd7bcc4` after rerunning Storage,
Desktop and Download module suites, formatting, diff check and docs audit. This
fixes C1.1's representation/compiler issue only; the v2 journal APIs still have no
production caller. C1.2/C1.3/C1.4 are not accepted: production v2 remains
fail-closed until two-phase file replay, atomic Tweet/media/Job/journal/Telegram
intent finalization, attempt fencing, COMMITTED filesystem+DB verification and a
service restart crash matrix are integrated. The prior E typed-args prototype was
removed after protocol compilation failed; do not retry it or count it as
implementation. The current aria2 parser conflicts with protected-options policy;
typed task persistence and gallery-dl/aria2 argv wiring remain absent. F remains
unimplemented. These are shared development items, not Windows blockers.

## 11. Progress

| Batch | Implementation | Verification / next action |
|---|---|---|
| A — page split | Implemented at `d406099` | Linux evidence recorded at `57edd18`; Windows fresh-artifact GUI pending |
| B1 — v1 compatibility / version dispatch | Implemented | v1 compatibility, v2 envelope, unknown-version and identity checks have targeted coverage; full module tests pass |
| B2 — v2 output snapshot | Implemented; Linux verification passed, Windows pending | Typed settings read/save commands and Download Config UI; one-off and batch submissions capture snapshots. Verify active reuse, retry/recovery and restart against immutable snapshots on Windows. |
| D0 — InternalV1 recovery | IMPLEMENTED; Linux safety acceptance complete | Existing InternalV1 recovery behavior and recorded crash-boundary coverage remain in place. New rename-capable v2 work is separate and incomplete; do not treat D0 evidence as proof of v2 rename recovery. Windows NTFS interruption evidence remains `NOT_RUN` (`WQ-PLAN-C-D0-01`). |
| C1.1 — durable journal | COMPLETED; Linux Storage module tests PASS on `b9ac571` | `transition_archive_recovery_v2` reads the immutable plan from `manifest_json` and advances only `rename_progress`/`phase`; creation retains migration 0017's default. This phase-machine repair is unit-tested, not production integration-verified. |
| C1.2/C1.3/C1.4 — naming/recovery | IN PROGRESS; v2 production gate intentionally closed | Shared renderer and preview API plus v2 schema/journal/file primitives exist. Production still rejects v2 commit fail-closed: actual startup replay, atomic archive-row/Telegram-intent finalization, COMMITTED directory+DB verification, attempt fencing and service crash matrix are not complete. The v2 journal API (`create_archive_recovery_manifest_v2`, `archive_recovery_manifest_v2`, `transition_archive_recovery_v2`) remains library-only with no production caller; its phase machine is unit-tested but not integration-verified. D1 remains inactive. Windows path/NTFS remains Windows-owner validation. |
| D1 — metadata outputs | BLOCKED pending safe C1 recovery gate | JSON/TXT switches remain stored/snapshotted but non-operative. v2 manifest carries the switches, but production must not omit either export until recovery dispatcher and final-path DB/Telegram consistency are implemented and tested. |
| E — tool arguments | IN PROGRESS; `CROSS_PLATFORM_CHANGE_REQUIRED`; not acceptance-ready | `downloader_args.rs` has an aria2 parser prototype, but its allowlist includes proxy and output options that conflict with the protected-options contract. A regression test records that the current prototype accepts these forbidden overrides; it documents the gap and is not an E acceptance test. The 2026-10-09 continuation shipped no E behavior. Reconcile/version Rust/Python shared option fixtures first; task-spec persistence, gallery-dl validation, UI and actual dual-backend execution wiring remain absent. |
| F — theme/config/record exchange | Planned; contracts clarified, not implemented | Separate versioned config JSON and separate download/archive record files; record bundle import accepts ZIP/7z; warnings, preview, idempotent merge and hash conflict rejection are required. Other compression formats deferred. |
| G/H — integration/Windows | PLANNED | Full Linux workspace regression and formal handoff remain; Windows native acceptance requires Windows Owner and fresh artifact |

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
