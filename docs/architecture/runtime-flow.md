# 运行流

本文描述当前代码中的主要运行路径，并单独标记目标架构；不把未来路径写成当前实现。

## 当前浏览器归档请求

```text
X 页面
  → Extension content script 提取 BrowserTweet
  → Extension background service worker 创建 BrowserRequest
  → Native Messaging Host framing/校验
  → Desktop transport endpoint（Linux/Unix socket；Windows Named Pipe backend）
  → BrowserTransportAdapter / `submit_executor_job`
  → `ArchiveJobSubmissionAdapter` 校验 BrowserRequest 并派生稳定 Job identity
  → 独立 SQLite context 创建/复用 Job，并写入 BrowserTweet/user
  → submit-and-schedule 返回初始 Job 状态
  → executor orchestration thread 打开独立 SQLite context
  → `ArchiveExecutionContext` / `ArchiveExecutionJob`
  → executor control worker 派发到独立 execution thread
  → Sidecar v2 `hello`/`extract`、gallery-dl extraction-only、aria2 transfer
  → cache/staging/ 下的 aria2 staging 文件
  → Rust 检查 metadata、路径、文件大小/reparse 和 SHA-256
  → ArchiveService 提交最终目录和数据库状态
  → list_jobs / BrowserResponse 返回状态
```

Native Host 的 framing 和 forwarding 核心位于 `crates/xarchive-native-host/`；U12 的 `desktop/scripts/native-host-package.mjs` 只生成并校验 Native Messaging host manifest 与安装布局契约，不执行 Registry 或浏览器安装。Linux/Unix Desktop endpoint 由 `desktop/src-tauri/src/transport.rs` 提供，Windows Named Pipe server、Registry、ACL 和浏览器 reload 仍属于平台适配边界，不由跨平台 framing 代码决定。

## WebSocket 本地通道（目标迁移路径）

目标链路为：

```text
X 页面
  → Extension content script
  → Extension background WebSocket bridge
  → authenticated loopback WebSocket
  → Desktop WebSocket listener
  → BrowserTransportAdapter
  → 现有 executor / SQLite / Sidecar 链路
```

WebSocket 使用独立 transport envelope 完成发现、认证和连接生命周期；业务消息仍严格使用 `BrowserRequest` / `BrowserResponse`，不新增归档业务命令。认证失败、连接断开、Desktop 未启动和端口发现失败必须在 Extension GUI 中可区分；断线时 pending request 明确失败。listener 必须加入 `RuntimeState` 的启停和 `replace_executor()` 代际切换，迁移期间保留 Native Messaging 回退。协议和 ADR 见 [`../architecture/decisions.md`](../architecture/decisions.md) ADR-014 与 [`../protocols/overview.md`](../protocols/overview.md)。


```text
desktop/src-tauri/src/main.rs
  → desktop/src-tauri/src/lib.rs::run()
  → RuntimeState::initialize()
  → portable root / config / cache / logs 路径解析
  → 读取 config/config.yaml
  → 检查 download 或 system Downloads/XArchive
  → FileStore 初始化 download root 与 cache/staging root
  → Database::open() 在 config/archive.sqlite3 应用 migrations
  → ExecutorRuntime 创建 bounded worker ownership 和 database context path
  → Tauri command registry
  → React Dashboard invoke commands
```

### Frontend bootstrap 与白屏诊断边界

Desktop native runtime 初始化不等于 WebView 前端已可用。当前启动链路按以下阶段诊断：

```text
Tauri window created
  → WebView document/navigation
  → index.html startup fallback visible
  → frontend entry module evaluated
  → React root mount
  → initial Tauri IPC requests
  → Dashboard shell visible
```

前端 bootstrap 必须在主 App imports 和 `createRoot()` 之前安装全局错误捕获，并记录有限的 startup markers。`index.html` 提供不依赖 React 的启动反馈；React root ErrorBoundary 负责 App shell render failure，页面级 ErrorBoundary 继续隔离单页异常。启动失败不得退化为无提示白屏。

`application runtime initialized` 只表示 `RuntimeState::initialize()` 已进入日志阶段，不能作为 asset load、WebView document、React mount 或 Dashboard readiness 的证明。frontend diagnostic event 可通过受限 Tauri command 写入应用日志，但不得携带凭据、signed URL、归档内容或无限长度 stack。

当前 Desktop 已按 `archive.rs`、`commands.rs`、`runtime.rs`、`platform.rs` 和 `aria2.rs` 完成模块化；Browser transport 与 `submit_executor_job` 都通过 `ArchiveApplicationService` 提交并调度 production Job，返回初始状态而不等待完整归档。U8 已删除同步 `archive_tweet` fallback，executor 命令是唯一业务入口；runner 从持久化 execution spec 自主创建 Database、FileStore 和 SidecarSupervisor，不依赖 RuntimeState 的 Sidecar lease；RuntimeState 不在长时间 Sidecar/FileStore I/O 期间持锁。

便携运行时的目录边界为：最终归档使用 `download/`，临时 staging 使用 `cache/staging/`，数据库和配置使用 `config/`，应用日志使用同级 `logs/`。构建脚本不预创建 `download/`，以便首次启动执行目录选择；系统 Downloads fallback 使用 `Downloads/XArchive` 子目录。

当前 executor control commands (`submit_executor_job`、`query_executor_job`、`cancel_executor_job`、`shutdown_executor`) 使用 bounded control worker、单 active runner 和独立 SQLite persistence context；submit 会通过独立 orchestration thread 调度 runner，runner 从 `job_id` 加载 execution spec 并创建 Database/FileStore/Sidecar/ArchiveExecutionJob。shutdown 会先持久化 active Job 为 `INTERRUPTED` 再关闭 control worker；U8 后用户入口只有 executor 命令，运行中 Sidecar process-tree interrupt 的 Windows Job Object 语义和 startup recovery 的真实 staging/final facts 仍由 Windows Queue 验证。

## 浏览器归档请求（executor 链路，CURRENT）

```text
X 页面
  → Extension / Native Host / Desktop command adapter
  → BrowserRequest 校验
  → ArchiveApplicationService submit-and-schedule/query/cancel/shutdown
  → SQLite 创建或复用 Job
  → JobExecutorHandle 投递 bounded command
  → executor worker 获取 Job snapshot 并派发 execution thread
  → Sidecar v2 extraction / aria2 transfer / FileStore I/O（不持有 RuntimeState 全局锁）
  → JobEvent 和状态持久化
  → ArchiveService 提交最终目录
  → list_jobs / BrowserResponse 查询状态
```

控制流独立于归档 I/O：`cancel`、`stop_sidecar`、`get_app_status` 和 `list_jobs` 必须能在 worker 等待 Sidecar 或文件处理期间继续响应。`get_app_status` 当前报告 `executor` 生命周期状态（`ready`/`stopped`）；该字段只反映 RuntimeState 持有的 worker ownership，不代表真实归档 Job 已切换到 executor worker。

## Sidecar extraction（v2，CURRENT）

```text
executor execution spec
  → SidecarV2Command::hello / extract
  → xarchive-sidecar-supervisor 写入 JSONL stdin（只接受 protocol v2 stdout）
  → Python worker_v2 读取 command
  → extraction.py 构造 extraction-only gallery-dl 参数（--skip-download）
  → gallery-dl 只写 metadata
  → Python worker 发出 ready/extraction_started/extracted/cancelled/failed/log
  → Rust 消费 typed ExtractionResult
  → Rust MediaTransferPlan
  → aria2-only transfer 写入 staging
  → Rust 再次检查 staging 文件并提交
```

Sidecar 只提供 extraction 事实，不能自行决定本地归档成功；媒体主体由 aria2 transfer 写入，Rust 必须在最终提交前重新检查文件系统结果。协议 v1 的 `download` command、gallery-dl 媒体下载和 `metadata/file/progress/complete` 事件已在 U8 删除，Supervisor 会把非 v2 stdout 事件记为 `ProtocolError`。

## 迁移边界

U8 已结束旧路径迁移：Sidecar protocol v1 runtime、`download` command/event、旧 `file/progress/complete` 事件、`DownloadRouter` fallback、`GalleryDlThenAria2` 和 `archive_tweet` 同步入口都已删除。当前运行流只保留 gallery-dl extraction-only → aria2-only transfer 链路；历史文档可以保留旧事实，但当前状态文档不再把 v1 路径描述为可运行路径。

## 下载路由

U8 后不存在业务级 backend 路由：aria2 是唯一媒体 transfer backend，`xarchive-download` 只暴露 `Aria2TransferDriver`、`MediaTransferPlan` 和 `RefreshCoordinator`。extraction 的 401/403/expired URL 由 `RefreshCoordinator` 触发一次性重新 extraction 和新 plan，不做 gallery-dl 下载回退。

## 本地归档提交

```text
Sidecar metadata/files
  → SidecarArchiveRequest identity binding
  → staging relative path validation
  → symlink/reparse rejection
  → local size/hash calculation
  → tweet/media/user/profile persistence
  → staging commit
  → JobEvent::DownloadCompleted
```

`crates/xarchive-storage/` 拥有 SQLite、FileStore、ArchiveService 和 migrations。`crates/xarchive-storage/migrations/` 是 migration 的唯一所有位置。

## Telegram 发送

Telegram contract crate 提供请求模型、formatter、transport、SecretStore abstraction 和发送状态接口。真实账号凭据、Credential Manager 和 Desktop 应用级发送调度不应被误认为已经由单元层 contract 实现。

## 维护边界

- 修改浏览器消息：同步 Extension、Native Host、protocol crate、Schema、fixtures 和协议文档。
- 修改 Sidecar command/event：同步 protocol crate、Schema、Python worker、Supervisor、Desktop adapter 和测试。
- 修改数据库 schema：只在 storage migrations 中添加 migration，并同步 data model、storage 测试和 Windows application-level validation queue。
- 修改 Tauri command：同步前端 invoke、Desktop command 测试、repository map、setup 文档和平台验证范围。