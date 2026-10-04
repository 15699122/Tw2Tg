# Windows Validation History

## 2026-10-04 — Telegram Windows local return f57c4b2

Owner Windows Platform Owner; priority P0 credentials / P1 remaining acceptance.
Method automated native/module/static plus Computer Use read-only smoke.
Fetched origin first, then exact target. Source `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff`;
handoff `f57c4b2090f02df3ba55c58a660e2eee143a9660` differs only in handoff/queue docs.
New branch `codex/windows-validation-f57c4b2`, implementation/build source
`b5dd8c95f2685e8d91adf2bb9c24f7d7caf50012`; return revision is this record's commit.
Initial tracked tree clean; existing untracked caches/tools/artifacts preserved.
No direct filesystem sync or production shared contract/schema changes.

Environment: native Windows x64, OS 10.0.29680.0, PowerShell, Rust/Cargo 1.98.0,
Node 24.19.0, npm 11.17.0, MSVC and WebView2 available through successful native
build/window. User's development display uses 200%; no new DPI matrix recorded.
Computer Use `@oai/sky` imported successfully and enumerated Edge and Unigram;
neither their versions nor real account availability were verified. User explicitly
confirmed bot/chat/api_id/api_hash not prepared and selected local-only checks.

Implementation: target-only pinned keyring 3.6.3, explicit Windows WinCredential,
sanitized errors, missing-entry semantics, no default/mock/plaintext fallback,
production RuntimeState provider seam. Existing shared Mutex serializes access.
Unique synthetic native test credential automatically cleaned after CRUD.

| Check / reproducible command | Result / evidence |
|---|---|
| `cargo test -p xarchive-desktop windows_credentials -- --include-ignored` | PASS 2 native/input/redaction tests; credentials.log |
| `cargo test -p xarchive-desktop telegram -- --nocapture` | PASS 48; desktop-telegram.log; explicitly set PYTHON to .venv-windows-validation/Scripts/python.exe |
| `cargo test -p xarchive-telegram -p xarchive-storage` | PASS 72 / 54; shared-modules.log; mock/module evidence only |
| `node --test desktop/test/telegram-state.test.mjs desktop/test/telegram-render.test.mjs` | PASS 6; stdout, exit 0 |
| `npm run check --workspace desktop` | PASS; ui-build.log |
| `node desktop/scripts/build-tauri.mjs --no-bundle` | PASS release-profile native build; native-build.log; development channel, not Full release package |
| `cargo fmt -p xarchive-desktop --check`; `node scripts/docs-audit.mjs`; `git diff --check` | PASS exit 0; docs-audit.log |
| Computer Use exact EXE launch → Settings | PASS scoped: dashboard rendered, credential service available, token not saved, sender stopped; tool accessibility/screenshot transcript |

Logs reside in `validation-artifacts/windows-tg-f57c4b2/` (local, untracked).
Artifact `target/release/xarchive-desktop.exe` SHA-256
`f5e59f8b4a47ed4ee0e5df22e30bf32f6739bcbd430b4171e3aa147ce2e4d826`.
Final native credential test EXE SHA-256
`1fffa8bb50375045896898a8378fe9c0c742518c612457e03420d43014e5f51f`.
The final input-neutral cfg binding adjustment was included in the committed build
and final two credential tests. Initial test binary was subsequently rebuilt.

Tool diagnostics, not product failures: sandbox Cargo fetch initially failed
Schannel SEC_E_NO_CREDENTIALS; elevated fetch passed. Incremental-cache finalization
reported Access denied but compilation/tests passed; OS CIM query denied, bounded
Environment.OSVersion alternative succeeded. Linker emitted informational library
creation warnings. No assertions/product behavior weakened to accommodate tools.

Acceptance limits: WQ-TG-001 full isolated-account GUI/restart/leakage acceptance
NOT_RUN; WQ-TG-002–009 and UNI-01–08 NOT_RUN except the above scoped subchecks.
WQ-TG-007 server deployment PLANNED because credentials/service target absent.
WQ-TG-008 full DPI/keyboard/narrow/cancel/retry/UNKNOWN matrix deliberately deferred.
Scheme B remainder NOT_RUN this round; historical pairing failures not closed.
Full regression, Full packaging and publication not run: this local native adapter
batch uses targeted Telegram/storage/UI checks and creates no release artifact.
No real Telegram client reception, bot verification, permanent credential rotation,
large file, crash recovery, proxy or diagnostics leakage PASS inferred from mocks.

Next Owner Windows Platform Owner; manual prerequisites/actions in §K1 of
windows-manual-steps.md. Cross-platform Owner reconciles commits/Windows-only
dependency through Git; no CROSS_PLATFORM_CHANGE_REQUIRED or shared fix requested.
This is WINDOWS_VERIFICATION_PENDING, not WINDOWS_PASS; no WINDOWS_BLOCKING declared.
Reuse only if native adapter, dependency, runtime setup and account capabilities
are unchanged; otherwise REVALIDATION_REQUIRED. Release/service acceptance remains
pending independently of these local PASS results.

## 2026-10-01 Automated release build — `v0.2.1-pre1` (tag target `3115c3b`)

- Owner: Cross-platform Owner invoked the release; the Windows job ran on a GitHub-hosted runner. This entry records a **machine-driven build/asset run**, not a Windows Platform Owner manual batch. Tag `v0.2.1-pre1` → `3115c3b50716be0155804ad4f94dd9d29e37d617`; `windows-release.yml` Run `36838400270` (2026-10-01T08:46:13Z → 09:10:23Z); dispatched by `pre-release.yml` Run `36837862702` (`success`).
- Build job: `success`. All seven assets uploaded to the Release: `…-windows-x64.exe` 19 812 864 B, `…-windows-x64.7z` 4 552 639 B, `…-repository-dependencies.7z` 6 079 660 B, `…-full.7z` 34 615 357 B, `…-extension.7z` 12 388 B, `…-release-manifest.json` 1 351 B, `SHA256SUMS-v0.2.1-pre1.txt` 538 B.
- G3 asset-integrity gate **PASS**: manifest `source_sha` = `3115c3b50716be0155804ad4f94dd9d29e37d617` = tag target, and the five `sha256` values in `SHA256SUMS-v0.2.1-pre1.txt` match the manifest entries (both files downloaded from the Release and compared). `catalog_version` is `unreleased`, matching the `v0.2.0` manifest.
- Independent WDIO job: **FAIL**, non-blocking, and **never promoted to GUI acceptance**. Failing step `Run final executable UI readiness gate`: `WebDriverError: session not created: DevToolsActivePort file doesn't exist` on `POST http://127.0.0.1:4445/session`. The executable had already been downloaded and identity-verified (job steps 4–6 `success`) and `msedgedriver` was listening (`[direct-driver] msedgedriver pid=1372 is listening on port 4445`). Diagnostics artifact: `XArchive-v0.2.1-pre1-wdio-diagnostics-36838400270` (ID `11150613263`, 662 629 bytes).
- Classification: **environment class** (`COMPUTER_USE_UNAVAILABLE` — a hosted runner has no interactive desktop session, so WebView2 cannot create a DevTools session). It is **not** a regression and **not** a product defect: `v0.2.0` Run `36705896154` (job `109862080081`) failed on the **same step** with the **same error text**, and every recorded `windows-release.yml` run to date ends in `failure` for this reason. Per the Tooling Failure Policy this produced no product change.
- Not validated by this run: GUI/DPI/keyboard matrix, extension install/pairing/archive, Named Pipe/ACL/reparse/long-path/Unicode, and Explorer/taskbar/Alt+Tab/tray icon rendering at each size. These remain `NOT_RUN` in [`windows-queue.md`](windows-queue.md). A green build is not acceptance.



## 2026-10-01 Windows batch — source 6be3269

- Owner: Windows Platform Owner; branch `codex/windows-validation-6be3269`; source/handoff `6be3269f73877651683e6165f0108ef2526efca0` (includes implementation `279d726`). Fetched main/dev and checked out through Git, tracked tree clean. Validation revision is the commit containing this record. No source changes, no release/tag dispatch. Previous `e844ffe` describes published 7910033 only and does not validate this source.
- Environment: native Windows x64, PowerShell, Node 24.19.0, npm 11.19.0 via npx, Rust 1.98.0, Python 3.12.14; existing WebView2/Edge environment retained. Computer Use initially found no hidden target; visible interactive launch provided equivalent real app-specific window evidence. Display scaling was not changed or certified; multi-DPI results are NOT_RUN.
- Build origin: `npm run build:portable:windows --workspace desktop` with XARCHIVE_RELEASE_TAG=v0.2.1-pre1; fresh optimized Desktop PASS. Initial output under validation-artifacts was rejected by the packaging path guard before build; corrected to isolated dist-portable and succeeded (operator configuration, not product defect). PyInstaller 6.22.3 fresh worker PASS; cargo build --locked -p xarchive-native-host --release PASS. Full assembled with fresh Desktop binary reuse only after successful fresh build, current worker override and refreshed Native Host. Full location: `dist-portable/windows-6be3269-20261001-prerelease`. Local validation build, not published pre-release; version files intentionally remain 0.2.0.
- Artifact SHA256: pre-release Desktop `3e310ef8a3350b7bd4829fb661db26c317a6d6416d18ec46e9fc328fa770cd8a`; stable-channel Desktop `d73c0174dbd49ff4938dddc58a540ec65ac9c195d13482823d97115e75b0bbc1`; Native Host `9f6c4c57ffadeac8a5af5c8123cf4e18b98c0d0379baf54c5bf74d7289813b84`; worker `1dfca7fdbdb95ca47b161925dc71baa170cedd5eb7957d625198fc652494eaf3`. Stable-channel optimized EXE built with tag v0.2.1 at `dist-portable/windows-6be3269-20261001-release`. Evidence directory: `validation-artifacts/windows-6be3269-20261001/` (ignored/local).
- Desktop npm module tests PASS 179/179; Python test_proxy_mode PASS 8/8. Rust changed-module scope: core 31/31, desktop 161/161, download 26/26, sidecar-supervisor 11/11 PASS with PYTHON pointing to `.venv-windows-validation/Scripts/python.exe`. First run desktop 158/161, three discovery fixtures failed because default python3 did not yield a functioning worker; existing PYTHON override fixed the environment, identical module scope rerun exit 0. Preserve both logs; no assertion/code weakening. Initial compilation also warned incremental cache finalization access denied, successful rebuild/test result retained.
- WQ-LOGS-020-02: backend channel/default subcheck PASS: real optimized startup log `level=debug`, channel=prerelease/channel_default=debug/effective_level=debug/user_override=none; runtime/network/transport/executor debug lines present. Full item FAIL: actual Logs page lowest-level ComboBox is Info despite effective Debug; screenshot and accessibility tree observed. Root cause `desktop/src/pages/logs-page.jsx:16` hardcodes useState("info"). Evidence `prerelease-logs-tree.txt`, `prerelease-logs-ui.txt`, package logs. CROSS_PLATFORM_CHANGE_REQUIRED: initialize/synchronize display filter from effective backend level with appropriate shared tests. Real child-process module coverage NOT_RUN, not inferred from unit tests.
- WQ-LOGS-020-03: stable channel/default subcheck PASS: independent startup `level=info`, channel=release/channel_default=info/effective_level=info; debug startup lines absent, info frontend diagnostics present. First simultaneous release launch had Named Pipe access-denied warning due to active pre-release instance; stopped it and separately relaunched release, clean startup log. This is isolated-instance fixture contamination, not new product failure. Real progress/error coverage NOT_RUN.
- WQ-LOGS-020-04: pre-release Error UI save + restart subcheck PASS: Settings initially Debug; selected Error and saved; config logging.level=error; restart new log header level=error. Silent, stable-channel override and old-config upgrade remain NOT_RUN. WQ-LOGS-020-01 current-size spacing and green checked checkbox observed PASS subchecks; three DPI/narrow/focus matrix NOT_RUN. WQ-LOGS-020-05 title-bar archive icon observed; Explorer/property/taskbar/all sizes NOT_RUN.
- Current new Full Dashboard/SQLite/native window observed. No new Sidecar GUI handshake/completed download/install acceptance is claimed. UI/settings/known-folder/cancel/proxy-real-boundary matrix remains manual as listed in windows-manual-steps.md; default Downloads redirection needs isolated user/VM, real archive dedicated non-personal account, long child-process fixture still required. System Proxy Batch B PLANNED, NOT_RUN/IMPLEMENTATION_NOT_READY; Telegram paused/out of scope. Historical Native Host installation WINDOWS_FAIL remains unclosed.
- Cleanup: exact batch app processes stopped, driver not started; final process and port inventory `cleanup-final.json`. No full regression or repeated WDIO recipe. Reuse unrelated historical PASS only; current shared changes invalidate old UI/behavior PASS. Blocks development=false; release acceptance still open. Next Owner Cross-platform Owner for log filter defect and Git reconciliation; Windows retains pending platform matrix for next concentrated batch.

### 2026-09-30 Windows candidate validation (`5f952d0`)

- Owner: Windows Platform Owner. Formal input/handoff: `5f952d0091644775ec633a9e62a1696fe71fd471`; underlying integration tree: `60110a6f000887d134bf5fc46a7b2d6da91635a8`. Started from clean tracked `release/v0.2.0` at `5a841a9`, fetched `main`/`dev`, and created `codex/windows-validation-5f952d0` through Git. Initial generic fetch did not acquire the new main ref; explicit `git fetch origin main dev` did. Existing untracked caches, user data and artifacts were preserved. No filesystem handoff.
- Workflow implementation: `951453cd55a06b913836e9fa6dd30314ffbf631c`; test-fixture implementation: `4d1d9148dd13f187a734aa994e8ba4c796a33bd0`. Validation record revision is the Git commit containing this section, to be resolved from history; no self-referential SHA is invented. Application/Native Host and CI worker derive from `951453c`; the only subsequent code change is the configuration test fixture, which was present as an uncommitted test-only diff during local tests/build. No runtime source changed after the input revision.
- Environment: Windows 11 Pro for Workstations Insider Preview `10.0.29671`, x64; Node `24.19.0`, system npm `11.17.0`, validation npm `11.19.0` via npx; Cargo `1.98.0`; CI Python 3.12 / PyInstaller `6.22.3`; Bandizip CLI `8.0 Beta x64`. Working directory: `E:\Shiraishi\VSCode Workspace\Tw2Tg`. Evidence: `validation-artifacts/windows-batch-20260930-5f952d0/` (ignored).

| Item | Result | Command / evidence / limit |
|---|---|---|
| Dependency resolution | PASS | `npx --yes --package npm@11.19.0 -c "npm ci --no-audit --no-fund"`, 546 packages; restricted first attempt had npm-cache EPERM, identical elevated command passed. npm install-script warnings retained in `npm-ci.log`. |
| WQ-REL-020-G1 disposition | PASS (review + targeted regression) | `5a1ecf1` allocated-port snapshot and `8a1714f` native-core shell/banner/pinned-driver behavior already exist in current launcher/patch/config. Disposition: **superseded; do not merge the old branch wholesale**. Current code has allocated port deduplication/fallback, snapshot before upstream teardown, direct driver spawning patch, path/runtime opt-ins and newer direct mode. Historical protocol/schema/docs diffs from the old branch must not be replayed. Native driver session acceptance is not implied. |
| Targeted regression | FAIL before fixture correction; PASS after | `node --test desktop/test/patch-wdio-tauri-service.test.mjs desktop/test/wdio-config.test.mjs desktop/test/wdio-tauri-service.test.mjs extension/tests/content.test.js`: 51/52 then **52/52**. Windows driver-directory fixture created no `msedgedriver.exe`, although config checks it before updating PATH. Added a zero-byte stand-in that is never executed; no production check weakened. `targeted-tests.log` / `targeted-tests-fixed.log`. Small shared test adjustment: `CROSS_PLATFORM_REVIEW_REQUIRED`. |
| Extension module | PASS | npm 11.19.0 `npm run test --workspace extension`: **33/33**; `npm run check --workspace extension`: PASS. New credentials/trailing-path assertions pass. |
| Installed dependency patch | PASS | `node desktop/scripts/patch-wdio-tauri-service.mjs`: all four files already patched, no missing-pattern warning (`patch-idempotency.log`). |
| WQ-SEC-PERMS-01 / G2 execution | PASS; default-branch closure pending | Explicit `permissions: contents: read` and checkout `persist-credentials: false`; no write permission required for artifact upload. [Worker run 36700506149](https://github.com/15699122/Tw2Tg/actions/runs/36700506149) SUCCESS at `951453c`, build/verify/package/upload succeeded. Downloaded artifact contains EXE and `_internal/python312.dll`; EXE SHA-256 `f7d51e7f2ec34a4adb49b4e816e8fdcc199fa242411916157823a0f6a96f72f2`. `--help`, v2 ready/capabilities, unknown field -> INVALID_COMMAND, shutdown/exit 0 PASS. Alert #2 remains open on `refs/heads/main`; default-branch integration/rescan is required. Full CI log download returned 403; run/step metadata and downloaded artifact were available and retained. |
| Fresh application / Native Host / Full directory | PASS (local build/assembly) | npm 11.19.0 `npm run build:portable:windows --workspace desktop`, no binary reuse; `cargo build --release --locked -p xarchive-native-host`. Final assembly uses this run's CI worker. Output: `dist-portable/windows-batch-20260930-5f952d0-full`. Initial output under validation-artifacts was rejected by the documented dist-portable namespace guard, corrected as an invocation error. Full script permits absent aria2; official aria2 `release-1.37.0` and gallery-dl `2026.09.20` were subsequently assembled as in release workflow. Final external probes: aria2 1.37.0, gallery-dl 1.32.13:2026.09.20. |
| Local four-shape archive rehearsal | PASS; formal release gate NOT_RUN | Bandizip `c -fmt:7z -l:1 -r`, `t`, `x` on application / full / dependencies / extension; all magic `377ABCAF271C`; SHA-256 roundtrip **1/77/74/12** files. Full includes 8 `.pyd`, python312.dll, external dependencies; filtering removes startup SQLite/log files. EXE FileVersion/ProductVersion and package manifest are `0.2.0`, installation tag `v0.2.0`. App SHA-256 `1b7c26e0e45422686682adf6a49b8979f8b5e2cecd6f0f36b9d191a4320b369b`; Native Host `0724c83e667c57322b8ad1642103941bc50b15eb06bbf47dbd7ab5b28e4941ea`. These are locally staged shape probes, **not seven-asset Release candidates**: dependency staging retains the Full manifest; no formal release-manifest/SHA256SUMS/tag/source publication gate was run. No v0.2.0 tag or release was created. |
| G4 native GUI | BLOCKED / COMPUTER_USE_UNAVAILABLE | Exact Full EXE launched, PID 53324; logs contain `react_mount_completed`, initial IPC started/settled. Computer Use found XArchive handle 1055818, but screenshot displayed another foreground app; activation returned `window is not a usable app window`, bounded fresh inventory no longer returned XArchive. No coordinate input was issued. Logs are startup evidence only, not Dashboard/interaction/restart PASS. |
| G5 browser/pairing/completed archive | BLOCKED / NOT_RUN | Stable observable application prerequisite unavailable; no isolated profile installation, real cookie, task/output or restart acceptance executed. Dedicated non-personal test accounts remain prerequisite. |
| G6 Telegram | NOT_RUN | Controlled account absent and advertised release scope has not been decided; Cross-platform Owner must record whether Telegram is included. |
| G7 filesystem/transport | NOT_RUN | No complete candidate-specific Named Pipe/ACL/reparse/long-path/Unicode harness was executed. Historical unrelated PASS is retained only for its original revision, not promoted to GA acceptance. |
| Native E2E / full regression | NOT_RUN | Old deterministic upstream capability recipe not retried. G1 changes are already present and targeted tests cover the disposition; no runtime/protocol/Rust/Python changes warrant rerunning the full suite. Final release scope still requires its separate gates. |
| Cleanup | PASS (this run) | Exact launched package PID stopped; final worker/msedgedriver/tauri-driver inventory empty. No native driver was launched, so no new driver listener was created. |

Native Host manifest in the local Full directory contains its build-directory absolute executable path. It is not installation acceptance and must be regenerated/registered through the supported installation flow after extraction; no Registry or browser settings were changed. Official 7zr download encountered transport EOF; existing Bandizip CLI successfully proved real 7z format instead. Original failure records remain visible.

Follow-up: Cross-platform Owner reviews the shared fixture adjustment and integrates Windows evidence/workflow fix; default-branch CodeQL rescan must close alert #2. Release SHA must be frozen after integration, then formal asset and manual gates executed. Windows retains GUI/install/filesystem execution responsibility. Manual steps and statuses are in windows-queue.md; release remains blocked by unmet acceptance gates, not by Linux development.

This document records completed Windows validation batches. It does not replace the current queue in windows-queue.md or the active state in ../status/platform-handoff.md.

### 2026-09-30 completed Windows pre-release migration

- Owner: Windows Platform Owner; branch: `release/v0.2.0`. Formal Linux handoff: `7a3374b7af1a647addb0b9ab4e50e315468b6d26`; batch: `ba0f8aa1fe4632917a3dde5990103508a44225b4`. Windows workflow fixes: `e6d71ad44f8ef8cd8539259896787426e389ad67` (vacant-target native exit handling) and `2ec1aba78579932e7788fb3b3bb9e5fca597f906` (publishing secret). All ten dry runs and the first four two-asset draft runs used the vacancy fix revision; the remaining six draft runs used the publishing-secret revision. No shared contract/schema/planner/ledger change. Existing untracked local caches and evidence preserved; no Linux state-changing action.
- **PASS**: all ten Windows dry runs, then all ten draft runs with local downloaded asset SHA-256/size/source checks and applicable external manifest/SHA256SUMS. All ten drafts were gated together before publication. All ten public pre-releases and all 41 public asset download URLs returned HTTP 200, with GitHub asset digests still matching the independently downloaded files. New tags point to the frozen historical source SHAs.
- **PASS**: all ten old Release objects retired only after public downloads passed. Before each deletion, old Notes linked the replacement and recorded origin/backup; exact Release IDs, asset IDs, downloads, hashes and Notes snapshots were saved. All 36 frozen old files were independently hash/size verified in backup. Every old Git tag remains at its frozen SHA. Deleted old Release/asset IDs, download counters and old download URLs cannot be restored losslessly.
- User configured repository secret `PRERELEASE_MIGRATION_TOKEN` at `2026-09-30T06:13:55Z`; publishing at revision `2ec1aba` subsequently passed. No credential value was read or recorded.
- PAT tag creation triggered ordinary `windows-release.yml` run `36677316346`; Windows Owner cancelled it before build/assets/upload and temporarily disabled that workflow to avoid its ordinary `--clobber` upload path interfering with migration. This run is **CANCELLED / build and upload NOT_RUN**, not product FAIL. After all ten drafts passed, workflow state was restored to its original `active` state and re-read. No ordinary build asset was uploaded.
- `windows-release.yml` same-source gates: **PASS (static review only)** for explicit tag checkout, dispatch ref equality, HEAD/tag/push parity, complete asset manifest/hash/size/SHA256SUMS and same-run WDIO executable identity. No fresh ordinary release acceptance was run. Migration used its separate collision-rejecting workflow and no clobber.
- Historical runs `36672663525` (vacancy native-exit FAIL), `36674672997` (HTTP 403 tag creation FAIL), old `36651747470` Rust FAIL and pre.16 WDIO FAIL remain failures. Earlier checkpoint sections below describe their then-current blockers; this completion supersedes pending/blocker status without rewriting those observations. All four registration CodeQL language checks eventually passed in run `36672108116`.
- `v0.2.0-pre5` used isolated artifact run `36655693790`, expiry `2026-12-29T01:33:36Z`, rechecked unexpired before dry and draft dispatch. Executable SHA-256: `4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4`. The old pre.6 two polluted assets remain backup evidence only, never accepted publication input. ZIP-era pre6 popup/options and historical package development-file warnings remain documented frozen-payload warnings.
- GUI/startup, browser/Native Host installation and full product regression: **NOT_RUN**. Native Computer Use inventory was available; no migrated XArchive window or installation was exercised. These package migration results do not promote historical GUI/WDIO/installation outcomes to PASS. See `windows-queue.md`. Planner tests 7/7, YAML parse and final diff checks cover the Windows workflow-only corrections.
- Local detailed evidence: `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\migration-20260930-7a3374b`; each run has dispatch/run/log/evidence/accepted records; draft assets were downloaded; `ten-draft-publication-gate.json`, per-target `published.json`, and retirement before/annotated/retired snapshots establish ordering. Backup: `/home/shiraishi/xarchive-pre-release-backup` (Windows read via `W:\home\shiraishi\xarchive-pre-release-backup`).

| Source tag | New pre-release | Source SHA | Dry run | Draft run | New / old Release ID | Assets |
|---|---|---|---|---|---|---|
| `v0.1.1-pre.1` | [v0.1.1-pre1](https://github.com/15699122/Tw2Tg/releases/tag/v0.1.1-pre1) | `5afc1b8289fbd38792280431159136531ae138da` | [36672977464](https://github.com/15699122/Tw2Tg/actions/runs/36672977464) | [36674300106](https://github.com/15699122/Tw2Tg/actions/runs/36674300106) | 399747460 / 390068354 | 2 |
| `v0.1.1-pre.2` | [v0.1.1-pre2](https://github.com/15699122/Tw2Tg/releases/tag/v0.1.1-pre2) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | [36673063515](https://github.com/15699122/Tw2Tg/actions/runs/36673063515) | [36674393927](https://github.com/15699122/Tw2Tg/actions/runs/36674393927) | 399747967 / 390459522 | 2 |
| `v0.1.1-pre.3` | [v0.1.1-pre3](https://github.com/15699122/Tw2Tg/releases/tag/v0.1.1-pre3) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | [36673130141](https://github.com/15699122/Tw2Tg/actions/runs/36673130141) | [36674492512](https://github.com/15699122/Tw2Tg/actions/runs/36674492512) | 399748496 / 390833384 | 2 |
| `v0.2.0-pre.1` | [v0.2.0-pre1](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre1) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | [36673250103](https://github.com/15699122/Tw2Tg/actions/runs/36673250103) | [36674586528](https://github.com/15699122/Tw2Tg/actions/runs/36674586528) | 399749017 / 391884476 | 2 |
| `v0.2.0-pre.2` | [v0.2.0-pre2](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre2) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | [36677235313](https://github.com/15699122/Tw2Tg/actions/runs/36677235313) | 399763919 / 392292514 | 4 |
| `v0.2.0-pre.3` | [v0.2.0-pre3](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre3) | `baf0b241237afbd9fb7435f96403af2de5598d91` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | [36677392476](https://github.com/15699122/Tw2Tg/actions/runs/36677392476) | 399765204 / 392324859 | 4 |
| `v0.2.0-pre.4` | [v0.2.0-pre4](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre4) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | [36677519304](https://github.com/15699122/Tw2Tg/actions/runs/36677519304) | 399765983 / 392350523 | 4 |
| `v0.2.0-pre.6` | [v0.2.0-pre5](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre5) | `ac586e609337947aeb51de8f5cce3185efc8995e` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | [36678289636](https://github.com/15699122/Tw2Tg/actions/runs/36678289636) | 399772991 / 392465704 | 7 |
| `v0.2.0-pre.7` | [v0.2.0-pre6](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre6) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | [36677644817](https://github.com/15699122/Tw2Tg/actions/runs/36677644817) | 399766680 / 392743241 | 7 |
| `v0.2.0-pre.16` | [v0.2.0-pre7](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre7) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | [36677777905](https://github.com/15699122/Tw2Tg/actions/runs/36677777905) | 399767938 / 399271706 | 7 |

#### Final published asset hashes

| Target | Asset | Bytes | SHA-256 |
|---|---|---:|---|
| v0.1.1-pre1 | XArchive-v0.1.1-pre1-windows-x64.7z | 4125276 | `6614c6b5551a248c46b0bb1bdf0d84c18b7a5b7d130f1caa755e70d53bd7744a` |
| v0.1.1-pre1 | XArchive-v0.1.1-pre1-windows-x64.exe | 17477632 | `8e8277555a945f19d4dbfca84a1ec772a62b28f931320a6a5307b1bbbb8f2b83` |
| v0.1.1-pre2 | XArchive-v0.1.1-pre2-windows-x64.7z | 4132005 | `a91e1b199c770b839cf6e12cfaf8b5454c2e20096d45eee04d8d8fa6ee9f8de8` |
| v0.1.1-pre2 | XArchive-v0.1.1-pre2-windows-x64.exe | 17507328 | `f74b9d89338aadc0601ea42f41153e5e8da0cb61fd83bff247a6ea491df15d28` |
| v0.1.1-pre3 | XArchive-v0.1.1-pre3-windows-x64.7z | 4226964 | `01a291d638bcd8ab54000399ced56495707e7f903ad54f216982f05816d4f8a3` |
| v0.1.1-pre3 | XArchive-v0.1.1-pre3-windows-x64.exe | 18023936 | `b5c6fdcce79fa3cb10635d4f9eaea6c436a5d5af5834ae24ecfde64403b0e8ab` |
| v0.2.0-pre1 | XArchive-v0.2.0-pre1-windows-x64.7z | 4232181 | `a493e3c2dc44428b6141f49c2f16b400689a9aaa0bffc32118e74e095e365dec` |
| v0.2.0-pre1 | XArchive-v0.2.0-pre1-windows-x64.exe | 18082304 | `9ca2686e9c8697db8f6dd8ac47c60762c0178e2d08daa8d7e8c599a01eebb156` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64-full.7z | 34253140 | `c10dbe5a1ff67fc31f5deef3c45f118d53d7bd7ed017010a1314b18626425aec` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64-repository-dependencies.7z | 5917419 | `e52b2650c62f5c9db96f09bf41656fe20ca3626ef8ae78a3aa1417c85fcd366e` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64.7z | 4270811 | `883224127a98f5d708cf2622ee6f7ebbd6b475d0296f3f0c326db29b61aaa530` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64.exe | 18249728 | `75d1c0dbf0d3dbbd8f136b8aefd219f228a973d1b615b0127c73a445ca70e7da` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64-full.7z | 34257241 | `dbbb38fe905717f97001e44250e9c53025e65e33df1ecec0f4e86cb281b799f2` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64-repository-dependencies.7z | 5916757 | `131c4292d9a49aa4be6e7c53ab5fec34ec857ad1ebb85bb514e739c9b9474dc4` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64.7z | 4268968 | `3b6c09911ce7072993e78ab4db649dc8d7a31d6684d05ec63d0eabc21ecfd6dd` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64.exe | 18250240 | `a25e7132e546045096b9acb96ea1c3c9f2e25353733871a905dcb8eee1743125` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64-full.7z | 34258098 | `98704ab4fdddfcd356dac9822dadcb9f31031713dee8a8e9b865d554a60dcb3b` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64-repository-dependencies.7z | 5918180 | `6fa1f9422af2faae987673d10fdb10be26ce98ca8fd42eeccf55db47cba96bc4` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64.7z | 4266292 | `b17ba34bbc21ff86d1bfc89fe4610f583af0e3ce99b6595e1f124274fb7475d8` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64.exe | 18250240 | `bf42e1456caec9f31d963f9bbd301ad3e16ccf65fa6462efaf67f13ee5481b9e` |
| v0.2.0-pre5 | SHA256SUMS-v0.2.0-pre5.txt | 538 | `ac29fc7e2286c1b2c9a0a426beb20d0013383871e2e87b5d95969cb758a894f3` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-extension.7z | 5280 | `7641105d80e61530c5a2c27c818c3f7404b0051ac44dfc80465e147f5016ebae` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-release-manifest.json | 1350 | `e64928a53a1a37d4e944cd881a3b348138f18f55e335e8bc2a652011171ac68a` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64-full.7z | 34319492 | `4424993b6c0fbbb3df8cfb3359635bb11c52d644fd855c4d8afa0f5355643758` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64-repository-dependencies.7z | 6053977 | `ad74ad738aeecc1e39b6a28e9d3fb499bcd445ec931f89167aab4c239a7a1df1` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64.7z | 4406570 | `6b73ba6aaa1f233b81a6374e3be084195d9703dd965c79dd58ddf50d6a133bc3` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64.exe | 18258944 | `4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4` |
| v0.2.0-pre6 | SHA256SUMS-v0.2.0-pre6.txt | 538 | `495b638e23f48f0e2e73a4d1265c00e6f2f666a7a18d7c5ef82af4d4b62f3b33` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-extension.7z | 5606 | `c616163037f3f9056a431cb0c51b3d5d12b170eaf24f56d43d203b27b0630341` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-release-manifest.json | 1350 | `f3ae37a8d85e962f6c95c7a5949893f10d1379a7e71e706ab2ea1bf96974c956` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64-full.7z | 34322489 | `d95f5bfd43e8c554880114de6ae157daeddf40ef845bd1b79b6303082edcd236` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64-repository-dependencies.7z | 6055952 | `d801df21e5cb68a6b4cdf592ea8341206c1219a6ed26fd8aa8ed6808b2d0685f` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64.7z | 4270916 | `7cbc14a9dc20768360f5623e7acbbc91aa0175e7f57a203aecef9c647e8c8e00` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64.exe | 18258944 | `9ab7a616acfaf6ab88ecd5db055772fc42c7d27bd64b69e3e3e64c1302e893f4` |
| v0.2.0-pre7 | SHA256SUMS-v0.2.0-pre7.txt | 538 | `06605c923f008d8eb729629ab166f74f985790eb9c137e4786c1d509024c9f8c` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-extension.7z | 12380 | `87cf3e8eb78358e53cc8b74fa76853d8a53d8d92cf51d0ba50c14feb18b941f5` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-release-manifest.json | 1351 | `ea5f2dd9a1a015f1535fad70d3d4b9c04699988baf39543cb513352ec2f593ef` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64-full.7z | 34535087 | `8a04c36f65d9cac3f8a5af8dc0a87d42c79a5aae1e4bc15d9877dc8d623921fd` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64-repository-dependencies.7z | 6078141 | `74e11b431f3bc506c599449c085e8413c782567fc1a5210fd13491d2f5c77f8d` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64.7z | 4471230 | `64b2440cba04f28e5afe5d33ab614614db0684e134248bf7c43c82fc10551e90` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64.exe | 19264512 | `615b60c16b38bf851c1efc65e28e8bb4bf757fc77af07d5f92b207d56f01121d` |

#### Old asset retirement inventory

| Old tag | Asset ID | Asset | Downloads at retirement | Backup |
|---|---:|---|---:|---|
| v0.1.1-pre.1 | 568310070 | XArchive-v0.1.1-pre.1-windows-x64.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.1/XArchive-v0.1.1-pre.1-windows-x64.7z` |
| v0.1.1-pre.1 | 568309988 | XArchive-v0.1.1-pre.1-windows-x64.exe | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.1/XArchive-v0.1.1-pre.1-windows-x64.exe` |
| v0.1.1-pre.2 | 569572099 | XArchive-v0.1.1-pre.2-windows-x64.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.2/XArchive-v0.1.1-pre.2-windows-x64.7z` |
| v0.1.1-pre.2 | 569572054 | XArchive-v0.1.1-pre.2-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.2/XArchive-v0.1.1-pre.2-windows-x64.exe` |
| v0.1.1-pre.3 | 570548950 | XArchive-v0.1.1-pre.3-windows-x64.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.3/XArchive-v0.1.1-pre.3-windows-x64.7z` |
| v0.1.1-pre.3 | 570548900 | XArchive-v0.1.1-pre.3-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.3/XArchive-v0.1.1-pre.3-windows-x64.exe` |
| v0.2.0-pre.1 | 573929132 | XArchive-v0.2.0-pre.1-windows-x64.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.1/XArchive-v0.2.0-pre.1-windows-x64.7z` |
| v0.2.0-pre.1 | 573929085 | XArchive-v0.2.0-pre.1-windows-x64.exe | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.1/XArchive-v0.2.0-pre.1-windows-x64.exe` |
| v0.2.0-pre.2 | 575996364 | XArchive-v0.2.0-pre.2-windows-x64-full.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64-full.7z` |
| v0.2.0-pre.2 | 575996344 | XArchive-v0.2.0-pre.2-windows-x64-repository-dependencies.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.2 | 575996290 | XArchive-v0.2.0-pre.2-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64.7z` |
| v0.2.0-pre.2 | 575996254 | XArchive-v0.2.0-pre.2-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64.exe` |
| v0.2.0-pre.3 | 576353696 | XArchive-v0.2.0-pre.3-windows-x64-full.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64-full.7z` |
| v0.2.0-pre.3 | 576353612 | XArchive-v0.2.0-pre.3-windows-x64-repository-dependencies.7z | 8 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.3 | 576353555 | XArchive-v0.2.0-pre.3-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64.7z` |
| v0.2.0-pre.3 | 576353495 | XArchive-v0.2.0-pre.3-windows-x64.exe | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64.exe` |
| v0.2.0-pre.4 | 576383855 | XArchive-v0.2.0-pre.4-windows-x64-full.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64-full.7z` |
| v0.2.0-pre.4 | 576383830 | XArchive-v0.2.0-pre.4-windows-x64-repository-dependencies.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.4 | 576383788 | XArchive-v0.2.0-pre.4-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64.7z` |
| v0.2.0-pre.4 | 576383744 | XArchive-v0.2.0-pre.4-windows-x64.exe | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64.exe` |
| v0.2.0-pre.6 | 577042844 | XArchive-v0.2.0-pre.6-windows-x64.7z | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.6/XArchive-v0.2.0-pre.6-windows-x64.7z` |
| v0.2.0-pre.6 | 577042799 | XArchive-v0.2.0-pre.6-windows-x64.exe | 1 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.6/XArchive-v0.2.0-pre.6-windows-x64.exe` |
| v0.2.0-pre.7 | 578464042 | SHA256SUMS-v0.2.0-pre.7.txt | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/SHA256SUMS-v0.2.0-pre.7.txt` |
| v0.2.0-pre.7 | 578464017 | XArchive-v0.2.0-pre.7-extension.zip | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-extension.zip` |
| v0.2.0-pre.7 | 578464040 | XArchive-v0.2.0-pre.7-release-manifest.json | 6 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-release-manifest.json` |
| v0.2.0-pre.7 | 578463954 | XArchive-v0.2.0-pre.7-windows-x64-full.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64-full.7z` |
| v0.2.0-pre.7 | 578463913 | XArchive-v0.2.0-pre.7-windows-x64-repository-dependencies.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.7 | 578463873 | XArchive-v0.2.0-pre.7-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64.7z` |
| v0.2.0-pre.7 | 578463791 | XArchive-v0.2.0-pre.7-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64.exe` |
| v0.2.0-pre.16 | 598557936 | SHA256SUMS-v0.2.0-pre.16.txt | 6 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/SHA256SUMS-v0.2.0-pre.16.txt` |
| v0.2.0-pre.16 | 598557860 | XArchive-v0.2.0-pre.16-extension.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-extension.7z` |
| v0.2.0-pre.16 | 598557937 | XArchive-v0.2.0-pre.16-release-manifest.json | 6 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-release-manifest.json` |
| v0.2.0-pre.16 | 598557715 | XArchive-v0.2.0-pre.16-windows-x64-full.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64-full.7z` |
| v0.2.0-pre.16 | 598557637 | XArchive-v0.2.0-pre.16-windows-x64-repository-dependencies.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.16 | 598557564 | XArchive-v0.2.0-pre.16-windows-x64.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64.7z` |
| v0.2.0-pre.16 | 598557462 | XArchive-v0.2.0-pre.16-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64.exe` |

### 2026-09-30 pre-release migration Windows preflight — 7a3374b

#### Draft continuation: historical-source tag permission blocker

- All ten Windows `upload=false` runs passed their source/payload/manifest/asset gates at workflow revision `e6d71ad44f8ef8cd8539259896787426e389ad67`; evidence downloaded and accepted locally. Run IDs in order: `36672977464`, `36673063515`, `36673130141`, `36673250103`, `36673316986`, `36673405745`, `36673504341`, `36673591851`, `36673841917`, `36673929521`.
- Four two-asset `upload=true` runs PASS, with draft assets downloaded and SHA-256 checked: `v0.1.1-pre1`=`36674300106`, `v0.1.1-pre2`=`36674393927`, `v0.1.1-pre3`=`36674492512`, `v0.2.0-pre1`=`36674586528`. All four remain draft/unpublished; old Releases remain available.
- `v0.2.0-pre2`, run `36674672997`: **FAIL** at `Publish draft pre-release with the verified assets`, specifically POST `git/refs`, `Resource not accessible by integration (HTTP 403)` / `Tag creation failed: v0.2.0-pre2`. Source and migrated four-asset gates PASS before this step; no pre2 tag/draft exists. Evidence is archived under `validation-artifacts/migration-20260930-7a3374b/draft/v0.2.0-pre2-failed-36674672997/`. The verification record's `uploaded=true` means upload was requested, not that it succeeded; `job_status=failure` remains authoritative.
- Classification: GitHub credential/permission blocker, not an asset mismatch or GUI/product failure. Repository secrets/rulesets listing returned no existing publishing secret/ruleset. The workflow explicitly grants contents:write, but GitHub's historical-workflow-file permission guard also requires workflow authority when tagging commits with workflow files differing from the default branch. Primary references: https://docs.github.com/en/rest/git/refs#create-a-reference and https://docs.github.com/en/rest/releases/releases#create-a-release; GITHUB_TOKEN cannot request Workflows:write (https://docs.github.com/en/actions/tutorials/authenticate-with-github_token).
- Windows-owned fix prepared: `.github/workflows/pre-release-series-migration.yml` binds `PRERELEASE_MIGRATION_TOKEN` only to the draft publishing step and fails explicitly if missing. Required repository-scoped fine-grained token permissions: Contents:write and Workflows:write; no broad local gh OAuth token is copied into the runner. Planner, ledger, payload and non-clobber/collision gates are unchanged. Remote credential-dependent verification is BLOCKED until the user installs that secret.
- Independent follow-ups: original 36-file backup verification PASS. Computer Use `@oai/sky` list_apps/list_windows is available; no XArchive window was opened and GUI/installation tests are **NOT_RUN**, not COMPUTER_USE_UNAVAILABLE. Full regression NOT_RUN because only Windows workflow/credential plumbing changed and twenty intended asset migration runs are the relevant scope. No publish or retirement occurred.

#### Windows continuation: registration and first dry-run failure

- Windows gh login verified for account `15699122` (repo/workflow scope); no token was recorded. `workflow view --ref` and branch dispatch initially returned 404 because the workflow was absent from default-branch registration.
- Formal registration PR: https://github.com/15699122/Tw2Tg/pull/6, merged as `dd57861d93aaa94beb90c7774777c730b79919e5`. Three historical add/add conflicts were resolved by retaining the already handed-off release files; reconciliation merge `f1a2ba3c3eea0242b9e489fa8b205a40b4de27d0` is tree-identical to `66c89a9`. Actions/JavaScript/Python CodeQL checks passed at merge; Rust CodeQL was still in progress (not reported PASS).
- Old backup verification: 36/36 frozen Release asset files matched both sizes and SHA-256 when read from `W:\home\shiraishi\xarchive-pre-release-backup`; detailed file/path/digest evidence is `validation-artifacts/migration-20260930-7a3374b/old-backup-verification.json`. This includes the two polluted pre.6 files as historical evidence only.
- `v0.1.1-pre1`, `upload=false`, run `36672663525`, workflow SHA `f1a2ba3c3eea0242b9e489fa8b205a40b4de27d0`: **FAIL** at `Refuse an existing target tag or release`. It printed `target v0.1.1-pre1 is unused`, then Actions reported `Process completed with exit code 1`. Source material verification/repackaging was NOT_RUN; no upload/tag mutation occurred. Log and evidence artifact were downloaded under `validation-artifacts/migration-20260930-7a3374b/dry/v0.1.1-pre1-failed-36672663525/`.
- Classification/root cause: Windows PowerShell Actions wrapper propagates the final native command's nonzero `$LASTEXITCODE`; `gh release view` returning 1 for the expected missing Release was not reset by `Write-Host`. Windows-owned fix in `.github/workflows/pre-release-series-migration.yml` replaces that query with a successful paginated Release listing, checks API/fetch errors explicitly, and handles `git show-ref` absence code 1 separately. No shared planner/schema/contract change. Local real-vacancy check PASS with native exit 0. Remote rerun is required; prior FAIL remains unchanged.

- Owner: Windows Platform Owner.
- Linux source branch/revision: `release/v0.2.0`, `7a3374b7af1a647addb0b9ab4e50e315468b6d26`; read-only `wsl -d Ubuntu -- bash -lc` Git inspection confirmed clean source worktree. Batch content: `ba0f8aa1fe4632917a3dde5990103508a44225b4`.
- Windows native worktree: `E:\Shiraishi\VSCode Workspace\Tw2Tg`; Git checkout matches the handoff. Existing untracked local dependencies/artifacts were preserved; no uncommitted source changes were included. Current edits are validation/Plan documentation only.
- Environment: Windows 11 Pro for Workstations Insider Preview `10.0.29671`, 64-bit; Node `v24.19.0`; official portable GitHub CLI `2.102.0` in `validation-artifacts/migration-tools/bin/gh.exe`. Official ZIP digest verified: `ae64e556ecc240b200f7eba60d550e4bb60d78e860e69dd88c449405b86067f4`.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Formal Git synchronization | PASS | `git fetch origin refs/heads/release/v0.2.0:refs/remotes/origin/release/v0.2.0`; `git checkout 7a3374b`; `git rev-parse HEAD`; `git status --short --branch` | HEAD equals handoff; tracked tree clean; later local branch `release/v0.2.0` created at the same SHA | Default fetch refspec only covers old branches; explicit fetch is required |
| Linux source state | PASS | Read-only WSL Git status/HEAD inspection | Branch `release/v0.2.0`; clean; same full SHA | No Linux state-changing operation performed |
| Ten frozen migration mappings | PASS | Node `readMigrationLedger` on `docs/release/migration/pre-release-asset-ledger.json` | Ten unique `pre.N -> preN` entries; nine `repackage`, one `artifact` | Ledger remains unchanged |
| New target tags free | PASS | `git ls-remote origin 'refs/tags/*pre[0-9]*'` | No matching remote tag; release branch is at handoff SHA | Repeat before mutations; workflow independently rejects collisions |
| Migration planner regression | PASS | `node --test desktop/test/pre-release-migration*.test.mjs` | 7 passed / 0 failed | Planner PASS is not asset migration PASS |
| Isolated pre5 artifact availability | PASS | GET `repos/15699122/Tw2Tg/actions/runs/36655693790/artifacts` | `legacy-pre6-source-build-36655693790`; `expired=false`; expiry `2026-12-29T01:33:36Z` | Recheck immediately before pre5 dispatch; keep old pre.6 available if expired |
| Windows GitHub CLI authentication | BLOCKED | Portable `gh auth status`; `gh workflow view ... --ref release/v0.2.0 --yaml` | Not logged into any GitHub hosts; workflow view cannot run without authentication | User must complete local `gh auth login --hostname github.com --web`; never send tokens in chat |
| Default-branch workflow registration | NOT_RUN | Public GET `repos/15699122/Tw2Tg/actions/workflows` | Six existing workflows; migration workflow absent | Inspect `view --ref` and branch dispatch after auth; use formal PR/merge if registration is needed |
| Ten upload=false Windows runs | BLOCKED | Sequential dispatch order recorded in current handoff Plan | No run dispatched, no migration run ID or asset verification artifact | Depends on authentication/registration; local Linux dry runs are not substituted |
| Ten upload=true draft runs | BLOCKED | Sequential dispatch only after all ten dry runs pass | No new draft/tag created by this batch | Depends on ten Windows dry-run gates |
| Publish and old Release retirement | BLOCKED | Publish only after ten draft gates; verify links; annotate/back up old Releases before deleting objects | No Release/tag mutation performed | Depends on verified drafts/new downloads; old tags must remain |
| windows-release.yml same-source review | PASS (static review only) | Read source/tag gate, complete-asset gate and independent WDIO identity gate at handoff | Checkout uses release ref; dispatch ref must equal tag; HEAD/tag parity; manifest size/hash and SHA256SUMS; WDIO downloads same-run exe and checks run/tag/source/hash | No ordinary release run dispatched. Existing ordinary workflow upload steps use `--clobber`; forbidden for migration, whose separate workflow has no clobber and rejects occupied targets |
| GUI and installation acceptance | NOT_RUN | Manual cases remain separate from static/package gates | No migrated package exists yet; no GUI/install result collected | See queue; historical WDIO FAIL and GUI NOT_RUN remain unchanged |

### Errors and unresolved issues

- Sandboxed Git initially failed at `.git/FETCH_HEAD` (permission denied), and sandboxed HTTPS Git returned `SEC_E_NO_CREDENTIALS`; approved Windows execution resolved both. Sandboxed WSL enumeration returned `E_ACCESSDENIED`; approved read-only enumeration/inspection succeeded.
- Default `git fetch origin` succeeded but did not retrieve the release branch because the local fetch refspec only includes two earlier branches. Explicit release ref fetch succeeded. `git switch -c ... --track origin/release/v0.2.0` could not infer tracking from that restricted refspec; a local branch at the verified SHA was created without changing the refspec.
- Actual dispatch blocker: Windows gh has no authenticated GitHub host. The official CLI was installed only as a machine-local validation artifact, without changing project dependencies or source. User authentication was requested; no credentials were read or emitted.
- Current Owner stays Windows. No product/shared contract change was made or justified by preflight. Linux documentation reconciliation waits for a Windows Git handoff; this batch does not write directly into the Linux worktree.

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

### 2026-09-27 Windows batch at shared handoff `6d60429`

- Branch: `feature/u7-desktop-production-integration`. Fetched `origin` and fast-forwarded the formal Windows working tree from `f269efea64ebf6e656094ba3d2230962a83cb29f` to exact handoff input `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`. Tracked files were clean before update; existing local `.codex`, `.venv-windows-validation`, aria2/gallery-dl, logs, manual-validation, build and validation-artifact directories were preserved.
- Scope: Windows revalidation of the accepted WebSocket stream blocking-mode fix and stage counters, plus Extension parsed-host URL validation. No Windows-owned production implementation change was required. The shared fix remains Linux-owned; it was not modified here.
- PASS: Windows-target regression `cargo test -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-batch-6d60429/target websocket_transport::tests::authenticates_when_the_accepted_stream_starts_non_blocking -- --exact --nocapture` (1/1; 111 filtered). This exercises a non-blocking accepted stream and delayed authentication frame on the Windows target.
- PASS: `npm test --workspace extension` (32/32), including exact parsed-host allowlist, credentials/ports/schemes, query/fragment and relative-link cases. `npm run check --workspace desktop` built 52 Vite modules; `cargo fmt --all -- --check` passed.
- PASS: `npm run build:tauri --workspace desktop -- --no-bundle --ci` built a fresh Windows release executable with isolated target `desktop/src-tauri/validation-artifacts/windows-batch-6d60429/target/release/xarchive-desktop.exe`; size 19,217,408 bytes; SHA-256 `BB597610902C23A3481E2FEAE4351170BA7B9C882ECAA65915864E284246FBDC`. The non-fatal MSVC linker messages reported import-library creation; exit status was 0.
- PASS: Full portable package assembled at `validation-artifacts/windows-batch-6d60429/full-package` using the fresh Desktop executable plus preserved local Native Host and worker/gallery-dl artifacts. Manifest type/platform are `full` / `windows-x64`; Extension ID and Native Host allowed origin agree (`iaajefkoanbkleojofoadeakelihbjne`). Static package contained 76 files, including `_internal/python312.dll` and bundled `gallery-dl.exe`.
- PASS: Extension ZIP plan/build/expand/verify at `validation-artifacts/windows-batch-6d60429/`: 12/12 files match plan `XArchive-v0.0.0-pre.2-extension.zip`; SHA-256 `244F30E27CF797CE02D4A384A94A3A4A1FFD9253ECC89803B5A385AE571644B9`.
- PASS: Bandizip 8.0 Beta created and tested `XArchive-v0.0.0-win-batch-6d60429-windows-x64-full.7z`; `bz t` returned `All OK`. Archive SHA-256 `F81C5777F17E98B084CA70E26DB95C32016CC28CAED4B17C3496539E79870CF1`. Extracted inventory matched 76/76 files, 0 missing, 0 extra, 0 SHA-256 differences. Both package names/tags are local validation metadata; nothing was signed or published.
- BLOCKED: Current exact-revision WQ-WS-01/02/03/04 browser load, authenticated pairing, request/response, reconnect/pending cleanup, and visual/keyboard checks. Computer Use was retried; both inventories returned no native app targets and only exposed the existing Edge profile. That profile was left untouched. Blocker: `COMPUTER_USE_UNAVAILABLE`; continue in a controlled/disposable Edge/Chrome profile using the package above. Prior `fcde594` user-reported pairing failure remains historical and is not reclassified as a current failure or pass.
- NOT RUN: full Desktop crate regression, full workspace regression, browser/GUI launch, real account/task/download, Native Host registration, code signing, and release publication. The diff was narrow; the new Windows socket-mode regression, affected Extension suite, frontend build, Rust formatting and fresh package build/inventory were selected. Full regression was not run for this change set.
- Cross-platform follow-up: `CROSS_PLATFORM_CHANGE_REQUIRED` none newly identified; `CROSS_PLATFORM_REVIEW_REQUIRED` none. Keep Windows ownership because only manual Windows GUI work remains and no cross-platform follow-up was found.

### 2026-09-27 user-reported Sidecar launch failure and corrected package probe

- Validation input: Windows Desktop source at `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`; this revision has no Sidecar source changes relative to the previously built current-source worker input `fcde5943af2f6ad15c833fa5ab88d6b1758345c6`.
- The user screenshot from `validation-artifacts/windows-batch-6d60429/full-package` shows first-run download-folder setup still pending, Sidecar stopped, Extension disconnected, and `sidecar is not running` after a Sidecar action.
- `desktop/src-tauri/src/config.rs` adds `--timeout-seconds 300 --discovery-timeout-seconds 600` to the worker launch arguments. The worker in that package (SHA-256 `96C19695AC46E30AA23AF184ED41D0C8339FE4C98A2D771CF0781906402A60C2`, same hash as preserved `sidecar/dist` artifact) omits both options from `--help`. Replaying those arguments exits `2` with `unrecognized arguments`, before the protocol-v2 `hello -> ready` handshake. The package's earlier static inventory/archive PASS remains valid for file presence/integrity, but its Sidecar runtime-compatibility result is now `FAIL`.
- Corrected isolated package assembled with `desktop/scripts/build-portable-windows.mjs` at `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`, using the same fresh Desktop executable and preserved Native Host/Extension, with the current-source worker from `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix`. The corrected worker accepts both timeout arguments; its package SHA-256 is `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`.
- PASS: exact-argument worker CLI and JSONL v2 probe against the corrected package, including `hello -> ready`, required `account_discovery` capability, `shutdown`, and exit code `0`. This verifies the worker/package handshake only; it does not prove the Desktop GUI starts the Sidecar.
- BLOCKED: corrected-package Dashboard Sidecar start and Extension connection were not re-run. The old package Desktop process (PID 29792) remains open at the stale package path; Computer Use reports no native app target. Do not replace its files or kill it. The new package's first-run download-folder choice requires the user. See WQ-WS-06 in `../validation/windows-queue.md`.
- Edge Computer Use reports an attached Edge extension instance but does not expose the profile name and currently lists multiple tabs, conflicting with the user's report that only the Codex profile tab remains. No tab was opened or navigated. Profile identity and XArchive Extension connection remain unverified; see WQ-WS-07.
- No shared contract/protocol failure is indicated: the cause is a stale local frozen package artifact, while a worker built from unchanged current Sidecar source accepts the current Desktop arguments. `CROSS_PLATFORM_CHANGE_REQUIRED`: none. `CROSS_PLATFORM_REVIEW_REQUIRED`: none.

### 2026-09-27 user retest: corrected package starts; Extension authentication rejected

- User-provided PowerShell process inventory identifies `xarchive-desktop.exe` PID `66608` and `xarchive-downloader.exe` PID `55608` under `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`.
- User Dashboard screenshot: SQLite connected; Sidecar running with `hello -> ready`; Extension remains disconnected. This closes WQ-WS-06 as `PASS` for corrected-package GUI startup/Sidecar state.
- User Extension Options screenshot: WebSocket enabled at port `17321`, settings saved/connecting, current status `认证失败`; Native Messaging fallback is shown.
- User Extension diagnostics screenshot: Extension files ready and Native Host registered; Desktop had not observed an authenticated browser connection. The counters show one accepted connection, one auth received, one auth failure, no handshake failure, and one close before authentication (point-in-time single-attempt evidence).
- User DevTools screenshot shows an outbound protocol-v1 `authenticate` frame and an inbound `authentication_response` with `authenticated: false` and `error_code: AUTHENTICATION_FAILED`. This proves the request reached Desktop and was rejected at authentication; HTTP 101 alone is not the basis for this result. Authentication/request routing is `FAIL` for this attempt.
- Likely explanation (inference): the Extension's stored token does not match the current Desktop process token, which can change after Desktop restart. This has not been proven from the masked screenshots. Next step: use the current running Desktop's Extension settings copy action, replace the stored Extension token, save/reconnect once, and verify `authenticated: true`; never expose the token.
- The user's standalone worker probe exited `2` because `Start-Process -ArgumentList` split the `--gallery-dl` path at spaces. The error names the path remainder `Workspace\Tw2Tg\...`; this is a PowerShell argument-quoting issue, not a worker runtime failure. A local rerun passed after using one quoted argument string (`--timeout-seconds 300 --discovery-timeout-seconds 600 --gallery-dl "<full path>"`); the Dashboard and process screenshots independently confirm the corrected worker is running.
- Profile label is not visible in the supplied screenshots, so the exact Edge Profile identity remains unconfirmed. Do not record account/page contents or token values.
- Current results: stale package Sidecar start `FAIL`; corrected package Sidecar GUI startup `PASS`; corrected worker probe from prior run `PASS`; current Extension authentication `FAIL`; complete connected/request/response lifecycle `NOT RUN`; full regression/release/signing `NOT RUN`. No new shared contract issue identified; `CROSS_PLATFORM_CHANGE_REQUIRED`: none; `CROSS_PLATFORM_REVIEW_REQUIRED`: none.

### 2026-09-27 Windows job-ID path failure and fix

- Input revision: `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`; Windows implementation and automated validation revision: `7eacb82`.
- User-provided evidence: after reconnect, the Extension Options page reported authenticated and two tasks appeared in Desktop. Both failed with `EXECUTOR_WORKER_FAILED` / `ARCHIVE_DOWNLOAD_FAILED`, Windows `os error 123` (invalid filename, directory name, or volume label). The generated archive job ID contains an ISO timestamp with a colon. This establishes authentication and task submission for that attempt, but archive execution failed.
- Implementation: `FileStore` maps opaque job IDs to Windows-safe staging directory components, escaping Windows-invalid characters, percent, trailing dots/spaces, and reserved device names. Creation, recovery lookup, and commit share the same mapping. The persisted job ID, database value, and protocol contract remain unchanged. This is a small shared implementation adjustment and is marked `CROSS_PLATFORM_REVIEW_REQUIRED`; no contract change was made.
- PASS: `cargo fmt --all -- --check`.
- PASS: `cargo test -p xarchive-storage --offline --target x86_64-pc-windows-msvc --target-dir validation-artifacts/windows-batch-os123-storage/target` (37/37; includes ISO-timestamp staging, recovery lookup, and commit regression).
- PASS: `npm run build:tauri --workspace desktop -- --no-bundle --ci` with isolated `CARGO_TARGET_DIR=validation-artifacts/windows-batch-os123/desktop-target`; fresh executable built at `validation-artifacts/windows-batch-os123/desktop-target/release/xarchive-desktop.exe`.
- PASS: Full package assembled at `validation-artifacts/windows-batch-os123/full-package` using that executable and the previously validated compatible worker/Native Host/Extension inputs. Required components and `windows-x64` package manifest are present. Package Desktop SHA-256: `2BA8861D56FC10C2EDC20B9B223641D56673F2471EA009F23090313F2FDC5039`; worker SHA-256: `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`.
- BLOCKED: post-fix GUI archive revalidation. Two Computer Use inventory attempts found no native app targets; Edge exposed multiple tabs and did not identify the intended Profile. No tab was touched. Manual steps are WQ-WS-08.
- Result classification: pre-fix archive execution `FAIL`; Windows storage regression and build/package `PASS`; post-fix GUI archive execution `BLOCKED — COMPUTER_USE_UNAVAILABLE`; unrelated complete regression, signing, and publication `NOT RUN`.
- Cross-platform follow-up: `CROSS_PLATFORM_CHANGE_REQUIRED` none; `CROSS_PLATFORM_REVIEW_REQUIRED` review the small shared FileStore mapping. Next owner: Linux Cross-platform Owner for review, then Windows for WQ-WS-08.

### 2026-09-27 user retest: executor runner saturation

- Validation input: current formal branch `0d3735a1ae8d2f3636acff489a933a1f3bc96e5b`; user tested the isolated Full package built from implementation `7eacb82` at `validation-artifacts/windows-batch-os123/full-package`.
- User screenshot/report: Sidecar and Extension connected. Four tasks appeared; two initially showed “Downloading” for more than 30 seconds, while later submissions failed with `EXECUTOR_WORKER_FAILED: job executor command queue is full`. The earlier `os error 123` did not recur; this is limited PASS evidence for the Windows staging path mapping and task submission, not archive completion.
- Read-only runtime evidence from the package: one Desktop process, one Dashboard Sidecar worker, one archive worker with gallery-dl child processes; SQLite snapshot at inspection time showed one `DOWNLOADING` job and three `FAILED` jobs. The package logs contain runtime initialization records only. No process was stopped and no task was cancelled.
- Source diagnosis: `JobExecutor::with_capacity_and_factory` creates a 32-entry command channel and a separate one-entry runner channel. `run_worker` forwards `RunJob` to the runner with `try_send`. `execute_persisted_from_factory` persists `DOWNLOADING` before calling `run_job`; a full runner channel returns `ExecutorError::QueueFull`, which is then persisted as `EXECUTOR_WORKER_FAILED`. Thus one job may be executing and one waiting while later concurrently scheduled jobs fail. The displayed “command queue” wording conflates the runner queue with the command channel.
- Timing limit: current package config sets extraction timeout to 300 seconds and transfer timeout to 1800 seconds. A 30-second wait does not prove an individual job is hung; the queue rejection itself is directly explained by the shared bounded runner behavior.
- Classification: current-token pairing/task submission `PASS` (user-reported); Windows path mapping `PASS` (limited, no `os error 123`); rapid scheduling behavior `FAIL` (user-reported and source-confirmed); full archive completion `NOT RUN`/unverified; post-fix burst GUI check `NOT RUN` until Linux returns an exact revision.
- Required follow-up: `CROSS_PLATFORM_CHANGE_REQUIRED` — Linux to define and implement durable scheduling/backpressure so accepted submissions remain queued instead of being failed on transient runner saturation. Preserve an accurate queued state until a runner slot is available, or otherwise return a defined retryable response without marking the durable Job failed. Add a deterministic regression test with at least three overlapping jobs. Windows revalidation is WQ-WS-09 after the exact revision is handed back.
- Ownership: Linux Cross-platform Owner; also review the small FileStore Windows-safe staging mapping from `7eacb82` (`CROSS_PLATFORM_REVIEW_REQUIRED`).

### 2026-09-27 user retest: extraction timeouts and final job states

- Validation input: current formal branch revision `57333ad3fa70702bbe9f889d128e5f230cf4a10e`; user continued testing the corrected Full package at `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`.
- User screenshot: all four submitted jobs reached `FAILED`. Two report `EXECUTOR_WORKER_FAILED: job execution error [ARCHIVE_DOWNLOAD_FAILED]: DOWNLOAD_TIMEOUT: gallery-dl timed out`; two report `EXECUTOR_WORKER_FAILED: job executor command queue is full`. The screenshot also shows the Dashboard Sidecar running. The Extension currently appears disconnected in the Dashboard screenshot; no new authentication conclusion is drawn from it.
- Package configuration confirms `extraction_timeout_seconds: 300`, `discovery_timeout_seconds: 600`, and `transfer_timeout_seconds: 1800`. `ExtractionRunner` raises `DOWNLOAD_TIMEOUT` when its configured extraction deadline expires, then terminates the gallery-dl process tree. The earlier observation of more than 30 seconds was below this timeout; the later explicit timeout error establishes that the extraction deadline was eventually reached, but does not identify why gallery-dl failed to finish (network/service response, content/authentication requirements, or another runtime cause remain untested).
- Classification: Windows-safe staging ID mapping remains limited `PASS` (no `os error 123` is reported); the four-task outcome is `FAIL` (user-reported: two extraction timeouts and two shared runner queue rejections); root cause of the gallery-dl timeouts is `NOT DETERMINED`; real archive completion is `FAIL` for this attempt. No source change is justified by the timeout evidence alone.
- Required follow-up: retain `CROSS_PLATFORM_CHANGE_REQUIRED` for durable runner scheduling/backpressure and retest both queued-job progression and a controlled single archive after Linux returns the exact fix revision. WQ-WS-10 records the Windows extraction-timeout investigation. Compare an explicitly controlled public/synthetic URL or known-good fixture with the user-selected post, record only elapsed time, terminal code and output-file presence/hash, and do not capture account content or credentials.
- Ownership: Linux Cross-platform Owner for the already-confirmed executor queue correction and shared FileStore review; Windows Platform Owner resumes the queue and extraction retest on the exact returned package.

### 2026-09-27 user confirmation: X access and Extension controls

- User-provided screenshot/report: `https://x.com/home` loaded with page content, and the user confirms the Extension buttons/controls load. The capture shows the X Home page and visible Extension controls. No account/post identifiers were added to this record.
- Classification: `PASS` for X page access and Extension control rendering only. The screenshot does not identify the Edge Profile or exact package revision; it does not verify manifest permissions, service-worker health, authenticated Desktop connectivity, request/response, or archive execution. WQ-WS-01 is updated with this limited evidence; WQ-WS-02/03 and WQ-WS-09/10 remain open as previously classified.
- Ownership is unchanged: Linux Cross-platform Owner retains the shared executor queue fix and FileStore review. Windows resumes WQ-WS-09/10 after the exact shared-fix revision is returned.

### 2026-09-27 current Windows batch handoff check

- Fetch/status: `git fetch origin` completed; formal branch `feature/u7-desktop-production-integration` was at `d55ad583c969af4c2d3b4e801d0b767df51f8a1d`, equal to origin. The tracked worktree was clean. Existing user-local untracked dependencies, logs, manual files, and validation artifacts were preserved.
- Plan: no separate Plan markdown file was present in the repository or `.codex`; the current plan was taken from `docs/status/platform-handoff.md` and the active WQ-WS-09/10 items. No production-code diff exists since Windows implementation `7eacb82`; the intervening commits update validation/handoff documentation, so code-dependent historical PASS results remain reusable.
- Current browser observation: Computer Use exposed Edge browser tabs but no native app targets (`apps: []` on two inventory observations); the browser backend did not expose a profile name and showed multiple tabs. The existing X Home tab loaded successfully and its accessibility tree exposed injected `XArchive：保存` buttons on timeline items. No buttons were clicked and no task was submitted.
- Classification: `PASS` for current X Home access and Extension content-script button rendering. `BLOCKED` (`COMPUTER_USE_UNAVAILABLE`) for native Desktop window inspection and exact Edge Profile identification. Full manifest/permission and Service Worker checks remain `NOT RUN`; previous build, package, worker-protocol, and storage tests are reused because there is no new production-code diff.
- Revision scope: repository validation input `d55ad583c969af4c2d3b4e801d0b767df51f8a1d`; observed browser artifact source/package revision is not exposed. `git diff --check` over the changed documentation passed. No implementation or full regression was needed in this doc-only/current-GUI batch.
- Follow-up and ownership: shared executor queue/backpressure remains `CROSS_PLATFORM_CHANGE_REQUIRED`; the small shared Windows-safe FileStore mapping remains `CROSS_PLATFORM_REVIEW_REQUIRED`. Linux Cross-platform Owner receives the batch next; Windows resumes WQ-WS-09/10 when the exact shared-fix revision is handed back.

### 2026-09-27 repeated handoff fetch and validation-scope check

- `git fetch origin` completed. On branch `feature/u7-desktop-production-integration`, Windows `HEAD` and `origin/feature/u7-desktop-production-integration` both resolve to `f03a9963db9b92bdfef6fd4d2570634c7fe43392`; tracked working tree is clean. Existing untracked local dependencies, logs, manual files, and validation artifacts remain untouched.
- No separate current Plan markdown was found in the repository or `.codex`; current work remains defined by this handoff and WQ-WS-01 through WQ-WS-10. The diff from Windows implementation `7eacb82` contains documentation only.
- `git diff --check 7eacb82..HEAD` passed. No Windows-owned production failure or new Windows implementation need was found in the current diff. Reuse Windows storage tests 37/37, `cargo fmt --all -- --check`, Tauri release build, and corrected Full package assembly PASS from `7eacb82`; no unrelated tests were rerun.
- Reuse the immediately preceding read-only GUI evidence: X Home loaded and injected `XArchive：保存` buttons were visible. Native app inventory was empty twice, Profile identity and the exact loaded Extension revision remain unverified, and existing archive failures remain open. No task was resubmitted.
- Cross-platform routing is unchanged: queue/backpressure fix is `CROSS_PLATFORM_CHANGE_REQUIRED`, and the small FileStore mapping awaits `CROSS_PLATFORM_REVIEW_REQUIRED`. Current next owner remains Linux Cross-platform Owner; Windows resumes WQ-WS-09/10 after the exact returned revision.

### 2026-09-30 accepted Windows run and SHA-256 inventory (credential-blocked checkpoint)

Workflow tooling SHA for accepted runs: `e6d71ad44f8ef8cd8539259896787426e389ad67`. All rows below are actual accepted Windows evidence; no GUI/installation result is implied.

| Phase / target | Run ID | Source SHA | Asset | Bytes | SHA-256 |
|---|---|---|---|---:|---|
| dry / `v0.1.1-pre1` | [36672977464](https://github.com/15699122/Tw2Tg/actions/runs/36672977464) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.7z` | 4125276 | `6614c6b5551a248c46b0bb1bdf0d84c18b7a5b7d130f1caa755e70d53bd7744a` |
| dry / `v0.1.1-pre1` | [36672977464](https://github.com/15699122/Tw2Tg/actions/runs/36672977464) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.exe` | 17477632 | `8e8277555a945f19d4dbfca84a1ec772a62b28f931320a6a5307b1bbbb8f2b83` |
| dry / `v0.1.1-pre2` | [36673063515](https://github.com/15699122/Tw2Tg/actions/runs/36673063515) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.7z` | 4132005 | `a91e1b199c770b839cf6e12cfaf8b5454c2e20096d45eee04d8d8fa6ee9f8de8` |
| dry / `v0.1.1-pre2` | [36673063515](https://github.com/15699122/Tw2Tg/actions/runs/36673063515) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.exe` | 17507328 | `f74b9d89338aadc0601ea42f41153e5e8da0cb61fd83bff247a6ea491df15d28` |
| dry / `v0.1.1-pre3` | [36673130141](https://github.com/15699122/Tw2Tg/actions/runs/36673130141) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.7z` | 4226964 | `01a291d638bcd8ab54000399ced56495707e7f903ad54f216982f05816d4f8a3` |
| dry / `v0.1.1-pre3` | [36673130141](https://github.com/15699122/Tw2Tg/actions/runs/36673130141) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.exe` | 18023936 | `b5c6fdcce79fa3cb10635d4f9eaea6c436a5d5af5834ae24ecfde64403b0e8ab` |
| dry / `v0.2.0-pre1` | [36673250103](https://github.com/15699122/Tw2Tg/actions/runs/36673250103) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.7z` | 4232181 | `a493e3c2dc44428b6141f49c2f16b400689a9aaa0bffc32118e74e095e365dec` |
| dry / `v0.2.0-pre1` | [36673250103](https://github.com/15699122/Tw2Tg/actions/runs/36673250103) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.exe` | 18082304 | `9ca2686e9c8697db8f6dd8ac47c60762c0178e2d08daa8d7e8c599a01eebb156` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64-full.7z` | 34253138 | `f772b8d3f9d28b8876a0bbbede59683b9a35fe95821cb7be3ca5a3348c25eb0a` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64-repository-dependencies.7z` | 5917420 | `476b04246c64cd3b820b783274ef7cfc2096dd88222bc78f37f56ff60ecdf902` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64.7z` | 4270811 | `883224127a98f5d708cf2622ee6f7ebbd6b475d0296f3f0c326db29b61aaa530` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64.exe` | 18249728 | `75d1c0dbf0d3dbbd8f136b8aefd219f228a973d1b615b0127c73a445ca70e7da` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64-full.7z` | 34257239 | `7381333dbd6c00b791541f390c533cc5940fdb8e391e73d89aa8c2da15b2973a` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64-repository-dependencies.7z` | 5916756 | `d6960333ead108133fdeedc4cff9bb7eda6458c1a00b8cbcec84f0806b086799` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64.7z` | 4268968 | `3b6c09911ce7072993e78ab4db649dc8d7a31d6684d05ec63d0eabc21ecfd6dd` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64.exe` | 18250240 | `a25e7132e546045096b9acb96ea1c3c9f2e25353733871a905dcb8eee1743125` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64-full.7z` | 34258100 | `a4008d6d69ce0b5e2bef69b66e06abd8b0d738783724c2e9f9108b9f4dbfbdaa` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64-repository-dependencies.7z` | 5918182 | `f62bf33115a6e3997125040341b588ddf1283ea2794b10a132a2862360373d30` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64.7z` | 4266292 | `b17ba34bbc21ff86d1bfc89fe4610f583af0e3ce99b6595e1f124274fb7475d8` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64.exe` | 18250240 | `bf42e1456caec9f31d963f9bbd301ad3e16ccf65fa6462efaf67f13ee5481b9e` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `SHA256SUMS-v0.2.0-pre5.txt` | 538 | `8d9ed3d1e9006c04a7dd09d913905063c84765ec40e0be54c7c67aae06dd4087` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-extension.7z` | 5280 | `7641105d80e61530c5a2c27c818c3f7404b0051ac44dfc80465e147f5016ebae` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-release-manifest.json` | 1350 | `d79fe0e401795b743340b49040e8003a00197772dcec42d840d06c1a93de645a` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64-full.7z` | 34319495 | `8c473b51d73ae11c4b73be94ffa297e9e4903bde7fdfe048dd15c377ed5aa75c` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64-repository-dependencies.7z` | 6053977 | `32af3d952a05ca0c68421da8d704d3a296caf530d0199c6674e2f447d738fdcf` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64.7z` | 4406570 | `6b73ba6aaa1f233b81a6374e3be084195d9703dd965c79dd58ddf50d6a133bc3` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64.exe` | 18258944 | `4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `SHA256SUMS-v0.2.0-pre6.txt` | 538 | `c05404bf2e03c7bb09d0524b9619bf899457df2d699116bb8c7ca416dd349880` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-extension.7z` | 5606 | `c616163037f3f9056a431cb0c51b3d5d12b170eaf24f56d43d203b27b0630341` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-release-manifest.json` | 1350 | `f73fb308f03daf7ee714cca707bb7a747e70003ec271c6cc4ac7b474733a5a08` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64-full.7z` | 34322488 | `cc00274dea6c00cfe2f8980e792ea5d88131bc02a9606bca454dfde9c043632a` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64-repository-dependencies.7z` | 6055960 | `de862ead603044360cddaa0132c24163e8cb9e6d4a72ae180f6eb1e2ab262714` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64.7z` | 4270916 | `7cbc14a9dc20768360f5623e7acbbc91aa0175e7f57a203aecef9c647e8c8e00` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64.exe` | 18258944 | `9ab7a616acfaf6ab88ecd5db055772fc42c7d27bd64b69e3e3e64c1302e893f4` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `SHA256SUMS-v0.2.0-pre7.txt` | 538 | `c0a6294ace0d3693c14d9638d1e8d69d19df91ccede6acb193984968dda2cf6e` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-extension.7z` | 12380 | `87cf3e8eb78358e53cc8b74fa76853d8a53d8d92cf51d0ba50c14feb18b941f5` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-release-manifest.json` | 1351 | `8968ea9a031fde6e5dafb74bd9f5f5abd0cf2de574776fd2dee00be90cf444a2` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64-full.7z` | 34535088 | `a4659a6c56c00796c871c7a9b41dccefb40c286e5dfadba489639cabfbaf312a` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64-repository-dependencies.7z` | 6078140 | `ade857fa5bbae091270b42dc7165dcd9a19cfc94a0b3826d8c13cd092317026d` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64.7z` | 4471230 | `64b2440cba04f28e5afe5d33ab614614db0684e134248bf7c43c82fc10551e90` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64.exe` | 19264512 | `615b60c16b38bf851c1efc65e28e8bb4bf757fc77af07d5f92b207d56f01121d` |
| draft / `v0.1.1-pre1` | [36674300106](https://github.com/15699122/Tw2Tg/actions/runs/36674300106) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.7z` | 4125276 | `6614c6b5551a248c46b0bb1bdf0d84c18b7a5b7d130f1caa755e70d53bd7744a` |
| draft / `v0.1.1-pre1` | [36674300106](https://github.com/15699122/Tw2Tg/actions/runs/36674300106) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.exe` | 17477632 | `8e8277555a945f19d4dbfca84a1ec772a62b28f931320a6a5307b1bbbb8f2b83` |
| draft / `v0.1.1-pre2` | [36674393927](https://github.com/15699122/Tw2Tg/actions/runs/36674393927) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.7z` | 4132005 | `a91e1b199c770b839cf6e12cfaf8b5454c2e20096d45eee04d8d8fa6ee9f8de8` |
| draft / `v0.1.1-pre2` | [36674393927](https://github.com/15699122/Tw2Tg/actions/runs/36674393927) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.exe` | 17507328 | `f74b9d89338aadc0601ea42f41153e5e8da0cb61fd83bff247a6ea491df15d28` |
| draft / `v0.1.1-pre3` | [36674492512](https://github.com/15699122/Tw2Tg/actions/runs/36674492512) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.7z` | 4226964 | `01a291d638bcd8ab54000399ced56495707e7f903ad54f216982f05816d4f8a3` |
| draft / `v0.1.1-pre3` | [36674492512](https://github.com/15699122/Tw2Tg/actions/runs/36674492512) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.exe` | 18023936 | `b5c6fdcce79fa3cb10635d4f9eaea6c436a5d5af5834ae24ecfde64403b0e8ab` |
| draft / `v0.2.0-pre1` | [36674586528](https://github.com/15699122/Tw2Tg/actions/runs/36674586528) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.7z` | 4232181 | `a493e3c2dc44428b6141f49c2f16b400689a9aaa0bffc32118e74e095e365dec` |
| draft / `v0.2.0-pre1` | [36674586528](https://github.com/15699122/Tw2Tg/actions/runs/36674586528) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.exe` | 18082304 | `9ca2686e9c8697db8f6dd8ac47c60762c0178e2d08daa8d7e8c599a01eebb156` |

Evidence directories: `validation-artifacts/migration-20260930-7a3374b/<phase>/<target>/`; each contains run logs/status, downloaded `migration-plan.json` and `migration-verification.json`; accepted drafts also contain downloaded assets. Artifact names are `pre-release-migration-<target>-<run_id>`.

## 2026-10-01 Windows validation — 01c40db

- Git: fetched origin main/dev; selected exact requested source `01c40db7324f71f0971f3da4ef88ef4b180869d7` on `codex/windows-validation-01c40db` through Git, tracked tree initially clean. Existing ignored/untracked caches preserved; no filesystem synchronization.
- Scope: current density/icon Plan and preceding logs default-filter fix; Windows native PowerShell, Node 24.19.0, npm 11.19.0 through npx, Rust 1.98, Python 3.12.14. Computer Use @oai/sky available, actual packaged WebView2 window inspected at 1082x790; system DPI not certified by this run.
- Build PASS: fresh optimized Desktop/Vite and Full assembly, XARCHIVE_RELEASE_TAG / PORTABLE_RELEASE_TAG=v0.2.1-pre1; PORTABLE_OUTPUT_DIR=dist-portable/windows-01c40db-20261001-full; binary reuse fallback not enabled. Worker supplied from windows-6be3269-20261001/worker-dist/xarchive-downloader, Native Host from unchanged previous build: reused components, not newly compiled components. Version files remain 0.2.0 by plan.
- Desktop EXE SHA256 `08CC43B5BDBD677898EFC789ADDE1C1820379535F5C65C7B6F5FA83CE4DA2DEC` (19521536 bytes). Source identity, rather than displayed version, binds evidence.
- Desktop tests: first FAIL 186/188, icon render function extraction used LF-only blank-line regex on Windows CRLF; outer test also failed for the same cause. Test-only fix accepts CRLF and LF, assertions unchanged. Rerun PASS 188/188, exit 0. Implementation revision `28543edc10c432415794943736644f7c9d712f1e`, CROSS_PLATFORM_REVIEW_REQUIRED. No product implementation defect inferred.
- GUI subchecks: compact sidebar and distinct dashboard icon PASS at current viewport; empty/stopped settings-link gap PASS; Core Bootstrap missing top divider and retained next divider PASS; actual settings accessibility order Core Bootstrap / Sidecar / aria2 / Extension / Storage / Logging / Network Proxy PASS. Title-bar green archive-box icon observed from fresh path. Full DPI, keyboard, populated/running and all native icon surfaces NOT_RUN.
- Logs focused revalidation PASS: default select Debug, startup channel=prerelease/channel_default=debug/effective_level=debug/user_override=none, debug diagnostics visible. Manually selected Info via keyboard after pointer targeting error; remained Info on later polling. Prior Info default failure is resolved for this subcheck only; complete real module diagnostic coverage remains NOT_RUN.
- Reuse: unchanged Rust229/Python8 and prior stable channel backend subchecks from 6be3269; no new Rust/Python/WDIO/full regression. Existing Native Host installation FAIL not closed; Telegram paused; System Proxy Batch B not delivered.
- Evidence (ignored): validation-artifacts/windows-01c40db-20261001/build-full.log, desktop-tests.log, desktop-tests-r2.log, exe-hash.json, logs-default-debug.txt, logs-manual-info.txt, settings-headings.txt, cleanup.json. GUI screenshots observed in session; no token-bearing Settings tree or screenshot stored. Cleanup PASS: app/worker/driver processes and known listen ports empty.
- Manual queue: current windows-queue latest section. Results do not close release acceptance. Validation revision: Git commit containing this record. Next Owner Cross-platform for shared test review/reconcile, no WINDOWS_VERIFICATION_BLOCKING.

## 2026-10-01 Windows runtime icon follow-up

User screenshots show clear Explorer/properties icon but blurred running taskbar icon. Repository has no notification-area tray creation; screenshot is taskbar. Root cause confirmed from installed tauri-codegen2.6.3 src/context.rs and src/image.rs: Windows selects ICO, CachedIcon::new_ico decodes entries()[0]. Current ICO starts16x16; Explorer instead selects size-specific PE resources. Thus the runtime loses the remaining ICO resolutions. Windows-owned fix in lib.rs overrides Context default_window_icon with compile-time embedded256x256 PNG under cfg(windows), retaining multi-size EXE ICO and non-Windows behavior. No dependency/API/contract change.

Fresh optimized pre-release Full build PASS, exit0, desktop19784704bytes; directory dist-portable/windows-runtime-icon-20261001-full. Worker/Native Host reused unchanged as in preceding batch. Initial sandbox npm-cache EPERM logged separately, permitted build retry succeeded. Evidence runtime-icon-build.log/runtime-icon-build-r2.log/runtime-icon-hash.json under validation-artifacts/windows-01c40db-20261001. Runtime taskbar/Alt+Tab/titlebar multi-DPI acceptance NOT_RUN; build is not visual acceptance. Use fresh executable path/new shortcut without clearing global icon cache, compare taskbar at100/125/150/200% with Explorer and ensure small titlebar remains legible. No new tests added/run for this follow-up.

## 2026-10-01 Owner manual confirmation — runtime taskbar icon

Result: PASS for the reported runtime taskbar blur defect. The human Owner explicitly confirmed the fix and supplied codex-clipboard-0581c50a-3237-4f67-b6cd-14796c7c44da.png in this conversation; the cropped screenshot shows a sharp green archive-box taskbar icon. Evidence provenance: Owner manual observation, not agent automation. Implementation: 9cc9d5d5c6f62b9e87f6357a8acf913ddf450e55, Windows Context override to256x256 PNG; Full at dist-portable/windows-runtime-icon-20261001-full, EXE SHA256 B725122CC9B8E67DA57C7064A3F2536F43F22B6B633F72F746034EA711C76144. The screenshot alone does not expose path/hash or DPI; artifact association follows the preceding repair-package handoff and Owner response. Original reported blur defect closed. Full100/125/150/200% DPI and Alt+Tab/tray matrix not established by this cropped screenshot and remains NOT_RUN where applicable (notification-area tray not implemented). Prior NOT_RUN records are retained as historical facts.

## 2026-10-02 Windows Telegram shared-layer validation — 1f14cea

### Identity, capability and selected scope

Source/input/implementation/tested SHA: `1f14cea6859dc1c0ecec164509579cfe4eb15f1a`, fetched explicitly from `origin/dev`; execution branch `codex/windows-validation-1f14cea`. Default remote fetch refspec did not include dev. Initial checkout was `codex/windows-validation-01c40db` at `40b637b`, with no tracked modifications; it is preserved. Git switch aligned the formal NTFS workspace, not a filesystem sync. Validation documentation/handoff revision is the commit containing this record, pushed to the execution branch. No product/test code changed.

Capability detected: native Windows build `10.0.29671`, AMD64, PowerShell 7.6.6; restricted filesystem sandbox (Git metadata/network-cache operations used approved escalation), network available via native execution. Node 24.19.0, Cargo 1.98.0, rustc 1.98.0, toolchain `1.98.0-x86_64-pc-windows-msvc`, native Python 3.12.14 at `.venv-windows-validation/Scripts/python.exe`. Working directory for all commands: `E:\Shiraishi\VSCode Workspace\Tw2Tg`. GUI/browser/driver/display scaling not exercised: this diff delivers backend modules and the Telegram GUI entry point is absent. There is no new COMPUTER_USE_UNAVAILABLE claim. No external Telegram service/account tested.

Impact baseline: released source `3115c3b50716be0155804ad4f94dd9d29e37d617` through input SHA changes Telegram transport/outbox/cache/plans, storage migration 0007, Desktop config/clock/send core and related dependencies/docs. Selected Module/Subsystem scope: both Telegram and storage suites, plus Desktop lib as their direct consumer, including config, clock, mock sending and native compilation. Full workspace regression, unrelated frontend/Python/packaging and old release GUI matrix NOT_RUN: this is not a release, and the affected subsystem/migration tests cover the delivered changes; no executable runtime send path exists. Older acceptance results retain their original revisions and limits rather than being promoted to this commit.

### Results and evidence

Common fields for rows below: owner Windows Platform Owner; implementation IMPLEMENTED for tested shared modules; platform Windows x64; method automated (tests) / static (fmt/docs); priority P1 (docs P2); prerequisites native MSVC/Rust and locked dependency access, native Python for discovery fixtures; expected all selected tests pass without weakening assertions; blocks_development=no, blocks_release=no for this local validation batch. Revalidation requires unchanged relevant code, dependency/contract and environment; otherwise REVALIDATION_REQUIRED. Evidence root `validation-artifacts/windows-batch-1f14cea/` is local and intentionally untracked; results/hashes are committed here.

| ID | Target / reproducible command | Status | Evidence / result |
|---|---|---|---|
| WIN-TG-MODULE-01 | `cargo test --locked -p xarchive-telegram -p xarchive-storage --no-fail-fast` | PASS | `telegram-storage-native.log`, exit 0: Telegram 48/48 and storage 51/51; both doc-test sets 0 tests. Local mock transport/SQLite evidence, not Telegram E2E. |
| WIN-TG-DESKTOP-01 | Set `PYTHON` to the absolute native interpreter path, then `cargo test --locked -p xarchive-desktop --lib --no-fail-fast` | PASS after environment correction | `desktop-lib-tests-r2.log`, exit 0: 175/175. Includes 8 send-core tests, config/clock, migration consumers and Windows Named Pipe fixtures. First-run failure preserved below. This compiles a test executable, not a packaged GUI/release binary. |
| WIN-TG-FMT-01 | `cargo fmt --all -- --check` | PASS | `fmt.log`, exit 0. |
| WIN-TG-DOCS-01 | `node scripts/docs-audit.mjs`; `git diff --check` | PASS | initial/final docs audit logs, exit 0; final diff check exit 0. |
| WIN-TG-UNIX-01 | Three storage tests gated by `#[cfg(unix)]` | NOT_APPLICABLE | `file_store.rs` intermediate/final symlink tests and `lib.rs` sidecar symlink escape test; Linux 54 vs Windows 51. Windows junction/real-filesystem acceptance is a different existing queue item, not proven by exclusion. |
| `WQ-TG-001`–`009`, `WQ-TG-UNI-01`–`08` | Native credentials, real send/recovery/large files, GUI and Unigram acceptance | NOT_RUN | `IMPLEMENTATION_NOT_READY`: send core exists, but production enqueue/claim loop, Tauri commands, settings/task UI and Windows Credential Manager adapter absent. No runnable acceptance artifact (artifact_sha256/build_origin=N/A). Steps/prerequisites/expected evidence remain in windows-manual-steps §K. Follow-up Cross-platform Owner runtime handoff, then Windows native/real-account work. |

Test artifacts produced by the commands above (SHA-256):

| Artifact under `target/debug/deps/` | SHA-256 |
|---|---|
| `xarchive_telegram-88beb19c2af1469b.exe` | `75D3E9DEC8F08E5DF7281B9171492774DF92833E56D66CFD90DA54D193D789BF` |
| `xarchive_storage-b8f9486e9e316f45.exe` | `EB39EA079B3078BB7C54B581C5E124A41863FBAAAB70AB634FA5D44824E04ECD` |
| `xarchive_desktop_lib-772e0490fa15adbe.exe` | `268C6FD128DB5CD445C05E3C2ECF418E1DF8387F9A998312EBA606381AEC3A61` |

For static checks artifact_sha256=N/A (source check). `artifact-hashes.json` stores absolute paths. `changed-files.txt` and `workspace-status.txt` preserve scope/local state. Local untracked `.codex`, Python environment, aria2/gallery-dl/sidecar components, desktop logs, dist-portable/manual-validation/validation-artifacts were retained and not staged.

### Failed initial paths, diagnosis and recovery

1. Initial sandbox `cargo test -p xarchive-telegram -p xarchive-storage --no-fail-fast` exit 101 before tests: Schannel `AcquireCredentialsHandle` / `SEC_E_NO_CREDENTIALS` fetching mime_guess. `telegram-storage-tests.log`. Locked offline attempt exit 101: mime_guess missing in cache (`telegram-storage-offline.log`). Equivalent approved native locked command downloaded dependencies and passed. These are tool/environment failures, not product results. Native compile also logged one incremental-cache os error 5 note and MSVC import-library stdout warnings; compilation and tests completed successfully. No production change made.
2. Initial native Desktop lib run exit 101, 172/175 (`desktop-lib-tests.log`): `batch::tests::{records_a_failed_discovery_on_the_batch,runs_discovery_and_persists_streamed_candidates,persists_each_discovery_candidate_before_completion}` reported sidecar not running. `batch.rs:1166–1168,1213` selects PYTHON or python3; Get-Command resolved python3 to WindowsApps, and independent `python3 --version` could not launch that alias. Explicit native Python 3.12.14 restored all 3 tests. Repeated command, unchanged source/artifact, 175/175 exit 0. Preserve both runs; default launcher configuration is unreliable, and future Windows test recipes must bind PYTHON. No shared defect/review request raised by this environment correction.

### Manual queue and handoff

All 17 Telegram/Unigram acceptance rows stay NOT_RUN; unimplemented portions PLANNED. The manual index had incorrectly called missing implementation BLOCKED and had an old baseline; both corrected to NOT_RUN/IMPLEMENTATION_NOT_READY at input SHA. The queue also incorrectly said no send core existed; it now distinguishes implemented core from absent runtime. Old release/icon/DPI/Native Host/transfer evidence and gates remain unchanged.

Next Owner Cross-platform Owner: reconcile this result and complete the already planned runtime enqueue/claim loop, commands/settings/task projection before a runnable Windows handoff. Windows retains Credential Manager, lifecycle/GUI, server deployment and controlled real-send/Unigram verification afterward. CROSS_PLATFORM_CHANGE_REQUIRED and CROSS_PLATFORM_REVIEW_REQUIRED: none new; no shared code/assertion edits. WINDOWS_VERIFICATION_BLOCKING: none. No publication, version bump or release acceptance.

## 2026-10-02 Owner manual DPI screenshots — v0.2.1-pre1

Method: manual (Owner changed scaling/captured images), followed by agent visual review; not GUI automation. Environment reported by Owner: 14-inch display, 2560x1600, Windows scaling 100/125/150/200%. Windows build/browser/runtime version not supplied for the capture environment; do not import the agent host metadata. Images 1/3/5/7 were resized in chat, so visual clarity conclusions are limited to the presented evidence, not pixel measurements.

Artifact path supplied by Owner and read-only checked: `E:\Shiraishi\Downloads\Compressed\XArchive-v0.2.1-pre1-windows-x64-full\xarchive-desktop.exe`; 19,812,864 bytes, FileVersion 0.2.1, SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Adjacent package-manifest records release_tag v0.2.1-pre1, platform windows-x64, package_type full. Build origin/source association: release record Run 36838400270 / `3115c3b50716be0155804ad4f94dd9d29e37d617`, with artifact association supplied by Owner and matching package metadata/version/size. Remote asset digest was not independently re-downloaded this round. This is a separate release-artifact result, not Windows GUI evidence for Telegram implementation 1f14cea.

Evidence is preserved locally in `validation-artifacts/manual-dpi-20261002/figure-1.png` through `figure-7.png`; screenshots.json contains original attachment paths and hashes. Screenshot files remain untracked; this committed record preserves their identities:

| Figure | Owner-reported scaling / window | SHA-256 |
|---|---|---|
| 1 | 200%, maximized | EDF5A23A579C604FAA0183D0C730AB3B6BA2FB307728FA06B22815F3D116E891 |
| 2 | 100%, windowed | D0D10EAC94E7FB151DC7660383FF89344F57970D35E95FD2C84F5B73E2843AC6 |
| 3 | 100%, maximized | A0C2B655DAA6E4FD2FB62D01941EAB93EF14651D7B4C8508F816AC77CBEF4FCA |
| 4 | 125%, windowed | B4B1C9DC6EF6AC3F6DA8A931AF418A78FA9F3382E60052AD3E1695126CBBF29A |
| 5 | 125%, maximized | AF461D0FDE2E1FB220325704A8EEC887F0BC7BA8EB5CDC629CE3B58B333C898F |
| 6 | 150%, windowed | E827CE39EB3C03BB3ED65FFFC793B28708B9DE40F005FF009D587ED96C9DA6F7 |
| 7 | 150%, maximized | 85118FDB9BF8850A32141603E1354AE5901B5C85012EA993ECEE5F41D0594002 |

Common acceptance fields: owner Windows Platform Owner; implementation IMPLEMENTED; priority P1; platform Windows x64; prerequisites identified Full artifact and reported display/scaling; expected legible icon and dashboard without overlap/clipping; blocks_development=no, blocks_release=no for these subchecks. Revalidate after icon/layout/dependency or artifact change. Documentation revision is the commit containing this record.

| ID / target | Result | Scope and remaining evidence |
|---|---|---|
| WQ-ICON-030-06 title-bar subcheck | PASS, screenshot-scoped | Green archive-box icon with distinguishable white mark visible in all 7 captures across reported 4 scales. Explorer, taskbar and Alt+Tab surfaces are absent: those current-artifact/multi-DPI subchecks remain NOT_RUN, defer reason missing surface evidence. No tray implemented. |
| WQ-UI-030-01 / 05 dashboard sidebar/glyph subchecks | PASS, screenshot-scoped | Service labels visible without apparent clipping/overlap; dashboard glyph distinct from sidecar waveform. Keyboard/focus and other pages not exercised. |
| WQ-UI-030-02 empty/stopped layout subcheck | PASS for visible gap/non-overlap only | Settings action follows start/stop row in all images; no stretched bottom alignment. Populated/running states not exercised. Button wrapping observation below is not closed by this limited PASS. |
| Settings/logs/keyboard/populated/running acceptance | NOT_RUN | Screenshots contain only the empty Dashboard with Sidecar stopped; prerequisites corresponding pages/actions/controlled fixtures still required. |

Visual observation for Cross-platform layout follow-up: figures 2/4/6 show start-button text split into two lines (Chinese start label above Sidecar), while stop remains one line; figures 1/3/5/7 show a single-line start label. Windowed captures are approximately the same logical width across scales, so this supports available-width sensitivity, not a scale-specific root cause. Current shared CSS uses a two-column dashboard with a 280px runtime-panel minimum, equal grow buttons and no dedicated label no-wrap rule (`desktop/src/style.css`, dashboard/control-panel/button-row; label in pages/dashboard-page.jsx). This is a plausible cause, not a reproduced/measured CSS diagnosis. The label is not visibly cut off or overlapping, and no single-line acceptance criterion was previously specified: record a layout review observation, not functional startup FAIL. Cross-platform Owner should review wrapping policy and, if adjusted, return the same window-width/scaling matrix for Windows revalidation. No code/assertions changed this round.

Next manual step: on the same artifact, capture Explorer file icon, actual running taskbar icon and Alt+Tab tile at the 4 scaling values, including the scale in evidence. Current screenshots do not complete WQ-ICON-030-06. No release gate or Telegram acceptance closes.

## 2026-10-02 Owner native-icon confirmation — 100/125/150%

WQ-ICON-030-06, priority P1, implementation IMPLEMENTED, owner Windows Platform Owner, method manual plus screenshot review. Same Owner-reported 14-inch 2560x1600 environment and artifact association as the preceding DPI record: `E:\Shiraishi\Downloads\Compressed\XArchive-v0.2.1-pre1-windows-x64-full\xarchive-desktop.exe`, FileVersion 0.2.1; read-only SHA-256 recheck unchanged at `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Build origin release Run 36838400270; source association `3115c3b50716be0155804ad4f94dd9d29e37d617` per release/package record, not 1f14cea. No new remote digest verification. No product/test changes; documentation revision is the commit containing this section.

Owner supplied three images (100/125/150%, in order) showing the running taskbar icon, and explicitly reported Explorer file icon and Alt+Tab icon match the main-window top-left icon. Taskbar PASS is supported by direct image review (legible green archive box/white mark, no plain blurred block); Explorer/Alt+Tab PASS is Owner manual confirmation, without screenshots of those surfaces. The confirmation is scoped to the three supplied scales; 200% is not inferred.

| Scale | Taskbar | Explorer | Alt+Tab |
|---|---|---|---|
| 100% | PASS, screenshot | PASS, Owner confirmation | PASS, Owner confirmation |
| 125% | PASS, screenshot | PASS, Owner confirmation | PASS, Owner confirmation |
| 150% | PASS, screenshot | PASS, Owner confirmation | PASS, Owner confirmation |
| 200% | NOT_RUN | NOT_RUN | NOT_RUN |

Local preserved evidence: `validation-artifacts/manual-icons-20261002/figure-1.png` SHA-256 `5EFD2D025F4160C693258287B204546EBDA4FFC730565684F731F5F895866778`; figure-2.png `790C54C3C7D1618C4A2A468E2EB733611F3269437046FE81E1486AD9CEFAB02B`; figure-3.png `8FFC1C469076CC6AA6E254C8C370555CBEE507AD82C70589015498AE136D98C1`. screenshots.json maps original attachment names/paths. Raw images remain local/untracked; identities and results committed here. Evidence for Explorer/Alt+Tab is the Owner's message accompanying these attachments.

Expected/prerequisites: same EXE, each reported scale, legible icon on each surface; no tray exists (NOT_APPLICABLE). 200% defer_reason=missing execution/confirmation evidence. Follow-up Windows/Owner: only remaining 200% taskbar/Explorer/Alt+Tab, no need to repeat completed scales. Prior four-scale title-bar PASS remains; full WQ-ICON-030-06 is not closed. Other keyboard/settings/logs/task-state checks and the windowed start-button wrapping observation remain unchanged. blocks_development=no; blocks_release=no for these subchecks; revalidate if icon/runtime/artifact/environment changes. No release approval.

## 2026-10-02 Owner 200% evidence reuse — icon matrix complete

The Owner clarified that the current development environment normally uses 200% scaling and instructed reuse of prior screenshots for the remaining icon checks. In the context of the outstanding 200% taskbar/Explorer/Alt+Tab request, this is recorded as **Owner manual confirmation** for those surfaces; it is not a new agent GUI run. Reuse the preceding 200% title-bar image (`validation-artifacts/manual-dpi-20261002/figure-1.png`, SHA-256 `EDF5A23A579C604FAA0183D0C730AB3B6BA2FB307728FA06B22815F3D116E891`) and the Owner's prior icon-consistency observation plus this explicit reuse confirmation. That image itself does not show taskbar/Explorer/Alt+Tab: those three PASS results rely on Owner confirmation, not on visual inspection of absent surfaces. Keep the earlier 100/125/150% image labels as supplied; do not relabel them 200%.

WQ-ICON-030-06: **PASS for the identified v0.2.1-pre1 Full artifact at 100/125/150/200%**, title bar/Explorer/taskbar/Alt+Tab. Notification-area tray NOT_APPLICABLE (not implemented). Priority P1, implementation IMPLEMENTED, owner Windows Platform Owner, method manual; expected clear green archive-box icon on each surface, no blurred green block. Source association `3115c3b50716be0155804ad4f94dd9d29e37d617`, build origin Run 36838400270, artifact identity from prior read-only checks and Owner association: EXE SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`, FileVersion 0.2.1, supplied Downloads Full path. Environment: Owner-reported 14-inch 2560x1600, default 200%, temporary scale tests as labelled. No new executable hash confirmed this turn: the previously supplied path was unavailable during the attempted read-only recheck. The record reuses the established artifact identity; no current file presence or different artifact is inferred.

This closes only the icon matrix at that artifact. Historical NOT_RUN records remain as earlier facts; UI start-label wrapping, settings/logs/keyboard/populated/running matrices remain open. Revalidate if icon/runtime/artifact or relevant environment changes. blocks_development=no, blocks_release=no for this subcheck; no Telegram or general release approval. Documentation revision is the commit containing this section. Next manual item: settings-page layout/order and keyboard traversal, beginning at the normal 200% scaling, with pairing token excluded/redacted in screenshots.

## 2026-10-02 Computer Use settings subchecks — v0.2.1-pre1

Owner Windows Platform Owner; method gui-automated via computer-use plugin 26.928.40906 / node_repl @oai/sky; priority P1; implementation IMPLEMENTED. Native Windows desktop capability confirmed live: helper initialized, list_apps/list_windows and target launch/capture/input succeeded. Exact target `E:\Shiraishi\Downloads\Compressed\XArchive-v0.2.1-pre1-windows-x64-full\xarchive-desktop.exe`; read-only SHA-256 restored/verified this turn as `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`, same established Full artifact, source association `3115c3b50716be0155804ad4f94dd9d29e37d617`, build origin release Run 36838400270. Desktop target selected uniquely from returned window id 3473714 and process path. Initial XArchive app absent from list_apps; launched through sky.launch_app. Scaling 200% is Owner-reported default, not independently measured this run. No system scaling setting changed.

1. Opened Settings through returned button 36; first immediate refresh still showed Dashboard, next observation confirmed Settings. Clicked reload (88) once; did not select archive directory, change settings, start Sidecar, register/unregister Host or trigger external route requests.
2. Captured actual top: Core Bootstrap had no preceding separator, following Sidecar separator visible. Actual accessibility headings: Core Bootstrap, Sidecar, aria2, Extension, Storage, Logging, Network Proxy (last).
3. Keyboard actual screenshot focus: reload -> start Sidecar, disabled stop skipped. At bottom, focused max-log-files without changing value; successive Tab actions visibly reached save logging -> proxy mode -> save proxy -> inspect route, each with visible focus ring. UIA focused_element inaccurately returned pane 8 rather than browser control; no full-page traversal PASS inferred from it.
4. Bottom actual rendering confirmed Logging before Network Proxy, no visible overlap at 1082x790 tool-reported window pixels. First attempted right-edge drag actually hit scrollbar and did not narrow; second boundary drag at x1081 narrowed to 761x790. Top Bootstrap/Sidecar and bottom Logging/Proxy at this narrow width had no visible overlap/clipping; responsive sidebar collapsed to icons. This is current-scale/current-width evidence, not every scale or intermediate section.

| Target | Result | Remaining scope |
|---|---|---|
| WQ-UI-030-03 | PASS top separator/current-scale and 761px narrow subcheck | Other scales/sections remain NOT_RUN. |
| WQ-UI-030-04 | PASS actual heading order + bottom Tab segment and focus + bottom narrow rendering | Full top-to-bottom traversal, intermediate Extension/aria2/storage focus and other scales NOT_RUN. |
| Complete settings keyboard/scaling matrix | NOT_RUN | Only bounded top/bottom segments exercised; full focus attribution unavailable in UIA and token-bearing middle captures excluded from saved evidence. Follow-up visual full traversal, preserving token privacy; not a COMPUTER_USE_UNAVAILABLE blanket blocker. |

Evidence: live tool captures/actions in this chat; safe saved `validation-artifacts/computer-use-settings-20261002/narrow-top.png` SHA-256 `A90F9264565C4093F3C89D535768B818452CEFB176912A8817D7D3DB2AD7A1DD`; `narrow-bottom.png` `DB11138E82F57308B5FCC029F5440F67EA7DE725E645CD7FB19D83925CF85D1D`; redacted settings-headings.txt and tab-observations.txt. Saved evidence local/untracked. Expected separators/order/focus and non-overlap defined in manual §L before execution. Revalidate after relevant CSS/JSX/runtime/artifact/environment changes; blocks_development=no, blocks_release=no for these subchecks.

Tool diagnostics: off-screen indexed input (max-log-files) rejected at y825 outside 790px window; refreshed and used current visible coordinate successfully. User input detected once, refreshed before continuation. Width-restoration drag rejected because destination x1081 was outside current 761px bounds; no further resize retries, app left open at narrow Settings for user review. These are tool limits, not product FAIL. No production code/assertion changes; Full regression not run for GUI-only validation of this artifact.

Sensitive-output incident: initial accessibility-label filtering missed the Extension token embedded in a button name, and two intermediate captures displayed its pairing area. No raw token value persisted in these documents or saved evidence. Current token should be rotated by Owner; previous rotation is historical and does not cover this token. Future captures must keep pairing area off-screen or fully redact before sharing. This run did not rotate or alter pairing security settings.

Next Owner stays Cross-platform for planned Telegram runtime work and windowed start-label layout review; Windows retains complete Settings/keyboard/scale and real-account/native validation. No new shared-contract finding or release approval. Documentation revision is the commit containing this record.

## 2026-10-02 Owner manual keyboard traversal confirmation

Owner reports keyboard traversal manually verified PASS. Scope: current v0.2.1-pre1 Full Settings page in the established normal 200% environment; no new assertion of 100/125/150% keyboard coverage. WQ-UI-030-04 full Settings keyboard traversal/current-scale subcheck supersedes the preceding NOT_RUN for that subcheck only. Method manual, owner Windows Platform Owner, priority P1, implementation IMPLEMENTED; expected visible focus and correct order through the full settings page; evidence Owner message in this conversation (no new screenshot/log supplied). Source association `3115c3b50716be0155804ad4f94dd9d29e37d617`, build origin Run 36838400270, previously verified EXE SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Artifact identity reused from preceding Computer Use run, not freshly checked this turn. blocks_development=no, blocks_release=no for this subcheck; revalidate after relevant layout/keyboard/runtime/artifact changes.

Keyboard traversal no longer a pending manual action at current scale. Other-scale Settings/Logs/Extension and populated/running Dashboard matrix remains NOT_RUN. Current token rotation is not confirmed by this message and remains pending. Other native/real-account/paths/recovery/proxy checks remain unchanged; use current protocol/production paths, not stale historical download/fallback recipes. No new functional tests for this documentation-only recording. Next priorities consolidated in windows-manual-steps §M; documentation revision is the commit containing this record.

## 2026-10-02 Computer Use follow-up: portable directory, logs and browser boundary

Revision: documentation branch `codex/windows-validation-1f14cea`, starting HEAD `63c44ca`; shared source baseline remains `1f14cea`. GUI target is the earlier release Full EXE, associated source `3115c3b50716be0155804ad4f94dd9d29e37d617`, not the Telegram baseline. Fresh EXE SHA-256: `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Native Windows Computer Use sky and connected Edge browser interface available. Scaling is Owner-reported 200%, not independently measured; capture dimensions are tool logical pixels (1082x790 normal, 761x790 narrow).

| Check | Result | Observed evidence / limit |
|---|---|---|
| Portable archive initialization | PASS (subcheck) | Owner authorized app-adjacent directory; clicked create portable directory, first-use prompt disappeared, settings showed package `download`, directory exists, same path after two normal restarts. No real archive was submitted. |
| Native folder picker cancel | PASS (subcheck) | Opened real folder chooser, cancelled with Escape, unchanged `download` path. Cached cancel element failed; fresh observation and keyboard recovery succeeded. Selection/path rejection/migration tests remain NOT_RUN. |
| Settings middle/narrow and logs layout | PASS (observed subchecks) | aria2 controls, Extension upper/lower controls, Storage and Logging viewed at 761px; no observed overlap. Logs normal/narrow checked/unchecked follow control works, checked green. Header buttons wrap at narrow width. Multi-scale matrix and new/live log-follow behavior remain NOT_RUN. |
| Error and Silent save/restart persistence | PASS (subcheck) | Saved each, visible saved feedback, each survives normal app restart; Error config `level: error` and rotated log header `level=error` read. Original Debug restored; config confirms `level: debug`, `max_files: 5`. Does not prove old-config upgrade, controlled child-process filtering or stable release default info. |
| Extension loaded | PASS (Owner manual subcheck) | Owner loaded extension and supplied Edge card showing XArchive 0.2.1 enabled. No automated extension-install claim. |
| Browser pairing/real archive | NOT_RUN / waiting for Owner | Desktop shows unauthenticated/not_loaded; Owner says popup/options open but not connected, screenshot pending. Loading alone does not prove Native Host, authentication or real archive. |
| Browser extension management automation | BLOCKED | Browser tool rejects `edge://extensions/`: URL policy permits only http/https and forbids workaround. Owner performs extension-page operations; do not bypass using alternate browser surfaces. |

Tooling observations are not product FAIL: one helper initialization binding error recovered; simultaneous Owner file-explorer work caused wrong-surface capture, agent paused and resumed only after Owner returned desktop control; folder-picker cache error recovered via Escape; WebView dropdown coordinate target rejection recovered using fresh observation and keyboard. One invalid observation option corrected without product changes. No protocol, production code, package or security settings changed.

Owner clarified current Extension token is one-time and auto-rotates; manual replacement requirement withdrawn, not recorded as an independently tested rotation PASS. No token values saved in committed records or retained evidence. Existing historical exposure records retained. Current Manual queue should no longer ask Owner to rotate it.

Evidence: local untracked `validation-artifacts/computer-use-followup-20261002/`, SHA-256 manifest `screenshots.json`. Representative screenshots: `picker-cancel.png` SHA-256 `104815116A22C485FD8A866A1162766D8ADC293E3B2AF75EAE53BD0F91F98588`; `error-restart.png` `21F18B696A5A9B1110D8F2B70BAA568D134ED2F3F42E70756ED23B57CDD87708`; `silent-restart.png` `05C7CBA4213A557F5F7644AB28178092D0B1E21A87C87AFF06121B06890A3B8E`; `debug-restored.png` `60BC488DD8D8EBCB6E5ED22FFACBD563C39CE23E718EE04EF070F894085D8C03`. Image payloads remain local. Windows Owner retains native tests; Cross-platform runtime wiring and prior button-wrap review unchanged.

## 2026-10-02 Extension WebSocket pairing and Desktop restart semantics

Starting documentation revision `60f95f0`, branch `codex/windows-validation-1f14cea`; baseline shared commit `1f14cea` unchanged. GUI evidence binds to v0.2.1-pre1 Full, associated source `3115c3b50716be0155804ad4f94dd9d29e37d617`, EXE fresh SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Owner manually enabled local WebSocket and saved pairing in Edge extension; browser tool remains unable to operate extension-scheme pages. Windows Computer Use verified Desktop accessibility state without emitting token.

- Initial WebSocket authentication: **PASS (subcheck, combined manual/automated evidence)**. Owner screenshot: WebSocket, authenticated, port 17321. Desktop: authenticated / connected, accepted 1, authentication received 1, success 1; handshake/authentication failure counters 0. This is authentication only, not real archive or Native Messaging acceptance.
- Normal Desktop exit and restart executed via Computer Use. New instance showed unauthenticated / not_loaded, accepted 0, success 0, initially and after a further 10-second observation. **Owner clarified restart resets the token; failure to reconnect with the previous token is expected**. Automatic reconnect with a stale token is not an acceptance requirement for this artifact. These counters do not prove an explicit old-token rejection: no new connection attempt was observed. Independent token rotation/rejection test remains NOT_RUN; Owner statement is expected-behavior provenance, not an automated rotation PASS.
- Current execution pauses before real archive: Owner must re-pair using the current token and supply a permitted public test-post URL. Same-session reconnect, browser restart, Native Messaging, actual media/SQLite consistency and duplicate submission remain NOT_RUN. No product defect or production change inferred from expected restart behavior.
- Tool capture had an extra 13x13 screenshot region; first indexed/coordinate clicks failed. Fresh observation identified actual 1280x760 region and recovered using its returned screenshot ID. Tool targeting failures are separate from product results. Source, branch and release acceptance unchanged.

Evidence local untracked `validation-artifacts/computer-use-pairing-20261002/`: `owner-extension-authenticated.png` SHA-256 `143E4B97E55CB6FFD5CED0AC66F1A58FBD8E6371FF4ABF21978A524C0C7C6276` (Owner token masked); `desktop-authenticated.txt` `3346653030DE32179E6167601EC92EDF58D45C2FC8141EABCE0C698AAF9AC6CF`; `desktop-after-restart.txt` `C044E68C9F3FA74B5741C90771B0EFC919DA3C4F3B0655F8C2019F8E287A9555`. No raw credential saved. Next execution Owner Windows; Cross-platform Telegram wiring and existing button-wrap review unchanged.

## 2026-10-02 Connection-status discrepancy: refreshed Extension and live socket

Starting documentation HEAD `b5b83cd`, same release Full artifact/source identity as preceding pairing record. Owner reports Desktop snapshot: accepted 3, handshake_failed 0, auth_read_failed 0, auth_received 3, auth_succeeded 1, auth_failed 2, auth_response_failed 0, close_before_auth 2, close_after_auth 1; unauthenticated / disconnected. This proves historical authentication success and closure, not the current connection state. Prior first-pair PASS is retained; current re-pair acceptance remains pending explicit Desktop refresh.

Owner refreshed the Extension options page without saving/reconnecting; it still shows authenticated on 17321. Screenshot retained locally as `validation-artifacts/computer-use-pairing-20261002/owner-options-after-reload.png`, SHA-256 `6EA6FEEF8005FDE5583811ECBA8A77970B65F74DF53A99B260D4654E50565347`. Token field blank on reload does not prove storage token deletion: options initialization loads enabled/port and status, not the saved token into the input. Token-bearing Desktop screenshot not copied into evidence or Git.

Read-only native endpoint check (approved escalation after sandbox CIM denied): exactly one Desktop process, PID 44384, target Full EXE path; listener 127.0.0.1:17321 owned by it and established loopback connection to client PID 26228, source port 54689. TCP Established alone does not prove WebSocket authentication. Multiple Desktop instances not observed.

Code inspection: packaged and checkout `extension/options.js` identical SHA-256 `0ED733F03A6BBAC76760AC22F0CC06C5C7B3122F851AC21BE6B5E403B60427A8`, snapshots update on load/save/reconnect, without live subscription/poll. Checkout `desktop/src/main.jsx:44,57-61` refreshes Extension explicitly, while periodic poll only refreshes jobs/batches; navigation preserves parent state and is not an equivalent refresh. Thus the earlier navigation-only Desktop re-pair check was insufficient. Desktop snapshot staleness is an inference, pending an explicit read; connection-close cause remains unclassified. No code change or completed shared-fix claim.

Computer Use indexed refresh click failed with coordinate geometry unavailable (tool failure, not product failure). Owner asked to click the actual Extension section refresh once after the options reload; current execution pauses for that result. Real archive/duplicates, browser restart and Native Messaging remain NOT_RUN. Windows Owner continues native evidence; any proposed shared connection-status implementation change routes to Cross-platform Owner review.

Owner then explicitly refreshed Desktop Extension section: still unauthenticated/disconnected, with accepted 8, auth_received 8, auth_succeeded 6, auth_failed 2, close_before_auth 2, close_after_auth 6; all handshake/auth-read/auth-response failure counters 0. Thus the navigation/snapshot-staleness hypothesis alone does not explain the refreshed observation. Initial authentication PASS remains historical; sustained/current re-pair connection is **FAIL (observed subcheck; root cause unclassified)**, real archive remains NOT_RUN. Do not equate failed authentication count 2 with all later closures: six authentications succeeded and six post-authentication sessions ended. No product implementation changed.

Candidate cause, not established: packaged Manifest V3 worker has no periodic heartbeat in inspected bridge/background code. Chromium guidance requires traffic inside a 30-second activity window to keep the extension WebSocket worker active: [Use WebSockets in service workers](https://developer.chrome.com/docs/extensions/how-to/web-platform/websockets). Actual Edge idle timing not measured; do not assert that Chrome guidance proves this Edge failure. Owner offered a bounded same-session test: reconnect once, explicit Desktop status reads around 5s and 45s, no Desktop restart or worker DevTools (which can change lifetime). Execution awaits Owner choice/manual browser action because extension-scheme automation is unavailable. Shared Extension/status changes, if pursued, route to Cross-platform Owner; Windows owns native timing evidence. Raw token-bearing new screenshot not retained in evidence/Git; numerical evidence comes from Owner message and screenshot observation.

## 2026-10-02 Current-token recovery, subsequent closure and Owner pause

Starting documentation revision `8beacbd406a21aac777a6b0dac84c7384bae2a51`, branch `codex/windows-validation-1f14cea`, tracked tree clean before this documentation-only update; untracked environments/artifacts preserved. GUI identity reused: Full v0.2.1-pre1, source `3115c3b50716be0155804ad4f94dd9d29e37d617`, build Run 36838400270, previously verified EXE SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. No fresh executable hash or automated UI execution claimed in this follow-up. Native Windows, Owner 200% display/Edge environment unchanged as reported.

- **PASS, current-token authentication subcheck (Owner manual)**: Owner reports Save and Connect showed authentication failure before entering the new token; failure-state screenshot was not supplied and the exact submitted credential is not established. After new-token entry, screenshots show Extension authenticated on 17321 and Desktop authenticated/connected. Desktop accepted/auth_received 16, auth_succeeded 10, auth_failed 6, close_before_auth 6, close_after_auth 9; handshake/auth-read/auth-response failures 0. This supersedes the prior FAIL for current re-pair only; cumulative closure counts alone had not established sustained idle failure.
- **FAIL, observed connection persistence/state agreement subcheck (Owner manual)**: subsequent explicit Desktop section refresh showed unauthenticated/disconnected; close_after_auth increased from 9 to 10 with other counters unchanged. Extension options still showed authenticated. This newly observed delta documents closure after the successful authentication snapshot, independently of the older cumulative totals. It does not establish that refreshing caused closure, exact session duration, which peer initiated it, or the MV3 hypothesis. Authentication PASS remains distinct; production root cause unclassified.
- **NOT_RUN**: controlled same-session 5s/45s idle measurement, real media/SQLite archive and duplicate submission, browser restart and Native Messaging acceptance. Owner supplied authorized public test URL https://x.com/thsottiaux/status/2105039482013757749 , then explicitly selected pause and Cross-platform investigation. URL recorded only; no page access, submission or download in this follow-up.

Evidence: Owner chat and four supplied screenshots (two before/after recovery, two after refresh). Safe masked Extension recovery screenshot retained locally/untracked as `validation-artifacts/computer-use-pairing-20261002/owner-current-token-authenticated.png`, SHA-256 `B6A8E4BD16A406FB064A89F87BE2F3BF7E18C77064211D1A572D31AF74742BE4`. Raw token-bearing Desktop screenshots were not copied into retained evidence/Git; only state/counters transcribed. Prior observations and classifications remain historical, with scope correction above.

`CROSS_PLATFORM_REVIEW_REQUIRED`: next Owner Cross-platform Owner should investigate shared Extension worker/WebSocket lifecycle and options status snapshots together with Desktop observation; identify closure initiator/timing before proposing a fix. Windows retains native reproduction and acceptance after Git handoff. No shared contract change established, no production modification, no release approval; this investigation does not block unrelated Telegram work. Documentation checks only: docs audit and diff check; handoff revision is the commit containing this record pushed to the named branch.

## 2026-10-02 Windows targeted revalidation — b015fbe

Input/implementation/validation source `b015fbe81a0b47c2b486a5256bd81ac95fd98d25`; branch `codex/windows-validation-1f14cea`, remote `origin` (`dev` explicitly fetched because the default fetch selects the validation branch). Clean tracked start at `7799afc`, fast-forwarded through Git to the exact supplied handoff; local untracked environments, components, logs and validation artifacts preserved. The source handoff still says CROSS_PLATFORM_IN_PROGRESS but explicitly assigns the next Windows checks; this run accepts that concrete scope, not an assertion that Telegram wiring is complete. Current Plan remains Telegram Batch A with runtime/UI/credentials unwired. No Windows implementation changes or direct source sync.

Environment: native Windows/NTFS E: workspace, PowerShell, Node v24.19.0, Cargo 1.98.0, rustc 1.98.0. Computer Use sky window inventory works; initial inventory contained no XArchive window. Edge browser connector available (id 3); extension-scheme operations remain manual under browser URL restrictions. Owner manually confirmed loading the current checkout Extension. Scaling reused from Owner's 200% environment report, not independently measured.

Scope: Targeted transport/background tests plus affected UI/native build; Full regression not run because this is a bounded connection-status change without dependency/schema changes. Previous Telegram/storage/icon/layout results stay evidence for their recorded revisions, not promoted to new GUI acceptance.

| ID / target | Method | Result / expected evidence |
|---|---|---|
| WQ-WS-02/03 native transport regression subset | automated | PASS 6/6: live-socket state after recent request, defaults, correct/wrong token, request routing, accepted non-blocking stream. `cargo test -p xarchive-desktop --lib websocket_transport::tests -- --test-threads=1`, exit 0. Test EXE SHA-256 `B0D0C5795CA3B814D589F7A7B0590DFD6E0E25114A6BE362E523756F3E554809`. Loopback tests do not prove Edge lifecycle. |
| WQ-WS-02/03 Extension regression subset | automated | PASS 19/19: `node --test extension/tests/background.test.js`, exit 0, including stale live-state and status-read reconnect cases. Browser/socket fakes do not prove a real MV3 worker. |
| Affected syntax/build | static/build | PASS Extension `npm run check --workspace extension`; UI `npm run build --workspace desktop`; native `node desktop/scripts/build-tauri.mjs --no-bundle` (dev channel, release optimization), all exit 0. Linker informational warnings and Vite mixed-import warning retained, no build failures. |
| Formatting/docs | static | PASS `cargo fmt --all -- --check`, `node scripts/docs-audit.mjs`. |
| Status command test-name probe | automated discovery | NOT_APPLICABLE: filter `extension_status` matched 0 tests (176 filtered), exit 0; this is not a test PASS or additional coverage. No assertions changed. |

Build provenance: fresh current Desktop copied into `validation-artifacts/windows-batch-b015fbe/app/xarchive-desktop.exe`, SHA-256 `B3550F5CEB6764C2D8DD1E06B94D1E143B936B7B28ACA318F8549EDF1677A1F2`. Current Extension copied from the checkout; downloader/gallery-dl/aria2 components reused from the Owner's older Downloads Full package solely for a local integration fixture. This directory is not a new canonical Full build, release asset or packaging PASS. No user config/DB copied from the old package. GUI launch and portable download initialization observed via Computer Use; first-use prompt disappeared and new directory selected. One screenshot initially captured the foreground Edge surface despite Desktop AX state; activating the selected Desktop recovered before input (tooling observation, not product failure).

Evidence local/untracked: `validation-artifacts/windows-batch-b015fbe/`, logs `extension-tests.log`, `websocket-tests.log`, `extension-check.log`, `ui-build.log`, `tauri-build.log`, `status-command-tests.log`, SHA-256 manifest `log-hashes.json`. Native integration, timed idle and real archive results are recorded separately below. Priority P0 for connection acceptance, owner Windows Platform Owner, implementation IMPLEMENTED; blocks_development=no, no release approval. Revalidate after related bridge/transport/build/environment changes.

Shared follow-up identified by current source inspection: handoff asks the Owner to read `last_request_age_seconds` from Desktop diagnostics, but `desktop/src/pages/settings-page.jsx:40` renders only the earlier counters; the new Rust field is absent from JSX. `CROSS_PLATFORM_REVIEW_REQUIRED`: expose it or provide a supported equivalent readout before that exact manual evidence requirement can be completed. Backend unit test covers its value, not GUI visibility. No production edit made to compensate.

### Current native/browser observations and disposition

- **PASS authentication subcheck**: Owner manually loaded checkout Extension and entered the new Desktop's current token; reports authenticated on 17321. Computer Use independently read authenticated/connected at baseline; accepted/auth_received 7, success 1, failed 6, close_before 6, close_after 0, other failure counters 0. Prior wrong/stale attempts remain observed counters, not six product defects or erased retries.
- **FAIL observed connection-persistence subcheck**: later Desktop display was unauthenticated/disconnected, close_after 1, other counters unchanged. Owner explicitly refreshed Desktop then options: Desktop unauthenticated, options authenticated. To account for status-read self-healing, Owner refreshed only Desktop once more; still unauthenticated/disconnected, accepted/auth_received 8, succeeded 2, failed 6, close_before 6, close_after 2, all handshake/auth-read/auth-response failures 0. A new successful authentication and a later close occurred after the options read. No Desktop restart or token change was requested in this sequence. Cause/close initiator/duration unclassified; sequential observations cannot prove simultaneous live-state disagreement or failure of the two cached-flag fixes in isolation.
- **NOT_RUN complete controlled ~5s/~45s idle test**: initial safe display baseline `2026-10-02T05:54:18.204Z`, later safe snapshot `2026-10-02T05:58:14.788Z` (13:54:18.204 / 13:58:14.788 Asia/Shanghai), gap 236.584s. An explicit 45s tool wait was performed, but subsequent manual reading latency prevents a 45s close-timing claim. `last_request_age_seconds` absent from UI; no equivalent IPC read performed. Evidence `pairing-t0.txt`, `pairing-later-snapshot.txt`, Owner messages and final counters.
- **NOT_RUN real archive/duplicates**: authorized X URL was read through Edge; target main post and XArchive save button visible. No save click/download/SQLite completion occurred. Execution paused on reproduced connection closure under Owner's earlier choice to hand issues back to Cross-platform. Reused Sidecar components were copied as a fixture, not exercised or accepted as current-source workers. Native Messaging/worker/browser restart and pending cleanup remain NOT_RUN; Telegram entry points remain IMPLEMENTATION_NOT_READY.

Final follow-up: `CROSS_PLATFORM_REVIEW_REQUIRED` for post-auth closure/status-read reconnect investigation plus missing diagnostic readout. Existing backend and bridge regression PASS remain; native lifecycle acceptance is not closed. Next Owner Cross-platform Owner, Windows retains controlled real-browser/native reproduction after the next handoff. No production/security/credential code changes. Tauri CLI rewrote Cargo.toml line endings only (content diff empty), restored that tool-produced change before final documentation review. No Full regression or release approval; validation/handoff revision is this record's Git commit.

## 2026-10-02 automatic pairing native implementation continuation

*Restored verbatim from evidence commit `30fc575` (branch `codex/browser-automatic-pairing-windows`), where this record was originally written; the section was previously absent from this branch.*

Owner Windows; branch `codex/browser-automatic-pairing-windows`; source `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e`; tested implementation is this commit's diff. Native Windows/PowerShell, Rust/Cargo 1.98, Node 24.19; existing E: caches/user data preserved, Git fetch/fast-forward only. Debug Host SHA-256 `dff2ecd767bf3d9b8d44c7724e732547466fffafaa365546cd61f40677de84ee`.

- PASS: Host library 11 + entry 1 + Windows process framing 1 (`cargo test -p xarchive-native-host --offline`). Stalled read/write deadline test checks real pipe backpressure. Unix integration NOT_APPLICABLE.
- PASS: Desktop Pipe/Registry targeted 4 and WebSocket targeted 12; `cargo test -p xarchive-desktop windows_transport --offline`, `cargo test -p xarchive-desktop websocket_transport --offline`. Existing business framing preserved; ticket consumption, exact manifest identity, duplicate listener and shutdown covered.
- Initial FAIL: bootstrap test received empty response before data arrived on nonblocking pipe; bounded retry of zero-byte reads repaired it. Initial Desktop module run hung in shutdown test and was terminated by its exact test process ID. First timeout fix still FAILed <1s at ~3s; stop-aware pre-auth polling and authenticated polling then PASS, unchanged assertion.
- Strict Clippy initially FAILed existing Registry collapsible-if; localized repair then PASS. Cargo incremental cache finalization AccessDenied notes and MSVC linker output warnings are tooling notes, not product failures. CIM process inventory was denied; Get-Process provided equivalent test-process identification.
- Desktop module rerun: 186/187 PASS, one config persistence test FAILed with AccessDenied under sandbox. The exact unchanged config test PASSed under permitted escalated execution, confirming an environment restriction; no config code changed. This is not a claim of a clean full-module run.
- Read-only Registry inspection: HKCU Edge and Chrome host keys ABSENT. NOT_RUN: installed registration/repair, cross-user ACL denial, Full package, native GUI, actual browser/MV3/E2E; prerequisites and current scope are in the current queue. No release publication or real archive performed.

`CROSS_PLATFORM_REVIEW_REQUIRED`: stop-aware shared WebSocket wrapper with Windows-only polling. Return through Git for Linux review/revalidation; no schema/protocol change.

## 2026-10-02 — exact 6060f5a Windows validation return

Source/handoff: 6060f5a2d554d32fa8400f7f5084319b19fabeb1; validation branch: codex/windows-validation-6060f5a. Native Windows PowerShell, Node and Rust/Cargo are available. Explicit Git fetch retrieved this SHA; tracked tree was clean at validation start. Existing untracked caches, artifacts and user data were preserved. No filesystem synchronization or production-code change.

Revision prerequisite: d65a01bfe2c6f93a071322bfef154fd947b0b70b is not an ancestor of this source (git merge-base --is-ancestor exits 1). Its Windows Host/Named Pipe/coordinator and stop-aware polling implementation, and 30fc575 evidence, remain on the prior Windows branch. Prior targeted PASS results do not apply to the missing implementation at 6060f5a.

| Check | Result | Evidence / limitation |
|---|---|---|
| Targeted package/UI Node tests | PASS 27/27 | node --test desktop/test/extension-package.test.mjs desktop/test/ui-state.test.mjs on Windows |
| Native Host release build | PASS | cargo build -p xarchive-native-host --release --offline |
| Canonical Full build | PASS | node desktop/scripts/build-portable-windows.mjs; fixed Extension ID; output dist-portable/validation-6060f5a; no binary-reuse fallback. Existing packaged worker dependencies reused; no fresh-worker or GUI acceptance claim. Rust unused pairing members and Vite chunk warnings are nonfatal. |
| Actual packaged Host framing diagnostic | PASS | Spawn packaged Host with fixed browser origin, one little-endian length-prefixed bootstrap frame, five-second process bound. Correlated response: BOOTSTRAP_NOT_IMPLEMENTED, retryable=false. This proves an implementation prerequisite failure, not successful pairing. |
| Actual Extension inventory | PASS scoped | extension-package.mjs plan + verify against Full extension directory: 13 files; src/browser-pairing.js exists. No archive extraction/install acceptance performed. |
| Full installation inventory | FAIL scoped | package-manifest.json installation.files still omits extension/src/browser-pairing.js, although the actual file ships. Shared native-host-package.mjs list is unchanged by 6060f5a. |
| Automatic pairing / UI mismatch reproduction / E2E / lifecycle | NOT_RUN | IMPLEMENTATION_NOT_READY on the exact tested 6060f5a source, which lacks Windows bootstrap. Do not replace working registration or claim historical d65a01b screenshots against this artifact. Subsequent Git integration is tracked separately and has no Windows result in this receipt. |

Artifact: E:/Shiraishi/VSCode Workspace/Tw2Tg/dist-portable/validation-6060f5a. SHA-256 Desktop 61cd19f0f9a192dbf21d1e494e360b30859134f7f8bbbda99c62809e2d3c6f86; Native Host 06d5fd6ff0296d5ec09d2771de08979cac5699c14214f47e6c495c0d114bbc81; browser-pairing.js ace9ba7defb8eb452f2f94e6da3c643787c2d3b721dbd13c09413e1b21efe351. Local probe and Extension inventory evidence: validation-artifacts/6060f5a/result.json and extension-plan.json. Extension per-file hashes: validation-artifacts/6060f5a/extension-hashes.json, inventory SHA-256 4c1c38f6bc76edafd19f82998d9d38851925f17d7774ae1e72975eb4721f3a6a. These local artifacts are not committed. Documentation audit and final diff whitespace check PASS.

Reconciliation clarification: earlier text describing a Full package as missing the helper must not be read as proof of an absent on-disk file. The prior d65a01b artifact shipped the helper but omitted it from installation.files; preserve the historical FAIL and this clarification. 6060f5a adds the standalone Extension required-file guard, not the Full installation-list correction.

Next owner: Cross-platform Owner. CROSS_PLATFORM_CHANGE_REQUIRED: integrate the Windows branch through Git and identify the resulting source revision before re-handoff. CROSS_PLATFORM_REVIEW_REQUIRED: review Windows stop-aware shared WebSocket wrapper, and fix Full installation inventory while preserving package abstraction. Shared UI/status and automatic-options semantics require their own review; do not infer the authoritative state from screenshots.

Manual queue after integrated source: rebuild and bind Desktop/Host/Extension hashes; register that Host; capture redacted browser_connection, websocket_connection, websocket_authenticated plus sidebar/detail at connect and 5s/45s idle; query status; execute the authorized archive fixture and duplicate/response-loss cases; MV3 worker/browser/Desktop restart and sleep; Edge/Chrome profiles; current-user/cross-user ACL; install/upgrade/uninstall/move and redaction. All remain NOT_RUN on this source. Previous automation BLOCKED and product FAIL records remain attached to their original artifacts.
