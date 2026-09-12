# 非 Windows 开发完成清单

> 截至 2026-09-10。本文件记录当前阶段已经在 Linux/跨平台代码中完成的内容，以及仍不应被错误归类为 Windows 阻塞的工作。

## 已完成

- Rust Job 状态机、ArchiveService、SQLite、FileStore、staging、SHA-256 和 Sidecar 结果转换。
- `xarchive-download` aria2 JSON-RPC 请求模型、状态解析、loopback HTTP client、fake-server 测试和基础 `Aria2Supervisor` 进程监督层；Linux 已验证配置校验、aria2 参数构造、进程启动失败映射和 secret 脱敏。
- BrowserRequest/BrowserResponse 模型、JSON Schema 和 fixture。
- Native Host 请求转发核心：校验 BrowserRequest、通过配置的 transport endpoint 转发 framing 请求并返回 BrowserResponse；Linux fake duplex transport 已覆盖成功转发和非法请求不写入 transport。Windows Named Pipe server/ACL/Registry 仍需平台实现和实机验证。
- Chromium Native Messaging 4 字节 little-endian framing、1 MiB payload 限制、JSON 边界处理和结构化错误响应。
- MV3 Extension classic content script、Tweet DOM 提取、按钮去重、MutationObserver、Service Worker Native Bridge、request_id 路由和断线处理。
- `xarchive-core` retry/backoff policy、错误分类、Windows-safe 用户目录名和 TagEngine。
- `xarchive-storage` users、user_names、tags、tweet_tags Repository API，包含幂等写入和排序查询测试。
- `xarchive-telegram` SecretStore abstraction、MemorySecretStore、BotToken 脱敏、Bot API request models、metadata formatter、UTF-8 continuation、media group 分组，以及基于 `reqwest 0.13.4` blocking + Rustls 的 Telegram HTTPS transport；已通过 fake-server 测试覆盖四种 Bot API 方法、HTTP/API 错误和 token 脱敏。另含 `SendState`/`SendStateStore` 契约、`send_idempotently` 幂等补传编排（同一 `(chat_id, idempotency_key)` 的投递闭包跨重启至多执行一次）和 `TelegramResponse.result_message_id`。上述单元层验证（含 storage 16 项、telegram 12 项）已在 Windows revision `add84c0` 及其后续 revision（业务代码一致，最新 `5d9dbd9` 为轻量复核）上实际执行并通过（69 项测试计数已在两侧按 crate 清点确认）。
- `xarchive-storage` `telegram_send_attempts` 发送状态持久化（migration `0002_telegram_send_state.sql`，版本化 migration loop）与 `Database` 的 `SendStateStore` SQLite 实现（`find_sent`/`record_pending`/`record_sent`/`record_failed`/`list_unsent`）。
- Tauri/React Dashboard、运行时状态、Sidecar 生命周期、最近 Job 查询、归档目录打开命令和项目内 Tauri CLI 入口。
- Desktop aria2 管理 UI 与 Rust commands：检测程序目录、`bin/`、应用数据目录和 `PATH`，展示版本/来源，提供官方 Windows x64 版本 allowlist、SHA-256 校验和下载入口；Linux 已通过 Rust fmt/check/test 与 Vite 构建验证。
- `xarchive-download` 纯 Rust `DownloadRouter`：默认 gallery-dl、可配置 aria2 fallback、仅对 `EXTRACT_OR_DOWNLOAD_FAILED` 回退、认证/限流/不存在错误不回退，以及 gallery-dl/aria2 双失败原因保留；已通过 Linux 单元测试。Desktop `archive_tweet` 已接入 Router 结果处理，并在 gallery-dl/aria2 失败时持久化 Job 错误状态和下载失败事件；真实 aria2 `AddUriRequest`、403 后重新提取 URL 和 transfer 生命周期仍待完成。
- Node/Rust/schema/config 静态验证和 fake transport 测试。
- **GUI Linux 修复已完成：**系统就绪联合判断、初始加载占位、带用户级区域标题的错误框 `role="alert"`/`aria-live`、按 Widget 分离错误并提供重试、紧凑侧栏 `aria-label`、移除未实现页面的禁用主导航、aria2 按 Windows 平台展示、最近任务统计命名、任务列表 `<ul>/<li>`/`<time>` 语义、共享 `:focus-visible` 和 `prefers-reduced-motion` 保护；Linux Vite check/build、Desktop Node test、Rust fmt/check/test 均通过。
- **GUI 白色 Vercel 风格重设计已完成：**白色主背景、细灰边框、近黑主按钮、清晰状态 Badge、结构化最近任务、运行环境/归档位置卡片、Skeleton 加载状态和更适合 Desktop/DPI 的字号层级；Linux Vite check/build、Desktop Node test、Rust fmt/check/test 已通过。
- **Desktop GUI 源码层设计审查已完成。参见 `docs/development/roadmap.md` M6 GUI 的“当前 GUI 设计评估”部分和 `docs/development/windows-validation.md` 的 W-P1-10 条目。结论是当前 GUI 为较高完成度的开发 Dashboard 原型，尚不符合直接进行视觉验收的正式用户界面，不得将 GUI 视觉验收等同于功能完成。GUI 的核心修补、按平台展示、焦点/键盘可访问性和对比度改善可以继续在 Linux 上推进；但最终验收仍受控于 Windows WebView2/DPI/助残环境验证，不得在此之前标记为已完成。**

- **M5 profile 文件已完成：**`xarchive-storage` 新增 `UserProfileSnapshot`/`UserProfileFile` 类型、`Database::user_profile()` 快照查询、`FileStore::write_user_profile()` 写入 `Users/<stable>/profile.json`，`ArchiveService::refresh_author_profile()` 在 `complete_local_archive` 中自动注册作者并刷新 profile（`user_id` 缺失时静默跳过）；`upsert_user` 增加名称去重逻辑。Linux 验证通过。

## 仍需独立技术或外部环境决策

- Telegram 发送状态持久化与幂等补传的跨平台代码已完成；真实账号/网络发送与生产 endpoint 验证仍需账号环境。
- aria2 基础 Windows artifact/RPC/断点/进程恢复链路已有实测证据；仍需独立处理新的业务集成、403 回退 gallery-dl、externalBin/打包分发和默认 Download Router。
- aria2 基础 Windows artifact/RPC/断点/进程恢复链路已有实测证据；Linux 已实现可测试的 `DownloadRouter` 策略，但仍需接入真实 Sidecar/Job 调度、403 后重新提取 gallery-dl URL、externalBin/打包分发和端到端默认 Download Router。
- **仍需 Windows 的 GUI 项目：**真实 WebView2/DPI 渲染、Tab 顺序、Focus-visible 实际表现、命中目标、屏幕阅读器反馈和最终对比度验收；这些项目保持 `WINDOWS_VERIFICATION_PENDING`，不能因 Linux 构建通过提前标记 PASS。
- **最近一次 Windows 强制同步复验已覆盖当前 GUI working tree：**Widget 错误分离、用户级区域标题与重试、未实现导航项移除、Job/时间语义、Focus-visible 和 reduced-motion 的构建/测试已在 Windows 通过；真实 WebView2/DPI、键盘、屏幕阅读器和对比度仍保持 `WINDOWS_VERIFICATION_PENDING`，因为当前环境缺少 GUI automation native app target。
- **Linux GUI 源码修补与本轮视觉重设计已收口：**在 Windows 原生 GUI target 可用前，不再继续重复 GUI 源码修补；下一次 Windows 验证应直接执行真实 WebView2/DPI、键盘、屏幕阅读器、命中目标和对比度验收。

## Windows/账号/发布环境专属

- Windows Named Pipe server/client、ACL 和生命周期。
- Native Host 到 Named Pipe 的 Windows 实际连接、ACL 和生命周期；跨平台转发编排核心已完成，但 Windows endpoint 尚未实机验证。
- Edge/Chrome Native Host manifest、Registry 注册和浏览器实机加载。
- Edge Cookie 读取、真实 X 认证归档和媒体场景验证。
- Windows Credential Manager backend。
- Windows WebView2/DPI/键盘/屏幕阅读器/命中目标/真实对比度 GUI 验收（`WINDOWS_VERIFICATION_PENDING`）。
- Tray、Single Instance、Autostart、Sidecar executable、Tauri externalBin、Windows bundle/installer、签名、杀毒软件和 Updater。

## 当前环境限制

- 当前 Linux 环境未安装 `pytest`，本轮仅执行 Python `compileall`；Windows 既有 10 个 Sidecar 测试结果仍保留在 Windows 验证文档。
- 当前 Linux 环境未安装 `cargo-clippy`；Linux clippy 记为 `NOT RUN`。Windows 最新复验曾发现 `run_sidecar_download` 的 `too_many_arguments`，该项目代码问题已在 Linux 用 `SidecarDownloadRequest` 上下文结构修复，并通过 Linux fmt/check/test；Windows 严格 clippy 复验已针对最新 HEAD `040b982` 多轮实际执行并通过，该项已闭环，不因 Windows PASS 改写为 Linux PASS。
- 最新 Windows workspace clippy 曾因 Desktop aria2 路径扫描的 `collapsible_if` 失败；Linux 已改为 let-chain 并完成 fmt/check/test 回归，Windows clippy re-validation 已于 2026-09-09 通过，该项 lint 闭环完成。