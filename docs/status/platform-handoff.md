# Current Platform Handoff

Status: `CURRENT` — browser automatic-pairing reconciliation after the Windows batch. Prior records are preserved in [handoff history](platform-handoff-history.md) and [Windows validation history](../validation/windows-validation-history.md).

## Batch and revisions

- Task: reconcile Windows automatic-pairing evidence, close independently fixable shared packaging gap, then return for Windows revalidation.
- Branch: `dev`; source baseline: `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e`. Windows implementation/evidence commits: `d65a01bfe2c6f93a071322bfef154fd947b0b70b` and `30fc575` on `origin/codex/browser-automatic-pairing-windows`.
- Handoff revision: pending this Linux batch commit and push. Working tree is currently modified; it is not yet a formal handoff revision.
- Current state: `CROSS_PLATFORM_IN_PROGRESS`. A shared Extension package required-file guard/test correction is being prepared. Windows evidence reports a Full-artifact UI status inconsistency (`FAIL`) whose authoritative state source is not yet determined; do not change semantics without reproducing and identifying the conflicting values.
- Current owner: Cross-platform Owner until the package fix, review, validation, documentation, commit and push are complete. Next owner: Windows Platform Owner for exact-revision package/UI reproduction and outstanding Windows checks.
- No direct filesystem sync. Preserve Windows E: caches, artifacts and user data.

## Shared work completed (source baseline `0c74087`)

- Desktop defaults to `127.0.0.1:0`, publishes the actual endpoint and random runtime generation; only the diagnostic port override remains. Legacy token authentication is retired.
- Listener-scoped CSPRNG tickets: 256 bits, 30-second TTL, atomic single consumption, exact fixed Extension origin, shutdown/generation invalidation, 64-ticket capacity and 100ms issuance interval.
- Exact single Origin and `/` resource; no query credentials. First frame consumes the ticket before business. Absolute three-second handshake/auth read deadline, 16KiB handshake/4KiB auth budgets, 1MiB frame/message limit, 32 workers and 64 admissions/second. Shutdown interrupts idle and handshake sockets and reaps workers.
- Native Host shared framing/control forwarding validates response/request identity and redacts malformed control errors. Unix Host control entry checks browser origin and forwards to current-user Desktop Unix IPC; real subprocess framing regression and IPC→WebSocket authentication pass.
- Extension defaults to automatic bootstrap, single-flight connect, generation cancellation, strict control/auth responses, bounded retries, pending cleanup, authenticated live-socket status and worker reconstruction. Credentials are memory-only; automatic success clears legacy stored token. Native business mode is explicit; failures never replay across channels.
- Sender ID/URL/payload allowlist separates trusted popup/options management from X/Twitter content requests.

## Validation and evidence

Environment: native Windows Codex shell orchestrating Ubuntu WSL2 `Linux 6.18.33.2-microsoft-standard-WSL2`; Rust/Cargo 1.98.0; Node 26.7.0. Tests bind to baseline above plus this commit's reviewed changes. Test builds are local debug artifacts, not packaged GUI evidence.

| Scope | Result | Evidence |
|---|---|---|
| Linux targeted Desktop Node | PASS, 27/27 | `node --test desktop/test/extension-package.test.mjs desktop/test/ui-state.test.mjs`; includes required-helper missing-file regression. |
| Documentation integrity | PASS | `node scripts/docs-audit.mjs`; `git diff --check`. |
| Windows Host/Named Pipe targeted | PASS, scoped | `d65a01b` / evidence in Windows history: Host 11+1+1, Pipe/Registry targeted 4, WebSocket targeted 12. Desktop module is 186/187 in sandbox; one unchanged config persistence test failed `AccessDenied`, then passed with permitted escalation. This is not a clean full-module PASS. |
| Windows Full artifact automatic-pairing UI | FAIL, scoped | Sidebar/detail status inconsistency reported; exact conflicting runtime fields still need capture and root-cause reproduction. |
| Windows Extension package | FAIL, scoped | Tested Full artifact omitted required `src/browser-pairing.js`; Linux batch adds a package guard and regression test. Windows package/install acceptance remains open. |
| Real browser automation | BLOCKED | `COMPUTER_USE_UNAVAILABLE`; tooling/environment blocker, not product failure. Retry in an interactive browser session. |
| Real archive/duplicate, idle, profiles, restart, registration/ACL, packaging/install | NOT_RUN | Remain open for Windows Owner on the exact returned revision/artifact. |

Earlier shared Phase 2/3 implementation evidence remains documented in the preceding historical handoff record; it is not rerun evidence for this reconciliation. The current Linux follow-up makes no protocol/schema changes and no UI state-authority change. No Windows test is inferred from Linux results.

Environment for this follow-up: Linux workspace `/home/shiraishi/VSCode Workspace/Tw2Tg`; Node test commands above, no browser or real account needed. No release is approved. The formal Windows return handoff is pending commit and push.

## Windows work and validation required

Windows Named Pipe bootstrap is implemented in `d65a01b`; Linux review/revalidation of the Windows stop-aware shared `DeadlineStream` change remains required after Git integration (`CROSS_PLATFORM_REVIEW_REQUIRED`). Do not transfer Windows native behavior evidence into Linux results.

Rebuild the Full artifact from the returned handoff and bind source SHA plus Desktop/Host/Extension hashes. Reproduce the sidebar/detail mismatch while recording redacted `browser_connection`, `websocket_connection`, `websocket_authenticated`, and both rendered states; then route the confirmed shared semantic correction to Linux if needed. Verify package inventory includes `src/browser-pairing.js` and execute the actual install/package check.

Outstanding Windows checks: current-user ACL/registration, Edge/Chrome and profiles, real browser bootstrap/query, real authorized archive and duplicate behavior, response-loss recovery, restart/worker/sleep and idle timing, package/install/upgrade, and log/diagnostic redaction. Browser automation was blocked by `COMPUTER_USE_UNAVAILABLE`; archive, duplicate, idle, profile and restart cases are `NOT_RUN`, not PASS.

## Risks and expected behavior

Old Extension token frames are rejected by the new Desktop. Explicit legacy mode only targets old diagnostic Desktop deployments; there is no automatic downgrade. New Extension against old Host yields a visible bounded control failure. Automatic bootstrap is implemented in the shared Extension; Windows Full-artifact UI status consistency and ordinary package installation remain open follow-ups, not grounds to revert the UI to manual-token semantics.

Fixed Extension identity is `iaajefkoanbkleojofoadeakelihbjne`, derived from the committed manifest key. WebSocket origin excludes the trailing slash; Native Messaging invocation origin includes it. Windows cross-user/ACL and browser-origin behavior are still unverified.

WQ-WS-02/03 post-auth persistence/controlled 5s/45s results remain historical and unresolved; automatic-pairing Linux tests do not close them. Real archive/duplicates remain paused pending reviewed Windows pairing behavior. Telegram work and release publication are outside this batch.
