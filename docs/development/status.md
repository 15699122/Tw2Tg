# 当前开发状态

> 本文只维护当前实现能力矩阵，不维护批次状态、验证记录或历史执行流水账。
>
> 当前实现/限制以本矩阵为准；当前批次、branch、source revision、Owner 以 [`../status/platform-handoff.md`](../status/platform-handoff.md) 为准；验证结果与证据以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 及 [`../validation/windows-validation-history.md`](../validation/windows-validation-history.md) 为准；计划阶段与需求以各自 Plan 为准。发布范围以对应版本 Release Notes 为准。
>
> 本文不替代逐轮验证报告，也不重复历史验证流水账的结论。

## 当前实现能力矩阵

每行只陈述能力本身与它的验证边界，不含测试计数。实现状态与验证结果是两件事：`IMPLEMENTED` 只说明代码存在，不代表任何平台验收已通过。

| 能力 | 实现状态 | 平台实现 | 验证边界 |
|---|---|---|---|
| Job 状态机与幂等 | `IMPLEMENTED` | 跨平台 | 单元与契约测试已覆盖；真实中断/恢复时序待 Windows 验证 |
| Sidecar protocol v2 | `IMPLEMENTED` | 跨平台 + Windows 打包 | Linux 与打包 worker 已验证；Windows Job Object 时序待验证 |
| extraction-only → aria2-only 传输 | `IMPLEMENTED` | 跨平台 | fake RPC 与错误分类已验证；真实 signed URL 与 aria2c.exe 行为待验证 |
| WebSocket 本地通道 | `IMPLEMENTED` | 跨平台 | loopback 与认证已验证；连接状态一致性缺陷（Extension 缓存标志、Desktop 30 秒宽限）已在 Linux 修复并有回归测试；Windows 连接持续性、受控空闲计时与实机配对待验证 |
| Native Messaging 回退 | `IMPLEMENTED` | 跨平台 + Windows 注册 | framing 已验证；Registry/ACL/浏览器加载待验证 |
| Extension 自动配对（Native bootstrap + 单次 ticket + WebSocket） | 共享与 Windows Named Pipe 接线 `IMPLEMENTED`；集成源码 Windows 验收 `PENDING` | 共享 Desktop/Host framing/Extension + Unix IPC + Windows Named Pipe；集成实现见当前 handoff | 共享 targeted 测试通过；旧 Windows 证据只绑定其原始 source/artifact，不能代表集成源码；集成源码 Windows 原生、注册与浏览器验收待执行，见 [计划](browser-automatic-pairing-plan.md) 与 [队列](../validation/windows-queue.md) |
| Core / Full 打包与 manifest | `IMPLEMENTED` | 跨平台 + Windows 构建 | 脚本契约已验证；真实 Windows 资产组装与启动形态待验证 |
| 配置持久化（含归档目录） | `IMPLEMENTED` | 跨平台 + Windows 已知目录 | 迁移与重启逻辑已验证；Windows known folder 实际解析待验证 |
| 日志渠道策略与图标 | `IMPLEMENTED` | 跨平台 + Windows 渲染 | 渠道解析与样式断言已验证；缩放与真实 pre-release 二进制输出待验证 |
| 三态代理模式 | `IMPLEMENTED`（Batch A） | Batch A 跨平台；Batch B 未实现 | `Direct`/`Manual` 可验证；`System` 下的 registry/PAC/WPAD 按 URL 解析为 `PLANNED`，Batch B 前不进入实机执行 |
| Telegram 发送 | 共享 contract、storage、Desktop send helper `IMPLEMENTED`；TG-06 生产接线恢复开发中 | 跨平台继续实现归档入队、Tauri commands、claim-loop 调度与任务投影；Windows Credential Manager / GUI / 部署由 Windows Owner 推进或验收 | 真实发送无平台验收证据；`WQ-TG-*` 仍为 `NOT_RUN`，缺少执行入口的项目随接线推进复核，不代表通过。Telegram 目前未被声明为已发布/已验收能力。见 [Telegram Plan](telegram-local-bot-api-plan.md)、[queue](../validation/windows-queue.md) 与 [manual steps §K](../validation/windows-manual-steps.md#k-telegram-发送wq-tg-batch-a-共享层已就绪当前全部跳过) |
| 账号/主页批量归档 | `IMPLEMENTED` | 跨平台 + Windows GUI | 契约与队列语义已验证；真实账号多页与 GUI 验收待验证 |

## 历史开发与验证记录

以下原有的逐轮开发流水账保留作历史证据；其分支、revision、计数、Owner 与完成度仅适用于各自记录时点，不用于描述当前状态。当前能力只看上方矩阵，当前批次只看 [platform handoff](../status/platform-handoff.md)，当前待验收项只看 [Windows queue](../validation/windows-queue.md)。

- 原 WebSocket/Extension GUI Plan 的设计决策与逐轮 Linux/Windows 证据继续保留在下方历史记录和对应验证历史中；当前集成状态及精确 source 见 handoff 和自动配对 Plan。
- **已知 Linux scope 限制：**当前第一版配对仍由用户在 Desktop 设置复制端口/token 到 Extension options；没有实现独立的自动端口发现文件或凭据轮换向导。该限制不阻塞当前 WebSocket listener、认证、bridge、GUI 和发布清单的 Linux 验证，但必须在 Windows queue 中作为人工配对/恢复步骤验证；后续如实现 discovery/rotation，应另行更新 ADR 和协议契约。

- **Plan 来源：**对照 `hureyqi/x-spider-mod-2026`（基线 `4fd46b6`）的评估已获批准，P1/P2/P3 与「指定账号批量下载」已写入 [`roadmap.md`](roadmap.md) 的 `2026-09-23` 章节。
- **非 Windows 实现已完成：**P1-A 提取契约贯通；P1-C 临时作者身份升级；P1-D README 状态归一；P2-A 统一网络配置与日志/前端诊断脱敏；P2-B 批次暂停/继续/取消/重试和队列背压语义；P2-C 媒体数量/大小/可选 SHA-256 完整性判定；P3-A/B/C/D 的协议、Sidecar 发现、SQLite 批次/候选持久化、有界派发、Desktop 命令和账号归档 UI。
- **本批次入口：**`desktop/src/pages/batches-page.jsx` 已接入侧栏“账号归档”，支持账号/主页、日期、数量、是否包含转帖/无媒体、刷新、暂停、继续、取消、失败重试；不显示发现未结束时的虚假百分比。
- **本轮共享修复（输入 `553e378`）：**补齐 executor replacement 时 Unix transport 的 stop/restart 代际切换，避免旧 `ExecutorApplicationService` 被 Browser/Native Host 持有；`executor_config` 统一复用 runtime 构造，保留 portable gallery-dl、网络参数和 discovery timeout。
- **共享 ownership 结论：**本轮输入 `553e378` 的 `EXECUTOR_UNAVAILABLE` 与账号发现 0 候选已由 Cross-platform Owner 处理；当前无 `CROSS_PLATFORM_CHANGE_REQUIRED`、无 `CROSS_PLATFORM_REVIEW_REQUIRED`、无 `WINDOWS_BLOCKING`。Windows Extension/Native Host 应用重启后重连仍归 Windows Platform Owner。
- **gallery-dl 共享适配：**Sidecar 使用官方 `--dump-json` + `output.jsonl=true` 输出，按 `Message.Directory=2` 与 `Message.Url=3` 聚合；支持真实 author dict、`content`、`reply_id` 和媒体 URL，保留 info.json 兼容 fallback。真实 gallery-dl artifact/账号验收仍未运行。
- **仍非本 Linux 代码闭环可完成：**P1-B 的脱敏真实 gallery-dl 输出样本验收、P3-E 受控真实 X 账号多页/认证/SHA-256 验收，以及所有 Windows 原生/浏览器/GUI/打包项目。它们分别需要真实 gallery-dl/X 凭据或 Windows 目标环境，不能由 synthetic fixture、Linux build 或本地 mock 替代。
- **本轮修复与验证（Linux，WebSocket follow-up）：**修复认证响应缺失导致 Extension `ready` 永久 pending；`WEBSOCKET_AUTH_TIMEOUT` 独立为 `auth_timeout` 状态且不自动重试；增加真实 loopback 正确 token 认证后 `query_status` 请求测试；Desktop `get_extension_status` 暴露不含 token 的 accepted/auth_received/auth_succeeded/auth_failed/auth_response_failed/close_before_auth/close_after_auth 计数，并显示在设置页；popup/options 增加“认证超时”文案。Linux targeted：Desktop WebSocket tests `4/4` PASS，`cargo clippy -p xarchive-desktop --all-targets --offline -- -D warnings` PASS，Desktop `cargo check` PASS，Extension Node `26/26` PASS，Extension check PASS，Desktop Node `93/93` PASS。
- **CodeQL 告警修复（GitHub Security code-scanning/1，Linux）：**`extension/src/content-core.js` 的 `canonicalTweetUrl` 曾在未解析的 URL 字符串上用 `includes("twitter.com")` 判定来源，再拼装可信的 `x.com`/`twitter.com` 规范 URL，触发 CodeQL `js/incomplete-url-substring-sanitization`（security severity high）。这不等于可直接跳转到恶意域名的漏洞，但会让 `https://evil.example/twitter.com/status/123` 之类地址被误判为可信 X 链接，造成 Tweet 身份与来源伪造。现改为先用 `new URL` 解析（含相对链接按 `document.baseURI` 解析），再对 `hostname` 做 `x.com`/`twitter.com` 精确允许列表，拒绝 userinfo、端口、非 https 与非本机 base，并只从 `pathname` 提取数字 ID；无有效 host 时返回 `null` 而非默认改写为 `x.com`。该规则与 `xarchive-protocol::extract_tweet_id` 的整 authority 比较一致，避免 Extension 生成 Desktop 会拒绝的 URL。Extension tests 新增 7 项恶意 host/协议/userinfo/端口/畸形路径/相对链接回归；已验证回退到字符串包含判断后其中 4 项立即失败。Linux 验证：Extension `32/32`、Desktop Node `93/93`、`xarchive-protocol` `19/19`、`npm run check` PASS。CodeQL 告警是否关闭需以推送后的新扫描结果为准，测试通过不等于告警关闭；真实 X DOM 与 Edge/Chrome 行为仍需 Windows 验证。
- **本轮根因修复（Linux，Windows `close_before_auth` 证据驱动）：**Windows 2026-09-25 证据显示 Desktop 计数为 `accepted=100, auth_received=0, close_before_auth=99`，握手成功但认证帧从未被读取，且连接在 2–52 ms 内结束（远短于 5 秒认证超时）。根因是共享 listener 为轮询 stop 标志而设置 `set_nonblocking(true)`，但从未把 accept 得到的 stream 恢复为阻塞模式：POSIX `accept` 不继承 `O_NONBLOCK`，Winsock 会继承，导致 `accept()` 后的握手/认证读取立即返回 `WouldBlock` 并被当作认证前关闭。修复为在 `handle_websocket_connection` 中显式 `set_nonblocking(false)` 后再设置读超时；新增 `handshake_failed` 与 `auth_read_failed` 计数以区分「握手失败」「认证帧未到达」「收到但非文本帧」。新增回归测试 `authenticates_when_the_accepted_stream_starts_non_blocking` 显式模拟 Winsock 继承行为并延迟 50 ms 发送认证帧；已验证回退修复后该测试立即失败（`HandshakeIncomplete`），恢复后通过，证明测试有效且根因判定正确。Linux 验证：Desktop Rust `111/111`、WebSocket targeted `5/5`、strict Clippy PASS、fmt PASS、Desktop Node `93/93`、Vite build PASS。Windows 仍需在该精确 handoff 上复验配对与请求链路。
- **Windows 状态：**本批次已按 `READY_FOR_WINDOWS` 收口，正式 handoff revision 以包含本批变更的文档提交为准；所有受影响 Windows runtime 项目统一标记 `REVALIDATION_REQUIRED` 或保留原 `FAIL`/`NOT RUN` 历史，不得将 Linux PASS 推导为 Windows PASS。手工步骤见 [`../validation/windows-queue.md`](../validation/windows-queue.md)。

## P0：Windows Desktop 启动与发布门禁（2026-09-21，Windows session blocker）

- **当前事实：**`v0.2.0-pre.8` GitHub Actions run `35593193897` 已完成 Windows Rust/build/Native Host/worker/外部依赖步骤，但最终 executable UI readiness gate 在创建 WebDriver session 时失败：`session not created: DevToolsActivePort file doesn't exist`。该 run 未进入 Dashboard spec，未生成或上传任何 release asset。
- **当前判定：**`extensionBusy` 和 startup marker 两个 Linux 项目问题已修复；当前 Windows blocker 属于 WebView2/Edge driver/tauri-driver native session 环境，不能据此重新归因于前端渲染，也不能标记 UI readiness PASS。`v0.2.0-pre.8` Release 当前为 pre-release、资产为空。
- **当前实现状态（LINUX_VERIFIED / Windows revalidation pending）：**`Sidebar` 显式解构 `extensionBusy`；`initial_ipc_started`/`initial_ipc_settled` 现在只作为 frontend diagnostic events 记录，不再覆盖最终 `data-xarchive-startup=react_mount_completed` marker；新增 startup contract test；Windows release gate 已增加直接 executable preflight、版本/进程/端口快照和失败诊断目录收集；WDIO 默认关闭 driver 自动安装/下载，release workflow 固定 `tauri-driver 2.1.0-alpha.0` 与 `msedgedriver 152.0.4191.66` 并记录 SHA-256；preflight 现同时记录 `MSEdgeDriver`/`Microsoft Edge WebDriver` banner 兼容性诊断。**2026-09-22 d3fd814 Windows 验证结果：**EdgeDriver 152/Edge 154 不兼容导致 session 创建失败；Edge 154 诊断确认 blank document 是 Windows native Tauri startup/target-attachment 失败而非 driver 问题；隔离检查因机器级 msedgewebview2 状态变化失败；WDIO 日志目录契约失败（服务写到 desktop/logs）。8个验证项目状态已更新至 `windows-queue.md` 和 `windows-validation.md`，需后续在 Windows 环境完成剩余验证。**2026-09-22 测试基础设施修复（LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING）：**依赖源码核实确认 04C 根因（`snapshotSessionStart` 未被 spec 调用）与 04B 根因（service 日志捕获读取 WDIO config `outputDir`，未设置时落到 `desktop/logs`）；已接入 session-start 快照、显式 `outputDir: logDir`、gate 非空日志完整性检查，并修复 01D 隔离检查（应用/driver 残留 FAIL，msedgewebview2 后台活动降级为诊断信息）。desktop 单元测试现为 91/91；WQ-P0-WHITE-01D/04B/04C 标记为 `WINDOWS_VERIFICATION_PENDING`，01A blank-target 问题仍待 Windows 端受控 WebView2/EdgeDriver runtime 对齐后诊断。**2026-09-22 Windows revalidation（22:25–22:30，WINDOWS_VERIFIED）：**本地 workflow 等价 gate 验证 01D（隔离检查）、04B（2 个非空 WDIO 日志 2,718,192/2,560 bytes）、04C（`startup/session-start.json` 记录 1 handle、`data:,`、capabilities）均 PASS，上述三项 PENDING 已关闭；01A 仍为 `WINDOWS_FAIL`（全程 `data:,`/`ONLY_BLANK_DOCUMENTS`）。**01A runtime-pairing 调查（LINUX_VERIFIED）：**依赖源码证实 msedgedriver 应匹配实际渲染引擎 WebView2 Runtime 153.0.4234.48 而非 Edge 浏览器 154；已登记 `WQ-P0-WHITE-05A` runtime-pairing 实验（`WINDOWS_VERIFICATION_PENDING`），通过后仍需回 pinned 工具链（01B）与 hosted 确认（01C）。
- **当前 Windows 事实（2026-09-21, post-banner-fix ccaa649）：**Windows 同步源码通过 Node `72/72`、Extension `21/21`、Rust fmt/check/workspace tests/strict Clippy、Sidecar pytest `21/21`、普通/WDIO-E2E release builds、Native Host release build、Full package 本地组装与直接 executable 10 秒 preflight；`@wdio/tauri-service@1.4.0` banner 解析补丁已通过 `desktop/scripts/patch-wdio-tauri-service.mjs` 幂等应用到 `node_modules`。WQ-P1-16/WQ-P1-17 仍为 `WINDOWS_BLOCKED`，待 Windows 重新以干净 `npm ci` + 未改动 `node_modules` 运行 ordinary `1/1` 与 advanced `2/2` 验证后才能关闭；Linux 端 Desktop Node 现为 `78/78`。Windows 现场已证明接受当前 banner 后 ordinary `1/1`、advanced `2/2` 且普通/WDIO-feature 两个二进制均原生渲染 Dashboard。
- **下一步：**保留 `v0.2.0-pre.8` tag/source 不变，先稳定 Windows WebView2/Edge driver/tauri-driver session 条件，再针对同一 tag 重跑 readiness gate；只有 gate 通过后才允许生成和上传 release assets。
- **完成标准：**最终待发布的普通 `.exe` 和 Full bundle 在 Windows 实际显示 React Dashboard；失败时不得出现无提示纯白屏；资源/入口/React mount/IPC 阶段可追踪；上传前对同一最终 artifact 执行 UI readiness smoke；所有 Linux 适用验证和 Windows 结果写回文档后，才可关闭本问题。

## 当前架构状态（2026-09-20）

- **当前 Git 事实（2026-09-20）：**当前分支为 `feature/u7-desktop-production-integration`，与 `origin/feature/u7-desktop-production-integration` 同步；`v0.2.0-pre.5` tag 保持指向 `ea2b8d3afb289239edec29e2e00620870bed2fe6`（对应 Windows run `35507188780` FAILED，无资产；GitHub Release 正文已后续更新为中文，不改变 tag 构建源代码）；`v0.2.0-pre.6` tag 指向 `ac586e609337947aeb51de8f5cce3185efc8995e`（对应 Windows run `35518801950` FAILED，无资产）。本轮工作树仅包含 `v0.2.0-pre.5` Release Notes 完整中文化和状态/验证文档更新。此前记录的 `bd3e58d` dirty-source 快照属于历史验证上下文，不代表当前 source；Linux source 仍是唯一事实来源。
- **Extension 当前事实（LINUX_VERIFIED / Windows pending）：**`extension/` 已实现 MV3 manifest、X/Twitter content script、Tweet/quote/reply DOM extraction、MutationObserver、去重按钮、`archive_request`/`query_status` 请求构造和 NativeBridge request_id 路由、timeout、structured error 与 generation fencing；E4 已加入页面状态映射、初始/增量 `query_status`、100-ID 分批、`archive_status_batch` 消费和 DOM-only 按钮状态机。现有 Node tests 21/21 覆盖 DOM identity、mutation filtering、状态批处理/映射、disconnect、runtime.lastError、timeout、duplicate ID、postMessage failure 和 reconnect；真实页面/桌面状态同步、Windows Named Pipe server、Native Host Registry lifecycle 或真实 Edge/Chrome acceptance 仍未完成。
- **U17 E4 页面状态同步（LINUX_VERIFIED；Windows pending）：**`content.js` 对可见 Tweet 初始扫描和 MutationObserver 增量 article 去重查询状态；`content-core.js` 将 Desktop Job state 映射为 `idle/checking/submitting/queued/running/complete/auth_required/failed/disconnected`，按钮状态不写入 `storage`，不持久化 Cookie、signed URL、本地路径或媒体数据。Linux Extension check/test/build 通过，真实 browser → Desktop 状态更新仍在 Windows queue。
- **Extension 后续 Plan（PLANNED）：**E0–E9 的依赖顺序、完成标准和验证边界见 [`roadmap.md`](roadmap.md)「U17：Browser Extension production hardening」。当前最先执行 E0 文档/事实对账，随后才修改 Browser schema、DOM identity 或 NativeBridge 行为。
- **U17 E1 Browser protocol/schema（LINUX_VERIFIED；Windows pending）：**`query_status` 现在返回 `archive_status_batch`，按请求顺序为每个 Tweet ID 返回 Job 状态或 `NOT_ARCHIVED`；Browser Rust request/response/tweet models 拒绝未知字段；`browser-response.schema.json`、Browser fixture、Native Host response validation、Desktop transport 和 Extension response recognition 已同步。Linux targeted verification：protocol 16/16、Native Host 8/8、Desktop Rust 86/86、Extension 10/10、fmt/check/diff hygiene 通过。Windows packaged protocol probe、真实 browser/Native Host transport 仍在 U17 queue。
- **U17 E2 DOM identity（LINUX_VERIFIED；Windows pending）：**`extension/src/content-core.js` 已不再无条件使用第一个 status link；主 Tweet permalink 会排除 quote card link，并优先使用 time 关联链接；reply parent link 会排除主 Tweet和 quote link，若无法可靠得到 parent ID 则返回 `null`；MutationObserver 只处理新增节点及其影响到的 article，而不是每次 mutation 全量重扫。Extension content tests 当前 13/13 通过，覆盖多 status link、quote/reply 分离、self-reply 防护、mutation article 过滤和既有 metadata/quote 回归。真实 X DOM、virtualized timeline、SPA 路由、节点复用和 Edge/Chrome rendering 仍在 Windows queue。
- **U17 E3 NativeBridge hardening（LINUX_VERIFIED；Windows pending）：**`extension/src/background.js` 已增加 10 秒默认 request timeout、timer cleanup、重复 `request_id` 拒绝、`code/request_id/retryable` 结构化错误、postMessage failure cleanup 和 port generation fencing；旧 port 的 late message/disconnect 不会影响新连接。Extension tests 当前 18/18 通过，覆盖 timeout、duplicate ID、structured error、old generation、disconnect、runtime.lastError、connect/postMessage failure 和 pending cleanup。真实 MV3 Service Worker reload、Native Host crash/restart、浏览器 runtime.lastError 生命周期仍在 Windows queue。
- **CURRENT / LINUX_VERIFIED：**Python v2 worker 与 Sidecar Supervisor 具备 cooperative `cancel`/`shutdown`、超时、EOF 退出和 POSIX 子进程 session 隔离；Rust Sidecar Supervisor 为 Sidecar 建立独立 Unix process group，并在 shutdown/force cleanup 时回收整个进程组；`spawn_ready_v2` 只接受带 capability 的 v2 `hello → ready`，stdout reader 拒绝非 v2 事件。
- **U8 legacy removal（LINUX_VERIFIED）：**Sidecar protocol v1 runtime、`download` command、`file/progress/complete` 事件、`DownloadRouter`/`GalleryDlThenAria2` fallback、`archive_tweet` 同步入口、v1 PyInstaller entrypoint、v1 Schema/fixtures、Python `gallery.py`/`DownloadedFile` 和 `ExtractedTweet.files/raw` 已删除；`xarchive-protocol` 只保留 `DownloadFile`（移至 `media.rs`）作为 commit 事实类型。`stop_sidecar`/`start_sidecar` 改用 v2 shutdown/handshake。Linux 验证：workspace Rust 184/184、protocol 14/14、Supervisor 6/6、download 17/17 + integration 7/7、Desktop Rust 82/82、Node Desktop 33/33、Extension 7/7、Sidecar pytest 21/21、fmt/check/clippy/build 通过。
- **U3 contract（LINUX_VERIFIED；Windows packaged handshake PASS，范围受限）：**Sidecar protocol v2 typed contract 已落地：`xarchive-protocol` 新增 `sidecar_v2` 模块（`hello` capability handshake、`extract` command、`ready/extraction_started/extracted/cancelled/failed/log` 事件、typed `ExtractionResult`、Tweet ID 与 X URL 绑定、header allowlist、v1/unknown field/缺失 capability 显式拒绝），Python 侧新增 `protocol_v2`/`worker_v2`/`extraction` 模块与 contract tests，Schema 与 fixtures 新增 `sidecar-v2-command/event` 和 v1 rejection fixture。Rust protocol 测试 15/15 通过。U7 已将 v2 Supervisor handshake/event consumption 接入 Desktop executor；U8 后 PyInstaller spec 只使用 v2 entrypoint，v1 fallback 入口已删除。Windows 当前已验证 packaged v2 hello/capability/unknown-field/shutdown，但真实 extraction/download 仍未验收。
- **U4 extraction-only adapter（LINUX_VERIFIED，v2 路径）：**`extraction.py` 已收缩为 extraction-only：gallery-dl 命令强制 `--skip-download` 且防御性拒绝 `--directory`/`--filename`/`--download`；metadata 读取兼容 `info.json` 与 `*.info.json`；媒体项生成 stable media identity；filename 经净化；result 只承载 metadata + media plan，不携带已下载文件事实；header 仅 Referer/Accept 且过 secret 检查。Sidecar pytest 33/33 通过；Windows 兼容的 Python-driven fake executable 已覆盖 extraction-only 回归，U7 已消费该 v2 result；v1 媒体下载链路和 `DownloadedFile` 已在 U8 删除。
- **U5 aria2-only transfer driver（LINUX_VERIFIED）：**`xarchive-download` 已提供 backend-neutral `MediaTransferPlan`、aria2-only `Aria2TransferDriver`、multi-GID polling、progress、timeout、cancel/shutdown、error/removed 分类和 partial/`.aria2` cleanup；U7 已将 driver 接入 Desktop production executor，U8 已删除旧 `DownloadRouter` 后当前为 17/17 unit、7/7 integration。
- **U6 URL refresh contract（LINUX_VERIFIED）：**`RefreshCoordinator` 和 `EXTRACTION_RESULT_CHANGED` contract 已完成；U7 production path 对 `TRANSFER_EXPIRED_URL` 执行一次完整 v2 extraction refresh，按 stable media identity + filename 集合匹配后使用新 plan/new GID 重试；普通 transfer failure、cancel、shutdown、timeout、权限/磁盘错误不 refresh。
- **U7 Desktop production integration（LINUX_VERIFIED；Windows runtime 未完成）：**新增 `desktop/src-tauri/src/production.rs`，将 v2 `hello/extract/events`、typed `ExtractionResult`、`MediaTransferPlan`、aria2 transfer、一次性 URL refresh、staging path/file/reparse checks 和 `ArchiveService::complete_sidecar_archive` 接入 runner-owned executor。`ArchiveExecutionContext` 保留 Database/FileStore/Sidecar/aria2 配置 ownership；U8 之前的 v1 `archive_tweet`/`DownloadRouter` fallback 已在 U8 删除，executor 命令是唯一业务入口。Desktop Rust 82/82、workspace tests/doc-tests、strict Clippy、Node Desktop 33/33、Extension 7/7、Sidecar pytest 33/33（U8 删除 v1 测试后为 21/21）、fmt/check/build 均通过。Windows current-source baseline、v2-only worker/packaged handshake、Core/Full build boundary、startup smoke 和当前 WDIO scope 已通过；aria2、真实 extraction/download、refresh、staging/commit、file lock 和 restart/recovery 仍为 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`。
- **U12 Native Host/Extension installation flow（LINUX_VERIFIED；Windows/browser runtime 未完成）：**新增 `desktop/scripts/native-host-package.mjs` 和安装契约测试，固定 `com.tw2tg.xarchive`、校验 MV3 manifest、nativeMessaging/storage 权限、X/Twitter host permissions、Extension ID、host manifest 和 Windows x64 安装布局。Desktop Extension 状态现在区分 `missing`、`not_loaded`、`not_registered`、`not_available`、`connected` 等边界；Linux 不伪造 Registry、ACL、Named Pipe 或浏览器连接。固定 Extension ID 仍依赖发布密钥/浏览器发布策略；实际 Edge/Chrome developer-mode load、Native Host registration、reconnect 和 Windows endpoint 进入 queue。
- **U13 Offline Bundle（LINUX_VERIFIED；Windows artifact/runtime 未完成）：**新增 `desktop/scripts/offline-bundle-package.mjs` 和契约测试，固定 Offline Bundle 组件集合（Desktop、worker、Native Host、Extension、gallery-dl、aria2），校验相对路径、SHA-256、size、required_files、license_files、运行时目录排除以及 release manifest/catalog/catalog_version parity。Linux 只验证 manifest 和 parity 逻辑，不生成、签名或伪造真实 Windows artifact；真实 bundle 组装、解压、许可证扫描、签名、embedded catalog 填充和 Windows startup 进入 queue。
- **U14 Linux full verification（LINUX_VERIFIED；验证对象含 working tree changes）：**2026-09-20 在 `feature/u7-desktop-production-integration`、HEAD `f2ae58d`、包含未提交 U12/U13 修改的 Linux working tree 上完成全量适用验证：Rust workspace fmt/check/test/clippy 通过，Desktop Rust 86/86，Node Desktop 44/44，Extension 7/7，Sidecar pytest 21/21，Linux Tauri/WDIO smoke 2/2，Node check/build、Python compileall 和 `git diff --check` 通过。随后 Windows 对 current source 完成 Node 44/44 + Extension 7/7、Rust 188 tests/fmt/check/strict Clippy、Sidecar 21/21、v2-only worker、Tauri release、Core/Full assembly/startup 和 WDIO scope 验证；这些结果不关闭真实 aria2/runtime、filesystem/activation、Registry/browser、Offline Bundle parity 或 tag-level release workflow failure。
- **U9 ComponentManager（LINUX_VERIFIED；Windows asset/runtime 未完成）：**新增 `desktop/src-tauri/src/components.rs`，完成固定 embedded catalog schema、SHA-256/size/layout/license/probe/protocol 字段校验、safe relative path、symlink/special-file 拒绝、deterministic directory digest、`.part` staging、atomic activation、current/previous marker 和 rollback。当前 catalog 为空是安全边界，因为 U11 尚未生成真实版本化 release assets/hash；不执行动态网络下载。Desktop Rust 86/86、fmt/check/strict Clippy 通过；Windows ZIP/EXE probe、权限、filesystem atomicity、真实 catalog/assets 和 Setup Wizard 仍为 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`。
- **U10 Core Bootstrap Setup Wizard（LINUX_VERIFIED；Windows runtime 未完成）：**新增 `get_component_bootstrap_status` Tauri command 和设置页 Core Bootstrap 状态卡，读取固定 embedded catalog、active versions、缺失组件和 catalog message；空 catalog 明确显示等待 release assets，不伪造组件已就绪。首次下载目录 Setup Wizard 仍通过 `complete_download_setup` 选择 portable/system Downloads，并重建 executor runtime。Desktop Rust 86/86、Node Desktop 34/34、Extension 7/7、fmt/check/strict Clippy/build 通过；Windows Core `.exe`/WebView2、权限、真实组件安装和 release catalog 仍为 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`。
- **U11 Release assets/pipeline（LINUX_VERIFIED；Windows tag-level release 仍失败）：**新增 `desktop/scripts/release-assets.mjs` 定义版本化 Windows x64 资产命名与 manifest 契约（tag、platform、catalog_version、assets、licenses、SHA-256、size_bytes、license 路径），禁止动态 `latest`、无 hash、无 size、无 license；新增 `desktop/test/release-assets.test.mjs` 覆盖接受与拒绝用例。Windows current-source local build/assembly boundary 已通过，但 `v0.2.0-pre.3` 外部 GitHub Actions run `35492155317` 仍在 Rust v2 handshake tests 失败，未产生任何 Release asset；真实签名、SHA256SUMS、许可证扫描、最终发布上传和 Core/Offline Bundle parity 仍为 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_BLOCKED` 或 `NOT RUN`。
- **WINDOWS_VERIFICATION_PENDING：**Windows Job Object/process-tree、文件锁、真实 aria2/extraction/refresh、staging/commit、WebView2 Bootstrap/manual setup、Named Pipe、Registry、真实浏览器 reconnect、真实账号和最终发布资产仍需集中验证。当前 Windows 路径仍未声称使用 Job Object；Linux 侧只完成了 Unix process-group contract。当前-source Windows local PASS 不得覆盖 `v0.2.0-pre.3` tag-level `WINDOWS_FAIL`。

目标终态和 Unit 依赖顺序见 [`roadmap.md`](roadmap.md)；目标架构决策见 [`../architecture/decisions.md`](../architecture/decisions.md)。

- 2026-09-19 U8 旧路径删除 Linux 收口：删除 `archive_tweet` Tauri command/注册和同步 archive fallback（含 `download_sidecar`、`run_sidecar_download`、`safe_sidecar_error_message`、未再使用的 `ArchiveExecutionContext::new`）、`crates/xarchive-protocol/src/sidecar.rs`（v1 command/event/`MessageType`）并把 `DownloadFile` 移到新的 `media.rs`、`xarchive-download` 的 `router.rs`（`DownloadRouter`/`GalleryDlThenAria2`/`GalleryDlFailure`/`DownloadRoute`/`DownloadResult`）、Supervisor 的 v1 `send`/`spawn_ready`/`SupervisorEvent::Download`（stdout reader 现在只接受 `protocol_version = 2`，其他版本记为 `ProtocolError`）、`shared/protocol-schema/download-command.schema.json`/`download-event.schema.json` 及对应 fixtures、Python v1 worker（`__init__.py` 重写为 v2 CLI 启动器）、`gallery.py`、`DownloadedFile`/`ExtractedTweet.files/raw`、`pyinstaller/entrypoint.py` 与 `entrypoint_v1.py`、以及 v1 测试文件（`test_worker.py`、`test_gallery.py`、`test_scaffold.py`）。`start_sidecar` 改用 `spawn_ready_v2`，`stop_sidecar` 改用 `send_v2_shutdown`。新增 `test_entrypoint.py` 覆盖启动器参数、legacy 模块/symbol 缺失、`python -m xarchive_downloader` 的 v2 handshake 和 v1 command 拒绝。Linux 验证：workspace Rust 184/184、`xarchive-protocol` 14/14、Supervisor 6/6、`xarchive-download` 17/17 + integration 7/7、Desktop Rust 82/82、Sidecar pytest 21/21、compileall、fmt、strict clippy、`cargo check` 和 `git diff --check` 通过；Node/Extension 测试与 build 一并复验。Windows 专属验证（packaged v2-only worker artifact、Tauri command surface、Job Object/Sidecar shutdown、真实 extraction/aria2/commit、file lock/restart recovery）已加入 Windows queue 并附手工步骤。
- 2026-09-18 U4 gallery-dl extraction-only Linux 收口：`extraction.py` 重写为 extraction-only 适配层——gallery-dl 命令强制 `--skip-download`/`--write-info-json` 并防御性拒绝任何媒体写入 flag；修复此前 `to_extraction_result` 尾部死代码与重复定义；新增 `sanitize_filename`（路径分隔符、控制字符、`..`、隐藏名、超长与空值 fallback `NN.bin`）与 `stable_media_id`（metadata `media_id` → URL 文件名段 → `media-NN`）；`_read_info_json` 兼容 `info.json` 与 `*.info.json`；移除未在 v2 事件词汇表中的 `metadata` 事件（raw metadata 不外发）；序列化剥离 `raw`，result 不含 `DownloadedFile`/staging 扫描事实。新增 `test_extraction_only.py`：命令无下载 flag、filename 净化、stable identity、真实 fake gallery-dl 进程写媒体文件但 result 无下载事实、AUTH_REQUIRED 不回退下载。Sidecar pytest 28/28、compileall、`git diff --check` 通过；Rust 本轮未改动。v2 extraction 仍未接 Supervisor/Desktop；v1 媒体下载链路保持 MIGRATION，Windows packaged worker 行为进入集中验证队列。

- 2026-09-18 U3 Sidecar protocol v2 contract Linux 收口：`xarchive-protocol` 新增 `sidecar_v2` 模块，覆盖 `hello` capability handshake、`extract` command、`ready/extraction_started/extracted/cancelled/failed/log` 事件、typed `ExtractionResult`（metadata + media plan，不承载已下载文件事实）、Tweet ID 与 X URL 绑定、Cookie/Authorization header 拒绝、v1 command/unknown field/缺失 capability 显式拒绝和 stable `CANCELLED`/`INTERRUPTED` 错误码。Python 侧新增 `protocol_v2` 模型、`worker_v2` 主循环与 `extraction` 适配层；Schema 新增 `sidecar-v2-command/event` 与 valid/invalid/v1-rejected fixtures。Rust protocol 15/15、Sidecar pytest 23/23、fmt/clippy/Node check/test、compileall 与 `git diff --check` 通过。Supervisor spawn v2 worker 与 Desktop 消费属于 U7 运行时接线，当前运行链路保持 v1；Windows packaged worker 行为进入集中验证队列。

- 2026-09-18 U2 Sidecar cooperative cancellation Linux 收口：Python worker 使用独立 command-reader 与控制队列，在 gallery-dl 运行期间持续消费 `cancel`/`shutdown`；`GalleryDlRunner` 通过短轮询、超时和稳定错误码区分 `CANCELLED`、`INTERRUPTED`、`DOWNLOAD_TIMEOUT`，POSIX 下载子进程建立独立 session，Windows 路径使用 `taskkill /T /F` 回收进程树。Sidecar compileall、pytest 17/17 和 diff check 已通过。Windows 子进程树、文件锁、残留进程和 shutdown 时序仍进入集中验证队列。
## 当前基线与限制

- 当前执行计划为 [`roadmap.md`](roadmap.md) 的 **R8：v0.2.0 整合与正式发布**（2026-09-29 计划定稿）：从 U7 与安全分支整合统一基线、统一版本 `0.2.0`、全部压缩包资产统一为 `.7z`、Linux 门禁 → Windows 集中验证 → Actions 演练 → 正式发布 → 回合 `dev`/`main`。发布口径（不承诺真实 X 归档成功、个人用途不保证其它设备可用、各资产定位）与资产格式契约见 R8。阶段 1（分支盘点 + 范围冻结）已于 2026-09-29 完成，整合起点 `783a021`，分支处置表与 P0 门禁表见 R8。

- 2026-09-26 工程审查（只读）：当前工作区为 `security/tweet-url-host-validation` `0321bf0`，基于本地 `dev` `1786c6a`，不是 U7 开发分支 `feature/u7-desktop-production-integration`。审查确认 2 项 HIGH（打包输出目录递归删除保护缺失、当前分支 Windows 条件编译 `PathBuf` 导入缺陷）、11 项 MEDIUM、3 项 LOW，详见 [`../review/engineering-audit-2026-09-26.md`](../review/engineering-audit-2026-09-26.md)。修复计划为 [`roadmap.md`](roadmap.md) 的 R7，风险登记为 RISK-014 至 RISK-022，Windows 队列为 `WQ-ENG-01` 至 `WQ-ENG-08`。审查未修改产品代码、依赖或配置。
- 当前工作区**不适合作为直接发布基线**：需先完成 R7 的 P0（ENG-01、ENG-02）。
- 依赖审计实际命中 RUSTSEC-2026-0285（`rustls 0.23.43`）与 16 个 Node 依赖条目（主要为 WDIO/测试工具链）；GitHub Dependabot 与 secret-scanning 开放列表为空。`npm audit` 条目数不等于独立漏洞数。
- 敏感数据生命周期已有初步结论，但真实 gallery-dl 错误脱敏、发布包内容与 Git 全历史凭据仍未验证。
- 审查报告中的 Windows 相关结论均未在 Windows 执行，不得当作平台验证结果。

## 已实现

- 2026-10-01 Telegram Batch A 续做（发送驱动与计划模型）：新增 `run_claimed_attempt()` 异步驱动（栅栏 → 发送 → 分类 → 持久化，丢失领取在发送前以 `StaleClaim` 终止）、`send_upload_attempt()`（从 body tracker 给出真实请求进度，成功时提取 message/file id）、`PlannedSend`/`plan_media_sends()`/`plan_text_send()`（稳定 idempotency_key + 内容 fingerprint，绑定 config_version 与归档媒体引用）、`record_outbox_cancelled()`（已领取未发出时的干净取消）与 `Database::list_outbox_for_tweet()`（任务发送状态投影的数据源）。Linux 验证：`xarchive-telegram` 46/46、`xarchive-storage` 54/54、workspace 严格 clippy 0 告警、fmt 与 docs-audit PASS。Windows 项仍全部 `NOT_RUN`（blocker `BATCH_B_NOT_IMPLEMENTED`），手工步骤见 [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md) §K。
- 2026-10-01 Telegram Batch A TG-01/TG-04/TG-05/TG-06 共享层续做：`TelegramError::Api` 新增 `parameters` 并修复非 2xx JSON 错误被折叠为 “HTTP status”，新增 `ResponseLost`（请求已发出但响应丢失）语义；`BotToken::identity()` 以单向 SHA-256 提供可存储的 bot 身份；新增 outbox 状态机（`OutboxState`）、`TelegramOutboxStore` 契约与 `classify_send_failure`/`decide_outbox_transition` 重试决策、`UNKNOWN` 原因常量；`xarchive-storage` 以**新增** migration `0007_telegram_outbox.sql` 实现 `telegram_outbox`（原子领取 + 租约 + `request_started` 栅栏 + `next_retry_at` + 崩溃恢复）与 `telegram_file_cache`（bot 隔离、`file_unique_id` 仅识别）；Desktop 新增 `TelegramConfig`/`TelegramSettings`（无 token 字段、仅 presence 标志、`revision` 绑定队列项、能力记录失效）。架构文档同步：Telegram 不在归档主链路、outbox 状态机、data-model 与 runtime-flow 已更新。Linux 验证：`cargo test -p xarchive-telegram` 40/40、`-p xarchive-storage` 52/52、`-p xarchive-desktop` 166/166、`cargo clippy`（三个 crate，--all-targets）0 警告、`cargo fmt --check`、`scripts/docs-audit.mjs` PASS。Windows 项全部保持 `NOT_RUN`（blocker `BATCH_B_NOT_IMPLEMENTED`），手工步骤见 [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md) §K；本条不构成任何 Windows PASS。

- 2026-10-01 Telegram Batch A TG-01/TG-02 共享 transport 实现（计划见 [`telegram-local-bot-api-plan.md`](telegram-local-bot-api-plan.md)）：`xarchive-telegram` 新增 `ReqwestTelegramTransport::with_api_endpoint()`，作为 `TelegramEndpoint` 契约的生产消费方（cloud HTTPS；local loopback HTTP 强制直连、不套用代理；旧的字符串构造器保持 HTTPS/test-only 规则，`with_endpoint("http://…")` 仍返回 `InvalidEndpoint`）。新增局部异步流式上传 `send_upload()`：文件按 `expected_size` 重新打开校验，以 64 KiB pull 分块（`TrackedUploadStream`）流进长度已知的 multipart part，整段视频不进内存；`UploadTimeouts` + `upload_watchdog` 分层判定 connect/body-stall/server-processing/overall 四类超时并各自映射独立错误变体；`CancellationToken` 取消映射为 `UploadCancelled`，已发出的请求不自动重试（由调用方记 `UNKNOWN`）；进度按 `UploadStage`（Queued→CheckingFile→Uploading→AwaitingResult→Confirmed，Confirmed 仅在解析 `ok: true` 后发出）；响应体 1 MiB 有界（异步上传与阻塞控制路径一致）并提取 `result_message_ids()`/`result_file_ids()`；redirect 全禁用（token 在 URL 中）。reqwest 0.13.4 在 debug 构建下禁止在 Tokio 上下文内构造阻塞客户端，该构造约束已写入结构体文档。Linux 验证：`cargo test -p xarchive-telegram` 32/32（新增 11 例：上传请求/超时校验、media kind 映射、结果提取、endpoint 契约、watchdog 三层分类、流式上传 e2e fake server、响应超限、API 错误分类、取消与文件校验短路）、`cargo clippy -p xarchive-telegram --all-targets` 0 警告、`cargo fmt --check`、`scripts/docs-audit.mjs` PASS。无生产调用方、无真实发送证据；`WQ-TG-*` 全部保持 `NOT_RUN`，本条不构成任何 Windows PASS。

- 2026-10-01 界面紧凑化、设置页顺序与高清图标批次（Linux 侧实现，计划见 [`desktop-ui-density-icon-fix-plan.md`](desktop-ui-density-icon-fix-plan.md)）：(1) 侧栏 `服务状态` 紧凑化——`.sidebar-caption` 下边距 `12px→6px`、`.connection-line-button` `min-height 30→28`、`padding 5px→4px`、行间距 `2px→0`，主导航行 `min-height: 40px` 与全局按钮尺寸不变；(2) `运行环境` 内 `管理组件与设置` 的 `margin-top: auto` 改为 `0`，改由 `.control-panel-content` 的 `16px` gap 提供节奏，卡片仍 `align-items: stretch`，多余高度落在操作组下方——仅调 gap 无法修复，因为 `auto` 会吸收全部剩余高度；(3) 新增 `#bootstrap-settings { padding-top: 25px; border-top: 0; }`，删除 `Core Bootstrap` 上方分隔线并以 `+1px` 补偿，标题位置不变、其他设置节分隔线不变；(4) `ProxySettings` 整块移入底部 `settings-layout-secondary` 容器，位于 `日志设置` 之后，id/props/回调/状态管理未改，为移动而非复制，DOM、视觉与 Tab 顺序一致；(5) 新增 `dashboard` SVG 路径并用于 `工作台` 导航，Sidecar 状态行仍用 `activity`；(6) `make-icon.py` 的 `render()` 改为在超采样画布上一次绘制底板/归档盒/X、仅末段缩小一次，原先“按目标尺寸绘制底板再放大”才是大尺寸模糊的根因，`SIZES` 补入 20/40 并重新生成全部资源，ICO 现含 16/20/24/32/40/48/64/128/256。图标模糊**不是缺资源**（原有 16/24/32/48/64/128/256 与 512 主图齐备）。Linux 门禁：`npm test` 188/188（新增 6 例，其中直接解析已提交的 ICO 二进制校验各尺寸均为独立 PNG 位图）、`npm run check` PASS（并复核产物 CSS 而非仅源码）、`scripts/docs-audit.mjs` PASS。**Linux 渲染验收记 `BLOCKED`**：本机无浏览器与 Windows WebView2，静态断言不是视觉验收；Windows 侧见 [`../validation/windows-queue.md`](../validation/windows-queue.md) `WQ-UI-030-01`..`WQ-ICON-030-06`，本批次不构成任何 Windows PASS。

- 2026-09-30 v0.2.0 修复批次（Linux 侧实现，计划见 [`desktop-ui-known-folder-fix-plan.md`](desktop-ui-known-folder-fix-plan.md)）：(1) Dashboard 双列 `.dashboard-grid` 由 `align-items: start` 改为 `stretch`，两张卡片改为纵向 flex 且 `.ui-card-content` 撑满，使“最近任务”与“运行环境”底边平齐，窄窗口单列不要求等高；(2) 新增 `.connection-line-button` 外观重置（`border: 0`、透明背景、统一内边距/圆角/宽度、浅灰 hover），去除侧栏三行服务状态的原生按钮黑边，并复用既有 `:focus-visible` 焦点环，不移除键盘焦点提示；(3) `.version-label` 去掉 `-8px` 负上边距、改用 `margin: 0 8px` 与 `11px/1.5` 行高，`.sidebar-footer-separator` 下间距改为 12px、侧栏底部内边距由 20px 调整为 14px，使版本行在分隔线与侧栏底边之间居中；(4) `system_download_directory` 不再拼接 `USERPROFILE`/`HOME` + `Downloads`，改为经 `crate::platform::system_download_directory` 使用 Tauri 路径解析器的 `download_dir()`，在 Windows 落到 Shell Known Folder API（`SHGetKnownFolderPath` + `FOLDERID_Downloads`，不使用 `KF_FLAG_DEFAULT_PATH`，由依赖负责 `CoTaskMemFree`），因此“使用系统下载目录”会遵循用户重定向位置；归档仍使用其下的 `XArchive` 子目录（`archive_directory_in`），查询失败返回明确错误且不静默回退，仓库 `unsafe_code = "forbid"` 规则不变。`get_portable_setup` / `complete_download_setup` 增加 `AppHandle` 参数以取得解析器。Linux 门禁：`cargo fmt --check`、严格 Clippy（`xarchive-desktop` all-targets）、`cargo test -p xarchive-desktop` 122/122（新增 `appends_archive_directory_to_a_resolved_download_directory`）、`npm run check`、`npm test` 157/157（`ui-wiring.test.mjs` 新增 3 条样式回归）。静态断言不是视觉验收，Windows 行为见 [`../validation/windows-queue.md`](../validation/windows-queue.md)，本条不构成任何 Windows PASS。

- 2026-09-29 v0.2.0 整合基线（分支 `release/v0.2.0`）：U7（Popup/Options、batch/websocket、七资产发布工具链）与安全分支（R7 审查整改、Windows 打包与 WDIO 配方）在 merge `61ff1ae` 合并，33 处冲突按协议安全 / Extension / 桌面与 Sidecar / 构建发布 / 文档五个边界裁决。移植到 U7 结构的整改项：ENG-06 真实时钟（单文件 executor，7 个调用点）、ENG-05 有界 stderr 捕获与 Supervisor 有界行读取、ENG-12 脱敏（Sidecar 事件、缺失依赖错误、日志写入前 redact）、ENG-13 aria2 secret 经 `--conf-path` 且 proxy 走子进程环境变量、ENG-03 `resolve_within`（staging 同样拒绝中间链接）、ENG-01/07/08/09 打包目录保护、发布文件排除（保留 `.pyd`/`.dll`）、tag 绑定与可执行文件 commit/SHA-256 记录。协议保持 U7 的 sidecar v2（v1 已随 U8 删除）。版本统一为 `0.2.0`（Cargo/npm/tauri/Extension/锁文件/侧栏字面量，含对齐回归）；Extension 独立发布包由 `.zip` 改为真实 `.7z`（命名契约、workflow 创建/解包/上传、ZIP 名拒绝用例）；Popup 显示 `chrome.runtime.getManifest().version`；README 新增《使用范围与限制》与《发布资产（v0.2.0）》。ENG-15 的 executor 拆分在整合中被 U7 单文件取代，RISK-024 保持 OPEN（见 [`roadmap.md`](roadmap.md) R7/R8）。Linux 门禁：`npm run check`、`npm test` 141/141 + extension 32/32、`cargo fmt --check`、严格 Clippy、`cargo test --workspace` 262/262、pytest 46/46、workflow YAML 解析。该条只记录 Linux 事实，Windows 行为以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准，不构成任何 Windows PASS。

- 2026-09-28 WQ-ENG-09b E2E 启动取证与 driver 版本钉版（测试层，不改产品行为）：新增 `desktop/e2e/support/startup-diagnostics.mjs`，在 dashboard readiness 超时时枚举 window handle 并逐个记录 URL/title 与截图，写入 `WDIO_LOG_DIR/startup`（默认 `desktop/test-artifacts/wdio/startup`，已 ignore）；`dashboard.e2e.mjs` 与 `wdio-plugin.e2e.mjs` 的 `before` 钩子在失败时调用它并原样抛出 readiness 错误，取证为 best-effort、不改变通过/失败判定。`wdio.conf.mjs` 新增可选 `TAURI_DRIVER_EDGE_VERSION`/`EDGEDRIVER_VERSION` → service `edgeDriverVersion`，用于把 msedgedriver 钉到应用实际加载的 WebView2 runtime 版本，未设置时保持上游按 Edge 注册表探测。触发背景：2026-09-28 Windows focused phase 在版本匹配的 EdgeDriver 155 下仍为单一 window handle、URL `data:,`、空标题、白屏（WQ-ENG-09b `WINDOWS_FAIL`）；Linux 侧复核确认应用导航路径自 2026-09-16 原生 WDIO PASS（`3f70894`）以来无任何变更（`tauri.conf.json`/`index.html`/vite config/CSP/tauri 依赖版本一致，diff 仅新增 command、dialog 插件与 capability 权限），故本轮只增强测试层可诊断性。Linux 验证：`node --check` 5 文件、WDIO 配置加载（默认 `undefined`／钉版 `"155"`／`"154"` 三态）、`npm run test --workspace desktop` 48/48。本机无 Xvfb，Linux 端无法运行真实 native E2E，Windows 侧行为见 `WQ-ENG-09b`，状态 `WINDOWS_FAIL`（待 focused revalidation）。

- 2026-09-27 ENG-13 aria2 RPC secret 传递（RISK-005 子向量已消除）：`Aria2SupervisorConfig::command_args` 不再输出 `--rpc-secret`，`spawn` 改为先经 `write_secret_file` 将 `rpc-secret=<secret>` 写入短期 `--conf-path` 文件再启动 aria2。文件以 `create_new` 创建防竞态，Unix 侧 `0o600` owner-only（Windows 侧继承调用方 temp 目录 ACL，见 WQ-ENG-12）；spawn 失败、启动超时、`shutdown`/`drop` 均删除该文件。新增测试覆盖 argv 无 secret、文件内容与权限、`shutdown` 删除、spawn 失败无残留。Linux 验证：`xarchive-download` 18/18、workspace 189/189（8 crates）、`cargo fmt --check`、严格 Clippy `-D warnings`。Windows 真实进程可见性与 aria2c.exe 行为见 `WQ-ENG-12`，状态 `WINDOWS_VERIFICATION_PENDING`。
- 2026-09-26 ENG-15 Executor 职责拆分（RISK-024）：将 `desktop/src-tauri/src/executor.rs`（4449 行）按“变化原因”拆为 `executor/{mod,model,persistence,service,runtime,tests}.rs`，属行为不变移动。`mod.rs` 仅做模块组合并以扁平 `pub use` 保持既有 `crate::executor::*` 路径，使 `archive.rs`、`commands.rs`、`runtime.rs`、`transport.rs` 无需改动。行为不变性以 token 级比对验证：规范化注释/导入/可见性后，新旧 token 袋唯一差异是 4 个 token（有意删去的冗余 `mod tests { }` 包装）；同时确认无任何原始行丢失，executor 测试数保持 50/50。拆分中修正了两处由切片边界造成的属性丢失（`JobSnapshot` 的 `#[derive(Debug, Clone, PartialEq, Eq)]`、`ArchiveJobSubmissionAdapter` 与 `JobRecord` 的 derive）并恢复。注意 `cargo fix` 会误删仅被其他模块测试引用的 `pub use runtime::*`，因此未采用自动修复，改为显式记录该 re-export 用途。Linux 验证：`cargo fmt --check`、严格 Clippy `-D warnings`、`cargo check --all-targets`、`cargo test --workspace --locked` 187/187（8 crates）全部通过。Windows MSVC 编译回归见 `WQ-ENG-11`。
- 2026-09-26 ENG-16 工具链锁定（RISK-025）：新增根 `rust-toolchain.toml` 固定 Rust `1.98.0` 并声明 `rustfmt`/`clippy`。选择 1.98.0 是因为它是当前 stable 的实际版本，若按旧版本固定会造成静默降级；该文件使本地与 CI 解析同一编译器。两个 workflow 的 5 个 Action（`actions/checkout`、`actions/setup-node`、`actions/setup-python`、`actions/upload-artifact`、`dtolnay/rust-toolchain`）全部改为按 commit SHA 固定并保留可读版本注释，workflow 中不再存在浮动 tag。`cargo --locked` 与 `pyinstaller==6.22.3` 锁定此前已完成。Linux 验证：pinned toolchain 下 `cargo check --workspace --locked --all-targets`、`cargo test --workspace --locked` 187/187、严格 Clippy、`cargo fmt --check` 全部通过；两个 workflow YAML 解析通过。Windows/CI 实际解析结果见 `WQ-ENG-10`。

- 2026-09-26 ENG-11 Node 依赖公告处置（RISK-023）：审计报告 16 条中仅 3 条为真实公告。已用根 `package.json` 的 npm `overrides` 消除两条：`serialize-javascript` 6.0.2→7.1.2（Mocha 侧）、`@wdio/tauri-service` 内嵌 `deepmerge-ts 7.1.6` 提升为 hoist 的 8.0.2。二者均跨越上游声明的主版本约束，因此不以审计数字结案：改用 19 组差分用例对比 6.0.2/7.1.2 与 7.1.6/8.0.2 的行为（含公告所述的 `RegExp.flags`、`Date.prototype.toISOString` 攻击面、递归合并栈耗尽、prototype 污染），结果完全一致。`extract-zip 2.0.1` 公告范围为 `*` 且无修复版本，作为带理由的风险接受单独记录；`desktop/wdio.conf.mjs` 的 `autoDownloadEdgeDriver: true` 使其路径可达，未按“不可达”结案。实现要点：npm 不会因 `overrides` 变化重新解析既有锁文件（`npm ci` 亦忽略 overrides），从零解析会连带升级 89 个无关包（含 `react`、`@tauri-apps/cli` 与 `undici` 降级），因此改为按 clean-room 解析得到的 integrity 精确改写锁文件，仅产生 3 处最小变更。Linux 验证：`npm ci` 通过、`npm ls` 无 invalid、`npm audit` 16→13 且仅剩 `extract-zip` 一条真实公告、`npm run check`/`test`（Extension 13/13）/`build` 通过、WDIO `ConfigParser` 成功解析配置且 adapter 继承上游 worker/launcher 完整、Mocha 失败上报含完整差异与堆栈且退出码为 1、Sidecar compileall 与 pytest 19/19、Rust workspace 8 crates 全通过（无 Rust 依赖变更）。Windows Tauri v2 原生 E2E 与 EdgeDriver 行为见 `WQ-ENG-09`，状态 `WINDOWS_VERIFICATION_PENDING`。

- 2026-09-26 ENG-10 Rust 依赖升级（RISK-017 部分）：`rustls` 由 `0.23.43` 升至 `0.23.45` 以修复 RUSTSEC-2026-0285（TLS 1.3 加密级别边界校验）。采用 `cargo update -p rustls --precise` 仅更新 `Cargo.lock` 的版本与 checksum，`Cargo.toml` 未修改，`hyper-rustls 0.27.9` 的约束已允许该补丁版本、无需连带升级。`cargo audit` 漏洞数由 1 归零；剩余 7 条为 unmaintained/unsound 警告而非漏洞，已在风险登记中如实记录。Linux 验证：`cargo test --workspace --locked` 182/182（8 个 crate）、`xarchive-telegram` 16/16 与 `xarchive-download` 12/12 的 fake HTTPS server 与 loopback 客户端测试通过、`cargo fmt --all -- --check`、严格 Clippy `-D warnings`。Windows release 构建与真实 TLS 连接行为见 `WQ-ENG-08`，状态 `WINDOWS_VERIFICATION_PENDING`。

- 2026-09-26 R7 IPC 资源上限 Linux 收口（ENG-04）：Unix transport accept 循环新增 `MAX_ACTIVE_CONNECTIONS = 64` 并发上限，达到上限时不再派生新线程；每个连接设置 15 秒读写期限，避免半帧或停滞请求长期占用线程。新增 2 项测试：超过上限的半帧连接突发后服务仍可用、上限与期限取值在单用户本地场景的合理区间内。Linux 验证：Desktop Rust 90/90、workspace 182/182、`cargo fmt --check`、严格 Clippy `-D warnings`。Windows Named Pipe 的同类限制与 ACL 行为见 `WQ-ENG-04`。

- 2026-09-26 R7 发布链路 Linux 收口（ENG-07、ENG-08、ENG-09、ENG-16 部分）：`windows-release.yml` 改为以 `RELEASE_TAG` checkout 并新增 HEAD 与 tag commit 一致性校验，不一致直接失败；产物收集步骤额外记录 commit、短 SHA 与文件 SHA-256，使 release asset 可追溯到具体提交；`cargo check/test` 改为 `--locked`。`build-portable-windows.mjs` 默认强制重新构建，复用既有二进制需显式 `PORTABLE_ALLOW_BINARY_REUSE=1`，且构建后校验二进制确实存在。新增 `filterPackageFiles` 与 glob 编译，排除 `.env`、SQLite 与 WAL/SHM、日志、`__pycache__`/`*.pyc`、`.pytest_cache`/`.ruff_cache`、`node_modules`、`target`、`test-artifacts`、`.DS_Store`、`Thumbs.db`，实测 `extension/.env` 哨兵文件被排除且其余源文件完整保留。worker workflow 固定 `pyinstaller==6.22.3`（已用 `pip index versions` 核实该版本存在，6.11.1 并不存在）。Linux 验证：Node Desktop 42/42 + Extension 13/13、Rust workspace 182/182、Sidecar pytest 19/19、两个 workflow YAML 解析、`git diff --check`。真实发布演练与包内容检查仍为 `WQ-ENG-07`，状态 `WINDOWS_VERIFICATION_PENDING`。

- 2026-09-26 R7 P1 Linux 批次（ENG-06、ENG-12、ENG-05、ENG-14）：新增 `desktop/src-tauri/src/clock.rs`，以无依赖 civil-from-days 算法输出真实 UTC `YYYY-MM-DDTHH:MM:SSZ`，替换 `executor.rs` 7 处与 `transport.rs` 1 处生产固定时间戳（任务 ID 实测为真实时间，transport 测试改为捕获实际 job_id）。`sidecar/errors.py` 新增 `sanitize_error_text`，在协议边界对 Authorization/Bearer、Telegram/GitHub/Slack token、URL 内嵌凭据、query secret、cookie 与绝对本地路径脱敏，并把消息限制为 2000 字符且标记截断；`gallery.py` 的 debug 日志同样先脱敏。`xarchive-sidecar-supervisor/readers.rs` 以 1 MiB 单行上限和分块扫描替代无界的 `lines()`，超限行排空至换行以保持流对齐且不先分配整行；`gallery.py` 改用临时文件替代 `capture_output=True` 并有界保留输出尾部。`logging.rs` 增加单行 16 KiB 截断、单文件 8 MiB 轮转、换行折叠防日志注入，`read_recent` 改为仅读尾部 512 KiB。Linux 验证：Rust workspace 182/182、Desktop 89/89、supervisor 9/9、Node Desktop 40/40 + Extension 13/13、Sidecar pytest 19/19、`cargo fmt --check`、严格 Clippy `-D warnings`。真实账号错误内容、IPC 连接行为、junction/reparse 与 Windows 子进程输出仍为 `WQ-ENG-03` 至 `WQ-ENG-06`，状态 `WINDOWS_VERIFICATION_PENDING`。

- 2026-09-26 R7 P0 Linux 收口（ENG-01、ENG-02）：`desktop/scripts/portable-package.mjs` 新增 `validatePortableOutputDir`，在任何构建或删除前拒绝文件系统根、项目根、项目根祖先、用户家目录及其祖先，以及项目内 `dist-portable/` 命名空间之外的相对路径；`build-portable-windows.mjs` 接入该校验，并把 required 组件与 Extension 存在性检查前移到 `rm` 之前，避免校验失败后留下空输出。`transport.rs` 的 `PathBuf` 改为无条件导入，仅 `Path` 保留 `#[cfg(unix)]`。Linux 验证：Node Desktop 40/40、Extension 13/13、Desktop Rust 80/80、`cargo check`、`cargo fmt`、`git diff --check`；`PORTABLE_OUTPUT_DIR=.` 实测在删除前失败（exit 1）且项目目录完好；旁路对照下 5 项安全测试立即失败。Windows junction/reparse 删除语义与 MSVC 条件编译仍为 `WQ-ENG-01`/`WQ-ENG-02`，状态 `WINDOWS_VERIFICATION_PENDING`。

- 2026-09-18 Desktop UI 布局与依赖选择交互收口：工作台概览移除重复的“数据库”指标卡；修复 Sidecar/Extension 状态图标容器的文字样式串接和垂直居中；运行日志搜索占位符调整为较小字号；设置页 gallery-dl 未检测时仅显示选择按钮，选择后自动调用校验/保存并反馈成功或重新选择；aria2 移除可编辑路径输入框并将下载/选择/校验/保存操作合并到同一操作区；日志等级与最大日志文件数改为并排字段，重新调整 Extension、存储和运行环境间距。Linux Desktop Node 33/33、Vite build、Rust check 和 `git diff --check` 已通过；真实 Windows WebView2、原生文件对话框、DPI 和剪贴板仍待集中验证。

- 2026-09-18 Plan Linux 收口：确认 Browser Native transport 已通过 Unix endpoint 接入 `BrowserTransportAdapter` 和 `ArchiveApplicationService`，R1 的 request_id、重复提交、状态查询、协议错误和 executor error mapping contract 已由 Desktop tests 覆盖；同步 `archive_tweet` fallback 保留用于运行时回退。R2 真实 aria2 fallback 暂不接线：当前 Sidecar failure event 没有 fresh media URL，现有 `DownloadRouter` 不能自行重新提取或安全构造 `AddUriRequest`；需要先完成跨 Rust/Python/Schema 的 extraction-result contract。该项已记录为后续 Linux 设计任务，不归因于 Windows 环境。

- 2026-09-18 异步 submit/schedule Linux 收口：`submit_and_schedule_persisted` 已增加调度前 SQLite 预检；调度线程无法打开 persistence 或 production executor/factory 执行失败时，Job 会记录明确的 `FAILED`、错误码和 `DOWNLOAD_FAILED` 事件，而不是静默丢弃。新增无效数据库路径和后台 executor failure contract tests。Rust workspace 80 项 Desktop tests、Node workspace Desktop 33/33 + Extension 7/7、Sidecar pytest 12/12、普通/WDIO Tauri release build、fmt/check/strict Clippy 和 `git diff --check` 全部通过；Windows executor/Named Pipe/WebView2/process/filesystem 行为仍待集中验证。

- 2026-09-17 Windows worker follow-up：根据最新 Windows bundled worker `--help` 失败结果，移除 PyInstaller entrypoint 重复 `main()` 调用；Windows artifact workflow 在 smoke 前强制检查 `sidecar/dist/xarchive-downloader/_internal/python312.dll`；Core portable manifest 不再声明不存在的本地 Extension import 能力。Linux Python compile、Node contract、Rust/前端回归已完成；WQ-WORKER-BUILD-01、WQ-PACKAGE-CORE-02、WQ-PACKAGE-FULL-01 保持 `WINDOWS_VERIFICATION_PENDING`，等待新 artifact 和 Windows runtime 重验。

- 2026-09-17 GUI/统计/日志收口：Dashboard 新增由 SQLite 全量聚合的 `JobMetrics`（全部、进行中、已完成、失败），不再从最近 20 条任务推算；日志前端统一使用 `error/warning/info/debug/silent` 五档并移除 GUI Trace；Sidebar 服务状态行改为可键盘操作并可跳转到设置页对应区块；gallery-dl 与 aria2 路径支持 Tauri 原生文件选择器；Extension 本地导入入口移除，改为打开 GitHub `extension` 目录。Linux Rust 24 项 storage tests、Desktop Node 31 项、Vite build、cargo check/fmt 和 `git diff --check` 已通过；真实 Windows WebView2、原生对话框、DPI、剪贴板和浏览器集成继续待 Windows 验证。

- 2026-09-17 非 Windows Plan 收口：portable 包类型/组件规划/manifest 已抽为可测试纯逻辑；新增 Full/Core 契约测试、Sidecar `--gallery-dl` 参数回归、PyInstaller worker spec、独立入口和 Windows artifact workflow。Linux 适用测试已完成；真实 Windows worker、Desktop `.exe`、WebView2、文件权限、浏览器集成和发布证书继续进入 Windows Validation Queue。

- 2026-09-17 Full/Core portable 契约第一批：Sidecar 配置分离 XArchive worker 与外部 gallery-dl；Core 设置页支持校验/保存用户提供的 `gallery-dl.exe`，并通过 GitHub `extension` 目录外链和浏览器指南完成 Extension 加载；portable 构建脚本支持 `PORTABLE_PACKAGE_TYPE=full|core` 并生成 `package-manifest.json`。可信自动下载发布源尚未定义，因此不实现任意网络下载；Windows artifact、真实 gallery-dl、WebView2 文件路径和浏览器加载仍需验证。

- 2026-09-17 发布问题修复：Runtime 启动时独立初始化 `config/archive.sqlite3`，任务列表不再因首次下载目录尚未选择而报告 `archive database is not initialized`；设置页接入页面级 Error Boundary，避免渲染异常导致白屏；新增“运行日志”页面，通过 `read_application_logs` 每秒读取最新日志，支持等级筛选、搜索、自动跟随、复制和打开日志目录；Release 主程序启用 Windows GUI subsystem，Sidecar、aria2 和下载 supervisor 的 Windows 子进程统一使用 `CREATE_NO_WINDOW`。Linux 已验证，真实 Windows WebView2、窗口和剪贴板行为仍待验证。

- 2026-09-17 设置页组件批次（pre-2）：前端抽出共享 `Icon`、`CopyablePath`、`ConnectionStatus`/`ExtensionConnectionStatus` 组件和 `ui-state.js` 纯逻辑模块（显示名提取、aria2 状态文案、Extension 状态映射）；设置页 Sidecar gallery-dl 路径与 Extension 目录改为可复制路径组件（经 `copy_text_to_clipboard` Tauri 命令 + `arboard` 写入系统剪贴板，不使用 `navigator.clipboard`）；Sidebar Extension 状态改为显式枚举映射，文件缺失时显示"文件缺失"而不是永久"检测中…"；aria2 设置移除多版本下拉，改为"受信任最新官方版本 + SHA-256 校验 + 自动安装"语义（`latest_aria2_release`），并新增自定义 aria2 路径输入、`validate_aria2_path` 自动校验和 `save_aria2_path` 持久化到 `config.yaml`；Rust `AppStatus` 侧新增 `get_sidecar_path` 返回真实 gallery-dl 可执行文件路径。Linux 门禁全部通过（Node 23/23、vite build、cargo fmt/clippy -D warnings/test 72、check、diff --check）。
- 2026-09-17 Linux Clippy fix：`RuntimeState` 的 post-construction mutation 仅保留在 Unix 构建，Windows 保持不可变；修复 Windows 历史 `unused_mut`，Linux workspace strict Clippy 通过，WQ-P0-01 回到 `WINDOWS_VERIFICATION_PENDING`，不直接标记 `WINDOWS_PASS`。
- 2026-09-17 Windows reconciliation follow-up：修复 runtime 路径测试的 POSIX 硬编码；PyInstaller worker spec 改为与 workflow/portable/config 一致的 one-dir layout；Core portable 明确排除 gallery-dl。Linux fmt/clippy、Desktop 31/31 Node、Extension 7/7、Sidecar 12/12 和 spec syntax 验证通过。WQ-P0-01、worker artifact、Full/Core portable 重新保持 `WINDOWS_VERIFICATION_PENDING`，等待修复后 Windows 重验。
- 2026-09-17 Windows R2 reconciliation：Windows 复验确认 worker one-dir artifact 与 Core portable/start smoke 通过，但暴露 runtime root fixture 和 portable-package path suffix 两个测试契约问题。Linux 已改用相对路径 fixture 与 `path.join()`，workspace Rust、Desktop 31/31、Extension 7/7、Sidecar 12/12 和构建/语法检查通过。WQ-P0-01、worker artifact、Core portable 保持 `WINDOWS_VERIFICATION_PENDING` 等待重验；Full portable 继续因缺少受控 gallery-dl artifact 为 `WINDOWS_BLOCKED`。
- 2026-09-16 GUI 收口批次：工作台与设置页分离；Sidecar、aria2、归档位置、日志设置和 Extension 指南移入设置页；侧栏底部增加设置入口和服务状态；统一 Windows 本地字体栈、图标 SVG 容器、按钮焦点和响应式布局；aria2 文本 Logo 不再使用会导致 `a`/`2` 上下错位的隐式 Grid 行。
- 2026-09-16 Desktop portable path 修复：配置相对路径现在进行不依赖文件系统的词法归一化，`./logs` 显示为 `<portable-root>/logs`，并覆盖 database、cache、download、Sidecar 和 Extension 配置路径；新增嵌套 `.`/`..` 回归测试。
- 2026-09-16 Extension 基础检测：Desktop 新增 `get_extension_status` 和 `open_extension_folder`，检查 `manifest.json`、`src/background.js`、`src/content.js` 是否存在，并在设置页展示 Edge/Chrome 分步骤加载指南。浏览器实时连接和 Native Host 状态当前明确返回未验证边界，不外推为已连接。

- Rust 核心 Job 状态、重试策略、TagEngine 和 Windows-safe 用户目录名。
- Native Host framing、forwarding 和错误处理已按职责拆分为独立模块，公共 API 保持不变。
- Sidecar Supervisor 已按进程监督、错误、事件和 stdout/stderr reader 拆分为独立模块，公共 API 保持不变。
- Protocol crate 已按 Browser、Sidecar、JSONL 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- Download crate 已按 model、router、RPC、HTTP client、supervisor 和 error 职责拆分为独立模块，`lib.rs` 仅组合并 re-export 公共 API。
- Storage crate 已完成模块化第一至第五批：`error.rs`、`models.rs`、`file_store.rs`、`metadata.rs`、`archive_service.rs` 以及 `database/users.rs`、`tags.rs`、`tweets.rs`、`jobs.rs`、`settings.rs`、`telegram.rs` 独立；Database connection 所有权、migration、事务和 public API 保持不变。
- Desktop Rust 已完成行为不变模块化：`aria2.rs` 独立负责 release allowlist、SHA-256 校验、程序发现/版本检测、Windows 下载解压和相关 Tauri commands；`archive.rs` 独立负责 `archive_tweet` 及归档编排；`commands.rs`、`runtime.rs`、`platform.rs` 分别负责 commands、RuntimeState 和平台命令边界；`lib.rs` 仅保留模块组合、请求模型、Tauri 入口/注册和测试入口。
- Desktop Rust 已完成 commands 低风险模块化：`commands.rs` 独立负责 App status、Sidecar 生命周期、Job 查询、archive root、文件夹打开、runtime health 和 Sidecar 配置解析；`archive_tweet`、RuntimeState 长锁和后台 Job executor 设计保持未改变。
- Desktop Rust 已完成 archive 模块化：`archive.rs` 负责 Browser user 绑定、Sidecar archive/download request/result、Browser relationship merge、Sidecar 下载事件处理、`archive_tweet` 编排、ArchiveService 提交、Job 事件/失败状态和安全错误映射；RuntimeState 所有权和现有长锁语义保持不变。
- Desktop Rust 已完成便携 runtime 路径接入：`runtime.rs` 使用 portable root 派生 `config/`、`cache/`、`download/` 和同级 `logs/`；数据库位于 `config/archive.sqlite3`，临时 staging 位于 `cache/staging/`，最终归档位于用户确认的 `download/` 或系统 `Downloads/XArchive`。旧的进程工作目录 `X-Archive` 不是当前便携运行时布局。
- Desktop 已增加 `config/config.yaml` YAML 模型、首次下载目录 setup、日志等级和日志数量设置；Debug 构建默认日志等级为 `debug`，Release 默认 `info`，显式配置优先。Linux 已完成编译和单元测试；完整日志接入、动态级别切换和 Windows 文件权限仍需目标环境验证。
- Desktop `get_app_status` 现在报告 executor 生命周期状态：RuntimeState 初始化后的 bounded worker ownership 为 `ready`，RuntimeState 销毁后 worker 为 `stopped`；该状态反映 control worker ownership，真实 archive execution 使用独立 execution thread 执行长 Sidecar/FileStore I/O。
- Desktop executor commands 已完成 runner-owned submit wiring：`submit_executor_job` 使用独立 SQLite context 写入真实 BrowserTweet/user/Job/spec，并通过 `ArchiveApplicationService::submit_and_schedule_persisted` 立即调度后台执行；Browser transport 的 Unix production endpoint 复用同一 submit-and-schedule 路径。后台由 `ProductionExecutionFactory` 按 `job_id` 加载 execution spec，独立创建 Database/FileStore/SidecarSupervisor/ArchiveExecutionJob；`query_executor_job`、`cancel_executor_job` 和 `shutdown_executor` 继续使用独立 persistence context。`archive_tweet` 保留为同步 fallback。
- Desktop Rust 已完成 platform 边界的行为不变拆分：`platform.rs` 独立负责 Explorer、macOS `open` 和 Linux `xdg-open` 命令选择；`open_archive_folder` 现在打开用户确认后的最终 download root。
- R1 executor 已完成阶段二的 Linux production execution path：`archive_job_requests` 保存 immutable request JSON/schema/request_id，`ExecutorConfig`/`ProductionExecutionFactory` 由 runner 按 `job_id` 加载 spec，独立创建 Database、FileStore、SidecarSupervisor 和 `ArchiveExecutionJob`；`submit_executor_job` 不再从 RuntimeState lease Sidecar。`attempt_count` 防止 late result 覆盖新状态，control loop 与单 active runner 分离。RuntimeState 初始化时会启动 recovery scan；缺失 spec 会标记 `EXECUTION_SPEC_MISSING`。运行中 cancellation 会通过共享 token、Sidecar `cancel`/shutdown 和 attempt fencing 保护 `INTERRUPTED` 状态；真实 Windows 进程/文件锁行为和最终用户入口切换仍需后续验证/开发。
- R1 executor 当前 Linux contract 覆盖 bounded queue、重复 submit、query/cancel/shutdown、execution spec persistence、runner-owned context creation、recovery/completion、execution success/failure、terminal skip、attempt fencing、运行中 cancellation、资源 ownership 和 event ordering；同步 `archive_tweet` 仍保留为显式 fallback。
- Storage Job event query contract 已修正：`events.payload_json` 对状态变更事件允许为 NULL，`list_events_for_job` 现在以 `Option<String>` 表达该事实，并已由 Desktop SQLite contract adapter 回归验证。
- 版本化跨进程协议、JSON Schema、Native Messaging framing 和协议校验。
- `SidecarCommand` 现在通过 `serde(deny_unknown_fields)` 拒绝未声明的 per-request 字段（包括已移除的 `executable` override）；协议层 targeted regression 已覆盖该安全边界。Windows 真实 Sidecar、路径权限和 reparse/link 仍未完全验证，WQ-P1-12 当前状态以 Windows 队列为准。
- 2026-09-16 Windows 增量复验发现 Python worker 未拒绝带 `executable` 的未知字段；Linux 已在 worker 入口增加与 `download-command.schema.json` 对齐的允许字段检查，并新增 worker regression。WQ-P1-12 已恢复为 `WINDOWS_VERIFICATION_PENDING`，等待 Windows 重验；路径权限、reparse/link 和真实 Sidecar download 仍因缺少受控 fixture 保持 `NOT RUN`，详见 `windows-validation.md`。
- Python gallery-dl Sidecar、JSONL worker、metadata 归一化和媒体文件事件。
- SQLite users、user names、tweets、media、jobs、events、tags、Telegram send state 和关系数据。
- staging → Rust 校验 → 最终归档目录的文件提交流程。
- Tweet/URL identity binding、Sidecar metadata identity binding、settings 输入限制和 Sidecar 错误脱敏。
- Tauri Desktop runtime、Job 查询、Sidecar lifecycle、aria2 discovery 和 React Dashboard。
- Windows 便携运行时 Linux 侧实现：portable root、`config/config.yaml`、`config/archive.sqlite3`、`cache/`、同级 `logs/`、`download/` setup、`sidecar/gallery-dl`/`sidecar/aria2` 路径优先级、Extension 复制和 `build:portable:windows` 目录组装脚本。
- MV3 Extension 的 Tweet DOM 提取、归档按钮、状态查询和 Native Messaging bridge。
- Telegram request/formatter/transport contract、SecretStore abstraction 和幂等发送状态模型。

## 部分实现

- `DownloadRouter` 已完成跨平台策略和单元测试，Desktop 当前仅通过 Router 包裹 gallery-dl 结果并统一错误映射；真实 aria2 fallback、fresh media URL contract、403 后重新提取 URL、应用级 transfer lifecycle 尚未形成完整链路。不得将当前半接入状态标记为 R2 完成。
- Native Host 的 framing、校验和可插拔 forwarding 已完成；Windows Named Pipe server、ACL、Registry 和浏览器安装仍未完成。
- GUI 的源码级状态、语义结构、焦点样式和视觉 token 已完成；真实 WebView2、DPI、键盘、屏幕阅读器和对比度仍需 Windows 验收。
- Telegram 的跨平台 transport 和发送状态模型已完成；Credential Manager、真实账号和生产发送链路仍未完成。
- **部分实现：**Desktop 已加入 WebdriverIO 9 + @wdio/tauri-service Windows automation baseline，并完成 tauri-plugin-wdio 1.4.0 的专用 wdio-e2e 配置；Linux service adapter 已加入。最新 current-dirty Windows 验证中 WQ-P1-16/WQ-P1-17 为 `WINDOWS_FAIL`：ordinary/advanced session 未渲染 Dashboard `h1`；该结果尚未证明是业务 UI 缺陷，需 Windows native render/session 诊断。

## 未实现或未完成

- **Browser/Native Host production integration（PLANNED / WINDOWS_VERIFICATION_PENDING）：**Browser protocol 当前 command 仍只有 `archive_request` 和 `query_status`；`query_status → archive_status_batch`、严格 Browser Rust/schema parity、DOM identity/mutation filtering、NativeBridge timeout/duplicate-id/generation fencing 和 Linux 页面状态同步逻辑已完成。真实 browser → Desktop 状态更新、Windows Named Pipe server、当前用户 Registry install/repair/unregister、真实浏览器连接状态和 Extension ZIP/release parity 尚未全部完成。不得把 Native Host package contract、Unix socket 测试或 Extension Node tests 作为 Windows 浏览器链路 PASS。

- 同步 `archive_tweet` 到 executor 的最终产品入口退役仍未完成；Browser transport 与 `submit_executor_job` 已统一为立即返回初始 Job 状态并后台调度 production execution，但同步 fallback 仍保留。executor 运行中 cancellation、真实 staging/final recovery action 已接入 Linux 生产路径并由回归测试覆盖；用户主动取消已收口为 `CANCELLED`，应用关闭/崩溃中断继续使用 `INTERRUPTED`，Sidecar process-tree 终止和取消/提交竞争语义仍待后续批次完成。Desktop 已在 Linux/Unix 上注册生产 transport endpoint（Unix domain socket），Native Host 在 Linux 上改用 `UnixStream` 连接；Windows 侧仍需验证实际子进程终止、文件锁、重启和打包行为。Windows Named Pipe server 尚未注册，Native Host 在 Windows 上仍通过 `OpenOptions` 文件路径连接。
- Windows Named Pipe server、Native Host manifest/Registry、Tray、Single Instance、Autostart 和 Credential Manager。
- Sidecar `externalBin` 的最终分发行为、正式 bundle、安装器、签名和 updater。当前阶段只生成便携版 `.exe`，不生成 installer。
- 便携版 `.exe` 同目录的真实路径解析、`config/config.yaml` 持久化、cache→download 跨卷提交、系统 Downloads fallback、sidecar/aria2/gallery-dl/Extension 实际分发和 Windows 文件权限。
- 真实 Edge Cookie/X 认证归档和真实 Telegram 账号发送。
- 应用级旧 SQLite 启动迁移、重启恢复和跨用户 ACL 验证。

## 当前开发方向

### 2026-09-21 Extension identity / release reliability 批次（U18）

本批次把 U17 E5–E9 之前缺失的前置显式化，计划与批次状态见 [`roadmap.md`](roadmap.md) U18；当前状态为 `IN_PROGRESS`，未完成项不得标为 PASS。

- **Extension identity 基线（B0 部分 / B1，LINUX_VERIFIED）：**`extension/manifest.json` 已加入 public `key`；`desktop/scripts/extension-identity.mjs` 成为 identity 单一入口（DER SHA-256 → 前 128 bit → `a`–`p` 映射，`derive`/`verify` CLI）；派生的 canonical ID 为 `iaajefkoanbkleojofoadeakelihbjne`。`native-host-package.mjs` 与 `build-portable-windows.mjs` 在生成 host manifest/installation manifest 之前强制 key 与 `XARCHIVE_EXTENSION_ID` 一致，ID 校验收紧为 `[a-p]{32}`。
- **Linux 验证证据（2026-09-21）：**`npm run test --workspace desktop` 55/55（含新增 `desktop/test/extension-identity.test.mjs` 7 项、`validateExtensionIdentity` 契约测试和 Native Host manifest CLI 测试）；`npm run check --workspace extension` 与 Extension tests 21/21；根 `npm run check` 通过；两个 workflow YAML 可解析。
- **Workflow parity（B2，LINUX_VERIFIED / WINDOWS_PASS，2026-09-21）：**`windows-release.yml` 的 checkout 绑定 `ref: ${{ inputs.release_tag || github.ref }}`，并执行 tag 格式、dispatch ref、tag SHA、checked-out HEAD 和 push SHA parity 检查；Native Host 步骤调用 `extension-identity.mjs verify`；repository-dependencies 与 full bundle 的 host manifest 统一由 `desktop/scripts/native-host-manifest-cli.mjs` 生成。`v0.2.0-pre.7` tag-push run `35570021396` 已通过该门禁。
- **私钥与备份（B0 未完成部分）：**私钥 `$HOME/xarchive-extension.pem`（`0600`）保持在仓库外；`.gitignore` 已排除 `xarchive-extension*.pem` 与 `*.sops.json`；age+SOPS 备份与离线第二份备份尚未执行，因此不得宣称 identity 可恢复。
- **GitHub 配置：**`XARCHIVE_EXTENSION_ID` 已配置为 Repository Variable（值 `iaajefkoanbkleojofoadeakelihbjne`，2026-09-21）；workflow 读取 `vars.XARCHIVE_EXTENSION_ID || secrets.XARCHIVE_EXTENSION_ID`，Repository Secret 保持未设置以避免双事实来源。配置方法与商店发布边界见 [`setup.md`](setup.md)「配置 Extension 发布身份」。
- **Release 污染：**`v0.2.0-pre.6` 页面上的两个资产来自 `main` 的手动 run `35518832674`（application-only），不代表 tag/source 或 Extension/Native Host 发布；2026-09-21 已在 Release notes 追加来源警告（未删除资产，因 `.7z` 已有下载记录），删除命令保留在 queue `WQ-REL-03`。`pre.5`、`pre.6` 都不再作为发布基线。
- **2026-09-21 pre.6 重试结果：**以 `--ref v0.2.0-pre.6`、`release_tag=v0.2.0-pre.6` 实际触发 run `35567742785`；source 为 tag commit `ac586e609337947aeb51de8f5cce3185efc8995e`。旧 tag workflow 的 Rust check、Rust tests 和 Tauri executable 通过，但在旧 `Build Native Messaging Host` 步骤因只读取未配置的 `secrets.XARCHIVE_EXTENSION_ID` 而失败；worker、打包、artifact 和 Release 上传均跳过，Release 资产未改变。该 run 不验证当前工作区 workflow，`pre.6` 仍不能作为完整发布基线。
- **B7 Extension packaging（LINUX_VERIFIED / WINDOWS_PASS，2026-09-21）：**新增 `desktop/scripts/extension-package.mjs` 生成并验证 Extension ZIP inventory；新增 `desktop/scripts/release-manifest-cli.mjs` 在 Windows runner 上对 `.exe`、两类 portable `.7z`、repository-dependencies `.7z` 和 Extension ZIP 计算 SHA-256/size，生成 JSON release manifest 与 `SHA256SUMS`，并在任何 artifact/Release 上传前调用 `release-assets.mjs` 做五资产门禁。Linux Desktop tests 62/62 通过；tag-push run `35570021396` 完成 Windows 构建、五资产打包、metadata 校验和 Release 上传；下载后的独立 SHA-256/size 对照通过。
- **仍待实现：**E5 Windows Named Pipe Desktop transport、E6 HKCU Registry lifecycle、E7 实时连接状态，以及 Windows Sidecar v2 handshake 的真实浏览器链路回归。`v0.2.0-pre.7` 已完成 Windows package/release scope，但不声明 Named Pipe、Registry 或真实 Edge/Chrome 归档链路完成。

### 2026-09-20 pre-release UI / Extension / Native Host 修复批次

本批次针对 XArchive Windows pre-release 的主页、设置页、浏览器 Extension 状态和 Native Messaging Host 链路。当前状态为 `IN_PROGRESS`；以下内容是实施范围和已确认问题，不代表功能已经完成或已通过 Windows 验证。

- **UI polish（LINUX_VERIFIED）：**已调整主页首次使用说明、下载目录设置区块、运行环境卡、侧栏设置/服务状态间距；设置页主要区块已改为分隔线布局，同时保留工作台和必要的内层控件边界。真实 WebView2/DPI/键盘视觉仍在 Windows queue。
- **Icon/path interaction（LINUX_VERIFIED）：**已修复 Extension globe SVG 路径；归档目录、日志目录、aria2、gallery-dl 和 Extension 目录已统一使用可复制路径控件。WebView2 剪贴板、中文/空格/长路径和 GUI 键盘行为仍需 Windows 验证。
- **Extension status（LINUX_VERIFIED）：**前端 checking 生命周期已与 `files_ready`、`browser_connection`、`native_host` 分离；`not_loaded` 不再显示为“检测中…”；Extension 区块已增加局部刷新和明确的文件缺失/Host 未注册/未连接映射。Desktop 目前仍缺少真实浏览器 transport session 的 Windows 检测证据。
- **Native Host packaging（LINUX_VERIFIED）：**Full portable/release packaging 已加入 Native Host executable、host manifest、Extension ID 必填校验和 `allowed_origins` 生成；Native Host Rust contract 8/8、Desktop Node 46/46 通过。当前用户 Registry 注册/修复/取消注册和 Windows Named Pipe 尚未实现，浏览器报 “Specified native messaging host not found” 仍可能发生。
- **Native connection（LINUX_VERIFIED，Windows pending）：**Extension `NativeBridge` 已处理同步连接失败、`runtime.lastError`、pending request 拒绝、失效 port 和下一次请求重连；真实 Edge/Chrome、Registry、Named Pipe 和 connected 状态仍进入 Windows queue。

本批次开发顺序：UI 布局与路径控件 → Extension 状态/刷新 → Native Host packaging/registration → Extension/transport reconnect → Linux targeted verification → Windows 集中验证。不得把 Linux PASS 或静态 manifest 测试写成 Windows Native Host PASS。

### 2026-09-20 Windows 结果 reconciliation

最新 Windows current-dirty 结果已读取并作为新的事实输入。当前 Plan 不再把旧 WDIO 结果视为可沿用的 PASS：由于本轮 UI diff 直接命中 native render 影响区，`WQ-P1-16/WQ-P1-17` 当前为 `WINDOWS_FAIL`；ordinary/advanced session 建立后均未渲染 Dashboard `h1`。现有日志位于 Windows 验证副本的 `validation-artifacts/current-dirty-20260920/`，根因仍未被证据确定为业务 UI、E2E feature injection、asset loading 或机器级 WebView2 状态。

Linux 侧本轮没有可确认的业务代码失败，不修改 UI 或 Native Host 代码以猜测修复 Windows native session。Native Host packaging 的 Windows 结果仅限 synthetic Extension ID 的本地 package boundary；真实 Extension ID、Registry/ACL、Edge/Chrome、Named Pipe、reconnect 和 release asset parity 仍未验证。下一步 Linux 工作是完成适用回归和文档收口；只有出现可复现 Linux failure 才新增实现。

### 2026-09-20 incremental Windows revalidation reconciliation

最新增量 Windows 验证使用同一 HEAD `bd3e58ddf064ab015a3c04036086a01a871062e6` 和同一业务影响区；新增变化仅为 Linux reconciliation 文档，没有新的业务代码、依赖、工具链或 Windows 影响区修改。Node check/test/build、Rust fmt/check、Native Host 8/8 和 package/worker 的既有证据继续有效；ordinary/advanced native WDIO failure 继续保持 `WINDOWS_FAIL`，没有重复执行相同 GUI 场景。Registry、Edge/Chrome、Named Pipe/reconnect、真实 release identity 和最终 asset parity 继续保持 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_BLOCKED` 或 `NOT RUN`。

1. 当前 R1 Linux-only contract validation 已完成：纯 Rust executor model、JobPersistence、Database factory、archive submit/query 对照、JobSummary projection、lifecycle event mapping、cancel/shutdown/recovery/completion、commit recovery facts/actions、批量 mixed recovery 和 SQLite 事件顺序均已完成并通过 Linux 验证。
2. 当前生产 executor integration 已完成 Linux 阶段二主体，并完成第一批入口调度统一：execution spec persistence、runner-owned resource creation、单 active runner、attempt fencing、spec-driven execution、独立 orchestration thread、调度失败补偿、运行中 cancellation、filesystem facts/action 和 startup recovery scan 均已接入；同步 `archive_tweet` fallback 仍保留，下一批处理用户入口退役和 cancellation/recovery 语义收口。
3. 阶段三的 Linux transport endpoint 已完成注册和 production scheduling 接入：Desktop 在 Linux/Unix 上启动 Unix domain socket transport endpoint，Native Host 在 Linux 上改用 `UnixStream` 连接；每个连接由独立线程处理，打开独立 SQLite persistence context，复用现有 `BrowserTransportAdapter` 完成请求校验、request_id 保留、Job submit-and-schedule、状态查询和错误映射。各 transport 回归测试（request_id 保留、重复 Job、状态查询、无匹配 Job、非法 Tweet URL 和非法协议版本）已通过。当前仍保留同步 `archive_tweet` fallback；Windows Named Pipe/ACL 仍需平台实现和实机验证。
4. R2 尚未进入可安全接线状态：下一 Linux 批次必须先定义 fresh media URL/403 refresh contract，再接入 aria2 backend、transfer polling/completion 和 Job event/state 提交；Windows 的 aria2 artifact、路径和进程验证在该批次完成后再按影响区重验。
5. Windows 自动化基线配置已实现：WDIO native smoke 可在已生成 Tauri release artifact 的 Windows 工作副本运行；真实 WebView2/DPI/键盘/辅助技术、应用 IPC 和 Native Host 仍需按队列验证。

## 验证状态

- Linux Rust `fmt/check/test/clippy` 已通过最终收口验证；`xarchive-desktop` 当前 80 tests、`xarchive-core` 12、`xarchive-native-host` 8、`xarchive-protocol` 11、`xarchive-sidecar-supervisor` 4、`xarchive-storage` 25、`xarchive-telegram` 12，workspace tests 全部通过。Desktop transport server（Unix domain socket）已接入 Desktop runtime，Native Host 在 Linux 上改用 `UnixStream::connect`。
- Desktop transport server 行为：Linux/Unix 下 Desktop 启动 Unix domain socket endpoint，Native Host 以 `XARCHIVE_PIPE_ENDPOINT` 配置连接；每个连接由独立线程处理，打开独立 SQLite persistence context，复用现有 `BrowserTransportAdapter` 完成请求校验、request_id 保留和错误映射。Windows 下 Native Host 仍保留 `OpenOptions` 文件打开路径，Desktop 不注册 Named Pipe server。
- **Windows WDIO 当前状态：**普通 release 的静态 capability/guest-JS 隔离检查和历史 spec 子项结果保留在 `windows-validation.md`；但最新 current-dirty UI/Extension 修改命中影响区，ordinary/advanced native session 均未渲染 Dashboard `h1`，因此当前 WQ-P1-16/WQ-P1-17 为 `WINDOWS_FAIL`，不能沿用旧 KEEP_VALID 或历史 spec PASS。
- tauri-plugin-wdio 1.4.0 已完成 Linux 配置：可选 wdio-e2e Rust feature、专用 E2E 注册、独立 wdio capability、withGlobalTauri、条件 guest JS 导入、高级 E2E spec 和 `wdio-tauri-service.mjs` worker 适配。Linux Rust 与 Node 门禁已在当前 Linux 环境通过；Windows session/driver 生命周期重验仍未完成。
- **Windows Node check/test/build、Extension tests、专用/普通 Tauri 构建和 Windows Rust fmt/check/test/clippy 的历史 PASS 只适用于各自记录的 source/revision；最新 current-dirty UI/Extension 验证中 ordinary/advanced native render 失败，当前不能把历史 spec PASS 外推为当前 WQ-P1-16/WQ-P1-17 PASS。
- Python `compileall`、Sidecar pytest 12/12 和 JSON Schema parse 已通过。
- Linux `cargo clippy --workspace --all-targets -- -D warnings`：`PASS`；已安装当前 stable toolchain 的 `clippy` component，版本为 `clippy 0.1.98 (88d9e12ae1 2026-08-18)`。
- **2026-09-20 incremental reconciliation verification：**Desktop Node tests `46/46`、Extension tests `10/10`、Desktop Vite check/build、Native Host `cargo check` 与 tests `8/8`、Rust fmt、portable/native-host script syntax 和 `git diff --check` 均通过；本轮没有新增业务代码修改。
- Python `.venv/bin/python -m pytest sidecar/tests -q`：`PASS`，12 passed；使用仓库根目录 `.venv`、editable `sidecar` 安装和 pytest 9.1.1。`compileall` 同时通过。
- 本轮 recovery/cancellation 增量：storage `24` 个单测通过，新增 staging metadata recovery 场景通过；Desktop workspace 测试 `64` 个通过，包含运行中 cancel、late-result fencing 和 Browser transport contract，`cargo check` 与 clippy 通过。
- Tauri MCP Bridge 已配置为 Debug-only Rust 依赖并固定绑定 `127.0.0.1`；Library 入口通过 `#[cfg(debug_assertions)]` shadowing 注册，Release 构建不再产生 `unused_mut` warning。MCP server（`@hypothesi/tauri-mcp-server`）属于 Agent 环境工具，不提交到项目 `package.json`。
- 最新 Windows reconciliation（2026-09-16，HEAD `cb1e581`，working tree clean、SHA-256 `24/24` 同步匹配）：Windows Node/Rust/Sidecar 门禁、普通/专用 Tauri build、Debug startup、便携 artifact 组装与首启 SQLite/log 初始化 smoke 均 `PASS`；WQ-P1-17 advanced 与 WQ-P1-16 ordinary 的 native session、Dashboard `2/2`、plugin API/mock/restore 为 spec 级 `PASS`，但两次成功退出后 `tauri-driver`/`msedgedriver` 与 4444/4445 仍残留，故整体不能判 PASS。GUI/Computer Use 仍受自动化前置阻塞；便携交互 setup、Downloads fallback、跨卷提交、sidecar/Extension 分发、文件权限和日志轮转保持 `WINDOWS_VERIFICATION_PENDING`。
- Linux teardown 修复（2026-09-16）：根因为 `@wdio/native-core` 的 `DriverProcess.stop()` 只 kill 直接子进程、无进程树清理；`wdio-tauri-service.mjs` launcher 现在在上游 teardown 前快照 driver PID 与 4444/4445 端口占用者，teardown 后对幸存进程执行进程树 kill（Windows `taskkill /T /F`、POSIX `SIGKILL`），无法清理时使运行失败。新增 `desktop/test/wdio-tauri-service.test.mjs`（`node --test` 8/8 通过），Linux Node 门禁 check/test/build 通过。WQ-P1-16/WQ-P1-17 修复后回到 `WINDOWS_VERIFICATION_PENDING`，Windows 重验不得依赖手工 `Stop-Process`。
- Linux teardown 修复第二轮（2026-09-16）：Windows 复验显示 spec 全过且进程/端口事后均已干净，但 hook 的固定 alive-check 窗口在 Windows 误报（`taskkill /F` 成功后 `kill(0)` 仍判已终止 PID 存活），且 `killTree` 测试因 exit 事件监听挂晚在 Windows 失败（`7 passed, 1 failed`）。判定改为 exit 事件 → 轮询（10s）→ 超时后以 tracked driver 端口 LISTEN 状态最终仲裁；测试监听时机已修正。WQ-P1-16/WQ-P1-17 保持 `WINDOWS_VERIFICATION_PENDING`。
- Windows 专属项目不得因 Linux 通过而标记为 Windows PASS；当前队列以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准，历史证据以 Windows 验证记录为准。
- 2026-09-29 WDIO 解阻移植（Linux，已随 `589142f` 提交）：新增 `desktop/scripts/patch-wdio-tauri-service.mjs`（根 `postinstall` + desktop `pretest:e2e*`，已安装树实测 patch 4 文件并幂等）修复上游 banner 正则与含空格路径 spawn；`wdio.conf.mjs` 新增 `TAURI_DRIVER_PATH`/`EDGEDRIVER_PATH`/`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`/`WDIO_AUTO_DOWNLOAD_EDGE_DRIVER=0`（默认关闭）与 `outputDir=logDir`；launcher 快照实际分配的 driver 端口。合并后 readiness 实现为 `desktop/e2e/support/native-startup.mjs`（session 快照 + `waitForApplicationDocument` + 失败取证，18/18），删除 `startup-diagnostics.mjs`。Desktop Node suite `84/84` PASS（`npm test --workspace desktop`）。`sidecar.rs`/schema/fixture 与远程分支字节相同、`main.js` 占位不采纳、前端 readiness 标记另立 batch。真实 session 结论仍待 Windows。
- 2026-09-29 Cross-platform batch 6（Linux 收口，Windows batch 5/6 reconcile）：Windows 交回四项全部完成——A `npm.cmd` spawn 修复收口为 `main()` + 直接执行守卫（`build-portable-windows.mjs`）；B `*.pyd` 移出全局打包排除清单（PyInstaller worker `_internal` 的 7 个运行时 C 扩展必须随包）+ 回归测试；C 版本来源统一到 Cargo workspace `0.1.1`（`tauri.conf.json`、根/desktop `package.json` 由 0.1.0 对齐，`package-manifest.json` 默认派生 `tauri.conf.json`，extension manifest 维持 0.1.0）+ 跨文件一致性测试；D WQ-ENG-09b-ORD reviewed recipe = `WDIO_DIRECT_DRIVER=1` 直连独立 msedgedriver（batch-2 步骤 4/5 形状），写入 `windows-wdio-handoff.md`。Linux 全量门禁：`node --test` 目标文件 31/31、根 `npm test` **91/91** + extension **13/13**、`npm run check`、`cargo fmt`/`cargo test`/strict clippy、pytest 19/19、`git diff --check` 全部 PASS。复验项 WQ-PACKAGE-FULL-01-R2（P0）、WQ-ENG-09b-ORD-R2（P1）、WQ-ENG-13-R2（P1，预期 91/91）、M11–M13 全部 `WINDOWS_VERIFICATION_PENDING`；下一 Owner：Windows Owner。

## 相关文档

- 未来方向：[`roadmap.md`](roadmap.md)
- 测试策略：[`testing.md`](testing.md)
- Windows 工作流：[`cross-platform-validation.md`](cross-platform-validation.md)
- Windows 执行规范：[`../validation/windows.md`](../validation/windows.md)

## Windows 复验后的 Linux 端当前动作

依据最新 Windows 复验，Linux 端已完成明确的 WDIO wrapper、spec API 和构建边界配置；以下两项仍需 Windows 重验确认：

1. **已完成**：`desktop/scripts/run-wdio-advanced.mjs` 不再直接对 `wdio.cmd` 使用 `spawnSync`，改为由当前 Node 进程加载 workspace WDIO CLI，并保留真实失败退出码。
2. **已完成**：`desktop/e2e/specs/wdio-plugin.e2e.mjs` 不再调用不存在的 `browser.tauri.isTauriApiAvailable`，改为通过 `browser.tauri.execute` 检查 `window.wdioTauri`。
3. **已完成**：普通 release 不加载 `@wdio/tauri-plugin` guest JS；仅 `VITE_WDIO_E2E=1` 的专用构建加载该 guest JS，并继续使用 `wdio-e2e` feature/capability。
4. **待 Windows 重验**：确认普通 release 不再产生 `plugin:wdio|execute not allowed by ACL`，并处理/确认 `@wdio/tauri-service` 的 sessionId、mock-store 和 driver teardown 生命周期；手工 Stop-Process 只能作为诊断清理，不能作为 PASS 条件。

- 当前 revision 的 Rust fmt/check、wdio-e2e feature check、workspace tests、strict Clippy、Node check/test/build、wrapper/spec/config syntax、WDIO config load 和 Sidecar pytest 10/10 均已在 Linux 现场通过。Windows 已按“专用构建/高级入口 → 普通构建/基础 smoke”重新同步并执行；匹配 driver 下载和 tauri-driver 启动成功，但两套 native session 均因 `DevToolsActivePort file doesn't exist` 失败，service adapter 的完整效果仍未被 Windows 原生 session 验收。完整证据见 windows-validation.md 最新章节。

## Linux 端当前剩余验证与改动（2026-09-15）

本节是当前行动清单，不是历史验证流水账。Linux Rust 与 Node 门禁已通过；剩余工作集中在 Windows 前置稳定后的 service adapter 重验，以及其他 Windows 专属队列项目。

| ID | 状态 | Linux 端要求 | 完成标准 |
|---|---|---|---|
| LINUX-WDIO-07 | DONE-LINUX / WINDOWS_REVALIDATION_PENDING | 新增 `desktop/scripts/wdio-tauri-service.mjs`；普通和高级入口均复用官方 launcher，但 worker 跳过依赖 `plugin:wdio` 的单窗口 focus probe | 普通 smoke 不再轮询不存在的 plugin；专用 artifact 仍可 execute/mock/log；不放宽普通 capability 或 guest JS；需 Windows 重验 |
| LINUX-WDIO-08 | DONE-LINUX / WINDOWS_REVALIDATION_PENDING | 适配层覆盖 `afterSession`，只在有效 session 存在时删除 session；高级 spec 保留显式 mock restore，避免 service 重复清理 | 不再由项目适配层产生 sessionId warning；应用、tauri-driver、msedgedriver 和相关端口自动清理仍需 Windows 实测确认 |
| LINUX-WDIO-09 | DONE-LINUX | 2026-09-18 Linux 使用 Ubuntu 26.04、Node v26.7.0/npm 11.19.0、Rust/Cargo 1.98.0、Python 3.14.4 完成 Node workspace check/test/build、Extension 7/7、Rust fmt/check/wdio-e2e feature check/workspace tests 150/150/strict Clippy、Sidecar pytest 10/10、普通/专用 Tauri build、wrapper/spec/config syntax 和 WDIO config load；安装发行版替代包 `webkitgtk-webdriver` 后 Linux Native WDIO Dashboard smoke 2/2 PASS | 结果来自 Linux 工具链本身；Browser Mode 在当前仓库无独立配置，记为 NOT APPLICABLE；该 PASS 不替代 Windows WebView2 验证 |
| LINUX-WDIO-10 | WINDOWS_VERIFICATION_PENDING | Linux 复核已完成并确认无新的 Linux 侧实现问题；Windows 本轮 driver 下载/tauri-driver 启动成功，但 advanced 与 ordinary native session 均因 `DevToolsActivePort file doesn't exist` 失败，失败路径还需手工清理 driver | 调查并稳定 Windows WebView2/Edge native session 和自动 teardown 后，按 advanced → teardown/cleanup → ordinary smoke 重验；两个队列项满足各自完整验收条件后才可改为 WINDOWS_PASS |

当前不需要在 Linux 端重复或修改的项目：Rust fmt/check/feature check/test/clippy、Node check/test/build、Python compile/pytest、普通/专用 Tauri build、tauri-plugin-wdio 的 Cargo feature/注册/capability、withGlobalTauri、条件 guest JS、wrapper、service adapter、plugin spec 和 WDIO config load 已有通过或完成证据。Linux Native WDIO 已在安装 `webkitgtk-webdriver` 后通过 Dashboard smoke；当前仓库无独立 Browser Mode 配置，记为 `NOT APPLICABLE`。`DevToolsActivePort`、Edge driver 下载、`uv_os_get_passwd returned ENOMEM` 属于 Windows 验证环境前置，不应通过本轮 Linux 配置“顺带解决”。Native Host、Named Pipe、真实 executor/transport IPC、Windows ACL/reparse/长路径、WebView2 accessibility、真实账号和 installer 仍属于 Windows 队列。

### 当前 Windows 重验结论（2026-09-15 17:20）

Linux 最新 working tree 已经经 /mnt/e 受控单向同步到 E:；Node/Rust 静态门禁与专用/普通 Tauri build 通过。普通和 advanced WDIO 均在 onPrepare 因匹配 Edge driver 下载失败、随后 tauri-driver code 1 退出而未进入 spec；历史结果记录为 FAIL，当前队列状态改为 WINDOWS_VERIFICATION_PENDING，等待 Windows 前置稳定后重验。Linux Node 已在后续 reconciliation 中补做并通过。详见 docs/development/windows-validation.md 最新章节。
### Windows WDIO 网络重试状态（2026-09-15 17:42）

已从 Microsoft 官方地址手动取得 Edge/WebView2 152.0.4191.66 对应 msedgedriver，并确认 tauri-driver 可启动；但 advanced 与 ordinary worker 均因 Windows Node 的 uv_os_get_passwd returned ENOMEM 在 spec 前失败，WQ-P1-16/WQ-P1-17 继续 WINDOWS_FAIL。手动停止残留 driver 仅是环境恢复，不代表自动 teardown 通过。详细证据见 windows-validation.md 最新章节。
### Windows driver 本地保存状态（2026-09-15）

msedgedriver 152.0.4191.66 已半永久保存于 E: 验证副本的 desktop/test-artifacts/msedgedriver/152.0.4191.66/，并在 setup.md、testing.md 和 windows-wdio-handoff.md 记录 PATH 使用方法。该目录不纳入 Git、不回写 Linux；使用它只能绕过自动下载网络前置，不能覆盖当前 Node worker ENOMEM、session、DOM 或自动 teardown 的失败结论。

2026-09-15 blocker recovery：已在 E: 验证副本中确认同版本 `msedgedriver 152.0.4191.66` 能独立启动，`tauri-driver` status 代理可用，当前 release app 可直接保持存活；随后对新 WebView2 profile、隔离 application identifier 和显式 native driver 路径做最小 session probe，均仍以 45 秒超时结束，未解决 `Chrome instance exited`/`DevToolsActivePort file doesn't exist`。失败路径的 driver 可人工停止但自动 teardown 仍无证据；Computer Use 仍因 `sky` trusted RPC/native target 不可用而 `BLOCKED_AUTOMATION`。未修改业务代码；WQ-P1-16/WQ-P1-17 继续待 Linux 测试基础设施和 Windows native session 条件处理后重验。

## 当前 Windows WDIO 重验结论（2026-09-15 20:18）

本轮 Linux dirty source 已受控同步到 E:，source/E: 关键文件 hash 一致，Windows 本地依赖、target、driver、test-artifacts、用户数据和其他 machine-local 目录保留。Windows `npm ci`、Node workspace check/test/build、Sidecar compileall/pytest 10/10、Rust fmt/check/feature check/test 150/150/strict Clippy、专用/普通 Tauri build、WDIO syntax/config load 和普通 artifact capability/guest-JS 隔离均通过。

WQ-P1-17 advanced 与 WQ-P1-16 ordinary 均为 `WINDOWS_FAIL`：driver 下载成功，tauri-driver 监听成功，但 WebView2 session 创建三次重试均以 `session not created: DevToolsActivePort file doesn't exist` 失败，spec 未执行。两条失败路径都留下 driver/端口，需要精确 PID 的手工清理；这不是自动 teardown PASS，也没有形成产品 DOM/API 失败证据。Computer Use native inventory 返回 `apps=[]` 且 `sky` 未配置，GUI/DPI/辅助技术项目为 `BLOCKED_AUTOMATION`。

Linux 后续只需处理 Windows WDIO 测试基础设施/环境跟进：调查 `DevToolsActivePort`/`Chrome instance exited`、临时 WebView2 user-data-dir、tauri-driver 生命周期和自动 cleanup；保持普通 release capability/guest-JS 安全边界，不修改业务代码以制造通过。真实账号、Named Pipe/Registry、应用级 SQLite/restart/recovery、ACL/reparse/长路径、externalBin/Tray/installer 等仍按 Windows queue 的 `BLOCKED` 或 `NOT RUN` 原因处理。

### 2026-09-22 latest Windows validation reconciliation

The latest current-source Windows run supersedes the earlier historical WDIO
failure entries for the current local scope: after clean npm ci, ordinary native
WDIO passed Dashboard 3/3 and advanced native WDIO passed Dashboard/plugin 5/5.
WQ-P1-16 and WQ-P1-17 are therefore WINDOWS_PASS for this controlled local
scope, with the service teardown survivor warning handled by the safety net and
no final process/port residue. The exact workflow tauri-drive
2.1.0-alpha.0, hosted/release-runner parity, manual setup, Registry, browser,
Named Pipe, real extraction and final release acceptance remain
WINDOWS_VERIFICATION_PENDING, WINDOWS_BLOCKED or NOT RUN as recorded in the
latest Windows validation and queue sections.
### 2026-09-22 exact pinned toolchain result

The workflow-pinned tauri-driver 2.1.0-alpha.0 was installed successfully on
the Windows machine. Its ordinary WDIO gate then failed during session creation
because EdgeDriver 152 supports only Edge 152 while the installed Edge is
154.0.4258.24. Advanced WDIO is blocked by that shared prerequisite. The local
v2.0.6 native WDIO result remains separate controlled-scope evidence and is not
a pinned workflow PASS.

### 2026-09-22 Linux banner-fix commit record

Linux 侧 banner 修复工作已提交为 `03332a1`（`fix: accept Microsoft Edge WebDriver banner in Tauri E2E harness`，18 files changed、+1231/-17），并推送到 `origin/feature/u7-desktop-production-integration`；本地与 remote HEAD 一致，working tree clean，`v0.2.0-pre.8` 仍指向 `90905f7`，tag 未移动。提交内容与 Windows 09-22 两轮验证所针对的 working tree 一致：验证后未修改任何业务代码、测试断言、工作流、依赖或 driver 版本，因此上述 09-22 本地范围结论继续适用于该 revision。

提交后 Linux 复核（同一 revision）：Desktop `npm run test --workspace desktop` `82/82`、Extension check/test `21/21`、Desktop Vite `check`（production build）通过、`git diff --check` 通过。本提交未触碰 `crates/`、`desktop/src-tauri/` 或业务前端代码，Rust fmt/check/tests/strict Clippy 沿用本批次先前记录。

下一轮 Windows 工作（保持现状）：以 `03332a1` 作为同步 revision 基线；WQ-P1-16 的 clean-install 本地 v2.0.6 scope 为 `WINDOWS_PASS`，exact pinned-toolchain 尝试为 `WINDOWS_FAIL`；WQ-P1-17 本地 scope 为 `WINDOWS_PASS`，pinned toolchain 为 `WINDOWS_BLOCKED`。在 pinned `msedgedriver 152.0.4191.66` 与实机 Edge/WebView2 版本对齐（或提供受控 Edge 152 runtime）之前，release readiness gate 不得记为 PASS。

### 2026-09-22 v0.2.0-pre.10 发布状态

`v0.2.0-pre.9` 因 readiness gate 脚本重复 `New-Item` 缺陷作废（tag `4578bb8` 保留，Release 转 draft），顺延为 `v0.2.0-pre.10`（commit `4bd0666`，含幂等修复）。Linux Pre-Release run `35698596565` 通过并创建 tag 与 pre-release；Windows Release Build run `35699308051` 三次执行：首次为已知的 `xarchive-sidecar-supervisor` handshake 瞬态失败，两次重跑均推进到最终 WDIO gate——pinned 工具链（tauri-driver 2.1.0-alpha.0 + msedgedriver 152.0.4191.66 与 WebView2 152.0.4191.66 配对、banner 接受、session 创建）全部通过，但 gate 附着的 WebView2 文档在 20 秒内始终停留在空白初始文档 `url:"data:,"`（`rootExists:false`），Dashboard spec 两次以相同证据失败，属于 hosted runner 上的确定性问题。

因此 `v0.2.0-pre.10` 按契约保持 **零资产** 状态：GitHub Release（pre-release，完整 notes）已发布，但无 exe/7z/Extension/manifest。后续为测试基础设施跟进（诊断收集截图/应用日志/窗口目标、按启动信号等待、延长预算、受控本地复现同 exe），不改产品代码；下一个发布尝试须使用新 tag，不得复用 `v0.2.0-pre.10`，也不得向其补传资产。

### 2026-09-22 readiness gate 修复计划登记（当前工作项）

pre.10 hosted gate 的 `data:,` 空白文档失败定性为「target 选择/应用首次导航不可区分」类阻塞。修复计划已登记：Phase 0–9 见 `docs/development/roadmap.md`；Windows 队列新增 `WQ-P0-WHITE-01A/01B/01C/01D` 与 `WQ-P0-WHITE-03R`（`docs/validation/windows-queue.md`）；执行步骤见 `docs/validation/windows-wdio-handoff.md`。

本轮 Linux 工作范围：`desktop/e2e/support/native-startup.mjs`（handle 枚举、应用文档识别、启动契约等待、失败证据收集）、`desktop/e2e/specs/dashboard.e2e.mjs` 改造、`desktop/test/native-startup.test.mjs`、`.github/workflows/windows-release.yml` 诊断与 preflight/gate 隔离增强、新增 hosted readiness diagnostic workflow、repository-map 登记。约束不变：不弱化断言、不改产品代码、`v0.2.0-pre.10` 保持零资产；Linux 验证完成后统一进入 Windows 验证阶段。
### 2026-09-22 d3fd814 readiness-gate Windows result

The latest readiness-gate working tree passed Linux Desktop 89/89, Extension
21/21, Vite check/build, syntax and diff checks. Windows npm ci, Node regression,
Tauri release build and direct executable preflight passed. The exact pinned
local gate reached tauri-driver 2.1.0-alpha.0 but failed before session creation
because EdgeDriver 152 does not support installed Edge 154.0.4258.24.
WQ-P0-WHITE-01B is FAIL; target discovery, failure evidence and session-start
snapshot are BLOCKED; hosted stability is NOT RUN. Preflight/gate isolation and
the WDIO log-directory contract also failed on this machine. No business code
was modified.
### 2026-09-22 Edge 154 local diagnostic result

EdgeDriver 154.0.4258.24 is retained in the E: validation project at
E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\msedgedriver-154.0.4258.24\msedgedriver.exe.
Version and SHA-256 verification passed, and WDIO confirmed an exact match with
Edge 154. The subsequent E2E run still failed because the WebDriver session
remained on data:, for 30 seconds and never exposed the XArchive application
document (0 passed, 1 failed). The evidence is diagnostic for Windows native
Tauri startup and target attachment; it does not alter the workflow's pinned
Edge 152 status. No business code was modified.

### 2026-09-22 current-source Windows validation rerun

The latest Linux dirty source was synchronized to E:\Shiraishi\VSCode Workspace\Tw2Tg
with 0 mismatches and 0 failed files. Linux and Windows dependency/static
regressions, workspace tests, Vite build and Windows Tauri release build passed.
The diagnostic EdgeDriver 154.0.4258.24 matched Edge 154 and tauri-driver was
ready, but native E2E failed at application-document discovery: the session
remained on data:, for 30000 ms with ONLY_BLANK_DOCUMENTS and rootExists false.
Failure evidence was captured; the WDIO log-directory and session-start snapshot
contracts still failed. Advanced native E2E is BLOCKED by the shared startup
prerequisite. Follow-up is Windows runtime/target attachment and test-harness
diagnostics, not a business-code change.

### 2026-09-22 current-source Windows revalidation after diagnostics fixes

本轮以 Linux HEAD `d3fd81459ce136a88080362e75a9b657f1d68ecc` 加 working-tree
changes 为源，单向同步到 `E:\Shiraishi\VSCode Workspace\Tw2Tg`。Windows
Desktop 单元测试 91/91、preflight、preflight→gate 隔离、WDIO 非空日志目录
契约和 session-start 快照均 PASS。普通 native E2E 仍 FAIL：EdgeDriver 154
成功创建 session，但目标始终停留在 `data:,`，30000 ms 内没有 XArchive 文档，
`ONLY_BLANK_DOCUMENTS` 且 `rootExists:false`。Advanced native E2E 为 BLOCKED，
hosted/release-runner 为 NOT RUN；本轮未修改业务代码。

需要 Linux 后续处理：调查 Windows Edge 154 / WebView2 Runtime 153 的版本配对
与 tauri-driver target attachment，或确认普通 release artifact 的窗口创建与
导航路径；在 01A 解决并完成 hosted diagnostic 前，不得宣称 Windows UI
readiness 或 release packaging acceptance。
