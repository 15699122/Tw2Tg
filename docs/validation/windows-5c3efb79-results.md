# Windows 5c3efb79 validation return — 2026-10-06

## Identity and capability

- Input/handoff: `5c3efb79d2c7ffe3c84fe1f33821cd94b6c77a49`, received after `git fetch origin`, branch `cross-platform/automatic-pairing-reconcile-20261002`. This includes merge `344ca10` and accepted shared repairs `8f7fde6` / `8e0169a`.
- Windows return branch: `codex/windows-validation-5c3efb79`.
- Windows implementation and validation revision: `44e60e369b5c57d3ed66b46fb610dc906a45780f`. Documentation return revision is the commit containing this record, discoverable through branch HEAD; no self-referential SHA is embedded.
- Native Windows / NTFS E: checkout, PowerShell 7.6.6, Rust 1.98.0, Node 24.19.0, Python launcher 3.14.7; Sidecar tests explicitly use `.venv-windows-validation/Scripts/python.exe`. Network, loopback HTTP, WinHTTP and WebView2 GUI available. Default sandbox shell initialization failed; permitted escalated noninteractive commands produced the evidence. No filesystem sync was used.
- Input tracked tree clean. Return tracked scope: one Windows-only adapter plus status/validation/Plan documents. Existing untracked dependencies, packages, `.codex`, logs and user/manual artifacts were preserved. Local evidence lives under `validation-artifacts/windows-5c3efb79/` and is not committed.
- Requested cross-platform-validation §16 is NOT_APPLICABLE: that file is navigation-only. Followed the authoritative ownership/validation rules and the revision template in `git-platform-handoff.md` §16.

## Windows implementation

Only `desktop/src-tauri/src/system_proxy_resolver.rs` changed. A configured PAC URL or WPAD inspection failure is retained independently of a successfully loaded script. The existing shared fail-closed predicate remains unchanged. Loaded sources preserve DHCP/DNS/configured precedence; explicit WinHTTP PAC evaluation exposes execution/download errors instead of accepting the upstream static/DIRECT fall-through. Environment overrides are considered only for a valid proxy applicable to the destination scheme. Descriptions retain failed configured PAC URLs with credentials redacted and report the selected inspection state. Upstream evaluation error text is not exposed.

No shared abstraction, schema, protocol, helper or dependency changed. Five Windows tests cover unloaded configured failure, WPAD error/source selection, environment scheme precedence, real WinHTTP explicit-source PAC results, and HTTP-client/child refusal boundaries. These are bounded native fixtures, not OS Registry/WPAD discovery or end-to-end transport acceptance.

## Executed results

| Check | Result | Evidence and limit |
|---|---|---|
| Native PAC targeted tests | PASS | Real `OfficialResolver::evaluate_pac_source` with loopback HTTP PAC sources: deliberate DIRECT for `https://fixture.invalid/resource`, throwing script, HTTP 404; five adapter tests included in final module run. WPAD snapshot selection is an injected configuration test, not actual DHCP/DNS discovery. |
| HTTP client / child boundary | PASS scoped | Throwing PAC for non-loopback fixture destination yields no candidates and refuses child routing. Deliberate DIRECT positive control sends/reads real loopback HTTP payload. This does not prove packet-capture zero-egress or the whole download/Telegram/aria2 call chain. |
| Desktop module | PASS | `CARGO_INCREMENTAL=0`, explicit `PYTHON`, `cargo test -p xarchive-desktop --lib --locked`: **257 PASS, 1 ignored**, log `desktop-module-final.log`. Ignored Credential Manager native create/read/delete test remains NOT_RUN. |
| Initial broader run | FAIL retained | `desktop-module.log`: 253 PASS, 4 FAIL, 1 ignored. Three discovery tests lacked an executable Sidecar Python configuration and passed after explicit PYTHON; do not assign a product repair to this tooling prerequisite. Fourth failure was the loopback PAC assertion described below. |
| Native debug build | PASS | From exact implementation revision: in `desktop`, `node ../node_modules/@tauri-apps/cli/tauri.js build --debug --no-bundle`; Vite build + native executable succeeded. `native-build.log`; existing unused-function/linker warnings retained. This is debug Desktop, not release/Full/package/installer acceptance. |
| Native startup and Settings | PASS scoped | Fresh isolated root, selected returned native window from the exact new executable. Settings navigation, collapsed panels, network proxy expansion and current system configuration summary rendered at 1082×790 capture size; no `proxySystem is not defined`. Summary showed Windows backend, unconfigured PAC, static/environment/bypass fields. No system settings or credentials changed. |
| GUI automation recovery | PASS scoped | Viewport-outside UIA clicks were tool failures, recovered by fresh observation/scroll and screenshot-derived click. The prior batch's COMPUTER_USE_UNAVAILABLE remains historical, not the current capability. |
| Documentation audit / whitespace | PASS | `node scripts/docs-audit.mjs`, final `git diff --check` before documentation commit. |

Fresh debug executable: `validation-artifacts/windows-5c3efb79/app/xarchive-desktop.exe`.
SHA-256: `660388A7F828E771B8B8B851A238177F3C6CA6210A9CF7852D5616278C4174E8`.
Isolated GUI root: `validation-artifacts/windows-5c3efb79/gui-root`; process identity recorded in `gui-pid.txt`.

## Remaining integration risk

The initial negative probe used a loopback destination. WinHTTP returned DIRECT for a throwing script, failing the refusal assertion. With a non-loopback `fixture.invalid` destination the same native throwing PAC was refused. This demonstrates that an explicit-source successful DIRECT result alone does not establish that WinHTTP executed the script for every destination: implicit local bypass needs a separate acceptance decision. The passing test was correctly scoped to a non-loopback destination; the original FAIL is retained. No Registry-bound product-level reproduction or universal fail-closed claim is made. WQ-PROXY-15/16 remain pending with this risk, rather than closing from the fixture PASS.

## Manual Windows queue and prerequisites

| ID | State / owner | Concrete next execution |
|---|---|---|
| M13 / WQ-PROXY-15/16 | NOT_RUN; Windows Platform Owner | Isolated Windows test user/VM and controlled PAC/WPAD HTTP + DHCP/DNS fixtures, no inherited scheme proxy override. Bind Registry configuration and artifact SHA; test missing/throwing PAC, discovery failure, valid DIRECT, static fallback, source precedence and localhost/loopback. Capture actual destination/proxy request counts or packets and verify HTTP/Telegram/download/child refusal. Restore fixture settings. Fixture tests do not close this row. |
| M10/P6 / WQ-PROXY-11/17, WQ-DL-06 remainder | NOT_RUN; Windows Platform Owner | Same fresh binary; full 100/125/150/200% DPI/narrow/maximized, visible Tab focus, Space/Enter, hidden panel Tab exclusion, refresh independence, busy/error/backend effects, download-mode save/restart. Current-size Settings render subset passed only. |
| WQ-PROXY-02..18 remaining routes/security | NOT_RUN; Windows Platform Owner | Controlled static/environment/manual/PAC proxy servers with actual route observation, ordered candidates/SOCKS, auth and credential-sink sweep; isolated OS configuration and redacted evidence. WQ-PROXY-01 new build/startup subset passed only. |
| WQ-DL-01..08 / Full / real download | NOT_RUN; Windows Platform Owner | Fresh Full package from this revision with worker/gallery-dl/aria2 inventory, test account/cookies outside evidence, completed archives/SHA, process-tree cancellation/recovery/reparse and actual transfer routes. Old independent-worker results are not promoted to this build. |
| Full workspace / Credential Manager ignored test | NOT_RUN; Windows Platform Owner | Full scope deferred after changed Windows adapter/module checks; explicit isolated native Credential Manager fixture required for ignored mutation test. |
| Automatic browser pairing and external-service acceptance | NOT_RUN; Windows Platform Owner | Fresh Full Desktop/Native Host/Extension identities, real browser restart timeline and test Telegram/X services. Prior FAIL/NOT_RUN remain preserved. |

## Git return and owner

State: `WINDOWS_VERIFICATION_PENDING`. Next Owner: **Cross-platform Owner** for Git integration/review of the Windows-owned adapter and documented WinHTTP implicit-bypass risk. No new shared implementation was changed, so no shared-repair approval is claimed. If the implicit-bypass acceptance decision requires changing the shared fail-closed contract, route it as `CROSS_PLATFORM_CHANGE_REQUIRED` before implementing that shared change. Windows Platform Owner retains the native integration/manual/Full matrix after reconciliation. No Windows-blocking interruption of unrelated Linux work is requested.
