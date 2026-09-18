# 当前开发状态

> 本文记录当前实现事实，不替代逐轮验证报告，也不记录已经关闭的历史问题。

## 已实现

- 2026-09-18 Desktop UI 布局与依赖选择交互收口：工作台概览移除重复的“数据库”指标卡；修复 Sidecar/Extension 状态图标容器的文字样式串接和垂直居中；运行日志搜索占位符调整为较小字号；设置页 gallery-dl 未检测时仅显示选择按钮，选择后自动调用校验/保存并反馈成功或重新选择；aria2 移除可编辑路径输入框并将下载/选择/校验/保存操作合并到同一操作区；日志等级与最大日志文件数改为并排字段，重新调整 Extension、存储和运行环境间距。Linux Desktop Node 33/33、Vite build、Rust check 和 `git diff --check` 已通过；真实 Windows WebView2、原生文件对话框、DPI 和剪贴板仍待集中验证。

- 2026-09-18 Plan Linux 收口：确认 Browser Native transport 已通过 Unix endpoint 接入 `BrowserTransportAdapter` 和 `ArchiveApplicationService`，R1 的 request_id、重复提交、状态查询、协议错误和 executor error mapping contract 已由 Desktop tests 覆盖；同步 `archive_tweet` fallback 保留用于运行时回退。R2 真实 aria2 fallback 暂不接线：当前 Sidecar failure event 没有 fresh media URL，现有 `DownloadRouter` 不能自行重新提取或安全构造 `AddUriRequest`；需要先完成跨 Rust/Python/Schema 的 extraction-result contract。该项已记录为后续 Linux 设计任务，不归因于 Windows 环境。

- 2026-09-18 异步 submit/schedule Linux 收口：`submit_and_schedule_persisted` 已增加调度前 SQLite 预检；调度线程无法打开 persistence 或 production executor/factory 执行失败时，Job 会记录明确的 `FAILED`、错误码和 `DOWNLOAD_FAILED` 事件，而不是静默丢弃。新增无效数据库路径和后台 executor failure contract tests。Rust workspace 80 项 Desktop tests、Node workspace Desktop 33/33 + Extension 7/7、Sidecar pytest 12/12、普通/WDIO Tauri release build、fmt/check/strict Clippy 和 `git diff --check` 全部通过；Windows executor/Named Pipe/WebView2/process/filesystem 行为仍待集中验证。

- 2026-09-17 Windows worker follow-up：根据最新 Windows bundled worker `--help` 失败结果，移除 PyInstaller entrypoint 重复 `main()` 调用；Windows artifact workflow 在 smoke 前强制检查 `sidecar/dist/xarchive-downloader/_internal/python312.dll`；Core portable manifest 不再声明不存在的本地 Extension import 能力。Linux Python compile、Node contract、Rust/前端回归已完成；WQ-WORKER-BUILD-01、WQ-PACKAGE-CORE-02、WQ-PACKAGE-FULL-01 保持 `WINDOWS_VERIFICATION_PENDING`，等待新 artifact 和 Windows runtime 重验。

- 2026-09-17 GUI/统计/日志收口：Dashboard 新增由 SQLite 全量聚合的 `JobMetrics`（全部、进行中、已完成、失败），不再从最近 20 条任务推算；日志前端统一使用 `error/warning/info/debug/silent` 五档并移除 GUI Trace；Sidebar 服务状态行改为可键盘操作并可跳转到设置页对应区块；gallery-dl 与 aria2 路径支持 Tauri 原生文件选择器；Extension 本地导入入口移除，改为打开 GitHub `extension` 目录。Linux Rust 24 项 storage tests、Desktop Node 31 项、Vite build、cargo check/fmt 和 `git diff --check` 已通过；真实 Windows WebView2、原生对话框、DPI、剪贴板和浏览器集成继续待 Windows 验证。

- 2026-09-17 非 Windows Plan 收口：portable 包类型/组件规划/manifest 已抽为可测试纯逻辑；新增 Full/Core 契约测试、Sidecar `--gallery-dl` 参数回归、PyInstaller worker spec、独立入口和 Windows artifact workflow。Linux 适用测试已完成；真实 Windows worker、Desktop `.exe`、WebView2、文件权限、浏览器集成和发布证书继续进入 Windows Validation Queue。

- 2026-09-17 Full/Core portable 契约第一批：Sidecar 配置分离 XArchive worker 与外部 gallery-dl；Core 设置页支持校验/保存用户提供的 `gallery-dl.exe`，并通过 GitHub `extension` 目录外链和浏览器指南完成 Extension 加载；portable 构建脚本支持 `PORTABLE_PACKAGE_TYPE=full|core` 并生成 `package-manifest.json`。可信自动下载发布源尚未定义，因此不实现任意网络下载；Windows artifact、真实 gallery-dl、WebView2 文件路径和浏览器加载仍需验证。

- 2026-09-17 发布问题修复：Runtime 启动时独立初始化 `config/archive.sqlite3`，任务列表不再因首次下载目录尚未选择而报告 `archive database is not initialized`；设置页接入页面级 Error Boundary，避免渲染异常导致白屏；新增“运行日志”页面，通过 `read_application_logs` 每秒读取最新日志，支持等级筛选、搜索、自动跟随、复制和打开日志目录；Release 主程序启用 Windows GUI subsystem，Sidecar、aria2 和下载 supervisor 的 Windows 子进程统一使用 `CREATE_NO_WINDOW`。Linux 已验证，真实 Windows WebView2、窗口和剪贴板行为仍待验证。

- 2026-09-17 设置页组件批次（pre-2）：前端抽出共享 `Icon`、`CopyablePath`、`ConnectionStatus`/`ExtensionConnectionStatus` 组件和 `ui-state.js` 纯逻辑模块（显示名提取、aria2 状态文案、Extension 状态映射）；设置页 Sidecar gallery-dl 路径与 Extension 目录改为可复制路径组件（经 `copy_text_to_clipboard` Tauri 命令 + `arboard` 写入系统剪贴板，不使用 `navigator.clipboard`）；Sidebar Extension 状态改为显式枚举映射，文件缺失时显示"文件缺失"而不是永久"检测中…"；aria2 设置移除多版本下拉，改为"受信任最新官方版本 + SHA-256 校验 + 自动安装"语义（`latest_aria2_release`），并新增自定义 aria2 路径输入、`validate_aria2_path` 自动校验和 `save_aria2_path` 持久化到 `config.yaml`；Rust `AppStatus` 侧新增 `get_sidecar_path` 返回真实 gallery-dl 可执行文件路径。Linux 门禁全部通过（Node 23/23、vite build、cargo fmt/clippy -D warnings/test 72、check、diff --check）。
- 2026-09-17 Linux Clippy fix：`RuntimeState` 的 post-construction mutation 仅保留在 Unix 构建，Windows 保持不可变；修复 Windows 历史 `unused_mut`，Linux workspace strict Clippy 通过，WQ-P0-01 回到 `WINDOWS_VERIFICATION_PENDING`，不直接标记 `WINDOWS_PASS`。
- 2026-09-17 Windows reconciliation follow-up：修复 runtime 路径测试的 POSIX 硬编码；PyInstaller worker spec 改为与 workflow/portable/config 一致的 one-dir layout；Core portable 明确排除 gallery-dl。Linux fmt/clippy、Desktop 31/31 Node、Extension 7/7、Sidecar 12/12 和 spec syntax 验证通过。WQ-P0-01、worker artifact、Full/Core portable 重新保持 `WINDOWS_VERIFICATION_PENDING`，等待修复后 Windows 重验。
- 2026-09-17 Windows R2 reconciliation：Windows 复验确认 worker one-dir artifact 与 Core portable/start smoke 通过，但暴露 runtime root fixture 和 portable-package path suffix 两个测试契约问题。Linux 已改用相对路径 fixture 与 `path.join()`，workspace Rust、Desktop 31/31、Extension 7/7、Sidecar 12/12 和构建/语法检查通过。WQ-P0-01、worker artifact、Core portable 保持 `WINDOWS_VERIFICATION_PENDING` 等待重验；Full portable 继续因缺少受控 gallery-dl artifact 为 `WINDOWS_BLOCKED`。
- 2026-09-16 GUI 收口批次：工作台与设置页分离；Sidecar、aria2、归档位置、日志设置和 Extension 指南移入设置页；侧栏底部增加设置入口和服务状态；统一 Windows 本地字体栈、图标 SVG 容器、按钮焦点和响应式布局；aria2 文本 Logo 不再使用会导致 `a`/`2` 上下错位的隐式 Grid 行。
- 2026-09-16 Desktop portable path 修复：配置相对路径现在进行不依赖文件系统的词法归一化，`./logs` 显示为 `<portable-root>/logs`，并覆盖 database、cache、download、Sidecar 和 Extension 配置路径；新增嵌套 `.`/`..` 回归测试。
- 2026-09-16 Extension 基础检测：Desktop 新增 `get_extension_status` 和 `open_extension_folder`，检查 `manifest.json`、`src/background.js`、`src/content.js` 是否存在，并在设置页展示 Edge/Chrome 分步骤加载指南。浏览器实时连接和 Native Host 状态当前明确返回未验证边界，不外推为已连接。

- Rust 核心 Job 状态、重试策略、TagEngine 和 Windows-safe 用户目录名。
- Native Host framing、forwarding 和错误处理已按职责拆分为独立模块，公共 API 保持不变。
- Sidecar Supervisor 已按进程监督、错误、事件和 stdout/stderr reader 拆分为独立模块，公共 API 保持不变。
- Protocol crate 已按 Browser、Sidecar、JSONL 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- Download crate 已按 model、router、RPC、HTTP client、supervisor 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- Storage crate 已完成模块化第一至第五批：`error.rs`、`models.rs`、`file_store.rs`、`metadata.rs`、`archive_service.rs` 以及 `database/users.rs`、`tags.rs`、`tweets.rs`、`jobs.rs`、`settings.rs`、`telegram.rs` 独立；Database connection 所有权、migration、事务和 public API 保持不变。
- Desktop Rust 已完成行为不变模块化：`aria2.rs` 独立负责 release allowlist、SHA-256 校验、程序发现/版本检测、Windows 下载解压和相关 Tauri commands；`archive.rs` 独立负责 `archive_tweet` 及归档编排；`commands.rs`、`runtime.rs`、`platform.rs` 分别负责 commands、RuntimeState 和平台命令边界；`lib.rs` 仅保留模块组合、请求模型、Tauri 入口/注册和测试入口。
- Desktop Rust 已完成 commands 低风险模块化：`commands.rs` 独立负责 App status、Sidecar 生命周期、Job 查询、archive root、文件夹打开、runtime health 和 Sidecar 配置解析；`archive_tweet`、RuntimeState 长锁和后台 Job executor 设计保持未改变。
- Desktop Rust 已完成 archive 模块化：`archive.rs` 负责 Browser user 绑定、Sidecar archive/download request/result、Browser relationship merge、Sidecar 下载事件处理、`archive_tweet` 编排、ArchiveService 提交、Job 事件/失败状态和安全错误映射；RuntimeState 所有权和现有长锁语义保持不变。
- Desktop Rust 已完成便携 runtime 路径接入：`runtime.rs` 使用 portable root 派生 `config/`、`cache/`、`download/` 和同级 `logs/`；数据库位于 `config/archive.sqlite3`，临时 staging 位于 `cache/staging/`，最终归档位于用户确认的 `download/` 或系统 `Downloads/XArchive`。旧的进程工作目录 `X-Archive` 不是当前便携运行时布局。
- Desktop 已增加 `config/config.yaml` YAML 模型、首次下载目录 setup、日志等级和日志数量设置；Debug 构建默认日志等级为 `debug`，Release 默认 `info`，显式配置优先。Linux 已完成编译和单元测试；完整日志接入、动态级别切换和 Windows 文件权限仍需目标环境验证。
- Desktop `get_app_status` 现在报告 executor 生命周期状态：RuntimeState 初始化后的 bounded worker ownership 为 `ready`，RuntimeState 销毁后 worker 为 `stopped`；该状态反映 control worker ownership，真实 archive execution 使用独立 execution thread 执行长 Sidecar/FileStore I/O。
- Desktop executor commands 已完成 runner-owned submit wiring：`submit_executor_job` 使用独立 SQLite context 写入真实 BrowserTweet/user/Job/spec，并通过 `ArchiveApplicationService::submit_and_schedule_persisted` 立即调度后台执行；Browser transport 的 Unix production endpoint 复用同一 submit-and-schedule 路径。后台由 `ProductionExecutionFactory` 按 `job_id` 加载 execution spec，独立创建 Database/FileStore/SidecarSupervisor/ArchiveExecutionJob；`query_executor_job`、`cancel_executor_job` 和 `shutdown_executor` 继续使用独立 persistence context。`archive_tweet` 保留为同步 fallback。
- Desktop Rust 已完成 platform 边界的行为不变拆分：`platform.rs` 独立负责 Explorer、macOS `open` 和 Linux `xdg-open` 命令选择；`open_archive_folder` 现在打开用户确认后的最终 download root。
- R1 executor 已完成阶段二的 Linux production execution path：`archive_job_requests` 保存 immutable request JSON/schema/request_id，`ExecutorConfig`/`ProductionExecutionFactory` 由 runner 按 `job_id` 加载 spec，独立创建 Database、FileStore、SidecarSupervisor 和 `ArchiveExecutionJob`；`submit_executor_job` 不再从 RuntimeState lease Sidecar。`attempt_count` 防止 late result 覆盖新状态，control loop 与单 active runner 分离。RuntimeState 初始化时会启动 recovery scan；缺失 spec 会标记 `EXECUTION_SPEC_MISSING`。运行中 cancellation 会通过共享 token、Sidecar `cancel`/shutdown 和 attempt fencing 保护 `INTERRUPTED` 状态；真实 Windows 进程/文件锁行为和最终用户入口切换仍需后续验证/开发。
- R1 executor 当前 Linux contract 覆盖 bounded queue、重复 submit、query/cancel/shutdown、execution spec persistence、runner-owned context creation、recovery/completion、execution success/failure、terminal skip、attempt fencing、运行中 cancellation、资源 ownership 和 event ordering；同步 `archive_tweet` 仍保留为显式 fallback。
- Storage Job event query contract 已修正：`events.payload_json` 对状态变更事件允许为 NULL，`list_events_for_job` 现在以 `Option<String>` 表达该事实，并已由 Desktop SQLite contract adapter 回归验证。
- 版本化跨进程协议、JSON Schema、Native Messaging framing 和协议校验。
- `SidecarCommand` 现在通过 `serde(deny_unknown_fields)` 拒绝未声明的 per-request 字段（包括已移除的 `executable` override）；协议层 targeted regression 已覆盖该安全边界。Windows 真实 Sidecar、路径权限和 reparse/link 仍未完全验证，WQ-P1-12 当前状态以 Windows 队列为准。
- 2026-09-16 Windows 增量复验发现 Python worker 未拒绝带 `executable` 的未知字段；Linux 已在 worker 入口增加与 `download-command.schema.json` 对齐的允许字段检查，并新增 worker regression。WQ-P1-12 已恢复为 `WINDOWS_VERIFICATION_PENDING`，等待 Windows 重验；路径权限、reparse/link 和真实 Sidecar download 仍因缺少受控 fixture 保持 `NOT RUN`，详见 `windows-validation.md`。
- Python gallery-dl Sidecar、JSONL worker、metadata 归一化和媒体文件事件。
- SQLite users、user names、tweets、media、jobs、events、tags、Telegram send state 和关系数据。
- staging → Rust 校验 → 最终归档目录的文件提交流程。
- Tweet/URL identity binding、Sidecar metadata identity binding、settings 输入限制和 Sidecar 错误脱敏。
- Tauri Desktop runtime、Job 查询、Sidecar lifecycle、aria2 discovery 和 React Dashboard。
- Windows 便携运行时 Linux 侧实现：portable root、`config/config.yaml`、`config/archive.sqlite3`、`cache/`、同级 `logs/`、`download/` setup、`sidecar/gallery-dl`/`sidecar/aria2` 路径优先级、Extension 复制和 `build:portable:windows` 目录组装脚本。
- MV3 Extension 的 Tweet DOM 提取、归档按钮、状态查询和 Native Messaging bridge。
- Telegram request/formatter/transport contract、SecretStore abstraction 和幂等发送状态模型。

## 部分实现

- `DownloadRouter` 已完成跨平台策略和单元测试，Desktop 当前仅通过 Router 包裹 gallery-dl 结果并统一错误映射；真实 aria2 fallback、fresh media URL contract、403 后重新提取 URL、应用级 transfer lifecycle 尚未形成完整链路。不得将当前半接入状态标记为 R2 完成。
- Native Host 的 framing、校验和可插拔 forwarding 已完成；Windows Named Pipe server、ACL、Registry 和浏览器安装仍未完成。
- GUI 的源码级状态、语义结构、焦点样式和视觉 token 已完成；真实 WebView2、DPI、键盘、屏幕阅读器和对比度仍需 Windows 验收。
- Telegram 的跨平台 transport 和发送状态模型已完成；Credential Manager、真实账号和生产发送链路仍未完成。
- Desktop 已加入 WebdriverIO 9 + @wdio/tauri-service Windows automation baseline，并完成 tauri-plugin-wdio 1.4.0 的专用 wdio-e2e 配置；Linux service adapter 已加入。WQ-P1-16/WQ-P1-17 当前按队列保持 `WINDOWS_VERIFICATION_PENDING`，历史 teardown/session FAIL 仅作为历史证据保留，不能外推为当前 PASS。

## 未实现或未完成

- 同步 `archive_tweet` 到 executor 的最终产品入口退役仍未完成；Browser transport 与 `submit_executor_job` 已统一为立即返回初始 Job 状态并后台调度 production execution，但同步 fallback 仍保留。executor 运行中 cancellation、真实 staging/final recovery action 已接入 Linux 生产路径并由回归测试覆盖；用户主动取消与应用中断仍共用 `INTERRUPTED` 语义，Sidecar process-tree 终止和取消/提交竞争语义尚未收口。Desktop 已在 Linux/Unix 上注册生产 transport endpoint（Unix domain socket），Native Host 在 Linux 上改用 `UnixStream` 连接；Windows 侧仍需验证实际子进程终止、文件锁、重启和打包行为。Windows Named Pipe server 尚未注册，Native Host 在 Windows 上仍通过 `OpenOptions` 文件路径连接。
- Windows Named Pipe server、Native Host manifest/Registry、Tray、Single Instance、Autostart 和 Credential Manager。
- Sidecar `externalBin` 的最终分发行为、正式 bundle、安装器、签名和 updater。当前阶段只生成便携版 `.exe`，不生成 installer。
- 便携版 `.exe` 同目录的真实路径解析、`config/config.yaml` 持久化、cache→download 跨卷提交、系统 Downloads fallback、sidecar/aria2/gallery-dl/Extension 实际分发和 Windows 文件权限。
- 真实 Edge Cookie/X 认证归档和真实 Telegram 账号发送。
- 应用级旧 SQLite 启动迁移、重启恢复和跨用户 ACL 验证。

## 当前开发方向

1. 当前 R1 Linux-only contract validation 已完成：纯 Rust executor model、JobPersistence、Database factory、archive submit/query 对照、JobSummary projection、lifecycle event mapping、cancel/shutdown/recovery/completion、commit recovery facts/actions、批量 mixed recovery 和 SQLite 事件顺序均已完成并通过 Linux 验证。
2. 当前生产 executor integration 已完成 Linux 阶段二主体，并完成第一批入口调度统一：execution spec persistence、runner-owned resource creation、单 active runner、attempt fencing、spec-driven execution、独立 orchestration thread、调度失败补偿、运行中 cancellation、filesystem facts/action 和 startup recovery scan 均已接入；同步 `archive_tweet` fallback 仍保留，下一批处理用户入口退役和 cancellation/recovery 语义收口。
3. 阶段三的 Linux transport endpoint 已完成注册和 production scheduling 接入：Desktop 在 Linux/Unix 上启动 Unix domain socket transport endpoint，Native Host 在 Linux 上改用 `UnixStream` 连接；每个连接由独立线程处理，打开独立 SQLite persistence context，复用现有 `BrowserTransportAdapter` 完成请求校验、request_id 保留、Job submit-and-schedule、状态查询和错误映射。各 transport 回归测试（request_id 保留、重复 Job、状态查询、无匹配 Job、非法 Tweet URL 和非法协议版本）已通过。当前仍保留同步 `archive_tweet` fallback；Windows Named Pipe/ACL 仍需平台实现和实机验证。
4. R2 尚未进入可安全接线状态：下一 Linux 批次必须先定义 fresh media URL/403 refresh contract，再接入 aria2 backend、transfer polling/completion 和 Job event/state 提交；Windows 的 aria2 artifact、路径和进程验证在该批次完成后再按影响区重验。
5. Windows 自动化基线配置已实现：WDIO native smoke 可在已生成 Tauri release artifact 的 Windows 工作副本运行；真实 WebView2/DPI/键盘/辅助技术、应用 IPC 和 Native Host 仍需按队列验证。

## 验证状态

- Linux Rust `fmt/check/test/clippy` 已通过最终收口验证；`xarchive-desktop` 当前 80 tests、`xarchive-core` 12、`xarchive-native-host` 8、`xarchive-protocol` 11、`xarchive-sidecar-supervisor` 4、`xarchive-storage` 25、`xarchive-telegram` 12，workspace tests 全部通过。Desktop transport server（Unix domain socket）已接入 Desktop runtime，Native Host 在 Linux 上改用 `UnixStream::connect`。
- Desktop transport server 行为：Linux/Unix 下 Desktop 启动 Unix domain socket endpoint，Native Host 以 `XARCHIVE_PIPE_ENDPOINT` 配置连接；每个连接由独立线程处理，打开独立 SQLite persistence context，复用现有 `BrowserTransportAdapter` 完成请求校验、request_id 保留和错误映射。Windows 下 Native Host 仍保留 `OpenOptions` 文件打开路径，Desktop 不注册 Named Pipe server。
- 历史 Windows WDIO 复验：普通 release 的静态 capability/guest-JS 隔离检查通过，但旧版 service 配置曾轮询无 plugin 的普通 artifact；专用 artifact 的 Dashboard 2/2、plugin window.wdioTauri/browser.tauri.execute 2/2 和 mock 子项通过，teardown 曾输出 A sessionId is required for this command。Linux service adapter 已完成；历史失败保留在 windows-validation.md，当前 WQ-P1-16/WQ-P1-17 等待 Windows 前置稳定后重新验证。
- tauri-plugin-wdio 1.4.0 已完成 Linux 配置：可选 wdio-e2e Rust feature、专用 E2E 注册、独立 wdio capability、withGlobalTauri、条件 guest JS 导入、高级 E2E spec 和 `wdio-tauri-service.mjs` worker 适配。Linux Rust 与 Node 门禁已在当前 Linux 环境通过；Windows session/driver 生命周期重验仍未完成。
- Windows Node check/test/build、Extension tests 7/7、专用/普通 Tauri 构建和 Windows Rust fmt/check/test/clippy 已通过；历史中的 Node ENOMEM、DevToolsActivePort 和 session teardown 证据仍保留。2026-09-15 的“未执行完整 spec 验收”表述已被 2026-09-16 reconciliation 取代：advanced/ordinary spec 已实际执行并通过，失败收敛为 teardown 进程残留。
- Python `compileall`、Sidecar pytest 12/12 和 JSON Schema parse 已通过。
- Linux `cargo clippy --workspace --all-targets -- -D warnings`：`PASS`；已安装当前 stable toolchain 的 `clippy` component，版本为 `clippy 0.1.98 (88d9e12ae1 2026-08-18)`。
- Python `.venv/bin/python -m pytest sidecar/tests -q`：`PASS`，12 passed；使用仓库根目录 `.venv`、editable `sidecar` 安装和 pytest 9.1.1。`compileall` 同时通过。
- 本轮 recovery/cancellation 增量：storage `24` 个单测通过，新增 staging metadata recovery 场景通过；Desktop workspace 测试 `64` 个通过，包含运行中 cancel、late-result fencing 和 Browser transport contract，`cargo check` 与 clippy 通过。
- Tauri MCP Bridge 已配置为 Debug-only Rust 依赖并固定绑定 `127.0.0.1`；Library 入口通过 `#[cfg(debug_assertions)]` shadowing 注册，Release 构建不再产生 `unused_mut` warning。MCP server（`@hypothesi/tauri-mcp-server`）属于 Agent 环境工具，不提交到项目 `package.json`。
- 最新 Windows reconciliation（2026-09-16，HEAD `cb1e581`，working tree clean、SHA-256 `24/24` 同步匹配）：Windows Node/Rust/Sidecar 门禁、普通/专用 Tauri build、Debug startup、便携 artifact 组装与首启 SQLite/log 初始化 smoke 均 `PASS`；WQ-P1-17 advanced 与 WQ-P1-16 ordinary 的 native session、Dashboard `2/2`、plugin API/mock/restore 为 spec 级 `PASS`，但两次成功退出后 `tauri-driver`/`msedgedriver` 与 4444/4445 仍残留，故整体不能判 PASS。GUI/Computer Use 仍受自动化前置阻塞；便携交互 setup、Downloads fallback、跨卷提交、sidecar/Extension 分发、文件权限和日志轮转保持 `WINDOWS_VERIFICATION_PENDING`。
- Linux teardown 修复（2026-09-16）：根因为 `@wdio/native-core` 的 `DriverProcess.stop()` 只 kill 直接子进程、无进程树清理；`wdio-tauri-service.mjs` launcher 现在在上游 teardown 前快照 driver PID 与 4444/4445 端口占用者，teardown 后对幸存进程执行进程树 kill（Windows `taskkill /T /F`、POSIX `SIGKILL`），无法清理时使运行失败。新增 `desktop/test/wdio-tauri-service.test.mjs`（`node --test` 8/8 通过），Linux Node 门禁 check/test/build 通过。WQ-P1-16/WQ-P1-17 修复后回到 `WINDOWS_VERIFICATION_PENDING`，Windows 重验不得依赖手工 `Stop-Process`。
- Linux teardown 修复第二轮（2026-09-16）：Windows 复验显示 spec 全过且进程/端口事后均已干净，但 hook 的固定 alive-check 窗口在 Windows 误报（`taskkill /F` 成功后 `kill(0)` 仍判已终止 PID 存活），且 `killTree` 测试因 exit 事件监听挂晚在 Windows 失败（`7 passed, 1 failed`）。判定改为 exit 事件 → 轮询（10s）→ 超时后以 tracked driver 端口 LISTEN 状态最终仲裁；测试监听时机已修正。WQ-P1-16/WQ-P1-17 保持 `WINDOWS_VERIFICATION_PENDING`。
- Windows 专属项目不得因 Linux 通过而标记为 Windows PASS；当前队列以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准，历史证据以 Windows 验证记录为准。

## 相关文档

- 未来方向：[`roadmap.md`](roadmap.md)
- 测试策略：[`testing.md`](testing.md)
- Windows 工作流：[`cross-platform-validation.md`](cross-platform-validation.md)
- Windows 执行规范：[`../validation/windows.md`](../validation/windows.md)

## Windows 复验后的 Linux 端当前动作

依据最新 Windows 复验，Linux 端已完成明确的 WDIO wrapper、spec API 和构建边界配置；以下两项仍需 Windows 重验确认：

1. **已完成**：`desktop/scripts/run-wdio-advanced.mjs` 不再直接对 `wdio.cmd` 使用 `spawnSync`，改为由当前 Node 进程加载 workspace WDIO CLI，并保留真实失败退出码。
2. **已完成**：`desktop/e2e/specs/wdio-plugin.e2e.mjs` 不再调用不存在的 `browser.tauri.isTauriApiAvailable`，改为通过 `browser.tauri.execute` 检查 `window.wdioTauri`。
3. **已完成**：普通 release 不加载 `@wdio/tauri-plugin` guest JS；仅 `VITE_WDIO_E2E=1` 的专用构建加载该 guest JS，并继续使用 `wdio-e2e` feature/capability。
4. **待 Windows 重验**：确认普通 release 不再产生 `plugin:wdio|execute not allowed by ACL`，并处理/确认 `@wdio/tauri-service` 的 sessionId、mock-store 和 driver teardown 生命周期；手工 Stop-Process 只能作为诊断清理，不能作为 PASS 条件。

- 当前 revision 的 Rust fmt/check、wdio-e2e feature check、workspace tests、strict Clippy、Node check/test/build、wrapper/spec/config syntax、WDIO config load 和 Sidecar pytest 10/10 均已在 Linux 现场通过。Windows 已按“专用构建/高级入口 → 普通构建/基础 smoke”重新同步并执行；匹配 driver 下载和 tauri-driver 启动成功，但两套 native session 均因 `DevToolsActivePort file doesn't exist` 失败，service adapter 的完整效果仍未被 Windows 原生 session 验收。完整证据见 windows-validation.md 最新章节。

## Linux 端当前剩余验证与改动（2026-09-15）

本节是当前行动清单，不是历史验证流水账。Linux Rust 与 Node 门禁已通过；剩余工作集中在 Windows 前置稳定后的 service adapter 重验，以及其他 Windows 专属队列项目。

| ID | 状态 | Linux 端要求 | 完成标准 |
|---|---|---|---|
| LINUX-WDIO-07 | DONE-LINUX / WINDOWS_REVALIDATION_PENDING | 新增 `desktop/scripts/wdio-tauri-service.mjs`；普通和高级入口均复用官方 launcher，但 worker 跳过依赖 `plugin:wdio` 的单窗口 focus probe | 普通 smoke 不再轮询不存在的 plugin；专用 artifact 仍可 execute/mock/log；不放宽普通 capability 或 guest JS；需 Windows 重验 |
| LINUX-WDIO-08 | DONE-LINUX / WINDOWS_REVALIDATION_PENDING | 适配层覆盖 `afterSession`，只在有效 session 存在时删除 session；高级 spec 保留显式 mock restore，避免 service 重复清理 | 不再由项目适配层产生 sessionId warning；应用、tauri-driver、msedgedriver 和相关端口自动清理仍需 Windows 实测确认 |
| LINUX-WDIO-09 | DONE-LINUX | 2026-09-18 Linux 使用 Ubuntu 26.04、Node v26.7.0/npm 11.19.0、Rust/Cargo 1.98.0、Python 3.14.4 完成 Node workspace check/test/build、Extension 7/7、Rust fmt/check/wdio-e2e feature check/workspace tests 150/150/strict Clippy、Sidecar pytest 10/10、普通/专用 Tauri build、wrapper/spec/config syntax 和 WDIO config load；安装发行版替代包 `webkitgtk-webdriver` 后 Linux Native WDIO Dashboard smoke 2/2 PASS | 结果来自 Linux 工具链本身；Browser Mode 在当前仓库无独立配置，记为 NOT APPLICABLE；该 PASS 不替代 Windows WebView2 验证 |
| LINUX-WDIO-10 | WINDOWS_VERIFICATION_PENDING | Linux 复核已完成并确认无新的 Linux 侧实现问题；Windows 本轮 driver 下载/tauri-driver 启动成功，但 advanced 与 ordinary native session 均因 `DevToolsActivePort file doesn't exist` 失败，失败路径还需手工清理 driver | 调查并稳定 Windows WebView2/Edge native session 和自动 teardown 后，按 advanced → teardown/cleanup → ordinary smoke 重验；两个队列项满足各自完整验收条件后才可改为 WINDOWS_PASS |

当前不需要在 Linux 端重复或修改的项目：Rust fmt/check/feature check/test/clippy、Node check/test/build、Python compile/pytest、普通/专用 Tauri build、tauri-plugin-wdio 的 Cargo feature/注册/capability、withGlobalTauri、条件 guest JS、wrapper、service adapter、plugin spec 和 WDIO config load 已有通过或完成证据。Linux Native WDIO 已在安装 `webkitgtk-webdriver` 后通过 Dashboard smoke；当前仓库无独立 Browser Mode 配置，记为 `NOT APPLICABLE`。`DevToolsActivePort`、Edge driver 下载、`uv_os_get_passwd returned ENOMEM` 属于 Windows 验证环境前置，不应通过本轮 Linux 配置“顺带解决”。Native Host、Named Pipe、真实 executor/transport IPC、Windows ACL/reparse/长路径、WebView2 accessibility、真实账号和 installer 仍属于 Windows 队列。

### 当前 Windows 重验结论（2026-09-15 17:20）

Linux 最新 working tree 已经经 /mnt/e 受控单向同步到 E:；Node/Rust 静态门禁与专用/普通 Tauri build 通过。普通和 advanced WDIO 均在 onPrepare 因匹配 Edge driver 下载失败、随后 tauri-driver code 1 退出而未进入 spec；历史结果记录为 FAIL，当前队列状态改为 WINDOWS_VERIFICATION_PENDING，等待 Windows 前置稳定后重验。Linux Node 已在后续 reconciliation 中补做并通过。详见 docs/development/windows-validation.md 最新章节。
### Windows WDIO 网络重试状态（2026-09-15 17:42）

已从 Microsoft 官方地址手动取得 Edge/WebView2 152.0.4191.66 对应 msedgedriver，并确认 tauri-driver 可启动；但 advanced 与 ordinary worker 均因 Windows Node 的 uv_os_get_passwd returned ENOMEM 在 spec 前失败，WQ-P1-16/WQ-P1-17 继续 WINDOWS_FAIL。手动停止残留 driver 仅是环境恢复，不代表自动 teardown 通过。详细证据见 windows-validation.md 最新章节。
### Windows driver 本地保存状态（2026-09-15）

msedgedriver 152.0.4191.66 已半永久保存于 E: 验证副本的 desktop/test-artifacts/msedgedriver/152.0.4191.66/，并在 setup.md、testing.md 和 windows-wdio-handoff.md 记录 PATH 使用方法。该目录不纳入 Git、不回写 Linux；使用它只能绕过自动下载网络前置，不能覆盖当前 Node worker ENOMEM、session、DOM 或自动 teardown 的失败结论。

2026-09-15 blocker recovery：已在 E: 验证副本中确认同版本 `msedgedriver 152.0.4191.66` 能独立启动，`tauri-driver` status 代理可用，当前 release app 可直接保持存活；随后对新 WebView2 profile、隔离 application identifier 和显式 native driver 路径做最小 session probe，均仍以 45 秒超时结束，未解决 `Chrome instance exited`/`DevToolsActivePort file doesn't exist`。失败路径的 driver 可人工停止但自动 teardown 仍无证据；Computer Use 仍因 `sky` trusted RPC/native target 不可用而 `BLOCKED_AUTOMATION`。未修改业务代码；WQ-P1-16/WQ-P1-17 继续待 Linux 测试基础设施和 Windows native session 条件处理后重验。

## 当前 Windows WDIO 重验结论（2026-09-15 20:18）

本轮 Linux dirty source 已受控同步到 E:，source/E: 关键文件 hash 一致，Windows 本地依赖、target、driver、test-artifacts、用户数据和其他 machine-local 目录保留。Windows `npm ci`、Node workspace check/test/build、Sidecar compileall/pytest 10/10、Rust fmt/check/feature check/test 150/150/strict Clippy、专用/普通 Tauri build、WDIO syntax/config load 和普通 artifact capability/guest-JS 隔离均通过。

WQ-P1-17 advanced 与 WQ-P1-16 ordinary 均为 `WINDOWS_FAIL`：driver 下载成功，tauri-driver 监听成功，但 WebView2 session 创建三次重试均以 `session not created: DevToolsActivePort file doesn't exist` 失败，spec 未执行。两条失败路径都留下 driver/端口，需要精确 PID 的手工清理；这不是自动 teardown PASS，也没有形成产品 DOM/API 失败证据。Computer Use native inventory 返回 `apps=[]` 且 `sky` 未配置，GUI/DPI/辅助技术项目为 `BLOCKED_AUTOMATION`。

Linux 后续只需处理 Windows WDIO 测试基础设施/环境跟进：调查 `DevToolsActivePort`/`Chrome instance exited`、临时 WebView2 user-data-dir、tauri-driver 生命周期和自动 cleanup；保持普通 release capability/guest-JS 安全边界，不修改业务代码以制造通过。真实账号、Named Pipe/Registry、应用级 SQLite/restart/recovery、ACL/reparse/长路径、externalBin/Tray/installer 等仍按 Windows queue 的 `BLOCKED` 或 `NOT RUN` 原因处理。
