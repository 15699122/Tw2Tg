# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: add the authenticated local WebSocket transport and operational Extension popup/options UI, then hand off to Windows for browser, GUI, lifecycle, and package validation.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Windows Platform Owner; automated validation is complete, with live GUI verification blocked pending a controlled Computer Use target.
- Current state: `WINDOWS_BLOCKED` (Windows-target Rust tests, Extension/Desktop suites, frontend checks, and Tauri release executable build pass at this handoff; WQ-WS-01/02/03/04 live GUI checks are blocked with `COMPUTER_USE_UNAVAILABLE`)

## Revisions

- Cross-platform input revision: `2d067cbf0412a8819fb52b14895f00e7c0b46bd2`
- Cross-platform implementation revision: `de46a8ee7ac937f9ed64db5f16b9e8c4cb1c178d` (shared authentication timeout, correct-token business-route test, redacted accept/auth/close counters, and Extension auth-timeout state)
- Previous Windows input/handoff revision: `084354a5ca433b52372aca4bc70ac5fc544104fc` (prior validation batch; current input is below).
- Windows implementation revision: none; no Windows production-source change was required.
- Current Windows package/validation input revision: `fcde5943af2f6ad15c833fa5ab88d6b1758345c6` (the commit adds validation documentation only; application source is unchanged from `98f16845b9e37f0dea419e7bffc890b1be3d2063`).
- Current Windows validation record: `../validation/windows-validation-history.md` under the 2026-09-25 revalidation and package preparation entries; current scoped statuses are in `../validation/windows-queue.md`.

## Cross-platform Work Completed

- Added an authenticated loopback WebSocket listener using the mature `tungstenite 0.28` crate. RFC 6455 framing/handshake is delegated to the library; XArchive owns only the transport envelope and BrowserRequest/BrowserResponse routing.
- Listener binds `127.0.0.1` on default port `17321`, supports controlled `XARCHIVE_WEBSOCKET_PORT` and `XARCHIVE_WEBSOCKET_TOKEN` overrides, generates a process-local token when not supplied, requires authentication before `BrowserTransportAdapter`, and preserves `request_id` semantics.
- Integrated the listener into `RuntimeState` startup/stop and `replace_executor()` generation handling so old connections do not retain a stale executor service.
- Added Extension WebSocket settings/bridge with storage-backed configuration, one-time authentication, request timeout/concurrency limits, pending cleanup, bounded reconnect, explicit auth-failure state, and diagnostic Native Messaging fallback without replaying already-submitted business requests.
- Added Extension popup and options pages for channel/status display, page availability, reconnect, settings, manual port/token pairing, and recovery states. Popup/options assets and WebSocket files are included in Extension package inventory.
- Updated protocol/runtime/architecture/risk/repository-map documentation and Windows validation queue. Current pairing is intentionally manual; automatic port discovery and credential rotation are not implemented in this batch.
- Fixed strict-Clippy issues in shared redaction, protocol validation, batch filtering, batch commands, and network diagnostics without changing runtime behavior.

## Windows Work Completed

- Revalidated the new shared auth-timeout/diagnostic-counter handoff on Windows: WebSocket-focused Rust tests 4/4; complete Windows Desktop crate tests 111/111 with the repository Python 3.12 interpreter; Extension 26/26; Desktop Node 93/93; `cargo fmt --all -- --check`; workspace `npm run check` (Vite 52 modules and Extension syntax); Windows Tauri release executable build (`--no-bundle --ci`) passed.
- Assembled an exact-HEAD Full portable package directory and built/expanded/verified the Extension ZIP at `validation-artifacts/windows-ws-fcde594/`; Extension ZIP inventory is 12/12 and the package Native Host allowed origin matches the repository-derived Extension ID. The package tag `v0.0.0-pre.1` is local validation metadata, not a release.
- Full package directory and Bandizip-generated `.7z` archive are available under `validation-artifacts/windows-ws-fcde594`; archive integrity and extracted-file hashes passed. The Full package was locally augmented with aria2 from the user's `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2` directory at `sidecar/aria2/aria2c.exe`; this is a validation artifact, not a release-builder change.
- Isolated outputs under `validation-artifacts/windows-ws-de46a8e/`; no Windows production code was changed.
- Computer Use was retried finitely; no native app targets were available, and the only Edge surface exposed the existing user profile. It was left untouched. Current live GUI WQ-WS-01/02/03/04 checks are `BLOCKED` with `COMPUTER_USE_UNAVAILABLE`.
- Previous `084354a` batch baseline (historical, not the current validation input): Windows 11 x64 validation without Windows production-code changes.
- Windows-target core/protocol/desktop Rust tests: 146 passed; Desktop Node: 93/93; Extension: 25/25; affected Sidecar tests: 15/15; Vite production build: 52 modules.
- In that previous `084354a` batch, built a Full package in an isolated validation directory and passed static package inventory; Extension ZIP extraction/inventory verified 12 files. This does not establish current-revision packaging.
- In that previous `084354a` batch, Full package Dashboard WebView2 readiness smoke passed 3/3 with pinned WebView2 Runtime `153.0.4234.48`, EdgeDriver `153.0.4234.46`, and tauri-driver `2.0.6`; this historical smoke is not a current GUI result.
- Existing local dependencies, logs, manual-validation files, and validation artifacts were preserved. Detailed hashes, commands, constraints, and first-attempt setup corrections are recorded in `../validation/windows-validation-history.md` under the `084354a` entry.

## Windows Work Required

- Use a controlled/disposable Edge profile with the Full package at `validation-artifacts/windows-ws-fcde594/full-package` and Extension ZIP `validation-artifacts/windows-ws-fcde594/XArchive-v0.0.0-pre.1-extension.zip` to verify extension load, `auth_timeout` and Desktop diagnostic counters, wrong/correct-token pairing, `query_status` response, and pending reconnect/cleanup. Do not use the signed-in profile; see the package preparation manual queue entry.
- Current-revision Full package directory assembly, Extension ZIP build/inventory, Bandizip `.7z` creation, archive integrity, and extracted-file SHA-256 comparison `PASS`; corrected Full package bundled Extension Options display `PASS` by user screenshot, while ZIP browser load remains `NOT RUN`. Signing/release gate remains `NOT RUN`.
- User-reported manual follow-up: Full package startup/close/content display and Extension options/popup display passed. Later, `127.0.0.1:17321` had a listener and TCP connect passed; one WebSocket request returned `101`, and DevTools showed an outbound authentication frame. The Extension remained disconnected/unauthenticated, no inbound authentication response or request response was observed, while Native Host requests arrived and X-page task submission created a task. Earlier attempts showed `ERR_CONNECTION_REFUSED`; preserve this chronology rather than treating the listener as continuously available. See the dated follow-up in `../validation/windows-validation-history.md` and `../validation/windows-queue.md`.
- Current corrected Full package Sidecar-running state and bundled Extension Options page display are `PASS` by user screenshots; browser permissions, service-worker health, and full popup/accessibility assertions remain `BLOCKED`/`NOT RUN`.
- Current WQ-WS-01 Extension Options load/display is `PASS` by user screenshot, while complete permission/service-worker/keyboard checks remain `BLOCKED`/`NOT RUN`. Current WQ-WS-02 live pairing is `FAIL`: the latest selected DevTools request shows an outbound `authenticate` frame, but no inbound `authentication_response`; Extension UI remains disconnected and Edge reports `ERR_SOCKET_NOT_CONNECTED`. An earlier aggregate Desktop snapshot showed accepted 100/auth received 0/closed-before-auth 99, but it is not correlated to this selected request and cannot establish whether that frame reached Desktop. WQ-WS-03 lifecycle and full WQ-WS-04 accessibility remain `BLOCKED`/`NOT RUN`.
- Current `WQ-WS-05` Full package directory and Extension ZIP inventory, plus Bandizip `.7z` create/test/extract/hash comparison `PASS`; corrected Full package Extension Options load is `PASS` by user screenshot; ZIP browser load and signing/release gate remain `NOT RUN`.
- Root cause for the supplied Full package startup failure is confirmed: its frozen worker predates Desktop's `--timeout-seconds` and `--discovery-timeout-seconds` launch arguments and exits with code 2 before the v2 handshake. A fresh worker built from current Sidecar source accepts those exact arguments and reaches `ready`; the corrected Full package archive passed integrity and 77-file extraction/hash comparison. The latest user screenshot now shows Sidecar connected in the corrected package; no archive task/download success is established.
- Follow-up on that corrected package: user screenshots show Sidecar connected and Extension WebSocket still disconnected. The latest selected DevTools request contains an outbound `authenticate` frame but no visible inbound response. The earlier aggregate listener counters are not correlated to that request; see the dated queue/history entries.
- Per the user's instruction, pairing-token privacy/security review is `NOT APPLICABLE`; the user states this is a local-validation token reset on Desktop restart. No compromise finding or rotation action is tracked. Pairing remains functionally unverified/failed; collect one correlated reconnect, Desktop counter deltas, and the selected Messages frame before assigning the receive/response failure boundary.
- Continue the existing Windows account-batch, gallery-dl/aria2, filesystem recovery, Native Host/Registry, signing, and release queue items. Previous account-batch and reconnect observations remain bound to their original artifact/revision until a new manual retest.

## Expected Behavior

- Only an authenticated WebSocket session can call `BrowserTransportAdapter`; unauthenticated messages receive a structured authentication failure and never create or query an archive Job.
- Business messages remain valid `BrowserRequest` values and responses retain the matching `request_id`.
- Desktop restart, listener replacement, Service Worker restart, and disconnect clear pending work and do not route a new request to a stale executor generation.
- The Extension UI reports the actual channel and distinguishes unconfigured, connecting, connected, disconnected, authentication-failed, request-failed, and Native fallback states.
- Manual Desktop-to-Extension port/token pairing is the supported first-version flow; loopback binding alone is not treated as identity.

## Validation Required

- Linux PASS: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --offline`; `cargo clippy --workspace --all-targets --offline -- -D warnings`; `cargo test --workspace --offline --no-fail-fast -q` (test groups 18/18, 110/110, 22/22, 7/7, 8/8, 19/19, 6/6, 36/36, 12/12); Sidecar `compileall` + pytest 35/35; Desktop Node 93/93; Extension 26/26; package/Native Host contract 10/10; Extension package plan/verify; `git diff --check`.
- Windows current revision: automated Rust/Node/frontend/format/Tauri executable checks pass as recorded below. Live GUI checks remain blocked until a controlled Edge profile and matching Full package are available; once available, verify auth timeout/counters, wrong/correct token, request delivery, reconnect and Native fallback. The exact manual steps and blocker evidence are in the dated queue/history records.

## Risks and Deferred Items

- The Extension now has a bounded authentication timeout, so an absent response cannot indefinitely block `ready` or settings changes. The underlying Windows observation that one auth frame received no response remains unisolated; the listener/Upgrade success alone does not prove server receipt or response delivery.
- The prior Windows WQ-WS-02 failure is preserved as historical evidence; automated exact-handoff Windows tests pass, but live browser revalidation is currently blocked, not promoted to PASS.
- Automatic WebSocket port discovery and credential rotation are not implemented; manual pairing is documented and must be exercised in Windows WQ-WS-02.
- Real Edge/Chrome permissions and MV3 lifecycle, WebSocket authentication/request flow, Native Host fallback operation, and actual account/archive transfer remain unverified or failed as detailed in the manual follow-ups.
- P1-B real gallery-dl samples and P3-E real-account multi-page/authentication/SHA-256 acceptance remain `NOT RUN` on Linux because they require controlled external samples, credentials, or target artifacts.
- Do not treat synthetic fixtures, Linux loopback tests, Node tests, Vite builds, or package inventory as Windows acceptance.

## Relevant Tests

- Linux evidence is recorded in `docs/development/status.md` and covers the shared listener, bridge contract, GUI assets, package inventory, workspace tests, strict Clippy, and build checks.
- New shared regression coverage verifies a correctly configured token receives `authentication_response`, an authenticated loopback connection routes a real `query_status` BrowserRequest, a silent peer reaches `WEBSOCKET_AUTH_TIMEOUT`, and Desktop exposes token-free accepted/auth-received/auth-success/auth-failure/auth-response-failure/close-stage counters.
- Windows evidence must be recorded in `docs/validation/windows-queue.md` and the Windows validation history with the exact pushed handoff revision.

## Manual Windows Validation Queue

Continue the exact rows in `../validation/windows-queue.md`, prioritizing `WQ-WS-02` and `WQ-WS-03` against `98f16845b9e37f0dea419e7bffc890b1be3d2063`. Capture client state/error code and Desktop listener/authentication state without copying the token. The user-reported WebSocket failure remains historical until live exact-revision GUI verification; do not promote package/static or Dashboard startup PASS to successful live Extension pairing or account/archive acceptance.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: none newly identified in Windows validation. The earlier shared follow-up was implemented in `de46a8ee7ac937f9ed64db5f16b9e8c4cb1c178d`; automated Windows revalidation passes, while live GUI verification is blocked. The original no-response root cause is not retroactively proven.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none outstanding.
- `WINDOWS_BLOCKING`: none.

## Next Owner

- Windows Platform Owner: keep ownership because there is no new cross-platform follow-up; complete the blocked current-revision GUI pairing/lifecycle checks when a controlled browser/app target is available, then close the batch or route a newly demonstrated shared issue as `CROSS_PLATFORM_CHANGE_REQUIRED`.
