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
| `crates/xarchive-storage/src/` | `lib.rs` 负责 Database 连接、migration 和模块组合；`database/users.rs`、`tags.rs`、`tweets.rs`、`jobs.rs`、`settings.rs`、`telegram.rs` 分别负责对应 repository；`error.rs` 定义 StorageError；`models.rs` 定义公开 persistence/profile models；`file_store.rs` 负责 staging、profile、hash、commit 和 reparse/path 防护；`metadata.rs` 负责 Sidecar metadata 归一化；`archive_service.rs` 负责本地归档提交和 profile refresh；migration 位于 `crates/xarchive-storage/migrations/` | storage 单元和升级测试；repository 子模块共享 `Database.connection`，保持事务、migration 和 public API 不变 |
| `crates/xarchive-download/src/` | `lib.rs` 组合并 re-export 公共 API；`model.rs` 传输模型；`router.rs` gallery-dl/aria2 路由；`rpc.rs` JSON-RPC 请求/响应和解析；`client.rs` loopback HTTP client；`supervisor.rs` aria2 进程生命周期；`error.rs` 错误模型 | fake HTTP server、配置错误和路由测试；真实 aria2 集成另行验证 |
| `crates/xarchive-telegram/src/lib.rs` | SecretStore abstraction、Telegram request/transport、formatter 和幂等发送契约 | fake HTTPS server、脱敏、格式化和发送状态测试 |

## Desktop

| Path | 入口/职责 | 维护说明 |
|---|---|---|
| `desktop/src-tauri/src/main.rs` | Tauri native entry，调用 library `run()` | 保持极薄 |
| `desktop/src-tauri/src/lib.rs` | Tauri library 入口、RuntimeState、归档编排和 Tauri command 注册 | 保持入口与模块组合职责；RuntimeState 并发重构与行为移动分开 |
| `desktop/src-tauri/src/commands.rs` | App status、Sidecar 生命周期、Job 查询、archive root、文件夹打开和 runtime health commands；包含 Sidecar 配置解析与退出状态刷新 | 保持现有 command 名称、参数、返回值和 RuntimeState 锁语义；后台 Job executor 不在本模块化批次中实现 |
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