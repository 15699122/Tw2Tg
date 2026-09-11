# Windows 开发与验证清单

> 适用平台：Windows 10/11，优先验证 Edge，并回归 Chrome。文档日期：2026-09-10。

Linux 可验证协议、Rust 核心、Python 逻辑和前端静态检查，但不能替代 Windows 专属集成验证。本文集中记录必须在 Windows 实机或 Windows CI 完成的任务。

## 状态定义

| 状态 | 含义 |
|---|---|
| 已完成 | 已有明确测试结果或代码验证结果 |
| 待实现 | 对应功能尚未开发 |
| 待验证 | 功能已有，但尚未在 Windows 目标环境验证 |
| 阻塞 | 依赖工具、凭据、证书或外部环境 |

> 本项目开发阶段遵循“批量开发、集中验证”规则：Linux 可继续完成的功能不因最终需要 Windows 验证而暂停。开发过程中发现的 Windows 项目先进入累计 Windows Validation Queue；只有缺少 Windows 结果会使后续 Linux 设计或实现无法可靠继续时，才使用 `WINDOWS_VERIFICATION_BLOCKING`。当前 Queue 没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。

## 当前 Windows Validation Queue

以下队列根据当前 Plan、最终工作区变更、Windows 相关模块和历史验证结果累计维护。除明确标记外，状态均为默认的 `WINDOWS_VERIFICATION_PENDING`，不要求中断当前 Linux development phase。

| ID | Validation item | Related feature/change | Files/modules | Why Windows is required | Exact behavior | Prerequisite | Expected result | Priority | Blocks Linux development | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-P0-01 | Windows toolchain and full baseline | 当前 Rust/Node/Tauri/Sidecar 工作区及后续批量变更 | `Cargo.toml`、`package.json`、`desktop/`、`sidecar/` | MSVC、Windows SDK、WebView2、Python executable 和 Windows 构建行为不能由 Linux 完全替代 | 执行 workspace fmt/check/test/clippy、Node check/test/build、Sidecar tests、Tauri build/start/cleanup | Windows toolchain、项目 `.venv`、Node dependencies | 所有适用基础检查通过，无项目代码失败 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P0-02 | 真实 X/Edge Cookie archive | gallery-dl 默认链路、Sidecar 和 ArchiveService | Edge Profile、`sidecar/`、`xarchive-sidecar-supervisor`、`xarchive-storage` | Cookie 加密存储、Edge Profile 和真实 X 响应只能在目标环境确认 | 无媒体/单图/多图/视频/Quote/Reply/重复任务/异常退出后的真实归档 | 明确测试账号、Edge Profile、gallery-dl、可用网络 | Cookie 不泄露；Tweet/媒体/SQLite/staging 正确且幂等 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P0-03 | 文件 SQLite 应用级恢复 | Telegram send state migration、Desktop 启动恢复 | `xarchive-storage`、Tauri Desktop、migrations | 文件锁、应用重启、Windows 路径和异常退出无法由 in-memory 测试充分判断 | 写入真实文件 DB、关闭/重启、遗留 staging、`0001 → 0002`、异常退出恢复 | Desktop artifact、受控测试目录、可重复数据 | 状态恢复、迁移、staging 清理和唯一约束符合预期 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P0-04 | Native Host/Named Pipe end-to-end | Native Host endpoint forwarding、后续 Windows Named Pipe backend | `xarchive-native-host`、`xarchive-protocol`、Desktop IPC | Named Pipe server、ACL、连接和 Windows IPC 生命周期是平台行为 | 请求/响应、request_id 路由、多连接、重连、关闭、非法消息和权限拒绝 | Windows Named Pipe server、endpoint `\\.\\pipe\\xarchive-v1`、ACL 方案 | 合法请求正确转发，非法/越权请求明确失败，无串线或挂起 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-01 | DownloadRouter and real aria2 business integration | `DownloadRouter`、Desktop Job 结果处理、403 fallback、Sidecar/Job 接入 | `crates/xarchive-download`、`desktop/src-tauri/src/lib.rs`、Sidecar/Job orchestration | aria2c.exe 进程、Windows 路径、真实 media URL 和进程恢复需目标环境确认 | gallery-dl 默认；Router 错误不 panic；失败 Job/事件持久化；403 后重新提取；aria2 transfer 生命周期；失败回退和 Job 状态同步 | 受控 aria2c.exe、真实或本地 HTTP media server、可重复 Desktop archive 场景 | fallback 只在适用错误触发，状态、事件和文件结果一致 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-02 | Native Host browser installation | Host manifest、Registry、Edge/Chrome 加载 | `crates/xarchive-native-host`、manifest/installer（待实现） | Registry、浏览器扩展 ID 和安装权限是 Windows 专属行为 | 安装/升级/卸载、管理员/非管理员、扩展加载、Service Worker 重启和重连 | Host manifest、固定 Extension ID、浏览器实机 | Edge/Chrome 能加载 Host，连接和错误反馈符合协议 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-03 | Windows GUI rendering and accessibility | Dashboard GUI 修补与白色视觉重设计 | `desktop/src/main.jsx`、`desktop/src/style.css`、Tauri | WebView2/DPI/系统字体/屏幕阅读器/命中区域不能由 Linux 静态检查替代 | 100/125/150% DPI、最小窗口、Tab、键盘、Focus-visible、Narrator/NVDA、对比度 | GUI automation native-app target、WebView2、辅助技术 | 真实渲染、交互和辅助技术反馈通过 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-04 | Credential Manager and Telegram account flow | `SecretStore` abstraction、真实 Telegram transport | `xarchive-telegram`、Windows secret backend（待实现） | Credential Manager 用户边界和真实账号/网络行为是 Windows/外部环境事项 | 保存/读取/更新/删除、应用重启、日志隔离、真实 Bot API 发送与限流 | Windows Credential Manager backend、Bot token、Telegram test chat | Secret 不泄露，真实发送和重试状态正确 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-05 | Sidecar/externalBin/process lifecycle | Tauri externalBin、Sidecar packaging、Tray/Autostart | `desktop/src-tauri/`、Sidecar packaging（待实现） | Windows 子进程、资源路径、关闭和自启动行为需要目标环境 | 启动、关闭、崩溃恢复、资源定位、Tray、Single Instance、Autostart | bundled Sidecar、Tauri bundle、Windows shell environment | 资源可定位，子进程安全退出，生命周期符合预期 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P2-01 | Installer, signing and updater | M7/M8 发布能力 | Tauri bundle、installer、updater（待实现） | 安装器、签名、SmartScreen、Defender、升级/回滚为 Windows 发布行为 | 全新安装、覆盖升级、自定义路径、卸载、签名、失败回滚、数据保留 | installer artifact、证书/签名环境、发布测试机 | 安装、升级、卸载和回滚符合发布要求 | P2 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P2-02 | Windows filesystem stress and stability | 用户目录、staging、文件恢复和历史偶发路径问题 | `xarchive-core`、`xarchive-storage`、Desktop | 保留字符、磁盘、锁、长路径和并行时序需要 Windows 文件系统确认 | 非系统盘、空格/中文/Unicode、保留名、长路径、文件锁、磁盘不足、并行恢复 | Windows 多盘/受控权限/磁盘空间 | 无路径逃逸、数据损坏或未处理崩溃 | P2 | no | WINDOWS_VERIFICATION_PENDING |

当前没有 `WINDOWS_VERIFICATION_BLOCKING`：上述项目虽有 P0/P1/P2 优先级，但当前 Linux 代码、测试和设计均可继续推进，不存在必须先取得 Windows 结果才能可靠完成的后续 Linux 实现。

## Windows Validation Preparation / 集中式 Handoff

Linux development phase 结束后，基于最终 `git diff`、当前 Plan、变更模块、Windows 代码路径、项目配置、历史验证和上方 Queue 合并重复场景，按以下顺序一次性执行。单次完整启动覆盖多个功能时，不拆成重复启动测试。

### 1. Build / Toolchain

- **ID:** W-H-01
- **Test name:** Windows workspace baseline and Tauri build
- **Purpose:** 确认 MSVC/SDK/Node/Python/Tauri 以及当前批量开发结果可构建。
- **Related changes:** 所有当前 Rust、Node、Tauri、Sidecar 变更。
- **Prerequisites:** Windows 10/11、MSVC、Windows SDK、WebView2、Node/npm、项目 `.venv`。
- **Steps / command:** `npm ci`; `npm run check`; `npm run test`; `npm run build`; `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `npm run build:tauri`。
- **Expected result:** 命令通过，构建 artifact 生成，无项目代码导致的失败。
- **Priority:** P0
- **Manual interaction required:** no

### 2. Runtime

- **ID:** W-H-02
- **Test name:** Combined Tauri, Sidecar and process lifecycle
- **Purpose:** 一次覆盖 Desktop 启动、SQLite 初始化、Sidecar hello、状态查询、停止和进程清理。
- **Related changes:** Tauri commands、Native Host endpoint、Sidecar orchestration、externalBin。
- **Prerequisites:** 可启动 Desktop artifact、Sidecar executable/configuration、Python fallback（如适用）。
- **Steps / command:** `npm run dev:tauri`；执行 start/stop Sidecar、状态刷新、退出；记录进程和日志。
- **Expected result:** 应用启动、Sidecar 握手、停止和退出清理成功，无残留进程；非阻塞 Chromium 清理 warning 单独记录。
- **Priority:** P0
- **Manual interaction required:** yes

### 3. Filesystem

- **ID:** W-H-03
- **Test name:** File SQLite restart, migration and Windows path matrix
- **Purpose:** 合并文件 DB 恢复、staging、路径、锁和 Unicode 场景。
- **Related changes:** `xarchive-storage` migrations、FileStore、用户目录和 Desktop archive root。
- **Prerequisites:** 真实文件 DB、系统盘和非系统盘、受控权限、可制造文件锁/异常退出。
- **Steps / command:** 按 WQ-P0-03/WQ-P2-02 执行真实文件 DB 写入、重启、迁移、遗留 staging、空格/中文/Unicode/长路径、文件锁和异常退出。
- **Expected result:** 文件不逃逸 archive root，迁移和重启恢复正确，锁/磁盘错误可诊断，数据不损坏。
- **Priority:** P0
- **Manual interaction required:** yes

### 4. Integration

- **ID:** W-H-04
- **Test name:** Browser Extension → Native Host → Named Pipe → Desktop
- **Purpose:** 合并浏览器集成、Native Messaging、Named Pipe、Registry 和 request_id 路由验证。
- **Related changes:** Native Host forwarding、Windows endpoint、manifest/Registry、Extension bridge。
- **Prerequisites:** Named Pipe server、ACL、Host manifest、固定 Extension ID、Edge/Chrome。
- **Steps / command:** 按 WQ-P0-04/WQ-P1-02 执行 archive/query、并发、多标签、重连、Desktop 未启动、非法请求和权限场景。
- **Expected result:** 合法消息按 request_id 正确返回；无效、断线、权限和 Desktop 不可用时快速返回结构化错误。
- **Priority:** P0
- **Manual interaction required:** yes

- **ID:** W-H-05
- **Test name:** Gallery-dl / DownloadRouter / aria2 / Telegram integration
- **Purpose:** 合并真实 X、403 fallback、aria2 transfer、Credential Manager 和 Telegram 发送链路。
- **Related changes:** `DownloadRouter`、Sidecar/Job 接入、Telegram SecretStore/send state。
- **Prerequisites:** 测试账号、Edge Profile、aria2c.exe、Bot token/test chat、优先使用本地 HTTP fake server。
- **Steps / command:** 执行真实或受控 media 场景、403/认证/限流错误、aria2 pause/resume/recovery、Telegram send/retry/restart。
- **Expected result:** 默认 gallery-dl；仅适用错误 fallback；Secret/ Cookie 不泄露；Job、文件、Telegram 状态一致。
- **Priority:** P0
- **Manual interaction required:** yes

### 5. Packaging

- **ID:** W-H-06
- **Test name:** Bundled Sidecar, installer, signing and updater
- **Purpose:** 确认发布 artifact、安装/升级/卸载和资源分发。
- **Related changes:** externalBin、Tauri bundle、Native Host manifest、installer/updater。
- **Prerequisites:** bundle/installer artifact、签名证书、发布测试机。
- **Steps / command:** 执行全新安装、覆盖升级、自定义路径、非 ASCII 路径、卸载、失败回滚、Updater 和 Defender/SmartScreen 检查。
- **Expected result:** 资源可定位，数据按策略保留，升级/回滚安全，签名和安装行为符合预期。
- **Priority:** P1
- **Manual interaction required:** yes

### 6. Regression

- **ID:** W-H-07
- **Test name:** GUI and previously verified Windows regression suite
- **Purpose:** 复验本轮修改涉及的 GUI 语义/错误恢复/aria2 gating，并保留历史 Windows 基线。
- **Related changes:** `desktop/src/main.jsx`、`desktop/src/style.css`、`xarchive-download`、`xarchive-native-host`。
- **Prerequisites:** W-H-01/W-H-02 通过；GUI automation target、WebView2、Narrator/NVDA（如适用）。
- **Steps / command:** 执行现有 Node/Rust/Sidecar/Tauri 基线，并按 WQ-P1-03 进行真实 DPI、Tab、键盘、Focus-visible、辅助技术和对比度复验。
- **Expected result:** 历史通过行为保持；新增项逐项给出 PASS/FAIL/BLOCKED，不以 Linux 结果替代 Windows 结论。
- **Priority:** P1
- **Manual interaction required:** yes

当前集中式 handoff 中没有 `WINDOWS_VERIFICATION_BLOCKING` 项。进入 Windows Validation Preparation 的前提是 Linux development phase 已完成，而不是某个普通 pending 项目单独完成。

## 当前基线（2026-09-09）

| 项目 | 最新结果 | 状态 |
|---|---|---|
| Node workspace | `npm ci` 成功安装 70 个依赖并审计为 0 个漏洞；`npm run check`、`npm run test`、`npm run build` 通过；Desktop 无 Node 测试用例，Extension 6 个测试全部通过。npm 提示 `esbuild` postinstall script 尚未批准 | 已完成基础验证；安装脚本警告已记录 |
| Rust workspace | 当前 Windows `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、严格 `cargo clippy --workspace --all-targets -- -D warnings`、Debug/Release 编译和 `npm run build:tauri` 均通过，共 69 个 crate 单元测试通过；Linux let-chain 修复已完成 Windows re-validation | 基础验证和 clippy 已完成 |
| Rust 测试稳定性 | 一次并行验证中 `xarchive-storage::completes_archive_directly_from_sidecar_result` 偶发报 Windows 路径不存在；目标测试单独重跑及串行完整 workspace 均通过 | 需后续观察 |
| Rust 格式 | Windows `cargo fmt --check` 通过 | 已完成 |
| Rust lint | `large_enum_variant`、Telegram formatter 的 `single_char_add_str` 和 Tauri aria2 路径扫描的 `clippy::collapsible_if` 均已修复；Windows 严格 workspace clippy re-validation 通过 | 已完成 |
| Python Sidecar | `.venv` + editable 安装，gallery-dl 1.32.11，10 个测试全部通过 | 已完成基础验证 |
| Sidecar 路径兼容 | 中文、空格、Unicode 路径下完成 JSONL `ready → started → log → failed` 流程 | 已完成基础验证 |
| 示例 X URL | 返回 `EXTRACT_OR_DOWNLOAD_FAILED` | 已记录，不能视为认证下载成功 |
| Edge Cookie/真实 X | 尚未使用明确账号环境验证 | 外部账号环境阻塞 |
| Native Messaging framing | Chromium 4 字节 little-endian framing、1 MiB payload 限制、JSON 读写和错误边界已在跨平台 Rust crate 中实现并测试 | 跨平台代码已完成，Windows Edge/Chrome 实机待验证 |
| Named Pipe | 对应 Windows transport 尚未实现 | 待开发，不是测试失败 |
| Retry/TagEngine/用户目录 | retry/backoff、TagEngine、Windows-safe 用户目录名和 users/user_names/tags/tweet_tags Repository 已在跨平台 Rust 中实现并测试 | 跨平台代码已完成，Windows 文件系统/并行故障注入待验证 |
| Telegram contract | SecretStore abstraction、Bot API request models、metadata formatter、UTF-8 continuation、media group 分组和 `reqwest 0.13.4` + Rustls HTTPS transport 已在跨平台 Rust 中实现并测试；fake-server 已覆盖四种 Bot API 方法及 HTTP/API 错误；发送状态持久化与幂等补传已作为跨平台代码实现并通过 Linux 测试 | 跨平台 transport 与发送状态持久化已完成；Windows 平台验证 `WINDOWS_VERIFICATION_PENDING`；Credential Manager 和真实账号发送仍待平台/账号验证 |
| Windows 构建依赖 | Visual Studio BuildTools/MSVC、Windows SDK、MSBuild、WebView2 可用；`aria2c`、`cmake`、`ninja` 不在 PATH | 工具链已完成，aria2c artifact/进程集成待实现 |
| Tauri Desktop 脚手架 | Tauri CLI 2.11.4 已由项目依赖安装；Windows `npm run dev:tauri` 已启动 Vite、Rust Debug 和 Desktop 可执行文件，`npm run build:tauri` 已生成 Release 可执行文件；当前未启用 bundle，真实 externalBin/安装包仍未配置 | 开发启动/构建已完成；GUI/打包待验证 |
| 执行过程错误与警告 | 首次未设置 `PYTHON` 时 Rust Supervisor 两个真实 Worker 测试因默认 `python3` 不存在而报 `NotRunning`，显式使用项目 `.venv\Scripts\python.exe` 后复验通过；Rust/Tauri 构建输出 MSVC linker stdout `#[warn(linker_messages)]` 非阻塞警告；`npm ci` 提示 `esbuild` postinstall script 尚未批准；停止 Tauri 开发进程时出现 Chromium `Error = 1411` 注销警告 | 已处理环境错误；其余为不阻塞警告 |

## 上一轮 Windows 平台验证记录（2026-09-09）

### Validation Environment

| 项目 | 实际值 |
|---|---|
| Linux source branch/revision | `main` / `f70f91399a0866ca8ee35741481e805a71556dc7`，包含 working tree changes |
| Linux working tree | 验证开始前已存在 `README.md`、`aidlc-docs/aidlc-state.md`、`crates/xarchive-download/src/lib.rs`、多份开发/架构文档的未提交修改，以及未跟踪的 `AGENTS.md`、`docs/development/cross-platform-validation.md`、`docs/validation/`；本轮仅修改本验证文档，未修改业务代码 |
| Windows 工作副本 | `E:\Shiraishi\VSCode Workspace\Tw2Tg` |
| Windows 系统/架构 | Windows 11 Insider Preview `10.0.29661` / 64 位 |
| 运行时/工具链 | Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、rustfmt/clippy `1.9.0`、Python `3.14.7`、Tauri CLI `2.11.4` |
| 验证日期 | 2026-09-09 |

同步方向为 Linux source → E: Windows validation workspace。同步排除了 `.git`、`target`、`node_modules`、`.venv`、`dist` 和缓存/数据库文件，未删除 E 盘额外文件；同步后 `rsync --checksum` 内容校验无差异。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Node 依赖 | PASS | `npm ci`；安装 70 个依赖，审计 0 个漏洞 |
| Node 检查/测试/构建 | PASS | `npm run check`、`npm run test`、`npm run build`；Desktop 0 项 Node 测试，Extension 6 项全部通过 |
| Rust 格式/编译 | PASS | `cargo fmt --all -- --check`、`cargo check --workspace` |
| Rust 全量测试（环境修正后） | PASS | 当前进程 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后执行 `cargo test --workspace`；60 个 crate 单元测试全部通过，含 Telegram 8 项测试 |
| Rust clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings` |
| Python Sidecar 测试 | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Sidecar Windows 路径/JSONL 进程链路 | PASS | 项目 `.venv` 在含空格、中文和 `Ω` 的临时路径启动；输出 `ready → started → failed` 及 `INVALID_JSON`，进程 exit 0 |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version`；`npm run build:tauri`；生成 `target\release\xarchive-desktop.exe` |
| Tauri 开发启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe` |
| Tauri GUI 视觉验收 | BLOCKED | GUI 自动化 helper 未提供可用的 native app target；仅确认启动日志，未将其当作 GUI 验收通过 |

### Errors

| 失败/警告 | 分类与原因 | 对后续验证的影响 |
|---|---|---|
| 默认 `cargo test --workspace` 的两个 Supervisor 测试报 `NotRunning` | Windows 环境配置/测试 harness 前提：测试默认调用 `python3`，但 Windows PATH 中不存在；设置当前进程 `PYTHON` 指向项目 `.venv\Scripts\python.exe` 后复验通过 | 不阻塞其他测试；后续开发事项是让 Windows 测试显式配置 Python 或改善默认探测，本次不修改代码 |
| `npm ci` 提示 `esbuild@0.28.2` postinstall script 未被 `allowScripts` 批准 | 依赖安装安全策略警告；本轮构建和测试均通过 | 不阻塞当前验证；是否批准该脚本需后续依赖策略决定 |
| Rust/Tauri 构建输出 MSVC linker stdout `#[warn(linker_messages)]` | 工具链非阻塞 warning，未导致构建失败 | 不阻塞 |
| 停止 Tauri 开发进程时出现 Chromium `Error = 1411`，进程返回 `STATUS_CONTROL_C_EXIT` | 主动 Ctrl+C 停止开发进程时的窗口类注销/终止警告；启动阶段已正常运行 | 不影响启动验证；GUI 视觉状态仍未确认 |

### Not Executed / Blocked / Not Applicable

| 验证项目 | 状态 | 原因 |
|---|---|---|
| Edge Cookie、真实 X 认证和真实媒体归档 | BLOCKED | 缺少明确账号/Profile；公开示例 URL 的失败链路不能替代认证验收 |
| Named Pipe transport、Native Host Registry/manifest、Edge/Chrome 实机 Extension | NOT RUN | Named Pipe/Registry 相关 Windows 集成功能尚未实现；浏览器实机链路依赖这些前置项 |
| Tauri GUI 前后端交互、视觉和资源路径人工验收 | BLOCKED | GUI 自动化 helper 无法提供可用 native app target；本轮只能确认进程启动日志 |
| Tauri bundle、安装器、升级/卸载、真实 externalBin Sidecar | NOT RUN | `bundle.active=false`，externalBin 和安装器 artifact 尚未配置/实现 |
| Windows 长路径、文件锁、磁盘不足、Tray/Single Instance/Autostart、Credential Manager | NOT RUN | 项目功能或 Windows backend 尚未实现，不能用基础构建结果替代专项验收 |
| aria2 管理 UI/检测/版本 allowlist | LINUX_VERIFIED | Desktop 已增加 PATH、程序目录、应用数据目录检测、版本显示、官方版本选择、SHA-256 allowlist 和下载按钮；Linux 已通过 Desktop Rust/Vite 构建验证 |
| aria2c executable/进程集成 | WINDOWS_VERIFICATION_PENDING | 本轮同步的 Windows 副本已包含当前 Linux 的 `Aria2Supervisor` 和 aria2 管理 UI，但 Windows 环境中 `aria2` 不在 PATH，也未发现项目提供的 `aria2c.exe`；因此未执行真实下载、PowerShell 解压、进程生命周期、大文件断点、崩溃恢复、`.aria2` 清理、Unicode staging 路径和 403 回退 gallery-dl 验证 |
| 真实 Telegram 账号发送与持久化 | BLOCKED | 真实账号/网络发送环境及发送状态持久化验收尚未提供；本轮仅覆盖 HTTPS contract/fake-server 单元测试 |
| Linux-only 的 pytest/cargo-clippy 重新执行 | NOT APPLICABLE | 本任务目标是 Windows validation；Windows 对应的 pytest 和 clippy 已实际执行 |

本节记录 2026-09-09 从当前 Linux working tree 同步后的 Windows 复验。默认 `python3` 测试探测、GUI automation target、Named Pipe/Registry、externalBin/安装器和 aria2c.exe Windows 集成仍应作为后续开发/验证事项处理。

### Linux Follow-up

| 后续事项 | 原因 |
|---|---|
| Windows Rust Supervisor 测试的 Python 前置配置 | 默认探测 `python3` 在 Windows PATH 中不可用；本轮需显式设置项目 `.venv\Scripts\python.exe` 才能通过，建议后续开发/测试流程明确 Python 解析规则 |
| `aria2c.exe` Windows 实际集成 | 当前 Linux 的 supervisor 修改已同步，但 Windows 缺少可执行文件；需提供或安装受控 artifact 后再验证版本/hash、生命周期、断点、崩溃恢复、`.aria2` 和 Unicode 路径 |
| GUI、Named Pipe/Registry、externalBin/安装器和真实账号链路 | 分别受 UI automation target、尚未实现的 Windows backend、未配置发布 artifact 和凭据/外部服务限制 |

本轮未为通过验证而修改业务代码、依赖或系统设置；以上事项作为后续 Linux 开发/验证任务保留。

### Linux Reconciliation（2026-09-09）

依据最新 Windows 结果和 `docs/development/cross-platform-validation.md` 重新评估当前 Plan：

| 事项 | 状态 | 当前事实 |
|---|---|---|
| Windows Rust fmt/check/test/完整 clippy、Node、Sidecar 基础链路、Tauri Debug/Release 构建 | WINDOWS_PASS | 已在 Windows 实际执行并通过；Supervisor 测试需要显式使用项目 `.venv\\Scripts\\python.exe` |
| Telegram HTTPS transport | LINUX_VERIFIED | Linux 已实现 `reqwest 0.13.4` + Rustls transport，并通过 fake-server 测试；真实账号发送仍为 `WINDOWS_BLOCKED`/账号环境事项，不因 Linux 测试改写为 Windows PASS |
| Telegram 发送状态持久化与幂等补传 | 单元层 WINDOWS_PASS / 应用层 WINDOWS_VERIFICATION_PENDING / 真实账号 BLOCKED | Windows 当前 revision 的 `cargo test --workspace` 通过 69 项，其中 storage 16 项、telegram 12 项覆盖状态往返、重试计数、迁移重开和幂等发送（实际执行，单元层可记 PASS）；但现有测试使用 in-memory SQLite，基于文件的 SQLite Windows 路径行为、应用重启现场恢复和 0001→0002 迁移升级仍需专项验证，不得整体标记 PASS；真实账号发送、Credential Manager 另受账号/Windows backend 限制 |
| aria2 supervisor core | LINUX_VERIFIED | Linux 已实现并验证配置校验、aria2 参数构造、进程启动失败映射、RPC 就绪检查和 secret 脱敏 |
| aria2c.exe Windows 实际集成 | WINDOWS_VERIFICATION_PENDING | 本轮已将当前 Linux working tree 同步到 Windows 工作副本；UI 和 Rust 下载管理命令已实现，但环境中没有 `aria2c.exe`，且尚未执行官方 ZIP 下载/PowerShell 解压；仍需提供受控 artifact 后验证版本/hash、安装目录检测、进程生命周期、断点、崩溃恢复、`.aria2` 清理、Unicode staging 和 403 回退 |
| Windows GUI 视觉验收 | WINDOWS_BLOCKED | GUI automation helper 仍未提供可用 native app target |
| Edge Cookie、真实 X、Named Pipe、Registry、externalBin、安装器、Credential Manager、Tray/Autostart | WINDOWS_BLOCKED / NOT_RUN | 依赖账号、Windows backend、发布 artifact 或尚未实现的前置功能 |

本轮 Linux 开发修改了 `crates/xarchive-download/src/lib.rs` 及相关 Plan/架构文档，未修改 Windows 工作副本代码；本轮已重新同步并执行可用的 Windows 验证。Linux 端已执行相关 regression tests；aria2c.exe 相关项目仍保持 `WINDOWS_VERIFICATION_PENDING`，直到实际进程集成验证通过。

---

## Linux Reconciliation（2026-09-10）

依据最新 Windows 结果和 `docs/development/cross-platform-validation.md` 重新评估当前 Plan。本轮 Linux 端完成了以下 bug 修复和清理工作：

### 本轮 Linux 变更

| 变更 | 文件 | 说明 |
|---|---|---|
| `record_event` SQL 参数修复 | `crates/xarchive-storage/src/lib.rs` | 修复 `INSERT INTO events` 语句缺少 `params!` 宏导致的 SQL 执行失败 |
| `archive_tweet` 所有权重构 | `desktop/src-tauri/src/lib.rs` | 修复 `database` 在闭包中移动后再次使用的编译错误，正确处理 `ArchiveService` 所有权转移 |
| `stop_sidecar` SidecarCommand 初始化 | `desktop/src-tauri/src/lib.rs` | 添加缺失的 `executable`, `browser`, `profile` 字段 |
| 未使用 import 清理 | `desktop/src-tauri/src/lib.rs` | 移除未使用的 `xarchive_core::JobState` 导入 |
| 跨平台验证规则更新 | `AGENTS.md`, `docs/development/cross-platform-validation.md`, `docs/validation/windows.md` | 新增"批量开发、集中验证"工作流规则 |
| Windows Validation Queue 整理 | `docs/development/windows-validation.md` | 新增 11 个队列项目，全部 `WINDOWS_VERIFICATION_PENDING` |

### 当前验证状态

| 项目 | 状态 | 说明 |
|---|---|---|
| Linux 编译 | PASS | `cargo check --workspace` 无错误无警告 |
| Linux 单元测试 | PASS | 79 个 crate 单元测试全部通过（11 core + 5 desktop + 16 download + 8 Native Host + 7 protocol + 4 supervisor + 16 storage + 12 Telegram） |
| Linux 格式检查 | PASS | `cargo fmt --all -- --check` 通过 |
| Windows 基础工具链 | WINDOWS_PASS | 已在 2026-09-09 Windows 验证中确认通过 |
| Windows 单元测试 | WINDOWS_PASS | 69 个 crate 单元测试通过（环境修正后） |
| Windows GUI 视觉验收 | WINDOWS_BLOCKED | GUI automation helper 不可用 |
| Edge Cookie/真实 X | WINDOWS_BLOCKED | 缺少测试账号 |
| Named Pipe/Registry | NOT RUN | 功能尚未实现 |
| aria2c.exe Windows 集成 | WINDOWS_VERIFICATION_PENDING | Windows 环境中无 aria2c.exe |
| Telegram 真实账号 | WINDOWS_BLOCKED | 缺少账号/网络环境 |

### Windows Validation Queue 更新

本轮 Linux 变更未新增 Windows 验证项目。现有 11 个队列项目保持 `WINDOWS_VERIFICATION_PENDING`，无 `WINDOWS_VERIFICATION_BLOCKING`。

### 下一步

根据 deferred Windows validation 规则：
1. 当前无 Linux-only 开发项需要继续
2. 所有非 Windows-dependent 开发工作已完成
3. Linux 端验证已全部通过
4. Windows 验证项目已累计记录，等待集中执行

当前可进入 Windows Validation Preparation 阶段，但无新的 blocking 项需要立即处理。

---

## P0：运行链路

### W-P0-01 工具链

**验证方式：** Windows 实机 + Windows CI；**状态：** 基础构建、测试和 lint 完成，GUI/打包仍待验证。

确认 Rust/Cargo、rustfmt、clippy、Node/npm、Python、Visual Studio C++ Build Tools、Windows SDK 和 x64 target 可用。开发阶段 `icons/icon.ico` 已补齐；Windows workspace 的 rustfmt、check、test、完整 clippy、Debug/Release 和 Tauri Release 构建均通过。Supervisor 测试首次因默认 `python3` 不在 Windows PATH 而报 `NotRunning`，改用项目 `.venv\Scripts\python.exe` 后通过。E 盘首次 Node 检查因缺少 `vite`，本轮执行项目内 `npm ci` 后复验通过；npm 另提示 `esbuild` postinstall script 尚未批准。

```powershell
rustc --version
cargo --version
rustfmt --version
cargo clippy --version
node --version
npm --version
py --version
```

最低验收：

```text
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run test
npm run build
```

### W-P0-02 Python Sidecar

**验证方式：** Windows 实机；**状态：** 基础测试和本地进程链路已完成，真实 X 提取和打包待实现。

验证 `.venv`、editable 安装、Worker 启动、`hello`、`download`、`shutdown`、stdout JSONL、stderr 日志、退出码，以及工作目录含空格/中文/Unicode 时的行为。

验收：Rust Supervisor 能启动 Worker；`hello → ready`、`download → started → complete/failed`、`shutdown → exit` 全部成立；不依赖全局 Python 包。当前已使用项目 `.venv`、gallery-dl 1.32.11 验证真实 sidecar 在含中文、空格和 Unicode 的工作路径中输出 JSONL；示例 URL 因 X 提取错误返回 `EXTRACT_OR_DOWNLOAD_FAILED`，真实账号下载仍待验证。

### W-P0-03 Edge Profile/Cookie

**验证方式：** Windows Edge 实机；**状态：** 待验证，依赖明确账号环境。

验证 Edge `Default` Profile、浏览器运行中/关闭后的 Cookie 读取、无效 Profile、Cookie 失效、登录可见内容、敏感内容和受保护账号内容。

安全验收：Cookie 不进入 Extension、Rust IPC、SQLite、日志、`tweet.json` 或 aria2 参数；失效映射为 `AUTH_REQUIRED`。

### W-P0-04 真实 X 本地归档

**验证方式：** Windows 实机；**状态：** 待验证，依赖 Edge Cookie 和可用 X 账号。

```text
真实 X URL → gallery-dl → Python Sidecar → Rust Supervisor
→ ArchiveService → SQLite → staging → 最终目录
```

至少覆盖：无媒体、单图、多图、视频、图文混合、Quote、Reply、不可访问 Tweet、重复点击、下载中关闭 Desktop、本地文件已存在。

验收：Tweet ID 幂等；文件可打开；JSON/TXT 正确；大小和 SHA-256 由 Rust 实际校验；重启后 Job 可恢复。

---

## P1：Desktop 与浏览器集成

### W-P1-01 Tauri Desktop

**验证方式：** Linux 开发环境 + Windows 实机/CI；**状态：** CLI 入口、Debug/Release 编译和开发启动已完成，Windows GUI/打包待验证。

当前已完成：Vite/React 前端、Tauri 2 Rust crate、状态/Job/目录/Sidecar commands、启动时 SQLite 初始化、Sidecar `hello → ready` 握手、最近 Job 查询、跨平台打开归档目录、本地 shadcn/ui 组件和 Dashboard、基础 capabilities、开发阶段 PNG/ICO 图标、Windows Debug/Release Rust 构建、项目内 Tauri CLI 入口，以及 Windows `npm run dev:tauri` 启动和 `npm run build:tauri` Release 构建。启动日志确认 Vite、Rust Debug 和 Desktop 可执行文件均启动；停止开发进程时出现 Chromium `Error = 1411` 注销警告，最终以 Ctrl+C 终止。当前尚未配置真实 `externalBin` Sidecar；仍需验证前后端通信、资源路径、打包后 Sidecar 启动、安装到含空格/非 ASCII 路径及非系统盘。UI 自动化 helper 初始化失败，因此本轮未完成 GUI 视觉和安装器验证。

当前 `bundle.active=false`，图标仍为开发阶段 PNG/ICO 资源；正式打包前必须替换正式图标集、启用 bundle 并完成安装器测试。

### W-P1-02 Named Pipe

**验证方式：** Windows 实机；**状态：** Windows transport 待实现。

目标：

```text
\\.\pipe\xarchive-v1
```

验证 Server 启动、Native Host 连接/重连、多连接、request_id 路由、批量 `query_status`、Desktop 退出、ACL、消息大小限制、非法 JSON/协议版本/action 拒绝。

### W-P1-03 Native Messaging Host

**验证方式：** 跨平台代码测试 + Windows Edge/Chrome 实机；**状态：** 跨平台代码已完成，Windows 集成待验证。

跨平台已验证 Chromium 长度前缀 framing、stdin/stdout 二进制读写、stdout 机器协议、stderr 诊断和大 payload 拒绝；Windows 仍需验证 origin allowlist、Host manifest、Desktop 离线、Host 反复启动/关闭和实际 Edge/Chrome 连接。

### W-P1-04 Registry 与 Host manifest

**验证方式：** Windows 实机；**状态：** 待实现。

确认 Chrome/Edge 注册路径、固定 Extension ID、安装/升级/卸载、管理员/非管理员权限、安装目录移动、路径含空格时的行为。

### W-P1-05 MV3 Extension

**验证方式：** Windows Edge/Chrome 实机；**状态：** Host/Extension 跨平台代码已完成，浏览器集成待验证。

跨平台代码已覆盖 Tweet ID/URL/metadata 提取、MutationObserver、按钮去重、Service Worker request_id 路由、Native Host 断线错误处理和最小消息边界；Windows 仍需验证开发版加载、Extension ID、Timeline、Tweet Detail、SPA 路由、虚拟滚动、多标签同步、Service Worker 重启和 Native Messaging 重连。

场景：

```text
x.com / twitter.com / Timeline / Detail / Quote / Reply
刷新 / 多标签 / Desktop 未启动
```

### W-P1-06 aria2c

**验证方式：** Linux/跨平台 fake server + Windows 实机/CI；**状态：** 基础跨平台 supervisor 已实现并完成 Linux 验证，Windows aria2c.exe 集成和恢复验证待执行（`WINDOWS_VERIFICATION_PENDING`）。

跨平台已验证 loopback HTTP RPC、Secret 参数、`addUri/tellStatus/pause/unpause/remove` 请求和响应、HTTP/RPC 错误映射，以及 `Aria2Supervisor` 的配置校验、aria2 启动参数、进程启动失败映射和 RPC 就绪检查；仍需在 Windows 验证随应用提供的 `aria2c.exe`、版本/hash、进程生命周期、大文件断点、崩溃恢复、`.aria2` 清理、Unicode staging 路径和 403 回退 gallery-dl。

优先使用本地 HTTP 测试服务器，不直接依赖 X CDN。

---

## P1：文件系统与生命周期

### W-P1-07 Windows 文件系统

**验证方式：** Windows 实机；**状态：** 逻辑已测试，Windows 待验证。

覆盖系统盘/非系统盘、空格、中文/Unicode、Windows 保留字符和文件名、`CON/PRN/AUX/NUL`、超长路径、磁盘不足、文件锁、目标目录已存在、遗留 staging 和异常退出恢复。

### W-P1-08 Tray/Single Instance/Autostart

**验证方式：** 跨平台代码测试 + Windows 实机；**状态：** 跨平台规则和抽象已完成，Windows backend/实机待验证。

验证 Tray 启动、关闭隐藏、打开/退出菜单、第二次启动激活已有实例、登录自启动、禁用自启动、后台 Sidecar 工作和关机安全退出。

### W-P1-09 Secret Store

**验证方式：** 跨平台代码测试 + Windows 实机；**状态：** SecretStore abstraction 已完成，Windows backend 待实现/验证。

跨平台已完成 `SecretStore` abstraction、内存测试实现、BotToken 脱敏、Telegram request contract 和基于 `reqwest 0.13.4` + Rustls 的 HTTPS transport；仍需实现 Windows Credential Manager 或 Stronghold backend，验证应用重启读取、删除/更新、日志/SQLite/Extension/Sidecar 隔离和 Windows 用户边界。真实 Telegram 账号发送、API 限制和发送状态持久化仍待账号/业务环境验证。

### W-P1-10 GUI 源码审查结论

**验证方式：** Linux 源码审查 + Windows 实机/CI；**状态：** Linux 源码审查已完成（2026-09-09），结论是当前 GUI 为较高完成度的开发 Dashboard 原型，尚不符合直接视觉验收的正式界面。GUI 核心修补、按平台展示、焦点/键盘可访问性和对比度仍需完成；Windows WebView2/DPI/Narrator/NVDA 真实渲染、Tab/焦点可见性、键盘流程、命中目标和最终对比度验证必须在 Windows 实机或 Windows CI 上完成，不能仅靠 Linux 静态审查替代。

参见 `docs/development/roadmap.md` M6 GUI 的“当前 GUI 设计评估”部分。

### W-P1-11 Tauri GUI 视觉与交互人工验收

**验证方式：** Windows 实机/CI + GUI automation target；**状态：** 待验证，受限于 UI automation target 是否可用。

验证侧栏导航名/键盘聚焦/焦点可见性、错误与成功反馈、加载与空状态、统计语义、aria2 按平台展示、主按钮层级、紧凑断点可操作性、Windows 缩放与显示缩放下的可读性、帮助文本对比度、实时任务列表更新的屏幕阅读器反馈和操作恢复。

Linux 侧仅能通过构建、静态可访问性检查和样式正文推断覆盖这些项，最终验收以 Windows 真实渲染为准。

### Linux GUI redesign reconciliation（2026-09-10）

Linux 已将 Desktop GUI 重设计为白色主色调、Vercel 风格的本地控制台：白色背景、细灰边框、近黑主按钮、浅色语义 Badge、结构化最近任务、运行环境/归档位置卡片、Skeleton 加载状态和更适合 Desktop/DPI 的字号层级。此次修改保留现有 Tauri commands、Widget 级错误隔离/重试、`aria2` Windows gating、`<ul>/<li>`/`<time>` 语义、`role="alert"`、`aria-label`、`:focus-visible` 和 reduced-motion 约束。

Linux 端适用的 Vite check/build、Extension 静态检查、Rust fmt/check/test 已通过。由于本轮没有可用的 Windows native GUI automation target，以下项目不得提前标记为 `WINDOWS_PASS`，当前保持 `WINDOWS_VERIFICATION_PENDING` / `BLOCKED`：WebView2 白底真实渲染、100%/125%/150% DPI、Tab 顺序、键盘操作、Focus-visible、Narrator/NVDA、真实对比度、命中区域、最小窗口布局和中文/Unicode 长路径显示。

---

## P2：安装与发布

### W-P2-01 安装器

**验证方式：** Windows 实机 + CI；**状态：** 待实现。

验证全新安装、覆盖升级、自定义路径、非 ASCII 路径、Sidecar/aria2 资源、Native Host 注册、失败回滚、卸载保留/删除 `X-Archive` 数据。

### W-P2-02 签名与杀毒软件

**验证方式：** Windows 实机/发布环境；**状态：** 待实现。

验证安装包、Sidecar、Native Host 签名策略，SmartScreen、Windows Defender、实时扫描导致的文件锁、重试和日志脱敏。

### W-P2-03 Tauri Updater

**验证方式：** Windows 实机 + CI；**状态：** 待实现。

验证签名更新、更新前关闭子进程、失败回滚、保留数据库/归档、schema migration、Native Host 注册保持有效、组件版本可追踪。

### W-P2-04 第三方许可证

**验证方式：** CI + 发布审核；**状态：** 文档已建立，扫描待实现。

检查 gallery-dl、aria2、Python、Rust crates、npm packages、可选 yt-dlp/ffmpeg 的许可证、安装包副本、源代码获取方式和闭源/商业发行法律审查。

---

## 推荐执行顺序

```text
W-P0-01 → W-P0-02
→ W-P0-03（准备账号/Profile）→ W-P0-04（真实归档）
→ W-P1-01（Windows Tauri 复验）→ W-P1-02 → W-P1-03 → W-P1-04 → W-P1-05
→ W-P1-07 → W-P1-08 → W-P1-09 → W-P1-06
→ W-P2-01 → W-P2-02 → W-P2-03 → W-P2-04
```

如果当前阶段没有可用于 X 的测试账号，W-P0-03/W-P0-04 应保持为“外部环境阻塞”，不要用公开示例 URL 失败结果替代认证验证。

## Windows CI 最低工作流

```powershell
npm ci
npm run check
npm run test
npm run build
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
py -m venv .venv
.\.venv\Scripts\python.exe -m pip install --upgrade pip
.\.venv\Scripts\python.exe -m pip install -e .\sidecar pytest
.\.venv\Scripts\pytest.exe .\sidecar\tests -q
```

rustfmt/clippy 应在项目专用 CI/toolchain 中安装，不要求修改开发者全局工具链。

## Windows MVP 通过标准

- 全部 P0 项目完成。
- W-P1-01 至 W-P1-05 完成并通过。
- Edge Profile 至少完成图片和视频归档验证。
- Desktop 重启不重复下载。
- Native Host 只提供 allowlist 业务能力。
- 本地文件、SQLite 和 Job 状态一致。
- Token、Cookie、RPC Secret 不泄露。
- 安装、升级和卸载不会意外删除用户归档。
- Windows CI 的格式、lint、构建和测试全部通过。

## 本轮 Windows 平台复验（2026-09-09，Linux revision `add84c0`）

### Validation Environment

| 项目 | 实际值 |
|---|---|
| Linux source branch/revision | `main` / `add84c0950436b912671c5a451b2e3090300cb9f`；验证开始前 working tree clean |
| Linux working tree | 验证开始前无未提交修改；本轮仅修改本验证文档，未修改业务代码 |
| Windows 工作副本 | `E:\Shiraishi\VSCode Workspace\Tw2Tg` |
| Windows 系统/架构 | Windows 11 专业工作站版 Insider Preview `10.0.29661` / 64 位 |
| 运行时/工具链 | Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、rustfmt/clippy `1.9.0`、Python `3.14.7`、Tauri CLI `2.11.4` |
| 验证日期 | 2026-09-09 |

同步方向为 Linux source → E: Windows validation workspace。同步排除了 `.git`、`node_modules`、`.venv`、`target`、`dist` 和缓存/数据库文件，也排除了 Linux 端验证结果文档；未删除 E 盘本地依赖和构建产物。同步后对其余项目内容执行 `rsync --checksum`，无差异。

本次复验说明：同步前发现 E 盘副本曾落后于该 Linux revision，因此本轮先重新单向同步，再对最新副本重新执行全部适用命令；结果与既有复验一致，未产生新的 FAIL。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Node 依赖 | NOT RUN | 本轮未重复执行 `npm ci`；E 盘工作副本已有依赖且 `package-lock.json` 未变化。上一次成功安装和审计结果保留在当前基线 |
| Node 检查/测试/构建 | PASS | `npm run check`、`npm run test`、`npm run build`；Desktop 0 项 Node 测试，Extension 6 项通过 |
| Rust 格式/编译 | PASS | `cargo fmt --all -- --check`、`cargo check --workspace` |
| Rust 全量测试 | PASS | 当前进程设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后执行 `cargo test --workspace`；68 个 crate 单元测试全部通过，storage 16 项、telegram 12 项及 Desktop aria2 allowlist/SHA-256 5 项测试通过 |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | Windows workspace 测试通过 `telegram_send_state_round_trip`、重试/未发送列表、已发送状态约束，以及幂等发送的失败重试、已送达稳定性和已发送跳过 transport 测试；真实 Telegram 账号发送仍未执行 |
| Rust clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；Linux let-chain 修复已在 Windows 当前 revision 上复验通过 |
| Python Sidecar 测试 | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Sidecar Windows 路径/JSONL 进程链路 | NOT RUN | 本轮执行了 `.venv\Scripts\pytest.exe sidecar\tests -q` 并通过，但未重复执行独立的 Unicode 路径 JSONL 进程链路；既有通过证据保留在历史基线 |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version`、`npm run build:tauri`；生成 `target\release\xarchive-desktop.exe` |
| Tauri 开发启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe` |
| Tauri GUI 视觉验收 | BLOCKED | GUI 自动化 helper 无可用 native app target；仅确认启动日志 |

### Errors

| 失败/警告 | 分类与原因 | 对后续验证的影响 |
|---|---|---|
| 默认 Python 探测 | 已知环境问题；Windows PATH 中没有 `python3`，本轮在执行 Rust workspace 测试前显式设置项目 `.venv\Scripts\python.exe`，未再复现 `NotRunning` | 不阻塞本轮验证；后续仍应明确 Windows 测试的 Python 解析规则 |
| 受限沙箱直接执行 Node 脚本报 `EPERM: operation not permitted, lstat 'E:\\Shiraishi\\VSCode Workspace'` | Windows 工作区父目录的沙箱访问边界；使用受控权限重新执行后 `npm run check` 通过，不属于项目代码失败 | 不阻塞；后续 Windows 验证需保留该权限前提 |
| 上一次 `npm ci` 提示 `esbuild@0.28.2` postinstall script 未被 `allowScripts` 批准 | 依赖安装安全策略警告；本轮未重复执行 `npm ci`，Node 构建测试通过 | 不阻塞 |
| Rust/Tauri 构建输出 MSVC linker stdout `#[warn(linker_messages)]` | 工具链非阻塞 warning | 不阻塞 |
| 停止 Tauri 开发进程时出现 Chromium `Error = 1411`、`STATUS_CONTROL_C_EXIT` | 主动 Ctrl+C 停止时的窗口类注销/终止警告；启动阶段正常 | 不影响启动结论；GUI 视觉仍未确认 |

### Not Executed / Blocked / Not Applicable

| 验证项目 | 状态 | 原因 |
|---|---|---|
| Tauri GUI 前后端交互、视觉和资源路径人工验收 | BLOCKED | GUI 自动化 helper 无法提供可用 native app target |
| Edge Cookie、真实 X 认证和真实媒体归档 | BLOCKED | 缺少明确账号/Profile；公开 URL 失败不能替代认证验收 |
| Named Pipe transport、Native Host Registry/manifest、Edge/Chrome 实机 Extension | NOT RUN | Windows backend/Registry 前置功能尚未实现 |
| Tauri bundle、安装器、升级/卸载、真实 externalBin Sidecar | NOT RUN | `bundle.active=false`，externalBin 和安装器 artifact 尚未配置 |
| aria2 官方 ZIP 下载、PowerShell 解压、`aria2c.exe` 检测和真实进程集成 | NOT RUN | 当前环境无 `aria2c.exe`，项目也未提供受控 artifact；本轮未下载/安装外部 artifact 或修改系统设置 |
| aria2 断点、崩溃恢复、`.aria2` 清理、Unicode staging、403 回退 gallery-dl | BLOCKED | 依赖上一项真实 `aria2c.exe` 集成通过 |
| Windows 长路径、文件锁、磁盘不足、Tray/Single Instance/Autostart、Credential Manager | NOT RUN | 项目功能或 Windows backend 尚未具备专项验收前置条件 |
| 真实 Telegram 账号发送与持久化 | BLOCKED | 缺少真实账号、凭据和发送状态持久化环境；仅覆盖 HTTPS fake-server contract |
| Linux-only 的 pytest/cargo-clippy 重新执行 | NOT APPLICABLE | 本轮目标为 Windows；Windows 对应 pytest 和 clippy 已实际执行，Linux-only 重复执行不属于本轮范围 |

### Linux Follow-up

| 后续事项 | 原因 |
|---|---|
| Windows clippy re-validation（PASS） | Linux 已将 `desktop/src-tauri/src/lib.rs:83` 的嵌套 `if let` 改为 let-chain；Windows 当前 revision 的严格 workspace clippy 已通过 |
| 提供受控 `aria2c.exe` artifact 并完成 Windows 集成验证 | 当前新增下载管理 UI/命令和 Rust supervisor 仅完成单元/构建层验证，真实下载、解压、版本/hash、生命周期和恢复仍未执行 |
| GUI、Named Pipe/Registry、externalBin/安装器和真实账号链路 | 分别受 UI automation target、尚未实现的 Windows backend、未配置发布 artifact 和凭据/外部服务限制 |

本轮未为通过验证而修改业务代码、依赖或系统设置；以上事项交由 Linux 后续开发/验证任务处理。

### Linux Follow-up after Windows result reconciliation (2026-09-09)

> 注：本节为 Windows clippy re-validation 之前的历史记录，其中 `WINDOWS_VERIFICATION_PENDING` 状态已被下文 "Windows re-validation after Linux clippy fix" 与 "Linux reconciliation after Windows clippy re-validation" 小节更新为已通过；本节内容按原文保留。

| 项目 | 结果 | 说明 |
|---|---|---|
| `candidate_aria2_paths` clippy fix | LINUX_VERIFIED | 将 `desktop/src-tauri/src/lib.rs:83` 的嵌套 `if let` 改为 let-chain；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 通过 |
| Linux Rust workspace clippy | NOT RUN | 当前 Linux toolchain 未安装 `cargo-clippy`；命令返回 `cargo-clippy is not installed for the toolchain stable-x86_64-unknown-linux-gnu` |
| Linux Node workspace | PASS | `npm run check`、`npm run test`、`npm run build` 和 Extension check/test 通过；Extension 6 项测试通过 |
| Linux Python/schema checks | PASS | `python3 -m compileall -q sidecar` 和 shared JSON/schema 解析通过 |
| Windows clippy re-validation | WINDOWS_VERIFICATION_PENDING | Linux 修复尚未在当时的 Windows 工作副本重新执行；本历史表格保留原始 pending 记录 |

本轮 reconciliation 结论：Windows 已通过项目仍保持其原 PASS 记录；Windows clippy 的历史 FAIL 仍保留，原因已在 Linux 修复，但需要下一轮 Windows workspace clippy re-validation。aria2 官方 ZIP 下载、PowerShell 解压、`aria2c.exe` 实际检测/进程生命周期、断点恢复、崩溃恢复、`.aria2` 清理、Unicode staging、403 回退、GUI、Named Pipe/Registry、externalBin/安装器和真实账号项目继续保持 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_BLOCKED`、`NOT RUN` 或 `BLOCKED`，不提前标记 PASS。

### Windows re-validation after Linux clippy fix（2026-09-09）

当前 Linux let-chain 修复已同步至 E 盘，并完成 Windows re-validation；上一轮 clippy FAIL 记录保留为历史记录。

本轮重新同步后复验（2026-09-09）：针对 Linux revision `add84c0950436b912671c5a451b2e3090300cb9f` 重新执行 `npm run check/test/build`、`cargo fmt --all -- --check`、`cargo check --workspace`、设置项目 `PYTHON` 后的 `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`.venv\Scripts\pytest.exe sidecar\tests -q`、`npm run build:tauri` 和 `npm run dev:tauri`；结果全部为 PASS。Rust workspace 68 项测试全部通过，严格 clippy 复验通过。

| 验证项目 | 状态 | 关键结果 |
|---|---|---|
| Node check/test/build | PASS | Vite 构建通过；Extension 6 项测试通过 |
| Rust fmt/check/test | PASS | `cargo test --workspace` 通过，68 项测试全部通过 |
| Rust clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings` 通过 |
| Python sidecar | PASS | 10 passed |
| Tauri Release/dev | PASS | Release executable 生成，Vite、Rust Debug、Desktop 启动成功 |
| GUI 视觉验收 | BLOCKED | GUI automation helper 无可用 native app target |

aria2 官方 ZIP 下载、解压、`aria2c.exe` 检测和真实进程/恢复验证仍为 `NOT RUN`，原因是当前环境没有 `aria2c.exe`，本轮未安装外部 artifact 或修改系统设置。Edge/真实 X/Telegram、Named Pipe/Registry、externalBin/安装器仍为 `BLOCKED` 或 `NOT RUN`，原因分别是凭据缺失、Windows backend 未实现或发布 artifact 未配置。

本轮验证错误仅包括 MSVC linker stdout `#[warn(linker_messages)]` 和停止 Tauri 时的 Chromium `Error = 1411`/`STATUS_CONTROL_C_EXIT`，均不影响构建或启动结论。

Linux 后续处理：提供受控 aria2c artifact 并完成真实下载/解压/生命周期/恢复验证；继续实现 GUI、Named Pipe/Registry、externalBin/安装器和凭据相关链路。clippy 修复已完成 Windows re-validation，无需继续作为失败项处理。

本轮基线：Linux `main` / `add84c0950436b912671c5a451b2e3090300cb9f`，验证开始前 working tree clean；Windows `E:\Shiraishi\VSCode Workspace\Tw2Tg`，Windows 11 Insider Preview `10.0.29661` / 64 位，Node `v24.19.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。同步排除 `.git`、依赖、缓存、构建产物和 Linux 验证文档，其他内容 `rsync --checksum` 校验通过。

### Linux reconciliation after Windows clippy re-validation (2026-09-09)

本轮 Linux 重新读取上述 Windows re-validation 结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成 reconciliation。上一轮 `collapsible_if` FAIL 的处理链路已闭环，历史 FAIL 记录保留：

```text
Previous Windows validation: FAIL (clippy::collapsible_if at desktop/src-tauri/src/lib.rs:83)

Linux fix: let-chain rewrite of candidate_aria2_paths

Linux verification: PASS (cargo fmt --all -- --check, cargo check --workspace,
cargo test --workspace, npm run check/test/build, Extension tests,
python3 -m compileall, JSON/schema parse)

Current Windows status: WINDOWS_PASS (strict workspace clippy re-validation)
```

| 项目 | 结果 | 说明 |
|---|---|---|
| Desktop aria2 `candidate_aria2_paths` clippy 修复 | WINDOWS_PASS | Windows `cargo clippy --workspace --all-targets -- -D warnings` re-validation 通过；let-chain 修复已在 Windows 确认，clippy 失败链路闭环 |
| 本轮 Linux 代码修改 | 无新增 | Windows 结果未引入新的代码失败；本轮仅做文档 reconciliation，未修改业务代码 |
| Linux 适用回归 | PASS | 重新执行 Rust fmt/check/test、Node check/test/build、Extension 测试、Python compileall 和 JSON/schema 解析，全部通过 |
| Linux cargo clippy | NOT RUN | Linux toolchain 未安装 `cargo-clippy`；clippy 结论以 Windows 严格 clippy re-validation 为准 |
| aria2c.exe 实际下载/解压/进程生命周期/断点与崩溃恢复/`.aria2` 清理/Unicode staging/403 回退 | NOT RUN | 依赖受控 `aria2c.exe` artifact 与 Windows 环境授权；状态保持待下一轮 Windows 验证 |
| GUI 视觉验收 | BLOCKED | UI automation helper 仍无可用 native app target |
| Named Pipe/Registry、externalBin/安装器、Edge Cookie/真实 X、Credential Manager、真实 Telegram 发送 | BLOCKED / NOT RUN | 分别受未实现 Windows backend、未配置发布 artifact 和凭据/外部服务限制 |

下一轮 Windows 验证重点保持不变：在获得受控 `aria2c.exe` artifact 后执行官方 ZIP 下载、PowerShell 解压、版本/hash 校验、进程生命周期、断点/崩溃恢复、`.aria2` 清理、Unicode staging 和 403 回退 gallery-dl；GUI 视觉验收、Named Pipe/Registry、externalBin/安装器和真实账号链路按各自前置条件推进。当前没有因 Windows 验证结果产生的待修复 Linux 代码问题。
### Linux reconciliation after Windows re-validation of revision `add84c0`（2026-09-09）

Windows 针对干净 working tree 的 Linux revision `add84c0950436b912671c5a451b2e3090300cb9f` 重新同步并完成全量复验。Linux 重新读取结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/test、严格 clippy `-D warnings`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部在 Windows 实际执行并通过；clippy 失败链路（`collapsible_if`）在当前 revision 上确认闭环，无遗留待修复 lint |
| Telegram 发送状态持久化与幂等补传（单元层：migration 应用、`SendStateStore` 语义、`send_idempotently`） | WINDOWS_PASS（单元层） | Windows `cargo test --workspace` 实际包含 storage 16 项、telegram 12 项并通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 现有测试使用 in-memory SQLite，未执行基于文件 DB 和应用重启的专项验证；保持 pending，不得提前标记 PASS |
| 真实 Telegram 账号发送、Credential Manager | BLOCKED | 依赖真实账号/凭据和未实现的 Windows backend，本轮无变化 |
| 测试计数差异 | 待复核 | Windows 报告 `cargo test --workspace` 共 68 项；Linux 同一 revision 复测为 69 项（11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram）。Windows 自报分项（storage 16、telegram 12）与 Linux 一致，差异最可能为计数笔误，但未经 Windows 端确认前按差异记录；下一轮 Windows 验证需按 crate 重新清点并回填 |
| Sidecar 手动 Unicode 路径 JSONL 进程链路 | NOT RUN（保留） | 本轮 pytest 10 项通过，但未重复独立手动链路；既有通过证据保留在历史基线 |
| Windows 环境记录（python3 缺失、沙箱 `EPERM lstat` 父目录、esbuild postinstall 警告） | 环境事项 | 分别通过显式 `PYTHON`、受控权限复跑处理或为非阻塞警告；不属于项目代码失败，无需 Linux 代码修改 |

本轮 Plan 重新评估结论：原 Plan（Telegram 发送状态持久化 + 幂等补传）已完成实现、Linux 验证和 Windows 单元级验证，Plan 无剩余步骤；Windows 结果未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点：按 crate 清点测试总数并回填差异；对发送状态持久化执行基于文件 SQLite、应用重启恢复和迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件继续 `WINDOWS_VERIFICATION_PENDING` / `WINDOWS_BLOCKED` / `NOT RUN`。

### Windows validation rerun for Linux revision `4d4b3f5`（2026-09-09）

本轮针对最新 Linux revision `4d4b3f5a16584cf209edefa94688554443fc8ff6` 执行验证。该 revision 仅为上一轮 Windows 结果的文档 reconciliation；验证开始前 Linux working tree 只有本验证文档未提交修改，业务代码无新增改动。E 盘副本由 Linux source 重新单向同步，验证文档本身按规则排除。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、`node_modules`、`.venv`、`target`、`dist`、缓存/数据库和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram migration、状态往返、重试计数和幂等发送测试通过；真实账号及应用级文件 DB 重启恢复仍未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；Ctrl+C 停止后无遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

本轮错误和未执行项：

- 初次同步后的终审发现 E 盘副本仍有三份开发文档落后于 Linux revision；重新执行同一 Linux→E: 同步后，排除本地依赖/构建产物/验证文档的 `rsync --checksum` 复核通过。该问题属于同步工作流/环境状态，不属于项目代码失败。
- Windows PATH 没有 `python3`，因此 Rust 测试显式使用项目 `.venv\Scripts\python.exe`；未产生测试失败。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于非阻塞 warning。
- `aria2c.exe` 不在 PATH，且项目未提供受控 artifact；aria2c 下载/解压/进程生命周期/恢复为 `NOT RUN`，依赖项为 `BLOCKED`。
- GUI 视觉验收为 `BLOCKED`，缺少可用 GUI automation native app target。
- Edge Cookie、真实 X、真实 Telegram 为 `BLOCKED`，缺少账号/Profile/凭据。
- Named Pipe、Registry/Host manifest、浏览器 Extension 实机、externalBin、安装器、Tray/Autostart、Credential Manager 为 `NOT RUN` 或 `BLOCKED`，相关 Windows backend、发布 artifact 或专项前置条件尚未具备。

本轮已按 crate 清点确认 Windows 测试总数为 69 项，解决此前文档中的 68/69 计数差异。未发现属于项目代码的 Windows `FAIL`。Linux 后续事项保持为：提供受控 `aria2c.exe` artifact，完成文件 SQLite/应用重启恢复专项，以及 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。

### Linux reconciliation after Windows validation of revision `4d4b3f5`（2026-09-09）

Windows 针对最新 Linux revision `4d4b3f5a16584cf209edefa94688554443fc8ff6`（仅含上一轮文档 reconciliation，无业务代码改动）重新同步并完成全量复验。Linux 重新读取结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；`4d4b3f5` 与 `add84c0` 业务代码一致，结论可覆盖两者 |
| 68/69 测试计数差异 | 已关闭 | Windows 本轮按 crate 清点确认 69 项（11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram），与 Linux 计数一致；上一轮差异确认为计数笔误，历史"待复核"记录保留 |
| Telegram 发送状态持久化与幂等补传（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项在 Windows 实际执行并通过，状态与上一轮一致 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 本轮仍未执行专项验证，状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成及依赖项（断点/崩溃恢复/`.aria2` 清理/Unicode staging/403 回退） | NOT RUN / BLOCKED | 环境仍无 `aria2c.exe` 且未提供受控 artifact |
| GUI 视觉验收、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | BLOCKED / NOT RUN | 各自前置条件（automation target、Windows backend、发布 artifact、账号/凭据）均未具备，无变化 |
| 首次同步后 E 盘三份开发文档落后于 Linux revision | 已修复的环境事项 | 重新执行同一 Linux→E: 同步后 `rsync --checksum` 复核通过；属于同步工作流/环境状态，不属于项目代码失败，无需 Linux 代码修改 |

本轮 Plan 重新评估结论：原 Plan 已无剩余步骤；Windows 复验结果未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

### Windows validation rerun for latest Linux revision `facd8d7`（2026-09-09）

本轮针对最新 Linux revision `facd8d70017bafbc695384a19b21812b72d5339e` 执行验证。该 revision 仅包含上一轮 Windows 验证结果的文档 reconciliation，验证开始前 Linux working tree clean，业务代码无新增改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、`node_modules`、`.venv`、`target`、`dist`、缓存/数据库和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram 的状态、重试、migration 和幂等发送测试全部通过；真实账号及应用级文件 DB 重启恢复未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；Ctrl+C 停止后无遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

本轮错误和未执行项：

- Windows PATH 没有 `python3`，Rust 测试显式使用项目 `.venv\Scripts\python.exe`，未产生测试失败。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于非阻塞 warning。
- `aria2c.exe` 不在 PATH，项目未提供受控 artifact；aria2c 下载、解压、进程生命周期、断点/崩溃恢复和 403 回退为 `NOT RUN` 或依赖阻塞。
- GUI 视觉验收为 `BLOCKED`，缺少可用 GUI automation native app target。
- Edge Cookie、真实 X、真实 Telegram 为 `BLOCKED`，缺少账号/Profile/凭据。
- Named Pipe、Registry/Host manifest、浏览器 Extension 实机、externalBin、安装器、Tray/Autostart、Credential Manager 为 `NOT RUN` 或 `BLOCKED`，相关 Windows backend、发布 artifact 或专项前置条件尚未具备。

本轮确认 Windows workspace 测试总数为 69 项，未发现项目代码导致的 Windows `FAIL`。Linux 后续事项保持为：提供受控 `aria2c.exe` artifact，完成文件 SQLite/应用重启恢复专项，以及 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。

### Windows validation rerun for Linux revision `facd8d7`（2026-09-09）

本轮针对最新 Linux revision `facd8d70017bafbc695384a19b21812b72d5339e` 执行验证。该 revision 仅包含上一轮 Windows 结果的文档 reconciliation；验证开始前 Linux working tree clean，业务代码无新增改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、依赖、缓存、构建产物和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram 状态、重试、迁移和幂等发送测试全部通过；真实账号及应用级文件 DB 重启恢复未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；主动 Ctrl+C 停止后未发现遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

本轮未发现项目代码导致的 Windows `FAIL`。环境/阻塞事项如下：

- Windows PATH 没有 `python3`，Rust 测试显式使用项目 `.venv\Scripts\python.exe`，未产生测试失败。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于非阻塞 warning。
- `aria2c.exe` 不在 PATH，项目未提供受控 artifact；aria2c 下载、解压、进程生命周期、断点/崩溃恢复和 403 回退为 `NOT RUN` 或依赖阻塞。
- GUI 视觉验收为 `BLOCKED`，缺少可用 GUI automation native app target。
- Edge Cookie、真实 X、真实 Telegram 为 `BLOCKED`，缺少账号/Profile/凭据。
- Named Pipe、Registry/Host manifest、浏览器 Extension 实机、externalBin、安装器、Tray/Autostart、Credential Manager 为 `NOT RUN` 或 `BLOCKED`，相关 Windows backend、发布 artifact 或专项前置条件尚未具备。

本轮确认 Windows workspace 测试总数为 69 项，上一轮 68/69 计数差异已关闭。Linux 后续事项保持为：提供受控 `aria2c.exe` artifact，完成基于文件 SQLite/应用重启恢复专项，并继续推进 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。
### Linux reconciliation after Windows validation of revision `facd8d7`（2026-09-09）

Windows 针对最新 Linux revision `facd8d70017bafbc695384a19b21812b72d5339e`（仅含上一轮文档 reconciliation，无业务代码改动）重新同步并完成全量复验。Linux 重新读取结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；`facd8d7` 业务代码与 `4d4b3f5`/`add84c0` 一致，结论可覆盖三者 |
| 测试计数 | 已确认一致 | Windows 按 crate 清点确认 69 项（11/5/10/4/7/4/16/12），与 Linux 复测一致；68/69 差异维持关闭状态 |
| 发送状态持久化（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项在 Windows 实际执行并通过，连续三轮保持一致 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 本轮仍未执行专项验证，状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成及依赖项、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 各自前置条件均未具备，无变化 |
| Windows 写回记录重复 | 文档事项 | 本轮 Windows 结果在同一文档中写入了两个内容相同的 `facd8d7` 复验小节（标题措辞略异）；按历史保留规则两节均不删除，仅在此记录重复事实。属于验证文档书写习惯问题，不属于项目代码或验证结论问题 |

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；Windows 复验未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。当前状态对 `add84c0`、`4d4b3f5`、`facd8d7` 三个 revision 保持一致。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。另建议 Windows 端后续写回结果时避免为同一 revision 重复创建小节。

### Windows validation rerun for latest Linux revision `5fbc675`（2026-09-09）

本轮针对最新 Linux revision `5fbc675fd6dec4a415776c868591bf268da53a53` 执行验证。该 revision 仅包含上一轮 Windows 验证结果的文档 reconciliation；验证开始前 Linux working tree clean，业务代码无新增改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、依赖、缓存、构建产物和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram 状态、重试、migration 和幂等发送测试全部通过；真实账号及应用级文件 DB 重启恢复未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；主动 Ctrl+C 停止后未发现遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

### Linux reconciliation after Windows validation of revision `5fbc675`（2026-09-09）

Windows 针对最新 Linux revision `5fbc675fd6dec4a415776c868591bf268da53a53`（仅含上一轮文档 reconciliation）重新同步并完成全量复验，本轮写回为单一小节，无重复。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；按 crate 清点 69 项与 Linux 一致 |
| 发送状态持久化（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项连续四轮在 Windows 实际执行并通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 仍未执行专项验证，状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

流程效率说明：经 Linux 端核实，`add84c0..HEAD` 业务代码（`crates`、`desktop/src-tauri/src`、`desktop/src`）为零改动，`add84c0`、`4d4b3f5`、`facd8d7`、`5fbc675` 四轮 Windows 复验对象为同一业务代码状态，结论一致。后续 Windows 轮次对仅含文档 reconciliation 的 revision 无需重复执行全量复验，可将验证资源集中于：含业务代码改动的 revision、新解除阻塞的专项（send-state 应用层验证、aria2c artifact 集成）或此前 `BLOCKED`/`NOT RUN` 项的前置变化。

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；Windows 复验未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

本轮未发现项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，故 Rust 测试显式使用项目 `.venv\Scripts\python.exe`；构建出现 MSVC linker stdout `#[warn(linker_messages)]` 非阻塞 warning。`aria2c.exe` 不在 PATH 且未提供受控 artifact，相关下载/解压/生命周期/恢复项目为 `NOT RUN` 或 `BLOCKED`；GUI、Edge Cookie、真实 X/Telegram、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager 等因缺少 automation target、backend、artifact 或凭据而为 `BLOCKED` / `NOT RUN`。

Linux 后续事项：提供受控 `aria2c.exe` artifact，完成基于文件 SQLite/应用重启恢复/迁移升级专项，并继续推进 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。

### Windows validation of latest Linux revision `f3faea3`（2026-09-09）

本轮针对最新 Linux revision `f3faea35b9bf836518ff753dc39675bbe56bd5dc` 执行 Windows 验证。该 revision 相比已完成 Windows 全量验证的 `5fbc675` 仅包含文档 reconciliation，`crates`、`desktop/src-tauri/src`、`desktop/src`、`extension`、`sidecar` 和 `shared` 均无业务代码差异；验证开始前 Linux working tree clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 Windows 本地 `.venv`、`node_modules`、`target`、`desktop\dist`，排除依赖、缓存、构建产物和验证文档后 checksum dry-run 通过 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过，Desktop Node tests 0 项 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 成功启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe`；主动 Ctrl+C 停止，未发现残留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

### Linux reconciliation after Windows validation of revision `f3faea3`（2026-09-09）

Windows 针对最新 Linux revision `f3faea35b9bf836518ff753dc39675bbe56bd5dc`（仅含上一轮文档 reconciliation，单一小节写回，无重复）完成全量复验。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；69 项测试按 crate 清点与 Linux 一致（11/5/10/4/7/4/16/12） |
| 发送状态持久化（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项连续五轮在 Windows 实际执行并通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 本轮仍未执行专项验证（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

流程效率说明（延续上一轮结论）：Linux 端核实 `add84c0..f3faea3` 全部业务源码目录（`crates`、`desktop/src-tauri/src`、`desktop/src`、`extension`、`sidecar`、`shared`）diff 为空，本轮复验对象与此前四轮为同一业务代码状态，结论一致。再次明确：后续 Windows 轮次对仅含文档 reconciliation 的 revision 无需重复执行全量复验；仅在出现含业务代码改动的 revision、新解除阻塞的专项（send-state 应用层验证、aria2c artifact 集成）或 `BLOCKED`/`NOT RUN` 项前置变化时执行相应验证。

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；Windows 复验未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

本轮未发现项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，使用项目 `.venv\Scripts\python.exe` 后 pytest 和 Rust 测试均通过；Rust/Tauri 构建的 MSVC linker stdout `#[warn(linker_messages)]` 为非阻塞 warning。`aria2c.exe` 不在 PATH 且未提供受控 artifact，aria2c 下载/解压/生命周期/断点恢复/崩溃恢复/403 回退为 `NOT RUN` 或 `BLOCKED`。基于文件 SQLite 的应用级重启恢复和迁移升级为 `NOT RUN`；GUI 视觉、Edge Cookie、真实 X/Telegram、Named Pipe/Registry、浏览器 Extension 实机、externalBin/安装器、Tray/Autostart、Credential Manager 因缺少 automation target、backend、artifact 或凭据而为 `BLOCKED` / `NOT RUN`。Sidecar 手动 Unicode JSONL 链路本轮未重复执行，既有通过证据保持有效。

Linux 后续事项：继续提供受控 `aria2c.exe` artifact，完成文件 SQLite/应用重启恢复/迁移升级专项，并推进 GUI、Windows backend、externalBin/安装器、凭据、真实账号及其他缺失前置条件的验证。由于本轮没有业务代码改动，不需要 Linux 代码修复或扩大 Plan。

### Linux reconciliation after Windows validation of revision `55bcdc8`（2026-09-09）

Windows 针对最新 Linux revision `55bcdc80ab82357983bcfbdd8350a343f72615c1`（仅含上一轮文档 reconciliation）按轻量模式复核：执行同步 checksum、`npm run check`、`cargo fmt/check` 和 Tauri CLI 版本检查（PASS），并将 Node test/build、Rust 全量测试、严格 clippy、pytest 和 Tauri Release/Debug 标记为 `NOT APPLICABLE`——因业务代码与已全量验证的 `f3faea3` 完全一致，既有通过证据继续有效。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows 轻量 check（同步一致性、Node check、Rust fmt/check、Tauri CLI 入口） | WINDOWS_PASS | 实际执行并通过；全量套件按效率指引标记 `NOT APPLICABLE`，非未执行遗漏 |
| 发送状态持久化（单元层） | WINDOWS_PASS（继承） | `f3faea3` 全量证据对相同业务代码继续有效；Linux 同步复测 69/69 通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 专项仍未执行（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

本轮确认 Windows 端已采纳「纯文档 revision 轻量复核」的工作方式，文档与验证成本显著降低，且未牺牲结论有效性。本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；无任何属于项目代码的 Windows FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

### Windows validation review for latest Linux revision `55bcdc8`（2026-09-09）

本轮针对最新 Linux revision `55bcdc80ab82357983bcfbdd8350a343f72615c1` 执行 Windows 平台复核。该 revision 相比已完成全量 Windows 验证的 `f3faea3` 仅包含验证结果的文档 reconciliation；业务源码目录无变化，Linux working tree 在验证开始前 clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
### Linux reconciliation after Windows validation of revision `8318569`（2026-09-09）

Windows 针对最新 Linux revision `831856946e76785d4efd9a531a6e9062e41fef52`（仅含上一轮文档 reconciliation）延续轻量复核：同步 checksum、`npm run check`、`cargo fmt/check` 和 Tauri CLI 版本检查 PASS；全量测试套件因业务代码与已全量验证的 `f3faea3` 一致标记 `NOT APPLICABLE`，既有证据继续有效。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows 轻量 check（同步一致性、Node check、Rust fmt/check、Tauri CLI 入口） | WINDOWS_PASS | 实际执行并通过；全量套件 `NOT APPLICABLE`，非未执行遗漏 |
| 发送状态持久化（单元层） | WINDOWS_PASS（继承） | Linux 同步复测 69/69 通过；`f3faea3` 全量证据对相同业务代码继续有效 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 专项仍未执行（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；无任何属于项目代码的 Windows FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。轻量复核模式运转正常。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘 `.venv`、`node_modules`、`target`、`desktop\dist`，排除 `.git`、依赖、缓存、构建产物、数据库和验证文档后 checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | NOT APPLICABLE | `f3faea3` 已对相同业务代码全量执行并通过（Rust 69 项、sidecar 10 项、Release/Debug）；`55bcdc8` 无业务源码或配置变化，按项目文档不重复执行 |
| Telegram 发送状态持久化单元层 | NOT APPLICABLE | 与上一轮相同业务状态，既有 Windows storage 16 项和 telegram 12 项通过证据继续有效 |
### Linux reconciliation after Windows validation of revision `5d9dbd9`（2026-09-09）

Windows 针对最新 Linux revision `5d9dbd9720f72642fe594969b92ce075db765253`（仅含上一轮文档 reconciliation）延续轻量复核：同步 checksum、`npm run check`、`cargo fmt/check` 和 Tauri CLI 版本检查 PASS；全量测试套件因业务代码与已全量验证的 `f3faea3` 一致标记 `NOT APPLICABLE`，既有证据继续有效。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows 轻量 check（同步一致性、Node check、Rust fmt/check、Tauri CLI 入口） | WINDOWS_PASS | 实际执行并通过；全量套件 `NOT APPLICABLE`，非未执行遗漏 |
| 发送状态持久化（单元层） | WINDOWS_PASS（继承） | Linux 同步复测 69/69 通过；`f3faea3` 全量证据对相同业务代码继续有效 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 专项仍未执行（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；无任何属于项目代码的 Windows FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围；轻量复核模式运转正常。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 专项仍未执行，缺少相应应用级验证场景 |
| aria2c artifact 集成及下载/恢复链路 | NOT RUN / BLOCKED | `aria2c.exe` 不在 PATH，项目未提供受控 artifact |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装外部 artifact、未修改系统设置。

本轮没有项目代码导致的 Windows `FAIL`。Windows PATH 仍没有 `python3`，但本轮轻量 check 未依赖该命令；历史 Rust/Tauri MSVC linker stdout warning 为非阻塞环境输出。Linux 后续仅需继续处理 aria2c artifact、文件 SQLite/重启恢复/迁移专项，以及 GUI/backend、安装器、凭据和真实账号验证前置条件；不需要因本轮文档-only revision 修改业务代码。

### Windows validation review for latest Linux revision `8318569`（2026-09-09）

本轮针对最新 Linux revision `831856946e76785d4efd9a531a6e9062e41fef52` 执行 Windows 平台复核。该 revision 相比已完成全量 Windows 验证的 `f3faea3` 仍仅包含验证文档 reconciliation；业务源码目录无变化，Linux working tree 在验证开始前 clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘依赖、`.venv`、Rust target 和 desktop 构建目录，checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | NOT APPLICABLE | 同一业务代码状态已在 `f3faea3` 全量通过（Rust 69 项、sidecar 10 项、Release/Debug）；当前 revision 无业务源码或配置变化，按项目文档不重复执行 |
| Telegram 发送状态持久化单元层 | NOT APPLICABLE | 既有 Windows storage 16 项和 telegram 12 项通过证据继续有效 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 缺少应用级专项验证场景，状态不变 |
| aria2c artifact 集成及下载/恢复链路 | NOT RUN / BLOCKED | `aria2c.exe` 不在 PATH，项目未提供受控 artifact |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境沿用并复核为：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装外部 artifact、未修改系统设置。

本轮没有项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，但本轮轻量 check 未依赖该命令；既有 MSVC linker stdout warning 为非阻塞环境输出。Linux 后续继续处理 aria2c artifact、文件 SQLite/重启恢复/迁移专项，以及 GUI/backend、安装器、凭据和真实账号验证前置条件；不扩大为开发任务。

### Windows validation review for latest Linux revision `5d9dbd9`（2026-09-09）

本轮针对最新 Linux revision `5d9dbd9720f72642fe594969b92ce075db765253` 执行 Windows 平台复核。该 revision 仅包含上一轮轻量 Windows 验证结果的文档 reconciliation；业务源码目录无变化，Linux working tree 在验证开始前 clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘依赖、`.venv`、Rust target 和 desktop 构建目录，checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | NOT APPLICABLE | `f3faea3` 已对相同业务代码全量通过（Rust 69 项、sidecar 10 项、Release/Debug）；当前 revision 无业务源码或配置变化，按项目文档不重复执行 |
| Telegram 发送状态持久化单元层 | NOT APPLICABLE | 既有 Windows storage 16 项和 telegram 12 项通过证据继续有效 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 缺少应用级专项验证场景，状态不变 |
| aria2c artifact 集成及下载/恢复链路 | NOT RUN / BLOCKED | `aria2c.exe` 不在 PATH，项目未提供受控 artifact |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装外部 artifact、未修改系统设置。

本轮没有项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，但轻量 check 未依赖该命令；既有 MSVC linker stdout warning 为非阻塞环境输出。Linux 后续继续处理 aria2c artifact、文件 SQLite/重启恢复/迁移专项，以及 GUI/backend、安装器、凭据和真实账号验证前置条件；不扩大为开发任务。

### Windows validation review for latest Linux revision `d239a1d`（2026-09-09）

本轮针对最新 Linux revision `d239a1dbb7bc8ee9f3b851ba2378a915e3312017` 执行 Windows 平台复核。该 revision 仅包含上一轮轻量 Windows 验证结果的文档 reconciliation；业务源码目录无变化，验证开始前仅有既存验证文档 working-tree 改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘依赖、`.venv`、Rust target 和 desktop 构建目录，checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | PASS | 用户授权下载组件后复跑；Node Extension 6 项、Rust workspace 69 项、sidecar pytest 10 项、严格 clippy、Release/Debug 均通过 |
| Telegram 发送状态持久化单元层 | PASS | storage 16 项、telegram 12 项 Windows 单元测试通过 |
| aria2 官方 Windows x64 artifact 下载、SHA-256、解压和版本 | PASS | 下载 `aria2-1.37.0-win-64bit-build1.zip`；SHA-256 `67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288` 与 allowlist 一致；解压后 `aria2c --version` 为 1.37.0 |
| aria2 RPC、Unicode/空格路径下载、暂停/恢复、Range 续传和 hash | PASS | 使用本地 Range-capable HTTP fixture；`aria2.getVersion`、本地下载、`pause`/`unpause`、16 MiB Release artifact Range 续传及 SHA-256 全部通过 |
| aria2 进程中断恢复与 `.aria2` 清理 | PASS | 模拟终止 aria2 进程后重新启动并续传；最终 SHA-256 一致，`.aria2` 控制文件完成后清理，验证进程正常停止 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 缺少应用级专项验证场景，状态不变 |
| aria2 403 回退 gallery-dl、真实 X/CDN | BLOCKED | 仅使用本地 HTTP fixture；真实外部服务、凭据和回退链路未具备 |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装系统组件、未修改 PATH 或系统设置。官方 aria2 ZIP 和本轮测试 artifact 后续已移动至 E 盘独立开发目录 `E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2\`。

本轮发现并已隔离的验证脚本问题：首次暂停尝试使用 Python 标准 HTTP server，因路径参数转义失败；修正后又因该 server 不支持 Range 响应导致 aria2 `Invalid range header`，另一次人为设置 `always-resume=false` 的暂停脚本触发 aria2 `Piece.cc:309` assertion。上述均属于测试 fixture/参数设置问题；使用正确的 Range server 和 aria2 默认续传设置重跑后，RPC、暂停/恢复、断点续传、进程中断恢复和 `.aria2` 清理均 PASS，未归类为项目代码 FAIL。

Linux 后续事项：继续完成文件 SQLite/重启恢复/迁移专项，补齐 aria2 403 回退、GUI/backend、安装器、凭据和真实账号验证前置条件；aria2 官方 artifact 的基础 Windows 集成与恢复验证已完成，不需要因本轮结果修改业务代码。

### Windows aria2 artifact 半永久保存（2026-09-09）

用户授权后，本轮将验证所需文件从 Windows 验证副本的 `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\aria2\` 移动至独立开发 artifact 目录：

`E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2\`

保存内容包括：官方 `aria2-1.37.0-win-64bit-build1.zip`、解压后的 `aria2c.exe` 及许可证/说明文件、Range-capable 本地测试 server、aria2 RPC/恢复日志、session、下载结果和失败尝试日志。ZIP SHA-256 为 `67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288`，与项目 allowlist 一致；`aria2c.exe --version` 为 `1.37.0`。

该目录属于 E 盘 Windows 本地开发/验证 artifact，不纳入 Linux→Windows 源同步，不反向同步到 WSL，不写入 PATH，也不安装系统服务。旧验证副本中的 `validation-artifacts\aria2\` 已确认不存在。

### Windows validation for latest Linux revision `34b5c67`（2026-09-10）

本轮针对最新 Linux revision `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 执行 Windows 平台验证。该 revision 包含 Dashboard 的加载状态、系统健康判断、错误播报、紧凑导航可访问名称和仅 Windows 显示 aria2 面板修补，以及对应路线图更新；验证开始前 Linux `main` working tree clean，未包含未提交修改。

### Validation Environment

- Windows 11 Insider Preview `10.0.29661.0` / 64 位。
- Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。
- Windows 工作副本：`E:\Shiraishi\VSCode Workspace\Tw2Tg`；该目录为非 Git 验证副本。
- 同步方向：WSL `/home/shiraishi/VSCode Workspace/Tw2Tg` → Windows E 盘；使用 Robocopy `/E /XJ /FFT /COPY:DAT /DCOPY:DAT`，未使用镜像删除。
- 排除 `.git`、`node_modules`、`.venv`、`target`、Desktop 构建目录、缓存、数据库、`.env` 和本地验证报告；E 盘既有依赖/构建产物/本地验证目录保持不变。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与关键文件一致性 | PASS | Robocopy 单向同步完成；`desktop/src/main.jsx`、`desktop/src/style.css`、`docs/development/roadmap.md` 与 WSL 源文件内容一致，依赖、缓存、构建产物和本地 artifact 未被覆盖 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules 构建通过，Extension 6 项测试通过，Desktop Node 测试为 0 项且无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后 `cargo test --workspace` 通过 69 项：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version` 为 2.11.4；`npm run build:tauri` 成功生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe`；启动期间进程可见，Ctrl+C 后 Tw2Tg 相关进程为 0 |
| aria2 官方 artifact 与下载/恢复链路 | NOT APPLICABLE | aria2 逻辑和 artifact 自上一轮 `d239a1d` Windows 实测后未变化；既有 aria2 RPC、Unicode/空格路径、Range 续传、进程恢复和 `.aria2` 清理 PASS 证据继续有效，本轮变化仅涉及 GUI 展示条件 |
| GUI 视觉、WebView2/DPI、键盘和辅助技术人工验收 | BLOCKED | 当前环境没有可用 GUI automation native app target；本轮只能确认构建和 Debug 启动，不能据此宣称真实渲染验收通过 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 当前仍缺少应用级专项场景；Rust repository 单元测试通过不替代 Desktop 现场重启/迁移验证 |
| Named Pipe、Native Host/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | Windows backend、发布 artifact、浏览器实机、automation target、账号或凭据等前置条件仍未具备 |

### Errors

- 首次直接执行 `cargo test --workspace` 时，`xarchive-sidecar-supervisor` 的 `spawn_ready_completes_the_hello_handshake` 和 `communicates_with_a_real_python_worker_when_available` 报 `NotRunning`。原因是 Windows 环境未显式设置项目要求的 `PYTHON`，不是业务代码失败；设置 `PYTHON` 指向项目 `.venv\Scripts\python.exe` 后，目标测试 4/4 和完整 workspace 69/69 均通过。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，只记录生成 `.lib/.exp` 的非阻塞 warning，不影响构建或测试结论。
- 停止 Tauri Debug 时记录 Chromium `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` 和 `STATUS_CONTROL_C_EXIT`。这是 Ctrl+C 主动终止开发进程时的 Windows 运行时清理输出；最终无 Tw2Tg 遗留进程，未导致启动验证失败。若未来要求无告警退出，应作为独立 Windows 生命周期问题调查，不在本轮修改业务代码。

### Linux Follow-up

本轮没有发现属于项目代码的 Windows `FAIL`，无需因验证结果修改业务代码。Linux 后续处理事项为：

- 在具备 GUI automation native app target 后，重新验证最新 Dashboard 的真实 WebView2/DPI 渲染、初始加载/错误播报、紧凑导航 aria-label、aria2 仅 Windows 显示、键盘焦点和辅助技术反馈；
- 设计并执行基于文件 SQLite 的 Desktop 应用重启恢复和 `0001 → 0002` 迁移专项；
- 继续补齐 Named Pipe/Registry、真实 externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram 等各自前置条件；
- 可选地单独调查 Tauri Ctrl+C 后的 Chromium `Error = 1412` 清理警告。

### Linux reconciliation after Windows validation of revision `34b5c67`（2026-09-10）

Linux 重新读取了本轮 Windows 验证结果和当前 Plan。Windows 针对 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 的 Node、Rust fmt/check/clippy/test、Python Sidecar、Tauri Debug/Release 构建和进程清理均通过；aria2 相关验证继承上一轮 `d239a1d` 的实际 Windows PASS。本轮没有项目代码导致的 Windows `FAIL`。

| 事项 | 状态 | 当前处理 |
|---|---|---|
| `34b5c67` 已同步的 GUI 修补：systemReady、initialLoad、错误播报、紧凑导航 `aria-label`、aria2 Windows gating、最近任务命名 | WINDOWS_PASS | Windows 已对这些 revision 中的实际代码执行构建/测试；真实 GUI 渲染结论仍不由构建结果替代 |
| Linux 后续 GUI 语义修补：Widget 级错误恢复、未实现导航项移除、Job `<ul>/<li>`、`<time>`、共享 `:focus-visible`、`prefers-reduced-motion` | LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING | Linux 已通过 Vite check/build、Desktop Node test、Rust fmt/check/test；这些新增代码尚未在 Windows 复验，不提前标记 `WINDOWS_PASS` |
| GUI 真实 WebView2/DPI、Tab 顺序、Focus-visible 实际表现、命中目标、Narrator/NVDA 和最终对比度 | WINDOWS_VERIFICATION_PENDING | 本轮仍因 GUI automation native app target 不可用而 BLOCKED；下一次具备 target 后执行 W-P1-11 |
| 文件 SQLite Desktop 应用重启、遗留 staging、`0001 → 0002` 迁移升级 | NOT RUN | 仍缺少应用级专项验证场景；Repository 单元测试不能替代现场恢复验证 |
| Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | WINDOWS_VERIFICATION_PENDING / BLOCKED / NOT RUN | 依赖 Windows backend、发布 artifact、浏览器实机、automation target、账号或凭据；不因本轮基础测试通过而关闭 |

本轮 Linux Plan 重新评估：systemReady、initialLoad、错误播报、Widget 级错误恢复/重试、用户级错误区域标题、紧凑导航可访问名称、未实现导航项移除、aria2 Windows gating、最近任务命名、Job/时间语义、Focus-visible 和 reduced-motion 基础修补均已完成；随后 Windows 使用 `/IS /IT` 强制同步并完成当前 working tree 的构建/测试复验。真实 GUI 渲染和辅助技术验收仍保持 `WINDOWS_VERIFICATION_PENDING`/`BLOCKED`，文件 SQLite 应用级专项保持 `NOT RUN`。Windows-only 项目不得提前标记为 `WINDOWS_PASS`。

### Current Linux working tree after Windows validation of `34b5c67`（2026-09-10）

在 `34b5c67` Windows 验证完成后，Linux 完成了不依赖 Windows 的 GUI 代码收口：Widget 级错误分离与重试、移除未实现的禁用主导航、Job `<ul>/<li>`/`<time>` 语义、Focus-visible/reduced-motion 和用户级错误区域标题。随后 Windows 使用 `/IS /IT` 强制同步并实际完成构建/测试复验；这些代码的当前状态为 `LINUX_VERIFIED`，真实 GUI 渲染/交互验收仍为 `WINDOWS_VERIFICATION_PENDING`/`BLOCKED`。

### Preliminary Windows validation for Linux working tree based on `34b5c67`（2026-09-10）

本轮实际验证对象为 Linux `main` 的 HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 加验证开始前已存在的 working-tree changes，而不是纯 Git commit。验证开始时 `git status --short` 显示以下未提交修改：`desktop/src/main.jsx`、`desktop/src/style.css`、`docs/development/non-windows-completion.md`、`docs/development/risk-register.md`、`docs/development/roadmap.md`、`docs/development/testing-strategy.md` 以及既有的本验证报告。当前 GUI working-tree changes 包括 Widget 级错误分离/重试、移除未实现禁用导航、Job `<ul>/<li>` 与 `<time>` 语义、Focus-visible 和 reduced-motion 样式；这些改动是本轮验证目标。

> 同步复核说明：本节最初执行的测试发生在 Robocopy 按时间/大小判断而遗漏同尺寸 working-tree 文件的旧副本上，不能作为最新 working-tree 的正式证据。随后已使用 `/IS /IT` 强制同步，并对最新副本完整重跑；正式结论见下方“after forced sync”小节。

### Validation Environment

- Windows 11 Insider Preview `10.0.29661.0` / 64 位。
- Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。
- Windows 工作副本：`E:\Shiraishi\VSCode Workspace\Tw2Tg`；未建立 Git checkout。
- 同步方向：WSL `/home/shiraishi/VSCode Workspace/Tw2Tg` → Windows E 盘；使用 Robocopy `/E /XJ /FFT /COPY:DAT /DCOPY:DAT`，未使用镜像删除。
- 排除 `.git`、`node_modules`、`.venv`、`target`、Desktop 构建目录、缓存、数据库、`.env` 和验证报告；E 盘本地依赖、缓存、构建产物与 `validation-artifacts` 均保留。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux working tree → Windows 同步与关键 GUI 文件一致性 | PASS | 当前 working-tree 的 `desktop/src/main.jsx`、`desktop/src/style.css`、`docs/development/roadmap.md` 已同步到 E 盘并参与测试；本地依赖/缓存/构建产物未被覆盖 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules 构建通过，Extension 6 项测试通过，Desktop Node 测试 0 项且无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后 `cargo test --workspace` 通过 69 项：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version` 为 2.11.4；`npm run build:tauri` 成功生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；启动期间进程可见，Ctrl+C 后 Tw2Tg 相关进程为 0 |
| 最新 GUI working-tree 修补的真实 WebView2/DPI、键盘、焦点、屏幕阅读器和对比度验收 | BLOCKED | 当前环境没有可用 GUI automation native app target；构建和启动 PASS 不能替代真实渲染/交互验收 |
| aria2 官方 artifact 与下载/恢复链路 | NOT APPLICABLE | 本轮改动未触及 aria2 核心逻辑；`d239a1d` 已完成官方 artifact、RPC、Unicode/空格路径、Range 续传、进程恢复和 `.aria2` 清理的实际 Windows 验证，证据继续有效 |
| 文件 SQLite 应用重启恢复、遗留 staging、`0001 → 0002` 迁移 | NOT RUN | 仍缺少应用级专项场景；Repository 单元测试不能替代 Desktop 现场恢复验证 |
| Named Pipe、Native Host/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | Windows backend、发布 artifact、浏览器实机、账号/凭据和自动化前置条件仍未具备 |

### Errors

- 首次直接执行 `cargo test --workspace` 时，`xarchive-sidecar-supervisor` 的 2 项真实 Python worker/hello 测试因未显式设置 `PYTHON` 报 `NotRunning`。这是 Windows 环境配置前置问题，不是业务代码 FAIL；设置 `PYTHON` 指向项目 `.venv\Scripts\python.exe` 后，目标测试 4/4 和完整 workspace 69/69 均通过。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于生成 `.lib/.exp` 的非阻塞 warning。
- 停止 Tauri Debug 时记录 Chromium `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` 和 `STATUS_CONTROL_C_EXIT`；最终 Tw2Tg 进程已清理，未影响启动结论。若要求无告警退出，应作为独立 Windows 生命周期事项调查。

### Linux Follow-up

本轮没有发现属于项目代码的 Windows `FAIL`。但由于本轮验证对象包含未提交 working-tree changes，Linux 后续应：

- 在可用 GUI automation native app target 后，对最新 GUI working-tree changes 执行 W-P1-11 的真实 WebView2/DPI、Tab/键盘、Focus-visible、aria-live/错误恢复、aria2 Windows gating、屏幕阅读器和对比度验证；
- 完成文件 SQLite Desktop 应用重启、遗留 staging 恢复和 `0001 → 0002` 迁移专项；
- 继续推进 Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram 等前置条件；
- 可选地单独调查 Tauri Ctrl+C 后 Chromium `Error = 1412` 清理警告；
- 在提交或继续修改 GUI 前，重新执行本轮代码变更对应的 Windows 验证，不能将本轮结果外推到后续未同步的 working-tree changes。

### Windows re-validation after forced sync of latest Linux working tree（2026-09-10）

本节是本轮正式结论。验证对象仍为 Linux `main` HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 加验证开始前已有的未提交 GUI/文档 working-tree changes；通过 Robocopy `/IS /IT` 强制同步后，最新文件才进入 Windows 工作副本并重新执行全部适用验证。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| 最新 Linux working tree → Windows 同步 | PASS | 使用 `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT` 单向覆盖受控源文件；排除 `.git`、依赖、缓存、构建产物、数据库、`.env` 和验证报告；关键 GUI 文件哈希复核一致 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules、Extension 6 项测试通过，Desktop Node 测试 0 项且无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后 `cargo test --workspace`；69 项全部通过：11/5/10/4/7/4/16/12 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri Release 构建 | PASS | `npm run build:tauri`；35 modules 构建并成功生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；进程检查可见，停止后 Tw2Tg 相关进程为 0 |
| 最新 GUI working-tree 的真实 WebView2/DPI、键盘、焦点、屏幕阅读器、对比度验收 | BLOCKED | 当前没有可用 GUI automation native app target；构建/启动结果不替代真实渲染和交互验收 |
| aria2 官方 artifact 与下载/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑；沿用 `d239a1d` 已完成的官方 artifact、RPC、Range/恢复和 `.aria2` 清理 Windows 证据 |
| 文件 SQLite Desktop 应用重启、遗留 staging、`0001 → 0002` 迁移 | NOT RUN | 仍没有应用级专项测试场景 |
| Named Pipe、Native Host/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 相应 backend、发布 artifact、浏览器实机、自动化 target、账号或凭据前置条件缺失 |

### Re-validation Errors and Follow-up

- 首次 Robocopy 同步未使用 `/IS /IT`，虽然返回允许的退出码 3，但同尺寸/近似时间戳文件未被覆盖；后续哈希检查发现 `desktop/src/main.jsx`、`desktop/src/style.css` 及文档存在差异。该同步过程错误已通过 `/IS /IT` 修正，旧副本测试结果作废，强制同步后的结果才是本轮有效证据。
- 强制同步后的 Rust 测试从一开始显式设置 `PYTHON`，未再出现 `NotRunning`；完整 workspace 69/69 通过。MSVC linker stdout 的 `#[warn(linker_messages)]` 仍为非阻塞 warning。
- Ctrl+C 停止 Tauri Debug 时出现 Chromium `Failed to unregister class Chrome_WidgetWin_0. Error = 1411` 与 `STATUS_CONTROL_C_EXIT`；进程最终清理干净。该 Windows 生命周期警告可另立事项调查，本轮不修改业务代码。

Linux 后续处理：本轮未发现项目代码导致的 Windows `FAIL`。需在具备 GUI automation native app target 后重新执行最新 working-tree 的 W-P1-11 真实渲染/交互验收；继续设计文件 SQLite 应用级恢复/迁移专项，以及 Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie 和真实账号链路。后续若 GUI working tree 再变化，必须重新执行受控同步和对应 Windows 验证。

### Historical Linux follow-up before latest Windows working-tree re-validation（2026-09-10）

依据上一节强制同步后的 Windows 正式结果，Node/Rust/Sidecar/Tauri 基础验证已经覆盖当时的 GUI working tree；真实 GUI 验收仍为 `BLOCKED`，文件 SQLite 应用级恢复/迁移仍为 `NOT RUN`。Linux 随后只继续了一个不依赖 Windows 的小范围改动：为 Widget 错误增加用户级区域标题，例如“任务列表加载失败”“归档文件夹打开失败”，并保留原始技术详情和局部重试。

该段记录的是本轮强制同步前的状态快照；随后已通过 `/IS /IT` 将该错误文案改动同步到 Windows 并完成正式复验，正式结果见下方最新条目。除实际同步并验证的 working tree 外，不外推此前 Windows 结果。

### Final Windows re-validation of latest Linux working tree（2026-09-10）

本轮最终验证对象为 Linux `main` HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 加验证开始前已有的未提交 GUI/文档 working-tree changes，包括 Widget 级错误分离、用户级区域标题与重试、未实现导航项移除、Job/时间语义、Focus-visible 和 reduced-motion。使用 `/IS /IT` 强制同步后，以下结果覆盖了该完整 working tree。

| 验证项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；关键源文件哈希一致，未覆盖依赖、缓存、构建产物、数据库、`.env` 或独立 artifact |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6，Desktop Node 测试无失败 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、`cargo test --workspace`；69/69 通过，Rust 测试显式使用项目 `.venv\Scripts\python.exe` |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 Release executable；`npm run dev:tauri` 启动 Vite/Rust/Desktop，停止后 Tw2Tg 进程为 0 |
| GUI 真实 WebView2/DPI/键盘/焦点/屏幕阅读器/对比度 | BLOCKED | 无可用 GUI automation native app target；构建和启动不能替代真实渲染验收 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 已完成的 Windows 实测证据 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | Windows backend、发布 artifact、浏览器实机、账号/凭据或自动化前置条件缺失 |

正式结论：本轮没有项目代码导致的 Windows `FAIL`。唯一的同步过程问题是初次 Robocopy 未强制覆盖同尺寸文件，已由 `/IS /IT` 修正并重新执行全部适用测试；Tauri Ctrl+C 仍记录 Chromium `Error = 1411`/`STATUS_CONTROL_C_EXIT`，但进程清理正常。Linux 已有 storage 层文件数据库重开与 migration 幂等测试，但这不替代 Windows Desktop 应用现场重启、路径行为、遗留 staging 和迁移升级验证。Linux 后续继续处理 GUI 真实渲染验收、文件 SQLite 应用级恢复/迁移专项及其余 Windows backend/账号前置条件；本轮不修改业务代码。

### Windows validation repeat for current Linux working tree（2026-09-10）

本轮按用户要求再次验证当前 Linux 最新状态。HEAD 仍为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`，验证开始时 working tree 仍包含既有 GUI/文档未提交修改；未发现新的业务代码提交。使用 Robocopy `/IS /IT` 将当前 WSL working tree 单向同步到 `E:\Shiraishi\VSCode Workspace\Tw2Tg`，关键文件哈希一致。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；排除 `.git`、依赖、缓存、构建产物、数据库、`.env` 和验证报告；本地额外目录保留 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules、Extension 6/6 通过 |
| Rust fmt/check/test/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、严格 clippy；69/69 通过，显式设置项目 `.venv\Scripts\python.exe` |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 成功生成 Release executable；`npm run dev:tauri` 启动 Vite/Rust/Desktop，停止后相关进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 当前 working tree 未修改 aria2 核心逻辑，沿用 `d239a1d` 的实际 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/辅助技术/对比度 | BLOCKED | 无 GUI automation native app target；构建和启动不能替代真实渲染验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | backend、发布 artifact、浏览器实机、账号/凭据和自动化前置条件缺失 |

本轮无项目代码导致的 Windows `FAIL`。构建仍有非阻塞 MSVC linker stdout warning；主动 Ctrl+C 停止 Tauri 时出现 Chromium `Error = 1411`/`STATUS_CONTROL_C_EXIT`，但相关进程已清理。Linux 后续继续处理 GUI 真实验收、文件 SQLite 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Latest Linux reconciliation after Windows repeat（2026-09-10）

最新 Windows repeat 已覆盖当前 Linux working tree，包含全部 GUI/文档未提交修改；Node、Rust、Sidecar、Tauri 构建和启动清理均为 `PASS`，本轮没有项目代码导致的 Windows `FAIL`。因此当前 Plan 不再继续 GUI 源码修补，也不重复已有 storage 层文件数据库重开测试。

当前仍需保持的状态：

- GUI 真实 WebView2/DPI、键盘/焦点、屏幕阅读器和对比度：`WINDOWS_VERIFICATION_PENDING` / `BLOCKED`；
- 文件 SQLite Desktop 应用重启、遗留 staging、迁移升级：`NOT RUN`；
- Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram：`BLOCKED / NOT RUN`。

Linux 本轮适用验证已通过：workspace Node check/test/build、Rust fmt/check/test、Python `compileall` 和 7 个 JSON/schema 文件解析。由于 Linux 环境缺少 `cargo-clippy` 与 `pytest`，不将其本地结果标记为 PASS；Windows 对应结果继续以最新验证记录为准。

### Windows validation repeat after current-state resynchronization（2026-09-10）

本轮针对当前 Linux 最新 working tree 重新执行 Windows 验证。Linux source 为 branch `main`、HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，包含此前已有的 GUI/文档未提交修改，本轮未新增业务代码修改。Windows 工作副本为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`。环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。使用 Robocopy `/IS /IT` 完成 Linux → Windows 单向同步；6 个关键文件哈希一致，Windows 本地依赖、缓存、构建产物和独立测试辅助文件按规则保留。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | `robocopy /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；排除 `.git`、`node_modules`、`.venv`、Rust target、构建缓存、数据库、`.env` 和验证报告；Robocopy exit 3 表示复制文件并保留目标额外目录，非错误 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules 构建通过，Extension 6/6 通过 |
| Rust fmt/check/test/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`；workspace 测试 69/69 通过 |
| Python sidecar | PASS | `.venv\\Scripts\\pytest.exe sidecar\\tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 成功生成 `target\\release\\xarchive-desktop.exe`；`npm run dev:tauri` 启动成功，停止后相关进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮 working tree 未修改 aria2 核心逻辑；沿用 `d239a1d` 的实际 Windows PASS 证据。独立保存的 aria2 测试辅助文件仍位于 `E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2` |
| GUI 真实 WebView2/DPI/键盘/辅助技术/对比度 | BLOCKED | 当前没有可用的 GUI automation native-app target；构建和启动不能替代真实渲染、输入法、屏幕阅读器和对比度验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前环境未执行 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 分别缺少对应 Windows backend、发布/安装验证、浏览器实机、账号或凭据及自动化前置条件 |

本轮未出现项目代码导致的 Windows `FAIL`。构建输出中的 MSVC linker warning 为非阻塞警告；主动 Ctrl+C 停止 Tauri 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但相关进程已清理。需要 Linux 后续处理的事项仍为 GUI 真实验收、文件 SQLite 应用级恢复/迁移，以及其余 Windows backend、发布安装和真实账号/凭据场景。本轮不扩大为开发任务。

### Latest Linux reconciliation after Windows results（2026-09-10）

Linux 重新读取了当前 `git diff`、最新 Windows 验证记录、现有 Plan、`AGENTS.md` 和跨平台验证规范。Windows 最新事实为：针对 `main` / `34b5c67` 加 working-tree changes 的 Node、Rust fmt/check/clippy/test、Python Sidecar、Tauri Debug/Release 构建和进程清理均通过；aria2 基础 artifact/RPC/恢复链路沿用既有 Windows PASS。GUI automation native-app target 仍不可用，因此真实 WebView2/DPI、Tab 顺序、键盘、Focus-visible 实际表现、命中区域、Narrator/NVDA 和最终对比度没有完成实际验收。文件 SQLite Desktop 应用级重启、遗留 staging 和 `0001 → 0002` 迁移也没有执行。

本轮 Linux 端没有发现需要继续修改的业务代码。已有 GUI 源码修补和白色 Dashboard 重设计已收口，不机械重复 GUI 重构；`roadmap.md` 的 M6 完成标准已改为明确区分“Windows 构建/启动已验证”和“真实 GUI 验收仍待验证”。

| 当前事项 | 状态 | 当前结论 |
|---|---|---|
| GUI Widget 错误分离/重试、用户级错误标题、语义 Job 列表/时间、Focus-visible、reduced-motion、aria2 Windows gating、白色 Dashboard | `LINUX_VERIFIED`；Windows 构建/启动证据有效 | Linux 回归通过；不把构建/启动替代真实 GUI 验收 |
| Windows WebView2/DPI、Tab/键盘、Focus-visible、Narrator/NVDA、命中区域、最终对比度 | `WINDOWS_VERIFICATION_PENDING` / `BLOCKED` | 缺少 GUI automation native-app target；具备 target 后执行 W-P1-11 |
| 文件 SQLite Desktop 应用重启、遗留 staging、`0001 → 0002` 迁移 | `NOT RUN` | 需要受控 Desktop 应用级专项场景；Repository 单元测试不能替代现场恢复验证 |
| Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | `WINDOWS_VERIFICATION_PENDING` / `WINDOWS_BLOCKED` / `NOT RUN` | 依赖尚未实现的 Windows backend、发布 artifact、浏览器实机、账号或凭据 |

### Latest Linux validation record（2026-09-10）

| 验证项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Node workspace | PASS | `npm run check && npm run test && npm run build`；Desktop Vite 构建通过，Extension 6/6，通过 |
| Desktop Tauri Linux 构建 | PASS | `npm run build:tauri`；生成 `/home/shiraishi/VSCode Workspace/Tw2Tg/target/release/xarchive-desktop` |
| Rust workspace | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`；69/69 通过 |
| Python Sidecar 静态验证 | PASS | `python3 -m compileall -q sidecar/src sidecar/tests`；通过 |
| Rust clippy | NOT RUN | 当前 Linux 未安装 `cargo-clippy`；沿用最新 Windows 严格 clippy PASS，不将其改记为 Linux PASS |
| Python pytest | NOT RUN | 当前 Linux 未安装 pytest；沿用最新 Windows Sidecar 10/10 PASS，不将其改记为 Linux PASS |

本轮 Linux 验证未发现失败。下一轮 Windows 仍不得提前标记上述真实 GUI 项目为 `WINDOWS_PASS`；只有在 Windows 实际执行并通过对应验收后才能关闭 `WINDOWS_VERIFICATION_PENDING`。

### Linux business backend development follow-up（2026-09-10）

Linux 继续实现当前文档明确缺失的业务后端，新增 `xarchive-download::DownloadRouter`。该 Router 是纯 Rust、无新增依赖的策略编排层：默认执行 gallery-dl；仅当 gallery-dl 返回稳定错误码 `EXTRACT_OR_DOWNLOAD_FAILED`、配置允许 aria2 且调用方提供新鲜 `AddUriRequest` 时才进入 aria2；`AUTH_REQUIRED`、`RATE_LIMITED` 和 `TWEET_NOT_FOUND` 不自动回退。Router 同时保留 gallery-dl 与 aria2 的双失败原因。

| 项目 | 状态 | 当前事实 |
|---|---|---|
| DownloadRouter 策略与错误模型 | `IMPLEMENTED` / `LINUX_VERIFIED` | `cargo test -p xarchive-download` 16/16 通过；覆盖默认 gallery-dl、下载失败 fallback、认证错误不回退、禁用 fallback、aria2 未配置和双失败 |
| Router 与真实 Sidecar/Job 调度接入 | `NOT RUN` | 当前仍需在 Desktop/归档 Job 执行路径中接入；本轮没有把闭包级策略测试误记为端到端完成 |
| 403 后 gallery-dl 重新提取、真实 aria2 transfer 生命周期 | `WINDOWS_VERIFICATION_PENDING` | 需要真实 media URL、Sidecar/aria2 进程和 Windows externalBin/aria2c.exe 场景；Linux Router 单元测试不能替代 Windows 实机验证 |

本轮没有修改 Windows 工作副本代码，也没有将该 Router 标记为 `WINDOWS_PASS`。下一步 Linux 开发应把 Router 接入实际 Job/Sidecar 调度，并补充 fake Sidecar + fake aria2 的端到端测试；接入完成后再执行 Linux 全量回归，并将新增 Windows 平台集成项目继续标记为 `WINDOWS_VERIFICATION_PENDING`。

### Linux Native Host forwarding development follow-up（2026-09-10）

Linux 继续实现当前文档明确缺失的 Native Host 转发后端。`xarchive-native-host` 新增 `forward_request`、`error_response` 和 `request_id` 公共编排函数；主程序读取 `XARCHIVE_PIPE_ENDPOINT`，在未配置时返回 `NATIVE_PIPE_UNAVAILABLE`，配置后尝试以读写方式连接 Desktop endpoint，并将连接/协议错误映射为 `NATIVE_PIPE_ERROR`。Linux fake duplex transport 已验证合法请求转发、响应保留以及非法请求在写入 transport 前被拒绝。

| 项目 | 状态 | 当前事实 |
|---|---|---|
| Native Host framing、BrowserRequest 校验和可插拔请求/响应转发 | `IMPLEMENTED` / `LINUX_VERIFIED` | Native Host crate 8/8 通过；本轮新增后 Rust workspace 最新总数为 79/79；包含 fake transport 成功转发、非法请求边界、request_id 不匹配和协议版本不匹配测试 |
| Windows Named Pipe server/client、endpoint 连接、ACL、生命周期 | `WINDOWS_VERIFICATION_PENDING` / `WINDOWS_BLOCKED` | 当前只实现 endpoint client boundary；Windows Named Pipe 服务端、ACL、多连接、重连和退出行为尚未在 Windows 实机验证 |
| Native Host manifest、Registry、Edge/Chrome 实机加载 | `WINDOWS_VERIFICATION_PENDING` / `NOT RUN` | 仍缺少 manifest/Registry 安装与浏览器实机前置条件 |

本轮 Linux 端没有实现 Windows 专用 server、ACL 或 Registry 代码，也没有把 fake transport 结果标记为 Windows PASS。下一步应在 Linux 补充 Native Host 与 fake Desktop endpoint 的多请求/错误响应测试；在 Windows 前置条件具备后，执行 W-P1-02/W-P1-05 的 Named Pipe、ACL、重连、Registry 和浏览器集成验证。

### Windows validation repeat after current Linux state（2026-09-10）

本轮重新读取并验证当前 Linux 状态。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有未提交修改仍包含 GUI 和文档文件，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT` 完成 Linux → Windows 单向同步，6 个关键文件 SHA-256 哈希一致；独立 aria2 测试辅助文件仍保存在 `E:/Shiraishi/VSCode Workspace/Tw2Tg-Windows-DevArtifacts/aria2`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | 首次受限执行环境访问 WSL/E: 被拒绝，使用授权执行后同步成功；Robocopy exit 3 为复制文件并保留目标额外目录的允许结果 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6，无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`；全部通过 |
| Rust workspace tests | PASS（设置环境后） | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release 构建 | PASS | `npm run build:tauri`；成功生成 `target/release/xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust 和 Desktop；主动停止后没有 Tw2Tg 相关残留进程 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 仅发现浏览器目标，没有可用的 GUI automation native-app target；构建和启动不能替代真实 GUI 验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前未执行 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮错误与分析：

- 首次在受限执行环境中运行 Robocopy 和 Node 命令时出现 `Access denied` / 找不到 `C:/package.json`；这是执行环境访问边界，不是项目错误。授权后从 E 盘工作副本重跑，结果为 PASS。
- 首次未设置 `PYTHON` 运行 `cargo test --workspace` 时，`xarchive-sidecar-supervisor` 的 `spawn_ready_completes_the_hello_handshake` 和 `communicates_with_a_real_python_worker_when_available` 失败并报 `NotRunning`。最可能原因是 sidecar Python 路径前置条件缺失，非 Windows 业务代码兼容性问题；设置项目 `.venv` Python 后 69/69 通过。
- Rust/Release 构建有非阻塞 MSVC linker stdout warning。Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，随后进程清理正常；本轮不将其判定为项目 FAIL。

本轮没有项目代码导致的 Windows FAIL。Linux 后续事项仍为 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移，以及 Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie 和真实账号链路。若后续 GUI working tree 或业务代码继续变化，需重新同步并执行对应 Windows 验证；本轮不扩大为开发任务。

### Windows validation repeat after latest Linux-state sync（2026-09-10）

本轮再次针对当前 Linux 最新 working tree 执行验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。通过 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT` 完成 Linux → Windows 单向同步，6 个关键文件 SHA-256 哈希一致。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy exit 3；复制文件并保留 E 盘本地额外目录，未同步 `.git`、依赖、缓存、target、数据库、`.env` 或验证报告 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6，均无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`；全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release 构建 | PASS | `npm run build:tauri`；成功生成 `target/release/xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 成功启动 Vite、Rust 和 Desktop；停止后无 Tw2Tg 项目进程残留 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据；独立 aria2 辅助文件仍保存在 E:/Shiraishi/VSCode Workspace/Tw2Tg-Windows-DevArtifacts/aria2 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 返回无可用原生应用目标，并报告浏览器策略加载错误；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前未执行 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮没有项目代码导致的 Windows `FAIL`。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但进程清理正常。Linux 后续仍需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation of latest Linux working tree with protocol/backend changes（2026-09-10）

本轮针对当前 Linux 最新 working tree 执行集中 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，包含既有 GUI、Rust crate、Tauri、协议 schema、文档修改和未跟踪的 `OPENAI_CODEX_WRITING_RULES.md`。本轮不修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，exit 3；新增 Rust/Tauri/协议/文档文件和未跟踪规则文件均已同步，关键文件哈希一致，未覆盖依赖、缓存、target、数据库、`.env` 或验证报告 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`；通过 |
| Rust clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后 `cargo test --workspace`；79/79 通过，包含新增 download fallback、Native Host 和协议回归测试 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release 构建 | PASS | `npm run build:tauri`；成功生成 `target/release/xarchive-desktop.exe`；构建不执行 clippy，因此不抵销 clippy FAIL |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Cargo 和 `xarchive-desktop.exe`；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 返回 `apps=[]`，没有可用原生 GUI automation target；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

### Errors and Linux follow-up

- `cargo clippy --workspace --all-targets -- -D warnings` 的 FAIL 属于项目代码质量门禁，不是 Windows-only 环境问题；建议 Linux 后续重构 `run_sidecar_download` 参数对象/上下文，或根据项目规范处理该 lint。由于本轮是验证任务，不在此处修改代码。该 FAIL 不阻塞独立的 Node、Python、workspace test、Tauri build/start 验证，但阻塞“严格 clippy 全通过”的结论。
- Rust/Release 构建的 MSVC linker stdout warning 为非阻塞 warning。Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但项目进程清理正常。
- GUI 原生验证仍受 Computer Use 无原生 app target 阻塞；文件 SQLite 应用级恢复/迁移、Windows backend、发布安装和真实账号链路仍未执行。

本轮最终结论：适用自动化验证中，Node、Rust fmt/check、workspace tests、Python sidecar、Tauri build/start 均 PASS；严格 Rust clippy 为 FAIL。Linux 后续必须处理 `desktop/src-tauri/src/lib.rs:299` 的 too-many-arguments 问题，然后重新执行 Linux lint 和 Windows clippy；同时继续安排 GUI 原生实机、SQLite 应用级恢复/迁移和其余 Windows backend/账号验证。本轮不扩大为开发任务。

### Windows validation repeat after latest Linux-state sync（第二次重跑，2026-09-10）

本轮再次读取当前 Linux source、Plan 和验证文档，并针对同一最新 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果，6 个关键文件 SHA-256 哈希一致，Windows 本地目录保留 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、设置 `PYTHON` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 `target/release/xarchive-desktop.exe`；`npm run dev:tauri` 启动 Vite/Rust/Desktop，停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据；独立辅助文件仍保存在 E:/Shiraishi/VSCode Workspace/Tw2Tg-Windows-DevArtifacts/aria2 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 返回无可用原生应用目标，无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮未发现项目代码导致的 Windows `FAIL`。Rust/Release 构建仍有非阻塞 MSVC linker stdout warning；Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但进程清理正常。Linux 后续需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation repeat after latest Linux working-tree update（第四次重跑，2026-09-10）

本轮重新读取当前 Linux source、Plan 和验证文档，并验证最新 working tree。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，exit 3；6 个关键文件 SHA-256 哈希一致，保留 E 盘本地目录和独立 aria2 辅助文件 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、设置 `PYTHON` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 `target/release/xarchive-desktop.exe`；`npm run dev:tauri` 启动成功，停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 无可用原生应用目标；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮未发现项目代码导致的 Windows `FAIL`。MSVC linker stdout warning 仍为非阻塞警告；主动 Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但项目进程清理正常。Linux 后续仍需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation repeat after latest Linux working-tree update（第五次重跑，2026-09-10）

本轮重新读取当前 Linux source、Plan 和验证文档，并针对最新 working tree 完成 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，exit 3；6 个关键文件 SHA-256 哈希一致，保留 E 盘本地目录和 aria2 辅助文件 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、设置 `PYTHON` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 `target/release/xarchive-desktop.exe`；`npm run dev:tauri` 启动成功，停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前返回 `apps=[]`，没有可用原生 GUI automation target；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮未发现项目代码导致的 Windows `FAIL`。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但项目进程清理正常。Linux 后续仍需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation after latest Linux working-tree update（2026-09-10）

本轮再次以当前 Linux source 为唯一来源读取仓库状态、Plan 和验证文档，并将最新 working tree 单向同步到 Windows 验证副本。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；working tree 非 clean，包含既有 Rust、Tauri、前端、协议、文档和未跟踪 `OPENAI_CODEX_WRITING_RULES.md` 修改。本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3（仅表示存在复制/跳过项，允许）；本轮关键源文件 SHA-256 哈希一致，保留 E 盘本地依赖、缓存、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；完成且 exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，产物构建完成 |
| Rust formatter | FAIL | `cargo fmt --all -- --check`；`crates/xarchive-core/src/job.rs` 与 `crates/xarchive-storage/src/lib.rs` 存在 rustfmt 差异 |
| Rust check | PASS | `cargo check --workspace`；完成且 exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；开发应用成功启动；Ctrl+C 后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前返回 `apps=[]`，没有可用原生 GUI automation target；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. `cargo fmt --check` 的 FAIL 是当前 working tree 的格式未整理问题，涉及 `xarchive-core/src/job.rs` 和 `xarchive-storage/src/lib.rs`；不是 Windows 专属问题。Linux 后续需先按项目约定重新运行 formatter 并确认 diff。
2. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。Linux 后续需处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多（建议重构参数上下文或采用项目批准的 lint 处理），然后重新执行 Linux lint 和 Windows clippy；本轮不直接修复。
3. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但这是主动 Ctrl+C 停止产生的现象，项目进程已清理。
4. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需具备相应 Windows 实机和凭据后再执行。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Windows validation after latest Linux working-tree sync（2026-09-10 22:54）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对当前 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；working tree 非 clean，包含既有 Rust、Tauri、前端、协议、文档和未跟踪 `OPENAI_CODEX_WRITING_RULES.md` 修改。本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；本轮同步后关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。当前 Plan 曾记录 Windows clippy 已通过，但本轮针对最新 working tree 的实际结果为 FAIL；Linux 后续必须重新 reconcile Plan/代码状态，处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多问题，然后重新执行 Linux lint 和 Windows clippy。本轮不直接修复。
2. Tauri Release/Debug 构建和启动均通过。构建期间的 MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需相应 Windows 实机和凭据。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Windows validation after Linux revision update（2026-09-11 10:03）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对最新 Linux revision 执行 Windows 验证。source branch 为 `main`，HEAD 为 `9334d1472843babae8910c02cb93fd7039e82387`；验证开始时 working tree 仅有未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；完成且无错误 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit 0 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 本轮适用的 Node、Rust、Python 和 Tauri 自动化验证均无 FAIL；之前的 `run_sidecar_download` clippy 问题已随当前 Linux revision 处理，Plan 中对应的 Windows clippy PASS 已得到实际重新验证。
2. Tauri 构建期间出现 MSVC linker stdout warning，为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理，未观察到持续性 Windows 运行时故障。
3. Linux 后续无需因本轮自动化 FAIL 修复业务代码；仍需在具备 Windows 原生 GUI 自动化目标、Desktop 应用级 SQLite 场景、发布/安装实机、浏览器 Cookie 环境及 X/Telegram 账号凭据后，补做上述 BLOCKED / NOT RUN 项目。本轮不扩大为开发任务。

### Windows validation after Linux revision update（2026-09-11 09:33）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对最新 Linux revision 执行 Windows 验证。source branch 为 `main`，HEAD 为 `9334d1472843babae8910c02cb93fd7039e82387`；working tree 仅有未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。Plan 仍记录 Windows clippy 已通过，但本轮针对最新 revision 的实际结果仍为 FAIL；Linux 后续需 reconcile Plan/代码状态，处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多问题，再重新执行 Linux lint 和 Windows clippy。本轮不直接修复。
2. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需相应 Windows 实机和凭据。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Windows validation after latest Linux working-tree sync（2026-09-10 23:06）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对当前 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；working tree 非 clean，包含既有 Rust、Tauri、前端、协议、文档和未跟踪 `OPENAI_CODEX_WRITING_RULES.md` 修改。本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。当前 Plan 仍记录 Windows clippy 已通过，但本轮针对最新 working tree 的实际结果为 FAIL；Linux 后续必须 reconcile Plan/代码状态，处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多问题，然后重新执行 Linux lint 和 Windows clippy。本轮不直接修复。
2. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需相应 Windows 实机和凭据。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Linux reconciliation after Windows clippy failure（2026-09-11）

Linux 已按 `docs/development/cross-platform-validation.md` 重新读取本轮 Windows 结果，并处理其中唯一明确属于项目代码的失败：`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 因 8 个参数触发 `clippy::too_many_arguments`。该问题不是 Windows-specific 行为，也不是 `WINDOWS_VERIFICATION_BLOCKING`；它已在 Linux 侧通过 `SidecarDownloadRequest` 参数上下文结构完成最小重构，保持 Sidecar 命令字段、事件过滤和归档行为不变。

本轮 Linux 验证结果：

| 验证项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Rust formatter | PASS | `cargo fmt --all -- --check` |
| Rust workspace check | PASS | `cargo check --workspace` |
| Rust workspace tests | PASS | `cargo test --workspace`；79 项 crate 测试及 doc-tests 通过 |
| Node check/build | PASS | `npm run check`、`npm run build` |
| Extension tests | PASS | `npm test` |
| Linux clippy | NOT RUN | 本机 stable toolchain 未安装 `cargo-clippy` |
| Git diff check | PASS | `git diff --check` |

该修复尚未在 Windows 实际重新执行严格 clippy，因此相关状态更新为：

| Windows 项目 | 状态 | 说明 |
|---|---|---|
| Windows strict clippy after `SidecarDownloadRequest` refactor | `WINDOWS_VERIFICATION_PENDING` | 必须在 Windows 工作副本同步最新 Linux 状态后执行 `cargo clippy --workspace --all-targets -- -D warnings`；Linux 通过不能替代 Windows 复验 |
| Windows Node/Rust build/test、Python Sidecar、Tauri build/start | `WINDOWS_VERIFICATION_PENDING` | 当前 Linux working tree 含业务代码变更，上一轮对旧状态的 PASS 不能自动覆盖本轮新 revision |
| GUI、文件 SQLite 应用级恢复/迁移、Named Pipe/Registry、externalBin/安装器、Credential Manager、Edge Cookie、真实 X/Telegram | `WINDOWS_VERIFICATION_PENDING` / `BLOCKED` / `NOT RUN` | 继续受各自 automation、backend、artifact、浏览器实机或账号前置条件约束 |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。Windows 严格 clippy 复验是下一次集中 Windows validation 的必做项，但不是继续 Linux 设计或实现的硬性前置；其余可在 Linux 完成的工作仍可按当前 Plan 继续。历史 Windows clippy `FAIL` 记录保留，不被本次 Linux 修复改写为 Windows `PASS`。

### Linux reliability batch after latest Windows results（2026-09-11）

根据上一轮 Windows 验证暴露的项目代码问题和当前 Plan，Linux 侧继续完成了一个不依赖 Windows 结果的可靠性批次：

- `run_sidecar_download` 已使用 `SidecarDownloadRequest` 上下文结构收敛参数，修复 Windows 严格 clippy 报告的 `too_many_arguments`；
- `archive_tweet` 不再忽略 `DownloadRouter` 返回值，也不再通过 `expect` 处理下载失败；
- gallery-dl/aria2 路由失败会映射到 `AUTH_REQUIRED` 或 `FAILED`，并写入 `last_error_code`/`last_error_message`；
- 归档流程新增 `JOB_CREATED`、`DOWNLOAD_STARTED`、`DOWNLOAD_FAILED` 和 `DOWNLOAD_COMPLETED` 事件持久化；
- Router 仍使用默认 gallery-dl 路径，真实 aria2 `AddUriRequest`、403 后重新提取 URL 和 transfer 生命周期尚未实现，不将本批次视为 Windows 端到端通过。

本轮 Linux 验证：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`（79 项 crate 测试及 doc-tests）、`npm test`（Extension 6/6）、`npm run check`、`npm run build` 和 `git diff --check` 均通过；本机未安装 `cargo-clippy`，Linux clippy 为 `NOT RUN`。

受本批次业务代码变更影响的 Windows 项目必须在下一次同步后重新执行，并保持 `WINDOWS_VERIFICATION_PENDING`：

| 项目 | 状态 | 下一步 |
|---|---|---|
| Windows strict clippy after `SidecarDownloadRequest` refactor | `WINDOWS_VERIFICATION_PENDING` | 执行 `cargo clippy --workspace --all-targets -- -D warnings`；不得使用 Linux 结果替代 |
| Desktop Router/Sidecar/Job failure and event persistence | `WINDOWS_VERIFICATION_PENDING` | 使用 Windows Desktop/Sidecar 场景确认无 panic、失败状态、事件历史和进程清理 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | `WINDOWS_VERIFICATION_PENDING` | 准备受控 `aria2c.exe`、media server 和可重复的过期 URL 场景 |

本轮仍无 `WINDOWS_VERIFICATION_BLOCKING`。GUI、文件 SQLite 应用级恢复/迁移、Named Pipe/Registry、externalBin/安装器、Credential Manager、Edge Cookie 和真实 X/Telegram 等项目继续按既有前置条件保持 `WINDOWS_VERIFICATION_PENDING`、`BLOCKED` 或 `NOT RUN`，不得提前标记为 `WINDOWS_PASS`。

### Windows validation of Linux reliability batch（2026-09-11 10:09）

本轮针对 Linux 最新 reliability batch 重新执行 Windows 验证。source branch 为 `main`，HEAD 为 `9334d1472843babae8910c02cb93fd7039e82387`；验证时 working tree 包含 `SidecarDownloadRequest`、下载失败状态/事件持久化等未提交代码和文档修改，以及未跟踪 `OPENAI_CODEX_WRITING_RULES.md`。这些 working-tree changes 在本轮验证操作之外产生，本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`、Python `3.14.7`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希不匹配数为 0，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit 0 |
| Rust workspace tests | PASS（含一次间歇性 FAIL） | 首次完整运行有 2 个 sidecar supervisor 握手测试超时；定向重跑 `cargo test -p xarchive-sidecar-supervisor --lib -- --nocapture` 为 4/4，随后再次完整 `cargo test --workspace` 为 79/79，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Python hello 直连复现 | PASS | 使用 Windows venv Python 直接发送 hello JSONL，正确返回 `ready`，exit 0 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| Desktop Router/Sidecar/Job failure and event persistence | NOT RUN | 当前仅覆盖 crate/unit 测试和 Desktop 启动，缺少应用级失败状态、事件历史和无 panic 场景 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改既有 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据仅覆盖既有 aria2 核心 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、媒体服务和可重复的过期 URL 场景；且当前 batch 尚未实现完整真实链路 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 首次 `cargo test --workspace` 的 `xarchive-sidecar-supervisor` 中 `communicates_with_a_real_python_worker_when_available` 和 `spawn_ready_completes_the_hello_handshake` 失败，分别表现为未收到 `Ready` 和 `HandshakeTimeout`。Windows venv Python 直连 hello 正常，定向重跑及随后完整 workspace 重跑均通过，因此当前判断为未稳定复现的进程启动/握手时序或环境瞬态问题，不能据此认定稳定的业务代码 FAIL；仍保留该失败证据，后续若复现应重点检查 Windows child-process 启动和握手等待路径。
2. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. Linux 后续无需因本轮最终自动化结果修改业务代码；仍需准备受控 `aria2c.exe`、媒体服务和过期 URL 场景，验证真实 aria2 fallback/403 refresh/transfer lifecycle，并在具备原生 GUI、Desktop 应用级失败/事件和 SQLite 场景、安装器、浏览器及账号凭据后完成相应 BLOCKED / NOT RUN 项目。本轮不扩大为开发任务。
