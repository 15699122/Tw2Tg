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
