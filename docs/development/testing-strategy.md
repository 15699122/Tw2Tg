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
- Rust workspace 检查和 26 个测试。
- Python `.venv` editable 安装 Sidecar、gallery-dl 1.32.11 后的 10 个 Sidecar 测试。
- Rust Supervisor 与真实 Python Worker 的进程集成测试已在开发环境通过。
- `cargo fmt --all -- --check` 已通过。
- `cargo clippy --workspace --all-targets -- -D warnings` 曾失败于 `crates/xarchive-sidecar-supervisor/src/lib.rs:17` 的 `large_enum_variant`；现已将 `SupervisorEvent::Download` 改为 `Box<DownloadEvent>`，待 Windows 环境复验。
- 真实 sidecar 已在含中文、空格和 Unicode 的路径中完成 JSONL 启动/下载/失败/退出链路验证；示例 X URL 未完成提取，真实账号下载仍待验证。

Windows 尚未验证的项目均对应尚未实现或需要外部环境的功能：Edge Cookie、真实 X 归档、Named Pipe、Native Host 注册、浏览器安装和 Tauri 打包。公开示例 URL 的 `EXTRACT_OR_DOWNLOAD_FAILED` 仅作为失败链路记录，不能替代真实账号验证。`large_enum_variant` 修复后仍需在 Windows 环境重新执行 clippy。

完整的 Windows 实机、Windows CI、安装器和发布验证项目见 [`windows-validation.md`](windows-validation.md)。
