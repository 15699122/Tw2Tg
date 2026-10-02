# Windows 专属验证汇总与手工步骤

Owner: Windows Platform Owner 执行；Cross-platform Owner 维护本索引。
Status: `CURRENT` — 汇总当前需要 Windows 环境执行的验证项。基线 `dev` `2c7632e`，工作区干净，与 `origin/dev` 同步。Telegram 条目见 §K（共享层已就绪，Batch B 未接线，当前整体跳过并附手工步骤）。

本文件只汇总**当前可执行**的 Windows 项，并给出 BLOCKED 项的手工步骤。历史执行记录在 [`windows-validation-history.md`](windows-validation-history.md)，完整队列在 [`windows-queue.md`](windows-queue.md)。

## 阅读规则

- **跳过不等于通过。** 本文件列出任何一项都不构成验收。
- 每项结果必须带 `source_sha`、`build_origin`、`artifact_sha256`、平台、环境、工具版本、方法、证据与后续动作（字段定义见 [`validation-policy.md`](validation-policy.md)）。
- 能力不可用时先按 validation-policy 的能力检测流程找等价路径；确实无法产出证据才记 `BLOCKED`。
- **不得为了让检查通过而修改生产代码、断言或验收标准。**

## A. 当前批次：日志样式 / 渠道日志策略 / 应用图标

Handoff revision `279d726`（实现提交）。计划见 [`../development/desktop-logs-release-icon-fix-plan.md`](../development/desktop-logs-release-icon-fix-plan.md)。**必须验证 `279d726` 本身，不是其后的文档提交。**

| ID | 优先级 | 目标 | 预期结果 |
|---|---|---|---|
| WQ-LOGS-020-02 | P0 | pre-Release 构建默认 debug 日志 | 启动日志头 `level=debug`；桌面/任务/Sidecar/gallery-dl/aria2/Telegram/代理各模块均有调试输出 |
| WQ-LOGS-020-03 | P0 | Release 构建默认 info 日志 | 日志头 `level=info`；调试内容被过滤，进度/警告/错误完整 |
| WQ-LOGS-020-04 | P0 | 用户显式级别优先与重启保持 | 手动选 `error`/`silent` 立即生效、重启保持、不被渠道默认覆盖；旧配置不被改写为 debug |
| WQ-LOGS-020-01 | P1 | 日志面板间距与复选框主题色 | 间距清晰；选中态为项目绿非蓝；100%/125%/150% 不重叠 |
| WQ-LOGS-020-05 | P1 | 应用图标 | EXE 文件图标/属性页/标题栏/任务栏为绿色归档盒；16/32/48/256 可辨识 |

前置：包含本批修复的 Windows 构建；日志目录可写。分别构建 pre-release 与 release 各一份，记录 build origin 与 exe SHA-256；启动后读取日志首部的渠道、渠道默认、有效级别与是否存在用户覆盖。

⚠️ WQ-LOGS-020-05 必须在**干净目录**首次启动验证，避免图标缓存干扰。

## B. 核心平台项

| ID | 优先级 | 目标 | 当前状态 |
|---|---|---|---|
| WQ-P0-01 | P0 | Windows workspace 与 Tauri baseline：Rust fmt/check/test/clippy、Node check/test/build、Sidecar tests、Tauri build/start/cleanup | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-02 | P0 | 真实 X/Edge Cookie 归档：无媒体/单图/多图/视频/Quote/回复/重复任务 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-03 | P0 | 文件 SQLite 应用级恢复：`0001→0002→0003`、异常退出恢复、staging 清理 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-04 | P0 | Native Host/Named Pipe 端到端：ACL、request_id 路由、多连接、重连、权限拒绝 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-01 | P1 | DownloadRouter 与真实 aria2 集成、403 回退、transfer lifecycle | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-02 | P1 | Native Host 浏览器安装（Registry、升级、卸载、管理员/非管理员） | **`WINDOWS_FAIL`** |

**WQ-P1-02 是当前唯一已确认的 Windows 产品失败项**，需先修复再推进后续验收。

## C. UI 与归档目录

| ID | 优先级 | 目标 | 备注 |
|---|---|---|---|
| WQ-SET-020-04 | P0 | 更改归档目录：选择器、校验、持久化与重启恢复 | 隔离测试用户或 VM |
| WQ-SET-020-05 | P0 | 边界：中文/空格路径、不可写目录、目标为已存在文件 | 不可写目录与已存在文件须被拒绝且原配置不变 |
| WQ-SET-020-06 | P0 | 切换目录后实际归档去向，及对运行中任务/浏览器 transport 的影响 | 已有文件、数据库与日志位置应不变 |
| WQ-SET-020-01~03 | P1 | 布局：Core Bootstrap 分隔线与间距、`Desktop 观察` 标签间距、三个 Extension 按钮间距/等高/窄窗口换行 | 100%/125%/150% 缩放 |

🔒 **配对 token：已由 Owner 于 2026-10-01 轮换，本项关闭。** 此前包含完整 token 的截图一律作废，不得再作为证据引用；后续共享必须重新截图并遮挡 token。轮换由 Owner 直接确认，Linux 侧未独立验证，不构成对轮换结果的验收。

**该项原先优先于 UI 验收，现已解除**，C 组 UI 项可按常规顺序执行。

⚠️ 残余风险：任何仍持有旧 token 的浏览器 profile 或已分发的截图仍具风险，建议在隔离 profile 中重新配对并确认旧 token 失效。

## D. Sidecar 协作式取消

| ID | 优先级 | 目标 |
|---|---|---|
| WQ-SIDECAR-CANCEL-01 | P0 | 下载期间 cancel 回收 gallery-dl 及其子进程树（`taskkill /T /F` 时序、句柄回收、无孤儿进程） |
| WQ-SIDECAR-CANCEL-02 | P0 | shutdown 与正常关闭区分：Job 应持久化为 `INTERRUPTED` 而非 `CANCELLED`，重启恢复可诊断且不重复提交 |
| WQ-SIDECAR-CANCEL-03 | P1 | timeout / cancel / 自然失败错误码边界；stdout 仍为合法 JSONL；stderr 不泄漏秘密 |

前置：可控长运行 gallery-dl fixture（**不访问真实 X 账号**），可观察 PID 的工具（Process Explorer 或 PowerShell）。
## E. 代理 Batch A 项（Batch B 未实现）

`WQ-PROXY-020-05` 至 `12` 不依赖 Batch B，可执行：`Direct` 在环境变量存在时仍直连、`Manual` 覆盖 Sidecar/aria2/Telegram、凭据不外泄（日志/SQLite/命令行/前端事件）、本地 aria2 RPC 与 Extension transport 不被代理、模式重启保持、旧配置迁移、Settings 布局、Windows 交叉编译目标编译。

`WQ-PROXY-020-01` 至 `04`（registry/PAC/WPAD/bypass）为 `PLANNED`，Batch B 交付前**不进入实机执行**，本轮不重复列出。

## F. 已发布版本遗留验收（G4–G7 / M-CAND-01..04）

针对已发布 `v0.2.0`（tag → `7910033`，Actions run `36705896154`）的冻结候选 `fe3feee7`。**不得继承上一版本 PASS。**

| 门禁 | 内容 | 状态 | 阻塞原因 |
|---|---|---|---|
| G4 / M-CAND-01 | 启动、Sidecar、生命周期与重启 | `WINDOWS_BLOCKED` | `COMPUTER_USE_UNAVAILABLE` |
| G5 / M-CAND-02 | 扩展安装、配对、真实归档完成终态 | `WINDOWS_BLOCKED` | 需稳定应用与专用账号 |
| G6 / M-CAND-03 | Telegram 发送 | **范围外**（`NOT_RUN` 的原因由阻塞改为范围决定） | 2026-10-01 Owner 决定暂不进行 Telegram 开发 |
| G7 / M-CAND-04 | Named Pipe ACL、Unicode/长路径、junction 越界 | `NOT RUN` | 缺可复现 harness |

⚠️ **G4–G7 未关闭，因此当前不存在发布许可。**「没有阻塞 Linux 开发的项」不等于「可以发布」。

## G. 手工验证步骤（BLOCKED / NOT RUN 项）

以下步骤供 GUI 自动化不可用时使用。**每一步都必须记录实际命令、退出码、日志与截图路径**；手工执行不替代自动化项的身份，只能改变方法字段。

### 通用准备

1. 记录 Windows 版本、架构、缩放设置、Node/Rust/Python/Tauri 版本。
2. 记录被测 revision、build origin 与 exe SHA-256；确认与本文件 A–F 节一致。
3. 使用隔离测试用户或 VM；准备独立浏览器 profile 与独立日志目录。
4. 记录 Computer Use 是否可用。不可用则所有 GUI 项记 `BLOCKED` + `COMPUTER_USE_UNAVAILABLE`，并保留本节步骤。
5. 确认测试 fixture **不访问真实 X 账号**；真实账号仅在明确授权的 G5/M-CAND-02 中使用。

### M-CAND-01（对应 G4）— 启动与生命周期

1. 双击待发布 `.exe`，不附加任何调试参数。
2. 30 秒内记录：进程是否存活、窗口是否出现、标题栏、图标外观。
3. 采集首部日志：channel、channel default、effective level、user override 是否存在。
4. 关闭应用，确认无残留进程、无锁定文件。
5. 再次启动，执行一次归档后强制结束进程，重启确认 Job 状态可诊断且不重复提交。

**通过判据：** 窗口出现、图标为绿色归档盒、日志头级别符合渠道预期、无残留进程、恢复不重复提交。
**证据：** 截图（含缩放比例）、日志文件路径、进程列表。

### M-CAND-02（对应 G5）— 扩展配对与真实归档

1. 在隔离 profile 中以开发者模式加载解压后的 Extension 目录。
2. 打开 Extension popup，确认版本来自 `chrome.runtime.getManifest().version`，连接状态**单独记录**（popup 打开不等于配对成功）。
3. 完成 token 配对（token 不得写入共享截图）。
4. 触发一次真实归档，等待终态。
5. 核对 SQLite 记录、媒体文件、staging 已清理。

**通过判据：** 配对成功且终态为完成；文件与数据库一致；重复触发不产生重复归档。
**证据：** 配对状态记录、归档后数据库查询、文件列表。**共享截图前必须遮挡 token。**

### M-CAND-03（对应 G6）— Telegram 发送

1. 配置专用 Bot token 与 chat id（**不得使用个人账号**）。
2. 触发一次含媒体的归档完成。
3. 在目标 chat 确认消息与媒体到达。
4. 中断一次发送后重试，确认不重复投递。

**前置：** 专用账号。Owner 已于 2026-10-01 决定暂不进行 Telegram 开发，本项**不在 `v0.2.1-pre1` 执行范围**；恢复开发后可直接执行本节步骤。

### M-CAND-04（对应 G7）— Named Pipe 与路径边界

1. 构造 Unicode、长路径（含空格、中文）归档目录并执行归档。
2. 创建指向归档目录的 junction，尝试越界写入，确认被拒绝。
3. 使用 Named Pipe 提交 `archive_request` 与 `query_status`，确认 `request_id` 正确回填。
4. 以非授权身份尝试连接，确认 ACL 拒绝。
5. 断开并重连，确认恢复行为。

**通过判据：** Unicode/长路径正常；junction 越界被拒绝；ACL 拒绝明确；`request_id` 不丢失。
**证据：** 每步的命令、退出码、错误文本、数据库结果。

### WQ-SIDECAR-CANCEL（自动化不可用时）

1. 启动一个只写入临时 staging 的长运行 gallery-dl fixture，并让其再启动一个子进程。
2. 发送 `hello`、`download`；下载进行中发送 `cancel`。
3. 记录 worker、gallery-dl 及孙进程 PID，检查进程树是否全部退出、staging 是否停止写入。
4. 重复一次下载期间关闭应用，确认 Job 为 `INTERRUPTED` 而非 `CANCELLED`。
5. 分别触发短 timeout、自然非零退出、缺失 executable，确认分别得到 `DOWNLOAD_TIMEOUT`、`EXTRACT_OR_DOWNLOAD_FAILED`、`SIDECAR_DEPENDENCY_MISSING`，且 stdout 为合法 JSONL、stderr 无秘密。

**禁止：** 手工 `Stop-Process` 之后把该项记为 PASS。人工终止不证明进程树回收正确。

## H. 执行顺序建议

配对 token 已轮换，原先的安全前置项关闭。推荐顺序：

| # | 内容 | 依赖 |
|---|---|---|
| 1 | **A 组 5 项**（日志/图标，revision `279d726`） | 无 — 当前批次交付物，直接决定下一个 pre-release 的可信度 |
| 2 | **B 组 WQ-P0-01**（Windows 工具链 baseline） | 无 — 其余 B 组项依赖它 |
| 3 | **B 组 WQ-P1-02**（Native Host 安装） | 需先修复现有 `WINDOWS_FAIL` |
| 4 | **B 组其余 4 项 P0** | 依赖 2 |
| 5 | **C、D、E 组** | 依赖 2 |
| 6 | **F 组 M-CAND-01..04** | 需专用账号与稳定桌面会话，可最后集中执行 |

配对 token 轮换后建议顺带确认：在隔离 profile 中重新配对，并验证旧 token 已被拒绝（对应 M-CAND-02 第 3 步）。

## H2. 需要 Windows 环境执行的步骤总览

以下为可直接执行的清单，每项完成后在队列中登记结果。

### A 组 — 日志样式 / 渠道日志策略 / 应用图标（revision `279d726`）

- [ ] A1 构建 pre-release（记录 build origin 与 exe SHA-256），启动后读取日志首部：channel、channel default、effective level → 期望 `level=debug`，各模块均有调试输出（WQ-LOGS-020-02）
- [ ] A2 构建 release，重复上述读取 → 期望 `level=info`，调试被过滤，进度/警告/错误完整（WQ-LOGS-020-03）
- [ ] A3 设置页手动选 `error` / `silent` → 立即生效；重启后保持；不被渠道默认覆盖；写入旧配置后升级不被改写为 debug（WQ-LOGS-020-04）
- [ ] A4 100%/125%/150% 缩放查看日志面板间距与 `自动跟随` 复选框 → 间距清晰、选中态为项目绿、无重叠（WQ-LOGS-020-01）
- [ ] A5 在**干净目录**首次启动，检查 exe 文件图标、属性页、标题栏、任务栏 → 绿色归档盒，16/32/48/256 可辨识（WQ-LOGS-020-05）

### B 组 — 核心平台

- [ ] B1 Rust fmt/check/test/clippy、Node check/test/build、Sidecar tests、Tauri build/start/cleanup 全部通过（WQ-P0-01）
- [ ] B2 真实 Edge Profile 归档：无媒体、单图、多图、视频、Quote/Reply、重复任务、异常退出（WQ-P0-02）
- [ ] B3 文件 SQLite：`0001→0002→0003` 迁移、关闭/重启、遗留 staging 清理、异常退出恢复（WQ-P0-03）
- [ ] B4 Named Pipe：请求/响应、`request_id` 路由、多连接、重连、关闭、非法消息、权限拒绝（WQ-P0-04）
- [ ] B5 修复并验证 Native Host 安装：安装、升级、卸载、管理员/非管理员、扩展加载、Service Worker 重连（WQ-P1-02，**当前 FAIL**）
- [ ] B6 真实 aria2：gallery-dl 默认、错误回退、403 后重新提取、transfer lifecycle、Job 状态同步（WQ-P1-01）

### C 组 — UI 与归档目录

- [ ] C1 更改归档目录：选择器打开、提示新路径、重启后仍为该目录、取消无副作用（WQ-SET-020-04）
- [ ] C2 边界：中文/空格路径被接受；不可写目录与已存在文件被拒绝且原配置不变（WQ-SET-020-05）
- [ ] C3 切换目录后执行归档 → 写入新目录；已有文件、数据库、日志位置不变；运行中任务与浏览器连接行为明确（WQ-SET-020-06）
- [ ] C4 三项布局：Core Bootstrap 分隔线与间距、`Desktop 观察` 标签间距、三个 Extension 按钮间距/等高/窄窗口换行（WQ-SET-020-01~03）

### D 组 — Sidecar 协作式取消

- [ ] D1 下载期间 cancel：进程树全部退出、staging 停止写入、无残留锁定或孤儿进程（WQ-SIDECAR-CANCEL-01）
- [ ] D2 下载期间 shutdown：Job 持久化为 `INTERRUPTED` 而非 `CANCELLED`；重启恢复可诊断且不重复提交（WQ-SIDECAR-CANCEL-02）
- [ ] D3 错误码边界：分别得到 `DOWNLOAD_TIMEOUT` / `CANCELLED` / `EXTRACT_OR_DOWNLOAD_FAILED`；stdout 为合法 JSONL；stderr 不泄密（WQ-SIDECAR-CANCEL-03）

### E 组 — 代理 Batch A

- [ ] E1 环境变量存在时选 `直连` → 实际未走代理（含混合大小写变量名）（WQ-PROXY-020-05）
- [ ] E2 `手动` 模式覆盖 Sidecar、aria2、Telegram 三条路径（WQ-PROXY-020-06）
- [ ] E3 含凭据代理的完整归档 → 日志、SQLite、命令行、前端事件均无明文凭据（WQ-PROXY-020-07）
- [ ] E4 三种模式下本地 aria2 RPC 与 Extension transport 均直连（WQ-PROXY-020-08）
- [ ] E5 模式重启后保持；旧 `config.yaml` 迁移为 `手动` 而非静默变为跟随系统（WQ-PROXY-020-09/10）
- [ ] E6 Settings 代理区块在 100%/125%/150% 与窄窗口下无重叠（WQ-PROXY-020-11）
- [ ] E7 `rustup target add x86_64-pc-windows-msvc` 后 `cargo build --workspace` 通过（WQ-PROXY-020-12）

### F 组 — 遗留门禁

- [ ] F1 M-CAND-01 启动与生命周期（见 G 节步骤）
- [ ] F2 M-CAND-02 扩展配对与真实归档（见 G 节步骤）
- [x] F3 M-CAND-03 Telegram 发送 — **范围外**（2026-10-01 Owner 决定暂不开发 Telegram，不写入发布范围）。非通过、非失败。步骤保留在 G 节，恢复开发后可直接执行。
- [ ] F4 M-CAND-04 Named Pipe 与路径边界（见 G 节步骤）

> E 组 01~04（registry/PAC/WPAD/bypass）为 `PLANNED`，Batch B 交付前不执行。

## K. Telegram 发送（`WQ-TG-*`，Batch A 共享层已就绪，当前全部跳过）

共享层已于 2026-10-01 落地并单元测试通过：`xarchive-telegram` 的 endpoint 契约与 transport
（阻塞 JSON 控制路径 + 局部异步流式上传）、outbox 状态机/原子领取/崩溃恢复、失败分类与重试
决策、bot 隔离 `file_id` 缓存、进度投影与文案；`xarchive-storage` 的 migration
`0007_telegram_outbox.sql`；Desktop 的 `TelegramConfig` 配置契约。

**当前不可执行的原因（`BLOCKED`，blocker = `BATCH_B_NOT_IMPLEMENTED`）**：Desktop 发送服务、设置界面、
Windows Credential Manager 适配器和 Local Bot API Server 部署尚未实现，本节步骤的执行入口不存在。
这些项**不是 PASS，也不是失败**；在 Batch B 接线完成后按下表执行并按标准判定。Linux 侧的契约与
存储测试已 PASS，但它们不能替代本节任何一项证据。

前置（Batch B 完成后）：包含该批次提交的 Windows 构建；专用测试 bot 与受控目标 chat/topic；
本地模式另需按 [`../development/telegram-local-bot-api-plan.md`](../development/telegram-local-bot-api-plan.md) TG-07 部署固定版本的 Local Bot API Server（仅回环端口）；Unigram 作为接收端（可选，但 UNI 组必需）。

| ID | 手工步骤 | 期望结果 | 证据 |
|---|---|---|---|
| `WQ-TG-001` | 在设置页写入/替换/删除 bot token；重启应用；查看配置 YAML、SQLite、日志目录与诊断导出 | Token 只经 Credential Manager 存取；配置/DB/日志/诊断/前端事件均无明文；删除后 `bot_token_present=false`；无明文回退 | 凭据管理器条目、配置/DB/日志截图（token 遮挡）、诊断导出文件 |
| `WQ-TG-002` | 云模式发送一条测试消息；再切本地模式（`http://127.0.0.1:8081`）分别验证 `Desktop → Local API` 与 `Local API → Telegram`；尝试非回环 HTTP、凭据 URL、重定向 | 云模式拒绝 http；本地模式仅回环+端口被接受；本地连接强制直连（代理不生效）；3xx 不会被跟随 | 各次请求的错误提示、服务端访问日志、代理配置截图 |
| `WQ-TG-003` | 发送超长文本（>4096 字符）、中英文/emoji 混排、含链接文本；分别设置 caption 与 metadata | 文本按序拆分、顺序不乱；caption 与 metadata 不混用；链接预览开关生效 | 目标 chat 实际截图（发送层） |
| `WQ-TG-004` | 发送 1、2、10、11 项媒体的相册；11 项时确认尾组单项走单条方法；照片/视频/文件回退各一次 | 相册项数合法（2–10）；顺序与逐项 message id 映射正确；11 项不产生一项“相册”；回退路径有明确结果 | Bot API 结果记录 + 目标 chat 截图 |
| `WQ-TG-005` | 分别在请求前、请求中、响应丢失后、DB 写入前强杀进程并重启；随后让 `UNKNOWN` 项到期 | 原子领取无双发；`UNKNOWN` 永不被自动重发；未开始的崩溃回到重试队列；已开始的记为 `UNKNOWN` 并可人工复核后重发 | SQLite outbox 行截图/查询输出、重启日志 |
| `WQ-TG-006` | 同一媒体重复发送；替换 token 后再发送；手工把缓存中的 `file_id` 改为无效值后发送 | 缓存命中复用；换 token 后不复用旧 bot 的 id；仅在明确的 invalid-file-id 错误时回退原始文件；权限/网络失败不清缓存 | `telegram_file_cache` 查询输出、Bot API 错误文本 |
| `WQ-TG-007` | 发送 >50 MB、接近服务器上限、以及超限文件；发送期间观察 Desktop 内存占用 | 上限内成功；超限给出明确错误且本地归档不受损；内存不随文件大小线性增长 | 服务器版本与上限依据、内存采样、错误提示 |
| `WQ-TG-008` | 100%/125%/150% 缩放与窄窗口下打开设置页与任务详情；键盘 Tab 遍历；触发取消/重试/`UNKNOWN` 复核 | 设置项顺序符合规范；键盘可达；归档状态与 Telegram 状态分开展示；仅在确认后显示“Telegram: send confirmed” | 截图序列、键盘遍历记录 |
| `WQ-TG-009` | 配置带凭据的代理并执行一次发送；制造重定向响应；检查日志/诊断/SQLite | 凭据不外泄；重定向不被跟随；日志中无 token 或带凭据 URL；本地回环不经过代理 | 日志片段、SQLite 导出、代理配置 |
| `WQ-TG-UNI-01` | 记录 Unigram 版本/渠道、Windows build、WebView2、GPU/驱动/HDR、下载设置与磁盘余量 | 形成可复现环境记录（缺失项写 `NOT_RUN` 或环境说明） | 环境记录表 |
| `WQ-TG-UNI-02` | 在 Unigram 查看文本/caption/长文本/链接 | 显示与发送层一致 | 截图 |
| `WQ-TG-UNI-03` | 查看 1/2/10/11 项相册与普通相册、评论线程相册 | 顺序与分组符合预期；1 项不显示为相册 | 截图 |
| `WQ-TG-UNI-04` | 视频持续播放、暂停、跳转、音轨、旋转 | 播放稳定、音画同步 | 录屏 |
| `WQ-TG-UNI-05` | 适用硬件上验证 HDR / 视频增强场景 | 不适用硬件记 `NOT_RUN` 并写明环境，不外推 | 硬件记录或 `NOT_RUN` 说明 |
| `WQ-TG-UNI-06` | 在 Unigram 下载文件、单条与批量下载、比对原始文件 SHA-256 | 文件名可区分；原始文件哈希一致 | 截图 + SHA-256 比对输出 |
| `WQ-TG-UNI-07` | 大文件手动下载与关闭自动下载场景 | 接收端行为明确、可预期 | 截图 |
| `WQ-TG-UNI-08` | 打开 `https://t.me/c/<id>/<msg>` 深链（含 Unigram 未运行时） | 按系统关联打开；未运行时记录真实行为，不承诺强制拉起 | 录屏 |

**判定标准**：每一项都必须记录 `source_sha`、`build_origin`、环境与工具版本、步骤、实际结果、
证据位置和 PASS/FAIL/FAIL 的失败细节；任一项未执行记 `NOT_RUN`，环境缺失记 `BLOCKED` 并写明
缺失能力。发送层通过不等于接收端验收通过，反之亦然。

## I. 需要 Owner 决定的事项

| # | 事项 | 影响 |
|---|---|---|
| 1 | ~~下一个 pre-release 的目标版本号~~ | **已决定：`v0.2.1-pre1`**，随后准备 `v0.2.1` 正式版 |
| 2 | ~~是否压缩发布范围授权~~ | **待定**：仍取决于 G4–G7 是否关闭 |
| 3 | ~~Telegram 是否写入发布范围（G6）~~ | **已决定：暂不进行 Telegram 相关功能开发，不写入发布范围** |
| 4 | 是否 fast-forward `main` | `main` 当前落后 `dev`，需在发布前决定 |
| 5 | gallery-dl / aria2 / PyInstaller 的许可证与固定版本 | 再分发义务未关闭，见 [`../references/external-sources.md`](../references/external-sources.md) |

## J. 版本目标（2026-10-01 Owner 决定）

- **首个目标：`v0.2.1-pre1`**（编号符合 [`release-policy.md`](../release/release-policy.md) 第 7 节的 `vMAJOR.MINOR.PATCH-preN`；已退役的 `pre.N` 不得复用）。
- **随后目标：`v0.2.1` 正式版。**
- 版本号需同步的位置：`package.json`、`desktop/package.json`、`extension/manifest.json`、`desktop/src-tauri/tauri.conf.json`（`Cargo.toml` 通过 `version.workspace = true` 取值）。修改属实现变更，需与本次发布一同验证。

### 范围决定：Telegram 暂不开发

Owner 于 2026-10-01 决定**暂不进行 Telegram 相关功能开发**。由此产生的记录规则：

1. `M-CAND-03` / `G6`（Telegram 真实发送）**不进入 `v0.2.1-pre1` 的执行范围**，状态从 `NOT RUN` 记为「范围外」。这是范围决定，**不是**通过，也不是失败。
2. 发布说明**不得宣传** Telegram 能力已实现。`v0.2.0` 已按此执行，`v0.2.1-pre1` 沿用同一口径。
3. 已实现的 Telegram 代码**保留不删**（契约、幂等发送、持久化）。本决定是「暂停开发与对外声明」，不是「移除功能」。
4. 根 `README.md` 原先把 Telegram 描述为可用能力，与本决定不一致，已改为如实表述。