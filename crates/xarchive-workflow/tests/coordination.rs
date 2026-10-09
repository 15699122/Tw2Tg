//! Cross-process coordination tests.
//!
//! `fcntl` record locks are per-process, so two acquisitions inside one test
//! process never contend. Every exclusivity claim here is therefore proved
//! with a real second OS process (`xarchive-lock-probe`), built only with
//! the `__lock_probe` feature and never linked into production code. These
//! tests are Unix-only; the Windows Owner validates NTFS behaviour on
//! Windows with the native adapter.

#![cfg(unix)]

use std::io::{BufRead as _, BufReader, Read as _};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use xarchive_workflow::coordination::{Coordinator, LockError, LockScope, unix::LinuxCoordinator};

/// Unique lock directory for one test.
fn lock_directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "xarchive-workflow-{}-{}-{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("create lock directory");
    directory
}

/// Path of the built probe binary: `<target>/debug/deps` -> `<target>/debug`.
fn probe_binary() -> PathBuf {
    let test_exe = std::env::current_exe().expect("test executable path");
    let deps = test_exe.parent().expect("deps directory");
    let profile = deps.parent().expect("profile directory");
    profile.join(format!(
        "xarchive-lock-probe{}",
        std::env::consts::EXE_SUFFIX
    ))
}

/// Spawn the probe and wait until it prints `READY` (it holds the lock) or
/// exits (it failed to acquire). Returns the child on success.
///
/// Spawning and waiting live in separate helpers so every `Child` has an
/// unambiguous owner: the `READY` path returns it to the caller (who later
/// calls `wait`), and the early-exit path waits inline before panicking.
fn spawn_probe(lock_dir: &std::path::Path, scope_args: &[&str], mode: &str) -> Child {
    let mut command = Command::new(probe_binary());
    command
        .arg(mode)
        .arg(lock_dir)
        .args(scope_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn lock probe");
    let stdout = child.stdout.take().expect("probe stdout");
    match wait_for_ready(&mut child, stdout) {
        Ready::Holding(stderr) => {
            child.stdout = None;
            child.stderr = None;
            drop(stderr);
            child
        }
        Ready::ExitedEarly { status, err_text } => {
            panic!("probe exited before READY ({status:?}): {err_text}");
        }
    }
}

enum Ready {
    Holding(std::process::ChildStderr),
    ExitedEarly {
        status: std::process::ExitStatus,
        err_text: String,
    },
}

/// Block until the probe prints `READY` or its stdout closes. Always waits
/// on the child before returning the early-exit variant, so no path leaks
/// a zombie process.
fn wait_for_ready(child: &mut Child, stdout: std::process::ChildStdout) -> Ready {
    let stderr = child.stderr.take().expect("probe stderr");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        line.clear();
        let read = reader.read_line(&mut line).expect("read probe output");
        if read == 0 {
            drop(reader);
            let mut err_text = String::new();
            let _ = BufReader::new(stderr).read_to_string(&mut err_text);
            let status = child.wait().expect("probe exit");
            return Ready::ExitedEarly { status, err_text };
        }
        if line.trim() == "READY" {
            drop(reader);
            return Ready::Holding(stderr);
        }
        assert!(
            Instant::now() < deadline,
            "probe did not become ready in time"
        );
    }
}

/// Second process holds the job lock: this process must see contention, and
/// must acquire it after the holder exits.
#[test]
fn second_process_observes_job_contention_and_release() {
    let lock_dir = lock_directory("job-contention");
    let scope = LockScope::Job("job-7".to_owned());
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let mut holder = spawn_probe(&lock_dir, &["job", "job-7"], "hold");
    assert!(
        matches!(coordinator.acquire(&scope), Err(LockError::Contended)),
        "a live second process holds the job lock"
    );

    drop(holder.stdin.take());
    let status = holder.wait().expect("probe exit");
    assert!(status.success(), "probe exits cleanly: {status:?}");
    coordinator
        .acquire(&scope)
        .expect("lock released with holder");
}

/// A holder that exits without releasing must not keep a successor out;
/// the kernel drops the record with the process.
#[test]
fn crashed_holder_does_not_block_successor() {
    let lock_dir = lock_directory("crash-recovery");
    let scope = LockScope::Job("job-9".to_owned());
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let mut crasher = spawn_probe(&lock_dir, &["job", "job-9"], "crash");
    drop(crasher.stdin.take());
    let status = crasher.wait().expect("probe exit");
    assert_eq!(
        status.code(),
        Some(7),
        "probe exits via the crash path: {status:?}"
    );
    coordinator
        .acquire(&scope)
        .expect("successor acquires after crash");
}

/// Two different jobs proceed in parallel while the same job stays
/// exclusive: the second process holds job-A, this process still takes
/// job-B but not job-A.
#[test]
fn different_jobs_proceed_while_same_job_stays_exclusive() {
    let lock_dir = lock_directory("job-parallel");
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let mut holder = spawn_probe(&lock_dir, &["job", "job-a"], "hold");
    assert!(matches!(
        coordinator.acquire(&LockScope::Job("job-a".to_owned())),
        Err(LockError::Contended)
    ));
    coordinator
        .acquire(&LockScope::Job("job-b".to_owned()))
        .expect("different job is not blocked");

    drop(holder.stdin.take());
    let status = holder.wait().expect("probe exit");
    assert!(status.success(), "probe exits cleanly: {status:?}");
    coordinator
        .acquire(&LockScope::Job("job-a".to_owned()))
        .expect("job-a released with holder");
}

/// Destination scope is exclusive across processes: two jobs resolving to
/// the same final path cannot commit at once.
#[test]
fn destination_scope_is_exclusive_across_processes() {
    let lock_dir = lock_directory("dest-contention");
    let destination = PathBuf::from("/archive/final/job-x");
    let scope = LockScope::Destination(destination);
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let mut holder = spawn_probe(&lock_dir, &["dest", "/archive/final/job-x"], "hold");
    assert!(
        matches!(coordinator.acquire(&scope), Err(LockError::Contended)),
        "two jobs must not commit one destination at once"
    );

    drop(holder.stdin.take());
    let status = holder.wait().expect("probe exit");
    assert!(status.success(), "probe exits cleanly: {status:?}");
    coordinator.acquire(&scope).expect("destination released");
}
