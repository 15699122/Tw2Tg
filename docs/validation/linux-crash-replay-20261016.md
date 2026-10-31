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
