//! Explicit writer lifetime contract. Drop is not evidence of termination.
//! Adapters must confirm all descendants have exited, not just the parent.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::AttemptIdentity;

/// Opaque proof that the platform adapter drained every production writer
/// role for the session-bound recovery operation.
///
/// This shared wrapper deliberately hides platform-specific evidence types;
/// only the trusted adapter module can mint it.
pub struct VerifiedRecoveryDrain {
    #[cfg(target_os = "linux")]
    evidence: crate::linux_containment::VerifiedAttemptDrain,
}

#[cfg(target_os = "linux")]
impl VerifiedRecoveryDrain {
    pub(crate) fn from_linux(evidence: crate::linux_containment::VerifiedAttemptDrain) -> Self {
        Self { evidence }
    }

    fn matches_binding(&self, binding: &crate::linux_containment::RecoveryDrainBinding) -> bool {
        self.evidence.matches_binding(binding)
    }

    pub(crate) fn identity(&self) -> &AttemptIdentity {
        self.evidence.identity()
    }

    pub fn workspace(&self) -> &Path {
        self.evidence.workspace()
    }
}

/// Validate and consume adapter evidence that retires the exact live session.
/// This authority is deliberately distinct from publication permits.
#[cfg(target_os = "linux")]
pub fn consume_predecessor_retirement<C: crate::coordination::Coordinator>(
    session: &crate::ExecutionSession<'_, C>,
    evidence: VerifiedRecoveryDrain,
) -> Result<(), PublicationError> {
    let binding = session.recovery_drain_binding(evidence.workspace());
    if !evidence.matches_binding(&binding) {
        return Err(PublicationError::IncompleteDrain);
    }
    if evidence.identity() != session.attempt() || session.attempt().job_id() != session.job_id() {
        return Err(PublicationError::AttemptMismatch);
    }
    drop(evidence);
    Ok(())
}

/// Error when minting or validating a publication permit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationError {
    /// The drained writers' attempt does not match the session's attempt.
    AttemptMismatch,
    /// The permit's destination does not match the publication directory.
    DestinationMismatch,
    IncompleteDrain,
    SessionMismatch,
    AlreadyPublished,
    JournalMismatch,
}

/// One-shot authority to replay a validated InternalV1 PREPARED journal.
/// Construction is restricted to Workflow after matching the adapter proof
/// against the live session and journal identity.
pub struct PreparedRecoveryPermit<'a, 'coordinator, C: crate::coordination::Coordinator> {
    attempt: AttemptIdentity,
    destination: std::path::PathBuf,
    journal_directory: std::path::PathBuf,
    _session: &'a crate::ExecutionSession<'coordinator, C>,
}

impl<C: crate::coordination::Coordinator> PreparedRecoveryPermit<'_, '_, C> {
    pub fn consume(
        self,
        attempt: &AttemptIdentity,
        destination: &Path,
        journal_directory: &Path,
    ) -> Result<(), PublicationError> {
        if &self.attempt != attempt {
            return Err(PublicationError::AttemptMismatch);
        }
        if self.destination != destination {
            return Err(PublicationError::DestinationMismatch);
        }
        if self.journal_directory != journal_directory {
            return Err(PublicationError::DestinationMismatch);
        }
        Ok(())
    }

    pub fn replay_identity(&self) -> (AttemptIdentity, std::path::PathBuf) {
        (self.attempt.clone(), self.journal_directory.clone())
    }

    pub fn destination(&self) -> &Path {
        &self.destination
    }
}

pub struct RecoveryJournalIdentity<'a> {
    pub attempt: &'a AttemptIdentity,
    /// Final destination identity used by the coordinator and active session.
    pub destination: &'a Path,
    /// Relative path recorded by the Storage journal (for example `Tweets/123`).
    /// Kept separate from `destination` so relative journal facts cannot be
    /// confused with the absolute destination held under lock.
    pub journal_directory: &'a Path,
    pub workspace: &'a Path,
    pub phase: RecoveryJournalPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryJournalPhase {
    Prepared,
    Committed,
}

pub fn authorize_committed_recovery<C: crate::coordination::Coordinator>(
    session: &crate::ExecutionSession<'_, C>,
    evidence: VerifiedRecoveryDrain,
    journal: RecoveryJournalIdentity<'_>,
) -> Result<(), PublicationError> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (session, evidence, journal);
        return Err(PublicationError::IncompleteDrain);
    }
    #[cfg(target_os = "linux")]
    {
        let binding = session.recovery_drain_binding(journal.workspace);
        if !evidence.matches_binding(&binding) {
            return Err(PublicationError::IncompleteDrain);
        }
        if session.attempt() != journal.attempt || evidence.identity() != journal.attempt {
            return Err(PublicationError::AttemptMismatch);
        }
        if session.destination() != journal.destination || evidence.workspace() != journal.workspace
        {
            return Err(PublicationError::DestinationMismatch);
        }
        if journal.journal_directory.as_os_str().is_empty()
            || journal.journal_directory.is_absolute()
            || journal
                .journal_directory
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(PublicationError::DestinationMismatch);
        }
        if journal.phase != RecoveryJournalPhase::Committed {
            return Err(PublicationError::JournalMismatch);
        }
        // The adapter workspace is the writer/staging domain, not necessarily the
        // archive root. Do not infer a filesystem relationship to the destination.
        drop(evidence);
        Ok(())
    }
}

pub fn authorize_prepared_recovery<'session, 'coordinator, C: crate::coordination::Coordinator>(
    session: &'session crate::ExecutionSession<'coordinator, C>,
    evidence: VerifiedRecoveryDrain,
    journal: RecoveryJournalIdentity<'_>,
) -> Result<PreparedRecoveryPermit<'session, 'coordinator, C>, PublicationError> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (session, evidence, journal);
        return Err(PublicationError::IncompleteDrain);
    }
    #[cfg(target_os = "linux")]
    {
        let binding = session.recovery_drain_binding(journal.workspace);
        if !evidence.matches_binding(&binding) {
            return Err(PublicationError::IncompleteDrain);
        }
        if session.attempt() != journal.attempt || evidence.identity() != journal.attempt {
            return Err(PublicationError::AttemptMismatch);
        }
        if session.destination() != journal.destination || evidence.workspace() != journal.workspace
        {
            return Err(PublicationError::DestinationMismatch);
        }
        if journal.journal_directory.as_os_str().is_empty()
            || journal.journal_directory.is_absolute()
            || journal
                .journal_directory
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(PublicationError::DestinationMismatch);
        }
        if session.attempt().job_id() != session.job_id() {
            return Err(PublicationError::AttemptMismatch);
        }
        if journal.phase != RecoveryJournalPhase::Prepared {
            return Err(PublicationError::JournalMismatch);
        }
        // The workspace is the writer/staging domain, not necessarily the archive
        // root. Do not infer a filesystem relationship between it and the locked
        // archive destination; the storage journal independently checks its path.
        let permit = PreparedRecoveryPermit {
            attempt: session.attempt().clone(),
            destination: session.destination().to_path_buf(),
            journal_directory: journal.journal_directory.to_path_buf(),
            _session: session,
        };
        drop(evidence);
        Ok(permit)
    }
}

impl RecoveryJournalIdentity<'_> {
    pub fn new<'a>(
        attempt: &'a AttemptIdentity,
        destination: &'a Path,
        journal_directory: &'a Path,
        workspace: &'a Path,
        phase: RecoveryJournalPhase,
    ) -> RecoveryJournalIdentity<'a> {
        RecoveryJournalIdentity {
            attempt,
            destination,
            journal_directory,
            workspace,
            phase,
        }
    }

    pub fn matches_session_destination<C: crate::coordination::Coordinator>(
        &self,
        session: &crate::ExecutionSession<'_, C>,
    ) -> bool {
        session.destination() == self.destination
    }
}

/// Platform adapter owning an exclusively assigned writer domain.
pub trait WriterContainment {
    type Error;

    /// Start the target only after the containment boundary is established.
    fn start(&mut self) -> Result<(), Self::Error>;

    /// Block new starts, stop writers, and confirm the entire domain is empty.
    /// Must also work after a partially failed start.
    fn stop_and_drain(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterState {
    Ready,
    Starting,
    Running,
    Stopping,
    Drained,
}

#[derive(Debug)]
pub enum WriterLifecycleError<E> {
    InvalidState,
    Adapter(E),
}

/// Non-cloneable evidence tied to the owner borrow, not a publication permit.
pub struct DrainedWriters<'a> {
    identity: &'a AttemptIdentity,
}

impl DrainedWriters<'_> {
    pub fn identity(&self) -> &AttemptIdentity {
        self.identity
    }
}

/// Move-only publication authority borrowing both the live session and its
/// exact writer-set owner. Only `PublicationOwner::authorize` can construct it.
#[derive(Debug)]
pub struct PublicationPermit<'a> {
    attempt: &'a AttemptIdentity,
    destination: &'a Path,
    _owner: &'a PublicationOwner,
}

impl PublicationPermit<'_> {
    pub fn attempt(&self) -> &AttemptIdentity {
        self.attempt
    }

    pub fn destination(&self) -> &Path {
        self.destination
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProductionWriters {
    pub(crate) closed: bool,
    published: bool,
    pub(crate) roles: std::collections::BTreeMap<String, bool>,
}

impl ProductionWriters {
    pub(crate) fn mark_drained(&mut self, role: &str) -> Result<(), PublicationError> {
        let drained = self
            .roles
            .get_mut(role)
            .ok_or(PublicationError::IncompleteDrain)?;
        *drained = true;
        Ok(())
    }
}

/// Fixed production writer-set owner, created only from a live session before
/// factory startup. Registration is Linux-adapter-specific, not a user trait.
#[derive(Debug)]
pub struct PublicationOwner {
    source: Arc<()>,
    attempt: AttemptIdentity,
    state: Arc<Mutex<ProductionWriters>>,
}

/// Factory capability: registers concrete kernel-contained writers but cannot
/// mint publication authorization. Clones share permanently closed admission.
#[derive(Clone)]
pub struct WriterRegistration {
    source: Arc<()>,
    attempt: AttemptIdentity,
    state: Arc<Mutex<ProductionWriters>>,
}

impl PublicationOwner {
    pub fn for_session<C: crate::coordination::Coordinator>(
        session: &crate::ExecutionSession<'_, C>,
    ) -> Self {
        Self {
            source: session.publication_source.clone(),
            attempt: session.attempt().clone(),
            state: Arc::new(Mutex::new(ProductionWriters::default())),
        }
    }
    pub fn registration(&self) -> WriterRegistration {
        WriterRegistration {
            source: self.source.clone(),
            attempt: self.attempt.clone(),
            state: self.state.clone(),
        }
    }
    pub fn close_admission(&self) -> Result<(), PublicationError> {
        self.state
            .lock()
            .map_err(|_| PublicationError::IncompleteDrain)?
            .closed = true;
        Ok(())
    }
    pub fn authorize<'a, C: crate::coordination::Coordinator>(
        &'a self,
        session: &'a crate::ExecutionSession<'_, C>,
    ) -> Result<PublicationPermit<'a>, PublicationError> {
        if !Arc::ptr_eq(&self.source, &session.publication_source) {
            return Err(PublicationError::SessionMismatch);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| PublicationError::IncompleteDrain)?;
        if state.published {
            return Err(PublicationError::AlreadyPublished);
        }
        if !state.closed
            || state.roles.get("sidecar") != Some(&true)
            || state.roles.values().any(|drained| !drained)
        {
            return Err(PublicationError::IncompleteDrain);
        }
        state.published = true;
        Ok(PublicationPermit {
            attempt: session.attempt(),
            destination: session.destination(),
            _owner: self,
        })
    }
}

impl WriterRegistration {
    pub fn same_owner(&self, owner: &PublicationOwner) -> bool {
        Arc::ptr_eq(&self.source, &owner.source) && Arc::ptr_eq(&self.state, &owner.state)
    }

    #[cfg(target_os = "linux")]
    pub fn register_linux(
        &self,
        containment: &mut crate::linux_containment::LinuxContainment,
        identity: &crate::linux_containment::AttemptIdentity,
    ) -> std::io::Result<()> {
        if identity.job_id() != self.attempt.job_id()
            || identity.attempt() != u64::from(self.attempt.attempt())
        {
            return Err(std::io::Error::other(
                "writer registration attempt mismatch",
            ));
        }
        let role = identity.writer();
        if !matches!(role, "sidecar" | "aria2-0" | "aria2-1") {
            return Err(std::io::Error::other("unknown production writer role"));
        }
        if !containment.matches_recorded(identity) {
            return Err(std::io::Error::other(
                "writer registration requires an unstarted recorded owner",
            ));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| std::io::Error::other("writer registry poisoned"))?;
        if state.closed || state.roles.contains_key(role) {
            return Err(std::io::Error::other(
                "writer admission closed or duplicate role",
            ));
        }
        state.roles.insert(role.to_owned(), false);
        containment.register_production(self.state.clone(), role.to_owned());
        Ok(())
    }
}

/// Single-owner, single-start lifecycle. No adapter or handle escapes.
pub struct WriterLifecycle<C> {
    identity: AttemptIdentity,
    containment: C,
    state: WriterState,
}

impl<C: WriterContainment> WriterLifecycle<C> {
    pub fn new(identity: AttemptIdentity, containment: C) -> Self {
        Self {
            identity,
            containment,
            state: WriterState::Ready,
        }
    }

    pub fn state(&self) -> WriterState {
        self.state
    }

    pub fn start(&mut self) -> Result<(), WriterLifecycleError<C::Error>> {
        if self.state != WriterState::Ready {
            return Err(WriterLifecycleError::InvalidState);
        }
        self.state = WriterState::Starting;
        self.containment
            .start()
            .map_err(WriterLifecycleError::Adapter)?;
        self.state = WriterState::Running;
        Ok(())
    }

    /// Retry failed cleanup, but never reopen admission after cleanup begins.
    pub fn stop_and_drain(&mut self) -> Result<DrainedWriters<'_>, WriterLifecycleError<C::Error>> {
        if self.state != WriterState::Drained {
            self.state = WriterState::Stopping;
            self.containment
                .stop_and_drain()
                .map_err(WriterLifecycleError::Adapter)?;
            self.state = WriterState::Drained;
        }
        Ok(DrainedWriters {
            identity: &self.identity,
        })
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

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
}
#[cfg(all(test, target_os = "linux"))]
mod tests {
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
        let wrong_attempt_session = crate::ExecutionReservation::acquire(
            &wrong_attempt_coordinator,
            "other-job",
            &destination,
        )
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

        let committed =
            |evidence, journal| authorize_committed_recovery(&session, evidence, journal);
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
        let foreign_owner_registration =
            PublicationOwner::for_session(&foreign_session).registration();
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
}
