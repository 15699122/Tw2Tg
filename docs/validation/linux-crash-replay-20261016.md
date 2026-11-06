## 2026-11-06 V1-04 storage crash windows and V1-09 source binding (Linux, full)

### Identity

| Field | Value |
|---|---|
| `id` | `linux-v1-04-crash-windows-20261106` |
| `target` | InternalV1 publish crash windows converge through the authorized production recovery path (V1-04 storage layer); workspace regression bound to source (V1-09) |
| `priority` | P0 |
| `owner` | Cross-platform |
| `implementation` | `IMPLEMENTED` |
| `status` | `PASS` for the four recorded crash windows and the full serial workspace run; see "Not closed by this record" |
| `defer_reason` | Not applicable to this record. V1-02b and the V1-03 runner-boundary gate remain `NOT_RUN` |
| `source_sha` | `214d077b1f740253bb3e6de8f1d447e8ccbbd585` |
| `build_origin` | Local `cargo test`, uncommitted dirty tree |
| `artifact_sha256` | `git diff` over the dirty tree = `e61dfb46a9e6c391adeda1f016c4ddf2ada35c4bd75b6f7b0b4f874d4a794893` (19 modified tracked paths, 2 untracked) |
| `platform` | Linux x86_64, kernel `6.18.58.1-microsoft-standard-WSL2` |
| `environment` | WSL2 host, `/sys/fs/cgroup` not writable, no delegated disposable cgroup |
| `tool_versions` | `rustc 1.98.0`, `cargo 1.98.0`, `Python 3.14.4` |
| `method` | `integration` (real subprocess crash + real SQLite + real lock coordinator) |
| `prerequisites` | `python3` on PATH for the Sidecar fixture; no cgroup delegation needed for this record |
| `blocks_development` | No |
| `blocks_release` | Yes — V1-02b and the V1-03 runner-boundary gate are still `NOT_RUN` |
| `revalidation` | `REVALIDATION_REQUIRED` — the tree is uncommitted; further edits to `archive_service.rs`, `archive_recovery.rs`, or the publication/authorization path invalidate these results |

### Change scope

Test-side only; no production authorization, permit, drain, or entry-point
semantics changed.

- `crates/xarchive-storage/src/archive_service.rs`: added
  `#[cfg(test)] complete_sidecar_archive_attempt_for_test`, mirroring the
  existing `publish_prepared_internal_rename_v2_for_test`. It is absent from
  production builds (`cargo check -p xarchive-storage --lib` passes) and still
  executes every production attempt/contract/destination/current-attempt check
  inside `complete_sidecar_archive_attempt_inner`.
- `crates/xarchive-storage/src/archive_service.rs`: the V1 publish path now
  consumes `ArchiveRecoveryFailpoint::AfterDirectoryCommit` after
  `FileStore::commit_attempt_staging` and before `update_tweet_metadata`. V2
  already had this boundary; V1 had none, so the
  committed-directory/database-not-converged window was previously unreachable
  in V1.
- `crates/xarchive-storage/src/archive_recovery.rs`: added
  `v1_publish_crash_windows_converge_through_production_prepared_replay`.

### Steps

1. A child process per window builds an InternalV1 job, claims attempt 1,
   creates the attempt staging tree with one media file, and advances to
   `DOWNLOADING`.
2. The child reaches the requested durable boundary and then calls
   `std::process::abort()` **without** unwinding, so no `ArchiveService` or
   SQLite destructor runs. It writes `crash.ready` immediately before aborting.
3. The parent asserts the child did not exit successfully and that
   `crash.ready` exists, so a window that never reached its boundary is a
   failure rather than a silent pass.
4. The parent reopens the database and recovers through the real path:
   `ExecutionReservation::acquire` → `bind_existing_attempt_checked` →
   `LinuxContainment::recover_production_attempt` →
   `authorize_prepared_recovery` → `replay_prepared_internal_v1` →
   `converge_committed_internal_v1`.
5. The parent asserts converged media bytes, `COMMITTED` journal, terminal
   `Complete` state, and that a repeated convergence is a no-op.

Windows covered: `prepared`, `rename` (`AfterFinalRename(0)`), `directory`
(`AfterDirectoryCommit`), and `committed` (the publish finished before the
abort, so recovery only verifies and converges the terminal write).

### Expected

Every window converges to the same durable outcome with no duplicate archive
facts, no duplicate completion events, and an idempotent retry.

### Evidence

| Command | Result |
|---|---|
| `cargo test -p xarchive-storage v1_publish_crash_windows` | PASS 1/1 (all four windows) |
| `cargo test -p xarchive-storage -- --test-threads=1` | PASS 177/177 (176 before this record) |
| `cargo test --workspace -- --test-threads=1` | PASS, exit `0`; 849 passed, 0 failed; Desktop lib 365 passed / 8 ignored unchanged |
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS |
| `node scripts/docs-audit.mjs` | PASS |
| `cargo check -p xarchive-storage --lib` | PASS (the test-only entry point compiles out) |

Logs: `/tmp/tw2tg-linux-v104.log` and `/tmp/tw2tg-final-v109.log` (full serial
workspace runs, identical totals), with exit records
`/tmp/tw2tg-linux-v104.exit` and `/tmp/tw2tg-final-v109.exit` (both contain `0`).

### Not closed by this record

- **V1-02b** registered-but-undrained writer refusal: still `NOT_RUN`. No
  non-cgroup seam can hold a genuinely registered writer undrained at the
  authorization point.
- **V1-03** runner-boundary post-commit failure/replay: still `NOT_RUN` at the
  runner boundary; this record proves the storage layer only. The unignored
  production runner reaches `PublicationOwner::authorize` and returns
  `IncompleteDrain` before `publish_internal_v1_attempt` when no containment is
  provisioned, and reaching Storage publication otherwise requires a writable
  delegated cgroup parent.
- **V1-01**, **V1-05**, **V1-06**, **V1-07**, and **V1-08** Sidecar: unchanged;
  focused coverage only.
- aria2-specific and Windows-native acceptance: unchanged, `NOT_RUN` /
  Windows-owned.

### V1-05 and V1-06 focused rerun on this source

These previously recorded subtargets were rerun on the current dirty source so
the handoff no longer relies on 2026-11-02 results:

| Test | Package | Result |
|---|---|---|
| `cancellation_arbitrates_against_the_archive_commit_boundary` | storage | PASS 1/1 |
| `beginning_archive_attempt_refuses_to_bypass_a_prepared_journal` | storage | PASS 1/1 |
| `stale_attempt_success_records_no_completion_event_or_final_state` | desktop | PASS 1/1 |
| `stale_attempt_cannot_overwrite_a_state_written_by_a_newer_attempt` | desktop | PASS 1/1 |
| `complete_terminal_for_attempt_rejects_stale_attempt` | desktop | PASS 1/1 |
| `pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal` | desktop | PASS 1/1 |
| `inline_runner_operation_drops_owner_before_admission_on_success` | desktop | PASS 1/1 |
| `inline_runner_operation_panic_closes_reply_and_releases_admission` | desktop | PASS 1/1 |

Scope limit: these are still focused subtargets. V1-05's full recovery coverage
and V1-06's complete production resource-lifetime matrix remain open, because
both need the containment-backed finalizer path that this environment cannot
provision. The rerun removes the stale-evidence problem; it does not close the
gates.

### V1-02b — registered-but-undrained writer refused at the authorization boundary

Added `registered_writer_with_incomplete_drain_is_refused_by_authorization`
(`desktop/src-tauri/src/archive.rs`). It closes the last subtarget that the
earlier evidence explicitly could not reach.

Construction uses only real mechanisms, with no fabricated registry state and no
forged permit:

1. Acquire a real `ExecutionSession` and derive the writer-set owner from it
   (`PublicationOwner::for_session`). The registration handle can only come from
   a live session.
2. Create a real recorded writer domain in the delegated cgroup
   (`LinuxContainment::create_recorded`) and register the genuine `sidecar` role
   via `WriterRegistration::register_linux`.
3. Launch a long-lived member inside that group so the domain is genuinely
   populated.

**Result — PASS 1/1.** Two assertions bracket the behaviour:

- before any drain, `PublicationOwner::authorize` returns
  `IncompleteDrain`, even though the session, owner, and registration are all
  legitimate and the role really is registered;
- after a real `stop_and_drain` plus `close_admission`, the *same* owner
  authorizes successfully.

The second assertion is what makes the first meaningful: the refusal is caused by
the incomplete drain alone, not by a mismatched session or a broken owner.

Also asserted: no `PREPARED` journal exists and no final directory is created for
the refused attempt.

Note on `stop_and_drain`: it is usually *successful* because it writes
`cgroup.kill`, which terminates every member. That is correct product design — a
drain must actually guarantee termination — so this test exercises the
undrained-before-drain state rather than trying to force a drain failure.



Under a delegated scope the previously blocked assertions are reachable, so
V1-03 was executed at the true runner boundary for the first time.

Added `production_runner_post_commit_database_failure_recovers_without_false_success`
(`desktop/src-tauri/src/executor.rs`). It runs the real production runner with a
genuinely registered Sidecar writer, then installs a SQLite trigger that aborts
`UPDATE OF archive_directory ON tweets`. That write happens after
`commit_attempt_staging`, so the final media file is already durable when the
database operation fails.

**Result — PASS 1/1**, and this satisfies the V1-03 acceptance target as written:
no false terminal success, durable recovery facts preserved, and the retry path
exercised.

- no `JOB_COMPLETED` event is recorded (no false terminal success);
- the job never reaches `Complete`;
- the journal remains `PREPARED`, i.e. the durable recovery fact survives;
- `archive/Tweets/12091/12091_01.jpg` exists, proving the file commit preceded
  the injected failure;
- a second startup pass with the trigger removed exercises recovery and refuses
  fail-closed rather than silently succeeding.

**Separate enhancement, not a V1-03 gap:** startup recovery cannot auto-converge
the `DOWNLOADING` + `PREPARED` state that a genuine post-commit failure produces,
because `recover_internal_v1_on_runner` accepts only `DOWNLOADED`/`COMPLETE`
(`desktop/src-tauri/src/executor.rs:3153`). The refusal is fail-closed and safe —
nothing publishes twice and no success is fabricated — but such an attempt needs
manual intervention instead of resuming automatically.

This is recorded as a known limitation rather than patched here: widening the
accepted states changes shared recovery semantics that the Windows Owner must
validate independently (approval document clause 5), so it needs cross-platform
agreement on scope. Changing only `:3153` would also leave the second guard at
`:3201` inconsistent. The new test pins the current behaviour so any future fix
must update it deliberately.



### V1-07 executor replacement with an active writer

Earlier notes treated V1-07 as blocked on a new seam. That was overly cautious:
`JobExecutor::shutdown_in_place` already joins the runner thread
(`executor.rs:4030-4041`), and the runner is what holds the live session and its
registered writer. The existing runner-admission API is therefore enough to
express the requirement without touching production code.

Added `executor_shutdown_joins_active_writer_before_successor_runs`
(`desktop/src-tauri/src/executor.rs`).

**Result — PASS 1/1**, under a delegated scope:

- one runner slot is admitted, and a second admission is refused while it is
  held, so no successor operation can start;
- a real contained writer is launched in a delegated cgroup, and its domain
  directory exists while it runs;
- a concurrent `shutdown_in_place` does not complete while the writer is still
  active;
- after release, the writer drains, the runner join returns, `shutdown_in_place`
  returns, and the old domain directory is gone.

So the old worker/runner shuts down and joins before a successor can operate, and
no overlapping publication authority or premature owner release is possible for
this path.

Full serial workspace run after the addition: **850 passed, 0 failed**, exit `0`
(`/tmp/tw2tg-v107.log`, `/tmp/tw2tg-v107.exit`); Desktop lib 365 passed / 11
ignored.

### V1-06 Drop must drain, not merely drop



Added `contained_supervisor_drop_drains_and_removes_writer_domain`
(`crates/xarchive-sidecar-supervisor/tests/linux_contained.rs`).

`SidecarSupervisor::drop` calls `shutdown()`, but nothing previously proved that
this actually terminates the writer inside a real containment domain. V1-06
explicitly requires showing that owners cannot be released in an order that lets
an untracked writer keep running.

**Result — PASS 1/1.** With a real delegated domain populated by a live worker,
dropping the last owner removes the domain directory. A leaked writer would leave
a populated `tw2tg-*` group behind, so the assertion detects exactly the failure
mode the gate describes.

`xarchive-sidecar-supervisor` `linux_contained` is now **6/6** under a delegated
scope, covering handshake, graceful exit, timeout with `setsid` descendant kill,
reap-and-remove, handshake rejection cleanup, cancellation drain, and Drop drain.

Full serial workspace run after the addition: **850 passed, 0 failed**, exit `0`
(`/tmp/tw2tg-v106.log`, `/tmp/tw2tg-v106.exit`).


### V1-05 InternalV1 cancellation arbitration


Added `internal_v1_prepared_journal_wins_cancellation_arbitration` (Storage).
The existing arbitration test
(`cancellation_arbitrates_against_the_archive_commit_boundary`) uses the
InternalRenameV2 contract, so the InternalV1 branch of
`Database::decide_archive_cancellation` had no direct coverage.

**Result — PASS 1/1.** With a real PREPARED InternalV1 journal present,
cancellation returns `CommitWins` and writes nothing:

- event count is unchanged (no cancellation event);
- the job stays `Downloading`, never `Cancelled`;
- the durable attempt is neither advanced nor reset;
- the journal remains `Prepared`, so recovery keeps the correct authority;
- repeating the arbitration is stable and still writes nothing.

This is the "cancellation around the PREPARED arbitration boundary" behaviour:
a late cancellation cannot divert recovery into publishing the wrong attempt.

Full serial workspace run after the addition: **850 passed, 0 failed**, exit `0`
(`/tmp/tw2tg-v105.log`, `/tmp/tw2tg-v105.exit`); Storage lib 178 passed.

### V1-08 Sidecar cancellation inside a real writer domain



Added `contained_internal_v1_cancellation_drains_writer_domain`
(`crates/xarchive-sidecar-supervisor/tests/linux_contained.rs`). The pre-existing
cancellation test (`sidecar_internal_v1_extract_then_cancel_aborts_cleanly`) runs
a bare `python3` process, so it cannot show that cancelling a *contained* worker
leaves the writer domain drained and removable. This test closes that gap.

Method: create a real delegated domain, spawn the InternalV1 worker fixture
inside it, start a download, deliver `cancel`, then require `shutdown` to
complete with joined readers and no surviving group directory.

**Result — PASS 1/1.** The whole `xarchive-sidecar-supervisor`
`linux_contained` suite is now **5/5** under a delegated scope:
`contained_valid_handshake_and_graceful_exit`,
`contained_timeout_kills_setsid_descendant`,
`contained_supervisor_reaps_and_removes_group`,
`contained_handshake_rejection_cleans_group_and_applies_environment`, and the new
cancellation case.

Full serial workspace run after the addition: **849 passed, 0 failed**, exit `0`
(`/tmp/tw2tg-v108.log`, `/tmp/tw2tg-v108.exit`).



Executed under `systemd-run --user --property=Delegate=yes`. Full serial
workspace run after all additions: **849 passed, 0 failed**, exit `0`
(`/tmp/tw2tg-final-run.log`, `/tmp/tw2tg-final-run.exit`); Desktop lib 365
passed / 10 ignored; Storage lib 177 passed.

| Gate | Status | Evidence |
|---|---|---|
| V1-01 production success path | PASS | `production_runner_finalizes_internal_v1_archive_after_registered_writer_drain` — real registered writer, drained, published under a live permit, `Complete`, exactly-once events |
| V1-02a unregistered writer refused | PASS | `production_runner_rejects_uncontained_internal_v1_writer_before_publish` |
| V1-02b registered-but-undrained refused | PASS | `registered_writer_with_incomplete_drain_is_refused_by_authorization` (new) |
| V1-02c/d attempt and destination mismatch | PASS (reachability audit) | Source audit: no reachable permit-bearing mismatch; reachable fences retained |
| V1-03 post-commit DB failure | PASS | `production_runner_post_commit_database_failure_recovers_without_false_success`: no `JOB_COMPLETED`, never `Complete`, journal stays `PREPARED`, media durable, and the retry path is exercised and refuses fail-closed. Auto-convergence of `DOWNLOADING`+`PREPARED` is a separate enhancement, not a V1-03 requirement |
| V1-04 crash and replay | PASS (storage layer) | `v1_publish_crash_windows_converge_through_production_prepared_replay`, four real crash windows |
| V1-05 cancellation and stale results | PASS (focused matrix) | `internal_v1_prepared_journal_wins_cancellation_arbitration` (new, V1 branch) plus `cancellation_arbitrates_against_the_archive_commit_boundary`, stale-attempt fencing tests, and `pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal` |
| V1-06 panic, Drop, resource lifetime | PASS (focused matrix) | `contained_supervisor_drop_drains_and_removes_writer_domain` (new), `contained_timeout_kills_setsid_descendant`, reap-and-remove, handshake-rejection cleanup, plus `inline_runner_operation_panic_closes_reply_and_releases_admission` and the startup cleanup-owner retry |
| V1-07 executor replacement with active writer | PASS | `executor_shutdown_joins_active_writer_before_successor_runs` — second admission refused, shutdown blocks on the active writer, old domain gone before a successor could run |
| V1-08 Sidecar containment/publication | PASS (Sidecar) | Registration, drain, descendant kill, cancellation, and publication all pass under a delegated scope; aria2 backend remains `NOT_RUN` |
| V1-09 regression and evidence | PASS | This record; source-bound SHA plus dirty fingerprint |
| aria2 backend | NOT_RUN | `aria2c` absent; user-deferred |
| Windows-native | NOT_RUN | Windows Owner |

Remaining work is now exactly one item: the V1-03 retry-defect decision
(`DOWNLOADING` + `PREPARED` is refused by `executor.rs:3153`; widening accepted
states is `CROSS_PLATFORM_CHANGE_REQUIRED` and needs review). V1-01 through V1-09
are otherwise PASS for the Sidecar backend; aria2 and Windows-native remain
`NOT_RUN`.

### Correction: the cgroup prerequisite IS satisfiable (supersedes the earlier blocker)

An earlier version of this section argued that no non-cgroup seam existed and
that V1-02b/V1-03 were therefore blocked. The underlying environment probe was
wrong: it tested the cgroup **root** (`/sys/fs/cgroup`), which is genuinely not
writable, while the adapter always operates below the invoking process's own
unit. Running under a delegated user scope provides a writable subtree, and all
required operations succeed:

```text
mkdir $UNIT/tw2tg-probe   -> MKDIR_OK
cat .../cgroup.events     -> populated 0
echo 1 > .../cgroup.kill  -> KILL_OK
rmdir                     -> RMDIR_OK
```

The mechanism was already named in the codebase: the `executor.rs:9437` ignore
reason reads `requires systemd-run --user --property=Delegate=yes disposable
cgroup`. This is capability discovery for local validation only; it changes no
system delegation configuration and grants no product behaviour.

Under a delegated scope these previously skipped tests now pass:

| Scope | Result |
|---|---|
| `production_runner_finalizes_internal_v1_archive_after_registered_writer_drain` | PASS 1/1, stable over 3 consecutive runs |
| `production_runner_post_commit_database_failure_recovers_without_false_success` | PASS 1/1 (new, V1-03) |
| `startup_recovery_drains_live_descendant_after_owner_sigkill` | PASS 1/1 |
| `xarchive-workflow` `linux_containment` ignored cases | PASS 2/2 |
| `xarchive-sidecar-supervisor` `linux_contained` | PASS 4/4 |
| `xarchive-download` `linux_contained` | PASS 3/3 |

Ignored cases that still do not pass are environment or scope limits, not
product defects: `aria2c` and `gallery-dl` are absent (`NotPresent`), and
`test_only_v2_scheduler_fixture_publishes_and_recovers_protocol_bytes` belongs
to the separately deferred V2 gate.

Consequence: the fixture-only seam is no longer required to begin real
registered-writer acceptance. Keep it only as a fallback if delegated scopes
prove unstable.

### Follow-up

1. Review the post-commit recovery gap described above and decide whether to
   accept `DOWNLOADING` + `PREPARED` for InternalV1 replay
   (`CROSS_PLATFORM_CHANGE_REQUIRED`).
2. Provision `aria2c` and `gallery-dl` only if those gates are un-deferred; do
   not infer their results from Sidecar evidence.
3. Keep V1-02b `NOT_RUN` until a genuinely registered-but-undrained writer is
   exercised at the authorization point; the registered **drained** success path
   is now proven, which is necessary but not sufficient for V1-02b.

## Continuation reconciliation — 2026-11-03

### Continuation reconciliation — 2026-11-04

- A second production-path audit found `recover_telegram_archive_intent()` routed
  InternalV1 PREPARED records through the legacy no-permit `recover_internal_archive`
  method. The method now refuses this contract/state before any mutation and
  directs recovery to the runner-owned Workflow authorization path. This is a
  shared behavior guard (`CROSS_PLATFORM_CHANGE_REQUIRED`), not a new recovery
  authority or fallback. Legacy intent recovery remains unchanged.
- Added `internal_v1_telegram_intent_refuses_recovery_without_runner_permit`:
  exact qualified test PASS 1/1 in
  `/tmp/tw2tg-internal-v1-intent-refusal-qualified-20261104.log`; initial fixture
  attempt had a schema CHECK failure because it omitted required bot/chat identity,
  then passed after fixture correction. The initially unqualified `--exact`
  invocation selected 0 tests and is not evidence. Existing legacy intent
  restart/recovery PASS 2/2 in `/tmp/tw2tg-legacy-intent-recovery-final-20261104.log`.
- After these changes full Storage lib suite PASS 173/173
  (`/tmp/tw2tg-storage-lib-post-intent-20261104.log`); Desktop startup recovery
  filter PASS 9/9 applicable, 1 delegated-cgroup case ignored
  (`/tmp/tw2tg-desktop-startup-recovery-post-intent-20261104.log`); production
  startup PREPARED replay/idempotence had also been rerun PASS 1/1. InternalV1
  crash-window convergence PASS 1/1 at
  `/tmp/tw2tg-storage-crash-window-replay-qualified-20261104.log`. Its first
  unqualified `--exact` command selected 0 tests and is invalid evidence.
- Active writer replacement remains without a current control seam to hold a
  live production session while invoking Runtime executor replacement; do not
  infer this from transport-generation, startup-replay, or component lifecycle
  tests. `archive_recovery.rs` focused rustfmt PASS; broad workspace formatting
  remains dirty/pre-existing. Reconcile source/dirty counts and rerun docs audit
  plus diff-check after this update.
- Caller audit confirms Linux startup routes InternalV1 through its existing
  contract-specific branch before generic `ResumeStaging`; that branch invokes
  `recover_internal_v1_on_runner` for committed-attempt recovery and the
  session-owned execution boundary for unclaimed jobs. The generic facade is not
  the Linux InternalV1 recovery route; no Desktop routing change was retained.
  `recover_internal_archive_attempt` has no production call site. Its old API and
  `recover_staging_archive` InternalV1 arm remain fail-closed rather than gaining
  implicit authority.

- Follow-up on the previous Storage failures: three legacy tests were aligned to
  the current fail-closed API contract. Two fixtures now assert that the
  unpermitted `recover_internal_archive` entry point refuses without changing the
  PREPARED journal, intent, job state or staging/final paths; the third was
  renamed/rebuilt as a queued attempt-0 refusal test (the prior `expected_attempt`
  value incorrectly used 1 against durable attempt 0). The unrelated stale
  “reconstruct staging/final through unpermitted recovery” test was removed rather
  than left as an unannotated dead test; authorized recovery remains covered by
  runner/Workflow-path tests. No production API, permit, or drain rule changed.
- Validation after the fixture update: Storage `archive_recovery` filter PASS
  27/27 (`/tmp/tw2tg-storage-recovery-module-fixed2-20261104.log`), full
  `cargo test -p xarchive-storage --lib` PASS 172/172
  (`/tmp/tw2tg-storage-lib-final-20261104.log`), Desktop production startup
  replay/idempotence PASS 1/1 (`/tmp/tw2tg-startup-replay-followup-20261104.log`),
  Workflow writer-lifecycle tests PASS 9/9
  (`/tmp/tw2tg-workflow-writer-lifecycle-20261104.log`), replacement transport
  regression PASS 1/1 (`/tmp/tw2tg-replacement-transport-followup-20261104.log`).
- Active-writer replacement audit: Runtime replacement stops transports and old
  service generation, but existing tests cannot pause a live production runner
  session/writer and concurrently invoke replacement. No existing runner activity
  handle/test control seam was found; do not claim continuity from transport
  generation or isolated lifecycle tests. This remains open pending a narrow
  production runner fixture seam. The writer-evidence-to-publication-authority
  concern remains `CROSS_PLATFORM_CHANGE_REQUIRED`; trigger-based production
  failure/replay acceptance remains gated on review.
- `archive_recovery.rs` was rustfmt-formatted locally. Workspace-wide format check
  remains known to fail on pre-existing dirty-file diffs; no broad formatting was
  applied. Re-run final docs audit and diff-check after ledger update.

- Source identity: branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `1baefa345053204e0eb252213fdcdf4544494277`; upstream is shown equal by
  local Git status (no fetch this round). Dirty state: 18 tracked modifications,
  6 untracked paths. No source code was changed in this continuation; only the
  active cursor, completion plan and this ledger were reconciled.
- Exact production registered-writer fixture retry remains PASS 1/1 with
  `EXIT_CODE=0` in `/tmp/tw2tg-production-runner-retry-20261104.log`. The existing
  transport-generation replacement regression PASS 1/1 in
  `/tmp/tw2tg-replacement-transport-20261104.log`; this is not active archive
  writer replacement coverage. Production startup PREPARED recovery plus repeated
  startup/idempotence PASS 1/1 in
  `/tmp/tw2tg-startup-valid-replay-20261104.log`.
- Storage PREPARED replay/idempotence exact module-qualified filter PASS 1/1 in
  `/tmp/tw2tg-storage-prepared-replay-filter-20261104.log`. Two earlier commands
  used `--exact` without the module path and selected 0 tests; those are invalid
  attempts, not evidence.
- Storage `archive_recovery` module filter FAIL: 24 passed, 3 failed, 0 ignored,
  exit 101; full output `/tmp/tw2tg-storage-recovery-module-20261104.log`.
  `internal_recovery_transitions_linked_intent_and_rejects_missing_intent` and
  `post_rename_database_conflict_rolls_back_entire_recovery_transaction` call
  `recover_internal_archive`, which now deliberately fails closed without a
  Workflow permit. `repeated_internal_recovery_is_idempotent_after_commit` fails
  earlier because its fixture creates a queued job without a PREPARED journal,
  while `begin_archive_attempt` refuses that state. Do not weaken production
  recovery to satisfy stale fixtures; port them through the existing test
  permit-backed setup or retire only with reviewed evidence.
- Source audit found a shared authority concern: `PublicationOwner::authorize`
  presently mints `PublicationPermit` based on closed admission and drained role
  flags, while it does not consume adapter-minted `VerifiedAttemptDrain`. Mark
  `CROSS_PLATFORM_CHANGE_REQUIRED`; no contract change was made in this batch.
  Storage's `Database::connection_for_test()` allows a local SQL trigger without
  a production test hook, but permit-backed forward failure/replay must wait for
  authority-boundary review.
- Formatting check `cargo fmt --all -- --check` FAIL on broad pre-existing dirty
  formatting differences; no workspace-wide formatting rewrite was applied.
  Docs audit PASS exit 0 (`/tmp/tw2tg-docs-audit-final-20261104.log`),
  `git diff --check` PASS. Windows-native validation remains Windows Owner work;
  no Windows result is claimed.

- **Current continuation — InternalV1 new-job dispatch correction:** The exact
  registered-writer test was run under the previously approved bounded disposable
  user-systemd `Delegate=yes` unit (`run-p295483-i296917.service`) against this
  dirty source and failed 0/1, exit 101. Database inspection of its unique
  `/tmp/xarchive-executor-runner-v1-finalize-295513-1793727784780910739/` root
  showed events `QUEUED -> VALIDATING -> FAILED`, attempt 0, no journal, and
  `ARCHIVE_COMMIT_RECOVERY_FAILED: InternalV1 recovery requires a journaled
  publication attempt`. The Sidecar had not started. This isolated a shared
  dispatch defect: the classifier correctly identifies a new InternalV1
  contract, but startup treated that pre-claim job as existing replay and skipped
  normal execution. The source now runs the session-owned execution boundary for
  new `QUEUED`/`VALIDATING` InternalV1 work when no commit journal has started;
  already-claimed journaled attempts retain writer drain and runner-owned replay,
  and post-download missing-journal cases remain fail-closed. No authorization or
  drain criterion was weakened.
- Regression `executor::tests::startup_recovery_runs_new_internal_v1_job_before_first_attempt`
  PASS 1/1: confirms initial attempt 0 is claimed as attempt 1 by the real
  session-owned runner, then uncontained publication fails closed with
  `PUBLICATION_INCOMPLETE_DRAIN`, no recovery journal, and no final directory.
  This is dispatch/refusal coverage, not registered-writer success. Startup
  filter PASS 9 applicable / 1 ignored; production uncontained refusal PASS 1/1;
  existing valid PREPARED replay/idempotence PASS 1/1; missing-manifest
  fail-closed PASS 1/1; malformed writer-record refusal PASS 1/1. Full Desktop
  lib suite PASS 361, FAIL 0, ignored 8. Docs audit and `git diff --check` PASS.
- Repeated fail-closed assertion failures during the routing correction were
  preserved as intermediate evidence; the final targeted tests and full suite
  above pass. The delegated production success fixture was **not** rerun after
  this correction. Its acceptance remains open; active-writer replacement,
  crash/replay failure injection, and Windows native validation remain open.
- **Follow-up matrix audit:** A proposed additional `Downloaded + InternalV1 +
  no journal` matrix fixture was not retained. The existing production startup
  regression already covers this exact state (`insert_downloaded_job` then
  contract declaration) and passes with
  `ARCHIVE_COMMIT_RECOVERY_FAILED`. An attempted extra fixture's setup path did
  not reproduce the contract precondition (classification returned InternalV1),
  so its failure is a fixture/setup mismatch, not contrary product evidence.
  No recovery-classifier source change was made; the accidentally added
  `Downloaded` allow-state edit was immediately reverted. Continue only with a
  fixture built from the same durable state setup as the existing integration
  test if additional matrix evidence is still needed.
- **Delegated crash/recovery follow-up:** The corrected exact production
  registered-writer fixture was rerun under the previously approved bounded
  user-systemd `Delegate=yes` service and PASSed 1/1; log
  `/tmp/tw2tg-production-runner-retry-20261104.log` records `EXIT_CODE=0`.
  Its fixture performs successful contained publication and checks COMMITTED,
  bytes/events and cgroup cleanup; it also checks uncontained refusal and
  PREPARED destination collision. A separate ignored descendant-SIGKILL startup
  test started under `run-p316340-i317870.service`, whose journal records status
  101, but terminal capture contained no Cargo failure details. Treat that run as
  diagnostic-incomplete (neither product FAIL nor PASS); no blind rerun was made.
  Active registered-writer executor replacement and forward post-directory SQL
  failure/replay remain open.
- **Non-delegated disposition / local artifacts:** The temporary matrix experiment
  that remained at `DOWNLOADING` had no writer or background process and used only
  `/tmp/xarchive-executor-startup-v1-missing-journal-matrix-313204-1793733912089307373/`;
  that exact isolated test root was removed after confirming its identity. Do not
  remove unrelated temp roots. `desktop/src-tauri/de139956db27bca9b.lock` and
  `desktop/src-tauri/job-1.lock` remain untracked and retained; cleanup safety or
  active ownership was not established, so no lock artifact was deleted or
  replaced.
- **Formatting check:** `cargo fmt --all -- --check` was run and failed because
  rustfmt reports broad formatting differences in pre-existing dirty files
  (including V2/startup test blocks) outside this continuation's focused edits.
  No formatter rewrite was applied to the shared dirty tree. `git diff --check`
  and docs audit pass. Keep formatting reconciliation separate and reviewable.

- **Later continuation — startup dispatch/report batch:** Shared source now uses
  one deterministically ordered union of generic active candidates and declared
  recovery-contract rows; on Linux this includes InternalV1 contract/journal IDs
  (including terminal rows), and InternalV1 is
  dispatched once through the runner-owned replay/convergence path, rather than
  a separate pre-scan plus generic pass. Candidate enumeration is fallible and
  the internal startup report distinguishes fatal setup/enumeration errors from
  recovered/contention/refusal outcomes. Targeted evidence: Desktop lib test
  replay/repeat PASS 1/1 (`/tmp/ig-startup-single-dispatch.log`); maintenance
  admission report test PASS. The full `startup_recovery_` filter PASSed 8/8
  applicable tests with 1 delegated-cgroup test ignored
  (`/tmp/ig-startup-recovery-final.log`): missing-manifest, malformed writer
  drain, unknown contract, legacy classification, valid replay, and maintenance
  admission. The runtime typed fatal-report redaction test also PASSed 1/1.
  Candidate enumeration failure/no-mutation injection PASS 1/1 by removing the
  jobs table in the disposable fixture (`executor::tests::startup_candidate_enumeration_failure_is_fatal_and_does_not_mutate_jobs`). Full state-matrix tests remain open. No real writer or Windows validation is claimed.

- **Continuation — non-delegated fixture and capability check:** Sidecar lifecycle
  tests `sidecar_internal_v1_hello_ready_and_shutdown_exits` and
  `sidecar_internal_v1_extract_then_cancel_aborts_cleanly` PASS 1/1 each on the
  current dirty source. They cover the protocol fixture only. Environment detected:
  Linux x86_64 WSL2, user `shiraishi`, membership `0::/non-systemd`, cgroup v2
  mounted, `systemd-run` available. The exact production registered-writer fixture
  is delegated-cgroup dependent; previous recorded probes did not establish a
  usable delegated child. No new systemd/cgroup operation was run. Registered-writer
  acceptance remains NOT_RUN/BLOCKED for missing usable disposable delegation;
  no product defect is inferred and no system delegation was changed.
- **Current cursor reconciliation:** This ledger's earlier 2026-11-03 entries
  describe prior stages and are historical snapshots, not the current next task.
  The current source review prioritizes startup recovery classification,
  single-path dispatch, and complete error/report propagation. Historical
  targeted startup convergence/idempotence and Storage replay-idempotence logs
  passed; reuse them only while the tested source/dependencies are unchanged.
- A bounded user-level delegated-service launch was previously authorized and
  succeeded, but it did not establish registered-writer containment acceptance.
  The real writer fixture has not reached its expected durable terminal state;
  root cause and exact failing boundary remain undetermined. Improve diagnostics
  and reproduce within that disposable scope before changing product code.
- The legacy Storage repeat-recovery invocation previously recorded as FAIL
  0/1 is superseded by `/tmp/ig-storage-replay-binding2.log`, which records the
  prepared-replay/terminal-convergence idempotence test PASS 1/1 and
  `EXIT_CODE=0`. Preserve the earlier failure as historical evidence; the test
  result is reusable only if the current source matches its tested source.
- Next: finish Documentation Gate, then implement/review typed startup outcomes
  and fallible candidate enumeration (`CROSS_PLATFORM_CHANGE_REQUIRED`), followed
  by real writer fixture diagnosis. No product tests were run in this
  documentation-only continuation. aria2 remains deferred/NOT_RUN; Windows
  native validation remains Windows Owner work after Git handoff.

- **Latest cursor — 2026-11-03:** Reconciled against branch
  `cross-platform/automatic-pairing-reconcile-20261002`, HEAD and upstream
  `1baefa345053204e0eb252213fdcdf4544494277`; dirty tree 15 modified tracked / 4
  untracked. Last `git diff --check` and docs audit PASS. Current execution order:
  inspect stale Storage repeat-recovery fixture's valid Workflow-permit setup
  without weakening authority; then resume remaining Sidecar V1-08 registration/
  retry checks. Keep V1-07 active registered-writer replacement and V1-03/B1 open;
  delegated-cgroup acceptance NOT_RUN, aria2 deferred, Windows-owned work pending.
- Current additional evidence is detailed in the active handoff: startup valid
  InternalV1 convergence/idempotence and live-session maintenance drain PASS on
  this dirty source; queued cancellation, commit arbitration, uncontained V1
  refusal and Storage commit-boundary cancellation PASS. Storage's direct
  repeat-recovery unit remains FAIL 0/1, exit 101 because it calls the InternalV1
  Storage path without the now-required Workflow permit. Preserve this FAIL and
  inspect whether a legitimate authorized Workflow/adapter fixture exists; do
  not add a public permit constructor or treat a Storage-only substitute as
  production crash replay.
- **Continuation result — 2026-11-03:** Workflow lib tests PASS 48/48, exit 0
  (`/tmp/tw2tg-current/workflow-lib.log` and `.exit`), including failed-drain retry
  with admission remaining closed. This is component evidence only. The Storage
  repeat-recovery test remains FAIL 0/1 because its legacy direct call cannot
  supply the now-required Workflow permit. No permit escape hatch was added; a
  Storage-only substitute would not establish production crash-replay evidence.
  Migrate that regression to a legitimate Workflow/adapter fixture before claiming
  it PASS. `cargo fmt --check -p xarchive-workflow` reports formatting diffs in
  existing dirty files; no formatting changes were applied. V1-08 production
  Sidecar registration/retry checks and V1-07 active registered-writer replacement
  remain open; delegated-cgroup acceptance remains NOT_RUN.

- **Latest continuation delta — Linux executable checks:** Bound to dirty
  `1baefa345053204e0eb252213fdcdf4544494277`; no commit/push. Storage replay and
  Desktop startup convergence latest PASS evidence is recorded in the active
  handoff. Additional exact Desktop tests PASS 1/1 each: uncontained InternalV1
  writer is rejected before publication (`/tmp/ig-desktop-uncontained-v1.log`),
  missing InternalV1 manifest fails closed (`/tmp/ig-startup-missing-v1.log`),
  unknown contract version fails closed
  (`/tmp/ig-startup-unknown-contract.log`), and malformed crashed-writer domain
  fails closed before recovery decisions
  (`/tmp/ig-startup-drain-failclosed.log`). Runtime recovery refusal logging
  regression PASS 1/1 (`/tmp/ig-runtime-recovery-observability.log`): application
  joins worker results, classifies refusal/contention, and does not log raw error
  strings. Workflow integration PASS 2/2 and Storage dispatch PASS 1/1 are in the
  handoff with logs. `git diff --check` and docs audit PASS after these edits.
- V1-05 targeted regressions PASS 1/1 each: cancel-before-PREPARED wins over
  execution failure (`/tmp/ig-cancel-before-prepare.log`), stale attempt success
  records no completion/final state
  (`/tmp/ig-stale-attempt-success-fence.log`), and running cancellation fences a
  late success (`/tmp/ig-running-cancel-late-success.log`). Runtime transport
  generation replacement PASS 1/1 (`/tmp/ig-runtime-replacement-transport.log`)
  is only transport lifecycle evidence, not V1-07 active registered-writer
  continuity. Pending-finalizer incomplete-drain/no-journal and PREPARED retention
  regressions also PASS 1/1 each in logs listed in the active handoff.
- **V1-03/B1 remains open:** Plan requires assessing a temporary SQLite trigger
  before adding a hook. The production Storage publish path has a post-move tweet
  metadata UPDATE suitable as a trigger target, but application/runtime does not
  expose the runner's active SQLite connection for installing a trigger, and the
  full registered-writer runner test is explicitly ignored unless run inside an
  authorized disposable delegated cgroup. No delegation operation was performed
  or authorized in this batch; do not substitute Storage-only failure evidence.
  No production failpoint or API was added. Follow-up must be authorized runner
  capability plus a production-faithful trigger/test setup, or a narrowly scoped
  reviewed seam if the trigger route is demonstrated infeasible.
- **Execution cursor update — 2026-11-03:** Rechecked active-writer replacement
  prerequisites: the transport-generation test does not exercise a live
  registered writer, and the production runner acceptance remains delegated-
  cgroup gated. Sidecar hello/ready/shutdown and extract/cancel-abort each PASS
  1/1 (`/tmp/ig-sidecar-hello-shutdown.log`,
  `/tmp/ig-sidecar-extract-cancel.log`). Workflow owner-retention, failed-drain
  retry, registered-role requirement and admission-close/later-verified-drain
  component checks each PASS 1/1 (logs named in the active handoff); these do
  not prove production active-writer continuity. Wrong initial filters selected
  zero and are excluded. Next re-evaluate the V1-07 gate against the plan/source
  and run only remaining non-delegated production-path checks. Unknown-contract
  and missing-manifest startup fail-closed boundaries each PASS 1/1 on the
  current dirty source (`/tmp/ig-startup-unknown-contract-recheck.log`,
  `/tmp/ig-startup-missing-manifest-recheck.log`). Capability re-probe found
  Linux x86_64 WSL2-like workspace, `systemctl --user` responds, but
  `/proc/self/cgroup` is `0::/non-systemd` and `aria2c` is absent from PATH; no
  delegation operation was attempted or authorized. Keep active
  registered-writer continuity, V1-03/B1 and delegated-cgroup acceptance open;
  aria2 remains deferred and Windows-native work Windows-owned.
- Additional non-delegated production recovery checks PASS 1/1 each, exit 0:
  crashed writer-domain drain fails closed before decisions
  (`/tmp/ig-startup-crashed-writer-domain-drain.log`), queued-job completion
  fence (`/tmp/ig-persisted-completion-queued-fence.log`), recovery decision
  matrix (`/tmp/ig-commit-recovery-decision-matrix.log`), mixed recovery action
  event-order isolation (`/tmp/ig-recover-mixed-actions-event-order.log`),
  valid InternalV1 startup convergence/idempotence
  (`/tmp/ig-startup-valid-internal-convergence-recheck.log`), and V2 missing
  journal fail-closed (`/tmp/ig-startup-v2-missing-journal.log`). These do not
  substitute for production registered-writer replacement or delegated recovery.
- **V1-08 follow-up — 2026-11-03:** Current-source non-delegated rechecks also
  PASS 1/1, exit 0 for uncontained production runner refusal
  (`/tmp/ig-production-uncontained-v1-refusal-recheck.log`), commit-started and
  commit-winning persisted cancellation (`/tmp/ig-persisted-cancel-commit-started.log`,
  `/tmp/ig-persisted-cancel-commit-race.log`), queued cancellation idempotence
  (`/tmp/ig-persisted-cancel-queued-idempotent.log`), and Storage commit-boundary
  cancellation arbitration (`/tmp/ig-storage-cancel-commit-boundary.log`). The
  short-name `--exact` attempt selected zero and is excluded. A separate Storage
  repeat-recovery test fails 0/1, exit 101 because its legacy direct call now
  requires a Workflow permit (`/tmp/ig-storage-internal-recovery-repeat-idempotent.log`);
  treat as stale fixture/API use, do not bypass permit enforcement. V1-08 Sidecar
  evidence remains partial; V1-01/02/03/04/07 and other applicable gaps stay open.

- Source identity: branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD `1baefa345053204e0eb252213fdcdf4544494277`, dirty working tree; no commit/push. The previously noted `desktop/src-tauri/job-1.lock` is absent in this workspace and was not recreated.
- PASS: Current continuation reran Workflow existing-attempt integration tests 2/2 (exit 0, `/tmp/ig-workflow-integration-current.log`), Storage dispatch 1/1 (exit 0, `/tmp/ig-storage-dispatch-current2.log`), Storage PREPARED replay/convergence 1/1 (`/tmp/ig-storage-replay-binding2.log`), and Desktop startup convergence 1/1 (`/tmp/ig-desktop-startup-fixed-fixture2.log`). Desktop startup fixture now uses attempt-scoped `attempt-1` staging and joins/asserts the recovery result before checking terminal state.
- Historical FAIL reconciled: the earlier Storage replay failure was followed by a fixture assertion correction (`Downloaded` after replay); latest exact run passes. Earlier Desktop convergence failure was diagnosed: fixture created legacy `job_id` staging while the recovery path correctly requires attempt-scoped staging. Correct fixture and terminal/idempotence checks pass; no production recovery logic was changed for that mismatch.
- Startup V1 enumeration errors and per-job `Ok(false)` are now surfaced in the returned startup outcomes. The generic recovery candidate API still masks its Storage query errors because its trait returns `Vec`; changing that trait is separate shared-contract work.
- `cargo fmt --check` reports extensive existing formatting drift across dirty sources; no broad formatting rewrite was applied. Delegated-cgroup acceptance remains NOT_RUN; no delegated operation was attempted. aria2 remains user-deferred/NOT_RUN, and no Windows acceptance was performed.
- The earlier executor module run (86 passed, 1 failed, 8 ignored) predates the fixture correction and is not a current module result. Full executor module suite was not rerun in this batch.

## Reconciliation — 2026-11-02 Act continuation

- Source: dirty branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `1baefa345053204e0eb252213fdcdf4544494277`; current tree includes
  pre-existing tracked modifications and untracked fixtures/docs. Preserve them.
- The active platform handoff contained unsupported claims that the Storage
  fixtures compiled and Workflow tests were fixed. Read-only source inspection
  found the Storage integration fixture has no tests and calls `process::exit(0)`;
  Storage module tests use incomplete direct SQL setup and unverified API calls;
  the Workflow test does not bind an existing attempt. Those claims are
  superseded, not retroactively converted into test failures or passes.
- Current tool integration could not reliably capture command completion/exit
  status. No Cargo result from this continuation is recorded as PASS or FAIL.
  Establish a reliable evidence path before changing code in response to a build
  failure. No delegated-cgroup operation was authorized or performed.
- Static source review identifies destination-path authorization inconsistency
  and missing explicit terminal completion after PREPARED replay; both require
  targeted reproduction/tests. See the active platform handoff.

## Act continuation baseline — 2026-11-01

> Historical snapshot. The user subsequently approved the trusted drain-evidence
> boundary on 2026-11-02; consult the active handoff for current status.

> Historical snapshot. The active cursor in
> [`platform-handoff.md`](../status/platform-handoff.md) supersedes all status
> claims below; older failure states and source inventories remain historical.

> Historical snapshot. The latest execution cursor is in
> [`docs/status/platform-handoff.md`](../status/platform-handoff.md). Results
> below must not be treated as current without matching source and rerun evidence.

- At read-only start, source was branch
  `cross-platform/automatic-pairing-reconcile-20261002`, HEAD
  `1baefa345053204e0eb252213fdcdf4544494277`, equal to the local upstream ref
  (no fetch). Nine tracked paths were modified and two plan/proposal documents
  were untracked; preserve all. Initial `git diff --check` passed.
- No tests have been rerun yet in this Act continuation. Previously recorded
  Workflow 41/41, coordination subset 8/8, Storage 172/172, Sidecar Python
  101/101, and supervisor 17/17 are historical dirty-source-bound component
  results, not evidence for the present batch. The pending-finalizer build
  failure recorded below likewise must be reproduced before repair; the old
  `/tmp` logs do not establish current-source results.
- The current source audit confirms `register_linux` is narrowed to
  `pub(crate)` while Desktop calls it cross-crate. Restore only the existing
  public registration method after the targeted build confirms E0624; preserve
  its role/attempt/containment/admission checks. This ordinary build integration
  fix does not mint drain evidence or authorize recovery.
- Recovery evidence minting was an unapproved shared-contract proposal at this
  historical snapshot. The user approved the narrow boundary on 2026-11-02;
  implementation and dependent crash-replay validation remain outstanding.
  Delegated-cgroup create/drain/remove and the ignored
  production fixture remain `NOT_RUN`; no cgroup operation is authorized or
  performed in this batch. The inline protocol worker fixture must not be
  conflated with complete real-Sidecar V1-08 acceptance. aria2 remains deferred.

### Act continuation results — 2026-11-01 (dirty source)

- Reproduced the Desktop library check failure before repair:
  `timeout 180s cargo check -p xarchive-desktop --lib` — FAIL, exit 101,
  E0624 at `desktop/src-tauri/src/executor.rs:1117`; the cross-crate call to
  `WriterRegistration::register_linux` was rejected because the method was
  narrowed to `pub(crate)`.
- Restored the existing `pub` visibility only, preserving the method's
  attempt/role/containment/admission checks. This is a shared normal-path
  registration API repair; it does not implement recovery or mint drain proof.
- Post-fix `timeout 180s cargo check -p xarchive-desktop --lib` PASS, exit 0.
  Exact `archive::completion_tests::pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`
  PASS 1/1 and exact
  `executor::tests::production_runner_rejects_uncontained_internal_v1_writer_before_publish`
  PASS 1/1 (both exit 0). Logs:
  `/tmp/tw2tg-act-20261101-desktop-check-before.log`,
  `/tmp/tw2tg-act-20261101-desktop-check-after.log`,
  `/tmp/tw2tg-act-20261101-pending.log`,
  `/tmp/tw2tg-act-20261101-uncontained.log`.
- These results are bound to the dirty post-fix source at the original HEAD;
  they do not imply full Desktop, production delegated-cgroup fixture,
  production recovery, real-Sidecar lifecycle, Sidecar phase, or Windows
  acceptance. `node scripts/docs-audit.mjs` and `git diff --check` pass after
  documentation updates. No cgroup operation was performed.

## Active-source validation update — 2026-11-01

### Independent continuation checks — 2026-11-01 (dirty source)

- Source remains branch `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD `1baefa345053204e0eb252213fdcdf4544494277`; see the current platform
  handoff for the complete dirty-file inventory. WSL2 membership is
  `/non-systemd`; user systemd is running. No delegated cgroup operation was
  authorized or run, and the ignored production Sidecar fixture remains
  `NOT_RUN`.
- `timeout 120s cargo test -p xarchive-workflow --lib -- --test-threads=1`
  PASS 41/41 (exit 0); `timeout 120s cargo test -p xarchive-workflow --lib
  coordination -- --test-threads=1` PASS 8/8 (exit 0). The latter is a subset
  of the former, not additional independent coverage.
- `timeout 180s cargo test -p xarchive-storage --lib -- --test-threads=1`
  PASS 172/172 (exit 0); `timeout 120s cargo test -p
  xarchive-sidecar-supervisor --lib -- --test-threads=1` PASS 17/17 (exit 0);
  `python -m pytest sidecar/tests -q` PASS 101/101 (exit 0). These are
  component checks and do not prove Desktop production writer registration,
  drain, publication, recovery, or real-tool compatibility.
- Attempted exact current-source pending-finalizer test:
  `timeout 180s cargo test -p xarchive-desktop --lib
  archive::completion_tests::pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal
  -- --exact --nocapture --test-threads=1` — build FAIL, exit 101 before test
  execution. `E0624`: dirty `crates/xarchive-workflow/src/writer_lifecycle.rs`
  changes `WriterRegistration::register_linux` from HEAD's `pub` to
  `pub(crate)`, while Desktop `executor.rs` calls it across the crate boundary.
  No product/test change was made in response; determine ownership and resolve
  this dirty-source API mismatch before further Desktop test execution. This is
  a build/integration failure, not a failed test assertion and not recovery
  evidence.
- The approved Option B implementation plan remains gated on the separate
  drain-evidence minting decision; the Act-mode continuation did not approve it.
  This batch records the failure and continues independent passing checks; it
  does not mark V1-02, V1-03/V1-04 or the Sidecar phase complete. No Windows
  validation was run.

## Historical active-source validation update — 2026-10-31

### Resumption reconciliation — 2026-10-31 (current workspace)

- Source HEAD/upstream remain
  `1baefa345053204e0eb252213fdcdf4544494277`; current dirty state is eleven
  modified tracked files plus the untracked completion/refactor plan. See the
  current platform handoff for the exact ownership and dependency matrix.
- Current source inspection found that startup recovery's InternalV1 PREPARED
  branch invokes Storage recovery outside a live runner session and performs
  terminal persistence outside the session as well. Storage documents live
  session and writer-drain authority as a caller precondition. Therefore a
  direct Storage replay test would not prove the acceptance target. This is
  `CROSS_PLATFORM_CHANGE_REQUIRED`: first route V1 replay through runner
  admission, the existing attempt's session/scopes, registered-writer drain,
  and terminal finalization under that owner; then add the deterministic
  post-directory-commit DB failure and same-attempt restart/replay assertion.
- No Storage seam or failpoint is present on this source; a briefly explored
  Storage edit was restored byte-for-byte to HEAD. No V1-03/V1-04 acceptance is
  claimed from this resumption.
- B5 active registered-writer continuity remains a separate executable gate
  requiring a real claimed session, registered Sidecar writer and pause inside
  its registered lifetime. Real aria2 remains NOT_RUN because the executable
  and delegated fixture prerequisites are unavailable. Windows-native checks
  remain Windows Owner work.
- Module regression checks are recorded in the following resumption section;
  that section supersedes the earlier checkpoint's not-yet-run statement.

### Latest continuation verification — 2026-10-31

- Rechecked the prior executor fixture correction against current dirty source:
  both exact mixed recovery tests PASS 1/1, executor module PASS 84/84 with 8
  ignored, and `cargo check -p xarchive-desktop --lib` PASS, each with captured
  exit code 0. `node scripts/docs-audit.mjs` also PASS exit 0. Logs are recorded
  in `../status/platform-handoff.md`; the earlier 82-pass/2-fail run remains
  historical failure evidence, followed by the corrected, passing rerun.
- Reconciled the competing cursor notes: V1-03's failure seam is downstream of
  the V1-04 production-recovery lifecycle prerequisite. A fixture-only Storage
  seam cannot establish that recovery runs under the original attempt session
  and drained writer owner. The next implementation is the narrow
  runner-owned InternalV1 PREPARED replay operation; only after that path exists
  should deterministic post-directory-commit DB failure/replay be added.
- Existing `recover_v2_on_runner` is evidence for admission and original-attempt
  binding, not a template to bypass InternalV1's publication-owner registration
  and drain contract. Generic `SessionExecutionBoundary::execute_session`
  claims a new execution and starts a new factory writer, so it is not valid for
  PREPARED replay. Storage's `recover_internal_archive_attempt` API documents
  caller-established session/drain authorization but takes no permit itself;
  therefore do not forge an empty publication owner for an already-published
  archive. The recovery authorization contract must be resolved narrowly before
  implementation. No production or Storage code changed in this continuation;
  V1-03/V1-04 remain unaccepted. B5, aria2, and Windows checks remain open as
  recorded in the active cursor.

### Resumption module validation — 2026-10-31

- `timeout 180s cargo test -p xarchive-storage --lib` PASS 172/172;
  `timeout 120s cargo test -p xarchive-workflow --lib` PASS 40/40;
  `timeout 180s cargo test -p xarchive-desktop --lib archive::completion_tests::
  -- --test-threads=1` PASS 7/7; exact
  `executor::tests::session_execution_error_preserves_prepared_job_for_startup_replay`
  PASS 1/1; `timeout 180s cargo check -p xarchive-desktop --lib` PASS. All
  commands completed with exit 0 on dirty source based on
  `1baefa345053204e0eb252213fdcdf4544494277`.
- `timeout 180s cargo test -p xarchive-desktop --lib executor::tests::
  -- --test-threads=1` FAIL, exit 101: 82 passed, 2 failed, 8 ignored. The
  failures are
  `recover_commits_persisted_mixes_actions_and_preserves_each_job_event_order`
  (`commit recovery facts do not match persisted Job snapshot`) and
  `storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution`
  (`invalid job transition: Downloading -> Complete`). The first fixture reports
  `Downloaded` facts for a snapshot that is not Downloaded; the second fixture
  attempts a direct `QUEUED -> COMPLETE` persistence transition. They are generic
  commit-recovery fixture failures, not V1 replay acceptance. Preserve them as
  FAIL evidence; assess a separate test-fixture correction before claiming the
  executor module green. No Windows or real aria2 result follows from these runs.
- The pre-existing `archive.rs` imports are used only by test code, which explains
  build warnings in both lib/test targets; cleanup was reverted to avoid adding
  unrelated source scope. No Storage or executor source diff remains.

### Continuation reconciliation — 2026-10-31

> Historical checkpoint: the immediately preceding resumption sections contain
> the current workspace and module-validation results; earlier file-count and
> validation statements below are preserved only as revision-bound history.

- Git remains on `cross-platform/automatic-pairing-reconcile-20261002`,
  HEAD/origin `1baefa345053204e0eb252213fdcdf4544494277`, with six tracked
-  modifications and no untracked files. The four recorded targeted log exit
  codes remain 0.
- Correct the prior continuation's delegated-runner characterization: the actual
  latest retry entered the ignored test and failed at the fixture assertion
  `protocol fixture did not reach barrier` (exit 101, unit
  `run-p204192-i205586.service`, invocation
  `dd7d832e56cd4633ae8ecd0a4266d523`). It is not a cgroup prerequisite result
  and is not product acceptance.
- Inspection confirmed no protocol-barrier/trigger source edit remains. An
  intermediate barrier edit accidentally targeted an adjacent v2 fixture; it was
  removed, and `executor.rs` has no final diff. An earlier `--no-run` compile
  passed while that temporary edit existed, but it is not current-source evidence.
  Do not count either as B1/B2 progress. The next safe action is to trace the
  source tracing located the deterministic InternalV1 failure window at
  `commit_attempt_staging` → `update_tweet_metadata`, after destination commit
  but before metadata DB finalization. The proposed Python `-c` barrier was invalid
  because `__file__` is not a fixture path and its argv cannot be extended through
  the production args boundary; its edits were discarded. `executor.rs` has no
  diff. No barrier baseline or SQLite injection is claimed. Next add a bounded,
  fixture-only Storage seam at the identified boundary, then verify the real
  runner retains PREPARED state and replays the same attempt.

### Latest execution reconciliation — same dirty source, 2026-10-31

- **Later continuation, revalidated against the live workspace:** Source is still
  branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD/origin
  `1baefa345053204e0eb252213fdcdf4544494277`, seven modified tracked files, no
  untracked files; Linux x86_64 WSL2, cgroup v2, user systemd running, `/proc/self/cgroup`
  reports `/non-systemd`, `aria2c` absent. The test source no longer contains the
  previously suspected orphan lines; it matches the clean `drop(runtime)` block.
  This correction supersedes the earlier false statement that the malformed
  terminal-test block remained present.
- **Current-source targeted checks:**
  `archive::completion_tests::pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`
  PASS 1/1, exit 0, log `/tmp/tw2tg-20261031-pending-finalizer-targeted.log`;
  `executor::tests::session_execution_error_preserves_prepared_job_for_startup_replay`
  PASS 1/1, exit 0, log `/tmp/tw2tg-20261031-prepared-replay-targeted.log`;
  `cargo check -p xarchive-desktop --lib` PASS, exit 0, log
  `/tmp/tw2tg-20261031-desktop-lib-check.log`; `runtime::tests::` PASS 10/10,
  exit 0, log `/tmp/tw2tg-20261031-runtime-replace-targeted.log`. Runtime
  replacement tests do not prove continuity of an active registered archive
  writer across replacement.
- **Delegated retry:** Current-source exact ignored production runner attempt
  returned exit 101 before entering the test behavior: fixture provisioning
  rejected the user systemd cgroup as `Linux execution cgroup delegation
  unavailable`. A separate bounded delegated-unit probe likewise returned 1;
  its child lacked usable cgroup delegation. Record this as an environment
  prerequisite failure, not a product result. Earlier unit PASS logs are tied
  to prior source identities and cannot replace current-source acceptance.
- **Continuity correction:** The prior paragraph's “malformed block remains”
  claim was wrong; current `executor.rs` has the intact `drop(runtime)` line.
  Do not report a source repair or attribute any current diff to that claim.

- **Continuation after the previous cursor (2026-10-31):** On branch
  `cross-platform/automatic-pairing-reconcile-20261002`, HEAD/origin
  `1baefa345053204e0eb252213fdcdf4544494277`, seven tracked modifications, no
  untracked files; Linux x86_64 WSL2, systemd user manager available, `aria2c`
  absent. The exact ignored test
  `executor::tests::startup_recovery_drains_live_descendant_after_owner_sigkill`
  PASS 1/1 under disposable delegated unit `run-p192306-i193597.service`
  (invocation `7db6a0e99ad24cb3b9a4650315012396`), command exit 0; 362 filtered,
  9.16s. It verified the descendant remained populated after owner SIGKILL until
  startup recovery drained it, and no late marker write occurred. The temporary
  fixture root/marker were absent in the post-run `/tmp` scan; systemd reported
  `inactive/dead`, `Result=success`. With `--collect`, the cgroup was removed
  before a direct post-run `cgroup.events` observation could be made.
- **Failure-window exploration (not an acceptance result):** InternalV1 performs
  the destination directory commit before DB finalization, so a fixture-local
  SQLite abort trigger can target the real transaction without a production hook.
  The current ignored runner fixture starts asynchronous dispatch in
  `submit_persisted`; installing the trigger afterwards races the protocol fixture.
  `recover_startup()` is not a safe retry for an ordinary in-flight DOWNLOADING
  job. No code change or test hook was retained. Add a deterministic protocol
  barrier before retrying this injection; only then assess same-attempt replay.
- **Tooling/diff note:** An attempted whole-file `rustfmt` touched both pre-existing
  dirty Rust files. There was no saved pre-format byte snapshot, so those formatting
  hunks were not reverted where they could not be distinguished safely from prior
  uncommitted edits. One malformed terminal-test hunk introduced while attempting
  a broad patch was identified against HEAD and repaired exactly; no test result
  is claimed for it. Review the final Rust diffs carefully before any commit.

- **Source/capability:** branch
  `cross-platform/automatic-pairing-reconcile-20261002`, HEAD/origin
  `1baefa345053204e0eb252213fdcdf4544494277`; seven tracked modified files,
  no untracked files. Linux x86_64 WSL2; `systemd-run` present, `aria2c` absent.
- **Module evidence:** `timeout 180s cargo test -p xarchive-desktop --lib
  executor::tests:: -- --test-threads=1` — 84 passed, 0 failed, 8 ignored,
  271 filtered out, exit 0. The ignored set includes delegated cgroup, real aria2,
  and intentionally deferred production V2 checks; ignored cases are not PASS.
  `batch::tests::` — 12 passed, 0 failed, 0 ignored. `archive::completion_tests::`
  — 7 passed, 0 failed, 0 ignored.
- **Exact follow-up tests:**
  `startup_recovery_classifies_legacy_without_recovery_facts_as_incomplete`,
  `startup_recovery_classifies_legacy_without_execution_spec_consistently`,
  `persisted_recovery_records_the_actual_source_state_for_queued_and_interrupted_jobs`,
  `maintenance_blocks_production_and_startup_recovery_before_mutation`,
  `persisted_recovery_updates_only_recoverable_candidates`,
  `batch::tests::failed_batch_retry_resumes_to_attempt_owned_success`, and
  `startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`
  each PASS 1/1 with fully qualified exact filters and bounded timeouts.
- **Production runner:** ignored
  `executor::tests::production_runner_finalizes_internal_v1_archive_after_registered_writer_drain`
  PASS 1/1 under
  `timeout 120s systemd-run --user --wait --pipe --collect --property=Delegate=yes
  --working-directory='/home/shiraishi/VSCode Workspace/Tw2Tg'`; disposable unit
  `run-p186531-i187788.service`, exit 0. Scope remains contained publication,
  uncontained refusal and PREPARED destination collision only.
- **Tool capture note:** An initial chained exact-test call printed all four tests
  PASS 1/1 but terminal integration did not expose successful command completion.
  A bounded retry's shell printed exit 0 but terminal closed and tool returned an
  error. The legacy recovery-facts test was then rerun through the background
  command channel and completed exit 0; the other three exact tests ran separately
  with observed successful tool completion. The initial tool-capture errors are
  not product failures and are not counted as command PASS.
- **Not run / next executable:**
  `executor::tests::startup_recovery_drains_live_descendant_after_owner_sigkill`
  is ignored for disposable delegated cgroup, not aria2. `systemd-run` is present;
  attempt it as the next delegated acceptance. Real aria2 remains NOT_RUN because
  `aria2c` is absent. Full Desktop/workspace regression was not run.
- **Identity scope correction:** the pending-finalizer identity test includes a
  fixture-supplied `ClaimOutcome::Claimed(attempt 2)` against pending attempt 1.
  It verifies component-level identity rejection and no mutation; it is not a
  durable-claim production result and does not prove
  `PublicationOwner::authorize` permit mismatch.

- **Continuation retest — 2026-10-31:** Source remains dirty at base
  `1baefa345053204e0eb252213fdcdf4544494277`, branch
  `cross-platform/automatic-pairing-reconcile-20261002` (Linux x86_64 WSL2).
  Exact Desktop tests PASS 1/1 each:
  `executor::tests::recover_commits_persisted_mixes_actions_and_preserves_each_job_event_order`,
  `executor::tests::storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution`,
  `executor::tests::startup_recovery_drains_crashed_writer_domains_before_decisions`,
  `executor::tests::startup_recovery_fails_closed_when_internal_manifest_is_missing`,
  `executor::tests::startup_recovery_fails_closed_on_unknown_contract_version`,
  `executor::tests::session_execution_error_preserves_prepared_job_for_startup_replay`,
  and `executor::tests::apply_commit_recovery_skips_complete_job_without_duplicate_event`.
  The ignored permit-bearing production runner
  `executor::tests::production_runner_finalizes_internal_v1_archive_after_registered_writer_drain`
  was run under `timeout 120s systemd-run --user --wait --pipe --collect
  --property=Delegate=yes --working-directory='/home/shiraishi/VSCode Workspace/Tw2Tg'`:
  PASS 1/1, unit `run-p176571-i177789.service`, exit 0. Its acceptance scope is
  contained publication, uncontained refusal and PREPARED collision only; it does
  not prove B1 permit mismatch, forward post-directory DB failure/replay, B2
  terminal failure/panic/Drop, real aria2, or active registered-writer replacement
  continuity. `aria2c` is absent. No approved production-faithful failure seam or
  active-writer replacement lifecycle setup is available. Full Desktop/workspace
  regression was not run because these are targeted retests and unresolved gates
  require the missing seam/tool/setup; Windows-native acceptance remains owned by
  the Windows Platform Owner. Cargo emitted existing unused-import/dead-code
  warnings; no test failure was observed.

- **Independent B5/source recheck (2026-10-31):** Exact
  `cargo test -p xarchive-desktop --lib
  runtime::tests::replacing_executor_restarts_transport_on_the_new_service_generation
  -- --exact --nocapture --test-threads=1` PASS 1/1, exit 0, 0.17s. Source
  review confirms replacement blocks while `shutdown_in_place` joins the runner,
  but this fixture does not create an active claimed session or registered writer;
  the result is not active-writer continuity evidence. `cargo test -p
  xarchive-storage --lib` PASS 172/172, exit 0, 4.67s. The new independent
  checks are on the same dirty source SHA above; no product source changed in this
  round. `git diff --check` and `node scripts/docs-audit.mjs` PASS.

- **B3 lifecycle targeted follow-up — 2026-10-31:** On the same dirty source,
  `executor::session_boundary_tests::archive_writer_drain_failure_is_terminal_and_never_completes_archive`,
  `executor::tests::production_factory_retains_a_real_stuck_cleanup_owner_and_resolve_aborts`,
  and `archive::completion_tests::pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`
  each PASS 1/1 with exact filters. An initial wrong module-qualified filter
  selected 0 tests for the drain-failure case; `cargo test -p xarchive-desktop
  --lib -- --list` confirmed its module, then the corrected exact test passed 1/1.
  The zero-test invocation is not evidence. This does not prove registered aria2
  drain/retry (`aria2c` absent) or active-writer executor-replacement continuity.

- **Shared writer and production-boundary follow-up — 2026-10-31:** Workflow
  exact tests `writer_lifecycle::tests::failed_drain_never_proves_exit_and_retry_keeps_admission_closed`,
  `writer_lifecycle::tests::production_drain_update_requires_a_registered_role`,
  and `writer_lifecycle::tests::admission_close_does_not_erase_a_later_verified_drain`
  each PASS 1/1. Desktop exact tests
  `executor::attempt_writer_domain_tests::predecessor_recovery_skips_the_first_attempt`,
  `executor::attempt_writer_domain_tests::predecessor_recovery_fails_closed_on_a_malformed_record`,
  `executor::tests::startup_redispatch_binds_attempt_before_successful_completion`,
  and `executor::tests::linux_launch_descriptor_is_explicit_bounded_and_fail_closed`
  each PASS 1/1. Initial module-missing filters selected 0 tests for the two
  predecessor cases and all three workflow cases; test listing established fully
  qualified paths, and only corrected exact runs are evidence. These checks do not
  establish registered aria2 retry or active-writer executor replacement.

- **B6 contract gate — 2026-10-31:** Source inspection of
  `PendingArchivePublication::finalize` and
  `ArchiveService::publish_internal_v1_attempt` confirms the live InternalV1
  request carries `SidecarArchiveRequest`, attempt count and permit, with no
  export payload. The active Plan explicitly defers D1/F2/F3 reconciliation until
  C1 acceptance. No output-contract change or D1 activation was made; treat B6 as
  prerequisite-waiting, then reconcile the shared export payload contract before
  testing the JSON/YAML output combinations.

- **Mixed recovery fixture continuation:** On dirty source based on
  `1baefa345053204e0eb252213fdcdf4544494277` (Linux x86_64 WSL2), reproduced the
  Storage test's setup error (`Downloading -> Complete`) and corrected its V2
  fixture to reach `Downloaded` before preparing the already-terminal snapshot.
  Also corrected the adjacent InMemory mixed-recovery fixture so its persisted
  snapshots agree with the provider's `Downloaded` facts. Exact commands:
  `cargo test -p xarchive-desktop --lib executor::tests::recover_commits_persisted_mixes_actions_and_preserves_each_job_event_order -- --exact`
  and
  `cargo test -p xarchive-desktop --lib executor::tests::storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution -- --exact`;
  both PASS 1/1 after correction (each initially reproduced a fixture failure).
  This is fixture-state validation only; it does not prove forward DB-failure
  replay, B1/B2 injection, registered-writer replacement continuity, real aria2,
  or Windows behavior. `git diff --check` PASS after the change.

- **Earlier validation source snapshot:** commit
  `1baefa345053204e0eb252213fdcdf4544494277`, branch
  `cross-platform/automatic-pairing-reconcile-20261002`, Linux x86_64 WSL2.
  This result block is historical; its clean/dirty working-tree wording is not
  the current cursor. The current dirty-tree scope is recorded in the handoff
  above; older results remain bound to their original source and execution.
- **Storage:** `cargo test -p xarchive-storage --lib` PASS 172/172 (exit 0).
- **Desktop targeted:**
  `executor::tests::startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`
  PASS 1/1;
  `executor::tests::session_execution_error_preserves_prepared_job_for_startup_replay`
  PASS 1/1;
  `archive::completion_tests::pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`
  PASS 1/1.
- **Delegated production runner:**
  `executor::tests::production_runner_finalizes_internal_v1_archive_after_registered_writer_drain`
  PASS 1/1, invoked with
  `timeout 120s systemd-run --user --wait --pipe --collect --property=Delegate=yes`
  around the exact Cargo test command. Transient unit `run-p142132-i143202.service`,
  invocation `3321ea3e488f4b4d9c22f9ae2f5cce3c`, exited 0; test completed in
  0.33s. It covers contained publication, uncontained fail-closed refusal before
  PREPARED, and prepared destination collision. It does not cover permit mismatch,
  post-directory-commit SQL failure, panic/Drop, or real aria2.
- **Environment / limits:** direct shell membership was `0::/non-systemd`, with
  no delegated write access there; the approved disposable user-systemd delegated
  scope successfully supplied the test's cgroup parent. `/usr/bin/aria2c` is
  absent, so real-aria2 validation is NOT_RUN. Desktop test compilation reports
  unused imports and dead-code warnings; no test failure.
- **Next:** extend production-authority mismatch and post-directory DB failure/replay
  coverage without test-minted permits or weakened drain criteria; continue B2
  terminal-failure/panic/Drop. Keep B1/B2/C1 open and Production V2 fail-closed.

### Continuation delta — 2026-10-31

- **Source:** dirty working tree based on
  `1baefa345053204e0eb252213fdcdf4544494277` on
  `cross-platform/automatic-pairing-reconcile-20261002`; no commit or push.
- **Pending-finalizer identity regression:** Extended the existing targeted test
  with a real attempt-2 session against attempt-1 pending publication. The
  finalizer returns `PUBLICATION_IDENTITY_MISMATCH`; the Storage and Sidecar
  owners remain held, and no V1 journal or destination is created. Exact test
  PASS 1/1. This does not exercise `PublicationOwner::authorize` permit mismatch
  branches and does not satisfy the delegated production-runner requirement.
- **Forward V1 failpoint experiment:** A proposed Storage-local `#[cfg(test)]`
  seam is not available in Desktop tests because Cargo compiles Storage as a
  normal dependency there. It was fully removed; no production or test failpoint
  is claimed. Do not substitute the existing V2 mechanics crash tests or recovery
  rollback tests for forward InternalV1 post-directory-commit DB failure/replay.
- **Remaining:** InternalV1 runner attempt/destination permit mismatch,
  post-directory-commit DB failure/replay, B2 terminal failure/panic/Drop,
  real aria2 (NOT_RUN; executable absent), and Windows-owned validation remain
  open. Production V2 remains fail-closed.
- **Continuation reconciliation:** Re-ran the pending-finalizer identity test on
  this dirty source: PASS 1/1. Invoking the ignored delegated runner outside its
  disposable delegated systemd scope selected 0 tests and provides no evidence.
  Source inspection confirms permit attempt/destination are session-derived and
  finalizer identity is checked before authorization; the current registered-
  writer fixture has no faithful mismatch injection. Terminal transaction
  failure/panic likewise lacks a seam combining concrete production persistence
  with the real registered-writer authority. No API or authorization change was
  made; these remain open. `cargo fmt --all -- --check` still fails on broad
  unrelated/pre-existing Rust formatting diffs; no bulk formatting was applied.

### Continuation delta — production entry-point and recovery sweep — 2026-10-31

- **Source:** dirty working tree based on
  `1baefa345053204e0eb252213fdcdf4544494277`, branch
  `cross-platform/automatic-pairing-reconcile-20261002`; no commit/push.
- **Exact Desktop tests, all PASS 1/1:**
  `executor::tests::startup_redispatch_binds_attempt_before_successful_completion`,
  `batch::tests::failed_batch_candidate_can_redispatch_through_persisted_scheduler`,
  `executor::tests::startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`,
  `executor::tests::session_execution_error_preserves_prepared_job_for_startup_replay`,
  `executor::tests::startup_recovery_fails_closed_when_internal_manifest_is_missing`,
  `executor::tests::startup_recovery_fails_closed_on_unknown_contract_version`, and
  `executor::tests::startup_recovery_drains_crashed_writer_domains_before_decisions`.
  `cargo test -p xarchive-workflow --lib` PASS 40/40. These provide reusable
  redispatch, InternalV1 replay/idempotence, PREPARED preservation, fail-closed
  contract and startup drain-order evidence; they do not close permit mismatch,
  terminal DB failure/panic/Drop or real registered aria2 lifecycle.
- **Source audit:** only one production call to
  `publish_internal_v1_attempt` exists, in `PendingArchivePublication::finalize`.
  `recover_startup()` passes `false` to the v2-parameterized recovery helper.
  Executor replacement has a descriptor/configuration revalidation test, not an
  active registered-writer replacement test.
- **Environment:** `aria2c` is unavailable. Two production registered-writer
  tests are ignored unless a real aria2 binary and disposable delegated cgroup
  are supplied. Do not substitute Workflow fake-containment unit tests for that
  runner acceptance.
- **Executor replacement:** exact
  `runtime::tests::replacing_executor_restarts_transport_on_the_new_service_generation`
  and `executor::tests::linux_launch_descriptor_is_explicit_bounded_and_fail_closed`
  both PASS 1/1. They cover generation routing and descriptor/configuration
  revalidation, not an active registered-writer session crossing replacement.
- **Tooling:** an initial repository-wide symbol search indexed cached binary
  artifacts and returned non-source noise; subsequent scoped text-source search
  established the call-site count above. `rg` is not installed; grep/source reads
  were used instead. These are tooling facts, not product failures.

### Continuation delta — runner-boundary owner and recovery retest — 2026-10-31

- **Source:** current dirty tree based on `1baefa345053204e0eb252213fdcdf4544494277`,
  branch `cross-platform/automatic-pairing-reconcile-20261002`; six modified
  tracked files, no untracked files; Linux x86_64 WSL2.
- **B3 owner lifecycle:**
  `executor::session_boundary_tests::startup_cleanup_pending_owner_crosses_the_boundary_and_resolves_same_owner`
  PASS 1/1, and
  `executor::tests::production_factory_retains_a_real_stuck_cleanup_owner_and_resolve_aborts`
  PASS 1/1. The first proves same-owner cleanup retry across the session outcome
  boundary; the second proves retention of a real Sidecar cleanup owner and abort
  when cleanup remains stuck. These do not exercise aria2 registration or replace
  real writer-domain drain evidence.
- **B3/B5 repeat checks:** startup recovery/idempotence, persisted batch retry,
  crashed-writer startup drain ordering, and live-session maintenance admission
  each PASS 1/1. Workflow library tests PASS 40/40. One earlier exact filter used
  the wrong test module path and selected zero tests; the corrected full path above
  ran and passed. The zero-test invocation is not counted.
- **Additional lifecycle checks:**
  `runtime::tests::runtime_refuses_replacement_until_inflight_worker_exits` and
  `archive::completion_tests::pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`
  each PASS 1/1. The former covers Telegram worker replacement gating only, not
  executor replacement while an archive writer is registered. The latter covers
  cancellation, incomplete drain and pending/session identity rejection while
  retaining Storage/Sidecar owners; it does not cover permit authorization mismatch.
- **Remaining:** `aria2c` is absent; registered aria2 drain/retry stays NOT_RUN.
  Active-session executor replacement, V1 permit mismatch, forward post-directory
  DB failure/replay, and B2 terminal DB failure/panic/Drop remain open. Storage's
  existing after-directory-commit recovery tests do not inject a failure into the
  V1 forward publication call and are not substituted for that acceptance. No
  production code, authorization semantics, or test seam changed; production V2
  remains fail-closed.

### Continuation delta — delegated runner and terminal transaction checks — 2026-10-31

- **Delegated production runner:**
  `executor::tests::production_runner_finalizes_internal_v1_archive_after_registered_writer_drain`
  PASS 1/1 in approved disposable user-systemd delegated scope; unit
  `run-p156442-i157577.service`, invocation
  `f6422e76e8d34853982b93aa247eefbf`, exit 0. The initial retry omitted a
  working directory and Cargo exited 101 before locating the workspace; the
  successful retry explicitly set the workspace directory. Coverage remains
  contained publication, uncontained refusal, and PREPARED collision.
- **Terminal transaction contracts:**
  `executor::session_boundary_tests::complete_terminal_for_attempt_is_atomic`,
  `...::complete_terminal_for_attempt_is_idempotent`,
  `...::complete_terminal_for_attempt_rejects_stale_attempt`, and
  `...::complete_terminal_for_attempt_requires_downloaded_state` each PASS 1/1.
  These exercise transaction semantics, not terminal DB failure in the real
  registered-writer runner.
- **Additional retest:** Unknown-contract and missing-InternalV1-manifest startup
  fail-closed checks each PASS 1/1. The owner/recovery/lifecycle tests and Workflow
  40/40 listed immediately above were also rerun with explicit completion output.
- **Limits unchanged:** `aria2c` remains absent; B1 permit mismatch and forward
  post-directory DB-failure/replay, B2 runner terminal DB-failure/panic/Drop, and
  active registered-writer executor-replacement continuity remain open. The
  scoped runner success does not close them; no seam or production behavior was
  changed. Windows verification remains pending with its owner.

### Continuation delta — current-round Storage and B5 retest — 2026-10-31

- `cargo test -p xarchive-storage --lib`: PASS 172/172.
- Desktop exact tests for `runtime::tests::replacing_executor_restarts_transport_on_the_new_service_generation`,
  `executor::tests::linux_launch_descriptor_is_explicit_bounded_and_fail_closed`,
  `executor::tests::startup_redispatch_binds_attempt_before_successful_completion`,
  `executor::tests::session_execution_error_preserves_prepared_job_for_startup_replay`,
  and startup unknown-contract/missing-manifest refusal each PASS 1/1.
- The delegated InternalV1 production runner PASS 1/1 in disposable systemd unit
  `run-p158830-i159971.service`, with explicit workspace working directory and
  exit 0. It does not add real aria2, permit mismatch, forward post-directory DB
  failure/replay, or B2 terminal failure/panic/Drop coverage.
- Source audit confirms one production `publish_internal_v1_attempt` call site,
  in `PendingArchivePublication::finalize`. The pending-finalizer API still does
  not carry export payload; D1 remains gated on C1. Existing InternalV1 recovery
  idempotence does not inject a failure in the forward publisher after directory
  commit. No production change or test seam was added; V2 remains fail-closed.
- `cargo test -p xarchive-storage --lib archive_recovery::tests::repeated_internal_recovery_is_idempotent_after_commit -- --exact`
  PASS 1/1. This is recovery idempotence evidence only, not forward failure injection.
- Current-round Desktop production startup recovery/terminal repair test
  `executor::tests::startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`
  PASS 1/1. The delegated production runner PASS 1/1 in disposable systemd unit
  `run-p160698-i161846.service`, exit 0.
- Session-boundary writer-drain-failure/no-completion, fenced no-double-completion,
  and non-fenced completion tests each PASS 1/1. Complete-job recovery duplicate
  suppression, maintenance admission, and terminal transaction atomic/idempotent/
  stale-attempt/requires-DOWNLOADED tests each PASS 1/1.
- **FAIL (test fixture, before production recovery call):**
  `executor::tests::storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution`
  panics at `executor.rs:10015` while setting up a second `Downloading -> Complete`
  transition. The state machine permits `Downloaded -> Complete`; the test never
  reaches `recover_commits_persisted`. The `&&` chain stopped there; subsequent
  tests were run separately and passed as listed above. No code was changed to
  accommodate this fixture, and this is not counted as a recovery-path product
  failure or pass.

### Continuation delta — startup drain and owner lifecycle retest — 2026-10-31

- Environment rechecked: Linux x86_64 WSL2/cgroup v2; shell membership
  `/non-systemd`; user systemd running; `aria2c` unavailable. HEAD/origin remain
  `1baefa345053204e0eb252213fdcdf4544494277`, with six tracked changes and no
  untracked files at round start.
- Exact Desktop checks each PASS 1/1: startup recovery terminal repair/idempotence,
  executor transport-generation replacement, crashed-writer startup drain ordering,
  cleanup-pending owner resolution across the session boundary, real Sidecar
  stuck-cleanup-owner retention/abort, persisted scheduler attempt binding, and
  V2 missing-journal fail-closed. Transport replacement does not prove active
  registered-writer continuity. No ignored real-aria2 or live-descendant SIGKILL
  check was counted as run.
- At that earlier continuation checkpoint, the mixed Storage recovery fixture
  failure remained unchanged. It was corrected and both mixed-recovery fixtures
  passed in the later continuation recorded at the top of this file.
- Remaining executable boundary sweep yielded no additional non-Windows production
  acceptance independent of the outstanding dependencies. Next: real aria2
  drain/retry when available; obtain reviewed production-faithful failure injection
  seam for B1/B2; correct the fixture in its owning batch; continue active-writer
  replacement continuity; defer Windows-native tests to the Windows Owner.

### Continuation delta — scheduler/recovery route retest — 2026-10-31

- Reconciled HEAD/origin `1baefa345053204e0eb252213fdcdf4544494277`, branch
  `cross-platform/automatic-pairing-reconcile-20261002`; six tracked modifications,
  no untracked files. Linux x86_64 WSL2, cgroup v2, shell membership
  `/non-systemd`, user systemd running; `aria2c` absent.
- Exact Desktop tests each PASS 1/1: persisted scheduler attempt binding;
  startup redispatch attempt binding; V2 missing-journal fail-closed; persisted
  recovery candidate filtering; V3 Storage-backed submission selecting InternalV1;
  recovery skipping terminal/duplicate jobs while restarting interrupted jobs.
  These cover dispatch/contract/recovery routing and do not substitute for writer
  authority or real aria2 acceptance.
- Delegated permit-bearing InternalV1 production runner PASS 1/1 in disposable
  systemd unit `run-p162160-i163317.service`, exit 0. The runner still covers only
  its existing contained publication, uncontained refusal, and PREPARED collision
  scenarios; no new failure injection was performed.
- B3 real aria2 drain/retry remains NOT_RUN because `aria2c` is unavailable.
  B1 permit mismatch and forward post-directory DB-failure/replay, B2 runner
  terminal failure/panic/Drop, active registered-writer executor-replacement
  continuity, and Windows-native acceptance remain open. No production code,
  test seam, or fixture was changed.

## Active-source reconciliation — 2026-10-30

### Continuation update — 2026-10-30

The initial reconciliation below is retained as the previous checkpoint and is
superseded where noted here. Current owner cursor is the active section in
[`../status/platform-handoff.md`](../status/platform-handoff.md).

- **Source:** dirty HEAD `2f6eb9a3db13603bc2571eea4e383407fc1eb119`, branch
  `cross-platform/automatic-pairing-reconcile-20261002`; 18 tracked files
  modified, no untracked files. Linux x86_64 WSL2/container; Rust/Cargo available.
- **Desktop targeted check:**
  `cargo test -p xarchive-desktop --lib archive::completion_tests::pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal -- --exact`
  PASS 1/1 (exit 0). Storage and Desktop `cargo check --lib` PASS (exit 0).
  Workflow lifecycle tests PASS 4/4. `node scripts/docs-audit.mjs` and
  `git diff --check` PASS.
- **Storage targeted V1 tests:** attempted the V1 database-finalize rollback/replay
  and InternalV1 intent recovery tests, but both stopped during test-crate
  compilation (exit 101) before test execution because 16 V2 test call sites in
  `crates/xarchive-storage/src/lib.rs` call nonexistent
  `PublicationPermit::mint`. This is a fixture/build blocker, not a V1 product
  result. Do not add a permit-minting escape hatch.
- **Production runner V1:** ignored pending an explicitly delegated disposable
  cgroup; NOT_RUN. No equivalent integration test was run. No Windows validation.
- **Source audit:** V4 D1 decoding/serialization exist, but current InternalV1
  pending publication does not carry serialized exports; four output combinations
  remain NOT_VERIFIED on the current finalization path. C1 remains open, so D1
  activation is deferred.

Full workspace regression was not run: Storage unit-test compilation is blocked,
and this continuation did not change production Rust; targeted desktop/workflow
checks and source audit were selected. Continue with a legitimate V2 fixture,
then run current-source V1 forward crash/replay and B2 owner-retention checks.

Current Linux dirty HEAD: `2f6eb9a3db13603bc2571eea4e383407fc1eb119`, branch
`cross-platform/automatic-pairing-reconcile-20261002`, WSL2. Current source has
runner-local move-only InternalV1 pending publication; older entries below that
describe an uncomposed session/result path and are superseded for implementation
status. Historical validation remains tied to its recorded revisions.

The latest targeted Desktop test attempt failed **before test execution**: the
active pending finalizer in `desktop/src-tauri/src/archive.rs` calls
`ArchiveService::complete_sidecar_archive_attempt`, which is `pub(crate)` in
`crates/xarchive-storage/src/archive_service.rs`. A second compile error is in
an uncalled Desktop V2 helper. This is an API visibility/integration blocker,
not a runtime publication result. Storage lib check, docs audit and diff check
passed in that previous continuation; Desktop check/test failed. No Full or
Windows validation was run.

Remaining Linux evidence: P1 pending finalizer panic/Drop/terminal-failure
retention; P2 production writer lifecycle permutations; P4 forward crash/replay;
P5/P6 production-entry and recovery integration. Existing recovery rollback
tests do not substitute for failures injected into the forward publication
path. D1 is C1-dependent. F2/F3 have historical shared implementation evidence
and require affected-source applicability review, not wholesale reimplementation.
Production V2 stays fail-closed; Windows-native acceptance remains separately
queued. The current execution matrix is in
[`../status/platform-handoff.md`](../status/platform-handoff.md).

## 2026-10-20 lifecycle continuation — documentation gate and baseline

- **Owner/state:** Cross-platform Owner; Linux x86_64 under WSL2. Branch
  `cross-platform/automatic-pairing-reconcile-20261002`, source/HEAD
  `918a29909792e34b2ef4f1c4a6a15bcb983e5cdd`, equal to upstream at start of
  this batch. Five documentation files were already dirty at entry; no product
  source changes, commit, push, or handoff were made. This entry is not yet
  bound to the final documentation dirty diff.
- **Capability:** `cargo check -p xarchive-desktop --lib` was available and
  completed successfully (exit 0; 5.00s). `git diff --check` completed with
  exit 0 before these documentation edits. Delegated cgroup availability was
  not tested/established; no delegated test result is claimed. No Windows-native
  validation was run.
- **Reconciliation:** the historical report of
  `SidecarStartupFailure` → `SupervisorError` failure does not reproduce in the
  current Desktop library check. Treat it as stale pending a targeted
  owner-lifecycle review; do not change product code on that old diagnostic.
  Startup/ready/partial-start/shutdown cleanup ownership remains open for L1.
- **Not run:** tests, delegated descendants, actual aria2/gallery-dl
  compatibility, Windows, crash/replay matrix, and integrated C1 acceptance.
  These are NOT_RUN, not PASS; this batch provides compile evidence only.

## 2026-10-20 L1 owner-lifecycle trace

- Source/HEAD: `918a29909792e34b2ef4f1c4a6a15bcb983e5cdd`, branch
  `cross-platform/automatic-pairing-reconcile-20261002`, Linux x86_64 WSL2.
  Source status at entry was clean; five documentation paths were already
  modified. No product source was changed.
- Reconfirmed `cargo check -p xarchive-desktop --lib`: PASS, exit 0. The
  prior `SidecarStartupFailure` compile report is stale at this revision.
- Source inspection (not runtime test): contained spawn/setup errors and
  handshake/shutdown errors are represented by `SupervisorError`; failed
  cleanup is flattened to `ShutdownIncomplete(String)`. Factory startup maps
  this to string-only `ExecutorError`; `JobExecutionFactory::create_for_attempt`
  has no move-only failed-resource return. Therefore a failed startup/drain
  cannot currently be handed to the runner/session for same-owner retry. This
  is the blocking L1 integration gap; no change was made because a Sidecar-only
  error-type adjustment would leave the runner ownership gap intact.
- False lead resolved by inspection: `collect_attempt_facts` stores retained
  aria2 back into `context.retained_aria2` before returning on the relevant
  error paths. No missing-owner defect was confirmed there.
- NOT_RUN: same-owner startup cleanup retry tests, delegated descendants/cgroup
  fixture, ignored-test inventory, L2/L3 integration, full regression, Windows,
  and C1 acceptance. No test PASS is claimed by this source trace.
- Next: inspect approved disposable cgroup prerequisites and test filters;
  then add the minimal runner-local move-only startup-failure handoff and retry
  coverage under ADR-015 before advancing L2. Keep production v2 fail-closed.

### 2026-10-20 V0 capability and test inventory addendum

- Capability probe: Linux x86_64 WSL2 kernel
  `6.18.54.1-microsoft-standard-WSL2`; `/proc/self/cgroup` initially
  `0::/non-systemd`; cgroup v2 mounted at `/sys/fs/cgroup`; `systemd-run`
  available; `aria2c` and `gallery-dl` absent from PATH. No host cgroup
  configuration was changed.
- `systemd-run --user --scope --property=Delegate=yes ...` successfully placed
  the probe in `user.slice/.../app.slice/run-p370066-i371413.scope`, but both
  `/sys/fs/cgroup/cgroup.procs` and `cgroup.subtree_control` were not writable
  from that scope. Therefore the approved disposable delegated-parent
  prerequisite is **BLOCKED** in this environment; delegated cgroup fixtures
  were not run. Two earlier probe invocations had incompatible `systemd-run`
  option combinations and did not execute; the successful probe above is the
  final capability evidence.
- Test inventory at the same source HEAD: `cargo test -p
  xarchive-sidecar-supervisor --lib -- --list` exit 0, 16 tests, 0 ignored;
  `cargo test -p xarchive-desktop --lib -- --list` exit 0, 357 tests. Desktop
  contains 4 `#[ignore]` delegated/real-tool fixtures, including contained
  handshake, protocol transfer, real aria2, and owner-SIGKILL startup recovery.
  Filters `shutdown` (Sidecar crate) matched 2; `handshake` matched 1;
  `contained` and `drain` matched 0 in that crate. These are list-only counts,
  not test execution results.
- Existing desktop tests matching `production_factory_contained_handshake`,
  `production_transfer_rpc_fixture`, `production_real_aria2`, and
  `startup_recovery_drains_live_descendant_after_owner_sigkill` are ignored
  unless their documented prerequisites are explicitly supplied. Real tool
  compatibility remains NOT_RUN (tools absent), not substituted by fixtures.
- Targeted non-delegated execution (serial Cargo, each command exit 0):
  `cargo test -p xarchive-sidecar-supervisor --lib` PASS 16/16;
  `cargo test -p xarchive-workflow --lib writer_lifecycle -- --nocapture`
  PASS 2/2; `cargo test -p xarchive-desktop --lib attempt_facts -- --nocapture`
  PASS 3/3; `cargo test -p xarchive-desktop --lib
  archive_context_shutdown_joins_sidecar_before_completion -- --nocapture`
  PASS 1/1. These confirm existing drains/facts behavior only; they do not cover
  contained startup failure retaining the same owner for retry, so L1 remains
  open.

## 2026-10-20 L1 acceptance reconciliation (failure-owner lifecycle)

- L1 scope as recorded above: the blocking gap was that a failed
  startup/ready/partial-start cleanup could not be handed to the runner/session
  for a same-owner retry — string-only factory/executor errors, no move-only
  return channel, and no retry coverage.
- Evidence bound to HEAD `918a29909792e34b2ef4f1c4a6a15bcb983e5cdd` plus the
  current uncommitted dirty diff (supervisor `error.rs`/`lib.rs`, desktop
  `executor.rs`/`batch.rs`, documentation):
  1. Supervisor mechanics — `cargo test -p xarchive-sidecar-supervisor`
     17 passed / 0 failed: `startup_failure_owner_retries_cleanup_on_the_same_supervisor`
     proves a failed cleanup retains the same supervisor and a later retry
     succeeds on it.
  2. Contract wiring — Desktop

- **L1 verdict (2026-10-20, scoped):** L1 is scoped-CLOSED on the shared-contract path —
  the production factory's pending-owner branch and the
  `SIDECAR_STARTUP_CLEANUP_INCOMPLETE` abort branch have real end-to-end
  evidence (`production_factory_retains_a_real_stuck_cleanup_owner_and_resolve_aborts`,
  PASS), and the retry-success path is covered by the boundary contract test.
  Residuals NOT accepted: the contained/delegated branch (its fixture stays
  `#[ignore]`: BLOCKED — no disposable delegated cgroup), contained-cleanup-failure
  injection (no fixture), non-startup shutdown-cleanup ownership (later plan work),
  and Windows behavior (NOT_RUN). Do not add a parallel lifecycle state machine
  and never release the owner after an unproven drain.
     `startup_cleanup_pending_owner_crosses_the_boundary_and_resolves_same_owner`
     proves factory → `SessionRunOutcome::StartupCleanupPending` →
     `resolve_session_run` with a real `StartupFailureOwner`, where the
     durable FAILED decision is returned only after the same-owner retry
     succeeds.
  3. Real production path —
     `production_factory_retains_a_real_stuck_cleanup_owner_and_resolve_aborts`
     (PASS, ~4.2s, no delegation required): a pipe-holding setsid descendant
     makes cleanup genuinely fail on the uncontained production spawn, so
     `create_for_attempt` returns a real pending owner and
     `resolve_session_run` reports `SIDECAR_STARTUP_CLEANUP_INCOMPLETE`
     instead of dropping it.
  4. Regression — Desktop full lib 353 passed / 0 failed / 6 ignored; Sidecar
     17/0; `cargo check -p xarchive-desktop --all-targets`,
     `cargo fmt --all --check`, `git diff --check`,
     `node scripts/docs-audit.mjs` all exit 0.
- **Verdict (scoped): the L1 blocking gap — startup/ready/partial-start
  failure-owner handoff through the runner/session contract with same-owner
  retry — is CLOSED on the shared-contract path.** The contained spawn shares
  exactly this conversion code (`require_ready_v2`, `shutdown`,
  `AttemptCreationError`, outcome/resolve); only its cgroup-specific
  verification rows remain. This is not a blanket PASS: it does not close the
  matrix rows for descriptor-conditioned contained enrollment/startup, C1/P6,
  or production v2, which stay fail-closed/conditional as recorded.
- Residual conditions explicitly NOT accepted here:
  (a) the contained/delegated spawn branch — fixture `#[ignore]`, BLOCKED by
      missing disposable delegated cgroup;
  (b) contained-cleanup-failure injection (cgroup `stop_and_drain`/
      `remove_group` error → pending owner) — no fixture exists; only the
      shared conversion code and compile reachability are evidenced;
  (c) runtime (non-startup) shutdown-cleanup ownership still uses the
      archive-boundary fail-stop adapter; typed owners through the archive
      result and attempt-fenced finalization remain the plan's later work;
  (d) Windows behavior — NOT_RUN, Windows-owned.
- Sequencing consequence: the repository has no formal L2/L3 definition; the
  labels are retired here in favor of the plan's P/B numbering. The next
  milestone is the plan-ordered B5 composition (live session, destination,
  owner-drain and verified-manifest authorization into Storage) under ADR-015.
  This reconciliation changed no product code; production v2 stays
  fail-closed.

## 2026-10-18 approved A + P1 implementation evidence

Base `14f465b48d999296fa3ed054af0c46de1d389476`, branch
`cross-platform/automatic-pairing-reconcile-20261002`, dirty; owner Cross-platform.
Method: local Linux module and embedded Tauri native automation, controlled files.
Migration 0020 adds the exchange ledger without fabricating legacy provenance.
Tests cover P1 directory resolution, missing/non-directory/link/traversal rejection,
JSON/YAML idempotency, ledger preservation, download-to-archive completion,
SQL-trigger rollback and accepted-prefix rollback. A former tautological user
assertion now checks the actual user/Tweet relationship.

| Check | Result | Local log/exit prefix |
|---|---|---|
| Storage library | PASS 170 | `/tmp/tw2tg-p1-storage-r3` |
| Desktop library | PASS 350, six default ignored | `/tmp/tw2tg-p1-desktop-full` |
| Frontend | PASS 230 | `/tmp/tw2tg-p1-ui` |
| Frontend production build | PASS | `/tmp/tw2tg-p1-ui-build` |
| Storage/Desktop Clippy | PASS exit 0, warnings remain | `/tmp/tw2tg-p1-clippy` |
| Embedded native build | PASS | `/tmp/tw2tg-p1-native-build` |
| Native Linux WDIO | PASS 5 | `/tmp/tw2tg-p1-native` |

Native archive test uses a disposable XDG profile and root, actual IPC preview,
accept, YAML export and repeat import. Independent account `collector` is preserved
despite author username `alice`; final_path survives. App SHA-256:
`0e9b684a11dea2001bf256b54c7b648929a1bd75980308f1cd3e9549950209e2`.
Identity snapshot `/tmp/tw2tg-p1-source.json` excludes unknown lock files; later
documentation edits do not change the tested executable. Terminal integration
intermittently reported closure/completion errors; results above were verified
using separately persisted logs and exit files. The initial Desktop name filter
matched zero tests and is compile-only evidence, superseded by the module run.
No new Windows, release or real-account acceptance; no commit/push. Batch export,
full conflict-preview parity and final G reconciliation remain follow-up work.

## 2026-10-18 G production entry reconciliation

Final follow-up supersedes earlier fingerprint/artifact below:
`/tmp/tw2tg-g-source-final.json`, source fingerprint
`85e4bdd5c88e99e5fcff607939076ec9bd76c79015dec49e5968dfb38abdafa8`,
embedded Linux app SHA-256
`ed96b4c1aff511777ef019a4d28492e5239996ad9a1b90c7b94e651e69dfa99c`.
Core 49 PASS (`tw2tg-g-core-close`), Desktop 349 PASS / 6 default ignored
(`tw2tg-g-desktop-close2`), frontend 230/build PASS (`tw2tg-g-ui-select`,
`tw2tg-g-build-select`), lint exit 0 (`tw2tg-g-lint-close2`), final embedded
native WDIO 4 PASS (`tw2tg-g-wdio-final`). Each prefix has log/exit records.
Earlier `desktop-close` and `lint-close` failed compilation on a redaction-helper
arity error; fixed and revalidated, not hidden. Nested unknown/duplicate metadata
keys now rejected for both formats; export refuses structural private URLs and
non-portable paths; accepted-record UI allows explicit per-record selection.

Remaining gate: ArchiveRecord.final_path has no approved durable mapping in the
existing metadata/root-only import transaction. Archive apply now explicitly
refuses rather than discarding final_path; preview remains available. Freeze
path/media/idempotency semantics or explicitly defer archive apply before overall
F3/G acceptance. CROSS_PLATFORM_CHANGE_REQUIRED, decision-required; not a tool
failure or Windows blocker. No immutable commit or formal handoff yet.

Owner Cross-platform; priority P1; Linux x86_64 WSL2
6.18.54.1-microsoft-standard-WSL2, local Cargo/Node/Python and user systemd
delegation. Base `14f465b48d999296fa3ed054af0c46de1d389476`, dirty delta,
branch `cross-platform/automatic-pairing-reconcile-20261002`.
Identity `/tmp/tw2tg-g-source-identity.json`; source fingerprint
`5b9c7420056a04ba6b4fb76feb2679d32d5387b922618107cbd66431fed9d61c`.
No commit/push, release readiness or formal Windows handoff claimed.

Expected: strict versioned parser reaches preview/apply, selected evidence is
rechecked as a whole after maintenance drain, canonical exports round-trip,
changed input/root invalidates preview, and affected lifecycle checks stay green.

| ID / check | Result | Evidence prefix under `/tmp/` (log and exit record) |
|---|---|---|
| G-CORE / canonical model | PASS 49 | `tw2tg-g-core` |
| G-STORAGE / evidence and transactions | PASS 166 | `tw2tg-g-storage-final` |
| G-DESKTOP / module regression | PASS 349, six ignored by default | `tw2tg-g-desktop-final` |
| G-F3 / final async command compile and regressions | PASS 6 | `tw2tg-g-async` |
| G-UI / frontend | PASS 230 | `tw2tg-g-ui-final` |
| G-BUILD / Vite | PASS | `tw2tg-g-build-final` |
| G-PYTHON / protocol sidecar | PASS 101 | `tw2tg-g-sidecar` |
| G-WORKFLOW / lib, integration, doctests | PASS | `tw2tg-g-workflow` |
| G-CONTAINMENT / delegated Workflow ignored checks | PASS 2 | `tw2tg-g-workflow-delegated` |
| G-REAL / actual aria2/gallery-dl loopback | PASS 2 | `tw2tg-g-real-20261018` (also JSON identity) |
| G-LINT / affected Storage/Desktop Clippy | PASS exit 0, existing warnings | `tw2tg-g-clippy-final` |
| G-GUI / embedded Linux Tauri smoke + preview IPC | PASS 4 | `tw2tg-g-wdio-f3` |

Four other delegated Desktop tests each PASS (exit 0), with prefixes
`tw2tg-g-production_factory_contained_handshake_and_drop_remove_writer_group`,
`tw2tg-g-production_transfer_rpc_fixture_drains_recorded_writer`,
`tw2tg-g-production_scheduler_publishes_protocol_fixture_bytes`, and
`tw2tg-g-startup_recovery_drains_live_descendant_after_owner_sigkill`.
These plus G-REAL execute all six default ignored Linux Desktop tests; the
Windows-only Credential Manager check is not compiled/executed on Linux.

Build: `cargo test -p xarchive-desktop --lib`; GUI artifact built through
Tauri `build --debug --features wdio-e2e --no-bundle`, prerelease channel,
VITE_WDIO_E2E=1. GUI binary SHA-256
`5b067559e2b7e1d65292f8b5bd8b30fdf4671ceedd639405a022ab09d4933571`.
Plain Cargo debug artifact initially failed smoke (no root at devUrl), recorded
in `tw2tg-g-wdio.{log,exit}`; embedded-asset rebuild passes, recorded separately
in `tw2tg-g-wdio-embedded.{log,exit}` and final `tw2tg-g-wdio-f3.{log,exit}`.
No assertions or production behavior weakened for the setup failure.
WebKitWebDriver and tauri-driver were detected; DISPLAY=:0. Existing warning
about static/dynamic Tauri imports remains non-fatal.

Scope selection: subsystem validation covers changed shared evidence and Desktop
commands plus lifecycle/transfer consumers; unrelated full-workspace/release
suite not run. Actual tools use controlled extraction and loopback bytes, not
live X/account acceptance. No power-loss, Windows NTFS/native/GUI or packaging
acceptance. Windows capability is not assumed absent: PowerShell interop exists,
but this batch performs no Windows execution or ownership transfer.
F3/G overall acceptance remains open for nested strictness/redaction assertions,
selective UI acceptance and archive-path persistence review. Reuse only where
code/dependencies/environment are unchanged; otherwise revalidation required.

## C1 bounded closure and D1 production integration — 2026-10-17

F2 continuation checkpoint: strict dedicated configuration JSON DTO and pure
target-local preview in `config.rs` pass two tests, exit 0, at
`/tmp/tw2tg-f2-contract-green3.{log,exit}`. Red compile evidence retained at
`/tmp/tw2tg-f2-contract-red.log`. This proves parser/preview only, not maintenance,
apply, persistence, UI or F2 acceptance. Shell-injected Ctrl-C prevented green2
log creation; no PASS inferred. Concrete contract is recorded in Plan §12.

## F2 maintenance barrier + command layer — 2026-10-17

Implemented and validated the shared cross-process maintenance barrier plus the
preview/apply command surface behind it (branch/HEAD unchanged, dirty).

| Check | Result | Evidence |
|---|---|---|
| `xarchive-workflow` maintenance tests | 3 PASS | `/tmp/tw2tg-f2-maint.{log,exit}` exit 0 |
| `xarchive-desktop` lib | 342 PASS / 6 ignored | `/tmp/tw2tg-f2-cmds.{log,exit}` exit 0 |
| `cargo fmt --all -- --check` | PASS | `/tmp/tw2tg-f2-fmt.{log,exit}` exit 0 |
| `cargo clippy` (workflow+desktop) | PASS (existing warnings only) | `/tmp/tw2tg-f2-clippy.{log,exit}` exit 0 |

New tests: `maintenance::tests::{shared_readers_drain_and_cancel_release_admission, killed_process_releases_maintenance}`
prove shared readers coexist, maintenance closes admission before draining, cancellation
releases, and a killed child process's OS locks release with no lease timeout.
`executor::tests::maintenance_blocks_production_and_startup_recovery_before_mutation`
proves a held `MaintenanceGuard` blocks both the production session boundary and
startup recovery before any durable mutation. `config::tests::configuration_exchange_atomic_persistence_rejects_stale_and_preserves_runtime_on_failure`
proves stale-revision rejection, create-new temp + atomic rename, and that a failed
persist leaves the in-memory config untouched.

Scope: NOT full F2 acceptance. UI wiring and a live-writer drain reconciliation
remain; Windows/native/GUI and live X acceptance remain unproven.

## F2 configuration-exchange UI — 2026-10-17

Wired export/preview/apply into the Download Config page (new "配置交换"
SettingsSection; apply gated behind a preview and sends `expectedRevision`).

| Check | Result | Evidence |
|---|---|---|
| `node --test desktop/test/{settings-render,ui-wiring}.test.mjs` | 55 PASS | `/tmp/tw2tg-f2-ui.{log,exit}` exit 0 |
| `xarchive-desktop` lib | 342 PASS / 6 ignored | `/tmp/tw2tg-f2-cmds2.{log,exit}` exit 0 |
| `npx vite build` (desktop) | PASS | `/tmp/tw2tg-f2-build.{log,exit}` exit 0 |

Scope: NOT full F2 acceptance. A live-writer drain reconciliation (a running
session blocking apply until it finalizes) is not yet exercised; Windows/native/GUI
and live X acceptance remain unproven.

## F2 Linux closure — live-writer drain reconciliation — 2026-10-17

Closed the last open F2 item. `executor::tests::live_production_session_blocks_maintenance_drain_until_finalize`
proves a held production session (active shared) keeps `drain` waiting, the turnstile
refuses new admission during the drain, and drain completes once the session
finalizes. Exit 0.

| Check | Result | Evidence |
|---|---|---|
| `xarchive-desktop` lib | 343 PASS / 6 ignored | `/tmp/tw2tg-f2-fmtfix-desk.{log,exit}` exit 0 |
| `xarchive-workflow` (maintenance 3 + doctests 5) | PASS | `/tmp/tw2tg-f2-final-wf.{log,exit}` exit 0 |
| `cargo fmt --all -- --check` | PASS (after formatting new test) | `/tmp/tw2tg-f2-fmt2.{log,exit}` exit 0 |
| `node scripts/docs-audit.mjs` | PASS | `/tmp/tw2tg-f2-final-docs.{log,exit}` exit 0 |
| `git --no-pager diff --check` | PASS | `/tmp/tw2tg-f2-diff2.{log,exit}` exit 0 |

F2 Linux/Cross-platform integration complete; all Plan §12 required tests covered.
Windows/NTFS adapter, native GUI and packaging acceptance remain Windows-owned and
are NOT claimed. Formal handoff requires a Git commit (not done).

## F3 first slice — canonical record model + round-trip — 2026-10-17

Implemented per the frozen F3 contract. New `crates/xarchive-core/src/record_exchange.rs`.

| Check | Result | Evidence |
|---|---|---|
| `cargo test -p xarchive-core` | 49 PASS / 0 failed | `/tmp/tw2tg-f3-core4.{log,exit}` exit 0 |
| `cargo fmt -p xarchive-core -- --check` | PASS | `/tmp/tw2tg-f3-fmt4.{log,exit}` exit 0 |
| `cargo clippy -p xarchive-core` | exit 0, no new warnings | `/tmp/tw2tg-f3-clippy.{log,exit}` exit 0 |
| `cargo check --workspace` | exit 0, downstream unaffected | `/tmp/tw2tg-f3-check.{log,exit}` exit 0 |

Covers JSON↔YAML round-trip to identical canonical bytes, final_path preservation,
strictness (unknown kind/schema/fields, duplicate JSON keys), duplicate-identity
rejection, derived identity, and kind distinction. Red evidence retained at
`/tmp/tw2tg-f3-core2.{log,exit}` (mangled test-nesting compile error + 48/49).
NOT F3 completion: evidence binding, preview/accept UI, maintenance-guarded import
transaction remain; ZIP/7z not authorized.

## F3 evidence binding — Storage preview — 2026-10-17

New `crates/xarchive-storage/src/record_evidence.rs`: pure accept/preserve/reject
preview of one record against a user-selected archive root (lexical escape
rejection before any probe, regular-file + size + constant-time SHA-256 check).

| Check | Result | Evidence |
|---|---|---|
| `cargo test -p xarchive-storage` | 161 PASS / 0 failed | `/tmp/tw2tg-f3-storage.{log,exit}` exit 0 |
| `cargo fmt -p xarchive-storage -- --check` | PASS | `/tmp/tw2tg-f3-evid-fmt.{log,exit}` exit 0 |
| `cargo clippy -p xarchive-storage --lib --tests` | exit 0 | `/tmp/tw2tg-f3-evid-clippy.{log,exit}` exit 0 |

Covers accept-on-match, size/digest mismatch preserved, `..`/absolute rejection
(attacker's probe never consulted), probe-error preserved. A stray-edit stub
fragment was repaired to one `constant_time_eq`. NOT F3 completion: the
maintenance-guarded import transaction, command surface and UI remain.

## F3 decision gate — import-transaction write mapping — 2026-10-17

Recon done, implementation deliberately NOT started. Landing the accepted record
set touches production durable tables (`tweets` UNIQUE tweet_id, `user_id → users`,
`merged_metadata_json`/`archived_at`; `jobs.tweet_id → tweets` + one-active-archive
partial unique index). Naming that upsert/merge/linkage without a frozen mapping
would be a guessed durable contract, forbidden by Plan §11. This is
`decision-required` (freeze: tweets upsert keys, metadata merge rule, media-fact
destination, job linkage, recovery interplay), not a tooling blocker and not a
budget claim. All remaining paths (import transaction → commands → preview/accept
UI → integrated G) are downstream of it. No product code changed for this gate.







Source branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD
`777e0b4cd58c825f9d5d21da6098932d62e0e43d` plus dirty worktree; Linux WSL2.
No commit/push, Windows ownership transfer, native GUI or live X acceptance.

C1 matrix below is superseded for the actual-tool row: real aria2 1.37.0 and
gallery-dl 1.31.10 execute the ordinary persisted/startup/publication chain with
loopback media and controlled extraction. Final media paths, bytes, DB paths and
all published business metadata agree; independently generated `archived_at` is
nonempty and excluded only from cross-job equality. Historical evidence
`/tmp/tw2tg-c1-equivalence-r3.{json,log,exit}` is replaced by current artifact
`/tmp/tw2tg-d1-real-v4.{json,log,exit}`: 2 PASS, exit 0, covering both legacy v3
JSON/TXT and v4 JSON/YAML. The bounded Linux C1 matrix is accepted; this is not
live X extractor/account, power-loss, Windows NTFS or GUI acceptance.

D1 now carries explicit v4 metadata settings from persisted settings, Desktop
and browser submission, and versioned batch inheritance to the executor factory,
archive execution and pre-PREPARED create-new publication. The recovery manifest
has an optional strictly validated v4 format selection; absent selection preserves
historical serialization and JSON/TXT semantics. No database migration or existing
archive rewrite. YAML joins the reserved export filenames. JSON/YAML serialization
uses the same final-path metadata DTO; recovery reads the internal candidate, not
user exports. New settings use `download.metadata_output.v4`; old setting and
batch snapshots are not mutated. New UI displays independent JSON/YAML switches.

Evidence: `/tmp/tw2tg-d1-storage-v4.{log,exit}` 158 PASS;
`/tmp/tw2tg-d1-desktop-v4.{log,exit}` 338 PASS/6 ignored;
`/tmp/tw2tg-d1-ui-final.{log,exit}` 55 PASS;
`/tmp/tw2tg-d1-build.{log,exit}` Vite build PASS. All exit 0.
Storage tests cover four combinations in forward publication and PREPARED crash
replay after deleting every optional export. Existing legacy compatibility,
corruption/unknown version, atomic finalization and collision tests remain green.
Desktop tests cover strict v4 dispatch, same-DTO semantic equivalence, old batch
inheritance and new batch snapshot decoding, browser v3 and v4 persisted snapshots.
Real-tool tests compare YAML/JSON and assert old TXT exists only for legacy jobs.
Intermediate incomplete shell captures and obsolete UI-text assertion failures
are retained separately; no production behavior was weakened to satisfy them.

## Real-tool continuation — 2026-10-17

Supersedes the tool-unavailable disposition below, not the remaining C1 review.
Actual aria2 1.37.0 (Ubuntu package 1.37.0+debian-4 and required libraries) and
gallery-dl 1.31.10 (isolated temporary venv) were provisioned under
`/tmp/tw2tg-c1-real-tools`. Default proxy TLS failed; direct PyPI access worked.
No system install, personal config change, external media or account access.

`scripts/validate-real-download-tools.py` requires explicit executable paths,
binds versions/source dirty status/library SHA-256, starts loopback HTTP media,
and runs ignored real-tool checks under disposable systemd user delegation.
Run it with `--binary <current Cargo JSON library test artifact> --aria2 <absolute
aria2c> --gallery-dl <absolute gallery-dl> --evidence <fresh absolute prefix>`.
For extracted shared libraries, provide `LD_LIBRARY_PATH` in the invoking process.

`/tmp/tw2tg-c1-real-final.{json,log,exit}`: 2 PASS, exit 0. Artifact SHA-256
`f1dd1800e1c0346e847cb8655dc7e2bec8db37148686dd7468274386f21f35d4`.
Both engines execute ordinary persisted startup scheduling through production
containment/transfer/publication/finalization and reach COMMITTED. Preview media
path equals final file, `tweet.json` media path and DB media path; bytes agree.
An additional real aria2 transfer proves recorded writer drain/group removal.
Controlled extraction and gallery directlink metadata keywords are fixture
inputs: NOT live X extraction, account compatibility, complete metadata equality,
Windows, NTFS, native GUI or overall C1 acceptance. Runtime executable_args are
test-only; application user-option policy is unchanged.

Desktop `/tmp/tw2tg-c1-real-desktop.{log,exit}`: 334 PASS/6 ignored, exit 0.
Build r3 retained missing test Duration import failure; r4 retained fixture
mutation of frozen config and incorrect expected job ID. Fixed fixtures only.
Initial isolated aria2 probe had zero retry defaults; supplying valid explicit
network settings fixed it. Standalone gallery probe first lacked canonical tweet
identity; corrected fixture keywords, production validation unchanged. Prior
Storage recovery results remain separate; no Storage code changed in this batch.

## C1 production acceptance reconciliation — 2026-10-17

ID `C1-PRODUCTION-REVIEW`; priority P0; owner Cross-platform; implementation
IN_PROGRESS; overall acceptance remains open. Source branch
`cross-platform/automatic-pairing-reconcile-20261002`, HEAD
`777e0b4cd58c825f9d5d21da6098932d62e0e43d` plus dirty worktree. No formal handoff.

| Requirement | Production entry/consumer | Current evidence and disposition |
|---|---|---|
| Admission before claim; bounded contention | `SessionExecutionBoundary::execute_on_runner`, `execute_session` | Desktop tests `a_saturated_runner_is_rejected_before_any_claim`, `a_lost_claim_is_reschedulable_releases_scopes_and_persists_no_failure`, `a_session_holds_both_scopes_across_execution_and_finalization`: PASS in current library run |
| Containment, drain before refresh, retained ownership | `collect_attempt_facts`, `transfer_once`, `transfer_with_one_refresh` | Delegated `production_transfer_rpc_fixture_drains_recorded_writer`: PASS; actual production functions exercise success, expiry, changed media, second expiry, cancellation and failed cleanup/same-owner retry. Protocol fixture, NOT real aria2/gallery-dl compatibility |
| Stale attempt fencing | Storage claim APIs, `publish_prepared_internal_rename_v2`, `finalize_archive_recovery_v2` | Storage stale-attempt and COMMITTED claim exclusion regressions PASS |
| No-replace publication | Storage forward publish and attempt staging commit | Destination/export collision and unchanged-filename regressions PASS; Windows NTFS evidence separate |
| Crash boundaries | Storage failpoints and `recover_internal_rename_v2` | `v2_forward_publish_crash_windows_converge_through_recovery_replay` PASS; live-descendant SIGKILL successor-drain delegated PASS; not power-loss proof |
| Atomic business facts/events/candidate retirement | `finalize_archive_recovery_v2`, Desktop `complete_terminal_for_attempt` | Event-trigger rollback and duplicate completion/read-only replay PASS; candidate retained on rollback, retired with COMMITTED |
| PREPARED compatibility, corruption and binding | `archive_recovery_manifest_v2`, `archive_recovery_v2_candidate` | Safe v1/v2 encode/decode PASS; new missing/corrupt/unsupported/identity/media matrix PASS without candidate/journal/staging mutation |
| Restart and legacy dispatch | `recover_startup_with_v2`, `recover_v2_on_runner` | Desktop startup suite and delegated ordinary-submit/restart fixture PASS; Legacy/InternalV1 and unprovisioned rejection preserved |
| Both actual transfer engines and final-path output equivalence | `download_v2` -> `execute_archive_context` -> Storage publication | BLOCKED for actual-tool acceptance: neither executable on PATH, no gallery-dl package in current venv or binaries at checked standard/local paths. Protocol fixtures cannot close this acceptance target; no installation, account or external download authorization inferred |

Production defect found independently of terminal tooling: a well-digested
candidate with changed media path passed journal read and replayed filesystem
publication before finalization rejected it. `archive_recovery_manifest_v2` now
compares all candidate media facts with the immutable manifest before any replay
mutation. Classification `CROSS_PLATFORM_REVIEW_REQUIRED`: enforcement of existing
binding, no schema/architecture expansion. The five-case regression asserts raw
journal fields, candidate bytes, original staging bytes and absent destination.

Reproduction/intermediates retained: `candidate.log` compile failure used a
nonexistent test hash helper; `candidate-r2.log` fixture incorrectly used the
validated reader to inspect an invalid record; `storage-final.log` then reproduced
the actual media-binding defect (156 PASS/1 FAIL). These are not final PASS logs.
All abbreviated paths below use `/tmp/tw2tg-c1-review-`.

Final automated evidence: `storage-fixed.{log,exit}` 157 PASS, exit 0;
`desktop-fixed.{jsonl,stderr,exit}` 334 PASS/4 ignored, exit 0;
`delegated-fixed.{log,exit}` 4 PASS, exit 0; `clippy.{log,exit}` exit 0 with warnings.
Build commands: `cargo test -p xarchive-storage --lib`, then
`cargo test -p xarchive-desktop --lib --message-format=json`, then current JSON
library test artifact under `systemd-run --user --wait --pipe --collect -p
Delegate=yes`, `--ignored --test-threads=1`. Artifact SHA-256
`5fd1290b19ab42cd05535a56a8d6300e8454546c8cb70f7b25e4d1c1b089eb0a`.
Environment Linux x86_64 WSL2 6.18.54.1; user systemd running, cgroup delegation
proven by execution. Shell integration intermittently returned unrelated terminal
content; separate completion files and final logs were read back, not assumed.

Revalidation: candidate/manifest/media binding, recovery dispatch, coordinator,
writer lifecycle, publication or dependencies change. No fresh GUI/Windows/native
artifact acceptance. Full suite not selected: affected Storage/Desktop subsystems
and delegated production paths are the justified scope. D1 remains dependency-
waiting under Plan section 9; F2/F3/G follow that dependency. Next acceptance action:
provision approved actual tools/fixtures and exercise both transfer modes through
the conditional Linux production path, then close C1 and activate D1. This is not
a request to reapprove option A or the architecture.

## Approved option A candidate contract — 2026-10-17

- ID `C1-CANDIDATE-CONTRACT-A`; owner Cross-platform; implementation IMPLEMENTED.
  Branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD
  `777e0b4cd58c825f9d5d21da6098932d62e0e43d` plus dirty changes; no commit/push.
- Target: explicit candidate field boundary, unchanged safe legacy encoding,
  reject structured transport URLs/unknown fields before PREPARED, preserve
  unsupported stored rows read-only. Contract and field necessity:
  `../development/recovery-candidate-contract.md`.
- Environment: Linux x86_64 WSL2 6.18.54.1, Rust 1.98.0; systemd delegated
  disposable cgroup execution. Neither aria2c nor gallery-dl found on PATH.
- Automated PASS: `/tmp/tw2tg-plan-a-storage-final2.{log,exit}` 156 tests;
  `desktop-r2.{log,exit}` 334 PASS/4 ignored; `delegated-r2.{log,exit}` 4 PASS;
  `clippy-final2.{log,exit}` exit 0 with warnings; `fmt.{log,exit}` exit 0.
  Abbreviated names use `/tmp/tw2tg-plan-a-` prefix. Delegated executable is
  extracted from current Cargo JSON library artifact, not a guessed old binary.
- Expected/actual: legacy v1 and production v2 DTO accepted without format change;
  future versions and nested unknown fields rejected; unsafe main/quoted URLs
  refused; failure leaves no PREPARED rows; stored invalid extension retained.
  Existing lifecycle/rollback/COMMITTED and production restart tests pass.
- Intermediate FAIL retained: delegated first run 3 PASS/1 FAIL because the first
  implementation incorrectly allowed only metadata v1; actual Desktop output v2
  established the compatibility defect. Corrected explicit v1/v2 support, not
  weakened fixtures. Storage r2 failed to compile after a fuzzy patch inserted
  assertions inside a macro; corrected placement and final2 passes. No stale
  Storage test result is substituted for that compile failure.
- Full/UI reruns not selected: this batch changes only candidate persistence
  boundary, not frontend/protocol/tool argv. Storage and Desktop subsystem tests,
  delegated containment fixtures and all-target Clippy cover affected paths.
  No real-tool/Windows/GUI/C1 overall acceptance claimed. Prior broad privacy
  stop superseded by user-approved option A; C1 acceptance review remains open.

## Conditional Linux opt-in continuation — 2026-10-17

### Subsequent C1 acceptance-review corrections

Final executable startup refusal (including actionable launcher repair guidance):
`/tmp/tw2tg-c1-exit-final-build.{log,exit}` build PASS;
`/tmp/tw2tg-c1-exit-final.{log,exit}` expected exit 2, no descriptor path/content.
Executable SHA-256 `fd36bf8d15c6ac47ecc4d08a7c3ed1f290c7e618ca5d0a675f1e85e27f25a8cb`.
Final format `/tmp/tw2tg-c1-final-fmt.*` and whitespace PASS. Linux display variables,
xdotool and gdbus were detected, but this evidence is CLI/log integration, not GUI
acceptance. No native Windows validation or real download-tool compatibility run.

Historical finding (narrowed by candidate lifecycle reconciliation below;
the assertion of no exception was incorrect): `C1-PRIVACY-CANDIDATE`.
Plan section 4
privacy promise excludes text/author/relationships from internal recovery storage,
but `archive_recovery_v2_candidate.metadata_json` serializes full `ArchiveMetadata`
in `prepare_archive_recovery_v2` even with both exports disabled, then supplies it
to transactional finalization. The candidate table is separate from the minimal
manifest; no accepted retention exception was found. Do not infer that manifest
minimality proves the full recovery subsystem's privacy promise. Resolution needs
the exact privacy/retention decision captured in the current handoff; C1/D1 not
accepted, no merge recommendation. No production privacy contract was rewritten.

Latest lifecycle evidence supersedes attempt-0-only routing and counts below:
`/tmp/tw2tg-c1-acceptance-desktop.{log,exit}` 334 PASS/4 ignored;
`/tmp/tw2tg-c1-acceptance-delegated.{log,exit}` 4 PASS serial;
`/tmp/tw2tg-c1-acceptance-clippy.{log,exit}` exit 0, warnings;
`/tmp/tw2tg-c1-claim-storage-r2.{log,exit}` 154 PASS. Exact Desktop test artifact
SHA-256 `dfb2432f425f7abaa4e1ed245ca7dfec51ee486ebac29d9dc34d3f3f3b5da440`.
Pre-PREPARED startup drains the old domain before successor claim; interrupted
attempt 1 -> 2 production fixture PASS. Published DOWNLOADED with missing facts
still refuses without mutation. PREPARED execution error preserves the candidate
state; targeted test `/tmp/tw2tg-c1-prepared-error.*` PASS. COMMITTED/DOWNLOADED
rejects begin/claim/claim-attempt without events or attempt changes.

Retained intermediate compile failures: `/tmp/tw2tg-c1-claim-storage.*` (test
variable scope corrected) and `/tmp/tw2tg-c1-preprepare-desktop.*` (wrong matching
call site corrected). Terminal capture loss on the r2 command left no completion
exit file; it is not PASS. No live Cargo process remained; explicit r3 rerun
completed 334/4 and delegated 4, then the latest acceptance run above revalidated
the added interrupted fixture. Tool capture failure is separate from product
results and was not used as a reason to weaken acceptance.

ID `C1-ENTRY-REVIEW-20261017`, P0, Cross-platform Owner, implementation IN_PROGRESS;
same branch/source dirty state as current handoff. Linux WSL2, local Cargo build,
user systemd delegated disposable cgroup; protocol fixtures, no real-tool claim.
Expected: invalid provisioning refuses before initialization; replacement failure
does not close old executor; diagnostics omit raw errors; unstarted durable jobs
recover without requiring nonexistent publication facts; historical exports remain
optional and never provide internal recovery authority.

- `/tmp/tw2tg-c1-current-storage.{log,exit}`: 154 PASS.
- `/tmp/tw2tg-c1-current-desktop.{log,exit}`: 333 PASS, 4 ignored, serial.
- `/tmp/tw2tg-c1-current-delegated.{log,exit}`: 4 PASS, serial `Delegate=yes`.
  Exact current Cargo test artifact SHA-256:
  `ae8ebc2e8ebc75eeec9142ac6d4a61129dde832826f3ba559a5fd453e982ccb6`.
- `/tmp/tw2tg-c1-startup-refusal.{log,exit}`: actual Desktop executable returned
  expected exit 2, sanitized stderr, no input path disclosure. Earlier executable
  SHA-256 `5b47bd75d3519aae0335b8fdf9130e30d5efa2f80603baf2a7303a8a197134a6`;
  this is startup-refusal evidence only, not the latest complete runtime artifact.
- `/tmp/tw2tg-c1-diagnostics.{log,exit}`: application-log readback PASS.
  Logs are exposed through existing Logs page / `read_application_logs`.
- Historical JSON/TXT four-switch recovery matrix and create-new collision
  rejection PASS. Export writes precede PREPARED and internal DB candidate remains
  recovery authority. No YAML alias or historical snapshot reinterpretation.

Full workspace regression not run: affected Desktop/Storage modules and delegated
integration selected; dependencies and Windows contracts unchanged. C1 overall
acceptance remains open for lifecycle review, next action in handoff. Earlier
patch-tool timeout was reconciled by file inspection before compilation.

Same source/branch/dirty Linux ownership as the current handoff cursor; no formal
handoff. Runtime provisioning, paired enrollment/startup and replacement are now
implemented but C1 final acceptance review remains open. Ordinary persisted
submission replaces fixture-only v2 insertion in the production scheduler test.

- `/tmp/tw2tg-optin-desktop.{log,exit}`: 332 PASS, 4 ignored, exit 0.
- `/tmp/tw2tg-optin-delegated.{log,exit}`: 4 PASS serially, exit 0, using user
  systemd `Delegate=yes` and current Cargo-produced
  `target/debug/deps/xarchive_desktop_lib-3cde2dc33a044b66`.
- `/tmp/tw2tg-optin-storage.{log,exit}`: 152 PASS, exit 0.
- `/tmp/tw2tg-optin-clippy.{log,exit}`: exit 0; warnings remain.
- `/tmp/tw2tg-optin-docs.{log,exit}`: docs audit PASS before this evidence update;
  final documentation audit is rerun after the update.

Preserved failure `/tmp/tw2tg-paired-delegated.log`: 3 PASS/1 FAIL because the
fixture Runtime socket path exceeded Unix SUN_LEN. Shortened only the fixture
runtime root; targeted rerun and final four-test serial run pass. This was a
fixture-path failure, not evidence to change production socket behavior.
The patch tool once timed out after applying an edit; file inspection confirmed
the edit before compilation. No unobserved execution is counted as PASS.
No Windows, GUI, real aria2/gallery-dl, or overall C1 acceptance claim.

## P4/P6 atomic event and recovery-result continuation — 2026-10-17

### Terminal transaction rollback follow-up

Same branch/source/dirty ownership and Linux environment as below. Targeted
`cargo test -p xarchive-storage v2_download_event_failure_rolls_back_publication_facts_and_replays_once`
PASS 1/1, exit 0 (`/tmp/tw2tg-terminal-rollback.log`, `.exit`). Injected
JOB_COMPLETED abort after successful Storage COMMITTED: asserted unchanged
DOWNLOADED state, complete event list, journal and archive DB facts. Dropped
handles, reopened disk SQLite/FileStore, verified COMMITTED read-only replay,
then retried terminal completion and asserted one download/completion event.
Only test code changed; prior production module/delegated checks are reused.
Full regression not rerun for this test-only extension. No production gate or
Windows acceptance changed. Follow-up remains paired entry enablement decision.

- ID `P4-P6-LINUX-ATOMIC-20261017`; priority P0; Cross-platform Owner;
  implementation IN_PROGRESS (C1 overall), target atomic download-event rollback,
  exactly-once terminal replay and gated retry/refusal diagnostics. Source
  `777e0b4cd58c825f9d5d21da6098932d62e0e43d`, dirty; no handoff.
- Environment: Linux x86_64 WSL2 6.18.54.1, Rust 1.98.0, user systemd delegated
  disposable cgroup. Method integration/delegated; prerequisites existing
  protocol fixtures, Python and real temporary files/SQLite. No Windows or
  real aria2/gallery-dl compatibility acceptance is inferred.
- Production change: Storage commits DownloadCompleted with archive/media,
  DOWNLOADED, Telegram and COMMITTED facts. Desktop retains its separately
  replayable terminal transaction without duplicating the download event.
  Historical COMMITTED rows missing that event are backfilled during pending
  terminal completion. COMMITTED replay itself remains read-only.
- Gated startup exposes joinable per-Job results and validates v2 journals under
  scopes. Refusal preserves durable state; contention returns false; an explicit
  later invocation retries the same attempt. No unbounded retry or unfenced
  error write is added. A public diagnostics consumer remains an enablement item.
- Steps/expected: inject abort trigger on DOWNLOAD_COMPLETED; expect publication
  DB facts/state/journal rollback; drop trigger and replay twice, then complete;
  expect one download and completion event. Exercise historical missing-event
  completion and repeated no-op. Repeat gated missing-journal recovery twice;
  expect inspectable errors and no mutation, then verify unchanged default gate.
- Fixture correction: simulated crash rollback now removes terminal/download
  events belonging to the rolled-back transaction; artificial phase resets are
  not independent process-crash evidence. Existing Storage subprocess crash
  matrix and separately delegated owner-SIGKILL tests remain distinct evidence.
- Intermediate results retained: initial Storage v2 run 29 PASS / 1 FAIL because
  the expected event count still excluded the newly atomic download event;
  corrected count is five (not weaker behavior). An initial delegated launch
  selected the CLI binary and ran zero tests: NOT_RUN, not PASS. Correct library
  binary ran the production fixture 1/1 PASS. Running all four ignored tests in
  parallel in one shared cgroup caused fixture global-cleanup interference
  (3 PASS / 1 FAIL); serial execution in the disposable cgroup passed 4/4.
  Logs `/tmp/tw2tg-cont-{storage,delegated,delegated-correct,all-delegated,all-delegated-serial}.log`.
- Final commands are recorded separately as
  `/tmp/tw2tg-cont-final-{storage,desktop,delegated,clippy}.log` with corresponding
  `.exit`. Delegated checks use the exact library binary from Cargo output and
  `--ignored --test-threads=1`. No workspace-wide full regression: selected
  Storage and Desktop subsystems cover the affected persistence/lifecycle seam.
- Final results: Storage 152 PASS (exit 0); Desktop parallel 330 PASS / 1 FAIL /
  4 ignored (exit 101), failure `ExecutableFileBusy / Text file busy` while
  spawning an existing temporary supervisor fixture. No product change made for
  this environment/fixture failure. Desktop serial rerun 331 PASS / 4 ignored,
  exit 0, `/tmp/tw2tg-cont-final-desktop-serial.log` and `.exit`; intermittent
  parallel fixture execution remains a follow-up, not an erased failure.
  Delegated serial 4 PASS (exit 0); Clippy exit 0, warnings retained; docs audit
  PASS (`/tmp/tw2tg-cont-docs.log`, `.exit`); `git diff --check` PASS.
- Exact final Desktop library artifact SHA256:
  `e20f5e1fa91012c708acb3d5a1efb0221540d671c81ca30e60fcb43a689b2aea`.
  Source SHA256 executor.rs:
  `7c5551fd92fd88974ba571526e54bf80aa33e43b705789b980e99501c8b8d89a`;
  archive_recovery.rs:
  `caa0775d78f64c97277e8c2fa25733f30d6a918c51122ad15a8147115ac8af30`;
  database/jobs.rs:
  `66e466402d839ace52c7f8ae88d97e68b6368ef849c48184911600496bfbf4ca`.
- Revalidation: any affected source, contract, dependency or environment change.
  Blocks development/release: production C1 enablement remains separately gated;
  D1/F2/F3/G cannot be promoted by this evidence. Follow-up: Cross-platform Owner
  reviews paired enrollment/startup enablement and diagnostics consumer.

## 2026-10-17 gated C1 implementation continuation

Source HEAD `777e0b4cd58c825f9d5d21da6098932d62e0e43d`, branch
`cross-platform/automatic-pairing-reconcile-20261002`, dirty Linux working tree;
Cross-platform Owner, no commit/push/handoff. Existing evidence below is retained.

Implemented runner admission → Job/destination reservation → original journal
attempt session binding → recorded writer drain → Storage replay → fenced atomic
terminal Job/events → scope release. No successor attempt is created. Contention
returns without mutation. Missing containment, corrupt/missing journal and corrupt
COMMITTED bytes refuse replay. Startup's gated integration keeps refusal state
unchanged rather than persisting an unfenced failure after releasing scopes;
diagnostic surfacing/retry policy remains a review item. Public startup remains
disabled and ordinary enrollment remains InternalV1.

Applied existing filename rendering to the persisted output snapshot before
PREPARED; final media paths agree across journal, archive metadata and Telegram
intent. The delegated fixture now uses `{tweet_id}_{index}` and verifies
`12091_01.jpg`. It reconstructs PLANNED/TEMPORARY/FINAL and directory-commit
windows on real files/SQLite, recovers via gated startup with the original attempt,
and checks repeated recovery plus contention/refusal event preservation.

Final commands/results (machine-local logs, not portable handoff artifacts):

- `cargo test -p xarchive-desktop --lib`: 331 PASS / 4 ignored, exit 0;
  `/tmp/tw2tg-c1-verified.log` and `.exit`.
- Delegated `executor::tests::production_scheduler_publishes_protocol_fixture_bytes`
  via `systemd-run --user --wait --pipe --collect --property=Delegate=yes`:
  1 PASS, exit 0; `/tmp/tw2tg-c1-verified-delegated.log` and `.exit`.
- `cargo clippy -p xarchive-desktop --lib --tests`: exit 0, warnings retained;
  `/tmp/tw2tg-c1-verified-clippy.log` and `.exit`.
- Earlier independently delegated owner-SIGKILL startup drain: 1 PASS, exit 0;
  `/tmp/tw2tg-v2-final-startup_recovery_drains_live_descendant_after_owner_sigkill.log`.

Final source SHA256: archive.rs
`f4da3a2f7b144400bbf0ae7c15460d7e3aa4be2bdf4d3f3dfbca68796c1583b3`;
executor.rs `d0f58a8a70cd6d39c53306563e73de6ddae285109422ca0ecbe124b2665ae564`.
Test artifact SHA256
`d01f5deb5c2fbc3239a0c725bbaa867cea14c3aaf0e78031c99007643702faf7`.

Intermediate fixture failures were test setup errors: expected unpadded index
instead of the established zero-padded renderer, then reconstructed PLANNED with
final rather than original source filenames. Both corrected without weakening
assertions; their logs remain `/tmp/tw2tg-naming-phase.log` and
`/tmp/tw2tg-naming-r2.log`. One shell launch had invalid trailing `&;` syntax and
did not run; corrected launch produced the final exit records. No Windows or real
aria2/gallery-dl compatibility result is claimed. C1/P6 and the overall goal remain
open; D1/F2/F3/G are not completed by this batch.

## 2026-10-19 authorization and acceptance reconciliation

### Current continuation status (2026-10-20; source HEAD `918a2990`)

The trusted-descriptor-conditioned paired Linux enrollment/startup is already
authorized by the Plan and prior approval record; this is not a request for
additional authorization and does not close C1/P6. Current source has
runner-owned session construction, attempt staging/facts verification, Storage
v2 publication invocation, and startup v2 replay under scopes/session. The
publication gap is that Desktop converts `AttemptArchiveFacts` to the legacy
result shape and Storage does not consume a live-session or typed drain proof.
Storage's durable-attempt and journal checks exist and must be preserved.

- Current Git source: `918a29909792e34b2ef4f1c4a6a15bcb983e5cdd`; only
  `docs/status/platform-handoff.md` was dirty at continuation start. This
  continuation records documentation changes only; it adds no test result.
- Descriptor parser: the earlier 1/1 result is a revision-bound prior claim;
  its complete standalone output was not saved. Do not promote it into current
  enrollment/startup acceptance.
- Descriptor-present ordinary enrollment/startup delegated fixture remains
  NOT_RUN unless a disposable delegated cgroup is successfully provisioned.
  Current environment probing alone has not established writable delegated
  containment. This does not prove a product failure and does not block
  independent shared implementation.
- Next shared batch: complete the B0 Documentation Gate, then make the minimum
  ADR-015-authorized session/attempt/destination/drain/manifest-to-Storage
  integration (`CROSS_PLATFORM_CHANGE_REQUIRED`), keeping v2 fail-closed until
  the complete C1 gate passes. Windows-native implementation and acceptance
  remain Windows-owned and are not inferred here.

The detailed prior results below remain bound to their recorded source SHA and
must not be relabeled as evidence from `918a2990`.

This is the active validation cursor; later-dated sections in this file are
historical evidence bound to their recorded source revisions.

Branch `cross-platform/automatic-pairing-reconcile-20261002`, HEAD
`9e8fc3055d052b5281546eff6a813a6da37c79d4`; Linux x86_64 under WSL2. Working
tree contains the preserved `.gitignore` edit and this documentation batch; no
product source/dependency changes, commit, push or Windows handoff.

Current status; later evidence in this record is revision-bound history and does
not supersede this cursor unless explicitly stated.

- **Authorization correction:** the existing handoff history records the user's
  authorization for paired Linux enrollment/startup when explicit containment is
  provisioned, preserving Windows gates. This supersedes the earlier request for
  another permission decision. ADR-015 clause 8 still excludes unconditional or
  default production-v2/D1 enablement before C1 acceptance.
- **Source review:** `RuntimeState::try_initialize` parses the explicit launch
  descriptor; `initialize_with_containment` constructs the executor with that
  containment or preserves the no-descriptor historical path. `replace_executor`
  carries and revalidates existing containment. `ExecutorRuntime::service` sets
  v2 enrollment from the same containment, and startup recovery receives that
  containment; the executor rejects v2 recovery without writer containment.
  Relevant coverage includes
  `executor::tests::linux_launch_descriptor_is_explicit_bounded_and_fail_closed`,
  descriptor-present initialization/replacement assertions inside the ignored
  delegated `executor::tests::production_scheduler_publishes_protocol_fixture_bytes`,
  fail-closed v2 recovery logic, and the PREPARED/COMMITTED startup recovery
  matrix in `executor.rs`. This explicitly delegated test is not run in this
  batch because its cgroup capability prerequisite has not been established.
- **Bounded current-source evidence:** the 2026-10-19 targeted Linux crate
  baseline recorded later in this file is bound to HEAD
  `9e8fc3055d052b5281546eff6a813a6da37c79d4`:
  Workflow 38/38, Storage 172/172, Desktop 351/351 (6 ignored), Download 53/53,
  Sidecar Supervisor 16/16; delegated Workflow cross-process tests 11/11 and
  5/5. This gives source-level/test evidence for the shared baseline, not proof
  that every runtime/descriptor branch was independently exercised in this run.
- **Disposition:** no product defect has yet been demonstrated. Descriptor-present
  ordinary enrollment/startup remains an uncovered acceptance case in this
  batch; source/test inspection exists, but the delegated fixture was not run.
  Do not present prior evidence from older source SHAs as current-source
  acceptance without matching-code verification. C1/P6 and integrated G remain
  open pending exact acceptance mapping and any missing targeted tests. D1 has
  bounded Linux evidence; F2 Linux integration is recorded complete; F3 shared
  implementation/evidence is partial. Windows-native adapter, NTFS/GUI and
- **Validation:** Desktop test target rebuilt from the current dirty source in
  `/tmp/tw2tg-act-target` (test executable SHA-256 recorded below); isolated
  Linux descriptor parsing, Legacy/InternalV1 classification, valid internal
  replay/idempotency, missing-journal fail-closed, and maintenance barrier tests
  all PASS. The runtime no-containment v2 refusal branch was inspected, not
  independently rerun. The delegated ordinary-enrollment + startup fixture is
  ignored unless a disposable cgroup is explicitly delegated; it was NOT_RUN.
  Full C1 acceptance is not inferred.
- Test artifact SHA-256: `9f39a18ac8e87b90d4a11d7ecc0686ec55954df88a8fc53b45552af28d9acfba`.
  Captured logs and matching exit files are under
  `/tmp/tw2tg-act-{legacy_spec,legacy_facts,internal_repeat,v2_missing_journal,maintenance_gate}.{log,exit}`; all five captured rerun exits are `0`.

### Targeted test details

- Source: HEAD `9e8fc3055d052b5281546eff6a813a6da37c79d4`; branch
  `cross-platform/automatic-pairing-reconcile-20261002`; dirty docs-only delta;
  Linux x86_64 WSL2. Built `/home/shiraishi/VSCode Workspace/Tw2Tg/desktop/src-tauri`
  test target from current source with Cargo into `/tmp/tw2tg-act-target`.
- Test binary SHA-256:
  `9f39a18ac8e87b90d4a11d7ecc0686ec55954df88a8fc53b45552af28d9acfba`.
- Exact tests PASS (each isolated; exit 0):
  - `executor::tests::linux_launch_descriptor_is_explicit_bounded_and_fail_closed`
    (standalone command returned exit 0; its output was not separately persisted).
  - `executor::tests::startup_recovery_classifies_legacy_without_execution_spec_consistently`
    (`/tmp/tw2tg-act-legacy_spec.log`, `.exit` = 0).
  - `executor::tests::startup_recovery_classifies_legacy_without_recovery_facts_as_incomplete`
    (`/tmp/tw2tg-act-legacy_facts.log`, `.exit` = 0).
  - `executor::tests::startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`
    (`/tmp/tw2tg-act-internal_repeat.log`, `.exit` = 0).
  - `executor::tests::startup_v2_missing_journal_remains_fail_closed`
    (`/tmp/tw2tg-act-v2_missing_journal.log`, `.exit` = 0).
  - `executor::tests::maintenance_blocks_production_and_startup_recovery_before_mutation`
    (`/tmp/tw2tg-act-maintenance_gate.log`, `.exit` = 0).
- **Not validated in this batch:** descriptor-present ordinary enrollment and
  startup through the ignored delegated fixture
  `executor::tests::production_scheduler_publishes_protocol_fixture_bytes`; the
  exact filter reports one ignored test because no disposable delegated cgroup
  capability was provisioned. Do not count this as PASS or as proof of failure.
  The matching test source asserts descriptor-based runtime initialization,
  replacement retention, persisted ordinary submission, startup recovery and
  interrupted-attempt successor behavior; rerun only through its documented
  delegated setup when that capability is available.
- The descriptor parser test passed in a standalone run (exit 0; output not
  separately persisted). An initial filter omitted the module prefix and ran
  zero tests; it is not counted. The five captured reruns have individual logs
  and exit files. Terminal integration did not confirm the batch completion,
  so only tests with individual exit files are counted.

- **Documentation validation:** `node scripts/docs-audit.mjs` PASS (112 tracked
  Markdown files; zero dead links/orphans/stale branches); `git diff --check`
  PASS. No product files or dependencies changed. Full regression not selected.


# Linux crash/replay incremental validation — 2026-10-16

## 2026-10-17 candidate lifecycle reconciliation

Prior `C1-PRIVACY-CANDIDATE` finding is narrowed: Plan §5.6 explicitly authorizes a
separate minimum attempt-bound durable business candidate with lifecycle/cleanup;
the prior assertion that no exception exists missed that clause. It does not
explicitly establish that the complete portable DTO is the permitted minimum.
The full PREPARED text/author/relationship retention scope still needs resolution;
no C1 acceptance or new privacy authorization is inferred. The lifecycle fix below
is independent, does not add retained fields and does not change architecture.

Fixed indefinite duplicate retention: the typed candidate contains the metadata
needed to complete business facts (not raw Sidecar output); PREPARED retains it
across errors; finalization deletes it atomically with business facts/events and
COMMITTED. Failed transactions retain the exact candidate. COMMITTED replay uses
manifest/files/business facts read-only without requiring the retired copy.
Existing already-committed rows from older code are not mutated by replay. The
UI now distinguishes database archive data, temporary candidate and manifest.
SQLite row deletion is logical cleanup, not a claim of secure physical erasure.

Evidence, same branch/source HEAD and dirty state as active handoff:
- `/tmp/tw2tg-c1-candidate-red.*`: expected regression failure (candidate retained).
- `candidate-storage-final.*`: 154 PASS; `candidate-desktop-final.*`: 334 PASS /
  4 ignored; `candidate-delegated-final.*`: 4 PASS, serial disposable delegation.
- `candidate-ui-green.*`: 226 PASS; `candidate-ui-build.*`: build exit 0;
  `candidate-clippy-final.*`: exit 0, existing warnings retained.
All abbreviated artifact names above have `/tmp/tw2tg-c1-` prefix and `.log/.exit`.
Intermediate failures are retained: UI expected old disclosure; delegated r2/r3
exposed incomplete synthetic crash rollback, fixed by restoring candidate at every
PREPARED fixture reset; Desktop r3 failed compilation due to an unavailable direct
SQLite dependency (removed, no dependency added). Delegated r4 used the prior
binary after that compile failure and is not current-source evidence. Initial
`candidate-delegated` selected the application executable instead of the lib test
binary, produced GUI/tooling output and is NOT test PASS. Corrected executions
use Cargo's reported `xarchive_desktop_lib-3cde2dc33a044b66` test artifact.

Final current-file verification uses `candidate-{storage,desktop,delegated,clippy}-verified.*`:
154 PASS, 334 PASS/4 ignored, 4 delegated PASS, Clippy exit 0 respectively.
`candidate-fmt.*` and `candidate-docs.*` exit 0; final whitespace check PASS.
Desktop test SHA-256:
`f43035aa7a4fe63132cddf2efbacf5b594d25240e12f056ff82e9edff365bcd2`.
Final verification removed a speculative concurrent-re-read branch: intermediate
delegated errors proved synthetic crash resets missing candidates, not a concurrent
product reader defect. No unproven race-specific production change retained.
The terminal wrapper lost completion capture once; separate logs/exit files and
absence of active Cargo processes confirmed completion without blind reruns.

## Resumed current-source verification — 2026-10-17

- ID: `P6-LINUX-RESUMED-20261017`; priority P0; owner Cross-platform Owner;
  implementation IN_PROGRESS (integrated C1), verification PASS for the checks
  below only. Source `777e0b4cd58c825f9d5d21da6098932d62e0e43d`, branch
  `cross-platform/automatic-pairing-reconcile-20261002`; initially no tracked
  modifications, untracked `desktop/src-tauri/job-1.lock` preserved. Subsequent
  documentation edits do not alter the tested source. No commit/push/handoff.
- Target/expected: current-source persisted scheduling and batch retry preserve
  attempt binding; Storage replay verifies committed facts; explicit-v2 production
  fixtures publish and replay; successor recovery drains crashed-owner descendants;
  transfer/refresh fixtures preserve retained owners until cleanup succeeds.
- Environment: Linux x86_64 WSL2 kernel 6.18.54.1; user systemd running;
  Rust 1.98.0, Python 3.14.4; Cargo and systemd-run available. No aria2c or
  gallery-dl on PATH. Method: integration/delegated; prerequisites: disk-backed
  temporary fixtures, protocol fixture Python, disposable delegated cgroup.
- Steps/build origin (serial Cargo invocations):
  1. `cargo test -p xarchive-storage --lib`: 151 PASS.
  2. `cargo test -p xarchive-desktop --lib startup`: 8 PASS / 1 ignored.
  3. `cargo test -p xarchive-desktop --lib persisted_scheduler_binds_attempt`:
     1 PASS.
  4. `cargo test -p xarchive-desktop --lib batch::tests`: 12 PASS.
  Evidence: `/tmp/tw2tg-20261017-{storage,startup,scheduler,batch}.log` and matching
  `.exit` records, each zero. Ignored checks are not counted as PASS here.
- Delegated steps: for each following exact `executor::tests::` name, run the
  Desktop test executable with `--exact <name> --ignored --nocapture` via
  `timeout 60s systemd-run --user --wait --pipe --collect --property=Delegate=yes`:
  `production_scheduler_publishes_protocol_fixture_bytes`,
  `startup_recovery_drains_live_descendant_after_owner_sigkill`,
  `production_transfer_rpc_fixture_drains_recorded_writer`.
  Each PASS 1/1, service exit 0, shell exit 0. Evidence:
  `/tmp/tw2tg-20261017-<test-name>.log` and matching `.exit` records. These prove
  fixture-level production paths, not actual aria2/gallery-dl compatibility.
- Exact executables: `target/debug/deps/xarchive_desktop_lib-3cde2dc33a044b66`,
  SHA-256 `43fef2b222f84cbb382487f8c72494bb51d1fc9d51f17c8dd7645bf40575fe7c`;
  `target/debug/deps/xarchive_storage-b918884c3a09f4af`, SHA-256
  `21793832baa849782bce37d1af9b9e4301b916cd31667919e4d6d5f083c975dc`.
- Tooling: shell integration sometimes reports capture failure after commands
  finish; separate completion files and final logs were inspected. Existing
  compiler warnings were not hidden or converted into product changes.
- Remaining: production v2 enrollment/replay enablement is BLOCKED by the review
  explicitly reserved in ADR-015 clause 8 and Plan P6, not by an undecided runner
  architecture. D1/F2/F3 remain dependency-waiting in their approved order.
  Windows implementation/native/real-tool/GUI acceptance is NOT_RUN here because
  it is Windows-owned; Linux protocol fixtures are not equivalent evidence.
  blocks_development: true for those dependent feature paths; blocks_release:
  C1/P6 and dependent acceptance remain open. Full regression not run: no product
  source changed, affected Storage module and Desktop entry-point/lifecycle checks
  cover this reconciliation. Revalidate on related source/dependency/containment
  or environment changes. Follow-up: Cross-platform Owner reviews authorized
  enrollment/startup scope at `StorageJobPersistence::create_or_reuse` and
  `recover_startup`; preserve existing Legacy/InternalV1 behavior and fail-closed
  gates until then. Machine-local logs are not portable handoff artifacts.

- Owner: Cross-platform Owner; implementation: IN_PROGRESS; no formal handoff.
- Source: `2d42a2e965a6a2cd6b5685f2c869539dc5b9843c`, branch
  `cross-platform/automatic-pairing-reconcile-20261002`, plus uncommitted test
  changes in Storage and Desktop. Existing `production.rs` modifications retained;
  its diff SHA-256 is `8dbc2713b7efd5c62b0656d1072d2c307a74b53248e9379d0b9c0ad5369833d2`.
- Platform/environment: Linux x86_64, WSL2 kernel 6.18.54.1, user systemd active;
  Rust 1.98.0, Python 3.14.4. No Windows acceptance or aria2 execution.
- Method: integration, local Cargo test binaries; prerequisites: Python launcher,
  disposable user-systemd service with `Delegate=yes`, writable temporary storage.

## P5-LINUX-OWNER-CRASH (P0) — PASS

Target/expected: a recorded, populated writer domain survives owner SIGKILL;
fresh startup recovery drains/removes that domain before the writer deadline,
and no marker appears after that deadline. Repeated startup remains safe.

Steps:
1. Build with `cargo test -p xarchive-desktop --lib session`.
2. Run the resulting Desktop test binary through `systemd-run --user --wait
   --pipe --collect --property=Delegate=yes`, bounded by `timeout 40s`, with
   `--exact executor::tests::startup_recovery_drains_live_descendant_after_owner_sigkill
   --ignored --nocapture`.
3. Fixture launches through the production cgroup launcher, checks `populated 1`,
   kills/reaps the owner with SIGKILL, invokes a new runtime's `recover_startup`,
   checks removal within three seconds and absence of writes after nine seconds.

Evidence: `/tmp/tw2tg-p5-run-final.log`, 1/1 PASS, service and shell exit 0.
Desktop executable SHA-256:
`b960cdaf9ca9d4cfe6a3cfa959c98a09c1019f4d8880851f3f6cd69cbfcd63e9`.

## P5-STORAGE-PROCESS-REPLAY (P0) — PASS

Target/expected: before-PREPARED refuses unauthorized replay/publication;
persistent PREPARED, temporary rename, final rename, directory
commit and completed-finalize windows survive abrupt process termination;
reopening SQLite and replay converges, with repeated replay adding no events
and preserving media bytes.

Steps: `ulimit -c 0`; run `cargo test -p xarchive-storage --lib v2_forward_publish`.
The matrix re-executes the same test binary per window. The child uses existing
prepare/publish APIs and production failpoints, records reaching the window and
aborts without service/SQLite destructors. The parent reopens disk state.

Evidence: `/tmp/tw2tg-p5-storage-process.log`, 7/7 PASS, exit 0 (one matrix test
covers six windows). Storage executable SHA-256:
`e92b088317c8bfdd3f75619f4d83101f63cb6508ffbadaee92d3c88d6182668b`.
This is failpoint-boundary process-abort evidence, not mid-syscall or power-loss
evidence. It does not by itself prove Desktop entry-point end-to-end publication.

## Scope and follow-up

### Resumed reconciliation — 2026-10-16

- Source HEAD/branch and prior dirty scope verified unchanged. Prior Desktop and
  Storage final logs and binary hashes match the reconciliation below. Current
  environment remains Linux WSL2; `systemd-run` exists; neither `aria2c` nor
  `gallery-dl` is on PATH. No install, platform handoff or enablement performed.
- `P5-COMMITTED-DB-CORRUPTION`, P0, Cross-platform Owner, IMPLEMENTED/PASS:
  seven fresh disk-backed published fixtures mutate index, path, media ID, type,
  MIME, nullable size or nullable hash. Each recovery rejects database mismatch
  and preserves recorded DB facts, journal manifest/COMMITTED phase, events and
  media bytes. Build: `cargo test -p xarchive-storage --lib v2_committed_replay`.
  Initial compile failed (nonexistent event helper); corrected to existing
  `list_events_for_job`, then 3/3 PASS. Logs:
  `/tmp/tw2tg-continued-db-facts.log` and
  `/tmp/tw2tg-continued-db-facts-fixed.log`.
- Module regression: Storage **151 PASS**, exit record
  `/tmp/tw2tg-continued-storage.exit` = 0; log
  `/tmp/tw2tg-continued-storage.log`; artifact SHA-256
  `21793832baa849782bce37d1af9b9e4301b916cd31667919e4d6d5f083c975dc`.
  Adjacent Desktop startup **8 PASS / 1 ignored**, exit record
  `/tmp/tw2tg-continued-startup.exit` = 0; log
  `/tmp/tw2tg-continued-startup.log`; Desktop artifact unchanged from below.
  Shell integration again reported capture failure despite printed completion;
  separate exit files and finished logs were checked, not rerun blindly.
- Call-chain review: runner retains session and execution through attempt-fenced
  terminal writes; archive requires matching attempt, drained supervisors and
  verified staging before fenced Storage publish. Ordinary enrollment and v2
  startup gates remain unchanged. No test-count increase closes P6/C1.
- Remaining classification: ordinary v2 enrollment/startup replay is
  decision-required under ADR-015 clause 8 and Plan P6; D1/F2/F3 wait on C1 and
  approved dependency order; Windows adapter/native/real-tool acceptance remains
  Windows-owned. No independent production milestone remains executable within
  the current authorization. Follow-up: explicit enablement scope/prerequisite
  decision, then implement the two entry gates as an ordered batch; no new
  architecture proposal. Blocks release, not a claim of Windows blocking.
- Full workspace regression not run: continuation changes Storage tests only;
  Storage module and adjacent startup checks are the justified scope. Revalidate
  on changes to recovery, DB facts, manifest, publication or containment contracts.

### Continuation reconciliation: production milestones

Final reconciliation supersedes intermediate counts/hashes below:
Desktop **331 PASS / 4 ignored**, Storage **150 PASS**, exit 0
(`/tmp/tw2tg-desktop-final-reconciled.log`,
`/tmp/tw2tg-storage-final-reconciled.log`). Desktop SHA-256
`43fef2b222f84cbb382487f8c72494bb51d1fc9d51f17c8dd7645bf40575fe7c`;
Storage `f7c5f133ee89f5176962ad73c331e310b2308b70d7df4a956d370b6dcf2b1c59`.
Valid-journal startup rejection separately passes 1/1, service exit 0
(`/tmp/tw2tg-p6-startup-gate-real-journal.log`). Missing-journal startup rejection
passes 1/1 (`/tmp/tw2tg-p6-missing-journal-fixed.log`); its initial fixture expected
the later enablement error but correctly hit the earlier missing-journal error.
Retained-owner failed retry preserves the same supervisor address before a
successful retry, delegated 1/1 (`/tmp/tw2tg-p3-owner-retry-matrix.log`).
Removing only the new cfg(test) include reconstructs the original `production.rs`
diff SHA-256 `8dbc2713b7efd5c62b0656d1072d2c307a74b53248e9379d0b9c0ad5369833d2`;
the user's existing changes are byte-preserved. Untracked lock remains untouched.

**Exit-gate disposition:** P1/P2/P4 mechanics have session/owner/staging/fenced
publication evidence; P3 has production-function protocol-fixture evidence, not
real tools; P5 has six Storage abort windows and Desktop live-descendant successor
drain, not power-loss or arbitrary syscall proof. P6 entry convergence is covered
at scheduler/batch/startup seams, but user-facing v2 enrollment and startup replay
remain deliberately unenabled. ADR-015 clause 8 explicitly excludes production v2
enablement authorization. No further enablement edit is made under this approval;
Windows native adapter/real-tool acceptance remains separate. P6/C1 remains open.

- Source remains the HEAD/branch above, dirty and uncommitted. Implementation
  remains IN_PROGRESS; no formal handoff. New production change is bounded
  COMMITTED read-only verification (existing contract); no enablement/schema change.
  `production.rs` additionally includes a cfg(test, Linux)-only fixture helper;
  prior user logic is retained, not reverted or reformatted.
- **P5-COMMITTED-DB-FACTS (P0, Cross-platform) PASS:** COMMITTED replay now also
  checks directory, media count and index/path/size/hash against durable facts.
  Missing media rows reject read-only. The original positive fixture now uses
  atomic finalization, not a manually COMMITTED journal without DB facts. An
  intermediate module result was 148 PASS / 1 FAIL while that fixture was still
  incomplete; final Storage module is **150/150**, exit 0,
  `/tmp/tw2tg-p5-storage-db-integrity-fixed.log`.
- **P3-TRANSFER-PROTOCOL (P0, Cross-platform) PASS at fixture level:** actual
  `transfer_once` success, expired URL and cancellation; actual
  `transfer_with_one_refresh` success, changed media rejection and second-expired
  rejection. Real RPC Python child is launched with the production cgroup
  launcher. Empty nested cgroup forces a real removal failure: the same retained
  supervisor blocks refresh, then retries successfully after removing the blocker.
  Every addUri also spawns a setsid delayed writer; groups are absent and no
  delayed marker exists after its deadline. Delegated **1/1**, exit 0,
  `/tmp/tw2tg-p3-owner-matrix.log`. Protocol fixture is not real aria2 compatibility.
  Earlier fixture builds had import/borrow errors and a zero-valued network
  configuration; a failed-transfer cleanup initially removed the reused success
  filename. These were fixture defects, corrected without production changes.
- Final Desktop library **330 PASS / 4 ignored**, exit 0,
  `/tmp/tw2tg-desktop-continuation-final.log`. Ignored delegated tests are not
  counted as PASS: owner-SIGKILL and explicit-v2 publication/replay were separately
  rerun **1/1 each**, service exit 0 (`/tmp/tw2tg-p5-owner-reconfirmed.log`,
  `/tmp/tw2tg-p6-production-db-replay-final.log`). Formatter and diff check pass;
  full workspace regression not run: bounded Storage/Desktop change, both affected
  library modules exercised. Docs audit passes.
- Artifact SHA-256: Desktop `498f3b5482ce85da0c2231e25cc83c4958046d169f7e663806b5068da992b673`;
  Storage `e8da9b09e11ce468f5acd9ba0ddfadae00361336af0f22969cf065136d1870a6`.
  WSL2 Linux x86_64 and delegated systemd verified; `aria2c` is not installed.
  Reuse requires unchanged relevant source/dependencies/environment and artifact.
- Remaining P6/C1 is **IN_PROGRESS**, not PASS: ordinary submission still enrolls
  InternalV1, while fenced production publication requires v2; startup v2 still
  rejects explicitly. These user-facing gates must not be removed as a test fix.
  Next call-site review is production enrollment/startup enablement against ADR-015
  and the Plan, with Windows adapter/real-tool/native acceptance separately owned.

- **P6-BATCH-RETRY-SUCCESS (P1) — PASS:** first dispatch fails, paused dispatch
  admits nothing, retry/resume creates a distinct Job, attempt-owned execution
  succeeds and candidate becomes DONE. Targeted 1/1 and batch module 12/12,
  exit 0 (`/tmp/tw2tg-p6-batch-success.log`,
  `/tmp/tw2tg-p6-batch-success-module.log`). Fake execution, not actual transfer.
- **P6-PRODUCTION-PUBLISH-FIXTURE (P0) — PASS with explicit v2 enrollment:**
  production factory, real contained Python protocol writer, real attempt bytes,
  fenced publication, Complete, exactly-once terminal events, repeated COMMITTED
  replay and absent writer groups. Delegated test 1/1, exit 0
  (`/tmp/tw2tg-p6-production-replay.log`). This is protocol-fixture integration,
  not gallery-dl/aria2 compatibility or ordinary v2 enablement. Initial fixture
  used invalid `image` enum and wrong expected Job ID; subsequent ordinary v3
  submission correctly remained InternalV1 and failed v2 publication enrollment
  (`/tmp/tw2tg-p6-production-publish-v3.log`). Explicit enrollment is confined to
  the test; production gates were not changed. A later fixture compile failure
  swapped schedule arguments and was corrected. Executor regression 101 PASS,
  3 ignored, exit 0 (`/tmp/tw2tg-p6-executor-integrated.log`).
- **P5-COMMITTED-INTEGRITY (P0; CROSS_PLATFORM_REVIEW_REQUIRED) — PASS after fix:**
  inspection found COMMITTED replay returned a path without checking media.
  It now validates manifest and regular-file/size/hash facts read-only, rejecting
  equal-sized swapped bytes while leaving journal and bytes unchanged. New test
  1/1 PASS (`/tmp/tw2tg-p5-committed-integrity.log`). Initial v2 module run was
  26 PASS / 1 FAIL because the prior read-only fixture used a fabricated hash;
  corrected the fixture to actual SHA-256 of `abc`, without weakening assertions.
  V2 tests 27/27, Storage module 149/149, exit 0
  (`/tmp/tw2tg-p5-v2-fixed.log`, `/tmp/tw2tg-p5-storage-module-integrity.log`).
  This preserves the existing verification contract; no schema/API change.

- **P6-STARTUP-SUCCESS — PASS:** startup redispatch invokes an attempt-bound
  fake execution and reaches durable Complete with each terminal event exactly
  once. Targeted test 1/1, exit 0:
  `/tmp/tw2tg-p6-startup-success-fixed.log`. Initial test cleanup failed compilation
  due to a partial move; corrected without production changes. Executor module
  regression: 101 PASS, 2 ignored, exit 0 (`/tmp/tw2tg-p6-executor-final.log`);
  ignored checks are not claimed executed by this run. Storage rename recovery:
  9 PASS, exit 0 (`/tmp/tw2tg-p5-rename-final.log`). Latest Desktop executable:
  `522a54f48f8792bfb42a03b6cf8896caf426fcc62b46cfdcf97d0b430871b213`.
  Fake execution does not establish real media publication or aria2 acceptance.

- **P6-SCHEDULED-SUCCESS (P1) — PASS:**
  `cargo test -p xarchive-desktop --lib persisted_scheduler_binds_attempt`
  passed 1/1, exit 0, `/tmp/tw2tg-p6-success.log`. Disk-backed ordinary submission
  reaches `create_for_attempt` with attempt 1 in Downloading, reaches Complete,
  and persists exactly one DownloadCompleted and Completed event. The legacy
  factory method panics if invoked. Execution itself is a fake success port:
  this proves scheduler/session/finalization integration, not real-tool writes.
  Executable SHA-256 for this later build:
  `73dcc511ea529479e3dc74ce3d9c59002c43d9293090f7b1ec23223747496a08`.
  Terminal wrapper misreported capture failure after printing explicit exit 0;
  this is tracked separately from the test result.

- **P6-BATCH-REDISPATCH (P1): FAIL then PASS after fix;
  CROSS_PLATFORM_REVIEW_REQUIRED.** A new disk-backed regression dispatches a
  candidate, waits for durable executor failure, resets the failed candidate and
  dispatches again. The first draft had an argument-order compile error (exit
  101); after correction it reproduced a real UNIQUE request-ID violation (exit
  101). The submission adapter's fixed tweet request ID was reused by batch
  redispatch. Batch now assigns an independent random request ID using the
  existing `getrandom` dependency; no schema/API changes. Regression passes 1/1,
  batch tests 15/15, scheduling tests 2/2, exit 0. Logs:
  `/tmp/tw2tg-p6-redispatch.log`, `/tmp/tw2tg-p6-redispatch-diagnostic.log`,
  `/tmp/tw2tg-p6-redispatch-fixed.log`, `/tmp/tw2tg-p6-batch-fixed.log`,
  `/tmp/tw2tg-p6-schedule.log`. This proves retry redispatch reaches persisted
  scheduling and durable factory failure, not successful real-tool downloading.

- Adjacent startup recovery: 6 PASS, delegated test separately run; session
  boundary: 19 PASS; batch-related tests: 14 PASS. Logs respectively
  `/tmp/tw2tg-p5-targeted.log`, `/tmp/tw2tg-p6-session.log`,
  `/tmp/tw2tg-p6-batch.log`. Formatter and diff whitespace checks pass.
- Initial build had four existing warnings; no production warning cleanup done.
- Full regression not run: scope is targeted recovery tests and a small batch
  request-identity fix; adjacent batch/scheduling regressions cover the latter.
- blocks_development: false; blocks_release: overall C1/P5/P6 gates remain open.
- Revalidation: required if related source, dependencies, containment protocol,
  launcher, runtime recovery or environment changes. Logs are machine-local,
  not a portable formal handoff artifact.
- Follow-up: Cross-platform Owner continues P6 integrated entry-point coverage
  and reconciles remaining P5 acceptance. Windows Owner retains real aria2/native
  acceptance. No production v2 gate is removed.

## F3 record-import command + preview-accept UI — 2026-10-18

Implemented and exposed the approved F3 record import transaction acceptance
commands and preview-accept UI surface (branch/HEAD unchanged, dirty):

- `crates/xarchive-storage/src/record_import.rs` (new): derive serde for
  `AcceptedRecord`, `RecordImportCommitFailure`, `record_import::module`
  exports. Adds `import_accepted_records`, `commit_imported_record`
  (pub(crate)), `transactional_upsert_user`, `RecordImportOutcome`,
  `RecordImportCommitFailure`, `AcceptedRecord`.
- `desktop/src-tauri/src/commands.rs`: new Tauri commands:
  - `preview_record_import` — pure media-evidence preview; no durable writes.
  - `accept_record_import` — Linux-only, maintenance-guarded single-transaction
    import reusing `Database::import_accepted_records`.
- `desktop/src-tauri/src/lib.rs`: registers both commands in the invoke handler.
- `desktop/src/pages/preview-accept-page.jsx` (new): React preview-accept page
  with JSON textarea, media evidence preview, preview/accept buttons, busy
  state and messages.
- `desktop/src/main.jsx`: added `recordsJson`/`archiveRoot` state, `previewImport`/
  `acceptImport` invoke callbacks, page branch and sidebar `NavItem` for the
  new `record-import` page.

| Check | Result | Evidence |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | `/tmp/tw2tg-f3/fmt.log` exit 0 |
| `cargo check -p xarchive-storage` | PASS | `/tmp/tw2tg-f3/check-storage.log` exit 0 |
| `cargo check -p xarchive-desktop` | PASS | `/tmp/tw2tg-f3/check-desktop.log` exit 0 |
| `cargo check --workspace` | PASS | `/tmp/tw2tg-f3/check-workspace.log` exit 0 |
| `cargo test -p xarchive-storage record_import` | 5/5 PASS | `/tmp/tw2tg-f3/tests-storage.log` exit 0 |
| `cargo clippy -p xarchive-storage --lib --tests` | PASS (existing warnings only) | `/tmp/tw2tg-f3/clippy-storage.log` exit 0 |
| `cargo clippy -p xarchive-desktop --lib` | PASS (existing warnings only) | `/tmp/tw2tg-f3/clippy-desktop.log` exit 0 |
| `cargo test -p xarchive-desktop --lib` | 343/343 PASS, 6 ignored | `/tmp/tw2tg-f3/tests-desktop.log` exit 0 |
| `npm --prefix desktop run check` (Vite build) | PASS | `/tmp/tw2tg-f3/vite-build.log` exit 0 |
| `npm --prefix desktop test` | 226/226 PASS | `/tmp/tw2tg-f3/frontend-tests.log` exit 0 |

Evidence prefixes: `/tmp/tw2tg-f3-{storage-check,desktop-check,workspace-check,tests-storage,tests-desktop,clippy-storage,clippy-desktop,vite-build,frontend-tests,fmt}`.

Scope: F3 surface implemented, preview/accept command + UI wired. Live Windows
acceptance, real media publication and G remain unfinished.
### 2026-10-18 F3/G acceptance reconciliation continuation

Source base `14f465b48d999296fa3ed054af0c46de1d389476`, branch
`cross-platform/automatic-pairing-reconcile-20261002`, dirty, Cross-platform Owner.
Previous P1 source manifest compared against live files: code matched; only
handoff/history docs had changed. Unknown Desktop lockfiles preserved.

- Batch export uses persisted candidate scope without UI pagination, canonical
  metadata only, no fabricated Jobs. Job, Tweet and Batch scope are mutually
  exclusive. Download/archive JSON/YAML share single-record export validation.
- Archive preview and transaction use the same directory/ledger/media comparison;
  download preview includes ledger/root conflict checks. Export compares all media
  identity/type/MIME/path/size/hash facts and recomputes local evidence.
- Acceptance mapping: version/schema/privacy/JSON-YAML equivalence (Core and
  Desktop serialization tests); identity/root/media conflicts (Storage regression
  tests); SQL and accepted-prefix rollback (Storage); existing DB upgrade to 0020
  without record rewrite (Storage); cancellation registration (Desktop), admission/
  cancellable drain/crash lock release (Workflow maintenance tests); stale revision
  rejection after drain plus native import/export/reimport (Linux WDIO); changed
  local media preserves record (WDIO). No check implies Windows acceptance.
- Storage 172 PASS: `/tmp/tw2tg-next-storage-r5.{log,exit}`. Frontend 230 PASS:
  `/tmp/tw2tg-next-ui.{log,exit}`; production build exit 0:
  `/tmp/tw2tg-next-ui-build.{log,exit}`. Workflow maintenance 3 PASS:
  `/tmp/tw2tg-next-maintenance.{log,exit}`. Initial `workspace` filter selected zero
  tests and is explicitly not acceptance evidence.
- Desktop 351 PASS / 6 default ignored at `desktop-full-r4`; latest final run and
  fresh native artifact evidence are recorded below when complete. Earlier
  `desktop-r3` failed compilation because a test referenced an absent uuid crate;
  corrected with existing std facilities, no dependency added. Failure retained.
- Clippy exit 0 with existing warnings (`/tmp/tw2tg-next-clippy.{log,exit}`);
  subsequent cleanup removed new helper-only dead-code and needless-borrow warnings.
  Native r2 5 PASS (`/tmp/tw2tg-next-native-r2.{log,exit}`), including stale revision
  rejection and changed-file preview. Final artifact identity must use fresh run.
- No commit/push, immutable tested revision, formal handoff or ownership transfer.
  Windows apply adapter/native/NTFS/GUI are Windows-owned. ZIP/7z stays deferred.

Final completion records: `/tmp/tw2tg-next-desktop-final` 351 PASS/6 ignored;
`tw2tg-next-native-final-build` exit 0; `tw2tg-next-native-final` 5 PASS;
`tw2tg-next-core` 5 PASS; `tw2tg-next-fmt`, `tw2tg-next-diff`,
`tw2tg-next-docs` exit 0 (each has separate `.log`/`.exit`).
Tested executable SHA-256 `489cb09a40a34f4f16d20c1b00c5ad25319df4938f025b3e7c3c10c15437790d`.
Code fingerprint `ec7ac03cfb8ca4375bcd1e86225e72b3e7c973c32432ef2318dacf69e8255f93`;
source manifest `/tmp/tw2tg-next-source.json` (docs refreshed after reconciliation).
Stop boundary: commit/push authorization is absent; remaining platform adapter
and acceptance belong to Windows. This is not full F3/G or release completion.

### 2026-10-18 authorized Git synchronization

User authorized batched commits and push. Storage `81205ac`, Desktop source
`16b6be05bb587fa929ebfabaa2a9d18661b6d49c`. All non-document blobs in the tested manifest
`/tmp/tw2tg-next-source.json` match this immutable revision. Existing test evidence
is content-bound reuse, not a post-commit rerun. The following docs commit records
this identity and supersedes the previous authorization blocker. Unknown local
lockfiles remain untracked/excluded. No Windows PASS or ownership transfer.
## Historical — 2026-10-19 C1 shared crate baseline (Linux, targeted)

Historical baseline record; the active 2026-10-19 calibration above adds
artifact-bound isolated tests and supersedes its earlier active status.

- Workspace: `cross-platform/automatic-pairing-reconcile-20261002`, HEAD
  `9e8fc3055d052b5281546eff6a813a6da37c79d4`; Linux x86_64 under WSL2; same
  pre-existing `.gitignore` edit, preserved.
- Scope: C1/B3-B7 shared implementation evidence. Targets:
  - `xarchive-workflow` library: 38/38 PASS.
  - `xarchive-storage` library: 172/172 PASS.
  - `xarchive-desktop` library: 351/351 PASS, 6 ignored (ignored are not counted).
  - `xarchive-download` library: 53/53 PASS.
  - `xarchive-sidecar-supervisor` library: 16/16 PASS.
- Resolved (PASS): the `xarchive-workflow` cross-process integration tests
  (`coordination`, 11/11) and (`execution_session`, 5/5) now pass. The earlier
  `spawn: "No such file or directory"` failure was a test-harness prerequisite,
  not a product defect: the `xarchive-lock-probe` `[bin]` is behind the
  `__lock_probe` feature and is not built by a plain `cargo test -p xarchive-workflow`.
  Reproducible unblock: `cargo build -p xarchive-workflow --features __lock_probe
  --bins && cargo test -p xarchive-workflow --test coordination -- --test-threads=1`
  and the same for `execution_session`.
- No product code changed. `node scripts/docs-audit.mjs` PASS; `git diff --check`
  PASS. Full product regression not run (no product source or dependencies changed).

## 2026-10-18 authorized Git synchronization
