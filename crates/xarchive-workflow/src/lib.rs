//! Platform-neutral archive workflow invariants.
//!
//! This crate has two layers. The identity, fencing and phase types at the top
//! are pure logic with no dependencies. The `coordination` module defines a
//! port for cross-process advisory locking plus its Unix adapter; that
//! adapter exists because a kernel-enforced lock is the only mechanism that
//! makes "an owner that dies releases the lock" true without a lease timeout.
//!
//! The Windows adapter is intentionally absent. Windows advisory locking has
//! different semantics and must be implemented and validated on NTFS by the
//! Windows Owner, so this crate stays `CROSS_PLATFORM_CHANGE_REQUIRED` rather
//! than pretending one portable mechanism covers both platforms.

use std::fmt;

pub mod coordination;

/// Identity of one fenced execution attempt for a durable archive job.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttemptIdentity {
    job_id: String,
    attempt: u32,
}

impl AttemptIdentity {
    pub fn new(job_id: impl Into<String>, attempt: u32) -> Result<Self, WorkflowError> {
        let job_id = job_id.into();
        if job_id.is_empty() || job_id.len() > 128 || job_id.contains('\0') || attempt == 0 {
            return Err(WorkflowError::InvalidIdentity);
        }
        Ok(Self { job_id, attempt })
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }
}

/// Irreversible commit phase for an archive attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPhase {
    /// Cancellation may still win; no irreversible filesystem work is allowed.
    Active,
    /// Durable PREPARED is the irreversible boundary; cancellation must lose.
    Prepared,
    Committed,
}

impl CommitPhase {
    pub fn prepare(self) -> Result<Self, WorkflowError> {
        match self {
            Self::Active => Ok(Self::Prepared),
            Self::Prepared | Self::Committed => Err(WorkflowError::InvalidTransition),
        }
    }

    pub fn commit(self) -> Result<Self, WorkflowError> {
        match self {
            Self::Prepared => Ok(Self::Committed),
            Self::Active | Self::Committed => Err(WorkflowError::InvalidTransition),
        }
    }

    pub const fn cancellation_is_allowed(self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Rejects stale worker results without consulting platform or persistence APIs.
pub fn attempt_may_mutate(current: &AttemptIdentity, writer: &AttemptIdentity) -> bool {
    current == writer
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowError {
    InvalidIdentity,
    InvalidTransition,
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity => formatter.write_str("invalid archive attempt identity"),
            Self::InvalidTransition => formatter.write_str("invalid archive commit transition"),
        }
    }
}

impl std::error::Error for WorkflowError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_identity_fences_stale_workers() {
        let active = AttemptIdentity::new("job-1", 2).expect("valid attempt");
        let stale = AttemptIdentity::new("job-1", 1).expect("valid attempt");
        assert!(attempt_may_mutate(&active, &active));
        assert!(!attempt_may_mutate(&active, &stale));
    }

    #[test]
    fn prepared_is_irreversible_and_only_prepared_can_commit() {
        assert!(CommitPhase::Active.cancellation_is_allowed());
        let prepared = CommitPhase::Active.prepare().expect("prepare");
        assert!(!prepared.cancellation_is_allowed());
        assert_eq!(prepared.commit(), Ok(CommitPhase::Committed));
        assert_eq!(
            CommitPhase::Active.commit(),
            Err(WorkflowError::InvalidTransition)
        );
        assert_eq!(
            CommitPhase::Committed.prepare(),
            Err(WorkflowError::InvalidTransition)
        );
    }

    #[test]
    fn rejects_invalid_attempt_identity() {
        assert_eq!(
            AttemptIdentity::new("", 1),
            Err(WorkflowError::InvalidIdentity)
        );
        assert_eq!(
            AttemptIdentity::new("job-1", 0),
            Err(WorkflowError::InvalidIdentity)
        );
    }

    #[test]
    fn rename_edges_reject_escapes_and_duplicates() {
        assert!(
            crate::coordination::RenameEdge::new("a.jpg", "t/a.jpg", "a.jpg").is_err(),
            "source and final must differ"
        );
        assert!(
            crate::coordination::RenameEdge::new("a.jpg", "a.jpg", "b.jpg").is_err(),
            "source and temporary must differ"
        );
        assert!(
            crate::coordination::RenameEdge::new("../evil", "t/a", "b").is_err(),
            "parent escape must be rejected"
        );
        assert!(
            crate::coordination::RenameEdge::new("/abs", "t/a", "b").is_err(),
            "absolute paths must be rejected"
        );
        let edge = crate::coordination::RenameEdge::new("a.jpg", ".tmp/a.jpg", "final/a.jpg")
            .expect("valid edge");
        assert_eq!(
            edge.temporary_edge(),
            (
                std::path::Path::new("a.jpg"),
                std::path::Path::new(".tmp/a.jpg")
            )
        );
        assert_eq!(
            edge.final_edge(),
            (
                std::path::Path::new(".tmp/a.jpg"),
                std::path::Path::new("final/a.jpg")
            )
        );
    }
}
