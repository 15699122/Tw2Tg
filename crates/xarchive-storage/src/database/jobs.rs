use rusqlite::{OptionalExtension, params};

use crate::*;

use std::io;
use xarchive_core::{JobEvent, JobState};

impl Database {
    pub fn create_archive_job(
        &self,
        job_id: &str,
        tweet_row_id: i64,
        now: &str,
    ) -> Result<bool, StorageError> {
        let existing: Option<String> = self.connection.query_row(
            "SELECT id FROM jobs WHERE tweet_id = ?1 AND job_type = 'archive' AND state IN ('QUEUED','VALIDATING','METADATA_READY','TG_METADATA_SENDING','TG_METADATA_SENT','DOWNLOADING','DOWNLOADED','TG_MEDIA_UPLOADING') LIMIT 1",
            params![tweet_row_id],
            |row| row.get(0),
        ).optional()?;
        if existing.is_some() {
            return Ok(false);
        }
        self.connection.execute(
            "INSERT INTO jobs (id, tweet_id, job_type, state, created_at, updated_at) VALUES (?1, ?2, 'archive', 'QUEUED', ?3, ?3)",
            params![job_id, tweet_row_id, now],
        )?;
        Ok(true)
    }

    pub fn job_state(&self, job_id: &str) -> Result<JobState, StorageError> {
        let value: String = self.connection.query_row(
            "SELECT state FROM jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )?;
        JobState::parse(&value).map_err(|_| StorageError::InvalidState(value))
    }

    pub fn list_recent_jobs(&self, limit: u32) -> Result<Vec<JobSummary>, StorageError> {
        let limit = i64::from(limit.clamp(1, 100));
        let mut statement = self.connection.prepare(
            "SELECT jobs.id, tweets.tweet_id, tweets.tweet_type, jobs.state, jobs.created_at, jobs.updated_at, jobs.last_error_code, jobs.last_error_message FROM jobs JOIN tweets ON tweets.id = jobs.tweet_id ORDER BY jobs.updated_at DESC, jobs.id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], |row| {
            let state: String = row.get(3)?;
            let state = JobState::parse(&state).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "unknown persisted job state",
                    )),
                )
            })?;
            Ok(JobSummary {
                job_id: row.get(0)?,
                tweet_id: row.get(1)?,
                tweet_type: row.get(2)?,
                state,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                last_error_code: row.get(6)?,
                last_error_message: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn transition_job(
        &mut self,
        job_id: &str,
        next: JobState,
        now: &str,
    ) -> Result<(), StorageError> {
        let current = self.job_state(job_id)?;
        current
            .transition_to(next)
            .map_err(StorageError::InvalidTransition)?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE jobs SET state = ?1, started_at = CASE WHEN ?1 = 'DOWNLOADING' AND started_at IS NULL THEN ?2 ELSE started_at END, finished_at = CASE WHEN ?1 IN ('COMPLETE','CANCELLED') THEN ?2 ELSE finished_at END, updated_at = ?2 WHERE id = ?3",
            params![next.as_str(), now, job_id],
        )?;
        transaction.execute(
            "INSERT INTO events (job_id, event_type, previous_state, new_state, created_at) VALUES (?1, 'JOB_STATE_CHANGED', ?2, ?3, ?4)",
            params![job_id, current.as_str(), next.as_str(), now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Persist a classified job failure while preserving the normal state
    /// transition rules. A newly-created job first enters `VALIDATING`, so a
    /// failed extraction can be retried without creating a second active job.
    pub fn fail_job(
        &mut self,
        job_id: &str,
        next: JobState,
        error_code: &str,
        error_message: &str,
        now: &str,
    ) -> Result<(), StorageError> {
        if !matches!(next, JobState::AuthRequired | JobState::Failed) {
            return Err(StorageError::InvalidState(format!(
                "job failure must use AUTH_REQUIRED or FAILED, got {}",
                next.as_str()
            )));
        }
        if self.job_state(job_id)? == JobState::Queued {
            self.transition_job(job_id, JobState::Validating, now)?;
        }
        if self.job_state(job_id)? != next {
            self.transition_job(job_id, next, now)?;
        }
        self.connection.execute(
            "UPDATE jobs SET last_error_code = ?1, last_error_message = ?2, updated_at = ?3 WHERE id = ?4",
            params![error_code, error_message, now, job_id],
        )?;
        Ok(())
    }

    pub fn count_job_events(&self, job_id: &str) -> Result<i64, StorageError> {
        Ok(self.connection.query_row(
            "SELECT COUNT(*) FROM events WHERE job_id = ?1",
            params![job_id],
            |row| row.get(0),
        )?)
    }

    pub fn record_event(
        &self,
        job_id: &str,
        event: &JobEvent,
        now: &str,
    ) -> Result<(), StorageError> {
        let event_type = event.event_type();
        let payload_json = serde_json::to_string(event).map_err(StorageError::Json)?;
        self.connection.execute(
            "INSERT INTO events (job_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![job_id, event_type, payload_json, now],
        )?;
        Ok(())
    }

    pub fn list_events_for_job(
        &self,
        job_id: &str,
        limit: u32,
    ) -> Result<Vec<(String, String, Option<String>)>, StorageError> {
        let limit = i64::from(limit.clamp(1, 100));
        let mut statement = self.connection.prepare(
            "SELECT event_type, payload_json, created_at FROM events WHERE job_id = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![job_id, limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }
}
