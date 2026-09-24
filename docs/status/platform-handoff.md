# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: P1/P2/P3 Linux cross-platform implementation and validation for the account batch workflow.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Cross-platform Owner (implementation complete; Windows validation pending)
- Current state: `READY_FOR_WINDOWS` (formal handoff record commit includes the complete Linux batch)

## Revisions

- Cross-platform input revision: `26f8c37ac995cea7fce6667fcdee2d2966e6f77b`
- Cross-platform handoff revision: this handoff record commit (contains all Linux batch changes)
- Windows input revision: not applicable for this Linux implementation batch
- Windows implementation revision: not applicable; no Windows implementation was performed
- Windows validation revision: not run

## Cross-platform Work Completed

- P1-A/C/D: extraction relationship/author fields, stable placeholder merge, README status normalization.
- P2-A/B/C: unified network settings, redaction, durable pause/resume/cancel/retry semantics, media completeness policy.
- P3-A/B/C/D: discovery protocol/Sidecar flow, SQLite batch schema, idempotent candidates, filters/completeness skip, bounded executor dispatch, restart reconciliation, Tauri commands, account archive UI.
- Sidecar `discover` uses a private worker; submitted archive jobs continue when a batch is paused or cancelled.

## Incorporated Windows Follow-up Evidence

- Remote Windows follow-up commit `bfa26ebae9820e73b3bacd646297e96b6acfcd15` was rebased under this Linux batch.
- `node --test desktop/test/wdio-tauri-service.test.mjs` passed 8/8 on input `26f8c37ac995cea7fce6667fcdee2d2966e6f77b`; the teardown item is closed as PASS.
- E5–E7 implementation/build/package/worker and Windows Named Pipe loopback evidence remains historical evidence for unchanged E5–E7 code. Because this Linux batch changes shared GUI, protocol, runtime, and executor modules, all affected E5–E7/Full outcomes require revalidation against the new handoff revision; no prior PASS is promoted to the new revision.
- GUI, live HKCU registry, browser Extension/Native Host connection, packaged Sidecar UI, and cross-user ACL checks remain `WINDOWS_BLOCKED` / `COMPUTER_USE_UNAVAILABLE`; no product failure was established.

## Windows Work Required

- Validate the new account archive page in the real Tauri/WebView2 window.
- Run the new batch flow against a controlled account and packaged/real gallery-dl/aria2 binaries.
- Verify pause during discovery, resume, cancellation, retry, restart recovery, and SHA-256 media completeness.
- Verify Windows packaged worker, aria2 transfer, signed URL refresh, staging/commit, file locks and process cleanup.
- Verify browser credentials/Extension/Native Host/Registry/Windows filesystem behavior where affected by the new flow.

## Expected Behavior

- Linux shared implementation is complete; Windows-specific results remain unclaimed.
- `WINDOWS_VERIFICATION_PENDING` means code exists but target validation is pending.
- `WINDOWS_BLOCKED` means target environment, GUI automation, account, credential or artifact prerequisite is unavailable.

## Validation Required

- Linux: `cargo fmt --all -- --check`, full Rust workspace tests, Sidecar pytest/compileall, Desktop Node tests and Vite check/build, Extension Node tests, schema parse, `git diff --check`.
- Windows: manual queue items in `../validation/windows-queue.md`; record actual Windows validation revision and per-item PASS/FAIL/BLOCKED.

## Risks and Deferred Items

- P1-B real gallery-dl output samples and P3-E real-account multi-page/authentication/SHA-256 acceptance are `NOT RUN` on Linux because they require controlled external samples/credentials.
- Windows GUI/WebView2/driver pairing, registry/native host, packaging, real Edge/Chrome and real X/Telegram acceptance remain `WINDOWS_VERIFICATION_PENDING` or `WINDOWS_BLOCKED`.
- Do not treat synthetic fixtures, Linux Unix transport loopback or Vite build as Windows acceptance.

## Relevant Tests

- Linux: Desktop Rust 100/100; Storage Rust 34/34; Desktop Node 92/92; Extension Node 21/21; Sidecar pytest 32/32; Vite check PASS; full Rust workspace and doc-tests PASS; JSON Schema parse and `git diff --check` PASS.
- Windows: use the exact manual steps and evidence fields in `../validation/windows-queue.md`.

## Manual Windows Validation Queue

Run the batch-specific and existing E5–E7/Full/WDIO items in `../validation/windows-queue.md`. The new batch items include `MANUAL-WIN-BATCH-01` through `MANUAL-WIN-BATCH-05`.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: none identified in this Linux batch.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none; Windows review is required only for platform integration and validation.

## Next Owner

- Windows Platform Owner: after a formal Git commit/push, fetch the handoff revision, confirm a clean Windows worktree, execute the manual queue, and record results against that exact revision.
