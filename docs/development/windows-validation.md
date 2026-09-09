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
| Rust workspace | 当前 Windows `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、严格 `cargo clippy --workspace --all-targets -- -D warnings`、Debug/Release 编译和 `npm run build:tauri` 均通过，共 62 个 crate 单元测试通过；Linux let-chain 修复已完成 Windows re-validation | 基础验证和 clippy 已完成 |
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
| Telegram 发送状态持久化与幂等补传 | LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING | Linux 已实现 `SendState`/`SendStateStore` 契约、`send_idempotently` 幂等补传编排和 SQLite migration `0002_telegram_send_state.sql`（`telegram_send_attempts` 表）的 `Database` 实现，7 项新测试通过（全量 69 个 crate 单元测试）；SQLite/路径行为、应用重启恢复和迁移升级仍需 Windows 实机验证，真实账号发送另受账号环境限制 |
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

## 本轮 Windows 平台复验（2026-09-09，当前 working tree）

### Validation Environment

| 项目 | 实际值 |
|---|---|
| Linux source branch/revision | `main` / `f70f91399a0866ca8ee35741481e805a71556dc7`，包含 working tree changes |
| Linux working tree | 验证前已存在 `Cargo.lock`、`crates/xarchive-download/src/lib.rs`、`desktop/src-tauri/Cargo.toml`、`desktop/src-tauri/src/lib.rs`、`desktop/src/main.jsx`、`desktop/src/style.css` 及多份 README/Plan/架构/开发文档修改；另有未跟踪 `AGENTS.md`、`docs/development/cross-platform-validation.md`、`docs/validation/`。本轮仅修改本验证文档，未修改业务代码 |
| Windows 工作副本 | `E:\Shiraishi\VSCode Workspace\Tw2Tg` |
| Windows 系统/架构 | Windows 11 专业工作站版 Insider Preview `10.0.29661` / 64 位 |
| 运行时/工具链 | Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、rustfmt/clippy `1.9.0`、Python `3.14.7`、Tauri CLI `2.11.4` |
| 验证日期 | 2026-09-09 |

同步方向为 Linux source → E: Windows validation workspace。同步排除了 `.git`、`node_modules`、`.venv`、`target`、`dist` 和缓存/数据库文件，也排除了 Linux 端验证结果文档；未删除 E 盘本地依赖和构建产物。同步后对其余项目内容执行 `rsync --checksum`，无差异。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Node 依赖 | PASS | `npm ci`；安装 70 个依赖，审计 0 个漏洞 |
| Node 检查/测试/构建 | PASS | `npm run check`、`npm run test`、`npm run build`；Desktop 0 项 Node 测试，Extension 6 项通过 |
| Rust 格式/编译 | PASS | `cargo fmt --all -- --check`、`cargo check --workspace` |
| Rust 全量测试 | PASS | 当前进程设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后执行 `cargo test --workspace`；62 个 crate 单元测试全部通过，Desktop aria2 allowlist/SHA-256 测试 5 项通过 |
| Rust clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:83` 报 `clippy::collapsible_if`，建议将嵌套 `if let` 合并 |
| Python Sidecar 测试 | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Sidecar Windows 路径/JSONL 进程链路 | PASS | 使用含空格、中文和 `Ω` 的临时路径；输出 `ready → started → failed` 及 `INVALID_JSON`，进程 exit 0 |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version`、`npm run build:tauri`；生成 `target\release\xarchive-desktop.exe` |
| Tauri 开发启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe` |
| Tauri GUI 视觉验收 | BLOCKED | GUI 自动化 helper 无可用 native app target；仅确认启动日志 |

### Errors

| 失败/警告 | 分类与原因 | 对后续验证的影响 |
|---|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` 失败 | 当前 working tree 新增的 `candidate_aria2_paths` 使用可合并的嵌套 `if let`；属于需 Linux 处理的代码质量问题，不是 Windows 工具链缺失 | 不阻塞独立测试、构建或启动；Linux 后续应修复/确认后重新执行 clippy 和 Windows 验证 |
| `npm ci` 提示 `esbuild@0.28.2` postinstall script 未被 `allowScripts` 批准 | 依赖安装安全策略警告；Node 构建测试通过 | 不阻塞 |
| Rust/Tauri 构建输出 MSVC linker stdout `#[warn(linker_messages)]` | 工具链非阻塞 warning | 不阻塞 |
| 停止 Tauri 开发进程时出现 Chromium `Error = 1411`、`STATUS_CONTROL_C_EXIT` | 主动 Ctrl+C 停止时的窗口类注销/终止警告；启动阶段正常 | 不影响启动结论；GUI 视觉仍未确认 |

### Not Executed / Blocked / Not Applicable

| 验证项目 | 状态 | 原因 |
|---|---|---|
| Tauri GUI 前后端交互、视觉和资源路径人工验收 | BLOCKED | GUI 自动化 helper 无法提供可用 native app target |
| Edge Cookie、真实 X 认证和真实媒体归档 | BLOCKED | 缺少明确账号/Profile；公开 URL 失败不能替代认证验收 |
| Named Pipe transport、Native Host Registry/manifest、Edge/Chrome 实机 Extension | NOT RUN | Windows backend/Registry 前置功能尚未实现 |
| Tauri bundle、安装器、升级/卸载、真实 externalBin Sidecar | NOT RUN | `bundle.active=false`，externalBin 和安装器 artifact 尚未配置 |
| aria2 官方 ZIP 下载、PowerShell 解压、`aria2c.exe` 检测和真实进程集成 | NOT RUN | 该专项需要下载/安装外部 artifact；本轮未获得本次安装授权，也未改变系统设置；当前环境无 `aria2c.exe` |
| aria2 断点、崩溃恢复、`.aria2` 清理、Unicode staging、403 回退 gallery-dl | BLOCKED | 依赖上一项真实 `aria2c.exe` 集成通过 |
| Windows 长路径、文件锁、磁盘不足、Tray/Single Instance/Autostart、Credential Manager | NOT RUN | 项目功能或 Windows backend 尚未具备专项验收前置条件 |
| 真实 Telegram 账号发送与持久化 | BLOCKED | 缺少真实账号、凭据和发送状态持久化环境；仅覆盖 HTTPS fake-server contract |
| Linux-only 的 pytest/cargo-clippy 重新执行 | NOT APPLICABLE | 本轮目标为 Windows；Windows 对应 pytest 已执行，clippy 已执行但失败 |

### Linux Follow-up

| 后续事项 | 原因 |
|---|---|
| Windows clippy re-validation | Linux 已将 `desktop/src-tauri/src/lib.rs:83` 的嵌套 `if let` 改为 let-chain，并通过 Linux fmt/check/test；下一轮 Windows 必须重新执行 `cargo clippy --workspace --all-targets -- -D warnings` |
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
| Windows clippy re-validation | WINDOWS_VERIFICATION_PENDING | Linux 修复尚未在 Windows 工作副本重新执行；不得将本地修复标记为 Windows PASS |

本轮 reconciliation 结论：Windows 已通过项目仍保持其原 PASS 记录；Windows clippy 的历史 FAIL 仍保留，原因已在 Linux 修复，但需要下一轮 Windows workspace clippy re-validation。aria2 官方 ZIP 下载、PowerShell 解压、`aria2c.exe` 实际检测/进程生命周期、断点恢复、崩溃恢复、`.aria2` 清理、Unicode staging、403 回退、GUI、Named Pipe/Registry、externalBin/安装器和真实账号项目继续保持 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_BLOCKED`、`NOT RUN` 或 `BLOCKED`，不提前标记 PASS。

### Windows re-validation after Linux clippy fix（2026-09-09）

当前 Linux let-chain 修复已同步至 E 盘，并完成 Windows re-validation；上一轮 clippy FAIL 记录保留为历史记录。

本轮再次复验（2026-09-09）：在同一当前 working tree 上重新执行 `npm run check/test/build`、`cargo fmt --all -- --check`、`cargo check --workspace`、设置项目 `PYTHON` 后的 `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`.venv\Scripts\pytest.exe sidecar\tests -q`、`npm run build:tauri` 和 `npm run dev:tauri`；结果全部为 PASS。Rust workspace 62 项测试全部通过，严格 clippy 复验通过。

| 验证项目 | 状态 | 关键结果 |
|---|---|---|
| Node check/test/build | PASS | Vite 构建通过；Extension 6 项测试通过 |
| Rust fmt/check/test | PASS | `cargo test --workspace` 通过，62 项测试全部通过 |
| Rust clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings` 通过 |
| Python sidecar | PASS | 10 passed |
| Tauri Release/dev | PASS | Release executable 生成，Vite、Rust Debug、Desktop 启动成功 |
| GUI 视觉验收 | BLOCKED | GUI automation helper 无可用 native app target |

aria2 官方 ZIP 下载、解压、`aria2c.exe` 检测和真实进程/恢复验证仍为 `NOT RUN`，原因是当前环境没有 `aria2c.exe`，本轮未安装外部 artifact 或修改系统设置。Edge/真实 X/Telegram、Named Pipe/Registry、externalBin/安装器仍为 `BLOCKED` 或 `NOT RUN`，原因分别是凭据缺失、Windows backend 未实现或发布 artifact 未配置。

本轮验证错误仅包括 MSVC linker stdout `#[warn(linker_messages)]` 和停止 Tauri 时的 Chromium `Error = 1411`/`STATUS_CONTROL_C_EXIT`，均不影响构建或启动结论。

Linux 后续处理：提供受控 aria2c artifact 并完成真实下载/解压/生命周期/恢复验证；继续实现 GUI、Named Pipe/Registry、externalBin/安装器和凭据相关链路。clippy 修复已完成 Windows re-validation，无需继续作为失败项处理。

本轮基线：Linux `main` / `f70f91399a0866ca8ee35741481e805a71556dc7`，包含 working tree changes；Windows `E:\Shiraishi\VSCode Workspace\Tw2Tg`，Windows 11 Insider Preview `10.0.29661` / 64 位，Node `v24.19.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。同步排除 `.git`、依赖、缓存、构建产物和 Linux 验证文档，其他内容 `rsync --checksum` 校验通过。

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
