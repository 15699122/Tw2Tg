# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: add the authenticated local WebSocket transport and operational Extension popup/options UI, then hand off to Windows for browser, GUI, lifecycle, and package validation.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Windows Platform Owner.
- Current state: `WINDOWS_VALIDATION_PARTIAL` (Windows-target automation, Full package assembly, and dashboard readiness passed; Edge/Chrome Extension UI and live WebSocket pairing remain blocked on a controlled GUI/browser session)

## Revisions

- Cross-platform input revision: `c60ddc0635a873373d3ea5cf4c9b301e64380330`
- Cross-platform implementation revision: `62b0368` (the implementation commit; a later handoff-record commit captured this implementation)
- Windows input/handoff revision: `084354a5ca433b52372aca4bc70ac5fc544104fc`
- Windows implementation revision: none; no Windows production-source change was required.
- Windows validation input revision: `084354a5ca433b52372aca4bc70ac5fc544104fc`.
- Windows validation record: appended in `../validation/windows-validation-history.md`; current queue statuses are in `../validation/windows-queue.md`. The tested source and validation input are both `084354a5ca433b52372aca4bc70ac5fc544104fc`.

## Cross-platform Work Completed

- Added an authenticated loopback WebSocket listener using the mature `tungstenite 0.28` crate. RFC 6455 framing/handshake is delegated to the library; XArchive owns only the transport envelope and BrowserRequest/BrowserResponse routing.
- Listener binds `127.0.0.1` on default port `17321`, supports controlled `XARCHIVE_WEBSOCKET_PORT` and `XARCHIVE_WEBSOCKET_TOKEN` overrides, generates a process-local token when not supplied, requires authentication before `BrowserTransportAdapter`, and preserves `request_id` semantics.
- Integrated the listener into `RuntimeState` startup/stop and `replace_executor()` generation handling so old connections do not retain a stale executor service.
- Added Extension WebSocket settings/bridge with storage-backed configuration, one-time authentication, request timeout/concurrency limits, pending cleanup, bounded reconnect, explicit auth-failure state, and diagnostic Native Messaging fallback without replaying already-submitted business requests.
- Added Extension popup and options pages for channel/status display, page availability, reconnect, settings, manual port/token pairing, and recovery states. Popup/options assets and WebSocket files are included in Extension package inventory.
- Updated protocol/runtime/architecture/risk/repository-map documentation and Windows validation queue. Current pairing is intentionally manual; automatic port discovery and credential rotation are not implemented in this batch.
- Fixed strict-Clippy issues in shared redaction, protocol validation, batch filtering, batch commands, and network diagnostics without changing runtime behavior.

## Windows Work Completed

- Validated the handoff on Windows 11 x64 without Windows production-code changes.
- Windows-target core/protocol/desktop Rust tests: 146 passed; Desktop Node: 93/93; Extension: 25/25; affected Sidecar tests: 15/15; Vite production build: 52 modules.
- Built a current-source Full package in an isolated validation directory with Tauri CLI, current Native Host and freshly frozen Sidecar worker. Static package inventory passed; Extension ZIP extraction/inventory verified 12 files.
- Full package Dashboard WebView2 readiness smoke passed 3/3 with the pinned WebView2 Runtime `153.0.4234.48`, EdgeDriver `153.0.4234.46`, and tauri-driver `2.0.6`.
- Existing local dependencies, logs, manual-validation files, and validation artifacts were preserved. Detailed hashes, commands, constraints, and first-attempt setup corrections are recorded in `../validation/windows-validation-history.md` under the `084354a` entry.

## Windows Work Required

- `WQ-WS-01`–`WQ-WS-04` remain `WINDOWS_BLOCKED` by unavailable controlled GUI/browser interaction; the bounded Computer Use retry found no native app launch target. Use an isolated browser profile and local test token/request fixture when available.
- `WQ-WS-05` passed Extension ZIP and Full package assembly/static inventory; loading the ZIP in Edge/Chrome remains `NOT RUN`.
- Continue the existing Windows account-batch, gallery-dl/aria2, filesystem recovery, Native Host/Registry, signing, and release queue items. Previous account-batch and reconnect observations remain bound to their original artifact/revision until a new manual retest.

## Expected Behavior

- Only an authenticated WebSocket session can call `BrowserTransportAdapter`; unauthenticated messages receive a structured authentication failure and never create or query an archive Job.
- Business messages remain valid `BrowserRequest` values and responses retain the matching `request_id`.
- Desktop restart, listener replacement, Service Worker restart, and disconnect clear pending work and do not route a new request to a stale executor generation.
- The Extension UI reports the actual channel and distinguishes unconfigured, connecting, connected, disconnected, authentication-failed, request-failed, and Native fallback states.
- Manual Desktop-to-Extension port/token pairing is the supported first-version flow; loopback binding alone is not treated as identity.

## Validation Required

- Linux PASS: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --offline`; `cargo clippy --workspace --all-targets --offline -- -D warnings`; `cargo test --workspace --offline --no-fail-fast -q` (test groups 18/18, 108/108, 22/22, 7/7, 8/8, 19/19, 6/6, 36/36, 12/12); Sidecar `compileall` + pytest 35/35; Desktop Node 93/93; Extension 25/25; package/Native Host contract 10/10; Extension package plan/verify; `git diff --check`.
- Windows: automated and package checks are complete for this input. Live Edge/Chrome Extension GUI and pairing/lifecycle remain `WINDOWS_BLOCKED` per WQ-WS-01–04; Extension ZIP browser load remains `NOT RUN` per WQ-WS-05. Other Windows account-batch/runtime/package rows retain their own revision-bound states.

## Risks and Deferred Items

- Automatic WebSocket port discovery and credential rotation are not implemented; manual pairing is documented and must be exercised in Windows WQ-WS-02.
- Real Edge/Chrome WebSocket permissions, MV3 Service Worker lifecycle, Windows Desktop GUI, Native Host fallback, packaged artifacts, and actual account/archive transfer remain unverified in this batch.
- P1-B real gallery-dl samples and P3-E real-account multi-page/authentication/SHA-256 acceptance remain `NOT RUN` on Linux because they require controlled external samples, credentials, or target artifacts.
- Do not treat synthetic fixtures, Linux loopback tests, Node tests, Vite builds, or package inventory as Windows acceptance.

## Relevant Tests

- Linux evidence is recorded in `docs/development/status.md` and covers the shared listener, bridge contract, GUI assets, package inventory, workspace tests, strict Clippy, and build checks.
- Windows evidence must be recorded in `docs/validation/windows-queue.md` and the Windows validation history with the exact pushed handoff revision.

## Manual Windows Validation Queue

Continue the exact outstanding rows in `../validation/windows-queue.md`: WQ-WS-01–04 (controlled Edge/Chrome UI, pairing/auth and lifecycle) are `WINDOWS_BLOCKED`; WQ-WS-05 package inventory passed but browser-loading remains `NOT RUN`. Then continue E5–E7/Full/WDIO/WebView2/filesystem/release items. Do not promote package/static or Dashboard startup PASS to live Extension/real-account acceptance.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: none outstanding for this batch.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none outstanding.
- `WINDOWS_BLOCKING`: none.

## Next Owner

- Windows Platform Owner: commit and push the Windows validation record, then continue the blocked GUI/browser rows when an isolated browser profile and native GUI session are available; keep independent Windows queue work moving.
- Cross-platform Owner (Linux): no newly identified cross-platform implementation or review follow-up from this batch.
