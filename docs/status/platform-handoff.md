# Current Platform Handoff

本文件只描述当前 WebView2/readiness gate batch。历史 validation 保留在
`docs/development/windows-validation.md` 与 `docs/validation/windows-queue.md`。

## Batch

- Task: WebView2 blank-document readiness gate investigation and Windows cleanup fix
- Branch: `windows/webview2-readiness-gate`
- Current owner: Linux Cross-platform Owner（Windows batch 已提交并 push；按 follow-up 规则返回 Linux）
- Current state: `CROSS_PLATFORM_CHANGE_REQUIRED` + `CROSS_PLATFORM_REVIEW_REQUIRED` + `WINDOWS_FAIL`（UI readiness）；manual GUI `BLOCKED`
- Scope source: 用户当前指令明确限定为 WebView2 / readiness gate；本目录未发现可追溯的独立 Plan 文件。

## Revisions

- Cross-platform input revision: `59c8221`
- Cross-platform handoff revision: `59c8221`
- Windows input revision: `59c8221`
- Windows implementation revision: `5a1ecf1`
- Windows validation revision: `5a1ecf1`（validation 使用该提交的代码内容；最终 handoff revision 仅再附带这次 revision 记录）

## Cross-platform Work Completed

- 输入为 GitHub `feature/u7-desktop-production-integration` revision `59c8221`；正式 E: repo 通过 Git checkout 到该 revision，未通过文件同步覆盖正式 repo。
- 用户授权覆盖的 tracked 本地差异已由 remote 文件取代；保留 `.venv`、`node_modules`、`target`、aria2/gallery-dl、本地日志、`manual-validation` 与 `validation-artifacts`。
- 先前已将 E: 中 6 个不在当前 remote revision 的人工源码文件作为用户要求的 carry-forward 纳入此 batch：`crates/xarchive-protocol/src/sidecar.rs`、`desktop/src/main.js`、两个 `shared/protocol-schema/*.schema.json` 和两个 JSONL fixtures。它们并非 WebView2 实现，未被本轮编译/验证，列入 `CROSS_PLATFORM_CHANGE_REQUIRED` 由 Linux Owner 逐项决定整合或移除。

## Windows Work Completed

- 使用 WebView2 Evergreen `153.0.4234.48` 对应的 Microsoft EdgeDriver `153.0.4234.46` 重跑 WQ-P0-WHITE-05A path A；driver 服务确认版本匹配，但应用 target 仍在 30 秒内停留于 `data:,`。因此 driver/runtime 主版本不匹配假设被证伪，WQ-P0-WHITE-01A 仍 `FAIL`。
- direct-startup preflight：release exe 存活 10 秒且 cleanup 可结束其进程树；此项不证明窗口可见或 UI 正常。
- 修复 WDIO launcher teardown 对动态端口的追踪：在 upstream 清空 driver pool 前读取实际 driver/native port pair，捕获监听 PID 并验证 teardown；focused test 10/10，修复后 ordinary gate 完成清理检查。
- Computer Use native app surface 不可用，有限重试后登记 `WQ-MAN-WEBVIEW-01`，状态 `BLOCKED / COMPUTER_USE_UNAVAILABLE`，等待人工直接启动 GUI 并区分 blank app 与 WebDriver attach 问题。

## Validation Required / Results

- `PASS`: Vite build；Tauri Windows release build；Desktop tests 91/91；WDIO service focused tests 10/10；direct-startup preflight 10 秒；session/discovery/failure diagnostics 与非空 logs；修复后 driver/application cleanup。
- `FAIL`: WQ-P0-WHITE-01A ordinary UI readiness；匹配 WebView2 153 的 EdgeDriver session 成功创建，但唯一 target 始终 `data:,`、无 XArchive DOM，0 passed / 1 failed。
- `BLOCKED`: manual native GUI acceptance（`COMPUTER_USE_UNAVAILABLE`）；Dashboard/startup-contract（依赖 01A）；advanced native E2E（依赖有效 ordinary session）。
- `NOT RUN`: hosted/release runner readiness、release archive/manifest/upload；普通 gate 失败时不尝试发布资产。
- 未运行完整 regression：diff 限于 Windows readiness teardown harness、focused test、文档以及用户明确要求携带但未整合的 local-only sources。
- 证据目录：`validation-artifacts/current-20260923-edge153-preflight`、`validation-artifacts/current-20260923-edge153-readiness-elevated`、`validation-artifacts/current-20260923-edge153-after-portfix`。所有目录、依赖、日志与 binaries 留在 E:，不提交。

## Expected Behavior

1. ordinary release executable 创建有效 WebView document target；在 discovery deadline 内 URL 离开 `data:,`，找到 XArchive 根节点和 startup marker。
2. readiness assertions 不因失败被弱化；contract/advanced/release steps 只在普通 UI gate 成功后运行。
3. WDIO teardown 清理 service 实际分配的 port pair 和 driver processes，不论是 preferred 4444/4445 还是 fallback dynamic ports。

## Known Risks / Current Blockers

- 对齐 driver/runtime 后仍保持 blank target。direct process alive 不能区别应用窗口 blank 与 automation target attachment/navigation failure；须先完成 `WQ-MAN-WEBVIEW-01` 人工 GUI 判定，再路由 Windows 或 Cross-platform implementation。
- 首次 gate 曾因 preferred ports 已占用分配到 `61104/61105`；旧 wrapper 只检查 4444/4445，留下该次 driver。新 wrapper 已按实际 pool allocation 追踪，修复后普通 pair teardown 复测通过；dynamic fallback 有单测覆盖。
- Computer Use 工具返回无 native app surface；没法在本轮确认实际窗口、Dashboard、交互性或 manual close。
- `desktop/scripts/wdio-tauri-service.mjs` 是跨平台 Node 测试基础设施；本轮变更保留抽象并只影响按 allocation 的清理范围，要求 Linux Owner 做 `CROSS_PLATFORM_REVIEW_REQUIRED`。
- 附带的 6 个 local-only source files 涉及共享 protocol/schema 且部分为孤立/旧接口。按用户要求上传，但不得直接集成进协议 API；需要 Linux Owner 做 `CROSS_PLATFORM_CHANGE_REQUIRED` review。`sidecar.rs` 当前未被 `crates/xarchive-protocol/src/lib.rs` 引入；`desktop/src/main.js` 是未使用 Sprint 0 placeholder；schema 与 fixtures 本轮未执行 contract validation。

## Manual Windows Validation Queue

`WQ-MAN-WEBVIEW-01`（P0, manual interaction required）已列入 `docs/validation/windows-queue.md`。请人工直接运行 `target/release/xarchive-desktop.exe`，等待 30 秒，记录窗口标题和 Dashboard 可见状态，附截图与 app/Tauri/WebView2 错误日志，关闭并确认进程退出；如果 UI 正常，再使用匹配 WebView2 的 EdgeDriver 重现 WDIO blank target。

## Cross-platform Follow-up

`CROSS_PLATFORM_CHANGE_REQUIRED`:

- Review carried-forward local-only `crates/xarchive-protocol/src/sidecar.rs`、`shared/protocol-schema/archive-request.schema.json`、`archive-status.schema.json` 与两个 JSONL fixtures：确认是否为有效/当前 contract、是否需要与 Rust/Sidecar/Extension/Native Host 同步和补测试；过时项请在 Linux 正式 branch 移除。不得因“文件已上传”宣称协议实现完成。
- 对 `desktop/src/main.js` Sprint 0 placeholder 做保留/删除决定；当前 production entry 是 `desktop/src/main.jsx`，`.js` 文件未被本轮验证。

`CROSS_PLATFORM_REVIEW_REQUIRED`:

- Review `desktop/scripts/wdio-tauri-service.mjs` actual allocated driver-port discovery for upstream `@wdio/tauri-service` compatibility and Linux behavior; focused Windows unit/gate evidence is recorded above.

## Next Owner

Owner: Linux Cross-platform Owner（因上述 shared follow-up；Windows 本批状态和验证证据已 push）

Required actions:

1. 读取本分支 commit 与本 handoff，审查 `CROSS_PLATFORM_REVIEW_REQUIRED` teardown patch。
2. 对 local-only protocol/schema carry-forward 逐项决定集成或移除；如集成，同步所有 protocol consumers、schema、fixture、tests 与 repository map，并执行 shared validation。
3. 等 `WQ-MAN-WEBVIEW-01` 结果后，按证据将 blank target 归为 Windows app startup 或 cross-platform WebDriver harness；保留 WQ-P0-WHITE-01A assertions。
4. 只有 shared follow-up 完成并再次形成 Windows batch 时，才把 ownership 回交 Windows 复验。
