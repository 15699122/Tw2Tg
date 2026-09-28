# Platform Handoff（Windows → Cross-platform）

Windows Validation Queue 的唯一事实源仍是 [`../validation/windows-queue.md`](../validation/windows-queue.md)；平台验证流程见 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md)。

## 当前状态

- Branch：`security/tweet-url-host-validation`
- Implementation revision：`e856343ee03e44134f2189cba7e66dc83e00f3a0`（本轮 WQ-ENG-09b triage 的测试层改动）
- Last Windows-validated implementation：`01067b66a3aa214fb90f7d893c57bb971a7c7882`（2026-09-28 focused phase 的验证对象）
- Validation record revision：`517016146e29e08cae38c8e33cfdb85c4b7e08`（Windows round-2 focused 结果写回 commit）
- 状态：`BLOCKED_AUTOMATION_UPSTREAM`（WQ-ENG-09b 产品侧已由 Windows batch 2 结清为 `WINDOWS_PASS`；ordinary/advanced E2E 仍被上游 `tauri-driver` 缺陷阻塞）
- 工作副本：Windows 正式 E: checkout；本轮全部改动在 Linux source 完成，未反向同步任何 Windows 工作副本代码。

## Reconcile 所需工作

- `CROSS_PLATFORM_CHANGE_REQUIRED`：**新增一项（本轮收尾发现，见下方 “跨会话交接总结”）**——`desktop/test/startup-diagnostics.test.mjs` 的 `resolveDiagnosticsDir` 断言在 Windows 失败（`\project\...` vs `E:\project\...`）。其余三项已全部关闭：metadata path containment（WQ-ENG-03，`WINDOWS_PASS`）、Node override/lock 依赖契约（WQ-ENG-09a，`WINDOWS_PASS`）、E2E evidence 来源核实（证据为陈旧提交产物，已 untrack）。
- `CROSS_PLATFORM_REVIEW_REQUIRED`：`killTree`/测试契约复核已完成，Windows 侧确认 `WINDOWS_PASS`（42/42；此前失败定位为验证沙箱 `taskkill` 权限，诊断信息由本轮改动提供）。另：`wdio.conf.mjs` / WDIO 版本层面的 workaround 尚未决定，若决定则属 shared implementation change。
- Windows PASS/FAIL/BLOCKED 的精确证据、命令、package/driver 版本与手工队列见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 2026-09-28 focused phase；队列状态以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准，round-3 focused 表是本轮交回 Windows 的执行清单。

## 下一 Owner

Windows Owner 已完成 queue round-3 的 WQ-ENG-09b revalidation，**结论与 round-3 预期不同**：产品侧无缺陷，失败根因在本仓库之外。

- 独立启动 `target\release\xarchive-desktop.exe`：进程存活，Win32 `EnumWindows` 枚举到可见顶层窗口 `TITLE='XArchive'`（`RECT=147,5,1240,801`），`target\release\logs\xarchive-*.log` 写入 `application runtime initialized`。
- 按 round-3 用 `TAURI_DRIVER_EDGE_VERSION`/`EDGEDRIVER_VERSION=153.0.4234.46` 钉版重跑 `npm run test:e2e:windows`：**仍然失败**，错误为 `session not created: This version of Microsoft Edge WebDriver only supports Microsoft Edge version 153 / Current browser version is 155.0.4283.18 with binary path ...\msedge.exe`；`xarchive-desktop` 从未启动。
- 绕过 tauri-driver 直连 `msedgedriver --port=4445`（W3C `alwaysMatch` 内联 `browserName=webview2` + `ms:edgeOptions.binary` + `webviewOptions`）：session 200，应用启动（`goog:processID` = 应用 PID），`url=http://tauri.localhost/`、`title=XArchive`、`h1=工作台`，并取得 211 949-byte 真实 dashboard 截图。

即：被记录为 “blank `data:,` WebView” 的窗口**不是 Tauri 应用，而是 msedgedriver 回退启动的 Edge 浏览器首屏**。根因是上游 `tauri-driver` `map_capabilities()` 只把 `ms:edgeOptions.binary` 写入 legacy `desiredCapabilities`，而 `webdriver` 9.31.9 只发 W3C `capabilities`；`tauri-driver` `3.0.0-alpha.1` 该逻辑逐字相同，未修复。**因此 round-3 的 driver 钉版本身不足以解除阻塞**，`e856343` 的 `edgeDriverVersion` 透传保留无害但不解决问题。

完整证据链见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 “WQ-ENG-09b startup triage — root cause isolated”。

下一 Owner：维持 Windows。仅当决定添加 workaround（改 `wdio.conf.mjs`，或将 WDIO 栈降到仍发送 `desiredCapabilities` 的 `webdriver` 8.x）时，ownership 才交回 Cross-platform Owner——该 workaround 属 shared implementation change，应标记 `CROSS_PLATFORM_REVIEW_REQUIRED`，完成后交回 Windows 做 focused revalidation。真实账号、GUI/WebView2、Named Pipe、workflow、archive extraction 和 WQ-P1-12 filesystem fixture 继续保留在 Manual Windows Validation Queue。

## 同步方式

见文末 “跨会话交接总结” 的同名小节（该处为当前有效版本）。

## 跨会话交接总结（2026-09-28，Windows batch 2 收尾）


### 仓库实际状态（以 git 为准）

- Branch：`security/tweet-url-host-validation`；HEAD = `b918d605fae464933babb00391d33e5cc6cda694`，与 `origin` 完全同步（`git ls-remote` 一致）。
- `git status`：tracked 文件**无任何改动**（`--untracked-files=no` 为空）；未跟踪项仅为 ignored/本地产物：`.codex/`、`.venv-windows-validation/`、`aria2/`、`desktop/logs/`、`dist-portable/`、`gallery-dl/`、`manual-validation/`、`sidecar/build/`、`sidecar/gallery-dl/`、`sidecar/xarchive-downloader/`、`validation-artifacts/`。
- 最近 commits：`b918d60`（本轮 docs）→ `8ddb09a` → `27dcebc` → `e856343` → `5170161` → `a6311b3` → `01067b6`。
- `b918d60` 只改 3 个 `docs/` 文件（`windows-validation.md`、`platform-handoff.md`、`windows-queue.md`），无代码/依赖/测试改动。

### ⚠ 与聊天记忆的差异（以仓库为准）

本轮最初在 `5170161` 上验证，当时 diff 为 docs-only。**push 时被拒（non-fast-forward），fetch 后发现 Linux 侧已推进到 `8ddb09a`，其中 `e856343` 新增了 WDIO 测试层代码**（`desktop/e2e/support/startup-diagnostics.mjs`、`desktop/test/startup-diagnostics.test.mjs`，并修改两个 E2E spec 与 `desktop/wdio.conf.mjs`）。已 rebase 并保留双方文档内容。

因此：**`e856343` 的新测试层此前从未在 Windows 执行过**，本轮收尾补做了最小验证并发现新缺陷（见下）。

### 本轮新发现：`CROSS_PLATFORM_CHANGE_REQUIRED`

`desktop/test/startup-diagnostics.test.mjs:54` `resolves the artifact directory from WDIO_LOG_DIR and the default` 在 **Windows 失败**（Linux 上通过）：

```
AssertionError [ERR_ASSERTION]: Expected values to be strictly equal:
+ actual   'E:\project\desktop\custom-logs\startup'
- expected '\project\desktop\custom-logs\startup'
    at test\startup-diagnostics.test.mjs:56:12
```

根因：测试用 `path.join(path.sep, "project", "desktop")` 构造期望值。在 POSIX 上 `path.sep="/"` 得 `/project/desktop`；在 Windows 上 `path.sep="\"` 得 `\project\desktop`，而被测实现 `resolveDiagnosticsDir` 内部用 `path.resolve(root, …)`，会把无盘符的 root 解析为**当前盘符**（`E:`），于是实际值带 `E:\` 而期望值不带。**这是测试断言的平台假设缺陷，不是实现缺陷**——`path.resolve` 的盘符解析行为在 Windows 上是正确语义。建议修复方向：用平台无关的临时目录（如 `fs.mkdtempSync`）或 `path.resolve` 构造期望值，而不是硬编码 `path.sep` 拼接。此项属 shared test contract，**应由 Cross-platform Owner 修改**，Windows 侧不自行改测试。

命令：`npx --yes --package npm@11.19.0 -c "npm test --workspace desktop"` → **47 pass / 1 fail / 48 total**（`killTree` 通过，无沙箱权限问题）。日志：`validation-artifacts\windows-batch-20260928-5170161\desktop-test-b918d60.log`。

### 未完成工作分类

**可由下一 Task 立即继续**
- 无需等待外部条件：WQ-ENG-09b 已结清，无待办实现工作。
- 收尾复核上述 `startup-diagnostics` 测试修复后，在 Windows 重跑 `npm test --workspace desktop`，确认 48/48。

**需要额外信息或外部依赖**
- WQ-ENG-09b ordinary/advanced E2E：需上游 `tauri-driver` 修复 `map_capabilities()`，或降级到 `webdriver` 8.x。**不要在本仓库反复重试同一路径**。
- WQ-WORKER-BUILD-01 / WQ-PACKAGE-FULL-01 / WQ-PACKAGE-CORE-02：需重新生成含 `_internal\python312.dll` 的 worker artifact。
- WQ-ENG-07 / WQ-ENG-10（Actions）：需授权 CI run 与发布证书。
- WQ-RELEASE-06：需 `bundle.active=true`、代码签名证书、Updater 签名密钥。

**需要人工验证（Manual Windows Validation Queue）**
- WQ-ENG-06 真实账号错误脱敏（需专用非个人 X/gallery-dl + Telegram 账号）。
- WQ-ENG-01 破坏性打包删除保护（需一次性隔离副本，不得指向真实工作副本）。
- WQ-ENG-04 Named Pipe（**实现尚不存在**，`transport.rs` 目前 Unix-gated）。
- GUI / WebView2 / DPI / 键盘焦点 / 辅助技术（Computer Use 仍 `BLOCKED`，须人工桌面会话）。
- WQ-P1-12 剩余范围：permission/reparse/junction、长 JSON/Unicode、受控真实 Sidecar 下载 fixture。
- WQ-P1-02～05、P1-13：Native Host 安装/Registry/浏览器加载/Extension 实机。

**暂时不应继续**
- 在上游 `tauri-driver` 缺陷解除前，不要重复运行 WQ-ENG-09b 的 ordinary/advanced E2E——已用三种方式（WDIO 钉版、manual tauri-driver probe、直连 msedgedriver）确定性复现，重试无意义。
- 不要把 `data:,` 空白窗口再解读为产品启动缺陷：已证明那是 Edge 浏览器首屏。

### 下一 Task 必须保留的上下文

关键路径：
- `desktop/wdio.conf.mjs`（`edgeDriverVersion` 透传，32-33、48 行）
- `desktop/e2e/support/startup-diagnostics.mjs`（`resolveDiagnosticsDir` 用 `path.resolve`，第 23-28 行）
- `desktop/test/startup-diagnostics.test.mjs:54-64`（**待修的平台假设断言**）
- `desktop/e2e/specs/dashboard.e2e.mjs`、`wdio-plugin.e2e.mjs`（readiness 钩子已接入诊断采集）
- `desktop/scripts/wdio-tauri-service.mjs`（launcher/teardown，Windows 专属；`killTree` win32 分支）
- `docs/validation/windows-queue.md`（队列唯一事实源）、`docs/development/windows-validation.md`（逐轮证据）

环境事实（勿重复探测）：
- WebView2 Runtime = **153.0.4234.48**；Edge browser = **155.0.4283.18**（两者不同，勿用 Edge 版本推断 WebView2）。
- `tauri-driver` v2.1.0-alpha.0（`~/.cargo/bin/tauri-driver.exe`），需 `msedgedriver.exe` 在 PATH，否则启动即报 `CannotFindBinaryPath`。
- 可用 driver：`validation-artifacts\msedgedriver-153.0.4234.46\`（匹配 WebView2）、`msedgedriver-154.0.4258.24\`。以 `EDGEDRIVER_VERSION` 或 `TAURI_DRIVER_EDGE_VERSION` 钉版。
- `msedgedriver` 必须用 `--port=NNNN` 等号形式，`--port NNNN` 会报 `Invalid port. Exiting...`。
- 驱动程序需 `--native-port` 独立端口；建议用 45460+ 隔离端口，避免与遗留 4444/4445 冲突。
- system npm 11.17.0 低于项目 `engines.npm >=11.18.0`；跑 npm 命令用 `npx --yes --package npm@11.19.0 -c "…"`。
- 验证产物统一写入被 ignore 的 `validation-artifacts\windows-batch-20260928-5170161\`。

已知 workaround（直连 msedgedriver，可在无 tauri-driver 时取证）：
以 W3C `capabilities.alwaysMatch` 内联 `browserName=webview2` + `ms:edgeOptions.binary=<exe绝对路径>` + `ms:edgeOptions.webviewOptions={}`，直连 `msedgedriver --port=NNNN`，即可正常启动应用并取得 `h1=工作台`。

### 同步方式

沿 Git 将验证文档写回 source branch；不通过直接文件同步覆盖正式 Windows repo。Windows phase 使用 `git fetch` 后验证 exact implementation revision（见下方 round-3 section），并以 Git 提交号记录验证文档 revision。

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

## 2026-09-28 交接：Cross-platform reconcile 第 3 轮（WQ-ENG-09b triage）

本节为本会话写入。起点为 Windows 写回的 round-2 focused 结果（`5170161`）；本轮完成 reconcile、review 与全部非 Windows-dependent 工作，状态更新为 `READY_FOR_WINDOWS`。

### 源状态

- Branch `security/tweet-url-host-validation`；起点 HEAD `517016146e29e08cae38c8e33cfdb85c4b7e08`（先 `git fetch` 并快进；本地此前落后 origin 1 个 commit）。
- 本轮 implementation commit：`e856343ee03e44134f2189cba7e66dc83e00f3a0`（测试层 5 个文件）；handoff 与队列/状态/地图文档在本 commit 之后的 docs commit 记录。
- 治理文档核对：任务所列 `docs/development/platform-ownership.md`、`docs/development/git-platform-handoff.md`、`docs/validation/validation-policy.md` 在本地与 `origin` 树中均不存在；本轮按实际存在的 `AGENTS.md`、`docs/development/cross-platform-validation.md`、`docs/validation/windows.md`、`docs/validation/windows-queue.md`、`docs/development/windows-validation.md` 执行，未臆造这些文档的内容。

### Windows 上一轮结果复核（reconcile）

| 项 | Windows 结论 | Cross-platform 复核 |
|---|---|---|
| WQ-ENG-03 | `WINDOWS_PASS`（storage 27/27；真实 junction harness 1/1，根外文件未被改写） | 与 `01067b6` 的修复一致（`build_archive_metadata` 复用 `resolve_within`；Windows 不编译 `#[cfg(unix)]` 复现测试，故为 27 而非 Linux 30）。接受，无需再动 |
| WQ-ENG-08 | `WINDOWS_PASS`（42/42） | 根因落在验证沙箱的 `taskkill` 权限（提升权限后通过），非产品代码缺陷；本轮加入的 taskkill stderr/exit-code 诊断正是该结论的证据来源。接受 |
| WQ-ENG-09a | `WINDOWS_PASS`（npm 11.19 cleanroom `npm ci`+`npm ls`；npm 11.17 出现预期 `EBADENGINE` 并复现 unsupported 树） | 与 Linux cleanroom 二分结论一致。接受 |
| WQ-ENG-09b | `WINDOWS_FAIL`（EdgeDriver 155 下单一 handle、URL `data:,`、空标题、白屏；GUI 观察 `BLOCKED`） | 本轮 triage 目标，见下 |

### WQ-ENG-09b triage 结论

- **失败特征**：会话成功建立但 WebView 从未提交任何文档（`data:,`、空标题、白屏、仅一个 window handle）。这排除了“前端 JS 运行时报错”——那种情况文档 URL 会是 `tauri://` 协议且通常有错误内容。
- **已排除的仓库侧原因**（对照 2026-09-16 原生 WDIO `WINDOWS_PASS` 的 `3f70894`）：`desktop/src-tauri/tauri.conf.json`（含 `devUrl`/`frontendDist`/`beforeBuildCommand`/CSP/窗口配置）、`desktop/index.html`、`desktop/vite.config.js` 与 tauri 依赖版本**完全未变**；差异只有新增 command、`tauri-plugin-dialog` 注册与 capability 权限，均不参与导航。构建链 `npm run build` → `desktop/dist/index.html` 存在且 `frontendDist: ../dist` 指向它。
- **最可能的剩余类别：测试工具链的 driver ↔ WebView2 runtime 绑定问题**。官方 `@wdio/tauri-service` 按 Windows 注册表中的 **Edge 浏览器版本** 选择并缓存 msedgedriver（`%TEMP%\msedgedriver\{major}\`），而 Tauri 应用由 **WebView2 runtime** 驱动；本轮 Windows 观测到探测值 153 与实际 WebView2 155 不一致，且 4444 端口存在遗留的 153 driver。版本不匹配可解释“会话建立但目标文档为空白”。
- **未被证据排除的备选**：应用自身在 WebView2 下未导航（需独立启动观察）。该项保持人工队列，不由本轮 Linux 证据推断。
- **本轮未修改产品代码**；改动限于测试层可诊断性与 driver 版本可控性。

### 本轮改动（`e856343`）

- 新增 `desktop/e2e/support/startup-diagnostics.mjs`：枚举 window handle，逐个记录 URL/title 与截图，并写入失败原因，输出到 `WDIO_LOG_DIR/startup`（默认 `desktop/test-artifacts/wdio/startup`，已被 `.gitignore` 覆盖）。契约：best-effort，任何采集异常都不改变用例判定、不掩盖原始错误。
- `dashboard.e2e.mjs` / `wdio-plugin.e2e.mjs`：`before` 的 dashboard readiness 超时时先取证，再原样抛出 readiness 错误（附加证据路径）。通过路径行为不变。
- `wdio.conf.mjs`：`TAURI_DRIVER_EDGE_VERSION` 或 `EDGEDRIVER_VERSION` 透传为 service `edgeDriverVersion`；未设置时保持上游按 Edge 注册表探测，行为与改动前一致。
- 新增 `desktop/test/startup-diagnostics.test.mjs`（6 项单测）。

### Linux 验证（按当前 diff 的最小必要范围）

| 检查 | 结果 |
|---|---|
| `node --check`（新增模块、两个 spec、`wdio.conf.mjs`、新测试） | PASS 5/5 |
| WDIO 配置加载三态（未设置 → `undefined`；`TAURI_DRIVER_EDGE_VERSION=155` → `"155"`；`EDGEDRIVER_VERSION=154` → `"154"`） | PASS |
| `npm run test --workspace desktop` | PASS 48/48（42 既有 + 6 新增） |
| 真实 native E2E（Linux 本地） | `NOT RUN`：本机无 Xvfb/显示服务，Linux 侧只能做 config load 与单测；native 会话行为属 Windows 项 |
| Rust 全量 / Sidecar pytest / `npm run build` | `NOT RUN`：本轮 diff 仅涉及 Node 测试层，无 Rust/Python/构建产物变更，按最小必要范围不执行 |
| Windows 侧行为 | `BLOCKED`（本会话在 Linux） |

### Windows 队列累计（交回本轮 focused 清单）

- `WQ-ENG-09b revalidation`（`WINDOWS_VERIFICATION_PENDING`）：记录应用实际加载的 WebView2 runtime 版本 → 清理 `%TEMP%\msedgedriver\*` 与端口残留 → 以 `TAURI_DRIVER_EDGE_VERSION=<runtime major>` 重跑 ordinary/advanced E2E → 提交套件自动生成的 `startup-diagnostics-*.json` 与 `window-*.png`。
- `WQ-ENG-09b standalone comparison`（`WINDOWS_VERIFICATION_PENDING`，人工）：独立启动 binary 与 WebDriver 会话对照；同样空白则升级为产品启动路径问题。
- `WQ-ENG-03` / `WQ-ENG-08` / `WQ-ENG-09a` 的 `WINDOWS_PASS` 继续有效，本轮 diff 未命中其影响面（仅 `desktop/e2e/**`、`wdio.conf.mjs` 与新增测试）。
- 本轮**无 `WINDOWS_VERIFICATION_BLOCKING`**；`WQ-ENG-09b` 在 revalidation 前保持 `WINDOWS_FAIL`，不得记 PASS；`BLOCKED_AUTOMATION` 项（GUI/Computer Use）状态不变。

## 2026-09-28 交接：Cross-platform batch 收口（Plan 中非 Windows 依赖项）

本节记录本 batch 最后一段工作：完成 Plan 中所有不依赖 Windows 的开发与测试，统一汇总 Windows 专属验证项目，并为 `BLOCKED`/`NOT RUN` 项生成手工验证程序。状态仍为 `READY_FOR_WINDOWS`。

### 完成项

1. **关闭历史 Linux 验证缺口（Sidecar）**：`non-windows-completion.md` 此前记载“Linux 未安装 pytest/clippy”，该结论已过时。实测项目 `.venv` 含 `pytest 9.1.1`（Python 3.14.4），本轮实际执行 `python -m compileall -q sidecar`（PASS）与 `python -m pytest sidecar/tests -q`（**19 passed**），并已更正该文档中的过时限制。Sidecar 测试从本轮起是 Linux 常驻门禁的一部分。
2. **`archive_tweet` 全局锁项复核（Plan 中唯一保留的非 Windows 依赖开发项）**：按 ADR-009 处理，**不实施重构**，改为记录代码级证据与前置条件（见 [`../architecture/decisions.md`](../architecture/decisions.md) 的 “2026-09-28 复核”）。核实事实：`desktop/src-tauri/src/archive.rs:544-663` 在整个 `download_sidecar` 与 `complete_sidecar_archive` 期间持有 `RuntimeState` guard，其余 `Mutex<RuntimeState>` 消费者（`commands.rs` 状态/指标、`transport.rs` Browser 入口）在归档期间阻塞；且无 command 级测试覆盖该 fallback。保持 `NEEDS_DEVELOPMENT_REVIEW` 的三条理由：释放锁会改变并发语义（并发归档将从串行变为第二个请求返回 “sidecar is not running”）、正确解法需要 supervisor 租约设计、roadmap 已将入口切换门控在 Windows runtime 证据之后。
3. **Windows 专属项统一汇总 + 手工程序**：在 [`../validation/windows-queue.md`](../validation/windows-queue.md) 追加 “Windows 专属验证项目总览” 一节，按 A（已关闭）/ B（`BLOCKED`）/ C（`NOT RUN`）/ D（待 focused revalidation）/ E（其余 pending，步骤已在原表）分组，并为 B、C 组生成 **M1–M7 手工验证程序**（每项含前置、步骤、预期与 PASS/FAIL 判定）。

### Linux 验证

| 检查 | 结果 |
|---|---|
| `.venv/bin/python -m pytest sidecar/tests -q` | PASS 19/19 |
| `.venv/bin/python -m compileall -q sidecar` | PASS |
| 代码改动 | `NOT RUN`（本轮仅文档；上一 commit `e856343` 的 Node 48/48 仍为最新代码证据） |
| Rust / Node / Windows | `NOT RUN` / 复用 `e856343` 结论 / `BLOCKED`（无 Windows 环境） |

### Windows 队列与阻塞

- 本轮**无 `WINDOWS_VERIFICATION_BLOCKING`**，也未新增任何 Windows 专属实现。
- 交回 Windows 的执行清单：`WQ-ENG-09b` focused revalidation（round-3 表）+ 手工程序 M1–M7 + E 组中命中 diff 的 pending 项。
- 明确跳过（不可由 Linux 解除）：`WQ-ENG-04`（Named Pipe 实现不存在）、`WQ-ENG-06`（无测试账号）、`WQ-WORKER-BUILD-01`/PACKAGE（缺 artifact）、GUI/Computer Use（`BLOCKED_AUTOMATION`）、`WQ-P1-02/03/04/05/13`（Windows 专属实现未完成）。

### 下一 Owner

Windows Owner：先执行 `WQ-ENG-09b` focused revalidation，再按 M1–M7 推进被阻塞项；E 组按 `cross-platform-validation.md` 的影响面分析择命中项执行。若 M5 的独立启动对照显示应用在 WebView2 下同样空白，或 `WQ-ENG-09b` 仍无法归因于 driver/runtime 绑定，则把启动路径问题交回 Cross-platform Owner。

## 2026-09-28 Windows focused phase result

- Implementation revision tested: `01067b66a3aa214fb90f7d893c57bb971a7c7882`; checkout handoff document revision: `a6311b31a6791b9eae0ca531919048d02063d9bd`.
- `WQ-ENG-03` junction containment: `WINDOWS_PASS` (storage 27/27; actual Windows junction probe 1/1, outside file unchanged).
- `WQ-ENG-08` killTree: `WINDOWS_PASS` (Desktop Node 42/42 with elevated retry after sandbox `taskkill` access-denied).
- `WQ-ENG-09a` npm contract: `WINDOWS_PASS` on npm 11.19 cleanroom (`npm ci` and `npm ls`); npm 11.17 emits the expected `EBADENGINE` and still reproduces `ELSPROBLEMS` when unsupported.
- `WQ-ENG-09b` ordinary/advanced Tauri E2E: `WINDOWS_FAIL` on matching EdgeDriver 155; session remained on blank `data:,`. Fresh evidence includes one window handle, empty title, blank screenshot, and zero-byte new app log files. Computer Use native app inventory was unavailable, so visual GUI validation is `BLOCKED` and remains in the manual queue.
- Next Owner: Cross-platform Owner to triage whether the blank WebView is an app navigation/startup issue or session attachment/environment issue. No implementation change is authorized by current evidence alone; return only concrete code/contract changes to Windows for focused revalidation.
