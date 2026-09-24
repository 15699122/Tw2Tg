# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: Windows revalidation of the account-batch/task lifecycle handoff and current-source Full package; retain Windows-owned Native Host reconnect diagnosis.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Windows Platform Owner.
- Current state: `WINDOWS_VERIFICATION_PENDING`; automated results are recorded and manual GUI/account acceptance remains open.

## Revisions

- Cross-platform input revision: `7ed02b5e94e17171e26ae75000b555932e166216`
- Cross-platform handoff revision: `7ed02b5e94e17171e26ae75000b555932e166216`
- Windows input revision: `7ed02b5e94e17171e26ae75000b555932e166216`
- Windows implementation revision: `e36705d` (WDIO test-harness configuration only; no application runtime/business-code change)
- Windows validation revision: `e36705d`; validation evidence is recorded in `../validation/windows-validation-history.md`

## Cross-platform Work Completed

- Reconciled the Windows manual findings at `dac0a15`: Dashboard/Job and batch projections now refresh from durable SQLite state without manual refresh; job failure codes and redacted messages are visible.
- Account discovery now streams and persists each candidate before completion, uses an isolated random temporary directory, and honors the configured discovery timeout.
- Production aria2 now creates a fresh 256-bit RPC secret and an available loopback port per supervisor; no user-provided `XARCHIVE_ARIA2_RPC_SECRET` is required.
- Pause/cancel transitions batch, discovery, and pending candidates atomically; cancellation registry generation guards prevent stale workers from deleting or sharing a newer token. Resume/retry reject terminal or still-stopping batches and compensate failed worker startup.
- Archive context reconstruction preserves unified network settings; Job failure messages are redacted before SQLite persistence and again before frontend projection.
- Storage migration `0006` adds `PAUSED` discovery state while preserving v5 batch/candidate rows, indexes, and foreign keys.

## Windows Work Completed

- Affected Windows-target Rust tests passed: 171 tests across Desktop, Download/Transfer Driver, and Storage; Native Host Windows tests passed 8/8.
- Sidecar discovery tests passed 8/8; Desktop targeted UI wiring tests passed 20/20; Extension tests passed 21/21; Vite built 52 modules.
- Current-source PyInstaller worker, isolated Windows Tauri release app and Native Host release executable built. Frozen worker `--help`, protocol-v2 `hello`/`ready` with `account_discovery`, unknown-field `INVALID_COMMAND`, and clean shutdown passed.
- Current-source Full package assembled at `validation-artifacts/windows-batch-revalidation-7ed02b5/full-package`. Static manifest, bundled worker/python312.dll/gallery-dl/Extension/Native Host, extension ID and matching Native Host allowed origin passed.
- Fixed-runtime dashboard WDIO initially failed in the restricted Node environment (`uv_os_get_passwd returned ENOMEM`). Running in the normal Windows process environment exposed a second issue: the WDIO config did not forward the pinned WebView2 folder or explicit tauri-driver path to `@wdio/tauri-service`. The config was fixed and the Full-package dashboard smoke then passed 3/3 on WebView2 `153.0.4234.48`, EdgeDriver `153.0.4234.46`, and tauri-driver `2.0.6`.
- Desktop full Node suite passed 93/93 in the normal Windows process environment. The earlier restricted-runner `ENOMEM`/child-process limitation is not a product failure.
- aria2 `1.37.0` from the local `aria2` directory passed Windows-target `xarchive-download` tests (22 unit + 7 integration) and a localhost JSON-RPC/HTTP Range fixture, including pause/resume, 2 MiB output, and SHA-256 verification. This does not establish an app-managed account archive transfer; the repo-local aria2 directory must be selected/saved in the app or be placed in the Full package's expected `sidecar\aria2` location.
- Computer Use native-app inventory was empty and its native launch API unavailable after a bounded retry. The targeted native dashboard was instead exercised through WDIO; real account/Extension workflow checks remain in the Manual Windows Validation Queue.
- Windows changes at `e36705d` are limited to WDIO test configuration and its regression assertions. Prior manual Full-package evidence remains bound to `dac0a153d03fa174c600435c87afabf476aded34`, not this source revision: clean app exit, Sidecar status/start/stop, account-batch page rendering, Chinese path and initial Edge connection passed; task visibility/download, post-restart Extension reconnect, and discovery/pause behavior failed there.
- Account names and Tweet IDs remain fully redacted. No successful download, archive completion, or media-integrity result is claimed for the current revision.

## Windows Work Remaining

- Complete `MANUAL-WIN-BATCH-REVAL-01` through `05` against this revision using a controlled account and package: Extension task visibility, early candidate persistence, pause/resume/cancel, app-managed aria2 transfer, and v5→v6 migration. The native dashboard smoke and local aria2 RPC fixture pass do not replace these GUI/integration cases.
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
- Windows: finish `MANUAL-WIN-BATCH-REVAL-01` through `06` from `../validation/windows-queue.md`, then continue the existing E5–E7, WebView2, filesystem and release queue. Native dashboard startup is now verified by WDIO; controlled-account workflows, Extension reconnect, filesystem, and release checks remain open.

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

- Windows Platform Owner: continue the open manual queue on implementation revision `e36705d`, using the fixed-runtime WDIO configuration; revalidate Extension task submission, account discovery/pause behavior, app-managed aria2 transfer, and Extension/Native Host restart reconnect with controlled inputs. No cross-platform follow-up was newly established in this batch.
