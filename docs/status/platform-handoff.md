# Current Platform Handoff

Status: `CURRENT`. Previous shared/Windows implementation checkpoints are preserved in [history](platform-handoff-history.md).

## Revision and owner

- Branch: `codex/browser-automatic-pairing-windows`.
- Shared source/handoff: `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e` on `dev`; original input `82a0df75d3d1a4a223a2caaf5df0bbbd86de0164`.
- Windows implementation and tested source: `d65a01bfe2c6f93a071322bfef154fd947b0b70b`, pushed to origin.
- Evidence/documentation return revision: the commit containing this record; before commit/push documentation remains uncommitted. This return does not imply integration into `dev`.
- Current owner: Windows Platform Owner; shared follow-up owner: Cross-platform Owner in WSL, after Git fetch/review.
- State: `CROSS_PLATFORM_REVIEW_REQUIRED` plus `WINDOWS_VERIFICATION_PENDING`. No `WINDOWS_PASS` for the complete feature.
- Source diff is empty after the build's Cargo.toml timestamp/line-ending refresh; existing untracked caches, Full directories, screenshots and user data are preserved. No direct sync.

## Windows results

Windows Named Pipe bootstrap/coordinator wiring, fixed origin registration checks, bounded I/O and worker cleanup are implemented. Native targeted tests and Host subprocess framing passed; full Desktop run retained 186/187 plus one sandbox config AccessDenied, whose unchanged targeted test passed outside sandbox. Initial product failures and repairs remain in [Windows history](../validation/windows-validation-history.md).

Current Full package was freshly assembled at `dist-portable/auto-pair-d65a01b` using release Host build and canonical portable script (no binary reuse). Desktop SHA-256 `bab1e19b394ec61d271f70438143e1c3a27a0c9e14a43e9373ac90f50184cd7a`; Host `a8f39f2a742d32eac0a3da674a10970883a01203f982d12c100f5a18d882f8bb`. Existing worker/dependency inputs were copied; this is a local dev-channel package, not release publication or a fresh worker build.

Native GUI startup and portable download setup PASS. Managed current-user registration was repaired through the Desktop GUI to the new Host; actual-user Registry read confirmed the new path. Earlier sandbox HKCU ABSENT results do not describe the interactive user's Registry: actual user had an older registered Host. Both observations are preserved as an environment distinction.

Owner manually loaded Extension and supplied four screenshots: settings/popup WebSocket authenticated on 61054; Desktop authenticated and `connected`, accepted/authenticated 1, close-after-auth 0; X/Twitter popup exposes page archive button. PASS for initial pairing and page availability only. Controlled 5s/45s idle, Desktop/browser/worker restart, query, actual archive, duplicate submission, multi-profile, cross-user ACL and uninstall/cold-start acceptance remain open. Screenshot collection timestamps do not prove idle duration or manual-input-free discovery.

Computer Use can automate native Desktop; browser internal tab claiming was rejected, and native Edge observation was stopped because the tool could not confidently determine its URL. This is `COMPUTER_USE_UNAVAILABLE` for that automated path, not product FAIL. Manual loading/pairing evidence is recorded separately; do not bypass the restriction.

## WSL follow-up (do not implement in Windows batch)

1. `CROSS_PLATFORM_REVIEW_REQUIRED`: review `d65a01b` shared WebSocket stop-aware wrapper and Windows polling; Linux targeted tests/fmt/Clippy, then integrate through Git. No protocol/schema redesign or Phase 1 repetition.
2. `CROSS_PLATFORM_CHANGE_REQUIRED`: align shared Desktop sidebar/header with authenticated live WebSocket. The screenshots show WebSocket `connected` while the sidebar says disconnected. Code reads `browser_connection` (short Native IPC activity) rather than `websocket_connection`; Native bootstrap completion is not loss of the persistent WebSocket. Update shared state mapping and targeted UI tests; preserve separate Native registration/IPC diagnostics.
3. Finish shared automatic-mode UI in `extension/options.html`, `options.js`, popup and Desktop load guidance. Remove production token-copy and fallback promises; represent bootstrap/Host/Desktop/version/recovery failures honestly. Keep explicit diagnostic legacy mode separate. Current runtime no-fallback must remain intact.
4. Audit package installation inventory: `browser-pairing.js` is actually shipped and imported by background, but installation manifest's file list omits it. Correct the shared inventory/contract and its targeted package test; do not rewrite the built artifact as evidence of a corrected build.
5. Freeze/test legacy/new Extension/Desktop compatibility and failure matrix before rerunning affected Windows acceptance. Additional shared diagnostics only where current evidence cannot distinguish worker reconstruction, stale status and real close.

Windows remaining acceptance and reproducible manual steps are in [queue](../validation/windows-queue.md) and [manual steps](../validation/windows-manual-steps.md). Normal pending Windows checks do not block WSL development. Telegram remains a separate deferred scope; no unrelated implementation or release is authorized by this return.
