# 开发环境与运行

## 开发平台

Linux 是主要开发环境。Windows 用于 Windows-specific build、runtime、filesystem、process、Native Host、GUI、packaging 和真实账号链路验证。

## 工具链

- Rust stable、Cargo 和 workspace dependencies。
- Node.js、npm 和 workspace dependencies。
- Python 3.10 或更高版本。
- `gallery-dl`，由 Sidecar 运行时提供。
- Tauri CLI 2，用于 Desktop 开发和构建。
- WebdriverIO 9 与 @wdio/tauri-service，用于 Windows Tauri 窗口 smoke/E2E 验证。
- `tauri-plugin-wdio` 1.4.0 与 `@wdio/tauri-plugin` 1.4.0，仅用于 Debug/专用高级 E2E 验证。
- Debug-only Tauri MCP Bridge：项目通过 Rust crate 提供 MCP WebSocket bridge；MCP server 不作为项目 npm 依赖提交。
- Windows 验证另外需要 MSVC、Windows SDK、WebView2、Edge/Chrome 和项目规定的 Python 环境。

## 安装依赖

在仓库根目录执行：

```bash
npm ci
cargo check --workspace
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e sidecar
python -m pip install pytest
```

Windows 使用项目虚拟环境中的解释器运行 Sidecar 测试；不要假设 `python3` 存在于 Windows PATH。

## 配置 Sidecar

复制 `.env.example`，并设置：

```dotenv
XARCHIVE_SIDECAR_PROGRAM=python3
XARCHIVE_SIDECAR_ARGS=["-m","xarchive_downloader"]
```

`XARCHIVE_SIDECAR_ARGS` 必须是 JSON 字符串数组，以保留包含空格或非 ASCII 字符的路径。

## 开发命令

```bash
npm run check
npm run test
npm run build
npm run dev:tauri
npm run build:tauri
npm run test:e2e:windows --workspace desktop
npm run build:tauri:wdio --workspace desktop
npm run test:e2e:windows:advanced --workspace desktop
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
.venv/bin/python -m compileall -q sidecar/src sidecar/tests
.venv/bin/python -m pytest sidecar/tests -q
```

Rust Clippy 不是所有 Rust 安装的默认组件。如果 `cargo clippy` 报告
`cargo-clippy is not installed`，执行：

```bash
rustup component add clippy
```

Sidecar 测试应使用仓库根目录的 `.venv`，不要依赖系统级 `pytest` 或 `python`：

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -e sidecar pytest
.venv/bin/python -m pytest sidecar/tests -q
```

## 运行关系

开发版 Desktop 由 Tauri 启动 Vite frontend，并通过 `XARCHIVE_SIDECAR_PROGRAM` 和 `XARCHIVE_SIDECAR_ARGS` 启动 Python Sidecar。当前 `tauri.conf.json` 未启用 bundle，不能将 `npm run build:tauri` 视为已生成可分发安装包。

### Tauri MCP Bridge

项目源码只提交 `tauri-plugin-mcp-bridge` Rust Bridge 配置，不把
`@hypothesi/tauri-mcp-server` 安装到项目 `package.json`。Bridge 仅在 Debug
构建注册，并固定绑定 `127.0.0.1`；Release 构建不会启动 MCP WebSocket 服务。

在有 Tauri GUI 的环境中，需要 MCP 工具时，由 Agent 环境单独启动 server：

```bash
npx -y @hypothesi/tauri-mcp-server
```

Bridge 默认从 WebSocket 端口 `9223` 开始寻找可用端口，最多扫描到 `9322`。
当前配置使用 localhost，避免把开发调试接口暴露到局域网。
### WebdriverIO + Tauri service

Desktop 的 wdio.conf.mjs 是 Windows 原生窗口自动化入口。它默认驱动仓库根目录的 target/release/xarchive-desktop.exe，使用 @wdio/tauri-service 的 external provider，并由 service 自动管理匹配的 Microsoft Edge WebDriver。运行前先在 Windows 工作副本中完成：

    npm ci
    npm run build:tauri
    npm run test:e2e:windows --workspace desktop

可用环境变量：

- WDIO_APP_BINARY：覆盖 Tauri .exe 的绝对或相对路径；
- TAURI_DRIVER_PORT：覆盖 external driver 端口，默认 4444；
- WDIO_AUTO_INSTALL_TAURI_DRIVER=0：关闭 external provider 所需的 tauri-driver 自动安装；默认开启，Windows 首次运行可自动准备匹配 driver；
- WDIO_LOG_LEVEL：覆盖 WDIO 日志级别；
- WDIO_CAPTURE_LOGS=1：显式启用 service 日志捕获；高级插件 E2E 命令默认启用；
- WDIO_LOG_DIR：保存 service 日志的目录，默认 desktop/test-artifacts/wdio。

当前 smoke spec 只验证真实 Tauri 窗口的 DOM/可见性和稳定区域，不调用尚未接入的 Native Host/Named Pipe 或 browser.tauri 扩展 API。Windows 原生 WebView2、DPI、键盘、辅助技术和真实应用 IPC 结论仍按 Windows validation queue 记录，不能由 Linux Node 检查替代。

#### tauri-plugin-wdio 高级能力注册

高级 Windows E2E 使用 `wdio-e2e` feature、`tauri.wdio.conf.json` 和 `wdio.json` capability，显式执行 `build:tauri:wdio` 与 `test:e2e:windows:advanced`。高级命令通过 Node wrapper 设置环境变量，在 PowerShell/CMD 与 Linux 上均可使用。

配置边界如下：

1. `tauri-plugin-wdio` 仅在 `wdio-e2e` feature 下注册；普通 Debug/Release 均不注册插件。
2. `tauri.conf.json` 只启用 `default`，`tauri.wdio.conf.json` 只启用 `wdio`，因此普通构建不开放 WDIO 命令。
3. `withGlobalTauri` 为插件 guest JS 提供所需的全局 Tauri API。
4. `@wdio/tauri-plugin` 在 `desktop/src/main.jsx` 中加载 guest JS。
5. 高级 spec 覆盖 `browser.tauri.isTauriApiAvailable`、`execute`、command mocking 和 mock cleanup；真实 Windows WebView2、日志收集和窗口生命周期仍必须在 Windows 执行。

external provider 不需要额外注册 `tauri-plugin-wdio-webdriver`；该 Rust-only 插件只适用于 embedded provider。

Linux 无 GUI 或远程开发服务器不应为了 MCP 启动 Tauri；继续执行 Rust、Node、
Python 和非 GUI integration checks，并把 GUI/Windows 项目统一放入 Windows
Validation Queue。Windows/Cline 有可用 Tauri GUI 时，再在对应 Agent 环境加载
`@hypothesi/tauri-mcp-server`。

## 数据目录

当前实现默认在进程工作目录下创建 `X-Archive/`，其中包含 SQLite、staging、归档文件和用户 profile。改变 archive root 或权限策略属于独立架构任务；开发和验证不得将真实私人数据写入共享目录。

## Windows 开发与验证

Windows 工作副本必须由 Linux 源目录单向同步，且不能把 Windows 本地配置、凭据、缓存或生成物反向同步到 Linux。完整流程见 [`cross-platform-validation.md`](cross-platform-validation.md)。