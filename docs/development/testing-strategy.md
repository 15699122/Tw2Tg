# 测试策略

> 兼容索引：当前测试策略以 [`testing.md`](testing.md) 为准。本文件保留历史验证记录，后续不再追加新的逐轮测试结论。

## 单元测试

- Rust：状态机、幂等、metadata 合并、路径清洗、hash、错误分类、retry/backoff、下载后端路由、TagEngine、用户目录命名和 users/tags Repository。
- Python：JSONL、gallery-dl 归一化、事件、错误映射、stdout/stderr 隔离。
- JavaScript：DOM 提取、按钮去重、状态映射、批量查询、Native Bridge、request_id 路由和断线处理；Rust Native Host 负责 framing、请求校验和 transport 转发。

## 契约测试

使用 `shared/protocol-schema/fixtures/`，在 Rust、Python 和 TypeScript 中验证相同的有效/无效消息。

## 集成测试

- Rust + Fake Sidecar、Rust + Real Sidecar、临时 SQLite、临时 FileStore、Telegram contract/mock transport、Native Host framing/请求转发 fake transport、aria2 本地 HTTP fake server、`Aria2Supervisor` 配置/进程错误测试、`DownloadRouter` 默认/回退/不回退/双失败测试、Desktop `archive_tweet` 的 Router 结果/Job 失败状态/下载事件处理，以及 Desktop aria2 检测/版本 allowlist/SHA-256 helper 测试；真实 aria2 `AddUriRequest` 构造、403 后重新提取 URL、transfer 生命周期、Windows Named Pipe server/ACL、浏览器 Native Messaging 实机、官方 ZIP 下载/解压、安装器和真实账号链路属于后续集成或 Windows 验证。

## 故障注入

覆盖 Sidecar/aria2 崩溃、JSON 截断、认证失败、URL 过期、SQLite busy、磁盘不足、文件锁、Telegram timeout/rate limit、Desktop 中途退出和 Native Host 断开。

## 平台验证

- Linux 验证协议、跨平台代码和 fake transport；Windows 必须验证 Named Pipe、Cookie、Native Host Registry、Tauri Sidecar 打包、长路径和安装/卸载。

## 已完成的平台验证

截至 **2026-09-10**，Windows 已完成：

- Node workspace 检查、测试和构建。
- Rust workspace 已在补齐 `desktop/src-tauri/icons/icon.ico` 后恢复；Windows workspace 的 fmt、check、test、严格 clippy、Debug/Release 和 Tauri Release 构建均通过，62 个 crate 单元测试全部通过。此前因 Desktop aria2 路径扫描 `collapsible_if` 的 clippy 失败已由 Linux let-chain 修复，Windows re-validation 已通过。Supervisor 测试需显式使用项目 `.venv\\Scripts\\python.exe`，否则默认 `python3` 不存在会导致两个测试报 `NotRunning`。
- Python `.venv` editable 安装 Sidecar、gallery-dl 1.32.11 后的 10 个 Sidecar 测试。
- Rust Supervisor 与真实 Python Worker 的进程集成测试已在开发环境通过。
- Tauri Desktop 的 Vite/React 构建、Windows Debug/Release 编译、Windows `npm run dev:tauri` 启动、`npm run build:tauri` Release 构建、项目内 Tauri CLI 入口声明、启动时 SQLite 初始化、状态/Job/目录 commands、Sidecar `hello → ready` 握手、开发控制按钮、打开归档目录、最近 Job 查询和本地 shadcn/ui 组件构建已通过；停止开发进程时记录 Chromium `Error = 1411` 注销警告，真实 externalBin Sidecar 和 Windows GUI/打包尚未验证。
- `cargo fmt --all -- --check` 已通过。
- 此前在 `crates/xarchive-sidecar-supervisor/src/lib.rs:17` 检出的 `large_enum_variant` 已通过 `Box<DownloadEvent>` 修复并在 Windows 复验；Telegram formatter 的 `single_char_add_str` 已修复；Desktop aria2 路径扫描的 `collapsible_if` 已在 Linux 用 let-chain 修复，Windows 完整 workspace clippy re-validation 已通过。
- 真实 sidecar 已在含中文、空格和 Unicode 的路径中完成 JSONL 启动/下载/失败/退出链路验证；示例 X URL 未完成提取，真实账号下载仍待验证。
- 2026-09-09 针对 Linux revision `add84c0`（干净 working tree）的 Windows 全量复验：Node check/test/build、Rust fmt/check/test、严格 `cargo clippy -D warnings`、`.venv` pytest、`npm run build:tauri` 和 `npm run dev:tauri` 全部 PASS；`collapsible_if` clippy 失败链路正式闭环。本轮 Windows 报告 Rust 测试 68 项，与 Linux 同 revision 复测的 69 项（11/5/10/4/7/4/16/12）存在差一差异，Windows 自报分项（storage 16、telegram 12）与 Linux 一致；差异按待复核记录，下一轮 Windows 验证按 crate 清点回填。同日 Windows 针对 `4d4b3f5` 复验并按 crate 清点确认 69 项，68/69 差异已关闭（确认为计数笔误）；随后对纯文档 revision `facd8d7`、`5fbc675`、`f3faea3` 复验，69 项结果保持一致（业务代码自 `add84c0` 起无改动）；`55bcdc8` 起按轻量模式复核（全量证据对相同业务代码继续有效，最新复核至 `5d9dbd9`）。
- 2026-09-10 针对 Linux revision `34b5c67` 的 Windows 验证：Node check/test/build、Rust fmt/check/clippy/test、Python sidecar、Tauri Debug/Release 构建和进程清理均 PASS；aria2 相关项目因上一轮 `d239a1d` 已完成实际 Windows artifact/RPC/恢复验证而 NOT APPLICABLE；GUI 真实视觉/WebView2/DPI/键盘/辅助技术验收 BLOCKED，文件 SQLite 应用级恢复/迁移 NOT RUN。该轮没有项目代码导致的 Windows FAIL。
- 2026-09-10 Linux reconciliation 后续已完成 Widget 级错误分离与重试、移除未实现的禁用主导航、Job `<ul>/<li>`/`<time>` 语义、共享 `:focus-visible` 和 `prefers-reduced-motion` 保护；Desktop `npm run check`、`npm run test`、`npm run build`，以及 workspace `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 均通过。随后 Windows 使用 `/IS /IT` 强制同步并对完整 working tree 复验，以上代码（含用户级错误区域标题）已完成构建/测试验证；真实 GUI 渲染仍为 `BLOCKED`。
- 2026-09-10 最新 Linux reconciliation：重新执行 `npm run check`、`npm run test`、`npm run build`、`npm run build:tauri`、`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 和 `python3 -m compileall -q sidecar/src sidecar/tests`，均通过；Rust workspace 为 69/69（本轮新增后最新总数为 79/79，见后续开发记录），Extension 为 6/6，Desktop Node 测试为 0 项且无失败。Windows 最新记录中的 Node/Rust/Sidecar/Tauri 基础复验继续有效；真实 GUI WebView2/DPI/键盘/辅助技术/对比度仍为 `WINDOWS_VERIFICATION_PENDING`/`BLOCKED`，文件 SQLite 应用级恢复/迁移仍为 `NOT RUN`。
- 2026-09-10 继续 Linux 开发：`xarchive-download` 新增纯 Rust `DownloadRouter`，覆盖 gallery-dl 默认路径、`EXTRACT_OR_DOWNLOAD_FAILED` → aria2 fallback、认证错误不回退、fallback 禁用、aria2 未配置和双失败原因保留；`cargo fmt --all -- --check` 与 `cargo test -p xarchive-download` 通过，目标 crate 16/16。该实现尚未接入真实 Desktop/Sidecar Job 调度，不提前宣称端到端完成。
- 2026-09-10 继续 Linux 开发：`xarchive-native-host` 新增 `XARCHIVE_PIPE_ENDPOINT` 配置的请求转发核心、响应协议版本/request_id 校验和 fake duplex transport 测试；Native Host crate 8/8、workspace `cargo check --workspace` 与 `cargo test --workspace` 通过。该实现尚未在 Windows Named Pipe endpoint、ACL、Registry 或浏览器实机上验证，相关状态保持 `WINDOWS_VERIFICATION_PENDING` / `WINDOWS_BLOCKED`。
- 2026-09-12 继续 Linux 开发（deferred Windows validation 批次）：完成 M5 Quote/Reply 建模跨层实现——`xarchive-protocol` `BrowserTweet` 新增嵌套 `quoted_tweet`（`Box`）与 `reply_to` 字段及递归校验（3 项新测试），shared JSON Schema 同步；Extension content script 新增 DOM quote card 嵌套提取（1 项新测试，共 7/7）；`xarchive-storage` 新增 migration `0003_quote_reply_relationships.sql`（`tweets.reply_to_tweet_id`/`quoted_tweet_id`）、`update_tweet_metadata` 关系持久化、`tweet_relationships` 查询和 legacy 数据库迁移升级测试（2 项新测试，共 18/18）；Desktop `archive_tweet` 新增 `merge_browser_relationships`，将浏览器 DOM 提供的 `reply_to`/`quoted_tweet` 填入 Sidecar 元数据空缺且不覆盖 Sidecar 已有数据（3 项新测试，Desktop crate 8/8）。Linux 验证：`cargo fmt --all`、`cargo check --workspace --all-targets`、`cargo test --workspace`（87 项 crate 测试）和 `python3 -m compileall -q sidecar/src` 均通过。Windows 相关新增代码随累计 working tree 进入 Windows Validation Queue，见 `windows-validation.md`。

- 2026-09-12 继续 Linux 开发（deferred Windows validation 批次）：完成 M5 profile 文件——`xarchive-storage` 新增 `UserProfileSnapshot`/`UserProfileFile` 类型（serde Serialize/Deserialize）、`Database::user_profile()` 快照查询（最新名称 + 完整名称历史）、`FileStore::write_user_profile()` 写入 `Users/<stable>/profile.json`、`ArchiveService::refresh_author_profile()` 在 `complete_local_archive` 中自动注册作者并刷新 profile（`user_id` 缺失时静默跳过）、`upsert_user` 增加名称去重逻辑（username 与 display_name 均未变化时不重复记录）。扩展现有测试：`persists_user_name_history_and_stable_directory_name` 增加名称去重断言、`completes_local_archive_and_writes_portable_metadata` 扩展为 profile.json 内容（schema_version/user_id/stable_directory_name/username/display_name/updated_at/names）与 tweet-user 关联断言（storage 测试 18 → 19）。修复编译错误：`UserNameSummary` 增加 `Deserialize` derive、`serde::Deserialize` 导入、移除测试中误插入的裸中文文本行。Linux 验证：`cargo fmt --all -- --check`、`cargo check -p xarchive-storage`、`cargo test --workspace`（storage 19/19，workspace 共 88 项）和 `python3 -m compileall -q sidecar/src` 均通过。profile 文件的 Windows 文件数据库迁移升级与 Unicode 路径写入已加入 Windows Validation Queue。

- 2026-09-12 Windows 验证结果 reconciliation：Windows 对同一 working tree 完成次轮自动化复验（88/88 crate tests with PYTHON preset、Node check/test/build 7/7、Sidecar pytest 10/10、Tauri Release build 与 Debug startup 均 PASS）；默认 Windows 命令环境 `PYTHON` 为空导致 2 项 supervisor 真实 worker 测试 `NotRunning`（环境前置条件 FAIL，非代码失败）；当前没有可归因于 M5 业务代码的 Windows 自动化 FAIL。WQ-M5-05 BLOCKED（GUI helper + 无浏览器/账号），WQ-M5-06/07/08/10/11 NOT RUN（缺少受控 gallery-dl payload、旧版数据库副本、Desktop 应用级归档驱动）。M5 Linux 代码层全部完成，无需额外 Linux 开发。

- 2026-09-12 Windows 验证结果 reconciliation（第三轮，2026-09-12 13:34–13:42）：Windows 对同一 working tree 完成第三轮自动化复验（88/88 crate tests with PYTHON preset、M5 Quote/Reply + Profile 定向回归 6/6、Node check/test/build 7/7、Sidecar pytest 10/10、Tauri Release build 与 Debug startup 均 PASS）；Computer Use 浏览器只读 AX 探测 PASS（`cua.getState()` Edge tab 检测可用，未点击或改变浏览器状态）；原生 Windows 桌面 Computer Use 服务 `sky.list_apps()` 返回 `Trusted RPC service is not configured: sky`，无法进行 Tauri 原生窗口交互验收；GUI WebView2/DPI/键盘/辅助技术/端到端仍为 BLOCKED。M5 Linux 代码层全部完成，无需额外 Linux 开发。

## 当前 Linux 验证限制

- 当前 Linux 环境的 `cargo-clippy` 组件未安装，因此本轮未重新执行 clippy；Windows 最新一次针对旧 working tree 的 clippy 曾因 `run_sidecar_download` 参数过多失败，该项目问题已在 Linux 通过 `SidecarDownloadRequest` 重构修复，但 Windows 严格 clippy 复验已针对最新 HEAD `040b982` 多轮实际执行并通过。Windows Supervisor 测试另需显式设置 `PYTHON` 指向项目 `.venv\\Scripts\\python.exe`，这是环境前提而非代码失败。
- 当前 Linux 环境未安装 `pytest`，因此本轮未重新执行 `sidecar/tests`；Windows 既有 10 个 Sidecar 测试结果继续作为 Windows 基线。
- Rust workspace 当前本地全量测试为 88 个 crate 单元测试（11 core、5 desktop、16 download、8 Native Host、7 protocol、4 supervisor、19 storage、12 Telegram），全部通过；Extension Node 测试 7 个，全部通过。
- 2026-09-09 新增 Telegram 发送状态持久化与幂等补传的 Linux 验证：`xarchive-telegram` 新增 `SendState`/`SendStateStore` 契约、`send_idempotently` 幂等补传编排和 `TelegramResponse.result_message_id`（4 项新测试）；`xarchive-storage` 通过 SQLite migration `0002_telegram_send_state.sql` 新建 `telegram_send_attempts` 表（`UNIQUE(chat_id, idempotency_key)`、state/attempt_count/error 字段）并为 `Database` 实现 `SendStateStore`（3 项新测试），crate 单元测试总数由 62 增至 69。`send_idempotently` 保证同一 `(chat_id, idempotency_key)` 的投递闭包跨重启至多执行一次，失败记录可通过 `list_unsent()` 重试。

- Windows 尚未验证或尚未实现的项目均对应平台集成、外部环境或发布 artifact：Edge Cookie、真实 X 归档、Named Pipe、Native Host 注册、浏览器安装、Tauri GUI/安装包、真实 externalBin Sidecar、Credential Manager、Tray/Autostart 和真实 Telegram 发送。`Aria2Supervisor` 基础进程监督层、Desktop aria2 管理 UI 和纯 Rust `DownloadRouter` 已作为跨平台代码完成，并通过 Linux 配置/错误/构建/单元测试；Windows `d239a1d` 证据已实际覆盖 aria2c.exe 下载、解压、版本/hash、RPC、Unicode/空格路径、Range 续传、进程恢复和 `.aria2` 清理，因此后续只需验证 Router 与实际 Sidecar/Job 调度的业务接入、403 回退 gallery-dl、externalBin/打包分发和默认 Download Router 端到端链路。Telegram HTTPS transport 已作为跨平台代码完成，使用 `reqwest 0.13.4` + Rustls 并通过 fake-server 测试，但不等于真实账号发送验证。开发阶段 `icons/icon.ico` 已补齐，Windows workspace Debug/Release 编译、Tauri 开发启动和 Release 构建已通过；根 workspace 和 Desktop workspace 已安装 Tauri CLI 2.11.4 并提供 `dev:tauri`/`build:tauri` 脚本。UI 自动化 helper 初始化失败，因此 GUI 真实视觉、WebView2/DPI、键盘和辅助技术验收仍未执行；停止开发进程时记录 Chromium `Error = 1411/1412` 注销警告。Supervisor 测试默认 `python3` 不存在，已通过显式 `PYTHON` 指向项目 `.venv\\Scripts\\python.exe` 复验。公开示例 URL 的 `EXTRACT_OR_DOWNLOAD_FAILED` 仅作为失败链路记录，不能替代真实账号验证。E 盘首次 Node 检查缺少 `vite`，已通过项目内 `npm ci` 补齐依赖后复验；npm 提示 `esbuild` postinstall script 尚未批准。

- 跨平台已完成清单见 [`non-windows-completion.md`](non-windows-completion.md)。Telegram 发送持久化、aria2c.exe 实际运行/恢复、artifact 分发和更完整业务 GUI 不是 Windows 验证本身，需按该清单单独继续开发。

一次并行 Windows 验证中，`xarchive-storage::completes_archive_directly_from_sidecar_result` 曾偶发报路径不存在；目标测试单独重跑及串行完整 workspace 均通过，暂列为需后续观察的测试稳定性问题。

完整的 Windows 实机、Windows CI、安装器和发布验证项目见 [`windows-validation.md`](windows-validation.md)。
