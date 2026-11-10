# download-supervisor

- **Path / entry:** `crates/xarchive-download/src/supervisor.rs` plus test modules
  `supervisor/tests/{secret_lifecycle,command_line,debug_redaction,network_proxy}.rs`.
- **Responsibility:** spawn and supervise the aria2 process: build its command line,
  write the RPC secret to a private temp file, wait for readiness, terminate the
  process tree, and remove the secret on every exit path.
- **Owner:** Cross-platform Owner for the shared supervision logic and command-line
  construction; Windows console-hiding behaviour is Windows-owned.
- **Depends on:** `std::process`, the network/proxy configuration section, and the
  platform process-group helpers.
- **Depended on by:** the executor's download path and the archive retention path.
- **Public contract:** unchanged. `Aria2Supervisor`, `Aria2StartupFailure`,
  `TELEGRAM`-independent timeout defaults, and the `Debug` redaction are untouched.
- **Boundary that must not move:** `spawn` / `wait_for_aria2` / `terminate_process_tree`
  / `Drop` are one ownership unit — the process handle, the secret file and the
  cleanup obligation must stay together, so the production code is not split.
- **State and ownership:** the supervisor owns the child, its process group and the
  secret file path; `Drop` performs tree termination and secret removal.
- **Failure behaviour:** unchanged. A startup failure keeps the owner able to retry
  real secret cleanup; a failed secret removal is reported and retryable.
- **Tests:** 14 tests moved into four groups (secret lifecycle 3, command line 4,
  debug redaction 1, network/proxy 6). The `config` helper stayed in the parent
  `mod tests`. One call site was rebound from `super::super::tests::` to
  `super::super::super::tests::` because the test moved one module deeper; without
  that the crate does not compile.
- **Gates to re-run:** `cargo test -p xarchive-download --lib` (53/0/0).
- **Exceptions:** none.
- **Open questions:** real `aria2c` acceptance remains deferred and was not run.
