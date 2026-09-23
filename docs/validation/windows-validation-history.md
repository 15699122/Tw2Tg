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
