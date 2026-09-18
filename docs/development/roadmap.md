# XArchive 总体开发路线图

> 基准日期：2026-09-18。本文只记录未来方向、依赖和完成标准；当前实现事实以 [`status.md`](status.md) 为准，Windows 验证事实以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准。

## 1. 目标终态

目标运行链路为：

```text
Browser Extension
  → Native Messaging Host
  → Desktop transport
  → Archive Job executor
  → Sidecar protocol v2
  → gallery-dl extraction-only
  → typed ExtractionResult
  → Rust MediaTransferPlan
  → aria2-only media transfer
  → Rust staging verification
  → ArchiveService final commit
```

目标分发模型为 Core Bootstrap：初始发布物是单个 Desktop `.exe`，首次运行后下载或手动指定 Worker、gallery-dl、aria2、Native Host 和 Extension。Offline Bundle 预置相同组件清单，但不是第二条业务路径。

目标组件来源：Worker、Native Host、Extension 使用 XArchive GitHub Release assets；gallery-dl 使用官方 Codeberg stable release；aria2 使用官方 GitHub stable release。以上均为 `PLANNED`，不能在当前代码尚未完成前写成已发布能力。

## 2. 已冻结的目标架构决策

以下决策从后续 Unit 起作为目标约束，不代表当前运行时已经完成迁移：

1. 唯一媒体链路是 gallery-dl extraction-only → aria2 transfer；不保留 gallery-dl 媒体下载、双 backend fallback、partial file 复用或 `DownloadRouter` 旧 fallback 语义。
2. Sidecar 直接升级到 protocol v2，命令为 `hello`、`extract`、`cancel`、`shutdown`；不支持 v1/v2 双解析或 capability 不足时降级旧路径。
3. 用户主动取消为 `CANCELLED`；应用退出、崩溃、系统关闭或 runner 意外中断为 `INTERRUPTED`。`CANCELLED` 不参与 startup recovery，`INTERRUPTED` 可恢复，late result 不得覆盖终态。
4. Python cooperative cancellation 与 OS process-tree enforcement 必须同时存在；Unix 使用 session/process group，Windows 使用 Job Object 边界，aria2 同样必须停止 active GID 并清理不完整文件。
5. Desktop 唯一业务入口为 `submit_executor_job`、`query_executor_job`、`cancel_executor_job`、`shutdown_executor`；`archive_tweet` 在 U8 前仍是当前代码中的迁移残留。
6. 第一版不持久化 signed URL、request headers、aria2 GID、extraction generation、refresh count 或浏览器 Cookie；恢复通过新的 extraction 和新的 transfer plan 完成。
7. Component Manifest 第一版编译进对应 Desktop `.exe`，不使用动态 `latest` 或未经签名的远程 manifest。
8. Extension 通过 Release ZIP 和浏览器开发者模式加载，不进入 Chrome Web Store、Microsoft Edge Add-ons 或自动安装流程。

签名远程 Component Catalog 属于后续 TODO，不属于本轮实现。

## 3. 实施单元与依赖顺序

```text
U0 Git 基线收口
  → U1 Job 取消语义与架构文档
  → U2 Sidecar cooperative/process-tree cancellation
  → U3 Sidecar protocol v2
  → U4 gallery-dl extraction-only
  → U5 aria2-only transfer driver
  → U6 extraction refresh
  → U7 Desktop production integration
  → U8 删除旧入口和旧下载代码
  → U9 ComponentManager
  → U10 Core Bootstrap Setup Wizard
  → U11 Release assets/pipeline
  → U12 Native Host/Extension installation flow
  → U13 Offline Bundle
  → U14 Linux 全量验证
  → U15 Windows 集中验证
  → U16 PR / merge main
```

每个 Unit 必须独立完成设计、代码、测试、受影响文档和适用验证，并保持可独立回滚的提交边界。协议升级不得与无关 GUI 修改混合；状态机不得与 packaging 修改混合；历史验证记录不得伪造新的 PASS。

## 4. Unit 完成标准

### U0：Git 基线收口

审查当前 working tree、完整 diff、secret/path/artifact、文档证据和 Linux 验证；确认后再提交并推送当前分支。新架构不能覆盖未审查的既有修改。

### U1：Job 取消语义

- active → `CANCELLED` 与 active → `INTERRUPTED` 明确区分；
- `INTERRUPTED → CANCELLED`、`FAILED → CANCELLED`、`AUTH_REQUIRED → CANCELLED` 按状态机测试；
- `COMPLETE → CANCELLED` 禁止；
- recovery 只处理 `INTERRUPTED`；
- late result、cleanup warning 和 attempt fencing 不得改变已确定终态；
- 本单元不新增 transfer checkpoint migration。

### U2：Sidecar cooperative/process-tree cancellation

Python worker 必须在 extraction 期间读取 cancel/shutdown，单 worker 同时只运行一个 extraction，终态事件只允许一个获胜；Rust supervisor 必须提供 Unix process group 与 Windows Job Object 的平台边界。Linux fake child、孙进程、EOF、JSONL 串行化、timeout 和 cleanup 必须有回归；Windows 行为保持 `WINDOWS_VERIFICATION_PENDING`。

### U3：Sidecar protocol v2

新增 typed extraction models、capability handshake、v1 rejection、unknown field rejection、request/job identity 绑定和脱敏错误边界，并同步 Rust、Python、Schema、fixtures、Supervisor 和 Desktop consumer。

当前进度（2026-09-18）：Rust `sidecar_v2` 模型、Python `protocol_v2`/`worker_v2`/`extraction`、Schema `sidecar-v2-command/event`、valid/invalid/v1-rejected fixtures 和跨语言 contract tests 已完成并通过 Linux 验证（Rust protocol 15/15）。剩余：Supervisor spawn v2 worker、Desktop v2 事件消费，随 U7 运行时接线完成；Windows packaged worker 验证保持集中队列。

### U4：gallery-dl extraction-only

gallery-dl 只负责 metadata、media discovery、stable identity、安全 filename 和 allowlisted headers，不写媒体主体文件。退役 `DownloadedFile`、file/progress/complete 旧事件、staging 扫描和 signed URL 持久化路径；需要凭据转发但不在 allowlist 内时明确失败，不回退 gallery-dl 下载。

当前进度（2026-09-18）：v2 extraction-only 适配层已完成并通过 Linux 验证（Sidecar pytest 33/33）：命令强制 `--skip-download` 且防御性拒绝 `--directory`/`--filename`/`--download`；`sanitize_filename`/`stable_media_id` 落地；`ExtractionResult` 彻底移除 `raw` 字段、不携带 `DownloadedFile` 或 staging 扫描事实；未在 v2 事件词汇表中的 `metadata` 事件已移除（raw metadata 不外发）；header 仅 Referer/Accept 且过 secret 检查。剩余：`DownloadedFile`/staging 扫描/file 事件的退役随 U7/U8 在 v1 链路删除时完成；signed URL 持久化删除同样随 U8；Supervisor/Desktop 接线随 U7。

### U5：aria2-only transfer driver

`xarchive-download` 提供与 Tauri/SQLite/Sidecar 解耦的 transfer driver，当前唯一实现为 aria2；覆盖 RPC supervisor、随机 secret、loopback、multi-GID polling、progress、timeout、retry、cancel、shutdown、error classification 和文件验证。

当前进度（2026-09-18）：driver/plan 层已完成 Linux 验证：typed extraction result 可构建 backend-neutral `MediaTransferPlan`；aria2-only driver 已覆盖 multi-GID、plan-order completion、progress monotonicity、timeout、cancel/shutdown、error/removed classification、submission failure 和 partial/`.aria2` cleanup。`xarchive-download` 单元 20/20、driver 集成 7/7、workspace test/clippy/fmt 通过。剩余范围：aria2 Supervisor 的 production ownership、Desktop executor 接线、U6 403/expired URL refresh、staging verification 和最终 ArchiveService commit；旧 `DownloadRouter` fallback 在 U8 前仍保留。

### U6：URL 过期与重新 extraction

每个 media/attempt 最多 refresh 一次；403/expired URL 触发完整 extraction、旧 GID 移除和新 plan，文件系统/权限/磁盘错误不 refresh；集合无法稳定匹配时返回 `EXTRACTION_RESULT_CHANGED`。

当前进度（2026-09-18）：refresh contract 已完成 Linux 验证。`xarchive-download` 将 401/403/expired/signature/access-denied 归类为 `TRANSFER_EXPIRED_URL`，`RefreshCoordinator` 只允许一次完整 extraction refresh；refresh 后按 stable media identity + filename 集合匹配，集合变化返回 `EXTRACTION_RESULT_CHANGED`，普通失败、cancel、shutdown、timeout 和本地文件系统错误不 refresh。`xarchive-download` unit 23/23、driver integration 7/7、workspace test/clippy/fmt、Sidecar 33/33、Desktop 33/33、Extension 7/7 通过。剩余：接入 Desktop/Supervisor production chain、真实旧 GID→新 GID lifecycle、U7 staging/commit integration，以及 Windows signed URL/file-lock/process 验证。

### U7：Desktop production integration

将 extraction result 合并到 durable metadata，构建 `MediaTransferPlan`，执行 aria2 transfer，验证 staging 后交给 ArchiveService commit。orchestration 按 extraction、transfer、commit 等职责拆分，不能把所有逻辑继续堆入 `archive.rs`。

### U8：删除旧路径

删除 `archive_tweet` 注册和实现、Sidecar v1 runtime、`download` command/event、旧 file/progress/complete 事件、`DownloadRouter` fallback、`GalleryDlThenAria2` 和相关死代码。历史文档可保留历史事实，但当前状态和运行流必须反映新链路。

### U9–U13：组件管理、Bootstrap、发布、浏览器集成和 Offline Bundle

实现固定 embedded catalog、SHA-256/size/probe/license/layout 校验、安全解压、atomic activation、rollback、Setup Wizard、版本化 Release assets、Native Host/Extension developer-mode 流程和 Core/Offline Bundle 布局 parity。安装器不得使用动态 `latest` 或未经验证的远程 manifest。

### U14–U16：验证、Windows handoff 和合并

U14 完成所有 Linux applicable verification 后，整理按 Build/Runtime/Filesystem/Integration/Packaging/Regression 分类的 Windows handoff。U15 集中执行 Windows queue；U16 只在 feature branch clean、Linux PASS、Windows 队列完整、旧路径清理完成、文档和 catalog 一致后创建 PR 到 `main`。

## 5. 当前迁移边界（2026-09-18）

当前代码仍属于迁移中状态：Sidecar protocol v1、gallery-dl 媒体下载、`DownloadRouter` fallback 和 `archive_tweet` 仍存在。当前已完成的是 executor/cancellation 和部分入口调度基础，不是目标终态。后续文档必须同时标注：

- `CURRENT`：代码和测试已证明；
- `PLANNED`：目标架构但尚未实现；
- `MIGRATION`：新旧路径并存；
- `WINDOWS_VERIFICATION_PENDING`：Linux 无法替代的 Windows 证据。

## 6. Signed Remote Component Catalog TODO

未来可评估签名远程 catalog，但不得在第一版替代 embedded catalog。完成标准至少包括 versioned schema、Ed25519 signature、编译进 Desktop 的公钥、key rotation、revocation、min/max Desktop compatibility、platform/arch、size/SHA、host allowlist、rollback protection、cached valid catalog、offline embedded fallback、stable/dev channels、tamper/replay tests 和 security review。

## 7. 合并前条件

- working tree clean，feature branch 已推送；
- 所有 Linux applicable verification PASS；
- 无 gallery-dl media download runtime、旧 aria2 fallback Router、Sidecar v1 runtime 或 `archive_tweet`；
- aria2 是唯一媒体 transfer backend；
- cancel/recovery race、URL/header redaction、component hash/install/rollback 测试通过；
- Core 缺组件时仍可启动设置页；
- Release assets、embedded catalog 和外部 manifest 一致；
- Windows Validation Queue 完整，未验证项目没有被写成 PASS；
- 最终 diff 不含 secrets、cache、artifact 或机器绝对路径。