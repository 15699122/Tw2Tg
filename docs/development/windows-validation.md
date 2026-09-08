# Windows 开发与验证清单

> 适用平台：Windows 10/11，优先验证 Edge，并回归 Chrome。文档日期：2026-09-08。

Linux 可验证协议、Rust 核心、Python 逻辑和前端静态检查，但不能替代 Windows 专属集成验证。本文集中记录必须在 Windows 实机或 Windows CI 完成的任务。

## 状态定义

| 状态 | 含义 |
|---|---|
| 已完成 | 已有明确测试结果或代码验证结果 |
| 待实现 | 对应功能尚未开发 |
| 待验证 | 功能已有，但尚未在 Windows 目标环境验证 |
| 阻塞 | 依赖工具、凭据、证书或外部环境 |

## 当前基线（2026-09-08）

| 项目 | 最新结果 | 状态 |
|---|---|---|
| Node workspace | 检查/测试/构建通过；两个 workspace 暂无实际测试用例 | 已完成基础验证 |
| Rust workspace | `cargo check --workspace` 通过；`cargo test --workspace` 的 26 个测试全部通过 | 已完成基础验证 |
| Rust 格式 | Windows `cargo fmt --check` 通过 | 已完成 |
| Rust lint | `large_enum_variant` 已通过 `Box<DownloadEvent>` 修复；需在 Windows 重新运行 clippy | 待复验 |
| Python Sidecar | `.venv` + editable 安装，gallery-dl 1.32.11，10 个测试全部通过 | 已完成基础验证 |
| Sidecar 路径兼容 | 中文、空格、Unicode 路径下完成 JSONL `ready → started → log → failed` 流程 | 已完成基础验证 |
| 示例 X URL | 返回 `EXTRACT_OR_DOWNLOAD_FAILED` | 已记录，不能视为认证下载成功 |
| Edge Cookie/真实 X | 尚未使用明确账号环境验证 | 外部账号环境阻塞 |
| Named Pipe/Native Host/Tauri | 对应功能尚未实现 | 待开发，不是测试失败 |

---

## P0：运行链路

### W-P0-01 工具链

**验证方式：** Windows 实机 + Windows CI；**状态：** 基础验证完成，待 clippy 复验。

确认 Rust/Cargo、rustfmt、clippy、Node/npm、Python、Visual Studio C++ Build Tools、Windows SDK 和 x64 target 可用。当前 rustfmt、check、test 已完成；clippy 需要在包含最新 `Box<DownloadEvent>` 修复的代码上重新执行。

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

**验证方式：** Windows 实机 + Windows CI；**状态：** 待实现。

验证 Tauri 2 开发/Release 构建、前后端通信、资源路径、`externalBin` Sidecar、打包后 Sidecar 启动、安装到含空格/非 ASCII 路径及非系统盘。

### W-P1-02 Named Pipe

**验证方式：** Windows 实机；**状态：** 待实现。

目标：

```text
\\.\pipe\xarchive-v1
```

验证 Server 启动、Native Host 连接/重连、多连接、request_id 路由、批量 `query_status`、Desktop 退出、ACL、消息大小限制、非法 JSON/协议版本/action 拒绝。

### W-P1-03 Native Messaging Host

**验证方式：** Windows Edge/Chrome 实机；**状态：** 待实现。

验证 Chromium 长度前缀 framing、stdin/stdout 二进制读写、stdout 无日志、stderr 诊断、origin allowlist、Host manifest、Desktop 离线、大 payload 拒绝，以及 Host 被反复启动/关闭。

### W-P1-04 Registry 与 Host manifest

**验证方式：** Windows 实机；**状态：** 待实现。

确认 Chrome/Edge 注册路径、固定 Extension ID、安装/升级/卸载、管理员/非管理员权限、安装目录移动、路径含空格时的行为。

### W-P1-05 MV3 Extension

**验证方式：** Windows Edge/Chrome 实机；**状态：** 待实现。

验证开发版加载、Extension ID、Timeline、Tweet Detail、SPA 路由、虚拟滚动、MutationObserver、按钮去重、多标签同步、Service Worker 重启、Native Messaging 重连和最小权限。

场景：

```text
x.com / twitter.com / Timeline / Detail / Quote / Reply
刷新 / 多标签 / Desktop 未启动
```

### W-P1-06 aria2c

**验证方式：** Windows 实机 + CI；**状态：** 协议层已完成，进程集成待实现。

验证随应用提供的 `aria2c.exe`、版本/hash、loopback RPC、Secret、`addUri/tellStatus/pause/unpause/remove`、大文件断点、崩溃恢复、`.aria2` 清理、Unicode staging 路径和 403 回退 gallery-dl。

优先使用本地 HTTP 测试服务器，不直接依赖 X CDN。

---

## P1：文件系统与生命周期

### W-P1-07 Windows 文件系统

**验证方式：** Windows 实机；**状态：** 逻辑已测试，Windows 待验证。

覆盖系统盘/非系统盘、空格、中文/Unicode、Windows 保留字符和文件名、`CON/PRN/AUX/NUL`、超长路径、磁盘不足、文件锁、目标目录已存在、遗留 staging 和异常退出恢复。

### W-P1-08 Tray/Single Instance/Autostart

**验证方式：** Windows 实机；**状态：** 待实现。

验证 Tray 启动、关闭隐藏、打开/退出菜单、第二次启动激活已有实例、登录自启动、禁用自启动、后台 Sidecar 工作和关机安全退出。

### W-P1-09 Secret Store

**验证方式：** Windows 实机；**状态：** 待实现。

验证 Bot Token 写入 Credential Manager 或 Stronghold；应用重启可读取；删除/更新不残留旧值；Token 不进入 SQLite、Extension、Sidecar、日志；不同 Windows 用户不能意外共享。

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
W-P0-01（重新运行 clippy）→ W-P0-02
→ W-P0-03（准备账号/Profile）→ W-P0-04（真实归档）
→ W-P1-01 → W-P1-02 → W-P1-03 → W-P1-04 → W-P1-05
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
