# 仓库文件职责地图

本文说明人工维护文件的职责、入口、运行关系和测试位置。生成物、缓存、依赖和机器本地数据不逐文件登记。

## 根目录

| Path | 职责 | 维护说明 |
|---|---|---|
| `README.md` | 用户和项目概览 | 不写验证历史和内部 Plan |
| `AGENTS.md` | Agent 强制开发与文档治理规则 | 只保留规则摘要，详细流程链接到 `docs/` |
| `Cargo.toml` | Rust workspace 成员和共享 lint/license | 新 crate 必须加入 workspace 并更新本地图 |
| `package.json` | Node workspaces 和根级命令 | 命令变化同步 `docs/development/setup.md` |
| `.env.example` | 非敏感本地配置示例 | 不放真实凭据 |
| `THIRD_PARTY_NOTICES.md` | 第三方运行时和依赖许可证说明 | 按实际分发内容维护 |

## Rust crates

| Path | 入口/职责 | 维护与测试 |
|---|---|---|
| `crates/xarchive-core/src/` | Job 状态、重试策略、TagEngine、稳定用户目录名和领域模型 | 纯 Rust 单元测试；不依赖 Tauri、SQLite 或平台 API |
| `crates/xarchive-protocol/src/` | `lib.rs` 组合并 re-export 公共 API；`browser.rs` 负责 Browser 消息和 Tweet 校验；`sidecar_v2.rs` 负责 Sidecar v2 extraction 命令/事件；`media.rs` 定义 commit 路径使用的 durable `DownloadFile`；`jsonl.rs` 负责 JSONL 编解码；`error.rs` 负责协议错误 | 修改时同步 `shared/protocol-schema/`、Extension、Sidecar 和 Native Host；v1 Sidecar 命令/事件类型已在 U8 删除，不得重新引入 |
| `crates/xarchive-native-host/src/` | Native Messaging framing、请求校验和 forwarding 核心；`error.rs` 错误、`framing.rs` 编解码、`forwarding.rs` 转发 | framing/transport fake 测试；Windows endpoint 在平台层验证 |
| `crates/xarchive-sidecar-supervisor/src/` | `lib.rs` 管理进程生命周期，`error.rs` 定义监督错误，`events.rs` 定义事件，`readers.rs` 解析 stdout/stderr 并只接受 protocol v2 event（其他版本记为 `ProtocolError`） | `spawn_ready_v2` capability handshake、v2 event 解析、legacy protocol line 拒绝和真实 Python worker 测试 |
| `crates/xarchive-storage/src/` | `lib.rs` 负责 Database 连接、migration 和模块组合；`database/users.rs`、`tags.rs`、`tweets.rs`、`jobs.rs`、`settings.rs`、`telegram.rs` 分别负责对应 repository；`error.rs` 定义 StorageError；`models.rs` 定义公开 persistence/profile models；`file_store.rs` 负责 staging、profile、hash、commit 和 reparse/path 防护；`metadata.rs` 负责 Sidecar metadata 归一化；`archive_service.rs` 负责本地归档提交和 profile refresh；`jobs.rs` 的事件查询正确表达可为空的 `payload_json`；migration 位于 `crates/xarchive-storage/migrations/` | storage 单元和升级测试；repository 子模块共享 `Database.connection`，保持事务、migration 和 public API 不变 |
| `crates/xarchive-download/src/` | `lib.rs` 组合并 re-export 公共 API；`model.rs` aria2 传输模型；`plan.rs` 从 typed extraction result 构建 allowlisted `MediaTransferPlan`；`driver.rs` aria2-only transfer driver；`refresh.rs` URL expiry 一次性 refresh 与 stable media matching；`rpc.rs` JSON-RPC 请求/响应和解析；`client.rs` loopback HTTP client；`supervisor.rs` aria2 进程生命周期；`error.rs` 错误模型 | plan/refresh/driver fake backend 测试和 fake HTTP server/配置测试；旧 `DownloadRouter` gallery-dl/aria2 fallback 已在 U8 删除，不得重新引入；真实 aria2/Windows 集成另行验证 |
| `crates/xarchive-telegram/src/lib.rs` | SecretStore abstraction、Telegram request/transport、formatter 和幂等发送契约 | fake HTTPS server、脱敏、格式化和发送状态测试 |

## Desktop

| Path | 入口/职责 | 维护说明 |
|---|---|---|
| `desktop/src-tauri/src/main.rs` | Tauri native entry，调用 library `run()` | 保持极薄 |
| `desktop/src-tauri/src/lib.rs` | Tauri library 入口、模块组合、`ArchiveTweetRequest`、`run()`、Tauri command 注册和 Debug-only localhost MCP Bridge 注册 | 保持入口与模块组合职责；MCP Bridge 只在 Debug 构建注册并绑定 `127.0.0.1`；不承载归档、RuntimeState、平台或 aria2 业务实现 |
| `desktop/src-tauri/src/commands.rs` | App status（包含 database、Sidecar 和 executor 生命周期状态）、Sidecar 生命周期、Job 查询、executor submit/query/cancel/shutdown commands、archive root、archive/Extension 文件夹打开、Extension 文件状态和 runtime health commands；submit 负责短事务写入 BrowserTweet/user/Job/spec，真实 worker context 由 ExecutorRuntime 创建 | 保持 command API；submit 不得 lease 生产 Sidecar；真实 SQLite/Sidecar/FileStore/ArchiveService I/O 不得放回 RuntimeState 全局锁；Extension status 只报告文件就绪和平台检测边界，不得把文件存在误报为浏览器已连接 |
| `desktop/src-tauri/src/archive.rs` | `ArchiveExecutionContext` resource bundle、`ArchiveExecutionJob` execution-port adapter、Browser user/relationship merge、v2 extraction/transfer 调用、ArchiveService 提交、Job 事件/失败状态 | 保持 metadata identity binding 和 context 独立资源 ownership；只通过 `production.rs` 的 v2 extraction/transfer orchestration 归档；`archive_tweet` 同步 fallback 已在 U8 删除 |
| `desktop/src-tauri/src/production.rs` | U7 production orchestration：Sidecar v2 extraction event consumption、`ExtractionResult` → `MediaTransferPlan`、aria2 transfer、一次性 URL refresh、media identity/filename matching、staging output verification 和 `DownloadFile` 转换 | 只由 production executor 调用；不得写入 signed URL/header/GID durable metadata；Linux fake/contract tests 与 workspace verification；Windows aria2/process/file-lock/restart 由 validation queue 覆盖 |
| `desktop/src-tauri/src/executor.rs` | R1 Job executor command/state model；`ExecutorConfig`、`ProductionExecutionFactory`、`ExecutorRuntime`、`ArchiveApplicationService`、`JobExecutorHandle`、`JobPersistence`/`JobExecution` ports、独立 SQLite adapter、execution spec persistence、attempt fencing、recovery/completion/cancel/shutdown、运行中 cancellation、`ArchiveJobSubmissionAdapter`、execution result/error contract、ExecutorEvent/JobEvent 映射、bounded control worker 和 single active runner | `ExecutorRuntime` 由 `RuntimeState` 持有，但 runner 通过 `ProductionExecutionFactory` 自主打开 Database/FileStore/Sidecar 并创建 ArchiveExecutionJob；control worker 不执行长 I/O；startup recovery 读取 final/staging facts 并执行 commit action；Windows 实际进程终止和文件锁行为仍需平台验证 |
| `desktop/src-tauri/src/transport.rs` | Browser `ArchiveRequest`/`QueryStatus` 到 executor application service 的协议 transport adapter；Linux/Unix Desktop socket server；统一 BrowserRequest 校验、request_id 保留、Job submit/query 响应和错误映射 | Unix server 仅负责 framing、独立 SQLite persistence context 和短请求处理；Windows Named Pipe/ACL backend 仍属平台适配；不得将 Unix socket 测试外推为 Windows PASS |
| desktop/wdio.conf.mjs | WebdriverIO 本地 runner 与 @wdio/tauri-service 配置；根据 `WDIO_ADVANCED` 选择普通 smoke 或高级 plugin spec，解析 Tauri binary、Windows external Edge WebDriver、driver 端口、日志和环境变量 | 普通 smoke 不启用高级 spec；高级 spec 使用 `wdio-e2e` artifact；Windows 原生验收结果写入验证文档 |
| `desktop/scripts/wdio-tauri-service.mjs` | WDIO worker 适配层；复用官方 launcher，跳过依赖 `plugin:wdio` 的单窗口 focus probe，并避免 service 与 spec 重复清理 mock/session | 不改变普通 artifact 的 capability/guest JS 边界；高级 spec 负责显式 mock restore；Windows session/driver 自动回收仍需实机验证 |
| `desktop/scripts/run-wdio-advanced.mjs` | 跨平台启动高级 WDIO E2E，使用当前 Node 进程加载 workspace 的 WDIO CLI，并设置 `WDIO_ADVANCED`/`WDIO_CAPTURE_LOGS` | 不直接调用平台 `.cmd` shim；保持 Windows PowerShell/CMD 与 Linux 命令行为一致，并透传 runner 退出码 |
| `desktop/scripts/build-tauri-wdio.mjs` | 跨平台启动 `wdio-e2e` 专用 Tauri 构建，使用当前 Node 进程加载 Tauri CLI，并设置 `VITE_WDIO_E2E=1` | 不直接调用平台 `.cmd` shim；专用构建才注入 WDIO guest JS 和 `wdio-e2e` feature |
| `desktop/src-tauri/capabilities/wdio.json` | 专用 WDIO capability，授予 `wdio:default` 和测试窗口权限 | 不加入默认 capability；仅随 Debug/专用 E2E 验证使用 |
| `desktop/src-tauri/tauri.wdio.conf.json` | 高级 WDIO 构建的配置 overlay，只选择 `wdio` capability | 通过 `build:tauri:wdio` 使用；普通构建只选择 `default` |
| `desktop/e2e/specs/dashboard.e2e.mjs` | Tauri 原生窗口最小 DOM smoke，验证 Dashboard heading、main、导航和概览区域 | 必须在已构建的 Windows Tauri artifact 上运行；不能替代 Native Host、真实 IPC、DPI 或辅助技术验证 |
| `desktop/e2e/specs/wdio-plugin.e2e.mjs` | 高级 Tauri plugin E2E，验证 plugin availability、frontend execute、command mocking 和 cleanup | 必须使用 `build:tauri:wdio` artifact；Windows 日志桥接和 WebView2 行为仍需平台验证 |
| `docs/validation/windows-wdio-handoff.md` | 当前 Linux WDIO 配置完成后的 Windows handoff；记录同步、专用构建、advanced E2E、teardown、普通 release 回归和结果回写步骤 | 只描述待执行步骤，不记录虚构结果；Windows 结果仍写入 `docs/development/windows-validation.md`，队列状态仍以 `docs/validation/windows-queue.md` 为准 |
| `desktop/src-tauri/src/runtime.rs` | RuntimeState、便携 root、config/cache/download/logs 路径初始化、SQLite 和 executor 初始化 | portable root 来自 `XARCHIVE_PORTABLE_ROOT`、`.exe` 父目录或受控 fallback；最终归档和 staging 使用分离根目录 |
| `desktop/src-tauri/src/portable.rs` | portable root、config/cache/download/logs/sidecar/extension 路径派生及系统 Downloads fallback | 相对路径以 portable root 为基准；不创建 telegram；Windows Known Folder/权限/reparse 行为仍需实机验证 |
| `desktop/src-tauri/src/config.rs` | `config/config.yaml` 的 YAML 模型、日志等级、日志数量、路径解析、校验和原子保存 | `logging.level` 允许 error/warning/info/debug/silent；Debug 构建默认 debug，Release 默认 info；secret 不进入配置 |
| `desktop/src-tauri/src/components.rs` | U9 ComponentManager、embedded catalog schema、目录 artifact hash/size/layout/license/probe 校验、safe path、atomic activation 和 rollback | 只接受固定 catalog 与本地已获取 artifact；不执行动态网络下载或 ZIP 解压；模块单元测试覆盖 catalog/path/hash/install/rollback，Windows 文件权限/EXE probe/真实 assets 进入 validation queue |
| `desktop/scripts/release-assets.mjs` | U11 release asset 命名/manifest 契约校验（tag、资产名、kind、SHA-256、size、license）；纯 Node、无网络、无文件副作用 | 测试在 `desktop/test/release-assets.test.mjs`；真实资产构建/哈希/签名/上传只能在 Windows/CI 完成，进入 Windows queue |
| `desktop/test/release-assets.test.mjs` | U11 release manifest 契约测试 | 覆盖 versioned tag、asset kind、hash/size/license 拒绝用例 |
| `desktop/scripts/native-host-package.mjs` | U12 Native Host/Extension 安装包纯逻辑契约；校验 MV3 manifest、Extension ID、Native Messaging host manifest 和 Windows x64 安装布局 manifest | 无 Registry、浏览器或 Named Pipe 副作用；测试在 `desktop/test/native-host-package.test.mjs`；实际 Registry/ACL/浏览器加载进入 Windows queue |
| `desktop/test/native-host-package.test.mjs` | U12 Native Host/Extension 安装契约测试 | 覆盖 Extension ID、MV3 权限、host manifest、release tag 和安装布局校验 |
| `desktop/scripts/offline-bundle-package.mjs` | U13 Offline Bundle 组件清单、相对路径、SHA-256/size/license、运行时目录排除和 Release/catalog parity 契约 | 纯 Node、无下载/签名/Registry/浏览器副作用；测试在 `desktop/test/offline-bundle-package.test.mjs`；真实 Windows artifact 组装进入 Windows queue |
| `desktop/test/offline-bundle-package.test.mjs` | U13 Offline Bundle manifest/parity 契约测试 | 覆盖组件完整性、重复/缺失组件、路径逃逸、runtime 目录排除和 parity mismatch |
| `docs/releases/v0.2.0-pre.2.md` | U10/U11 Windows x64 pre-release notes、资产边界、外部依赖来源和已知限制 | 只记录实际发布范围；资产状态以 GitHub Release 和 workflow 结果为准，不把 BLOCKED/PENDING Windows 项目写成 PASS |
| `docs/releases/v0.2.0-pre.3.md` | U12/U13/U14 pre-release notes、Linux verification evidence、Windows validation boundaries and expected assets | Release Notes must distinguish expected assets from actual GitHub Release assets; Windows BLOCKED/PENDING items remain traceable to the validation queue |
| `desktop/src-tauri/src/commands.rs::get_component_bootstrap_status` | U10 Core Bootstrap 状态查询；报告 catalog version、active/missing component、ready/message | 只读取固定 embedded catalog 和本地 activation marker；不下载、不激活、不绕过 ComponentManager；Rust command test 与 Desktop UI wiring test |
| `desktop/src-tauri/src/logging.rs` | 同级 `logs/` 应用日志文件创建、等级过滤和 `xarchive-*.log` 数量轮转 | 默认最多 5 个；仅管理匹配命名的 `.log`；运行期完整日志接入和 Windows 文件权限仍需验证 |
| `desktop/scripts/build-portable-windows.mjs` | 组装 Windows Full/Core portable 目录并生成 `package-manifest.json` | `PORTABLE_PACKAGE_TYPE=full|core`；Full 缺少必需组件时失败，Core 不包含 gallery-dl/Extension；不生成 installer、不预创建 `download/`；Windows 实际 sidecar artifact、许可证和 `.exe` 组装仍需验证 |
| `desktop/scripts/portable-package.mjs` | portable 包类型校验、组件规划和 Full/Core manifest 纯逻辑 | 无文件系统副作用；测试位于 `desktop/test/portable-package.test.mjs`；修改包边界时同步更新 Windows Validation Queue |
| `sidecar/pyinstaller/xarchive-downloader.spec` | Windows PyInstaller worker 的入口、模块收集和 executable 构建定义 | 只生成 worker，不捆绑 gallery-dl；由 `.github/workflows/windows-worker-artifact.yml` 执行；真实 `.exe` smoke、哈希和运行仍需 Windows 验证 |
| `sidecar/pyinstaller/entrypoint_v2.py` | 当前 PyInstaller worker artifact 的唯一入口，委托 `xarchive_downloader.main` 解析 `--gallery-dl` 并启动 `worker_v2` | 当前 spec 必须指向该入口；v1 fallback 入口 `entrypoint_v1.py` 和重复入口 `entrypoint.py` 已在 U8 删除 |
| `.github/workflows/windows-worker-artifact.yml` | 在 Windows runner 上生成、smoke check、打包并上传 PyInstaller worker artifact | 只构建 Sidecar worker，不反向同步 artifact；修改 worker 入口或依赖时同步更新 spec、Windows Queue 和 artifact 哈希记录 |
| `desktop/src-tauri/src/platform.rs` | 平台相关的 archive folder 打开命令选择（Explorer、open、xdg-open） | 保持平台命令和路径参数边界；平台实机行为由 Windows/桌面验证队列确认 |
| `desktop/src-tauri/src/aria2.rs` | aria2 release allowlist、`latest_aria2_release` 最新版本语义、SHA-256 校验、可执行文件发现/版本检测/路径校验（`validate_aria2_path`）、Windows 下载解压和 aria2 Tauri commands | 保持官方版本 allowlist、错误脱敏和 Windows-only 下载边界；真实 aria2 业务集成仍由 Windows 队列验证 |
| `desktop/src-tauri/migrations/` | 不再使用；migration ownership 已迁移到 storage crate | 不应重新添加 migration |
| `desktop/src/main.jsx` | React Dashboard 的工作台/设置页入口、Tauri command adapter、任务概览、组件设置、Extension 加载指南和错误反馈 | 保持页面组合层；工作台只放高频概览，详细配置放设置页；新增 Tauri command 时同步 Rust 注册、测试和 Windows 队列 |
| `desktop/src/components/icon.jsx` | 统一 SVG `Icon` 组件（导航、状态、操作图标） | 图标几何/尺寸变更同步 Windows GUI/DPI 队列 |
| `desktop/src/components/copyable-path.jsx` | 可复制路径显示组件（显示名 + 等宽完整路径 + 复制反馈） | 剪贴板写入必须走 `copy_text_to_clipboard` Tauri 命令；WebView2 行为由 Windows 队列验证 |
| `desktop/src/components/connection-status.jsx` | `ConnectionStatus` 与 `ExtensionConnectionStatus`；Extension 状态使用显式枚举映射，文件缺失不得显示为"检测中…" | 状态语义变更同步 Extension 检测命令与测试 |
| `desktop/src/lib/ui-state.js` | 前端共享纯逻辑：显示名提取、aria2 状态文案、Extension 状态映射 | 无 Tauri 依赖；测试在 `desktop/test/ui-state.test.mjs` |
| `desktop/src/lib/log-lines.js` | 运行日志行解析、等级过滤、搜索前处理和轮询快照合并 | 无 Tauri 依赖；测试在 `desktop/test/log-lines.test.mjs`；日志读取由 `LogsPage` 调用 Tauri command |
| `desktop/src/pages/logs-page.jsx` | 运行日志页面：历史日志读取、1 秒轮询、等级筛选、搜索、自动跟随、复制和打开日志目录 | 真实 WebView2、剪贴板和窗口行为由 Windows Validation Queue 验证 |
| `desktop/src/style.css` | Dashboard 全局样式、字体栈、图标容器、工作台/设置页布局和响应式设计 token | 使用本地系统字体 fallback；视觉变更同步 Windows GUI/DPI/辅助技术队列 |
| `desktop/src/lib/utils.js` | 前端共享工具 | 保持无 Tauri 状态依赖 |
| `desktop/src-tauri/tauri.conf.json` | Tauri build、窗口、CSP 和 bundle 配置 | bundle 当前关闭，不能假设存在安装器 |

## Browser Extension

| Path | 职责 | 维护说明 |
|---|---|---|
| `extension/manifest.json` | MV3 权限、host、content script 和 service worker 声明 | 遵循最小权限；权限变化需安全审查 |
| `extension/src/content-core.js` | 纯 DOM Tweet/quote/reply 提取 | 不访问 Cookie、文件或 Tauri |
| `extension/src/content.js` | 页面注入、按钮和 MutationObserver | 只调用 background bridge |
| `extension/src/background.js` | Native Messaging bridge、request_id 路由和状态请求 | 与 browser protocol/schema 同步维护 |
| `extension/tests/` | DOM、bridge、断线和消息测试 | 新消息字段必须增加契约测试 |
| `extension/manifest.json` + `desktop/scripts/native-host-package.mjs` | U12 版本化 Extension/Native Host 发布边界 | 当前 Extension 使用开发者模式加载；固定 Extension ID 需由发布密钥/浏览器发布策略提供，不能在 Linux 伪造；Windows host registration、Registry、ACL 和浏览器 reload 由 queue 验证 |

## Python Sidecar

| Path | 职责 | 维护说明 |
|---|---|---|
| `sidecar/src/xarchive_downloader/__init__.py` | worker 公共入口：`--gallery-dl` CLI 解析、`main()` 和 `run_v2_worker` 导出 | 只启动 protocol v2 worker；v1 worker、`download` command 和 gallery-dl 媒体下载适配已在 U8 删除 |
| `sidecar/src/xarchive_downloader/worker_v2.py` | Sidecar v2 command reader / single extraction task / terminal event fence | cancel/shutdown、busy、EOF、JSONL serialisation |
| `sidecar/src/xarchive_downloader/protocol_v2.py` | v2 command/event/capability 校验、typed extraction result 序列化 | valid/invalid fixture、v1 rejection、unknown field、secret/header allowlist |
| `sidecar/src/xarchive_downloader/extraction.py` | gallery-dl extraction-only adapter（强制 `--skip-download`、stable identity、安全 filename） | 不写媒体主体文件；result 不携带下载事实 |
| `sidecar/src/xarchive_downloader/process.py` | gallery-dl 子进程的跨平台进程树隔离与终止（POSIX session、Windows `taskkill /T`） | 取消/超时必须回收整个 extraction 子树；Windows 进程树行为由 Windows 队列验证 |
| `sidecar/src/xarchive_downloader/models.py` | gallery-dl metadata 归一化（MediaItem/QuotedTweet/ExtractedTweet） | 与 protocol metadata identity 规则同步；不建模已下载文件 |
| `sidecar/src/xarchive_downloader/errors.py` | gallery-dl 错误分类 | 对外错误必须保持稳定、安全、有限长度 |
| `sidecar/tests/` | v2 worker/entrypoint、extraction adapter 和 metadata 测试 | 运行时依赖项目 Python 环境和 pytest；v1 worker/gallery 测试已在 U8 删除 |

### 规划中的新架构模块

以下路径是总体 Plan 的 `PLANNED` 模块，不代表当前文件已经存在；创建或移动文件时必须同步补充本表的职责、入口、运行关系、维护约束和测试位置。

| Planned path | 计划职责 | 计划测试/约束 |
|---|---|---|
| `crates/xarchive-download/src/transfer.rs` | 历史规划路径；当前 driver boundary 已由 `driver.rs` 承担 | 不再新增；保持 map 与实际实现一致 |
| `crates/xarchive-download/src/aria2.rs` | 历史规划路径；当前 aria2 implementation 已由 `driver.rs` + `client.rs` + `supervisor.rs` 承担 | 不再新增；保持 map 与实际实现一致 |
| `desktop/src-tauri/src/archive/{extraction,transfer,orchestrator,commit}.rs` | extraction、transfer、orchestration、commit 职责拆分 | executor integration、staging verification、recovery |
| `desktop/src-tauri/src/components/` | ComponentManager、catalog、safe install、probe、rollback | hash/layout/safe extraction/rollback |
| `shared/protocol-schema/sidecar-v2/` | Sidecar v2 JSON Schema 和 fixtures | cross-language contract validation |

## Protocol schemas and fixtures

| Path | 职责 |
|---|---|
| `shared/protocol-schema/*.schema.json` | Rust、JavaScript、Python 之间的字段和边界契约 |
| `shared/protocol-schema/fixtures/` | 跨语言有效/无效消息、aria2 response 和 JSONL 样例 |

Sidecar v1 的 `download-command.schema.json`、`download-event.schema.json` 和对应 fixtures 已在 U8 删除；`fixtures/sidecar-v1-rejected.jsonl` 保留，用于证明 v2 消费者拒绝 legacy 命令。修改 Schema 时必须检查所有 producer、consumer、fixture 和相关测试。

## 文档与验证

| Path | 职责 |
|---|---|
| `docs/architecture/` | 稳定架构、数据模型、运行流、ADR 和文件地图 |
| `docs/architecture/runtime-flow.md` | 当前浏览器、Desktop、Sidecar、storage、download 和 Telegram 运行流 |
| `docs/protocols/overview.md` | Browser/Desktop/Sidecar 跨进程命令、事件顺序、Schema 关系和 v1→v2 迁移边界 |
| `docs/development/status.md` | 当前实现状态 |
| `docs/development/roadmap.md` | 未来方向和完成标准 |
| `docs/development/testing.md` | 测试策略、命令和增量验证范围选择/升级规则 |
| `docs/development/risk-register.md` | 当前仍有效的风险、状态、责任模块和验证入口 |
| `docs/development/cross-platform-validation.md` | 跨平台开发/验证流程，含 Linux/Windows 增量验证范围和 Windows 重验判定规则 |
| `docs/validation/windows.md` | Windows 验证规范和报告模板，含最小验证范围、重验判定和 Validated/Not required/Deferred/Blocked 结论要求 |
| `docs/validation/windows-queue.md` | 当前 Windows Validation Queue 的唯一事实源，含重验元数据与增量重验规则 |
| `docs/development/windows-validation.md` | 历史 Windows 验证记录和兼容入口 |
| `docs/releases/` | 版本和 pre-release notes；每份说明必须区分 Linux 验证事实、Windows pending/blocking 项和未实现范围 | 发布 notes 必须与对应 tag、workflow 和 Windows Validation Queue 一致，不得把 planned/blocked 项写成 release capability |
| `aidlc-docs/inception/` | 初始需求、设计和工作包快照，不覆盖当前代码事实 |

## 不登记为人工维护源文件的内容

以下内容由工具生成或属于机器本地状态，不应写入 repository map 的功能职责表：

- `node_modules/`、`.venv/`、`target/`、`dist/`、`.vite/`；
- `__pycache__/`、`.pytest_cache/`、IDE cache；
- `X-Archive/`、SQLite、日志和 secrets；
- Tauri `gen/` 和本地构建 artifacts；
- `Cargo.lock`、`package-lock.json` 等锁文件只需在依赖变更时更新。