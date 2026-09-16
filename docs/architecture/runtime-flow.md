# 运行流

本文描述当前代码中的主要运行路径，不记录测试结果或未来功能状态。

## 当前浏览器归档请求

```text
X 页面
  → Extension content script 提取 BrowserTweet
  → Extension background service worker 创建 BrowserRequest
  → Native Messaging Host framing/校验
  → Desktop transport endpoint
  → Tauri `submit_executor_job` command
  → `ArchiveJobSubmissionAdapter` 校验 BrowserRequest 并派生稳定 Job identity
  → 独立 SQLite context 创建/复用 Job，并写入 BrowserTweet/user
  → 短暂从 RuntimeState lease SidecarSupervisor 和 archive root
  → `ArchiveExecutionContext` / `ArchiveExecutionJob`
  → executor control worker 派发到独立 execution thread
  → Sidecar Download、FileStore staging、ArchiveService commit（不持有 RuntimeState 全局锁）
  → staging 文件
  → Rust 检查 metadata、路径、文件大小和 SHA-256
  → ArchiveService 提交最终目录和数据库状态
  → list_jobs / BrowserResponse 返回状态
```

Native Host 的 framing 和 forwarding 核心位于 `crates/xarchive-native-host/`；Windows Named Pipe server 和 ACL 属于平台适配边界，不由跨平台 framing 代码决定。

## Desktop 启动

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

当前 Desktop 已按 `archive.rs`、`commands.rs`、`runtime.rs`、`platform.rs` 和 `aria2.rs` 完成模块化；`submit_executor_job` 是真实 executor 归档入口，`archive_tweet` 保留为同步 fallback。runner 从持久化 execution spec 自主创建 Database、FileStore 和 SidecarSupervisor，不依赖 RuntimeState 的 Sidecar lease；RuntimeState 不在长时间 Sidecar/FileStore I/O 期间持锁。

便携运行时的目录边界为：最终归档使用 `download/`，临时 staging 使用 `cache/staging/`，数据库和配置使用 `config/`，应用日志使用同级 `logs/`。构建脚本不预创建 `download/`，以便首次启动执行目录选择；系统 Downloads fallback 使用 `Downloads/XArchive` 子目录。

当前 executor control commands (`submit_executor_job`、`query_executor_job`、`cancel_executor_job`、`shutdown_executor`) 使用 bounded control worker、单 active runner 和独立 SQLite persistence context；runner 从 `job_id` 加载 execution spec 并创建 Database/FileStore/Sidecar/ArchiveExecutionJob。shutdown 会先持久化 active Job 为 `INTERRUPTED` 再关闭 control worker；startup 自动调度和运行中 Sidecar interrupt 仍是后续边界。

## R1 目标浏览器归档请求

```text
X 页面
  → Extension / Native Host / Desktop command adapter
  → BrowserRequest 校验
  → ArchiveApplicationService submit/query/cancel/shutdown
  → SQLite 创建或复用 Job
  → JobExecutorHandle 投递 bounded command
  → executor worker 获取 Job snapshot 并派发 execution thread
  → Sidecar / DownloadRouter / FileStore I/O（不持有 RuntimeState 全局锁）
  → JobEvent 和状态持久化
  → ArchiveService 提交最终目录
  → list_jobs / BrowserResponse 查询状态
```

控制流独立于归档 I/O：`cancel`、`stop_sidecar`、`get_app_status` 和 `list_jobs` 必须能在 worker 等待 Sidecar 或文件处理期间继续响应。`get_app_status` 当前报告 `executor` 生命周期状态（`ready`/`stopped`）；该字段只反映 RuntimeState 持有的 worker ownership，不代表真实归档 Job 已切换到 executor worker。

## Sidecar 下载

```text
archive_tweet
  → SidecarCommand::Download
  → xarchive-sidecar-supervisor 写入 JSONL stdin
  → Python worker 读取 command
  → gallery.py 构造 gallery-dl 参数
  → gallery-dl 写入 staging
  → Python worker 发出 started/metadata/file/complete 或 failed
  → Rust 收集事件
  → ArchiveService 再次读取 staging 并提交
```

Sidecar 只提供执行事件和 metadata，不能自行决定本地归档成功。Rust 必须在最终提交前重新检查文件系统结果。

## 下载路由

`xarchive-download::DownloadRouter` 默认使用 gallery-dl。只有调用方启用 fallback 并提供新鲜的 aria2 请求时，才会尝试 aria2；认证失败、限流和 Tweet 不存在不会被错误地回退为 aria2 下载。

当前 Router、aria2 client 和 supervisor 仍位于同一 crate 文件中，后续按业务路由、协议模型、HTTP client 和进程监督职责拆分。

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