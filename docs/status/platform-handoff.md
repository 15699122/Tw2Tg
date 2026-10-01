# Current Platform Handoff

Status: `CURRENT` — 本文件只记录**当前批次**。历史交接记录在 [`platform-handoff-history.md`](platform-handoff-history.md)（2026-09-28 至 2026-10-01，原文归档）。

本文件不保存历史批次。完成一个批次后，将该批次原文追加到 `platform-handoff-history.md`，再在此写入新的当前批次。

## 历史批次速查

| 批次 | 归档位置 |
|---|---|
| Settings 布局与归档目录、UI polish 与 known folder、`v0.2.0` 发布授权、候选冻结、canonical 集成、分支收敛、依赖整改、preN 迁移、集成基线、Sidecar follow-up、日志样式/渠道/图标、`6be3269` 的 Windows 验证 | [`platform-handoff-history.md`](platform-handoff-history.md) |

## 当前批次：日志显示过滤器绑定后端有效等级（2026-10-01 批次）

- Task: reconcile the Windows validation of `6be3269` and clear the single `CROSS_PLATFORM_CHANGE_REQUIRED` it raised. Windows confirmed the backend half of the log/channel work and found the Logs page filtering away the diagnostics that channel exists to produce.
- Branch: `dev`. Windows input/source: `6be3269` (includes implementation `279d726`). Windows validation revision: `a56ff59` on `codex/windows-validation-6be3269`, fast-forwarded into `dev` before this batch. Plan: [`../development/desktop-logs-release-icon-fix-plan.md`](../development/desktop-logs-release-icon-fix-plan.md) §9–10.
- Current owner: **Cross-platform -> Windows**. Current state: `READY_FOR_WINDOWS`. This batch changes only frontend source and adds no shared contract, so Windows must revalidate this new revision rather than `6be3269`.
- Uncommitted state: none at the handoff revision.

### Reconciliation of the Windows results at `6be3269`

- Windows revalidated a real optimized pre-release Full and a separate stable-channel EXE from that source. Backend channel defaults behaved as designed: pre-release startup reported `channel=prerelease effective_level=debug`, the stable build `channel=release effective_level=info`, and the pre-release Error override survived save and restart. Desktop 179/179, Python proxy 8/8 and the changed Rust modules 229/229 PASS. The first Rust run failed three discovery fixtures because the default `python3` gave no working worker; the existing `PYTHON` override fixed the environment and the rerun passed, which is a tooling/environment result, not a product one, and both runs are retained.
- `WQ-LOGS-020-02` **FAIL**, the only `CROSS_PLATFORM_CHANGE_REQUIRED`. `LogsPage` hardcoded `useState("info")` while the backend effective level was `debug`, so the page hid the module diagnostics the pre-release was built to produce. `CROSS_PLATFORM_CHANGE_REQUIRED` is now **discharged on Linux**; the item stays open pending Windows revalidation, because Linux cannot observe the running Windows page.
- The remaining items stay exactly as Windows recorded them: multi-DPI/narrow/focus matrix, all icon surfaces, the complete override matrix, real child-process module diagnostics, real progress/warning/error coverage, the proxy real-boundary checks, System Proxy Batch B (`PLANNED`, `IMPLEMENTATION_NOT_READY`), Telegram `OUT OF SCOPE`, and the historical Native Host installation `WINDOWS_FAIL` that is still unclosed. `WQ-PROXY-020-12` remains a compilation PASS only.

### Cross-platform work completed

- `LogsPage` accepts `loggingLevel` and initializes the `最低等级` filter with the new `displayLogLevel()` instead of a literal `"info"`. `main.jsx` passes `status.logging_level`, which is `LoggingConfig::effective_level()`, so the page now uses the same single accessor as the runtime and the status command rather than a second, divergent default.
- `effectiveLevelChanged()` gates resynchronization against the last observed backend level. The page polls once per second, so re-applying the backend value unconditionally would erase a level the user selected on that page; a manual selection now survives until the backend level genuinely changes, for example after Settings saves.
- `displayLogLevel()` falls back to `info` for a missing or unrecognized level so the select can never render outside its own `LOG_LEVEL_OPTIONS`, and reuses `normalizeLogLevel` so `warn` and `warning` are the same level.
- Three new cases in `desktop/test/ui-wiring.test.mjs` cover level mapping and fallback, the change-detection guard, and the actual visibility of pre-release module diagnostics under a `debug` filter. Static assertions pin the page state and its initialization so the hardcoded `info` cannot return.

### Linux validation performed

`npm test` (desktop) 182/182 PASS, up from 179 by the three new cases; `npm run check` (Vite production build) PASS; `cargo fmt --check` PASS; `cargo test -p xarchive-desktop` 162/162 PASS unchanged, confirming no Rust side effect. The batch's CSS spacing and themed-checkbox assertions still pass. Full workspace regression was not run: this batch touches one page, one pure helper, and one wiring call, none of which the Rust or packaging tests cover. None of these results say anything about Windows behavior.

### Known gaps and risks

- The fix is Linux-verified only. The defect was proven against a real Windows pre-release, so a passing Linux assertion is not evidence that the Windows page now renders `Debug`; that requires the Windows rerun.
- The resynchronization rule is deliberately narrow: a level saved in Settings resyncs the page through the fresh `AppStatus`, but a level changed by any other path would not until the page remounts.
- `displayLogLevel()` falls back to `info` on an unrecognized value. That fallback is only correct while the backend keeps reporting one of the five known levels.
- `detect_aria2` still has no runtime state, so aria2 detection remains outside the shared diagnostic helper; unchanged by this batch.

### Windows work and validation required

`WQ-LOGS-020-02` is the focused revalidation: on a fresh optimized pre-release built from the handoff revision, the `最低等级` select must read `Debug` on load with the runtime/network/transport/executor debug lines visible, and changing the level in Settings must be reflected after the page resynchronizes. Everything else in [`../validation/windows-queue.md`](../validation/windows-queue.md) and [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md) remains as Windows recorded it, and the historical PASS items unrelated to the logs page stay valid. If GUI automation is unavailable, record `BLOCKED` with `COMPUTER_USE_UNAVAILABLE` and keep the items open.

### Scope boundaries

This batch does not publish a release, does not modify an existing GitHub Release, does not close any `v0.2.0` acceptance gate, does not bump version files, and does not enable the inactive Tauri bundler. No release approval is implied.

