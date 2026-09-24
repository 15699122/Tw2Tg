# XArchive 总体开发路线图

> 基准日期：2026-09-20。本文只记录未来方向、依赖和完成标准；当前实现事实以 [`status.md`](status.md) 为准，Windows 验证事实以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准。

## 1. 目标终态

目标运行链路为：

```text
Browser Extension
  → Native Messaging Host
  → Desktop transport
  → Archive Job executo
  → Sidecar protocol v2
  → gallery-dl extraction-only
  → typed ExtractionResult
  → Rust MediaTransferPlan
  → aria2-only media transfe
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
  → U5 aria2-only transfer drive
  → U6 extraction refresh
  → U7 Desktop production integration
  → U8 删除旧入口和旧下载代码
  → U9 ComponentManage
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

### U5：aria2-only transfer drive

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

### U10：Core Bootstrap Setup Wizard（Linux scope 完成）

完成状态（2026-09-20，Linux scope）：Core Bootstrap 已接入 Desktop：新增 `get_component_bootstrap_status` command，启动/设置页可读取 embedded catalog、active component version、缺失组件和 catalog 状态；空 catalog 明确显示为“等待 release assets”，不会伪造组件已安装。现有首次下载目录 Setup Wizard 继续负责 portable/system Downloads 选择，并在 setup 完成后重建 executor runtime；Bootstrap 不执行动态网络下载、不绕过 ComponentManager 校验。Linux 覆盖 Desktop bootstrap/component/UI wiring tests；Windows Core `.exe` 启动、WebView2 设置页、文件权限、真实 component asset 安装和 U11 release catalog 仍进入 Windows queue。

### U11：Release assets/pipeline（Linux scope 完成）

完成状态（2026-09-20，Linux scope）：新增 `desktop/scripts/release-assets.mjs`，定义版本化 Windows x64 资产命名（`XArchive-<tag>-windows-x64.exe/.7z`）、manifest schema（tag、platform、catalog_version、assets、licenses）、SHA-256/size/license 约束和校验函数；禁止动态 `latest`、无 hash、无 size、无 license 的 manifest。新增 `desktop/test/release-assets.test.mjs` 覆盖 tag/asset/kind/hash/size/license 的接受与拒绝用例。Linux 只完成命名与 manifest 契约，不生成、签名、上传真实资产；真实构建、哈希、签名、许可证扫描、发布上传和 Core/Offline Bundle parity 仍进入 Windows queue。

### U12：Native Host/Extension installation flow（Linux scope 完成）

完成状态（2026-09-20，Linux scope）：新增 `desktop/scripts/native-host-package.mjs`，固定 Native Host name `com.tw2tg.xarchive`，校验 MV3 Extension manifest、Native Messaging 权限、X/Twitter host permissions、32 位小写 Chrome Extension ID、host manifest 和 versioned Windows x64 installation manifest。Desktop Extension 状态将“文件就绪”与“浏览器未加载/Native Host 未注册/不可用”分开，不再把文件存在误报为浏览器连接。Linux 不执行 Registry、ACL、Named Pipe、Edge/Chrome 加载或真实 reconnect；Extension ID 仍需由发布密钥/浏览器发布策略提供，不能凭空写入仓库。对应纯逻辑测试已加入 `desktop/test/native-host-package.test.mjs`，Windows 项目进入 queue 并附手工步骤。

### U12 follow-up：pre-release UI、Extension status 和 Native Host registration

当前 Plan（2026-09-20，`IN_PROGRESS`）针对已发现的 pre-release 问题补充以下完成标准。Linux implementation scope 已完成；Windows-only scope 仍未完成：

1. **Desktop UI（LINUX_VERIFIED）：**主页首次使用说明和运行环境区块的垂直节奏符合现有 design token；侧栏设置/服务状态间距不再由叠加 separator/footer margin 产生异常留白；设置页主要 section 使用分隔线而不是重复卡片边框；globe SVG path 已修复。常见 DPI 和真实 GUI 仍需 Windows 验证。
2. **Path interaction（LINUX_VERIFIED）：**所有用户需要复制或识别的目录/可执行文件路径使用统一可聚焦控件，支持点击、截断显示、完整 `title` 和复制反馈；Windows clipboard、中文/空格/长路径和键盘行为仍需验证。
3. **Extension status（LINUX_VERIFIED）：**前端 checking 只表示正在执行状态查询；文件缺失、Host 未注册、浏览器未连接、已连接和错误有独立映射；Extension section 提供局部刷新，不会把 `not_loaded` 永久显示为“检测中…”。真实 connected 状态仍需 Windows transport 证据。
4. **Native Host packaging（LINUX_VERIFIED；Windows implementation complete / live verification pending）：**Full portable/release 包含 host executable、manifest 和 Extension ID/`allowed_origins` 校验；Windows 已实现当前用户 HKCU 注册、修复、取消注册与移动后路径更新。真实用户浏览器注册/修复/取消注册仍在 Windows validation queue。
5. **Connection/retry（LINUX_VERIFIED；Windows implementation complete / live verification pending）：**NativeBridge 已处理 `runtime.lastError`、断开时 pending request、失效 port、同步连接失败和下一次请求重新连接；Desktop status 已读取 Windows transport session、Native Host 注册及启动错误。Edge/Chrome 实际加载、断开/重连与页面状态仍需 Windows 验证。
6. **验证边界：**Linux 完成纯逻辑、契约、Node/Rust 静态检查和 UI wiring；Windows 集中验证 Registry、Edge/Chrome、Named Pipe/transport、WebView2/DPI、portable package 和真实 Extension archive request。上述 Windows 项目在实际证据前保持 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`。

### U13：Offline Bundle（Linux scope 完成）

完成状态（2026-09-20，Linux scope）：新增 `desktop/scripts/offline-bundle-package.mjs`，定义 Offline Bundle 的固定 Windows x64 组件集合（Desktop、worker、Native Host、Extension、gallery-dl、aria2）、相对路径和路径逃逸拒绝、SHA-256/size/license/required_files 校验、运行时目录排除，以及 `release_manifest`、`embedded_catalog`、`catalog_version` parity 约束。新增 `desktop/test/offline-bundle-package.test.mjs` 覆盖组件缺失/重复、路径逃逸、运行时目录预创建和 parity mismatch。Linux 不生成或签名真实 Windows artifact，不填充未经验证的 embedded catalog 条目；真实 bundle 组装、许可证扫描、签名、解压和 Windows startup 进入 queue。

### U14：Linux full verification（完成）

完成状态（2026-09-20）：在包含 U12/U13 未提交 working tree changes 的 Linux source 上完成全量适用验证。Rust workspace fmt/check/test/clippy、Node workspace check/test/build、Desktop Linux WDIO native smoke、Extension、Sidecar compile/pytest 和 `git diff --check` 均通过。U14 不替代 Windows WebView2、Registry、Named Pipe、真实 Windows filesystem、签名、浏览器和真实账号验证；这些项目已统一收口到 Windows Validation Queue，并附 BLOCKED 手工步骤。

### U14–U16：验证、Windows handoff 和合并

U14 完成所有 Linux applicable verification 后，整理按 Build/Runtime/Filesystem/Integration/Packaging/Regression 分类的 Windows handoff。2026-09-20 的 current-source Windows 验证已确认 baseline、U8 v2-only worker/WDIO、Core/Full assembly/startup 和当前 Node/Rust/Sidecar scope；但 U7 真实 runtime、U9/U10 filesystem/activation、U12 browser/Registry、U13 final bundle parity 仍未完成，均保持 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`。此外，`v0.2.0-pre.3` 的 tag-level GitHub Actions release workflow 仍为 `WINDOWS_FAIL`，因为 run `35492155317` 在 Sidecar v2 handshake tests 失败且没有生成资产。

当前 Plan 重新收敛为（基于 2026-09-20 current-dirty Windows 结果）：

1. **Linux reconciliation：**更新当前 Git/source 状态、Plan、Windows Queue 和验证记录；运行受影响的 Linux Node/Rust/Extension/package 回归。当前没有证据要求修改业务 UI 或 Native Host 代码。
2. **Windows native-session follow-up：**`WQ-P1-16/WQ-P1-17` 保持 `WINDOWS_FAIL`，调查 Windows WebView2/tauri-driver/WDIO render path、E2E asset loading 和 machine-local session 条件；Linux 不为该环境失败降低 UI 断言或修改生产 capability。
3. **Windows integration follow-up：**Registry/ACL、真实 Extension ID、Edge/Chrome developer-mode、Named Pipe、Native Host reconnect、真实 `archive_request`、release asset hash/license/parity 和 Offline Bundle 继续保持 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`，不得用 synthetic-ID local package PASS 替代。
4. **U16 合并：**只有在 feature branch clean、Linux applicable verification PASS、`WINDOWS_FAIL` 有明确后续处理、所有未完成 Windows 项有准确状态、最终 Release asset/manifest/catalog parity 有证据且没有未处理 tag-level failure 时，才创建 PR 到 `main`。

2026-09-20 增量 Windows revalidation 未改变上述 Plan：当前业务影响区没有新增代码变化，Linux 适用门禁继续通过；`WQ-P1-16/WQ-P1-17` 的 native WDIO failure 继续作为 Windows follow-up，不在 Linux 端猜测性修改；Registry、真实浏览器连接、Named Pipe、release workflow 和 final asset parity 继续留在 Windows handoff。

不得使用 current-source local build、Core/Full startup smoke 或 WDIO scope PASS 覆盖失败的外部 Release workflow，也不得把未执行的 Windows 项目提前改为 `WINDOWS_PASS`。

### P0 follow-up：Windows pre.7 Desktop 白屏修复（2026-09-21，LINUX_DIAGNOSTIC_FIX_IMPLEMENTED / WINDOWS_REVALIDATION_PENDING）

`v0.2.0-pre.7` 用户反馈、截图和后续 Windows 重验共同表明：窗口、前端和 Dashboard 已能启动；先前的 `extensionBusy is not defined` 已修复。当前 Windows 失败来自新增 startup marker 的状态竞态：异步 IPC 事件把最终 `react_mount_completed` marker 覆盖为 `initial_ipc_started` 或 `initial_ipc_settled`。

执行顺序固定为：

```text
P0-1 启动阶段与前端异常可观测性
  → P0-2 HTML/React 双层启动 fallback
  → P0-3 production dist/embedded asset contract
  → P0-4 Windows native E2E 分层诊断
  → P0-5 release UI readiness gate
  → P0-6 依据证据修复具体根因并回归
```

Linux implementation 已完成：

1. `desktop/src/bootstrap.js` 记录 `document_loaded`、`entry_module_evaluated`、`react_mount_started`、`react_mount_completed`、`initial_ipc_started` 和 `initial_ipc_settled`，并捕获全局 `error`/`unhandledrejection`。
2. `desktop/index.html` 提供不依赖 React 的启动占位、资源加载错误和 15 秒超时提示；React 外层增加 root ErrorBoundary，保留页面级 ErrorBoundary。
3. `log_frontend_event` 以 allowlisted、限长字段写入应用日志，不让启动诊断依赖 WDIO console forwarding。
4. `desktop/test/startup-contract.test.mjs` 校验 production dist 的 JS/CSS 引用、文件存在性、启动 fallback、React marker 和普通 bundle 的 WDIO guest 隔离。
5. `dashboard.e2e.mjs` 已从单一 `h1` 等待扩展为 URL、readyState、`#root`、startup marker、fallback 文本和失败截图/证据采集。
6. `windows-release.yml` 已在 `Collect executable` 后、任何资产归档/上传前加入最终 `.exe` UI readiness gate，并在失败时上传诊断 artifact。
7. **本轮 Linux fix：**`desktop/src/main.jsx` 的 `Sidebar` 显式接收 `extensionBusy`；`desktop/test/ui-wiring.test.mjs` 增加 parent/child prop contract 断言，避免该 runtime mismatch 仅在 Windows 截图中暴露。
8. **本轮 Linux diagnostic fix：**initial IPC 阶段改为只调用 `emitFrontendEvent()`，不再调用 `setStartupState()`；`desktop/test/startup-contract.test.mjs` 和 `ui-wiring.test.mjs` 固定最终 readiness marker 不可被 IPC 覆盖。

仍需 Windows 重验：

- 修复后最终普通 `.exe` 和 Full bundle 的真实 WebView2 resource/document/React mount 验证；
- Windows frontend diagnostic log、WebView2/msedgedriver stderr、artifact SHA-256 和进程清理；
- 若 readiness gate 仍失败，基于新证据判断是否还有独立的 WebView2/资源/环境问题；
- 只有最终 artifact 通过后，才可将本问题从发布阻断状态关闭。

### Windows session blocker follow-up（2026-09-21，LINUX_DIAGNOSTICS_IMPLEMENTED / WINDOWS_VERIFICATION_PENDING）

`v0.2.0-pre.8` GitHub Actions run `35593193897` 的 build、Rust tests、Native Host、worker 和外部依赖均通过，但最终 readiness gate 在 WebDriver session 创建阶段失败：`DevToolsActivePort file doesn't exist`。该 run 未进入 Dashboard spec，因此当前需要优先改善 Windows session 诊断，而不是修改 Dashboard 断言或生产 capability。

当前 Linux Plan：

1. 增加 `windows-ui-readiness-preflight.ps1`，记录 Windows/WebView2/Node/Rust/driver 版本、最终 executable 直接启动、进程和端口状态；
2. 让 readiness workflow 在成功和失败路径都收集统一 diagnostics 目录，并复制应用日志、WDIO logs、进程和端口快照；
3. 在 Linux contract tests 中固定 preflight、diagnostics、gate-before-upload 的 workflow wiring；
4. 在 Windows 重验中区分 `APP_START_FAILED`、`TAURI_DRIVER_START_FAILED`、`SESSION_CREATION_FAILED` 和 `DOM_READINESS_FAILED`；
5. 如果 hosted `windows-latest` 仍无法稳定创建 WebView2 session，下一轮将评估 self-hosted interactive Windows validation job；build 与 publish 必须继续使用同一 artifact hash。

本轮 Linux implementation verification：Desktop Node `70/70`、Vite build、WDIO syntax、Rust fmt/check/test `87/87`、`git diff --check` 均通过。Windows preflight 和 diagnostics bundle 尚未在 Windows runner 重验，保持 `WINDOWS_VERIFICATION_PENDING`。

本轮追加 R3/R7 verification：Desktop Node `71/71`、Vite `check/build`、全部 WDIO/Node syntax、workspace Rust check/test（core 13、desktop 87、protocol 16、sidecar supervisor 6、storage 25、telegram 12）、strict Clippy 和 `git diff --check` 均通过。`tauri-driver`/`msedgedriver` 固定策略已完成 Linux wiring，但版本可用性、Windows PATH、WebView2 匹配和真实 session 仍只能在 Windows 确认。

### 2026-09-21 ccaa649 Windows 回写后的 Linux re-reconciliation

`ccaa649` 在 Windows 的新事实：同步/PASS 范围（Node check/build、workspace tests、Rust fmt/check、显式 venv Python 的 workspace tests、Sidecar pytest、普通/WDIO-E2E build、preflight 进程存活、worker v2、本地 package boundary）为 `WINDOWS_PASS` 或有限范围 PASS；WQ-P1-16/WQ-P1-17 因已安装 `@wdio/tauri-service 1.4.0` 的 banner 正则只接受 `MSEdgeDriver x.y`，而实际 driver 报告 `Microsoft Edge WebDriver x.y`，导致 `Driver: unknown`，仍为 `WINDOWS_BLOCKED`；其余 U7/U9/U10/U11/U12/U13 项目按 fixture 缺失保持 `WINDOWS_BLOCKED` 或 `NOT RUN`。`extensionBusy` 与 startup marker 两个旧 Linux 修复未被新证据推翻，但仍未获得原生 session/DOM 验证。

本轮 Linux 只做测试基础设施诊断增强：`windows-ui-readiness-preflight.ps1` 新增 `msedgedriver_banner_accepted` 与 `msedgedriver_banner_reason`，同时匹配 `MSEdgeDriver` 与 `Microsoft Edge WebDriver` banner，仅记录诊断事实，不修改业务代码、UI 断言、生产 capability、service 依赖或自动下载策略。Linux verification：Desktop Node `72/72`、Vite check/build、WDIO/Node syntax、Rust fmt/check/workspace test、strict Clippy、`git diff --check` 通过。WQ-P1-16/WQ-P1-17 继续 `WINDOWS_BLOCKED`；preflight banner 通过仍不得记为原生 UI PASS。

### 2026-09-21 synchronized revalidation 后的 Linux closeout

新 Windows 事实确认：当前同步源码在 Windows 通过 Node `72/72`、Extension `21/21`、Rust fmt/check/workspace tests/strict Clippy、Sidecar pytest `21/21`、普通/WDIO-E2E build、Native Host release build、Full package assembly 与直接 executable preflight；WQ-P1-16/WQ-P1-17 仍因 `@wdio/tauri-service 1.4.0` 的 banner 正则与实际 `Microsoft Edge WebDriver` 输出不兼容而 `WINDOWS_BLOCKED`。

本轮 Linux 新增 `desktop/scripts/edge-driver-banner.mjs` 与 `desktop/test/edge-driver-banner.test.mjs`，把上述 service 行为固定为 Linux 可回归的契约：legacy banner 可被 service 接受，当前 Microsoft banner 可被 preflight 接受但仍被 service 拒绝。该模块仅为测试基础设施诊断，不改变业务代码、Dashboard 断言、生产 capability、依赖版本或自动下载策略。Linux verification：Desktop Node `76/76`、Vite check/build、WDIO/Node syntax、Rust fmt/check/workspace tests、strict Clippy、`git diff --check` 通过。WQ-P1-16/WQ-P1-17 继续 `WINDOWS_BLOCKED`，仍需真实 Windows session 证据才能关闭。

禁止项：不降低 `react_mount_completed`、Dashboard `h1` 或稳定区域断言；不把进程存活替代 UI readiness；不移动 `v0.2.0-pre.8` tag；不向 `pre.8` 上传后续不同 commit 的资产。

完成标准：普通 release、Full bundle、首次启动和重复启动均不出现无提示白屏；失败有可读 UI 和日志；Linux applicable verification PASS；Windows queue 中关联项目获得真实结果；不以延长 timeout、降低断言或扩大 production capability 代替修复。

---

### 2026-09-21 post-banner-fix Linux closeout

Windows 现场证实：接受当前 `Microsoft Edge WebDriver` banner 后，ordinary WDIO `1/1`、advanced WDIO `2/2` 通过，且普通与 WDIO-feature 两个 release 二进制均原生渲染 Dashboard（pre.7 白屏产品问题在当前源码上未复现）。

根因定位到 `@wdio/tauri-service@1.4.0` `findMsEdgeDriver` 的 discovery regex 仅匹配 `MSEdgeDriver x.y`；Windows 手工纠正该 parser 后即进入 session 并渲染 Dashboard。为在 Linux 交付可复现的依赖修复，新增：

- `desktop/scripts/patch-wdio-tauri-service.mjs`：幂等重写 service discovery regex，接受 `MSEdgeDriver` 与 `Microsoft Edge WebDriver`；对未来 service 版本保持宽容；不修改 business 代码、UI 断言、production capability、driver 版本或下载策略。
- Linux 回归契约：`desktop/test/patch-wdio-tauri-service.test.mjs` 与 `desktop/test/ui-wiring.test.mjs` 固定 wiring（root `postinstall` + desktop e2e `pre*` hooks）。
- Linux 诊断模型：`desktop/scripts/edge-driver-banner.mjs` + `desktop/test/edge-driver-banner.test.mjs`。
- `.github/workflows/windows-release.yml`：保持固定 `tauri-driver`/`msedgedriver`、关闭自动安装/下载，不变。

Linux verification：Desktop Node `78/78`、Vite check/build、JS/Node syntax、Rust fmt/check/workspace tests/strict Clippy、`git diff --check` 均通过；`node_modules` 中 service `dist/esm/index.js` L1689 与 `dist/cjs/index.js` L1693 均确认已接受 `Microsoft Edge WebDriver`。

禁止项：不降低 `react_mount_completed`/`Dashboard h1` 等断言；不把进程存活替代 UI readiness；不移动 `v0.2.0-pre.8` tag；不向 `pre.8` 上传后续不同 commit 的资产。

下一步 Windows revalidation（全部在 `WINDOWS_VERIFICATION_PENDING`）：

1. 同步当前 Linux working tree 到干净 Windows 工作副本；
2. `npm ci`（触发 root `postinstall` 自动 patch）后，**不手动改动 `node_modules`**；
3. 运行 `npm run test:e2e:windows --workspace desktop`（ordinary）→ 要求 `1/1`；
4. 运行 `npm run build:tauri:wdio --workspace desktop` + `npm run test:e2e:windows:advanced --workspace desktop`（advanced）→ 要求 `2/2`；
5. 要求 session URL/readyState/`#root`/`react_mount_completed`/Dashboard heading 均可见；
6. 要求退出后无 `xarchive-desktop`、`tauri-driver`、`msedgedriver` 进程及端口 `1420/4444/4445/9223` 遗留。

仅当上述 clean-invocation 全部通过同一 artifact hash 后，才可将 WQ-P1-16/WQ-P1-17 从 `WINDOWS_BLOCKED` 改为 `WINDOWS_PASS`，否则保持 `WINDOWS_BLOCKED` 不得提前标记为 PASS。

U17 是在 U12 Linux scope 完成后新增的 Extension 专项开发单元。它不把当前的 Native Host package contract、Unix transport 测试或 Extension Node 测试外推为 Windows 浏览器集成完成。U17 的目标是将当前“MV3 DOM adapter + NativeBridge 原型”推进到可诊断、可测试、可集中 Windows 验证的浏览器归档链路。

依赖顺序固定为：

```text
E0 文档/事实对账
  → E1 Browser protocol/schema 收口
  → E2 DOM identity 与 fixture 测试
  → E3 NativeBridge timeout/reconnect hardening
  → E4 页面状态同步与 query_status 批量消费
  → E5 Windows Named Pipe Desktop transport
  → E6 Native Host Registry install/repair/unregiste
  → E7 实时 Extension/Native Host/transport 状态
  → E8 Extension 版本、ZIP 与 release parity
  → E9 Linux contract/integration 与 Windows 集中验证
  → E10 popup、右键菜单及其他可选 Browser commands
```

#### E0：文档与事实对账

- 统一当前 branch、commit、working tree 和 U12 实际实现状态；
- 将 `archive_request`、`query_status` 标为当前 Browser command；其他未实现命令标为 `PLANNED`；
- 修正 README、architecture、protocol、risk register 中已删除的 Sidecar v1/旧下载 fallback 描述；
- 明确 Extension 文件存在、Native Host 已注册、浏览器已加载、transport 已连接是四种不同状态；
- 完成标准：当前文档不把计划、Linux 证据或 synthetic-ID package contract 写成 Windows production PASS。

#### E1：Browser protocol/schema 收口

- **当前进度（2026-09-20）：LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING。**已新增 `archive_status_batch` response；`query_status` 对每个请求 Tweet ID 返回状态，未找到 Job 返回 `NOT_ARCHIVED`；Rust Browser request/response/tweet models 已增加 `serde(deny_unknown_fields)`；Browser response schema、fixture、Native Host response validation、Desktop transport 和 Extension response recognition 已同步。真实 packaged/browser protocol probe 仍属于 Windows queue。
- 以 `browser-request.schema.json` 和 `browser-response.schema.json` 作为唯一 Browser schema source；
- 已移除未被 producer/consumer 引用的 `archive-request.schema.json`、`archive-status.schema.json` 重复定义；后续新增 Browser message 必须直接扩展 Browser request/response schemas 或通过 `$ref` 复用定义；
- Rust、Native Host、Desktop 和 Extension 对 unknown fields、边界长度、URL、Tweet ID 和错误结构执行同一规则；
- 将 `query_status` 改为真正的 batch response，覆盖已归档、处理中、失败和未归档 Tweet；
- 同步 fixtures、Rust tests、Extension tests 和协议文档；
- 完成标准：同一 fixture 在 Schema、Rust、Native Host、Desktop 和 JavaScript 中得到一致结果。

#### E2：DOM identity 与 fixture 测试

- **当前进度（2026-09-20）：LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING。**`content-core.js` 已新增 status-link 候选筛选、quote link 排除、主 permalink 优先、reply parent 自身 ID 防护和 mutation 影响范围处理；Extension tests 新增多 status link、reply/quote 分离、self-reply 防护和 mutation article 过滤回归。真实 X DOM、virtualized timeline、SPA 路由和浏览器渲染仍需 Windows Edge/Chrome 验证。
- 修复“第一个 `/status/` 链接即当前 Tweet”的脆弱识别；
- 防止 `reply_to` 指向自身；quote card 必须与主 Tweet identity 分离；
- 稳定处理 display name、username、reply、quote、详情页、媒体 Tweet、动态节点和 virtualized article；
- 增加脱敏、最小化 X DOM fixtures，不引入未确认使用的测试框架；
- 完成标准：主 Tweet ID、canonical URL、reply_to、quoted_tweet 在 fixture 和 fake DOM 测试中稳定，失败时安全降级为 null 而不是错误关系。

#### E3：NativeBridge hardening

- **当前进度（2026-09-20）：LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING。**`NativeBridge` 已增加默认 10 秒 timeout、pending timer cleanup、重复 `request_id` 拒绝、结构化 `code/request_id/retryable` 错误、postMessage failure cleanup 和 port generation fencing；旧 port 的 late message/disconnect 不会影响新 port。Extension background tests 当前 18/18 通过。真实 MV3 Service Worker 生命周期、Native Host crash/restart 和浏览器 `runtime.lastError` 仍需 Windows Edge/Chrome 验证。
- 为 pending request 增加 timeout、timer cleanup、重复 request ID 拒绝和结构化错误传播；
- 防止旧 port 的 late message/disconnect 影响重连后的新 port；
- 覆盖乱序响应、postMessage 失败、runtime.lastError、Host crash、超时和 pending 上限；
- 完成标准：不存在无限 pending、重复 ID 覆盖 waiter 或错误码丢失。

#### E4：页面状态同步

- **当前进度（2026-09-20）：LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING。**`content-core.js` 已提供状态映射、按钮状态机和每批最多 100 个 Tweet ID 的去重分批；`content.js` 已对初始可见 article 和 MutationObserver 增量 article 发起 `query_status`，消费 `archive_status_batch`，并对 `archive_request` 单条响应复用同一状态映射。按钮状态包括 `idle/checking/submitting/queued/running/complete/auth_required/failed/disconnected`；状态只存在于 DOM，不写入 Extension storage。Extension tests 当前 21/21 通过。真实页面注入、Desktop 状态更新和 Edge/Chrome Service Worker 行为仍需 Windows 验证。
- 真正接入 `query_status`，对可见 Tweet 分批、去重、增量查询；
- 建立 `idle/checking/submitting/queued/running/complete/failed/disconnected` 等 Extension UI 状态；
- 已完成或处理中 Tweet 不重复提交；断线和错误可操作且不伪造成功；
- 仅在确有用途时使用 `storage`，不得保存 Cookie、signed URL、本地路径或媒体数据；
- 完成标准：页面按钮能反映 Desktop job 状态，且页面动态更新不会造成重复请求或重复注入。

#### E5：Windows Named Pipe Desktop transport

- **当前进度（2026-09-23）：WINDOWS_IMPLEMENTED / WINDOWS_VERIFICATION_PENDING。**共享端点契约固定在 `xarchive-protocol`，Native Host Windows client 未设变量时回退到默认管道名；Windows Desktop 已加入 Named Pipe listener、当前 owner ACL、并发连接处理、既有 Native Messaging framing/BrowserRequest adapter 与 runtime wiring。x64 Windows-target library tests（含真实本机 Named Pipe loopback、多次连接）89/89 PASS；另有隔离 Full package 组装与 worker 协议探针 PASS。ACL 的跨用户拒绝、Desktop GUI 生命周期及 packaged Native Host 到 Desktop 的真实连接仍在 Manual Windows Validation Queue。
- 在 Desktop 增加 Windows Named Pipe server，与现有 Unix transport adapter 保持相同 BrowserRequest/BrowserResponse 契约；
- 明确固定 pipe name、当前用户 ACL、多连接、退出、错误和 reconnect 行为；
- Native Host Windows client 仅负责 Named Pipe client，不把平台逻辑混入协议 crate；
- 完成标准：Windows 上真实 `query_status` 与 `archive_request` 可由 Native Host 转发到 Desktop，request_id 正确匹配。

#### E6：Native Host Registry lifecycle

- **当前进度（2026-09-23）：WINDOWS_IMPLEMENTED / WINDOWS_VERIFICATION_PENDING。**已实现 Edge/Chrome 当前用户 HKCU 注册、repair、unregister；在写入前拒绝未知 manifest/Registry mapping，repair 更新 portable root 移动后的绝对 exe 路径，注册状态检查 manifest identity、allowed origins 和 exe 是否存在。Full package 的 Extension identity/Native Host manifest 静态核对 PASS；未在当前用户浏览器 hive 执行真实注册/修复/取消注册。
- 增加当前用户级 Chrome/Edge Native Messaging Host inspect/install/repair/unregister；
- portable root 移动后能诊断和修复绝对路径；默认不写 HKLM，不覆盖未知注册项；
- 将 Registry 副作用保持在平台适配层；
- 完成标准：真实用户环境下注册、修复、取消注册均可回滚且不残留失效路径。

#### E7：实时连接状态

- **当前进度（2026-09-23）：WINDOWS_IMPLEMENTED / WINDOWS_VERIFICATION_PENDING。**Windows status command/UI 已区分文件缺失、Native Host 文件缺失、未注册、未观察到连接、活动/近期连接、断开及 Named Pipe listener 错误；Vite production build PASS。真实 Edge/Chrome load、扩展 UI、连接变化和 archive/query 流尚未执行。
- 用明确的 transport/session facts 区分 `MISSING`、`FILES_READY`、`NATIVE_HOST_NOT_REGISTERED`、`BROWSER_NOT_LOADED`、`DISCONNECTED`、`CONNECTED`；
- 必要时增加轻量 Browser ping/pong handshake，但不得传输 Cookie、profile path 或本地敏感信息；
- Desktop UI 不得把文件存在误报为浏览器连接成功；
- 完成标准：状态来源可追溯到实际 Registry、transport、最近请求或错误证据。

#### E8：Extension packaging/release parity

- 统一 Extension manifest version、release tag、ZIP、Native Host manifest、installation manifest 和 `allowed_origins`；
- 增加 Extension ZIP 的 required files、排除 tests/cache/node_modules、hash/size/license 检查；
- 明确开发者模式下稳定 Extension ID 的来源，不使用无法复现的 synthetic ID 作为正式发布证据；
- 完成标准：workflow 不重复手写 host manifest，Full/Core/Extension ZIP 的边界和版本可自动比对。

#### E9：验证收口

- Linux：Extension unit、Browser schema contract、Native Host fake/Unix integration、Desktop transport adapter、package contract；
- Windows：按 Build/Runtime/Filesystem/Integration/Packaging/Regression 集中验证 Named Pipe、Registry、Edge/Chrome、Service Worker reload、真实 request、reconnect 和 package parity；
- 所有 Windows-only 项目保持 `WINDOWS_VERIFICATION_PENDING`，直到真实环境执行并记录证据；
- 完成标准：失败、阻塞和未执行项均有原因，不能用 Linux PASS 替代 Windows PASS。

#### E10：可选 Browser UX

popup、右键菜单、批量归档、retry/cancel/open-folder 等功能必须在 E1–E9 完成后单独评估。新增 Browser command 前必须同步协议、Desktop command、权限安全审查、fixtures、测试和 Windows queue，不得恢复旧文档中的未实现 command 列表。

### U18：Extension identity 与发布可靠性（2026-09-21 当前 Plan）

U17 的 E1–E4 已在 Linux 完成，E5–E9 仍未实现或未验收。U18 不改变 E1–E10 的目标，而是补齐三项在此之前缺失的前置：canonical Extension identity、GitHub Actions 的 tag/source parity、Windows Named Pipe/Registry 的可验收实现，并把它们排入可执行批次。

#### 当前 identity 事实（2026-09-21）

- 私钥 `xarchive-extension.pem` 保存在仓库外（`$HOME/xarchive-extension.pem`，`0600`），不进入 Git、ZIP、Full bundle、日志或 Actions 产物；
- manifest public key 固定开发/自托管 identity；由该公钥派生的 ID 为 `iaajefkoanbkleojofoadeakelihbjne`（满足 Chromium `[a-p]{32}`）；
- 上架 Chrome Web Store / Edge Add-ons 前必须重新确认商店最终 ID；若两者不同，需要改为多 `allowed_origins` 模型并同步 `native-host-package.mjs`、workflow、installation manifest 和测试；
- synthetic ID 只能用于本地 package-contract 测试，不能作为发布证据。

#### 批次计划

| Batch | 内容 | 依赖 | 完成标准 |
|---|---|---|---|
| B0 | 私钥安全与 identity 基线 | 无 | PEM 在仓库外且权限 0600；SOPS/离线备份可恢复；Git 防误提交与 secret scan 规则就位 |
| B1 | manifest `key`、ID 派生与一致性校验 | B0 | build/package 由公钥派生 ID 并与 `XARCHIVE_EXTENSION_ID` 比较；漂移即失败；`[a-p]{32}` 收紧 |
| B2 | GitHub 配置与 Windows workflow source parity | B1 | 手动触发只构建目标 tag；source/tag/SHA 不一致在打包上传前失败；host manifest 生成不再多处手写 |
| B3 | Windows Sidecar v2 handshake 稳定性 | 无 | 目标测试在 Windows 连续 3 次通过；失败输出可诊断；不使用 `continue-on-error` |
| B4 | E5 Windows Named Pipe Desktop transport | B3（发布前） | Linux adapter/framing/error-mapping 测试通过；Windows ACL、多连接、重启、reconnect 保持 `WINDOWS_VERIFICATION_PENDING` |
| B5 | E6 Native Host Registry lifecycle | B1 | HKCU inspect/install/repair/unregister 幂等、可回滚、不覆盖未知项、写入 platform adapter |
| B6 | E7 实时连接状态 | B4、B5 | files/Registry/browser/transport 四类状态不混淆；状态可追溯到实际证据 |
| B7 | E8 Extension ZIP 与 packaging/release parity | B1、B2 | Extension ZIP、四类 Windows 资产、hash/size/license/source/version 自动一致；包内无私钥 |
| B8 | Linux development phase 收口 | B0–B7 | Linux applicable verification PASS；Windows Validation Queue 完整 |
| B9 | Windows Validation Preparation | B8 | 按 Build/Runtime/Filesystem/Integration/Packaging/Regression 合并重复场景后输出 handoff |
| B10 | Windows 集中验证 E1–E9 | B9 | 逐项记录 PASS/FAIL/BLOCKED/NOT RUN 及证据 |
| B11 | 新 pre-release | B10 | tag、workflow `headSha`、source commit 与资产完全一致；不复用也不移动旧 tag |

#### 发布级别

- **Level A（Desktop-only）：**只发布 `.exe` 与 application-only `.7z`；不得声明 Extension/Native Host 可用；
- **Level B（Packaging）：**四类 Windows 资产（`.exe`、application-only `.7z`、repository-dependencies `.7z`、full `.7z`）齐全且 source parity 正确；不得声明真实浏览器链路可用；
- **Level C（Browser integration）：**在 Level B 之上再满足 Registry PASS、Browser load PASS、Named Pipe PASS、archive/query/reconnect PASS。

#### 发布门禁

Build PASS、Packaging PASS、Registry PASS、Browser load PASS、Named Pipe PASS、End-to-end archive/query PASS 必须分别取证；只完成较低级别时，Release notes 必须写明 Windows package build verified、real browser-to-Desktop integration remains pending。

#### 已知污染的 Release

`v0.2.0-pre.5` 与 `v0.2.0-pre.6` 不能再作为 Extension/Native Host 发布基线：`pre.5` 缺少 `XARCHIVE_EXTENSION_ID` 导致 Native Host 步骤失败，`pre.6` 既是 Sidecar handshake 失败，其现有资产又来自 `main` 的手动 run `35518832674`（仅为 application-only 资产）。修复后应创建新 tag（如 `v0.2.0-pre.7`），不得 force-move 已发布 tag。

## 5. 当前迁移边界（2026-09-19）

U8 之后旧路径迁移已结束：Sidecar protocol v1 runtime、gallery-dl 媒体下载、`DownloadRouter` fallback、`archive_tweet` 同步入口和 v1 Schema/entrypoint 都不再存在，当前媒体链路是 gallery-dl extraction-only → aria2-only transfer。文档仍需区分：

- `CURRENT`：代码和测试已证明；
- `PLANNED`：目标架构但尚未实现；
- `MIGRATION`：新旧路径并存；
- `WINDOWS_VERIFICATION_PENDING`：Linux 无法替代的 Windows 证据。

## 6. Signed Remote Component Catalog TODO

### 2026-09-22 Windows validation reconciliation

The current ccaa649 working tree has completed the controlled local Windows
native WDIO revalidation for WQ-P1-16 and WQ-P1-17 after clean npm ci:
ordinary Dashboard 3/3 and advanced Dashboard/plugin 5/5 passed. The remaining
release prerequisites are exact tauri-driver 2.1.0-alpha.0 parity, WDIO
teardown cleanup follow-up, hosted/release-runner repetition, and the
manual/real integration items retained in the Windows queue. These results do
not promote Registry, browser, Named Pipe, real extraction, signing or final
release acceptance to PASS.

Linux 提交记录（2026-09-22）：上述 banner 修复测试基础设施已提交为 `03332a1`（`fix: accept Microsoft Edge WebDriver banner in Tauri E2E harness`，18 files changed）并推送到 `origin/feature/u7-desktop-production-integration`；working tree clean，`v0.2.0-pre.8` 未移动。提交边界前后的内容一致（提交未修改任何内容文件），因此 Windows 09-22 两轮结果仍描述该 revision。提交后 Linux 复核：Desktop Node `82/82`、Extension check/test `21/21`、Desktop Vite production build 与 `git diff --check` 通过；本提交未改动 `crates/`、`desktop/src-tauri/` 或业务前端代码，Rust 门禁沿用本批次先前记录。下一轮 Windows 同步以 `03332a1` 为 revision 基线，并必须先解决 pinned `msedgedriver 152.0.4191.66` 与实机 Edge 154 的兼容性（或提供受控 Edge 152 runtime），再重跑 ordinary/advanced gate；断言、timeout 与生产 capability 不得为此放宽。

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

---

### 2026-09-22 readiness gate 目标发现修复计划（v0.2.0-pre.10 后续，当前工作项）

> 背景：Windows run `35699308051` 两次确定性地失败于最终 WDIO gate：session 创建成功，但附着的 WebView2 文档在 20 秒预算内始终为空白 `data:,`（`rootExists:false`），Dashboard 未出现。现有证据不足以区分「session 附着到错误/初始 target」与「应用从未完成首次导航」，因此本轮先补可诊断性，再修 target 选择，最后才考虑产品修复。

#### Phase 划分

- Phase 0（发布语义冻结）：`v0.2.0-pre.10` tag/零资产状态保持不变；修复在新 prerelease tag 收口；不弱化 readiness 断言。
- Phase 1–2（harness，Linux 先行）：新增 `desktop/e2e/support/native-startup.mjs`：窗口 handle 枚举、按产品标记（`#root`/`data-xarchive-startup`/fallback/title）识别应用文档、`waitForApplicationDocument` 状态机（`NO_WINDOW_HANDLES`/`ONLY_BLANK_DOCUMENTS`/`APPLICATION_DOCUMENT_NOT_FOUND`/`APPLICATION_DOCUMENT_FOUND`）、`waitForStartupContract`（root + `react_mount_completed` + fallback 缺失）；`dashboard.e2e.mjs` 改为先发现目标再断言；失败写入 `failure.json`、handle timeline、page source、screenshot（不再静默吞错）；单元测试 `desktop/test/native-startup.test.mjs`。只使用标准 WebDriver 命令，不恢复 `plugin:wdio` focus probe。
- Phase 3（workflow 诊断）：spec 证据直接写入 `READINESS_DIAGNOSTICS/startup/`（绝对路径）；`WDIO_STARTUP_DISCOVERY_TIMEOUT`/`WDIO_STARTUP_CONTRACT_TIMEOUT` 由 workflow 注入；WDIO service 日志目录与 `WDIO_LOG_DIR` 不一致的契约问题需在依赖源码核实后修复，不得继续用 `Copy-Item -ErrorAction SilentlyContinue` 掩盖日志缺失。
- Phase 4（hooks 记录）：session 建立后的初始 handle/URL/capabilities 快照写入诊断目录（在 Phase 1 模块内实现，不恢复 plugin probe）。
- Phase 5（preflight/gate 隔离）：preflight 与 gate 之间新增残留检查（应用/driver 进程与 1420/4444/4445/9223 端口），有残留即 FAIL，不得带污染启动 gate。
- Phase 6（产品修复门槛）：仅当诊断证明「应用 handle 存在但资源未加载」或「root 存在但 marker 未完成」时才允许修改产品代码，且必须独立 commit。
- Phase 7（Linux 验证）：已完成。desktop node tests 89/89、`node --check`、Vite check、workflow YAML 解析、`git diff --check` 均通过；Rust/Python 仅在 release candidate 前全量执行。
- Phase 8–9（Windows 验证）：因环境限制无法在当前 Linux 开发环境执行。8个验证项目已在 `windows-queue.md` 中登记为 `WINDOWS_VERIFICATION_PENDING`。需要 Windows 10/11 环境、WebView2、Node/npm、Tauri driver、匹配的 msedgedriver 版本才能执行。详细手工验证步骤见 `windows-queue.md` 和 `windows-wdio-handoff.md`。

#### 提交拆分

1. docs：本计划与队列/handoff 登记；
2. harness：native-startup 支持模块 + spec 改造 + 单元测试 + repository-map 登记；
3. workflow：`windows-release.yml` 诊断/隔离增强 + hosted readiness diagnostic workflow；
4. 结果回写与 handoff 更新在 Windows 验证后进行。

#### 完成标准（当前）

Linux development phase 已完成：Phase 0-7 全部验证通过，修改的文件已准备好提交。具体验证结果见上述记录。

Windows validation phase 已部分执行（d3fd814）：EdgeDriver 152/Edge 154 不兼容导致 session 创建失败，Edge 154 诊断确认 blank document 是 Windows native Tauri startup/target-attachment 失败而非 driver 问题。8个验证项目状态已更新至 `windows-queue.md` 和 `windows-validation.md`：
- WQ-P0-WHITE-04A: PASS (invocation scope) — 超时注入已验证
- WQ-P0-WHITE-01B: FAIL — EdgeDriver 152/Edge 154 不兼容
- WQ-P0-WHITE-01D: FAIL — msedgewebview2 状态变化导致隔离检查失败
- WQ-P0-WHITE-04B: FAIL — WDIO 日志写到 desktop/logs
- WQ-P0-WHITE-01A/01C/03R/04C: BLOCKED/NOT RUN — 依赖 session 创建

详细手工验证步骤见 `windows-queue.md` 和 `windows-wdio-handoff.md`。

#### 完成标准（最终）

hosted runner 上以正式 pinned 工具链连续通过：应用 WebView 目标被识别（URL 离开 `data:,`）、`#root` 存在、`react_mount_completed`、fallback 缺失、Dashboard 3/3、诊断产物完整、退出无应用/driver 进程与端口残留；随后以新 tag 发布并保持 manifest/hash/资产一致。

#### 禁止项

不删除或放松 Dashboard、`#root`、`react_mount_completed` 断言；不用 `browser.url()` 人工导航制造通过；不用进程存活替代 UI readiness；不向 `v0.2.0-pre.10` 补传资产或复用其 tag；不为通过测试修改产品 capability、依赖版本或 driver 策略。
### 2026-09-22 d3fd814 Windows validation reconciliation

The current readiness-gate implementation is Linux-verified and Windows
build/preflight-verified, but the exact pinned local readiness gate is not
ready to pass: EdgeDriver 152 rejects installed Edge 154 before target
discovery. The new target-discovery and evidence checks remain blocked behind
that prerequisite; isolation and log-directory checks exposed separate
machine/harness issues. Next work must align the controlled browser runtime and
driver, then rerun local pinned readiness before hosted diagnostics.
### 2026-09-22 Edge 154 diagnostic reconciliation

The Edge 154 diagnostic driver is preserved in the E: validation project at
E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\msedgedriver-154.0.4258.24\msedgedriver.exe.
The driver version/hash and EdgeDriver-to-Edge compatibility checks passed, but
the local E2E gate remained FAIL: even with an absolute release executable
path, the session stayed at data:, and no XArchive application document was
found within 30000 ms. Linux follow-up is limited to investigating Windows
Tauri launch and WebView target attachment; this result does not authorize
product-code changes or promote the pinned Edge 152 workflow gate.

### 2026-09-22 current-source Windows validation rerun

The current readiness-gate Plan remains Linux-verified but not Windows-passing.
The local Edge154 diagnostic run passed dependency, build, driver compatibility,
direct preflight and failure-evidence capture, then failed because the
WebDriver-attached WebView2 stayed on data:, and never exposed the XArchive
document. Advanced native validation is BLOCKED by this shared prerequisite;
hosted stability is NOT RUN. Next Linux work is limited to controlled runtime
alignment, WebView target-attachment diagnosis and session-start/log contract
repair. Do not expand this validation into business-code development or weaken
the readiness assertions.
### 2026-09-22 Linux 测试基础设施修复：session-start 快照与日志目录契约

current-source rerun 指出的「session-start/log capture contracts」Linux 修复
任务已完成（仅测试基础设施，业务代码零改动）：

- **04C**：`snapshotSessionStart` 已接入 `dashboard.e2e.mjs` 的 `before` hook
  （在等待应用文档之前），并填充 `capabilities`；session 创建后必然产生
  `startup/session-start.json`。
- **04B**：依赖源码核实确认 service 日志捕获读取 WDIO config 的 `outputDir`
  而非 service 选项 `logDir`；`wdio.conf.mjs` 现显式设置 `outputDir: logDir`
  （与 `WDIO_LOG_DIR`/`READINESS_DIAGNOSTICS` 一致），gate 结束后新增非空
  `*.log` 完整性检查，无日志即 FAIL，不再用 SilentlyContinue 掩盖缺失。
- **01D**：隔离检查修复为只对应用/driver 进程残留与 readiness 端口监听
  FAIL（写 `isolation-failure.txt`）；msedgewebview2 后台活动降级为诊断信息，
  消除机器级噪声误报。

Linux 验证通过：`node --check`、desktop 单元测试 91/91（含 2 个新增
`snapshotSessionStart` 测试）、Vite check、workflow YAML 解析、
`git diff --check`。WQ-P0-WHITE-01D/04B/04C 已在
`windows-queue.md` 中标记为 `WINDOWS_VERIFICATION_PENDING`，等待 Windows
受控 runtime 对齐后重新验证；01A（blank target）仍为 Windows 端
Tauri startup/target-attachment 问题，未因此轮修复而改变。

### 2026-09-22 Windows revalidation outcome for the readiness-gate plan

本地 E: workflow 等价 gate 已验证 01D、04B、04C：隔离检查、非空 WDIO 日志
和 session-start 快照均 PASS。01A 仍为 Windows FAIL（session 为 `data:,`、
全程 `ONLY_BLANK_DOCUMENTS`）；advanced/native dashboard checks 因共同前置
而 BLOCKED，hosted diagnostic 仍 NOT RUN。计划下一步保持为 Windows target
attachment/runtime investigation 与 hosted confirmation，不扩大为业务代码开发。
### 2026-09-22 01A runtime-pairing 调查与 05A 验证项（22:30 后续）

revalidation 确认 01D/04B/04C 修复有效后，剩余唯一 blocker 01A 的两个候选
原因已在 Linux 端完成代码级调查（零代码修改）：应用窗口配置正常（单个
`main` 窗口，无 `visible:false`）、直接启动可渲染 Dashboard 排除「应用从不
导航」；依赖源码（`@wdio/tauri-service` `resolveTargetEdgeVersion`）证明
msedgedriver 应匹配实际渲染应用的 WebView2 Runtime（E: 机器为 Evergreen
153.0.4234.48），而 22:30 诊断 run 按 Edge 浏览器 154 匹配 driver 造成
driver(154) 驱动渲染引擎(153) 的错配——与「session 创建成功但 target 永远
`data:,`」症状一致。已登记 `WQ-P0-WHITE-05A` runtime-pairing 实验
（路径 A：msedgedriver 153.0.4234.x；路径 B：固定版本 WebView2 Runtime 154 +
`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`），状态 `WINDOWS_VERIFICATION_PENDING`。
05A 结果不得直接提升 pinned 152 workflow gate 或 hosted 稳定性；05A 通过后
仍需回到 pinned 工具链（01B）与 hosted 确认（01C）。

### 2026-09-23 P1–P3 修复与「指定账号批量下载」Plan（已批准，非 Windows 实现已收口）

> 本节是对照参考仓库 `hureyqi/x-spider-mod-2026`（评估基线 commit
> `4fd46b66269761e1109309c6f21ba573f4836444`）后批准的功能计划。P1-A/C/D、
> P2-A/B/C 和 P3-A/B/C/D 的非 Windows 实现与测试已完成；P1-B 真实样本、
> P3-E 真实账号和 Windows 验证仍按环境边界保持 NOT RUN/WINDOWS_*。参考仓库的
> 分页发现/筛选/下载管理可借鉴，但其硬编码 GraphQL、仅按文件存在跳过下载、
> 以及完整 RPC 参数入日志的做法不进入本项目。

#### P1：归档完整性前置修复（必须先于批量能力）

| ID | 内容 | 责任方 | 完成标准 | 所需验证 |
|---|---|---|---|---|
| P1-A | 提取契约贯通 `user_id`、`reply_to`、`quoted_tweet`（含 quoted `user_id`）：Python `ExtractedTweet` → protocol v2 `ExtractionResult` → Rust `extraction_to_metadata` → `ArchiveMetadata`/SQLite | Linux Cross-platform Owner | 四层字段一一对应；JSON Schema、Rust `deny_unknown_fields`、Python 序列化键集合一致；不引入 signed URL/headers/Cookie | Rust protocol/storage/desktop 定向测试 + Sidecar pytest |
| P1-B | 真实 gallery-dl 输出契约验证：`info.json`/`*.info.json` 多文件选择、媒体结构、失败重试后的旧文件污染 | Linux Cross-platform Owner | 用脱敏真实样本覆盖单图/多图/视频/纯文本/引用/回复；不允许静默漏媒体 | Sidecar pytest（fixture 必须标注 synthetic；真实样本验收单独记录） |
| P1-C | 作者身份策略：浏览器临时 `browser-<tweet_id>` 身份在 Sidecar 返回稳定 `user_id` 后升级为稳定用户；明确临时行处置 | Linux Cross-platform Owner | 同作者多 Tweet 稳定聚合；改名不改变主键；临时身份有可追溯迁移语义 | storage/desktop 测试；Windows 浏览器实测仍走 Windows queue |
| P1-D | README 中 U8 legacy-path removal 与「尚未完成」的矛盾表述归一 | Linux Cross-platform Owner | CURRENT/PLANNED 边界与 `status.md` 一致 | 文档核对 |

#### P2：可靠性与操作闭环

| ID | 内容 | 责任方 | 完成标准 |
|---|---|---|---|
| P2-A | 统一提取、aria2、Telegram 网络配置（代理/超时/重试）；秘密不入 SQLite、普通日志与浏览器消息 | Linux Cross-platform Owner | 配置路径可诊断；日志 redaction 测试通过 |
| P2-B | 任务暂停/恢复/批量重试语义先落共享状态模型，再接 UI；重启后状态可恢复 | Linux Cross-platform Owner | 暂停≠取消；重试幂等；队列满为背压而非失败 |
| P2-C | 媒体规格策略：原图、视频 variants、动图、无媒体 Tweet 的完整性定义 | Linux Cross-platform Owner | 归档完整性判定不退化为「文件存在」 |

#### P3：指定账号批量下载（新功能，复用单 Tweet 归档链路）

目标结构（非 Windows 生产链路已实现）：

```text
输入 @username/主页地址 + 筛选（日期范围、数量上限；默认仅本人含媒体 Tweet）
  → 账号解析（绑定稳定 user_id，username 仅展示）
  → 账号内容发现（分页/流式候选，可取消）
  → Rust 持久化批次与候选、去重、筛选
  → 有界派发到现有单 Tweet 归档执行器
  → gallery-dl extraction-only → aria2 transfer → staging 校验 → ArchiveService 提交
  → 批次进度/失败汇总（暂停、继续、取消、失败重试）
  → 暂停只阻止后续派发并取消活动发现，不取消已提交归档 Job；重启后按 Job 表恢复 SUBMITTED，按候选表重新选择 PENDING。
```

依赖顺序与阶段：

| 阶段 | 内容 | 依赖 | 完成标准 |
|---|---|---|---|
| A | P1-A/P1-C 提取契约与作者身份 | 无 | 批次绑定的 user_id 在归档后可查询、可聚合 |
| B | gallery-dl 账号发现可行性 spike：能否分页/流式、能否取回稳定 Tweet ID 列表、认证/限流错误可分类 | P1-B | 记录 CURRENT 或 NOT POSSIBLE；不虚构断点游标 |
| C | 批次/候选持久化与有界派发（复用 `JobExecutor` 背压与活动任务复用） | B | 重启不丢候选、不重复提交；取消批次不误杀共享任务 |
| D | Desktop 入口与进度控制 | C | 创建/暂停/继续/取消/重试闭环；发现未结束不显示虚假百分比 |
| E | 受控真实账号验收 | D | 多页发现、重复运行幂等、认证失效暂停、文件 SHA-256 完整 |

明确边界：首期一批次一账号、手动触发、完整归档入选 Tweet 的全部媒体；不包含
定时订阅、多账号登录池、自动 Telegram 群发。媒体类型筛选若只归档部分媒体，必须
先引入「部分归档」状态，否则暂缓。发现结果只作候选，媒体 URL 在执行归档时重新
extraction，不长期复用。

责任路由：批次/候选 Schema、调度与共享 GUI 状态归 Cross-platform Owner；
Windows 浏览器凭据读取、Registry/Named Pipe、打包与原生 GUI 验收归 Windows
Platform Owner，按最小必要批次 handoff，不由常规 Windows 验收打断共享开发。
