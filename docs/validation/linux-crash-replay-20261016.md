# Linux crash/replay incremental validation — 2026-10-16

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