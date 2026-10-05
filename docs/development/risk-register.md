# 风险登记

本文只保留当前仍有效的风险。已关闭的 lint、一次性环境故障和历史验证过程保存在 Windows 历史验证文档，不在此重复维护。

| ID | 风险 | 等级 | 状态 | 责任模块 | 缓解措施 | 验证入口 |
|---|---|---:|---|---|---|---|
| RISK-001 | X DOM 结构变化导致 Tweet/quote/reply 提取失效 | P0 | OPEN | `extension/src/content-core.js`、`extension/src/content.js` | 集中 selector、保留 Sidecar metadata、DOM fixture 和失败降级 | Extension tests、真实 Edge 验证 |
| RISK-002 | gallery-dl 认证或 extractor 行为变化 | P0 | OPEN | `sidecar/src/xarchive_downloader/gallery.py`、`models.py` | 固定运行时版本、Adapter 隔离、稳定错误码、保留 extractor metadata | Sidecar tests、WQ-P0-02 |
| RISK-003 | Edge Cookie 或真实 X 认证不可用 | P0 | OPEN | Sidecar、Edge Profile、Desktop archive flow | AUTH_REQUIRED 映射、受控测试账号、Cookie 不进入日志和协议 | WQ-P0-02 |
| RISK-004 | Sidecar/metadata identity confusion | P0 | MITIGATED | Protocol、Storage、Desktop | URL status ID、请求 Tweet ID、Sidecar metadata Tweet ID 三方绑定；保留 mismatch tests | WQ-P1-12 |
| RISK-005 | Token/Cookie/RPC secret 泄露 | P0 | OPEN | SecretStore、Sidecar、Desktop、Extension、`xarchive-download` supervisor | SecretStore abstraction、错误脱敏、settings allowlist、端到端日志审查；ENG-13 已消除 aria2 RPC secret 的 argv 暴露子向量（改走 owner-only 短期 `--conf-path` 文件），Token/Cookie 持久化与真实账号边界仍待验证 | WQ-P1-04、WQ-P1-12、WQ-ENG-12 |
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
| RISK-014 | 打包输出目录可被配置为项目根导致递归删除 | P0 | MITIGATED | `desktop/scripts/portable-package.mjs`、`build-portable-windows.mjs` | `validatePortableOutputDir` 在任何构建/删除前拒绝文件系统根、项目根、其祖先、家目录及命名空间外路径；输入组件检查先于删除 | Linux 12/12 负向测试 + `PORTABLE_OUTPUT_DIR=.` 实测拒绝且项目完好；junction 场景见 WQ-ENG-01 |
| RISK-015 | 当前分支 Windows 条件编译失败 | P0 | MITIGATED | `desktop/src-tauri/src/transport.rs` | `PathBuf` 无条件导入，仅 `Path` 保留 `#[cfg(unix)]` | Linux `cargo check`/80 tests 通过；Windows `cargo check --workspace --locked` 见 WQ-ENG-02 |
| RISK-016 | 发布标签与实际构建源码未绑定、可能复用旧二进制 | P1 | MITIGATED | `.github/workflows/windows-release.yml`、`build-portable-windows.mjs` | workflow 以 `RELEASE_TAG` checkout 并校验 HEAD 与 tag commit 一致，不一致即失败；产物记录 commit 与 SHA-256；打包默认强制构建，复用需 `PORTABLE_ALLOW_BINARY_REUSE=1` | 两个 workflow YAML 解析通过；发布演练见 WQ-ENG-07 |
| RISK-017 | 依赖审计命中 RUSTSEC-2026-0285 与 Node 测试链公告 | P1 | MITIGATED | `Cargo.lock`、`package-lock.json` | Rust 侧：`cargo update -p rustls --precise 0.23.45`（仅锁文件变更，`Cargo.toml` 未动，`hyper-rustls` 无需连带升级），`cargo audit` 漏洞数归零。Node 侧待 ENG-11 决策 | `cargo audit` 0 漏洞；workspace 182/182；telegram+download TLS 路径 28/28；`cargo fmt --check`、严格 Clippy。Node 侧见 RISK-023 |
| RISK-018 | 归档目录中间 symlink/junction 未逐层约束 | P1 | MITIGATED | `xarchive-storage` FileStore | 新增 `resolve_within` 逐段 `symlink_metadata` 校验，任一中间或末级组件为 reparse point 即返回 `InvalidPath`；缺失组件仍可创建 | storage 29/29（4 项新增：父级/绝对路径、缺失中间目录、中间 symlink 逃逸、末级 symlink 覆盖）；旁路对照下 2 项失败；Windows junction 见 WQ-ENG-03 |
| RISK-019 | IPC 与 Sidecar 输出缺少资源上限 | P1 | MITIGATED | `xarchive-sidecar-supervisor/readers.rs`、`sidecar/gallery.py`、`desktop/src-tauri/src/transport.rs` | Sidecar 单行上限 1 MiB 且分块扫描不先分配整行；gallery-dl 改用临时文件并有界保留尾部；桌面日志单行 16 KiB、单文件 8 MiB 轮转；Unix transport 并发上限 64 且每连接 15 秒读写期限 | 9 项 supervisor、3 项日志、19 项 sidecar、2 项 transport 测试；Windows Named Pipe 行为见 WQ-ENG-04 |
| RISK-020 | 外部工具错误透传可能残留本地路径或凭据 | P1 | MITIGATED | `sidecar/errors.py`、`gallery.py`、`storage/database/jobs.rs` | 协议边界统一 `sanitize_error_text`：Authorization、Bearer、Telegram/GitHub/Slack token、URL 凭据、query secret、cookie 与绝对本地路径；限长 2000 字符并标记截断 | 7 项脱敏测试（token/header/URL 凭据/路径/长度/分类）；真实账号错误内容见 WQ-ENG-06 |
| RISK-021 | 生产持久化使用固定测试时钟 | P1 | MITIGATED | `desktop/src-tauri/src/clock.rs`、`executor.rs`、`transport.rs` | 新增无依赖 `clock` 模块输出真实 UTC `YYYY-MM-DDTHH:MM:SSZ`（civil-from-days，支持纪元前）；executor 7 处与 transport 1 处改用真实时钟 | 6 项 clock 测试；任务 ID 实测为真实时间；Desktop Rust 89/89 |
| RISK-022 | 日志无大小上限且读取全量加载 | P2 | MITIGATED | `desktop/src-tauri/src/logging.rs` | 单行 16 KiB 截断、单文件 8 MiB 轮转、换行折叠防注入、`read_recent` 仅读尾部 512 KiB | 3 项日志测试：超长行截断、换行折叠、大文件有界读取 |
| RISK-023 | Node 测试工具链传递依赖命中公告 | P1 | MITIGATED | `package.json` overrides、`package-lock.json` | 三条真实公告中两条已消除：`serialize-javascript 6.0.2→7.1.2`（Mocha 侧）、内嵌 `deepmerge-ts 7.1.6` 提升为 hoist 的 `8.0.2`。均为跨主版本 override，经 19 组差分用例验证行为与原版本一致。`extract-zip 2.0.1` **无修复版本**，保留为已接受风险 | `npm audit` 16→13 且仅剩 `extract-zip` 一条真实公告；`npm ls` 无 invalid；差分用例 19/19 一致；`npm run check/test/build`、WDIO ConfigParser、Extension 13/13、pytest 19/19、workspace Rust 8 crates。Windows 见 WQ-ENG-09 |
| RISK-024 | Executor 模块拆分引入行为漂移 | P2 | MITIGATED | `desktop/src-tauri/src/executor/*` | 按变化原因拆为 model/persistence/service/runtime/tests，`mod.rs` 仅做组合并以扁平 `pub use` 保持 `crate::executor::*`；行为不变性由 token 级比对证明（唯一差异为有意删去的冗余 `mod tests { }` 包装），原始行零丢失，50/50 测试保留；拆分中两处切片导致的 derive 丢失已修复并恢复 | `cargo test --workspace --locked` 187/187；`cargo fmt --check`；严格 Clippy；`cargo check --all-targets`。Windows MSVC 见 WQ-ENG-11 |
| RISK-025 | 构建工具链未固定导致构建不可复现 | P2 | MITIGATED | `rust-toolchain.toml`、`.github/workflows/*.yml` | 新增 `rust-toolchain.toml` 固定 Rust 1.98.0（等于当前 stable，避免静默降级）并声明 rustfmt/clippy；5 个 Action 改为按 commit SHA 固定，workflow 内不再有浮动 tag；`cargo --locked` 与 `pyinstaller==6.22.3` 此前已锁定 | pinned toolchain 下 `cargo check --locked --all-targets`、workspace 187/187、严格 Clippy、`cargo fmt --check`；workflow YAML 解析。Windows/CI 解析见 WQ-ENG-10 |
| RISK-026 | GitHub Security 未关闭告警（`ip-address`、`brace-expansion`、`glib`、worker 工作流权限） | P1 | MITIGATED | `package-lock.json`、`Cargo.lock`、`.github/workflows/windows-worker-artifact.yml` | 计划见 [`security-remediation-plan.md`](security-remediation-plan.md)。`ip-address` 10.7.0→10.7.2、`brace-expansion` 2.1.4→2.1.7 与 1.1.18→1.1.21 仅锁文件变更即消除（批次 A，`ea465fa`）；`glib` 经 `cargo tree --locked --target x86_64-pc-windows-msvc -i glib` 证明为 Linux-only，Windows 不受影响；C1 兼容升级经 crates.io API 核验**无可用路径**（最新 `wry 0.57.0` 仍为 `gtk ^0.18` + `webkit2gtk =2.0.2`），未改 Cargo 依赖，告警保持未解决，待上游 GTK4/WebKit6 迁移；worker 工作流权限已由 Windows Platform Owner 补齐 `permissions: contents: read` 与 `persist-credentials: false`（`951453c`），Windows run `36700506149` SUCCESS，默认分支重扫后 Code scanning #2 已 `fixed`（`dismissed_by` null，非人工豁免） | `npm ci` 后 `npm ls` 无漏洞版本、`npm audit` 仅剩 `extract-zip` 链、desktop 154/154、extension 33/33、`npm run check` 与 `npm run build`、`cargo test --workspace --locked` 262/262、`pytest sidecar/tests` 46/46、严格 Clippy；`cargo audit --no-fetch` 的 `warnings.unsound` 仍含 `RUSTSEC-2024-0429`；默认分支 Code scanning 开放告警 0，Dependabot 仅余 #1/#4/#6 |
| RISK-027 | Sidecar stdin 命令队列在长任务期间无界增长 | P1 | MITIGATED | `sidecar/src/xarchive_downloader/worker_v2.py` | 容量 32 的有界队列在输入端实施背压；EOF drain 时停止 reader；协议事件/schema 不变。Linux 回归覆盖大于队列容量的 metadata 积压及 shutdown；Windows stdin/进程退出语义待 WQ-SEC-SIDECAR-QUEUE-01 验证 | `pytest sidecar/tests/test_protocol_v2.py sidecar/tests/test_discovery.py`；`pytest sidecar/tests`；Windows queue/shutdown smoke |

## 状态说明

- `OPEN`：当前仍需要开发、环境或验证工作。
- `MITIGATED`：已有代码或测试降低风险，但仍可能需要平台专项验证。
- `CLOSED`：只有在当前代码、文档和验证证据均不再需要后续动作时使用。

## 风险接受：extract-zip（GHSA-jmr9-qjv8-65gv、GHSA-7pqw-9j4j-h8q3）

- **公告范围**：`extract-zip` 为 `*`，最新发布版本 2.0.1 仍受影响，**上游没有修复版本**。
- **依赖链**：`@wdio/utils` → `@puppeteer/browsers 2.13.2` → `extract-zip 2.0.1`，仅存在于测试工具链。
- **触发条件**：仅在 WDIO 自动下载并解压浏览器时执行；本项目 `desktop/wdio.conf.mjs` 在 Windows 设置 `autoDownloadEdgeDriver: true`，因此该路径**可达**，不得按“不可达”结案。
- **不采用的方案**：不将其它 ZIP 库 override 伪装为 `extract-zip`（API 与安全语义未经证明）；不强推 `@puppeteer/browsers` 3.x（不满足 `@wdio/utils` 声明的 `^2.2.0`，且属破坏性变更）。
- **接受理由**：位于开发期测试依赖，不进入发布产物；实际暴露取决于 EdgeDriver 取得方式。
- **复核触发条件**：`@wdio/utils` 采用 `@puppeteer/browsers` 3.x，或上游 `extract-zip` 发布修复版本时，立即重新评估。
- **2026-09-30 复核（两次）**：
  - Dependabot #4 与 #6 为同一依赖的两条独立公告（GHSA-jmr9-qjv8-65gv 符号链接目录穿越、GHSA-7pqw-9j4j-h8q3 符号链接任意写入），受影响版本同为 `<=2.0.1`，**接受状态维持不变**。因两条公告攻击形态不同，后续若实施自维护补丁或回归用例，必须分别覆盖，不得只验证其中一种。
  - 上游状态未变：`extract-zip` 最新发布版本仍为 2.0.1，无修复版本；`@wdio/utils` 仍声明 `@puppeteer/browsers ^2.2.0`，因此既无法通过定向升级消除，也不满足强推 3.x 的前提。已确认解压路径仍可达（`@puppeteer/browsers` 的 `unpackArchive` → `extract-zip`），且 `desktop/wdio.conf.mjs` 在 Windows 默认 `autoDownloadEdgeDriver: true`。
  - **缓解证据（非修复）：**`windows-release.yml` 的 WDIO 任务已显式设置 `WDIO_AUTO_DOWNLOAD_EDGE_DRIVER: "0"` 并使用预装 `msedgedriver`，发布流水线不经过自动下载与解压路径。
- 复核计划见 [`security-remediation-plan.md`](security-remediation-plan.md) 批次 B。
- **相关项**：WQ-ENG-09、WQ-SEC-EXTRACTZIP-01。
