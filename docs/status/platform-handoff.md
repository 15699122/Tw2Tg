# Platform Handoff（Windows → Cross-platform）

Windows Validation Queue 的唯一事实源仍是 [`../validation/windows-queue.md`](../validation/windows-queue.md)；平台验证流程见 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md)。

## 当前状态

- Branch：`security/tweet-url-host-validation`
- Implementation revision：`7b218f8a5ff4a10590cea3cf762fd30a82d6c6c9`
- Validation revision：`cd04f269fca55d92f1a2336df6fbd8e9911d7dbf`（验证结果首次写入 validation 文档的 commit）。
- 状态：`CROSS_PLATFORM_RECONCILE_REQUIRED`
- 工作副本：Windows 正式 E: checkout；本轮只写入验证与 handoff 文档，未改实现源码。

## Reconcile 所需工作

- `CROSS_PLATFORM_CHANGE_REQUIRED`：修复 Windows Junction 暴露的共享 metadata path containment 缺陷；修复 clean `npm ci` 后的 Node override/lock dependency contract；调查 Tauri WebDriver 155 会话停在 `data:,` 的 native startup 问题。
- `CROSS_PLATFORM_REVIEW_REQUIRED`：检查 `desktop/scripts/wdio-tauri-service.mjs` 的 Windows `killTree` 与测试退出证据契约。Windows 全量 Node 测试中该断言失败且测试进程未及时退出。
- Windows PASS/FAIL/BLOCKED 的精确证据、命令、package/driver 版本与手工队列见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 2026-09-27 phase section 和 [`../validation/windows-queue.md`](../validation/windows-queue.md) 的 phase result。

## 下一 Owner

Cross-platform Owner 先完成上述 reconcile 并更新 implementation/handoff revision；随后仅把命中改动的条目交回 Windows 做 focused revalidation。未完成的真实账号、GUI、Named Pipe、workflow 和 archive extraction 项继续保留在 Manual Windows Validation Queue。

## 同步方式

沿 Git 将验证文档写回 source branch；不通过直接文件同步覆盖正式 Windows repo。Windows phase 使用 `git fetch` 后验证 exact handoff revision `7b218f8a5ff4a10590cea3cf762fd30a82d6c6c9`。

## 2026-09-28 交接：Cross-platform reconcile 第 1 轮（状态确认 + 根因定位）

本节为收尾会话写入；只做仓库状态核对、定向验证和根因定位，未修改实现源码、依赖或测试。上方 Reconcile 清单与状态 `CROSS_PLATFORM_RECONCILE_REQUIRED` 保持有效。

### 源状态（本会话核对）

- Branch：`security/tweet-url-host-validation`；HEAD：`84af1ef18d964a1f05e096424bd67b9de78a89e1`（`docs: hand off Windows validation findings`）。
- Working tree clean；与 `origin/security/tweet-url-host-validation` 同步（0 ahead / 0 behind）；PR #5（→ `dev`）保持 OPEN。
- Implementation revision 仍为 `7b218f8`（Windows phase 验证对象）；reconcile 修复尚未开始。

### 本轮验证结果

| 检查 | 状态 | 结果 / 原因 |
|---|---|---|
| `npm run test --workspace desktop` | `PASS` | 42/42；含 `killTree → terminates a spawned child process`（Linux ~4 ms；Windows FAIL 未在 Linux 复现） |
| `cargo test -p xarchive-storage` | `PASS` | 29/29 |
| `npm ls --all`（当前已安装树） | `PASS` | exit 0，0 invalid；Windows cleanroom `npm ci` 后的 `ELSPROBLEMS` 在本机已安装树不复现 |
| cleanroom `npm ci` + `npm ls` 隔离复现 | `NOT RUN` | 本轮未搭建隔离 cleanroom；WQ-ENG-09 依赖契约复现属下一轮首要动作 |
| 全 workspace 回归（Rust 全量 / `npm run check`+`test` / Sidecar pytest） | `NOT RUN` | 本轮无实现改动，留给修复后的收口轮次 |
| Windows 侧行为 | `BLOCKED` | 本会话在 Linux 环境；WQ-ENG-03/08/09 的 Windows 重验须待共享修复完成后按 focused revalidation 执行 |

### 根因定位（第 1 轮调查结论）

- **WQ-ENG-03 metadata junction（`CROSS_PLATFORM_CHANGE_REQUIRED`）**：`crates/xarchive-storage/src/metadata.rs` 的 `build_archive_metadata` 只做 `relative` 词法检查（拒绝 `ParentDir`/`RootDir`/`Prefix`）并对最终路径做 `fs::symlink_metadata` reparse 检查；中间组件为指向根外的 junction/symlink 时会被路径解析跟随（Linux 可用中间 symlink 复现同一缺口）。共享的逐组件 containment 已存在于 `FileStore::resolve_within`（`crates/xarchive-storage/src/file_store.rs:147`），但该关联函数为模块私有、且 metadata 路径未复用。修复方向：提升可见性或提供 `pub(crate)` 包装，让 metadata 构建复用逐组件解析并补测试；完成后在 Windows 重跑真实 junction probe。
- **WQ-ENG-08 killTree（`CROSS_PLATFORM_REVIEW_REQUIRED`）**：`desktop/scripts/wdio-tauri-service.mjs` 的 `killTree()` win32 分支为 `taskkill /pid … /T /F`；`desktop/test/wdio-tauri-service.test.mjs:72` 已按 2026-09-16 结论在 killTree 前挂 `exit`/`close` 监听并以 10 s 窗口断言。Linux 本轮 42/42 通过；Windows 2026-09-27 失败（10 s 内未观察到终止、测试进程需人工中断）未在 Linux 复现，属 Windows 侧时序/进程生命周期问题，根因未定位。
- **WQ-ENG-09 依赖契约（`CROSS_PLATFORM_CHANGE_REQUIRED`）**：根 `package.json` 声明 `overrides`（`serialize-javascript: ^7.1.2`、`deepmerge-ts: ^8.0.2`），但 `package-lock.json` 根条目 `packages[""]` 只有 `name/version/workspaces`，**不含 `overrides`**（lockfileVersion 3；最近一次 lock 改动 `f9c088b`）。Windows cleanroom `npm ci` 后 `npm ls` 报 `serialize-javascript@7.1.2` 违反 Mocha `^6.0.2`、根 `deepmerge-ts@8.0.2` 违反嵌套 `@wdio/tauri-service 9.30.1` 的 `^7.0.3`。本机已安装树 `npm ls` 干净、隔离复现未做；修约方案（补记 overrides 进 lock / 调整 override 范围 / 其他）未定。
- **WQ-ENG-09 native E2E `data:,`**：`desktop/e2e/test-artifacts/wdio/startup/evidence.json` 是被 Git 跟踪的文件，由 `6555d35`（2026-09-27 01:11 +0800）加入且提交内容已是 `{"url": "data:,", "rootExists": false}`。当前分支没有任何生产者：全仓仅该文件本身含 `rootExists`；`desktop/e2e/support/native-startup.mjs` 只存在于 `feature/u7-desktop-production-integration`（`382258c`）与 `windows/webview2-readiness-gate`，不是 HEAD 祖先。推论：[`../development/windows-validation.md`](../development/windows-validation.md) 中 “current-run evidence.json” 可能引用已提交的陈旧文件而非 2026-09-27 run 的真实产出；triage 必须先核实证据来源，再查会话停在 `data:,` 的启动问题。文件处置决策（untrack + gitignore，或恢复生产者）未做。

### 未完成工作分类

- **可由下一 Task 立即继续**：WQ-ENG-03 metadata 修复与测试（Linux 可用中间 symlink 复现）；WQ-ENG-09 依赖契约隔离复现与 lock/override 修约；WQ-ENG-09 E2E 证据来源核实 + 启动 triage；WQ-ENG-08 依据 Windows 失败证据的代码级复核。
- **需要外部信息/环境**：Windows 侧 focused revalidation（须在共享修复完成、记录新 implementation revision 后）；Windows E: 工作副本上的原始失败日志。
- **需要人工验证**：真实账号、GUI/WebView2/辅助技术、Named Pipe、CI workflow run 等原 Manual Windows Validation Queue 项（状态不变）。
- **暂不应继续**：在未确认生产者前改写 `windows-validation.md` 的历史结论；不先隔离复现就改 overrides。

### 关键上下文（供下一 Task）

- 关键文件：`crates/xarchive-storage/src/metadata.rs`、`crates/xarchive-storage/src/file_store.rs`（`resolve_within`、`is_reparse_point`）、`desktop/scripts/wdio-tauri-service.mjs`（`killTree`/`waitForProcessGone`/`isPidAlive`）、`desktop/test/wdio-tauri-service.test.mjs`、根 `package.json` `overrides`、`package-lock.json`、`desktop/wdio.conf.mjs`、`docs/development/windows-validation.md:4378-4385`、`docs/validation/windows-queue.md:595-614`。
- 设计约束：AGENTS.md 跨平台工作流（Linux 先完成开发与验证，再集中进入 Windows phase）；Windows Validation Queue 唯一事实源为 `windows-queue.md`；`BLOCKED_AUTOMATION` 不得记 PASS；不得为过测试改业务行为。
- 不应重复的失败路径：不要用 Linux `npm ls` 干净结果替代 cleanroom 复现；不要在未确认生产者前把 `evidence.json` 当作 2026-09-27 run 证据；不要假设 `native-startup.mjs` 在当前分支存在。
- 未解决错误：Windows cleanroom `npm ls` `ELSPROBLEMS`（未复现）；Windows `killTree` 10 s 断言失败（未复现）；E2E 会话停在 `data:,`（未复现，无本地 producer）。
