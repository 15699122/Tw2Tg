# Windows 40858835 validation return — 2026-10-06

## Revisions, environment and scope

- Input: `40858835d249ead37ad1f3efb2dc1571821f9f8f`; input implementation `8a8fa8ab3cb4725282fd10d15aa178deda6fcbfd`, documented handoff `a3b7870b12eb5df5ee86139c89f23a2e8a671818`. The requested input includes subsequent documentation-only corrections. Fetch first; explicit SHA fetch was required because the configured fetch selected the previous Windows return branch.
- Return branch: `codex/windows-validation-40858835`. Windows implementation and final validation: **`60b76f357acbcf3510fa9bfcdc32309a35ca97cf`**. The documentation handoff revision is the commit containing this report; resolve through Git history on the return branch.
- Input tracked tree clean; existing untracked dependencies, caches, validation artifacts and user data preserved. Formal source update used Git only. Tauri rewrote Cargo.toml line endings during build without a content diff; that generated change was restored after the build.
- Native Windows 11 Pro for Workstations Insider 10.0.29680 / x64, NTFS E:, PowerShell 7.6.6, Node 24.19.0, Rust 1.98.0. Default Python is 3.14.7; validation interpreter explicitly `.venv-windows-validation/Scripts/python.exe` 3.12.14. WebView2 installed versions 154.0.4258.48 and .53; exact loaded runtime version and monitor DPI were not established.
- Computer Use available. Observed ordinary capture 1082×790 and maximized 1280×752; image bounds do not prove OS scaling. A 13×13 ancillary capture and UIA hit-position/focus limitations were observed; full-window screenshot coordinates provided an alternative for supported navigation. An unrelated always-on-top window occluded lower controls. No capture containing that application's content was persisted or committed.
- Read ownership, Git handoff, validation policy, Windows recipes/current queue/manual steps and pairing/settings plans. `cross-platform-validation.md` has no §16; applied its authoritative links and `git-platform-handoff.md` §16 revision template. Batch plan: [Windows 40858835 plan](../development/windows-40858835-validation-plan.md).
- Scope: affected frontend event path, Storage/history ordering, Desktop module, exact-artifact packaging and native GUI subset. Full workspace regression was not run: the change is limited to UI plus a history-position query/command. External accounts, Telegram credentials and real download jobs were not used.

## Input defect and minimal repair

Input Sidebar download and SQLite handlers call `refreshDownloadPage` without receiving it. The received branch omitted the prior Windows binding repair. Direct execution of the actual JSX-transformed Sidebar handler independently reproduced `ReferenceError: refreshDownloadPage is not defined`; the 82-test source suite passed despite this missing binding.

`60b76f3` supplies the existing callback through the parent and Sidebar prop, and restores an event-invocation regression covering both actions. This preserves the existing abstraction, protocol and behavior: **CROSS_PLATFORM_REVIEW_REQUIRED**. Cross-platform Owner must integrate/review this repair and retain the regression when reconciling branches. No shared architecture, executor policy or metrics contract was changed by Windows.

## Artifact identity and packaging limits

Canonical build command: `node desktop/scripts/build-portable-windows.mjs`, Full/dev, `PORTABLE_ALLOW_BINARY_REUSE` unset. Input and repaired Desktop builds succeeded. First assembled packages failed gallery-dl pin validation because the wrong local component directory was temporarily replaced; they were rejected before GUI acceptance. Corrected `sidecar/gallery-dl` input was used for a further canonical build. Original local gallery inputs were restored and hash checked.

Final candidate: `dist-portable/XArchive-0.2.1-dev-60b76f3-verified-windows-x64-full-validated.zip`, 42,303,047 bytes.

| Artifact | SHA-256 |
|---|---|
| Final ZIP | `0022b226ea4d851a321561495a65af77dd7bc92f011c1ef08f86693f3c04b6c6` |
| Desktop | `7d88d988f73ad44d32851a1e61804f5c70ba665253fa214e00b643fb89749a5a` |
| Native Host | `34d8ad97ff6afce042ca6950ad89364eb18d7f77b819ad6eb18b76b35a9a4665` |
| Worker | `96c19695ac46e30aa23af184ed41d0c8339fe4c98a2d771cf0781906402a60c2` |

Desktop was freshly built at the final implementation SHA. Native Host is unchanged from the verified earlier build. Worker was copied from the existing local packaging input; its fresh-build/source provenance is **not established** and its hash differs from the earlier f22 package. Independent startup/help and runtime-library inventory passed, but this is not worker source parity or download acceptance. Fresh worker provenance remains a packaging follow-up before release acceptance.

The first verified ZIP `d0b75a66…` had an overly broad component-reuse label inherited by the local verification helper. Corrected build identity metadata explicitly records the worker provenance limitation; the final ZIP above supersedes it. Executable/component bytes are unchanged between those ZIPs, so native observations bind to the same exact Desktop hash; only identity metadata changed. No product code was changed to accommodate packaging inputs.

Inventory/ZIP checks: 80 files, every installation-manifest file, pairing helper, x64 PE executables, Python runtime and 8 worker .pyd modules; pinned gallery SHA and aria2 verified-extraction hash; component version/help exit 0; ZIP CRC and every file SHA matched. This is a dev portable validation candidate, not MSI/NSIS/signing or release GO.

Local evidence root: `validation-artifacts/windows-40858835/`. Evidence files remain local; no credentials, user archives, binaries or unrelated screenshots are committed.

## Executed results

| ID / target | Status and method | Evidence / limits |
|---|---|---|
| FRONTEND-INPUT / affected source tests | PASS / automated | `frontend-tests.log`, 82/82, exit 0; does not establish Sidebar event correctness. |
| SIDEBAR-EVENT / real handler invocation | Input FAIL → repaired PASS / automated | `sidebar-input-fail.log`, actual ReferenceError, exit 1; `frontend-repaired.log`, 83/83 including both handlers, exit 0. Original failure retained. |
| STORAGE-HISTORY / ordering and pagination module | PASS / automated | `storage.log`, 74/74, exit 0; includes different created/updated order regression. |
| DESKTOP-MODULE / native library | Initial FAIL; final PASS with environment correction / automated | `desktop-module.log`: 254 PASS/3 FAIL/1 ignored; sequential `batch-retry.log` repeated three discovery-stub failures. Default `python3` did not provide the usable fixture interpreter. `desktop-python-explicit.log` with Python 3.14 restored batch tests but got one WinHTTP PAC fixture failure. `desktop-final.log` with explicit Python 3.12: 257 PASS/1 ignored, exit 0. PAC failure is intermittent; no production change or assertion weakening. Ignored Credential Manager mutation test remains unexecuted. |
| FULL-INVENTORY / fresh Desktop, component inventory and ZIP | PASS scoped / automated | Build logs, `package-verification.json`, `package-files-sha256.json`, `verify_package.py`; initial gallery integrity FAIL retained, corrected assembly PASS. Worker fresh-source provenance remains NOT_RUN. |
| WQ-UI-CONSISTENCY-01 / ordinary and maximized layout | PASS scoped / gui-automated | Actual packaged WebView2 observed; task buttons horizontal, card history/metrics, long ID/error wrapping, Batch field labels. Lower controls were partially occluded; no whole-window/DPI PASS. `dashboard-fixture.txt`, `dashboard-maximized.txt`, `batches-maximized.txt`. |
| WQ-UI-CONSISTENCY-01 / detail index 0 | PASS scoped / gui-automated | `fixture-identity.json`, `detail-index0-page2.txt`: 21 terminal synthetic records with reversed updated vs created order; first dashboard summary correctly opened its matching sole page-2 record and detail message. No actual job submitted. Index 19 remains deferred. |
| WQ-UI-CONSISTENCY-01 / 20+1, empty/loading state | PASS scoped / gui-automated | Empty history and disabled end controls observed; loading text observed during first navigation. Detail page 2 contains one record/disabled next; previous returns 20 records, counted in `history-page1.txt`. Does not cover injected error state or complete repeated next/previous interaction matrix. |
| WQ-UI-CONSISTENCY-01 / section navigation | PASS scoped / gui-automated | Mouse task-record anchor targeted history; Tab visibly focused download-settings button, Return scrolled to its section. `download-settings-anchor.txt` plus live tool observations. Full keyboard order/Space/hidden-panel exclusion not run. |
| WQ-UI-CONSISTENCY-01 / Extension navigation/copy | PASS scoped / gui-automated | Actual service status click opened expanded matching Settings disclosure. Current disconnected state, localized “未启动”, historical cumulative counter label observed in `extension-panel.txt`. Five-state mapping has source-test coverage only; real connected/browser/reconnect not run. |
| WQ-DL-06 / backend help | PASS scoped / gui-automated | `transfer-copy.txt` warns rebuild may interrupt active tasks. No backend toggle or active-task cancellation acceptance in this round. |
| DOCS / governance and whitespace | PASS / static | `docs-audit-before.log` and `docs-audit-final.log`, exit 0; final `git diff --check` exit 0. |

All rows bind to input `40858835` or repaired `60b76f3` as stated, the artifact table, Windows environment above, and their named reproducible commands/tool observations. Neither build nor GUI fixture navigation proves a real completed archive, runtime metrics, browser automatic pairing or external-service delivery.

## Manual Windows queue for this return

Common owner: Windows Platform Owner; implementation IMPLEMENTED except explicitly deferred runtime metrics; source `60b76f3`; artifact hash as above; method manual/native GUI. Prerequisite: integrate Windows repair, retain exact-artifact identity, isolated synthetic profile, remove unrelated occluding windows. Nonblocking to Linux development; no release acceptance inferred. Record screenshots, display scaling, loaded WebView2 version, values and every expected/actual outcome before closing the umbrella row.

| ID / priority | Status / defer reason | Reproducible next steps and expected result |
|---|---|---|
| WQ-UI-CONSISTENCY-01-DETAIL19 / P1 | BLOCKED / COMPUTER_USE_UNAVAILABLE: summary-index-19 action occluded by another app | 1. Use the existing 21-row reversed-order fixture. 2. Open dashboard summary index 19 (`windows-ui-fixture-20`). 3. Confirm its detail on history page 1, not a summary-index-derived page. Save unobstructed evidence. |
| WQ-UI-CONSISTENCY-01-DATE / P1 | BLOCKED / COMPUTER_USE_UNAVAILABLE: native date value entry not confirmed by UIA/type-text attempts | 1. Enter controlled start/end dates by real keyboard/date picker without creating a batch. 2. Try an earlier end date. 3. Confirm valid entry, clear constraints and correct focus; no real account or discovery request. |
| WQ-UI-CONSISTENCY-01-DPI / P1 | NOT_RUN / minimum incremental scope; OS DPI not established or changed | 1. On approved isolated display/VM, inspect ordinary/maximized/minimum 720×540 and 100/125/150/200% scaling. 2. Use long IDs, paths/errors and metrics. 3. Require no clipping/overlap and unobstructed screenshots; identify actual runtime/scaling. |
| WQ-UI-CONSISTENCY-01-STATES / P1 | NOT_RUN / controlled five-state and error/busy fixtures absent | 1. Exercise connected/disconnected/not_loaded/checking/unknown and target disclosure. 2. Inject controlled history error/loading. 3. Require localized unknown and readable recovery state; no real credentials. |
| WQ-UI-CONSISTENCY-01-FOCUS / P2 | NOT_RUN remainder / only targeted Tab/Enter verified | 1. Check both anchors with Enter/Space, reverse navigation and focus outline. 2. Traverse Batch/Telegram/Proxy and collapsed settings. 3. Hidden controls must be excluded and field heights/labels consistent. |
| WQ-DL-01..08 / existing priorities | NOT_RUN remaining acceptance / account, browser, controlled downloads and full matrices not supplied this UI batch | Execute corresponding P1–P8 in `windows-manual-steps.md` on this new exact artifact. Worker startup/inventory and synthetic FAILED rows do not close archive completion, real aria2, restart/credential or runtime metrics targets. Preserve IMPLEMENTATION_NOT_READY for executor metrics. |
| FULL-WORKER-PROVENANCE / P1 | NOT_RUN / existing local worker build origin unconfirmed | Rebuild worker using documented locked inputs or establish a traceable reviewed origin; record source/dependencies and hash, then repeat affected component/integration checks. Current inventory PASS is narrower. |

Revalidation required after shared repair integration or materially changed UI, command, dependency, artifact, WebView2 or DPI target. Existing historical download and WebSocket-close FAILs remain unresolved historical evidence; no equivalent current-artifact check superseded them.

## Follow-up and next Owner

**Cross-platform Owner next**: fetch this return, review/integrate `60b76f3` under CROSS_PLATFORM_REVIEW_REQUIRED, retain the Sidebar regression, reconcile current Plan/handoff, and investigate the repeated omission of the prior Windows binding repair. No new shared-contract change is required by this repair. Previously deferred executor metrics and lifecycle/transport findings stay in their existing queues.

Windows Platform Owner retains the remaining manual/native matrix, artifact source-provenance completion and real integration validation after reconciliation. State: WINDOWS_VERIFICATION_PENDING, not WINDOWS_PASS or COMPLETE.
