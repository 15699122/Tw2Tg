# Current Platform Handoff

Status: `CURRENT` — 本文件只记录**当前批次**。历史交接记录在 [`platform-handoff-history.md`](platform-handoff-history.md)（2026-09-28 至 2026-10-01，原文归档）。

本文件不保存历史批次。完成一个批次后，将该批次原文追加到 `platform-handoff-history.md`，再在此写入新的当前批次。

## 历史批次速查

| 批次 | 归档位置 |
|---|---|
| Settings 布局与归档目录、UI polish 与 known folder、`v0.2.0` 发布授权、候选冻结、canonical 集成、分支收敛、依赖整改、preN 迁移、集成基线、Sidecar follow-up、日志样式/渠道/图标、`6be3269` 的 Windows 验证、日志过滤器绑定有效等级 | [`platform-handoff-history.md`](platform-handoff-history.md) |

## 当前批次：界面紧凑化、设置页顺序与高清图标（2026-10-01 批次）

- Task: fix six presentation defects reported by the user against the running Windows build — sidebar `服务状态` spacing, the `运行环境` action gap, the separator above `Core Bootstrap`, the position of `网络代理`, the `工作台` icon colliding with the sidecar icon, and a blurry application icon. Plan: [`../development/desktop-ui-density-icon-fix-plan.md`](../development/desktop-ui-density-icon-fix-plan.md).
- Branch: `dev`. Source revision: `c2b754b825e618eade7533e8f1f0ae9455cf1fc3` (the previous batch's handoff). This batch is presentation-only and adds no shared contract.
- Current owner: **Cross-platform -> Windows**. Current state: `READY_FOR_WINDOWS`.
- Uncommitted state: none at the handoff revision.
- Defect evidence: user screenshots of the Windows build. They display `v0.2.0`, but the visible sidecar path contains `windows-6be3269-20261001-prerelease`, so the evidence is from a `6be3269` pre-release rather than the published assets. Windows acceptance must name the actual build revision; a version label alone is not evidence.

### Root causes confirmed in this repository

- The `运行环境` gap was not a spacing typo: `.dashboard-grid .control-panel-content .settings-link` carried `margin-top: auto`, which absorbs all remaining card height, so the gap grew with the job list. Shrinking `gap` alone would not have fixed it.
- The line above `Core Bootstrap` is the generic `.settings-section` `border-top`, not a `Separator`, so it is removed by id only.
- `网络代理` order was purely a JSX position; no state or callback needed to change.
- `工作台` used `icon="activity"`, the same ECG waveform as the not-started sidecar `StatusRow`.
- The blurry icon is **not** a missing-asset defect. The repository already shipped 16/24/32/48/64/128/256 plus a 512 master. The cause was a draw-order defect in `make-icon.py`: the plate was drawn at the target size and then upscaled, so large sizes inherited interpolated edges.

### Cross-platform work completed

Sidebar `服务状态` tightened only (caption `12px→6px`, row `min-height 30→28`, `padding 5→4`, row margin `2→0`); `.nav-item` keeps `min-height: 40px` and global button sizing is unchanged. The `运行环境` settings link follows normal flow at the panel's `16px` gap while the cards stay stretched, so slack falls below the action group. `#bootstrap-settings` drops its border with `padding-top: 25px` compensating the removed `1px`, keeping the title in place and every other divider unchanged. `ProxySettings` moved into the trailing settings container after `日志设置` — moved, not duplicated. A new `dashboard` SVG path serves the `工作台` nav item; the sidecar rows keep `activity`. `make-icon.py` now draws every primitive at the supersampled resolution and downscales exactly once, `SIZES` gained 20 and 40, and the whole asset set was regenerated.

### Linux validation performed

### Known gaps and risks

- **The rendered check was not performed and is not claimed.** This Linux host has no browser and no Windows WebView2, so the sidebar block and the control-panel action group were never compared visually; static assertions are not visual acceptance. Values were chosen from the reported symptoms and may need tuning after a real render.
- Icon clarity on Windows is likewise unproven here. The image was inspected at 128 px and a 256 px before/after edge measurement moved from 36339 to 35651 soft-edge pixels with higher mean edge energy — consistent with the fix, but a proxy, not Windows acceptance. If Windows shows the runtime still using a small bitmap for a large surface, that part is native and belongs to the Windows Owner.
- Windows must distinguish a stale icon cache from an asset defect by using a fresh build path or a new shortcut; "looks fine after clearing the cache" is not sufficient evidence.
- The spacing numbers are a starting point chosen against screenshots, not measured from a live render.

### Windows work and validation required

`WQ-UI-030-01`..`05` and `WQ-ICON-030-06` in [`../validation/windows-queue.md`](../validation/windows-queue.md), all `WINDOWS_VERIFICATION_PENDING`: confirm the tightened service block, the constant action gap in empty/populated/running/stopped states, the missing first-section separator, `网络代理` last with matching Tab order, the distinct dashboard icon, and crisp icons across title bar, taskbar, Alt+Tab, Explorer and tray at 100%/125%/150%/200%. Everything else in the queue stays exactly as recorded, and `WQ-LOGS-020-02` remains open from the previous batch. If GUI automation is unavailable, record `BLOCKED` with `COMPUTER_USE_UNAVAILABLE` and keep the items open.

### Scope boundaries

This batch changes presentation only. It does not change proxy resolution, settings persistence, log levels, sidecar lifecycle, or any Rust command contract; it does not publish a release, modify a GitHub Release, close any acceptance gate, bump version files, or enable the inactive Tauri bundler. No release approval is implied.
`npm test` (desktop) **188/188 PASS**, up from 182 by six new cases covering the sidebar density (with navigation-row and focus-ring guards), the control-panel link gap (with `margin-top: auto` banned), the first-section separator and its compensation, the settings order including the `Aria2Settings` call site, the distinct dashboard glyph, and the icon generator plus the **committed ICO binary** parsed to confirm every required size is PNG-encoded rather than a runtime resize. `npm run check` PASS, with the built CSS inspected so the shipped rules are verified, not only the source. `scripts/docs-audit.mjs` PASS. Full regression deliberately not run: the diff is CSS spacing, JSX ordering, one icon path and regenerated images, which the Rust and packaging tests do not cover.
