# Desktop Settings Panels and Telegram Presentation Plan

Owner: Linux Cross-platform Owner (shared Desktop React/CSS and tests); Windows Platform Owner (native WebView2, DPI, keyboard and visual acceptance).
Status: `READY_FOR_WINDOWS` — implemented from source revision `c8f63c91ab0f8915e9a61713e2af65d98d7ab5df`. Desktop Node tests 197/197 PASS; `npm run check` and documentation audit PASS; `git diff --check` clean. Formal revision is recorded in `docs/status/platform-handoff.md`. Windows GUI evidence is not implied by this plan.

## 1. Scope

Address the screenshot findings: normalize Telegram typography/form layout, add a Telegram icon matching the existing component icon treatment, reduce the Extension-to-footer-separator gap, correct the Sidecar JSONL description to match protocol v2 and the gallery-dl extraction-only/aria2 transfer architecture, and group settings in accessible collapsible panels.

This is a presentation and copy change. It does not change Telegram commands, settings persistence, credentials, send semantics, Sidecar protocol, aria2 transfer behavior, or Windows-native integration.

## 2. Implementation

- A shared `SettingsSection` disclosure card is used by Core Bootstrap, Sidecar, Windows-only aria2, Extension, Telegram, storage, logging and proxy groups. Each has an independent expanded state; contents remain mounted but use `hidden` when closed, so drafts and component state survive toggling while closed controls are removed from keyboard navigation.
- Sidebar SQLite, Sidecar and Extension status actions expand their matching section before scrolling/focusing its disclosure control. Storage has a fallback focus target because it renders inside a section component.
- Telegram fields use the existing system type stack and shared field sizing, explicit label associations, responsive two-column/one-column layout, separated status and action groups, and a monochrome 20px paper-plane icon in the existing 38px component-icon frame. Credential and real-send confirmations remain intact.
- Sidecar description now states JSONL line framing and protocol v2 commands/events, gallery-dl metadata/URL extraction only, and aria2 media transfer. aria2's summary now describes its actual media-download role.
- `.sidebar-footer-separator` top spacing changes from 16px to 8px; its bottom spacing and the service rows remain unchanged.

## 3. Linux checks

Linux validation: `npm test` in `desktop/` 197/197 PASS; `npm run check` PASS (Vite reports the existing mixed static/dynamic `@tauri-apps/api/core.js` chunk warning); documentation audit PASS; `git diff --check` clean. These checks validate source/build and static wiring, not Windows rendering or accessibility.

## 4. Windows acceptance required

On the exact artifact identified in the handoff, inspect 100%, 125%, 150% and 200% DPI; narrow and wide window widths; expansion by mouse and keyboard; focus visibility; hidden-control tab order; all three sidebar status jumps; long descriptions/paths and error states; Extension gap; and Telegram form layout, password masking and disabled/busy states. Confirm refresh controls do not toggle their panel and collapsing does not trigger backend actions. Record Windows GUI outcome separately from Linux checks.

## Windows return — 2026-10-05

Exact aa8dabd native builds and targeted automated checks passed; acceptance remains WINDOWS_VERIFICATION_PENDING. Packaged queue fixture startup and new settings GUI matrix remain pending. Release trusted digest and job isolation static review did not meet criteria; implementation and isolated Actions remain pending. Current evidence, hashes and Owner routing are in ../validation/windows-validation-history.md and ../status/platform-handoff.md.


## Windows continuation — 2026-10-05 / 39f54e5

SEC-A trusted Windows asset pins and isolated read-only build / separate publish job are implemented. Negative fixtures and isolated Windows Actions run 37255273960 PASS; full production upload=false rehearsal and CodeQL evidence remain pending. Packaged queue control/worker-exit cases and scoped current-size settings GUI checks now PASS; subtree cleanup and full settings matrix remain open. Evidence and formal Owner routing: ../validation/windows-validation-history.md and ../status/platform-handoff.md. Real Telegram testing deferred by Owner for this local/CI batch.

## Cross-platform reconciliation — 2026-10-05

Windows return `52d8bc5` was Git-integrated at merge `b937b5b`; see the current
handoff and Windows history for revision-bound GUI evidence and exact remaining
acceptance. Scoped current-size mouse/form/navigation observations remain PASS;
the full DPI, keyboard/focus, Extension, busy/error and backend-side-effect
matrix remains `WINDOWS_VERIFICATION_PENDING`. Linux Node/build checks do not
replace that Windows acceptance.

## Windows ca25e53 return — 2026-10-05

47/47 targeted tests and fresh native build PASS; current-size/maximized rendered copy, spacing/icon consistency, Sidecar Space/Enter, Extension navigation and refresh independence PASS scoped. Full GUI acceptance remains WINDOWS_VERIFICATION_PENDING; exact artifact/revisions, tooling limitations and remaining manual matrix are in latest Windows history and queue. Next execution Owner: Windows Platform Owner; Cross-platform Owner reconciles Git documentation return.
