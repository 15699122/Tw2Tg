# Tw2Tg / XArchive

XArchive 是一个本地优先的 X/Twitter 归档桌面应用。用户可以从浏览器中的 X 页面发起归档，由桌面应用保存 Tweet metadata、原始媒体、用户资料和任务状态。

> **Telegram 发送当前不在开发与发布范围内。** 项目保留 Telegram 的请求模型、格式化、媒体分组与幂等发送等基础代码，但自 2026-10-01 起暂停该方向的功能开发，发布说明不宣传 Telegram 能力已实现，真实发送也未取得任何平台验收证据。不要把本仓库理解为可用的 Telegram 发送功能。

## 使用范围与限制

- 本项目用于**个人用途**，按现状提供；主要在作者受控的开发环境中开发和验证。
- **不保证在其他设备、账号、网络或浏览器配置下可用。**
- **不承诺真实 X 帖子归档成功。** 真实归档依赖 X 的登录状态、页面结构、限流和网络环境，本版本没有对真实 X 归档做出成功保证。
- 各发布资产不做跨设备或完整功能承诺；下表只说明每类文件的形态与定位。
- 不做功能承诺不等于可以发布来源不明或与说明不符的文件：每个资产的名称、内容、版本和校验值必须与实际发布物一致。

## 功能

- 从 X/Twitter 页面发起单条 Tweet 归档。
- 保存 Tweet metadata、`tweet.json`、`tweet.txt` 和原始媒体文件。
- 使用 SQLite 保存任务、用户、标签、媒体和事件状态。
- 通过 gallery-dl 处理 X metadata extraction，并由 aria2-only transfer 写入媒体 staging。
- 记录回复、引用 Tweet、用户名称历史和用户 profile 文件。
- 保留 Telegram Bot API 请求模型、格式化、媒体分组和幂等发送的基础代码，但该方向**当前不开发、不在发布范围内、无平台验收证据**（见开头说明）。
- 提供可选的 aria2 下载传输和 Windows aria2 管理入口。
- 提供浏览器 Extension、Native Messaging 协议和 Tauri Desktop Dashboard。

> 当前实现已完成 U8 legacy-path removal：Sidecar protocol v2、extraction-only 和 aria2-only transfer 是当前代码链路；Core Bootstrap、真实 Windows Browser/Native Host integration、Registry、Named Pipe 和最终发布验收仍有 pending 项。目标终态和后续顺序见 [`docs/development/roadmap.md`](docs/development/roadmap.md)。

当前阶段构建 Windows 便携版（便携目录），不生成安装器；发布页另按《发布资产（v0.2.0）》提供 `.exe`、7z 资产与校验文件。便携目录包含 `config/`、`cache/`、`download/`、`extension/`、`logs/` 和 `sidecar/`；不创建 `telegram/` 目录。首次启动时，如果便携目录不存在 `download/`，应用会询问创建该目录，拒绝后使用系统“下载”目录下的 `XArchive/`。Windows Native Host 注册、Named Pipe、真实 X 账号链路、Credential Manager、真实 Telegram 账号发送和完整发布验收仍未完成。

## 技术栈

- Tauri 2 + Rust：桌面应用、业务状态、任务编排、SQLite 和文件提交。
- React + Vite：桌面 Dashboard。
- Manifest V3：Edge/Chrome 浏览器扩展。
- Python + gallery-dl：X metadata extraction-only Sidecar。
- SQLite：本地任务、metadata、用户、标签、事件和发送状态。
- JSON/JSONL + JSON Schema：跨进程协议。
- Telegram Bot API：代码层面保留的可选展示层，**当前不开发、不在发布范围内**。

当前运行时已将 gallery-dl 限定为 extraction-only，并由 aria2 作为唯一媒体 transfer backend（U8 legacy-path removal 已完成，见上方说明）；尚未完成的是真实 Windows Browser/Native Host integration 与最终发布验收，不应据此外推这些已通过。

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
- gallery-dl 是 extraction-only adapter；aria2 是唯一媒体 transfer backend。
- IDM 不进入核心下载链路。
- 所有跨进程通信使用版本化 JSON/JSONL 协议。
- 当前可移植层优先提供可测试的协议、请求模型和 mock/fake transport；平台适配不进入核心状态模型。

## 数据与隐私

- Rust/Desktop 是业务状态的唯一所有者。
- 浏览器扩展只传递 Tweet metadata、命令和状态，不传 Cookie 或媒体二进制。
- Python Sidecar 不直接访问主 SQLite，也不负责 Telegram。
- 下载内容先写入 staging，由 Rust 检查路径、大小和 SHA-256 后再提交到归档目录。
- Token、Cookie 和 RPC secret 不应写入 SQLite、浏览器消息或普通日志。
- 归档数据保存在本机文件系统；用户应自行选择适合备份和访问控制的目录。

## 项目结构

| 目录 | 用途 |
|---|---|
| `desktop/` | Tauri Desktop、React 前端和本地 shadcn/ui 组件 |
| `extension/` | Manifest V3 Extension |
| `sidecar/` | Python gallery-dl Sidecar |
| `crates/` | Rust 核心、协议和 Native Host |
| `shared/protocol-schema/` | 跨语言 JSON Schema |
| `docs/` | 架构、协议和开发文档 |
| `aidlc-docs/` | AI-DLC 规划产物 |

详细文件职责和入口见 [`docs/architecture/repository-map.md`](docs/architecture/repository-map.md)。

## 使用与配置

当前版本主要面向开发和受控测试环境。开发环境、运行命令和环境变量见 [`docs/development/setup.md`](docs/development/setup.md)；完整开发状态见 [`docs/development/status.md`](docs/development/status.md)。

Windows 便携构建使用 `npm run build:portable:windows --workspace desktop`。构建输出为可移动目录，不包含安装器；运行时配置写入便携目录的 `config/config.yaml`，应用数据库位于 `config/archive.sqlite3`，临时文件位于 `cache/`，日志位于与 `.exe` 同级的 `logs/`。

## 发布资产（v0.2.0）

v0.2.0 的发布页包含**一个 `.exe`、四个 `.7z` 压缩包和两个校验文件**。所有压缩包统一为 7z 格式。

| 资产 | 格式 | 形态与定位 |
|---|---|---|
| `XArchive-<tag>-windows-x64.exe` | `.exe` | 桌面程序本体，可尝试直接启动；不含 Extension、Native Host 和 Sidecar 组件 |
| `XArchive-<tag>-windows-x64.7z` | `.7z` | 仅包含上面的 `.exe`，解压后得到同一个可尝试启动的桌面程序 |
| `XArchive-<tag>-windows-x64-full.7z` | `.7z` | 包含可尝试启动的桌面 `.exe`，以及 Extension、Native Host、Sidecar 等随附组件；包含组件不等于浏览器集成已自动配置 |
| `XArchive-<tag>-windows-x64-repository-dependencies.7z` | `.7z` | 组件/依赖分发包（本仓库 Extension、worker、manifest、许可证文件）；**不是可直接运行的桌面程序** |
| `XArchive-<tag>-extension.7z` | `.7z` | 浏览器 Extension 组件分发包；**不是可直接运行的桌面程序** |
| `XArchive-<tag>-release-manifest.json` | `.json` | 校验与来源追溯材料 |
| `SHA256SUMS-<tag>.txt` | `.txt` | 校验与来源追溯材料 |

- `.7z` 是分发容器，Edge/Chrome 不能直接导入：使用 Extension 包时需**先解压**，再按受控测试过的方式加载解压得到的扩展目录。
- 上表中“可尝试启动”只描述文件形态，不构成在任意设备上都能启动的保证；实际启动行为以该版本对应的 Windows 验证记录为准。
- 归档成功不承诺：即使桌面程序正常启动，本版本也不保证能完成一次真实 X 帖子的归档。

## 配置

复制 `.env.example` 并按本机环境设置 Sidecar：

```dotenv
XARCHIVE_SIDECAR_PROGRAM=python3
XARCHIVE_SIDECAR_ARGS=["-m","xarchive_downloader"]
```

Sidecar、gallery-dl、aria2 和真实账号运行要求见开发文档；不要把凭据提交到仓库。

## 许可证说明

本项目使用 MIT License。第三方运行时、依赖和可选工具的许可证另见 [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)。正式分发前必须重新核对实际捆绑内容和对应许可证义务。

## 文档

- 用户和项目概览：本文档。
- 文档索引：[`docs/README.md`](docs/README.md)。
- 开发环境与运行：[`docs/development/setup.md`](docs/development/setup.md)。
- 当前开发状态：[`docs/development/status.md`](docs/development/status.md)。
- 架构和文件职责：[`docs/architecture/overview.md`](docs/architecture/overview.md)、[`docs/architecture/repository-map.md`](docs/architecture/repository-map.md)。
