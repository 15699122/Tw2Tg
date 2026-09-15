# 测试策略

## 测试层级

### Unit

- Rust：Job 状态、重试、metadata、路径安全、hash、DownloadRouter、TagEngine、Repository、Telegram formatter、发送状态，以及 Desktop R1 executor 的 submit/query/cancel/shutdown/recovery-scan、active/interrupted candidate、terminal skip、event-ordering、execution spec persistence、runner-owned ExecutorConfig/Database/FileStore/Sidecar context、attempt fencing、运行中 cancellation、late-result fencing、创建/下载开始/下载完成/下载失败/完成 lifecycle event mapping、fake Sidecar crash、事务性状态事件去重、queued/interrupted recovery source state、persisted cancel/shutdown/completion 幂等和状态边界、`DOWNLOADED → COMPLETE`、commit recovery decision/action、`CommitRecoveryFactsProvider` facts/snapshot 一致性、批量 mixed recovery、SQLite 状态/事件/错误字段顺序与单 Job 错误隔离、`EXECUTOR_UNAVAILABLE` compensation、shutdown interruption 与 worker shutdown 分离、独立 SQLite context、State-independent `ArchiveExecutionContext`、真实 `ArchiveExecutionJob` identity/download/commit/error mapping、runner spec missing failure、control worker 与 single active runner 分离、production Tauri submit wiring、同步 fallback 对照、`RuntimeState` ownership、`get_app_status` 生命周期状态、Tauri executor commands、JobSummary→JobSnapshot 字段投影、错误字段保留和 SQLite Job repository contract model、Browser transport adapter 的协议校验、request_id 路由、重复提交、状态查询和错误映射。
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
npm run test:e2e:windows --workspace desktop
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
- WebdriverIO：desktop/e2e/specs/ 的 Tauri 原生窗口 smoke；验证真实窗口 DOM、可见性和稳定区域，不替代 Native Host、真实账号或应用级 IPC。
- 最新 Windows 进一步验证：专用 artifact 的 wdioTauri 和 browser.tauri.execute probe 通过，mock/日志子项有通过证据；Linux follow-up 已移除 Windows `.cmd` 直接 spawn，并将 availability 断言改为 `window.wdioTauri` 检查。普通构建的 guest JS/ACL 边界和 service teardown 仍需 Windows 重验，详见 windows-validation.md。
- tauri-plugin-wdio 高级路径已完成 Linux 配置；使用 `wdio-e2e` feature、独立 capability 和 `wdio-plugin.e2e.mjs` 验证 `browser.tauri.execute`、mocking 与 cleanup。真实 Windows WebView2、日志转发和窗口生命周期仍作为独立 Windows 队列项验证。

## Linux 端当前 WDIO follow-up

Windows 复验后，Linux 端的自动化工作按以下顺序处理：

1. 已修正 wrapper 对 Windows `.cmd` 的调用和退出码传播；需用 node --check、WDIO 配置加载/dry-run 做无 GUI 检查。
2. 已将 plugin availability 断言统一为通过 `browser.tauri.execute` 检查 `window.wdioTauri`；不得继续使用 `browser.tauri.isTauriApiAvailable`。
3. 已将 `@wdio/tauri-plugin` 的 guest JS 加载与 `VITE_WDIO_E2E=1` 专用构建边界对齐；仍需 Windows 验证普通 release 不触发 WDIO ACL 命令，专用 artifact 仍可 execute/mock/log。
4. 在当前 lockfile 下继续核对 service teardown 的 sessionId、mock store 和 driver 生命周期；该项需要 Windows native session 结果，暂不以手工杀进程替代修复。
5. 当前 revision 的 `npm run check`、`npm run test`、`npm run build`、wrapper syntax check 及 Rust fmt/check/test/clippy 已通过。Linux native WebView/WDIO 仍是 NOT RUN，不因 Linux 门禁通过而改为 Windows PASS。
