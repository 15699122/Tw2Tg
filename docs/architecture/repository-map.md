# 仓库文件职责地图

本文说明人工维护文件的职责、入口、运行关系和测试位置。生成物、缓存、依赖和机器本地数据不逐文件登记。

## 根目录

| Path | 职责 | 维护说明 |
|---|---|---|
| `README.md` | 用户和项目概览 | 不写验证历史和内部 Plan |
| `AGENTS.md` | Agent 强制开发与文档治理规则 | 只保留规则摘要，详细流程链接到 `docs/` |
| `Cargo.toml` | Rust workspace 成员和共享 lint/license | 新 crate 必须加入 workspace 并更新本地图 |
| `package.json` | Node workspaces 和根级命令 | 命令变化同步 `docs/development/setup.md` |
| `.env.example` | 非敏感本地配置示例 | 不放真实凭据 |
| `THIRD_PARTY_NOTICES.md` | 第三方运行时和依赖许可证说明 | 按实际分发内容维护 |

## Rust crates

| Path | 入口/职责 | 维护与测试 |
|---|---|---|
| `crates/xarchive-core/src/` | Job 状态、重试策略、TagEngine、稳定用户目录名和领域模型 | 纯 Rust 单元测试；不依赖 Tauri、SQLite 或平台 API |
| `crates/xarchive-protocol/src/` | `lib.rs` 组合并 re-export 公共 API；`browser.rs` 负责 Browser 消息和 Tweet 校验；`sidecar.rs` 负责 Sidecar 命令/事件；`jsonl.rs` 负责 JSONL 编解码；`error.rs` 负责协议错误 | 修改时同步 `shared/protocol-schema/`、Extension、Sidecar 和 Native Host |
| `crates/xarchive-native-host/src/` | Native Messaging framing、请求校验和 forwarding 核心；`error.rs` 错误、`framing.rs` 编解码、`forwarding.rs` 转发 | framing/transport fake 测试；Windows endpoint 在平台层验证 |
| `crates/xarchive-sidecar-supervisor/src/` | `lib.rs` 管理进程生命周期，`error.rs` 定义监督错误，`events.rs` 定义事件，`readers.rs` 解析 stdout/stderr | fake worker 与真实 Python worker 测试 |
| `crates/xarchive-storage/src/` | `lib.rs` 负责 Database 连接、migration 和模块组合；`database/users.rs`、`tags.rs`、`tweets.rs`、`jobs.rs`、`settings.rs`、`telegram.rs` 分别负责对应 repository；`error.rs` 定义 StorageError；`models.rs` 定义公开 persistence/profile models；`file_store.rs` 负责 staging、profile、hash、commit 和 reparse/path 防护；`metadata.rs` 负责 Sidecar metadata 归一化；`archive_service.rs` 负责本地归档提交和 profile refresh；`jobs.rs` 的事件查询正确表达可为空的 `payload_json`；migration 位于 `crates/xarchive-storage/migrations/` | storage 单元和升级测试；repository 子模块共享 `Database.connection`，保持事务、migration 和 public API 不变 |
| `crates/xarchive-download/src/` | `lib.rs` 组合并 re-export 公共 API；`model.rs` 传输模型；`router.rs` gallery-dl/aria2 路由；`rpc.rs` JSON-RPC 请求/响应和解析；`client.rs` loopback HTTP client；`supervisor.rs` aria2 进程生命周期；`error.rs` 错误模型 | fake HTTP server、配置错误和路由测试；真实 aria2 集成另行验证 |
| `crates/xarchive-telegram/src/lib.rs` | SecretStore abstraction、Telegram request/transport、formatter 和幂等发送契约 | fake HTTPS server、脱敏、格式化和发送状态测试 |

## Desktop

| Path | 入口/职责 | 维护说明 |
|---|---|---|
| `desktop/src-tauri/src/main.rs` | Tauri native entry，调用 library `run()` | 保持极薄 |
| `desktop/src-tauri/src/lib.rs` | Tauri library 入口、模块组合、`ArchiveTweetRequest`、`run()`、Tauri command 注册和 Debug-only localhost MCP Bridge 注册 | 保持入口与模块组合职责；MCP Bridge 只在 Debug 构建注册并绑定 `127.0.0.1`；不承载归档、RuntimeState、平台或 aria2 业务实现 |
| `desktop/src-tauri/src/commands.rs` | App status（包含 database、Sidecar 和 executor 生命周期状态）、Sidecar 生命周期、Job 查询、executor submit/query/cancel/shutdown commands、archive root、文件夹打开和 runtime health commands；submit 负责短事务写入 BrowserTweet/user/Job/spec，真实 worker context 由 ExecutorRuntime 创建 | 保持 command API；submit 不得 lease 生产 Sidecar；真实 SQLite/Sidecar/FileStore/ArchiveService I/O 不得放回 RuntimeState 全局锁；同步 archive_tweet fallback 可保留旧 ownership |
| `desktop/src-tauri/src/archive.rs` | `archive_tweet` fallback、`ArchiveExecutionContext` resource bundle、`ArchiveExecutionJob` execution-port adapter、Browser user/relationship merge、DownloadRouter/Sidecar archive/download、ArchiveService 提交、Job 事件/失败状态和安全错误映射 | 保持 metadata identity binding、文件事件归一化和错误脱敏；`ArchiveExecutionJob` 由 executor factory 或同步 fallback 消费，context 必须保持独立资源 ownership |
| `desktop/src-tauri/src/executor.rs` | R1 Job executor command/state model；`ExecutorConfig`、`ProductionExecutionFactory`、`ExecutorRuntime`、`ArchiveApplicationService`、`JobExecutorHandle`、`JobPersistence`/`JobExecution` ports、独立 SQLite adapter、execution spec persistence、attempt fencing、recovery/completion/cancel/shutdown、运行中 cancellation、`ArchiveJobSubmissionAdapter`、execution result/error contract、ExecutorEvent/JobEvent 映射、bounded control worker 和 single active runner | `ExecutorRuntime` 由 `RuntimeState` 持有，但 runner 通过 `ProductionExecutionFactory` 自主打开 Database/FileStore/Sidecar 并创建 ArchiveExecutionJob；control worker 不执行长 I/O；startup recovery 读取 final/staging facts 并执行 commit action；Windows 实际进程终止和文件锁行为仍需平台验证 |
| `desktop/src-tauri/src/transport.rs` | Browser `ArchiveRequest`/`QueryStatus` 到 executor application service 的协议 transport adapter；统一 BrowserRequest 校验、request_id 保留、Job submit/query 响应和错误映射 | 当前作为 crate 内 contract adapter 和 Linux 单测边界维护；尚未接入 Native Host/Named Pipe 或替换现有同步入口；接入前必须完成跨进程 transport wiring 和 Windows 端到端验证 |
| desktop/wdio.conf.mjs | WebdriverIO 本地 runner 与 @wdio/tauri-service 配置；根据 `WDIO_ADVANCED` 选择普通 smoke 或高级 plugin spec，解析 Tauri binary、Windows external Edge WebDriver、driver 端口、日志和环境变量 | 普通 smoke 不启用高级 spec；高级 spec 使用 `wdio-e2e` artifact；Windows 原生验收结果写入验证文档 |
| `desktop/scripts/run-wdio-advanced.mjs` | 跨平台启动高级 WDIO E2E，使用当前 Node 进程加载 workspace 的 WDIO CLI，并设置 `WDIO_ADVANCED`/`WDIO_CAPTURE_LOGS` | 不直接调用平台 `.cmd` shim；保持 Windows PowerShell/CMD 与 Linux 命令行为一致，并透传 runner 退出码 |
| `desktop/scripts/build-tauri-wdio.mjs` | 跨平台启动 `wdio-e2e` 专用 Tauri 构建，使用当前 Node 进程加载 Tauri CLI，并设置 `VITE_WDIO_E2E=1` | 不直接调用平台 `.cmd` shim；专用构建才注入 WDIO guest JS 和 `wdio-e2e` feature |
| `desktop/src-tauri/capabilities/wdio.json` | 专用 WDIO capability，授予 `wdio:default` 和测试窗口权限 | 不加入默认 capability；仅随 Debug/专用 E2E 验证使用 |
| `desktop/src-tauri/tauri.wdio.conf.json` | 高级 WDIO 构建的配置 overlay，只选择 `wdio` capability | 通过 `build:tauri:wdio` 使用；普通构建只选择 `default` |
| `desktop/e2e/specs/dashboard.e2e.mjs` | Tauri 原生窗口最小 DOM smoke，验证 Dashboard heading、main、导航和概览区域 | 必须在已构建的 Windows Tauri artifact 上运行；不能替代 Native Host、真实 IPC、DPI 或辅助技术验证 |
| `desktop/e2e/specs/wdio-plugin.e2e.mjs` | 高级 Tauri plugin E2E，验证 plugin availability、frontend execute、command mocking 和 cleanup | 必须使用 `build:tauri:wdio` artifact；Windows 日志桥接和 WebView2 行为仍需平台验证 |
| `docs/validation/windows-wdio-handoff.md` | 当前 Linux WDIO 配置完成后的 Windows handoff；记录同步、专用构建、advanced E2E、teardown、普通 release 回归和结果回写步骤 | 只描述待执行步骤，不记录虚构结果；Windows 结果仍写入 `docs/development/windows-validation.md`，队列状态仍以 `docs/validation/windows-queue.md` 为准 |
| `desktop/src-tauri/src/runtime.rs` | RuntimeState 数据结构、archive root/database 初始化和时间标记 helper | 保持当前工作目录下 `X-Archive` 路径、SQLite 初始化失败状态和字段所有权；后台 Job executor 与锁模型另行设计 |
| `desktop/src-tauri/src/platform.rs` | 平台相关的 archive folder 打开命令选择（Explorer、open、xdg-open） | 保持平台命令和路径参数边界；平台实机行为由 Windows/桌面验证队列确认 |
| `desktop/src-tauri/src/aria2.rs` | aria2 release allowlist、SHA-256 校验、可执行文件发现/版本检测、Windows 下载解压和 aria2 Tauri commands | 保持官方版本 allowlist、错误脱敏和 Windows-only 下载边界；真实 aria2 业务集成仍由 Windows 队列验证 |
| `desktop/src-tauri/migrations/` | 不再使用；migration ownership 已迁移到 storage crate | 不应重新添加 migration |
| `desktop/src/main.jsx` | React Dashboard 当前入口和 Widget 组合 | 后续拆为 App、API、hooks、components 和 formatting |
| `desktop/src/style.css` | Dashboard 全局样式和设计 token | 视觉变更同步 Windows GUI 队列 |
| `desktop/src/lib/utils.js` | 前端共享工具 | 保持无 Tauri 状态依赖 |
| `desktop/src-tauri/tauri.conf.json` | Tauri build、窗口、CSP 和 bundle 配置 | bundle 当前关闭，不能假设存在安装器 |

## Browser Extension

| Path | 职责 | 维护说明 |
|---|---|---|
| `extension/manifest.json` | MV3 权限、host、content script 和 service worker 声明 | 遵循最小权限；权限变化需安全审查 |
| `extension/src/content-core.js` | 纯 DOM Tweet/quote/reply 提取 | 不访问 Cookie、文件或 Tauri |
| `extension/src/content.js` | 页面注入、按钮和 MutationObserver | 只调用 background bridge |
| `extension/src/background.js` | Native Messaging bridge、request_id 路由和状态请求 | 与 browser protocol/schema 同步维护 |
| `extension/tests/` | DOM、bridge、断线和消息测试 | 新消息字段必须增加契约测试 |

## Python Sidecar

| Path | 职责 | 维护说明 |
|---|---|---|
| `sidecar/src/xarchive_downloader/__init__.py` | 当前 worker 公共入口和 JSONL loop | 后续拆为 `worker.py`、`protocol.py` 和公共导出 |
| `sidecar/src/xarchive_downloader/gallery.py` | gallery-dl command 构造和执行 | 只接收可信运行时配置，不接受 per-request executable override |
| `sidecar/src/xarchive_downloader/models.py` | gallery-dl metadata 和文件结果归一化 | 与 protocol metadata identity 规则同步 |
| `sidecar/src/xarchive_downloader/errors.py` | gallery-dl 错误分类 | 对外错误必须保持稳定、安全、有限长度 |
| `sidecar/tests/` | Worker、gallery adapter 和 metadata 测试 | 运行时依赖项目 Python 环境和 pytest |

## Protocol schemas and fixtures

| Path | 职责 |
|---|---|
| `shared/protocol-schema/*.schema.json` | Rust、JavaScript、Python 之间的字段和边界契约 |
| `shared/protocol-schema/fixtures/` | 跨语言有效/无效消息、aria2 response 和 JSONL 样例 |

修改 Schema 时必须检查所有 producer、consumer、fixture 和相关测试。

## 文档与验证

| Path | 职责 |
|---|---|
| `docs/architecture/` | 稳定架构、数据模型、运行流、ADR 和文件地图 |
| `docs/architecture/runtime-flow.md` | 当前浏览器、Desktop、Sidecar、storage、download 和 Telegram 运行流 |
| `docs/development/status.md` | 当前实现状态 |
| `docs/development/roadmap.md` | 未来方向和完成标准 |
| `docs/development/testing.md` | 测试策略和命令 |
| `docs/development/risk-register.md` | 当前仍有效的风险、状态、责任模块和验证入口 |
| `docs/development/cross-platform-validation.md` | 跨平台开发/验证流程 |
| `docs/validation/windows.md` | Windows 验证规范和报告模板 |
| `docs/validation/windows-queue.md` | 当前 Windows Validation Queue 的唯一事实源 |
| `docs/development/windows-validation.md` | 历史 Windows 验证记录和兼容入口 |
| `aidlc-docs/inception/` | 初始需求、设计和工作包快照，不覆盖当前代码事实 |

## 不登记为人工维护源文件的内容

以下内容由工具生成或属于机器本地状态，不应写入 repository map 的功能职责表：

- `node_modules/`、`.venv/`、`target/`、`dist/`、`.vite/`；
- `__pycache__/`、`.pytest_cache/`、IDE cache；
- `X-Archive/`、SQLite、日志和 secrets；
- Tauri `gen/` 和本地构建 artifacts；
- `Cargo.lock`、`package-lock.json` 等锁文件只需在依赖变更时更新。