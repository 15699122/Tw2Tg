# Windows Validation Queue

本文是当前 Windows 验证队列的唯一入口。历史执行结果、环境日志和逐轮 reconciliation 保存在 [`../development/windows-validation.md`](../development/windows-validation.md)；Windows 执行规范和报告模板见 [`windows.md`](windows.md)。

当前 WQ-P1-16/WQ-P1-17 的具体执行顺序和 PowerShell 步骤见 [`windows-wdio-handoff.md`](windows-wdio-handoff.md)。

## 状态规则

- `WINDOWS_VERIFICATION_PENDING`：功能或代码已有，但需要在 Windows 目标环境确认；不阻塞 Linux 开发。
- `WINDOWS_VERIFICATION_BLOCKING`：只有缺少 Windows 结果会使后续 Linux 设计或实现无法可靠继续时使用。
- `WINDOWS_PASS`：当前关联 revision 已完成 Windows 验证。
- `WINDOWS_FAIL`：Windows 验证发现需要处理的项目代码或平台问题。
- `WINDOWS_BLOCKED`：前置环境、账号、权限或外部服务不可用。
- `NOT RUN`：本轮没有执行，必须同时说明原因。
- `NOT APPLICABLE`：当前项目配置或验证范围不适用。

当前没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。

## 重验元数据与增量重验

队列状态按 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md) §3.3 的重验规则维护。每个验证项可记录重验元数据：

- `last validated revision`：最近一次给出当前状态时的 Linux revision；
- `impact area`：相关的文件、模块或行为；
- `dependencies`：影响结论有效性的依赖或前置；
- `revalidation decision`：`KEEP_VALID`（当前 diff 无交集，保持原结论）或 `REVALIDATION_REQUIRED`（有交集或依赖变化）。

判定规则：

- 若当前 diff 与某项影响区无交集且相关依赖未变化，则该项保持上一轮结论（含 `WINDOWS_PASS`），本轮不必重复执行，也不得仅凭「队列仍为 pending」就把整份队列当成下一轮默认执行清单；
- 若存在交集、依赖变化或行为可能使原结论失效，则标记 `REVALIDATION_REQUIRED` 并回到 `WINDOWS_VERIFICATION_PENDING`；
- 历史条目不强求补填无法可靠追溯的 revision；元数据在后续轮次实际重验时随结果一并维护。

### 2026-09-16 Linux security-contract follow-up

- Linux 已将 `SidecarCommand` 标记为 `serde(deny_unknown_fields)`，并新增协议层回归测试，确认已移除的 per-request `executable` 字段不会被 JSONL consumer 接受。
- 这只闭合了跨层 schema/model contract 的 Linux 可验证部分；WQ-P1-12 仍为 `WINDOWS_VERIFICATION_PENDING`。Windows 仍需在真实 Sidecar/便携运行时中验证命令拒绝、合法命令执行、路径权限、symlink/junction/reparse 和错误诊断。
- 若真实 Windows endpoint、受控 Sidecar fixture 或 reparse harness 不可用，跳过对应验证并记录为 `BLOCKED`/`NOT RUN`；手工步骤继续使用本文件“BLOCKED / NOT RUN 项目与手工验证入口”中的步骤，不得将 Linux 协议测试外推为 Windows PASS。

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
| WQ-P1-01 | Integration | DownloadRouter 与真实 aria2 业务集成 | `xarchive-download`、Desktop Job、Sidecar/Job orchestration | aria2c.exe、Windows 路径、真实 media URL 和进程恢复需目标环境确认 | 受控 aria2c.exe（当前半永久化验证目录：E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2）、media server、可重复归档场景 | 验证 gallery-dl 默认、错误回退、403 后重新提取、transfer lifecycle 和 Job 状态同步 | fallback 只在适用错误触发，状态、事件和文件结果一致 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-02 | Packaging/Integration | Native Host browser installation | Native Host manifest、Registry、Installer | Registry、浏览器扩展 ID 和安装权限是 Windows 专属行为 | Host manifest、固定 Extension ID、Edge/Chrome 实机 | 验证安装、升级、卸载、管理员/非管理员、扩展加载和 Service Worker 重连 | 浏览器能加载 Host，连接和错误反馈符合协议 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-03 | Regression | Windows GUI rendering and accessibility | `desktop/src/main.jsx`、`desktop/src/style.css`、Tauri | WebView2、DPI、系统字体、屏幕阅读器和命中区域不能由 Linux 静态检查替代 | WebView2、DPI 环境、键盘、Narrator/NVDA | 验证 100/125/150% DPI、最小窗口、Tab、键盘、Focus-visible、辅助技术和对比度 | 真实渲染、交互和辅助技术反馈符合预期 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-04 | Integration | Credential Manager 与 Telegram account flow | `xarchive-telegram`、Windows secret backend | Credential Manager 用户边界和真实账号/网络行为依赖 Windows/外部环境 | Credential Manager backend、Bot token、Telegram test chat | 验证保存/读取/更新/删除、应用重启、日志隔离、真实发送和限流 | Secret 不泄露，真实发送和重试状态正确 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-05 | Runtime/Packaging | Sidecar、externalBin 和进程生命周期 | Tauri packaging、Sidecar、Tray/Autostart | Windows 子进程、资源路径、关闭和自启动行为需要目标环境 | bundled Sidecar、Tauri bundle | 验证启动、关闭、崩溃恢复、资源定位、Tray、Single Instance 和 Autostart | 资源可定位，子进程安全退出，生命周期正确 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-12 | Security/Privacy | Security boundary regression | Protocol、Storage、Desktop、Sidecar protocol/schema | Windows path semantics、reparse points、Desktop IPC packaging 和真实 Sidecar 边界不能由 Linux 完全替代 | 最新 Linux working tree、Windows workspace、受控 staging | 验证 URL/Tweet ID mismatch、metadata mismatch、executable override、敏感 settings、长 JSON、普通文件和 symlink/junction/reparse | 非法 identity、override、敏感 settings 和 link/reparse 被拒绝；合法 Unicode 文件仍可归档 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-13 | Privacy/Filesystem | Windows user-data privacy boundary | Desktop archive root、SQLite、staging、settings | Windows ACL、user profile、working directory 和共享目录权限不能由 Linux mode 替代 | 普通用户、非管理员账户、ACL 工具、受控临时目录 | 验证不同 working directory/盘符下 archive root、SQLite、WAL/SHM、staging ACL 和跨用户读取 | 数据目录定位稳定，仅当前用户可读写，权限错误可诊断 | P1 | no | `WINDOWS_FAIL` |
| WQ-P2-01 | Packaging | Installer、signing 和 updater | Tauri bundle、installer、updater | 安装器、签名、SmartScreen、Defender、升级/回滚为 Windows 发布行为 | installer artifact、证书/签名环境、发布测试机 | 验证全新安装、覆盖升级、自定义路径、卸载、签名、失败回滚和数据保留 | 安装、升级、卸载和回滚符合发布要求 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P2-02 | Filesystem/Regression | Windows filesystem stress and stability | Core、Storage、Desktop 用户目录/staging | 保留字符、长路径、文件锁和并行时序需要 Windows 文件系统确认 | 多盘、空格/中文/Unicode、保留名、长路径、文件锁、磁盘不足 | 执行路径、锁、磁盘和并行恢复压力场景 | 无路径逃逸、数据损坏或未处理崩溃 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-14 | Runtime/Integration | R1 executor runner-owned production integration, Browser transport contract and synchronous fallback regression | `desktop/src-tauri/src/executor.rs`、`transport.rs`、`archive.rs`、`runtime.rs`、`commands.rs`、`crates/xarchive-storage/migrations/0004_archive_job_requests.sql`、Storage、SidecarSupervisor、FileStore | Linux 已完成 immutable execution spec、`ExecutorConfig`/`ProductionExecutionFactory`、job_id spec loading、runner-owned Database/FileStore/Sidecar、attempt fencing、single active runner、startup recovery scan、真实 recovery action、运行中 cancellation 和 transport protocol contract；最终用户入口切换以及 Windows 运行时行为仍需目标环境确认 | 在当前最终接入 revision 上执行 executor lifecycle status、submit/query/cancel/shutdown、duplicate/concurrent jobs、Browser request_id 路由、synchronous fallback、execution spec reload、attempt fencing、Sidecar crash、graceful shutdown、startup/restart recovery、resource ownership 和无长锁阻塞检查 | Windows Tauri artifact、项目 Python/Sidecar、受控 SQLite/archive root、可重复 fixtures、可制造异常退出和文件锁 | lifecycle status 与实际 worker 存活一致；execution spec 可重启加载；transport request_id 和状态枚举与 schema 一致；cancel 后 Sidecar 退出；late result 不覆盖 INTERRUPTED；Sidecar/Database/FileStore 由 runner ownership；无 RuntimeState 长锁阻塞；startup recovery 与 WQ-P1-15 facts/action 一致；无残留进程或重复归档 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-15 | Filesystem/Runtime | R1 commit recovery facts and action validation | `desktop/src-tauri/src/executor.rs`、Storage `ArchiveService`、FileStore staging/archives | Linux 已实现 filesystem facts/action；真实 Windows rename/lock/restart/reparse 行为必须在目标文件系统确认 | 覆盖 DOWNLOADED+staging、DOWNLOADED+final、DOWNLOADED+neither、COMPLETE+final、COMPLETE+missing、mixed batch recovery；检查 SQLite state/event/error order | Windows workspace、受控 archive root、可制造异常退出和文件锁、旧/新 SQLite fixtures | Resume 不伪造 COMPLETE；final 存在时补写 COMPLETE；staging 可从 `tweet.json` 重做本地 commit；缺失 commit 可诊断；terminal 不重复处理；批量错误隔离 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-16 | Regression/Automation | WebdriverIO + @wdio/tauri-service native Windows smoke | desktop/wdio.conf.mjs、desktop/e2e/specs/dashboard.e2e.mjs、Tauri release artifact、WebView2 | Windows WebView2、Edge WebDriver 版本匹配、真实 native window 生命周期和 DOM 可见性不能由 Linux Node/Vite 检查替代 | Windows 11、WebView2、Node/npm、已安装 WDIO dependencies、npm run build:tauri 生成的 xarchive-desktop.exe | 在 Windows 执行 npm run test:e2e:windows --workspace desktop；验证 service external provider 启动/连接/关闭、Dashboard heading、main、导航和概览区域；保留 WDIO/service 日志 | service 自动准备匹配 Edge WebDriver；Tauri 窗口可连接并在测试结束退出；smoke 全部通过；失败时输出 binary/driver/port 原因 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-17 | Regression/Automation | tauri-plugin-wdio advanced API and log bridge | desktop/src-tauri/Cargo.toml, src-tauri/src/lib.rs, src-tauri/capabilities/wdio.json, tauri.conf.json, desktop/src/main.jsx, desktop/wdio.conf.mjs, desktop/e2e/specs/wdio-plugin.e2e.mjs | Linux 已完成 plugin 配置，但 browser.tauri.execute、mocking、窗口级 IPC 和前后端日志转发仍依赖 Windows WebView2/native window | Windows WebView2、Node/npm、WDIO dependencies、npm run build:tauri:wdio --workspace desktop 生成的专用 artifact | 在 Windows 运行 npm run test:e2e:windows:advanced --workspace desktop；通过 browser.tauri.execute 检查 window.wdioTauri，验证 execute/invoke interception、mock 生命周期、日志捕获和 session teardown；随后用普通 release artifact 运行 WQ-P1-16，确认不携带 debug-only plugin | 高级 API 可用且 teardown 无 mock-store warning；普通 release smoke 仍通过；日志不泄露凭据；插件不进入普通 release artifact | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-18 | Runtime/Filesystem/Packaging | 便携 `.exe` 目录布局与首次下载 setup | `portable.rs`、`config.rs`、`runtime.rs`、`commands.rs`、`aria2.rs`、`crates/xarchive-storage/src/file_store.rs`、`desktop/scripts/build-portable-windows.mjs`、`desktop/src/main.jsx` | exe 同目录解析、系统 Downloads 定位、跨卷 rename 提交、sidecar/Extension 实际分发和 Windows 文件权限只能在目标环境确认 | `build:portable:windows` 便携目录、无 `config.yaml` 首启状态、受控 `download` 目录、可选第二盘符 | 首启生成 `config/`（`config.yaml` + `archive.sqlite3`）、`cache/staging/`、`download/`、同级 `logs/`、`sidecar/`、`extension/`，不创建 `telegram/`；GUI 选择 portable 或系统 `Downloads/XArchive`；拒绝创建 `download/` 时 fallback；staging→最终目录提交；sidecar env 优先、fallback `config.yaml` | 布局与文档一致、`config.yaml` 持久化、fallback 生效、便携目录移动后无绝对路径残留、无路径逃逸 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-19 | Runtime/Regression | 日志等级与轮转 | `logging.rs`、`config.rs`、`desktop/src/main.jsx` 设置 UI、便携 `logs/` | 日志文件创建/删除、只读/权限行为和轮转时序依赖 Windows 文件系统 | 便携工作副本、`logs/` 可写与只读两种场景、可编辑 `config.yaml` | 验证 Release 默认 `info`/Debug 默认 `debug`、YAML/GUI 显式配置优先、等级过滤、`silent` 不写文件、`xarchive-*.log` 超过 `max_files`（1–100，默认 5）删除最旧、越界值回退默认并给出诊断 | 等级与轮转符合预期，权限错误可诊断且应用不崩溃 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |


## 历史 WDIO 队列核验（2026-09-15）

- WQ-P1-16 当前为 WINDOWS_FAIL：本轮 ordinary smoke 已执行，driver 下载成功但 Dashboard session 创建三次均报 `DevToolsActivePort file doesn't exist`，0/1 spec 通过；失败路径留下 driver/端口，手工清理后才恢复环境。
- WQ-P1-17 当前为 WINDOWS_FAIL：本轮 advanced 已执行，2/2 spec 均未创建 session，三次重试均报 `DevToolsActivePort file doesn't exist`；advanced plugin API、日志桥接、mock cleanup 和自动 teardown 未能完成验收。
- 具体命令、输出摘要、进程清理证据和 Linux 后续动作见 `../development/windows-validation.md` 的 2026-09-15 最新章节。当前 driver 下载不是阻塞根因；下一轮先调查 Windows WebView2/Edge native session 启动和自动 cleanup，不使用 WINDOWS_VERIFICATION_BLOCKING。

## 便携 runtime 队列说明（2026-09-16）

- WQ-P1-18/WQ-P1-19 对应本轮便携 `.exe` 布局、`config/config.yaml`、首次下载目录 setup、日志等级与轮转实现；Linux Rust/Node 门禁和便携目录组装 smoke 已通过，Windows 验证前保持 `WINDOWS_VERIFICATION_PENDING`，不阻塞后续 Linux 开发。
- 对应 Windows 执行步骤见 [`windows-wdio-handoff.md`](windows-wdio-handoff.md) 的 PORTABLE-W-01/PORTABLE-W-02 与第 10 节。

## Linux 详细测试结果与 Windows 后续步骤（2026-09-15）

本轮 Linux source 为 branch `dev`、HEAD `0537d32c9b4d2ef71ec508467d75378a767a34e7`，working tree dirty（包含本任务前已有的 WDIO 实现与文档修改）。环境为 Node v26.7.0/npm 11.19.0、Rust/Cargo 1.98.0、Python 3.14.4；未发现 `FAIL_PRODUCT` 或 `FAIL_TEST`。

| 层级/项目 | 状态 | 证据摘要 |
|---|---|---|
| static：WDIO scripts/config/spec syntax | `PASS` | `node --check` 覆盖 service adapter、wdio config、advanced wrapper、build wrapper、plugin spec；WDIO config load 和 service adapter/provider/app binary/spec 探针通过 |
| unit/integration：Node workspace | `PASS` | `npm run check`、`npm run test`、`npm run build`；Extension 7/7，Desktop Node test 0/0 |
| unit/integration：Rust workspace | `PASS` | fmt、workspace/all-targets check、`wdio-e2e` feature check、workspace tests 150/150、strict Clippy |
| unit/integration：Sidecar | `PASS` | compileall 通过，pytest 10/10 |
| packaging/build：普通 Tauri | `PASS` | `npm run build:tauri --workspace desktop`，生成 `target/release/xarchive-desktop` |
| packaging/build：`wdio-e2e` Tauri | `PASS` | `npm run build:tauri:wdio --workspace desktop`，生成 Linux release binary |
| browser_e2e：独立 Browser Mode | `NOT APPLICABLE` | 当前仓库没有独立 Browser Mode 配置或脚本；未临时创建测试架构 |
| native_e2e：Linux WDIO | `BLOCKED_AUTOMATION` | 一次低成本实际尝试；`webkit2gtk-driver` 缺失，service 安装 `tauri-driver` 后其启动 code 1，未进入 spec/session/teardown |

Windows 后续必须按以下顺序执行，Linux 结果不能替代其中任何一项：

1. 在 `E:\Shiraishi\VSCode Workspace\Tw2Tg` 检查同步 revision、工作树是否包含 dirty changes，并确认 Windows 依赖、`.venv`、WebView2 和 Edge driver 前置。
2. 设置并验证匹配的 `msedgedriver.exe`：`where.exe msedgedriver.exe`、`msedgedriver.exe --version`；优先使用已保存的 152.0.4191.66，并记录 PATH、版本和 SHA-256。
3. 运行 `npm ci --no-audit --no-fund`、`npm run check`、`npm run test`、`npm run build`、`npm run build:tauri:wdio --workspace desktop`，确认专用 artifact、`wdio` capability 和 guest JS 边界。
4. 设置 `WDIO_APP_BINARY`、`WDIO_ADVANCED=1`、`WDIO_CAPTURE_LOGS=1` 和 `WDIO_LOG_DIR`，运行 `npm run test:e2e:windows:advanced --workspace desktop`；确认 `window.wdioTauri`、`browser.tauri.execute`、invoke interception、mock/restore、前后端日志和非零失败退出码。
5. 无论 advanced 成功或失败，都检查 `xarchive-desktop`、`tauri-driver`、`msedgedriver` 进程以及 1420/4444/4445/9223 端口；不使用手工 `Stop-Process` 作为 PASS 证据。
6. 清理 advanced 专用环境变量，执行 `npm run build:tauri --workspace desktop` 和 `npm run test:e2e:windows --workspace desktop`；确认 Dashboard DOM、普通 artifact 不含 WDIO capability/guest JS，且无 `plugin:wdio|execute not allowed by ACL`。
7. 仅在上述 WDIO 不能稳定覆盖时，使用 Computer Use 验证原生文件选择器、托盘、通知、安装器、DPI/多显示器、拖放或视觉布局；工具不可用时记录 `BLOCKED_AUTOMATION` 并给出完整人工步骤。

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

### 最新重验结论（2026-09-15 17:20）

本轮从 Linux 最新 dirty working tree 经 /mnt/e 完成受控单向同步；Node/Rust 静态门禁和两套 Tauri build 通过。普通与 advanced WDIO 均实际启动 tauri-driver 并尝试创建 WebView2 session，但均因 `DevToolsActivePort file doesn't exist` 未进入 spec；失败路径均需手工清理 driver。两项保持 WINDOWS_FAIL，当前没有 WINDOWS_VERIFICATION_BLOCKING。
### 手动 driver 后的最新重验（2026-09-15 17:42）

Microsoft 官方 msedgedriver 152.0.4191.66 已下载到 E: 验证副本并经 PATH 发现；本轮 service 自动下载同版本 driver 成功，tauri-driver 也正常监听，但 advanced/ordinary session 均因 `DevToolsActivePort file doesn't exist` 失败。两项继续 WINDOWS_FAIL；手工清理残留 driver 不计为自动 teardown PASS。

### Blocker recovery follow-up (2026-09-15)

- `BLOCKED_ENV` 的实际阻塞已进一步收敛：Edge WebDriver 自身、tauri-driver `/status` 代理、直接启动 release app、driver PATH/显式路径和临时独立 identifier/profile 均已受控检查；最小 HTTP WebDriver probe 仍在 45 秒内超时，不能确认 session 创建。
- `BLOCKED_AUTOMATION` 的失败清理仍未修复：driver 残留可以按精确 PID 人工清理，但没有自动 teardown 证据；Computer Use 的 `sky` trusted RPC/native app inventory 仍不可用。
- 本轮没有修改业务代码、生产 Tauri 配置或依赖；E: 临时 probe/config/driver 副本已清理，普通 release artifact 已恢复。WQ-P1-16/WQ-P1-17 继续保持 `WINDOWS_FAIL`，待 Linux 后续处理测试生命周期/Windows native session 条件后重验。
- 人工操作指南与停止条件见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 “BLOCKED_ENV / BLOCKED_AUTOMATION blocker recovery analysis” 章节。

### 当前 dirty source 的 Windows WDIO 重验（2026-09-15 20:18）

- Build/Toolchain、Sidecar、Rust workspace 150/150、专用/普通 Tauri build、WDIO syntax/config load 和普通 artifact 的 capability/guest-JS 隔离均为 `PASS`。
- WQ-P1-17 advanced 为 `FAIL`：2 workers/2 specs，0 passed；最终 `EXIT_CODE=1`，错误为 `session not created: DevToolsActivePort file doesn't exist`。
- WQ-P1-16 ordinary 为 `FAIL`：1 spec，0 passed；最终 `EXIT_CODE=1`，同一 native session 错误。
- 两个失败路径都留下 `tauri-driver`/`msedgedriver` 和 4444/4445 或动态端口监听；按精确 PID 手工清理后恢复为无相关进程/端口，不能视为自动 teardown 通过。
- Computer Use native inventory 不可用（`sky` 未配置、`apps=[]`），GUI/DPI/键盘/屏幕阅读器项目保持 `BLOCKED_AUTOMATION`；真实账号、Named Pipe、应用级 SQLite/restart/recovery、ACL/reparse/长路径、externalBin 和发布流程保持各自 `BLOCKED`/`NOT RUN`。

### 最新 Windows 重验结论（2026-09-16，Linux `dev` HEAD `cb1e5816bcef7480c46b255782c586682ceab16c`）

- 本轮 Linux source working tree clean；受控同步到 `E:\Shiraishi\VSCode Workspace\Tw2Tg` 后关键文件 SHA-256 `24/24` 匹配。Windows Node/Rust/Sidecar 门禁、普通/专用 Tauri build、Debug startup、便携 artifact 组装与首启 SQLite/log 初始化均 `PASS`。
- WQ-P1-17 advanced 的 native session、Dashboard `2/2`、plugin API/execute、mock/restore 为 `PASS`；WQ-P1-16 ordinary Dashboard `2/2` 为 `PASS`。但两次成功退出后均留下 `tauri-driver`/`msedgedriver` 和 4444/4445，手工清理才恢复，因此 WQ-P1-16/WQ-P1-17 整体继续 `WINDOWS_FAIL`，不能把 spec PASS 外推为完整生命周期 PASS。
- WQ-P1-18 仅完成便携目录组装和进程/SQLite/log 初始化 smoke；首次下载目录交互、`config.yaml` 持久化、fallback、跨卷提交保持 `WINDOWS_VERIFICATION_PENDING`。WQ-P1-19 日志等级/轮转保持 `WINDOWS_VERIFICATION_PENDING`。
- 当前没有 `WINDOWS_VERIFICATION_BLOCKING`。Linux 后续优先处理 WDIO service/tauri-driver 自动 teardown；详细命令、PID、状态边界和未执行项目见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 2026-09-16 章节。

### Linux 修复与队列状态更新（2026-09-16）

- 根因定位：`@wdio/native-core` 的 `DriverProcess.stop()` 只对 tauri-driver 直接子进程执行 `SIGTERM`/`SIGKILL`，没有进程树清理（同库对 dev-server 使用 `taskkill /T /F`）；Windows 上 tauri-driver 及其 msedgedriver 子进程因此残留，4444/4445 持续监听。
- Linux 修复：`desktop/scripts/wdio-tauri-service.mjs` 的 launcher 在上游 teardown 前快照 driver PID 与驱动端口占用者，teardown 后对幸存进程执行进程树 kill（Windows `taskkill /T /F`、POSIX `SIGKILL`），无法清理时使运行失败；新增 `desktop/test/wdio-tauri-service.test.mjs`（`node --test` 8/8 通过），Linux Node 门禁 check/test/build 通过。业务 Rust、前端和生产 Tauri capability 无改动。
- WQ-P1-16/WQ-P1-17 修复后按 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md) §10 回到 `WINDOWS_VERIFICATION_PENDING`：下一轮 Windows 必须重跑 advanced 与 ordinary，且成功与失败退出路径均无 `tauri-driver`/`msedgedriver`/4444/4445 残留、无需手工 `Stop-Process`，才可改判 `WINDOWS_PASS`。
- WQ-P1-18/WQ-P1-19 保持 `WINDOWS_VERIFICATION_PENDING`，等待受控 Windows 交互/日志 fixture；本轮未将其提前改判。

### Windows 修复后重验结论（2026-09-16 11:20）

- Linux `dev` HEAD `a20027455651ef5f4f9faed527948bc1830375a6` 已按受控规则同步到 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；源工作树在验证开始时 clean，未将 E: 的依赖、target、driver、日志或用户数据反向同步。
- Rust/Node/Sidecar/build 基线均通过；portable `.exe` 可启动并创建 `config\archive.sqlite3` 与同级日志，验证结束后进程已清理。
- WQ-P1-17 advanced：native session、Dashboard 2/2、plugin API、execute、mock/restore 均通过；但 `onComplete` 仍报告 PID `23148` 未在 5 秒内确认退出（tracked survivors `23148, 40748`），因此整体为 `WINDOWS_FAIL`。
- WQ-P1-16 ordinary：native Dashboard 2/2 通过；但 `onComplete` 仍报告 PID `45032` 未在 5 秒内确认退出（tracked survivors `49032, 45032`），因此整体为 `WINDOWS_FAIL`。
- 两次命令退出后的立即复查均未发现 `tauri-driver`、`msedgedriver`、`xarchive-desktop` 或 4444/4445/1420/9223 LISTEN；这只能说明最终环境恢复，不能抵销 teardown hook 的失败证据。未使用手工 Stop-Process 作为通过条件。
- 新增 `desktop/test/wdio-tauri-service.test.mjs` 在 Windows 为 `7 passed, 1 failed`：`killTree` 子进程终止测试约 5.3 秒后断言失败；`npm run test` 因同一桌面测试失败而为 `FAIL`。该失败需要 Linux 后续处理，验证阶段不修改代码。

### Linux 修复与队列状态更新（2026-09-16 第二轮）

针对上一节 Windows 复验暴露的 teardown hook 误报与测试失败，Linux 端完成第二轮修复（仅测试基础设施，业务代码零改动）：

- **根因一（hook 误报）**：Windows 上 `taskkill /T /F` 报告成功后，OS 尚未完成回收，`kill(0)` 在确认窗口内仍把已终止的 driver PID 判为存活；两次运行的事后复查（无 `tauri-driver`/`msedgedriver` 进程、无 4444/4445 监听）证实进程实际已清理。固定 alive-check 窗口在 Windows 双向不可靠：既可把已死进程误判为活（本轮），PID 复用时也可把活进程误判为死。
- **根因二（测试失败）**：`killTree` 在 Windows 等待 taskkill 自身的 `close` 事件才 resolve，此时受害进程可能已发出 `exit` 事件；测试在 `killTree` 之后才挂 `once(child, "exit")` 监听器，事件已被错过，等待直至超时后断言失败（约 5.3 秒），与进程是否被杀无关。
- **修复内容**：`waitForProcessGone` 改为「child `exit` 事件（仍持有句柄时）→ 轮询（确认窗口 5s→10s，poll 250ms）→ 超时后以 tracked driver 端口是否仍 LISTEN 做最终仲裁」；被复用的 stale PID 无端口监听时不再使运行失败。`portListenerCheck` 在 win32 用 netstat 检查；POSIX 依赖 `kill(0)` 轮询（SIGKILL 后幸存者只能是僵尸进程，不占用端口）。`killTree` 测试改为在 `killTree` 之前挂 `exit`/`close` 监听。
- **队列状态**：WQ-P1-16/WQ-P1-17 依据上述修复回到 `WINDOWS_VERIFICATION_PENDING`；历史 `WINDOWS_FAIL` 证据全部保留在 windows-validation.md。

Windows 重验要求（在原要求之上补充）：

1. 成功与失败退出路径均无 `tauri-driver`/`msedgedriver`/4444/4445 残留、无需手工 `Stop-Process`（不变）；
2. teardown hook 必须无错误完成：允许出现 safety-net 警告（需作为证据记录），但不得再出现「PID 未在确认窗口内退出」导致的运行失败——若警告后 tracked 端口仍 LISTEN 则仍为 FAIL；
3. `desktop/test/wdio-tauri-service.test.mjs` 在 Windows 以 `8 passed, 0 failed` 通过。