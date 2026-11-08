# desktop-commands-jobs

- **Path / entry:** `desktop/src-tauri/src/commands/jobs.rs`; reached through
  `crate::commands::jobs`, declared by `commands.rs` as `pub(crate) mod jobs;`.
- **Responsibility:** The job read-model surface — recent jobs, paged jobs, history
  index, per-job download metrics and aggregate metrics.
- **Owner:** Cross-platform.
- **Depends on:** `crate::RuntimeState`, `xarchive_storage::Database`
  (`list_recent_jobs`, `list_jobs_page`, `job_history_index`,
  `job_download_metrics`, `job_metrics`) and `crate::logging::redact_line`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, naming
  `commands::jobs::<name>`. The parent's `frontend_diagnostic_tests` also imports
  `redact_job_errors` from here.
- **Public contract:** the 5 registered command names, `ListJobsPageRequest`'s field
  names, the returned read-model shapes and every error string (including
  `"invalid job id"` and the lock-poison message). Unchanged by the move.
- **Why this is not part of `commands/executor.rs`:** these commands read the
  persisted read-model directly and never take the executor service handle, so they
  do not participate in admission, claim, drain or finalize.
- **Redaction boundary preserved:** `list_jobs` and `list_jobs_page` run
  `redact_job_errors` with the configured log secrets before returning, so an error
  URL is never projected to the frontend with embedded credentials.
- **Visibility note:** `redact_job_errors` is `pub(crate)` because the parent's
  redaction test asserts it. This is a deliberate, minimal widening caused by the
  test moving relative to the function; the alternative was duplicating the test.
- **State and ownership:** reads `RuntimeState.database` when the runtime already
  holds a handle, otherwise opens the executor database path for that one call.
- **Failure behaviour:** fail closed with a redacted `String` error; a job id that
  is empty or longer than 200 characters is refused before any database access.
- **Tests:** the parent's `frontend_diagnostic_tests` keeps asserting the redaction
  behaviour; the paged and metric projections are covered through Storage tests.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`.
- **Exceptions:** none; 123 lines.
- **Open questions:** none.
