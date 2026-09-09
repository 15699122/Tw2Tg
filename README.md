# Tw2Tg / XArchive

X/Twitter 本地归档桌面应用。用户在 Edge/Chrome 的 X 页面点击归档按钮后，由 Tauri/Rust 统一管理任务、SQLite 状态、本地原文件和 Telegram 展示；Python Sidecar 使用 gallery-dl 负责 X metadata 提取与默认媒体下载。

## 当前状态

项目于 **2026-09-08** 按 Greenfield Monorepo 初始化。当前已完成 Sprint 0、Sprint 1 协议链路、M1 本地归档核心、gallery-dl Adapter、媒体文件结果契约、Rust Sidecar 结果转换、ArchiveService 端到端闭环、aria2 跨平台 RPC client、浏览器协议/Native Messaging framing、Extension DOM/Bridge 基础、retry/backoff、TagEngine、用户/标签 SQLite Repository、Telegram request/formatter/SecretStore contract、Telegram HTTPS transport 和 Tauri Desktop Dashboard。
Windows Rust workspace 的 fmt、check、test、Debug/Release 构建和完整 clippy 已通过；Supervisor 测试需在 Windows 显式使用项目 `.venv\Scripts\python.exe`，默认 `python3` 不在 Windows PATH。真实 X 认证下载、aria2c executable supervisor、Windows Named Pipe、Native Host 注册、Credential Manager、Tray/Autostart、Sidecar externalBin、安装器和真实 Telegram 发送仍需 Windows、账号或发布环境。

## 架构原则

```text
MV3 Extension → Rust Native Messaging Host → Windows Named Pipe → Tauri/Rust Desktop
                                                          ├─ Python gallery-dl Sidecar
                                                          ├─ SQLite
                                                          ├─ Telegram
                                                          └─ optional aria2
```

- Rust 是唯一业务状态所有者。
- Python 只负责 X 提取和受控下载，不访问主 SQLite，也不负责 Telegram。
- Extension 只传 Tweet metadata 和状态，不传 Cookie 或媒体二进制。
- gallery-dl 是默认提取器和下载器。
- aria2 后期作为可选 DownloadTransport；IDM 不进入核心下载链路。
- 所有跨进程通信使用版本化 JSON/JSONL 协议。
- 当前可移植层优先提供可测试的协议、请求模型和 mock/fake transport；平台适配不进入核心状态模型。

## 目录

| 目录 | 用途 |
|---|---|
| `desktop/` | Tauri Desktop、React 前端和本地 shadcn/ui 组件 |
| `extension/` | Manifest V3 Extension |
| `sidecar/` | Python gallery-dl Sidecar |
| `crates/` | Rust 核心、协议和 Native Host |
| `shared/protocol-schema/` | 跨语言 JSON Schema |
| `docs/` | 架构、协议和开发文档 |
| `aidlc-docs/` | AI-DLC 规划产物 |

## 常用命令

```bash
npm run check
npm run test
cargo check --workspace
cargo test --workspace
python3 -m pytest sidecar/tests
npm run test --workspace extension
```

## 开发顺序

1. Sprint 0：工程骨架和工具链。
2. Sprint 1：共享协议与 Fake Sidecar。
3. Sprint 2：真实 gallery-dl 下载。
4. M1：SQLite、本地文件和幂等恢复。
5. 当前阶段跨平台开发：aria2 client、Browser protocol、Native Messaging framing、Extension 基础、retry/backoff、TagEngine、用户/标签 Repository、Telegram contract。
6. Windows/外部环境阶段：Named Pipe、Native Host 注册、Edge Cookie、真实 X、Credential Manager、Sidecar externalBin、Tray、安装器和真实 Telegram。

详见 [`docs/development/roadmap.md`](docs/development/roadmap.md)。

## 平台说明

当前目标平台为 Windows。Linux 可用于协议、Rust、Python 和前端开发；Named Pipe、Native Host Registry、Edge/Chrome 安装和 Tauri 打包必须在 Windows 或 Windows CI 上验证。

### Windows 验证记录（2026-09-09）

- Node 检查、测试和构建通过；首次执行发现 E 盘项目依赖缺少 `vite`，本轮执行项目内 `npm ci` 安装 70 个依赖并审计为 0 个漏洞；npm 提示 `esbuild` postinstall script 尚未批准，但不影响本轮构建。Desktop workspace 当前无 Node 测试用例，Extension workspace 当前 6 个测试全部通过。
- Windows 验证前曾因缺少 `desktop/src-tauri/icons/icon.ico` 阻塞完整 Tauri 构建；当前已补齐开发阶段 `icon.ico`，Windows workspace 的 check、test、Debug/Release 构建和 `npm run build:tauri` 已复验通过。
- Windows Rust workspace 当前共有 54 个单元测试，全部通过；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 和 `cargo clippy --workspace --all-targets -- -D warnings` 均通过。首次未设置 `PYTHON` 时两个 Supervisor 测试因默认 `python3` 不存在而报 `NotRunning`，改用项目 `.venv\Scripts\python.exe` 后复验通过。
- Python 在本地 `.venv` 中 editable 安装 Sidecar，并安装 gallery-dl 1.32.11 后，10 个测试全部通过。
- `cargo fmt --check` 通过。
- 此前在 `crates/xarchive-sidecar-supervisor/src/lib.rs:17` 检出的 `large_enum_variant` 已通过 `Download(Box<DownloadEvent>)` 修复并在 Windows 复验；Telegram formatter 的 `single_char_add_str` 已修复，Windows 完整 clippy 已通过。
- 真实 sidecar 使用 gallery-dl 1.32.11，在含中文、空格和 Unicode 的路径中完成 `ready → started → log → failed` JSONL 流程；示例 X URL 返回 `EXTRACT_OR_DOWNLOAD_FAILED`，未进行真实账号认证下载。
- Visual Studio BuildTools/MSVC、Windows SDK、MSBuild 和 WebView2 可用；`aria2c`、`cmake`、`ninja` 不在 PATH。aria2 跨平台 HTTP client 已实现，但 aria2c 进程集成尚未实现。
- Named Pipe、Native Host 注册、浏览器安装、Tauri GUI 视觉验收和安装包验证尚未执行；这些仍需 Windows 实机或 Windows CI 验证。Native Messaging framing、Extension DOM/Bridge 和结构化 unavailable 错误已在跨平台环境完成测试。Windows `npm run dev:tauri` 已启动 Vite、Rust Debug 和 Desktop 可执行文件，`npm run build:tauri` 已生成 Release 可执行文件；UI 自动化 helper 仍未能初始化。
- Tauri Desktop 前端、Rust workspace Debug/Release 编译和测试已通过；Release 可执行文件已生成。当前未启用 bundle，Sidecar 仍通过显式环境配置启动，尚未配置真实 `externalBin` 打包资源。
- Windows 已通过 `npm run dev:tauri` 启动链路和 `npm run build:tauri` Release 构建；停止开发进程时出现 Chromium 类注销警告 `Error = 1411`，进程最终以 Ctrl+C 正常终止。仍需接入可分发的 Sidecar executable、真实 `externalBin` 配置和 bundle/安装器验证。
- 上述 Tauri 命令可从仓库根目录执行；对应脚本会转发到 `desktop` workspace。

Windows 相关的开发、实机验证、Windows CI、安装器和发布任务统一见 [`docs/development/windows-validation.md`](docs/development/windows-validation.md)。

当前阶段非 Windows 开发完成清单见 [`docs/development/non-windows-completion.md`](docs/development/non-windows-completion.md)。

## 许可证说明

gallery-dl 和 aria2 均涉及 GPL 许可证。正式分发前必须维护 `THIRD_PARTY_NOTICES.md`、许可证副本、版本清单和对应源代码获取方案。闭源或商业发行前应进行法律审查。
