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

当前发布范围只生成 Windows 便携版 `.exe`，不生成 installer/bundle。便携版以 `.exe` 所在目录为 portable root，使用 `config/`、`cache/`、`download/`、`extension/`、`logs/` 和 `sidecar/`；不创建 `telegram/`。

目标发布模型是 Core Bootstrap + Offline Bundle：Core 初始发行物只包含 Desktop `.exe`，首次运行后由固定 embedded component catalog 管理 Worker、gallery-dl、aria2、Native Host 和 Extension；Offline Bundle 预置相同组件清单。U9 已完成 ComponentManager 的 catalog/校验/本地激活/rollback Linux scope；U13 已完成 Offline Bundle manifest/parity 契约，当前 catalog 尚无真实组件条目，待版本化 release assets、精确 SHA-256、license 和 probe 定稿后填充，不执行动态 `latest` 或未经验证的网络下载。

U10 的 Linux Bootstrap 行为：Desktop 启动后可通过 `get_component_bootstrap_status` 查询固定 catalog 和本地 active markers；设置页显示 catalog 版本、ready/missing 状态和诊断信息。首次归档目录选择仍由 `complete_download_setup` 处理，选择后会重建 executor runtime。Bootstrap 状态不等于 Windows 组件已安装，也不替代 U11 release asset 验证。

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

## 配置 Extension 发布身份

Browser Extension 的 ID 由 manifest public key 决定，而不是由运行目录或随机值决定。当前仓库使用固定的开发/自托管 identity：

```text
XARCHIVE_EXTENSION_ID=iaajefkoanbkleojofoadeakelihbjne
```

### 私钥与公钥

| 内容 | 位置 | 是否可提交 |
|---|---|---|
| Extension signing private key | 仓库外，例如 `$HOME/xarchive-extension.pem`（`0600`） | 否；同时不得进入 ZIP、Full bundle、日志或 Actions 产物 |
| Manifest `"key"`（DER 公钥 Base64） | `extension/manifest.json` | 是 |
| Extension ID | GitHub Repository Variable/Secret、Native Host `allowed_origins` | 是（本身不是秘密） |

派生关系：

```text
public key DER → SHA-256 → 取前 128 bit → 每个 nibble 映射 a–p → 32 位 Extension ID
```

仓库脚本 `desktop/scripts/extension-identity.mjs` 提供同一派生逻辑，并作为单一校验入口：

```bash
node desktop/scripts/extension-identity.mjs verify --extension-dir extension --expected-id "$XARCHIVE_EXTENSION_ID"
```

### 配置 GitHub Actions

Extension ID 不是秘密，仓库使用 Repository Variable 作为单一事实来源（workflow 读取 `vars.XARCHIVE_EXTENSION_ID || secrets.XARCHIVE_EXTENSION_ID`，Secret 保持未设置）：

```bash
gh variable set XARCHIVE_EXTENSION_ID --repo <owner>/<repo> --body 'iaajefkoanbkleojofoadeakelihbjne'
```

检查（Variable 会回显值，Secret 不会）：

```bash
gh variable list --repo <owner>/<repo>
gh secret list --repo <owner>/<repo>
```

变量与 `extension/manifest.json` 的 `key` 必须一致；`windows-release.yml` 会在构建 Native Host 之前用 `extension-identity.mjs verify` 校验，不一致时该步骤失败。

### 私钥备份

私钥应保存在仓库外并至少有一份独立备份；SOPS 只保护私钥，不用来加密公开的 Extension ID：

```bash
sops encrypt --age "$AGE_RECIPIENT" --input-type binary --output-type json \
  "$HOME/xarchive-extension.pem" > "$HOME/xarchive-secret-backup/xarchive-extension.pem.sops.json"
```

缺少 age identity 或多份备份时，不得宣称 identity 可恢复。

### 商店发布边界

当前 ID 只是开发/自托管 identity。创建 Chrome Web Store 或 Edge Add-ons 项目后必须确认最终商店 ID；若 Chrome 与 Edge 最终使用不同 ID，需要改为多 `allowed_origins` 模型并同步 `desktop/scripts/native-host-package.mjs`、workflow、installation manifest 和测试，不能把两个 ID 拼进单个变量。

## 开发命令

```bash
npm run check
npm run test
npm run build
npm run dev:tauri
npm run build:tauri
npm run test:e2e:windows --workspace desktop
    npm run build:tauri:wdio --workspace desktop
    npm run build:portable:windows --workspace desktop
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

### 构建 Windows Sidecar worker artifact

Windows worker 使用 PyInstaller 生成，不依赖目标机器上的 Python/venv。构建定义位于
`sidecar/pyinstaller/xarchive-downloader.spec`，推荐通过 GitHub Actions 的
`Windows Sidecar Worker Artifact` workflow 生成并保留 SHA-256。生成的目录应包含
`xarchive-downloader.exe`，并在交给 portable 组装脚本前完成 `--help` smoke check。

当前 Linux 环境没有 Windows bootloader，因此本阶段只验证 spec/脚本结构，不把 Linux
环境中的 Python worker 运行结果当作 Windows `.exe` artifact。

正式目标还要求 Worker、Native Host、Extension、gallery-dl 和 aria2 使用固定版本与 SHA-256，并在 Release pipeline 中生成版本化资产；在相应 Unit 完成前，不要从 `latest` 或临时 Actions artifact 推断可发布组件。

## 运行关系

开发版 Desktop 由 Tauri 启动 Vite frontend，并通过 `XARCHIVE_SIDECAR_PROGRAM` 和 `XARCHIVE_SIDECAR_ARGS` 启动 Python Sidecar。`npm run build:tauri` 生成平台 binary；`npm run build:portable:windows --workspace desktop` 负责组装便携目录。当前 `tauri.conf.json` 未启用 bundle，因此不会生成 installer。

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

`npm ci` 会触发仓库根 `postinstall` 钩子，自动执行 `desktop/scripts/patch-wdio-tauri-service.mjs`，该脚本幂等修补已安装 service 中仅接受 `MSEdgeDriver x.y` 的发现正则，使其同时接受当前 Microsoft 可执行文件输出的 `Microsoft Edge WebDriver x.y`。该补丁仅作用于 `node_modules/@wdio/tauri-service/dist/esm/index.js` 与 `dist/cjs/index.js`，不修改业务代码、Dashboard 断言、production capability、driver 版本或自动下载策略；未来 service 版本若移除该正则则不会失败。参见 `docs/development/testing.md`“WDIO service Edge driver banner 兼容性补丁”。

可用环境变量：

- WDIO_APP_BINARY：覆盖 Tauri .exe 的绝对或相对路径；
- TAURI_DRIVER_PORT：覆盖 external driver 端口，默认 4444；
- WDIO_AUTO_INSTALL_TAURI_DRIVER=1：显式允许 external provider 自动安装 tauri-driver；默认关闭，Windows release workflow 使用预先固定并校验的 driver；
- WDIO_AUTO_DOWNLOAD_EDGE_DRIVER=1：显式允许 service 自动下载 EdgeDriver；默认关闭，Windows release workflow 要求 PATH 中存在匹配的 `msedgedriver.exe`；
- EDGEDRIVER_VERSION：固定 EdgeDriver 版本，例如 `152.0.4191.66`；Windows workflow 会检查命令输出与该版本一致；
- WDIO_LOG_LEVEL：覆盖 WDIO 日志级别；
- WDIO_CAPTURE_LOGS=1：显式启用 service 日志捕获；高级插件 E2E 命令默认启用；
- WDIO_LOG_DIR：保存 service 日志的目录，默认 desktop/test-artifacts/wdio。

当前 smoke spec 只验证真实 Tauri 窗口的 DOM/可见性和稳定区域，不调用尚未接入的 Native Host/Named Pipe 或 browser.tauri 扩展 API。Windows 原生 WebView2、DPI、键盘、辅助技术和真实应用 IPC 结论仍按 Windows validation queue 记录，不能由 Linux Node 检查替代。

#### tauri-plugin-wdio 高级能力注册

高级 Windows E2E 使用 `wdio-e2e` feature、`tauri.wdio.conf.json` 和 `wdio.json` capability，显式执行 `build:tauri:wdio` 与 `test:e2e:windows:advanced`。两个命令都通过 Node wrapper 直接加载本地 CLI，在 PowerShell/CMD 与 Linux 上均可使用，并透传真实退出码。

配置边界如下：

1. `tauri-plugin-wdio` 仅在 `wdio-e2e` feature 下注册；普通 Debug/Release 均不注册插件。
2. `tauri.conf.json` 只启用 `default`，`tauri.wdio.conf.json` 只启用 `wdio`，因此普通构建不开放 WDIO 命令。
3. `withGlobalTauri` 为插件 guest JS 提供所需的全局 Tauri API。
4. `@wdio/tauri-plugin` 仅在 `VITE_WDIO_E2E=1` 的专用构建中由 `desktop/src/main.jsx` 加载 guest JS；普通 release 不加载该 guest JS。
5. 高级 spec 通过 `browser.tauri.execute` 检查 `window.wdioTauri`，再覆盖 frontend execute、command mocking 和 mock cleanup。真实 Windows WebView2、日志收集和窗口生命周期仍必须在 Windows 执行。
6. wrapper 不直接调用 Windows `.cmd` shim；`desktop/scripts/wdio-tauri-service.mjs` 复用官方 launcher，但跳过普通 artifact 不具备的 `plugin:wdio` focus probe，并避免 service 与 spec 重复执行 mock/session cleanup。Windows driver 生命周期仍需实机确认，不能仅凭 Linux 静态检查标记通过。

external provider 不需要额外注册 `tauri-plugin-wdio-webdriver`；该 Rust-only 插件只适用于 embedded provider。

Linux 无 GUI 或远程开发服务器不应为了 MCP 启动 Tauri；继续执行 Rust、Node、
Python 和非 GUI integration checks，并把 GUI/Windows 项目统一放入 Windows
Validation Queue。Windows/Cline 有可用 Tauri GUI 时，再在对应 Agent 环境加载
`@hypothesi/tauri-mcp-server`。

## 数据目录

当前便携实现使用 `.exe` 所在目录作为 portable root：`config/config.yaml` 保存配置，`config/archive.sqlite3` 保存数据库，`cache/staging/` 保存临时 staging，`cache/downloads/` 和 `cache/runtime/` 保存临时内容，最终归档保存到 `download/` 或用户选择的系统 `Downloads/XArchive`。应用日志位于同级 `logs/`，默认最多保留 5 个 `xarchive-*.log`，可通过 GUI 或 YAML 的 `logging.max_files` 调整。Telegram send-state 暂时继续存储在主 SQLite，不创建 `telegram/`。

首次启动若最终下载目录不存在，GUI 会提供创建便携 `download/` 或使用系统 Downloads 的选择；拒绝创建不会回退到进程工作目录。

## Windows 开发与验证

Windows 工作副本必须由 Linux 源目录单向同步，且不能把 Windows 本地配置、凭据、缓存或生成物反向同步到 Linux。完整流程见 [`cross-platform-validation.md`](cross-platform-validation.md)。

Windows 无外网时不能把 service 的自动下载作为前置保证：应手动安装与 WebView2/Edge 主版本匹配的 msedgedriver，并让 Windows "where msedgedriver.exe" 能解析到它；仅存在于临时缓存目录不等于 service 可发现。若驱动未就绪，WDIO 可能在 onPrepare 或 tauri-driver 启动阶段结束，不能把该结果记为 native smoke PASS。

### Windows 本地 msedgedriver 半永久目录

当前 Windows 验证副本已保存与 Edge/WebView2 版本匹配的 driver：

- 版本：152.0.4191.66
- 路径：E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\test-artifacts\msedgedriver\152.0.4191.66\msedgedriver.exe
- SHA-256：9E9B1F048D2CC781DEEE084E6CB6E9F2F3417A33ED45D96CF7C34BE4EB23077B
- 目录受 .gitignore 的 desktop/test-artifacts/ 规则保护，仅用于 E: Windows 验证，不同步回 WSL/Linux source。

运行 WDIO 前，在当前 PowerShell 会话将 driver 目录加入 PATH：

    $root = "E:\Shiraishi\VSCode Workspace\Tw2Tg"
    $driverDir = Join-Path $root "desktop\test-artifacts\msedgedriver\152.0.4191.66"
    $env:Path = "$driverDir;$env:Path"
    where.exe msedgedriver.exe
    msedgedriver.exe --version
    $env:WDIO_APP_BINARY = Join-Path $root "target\release\xarchive-desktop.exe"
    npm run test:e2e:windows --workspace desktop

advanced 验证使用：

    $env:WDIO_ADVANCED = "1"
    npm run test:e2e:windows:advanced --workspace desktop

上述 PATH 只影响当前 PowerShell 会话；若需要对当前用户长期生效，可将同一目录加入用户级 Path，之后重新打开 PowerShell。优先使用会话级 PATH，避免污染其他项目。

注意：当前 wdio.conf.mjs 在 Windows 仍启用 autoDownloadEdgeDriver。@wdio/tauri-service 1.4.0 对当前 driver 输出文本的版本识别可能不命中，因此即使 PATH 中已有该 driver，service 仍可能尝试联网下载并输出 warning；tauri-driver 仍可使用 PATH 中的手动 driver。网络不可用时，该 warning 不应被误记为 driver 文件不存在，最终仍需观察 tauri-driver、Node worker 和真实 WebView2 session 结果。