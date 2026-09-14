# 测试策略

## 测试层级

### Unit

- Rust：Job 状态、重试、metadata、路径安全、hash、DownloadRouter、TagEngine、Repository、Telegram formatter、发送状态，以及 Desktop R1 executor 的 submit/query/cancel/shutdown/recovery-scan、active/interrupted candidate、terminal skip、event-ordering、execution spec persistence、runner-owned ExecutorConfig/Database/FileStore/Sidecar context、attempt fencing、运行中 cancellation、late-result fencing、创建/下载开始/下载完成/下载失败/完成 lifecycle event mapping、fake Sidecar crash、事务性状态事件去重、queued/interrupted recovery source state、persisted cancel/shutdown/completion 幂等和状态边界、`DOWNLOADED → COMPLETE`、commit recovery decision/action、`CommitRecoveryFactsProvider` facts/snapshot 一致性、批量 mixed recovery、SQLite 状态/事件/错误字段顺序与单 Job 错误隔离、`EXECUTOR_UNAVAILABLE` compensation、shutdown interruption 与 worker shutdown 分离、独立 SQLite context、State-independent `ArchiveExecutionContext`、真实 `ArchiveExecutionJob` identity/download/commit/error mapping、runner spec missing failure、control worker 与 single active runner 分离、production Tauri submit wiring、同步 fallback 对照、`RuntimeState` ownership、`get_app_status` 生命周期状态、Tauri executor commands、JobSummary→JobSnapshot 字段投影、错误字段保留和 SQLite Job repository contract model。
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
.venv/bin/python -m compileall -q sidecar/src sidecar/tests
.venv/bin/python -m pytest sidecar/tests -q
```

如果当前环境没有 `cargo-clippy`，先执行 `rustup component add clippy`；如果没有 `.venv` 或 pytest，先执行 `python3 -m venv .venv` 和 `.venv/bin/python -m pip install -e sidecar pytest`。Sidecar 测试统一使用 `.venv/bin/python -m pytest`，不要依赖系统级 `pytest`。

## 测试维护规则

- 新增功能必须增加与边界对应的测试。
- 业务逻辑测试不应依赖真实账号、用户目录或共享数据库。
- 使用临时目录、fake transport 或本地 server 隔离外部系统。
- 失败测试应记录错误分类和是否阻塞其他验证。
- 测试数量变化不是完成标准；行为覆盖和失败边界才是完成标准。

## Windows 验证

Windows 的执行流程、同步方向、状态定义和报告要求见 [`cross-platform-validation.md`](cross-platform-validation.md) 与 [`../validation/windows.md`](../validation/windows.md)。具体历史结果不写入本策略文档。