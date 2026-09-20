# XArchive 总体开发路线图

> 基准日期：2026-09-20。本文只记录未来方向、依赖和完成标准；当前实现事实以 [`status.md`](status.md) 为准，Windows 验证事实以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准。

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
4. **Native Host packaging（LINUX_VERIFIED）：**Full portable/release 相关包契约已明确包含 host executable、manifest 和 Extension ID/`allowed_origins` 校验；当前用户 Registry registration、repair、unregister 和安装路径修复尚未实现，仍为 Windows follow-up。
5. **Connection/retry（LINUX_VERIFIED，Windows pending）：**NativeBridge 已处理 `runtime.lastError`、断开时 pending request、失效 port、同步连接失败和下一次请求重新连接；Desktop status 仍需接入真实 Windows transport session，不能由文件存在推断 connected。
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

### U17：Browser Extension production hardening（当前后续开发 Plan）

U17 是在 U12 Linux scope 完成后新增的 Extension 专项开发单元。它不把当前的 Native Host package contract、Unix transport 测试或 Extension Node 测试外推为 Windows 浏览器集成完成。U17 的目标是将当前“MV3 DOM adapter + NativeBridge 原型”推进到可诊断、可测试、可集中 Windows 验证的浏览器归档链路。

依赖顺序固定为：

```text
E0 文档/事实对账
  → E1 Browser protocol/schema 收口
  → E2 DOM identity 与 fixture 测试
  → E3 NativeBridge timeout/reconnect hardening
  → E4 页面状态同步与 query_status 批量消费
  → E5 Windows Named Pipe Desktop transport
  → E6 Native Host Registry install/repair/unregister
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

- 在 Desktop 增加 Windows Named Pipe server，与现有 Unix transport adapter 保持相同 BrowserRequest/BrowserResponse 契约；
- 明确固定 pipe name、当前用户 ACL、多连接、退出、错误和 reconnect 行为；
- Native Host Windows client 仅负责 Named Pipe client，不把平台逻辑混入协议 crate；
- 完成标准：Windows 上真实 `query_status` 与 `archive_request` 可由 Native Host 转发到 Desktop，request_id 正确匹配。

#### E6：Native Host Registry lifecycle

- 增加当前用户级 Chrome/Edge Native Messaging Host inspect/install/repair/unregister；
- portable root 移动后能诊断和修复绝对路径；默认不写 HKLM，不覆盖未知注册项；
- 将 Registry 副作用保持在平台适配层；
- 完成标准：真实用户环境下注册、修复、取消注册均可回滚且不残留失效路径。

#### E7：实时连接状态

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