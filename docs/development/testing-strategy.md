# 测试策略

## 单元测试

- Rust：状态机、幂等、metadata 合并、路径清洗、hash、错误分类、retry/backoff、下载后端路由、TagEngine、用户目录命名和 users/tags Repository。
- Python：JSONL、gallery-dl 归一化、事件、错误映射、stdout/stderr 隔离。
- JavaScript：DOM 提取、按钮去重、状态映射、批量查询、Native Bridge、request_id 路由和断线处理；Rust Native Host 负责 framing、请求校验和 transport 转发。

## 契约测试

使用 `shared/protocol-schema/fixtures/`，在 Rust、Python 和 TypeScript 中验证相同的有效/无效消息。

## 集成测试

- Rust + Fake Sidecar、Rust + Real Sidecar、临时 SQLite、临时 FileStore、Telegram contract/mock transport、Native Host framing/请求转发 fake transport、aria2 本地 HTTP fake server、`Aria2Supervisor` 配置/进程错误测试、`DownloadRouter` 默认/回退/不回退/双失败测试，以及 Desktop aria2 检测/版本 allowlist/SHA-256 helper 测试；Router 与真实 Sidecar/Job 调度的端到端接入、Windows Named Pipe server/ACL、浏览器 Native Messaging 实机、aria2c.exe 实际生命周期、官方 ZIP 下载/解压、安装器和真实账号链路属于后续集成或 Windows 验证。

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

## 当前 Linux 验证限制

- 当前 Linux 环境的 `cargo-clippy` 组件未安装，因此本轮未重新执行 clippy；Windows 最新 workspace clippy（含 let-chain 修复后的 re-validation）已执行并通过。Windows Supervisor 测试另需显式设置 `PYTHON` 指向项目 `.venv\\Scripts\\python.exe`，这是环境前提而非代码失败。
- 当前 Linux 环境未安装 `pytest`，因此本轮未重新执行 `sidecar/tests`；Windows 既有 10 个 Sidecar 测试结果继续作为 Windows 基线。
- Rust workspace 当前本地全量测试为 79 个 crate 单元测试（11 core、5 desktop、16 download、8 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram），全部通过；Extension Node 测试 6 个，全部通过。
- 2026-09-09 新增 Telegram 发送状态持久化与幂等补传的 Linux 验证：`xarchive-telegram` 新增 `SendState`/`SendStateStore` 契约、`send_idempotently` 幂等补传编排和 `TelegramResponse.result_message_id`（4 项新测试）；`xarchive-storage` 通过 SQLite migration `0002_telegram_send_state.sql` 新建 `telegram_send_attempts` 表（`UNIQUE(chat_id, idempotency_key)`、state/attempt_count/error 字段）并为 `Database` 实现 `SendStateStore`（3 项新测试），crate 单元测试总数由 62 增至 69。`send_idempotently` 保证同一 `(chat_id, idempotency_key)` 的投递闭包跨重启至多执行一次，失败记录可通过 `list_unsent()` 重试。

- Windows 尚未验证或尚未实现的项目均对应平台集成、外部环境或发布 artifact：Edge Cookie、真实 X 归档、Named Pipe、Native Host 注册、浏览器安装、Tauri GUI/安装包、真实 externalBin Sidecar、Credential Manager、Tray/Autostart 和真实 Telegram 发送。`Aria2Supervisor` 基础进程监督层、Desktop aria2 管理 UI 和纯 Rust `DownloadRouter` 已作为跨平台代码完成，并通过 Linux 配置/错误/构建/单元测试；Windows `d239a1d` 证据已实际覆盖 aria2c.exe 下载、解压、版本/hash、RPC、Unicode/空格路径、Range 续传、进程恢复和 `.aria2` 清理，因此后续只需验证 Router 与实际 Sidecar/Job 调度的业务接入、403 回退 gallery-dl、externalBin/打包分发和默认 Download Router 端到端链路。Telegram HTTPS transport 已作为跨平台代码完成，使用 `reqwest 0.13.4` + Rustls 并通过 fake-server 测试，但不等于真实账号发送验证。开发阶段 `icons/icon.ico` 已补齐，Windows workspace Debug/Release 编译、Tauri 开发启动和 Release 构建已通过；根 workspace 和 Desktop workspace 已安装 Tauri CLI 2.11.4 并提供 `dev:tauri`/`build:tauri` 脚本。UI 自动化 helper 初始化失败，因此 GUI 真实视觉、WebView2/DPI、键盘和辅助技术验收仍未执行；停止开发进程时记录 Chromium `Error = 1411/1412` 注销警告。Supervisor 测试默认 `python3` 不存在，已通过显式 `PYTHON` 指向项目 `.venv\\Scripts\\python.exe` 复验。公开示例 URL 的 `EXTRACT_OR_DOWNLOAD_FAILED` 仅作为失败链路记录，不能替代真实账号验证。E 盘首次 Node 检查缺少 `vite`，已通过项目内 `npm ci` 补齐依赖后复验；npm 提示 `esbuild` postinstall script 尚未批准。

- 跨平台已完成清单见 [`non-windows-completion.md`](non-windows-completion.md)。Telegram 发送持久化、aria2c.exe 实际运行/恢复、artifact 分发和更完整业务 GUI 不是 Windows 验证本身，需按该清单单独继续开发。

一次并行 Windows 验证中，`xarchive-storage::completes_archive_directly_from_sidecar_result` 曾偶发报路径不存在；目标测试单独重跑及串行完整 workspace 均通过，暂列为需后续观察的测试稳定性问题。

完整的 Windows 实机、Windows CI、安装器和发布验证项目见 [`windows-validation.md`](windows-validation.md)。
