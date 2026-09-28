# Platform Handoff（Windows → Cross-platform）

Windows Validation Queue 的唯一事实源仍是 [`../validation/windows-queue.md`](../validation/windows-queue.md)；平台验证流程见 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md)。

## 当前状态

- Branch：`security/tweet-url-host-validation`
- Implementation revision：`01067b66a3aa214fb90f7d893c57bb971a7c7882`（reconcile 第 2 轮修复；Windows phase 已验证对象仍为 `7b218f8a5ff4a10590cea3cf762fd30a82d6c6c9`）
- Validation revision：`cd04f269fca55d92f1a2336df6fbd8e9911d7dbf`（验证结果首次写入 validation 文档的 commit）。
- 状态：`FOCUSED_WINDOWS_REVALIDATION_REQUIRED`（原 `CROSS_PLATFORM_RECONCILE_REQUIRED` 的四项 reconcile 已在第 2 轮完成，见下）
- 工作副本：Windows 正式 E: checkout；第 2 轮在 Linux source 完成实现，Windows 侧仅执行 focused revalidation。

## Reconcile 所需工作

- `CROSS_PLATFORM_CHANGE_REQUIRED`（第 2 轮已全部完成）：Windows Junction 暴露的共享 metadata path containment 缺陷已修复（`01067b6`）；clean `npm ci` 后的 Node override/lock dependency contract 已复现定位为 npm ≤ 11.17 的 `npm ls` 行为并以 `engines.npm >=11.18.0` 修约（`01067b6`）；Tauri WebDriver 会话停在 `data:,` 的 evidence 来源已核实（陈旧提交产物，已 untrack）并把启动 triage 所需的证据采集入队。
- `CROSS_PLATFORM_REVIEW_REQUIRED`（第 2 轮已完成复核）：`killTree`/测试契约复核未发现 Linux 可见缺陷；win32 分支补 taskkill 诊断（布尔契约不变），Windows 重验入队，不记 PASS。
- Windows PASS/FAIL/BLOCKED 的精确证据、命令、package/driver 版本与手工队列见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 2026-09-27 phase section 和 [`../validation/windows-queue.md`](../validation/windows-queue.md) 的 phase result；第 2 轮 focused revalidation 清单见 windows-queue.md 的 “Cross-platform reconcile round 2”。

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

## 2026-09-28 交接：Cross-platform reconcile 第 2 轮（实现轮）

本节为收尾会话写入；完成第 1 轮列出的全部四项 reconcile 工作，含实现改动。上方状态与 Reconcile 清单已同步更新为 `FOCUSED_WINDOWS_REVALIDATION_REQUIRED`。

### 源状态（本会话核对与产出）

- Branch：`security/tweet-url-host-validation`；起点 HEAD：`8d81edd`（round-1 handoff，working tree clean）。
- 实现 commit：`01067b66a3aa214fb90f7d893c57bb971a7c7882`（`fix: close metadata intermediate-link escape; reconcile WQ-ENG-08/09`）；本 handoff 与队列/验证/设置文档记录在随后的 docs commit。
- 变更文件（仅 8 个实现 + 4 个文档）：`crates/xarchive-storage/{metadata.rs,file_store.rs,lib.rs}`、`desktop/scripts/wdio-tauri-service.mjs`、`.gitignore`、`package.json`、`package-lock.json`、删除 `desktop/e2e/test-artifacts/wdio/startup/evidence.json`；`docs/{development/setup.md,development/windows-validation.md,validation/windows-queue.md,status/platform-handoff.md}`。

### 各项结论

| 项 | 结论 | 证据 / 关键事实 |
|---|---|---|
| WQ-ENG-03 metadata 中间 junction/symlink 逃逸 | 已修复 | 先写 Linux 复现测试 `rejects_sidecar_intermediate_symlink_escape`（中间 symlink 指向根外）确认修复前 FAIL（根外文件被算入 metadata sha256）；`FileStore::resolve_within` 提为 `pub(crate)`，`build_archive_metadata` 放弃纯词法检查改为复用逐组件 containment；修复后 storage 30/30 |
| WQ-ENG-09a 依赖契约 | 已复现并完成最小修约（未动 overrides/lock 内容） | 隔离 cleanroom：npm 11.17.0（Windows 同版本）`npm ci` exit 0、`npm ls --all` exit 1，invalid 明细与 Windows 完全一致；同树版本二分 11.17 FAIL / 11.18 PASS / 11.19 PASS；`npm install --package-lock-only` 零 diff、手工补写 lock 根条目 `overrides` 后 11.17 仍 FAIL——“补记 overrides 进 lock” 两个候选均被证伪。修约：根 `engines.npm >=11.18.0` + lock 根条目由 npm 同步（仅 +3 行）+ setup.md 版本底线；11.17 对该 manifest 报 `npm warn EBADENGINE`（信号已验证）。engines 方案另在 cleanroom 全链验证：11.18 `npm ci` exit 0、`npm ls --all` exit 0 |
| WQ-ENG-09b evidence 来源与 `data:,` triage | 来源核实完成 + 文件处置完成；启动 triage 已入队（需 Windows 采集） | `git log --follow`：evidence.json 唯一提交为 `6555d35`（2026-09-27 01:11 +0800，提交内容已是 `{"url":"data:,","rootExists":false}`）；`git log --all -S`：写入方 `desktop/e2e/support/native-startup.mjs` 只在 `382258c`/`windows/webview2-readiness-gate`，`merge-base --is-ancestor` 确认非 HEAD 祖先；本分支 `wdio.conf.mjs` 日志目录为 `desktop/test-artifacts/wdio`（已 ignore），无任何代码写 startup/evidence.json。处置：untrack 该文件 + `desktop/e2e/test-artifacts/` 加入 `.gitignore`（不恢复非祖先分支的生产者）；`windows-validation.md` 4383 行 “current-run evidence.json” 引用已发勘误。`data:,` triage 结论：会话 URL 停在初始空文档、20 s 内无 `h1`；当前分支 spec 无 readiness gate（窗口 target 切换/空白文档处理只存在于非祖先分支）；三类假设（应用未导航 / 会话附着空白窗口 / 应用启动失败）的区分所需证据（window handles+URL/title 枚举、截图、`WDIO_LOG_DIR` service 日志、`desktop/logs` 应用日志、单独启动 binary）已写入队列 focused 步骤 |
| WQ-ENG-08 killTree/测试契约复核 | 复核完成；补诊断；不记 PASS | 契约检查：测试已按 2026-09-16 结论在 `killTree` 前挂 `exit`/`close` 监听（`test.mjs:84-90`），win32 分支为 `taskkill /pid … /T /F`；失败证据语义：10 s 窗口跑完 ⇒ `await killTree` 已返回（taskkill 未挂起）而 victim 未退出，随后存留的 child 阻断测试进程退出——布尔返回值无法区分 spawn 失败 / 非零退出 / 杀后存活。改动：win32 分支捕获 taskkill stderr、对 spawn 失败与非零退出 `console.warn`（含 exit code/stderr），resolve 契约与断言不变。Linux 42/42 通过仅作回归，不替代 Windows 结论 |

### Linux 验证（全部 PASS）

| 检查 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test -p xarchive-storage` | PASS 30/30（含新增复现测试） |
| `cargo test --workspace --locked` | PASS 190/190 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `npm run check` / `npm run test`（root，desktop 42/42 + extension 13/13）/ `npm run build` | PASS |
| `npm ls --all`（本机 npm 11.19.0，含 engines manifest） | PASS exit 0 |
| cleanroom 复现（npm 11.17.0 `npm ci`+`npm ls`；11.18 engines 方案全链） | FAIL(11.17 ls，按预期复现) / PASS(11.18) |
| `node --check desktop/scripts/wdio-tauri-service.mjs`、`git diff --check` | PASS |
| Windows 侧行为 | `BLOCKED`（本会话在 Linux；见队列 focused 项） |

### Windows 队列累计

- 新增 focused revalidation 4 项（WQ-ENG-03 / WQ-ENG-08 / WQ-ENG-09a / WQ-ENG-09b），全部 `WINDOWS_VERIFICATION_PENDING`，记录于 [`../validation/windows-queue.md`](../validation/windows-queue.md) 的 “Cross-platform reconcile round 2”；本轮无 `WINDOWS_VERIFICATION_BLOCKING`。
- 原 Manual Windows Validation Queue（真实账号、GUI/WebView2、Named Pipe、workflow、archive extraction 等）与独立 PASS 复用结论不变。
- 按 AGENTS.md 批量开发、集中验证策略，本轮不在 Linux 阶段切换 Windows；进入 Windows phase 前按最终 diff 统一合并重复场景（队列 focused 表即本轮 handoff 清单）。

### 剩余工作分类

- **下一 Windows phase 立即执行**：队列 round-2 focused 四项（含 npm ≥ 11.18 升级前置、killTree 诊断采集、E2E window-handle 证据采集）。
- **需要人工验证**：原 Manual Windows Validation Queue 项（状态不变）。
- **暂不应继续**：不把本轮 Linux 结果记为 Windows PASS；不再引用被跟踪 evidence.json 作为任何 run 的证据；不在未采集 fresh 证据前改写 4383 行的 session 观察记录（勘误仅针对 “current-run evidence.json” 引用）。
