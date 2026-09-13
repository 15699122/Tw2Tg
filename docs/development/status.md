# 当前开发状态

> 本文记录当前实现事实，不替代逐轮验证报告，也不记录已经关闭的历史问题。

## 已实现

- Rust 核心 Job 状态、重试策略、TagEngine 和 Windows-safe 用户目录名。
- Native Host framing、forwarding 和错误处理已按职责拆分为独立模块，公共 API 保持不变。
- Sidecar Supervisor 已按进程监督、错误、事件和 stdout/stderr reader 拆分为独立模块，公共 API 保持不变。
- Protocol crate 已按 Browser、Sidecar、JSONL 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- Download crate 已按 model、router、RPC、HTTP client、supervisor 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- 版本化跨进程协议、JSON Schema、Native Messaging framing 和协议校验。
- Python gallery-dl Sidecar、JSONL worker、metadata 归一化和媒体文件事件。
- SQLite users、user names、tweets、media、jobs、events、tags、Telegram send state 和关系数据。
- staging → Rust 校验 → 最终归档目录的文件提交流程。
- Tweet/URL identity binding、Sidecar metadata identity binding、settings 输入限制和 Sidecar 错误脱敏。
- Tauri Desktop runtime、Job 查询、Sidecar lifecycle、aria2 discovery 和 React Dashboard。
- MV3 Extension 的 Tweet DOM 提取、归档按钮、状态查询和 Native Messaging bridge。
- Telegram request/formatter/transport contract、SecretStore abstraction 和幂等发送状态模型。

## 部分实现

- `DownloadRouter` 已完成跨平台策略和单元测试，但真实 aria2 fallback、403 后重新提取 URL、Desktop transfer lifecycle 尚未形成完整应用链路。
- Native Host 的 framing、校验和可插拔 forwarding 已完成；Windows Named Pipe server、ACL、Registry 和浏览器安装仍未完成。
- GUI 的源码级状态、语义结构、焦点样式和视觉 token 已完成；真实 WebView2、DPI、键盘、屏幕阅读器和对比度仍需 Windows 验收。
- Telegram 的跨平台 transport 和发送状态模型已完成；Credential Manager、真实账号和生产发送链路仍未完成。

## 未实现或未完成

- Desktop 应用级后台 Job executor、取消和并发模型；当前 `archive_tweet` 仍在 Tauri command 生命周期内持有全局 RuntimeState 锁执行长时间 Sidecar I/O。
- Windows Named Pipe server、Native Host manifest/Registry、Tray、Single Instance、Autostart 和 Credential Manager。
- Sidecar `externalBin`、正式 bundle、安装器、签名和 updater。
- 真实 Edge Cookie/X 认证归档和真实 Telegram 账号发送。
- 应用级旧 SQLite 启动迁移、重启恢复和跨用户 ACL 验证。

## 当前开发方向

1. 文档事实源、文件职责地图和 Windows Validation Queue 已完成第一轮整理。
2. 对大型 Rust/Python/React 文件进行行为不变的模块化拆分。
3. 单独设计并实现后台 Job executor，解除 RuntimeState 长锁。
4. 再推进 Windows 平台适配、真实账号链路和发布打包。

## 验证状态

- Linux Rust fmt/check/test、Node check/test/build、Python compileall 和 JSON Schema parse 已通过最近一次验证。
- Linux `cargo clippy` 和 `pytest` 是否可执行取决于当前环境；缺少工具时必须标记 `NOT RUN`。
- Windows 专属项目不得因 Linux 通过而标记为 Windows PASS；当前队列以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准，历史证据以 Windows 验证记录为准。

## 相关文档

- 未来方向：[`roadmap.md`](roadmap.md)
- 测试策略：[`testing.md`](testing.md)
- Windows 工作流：[`cross-platform-validation.md`](cross-platform-validation.md)
- Windows 执行规范：[`../validation/windows.md`](../validation/windows.md)