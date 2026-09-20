# 非 Windows 开发完成清单

> 兼容索引：当前实现状态以 [`status.md`](status.md) 为准；未来方向以 [`roadmap.md`](roadmap.md) 为准；本文件保留历史完成清单和迁移链接，不再作为新的状态事实源。

> 截至 2026-09-10。本文件记录当前阶段已经在 Linux/跨平台代码中完成的内容，以及仍不应被错误归类为 Windows 阻塞的工作。

## 已完成

- **2026-09-19 U8 旧路径删除：**删除 `archive_tweet` 注册/实现和同步 archive fallback、Rust v1 Sidecar command/event 类型（`sidecar.rs`，`DownloadFile` 移至 `media.rs`）、Supervisor v1 `send`/`spawn_ready`/`Download` 事件（stdout reader 只接受 protocol v2）、Python v1 worker 与 `gallery.py`/`DownloadedFile`、`DownloadRouter`/`GalleryDlThenAria2`、v1 Schema/fixtures 和 v1 PyInstaller entrypoint；`start_sidecar`/`stop_sidecar` 切换到 v2 handshake/shutdown。新增 `test_entrypoint.py` 覆盖 v2 启动器契约与 legacy 命令拒绝。Linux 验证：workspace Rust 184/184、Sidecar pytest 21/21、Node Desktop 33/33、Extension 7/7、fmt/check/clippy/build 通过。Windows packaged v2-only worker、Tauri command surface、Job Object shutdown、真实 extraction/aria2/commit 和 restart/recovery 仍进入集中 Windows queue，并按手工步骤执行。

- **2026-09-19 U7 Desktop production integration：**从 `feat/extraction-aria2-pipeline` 创建 `feature/u7-desktop-production-integration`，新增 `desktop/src-tauri/src/production.rs`，完成 Sidecar v2 capability handshake、typed extraction event consumption、`ExtractionResult` → `MediaTransferPlan`、aria2-only transfer、一次性 expired URL refresh、stable media identity/filename 集合校验、staging path/file/reparse verification 和 `ArchiveService` commit 接线。`ArchiveExecutionContext` 继续由 runner 持有 Database/FileStore/Sidecar/aria2 配置；v1 `archive_tweet`/DownloadRouter 保留为 U8 前 migration fallback。Linux 验证：Desktop Rust 81/81、workspace Rust tests/doc-tests、workspace strict Clippy、xarchive-download 23/23 + integration 7/7、protocol 15/15、Supervisor 5/5、Node Desktop 33/33、Extension 7/7、Sidecar pytest 33/33、fmt/check/build 全部通过。Windows Tauri artifact、aria2c.exe、Job Object、file lock、restart/recovery、WebView2、Named Pipe 和真实 signed URL 仍进入集中 Windows queue。
- **2026-09-19 U7 Windows reconciliation：**Windows 当前 revision 已通过 Node/Rust/Tauri baseline、Sidecar full pytest 33/33、PyInstaller v2 one-dir artifact、`--help`、v2 `hello/capabilities`、unknown-field rejection、shutdown probe，以及 Core/Full assembly/startup smoke。该结果只关闭 packaged worker/handshake 范围；aria2 transfer、真实 extraction/download、expired URL refresh、staging/ArchiveService commit、Windows file lock/reparse、cancel/shutdown/restart/recovery、GUI、Named Pipe 和真实账号仍为 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`，不能将 U7 runtime 标记为 Windows PASS。
- **2026-09-19 U7 Desktop production integration：**从 `feat/extraction-aria2-pipeline` 创建 `feature/u7-desktop-production-integration`，新增 `desktop/src-tauri/src/production.rs`，完成 Sidecar v2 capability handshake、typed extraction event consumption、`ExtractionResult` → `MediaTransferPlan`、aria2-only transfer、一次性 expired URL refresh、stable media identity/filename 集合校验、staging path/file/reparse verification 和 `ArchiveService` commit 接线。`ArchiveExecutionContext` 继续由 runner 持有 Database/FileStore/Sidecar/aria2 配置；v1 `archive_tweet`/DownloadRouter 保留为 U8 前 migration fallback。Linux 验证：Desktop Rust 82/82、workspace Rust tests/doc-tests、workspace strict Clippy、xarchive-download 23/23 + integration 7/7、protocol 15/15、Supervisor 5/5、Node Desktop 33/33、Extension 7/7、Sidecar pytest 33/33、fmt/check/build 全部通过。Windows Tauri baseline、packaged v2 worker、Sidecar full pytest 33/33 已通过；U7 真实 transfer/runtime 仍在集中 Windows queue。

- **2026-09-18 U6 URL refresh contract：**`xarchive-download` 新增 403/401/expired/signature/access-denied transfer failure 分类、`RefreshCoordinator` 和 `EXTRACTION_RESULT_CHANGED` contract；首次 transfer 仅在 URL 过期类错误时触发一次完整 re-extraction，重新按 stable media identity/filename 集合匹配，集合变化明确失败，普通失败、取消、shutdown、timeout、磁盘/权限类错误不 refresh。新增 refresh coordinator tests；`xarchive-download` 23/23 unit、7/7 integration、Sidecar pytest 33/33、Node desktop 33/33、Extension 7/7、workspace test/clippy/fmt 通过。当前 refresh contract 尚未接入 Desktop production executor；Windows aria2/文件锁/真实 signed URL expiry 保持集中验证。

- **2026-09-18 U5 aria2-only transfer driver：**`xarchive-download` 新增 backend-neutral `MediaTransferPlan` 构建与 aria2-only `Aria2TransferDriver`；从 typed `ExtractionResult` 生成稳定 media identity/filename/header allowlist，支持 multi-GID 提交、按 plan 顺序完成、progress 单调性检查、timeout、cancel、shutdown、失败/removed 状态分类、提交失败清理和 `.aria2`/partial artifact 清理。新增 7 个 transfer-driver 集成测试和 4 个 plan 测试；`cargo test -p xarchive-download` 20/20、集成测试 7/7、workspace test、workspace clippy、fmt 通过。当前 driver 尚未接入 Supervisor/Desktop executor，403 refresh 属于 U6；Windows aria2c、文件锁、进程恢复和 packaged artifact 保持集中验证。

- **2026-09-18 U4 gallery-dl extraction-only：**`extraction.py` 重写为 extraction-only 适配层（强制 `--skip-download`、防御性拒绝媒体写入 flag、`sanitize_filename` 净化、`stable_media_id` 三级 identity、兼容 `*.info.json`、result 序列化剥离 `raw` 且不携带下载事实）。新增 `test_extraction_only.py` 覆盖命令约束、净化、identity、fake gallery-dl 写媒体但 result 无下载事实和 AUTH_REQUIRED 不回退。Sidecar pytest 28/28、compileall、`git diff --check` 通过；Rust 本轮未改动。v2 extraction 未接 Supervisor/Desktop，v1 链路保持 MIGRATION；Windows packaged worker 行为保持集中验证。

- **2026-09-18 U3 Sidecar protocol v2 contract：**`xarchive-protocol` 新增 `sidecar_v2` 模块（typed `extract` command、`ready/extraction_started/extracted/cancelled/failed/log` 事件、typed `ExtractionResult`、Tweet ID 与 X URL 绑定、header allowlist/secret 脱敏、v1/unknown field/缺失 capability 显式拒绝）；Python 侧新增 `protocol_v2`、`worker_v2` 与 `extraction` 模块及 contract tests；Schema 新增 `sidecar-v2-command/event` 与 valid/invalid/v1-rejected fixtures。Rust protocol 15/15、Sidecar pytest 23/23、fmt/clippy、Node check/test、compileall 与 `git diff --check` 通过。Supervisor spawn v2 worker 与 Desktop 消费属于 U7，运行链路仍为 v1；Windows packaged worker 行为保持集中验证。

- **2026-09-18 U2 Sidecar cooperative cancellation：**worker 已在下载期间通过 command-reader/control queue 消费 `cancel` 与 `shutdown`；gallery-dl 子进程支持超时、取消和 shutdown interruption，POSIX 使用独立 session，Windows 使用 `taskkill /T /F` 进程树回收；Sidecar compileall 与 pytest 通过。Windows 进程树、文件锁、残留进程和 packaged worker 行为仍需集中验证。

- **2026-09-18 U2 Rust process-group follow-up：**`xarchive-sidecar-supervisor` 在 Unix 上使用标准库 `Command::process_group(0)` 创建独立 process group，shutdown/force cleanup 使用组级 SIGTERM/SIGKILL，避免仅回收 Sidecar 直接子进程而遗留 gallery-dl 子树。Supervisor 4/4、Sidecar pytest 17/17、相关 clippy 通过；Windows Job Object/进程树行为仍需集中验证。

- **2026-09-17 Windows worker 失败后的 Linux follow-up：**移除 PyInstaller worker 入口的重复 `main()` 执行；workflow 在上传前检查 one-dir 目录中的 `_internal/python312.dll`；portable Core manifest 将 `user_importable` 与当前 GitHub 外链 Extension 流程对齐为 false。Linux compile/contract/build 回归通过；真实 Windows worker `--help`、Full Sidecar handshake 和 portable runtime 仍需 Windows 重验，状态保持 `WINDOWS_VERIFICATION_PENDING`。

- **2026-09-17 GUI/日志/任务统计收口：**Dashboard 使用 storage crate 的全量 `JobMetrics` 查询展示全部、进行中、已完成、失败和数据库五项指标；日志等级统一为 `error/warning/info/debug/silent`；Sidebar 服务状态支持键盘激活并跳转设置页目标区块；gallery-dl/aria2 使用 Tauri dialog 原生文件选择器；Extension 移除本地导入入口，改为打开 GitHub `extension` 目录。Linux 已通过 storage 24/24、Desktop Node 31/31、Vite build、Rust fmt/check 和 `git diff --check`。Windows 原生对话框、WebView2/DPI、剪贴板和浏览器加载仍不能由 Linux 结果替代。

- **Full/Core portable Linux 收口（2026-09-17）：**portable 包类型校验、组件规划和
  `package-manifest.json` 已抽为无副作用纯逻辑并由 Desktop Node 测试覆盖；Rust 覆盖
  worker 的 `--gallery-dl` 参数和 Full/Core 配置默认值；Sidecar 覆盖 executable 参数
  的真实子进程入口；新增 PyInstaller spec、独立入口和 Windows artifact workflow。
  这些内容只证明跨平台代码/构建定义正确，不替代 Windows `.exe`、WebView2 或文件系统验证。

- **Security/Privacy hardening（2026-09-12）：**协议层将 Tweet ID 与 X URL status ID 绑定；Desktop 不再接受每次归档请求指定任意 Sidecar executable；Sidecar 失败信息改为稳定安全文案，不将原始 stderr 持久化到 Job/UI；storage 层要求 Sidecar metadata Tweet ID 与请求一致，拒绝 symlink/reparse 文件，并将 settings 限定为 `ui.*`/`download.*`、合法 JSON 和 16 KiB 上限；generic settings Tauri IPC 已移除。Linux fmt/check/test、Node check/test/build 和 Python compileall 已通过。

- Rust Job 状态机、ArchiveService、SQLite、FileStore、staging、SHA-256 和 Sidecar 结果转换。
- `xarchive-download` aria2 JSON-RPC 请求模型、状态解析、loopback HTTP client、fake-server 测试和基础 `Aria2Supervisor` 进程监督层；Linux 已验证配置校验、aria2 参数构造、进程启动失败映射和 secret 脱敏。
- BrowserRequest/BrowserResponse 模型、JSON Schema 和 fixture。
- Native Host 请求转发核心：校验 BrowserRequest、通过配置的 transport endpoint 转发 framing 请求并返回 BrowserResponse；Linux fake duplex transport 已覆盖成功转发和非法请求不写入 transport。Desktop 已在 Linux/Unix 上注册 Unix domain socket transport endpoint，Native Host 在 Linux 上改用 `UnixStream` 连接；Windows Named Pipe server/ACL/Registry 仍需平台实现和实机验证。
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

- **仍保留的 Linux 后续开发项：**`archive_tweet` 当前仍在 Tauri command 生命周期内持有全局 `RuntimeState` 锁并执行长时间 Sidecar I/O。该项不依赖新的 Windows 结果，但需要单独的后续 Linux 架构批次，不能因本轮安全修复已通过而误标记完成；本轮未冒险进行未充分设计的锁/资源生命周期重构。

- **Migration ownership（2026-09-12）：**版本化 SQLite migration 已从 Desktop 目录迁移到 `crates/xarchive-storage/migrations/`，由 storage crate 自主管理；storage reopen、旧版本升级和 workspace 测试已通过。Windows Desktop 应用级旧库启动、迁移和恢复仍保持 `WINDOWS_VERIFICATION_PENDING`。

- **2026-09-12 Windows reconciliation：**Windows 最新安全加固验证暴露的 `complete_sidecar_archive` clippy 8 参数问题和 `download-command.schema.json`/Python Worker 的 `executable` 契约残留已在 Linux 修复。Linux `cargo fmt/check/test`、Node check/test/build、Python compileall 和 schema JSON parse 已通过；Linux clippy/pytest 因工具缺失记为 `NOT RUN`。WQ-P1-12 必须等待 Windows 重新验证，保持 `WINDOWS_VERIFICATION_PENDING`。

## Windows/账号/发布环境专属

- Windows Named Pipe server/client、ACL 和生命周期。
- Native Host 到 Named Pipe 的 Windows 实际连接、ACL 和生命周期；跨平台转发编排核心已完成，Desktop 已在 Linux/Unix 上注册生产 transport endpoint（Unix domain socket），Native Host 在 Linux 上改用 `UnixStream` 连接；Windows Named Pipe endpoint 尚需实机验证。
- Edge/Chrome Native Host manifest、Registry 注册和浏览器实机加载。
- Edge Cookie 读取、真实 X 认证归档和媒体场景验证。
- Windows Credential Manager backend。
- Windows WebView2/DPI/键盘/屏幕阅读器/命中目标/真实对比度 GUI 验收（`WINDOWS_VERIFICATION_PENDING`）。
- Tray、Single Instance、Autostart、Sidecar executable、Tauri externalBin、Windows bundle/installer、签名、杀毒软件和 Updater。
- 本轮安全边界的 Windows 回归：URL/Tweet ID 与 Sidecar metadata 绑定、Tauri command surface、executable override 拒绝、symlink/junction/reparse point 拒绝，以及 archive root/SQLite/staging ACL。

## 当前环境限制

- 当前 Linux 环境未安装 `pytest`，本轮仅执行 Python `compileall`；Windows 既有 10 个 Sidecar 测试结果仍保留在 Windows 验证文档。
- 当前 Linux 环境未安装 `cargo-clippy`；Linux clippy 记为 `NOT RUN`。Windows 最新安全加固验证曾发现 `complete_sidecar_archive` 的 8 参数 `too_many_arguments`，该项目代码问题已在 Linux 用 `SidecarArchiveRequest` 上下文结构修复，并通过 Linux fmt/check/test；Windows strict clippy 仍需针对最新 Linux working tree 重新执行，保持 `WINDOWS_VERIFICATION_PENDING`。
- 最新 Windows workspace clippy 曾因 Desktop aria2 路径扫描的 `collapsible_if` 失败；Linux 已改为 let-chain 并完成 fmt/check/test 回归，Windows clippy re-validation 已于 2026-09-09 通过，该项 lint 闭环完成。