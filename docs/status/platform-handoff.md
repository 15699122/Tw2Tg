# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: add the authenticated local WebSocket transport and operational Extension popup/options UI, then hand off to Windows for browser, GUI, lifecycle, and package validation.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Linux Cross-platform Owner for the executor queue/backpressure fix and review of the Windows storage-path adjustment.
- Current state: `CROSS_PLATFORM_CHANGE_REQUIRED` (the Windows path mapping now passes limited GUI validation; rapid task submissions expose shared runner queue saturation and misleading `DOWNLOADING` states)

## Revisions

- Cross-platform input revision: `fcde5943af2f6ad15c833fa5ab88d6b1758345c6` (Windows validation batch that produced the `close_before_auth` evidence).
- Cross-platform implementation revision: `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3` (includes `ff94af7` accepted-stream blocking-mode fix and stage counters, plus parsed-host validation for Extension Tweet links).
- Previous Windows input/handoff revision: `084354a5ca433b52372aca4bc70ac5fc544104fc` (prior validation batch; current input is below).
- Windows implementation revision: `7eacb82` (`fix(windows): map archive job IDs to safe staging names`).
- Windows validation revision: `7eacb82` (Windows-target `xarchive-storage` 37/37; formatting check; isolated Desktop release build and Full package assembly).
- Windows validation input revision: `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`.
- Latest Windows runtime report revision: `57333ad3fa70702bbe9f889d128e5f230cf4a10e` (user's follow-up on the isolated package at implementation `7eacb82`).
- Windows validation record: `../validation/windows-validation-history.md` under the 2026-09-27 entries; scheduler follow-up is WQ-WS-09 and extraction timeout investigation is WQ-WS-10 in `../validation/windows-queue.md`.

## Cross-platform Work Completed

- Added an authenticated loopback WebSocket listener using the mature `tungstenite 0.28` crate. RFC 6455 framing/handshake is delegated to the library; XArchive owns only the transport envelope and BrowserRequest/BrowserResponse routing.
- Listener binds `127.0.0.1` on default port `17321`, supports controlled `XARCHIVE_WEBSOCKET_PORT` and `XARCHIVE_WEBSOCKET_TOKEN` overrides, generates a process-local token when not supplied, requires authentication before `BrowserTransportAdapter`, and preserves `request_id` semantics.
- Integrated the listener into `RuntimeState` startup/stop and `replace_executor()` generation handling so old connections do not retain a stale executor service.
- Added Extension WebSocket settings/bridge with storage-backed configuration, one-time authentication, request timeout/concurrency limits, pending cleanup, bounded reconnect, explicit auth-failure state, and diagnostic Native Messaging fallback without replaying already-submitted business requests.
- Added Extension popup and options pages for channel/status display, page availability, reconnect, settings, manual port/token pairing, and recovery states. Popup/options assets and WebSocket files are included in Extension package inventory.
- Updated protocol/runtime/architecture/risk/repository-map documentation and Windows validation queue. Current pairing is intentionally manual; automatic port discovery and credential rotation are not implemented in this batch.
- Fixed strict-Clippy issues in shared redaction, protocol validation, batch filtering, batch commands, and network diagnostics without changing runtime behavior.
- Diagnosed and fixed the Windows pre-authentication disconnect. The shared listener polls its stop flag through a non-blocking `TcpListener`, but it never restored blocking mode on the accepted stream. POSIX `accept` does not inherit `O_NONBLOCK` while Winsock does, so on Windows the handshake and the authentication read returned `WouldBlock` immediately and were counted as a close before authentication. `handle_websocket_connection` now sets the accepted stream to blocking mode before applying the read timeout, and increments the read timeout only after authentication.
- Added `handshake_failed` and `auth_read_failed` diagnostics so a target environment can distinguish an HTTP upgrade failure, a missing authentication frame, and a received non-text frame without exposing the token.
- Added the regression test `authenticates_when_the_accepted_stream_starts_non_blocking`, which explicitly sets the accepted stream non-blocking to reproduce the Winsock inheritance and delays the authentication frame by 50 ms. Removing the fix makes this test fail immediately with `HandshakeIncomplete`, which confirms the test detects the defect rather than passing trivially.

## Windows Work Completed

- At `6d60429`, Windows-targeted `authenticates_when_the_accepted_stream_starts_non_blocking` passed (1/1); Extension Node tests passed (32/32); Desktop Vite build and `cargo fmt --all -- --check` passed. A fresh Windows Tauri release executable was built, and current Full package/Extension ZIP inventory and archive integrity checks passed. Detailed outputs and hashes are in the 2026-09-27 Windows validation history entry.
- The user reports successful current-token authentication and Extension task submission. Those submitted archive jobs failed in the pre-fix package with Windows `os error 123` because the generated job ID contains `:` from its ISO timestamp.
- Implemented `7eacb82`: `FileStore` now maps opaque job IDs to Windows-safe staging components consistently across creation, recovery lookup, and commit, while the persisted job ID remains unchanged. Small shared implementation adjustment; `CROSS_PLATFORM_REVIEW_REQUIRED`.
- PASS at `7eacb82`: Windows-target `xarchive-storage` suite (37/37), `cargo fmt --all -- --check`, Tauri Windows release build, and isolated Full package assembly at `validation-artifacts/windows-batch-os123/full-package`. Package components and manifest were present; package SHA-256 values are recorded in the validation history.
- Limited PASS by user report at `7eacb82`: Sidecar and Extension connected, and new tasks entered “Downloading” without the prior Windows `os error 123`; successful archive completion/output is not yet established.
- New user report: after four rapid submissions, two tasks stayed “Downloading” for more than 30 seconds and later tasks failed with `job executor command queue is full`. A read-only package database snapshot showed one `DOWNLOADING` and three `FAILED` jobs; current package process inventory showed Desktop, the Dashboard Sidecar, and one archive worker with gallery-dl child processes. This is consistent with one active runner and a full one-entry runner queue.
- Source inspection isolates the shared behavior: the runner channel has capacity 1 and receives work through `try_send`; execution state is persisted as `DOWNLOADING` before this send. Queue saturation therefore fails submitted jobs instead of retaining them as queued work. The 300-second extraction timeout means 30 seconds alone does not establish an extraction timeout.
- Latest user screenshot: all four jobs ultimately failed: two with `DOWNLOAD_TIMEOUT: gallery-dl timed out` and two with the already-diagnosed full runner queue. The corrected package config sets extraction/discovery/transfer budgets to 300/600/1800 seconds; the extraction code emits this timeout when gallery-dl exceeds its 300-second deadline. The reason gallery-dl did not finish is not localized, so no Windows-owned implementation failure is inferred. WQ-WS-10 queues a controlled single-job retest after the Linux queue fix.
- No UI actions were taken during this follow-up. The Windows Profile identity and archive completion remain unverified; WQ-WS-09 tracks Windows revalidation after the shared fix.

### Previous revision evidence (historical, not current input)

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

- On an explicitly controlled/disposable Edge profile and the current Full package/Extension ZIP listed in the 2026-09-27 history entry, complete WQ-WS-01/02/03/04: Extension load and permissions; one correlated reconnect with `handshake_failed`/`auth_read_failed`/`auth_received` deltas; wrong/correct-token pairing and `query_status`; pending cleanup/reconnect; popup/options scaling and keyboard checks. Do not use the signed-in profile or record the token.
- At current input `6d60429`, Full package directory assembly, Extension ZIP build/inventory, Bandizip `.7z` creation, archive integrity and extracted-file SHA-256 comparison `PASS`; browser load is `BLOCKED` with `COMPUTER_USE_UNAVAILABLE`, and signing/release gate is `NOT RUN`. The following GUI observations are historical and bound to their stated artifacts/revisions.
- User-reported manual follow-up: Full package startup/close/content display and Extension options/popup display passed. Later, `127.0.0.1:17321` had a listener and TCP connect passed; one WebSocket request returned `101`, and DevTools showed an outbound authentication frame. The Extension remained disconnected/unauthenticated, no inbound authentication response or request response was observed, while Native Host requests arrived and X-page task submission created a task. Earlier attempts showed `ERR_CONNECTION_REFUSED`; preserve this chronology rather than treating the listener as continuously available. See the dated follow-up in `../validation/windows-validation-history.md` and `../validation/windows-queue.md`.
- Current corrected Full package Sidecar-running state and bundled Extension Options page display are `PASS` by user screenshots; browser permissions, service-worker health, and full popup/accessibility assertions remain `BLOCKED`/`NOT RUN`.
- At the previous `fcde594` revision, WQ-WS-02 live pairing was `FAIL` by user screenshots. At current input `6d60429`, the shared fix's Windows regression test is `PASS`, but current live pairing has not been retested and is `BLOCKED` with `COMPUTER_USE_UNAVAILABLE`; preserve the prior failure as historical evidence. WQ-WS-01 current browser load, WQ-WS-03 lifecycle, and WQ-WS-04 full accessibility are also `BLOCKED`/`NOT RUN` pending a controlled GUI target.
- Current `WQ-WS-05` static package directory, Extension ZIP inventory, and Bandizip archive/extract/hash checks `PASS` at `6d60429`; browser load and signing/release gate remain `NOT RUN`.
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

- The Windows observation that connections closed before authentication is now explained and fixed in shared code: the accepted stream inherited the listener's non-blocking mode on Winsock, so the authentication read returned `WouldBlock` instead of waiting. The fix is Linux-verified with a defect-sensitive regression test, but only a Windows revalidation against the exact handoff can confirm live pairing.
- The prior Windows WQ-WS-02 failure is preserved as historical evidence; the current queue status is pending exact-handoff revalidation, not a new Windows PASS.
- Automatic WebSocket port discovery and credential rotation are not implemented; manual pairing is documented and must be exercised in Windows WQ-WS-02.
- Real Edge/Chrome permissions and MV3 lifecycle, WebSocket authentication/request flow, Native Host fallback operation, and actual account/archive transfer remain unverified or failed as detailed in the manual follow-ups.
- P1-B real gallery-dl samples and P3-E real-account multi-page/authentication/SHA-256 acceptance remain `NOT RUN` on Linux because they require controlled external samples, credentials, or target artifacts.
- Do not treat synthetic fixtures, Linux loopback tests, Node tests, Vite builds, or package inventory as Windows acceptance.

## Relevant Tests

- Linux evidence is recorded in `docs/development/status.md` and covers the shared listener, bridge contract, GUI assets, package inventory, workspace tests, strict Clippy, and build checks.
- New shared regression coverage verifies a correctly configured token receives `authentication_response`, an authenticated loopback connection routes a real `query_status` BrowserRequest, a silent peer reaches `WEBSOCKET_AUTH_TIMEOUT`, and Desktop exposes token-free accepted/auth-received/auth-success/auth-failure/auth-response-failure/close-stage counters.
- Windows evidence must be recorded in `docs/validation/windows-queue.md` and the Windows validation history with the exact pushed handoff revision.

## Manual Windows Validation Queue

After Linux's executor scheduling change is handed back, run WQ-WS-09 and WQ-WS-10 in `../validation/windows-queue.md` on the exact returned revision. Confirm multiple rapid submissions remain durable and queued, execute without `job executor command queue is full`, then investigate one controlled archive that reaches terminal success or a sanitized, attributable failure. The configured extraction timeout is 300 seconds and transfer timeout is 1800 seconds.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: fix shared executor scheduling/backpressure. `runner_sender` is `sync_channel(1)` and uses `try_send`; `execute_persisted_from_factory` marks jobs `DOWNLOADING` before queue admission, so saturation produces `EXECUTOR_WORKER_FAILED` and loses accepted work rather than keeping it queued. Add defect-sensitive regression coverage for at least three overlapping submissions and define the durable queued/active state behavior.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: review the small `xarchive-storage::FileStore` Windows staging-component mapping in `7eacb82`; persisted IDs and public contracts are unchanged.
- `WINDOWS_BLOCKING`: none.

## Next Owner

- Linux Cross-platform Owner: implement and verify the shared runner queue/backpressure correction, and review the small shared FileStore mapping at `7eacb82`. Windows Platform Owner resumes after that handoff for WQ-WS-09 exact-revision verification.

## 2026-09-27 Sidecar Follow-up Addendum

- Current owner remains Windows Platform Owner. Source implementation under validation is `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`; no tracked production source was changed in this follow-up.
- The Full package initially assembled for this batch contains stale worker SHA-256 `96C19695AC46E30AA23AF184ED41D0C8339FE4C98A2D771CF0781906402A60C2`; its CLI rejects Desktop timeout arguments, so that package's Sidecar runtime check is `FAIL` despite the earlier static inventory/archive PASS.
- An isolated corrected package is at `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`; it uses the previously rebuilt compatible worker SHA-256 `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`. User screenshots and process paths confirm corrected Dashboard startup and Sidecar `hello -> ready` (`PASS`). Extension WebSocket reached Desktop, but the server returned `AUTHENTICATION_FAILED` (`FAIL`); WQ-WS-07 remains open for a current-token re-pair and authenticated request/response check.
- Computer Use exposes no native app target. Its Edge adapter does not expose the selected profile name and reports multiple tabs; no tab was opened. The user reports the intended Codex Profile is ready, but identity/one-tab state has not been independently confirmed.
- No `CROSS_PLATFORM_CHANGE_REQUIRED` or `CROSS_PLATFORM_REVIEW_REQUIRED` finding. Windows retains ownership and should complete WQ-WS-06/07 before closing this batch.
