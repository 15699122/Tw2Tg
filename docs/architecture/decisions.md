# 架构决策记录

## ADR-001：Rust 是唯一业务状态所有者

**状态：已接受**

SQLite、Job 状态、文件提交、去重、Telegram 和恢复全部由 Rust 管理。Python 只做受控执行，避免多个事实来源不一致。

## ADR-002：使用独立 Native Host

**状态：已接受**

浏览器连接极小的 Rust Bridge，Bridge 再通过 Windows Named Pipe 连接 Desktop。Tauri 进程不直接作为浏览器 Native Host。

## ADR-003：Python Sidecar 使用 JSONL stdio

**状态：已接受**

不开放 Sidecar localhost HTTP，减少端口、鉴权和进程暴露。stdout 是机器协议，stderr 是诊断日志。

## ADR-004：gallery-dl 是默认 X extractor 和下载器

**状态：已接受**

gallery-dl 负责适应 X 变化、读取浏览器登录状态、提取 metadata 和第一版媒体下载。

## ADR-005：aria2 只作为可选 DownloadTransport

**状态：条件接受**

aria2 的 RPC、断点续传和进度适合大直链文件，但不理解 Tweet，也可能扩大 Cookie/Header 传播范围。M1.5 完成技术 Spike 后再决定是否正式启用。

当前已完成请求模型、状态映射和安全校验；尚未启动真实 aria2c 或将其加入默认下载路由。

跨平台的 loopback HTTP JSON-RPC client 和基础 `Aria2Supervisor` 已在 `xarchive-download` 实现。Linux 测试覆盖本地 fake server、配置校验、aria2 参数构造、进程启动失败和 secret 脱敏；真实 aria2c.exe 生命周期、断点恢复、崩溃恢复、artifact 分发和 Windows 验证仍不属于本阶段已完成内容。

## ADR-006：IDM 不作为核心后端

**状态：已拒绝作为核心**

官方 CLI 能提交 URL，但缺少足够的公开结构化状态接口来可靠映射 Job 状态、进度、失败和恢复。未来只考虑实验性外部提交。

## ADR-007：原始媒体优先

**状态：已接受**

本地保存原文件，不默认转码，不默认删除 hash 重复文件。

## ADR-008：跨平台能力先于 Windows 适配

**状态：已接受**

协议模型、Native Messaging framing、Extension 纯逻辑、aria2 RPC client、retry policy、TagEngine、Telegram request/formatter 和 SecretStore abstraction 必须先在不依赖 Windows 的环境中完成并测试。Named Pipe、Registry、Credential Manager、Tray、Autostart、安装器和真实 Edge Cookie 作为平台适配层单独实现和验证。

## ADR-009：后台 Job executor 与 Tauri command 解耦

**状态：提议，尚未实现**

### 背景

当前 `desktop/src-tauri/src/lib.rs::archive_tweet` 在 Tauri command 生命周期内持有全局 `RuntimeState` 锁，并执行最长约 15 分钟的 Sidecar/文件 I/O。这样会阻塞状态查询、Sidecar 停止、设置读取和其他 Job 的并发处理。

### 决策方向

将归档流程拆成：

```text
Tauri command
  → 输入校验与 Job 创建/复用
  → 后台 executor 投递
  → 短生命周期 Database/FileStore 事务
  → Sidecar/Download backend 控制 channel
  → JobEvent 和状态持久化
```

Tauri command 只负责 IPC adapter；后台 executor 负责网络、进程、文件和 Telegram I/O。RuntimeState 不应作为覆盖整个下载生命周期的互斥锁使用。

### 约束

- 不改变 Job 状态机的业务语义。
- 取消、Sidecar 崩溃、应用退出和重复请求必须有明确状态。
- 数据库连接、Sidecar supervisor 和 FileStore 的所有权必须在设计中明确。
- 先增加并发/取消/恢复测试，再替换当前同步实现。
- 不与行为不变的文件移动混在同一批次。

### 后果

该决策预计会改变 `archive_tweet` 的返回时机和 RuntimeState 结构，因此当前仅记录设计边界，不将其标记为已完成，也不因该项要求提前进行 Windows 验证。