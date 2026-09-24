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

## ADR-004：gallery-dl 是默认 X extractor 和下载器（历史，待 ADR-010 替代）

**状态：历史事实；目标架构中将被 ADR-010 替代（`SUPERSEDED_PENDING`）**

gallery-dl 负责适应 X 变化、读取浏览器登录状态、提取 metadata 和第一版媒体下载。

## ADR-005：aria2 只作为可选 DownloadTransport（历史，待 ADR-010 替代）

**状态：历史事实；目标架构中将被 ADR-010 替代（`SUPERSEDED_PENDING`）**

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

**状态：部分实现，继续收口中**

### 背景

U7 之前，`desktop/src-tauri/src/archive.rs::archive_tweet`（U8 已删除）在 Tauri command 生命周期内持有全局 `RuntimeState` 锁，并执行最长约 15 分钟的 Sidecar/文件 I/O。这样会阻塞状态查询、Sidecar 停止、设置读取和其他 Job 的并发处理。

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
      → Sidecar v2 extraction / aria2 transfer / FileStore I/O
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
- `cancel` 是幂等控制请求：用户主动取消的 Job 进入 `CANCELLED`；正在运行的 Job 先发送 Sidecar shutdown/取消动作，停止接收新事件，清理或保留 staging 后持久化 `CANCELLED` 和取消事件；应用关闭、崩溃或非用户中断使用 `INTERRUPTED`；已完成/失败/认证要求 Job 按状态机规则返回当前状态或进入 `CANCELLED`，不重复修改。
- `stop_sidecar` 不得静默破坏运行中的 Job。它应向 executor 发送 shutdown 请求，由 executor 将受影响 Job 标记为 `INTERRUPTED` 或明确的内部失败，并记录事件。
- Sidecar 非预期退出时，当前 Job 必须记录安全的 `SIDECAR_INTERNAL_ERROR`，并根据是否已有可恢复 staging 进入 `FAILED` 或 `INTERRUPTED`；不能继续使用失效 supervisor。
- 应用关闭先停止接受新 Job，再等待 worker 的有限 grace period；超时 Job 记录 interrupted/shutdown 事件，遗留 staging 由下一次启动恢复扫描处理。

#### 恢复和幂等不变量

- 启动恢复只处理数据库中 active/queued/`INTERRUPTED` Job，不重放已完成或用户主动取消的 `CANCELLED` Job。
- 每个 Job 的 `job_id`、request identity、staging directory 和 JobEvent 顺序必须稳定可追踪。
- 最终目录 commit 成功但进程在状态更新前退出时，恢复流程必须通过 metadata/目录和数据库状态检查决定补写完成状态或标记人工可诊断失败；不得重复覆盖已有合法归档。
- 文件校验、identity binding 和 reparse/path 防护仍由 Storage/ArchiveService 负责，executor 不复制这些安全规则。
- 一个 Job 只能有一个 active worker；取消、Sidecar 退出和应用关闭必须通过同一控制路径串行化。

#### 分阶段实现顺序

1. 先增加 `ArchiveApplicationService`/`JobExecutorHandle` 的纯 Rust command/state model，以及 fake worker 测试。当前 `desktop/src-tauri/src/executor.rs` 已完成该阶段，并已由 `RuntimeState` 持有 `ExecutorRuntime`；同时已增加 `JobPersistence` port、`JobDatabaseFactory`、in-memory/SQLite adapter 和 `ExecutorEvent` lifecycle model，用于验证 submit/query/recovery/event ordering contract，并证明 Job context 可以脱离 `RuntimeState` 长锁创建。
2. 增加并发 submit、重复请求、cancel、shutdown、Sidecar crash 和恢复测试，并接入最小 Tauri control command boundary。当前已覆盖 fake Sidecar crash、shutdown interruption、`SIDECAR_INTERNAL_ERROR` 映射、创建/下载开始/下载完成/下载失败/完成事件到核心 `JobEvent` 的审计映射、事件顺序、SQLite Job repository contract adapter、`JobSummary` 到 executor snapshot 的字段投影、事务性 `JOB_STATE_CHANGED` 去重、queued/interrupted recovery source state、persisted cancel 幂等与终态 no-op、persisted shutdown 的 active interruption 与 terminal skip、`DOWNLOADED → COMPLETE` completion contract、commit recovery decision/action、`CommitRecoveryFactsProvider` facts/snapshot 一致性、批量 mixed recovery、SQLite 状态/事件/错误字段顺序、单 Job 错误隔离、`EXECUTOR_UNAVAILABLE` submit compensation、shutdown interruption 与 worker shutdown 分离、`JobExecution` port 成功/失败/terminal skip、control worker 与 execution thread 分离以及 context lease 回收测试。`COMPLETE` 但 final archive 缺失时仍保留为诊断/人工处理边界。
3. 接入现有 archive Job 创建/复用路径；阶段二已完成 execution spec persistence、attempt fencing、单 active runner、runner-owned Database/FileStore/Sidecar context、`job_id` spec loading 和 startup recovery scan。U8 已删除 `archive_tweet` 同步 fallback，executor 命令成为唯一用户入口。
4. 真实 staging/final recovery action 已接入 startup recovery：先读取 final/staging filesystem facts，final 存在时补写 `COMPLETE`，仅 staging 存在时从持久化 `tweet.json` 重做本地 commit，二者缺失时记录可诊断失败。运行中 cancellation 已通过共享 token、Sidecar cancel/shutdown 和 late-result fencing 接入；后续评估用户入口切换和 Windows runtime 回归验证。
5. 当前 Linux 第一批入口调度统一已完成：Browser transport 与 Tauri `submit_executor_job` 都通过 `submit_and_schedule_persisted` 写入 Job/spec 后立即返回初始状态，由独立 orchestration thread 打开 persistence context 并调用 production execution factory。核心状态模型和 Sidecar error path 已区分用户取消与 shutdown interruption；U8 已删除同步 fallback，Desktop 全链路取消/恢复收口和 Sidecar process-tree 的平台级终止语义仍待 Windows runtime 验证。

### R1 验收测试矩阵

| 场景 | 必须验证的结果 |
|---|---|
| 两个不同 Tweet 同时 submit | Job 不互相阻塞查询/控制；资源所有权明确；状态和事件不串线 |
| 同一 Tweet 重复 submit | 复用 active Job，只有一个 worker 和一组 staging |
| queued Job cancel | 不启动 Sidecar，状态和取消事件持久化为 `CANCELLED` |
| 下载中 cancel | 控制请求可达，Sidecar 停止，staging 不逃逸，Job 有明确终态 |
| Sidecar 非预期退出 | 记录 `SIDECAR_INTERNAL_ERROR`，失效 supervisor 不再复用 |
| 应用 shutdown | 停止接收新 Job，有限等待，遗留 active Job 可在下次启动恢复 |
| commit 前后进程退出 | 不产生重复归档或错误地报告 COMPLETE |
| RuntimeState 健康查询 | 长时间 I/O 期间仍可获取健康、Job 和控制状态 |

### 后果

该决策已完成 Linux 第一批 submit/schedule 接入，核心状态模型已区分用户主动 `CANCELLED` 与 shutdown/崩溃导致的 `INTERRUPTED`；U8 已删除同步 fallback，Desktop 全链路的取消/恢复收口和平台级 process-tree 语义仍待 Windows runtime 验证，因此不能标记为 Windows 完整完成。

## ADR-010：目标媒体链路为 extraction-only 与 aria2-only transfer

**状态：已实现（U8 完成旧路径删除）**

目标终态只允许以下链路：

```text
gallery-dl extraction-only → typed ExtractionResult → Rust MediaTransferPlan → aria2 transfer
```

gallery-dl 不写入媒体主体文件；aria2 是唯一媒体传输 backend。`DownloadRouter` 的 gallery-dl→aria2 / aria2 fallback、同一 Job 混用 backend、partial file 复用和 `GalleryDlThenAria2` 语义已在 U8 删除，`xarchive-download` 只保留 plan/driver/refresh/client/supervisor。Windows 上的真实 aria2 transfer、file lock 和 restart/recovery 行为仍由 Windows Validation Queue 覆盖。

## ADR-011：Sidecar protocol v2 与 typed extraction contract

**状态：已实现（U8 完成 v1 路径删除）**

Sidecar v2 使用 JSONL stdio，命令固定为 `hello`、`extract`、`cancel`、`shutdown`，不存在 v1/v2 双解析或 capability 不足时降级旧路径。事件集合为 `ready`、`extraction_started`、`extracted`、`cancelled`、`failed`、`log`；Rust、Python、Schema、fixtures、Supervisor 和 Desktop consumer 同批更新，v1 命令/事件类型与 Schema 已在 U8 删除。Supervisor 只接受 `protocol_version = 2` 的 stdout 事件，legacy line 记为 `ProtocolError` 并导致 handshake 失败。

Browser/Native Host protocol version 与 Sidecar protocol version 分离，使用独立常量 `BROWSER_PROTOCOL_VERSION` 和 `SIDECAR_PROTOCOL_VERSION`。Ready capabilities 至少表达 `extract_media`、`cancel_active_extraction` 和 `structured_media_plan`；缺失 capability 时归档明确失败，不回退旧 download path。

## ADR-012：取消、恢复和 transfer data 的 durable 边界

**状态：目标架构已接受；Job 基础部分已实现（`MIGRATION`）**

- 用户主动取消得到 `CANCELLED`，是终态且不参与 startup recovery；
- 应用退出、崩溃、系统关闭或 runner 意外中断得到 `INTERRUPTED`，可由新 attempt 恢复；
- `FAILED` 与 `AUTH_REQUIRED` 不自动恢复；
- late result 不得覆盖 `CANCELLED`；cleanup warning 不得把 `CANCELLED` 改写为 `FAILED`；
- 第一版不持久化 signed URL、request headers、aria2 GID、extraction generation、refresh count 或浏览器 Cookie；恢复必须重新 extraction 并创建新的 transfer plan。

若当前 Schema 无法表达必要 durable Job state，才新增正式 migration；不得改写既有 migration。

## ADR-013：Core Bootstrap、embedded catalog 与 Extension 分发

**状态：Linux scope 已实现；Bootstrap/release integration 仍为 `PLANNED`**

Core 初始发行物为单个 Desktop `.exe`，运行后管理 `config/`、`cache/`、`logs/`、`download/` 和 `components/`。U9 已实现 ComponentManager 的固定 catalog schema、版本/平台/架构、artifact、SHA-256、大小上限、布局、probe、license 和 protocol compatibility 校验，以及本地 atomic activation/rollback。真实 catalog 条目和下载地址要等 U11 release assets 定稿；不使用动态 `latest` 或未经签名的远程 manifest。

Offline Bundle 预置相同 catalog 中的组件，不形成第二条业务路径。Extension 只通过版本化 Release ZIP 解压到固定目录并由用户开启浏览器开发者模式加载；不进入 Chrome Web Store、Microsoft Edge Add-ons 或自动浏览器安装流程。

Signed Remote Component Catalog 是后续 TODO，必须具备 Ed25519 签名、公钥内置、防降级、撤销、key rotation、host allowlist、replay/tamper tests 和离线 embedded fallback 后才能评估实现。

## ADR-014：Extension WebSocket 本地通道与安全迁移

**状态：已接受，进入实施**

### 背景

当前 Extension 通过 Native Messaging Host 访问 Desktop。该路径在 Windows 上需要 Native Host manifest、Registry 注册、Named Pipe 和浏览器安装流程；Extension 本身只使用经典内容脚本与 MV3 background service worker。现有 `BrowserRequest`、`BrowserResponse` 和 `BrowserTransportAdapter` 已经定义了稳定的业务消息契约，迁移不应复制归档逻辑或改变 `archive_request` / `query_status` 语义。

Loopback WebSocket 可以减少 Native Host 安装链路，但监听地址本身不是身份认证，MV3 service worker 也不能依赖永久连接来保持存活。WebSocket 只能作为受控的本地传输边界，不能绕过 Desktop 的 executor、SQLite、恢复和错误校验。

### 决策

1. Desktop WebSocket listener 只绑定 `127.0.0.1`，端口由固定默认值加受控环境变量覆盖；Extension 通过本机发现信息获得端口。第一版不把随机端口硬编码进扩展包。
2. 连接建立后必须完成一次性认证。认证凭据由 Desktop 生成并通过本机受控配对流程交付 Extension；凭据只保存于浏览器 Extension storage，不进入 Browser protocol、Job spec、日志或 Native Host manifest。Origin、端口和“已连接”状态均不单独构成可信凭据。
3. WebSocket 复用现有 Browser protocol：每个文本消息必须是一个符合 schema 的 `BrowserRequest`，响应必须保持对应 `request_id`。认证/握手消息使用独立的 transport envelope，不扩展业务协议版本。
4. Desktop 优先采用成熟的 `tungstenite` WebSocket 实现；浏览器侧使用平台内建 `WebSocket`。本仓库不自行实现 RFC 6455 帧、握手或关闭协议。引入依赖前固定版本并检查许可证与 transitive dependencies。
5. WebSocket 连接断开时，Extension 拒绝所有未完成请求；后台按有上限的退避策略重连，并限制并发请求和 request timeout。service worker 重启后必须重新读取 storage、重新发现并重新认证；长连接不作为 worker 保活机制。
6. 迁移期间保留 Native Messaging 作为显式回退通道。WebSocket 连接失败、认证失效或 Desktop 不可用时，回退必须可诊断且不重复提交同一请求；只有在 Windows Edge/Chrome 精确 revision 验收通过后，才可另行决策是否移除 Native Messaging 资产。
7. WebSocket listener 纳入 `RuntimeState` 生命周期和 `replace_executor()` 代际切换。旧连接不能继续引用已关闭的 executor；Desktop 退出、重启或 executor 替换时，旧 listener/连接必须停止并重新绑定。
8. Extension 新增紧凑 popup 与 options 页面。GUI 展示真实通道、连接/配对状态、当前页面可用性和恢复操作；截图中的编辑器字段、Obsidian 按钮等产品无关能力不迁移。所有状态必须区分文件就绪、连接中、已连接、断开、未配对、认证失败、Desktop 未启动和请求失败。

### 后果

- 迁移新增了端口发现、凭据配对、凭据轮换和认证失败恢复的开发与验证工作。
- Native Messaging 在过渡期仍是可用回退，发布包不会立即删除其资产。
- Edge/Chrome 实机、Windows 打包、权限和 service worker 重启验证是发布门槛；Linux fixture 或 listener 单测不能替代这些证据。
- WebSocket 只改变 Extension 到 Desktop 的传输适配层，不改变归档业务协议、executor、Sidecar 或媒体链路。
