# Current Platform Handoff

Status: `CURRENT`. Prior batches are archived verbatim in [handoff history](platform-handoff-history.md): Telegram Batch A shared layer and the Windows validation of the delivered Telegram shared modules.

## Batch and revisions

- Task: Windows targeted revalidation of the delivered live-socket status fix, preserving the remaining Telegram [Plan](../development/telegram-local-bot-api-plan.md).
- Branch: `codex/windows-validation-1f14cea`. Input remote branch: `dev`, explicitly fetched and fast-forwarded; tracked tree clean at validation start. Local untracked data/caches/artifacts preserved.
- Cross-platform implementation/handoff and Windows input/tested source: `b015fbe81a0b47c2b486a5256bd81ac95fd98d25`. Windows implementation changes: none. Earlier `1f14cea` results below are historical evidence only.
- Windows validation-record/handoff revision: the commit containing this updated record, pushed to the branch above. Fresh local Desktop EXE SHA-256 `B3550F5CEB6764C2D8DD1E06B94D1E143B936B7B28ACA318F8549EDF1677A1F2`; local dev-channel integration fixture, not a new Full release package.
- Current state: `CROSS_PLATFORM_REVIEW_REQUIRED`; next Owner **Cross-platform Owner** for post-auth closure/status-read reproduction and diagnostic-display follow-up. Windows retains native acceptance after the next Git handoff.
- No direct sync; the Windows working tree is updated through Git only.

## Reconciled Windows results

Accepted as Windows evidence for `1f14cea`, unchanged by this batch:

- PASS: Telegram 48/48, storage 51/51 (three Unix-only symlink tests `NOT_APPLICABLE`), Desktop lib 175/175 after binding `PYTHON` to the native interpreter; fmt and docs audit PASS. Windows test executable compiled.
- Tooling recoveries (Cargo sandbox Schannel dependency fetch, missing offline `mime_guess` cache, WindowsApps `python3` alias in discovery stubs) are environment findings; **no product defect was inferred** and none is recorded here.
- `NOT_RUN` with `IMPLEMENTATION_NOT_READY`: `WQ-TG-001`-`009` and `WQ-TG-UNI-01`-`08`. No real-send, Unigram-display or large-file acceptance is claimed.
- Release/GUI follow-ups (icon matrix, keyboard traversal, settings/logs subchecks) stand as recorded in [Windows history](../validation/windows-validation-history.md) and [manual steps](../validation/windows-manual-steps.md) sections L/M. This batch changes none of them.

## Routed review: connection/status agreement — outcome

`CROSS_PLATFORM_REVIEW_REQUIRED` from the Windows batch asked for a review of shared Extension lifecycle/status reporting and Desktop status observation, **without** assuming that a Desktop refresh caused the close or that MV3 idle behaviour is the cause. That assumption is not made and is not needed: two status-reporting defects are demonstrable on Linux alone.

1. **Extension reported authentication from a cached flag.** `WebSocketBridge.getStatus()` answered `state: "connected"` / `authenticated: true` from `this.state`, a value set when authentication last succeeded. A socket that is gone makes that flag stale, so the Extension could display "authenticated" with nothing connected. The reported state is now derived from the live socket (`liveState()`), which is correct regardless of *why* the socket disappeared. `TransportBridge.send()` uses the same derivation, so a dead socket is no longer trusted for a send.
2. **Desktop could report a connection that did not exist.** `browser_connection()` also returned `"connected"` for 30 seconds after the last request, even with zero open sockets. It now answers from the live socket count only. The removed time window is replaced by a diagnostic (`last_request_age_seconds`), which separates "never used the connection" from "used it and then went silent" without the status speaking in the connection's favour.
3. **Self-healing after a silent close.** Reconnection was reachable only from `onclose`/`onerror` or a manual reconnect click. A silent close leaves no timer, so the state stayed wrong until the user acted. `ensureConnected()` now repairs a configured-but-disconnected bridge on the next status read.

Linux evidence for each: the new Desktop test and the two new Extension tests fail against the pre-fix implementation and pass after it (verified by temporarily restoring the old logic), so they pin the defects rather than the new wording. Full Linux gate in this batch: workspace tests 381/381 PASS, strict Clippy `-D warnings` PASS, `cargo fmt --check` PASS, Desktop Node 189/189 PASS and `vite build` PASS, Extension Node 35/35 PASS and `check` PASS, documentation audit PASS.

Not established: why the post-authentication closes accumulate, whether MV3 worker lifetime contributes, and whether any idle duration is unsafe. Those remain Windows observations. This batch fixed only the parts provable without Windows.

## This batch's changes

- `extension/src/websocket-bridge.js`: `liveState()`, `ensureConnected()`, status and send guard on the live socket; `SOCKET_OPEN` exported.
- `extension/src/background.js`: a status read heals a silently closed connection before answering.
- `desktop/src-tauri/src/websocket_transport.rs`: `browser_connection()` from live sockets only; `last_request_age_seconds()` added to the session and to `WebSocketDiagnosticSnapshot`.
- `desktop/src-tauri/src/commands.rs`: the not-started snapshot reports the new diagnostic field as absent.
- Tests: one Desktop session-state test, two Extension bridge tests.
- Documentation: this handoff, [handoff history](platform-handoff-history.md), [queue](../validation/windows-queue.md), [manual steps](../validation/windows-manual-steps.md) section M, and the [status matrix](../development/status.md).

## Windows work and validation required

Minimum scope for the next Windows batch, bound to this batch's commit:

1. **Re-check connection/status agreement (WQ-WS-02/03 area).** With one authenticated connection established and no Desktop restart, refresh the Extension options page and the Desktop Extension section, then read both. Expectation: both sides show the same state at the same moment, and the Extension never displays "authenticated" while Desktop reports no open socket. The Desktop Extension diagnostics now expose `last_request_age_seconds` (`null` until a request arrives); use it to distinguish a connection that was never used from one that went silent, instead of inferring from cumulative counters.
2. **Controlled idle-duration observation (still `NOT_RUN`).** At ~5 s and ~45 s after a confirmed connection, record both sides' state plus `accepted`, `auth_succeeded`, `close_after_auth` and `last_request_age_seconds`. Record what was observed; do not attribute a cause that was not reproduced.
3. **Real archive and duplicate submission** using the authorized URL `https://x.com/thsottiaux/status/2105039482013757749`, as already queued.
4. Telegram `WQ-TG-*` remain `NOT_RUN` / `IMPLEMENTATION_NOT_READY`; their entry points still do not exist and this batch did not change that.

Preserved history: `WQ-WS-05` packaging PASS and the earlier authentication PASS stay valid for their recorded revisions and are not re-run by default.

`CROSS_PLATFORM_CHANGE_REQUIRED`: none. `CROSS_PLATFORM_REVIEW_REQUIRED`: the routed item is answered above; Windows re-validation decides its status. `WINDOWS_VERIFICATION_BLOCKING`: none — no Linux work waits on these results.

## Next Owner

**Cross-platform Owner** for `CROSS_PLATFORM_REVIEW_REQUIRED`: current-revision pairing succeeds but sessions subsequently close; review close initiator/timing and status-read reconnect before claiming persistence fixed. Also `last_request_age_seconds` is returned by Rust but absent from `desktop/src/pages/settings-page.jsx:40`; expose a supported readout or revise the evidence recipe. Continue remaining Telegram wiring independently. **Windows Owner** retains browser/idle/real-archive continuation and native acceptance after a new Git handoff; Computer Use was detected and used, no blanket COMPUTER_USE_UNAVAILABLE classification applies. Extension-scheme controls remain manual. Procedures: [manual steps](../validation/windows-manual-steps.md).

## Windows b015fbe results

- PASS: targeted Extension 19/19 and native Windows WebSocket 6/6; Extension syntax, UI build, fresh Tauri native build, fmt and docs audit. Full regression deliberately not run for this bounded transport status change. No production assertions changed.
- Current-token pairing subcheck PASS: Owner loaded current checkout Extension and reported authenticated on 17321; new Desktop Computer Use observed authenticated/connected, accepted/auth_received 7, success 1, failed 6, close_before 6, close_after 0, other failure counters 0. Failed attempts retained, not erased.
- FAIL observed connection-persistence subcheck: Owner first refreshed Desktop (unauthenticated) then options (authenticated); a further Desktop-only refresh still showed disconnected, accepted/auth_received 8, success 2, failed 6, close_before 6, close_after 2, other failure counters 0. A new authentication and subsequent closure occurred after the options read; cause/duration unclassified. Sequential displays alone do not prove simultaneous live-state disagreement. Initial pairing PASS retained.
- Controlled ~5s/~45s acceptance NOT_RUN: manual response latency exceeded the requested window, and last-request-age is absent from UI. Initial and later saved Desktop display snapshots are 236.584 seconds apart, not a measured close timeout. Real archive/duplicates NOT_RUN, paused under the Owner's prior issue-handling choice; authorized test URL unchanged.
- Fresh fixture: `validation-artifacts/windows-batch-b015fbe/app/`; current Desktop/Extension plus reused older Full download components, fresh config/DB. Safe logs and hash manifests in Windows history. Telegram acceptance remains IMPLEMENTATION_NOT_READY, no release approval.
