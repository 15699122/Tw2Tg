# Desktop Download Output and Settings Plan

Owner: Cross-platform Owner for shared contracts, Rust/Storage/Sidecar integration,
and shared UI; Windows Platform Owner for Windows-specific integration, native GUI,
packaging, and Windows validation.

Status: `IN_PROGRESS` — B1 and the initial B2 persistence/wiring are implemented
in the current uncommitted Linux working tree. Storage/Desktop module tests pass;
Windows acceptance and the remaining recovery, naming, downloader, theme, and
configuration-exchange batches are not complete. The existing page split is
implemented at handoff revision `57edd18`; Windows acceptance of that exact
revision remains pending.

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

Batch A is implemented for the narrow page relocation only. It is not evidence that
all intended screens, controls, fields, or Windows acceptance are complete.

## 4. Confirmed contract decisions

The following choices were confirmed by the user on 2026-10-06 and are the
implementation baseline. Screenshots remain interaction examples, not a visual
specification.

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

Provide separate gallery-dl and aria2 settings. Persist user text plus
backend-parsed/validated argv in the task spec; validate again immediately before
execution. Never invoke a shell. Document that quotes group arguments but pipes
and redirection have no shell meaning. Do not conflate tool argv with Sidecar
worker program/args.

Trace and test both paths end-to-end:

```text
execution spec -> explicit Sidecar config -> gallery-dl argv
execution spec -> Rust aria2 command construction -> aria2 argv
```

Reject options that replace application-controlled input/output/protocol,
disable required metadata, execute hooks/commands, or bypass shared proxy policy;
check indirect config-file routes too. Ordinary unknown tool flags may be
reported by the tool, but must not be silently dropped. Do not log full argv.
Config export excludes the entire custom-argument text by default; any explicit
include flow requires a separate warning/confirmation.

Acceptance covers quoting, escaping, Unicode, ordering, protected/indirect
overrides, inactive-engine save/apply behavior, cancellation/failure/timeout,
and redaction. Windows child-process semantics remain a Windows-owned check.

Escalation: `CROSS_PLATFORM_CHANGE_REQUIRED` if Sidecar/config protocol or shared
execution semantics change; use `CROSS_PLATFORM_REVIEW_REQUIRED` only for a small
implementation preserving an existing contract.

## 9. Batch F — Theme and local configuration exchange

Theme authority is persisted `system | light | dark`. Audit all pages, fields,
warning tokens, previews, labels, logs, and status badges. WebView theme evidence
does not prove native titlebar/dialog appearance.

Import/export uses a dedicated versioned local-file format, never a URL and never
blind replacement of arbitrary `config.yaml`. Preview changed/preserved/excluded
fields and restart requirements; strictly validate a candidate before writing;
save atomically and report success only after persistence. Do not force-restart.
With active tasks, offer cancel-import or wait; do not interrupt implicitly. A
failed validation/write must leave existing configuration intact and must not use
startup's fallback-to-default path as an import-success path. Do not expand shell
or filesystem permissions.

Acceptance covers round-trip, unknown/invalid schema, failure atomicity, active
task handling, explicit restart confirmation, theme token coverage, and Windows
native UI separately.

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

## 11. Progress

| Batch | Implementation | Verification / next action |
|---|---|---|
| A — page split | Implemented at `d406099` | Linux evidence recorded at `57edd18`; Windows fresh-artifact GUI pending |
| B1 — v1 compatibility / version dispatch | Implemented in current working tree | v1 compatibility, v2 envelope, unknown-version and identity checks have targeted coverage; full module tests pass |
| B2 — v2 output snapshot | Partially implemented in current working tree | One-off tasks persist v2; batch snapshot migration, typed Storage DTO, dispatcher inheritance and reuse checks are wired. Batch creation still supplies defaults because no configurable output-settings UI/request source exists. Validate startup/retry/recovery and complete that settings input path before claiming B2 complete. |
| D0 — internal recovery facts | Planned | Must precede rename and user output toggles |
| C1 — naming/recovery | Planned | Flat filenames; depends on B2 and D0 |
| D1 — metadata outputs | Planned | JSON/TXT switches only after D0/C1 recovery acceptance |
| E — tool arguments | Planned | Depends on B and verified Rust/Python call paths |
| F — theme/config exchange | Planned | Contract/design decision and implementation pending |
| G/H — integration/Windows | Planned | Per-batch handoff and artifact-bound validation |