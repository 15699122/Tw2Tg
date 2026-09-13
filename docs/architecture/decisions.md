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

### R1 目标设计

R1 的第一版采用单个应用级 executor 管理多个归档 Job。Tauri command 不直接持有可执行资源，也不执行 Sidecar、网络或文件 I/O；command 只通过 executor handle 完成输入校验、Job 创建/复用和控制消息投递。

```text
Tauri command
  → ArchiveApplicationService::submit/query/cancel
  → JobExecutorHandle (bounded command channel)
  → JobExecutor worker
      → 为单个 Job 打开 Database 访问上下文
      → 创建/使用 FileStore staging
      → 独占 SidecarSupervisor lease
      → DownloadRouter / Sidecar I/O
      → ArchiveService 提交 staging 与数据库状态
      → 持久化 JobEvent 和最终状态
```

#### 所有权和锁边界

- `RuntimeState` 只保存稳定应用资源和轻量句柄：archive root、Database 初始化/恢复信息、Sidecar 配置、`JobExecutorHandle`、应用关闭 token 和错误快照。
- `RuntimeState` 的 Mutex 只保护句柄替换、健康状态、关闭状态和启动/停止短操作；不得跨越下载、等待 Sidecar 事件、文件 hash 或最终 commit。
- `Database` 由 executor/application service 通过明确的 Job 上下文访问。第一版可以使用单独的 storage connection 或受控 connection factory，不把长任务包在 RuntimeState 锁内。
- `FileStore` 是无长生命周期共享状态的归档根适配器；Job 只持有自己的 staging lease 和相对路径结果。
- `SidecarSupervisor` 由 executor 独占。并发 Job 不共享同一个 supervisor；若 Sidecar worker 仍是单进程，则 executor 必须串行化需要 Sidecar 的阶段，并允许查询/取消 command 独立运行。

#### Job 生命周期和控制语义

- `submit` 先执行 BrowserRequest 校验，再以现有 `create_archive_job` 规则创建或复用 active Job；重复请求返回现有 Job，不创建第二个 worker。
- worker 启动后沿用现有 `QUEUED → VALIDATING → METADATA_READY → DOWNLOADING → DOWNLOADED/COMPLETE` 语义，不增加未经必要的新持久化状态。
- `cancel` 是幂等控制请求：未开始的 Job 进入 `INTERRUPTED`；正在下载的 Job 先发送 Sidecar shutdown/取消动作，停止接收新事件，清理或保留 staging 后持久化 `INTERRUPTED` 和取消事件；已完成/失败/认证要求 Job 返回当前终态而不重复修改。
- `stop_sidecar` 不得静默破坏运行中的 Job。它应向 executor 发送 shutdown 请求，由 executor 将受影响 Job 标记为 `INTERRUPTED` 或明确的内部失败，并记录事件。
- Sidecar 非预期退出时，当前 Job 必须记录安全的 `SIDECAR_INTERNAL_ERROR`，并根据是否已有可恢复 staging 进入 `FAILED` 或 `INTERRUPTED`；不能继续使用失效 supervisor。
- 应用关闭先停止接受新 Job，再等待 worker 的有限 grace period；超时 Job 记录 interrupted/shutdown 事件，遗留 staging 由下一次启动恢复扫描处理。

#### 恢复和幂等不变量

- 启动恢复只处理数据库中 active/queued Job，不重放已完成 Job。
- 每个 Job 的 `job_id`、request identity、staging directory 和 JobEvent 顺序必须稳定可追踪。
- 最终目录 commit 成功但进程在状态更新前退出时，恢复流程必须通过 metadata/目录和数据库状态检查决定补写完成状态或标记人工可诊断失败；不得重复覆盖已有合法归档。
- 文件校验、identity binding 和 reparse/path 防护仍由 Storage/ArchiveService 负责，executor 不复制这些安全规则。
- 一个 Job 只能有一个 active worker；取消、Sidecar 退出和应用关闭必须通过同一控制路径串行化。

#### 分阶段实现顺序

1. 先增加 `ArchiveApplicationService`/`JobExecutorHandle` 的纯 Rust command/state model，以及 fake worker 测试。
2. 增加并发 submit、重复请求、cancel、shutdown、Sidecar crash 和恢复测试，不接入真实 Tauri command。
3. 接入现有 `archive_tweet` 的 Job 创建/复用路径，保持旧同步实现作为受控 fallback，完成行为对照测试。
4. 将 Sidecar/FileStore/ArchiveService I/O 移入 worker，并删除 RuntimeState 长锁覆盖范围。
5. 最后补充应用退出、真实 Sidecar 和 Windows runtime 回归验证。

### R1 验收测试矩阵

| 场景 | 必须验证的结果 |
|---|---|
| 两个不同 Tweet 同时 submit | Job 不互相阻塞查询/控制；资源所有权明确；状态和事件不串线 |
| 同一 Tweet 重复 submit | 复用 active Job，只有一个 worker 和一组 staging |
| queued Job cancel | 不启动 Sidecar，状态和取消事件持久化为 `INTERRUPTED` |
| 下载中 cancel | 控制请求可达，Sidecar 停止，staging 不逃逸，Job 有明确终态 |
| Sidecar 非预期退出 | 记录 `SIDECAR_INTERNAL_ERROR`，失效 supervisor 不再复用 |
| 应用 shutdown | 停止接收新 Job，有限等待，遗留 active Job 可在下次启动恢复 |
| commit 前后进程退出 | 不产生重复归档或错误地报告 COMPLETE |
| RuntimeState 健康查询 | 长时间 I/O 期间仍可获取健康、Job 和控制状态 |

### 后果

该决策预计会改变 `archive_tweet` 的返回时机和 RuntimeState 结构，因此当前仅记录设计边界，不将其标记为已完成，也不因该项要求提前进行 Windows 验证。