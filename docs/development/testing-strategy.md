# 测试策略

## 单元测试

- Rust：状态机、幂等、metadata 合并、路径清洗、hash、错误分类、retry/backoff、TagEngine、用户目录命名和 users/tags Repository。
- Python：JSONL、gallery-dl 归一化、事件、错误映射、stdout/stderr 隔离。
- JavaScript：DOM 提取、按钮去重、状态映射、批量查询、Native Bridge、request_id 路由和断线处理。

## 契约测试

使用 `shared/protocol-schema/fixtures/`，在 Rust、Python 和 TypeScript 中验证相同的有效/无效消息。

## 集成测试

- Rust + Fake Sidecar、Rust + Real Sidecar、临时 SQLite、临时 FileStore、Telegram contract/mock transport、Native Host framing、aria2 本地 HTTP fake server；Named Pipe、浏览器 Native Messaging、安装器和真实账号链路属于 Windows 集成测试。

## 故障注入

覆盖 Sidecar/aria2 崩溃、JSON 截断、认证失败、URL 过期、SQLite busy、磁盘不足、文件锁、Telegram timeout/rate limit、Desktop 中途退出和 Native Host 断开。

## 平台验证

- Linux 验证协议、跨平台代码和 fake transport；Windows 必须验证 Named Pipe、Cookie、Native Host Registry、Tauri Sidecar 打包、长路径和安装/卸载。

## 已完成的平台验证

截至 **2026-09-08**，Windows 已完成：

- Node workspace 检查、测试和构建。
- Rust workspace 已在补齐 `desktop/src-tauri/icons/icon.ico` 后恢复；Windows 完整 workspace 的 fmt、clippy、check、test 和 Release 编译均通过，29 个单元测试全部通过。
- Python `.venv` editable 安装 Sidecar、gallery-dl 1.32.11 后的 10 个 Sidecar 测试。
- Rust Supervisor 与真实 Python Worker 的进程集成测试已在开发环境通过。
- Tauri Desktop 的 Vite/React 构建、Windows Debug/Release 编译、Linux `npm run build:tauri` Release 构建、项目内 Tauri CLI 入口声明、启动时 SQLite 初始化、状态/Job/目录 commands、Sidecar `hello → ready` 握手、开发控制按钮、打开归档目录、最近 Job 查询和本地 shadcn/ui 组件构建已通过；真实 externalBin Sidecar 和 Windows GUI/打包尚未验证。
- `cargo fmt --all -- --check` 已通过。
- Windows 完整 workspace 的 `cargo clippy --workspace --all-targets -- -D warnings` 通过；此前在 `crates/xarchive-sidecar-supervisor/src/lib.rs:17` 检出的 `large_enum_variant` 已通过 `Box<DownloadEvent>` 修复并在 Windows 复验。
- 真实 sidecar 已在含中文、空格和 Unicode 的路径中完成 JSONL 启动/下载/失败/退出链路验证；示例 X URL 未完成提取，真实账号下载仍待验证。

## 当前 Linux 验证限制

- 当前 Linux 环境的 `cargo-clippy` 组件未安装，因此本轮未重新执行 clippy；Windows 既有完整 workspace clippy 结果继续作为 Windows 基线。
- 当前 Linux 环境未安装 `pytest`，因此本轮未重新执行 `sidecar/tests`；Windows 既有 10 个 Sidecar 测试结果继续作为 Windows 基线。
- Rust workspace 当前本地全量测试为 54 个 crate 单元测试（11 core、3 desktop、8 download、4 Native Host、7 protocol、4 supervisor、13 storage、4 Telegram），全部通过；Extension Node 测试 6 个，全部通过。

- Windows 尚未验证或尚未实现的项目均对应平台集成、外部环境或发布 artifact：Edge Cookie、真实 X 归档、Named Pipe、Native Host 注册、浏览器安装、Tauri GUI/安装包、真实 externalBin Sidecar、aria2c executable、Credential Manager、Tray/Autostart 和真实 Telegram 发送。开发阶段 `icons/icon.ico` 已补齐，Windows workspace Debug/Release 编译已通过，根 workspace 和 Desktop workspace 已声明 Tauri CLI 2.11.4 并提供 `dev:tauri`/`build:tauri` 脚本；UI 自动化 helper 初始化失败，因此 GUI 视觉与安装器验证仍未执行。公开示例 URL 的 `EXTRACT_OR_DOWNLOAD_FAILED` 仅作为失败链路记录，不能替代真实账号验证。E 盘首次 Node 检查缺少 `vite`，已通过项目内 `npm ci` 补齐依赖后复验。

跨平台已完成清单见 [`non-windows-completion.md`](non-windows-completion.md)。真实 Telegram HTTPS transport、发送持久化、aria2c executable supervisor 和更完整业务 GUI 不是 Windows 验证本身，需按该清单单独继续开发。

一次并行 Windows 验证中，`xarchive-storage::completes_archive_directly_from_sidecar_result` 曾偶发报路径不存在；目标测试单独重跑及串行完整 workspace 均通过，暂列为需后续观察的测试稳定性问题。

完整的 Windows 实机、Windows CI、安装器和发布验证项目见 [`windows-validation.md`](windows-validation.md)。
