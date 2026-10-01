# Current Platform Handoff

Status: `CURRENT` — 本文件只记录**当前批次**。历史交接记录在 [`platform-handoff-history.md`](platform-handoff-history.md)（2026-09-28 至 2026-09-30，原文归档）。

本文件不保存历史批次。完成一个批次后，将该批次原文追加到 `platform-handoff-history.md`，再在此写入新的当前批次。

## 历史批次速查

| 批次 | 归档位置 |
|---|---|
| Settings 布局与归档目录、UI polish 与 known folder、`v0.2.0` 发布授权、候选冻结、canonical 集成、分支收敛、依赖整改、preN 迁移、集成基线、Sidecar follow-up | [`platform-handoff-history.md`](platform-handoff-history.md) |

## 当前批次：日志样式 / 渠道日志策略 / 应用图标（2026-10-01 批次）

- Task: fix four defects reported against the released `v0.2.0` Windows build — the runtime log panel sitting too close to the page description, the blue checked state of the `自动跟随` checkbox, a default log level tied to the Rust build profile instead of the release channel, and an `xarchive-desktop.exe` icon that renders as a plain green block.
- Branch: `dev`. Source commit: `ba395ce` (the Batch A system-proxy handoff tip). **Cross-platform handoff revision: `279d726`** (implementation commit). Plan: [`../development/desktop-logs-release-icon-fix-plan.md`](../development/desktop-logs-release-icon-fix-plan.md). The screenshots are defect evidence for the shipped build only; they are not acceptance for the fix.
- Current owner: **Cross-platform -> Windows**. Uncommitted state: this documentation record only; the working tree was clean at the implementation commit.
- Current state: `READY_FOR_WINDOWS`. Windows must validate the exact revision `279d726`, not the documentation commit that follows it.
- **Pairing token: rotated by the Owner on 2026-10-01.** Screenshots taken before that date that contain the full token are void and must not be cited as evidence. The rotation was confirmed by the Owner; Linux neither performed nor independently verified it, so this records the decision rather than an acceptance result.
- Executable steps for every outstanding Windows item are consolidated in [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md), grouped A–F with a checklist per item. Results are recorded back in [`../validation/windows-queue.md`](../validation/windows-queue.md).

### Owner decisions recorded 2026-10-01

- **Next target: `v0.2.1-pre1`**, then the `v0.2.1` release. The pre-release workflow defaults now point at `v0.2.1-pre1` with `source_ref: dev`. Version *files* (`package.json`, `desktop/package.json`, `extension/manifest.json`, `desktop/src-tauri/tauri.conf.json`) are deliberately **not** bumped in this documentation commit: that is a product change which must be validated together with the release, and it affects the migration test.
- **Telegram development is paused** and is out of the advertised scope for `v0.2.1-pre1`. G6/M-CAND-03 is recorded as out of scope rather than `NOT_RUN`; the distinction matters because a scope decision is neither a pass nor a failure. Existing Telegram code is retained, not removed. The root `README.md` previously described Telegram as a usable capability and has been corrected.
- **Still open:** the reduced-scope authorization and whether to fast-forward `main`. G4, G5 and G7 remain unclosed, so there is no release approval yet.

### Confirmed causes

- `desktop/src-tauri/src/config.rs` derived the default log level from `debug_assertions`, and `windows-release.yml` always builds optimized, so a pre-release still defaulted to `info`.
- `desktop/src/style.css` had no rule placing space between the page description and the log panel, and no checked-state color, so the checkbox kept the WebView2 blue accent.
- `file` reported `desktop/src-tauri/icons/icon.ico` and `icon.png` as 1x1 images, which is the cause of the green block.
- The log page `level` state is a display filter over already-written lines, not the backend level; changing it alone would not have satisfied the requirement.

### Cross-platform work completed

- New `desktop/src-tauri/src/build_channel.rs` resolves a compile-time channel, and `LoggingConfig.level` is now `Option<LogLevel>`: `None` follows the channel, `Some` is an explicit user choice that survives an upgrade. `effective_level()` is the single accessor.
- New `desktop/scripts/build-tauri.mjs` injects the channel before `tauri build`; `build:tauri` routes through it, and `windows-release.yml` exports the already validated `RELEASE_TAG`. An unknown channel fails rather than defaulting.
- `RuntimeState::record`/`debug`/`warn`/`error` form the single diagnostic write path, with coverage for runtime, config, database, network, transport, executor, Sidecar, storage, and logging. Secret redaction, rotation, and size caps still apply.
- `desktop/src/style.css` adds a page-scoped `.logs-page-panel` spacing rule and a themed checkbox; `desktop/scripts/make-icon.py` generates a multi-size icon and the assets replace the 1x1 placeholders.
- Startup logs the channel, channel default, effective level, and whether a user override is present, so a verbose pre-release is explainable.

### Linux validation performed

`cargo fmt --check` PASS; `cargo clippy --workspace --all-targets` PASS with no warnings; `cargo test --workspace` PASS (all targets); `cargo test -p xarchive-desktop` 162/162 PASS; `npm test` (desktop) 179/179 PASS; `npm run check` (Vite production build) PASS. The channel was exercised by compiling under `prerelease`, `release`, an unset variable, and an invalid value. These results say nothing about Windows behavior.

### Ownership routing

- Cross-platform (Linux): log page spacing and checkbox styling, the channel-driven log default and its configuration migration, module debug coverage, the icon artwork and Tauri icon declaration, and the release workflow channel derivation.
- Windows: GUI appearance at three scaling factors, the real optimized pre-release and release binaries and their actual log output, real child-process diagnostics, and the icon on the built executable, in Explorer, the title bar, and the task bar.

### Known gaps and risks

- `detect_aria2` receives only an `AppHandle` and has no runtime state, so aria2 detection is not yet covered by the shared diagnostic helper.
- The Linux CSS and script assertions are static checks and are not visual or artifact acceptance.
- Module debug coverage in the Sidecar, aria2, and Telegram paths still depends on the Windows build to demonstrate real output.

### Windows work and validation required

Queue entries `WQ-LOGS-020-01` .. `WQ-LOGS-020-05` in [`../validation/windows-queue.md`](../validation/windows-queue.md), all `WINDOWS_VERIFICATION_PENDING`. If GUI automation is unavailable, record `BLOCKED` with `COMPUTER_USE_UNAVAILABLE` and keep the items open.

### Scope boundaries

This batch does not publish a release, does not modify an existing GitHub Release, does not close any `v0.2.0` acceptance gate, and does not enable the inactive Tauri bundler.
