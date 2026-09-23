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
