# Current Platform Handoff

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
