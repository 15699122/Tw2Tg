# Current Platform Handoff

This file contains only the current batch. Historical Windows results are in
`../validation/windows-validation-history.md`; outstanding manual checks are in
`../validation/windows-queue.md`.

## Batch

- Task: add the authenticated local WebSocket transport and operational Extension popup/options UI, then hand off to Windows for browser, GUI, lifecycle, and package validation.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Cross-platform Owner (Linux) for shared implementation; Windows Platform Owner owns Edge/Chrome, Windows runtime, GUI behavior, packaging, and Windows validation.
- Current state: `READY_FOR_WINDOWS` (shared implementation and Linux validation complete; Windows validation queued)

## Revisions

- Cross-platform input revision: `c60ddc0635a873373d3ea5cf4c9b301e64380330`
- Cross-platform handoff revision: this handoff record commit (contains the complete Linux WebSocket/Extension GUI batch)
- Windows input revision: `c60ddc0635a873373d3ea5cf4c9b301e64380330`
- Windows implementation revision: not applicable to this Linux batch; Windows must record any platform-owned implementation separately.
- Windows validation revision: pending; prior Windows results remain bound to their original revisions and are not promoted to this handoff.

## Cross-platform Work Completed

- Added an authenticated loopback WebSocket listener using the mature `tungstenite 0.28` crate. RFC 6455 framing/handshake is delegated to the library; XArchive owns only the transport envelope and BrowserRequest/BrowserResponse routing.
- Listener binds `127.0.0.1` on default port `17321`, supports controlled `XARCHIVE_WEBSOCKET_PORT` and `XARCHIVE_WEBSOCKET_TOKEN` overrides, generates a process-local token when not supplied, requires authentication before `BrowserTransportAdapter`, and preserves `request_id` semantics.
- Integrated the listener into `RuntimeState` startup/stop and `replace_executor()` generation handling so old connections do not retain a stale executor service.
- Added Extension WebSocket settings/bridge with storage-backed configuration, one-time authentication, request timeout/concurrency limits, pending cleanup, bounded reconnect, explicit auth-failure state, and diagnostic Native Messaging fallback without replaying already-submitted business requests.
- Added Extension popup and options pages for channel/status display, page availability, reconnect, settings, manual port/token pairing, and recovery states. Popup/options assets and WebSocket files are included in Extension package inventory.
- Updated protocol/runtime/architecture/risk/repository-map documentation and Windows validation queue. Current pairing is intentionally manual; automatic port discovery and credential rotation are not implemented in this batch.
- Fixed strict-Clippy issues in shared redaction, protocol validation, batch filtering, batch commands, and network diagnostics without changing runtime behavior.

## Windows Work Completed

- No Windows implementation or validation was performed for this WebSocket/GUI handoff. Prior Windows findings remain historical and require revalidation against the exact pushed handoff.
- The existing Windows queue remains authoritative for E5–E7, WDIO/WebView2, Native Host/Registry, filesystem, aria2, packaging, signing, and release items.

## Windows Work Required

- Run `WQ-WS-01` through `WQ-WS-05` against the exact handoff revision on Edge and Chrome 116+.
- Verify Extension load, popup/options operation, host permissions, CSP/import behavior, manual token pairing, wrong-token rejection, authenticated `query_status`/`archive_request`, and token redaction.
- Verify Desktop restart, Service Worker reload, WebSocket disconnect/reconnect, pending cleanup, bounded retry, executor replacement, and Native Messaging fallback without request replay.
- Verify popup/options at 100%, 125%, and 150% scaling with keyboard, focus, long text, overflow, and save-failure behavior.
- Verify Extension ZIP and Full package inventory, hashes, version, file encoding, and absence of tests, caches, credentials, logs, and private keys.
- Continue the existing Windows account-batch, gallery-dl/aria2, filesystem recovery, WebView2, Native Host/Registry, signing, and release queue items.

## Expected Behavior

- Only an authenticated WebSocket session can call `BrowserTransportAdapter`; unauthenticated messages receive a structured authentication failure and never create or query an archive Job.
- Business messages remain valid `BrowserRequest` values and responses retain the matching `request_id`.
- Desktop restart, listener replacement, Service Worker restart, and disconnect clear pending work and do not route a new request to a stale executor generation.
- The Extension UI reports the actual channel and distinguishes unconfigured, connecting, connected, disconnected, authentication-failed, request-failed, and Native fallback states.
- Manual Desktop-to-Extension port/token pairing is the supported first-version flow; loopback binding alone is not treated as identity.

## Validation Required

- Linux PASS: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --offline`; `cargo clippy --workspace --all-targets --offline -- -D warnings`; `cargo test --workspace --offline --no-fail-fast -q` (test groups 18/18, 108/108, 22/22, 7/7, 8/8, 19/19, 6/6, 36/36, 12/12); Sidecar `compileall` + pytest 35/35; Desktop Node 93/93; Extension 25/25; package/Native Host contract 10/10; Extension package plan/verify; `git diff --check`.
- Windows: execute the `WQ-WS-01`–`WQ-WS-05` manual queue and the existing Windows account-batch/runtime/package items against the exact handoff. Any missing Windows target, browser, WebView2, account, artifact, or GUI access is `WINDOWS_BLOCKED` with evidence, not PASS.

## Risks and Deferred Items

- Automatic WebSocket port discovery and credential rotation are not implemented; manual pairing is documented and must be exercised in Windows WQ-WS-02.
- Real Edge/Chrome WebSocket permissions, MV3 Service Worker lifecycle, Windows Desktop GUI, Native Host fallback, packaged artifacts, and actual account/archive transfer remain unverified in this batch.
- P1-B real gallery-dl samples and P3-E real-account multi-page/authentication/SHA-256 acceptance remain `NOT RUN` on Linux because they require controlled external samples, credentials, or target artifacts.
- Do not treat synthetic fixtures, Linux loopback tests, Node tests, Vite builds, or package inventory as Windows acceptance.

## Relevant Tests

- Linux evidence is recorded in `docs/development/status.md` and covers the shared listener, bridge contract, GUI assets, package inventory, workspace tests, strict Clippy, and build checks.
- Windows evidence must be recorded in `docs/validation/windows-queue.md` and the Windows validation history with the exact pushed handoff revision.

## Manual Windows Validation Queue

Run `WQ-WS-01` through `WQ-WS-05` from `../validation/windows-queue.md` against the exact pushed handoff, then continue E5–E7/Full/WDIO/WebView2/filesystem/release items. If a prerequisite is unavailable, skip only the affected step and record `WINDOWS_BLOCKED` with the manual steps in the queue.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: none outstanding for this batch.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: none outstanding.
- `WINDOWS_BLOCKING`: none.

## Next Owner

- Cross-platform Owner (Linux): no further shared implementation remains in this batch; await Windows validation evidence.
- Windows Platform Owner: fetch the pushed handoff, run WQ-WS-01–05 and the existing Windows queue, record exact revision-bound PASS/FAIL/BLOCKED/NOT_RUN results, and route any shared-contract finding back as `CROSS_PLATFORM_CHANGE_REQUIRED`.
