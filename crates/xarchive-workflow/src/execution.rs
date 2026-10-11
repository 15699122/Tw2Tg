//! Runner-owned execution session: the authorization token for one attempt.
//!
//! A session is what makes mutating a job and its destination legal. It is
//! deliberately generic over the [`Coordinator`] port rather than over any
//! platform adapter, because the shared lifecycle has to be able to hold a
//! session on every target and the platform difference belongs entirely to
//! the adapter implementation. That is why the Unix adapter's `cfg(unix)`
//! gate must never leak into this type: a session compiled out of existence
//! on one platform would silently turn the shared path into an
//! unsynchronised one there.
//!
//! Two properties define the contract.
//!
//! First, holding the session is the sole evidence that mutation is
//! permitted. The guard is owned by the session and is never handed out in a
//! releasable form, so no caller can drop exclusivity while the session is
//! live, and no caller can rebuild a guard from a cached value to fake
//! ownership it never acquired.
//!
//! Second, contention is not failure. A session that cannot be established
//! because another live owner holds the scope reports
//! [`SessionError::Contended`], which the caller must treat as a
//! reschedulable outcome. Converting a busy peer into a terminal `FAILED`
//! job is the defect this contract exists to prevent.
//!
//! # Why establishment is two-phase
//!
//! The durable attempt number is produced by the claim transaction, so it
//! cannot be known before the claim runs. But the claim must not run before
//! the scope is held: a claim made outside the lock mutates the job and
//! increments its attempt, and only then discovers that a peer already owns
//! the job, leaving a Job whose attempt advanced for nothing while its
//! writers were never serialized. Claiming inside the lock would mean moving
//! SQLite into this crate, so the ordering is expressed as a callback instead:
//!
//! 1. [`ExecutionReservation::acquire`] takes the Job and destination scopes
//!    with no attempt yet known.
//! 2. [`ExecutionReservation::claim`] runs the caller's durable claim
//!    transaction *while that guard is already held*, then binds the session
//!    to the attempt the claim actually returned.
//!
//! The callback makes the ordering impossible to invert by construction. An
//! earlier single-step `begin` that accepted a pre-built
//! [`AttemptIdentity`] was removed for exactly that reason: it permitted the
//! claim-then-lock sequence that this module exists to forbid.

use std::path::{Path, PathBuf};

use crate::AttemptIdentity;
use crate::coordination::{Coordinator, LockError, LockScope};

/// What the caller's durable claim transaction decided.
///
/// The transaction is the authority on whether this process owns the next
/// attempt, because only it can compare expected state and attempt atomically.
/// The coordination layer cannot make that decision and must not invent one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// This process won; the attempt identity it now owns.
    Claimed(AttemptIdentity),
    /// Another writer already advanced the job. No attempt may be started and
    /// no writer may run, but the Job is healthy and must stay reschedulable.
    Lost,
    /// The durable claim itself failed. This is not contention and must not be
    /// reported as a busy peer.
    Failed,
}

/// Why a session could not be established, or could not authorize a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// Another live owner holds the Job or destination scope. The Job must be
    /// left queued and reschedulable; this is never a download failure.
    Contended,
    /// The durable claim was won by another writer. Same reschedulable
    /// treatment as [`Self::Contended`], and likewise never a download failure.
    ClaimLost,
    /// The job id or destination cannot serve as a lock identity.
    InvalidScope,
    /// The lock could not be opened for a reason other than contention.
    Io,
    /// The writer's attempt is older than the session's attempt, so it has
    /// lost the arbitration and must stop writing states, files and events.
    StaleAttempt,
    /// The durable claim reported an identity for a different job, or failed
    /// without reporting contention. The scope is released and no writer runs.
    ClaimRejected,
}

impl SessionError {
    /// Whether the Job may be left queued and retried rather than failed.
    ///
    /// Only genuine contention and a lost claim are reschedulable. Everything
    /// else describes a Job that cannot proceed, so a caller must not turn an
    /// I/O or claim error into a silent retry loop.
    pub const fn is_reschedulable(self) -> bool {
        matches!(self, Self::Contended | Self::ClaimLost)
    }
}

impl From<LockError> for SessionError {
    fn from(error: LockError) -> Self {
        match error {
            LockError::Contended => Self::Contended,
            LockError::InvalidName => Self::InvalidScope,
            LockError::Io => Self::Io,
        }
    }
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contended => formatter.write_str(
                "another live owner holds this job or destination; the job stays reschedulable",
            ),
            Self::ClaimLost => formatter
                .write_str("another writer already claimed this job; the job stays reschedulable"),
            Self::InvalidScope => {
                formatter.write_str("job or destination is not a usable coordination scope")
            }
            Self::Io => formatter.write_str("coordination lock could not be opened"),
            Self::StaleAttempt => {
                formatter.write_str("this attempt lost the arbitration and may not mutate")
            }
            Self::ClaimRejected => formatter
                .write_str("the durable claim did not return an identity for this job's attempt"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Held Job and destination scopes for an attempt whose number is not yet known.
///
/// This is the phase that makes the durable claim safe to run. While a
/// reservation is alive no other process can own either scope, so the caller's
/// claim transaction observes a job that no peer is concurrently mutating.
///
/// Dropping a reservation releases both scopes. That is the failure path for a
/// claim that does not win: an attempt that never starts must not leave the
/// job locked, or the Job would be unclaimable until the process exits.
pub struct ExecutionReservation<'coordinator, C: Coordinator> {
    job_id: String,
    destination: PathBuf,
    coordinator: &'coordinator C,
    guard: C::Guard,
}

impl<C: Coordinator> std::fmt::Debug for ExecutionReservation<'_, C> {
    /// Reports the scopes held, never the guard, which an adapter might render
    /// as a copyable lock file path.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExecutionReservation")
            .field("job_id", &self.job_id)
            .field("destination", &self.destination)
            .finish_non_exhaustive()
    }
}

impl<'coordinator, C: Coordinator> ExecutionReservation<'coordinator, C> {
    /// Take the Job and destination scopes for one job, before any claim.
    ///
    /// Both scopes are requested together so the adapter applies one canonical
    /// order for every caller: two processes needing the same pair cannot
    /// deadlock by asking in opposite orders. The request order is
    /// job-then-destination, which keeps a job's own execution scope ahead of
    /// the shared destination scope whenever the adapter preserves request
    /// order; either way the single global order is what removes deadlock.
    ///
    /// `destination` is the final archive directory this attempt intends to
    /// commit. It must be non-empty, because an empty destination would
    /// collapse every destination onto one lock identity.
    pub fn acquire(
        coordinator: &'coordinator C,
        job_id: impl Into<String>,
        destination: &Path,
    ) -> Result<Self, SessionError> {
        let job_id = job_id.into();
        let guard = coordinator.acquire_all(&[
            LockScope::Job(job_id.clone()),
            LockScope::Destination(destination.to_path_buf()),
        ])?;
        Ok(Self {
            job_id,
            destination: destination.to_path_buf(),
            coordinator,
            guard,
        })
    }

    /// Run the durable claim while both scopes are held, then bind a session.
    ///
    /// The callback runs exactly once, with the guard already held, and it is
    /// the only place a session's attempt number can come from. It must
    /// perform the atomic state-plus-attempt claim; it must not be used to run
    /// unrelated work, because every instruction between the lock and the
    /// session is a window in which the job is locked but unowned.
    ///
    /// A claim that does not win drops this reservation, so both scopes are
    /// released and the Job stays claimable.
    pub fn claim<F>(self, claim: F) -> Result<ExecutionSession<'coordinator, C>, SessionError>
    where
        F: FnOnce() -> ClaimOutcome,
    {
        let Self {
            job_id,
            destination,
            coordinator,
            guard,
        } = self;
        let attempt = match claim() {
            ClaimOutcome::Claimed(attempt) => attempt,
            ClaimOutcome::Lost => {
                // Dropping the guard here is the point: a lost claim must not
                // leave the job locked, but it also must not persist a failure.
                drop(guard);
                return Err(SessionError::ClaimLost);
            }
            ClaimOutcome::Failed => {
                drop(guard);
                return Err(SessionError::ClaimRejected);
            }
        };
        // A claim that reports another job's identity would authorize this
        // session over a job it does not own, so it is refused rather than
        // used. Both scopes are released because no attempt is legitimate.
        if attempt.job_id() != job_id {
            drop(guard);
            return Err(SessionError::ClaimRejected);
        }
        Ok(ExecutionSession {
            attempt,
            job_id,
            destination,
            coordinator,
            guard,
        })
    }

    /// The job whose execution, recovery and retry this reservation serialises.
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// The final directory a session from this reservation will be bound to.
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// The coordinator that produced this reservation.
    pub fn coordinator(&self) -> &'coordinator C {
        self.coordinator
    }

    /// The retained guard, as evidence and never as a releasable handle.
    ///
    /// `CoordinationGuard::release` consumes `self`, so a borrowed reference
    /// cannot be released through this accessor.
    pub fn guard(&self) -> &C::Guard {
        &self.guard
    }
}

/// One runner-owned, fenced authorization for a durable archive attempt.
///
/// The session borrows its coordinator for its whole life. That borrow is a
/// real invariant rather than a convenience: a session cannot outlive the
/// coordinator whose adapter owns the underlying lock records, so the
/// exclusivity it represents cannot be claimed to persist past the object
/// that would have to release it.
pub struct ExecutionSession<'coordinator, C: Coordinator> {
    attempt: AttemptIdentity,
    job_id: String,
    destination: PathBuf,
    coordinator: &'coordinator C,
    guard: C::Guard,
}

impl<C: Coordinator> std::fmt::Debug for ExecutionSession<'_, C> {
    /// Reports the identity the session authorizes, not its guard.
    ///
    /// The guard is deliberately not printed: an adapter's guard may expose
    /// the lock file path, and a session's authority is not something a log
    /// line should make look copyable. `finish_non_exhaustive` marks that the
    /// held guard is part of this value without rendering it.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExecutionSession")
            .field("job_id", &self.job_id)
            .field("attempt", &self.attempt)
            .field("destination", &self.destination)
            .finish_non_exhaustive()
    }
}

impl<'coordinator, C: Coordinator> ExecutionSession<'coordinator, C> {
    /// The immutable attempt identity this session authorizes.
    ///
    /// Built only by [`ExecutionReservation::claim`], from the attempt the
    /// durable claim returned.
    pub fn attempt(&self) -> &AttemptIdentity {
        &self.attempt
    }

    /// The job whose execution, recovery and retry this session serialises.
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// The final directory this session is bound to commit.
    ///
    /// A publication to any other directory is outside the scope this session
    /// holds, even though it would still pass [`Self::authorize`], which fences
    /// on attempt identity only. A caller must compare the destination it is
    /// about to publish against this value.
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// The coordinator that produced this session.
    ///
    /// The borrow exists so a session cannot outlive its coordinator, and so
    /// later scopes can be acquired from the same adapter rather than from a
    /// second one that knows nothing about the records already held.
    pub fn coordinator(&self) -> &'coordinator C {
        self.coordinator
    }

    /// The retained guard, as evidence and never as a releasable handle.
    ///
    /// `CoordinationGuard::release` consumes `self`, so a borrowed reference
    /// cannot be released through this accessor: a caller holding the session
    /// cannot drop exclusivity out from under it, and cannot double-release
    /// it either.
    pub fn guard(&self) -> &C::Guard {
        &self.guard
    }

    /// Decide whether `writer` may still mutate this job's state and files.
    ///
    /// This is the fence a stale worker must pass before it publishes,
    /// finalizes, cleans up or writes a terminal state. A current-attempt
    /// counter read elsewhere is not a substitute: the attempt can advance
    /// after such a read while the old writer is still alive.
    pub fn authorize(&self, writer: &AttemptIdentity) -> Result<(), SessionError> {
        if crate::attempt_may_mutate(&self.attempt, writer) {
            Ok(())
        } else {
            Err(SessionError::StaleAttempt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contention_is_distinguished_from_every_other_lock_failure() {
        assert_eq!(
            SessionError::from(LockError::Contended),
            SessionError::Contended
        );
        assert_eq!(
            SessionError::from(LockError::InvalidName),
            SessionError::InvalidScope
        );
        assert_eq!(SessionError::from(LockError::Io), SessionError::Io);
        // The distinction is the whole point: only `Contended` may be treated
        // as a reschedulable outcome by a caller, so it must never be
        // conflated with an I/O failure.
        assert_ne!(SessionError::Contended, SessionError::Io);
    }

    #[test]
    fn only_contention_and_a_lost_claim_are_reschedulable() {
        assert!(SessionError::Contended.is_reschedulable());
        assert!(SessionError::ClaimLost.is_reschedulable());
        // A busy peer and a healthy lost race must both stay out of the
        // generic failure path, which would persist a false `FAILED`.
        for error in [
            SessionError::InvalidScope,
            SessionError::Io,
            SessionError::StaleAttempt,
            SessionError::ClaimRejected,
        ] {
            assert!(
                !error.is_reschedulable(),
                "{error} must not be retried as if it were contention"
            );
        }
    }

    /// A session must be usable on a runner thread, because the runner is where
    /// the claim and the writer lifetime live. This is a compile-time claim: if
    /// the guard or the coordinator borrow stops being `Send`, the runner cannot
    /// own a session and this stops compiling rather than failing at runtime.
    #[test]
    fn a_session_is_send_across_threads() {
        fn assert_send<T: Send>() {}
        assert_send::<ExecutionReservation<'static, CoordinatorProbe>>();
        assert_send::<ExecutionSession<'static, CoordinatorProbe>>();
    }

    /// Stand-in coordinator used only for the trait-bound assertion above.
    struct CoordinatorProbe;

    struct GuardProbe;

    impl crate::coordination::CoordinationGuard for GuardProbe {
        fn release(self) -> Result<(), LockError> {
            Ok(())
        }
    }

    impl Coordinator for CoordinatorProbe {
        type Guard = GuardProbe;

        fn acquire(&self, _scope: &LockScope) -> Result<Self::Guard, LockError> {
            Ok(GuardProbe)
        }

        fn acquire_all(&self, _scopes: &[LockScope]) -> Result<Self::Guard, LockError> {
            Ok(GuardProbe)
        }
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;
    use crate::coordination::unix::LinuxCoordinator;
    use std::cell::Cell;
    use std::path::PathBuf;

    fn lock_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "xarchive-session-unit-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("lock directory");
        directory
    }

    fn attempt_of(job_id: &str, attempt: u32) -> AttemptIdentity {
        AttemptIdentity::new(job_id, attempt).expect("valid attempt")
    }

    fn claim_won(job_id: &str, attempt: u32) -> ClaimOutcome {
        ClaimOutcome::Claimed(attempt_of(job_id, attempt))
    }

    #[test]
    fn the_durable_claim_runs_while_both_scopes_are_already_held() {
        let directory = lock_directory("claim-ordering");
        let claimer = LinuxCoordinator::new(&directory).expect("claiming coordinator");
        let peer = LinuxCoordinator::new(&directory).expect("peer coordinator");
        let destination = Path::new("/archive/final/job-1");

        let reservation =
            ExecutionReservation::acquire(&claimer, "job-1", destination).expect("reservation");
        let peer_saw_contention = Cell::new(false);
        let session = reservation
            .claim(|| {
                // Inside the claim, no other process may own either scope.
                // This is the ordering the two-phase type exists to enforce.
                peer_saw_contention.set(
                    ExecutionReservation::acquire(&peer, "job-1", destination)
                        .err()
                        .is_some_and(|error| error.is_reschedulable()),
                );
                claim_won("job-1", 1)
            })
            .expect("session");

        assert!(
            peer_saw_contention.get(),
            "the job scope must already be held while the durable claim runs"
        );
        assert_eq!(session.attempt().attempt(), 1);
        assert_eq!(session.destination(), destination);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_session_binds_the_attempt_the_claim_returned() {
        let directory = lock_directory("bind-attempt");
        let coordinator = LinuxCoordinator::new(&directory).expect("coordinator");
        let destination = Path::new("/archive/final/job-1");

        let session = ExecutionReservation::acquire(&coordinator, "job-1", destination)
            .expect("reservation")
            // The claim, not the caller's expectation, is the source of the
            // attempt number, so a caller that guesses must be overruled.
            .claim(|| claim_won("job-1", 7))
            .expect("session");

        assert_eq!(session.job_id(), "job-1");
        assert_eq!(session.attempt().attempt(), 7);
        assert_eq!(session.destination(), destination);
        assert!(session.authorize(&attempt_of("job-1", 7)).is_ok());
        assert_eq!(
            session.authorize(&attempt_of("job-1", 6)),
            Err(SessionError::StaleAttempt)
        );
        // A different job must never be authorized by this session, even with
        // a numerically larger attempt number.
        assert_eq!(
            session.authorize(&attempt_of("job-2", 9)),
            Err(SessionError::StaleAttempt)
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_lost_claim_releases_the_scope_and_starts_no_writer() {
        let directory = lock_directory("claim-lost");
        let claimer = LinuxCoordinator::new(&directory).expect("claiming coordinator");
        let peer = LinuxCoordinator::new(&directory).expect("peer coordinator");
        let destination = Path::new("/archive/final/job-1");

        assert_eq!(
            ExecutionReservation::acquire(&claimer, "job-1", destination)
                .expect("reservation")
                .claim(|| ClaimOutcome::Lost)
                .err(),
            Some(SessionError::ClaimLost),
            "a lost race must be reported as a lost claim, never as contention"
        );
        // The scope must be free again, or a healthy Job would be unclaimable
        // until this process exited.
        ExecutionReservation::acquire(&peer, "job-1", destination)
            .expect("scope released by the lost claim");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_failed_claim_releases_the_scope_without_reporting_contention() {
        let directory = lock_directory("claim-failed");
        let claimer = LinuxCoordinator::new(&directory).expect("claiming coordinator");
        let peer = LinuxCoordinator::new(&directory).expect("peer coordinator");
        let destination = Path::new("/archive/final/job-1");

        let error = ExecutionReservation::acquire(&claimer, "job-1", destination)
            .expect("reservation")
            .claim(|| ClaimOutcome::Failed)
            .expect_err("a failed claim must not yield a session");
        assert_eq!(error, SessionError::ClaimRejected);
        assert!(
            !error.is_reschedulable(),
            "a failed claim is not a busy peer and must not be retried as one"
        );
        ExecutionReservation::acquire(&peer, "job-1", destination)
            .expect("scope released by the failed claim");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_claim_reporting_another_job_is_refused_and_releases_the_scope() {
        let directory = lock_directory("claim-mismatch");
        let claimer = LinuxCoordinator::new(&directory).expect("claiming coordinator");
        let peer = LinuxCoordinator::new(&directory).expect("peer coordinator");
        let destination = Path::new("/archive/final/job-1");

        assert_eq!(
            ExecutionReservation::acquire(&claimer, "job-1", destination)
                .expect("reservation")
                .claim(|| claim_won("job-2", 1))
                .err(),
            Some(SessionError::ClaimRejected),
            "an identity for another job must never authorize this session"
        );
        ExecutionReservation::acquire(&peer, "job-1", destination)
            .expect("scope released by the refused claim");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_live_session_makes_a_second_session_for_the_same_job_contended() {
        let directory = lock_directory("same-job");
        let first = LinuxCoordinator::new(&directory).expect("first coordinator");
        let second = LinuxCoordinator::new(&directory).expect("second coordinator");
        let destination = Path::new("/archive/final/job-1");

        let session = ExecutionReservation::acquire(&first, "job-1", destination)
            .expect("reservation")
            .claim(|| claim_won("job-1", 1))
            .expect("first session");
        assert_eq!(
            ExecutionReservation::acquire(&second, "job-1", destination).err(),
            Some(SessionError::Contended),
            "a second owner of the same job must be told it is busy, not failed"
        );

        drop(session);
        ExecutionReservation::acquire(&second, "job-1", destination)
            .expect("scope released with the session");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn an_empty_destination_is_refused_as_a_scope() {
        let directory = lock_directory("empty-destination");
        let coordinator = LinuxCoordinator::new(&directory).expect("coordinator");
        assert_eq!(
            ExecutionReservation::acquire(&coordinator, "job-1", Path::new("")).err(),
            Some(SessionError::InvalidScope),
            "an empty destination would collapse every destination onto one lock"
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
