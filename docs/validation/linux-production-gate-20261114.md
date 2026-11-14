# Linux 生产门验收批次 — 2026-11-14

Owner: Cross-platform Owner。绑定计划：
[`desktop-download-output-plan.md`](../development/desktop-download-output-plan.md)
（B4/B5/B6/B7 生产门）与
[`shared-state-reconciliation-plan.md`](../development/shared-state-reconciliation-plan.md)。

本页只记录结果与证据。结果状态取值、必填字段与非 PASS 判定标准由
[`validation-policy.md`](validation-policy.md) 定义，本页不重复定义。

## 0. 绑定范围

| 字段 | 值 |
|---|---|
| `source_sha` | `57a8a92c`（`dev`，本地 tracking ref 与 `origin/dev` 一致；本批未查询实时远端） |
| `dirty_state` | **三个阶段分开记**：(a) 基础模块套件运行时工作树干净（`git status --porcelain` 当次为空）；(b) 第一阶段 Act 批（D1/F2）在 dirty 树上运行：**28 个已跟踪修改 + 1 未跟踪**，`git diff --stat` = 1840 增 / 92 删，无 manifest / lockfile / migration 变更；(c) 第二阶段 Documentation Gate 只改文档，不改变 (b) 的源码字节。本页记录的测试结果绑定 (a) 或 (b)，**不覆盖**第二阶段之后的代码 |
| `platform` | Linux x86_64（WSL2） |
| `environment` | `$XDG_RUNTIME_DIR` 用户会话；进程位于 `user@1000.service/app.slice` |
| `tool_versions` | Node `26.7.0`、Cargo `1.98.0`、Python `3.14.4` |
| `build_origin` | 本地 `cargo test`，共享 `target/` 目录 |
| `delegated scope` | `systemd-run --user --scope -p Delegate=yes`，瞬态，不修改持久配置 |

## 0b. Documentation Gate 状态（2026-11-14）

本批**先**完成文档对账，再谈业务代码：

| # | 项目 | 状态 |
|---:|---|---|
| 1 | 游标区分"运行时空树"与"当前 dirty"，并列出具体文件 | DONE |
| 2 | V2 fixture 定性由"不是产品缺陷"改为可证伪的 `ROOT_CAUSE_UNKNOWN`；P1-B 后进一步确证为 `ROOT_CAUSE_ESTABLISHED`（见 §3） | DONE |
| 3 | 游标下一步改为 `C1 -> D1 -> F2/F3 -> G` 依赖顺序，D1 明确不得先行 | DONE |
| 4 | 记录 V1-02c/02d 的**已批准**验收方式（可达性证明，不伪造 permit） | DONE |
| 5 | 更正 D1 现状：V2 Storage 层四组合已有测试；缺的是 InternalV1 生产接线 | DONE |
| 6 | Windows queue 的 source revision 修正为 `57a8a92c` | DONE |
| 7 | 历史归档 Stage 3 以实测行数与架构 0 findings 对账关闭；Stage 4 保持 `NEEDS_REVIEW` | DONE |
| 8 | **P1 更正**：`internal-v1-completion-and-refactor-plan.md` 的 V1-02b 由"未证明"改为**本批 delegated 实测 PASS**（授权边界 + 生产 finalizer 边界各 1 例） | DONE |
| 9 | **P1 产出**：C1 验收矩阵 32 项，无一落在 `IMPLEMENTATION_MISSING` / `ASSERTION_MISSING`（见 §3a） | DONE |
| 10 | **Act 批复核**：重测工作树为 **26 个已跟踪修改 + 1 未跟踪**（非 5 个 Rust 文件）、baseline **17 条改封存**（非 5 条，0 增 0 删）→ 游标已改 | DONE |
| 11 | **Act 批复核**：D1 三项过度表述（exports-off replay / production factory / serializer failure）已改为 `NOT_CLOSED` 并逐条写明真实覆盖（§4） | DONE |
| 12 | **Act 批复核**：F2 取消可达性与"取消 vs 写入"窗口列为未验证，Node 用例明确标注为源码匹配（§5） | DONE |
| 13 | **Act 批复核**：Plan 中"D1 scope 待决定"已删除，改为"已批准、不得重问"，并写入下一阶段 Batch 0–5 表 | DONE |

Gate 第 10–13 项是 2026-11-14 Act 批新增，**只改文档，不改业务代码**。它们的前提是
`git diff --check` 退出码 0（由子进程返回值确认），以及逐条对比 `HEAD` 的
`scripts/architecture-baseline.json` 得到"17 改动 / 0 新增 / 0 删除"。

历史归档迁移（Stage 2 剩余部分、Stage 4）**不是**本批前置，未执行。

**这些结果不证明任何 Windows 行为。** Windows 原生 adapter、NTFS、Job Object、
GUI、打包与真实 Telegram 账号验收仍归 Windows Platform Owner，保持 `NOT_RUN`。

## 1. 基线模块门（A2）

| # | 检查 | 命令 | 退出码 | 结果 |
|---:|---|---|---:|---|
| 1 | Desktop lib | `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored**（378 results） |
| 2 | Storage lib | `cargo test -p xarchive-storage --lib -- --test-threads=1` | 0 | **178 passed / 0 failed / 0 ignored** |
| 3 | Workflow lib + 集成 | `cargo test -p xarchive-workflow --lib` / `--tests` | 0 | lib **49/0/0**；集成 49/11/5/3/7(2 ignored)/2 全部 ok |
| 4 | Core lib | `cargo test -p xarchive-core --lib` | 0 | **49/0/0** |
| 5 | Protocol lib | `cargo test -p xarchive-protocol --lib` | 0 | **31/0/0** |
| 6 | Download lib + 集成 | `cargo test -p xarchive-download --lib` / `--tests` | 0 | lib **53/0/0**；集成 53/0/3ignored/7 ok |
| 7 | Sidecar supervisor lib | `cargo test -p xarchive-sidecar-supervisor --lib` | 0 | **17/0/0** |
| 8 | Node 套件 | `npm test` | 0 | Desktop **233（231 pass / 2 Windows-only skipped）**，Extension **52/52** |
| 9 | Sidecar pytest | `python3 -m pytest sidecar/tests -q` | 0 | **101 passed** |
| 10 | 架构门禁 | `node scripts/check-architecture.mjs` | 0 | 278 files、44 js/ts、22 edges、**0 findings** |
| 11 | 文档结构 | `node scripts/docs-audit.mjs` | 0 | PASS |
| 12 | diff 卫生 | `git --no-pager diff --check` | 0 | 干净 |

日志目录：`.tw2tg-tmp/agent-scratch/build-logs/goal-20261114/`。

## 2. 生产门 ignored 用例（A1）

此前 R7 只执行了 4 项 delegated 用例。本批补齐**其余 5 项中可执行的 4 项**，
每项均用 `--exact --ignored`，`running 1 test` 且 `0 ignored` 证明确实执行了
原本 `#[ignore]` 的用例。

**这四项在本仓库并非首次执行——本页更正一个容易产生的误读。** 它们都已在
[`linux-crash-replay-20261016.md`](linux-crash-replay-20261016.md) 中记录过 PASS
（V1-01/V1-03、V1-07 等条目，绑定当时各自的 source 与 `/tmp/p4-*.log`）。本轮的
角色是**重验**：`dev` 自那时起前进了约 60 个提交（P0–P13 模块拆分、`fcf7c0ef`
可见性收窄），按 [`validation-policy.md`](validation-policy.md)"PASS 仅在相关代码、
依赖、平台契约与环境不变时可复用"的规则，旧结果对 `57a8a92c` 为
`REVALIDATION_REQUIRED`。本轮给出的价值是：**在当前 revision 上重新绑定**，且日志
保存在 `build-logs/` 下而非已丢失的 `/tmp` 路径。

| # | 测试 | 结果 | 证据 |
|---:|---|---:|---|
| 1 | `executor::session_boundary_tests::executor_shutdown_joins_active_writer_before_successor_runs` | **PASS** 1 passed / 377 filtered | `a1/01-shutdown-join.log` |
| 2 | `executor::session_boundary_tests::runtime_replacement_joins_active_writer_then_runs_successor` | **PASS** 1 passed / 377 filtered | `a1/02-runtime-replace.log` |
| 3 | `executor::tests::containment_publication::production_runner_finalizes_internal_v1_archive_after_registered_writer_drain` | **PASS** 1 passed / 377 filtered | `a1/03-finalize.log` |
| 4 | `executor::tests::containment_publication::production_runner_post_commit_database_failure_recovers_without_false_success` | **PASS** 1 passed / 377 filtered | `a1/04-post-commit.log` |
| 5 | `executor::tests::containment_publication::test_only_v2_scheduler_fixture_publishes_and_recovers_protocol_bytes` | **FAIL**（见 §3） | `a1/05-v2-fixture.log` |

执行后资源核对：`tw2tg-att-*` 残留 cgroup **0**，`cgroup_launcher` 残留进程 **0**。

### 已关闭的生产门

- **B7 shutdown/join（项 1、2）**：证明 executor shutdown 与 runtime replacement
  都会先 join 活动 writer 再运行 successor，且 successor 在真实
  `RuntimeState::replace_executor` 路径上执行。
- **B5 注册 writer drain 后 finalize（项 3）**：证明生产 runner 在注册 writer
  排空后完成 InternalV1 archive 终结。
- **B6 post-commit DB 失败恢复（项 4）**：证明 attempt staging 已提交到最终目录、
  归档元数据尚未收敛时数据库写失败，runner 不得报告终态成功，且启动恢复能把
  attempt 恰好收敛一次、不重复事件。

这四项的历史记录绑定更早的 source；本轮把它们重新绑定到 `57a8a92c`，并提供
可复核的本地日志，见上方更正说明。

## 3. 未通过项：`test_only_v2_scheduler_fixture_publishes_and_recovers_protocol_bytes`

`status` = **FAIL**，`defer_reason` = **`ROOT_CAUSE_ESTABLISHED`（失败点已定位；V2 验收仍未开启）**。

### 失败点 — 已于本批确证（2026-11-14）

`startup.rs` 原先用 `Err(_)` 丢弃原因，导致该 FAIL 长期不可诊断。本批改为把每个
`Refused` 的原因记入内部 `StartupRecoveryReport::refusals`（不进入
`StartupRecoveryOutcome` 枚举、不写入用户日志），随后重跑精确 fixture，得到：

```text
job state             = Failed
last_error_code       = EXECUTOR_WORKER_FAILED
last_error_message    = invalid persisted job state:
                        attempt-scoped Sidecar publication requires InternalV1
```

**这条链路是自洽的，不是竞态或环境问题：**

1. fixture 通过 `v2_service.v2_enrollment = true` 显式登记 **V2** contract；
2. pre-PREPARED V2 分支（`recovery.rs` 的 pre-PREPARED 处理）**确实被执行**，
   并经真实 `SessionExecutionBoundary::execute_session` 走普通执行工厂；
3. 该工厂的 finalizer `PendingArchivePublication::finalize`
   （`archive.rs:297-298`）固定调用 InternalV1 发布 facade，
   `publish_internal_v1_attempt` → `complete_sidecar_archive_attempt_inner`
   在 `internal_v1_publication.rs` 内要求 contract 必须是 InternalV1，否则返回
   `StorageError::InvalidState("attempt-scoped Sidecar publication requires InternalV1")`；
4. 该错误使 Job 落为 `Failed`，`recover_v2_on_runner` 的
   `Finished(Failed)` 分支返回 `Err`，最终被归一为 `Refused`。

这同时确认了改动前三项假设中的**第三项**（InternalV1-only finalizer），并澄清了
第三项表述的偏差：pre-PREPARED V2 分支不是"没运行"，而是运行后被 contract 检查
拒绝。因此准确说法是"失败点已定位"，不是"V2 恢复路径未被执行"。

### 该失败点**不**证明的事

- **不**证明 V2 恢复算法本身有缺陷 —— journal/rename replay 未到达；
- **不**证明生产有回归 —— 生产 `recover_startup()` 硬编码
  `recover_startup_with_v2(false)`，该路径不可达；
- **不**证明 fixture 是错的 —— "V2 enrollment 必须走 V2 finalize" 与
  "V2 enrollment 复用 InternalV1 finalizer" 两种 intended contract 都未被本批确认；
- **不**授权"为通过而改断言"。

### 结论与后续

该 fixture 保持在 deferred 集合：`status` = `FAIL`，`defer_reason` =
`ROOT_CAUSE_ESTABLISHED`（不再是 `ROOT_CAUSE_UNKNOWN`）。

`follow_up`：在 V2 生产 finalize 接线被正式纳入范围（C1 之后、需新批准）**或**
显式决定 V2 复用 InternalV1 finalizer 之前，该用例没有可满足的路径。不得删除用例、
不得标记 PASS、不得放宽断言。生产 v2 仍 fail-closed，符合 ADR-015 条款 8。

## 3a. C1 验收矩阵（P1 产出，2026-11-14）

本矩阵把 C1 生命周期行为逐项映射到实现、测试与当前证据。**不按缺失的测试名推断
缺口**；已存在的覆盖必须被承认，未被覆盖的必须写成具体差额。

定性取值：`REUSED`（既有证据可复用）、`REBOUND`（本次重新绑定到 `57a8a92c`）、
`ASSERTION_MISSING`（断言确实缺）、`IMPLEMENTATION_MISSING`（实现确实缺）、
`NOT_REACHABLE`（合法状态不可达，需构造证明）、`DEFERRED`、`WINDOWS_OWNED`。

| # | C1 行为 | 实现位置 | 证据 | 定性 |
|---:|---|---|---|---|
| 1 | runner admission 饱和不失败 | runner slot | `saturated_runner_rejection_is_non_failing_and_reschedulable` | `REUSED` |
| 2 | reservation → claim 顺序与竞态 | `ExecutionReservation` | `claiming_an_archive_job_is_atomic_against_state_and_attempt`、`persisted_scheduler_binds_attempt_before_successful_completion` | `REUSED` |
| 3 | claim 拒绝 terminal / overflow / PREPARED 绕过 | `beginning_archive_attempt_*` | `beginning_archive_attempt_rejects_terminal_jobs_without_incrementing_attempt`、`..._refuses_to_bypass_a_prepared_journal` | `REUSED` |
| 4 | InternalV1 successor CAS | `internal_v1_successor_claim_*` | `internal_v1_successor_claim_is_predecessor_cas_and_preserves_downloading`、`..._refuses_journaled_or_non_v1_jobs` | `REUSED` |
| 5 | attempt 隔离 staging | `create_attempt_staging_dir` | 见 #2/#3 与 #9 | `REBOUND` |
| 6 | 未注册 writer 拒绝发布 | `PublicationOwner::authorize` | `production_runner_rejects_uncontained_internal_v1_writer_before_publish`（断言无 PREPARED journal、无最终目录） | `REBOUND` |
| 7 | **V1-02b 已注册未排空（授权边界）** | `authorize` | `registered_writer_with_incomplete_drain_is_refused_by_authorization` — **本批 delegated 实测 PASS，`running 1 test` / `0 ignored`** | `REBOUND` |
| 8 | **V1-02b 已注册未排空（生产 finalizer 边界）** | `finalize` | `registered_undrained_writer_is_refused_by_production_finalizer` — **本批同样实测 PASS** | `REBOUND` |
| 9 | 注册 writer 排空后 finalize | `finalize` | `production_runner_finalizes_internal_v1_archive_after_registered_writer_drain` | `REBOUND` |
| 10 | V1-02c attempt 不匹配 | permit 借用 session attempt | 合法构造均经 `authorize(session)`，permit 借用同一 session；finalizer 再比对 pending attempt | `NOT_REACHABLE` |
| 11 | V1-02d destination 不匹配 | permit 借用 session destination | 同上；Storage 另比对 permit destination 与归一化请求路径 | `NOT_REACHABLE` |
| 12 | PREPARED 与取消仲裁 | journal commit + cancel | `internal_v1_prepared_journal_wins_cancellation_arbitration`、`persisted_cancel_commit_wins_race_and_writes_nothing` | `REUSED` |
| 13 | 取消 idempotent、终态不变 | executor cancel | `persisted_cancel_transitions_queued_to_cancelled_and_is_idempotent` | `REUSED` |
| 14 | 迟到结果被 fence | attempt fence | `running_cancel_signals_execution_and_fences_late_success`、`stale_attempt_cannot_overwrite_a_state_written_by_a_newer_attempt` | `REUSED` |
| 15 | directory commit 后 DB 失败 | failpoint | `production_runner_post_commit_database_failure_recovers_without_false_success` | `REBOUND` |
| 17 | PREPARED replay 原子性与终态幂等 | `prepared_replay_*` | `prepared_replay_is_atomic_under_sql_fault_and_terminal_convergence_is_idempotent`、`repeat_convergence_is_idempotent` | `REUSED` |
| 18 | manifest 写入中途崩溃可重复 | journal 写入 | `crashing_between_manifest_writes_leaves_a_repeatable_journal_row` | `REUSED` |
| 19 | rename 后 DB 冲突整体回滚 | 事务边界 | `post_rename_database_conflict_rolls_back_entire_recovery_transaction`、`v2_finalization_rejects_stale_attempt_and_rolls_back_archive_facts` | `REUSED` |
| 20 | COMMITTED 只读验证 | `verify_committed_internal_v1` | `recover_internal_rename_v2_committed_is_read_only`、`..._rejects_identity_mismatch` | `REUSED` |
| 21 | terminal 事件恰好一次 | `complete_terminal_for_attempt` | `storage_persistence_does_not_duplicate_transactional_state_events`、`persisted_completion_requires_downloaded_state_and_records_completion_once` | `REUSED` |
| 22 | shutdown / replacement join 活动 writer | shutdown + `replace_executor` | `executor_shutdown_joins_active_writer_before_successor_runs`、`runtime_replacement_joins_active_writer_then_runs_successor`（本批 delegated 重跑 PASS） | `REBOUND` |
| 23 | startup 恢复编排 | `recover_startup_with_v2` | `startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`、`startup_recovery_takes_over_no_journal_internal_v1_downloading_attempt` | `REUSED` |
| 24 | startup 崩溃 writer domain drain | `recover_writer_domain_set` | `startup_recovery_drains_crashed_writer_domains_before_decisions`、`successor_recovers_crashed_owner_domain` | `REBOUND` |
| 25 | startup CAS 失败 fail-closed | CAS 边界 | `startup_recovery_sqlite_successor_cas_loss_fails_closed_at_runner_boundary` | `REUSED` |
| 26 | unknown contract fail-closed | contract dispatch | `startup_recovery_fails_closed_on_unknown_contract_version`、`startup_v2_missing_journal_remains_fail_closed` | `REUSED` |
| 27 | maintenance 阻塞生产与恢复 | `WorkspaceGuard` | `maintenance_blocks_production_and_startup_recovery_before_mutation`、`live_production_session_blocks_maintenance_drain_until_finalize` | `REUSED` |
| 28 | batch / retry / resume 入口 | submission + batch | `v3_submission_uses_internal_recovery_for_storage_backed_archive_jobs`、`submits_and_reuses_an_active_job_for_duplicate_tweets` | `REUSED` |
| 29 | partial startup 保留 owner 并重试 | `StartupFailureOwner` | `production_factory_retains_a_real_stuck_cleanup_owner_and_resolve_aborts`、`startup_recovery_retries_successor_after_cas_owner_is_sigkilled` | `REBOUND` |
| 30 | 真实 aria2 / gallery-dl 二进制验收 | — | 本机无二进制，用户已延期 | `DEFERRED` |
| 31 | V2 enrollment 生产发布 | 未接线 | `test_only_v2_scheduler_fixture_*` | `DEFERRED` |
| 32 | Windows Job Object / NTFS / Named Pipe / GUI | — | — | `WINDOWS_OWNED` |

### 本矩阵对旧文档的更正

1. **V1-02b 不再是"未证明"。** 旧表称
   "V1-02a and lifecycle component tests do not prove this registered production
   case"，但两个针对该目标的 delegated 用例在源码中已存在，本批实测 PASS。
2. **V1-02c/d 不是"没测"，而是"合法构造不可达"。** 按 2026-11-08 已批准方式记为
   `NOT_APPLICABLE (no reachable path)` 并附构造证明，不是生产 `PASS`。
3. **#16 是真实进程崩溃证据**（子进程 abort 后重放），不是"返回错误即算崩溃"。

### 结论：矩阵未发现新的实现缺口

没有任何一项落在 `IMPLEMENTATION_MISSING` 或 `ASSERTION_MISSING`。

因此 **P2 的"补缺口"范围为空**：没有已证实的 C1 产品缺陷需要修复。这也意味着
本阶段不需要为了"凑完整"而新增生产代码。C1 Linux/shared 门禁的判定依据是**上表
加 §2/§3b 的实测重验**，不是新增实现。

这**不**等于"C1 全部完成"：#30 真实二进制、#31 V2 生产发布、#32 Windows 原生
仍未验收，必须按各自状态分别保留。



## 3b. Delegated 容器套件重验（B4 writer containment）

`linux-crash-replay-20261016.md` 记录过这些用例在其当时的 source 上通过，但 `dev`
此后前进了约 60 个提交（含 P0–P13 模块拆分与 `fcf7c0ef` 可见性收窄），按
[`validation-policy.md`](validation-policy.md) 的"PASS 仅在相关代码、依赖、平台契约与
环境不变时可复用"规则，这些结果对 `57a8a92c` 一律 `REVALIDATION_REQUIRED`。
本轮按同一 delegated scope 重跑，全部 **PASS**。

| 套件 | 命令 | 退出码 | 结果 |
|---|---|---:|---|
| `xarchive-workflow` `linux_containment` | `cargo test -p xarchive-workflow --test linux_containment -- --ignored --test-threads=1` | 0 | **2/2 PASS**（`real_owner_launches_and_drains`、`successor_recovers_crashed_owner_domain`） |
| `xarchive-download` `linux_contained` | `cargo test -p xarchive-download --test linux_contained -- --ignored --test-threads=1` | 0 | **3/3 PASS**（`contained_invalid_configuration_cleans_unlaunched_group`、`contained_rpc_ready_stops_and_removes_group`、`contained_startup_exit_cleans_group`） |
| `xarchive-sidecar-supervisor` `linux_contained` | `cargo test -p xarchive-sidecar-supervisor --test linux_contained -- --ignored --test-threads=1` | 0 | **6/6 PASS**（handshake 拒绝清组并应用环境、InternalV1 取消 drain writer domain、Drop 排空并移除 writer domain、reap 并移除组、超时 kill `setsid` 后代、正常握手与优雅退出） |

日志：`.tw2tg-tmp/agent-scratch/build-logs/goal-20261114/b/*.log`。

执行后资源核对：`tw2tg-att-*` 残留 cgroup **0**、`cgroup_launcher` 残留进程 **0**、
`xarchive_downloader` 残留进程 **0**。

**绑定范围**：这 11 项证明 Linux 侧 cgroup v2 委托、进程组终止、handler drain、
取消与拒收的共享行为。它们**不证明** Windows Job Object、NTFS 语义、Named Pipe
或任何 GUI 行为；Windows 对应项仍为 `NOT_RUN`。这些是**原语级与容器级**证据，
不替代 §2 的生产门（生产门证明的是 Desktop 真实执行链路，不是 supervisor 原语）。

## 4. D1 — InternalV1 生产链路接线（P3，2026-11-14）

**实现状态 `IMPLEMENTED`；验收状态：三项证据已补齐（2026-11-14 Act 批）。**
接线代码与真实生产链路、全关导出的崩溃/replay、注入式序列化失败三类证据均已取得，
详见下方"三项证据已补齐"小节。

本批**不启用 V2**，D1 落在当前 permit-bearing InternalV1 生产链路（用户已批准该范围）。

### 实现

| 环节 | 变化 |
|---|---|
| `execution_spec.rs` | `serialize_metadata` 不再是测试专用：移除 `#[allow(dead_code)]` 与"未接线"注释；它现在由 `finalize` 调用 |
| `archive.rs` | `PendingArchivePublication` 新增 `metadata_output`；`execute_archive_context` 不再丢弃该参数；`finalize` 在**任何文件变更或 PREPARED journal 之前**完成序列化，失败即 `ARCHIVE_METADATA_EXPORT_FAILED` |
| `SidecarArchiveRequest` | 新增 `metadata_exports: Option<&[(&str, Vec<u8>)]>`。`None` 保留历史行为（无条件写 `tweet.json` + `tweet.txt`）；`Some` 只写所选格式 |
| `internal_v1_publication.rs` | 原本**无条件**写 `tweet.json`/`tweet.txt`，现改由 `metadata_exports` 决定；写入前经 `FileStore::resolve_within` 防止路径穿越 |

**未修改 V1 manifest。** `ArchiveRecoveryManifest` 保持严格 `deny_unknown_fields` 且无导出字段，
因此没有引入共享 schema 变更。恢复仍只依赖内部 manifest 与媒体事实。

### 验证（全部为真实退出码）

| 检查 | 命令 | 结果 |
|---|---|---|
| 四组合 + COMMITTED | `cargo test -p xarchive-storage --lib internal_v1_attempt_publish_writes_exactly_the_selected_metadata_exports` | **PASS** 1/1 |
| 全关发布后内部校验收敛 | `... prepared_replay_converges_when_all_user_exports_are_disabled` | **PASS** 1/1 |
| Storage lib | `cargo test -p xarchive-storage --lib` | **180 / 0 / 0** |
| Desktop 选择传递 | `... production_factory_carries_the_v4_d1_export_selection_into_archive_execution` | **PASS** 1/1 |
| Desktop lib | `cargo test -p xarchive-desktop --lib -- --test-threads=1` | **367 / 0 / 13** |
| 其余模块 | workflow / core / protocol / download / sidecar-supervisor | 49 / 49 / 31 / 53 / 17，全部 0 失败 |
| 格式 | `cargo fmt --all -- --check` | exit 0 |

新测试断言的是**文件实际存在性**而非参数传递：四种开关组合下 `tweet.json` /
`tweet.yaml` 的有无必须严格跟随选择，且 `tweet.txt` 在 v4 选择下不得出现。

### 2026-11-14 Act 批更正：本节原先的三处过度表述

Documentation Gate 逐行重读测试源码后，以下三条**不能**再按原措辞引用：

| 原措辞 | 测试实际做什么 | 结论 |
|---|---|---|
| "`prepared_replay_converges_when_all_user_exports_are_disabled` 证明全关导出的 PREPARED replay 收敛" | 该用例用**空导出选择正常发布**，然后读回内部 manifest、校验 COMMITTED 目录树；**没有崩溃，也没有 replay** | 过度表述。它证明的是"提交后校验不依赖用户导出文件"，不是"崩溃后恢复收敛" |
| "`production_factory_carries_the_v4_d1_export_selection_into_archive_execution` 证明工厂链路传递选择" | 只做 v4 spec 解码 + 对一个 JSON 字面量序列化；**没有 runner、没有 finalizer、没有真实文件** | 过度表述。工厂→runner→finalize→落盘链路未被该用例覆盖 |
| "序列化失败不可能发布半成品" | 依据是 `archive.rs` 中"序列化在文件变更与 PREPARED journal 之前"的**源码顺序** | 目前只是推理，生产路径上没有注入失败证据 |

**仍然成立的部分：** 四组合的文件存在性断言（第一行）有效；`ArchiveRecoveryManifest`
（V1）确实未被修改；`metadata_exports` 的两态语义与 `resolve_within` 路径校验确实已实现。

### 2026-11-14 Act 批：三项证据已补齐（本节结论取代上表三行"过度表述"）

上表记录的是**补齐前**的真实覆盖。Act 批新增了对应证据，因此 D1 的验收状态从
`NOT_CLOSED` 改为**三项缺口均已关闭**：

| 缺口 | 新增证据 | 结果 |
|---|---|---|
| 真实生产链路（factory→runner→finalize→落盘） | `production_runner_writes_exactly_the_frozen_v4_export_selection`（Desktop，delegated cgroup）：四种组合各自以 v4 快照提交，走 `ArchiveJobSubmissionAdapter::prepare_with_metadata_settings` → `submit_persisted` → contained factory → `PendingArchivePublication::finalize`，断言最终目录中 `tweet.json`/`tweet.yaml` 的**存在性严格跟随冻结选择**、`tweet.txt` 永不出现、媒体正常落地、journal 到 `COMMITTED` | **PASS** |
| 全关导出的真实崩溃/replay | `exports_off_publish_crash_after_prepared_replays_without_user_exports`（Storage）：子进程在 `prepared` 与 `rename` 两个真实窗口 `abort()`，随后经既有 permit-bearing `replay_prepared_internal_v1` 收敛；崩溃点与 replay 后都断言三个用户导出文件均不存在 | **PASS** |
| 注入式序列化失败 | `serializer_failure_in_production_finalizer_publishes_nothing`（Desktop，delegated cgroup）：在真实 `finalize` 上注入序列化失败（`#[cfg(test)]` thread-local 守卫），断言 `ARCHIVE_METADATA_EXPORT_FAILED`、**无 PREPARED journal、无 archive facts、无最终目录、无 staging 导出文件**，且同一 pending 值仍是一次性的 | **PASS** |

**证据边界（不得外推）：**

- 前两项和第三项都由 `--exact --ignored` 在
  `systemd-run --user --scope -p Delegate=yes` 中真实执行，`running 1 test`
  且 `0 ignored`；执行后 `tw2tg-att-*` 残留为 **0**。
- 序列化注入开关是 `#[cfg(test)]` + thread-local，**不进入生产构建**；它没有
  改变任何成功路径的序列化结果。
- 全关导出的崩溃证据用的是 Storage 的 production publish 入口，**不是** Desktop
  runner 的子进程崩溃；Desktop 侧的该组合由第一项的正常路径覆盖。
- 仍未验证：真实 `aria2c`/`gallery-dl` 二进制，以及任何 Windows 行为。

### 边界

- 未启用 V2；`test_only_v2_scheduler_fixture_*` 仍为 `FAIL`（§3）。
- 未修改 V1 manifest、未新增 migration、未改变 permit 语义。
- Windows 原生输出路径与兼容性仍归 Windows Platform Owner（`NOT_RUN`）。

## 5. F2 / F3（P4，2026-11-14）

**F2 两处未闭合点已修复并取得运行证据；F3 在其未改动路径上有效。**
（2026-11-14 Act 批：取消可达性与"取消 vs 写入"仲裁均已修复，见下方小节。）

### F2 — 配置交换取消等待（实现缺口已补）

**缺口**（源码核对，非推测）：`apply_configuration_exchange` 在 drain 闭包内只有
30 秒 deadline，`lib.rs` 未注册任何取消命令，GUI 也没有取消入口。已批准合同要求
"等待可取消；取消后重新开放 admission 且配置不变"。

**修复**（`CROSS_PLATFORM_CHANGE_REQUIRED`：新增共享 IPC 命令）：

| 环节 | 变化 |
|---|---|
| `commands/records.rs` | 新增 `cancel_configuration_exchange`；`apply` 安装 `CancellationToken`，`drain` 闭包改为 `cancellation.is_cancelled() \|\| deadline`；`ConfigurationExchangeWait` 的 `Drop` 保证任何退出路径（含 panic）都释放 in-flight 槽位 |
| 互斥 | 已有 apply 在飞时返回 `another configuration exchange apply is active` |
| 锁顺序 | **等待期间不持有 `RuntimeState` 锁**，否则取消命令本身会被阻塞；drain 完成、写入前再取锁 |
| 取消语义 | drain 后若已取消则返回 `cancelled before writing`，durable/runtime 配置均不变；**原子 rename 之后的取消不回滚**，仍报告已应用修订 |
| `lib.rs` | 注册 `cancel_configuration_exchange` |
| `main.jsx` / `download-config-page.jsx` | `cancelExchange` 调用并有"取消等待"按钮（仅 busy 时出现） |

**验证**：

| 检查 | 结果 |
|---|---|
| `cancel_command_interrupts_an_in_flight_configuration_exchange`（Rust） | **PASS** |
| `configuration exchange apply wait is cancellable end to end`（Node 源码接线断言） | **PASS** |
| `desktop/test/record-import.test.mjs` | **5/5 PASS** |
| Desktop Node 全量 | **234 tests / 232 pass / 2 Windows-only skip / 0 fail** |
| Desktop lib | **368 / 0 / 13** |

### 2026-11-14 Act 批：两处未闭合点已修复并有运行证据

上表记录的是**修复前**的状态。Act 批确认了根因并修复：

| 问题 | 根因（源码确证） | 修复 | 证据 |
|---|---|---|---|
| 取消不可达 | `apply_configuration_exchange` 是同步 `#[tauri::command]`；Tauri 的 `Webview::on_message` → `Manager::run_invoke_handler` → `body_blocking`（`tauri-2.11.5/src/webview/mod.rs:1909`、`manager/mod.rs:471`、`tauri-macros-2.6.3/src/command/wrapper.rs:404`）**在 IPC 线程内联执行**命令体，阻塞的 drain 会占住该线程，取消命令无法被投递 | 改为 `async fn` + `tauri::async_runtime::spawn_blocking`，把阻塞等待移出 IPC 线程；`RuntimeState` 锁仅在取根路径与最终写入时短暂持有 | `cancel_command_reaches_a_blocked_maintenance_drain`：一个 active reader 占住 workspace，后台 drain 确实阻塞（`!is_finished()`），取消命令使其以错误返回 |
| 取消 vs 写入无仲裁 | drain 返回后只检查一次 `is_cancelled()`，随后取锁并无条件 `persist_preview` | `persist_preview` 新增 `cancellation: Option<&CancellationToken>` 参数，在**创建临时文件与 rename 之前**做最后一次检查；该点之前到达的取消不写入，之后到达的取消不会被报告为已取消 | `cancelled_apply_writes_nothing_and_a_later_apply_still_succeeds`：已取消时不产生配置文件、不改运行态；同一文档在未取消时仍能成功应用 |

语义变化（`CROSS_PLATFORM_CHANGE_REQUIRED`，IPC 命令签名由同步变异步，参数与返回
结构不变）：前端在 `apply` 的 reject 文本含 `cancelled` 时显示"已取消等待；本机配置
未改动"，取消按钮点击后显示"已请求取消"，不再把请求取消当作已取消。

**仍然成立的不夸张结论：** 取消成功与提交成功现在有确定的先后边界，但**没有**尝试
抢占一个已经进入 `rename` 的写入；那种情况仍报告已应用，符合批准合同。

**F2 验证结果：** `configuration_exchange` 相关 5 项测试 PASS；Node
`record-import.test.mjs` 5/5 PASS。

**2026-11-14 第二阶段更正：上面"两处未闭合点由运行证据关闭"的结论曾不足以支撑验收，
现已在同一阶段第二批修复并重新取证。** 第一阶段确实修好了两个真实缺陷（`async` +
`spawn_blocking`；`persist_preview` 的最后一次 token 读取），但复核发现三点：

| # | 当时的问题 | 后续修复与证据 |
|---:|---|---|
| 1 | `WorkspaceGuard::drain` 对"取消"与"真实争用"都返回 `LockError::Contended`，命令统一映射为 `workspace maintenance unavailable: ...`，不含前端匹配的 `cancelled`，取消会显示成普通失败 | 新增 `exchange_apply_wait`，在 token 与 deadline 都可见处分类：`cancelled` / `timed out waiting` / 真实 maintenance 错误。`WorkspaceGuard`/`LockError` 保持不变。`cancel_command_reaches_a_blocked_maintenance_drain` 断言错误含 `cancelled` 且**不含** `workspace maintenance unavailable`，并验证取消后 admission 重新开放 |
| 2 | 没有测试执行 `apply_configuration_exchange`：可达性用例手工写入全局 token 并自行起线程 `drain`，另一用例直接调 `persist_preview` | `exchange_apply_wait` + `exchange_apply_persist` 成为命令本身调用的两个函数；`apply_flow_cancelled_after_the_wait_writes_nothing_and_still_applies_later` 依次驱动两者：取消后不建配置文件、不改运行态，随后同一文档仍可应用 |
| 3 | 两个新增 Rust 用例共用全局槽位 `CONFIGURATION_EXCHANGE_CANCELLATION`，正确性依赖调度 | 新增 `EXCHANGE_SLOT_TEST_ORDER` 串行化占用槽位的用例，`hold_slot` 在检测到泄漏时直接失败 |

另外两处表述已更正：

- **过滤器覆盖范围。** `cargo test -p xarchive-desktop --lib configuration_exchange`
  按名称过滤，不匹配 `cancelled_apply_writes_nothing_and_a_later_apply_still_succeeds`。
  本批改用显式双名选择器 `configuration_exchange configuration_exchange_cancellation_tests`
  （6/6），并按名称单独运行该用例（1/1）。
- **合同措辞。** 最后一次 token 读取发生在创建临时文件之前，也发生在为持久化取
  `RuntimeState` 锁之前。边界是"提交接纳点"，不是"同一把锁下的 cancel-versus-rename
  原子仲裁"。

**仍未验证（不得冒充通过）：** 没有测试执行真实 Tauri IPC 往返或 GUI 交互；Rust
用例驱动的是生产函数而非 IPC 包装层。该缺口归 Windows/desktop 集成队列。

结论：F2 在 **Linux 范围**内实现 `IMPLEMENTED`、验收 `PASS`；Windows 原生 apply 仍归
Windows Owner（`NOT_RUN`）。F3 的既有覆盖在本批重跑通过（Storage 9/9、Core 5/5），
未受影响路径未重写。

| 已证明 | 未证明 |
|---|---|
| 取消命令能翻转一个已被放入 in-flight 槽位的 token | **取消命令能否在 apply 阻塞期间真正被执行** |
| `ConfigurationExchangeWait` 的 `Drop` 会清空槽位 | 端到端"等待→取消→不写配置→重新开放 admission" |
| 命令已在 `lib.rs` 注册 | 取消落在"检查之后、写入之前"窗口时的行为 |
| GUI 在 busy 时渲染"取消等待"按钮 | 该按钮在真实 Tauri IPC 下能打断阻塞调用 |

具体两处未闭合点：

1. **可达性。** `apply_configuration_exchange` 是同步 `#[tauri::command]`；本地
   `tauri-macros 2.6.3` 的包装默认 `ExecutionContext::Blocking`
   （`wrapper.rs:50`、`:248-253`、`body_blocking`）。释放业务互斥锁并不等于证明
   IPC 层可并发执行取消命令。
2. **取消 vs 写入。** 代码在 drain 返回后只检查一次 `cancellation.is_cancelled()`，
   随后取得 `RuntimeState` 锁并**无条件**调用 `persist_preview`。落在这个窗口内的
   取消会被静默忽略，而写入仍会成功。

另注：Rust 用例是**手工插入 token** 后断言其被翻转，并未运行一次真实的 apply；
Node 用例是**源码匹配**。因此 `status` 应记为"机制存在、边界未验证"，而不是
"端到端可取消已 PASS"。

结论：F2 实现 `IMPLEMENTED`，验收 `NOT_CLOSED`，下一步工作列入 Batch 2。

### F3 — 记录交换（复用既有实现，无重写）

F3 已有 `cancel_record_import`，因此不存在 F2 那样的取消缺口。当前源码下重跑既有
覆盖，全部通过：

| 套件 | 结果 |
|---|---|
| Storage `record_import`（幂等 reimport、混合冲突整体回滚、SQL 失败原子性、ledger 迁移、归档 final path 冲突） | **9 / 9 PASS** |
| Core `record_exchange`（JSON↔YAML round-trip 同模型、未知 kind/schema/重复键拒绝、身份去重、两种 kind 区分） | **5 / 5 PASS** |

这些测试覆盖的是已批准合同的核心条款。**未在本批重做 ZIP/7z**（需单独批准）。

### 边界

- F2 的原生 Windows 应用（GUI 对话框、文件选择器）仍归 Windows Owner，`NOT_RUN`。
- 未修改 V2 门禁；未安装任何依赖；未改变维护屏障语义。



## 6. `NOT_RUN`（不得记 PASS）

- 真实 `aria2c` / `gallery-dl` 二进制验收：本机无二进制，且用户已延期。
  `production_real_tools_publish_local_media`、`production_real_aria2_drains_recorded_writer`、
  `production_real_aria2_transfer_and_refresh_drain_recorded_writer`、
  `production_runner_registers_and_drains_real_aria2_before_internal_v1_publish`
  保持 `NOT_RUN`。
- Windows 原生全部项目：Windows Platform Owner。
- 四项 R7 delegated 用例：绑定 `614e31b7`，本批未重跑，也没有用本轮结果替代。
