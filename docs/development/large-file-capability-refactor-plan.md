# Tw2Tg 大文件治理与业务能力模块化计划

Owner: Cross-platform Owner。Status: **COMPLETE**（2026-11-10，P0–P13 全部完成，提交 `e35bd091` + `7329dcda`，已推送）。2026-11-11 追加了一次收尾：评审发现 P6/P8 有 21 个原模块私有 helper 被提升为 `pub(crate)` 并由父模块重新导出（可见性放宽，文档此前误记为"未放宽"），已收窄回 `pub(super)`／`pub(in crate::batch)` 与私有桥接；同时补跑 `npm test`（Desktop 230 + Extension 52）与 Full workspace（含 doctest）。该收尾的提交与基线更新在后续批次完成。**后继计划：[`shared-state-reconciliation-plan.md`](shared-state-reconciliation-plan.md)（进行中）。** 当前执行游标唯一来源为 [platform-handoff](../status/platform-handoff.md)。这是 InternalV1 R0–R7 之后的新目标，不继承旧 Git 操作授权。后续工作应作为新目标立项，而不是沿用本文件的授权。

## 1. 范围与不变量

按业务职责治理大型 Rust 生产模块及测试，行数仅为审查信号，内聚实现可有依据地保留。必须保持 InternalV1 恢复/发布/终态持久化及 ADR-015 顺序（admission → scopes → claim → execute → drain → authorize → finalize → terminal persistence → release）；IPC、serde、错误行为、公共导入路径及兼容导出；permit/evidence 构造权限、借用/消费语义和 attempt/session/writer/supervisor 所有权；PREPARED replay 与 COMMITTED 验证区别、完整事务、CAS/attempt fence；生产 V2 enrollment 关闭状态、Windows Owner 边界及 aria2 延期状态。

不新增 crate、依赖、通用事件总线、DI/Strategy；不为搬测试扩大生产可见性；不借重构改行为、删断言或修未经独立证实的缺陷；不追求所有文件低于 1,000 行。

## 2. 权限、操作和验证纪律

授权本计划内文档、源码、测试修改及本地验证。**本次会话未授权**：staging、commit、push、建/切分支、reset、stash、历史重写、删除备份/日志/恢复证据/用户文件、修改 Windows 正式工作树、安装依赖、修改系统权限或 delegation 配置、扩展 aria2。仓库文档中记录的历次 Git 授权属于各自历史范围，不能据以推导本次权限；若某微批确实需要 staging/commit/push 或基线重封存，必须在当次取得可追溯的用户授权才能执行，否则只交付已验证工作区并明确记录该项未闭合。若某项确需额外权限，写明具体操作与原因并等待确认，同时继续独立工作。

### Documentation Gate

业务代码修改前必须先完成文档同步，顺序固定为：先改 Plan／证据／游标，再改源码。

1. 计划口径与当前批次一致（例如 P5 是生产能力提取，不是整理测试）；
2. 证据页不含已知过高的保真度或覆盖率表述；
3. `docs/status/platform-handoff.md` 只有一个有效游标，且下一动作具体可执行：
   涉及文件、符号、期望结果；
4. `docs/architecture/repository-map.md` 的模块职责与当前文件集合一致。

Gate 未闭合时，允许做只读调查、补证脚本和文档修正，不允许搬动业务代码。

### 证据强度规则

- 分别报告：字节一致、去空白后一致、去空白与逗号后一致。三者不可互相代替。
- 名称不可作为项身份的唯一键；同一模块内或不同模块间都可能出现同名项。
- 完整测试路径变化必须显式列表，不能用“叶名相同”代替路径台账。
- `ignored` 数量相等不等于分布未变；必须按实际结果逐条归属。
- 比较器自身的缺陷要写进证据页，并说明新比较口径，不得删掉旧失败记录掩盖。

每个微批先核对 Git/游标，完整阅读目标源码和调用方，列迁移映射；移动一个完整职责；核对函数体、注释、字符串、属性、宏、cfg、可见性；Targeted → Module → Subsystem 风险递增验证；审查完整 diff、新源码和测试映射；更新模块说明、repository map、证据、游标后继续。Cargo 共享 target 串行，独立只读项可并行。遵循统一[验证政策](../validation/validation-policy.md)。

## 3. 有序批次与验收

均依赖 P0、P1 完成；原 `.rs` 保留兼容入口，子模块放同名目录，不无必要改为 `mod.rs`。依赖顺序：P0 → P1 → P2 → P3 → P4 → P5 → P6 → P7a → P7b → P8 → P9 → P10 → P11 → P12 → P13。P9/P10/P11 必须串行。

| 批次 | 工作范围 | 保持与验证重点 |
|---|---|---|
| P0 | 新计划、校准 InternalV1 计划顶部、拆分 handoff 双游标、独立证据页、repository map | 只改文档；docs audit、diff check、范围审查 |
| P1 | 22 文件完整阅读台账、当前行数、生产/测试区间、真实类型/impl/函数/helper/调用者、私有字段/cfg/feature/宏、子进程/测试名/env、符号映射及基线 | 只有实际读完才能记完成；不凭 grep/旧行号/代理摘要搬代码 |
| P2 | Telegram `lib.rs` 测试按 contract、formatting/planning、delivery/outbox/retry、upload、cache/projection 移到 `src/tests/` | 根生产模块/导出不变；不拆 SecretStore；测试名、ignored、断言逐项映射 |
| P3 | Storage `archive_recovery`、`archive_service` 测试按职责移入各自同名测试目录，fixtures 留局部子树 | recovery 覆盖 claim/CAS、dispatch、V1 replay/terminal、crash/cancel、V2/legacy；service 覆盖 local、Sidecar/path、preview、Telegram intent、V2 candidate、rename/forward publish。exact selector 恰选一项；marker、预期崩溃、恢复收敛核验 |
| P4 | Executor 测试分 submission/dedup/transitions、persistence/spec、scheduling/cancel/shutdown、startup/CAS/crash、containment/publication 与 fixtures；`session_boundary_tests` 整体先外置 | 保持 `#[cfg(test)]` 单元测试和逻辑模块名；审查自启动/delegated 选择器；不同时搬 StorageJobPersistence/coordinator |
| P5 | `telegram_send/` 依次提 credentials、intent_planning、payload、delivery、runner（**生产职责提取**，测试随之组织，不是单纯整理测试） | generation 围栏与激活重查、不可变持久化快照、UNKNOWN 不自动重发、仅确认成功后写缓存、stage 顺序与 cooperative stop 不变；原入口保留兼容导出 |
| P6 | `batch/` 提 discovery、selection、submission、dispatch | 筛选顺序、limit、已归档判定、spec 继承、取消 guard/清理和线程生命周期不变 |
| P7a | `commands/records/` 提 export、import_document、import_evidence | 七个 tauri command 入口与 handler 路径留原处；取消注册与 RecordImportWait 同归属；不假定普通 re-export 保留宏生成项 |
| P7b | Unix server/连接/endpoint 移至 `transport/unix.rs` | BrowserTransportAdapter 和协议错误映射留原入口；不重设计 Windows Named Pipe |
| P8 | `production/` 依次提 tool_compatibility、discovery、sidecar_operations、transfer、result_mapping | containment 失败不 fallback 普通 spawn；retained owner、drain、单次 refresh、媒体集合身份不变 |
| P9 | `executor/runtime/` 先 startup，再 application_service | 只调用既有 recovery；JobRecord、Command/RunnerCommand、cancel registry、worker/runner、admission、shutdown/join 同控制边界；不自排队/重复 admission/提前 release，response 前 teardown |
| P9 状态 | 实现已提交 `1cd1692e`；普通库回归有通过记录 | **专项 delegated 验收已闭合（2026-11-09）**：registered-writer drain 后发布、post-commit DB 失败恢复、uncontained writer 前拒绝、startup 先 drain 后决定、runtime replacement 等待 writer 退出，五项均真实执行通过；另加 shutdown/join 与 SIGKILL recovery。未覆盖：真实 aria2c/gallery-dl（延期）、Windows 原生 |
| P10 | Storage `archive_service/` 按完整操作提正常 preview/旧完成、InternalV1 发布/恢复、V2 rename 发布/恢复、legacy/Telegram intent 恢复 | Storage 仅消费授权，不接管 admission/创建 permit；COMMITTED 验证与 PREPARED replay 区别不变 |
| P11 | Storage `archive_recovery/` 按 dispatch、V1 journal、V2 journal/manifest、原子恢复/finalization 组织 | 不造无依据 database 大目录；事务整体、CAS、attempt fence、phase 单调不变 |
| P12 | 其余十文件分类测试；仅证据充分时窄提取，否则说明保留依据 | config：测试分类、遵循 R6 keep-together；desktop archive：保留 PendingArchivePublication 所有权；sidecar_v2：协议模型/验证留聚；file_store：路径安全/文件操作留聚；desktop runtime：整理测试，只有依赖证据清楚才提参数转换/日志投影，不拆组合根生命周期；supervisor：spawn/wait/terminate/Drop 留聚；proxy：测试分类、路由决策留聚；websocket_transport：测试分类，认证/deadline/stop 边界留聚；upload：完整复核并保留，不默认拆 progress；writer_lifecycle：外置两测试模块，不动 permit/evidence 权限 |
| P12a 状态 | `config.rs`、`sidecar_v2.rs`、`file_store.rs`、`proxy.rs` 测试分类（提交 `1ba73fae`） | 生产结构未动；逐项保真复核见证据页；本轮修正了随该提交进入 HEAD 的重复 `use super::*;` |
| P12b | `archive.rs`、`runtime.rs`、`websocket_transport.rs` 测试分类 | `PendingArchivePublication` move-only 所有权不动；组合根生命周期不拆；websocket 认证/deadline/stop 状态机不拆；`archive.rs` 的 2 个 `#[ignore]` 与 `#[cfg(target_os = "linux")]` 随测试移动，ignored 计数须仍为 13 |
| P12c | `supervisor.rs`、`writer_lifecycle.rs`、`upload.rs` | supervisor 的 spawn/wait/terminate/Drop 留聚；writer_lifecycle 外置两测试模块，permit/evidence 权限与借用/消费语义不变；upload 完整复核后保留，不默认拆 progress |
| P13 | 核销 22 项、汇总生产/测试行数和新边界、最终集成、文档/架构/游标收尾 | 准备交接增量；无 Git 授权只交付已验证工作区，不称正式交接 |
| P13 状态 | **已完成并闭环（2026-11-10，已提交 `e35bd091` 并推送）** | 22 项核销表见证据页；workspace **`--tests` 22 target 845/0/24**（含 `--lib` 806/0/13）、JS check、Sidecar pytest **101 passed**、架构检查器自测 **12/12**、delegated 专项 **7/7 + 集成 11/11 PASS**；架构 baseline 已按 26 条已审查预期 drift 重封存（278 files / 0 findings），未扩大例外、未放宽门禁；游标对账提交 `7329dcda`，本地与 upstream 一致 |

## 4. P1 的 22 文件台账入口

以下是用户提供的历史行数，不代表当前计数或已阅读。P1 重算。台账记录每文件总行数、生产/测试区间、完整顶层类型及 impl/函数/helper、私有字段访问、cfg/feature/宏、子进程与完整测试名/env、迁移处置，以及“原符号 → 目标模块 → 兼容路径 → 可见性变化 → 覆盖测试”和阅读/未读范围。按文件分页阅读全文并复核调用方；备份/生成文件/依赖不计入。

| # | 文件 | 历史行数 | 批次 |
|---:|---|---:|---|
| 1 | `desktop/src-tauri/src/executor.rs` | 9815 | P4/P9 |
| 2 | `desktop/src-tauri/src/telegram_send.rs` | 3153 | P5 |
| 3 | `crates/xarchive-storage/src/tests/archive_recovery.rs` | 2748 | P3 |
| 4 | `desktop/src-tauri/src/executor/runtime.rs` | 2692 | P4/P9 |
| 5 | `crates/xarchive-telegram/src/lib.rs` | 2691 | P2 |
| 6 | `crates/xarchive-storage/src/tests/archive_service.rs` | 2309 | P3 |
| 7 | `desktop/src-tauri/src/batch.rs` | 1968 | P6 |
| 8 | `desktop/src-tauri/src/config.rs` | 1814 | P12 |
| 9 | `desktop/src-tauri/src/archive.rs` | 1745 | P12 |
| 10 | `crates/xarchive-storage/src/archive_service.rs` | 1614 | P10 |
| 11 | `desktop/src-tauri/src/production.rs` | 1595 | P8 |
| 12 | `crates/xarchive-protocol/src/sidecar_v2.rs` | 1548 | P12 |
| 13 | `crates/xarchive-storage/src/file_store.rs` | 1266 | P12 |
| 14 | `desktop/src-tauri/src/runtime.rs` | 1258 | P12 |
| 15 | `crates/xarchive-download/src/supervisor.rs` | 1218 | P12 |
| 16 | `desktop/src-tauri/src/proxy.rs` | 1182 | P12 |
| 17 | `desktop/src-tauri/src/websocket_transport.rs` | 1146 | P12 |
| 18 | `desktop/src-tauri/src/commands/records.rs` | 1097 | P7a |
| 19 | `crates/xarchive-storage/src/archive_recovery.rs` | 1097 | P11 |
| 20 | `crates/xarchive-telegram/src/upload.rs` | 1033 | P12 |
| 21 | `crates/xarchive-workflow/src/writer_lifecycle.rs` | 1030 | P12 |
| 22 | `desktop/src-tauri/src/transport.rs` | 1007 | P7b |

## 5. 基线、验证、证据与最终集成

P1 保存受影响测试名及 ignored 属性，串行基线：

```sh
cargo test -p xarchive-telegram --lib -- --test-threads=1
cargo test -p xarchive-storage --lib -- --test-threads=1
cargo test -p xarchive-desktop --lib -- --test-threads=1
```

每个源文件及必要调用方须实际阅读全文后才可标记已审；本地运行环境单次读取上限不代表
源码只需抽样。将每份已审内容分页后结合读取工具输出核对边界，给出覆盖范围/未读范围。
测试台账保留完整 `#[test]` 名称、全部属性（特别是 `#[ignore]`）、fixture/helper 归属，
并对照 Cargo 基线枚举；无法自动解析的测试必须人工核验。每条生产符号也要记录全部
调用位置与导出/可见性，不能只用声明 grep 代替完整实现复核。

其他 crate 在相应批次前建基线。按测试名称/映射/ignored/断言比较，不只比总数。崩溃测试必须证明实际目标、marker、崩溃及恢复结果。恢复/发布/所有权/shutdown 改动重跑相关 InternalV1 与 delegated 生产路径；普通/mock/ignored 结果不升级为真实进程/原生/GUI 验收。

常规运行 `cargo fmt --all -- --check`、`git --no-pager diff --check`、`node scripts/docs-audit.mjs`、`node scripts/check-architecture.mjs`。架构阈值用当前政策 Rust library 3000、Desktop 4000；1000 仅为评审信号。基线针对 tracked 集；未授权 staging 时未跟踪源码未纳入检查器的结果不算完整 PASS。不得自动重写 baseline 或扩大例外；修改前审查并取得授权。证据绑定 source SHA 与 dirty identity，记录真实退出码和日志；失败及重试都保留。终端未捕获完成状态先核对，不盲目重跑。

P13 在根目录最终集成：

```sh
cargo test --workspace --no-fail-fast -- --test-threads=1
cargo fmt --all -- --check
npm test
npm run check
npm run build
node --test scripts/check-architecture.test.mjs
node scripts/check-architecture.mjs
node scripts/docs-audit.mjs
git --no-pager diff --check
```

Sidecar/delegated 入口须阅读仓库后确认，不臆造命令/测试名。Cargo target 串行；仅风险允许的独立 Node 只读检查可并行。

## 6. 完成、暂停与续跑

完成要求：22 项均经源码复核明确处置；计划拆分及测试组织完成；无测试丢失、零选择伪通过、未经授权权限/契约变更；相关 InternalV1 与最终集成证据完整；文档、架构、基线、工作区一致；Windows、aria2、Git 待办分列。若契约变化、私有边界无法保持、用户修改冲突或能力不足，暂停该批、记录原因，继续独立项；不得删断言、跳过或弱化 fail-closed。中断时 handoff 记录已完成范围、未完成项、source/dirty/index、证据、Owner 和下一具体动作。Windows 原生工作归 Windows Owner，aria2 仍延期，正式 Git handoff 需另有授权。

依据：[统一验证政策](../validation/validation-policy.md)、[模块化设计政策](../architecture/MODULAR_DESIGN_POLICY.md)、[InternalV1 历史计划](internal-v1-completion-and-refactor-plan.md)、[当前游标](../status/platform-handoff.md)、[新计划证据](../validation/large-file-capability-refactor-evidence.md)。
