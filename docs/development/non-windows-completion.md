# 非 Windows 开发完成清单

> 截至 2026-09-09。本文件记录当前阶段已经在 Linux/跨平台代码中完成的内容，以及仍不应被错误归类为 Windows 阻塞的工作。

## 已完成

- Rust Job 状态机、ArchiveService、SQLite、FileStore、staging、SHA-256 和 Sidecar 结果转换。
- `xarchive-download` aria2 JSON-RPC 请求模型、状态解析、loopback HTTP client、fake-server 测试和基础 `Aria2Supervisor` 进程监督层；Linux 已验证配置校验、aria2 参数构造、进程启动失败映射和 secret 脱敏。
- BrowserRequest/BrowserResponse 模型、JSON Schema 和 fixture。
- Chromium Native Messaging 4 字节 little-endian framing、1 MiB payload 限制、JSON 边界处理和结构化错误响应。
- MV3 Extension classic content script、Tweet DOM 提取、按钮去重、MutationObserver、Service Worker Native Bridge、request_id 路由和断线处理。
- `xarchive-core` retry/backoff policy、错误分类、Windows-safe 用户目录名和 TagEngine。
- `xarchive-storage` users、user_names、tags、tweet_tags Repository API，包含幂等写入和排序查询测试。
- `xarchive-telegram` SecretStore abstraction、MemorySecretStore、BotToken 脱敏、Bot API request models、metadata formatter、UTF-8 continuation、media group 分组，以及基于 `reqwest 0.13.4` blocking + Rustls 的 Telegram HTTPS transport；已通过 fake-server 测试覆盖四种 Bot API 方法、HTTP/API 错误和 token 脱敏。另含 `SendState`/`SendStateStore` 契约、`send_idempotently` 幂等补传编排（同一 `(chat_id, idempotency_key)` 的投递闭包跨重启至多执行一次）和 `TelegramResponse.result_message_id`。上述单元层验证（含 storage 16 项、telegram 12 项）已在 Windows revision `add84c0` 及其后续 revision（最新 `f3faea3`，业务代码一致）上实际执行并通过（69 项测试计数已在两侧按 crate 清点确认）。
- `xarchive-storage` `telegram_send_attempts` 发送状态持久化（migration `0002_telegram_send_state.sql`，版本化 migration loop）与 `Database` 的 `SendStateStore` SQLite 实现（`find_sent`/`record_pending`/`record_sent`/`record_failed`/`list_unsent`）。
- Tauri/React Dashboard、运行时状态、Sidecar 生命周期、最近 Job 查询、归档目录打开命令和项目内 Tauri CLI 入口。
- Desktop aria2 管理 UI 与 Rust commands：检测程序目录、`bin/`、应用数据目录和 `PATH`，展示版本/来源，提供官方 Windows x64 版本 allowlist、SHA-256 校验和下载入口；Linux 已通过 Rust fmt/check/test 与 Vite 构建验证。
- Node/Rust/schema/config 静态验证和 fake transport 测试。

## 仍需独立技术或外部环境决策

- Telegram 发送状态持久化与幂等补传的跨平台代码已完成；真实账号/网络发送与生产 endpoint 验证仍需账号环境。
- profile 文件、Quote/Reply 完整建模和更完整的 Users/Archive/Settings GUI。
- aria2c.exe 在 Windows 的实际下载/解压/运行验证、断点恢复、崩溃恢复、artifact 分发和默认 Download Router。

## Windows/账号/发布环境专属

- Windows Named Pipe server/client、ACL 和生命周期。
- Native Host 到 Named Pipe 的实际转发。
- Edge/Chrome Native Host manifest、Registry 注册和浏览器实机加载。
- Edge Cookie 读取、真实 X 认证归档和媒体场景验证。
- Windows Credential Manager backend。
- Tray、Single Instance、Autostart、Sidecar executable、Tauri externalBin、Windows bundle/installer、签名、杀毒软件和 Updater。

## 当前环境限制

- 当前 Linux 环境未安装 `pytest`，本轮仅执行 Python `compileall`；Windows 既有 10 个 Sidecar 测试结果仍保留在 Windows 验证文档。
- 当前 Linux 环境未安装 `cargo-clippy`；Windows 既有完整 workspace clippy 结果仍保留在 Windows 验证文档。2026-09-09 Windows 已针对 revision `add84c0` 重新执行严格 workspace clippy 并通过，clippy 失败链路闭环。
- 最新 Windows workspace clippy 曾因 Desktop aria2 路径扫描的 `collapsible_if` 失败；Linux 已改为 let-chain 并完成 fmt/check/test 回归，Windows clippy re-validation 已于 2026-09-09 通过，该项 lint 闭环完成。