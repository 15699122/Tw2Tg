# 测试策略

## 测试层级

### Unit

- Rust：Job 状态、重试、metadata、路径安全、hash、DownloadRouter、TagEngine、Repository、Telegram formatter、发送状态，以及 Desktop R1 executor 的 submit/query/cancel/shutdown/recovery-scan、active/interrupted candidate、terminal skip、event-ordering、创建/下载开始/下载完成/下载失败/完成 lifecycle event mapping、fake Sidecar crash、事务性状态事件去重、queued/interrupted recovery source state、persisted cancel/shutdown/completion 幂等和状态边界、`DOWNLOADED → COMPLETE`、commit 前后退出 recovery decision、Resume/Complete/MarkFailed/Skip recovery actions、`CommitRecoveryFactsProvider` facts/snapshot 一致性、批量 recovery 稳定排序、混合动作结果、SQLite 状态/事件/错误字段顺序与单 Job 错误隔离、`EXECUTOR_UNAVAILABLE` submit compensation、shutdown interruption 与 worker shutdown 分离、`ArchiveExecutionContext` 的 State-independent resource boundary、`JobExecution` execution port 的成功/失败/terminal skip/lifecycle ordering contract、`ArchiveExecutionJob` identity/download/commit/error mapping boundary、`COMPLETE` 缺失归档的安全边界、JobDatabaseFactory 独立上下文边界、ArchiveJobSubmissionAdapter 的同步入口行为对照及生产 fallback 复用、`RuntimeState` executor ownership、`get_app_status` 生命周期状态、Tauri executor control command 的 submit/query/cancel/shutdown 边界、JobSummary→JobSnapshot 字段投影、错误字段保留和 SQLite Job repository contract model。
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

如果当前环境没有 `cargo-clippy` 或 `pytest`，必须记录为 `NOT RUN`，不能记录为 PASS。本轮最终 Linux 收口中，clippy 和 pytest 均为 `NOT RUN`；fmt/check/test、Node check/test/build、Python compileall 和 JSON Schema parse 已通过。

## 测试维护规则

- 新增功能必须增加与边界对应的测试。
- 业务逻辑测试不应依赖真实账号、用户目录或共享数据库。
- 使用临时目录、fake transport 或本地 server 隔离外部系统。
- 失败测试应记录错误分类和是否阻塞其他验证。
- 测试数量变化不是完成标准；行为覆盖和失败边界才是完成标准。

## Windows 验证

Windows 的执行流程、同步方向、状态定义和报告要求见 [`cross-platform-validation.md`](cross-platform-validation.md) 与 [`../validation/windows.md`](../validation/windows.md)。具体历史结果不写入本策略文档。