# Windows 开发与验证清单

> 适用平台：Windows 10/11，优先验证 Edge，并回归 Chrome。文档日期：2026-09-09。

Linux 可验证协议、Rust 核心、Python 逻辑和前端静态检查，但不能替代 Windows 专属集成验证。本文集中记录必须在 Windows 实机或 Windows CI 完成的任务。

## 状态定义

| 状态 | 含义 |
|---|---|
| 已完成 | 已有明确测试结果或代码验证结果 |
| 待实现 | 对应功能尚未开发 |
| 待验证 | 功能已有，但尚未在 Windows 目标环境验证 |
| 阻塞 | 依赖工具、凭据、证书或外部环境 |

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
