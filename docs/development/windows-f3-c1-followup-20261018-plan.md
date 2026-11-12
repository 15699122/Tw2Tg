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
