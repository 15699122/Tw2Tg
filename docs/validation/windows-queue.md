# Windows Validation Queue

本文是当前 Windows 验证队列的唯一入口。历史执行结果、环境日志和逐轮 reconciliation 保存在 [`../development/windows-validation.md`](../development/windows-validation.md)；Windows 执行规范和报告模板见 [`windows.md`](windows.md)。

## 状态规则

- `WINDOWS_VERIFICATION_PENDING`：功能或代码已有，但需要在 Windows 目标环境确认；不阻塞 Linux 开发。
- `WINDOWS_VERIFICATION_BLOCKING`：只有缺少 Windows 结果会使后续 Linux 设计或实现无法可靠继续时使用。
- `WINDOWS_PASS`：当前关联 revision 已完成 Windows 验证。
- `WINDOWS_FAIL`：Windows 验证发现需要处理的项目代码或平台问题。
- `WINDOWS_BLOCKED`：前置环境、账号、权限或外部服务不可用。
- `NOT RUN`：本轮没有执行，必须同时说明原因。
- `NOT APPLICABLE`：当前项目配置或验证范围不适用。

当前没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。

## 本轮收口的 BLOCKED / NOT RUN 项目与手工验证入口

以下项目不因 Linux 收口而标记为 PASS。它们要么缺少 Windows/外部前置，要么关联功能尚未实现；进入 Windows validation phase 时按下列手工步骤处理。

| 项目 | 状态 | 原因 | 手工验证步骤 |
|---|---|---|---|
| 真实 Edge/X Cookie、Telegram 账号、Credential Manager | `BLOCKED` | 缺少受控测试账号、Edge profile、凭据和外部服务授权 | 准备专用非个人测试账号和空白 Edge profile；设置项目 Python/Sidecar；执行单媒体、多媒体、Quote/Reply、重复提交、认证失败和重启恢复；确认 Cookie/token/secret 不进入 stdout、SQLite payload、WebView 或日志；Telegram 使用测试 chat 验证保存/读取/删除、重启和失败重试。 |
| GUI WebView2/DPI/屏幕阅读器/原生桌面自动化 | `BLOCKED` | 依赖 Windows WebView2、DPI 环境和可用 GUI automation target；Linux 静态检查不能替代 | 在 Windows 启动 Tauri Debug；设置 100%、125%、150% DPI；测试最小窗口、Tab/Shift+Tab、Enter/Escape、焦点和错误状态；使用 Narrator/NVDA 检查角色、名称、状态、焦点和对比度；保存截图/录屏及工具错误。 |
| Native Host Named Pipe、Registry、浏览器安装 | `NOT RUN` / `BLOCKED` | Named Pipe server、manifest/Registry/installer 前置尚未形成最终可验证 artifact | 若 artifact 已提供：注册 host manifest，使用 `\\.\\pipe\\xarchive-v1`；管理员/普通用户分别测试启动、request/response、request_id、多连接、断线重连、非法消息和退出。若 server/manifest 未提供，保留 `NOT RUN`，不得用 framing 单测替代。 |
| externalBin、Installer、signing、updater、Tray/Autostart | `NOT RUN` / `BLOCKED` | 当前 bundle/installer 或签名前置未完成/未提供 | 若生成 artifact：执行全新安装、覆盖升级、自定义非 ASCII 路径、卸载、签名/SmartScreen、失败回滚、数据保留、Tray、Single Instance 和 Autostart；若 bundle inactive 或 artifact 不存在，记录 `NOT APPLICABLE`/`NOT RUN` 及缺失前置。 |
| 真实 executor worker 接管 `ArchiveExecutionContext` | `NOT RUN` | Linux 已完成 runner-owned `ExecutorConfig`/`ProductionExecutionFactory`、immutable execution spec、single active runner、startup recovery scan、filesystem recovery action 和运行中 cancellation；最终用户入口切换尚未完成，Windows 运行时行为仍需目标环境确认 | 在最终接入 revision 上启动 Desktop；设置项目 Python/Sidecar；执行 submit/query/cancel/shutdown、成功/失败/terminal skip、duplicate/concurrent jobs；确认 runner 从 `archive_job_requests` 加载 request，独立创建 Database/FileStore/Sidecar，检查 RuntimeState 锁、SQLite state/event/error 顺序、attempt fencing、cancel 后 Sidecar 进程退出、异常退出和重启恢复。若缺少最终 artifact 或 restart fixture，保留 `NOT RUN`，不得用 Linux contract 替代。 |

## 当前队列

| ID | 类别 | 验证项目 | 关联模块/修改 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-P0-01 | Build/Toolchain | Windows workspace 与 Tauri baseline | `Cargo.toml`、`package.json`、`desktop/`、`sidecar/` | MSVC、Windows SDK、WebView2、Python executable 和 Tauri 构建不能由 Linux 完全替代 | Windows toolchain、项目 `.venv`、Node dependencies | 执行 Rust fmt/check/test/clippy、Node check/test/build、Sidecar tests、Tauri build/start/cleanup | 所有适用检查通过，无项目代码失败 | P0 | no | `WINDOWS_PASS` |
| WQ-P0-02 | Integration | 真实 X/Edge Cookie archive | Edge Profile、Sidecar、Storage、Desktop archive flow | Cookie 加密存储、Edge Profile 和真实 X 响应只能在目标环境确认 | 测试账号、Edge Profile、gallery-dl、可用网络 | 覆盖无媒体、单图、多图、视频、Quote/Reply、重复任务和异常退出后的真实归档 | Cookie 不泄露；Tweet、媒体、SQLite、staging 正确且幂等 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-03 | Filesystem | 文件 SQLite 应用级恢复 | `crates/xarchive-storage/migrations/`、Storage、Tauri Desktop | 文件锁、应用重启、Windows 路径和异常退出无法由 in-memory 测试充分判断 | Desktop artifact、受控目录、可重复数据、旧库副本 | 验证真实文件 DB、关闭/重启、遗留 staging、`0001 → 0002 → 0003` 和异常退出恢复 | 状态恢复、迁移、staging 清理和唯一约束正确 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-04 | Integration | Native Host/Named Pipe end-to-end | Native Host、Protocol、Desktop `transport.rs`、Desktop IPC | Named Pipe server、ACL、连接和 Windows IPC 生命周期是平台行为；Linux transport contract 不能替代端到端验证 | Named Pipe server、`\\.\\pipe\\xarchive-v1`、ACL 方案、最新 Desktop artifact | 验证请求/响应、request_id 路由、多连接、重连、关闭、非法消息和权限拒绝；确认 transport adapter 的 submit/query 响应与 Native Host framing 一致 | 合法请求正确转发，非法或越权请求明确失败，无串线或挂起；request_id 不丢失 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-01 | Integration | DownloadRouter 与真实 aria2 业务集成 | `xarchive-download`、Desktop Job、Sidecar/Job orchestration | aria2c.exe、Windows 路径、真实 media URL 和进程恢复需目标环境确认 | 受控 aria2c.exe（当前半永久化验证目录：E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2）、media server、可重复归档场景 | 验证 gallery-dl 默认、错误回退、403 后重新提取、transfer lifecycle 和 Job 状态同步 | fallback 只在适用错误触发，状态、事件和文件结果一致 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-02 | Packaging/Integration | Native Host browser installation | Native Host manifest、Registry、Installer | Registry、浏览器扩展 ID 和安装权限是 Windows 专属行为 | Host manifest、固定 Extension ID、Edge/Chrome 实机 | 验证安装、升级、卸载、管理员/非管理员、扩展加载和 Service Worker 重连 | 浏览器能加载 Host，连接和错误反馈符合协议 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-03 | Regression | Windows GUI rendering and accessibility | `desktop/src/main.jsx`、`desktop/src/style.css`、Tauri | WebView2、DPI、系统字体、屏幕阅读器和命中区域不能由 Linux 静态检查替代 | WebView2、DPI 环境、键盘、Narrator/NVDA | 验证 100/125/150% DPI、最小窗口、Tab、键盘、Focus-visible、辅助技术和对比度 | 真实渲染、交互和辅助技术反馈符合预期 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-04 | Integration | Credential Manager 与 Telegram account flow | `xarchive-telegram`、Windows secret backend | Credential Manager 用户边界和真实账号/网络行为依赖 Windows/外部环境 | Credential Manager backend、Bot token、Telegram test chat | 验证保存/读取/更新/删除、应用重启、日志隔离、真实发送和限流 | Secret 不泄露，真实发送和重试状态正确 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-05 | Runtime/Packaging | Sidecar、externalBin 和进程生命周期 | Tauri packaging、Sidecar、Tray/Autostart | Windows 子进程、资源路径、关闭和自启动行为需要目标环境 | bundled Sidecar、Tauri bundle | 验证启动、关闭、崩溃恢复、资源定位、Tray、Single Instance 和 Autostart | 资源可定位，子进程安全退出，生命周期正确 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-12 | Security/Privacy | Security boundary regression | Protocol、Storage、Desktop、Sidecar protocol/schema | Windows path semantics、reparse points、Desktop IPC packaging 和真实 Sidecar 边界不能由 Linux 完全替代 | 最新 Linux working tree、Windows workspace、受控 staging | 验证 URL/Tweet ID mismatch、metadata mismatch、executable override、敏感 settings、长 JSON、普通文件和 symlink/junction/reparse | 非法 identity、override、敏感 settings 和 link/reparse 被拒绝；合法 Unicode 文件仍可归档 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-13 | Privacy/Filesystem | Windows user-data privacy boundary | Desktop archive root、SQLite、staging、settings | Windows ACL、user profile、working directory 和共享目录权限不能由 Linux mode 替代 | 普通用户、非管理员账户、ACL 工具、受控临时目录 | 验证不同 working directory/盘符下 archive root、SQLite、WAL/SHM、staging ACL 和跨用户读取 | 数据目录定位稳定，仅当前用户可读写，权限错误可诊断 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P2-01 | Packaging | Installer、signing 和 updater | Tauri bundle、installer、updater | 安装器、签名、SmartScreen、Defender、升级/回滚为 Windows 发布行为 | installer artifact、证书/签名环境、发布测试机 | 验证全新安装、覆盖升级、自定义路径、卸载、签名、失败回滚和数据保留 | 安装、升级、卸载和回滚符合发布要求 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P2-02 | Filesystem/Regression | Windows filesystem stress and stability | Core、Storage、Desktop 用户目录/staging | 保留字符、长路径、文件锁和并行时序需要 Windows 文件系统确认 | 多盘、空格/中文/Unicode、保留名、长路径、文件锁、磁盘不足 | 执行路径、锁、磁盘和并行恢复压力场景 | 无路径逃逸、数据损坏或未处理崩溃 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-14 | Runtime/Integration | R1 executor runner-owned production integration, Browser transport contract and synchronous fallback regression | `desktop/src-tauri/src/executor.rs`、`transport.rs`、`archive.rs`、`runtime.rs`、`commands.rs`、`crates/xarchive-storage/migrations/0004_archive_job_requests.sql`、Storage、SidecarSupervisor、FileStore | Linux 已完成 immutable execution spec、`ExecutorConfig`/`ProductionExecutionFactory`、job_id spec loading、runner-owned Database/FileStore/Sidecar、attempt fencing、single active runner、startup recovery scan、真实 recovery action、运行中 cancellation 和 transport protocol contract；最终用户入口切换以及 Windows 运行时行为仍需目标环境确认 | 在当前最终接入 revision 上执行 executor lifecycle status、submit/query/cancel/shutdown、duplicate/concurrent jobs、Browser request_id 路由、synchronous fallback、execution spec reload、attempt fencing、Sidecar crash、graceful shutdown、startup/restart recovery、resource ownership 和无长锁阻塞检查 | Windows Tauri artifact、项目 Python/Sidecar、受控 SQLite/archive root、可重复 fixtures、可制造异常退出和文件锁 | lifecycle status 与实际 worker 存活一致；execution spec 可重启加载；transport request_id 和状态枚举与 schema 一致；cancel 后 Sidecar 退出；late result 不覆盖 INTERRUPTED；Sidecar/Database/FileStore 由 runner ownership；无 RuntimeState 长锁阻塞；startup recovery 与 WQ-P1-15 facts/action 一致；无残留进程或重复归档 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-15 | Filesystem/Runtime | R1 commit recovery facts and action validation | `desktop/src-tauri/src/executor.rs`、Storage `ArchiveService`、FileStore staging/archives | Linux 已实现 filesystem facts/action；真实 Windows rename/lock/restart/reparse 行为必须在目标文件系统确认 | 覆盖 DOWNLOADED+staging、DOWNLOADED+final、DOWNLOADED+neither、COMPLETE+final、COMPLETE+missing、mixed batch recovery；检查 SQLite state/event/error order | Windows workspace、受控 archive root、可制造异常退出和文件锁、旧/新 SQLite fixtures | Resume 不伪造 COMPLETE；final 存在时补写 COMPLETE；staging 可从 `tweet.json` 重做本地 commit；缺失 commit 可诊断；terminal 不重复处理；批量错误隔离 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-16 | Regression/Automation | WebdriverIO + @wdio/tauri-service native Windows smoke | desktop/wdio.conf.mjs、desktop/e2e/specs/dashboard.e2e.mjs、Tauri release artifact、WebView2 | Windows WebView2、Edge WebDriver 版本匹配、真实 native window 生命周期和 DOM 可见性不能由 Linux Node/Vite 检查替代 | Windows 11、WebView2、Node/npm、已安装 WDIO dependencies、npm run build:tauri 生成的 xarchive-desktop.exe | 在 Windows 执行 npm run test:e2e:windows --workspace desktop；验证 service external provider 启动/连接/关闭、Dashboard heading、main、导航和概览区域；保留 WDIO/service 日志 | service 自动准备匹配 Edge WebDriver；Tauri 窗口可连接并在测试结束退出；smoke 全部通过；失败时输出 binary/driver/port 原因 | P1 | no | WINDOWS_PASS |
| WQ-P1-17 | Regression/Automation | tauri-plugin-wdio advanced API and log bridge | `desktop/src-tauri/Cargo.toml`、`src-tauri/src/lib.rs`、`src-tauri/capabilities/wdio.json`、`tauri.conf.json`、`desktop/src/main.jsx`、`desktop/wdio.conf.mjs`、`desktop/e2e/specs/wdio-plugin.e2e.mjs` | Linux 已完成 plugin 配置，但 browser.tauri.execute、mocking、窗口级 IPC 和前后端日志转发仍依赖 Windows WebView2/native window | Windows WebView2、Node/npm、WDIO dependencies、`npm run build:tauri:wdio --workspace desktop` 生成的专用 artifact | 在 Windows 运行 `npm run test:e2e:windows:advanced --workspace desktop`；验证 `isTauriApiAvailable`、execute/invoke interception、mock 生命周期、日志捕获和 session teardown；随后用普通 release artifact 运行 WQ-P1-16，确认不携带 debug-only plugin | 高级 API 可用且 teardown 无 mock-store warning；普通 release smoke 仍通过；日志不泄露凭据；插件不进入普通 release artifact | P1 | no | `WINDOWS_VERIFICATION_PENDING` |

## 已有 Windows 结果但不关闭当前队列的项目

以下结果已在历史报告中记录，但不能扩大为当前队列项目的完整 PASS：

- Windows workspace fmt/check/clippy/test、Node check/test/build、Sidecar pytest 和 Tauri Debug/Release build 已在 `5f18ae0` clean Linux commit 对应的 canonical E: 工作副本完成；项目 `.venv` 前置下 WQ-P0-01 为 `WINDOWS_PASS`。未设置 `PYTHON` 的额外 Rust 测试诊断仍复现 `NotRunning`，属于环境前置失败，不改写为项目代码失败。
- 2026-09-14 Windows 轮：项目 Python 前置下 Windows `144/144` workspace tests、Desktop `58` tests、Tauri Release build、Debug startup/cleanup 和 Tauri MCP backend/window smoke（`127.0.0.1:9223`）为 `PASS`；Tauri MCP WebView eval 层（DOM/截图/console/IPC invoke）为 `BLOCKED`（2 秒 timeout）。应用级 executor real-worker integration、应用级旧库迁移/重启、reparse/ACL/长路径、bundle/packaging 和真实外部账号仍然 `NOT RUN` / `BLOCKED` / pending。该轮验证对象包含 MCP Bridge、executor recovery 和 cancellation 的 dirty working tree；其后的 Linux 仅做 release `unused_mut` warning 的 warning-only 修复，不影响行为。
- aria2 artifact、版本/hash、loopback RPC、Unicode/空格路径、暂停/恢复、进程中断恢复和 .aria2 清理已有独立 Windows 证据；当前半永久化验证目录为 E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2。WQ-P1-01 的项目级 DownloadRouter、Desktop Job、403 refresh 和 transfer lifecycle 仍 pending。
- WQ-P1-12 的自动化安全边界子集已有通过记录；reparse/junction/长 JSON/Unicode 专项缺少可重复 Windows harness，仍 pending。
- Storage 库级 migration/reopen/profile 测试已有通过记录；WQ-P0-03 的 Desktop 文件数据库、应用重启和旧库升级仍 pending。

## 集中式 Windows handoff

进入 Windows validation phase 前，基于最终 diff、当前 Plan、变更模块和本队列合并重复场景，按以下类别执行：

1. **Build/Toolchain**：WQ-P0-01。
2. **Runtime**：Sidecar、Tauri、Job executor、production fallback 和进程清理，关联 WQ-P0-01、WQ-P1-05、WQ-P1-14。
3. **Filesystem**：WQ-P0-03、WQ-P1-12、WQ-P1-13、WQ-P1-14、WQ-P1-15、WQ-P2-02。
4. **Integration**：WQ-P0-02、WQ-P0-04、WQ-P1-01、WQ-P1-02、WQ-P1-04。
5. **Packaging**：WQ-P1-05、WQ-P2-01。
6. **Regression**：WQ-P1-03、WQ-P1-16 和所有本轮受影响的协议/存储/Sidecar 场景。

具体命令、人工交互要求、状态记录格式和错误分类以 [`windows.md`](windows.md) 为准。验证结束后将结果写入历史验证报告，并回到本文件更新当前队列状态。
