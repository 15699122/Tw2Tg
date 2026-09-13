# Tw2Tg / XArchive

XArchive 是一个本地优先的 X/Twitter 归档桌面应用。用户可以从浏览器中的 X 页面发起归档，由桌面应用保存 Tweet metadata、原始媒体、用户资料和任务状态，并可将内容发送到 Telegram。

## 功能

- 从 X/Twitter 页面发起单条 Tweet 归档。
- 保存 Tweet metadata、`tweet.json`、`tweet.txt` 和原始媒体文件。
- 使用 SQLite 保存任务、用户、标签、媒体和事件状态。
- 通过 gallery-dl 处理 X metadata 提取和默认下载。
- 记录回复、引用 Tweet、用户名称历史和用户 profile 文件。
- 提供 Telegram Bot API 请求模型、格式化、媒体分组和幂等发送基础能力。
- 提供可选的 aria2 下载传输和 Windows aria2 管理入口。
- 提供浏览器 Extension、Native Messaging 协议和 Tauri Desktop Dashboard。

当前项目仍处于开发阶段。Windows 安装器、Native Host 注册、Named Pipe 服务、真实 X 账号链路、Credential Manager、正式 externalBin 打包和真实 Telegram 账号发送不应视为已发布功能。

## 技术栈

- Tauri 2 + Rust：桌面应用、业务状态、任务编排、SQLite 和文件提交。
- React + Vite：桌面 Dashboard。
- Manifest V3：Edge/Chrome 浏览器扩展。
- Python + gallery-dl：X metadata 提取和默认媒体下载 Sidecar。
- SQLite：本地任务、metadata、用户、标签、事件和发送状态。
- JSON/JSONL + JSON Schema：跨进程协议。
- Telegram Bot API：可选的人类可读展示层。

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
