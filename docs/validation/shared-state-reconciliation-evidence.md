# 共享层状态对账与交付收口 — 证据页

Owner: Cross-platform Owner。绑定计划：
[`shared-state-reconciliation-plan.md`](../development/shared-state-reconciliation-plan.md)。

本页记录结果与证据。结果状态取值、必填字段与非 PASS 判定标准由
[`validation-policy.md`](validation-policy.md) 定义，本页不重复定义。

基线环境（2026-11-11 实测）：

- 分支 `cross-platform/automatic-pairing-reconcile-20261002`，HEAD
  `e8cb320fee2cf36221d013bf7f520795c2696d43`；
- 工作区 20 个已跟踪文件修改；本地 tracking ref 显示 ahead 2 / behind 0，未 fetch；
- WSL2 Linux，Node `26.7.0`，Cargo `1.98.0`；
- 进程 cgroup `/non-systemd`，根 cgroup 对当前用户不可写，`Delegate=no`。

> **2026-11-11 集成说明（不改写上方基线）。** 上方基线绑定的是当时的分支与 HEAD，
> 是历史证据，保持原样。该批次随后以快进方式 `918a2990..aea19806` 集成进 `dev`，
> 现 `dev` 与 `cross-platform/automatic-pairing-reconcile-20261002` 同指 `aea19806`。
> `e8cb320f` 与 `aea19806` 之间的差异只涉及文档。集成本身不是验证结果，不升级本页
> 任何一项的状态；各项结果仍绑定到其记录的 source revision。集成事实见
> [`../status/platform-handoff.md`](../status/platform-handoff.md) 的
> “Branch integration into `dev`” 一节。

## R0 — Documentation Gate

| 项 | 状态 | 证据 |
|---|---|---|
| 计划与证据页存在 | `PASS` | 本次创建的两个文件 |
| 单一游标 | `PASS` | `docs/status/platform-handoff.md` 顶部 Resume summary |
| repository map 登记 | `PASS` | `docs/architecture/repository-map.md` 新增行 |
| 旧 Plan 后继引用 | `PASS` | large-file Plan 顶部指向本计划，`COMPLETE` 未改写 |

## R1 — 既有源码核销

审查方式：逐文件读取 `git diff`、核对调用点与导出路径、核对 `include!` 展开位置。
**只读审查，本轮未修改任何源码。**

| # | 文件 | 变更性质 | 结论 |
|---:|---|---|---|
| 1 | `desktop/src-tauri/src/production.rs` | 父模块桥接由 `pub(crate) use` 收窄为私有 `use`；`resolved_tool_version` 保持 `pub(crate)` | 无行为变更 |
| 2 | `desktop/src-tauri/src/production/result_mapping.rs` | 2 处 `pub(crate)` → `pub(super)` | 无行为变更 |
| 3 | `desktop/src-tauri/src/production/sidecar_operations.rs` | 3 处 `pub(crate)` → `pub(super)` | 无行为变更 |
| 4 | `desktop/src-tauri/src/production/tool_compatibility.rs` | 2 处 `pub(crate)` → `pub(super)`；`resolved_tool_version` 保持 `pub(crate)` | 无行为变更 |
| 5 | `desktop/src-tauri/src/production/transfer.rs` | 可见性收窄至 `pub(super)`；`transfer_files` 收为模块私有；`TransferAttemptFailure` → `pub(super)` | 无行为变更 |
| 6 | `desktop/src-tauri/src/batch.rs` | `discovery_error_code` 桥接 `pub(crate)` → `pub(in crate::batch)` | 无行为变更 |
| 7 | `desktop/src-tauri/src/batch/discovery.rs` | 同上定义处收窄 | 无行为变更 |
| 8 | `desktop/src-tauri/src/executor.rs` | `InMemoryCommitRecoveryFactsProvider` 由基础导出改为 `#[cfg(test)]` 独立导出 | 仅测试构建面 |
| 9 | `desktop/src-tauri/src/runtime.rs` | `record_recovery_results` 加 `#[cfg(test)]` 与说明注释 | 仅测试构建面 |
| 10 | `crates/xarchive-storage/src/tests/archive_recovery/v2_finalization.rs` | 去掉未使用的 `mut` | 测试局部 |
| 11 | `crates/xarchive-storage/src/tests/archive_service/telegram_intent.rs` | 去掉未使用的 `mut` | 测试局部 |

### 调用点核对（用于判定收窄是否安全）

- `execute_gallery_dl_archive`、`media_type_name`、`extract_v2`、`download_v2`、
  `verify_downloaded_files`、`revalidate_tool_compatibility`、`executable_version`、
  `extraction_to_metadata`、`execute_aria2_archive`、`ensure_same_media_collection`、
  `format_transfer_failure`、`aria2_config`、`run_transfer_with_supervisor`、
  `transfer_with_one_refresh`、`transfer_once`：全部调用点位于 `production` 子树内，
  或位于 `production.rs:240` 的 `include!("production_transfer_fixture.rs")` 展开处
  （该展开在本模块作用域内，私有 `use` 足以解析）。
- `transfer_files`：唯一调用点为 `transfer.rs` 内部，收为模块私有正确。
- `resolved_tool_version`：被 `runtime.rs`、`commands/executor.rs` 使用，因此必须保持
  `pub(crate)`，当前 diff 确实保留。
- `discovery_error_code`：调用点为 `batch/discovery.rs` 内部，`pub(in crate::batch)`
  覆盖实际使用范围。
- `InMemoryCommitRecoveryFactsProvider`：生产构建无调用点；全部使用者在
  `executor/tests/persistence_spec.rs`，`#[cfg(test)]` 导出与调用面一致。
- `record_recovery_results`：生产路径使用 `record_startup_recovery_report`；唯一使用者为
  `runtime/tests/startup_recovery.rs`。

结论：11 个文件的 diff 未改变任何函数体、签名、属性语义、字符串、断言或测试名，
未放宽任何作用域（全部为收窄或等价保持）。**不存在需要回退的改动。**

## R2 — 架构基线收口

| 项 | 状态 | 证据 |
|---|---|---|
| 收口前测量 | `PASS` | `check-architecture.mjs --json` 退出 1，2 条 `baseline-delta`（`executor.rs` 907→913、`runtime.rs` 916→922） |
| 基线内容哈希差异清点 | `PASS` | 11 个文件 SHA-256 与基线不同，与 R1 审查集合完全一致 |
| 基线更新 | `PASS` | 见下方"更新后" |
| 收口后门禁 | `PASS` | `check-architecture.mjs` 退出 0 |
| 检查器自测 | `PASS` | `node --test scripts/check-architecture.test.mjs` 12/12 |

### 更新后

- `node scripts/check-architecture.mjs` 退出 **0**，278 files，**0 findings**。
- 基线仅反映 R1 已核销的 11 个文件行数与哈希；未新增或扩大
  `scripts/architecture-exceptions.json`，未修改 `architecture-policy.json`
  阈值，未修改 `check-architecture.mjs`。

## R3 — 产品事实对账

核对方式：直接读取源码判定"当前实现"，与文档既有"目标"表述对照。**只改文档，未改产品行为。**

| 文档 | 修正内容 | 代码依据 |
|---|---|---|
| `README.md` | 删除"aria2-only 是当前代码链路"的断言，改为双路径 + 目标未实施；同时修正 Telegram 状态与 Credential Manager 表述 | `config.rs:741` `use_aria2: false` 默认；`production.rs:69` `if use_aria2 { execute_aria2_archive } else { execute_gallery_dl_archive }` |
| `docs/architecture/overview.md` | 媒体结果契约补充"目标尚未收敛的双路径" | 同上 |
| `docs/development/status.md` | 顶部新增媒体传输事实校正，说明"aria2-only transfer driver"是 driver 层能力，非运行时单一后端 | `worker_v2.py:145` 仍处理 `download` 命令；`sidecar_operations.rs:128` `download_v2` 存在 |
| `docs/development/roadmap.md` | 去除"当前媒体链路是 aria2-only"的等价表述，标注为目标 | 同 README |
| `docs/user-guide/telegram.md` | 更正 Telegram 当前状态：共享层与接线已存在，缺原生实机验收 | `runtime.rs:96,144,197` sender 生命周期；`lib.rs:71` `WindowsCredentialStore` 注入；`windows_credentials.rs` 完整 adapter |

结论：这些修正缩小了文档断言，没有扩大任何能力声明，也没有把未验证项标为 PASS。

## R4 — 记忆规则纠偏

修正上一轮引入的四处与既有规则冲突的表述：

1. 来源表由"单一事实来源（第二套规则）"改为**导航索引**，明确不覆盖各文档既有权威性。
2. 移除全局文档优先级排序，改为"按事实类型找归属文档"：源码行为由代码判定，范围由 Plan，执行结果由证据页，当前批次由游标。
3. 验证结果表补入 Linux/共享证据页；原表只列 Windows，遗漏了 `docs/validation/` 的共享证据。
4. 撤销"never the current cursor's validation claims"的绝对限制，允许游标简述并强制链接证据；同时明确验证证据不可由 `git diff` 重建，必须保留。

`docs/README.md` 与 `docs/architecture/repository-map.md` 的对应条目同步更新为新定义。

## R5 — 最小必要验证

执行日期 2026-11-11，工作区为本次批次结束状态（HEAD `e8cb320f` + 未提交的文档与基线改动）。
日志目录：`.tw2tg-tmp/agent-scratch/build-logs/reconcile/`。

| # | 检查 | 命令 | 退出码 | 结果 |
|---:|---|---|---:|---|
| 1 | Desktop 编译（含测试目标） | `cargo check -p xarchive-desktop --lib --tests` | 0 | 通过，`#[cfg(test)]` 与私有桥接在两种构建面均解析 |
| 2 | Desktop lib | `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored** |
| 3 | Storage lib | `cargo test -p xarchive-storage --lib -- --test-threads=1` | 0 | **178 passed / 0 failed / 0 ignored** |
| 4 | startup recovery 选择器 | `cargo test -p xarchive-desktop --lib 'runtime::tests::startup_recovery'` | 0 | **2 passed**，376 filtered out（非零选择） |
| 5 | recovery 提交选择器 | `cargo test -p xarchive-desktop --lib 'executor::tests::persistence_spec::recover_commit'` | 0 | **5 passed**，373 filtered out（非零选择） |
| 6 | 格式 | `cargo fmt --all -- --check` | 0 | 干净 |
| 7 | Node 测试 | `npm test` | 0 | Desktop **230/230**，Extension **52/52** |
| 8 | 架构检查 | `node scripts/check-architecture.mjs` | 0 | 278 files，**0 findings** |
| 9 | 架构检查器自测 | `node --test scripts/check-architecture.test.mjs` | 0 | **12/12 pass** |
| 10 | 文档结构 | `node scripts/docs-audit.mjs` | 0 | PASS |
| 11 | diff 卫生 | `git --no-pager diff --check` | 0 | 干净 |

第 4、5 项直接证明 R1 判定的两处 `#[cfg(test)]` 收窄没有破坏测试可访问性：
被收窄的 `record_recovery_results` 与 `InMemoryCommitRecoveryFactsProvider` 均由真实测试执行覆盖。

### 未执行（`NOT_RUN`，不得记 PASS）

- 四项 disposable delegated-cgroup 验收（计划 R7）。
- 真实 aria2c / gallery-dl 二进制验收：本机无二进制，且用户已延期。
- Windows 原生（Named Pipe、GUI、打包、安装器、真实 Telegram 账号）：属 Windows Owner。
- Full workspace 全量回归：本批次只改文档与基线，源码行为未变，未达触发 Full 的必要条件；
  上一次 Full（852/0/24）仍绑定其记录的 source 与 dirty identity，本轮不重跑也不据其主张本轮结论。

## R6 — Git 交付

| 项 | 状态 | 证据 |
|---|---|---|
| 远端核对（推送前） | `PASS` | HTTPS + `gh` 凭据 fetch：远端为 `7329dcda`，本地 ahead 4 / behind 0，无分叉 |
| 提交 1：源码收窄 + 基线 | `PASS` | `fcf7c0ef`，12 文件（11 源码/测试 + `architecture-baseline.json`） |
| 提交 2：文档批次 | `PASS` | `614e31b7`，15 文件 |
| 推送 | `PASS` | `7329dcda..614e31b7`，HTTPS 通道 |
| 远端实际 ref 核实 | `PASS` | 推送后重新 fetch：`FETCH_HEAD` = `614e31b7`，与本地 `HEAD...FETCH_HEAD` 为 **0/0** |

说明：configured origin 仍是 SSH，但本机 `~/.ssh` 无密钥。本次沿用历史做法，使用已认证的
HTTPS 通道（`gh` CLI，scope 含 `repo`）完成 fetch/push，**未修改 remote 配置、未写入任何凭据到仓库**。

提交分组理由：源码收窄与基线必须成对出现——基线记录的是收窄后的行数与哈希，
分开提交会让中间提交出现"基线已更新但源码未变"的误导状态。

远端推送时 GitHub 报告 default branch 有 21 个依赖漏洞告警（10 high / 7 moderate / 4 low）。
这是**依赖安全状态**，与本次改动无关；未在本批次处理，也不改变任何验证结论。需要时应按
`docs/development/security-remediation-plan.md` 另立批次。

## R7 — Delegated 四项

执行方式：`systemd-run --user --scope --property=Delegate=yes` 提供**瞬态** delegated
cgroup，不修改任何持久化 systemd 配置。实测该 scope 内 `DIR_WRITABLE=yes`、
`MKDIR_OK=yes`、`RMDIR_OK=yes`，控制器为 `cpu memory pids`。

| # | 测试 | 命令 | 退出码 | 结果 |
|---:|---|---|---:|---|
| 1 | `archive::completion_tests::writer_drain::registered_undrained_writer_is_refused_by_production_finalizer` | `--ignored --exact` + 上述 scope | 0 | **PASS**，1 passed / 377 filtered out |
| 2 | `archive::completion_tests::writer_drain::registered_writer_with_incomplete_drain_is_refused_by_authorization` | 同上 | 0 | **PASS**，1 passed / 377 filtered out |
| 3 | `executor::tests::containment_publication::production_factory_contained_handshake_and_drop_remove_writer_group` | 同上 | 0 | **PASS**，1 passed / 377 filtered out |
| 4 | `executor::tests::startup_recovery_drains_live_descendant_after_owner_sigkill` | 同上 | 0 | **PASS**，1 passed / 377 filtered out |

每项都用 `--exact` 且 `running 1 test` 确认**恰好选中一项**，`0 ignored` 确认确实执行了
原本 `#[ignore]` 的用例。日志：`.tw2tg-tmp/agent-scratch/build-logs/reconcile/r7-0*.log`。

执行后资源核对：`tw2tg-att-*` 残留 cgroup **0**，`cgroup_launcher` 残留进程 **0**。

**绑定范围**：这四项绑定本地 source `614e31b7` 与其未提交前的同源工作区状态，证明
Linux 共享层的 writer drain、authorization 拒绝、containment handshake 与 startup
recovery 行为。它们**不证明**任何 Windows 原生 adapter、NTFS 语义或 GUI 行为。

前面的 7/7 与 11/11 delegated 历史证据仍绑定各自记录的 source SHA，本批次未重跑，
也未用本轮四项替代它们。
