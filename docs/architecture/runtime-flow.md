# 运行流

本文描述当前代码中的主要运行路径，不记录测试结果或未来功能状态。

## 浏览器归档请求

```text
X 页面
  → Extension content script 提取 BrowserTweet
  → Extension background service worker 创建 BrowserRequest
  → Native Messaging Host framing/校验
  → Desktop transport endpoint
  → Tauri archive_tweet command
  → SQLite 创建或复用 Job
  → Sidecar Download command
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
  → FileStore 初始化 archive root
  → Database::open() 应用 migrations
  → Tauri command registry
  → React Dashboard invoke commands
```

当前 Tauri runtime、commands 和归档编排仍集中在 `desktop/src-tauri/src/lib.rs`。后续拆分必须保持 `run()` 作为入口，并把 command adapter、应用服务、平台逻辑和 storage 调用分离。

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