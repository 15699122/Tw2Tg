# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: reconcile the latest Windows findings and close the shared executor/discovery contract before the next Windows batch.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Cross-platform Owner (Linux) for shared implementation; Windows Platform Owner retains Extension/Native Host reconnect and Windows runtime work.
- Current state: `READY_FOR_WINDOWS` (shared code complete; Windows runtime revalidation queued)

## Revisions

- Cross-platform input revision: `553e3788467041ef43ac5657c969064ce45d016c`
- Cross-platform handoff revision: this handoff record commit (contains the complete Linux batch)
- Windows input revision: `553e3788467041ef43ac5657c969064ce45d016c`
- Windows implementation revision: `e36705d` (WDIO test-harness configuration only; no application runtime/business-code change)
- Windows validation revision: `e36705d`; prior evidence remains bound to the Windows validation record in `../validation/windows-validation-history.md`

## Cross-platform Work Completed

- Reconciled the Windows manual findings at `dac0a15`: Dashboard/Job and batch projections refresh from durable SQLite state; job failure codes and redacted messages are visible.
- Account discovery streams and persists candidates before completion, uses an isolated random temporary directory, and honors configured discovery timeout.
- Production aria2 uses a fresh 256-bit RPC secret and available loopback port per supervisor; user-provided `XARCHIVE_ARIA2_RPC_SECRET` is no longer required.
- Pause/cancel transitions are atomic; cancellation registry generation guards prevent stale workers from deleting or sharing a newer token. Resume/retry reject terminal/stopping batches and compensate failed worker startup.
- Archive context reconstruction preserves unified network settings; Job failure messages are redacted before SQLite persistence and again before frontend projection.
- Storage migration `0006` adds `PAUSED` discovery state while preserving v5 rows, indexes, and foreign keys.
- Executor replacement now restarts the Unix transport on the new service generation and preserves portable gallery-dl/network/discovery arguments.
- gallery-dl extraction/discovery now uses official `--dump-json` + `output.jsonl=true`, parses `Message.Directory=2` and `Message.Url=3`, handles real author dictionaries, `content`, `reply_id`, and media URLs, with info.json fallback.

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

- Re-run `MANUAL-WIN-BATCH-REVAL-01` through `06` against this exact handoff. The previous `EXECUTOR_UNAVAILABLE`, zero-candidate discovery, and Extension reconnect results are historical evidence from older artifacts and are not current-source results.
- Execute the real account-batch flow with controlled credentials, packaged gallery-dl/aria2, and current v5 database copy; retain exact artifact hash, screenshots, logs, SQLite counts and integrity evidence.
- Keep Extension/Native Host restart reconnect, E5–E7, WebView2, filesystem fault/recovery, signing and release items in the Windows queue. These are not shared Linux blockers.

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

- Linux: affected module validation only — `cargo fmt --all -- --check`, `git diff --check`, targeted Desktop executor replacement/config tests, Sidecar `compileall` and `pytest` 35/35. Full workspace regression was intentionally skipped because the diff changes only shared runtime/executor and Sidecar adapter behavior; no protocol/schema change.
- Windows: re-run affected manual items `MANUAL-WIN-BATCH-REVAL-01` through `06` against the exact pushed handoff. Reuse prior PASS only when source, dependencies, contracts and package inputs are unchanged; otherwise record `REVALIDATION_REQUIRED`. Continue E5–E7, WebView2, filesystem and release checks.


## Risks and Deferred Items

- P1-B real gallery-dl output samples and P3-E real-account multi-page/authentication/SHA-256 acceptance are `NOT RUN` on Linux because they require controlled external samples/credentials.
- Successful Edge/Chrome account-task integration, native-host reconnect after application restart, controlled-account acceptance, filesystem fault cases, signing, and upload gate remain failed, unverified, or `NOT RUN` as detailed in the queue.
- Do not treat synthetic fixtures, Linux Unix transport loopback or Vite build as Windows acceptance.

## Relevant Tests

- Linux: `cargo fmt --all -- --check`, `git diff --check`, targeted Desktop executor replacement/config tests, and Sidecar `compileall` + `pytest` 35/35 PASS. JSONL fixtures cover real gallery-dl message shape, author dict, `content`, `reply_id`, media URL, and discovery candidate identity. Full workspace suite was not rerun because this batch changes only shared runtime/executor and Sidecar adapter behavior; no protocol/schema change.
- Windows: run the current-source revalidation items against the exact pushed handoff. Real gallery-dl account discovery, Extension archive submission/automatic visibility, application-managed aria2 transfer, v5 database copy migration, Extension/Native Host restart reconnect, filesystem fault/recovery, signing and release remain Windows queue items; prior PASS/FAIL/NOT_RUN evidence stays bound to its original revision.

## Manual Windows Validation Queue

Run the current-source `MANUAL-WIN-BATCH-REVAL-01` through `06` items in `../validation/windows-queue.md` against the exact pushed handoff revision, then continue the existing E5–E7/Full/WDIO/WebView2/filesystem/release items.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: none; the shared executor lifecycle and account-discovery findings from the prior Windows report are handled in this Linux batch. Real gallery-dl/Windows acceptance remains validation, not an open shared-code change.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none outstanding.
- `WINDOWS_BLOCKING`: none.

## Next Owner

- Cross-platform Owner (Linux): no further shared implementation remains in this batch; await Windows revalidation evidence.
- Windows Platform Owner: fetch the pushed handoff, run the current-source revalidation queue, and record exact revision-bound PASS/FAIL/BLOCKED/NOT_RUN results. Extension/Native Host reconnect remains Windows-owned diagnosis.

## 2026-09-24 Manual Result Update

- User-reported current-build Extension test: load and page action `PASS`; task visibility in about 0.5 seconds and duplicate suppression `PASS`; executor `FAIL` with `EXECUTOR_UNAVAILABLE: job executor is closed`.
- Account discovery: `FAIL` for two tested accounts (no candidates/tasks); pause/cancel UI controls `PASS` in the observed run. Atomic worker-stop/resume/recovery semantics remain unverified.
- v5-to-v6 package migration: `PASS` by user report; migration hashes/counts/FK output were not supplied.
- Settings refresh did not reconnect Edge Extension: `FAIL`; additional reconnect test deferred until Extension refactor.
- Staging/path/fault/recovery: `NOT RUN`; “expected PASS” is recorded only as user expectation.
- Artifact source revision/hash was not included with this report. Account, Tweet, batch and URL identifiers remain fully redacted. Details are in `../validation/windows-queue.md` and `../validation/windows-validation-history.md`.
