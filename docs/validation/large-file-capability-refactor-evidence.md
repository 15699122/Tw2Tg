# 大文件治理与业务能力模块化验证证据

本文件仅记录新计划 P0–P13 的验证，不将 InternalV1 R5–R7 历史结果重绑定到新源码。范围见[计划](../development/large-file-capability-refactor-plan.md)。状态、证据字段、能力判断、重试及复用规则统一引用[验证政策](validation-policy.md)。

## 验证槽位

| Round | Source SHA / dirty identity | Scope / selected checks | Result / evidence |
|---|---|---|---|
| P0 initial documentation gate | `649435430fcb2b5576e976105dc93494a2a65051` + dirty docs-only diff; protected backup SHA-256 `652191354b45e9f5dcd368058b638d7cff35f967f1b4bee1007821e5d4be42db` unchanged | Re-executed via exit-capturing Python harness: `node scripts/docs-audit.mjs` exit 0 (153 tracked Markdown; 0 dead links/unlinked docs/stale branch links); `git --no-pager diff --check` exit 0; diff scope review: documentation paths only | PASS for P0 documentation gate. Source code not changed; index empty. Earlier shell completion capture anomaly is superseded for these exact checks by the independently captured exit-0 rerun. |
| P1 inventory and module baselines | `649435430fcb2b5576e976105dc93494a2a65051` + docs-only dirty diff（3 改 2 新）；index 空；backup SHA-256 `652191354b45e9f5dcd368058b638d7cff35f967f1b4bee1007821e5d4be42db` 不变 | (1) 三个 Cargo module baseline 各 exit 0：`xarchive-telegram` 54 passed/0 failed/0 ignored、`xarchive-storage` 178/0/0、`xarchive-desktop` 365/0/13 ignored；真实日志 `.tw2tg-tmp/agent-scratch/build-logs/large-file-p1-{xarchive-telegram,xarchive-storage,xarchive-desktop}.log`。(2) 22 项文件 `wc -l` 实测与计划 §4 表一致。(3) 台账 23 条：22 项源文件全覆盖 + 1 条 Storage 父模块（`tests/mod.rs` 403 行，P3 硬约束）。(4) 本轮重执行门禁：`node scripts/docs-audit.mjs` exit 0（`.tw2tg-tmp/agent-scratch/docs-audit-p1-final.log`）、`node scripts/check-architecture.mjs` exit 0（`.tw2tg-tmp/agent-scratch/arch-p1-final.log`）、`git --no-pager diff --check` exit 0；`git status --porcelain` 仅 3 个已修改文档 + 2 个新文档 + 受保护 backup，index 空。 | PASS。缺陷保留记录：上一轮曾在未读完 `crates/xarchive-telegram/src/lib.rs` 的情况下写入“全文已读”与按区间估算的分组（“约 20 项”“从约 27 左右开始”），并把该条误置于「边界与待办」之下。本轮已删除该失实文本，改为源码逐行定位的 47 项测试名与行号（第 22 条），并补齐 `desktop/src-tauri/src/telegram_send.rs` 的 30 项（第 23 条）。13 个 ignored 的分布已定位：`executor.rs` 11 + `archive.rs` 2，与基线 365/0/13 吻合。P1 门禁成立；P2 可在本游标下开始。 |
| P2 Telegram test organization | `649435430fcb2b5576e976105dc93494a2a65051` + P2 dirty diff（源码 1 改 6 新，文档 2 改）；index 空 | (1) 迁移前基线：`cargo test -p xarchive-telegram --lib -- --test-threads=1` exit 0，54 passed/0 failed/0 ignored（`.tw2tg-tmp/agent-scratch/build-logs/p2-baseline-telegram.log`）；迁移后同命令 exit 0，54/0/0（`p2-after.log`）。(2) 名称集合逐项比对（`p2-name-compare.txt`）：before 54 / after 54，missing `[]`、new `[]`、status changed `[]`。(3) 66 个顶层项（47 测试 + 19 helper/fixture）逐函数体 token 比对（`p2-fidelity.txt`、`p2-fidelity2.txt`）：MISSING `[]`、NEW `[]`，仅 2 处 rustfmt 尾逗号折叠，语义等价。(4) 公共 API 23 条 `pub use/trait/enum/struct/fn/const` 逐行 diff 为空（PUBLIC_API_IDENTICAL）。(5) `lib.rs` 1–215 行生产部分与迁移前字节一致（PRODUCTION_IDENTICAL）。(6) 下游 `cargo test -p xarchive-storage --lib` exit 0，178/0/0（`p2-storage-downstream.log`）。(7) `cargo fmt --all -- --check` exit 0、`git --no-pager diff --check` exit 0、`node scripts/docs-audit.mjs` exit 0（`p2-docs.log`）。 | **PASS（代码与测试）；架构门禁 BLOCKED 待授权**。`node scripts/check-architecture.mjs` exit 1，唯一 finding 为 `baseline-delta crates/xarchive-telegram/src/lib.rs: lines changed 2691 -> 217 (-2474)`，属计划 §5 规定的授权闸口，未擅自 `--write-baseline`。6 个新文件因未被 git 跟踪，检查器 `git ls-files` 尚未纳入（计划 §5 已注明）。Setup 更正：`transport.rs` 的 `request_payload`(263)/`json_error`(330) 实为 `pub(crate)`，非私有；本次迁移未扩大任何生产可见性。 |
| P3 Storage test organization | `649435430fcb2b5576e976105dc93494a2a65051` + P3 dirty diff（Storage 测试 2 删 + 2 目录新增）；index 空 | (1) 迁移前 `cargo test -p xarchive-storage --lib -- --test-threads=1` exit 0，178 passed/0 failed/0 ignored；迁移后同命令 exit 0，178/0/0（`p3b-after.log`）。(2) 名称集合逐项比对（`p3b-verify.txt`）：before 178 / after 178，MISSING `[]`、NEW `[]`、STATUS CHANGED `[]`。(3) 逐函数体 token 比对：`archive_recovery` 28 项（6 helper + 22 测试）零差异（`p3a-fidelity.txt`）；`archive_service` 48 项零差异（`p3b-verify.txt`）。(4) 两个崩溃自执行测试的 `--exact` 选择器实测各恰选 1 项、177 filtered out：`tests::archive_recovery::v1_publish_crash_windows_converge_through_production_prepared_replay`、`tests::archive_service::v2_forward_publish_crash_windows_converge_through_recovery_replay`。(5) `cargo fmt --all -- --check` exit 0、`git --no-pager diff --check` exit 0、`node scripts/docs-audit.mjs` exit 0（`p3-docs.log`）。 | **PASS（代码与测试）；架构门禁 BLOCKED（非产品缺陷）**。`node scripts/check-architecture.mjs` 本次以 `ENOENT` 崩溃而非给出 finding：检查器用 `git ls-files` 枚举，而 `archive_recovery.rs`/`archive_service.rs` 在工作树中已删除但未暂存，仍留在索引里（`git status` 显示 ` D`），于是 `readFileSync` 打不开。P2 未触发此崩溃，因为 P2 只新增文件、`lib.rs` 仍存在。解除需暂存/提交授权（与 P2 的 baseline reseal 属同一授权点）。ignored 数量仍为 0；未放宽任何断言。 |
| P4 Executor test organization | `649435430fcb2b5576e976105dc93494a2a65051` + P4 dirty diff（`executor.rs` 1 改 + 2 新模块 + 6 个新测试组文件）；index 空；backup SHA-256 `652191354b45e9f5dcd368058b638d7cff35f967f1b4bee1007821e5d4be42db` 不变 | (1) 迁移前 `cargo test -p xarchive-desktop --lib -- --test-threads=1` exit 0，365 passed/0 failed/13 ignored（`build-logs/p4-baseline-desktop.log`）；P4a 后同命令 exit 0，365/0/13（`p4a-after.log`）；P4b 后同命令 exit 0，365/0/13（`p4b-after.log`）。(2) 名称集合逐项比对（`p4a-verify.txt`、`p4b-verify.txt`）：两轮均 before 378 / after 378 全路径名，MISSING `[]`、NEW `[]`、STATUS CHANGED `[]`，ignored 恒为 13。(3) 逐项 token 保真：P4a 161 顶层项（124 + 38 减去模块外壳）MISSING/NEW 均 `[]`，仅 23 处 rustfmt 尾逗号折叠与 1 处冗余闭包块折叠（`p4a-fidelity2.txt`、`p4a-fidelity5.txt`）；P4b 124 项 `TOKEN DIFFS: 0`（`p4b-fidelity.txt`）。(4) 两条自启动 `--exact` 选择器实测各恰选 1 项、377 filtered out（其中一条为既有 ignored）：`executor::tests::startup_recovery_retries_successor_after_cas_owner_is_sigkilled` 1 passed、`executor::tests::startup_recovery_drains_live_descendant_after_owner_sigkill` 1 ignored。(5) 编译零警告；`cargo fmt --all -- --check` exit 0、`git --no-pager diff --check` exit 0；下游 `cargo test -p xarchive-storage --lib` exit 0，178/0/0。 | **PASS（代码与测试）；架构门禁 BLOCKED（非产品缺陷，待授权）**。`node scripts/check-architecture.mjs` 仍以 `ENOENT` 崩溃于 `crates/xarchive-storage/src/tests/archive_recovery.rs`，与 P3 同一未暂存删除根因，非 P4 引入。P4 新增的两个模块声明保持 `executor::tests` / `executor::session_boundary_tests` 路径不变，`session_boundary_tests` 的 `#[cfg(all(test, unix))]` 与 `mod tests` 的 `#[cfg(test)]` 属性语义等价保留；23 个共享夹具与 2 个自启动选择器测试留在 `mod.rs`，子模块经 `use super::*;` 继承，未改动任何可见性关键字。 |

| P5–P8 production responsibility extraction | 待填写 | 每个微批的符号/兼容入口审计及受影响模块验证 | NOT_RUN；逐微批记录 |
| P9 Executor runtime | 待填写 | ADR-015、shutdown/join/ownership、InternalV1/delegated production paths | NOT_RUN |
| P10–P11 Storage service/recovery | 待填写 | PREPARED/COMMITTED、事务、CAS/attempt fence、replay/finalization | NOT_RUN |
| P12 remaining ten files | 待填写 | 十项逐一处置及对应模块验证 | NOT_RUN |
| P13 final integration | 待填写 | workspace、fmt、npm、architecture、docs、diff gates | NOT_RUN |

## P1 文件台账（已完成，2026-11-09 第二轮）

阅读方式：`functions.read_files` 单范围分页（同一文件多个范围会在同一响应内合并截断，因此每次请求只取一个窗口并逐段复核边界）。“覆盖”表示实际读到并核对的连续行段。

### 1. `crates/xarchive-storage/src/tests/archive_recovery.rs`（2748 行，P3）

- 覆盖：1–390、391–780、781–1170、1171–1560、1561–1950、1951–2340、2341–2748，外加补齐窗口 90–300、186–240、490–700、880–1000、1001–1080、1255–1360、1361–1480、1649–1800、1801–1950、2050–2242、2436–2652、2520–2652。全文无未读区间。
- 头部：文件级 `//!` 文档；`use super::*;`、`use crate::{ArchiveService, FileStore};`、`use sha2::Digest;`、`use xarchive_core::JobState;`。无内部属性、无 `#![allow]`。父模块可见性是硬依赖：`Database`、`ArchiveRecoveryManifest`、`ArchiveRecoveryManifestV2`、`ArchiveRecoveryMediaFact`、`ArchiveRenamePlan`、`ArchiveRecoveryPhase`、`ArchiveCancellationDecision`、`ArchiveRecoveryDispatch`、`ArchiveRecoveryContract`、`StorageError`、`Path`、`ArchiveMetadata` 全部来自 `super::*`，本文件未声明这些导入。
- 私有 helper（6 个，均为文件内非 `pub` `fn`）：`test_candidate_metadata`（8–29）、`manifest`（30–49）、`replay_crashed_v1_window`（52–189）、`repeat_convergence_is_idempotent`（192–213）、`assert_attempt_fenced_terminal_convergence`（216–233）、`run_v1_crash_window_child`（237–390，返回类型 `!`）。`run_v1_crash_window_child` 在 389 行 `std::process::abort()`；`replay_crashed_v1_window` 依赖 `xarchive_workflow`（`AttemptIdentity`、`LinuxCoordinator`、`ExecutionReservation`、`linux_containment::LinuxContainment`、`authorize_prepared_recovery`、`RecoveryJournalIdentity`、`RecoveryJournalPhase`）。
- 子进程与 env（迁移硬约束）：测试 `v1_publish_crash_windows_converge_through_production_prepared_replay`（1426）定义 `TW2TG_V1_PUBLISH_CRASH_ROOT`／`TW2TG_V1_PUBLISH_CRASH_WINDOW`，并在 1452–1461 用 `current_exe()` + `--exact` + 硬编码 `"tests::archive_recovery::v1_publish_crash_windows_converge_through_production_prepared_replay"` 重启自身。搬迁到 `src/tests/archive_recovery/` 后该字符串必须同步改写，否则选择器选中 0 项。`crash.ready` 内容为 `"1"`（386）。
- 断言/证据性质：claim/CAS 7 项、dispatch 1 项、V1 replay/终态 3 项、crash/cancel 3 项、V2 journal/finalization 5 项、legacy 恢复 3 项。分组必须按方法逐项归属，不按行区间切分。
- 私有访问（生产侧符号，P11 迁移时不得扩大可见性）：`ArchiveService::{preview_sidecar_archive_attempt, complete_sidecar_archive_attempt_for_test, replay_prepared_internal_v1, converge_committed_internal_v1, recover_internal_archive, recover_internal_rename_v2}`、`service.database`／`service.files`、`Database::{open, open_in_memory, connection, execute_batch, insert_tweet, insert_media, count_job_events, job_state, archive_attempt, claim_archive_attempt, claim_archive_job, claim_internal_v1_successor, begin_archive_attempt, startup_recovery_dispatch, create_archive_recovery_manifest, verify_archive_recovery_manifest, archive_recovery_journal, archive_commit_started, decide_archive_cancellation, prepare_archive_recovery_v2, transition_archive_recovery_v2, finalize_archive_recovery_v2, archive_recovery_manifest_v2, tweet_archive_facts, prepare_telegram_archive_intent, telegram_archive_intent}`、`FileStore::{with_staging_root, sha256, staging_dir, create_attempt_staging_dir}`、`crate::archive_service::{PendingRename, RENAME_PLAN_FILE}`、`crate::{ArchiveRecoveryFailpoint, BatchOutputSettings, SidecarArchiveRequest, TelegramArchiveIntentRecord, BatchNamingMode}`。
- 磁盘 fixture 约定：`.xarchive-recovery.json`、`_staging/<job>/...`、`Tweets/<id>/...`；`/tmp` 下按 pid+nanos 生成唯一根。

### 2. `crates/xarchive-storage/src/tests/archive_service.rs`（2309 行，P3）

- 覆盖：1–400、401–800、801–1200、1201–1600、1601–2000、2001–2309，外加补齐窗口 504–640、900–1105、1288–1505、1702–1912。全文无未读区间。
- 头部：`use super::*;`、`use rusqlite::OptionalExtension;`（仅 2 行，无文件级文档注释）。完全依赖父 `tests` 模块的共享 fixture：`temp_root()`、`outbox_entry()`、`BOT_A`、`OUTBOX_NOW`、`prepared_v2_rename_fixture()`、`unprepared_v2_publish_inputs()`、`journal_snapshot()`；`build_archive_metadata` 是 crate 生产函数而非测试 helper。**这些 helper 不在本文件内定义**，P3 必须先冻结父 `tests/mod.rs` 中的定义再分组。
- 子进程与 env：测试 `v2_forward_publish_crash_windows_converge_through_recovery_replay`（1861）定义 `TW2TG_P5_PUBLISH_CRASH_ROOT`／`TW2TG_P5_PUBLISH_CRASH_WINDOW`，`--exact` 硬编码 `"tests::archive_service::v2_forward_publish_crash_windows_converge_through_recovery_replay"`（1917）；`crash.ready` 写入 `manifest.tweet_row_id.to_string()`（1899），子进程 1903 行 `std::process::abort()`。窗口集合 6 项：`unprepared`、`prepared`、`temporary`、`final`、`directory`、`committed`。
- 48 个 `#[test]`、`#[ignore]` 0 个、`#[cfg]` 门控 2 处：`#[cfg(unix)]` 包住测试 `rejects_sidecar_intermediate_symlink_escape`（507–509，Windows 上不编译），以及测试 `staging_job_id_with_iso_timestamp_is_resolvable` 内部的 `#[cfg(windows)]` 断言分支（49–56）。搬迁时两者必须逐字保留。
- 测试名与行号（顺序即源码顺序）：5 `persists_transition_and_event`；23 `protects_files_and_commits_staging`；43 `staging_job_id_with_iso_timestamp_is_resolvable`；79 `completes_local_archive_and_writes_portable_metadata`；170 `recovers_staging_archive_from_persisted_metadata`；237 `archive_intent_journal_recovers_staging_after_database_reopen`；241 `archive_intent_journal_recovers_renamed_archive_after_database_reopen`；245 `internal_v1_telegram_intent_refuses_recovery_without_runner_permit`；325 `archive_intent_journal_rejects_changed_facts_and_invalid_transitions`；420 `outbox_batch_rolls_back_on_conflict_and_replays_idempotently`；457 `converts_sidecar_files_using_actual_size_and_hash`；487 `rejects_sidecar_path_escape`；509 `rejects_sidecar_intermediate_symlink_escape`；537 `accepts_numeric_tweet_id_from_json`；547 `builds_archive_metadata_with_author_and_relationship_fields`；576 `completes_archive_directly_from_sidecar_result`；664 `attempt_archive_preview_uses_attempt_payload_not_legacy_staging`；714 `attempt_archive_preview_binds_metadata_to_transfer_relative_path_and_hash`；779 `attempt_archive_preview_never_falls_back_to_legacy_staging`；826 `attempt_archive_preview_rejects_reported_path_escape`；864 `v2_candidate_contract_preserves_legacy_business_payload_and_rejects_extensions`；955 `v2_candidate_contract_refuses_unsafe_urls_before_any_prepared_writes`；983 `v2_prepared_candidate_failures_preserve_journal_and_staging`；1076 `v2_rename_recovery_replays_planned_to_committed_from_disk`；1151 `v2_rename_recovery_is_idempotent_across_repeated_replays`；1172 `v2_rename_recovery_preserves_an_unchanged_media_filename`；1208 `v2_manifest_rejects_media_path_without_matching_rename_plan`；1218 `v2_unchanged_filename_recovery_converges_across_temporary_phase_crash`；1256 `v2_rename_recovery_converges_when_files_moved_but_journal_lagged`；1285 `v2_rename_recovery_rejects_a_mismatched_target_without_finalizing`；1310 `v2_rename_recovery_fails_closed_on_a_stale_attempt`；1332 `v2_rename_recovery_fails_closed_when_the_journal_is_missing`；1359 `v2_rename_recovery_finalizes_when_committed_directory_exists_but_journal_lagged`；1414 `v2_rename_recovery_rejects_a_committed_directory_with_swapped_media`；1445 `v2_download_event_failure_rolls_back_publication_facts_and_replays_once`；1627 `v2_export_collision_refuses_before_prepared_or_publication`；1667 `v2_historical_export_switches_do_not_control_internal_recovery`；1724 `v2_forward_publish_composes_prepare_rename_commit_and_fenced_finalize`；1788 `d1_optional_exports_publish_and_recover_without_user_files`；1861 `v2_forward_publish_crash_windows_converge_through_recovery_replay`；1999 `v2_committed_replay_rejects_swapped_bytes_without_mutation`；2044 `v2_committed_replay_rejects_missing_database_facts`；2088 `v2_committed_replay_rejects_changed_database_facts_without_mutation`；2163 `v2_forward_publish_rejects_destination_collision_without_finalizing`；2198 `v2_forward_publish_fails_closed_on_a_stale_attempt`；2225 `v2_forward_publish_keeps_prepared_commit_winning_over_cancellation`；2253 `v2_forward_publish_rejects_a_rolled_back_candidate_without_finalizing`；2286 `v2_forward_publish_duplicate_completion_replays_read_only`。
- SQL/文件系统故障注入手法（P3 必须原样保留）：`CREATE TRIGGER reject_download BEFORE INSERT ON events ...`（1449）、`CREATE TRIGGER reject_terminal ...`（1520）、`DELETE FROM events ...`+`UPDATE jobs SET state='DOWNLOADED'`（1588）、直接 `UPDATE archive_recovery_v2_candidate SET metadata_json=?, metadata_sha256=?`（932、1023）、`UPDATE jobs SET attempt_count = attempt_count + 1`（1316、2083）、`DELETE FROM media WHERE tweet_id = ?1`（2061）、`fs::rename` 手工模拟 half-rename（1265）、`files.commit_attempt_staging`（1430）。
- 精确计数/文案断言（不得改写）：`count_job_events("job-1") == 5`（1780–1783，注释说明为三次 fixture 转移加 DOWNLOADING→DOWNLOADED 与下载事件）、`"a stale attempt identity must not publish"`（2215）、`"atomic finalization must retire the temporary business metadata copy"`（1765）、`"export switches never suppress the prepared business candidate"`（1696）、`"failed finalization retains the exact prepared business candidate"`（1500）。

- 已确认测试名（`mod session_boundary_tests`，`#[cfg(all(test, unix))]`）：`the_boundary_is_send_so_it_can_move_into_the_scheduler_thread` 7958、`cancellation_that_wins_before_prepare_is_not_replaced_by_execution_failure` 7964、`factory_failure_finalizes_the_claim_and_releases_admission_and_scopes` 8030、`archive_writer_drain_failure_is_terminal_and_never_completes_archive` 8084、`stale_attempt_success_records_no_completion_event_or_final_state` 8169、`fenced_publication_records_download_completed_and_skips_duplicate_complete` 8329、`non_fenced_execution_records_download_completed_and_completes` 8419、`inline_runner_operation_is_static_runs_on_runner_and_drops_before_admission` 8504、`executor_shutdown_joins_active_writer_before_successor_runs` 8578、`runtime_replacement_joins_active_writer_then_runs_successor` 8725、`inline_runner_operation_error_drops_capture_before_releasing_admission` 8882、`session_boundary_execute_on_runner_runs_session_on_named_runner` 8921、`startup_cleanup_pending_owner_crosses_the_boundary_and_resolves_same_owner` 8961、`inline_runner_operation_drops_owner_before_admission_on_success` 9057、`inline_runner_operation_panic_closes_reply_and_releases_admission` 9085、`a_session_holds_both_scopes_across_execution_and_finalization` 9133、`a_live_peer_makes_the_run_reschedulable_without_any_durable_write` 9206、`a_lost_claim_is_reschedulable_releases_scopes_and_persists_no_failure` 9276、`no_journal_internal_v1_successor_cas_loss_does_not_start_or_fail_successor` 9334、`a_saturated_runner_is_rejected_before_any_claim` 9444、`complete_terminal_for_attempt_is_atomic` 9575、`complete_terminal_for_attempt_is_idempotent` 9643、`complete_terminal_for_attempt_rejects_stale_attempt` 9715、`complete_terminal_for_attempt_requires_downloaded_state` 9794。
- P4 分组依据（已读区间证据）：`execution_spec_*` 系列（2290–2557）覆盖 v1/v2/v3/v4 解码、未知版本拒绝、unknown fields 拒绝、`policy_version` 不匹配拒绝、argv 字节级往返、诊断不泄露值/版本；`persisted_*`／`startup_recovery_*` 系列覆盖 CAS 输赢、stale attempt 围栏、`QueueFull` 非失败可重调度、启动恢复四类分流；`session_boundary_tests` 覆盖许可、双 scope 持有、shutdown join、panic 关闭回复并释放许可。

- 已确认测试名（`mod tests` 续，来自函数名扫描）：`startup_recovery_sqlite_successor_cas_loss_fails_closed_at_runner_boundary` 3694、`startup_recovery_retries_successor_after_cas_owner_is_sigkilled` 3834、`startup_recovery_drains_crashed_writer_domains_before_decisions` 4072、`startup_recovery_drains_live_descendant_after_owner_sigkill` 4183、`startup_recovery_fails_closed_on_unknown_contract_version` 4377、`production_factory_carries_v2_output_snapshot_into_archive_execution` 4420、`startup_v2_missing_journal_remains_fail_closed` 4444、`production_runner_rejects_uncontained_internal_v1_writer_before_publish` 4495、`startup_recovery_runs_new_internal_v1_job_before_first_attempt` 4573、`production_runner_post_commit_database_failure_recovers_without_false_success` 4668、`production_runner_finalizes_internal_v1_archive_after_registered_writer_drain` 4902、`production_runner_registers_and_drains_real_aria2_before_internal_v1_publish` 5149、`linux_launch_descriptor_is_explicit_bounded_and_fail_closed` 5299、`startup_recovery_classifies_legacy_without_recovery_facts_as_incomplete` 5342、`startup_recovery_classifies_legacy_without_execution_spec_consistently` 5382、`startup_candidate_enumeration_failure_is_fatal_and_does_not_mutate_jobs` 5420、`startup_recovery_completes_valid_internal_archive_and_repeats_idempotently` 5450、`archive_submission_adapter_preserves_sync_job_identity_and_reuse_contract` 5649、`archive_submission_adapter_rejects_invalid_browser_requests_before_submit` 5676、`persisted_recovery_updates_only_recoverable_candidates` 5695、`persisted_cancel_transitions_queued_to_cancelled_and_is_idempotent` 5737、`persisted_cancel_does_not_change_terminal_job_or_record_an_event` 5768、`session_execution_error_preserves_prepared_job_for_startup_replay` 5830、`maintenance_blocks_production_and_startup_recovery_before_mutation` 5870、`live_production_session_blocks_maintenance_drain_until_finalize` 5929、`persisted_cancel_refuses_before_stopping_when_commit_started` 6067、`persisted_cancel_commit_wins_race_and_writes_nothing` 6108、`persisted_shutdown_interrupts_active_jobs_once_and_skips_terminal_jobs` 6142、`interrupt_persisted_is_separate_from_worker_shutdown` 6212、`persisted_completion_requires_downloaded_state_and_records_completion_once` 6234、`persisted_completion_rejects_queued_job_without_writing_state_or_event` 6293、`commit_recovery_decision_distinguishes_resume_complete_failure_and_skip` 6317、`commit_recovery_decision_detects_missing_archive_for_complete_job` 6367、`commit_recovery_does_not_rewrite_unrelated_job_states` 6383、`apply_commit_recovery_resumes_staging_without_fabricating_completion` 6397、`apply_commit_recovery_completes_existing_final_directory` 6448、`apply_commit_recovery_marks_missing_commit_as_failed_and_records_error` 6481、`apply_commit_recovery_skips_complete_job_without_duplicate_event` 6540、`recover_commit_persisted_uses_injected_directory_facts_and_matches_snapshot` 6598、`recover_commit_persisted_rejects_facts_that_do_not_match_snapshot` 6628、`storage_recovery_facts_preserve_job_summary_state_and_injected_directories` 6657、`recover_commits_persisted_keeps_results_sorted_and_isolates_fact_errors` 6705、`recover_commits_persisted_skips_terminal_jobs_from_candidate_scan` 6757、`recover_commits_persisted_mixes_actions_and_preserves_each_job_event_order` 6806、`storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution` 6910、`storage_completion_persists_state_then_job_completed_event` 7048、`storage_persistence_adapter_matches_job_repository_contract` 7104、`database_factory_opens_a_job_context_without_runtime_state` 7138、`storage_persistence_preserves_state_transition_and_created_event` 7152、`executor_lifecycle_events_match_existing_archive_job_event_types` 7195、`storage_persistence_does_not_duplicate_transactional_state_events` 7242、`job_summary_projection_preserves_executor_fields_without_synthesizing_storage_fields` 7283、`storage_summary_and_event_queries_preserve_error_and_nullable_payload_contracts` 7311、`storage_persistence_recovery_scans_active_jobs_and_skips_terminal_jobs` 7338、`persisted_recovery_records_the_actual_source_state_for_queued_and_interrupted_jobs` 7408、`executor_events_preserve_state_transition_order` 7444、`sidecar_crash_maps_to_safe_failure_and_core_job_event` 7465。

- 已确认测试名（`mod tests`，按源码顺序）：`production_factory_contained_handshake_and_drop_remove_writer_group` 937、`production_factory_retains_a_real_stuck_cleanup_owner_and_resolve_aborts` 1031、`execution_spec_v1_decodes_historical_request_json_without_rewriting` 1130、`production_real_tools_publish_local_media` 1145、`sidecar_internal_v1_hello_ready_and_shutdown_exits` 1338、`sidecar_internal_v1_extract_then_cancel_aborts_cleanly` 1402、`production_real_aria2_drains_recorded_writer` 1511、`production_real_aria2_transfer_and_refresh_drain_recorded_writer` 1603、`test_only_v2_scheduler_fixture_publishes_and_recovers_protocol_bytes` 1714、`v2_execution_settings_are_preserved_by_decoding` 2290、`execution_spec_v1_rejects_invalid_json_and_v2_rejects_request_identity_mismatch` 2309、`execution_spec_v2_decodes_request_and_immutable_output_snapshot` 2327、`submission_adapter_persists_the_captured_output_settings_snapshot` 2349、`v3_submission_uses_internal_recovery_for_storage_backed_archive_jobs` 2369、`execution_spec_rejects_unknown_version_explicitly` 2393、`execution_spec_v3_decodes_request_output_and_downloader_argument_snapshot` 2414、`execution_spec_v1_and_v2_decode_to_empty_historical_argument_snapshot` 2446、`execution_spec_v3_refuses_a_mismatched_argument_policy_version` 2474、`execution_spec_v3_rejects_invalid_json_explicitly` 2497、`execution_spec_v3_rejects_unknown_fields_in_the_argument_snapshot` 2505、`v3_argument_snapshot_round_trips_through_json_without_rewriting_argv` 2521、`downloader_argument_diagnostics_do_not_disclose_values_or_tool_versions` 2543、`execute_reply_waits_until_execution_resources_are_dropped` 2762、`submits_and_reuses_an_active_job_for_duplicate_tweets` 2786、`cancel_is_idempotent_and_does_not_change_terminal_state` 2799、`running_cancel_signals_execution_and_fences_late_success` 2810、`shutdown_interrupts_active_jobs_and_rejects_later_commands` 2840、`concurrent_submissions_keep_one_active_worker_per_tweet` 2851、`start_preserves_job_state_machine_boundaries` 2871、`application_service_delegates_submit_query_cancel_and_shutdown` 2886、`recovery_skips_terminal_and_duplicate_jobs_and_restarts_interrupted_jobs` 2897、`persisted_submit_reuses_database_like_active_job_semantics` 2932、`execute_persisted_records_successful_lifecycle_and_completion_order` 2965、`execute_persisted_records_failure_without_polluting_other_jobs` 3008、`stale_attempt_cannot_overwrite_a_state_written_by_a_newer_attempt` 3038、`saturated_runner_rejection_is_non_failing_and_reschedulable` 3166、`execute_persisted_skips_terminal_jobs_without_calling_execution_port` 3256、`persisted_submit_marks_created_job_failed_when_executor_is_closed` 3277、`submit_and_schedule_persists_background_executor_failure` 3300、`persisted_scheduler_binds_attempt_before_successful_completion` 3340、`startup_redispatch_binds_attempt_before_successful_completion` 3394、`submit_and_schedule_marks_job_failed_when_database_path_is_invalid` 3441、`startup_recovery_fails_closed_when_internal_manifest_is_missing` 3481、`startup_recovery_takes_over_no_journal_internal_v1_downloading_attempt` 3554。

- 生产符号（1–899）：`CommitRecoveryFactsProvider` trait 253、`CancellationToken` 257（`Arc<AtomicBool>`；`same_instance` 267 为 `pub(crate)`；`cancel`/`is_cancelled` 用 Release/Acquire）、`ExecutorEvent` 280（10 变体）+ `to_job_event` 318（`SidecarCrashed` → `DownloadFailed{ error_code: "SIDECAR_INTERNAL_ERROR" }`）、`CancelArbitration` 361、`StorageJobPersistence`（396 段：`from_database` 403、`now` 412、`remember_tweet` 419、`stored_events` 440、`stored_job_summary` 453、`recovery_facts` 460）、`ProductionCoordinator` 484（unix）+ `open` 501（错误码 `NO_DATABASE_PARENT`／`COORDINATOR_OPEN_FAILED`／`WORKSPACE_ADMISSION_UNAVAILABLE`）+ `impl Coordinator` 532、`state_sort_priority` 545、`impl JobPersistence for StorageJobPersistence` 562–852、`ExecutorError` 854（7 变体）+ `Display` 872 + `std::error::Error` 897。
- `impl JobPersistence` 关键语义：`configure_v2_enrollment`、`create_or_reuse`（`v2_enrollment` 决定 `InternalRenameV2` 或 `InternalV1`，570–576）、`fail` 662、`record_event` 680（**刻意不为 `StateChanged`/`Recovered`/`ShutdownInterrupted` 重复落库**，因 `Database::transition_job` 已原子写入）、`events` 703、`complete_terminal_for_attempt` 710（`!applied` 时直接回读快照再镜像内存事件）、`begin_attempt` 746、`current_attempt` 752、`claim_attempt` 758、`claim_internal_v1_successor` 770、`internal_v1_attempt_phase` 780（`Prepared`→1、`Committed`→2）、`is_internal_v1_contract` 794、`record_claimed_attempt_state` 801、`archive_commit_started` 814、`persist_cancellation` 820（三态映射）、`attempt_is_current` 838、`snapshot_for_tweet` 845。
- **`#[ignore]` 完整清单（11 个，逐字保留，全部同时带 `#[cfg(target_os = "linux")]`）**：936 `requires explicitly delegated disposable cgroup`（`production_factory_contained_handshake_and_drop_remove_writer_group`）、1144 同（`production_real_tools_publish_local_media`）、1510 `requires explicitly delegated disposable cgroup and real aria2`、1602 同（`production_real_aria2_transfer_and_refresh_drain_recorded_writer`）、1713 `test-only v2 acceptance is deferred; production v2 stays fail-closed until C1`（`test_only_v2_scheduler_fixture_publishes_and_recovers_protocol_bytes`）、4182 `requires systemd-run --user --property=Delegate=yes disposable cgroup`（`startup_recovery_drains_live_descendant_after_owner_sigkill`）、4667 与 4901 `requires an explicitly delegated disposable cgroup`、5148 `requires explicitly delegated disposable cgroup and real aria2`（`production_runner_registers_and_drains_real_aria2_before_internal_v1_publish`）、8577 与 8724 `requires an explicitly delegated disposable cgroup`（`executor_shutdown_joins_active_writer_before_successor_runs`、`runtime_replacement_joins_active_writer_then_runs_successor`）。
- 测试结构：`#[cfg(test)] mod tests` 900–7495（`use super::*`）；`#[cfg(all(test, unix))] mod session_boundary_tests` 7496–9815（`use super::*` + `Mutex`／`mpsc::{Receiver, Sender}`／`thread`），注释说明边界测试需真实 Unix 协调适配器，边界本身跨平台编译。
- 测试内环境变量依赖：`TW2TG_TEST_GALLERY`（1147）、`TW2TG_TEST_MEDIA_URL`（1148）、`TW2TG_TEST_ARIA2`（1159、1613）、`PYTHON`（1340、1404）。均只出现在显式 `#[ignore]` 的真实工具用例中。
- 测试 fixture（`mod tests`）：`request` 906、`archive_request` 916、`v3_spec` 2400、`insert_downloaded_job` 2559、`complete_job_for_fixture` 2578、`v2_request` 2596、`insert_v2_downloaded_job` 2610、`temporary_database_path` 2634、`wait_for_persisted_state` 2648（100×10 ms 轮询后 panic）、`FakeJobExecution` 2666（**结果 `Option` 一次性 take，二次执行 panic**）、`CancellationAwareExecution` 2714、`DropNotifiesExecution` 2735 + `Drop` 2755。

### 21. `desktop/src-tauri/src/executor.rs`（9815 行，P4/P9；分段全覆盖）

- 覆盖：1–250、251–500、501–750、751–899（生产）；测试区按 350 行窗口连续读完 900–1150、1151–1400、1401–1650、1651–1900、1901–2150、2151–2400、2400–2600、2601–2800、2801–3000、3001–3300、3301–3600、3601–3950、3951–4300、4301–4650、4651–5000、5001–5350、5351–5700、5701–6050、6051–6400、6401–6750、6751–7100、7101–7450、7451–7800、7801–8200、8201–8600、8601–9000、9001–9400、9401–9815。窗口边界连续；个别大窗口内部出现工具截断（记录于各窗口读取结果），这些被截断的中段在需要判定语义时已按更小窗口补读，未用于代替全文复核。
- 测试计数（实测）：**`#[test]` 125 个、`#[ignore]` 11 个**。结合 `archive.rs` 的 9 个测试含 2 个 `#[ignore]`，合计 ignored = **13**，与 P1 基线 `cargo test -p xarchive-desktop --lib` 的 `365 passed / 0 failed / 13 ignored` 在 ignored 维度完全吻合。
- 文件级属性：**`#![allow(dead_code)]`（9 行）** —— P4/P9 拆目录后必须在新模块根部保留。
- 模块声明（47–62）：`mod containment; mod contracts; mod factory; mod session; mod runtime; mod recovery; mod traits;` —— **`executor` 已经是 `executor.rs` + `executor/` 子目录的双形态**，`runtime.rs`（2692 行）即 `mod runtime`。P4/P9 必须沿用既有布局，不得新建第二套。
- 重新导出（P4 不得改可见性）：`session::{SessionExecutionBoundary, SessionRescheduleReason, SessionRunOutcome, resolve_session_run}`（52–54，`#[allow(unused_imports)]` + `pub(crate) use`）、`runtime::{ArchiveApplicationService, ExecutorRuntime, JobExecutor, JobExecutorHandle}`（57，`pub use`）、`recovery::{StartupRecoveryOutcome, StartupRecoveryReport, StartupRecoveryStatus}`（59，`pub(crate) use`）、`factory::ProductionExecutionFactory`（64，`pub use`）、`UnconfiguredExecutionFactory` 65、Linux `containment::{AttemptWriterDomain, LinuxExecutionContainment}`（68，`pub use`）、Linux `pub(crate) use containment::{attempt_writer_base, containment_record_root, recover_and_create_writer_group, recover_predecessor_writer_domains, recover_writer_domain_set, recover_writer_domain_set_with_evidence}`（75–79），注释明确排除 `ATTEMPT_WRITER_ROLES`（“re-exporting it produced an unused import”）。
- 条件编译导入（逐条保留）：`#[cfg(test)] use std::path::PathBuf;`（14–15）、`#[cfg(test)] use std::sync::atomic::AtomicUsize;`（17–18）、`#[cfg(test)] use std::sync::mpsc;`（20–21）、`#[cfg(test)] use xarchive_core::ProxyMode;`（26–27）、`#[cfg(test)] use xarchive_sidecar_supervisor::SidecarSupervisor;`（31–32）、**`#[cfg(test)] pub use xarchive_storage::BatchNamingMode as NamingMode;`（33–34）** —— `#[cfg(test)]` 与 `pub use` 组合的测试专用对外别名，极易被误删。

### 20. `desktop/src-tauri/src/executor/runtime.rs`（2692 行，P4/P9；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1250、1251–1500、1501–1750、1751–2000、2001–2250、2251–2500、2501–2692。无未读行。
- `//!` 文档仅 1 行：“Executor runtime/control plane: queue admission, commands and worker ownership.”
- **本文件没有测试模块**（`#[cfg(test)]` = 0，`#[test]` = 0，`mod tests` = 0）。测试位于 `executor.rs` 的 899+ 行（含嵌套 `session_boundary_tests`），P4 迁移时该目录是唯一测试来源。
- 生产结构：`ExecutorRuntime` 29（`pub(super) executor`、私有 `database_path`／`config`、Linux `pub(super) containment`）；其 `new` 38（从 db 路径推导 archive_root／staging_root，读 4 个 env）、`with_config` 67、`with_production_factory`、`execute_session`（150–182 段，`SessionRunOutcome::StartupCleanupPending` 映射为 `UNRESOLVED_STARTUP_CLEANUP`）、`database_path` 184、`config` 188、`linux_containment` 192（Linux）、`aria2_program` 197、`gallery_dl_program` 201、`recover_startup` 205、`recover_startup_with_v2` 217（`pub(super)`，生产恒 `false`）、`is_running` 966、`shutdown` 970、`shutdown_in_place` 974。
- 控制面类型：`ArchiveApplicationService` 980（`pub(super) executor: JobExecutorHandle`、`pub(super) v2_enrollment: bool`）、`JobRecord` 986（`snapshot`／`events`／`cancellation`）、`CancellationRegistry` 993（`Arc<Mutex<HashMap<String, CancellationToken>>>`）+ `token` 995；`enum Command` 1008（11 个变体，全部 `mpsc::Sender` 响应）；`enum RunnerCommand` 1053（`Operation`／`Execute`／`RunJob`／`Shutdown`）；`trait RunnerOperation` 1068；`RunnerOperationTask<F,T>` 1074 + `run` 1091（`catch_unwind` + 显式 `drop(self.operation.take())`）；`JobExecutorHandle`（1149 段，含 `worker`／`runner` 的 `JoinHandle`）；`JobExecutor` + `new` 1154／`with_capacity` 1158／`with_capacity_and_factory` 1162／`handle` 1191／`is_running` 1195／`shutdown` 1199／`shutdown_in_place` 1211／`Drop` 1225。
- `JobExecutorHandle` 方法：`cancellation_for` 1241、`runner_try_admit` 1253（`fetch_update` + `in_flight < RUNNER_CAPACITY`）、`runner_release` 1262、`owned_runner_admission` 1269、`run_operation` 1276（`try_send` → `QueueFull`/`Closed`，`drop(operation)` 后返回）、`submit` 1309、`start` 1313、`cancel` 1320、`snapshot` 1327、`shutdown` 1334、`recover`、`events`、`execute`（1370–1409 段）、`run_job` 1411、`request` 1432。
- `ArchiveApplicationService` 方法（1449 起）：`new` 1450、`submit` 1457、`query` 1461、`cancel` 1465、`recover` 1469、`shutdown` 1473、`events` 1477、`run_job` 1481、`submit_persisted` 1489、`submit_and_schedule_persisted` 1542、`schedule_persisted` 1579、`execute_persisted`（1650 段）、`execute_persisted_on_worker` 1750、`execute_persisted_from_factory`（1900 段）、`recover_persisted` 2052、`interrupt_persisted`（2130 段）、`shutdown_persisted` 2168、`complete_persisted` 2176、`apply_commit_recovery` 2208、`recover_commit_persisted` 2238、`recover_commits_persisted` 2259。
- 内部线程入口：`run_runner` 2295（`RunnerCommand::Execute` 执行后**先 `drop(execution)` 再发响应**，注释“tear them down before the caller can proceed to finalization”）、`run_worker`（2400 段，`Command::Cancel` 里**同一 token 被 cancel 两次**，属既有冗余实现，P9 不得“顺手修正”）、`transition_job` 2613、`cancel_job` 2636、`sidecar_crash` 2664。
- **P9 硬约束（ADR-015，全部必须保持）**：`recover_startup` 恒以 `false` 调用 `recover_startup_with_v2`（205–213），生产 v2 enrollment 保持关闭；`recover_startup_with_v2` 的恢复顺序为 list_recovery_candidates → 追加 `internal_v1_recovery_job_ids`（Linux 且 containment 存在时还追加 `DOWNLOADING` 且 `internal_v1_attempt_phase == Ok(Some(None))` 的作业）→ 按 `job_id` 排序 → 逐作业按 contract 分流（v2 走 `recover_v2_on_runner`；InternalV1 依次判断 `archive_commit_started` / `internal_v1_published_candidate` / containment drain / `recover_internal_v1_on_runner`；`Unknown(_)` 直接 `ARCHIVE_RECOVERY_UNSUPPORTED_VERSION`；缺 spec 且非 Legacy → `EXECUTION_SPEC_MISSING`；`InternalRenameV2` 未被 gated 时一律 `ARCHIVE_RECOVERY_V2_NOT_READY`）→ `CommitRecoveryDecision` 四分支（`Resume`／`Complete`／`MarkFailed`／`Skip|NotApplicable`）→ 结果按 job_id 排序映射为 `StartupRecoveryStatus::{Recovered, DeferredByContention, Refused}`。
- 队列/所有权不变量：`runner_try_admit` 必须**先**于 durable attempt claim（1760–1767 注释），失败返回 `QueueFull` 且不递增 attempt、不写 FAILED；`execute_persisted_on_worker` 只在 runner 交互完成后 `drop(_reservation)`（1987），使后续持久化不占用 runner；`claim_attempt` 输者返回 winner snapshot 且无副作用；`persist_state_for_attempt` 为 false 即放弃写并回读快照；`Command::Execute`/`RunJob` 的 `try_send` 错误匹配覆盖全部 `RunnerCommand` 变体，`Operation`/`Shutdown` 走 `unreachable!`（2548–2561、2598–2605）。
- `Drop for JobExecutor` 只在 `worker.is_some()` 时 shutdown（1227），避免重复 join。

### 19. `desktop/src-tauri/src/production.rs`（1595 行，P8；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1250、1251–1595，补齐 1073–1160、1335–1480、1481–1485。无未读行。
- `//!` 文档 1–6：本模块拥有 v2 的 extraction → aria2 transfer 边界；v1 代码已在 U8 移除，`archive.rs` 拥有 executor context、staging 验证与 `ArchiveService` 提交。
- 生产区间 1–1222；**三个测试单元**：`mod e3_argument_tests` 208–405（8 个测试，含 3 个 `#[cfg(unix)]`）、`include!("production_transfer_fixture.rs")` 1121–1122（`#[cfg(all(test, target_os = "linux"))]`，外部文件）、`mod tests` 1223–1595（9 个测试）。`#[ignore]` = 0。
- **外部测试文件依赖（P8 必须处理）**：`#[cfg(all(test, target_os = "linux"))] include!("production_transfer_fixture.rs");`（1121–1122）。该 fixture 是独立源文件，通过 `include!` 文本内联到本模块作用域，因此它能直接访问本模块私有项；搬迁 `production/` 子模块时 `include!` 的相对路径与作用域都必须重新核对。
- 生产符号与 P8 五组划分：
  - **tool_compatibility**：`revalidate_tool_compatibility` 63（校验 `validate_downloader_argument_snapshot` → backends 交叉匹配 → `executable_version` 与快照 `tool_compatibility` 精确比较）、`resolved_tool_version`（`crate::production::resolved_tool_version`，被 `runtime.rs` 调用）、`executable_version`。
  - **discovery**：`DiscoveryRequest` 961、`DiscoveryOutcome` 970、`execute_v2_discovery` 981（流式，每个 `candidate` 先 `validate` 再交给 `on_candidate` 回调，`discovery_completed` 收尾）。
  - **sidecar_operations**：`extract_v2`（678–762，15 分钟 deadline）、`download_v2` 768（30 分钟 deadline）、`execute_gallery_dl_archive` 942。
  - **transfer**：`aria2_config` 1085、`available_loopback_port` 1114、`transfer_once`（Linux 405–462／非 Linux 464–486 两个 `#[cfg]` 版本）、`run_transfer_with_supervisor` 488、`transfer_with_one_refresh` 571、`transfer_files` 1124、`TransferAttemptFailure` 174（Linux）、`retry_retained_drain_with` 154、`retry_retained_drain` 167（均 Linux）、`transfer_failure_allows_refresh` 149（Linux）。
  - **result_mapping**：`media_type_name` 1173、`extraction_to_metadata` 1182、`format_transfer_failure` 1211、`verify_downloaded_files`。
- 平台门控（逐字保留）：`ArchiveExecutionFailure` 类型别名 334/336 双版本；`transfer_once` 是 Linux／非 Linux **两个同名函数**（406／464）；`TransferAttemptFailure` 仅在 Linux 编译（173）；`include!` 仅在 `all(test, target_os="linux")` 生效（1121）；`execute_v2_archive` 的 `containment` 参数用 `#[cfg(target_os = "linux")]` 内联属性（39、50）。
- 不变量（逐字保留）：`execute_v2_archive` 先 `revalidate_tool_compatibility`（失败 `(message, None)`），`use_aria2` 为 true 才走 aria2 路径，`aria2_program.is_some()` 但 `use_aria2 == false` 时打印 `"warning: aria2 path is configured but optional downloads are off; aria2 will not start"`（54–58）；**containment 失败绝不 fallback 普通 spawn**（`transfer_once` Linux 版 `Some(domain)` 分支失败即返回，`crate::executor::recover_and_create_writer_group` + `spawn_contained_owned`，只有 `None` 才走 `spawn`）；`run_transfer_with_supervisor` 用 10 ms 轮询线程把 cancellation 桥接到 `control.cancel()`，结束后必 `join` 监控线程并 `aria2.shutdown()`，任一失败即 `ShutdownIncomplete` 且**保留 owner**（`retained_supervisor: Some(aria2)`）；`transfer_with_one_refresh` 允许单次刷新且仅当 `TransferFailureCode::ExpiredUrl && !has_retained_owner`（149–151）；`aria2_config` 的 port 优先 `XARCHIVE_ARIA2_PORT`（0 视为未设），否则绑定 `127.0.0.1:0` 取实际端口，失败回退 6800；`transfer_files` 与 `verify_downloaded_files` 都做 `strip_prefix(staging_dir)` + `ParentDir|RootDir|Prefix` 拒绝 + `symlink_metadata` 常规文件校验。
- 环境变量依赖：`XARCHIVE_ARIA2_PROGRAM`（1092）、`XARCHIVE_ARIA2_PORT`（1094）。
- 测试清单（17）：`e3_argument_tests` 213 `transfer_containment_failure_never_falls_back_to_plain_spawn`、248 `gallery_dl_snapshot_rejects_before_any_download_process_is_spawned`、272 `version_probe_uses_resolved_executable_and_parses_reported_version`（`#[cfg(unix)]`）、292 `version_probe_failure_is_diagnostic_but_does_not_echo_arguments`、301 `empty_snapshot_does_not_probe_or_spawn_gallery_dl`（`#[cfg(unix)]`）、及该模块内另外 3 个（共 8）；`mod tests` 1295 `verifies_reported_download_files_against_the_staging_directory`、1309 `rejects_escaping_missing_mismatched_and_empty_downloads`、1359 `production_aria2_configuration_generates_a_fresh_secret_without_environment_setup`、1378 `aria2_config_forwards_validated_user_args_into_the_spawned_config`、1402 `the_aria2_configuration_carries_the_selected_mode`、1449 `converts_extraction_to_durable_metadata_without_headers_or_urls`、1487 `rejects_changed_media_identity_or_filename_sets`、1508 `transfer_failure_enters_refresh_expired_url_without_retained`、1523 `transfer_failure_enters_refresh_other_code`、1538 `retry_retained_drain_no_owner_returns_false`、1555 `retained_drain_failure_preserves_owner_and_blocks_refresh`、1574 `retained_drain_success_releases_owner_only_after_shutdown`、1589 `expired_url_refresh_policy_rejects_retained_owner`。
  - **注意**：`transfer_failure_enters_refresh` 系列测试构造 `TransferAttemptFailure`，而该类型仅 Linux 编译；这些测试**没有** `#[cfg(target_os = "linux")]` 属性，说明 `mod tests` 在非 Linux 上会编译失败 —— 这是既有状态，P8 迁移时必须原样保留并单独记录为“跨平台测试可编译性风险”，不得借此修改生产代码或删测试。
- 测试 fixture：`staging_extraction` 1229、`staging_download` 1253、`temp_case_root` 1273、`temp_staging` 1283、`remove_case_root` 1289（`remove_case_root` 用 `staging.parent()` 推导，注释说明逃逸路径断言需要独占父目录）。

### 18. `desktop/src-tauri/src/archive.rs`（1745 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1250、1251–1500、1501–1745，补齐 100–180、334–410、593–660。无未读行。
- 生产区间 1–1000；`#[cfg(test)] mod completion_tests` 1001–1745（9 个 `#[test]`，其中 **2 个 `#[ignore]`**）。
- `#[ignore]` 测试完整属性：1320–1322 `#[test]` + `#[ignore = "requires an explicitly delegated disposable cgroup"]` + `fn registered_writer_with_incomplete_drain_is_refused_by_authorization`；1481–1483 同属性 + `fn registered_undrained_writer_is_refused_by_production_finalizer`。两者的附加门控是 `#[cfg(target_os = "linux")]`（1319、1480），因此在非 Linux 平台既不编译也不计数。
- 生产符号：`upsert_browser_user` 21（`pub(crate)`）、`SidecarArchiveResult` 44（`pub(crate)`，第三个字段 `retained_aria2` 仅 Linux）、`PendingArchivePublication` 57（**pub**，14 私有字段 + 2 个 `#[cfg(target_os = "linux")]` 字段）、`from_verified_facts` 79（私有）、`finalize` 150（`pub(crate)`，泛型 `C: Coordinator`）、`impl Debug` 314；`ArchiveExecutionFailure` 类型别名 334/336（Linux 为 `(String, Option<Aria2Supervisor>)`，否则 `String`）；`AttemptArchiveFacts` 351（`pub(crate)`，`#[derive(Debug)]` 且**故意无 `Clone`**）+ `after_sidecar_drain` 369（`#[allow(dead_code)]`）+ `verify_attempt_facts_staging` 396；`SidecarDownloadRequest` 417（`pub(crate)`，8 字段）；`ArchiveExecutionContext` 436（`pub(crate)`，12 字段，含 Linux `containment` 与私有 `retained_aria2`）；`ArchiveExecutionJob` 457 + `with_metadata_output` 468／`new` 476；`impl JobExecution for ArchiveExecutionJob::execute` 501；`execute_archive_context` 565（`#[allow(clippy::too_many_arguments)]`）；`collect_attempt_facts` 656（同 lint）；`capture_completed_intent` 760（`#[allow(clippy::too_many_arguments, dead_code)]`）；`sample_completion_config` 795；`impl ArchiveExecutionContext` 810：`shutdown_sidecar` 815、`completion_telegram_config` 819、`with_aria2_and_network` 823、`with_gallery_dl_program` 850、`with_attempt_identity`（在 850–908 段内）、`download_v2`、`create_attempt_staging_directory` 927；`merge_browser_relationships` 947（`pub(crate)`）。
- **P12 处置依据（保留 PendingArchivePublication 所有权）**：`PendingArchivePublication` 是 move-only 发布凭据 —— 通过 `&mut self` + `finalized: bool` 的 `std::mem::replace` 实现“只能 finalize 一次”（156–162），并通过 `_sidecar_owner: SidecarSupervisor` 与 `_retained_aria2` 把已 drain 的进程所有者钉在发布值里；`AttemptArchiveFacts` 无 `Clone` 且要求 `Send`（1682 测试锁定）。这些是编译期所有权证据，拆文件会诱使新增 `Clone`/公开字段。**保持单文件。**
- `CROSS_PLATFORM_CHANGE_REQUIRED` 标记：55 行（ADR-015 clause 8，runner 获得同一 Storage/进程所有者但不转移借用 session）。
- 不变量（逐字保留）：`finalize` 顺序固定为 `finalized` 一次性检查 → 取消检查（`TRANSFER_CANCELLED`）→ 四重身份校验（`session.job_id`／`session.attempt().job_id`／`session.attempt == attempt_identity`／`session.destination() == service.files.archive_path(final_directory)`，失败为 `PUBLICATION_IDENTITY_MISMATCH`）→ `close_admission()`（失败 `PUBLICATION_DRAIN_INCOMPLETE`）→ `authorize(session)` 并把 6 种 `PublicationError` 映射为固定 error_code（`PUBLICATION_ATTEMPT_MISMATCH`／`_DESTINATION_MISMATCH`／`_INCOMPLETE_DRAIN`／`_SESSION_MISMATCH`／`_ALREADY_PUBLISHED`／其它 `_AUTHORIZATION_REJECTED`）→ `preview_sidecar_archive_attempt`（失败 `ARCHIVE_PREVIEW_FAILED`）→ 可选 telegram intent 准备（失败 `TELEGRAM_INTENT_PREPARE_FAILED`）→ `publish_internal_v1_attempt`（失败 `ARCHIVE_PUBLISH_FAILED`）。
- `attempt_identity`/owner 保活：`collect_attempt_facts` 在下载失败时把 `retained_aria2` 放回 context（696–699），在 `shutdown_sidecar()` 失败时**无条件**回填（716–721，注释说明 `Some` 即未 drain 的 writer，不得 drop），并在 facts 构造前拒绝 `retained_aria2.is_some()`（732–738，`ARCHIVE_WRITER_DRAIN_FAILED` + `"retained aria2 owner is not drained"`）。
- `merge_browser_relationships` 规则：只填空缺（`None`/`Null`/空字符串视为缺失），Sidecar 已有值绝不覆盖；`tweet_type` 仅在 Sidecar 为 `"post"` 且浏览器观察到别的类型时升级（939–999）。
- `sample_completion_config`：配置路径读取失败/解析失败/校验失败一律回退 `TelegramConfig::default()`（即 fail-closed 到“未启用”），`None` 则用注入的 fallback（795–808）。
- 测试清单（9）：1006 `completion_samples_latest_persisted_settings_and_fails_closed`；1035 `attempt_staging_resolution_requires_matching_identity_and_isolates_attempts`；1089 `archive_context_shutdown_joins_sidecar_before_completion`；1117 `attempt_facts_require_sidecar_drain_and_carry_transfer_count`；1158 `pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`；1322 `registered_writer_with_incomplete_drain_is_refused_by_authorization`（`#[ignore]` + Linux）；1483 `registered_undrained_writer_is_refused_by_production_finalizer`（`#[ignore]` + Linux）；1682 `attempt_facts_are_attempt_owned_and_not_cloneable`；1694 `attempt_facts_staging_verification_rejects_missing_or_replaced_trees`。
- 测试直接断言私有细节：`&*pending.service as *const ArchiveService` 与 `&pending._sidecar_owner as *const SidecarSupervisor` 的指针相等（1250–1251、1288–1292），`pending.finalized` 为 true，`pending.service.database.archive_recovery_journal(...)` 为 `None`。→ P12 迁移测试时**不得**把这些字段改为 pub，也不得把测试移出本模块。

### 17. `desktop/src-tauri/src/runtime.rs`（1258 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1258，补齐 89–163、343–415、839–918。无未读行。
- 生产区间 1–906；**两个测试模块**：`mod browser_tool_compatibility_tests` 853–884（2 个测试，`use super::{configured_sidecar_args, resolve_browser_tool_compatibility, resolved_gallery_dl_program}` —— **显式具名导入**，不是 `use super::*`），`mod tests` 908–1258（11 个测试，`use super::*`）。共 13 个测试、`#[ignore]` 0 个。
- 生产符号：`RuntimeState` 19（27 个 `pub(crate)` 字段，含 4 个 `#[cfg]` 条件字段：`transport_server`(unix)／`transport_server`+`transport_error`(windows)）；`SharedTelegramSecrets` 53（`pub(crate)`，`Arc<StdMutex<Box<dyn SecretStore + Send>>>`）+ `impl SecretStore` 57（三个方法都把底层错误折叠为 `secret_unavailable()`）；`secret_unavailable` 81；`timestamp_marker` 85；`RuntimeState` 方法：`install_telegram_secrets` 96（`#[allow(dead_code)]`）、`telegram_proxy` 107、`restart_telegram_sender` 144、`delete_telegram_secret` 172、`start_telegram_sender` 197（`#[allow(dead_code)]`）、`install_telegram_worker` 238（`#[allow(dead_code)]`）、`stop_telegram_worker` 256、`record` 272、`debug` 280、`warn` 285、`recover_executor_startup` 289、`record_startup_recovery_report` 301、`record_recovery_results` 339、`error` 358、`start_transport` 362、`stop_transport` 456、`reconcile_telegram_plans` 463、`recover_telegram_archives` 493、`replace_executor` 537、`initialize` 573（`#[cfg(test)]`）、`try_initialize` 577、`initialize_at` 600、`initialize_with_containment` 608；自由函数 `executor_config` 767、`resolved_gallery_dl_program` 813、`resolve_browser_tool_compatibility` 823、`configured_sidecar_args` 886、`sidecar_runtime_args` 894；`impl Drop for RuntimeState` 900。
- **P12 处置依据（组合根生命周期不拆）**：`RuntimeState` 是唯一的组合根，`replace_executor` 必须按“停 telegram worker → 停 transport → shutdown executor → 重建 executor → 重启 transport → 恢复 telegram 归档 → 执行器启动恢复 → 重建 telegram 计划 → 重启 sender”这一固定顺序执行（547–569），任何拆分都会让这条 shutdown/join 顺序失去单一控制点。故只整理测试，不拆生产代码。
- 平台门控（逐字保留）：`transport_server` 在 unix 与 windows 各有一个字段声明（37–40）；`start_transport` 中 unix 用 `crate::transport::{transport_endpoint, DesktopTransportServer}`，windows 用 `crate::windows_transport::{transport_endpoint, DesktopTransportServer}`（418–452）；`replace_executor` 中 Linux 先 `linux_containment()` 并调 `validate_launch_paths()`，重建时按是否有 containment 走 `with_linux_containment` 或 `with_config`（541–563）；`try_initialize` 在 Linux 从 `std::env::args_os().skip(1)` 构造 `LinuxExecutionContainment`（578–590）。
- 不变量（逐字保留）：`telegram_proxy` 对 `EndpointMode::Local` 直接返回 `None`（“Local Bot API server is on loopback, so it never uses a proxy”）；对远端直接用 `resolution.primary()` 而不是 `to_decision()`，注释明确后者会把 SOCKS 折叠成字符串再被重新规范化为 HTTP（118–124）；`restart_telegram_sender` 在 `migration_pending` 时返回 `"endpoint migration pending; sender remains paused"`，在未启用时静默 `Ok(())`，无 provider 时报 `"platform credential provider unavailable"`；`delete_telegram_secret` 先停 worker 再 `retire_telegram_credential_generation` 再删 secret（“revokes before native failure”）；`recover_telegram_archives` 用**独立连接**（`Database::open`）而不替换 runtime 句柄（491–492 注释）；`start_transport` 在无选中 downloader 参数时 `tool_compatibility = None`（398–406），避免不必要的外部版本探测；`Drop` 只做 `stop_telegram_worker(2s)` + `executor.shutdown_in_place()`，`Drop` 位于 900–906，**在测试模块之前**。
- 环境变量依赖：`XARCHIVE_SIDECAR_PROGRAM`（780）、`XARCHIVE_GALLERY_DL_PROGRAM`（814）、`XARCHIVE_ARIA2_PROGRAM`（788）、`XARCHIVE_SIDECAR_ARGS`（887）。这些 `std::env::var` 在源码中直接可见，属于 P12 必须记录的隐式依赖。
- 测试清单（13）：`browser_tool_compatibility_tests` 861 `browser_compatibility_uses_the_selected_production_tool_identity`、877 `resolved_gallery_program_matches_the_sidecar_default_argument`；`mod tests` 913 `recovery_refusal_is_visible_in_application_logs_without_raw_errors`、936 `fatal_startup_recovery_error_propagates_without_logging_raw_details`、961 `provider_lifecycle_fails_closed_and_deletion_revokes_before_native_failure`、1018 `injected_provider_starts_and_restarts_exactly_one_worker_with_loopback_direct`、1046 `disabled_runtime_does_not_start_sender_or_read_credentials`、1075 `runtime_refuses_replacement_until_inflight_worker_exits`、1123 `runtime_owns_and_stops_injected_telegram_worker`、1146 `configured_sidecar_args_passes_portable_gallery_dl_path_to_worker`、1166 `executor_config_preserves_gallery_dl_and_network_arguments`、1196 `replacing_executor_restarts_transport_on_the_new_service_generation`（`#[cfg(unix)]` 1194）、1252 `parse_sidecar_args_accepts_explicit_json_array`。
- 测试特征：几乎所有测试都用 `RuntimeState::initialize_at(std::env::temp_dir().join("...-{pid}-{timestamp_marker}"))` 建真实运行时并 `std::fs::remove_dir_all` 收尾；`disabled_runtime_does_not_start_sender_or_read_credentials` 用 `ForbiddenSecrets` 在三个 trait 方法里 `panic!` 证明“未启用时不触碰凭据”；`provider_lifecycle_...` 用 `FailingDelete` 返回含 `"sensitive-native-detail"` 的错误并断言对外错误被折叠。`replacing_executor_restarts_transport_on_the_new_service_generation` 依赖 `crate::transport::transport_endpoint`，是 runtime↔transport 的跨模块耦合点。

### 16. `desktop/src-tauri/src/websocket_transport.rs`（1146 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1146，补齐 87–170、315–418、606–673。无未读行。
- `//!` 文档 1–5：WebSocket 帧与握手由 `tungstenite` 提供，本模块只拥有 XArchive 传输信封并把业务消息交给既有 `BrowserTransportAdapter`。
- 生产区间 1–587；`#[cfg(test)] mod tests` 589–1146（13 个测试，其中 1 个 `#[cfg(unix)]`）。
- 生产符号：`MAX_MESSAGE_BYTES` 22（1 MiB）、`validate_handshake` 25（`#[allow(clippy::result_large_err)]`，校验 path == `/`、无 query、`Origin` 恰好一个且等于 `EXTENSION_ORIGIN`，否则 403）、`socket_config` 39（read/write buffer 4096，max write buffer 2 MiB，max message/frame 1 MiB）、`MAX_CONNECTIONS` 53 = 32、`AUTH_TIMEOUT` 54 = 3s、`DeadlineStream<'a>` 55 + `Read` 61／`Write` 113、`AuthenticationEnvelope` 别名 122（= `xarchive_protocol::BrowserPairingAuthentication`）、`AuthenticationResponse` 125、`WebSocketDiagnostics` 134（9 个 `AtomicUsize`）、`WebSocketSessionState` 147（`pub(crate)`，5 私有字段）、`WebSocketDiagnosticSnapshot` 156（`pub`，10 pub 字段）、`WebSocketSessionState::{diagnostic_snapshot 174, browser_connection 200, authenticated 210, last_request_age_seconds 219}`、`DesktopWebSocketServer` 228（5 私有字段，`session` 为 `pub(crate)`）、`start` 237、`port` 345、`runtime_instance_id` 349、`pairing` 353、`Drop` 359、`configured_port` 368、`handle_websocket_connection` 386（`#[allow(clippy::too_many_arguments)]`，注释明确“grouping them would change this shared transport signature and needs its own reviewed batch”）、`authenticate` 519。
- **P12 处置依据**：握手/鉴权/诊断计数/连接上限/关闭握手是一个整体生命周期；`DeadlineStream` 同时承担认证超时、预认证字节预算与停止轮询，拆分会让这三个约束分家。保持单文件，仅整理测试。
- 平台门控（逐字保留）：`DeadlineStream::read` 在 Windows 把剩余超时上限压到 100 ms 以便轮询（79–80），且 `#[cfg_attr(not(windows), allow(clippy::never_loop))]` 63 说明 Unix 只做一次阻塞读；`WouldBlock|TimedOut` 重试只编译于 Windows（94–103）；认证后读超时在非 Windows 为 `None`、Windows 为 100 ms（434–442，注释解释 Winsock 克隆句柄的 shutdown 不能可靠中断 recv）。
- 不变量（逐字保留）：accept 循环先把已结束 worker `swap_remove` + `join`，再按 1 秒窗口限流（≤64 次/秒）且 `workers.len() >= MAX_CONNECTIONS` 时 `continue`（272–286）；`stream.try_clone()` 失败直接 `continue`（288–290）；`handle_websocket_connection` 首先 `set_nonblocking(false)`，注释解释 Winsock 会继承 `O_NONBLOCK` 而 POSIX `accept` 不会（397–402）；`set_write_timeout(1s)`；认证前 `remaining_bytes = 16*1024`，握手成功后收紧到 4096（407、418）；认证后清零 deadline/remaining 并放开读超时（432–442）；`Close` 帧必须 `close(frame)` + `flush()` 再 break（500–508，注释引 RFC 6455 §7.1.2）；`Binary`/`Pong`/`Frame` 忽略；退出时 `active.fetch_sub` + `close_after_auth` 计数（512–516）；`authenticate` 区分 `auth_read_failed`（读失败/非 Text）与 `auth_failed`（ticket 无效），并要求响应实际发出成功才算成功（586）；ticket 校验为 `envelope.validate().is_ok() && pairing.consume(&ticket, EXTENSION_ORIGIN)`（554–557）；`configured_port` 读 `WEBSOCKET_PORT_ENV`，非法值报 `"XARCHIVE_WEBSOCKET_PORT must be a valid TCP port"`，未设置返回 0（让 OS 分配）。
- 诊断语义（新增契约）：`browser_connection()` 只依据 `active` 计数返回 `connected`/`disconnected`/`not_loaded`，注释明确否决了“最近 30 秒内有过请求即算连接”的旧实现（192–199）；`last_request_age_seconds` 作为独立证据字段。
- 测试清单（13）：622 `the_connection_state_follows_the_live_socket_not_a_recent_request`；660 `pins_websocket_transport_defaults`（断言 `WEBSOCKET_DEFAULT_PORT == 17321`、`WEBSOCKET_PORT_ENV == "XARCHIVE_WEBSOCKET_PORT"`、`WEBSOCKET_TOKEN_ENV == "XARCHIVE_WEBSOCKET_TOKEN"`）；673 `authentication_consumes_ticket_and_returns_a_response`；720 `authenticated_connection_routes_a_browser_request`；802 `authentication_rejects_a_wrong_ticket`；840 `authenticates_when_the_accepted_stream_starts_non_blocking`；907 `handshake_requires_exact_single_origin_and_resource`；944 `deadline_cannot_be_extended_by_partial_reads`；966 `server_shutdown_interrupts_handshake_and_authenticated_idle_connections`；1008 `unix_bootstrap_yields_a_ticket_for_the_current_listener`（`#[cfg(unix)]` 1006）；1079 `oversized_first_frame_is_rejected_before_business`；1099 `preauth_connections_are_capped_and_reclaimed`；1128 `preauth_byte_budget_is_total_across_reads`。
- 测试 fixture：`client(port)` 595（构造带 `Origin` 的客户端握手）、`credentials()` 609（用固定端口 43127 造 `PairingCoordinator` + bootstrap ticket）。测试直接读写私有原子字段（`session.active.store`、`session.last_request.lock()`）与 `server.session`，因此测试必须留在同模块。

### 15. `desktop/src-tauri/src/proxy.rs`（1182 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1182，补齐 80–166、343–400、500–560、583–650、843–913。无未读行。
- `//!` 文档 1–15 给出三条设计规则：解析不得在 UI 线程运行（Windows PAC/WPAD 可阻塞数秒）、client 不得每请求重建（`ProxyHttpClient` 按 route 缓存）、`System` 只能声明平台真正能做��的事（无法兑现时必须失败而非直连）。
- 生产区间 1–593；`#[cfg(test)] mod tests` 595–1182（30 个测试，**`#[ignore]` 0 个**）。
- 生产符号与可见性：`ProxyResolver` trait 30（4 个方法，仅 `resolve` 必需）、`SystemProxyDescription` 58（`pub(crate)`，9 个 pub 字段）、`ChildProxyCoverage` 88、`SIDECAR_PROXY_PROBE_URL` 102、`EnvironmentProxyResolver` 111（`candidate_for` 116 私有 + trait 实现 140–188）、`into_resolution` 192、`first_env` 216、`split_destination` 229、`with_default_scheme` 253、`platform_resolver` 269、`resolve` 281、`failure_reason` 303、`build_client_for_candidate` 320、`normalize_candidate` 350、`pac_fallback_must_fail_closed` 381（`#[allow(dead_code)]`，注释说明仅 Windows adapter 消费、此处只由测试驱动）、`authority` 390、`with_http_scheme` 406、`ProxyHttpClient` 427（4 私有字段）、其 `new` 436／`mode` 451／`from_config` 459／`resolution_for` 473／`describe_system` 483／`child_proxy_coverage` 492／`require_child_routable` 513／`candidates_for` 540。
- **P12 处置依据**：`ProxyResolver` 是注入点（`Box<dyn ProxyResolver>`），Windows 通过 `platform_resolver()` 的 `#[cfg(windows)]` 分支换成 WinHTTP 实现，Linux 用 `EnvironmentProxyResolver`（270–278）。单一模块保证“一次配置、所有边界同一路由”。拆分会让 `pac_fallback_must_fail_closed` 这类纯函数与 Windows 适配器分离，故保持单文件。
- 不变量（逐字保留）：`resolve` 中 `Manual` 无值 → `ProxyResolution::unresolved(ProxySource::Manual)`，绝不直连（294–297）；`ProxyDecision::ResolutionFailed`/`Unsupported` 在 `into_resolution` 中保留原因而不降级为 Direct（197–201）；`TRANSPORT_SUPPORTS_SOCKS = true` 与 `resolution.candidates_for(...)` 的 `None` → 报错并要求改配置，不静默直连（557–564）；缓存键为 `"{direct|proxy:<endpoint>}|g<generation>"`（573–580），`generation()` 变化必须换新 client；`child_proxy_coverage` 对 `System` 空解析返回 `RequiresCallerRouting`（497–501）；`pac_fallback_must_fail_closed` 用**白名单**（`available|unconfigured|disabled|unsupported`）而不是黑名单，因为 `PacSourceState` 是 `#[non_exhaustive]`（371–386）；`normalize_candidate` 由 candidate kind 决定 scheme（`Socks` → `socks5://`），不从主机名推断（345–355）。
- 凭据脱敏：所有错误路径用 `proxy.redacted()` / `resolution.to_decision().redacted_proxy()` / `candidate.redacted()`，注释与测试都断言 `s3cret` 不出现（801–817）。
- 测试清单（30）：649 `direct_never_consults_the_resolver`；661 `manual_never_consults_the_resolver`；676 `manual_without_a_value_is_unsupported_rather_than_direct`；692 `system_delegates_to_the_platform_resolver`；708 `a_system_resolution_failure_blocks_the_request`；728 `an_unsupported_mode_blocks_the_request`；739 `one_route_builds_one_client_and_is_reused`；757 `two_routes_are_cached_separately`；780 `a_direct_candidate_is_cached_apart_from_a_proxy`；801 `a_decision_rendering_never_carries_the_configured_credentials`；820 `the_environment_resolver_reports_a_route_or_direct`；829 `every_reachable_candidate_builds_a_client`；843 `an_unresolved_system_policy_refuses_to_build_a_client`；858 `a_manual_mode_without_a_value_refuses_instead_of_going_direct`；871 `an_ordered_list_is_returned_in_priority_order`；891 `a_socks_candidate_builds_a_client_because_the_transport_supports_it`；912 `a_socks_endpoint_named_like_one_is_still_treated_as_socks`；926 `a_generation_change_drops_the_cached_client`；978 `a_socks_candidate_survives_the_route_that_a_transport_uses`；997 `a_direct_candidate_stays_direct_for_a_transport`；1011 `a_destination_splits_into_scheme_host_and_port`；1050 `a_per_url_policy_cannot_be_handed_to_a_child_process`；1076 `a_static_environment_policy_is_inheritable`；1096 `an_absolute_mode_never_depends_on_child_coverage`；1117 `an_unresolved_system_policy_blocks_the_child_with_its_own_cause`；1134 `a_per_url_source_is_reported_by_the_shared_contract`；1142 `a_pac_that_could_not_be_obtained_is_not_honored_as_a_deliberate_direct`；1154 `a_pac_that_ran_and_chose_direct_is_honored`；1164 `no_pac_means_direct_is_the_only_policy_in_play`；1176 `an_unknown_pac_state_does_not_silently_allow_direct`。
- 测试 fixture（`mod tests` 内）：`StaticResolver` 601、`static_resolver` 609、`UnresolvedResolver` 615、`OrderedResolver` 624、`ordered_resolver` 644、`PerUrlResolver` 1034；`a_generation_change_drops_the_cached_client` 内定义 `CountingResolver`／`Shared` 两个本地 resolver。测试直接读私有字段 `client.clients.lock()`（752、771、776、794、962、971），因此测试必须留在同模块。

### 14. `crates/xarchive-protocol/src/sidecar_v2.rs`（1548 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1050、1051–1548，补齐 105–172、352–400、614–668、834–880、880–940、900–1050、1144–1300、1301–1465。无未读行。
- `//!` 文档 1–7：提取结果仅是持久元数据；签名 URL、cookie jar、浏览器凭据路径、文件系统根、aria2 传输状态一律不出现（属于会话传输计划）。
- 生产区间 1–1069；`#[cfg(test)] mod tests` 1071–1548（14 个测试，**`#[ignore]` 0 个**）。
- 类型：`SidecarV2Capability` 15（5 变体）、`REQUIRED_V2_CAPABILITIES` 24、`SidecarV2CommandType` 35（6 变体，注释明确“no v1 `download` fallback”）、`SidecarV2EventType` 50（11 变体）、`SidecarV2Error` 67 + `new` 73、`ExtractionMediaItem` 84、`ExtractionMediaType` 96、`ExtractionRequestHeader` 105、`DownloadedMediaFile` 117、`DownloadResult` 131、`ExtractionQuotedTweet` 142、`DiscoveryCandidate` 166、`ExtractionResult` 184、`SidecarV2Command` 215、`SidecarV2Event` 241。
- 构造/校验：`SidecarV2Command::{hello 268, extract 282, download 302, discover 324, validate 345}`、`SidecarV2Event::{ready 434, extraction_started 456, discovery_started 474, candidate 492, discovery_completed 514, extracted 536, download_started 558, download_completed 576, failed 599, validate 622}`、`ExtractionResult::validate` 736、`DownloadResult::validate` 801、`DiscoveryCandidate::validate` 848、`has_required_capabilities` 930。
- 私有校验/清理 helper：`validate_extraction_quoted_tweet` 880、`validate_numeric_id` 907、`is_profile_url` 915、`validate_protocol_version` 936、`validate_request_id` 943、`validate_job_id` 950、`reject_success_error_fields` 957、`is_safe_filename` 964、`is_safe_media_identity` 989、`validate_relative_media_path` 998、`is_allowlisted_header_name` 1023、`Sanitizer` 1030（`redacted_code` 1033／`bounded_message` 1048）、`looks_like_secret_value` 1061。
- **P12 处置依据（协议模型/验证留聚）**：这是 Rust↔Python 的跨语言契约，`serde(deny_unknown_fields)`、`rename_all = "snake_case"`、`skip_serializing_if` 的组合就是线上格式；验证函数与 sanitizer 直接决定拒绝面。拆分会让“模型定义”与“对应校验”分散，且 `extract_tweet_id` 被多处共用。**保持单文件。**
- 不变量（逐字保留）：每个结构体都是 `deny_unknown_fields`（67、83、104、116、130、141、165、183、214、240）；`SidecarV2Command::validate` 按命令类型分支（Hello/Shutdown 禁止 url/staging_dir/user_args；Extract/Download 要求 `extract_tweet_id(url)` 成功且 job_id ≠ `"system"`；Download 额外要求 `staging_dir` 绝对、≤4096、无 `\0`，`user_args` ≤128 项、每项非空 ≤4096、无 `\0`/`\n`/`\r`；Discover 要求 `is_profile_url`）；`SidecarV2Event::validate` 先禁止跨事件字段（candidate/candidates_found/download 只在对应事件允许），再按事件类型校验（Ready 必须有全部 5 项能力、Log 消息 1..=4000 且 level ∈ {debug,info,warning,error}、DiscoveryCompleted 计数 ≤1_000_000、Failed 要求 code ≤64 且 message ≤500）；`ExtractionResult::validate` 要求 tweet_id 全数字 ≤32、`extract_tweet_id(url) == tweet_id`、tweet_type ∈ {post,reply,quote}、media ≤32 且索引严格从 1 递增、URL ≤2048 且 http(s)、文件名安全、URL/header 值不得形似密钥、headers ≤8 且名称在 {referer,origin,user-agent,accept,accept-language} 白名单内；`DownloadResult::validate` media ≤64、相对路径经 `validate_relative_media_path`（≤512、无绝对/盘符/`\`/`\0`、≤8 段）、size_bytes ∈ 1..=4 GiB。
- 跨语言 fixture 依赖（**P12 不得改动路径**）：`shared/protocol-schema/fixtures/sidecar-v2-command.jsonl` 与 `sidecar-v2-events.jsonl` 经 `include_str!("../../../shared/protocol-schema/fixtures/...")` 编译期内嵌（1510、1528），任何目录层级变化都会破坏构建。
- 平台相关测试辅助：`host_absolute_staging_dir` 1081 用 `std::env::temp_dir()` 派生绝对路径并断言 `is_absolute()`，注释解释“hard-coded POSIX path is not absolute on Windows”。
- 测试清单（14）：1127 `accepts_valid_extract_command_and_event`；1141 `rejects_missing_capabilities`；1154 `rejects_identity_mismatch_and_cookie_headers`；1173 `rejects_invalid_author_and_relationship_fields`；1200 `validates_discovery_command_and_candidate_events`；1266 `rejects_unknown_fields`；1290 `validates_download_command_and_events`；1315 `download_user_args_are_bounded_and_only_allowed_on_download`；1367 `download_command_matches_shared_user_args_json_fixture`；1390 `historical_download_command_without_user_args_still_deserializes`；1406 `download_command_requires_absolute_staging_dir`；1454 `rejects_unsafe_download_payloads_and_event_riding`；1505 `shared_fixtures_deserialize_and_validate`；1538 `requires_download_media_capability_for_handshake`。
- 测试 fixture：`host_absolute_staging_dir` 1081、`sample_result` 1090、`sample_download` 1272。

### 13. `crates/xarchive-download/src/supervisor.rs`（1218 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1218，补齐 80–170、340–420、600–640、835–920。无未读行。
- 生产区间 1–732；`#[cfg(test)] mod tests` 733–1218（14 个测试，其中 2 个为 `#[cfg(unix)]`）。**本文件 `#[ignore]` = 0** —— 全仓 2 个 `#[ignore]` 位于 `desktop/src-tauri/src/archive.rs`。
- `SECRET_FILE_COUNTER: AtomicU64` 18 —— ENG-13 注释明确“the aria2 RPC secret must never appear in the child process argv”；密钥经 `--conf-path` 短命文件传递，shutdown/drop 时删除，Windows ACL 硬化在 WQ-ENG-12 验证。
- 生产符号：`Aria2SupervisorConfig` 21（pub 字段 + 手写 `Debug` 55：`rpc_secret`／`proxy` 脱敏为 `[REDACTED]`，`user_args` 只报 `user_argument_count`）；方法 `new` 76、`new_with_random_secret` 88（32 字节 → 64 hex）、`build` 102（默认 startup 5s／request 15s／connect 30s／idle 60s／max_tries 3／`ProxyMode::System`）、`with_startup_timeout` 127、`with_secret_file_path` 132、`with_request_timeout` 137、`with_network` 143、`with_proxy` 157、`with_user_args` 185、`validate` 190、`command_args` 213、`environment` 242、`environment_remove` 250、`write_secret_file` 256；`Aria2Supervisor` 291（`child`／Linux `containment`／`client`／`secret_file`）、`Aria2StartupFailure` 302 + `retry_cleanup` 310、`spawn_contained` 344（`#[cfg(target_os="linux")]` + `#[deprecated(note="use spawn_contained_owned to retain failed startup cleanup owners")]`）、`spawn_contained_owned` 357、`with_secret_file_for_test` 459（`#[cfg(test)]`）、`spawn` 469、`backend` 526、`get_version` 530、`is_running` 534、`wait_for_exit` 561、`shutdown` 604、`Drop` 697；自由函数 `hide_console_window` 662、`configure_process_group` 674、`terminate_process_tree` 684、`wait_for_aria2` 703。
- 平台门控（必须逐字保留）：`hide_console_window` 在 Windows 用 `creation_flags(0x0800_0000)`；`configure_process_group` 在 unix 用 `command.process_group(0)`；`terminate_process_tree` 在 unix 用 `nix::sys::signal::kill` 向进程组 `-(child.id())` 发 `SIGTERM` → 25 ms → `SIGKILL`，再 `child.kill()`；`write_secret_file` 在 unix 用 `options.mode(0o600)`；`shutdown` 的 containment 分支只在 `target_os = "linux"` 编译。
- 不变量（逐字保留）：`command_args` 故意不含 `--rpc-secret`，`--conf-path` 由 spawner 在向量末尾追加；`user_args` 永远排在应用默认值之后；`ProxyMode::Direct` 时 `spawn` 额外遍历 `std::env::vars_os()` 用 `xarchive_core::is_proxy_environment_key` 大小写不敏感清除（489–496，注释解释 Windows 混合拼写）；`write_secret_file` 在 `sync_data()` 失败时删文件并返回错误；`spawn` 在 spawn 失败时删密钥文件；`wait_for_aria2` 失败路径先 `terminate_process_tree` + `wait` + 删密钥文件再返回；`is_running` 把“观察到已退出”当终态并在内部调 `shutdown()`（554–557）；`shutdown` 在 containment 未清理完时**保留** `Child` 并返回错误（619–622），密钥文件只在 `child.is_none()` 时删除（643–653），删除失败会把路径放回 `secret_file` 供重试。
- 测试清单（14）：763 `natural_parent_exit_stops_descendant_writer_and_removes_secret`（`#[cfg(unix)]`）；824 `startup_failure_owner_can_retry_real_secret_cleanup`；853 `shutdown_reports_secret_file_removal_failure_and_can_retry`；884 `spawn_preserves_argv_order_and_keeps_rpc_secret_out_of_argv`（`#[cfg(unix)]`）；1036 `random_rpc_secrets_are_fresh_hex_values`；1053 `defaults_apply_a_bounded_network_budget`；1067 `unified_network_settings_reach_the_command_line_without_the_proxy`；1129 起 `command_args_keep_application_order_without_user_args`（`args.last() == "--quiet=true"`）；1138 `proxy_reaches_aria2_through_the_environment_and_never_through_debug`；1164 `direct_strips_the_inherited_proxy_environment`；1188 `system_leaves_the_inherited_environment_for_aria2`；1202 `blank_or_whitespace_proxies_are_ignored_and_malformed_ones_rejected`。
- 测试 fixture：`supervisor_with_secret_file` 743、`config` 756；`spawn_preserves_argv_order_and_keeps_rpc_secret_out_of_argv` 在 887 行调用 `super::super::tests::lock_aria2_secret_temp_dir_for_test()` —— **跨模块测试依赖（crate 级 `tests` 的锁 fixture），搬迁本文件测试时必须保证该路径仍可解析。**

### 12. `crates/xarchive-workflow/src/writer_lifecycle.rs`（1030 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1030，补齐 98–160、600–650。无未读行。
- `//!` 文档 1–2：“Explicit writer lifetime contract. Drop is not evidence of termination. Adapters must confirm all descendants have exited, not just the parent.”
- 生产区间 1–477；测试：`mod lifecycle_tests` 479–562（4 个测试，`#[cfg(test)]`）、`mod tests` 563–1030（`#[cfg(all(test, target_os = "linux"))]`，6 个测试）。
- 生产符号：`VerifiedRecoveryDrain` 14（Linux-only 私有字段 `evidence`）、`PublicationError` 58（6 个变体）、`PreparedRecoveryPermit<'a,'coordinator,C>` 72、`RecoveryJournalIdentity<'a>` 107、`RecoveryJournalPhase` 120、`consume_predecessor_retirement` 41、`authorize_committed_recovery` 125、`authorize_prepared_recovery` 167、`WriterContainment` trait 245、`WriterState` 257、`WriterLifecycleError<E>` 266、`DrainedWriters<'a>` 272、`PublicationPermit<'a>` 285、`ProductionWriters` 302（`pub(crate)`）、`PublicationOwner` 322、`WriterRegistration` 331、`WriterLifecycle<C>` 433。
- **P12 处置依据（spawn/wait/terminate/Drop 留聚）**：`PublicationOwner::authorize` 必须与 `WriterRegistration::register_linux` 的 `state` 共享同一 `Arc<Mutex<ProductionWriters>>`，`close_admission` 的 `closed` 标志必须在注册与授权两侧同时可见；`DrainedWriters`/`PublicationPermit` 通过 `&'a` 借用把 owner 与 session 绑在一起，这是编译期所有权契约。拆文件不改变语义但会引入不必要的 `pub(crate)` 扩张，故保持单文件。
- 平台门控：`VerifiedRecoveryDrain` 字段与大部分 impl 均 `#[cfg(target_os = "linux")]`（15/19/40）；`authorize_committed_recovery` 与 `authorize_prepared_recovery` 都用 `#[cfg(not(target_os = "linux"))]` 直接返回 `PublicationError::IncompleteDrain`（130–134、172–176）；`register_linux` 396。
- 不变量（逐字保留）：`authorize_prepared_recovery` 的 7 步校验顺序（binding 匹配 → attempt 一致 → destination/workspace 一致 → journal_directory 必须非空/非绝对/全 `Normal` 组件 → job_id 一致 → phase 必须 `Prepared` → 构造 permit 后 `drop(evidence)`）；`authorize_committed_recovery` 同构但 phase 必须 `Committed`；`consume_predecessor_retirement` 校验 binding + attempt + job_id 后 `drop(evidence)`；`WriterLifecycleError::InvalidState` 与失败后 `state` 停在 `Starting`／`Stopping`（不自动回滚）；`stop_and_drain` 幂等；`ProductionWriters` 中 `published` 为私有、`closed`/`roles` 为 `pub(crate)`；`register_linux` 只允许角色 `"sidecar"`／`"aria2-0"`／`"aria2-1"`，且要求 `containment.matches_recorded(identity)`。
- 测试清单（10）：`lifecycle_tests` 内 502 `failed_start_requires_cleanup_and_cannot_restart`、522 `failed_drain_never_proves_exit_and_retry_keeps_admission_closed`、542 `production_drain_update_requires_a_registered_role`、554 `admission_close_does_not_erase_a_later_verified_drain`；`mod tests`（Linux）640 `recovery_authorization_rejects_mismatched_and_invalid_evidence`，以及重名的 `failed_start_requires_cleanup_and_cannot_restart` 810、`failed_drain_never_proves_exit_and_retry_keeps_admission_closed` 830、`production_drain_update_requires_a_registered_role` 850、`admission_close_does_not_erase_a_later_verified_drain` 与 1 个 retained-owner 测试。**同名测试在两个模块中形成两条独立路径（`lifecycle_tests::<name>` 与 `tests::<name>`），P12 不得去重。**
- 测试 fixture：`Fake`（`fail_start`／`fail_stop`）在两个模块各定义一次、`TestCoordinator`／`TestGuard` 579/580、`recovery_session` 600、`drain_evidence` 609、`journal` 623、`unique_record_root` 571、`static NEXT_DRAIN_FIXTURE: AtomicU64` 569。

### 11. `desktop/src-tauri/src/config.rs`（1814 行，P12；全文已读）

- 覆盖：1–200、201–400、401–600、601–800、801–1000、1001–1200、1201–1400、1401–1600、1601–1814，补齐 886–1000、1292–1317、1494–1515。无未读行。
- 生产区间 1–1065；`#[cfg(test)] mod tests` 1067–1814（34 个 `#[test]`、**0 个 `#[ignore]`**）。
- 顶层类型：`LogLevel` 18（`serde(rename_all="lowercase")`）、`LoggingConfig` 47、`StorageConfig` 76、`DownloadConfig` 91、`SidecarConfig` 115、`NetworkConfig` 155（11 个 pub 字段，含 `#[serde(skip)] proxy_mode_declared` 170）、`NetworkDiagnostics` 234（`#[allow(dead_code)]` + 仅 `Serialize`）、`ExtensionConfig` 425、`VerifiedTelegramCapability` 450、`CredentialRotationResumePolicy` 465、`TelegramConfig` 476、`TelegramSettings` 514、`AppConfig` 702、`ConfigurationExchange` 879（`pub(crate)`，`deny_unknown_fields`）、`ConfigurationExchangeSettings` 887（`pub(crate)`）、`ConfigurationExchangePreview` 1060（`pub(crate)`）。
- 常量：`DEFAULT_LOG_MAX_FILES` 12 = 5、`MIN_LOG_MAX_FILES` 13 = 1、`MAX_LOG_MAX_FILES` 14 = 100、`MIN_NETWORK_TIMEOUT_SECONDS` 135 = 1、`MAX_NETWORK_TIMEOUT_SECONDS` 136 = 86_400、`MIN_ARIA2_MAX_TRIES` 137 = 1、`MAX_ARIA2_MAX_TRIES` 138 = 100、`TELEGRAM_BOT_TOKEN_KEY` 443 = `"telegram.bot_token"`。
- 导出：`pub use xarchive_core::ProxyMode;` 145（re-export，P12 必须保留路径）。
- 关键方法：`NetworkConfig::{apply_legacy_migration 262, normalized_proxy 272, active_manual_proxy 284, redacted_proxy 293, secrets 299, sidecar_env 309, sidecar_env_remove 325, sidecar_args 330, diagnostics 340, validate 356}`；`validate_proxy` 396；`TelegramConfig::{endpoint 571, capability_matches 577, record_settings_change 587, replace_bot_token 596, delete_bot_token 625, bot_token_present 640, settings 655, validate 677}`；`declares_proxy_mode` 797；`AppConfig::{validate 806, log_secrets 825, database_path 829, cache_path 832, download_path 835, logs_path 838, load 842, save 866}`；`ConfigurationExchange::{persist_preview 901（`#[cfg(target_os = "linux")]`）, preview 945, parse 1031, export 1042}`。
- **P12 处置依据（R6 keep-together）**：`AppConfig`／`NetworkConfig`／`TelegramConfig` 的 serde 默认值、迁移与校验共享同一组私有 `default_*` 函数（68/71/83/86/107/110/124/128/131/190–210/436/531/535/539/543/723），且 `declares_proxy_mode` → `apply_legacy_migration` → `validate` 是加载时唯一顺序契约；拆分会把“默认值/迁移/校验”三件互相依赖的事分开。故仅整理测试，不拆生产代码。
- 不变量（P12 不得改变）：`AppConfig::load` 顺序为 deserialize → 由原始 YAML 推导 `proxy_mode_declared` → `apply_legacy_migration` → `validate`（846–857，注释明确指出“migration runs before validation”）；`save` 用 `.yaml.tmp` 临时名 + `rename`（870–872）；`persist_preview` 用 `create_new` + `sync_all` + `rename`，失败时删除临时文件，且**仅在 rename 成功后才 `*local = candidate`**（926–943）；`preview` 的 revision 为 `sha256:<hex>` 或字面量 `"missing"`（952–954），路径有效性要求非空、无控制字符、非 symlink、目录/文件类型匹配（990–999）；`restart_required` 只比较 download.directory／download.use_aria2／sidecar 三项（1019–1023）；`parse` 上限 64 KiB 且要求 `schema_version == 1 && kind == "tw2tg.configuration"`（1032–1038）。
- 脱敏契约：`TelegramConfig` 无 token 字段（471–475 注释）；`redacted_proxy` 输出 `http://[REDACTED]@host:port`；`secrets()` 供日志脱敏；`sidecar_args()` 永不含 proxy 值（1435–1441 测试断言）。
- 测试清单（34）：1073 `configuration_exchange_is_strict_and_never_exports_secret_categories`；1106 `configuration_exchange_atomic_persistence_rejects_stale_and_preserves_runtime_on_failure`；1154 `configuration_exchange_preview_preserves_missing_paths_and_binds_exact_bytes`；1183 `download_config_defaults_to_gallery_dl_direct_download`；1195 `telegram_rotation_defaults_do_not_enable_sending`；1210 `telegram_rotation_confirmation_survives_serialization`；1234 `token_replacement_verifies_before_mutating_secret_store`；1265 `token_delete_removes_only_the_secret_store_value`；1282 `a_fresh_config_defers_the_level_to_the_build_channel`；1295 `an_explicit_user_level_outranks_the_channel_default`；1302 `a_stored_level_survives_a_save_and_load_cycle`；1319 `a_config_without_a_level_follows_the_channel`；1332 `validates_log_file_range`；1341 `resolves_database_and_download_paths`；1359 `separates_worker_gallery_dl_and_extension_defaults`；1369 `worker_default_path_matches_portable_layout`；1381 `network_defaults_are_bounded_and_follow_the_system_proxy`；1405 `network_proxy_credentials_never_appear_in_diagnostics`；1454 `rejects_malformed_proxy_and_out_of_range_network_values`；1484 `app_config_validation_checks_the_network_section`；1512 `a_configuration_without_a_mode_loads_as_manual_and_keeps_its_value`；1529 `a_configuration_without_a_proxy_loads_as_system`；1538 `an_explicit_system_mode_is_not_migrated_away_from_a_stored_value`；1552 `an_explicit_mode_is_never_overwritten_by_the_migration`；1569 `a_pending_telegram_migration_survives_a_configuration_round_trip`；1592 `the_settings_projection_reports_the_pause_without_exposing_a_token`；1608 `only_the_manual_mode_hands_its_value_to_a_client_or_child`；1650 `direct_removes_the_inherited_proxy_environment_of_child_processes`；1677 `manual_requires_a_value_and_blank_values_do_not_count`；1690 `the_mode_survives_a_save_and_load_round_trip`；1716 `a_document_without_telegram_keys_loads_disabled`；1731 `an_enabled_telegram_section_must_be_complete_and_secure`；1760 `a_settings_change_advances_the_revision_and_drops_the_capability_record`；1790 `telegram_configuration_never_carries_the_bot_token`。
- 测试 helper：`load_document` 1496（唯一 helper；用 `raw.len()` 拼临时目录名，内部断言 `error == None`）。测试直接引用私有项 `declares_proxy_mode`、`validate_proxy`、`default_extraction_timeout_seconds`、`ConfigurationExchange::persist_preview`，因此**测试必须留在同模块内**，P12 只能移动到同文件的子模块，不能外置到集成测试。

### 9. `desktop/src-tauri/src/commands/records.rs`（1097 行，P7a；全文已读）

- 覆盖：1–250、251–500、501–750、751–1097，补齐 97–146、107–146、329–412、591–668、837–1013。无未读行。
- `//!` 文档 1–6 明确“Command names, argument shapes, serialized fields and error strings are the registered IPC contract and must not change here.” 生产区间 1–749；测试模块 `mod record_import_acceptance_tests` 750–1097（8 个 `#[test]`）。
- **P7a 硬约束（七个 tauri command 入口必须留在原处）**：`export_record_exchange` 525、`preview_record_import` 573、`cancel_record_import` 627、`accept_record_import` 639（`async`）、`export_configuration_exchange` 452、`preview_configuration_exchange` 460、`apply_configuration_exchange` 482。每个都带 `#[tauri::command]` 宏，宏生成的注册项不可用普通 `pub use` 复刻，因此 P7a 只把纯逻辑（export/import_document/import_evidence）移入 `commands/records/` 并保留原 handler 路径。
- 三段待提职责（P7a 建议）：
  - **export**：`serialize_batch_record_export` 176、`serialize_persisted_record` 239、`serialize_record_export_with_account` 334（`serialize_record_export` 329 已是 `#[cfg(test)]` 包装）。
  - **import_document**：`RecordImportDocument` 72 + `parse` 109／`records` 131／`evidence` 142、`RecordImportOutcomeResponse` 48 + `From` 77、`RecordImportPreviewEntry` 58、`RecordImportPreviewResponse` 66、`ConfigurationExchangePreviewResponse` 30、`ConfigurationExchangeApplyResponse` 36。
  - **import_evidence**：`record_import_root` 378、`record_import_revision` 389、`record_import_media_probe` 405、`verify_accepted_media_evidence` 423。
- 取消注册/等待同归属：`RecordImportWait` 165 + `Drop` 166、`static RECORD_IMPORT_CANCELLATION: Mutex<Option<CancellationToken>>` 174 —— 两者被 `accept_record_import`（662–672）与 `cancel_record_import`（629）共用，**必须与两个 command 一起留在原位或在拆分后共享同一模块**，否则静态量与 guard 失效。
- 平台门控：`apply_configuration_exchange` 与 `accept_record_import` 都用 `#[cfg(not(target_os = "linux"))]` 提前返回 `"... is not implemented on this platform"`／`#[cfg(target_os = "linux")]` 真实实现；Windows Owner 侧这两个命令恒返回错误字符串，属于既有契约，P7a 不得改动。
- 语义不变量：`record_import_revision` 在 `#[cfg(unix)]` 下混入 `dev()`/`ino()`（394–399），因此同一路径被替换后 revision 变化；`serialize_record_export_with_account` 拒绝私有 URL（`xarchive_core::redact(url, &[]) == url`）与非可移植媒体路径（绝对路径、`\\`、`:`、`ParentDir`）；`verify_accepted_media_evidence` 要求 accepted 非空、索引不重复且在范围内，任一记录 Preserve 即整体失败（“never import a valid prefix”）；`accept_record_import` 在持有 `WorkspaceGuard::drain` 时再次校验 revision，落库前检查 cancellation。
- 测试清单（8）：755 `cancellation_command_signals_waiter_and_drop_clears_registration`；766 `production_parser_accepts_both_formats_and_refuses_unversioned_input`；793 `batch_exports_all_available_records_in_both_formats`；914 `archive_parser_uses_final_directory_and_rechecks_changed_media`；966 `root_and_input_revision_changes_invalidate_preview`（内部 `#[cfg(unix)]` 断言 976–977）；983 `changed_media_rejects_the_whole_selection`（内部 `#[cfg(unix)]` 块 1046–1054）；1059 `invalid_selections_fail_before_import`；1069 `outcomes_have_flat_serialized_shapes`。

### 10. `desktop/src-tauri/src/transport.rs`（1007 行，P7b；全文已读）

- 覆盖：1–250、251–500、501–750、751–1007，补齐 336–420、588–665、849–925。无未读行。
- `//!` 文档 1–12 说明 Unix endpoint 由 Desktop runtime 注册，Windows Named Pipe 是独立的平台实现（roadmap E5，Windows Owner），默认须监听 `xarchive_protocol::WINDOWS_PIPE_ENDPOINT`。文件级 **`#![allow(dead_code)]` 在第 10 行**（夹在 `//!` 文档之间），P7b 拆分后必须在新模块根部保留。
- 生产区间 1–590；测试：`mod tests` 592–943（7 个测试）、`mod transport_limit_tests` 945–1007（`#[cfg(all(test, unix))]`，2 个测试）。
- **P7b 划分**：保留在原入口的是 `BrowserTransportAdapter` 35（6 个私有字段）与其构造/链式方法（`new` 45、`with_database_path` 56、`with_use_aria2` 70、`with_tool_compatibility` 75、`with_output_settings` 83）、`handle_request` 97、`handle_query_status` 281、`now_iso` 564、`error_response` 570、`error_error_code` 580 —— 即全部协议错误映射。要移到 `transport/unix.rs` 的是 `DesktopTransportServer` 337（`stop`／`endpoint`／`thread` 三私有字段）、其 `impl` 344（`MAX_ACTIVE_CONNECTIONS` 350 = 64、`CONNECTION_TIMEOUT` 352 = 15s、`#[cfg(test)] start` 355、`start_with_pairing` 370）、`Drop` 464、`handle_unix_connection` 475、`transport_endpoint` 558。
- Unix 专属不变量（搬迁不得改变）：endpoint 权限 `0o600`（396）、`set_nonblocking(true)`（399）、accept 线程名 `"xarchive-desktop-transport"`、每请求线程名 `"xarchive-desktop-transport-request"`、连接数与上限比较后 `continue`（414–418）、`set_read_timeout`／`set_write_timeout` 均为 `CONNECTION_TIMEOUT`（432–434）、`WouldBlock` 时 `sleep(25ms)` 而非退出（447–449）、`Drop` 中 `stop.store(true)` + `join` + `remove_file(endpoint)`（466–470）、`transport_endpoint` 读取 `xarchive_protocol::PIPE_ENDPOINT_ENV` 回退 `<portable_root>/cache/xarchive-v1.sock`（559–561）。
- 平台门控：所有 Unix 专属项都带 `#[cfg(unix)]`（16/19/25/336/343/463/474/557），`BrowserTransportAdapter` 与 `PathBuf` 导入则无条件编译（13–18 注释明确此点）。
- 测试清单（9）：619 `transport_preserves_request_id_on_archive_request`；659 `persisted_output_settings_are_used_by_default_browser_adapter`；766 `transport_reuses_existing_job_for_duplicate_archive_request`；799 `transport_replies_with_current_state_on_query_status`；850 `transport_reports_not_archived_for_query_status_with_no_matching_jobs`；879 `transport_reports_error_when_archive_request_validation_fails`；918 `transport_rejects_invalid_protocol_version_before_persistence`；957 `survives_a_burst_of_incomplete_connections`（`transport_limit_tests`）；996 `connection_limits_are_reasonable_for_local_clients`（同上）。

### 8. `desktop/src-tauri/src/batch.rs`（1968 行，P6；全文已读）

- 覆盖：1–250、251–500、501–750、751–1000、1001–1200、1201–1450、1451–1700、1701–1968，补齐 79–160、331–410、594–670、838–915、1101–1119、1284–1363、1536–1610、1790–1885。无未读行。
- 文件级 `//!` 文档 1–17 声明职责并给出关键语义：“A batch decides *which* Tweets to archive. The archive completeness decision stays with the single-Tweet commit path (`ArchiveService`), so a skip here only means … never 'the download is complete'.” 另有文件级内部属性 **`#![allow(dead_code)]`（18 行）**，P6 搬迁后必须在新模块根部重新声明。
- 生产区间 1–1055；两个测试模块：`mod d1_batch_tests` 611–660（1 个测试）与 `mod tests` 1057–1968（12 个测试），均 `use super::*`。
- 生产符号与 P6 四组划分：
  - **discovery**：`DiscoverySummary` 110、`discovery_request` 122、`candidate_record` 133、`persist_discovered_candidates` 156、`run_account_discovery` 199、`discovery_error_code` 320。
  - **selection**：`SKIP_ALREADY_ARCHIVED`/`SKIP_REPOST`/`SKIP_NO_MEDIA`/`SKIP_BEFORE_RANGE`/`SKIP_AFTER_RANGE`/`SKIP_BEYOND_LIMIT` 39–44（`pub(crate)`）、`SELECTION_SCAN_LIMIT` 47（512）、`BatchFilters` 55 + `to_json` 68／`from_json` 72／`exclusion` 81、`archive_facts_present` 345（注释标注“slow path kept for tests”）、`safe_media_path` 361、`candidate_already_archived` 384、`select_dispatch_candidates` 418、`apply_batch_limit` 462、`newest_first` 489、`mark_skipped` 499。
  - **submission**：`candidate_archive_request` 521、`inherited_execution_spec` 556、`MetadataBatchSnapshot` 591、`metadata_batch_settings` 596、`truncate_message` 335、`storage_error` 339。
  - **dispatch**：`BatchDispatchPass` 664、`BatchCancellationGuard` 669 + `Drop` 675、`dispatch_batch_pass` 687、`reconcile_batch_candidates` 812、`spawn_batch_dispatch` 883、`spawn_account_batch` 944（`#[allow(clippy::too_many_arguments)]`）。
- 不变量（P6 必须保持）：筛选顺序固定为 already-archived → `filters.exclusion` → approved，再 `newest_first` → `apply_batch_limit` → `truncate(limit)`；`unknown created_at` 永不被日期范围丢弃（88–103、1950 注释）；`filters.limit` 计入已 submitted+done，超限时最新者胜、最旧者标 `FILTERED_LIMIT`（456–483）；dispatch 只用 `evaluate_archive_completeness(fast())` 且明确不做 digest 校验（377–405）；`dispatch_batch_pass` 无 pending 时仍先校验不可变快照（712–718），`ExecutorError::QueueFull` 直接 `break` 使候选保持 PENDING（777）；`spawn_*` 用 `thread::Builder::name(...)` + `BatchCancellationGuard` 的 `same_instance` 检查保证新 generation 不被旧 guard 移除；`spawn_account_batch` 用 `ChildProcessEnvironment::new(sidecar_env, sidecar_env_remove)`，注释明确“a `Direct` batch worker cannot inherit a proxy from the launching shell”（986–994）。
- 环境变量依赖：`python_stub` 1590 与 1638 用 `std::env::var("PYTHON")` 回退 `python3`；`SidecarSupervisor::spawn(...).ok()` 失败时测试直接 `return`（**skip 而非 PASS**，1519–1522 注释明确此点）。
- 测试清单（13，行号）：616 `d1_batch_snapshot_dispatches_v4_and_refuses_future_or_unknown_fields`（`d1_batch_tests`）；1125 `child_execution_spec_inherits_persisted_batch_snapshot`；1206 `failed_batch_candidate_can_redispatch_through_persisted_scheduler`（`#[cfg(unix)]` 1204）；1290 `failed_batch_retry_resumes_to_attempt_owned_success`（`#[cfg(unix)]` 1288）；1411 `batch_cancellation_guard_keeps_newer_worker_generation`；1435 `batch_filters_use_product_defaults_and_reject_unknown_fields`；1455 `persists_discovery_candidates_once_and_binds_stable_identity`；1599 `runs_discovery_and_persists_streamed_candidates`；1634 `persists_each_discovery_candidate_before_completion`；1701 `records_a_failed_discovery_on_the_batch`；1802 `skips_candidates_whose_archive_is_already_on_disk`；1852 `filters_and_batch_limit_record_every_exclusion_reason`；1927 `date_filters_exclude_out_of_range_candidates_and_keep_unknown_dates`。
- 测试 fixture：`temp_root` 1068、`candidate_full` 1078、`outcome` 1101、`open_batch` 1108、`selected_ids` 1281、`python_stub` 1589、`batch_request` 1594、`archive_tweet` 1738；三个 Python stub 常量 `DISCOVERY_STUB_OK` 1523、`DISCOVERY_STUB_DELAYED` 1548、`DISCOVERY_STUB_AUTH_FAILURE` 1577；`const NOW` 1066。`failed_batch_retry_resumes_to_attempt_owned_success` 定义本地 `Factory`/`Execution` 并断言 `create_for_attempt` 被调用（`create` 处 `panic!("batch must bind an attempt")`）。

### 7. `crates/xarchive-storage/src/file_store.rs`（1266 行，P12；全文已读）

- 覆盖：1–250、251–500、501–750、751–1010、1011–1266，补齐 91–168、333–402、587–650、848–928、1101–1181。无未读行。
- 生产区间 1–751；测试模块 `#[cfg(test)] mod tests` 752–1266（`use super::*`，13 个 `#[test]`、0 个 `#[ignore]`）。**P12 处置依据：路径安全（`resolve_within`／`safe_child`／`safe_staging_child`／`is_reparse_point`）与全部文件操作共享同一套不变量，拆分会把安全边界切碎，故保持单文件。**
- 生产符号：`FileStore { archive_root, staging_root }` 8（两字段均私有）；方法 `new` 14、`with_staging_root` 20、`staging_dir` 34、`attempt_staging_dir` 59（`#[deprecated(since="0.1.0", note="use create_attempt_staging_dir ... or existing_attempt_staging_dir ...; do not claim a fresh attempt through this helper")]`）、`create_attempt_staging_dir` 81、`existing_attempt_staging_dir` 110、`recovery_attempt_directory_exists` 135、`attempt_staging_path` 159（私有）、`rename_staged_directory` 176、`existing_staging_dir` 186、`rename_staged_file` 196、`replay_attempt_rename_stage` 230、`recover_attempt_rename` 303、`recover_file_within` 319（私有）、`write_attempt_json` 357、`write_attempt_text` 368、`create_attempt_file` 381、`write_file_within` 392（私有）、`commit_attempt_staging` 421、`write_json` 448、`write_text` 461、`user_profile_path` 475、`write_user_profile` 489、`sha256` 498、`commit_staging` 512、`commit_recovered_staging` 535、`recovery_directory_exists` 564、`archive_path` 582、`resolve_within` 594（`pub(crate)`，文档明确供 `build_archive_metadata` 复用 WQ-ENG-03 容器契约）、`safe_child` 623（私有）、`safe_staging_child` 627（私有）。
- 平台条件编译（P12 必须逐字保留）：`#[cfg(unix)]`／`#[cfg(not(unix))]` 成对的原子 no-replace 分支（274–288、332–345、432–445，非 unix 一律 fail-closed 返回 `"atomic no-replace rename requires the platform adapter"`）；`#[cfg(unix)]` `is_reparse_point` 734 用 `is_symlink()`、`#[cfg(windows)]` 739 用 `file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT`、`#[cfg(not(any(unix, windows)))]` 747；`#[cfg(windows)] fn windows_safe_component` 677（保留 `CON`/`PRN`/`AUX`/`NUL`/`COM1-9`/`LPT1-9` 设备名，`%XX` 转义非法/尾随点空格/保留前缀，`%` 自身也转义）；`create_attempt_file` 用 `OpenOptions::create_new(true)` + Unix `O_NOFOLLOW`(0x0002_0000) / Windows `FILE_FLAG_OPEN_REPARSE_POINT`(0x0020_0000) + `sync_all()`（402–418）。
- 安全不变量：`resolve_within` 拒绝绝对路径与 `ParentDir`/`RootDir`/`Prefix`，并逐组件 `symlink_metadata` 拒绝任何 reparse point，遇 `NotFound` 即停（不创建）；`safe_staging_child` 拒绝空 job_id、含 `/` 或 `\`、组件数 ≠ 1、非 `Normal` 组件；`commit_staging` 拒绝已存在目标；`commit_recovered_staging` 区分 `NotFound`（rename）/ 已存在目录（`AlreadyExists`）/ 其它（`InvalidPath`）；`verified_regular_file` 655 同时校验 `is_file`、非 reparse、尺寸、SHA-256。
- 测试清单（13，文件内行号）：771 `rejects_parent_and_absolute_paths`；787 `attempt_rename_recovery_is_idempotent_and_checks_identity`；840 `attempt_rename_recovery_rejects_conflicts_and_missing_edges`；894 `allows_a_missing_intermediate_directory`；906 `attempt_staging_isolates_concurrent_attempts`；926 `fresh_attempt_staging_creation_never_reuses_an_existing_attempt`；955 `fresh_attempt_staging_rejects_a_symlinked_job_parent`（`#[cfg(unix)]`）；976 `fresh_attempt_staging_rejects_a_symlinked_staging_root`（`#[cfg(unix)]`）；1001 `attempt_file_creation_never_overwrites_existing_files_or_links`（内部含 `#[cfg(unix)]` 块 1021–1035）；1040 `replay_attempt_stage_moves_pending_edges_atomically`（内部含 `#[cfg(unix)]` 块 1136–1169）；1177 `rejects_an_intermediate_symlink_that_points_outside_the_root`（`#[cfg(unix)]`）；1203 `existing_attempt_staging_rejects_symlinked_ancestors`（`#[cfg(unix)]`）；1243 `rejects_a_final_symlink_component`（`#[cfg(unix)]`）。→ 8 个 `#[cfg(unix)]` 测试在 Windows 上不编译，Windows Owner 侧计数必然更低，基线比对时必须按平台分别核对。

### 6. `crates/xarchive-telegram/src/upload.rs`（1033 行，P12；全文已读）

- 覆盖：1–210、211–420、421–630、631–840、841–1033，补齐 84–122、310–332、498–545、721–762、920–943。无未读行。
- 文件级 `//!` 文档明确职责边界（1–8）：“owns the upload half of the Bot API transport: layered upload timeouts, ordered phase reporting, the pull-based streamed body, the album watchdog and the per-media send call. It does not own the blocking control client, the outbox state machine or any retry decision.” 并声明“The crate root re-exports every public item, so call sites are unchanged.” —— P12 处置依据：职责单一、已内聚，**保持单文件**。
- 公开 API（12 项）：`TELEGRAM_UPLOAD_CHUNK_BYTES` 36（64 KiB）、`UploadTimeouts` 54（4 个 pub 字段 + `validate()` 64 为 `pub(crate)` + `Default` 84：connect 10s／body_stall 60s／server_processing 300s／total None）、`UploadStage` 102（`Queued`／`CheckingFile`／`Uploading{sent_bytes,total_bytes}`／`AwaitingResult`／`Confirmed`）、`UploadRequest` 112（7 个 pub 字段）、`validate_upload_request` 130（`pub(crate)`）、`upload_method` 161（`pub(crate)`）、`AlbumItemUpload` 392（`File(Box<UploadRequest>)`／`CachedFileId{media_kind,file_id,caption}`）、`send_media_group_attempt` 407（泛型 `P: FnMut(UploadStage) + Send + 'static`，`#[allow(clippy::too_many_arguments)]`）、`upload_watchdog` 654（`pub(crate)`）、`upload_watchdog` 与 `album_watchdog` 354（私有）两套 watchdog、`ReqwestTelegramTransport::send_upload` 970、`send_upload_attempt` 991。
- 私有内部件：`UPLOAD_PHASE_PRE_BODY`／`STREAMING`／`FINISHED` 170–172、`UPLOAD_WATCHDOG_TICK` 177（25 ms）、`UploadTracker` 181（7 个原子字段，6 个方法）、`BodyProgressSink` trait 261（`on_chunk`／`on_item_finished`／`snapshot`）及其两个实现（`UploadTracker` 268、`AlbumProgress` 334）、`AlbumProgress` 284、`TrackedUploadStream` 706 + `impl Stream` 713、`UploadProgressCallback` 694 = `Box<dyn FnMut(UploadStage) + Send>`、`emit_upload_progress` 696（`Mutex::lock().expect("upload progress callback")`）、`map_upload_send_error` 747（严禁格式化 `reqwest::Error`，因 URL 含 token）、`read_bounded_upload_response` 763、`progress_from_phase` 800、`ReqwestTelegramTransport::{upload_client 812, send_upload_outcome 843}`。
- 不变量（P12 必须保持）：`Uploading{sent == total}` 不等于已发送，只有 `Confirmed`（解析到 `ok: true` 之后）才是发送成功（97–100）；取消语义 `UploadCancelled` 且已发请求不得自动重试（834–838）；所有回调在 `Arc<Mutex<..>>` 后共享，`Confirmed` 与 `AwaitingResult` 由 body 流与最终阶段共同发出；`send_media_group_attempt` 在任何写线前先预开全部文件（444–505，`AlbumItemUpload::File` 全部校验+打开+尺寸核对），任一失败返回 `NotSent`。
- 依赖：`futures_core::Stream`、`tokio::fs`／`tokio::time`、`tokio_util::io::ReaderStream`、`bytes`、`reqwest::multipart`、`serde_json`；`crate::contract::{BotToken, TelegramError, TelegramResponse}`、`crate::formatting::MediaKind`、`crate::retry_policy::RequestProgress`、`crate::transport::{ReqwestTelegramTransport, TELEGRAM_MAX_RESPONSE_BYTES, validate_chat_id, validate_thread_id}`、`crate::{CancellationToken, SendAttemptError, SendAttemptSuccess, TELEGRAM_ALBUM_MIN_ITEMS, TELEGRAM_MEDIA_GROUP_LIMIT}`。该文件的测试在 crate root 测试模块内（见第 7 条），文件本身无 `#[cfg(test)]` 模块。

### 5. `crates/xarchive-storage/src/archive_recovery.rs`（1097 行，P11；全文已读）

- 覆盖：1–460（首轮）、98–300、185–215、301–390、461–700、538–624、701–1097、751–880、881–1016。拼接连续，无未读行。
- 导入 1–5：`crate::archive_recovery_manifest::{ArchiveRecoveryManifest, ArchiveRecoveryManifestV2}`、`crate::{Database, StorageError}`、`rusqlite::OptionalExtension`、`sha2::Digest`、`xarchive_core::ArchiveMetadata`。
- 公开类型：`ArchiveRecoveryContract` 8（`InternalV1`／`InternalRenameV2`，`const fn version()` 14 返回 1／2）；`ArchiveRecoveryDispatch` 26（`Legacy`／`InternalV1`／`InternalRenameV2`／`Unknown(i64)`）；`ArchiveRecoveryPhase` 34（`Prepared`／`Committed`，`as_str()` 54）；`ArchiveCancellationDecision` 42（`Cancelled`／`CommitWins`／`AlreadyDecided`）；`ArchiveRecoveryJournalRecord` 111（12 个 pub 字段）；`ArchiveRecoveryManifestV2Record` 127（10 个 pub 字段，含 `rename_progress`）。
- 两个 `impl Database` 块：209–207（contract/journal 读取）与 144–207（dispatch）。P11 分组依据（四个职责）：
  - **dispatch**：`startup_recovery_dispatch` 151、`archive_recovery_contract` 210、`internal_v1_recovery_identity` 81、`v2_journal_state` 222（私有）、`archive_commit_started` 247、`decide_archive_cancellation` 270、`internal_v1_recovery_job_ids` 63、`declared_recovery_contract_job_ids` 72。
  - **V1 journal**：`create_archive_recovery_manifest` 792、`archive_recovery_journal` 864、`verify_archive_recovery_manifest` 878、`list_prepared_archive_recovery_manifests` 919、`list_internal_v1_recovery_jobs` 933、`internal_v1_recovery_tweet_row_id` 942、`commit_archive_recovery_manifest` 955、`recover_internal_v1_rows_and_commit_manifest` 977（`pub(crate)`）。
  - **V2 journal/manifest**：`prepare_archive_recovery_v2` 320、`archive_recovery_v2_candidate` 447、`archive_recovery_manifest_v2` 478、`transition_archive_recovery_v2` 570、`finalize_archive_recovery_v2` 635。
  - **行映射 helper**（文件级私有 `fn`）：`journal_from_row` 1050、`archive_recovery_v2_record_from_row` 1073。
- 事务与 fence 不变量（逐字保留）：`decide_archive_cancellation` 用 `unchecked_transaction` + 三态判定，无写入路径靠 drop rollback（291）；`prepare_archive_recovery_v2` 三重校验（media facts 一致 → contract/attempt/tweet 身份 `Some((2, attempt, tweet_id))` → 状态可转移）后才 `INSERT ... ON CONFLICT DO NOTHING`；`archive_recovery_manifest_v2` 十项完整性校验（含 `manifest.digest() != record.manifest_sha256`、`rename_phase != "PLANNED"`、`rename_progress` 三值、COMMITTED 必须 FINAL、contract == 2、tweet 身份双向一致、attempt 与 jobs 一致），PREPARED 时强制候选存在且媒体匹配；`transition_archive_recovery_v2` 只允许 `PLANNED→TEMPORARY→FINAL→COMMITTED`，幂等重放返回 current，CAS 失败报 `"v2 recovery phase compare-and-set lost"`；`finalize_archive_recovery_v2` 在单事务内校验 journal `(attempt, "PREPARED", "FINAL", manifest_json)` 完全匹配 → `jobs JOIN tweets` fenced 查询 → 状态白名单 → 逐级 `UPDATE ... WHERE state = ?` 且 `changed != 1` 即报 `"v2 Job transition was fenced"` → intent ARCHIVED → journal COMMITTED CAS → `DOWNLOAD_COMPLETED` 事件（payload 含 `files_copied`）→ 删除 candidate；`recover_internal_v1_rows_and_commit_manifest` 11 项身份校验 + 事务内 `recover_archive_rows_from_manifest_in_transaction` + journal CAS（`changed != 1` 报 `"InternalV1 journal finalization compare-and-set failed"`）。
- `CROSS_PLATFORM_CHANGE_REQUIRED` 标记保留在 149（dispatch 分流）与 1033＋770（ADR-015 原子边界）；这些注释是契约信号，搬迁时必须随代码一起移动，不得删除。
- 依赖注入方向：`archive_recovery.rs` 只依赖 `crate::{Database, StorageError}` 与 manifest 类型，不引用 `ArchiveService`／`FileStore`，因此 P11 拆目录时不需要反向依赖 P10 产物（与 `archive_service.rs` 单向依赖方向相反，可作为拆分顺序依据）。

### 4. `crates/xarchive-storage/src/archive_service.rs`（1614 行，P10；全文已读）

- 覆盖：1–480（首轮，含 94 起的截断）、94–340、176–258、341–590、422–508、591–840、682–754、841–1090、928–1014、1091–1350、1166–1274、1351–1614、1438–1540。全部区间连续可拼接，无未读行。
- 顶层条目：导入 1–12（`serde`、`std::fs`、`std::path::{Path, PathBuf}`、`xarchive_core::{ArchiveMetadata, JobState}`、`crate::{ArchiveRecoveryContract, ArchiveRecoveryManifest, ArchiveRecoveryManifestV2, ArchiveRecoveryPhase, BatchOutputSettings, Database, FileStore, StorageError, TelegramArchiveIntentRecord, UserProfileFile, build_archive_metadata}`、`xarchive_workflow::PublicationPermit`）；`pub(crate) const RENAME_PLAN_FILE` 18（值 `.xarchive-rename-plan.json`）；`pub struct PendingRename` 23–29（4 个 pub 字段，derive Debug/Clone/Serialize/Deserialize/PartialEq/Eq）；`pub struct ArchiveService { pub database: Database, pub files: FileStore }` 31–34；`pub struct SidecarArchiveRequest<'a>` 39–51（9 个 pub 字段，生命周期 `'a`）；`pub struct ArchivePreview` 53–57；`pub enum ArchiveRecoveryFailpoint` 59–64（`AfterTemporaryRename(usize)`、`AfterFinalRename(usize)`、`AfterDirectoryCommit`）。
- `impl ArchiveService`（66–1614）全部方法与真实可见性：
  - 公开：`new` 67、`preview_sidecar_archive` 71、`preview_sidecar_archive_attempt` 94、`complete_local_archive` 122、`complete_sidecar_archive` 263、`complete_sidecar_archive_with_failpoint` 270、`publish_internal_v1_attempt` 345、`recover_telegram_archive_intent` 896、`recover_staging_archive` 1056、`recover_internal_archive` 1084（恒返回 `InvalidState`，迁移期 fail-closed 入口）、`recover_internal_archive_attempt` 1098（`#[deprecated(note = "use replay_prepared_internal_v1 with a Workflow permit")]`）、`verify_committed_internal_v1` 1186、`replay_committed_terminal_internal_v1` 1238、`verify_committed_terminal_internal_v1` 1252、`replay_prepared_internal_v1` 1265（泛型 `<C: xarchive_workflow::coordination::Coordinator>`，参数 `PreparedRecoveryPermit<'_, '_, C>`）、`converge_committed_internal_v1` 1360、`recover_internal_rename_v2` 1449。
  - `pub(crate)`：`publish_prepared_internal_rename_v2` 611（`#[allow(dead_code)]`）、`publish_prepared_internal_rename_v2_with_exports` 635（`#[allow(dead_code)]` + `#[allow(clippy::too_many_arguments)]`）、`complete_sidecar_archive_attempt_for_test` 369（`#[cfg(test)]`）、`publish_prepared_internal_rename_v2_for_test` 660（`#[cfg(test)]`）、`publish_prepared_internal_rename_v2_with_exports_for_test` 680（`#[cfg(test)]`）。
  - 私有：`refresh_author_profile` 212、`complete_sidecar_archive_attempt_inner` 388、`build_pending_renames` 524、`apply_pending_renames` 559、`publish_prepared_internal_rename_v2_core` 701（`#[allow(clippy::too_many_arguments)]`）、`finish_committed_archive` 989、`validate_recovery_metadata` 1016、`verify_internal_attempt_recovery_tree` 1402。
- P10 分组候选（按完整操作，非按行数均分）：① 正常 preview 与旧完成路径 `preview_sidecar_archive`／`preview_sidecar_archive_attempt`／`complete_local_archive`／`refresh_author_profile`／`complete_sidecar_archive`／`complete_sidecar_archive_with_failpoint`；② InternalV1 发布与恢复 `publish_internal_v1_attempt`／`complete_sidecar_archive_attempt_for_test`／`complete_sidecar_archive_attempt_inner`／`build_pending_renames`／`apply_pending_renames`／`recover_internal_archive`／`recover_internal_archive_attempt`／`verify_committed_internal_v1`／`replay_committed_terminal_internal_v1`／`verify_committed_terminal_internal_v1`／`replay_prepared_internal_v1`／`converge_committed_internal_v1`／`verify_internal_attempt_recovery_tree`；③ V2 rename 发布与恢复 `publish_prepared_internal_rename_v2` 系列与 `recover_internal_rename_v2`；④ legacy/Telegram intent 恢复 `recover_staging_archive`／`recover_telegram_archive_intent`／`finish_committed_archive`／`validate_recovery_metadata`。
- 跨组私有依赖（禁止扩大 API）：`publish_prepared_internal_rename_v2_core` 被 4 个入口共用（2 个 `#[cfg(test)]` + 2 个 `pub(crate)`），拆目录后它必须与共用入口同层或改为同级 `pub(super)`；`verify_internal_attempt_recovery_tree` 同时被 `replay_prepared_internal_v1`（1337）与 `verify_committed_internal_v1`（1234）调用；`refresh_author_profile` 同时被 `complete_local_archive`（190）与 `finish_committed_archive`（1001）调用；`build_pending_renames`/`apply_pending_renames` 被 InternalV1 发布（438/479）与 legacy 恢复（1163/1333）共用。
- 关键不变量（迁移期间逐字保留）：`publish_prepared_internal_rename_v2_core` 中 v2 enrollment 闸门 `archive_recovery_contract != Some(2)` 必须拒绝（742）；D1 exports 校验顺序（`manifest.metadata_output` → YAML 选择匹配 → `export_text` 禁止 + 名称去重 → `export_json` 与 `tweet.json` 一致）；`create_attempt_file` 只在 journal 缺失时写入（768–812）；`AfterTemporaryRename`／`AfterFinalRename`／`AfterDirectoryCommit` 三个 failpoint 位置（848、862、871）与错误文案（`"injected v2 publish crash after temporary/final/directory commit"`）；InternalV1 对应 `"injected v1 publish crash after rename"`（576）、`"injected v1 publish crash after directory commit"`（494）。
- 契约与所有权：`PublicationPermit<'_>` 按值消费（`publish_internal_v1_attempt` 借用到 `complete_sidecar_archive_attempt_inner`，`replay_prepared_internal_v1` 在 1307 调 `permit.consume(attempt, &destination, final_directory)`）；`AttemptIdentity` 校验四个分量（job_id、attempt、destination、current attempt）。这两处语义在任何搬迁中不得改变。

### 3. 父模块 `crates/xarchive-storage/src/tests/mod.rs`（403 行，P3 硬约束）

- 覆盖：1–200、201–403（两窗口，全文）。文件级 `//!` 说明“按职责分组、共享 fixture 与导入是唯一继承项”。
- 8 条子模块声明（6–12）：`archive_recovery`、`archive_recovery_manifest`、`archive_service`、`database`、`database_telegram`、`telegram_credentials`、`telegram_outbox`（7 个，与实际目录内 8 个 `.rs` 中除 `mod.rs` 外一致）。
- 父层导入（14–24）：`use super::*;`、`rusqlite::params`、`std::fs`、`std::path::{Path, PathBuf}`、`std::time::{SystemTime, UNIX_EPOCH}`、`xarchive_core::{ArchiveMetadata, JobState}`、`xarchive_telegram::{CachedFileId, FileCacheKey, FileIdCacheStore, MediaKind, NewOutboxEntry, OutboxState, SendState, SendStateError, SendStateStore, SentSendRecord, TELEGRAM_FILE_CACHE_VERSION, TelegramOutboxStore, UNKNOWN_REASON_LEASE_EXPIRED}`。子文件 `use super::*;` 继承的正是这一组。
- 父层共享 fixture/helper（P3 不得移动或改名）：`temp_root` 26、`check_archive_intent_recovery` 35、`unprepared_v2_publish_inputs` 183（带 `#[allow(clippy::type_complexity)]`）、`prepared_v2_rename_fixture` 289、`new_candidate` 297、`insert_alice_batch` 312、常量 `BOT_A` 326／`BOT_B` 327／`OUTBOX_NOW` 328／`OUTBOX_LEASE` 329／`OUTBOX_LATER` 330／`OUTBOX_EXPIRED` 331、`outbox_entry` 332、`cache_key` 352、`test_candidate_metadata` 361、`manifest` 384。
- **同名遮蔽风险（P3 最高优先级发现）**：父层 361 行的 `test_candidate_metadata` 与 384 行的 `manifest`，和 `archive_recovery.rs` 内 8 行／30 行的同名私有 helper 重复。当前 Rust 名称解析下子模块内定义遮蔽父层导入，因此行为确定；但若把 `archive_recovery.rs` 拆成 `src/tests/archive_recovery/` 子目录并把这两个 helper 留在父 `mod.rs`，遮蔽关系会改变（搬走的使用点将静默改为父层版本，两者 `text` 文案不同：子层 `"recovery candidate"` 对父层 `"recovery candidate"` 相同，但父层 `manifest` 的 `tweet_row_id` 固定为 1、`archive_directory` 固定 `Tweets/123`，与子层一致）。结论：这两个 helper 必须随使用点一起留在同一模块层级，或在拆分后显式改名，禁止依赖遮蔽。
- `journal_snapshot`：`archive_service.rs` 使用了它但 `mod.rs` 中未定义（grep 全文无定义）。P3 前必须定位其真实定义位置（可能在其他子模块或 crate 内部），否则分组会破坏解析。

### 22. `crates/xarchive-telegram/src/lib.rs`（2691 行，P2；全文已读）

- 覆盖：1–2691（首轮 500–550 行分页 + 本轮补齐 1–50、205–270）。生产区间 1–215；测试模块 `#[cfg(test)] mod tests` 216–2691。
- 文件级 `//!` 文档 1–5：“Telegram Bot API contracts, formatting primitives, and HTTPS transport.”；“The Windows Credential Manager adapter remains outside this crate. The network transport uses blocking reqwest with Rustls and keeps the token out of error messages and debug output.”
- 模块布局（11–20）：`mod contract; delivery; file_cache; formatting; outbox; planning; projection; retry_policy; transport; upload;`（10 个生产子模块）。**P2 只搬测试，这 10 条 `mod` 声明与 22–39 的 `pub use` 段必须逐字保留。**
- re-export（22–39 + 214）：`file_cache::{CachedFileId, FileCacheKey, FileIdCacheStore, TELEGRAM_FILE_CACHE_VERSION, is_invalid_file_id_error}`；`outbox::{NewOutboxEntry, OutboxEntry, OutboxState, RunAttemptError, SendAttemptError, SendAttemptSuccess, TelegramOutboxStore, run_claimed_attempt, run_claimed_attempt_with_clock}`；`planning::{PlannedMediaItem, PlannedSend, SendPayload, plan_media_sends, plan_text_send}`；`transport::{ReqwestTelegramTransport, TELEGRAM_MAX_RESPONSE_BYTES, validate_message}`；`upload::{AlbumItemUpload, TELEGRAM_UPLOAD_CHUNK_BYTES, UploadRequest, UploadStage, UploadTimeouts, send_media_group_attempt}`；`projection::{SendProjection, message_link, outbox_projection}`；另 `pub use tokio_util::sync::CancellationToken;`（9）。
- 根级生产 fn 仅 2 个：`sha256_hex`（46，私有）与 `validate_key`（131）；其余为 `SecretStore` trait 与其实现（64–124，**保持原位，P2 不拆**）。
- 测试模块导入（218–227）：`std::time::Duration`、`use super::*;`、`crate::transport::{json_error, request_payload}`、`crate::upload::{UploadTracker, upload_method, upload_watchdog, validate_upload_request}`、`std::io::{Read, Write}`、`std::net::TcpListener`、`std::path::PathBuf`、`std::sync::{Arc, Mutex}`、`std::thread`。
- **P2 可见性核实结论（本轮实测修正）**：`transport.rs` 的 `request_payload`（263）与 `json_error`（330）均为 `pub(crate) fn`，`upload.rs` 的 `validate_upload_request`（130）、`upload_method`（161）与 `UploadTracker`（181）亦为 `pub(crate)`。此前台账称前两者为“私有 fn”属误记，已更正。结论：这些符号在 crate 内任意模块可见，因此把对应测试移入 `src/tests/` **不需要**扩大任何生产可见性，P2 不触发“为搬测试扩大 API”的禁令。P2 仍须保持 `src/lib.rs` 的 10 条 `mod` 声明与全部 `pub use` 逐字不变。
- 测试计数：**47 个（46 `#[test]` + 1 `#[tokio::test]`），`#[ignore]` = 0，`#[cfg]` 门控测试 = 0，无子进程/`current_exe()`/env 变量。**
- 测试 fixture/helper（10 个，全部在 `mod tests` 内）：`fake_server` 271、`sample_upload_request` 882、`temp_upload_file` 1163、`read_http_request` 1174、`parse_multipart_request` 1217、`upload_http_server` 1309、`block_on` 1340、`schedule_label` 2050、`planned_item` 2317、`album_upload_request` 2530。

- P2 分类候选（按测试实际断言对象，非按行数均分；逐项归属须在 P2 首个微批复核）：SecretStore 1（230）；contract/transport/endpoint/proxy 6（295、324、359、383、427、481、490、537、551 中按 endpoint 语义归组）；upload/timeout/method 5（896、952、987、997、1349、1399、1422、1457、1534、1562）；idempotent delivery 4（668、678、704、740）；endpoint migration 2（1079、1120）；retry/outbox 4（1595、1732、1742、1834）；planning 3（2295、2330、2403）；album/cache 3（2482、2544、2621）；diagnostics 2（868、1769、1789）。**此表为候选分组，P2 执行时必须按方法逐项核验，不得按区间粗暴切分。**

- 测试名与行号（源码顺序，46 `#[test]` + 1 `#[tokio::test]`）：230 `stores_and_deletes_secrets_without_exposing_values`；247 `validates_message_boundaries`；295 `sends_message_over_https_transport_contract`；324 `single_upload_contains_exactly_one_topic_field`；359 `a_control_request_with_a_lost_response_is_not_safe_to_retry`；383 `sends_photo_video_and_media_group_methods`；427 `maps_telegram_api_and_http_errors_without_exposing_token`；481 `rejects_non_https_production_endpoints`；490 `a_resolved_proxy_actually_receives_the_request`；537 `a_disabled_proxy_still_builds_a_direct_client`；551 `a_malformed_proxy_is_rejected_rather_than_silently_ignored`；668 `parses_result_message_id_from_bot_api_response`；678 `idempotent_send_skips_transport_when_already_sent`；704 `idempotent_send_records_delivered_state_and_stays_stable`；740 `idempotent_send_persists_failure_and_retries_with_same_key`；801 `accepts_cloud_and_loopback_local_endpoints`；831 `rejects_endpoints_that_could_redirect_or_leak_the_token`；868 `secret_store_error_variants_do_not_carry_the_secret`；896 `validates_upload_request_boundaries`；952 `validates_upload_timeout_layers`；987 `maps_media_kinds_to_upload_methods_and_fields`；997 `extracts_message_and_file_ids_from_success_results`；1020 `verified_bot_identity_is_stable_across_credentials_and_rejects_non_bots`；1079 `api_endpoint_contract_gates_local_http_production_transport`；1120 `migration_source_and_target_apply_endpoint_specific_proxy_policy`；1349 `watchdog_classifies_the_first_layered_deadline`（唯一 `#[tokio::test]`）；1399 `upload_short_circuits_before_the_network_when_cancelled`；1422 `upload_reports_missing_and_changed_files_before_any_network`；1457 `send_upload_streams_the_file_and_reports_ordered_progress`；1534 `send_upload_rejects_an_oversized_response_before_reading_it`；1562 `send_upload_classifies_a_bot_api_error_result`；1595 `classifies_send_failures_into_the_retry_policy`；1732 `bounded_backoff_grows_then_caps`；1742 `outbox_states_roundtrip_and_gate_automatic_sending`；1769 `bot_identity_is_stable_reversible_free_and_token_sized`；1789 `invalid_file_id_detection_is_conservative`；1834 `failed_attempts_map_onto_the_next_durable_transition`；2055 `clock_aware_attempt_reads_time_before_and_after_io`；2081 `attempt_driver_fences_before_sending_and_records_a_confirmed_result`；2106 `attempt_driver_persists_the_retry_policy_of_each_failure_class`；2267 `attempt_driver_drops_a_lost_claim_without_sending_or_writing`；2295 `media_planner_splits_document_runs_without_reordering`；2330 `media_plans_promote_a_trailing_single_and_keep_stable_keys`；2403 `a_planned_send_fingerprints_its_content_and_becomes_an_outbox_entry`；2482 `send_upload_attempt_reports_confirmed_ids_and_honest_progress`；2544 `an_album_streams_every_file_and_reuses_a_cached_file_id`；2621 `an_album_outside_the_legal_range_is_refused_before_any_request`。
- 断言证据（P2 不得删改）：`a_resolved_proxy_actually_receives_the_request`（490）、`upload_http_server`（1309）、`parse_multipart_request`（1217）组成真实 loopback HTTP 服务器 fixture，验证 multipart 边界与 progress 顺序；`send_upload_rejects_an_oversized_response_before_reading_it`（1534）锁定 `TELEGRAM_MAX_RESPONSE_BYTES` 前置检查；`maps_telegram_api_and_http_errors_without_exposing_token`（427）与 `secret_store_error_variants_do_not_carry_the_secret`（868）锁定 token 永不出现在错误/Debug 输出；`idempotent_send_*` 三项（678/704/740）使用 `mod tests` 内自定义 `SendStateStore` 实现（575–666 段：`find_sent`／`record_pending`／`record_sent`／`record_failed`／`list_unsent`）；`failed_attempts_map_onto_the_next_durable_transition`（1834）内含 `SendStateStore`/`TelegramOutboxStore` 双重实现的 mock（1914–2045 段）。

### 23. `desktop/src-tauri/src/telegram_send.rs`（3153 行，P5；全文已读）

- 覆盖：生产 1–1274（本轮补齐 60–470、147–380、470–920、920–1280、1015–1160；首轮已覆盖 1275–3153）+ 测试 1275–3153（测试名/helper 已全量定位）。无未读行。
- 文件级 `//!` 文档 1–10：“Telegram sending core (plan TG-06 shared part). The Desktop runtime decides *when* this runs; this module owns *how* a finished archive becomes queued sends and how due entries are executed. Automatic sending stays off until the user enables it (plan §2.1) … An outbox row stores archive-relative media paths. The desktop resolves those paths against recorded archive facts before any transport request.”
- 文件级内部属性：**`#![allow(dead_code)]`（12 行，注释“Consumed by the Telegram settings/send surface (Batch B)”）** —— P5 拆分后必须在每个新模块根部重新声明或收敛到父模块，否则出现 warning。
- 生产符号与 P5 分组（按真实 `fn` 边界）：
  - **credentials**：`apply_credential_resume_policy` 34、`activate_verified_credential` 76、`activate_verified_credential_with_policy` 100（`#[allow(clippy::too_many_arguments)]`）。
  - **intent_planning**：`auto_send_enabled` 164、`ArchiveSendIntent` 175（9 pub 字段）+ `to_journal_record` 192／`from_archive_metadata` 225／`from_verified_config` 291、`safe_relative_path` 319、`queue_archive_sends` 330、`ArchivedIntentQueueResult` 340、`reconcile_archived_send_intents` 345、`queue_persisted_archive_sends` 365、`archive_plan_entries` 402、`SendRunSummary` 149、`MAX_SENDS_PER_RUN` 141（10）、`CLAIM_LEASE` 145（300 s）。
  - **payload**：`restore_payload_from_entry` 472、`resolve_archived_payload` 481、`recover_expired_claims` 539。
  - **runner/delivery**：`run_due_sends_with_generation` 618、`with_claim_heartbeat` 744、`with_claim_heartbeat_interval` 757、`upload_timeouts` 905、`claim_token` 916、`execute_payload` 926、`send_single_media` 1042（`#[allow(clippy::too_many_arguments)]`）、`album_upload` 1098、`cached_media_request` 1133、`confirm_cached_send` 1157、`control_progress` 1178、`run_active_archived_sends` 827、`run_active_archived_sends_cooperative` 851（`#[allow(clippy::too_many_arguments)]`）。
  - **file_cache**：`cache_key` 1186、`cached_file_id` 1196、`store_confirmed_files` 1210、`evict_rejected_files` 1238、`confirmed_file_ids` 1258、`invalid_file_id` 1272。
- 测试计数：**30 个（27 `#[test]` + 3 `#[tokio::test]`），`#[ignore]` = 0，`#[cfg]` 门控 = 0，无 env/子进程。**
- 测试 fixture/helper（8 个，全部在 `mod tests` 内）：`confirmed_server` 1552、`expected_request_length` 1584、`configured` 1602、`test_intent` 2102、`topic_intent` 2122、`tweet_row` 2144、`photo_plan` 2169、`outbox_entry` 2181。

- 不变量（P5 逐字保留）：
  - `activate_verified_credential_with_policy` 顺序：`BotToken::new(trim)` → `verify_bot_identity` → `prepare_telegram_credential_generation` → `store.set` → `activate_telegram_credential_with_resume`；激活失败时 `let _ = store.delete(secret_reference);` 只留 inactive orphan，绝不授予发送权（132–134）。
  - `auto_send_enabled` 四条件合取：`!migration_pending && enabled && auto_send_on_archive && !chat_id.trim().is_empty()`（165–168）。
  - `archive_plan_entries`：`plan_version` ∈ 1..=3；v3 且 metadata 文本 ≤ `TELEGRAM_CAPTION_LIMIT` 且首媒体无 caption 时合并为 caption；否则文本按 v1（单块）或 v2/v3（`split_for_telegram`）切块，键 `tweet-{id}:metadata` 或 `tweet-{id}:metadata:{index}`；媒体随后以 `tweet-{id}:media` 前缀追加（406–450）。
  - `resolve_archived_payload` 六重校验：payload schema 必须为 1 → 必须是 `SendPayload::Media` → `tweet_id` → `tweet_external_id` → `tweet_archive_facts` → 每个 item 必须匹配记录 media fact 且 `resolved.starts_with(archive_directory)` 且 `is_file()`，并依次比对 `fact.sha256`、`item.content_sha256`、`fact.size_bytes`；任一不符返回 `None`（481–532）。
  - `run_active_archived_sends_cooperative` 顺序：stop 信号 → `!enabled || migration_pending` 早退（返回 default summary，不触网不读 secret）→ `recover_expired_claims` → `reconcile_archived_send_intents` → 取 active credential → `secrets.get` → `BotToken::new` → **再读一次 active credential 比对 `(generation, identity, reference)`，不等则 `"credential generation changed during secret retrieval"`**（882–888）。
  - `upload_timeouts`：`connect = connect_timeout_seconds.max(1)`；`body_stall` 与 `server_processing` 均取 `upload_processing_timeout_seconds.max(1)`；`total = None`（905–913）。
  - `claim_token`：`{now_iso}-{bot_identity 前 8 字符}-{index}-{pid}`（916–923）。
  - `execute_payload`：`None` payload → `InvalidUploadRequest("the planned payload is no longer available")` + `NotSent`（不静默跳过、不盲重试）；text 走 `spawn_blocking` 并推入 `Queued`/`AwaitingResult`/`Confirmed`；单媒体走 `send_single_media`；多媒体走 `send_media_group_attempt`。
  - `send_single_media`：优先 `cached_file_id` + `cached_media_request` 走 JSON 控制法；**Document 无缓存控制法，永远重新上传**（1078–1079）。
- 测试内嵌 mock（P5 不得扩大可见性）：`stopped_active_sender_never_reads_credentials_or_connects`（2615）、`active_sender_rejects_rotation_during_secret_retrieval`（2668）、`active_sender_disabled_and_missing_credential_never_access_secrets_or_network`（2735）各自定义 `impl xarchive_telegram::SecretStore`，`get/set/delete` 一律 `panic!` 或返回固定值，以此证明“未启用/未就绪时绝不触碰凭据”；`candidate_activation_preserves_active_secret_on_stale_generation`（1612）内定义 `TelegramTransport::send` 桩。

- 测试名与行号（源码顺序）：1278 `concurrent_journal_materialization_has_one_plan_and_one_authority_set`；1358 `journal_restart_sends_once_and_api_failure_preserves_local_archive`；1493 `new_journal_rows_are_authorized_but_replay_never_grants_after_rotation`；1612 `candidate_activation_preserves_active_secret_on_stale_generation`；1740 `caption_layout_is_versioned_and_preserves_existing_media_text`；1766 `long_archive_text_is_split_into_stable_ordered_units`；1804 `persisted_archive_plan_rejects_corrupt_and_unknown_snapshots_without_progress`；1861 `persisted_archive_plan_is_atomic_and_replayable`；1928 `heartbeat_renews_a_pending_attempt_before_its_original_deadline`（tokio）；1971 `heartbeat_loss_drops_pending_attempt_without_polling_it_to_completion`（tokio）；1996 `heartbeat_storage_error_stops_the_attempt_and_is_not_retried`（tokio）；2036 `an_unusable_file_cache_stops_the_row_without_claiming_it_sent`；2219 `archived_snapshot_resolves_against_archive_facts_and_rejects_unsafe_paths`；2373 `legacy_null_snapshot_is_not_reconstructed_from_current_settings`；2408 `automatic_sending_requires_the_switch_and_a_target`；2422 `a_pending_endpoint_migration_suspends_automatic_sending`；2435 `queueing_an_archive_is_idempotent_and_ordered`；2483 `an_empty_plan_queues_nothing`；2507 `archive_metadata_mapping_orders_media_and_rejects_unsafe_references`；2615 `stopped_active_sender_never_reads_credentials_or_connects`；2668 `active_sender_rejects_rotation_during_secret_retrieval`；2735 `active_sender_disabled_and_missing_credential_never_access_secrets_or_network`；2801 `a_due_message_is_sent_and_confirmed`；2840 `a_claimed_row_reports_terminal_progress_stages_only`；2897 `text_payload_sends_to_its_persisted_topic`；2931 `an_unknown_entry_is_never_claimed_automatically`；2988 `a_claim_that_expired_before_the_request_retries_instead`；3017 `confirmed_cache_remains_scoped_to_original_bot_after_rotation`；3054 `a_cached_photo_is_sent_by_identifier_without_a_body`；3113 `a_media_plan_without_a_resolvable_payload_needs_a_new_plan`。

- P5 分组顺序（计划 §3 P5 要求“依次提 credentials、intent_planning、payload、delivery、runner”）：本文件的真实依赖方向是 credentials（无依赖）→ intent_planning（依赖 credentials 的错误串与 storage intent）→ payload（依赖 storage facts + FileStore）→ delivery/file_cache（依赖 payload + transport）→ runner（依赖以上全部）。**提取顺序必须与 §3 一致，不得先提 runner。**

## P2 Telegram 测试组织（已完成代码迁移，2026-11-09）

`crates/xarchive-telegram/src/lib.rs` 由 2691 行降为 217 行：前 215 行生产代码逐字保留，末尾仅留 `#[cfg(test)] mod tests;`。测试按**断言目标**（非行区间）分为五组，位于 `crates/xarchive-telegram/src/tests/`。

共享脚手架留在 `tests/mod.rs`，各组合集通过 `use super::*` 继承（与 `crates/xarchive-storage/src/tests/` 既有约定一致）：`fake_server`、`read_http_request`、`block_on`，以及全部原 `mod tests` 头部 `use`（含 `crate::transport::{json_error, request_payload}` 与 `crate::upload::{...}`）。

| 组文件 | 行数 | 测试数 | 覆盖职责 |
|---|---:|---:|---|
| `tests/contract.rs` | 528 | 18 | SecretStore 存取/脱敏、`validate_message`、`TelegramResponse` 结果提取、endpoint 策略（cloud/loopback/HTTPS/畸形）、代理路由与禁用、token 脱敏错误映射、`verify_bot_identity`、`BotToken::identity` |
| `tests/delivery_outbox_retry.rs` | 893 | 12 | `send_idempotently` 三态、`classify_send_failure` 分类表、`bounded_backoff`、`OutboxState` 语义、`decide_outbox_transition`、`run_claimed_attempt`/`run_claimed_attempt_with_clock` fence 与失败落库 |
| `tests/upload.rs` | 752 | 13 | `validate_upload_request`、`UploadTimeouts` 分层、`upload_method`、分层 watchdog、取消/缺文件/改大小短路、流式进度、响应上限、Bot API 错误、`send_media_group_attempt` 与相册边界 |
| `tests/formatting_planning.rs` | 192 | 3 | `plan_media_sends` 分组/尾单项提升/稳定幂等键、`PlannedSend::fingerprint` 与 `to_outbox_entry` 投影 |
| `tests/cache_projection.rs` | 48 | 1 | `is_invalid_file_id_error` 保守判定（仅失效 file id 触发回退） |
| `tests/mod.rs` | 92 | — | 模块声明、共享 `use`、共享 helper |

逐项对照（迁移后测试全名 → 迁移前行号 → 归属依据）：

`contract.rs`：`stores_and_deletes_secrets_without_exposing_values`(229)、`validates_message_boundaries`(246)、`sends_message_over_https_transport_contract`(294)、`sends_photo_video_and_media_group_methods`(382)、`maps_telegram_api_and_http_errors_without_exposing_token`(426)、`rejects_non_https_production_endpoints`(480)、`a_resolved_proxy_actually_receives_the_request`(489)、`a_disabled_proxy_still_builds_a_direct_client`(536)、`a_malformed_proxy_is_rejected_rather_than_silently_ignored`(550)、`parses_result_message_id_from_bot_api_response`(667)、`accepts_cloud_and_loopback_local_endpoints`(800)、`rejects_endpoints_that_could_redirect_or_leak_the_token`(830)、`secret_store_error_variants_do_not_carry_the_secret`(867)、`extracts_message_and_file_ids_from_success_results`(996)、`verified_bot_identity_is_stable_across_credentials_and_rejects_non_bots`(1019)、`api_endpoint_contract_gates_local_http_production_transport`(1078)、`migration_source_and_target_apply_endpoint_specific_proxy_policy`(1119)、`bot_identity_is_stable_reversible_free_and_token_sized`(1768)。

`delivery_outbox_retry.rs`：`a_control_request_with_a_lost_response_is_not_safe_to_retry`(358)、`idempotent_send_skips_transport_when_already_sent`(677)、`idempotent_send_records_delivered_state_and_stays_stable`(703)、`idempotent_send_persists_failure_and_retries_with_same_key`(739)、`classifies_send_failures_into_the_retry_policy`(1594)、`bounded_backoff_grows_then_caps`(1731)、`outbox_states_roundtrip_and_gate_automatic_sending`(1741)、`failed_attempts_map_onto_the_next_durable_transition`(1833)、`clock_aware_attempt_reads_time_before_and_after_io`(2054)、`attempt_driver_fences_before_sending_and_records_a_confirmed_result`(2080)、`attempt_driver_persists_the_retry_policy_of_each_failure_class`(2105)、`attempt_driver_drops_a_lost_claim_without_sending_or_writing`(2266)。随组迁移的 fixture：`MemorySendStateStore`(560)、`MemorySendRecord`(565)、`impl SendStateStore`(574)、`RecordingOutbox`(1902)、`RecordingOutboxState`(1907)、`impl RecordingOutbox`(1913)、`impl TelegramOutboxStore`(1932)、`schedule_label`(2050)。

`upload.rs`：`single_upload_contains_exactly_one_topic_field`(323)、`validates_upload_request_boundaries`(895)、`validates_upload_timeout_layers`(951)、`maps_media_kinds_to_upload_methods_and_fields`(986)、`watchdog_classifies_the_first_layered_deadline`(1348)、`upload_short_circuits_before_the_network_when_cancelled`(1398)、`upload_reports_missing_and_changed_files_before_any_network`(1421)、`send_upload_streams_the_file_and_reports_ordered_progress`(1456)、`send_upload_rejects_an_oversized_response_before_reading_it`(1533)、`send_upload_classifies_a_bot_api_error_result`(1561)、`send_upload_attempt_reports_confirmed_ids_and_honest_progress`(2481)、`an_album_streams_every_file_and_reuses_a_cached_file_id`(2543)、`an_album_outside_the_legal_range_is_refused_before_any_request`(2620)。随组迁移：`sample_upload_request`(882)、`temp_upload_file`(1163)、`ParsedMultipartFile`(1207)、`ParsedMultipartRequest`(1212)、`parse_multipart_request`(1217)、`upload_http_server`(1309)、`album_upload_request`(2530)。

`formatting_planning.rs`：`media_planner_splits_document_runs_without_reordering`(2294)、`media_plans_promote_a_trailing_single_and_keep_stable_keys`(2329)、`a_planned_send_fingerprints_its_content_and_becomes_an_outbox_entry`(2402)，随迁 `planned_item`(2317)。

`cache_projection.rs`：`invalid_file_id_detection_is_conservative`(1788)。

迁移约束落实：`lib.rs` 的 10 条 `mod` 声明与 23 条 `pub use`/公共项逐字未变；`json_error`/`request_payload`/`UploadTracker`/`upload_method`/`upload_watchdog`/`validate_upload_request` 均为 `pub(crate)`，无需放宽可见性。本 crate 无子进程测试、无 `--exact` 选择器、无 `#[ignore]`，故 P2 不涉及 self-exec 选择器改写。`formatting.rs`(5) 与 `projection.rs`(2) 的内联测试保持原处未动，总计仍为 54。

## P3 Storage 测试组织（已完成代码迁移，2026-11-09）

两个文件原来既是生产测试集又是父 helper 宿主。迁移后各自成为同名目录，`mod.rs` 保留共享 fixture 与自执行崩溃测试。

### `tests/archive_recovery/`（原 2748 行，22 测试）

`mod.rs` 保留 6 个文件内 helper（`test_candidate_metadata`、`manifest`、`replay_crashed_v1_window`、`repeat_convergence_is_idempotent`、`assert_attempt_fenced_terminal_convergence`、`run_v1_crash_window_child`）与崩溃自执行测试 `v1_publish_crash_windows_converge_through_production_prepared_replay`。

| 子模块 | 测试数 | 覆盖 |
|---|---:|---|
| `claims.rs` | 8 | `begin_archive_attempt` 终态/溢出/PREPARED 拦截、`claim_archive_job` 原子性、`claim_archive_attempt` 推进一次、InternalV1 successor CAS、journaled/非 V1 拒绝、overflow+PREPARED 拒绝 |
| `dispatch.rs` | 2 | `startup_recovery_dispatch` 路由 InternalV1 并在 Downloaded 后 fail-closed；PREPARED replay 在 SQL fault 下原子且终态收敛幂等 |
| `v1_journal.rs` | 2 | V2 journal 幂等/严格性/phase 单调；InternalV1 PREPARED journal 在取消仲裁中胜出 |
| `cancellation.rs` | 1 | 取消相对 archive commit 边界的仲裁 |
| `v2_finalization.rs` | 6 | V2 finalization 原子提交 tweet/media/job/intent/journal；stale attempt 拒绝并回滚；legacy linked-intent 拒绝且不改动；manifest 写入间崩溃可重放；post-rename DB 冲突整体回滚；legacy internal 恢复不改 attempt |
| `v2_committed.rs` | 2 | COMMITTED 只读复核；身份不匹配 fail-closed |

### `tests/archive_service/`（原 2309 行，48 测试）

`mod.rs` 保留崩溃自执行测试 `v2_forward_publish_crash_windows_converge_through_recovery_replay`；全部共享 fixture（`temp_root`、`unprepared_v2_publish_inputs`、`prepared_v2_rename_fixture`、`outbox_entry`、`BOT_A`、`OUTBOX_NOW`）仍由父 `tests/mod.rs` 提供。

| 子模块 | 测试数 | 覆盖 |
|---|---:|---|
| `local.rs` | 4 | 本地归档状态迁移/事件、文件保护与 staging 提交、ISO 时间戳 job id 可解析、portable metadata |
| `sidecar_paths.rs` | 7 | staging 从持久化 metadata 恢复；Sidecar 文件实际大小/哈希换算；路径逃逸、中间符号链接逃逸（`#[cfg(unix)]` 随函数迁移）；数字 tweet_id；author/relationship 字段；直接由 Sidecar 结果完成归档 |
| `preview.rs` | 4 | attempt preview 使用 attempt payload 而非 legacy staging；metadata 绑定 transfer 相对路径与哈希；不回退 legacy staging；拒绝上报路径逃逸 |
| `telegram_intent.rs` | 5 | intent journal 在 DB 重开后恢复 staging/已改名归档；InternalV1 Telegram intent 无 runner permit 时拒绝；intent journal 拒绝改写事实与非法迁移；outbox 批量冲突回滚并可幂等重放 |
| `v2_candidate.rs` | 3 | V2 candidate 保留 legacy 业务载荷并拒绝扩展；不安全 URL 在写入前拒绝；PREPARED candidate 失败时保留 journal 与 staging |
| `v2_rename_recovery.rs` | 12 | PLANNED→COMMITTED 磁盘重放、重复重放幂等、同名文件保持、rename plan 不匹配拒绝、temporary phase 崩溃收敛、文件已移动但 journal 滞后、目标不匹配、stale attempt、journal 缺失、committed 目录存在但 journal 滞后、committed 目录媒体被换、下载事件失败回滚并只重放一次 |
| `v2_forward_publish.rs` | 12 | export 冲突前置拒绝、历史 export 开关不影响内部恢复、prepare/rename/commit/fenced finalize 组合、可选 export 无用户文件发布恢复、COMMITTED 重放拒绝换字节/缺失事实/改动事实、目标冲突、stale attempt、PREPARED 胜过取消、回滚 candidate 拒绝、重复完成只读重放 |

### P1 台账两处存疑的实测结论

1. **`journal_snapshot` 并非缺失 helper。** 全仓库唯一出现是 `archive_service.rs:988` 内的局部闭包（`let journal_snapshot = |database: &Database| -> (String, String, String)|`），随所在测试一起迁移，父模块从未定义它也无需定义。
2. **`test_candidate_metadata` / `manifest` 的双份定义已字节级比对为完全相同**（`p3-shadow-compare.txt`：`identical: True` / `True`）。因此它们属于**良性重复**而非冲突：`archive_recovery.rs` 内的私有副本遮蔽父模块同名项，但两者行为一致。迁移后副本移入 `archive_recovery/mod.rs`，遮蔽关系不变，其余组与 `archive_service` 组仍通过 `use super::*` 取用父模块版本。

### 选择器零漂移设计

两个自执行崩溃测试都**留在各自 `mod.rs`**，因此模块路径仍是 `tests::archive_recovery::<name>` / `tests::archive_service::<name>`，`current_exe()` 子进程里的硬编码 `--exact` 字符串**完全未改动**。这是刻意的：把子模块化与选择器改写解耦，避免一次变更同时承担两种风险。实测两条 `--exact` 各恰选 1 项（177 filtered out），且 `crash.ready` marker 与 `std::process::abort()` 行为未变。

## P4 Executor 测试组织（已完成代码迁移，2026-11-09）

`desktop/src-tauri/src/executor.rs` 原 9815 行：生产代码 1–898（含 `ExecutorError`），随后 `#[cfg(test)] mod tests` 899–7489（124 顶层项 = 101 测试 + 23 夹具），`#[cfg(all(test, unix))] mod session_boundary_tests` 7490–9815（38 顶层项 = 24 测试）。分两步执行，每步独立复验。

### P4a 整体外置两个模块

| 变更 | 结果 |
|---|---|
| `executor.rs` | 9815 → 907 行；尾部仅留 `#[cfg(test)] mod tests;` 与 `#[cfg(all(test, unix))] mod session_boundary_tests;` |
| `executor/tests/mod.rs` | 新文件，P4a 后 6527 行（`mod tests` 体内 124 项整体降一级缩进）；P4b 分组后收缩为 824 行 |
| `executor/session_boundary_tests.rs` | 新文件，2279 行（`mod session_boundary_tests` 体内 38 项整体降一级缩进），P4b 未改动它 |

（`wc -l` 实测。早前表内记的 6593 / 2324 与快照不符，以本节为准。）

模块名刻意不改，因此模块路径仍是 `executor::tests::*` 与 `executor::session_boundary_tests::*`。P4a 这一步**不改变任何测试的完整路径**：实测 378 个结果的 Cargo 全路径名集合前后完全相同（`MISSING: []`、`NEW: []`）。这一点在 P4b 之后只对未分组的项继续成立，见下文“完整路径映射”。

`mod.rs` 的 `use super::*;` 由原来的 `use super::*` 语义继承：外置后它绑定到 `executor` 模块，因此生产符号可见性不变；两个文件顶部各自新增 `//!` 模块文档说明迁出来源与选择器约束。

### P4b `mod tests` 按责任分组

`mod.rs` 保留 **25 项**：23 个共享夹具（`request`、`archive_request`、`v3_spec`、`insert_downloaded_job`、`complete_job_for_fixture`、`v2_request`、`insert_v2_downloaded_job`、`temporary_database_path`、`wait_for_persisted_state`、`FakeJobExecution` 与其 `impl`、`CancellationAwareExecution`、`DropNotifiesExecution` 与其 `impl`/`Drop`、`BlockingUntilReleased` 与其 `impl`、`ReleaseOnDrop`、`N1AdmissionFactory` 与其 `impl`、`CommitBoundaryPersistence` 与其 `impl JobPersistence`）＋ **2 个自启动 `--exact` 选择器测试**。其余 99 个测试分入 6 个责任模块：

| 子模块 | 测试数 | 覆盖 |
|---|---:|---|
| `submission_dedup.rs` | 7 | 重复 tweet 复用活跃作业、并发提交单 worker、应用服务 facade 委派、同步作业标识与复用契约、非法浏览器请求前置拒绝、输出设置快照持久化、Storage 支撑作业走 InternalV1 恢复 |
| `state_transitions.rs` | 7 | 取消幂等与终态不变、运行中取消围栏迟到成功、状态机边界、`execute` 回复等待执行资源释放、恢复跳过终态与重复并重启中断、事件顺序、Sidecar 崩溃映射为安全失败 |
| `persistence_spec.rs` | 48 | v1/v2/v3/v4 执行规格解码与拒绝（未知版本、未知字段、`policy_version` 不匹配、argv 字节往返、诊断不泄露值/版本）、持久化提交/调度/恢复、stale attempt 围栏、`commit_recovery_decision` 四分支、`apply_commit_recovery` 四态、Storage 适配器契约与投影 |
| `scheduling_cancel_shutdown.rs` | 11 | runner 饱和拒绝非失败可重调度、调度绑定 attempt、持久化取消四态、maintenance 与活会话互斥、取消与 commit 竞态（拒绝/胜出）、shutdown 一次中断并跳过终态、`interrupt_persisted` 与 worker shutdown 分离 |
| `startup_cas_crash.rs` | 14 | Sidecar InternalV1 hello/ready/shutdown 与 extract-then-cancel、启动重派绑定 attempt、缺 manifest fail-closed、无 journal InternalV1 接管、SQLite successor CAS 丢失、崩溃 writer 域 drain、未知 contract 版本、v2 缺 journal、legacy 分类两种、候选枚举失败无副作用、有效 InternalV1 完成并幂等重复 |
| `containment_publication.rs` | 12 | 生产工厂 containment 握手与 writer group 移除、真实 stuck cleanup owner、真实工具本地发布、真实 aria2 drain 与 transfer/refresh、仅测试 v2 调度夹具发布、v2 输出快照带入归档执行、未受控 writer 前置拒绝、post-commit DB 失败恢复、writer drain 后 finalize、注册并 drain 真实 aria2 后发布、Linux 启动描述符有界 fail-closed |

分组只按**责任**归属，组内保持原文件相对顺序，因此审阅时的 diff 是可对照的“整段搬运”。99 个测试全部迁移。

### 保真度复验（2026-11-09 补证，取代早前表述）

P4 最初写下的“`TOKEN DIFFS: 0`，即除空白外逐字节一致”是一处**过高表述**，本轮用 `.tw2tg-tmp/agent-scratch/p4-closeout.py`（输出 `p4-closeout.txt`）重新核对并更正。旧比较器有两个缺陷：它把函数名当唯一键（162 个解析项因此塌缩成 161 个键，两个不同模块里的 `request` 合并）；它的归一化同时丢弃空白**和逗号**，所以结果既不是字节一致也不是严格 token 一致。

补证以 `(module, kind, name, 序号)` 作为身份，对 `mod tests` 与 `mod session_boundary_tests` 体内全部 **162 个顶层项**逐一配对：

| 指标 | 结果 |
|---|---|
| 迁移前顶层项 | 162（`tests` 124 ＋ `session_boundary_tests` 38） |
| 迁移后 8 个文件的顶层项合计 | 162 |
| 已配对 | 162；未配对 0；遗留 0；重复身份 0 |
| 降一级缩进后**字节完全一致** | 130 |
| 有差异 | 32 |
| └ 仅空白重排（去掉全部空白后相同） | 9 |
| └ 仅多一个尾随逗号（再去掉逗号后相同） | 22 |
| └ 其他非空白差异 | **1** |

唯一一处非空白差异是 `session_boundary_tests.rs` 中的 `impl JobExecutionFactory for ProbeFactory`：原本用两行的 `map_err(|error| { ExecutorError::Execution { ... } })` 块，外置降缩进后 rustfmt 折叠了外层块，成为 `map_err(|error| ExecutorError::Execution { ... })`。语义等价（同一 `ExecutorError::Execution` 值、同一 `?` 传播），但它确实不是逐字节保真，必须显式记录而非归入“格式噪声”。去除空白与逗号的 22 项对应 rustfmt 的尾随逗号规则；9 项为纯换行重排。

### 完整路径映射

P4b 的 99 个测试**确实更换了完整测试路径**，因为分组模块被插入了路径中间；只有留在 `tests/mod.rs` 的 25 项保持原路径。用 `p4a-after.log` 与 `p4b-after.log` 的 Cargo 输出求差集：消失 99 条、新增 99 条，按短名一一配对后 **99 对全部解析成功、未解析 0、状态改变 0**。因此下表隐含的结论是：**叶测试名与结果状态不变，完整路径按分组显式改变**。

修复后的实测数据：

| 项目 | 基线 | P4a 后 | P4b 后 |
|---|---|---|---|
| 结果条数 | 378 | 378 | 378 |
| passed / failed / ignored | 365 / 0 / 13 | 365 / 0 / 13 | 365 / 0 / 13 |
| 完整路径未变的项 | — | 378 | 279 |
| 完整路径改变的项 | — | 0 | 99 |

`p4-closeout.txt` 的 `=== moved test mapping ===` 段逐条列出 99 组「旧路径 → 新路径 ＋ 前后状态」，这是 P4b 的路径漂移台账，不应再被描述为“路径全部未变”。

### 选择器零漂移设计（延续 P3）

两处 `current_exe()` + `--exact` 自启动测试（`startup_recovery_retries_successor_after_cas_owner_is_sigkilled`、`startup_recovery_drains_live_descendant_after_owner_sigkill`）都**留在 `tests/mod.rs`**，所以硬编码字符串 `executor::tests::<name>` 完全未改动。脚本里为此加了强制断言 `SELF_SELECTING <= kept_names`，防止后续再分组时把它们误搬出去而静默失效。实测两条 `--exact` 各恰选 1 项（377 filtered out），其中一条是既有 `#[ignore]`（需要显式委派的 disposable cgroup），因此结果为 1 passed / 1 ignored，与基线一致。

### 属性与可见性

`mod tests` 的 `#[cfg(test)]` 与 `session_boundary_tests` 的 `#[cfg(all(test, unix))]` 均原样保留在 `executor.rs` 的模块声明上。子模块通过 `use super::*;` 取用父级夹具——Rust 允许后代模块访问祖先的私有项，因此**未新增或修改任何一个 `pub` 关键字**，也没有 `pub(crate)` 扩散。

### ignored 与既有警告

13 个 `#[ignore]` 全部随所属测试迁移，忽略理由字符串逐字未改。**分布需要更正**：早前写“`executor/tests/` 11 个、`session_boundary_tests.rs` 2 个”是错的。按 `p4b-after.log` 的 `ignored` 结果逐条统计（`p4-closeout.txt` 的 `=== ignored distribution ===` 段）：

| 归属 | ignored |
|---|---:|
| `executor::tests::containment_publication` | 8 |
| `executor::session_boundary_tests` | 2 |
| `executor::tests::mod`（自启动下落后代选择器） | 1 |
| `archive::completion_tests` | 2 |
| **合计** | **13** |

因此 Executor 侧是 **9 + 2**（`tests/` 内 8 个分组项 ＋ 1 个留在 `mod.rs` 的自启动选择器，另加 `session_boundary_tests.rs` 2 个），再加 `archive.rs` 的 2 个才等于 13。P1 定位的“`executor.rs` 11 ＋ `archive.rs` 2”在数量上仍成立，但 P4b 之后 `executor.rs` 这 11 个已分属两个文件与两个模块层级，不能再写成一个文件的 11 个。

编译零警告，未“顺手修正”任何既有 `unused_mut` 等告警。

## P5 Telegram 发送生产能力拆分（已完成，2026-11-09）

`desktop/src-tauri/src/telegram_send.rs` 原 3153 行（生产 1–1274，`#[cfg(test)] mod tests` 1275–3153，**30 个测试、0 ignored**）。Plan §3 把 P5 定义为**生产职责提取**，不是整理测试；本轮按 credentials → intent_planning → payload → delivery → runner 顺序执行五个微批。

### 五个能力模块

| 顺序 | 模块 | 行数 | 职责与真实符号 |
|---|---|---:|---|
| P5.1 | `telegram_send/credentials.rs` | 123 | `apply_credential_resume_policy`、`activate_verified_credential`、`activate_verified_credential_with_policy` |
| P5.2 | `telegram_send/intent_planning.rs` | 323 | `ArchiveSendIntent` 与其 impl、`safe_relative_path`、`queue_archive_sends`、`reconcile_archived_send_intents`、`queue_persisted_archive_sends`、`archive_plan_entries` |
| P5.3 | `telegram_send/payload.rs` | 96 | `restore_payload_from_entry`、`resolve_archived_payload`、`recover_expired_claims` |
| P5.4 | `telegram_send/delivery.rs` | 395 | `upload_timeouts`、`execute_payload`、`send_single_media`、`album_upload`、`cached_media_request`、`confirm_cached_send`、`control_progress`、`cache_key`、`cached_file_id`、`store_confirmed_files`、`evict_rejected_files`、`confirmed_file_ids`、`invalid_file_id` |
| P5.5 | `telegram_send/runner.rs` | 422 | `MAX_SENDS_PER_RUN`、`CLAIM_LEASE`、`SendRunSummary`、`run_due_sends`、`run_due_sends_with_clock`、`run_due_sends_with_generation`、`with_claim_heartbeat`、`with_claim_heartbeat_interval`、`entry_payload`、`run_due_archived_sends`、`run_active_archived_sends`、`run_active_archived_sends_cooperative`、`claim_token` |

`telegram_send.rs` 从 3153 行降到 **1980 行**，尾部是**未改动的** `mod tests`（30 个测试仍在原处，测试路径仍是 `telegram_send::tests::<name>`）。`auto_send_enabled` 留原文件：它是 archive 路径直接调用的纯配置谓词，不属于 claim 循环。

### 保真度（`p5-fidelity.txt`）

分类口径按 Plan §2 分开报告，互不替代：

| 分类 | 数量 |
|---|---:|
| 字节完全一致 | 23 |
| 仅格式（rustfmt 重排） | 0 |
| 仅可见性收窄 | 0 |
| 语义差异 | **0** |

外加 16 项因解析器识别 `pub(in ...)` / `const` / `struct` 前缀不足而未能自动配对，已用直接花括号配对复核：其中 15 项函数体去空白后**完全相同**；`cache_key` 仅新增 `pub(in crate::telegram_send)` 前缀并因签名变长被 rustfmt 折行，**函数体逐行相同**（`p5-fidelity.txt` 之外的直接比对，仍在本次会话终端中复核）。`ArchiveSendIntent`、`impl ArchiveSendIntent` 与其 `const`/`struct` 项亦经同一方式确认。

### 测试身份（`p5-baseline-desktop.log` → `p5-after.log`）

| 项目 | 基线 | P5 后 |
|---|---|---|
| Desktop 结果条数 | 378 | 378 |
| passed / failed / ignored | 365 / 0 / 13 | 365 / 0 / 13 |
| `telegram_send::tests::*` | 30 | 30 |
| 移除 / 新增测试路径 | — | 0 / 0 |
| 状态改变 | — | 0 |

**没有测试改名、丢失或转为 ignored**，也没有零选择伪通过：每条微批的定向命令都是 `telegram_send::tests` 的 30/30（348 filtered out）。

### 可见性与兼容路径

`mod telegram_send;` 在 `lib.rs` 中本就是 crate 私有，因此新增五个子模块**未扩大任何外部可见性**。原公共路径全部保留，通过父模块重导出：

- `pub use`：`apply_credential_resume_policy`、两个 `activate_verified_credential*`、`ArchiveSendIntent`、`ArchivedIntentQueueResult`、`queue_archive_sends`、`queue_persisted_archive_sends`、`reconcile_archived_send_intents`、`restore_payload_from_entry`、`resolve_archived_payload`、`recover_expired_claims`、`SendRunSummary`、`run_due_sends`、`run_due_sends_with_clock`、`run_active_archived_sends`；
- `pub(crate) use`：`run_active_archived_sends_cooperative`（`telegram_worker.rs` 以该作用域调用，不能放宽）；
- `pub(in crate::telegram_send) use`：`archive_plan_entries`、`upload_timeouts`、`invalid_file_id`、`cache_key`、`cached_file_id`、`execute_payload`、`store_confirmed_files`、`evict_rejected_files`、`claim_token`、`entry_payload`、`with_claim_heartbeat`、`with_claim_heartbeat_interval`、`run_due_sends_with_clock` 的子树别名。

**这些桥接全部写在 `pub(in crate::telegram_send)`**，即只在本模块子树内可见，**没有一处 `pub(crate)` 扩散，也没有一处改为 `pub`**。仅 `run_active_archived_sends_cooperative` 保持它原本就有的 `pub(crate)`。这是为满足“父子模块不可见兄弟私有项”而做的最小桥接，已逐项记录，不能解读为可见性放宽。

### 编译与告警

`cargo check -p xarchive-desktop --lib --tests` 的告警回到基线两条：`executor.rs` 的 `InMemoryCommitRecoveryFactsProvider` 未使用导入与 `runtime.rs` 的 `record_recovery_results` 未使用。**未新增任何告警**，也未“顺手修正”既有告警。逐批新增的 `unused_imports` 全部通过收窄导入与测试模块内导入消除，不使用宽泛 `allow` 掩盖。

`cargo fmt --all -- --check` 退出 0（本轮先 `cargo fmt --all` 应用 rustfmt，再复查）。

### P5 最终日志

| 检查 | 命令 | 退出码 | 结果 |
|---|---|---:|---|
| Desktop 全模块 | `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored** |
| Storage 回归 | `cargo test -p xarchive-storage --lib -- --test-threads=1` | 0 | **178 passed / 0 failed / 0 ignored** |
| 格式 | `cargo fmt --all -- --check` | 0 | 干净 |
| diff 检查 | `git --no-pager diff --check` | 0 | 干净 |
| 文档审计 | `node scripts/docs-audit.mjs` | 0 | PASS（153 tracked Markdown） |
| 未跟踪文档补充检查 | `check-untracked-docs.mjs` | 0 | PASS |
| 保护备份 | `sha256sum` | — | 与 P4 记录相同，未变 |

日志：`.tw2tg-tmp/agent-scratch/build-logs/p5-baseline-desktop.log`、`p5-after.log`、`p5-final-desktop.log`、`p5-final-storage.log`。本轮**没有**运行 workspace Full、架构门禁重封存或任何 Windows 检查，也没有 staging/commit。

## P6 Batch 生产能力拆分（已完成，2026-11-09）

`desktop/src-tauri/src/batch.rs` 原 1968 行（生产 1–610，`#[cfg(test)] mod d1_batch_tests` 611–660，`#[cfg(test)] mod tests` 1057–1968；**13 个测试、0 ignored**）。按 Plan §3 顺序执行四个微批：discovery → selection → submission → dispatch。

| 顺序 | 模块 | 行数 | 职责与真实符号 |
|---|---|---:|---|
| P6.1 | `batch/discovery.rs` | 257 | `SKIP_*` 六个跳过原因常量、`DiscoverySummary`、`discovery_request`、`candidate_record`、`persist_discovered_candidates`、`run_account_discovery`、`discovery_error_code` |
| P6.2 | `batch/selection.rs` | 309 | `SELECTION_SCAN_LIMIT`、`BatchFilters`（含 `to_json`/`from_json`/`exclusion`）、`select_dispatch_candidates`、`apply_batch_limit`、`newest_first`、`mark_skipped`、`candidate_already_archived`、`archive_facts_present`、`safe_media_path`、`candidate_archive_request` |
| P6.3 | `batch/submission.rs` | 65 | `inherited_execution_spec`、`MetadataBatchSnapshot`、`metadata_batch_settings` |
| P6.4 | `batch/dispatch.rs` | 429 | `BatchDispatchPass`、`BatchCancellationGuard` 与其 `Drop`、`dispatch_batch_pass`、`reconcile_batch_candidates`、`spawn_batch_dispatch`、`spawn_account_batch` |

`batch.rs` 从 1968 行降到 **1048 行**，仅保留 `truncate_message`、`storage_error`（discovery 与 dispatch 共用）以及**未改动的两个测试模块**。因此 13 个测试的路径仍是 `batch::d1_batch_tests::*` 与 `batch::tests::*`。

### 保真度与测试身份

`p6-fidelity.txt` 与直接核对结果：

| 分类 | 数量 |
|---|---:|
| 字节完全一致（去空白后亦一致） | 18 |
| 仅 rustfmt 重排（可见性前缀导致签名折行） | 1（`reconcile_batch_candidates`） |
| 语义差异 | **0** |
| 常量逐条比对一致 | 7/7 |
| 结构体/函数逐项复核（`DiscoverySummary`、`inherited_execution_spec`、`metadata_batch_settings`） | 3/3 一致 |

测试身份（`p6-baseline-desktop.log` → `p6-after.log`）：Desktop **378 条结果、365/0/13**，移除 0、新增 0、状态改变 0，`batch::*` **13 → 13**，每一微批的定向运行均为 13/13（365 filtered out）。

### 可见性与兼容路径

`mod batch;` 在 `lib.rs` 中本就是 crate 私有。原路径全部保留：`BatchFilters` 与 `spawn_batch_dispatch` 仍经 `crate::batch::*` 解析（这是外部仅有的两处调用）。桥接作用域分三档且均未放宽：

- `pub(crate) use`：`BatchFilters`、`candidate_archive_request`、`select_dispatch_candidates`、`DiscoverySummary`、六个 `SKIP_*`、`discovery_request`、`candidate_record`、`persist_discovered_candidates`、`run_account_discovery`、`discovery_error_code`、`dispatch_batch_pass`、`spawn_account_batch`、`spawn_batch_dispatch`；
- `pub(in crate::batch) use`：`inherited_execution_spec`、`metadata_batch_settings`、`BatchDispatchPass`、`BatchCancellationGuard`、`reconcile_batch_candidates`；
- 定义侧的 `pub(in crate::batch)` 仅用于上述五项、`discovery_error_code` 与 `BatchCancellationGuard` 的三个字段——这些都在迁移后需要被父模块（测试）与同级 `dispatch` 访问，作用域始终限于 `batch` 子树。**没有一处提升为 `pub` 或新增 `pub(crate)` 以外的暴露。**

### 编译与告警

`cargo check -p xarchive-desktop --lib --tests` 回到基线两条告警（`executor.rs` 未使用导入、`runtime.rs` 未使用方法），**零新增**。父模块中只被测试使用的导入改为 `#[cfg(test)]` 限定，测试专用类型（`BatchCandidateRecord`、`DiscoveryOutcome`、`DiscoveryRequest`）移入测试模块，未使用宽泛 `allow` 掩盖。`cargo fmt --all -- --check` 退出 0。

### P6 最终日志

| 检查 | 退出码 | 结果 |
|---|---:|---|
| `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored** |
| `cargo fmt --all -- --check` | 0 | 干净 |
| `git --no-pager diff --check` | 0 | 干净 |
| `node scripts/docs-audit.mjs` | 0 | PASS |
| 未跟踪文档补充检查 | 0 | PASS |

本轮未运行 workspace Full、未动架构门禁/基线、未 staging/commit、未做 Windows 检查。

## P7a Records 命令职责拆分（已完成，2026-11-09）

`desktop/src-tauri/src/commands/records.rs` 原 1097 行（含 7 个 `#[tauri::command]` 入口与 `record_import_acceptance_tests` 的 8 个测试）。按 Plan §3 顺序执行三个微批。

| 顺序 | 模块 | 行数 | 职责与真实符号 |
|---|---|---:|---|
| P7a.1 | `commands/records/export.rs` | 131 | `serialize_batch_record_export`、`serialize_record_export`(cfg(test))、`serialize_record_export_with_account` |
| P7a.2 | `commands/records/import_document.rs` | 137 | `RecordImportOutcomeResponse`/`RecordImportPreviewEntry`/`RecordImportPreviewResponse`、`RecordImportDocument` 与其 `impl`（`parse`/`records`/`evidence`）、`record_import_root`、`record_import_revision` |
| P7a.3 | `commands/records/import_evidence.rs` | 69 | `record_import_media_probe`、`verify_accepted_media_evidence` |

`records.rs` 从 1097 行降到 **832 行**。

### 命令入口与注册未动

7 个 `#[tauri::command]` 全部仍在 `records.rs` 原位置：`export_configuration_exchange`、`preview_configuration_exchange`、`apply_configuration_exchange`、`export_record_exchange`、`preview_record_import`、`cancel_record_import`、`accept_record_import`。`lib.rs` 中 7 处 `commands::records::<name>` 注册路径**逐一未改**，`generate_handler!` 仍解析到同一批符号。

`RecordImportWait` 与 `RECORD_IMPORT_CANCELLATION` 仍同在 `records.rs`，取消注册与 `accept_record_import` 的 `_wait` 生命周期**同一归属未被拆开**。这是 Plan §3 对 P7a 的明确约束。

### 保真度与测试身份

逐项配对结果（去空白、去可见性前缀后比较）：

| 项目 | 结果 |
|---|---|
| 12 个搬家项中逐项一致 | **11** |
| `impl RecordImportDocument` 的三个方法（`parse`/`records`/`evidence`） | **3/3 函数体一致**，差异仅为方法上新增 `pub(super)` |
| 语义差异 | **0** |

`impl` 项本身无法在"去可见性前缀"后直接比较，因此改为按方法逐一核对，结论一致；这与把差异笼统记为"格式噪声"不同。

测试身份（`p6-baseline-desktop.log` → `p7a-after.log`）：Desktop **378 条结果、365/0/13**，移除 0、新增 0、状态改变 0；`records` 相关测试 **17 → 17**，其中 `record_import_acceptance_tests` 8/8 每批定向运行通过（370 filtered out）。

### 可见性与兼容路径

`commands::records` 本就是 crate 内模块。父模块用 `use` 桥接，未放宽任何项：

- `pub(crate) use import_document::{RecordImportOutcomeResponse, RecordImportPreviewEntry, RecordImportPreviewResponse}`：三者原本就是 `pub(crate)`，命令返回值形状不变；
- 模块内 `use`：`serialize_batch_record_export`、`serialize_record_export_with_account`、`RecordImportDocument`、`record_import_root`、`record_import_revision`、`record_import_media_probe`、`verify_accepted_media_evidence`；
- `#[cfg(test)] use export::serialize_record_export`：该函数原本就带 `#[cfg(test)]`，保持只读测试用途。

子模块内部对跨模块项的访问使用 `pub(super)`，作用域限于 `commands::records` 子树。**没有一处改为 `pub`，也没有新增 `pub(crate)` 以外的暴露。**

### 编译与告警

`cargo check -p xarchive-desktop --lib --tests` 回到基线两条告警，**零新增**。`std::fs` 因配置预览的生产代码需要使用而保留普通导入；`Sha256`/`Digest` 只被测试使用，改为 `#[cfg(test)]` 限定。

### P7a 最终日志

| 检查 | 退出码 | 结果 |
|---|---:|---|
| `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored** |
| `cargo fmt --all -- --check` | 0 | 干净 |
| `git --no-pager diff --check` | 0 | 干净 |
| 7 个 `#[tauri::command]` 仍在 `records.rs` | — | 7/7 |
| `lib.rs` 的 `commands::records::` 注册 | — | 7/7 |

本轮未运行 workspace Full、未动架构门禁/基线、未 staging/commit、未做 Windows 检查。

## P7b Unix transport 拆分（已完成，2026-11-09）

`desktop/src-tauri/src/transport.rs` 原 1007 行。按 Plan §3，把 Unix server、连接处理与 endpoint 辅助移至 `transport/unix.rs`，同时把 `BrowserTransportAdapter` 与协议错误映射留在原入口。

| 迁出 | 保留在原入口 |
|---|---|
| `DesktopTransportServer` 结构体、其 `impl`（含 `MAX_ACTIVE_CONNECTIONS`/`CONNECTION_TIMEOUT`/`start`/`start_with_pairing`）、`impl Drop` | `BrowserTransportAdapter` 与其全部 `impl`（`handle_request`、`with_*` 构造器） |
| `handle_unix_connection` | `error_response`、`error_error_code`（协议错误映射） |
| `transport_endpoint` | `now_iso` |
| — | `#[cfg(unix)]` 的 `Path`/`Arc`/`AtomicBool`/`StorageJobPersistence` 导入随之收敛 |

`transport.rs` 从 1007 行降到 **774 行**，`transport/unix.rs` 为 **271 行**（含模块文档）。

### 保真度与测试身份

五个迁移项在去空白、去可见性前缀后**逐项完全一致**（`DesktopTransportServer` 结构体、其 `impl`、`Drop`、`handle_unix_connection`、`transport_endpoint`），**语义差异 0**。保留项 `BrowserTransportAdapter`、`error_response`、`error_error_code`、`now_iso` 仍在父模块原位。

测试身份（`p6-baseline-desktop.log` → `p7b-after.log`）：Desktop **378 条结果、365/0/13**，移除 0、新增 0、状态改变 0，ignored 13 → 13，`transport::*` **9 → 9**；`transport::` 定向运行 22/22 通过（356 filtered out）。

### 路径与可见性

`crate::transport::{DesktopTransportServer, transport_endpoint}` 有三处外部调用（`runtime.rs` 的构造与 endpoint 派生、`websocket_transport.rs` 的测试 IPC server），因此父模块用 `#[cfg(unix)] pub(crate) use unix::{DesktopTransportServer, transport_endpoint}` 保留**原路径与原有 `pub(crate)` 作用域**。子模块内 `handle_unix_connection` 仍为私有，经 `super` 访问 `BrowserTransportAdapter`。**没有一处提升为 `pub`。**

`#[cfg(unix)]` 边界完整保留：`mod unix;` 与重导出都带该 cfg，非 Unix 目标不编译这些符号。Windows Named Pipe 后端（`windows_transport.rs`）**未改动**，`runtime.rs` 中 `#[cfg(windows)]` 分支仍指向 `crate::windows_transport::*`。

### 告警

`cargo check -p xarchive-desktop --lib --tests` 回到基线两条告警，**零新增**。父模块中随迁移失效的 `Path`/`Arc`/`AtomicBool`/`Ordering` 导入已删除；`StorageJobPersistence` 仅测试使用，改为 `#[cfg(test)]`。

### P7b 最终日志

| 检查 | 退出码 | 结果 |
|---|---:|---|
| `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored** |
| `cargo test -p xarchive-desktop --lib transport:: -- --test-threads=1` | 0 | 22 passed / 0 failed |
| `cargo fmt --all -- --check` | 0 | 干净 |
| `git --no-pager diff --check` | 0 | 干净 |

本轮未运行 workspace Full、未动架构门禁/基线、未 staging/commit、未做 Windows 检查；Unix socket 测试结果不外推为 Windows Named Pipe/ACL 验收。

## P8 Production 编排职责拆分（已完成，2026-11-09）

`desktop/src-tauri/src/production.rs` 原 1595 行。按 Plan §3 顺序执行五个微批。

| 顺序 | 模块 | 行数 | 职责与真实符号 |
|---|---|---:|---|
| P8.1 | `production/tool_compatibility.rs` | 88 | `revalidate_tool_compatibility`、`resolved_tool_version`、`executable_version` |
| P8.2 | `production/discovery.rs` | 143 | `DiscoveryRequest`、`DiscoveryOutcome`、`execute_v2_discovery` |
| P8.3 | `production/sidecar_operations.rs` | 295 | `extract_v2`、`download_v2`、`verify_downloaded_files` |
| P8.4 | `production/transfer.rs` | 549 | `execute_aria2_archive`、两个 `transfer_once`（linux / 非 linux）、`run_transfer_with_supervisor`、`transfer_with_one_refresh`、`ensure_same_media_collection`、`TransferAttemptFailure` 与其 `impl`、`transfer_failure_enters_refresh`、`transfer_failure_allows_refresh`、`retry_retained_drain_with`、`retry_retained_drain`、`aria2_config`、`available_loopback_port`、`transfer_files`、`extraction_to_metadata`、`format_transfer_failure` |
| P8.5 | `production/result_mapping.rs` | 53 | `execute_gallery_dl_archive`、`media_type_name` |

`production.rs` 从 1595 行降到 **614 行**，仅保留组合入口 `execute_v2_archive`、模块声明与兼容重导出，以及未改动的 `e3_argument_tests` 与 `tests` 两个测试模块。

### 保真度与测试身份

27 个被搬移项中 **25 项** 在去空白、去可见性前缀后**逐项完全一致**；未配对 1 项是 `execute_v2_archive`——它按 Plan 要求**留在父模块**（组合入口）。唯一"差异"项 `transfer_failure_allows_refresh` 经 diff 确认只是 rustfmt 因新增 `pub(crate)` 前缀而把签名折成多行，函数体完全相同。**语义差异 0。**

测试身份（`p6-baseline-desktop.log` → `p8-after.log`）：Desktop **378 条结果、365/0/13**，移除 0、新增 0、状态改变 0；`production::` 定向运行在五个微批后均为 **18/18**（360 filtered out）。

### Plan §3 约束的落实

- **containment 失败不 fallback 普通 spawn**：`transfer_once` 的 linux 变体与 `TransferAttemptFailure` 一同迁入 `transfer.rs`，同一模块内保持"拒绝启动即失败"的路径；测试 `transfer_containment_failure_never_falls_back_to_plain_spawn` 仍通过。
- **retained owner 与 drain**：`retained_supervisor` 字段与 `retry_retained_drain*` 同在 `transfer.rs`，未分散到不同模块。
- **单次 refresh**：`transfer_with_one_refresh` 与 `transfer_failure_enters_refresh` 同模块，重试语义未被拆开。
- **媒体集合身份**：`ensure_same_media_collection` 随 transfer 一起迁移。

### `include!` fixture 的处理

`production_transfer_fixture.rs` 通过 `#[cfg(all(test, target_os = "linux"))] include!("production_transfer_fixture.rs")` 展开到 `production` 模块作用域内，因此它调用的是 `production::*` 下的名字。**fixture 文件本身未移动、未被修改**；父模块以 `pub(crate) use transfer::{...}` 把 transfer 的完整可见面桥接回 `production::*`，使 include 展开后的非限定调用继续解析。这是保持原路径而非重写 fixture 的选择。

### 可见性与告警

`pub(crate) use` 用于原本就在 crate 内可见的面（`execute_aria2_archive`、`retry_retained_drain`、`format_transfer_failure`、`download_v2`/`extract_v2`/`verify_downloaded_files`、`execute_gallery_dl_archive`、`media_type_name`、discovery 三项、tool-compatibility 三项）；`TransferAttemptFailure` 的两个字段用 `pub(in crate::production)`，与结构体本身的作用域一致。**没有一处提升为 `pub`。**

`cargo check -p xarchive-desktop --lib --tests` 回到基线两条告警，**零新增**。父模块中被迁移带走而失效的导入（`Component`/`PathBuf`/`Arc`/`AtomicBool`/`Ordering`/`Duration`/多数 `xarchive_download` 项）已删除；fixture 仍需要的 `std::path`、`MediaTransferPlan`、`TransferFailure`、`TransferFailureCode`、`media_transfer_plan`、`ExtractionResult`、`ExtractionMediaType`、`Duration` 改为 `#[cfg(test)]` 限定。

### P8 最终日志

| 检查 | 退出码 | 结果 |
|---|---:|---|
| `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored** |
| `cargo test -p xarchive-desktop --lib production:: -- --test-threads=1` | 0 | 18 passed / 0 failed |
| `cargo fmt --all -- --check` | 0 | 干净 |
| `git --no-pager diff --check` | 0 | 干净 |

本轮未运行 workspace Full、未动架构门禁/基线、未 staging/commit、未做 Windows 检查；aria2 真实验收仍未进行，不因本次拆分被视为已完成。

## P9 Executor Runtime 拆分（已完成，2026-11-09）

`desktop/src-tauri/src/executor/runtime.rs` 原 2692 行。这是 Plan 中风险最高的批次，按 §3 顺序只做两步：先 startup，再 application_service。

| 顺序 | 模块 | 行数 | 职责与真实符号 |
|---|---|---:|---|
| P9.1 | `executor/runtime/startup.rs` | 981 | `impl ExecutorRuntime` 全部（`new`、`with_config`、`with_production_factory`、`recover_startup*`、containment 供给、shutdown/join） |
| P9.2 | `executor/runtime/application_service.rs` | 883 | `ArchiveApplicationService` 结构体与其 `impl` |

`runtime.rs` 从 2692 行降到 **896 行**。

### 控制面未被拆散（Plan §3 明确要求）

以下 13 项**全部留在 `runtime.rs` 原位**，逐项验证存在：

`JobRecord`、`Command`、`RunnerCommand`、`RunnerOperation`/`RunnerOperationTask`、`OwnedRunnerAdmission`、`CancellationRegistry`、`JobExecutorHandle`、`JobExecutor`、`impl JobExecutor`、`impl Drop for JobExecutor`、`run_runner`、`run_worker`、`handle_command`、`transition_job`、`cancel_job`、`sidecar_crash`。

即 worker/runner、admission、取消注册与 shutdown/join 仍在同一控制边界内，没有被这次拆分分散到不同模块。

### 保真度与测试身份

三个被搬移项在去空白、去可见性前缀后**逐项完全一致**：`impl ExecutorRuntime`、`struct ArchiveApplicationService`、`impl ArchiveApplicationService`。**语义差异 0。**

测试身份（`p6-baseline-desktop.log` → `p9-after.log`）：Desktop **378 条结果、365/0/13**，移除 0、新增 0、状态改变 0，ignored 13 → 13，`executor::*` **129 → 129**。P9.1 与 P9.2 各自完成后都跑过完整 Desktop lib，结果均为 365/0/13。

### 可见性处理（未放宽到 crate 外）

- `executor.rs` 的 `pub use runtime::{ArchiveApplicationService, ...}` 保持不变，因此父模块需以 `pub use` 重导出该结构体（它本身仍是 `pub`）；
- `ArchiveApplicationService` 的两个字段 `executor`/`v2_enrollment` 原为 `pub(super)`（即 `runtime` 内），迁移到子模块后会被 `executor::session` 与 executor 子树测试使用，改为 **`pub(in crate::executor)`**，作用域仍限于 executor 子树；
- `ExecutorRuntime::with_production_factory` 与 `recover_startup_with_v2` 原为 `pub(super)`，改为 **`pub(crate)`**——它们在迁移后由 `executor::tests::*` 跨模块调用。这是本次唯一一次比原 `pub(super)` 更宽的作用域，且**仍限于 crate 内**，未提升为 `pub`。此处与 P5–P8 的"完全不放宽"不同，已在证据页显式记录，不视为等价。

### 告警

`cargo check -p xarchive-desktop --lib --tests` 回到基线两条告警，**零新增**。父模块与两个子模块中因迁移失效的导入全部清理；`cargo fmt --all -- --check` 退出 0。

### P9 最终日志

| 检查 | 退出码 | 结果 |
|---|---:|---|
| `cargo test -p xarchive-desktop --lib -- --test-threads=1` | 0 | **365 passed / 0 failed / 13 ignored** |
| `cargo fmt --all -- --check` | 0 | 干净 |
| `git --no-pager diff --check` | 0 | 干净 |

本轮未运行 workspace Full、未动架构门禁/基线、未 staging/commit、未做 Windows 检查。P9 涉及生命周期与运行中取消，其**真实 delegated 生产路径**（InternalV1 恢复/发布/终态持久化）按验证政策需另行执行或证明旧证据可复用，本轮未执行，不记为 PASS。

## 逐轮记录要求


每轮按统一验证政策填写稳定 ID、target、priority、owner、implementation/status、非 PASS 原因、source SHA、dirty identity、build origin、适用的 artifact/platform/environment/tool versions、method、prerequisites、可复现 steps、expected、命令/真实退出码/日志、blocking、revalidation 和 follow-up。不得把未执行、零选择、ignored、mock 或其他平台结果记为 PASS；失败和重试均保留。

## 边界与待办

- 每一批重查 Git/游标；P0 前状态不能替代后续工作区核验。
- Windows 原生验证归 Windows Owner；aria2 延期不记 PASS。
- 只有迁移完成后的 P13 才运行最终 Full 集成；早期 targeted/module 通过不代表目标完成。
- P1 台账编号不以源码顺序排列：第 1–2 条为两个 Storage 测试源，第 3 条为 Storage `tests/mod.rs` 父模块（P3 硬约束），第 4–22 条按“先建基线、后按批次”记录，第 22–23 条为本轮两名 Telegram 文件的最终台账。编号仅作引用锚点，不代表阅读顺序或批次顺序。
- P1 已完成的 22 项文件来源与行数（实测 `wc -l`，与计划 §4 表格一致）：executor.rs 9815、telegram_send.rs 3153、storage tests/archive_recovery.rs 2748、executor/runtime.rs 2692、telegram lib.rs 2691、storage tests/archive_service.rs 2309、batch.rs 1968、config.rs 1814、archive.rs 1745、storage archive_service.rs 1614、production.rs 1595、protocol sidecar_v2.rs 1548、storage file_store.rs 1266、runtime.rs 1258、download supervisor.rs 1218、proxy.rs 1182、websocket_transport.rs 1146、commands/records.rs 1097、storage archive_recovery.rs 1097、telegram upload.rs 1033、workflow writer_lifecycle.rs 1030、transport.rs 1007。
- P1 期间未修改任何生产源码、测试源码或 Cargo 清单；仅新增/更新文档。`desktop/src-tauri/src/executor.rs.pre-r5-recovery.bak` 保持未跟踪、未暂存、SHA-256 不变。
- 第 22 条与第 23 条是本轮唯一改写的台账内容：此前版本曾以未核实的分组区间（“约 20 项”“从约 27 左右开始”）记录 Telegram 47 项测试，并误置于「边界与待办」之下。本轮已用源码逐行定位替换为准确的 47 项／30 项测试名与行号，并移回台账区内。凡与本文件旧表述冲突，以本条目为准。
