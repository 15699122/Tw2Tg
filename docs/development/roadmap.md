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
5. Desktop 唯一业务入口为 `submit_executor_job`、`query_executor_job`、`cancel_executor_job`、`shutdown_executor`；同步 `archive_tweet` command 已在 U8 删除。
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

当前进度（2026-09-19）：Rust `sidecar_v2` 模型、Python `protocol_v2`/`worker_v2`/`extraction`、Schema `sidecar-v2-command/event`、valid/invalid/v1-rejected fixtures、Supervisor spawn/handshake 和 Desktop v2 事件消费已完成并通过 Linux 验证（Rust protocol 15/15、Supervisor 5/5、Sidecar pytest 33/33）。PyInstaller spec 使用专用 `entrypoint_v2.py`，v1 fallback 保留为 `entrypoint_v1.py`；当前 Windows 已通过 v2 packaged handshake/worker artifact 范围验证。真实 extraction/download 仍待 U7 runtime 验收。

### U4：gallery-dl extraction-only

gallery-dl 只负责 metadata、media discovery、stable identity、安全 filename 和 allowlisted headers，不写媒体主体文件。退役 `DownloadedFile`、file/progress/complete 旧事件、staging 扫描和 signed URL 持久化路径；需要凭据转发但不在 allowlist 内时明确失败，不回退 gallery-dl 下载。

当前进度（2026-09-19）：v2 extraction-only 适配层已完成并通过 Linux 验证（Sidecar pytest 33/33）；命令强制 `--skip-download` 且防御性拒绝 `--directory`/`--filename`/`--download`；`sanitize_filename`/`stable_media_id` 落地；`ExtractionResult` 彻底移除 `raw` 字段、不携带 `DownloadedFile` 或 staging 扫描事实；未在 v2 事件词汇表中的 `metadata` 事件已移除；header 仅 Referer/Accept 且过 secret 检查。Windows full pytest 的 POSIX fake executable fixture 已改为跨平台 Python fixture，Linux 33/33（U8 删除 v1 测试后为 21/21）通过，当前 revision 的 Windows Sidecar full pytest 也已通过。剩余旧 `DownloadedFile`/file event 退役和 signed URL 持久化删除已在 U8 完成。

### U5：aria2-only transfer driver

`xarchive-download` 提供与 Tauri/SQLite/Sidecar 解耦的 transfer driver，当前唯一实现为 aria2；覆盖 RPC supervisor、随机 secret、loopback、multi-GID polling、progress、timeout、retry、cancel、shutdown、error classification 和文件验证。

当前进度（2026-09-19）：driver/plan 层和 U7 production ownership/接线已完成 Linux 验证：typed extraction result 可构建 backend-neutral `MediaTransferPlan`；aria2-only driver 已覆盖 multi-GID、plan-order completion、progress monotonicity、timeout、cancel/shutdown、error/removed classification、submission failure 和 partial/`.aria2` cleanup。`xarchive-download` 单元 23/23、driver 集成 7/7、workspace test/clippy/fmt 通过。Windows 真实 aria2c、process cleanup、signed URL、staging/commit 和 restart/recovery 仍待集中验证；旧 `DownloadRouter` fallback 已在 U8 删除。

### U6：URL 过期与重新 extraction

每个 media/attempt 最多 refresh 一次；403/expired URL 触发完整 extraction、旧 GID 移除和新 plan，文件系统/权限/磁盘错误不 refresh；集合无法稳定匹配时返回 `EXTRACTION_RESULT_CHANGED`。

当前进度（2026-09-19）：refresh contract 已完成 Linux 验证并由 U7 production path 接入 Desktop executor。`xarchive-download` 将 401/403/expired/signature/access-denied 归类为 `TRANSFER_EXPIRED_URL`；U7 对此只执行一次完整 v2 extraction refresh，按 stable media identity + filename 集合匹配后使用新 plan/new GID 重试，集合变化返回 `EXTRACTION_RESULT_CHANGED`。Windows signed URL expiry、file lock、process cleanup、restart recovery 仍待集中验证。

### U7：Desktop production integration

完成状态（2026-09-19，Linux scope）：`desktop/src-tauri/src/production.rs` 已将 v2 extraction result 合并为 durable metadata，构建 `MediaTransferPlan`，执行 aria2 transfer，处理一次性 expired URL refresh，检查 staging path/file/reparse/identity，并交给 `ArchiveService` commit。`ExecutorConfig`/`ArchiveExecutionContext` 传递 portable aria2 path；`archive.rs` 只保留资源适配和 v2 调用，不再包含 v1 fallback。Windows baseline、worker artifact、packaged v2 handshake 和 Sidecar pytest 已通过；aria2 transfer、真实 extraction/download、expired URL refresh、Windows filesystem/commit、cancel/shutdown/recovery 和 late-result fencing 仍未执行，继续为 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`。

### U8：删除旧路径

完成状态（2026-09-19，Linux scope）：`archive_tweet` 注册与实现、Sidecar v1 runtime（Rust v1 command/event 类型、Supervisor v1 handshake、Python v1 worker）、`download` command、旧 `file/progress/complete` 事件、`DownloadRouter`/`GalleryDlThenAria2` fallback、v1 Schema/fixtures、v1 PyInstaller entrypoint 和相关死代码已删除。`xarchive-protocol` 只保留 `sidecar_v2` 与新的 `media.rs`（commit 事实 `DownloadFile`）；Supervisor stdout reader 只接受 protocol v2 event。历史文档保留历史事实，当前状态和运行流已反映 v2-only extraction/aria2 链路。Linux 验证：workspace Rust 184/184、Sidecar pytest 21/21、Node/Extension 测试与 build 通过；Windows 专属验证项进入 queue（见 `docs/validation/windows-queue.md`）。

### U9–U13：组件管理、Bootstrap、发布、浏览器集成和 Offline Bundle

实现固定 embedded catalog、SHA-256/size/probe/license/layout 校验、安全解压、atomic activation、rollback、Setup Wizard、版本化 Release assets、Native Host/Extension developer-mode 流程和 Core/Offline Bundle 布局 parity。安装器不得使用动态 `latest` 或未经验证的远程 manifest。

### U9：ComponentManager（Linux scope 完成）

完成状态（2026-09-20，Linux scope）：新增 `desktop/src-tauri/src/components.rs`，提供版本化 embedded catalog schema、组件 id/version/platform/architecture/artifact/hash/size/layout/license/probe/protocol 字段校验、固定 catalog 版本检查、目录 artifact 的 deterministic SHA-256/size 校验、safe relative path 与 symlink/special-file 拒绝、`.part` staging、atomic activation、`current` marker、previous-version rollback 和诊断错误分类。当前 embedded catalog 为空是有意的安全边界：U11 尚未生成真实 release asset/hash，U9 不伪造可激活组件，也不执行动态网络下载。Linux 已覆盖 catalog/path/hash、拒绝 traversal/hash mismatch、安装/激活/rollback 测试；ZIP 解压、Windows executable probe、真实 release asset、签名/权限和 GUI Setup Wizard 进入 Windows/U10/U11 queue。

### U14–U16：验证、Windows handoff 和合并

U14 完成所有 Linux applicable verification 后，整理按 Build/Runtime/Filesystem/Integration/Packaging/Regression 分类的 Windows handoff。U15 集中执行 Windows queue；U16 只在 feature branch clean、Linux PASS、Windows 队列完整、旧路径清理完成、文档和 catalog 一致后创建 PR 到 `main`。

## 5. 当前迁移边界（2026-09-19）

U8 之后旧路径迁移已结束：Sidecar protocol v1 runtime、gallery-dl 媒体下载、`DownloadRouter` fallback、`archive_tweet` 同步入口和 v1 Schema/entrypoint 都不再存在，当前媒体链路是 gallery-dl extraction-only → aria2-only transfer。文档仍需区分：

- `CURRENT`：代码和测试已证明；
- `PLANNED`：目标架构但尚未实现；
- `MIGRATION`：新旧路径并存；
- `WINDOWS_VERIFICATION_PENDING`：Linux 无法替代的 Windows 证据。

## 6. Signed Remote Component Catalog TODO

未来可评估签名远程 catalog，但不得在第一版替代 embedded catalog。完成标准至少包括 versioned schema、Ed25519 signature、编译进 Desktop 的公钥、key rotation、revocation、min/max Desktop compatibility、platform/arch、size/SHA、host allowlist、rollback protection、cached valid catalog、offline embedded fallback、stable/dev channels、tamper/replay tests 和 security review。

## 7. 合并前条件

- working tree clean，feature branch 已推送；
- 所有 Linux applicable verification PASS；
- 无 gallery-dl media download runtime、旧 aria2 fallback Router、Sidecar v1 runtime 或 `archive_tweet`（U8 已满足）；
- aria2 是唯一媒体 transfer backend；
- cancel/recovery race、URL/header redaction、component hash/install/rollback 测试通过；
- Core 缺组件时仍可启动设置页；
- Release assets、embedded catalog 和外部 manifest 一致；
- Windows Validation Queue 完整，未验证项目没有被写成 PASS；
- 最终 diff 不含 secrets、cache、artifact 或机器绝对路径。