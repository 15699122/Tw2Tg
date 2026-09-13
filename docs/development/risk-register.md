# 风险登记

本文只保留当前仍有效的风险。已关闭的 lint、一次性环境故障和历史验证过程保存在 Windows 历史验证文档，不在此重复维护。

| ID | 风险 | 等级 | 状态 | 责任模块 | 缓解措施 | 验证入口 |
|---|---|---:|---|---|---|---|
| RISK-001 | X DOM 结构变化导致 Tweet/quote/reply 提取失效 | P0 | OPEN | `extension/src/content-core.js`、`extension/src/content.js` | 集中 selector、保留 Sidecar metadata、DOM fixture 和失败降级 | Extension tests、真实 Edge 验证 |
| RISK-002 | gallery-dl 认证或 extractor 行为变化 | P0 | OPEN | `sidecar/src/xarchive_downloader/gallery.py`、`models.py` | 固定运行时版本、Adapter 隔离、稳定错误码、保留 extractor metadata | Sidecar tests、WQ-P0-02 |
| RISK-003 | Edge Cookie 或真实 X 认证不可用 | P0 | OPEN | Sidecar、Edge Profile、Desktop archive flow | AUTH_REQUIRED 映射、受控测试账号、Cookie 不进入日志和协议 | WQ-P0-02 |
| RISK-004 | Sidecar/metadata identity confusion | P0 | MITIGATED | Protocol、Storage、Desktop | URL status ID、请求 Tweet ID、Sidecar metadata Tweet ID 三方绑定；保留 mismatch tests | WQ-P1-12 |
| RISK-005 | Token/Cookie/RPC secret 泄露 | P0 | OPEN | SecretStore、Sidecar、Desktop、Extension | SecretStore abstraction、错误脱敏、settings allowlist、端到端日志审查 | WQ-P1-04、WQ-P1-12 |
| RISK-006 | staging symlink/junction/reparse escape | P1 | MITIGATED | `xarchive-storage` FileStore | `symlink_metadata`、reparse rejection、相对路径校验；自动化 Windows harness 仍待补齐 | WQ-P1-12、WQ-P2-02 |
| RISK-007 | 文件与 SQLite 状态不一致 | P0 | OPEN | ArchiveService、Job executor、Storage | staging-then-commit、事务、事件历史、应用级恢复测试 | WQ-P0-03、R5 |
| RISK-008 | Desktop 全局锁覆盖长时间 Sidecar I/O | P1 | OPEN | `desktop/src-tauri/src/lib.rs` | 设计后台 Job executor、短事务和取消 channel；不得在未有测试前直接重构 | R1、WQ-P0-01 |
| RISK-009 | DownloadRouter/aria2 fallback 与 Job 状态脱节 | P1 | OPEN | `xarchive-download`、Desktop archive flow | gallery-dl 默认、只使用新鲜 URL、记录双侧失败、补充应用级 fixture | R2、WQ-P1-01 |
| RISK-010 | Named Pipe/Registry/Native Host 平台边界未完成 | P1 | OPEN | `xarchive-native-host`、Desktop IPC、Installer | 保持 framing/forwarding 与 Windows endpoint 分离，先完成 ACL 设计 | R3、WQ-P0-04、WQ-P1-02 |
| RISK-011 | archive root 位于当前工作目录导致隐私和可恢复性问题 | P1 | OPEN | Desktop RuntimeState、Storage FileStore | 明确应用数据目录与用户归档目录职责，增加重启和 ACL 场景 | R5、WQ-P1-13 |
| RISK-012 | 第三方运行时许可证和实际捆绑内容不一致 | P1 | OPEN | Packaging、Release docs | 维护 LICENSE/THIRD_PARTY_NOTICES、锁定版本、SBOM 和发布审查 | R6、WQ-P2-01 |
| RISK-013 | Linux 验证工具缺失导致验证结论不完整 | P2 | OPEN | Development environment | setup 文档说明 `cargo-clippy`/`pytest` 前置；缺失时明确记录 `NOT RUN` | `docs/development/setup.md` |

## 状态说明

- `OPEN`：当前仍需要开发、环境或验证工作。
- `MITIGATED`：已有代码或测试降低风险，但仍可能需要平台专项验证。
- `CLOSED`：只有在当前代码、文档和验证证据均不再需要后续动作时使用。