# 开发环境与运行

## 开发平台

Linux 是主要开发环境。Windows 用于 Windows-specific build、runtime、filesystem、process、Native Host、GUI、packaging 和真实账号链路验证。

## 工具链

- Rust stable、Cargo 和 workspace dependencies。
- Node.js、npm 和 workspace dependencies。
- Python 3.10 或更高版本。
- `gallery-dl`，由 Sidecar 运行时提供。
- Tauri CLI 2，用于 Desktop 开发和构建。
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
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --no-fail-fast
python3 -m compileall -q sidecar/src sidecar/tests
python3 -m pytest sidecar/tests -q
```

## 运行关系

开发版 Desktop 由 Tauri 启动 Vite frontend，并通过 `XARCHIVE_SIDECAR_PROGRAM` 和 `XARCHIVE_SIDECAR_ARGS` 启动 Python Sidecar。当前 `tauri.conf.json` 未启用 bundle，不能将 `npm run build:tauri` 视为已生成可分发安装包。

## 数据目录

当前实现默认在进程工作目录下创建 `X-Archive/`，其中包含 SQLite、staging、归档文件和用户 profile。改变 archive root 或权限策略属于独立架构任务；开发和验证不得将真实私人数据写入共享目录。

## Windows 开发与验证

Windows 工作副本必须由 Linux 源目录单向同步，且不能把 Windows 本地配置、凭据、缓存或生成物反向同步到 Linux。完整流程见 [`cross-platform-validation.md`](cross-platform-validation.md)。