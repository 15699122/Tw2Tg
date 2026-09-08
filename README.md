# Tw2Tg / XArchive

X/Twitter 本地归档桌面应用。用户在 Edge/Chrome 的 X 页面点击归档按钮后，由 Tauri/Rust 统一管理任务、SQLite 状态、本地原文件和 Telegram 展示；Python Sidecar 使用 gallery-dl 负责 X metadata 提取与默认媒体下载。

## 当前状态

项目于 **2026-09-08** 按 Greenfield Monorepo 初始化。当前已完成 Sprint 0、Sprint 1 协议链路和 M1 本地归档核心；真实 gallery-dl 下载、Telegram、Native Messaging 和完整 GUI 尚未实现。

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

## 目录

| 目录 | 用途 |
|---|---|
| `desktop/` | Tauri Desktop 与 React 前端 |
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
```

## 开发顺序

1. Sprint 0：工程骨架和工具链。
2. Sprint 1：共享协议与 Fake Sidecar。
3. Sprint 2：真实 gallery-dl 下载。
4. M1：SQLite、本地文件和幂等恢复。
5. M1.5：aria2 技术验证。
6. M2：Telegram。
7. M3：MV3 Extension、Native Host 和 Named Pipe。

详见 [`docs/development/roadmap.md`](docs/development/roadmap.md)。

## 平台说明

当前目标平台为 Windows。Linux 可用于协议、Rust、Python 和前端开发；Named Pipe、Native Host Registry、Edge/Chrome 安装和 Tauri 打包必须在 Windows 或 Windows CI 上验证。

## 许可证说明

gallery-dl 和 aria2 均涉及 GPL 许可证。正式分发前必须维护 `THIRD_PARTY_NOTICES.md`、许可证副本、版本清单和对应源代码获取方案。闭源或商业发行前应进行法律审查。