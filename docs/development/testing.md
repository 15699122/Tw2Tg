# 测试策略

## 测试层级

### Unit

- Rust：Job 状态、重试、metadata、路径安全、hash、DownloadRouter、TagEngine、Repository、Telegram formatter 和发送状态。
- Python：JSONL worker、gallery-dl command、metadata 归一化和错误映射。
- JavaScript：DOM 提取、按钮去重、状态映射、Native Bridge、request_id 路由和断线处理。

### Contract

使用 `shared/protocol-schema/fixtures/` 验证 Rust、Python 和 JavaScript 对相同消息的兼容性。修改 Schema 时必须同步更新模型、fixture、producer、consumer 和测试。

### Integration

覆盖 Rust 与 Fake/Real Sidecar、临时 SQLite、临时 FileStore、Native Host fake transport、aria2 fake HTTP server、Telegram fake HTTPS server、Desktop commands 和 Tauri 构建。

### Platform

Windows 专属验证包括 Named Pipe、Registry、Edge/Chrome Native Host、WebView2、长路径、ACL、externalBin、installer、Credential Manager 和真实账号链路。Linux 测试不能替代这些结论。

## Linux 命令

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run test
npm run build
python3 -m compileall -q sidecar/src sidecar/tests
python3 -m pytest sidecar/tests -q
```

如果当前环境没有 `cargo-clippy` 或 `pytest`，必须记录为 `NOT RUN`，不能记录为 PASS。

## 测试维护规则

- 新增功能必须增加与边界对应的测试。
- 业务逻辑测试不应依赖真实账号、用户目录或共享数据库。
- 使用临时目录、fake transport 或本地 server 隔离外部系统。
- 失败测试应记录错误分类和是否阻塞其他验证。
- 测试数量变化不是完成标准；行为覆盖和失败边界才是完成标准。

## Windows 验证

Windows 的执行流程、同步方向、状态定义和报告要求见 [`cross-platform-validation.md`](cross-platform-validation.md) 与 [`../validation/windows.md`](../validation/windows.md)。具体历史结果不写入本策略文档。