# Current Platform Handoff

Status: `CURRENT`. Prior batches are archived verbatim in [handoff history](platform-handoff-history.md): Telegram Batch A shared layer and the Windows validation of the delivered Telegram shared modules.

## Current active batch — browser automatic-pairing Phase 1 contract

- Plan: [`../development/browser-automatic-pairing-plan.md`](../development/browser-automatic-pairing-plan.md).
- Input branch/revision: `dev` / `b015fbe81a0b47c2b486a5256bd81ac95fd98d25`; working tree was clean before the browser-pairing batch. Current changes remain uncommitted, so this is **not** a formal Windows handoff.
- Current owner: **Cross-platform Owner**. State: `CROSS_PLATFORM_IN_PROGRESS`; completed scope is Phase 1 Rust contract/schema verification. Phase 2/3 runtime work is not complete.
- Progress sync: WIP commit `TBD` on `dev` records the contract/schema batch plus the reviewed Extension no-fallback boundary and the removal of the legacy WebSocket token from Desktop status. It is a progress checkpoint only, not `READY_FOR_WINDOWS`; no uncommitted changes are included in the checkpoint.
- Existing WQ-WS-02 authentication failure and WQ-WS-03 lifecycle blocker remain historical, unresolved evidence. Automatic pairing does not close them.
- Completed in this working batch: Plan/index/status/queue routing; versioned pairing Rust request/response/authentication types and validation; four Draft 2020-12 schemas reviewed against Rust semantics; `scripts/validate-browser-pairing-schema.py` validates schema metaschemas and positive/negative fixtures using development-only `jsonschema` in the local `.venv` (no project/runtime dependency added). Rust additionally rejects control characters in request IDs/error codes/messages.
- Linux evidence for this batch: `cargo fmt --all -- --check` PASS; `cargo test -p xarchive-protocol -p xarchive-native-host` PASS (23 protocol + 8 Host tests); Extension Node tests PASS 35/35, `check` and `build` PASS; Desktop Node tests PASS 189/189 and `vite build` PASS; Draft 2020-12 schema metaschema plus positive/negative fixtures PASS for all four schemas; docs audit PASS; `git diff --check` PASS. WSL2 has no native Windows GUI/browser or Windows execution path; Windows validation is not run. The local `.venv` install is not committed as a project dependency.
- Not implemented: Desktop random-port listener and ticket issue/consume/invalidation; WebSocket Origin/auth/resource hardening; Native Host control-message forwarding to Desktop; Extension automatic state machine/UI/sender allowlist; Windows pipe/Registry/cold start/package; end-to-end Windows acceptance. AUTO-PAIR-TICKET/ORIGIN/EXT-LIFECYCLE remain `NOT_RUN / IMPLEMENTATION_NOT_READY`; Windows-specific rows are `BLOCKED` with manual procedure in [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md#l-browser-automatic-pairing--current-source-b015fbe).
- Next: continue cross-platform implementation in a dedicated complete Phase 2/3 batch. Windows implementation/validation must start only after an explicit commit+push Git handoff; this working tree does not satisfy `READY_FOR_WINDOWS`.
- No direct filesystem sync. Windows formal working tree remains Git-only.

## Incremental follow-up — Edge static review and channel fallback boundary (2026-10-02)

- Source branch/revision remains `dev` / `b015fbe81a0b47c2b486a5256bd81ac95fd98d25`; the whole automatic-pairing batch, including earlier uncommitted contract/schema work, remains uncommitted. Current owner: Cross-platform Owner. This is not a formal handoff.
- `extension/src/background.js`: explicit WebSocket selection no longer falls back to Native Messaging after connection/authentication failure, and a submitted request is never replayed across transports. Native remains selected only when WebSocket is not configured. Initialization/settings/reconnect errors now remain visible to callers.
- Added regression tests for intentional Native selection, authentication rejection, and no replay after response loss. Edge official-documentation static findings are registered in `docs/references/external-sources.md`.
- Linux evidence: Extension tests PASS 38/38; Extension `check` PASS; Extension `build` PASS; `git diff --check` PASS. WSL2 has no native Windows GUI/browser or Windows execution path, so Edge/Windows checks were not run.
- Escalation: `CROSS_PLATFORM_REVIEW_REQUIRED` — shared Extension transport-selection behavior changed without altering the transport abstraction. The broader pairing contract remains `CROSS_PLATFORM_CHANGE_REQUIRED` / in progress; no auto-pair runtime is implemented.
- Windows 11 + Edge extension ID/policy, MV3 worker restart/idle recovery, restart/sleep recovery and end-to-end archive remain unvalidated; retain existing `BLOCKED`/`NOT_RUN` queue states. No Windows Owner batch should start until the complete shared Phase 2/3 work is committed and pushed.

## Batch and revisions

- Task: reconcile the Windows `1f14cea` validation evidence, action the routed `CROSS_PLATFORM_REVIEW_REQUIRED`, and continue the planned Telegram wiring. [Plan](../development/telegram-local-bot-api-plan.md).
- Branch: `dev`. Input remote: `origin/dev`; Windows validation branch `origin/codex/windows-validation-1f14cea` fast-forwarded into this branch.
- Windows input/implementation/tested revision: `1f14cea6859dc1c0ecec164509579cfe4eb15f1a`. Windows implementation changes: none.
- This batch's implementation revision: the Git commit carrying the changes below. Handoff revision: the commit carrying this record. Cross-platform source and next Windows input: the same commit.
- Current owner: **Cross-platform Owner**. State: `CROSS_PLATFORM_IN_PROGRESS`.
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

**Windows Owner** for items 1-3 above and the still-open release/GUI items, then back to **Cross-platform Owner** for the remaining Telegram wiring (Tauri commands, settings and task UI, claim-loop scheduling) that keeps Batch A moving. `WINDOWS_BLOCKED` stays `COMPUTER_USE_UNAVAILABLE` where automation remains unavailable; manual procedures are in [manual steps](../validation/windows-manual-steps.md).
