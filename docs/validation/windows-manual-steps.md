# Windows 专属验证汇总与手工步骤

Owner: Windows Platform Owner 执行；Cross-platform Owner 维护本索引。
Status: `CURRENT` — 汇总当前需要 Windows 环境执行的验证项。基线 `dev` `0e3ac4d`，工作区干净，与 `origin/dev` 同步。

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

🔒 **安全优先项：** 队列记录设置页截图包含完整 Extension 配对 token。对外共享前必须遮挡；若已外发，应立即轮换该 token 并记录。此项优先于上述 UI 验收。

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
| G6 / M-CAND-03 | Telegram 发送 | `NOT RUN` | 无专用账号；且发布范围待 Owner 决定 |
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

**前置：** 专用账号，且 Owner 已决定 Telegram 是否写入发布范围。**该决定未作出前本项保持 `NOT RUN`。**

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

1. **安全项**：遮挡/轮换配对 token（若已外发）。
2. **A 组 5 项** — 当前批次交付物，直接决定下一个 pre-release 的可信度。
3. **B 组 WQ-P0-01** — 建立工具链 baseline，其余依赖它。
4. **B 组 WQ-P1-02** — 已确认 FAIL，先修复。
5. **C、D、E 组** — 可并行。
6. **F 组 M-CAND-01..04** — 需专用账号与稳定桌面会话，可最后集中执行。

## I. 需要 Owner 决定的事项

| # | 事项 | 影响 |
|---|---|---|
| 1 | 下一个 pre-release 的目标版本号 | 决定 tag 与 workflow 输入 |
| 2 | 是否压缩发布范围授权 | 决定 G4–G7 未关闭时能否发布 |
| 3 | 是否 fast-forward `main` | `main` 当前落后 `dev`，需在发布前决定 |
| 4 | Telegram 是否写入发布范围（G6） | 决定 M-CAND-03 是否执行 |
| 5 | gallery-dl / aria2 / PyInstaller 的许可证与固定版本 | 再分发义务未关闭，见 [`../references/external-sources.md`](../references/external-sources.md) |