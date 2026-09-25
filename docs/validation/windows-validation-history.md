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

### 2026-09-24 Windows batch revalidation at Linux handoff `7ed02b5`

- Owner: Windows Platform Owner
- Branch/source revision: `feature/u7-desktop-production-integration` / `7ed02b5e94e17171e26ae75000b555932e166216`.
- Working tree: canonical Windows branch fast-forwarded to the fetched handoff revision; tracked files were clean before this documentation closeout. Existing local dependencies, logs, manual-validation files, and prior artifacts were preserved. New generated output is isolated under `validation-artifacts/windows-batch-revalidation-7ed02b5/`.
- Scope: current-source Windows-target affected Rust tests, Native Host tests/build, Sidecar discovery tests, targeted Desktop/Extension tests, Vite build, frozen worker protocol probe, Tauri/Native Host release builds, Full package assembly/static contract, fixed-runtime dashboard WDIO attempt, and bounded Computer Use retry.
- Environment: Windows x64, `x86_64-pc-windows-msvc`, Cargo offline, Node 24.19.0, Python 3.12.14 / PyInstaller 6.22.3. Fixed WebView2 Runtime `153.0.4234.48`, EdgeDriver `153.0.4234.46`, and tauri-driver `2.0.6` were available.
- Implementation: no Windows production-code changes. Source/validation revision is `7ed02b5e94e17171e26ae75000b555932e166216`; this documentation closeout is committed separately.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Affected Windows Rust modules | PASS | `cargo test -p xarchive-download -p xarchive-storage -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --quiet` using an isolated target | 171 tests passed across Desktop (106), Download/Transfer Driver (29), and Storage (36) | Current-source real account/download GUI checks remain open |
| Native Host Windows tests and release build | PASS | `cargo test -p xarchive-native-host --target x86_64-pc-windows-msvc --offline --quiet`; release build in isolated target | 8 tests passed; release executable built | Live HKCU registration and restart reconnect remain queued |
| Sidecar discovery tests | PASS | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests\test_discovery.py -p no:cacheprovider --basetemp validation-artifacts\windows-batch-revalidation-7ed02b5\pytest-temp` | 8 passed. Initial invocation only lacked the basetemp parent; after creating it, the same test passed. | Real account discovery remains queued |
| Desktop targeted UI wiring tests | PASS | `node --test desktop/test/ui-wiring.test.mjs` | 20 passed | Full Desktop Node suite did not terminate after about two minutes and was interrupted; record as `BLOCKED`, not a passing suite |
| Extension tests | PASS | `npm test --workspace extension` | 21 passed | Real Edge behavior remains queued |
| Vite frontend build | PASS | `npm run check --workspace desktop -- --outDir ../validation-artifacts/windows-batch-revalidation-7ed02b5/desktop-dist` | 52 modules built; existing Tauri dynamic/static import warning | No claim of native GUI readiness from this build |
| Current-source frozen Sidecar | PASS | PyInstaller build in isolated dist/work dirs; `--help`, v2 `hello`/`ready`, unknown-field and shutdown probes | `account_discovery` capability present; invalid field returned `INVALID_COMMAND`; shutdown exit 0. Probe SHA-256: `d993cb9bce18760429a3093ff85754b9ce2273a20157f7e4fda428fcb4258bc0` | Real extraction and aria2 transfer not established |
| Isolated Tauri release build | PASS | Windows x64 Tauri release build with validation config and isolated Cargo target | `validation-artifacts/windows-batch-revalidation-7ed02b5/target/x86_64-pc-windows-msvc/release/xarchive-desktop.exe` built (19,017,216 bytes) | GUI launch remains unverified for this revision |
| Current-source Full package | PASS (assembly/static contract only) | Assembled to `validation-artifacts/windows-batch-revalidation-7ed02b5/full-package` | App, worker, `python312.dll`, gallery-dl, Extension and Native Host present; package manifest and host allowed origin match Extension ID `iaajefkoanbkleojofoadeakelihbjne` | No browser registration, task, download, or GUI PASS inferred |
| Fixed-runtime dashboard WDIO | BLOCKED (automation/environment) | `npm run test:e2e:windows --workspace desktop`, with WebView2 `153.0.4234.48`, EdgeDriver `153.0.4234.46`, tauri-driver `2.0.6`, and the newly built Full package | Worker failed before application launch: Node `uv_os_get_passwd returned ENOMEM`; spec retry failed identically. This is not an app assertion or product failure. | Retry when Node/WDIO worker environment is healthy; preserve pinned runtime/driver pairing |
| Computer Use native GUI | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | Bounded native-app inventory and launch attempt | No native apps available; launch API unavailable. No Edge or XArchive window was controlled in this batch. | Keep `MANUAL-WIN-BATCH-REVAL-01`–`06` open |
| Full Desktop Node suite | BLOCKED (runner did not terminate) | `npm test --workspace desktop`; interrupted after about two minutes without a completion summary | Targeted 20/20 UI wiring tests passed separately; full-suite result incomplete | Re-run the workspace suite in a healthy runner; do not infer PASS |
| Real account discovery/download, package migration, reconnect, filesystem fault, signing/release gate | NOT RUN / BLOCKED | Not attempted or unavailable in this batch | Requires controlled browser/account, successful real download, manual GUI/registry access, filesystem fault fixtures, or signing/release target | Continue from `windows-queue.md`; prior manual FAIL remains bound to package/source `dac0a153`, not upgraded to current-source result |

#### Current triage and ownership

- Prior user-observed manual results remain evidence for Full package `full-package-r1` at `dac0a153`: task visible only after Dashboard refresh then failed; manually starting aria2 did not produce a download; Extension did not reconnect after Desktop restart; discovery remained at zero and still showed `RUNNING` for about 30 seconds after pause before user cancellation. Do not project these results onto the new source revision without retest.
- Current-source account task visibility, candidate progression, pause/cancel GUI state, aria2 transfer, v5 database copy migration, and Extension reconnect are `NOT RUN` or `BLOCKED` in this batch. Automated module tests and package assembly do not resolve those outcomes.
- `CROSS_PLATFORM_CHANGE_REQUIRED`: none newly established against `7ed02b5`; prior shared changes require manual revalidation. `CROSS_PLATFORM_REVIEW_REQUIRED`: none. Windows reconnect remains Windows-owned diagnosis.
- Next owner: Windows Platform Owner, to complete the open manual queue when native GUI access and controlled inputs are available.

### 2026-09-24 targeted WDIO and aria2 revalidation

- Owner: Windows Platform Owner
- Branch/source input: `feature/u7-desktop-production-integration`, source `7ed02b5e94e17171e26ae75000b555932e166216`; Windows WDIO/test changes were made after this input revision. Validation ran on the same current working tree as those changes.
- Implementation/validation revision: `e36705d` (`fix(windows): pass pinned runtime to WDIO service`).
- Scope: resolve the fixed-runtime Full-package dashboard startup failure, execute the complete Desktop Node suite, and validate the local aria2 runtime/transfer path without using account credentials or external services.
- Environment: Windows x64, Node `24.19.0`; fixed WebView2 Runtime `153.0.4234.48`; EdgeDriver `153.0.4234.46`; tauri-driver `2.0.6`; local aria2 `1.37.0` at `aria2/1.37.0/aria2-1.37.0-win-64bit-build1/aria2c.exe`.
- Windows-owned implementation: `desktop/wdio.conf.mjs` now forwards the fixed WebView2 folder to the Tauri service `env` option and honors an explicit `TAURI_DRIVER_PATH`; `desktop/test/ui-wiring.test.mjs` asserts both settings. This makes the configured driver/runtime pair reproducible and prevents the service from silently using another driver/runtime source.

| Item | Status | Command/steps | Evidence | Limits / follow-up |
|---|---|---|---|---|
| Full-package dashboard startup | PASS | `npm run test:e2e:windows --workspace desktop` with `WDIO_APP_BINARY` set to the assembled Full package; pinned WebView2, EdgeDriver, and tauri-driver paths; downloads disabled | Tauri smoke opened `http://tauri.localhost/`; actual session reported `webview2 153.0.4234.48`; 3/3 tests passed. Evidence under `validation-artifacts/windows-batch-revalidation-7ed02b5/wdio-config-fix` | This covers startup/readiness/dashboard shell, not Extension, real account submission, or download behavior. |
| Desktop full Node suite | PASS | `npm test --workspace desktop` in normal Windows process environment | 93 passed, 0 failed, 0 skipped. An earlier restricted execution had `uv_os_get_passwd returned ENOMEM`; the normal Windows run completed, confirming that was an execution-environment restriction. | WDIO GUI suite is separately reported above. |
| Windows-target aria2 transfer crate | PASS | `cargo test -p xarchive-download --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-batch-revalidation-7ed02b5/target-aria2 --quiet` | 22 unit tests and 7 integration tests passed. Cargo printed a non-fatal incremental-cache access warning after tests; command exit was 0. | Tests use the crate's fake backend; live binary check is recorded separately. |
| Local aria2 binary/RPC/range download | PASS | Isolated `aria2c.exe` 1.37.0 with loopback-only JSON-RPC and HTTP Range fixture; `getVersion`, `addUri`, `pause`, `unpause`, `tellStatus`, shutdown; Chinese/space output filename | 2 MiB completed; output SHA-256 matched fixture `4695B93998D2BE3CAE3354AD1D00A054ABC3DE0241A6FA2E632F7824108F45A2`; aria2 exited 0. Evidence under `validation-artifacts/windows-batch-revalidation-7ed02b5/aria2-runtime-20260924-r2/summary.json` | Does not prove Desktop's managed supervisor, account extraction, URL refresh, staging/commit, or app archive state machine. The source-tree `aria2` directory is not the Full package's default `sidecar/aria2` path; select/save its executable in Settings or package it at the expected relative location. A manually launched aria2 process does not substitute for the app's per-job loopback RPC child. |
| Desktop-managed real archive transfer | NOT RUN | No account/Extension submission or controlled extracted media URL was available for this run | No user account, external service, or media URL was used | Keep `MANUAL-WIN-BATCH-REVAL-04` open for a controlled end-to-end package run. |

The earlier fixed-runtime attempt that remained on `data:,` was caused by WDIO configuration not explicitly forwarding the runtime folder and driver path; it is superseded for dashboard startup by the passing rerun above. The earlier `uv_os_get_passwd` failure was a restricted-runner issue, not an app assertion. Other manual account-batch failures and Extension reconnect results remain bound to `full-package-r1` at `dac0a153` until retested.

### 2026-09-24 user-reported Extension and account-batch revalidation

- Owner: Windows Platform Owner (manual observations supplied by the user).
- Date: 2026-09-24.
- Artifact provenance: the user described the tested build as the current Extension/account-batch build, but did not include its exact source revision or package SHA-256. Treat the results below as user-reported observations; bind the artifact identity before using them as release acceptance evidence.
- Privacy: account names, Tweet IDs, batch IDs, and account URLs are omitted. Do not recover identifiers from screenshots into this history.

| Item | Result | Evidence / limit |
|---|---|---|
| Extension load and page action | PASS (user report) | Extension loaded; page action appeared and was clickable. |
| Extension-to-Dashboard submission | PASS for prompt visibility and idempotency; FAIL for execution | A task appeared in about 0.5 seconds without Dashboard refresh; repeating the same Tweet did not create a duplicate, and a different Tweet created another task. The tasks failed with `EXECUTOR_UNAVAILABLE: job executor is closed`. This is not a successful archive or download. |
| Account discovery | FAIL | Two tested accounts produced no candidates/tasks. The UI showed an active batch but zero candidates, submitted, completed, and failed entries. Pause and cancel controls worked in the observed UI. No SQLite timing or worker/Sidecar diagnostics were supplied. |
| v5-to-v6 package migration | PASS (user report) | User reports the migration test completed normally. No database hashes, row-count comparison, or foreign-key output were supplied. |
| Edge Extension refresh/reconnect | FAIL (user report) | Refresh in Settings did not reconnect Extension; UI showed Extension disconnected while Sidecar was connected. Further test is deferred at the user's request until Extension refactor. |
| Staging and Windows filesystem/recovery | NOT RUN | Long path, cross-volume, lock, abnormal-exit, recovery, and staging cleanup were skipped. User's “expected PASS” is recorded as an expectation only, not a result. |
| Application-managed aria2/extraction | BLOCKED / NOT RUN | The executor error prevented successful task execution. No successful app-managed aria2 transfer or file-integrity evidence was provided. Earlier local aria2 fixture PASS remains component-only. |
| Named Pipe and registry details | NOT RUN | No pipe endpoint/ACL, HKCU values, manifest path/origin, or unregister cleanup evidence was supplied. |

#### Routing

- `CROSS_PLATFORM_CHANGE_REQUIRED`: route job executor closure and non-producing account discovery to the Linux/Cross-platform Owner for diagnosis. This is an ownership classification for shared behavior; the report does not establish root cause or a required contract change.
- User recommends temporarily disabling account discovery pending further implementation. This is recorded as a recommendation only; no code or feature gate was changed in this validation entry.
- Windows Extension reconnect remains an observed Windows integration failure; resume that queue after Extension refactor.

### 2026-09-24 authenticated WebSocket / Extension UI Windows batch

- Owner: Windows Platform Owner.
- Branch: `feature/u7-desktop-production-integration`.
- Source and validation input revision: `084354a5ca433b52372aca4bc70ac5fc544104fc`.
- Windows implementation: none; no Windows production-source change was required.
- Working tree: fast-forwarded from `553e3788467041ef43ac5657c969064ce45d016c` to the fetched handoff. Tracked source was clean before this record; local dependencies, logs, manual-validation files, and existing validation artifacts were preserved. New output is under `validation-artifacts/windows-ws-084354a/`.
- Environment: Windows 11 x64; Node `24.19.0`; Python `3.12.14` / PyInstaller `6.22.3`; WebView2 Fixed Runtime `153.0.4234.48`; EdgeDriver `153.0.4234.46`; tauri-driver `2.0.6`; extension identity derived from the committed manifest key.

| Item | Status | Command / evidence | Limits |
|---|---|---|---|
| Affected Windows Rust targets | `PASS` | `cargo test -p xarchive-core -p xarchive-protocol -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-ws-084354a/target --quiet`; 146 passed (18 core, 19 protocol, 109 desktop) | Non-fatal linker/incremental-cache access warnings; exit code 0. |
| Desktop Node suite | `PASS` | `npm test --workspace desktop` with normal process-control permission; 93 passed, 0 failed, 0 skipped | The first restricted attempt hit the `killTree` child-process assertion and hung because process control was denied. The isolated teardown test then passed 8/8 and the full suite passed 93/93 when rerun with normal process control. |
| Extension tests | `PASS` | `npm test --workspace extension`; 25 passed | No real MV3 browser session. |
| Sidecar discovery/extraction affected tests | `PASS` | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests\test_discovery.py sidecar\tests\test_extraction_only.py -p no:cacheprovider --basetemp validation-artifacts\windows-ws-084354a\pytest-temp`; 15 passed | No real account or external media source. |
| Desktop production Vite build | `PASS` | `npm run check --workspace desktop`; 52 modules transformed | Existing Tauri API dynamic/static import advisory remains. |
| Extension ZIP package | `PASS` (inventory/extraction) | `validation-artifacts/windows-ws-084354a/XArchive-v0.0.0-pre.1-extension.zip`; 16,642 bytes; 12-file plan verified against extracted ZIP; SHA-256 `779F54F69528EFEE34EDD6F7FF8D6D412FEC7E5FEC9EC4BB008E587049944820` | Local synthetic pre-release tag only; no release publication. Loading the ZIP in Edge/Chrome remains `NOT RUN`. |
| Full package assembly/inventory | `PASS` (static contract) | Isolated current-source package at `validation-artifacts/windows-ws-084354a/full-package`; package type `full`, 11 required paths present, Extension directory 15 files. App SHA-256 `6B5284DBDD1BB843BCF49DEC296024F8391134B261D65D3CB2FE5C64303133EF`; worker SHA-256 `8F4FBBDB5AC084FCD73B5DD899695F67AEFC8540695FD8D87A0515C121EAEAD4`. | Fresh Tauri CLI release build, Native Host, and PyInstaller worker were isolated under this artifact root. aria2 is optional and was not included. No real task or transfer implied. |
| Full-package dashboard startup/readiness | `PASS` | `npm run test:e2e:windows --workspace desktop`, `WDIO_APP_BINARY` set to the isolated Full package; PATH prefixed with pinned EdgeDriver directory; pinned WebView2 and tauri-driver paths. Session URL `http://tauri.localhost/`; 3/3 passed. | A first attempt set `EDGEDRIVER_PATH` alone, but the service discovers EdgeDriver from PATH; after correcting PATH, it started. A direct Cargo release build without the Tauri CLI frontend step loaded the unused dev URL `localhost:1420` and failed; rebuilt via `tauri build --no-bundle --ci` with isolated `CARGO_TARGET_DIR`, after which the smoke passed. These setup failures are superseded, not source regressions. |
| WQ-WS-01 Edge/Chrome load and permissions | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | Two bounded inventory observations returned `apps: []`; no supported native-app launch target or isolated browser profile was available. Existing Edge profile contains user tabs and was not manipulated. | Re-run with a disposable Edge/Chrome profile; confirm MV3 worker, popup/options and permissions. |
| WQ-WS-02 live token pairing/auth | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | No controlled Desktop/browser pairing session or synthetic local request fixture was available. | Test wrong token rejection before valid token, authenticated status/archive requests, and token redaction. |
| WQ-WS-03 Desktop/Service Worker lifecycle | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | No controlled native GUI + browser profile to interrupt and restore a pending request. | Exercise Desktop/worker restart, pending cleanup, bounded reconnect, executor replacement, and Native fallback without replay. |
| WQ-WS-04 popup/options scaling and keyboard | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | Popup/options were not opened in a controlled browser profile. | Test 100/125/150% scaling, keyboard/focus, long status text, overflow, and settings save failure. |
| WQ-WS-05 packaging | `PASS` for ZIP/full assembly and static inventory; ZIP browser load `NOT RUN` | ZIP inventory/extraction passed; current Full package app/worker/Extension/Native Host and manifest inventory passed; dashboard smoke passed. | Package/static PASS is not Edge/Chrome Extension-load or live pairing acceptance. |
| Signing/release gate | `NOT RUN` | No signing credentials or release workflow invocation. | Requires release environment and target. |

#### Classification and next owner

- At the time of the automated validation record, `CROSS_PLATFORM_CHANGE_REQUIRED` had not been identified from automation/package evidence; no shared implementation was changed.
- The later manual follow-up below reports a live connection failure and establishes a cross-platform diagnosis follow-up; it does not isolate the failure to a code defect or a specific component.
- The user's previous account-discovery/executor-closure/reconnect results remain tied to their separately recorded artifact/revision; this dashboard smoke does not upgrade or clear them.

### 2026-09-24 Full package manual WebSocket follow-up

- Artifact: `validation-artifacts/windows-ws-084354a/full-package`; Extension directory: `validation-artifacts/windows-ws-084354a/full-package/extension`.
- Source/validation input: `084354a5ca433b52372aca4bc70ac5fc544104fc` on `feature/u7-desktop-production-integration`; this entry records user-supplied manual results against that previously assembled package. The report has not been independently reproduced in this turn.
- Privacy: account names, Tweet IDs, Extension identity, and token are omitted. The supplied options screenshot masks the token; no token value was recorded.

| Check | Result | Evidence and scope |
|---|---|---|
| Full package app startup, close, and content display | `PASS` (user-reported) | User confirmed normal startup/exit and normal main content display. |
| Extension load and basic settings/popup display | `PASS` (user-reported) | Extension from the Full package's `extension` directory loaded; settings/options and popup were displayed. No assertion is made for keyboard, scaling, or ZIP loading. |
| Extension/Desktop WebSocket integration | `FAIL` (user-reported) | Options displayed port `17321` and a masked pairing token while attempting to connect; the status section showed WebSocket disconnected. Popup reported Desktop disconnected and WebSocket connection closed. Desktop's Extension view reported no Native Host request and no active request. This demonstrates failed user-visible connection, not the underlying cause. |
| Reconnect/pending-request lifecycle | `NOT RUN` | No controlled pending request, Service Worker restart, network interruption, or reconnection sequence was reported. |
| ZIP installation, scale/keyboard accessibility | `NOT RUN` | The evidence covers the unpacked Full package Extension directory and basic rendering only. |

- Classification: `CROSS_PLATFORM_CHANGE_REQUIRED` for diagnosis and correction of the shared Desktop/Extension WebSocket integration. The Windows evidence establishes the failure on this package but does not isolate the responsible component; no Windows-specific root cause is established.
- No application or Extension source was changed in this follow-up. This record is based on the user's 2026-09-24 report and screenshots; no token, account name, Tweet ID, or browser identity has been copied into the repository.

### 2026-09-24 Extension ZIP load and token pairing retry

- Source/validation input remains `084354a5ca433b52372aca4bc70ac5fc544104fc`; the user report concerns the Extension ZIP generated in that batch (`validation-artifacts/windows-ws-084354a/XArchive-v0.0.0-pre.1-extension.zip`).
- `PASS` (user-reported): ZIP loaded in the browser and the Extension UI appeared.
- `FAIL` (user-reported): the Desktop-generated pairing token did not connect when transferred by either the automatic copy action or manual entry. The Popup screenshot reports Desktop `未配置`, WebSocket `未配置 · 端口 17321`, WebSocket closed, and Native Messaging fallback.
- `NOT RUN`: authenticated/unauthenticated response behavior, verification that the token persisted in Extension storage, Desktop receipt of a WebSocket handshake, and archive request delivery. The visible disconnected/unconfigured state does not establish token rejection or identify the faulty component.
- Privacy: no token, account name, Tweet ID, or Extension identity was copied from the screenshot. No independent GUI reproduction was performed.
- Classification remains `CROSS_PLATFORM_CHANGE_REQUIRED` for shared-path diagnosis. First isolate configuration persistence, browser WebSocket connection/handshake, listener reachability, and token authentication before selecting a code owner or claiming an auth defect.

### 2026-09-24 Full package WebSocket listener and authentication-frame follow-up

- Artifact/source: same `validation-artifacts/windows-ws-084354a/full-package` run and `084354a5ca433b52372aca4bc70ac5fc544104fc` input. This records later, user-supplied evidence; it does not erase the earlier `ERR_CONNECTION_REFUSED` snapshot or the ZIP run's UI state. Observations are point-in-time and were not independently reproduced.
- Record scope: account names, Tweet IDs, pairing-token values, and Extension identity are not reproduced in this historical note. The screenshot associated with this earlier observation masked the token. Per the user's clarification, token privacy/security review is skipped and is not classified as a finding.

| Check | Result | Evidence and limit |
|---|---|---|
| Desktop loopback listener reachability | `PASS` (user-reported, point-in-time) | PowerShell showed a listener on `127.0.0.1:17321` owned by the Full package `xarchive-desktop` process; `Test-NetConnection 127.0.0.1 -Port 17321` returned `True`. This does not establish WebSocket authentication or request handling. |
| WebSocket HTTP upgrade | `PASS` (user-supplied DevTools evidence, one selected request) | The selected `ws://127.0.0.1:17321/` request returned `101 Switching Protocols`. This proves that request reached a WebSocket endpoint and upgraded; it does not prove auth success or that every retry reached the same process. |
| Extension authentication and connection state | `FAIL` (user-reported) | Extension settings remained enabled on port `17321` with a nonempty masked token, while WebSocket status remained disconnected; Desktop showed Native Host requests arriving, but WebSocket unauthenticated and Desktop observation `not_loaded`. DevTools showed an outbound `authenticate` frame and no inbound server response in the supplied capture. The user reports that no requests received a response. |
| Browser-side connection errors | `FAIL` (user-supplied console evidence) | Captures include `WebSocket is closed before the connection is established`, `ERR_SOCKET_NOT_CONNECTED`, and earlier `ERR_CONNECTION_REFUSED`. Later listener/TCP and one `101` observation show that reachability differed across attempts or times; the earlier refusal alone does not describe the later state. |
| Native Messaging archive-button path | `PASS` (limited, user-reported) | Clicking the X-page archive button created a task and a Native Host request was visible. This only verifies task submission/Native Host receipt; task execution/download is not established by this observation. |
| Authentication response / request completion | `FAIL` (user-reported no response; protocol outcome `NOT RUN`) | No inbound `authentication_response` was visible in the supplied Messages capture and no request response was observed. Therefore the token is not proven rejected or accepted, and the failure point (client close, server receive/response, or other lifecycle race) remains unknown. |

- Diagnosis boundary: these observations make a permanently absent listener an insufficient explanation for the later capture. An outbound auth frame plus missing inbound response localizes the open question to after the browser sent the frame, but does not show whether Desktop received or processed it. Repeated reconnect/close behavior could be consistent with a client lifecycle race, but that remains a hypothesis, not a verified cause. Next diagnostics should correlate per-connection server accept/auth/close events with a client attempt, without recording the token, and distinguish client-initiated close from server failure to respond.
- Classification: `CROSS_PLATFORM_CHANGE_REQUIRED`; no component-level root cause or fix is established. Native Messaging and WebSocket evidence are separate; a working Native Host request does not imply WebSocket readiness.
- No code change or automated GUI reproduction was performed for this documentation update.

### 2026-09-25 Windows revalidation of authenticated WebSocket handoff

- Owner: Windows Platform Owner.
- Branch: `feature/u7-desktop-production-integration`.
- Handoff/source validation input: `98f16845b9e37f0dea419e7bffc890b1be3d2063` (includes shared implementation `de46a8ee7ac937f9ed64db5f16b9e8c4cb1c178d`).
- Windows implementation revision: none; the current diff was the shared auth-timeout/diagnostics change. This record is committed separately as Windows validation documentation.
- Working tree: fetched `origin` and fast-forwarded the clean tracked tree from `2d067cbf0412a8819fb52b14895f00e7c0b46bd2` to the handoff. Existing untracked dependencies, logs, local aria2/gallery-dl assets, manual-validation files, and artifacts were preserved. New build/test output is isolated under `validation-artifacts/windows-ws-de46a8e/`.
- Environment: Windows 10 Pro for Workstations; Node `24.19.0`, npm `11.17.0`, Rust/Cargo `1.98.0`, Python `3.12.14`, Windows target `x86_64-pc-windows-msvc`.

| Item | Status | Command / evidence | Limits |
|---|---|---|---|
| WebSocket-focused Windows Rust tests | `PASS` | `cargo test -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-ws-de46a8e/target websocket_transport::tests -- --nocapture`; 4 passed, 107 filtered. Covers defaults, wrong token, valid-token authentication response, and authenticated `query_status` route. | Automated Windows-target tests do not exercise a real browser or packaged runtime. Nonfatal linker/incremental-cache access warnings; exit code 0. |
| Extension Node suite | `PASS` | `npm test --workspace extension`; 26/26 passed. | No real MV3 browser session. |
| Desktop Node suite | `PASS` | `npm test --workspace desktop`; 93/93 passed when rerun with normal process-control permissions. | First restricted attempt failed/hung at child-process teardown because `killTree` was denied; successful rerun supersedes the sandbox artifact. |
| Windows Desktop crate suite | `PASS` | `cargo test -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-ws-de46a8e/target --quiet`; 111 passed, 0 failed with `PYTHON` set to `.venv-windows-validation\Scripts\python.exe`. | An initial run without `PYTHON` had 3 batch tests fail because the Sidecar did not start; the project Python prerequisite was then set and all 111 tests passed. This was setup, not a reproduced product failure. |
| Formatting and frontend checks | `PASS` | `cargo fmt --all -- --check`; `npm run check`; Desktop Vite transformed 52 modules and Extension syntax checks passed. | Existing nonfatal Tauri API dynamic/static import advisory remains. |
| Windows Tauri release executable | `PASS` | `npm run build:tauri --workspace desktop -- --no-bundle --ci` with isolated `CARGO_TARGET_DIR`; produced `validation-artifacts/windows-ws-de46a8e/tauri-target/release/xarchive-desktop.exe`. | Executable build only; no Full package assembly, signing, GUI launch, or live pairing implied. |
| Full package / Extension ZIP exact-revision assembly | `NOT RUN` | No packaging command was required by the narrow auth-timeout/diagnostics diff; package plan/static checks remain covered by existing valid evidence. | No current-revision ZIP/Full package artifact was built or browser-loaded. |
| Live browser GUI and authenticated pairing (WQ-WS-01/02/04) | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | Limited Computer Use attempts returned no native app targets (`apps: []`); the available Edge surface exposed an existing user profile, which was left untouched. `listWindows()` was unavailable in the tool surface. | Need a controlled/disposable Edge profile and current-revision Full package. Prior user-reported manual failure remains historical and is not upgraded by automated PASS. |
| Desktop/Service Worker lifecycle (WQ-WS-03) | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | No controlled GUI/browser target for pending request, restart, reconnection, or cleanup sequence. | Continue in a disposable profile with a synthetic/test-owned request. |
| Full workspace regression | `NOT RUN` | Current source diff is limited to Desktop WebSocket auth/diagnostics, Extension bridge/UI status, and focused tests; affected Desktop/Extension suites, Windows Rust target tests, frontend checks, formatter, and Windows executable build passed. | No unrelated Sidecar, aria2, account-discovery, filesystem, or complete workspace suite was run. |
| Signing/release gate | `NOT RUN` | No signing credentials or release workflow invoked. | Not required to validate this source handoff. |

#### Manual Windows Validation Queue

1. Build or obtain a Full package and Extension ZIP from source revision `98f16845b9e37f0dea419e7bffc890b1be3d2063`; verify the exact source revision before launch. Use a disposable Edge profile rather than the user's signed-in profile.
2. Launch the Full package. Open Extension settings and popup. Confirm the new bounded `auth_timeout` state is visible after a silent/nonresponsive peer and Desktop's auth/connection diagnostic counters update. Never copy the pairing token into screenshots, logs, or documentation.
3. With a local test token, attempt a wrong token and confirm explicit auth failure with no business request routed. Then pair using the correct token and confirm authenticated status.
4. Send a controlled `query_status` request and a synthetic/test-owned archive request. Confirm matching `request_id` responses and that no token appears in Browser payloads, Job specs, or logs.
5. With one request pending, stop/restart Desktop and restart/reload the Extension worker; verify pending cleanup, bounded reconnect, fresh authentication, no duplicate/replayed request, and accurate Native Messaging fallback state.
6. Separately check popup/options at 100%, 125%, and 150% zoom; Tab/Enter/Space/Escape behavior; long status text; and save-error state. Record each result against the exact source/artifact revision.

No current-revision GUI result is inferred from the earlier user-supplied Full package observations. No new Windows-specific or shared-contract root cause was established by this batch; the Linux-owned shared fix is Windows-automated-test verified, while live integration remains blocked.

### 2026-09-25 current-revision Full package and Extension ZIP preparation

- Package source revision: `fcde5943af2f6ad15c833fa5ab88d6b1758345c6`. It adds validation documentation only after application-source validation input `98f16845b9e37f0dea419e7bffc890b1be3d2063`; production source is unchanged. The Tauri executable was built at the prior source input and reused because the intervening commit changed documentation only.
- Full package directory: `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\full-package`.
- Extension ZIP: `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\XArchive-v0.0.0-pre.1-extension.zip`.
- Package metadata tag `v0.0.0-pre.1` is a local validation label, not a release tag. Bandizip CLI 8.0 (Beta, x64) generated `XArchive-v0.0.0-pre.1-windows-x64-full.7z` from the portable Full directory at compression level 9. Signing/publishing was not attempted.
- Full package `PASS` (assembly and static inventory): executable, `package-manifest.json`, Extension directory, Native Host executable and manifest, worker, and Full gallery-dl payload exist. `package_type=full`, platform `windows-x64`; package Extension identity and Native Host allowed origin match `iaajefkoanbkleojofoadeakelihbjne` from the manifest. This is not browser/GUI acceptance.
- Extension ZIP `PASS` (plan/build/extract/verify): generated a release-plan inventory, staged its 12 declared files, compressed using Windows `Compress-Archive`, expanded to an isolated directory, and passed `extension-package.mjs verify` (12/12). ZIP size 16,796 bytes; SHA-256 `32FDCB29AA847CFF4DDE48A1C0BE4FB10F44C92549782A1B87436BADD30A4FAB`.
- SHA-256: Desktop executable `AF07ED237538125E0ECA1689AE2D6295479CD50334318F76F8D421E4F888F074`; Native Host `76DC613744D83B65A3478FB6D72B534E7F72CDF983D4D83E5BE78C3279BCE153`; gallery-dl `0B36AE6734ED41E12BE6BE1B33D3165A450B3E0A811FC1B8C664C032F7F13B2C`.
- User provided aria2 directory `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2`; source executable `1.37.0\aria2-1.37.0-win-64bit-build1\aria2c.exe`, aria2 1.37.0, SHA-256 `BE2099C214F63A3CB4954B09A0BECD6E2E34660B886D4C898D260FEBFE9D70C2`. Copied into this validation package at `full-package\sidecar\aria2\aria2c.exe` before archive creation. This is local artifact staging; the package builder and release workflow were not changed. Do not infer app-managed task/download success from binary presence or independent aria2 tests.
- Full `.7z` archive: `validation-artifacts/windows-ws-fcde594/XArchive-v0.0.0-pre.1-windows-x64-full.7z`, size 34,865,707 bytes, SHA-256 `35604266D32526632DF02145D2126310DDA7262C848189979ECF8F8E33C4F077`. Bandizip `t` result `All OK`; extracted with `bz x -target:name -o:<isolated-directory> -y <archive>`. Source/extracted inventory comparison: 77/77 files, missing 0, extra 0, SHA-256 differences 0. Archive creation/integrity/extract comparison `PASS`.
- Bandizip generation command used from the package directory: `bz c -fmt:7z -l:9 -r <absolute-archive-path> '*'`; Bandizip installed at `C:\Program Files\Bandizip\bz.exe` (8.0 Beta, x64). Archive input package directory is `validation-artifacts/windows-ws-fcde594/full-package`.
- Browser Extension load, Desktop GUI launch, WebSocket authentication, request/response, app-managed download, and lifecycle manual checks remain `NOT RUN` or `BLOCKED` according to the queue. Package assembly, aria2 presence, and archive integrity do not resolve the prior user-reported no-response failure or establish download success.
- Detailed numbered steps are in the “2026-09-25 package preparation for manual WebSocket verification” section of `windows-queue.md`.

### 2026-09-25 Full package Sidecar startup investigation

- User supplied a Full package Dashboard screenshot showing the first-run download-folder prompt, `Sidecar 未启动`, and `Sidecar 操作失败：sidecar is not running`.
- The original package at `validation-artifacts/windows-ws-fcde594/full-package` contained a stale frozen worker. Desktop's `start_sidecar` passes `--timeout-seconds 300 --discovery-timeout-seconds 600` from its Windows configuration. The old worker's `--help` omitted both arguments. Replaying this exact argument set against the original packaged worker reproduced exit code 2 with `error: unrecognized arguments: --timeout-seconds 300 --discovery-timeout-seconds 600`. Thus the worker quits before the protocol handshake and Desktop reports Sidecar not running. The prior isolated hello test omitted Desktop's actual arguments and was insufficient to validate the launch path.
- Rebuilt the worker from the current Sidecar source using Python 3.12.14 and PyInstaller 6.22.3. Fresh `--help` exposes both timeout arguments. A direct process-level probe with Desktop's exact arguments, packaged gallery-dl, and v2 JSONL `hello` returned `ready` plus the expected capabilities; graceful `shutdown` exited 0. Worker SHA-256: `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`.
- Reassembled a clean corrected package at `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix`; included aria2 from `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2\1.37.0\aria2-1.37.0-win-64bit-build1\aria2c.exe` at `sidecar/aria2/aria2c.exe`. Bandizip 8.0 Beta x64 created `XArchive-v0.0.0-pre.1-windows-x64-full-sidecar-fix.7z` (34,877,156 bytes; SHA-256 `1DB74B9DEBF5CB6E72A098D871EE20C70582D28E8E4B495E0B1AEF16BC9552C5`). Bandizip archive test passed. Extracted inventory/hash comparison: 77 package files, 77 extracted files, 0 missing, 0 extra, 0 differing.
- Validation classification at the initial automation attempt: old Full package exact Desktop worker invocation `FAIL` (confirmed stale worker CLI); fresh worker exact-argument protocol handshake `PASS`; corrected archive integrity and extraction comparison `PASS`; corrected package GUI start was initially `BLOCKED` when the user stopped Computer Use with physical Escape. A later user screenshot on the same date supersedes that GUI block for the visible state: corrected-package Dashboard shows Sidecar connected. No archive task/download success is claimed.
- Remaining manual step: submit one controlled task from the corrected Full package while Sidecar reports `ready`; record task execution/download separately. The Extension pairing failure remains separate and is tracked in the subsequent dated entries.

### 2026-09-25 corrected Full package Extension WebSocket follow-up

- User-provided screenshots are from the Extension directory under `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix`. Options rendered, WebSocket was enabled on port `17321`, and its UI still showed disconnected. Desktop showed Sidecar connected, Extension files ready, Native Host registered, browser observation `not_loaded`, and all WebSocket counters at zero.
- DevTools Network showed six `127.0.0.1` WebSocket requests, all status `101`, displayed resource size `0 B`, durations 2–52 ms. These entries establish repeated HTTP Upgrade success only. The screenshots do not include WebSocket Messages, so they do not prove an `authenticate` frame or any server response.
- Read-only Windows process inspection during this investigation found the running `xarchive-desktop.exe` at the corrected package path and a listener on `127.0.0.1:17321`; local TCP `TIME_WAIT` entries were also present. This is a separate point-in-time observation, not per-request correlation with the six DevTools entries.
- Classification: Extension Options page load/display `PASS` (user screenshot); WebSocket Upgrade attempts `PASS` (user screenshot); authenticated session and request/response `FAIL` (Extension remained disconnected/no response); authentication outcome and component root cause `NOT RUN` / unresolved. Desktop diagnostics may be stale until refresh. If refreshed counters still show `accepted=0` after a newly generated 101, that conflicts with the inspected listener's accept path and should be investigated as a stale/different runtime or diagnostics defect before concluding the token is wrong.
- User clarification (2026-09-25): the token is for local validation and resets on Desktop restart. Per the user's request, token privacy/security review is `NOT APPLICABLE`; no compromise finding or rotation action is tracked. The literal token is omitted from committed records.
- Next evidence needed: refresh Desktop Extension diagnostics, create one reconnect, select that same newest DevTools request and inspect Messages for outbound `authenticate` plus inbound `authentication_response`; provide only frame type/boolean outcome and before/after counters. Do not include the token or full frame payload.

### 2026-09-25 WebSocket accepted but closed before authentication

- New user screenshots show refreshed Desktop counters: `accepted=100`, `auth_received=0`, `auth_succeeded=0`, `auth_failed=0`, `auth_response_failed=0`, `close_before_auth=99`, `close_after_auth=0`. The Extension popup remains disconnected and uses Native Messaging fallback. Edge's Extension error page reports `net::ERR_SOCKET_NOT_CONNECTED` at the WebSocket constructor call (`websocket-bridge.js:55`).
- For this aggregate snapshot only, WebSocket connections reached Desktop's accept loop, but no authentication envelope was counted. 99 connections closed before auth; the 100th was not yet counted closed in the screenshot. A wrong token would be evaluated only after receiving/parsing the auth envelope and incrementing `auth_failed`. HTTP 101 remains handshake-only evidence.
- The earlier six DevTools rows had 2–52 ms durations, suggesting early teardown rather than the listener's 5-second auth read timeout, but they were not correlated to the refreshed cumulative counters. At the time of this snapshot the receive/send/close stage was unresolved. No response-path or token defect was established.
- Result: live authenticated pairing `FAIL`; listener accepts WebSocket upgrades `PASS`; auth receipt `NOT RUN` (zero observed); auth outcome `NOT RUN`; exact client/server teardown cause unresolved. The previous screenshot's all-zero counters are superseded by this refreshed snapshot.
- This counter snapshot is point-in-time evidence and is not correlated with any later selected DevTools request. Do not treat it as proof that a later outbound auth frame was not received.
- Next observation: initiate one reconnect and correlate before/after Desktop counters with that exact request's Messages view. Report only whether `authenticate` and `authentication_response` appeared, the boolean auth outcome/error code, and relevant counter deltas.

### 2026-09-25 WebSocket authentication frame observed

- User supplied a new Edge DevTools screenshot for `ws://127.0.0.1:17321/`. The selected request's Messages pane visibly contains one outbound `authenticate` frame. Its payload includes the pairing token; the value is deliberately not copied here. No inbound `authentication_response` is visible in the capture.
- In the same evidence set, Extension Options remains `已断开`, the browser console reports `ERR_SOCKET_NOT_CONNECTED` at `websocket-bridge.js:55`, and repeated requests continue. This establishes that the Extension emitted an auth frame for the selected request, but it does not establish Desktop receipt, authentication outcome, or response delivery.
- The earlier Desktop aggregate snapshot (`accepted=100`, `auth_received=0`, `close_before_auth=99`) was not taken in a per-request-correlated manner. It cannot be applied to this selected DevTools connection; server receipt for this connection remains unknown.
- User instruction: the token is local-validation-only and resets on each Desktop restart; skip privacy/security review of this token. Classification: token privacy/security check `NOT APPLICABLE`; no compromise finding or rotation action. The literal value is omitted from committed records.
- Result: outbound authentication frame `PASS` (selected request, user screenshot); inbound authentication response `NOT OBSERVED`; live authenticated pairing remains `FAIL` (UI disconnected); exact transport/server outcome `NOT RUN` / unresolved. The next useful observation is one correlated reconnect with Desktop counter deltas and the same request's Messages pane.
