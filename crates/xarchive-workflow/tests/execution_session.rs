//! Cross-process execution-session tests.
//!
//! These prove the B3 session contract against a real second OS process, not
//! only against in-process guard handles. A session held in another process
//! must make the same job contended here, different jobs must still proceed in
//! parallel, the same destination must serialise two different jobs, the guard
//! must be released on every exit path including a crash, and a stale attempt
//! must be refused.
//!
//! The probe is `xarchive-lock-probe` in `session` mode, built only with the
//! `__lock_probe` feature. These tests are Unix-only: the Windows Owner
//! validates NTFS session behaviour with the native adapter.

#![cfg(unix)]

use std::io::{BufRead as _, BufReader, Read as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use xarchive_workflow::coordination::unix::LinuxCoordinator;
use xarchive_workflow::{
    AttemptIdentity, ClaimOutcome, ExecutionReservation, ExecutionSession, SessionError,
};

fn lock_directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "xarchive-session-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("create lock directory");
    directory
}

fn probe_binary() -> PathBuf {
    let test_exe = std::env::current_exe().expect("test executable path");
    let deps = test_exe.parent().expect("deps directory");
    let profile = deps.parent().expect("profile directory");
    profile.join(format!(
        "xarchive-lock-probe{}",
        std::env::consts::EXE_SUFFIX
    ))
}

fn attempt_of(job_id: &str, attempt: u32) -> AttemptIdentity {
    AttemptIdentity::new(job_id, attempt).expect("valid attempt")
}

/// Take the scopes and claim `attempt`, the way production does.
///
/// The tests use the two-phase entry point rather than any single-step
/// constructor, so what they prove is the sequence a runner performs: hold the
/// Job and destination scopes first, run the durable claim under them, and only
/// then authorize writes.
fn claim_session<'coordinator, C: xarchive_workflow::coordination::Coordinator>(
    coordinator: &'coordinator C,
    job_id: &str,
    destination: &str,
    attempt: u32,
) -> Result<ExecutionSession<'coordinator, C>, SessionError> {
    ExecutionReservation::acquire(coordinator, job_id, Path::new(destination)).and_then(
        |reservation| reservation.claim(|| ClaimOutcome::Claimed(attempt_of(job_id, attempt))),
    )
}

/// Spawn a probe holding a real session and wait for `READY` plus the `HELD`
/// line, returning the child and what it reported holding.
///
/// Waiting for `HELD` rather than only `READY` is what makes the contention
/// result meaningful: the test then knows the second process established the
/// exact session it asked for, so a later `Contended` cannot be an artifact of
/// the probe having acquired something else.
fn spawn_session_probe(
    lock_dir: &Path,
    mode: &str,
    job_id: &str,
    destination: &str,
    attempt: u32,
) -> (Child, String) {
    let mut command = Command::new(probe_binary());
    command
        .arg(mode)
        .arg("session")
        .arg(lock_dir)
        .arg(job_id)
        .arg(destination)
        .arg(attempt.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn session probe");
    let stdout = child.stdout.take().expect("probe stdout");
    let stderr = child.stderr.take().expect("probe stderr");

    let mut reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);
    let mut ready = String::new();
    let mut held = String::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        ready.clear();
        let read = reader.read_line(&mut ready).expect("read probe stdout");
        if read == 0 {
            let mut error_text = String::new();
            let _ = stderr_reader.read_to_string(&mut error_text);
            let status = child.wait().expect("probe exit");
            panic!("session probe exited before READY ({status:?}): {error_text}");
        }
        if ready.trim() == "READY" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "session probe did not report READY in time"
        );
    }
    loop {
        held.clear();
        let read = reader.read_line(&mut held).expect("read probe HELD line");
        if read == 0 {
            panic!("session probe closed stdout before reporting what it held");
        }
        if held.trim_start().starts_with("HELD ") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "session probe did not report what it held in time"
        );
    }
    drop(reader);
    drop(stderr_reader);
    child.stdout = None;
    child.stderr = None;
    let described = held.trim().to_owned();
    (child, described)
}

/// Release a probe cleanly and require a clean exit.
fn release_probe(mut child: Child) {
    drop(child.stdin.take());
    let status = child.wait().expect("probe exit");
    assert!(status.success(), "probe exits cleanly: {status:?}");
}

/// A session held by a live second process must make the same job contended
/// here, and must be acquirable again once that process exits. This is the
/// core B3 claim: one winner for concurrent same-job claims, proven across
/// processes rather than within one.
#[test]
fn live_session_in_another_process_makes_the_same_job_contended() {
    let lock_dir = lock_directory("cross-job");
    let destination = "/archive/final/job-1";
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let (holder, described) = spawn_session_probe(&lock_dir, "hold", "job-1", destination, 4);
    assert_eq!(
        described, "HELD session:job=job-1:attempt=4",
        "the probe must hold the session the test asked for"
    );

    assert_eq!(
        claim_session(&coordinator, "job-1", destination, 1).err(),
        Some(SessionError::Contended),
        "a live peer session must be reported as busy, never as a failure"
    );

    release_probe(holder);
    claim_session(&coordinator, "job-1", destination, 1)
        .expect("job scope released with the peer process");
}

/// A peer holding job-A must not block this process from running job-B, so
/// distinct jobs really do proceed in parallel.
#[test]
fn a_peer_session_for_one_job_does_not_block_a_different_job() {
    let lock_dir = lock_directory("cross-parallel");
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let (holder, _) = spawn_session_probe(&lock_dir, "hold", "job-a", "/archive/final/job-a", 1);
    claim_session(&coordinator, "job-b", "/archive/final/job-b", 1)
        .expect("a different job must not be blocked by a peer session");

    release_probe(holder);
}

/// Two different jobs resolving to the same final directory must serialise:
/// the destination scope is what prevents two commits into one path, and a
/// different job id must not bypass it.
#[test]
fn two_different_jobs_sharing_a_destination_serialise() {
    let lock_dir = lock_directory("cross-destination");
    let destination = "/archive/final/shared";
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let (holder, _) = spawn_session_probe(&lock_dir, "hold", "job-a", destination, 1);
    assert_eq!(
        claim_session(&coordinator, "job-b", destination, 1).err(),
        Some(SessionError::Contended),
        "a different job must not bypass a held destination scope"
    );

    release_probe(holder);
    claim_session(&coordinator, "job-b", destination, 1)
        .expect("destination scope released with the peer process");
}

/// A peer that dies holding a session must not keep a successor out: the
/// kernel drops the lock records with the process, which is what makes a
/// crashed owner safe to supersede without any lease timeout.
///
/// The `HELD` line is the evidence that the session really was established
/// before the death, so this is not a probe that failed to acquire and then
/// exited. Live-owner contention is covered separately by `hold`; here the
/// process is confirmed dead before the successor tries, so nothing can
/// attribute the success to the owner still being alive.
#[test]
fn a_crashed_peer_session_does_not_block_a_successor() {
    let lock_dir = lock_directory("cross-crash");
    let destination = "/archive/final/job-crash";
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let (mut crasher, described) =
        spawn_session_probe(&lock_dir, "crash", "job-crash", destination, 2);
    assert_eq!(
        described, "HELD session:job=job-crash:attempt=2",
        "the crasher must have established the session before dying"
    );

    drop(crasher.stdin.take());
    let status = crasher.wait().expect("probe exit");
    assert_eq!(status.code(), Some(7), "probe exits via the crash path");
    claim_session(&coordinator, "job-crash", destination, 3)
        .expect("successor acquires after the crashed owner's process is gone");
}

/// A stale writer must be refused by the live session even though the job
/// scope is free, because fencing is about attempt identity rather than about
/// who happens to hold the lock.
#[test]
fn a_live_session_refuses_a_stale_attempt_identity() {
    let lock_dir = lock_directory("cross-stale");
    let destination = "/archive/final/job-stale";
    let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");

    let session = claim_session(&coordinator, "job-stale", destination, 5).expect("session");
    assert!(session.authorize(&attempt_of("job-stale", 5)).is_ok());
    assert_eq!(
        session.authorize(&attempt_of("job-stale", 4)),
        Err(SessionError::StaleAttempt),
        "an older attempt must never be authorized to publish or finalize"
    );
}
