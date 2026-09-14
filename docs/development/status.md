# 当前开发状态

> 本文记录当前实现事实，不替代逐轮验证报告，也不记录已经关闭的历史问题。

## 已实现

- Rust 核心 Job 状态、重试策略、TagEngine 和 Windows-safe 用户目录名。
- Native Host framing、forwarding 和错误处理已按职责拆分为独立模块，公共 API 保持不变。
- Sidecar Supervisor 已按进程监督、错误、事件和 stdout/stderr reader 拆分为独立模块，公共 API 保持不变。
- Protocol crate 已按 Browser、Sidecar、JSONL 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- Download crate 已按 model、router、RPC、HTTP client、supervisor 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- Storage crate 已完成模块化第一至第五批：`error.rs`、`models.rs`、`file_store.rs`、`metadata.rs`、`archive_service.rs` 以及 `database/users.rs`、`tags.rs`、`tweets.rs`、`jobs.rs`、`settings.rs`、`telegram.rs` 独立；Database connection 所有权、migration、事务和 public API 保持不变。
- Desktop Rust 已完成行为不变模块化：`aria2.rs` 独立负责 release allowlist、SHA-256 校验、程序发现/版本检测、Windows 下载解压和相关 Tauri commands；`archive.rs` 独立负责 `archive_tweet` 及归档编排；`commands.rs`、`runtime.rs`、`platform.rs` 分别负责 commands、RuntimeState 和平台命令边界；`lib.rs` 仅保留模块组合、请求模型、Tauri 入口/注册和测试入口。
- Desktop Rust 已完成 commands 低风险模块化：`commands.rs` 独立负责 App status、Sidecar 生命周期、Job 查询、archive root、文件夹打开、runtime health 和 Sidecar 配置解析；`archive_tweet`、RuntimeState 长锁和后台 Job executor 设计保持未改变。
- Desktop Rust 已完成 archive 模块化：`archive.rs` 负责 Browser user 绑定、Sidecar archive/download request/result、Browser relationship merge、Sidecar 下载事件处理、`archive_tweet` 编排、ArchiveService 提交、Job 事件/失败状态和安全错误映射；RuntimeState 所有权和现有长锁语义保持不变。
- Desktop Rust 已完成 runtime 边界的行为不变拆分：`runtime.rs` 独立负责 RuntimeState 数据结构、archive root/database 初始化和时间标记 helper；当前工作目录、`X-Archive`、SQLite 初始化失败状态和 RuntimeState 锁模型保持不变。
- Desktop `get_app_status` 现在报告 executor 生命周期状态：RuntimeState 初始化后的 bounded worker ownership 为 `ready`，RuntimeState 销毁后 worker 为 `stopped`；该状态反映 control worker ownership，真实 archive execution 使用独立 execution thread 执行长 Sidecar/FileStore I/O。
- Desktop executor commands 已完成 runner-owned submit wiring：`submit_executor_job` 使用独立 SQLite context 写入真实 BrowserTweet/user/Job/spec，随后由 `ProductionExecutionFactory` 按 `job_id` 加载 execution spec，独立创建 Database/FileStore/SidecarSupervisor/ArchiveExecutionJob；`query_executor_job`、`cancel_executor_job` 和 `shutdown_executor` 继续使用独立 persistence context。`archive_tweet` 保留为同步 fallback。
- Desktop Rust 已完成 platform 边界的行为不变拆分：`platform.rs` 独立负责 Explorer、macOS `open` 和 Linux `xdg-open` 命令选择；`open_archive_folder` 的 command API、路径参数和错误映射保持不变。
- R1 executor 已完成阶段二的 Linux production execution path：`archive_job_requests` 保存 immutable request JSON/schema/request_id，`ExecutorConfig`/`ProductionExecutionFactory` 由 runner 按 `job_id` 加载 spec，独立创建 Database、FileStore、SidecarSupervisor 和 `ArchiveExecutionJob`；`submit_executor_job` 不再从 RuntimeState lease Sidecar。`attempt_count` 防止 late result 覆盖新状态，control loop 与单 active runner 分离。RuntimeState 初始化时会启动 recovery scan；缺失 spec 会标记 `EXECUTION_SPEC_MISSING`。运行中 cancellation 会通过共享 token、Sidecar `cancel`/shutdown 和 attempt fencing 保护 `INTERRUPTED` 状态；真实 Windows 进程/文件锁行为和最终用户入口切换仍需后续验证/开发。
- R1 executor 当前 Linux contract 覆盖 bounded queue、重复 submit、query/cancel/shutdown、execution spec persistence、runner-owned context creation、recovery/completion、execution success/failure、terminal skip、attempt fencing、运行中 cancellation、资源 ownership 和 event ordering；同步 `archive_tweet` 仍保留为显式 fallback。
- Storage Job event query contract 已修正：`events.payload_json` 对状态变更事件允许为 NULL，`list_events_for_job` 现在以 `Option<String>` 表达该事实，并已由 Desktop SQLite contract adapter 回归验证。
- 版本化跨进程协议、JSON Schema、Native Messaging framing 和协议校验。
- Python gallery-dl Sidecar、JSONL worker、metadata 归一化和媒体文件事件。
- SQLite users、user names、tweets、media、jobs、events、tags、Telegram send state 和关系数据。
- staging → Rust 校验 → 最终归档目录的文件提交流程。
- Tweet/URL identity binding、Sidecar metadata identity binding、settings 输入限制和 Sidecar 错误脱敏。
- Tauri Desktop runtime、Job 查询、Sidecar lifecycle、aria2 discovery 和 React Dashboard。
- MV3 Extension 的 Tweet DOM 提取、归档按钮、状态查询和 Native Messaging bridge。
- Telegram request/formatter/transport contract、SecretStore abstraction 和幂等发送状态模型。

## 部分实现

- `DownloadRouter` 已完成跨平台策略和单元测试，但真实 aria2 fallback、403 后重新提取 URL、Desktop transfer lifecycle 尚未形成完整应用链路。
- Native Host 的 framing、校验和可插拔 forwarding 已完成；Windows Named Pipe server、ACL、Registry 和浏览器安装仍未完成。
- GUI 的源码级状态、语义结构、焦点样式和视觉 token 已完成；真实 WebView2、DPI、键盘、屏幕阅读器和对比度仍需 Windows 验收。
- Telegram 的跨平台 transport 和发送状态模型已完成；Credential Manager、真实账号和生产发送链路仍未完成。

## 未实现或未完成

- 同步 `archive_tweet` 到 executor 的最终产品入口切换仍未完成；executor 运行中 cancellation、真实 staging/final recovery action 已接入 Linux 生产路径并由回归测试覆盖。Windows 侧仍需验证实际子进程终止、文件锁、重启和打包行为。
- Windows Named Pipe server、Native Host manifest/Registry、Tray、Single Instance、Autostart 和 Credential Manager。
- Sidecar `externalBin`、正式 bundle、安装器、签名和 updater。
- 真实 Edge Cookie/X 认证归档和真实 Telegram 账号发送。
- 应用级旧 SQLite 启动迁移、重启恢复和跨用户 ACL 验证。

## 当前开发方向

1. 当前 R1 Linux-only contract validation 已完成：纯 Rust executor model、JobPersistence、Database factory、archive submit/query 对照、JobSummary projection、lifecycle event mapping、cancel/shutdown/recovery/completion、commit recovery facts/actions、批量 mixed recovery 和 SQLite 事件顺序均已完成并通过 Linux 验证。
2. 当前生产 executor integration 已完成 Linux 阶段二主体：execution spec persistence、runner-owned resource creation、单 active runner、attempt fencing、spec-driven execution、运行中 cancellation、filesystem facts/action 和 startup recovery scan；后续 Linux 任务是最终用户入口切换。
3. 阶段三用户入口切换暂不执行：当前 Extension/Native Host 仍通过独立协议链路，仓库没有可直接切换且已验证的 Desktop transport adapter；先保持同步 `archive_tweet` fallback 和显式 executor command。

## 验证状态

- Linux Rust `fmt/check/test` 已通过最终收口验证；`xarchive-desktop` 58 tests、workspace 其他 crate tests 全部通过。
- Node `check/test/build` 已通过最终收口验证；Extension tests 7/7。
- Python `compileall` 和 JSON Schema parse 已通过。
- Linux `cargo clippy --workspace --all-targets -- -D warnings`：`PASS`；已安装当前 stable toolchain 的 `clippy` component，版本为 `clippy 0.1.98 (88d9e12ae1 2026-08-18)`。
- Python `.venv/bin/python -m pytest sidecar/tests -q`：`PASS`，10 passed；使用仓库根目录 `.venv`、editable `sidecar` 安装和 pytest 9.1.1。`compileall` 同时通过。
- 本轮 recovery/cancellation 增量：storage `24` 个单测通过，新增 staging metadata recovery 场景通过；Desktop workspace 测试 `58` 个通过，包含运行中 cancel 与 late-result fencing，`cargo check` 与 clippy 通过。
- Tauri MCP Bridge 已配置为 Debug-only Rust 依赖并固定绑定 `127.0.0.1`；Library 入口通过 `#[cfg(debug_assertions)]` shadowing 注册，Release 构建不再产生 `unused_mut` warning。MCP server（`@hypothesi/tauri-mcp-server`）属于 Agent 环境工具，不提交到项目 `package.json`。
- 最新 Windows reconciliation（2026-09-14）：`WQ-P0-01` 保持 `WINDOWS_PASS`；项目 Python 前置下 Windows `144/144` workspace tests 通过，`Desktop 58` tests 覆盖 executor lifecycle/control、SQLite adapter、recovery decision/action、completion/failure、cancellation fencing 和 event ordering；Tauri MCP backend/window smoke `PASS`（`127.0.0.1:9223`）；默认无 `PYTHON` 的两项 `NotRunning` 单独保留为 environment FAIL。Tauri MCP WebView eval 层 `BLOCKED`（2 秒 timeout），应用级 executor real-worker integration、应用级旧库迁移/重启、reparse/ACL/长路径、bundle/packaging、真实账号和 Native Host/Named Pipe 仍为 `WINDOWS_VERIFICATION_PENDING`、`NOT RUN` 或 `BLOCKED`，不扩大为全量 Windows PASS。
- Windows 专属项目不得因 Linux 通过而标记为 Windows PASS；当前队列以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准，历史证据以 Windows 验证记录为准。

## 相关文档

- 未来方向：[`roadmap.md`](roadmap.md)
- 测试策略：[`testing.md`](testing.md)
- Windows 工作流：[`cross-platform-validation.md`](cross-platform-validation.md)
- Windows 执行规范：[`../validation/windows.md`](../validation/windows.md)