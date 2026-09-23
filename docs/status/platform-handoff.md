# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: Linux reconciliation of the Windows E5–E7 round and §13 cross-platform review of the OS-gated shared wiring marked `CROSS_PLATFORM_REVIEW_REQUIRED`. No code changes in this batch.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Cross-platform Owner (review accepted) -> Windows Platform Owner (queued manual acceptance)
- Current state: `READY_FOR_WINDOWS` (shared review accepted; Windows manual verification pending)

## Revisions

- Cross-platform input revision: `0e35fde`
- Cross-platform handoff revision: this record commit (tree identical to `0e35fde` except this file; no code changes in this batch)
- Windows input revision: `bde6dc375bda68944b500714db8f9c39693bd06f`
- Windows implementation revision: `2dcae6c9a04175d6aa7e2478d421b497934a039e`
- Windows validation revision: `2dcae6c9a04175d6aa7e2478d421b497934a039e` (Windows docs commit: `0e35fde`)

## Cross-platform Review (§13)

Verdict: **ACCEPTED** (2026-09-23, Cross-platform Owner). No findings returned.

Reviewed scope: shared `desktop/src/main.jsx`, `desktop/src/pages/settings-page.jsx`, `desktop/src-tauri/src/commands.rs`, `desktop/src-tauri/src/runtime.rs`, `desktop/src-tauri/src/lib.rs`, `desktop/src-tauri/Cargo.toml`, `scripts/build-portable-windows.mjs`; Windows-only `windows_transport.rs` inspected for platform-logic leakage.

Findings:

- Non-Windows `extension_status_from_state` output is value-identical to the pre-batch behavior (`browser_connection: not_loaded|missing`, `native_host: missing|not_available`, same message strings); claim "non-Windows status behavior remains unchanged" verified.
- `ExtensionStatus.native_host` already existed before this round; no shared schema addition.
- `register_native_host` / `unregister_native_host` are registered on all platforms but return an explicit error off Windows; frontend actions are rendered only under `isWindows`, following the existing handler pattern.
- Runtime wiring is fully `cfg`-gated; the Unix transport path is untouched. New dependencies live under `[target.'cfg(windows)'.dependencies]`.
- Windows platform logic is confined to the `cfg(windows)` module; protocol crate/schema unchanged — no shared-contract impact, so no `CROSS_PLATFORM_CHANGE_REQUIRED`.

Linux validation for this review (minimal necessary; full suite not run — review-scope only): `cargo test -p xarchive-desktop --lib` 87 PASS (exit 0); `cargo test -p xarchive-protocol -p xarchive-native-host` 17+8 PASS; `cargo fmt --all --check` clean; `npm run check --workspace desktop` (vite build) PASS.

## Windows Implementation (reconciled from the Windows round)

- E5: added a Windows-only Named Pipe listener using the shared `xarchive_protocol::WINDOWS_PIPE_ENDPOINT` default and `XARCHIVE_PIPE_ENDPOINT` diagnostic override; owner-only protected DACL, concurrent connections via existing Native Messaging framing helpers and Desktop adapter, connection facts exposed to status.
- E6: current-user HKCU Edge/Chrome registration, repair after portable-root movement, unregister commands; preflights manifest identity and refuses unknown mappings.
- E7: Windows Native Host registration and transport facts wired into Extension status and Windows-only Settings actions.
- Full package builder accepts isolated `PORTABLE_APP_BINARY`, `PORTABLE_NATIVE_HOST_BINARY`, `PORTABLE_WORKER_DIRECTORY` inputs.
- Protocol/schema unchanged; shared frontend wiring reviewed and accepted above.

## Validation (reconciled from the Windows round)

PASS:
- `cargo fmt --all -- --check`.
- `cargo test -p xarchive-desktop --lib --target x86_64-pc-windows-msvc --offline`: 89 passed, including Windows Named Pipe loopback with two sequential framed requests and request IDs.
- Isolated Windows release build of Desktop and Native Host from current Rust source.
- PyInstaller worker rebuilt from `sidecar/src`; isolated Full package assembled with current Extension source and existing gallery-dl artifact.
- Package required-file/manifest/Extension-ID/Native-Host path checks; packaged worker v2 `ready`, unknown-field `INVALID_COMMAND`, clean `shutdown` probe.
- `npm run check --workspace desktop`; 11 targeted Native Host/package Node tests; `git diff --check`.

FAIL:
- None established as a product failure.

BLOCKED:
- Full package GUI launch and visual checks, live HKCU registration/repair/unregister, Edge/Chrome Extension loading, packaged Native Host-to-Desktop connection, cross-user pipe denial, browser connection-state transitions: `COMPUTER_USE_UNAVAILABLE`. Steps queued in `windows-queue.md`.
- `npm test --workspace desktop`: one WDIO process-termination test hit `taskkill /T /F` `Access denied` in the restricted environment; environment limitation, not a product assertion failure.

## Manual Windows Validation Queue

Run `MANUAL-WIN-E5-01`, `MANUAL-WIN-E6-01`, `MANUAL-WIN-E7-01`, `MANUAL-WIN-FULL-01`, `MANUAL-WIN-WDIO-TEARDOWN-01` in `../validation/windows-queue.md` when native GUI automation or manual Windows access is available.

## Windows Work Required

- Execute the manual queue above against this handoff revision; record per-item PASS/FAIL/BLOCKED plus the actual validation revision.
- Re-run the WDIO teardown test outside the restricted process-control environment.
- Do not claim Native Messaging end-to-end PASS from package-boundary or loopback evidence alone.

## Windows Validation Required

- Clear `WQ-EXT-E5-01`, `WQ-EXT-E6-01`, `WQ-EXT-E7-01`, `WQ-EXT-E9-01` prerequisites with live evidence; keep `WINDOWS_VERIFICATION_PENDING` until then.

## Cross-platform Follow-up

CROSS_PLATFORM_CHANGE_REQUIRED:
- None.

CROSS_PLATFORM_REVIEW_REQUIRED:
- ACCEPTED by the Cross-platform Owner in this record; review evidence in "Cross-platform Review (§13)". No findings returned.

## Next Owner

- Windows Platform Owner: fetch remote; confirm clean working tree; update to branch tip (>= `0e35fde`); run the manual Windows queue; record statuses, validation revision, commit, push. Return ownership only if a `CROSS_PLATFORM_*` follow-up arises.

Update this file for the active batch only; move completed outcomes to the history document.
