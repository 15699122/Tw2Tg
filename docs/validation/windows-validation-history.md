# Windows Validation History

This document records completed Windows validation batches. It does not replace the current queue in windows-queue.md or the active state in ../status/platform-handoff.md.

## Recording template

### [date] [source revision]

- Owner: Windows Platform Owner
- Source branch/revision:
- Working tree included:
- Scope:
- Environment/toolchain:
- Canonical integration status:

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
|  | PASS / FAIL / BLOCKED / NOT_RUN / NOT_APPLICABLE |  |  |  |

### Errors and unresolved issues

- Issue:
- Classification: Windows-specific / shared / environment / test / unknown
- Blocker:
- Reproduction and evidence:
- Owner and next action:

Do not delete historical failures when a later handoff succeeds. Record the later result as a new batch and state whether the previous conclusion remains reusable.

### 2026-09-23 Windows E5–E7 implementation and Full package validation

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / input `bde6dc375bda68944b500714db8f9c39693bd06f`
- Working tree included: Windows E5 Named Pipe server, E6 current-user Native Host lifecycle, E7 status wiring, Full-package input-path fix, and the shared UI wiring pending Linux review.
- Scope: E5–E7 for the current U12/U17 Windows batch; Full package built from current Desktop/Native Host Rust source, current Sidecar Python source and repository Extension source. Existing local gallery-dl artifact was bundled as an external dependency.
- Environment/toolchain: Windows 11; x86_64-pc-windows-msvc; Cargo offline; Node 24.19.0; PyInstaller 6.22.3 / Python 3.12.14. Existing `target/release`, Sidecar artifact and prior validation artifacts were preserved; build and package outputs used new directories under `validation-artifacts/`.
- Local cache note: the PyInstaller build was invoked with `--clean`, which cleared its regenerable `%LOCALAPPDATA%\pyinstaller` cache. The project virtual environment, existing frozen Sidecar artifact, logs, and prior validation-artifact directories were not cleaned or replaced.
- Canonical integration status: Windows implementation committed and pushed; current-user browser registration and GUI/browser acceptance remain queued. Shared UI changes require `CROSS_PLATFORM_REVIEW_REQUIRED`.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Windows formatting | PASS | `cargo fmt --all -- --check` | Clean | None |
| E5/E6 Windows library tests | PASS | `cargo test -p xarchive-desktop --lib --target x86_64-pc-windows-msvc --offline` | 89 passed, 0 failed; includes two sequential Named Pipe framed requests with echoed request IDs | Cross-user ACL and packaged Native Host integration remain manual |
| Windows Release build | PASS | `cargo build -p xarchive-desktop -p xarchive-native-host --release --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-full-e5-e7-build-20260923` | Desktop and Native Host built from the implementation source into an isolated target directory | Native GUI launch is queued |
| Sidecar current-source build | PASS | PyInstaller spec `sidecar/pyinstaller/xarchive-downloader.spec`, separate `--distpath`/`--workpath` | Fresh one-dir worker built from `sidecar/src`; `_internal/python312.dll` included | Bundled worker handshake probe PASS |
| Full package assembly | PASS | `node desktop/scripts/build-portable-windows.mjs` with isolated Desktop, Native Host, Sidecar and output paths | `validation-artifacts/windows-full-e5-e7-current-source-20260923`; required files, Full manifest, Extension ID and host executable path/origin verified | GUI/Native Host-to-Desktop live interaction queued |
| Packaged Sidecar v2 contract | PASS | Pipe `hello`, unknown-field `extract`, then `shutdown` to packaged worker | `ready` with protocol v2, `INVALID_COMMAND`, clean exit 0 | None |
| Extension/Native Host package contract | PASS | `node desktop/scripts/extension-identity.mjs verify ...`; targeted package Node tests | Derived Extension ID matches bundled public key and `allowed_origins`; 11/11 package tests passed | Live Edge/Chrome registration/load queued |
| Frontend build | PASS | `npm run check --workspace desktop` | Vite production build completed (51 modules); existing dynamic/static import chunk warning only | None |
| Desktop Node full suite | BLOCKED | `npm test --workspace desktop` | One existing `wdio-tauri-service` process-termination test hit Windows `taskkill /T /F` `Access denied` in the restricted process-control environment; no application assertion failure was reported | Re-run outside restricted process-control environment; see `MANUAL-WIN-WDIO-TEARDOWN-01` |
| Full package GUI / browser status / registry lifecycle | BLOCKED | Computer Use inventory and launch attempt | Native app inventory empty; `computer.launch_app` unavailable in the Computer Use surface (`COMPUTER_USE_UNAVAILABLE`) | Complete `MANUAL-WIN-E5-01` through `MANUAL-WIN-FULL-01` in `windows-queue.md`; no unexecuted UI test is PASS |

#### Errors and unresolved issues

- Issue: packaged Full Desktop GUI, live HKCU registration and Edge/Chrome Extension connection have not been exercised.
- Classification: test/automation boundary; not a product failure based on current evidence.
- Blocker: `COMPUTER_USE_UNAVAILABLE`; native app launch surface is not exposed. HKCU/browser live operations were not performed.
- Reproduction and evidence: `cua.getState()` returned no native apps; launch attempt failed because `cua.computer.launch_app` was unavailable. Build/package/Sidecar probe and Windows Named Pipe unit tests were independently completed.
- Owner and next action: Cross-platform Owner reviews the shared UI wiring; Windows Owner runs the Manual Windows Validation Queue once native GUI automation or manual access is available.

### 2026-09-23 Windows E5–E7 follow-up validation

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / fetched handoff `26f8c37ac995cea7fce6667fcdee2d2966e6f77b`
- Working tree included: documentation-only Linux handoff update on top of the E5–E7 implementation at `2dcae6c9a04175d6aa7e2478d421b497934a039e`; all local dependencies, logs, and validation artifacts preserved.
- Scope: complete remaining non-GUI validation that can be run in this Windows environment; retry Computer Use once; reuse implementation/package validations because the fetched commit changes only `docs/status/platform-handoff.md`.
- Environment/toolchain: Windows 11; Node test runner; Computer Use native app inventory unavailable.
- Canonical integration status: Windows follow-up evidence and queue updated; docs commit to be pushed with this record.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| WDIO teardown lifecycle | PASS | `node --test desktop/test/wdio-tauri-service.test.mjs` | 8 passed, 0 failed; `killTree` terminated the test-owned child process with normal process-control access | Close `MANUAL-WIN-WDIO-TEARDOWN-01` |
| E5–E7 implementation/build/package/worker probes | PASS (reused) | Reuse the checks recorded in the preceding E5–E7 round | Fetched `26f8c37` changes only the handoff document; implementation, dependencies, package inputs, and contracts are unchanged | No rerun needed under validation policy |
| Computer Use retry | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | `cua.getState()`; reset session and retry once | First observation returned `apps: []`; retry returned `apps: []` and `Unable to load browser request-header policy` | Keep GUI/browser/registry items in Manual Windows Validation Queue |
| Full Desktop Node suite / full regression | NOT_RUN | Not rerun | Current fetched diff is documentation-only; the previously blocked teardown test was isolated and passed | No broad regression required for this batch |
| GUI, live HKCU registry, real browser Native Host connection, packaged Sidecar UI, cross-user ACL | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | Native-window observation/interaction unavailable | No product failure inferred; previous build, static package checks, worker protocol probe, and pipe loopback do not establish these outcomes | Manual queue remains with Windows Platform Owner |

#### Errors and unresolved issues

- Issue: live E5–E7 GUI, Registry, browser, and cross-user ACL scenarios remain unverified.
- Classification: automation boundary; no Windows product failure established.
- Blocker: native GUI inventory unavailable after one bounded retry; second observation also reported browser request-header policy initialization failure.
- Reproduction and evidence: current Computer Use retry returned no native apps; see the `2026-09-23 E5–E7 / current-source Full package` queue entries.
- Owner and next action: Windows Platform Owner completes `MANUAL-WIN-E5-01`, `MANUAL-WIN-E6-01`, `MANUAL-WIN-E7-01`, and `MANUAL-WIN-FULL-01` when native GUI automation or manual access is available. No Linux-owned follow-up remains.

### 2026-09-24 P1/P2/P3 account batch Windows validation

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / `dac0a153d03fa174c600435c87afabf476aded34`
- Working tree included: fetched remote handoff revision; tracked tree was clean before validation. Existing local dependencies, logs, prior artifacts, and ignored user directories were preserved. Build outputs used fresh paths below `validation-artifacts/windows-account-batch-20260924/`.
- Scope: affected Windows-target Rust tests; Sidecar, Desktop, Extension, schema, Vite and frozen worker checks; isolated Tauri/Native Host release builds; Full and Core package assembly; static package contract checks; bounded Computer Use attempt.
- Environment/toolchain: Windows x64, `x86_64-pc-windows-msvc`, Cargo offline; Node.js 24.19.0; Python 3.12.14 / PyInstaller 6.22.3; existing `.venv-windows-validation`; local gallery-dl and aria2 artifacts. Ruff was not available in the preserved virtual environment and was not installed.
- Canonical integration status: no Windows production-code change was required. Windows validation results apply to source revision `dac0a153`; this history entry and queue update are committed separately below.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Affected Windows Rust modules | PASS | `cargo test -p xarchive-core -p xarchive-download -p xarchive-protocol -p xarchive-sidecar-supervisor -p xarchive-storage -p xarchive-telegram -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-account-batch-20260924/target --quiet` | 219 passed, 0 failed across Core 18, Desktop 102, Download 21, Transfer Driver 7, Protocol 19, Supervisor 6, Storage 34, Telegram 12; includes Windows Named Pipe loopback test | Live GUI/ACL/Host integration remains in queue |
| Native Host Windows tests | PASS | `cargo test -p xarchive-native-host --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-account-batch-20260924/target --quiet` | 8 passed, 0 failed | Live registration and browser connection remain queued |
| Sidecar Python tests | PASS | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests -p no:cacheprovider --basetemp validation-artifacts\windows-account-batch-20260924\pytest-temp` with `PYTHONDONTWRITEBYTECODE=1` | 32 passed, 0 failed | Ruff `NOT RUN`: executable not installed in the preserved venv |
| Desktop Node tests | PASS | `npm test --workspace desktop` | 92 passed, 0 failed | No full regression rerun; relevant workspace suite passed |
| Extension Node tests | PASS | `npm test --workspace extension` | 21 passed, 0 failed | Live Edge/Chrome load remains queued |
| Vite frontend build | PASS | `npm run check --workspace desktop -- --outDir ../validation-artifacts/windows-account-batch-20260924/desktop-dist` | 52 modules built into isolated output; existing `desktop/dist` preserved | Known dynamic/static Tauri API import warning only |
| Modified schema JSON | PASS | Node `JSON.parse` over all three modified schema files | All three parsed | None |
| Current-source frozen Sidecar | PASS | PyInstaller `sidecar/pyinstaller/xarchive-downloader.spec` with isolated dist/work paths; run packaged `--help`, protocol-v2 handshake, invalid field and shutdown probes | `hello`/`ready` included `account_discovery`; unknown field returned `INVALID_COMMAND`; shutdown exit 0. Probe: `validation-artifacts/windows-account-batch-20260924/sidecar-probe.json`. Worker SHA-256: `CC7B65F3CD7CA32441F1FFD9291B290D7F7523E0CE7A394B3B3B9E5FD51EF114` | Real account/gallery-dl extraction remains blocked pending controlled session |
| Isolated Tauri release build | PASS | `node ..\node_modules\@tauri-apps\cli\tauri.js build --config ..\validation-artifacts\windows-account-batch-20260924\tauri-validation.json --target x86_64-pc-windows-msvc --no-bundle` with isolated Cargo target and frontend dist | `validation-artifacts/windows-account-batch-20260924/target/x86_64-pc-windows-msvc/release/xarchive-desktop.exe`, 19,002,880 bytes | GUI launch/first-run remains blocked by Computer Use surface |
| Isolated Native Host release build | PASS | `cargo build -p xarchive-native-host --release --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-account-batch-20260924/target` | Release executable built in isolated target | Live HKCU install/repair/unregister remains queued |
| Full portable package | PASS (assembly/static contract only) | `node desktop/scripts/build-portable-windows.mjs` using isolated app, Native Host and worker paths, derived Extension ID `iaajefkoanbkleojofoadeakelihbjne`, output `validation-artifacts/windows-account-batch-20260924/full-package-r1` | Required app, worker, `python312.dll`, gallery-dl, Extension and Native Host files present; manifest, Extension ID, Native Host executable path and allowed origin match | No package GUI launch or Edge/Chrome registration/connection claim |
| Core portable package | PASS (assembly/static boundary only) | Same package script with `PORTABLE_PACKAGE_TYPE=core` and output `validation-artifacts/windows-account-batch-20260924/core-package` | Core manifest says `package_type=core`, `gallery_dl_bundled=false`, Extension not bundled, Native Host null | No launch claim |
| Computer Use / account GUI | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | Inventory showed `apps: []` and Edge browser tabs; attempted `cua.computer.launch_app` for isolated Full package | `launch_app` was undefined in the available Windows Computer Use surface. No native app was opened. No account request or registry change was attempted. This is not a product failure. | `MANUAL-WIN-BATCH-01`, `02`, `04`, `MANUAL-WIN-E5-01` through `E7-01`, and `FULL-01` remain in `windows-queue.md` |
| Real account discovery/archive, credential redaction, pause/resume/retry/restart, Windows filesystem fault/ACL/reparse, release signing/upload | BLOCKED / NOT_RUN | Not attempted | Requires a controlled account/browser session and GUI for account cases; requires controlled filesystem fault fixtures and actual signing/release target for the other cases | Keep `MANUAL-WIN-BATCH-02` through `05` open; no PASS inferred from synthetic tests or package builds |
| Full regression | NOT_RUN | Not run | Risk-based scope covered changed crates and affected Desktop/Extension/Sidecar modules, package build and worker contract. No dependency overhaul/release operation was part of this validation batch. | Reassess if remaining platform evidence reveals a shared defect |

#### Errors and unresolved issues

- Issue: native GUI, live browser/Native Host/Registry, real controlled-account extraction, and Windows filesystem fault scenarios are not validated.
- Classification: environment/automation prerequisites; no Windows-owned or shared-contract product failure was established.
- Blocker: Computer Use returned no native apps and exposed no `launch_app`; real account credentials/session, signing, and release target were not supplied or used.
- Reproduction and evidence: Computer Use inventory and launch attempt recorded above; static package evidence and isolated build outputs are under `validation-artifacts/windows-account-batch-20260924/`.
- Owner and next action: Windows Platform Owner completes the open queue when manual GUI access, controlled account data, and applicable release fixtures are available. `CROSS_PLATFORM_CHANGE_REQUIRED`: none. `CROSS_PLATFORM_REVIEW_REQUIRED`: none.

### 2026-09-24 Full package manual GUI and account-batch follow-up

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / `dac0a153d03fa174c600435c87afabf476aded34`.
- Package: `validation-artifacts/windows-account-batch-20260924/full-package-r1` (Full package assembled from the source revision above).
- Test date: 2026-09-24. Evidence consists of user-provided screenshots and observations; screenshots are not copied into the repository.
- Privacy: account names, Tweet IDs, and batch identifiers are intentionally omitted/redacted. No account-identifying values are reproduced in this record.
- Scope: manual launch/exit, Sidecar state, Extension/Native Host connection, archive-button task propagation, account-batch display and pause/cancel behavior, and path handling.
- Environment limitations: Windows build and Edge/WebView2 exact versions were not recorded in this manual follow-up. No successful extraction/download occurred, so downstream file and batch acceptance could not be exercised.
- Source changes: none. This manual report supplements, and does not replace, the automated results already recorded for `dac0a153`.

| Item | Status | Observation | Follow-up / limits |
|---|---|---|---|
| Application launch and clean exit | PASS | Application launched and closed normally. After closing, `Get-CimInstance Win32_Process -Filter "Name = 'xarchive-desktop.exe'"` returned no remaining application process. | This is a user-observed manual result for this package. |
| Sidecar start, stop, and service indicator | PASS | Sidecar started and stopped; the lower-left service status indicator reflected the service state. The UI showed the Sidecar ready state during the successful run. | Does not establish successful real account extraction or download. |
| Account archive page rendering | PASS | Account archive page opened and rendered its controls and batch panel. | Batch discovery/processing results are separately recorded below. |
| Extension initial load and recognition | PASS | Edge loaded the Extension and the application initially recognized it as connected. | Later post-restart connection recovery failed; see below. |
| Extension archive action and task visibility | FAIL | Clicking the archive action on a Tweet did not immediately show a task in the main UI. Refreshing the main UI made the task appear, after which it showed download failure. The X page action changed to a retry state. | Delayed propagation and the failed download require diagnosis. Account name and Tweet ID omitted. |
| Download after manually starting aria2 | FAIL | Manually starting an aria2 process did not make the failed task download successfully. The settings page also displayed aria2 as not detected in a screenshot. | The UI state is a diagnostic clue only; this record does not assert aria2 detection as the root cause. No downloaded file was produced for inspection. |
| Extension reconnect after application restart | FAIL | After restarting the application, the UI showed Extension disconnected. Refresh, Native Host repair, unregister, and repair again did not restore the connection. | Explicit registry-path/value inspection and Native Host pipe-request tracing were not performed. |
| Account-batch discovery progress | FAIL | A batch remained at zero candidates and zero pending submissions for an extended period. The user then paused it; discovery still displayed `RUNNING` for about 30 seconds, after which the user manually cancelled the batch. | Batch identifier omitted. Resume, retry, and restart-recovery behavior were not tested. The observed delay is not a measured service-level threshold. |
| Chinese path handling | PASS | Chinese path was used successfully. | Long paths and cross-volume operations were not tested. |
| Downloaded file content, size, and checksum | NOT RUN | No successful download was produced. | Requires a working extraction/download flow and a controlled expected output. |
| Batch records and successful multi-item processing | NOT RUN | No normal batch completion was possible because discovery/download did not progress successfully. | Requires working account discovery and download. |
| File lock, abnormal exit, staging cleanup | NOT RUN | Not exercised. | Requires controlled Windows filesystem/process fixtures. |
| Credential/identifier leakage inspection | NOT RUN | No systematic log/database/package inspection was performed for this manual follow-up. | Any future evidence must continue to redact account names and Tweet IDs. |
| Native Host registry assertions and pipe ACL/cross-user access | NOT RUN | Repair actions were attempted, but registry values, exact executable registration, pipe requests, and cross-user ACL behavior were not inspected. | Requires targeted registry/pipe verification. |
| Signing and release gate | NOT RUN | No signing, release gate, or upload was performed. | Requires the release signing environment and release target. |
| Other filesystem acceptance | NOT RUN | Long path, cross-volume move, file lock, abnormal exit, and staging cleanup were not tested. | Keep each as a separate acceptance item. |

#### Triage and ownership

- `CROSS_PLATFORM_CHANGE_REQUIRED`: investigate account discovery/candidate progression, delayed task visibility from Extension to Desktop, and pause-to-terminal-state propagation. The evidence identifies failures at shared workflow boundaries, but does not establish a root cause or prove that a shared contract change is required; Linux should triage and decide the implementation owner after diagnosis.
- Windows-owned follow-up remains: investigate and retest Native Host/Extension reconnection after Desktop restart, including Windows registration and process/pipe lifecycle.
- Existing automated PASS results at source revision `dac0a153` remain valid for their recorded test scope; they do not override the manual FAIL results or imply successful account extraction/download.
