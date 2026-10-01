# Desktop UI Density, Settings Order and HiDPI Icon Fix Plan

Owner: Linux Cross-platform Owner (shared CSS, shared page composition, shared icon component, shared icon asset generation), Windows Platform Owner (Windows GUI acceptance, Windows native icon embedding, packaged artifact inspection).
Status: `IMPLEMENTED_ON_LINUX` — plan recorded 2026-10-01 on `dev` at `c2b754b825e618eade7533e8f1f0ae9455cf1fc3`; Linux implementation and Targeted/Module checks are complete and the Windows queue carries `WQ-UI-030-01`..`WQ-ICON-030-06`. This document is a plan plus an implementation record, not validation evidence.

## 1. Objective

Close six defects reported by the user against the current release build, all visible in Windows screenshots of the dashboard, the settings page and the Windows title bar:

1. Sidebar `服务状态` rows are spaced too loosely.
2. In `运行环境`, the `管理组件与设置` link sits too far below the `启动 Sidecar` / `停止` buttons.
3. On the settings page, the separator above `Core Bootstrap` should be removed while the text gap stays as it is.
4. The `网络代理` section should move to the bottom of the settings page, next to `存储位置` and `日志设置`.
5. The `工作台` navigation icon is the same waveform icon used for `Sidecar 配置`.
6. The new application icon is visible on Windows but blurry, so high-definition assets are required.

The outcome is a denser sidebar, a control panel whose actions keep a constant vertical rhythm, a settings page without a first-section separator and with a single settings group at the bottom, a distinct dashboard icon, and crisp icon assets at every size Windows requests.

This batch changes presentation only. It does not change proxy resolution behavior, settings persistence, log levels, sidecar lifecycle, or any Rust command contract.

### 1.1 Screenshot provenance caveat

The screenshots show version `v0.2.0`, but the visible sidecar path contains `windows-6be3269-20261001-prerelease`. The evidence therefore comes from a pre-release build of `6be3269`, not necessarily from the published `v0.2.0` assets. Every acceptance record must name the actual build revision it was produced from; a version label alone is not sufficient evidence.

## 2. Confirmed causes

| # | Location | Cause |
|---|---|---|
| 1 | `desktop/src/style.css` (`.sidebar-caption`, `.connection-line-button`) | The caption carries `margin: 0 8px 12px`, and each status row carries `min-height: 30px`, `padding: 5px 8px` and `margin: 2px 8px`. At the DPI used for the screenshots the three rows occupy far more vertical space than the design intends. |
| 2 | `desktop/src/style.css` (`.dashboard-grid .control-panel-content .settings-link`) | `margin-top: auto` pushes the settings link to the bottom of the stretched card. The gap above it therefore grows with the job list, and shrinking `.control-panel-content`'s `gap: 16px` alone cannot fix it. |
| 3 | `desktop/src/style.css` (`.settings-section`) | Every section, including the first one, draws `border-top: 1px solid var(--border)`. The line above `Core Bootstrap` is this rule, not a `Separator` component, so removing it must target `#bootstrap-settings` only. |
| 4 | `desktop/src/pages/settings-page.jsx` | `ProxySettings` is rendered after `Sidecar 配置` and before `Aria2Settings`, while `存储位置` and `日志设置` live in the trailing `settings-layout-secondary` container. The order is purely a consequence of JSX position. |
| 5 | `desktop/src/main.jsx` (`Sidebar`) and `desktop/src/components/icon.jsx` | The `工作台` nav item uses `icon="activity"`, and `PATHS.activity` is the ECG waveform also used for the not-started sidecar `StatusRow` in both `settings-page.jsx` and `dashboard-page.jsx`. |
## 3. Scope

### 3.1 Sidebar `服务状态` density

- Tighten only the sidebar service-status block; do not touch the main navigation rows, the settings nav row, or global button sizing.
- Reduce `.sidebar-caption` bottom margin, the row `min-height`, the row vertical padding, and the inter-row margin together.
- Preserve the three-column grid, the status dot, the label, the trailing status text, the hover background, and `:focus-visible`.
- Preserve the click-to-settings behavior and `aria-label` of every row.
- Values are a starting point and may be adjusted after a rendered check; the requirement is a visibly tighter block with no clipping of Chinese labels.

### 3.2 `运行环境` action spacing

- Remove the `margin-top: auto` on the settings link inside the Dashboard control panel so the status row, the start/stop buttons, and the settings link follow normal document flow.
- Keep the two cards stretched to a shared bottom edge as delivered by the earlier UI polish batch; the extra height must fall below the action group, not between the buttons and the link.
- Use a single consistent vertical gap for the action group, to be tuned after rendering.
- Keep `启动 Sidecar` and `停止` side by side and equal width, and keep the existing narrow-window wrap behavior.
- Do not add spacers, fixed heights, or JavaScript measurement.

### 3.3 Settings first-section separator

- Remove the top border of `#bootstrap-settings` only.
- Compensate the removed `1px` border so the `Core Bootstrap` title keeps its current vertical position relative to the page description.
- Keep `padding-top` and the separator behavior of every other settings section unchanged.

### 3.4 Settings section order

Target order:

1. `Core Bootstrap`
2. `Sidecar 配置`
3. `aria2` (Windows only)
4. `浏览器 Extension`
5. `存储位置`
6. `日志设置`
7. `网络代理`

- Move the existing `ProxySettings` element into the trailing settings container instead of duplicating it.
- Keep the section id, props, save and inspect callbacks, validation messages, manual proxy input, and existing state management untouched.
- Visual order, DOM order, and Tab order must agree.
- Do not fix, extend, or re-scope PAC/WPAD resolution or any other proxy work in this batch.

### 3.5 Dashboard navigation icon

- Add a `dashboard` entry to `PATHS` in `desktop/src/components/icon.jsx`, drawn as a simple partitioned panel outline in the same `24x24`, stroke-based style.
- Use it for the `工作台` nav item only.
- Keep `activity` for the not-started sidecar status rows.
- Keep the existing render size in the navigation and introduce no icon library or new dependency.

### 3.6 HiDPI icon assets

## 4. Ownership routing

- Linux Cross-platform Owner: `desktop/src/style.css`, `desktop/src/pages/settings-page.jsx`, `desktop/src/main.jsx`, `desktop/src/components/icon.jsx`, `desktop/scripts/make-icon.py`, the generated assets under `desktop/src-tauri/icons/`, shared test updates, Linux checks.
- Windows Platform Owner: the native title bar and taskbar icon, packaged artifact inspection, DPI matrix acceptance, and any native or packaging fix required for item 6.
- Changing icon-generation semantics, the shared icon component contract, or the settings section contract is `CROSS_PLATFORM_CHANGE_REQUIRED` and returns to Linux. This batch deliberately stays inside existing abstractions, so no shared contract changes.
- A small shared adjustment that preserves an existing abstraction is `CROSS_PLATFORM_REVIEW_REQUIRED`.

## 5. Validation plan

Escalation starts at Targeted and escalates only if the impact area requires it.

| Area | Content |
|---|---|
| Frontend targeted | Extend `desktop/test/ui-wiring.test.mjs` so the settings section order, the navigation icon mapping, the removal of `margin-top: auto` on the settings link, and the first-section border rule cannot silently regress. Add icon asset assertions that the ICO size list and the PNG dimensions match the generator. |
| Frontend build | `npm run check` in `desktop`. |
| Module | `npm test` in `desktop`. |
| Rendered check | Dashboard, settings page, and narrow window, comparing the service-status block and the control-panel action group before and after. Static assertions are not visual acceptance. |
| Windows native | Title bar, taskbar, Alt+Tab, Explorer, and the tray icon if one exists, at 100%/125%/150%/200% scaling, distinguishing a stale icon cache from an asset defect. |
| Windows GUI | The five spacing/order/icon items in the running application, with the build revision recorded. |

Full regression is not run for this change set because the diff is limited to CSS spacing, JSX ordering, one icon path, and regenerated image assets; Rust and packaging tests do not cover these paths. The reason is recorded in the round result.

When GUI automation is unavailable, record `BLOCKED` with `COMPUTER_USE_UNAVAILABLE` plus a manual validation item. An unexecuted GUI check is never recorded as PASS.

## 6. Handoff

- Deliver this batch through Git as one commit group, recording branch, source commit, handoff commit, uncommitted-state status, and owner.
- Do not overwrite the Windows Owner's canonical working tree; direct sync is diagnostic only.
- Review the final `git diff` before finishing.
- This batch does not close any open `v0.2.0` acceptance gate, does not close `WQ-LOGS-020-02`, and does not alter any recorded Windows result.
- This batch does not publish a release, does not modify a GitHub Release, and does not bump a version file. It is not a release approval.

## 7. Completion criteria

- The sidebar `服务状态` block is visibly tighter with no clipped labels and intact keyboard focus.
- `管理组件与设置` follows the start/stop buttons at a constant gap in empty, populated, running, and stopped states.
- No separator remains above `Core Bootstrap`, its title keeps its current position, and other settings separators are unchanged.
- `网络代理` renders last, after `日志设置`, with DOM and Tab order matching the visual order.
- The `工作台` icon is visually distinct from the sidecar waveform icon.
- The icon assets are regenerated at full resolution and Windows confirms the embedded and displayed icons are crisp at every supported scale, with the build revision recorded.
## 8.1 What changed on Linux

1. `desktop/src/style.css`: `.sidebar-caption` bottom margin `12px → 6px`; `.connection-line-button` `min-height 30px → 28px`, `margin: 2px 8px → 0 8px`, `padding: 5px 8px → 4px 8px`. `.nav-item` keeps `min-height: 40px` and no global button sizing changed.
2. `desktop/src/style.css`: `.dashboard-grid .control-panel-content .settings-link` changed from `margin-top: auto` to `margin-top: 0`. The flex `gap: 16px` on `.control-panel-content` now supplies the rhythm, and `.dashboard-grid` keeps `align-items: stretch`, so the slack falls below the action group. Shrinking the gap alone would not have fixed this, because `margin-top: auto` absorbed all remaining height.
3. `desktop/src/style.css`: added `#bootstrap-settings { padding-top: 25px; border-top: 0; }`. The `+1px` compensates the removed border so the `Core Bootstrap` title keeps its current vertical position. The generic `.settings-section` border and `padding: 24px 0` are untouched, so every other divider is unchanged.
4. `desktop/src/pages/settings-page.jsx`: the existing `ProxySettings` element moved into the trailing `settings-layout-secondary` container after `日志设置`. It was moved, not duplicated; the id, props, callbacks, validation and state management are unchanged, and no PAC/WPAD work was touched.
5. `desktop/src/components/icon.jsx` and `desktop/src/main.jsx`: added a `dashboard` path (`M4 5a1 1 0 0 1 1-1h14…`, a panel outline with a top band and a left column) and used it for the `工作台` nav item. The sidecar `StatusRow` keeps `activity` in both pages.
6. `desktop/scripts/make-icon.py`: `render()` now draws the plate, the box and the `X` at the supersampled resolution and downscales exactly once; the previous `plate.resize(...)` upscaling is gone. `SIZES` gained 20 and 40 for taskbar and Alt+Tab, keeping every prior size, and the whole asset set was regenerated: the ICO now carries 16/20/24/32/40/48/64/128/256, each as its own PNG payload.

## 8.2 Linux validation

- `npm test` (desktop) **188/188 PASS**, up from 182 with six new cases: sidebar row density with navigation-row and focus-ring guards; the control-panel link gap with `margin-top: auto` banned and stretch retained; the first-section separator with its `1px` compensation; the settings section order including the `Aria2Settings` call site, with a single `ProxySettings` render asserted inside the trailing container; the distinct dashboard glyph with the sidecar waveform kept; and the icon generator plus the committed ICO binary, which is parsed to confirm every required size is present and PNG-encoded rather than a runtime resize of one bitmap.
- `npm run check` PASS. The built CSS was inspected to confirm the shipped rules, not only the source: `#bootstrap-settings{padding-top:25px;border-top:0}`, `.connection-line-button{…min-height:28px;margin:0 8px;padding:4px 8px…}`, `.sidebar-caption{margin:0 8px 6px…}` and `settings-link,.control-panel-content .button-row{margin-top:0}`.
- `scripts/docs-audit.mjs` PASS after indexing this plan in `docs/README.md` and `docs/architecture/repository-map.md`.
- Full regression deliberately not run: the diff is CSS spacing, JSX ordering, one icon path and regenerated images; Rust and packaging tests do not cover these paths.

## 8.3 Not claimed, and open on Windows

The rendered check is **not** claimed. This Linux host has no browser and no Windows WebView2, so no visual comparison of the sidebar block or the control-panel action group was performed; static assertions are not visual acceptance. The icon image itself was inspected at 128 px and renders cleanly, and a before/after edge measurement at 256 px shows the soft-edge pixel count falling from 36339 to 35651 with higher mean edge energy, which is consistent with the fix but is a proxy, not Windows icon acceptance.

`WQ-UI-030-01`..`05` and `WQ-ICON-030-06` are `WINDOWS_VERIFICATION_PENDING`. `WQ-LOGS-020-02`, the outstanding DPI/narrow/focus matrix, the complete override matrix, System Proxy Batch B, Telegram and the historical Native Host `WINDOWS_FAIL` are all unchanged and remain open.

## 8. Implementation record

Recorded after the Linux implementation and its checks. This section is an implementation record and is not Windows validation evidence.
- In `desktop/scripts/make-icon.py`, draw the plate, the archive box, and the `X` at the supersampled resolution and downscale exactly once, so no large size inherits low-resolution edges.
- Keep the existing green archive-box brand and do not redesign the logo.
- Keep every currently shipped size and regenerate the whole set from the single source so the ICO and the PNG files stay consistent.
- Consider adding the sizes Windows commonly requests for taskbar and Alt+Tab, such as 20 and 40 px, without dropping any existing entry.
- Verify small-size legibility of the `X` mark and edge alignment after regeneration.
- The Windows side must confirm that the icon embedded in the executable, and the icon the running window shows, are the ones just generated.
| 6 | `desktop/scripts/make-icon.py` (`render`) | The green plate is drawn at the target size and only then upscaled by the supersampling factor before the box and `X` are composited and the whole canvas is downscaled once. Large sizes therefore inherit edge softness from a low-resolution plate instead of being drawn at full resolution. |

### 2.1 Item 6 is a quality defect, not a missing-asset defect

The repository already ships every size Windows asks for, verified in this repository:

- `desktop/src-tauri/icons/icon.png` is 512x512;
- `32x32.png`, `128x128.png`, `128x128@2x.png` are present at their declared sizes;
- `icon.ico` contains 16/24/32/48/64/128/256 entries, each stored as its own PNG payload.

Adding one more large PNG would therefore not address the cause. The fix is to draw every primitive on the supersampled canvas and downscale exactly once, so each size is rendered at full resolution, plus a Windows check that the embedded resource is the one actually shown. If the evidence shows the runtime is picking a small bitmap for a large surface, that part is native behavior and belongs to the Windows Platform Owner.

## Windows execution — 2026-10-01 / 01c40db

Fresh optimized Desktop Full assembly PASS and desktop module188/188 after a test-only CRLF extraction fix (28543ed, CROSS_PLATFORM_REVIEW_REQUIRED). Actual packaged WebView2 current-window sidebar/glyph, empty/stopped action gap, Bootstrap divider and settings accessibility order subchecks PASS. Logs default Debug and manual filter retention PASS. Complete DPI/native icon/keyboard/populated/running matrix remains NOT_RUN in the Manual Windows Validation Queue; no release acceptance inferred. Exact evidence, reused component origins and remaining steps: windows-validation-history.md and windows-queue.md latest01c40db section.
