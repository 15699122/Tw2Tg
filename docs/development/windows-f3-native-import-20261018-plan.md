# Windows F3 native import batch — 2026-10-18

Owner: Windows Platform Owner. State: `CROSS_PLATFORM_REVIEW_REQUIRED` after
Git return. Input: remote `dev` at `9e8fc3055d052b5281546eff6a813a6da37c79d4`.
Branch: `codex/windows-f3-native-import-20261018`.

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
