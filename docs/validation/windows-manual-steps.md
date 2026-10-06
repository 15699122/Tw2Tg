# Windows 专属验证汇总与手工步骤

## Download history continuation — 2026-10-06 Linux batch

- Target branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source baseline: commit `2e3d738eb6833d2c9b88f5f64b911a342d9470e5`. The final
  Linux handoff revision is `72762cd97247b881180a853b76714d29a543c28d`, pushed
  to `origin/cross-platform/automatic-pairing-reconcile-20261002`. Build and
  test that exact commit; do not use the old baseline or a copied workspace as
  the canonical handoff artifact.
- Linux environment detected: WSL2; Windows PowerShell bridge is present, but
  display variables and `wsl.exe` are also present. There is no Windows-owner
  artifact identity, artifact-bound native GUI/runtime session, or authorized
  test account available here. Native acceptance is `BLOCKED` / `NOT_RUN`, not
  PASS. Windows Owner should run all WQ-DL-01..08 below against one fresh
  artifact and record source SHA, build origin, executable SHA-256,
  Windows/WebView2/DPI and tool versions in the queue.
- Windows handoff status: `READY_FOR_WINDOWS`; current owner is Windows Platform
  Owner for native acceptance. Executor attempt-metric instrumentation remains
  deferred pending shared contract review and is not a prerequisite for this
  handoff. No Windows acceptance is claimed here.

Before running any steps, update/check out the eventual Git handoff commit,
build a fresh Full package on Windows, verify its source revision and record its
exe/package SHA-256. Use a dedicated test profile and authorized test account;
never include cookies, tokens, authorization headers, signed URLs or raw
credential-bearing logs in shared evidence. Keep redacted evidence linked from
`docs/validation/windows-queue.md`.

Run the **P1–P8 `use_aria2` manual procedure** below, together with the exact
artifact/evidence prerequisites above; it covers WQ-DL-01..08: packed worker
handshake, real gallery-dl archive, aria2 process boundary, cancellation/timeout,
junction/reparse containment, Downloads page navigation and toggle persistence
across DPI/keyboard/restart, aria2 archive regression, and interruption/recovery.
Each WQ row requires an individual
PASS/FAIL/BLOCKED/NOT_RUN result; do not combine distinct acceptance targets.
Record screenshots only for GUI layout/focus/navigation; record process-tree
and redacted application logs for runtime checks; record archive inventory,
sizes and hashes for completed transfers. Any failure must retain the exact
artifact identity and reproducible steps, then route shared-contract findings
back with `CROSS_PLATFORM_CHANGE_REQUIRED`.

## Current Full candidate — 2b98952

Use the new 0.2.1 dev Full ZIP, not previous packages. [Installation/manual continuation](windows-full-2b98952-manual.md), [exact results/remaining prerequisites](windows-full-2b98952-results.md). Current-size GUI and focused keyboard/restart subset passed; real-account/browser/Telegram/PAC/DPI matrix remains pending. Cross-platform reviews WebSocket close timing, Windows executes native/manual acceptance.

---

## Current 5c3efb79 manual continuation

Use `44e60e369b5c57d3ed66b46fb610dc906a45780f` fresh artifact, not earlier package hashes. [Batch manual queue and prerequisites](windows-5c3efb79-results.md#manual-windows-queue-and-prerequisites) is the current M13/M10/P6 continuation; queue IDs stay in windows-queue.md. M13 must include implicit localhost/loopback bypass versus throwing PAC and actual egress under isolated Registry/WPAD settings. Current-size Settings/proxy render passed; full GUI matrix and Full/real integrations remain NOT_RUN. Windows Platform Owner executes after Cross-platform Git reconciliation.
---

## P. 可选下载模式 `use_aria2` — WQ-DL-01..08

Source: Linux batch 的最终 Git handoff commit
`72762cd97247b881180a853b76714d29a543c28d`（分支
`cross-platform/automatic-pairing-reconcile-20261002`）。不得使用旧基线
`16beade` 或 `2e3d738` 制作验收 artifact。状态：全部
`NOT_RUN`；Owner: Windows Platform Owner。
对应队列项见 [`windows-queue.md`](windows-queue.md) 的 WQ-DL-01..08。

前提：全新 Full package（内置 worker 与 gallery-dl）、交互桌面、可控测试数据。
不得在任何共享证据中写入 cookie/token；worker 日志只保留脱敏片段。

### P1 握手与 capability（WQ-DL-01）

1. 从 Settings 启动 Sidecar，确认侧栏显示"正在运行"。
2. 检查 worker stdout/应用日志中的 `ready` 事件，capabilities 必须恰好为
   `extract_media`、`download_media`、`cancel_active_extraction`、
   `structured_media_plan`、`account_discovery` 五项。
3. 用手工 JSONL 探针对 worker 发送一条 `download` 命令，必须携带 job 作用域的绝对
   `staging_dir`：
   `{"protocol_version":2,"request_id":"r1","cmd":"download","job_id":"job-1","url":"https://x.com/<user>/status/<id>","staging_dir":"C:\\...\\job-1"}`。
4. 期望：`download` 被接受（不是 `INVALID_COMMAND`），并依次输出 `download_started`
   与 `download_completed`。若 worker 未声明 `download_media`，握手必须失败而不是回退。

### P2 真实下载与归档（WQ-DL-02）

1. 侧边栏 → "内容下载" → "下载方式"确认开关处于关闭状态并保存；重启应用确认持久化。
2. 用已授权账号提交一条至少含 1 张图片和 1 个视频的 Tweet，跟踪 Job 进度。
3. 完成后检查归档目录：应含 `tweet.json`、`tweet.txt` 与媒体文件；gallery-dl 写入的
   文件名为 `{num:>02}.{ext}`；记录的 size/SHA-256 与磁盘一致。
4. 确认归档目录中**没有** `info.json`、`gallery-dl.jsonl` 等 gallery-dl 元数据残留。
5. Job 应到达 `COMPLETE`，不出现 `DOWNLOAD_TIMEOUT` 或 metadata mismatch。

### P3 aria2 未启动（WQ-DL-03）

1. 开关关闭时提交下载，边运行边采样进程树（Process Explorer 或
   `Get-CimInstance Win32_Process | Where-Object Name -eq 'aria2c.exe'`）。
2. 期望：整个过程**不出现** `aria2c.exe`。
3. 再开启开关重复一次，确认此时 `aria2c.exe` 出现并随 Job 退出。

### P4 取消与超时（WQ-DL-04）

1. 开始一次可控的长下载（大视频或受控 fixture）。
2. 下载进行中发送 cancel，等待终止后检查 worker、gallery-dl 及其子进程 PID。
3. 期望：得到稳定 `CANCELLED`；进程树全部退出；staging 不再写入；不会提交半成品归档；
   残留文件不得被当作本次下载结果上报。
4. 另测 timeout：确认得到 `DOWNLOAD_TIMEOUT` 而非其他错误码。

### P5 路径安全与 reparse（WQ-DL-05）

1. 在 staging 根下创建指向归档树之外的 junction/reparse 点。
2. 构造使上报路径经过该链接或逃逸 staging 的场景。
3. 期望：被拒绝并给出路径错误，**不向归档树之外写入任何文件**；正常路径仍能成功。

### P6 内容下载页面与设置 GUI（WQ-DL-06）

1. 在新 handoff 的 Desktop artifact 上启动应用；从侧边栏打开"内容下载"，确认导航高亮、标题、页面切换正确，返回"设置"也正常。
2. 在 100%/125%/150%/200% DPI 及窄/宽窗口下检查"下载方式"和 aria2 区块：标题、标签、帮助文字不裁切，区块之间无重叠；Windows 上 aria2 管理控件可用。
3. 键盘操作 Tab/Shift+Tab 到下载方式开关，确认可见焦点；用 Space 和 Enter 各切换一次，确认只触发一次状态变化且忙碌期间不可重复提交。
4. 开关切换后重启应用，确认状态与 `config.yaml` 的 `use_aria2` 一致；导航切页再返回后页面状态仍与保存值一致。
5. 对精确 artifact 记录 source SHA、构建来源、exe SHA-256、Windows 版本、缩放比例/窗口尺寸、WebView2/Node/Rust 版本与结果；截图不得包含账号凭据。

### P7 aria2 路径回归（WQ-DL-07）

1. 开启开关，用与 P2 相同类型的 Tweet 提交归档。
2. 期望：aria2 transfer 路径正常完成，归档内容与 P2 等价；新 capability 未引入协议回归。

### P8 重启恢复（WQ-DL-08）

1. 在 `download` 阶段关闭应用模拟中断，重启后观察对账。
2. 期望：Job 记为 interrupted 而非 complete；恢复过程可诊断；不提交部分归档；
   重试后能得到完整归档。

> 以上任何一项都不得由 Linux 证据、gallery-dl 命令形状或单元/fixture 测试推定为 PASS。

## O. ca25e53 settings presentation acceptance — 2026-10-05

Source ca25e5379302431a3130436896735b8be2bb4744; use target/release/xarchive-desktop.exe with SHA-256 d9457efe8154f165650d1bd18a49844aed9d7d19a37f8d6d6e12350980e67ccc. Status NOT_RUN for the remaining matrix; Windows Platform Owner. Prerequisite: exact artifact, interactive desktop and controlled test data.

1. At 100/125/150/200% Windows scaling, inspect narrow and wide windows: Bootstrap/Telegram spacing, icons and summary/help wrapping; capture scale, dimensions and screenshots. Previous Full-package screenshots do not prove this build.
2. Keyboard-test every disclosure with Enter/Space, visible focus and Tab/Shift+Tab; confirm closed fields are skipped and sidebar jumps focus their corresponding header. Sidecar toggle behavior and Extension scroll/expand are already scoped PASS.
3. With controlled fake credentials and no external send, verify masking and disabled/busy/error layouts. Confirm folding/expanding and refresh do not alter saved settings or trigger unrelated services; retain independent before/after evidence.
4. Record each subcheck with exact SHA/hash and outcome; keep inherited release/subtree/Telegram queues at their own scope.


## N. Current continuation follow-up — 2026-10-05

1. WQ-SEC-SIDECAR-QUEUE-01: independently verify fake descendant startup and entire subtree exit after cancel and full shutdown. Backpressure, matching cancellation, shutdown, EOF and worker exit on exact aa8dabd worker already passed; do not repeat solely because subtree evidence is incomplete. EOF marker child exit passed.
2. WQ-SEC-RELEASE-PERMISSIONS-01: after integrating 39f54e5, perform a full production pipeline rehearsal in an isolated scratch repository/fork with tag publication disabled and upload=false. Never push a v* rehearsal tag to the canonical repository: its tag-push workflow publishes automatically. Verify flat seven-file transfer and publication revalidation without release writes; retain run URL and default-branch CodeQL rescan evidence. Trusted resource gate and isolated Windows Actions already PASS.
3. WQ-SETTINGS-PANELS-01: current exact Desktop, remaining 100/125/150/200% narrow/wide matrix, keyboard/focus and hidden Tab, Extension jump, refresh independence, fake credential masking, busy/error and backend-effect checks. Mouse Telegram and SQLite/Sidecar jumps already observed at current size.
4. Real Telegram/Channel and archive/duplicate remain pending under existing prerequisites; Owner chose local/CI only for this continuation. Do not request credentials in chat.

## O. Reconciled ca25e53 Full follow-up — 2026-10-05

Return `0aa8d14be1f3d4d3e5aac0886def850e9536f1b7` was integrated by the
Cross-platform Owner. Use the next committed handoff and a fresh artifact for
follow-up; the prior Full artifact is identified in
[`windows-full-ca25e53-manual.md`](windows-full-ca25e53-manual.md).

1. Preserve scoped reported PASS: automatic browser connection/task creation,
   Telegram authentication and explicit Channel test receipt. Do not infer real
   archive completion or automatic archive-to-Telegram delivery.
2. Reproduce the reported XArchive `DOWNLOAD_TIMEOUT` only with the authorized
   controlled sample; capture task-scoped, redacted gallery arguments, stderr and
   progress. Compare application invocation with the reported successful standalone
   invocation of the same packaged binary and URL. Do not expose cookies/tokens.
3. Reproduce confirmation-dialog ACL errors/action-unknown without blindly
   repeating an uncertain action; capture the exact UI action and redacted error,
   then inspect its authorization/capability path with Cross-platform Owner.
4. Verify a successful archive, readable outputs/hashes and duplicate behavior.
5. After Desktop restart, timestamp browser and Desktop observations; compare
   automatic recovery against explicit manual “Reconnect.” Do not claim a recovery
   timeout without measured timings. Track the observed port-display mismatch
   separately unless concurrent evidence demonstrates state corruption.
6. Continue all unclosed settings GUI checks from the current queue: 100/125/150/200%
   DPI, narrow/wide layouts, keyboard/focus/hidden Tab, busy/error, backend effects.

Current routing: Windows Owner performs native reproduction/acceptance;
Cross-platform Owner reviews shared extraction diagnostics and confirmation
authorization under `CROSS_PLATFORM_REVIEW_REQUIRED`. No shared defect is
established and no production fix is authorized by this evidence alone.

Owner: Windows Platform Owner 执行；Cross-platform Owner 维护本索引。
Status: `CURRENT` — 各项验证绑定对应 revision/artifact；当前范围与结果以最新 Windows history 和 queue 为准。本轮仅完成本地／CI 验证，Telegram 人工步骤保留。

## M. Security remediation — release assets and Sidecar queue

本轮源码基线：`adedcceca5e01a8f0a2c9d2bfa9432308496d177`；正式执行须先 fetch
并 checkout 后续 Git handoff commit。当前环境为 Linux WSL2，无 Windows runner、
PowerShell、Windows process/runtime 或 GUI 证据；以下检查 `NOT_RUN`，不可记 PASS。

### M.1 Release asset verification — WQ-SEC-RELEASE-ASSET-01

前置：Windows Owner 确认 gallery-dl/aria2 的可信来源，并审查固定 SHA-256 或
签名来源；隔离 Windows VM/Actions runner；使用非生产测试资产。

1. 在最新 handoff SHA 上检查 asset 下载配置、固定版本、预期摘要来源及保存位置；
   确认摘要不是从同一次未验证下载中推导。
2. 下载原始预期资产，记录 source URL/version 与资产 SHA-256；在执行/解压前验证。
3. 验证原始资产通过后，再用隔离副本执行最小 worker 启动或 ZIP 列表/解压检查。
4. 对副本改动至少一个字节；分别测试错误摘要、缺失摘要、下载失败/空文件。
5. 对每种负例确认流程 fail closed，执行和解压步骤均未发生；保存 runner log、
   source SHA、artifact hash、工具版本及 workflow run URL。

预期：只允许与独立可信固定摘要/签名匹配的资产进入执行/解压；所有负例在该
边界之前失败。禁止用真实 release token 或正式发布资产做负例。

### M.2 Build/publish permissions — WQ-SEC-RELEASE-PERMISSIONS-01

前置：测试分支和可查看 Actions 权限/日志的仓库管理员；不得触发正式发布。

1. 逐个检查 release workflow 的 workflow/job `permissions`、`GITHUB_TOKEN` 使用、
   `actions/checkout` 凭据持久化、artifact 上传与发布步骤。
2. 检查 build job 是否获得 contents/releases/write 或等价 publish secret；能拆分时
   让 build job 只构建/上传不可发布 artifact，publish job 仅在验证成功后启动。
3. 在测试分支 dispatch：确认 build job 无写权限且不能创建/修改 release；检查
   publish job 只拿所需最小 scope。
4. 使验证 gate 在测试分支可控失败，确认 publish job 被跳过；保留 job 权限摘要、
   run URL、失败 gate 日志和成功隔离 run 的 artifact SHA-256。

预期：构建凭据不能发布；发布凭据不进入构建 job；缺摘要/验证失败不能发布。
若 GitHub Actions 无法为 job 提供等价隔离，记录具体权限模型限制并返回设计审查，
不得用文档声明替代实测。

### M.3 Sidecar stdin queue / shutdown — WQ-SEC-SIDECAR-QUEUE-01

前置：最新 handoff revision 的 Windows worker artifact；本地 fake extractor；不使用
真实账号、网络或凭据。

1. 启动 worker 并发出长时间 fake extraction；在其运行时通过 stdin 推送超过 32 条
   合法 metadata/log 事件，记录 producer 是否受 backpressure、worker 仍存活且队列
   未无限接收；不要尝试通过诊断工具读取敏感数据。
2. 同一忙任务期间发送匹配 job 的 cancel，确认 fake child 被停止且 worker 返回取消事件。
3. 重复测试：队列满时发送 shutdown；关闭 stdin/EOF；分别确认 shutdown/EOF 不会
   永久阻塞，进程在仓库既定测试超时内退出。
4. 保存 source SHA、worker artifact SHA-256、Windows build/architecture、Python
   版本、每条命令/退出码及脱敏日志；分别记录 cancel、shutdown、EOF 结果。

预期：容量有界并对 stdin 生产者施加背压；cancel/shutdown 有响应；正常协议事件
schema 不变。若发现平台特定失效，附最小复现并标记 `CROSS_PLATFORM_CHANGE_REQUIRED`
或 Windows adapter follow-up，按根因归属。

## L. Browser automatic pairing — current integrated source; handoff pending

**Session capability / status:** Linux WSL2, Linux Rust target `x86_64-unknown-linux-gnu`; no Windows GUI, Edge/Chrome session, Registry, Windows target, or Windows Full artifact was available here. Windows-specific execution is `BLOCKED` for this session (`WINDOWS_EXECUTION_UNAVAILABLE`); this is not a product failure. Real-browser automation additionally has a historical `COMPUTER_USE_UNAVAILABLE` blocker and must be checked again in the actual interactive Windows session. Implementation and documentation are committed and pushed on `cross-platform/automatic-pairing-reconcile-20261002`. Test the source code commit `9920566ef1df115effc3ab5df5d9a12fd42210ed` (inventory fix; includes `5ff0a22` integration), not a stale validation artifact. The branch tip includes subsequent handoff documentation updates.

### L.1 Shared Linux work already completed

Shared contract, ticket/Origin/WebSocket, Host common framing/Unix path, Extension bootstrap/recovery, Windows Pipe implementation integration, and Full inventory correction are implemented. Linux checks for the current code have passed: `npm test`, `npm run check`, `npm run build`; `cargo fmt --all -- --check`; protocol/Native Host tests (protocol 23 tests); targeted WebSocket tests (13/13); docs audit; staged and unstaged `git diff --check`. This evidence validates only those Linux/shared targets and does not establish Winsock, Named Pipe, Registry, GUI, installer, or browser behavior. No full Rust workspace regression suite was run because this batch uses targeted reconciliation checks.

### L.2 Windows handoff procedure (run only after exact Git handoff)

Use an isolated Windows account/VM and disposable Edge and Chrome profiles. Preserve existing user data and registration until the install/upgrade cases explicitly require a disposable test registration. Before testing, record the pushed source SHA, clean/dirty status, Windows build/architecture, Node/Rust/MSVC versions, browser versions/profile, command results, and hashes for Desktop, Host and Full archive. Use the canonical build instructions in [`windows.md`](windows.md); create a fresh isolated output and do not reuse binaries. Redact tickets, tokens and private URLs from evidence.

1. **Build / toolchain (`AUTO-PAIR-BUILD-01`, P0):** fetch and checkout the exact pushed SHA; verify `git status --short` is clean; run `cargo build --locked --workspace --release` and the canonical Full build. Save source SHA, build origin and artifact SHA-256. Expected: build and Full assembly succeed with no binary-reuse fallback.
2. **Host / Pipe bootstrap (`AUTO-PAIR-RUNTIME-01`, P0):** run Host framing/process and targeted Windows Pipe tests, then probe one framed bootstrap through the packaged Host while Desktop is running. Check correlation/request ID, loopback endpoint, generation and in-memory one-use ticket; test bounded retryable response when Desktop is unavailable. Expected: no `BOOTSTRAP_NOT_IMPLEMENTED`, no unbounded wait, no secret in logs.
3. **Winsock (`AUTO-PAIR-WS-01`, P0):** run targeted WebSocket tests on Windows. Verify stop interrupts a stalled pre-auth handshake within the existing bound, idle authenticated sockets are not closed prematurely, and deadline/byte limits hold. Record each test command/result separately; Linux pass is not Windows pass.
4. **Package/filesystem (`AUTO-PAIR-PATH-01`, P0):** inspect `package-manifest.json` `installation.files`, assert `extension/src/browser-pairing.js` is declared and exists in Full, extract the archive into an isolated directory, and compare every declared path against files on disk. Expected: no missing files, path escape or inventory mismatch.
5. **Native registration/install (`AUTO-PAIR-INSTALL-01`, P0):** verify Host manifest name, executable path and `allowed_origins` against the fixed Extension ID; exercise current-user Edge and Chrome registration, install/repair/upgrade/uninstall, and portable-root move using disposable data. Expected: correct registration and helper paths, diagnosable missing Host/Desktop, no arbitrary caller-supplied executable path.
6. **Pairing / UI (`AUTO-PAIR-UI-01`, P0):** load the exact package in each supported browser. Pair with Desktop running. At connect and controlled ~5 s/~45 s observations, capture same-time rendered sidebar/detail and redacted `browser_connection`, `websocket_connection`, `websocket_authenticated` values. Expected: observations are bound to one artifact and time; do not guess which field is authoritative or change semantics before classifying the discrepancy.
7. **Protocol/security negatives (`AUTO-PAIR-ORIGIN`, P0):** test wrong/missing Origin, wrong protocol version, malformed/expired/replayed ticket, two concurrent consumers, and business frame as first WebSocket message. Expected: reject before `BrowserTransportAdapter`; exact allowed origin only; no fallback to unauthenticated Native business forwarding. Confirm ticket TTL is 1–30,000 ms and ticket is one-use/current-generation.
8. **Browser E2E / lifecycle (`AUTO-PAIR-E2E-01`, `AUTO-PAIR-LIFECYCLE-01`, P0/P1):** in Edge and Chrome, query status and exercise the authorized synthetic/approved fixture; do not use a real post without explicit authorization. Test response loss (no blind business replay), worker/browser/Desktop restart, sleep/resume, profiles, and idle durations. Expected: bounded recovery with fresh bootstrap, no stale authenticated state or duplicate submission. If GUI/browser automation is unavailable, record that specific check `BLOCKED` with `COMPUTER_USE_UNAVAILABLE`; continue independent CLI checks.
9. **ACL (`AUTO-PAIR-ACL-01`, P1):** as current user verify expected access; from a second user context attempt pipe access and record denial. Check remote access is rejected. Do not broaden ACL to make the test pass.
10. **Redaction (`AUTO-PAIR-REDACT-01`, P0):** inspect Host/Desktop/Extension logs, errors, URLs, storage, job payload and diagnostic export for ticket/token exposure on success and failure. Expected: no secret persistence or disclosure.

For every row record the required evidence fields from [`validation-policy.md`](validation-policy.md). A missing GUI/browser capability blocks only the dependent GUI checks; it does not block build, package inventory or native CLI checks. Keep historical `6060f5a` failure and `d65a01b` scoped PASS results attached to their own source/artifacts. Feature-level automatic-pairing acceptance remains open until all applicable rows are individually resolved.

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

## K. Telegram 发送（`WQ-TG-*`，共享生产路径已就绪，待 Windows/真实环境验收）

### K1. Local Windows return — 2026-10-04

Current override: adapter IMPLEMENTED at `b5dd8c9`, native synthetic CRUD PASS.
Full product/manual acceptance remains NOT_RUN. User chose local-only validation;
no bot/chat/api_id/api_hash prepared. Earlier Linux capability statements below
describe that prior environment, not the current Windows host.

1. WQ-TG-001 / Windows Owner / P0: use the exact rebuilt artifact in history and
   an isolated Windows account. After a dedicated bot is prepared, use settings
   to verify/replace/delete credentials, restart, check presence-only projection
   and inspect YAML/SQLite/logs/diagnostics for leakage. Expected: native-only
   storage, deletion removes presence, no fallback. Record redacted evidence;
   do not send secrets in chat. Current result NOT_RUN, missing isolated account
   and real bot; synthetic CRUD does not close this row.
2. WQ-TG-008 / Windows Owner / P1: on the same artifact capture 100/125/150%
   and narrow Telegram settings/task layouts, keyboard traversal, cancel/retry
   and UNKNOWN review. Expected: separate archive/send states and explicit
   reviewed resend. Current NOT_RUN, matrix deliberately deferred; previous
   dashboard DPI screenshots/keyboard PASS do not cover the new Telegram UI.
3. WQ-TG-007 then 002–006/009 / Windows Owner: prepare api_id/api_hash through
   a secure local channel, deploy a pinned loopback Local Bot API Server and
   record version/hash before the numbered real-service procedures below.
   Expected: separate network-leg results and version-bound size/memory evidence.
   Current PLANNED/NOT_RUN; user credentials and controlled target unavailable.
4. WQ-TG-UNI-01–08 / Windows Owner: inventory the detected Unigram installation
   before receiver checks. Current NOT_RUN; an open window proves neither version
   nor reception. Scheme B remainder also needs a separate exact-source batch.

Evidence: [Windows history](windows-validation-history.md#2026-10-04--telegram-windows-local-return-f57c4b2).

共享生产发送路径已实现并接线：completion-time 配置采样、不可变 archive intent capture 与 journal 对账、原子新行授权、凭据轮换代次与授权栅栏（迁移 `0009`–`0013`）、verified bot identity、带 claim heartbeat 的 sender worker 与启动/设置重启、注册 Tauri 命令、设置与任务投影（含实时上传阶段）、显式确认且永不重试的 endpoint 迁移控制路径，以及只有迁移命令能清除的 `migration_pending` 持久闩锁。Linux 测试只证明共享模块，不构成 Windows 或真实发送验收。

剩余工作均为 Windows/真实环境执行，不是跨平台开发：Windows Credential Manager adapter、打包 Local Bot API Server 部署、GUI/键盘/DPI 验收、真实账号受控发送与 Unigram 接收端验收。归档文件系统提交与 SQLite outbox intent 的协调契约已在共享层以显式事务与幂等重放实现并测试，不再是未决项。

**本轮状态：** 2026-10-04 WSL2 Linux，PowerShell interop 可用但仅安装 Linux Rust target，且无 MSVC、packaged Windows artifact、浏览器进程、credential harness、Telegram server/bot/target 或 Unigram。未执行任何 Windows build/runtime/GUI/凭据/Local Bot API/真实 Telegram/Unigram 验收。队列 `WQ-TG-001`–`009`、`WQ-TG-UNI-01`–`08` 均保持 `NOT_RUN`；`WQ-TG-001`/`007` 的 Windows 专属实现仍为 `PLANNED`，其余共享实现为 `IMPLEMENTED`，延期原因按缺失的 Windows 能力或真实环境记录，不再记为缺少生产入口。详情见 [`windows-queue.md`](windows-queue.md) 的 TG 表。不得将 BLOCKED 或 NOT_RUN 记为 PASS。

### Windows Owner 手工步骤（前置就绪后执行）

前置：本队列中共享实现已就绪；Windows Owner 须先核对正式 Git handoff 的精确 source SHA、干净工作树、Windows build/架构/工具版本和新构建 artifact SHA-256。`WQ-TG-001`（Credential Manager adapter）与 `WQ-TG-007`（打包 Local Bot API Server 部署）仍需 Windows 实现先就绪；其余步骤的前置（生产发送命令、设置 UI、SecretStore、runtime queue/heartbeat）已具备。另需准备隔离 Windows 账户、专用测试 bot 和受控 chat/topic。Local 模式需固定版本 Local Bot API Server，仅绑定 loopback；Unigram 验收另需 Unigram 接收端。以下步骤在对应 Windows 前置具备前是验收模板，不是当前可执行的产品 PASS。

1. `WQ-TG-001`：经 Windows Credential Manager 写入、读取存在标志、替换、删除并重启；检查 YAML、SQLite、日志、诊断导出和前端事件。期望仅存凭据管理器，删除后 presence 为 false，绝无明文 fallback/泄漏。保存脱敏后的凭据条目及文件检查证据。
2. `WQ-TG-002`：测试 Cloud HTTPS，再测试 `http://127.0.0.1:<port>`；分别记录 Desktop→Local API 与 Local API→Telegram。尝试非回环明文、URL 内凭据及 3xx。期望拒绝不安全 endpoint、不跟随 redirect、本地 endpoint 直连。
3. `WQ-TG-003`：发超长文本、中英文/emoji、链接；核对拆分顺序、caption 与 metadata 边界及预览策略。保留脱敏 API 结果和目标截图。
4. `WQ-TG-004`：发 1、2、10、11 项；包括单图、相册、视频、文档；删除/改写 album 中一个文件后再发送。期望单项使用单媒体接口、album 为 2–10 项、尾部单项提升单发，且不可读项导致整册发出前拒绝。保存 API 结果、SQLite 映射、目标截图。
5. `WQ-TG-005`：在请求前、请求进行中、响应丢失后、数据库写入前分别强杀并重启；检查 `UNKNOWN` 不自动重发。制造前序永久失败/UNKNOWN，确认后续 plan order 被暂停；执行超过租约时间的上传并观察 heartbeat 与过期 claim。保存脱敏日志、outbox state/lease 与恢复记录。
6. `WQ-TG-006`：重复发照片/视频并确认 cache hit 不上传 body；文档仍上传原文件；替换 token 检查 bot 隔离；只在显式 invalid-file-id 响应后回退。保存脱敏 cache 行及服务端记录。
7. `WQ-TG-007`：按固定 server 版本测 >50 MB、接近上限与超限文件，同时采样 Desktop 内存。记录 server/version/上限来源、文件 hash、时间、内存和错误结果。
8. `WQ-TG-008`：设置/任务 UI 在 100/125/150% 与窄窗口操作，执行键盘遍历、取消、重试和 UNKNOWN 复核。期望归档与发送状态分离，只有 Bot API 确认后显示 send confirmed。保存截图/键盘记录。
9. `WQ-TG-009`：配置含凭据代理，触发 redirect 并检查配置、DB、日志、诊断及路径映射。期望无凭据/token 泄漏，loopback 直连，路径 traversal/link/escape 被拒绝。
10. `WQ-TG-UNI-01`–`08`：记录 Windows/Unigram/WebView2/GPU/HDR/磁盘环境；验证文本、caption、1/2/10/11 相册、视频、适用 HDR、下载及 SHA-256、large-file 与自动下载关闭、深链在 Unigram 未运行时的系统关联。每项单独保存截图/录屏/hash；不适用硬件写明环境并记 NOT_RUN。

每项登记准确 `source_sha`、`build_origin`、artifact SHA-256、环境/工具版本、步骤、实际结果、证据和后续责任人。真实发送/接收结果不可由 mock 或 Linux 测试替代。

### Windows/外部服务手工验证汇总（实现前置完成后执行）

本轮环境确认：WSL2 Linux x86_64，可调用 PowerShell，Windows 报告版本 `10.0.29680.0`；当前 Rust target 仅有 `x86_64-unknown-linux-gnu`，未发现 MSVC/Visual Studio 工具链、MS Edge/Chrome 进程或 Windows 构建 artifact。没有 Credential Manager 安全测试 harness、Local Bot API Server、专用 Telegram bot/target 或 Unigram 接收端。故本轮没有 Windows/native/真实服务证据；下列操作是待前置实现的手工方案。不要因为 PowerShell interop 可用就把它等同 Windows GUI/native build。

1. **精确构建与身份（`WQ-TG-001`–`009`、`WQ-TG-UNI-*` 共同前置）**：从正式 Git handoff checkout 指定 source SHA，验证 clean tree；记录 Windows build/架构、Rust/MSVC/Node/WebView2/浏览器版本；从干净输出重建目标包，记录命令、build origin、Desktop/Full artifact SHA-256。不得复用 Linux 或旧 Windows binary。预期 source SHA 与 manifest/artifact 对应；若实现 prerequisite 未满足，记录 `NOT_RUN / IMPLEMENTATION_NOT_READY`，不执行远端操作。
2. **凭据轮换与静态泄漏（`WQ-TG-001`；依赖 Credential Manager adapter 和 UI/commands）**：用隔离 Windows 用户写入 token A，检查仅有 presence flag；尝试错误 token B，验证旧凭据仍可用；用有效同 bot token 替换，重启后只确认新 token 可用；测试同 bot 自动续接及 `confirm` 授权范围；换 bot 验证旧 bot 队列/cache 不被接管；再执行删除并重启。逐一搜索配置、SQLite、logs、诊断导出、Tauri events 和前端 state，保留脱敏证据。预期无明文 token，失败更新不损坏旧 token，`UNKNOWN` 从不因轮换自动重发。
3. **Endpoint、proxy、redirect 与 redaction（`WQ-TG-002`、`009`；依赖生产 sender、secret adapter 和服务 fixture）**：在无真实账号的本地受控 fixture 上验证 Cloud HTTPS、loopback Local HTTP、拒绝非回环明文/URL userinfo/query/fragment/额外 path；从记录器触发 redirect、HTTP error、malformed body。Local endpoint 开启系统代理仍应直连；Cloud 显式代理应按设置路由。检查 token 不出现在 URL/error/log、路径 traversal/symlink escape 被拒绝。真实 Bot API 的两条路径（Desktop→Local API、Local API→Telegram）另用受控 bot 分别测，不把 mock 当 E2E。
4. **文本、媒体及 receiver（`WQ-TG-003`、`004`、`006`、`WQ-TG-UNI-02`–`06`；依赖发送入口与隔离 target）**：先确认 outbox snapshot 和原目标，再分别发送 Unicode/emoji/链接/超长文本、caption 边界、1/2/10/11 项、照片/视频/document、混合缓存 album。删除/修改一个待发媒体后确认整组在任何请求前拒绝；重复发送 photo/video 检查 cache hit 不上传 body，document 必须上传原文件，只在明确 invalid-file-id 响应后清 cache/fallback。Unigram 端分别验 display、顺序/播放、下载文件名和 SHA-256；“发送成功”不得替代接收/文件完整性结果。
5. **恢复、顺序与租约（`WQ-TG-005`；依赖 sender loop、heartbeat 和 startup recovery 实现）**：对请求前、body 上传中、服务器接受后响应丢失、本地确认写库前分别强停并重启；核对未上网 claim 回到 retry，越过 request-started fence 的 claim 变 `UNKNOWN` 且不会自动发送。令前序计划进入 permanent failure/UNKNOWN，确认后续同 archive 计划阻塞；仅用显式人工 retry/授权释放。用超出初始 lease 的慢 fixture 上传观察 heartbeat 延长有效 lease；失去 lease 的 worker 不得覆盖后继 owner 结果。
6. **大文件、取消及 Unigram 接收（`WQ-TG-007`、`WQ-TG-UNI-01`、`04`、`05`、`07`、`08`）**：固定并记录 server build/version/flags、loopback bind、上传上限依据；用 hash 记录 >50 MB、接近有效 ceiling 和超限样本，并采样 Desktop RSS、耗时和错误状态；取消传输后检查请求分类与 archive 原件保留。记录 Unigram、WebView2、GPU/driver、HDR/增强、磁盘空间和 auto-download 设置；逐个做视频播放/seek/audio、适用 HDR、手动下载/校验和深链（Unigram 未启动及已启动）。不具备特定 GPU/HDR 条件写 `NOT_APPLICABLE` 或 `NOT_RUN` 并说明硬件，不推断通过。
7. **UI/键盘/缩放（`WQ-TG-008`）**：在 Windows 原生 WebView2 用 100/125/150% 与窄窗口查看设置、任务状态，进行 Tab/Enter/Space、取消、retry、UNKNOWN 复核；确认 archive 和 Telegram 状态分离、只有 Bot API 确认才显示 send confirmed。截图记录窗口尺寸和缩放；任何 Native GUI 自动化失败单独记 `BLOCKED / COMPUTER_USE_UNAVAILABLE`，不可写成产品 FAIL。

每个手工子项单独保存 `id`、准确 source SHA、build origin、artifact SHA-256、Windows environment/tool versions、prerequisites、steps、expected、actual result、evidence、blocker/defer reason、blocks-development/release、revalidation 条件和 follow-up Owner。未有产品入口的条目维持 `NOT_RUN / IMPLEMENTATION_NOT_READY`；有入口但缺 Windows/服务/账号的条目维持 `NOT_RUN` 并写具体 `WINDOWS_EXECUTION_UNAVAILABLE` 前置。只有实际运行并有同一 artifact 的证据才可标 PASS。

## I. 需要 Owner 决定的事项

| # | 事项 | 影响 |
|---|---|---|
| 1 | ~~下一个 pre-release 的目标版本号~~ | **已决定：`v0.2.1-pre1`**，随后准备 `v0.2.1` 正式版 |
| 2 | ~~是否压缩发布范围授权~~ | **待定**：仍取决于 G4–G7 是否关闭 |
| 3 | ~~Telegram 是否写入 `v0.2.1-pre1` 发布范围（G6）~~ | **已决定：`v0.2.1-pre1` 不纳入 Telegram；该历史版本范围决定保持不变。Telegram 开发于 2026-10-03 恢复，未来版本是否纳入另行决策。** |
| 4 | 是否 fast-forward `main` | `main` 当前落后 `dev`，需在发布前决定 |
| 5 | gallery-dl / aria2 / PyInstaller 的许可证与固定版本 | 再分发义务未关闭，见 [`../references/external-sources.md`](../references/external-sources.md) |

## J. 版本目标（2026-10-01 Owner 决定）

- **首个目标：`v0.2.1-pre1`**（编号符合 [`release-policy.md`](../release/release-policy.md) 第 7 节的 `vMAJOR.MINOR.PATCH-preN`；已退役的 `pre.N` 不得复用）。
- **随后目标：`v0.2.1` 正式版。**
- 版本号需同步的位置：`package.json`、`desktop/package.json`、`extension/manifest.json`、`desktop/src-tauri/tauri.conf.json`（`Cargo.toml` 通过 `version.workspace = true` 取值）。修改属实现变更，需与本次发布一同验证。

### 历史范围决定：Telegram 暂不纳入 `v0.2.1-pre1`（2026-10-01）

Owner 于 2026-10-01 决定 Telegram 不纳入 `v0.2.1-pre1` 范围；Owner 于 2026-10-03 恢复 Telegram 开发。以下规则仅记录该历史版本，不代表当前暂停开发：

1. `M-CAND-03` / `G6`（Telegram 真实发送）**不进入 `v0.2.1-pre1` 的执行范围**，状态从 `NOT RUN` 记为「范围外」。这是范围决定，**不是**通过，也不是失败。
2. 发布说明**不得宣传** Telegram 能力已实现。`v0.2.0` 已按此执行，`v0.2.1-pre1` 沿用同一口径。
3. 已实现的 Telegram 代码**保留不删**（契约、幂等发送、持久化）；2026-10-03 起继续推进生产接线。
4. 根 `README.md` 当前说明 Telegram 已恢复开发但仍未接线/验收；历史发布说明保持原样。

## L. 图标矩阵已关闭（WQ-ICON-030-06，2026-10-02）

被测 artifact 为 Owner 指定的 `E:\Shiraishi\Downloads\Compressed\XArchive-v0.2.1-pre1-windows-x64-full\xarchive-desktop.exe`（此前核验 SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`；本次路径已不可读，沿用已建立身份）。

Owner 已补充默认 200% 开发环境的确认，并要求复用此前截图。因此同一已识别 artifact 的 100%、125%、150%、200% 标题栏／资源管理器／任务栏／Alt+Tab 图标矩阵记为 PASS，无需补拍。200% 后三处依据 Owner 人工确认，不能描述为截图直接展示或 Agent 自动化通过；前三档原图标注不变。通知区域托盘不适用（未实现）。证据身份与复用限制见 windows-validation-history 最新 closure 节。

Computer Use 已于 2026-10-02 完成 WQ-UI-030-03/04 子项：Bootstrap 顶部分隔线、实际区块顺序、底部 Tab 顺序及焦点、761px 窄窗口顶部／底部布局 PASS。底部已测：最大日志文件数 → 保存日志设置 → 代理模式 → 保存代理设置 → 检测当前路由；没有触发保存／检测动作。完整遍历与其他缩放档仍 NOT_RUN，不重复要求已测子项。

Owner 后续确认当前 200% 设置页完整键盘遍历手动 PASS，该子项无需再测。中间区块窄窗口与其他缩放档仍未完整覆盖；局部结果不能关闭完整设置页矩阵。剩余操作已合并到 §M。**本轮 token 出现在工具中间输出，需 Owner 重新轮换**；此前轮换记录保持历史事实。窗口模式启动按钮文字换行另交 Cross-platform Owner 评审。

## M. 当前剩余人工操作汇总（2026-10-02，键盘确认之后）

已完成：v0.2.1-pre1 Full 四档原生图标矩阵；200% 当前设置页完整键盘遍历（Owner 手工 PASS）；正常／761px 窄窗口顶部和底部设置布局子项。无需重做。沿用已核验 Full EXE，具体身份见 Windows history。以下仍未通过完整验收：

| 顺序 | 手动操作 | 判断与前置 |
|---|---|---|
| 1 | Extension 页面配对与连接状态检查 | Owner 说明 token 为一次性验证并自动轮换，无需手动替换；自动轮换本身未由 Agent 验证。首次双端 WebSocket 认证已 PASS；Desktop 重启重置 token（Owner 明确的预期行为），后续归档前需用当前 token 重新配对。旧 token 自动重连不是本产物验收要求。 |
| 2 | 设置／日志／Extension 在 100/125/150% 与窄窗口检查；工作台用已有任务、运行中和停止状态检查操作间距 | 检查标签／路径／按钮无重叠裁切，Extension 按钮等高换行正常，日志自动跟随样式正常；其他档键盘覆盖不由 200% PASS 推导。运行中任务需受控 fixture。 |
| 3 | 在隔离配置下改归档目录，取消选择、重启恢复；测试中文／空格、不可写目录、已存在文件；切换后归档 | 新归档去向正确，取消／拒绝不改变原配置；已有文件、DB、日志不迁移；运行中任务与浏览器连接行为可恢复（WQ-SET-020-04..06）。 |
| 4 | 日志等级改 error/silent，确认即时生效并重启保持；检查旧配置升级、真实子进程模块日志 | 级别不被渠道默认覆盖。稳定 release 默认 info 须另用明确的 release 产物；本 pre-release 不能证明该项（WQ-LOGS-020-02..04）。 |
| 5 | Native Host 安装／修复、扩展加载与配对、浏览器／Desktop 重启重连；专用账号完成一次归档并重复提交 | 历史安装 FAIL 仍需 Windows 修复／复验；确认终态完成、媒体/SQLite 一致、无重复。需隔离 profile、受控账号与已修复安装路径；打开 popup 不等于成功（WQ-P1-02、M-CAND-02）。 |
| 6 | 受控长任务中取消／关闭应用、异常退出后重启；检查 worker/gallery-dl/aria2 子进程、staging 与文件锁 | 取消／中断状态正确，无孤儿进程和重复提交。先由 Windows 准备当前协议 fixture；不能照抄旧 download/fallback recipe，也不能手动杀子进程后宣称产品清理 PASS。 |
| 7 | 隔离环境 Direct/Manual 代理、中文／长路径／junction 与 Named Pipe 权限／重连 | 需可复现本地代理/路径/权限 harness，Windows 负责准备与证据；非单纯 GUI 点击。代理凭据不进日志/DB/命令行，本地 RPC/Extension 保持直连。Telegram 分支暂缓。 |

暂不执行：Telegram/Unigram 真实发送（运行时/UI/凭据未接线）、系统代理 Batch B 的 registry/PAC/WPAD/bypass（未实现）。构建、Rust/Node/Python 测试、哈希和文档检查由 Agent 执行，不列为用户手动操作。旧 H/H2/G 配方含历史 revision/契约，须先按当前产物和代码校正，不直接视为本轮命令。


2026-10-02 follow-up：便携目录创建/重启保持、原生选择器取消保留路径、Error/Silent 保存与重启保持、当前正常/窄窗口日志样式及设置中间区块观察子项已完成。原日志等级 Debug 已恢复。上述局部 PASS 不关闭整行验收；真实归档、路径拒绝、子进程日志、其它缩放与连接仍待验证。Owner 最新说明取代 §L 的人工 token 轮换要求，无需手动替换。扩展管理页被浏览器 URL 策略阻止，Owner 已手动加载；配对等待截图，测试暂停该项并继续独立项目。


2026-10-02 pairing follow-up：Owner 完成 WebSocket 配置，扩展与 Desktop 双端已认证子项 PASS；正常重启后旧 token 无法重连符合 Owner 明确预期。第 5 行重启重连按当前 token 重新配对验证，不要求旧 token 自动恢复。当前等重新配对与专用测试帖子 URL；真实归档/重复提交、Native Messaging、浏览器重启尚未执行。


当前接续（2026-10-02）：扩展设置页刷新后仍已认证，本机 17321 有实际连接；Desktop 旧计数 disconnected 需显式点 Extension 区块刷新再判定。页面切换不刷新该状态，不重复要求换 token。等当前双端结果后再执行真实帖子归档与重复提交。

最新显式刷新仍 disconnected（成功 6 / 认证后关闭 6）：不再仅按旧状态解释。先选择是否执行同一会话重新连接后约 5s/45s 状态读数，不打开 worker DevTools、不重启 Desktop；或交 Cross-platform 排查，或继续其它独立验证。真实归档保持暂停。

最新更正：Owner 点击“保存并连接”后观察到认证失败，输入新 token 后两端均已认证，Desktop connected；当前 token 重新配对子项手动 PASS，无需重复配对。此前累计关闭计数不能单独判定持续性缺陷，受控 5s/45s 空闲稳定性测试仍 NOT_RUN。下一项为提供允许下载的公开测试帖子 URL，然后执行真实归档与重复提交；此项尚未执行。浏览器重启、Native Messaging 和其它整体验收仍未关闭。

当前暂停：随后显式刷新 Desktop 又为未认证/disconnected，认证后关闭 9 → 10，其它计数不变；Extension 仍已认证。配对成功与后续关闭分别记录，不能宣称连接已稳定，也不能断言刷新导致关闭。Owner 选择暂停连接测试并交 Cross-platform Owner 排查；不再要求当前人工重连或归档。已授权测试帖子 https://x.com/thsottiaux/status/2105039482013757749 ，待调查/后续 handoff 后用于真实归档及重复提交（NOT_RUN）。受控空闲计时、浏览器重启与 Native Messaging 仍 NOT_RUN。

跨平台处置（2026-10-02，Cross-platform batch）：上述状态不一致已定位到两处**共享状态上报缺陷**，并在 Linux 上修复并测试：Extension 的 `getStatus()` 原本从缓存标志（而非实时 socket）回答"已认证"；Desktop 的 `browser_connection()` 原本在最后一次请求后 30 秒内即使没有任何打开的 socket 也回答"connected"。两处现均以实时 socket 为准，Extension 在状态读取时会自愈被静默关闭的连接。**未确定**的部分仍然是：认证后关闭的累积原因、MV3 worker 生命周期是否参与、是否存在不安全的空闲时长——这些只能在 Windows 上观测，不得由本次修复推断为已解决。

复验要点（下一 Windows 批次，绑定该 batch commit）：建立一次已认证连接且不重启 Desktop，刷新 Extension options 页与 Desktop Extension 区块，比较两端同一时刻的状态；Extension 不得在 Desktop 无打开 socket 时显示"已认证"。Desktop Extension 诊断新增 `last_request_age_seconds`（未收到请求前为 `null`），用于区分"从未使用过连接"与"使用后转为静默"，不要再用累计计数推断。受控空闲计时仍按上一条执行（~5s / ~45s），只记录观测，不归因未复现的原因。

### b015fbe 本轮实际入口与待补人工证据

Windows 已 Git 对齐 `b015fbe81a0b47c2b486a5256bd81ac95fd98d25`，构建并启动隔离产物 `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-batch-b015fbe\app\xarchive-desktop.exe`（SHA-256 `B3550F5CEB6764C2D8DD1E06B94D1E143B936B7B28ACA318F8549EDF1677A1F2`）。Edge 使用 Owner 已手动加载的 checkout `extension`；配对已 PASS。旧 Downloads EXE 不用于此修复的复验。新应用旁 download 已初始化；下载组件复用旧 Full，只用于本机集成，不代表新 Full 包通过。

1. WQ-WS-03/P0：本轮配对后连接关闭子项 FAIL；先刷新 Desktop 未认证、再刷新 options 已认证、再只刷新 Desktop 仍 disconnected（接受 8/成功 2/认证后关闭 2）。不要求反复重新配对；已暂停并交 Cross-platform 排查。新 handoff 后再不保存/不重启地执行受控 ~5s/~45s 场景，并记录每次读数顺序、时间及状态读取可能触发的自愈。本轮实际人工取证延迟超过 45 秒，完整定时场景仍 NOT_RUN，不能宣称确定的关闭超时或同时刻状态不一致。
2. `last_request_age_seconds`：目前 JSX 未显示，NOT_RUN / DIAGNOSTIC_UI_NOT_IMPLEMENTED；交 Cross-platform Owner 补支持的读数方式。不要反复寻找 UI 中不存在的字段，也不要求提供 token。
3. 连接问题经调查/新 handoff 后，使用 https://x.com/thsottiaux/status/2105039482013757749 仅提交主帖归档；核对完成状态、真实媒体及 SQLite，再重复同帖验证幂等。当前 NOT_RUN / 暂停连接问题后的依赖验证；页面可读且注入保存按钮只证明渲染，未点击提交。浏览器/worker重启、pending cleanup 和 Native Messaging 仍未验收。

证据须绑定上述 EXE/Extension revision、操作时间、手动或自动方法与状态读数；凭据不进文档/截图。相关自动测试子项已 PASS，不能据此关闭 WQ-WS-02/03 全行。Windows 负责此人工/原生续验，Cross-platform 负责诊断显示和既定 Telegram 接线。

## P. Current Full validation directory — 2026-10-05

Fresh ca25e53 dev Full directory and current executable manual Extension/Telegram recipe: [Full validation steps](windows-full-ca25e53-manual.md). Package completeness/component startup PASS scoped; automatic pairing, real archive and Telegram acceptance NOT_RUN until executed. Do not reuse old package or manual-token evidence as automatic-pairing acceptance.

## M. System proxy Batch B (Windows) — manual steps

These steps cover `WQ-PROXY-01` through `WQ-PROXY-11`. All are
`WINDOWS_VERIFICATION_PENDING`. Run them from a build of the exact handoff
commit, and record the commit, the executable SHA-256, the Windows build number,
and the DPI for each result. Use a controlled PAC file and a local SOCKS
listener; never use a production corporate PAC.

**M0. Build and resolver wiring.** Build the Windows artifact and confirm it
starts. If the build fails inside `system_proxy_resolver.rs`, capture the exact
error before anything else: that module has never been compiled. Confirm the
resolved `os-proxy-resolver` commit is `796b027c9361bb407f2a8d9d79c56b2dc4a42ee2`.

**M1. Static system proxy and bypass.** In Windows Settings, set a manual proxy
and a bypass list that includes `localhost`. In XArchive Settings, expand
"网络代理", select "使用系统代理", and open "当前系统代理配置". The reported
resolver must be "Windows 系统代理（静态 / PAC / WPAD）", the static entries and
bypass list must be shown redacted, and a request to a bypassed host must go
direct while another host uses the proxy.

**M2. PAC per-URL routing.** Serve a PAC file that returns `PROXY` for one host
and `DIRECT` for another, and point the system auto-configuration URL at it. Use
"路由检测地址" to resolve both hosts. The two results must differ, the displayed
source must be PAC, and an ordered list must appear when the script returns more
than one candidate.

**M3. Ordered candidates and SOCKS.** Use a PAC returning
`PROXY a:8080; SOCKS b:1080; DIRECT`. All three must be listed in that order. Run
a request that uses the SOCKS entry and confirm it reaches the SOCKS listener, not
an HTTP CONNECT.

**M4. WPAD states.** With auto-detection on, confirm the reported PAC state
distinguishes a discovered script, "not found", and a discovery failure. A
failure must be visible as a state, not silently presented as "no proxy".

**M5. Environment versus OS precedence.** Export `HTTPS_PROXY` in the launching
shell *and* configure an OS proxy. The reported source must match the official
resolver's documented precedence, and the summary must show that the environment
value is in play. This precedence is not configurable in this batch.

**M6. Configuration change.** With the application running, change the system
proxy. The next request must use the new route; the configuration revision shown
in the summary must change. A stale route after a change is a defect.

**M7. Transport coverage.** Run a real gallery-dl extraction and a real media
download, then an aria2 download, with a proxy in place. Confirm the actual
outbound route for the entry host and for the media CDN. If a backend silently
ignores the resolved route, record the backend and the host.

**M8. Telegram.** Verify Telegram Cloud uses the resolved route and Telegram
Local stays direct. Then configure a PAC that fails for the API host and confirm
the send is **refused** with a message rather than silently going direct.

**M9. Credential sweep.** After a proxied run, search the application log
directory, the SQLite database, the settings UI, and the gallery-dl and aria2
child command lines for the proxy password, any PAC URL credential, and the Bot
Token. None may appear.

**M10. Settings UI.** Verify the system summary disclosure, the ordered candidate
list, and the route-diagnosis field at 100%, 125%, 150%, and 200% DPI and at a
narrow window width: keyboard reachable, no clipping, and a long PAC URL wraps.

**M11. Local traffic stays direct.** Confirm the local aria2 RPC, the browser
Extension connection, and Telegram Local are unaffected by any system proxy
setting.

**M12. Child-process coverage guard.** With a controlled PAC active and the
mode set to "使用系统代理", submit an archive job. It must be **refused** with
`PROXY_POLICY_NOT_APPLICABLE`, the settings page must show the child-coverage
explanation, and gallery-dl must not have been started. Then switch to a static
system proxy and confirm the job starts and honours the proxy. Repeat for
"直连" and "手动代理" to confirm neither is affected.

## 6cf4cc8 continuation prerequisites — 2026-10-05

Use [batch manual continuation](windows-6cf4cc8-results.md#manual-windows-continuation) for the current IDs, states, artifact prerequisites and expected evidence. M recipes must use the new reviewed handoff and controlled isolated PAC/WPAD environment; WQ-PROXY-08 per-URL child wording is pending reconciliation with M12 refusal. Download completion and real backend/Cancel/junction/restart acceptance need a fresh Full package. Current independent worker/GUI render checks do not close these manual items.

**M13. PAC failure must not become a direct route.** Configure a PAC URL that
cannot be downloaded (an unreachable host, or a script that throws). Confirm the
route diagnostic and any outbound attempt **fail with a reason naming the PAC
state**, and that no request leaves the machine unproxied. Then configure a
working PAC that itself returns `DIRECT` for some host and confirm that host
still routes direct — the refusal must apply only to a policy that failed.

**M14. Protocol fixtures on native Windows.** Rerun `cargo test -p xarchive-protocol
--locked` and the Python protocol module. Both must pass, and a relative
`staging_dir` must still be rejected.

**M15. Bypass wildcard parity.** With a controlled bypass list, confirm that
`*.example.test` excludes a subdomain but not the apex host, and that
`.example.test` excludes both. Before this, the `*.` spelling matched nothing.

## Windows 130affaf continuation

Use [batch manual queue](windows-130affaf-results.md#manual-queue-and-deferred-checks) for current artifact identities and prerequisites. P6 initial Enter FAIL is repaired but remaining navigation/DPI/busy/focus and false-state restart must use the reviewed final source. Settings UI automation was blocked by two activation failures; manual Owner can complete P6/M10. M13 must verify actual failed-PAC egress after adapter configured-policy/state mapping review, not assume the shared helper proves it. M14 shared JSONL fixture was repaired on Windows; original FAIL remains in history. WQ-DL full/real-download recipes still require a fresh Full package and authorized fixture.
