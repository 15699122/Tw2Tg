# Current Platform Handoff

## Plan C1 non-Windows continuation — 2026-10-10 (current; WIP, not Windows handoff)

Current owner: Cross-platform Owner. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`; source HEAD before this
batch `32730527ff9a22ee900fae0f4a93536f2a1efda4`. This batch is being prepared as
a WIP commit for GitHub visibility; it is not a Windows handoff. The batch includes
shared changes in `crates/xarchive-workflow/src/coordination/unix.rs`,
`crates/xarchive-workflow/src/coordination.rs` and
`crates/xarchive-workflow/tests/coordination.rs` (process-shared local lock
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
  Atomic Job claim and full attempt-ownership/file-writer fencing remain open.
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
| `WQ-PLAN-C-D1-01` | Independent JSON/TXT export switches and recovery when exports are disabled | P1 / Windows Platform Owner | `BLOCKED` | D1 remains intentionally inactive until v2 startup recovery and final-path consistency are proven; Windows artifact required. |
| `WQ-PLAN-C-E-01` | Per-tool downloader args via persisted spec → Sidecar/gallery-dl argv and Rust aria2 argv | P1 / Windows Platform Owner | `NOT_RUN` | Shared parser unit coverage exists; production allowlist/wiring not implemented. |
| `WQ-PLAN-C-F-01` | Theme persistence and versioned configuration exchange | P1 / Windows Platform Owner | `NOT_RUN` | Configuration/theme implementation not ready. |
| `WQ-PLAN-C-F-02` | Download/archive record export/import; individual JSON, ZIP and 7z | P1 / Windows Platform Owner | `NOT_RUN` | Record exchange implementation not ready. |
| `WQ-PLAN-C-B1B2-01` | v1/v2 execution-spec compatibility, unknown schema rejection, task reuse/retry/restart snapshot | P1 / Windows Platform Owner | `NOT_RUN` | Exact-source fresh artifact required. |

#### Manual verification steps to be executed on Windows after handoff

1. **D0 recovery (implemented; Windows NOT_RUN):** use a disposable NTFS archive root and synthetic media. Interrupt the process at each documented journal/manifest write, staging-to-final rename, and database-finalization boundary. Restart the exact artifact; verify the manifest version/hash, Tweet/Job identity, every final relative media path/size/SHA-256 and Telegram-intent reference. Repeat missing, corrupt and unknown-version records; each must fail explicitly without guessing or falling back to exports.
2. **C1 mapping (BLOCKED):** enter synthetic username/Tweet/media fixtures and compare the Rust-generated preview with committed names. Include Unicode, Windows reserved names, case-insensitive duplicates, invalid/missing template fields, extension preservation and traversal attempts. Force a commit interruption and restart; verify no overwrite and identical final paths in disk, SQLite, metadata and any Telegram payload.
3. **D1 exports (BLOCKED):** on separate fresh jobs run JSON/TXT combinations on/on, on/off, off/on and off/off. Inspect archive files and confirm internal recovery remains readable and verified in every case. Interrupt and restart one exports-off job; verify no recovery dependence on `tweet.json` or `tweet.txt`. Confirm an existing legacy archive is not rewritten.
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
