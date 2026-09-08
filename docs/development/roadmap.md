# 开发路线图

## M0：Rust ↔ Python 单 Tweet 下载

建立 Sidecar 握手、JSONL 命令和事件、Fake Worker、gallery-dl Adapter、staging 下载和崩溃检测。

## M1：SQLite 与本地归档

加入 migrations、Archive Manager、Job 状态机、Metadata Merger、FileStore、JSON/TXT、Tweet ID 幂等、恢复和 SHA-256。

当前进度：已完成 Job 状态机、SQLite migration、SQLite 基础 Repository、staging FileStore、SHA-256、ArchiveService、gallery-dl CLI Adapter、JSONL Worker 集成、媒体文件扫描、`file`/`complete.files` 事件契约、Rust Sidecar 结果转换、ArchiveService 端到端闭环、Windows 基础验证、Tauri Desktop 脚手架和核心测试；`SupervisorEvent::Download` 的 `large_enum_variant` 已修复并通过 Windows clippy，真实 X 认证下载、Tauri Windows GUI/打包验证仍待实现。
已完成 Rust SidecarSupervisor 与真实 Python Worker 的 hello/download/shutdown 进程集成测试，并验证真实 sidecar 在 Unicode/空格路径中的 JSONL 失败链路。

## M1.5：aria2 技术验证

实现 `DownloadTransport` 抽象和 Rust aria2 Supervisor。验证 RPC、进度、取消、断点恢复、URL 过期、认证 Header、崩溃恢复和许可证分发要求。通过门槛后才加入 Automatic Router。

当前进度：已完成 `xarchive-download` 协议模型、RPC 请求构造、状态/字节数解析、安全校验和 fixture 测试；Windows Rust check/test/format/clippy 已通过；真实 aria2c Supervisor、HTTP/WebSocket client、断点恢复和 Windows 打包仍待实现。默认下载仍使用 gallery-dl。

## M2：Telegram

实现 SecretStore、Official API Transport、Formatter、TagEngine、media reply/group、长文本 continuation、幂等补传。

## M3：MV3 Extension 与 Native Messaging

实现 XDomAdapter、MutationObserver、按钮、Service Worker、Native Host、Named Pipe、批量状态同步和重连。

## M4：可靠性

重试退避、错误分类、Cancel、Sidecar/aria2 恢复、文件完整性扫描、URL 刷新和事件历史。

## M5：用户与标签

稳定用户目录、名称历史、profile 文件、Quote/Reply 建模和用户自定义 TagEngine 规则。

## M6：Tauri GUI

Dashboard、Archive、Users、Settings 和操作菜单。

当前已完成最小 Tauri/React 工程、启动时 SQLite 初始化、`get_app_status`/`get_archive_root`/`get_runtime_health` commands、开发配置和 Linux 构建验证；真实 Sidecar externalBin、完整业务 GUI 和 Windows GUI/打包仍待实现或验证。

## M7：安装与生命周期

Sidecar 打包、Tauri externalBin、Native Host manifest/Registry、Single Instance、Tray、Autostart、Windows 安装器。

Windows 专项任务、前置条件和验收标准集中维护在 [`windows-validation.md`](windows-validation.md)。

## M8：发布

签名更新、日志脱敏、备份恢复、诊断包、第三方许可证、版本回滚和发布 CI。

## 依赖顺序

```mermaid
flowchart TD
    A[M0 工程与协议] --> B[Fake/Real Sidecar]
    B --> C[M1 Archive + SQLite]
    C --> D[M1.5 aria2 Spike]
    C --> E[M2 Telegram]
    C --> F[M3 Native Host + Extension]
    D --> G[M4 Reliability]
    E --> G
    F --> G
    G --> H[M5 Users/Tags]
    H --> I[M6 GUI]
    I --> J[M7 Installer]
    J --> K[M8 Release]
```
