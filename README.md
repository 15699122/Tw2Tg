# Tw2Tg / XArchive

X/Twitter 本地归档桌面应用。用户在 Edge/Chrome 的 X 页面点击归档按钮后，由 Tauri/Rust 统一管理任务、SQLite 状态、本地原文件和 Telegram 展示；Python Sidecar 使用 gallery-dl 负责 X metadata 提取与默认媒体下载。

## 当前状态

项目于 **2026-09-08** 按 Greenfield Monorepo 初始化。当前已完成 Sprint 0、Sprint 1 协议链路、M1 本地归档核心、gallery-dl Adapter、媒体文件结果契约、Rust Sidecar 结果转换、ArchiveService 端到端闭环和 aria2 RPC 协议层 Spike；Windows 已验证 Rust/Node/Python 基础测试通过。真实 X 认证下载、aria2c Supervisor、Telegram、Native Messaging 和完整 GUI 尚未实现。
当前已增加 Rust Supervisor 与真实 Python Worker 的本地进程集成测试；Windows 已安装项目本地 gallery-dl 并验证 sidecar 可调用，但真实 X 认证下载仍待具备账号环境后验证。

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

### Windows 验证记录（2026-09-08）

- Node 检查、测试和构建通过；当前两个工作区暂无测试用例。
- `cargo check --workspace` 通过。
- `cargo test --workspace`：26 个测试全部通过。
- Python 在本地 `.venv` 中 editable 安装 Sidecar，并安装 gallery-dl 1.32.11 后，10 个测试全部通过。
- `cargo fmt --check` 通过。
- `cargo clippy --workspace --all-targets -- -D warnings` 曾检出 `SupervisorEvent::Download(DownloadEvent)` 的 `large_enum_variant` 问题，位置为 `crates/xarchive-sidecar-supervisor/src/lib.rs:17`；现已改为 `Download(Box<DownloadEvent>)`，待 Windows 环境重新运行 clippy 复验。
- 真实 sidecar 使用 gallery-dl 1.32.11，在含中文、空格和 Unicode 的路径中完成 `ready → started → log → failed` JSONL 流程；示例 X URL 返回 `EXTRACT_OR_DOWNLOAD_FAILED`，未进行真实账号认证下载。
- Named Pipe、Native Host 注册、浏览器安装和 Tauri 打包尚未执行，因为对应功能尚未实现。

Windows 相关的开发、实机验证、Windows CI、安装器和发布任务统一见 [`docs/development/windows-validation.md`](docs/development/windows-validation.md)。

## 许可证说明

gallery-dl 和 aria2 均涉及 GPL 许可证。正式分发前必须维护 `THIRD_PARTY_NOTICES.md`、许可证副本、版本清单和对应源代码获取方案。闭源或商业发行前应进行法律审查。
