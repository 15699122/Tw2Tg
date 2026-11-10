//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl Database {
    pub fn archive_recovery_contract(&self, job_id: &str) -> Result<Option<i64>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT recovery_contract_version FROM jobs WHERE id = ?1",
                [job_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten())
    }

    /// Whether a durable archive commit boundary already owns this job.
    ///
    /// Both journal generations count: a PREPARED or COMMITTED row in either
    /// table means the archive commit has claimed the job, and cancellation
    /// must never write a cancelled state over it. The read is advisory — the
    /// authoritative check happens inside
    /// [`Self::decide_archive_cancellation`]'s transaction.
    pub fn archive_commit_started(&self, job_id: &str) -> Result<bool, StorageError> {
        if self.v2_journal_state(job_id)?.is_some() {
            return Ok(true);
        }
        let v1: Option<String> = self
            .connection
            .query_row(
                "SELECT phase FROM archive_recovery_journal WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(v1.is_some())
    }

    /// Atomically decide and persist cancellation against the commit boundary.
    ///
    /// One transaction re-checks both journals, validates the state
    /// transition, and writes the cancelled state together with its
    /// state-change event. A prepare committing concurrently can therefore
    /// never be overwritten by `CANCELLED`, and `CANCELLED` can never be
    /// written over a PREPARED journal: whichever side commits first wins and
    /// the other observes it here.
    pub fn decide_archive_cancellation(
        &self,
        job_id: &str,
        now: &str,
    ) -> Result<ArchiveCancellationDecision, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let v2: Option<String> = transaction
            .query_row(
                "SELECT phase FROM archive_recovery_v2 WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .optional()?;
        let v1: Option<String> = transaction
            .query_row(
                "SELECT phase FROM archive_recovery_journal WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .optional()?;
        if v2.is_some() || v1.is_some() {
            // No writes: dropping the transaction rolls back.
            return Ok(ArchiveCancellationDecision::CommitWins);
        }
        let current: String =
            transaction.query_row("SELECT state FROM jobs WHERE id = ?1", [job_id], |row| {
                row.get(0)
            })?;
        let current = xarchive_core::JobState::parse(&current)
            .map_err(|_| StorageError::InvalidState(current))?;
        if current.is_terminal() {
            return Ok(ArchiveCancellationDecision::AlreadyDecided);
        }
        current
            .transition_to(xarchive_core::JobState::Cancelled)
            .map_err(StorageError::InvalidTransition)?;
        // Mirrors Database::transition_job for a cancellation, inside the same
        // transaction as the boundary check.
        transaction.execute(
            "UPDATE jobs SET state = 'CANCELLED', finished_at = ?1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, job_id],
        )?;
        transaction.execute(
            "INSERT INTO events (job_id, event_type, previous_state, new_state, created_at) VALUES (?1, 'JOB_STATE_CHANGED', ?2, 'CANCELLED', ?3)",
            rusqlite::params![job_id, current.as_str(), now],
        )?;
        transaction.commit()?;
        Ok(ArchiveCancellationDecision::Cancelled)
    }

    pub(super) fn v2_journal_state(
        &self,
        job_id: &str,
    ) -> Result<Option<ArchiveRecoveryPhase>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT phase FROM archive_recovery_v2 WHERE job_id = ?1",
                [job_id],
                |row| {
                    let phase: String = row.get(0)?;
                    match phase.as_str() {
                        "PREPARED" => Ok(ArchiveRecoveryPhase::Prepared),
                        "COMMITTED" => Ok(ArchiveRecoveryPhase::Committed),
                        _ => Err(rusqlite::Error::InvalidQuery),
                    }
                },
            )
            .optional()?)
    }
}
