//! Telegram archive intent journal.
//!
//! An intent records the planning facts of a Telegram archive before the
//! directory is renamed, and advances them through an explicit state machine.
//! The SQL, transition rules and error mapping are unchanged from the previous
//! single-file implementation: repeating an identical write is idempotent and
//! changing the recorded facts is refused instead of silently redirecting
//! queued work.

use rusqlite::OptionalExtension;

use crate::{Database, StorageError, TelegramArchiveIntentRecord};

impl Database {
    /// Persist the immutable planning facts before the archive directory is
    /// renamed. Repeating the same operation is safe; changing its facts is
    /// rejected rather than silently redirecting queued work.
    pub fn prepare_telegram_archive_intent(
        &self,
        record: &TelegramArchiveIntentRecord,
    ) -> Result<(), StorageError> {
        if record.state != "PREPARED" && record.state != "SKIPPED" {
            return Err(StorageError::InvalidState(
                "invalid initial archive intent state".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO telegram_archive_intents \
             (tweet_id, job_id, archive_directory, state, metadata_text, media_json, bot_identity, chat_id, \
              message_thread_id, config_revision, plan_version, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
             ON CONFLICT(tweet_id) DO NOTHING",
            rusqlite::params![
                record.tweet_row_id, record.job_id, record.archive_directory, record.state,
                record.metadata_text, record.media_json, record.bot_identity, record.chat_id,
                record.message_thread_id, record.config_revision, record.plan_version,
                record.created_at, record.updated_at,
            ],
        )?;
        let existing = self
            .telegram_archive_intent(record.tweet_row_id)?
            .map(|mut existing| {
                // Lifecycle progress is not part of the immutable intent identity.
                existing.state = record.state.clone();
                existing.updated_at = record.updated_at.clone();
                existing
            });
        if existing.as_ref() != Some(record) {
            return Err(StorageError::InvalidState(
                "archive intent already exists with different immutable facts".into(),
            ));
        }
        Ok(())
    }

    pub fn telegram_archive_intent(
        &self,
        tweet_row_id: i64,
    ) -> Result<Option<TelegramArchiveIntentRecord>, StorageError> {
        Ok(self.connection.query_row(
            "SELECT job_id, tweet_id, archive_directory, state, metadata_text, media_json, bot_identity, \
             chat_id, message_thread_id, config_revision, plan_version, created_at, updated_at \
             FROM telegram_archive_intents WHERE tweet_id = ?1",
            [tweet_row_id],
            |row| Ok(TelegramArchiveIntentRecord {
                job_id: row.get(0)?, tweet_row_id: row.get(1)?, archive_directory: row.get(2)?,
                state: row.get(3)?, metadata_text: row.get(4)?, media_json: row.get(5)?,
                bot_identity: row.get(6)?, chat_id: row.get(7)?, message_thread_id: row.get(8)?,
                config_revision: row.get(9)?, plan_version: row.get(10)?, created_at: row.get(11)?,
                updated_at: row.get(12)?,
            }),
        ).optional()?)
    }

    /// Advance the journal using compare-and-set semantics. This small API
    /// deliberately cannot rewrite the captured archive or target facts.
    pub fn transition_telegram_archive_intent(
        &self,
        tweet_row_id: i64,
        expected_state: &str,
        next_state: &str,
        now: &str,
    ) -> Result<bool, StorageError> {
        let allowed = matches!(
            (expected_state, next_state),
            ("PREPARED", "ARCHIVED") | ("ARCHIVED", "QUEUED")
        );
        if !allowed {
            return Err(StorageError::InvalidState(
                "invalid archive intent transition".into(),
            ));
        }
        Ok(self.connection.execute(
            "UPDATE telegram_archive_intents SET state = ?1, updated_at = ?2 \
             WHERE tweet_id = ?3 AND state = ?4",
            rusqlite::params![next_state, now, tweet_row_id, expected_state],
        )? == 1)
    }

    pub fn list_recoverable_telegram_archive_intents(
        &self,
    ) -> Result<Vec<TelegramArchiveIntentRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT job_id, tweet_id, archive_directory, state, metadata_text, media_json, bot_identity, \
             chat_id, message_thread_id, config_revision, plan_version, created_at, updated_at \
             FROM telegram_archive_intents WHERE state IN ('PREPARED', 'ARCHIVED') ORDER BY created_at, tweet_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TelegramArchiveIntentRecord {
                job_id: row.get(0)?,
                tweet_row_id: row.get(1)?,
                archive_directory: row.get(2)?,
                state: row.get(3)?,
                metadata_text: row.get(4)?,
                media_json: row.get(5)?,
                bot_identity: row.get(6)?,
                chat_id: row.get(7)?,
                message_thread_id: row.get(8)?,
                config_revision: row.get(9)?,
                plan_version: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
