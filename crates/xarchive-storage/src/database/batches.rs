use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::*;

/// Candidate counts for one batch, keyed by candidate state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BatchCounts {
    pub total: u64,
    pub pending: u64,
    pub submitted: u64,
    pub done: u64,
    pub failed: u64,
    pub skipped: u64,
    pub cancelled: u64,
}

/// One batch row plus its candidate counts, exposed to the Desktop UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountBatchSummary {
    pub id: String,
    pub username: String,
    pub profile_url: String,
    pub user_id: Option<String>,
    pub browser: Option<String>,
    pub profile: Option<String>,
    pub filters_json: String,
    pub state: String,
    pub discovery_state: String,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub retry_at_ms: Option<i64>,
    pub counts: BatchCounts,
    pub created_at: String,
    pub updated_at: String,
}

/// One candidate row used by the batch dispatcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchCandidateRecord {
    pub tweet_id: String,
    pub url: String,
    pub created_at: Option<String>,
    pub tweet_type: String,
    pub is_repost: bool,
    pub has_media: bool,
    pub media_count: u64,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub state: String,
    pub job_id: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub skip_reason: Option<String>,
}

/// Insert input for one discovered candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBatchCandidate {
    pub tweet_id: String,
    pub url: String,
    pub created_at: Option<String>,
    pub tweet_type: String,
    pub is_repost: bool,
    pub has_media: bool,
    pub media_count: u64,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub state: String,
    pub skip_reason: Option<String>,
}

const BATCH_SUMMARY_SELECT: &str = "SELECT b.id, b.username, b.profile_url, b.user_id, b.browser, b.profile, b.filters_json, b.state, b.discovery_state, b.last_error_code, b.last_error_message, b.retry_at_ms, b.created_at, b.updated_at, COALESCE(c.total, 0), COALESCE(c.pending, 0), COALESCE(c.submitted, 0), COALESCE(c.done, 0), COALESCE(c.failed, 0), COALESCE(c.skipped, 0), COALESCE(c.cancelled, 0) FROM archive_batches b LEFT JOIN (SELECT batch_id, COUNT(*) AS total, SUM(state = 'PENDING') AS pending, SUM(state = 'SUBMITTED') AS submitted, SUM(state = 'DONE') AS done, SUM(state = 'FAILED') AS failed, SUM(state = 'SKIPPED') AS skipped, SUM(state = 'CANCELLED') AS cancelled FROM batch_candidates GROUP BY batch_id) c ON c.batch_id = b.id";

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountBatchSummary> {
    Ok(AccountBatchSummary {
        id: row.get(0)?,
        username: row.get(1)?,
        profile_url: row.get(2)?,
        user_id: row.get(3)?,
        browser: row.get(4)?,
        profile: row.get(5)?,
        filters_json: row.get(6)?,
        state: row.get(7)?,
        discovery_state: row.get(8)?,
        last_error_code: row.get(9)?,
        last_error_message: row.get(10)?,
        retry_at_ms: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        counts: BatchCounts {
            total: row.get::<_, i64>(14)?.max(0) as u64,
            pending: row.get::<_, i64>(15)?.max(0) as u64,
            submitted: row.get::<_, i64>(16)?.max(0) as u64,
            done: row.get::<_, i64>(17)?.max(0) as u64,
            failed: row.get::<_, i64>(18)?.max(0) as u64,
            skipped: row.get::<_, i64>(19)?.max(0) as u64,
            cancelled: row.get::<_, i64>(20)?.max(0) as u64,
        },
    })
}

fn candidate_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BatchCandidateRecord> {
    Ok(BatchCandidateRecord {
        tweet_id: row.get(0)?,
        url: row.get(1)?,
        created_at: row.get(2)?,
        tweet_type: row.get(3)?,
        is_repost: row.get(4)?,
        has_media: row.get(5)?,
        media_count: row.get::<_, i64>(6)?.max(0) as u64,
        user_id: row.get(7)?,
        username: row.get(8)?,
        state: row.get(9)?,
        job_id: row.get(10)?,
        error_code: row.get(11)?,
        error_message: row.get(12)?,
        skip_reason: row.get(13)?,
    })
}

fn validate_candidate_state(state: &str) -> Result<(), StorageError> {
    if matches!(
        state,
        "PENDING" | "SUBMITTED" | "DONE" | "FAILED" | "SKIPPED" | "CANCELLED"
    ) {
        Ok(())
    } else {
        Err(StorageError::InvalidMetadata(format!(
            "invalid candidate state: {state}"
        )))
    }
}

impl Database {
    pub fn create_account_batch(
        &self,
        id: &str,
        username: &str,
        profile_url: &str,
        browser: Option<&str>,
        profile: Option<&str>,
        filters_json: &str,
    ) -> Result<(), StorageError> {
        if id.trim().is_empty() || username.trim().is_empty() || profile_url.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "batch id, username and profile_url are required".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO archive_batches (id, username, profile_url, browser, profile, filters_json, state, discovery_state) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'ACTIVE', 'PENDING')",
            params![id, username, profile_url, browser, profile, filters_json],
        )?;
        Ok(())
    }

    pub fn account_batch(&self, id: &str) -> Result<Option<AccountBatchSummary>, StorageError> {
        let sql = format!("{BATCH_SUMMARY_SELECT} WHERE b.id = ?1");
        Ok(self
            .connection
            .query_row(&sql, params![id], summary_from_row)
            .optional()?)
    }

    pub fn list_account_batches(
        &self,
        limit: u32,
    ) -> Result<Vec<AccountBatchSummary>, StorageError> {
        let sql = format!("{BATCH_SUMMARY_SELECT} ORDER BY b.created_at DESC, b.id DESC LIMIT ?1");
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params![limit], summary_from_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    /// Current lifecycle state used by the batch worker's control checks.
    pub fn account_batch_state(&self, id: &str) -> Result<Option<String>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT state FROM archive_batches WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn set_account_batch_state(&self, id: &str, state: &str) -> Result<(), StorageError> {
        if !matches!(
            state,
            "ACTIVE" | "PAUSED" | "CANCELLED" | "COMPLETED" | "FAILED"
        ) {
            return Err(StorageError::InvalidMetadata(format!(
                "invalid account batch state: {state}"
            )));
        }
        let changed = self.connection.execute(
            "UPDATE archive_batches SET state = ?2, retry_at_ms = CASE WHEN ?2 = 'ACTIVE' THEN NULL ELSE retry_at_ms END, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id, state],
        )?;
        if changed == 0 {
            return Err(StorageError::InvalidMetadata(format!(
                "unknown batch: {id}"
            )));
        }
        Ok(())
    }

    pub fn set_account_batch_discovery_state(
        &self,
        id: &str,
        state: &str,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE archive_batches SET discovery_state = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id, state],
        )?;
        Ok(())
    }
    pub fn set_account_batch_error(
        &self,
        id: &str,
        code: Option<&str>,
        message: Option<&str>,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE archive_batches SET last_error_code = ?2, last_error_message = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id, code, message],
        )?;
        Ok(())
    }

    pub fn set_account_batch_retry_at(
        &self,
        id: &str,
        retry_at_ms: Option<i64>,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE archive_batches SET retry_at_ms = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id, retry_at_ms],
        )?;
        Ok(())
    }

    /// Bind the stable X user identity discovered during account discovery.
    pub fn resolve_account_batch_identity(
        &self,
        id: &str,
        user_id: Option<&str>,
        username: Option<&str>,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE archive_batches SET user_id = COALESCE(?2, user_id), username = COALESCE(?3, username), updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id, user_id, username],
        )?;
        Ok(())
    }

    /// Resume PAUSED batches whose rate-limit backoff elapsed (P2-B: rate
    /// limiting pauses with a bounded wait; auth failures wait for the user).
    pub fn resume_rate_limited_batches(&self, now_ms: i64) -> Result<u64, StorageError> {
        let changed = self.connection.execute(
            "UPDATE archive_batches SET state = 'ACTIVE', retry_at_ms = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE state = 'PAUSED' AND retry_at_ms IS NOT NULL AND retry_at_ms <= ?1",
            params![now_ms],
        )?;
        Ok(changed as u64)
    }

    /// Insert discovered candidates; duplicates within/between re-runs are ignored.
    pub fn insert_batch_candidates(
        &self,
        batch_id: &str,
        candidates: &[NewBatchCandidate],
    ) -> Result<u64, StorageError> {
        let mut inserted = 0u64;
        for candidate in candidates {
            validate_candidate_state(candidate.state.as_str())?;
            let changed = self.connection.execute(
                "INSERT INTO batch_candidates (batch_id, tweet_id, url, created_at, tweet_type, is_repost, has_media, media_count, user_id, username, state, skip_reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(batch_id, tweet_id) DO NOTHING",
                params![
                    batch_id,
                    candidate.tweet_id,
                    candidate.url,
                    candidate.created_at,
                    candidate.tweet_type,
                    candidate.is_repost,
                    candidate.has_media,
                    candidate.media_count.min(i64::MAX as u64) as i64,
                    candidate.user_id,
                    candidate.username,
                    candidate.state,
                    candidate.skip_reason,
                ],
            )?;
            inserted += changed as u64;
        }
        Ok(inserted)
    }
    pub fn list_batch_candidates(
        &self,
        batch_id: &str,
        state: Option<&str>,
        limit: u32,
    ) -> Result<Vec<BatchCandidateRecord>, StorageError> {
        let base = "SELECT tweet_id, url, created_at, tweet_type, is_repost, has_media, media_count, user_id, username, state, job_id, error_code, error_message, skip_reason FROM batch_candidates";
        let sql = match state {
            Some(_) => format!(
                "{base} WHERE batch_id = ?1 AND state = ?2 ORDER BY created_at IS NULL, created_at ASC, tweet_id ASC LIMIT ?3"
            ),
            None => format!(
                "{base} WHERE batch_id = ?1 ORDER BY created_at IS NULL, created_at ASC, tweet_id ASC LIMIT ?2"
            ),
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = match state {
            Some(state) => {
                statement.query_map(params![batch_id, state, limit], candidate_from_row)?
            }
            None => statement.query_map(params![batch_id, limit], candidate_from_row)?,
        };
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn batch_candidate_counts(&self, batch_id: &str) -> Result<BatchCounts, StorageError> {
        let row = self.connection.query_row(
            "SELECT COUNT(*), SUM(state = 'PENDING'), SUM(state = 'SUBMITTED'), SUM(state = 'DONE'), SUM(state = 'FAILED'), SUM(state = 'SKIPPED'), SUM(state = 'CANCELLED') FROM batch_candidates WHERE batch_id = ?1",
            params![batch_id],
            |row| {
                Ok(BatchCounts {
                    total: row.get::<_, i64>(0)?.max(0) as u64,
                    pending: row.get::<_, i64>(1)?.max(0) as u64,
                    submitted: row.get::<_, i64>(2)?.max(0) as u64,
                    done: row.get::<_, i64>(3)?.max(0) as u64,
                    failed: row.get::<_, i64>(4)?.max(0) as u64,
                    skipped: row.get::<_, i64>(5)?.max(0) as u64,
                    cancelled: row.get::<_, i64>(6)?.max(0) as u64,
                })
            },
        )?;
        Ok(row)
    }
    /// Transition one candidate, writing job/error/skip facts in one update.
    #[allow(clippy::too_many_arguments)]
    pub fn mark_batch_candidate(
        &self,
        batch_id: &str,
        tweet_id: &str,
        state: &str,
        job_id: Option<&str>,
        error_code: Option<&str>,
        error_message: Option<&str>,
        skip_reason: Option<&str>,
    ) -> Result<(), StorageError> {
        validate_candidate_state(state)?;
        let changed = self.connection.execute(
            "UPDATE batch_candidates SET state = ?3, job_id = ?4, error_code = ?5, error_message = ?6, skip_reason = ?7, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE batch_id = ?1 AND tweet_id = ?2",
            params![batch_id, tweet_id, state, job_id, error_code, error_message, skip_reason],
        )?;
        if changed == 0 {
            return Err(StorageError::InvalidMetadata(format!(
                "unknown batch candidate: {batch_id}/{tweet_id}"
            )));
        }
        Ok(())
    }

    /// Retry semantics: FAILED candidates return to PENDING with a clean slate.
    pub fn retry_failed_batch_candidates(&self, batch_id: &str) -> Result<u64, StorageError> {
        let changed = self.connection.execute(
            "UPDATE batch_candidates SET state = 'PENDING', job_id = NULL, error_code = NULL, error_message = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE batch_id = ?1 AND state = 'FAILED'",
            params![batch_id],
        )?;
        Ok(changed as u64)
    }

    /// Cancel semantics stops future dispatch: only un-dispatched candidates
    /// become CANCELLED; submitted jobs keep running to completion.
    pub fn cancel_pending_batch_candidates(&self, batch_id: &str) -> Result<u64, StorageError> {
        let changed = self.connection.execute(
            "UPDATE batch_candidates SET state = 'CANCELLED', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE batch_id = ?1 AND state = 'PENDING'",
            params![batch_id],
        )?;
        Ok(changed as u64)
    }
}
