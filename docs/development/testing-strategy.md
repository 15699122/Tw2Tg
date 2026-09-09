# 测试策略

## 单元测试

- Rust：状态机、幂等、metadata 合并、路径清洗、hash、错误分类、retry/backoff、TagEngine、用户目录命名和 users/tags Repository。
- Python：JSONL、gallery-dl 归一化、事件、错误映射、stdout/stderr 隔离。
- JavaScript：DOM 提取、按钮去重、状态映射、批量查询、Native Bridge、request_id 路由和断线处理。

## 契约测试

使用 `shared/protocol-schema/fixtures/`，在 Rust、Python 和 TypeScript 中验证相同的有效/无效消息。

## 集成测试

- Rust + Fake Sidecar、Rust + Real Sidecar、临时 SQLite、临时 FileStore、Telegram contract/mock transport、Native Host framing、aria2 本地 HTTP fake server、`Aria2Supervisor` 配置/进程错误测试，以及 Desktop aria2 检测/版本 allowlist/SHA-256 helper 测试；Named Pipe、浏览器 Native Messaging、aria2c.exe 实际生命周期、官方 ZIP 下载/解压、安装器和真实账号链路属于 Windows 集成测试。

## 故障注入

覆盖 Sidecar/aria2 崩溃、JSON 截断、认证失败、URL 过期、SQLite busy、磁盘不足、文件锁、Telegram timeout/rate limit、Desktop 中途退出和 Native Host 断开。

## 平台验证

- Linux 验证协议、跨平台代码和 fake transport；Windows 必须验证 Named Pipe、Cookie、Native Host Registry、Tauri Sidecar 打包、长路径和安装/卸载。

## 已完成的平台验证

截至 **2026-09-09**，Windows 已完成：

- Node workspace 检查、测试和构建。
- Rust workspace 已在补齐 `desktop/src-tauri/icons/icon.ico` 后恢复；Windows workspace 的 fmt、check、test、严格 clippy、Debug/Release 和 Tauri Release 构建均通过，62 个 crate 单元测试全部通过。此前因 Desktop aria2 路径扫描 `collapsible_if` 的 clippy 失败已由 Linux let-chain 修复，Windows re-validation 已通过。Supervisor 测试需显式使用项目 `.venv\\Scripts\\python.exe`，否则默认 `python3` 不存在会导致两个测试报 `NotRunning`。
- Python `.venv` editable 安装 Sidecar、gallery-dl 1.32.11 后的 10 个 Sidecar 测试。
- Rust Supervisor 与真实 Python Worker 的进程集成测试已在开发环境通过。
- Tauri Desktop 的 Vite/React 构建、Windows Debug/Release 编译、Windows `npm run dev:tauri` 启动、`npm run build:tauri` Release 构建、项目内 Tauri CLI 入口声明、启动时 SQLite 初始化、状态/Job/目录 commands、Sidecar `hello → ready` 握手、开发控制按钮、打开归档目录、最近 Job 查询和本地 shadcn/ui 组件构建已通过；停止开发进程时记录 Chromium `Error = 1411` 注销警告，真实 externalBin Sidecar 和 Windows GUI/打包尚未验证。
- `cargo fmt --all -- --check` 已通过。
- 此前在 `crates/xarchive-sidecar-supervisor/src/lib.rs:17` 检出的 `large_enum_variant` 已通过 `Box<DownloadEvent>` 修复并在 Windows 复验；Telegram formatter 的 `single_char_add_str` 已修复；Desktop aria2 路径扫描的 `collapsible_if` 已在 Linux 用 let-chain 修复，Windows 完整 workspace clippy re-validation 已通过。
- 真实 sidecar 已在含中文、空格和 Unicode 的路径中完成 JSONL 启动/下载/失败/退出链路验证；示例 X URL 未完成提取，真实账号下载仍待验证。
- 2026-09-09 针对 Linux revision `add84c0`（干净 working tree）的 Windows 全量复验：Node check/test/build、Rust fmt/check/test、严格 `cargo clippy -D warnings`、`.venv` pytest、`npm run build:tauri` 和 `npm run dev:tauri` 全部 PASS；`collapsible_if` clippy 失败链路正式闭环。本轮 Windows 报告 Rust 测试 68 项，与 Linux 同 revision 复测的 69 项（11/5/10/4/7/4/16/12）存在差一差异，Windows 自报分项（storage 16、telegram 12）与 Linux 一致；差异按待复核记录，下一轮 Windows 验证按 crate 清点回填。同日 Windows 针对 `4d4b3f5` 复验并按 crate 清点确认 69 项，68/69 差异已关闭（确认为计数笔误）；随后又对纯文档 revision `facd8d7` 复验，69 项结果保持一致。

## 当前 Linux 验证限制

- 当前 Linux 环境的 `cargo-clippy` 组件未安装，因此本轮未重新执行 clippy；Windows 最新 workspace clippy（含 let-chain 修复后的 re-validation）已执行并通过。Windows Supervisor 测试另需显式设置 `PYTHON` 指向项目 `.venv\\Scripts\\python.exe`，这是环境前提而非代码失败。
- 当前 Linux 环境未安装 `pytest`，因此本轮未重新执行 `sidecar/tests`；Windows 既有 10 个 Sidecar 测试结果继续作为 Windows 基线。
- Rust workspace 当前本地全量测试为 69 个 crate 单元测试（11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram），全部通过；Extension Node 测试 6 个，全部通过。
- 2026-09-09 新增 Telegram 发送状态持久化与幂等补传的 Linux 验证：`xarchive-telegram` 新增 `SendState`/`SendStateStore` 契约、`send_idempotently` 幂等补传编排和 `TelegramResponse.result_message_id`（4 项新测试）；`xarchive-storage` 通过 SQLite migration `0002_telegram_send_state.sql` 新建 `telegram_send_attempts` 表（`UNIQUE(chat_id, idempotency_key)`、state/attempt_count/error 字段）并为 `Database` 实现 `SendStateStore`（3 项新测试），crate 单元测试总数由 62 增至 69。`send_idempotently` 保证同一 `(chat_id, idempotency_key)` 的投递闭包跨重启至多执行一次，失败记录可通过 `list_unsent()` 重试。

- Windows 尚未验证或尚未实现的项目均对应平台集成、外部环境或发布 artifact：Edge Cookie、真实 X 归档、Named Pipe、Native Host 注册、浏览器安装、Tauri GUI/安装包、真实 externalBin Sidecar、aria2c.exe 实际运行与分发、aria2 官方 ZIP 下载/PowerShell 解压、Credential Manager、Tray/Autostart 和真实 Telegram 发送。`Aria2Supervisor` 基础进程监督层和 Desktop aria2 管理 UI 已作为跨平台代码完成，并通过 Linux 配置/错误/构建测试；Windows 曾检出 Desktop aria2 路径扫描的 `clippy::collapsible_if`，Linux 已使用 let-chain 修复并通过 Windows clippy re-validation，但这仍不等于 Windows aria2c.exe 下载、解压、生命周期、断点或打包验证。Telegram HTTPS transport 已作为跨平台代码完成，使用 `reqwest 0.13.4` + Rustls 并通过 fake-server 测试，但不等于真实账号发送验证。开发阶段 `icons/icon.ico` 已补齐，Windows workspace Debug/Release 编译、Tauri 开发启动和 Release 构建已通过；根 workspace 和 Desktop workspace 已安装 Tauri CLI 2.11.4 并提供 `dev:tauri`/`build:tauri` 脚本。UI 自动化 helper 初始化失败，因此 GUI 视觉与安装器验证仍未执行；停止开发进程时记录 Chromium `Error = 1411` 注销警告。Supervisor 测试默认 `python3` 不存在，已通过显式 `PYTHON` 指向项目 `.venv\\Scripts\\python.exe` 复验。公开示例 URL 的 `EXTRACT_OR_DOWNLOAD_FAILED` 仅作为失败链路记录，不能替代真实账号验证。E 盘首次 Node 检查缺少 `vite`，已通过项目内 `npm ci` 补齐依赖后复验；npm 提示 `esbuild` postinstall script 尚未批准。

- 跨平台已完成清单见 [`non-windows-completion.md`](non-windows-completion.md)。Telegram 发送持久化、aria2c.exe 实际运行/恢复、artifact 分发和更完整业务 GUI 不是 Windows 验证本身，需按该清单单独继续开发。

一次并行 Windows 验证中，`xarchive-storage::completes_archive_directly_from_sidecar_result` 曾偶发报路径不存在；目标测试单独重跑及串行完整 workspace 均通过，暂列为需后续观察的测试稳定性问题。

完整的 Windows 实机、Windows CI、安装器和发布验证项目见 [`windows-validation.md`](windows-validation.md)。
