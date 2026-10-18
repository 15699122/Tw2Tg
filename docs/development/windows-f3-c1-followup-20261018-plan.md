# Windows F3 / C1 follow-up plan — 2026-10-18

Input handoff: `2f0f140558f1be57293886216298484c1902e9b7` on
`codex/windows-f3-native-import-20261018`. Execution branch:
`codex/windows-f3-c1-followup-20261018`. Windows Platform Owner.

## Executed milestones

1. Fetch exact remote, inspect tracked/unpushed state, fast-forward formal workspace
   through Git, create current branch. No filesystem synchronization; old artifacts
   and unrelated untracked work remain intact.
2. Fix WQ-ENG-DRIVER-FOREIGN-OWNER: implementation `25c6ef6` plus final correction
   `9dc3762`, final ownership-failure guard `0a9a6b6`. Launcher-owned ChildProcess handles authorize root identities;
   descendants must match parent lineage, executable and creation time. Snapshot
   before upstream teardown, recheck before cleanup, never authorize by port owner.
   Unknown/foreign listeners are preserved. Real native and occupied-port recipes
   verify this; earlier FAIL remains in the original receipt.
3. F3 fault matrix: native NTFS journal-create denial with exact ACL restoration;
   native SQLite capacity exhaustion at selected index1; complete rollback and
   retry. Existing production Coordinator admission and real startup recovery
   paths tested under import drain/cancellation/release. No test-only production
   command, schema or runtime fault switch was added.
4. F3 GUI matrix: exact handed-off Full release executable, independent new native
   WDIO execution. Job/Tweet/Batch × JSON/YAML × download/archive, Batch41, original
   save files, input invalidation, canonical conflicts, root/media stale rejection.
   Current page has text inputs/Blob download and no file chooser; unsupported
   chooser/Save As interaction is NOT_APPLICABLE to this existing UI.
5. C1 call-chain review: implementation remains PLANNED and native C1 acceptance
   NOT_RUN. The shared boundary decision below is required before safe production
   implementation; this is not a Windows capability/tool failure.

Detailed [results and evidence](../validation/windows-followup-20261018-results.md)
own counts, artifacts and intermediate failures. Full regression not run: Windows
tooling implementation and Windows-only tests/fixtures are the changed scope.

## C1 required cross-platform decision

State: `CROSS_PLATFORM_CHANGE_REQUIRED`. Keep current C1 gates closed.

Existing generic `NoReplaceMover` and `WriterContainment` ports must remain.
The latter only models start/drain lifecycle; production supervision and recovery
still concretely take `LinuxContainment` / `LinuxExecutionContainment`, Linux
launch provisioning and cgroup identity. Windows has no approved production launch
descriptor or durable Job Object recovery identity at these call sites:

- `crates/xarchive-workflow/src/writer_lifecycle.rs`: generic lifecycle, not the
  production spawn/stdio/provenance/recovery interface.
- `crates/xarchive-sidecar-supervisor/src/lib.rs`: Linux-only containment owner and
  spawn methods; `crates/xarchive-download/src/supervisor.rs` has equivalent
  Linux-specific ownership (follow actual source paths from repository map).
- `desktop/src-tauri/src/executor.rs`: production factory, explicit provisioning
  and startup recovery use Linux-specific concrete types; Windows v2 stays gated.
- `Cargo.toml`: `unsafe_code = "forbid"`. No approved safe native Windows layer
  for atomic no-replace rename and Job Object creation/attachment/drain exists in
  the current Workflow dependencies. Do not override the workspace safety policy
  or introduce a new shared dependency/launch contract unilaterally.

Cross-platform Owner next actions:

1. Approve the safe Windows API binding/dependency boundary while preserving the
   generic mover and workspace unsafe prohibition. No-replace needs native atomic
   file **and directory** rename, collision preservation and cross-volume failure;
   copy/delete, hard-link staging or overwrite-then-check are not substitutes.
2. Specify how production supervisors receive the platform containment owner,
   launch pipes/environment and attempt/writer identity; define Windows explicit
   provisioning and durable recovery identity/validation. Avoid treating a PID or
   named object alone as ownership, as the driver defect demonstrates.
3. Return that committed shared decision/port to Windows. Windows implements native
   Job Objects, start-before-writer admission, kill-on-close/drain, no-replace and
   the required restart/crash/publication matrix; Linux validates its retained
   adapter. Do not enable Windows v2 merely to run a fixture.

Normative native API references (reference only, no implementation copied):
[MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw),
[Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
These describe native mechanisms, not accepted implementation or current PASS.
Safe-wrapper review also checked
[TempPath3.27.0](https://docs.rs/tempfile/3.27.0/tempfile/struct.TempPath.html)
and [win32job2.0.3 Job](https://docs.rs/win32job/2.0.3/win32job/struct.Job.html).
The former's public noclobber contract explicitly disclaims general atomicity;
the latter exposes process-handle assignment but does not supply this project's
production launch/provenance/recovery contract. Neither was added as a dependency
or treated as a validated drop-in. Native SDK binding or a new trusted helper must
be chosen at the shared boundary; an extra runtime helper would itself be a writer
requiring containment and packaging ownership, not a shell workaround.
ZIP/7z record exchange remains deferred.

## Cross-platform reconciliation — 2026-10-19

Received and verified Windows return `9acd84dce3d1c986a542412c70f5a3fb042747a4`;
final native implementation source remains `0a9a6b612b654fbdbd59099af5db3c28f05f2815`.
The prior Linux local-only commit `9e8fc3055d052b5281546eff6a813a6da37c79d4` is an
ancestor of the return and was retained through Git history. No filesystem mirroring
was used.

### Driver tooling review (`CROSS_PLATFORM_REVIEW_REQUIRED`)

Accepted as a Windows-owned tooling fix, not product code. On Windows, authority
starts from a live driver-pool `ChildProcess` and matching executable identity;
descendants are selected through parent lineage and creation-time ordering, then
PID/creation-time/executable identity is rechecked before termination. Port listeners
are diagnostic only and never grant process ownership. If the ownership snapshot
fails, the error is surfaced without a port-based kill fallback; upstream still gets
its own direct-child teardown in `finally`. The original foreign-owner FAIL remains
in the original Windows receipt; the new ownership PASS is a separate result.
The upstream blank-Edge startup BLOCKED result is independent and is not satisfied
by successful cleanup.

### C1 shared boundary decision (`CROSS_PLATFORM_CHANGE_REQUIRED` remains open)

Source review found no production-ready portable binding to implement merely by
adding a Windows adapter. Existing `NoReplaceMover` is the right semantic port
(`xarchive-workflow/src/coordination.rs`), but Storage call sites still select the
Linux adapter and reject non-Unix; it does not yet route a production Windows mover.
Existing `WriterContainment`/`WriterLifecycle` expresses start/drain state but is
not connected to production Sidecar/aria2 spawning or Desktop recovery. Production
factory, per-writer launch provisioning and durable successor recovery identity are
still Linux-specific (`desktop/src-tauri/src/executor.rs`,
`crates/xarchive-sidecar-supervisor/src/lib.rs`, and
`crates/xarchive-download/src/supervisor.rs`). Thus neither a new trait stub nor a
platform dependency alone would satisfy the missing lifecycle.

Decision: preserve `NoReplaceMover`, `WriterContainment`, `AttemptIdentity`, the
workspace `unsafe_code = "forbid"` lint and the existing v2 fail-closed gate. Do not
add a Windows-only dependency or contract until its safe API boundary is selected and
reviewed. The cohesive shared design required before Windows native implementation
must bind the claimed attempt and writer role to (1) explicit provisioning,
(2) containment-before-spawn with owned pipes/environment and retained owner, (3)
drain/termination evidence, and (4) crash-surviving, validated successor recovery
identity. The publication port must preserve atomic no-replace semantics for files
and directories, collision refusal and cross-volume failure; a copy/delete helper is
not an adapter. Existing persistence schema does not currently need migration for
this boundary; do not introduce durable fields unless design proves OS-native
recovery identity must survive process death and defines backward-compatible
versioning. Until then Windows C1 remains PLANNED/NOT_RUN and v2 remains gated.

Linux checks at return source `9acd84d`: fmt PASS; `writer_lifecycle` 2/2;
`xarchive-workflow --test no_replace` 2/2; Storage `record_import` 9/9. Windows
tooling Node tests were attempted but BLOCKED before test collection because this
worktree lacks `@wdio/tauri-service`; this is not a product failure. Full regression
NOT_RUN: bounded interface/call-graph review and affected Linux module tests only.
Exact Windows F3 evidence and source/artifact identities remain in the linked result
and receipt; no Windows PASS was reclassified or inherited.

Next: Windows Owner implements the approved native adapters only after the shared
production lifecycle decision is completed; then execute the native NTFS writer,
drain, recovery, no-replace, restart and crash matrix. Keep ZIP/7z deferred.
