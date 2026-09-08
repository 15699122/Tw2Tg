# 测试策略

## 单元测试

- Rust：状态机、幂等、metadata 合并、路径清洗、hash、错误分类、TagEngine。
- Python：JSONL、gallery-dl 归一化、事件、错误映射、stdout/stderr 隔离。
- TypeScript：DOM 提取、按钮去重、状态映射、批量查询和消息路由。

## 契约测试

使用 `shared/protocol-schema/fixtures/`，在 Rust、Python 和 TypeScript 中验证相同的有效/无效消息。

## 集成测试

Rust + Fake Sidecar、Rust + Real Sidecar、临时 SQLite、临时 FileStore、Mock Telegram、Native Host framing、Named Pipe 和 aria2 本地 HTTP Server。

## 故障注入

覆盖 Sidecar/aria2 崩溃、JSON 截断、认证失败、URL 过期、SQLite busy、磁盘不足、文件锁、Telegram timeout/rate limit、Desktop 中途退出和 Native Host 断开。

## 平台验证

Linux 验证协议和跨平台代码；Windows 必须验证 Named Pipe、Cookie、Native Host Registry、Tauri Sidecar 打包、长路径和安装/卸载。

## 已完成的平台验证

截至 **2026-09-08**，Windows 已完成：

- Node workspace 检查、测试和构建。
- Rust workspace check/test 已在补齐 `desktop/src-tauri/icons/icon.ico` 后恢复；当前 Linux 完整 workspace 29 个单元测试通过，Windows 既有核心 clippy/check/test 结果需重跑 Desktop 相关验证。
- Python `.venv` editable 安装 Sidecar、gallery-dl 1.32.11 后的 10 个 Sidecar 测试。
- Rust Supervisor 与真实 Python Worker 的进程集成测试已在开发环境通过。
- Tauri Desktop 的 Vite/React 构建、启动时 SQLite 初始化、状态 commands、Sidecar `hello → ready` 握手、开发控制按钮和最近 Job 查询已通过；真实 externalBin Sidecar 和 Windows GUI/打包尚未验证。
- `cargo fmt --all -- --check` 已通过。
- 核心 workspace 的 `cargo clippy --workspace --exclude xarchive-desktop --all-targets -- -D warnings` 通过；此前在 `crates/xarchive-sidecar-supervisor/src/lib.rs:17` 检出的 `large_enum_variant` 已通过 `Box<DownloadEvent>` 修复并在 Windows 复验。
- 真实 sidecar 已在含中文、空格和 Unicode 的路径中完成 JSONL 启动/下载/失败/退出链路验证；示例 X URL 未完成提取，真实账号下载仍待验证。

Windows 尚未验证的项目均对应尚未实现或需要外部环境的功能：Edge Cookie、真实 X 归档、Named Pipe、Native Host 注册、浏览器安装、Tauri Windows GUI/Release/打包和真实 externalBin Sidecar。开发阶段 `icons/icon.ico` 已补齐并在 Linux 解除构建阻塞；公开示例 URL 的 `EXTRACT_OR_DOWNLOAD_FAILED` 仅作为失败链路记录，不能替代真实账号验证。E 盘首次 Node 检查缺少 `vite`，已通过项目内 `npm ci` 补齐依赖后复验。

一次并行 Windows 验证中，`xarchive-storage::completes_archive_directly_from_sidecar_result` 曾偶发报路径不存在；目标测试单独重跑及串行完整 workspace 均通过，暂列为需后续观察的测试稳定性问题。

完整的 Windows 实机、Windows CI、安装器和发布验证项目见 [`windows-validation.md`](windows-validation.md)。
