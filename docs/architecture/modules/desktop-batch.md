# desktop-batch

- **Path / entry:** `desktop/src-tauri/src/batch.rs` plus child modules
  `batch/{discovery,selection,submission,dispatch}.rs`.
- **Responsibility:** decide which Tweets of an account batch may be archived, then
  dispatch them within the batch's filter, limit and cancellation rules.
- **Owner:** Cross-platform Owner.
- **Depends on:** `xarchive_storage` (batch rows, candidate rows, archive facts),
  `crate::production` discovery, `crate::executor` submission adapter.
- **Depended on by:** `crate::commands` (`BatchFilters`, `spawn_batch_dispatch`).
- **Public contract:** unchanged. `crate::batch::BatchFilters` and
  `crate::batch::spawn_batch_dispatch` keep resolving through `pub(crate) use`.
- **Compatibility entry:** `mod batch;` is crate-private; the parent re-exports the
  pre-existing crate-visible surface.
- **State and ownership:** the dispatch coordinator owns the worker thread and the
  cancellation token; `BatchCancellationGuard::drop` unregisters it, and a failed
  spawn unregisters too.
- **Failure behaviour:** a full queue leaves the candidate PENDING and stops the
  pass; a submit failure is recorded per candidate and never retried implicitly; a
  batch whose state is no longer ACTIVE is never dispatched.
- **Constraints:** filter order, batch limit, already-archived detection, execution
  spec inheritance and cancellation cleanup are unchanged. A skip still means only
  "a committed directory holds every recorded media file", never "download is
  complete".
- **Tests:** `batch::d1_batch_tests` and `batch::tests` (13 tests, 0 ignored),
  unmoved.
- **Gates to re-run:** batch module regression; Desktop lib regression.
- **Exceptions:** none.
- **Related:** [execution spec](execution-spec.md),
  [desktop executor runtime](desktop-executor-runtime.md).
