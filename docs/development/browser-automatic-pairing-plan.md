# Browser Extension 自动配对开发计划

Owner: Cross-platform Owner（共享协议、ticket、Extension、跨平台测试）；Windows Platform Owner（Native Host / Named Pipe Windows 接线、注册、冷启动、GUI、打包与 Windows 验收）。

Status: `READY_FOR_WINDOWS` — Shared Phase 1–3 implementation is present; integrated Windows code and Full installation inventory fix are Linux-validated in this batch. The exact Windows return tested `6060f5a`, not the integrated source: it recorded `BOOTSTRAP_NOT_IMPLEMENTED` and a missing manifest entry. Formal Windows test source commit is `9920566ef1df115effc3ab5df5d9a12fd42210ed`; reconciled docs are pushed on the batch branch tip. Windows build/runtime, registration, UI, browser, lifecycle, archive, and install acceptance remain pending. Prior browser automation was BLOCKED (`COMPUTER_USE_UNAVAILABLE`). See [current handoff](../status/platform-handoff.md), [Windows queue](../validation/windows-queue.md), and [manual steps](../validation/windows-manual-steps.md#l-browser-automatic-pairing--current-integrated-source-handoff-pending). Linux tests do not constitute Windows acceptance.

## 1. 目标与范围

使已安装并正确注册的 Edge / Chrome Extension 与同机 XArchive Desktop 自动发现并建立已认证连接，日常流程不要求用户输入端口、复制或粘贴 token。保留 WebSocket 作为持续业务数据通道；Native Messaging 负责 bootstrap/discovery，Named Pipe 连接 Native Host 与 Desktop。

```text
Extension service worker
  ── Native Messaging: bootstrap ──> Native Host
  ── current-user Named Pipe ──────> Desktop bridge coordinator
  <── endpoint + short-lived ticket ─
  ── loopback WebSocket + ticket ──> Desktop BrowserTransportAdapter
```

已有 WebSocket、Native Messaging framing/forwarding、Windows Named Pipe、Native Host 注册和固定 Extension identity 均应复用，不重建。业务仍复用 `BrowserRequest` / `BrowserResponse`，不迁移 Sidecar 或媒体链路。

明确非目标：本计划不移除或重写 WebSocket；不新增通用 RPC 框架；不扫描 localhost 端口；不在 Extension 内置长期 secret；不支持远程配对；不宣称安装扩展即可免除 Native Host 安装/注册；不以自动配对实现替代 Windows 连接断开调查。

## 2. 基线事实与未解决问题

代码基线检查发现：

- Extension 默认使用端口 `17321`，把 `enabled/port/token` 存入 `chrome.storage.local`。
- Desktop WebSocket 绑定 `127.0.0.1`，可由环境变量固定端口；认证 token 可由环境变量指定，否则启动时随机生成，但当前 WebSocket 仍复用同一 token。
- WebSocket 首条应用消息认证和认证超时已经存在；当前检查未发现 Origin 白名单验证、ticket TTL/单次消费或动态端口 bootstrap。
- Native Host 当前转发归档/查询业务消息；没有 bootstrap 控制消息和 Desktop 冷启动协调。
- Windows Named Pipe listener、当前用户安全描述符和 Edge/Chrome Native Host 当前用户注册代码已存在；仍须由 Windows Owner 验证权限及运行行为。
- Extension 运行时消息入口已有归档和管理操作；sender/origin 分层校验仍需审计和测试。
- HEAD `b015fbe` 修复 Extension/Desktop 的 live connection status 语义，但认证后连接持续性、空闲行为及 MV3 影响仍未由受控 Windows 试验证明。

以上是基线代码检查结果，不等于 Windows 实机验证。现有故障调查与自动配对实现分别记录，不能把架构升级宣称为连接故障修复。

## 3. 目标安全与兼容契约

### 3.1 Bootstrap 与 ticket

- Bootstrap 经 Native Messaging 到达 Host，再经 Named Pipe 请求 Desktop；Host 不自行生成或长期保存 WebSocket secret。
- Desktop 默认将 WebSocket listener 绑定 `127.0.0.1:0`，取得端口后才允许签发 bootstrap 响应。仅保留明确的诊断/测试覆盖入口；生产默认不得依赖固定端口。
- Desktop 生成 256-bit CSPRNG ticket，仅存内存；当前共享契约固定为 64 字符小写十六进制、`expires_in_ms` 取值 1–30,000 ms；运行时默认 TTL 为 30 秒；原子单次消费，竞争连接最多一个成功。
- ticket 绑定当前 Desktop listener/runtime generation 和预期 Extension origin；listener/应用重启后旧 ticket 失效。
- ticket 不落盘、不进扩展持久 storage、不写 URL、日志、错误文本或诊断导出。扩展仅在内存中暂存，重连时重新 bootstrap。
- `runtime_instance_id` 是诊断/代际标识，不是 secret；如引入安装 ID，也不得当作密码。

### 3.2 WebSocket 接受与认证

- 只允许 loopback；WebSocket 握手必须校验精确的允许 Origin。Origin 是纵深防御，不替代 ticket。
- 建连后的第一条应用消息必须是 bootstrap 返回的 ticket 认证；不得先处理业务请求。
- 认证预算建议 3 秒；过期、错误、重放、并发二次消费及版本不兼容均明确拒绝并关闭连接。
- 对未认证连接、并发数、帧/消息大小、认证频率及握手占用设定有界策略；认证读超时必须是整体 deadline，不能被慢速分片无限延长。
- 认证成功后继续使用既有业务协议和 `request_id` 语义；认证失败不得静默降级到 Native 业务转发。
- Native Messaging 业务回退若继续保留，必须作为显式兼容模式；不能在错误 Origin、错误 ticket 或协议错误后自动绕过失败。

### 3.3 Extension 权限和生命周期

- 管理消息（保存设置、重连、诊断）仅允许受信任 Extension 页面；内容脚本只允许白名单归档/状态请求。
- 校验 `sender.id`、`sender.url`/origin、消息类型及 payload；不可相信消息体自报来源。
- 并发调用共享单一 bootstrap/connect 尝试；以 generation 防止旧 socket 回调覆盖新状态；断开时拒绝/清理 pending。
- 启动、状态恢复和下一次业务请求都可触发有限退避的重连；每次重连重新发现端口并获取新 ticket。
- WebSocket open 不等同 authenticated；状态只反映 live authenticated socket。
- 评估约 20 秒应用层心跳是否为持续在线目标所需；必须同时支持 worker 被终止后的重新 bootstrap，不依赖定时器保活作为唯一恢复机制。
- 归档请求若响应丢失，不可盲目跨通道重放；使用现有幂等/状态查询语义恢复。

### 3.4 Desktop 冷启动

Windows Owner 评估 Native Host 在收到可信 bootstrap 时是否启动 Desktop。若实现：可执行文件路径必须来自受信任安装/注册信息；不得接受 Extension 提供的任意路径/参数；须有单实例协调、有界等待、启动失败诊断和用户可控策略。后台重试不得无上限启动桌面进程。此项为可独立验收的 Windows 子功能，不得阻塞共享协议测试。

## 4. 实施阶段、Owner 与交付

### Phase 0 — 基线调查和验收条目冻结

**Owner：Cross-platform Owner；Windows Owner 在其验证批次执行实机观察。**

- 以 `b015fbe` 和明确产物 SHA 绑定现有认证后断连调查。
- Windows 记录成功认证后的即时、约 5 秒、约 45 秒双端状态及诊断计数；分别测试 Desktop 重启、浏览器重启及 worker 重建；不从累计关闭计数推断持续时长。
- 如诊断不足，先补脱敏 connection/generation、连接关闭原因、close code、活动时间数据；分辨真实断连与状态误报。
- 不因本阶段观察结果而未经复现修改产品代码；共享契约/架构问题按 `CROSS_PLATFORM_CHANGE_REQUIRED`，保留既有抽象的小改动按 `CROSS_PLATFORM_REVIEW_REQUIRED` 路由。

### Phase 1 — 共享 bootstrap/ticket 契约（契约类型与初始 Schema 已实现；跨语言完整消费待后续阶段）

**Owner：Cross-platform Owner。**

- 在 `crates/xarchive-protocol` 建立版本化 bootstrap request/response、认证和错误类型，并建立机器可验证 schema/fixture（若 schema 目录的现有约定适用）。
- 明确 Host 与 Desktop pipe 控制消息和既有业务 Browser 消息的区分，避免把 bootstrap 反序列化成归档请求。
- 决定未知字段、版本协商、最大字段/响应大小、错误码及可重试分类。
- 添加正反例契约测试；记录与旧 Extension/Desktop 的兼容窗口和升级失败文案。

**验收：**契约可被 Rust、Host、Extension 独立消费；错误 Origin/协议版本和不完整响应均有确定结果。Rust 类型/校验、初始 Schema 及 Host/Extension/WebSocket 运行时消费已在本批次实现；Windows 集成源码验收待执行（见 §7）。

### Phase 2 — Desktop 动态 endpoint、ticket store 与 WebSocket 强化

**Owner：Cross-platform Owner；Windows-only 适配留给 Windows Owner。**

- 默认随机 loopback 端口；将 listener endpoint、runtime generation、ticket store 纳入 Desktop Runtime 生命周期及 executor/listener replacement。
- 实现内存 ticket 签发、TTL、原子单次消费、generation 绑定、并发安全及速率/容量限制。
- 添加精确 Origin 检查、ticket 首帧认证、整体认证 deadline、消息和连接上限；保留 `tungstenite`，不自制 WebSocket 协议。
- shutdown/replacement 正确取消旧连接并清理票据；阻塞 I/O 不得卡住 Runtime shutdown；状态仅依据活跃认证 socket。
- 覆盖错误/过期/重放/并发消费、错误 Origin、首帧业务、超限帧、慢速客户端、响应丢失、旧 generation、重启失效及线程资源边界测试。

**验收：**未经认证无法进入 `BrowserTransportAdapter`；单次 ticket 最多建立一条连接；Rust targeted tests、fmt、Clippy 通过。

### Phase 3 — Native bootstrap IPC 和 Extension 自动状态机

**Owner：Cross-platform Owner；Windows Named Pipe 具体客户端接线归 Windows Owner。**

- Native Host 控制消息通过抽象 IPC 请求当前 Desktop bootstrap 信息；业务请求转发契约不混淆。
- Extension `NativeBridge` 增加可复用的 bootstrap 请求/响应处理和超时/断连清理。
- WebSocket settings 移除生产手动 token/端口作为自动连接必需条件；旧配置采用显式迁移，成功进入自动模式后清除旧 token，不静默降低认证。
- 建立单飞连接、generation 防护、每次重连重新 bootstrap、pending cleanup、退避上限和可诊断错误状态。
- 增加可信 sender 校验及消息 allowlist；确保 content script 不能调用管理接口或构造任意本地命令。
- 测试同时触发的连接请求、worker 重建、超时、Host 不存在、Desktop 未就绪、旧 callback、重复归档响应不确定等场景。
- 决定持续连接的心跳策略，并通过显式状态机测试验证 idle/恢复逻辑，不假定心跳等同 worker 永久存活。

**验收：**测试环境内无需 storage 中预置 token/端口即可自动 bootstrap；用户状态显示与 live socket 一致；业务协议无变更。

### Phase 4 — Windows Native Host/Named Pipe、注册和冷启动

**Owner：Windows Platform Owner。**

- Host 检查浏览器调用方 origin，并仅对预期扩展接受控制请求。
- 实现 Named Pipe bootstrap 控制请求及 response framing；审查当前用户 ACL、Pipe 命名抢占、多用户会话和本机边界。
- 分别验收 Edge、Chrome 当前用户 Native Host manifest/Registry、固定扩展 ID、安装/升级/修复/卸载和便携目录变化。
- 冷启动按第 3.4 节设计独立实现与测试；不能使用调用方提供的任意程序路径。
- 记录 Windows OS、浏览器版本/profile、Desktop/Host/Extension source revision 和产物哈希。

**验收：**正式 Git 产物下 Desktop 已运行和允许冷启动两个场景均能 bootstrap；注册缺失和启动失败可诊断。Linux 测试不替代本阶段验收。

### Phase 5 — UI、升级兼容与打包

**Owner：共享状态语义由 Cross-platform Owner；Windows UI/注册/打包由 Windows Platform Owner。**

- Popup/options 显示自动连接、正在发现、未安装/未注册、Desktop 未启动、版本不兼容、恢复中等真实状态。
- 移除常规 token 复制/粘贴路径；仅在经批准的诊断兼容模式下保留旧配置入口。
- 定义旧 Extension + 新 Desktop、新 Extension + 旧 Desktop 的兼容矩阵；认证失败不得自动转为无票据连接。
- release/package inventory 纳入 Host manifest 和相关 assets；对齐 Extension 公钥派生 ID、`allowed_origins`、Host 路径及协议版本。

**验收：**安装/注册完成后普通用户无需输入 Token；不兼容、安装缺失、Host 不可用和 ticket 失败分别显示；更新或回滚不留下明文凭据。

### Phase 6 — Windows 集中验证与交接

**Owner：Windows Platform Owner；共享缺陷由 Cross-platform Owner 回收。**

覆盖首次配对、Desktop 冷启动与失败、Edge/Chrome、两个浏览器 profile、浏览器/Desktop 重启、worker 生命周期、睡眠恢复、5s/45s/较长空闲、错误来源、ticket 过期/重放/并发、业务状态查询、真实归档和重复提交、升级/卸载/便携目录迁移、日志脱敏及资源限制。

每项按统一 validation policy 记录结果、artifact identity、环境、方法、命令/步骤、证据及限制。未执行保持 `NOT_RUN`；GUI automation 失败与产品缺陷分开。Windows 对共享行为的契约/架构发现用 `CROSS_PLATFORM_CHANGE_REQUIRED` 回到 Linux；保持既有抽象的小型共享实现修正使用 `CROSS_PLATFORM_REVIEW_REQUIRED`。

### Phase 7 — 后续 Telegram 批次

自动配对共享契约交付后，不必等待所有非阻塞 Windows GUI 项关闭即可继续共享工作：Cross-platform Owner 按 `telegram-local-bot-api-plan.md` 完成 Tauri commands、运行时 enqueue/claim loop、恢复和设置/任务状态共享模型；Windows Owner 负责 Credential Manager、实际 GUI、Local Bot API 部署及真实发送验收。自动配对与 Telegram 真实发送是不同验收目标。

## 5. 验证矩阵和完成标准

| ID | 目标 | 最低验收 |
|---|---|---|
| AUTO-PAIR-CONTRACT | schema 与兼容 | Rust/Extension/Host fixture 一致，非法版本/字段明确拒绝 |
| AUTO-PAIR-TICKET | 身份与重放防护 | TTL、单次、并发原子消费、listener generation 失效均有测试 |
| AUTO-PAIR-ORIGIN | 本地 WebSocket 边界 | loopback-only + 精确 origin；错误源不进入业务适配器 |
| AUTO-PAIR-EXT-LIFECYCLE | worker/重连 | 单飞 bootstrap、断连 pending 清理、worker 重建可恢复、无 stale authenticated |
| AUTO-PAIR-NATIVE-IPC | Native/Named Pipe | framing、ACL、冷启动决策和故障诊断在 Windows 有证据 |
| AUTO-PAIR-PACKAGE | 身份/注册/更新 | 固定 Extension ID 与 allowed origin 匹配，Edge/Chrome 安装注册链正确 |
| AUTO-PAIR-E2E | 实际业务 | 状态查询、获准 URL 归档、重复提交/响应丢失恢复均在 Windows 产物验证 |
| AUTO-PAIR-REDACTION | 密钥暴露 | ticket 不在配置、URL、日志、错误、Job payload 或诊断包 |

全部目标通过后才可宣称“同机自动配对”。Native Host 安装/注册仍是安装前提，不宣称扩展单独安装即可连桌面。

## 6. 优先级、依赖和风险

1. P0：共享控制协议、ticket、Origin 与自动 Extension 状态机。
2. P0：现有认证后断连 Windows 受控复验；它是独立问题，不作为共享开发的默认阻塞项。
3. P0：Windows Host/pipe/Registry/Edge/Chrome 集成及真实自动配对。
4. P1：冷启动、升级兼容、UI 文案和长时稳定性。
5. 后续独立批次：Telegram Runtime/Tauri/UI 与 Windows 真实发送。

关键风险：Native Messaging origin 不能替代 Desktop ticket；WebSocket Origin 不能单独视为认证；随机端口只降低碰撞/盲猜，不构成认证；MV3 worker 生命周期和后台心跳行为须实测；pipe 默认安全描述符须在目标 Windows 验证；旧 token 配置的迁移不能静默扩大权限；真实归档受 X/网络/下载组件影响，不应把外部服务失败误判为配对失败。

## 7. 当前执行状态与 Owner

### 已完成的非 Windows 开发与验证

- Phase 1–3 共享实现已在本批次源码中：版本化配对契约；Desktop 动态 loopback endpoint、generation-bound ticket、精确 Origin、首帧认证、deadline/资源限制；Native Host 通用 framing/control forwarding 与 Unix IPC；Extension 自动 bootstrap、sender allowlist、单飞/恢复、pending 清理和不跨通道重放。
- Windows Owner 的 Native Host/Named Pipe 代码通过 Git 集成为 `5ff0a22`。Full `installation.files` 加入 `extension/src/browser-pairing.js` 并由回归断言保护，提交 `9920566`。
- 本轮 Linux 验证（针对当前源码；不是 Windows 证据）：`npm test`、`npm run check`、`npm run build` 均 PASS（Desktop Node tests、Extension tests/check、Vite build；有既有非阻塞 Vite chunk warning）；`cargo fmt --all -- --check` PASS；`cargo test -p xarchive-protocol -p xarchive-native-host` PASS（protocol 23 tests，Host 测试通过）；WebSocket targeted tests PASS（13/13）；`node scripts/docs-audit.mjs` PASS；`git diff --check` 与 staged diff whitespace check PASS。未运行 Full Rust workspace regression；本次为定向 reconciliation，Full regression 不在选择范围。
- Phase 2/3 无已知待做的纯 Linux 实现项。本轮不改动配对语义或 UI 权威状态；历史 UI 状态不一致须先由 Windows 在精确产物上复现并采集脱敏字段。

### 交接状态和历史回执

Windows 回执 `ebdc44d` 精确测试 `6060f5a`，不含 `d65a01b`。该源的 27/27 targeted Node tests、Host/Full build、13-file Extension 目录检查为各自 scoped PASS；但 Host bootstrap 返回 `BOOTSTRAP_NOT_IMPLEMENTED`，Full 清单漏列已随包的 pairing helper（scoped FAIL），UI/E2E/lifecycle 未运行。对应历史不可转移为 integrated source 的 Windows PASS。

当前分支 `cross-platform/automatic-pairing-reconcile-20261002` 已推送。Windows 验证绑定代码提交 `9920566ef1df115effc3ab5df5d9a12fd42210ed`；fetch 后 checkout 此确切源提交，确认干净工作树，再构建新 Full 产物并执行 Windows 队列。分支后续提交只调整 handoff 文档；不得复用旧 artifact 的 Windows 结果。

连接恢复策略：启动、状态读取和下一业务请求可恢复 worker；一次失败周期最多四次退避重试；业务响应丢失返回不确定错误，不自动重放；没有新增定时 heartbeat。idle/sleep/MV3 在线行为仍需实测。自动模式不要求保存 port/token；ticket 仅内存暂存且成功自动配对后清除 legacy token。

**Linux 批次已收尾并交接。** Windows GUI、注册、安装、Winsock、ACL、真实浏览器、生命周期及归档是 Windows 专属验证，由 Windows Platform Owner 按手工步骤执行并将结果通过 Git 回交。它们不阻塞已完成的 Linux 实现与验证。

### Exact-revision Windows return — 6060f5a (2026-10-02)

The Windows receipt `ebdc44d` tested exactly `6060f5a`, which does not include the separate Windows implementation `d65a01b`. On that tested source, Full build and 13-file Extension-directory inventory passed scoped checks, but the packaged Host returned `BOOTSTRAP_NOT_IMPLEMENTED`, pairing/UI/E2E/lifecycle were `NOT_RUN`, and the Full installation manifest omitted the shipped `extension/src/browser-pairing.js`. Cross-platform integration is `5ff0a22`; the installation inventory correction is `9920566`. Neither prior Windows PASS evidence nor subsequent Linux tests validate Windows behavior of the integrated source. See current handoff and queue for the concentrated next-Owner matrix.
