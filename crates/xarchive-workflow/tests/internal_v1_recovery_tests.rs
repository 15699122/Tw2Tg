//! Integration checks for binding recovery to an already-persisted attempt.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xarchive_workflow::coordination::unix::LinuxCoordinator;
use xarchive_workflow::coordination::{Coordinator, LockError, LockScope};
use xarchive_workflow::{AttemptIdentity, ExecutionReservation, SessionError};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let base = std::env::temp_dir();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        for _ in 0..100 {
            let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!(
                "xarchive-v1-recovery-{}-{timestamp}-{id}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create isolated test directory: {error}"),
            }
        }
        panic!("could not allocate a unique test directory")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn binding_existing_attempt_does_not_claim_and_holds_both_scopes() {
    let directory = TestDirectory::new();
    let first = LinuxCoordinator::new(directory.path().join("locks")).expect("first coordinator");
    let second =
        LinuxCoordinator::new(directory.path().join("locks")).expect("independent coordinator");
    let destination = directory.path().join("archive/Tweets/123");
    let attempt = AttemptIdentity::new("job-123", 7).expect("attempt identity");

    let session = ExecutionReservation::acquire(&first, "job-123", &destination)
        .expect("reserve job and destination")
        .bind_existing_attempt_checked(attempt.clone(), |bound| bound == &attempt)
        .expect("bind the persisted attempt without claiming");

    assert_eq!(session.attempt(), &attempt);
    assert!(matches!(
        second.acquire(&LockScope::Job("job-123".into())),
        Err(LockError::Contended)
    ));
    let destination_guard = second
        .acquire(&LockScope::Destination(destination.clone()))
        .expect_err("the destination scope is retained too");
    assert!(matches!(destination_guard, LockError::Contended));
    assert!(matches!(
        ExecutionReservation::acquire(&second, "different-job", &destination),
        Err(SessionError::Contended)
    ));

    drop(session);
    let released = ExecutionReservation::acquire(&second, "job-123", &destination)
        .expect("dropping the bound session releases both scopes");
    drop(released);
}

#[test]
fn rejected_existing_attempt_validation_releases_reserved_scopes() {
    let directory = TestDirectory::new();
    let first = LinuxCoordinator::new(directory.path().join("locks")).expect("first coordinator");
    let second =
        LinuxCoordinator::new(directory.path().join("locks")).expect("independent coordinator");
    let destination = directory.path().join("archive/Tweets/456");
    let attempt = AttemptIdentity::new("job-456", 3).expect("attempt identity");

    let error = ExecutionReservation::acquire(&first, "job-456", &destination)
        .expect("reservation")
        .bind_existing_attempt_checked(attempt, |_| false)
        .expect_err("failed durable revalidation must reject binding");
    assert!(matches!(error, SessionError::ClaimRejected));

    let reacquired = ExecutionReservation::acquire(&second, "job-456", &destination)
        .expect("rejected binding must release both scopes");
    drop(reacquired);
}

#[cfg(target_os = "linux")]
#[test]
fn drained_predecessor_takeover_consumes_evidence_and_retains_both_scopes() {
    let directory = TestDirectory::new();
    let first = LinuxCoordinator::new(directory.path().join("locks")).expect("first coordinator");
    let second = LinuxCoordinator::new(directory.path().join("locks")).expect("second coordinator");
    let destination = directory.path().join("archive/Tweets/789");
    let predecessor = AttemptIdentity::new("job-789", 4).expect("predecessor");
    let successor = AttemptIdentity::new("job-789", 5).expect("successor");
    let workspace = directory.path().join("staging");
    let record_root = directory.path().join("records");

    let reservation = ExecutionReservation::acquire(&first, "job-789", &destination)
        .expect("reserve both scopes");
    let predecessor_session = reservation
        .bind_existing_attempt(predecessor.clone())
        .expect("bind predecessor");
    let evidence =
        xarchive_workflow::linux_containment::LinuxContainment::recover_production_attempt(
            &record_root,
            predecessor_session.recovery_drain_binding(&workspace),
            std::time::Duration::from_millis(10),
        )
        .expect("empty production writer domains are drained");

    let peer = LinuxCoordinator::new(directory.path().join("locks")).expect("peer coordinator");
    assert!(matches!(
        ExecutionReservation::acquire(&peer, "job-789", &destination),
        Err(SessionError::Contended)
    ));
    let successor_session = predecessor_session
        .take_over_drained_predecessor(evidence, |predecessor| {
            assert_eq!(predecessor, &AttemptIdentity::new("job-789", 4).unwrap());
            Ok(Some(successor.clone()))
        })
        .expect("consume predecessor proof and retain the same scopes");
    assert_eq!(successor_session.attempt(), &successor);
    assert!(matches!(
        ExecutionReservation::acquire(&peer, "job-789", &destination),
        Err(SessionError::Contended)
    ));
    drop(successor_session);
    let released = ExecutionReservation::acquire(&second, "job-789", &destination)
        .expect("successor session drop releases its scopes");
    drop(released);
}
