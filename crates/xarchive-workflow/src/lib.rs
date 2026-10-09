//! Platform-neutral archive workflow invariants.
//!
//! This crate deliberately contains no operating-system locking or filesystem
//! adapters. Those adapters and production wiring are a separate cross-platform
//! handoff item; the types here only make workflow identities and transitions
//! explicit and testable.

use std::fmt;

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
}
