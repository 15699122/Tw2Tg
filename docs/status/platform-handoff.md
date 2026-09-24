# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: triage confirmed Windows manual-validation failures in P1/P2/P3 account batch and Extension task lifecycle; retain Windows reconnect checks for follow-up.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Cross-platform Owner for shared workflow triage; Windows Platform Owner retains Windows-specific reconnect validation.
- Current state: `CROSS_PLATFORM_CHANGE_REQUIRED` (diagnosis required; manual evidence shows delayed task visibility and stalled account discovery, but does not establish root cause or required contract change)

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
- No Windows production-code change was made in the automated validation batch.
- Manual Full-package checks on 2026-09-24 confirmed clean app exit, Sidecar start/stop and status display, account-batch page rendering, Chinese-path handling, and initial Edge Extension/Native Host connection.
- Manual failures are recorded separately: Extension task appears only after Dashboard refresh and then fails; download still fails after manually starting aria2; Extension does not reconnect after app restart despite refresh/repair/unregister/re-repair; account discovery remains at zero candidates/pending and displays `RUNNING` after pause for about 30 seconds, then the user cancels it.
- The manual report fully redacts account names and Tweet IDs. No successful download, archive completion, or media-integrity result is claimed.

## Windows Work Remaining

- Cross-platform Owner: inspect delayed Extension task/status propagation and account discovery/candidate persistence; clarify pause semantics and add a focused reproduction before changing shared code or contract.
- Windows Platform Owner: investigate Edge Extension/Native Host reconnection after Desktop restart against the next shared fix revision; live registry values, Native Messaging pipe requests, and cross-user ACL remain unverified.
- `MANUAL-WIN-BATCH-02`: pause observation is recorded; resume, retry, and restart recovery are `NOT RUN`.
- `MANUAL-WIN-BATCH-03`: discovery/download did not complete; file count, content, size, and SHA-256 checks are `NOT RUN` because no successful download was produced.
- `MANUAL-WIN-BATCH-04`: initial Edge load/connection passed; task propagation/download and post-restart reconnect failed. Credential redaction and explicit registry-state inspection are `NOT RUN`.
- `MANUAL-WIN-BATCH-05`: Chinese path passed; long path, cross-volume, file lock, abnormal-exit, staging cleanup, signing, and release gate are `NOT RUN`. Prior Full/Core assembly and static package-boundary checks remain PASS.
- `MANUAL-WIN-E5-01` live pipe/ACL remains `NOT RUN`; `MANUAL-WIN-E6-01` repair/reconnect failed but registry-path and moved-package checks remain `NOT RUN`; `MANUAL-WIN-E7-01` initial connection passed and reconnect after restart failed; `MANUAL-WIN-FULL-01` app lifecycle and Sidecar start/stop passed.

## Windows Work Required

- Validate the new account archive page in the real Tauri/WebView2 window.
- Run the new batch flow against a controlled account and packaged/real gallery-dl/aria2 binaries.
- Verify pause during discovery, resume, cancellation, retry, restart recovery, and SHA-256 media completeness.
- Verify Windows packaged worker, aria2 transfer, signed URL refresh, staging/commit, file locks and process cleanup.
- Verify browser credentials/Extension/Native Host/Registry/Windows filesystem behavior where affected by the new flow.

## Expected Behavior

- The Linux shared implementation at `dac0a153` is complete and passed the Windows automated validation listed in the history record.
- Automated validation passed on `dac0a153`; manual account-task and reconnect failures are separate and are not explained by those automated checks.
- The account batch could not produce candidates/downloads in this manual run. Pause behavior beyond the recorded 30-second observation still needs its intended semantics clarified.
- A `NOT RUN` check means no evidence was collected; it is not a PASS or FAIL.

## Validation Required

- Windows automated validation against `dac0a153d03fa174c600435c87afabf476aded34` is recorded in `../validation/windows-validation-history.md`.
- Remaining Windows GUI, browser/registry, controlled-account, filesystem-fault, and release checks are listed with exact reasons in `../validation/windows-queue.md`.
- Full regression was not repeated; affected Windows-target module tests, package build, worker probe, and package assembly were run. Unrelated full-workspace behavior remains outside this diff's necessary validation scope.

## Risks and Deferred Items

- P1-B real gallery-dl output samples and P3-E real-account multi-page/authentication/SHA-256 acceptance are `NOT RUN` on Linux because they require controlled external samples/credentials.
- Successful Edge/Chrome account-task integration, native-host reconnect after application restart, controlled-account acceptance, filesystem fault cases, signing, and upload gate remain failed, unverified, or `NOT RUN` as detailed in the queue.
- Do not treat synthetic fixtures, Linux Unix transport loopback or Vite build as Windows acceptance.

## Relevant Tests

- Windows: affected Rust tests 219/219; Native Host 8/8; Sidecar pytest 32/32; Desktop Node 92/92; Extension Node 21/21; Vite build, schema parsing, worker protocol probe, isolated Tauri/Native Host builds, and Full/Core package checks PASS. Full regression was not run.
- Windows manual outcomes and prerequisites: see `../validation/windows-queue.md` and the 2026-09-24 history entry.

## Manual Windows Validation Queue

Run the remaining `FAIL`/`NOT RUN` items in `../validation/windows-queue.md` after the shared task/discovery triage; then repeat Windows Extension/Native Host reconnect checks against the resulting revision.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: triage shared account discovery/candidate progress and Extension task-status propagation. Evidence: candidates and pending stayed at zero; discovery remained `RUNNING`; Extension tasks appeared only after Dashboard refresh and then failed. Root cause and whether a contract change is required are not yet established.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none; no shared code was modified during Windows validation.

## Next Owner

- Cross-platform Owner: diagnose shared account discovery/candidate progress and Extension task-status propagation at the reported revision, clarify pause semantics, and provide a fix or evidence-based disposition.
- Windows Platform Owner: retain the post-restart Edge Extension/Native Host reconnect and remaining Windows-only queue items for validation after shared triage.
