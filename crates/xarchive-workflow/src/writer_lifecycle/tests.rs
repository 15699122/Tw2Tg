use super::*;
use crate::coordination::{CoordinationGuard, Coordinator, LockError, LockScope};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DRAIN_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn unique_record_root() -> std::path::PathBuf {
    let sequence = NEXT_DRAIN_FIXTURE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "xarchive-drain-proof-{}-{sequence}",
        std::process::id()
    ))
}

struct TestCoordinator;
struct TestGuard;

impl CoordinationGuard for TestGuard {
    fn release(self) -> Result<(), LockError> {
        Ok(())
    }
}

impl Coordinator for TestCoordinator {
    type Guard = TestGuard;

    fn acquire(&self, _scope: &LockScope) -> Result<Self::Guard, LockError> {
        Ok(TestGuard)
    }

    fn acquire_all(&self, _scopes: &[LockScope]) -> Result<Self::Guard, LockError> {
        Ok(TestGuard)
    }
}

fn recovery_session<'a>(
    coordinator: &'a TestCoordinator,
) -> crate::ExecutionSession<'a, TestCoordinator> {
    crate::ExecutionReservation::acquire(coordinator, "job", Path::new("/archive/Tweets/123"))
        .expect("reservation")
        .bind_existing_attempt(crate::AttemptIdentity::new("job", 3).unwrap())
        .expect("session")
}

fn drain_evidence(
    session: &crate::ExecutionSession<'_, TestCoordinator>,
    workspace: &Path,
) -> VerifiedRecoveryDrain {
    let root = unique_record_root();
    let evidence = crate::linux_containment::LinuxContainment::recover_production_attempt(
        &root,
        session.recovery_drain_binding(workspace),
        std::time::Duration::from_millis(10),
    )
    .expect("empty fixed writer-role set is recovered");
    evidence
}

fn journal<'a>(
    attempt: &'a AttemptIdentity,
    destination: &'a Path,
    workspace: &'a Path,
    phase: RecoveryJournalPhase,
) -> RecoveryJournalIdentity<'a> {
    RecoveryJournalIdentity {
        attempt,
        destination,
        journal_directory: Path::new("Tweets/123"),
        workspace,
        phase,
    }
}

#[cfg(target_os = "linux")]
#[test]
fn recovery_authorization_rejects_mismatched_and_invalid_evidence() {
    let coordinator = TestCoordinator;
    let session = recovery_session(&coordinator);
    let workspace = Path::new("/staging");
    let destination = session.destination().to_path_buf();

    let wrong_attempt_coordinator = TestCoordinator;
    let wrong_attempt_session =
        crate::ExecutionReservation::acquire(&wrong_attempt_coordinator, "other-job", &destination)
            .expect("wrong-attempt reservation")
            .bind_existing_attempt(AttemptIdentity::new("other-job", 3).unwrap())
            .expect("wrong-attempt session");
    let wrong_attempt = drain_evidence(&wrong_attempt_session, workspace);
    assert_eq!(
        authorize_prepared_recovery(
            &session,
            wrong_attempt,
            journal(
                session.attempt(),
                &destination,
                workspace,
                RecoveryJournalPhase::Prepared
            )
        )
        .err(),
        Some(PublicationError::IncompleteDrain)
    );

    let wrong_workspace = drain_evidence(&session, Path::new("/other-staging"));
    assert_eq!(
        authorize_prepared_recovery(
            &session,
            wrong_workspace,
            journal(
                session.attempt(),
                &destination,
                workspace,
                RecoveryJournalPhase::Prepared
            )
        )
        .err(),
        Some(PublicationError::IncompleteDrain)
    );

    let wrong_operation_coordinator = TestCoordinator;
    let wrong_operation_session = recovery_session(&wrong_operation_coordinator);
    let wrong_operation = drain_evidence(&wrong_operation_session, workspace);
    assert_eq!(
        authorize_prepared_recovery(
            &session,
            wrong_operation,
            journal(
                session.attempt(),
                &destination,
                workspace,
                RecoveryJournalPhase::Prepared
            )
        )
        .err(),
        Some(PublicationError::IncompleteDrain)
    );

    let incomplete_identity = RecoveryJournalIdentity {
        attempt: session.attempt(),
        destination: &destination,
        journal_directory: Path::new("../escape"),
        workspace,
        phase: RecoveryJournalPhase::Prepared,
    };
    assert_eq!(
        authorize_prepared_recovery(
            &session,
            drain_evidence(&session, workspace),
            incomplete_identity
        )
        .err(),
        Some(PublicationError::DestinationMismatch)
    );

    assert_eq!(
        authorize_committed_recovery(
            &session,
            drain_evidence(&session, workspace),
            journal(
                session.attempt(),
                &destination,
                workspace,
                RecoveryJournalPhase::Prepared
            )
        ),
        Err(PublicationError::JournalMismatch)
    );
    assert_eq!(
        authorize_prepared_recovery(
            &session,
            drain_evidence(&session, workspace),
            journal(
                session.attempt(),
                &destination,
                workspace,
                RecoveryJournalPhase::Committed
            )
        )
        .err(),
        Some(PublicationError::JournalMismatch)
    );

    let committed = |evidence, journal| authorize_committed_recovery(&session, evidence, journal);
    assert_eq!(
        committed(
            drain_evidence(&wrong_attempt_session, workspace),
            journal(
                session.attempt(),
                &destination,
                workspace,
                RecoveryJournalPhase::Committed
            ),
        ),
        Err(PublicationError::IncompleteDrain)
    );
    assert_eq!(
        committed(
            drain_evidence(&session, Path::new("/other-staging")),
            journal(
                session.attempt(),
                &destination,
                workspace,
                RecoveryJournalPhase::Committed
            ),
        ),
        Err(PublicationError::IncompleteDrain)
    );
    assert_eq!(
        committed(
            drain_evidence(&session, workspace),
            RecoveryJournalIdentity {
                attempt: session.attempt(),
                destination: &destination,
                journal_directory: Path::new("../escape"),
                workspace,
                phase: RecoveryJournalPhase::Committed,
            },
        ),
        Err(PublicationError::DestinationMismatch)
    );
}

struct Fake {
    fail_start: bool,
    fail_stop: bool,
}
impl WriterContainment for Fake {
    type Error = ();
    fn start(&mut self) -> Result<(), ()> {
        if self.fail_start { Err(()) } else { Ok(()) }
    }
    fn stop_and_drain(&mut self) -> Result<(), ()> {
        if std::mem::take(&mut self.fail_stop) {
            Err(())
        } else {
            Ok(())
        }
    }
}

#[test]
fn failed_start_requires_cleanup_and_cannot_restart() {
    let identity = AttemptIdentity::new("job", 1).unwrap();
    let mut owner = WriterLifecycle::new(
        identity.clone(),
        Fake {
            fail_start: true,
            fail_stop: false,
        },
    );
    assert!(owner.start().is_err());
    assert_eq!(owner.state(), WriterState::Starting);
    assert!(matches!(
        owner.start(),
        Err(WriterLifecycleError::InvalidState)
    ));
    assert_eq!(owner.stop_and_drain().unwrap().identity(), &identity);
    assert_eq!(owner.state(), WriterState::Drained);
}

#[test]
fn failed_drain_never_proves_exit_and_retry_keeps_admission_closed() {
    let mut owner = WriterLifecycle::new(
        AttemptIdentity::new("job", 2).unwrap(),
        Fake {
            fail_start: false,
            fail_stop: true,
        },
    );
    owner.start().unwrap();
    assert!(owner.start().is_err());
    assert!(owner.stop_and_drain().is_err());
    assert_eq!(owner.state(), WriterState::Stopping);
    assert!(owner.start().is_err());
    owner.stop_and_drain().unwrap();
    owner.stop_and_drain().unwrap();
    assert_eq!(owner.state(), WriterState::Drained);
    assert!(owner.start().is_err());
}

#[test]
fn production_drain_update_requires_a_registered_role() {
    let mut writers = ProductionWriters::default();
    writers.roles.insert("sidecar".to_owned(), false);
    writers.mark_drained("sidecar").unwrap();
    assert_eq!(writers.roles.get("sidecar"), Some(&true));
    assert_eq!(
        writers.mark_drained("unknown"),
        Err(PublicationError::IncompleteDrain)
    );
}

#[test]
fn admission_close_does_not_erase_a_later_verified_drain() {
    let mut writers = ProductionWriters::default();
    writers.roles.insert("sidecar".to_owned(), false);
    writers.closed = true;
    writers.mark_drained("sidecar").unwrap();
    assert!(writers.closed);
    assert_eq!(writers.roles.get("sidecar"), Some(&true));
}

#[test]
fn lifecycle_owner_is_retained_until_adapter_drain_completes() {
    use crate::coordination::unix::LinuxCoordinator;
    use crate::execution::{ClaimOutcome, ExecutionReservation};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn lock_directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "xarchive-writer-lifecycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    let lock_directory = lock_directory();
    let coordinator = LinuxCoordinator::new(&lock_directory).unwrap();
    let session = ExecutionReservation::acquire(
        &coordinator,
        "job-active-writer",
        Path::new("/archive/job-active-writer"),
    )
    .unwrap()
    .claim(|| ClaimOutcome::Claimed(AttemptIdentity::new("job-active-writer", 7).unwrap()))
    .unwrap();
    let publication_owner = PublicationOwner::for_session(&session);
    let registration = publication_owner.registration();
    let late_registration = registration.clone();
    let (writer_started_tx, writer_started_rx) = mpsc::channel();
    let (writer_drained_tx, writer_drained_rx) = mpsc::channel();
    let foreign_job = "job-active-writer-foreign";
    let foreign_session = ExecutionReservation::acquire(
        &coordinator,
        foreign_job,
        Path::new("/archive/job-active-writer-foreign"),
    )
    .unwrap()
    .claim(|| ClaimOutcome::Claimed(AttemptIdentity::new(foreign_job, 8).unwrap()))
    .unwrap();
    let foreign_owner_registration = PublicationOwner::for_session(&foreign_session).registration();
    struct PausedWriter {
        release: mpsc::Receiver<()>,
        started: mpsc::Sender<()>,
        identity: AttemptIdentity,
        drained: mpsc::Sender<AttemptIdentity>,
    }

    impl WriterContainment for PausedWriter {
        type Error = ();

        fn start(&mut self) -> Result<(), Self::Error> {
            self.started.send(()).unwrap();
            Ok(())
        }

        fn stop_and_drain(&mut self) -> Result<(), Self::Error> {
            self.started.send(()).unwrap();
            self.release.recv().unwrap();
            self.drained.send(self.identity.clone()).unwrap();
            Ok(())
        }
    }

    let identity = AttemptIdentity::new("job-active-writer", 7).unwrap();
    let (release_drain_tx, release_drain_rx) = mpsc::channel();
    let writer_identity = identity.clone();
    assert!(registration.same_owner(&publication_owner));
    assert_eq!(registration.attempt, session.attempt().clone());
    assert!(!foreign_owner_registration.same_owner(&publication_owner));
    assert_ne!(
        foreign_owner_registration.attempt,
        session.attempt().clone()
    );
    let mut lifecycle = WriterLifecycle::new(
        identity.clone(),
        PausedWriter {
            release: release_drain_rx,
            started: writer_started_tx,
            identity: identity.clone(),
            drained: writer_drained_tx,
        },
    );
    lifecycle.start().unwrap();
    assert_eq!(lifecycle.state(), WriterState::Running);
    writer_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("writer lifecycle adapter is active");
    let started_lifecycle = lifecycle;
    let (owner_tx, owner_rx) = mpsc::channel();
    let (finish_tx, finish_rx) = mpsc::channel();
    let drain = thread::spawn(move || {
        let mut lifecycle = started_lifecycle;
        let drained = lifecycle.stop_and_drain().unwrap();
        assert_eq!(drained.identity(), &identity);
        assert_eq!(lifecycle.state(), WriterState::Drained);
        owner_tx.send(lifecycle).unwrap();
        finish_rx.recv().unwrap();
    });

    writer_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("drain reaches the paused lifecycle adapter");
    assert!(writer_drained_rx.try_recv().is_err());

    // Admission closes before this independent lifecycle adapter drains.
    // Its notification is not production registry evidence. The real
    // Linux registration path is exercised only with delegated cgroups.
    publication_owner.close_admission().unwrap();
    assert!(matches!(
        publication_owner.authorize(&session),
        Err(PublicationError::IncompleteDrain)
    ));
    assert!(registration.state.lock().unwrap().roles.is_empty());
    assert!(late_registration.same_owner(&publication_owner));
    assert_eq!(late_registration.attempt, session.attempt().clone());
    assert!(late_registration.state.lock().unwrap().closed);

    release_drain_tx.send(()).unwrap();
    let drained = writer_drained_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("writer confirms drain before owner finalization");
    assert_eq!(
        drained,
        AttemptIdentity::new("job-active-writer", 7).unwrap()
    );
    assert!(registration.state.lock().unwrap().roles.is_empty());
    assert!(matches!(
        publication_owner.authorize(&session),
        Err(PublicationError::IncompleteDrain)
    ));
    let mut retained_owner = owner_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("same owner is returned only after drain completes");
    assert_eq!(retained_owner.state(), WriterState::Drained);
    assert_eq!(writer_identity.job_id(), session.attempt().job_id());
    assert_eq!(writer_identity.attempt(), session.attempt().attempt());
    assert_eq!(
        retained_owner.stop_and_drain().unwrap().identity(),
        &AttemptIdentity::new("job-active-writer", 7).unwrap(),
        "the same lifecycle owner remains available for idempotent drain"
    );
    assert!(matches!(
        publication_owner.authorize(&session),
        Err(PublicationError::IncompleteDrain)
    ));
    finish_tx.send(()).unwrap();
    drain.join().unwrap();
    drop(foreign_session);
    drop(session);
    drop(coordinator);
    std::fs::remove_dir_all(lock_directory).unwrap();
}
