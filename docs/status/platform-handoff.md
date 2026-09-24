# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: resolve Windows-reported shared account-batch/task lifecycle findings without taking over Windows Extension/Native Host implementation.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Cross-platform Owner completed shared work; next owner is Windows Platform Owner.
- Current state: `READY_FOR_WINDOWS` (formal handoff record commit includes the complete Linux reconciliation)

## Revisions

- Cross-platform input revision: `89258c52100113a6a1cdfccf25f16ae915a45668`
- Cross-platform handoff revision: this handoff record commit (contains the complete Linux reconciliation)
- Windows input revision: `89258c52100113a6a1cdfccf25f16ae915a45668`
- Windows implementation revision: not applicable; Extension/Native Host reconnect work remains owned by Windows Platform Owner
- Windows validation revision: prior manual evidence is bound to `dac0a153d03fa174c600435c87afabf476aded34`; affected outcomes require revalidation against this handoff

## Cross-platform Work Completed

- Reconciled the Windows manual findings at `dac0a15`: Dashboard/Job and batch projections now refresh from durable SQLite state without manual refresh; job failure codes and redacted messages are visible.
- Account discovery now streams and persists each candidate before completion, uses an isolated random temporary directory, and honors the configured discovery timeout.
- Production aria2 now creates a fresh 256-bit RPC secret and an available loopback port per supervisor; no user-provided `XARCHIVE_ARIA2_RPC_SECRET` is required.
- Pause/cancel transitions batch, discovery, and pending candidates atomically; cancellation registry generation guards prevent stale workers from deleting or sharing a newer token. Resume/retry reject terminal or still-stopping batches and compensate failed worker startup.
- Archive context reconstruction preserves unified network settings; Job failure messages are redacted before SQLite persistence and again before frontend projection.
- Storage migration `0006` adds `PAUSED` discovery state while preserving v5 batch/candidate rows, indexes, and foreign keys.

## Windows Work Completed

- Windows-target Rust tests passed for the account batch affected crates: 219 tests across Core, Desktop, Download, Transfer Driver, Protocol, Sidecar Supervisor, Storage, and Telegram.
- Native Host Windows-target tests passed 8/8. Sidecar pytest passed 32/32; Desktop Node passed 92/92; Extension Node passed 21/21; Vite production build and all three modified schema JSON parses passed.
- Current-source PyInstaller worker was built in an isolated validation directory. Frozen worker `--help`, protocol-v2 `hello`/`ready` including `account_discovery`, unknown-field `INVALID_COMMAND`, and clean shutdown passed.
- Isolated Windows Tauri release build and Native Host release build passed. Full and Core portable packages were assembled in new validation-artifact directories. Full manifest, required worker/runtime/gallery-dl/Extension/Native Host files, Extension ID, and host manifest executable path/origin passed static checks.
- No Windows production-code change was made in the automated validation batch.
- Manual Full-package checks on 2026-09-24 confirmed clean app exit, Sidecar start/stop and status display, account-batch page rendering, Chinese-path handling, and initial Edge Extension/Native Host connection.
- Manual failures are recorded separately: Extension task appears only after Dashboard refresh and then fails; download still fails after manually starting aria2; Extension does not reconnect after app restart despite refresh/repair/unregister/re-repair; account discovery remains at zero candidates/pending and displays `RUNNING` after pause for about 30 seconds, then the user cancels it.
- The manual report fully redacts account names and Tweet IDs. No successful download, archive completion, or media-integrity result is claimed.

## Windows Work Remaining

- Revalidate `MANUAL-WIN-BATCH-REVAL-01` through `05` against this handoff: external task visibility, early candidate persistence, pause/resume/cancel generation safety, automatic aria2 RPC setup, and v5→v6 migration.
- `MANUAL-WIN-BATCH-REVAL-06`: diagnose and retest Edge Extension/Native Host reconnection after Desktop restart, including HKCU registration, manifest, Named Pipe, Service Worker and process evidence. This remains Windows-specific implementation/validation.
- P1-B real gallery-dl samples and P3-E controlled real-account multi-page/authentication/SHA-256 acceptance remain `NOT RUN` until suitable external inputs are available.
- `MANUAL-WIN-BATCH-05` long path/cross-volume/file-lock/abnormal-exit/signing/release checks and the existing E5–E7/WebView2 queue remain open according to `../validation/windows-queue.md`.
- No `WINDOWS_BLOCKING`: Windows work can proceed from this handoff and is not required to close the Linux batch.

## Windows Work Required

- Validate the new account archive page in the real Tauri/WebView2 window.
- Run the new batch flow against a controlled account and packaged/real gallery-dl/aria2 binaries.
- Verify pause during discovery, resume, cancellation, retry, restart recovery, and SHA-256 media completeness.
- Verify Windows packaged worker, aria2 transfer, signed URL refresh, staging/commit, file locks and process cleanup.
- Verify browser credentials/Extension/Native Host/Registry/Windows filesystem behavior where affected by the new flow.

## Expected Behavior

- External Browser/Native Host submissions appear in Desktop without a manual refresh; durable failures show a code and redacted message.
- Discovery candidates are visible in SQLite before gallery-dl/discovery completion; a slow account does not produce a false zero-candidate state.
- Production archive startup does not require an aria2 RPC secret environment variable. The secret is process-local and bound to loopback.
- Pause makes both batch and discovery `PAUSED` durably; cancel stops pending discovery/dispatch but does not kill submitted archive Jobs; resume cannot overlap an old worker.
- A v5 database upgrades to v6 without losing batch/candidate rows or foreign-key integrity.
- Prior Windows manual FAIL/NOT RUN evidence remains historical; none is promoted to PASS without revalidation against this handoff.

## Validation Required

- Linux: affected module validation only — Download/Storage, Desktop Rust, Sidecar, Desktop Node and Vite build. Full workspace regression was intentionally skipped because the diff is confined to those modules and does not change protocol/schema.
- Windows: run `MANUAL-WIN-BATCH-REVAL-01` through `06` from `../validation/windows-queue.md`, then continue the existing E5–E7, WebView2, filesystem and release queue.

## Risks and Deferred Items

- P1-B real gallery-dl output samples and P3-E real-account multi-page/authentication/SHA-256 acceptance are `NOT RUN` on Linux because they require controlled external samples/credentials.
- Successful Edge/Chrome account-task integration, native-host reconnect after application restart, controlled-account acceptance, filesystem fault cases, signing, and upload gate remain failed, unverified, or `NOT RUN` as detailed in the queue.
- Do not treat synthetic fixtures, Linux Unix transport loopback or Vite build as Windows acceptance.

## Relevant Tests

- Linux: `xarchive-download` 22 unit + 7 integration; `xarchive-storage` 36; Desktop Rust 104; Sidecar pytest 33; Desktop Node 93; Vite build; fmt and diff check. Focused evidence covers fresh RPC secrets, v5→v6 row/FK preservation, candidate streaming before completion and before `discovery_completed`, atomic pause/cancel, worker generation isolation, and two-layer error redaction.
- Windows: prior automated and manual evidence remains in `../validation/windows-validation-history.md`; run the new revalidation items and record the actual handoff revision.

## Manual Windows Validation Queue

Run `MANUAL-WIN-BATCH-REVAL-01` through `06` in `../validation/windows-queue.md` against the exact pushed handoff revision, then continue the existing E5–E7/Full/WDIO/WebView2/filesystem/release items.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: resolved by this Linux batch; no shared contract/protocol/schema owner action remains.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none outstanding.
- `WINDOWS_BLOCKING`: none.

## Next Owner

- Windows Platform Owner: fetch the pushed handoff revision, confirm a clean canonical Windows worktree, run the revalidation queue, diagnose Extension/Native Host restart reconnect within the Windows boundary, and record PASS/FAIL/NOT_RUN/BLOCKED against the exact revision.
