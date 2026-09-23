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

### 2026-09-23 WebView2 readiness recovery (Windows)

- Owner: Windows Platform Owner
- Source branch: `windows/webview2-readiness-gate`
- Base revision: `35748f41fe3c01954cadb150c7aa33edc3aa4152`
- Working tree included: Windows changes in `desktop/wdio.conf.mjs` and
  `desktop/scripts/patch-wdio-tauri-service.mjs`; validation documents updated
  after the user's run. User-local dependencies, logs, and validation artifacts
  remain untracked and are excluded from the commit.
- Scope: restore Windows WDIO native WebView2 app-document discovery and
  Dashboard readiness; no product business logic changed.
- Environment: Windows E: checkout; fixed WebView2 Runtime `153.0.4234.48`,
  EdgeDriver `153.0.4234.46`, explicit local tauri-driver; exact runtime and
  driver paths are retained under `validation-artifacts/` and not committed.
- Canonical integration: pending commit/push at time of recording.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Windows native-driver process launch | PASS | `npm run test:e2e:windows --workspace desktop` pretest patch; WDIO E2E | User run confirms service/native-core patches already applied; service starts without path splitting | `CROSS_PLATFORM_REVIEW_REQUIRED`: review installed `@wdio/native-core` patch and Linux behavior |
| WebView2 target discovery | PASS | Same E2E command | session-start URL `http://tauri.localhost/`; no longer `data:,` | Historical blank-target failures remain historical, superseded for this harness/config |
| Frontend startup contract | PASS | Same E2E command | Spec: “completes the frontend startup contract before rendering the dashboard” | None within current scope |
| Native Dashboard shell and regions | PASS | Same E2E command | 3 passing, 0 failing; user confirms XArchive window appeared | Native console diagnostics visible in screenshot are not classified as failure; app dashboard E2E passed |
| Standalone GUI close and native logs | NOT_RUN | Not separately measured/collected in this run | User confirms the window appeared; no separate timed close/log bundle was supplied for this run | Do not infer 30-second wait or log completeness |
| Advanced native E2E | NOT_RUN | `WDIO_ADVANCED=1` not used | Not part of submitted run | Run only if broader batch scope requires it |
| Hosted/release gate | NOT_RUN | No hosted workflow run | Local E2E does not establish hosted behavior | Separate release validation remains |
| Full Sidecar/Extension integration | NOT_RUN | Not in WebView2 readiness command | No worker/Native Host/browser messaging evidence | Retain package integration follow-up |
| Syntax and whitespace checks | PASS | `node --check desktop/wdio.conf.mjs`; `node --check desktop/scripts/patch-wdio-tauri-service.mjs`; `git diff --check` | All commands exited successfully | Targeted checks only; no full regression |

Root cause was the Windows native driver launch path: `@wdio/native-core` used
`shell: true` while spawning an executable beneath a checkout path containing
spaces. `cmd.exe` split the executable path. The idempotent pretest patch now
sets `shell: false` for direct executable spawning. The WDIO config also accepts
explicit tauri-driver / EdgeDriver paths, adds the EdgeDriver directory to PATH,
passes the fixed runtime folder, and sets `webviewOptions: {}` on Windows.
The successful target discovery and 3/3 assertions validate this combined
configuration; they do not isolate each setting's individual causal effect.

Full regression was not run: the change is limited to WDIO Windows launch and
configuration. Sidecar/Extension package readiness and hosted release behavior
are separate validation scopes.

### 2026-09-23 Full portable package component validation (Windows)

- Owner: Windows Platform Owner
- Source branch/revision: `windows/webview2-readiness-gate` /
  `75d8c2ca1d59f14fcf83a9aef8e36c1990ee7144`
- Working tree included: source tree at the recorded HEAD; package outputs and
  toolchain artifacts are local and untracked under `validation-artifacts/`.
- Scope: build a current-source Windows x64 Full package with Sidecar worker,
  Extension and Native Host, then exercise independently testable package
  components. No source code was changed for this validation run.
- Environment: Windows E: checkout; fixed WebView2 Runtime `153.0.4234.48`,
  EdgeDriver `153.0.4234.46`, tauri-driver `2.0.6`; Python 3.12.14 and
  PyInstaller 6.22.3; package output
  `validation-artifacts/portable-full-75d8c2c-r3/`.
- Canonical integration status: component assembly and protocol probes pass;
  GUI package E2E and real Extension-to-Desktop messaging are not accepted.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Tauri Desktop release build | PASS | `npm run build:tauri --workspace desktop` | Current source produced `target/release/xarchive-desktop.exe` | None for package assembly |
| Native Host release build | PASS | `cargo build -p xarchive-native-host --release` | Current source produced `target/release/xarchive-native-host.exe` | Windows pipe endpoint is a separate missing feature |
| Sidecar worker build | PASS | Windows validation venv + `sidecar/pyinstaller/xarchive-downloader.spec` | PyInstaller output includes `xarchive-downloader.exe` and `_internal/python312.dll`; staged at builder input path | Build script staging/output path mismatch is locally worked around; no source modification |
| Full package assembly | PASS | `node desktop/scripts/build-portable-windows.mjs` with extension ID and `PORTABLE_APP_VERSION=v0.0.0-local` | Manifest says `full` / `windows-x64`; contains Desktop, worker, gallery-dl, Extension, Native Host exe and manifest | Local-only package retained; no aria2 executable was bundled |
| Packaged worker help and v2 handshake | PASS | `xarchive-downloader.exe --help`; `node package-protocol-probe.mjs` | Help exit 0; v2 `ready` + required capabilities; unknown field returns `INVALID_COMMAND`; shutdown exit 0 | Protocol smoke is not media extraction or network integration |
| Packaged Extension identity | PASS | `node desktop/scripts/extension-identity.mjs verify --manifest <package>/extension/manifest.json --expected-id iaajefkoanbkleojofoadeakelihbjne` | Output: `Extension identity OK: iaajefkoanbkleojofoadeakelihbjne` | Actual Edge load and message flow not run |
| Packaged Native Host framing | PASS | Length-prefixed JSON request to package Native Host | Host returns correlated response and exits 0 | Response is `NATIVE_PIPE_UNAVAILABLE`; Desktop endpoint absent |
| Windows Desktop Named Pipe forwarding | BLOCKED | Inspected `docs/protocols/overview.md`; standalone Host probe | Windows Desktop Named Pipe server is not implemented in current source; Host reports unavailable endpoint | Implement Windows endpoint/ACL and then test request/response path; Windows Platform Owner |
| Packaged Tauri + Sidecar GUI E2E | FAIL (automation session) | Custom package WDIO spec with fixed Runtime/EdgeDriver; repeated against source exe baseline | Both sessions disconnected from DevTools before spec/window handle; package-specific assertions did not execute | Keep as WDIO/browser automation failure; investigate session attach separately; do not mark package GUI PASS |
| Computer Use / browser Extension test | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | Limited `sky.list_apps()` retries | `Trusted RPC service is not configured: sky` | Manual Windows Validation Queue remains open |

The Full package directory is retained locally and is not committed. Artifact
SHA-256: Desktop `5523B03EFD5BAB171BA5BAD30F21F1AC955B1DDA408AE11627DB217A71C3AA9F`;
worker `96C19695AC46E30AA23AF184ED41D0C8339FE4C98A2D771CF0781906402A60C2`;
Native Host `76DC613744D83B65A3478FB6D72B534E7F72CDF983D4D83E5BE78C3279BCE153`.
Detailed queue and manual steps are in `docs/validation/windows-queue.md` under
“Full package build and integration follow-up”.
