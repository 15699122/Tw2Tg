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
- Desktop `get_app_status` 现在报告 executor 生命周期状态：RuntimeState 初始化后的 bounded worker ownership 为 `ready`，RuntimeState 销毁后 worker 为 `stopped`；该状态仅用于 control-plane observability，不表示真实归档 I/O 已迁移到 executor。
- Desktop 已注册最小 executor control commands：`submit_executor_job`、`query_executor_job`、`cancel_executor_job` 和 `shutdown_executor`。这些 commands 使用独立 SQLite persistence context，避免在 RuntimeState 锁内执行数据库操作；submit 在 worker 不可用时以 `EXECUTOR_UNAVAILABLE` 补偿失败，shutdown 先持久化 active Job 的 `INTERRUPTED` 状态和事件再关闭 worker；当前只完成 Job control/persistence，不执行 Sidecar、FileStore 或 ArchiveService I/O，`archive_tweet` 仍是同步 fallback。
- Desktop Rust 已完成 platform 边界的行为不变拆分：`platform.rs` 独立负责 Explorer、macOS `open` 和 Linux `xdg-open` 命令选择；`open_archive_folder` 的 command API、路径参数和错误映射保持不变。
- R1 后台 Job executor 的设计边界已记录在 ADR-009，并已进入分阶段运行时接入：`RuntimeState` ownership、executor lifecycle status、最小 Tauri control commands 和同步 fallback 对照已完成；真实 Sidecar/FileStore worker I/O、completion/recovery command path 和 fallback 切换仍未完成。
- R1 第一实现阶段已完成并扩展到最小 control plane：`desktop/src-tauri/src/executor.rs` 提供 `ExecutorRuntime`、纯 Rust command/state model、`ArchiveApplicationService`、`JobPersistence` port、`JobDatabaseFactory`、`CommitRecoveryFactsProvider`/`InMemoryCommitRecoveryFactsProvider`、`CommitRecoveryBatchResult`、`ArchiveJobSubmissionAdapter`、`JobSummary`→`JobSnapshot` 字段投影、纯 Rust 恢复决策/动作、in-memory/SQLite contract adapter、`ExecutorEvent` lifecycle model 和核心 `JobEvent` 映射，覆盖 bounded queue、重复 submit、query、queued cancel、persisted cancel、shutdown、persisted shutdown、recovery scan、active/interrupted candidate、terminal skip、queued/interrupted recovery source state、`DOWNLOADED → COMPLETE` completion、重复 completion no-op、commit 前后退出 recovery decision、facts/snapshot 一致性校验、批量 recovery 稳定排序、混合动作结果、SQLite 状态/事件/错误字段顺序与单 Job 错误隔离、创建/下载开始/下载完成/下载失败/完成事件、事件顺序、fake Sidecar crash、`SIDECAR_INTERNAL_ERROR`、事务性状态事件去重、SQLite Job repository contract、独立 Job database context、同步 archive 入口的 request validation/Job identity 对照、Storage 错误字段保留、单 worker invariant、JobState transition boundary 以及 Tauri submit/query/cancel/shutdown control command boundary；`archive.rs` 已增加 State-independent `ArchiveExecutionContext`，统一 Database、FileStore 和 SidecarSupervisor 的资源 bundle，并由同步 fallback 使用；真实 executor worker I/O 尚未接入。
- executor 已增加 `JobExecution` worker execution port、`JobExecutionResult`/`JobExecutionError` result contract 和 `execute_persisted` persistence/event adapter；Linux fake execution 已覆盖成功 completion、失败隔离、terminal skip 和 lifecycle event 顺序，`ArchiveExecutionJob` 已实现真实 context-to-port adapter，包含 identity、download、commit 和 event/error mapping，但尚未由后台 worker 或 Tauri executor command 消费。
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

- Desktop 应用级后台 Job executor、取消和并发模型；control command 的持久化 submit/query/cancel/shutdown boundary 已完成，但真实 Sidecar/FileStore worker I/O 尚未迁移，当前 `archive_tweet` 仍在 Tauri command 生命周期内持有全局 RuntimeState 锁执行长时间 Sidecar I/O。
- Windows Named Pipe server、Native Host manifest/Registry、Tray、Single Instance、Autostart 和 Credential Manager。
- Sidecar `externalBin`、正式 bundle、安装器、签名和 updater。
- 真实 Edge Cookie/X 认证归档和真实 Telegram 账号发送。
- 应用级旧 SQLite 启动迁移、重启恢复和跨用户 ACL 验证。

## 当前开发方向

1. 当前 R1 Linux-only contract validation 已完成：纯 Rust executor model、JobPersistence、Database factory、archive submit/query 对照、JobSummary projection、lifecycle event mapping、cancel/shutdown/recovery/completion、commit recovery facts/actions、批量 mixed recovery 和 SQLite 事件顺序均已完成并通过 Linux 验证。
- 2. 当前生产 executor integration 的 Linux-independent contract 已收口：同步 `archive_tweet` 使用 `ArchiveExecutionContext`，`ArchiveExecutionJob` 已可连接 `JobExecution` port；尚未将该 adapter 接入后台 worker，也未改变 Tauri command 的同步 fallback。后续开发任务是原子切换 SidecarSupervisor/FileStore/ArchiveService I/O、completion/recovery 和 worker startup ownership。
3. Linux development phase 已进入最终 diff 与 Windows handoff preparation；Windows 专属平台、真实 I/O、打包和账号链路统一在后续集中验证。

## 验证状态

- Linux Rust `fmt/check/test` 已通过最终收口验证；`xarchive-desktop` 57 tests、workspace 其他 crate tests 全部通过。
- Node `check/test/build` 已通过最终收口验证；Extension tests 7/7。
- Python `compileall` 和 JSON Schema parse 已通过。
- Linux `cargo clippy --workspace --all-targets -- -D warnings`：`NOT RUN`，当前 toolchain 未安装 `cargo-clippy`。
- Python `pytest sidecar/tests -q`：`NOT RUN`，当前环境未安装 `pytest`。
- Windows 专属项目不得因 Linux 通过而标记为 Windows PASS；当前队列以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准，历史证据以 Windows 验证记录为准。

## 相关文档

- 未来方向：[`roadmap.md`](roadmap.md)
- 测试策略：[`testing.md`](testing.md)
- Windows 工作流：[`cross-platform-validation.md`](cross-platform-validation.md)
- Windows 执行规范：[`../validation/windows.md`](../validation/windows.md)