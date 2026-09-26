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
| RISK-014 | 打包输出目录可被配置为项目根导致递归删除 | P0 | MITIGATED | `desktop/scripts/portable-package.mjs`、`build-portable-windows.mjs` | `validatePortableOutputDir` 在任何构建/删除前拒绝文件系统根、项目根、其祖先、家目录及命名空间外路径；输入组件检查先于删除 | Linux 12/12 负向测试 + `PORTABLE_OUTPUT_DIR=.` 实测拒绝且项目完好；junction 场景见 WQ-ENG-01 |
| RISK-015 | 当前分支 Windows 条件编译失败 | P0 | MITIGATED | `desktop/src-tauri/src/transport.rs` | `PathBuf` 无条件导入，仅 `Path` 保留 `#[cfg(unix)]` | Linux `cargo check`/80 tests 通过；Windows `cargo check --workspace --locked` 见 WQ-ENG-02 |
| RISK-016 | 发布标签与实际构建源码未绑定、可能复用旧二进制 | P1 | OPEN | `.github/workflows/windows-release.yml`、`build-portable-windows.mjs` | checkout 精确 tag 并核对 SHA；发布默认强制构建 | 发布演练、provenance 记录 |
| RISK-017 | 依赖审计命中 RUSTSEC-2026-0285 与 Node 测试链公告 | P1 | OPEN | `Cargo.lock`、`package-lock.json` | 单独升级 rustls 到修复版本；追踪 Node 依赖链后再升级 | `cargo audit`、`npm audit` 复跑 |
| RISK-018 | 归档目录中间 symlink/junction 未逐层约束 | P1 | OPEN | `xarchive-storage` FileStore/metadata | 约束根目录权限，逐层链接检查 | 父目录链接与竞态测试、Windows reparse 验证 |
| RISK-019 | IPC 与 Sidecar 输出缺少资源上限 | P1 | MITIGATED | `xarchive-sidecar-supervisor/readers.rs`、`sidecar/gallery.py`、`desktop/src-tauri/src/transport.rs` | Sidecar 单行上限 1 MiB 且分块扫描不先分配整行；gallery-dl 改用临时文件并有界保留尾部；桌面日志单行 16 KiB、单文件 8 MiB 轮转 | 9 项 supervisor 测试、3 项日志测试、19 项 sidecar 测试；IPC 连接上限仍待 WQ-ENG-04 |
| RISK-020 | 外部工具错误透传可能残留本地路径或凭据 | P1 | MITIGATED | `sidecar/errors.py`、`gallery.py`、`storage/database/jobs.rs` | 协议边界统一 `sanitize_error_text`：Authorization、Bearer、Telegram/GitHub/Slack token、URL 凭据、query secret、cookie 与绝对本地路径；限长 2000 字符并标记截断 | 7 项脱敏测试（token/header/URL 凭据/路径/长度/分类）；真实账号错误内容见 WQ-ENG-06 |
| RISK-021 | 生产持久化使用固定测试时钟 | P1 | MITIGATED | `desktop/src-tauri/src/clock.rs`、`executor.rs`、`transport.rs` | 新增无依赖 `clock` 模块输出真实 UTC `YYYY-MM-DDTHH:MM:SSZ`（civil-from-days，支持纪元前）；executor 7 处与 transport 1 处改用真实时钟 | 6 项 clock 测试；任务 ID 实测为真实时间；Desktop Rust 89/89 |
| RISK-022 | 日志无大小上限且读取全量加载 | P2 | MITIGATED | `desktop/src-tauri/src/logging.rs` | 单行 16 KiB 截断、单文件 8 MiB 轮转、换行折叠防注入、`read_recent` 仅读尾部 512 KiB | 3 项日志测试：超长行截断、换行折叠、大文件有界读取 |

## 状态说明

- `OPEN`：当前仍需要开发、环境或验证工作。
- `MITIGATED`：已有代码或测试降低风险，但仍可能需要平台专项验证。
- `CLOSED`：只有在当前代码、文档和验证证据均不再需要后续动作时使用。