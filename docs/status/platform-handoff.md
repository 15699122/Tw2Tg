# Current Platform Handoff

Status: `CURRENT` — 本文件只记录**当前批次**。历史交接记录在 [`platform-handoff-history.md`](platform-handoff-history.md)（2026-09-28 至 2026-10-01，原文归档）。

本文件不保存历史批次。完成一个批次后，将该批次原文追加到 `platform-handoff-history.md`，再在此写入新的当前批次。

## 历史批次速查

| 批次 | 归档位置 |
|---|---|
| Settings 布局与归档目录、UI polish 与 known folder、`v0.2.0` 发布授权、候选冻结、canonical 集成、分支收敛、依赖整改、preN 迁移、集成基线、Sidecar follow-up、日志样式/渠道/图标、`6be3269` 的 Windows 验证、日志过滤器绑定有效等级 | [`platform-handoff-history.md`](platform-handoff-history.md) |

## 当前批次：界面紧凑化／设置顺序／高清图标的 Windows 验证与图标运行时修复（2026-10-01 批次）

- Task: reconcile the Windows validation of `01c40db`, discharge the `CROSS_PLATFORM_REVIEW_REQUIRED` it raised, and prepare the next Windows handoff. Plan: [`../development/desktop-ui-density-icon-fix-plan.md`](../development/desktop-ui-density-icon-fix-plan.md).
- Branch: `dev`. Windows branch `codex/windows-validation-01c40db`, fast-forwarded into `dev`; HEAD is `40b637b`.
- Revisions: Windows product implementation `01c40db`; Windows test-only implementation `28543ed`; Windows-owned runtime icon fix `9cc9d5d`; Owner confirmation record `40b637b`.
- Current owner: **Cross-platform -> Windows**. Current state: `READY_FOR_WINDOWS`.
- Uncommitted state: none at the handoff revision.

### Windows results reconciled

Windows validated a fresh optimized pre-release Full from `01c40db` and passed every current-window subcheck: compact sidebar without clipping, the dashboard glyph distinct from the sidecar waveform, the settings action following the buttons without a stretched gap in the empty/stopped state, the absent Core Bootstrap divider with the next divider retained, the actual settings accessibility order ending in Network Proxy, and the logs page defaulting to `Debug` with a manually chosen `Info` retained across polling. `WQ-LOGS-020-02` is therefore resolved **for that subcheck only**; real child-process module coverage remains `NOT_RUN`.

The Windows runtime icon follow-up (`9cc9d5d`) is a Windows-owned native change and is accepted here. The Owner manually confirmed the taskbar blur resolved on 2026-10-01, which closes that reported defect; the complete 100/125/150/200% and Alt+Tab matrix stays `NOT_RUN`.

### Cross-platform review discharged

`28543ed` was raised as `CROSS_PLATFORM_REVIEW_REQUIRED`. Reviewed and accepted: it makes the icon-generator function extraction tolerate CRLF without weakening any assertion, and it is not a product defect — the first Windows run failed 186/188 on an LF-only blank-line pattern against a CRLF checkout. The fix is now covered on Linux by a permanent case that asserts both endings and that the pattern stays bounded by the first blank line, so the failure cannot return unnoticed.

The runtime root cause Windows reported was verified here against the installed sources rather than accepted on trust: `tauri-codegen 2.6.3` `src/image.rs` `CachedIcon::new_ico` takes `icon_dir.entries()[0]`, and `src/context.rs` selects the `.ico` for Windows targets. Since the multi-size ICO starts at 16 px, the runtime window icon could only ever use the first entry. `tauri::include_image!` and `Context::set_default_window_icon` were confirmed to exist in `tauri 2.11.6`. The diagnosis is correct.

### Linux validation performed

`npm test` (desktop) **189/189 PASS**, up from 188 by the new CRLF case; `cargo fmt --check` and `cargo test -p xarchive-desktop` PASS unchanged, confirming the Windows `cfg(windows)` block has no Linux side effect — it cannot be compiled here, so it is Linux-unverified by construction. Full regression was not run: the cross-platform diff is one test case. None of this is Windows evidence.

### Known gaps and risks

- `cfg(windows)` code is never compiled on Linux. The icon override is validated by Windows builds only; a Linux PASS says nothing about it.
- The Owner confirmation is a cropped screenshot. It binds the artifact through the preceding handoff and the Owner's response, not through a path or hash visible in the image, and it does not establish DPI or Alt+Tab behavior.
- The icon asset fix and the runtime icon fix address different layers. Assets are now full resolution and the runtime no longer uses the 16 px entry, but only Windows can confirm the combined result at every scale.
- Windows replaced this file wholesale and dropped the governed structure above. The facts were preserved and the structure restored here; future Windows batches should append rather than rewrite.

### Windows work and validation required

The outstanding matrix in [`../validation/windows-queue.md`](../validation/windows-queue.md): the 100/125/150/200% DPI sweep across sidebar, settings and every icon surface; populated and running task states for the constant action gap; complete keyboard traversal and narrow-window traversal of the settings order; and real child-process module diagnostics for the logs page. Also still open: the historical Native Host installation `WINDOWS_FAIL`, System Proxy Batch B (`IMPLEMENTATION_NOT_READY`) and Telegram (`OUT OF SCOPE`). See [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md).

### Scope boundaries

This batch (the UI-density / icon validation work) does not by itself publish a release, close a release acceptance gate, or enable the inactive Tauri bundler, and it implies no release approval.

### Pre-release `v0.2.1-pre1` (2026-10-01)

The Owner separately authorized a **restricted** development pre-release ([`../release/release-policy.md`](../release/release-policy.md) §6). `dev` now carries the version bump from `0.2.0` to `0.2.1` (synchronized across `Cargo.toml`/`Cargo.lock`, `tauri.conf.json`, `package.json`/`package-lock.json`, `desktop/package.json`, `extension/manifest.json`/`extension/package.json`, and the `main.jsx` `app_version` placeholder) plus `docs/release/notes/v0.2.1-pre1.md`.

`.github/workflows/pre-release.yml` verifies Linux on the pinned `dev` revision, creates the `v0.2.1-pre1` tag and pre-release object, and dispatches `.github/workflows/windows-release.yml` on that tag for the seven Windows assets. This publishes a **narrower claim**: it does **not** close G4/G5/G7, does **not** promote any unverified capability, and is **not** a `v0.2.1` release approval.

Result: tag `v0.2.1-pre1` → `3115c3b50716be0155804ad4f94dd9d29e37d617`; `pre-release.yml` Run `36837862702` `success`; `windows-release.yml` Run `36838400270` build `success` with all seven assets uploaded and `source_sha` matching the tag target. The independent WDIO job is `FAIL` (`DevToolsActivePort file doesn't exist`, an environment-class failure that reproduces on the `v0.2.0` run) and is explicitly non-blocking. GUI, extension and filesystem/transfer acceptance remain `WINDOWS_BLOCKED` / `NOT_RUN`. The confirmed source SHA, asset state and CI results are recorded in [`../release/release-history.md`](../release/release-history.md) and [`../validation/windows-queue.md`](../validation/windows-queue.md).
