# Windows F3 native import batch — 2026-10-18

Owner: Windows Platform Owner. State: `CROSS_PLATFORM_REVIEW_REQUIRED` after
Git return. Input: remote `dev` at `9e8fc3055d052b5281546eff6a813a6da37c79d4`.
Branch: `codex/windows-f3-native-import-20261018`.

## Cross-platform reconciliation — 2026-10-18

Fetched and reconciled Windows return at `e7a75aaf5c6e61f53b26ae39880feb141059d145`;
final product source remains `dc731cce8a1b696af1dea00c3ad491b3e2ecc045`.
No Windows finding requires a shared-contract change. Reviewed the requested
mutex-release/recheck, Windows cfg/types, reparse/path handling and fixture changes;
these preserve the existing abstractions, so the review disposition is accepted for
this batch. Linux Storage module validation: `cargo test --locked -p xarchive-storage`
PASS 172/172. `cargo fmt --all -- --check` PASS. Workflow coordination integration
tests: 4 PASS, 7 FAIL because subprocess fixtures could not spawn the test executable
(`Os { code: 2, kind: NotFound }`) in this WSL environment; classify as tooling/test
environment failure, not a product failure. Executor/containment follow-up remains
to be rerun with a working subprocess fixture; no Full regression was run. The
current worktree's `cargo test --locked -p xarchive-workflow executor` matched zero
tests and is not counted as coverage.

Windows disposition remains scoped: NTFS/import/GUI checks have artifact-bound PASS
receipts; transaction I/O fault matrix and complete GUI matrix are NOT_RUN; driver
foreign-port cleanup ownership is FAIL and requires Windows tooling follow-up; C1
native no-replace/restart work remains PLANNED/NOT_RUN. No whole-queue WINDOWS_PASS.
Next Owner: Windows Platform Owner for those remaining Windows-only items after this
Linux integration handoff. Handoff source is the commit containing this update.
Full regression not run: bounded shared integration review, with affected Storage
tests and formatting selected; workflow subprocess-based tests exposed an environment
failure and should be repeated separately.

## Scope and completion

1. Fetch and inspect formal workspace: completed; exact remote baseline matched,
   tracked tree initially clean, older branches had no unpublished commits after
   their remote refs were fetched. Local ignored/untracked caches, artifacts and
   user data were retained. No direct filesystem synchronization.
2. Reconcile five historical branches: completed, 14 unique commits inspected.
   Preserve branch heads; do not merge old queues, handoff cursors or obsolete
   implementations. Restore only missing original validation receipts.
3. Implement Windows F3 adapter: completed. OS-released file locks implement
   existing coordination/maintenance ports. Admission spans existing production
   and recovery writers; drain supports cancellation. Local path handles reject
   reparse traversal/ADS and prevent root replacement/media mutation during apply.
   The existing versioned documents, selection semantics and transaction remain.
4. Validate incrementally: native NTFS, affected import modules, native GUI,
   exact Full executable, driver cleanup, component inventory and hashes. Detailed
   observations, failed attempts and unexecuted acceptance stay in the
   [results](../validation/windows-f3-20261018-results.md).
5. Return through Git: implementation commits, then evidence/docs commit on the
   named branch; push and compare remote branch SHA. Next Owner: Cross-platform
   Owner for review/integration; Windows retains the queue's remaining acceptance.

## Ownership and remaining work

`CROSS_PLATFORM_REVIEW_REQUIRED`: small shared integration changes preserve the
existing abstractions: release RuntimeState mutex during drain, recheck workspace
afterward, Windows cfg/type wiring, reparse detection, portable path serialization,
and platform-correct fixtures. Existing `same-file` 1.0.6 is reused as a Windows
dependency; no new dependency version, schema, migration or protocol is introduced.
Linux must revalidate its existing import/maintenance/executor modules before
integration. No shared-contract redesign was attempted.

Windows C1 no-replace publication/containment and restart matrix remain separate
implementation/acceptance work. Full regression was not run: this is a bounded
native adapter batch; affected modules were run and their broader failures are
retained. ZIP/7z record exchange remains deferred. Building a portable ZIP does
not enable record-exchange archive import.
