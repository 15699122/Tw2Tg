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
| RISK-008 | Desktop 全局锁覆盖长时间 Sidecar I/O | P1 | OPEN | `desktop/src-tauri/src/runtime.rs`、`archive.rs`、ADR-009 | ADR-009 已完成设计；下一步实现短事务、取消 channel 和 executor 测试，未有测试前不得替换同步实现 | R1、WQ-P0-01 |
| RISK-009 | DownloadRouter/aria2 fallback 与 Job 状态脱节 | P1 | OPEN | `xarchive-download`、Desktop archive flow | gallery-dl 默认、只使用新鲜 URL、记录双侧失败、补充应用级 fixture | R2、WQ-P1-01 |
| RISK-010 | Named Pipe/Registry/Native Host 平台边界未完成 | P1 | OPEN | `xarchive-native-host`、Desktop IPC、Installer | 保持 framing/forwarding 与 Windows endpoint 分离，先完成 ACL 设计 | R3、WQ-P0-04、WQ-P1-02 |
| RISK-011 | archive root 位于当前工作目录导致隐私和可恢复性问题 | P1 | OPEN | Desktop RuntimeState、Storage FileStore | 明确应用数据目录与用户归档目录职责，增加重启和 ACL 场景 | R5、WQ-P1-13 |
| RISK-012 | 第三方运行时许可证和实际捆绑内容不一致 | P1 | OPEN | Packaging、Release docs | 维护 LICENSE/THIRD_PARTY_NOTICES、锁定版本、SBOM 和发布审查 | R6、WQ-P2-01 |
| RISK-013 | Linux 验证工具缺失导致验证结论不完整 | P2 | OPEN | Development environment | setup 文档说明 `cargo-clippy`/`pytest` 前置；缺失时明确记录 `NOT RUN` | `docs/development/setup.md` |
| RISK-014 | Sidecar v1 到 v2 迁移造成 Rust/Python/Schema 不一致 | P0 | OPEN | `crates/xarchive-protocol`、Sidecar、Supervisor、Desktop | v1 rejection、unknown field/capability tests、Rust/Python/Schema round-trip；U3 前不得切换单侧 producer/consumer | U3、protocol fixtures |
| RISK-015 | extraction-only 后媒体 credential/header 转发不足 | P0 | OPEN | Sidecar extraction、aria2 transfer、SecretStore | 结构化 header allowlist；拒绝 Cookie/Authorization/CRLF；不支持的凭据需求明确失败，不回退 gallery-dl 下载 | U4、U5、WQ-SIDECAR-CANCEL-03 |
| RISK-016 | aria2-only transfer 与 staging commit 状态不一致 | P0 | OPEN | `xarchive-download`、ArchiveService、Job executor | fake RPC/media server、GID 状态映射、文件 identity/size/hash 验证和 commit fencing | U5、U7、WQ-P1-01 |
| RISK-017 | signed URL/header 泄露到 SQLite、日志或错误 | P0 | OPEN | ExtractionResult、MediaTransferPlan、Storage、logging | durable metadata 与 transfer data 分离；URL/header redaction tests；禁止写入 Job spec、`tweet.json` 和普通日志 | U4–U7、security scan |
| RISK-018 | Component Manifest hash/version/catalog 不一致 | P1 | OPEN | ComponentManager、release pipeline、Desktop embedded catalog | 固定 catalog、SHA/size/probe 校验、embedded/external parity test、防降级 | U9、U11、U13 |
| RISK-019 | Core Bootstrap 缺组件启动或安装失败恢复不完整 | P1 | OPEN | ComponentManager、Setup Wizard、portable runtime | 无组件仍可启动设置页；`.part`、atomic activation、safe extract、rollback、断网和 hash mismatch tests | U9–U10、WQ-COMPONENT-01 |
| RISK-020 | Extension 文件就绪被错误显示为浏览器已连接 | P1 | OPEN | Desktop Extension status、Extension、Native Host | 明确区分 `MISSING`、`FILES_READY`、`BROWSER_NOT_LOADED`、`NATIVE_HOST_NOT_REGISTERED`、`DISCONNECTED`、`CONNECTED` | U12、GUI-W-EXT-10 |
| RISK-021 | Windows Job Object/process-tree cleanup 未验证 | P0 | OPEN | Sidecar supervisor、Worker、aria2 supervisor | Linux 已完成 Unix process-group contract；Windows Job Object/taskkill 行为、grace/force、孙进程、文件锁和残留进程仍需实机验证 | U2、U5、WQ-SIDECAR-CANCEL-01 |
| RISK-022 | 旧 `archive_tweet`、`DownloadRouter` 或 Sidecar v1 runtime 残留 | P0 | OPEN | Desktop archive、protocol、download crate、Schema | U8 前禁止声明目标终态；全仓库搜索、runtime smoke 和 dead-code review 后再关闭 | U8、U14 |
| RISK-023 | Extension WebSocket listener 未认证或 MV3 worker 重建后状态丢失 | P0 | OPEN | ADR-014、Desktop WebSocket transport、Extension bridge | loopback 监听、一次性认证、凭据轮换、pending 清理、端口发现、有限退避和代际 fencing；Native Messaging 迁移期回退；安全/重连/GUI/Windows 验证 | ADR-014、Extension tests、Rust listener tests、WQ WebSocket 队列 |
| RISK-024 | Extension 从未解析的 URL 字符串推断 Tweet 身份 | P0 | MITIGATED | `extension/src/content-core.js` | 先 `new URL` 解析再对 `hostname` 做 `x.com`/`twitter.com` 精确允许列表；拒绝 userinfo、端口、非 https 与非本机 base；ID 只从 `pathname` 提取；与 `xarchive-protocol::extract_tweet_id` 的 authority 规则对齐；CodeQL `js/incomplete-url-substring-sanitization` 回归测试 | Extension tests、CodeQL 扫描、WQ-WS-02 |

## 状态说明

- `OPEN`：当前仍需要开发、环境或验证工作。
- `MITIGATED`：已有代码或测试降低风险，但仍可能需要平台专项验证。
- `CLOSED`：只有在当前代码、文档和验证证据均不再需要后续动作时使用。