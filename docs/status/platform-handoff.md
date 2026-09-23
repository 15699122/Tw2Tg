# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: Windows E5 Named Pipe Desktop transport, E6 current-user Native Host Registry lifecycle, E7 live Extension/Host/transport status, and current-source Full package validation.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Cross-platform Owner (shared UI review), then Windows Platform Owner for queued manual acceptance.
- Current state: `CROSS_PLATFORM_REVIEW_REQUIRED` / `WINDOWS_VERIFICATION_PENDING`

## Revisions

- Cross-platform input revision: `59c8221`
- Cross-platform handoff revision: `c40f312`
- Windows input revision: `bde6dc375bda68944b500714db8f9c39693bd06f`
- Windows implementation revision: `2dcae6c9a04175d6aa7e2478d421b497934a039e`
- Windows validation revision: `2dcae6c9a04175d6aa7e2478d421b497934a039e` (source built and Windows-target tests run from this implementation tree)

## Windows Implementation

- E5: added a Windows-only Named Pipe listener using the shared `xarchive_protocol::WINDOWS_PIPE_ENDPOINT` default and `XARCHIVE_PIPE_ENDPOINT` diagnostic override; applies an owner-only protected DACL, processes concurrent connections with the existing Native Messaging framing helpers and Desktop adapter, and exposes connection facts to status.
- E6: added current-user HKCU Edge/Chrome registration, repair after portable-root movement, and unregister commands. Preflights manifest identity and existing registry mappings and refuses unknown mappings. Status verifies the managed manifest and executable.
- E7: wired Windows Native Host registration and transport facts into Extension status and Windows-only Settings actions; non-Windows status behavior remains unchanged.
- Full package builder now accepts isolated `PORTABLE_APP_BINARY`, `PORTABLE_NATIVE_HOST_BINARY`, and `PORTABLE_WORKER_DIRECTORY` inputs without overwriting the isolated Native Host with `target/release`.
- Protocol/schema were unchanged. The shared `main.jsx`/Settings UI wiring is OS-gated and requires `CROSS_PLATFORM_REVIEW_REQUIRED`; no `CROSS_PLATFORM_CHANGE_REQUIRED` was found.

## Validation

PASS:
- `cargo fmt --all -- --check`.
- `cargo test -p xarchive-desktop --lib --target x86_64-pc-windows-msvc --offline`: 89 passed, including Windows Named Pipe loopback with two sequential framed requests and request IDs.
- Isolated Windows release build of Desktop and Native Host from current Rust source.
- PyInstaller worker rebuilt from `sidecar/src`; isolated Full package assembled with current Extension source and the existing gallery-dl dependency artifact.
- Package required-file/manifest/Extension-ID/Native-Host path checks; packaged worker v2 `ready`, unknown-field `INVALID_COMMAND`, and clean `shutdown` probe.
- `npm run check --workspace desktop`; 11 targeted Native Host/package Node tests; `git diff --check`.

FAIL:
- None established as a product failure.

BLOCKED:
- Full package GUI launch and visual checks, live HKCU registration/repair/unregister, Edge/Chrome Extension loading, packaged Native Host-to-Desktop connection, cross-user pipe denial, and browser connection-state transitions: `COMPUTER_USE_UNAVAILABLE` (native app inventory empty and `computer.launch_app` unavailable). Exact steps are queued in `windows-queue.md`.
- `npm test --workspace desktop`: one existing WDIO process-termination test could not complete because `taskkill /T /F` returned `Access denied` in the restricted process-control environment. Re-run outside that environment; this is not a product assertion failure.

The previous WebView2 readiness history is retained as historical evidence. This batch did not rerun the full readiness regression; current Full-package GUI acceptance is explicitly BLOCKED and is not inferred from build or unit tests.

## Manual Windows Validation Queue

Run `MANUAL-WIN-E5-01`, `MANUAL-WIN-E6-01`, `MANUAL-WIN-E7-01`, `MANUAL-WIN-FULL-01`, and `MANUAL-WIN-WDIO-TEARDOWN-01` in `../validation/windows-queue.md` when native GUI automation or manual Windows access is available.

## Cross-platform Follow-up

CROSS_PLATFORM_CHANGE_REQUIRED:
- None.

CROSS_PLATFORM_REVIEW_REQUIRED:
- Review OS-gated shared frontend wiring in `desktop/src/main.jsx` and `desktop/src/pages/settings-page.jsx`.

## Next Owner

- Cross-platform Owner: review the shared frontend wiring and either accept it or return a review finding through Git.
- Windows Platform Owner: after review, execute the remaining manual Windows queue; update statuses and validation revision before declaring the batch complete.
