# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: Windows validation batch for the P1/P2/P3 account batch workflow and current E5–E7/Full-package integration.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Windows Platform Owner
- Current state: `WINDOWS_VERIFICATION_PENDING` (automated Windows checks passed; GUI, live account, registry/browser integration, and filesystem fault scenarios remain queued or blocked)

## Revisions

- Cross-platform input revision: `dac0a153d03fa174c600435c87afabf476aded34`
- Cross-platform handoff revision: `dac0a153d03fa174c600435c87afabf476aded34`
- Windows input revision: `dac0a153d03fa174c600435c87afabf476aded34`
- Windows implementation revision: not applicable; this batch required no Windows production-code changes
- Windows validation revision: `dac0a153d03fa174c600435c87afabf476aded34`

## Cross-platform Work Completed

- P1-A/C/D: extraction relationship/author fields, stable placeholder merge, README status normalization.
- P2-A/B/C: unified network settings, redaction, durable pause/resume/cancel/retry semantics, media completeness policy.
- P3-A/B/C/D: discovery protocol/Sidecar flow, SQLite batch schema, idempotent candidates, filters/completeness skip, bounded executor dispatch, restart reconciliation, Tauri commands, account archive UI.
- Sidecar `discover` uses a private worker; submitted archive jobs continue when a batch is paused or cancelled.

## Windows Work Completed

- Windows-target Rust tests passed for the account batch affected crates: 219 tests across Core, Desktop, Download, Transfer Driver, Protocol, Sidecar Supervisor, Storage, and Telegram.
- Native Host Windows-target tests passed 8/8. Sidecar pytest passed 32/32; Desktop Node passed 92/92; Extension Node passed 21/21; Vite production build and all three modified schema JSON parses passed.
- Current-source PyInstaller worker was built in an isolated validation directory. Frozen worker `--help`, protocol-v2 `hello`/`ready` including `account_discovery`, unknown-field `INVALID_COMMAND`, and clean shutdown passed.
- Isolated Windows Tauri release build and Native Host release build passed. Full and Core portable packages were assembled in new validation-artifact directories. Full manifest, required worker/runtime/gallery-dl/Extension/Native Host files, Extension ID, and host manifest executable path/origin passed static checks.
- No Windows-owned product failure or shared-contract defect was found. No Windows production code change was required.

## Windows Work Remaining

- `MANUAL-WIN-BATCH-01`, `MANUAL-WIN-BATCH-02`, and `MANUAL-WIN-E7-01` remain `BLOCKED` by `COMPUTER_USE_UNAVAILABLE`: the available Computer Use surface enumerated Edge but no native app and exposed no `launch_app` method, so the packaged GUI could not be opened or inspected.
- `MANUAL-WIN-BATCH-03` and `MANUAL-WIN-BATCH-04` remain `BLOCKED` pending a controlled account/browser session and real credentials; no account job was started.
- `MANUAL-WIN-BATCH-05` is partially verified: Full/Core assembly and static package boundary checks passed; actual Windows path/ACL/lock/reparse/fault recovery, GUI release gate, signing, and upload gate remain `NOT RUN` or `BLOCKED` pending their prerequisites.
- Existing E5/E6 live pipe/registry and Full GUI items remain in the queue. Windows Named Pipe loopback unit coverage is PASS but does not substitute for live GUI/ACL/registry validation.

## Windows Work Required

- Validate the new account archive page in the real Tauri/WebView2 window.
- Run the new batch flow against a controlled account and packaged/real gallery-dl/aria2 binaries.
- Verify pause during discovery, resume, cancellation, retry, restart recovery, and SHA-256 media completeness.
- Verify Windows packaged worker, aria2 transfer, signed URL refresh, staging/commit, file locks and process cleanup.
- Verify browser credentials/Extension/Native Host/Registry/Windows filesystem behavior where affected by the new flow.

## Expected Behavior

- The Linux shared implementation at `dac0a153` is complete and passed the Windows automated validation listed in the history record.
- `WINDOWS_VERIFICATION_PENDING` remains because GUI/native/browser/account/filesystem validation is incomplete.
- A `BLOCKED` check records an automation or environment prerequisite, not a product failure.

## Validation Required

- Windows automated validation against `dac0a153d03fa174c600435c87afabf476aded34` is recorded in `../validation/windows-validation-history.md`.
- Remaining Windows GUI, browser/registry, controlled-account, filesystem-fault, and release checks are listed with exact reasons in `../validation/windows-queue.md`.
- Full regression was not repeated; affected Windows-target module tests, package build, worker probe, and package assembly were run. Unrelated full-workspace behavior remains outside this diff's necessary validation scope.

## Risks and Deferred Items

- P1-B real gallery-dl output samples and P3-E real-account multi-page/authentication/SHA-256 acceptance are `NOT RUN` on Linux because they require controlled external samples/credentials.
- Windows GUI/WebView2/driver pairing, live registry/Native Host, real Edge/Chrome integration, controlled account acceptance, filesystem fault cases, signing, and upload gate remain `BLOCKED` or `NOT RUN` as detailed in the queue.
- Do not treat synthetic fixtures, Linux Unix transport loopback or Vite build as Windows acceptance.

## Relevant Tests

- Windows: affected Rust tests 219/219; Native Host 8/8; Sidecar pytest 32/32; Desktop Node 92/92; Extension Node 21/21; Vite build, schema parsing, worker protocol probe, isolated Tauri/Native Host builds, and Full/Core package checks PASS. Full regression was not run.
- Windows manual outcomes and prerequisites: see `../validation/windows-queue.md` and the 2026-09-24 history entry.

## Manual Windows Validation Queue

Run the remaining `BLOCKED`/`NOT RUN` items in `../validation/windows-queue.md`, including `MANUAL-WIN-BATCH-01` through `MANUAL-WIN-BATCH-05` and live E5–E7/Full checks.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: none found during Windows validation.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none; no shared implementation was modified in this Windows batch.

## Next Owner

- Windows Platform Owner: continue the remaining manual Windows validation when native GUI automation/manual access and controlled account prerequisites are available. Cross-platform ownership does not return to Linux because no shared follow-up was found.
