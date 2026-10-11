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

use std::path::Path;

use crate::AttemptIdentity;
use crate::coordination::{Coordinator, LockError, LockScope};

/// Why a session could not be established, or could not authorize a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// Another live owner holds the Job or destination scope. The Job must be
    /// left queued and reschedulable; this is never a download failure.
    Contended,
    /// The job id or destination cannot serve as a lock identity.
    InvalidScope,
    /// The lock could not be opened for a reason other than contention.
    Io,
    /// The writer's attempt is older than the session's attempt, so it has
    /// lost the arbitration and must stop writing states, files and events.
    StaleAttempt,
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
            Self::InvalidScope => {
                formatter.write_str("job or destination is not a usable coordination scope")
            }
            Self::Io => formatter.write_str("coordination lock could not be opened"),
            Self::StaleAttempt => {
                formatter.write_str("this attempt lost the arbitration and may not mutate")
            }
        }
    }
}

impl std::error::Error for SessionError {}

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
            .finish_non_exhaustive()
    }
}

impl<'coordinator, C: Coordinator> ExecutionSession<'coordinator, C> {
    /// Establish a session for one attempt over its job and destination.
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
    pub fn begin(
        coordinator: &'coordinator C,
        attempt: AttemptIdentity,
        destination: &Path,
    ) -> Result<Self, SessionError> {
        let job_id = attempt.job_id().to_owned();
        let guard = coordinator.acquire_all(&[
            LockScope::Job(job_id.clone()),
            LockScope::Destination(destination.to_path_buf()),
        ])?;
        Ok(Self {
            attempt,
            job_id,
            coordinator,
            guard,
        })
    }

    /// The immutable attempt identity this session authorizes.
    pub fn attempt(&self) -> &AttemptIdentity {
        &self.attempt
    }

    /// The job whose execution, recovery and retry this session serialises.
    pub fn job_id(&self) -> &str {
        &self.job_id
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
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;
    use crate::coordination::unix::LinuxCoordinator;
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

    #[test]
    fn session_authorizes_its_own_attempt_and_rejects_stale_or_foreign_ones() {
        let directory = lock_directory("authorize");
        let coordinator = LinuxCoordinator::new(&directory).expect("coordinator");
        let session = ExecutionSession::begin(
            &coordinator,
            attempt_of("job-1", 3),
            Path::new("/archive/final/job-1"),
        )
        .expect("session");

        assert_eq!(session.job_id(), "job-1");
        assert_eq!(session.attempt().attempt(), 3);
        assert!(session.authorize(&attempt_of("job-1", 3)).is_ok());
        assert_eq!(
            session.authorize(&attempt_of("job-1", 2)),
            Err(SessionError::StaleAttempt)
        );
        // A different job must never be authorized by this session, even with
        // a numerically larger attempt number.
        assert_eq!(
            session.authorize(&attempt_of("job-2", 9)),
            Err(SessionError::StaleAttempt)
        );

        drop(session);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_live_session_makes_a_second_session_for_the_same_job_contended() {
        let directory = lock_directory("same-job");
        let first = LinuxCoordinator::new(&directory).expect("first coordinator");
        let second = LinuxCoordinator::new(&directory).expect("second coordinator");
        let destination = Path::new("/archive/final/job-1");

        let session = ExecutionSession::begin(&first, attempt_of("job-1", 1), destination)
            .expect("first session");
        assert_eq!(
            ExecutionSession::begin(&second, attempt_of("job-1", 2), destination).err(),
            Some(SessionError::Contended),
            "a second owner of the same job must be told it is busy, not failed"
        );

        drop(session);
        ExecutionSession::begin(&second, attempt_of("job-1", 2), destination)
            .expect("scope released with the session");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn an_empty_destination_is_refused_as_a_scope() {
        let directory = lock_directory("empty-destination");
        let coordinator = LinuxCoordinator::new(&directory).expect("coordinator");
        assert_eq!(
            ExecutionSession::begin(&coordinator, attempt_of("job-1", 1), Path::new("")).err(),
            Some(SessionError::InvalidScope),
            "an empty destination would collapse every destination onto one lock"
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
