//! SQLite persistence and local file storage for the Desktop application.

mod archive_service;
mod database {
    pub mod batches;
    pub mod jobs;
    pub mod settings;
    pub mod tags;
    pub mod telegram;
    pub mod telegram_outbox;
    pub mod tweets;
    pub mod users;
}
mod archive_completeness;
mod error;
mod file_store;
mod metadata;
mod models;

pub use archive_completeness::{
    ArchiveCompleteness, ArchiveCompletenessOptions, CompletenessIssue,
    evaluate_archive_completeness, safe_media_path,
};
pub use archive_service::{ArchiveService, SidecarArchiveRequest};
pub use database::batches::{
    AccountBatchSummary, BatchCandidateRecord, BatchCounts, NewBatchCandidate,
};
pub use error::StorageError;
pub use file_store::FileStore;
pub use metadata::build_archive_metadata;
pub use models::{
    ArchivedMediaFact, JobDownloadMetrics, JobEventRecord, JobMetrics, JobSummary, PagedJobs, SettingEntry,
    TelegramArchiveIntentRecord, TweetArchiveFacts, TweetRelationships, UserNameSummary,
    UserProfileFile, UserProfileSnapshot, UserSummary,
};

use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_telegram_send_state.sql"),
    include_str!("../migrations/0003_quote_reply_relationships.sql"),
    include_str!("../migrations/0004_archive_job_requests.sql"),
    include_str!("../migrations/0005_account_batches.sql"),
    include_str!("../migrations/0006_batch_discovery_paused.sql"),
    include_str!("../migrations/0007_telegram_outbox.sql"),
    include_str!("../migrations/0008_telegram_outbox_snapshot.sql"),
    include_str!("../migrations/0009_telegram_archive_intents.sql"),
    include_str!("../migrations/0010_telegram_credential_generations.sql"),
    include_str!("../migrations/0011_telegram_resume_authorizations.sql"),
    include_str!("../migrations/0012_telegram_rotation_decisions.sql"),
    include_str!("../migrations/0013_telegram_claim_generation.sql"),
    include_str!("../migrations/0014_download_task_metrics.sql"),
];

pub struct Database {
    connection: Connection,
}

impl Database {
    /// Fence credential verification and claiming in one SQLite write transaction.
    /// A stale worker gets no work, even if the replacement authorized the row.
    pub fn claim_due_outbox_for_generation(
        &self,
        generation: i64,
        bot_identity: &str,
        claim_token: &str,
        now: &str,
        lease_until: &str,
    ) -> Result<Option<xarchive_telegram::OutboxEntry>, xarchive_telegram::SendStateError> {
        use xarchive_telegram::{SendStateError, TelegramOutboxStore};
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| SendStateError::Store("claim transaction unavailable".into()))?;
        // Acquire the writer lock before reading the active generation. A deferred
        // read alone would permit a competing activation between read and claim.
        transaction
            .execute(
                "UPDATE telegram_credential_generations SET status = status WHERE generation = ?1",
                [generation],
            )
            .map_err(|_| SendStateError::Store("credential fence unavailable".into()))?;
        let matches: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM telegram_credential_generations WHERE generation = ?1 AND bot_identity = ?2 AND status = 'ACTIVE')",
            rusqlite::params![generation, bot_identity], |row| row.get(0),
        ).map_err(|_| SendStateError::Store("credential fence unavailable".into()))?;
        if !matches {
            return Ok(None);
        }
        let claimed = self.claim_due_outbox(bot_identity, claim_token, now, lease_until)?;
        if claimed.is_some() {
            transaction.execute(
                "UPDATE telegram_outbox SET claim_generation = ?2 WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
                rusqlite::params![claim_token, generation],
            ).map_err(|_| SendStateError::Store("claim generation binding failed".into()))?;
        }
        transaction
            .commit()
            .map_err(|_| SendStateError::Store("claim commit failed".into()))?;
        Ok(claimed)
    }

    /// Capture a rotation decision once. Replays must match the original facts;
    /// changing settings does not retroactively change an existing decision.
    pub fn record_telegram_rotation_decision(
        &self,
        generation: i64,
        policy: &str,
        candidate_ids: &[i64],
        now: &str,
    ) -> Result<(), StorageError> {
        if !matches!(policy, "automatic" | "confirm") {
            return Err(StorageError::InvalidState("invalid rotation policy".into()));
        }
        let json = serde_json::to_string(candidate_ids)?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO telegram_rotation_decisions(generation, policy, candidate_ids_json, created_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
            rusqlite::params![generation, policy, json, now],
        )?;
        let existing: (String, String) = transaction.query_row(
            "SELECT policy, candidate_ids_json FROM telegram_rotation_decisions WHERE generation = ?1",
            [generation], |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if existing != (policy.to_owned(), json) {
            return Err(StorageError::InvalidState(
                "rotation decision conflict".into(),
            ));
        }
        if policy == "automatic" {
            let bot: String = transaction.query_row(
                "SELECT bot_identity FROM telegram_credential_generations WHERE generation = ?1 AND status = 'ACTIVE'",
                [generation], |row| row.get(0),
            )?;
            for id in candidate_ids {
                let eligible: bool = transaction.query_row(
                    "SELECT EXISTS(SELECT 1 FROM telegram_outbox WHERE id = ?1 AND bot_identity = ?2 AND state IN ('QUEUED', 'RETRY_WAIT') AND payload_json IS NOT NULL AND payload_schema_version = 1)",
                    rusqlite::params![id, bot], |row| row.get(0),
                )?;
                if !eligible {
                    return Err(StorageError::InvalidState(
                        "rotation candidate no longer eligible".into(),
                    ));
                }
                transaction.execute(
                    "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                    rusqlite::params![generation, id, now],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn telegram_rotation_decision(
        &self,
        generation: i64,
    ) -> Result<Option<(String, Vec<i64>)>, StorageError> {
        let record: Option<(String, String)> = self.connection.query_row(
            "SELECT policy, candidate_ids_json FROM telegram_rotation_decisions WHERE generation = ?1",
            [generation], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        record
            .map(|(policy, json)| Ok((policy, serde_json::from_str(&json)?)))
            .transpose()
    }

    /// Snapshot rows eligible for rotation; future rows are never included.
    pub fn telegram_resume_candidates(&self, generation: i64) -> Result<Vec<i64>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT o.id FROM telegram_outbox o JOIN telegram_credential_generations g ON g.bot_identity = o.bot_identity WHERE g.generation = ?1 AND g.status = 'ACTIVE' AND o.state IN ('QUEUED', 'RETRY_WAIT') AND o.payload_json IS NOT NULL AND o.payload_schema_version = 1 ORDER BY o.id",
        )?;
        Ok(statement
            .query_map([generation], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Grant only explicitly selected recoverable rows to the current generation.
    /// Any invalid selection rolls back the entire grant; UNKNOWN is never eligible.
    pub fn authorize_telegram_resume(
        &self,
        generation: i64,
        outbox_ids: &[i64],
        now: &str,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let bot: Option<String> = transaction.query_row(
            "SELECT bot_identity FROM telegram_credential_generations WHERE generation = ?1 AND status = 'ACTIVE'",
            [generation], |row| row.get(0),
        ).optional()?;
        let bot =
            bot.ok_or_else(|| StorageError::InvalidState("active credential changed".into()))?;
        for id in outbox_ids {
            let eligible: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM telegram_outbox WHERE id = ?1 AND bot_identity = ?2 AND state IN ('QUEUED', 'RETRY_WAIT') AND payload_json IS NOT NULL AND payload_schema_version = 1)",
                rusqlite::params![id, bot], |row| row.get(0),
            )?;
            if !eligible {
                return Err(StorageError::InvalidState(
                    "outbox row is not eligible for resume".into(),
                ));
            }
            transaction.execute(
                "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                rusqlite::params![generation, id, now],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Explicit operator review of one uncertain outcome. This is not an
    /// automatic retry: the caller must acknowledge possible duplication.
    /// Preserve unknown_reason as audit evidence until a subsequent result.
    pub fn review_telegram_unknown_for_resend(
        &self,
        generation: i64,
        outbox_id: i64,
        confirmed: bool,
        now: &str,
    ) -> Result<(), StorageError> {
        if !confirmed {
            return Err(StorageError::InvalidState(
                "duplicate-risk confirmation required".into(),
            ));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let changed = transaction.execute(
            "UPDATE telegram_outbox SET state = 'QUEUED', next_retry_at = NULL, updated_at = ?3 \
             WHERE id = ?2 AND state = 'UNKNOWN' AND payload_schema_version = 1 AND payload_json IS NOT NULL \
             AND EXISTS(SELECT 1 FROM telegram_credential_generations g WHERE g.generation = ?1 AND g.status = 'ACTIVE' AND g.bot_identity = telegram_outbox.bot_identity)",
            rusqlite::params![generation, outbox_id, now],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidState(
                "review target or credential changed".into(),
            ));
        }
        transaction.execute(
            "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
            rusqlite::params![generation, outbox_id, now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Revoke sending authority before deleting a native secret. A failed
    /// secret deletion leaves an inactive orphan, never an active credential.
    pub fn retire_telegram_credential_generation(&self, expected: i64) -> Result<(), StorageError> {
        let changed = self.connection.execute(
            "UPDATE telegram_credential_generations SET status = 'RETIRED' WHERE generation = ?1 AND status = 'ACTIVE'",
            [expected],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidState(
                "active credential changed".into(),
            ));
        }
        Ok(())
    }

    pub fn telegram_resume_authorized(
        &self,
        generation: i64,
        outbox_id: i64,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM telegram_resume_authorizations a JOIN telegram_credential_generations g ON g.generation = a.generation JOIN telegram_outbox o ON o.id = a.outbox_id WHERE a.generation = ?1 AND a.outbox_id = ?2 AND g.status = 'ACTIVE' AND g.bot_identity = o.bot_identity AND o.state IN ('QUEUED', 'RETRY_WAIT'))",
            rusqlite::params![generation, outbox_id], |row| row.get(0),
        )?)
    }

    /// Register an already verified identity and an opaque SecretStore key.
    /// Registration never activates a credential or authorizes queued sends.
    pub fn prepare_telegram_credential_generation(
        &self,
        bot_identity: &str,
        secret_reference: &str,
        now: &str,
    ) -> Result<i64, StorageError> {
        let id = bot_identity
            .strip_prefix("telegram-bot:")
            .and_then(|value| value.parse::<i64>().ok());
        if !id.is_some_and(|id| id > 0) || secret_reference.trim().is_empty() {
            return Err(StorageError::InvalidState(
                "invalid credential identity or reference".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO telegram_credential_generations(bot_identity, secret_reference, status, created_at) VALUES (?1, ?2, 'CANDIDATE', ?3)",
            rusqlite::params![bot_identity, secret_reference, now],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    /// Compare-and-set activation prevents a stale verification from replacing
    /// a newer active credential. This does not itself grant queue authority.
    pub fn activate_telegram_credential_generation(
        &self,
        candidate: i64,
        expected_active: Option<i64>,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let active: Option<i64> = transaction
            .query_row(
                "SELECT generation FROM telegram_credential_generations WHERE status = 'ACTIVE'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if active != expected_active {
            return Err(StorageError::InvalidState(
                "credential generation changed".into(),
            ));
        }
        let ready: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM telegram_credential_generations WHERE generation = ?1 AND status = 'CANDIDATE')",
            [candidate], |row| row.get(0),
        )?;
        if !ready {
            return Err(StorageError::InvalidState(
                "credential candidate unavailable".into(),
            ));
        }
        transaction.execute(
            "UPDATE telegram_credential_generations SET status = 'RETIRED' WHERE status = 'ACTIVE'",
            [],
        )?;
        transaction.execute(
            "UPDATE telegram_credential_generations SET status = 'ACTIVE' WHERE generation = ?1",
            [candidate],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn active_telegram_credential_generation(
        &self,
    ) -> Result<Option<(i64, String, String)>, StorageError> {
        Ok(self.connection.query_row(
            "SELECT generation, bot_identity, secret_reference FROM telegram_credential_generations WHERE status = 'ACTIVE'",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?)
    }

    /// Atomically activate a candidate and capture the rotation policy and
    /// eligible queue. SecretStore writes must already have succeeded.
    pub fn activate_telegram_credential_with_resume(
        &self,
        candidate: i64,
        expected_active: Option<i64>,
        policy: &str,
        now: &str,
    ) -> Result<Vec<i64>, StorageError> {
        if !matches!(policy, "automatic" | "confirm") {
            return Err(StorageError::InvalidState("invalid rotation policy".into()));
        }
        let tx = self.connection.unchecked_transaction()?;
        let previous: Option<(i64, String)> = tx.query_row(
            "SELECT generation, bot_identity FROM telegram_credential_generations WHERE status = 'ACTIVE'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        if previous.as_ref().map(|value| value.0) != expected_active {
            return Err(StorageError::InvalidState(
                "credential generation changed".into(),
            ));
        }
        let bot: String = tx.query_row(
            "SELECT bot_identity FROM telegram_credential_generations WHERE generation = ?1 AND status = 'CANDIDATE'",
            [candidate], |row| row.get(0),
        )?;
        let mut ids = Vec::<i64>::new();
        if previous.as_ref().is_some_and(|value| value.1 == bot) {
            let mut statement = tx.prepare(
                "SELECT id FROM telegram_outbox WHERE bot_identity = ?1 AND state IN ('QUEUED', 'RETRY_WAIT') AND payload_json IS NOT NULL AND payload_schema_version = 1 ORDER BY id",
            )?;
            ids = statement
                .query_map([&bot], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
        }
        tx.execute(
            "UPDATE telegram_credential_generations SET status = 'RETIRED' WHERE status = 'ACTIVE'",
            [],
        )?;
        tx.execute(
            "UPDATE telegram_credential_generations SET status = 'ACTIVE' WHERE generation = ?1",
            [candidate],
        )?;
        tx.execute(
            "INSERT INTO telegram_rotation_decisions(generation, policy, candidate_ids_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![candidate, policy, serde_json::to_string(&ids)?, now],
        )?;
        if policy == "automatic" {
            for id in &ids {
                tx.execute(
                    "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![candidate, id, now],
                )?;
            }
        }
        tx.commit()?;
        Ok(ids)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// Execute a narrow database operation for crate consumers that need a
    /// transactional recovery/state repair unsupported by the public API.
    pub fn execute_batch(&self, sql: &str) -> Result<(), StorageError> {
        self.connection.execute_batch(sql)?;
        Ok(())
    }

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

    fn from_connection(connection: Connection) -> Result<Self, StorageError> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
        )?;
        let current_version: i64 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        for (index, migration) in MIGRATIONS.iter().enumerate() {
            let version = (index + 1) as i64;
            if current_version >= version {
                continue;
            }
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(migration)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                [version],
            )?;
            transaction.commit()?;
        }
        Ok(Self { connection })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use xarchive_core::{ArchiveMetadata, JobState};
    use xarchive_telegram::{
        CachedFileId, FileCacheKey, FileIdCacheStore, MediaKind, NewOutboxEntry, OutboxState,
        SendState, SendStateError, SendStateStore, SentSendRecord, TELEGRAM_FILE_CACHE_VERSION,
        TelegramOutboxStore, UNKNOWN_REASON_LEASE_EXPIRED,
    };

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xarchive-storage-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn credential_generation_activation_is_atomic_and_fenced() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join("credentials.sqlite3");
        let database = Database::open(&path).expect("database");
        assert!(
            database
                .prepare_telegram_credential_generation("token-hash", "secret/1", "t0")
                .is_err()
        );
        let first = database
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/1", "t0")
            .expect("candidate");
        assert!(
            database
                .active_telegram_credential_generation()
                .unwrap()
                .is_none()
        );
        database
            .activate_telegram_credential_generation(first, None)
            .expect("activate");
        let second = database
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/2", "t1")
            .expect("rotation");
        assert!(
            database
                .activate_telegram_credential_generation(second, None)
                .is_err()
        );
        assert_eq!(
            database
                .active_telegram_credential_generation()
                .unwrap()
                .unwrap()
                .0,
            first
        );
        database
            .activate_telegram_credential_generation(second, Some(first))
            .expect("rotate");
        assert!(
            database
                .activate_telegram_credential_generation(first, Some(second))
                .is_err()
        );
        drop(database);
        let database = Database::open(&path).expect("reopen");
        assert_eq!(
            database.active_telegram_credential_generation().unwrap(),
            Some((second, "telegram-bot:42".into(), "secret/2".into()))
        );
        drop(database);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stale_credential_activation_from_another_connection_cannot_replace_winner() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join("activation.sqlite3");
        let first_connection = Database::open(&path).expect("first connection");
        let old = first_connection
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/old", "t0")
            .expect("old candidate");
        first_connection
            .activate_telegram_credential_generation(old, None)
            .expect("initial activation");
        let second_connection = Database::open(&path).expect("second connection");
        let winner = first_connection
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/winner", "t1")
            .expect("winner");
        let stale = second_connection
            .prepare_telegram_credential_generation("telegram-bot:43", "secret/stale", "t1")
            .expect("stale");
        first_connection
            .activate_telegram_credential_generation(winner, Some(old))
            .expect("winner activation");
        assert!(
            second_connection
                .activate_telegram_credential_generation(stale, Some(old))
                .is_err()
        );
        assert_eq!(
            second_connection
                .active_telegram_credential_generation()
                .expect("active"),
            Some((winner, "telegram-bot:42".into(), "secret/winner".into()))
        );
        drop(second_connection);
        drop(first_connection);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn automatic_rotation_grants_only_existing_same_bot_rows() {
        let database = Database::open_in_memory().unwrap();
        let old = database
            .prepare_telegram_credential_generation("telegram-bot:42", "old", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(old, None)
            .unwrap();
        let queued = database
            .enqueue_outbox(outbox_entry("telegram-bot:42", "queued", 0))
            .unwrap();
        let unknown = database
            .enqueue_outbox(outbox_entry("telegram-bot:42", "unknown", 0))
            .unwrap();
        database
            .execute_batch(&format!(
                "UPDATE telegram_outbox SET state = 'UNKNOWN' WHERE id = {unknown}"
            ))
            .unwrap();
        let other = database
            .enqueue_outbox(outbox_entry("telegram-bot:43", "other", 0))
            .unwrap();
        let next = database
            .prepare_telegram_credential_generation("telegram-bot:42", "next", "t1")
            .unwrap();
        assert_eq!(
            database
                .activate_telegram_credential_with_resume(next, Some(old), "automatic", "t2")
                .unwrap(),
            vec![queued]
        );
        assert!(database.telegram_resume_authorized(next, queued).unwrap());
        assert!(!database.telegram_resume_authorized(next, unknown).unwrap());
        assert!(!database.telegram_resume_authorized(next, other).unwrap());
        let future = database
            .enqueue_outbox(outbox_entry("telegram-bot:42", "future", 0))
            .unwrap();
        assert!(!database.telegram_resume_authorized(next, future).unwrap());
        let changed = database
            .prepare_telegram_credential_generation("telegram-bot:43", "changed", "t3")
            .unwrap();
        assert!(
            database
                .activate_telegram_credential_with_resume(changed, Some(next), "automatic", "t4")
                .unwrap()
                .is_empty()
        );
        assert!(!database.telegram_resume_authorized(changed, other).unwrap());
    }

    #[test]
    fn resume_grants_are_exact_atomic_and_generation_scoped() {
        let database = Database::open_in_memory().expect("database");
        let generation = database
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/1", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(generation, None)
            .unwrap();
        let first = database
            .enqueue_outbox(outbox_entry("telegram-bot:42", "first", 0))
            .unwrap();
        let other_bot = database
            .enqueue_outbox(outbox_entry("telegram-bot:43", "other", 0))
            .unwrap();
        assert!(
            database
                .authorize_telegram_resume(generation, &[first, other_bot], "t1")
                .is_err()
        );
        assert!(
            !database
                .telegram_resume_authorized(generation, first)
                .unwrap()
        );
        database
            .authorize_telegram_resume(generation, &[first], "t1")
            .unwrap();
        database
            .authorize_telegram_resume(generation, &[first], "t2")
            .unwrap();
        assert!(
            database
                .telegram_resume_authorized(generation, first)
                .unwrap()
        );
        let future = database
            .enqueue_outbox(outbox_entry("telegram-bot:42", "future", 0))
            .unwrap();
        assert!(
            !database
                .telegram_resume_authorized(generation, future)
                .unwrap()
        );
        database
            .execute_batch(&format!(
                "UPDATE telegram_outbox SET state = 'UNKNOWN' WHERE id = {first}"
            ))
            .unwrap();
        assert!(
            !database
                .telegram_resume_authorized(generation, first)
                .unwrap()
        );
        assert!(
            database
                .authorize_telegram_resume(generation, &[first], "t3")
                .is_err()
        );
        let rotated = database
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/2", "t3")
            .unwrap();
        database
            .activate_telegram_credential_generation(rotated, Some(generation))
            .unwrap();
        assert!(
            database
                .authorize_telegram_resume(generation, &[future], "t4")
                .is_err()
        );
        assert!(
            !database
                .telegram_resume_authorized(rotated, future)
                .unwrap()
        );
    }

    #[test]
    fn rotation_after_request_start_recovers_unknown_and_never_auto_resends() {
        let database = Database::open_in_memory().unwrap();
        let bot = "telegram-bot:42";
        let row = database
            .enqueue_outbox(outbox_entry(bot, "started-rotation", 0))
            .unwrap();
        let old = database
            .prepare_telegram_credential_generation(bot, "secret/started-old", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(old, None)
            .unwrap();
        database
            .authorize_telegram_resume(old, &[row], OUTBOX_NOW)
            .unwrap();
        database
            .claim_due_outbox_for_generation(old, bot, "started-old", OUTBOX_NOW, OUTBOX_LEASE)
            .unwrap()
            .unwrap();
        database
            .mark_request_started("started-old", OUTBOX_NOW)
            .unwrap();
        let new = database
            .prepare_telegram_credential_generation(bot, "secret/started-new", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(new, Some(old))
            .unwrap();
        assert!(matches!(
            database.record_outbox_sent("started-old", "77", None, OUTBOX_NOW),
            Err(SendStateError::StaleClaim)
        ));
        assert_eq!(database.recover_outbox_claims(OUTBOX_LEASE).unwrap(), 1);
        assert_eq!(database.recover_outbox_claims(OUTBOX_LEASE).unwrap(), 0);
        let rows = database.list_outbox_for_tweet(bot, 0).unwrap();
        assert!(rows.is_empty());
        assert!(
            database
                .authorize_telegram_resume(new, &[row], OUTBOX_LEASE)
                .is_err()
        );
        assert!(
            database
                .claim_due_outbox_for_generation(
                    new,
                    bot,
                    "started-new",
                    OUTBOX_LEASE,
                    OUTBOX_LATER
                )
                .unwrap()
                .is_none()
        );
        assert!(
            database
                .claim_outbox(
                    bot,
                    "started-rotation",
                    "manual-new",
                    OUTBOX_LEASE,
                    OUTBOX_LATER
                )
                .unwrap()
                .is_none()
        );
        let state: String = database
            .connection
            .query_row(
                "SELECT state FROM telegram_outbox WHERE id = ?1",
                [row],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "UNKNOWN");
    }

    #[test]
    fn explicit_unknown_review_requires_current_identity_and_duplicate_acknowledgement() {
        let database = Database::open_in_memory().unwrap();
        let mut entry = outbox_entry("telegram-bot:42", "review", 0);
        entry.payload_schema_version = 1;
        entry.payload_json = "{}".into();
        let row = database.enqueue_outbox(entry).unwrap();
        let generation = database
            .prepare_telegram_credential_generation("telegram-bot:42", "review-ref", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(generation, None)
            .unwrap();
        database
            .authorize_telegram_resume(generation, &[row], OUTBOX_NOW)
            .unwrap();
        database
            .claim_due_outbox_for_generation(
                generation,
                "telegram-bot:42",
                "review-claim",
                OUTBOX_NOW,
                OUTBOX_LEASE,
            )
            .unwrap()
            .unwrap();
        database
            .mark_request_started("review-claim", OUTBOX_NOW)
            .unwrap();
        database.recover_outbox_claims(OUTBOX_LEASE).unwrap();
        assert!(
            database
                .review_telegram_unknown_for_resend(generation, row, false, OUTBOX_LEASE)
                .is_err()
        );
        assert!(
            database
                .review_telegram_unknown_for_resend(generation + 1, row, true, OUTBOX_LEASE)
                .is_err()
        );
        assert!(
            database
                .claim_due_outbox_for_generation(
                    generation,
                    "telegram-bot:42",
                    "auto",
                    OUTBOX_LEASE,
                    OUTBOX_LATER
                )
                .unwrap()
                .is_none()
        );
        database
            .review_telegram_unknown_for_resend(generation, row, true, OUTBOX_LEASE)
            .unwrap();
        assert!(
            database
                .review_telegram_unknown_for_resend(generation, row, true, OUTBOX_LEASE)
                .is_err()
        );
        assert!(
            database
                .telegram_resume_authorized(generation, row)
                .unwrap()
        );
        assert!(
            database
                .claim_due_outbox_for_generation(
                    generation,
                    "telegram-bot:42",
                    "reviewed",
                    OUTBOX_LEASE,
                    OUTBOX_LATER
                )
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn rotation_after_claim_fences_request_renewal_and_result_writes() {
        let database = Database::open_in_memory().unwrap();
        let bot = "telegram-bot:42";
        let row = database
            .enqueue_outbox(outbox_entry(bot, "rotation-after-claim", 0))
            .unwrap();
        let old = database
            .prepare_telegram_credential_generation(bot, "secret/claimed-old", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(old, None)
            .unwrap();
        database
            .authorize_telegram_resume(old, &[row], OUTBOX_NOW)
            .unwrap();
        database
            .claim_due_outbox_for_generation(old, bot, "claimed-old", OUTBOX_NOW, OUTBOX_LEASE)
            .unwrap()
            .unwrap();
        let new = database
            .prepare_telegram_credential_generation(bot, "secret/claimed-new", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(new, Some(old))
            .unwrap();
        assert!(matches!(
            database.mark_request_started("claimed-old", OUTBOX_NOW),
            Err(SendStateError::StaleClaim)
        ));
        assert!(
            !database
                .renew_outbox_claim("claimed-old", OUTBOX_LEASE, OUTBOX_NOW)
                .unwrap()
        );
        assert!(matches!(
            database.record_outbox_sent("claimed-old", "77", None, OUTBOX_NOW),
            Err(SendStateError::StaleClaim)
        ));
        database.recover_outbox_claims(OUTBOX_LEASE).unwrap();
        database
            .authorize_telegram_resume(new, &[row], OUTBOX_LEASE)
            .unwrap();
        database
            .claim_due_outbox_for_generation(
                new,
                bot,
                "claimed-new",
                OUTBOX_LEASE,
                "2026-10-02T00:00:00Z",
            )
            .unwrap()
            .unwrap();
        database
            .mark_request_started("claimed-new", OUTBOX_LEASE)
            .unwrap();
        assert!(matches!(
            database.record_outbox_sent("claimed-old", "77", None, OUTBOX_LEASE),
            Err(SendStateError::StaleClaim)
        ));
        database
            .record_outbox_sent("claimed-new", "78", None, OUTBOX_LEASE)
            .unwrap();
    }

    #[test]
    fn generation_fenced_claim_rejects_retired_worker() {
        let database = Database::open_in_memory().unwrap();
        let bot = "telegram-bot:42";
        let row = database
            .enqueue_outbox(outbox_entry(bot, "fenced", 0))
            .unwrap();
        let old = database
            .prepare_telegram_credential_generation(bot, "secret/fence-old", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(old, None)
            .unwrap();
        let new = database
            .prepare_telegram_credential_generation(bot, "secret/fence-new", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(new, Some(old))
            .unwrap();
        database
            .authorize_telegram_resume(new, &[row], OUTBOX_NOW)
            .unwrap();
        assert!(
            database
                .claim_due_outbox_for_generation(old, bot, "old-worker", OUTBOX_NOW, OUTBOX_LEASE)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            database
                .claim_due_outbox_for_generation(new, bot, "new-worker", OUTBOX_NOW, OUTBOX_LEASE)
                .unwrap()
                .unwrap()
                .id,
            row
        );
    }

    #[test]
    fn stable_bot_claim_requires_active_generation_row_grant() {
        let database = Database::open_in_memory().expect("database");
        let bot = "telegram-bot:42";
        let row = database
            .enqueue_outbox(outbox_entry(bot, "grant-claim", 0))
            .unwrap();
        assert!(
            database
                .claim_due_outbox(bot, "no-grant", OUTBOX_NOW, OUTBOX_LEASE)
                .unwrap()
                .is_none()
        );
        assert!(
            database
                .claim_outbox(
                    bot,
                    "grant-claim",
                    "manual-no-grant",
                    OUTBOX_NOW,
                    OUTBOX_LEASE
                )
                .unwrap()
                .is_none()
        );
        let generation = database
            .prepare_telegram_credential_generation(bot, "secret/claim", OUTBOX_NOW)
            .unwrap();
        database
            .activate_telegram_credential_generation(generation, None)
            .unwrap();
        database
            .authorize_telegram_resume(generation, &[row], OUTBOX_NOW)
            .unwrap();
        assert_eq!(
            database
                .claim_due_outbox(bot, "granted", OUTBOX_NOW, OUTBOX_LEASE)
                .unwrap()
                .unwrap()
                .id,
            row
        );
    }

    #[test]
    fn rotation_decision_preserves_policy_and_exact_candidates_across_reopen() {
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("rotation.sqlite3");
        let database = Database::open(&path).unwrap();
        let generation = database
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/decision", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(generation, None)
            .unwrap();
        assert!(
            database
                .record_telegram_rotation_decision(generation, "automatic", &[999999], "t0")
                .is_err()
        );
        assert!(
            database
                .telegram_rotation_decision(generation)
                .unwrap()
                .is_none()
        );
        database
            .record_telegram_rotation_decision(generation, "confirm", &[7, 8], "t1")
            .unwrap();
        database
            .record_telegram_rotation_decision(generation, "confirm", &[7, 8], "t2")
            .unwrap();
        assert!(
            database
                .record_telegram_rotation_decision(generation, "automatic", &[7, 8], "t3")
                .is_err()
        );
        assert!(
            database
                .record_telegram_rotation_decision(generation, "confirm", &[7, 8, 9], "t3")
                .is_err()
        );
        drop(database);
        let database = Database::open(&path).unwrap();
        assert_eq!(
            database.telegram_rotation_decision(generation).unwrap(),
            Some(("confirm".into(), vec![7, 8]))
        );
        drop(database);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn activation_and_rotation_decision_commit_together_or_not_at_all() {
        let database = Database::open_in_memory().unwrap();
        let old = database
            .prepare_telegram_credential_generation("telegram-bot:42", "old", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(old, None)
            .unwrap();
        let candidate = database
            .prepare_telegram_credential_generation("telegram-bot:42", "new", "t1")
            .unwrap();
        // Force decision insertion to fail after the attempted state changes.
        database
            .record_telegram_rotation_decision(candidate, "confirm", &[], "t1")
            .unwrap();
        assert!(
            database
                .activate_telegram_credential_with_resume(candidate, Some(old), "automatic", "t2")
                .is_err()
        );
        assert_eq!(
            database
                .active_telegram_credential_generation()
                .unwrap()
                .unwrap()
                .0,
            old
        );
        let next = database
            .prepare_telegram_credential_generation("telegram-bot:42", "next", "t3")
            .unwrap();
        assert!(
            database
                .activate_telegram_credential_with_resume(next, Some(old), "automatic", "t4")
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            database
                .active_telegram_credential_generation()
                .unwrap()
                .unwrap()
                .0,
            next
        );
        assert_eq!(
            database.telegram_rotation_decision(next).unwrap(),
            Some(("automatic".into(), vec![]))
        );
        assert!(
            database
                .activate_telegram_credential_with_resume(candidate, Some(old), "confirm", "t5")
                .is_err()
        );
    }

    #[test]
    fn initializes_schema_and_creates_idempotent_job() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = database
            .insert_tweet("1", "https://x.com/a/status/1", "post", "text", "now")
            .expect("tweet");
        assert!(
            database
                .create_archive_job("job-1", tweet_id, "now")
                .expect("job")
        );
        assert!(
            !database
                .create_archive_job("job-2", tweet_id, "now")
                .expect("idempotent job")
        );
    }
    #[test]
    fn exact_job_queries_are_not_limited_to_the_latest_one_hundred() {
        let database = Database::open_in_memory().expect("database");
        let first_tweet = database
            .insert_tweet("1", "https://x.com/a/status/1", "post", "", "t0")
            .expect("first tweet");
        assert!(
            database
                .create_archive_job("job-1", first_tweet, "t0")
                .expect("first job")
        );
        for index in 2..=105 {
            let tweet_id = index.to_string();
            let row_id = database
                .insert_tweet(
                    &tweet_id,
                    &format!("https://x.com/a/status/{tweet_id}"),
                    "post",
                    "",
                    &format!("t{index}"),
                )
                .expect("tweet");
            assert!(
                database
                    .create_archive_job(&format!("job-{index}"), row_id, &format!("t{index}"))
                    .expect("job")
            );
        }
        assert_eq!(database.list_recent_jobs(100).expect("recent").len(), 100);
        assert_eq!(
            database
                .job_summary("job-1")
                .expect("exact summary")
                .expect("old job")
                .tweet_id,
            "1"
        );
        assert_eq!(
            database
                .active_job_for_tweet("1")
                .expect("active by tweet")
                .expect("old active job")
                .job_id,
            "job-1"
        );
        assert_eq!(
            database
                .list_recovery_candidate_jobs()
                .expect("recovery")
                .len(),
            105
        );
        assert!(database.job_summary("missing").expect("missing").is_none());
        assert!(
            database
                .active_job_for_tweet("missing")
                .expect("missing active")
                .is_none()
        );
        let page = database.list_jobs_page(100, 20).expect("old page");
        assert_eq!(page.total, 105);
        assert_eq!(page.jobs.len(), 5);
        assert_eq!(page.offset, 100);
        assert_eq!(page.limit, 20);
        assert_eq!(page.jobs.last().expect("last page row").job_id, "job-1");
        assert!(database.job_download_metrics("job-1").expect("metrics").is_none());
    }

    #[test]
    fn download_metrics_are_persisted_and_stale_attempts_are_fenced() {
        let database = Database::open_in_memory().expect("database");
        let tweet = database.insert_tweet("50", "https://x.com/a/status/50", "post", "", "t0").expect("tweet");
        database.create_archive_job("metric-job", tweet, "t0").expect("job");
        let metrics = JobDownloadMetrics {
            job_id: "metric-job".into(), backend: Some("aria2".into()),
            download_started_at: Some("t1".into()), download_finished_at: Some("t2".into()),
            task_finished_at: None, downloaded_bytes: Some(4096), download_duration_ms: Some(2000), attempt_count: 2,
        };
        database.record_download_metrics(&metrics, "t2").expect("record metrics");
        let stale = JobDownloadMetrics { attempt_count: 1, downloaded_bytes: Some(1), ..metrics.clone() };
        database.record_download_metrics(&stale, "t3").expect("stale result ignored");
        let stored = database.job_download_metrics("metric-job").expect("read").expect("stored metrics");
        assert_eq!(stored.attempt_count, 2);
        assert_eq!(stored.downloaded_bytes, Some(4096));
        assert_eq!(stored.download_duration_ms, Some(2000));
        assert_eq!(stored.backend.as_deref(), Some("aria2"));
    }

    #[test]
    fn persists_user_name_history_and_stable_directory_name() {
        let database = Database::open_in_memory().expect("database");
        let user_id = database
            .upsert_user(
                "123",
                Some("alice"),
                Some("Alice / One"),
                "2026-09-08T00:00:00Z",
            )
            .expect("user");
        database
            .record_user_name(
                user_id,
                "alice_new",
                Some("Alice New"),
                "dom",
                "2026-09-08T00:01:00Z",
            )
            .expect("name history");
        let names = database.list_user_names(user_id).expect("names");
        assert_eq!(names.len(), 2);
        assert_eq!(names[0].username, "alice_new");
        assert_eq!(names[1].username, "alice");
        let user: UserSummary = database
            .connection
            .query_row(
                "SELECT x_user_id, stable_directory_name, first_seen_at, last_seen_at FROM users WHERE id = ?1",
                params![user_id],
                |row| {
                    Ok(UserSummary {
                        user_id: row.get(0)?,
                        stable_directory_name: row.get(1)?,
                        first_seen_at: row.get(2)?,
                        last_seen_at: row.get(3)?,
                    })
                },
            )
            .expect("user summary");
        assert_eq!(user.stable_directory_name, "@alice - Alice _ One [123]");
    }

    #[test]
    fn does_not_duplicate_name_history_when_names_are_unchanged() {
        let database = Database::open_in_memory().expect("database");
        let user_id = database
            .upsert_user("123", Some("alice"), Some("Alice"), "t1")
            .expect("user");
        database
            .upsert_user("123", Some("alice"), Some("Alice"), "t2")
            .expect("unchanged upsert");
        database
            .record_user_name(user_id, "alice_new", Some("Alice New"), "dom", "t3")
            .expect("name change");
        database
            .upsert_user("123", Some("alice_new"), Some("Alice New"), "t4")
            .expect("unchanged upsert after change");
        let names = database.list_user_names(user_id).expect("names");
        let usernames: Vec<&str> = names.iter().map(|name| name.username.as_str()).collect();
        // There is also the case where display_name changes but username stays the same — it must still be recorded.
        assert_eq!(usernames, ["alice_new", "alice"]);
    }

    #[test]
    fn persists_reply_and_quote_relationships() {
        let database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "quote", "", "now")
            .expect("tweet");
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: "123".into(),
            url: "https://x.com/a/status/123".into(),
            tweet_type: "quote".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: Some("alice".into()),
                display_name: Some("Alice".into()),
            },
            created_at: None,
            text: "quoting".into(),
            media: Vec::new(),
            archived_at: "2026-09-08T00:01:00Z".into(),
            reply_to: Some("111".into()),
            quoted_tweet: Some(xarchive_core::ArchiveQuotedTweet {
                tweet_id: "987".into(),
                url: "https://x.com/b/status/987".into(),
                username: Some("bob".into()),
                display_name: Some("Bob".into()),
                user_id: Some("9002".into()),
                text: Some("original".into()),
                created_at: None,
                tweet_type: Some("post".into()),
            }),
        };
        database
            .update_tweet_metadata(tweet_row_id, &metadata, "archive/123")
            .expect("metadata");
        let relationships = database
            .tweet_relationships("123")
            .expect("relationships")
            .expect("relationship row");
        assert_eq!(
            relationships,
            TweetRelationships {
                reply_to_tweet_id: Some("111".into()),
                quoted_tweet_id: Some("987".into()),
            }
        );
        assert_eq!(
            database.tweet_relationships("missing").expect("missing"),
            None
        );
    }

    #[test]
    fn upgrades_existing_database_with_relationship_columns() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("root");
        let database_path = root.join("archive.sqlite3");
        {
            let connection = rusqlite::Connection::open(&database_path).expect("raw database");
            connection
                .execute_batch(
                    "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
                )
                .expect("legacy version table");
            connection
                .execute_batch(include_str!("../migrations/0001_initial.sql"))
                .expect("legacy schema");
            connection
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-09-08T00:00:00Z')",
                    [],
                )
                .expect("legacy version");
            connection
                .execute(
                    "INSERT INTO tweets (tweet_id, canonical_url, tweet_type, text, created_at, updated_at) VALUES ('123', 'https://x.com/a/status/123', 'post', '', 'now', 'now')",
                    [],
                )
                .expect("legacy tweet");
        }
        let database = Database::open(&database_path).expect("upgraded database");
        let version: i64 = database
            .connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("version");
        // The legacy database is upgraded through the current migration set
        // (0001…0009, including the archive-intent recovery journal).
        assert_eq!(version, MIGRATIONS.len() as i64);
        let relationships = database
            .tweet_relationships("123")
            .expect("relationships")
            .expect("legacy row");
        assert_eq!(
            relationships,
            TweetRelationships {
                reply_to_tweet_id: None,
                quoted_tweet_id: None,
            }
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn attaches_tags_idempotently_and_lists_them_in_order() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = database
            .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
            .expect("tweet");
        let launch = database.upsert_tag("launch", "now").expect("tag");
        let person = database.upsert_tag("person", "now").expect("tag");
        assert_eq!(
            database.upsert_tag("launch", "later").expect("same tag"),
            launch
        );
        database.attach_tag(tweet_id, launch).expect("attach");
        database
            .attach_tag(tweet_id, launch)
            .expect("duplicate attach");
        database.attach_tag(tweet_id, person).expect("attach");
        assert_eq!(
            database.list_tweet_tags(tweet_id).expect("tags"),
            vec!["launch", "person"]
        );
    }

    #[test]
    fn rejects_invalid_user_and_tag_inputs() {
        let database = Database::open_in_memory().expect("database");
        assert!(matches!(
            database.upsert_user("", None, None, "now"),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.upsert_tag("\n", "now"),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.record_user_name(999, "", None, "dom", "now"),
            Err(StorageError::InvalidMetadata(_))
        ));
    }

    #[test]
    fn lists_recent_jobs_in_updated_order_and_preserves_errors() {
        let database = Database::open_in_memory().expect("database");
        let first_tweet = database
            .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
            .expect("first tweet");
        let second_tweet = database
            .insert_tweet("2", "https://x.com/b/status/2", "reply", "", "now")
            .expect("second tweet");
        database
            .create_archive_job("job-1", first_tweet, "2026-09-08T00:00:00Z")
            .expect("first job");
        database
            .create_archive_job("job-2", second_tweet, "2026-09-08T00:01:00Z")
            .expect("second job");
        let mut database = database;
        database
            .transition_job("job-2", JobState::Validating, "2026-09-08T00:02:00Z")
            .expect("transition");
        let jobs = database.list_recent_jobs(10).expect("jobs");
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].job_id, "job-2");
        assert_eq!(jobs[0].tweet_id, "2");
        assert_eq!(jobs[0].state, JobState::Validating);
        assert_eq!(jobs[1].job_id, "job-1");
        database
            .fail_job(
                "job-1",
                xarchive_core::JobState::Failed,
                "TRANSFER_FAILED",
                "http://u:p@cdn.example/file?token=secret&id=1",
                "now",
            )
            .expect("fail");
        let summary = database
            .job_summary("job-1")
            .expect("summary")
            .expect("job");
        assert_eq!(
            summary.last_error_message.as_deref(),
            Some("http://[REDACTED]@cdn.example/file?token=[REDACTED]&id=1")
        );
    }

    #[test]
    fn aggregates_job_metrics_across_all_persisted_states() {
        let database = Database::open_in_memory().expect("database");
        assert_eq!(
            database.job_metrics().expect("empty metrics"),
            JobMetrics {
                total: 0,
                active: 0,
                completed: 0,
                failed: 0
            }
        );
        let tweet_ids = (1..=4)
            .map(|id| {
                database
                    .insert_tweet(
                        &id.to_string(),
                        &format!("https://x.com/a/status/{id}"),
                        "post",
                        "",
                        "now",
                    )
                    .expect("tweet")
            })
            .collect::<Vec<_>>();
        let mut database = database;
        for (index, tweet_id) in tweet_ids.into_iter().enumerate() {
            database
                .create_archive_job(&format!("job-{index}"), tweet_id, "now")
                .expect("job");
        }
        database
            .transition_job("job-0", JobState::Validating, "later")
            .expect("active");
        database
            .transition_job("job-1", JobState::Validating, "later")
            .expect("active");
        database
            .transition_job("job-1", JobState::MetadataReady, "later")
            .expect("metadata");
        database
            .fail_job("job-2", JobState::Failed, "TEST", "failed", "later")
            .expect("failed");
        for next in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::TgMetadataSending,
            JobState::TgMetadataSent,
            JobState::Downloading,
            JobState::Downloaded,
            JobState::TgMediaUploading,
            JobState::Complete,
        ] {
            database
                .transition_job("job-3", next, "later")
                .expect("complete transition");
        }
        assert_eq!(
            database.job_metrics().expect("metrics"),
            JobMetrics {
                total: 4,
                active: 2,
                completed: 1,
                failed: 1
            }
        );
    }

    #[test]
    fn reopens_persistent_database_without_reapplying_schema() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("root");
        let database_path = root.join("archive.sqlite3");
        {
            let database = Database::open(&database_path).expect("first open");
            let tweet_id = database
                .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
                .expect("tweet");
            database
                .create_archive_job("job-1", tweet_id, "now")
                .expect("job");
        }
        let reopened = Database::open(&database_path).expect("reopen");
        assert_eq!(
            reopened.job_state("job-1").expect("state"),
            JobState::Queued
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn persists_transition_and_event() {
        let mut database = Database::open_in_memory().expect("database");
        let tweet_id = database
            .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job("job-1", tweet_id, "now")
            .expect("job");
        database
            .transition_job("job-1", JobState::Validating, "later")
            .expect("transition");
        assert_eq!(
            database.job_state("job-1").expect("state"),
            JobState::Validating
        );
        assert_eq!(database.count_job_events("job-1").expect("events"), 1);
    }

    #[test]
    fn protects_files_and_commits_staging() {
        let root = temp_root();
        let store = FileStore::new(&root).expect("store");
        let staging = store.staging_dir("job-1").expect("staging");
        fs::write(staging.join("01.txt"), b"hello").expect("file");
        assert_eq!(
            FileStore::sha256(staging.join("01.txt")).expect("hash"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert!(matches!(
            store.write_text("../escape.txt", "nope"),
            Err(StorageError::InvalidPath)
        ));
        let destination = store
            .commit_staging("job-1", Path::new("Users/alice/2026/09/1"))
            .expect("commit");
        assert!(destination.join("01.txt").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn staging_job_id_with_iso_timestamp_is_resolvable() {
        let root = temp_root();
        let store = FileStore::new(&root).expect("store");
        let job_id = "archive-2104110366951383099-2026-09-13T00:00:00Z";
        let staging = store.staging_dir(job_id).expect("staging");

        #[cfg(windows)]
        assert!(
            !staging
                .file_name()
                .expect("staging component")
                .to_string_lossy()
                .contains(':')
        );

        fs::write(staging.join("01.txt"), b"hello").expect("file");
        assert!(
            store
                .recovery_directory_exists(job_id, "_staging")
                .expect("staging recovery lookup")
        );
        let destination = store
            .commit_staging(job_id, Path::new("Users/alice/2026/09/13"))
            .expect("commit");
        assert_eq!(
            fs::read(destination.join("01.txt")).expect("read"),
            b"hello"
        );
        assert!(
            !store
                .recovery_directory_exists(job_id, "_staging")
                .expect("staging recovery lookup after commit")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn completes_local_archive_and_writes_portable_metadata() {
        let root = temp_root();
        let database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job("job-1", tweet_row_id, "now")
            .expect("job");
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        fs::write(staging.join("01.jpg"), b"image").expect("media");
        let media = xarchive_core::ArchiveMedia {
            index: 1,
            media_id: Some("media-1".into()),
            media_type: "photo".into(),
            file: "01.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 5,
            sha256: FileStore::sha256(staging.join("01.jpg")).expect("hash"),
        };
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: "1".into(),
            url: "https://x.com/a/status/1".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: Some("user-1".into()),
                username: Some("alice".into()),
                display_name: Some("Alice".into()),
            },
            created_at: Some("2026-09-08T00:00:00Z".into()),
            text: "hello".into(),
            media: vec![media],
            archived_at: "2026-09-08T00:01:00Z".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        let mut service = ArchiveService::new(database, files);
        let destination = service
            .complete_local_archive(
                "job-1",
                tweet_row_id,
                &metadata,
                Path::new("Users/alice/2026/09/1"),
            )
            .expect("archive");
        assert!(destination.join("tweet.json").is_file());
        assert!(destination.join("tweet.txt").is_file());
        let stable =
            xarchive_core::stable_user_directory_name(Some("alice"), Some("Alice"), "user-1");
        let profile_path = root.join("Users").join(&stable).join("profile.json");
        assert!(profile_path.is_file());
        let profile: UserProfileFile =
            serde_json::from_str(&fs::read_to_string(&profile_path).expect("profile content"))
                .expect("profile JSON");
        assert_eq!(profile.schema_version, 1);
        assert_eq!(profile.user_id, "user-1");
        assert_eq!(profile.stable_directory_name, stable);
        assert_eq!(profile.username.as_deref(), Some("alice"));
        assert_eq!(profile.display_name.as_deref(), Some("Alice"));
        assert_eq!(profile.updated_at, "2026-09-08T00:01:00Z");
        let user_row_id: i64 = service
            .database
            .connection
            .query_row(
                "SELECT id FROM users WHERE x_user_id = 'user-1'",
                [],
                |row| row.get(0),
            )
            .expect("user row");
        let linked: Option<i64> = service
            .database
            .connection
            .query_row(
                "SELECT user_id FROM tweets WHERE id = ?1",
                params![tweet_row_id],
                |row| row.get(0),
            )
            .expect("tweet user link");
        assert_eq!(linked, Some(user_row_id));
        assert_eq!(
            service.database.job_state("job-1").expect("state"),
            JobState::Downloaded
        );
        assert_eq!(
            service.database.count_job_events("job-1").expect("events"),
            4
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovers_staging_archive_from_persisted_metadata() {
        let root = temp_root();
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet(
                "recovery-1",
                "https://x.com/a/status/recovery-1",
                "post",
                "",
                "now",
            )
            .expect("tweet");
        database
            .create_archive_job("recovery-job", tweet_row_id, "now")
            .expect("job");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
            JobState::Downloaded,
        ] {
            database
                .transition_job("recovery-job", state, "now")
                .expect("state");
        }
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("recovery-job").expect("staging");
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: "recovery-1".into(),
            url: "https://x.com/a/status/recovery-1".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: Some("alice".into()),
                display_name: Some("Alice".into()),
            },
            created_at: None,
            text: "recovered".into(),
            media: Vec::new(),
            archived_at: "2026-09-13T00:00:00Z".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        fs::write(
            staging.join("tweet.json"),
            serde_json::to_vec_pretty(&metadata).expect("metadata JSON"),
        )
        .expect("metadata");
        fs::write(staging.join("tweet.txt"), "recovered\n").expect("text");

        let mut service = ArchiveService::new(database, files);
        let destination = service
            .recover_staging_archive(
                "recovery-job",
                tweet_row_id,
                Path::new("archives/recovery-1"),
            )
            .expect("recover");
        assert!(destination.join("tweet.json").is_file());
        assert_eq!(
            service.database.job_state("recovery-job").expect("state"),
            JobState::Downloaded
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn archive_intent_journal_recovers_staging_after_database_reopen() {
        check_archive_intent_recovery(false);
    }

    #[test]
    fn archive_intent_journal_recovers_renamed_archive_after_database_reopen() {
        check_archive_intent_recovery(true);
    }

    fn check_archive_intent_recovery(rename_before_restart: bool) {
        let root = temp_root();
        fs::create_dir_all(&root).expect("root");
        let database_path = root.join("recovery.sqlite3");
        let mut database = Database::open(&database_path).expect("database");
        let tweet_row_id = database
            .insert_tweet(
                "intent-1",
                "https://x.com/a/status/intent-1",
                "post",
                "",
                "t0",
            )
            .expect("tweet");
        database
            .create_archive_job("intent-job", tweet_row_id, "t0")
            .expect("job");
        let files = FileStore::new(&root).expect("files");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
        ] {
            database
                .transition_job("intent-job", state, "t0")
                .expect("advance job");
        }
        let staging = files.staging_dir("intent-job").expect("staging");
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: "intent-1".into(),
            url: "https://x.com/a/status/intent-1".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: None,
                display_name: None,
            },
            created_at: None,
            text: "recover me".into(),
            media: Vec::new(),
            archived_at: "t1".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        fs::write(
            staging.join("tweet.json"),
            serde_json::to_vec(&metadata).expect("json"),
        )
        .expect("metadata");
        fs::write(staging.join("tweet.txt"), "recover me").expect("text");
        let intent = TelegramArchiveIntentRecord {
            job_id: "intent-job".into(),
            tweet_row_id,
            archive_directory: "Tweets/intent-1".into(),
            state: "PREPARED".into(),
            metadata_text: Some("recover me".into()),
            media_json: "[]".into(),
            bot_identity: Some("bot-fingerprint".into()),
            chat_id: Some("-100".into()),
            message_thread_id: Some(17),
            config_revision: Some(3),
            plan_version: 1,
            created_at: "t1".into(),
            updated_at: "t1".into(),
        };
        database
            .prepare_telegram_archive_intent(&intent)
            .expect("prepare");

        if rename_before_restart {
            // Interruption after rename but before SQLite archive facts.
            files
                .commit_staging("intent-job", "Tweets/intent-1")
                .expect("rename");
        }
        drop(database);
        let database = Database::open(&database_path).expect("reopen database");
        assert_eq!(
            database
                .telegram_archive_intent(tweet_row_id)
                .expect("load after restart"),
            Some(intent.clone())
        );
        let mut service = ArchiveService::new(database, files);
        assert_eq!(
            service
                .recover_telegram_archive_intent(&intent, "t2")
                .expect("recover"),
            "ARCHIVED"
        );
        assert!(root.join("Tweets/intent-1/tweet.json").is_file());
        assert!(!root.join("_staging/intent-job").exists());
        service
            .database
            .prepare_telegram_archive_intent(&intent)
            .expect("repeat original intent after recovery");
        assert_eq!(
            service.database.job_state("intent-job").expect("job state"),
            JobState::Downloaded
        );
        let stored = service
            .database
            .telegram_archive_intent(tweet_row_id)
            .expect("load")
            .expect("record");
        assert_eq!(stored.state, "ARCHIVED");
        assert_eq!(stored.chat_id.as_deref(), Some("-100"));
        assert_eq!(stored.message_thread_id, Some(17));
        // An untrusted/mismatched final record must not become ARCHIVED.
        fs::write(root.join("Tweets/intent-1/tweet.json"), b"{}").expect("damage metadata");
        assert!(
            service
                .recover_telegram_archive_intent(&intent, "t3")
                .is_err()
        );
        assert_eq!(
            service
                .recover_telegram_archive_intent(&stored, "t3")
                .expect("repeat"),
            "ARCHIVED"
        );
        assert_eq!(
            service
                .database
                .list_recoverable_telegram_archive_intents()
                .expect("list")
                .len(),
            1
        );
        drop(service);
        let database = Database::open(&database_path).expect("second restart");
        assert_eq!(
            database
                .telegram_archive_intent(tweet_row_id)
                .expect("persisted progress")
                .expect("intent")
                .state,
            "ARCHIVED"
        );
        drop(database);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn archive_intent_journal_rejects_changed_facts_and_invalid_transitions() {
        let database = Database::open_in_memory().expect("database");
        let tweet = database
            .insert_tweet("intent-2", "url", "post", "", "t0")
            .expect("tweet");
        database
            .create_archive_job("intent-job-2", tweet, "t0")
            .expect("job");
        let intent = TelegramArchiveIntentRecord {
            job_id: "intent-job-2".into(),
            tweet_row_id: tweet,
            archive_directory: "Tweets/intent-2".into(),
            state: "PREPARED".into(),
            metadata_text: None,
            media_json: "[]".into(),
            bot_identity: Some("bot".into()),
            chat_id: Some("-100".into()),
            message_thread_id: None,
            config_revision: Some(1),
            plan_version: 1,
            created_at: "t0".into(),
            updated_at: "t0".into(),
        };
        database
            .prepare_telegram_archive_intent(&intent)
            .expect("prepare");
        let mut changed = intent.clone();
        changed.chat_id = Some("-200".into());
        assert!(database.prepare_telegram_archive_intent(&changed).is_err());
        assert!(
            database
                .transition_telegram_archive_intent(tweet, "PREPARED", "QUEUED", "t1")
                .is_err()
        );
        assert!(
            database
                .transition_telegram_archive_intent(tweet, "PREPARED", "SKIPPED", "t1")
                .is_err()
        );
        assert!(
            database
                .transition_telegram_archive_intent(tweet, "PREPARED", "ARCHIVED", "t1")
                .expect("transition")
        );
        assert!(
            !database
                .transition_telegram_archive_intent(tweet, "PREPARED", "ARCHIVED", "t2")
                .expect("repeat transition")
        );
        let mut entry = outbox_entry("bot", "journal-plan", 0);
        entry.tweet_id = Some(tweet);
        entry.chat_id = "-100".into();
        entry.target_scope = "-100:none".into();
        entry.config_version = 1;
        let mut wrong = entry.clone();
        wrong.bot_identity = "other-bot".into();
        wrong.idempotency_key = "wrong-plan".into();
        assert!(
            database
                .enqueue_archived_intent(tweet, vec![entry.clone(), wrong], "t3")
                .is_err()
        );
        assert!(
            database
                .list_outbox_for_tweet("bot", tweet)
                .expect("rolled back rows")
                .is_empty()
        );
        assert_eq!(
            database
                .telegram_archive_intent(tweet)
                .expect("journal")
                .expect("intent")
                .state,
            "ARCHIVED"
        );
        let ids = database
            .enqueue_archived_intent(tweet, vec![entry.clone()], "t4")
            .expect("materialize");
        assert_eq!(
            database
                .telegram_archive_intent(tweet)
                .expect("journal")
                .expect("intent")
                .state,
            "QUEUED"
        );
        assert_eq!(
            database
                .enqueue_archived_intent(tweet, vec![entry], "t5")
                .expect("replay"),
            ids
        );
    }

    #[test]
    fn outbox_batch_rolls_back_on_conflict_and_replays_idempotently() {
        let database = Database::open_in_memory().expect("database");
        let original = outbox_entry(BOT_A, "batch-existing", 1);
        database.enqueue_outbox(original.clone()).expect("existing");
        let fresh = outbox_entry(BOT_A, "batch-new", 0);
        let mut conflict = original.clone();
        conflict.request_fingerprint = "different".into();
        assert!(
            database
                .enqueue_outbox_batch(vec![fresh.clone(), conflict])
                .is_err()
        );
        assert_eq!(
            database
                .list_due_outbox(BOT_A, OUTBOX_NOW)
                .expect("due")
                .len(),
            1
        );
        let ids = database
            .enqueue_outbox_batch(vec![fresh.clone(), original.clone()])
            .expect("batch");
        assert_eq!(
            database
                .enqueue_outbox_batch(vec![fresh, original])
                .expect("replay"),
            ids
        );
        assert_eq!(
            database
                .list_due_outbox(BOT_A, OUTBOX_NOW)
                .expect("due")
                .len(),
            2
        );
    }

    #[test]
    fn converts_sidecar_files_using_actual_size_and_hash() {
        let root = temp_root();
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        fs::write(staging.join("01.jpg"), b"actual").expect("media");
        let raw = serde_json::json!({
            "tweet_id": "123",
            "tweet_url": "https://x.com/alice/status/123",
            "username": "alice",
            "display_name": "Alice",
            "text": "hello",
            "media": [{"id": "media-1", "type": "photo"}]
        });
        let sidecar_files = vec![xarchive_protocol::DownloadFile {
            relative_path: "01.jpg".into(),
            size_bytes: 999,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
        }];
        let metadata =
            build_archive_metadata("123", &raw, &sidecar_files, &staging, "now").expect("metadata");
        assert_eq!(metadata.media[0].size_bytes, 6);
        assert_eq!(metadata.media[0].media_id.as_deref(), Some("media-1"));
        assert_eq!(
            metadata.media[0].sha256,
            "e5c6fde86910ded72db5cc7afc32f850440d4ef7caa5dbb69f5bdc0d3e39cb3b"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_sidecar_path_escape() {
        let root = temp_root();
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        let raw = serde_json::json!({"tweet_id": "123"});
        let sidecar_files = vec![xarchive_protocol::DownloadFile {
            relative_path: "../escape.jpg".into(),
            size_bytes: 1,
            media_type: "photo".into(),
            mime_type: None,
        }];
        assert!(matches!(
            build_archive_metadata("123", &raw, &sidecar_files, &staging, "now"),
            Err(StorageError::InvalidPath)
        ));
        let _ = fs::remove_dir_all(root);
    }

    /// WQ-ENG-03: an intermediate directory symlink inside staging that points
    /// outside the staging root must not let metadata be built from an
    /// outside-root file (same gap a Windows junction exposes).
    #[cfg(unix)]
    #[test]
    fn rejects_sidecar_intermediate_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = temp_root();
        let outside = temp_root();
        fs::create_dir_all(&outside).expect("outside");
        fs::write(outside.join("01.jpg"), b"outside-media").expect("outside file");
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        symlink(&outside, staging.join("escape")).expect("intermediate symlink");

        let raw = serde_json::json!({"tweet_id": "123"});
        let sidecar_files = vec![xarchive_protocol::DownloadFile {
            relative_path: "escape/01.jpg".into(),
            size_bytes: 13,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
        }];
        let result = build_archive_metadata("123", &raw, &sidecar_files, &staging, "now");
        assert!(
            matches!(result, Err(StorageError::InvalidPath)),
            "intermediate symlink escape must be rejected, got {result:?}"
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn accepts_numeric_tweet_id_from_json() {
        let root = temp_root();
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        let raw = serde_json::json!({"tweet_id": 123});
        let metadata = build_archive_metadata("123", &raw, &[], &staging, "now").expect("metadata");
        assert_eq!(metadata.tweet_id, "123");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn builds_archive_metadata_with_author_and_relationship_fields() {
        let root = temp_root();
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        let raw = serde_json::json!({
            "tweet_id": "123",
            "url": "https://x.com/alice/status/123",
            "user_id": "9001",
            "username": "alice",
            "reply_to": "111",
            "quoted_tweet": {
                "tweet_id": "987",
                "url": "https://x.com/bob/status/987",
                "username": "bob",
                "user_id": "9002",
                "text": "original",
                "tweet_type": "post"
            }
        });
        let metadata = build_archive_metadata("123", &raw, &[], &staging, "now").expect("metadata");
        assert_eq!(metadata.author.user_id.as_deref(), Some("9001"));
        assert_eq!(metadata.reply_to.as_deref(), Some("111"));
        let quoted = metadata.quoted_tweet.expect("quoted tweet");
        assert_eq!(quoted.tweet_id, "987");
        assert_eq!(quoted.user_id.as_deref(), Some("9002"));
        assert_eq!(quoted.tweet_type.as_deref(), Some("post"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn completes_archive_directly_from_sidecar_result() {
        let root = temp_root();
        let database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job("job-1", tweet_row_id, "now")
            .expect("job");
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        fs::write(staging.join("01.jpg"), b"actual media").expect("media");
        let raw = serde_json::json!({
            "tweet_id": "123",
            "tweet_url": "https://x.com/alice/status/123",
            "username": "alice",
            "display_name": "Alice",
            "text": "archived from sidecar",
            "media": [{"id": "media-1", "type": "photo"}]
        });
        let sidecar_files = vec![xarchive_protocol::DownloadFile {
            relative_path: "01.jpg".into(),
            size_bytes: 1,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
        }];
        let mut service = ArchiveService::new(database, files);
        let destination = service
            .complete_sidecar_archive(SidecarArchiveRequest {
                job_id: "job-1",
                tweet_row_id,
                expected_tweet_id: "123",
                metadata: &raw,
                files: &sidecar_files,
                final_directory: Path::new("Users/alice/2026/09/123"),
                archived_at: "2026-09-08T00:01:00Z",
                telegram_intent: None,
            })
            .expect("sidecar archive");
        assert!(destination.join("01.jpg").is_file());
        assert!(destination.join("tweet.json").is_file());
        // The sidecar payload has no resolvable user_id, so no user row or
        // profile file must be created.
        let users_count: i64 = service
            .database
            .connection
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .expect("users count");
        assert_eq!(users_count, 0);
        assert!(!root.join("Users").join("profile.json").exists());
        assert_eq!(
            service.database.job_state("job-1").expect("state"),
            JobState::Downloaded
        );
        let media_size: i64 = service
            .database
            .connection
            .query_row(
                "SELECT size_bytes FROM media WHERE tweet_id = ?1",
                params![tweet_row_id],
                |row| row.get(0),
            )
            .expect("media size");
        assert_eq!(media_size, 12);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn telegram_send_state_round_trip() {
        let database = Database::open_in_memory().expect("database");
        assert!(
            database
                .find_sent("-100", "tweet-1:metadata")
                .expect("find")
                .is_none()
        );
        database
            .record_pending("-100", "tweet-1:metadata", "metadata", "now-1")
            .expect("pending");
        assert!(
            database
                .find_sent("-100", "tweet-1:metadata")
                .expect("find")
                .is_none()
        );
        database
            .record_sent(
                "-100",
                "tweet-1:metadata",
                "4242",
                Some("file-id-1"),
                "now-2",
            )
            .expect("sent");
        let sent = database
            .find_sent("-100", "tweet-1:metadata")
            .expect("find")
            .expect("sent record");
        assert_eq!(
            sent,
            SentSendRecord {
                chat_id: "-100".into(),
                idempotency_key: "tweet-1:metadata".into(),
                message_kind: "metadata".into(),
                telegram_message_id: "4242".into(),
                telegram_file_id: Some("file-id-1".into()),
            }
        );
        assert!(database.list_unsent().expect("unsent").is_empty());
    }

    #[test]
    fn telegram_send_state_counts_retries_and_lists_unsent() {
        let database = Database::open_in_memory().expect("database");
        database
            .record_pending("-100", "tweet-1:media:1", "media", "t1")
            .expect("pending");
        database
            .record_failed(
                "-100",
                "tweet-1:media:1",
                Some(429),
                "Too Many Requests",
                "t2",
            )
            .expect("failed");
        database
            .record_pending("-100", "tweet-1:media:1", "media", "t3")
            .expect("pending again");
        let unsent = database.list_unsent().expect("unsent");
        assert_eq!(unsent.len(), 1);
        assert_eq!(unsent[0].state, SendState::Pending);
        assert_eq!(unsent[0].attempt_count, 2);
        assert_eq!(unsent[0].message_kind, "media");
        database
            .record_failed("-100", "tweet-1:media:1", None, "HTTP status 503", "t4")
            .expect("failed again");
        database
            .record_pending("-200", "tweet-1:metadata", "metadata", "t5")
            .expect("other chat pending");
        let unsent = database.list_unsent().expect("unsent");
        assert_eq!(unsent.len(), 2);
        assert_eq!(unsent[0].chat_id, "-100");
        assert_eq!(unsent[0].attempt_count, 2);
        assert_eq!(unsent[1].chat_id, "-200");
        database
            .record_sent("-100", "tweet-1:media:1", "77", None, "t6")
            .expect("sent");
        let unsent = database.list_unsent().expect("unsent");
        assert_eq!(unsent.len(), 1);
        assert_eq!(unsent[0].chat_id, "-200");
    }

    #[test]
    fn telegram_send_state_sent_update_requires_existing_record() {
        let database = Database::open_in_memory().expect("database");
        assert_eq!(
            database
                .record_sent("-100", "missing", "1", None, "now")
                .expect_err("not found"),
            SendStateError::NotFound
        );
        assert_eq!(
            database
                .record_failed("-100", "missing", Some(400), "bad", "now")
                .expect_err("not found"),
            SendStateError::NotFound
        );
    }

    #[test]
    fn set_and_get_setting_round_trips() {
        let database = Database::open_in_memory().expect("database");
        assert!(database.get_setting("ui.theme").expect("get").is_none());
        database
            .set_setting("ui.theme", "\"dark\"", "2026-09-12T00:00:00Z")
            .expect("set");
        assert_eq!(
            database.get_setting("ui.theme").expect("get"),
            Some("\"dark\"".to_owned())
        );
        database
            .set_setting("ui.theme", "\"light\"", "2026-09-12T00:01:00Z")
            .expect("update");
        assert_eq!(
            database.get_setting("ui.theme").expect("get"),
            Some("\"light\"".to_owned())
        );
    }

    #[test]
    fn list_and_delete_settings() {
        let database = Database::open_in_memory().expect("database");
        assert!(database.list_settings().expect("list").is_empty());
        database.set_setting("ui.a", "\"1\"", "now").expect("set a");
        database
            .set_setting("download.b", "\"2\"", "now")
            .expect("set b");
        let settings = database.list_settings().expect("list");
        assert_eq!(settings.len(), 2);
        assert_eq!(settings[0].key, "download.b");
        assert_eq!(settings[1].key, "ui.a");
        assert!(database.delete_setting("ui.a").expect("delete"));
        assert!(!database.delete_setting("ui.a").expect("delete again"));
        assert_eq!(database.list_settings().expect("list").len(), 1);
    }

    #[test]
    fn rejects_invalid_setting_inputs() {
        let database = Database::open_in_memory().expect("database");
        assert!(matches!(
            database.set_setting("", "\"v\"", "now"),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.set_setting("telegram.bot_token", "\"secret\"", "now"),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.set_setting("ui.k", "not-json", "now"),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.get_setting(""),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.delete_setting("telegram.bot_token"),
            Err(StorageError::InvalidMetadata(_))
        ));
    }

    #[test]
    fn rejects_sidecar_metadata_for_a_different_tweet() {
        let root = temp_root();
        let files = FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        let raw = serde_json::json!({"tweet_id": "456"});
        assert!(matches!(
            build_archive_metadata("123", &raw, &[], &staging, "now"),
            Err(StorageError::InvalidMetadata(message))
                if message.contains("does not match")
        ));
        let _ = fs::remove_dir_all(root);
    }

    fn new_candidate(tweet_id: &str, state: &str) -> NewBatchCandidate {
        NewBatchCandidate {
            tweet_id: tweet_id.into(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            created_at: Some("2026-09-20T00:00:00Z".into()),
            tweet_type: "post".into(),
            is_repost: false,
            has_media: true,
            media_count: 2,
            user_id: Some("42".into()),
            username: Some("alice".into()),
            state: state.into(),
            skip_reason: None,
        }
    }

    fn insert_alice_batch(database: &Database, id: &str) {
        database
            .create_account_batch(id, "alice", "https://x.com/alice", None, None, "{}")
            .expect("batch");
    }

    #[test]
    fn creates_account_batch_with_pending_discovery_and_zero_counts() {
        let database = Database::open_in_memory().expect("database");
        insert_alice_batch(&database, "batch-1");
        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.state, "ACTIVE");
        assert_eq!(batch.discovery_state, "PENDING");
        assert_eq!(batch.counts, BatchCounts::default());
        assert_eq!(batch.username, "alice");
        assert_eq!(
            database.account_batch_state("batch-1").expect("state"),
            Some("ACTIVE".into())
        );
        assert!(
            database
                .account_batch("missing")
                .expect("missing")
                .is_none()
        );
        database
            .set_account_batch_discovery_state("batch-1", "RUNNING")
            .expect("discovery state");
        database
            .resolve_account_batch_identity("batch-1", Some("42"), Some("alice_real"))
            .expect("identity");
        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.discovery_state, "RUNNING");
        assert_eq!(batch.user_id.as_deref(), Some("42"));
        assert_eq!(batch.username, "alice_real");
        assert!(matches!(
            database.create_account_batch("", "alice", "https://x.com/alice", None, None, "{}"),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.set_account_batch_state("missing", "COMPLETED"),
            Err(StorageError::InvalidMetadata(_))
        ));
    }

    #[test]
    fn lists_account_batches_newest_first_and_honours_limit() {
        let database = Database::open_in_memory().expect("database");
        insert_alice_batch(&database, "batch-1");
        database
            .create_account_batch("batch-2", "bob", "https://x.com/bob", None, None, "{}")
            .expect("second batch");
        let batches = database.list_account_batches(10).expect("list");
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].id, "batch-2");
        assert_eq!(batches[1].id, "batch-1");
        assert_eq!(database.list_account_batches(1).expect("limited").len(), 1);
    }

    #[test]
    fn inserts_batch_candidates_once_and_filters_by_state() {
        let database = Database::open_in_memory().expect("database");
        insert_alice_batch(&database, "batch-1");
        let candidates = vec![
            new_candidate("100", "PENDING"),
            new_candidate("101", "PENDING"),
        ];
        assert_eq!(
            database
                .insert_batch_candidates("batch-1", &candidates)
                .expect("insert"),
            2
        );
        assert_eq!(
            database
                .insert_batch_candidates("batch-1", &candidates)
                .expect("reinsert"),
            0
        );
        let listed = database
            .list_batch_candidates("batch-1", None, 10)
            .expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].tweet_id, "100");
        assert_eq!(listed[0].media_count, 2);
        assert!(listed[0].has_media);
        assert!(!listed[0].is_repost);
        assert_eq!(listed[0].state, "PENDING");
        database
            .mark_batch_candidate(
                "batch-1",
                "100",
                "SUBMITTED",
                Some("job-100"),
                None,
                None,
                None,
            )
            .expect("mark submitted");
        assert_eq!(
            database.batch_candidate_counts("batch-1").expect("counts"),
            BatchCounts {
                total: 2,
                pending: 1,
                submitted: 1,
                ..BatchCounts::default()
            }
        );
        let submitted = database
            .list_batch_candidates("batch-1", Some("SUBMITTED"), 10)
            .expect("submitted");
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].job_id.as_deref(), Some("job-100"));
        assert!(matches!(
            database.mark_batch_candidate("batch-1", "999", "DONE", None, None, None, None),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            database.mark_batch_candidate("batch-1", "101", "BOGUS", None, None, None, None),
            Err(StorageError::InvalidMetadata(_))
        ));
    }

    #[test]
    fn retries_failures_and_cancels_only_undispatched_candidates() {
        let database = Database::open_in_memory().expect("database");
        insert_alice_batch(&database, "batch-1");
        let candidates = (0..3)
            .map(|index| new_candidate(&format!("10{index}"), "PENDING"))
            .collect::<Vec<_>>();
        database
            .insert_batch_candidates("batch-1", &candidates)
            .expect("insert");
        database
            .mark_batch_candidate(
                "batch-1",
                "100",
                "FAILED",
                None,
                Some("HTTP_429"),
                Some("rate limited"),
                None,
            )
            .expect("failed");
        database
            .mark_batch_candidate(
                "batch-1",
                "101",
                "SUBMITTED",
                Some("job-101"),
                None,
                None,
                None,
            )
            .expect("submitted");
        assert_eq!(
            database
                .retry_failed_batch_candidates("batch-1")
                .expect("retry"),
            1
        );
        assert!(
            database
                .list_batch_candidates("batch-1", Some("FAILED"), 10)
                .expect("failed list")
                .is_empty()
        );
        let retried = database
            .list_batch_candidates("batch-1", Some("PENDING"), 10)
            .expect("pending list");
        assert_eq!(retried.len(), 2);
        assert!(
            retried
                .iter()
                .all(|candidate| candidate.error_code.is_none() && candidate.job_id.is_none())
        );
        // cancel only stops future dispatch: submitted jobs keep running
        assert_eq!(
            database
                .cancel_pending_batch_candidates("batch-1")
                .expect("cancel"),
            2
        );
        let counts = database.batch_candidate_counts("batch-1").expect("counts");
        assert_eq!(counts.cancelled, 2);
        assert_eq!(counts.submitted, 1);
        assert_eq!(counts.pending, 0);
        assert_eq!(
            database
                .cancel_pending_batch_candidates("batch-1")
                .expect("cancel again"),
            0
        );
    }

    #[test]
    fn upgrades_v5_account_batches_without_losing_candidates_or_foreign_keys() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("root");
        let database_path = root.join("archive.sqlite3");
        {
            let connection = rusqlite::Connection::open(&database_path).expect("raw database");
            connection
                .execute_batch(include_str!("../migrations/0001_initial.sql"))
                .expect("base schema");
            connection
                .execute_batch(include_str!("../migrations/0005_account_batches.sql"))
                .expect("v5 batch schema");
            connection
                .execute(
                    "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
                    [],
                )
                .expect("versions");
            for version in 1..=5 {
                connection
                    .execute(
                        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, '2026-09-24T00:00:00Z')",
                        [version],
                    )
                    .expect("version row");
            }
            connection
                .execute(
                    "INSERT INTO archive_batches (id, username, profile_url, filters_json, state, discovery_state) VALUES ('batch-old', 'alice', 'https://x.com/alice', '{}', 'ACTIVE', 'RUNNING')",
                    [],
                )
                .expect("batch");
            connection
                .execute(
                    "INSERT INTO batch_candidates (batch_id, tweet_id, url, tweet_type, is_repost, has_media, media_count, state) VALUES ('batch-old', '123', 'https://x.com/alice/status/123', 'post', 0, 1, 1, 'PENDING')",
                    [],
                )
                .expect("candidate");
        }
        let mut database = Database::open(&database_path).expect("upgrade");
        let candidates = database
            .list_batch_candidates("batch-old", Some("PENDING"), 10)
            .expect("candidates");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].tweet_id, "123");
        let paused = database
            .pause_account_batch("batch-old")
            .expect("paused after migration");
        assert_eq!(paused.discovery_state, "PAUSED");
        let foreign_key_errors = database
            .connection
            .prepare("PRAGMA foreign_key_check")
            .expect("fk check")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("fk rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("fk values");
        assert!(foreign_key_errors.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn atomic_batch_control_keeps_discovery_state_consistent() {
        let mut database = Database::open_in_memory().expect("database");
        database
            .create_account_batch(
                "batch-pause",
                "alice",
                "https://x.com/alice",
                None,
                None,
                "{}",
            )
            .expect("pause batch");
        database
            .set_account_batch_discovery_state("batch-pause", "RUNNING")
            .expect("running");
        let paused = database
            .pause_account_batch("batch-pause")
            .expect("pause transaction");
        assert_eq!(paused.state, "PAUSED");
        assert_eq!(paused.discovery_state, "PAUSED");

        database
            .create_account_batch("batch-cancel", "bob", "https://x.com/bob", None, None, "{}")
            .expect("cancel batch");
        database
            .insert_batch_candidates(
                "batch-cancel",
                &[NewBatchCandidate {
                    tweet_id: "1".to_owned(),
                    url: "https://x.com/bob/status/1".to_owned(),
                    created_at: None,
                    tweet_type: "post".to_owned(),
                    is_repost: false,
                    has_media: true,
                    media_count: 1,
                    user_id: Some("2".to_owned()),
                    username: Some("bob".to_owned()),
                    state: "PENDING".to_owned(),
                    skip_reason: None,
                }],
            )
            .expect("candidate");
        database
            .set_account_batch_discovery_state("batch-cancel", "RUNNING")
            .expect("running");
        let cancelled = database
            .cancel_account_batch("batch-cancel")
            .expect("cancel transaction");
        assert_eq!(cancelled.state, "CANCELLED");
        assert_eq!(cancelled.discovery_state, "CANCELLED");
        assert_eq!(cancelled.counts.cancelled, 1);
        assert!(database.pause_account_batch("batch-cancel").is_err());
    }

    #[test]
    fn pauses_batches_with_bounded_retry_and_resumes_after_backoff() {
        let database = Database::open_in_memory().expect("database");
        insert_alice_batch(&database, "batch-1");
        database
            .create_account_batch("batch-2", "bob", "https://x.com/bob", None, None, "{}")
            .expect("second batch");
        database
            .set_account_batch_state("batch-1", "PAUSED")
            .expect("pause");
        database
            .set_account_batch_error("batch-1", Some("HTTP_429"), Some("rate limited"))
            .expect("error");
        database
            .set_account_batch_retry_at("batch-1", Some(2_000))
            .expect("retry at");
        // auth failures have no retry deadline and therefore wait for the user
        database
            .set_account_batch_state("batch-2", "PAUSED")
            .expect("pause auth batch");
        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.retry_at_ms, Some(2_000));
        assert_eq!(batch.last_error_code.as_deref(), Some("HTTP_429"));
        assert_eq!(
            database
                .resume_rate_limited_batches(1_999)
                .expect("too early"),
            0
        );
        assert_eq!(database.resume_rate_limited_batches(2_000).expect("due"), 1);
        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.state, "ACTIVE");
        assert_eq!(batch.retry_at_ms, None);
        assert_eq!(
            database.account_batch_state("batch-2").expect("state"),
            Some("PAUSED".into())
        );
        // an active transition clears any stale pause deadline
        database
            .set_account_batch_retry_at("batch-2", Some(9_000))
            .expect("stale deadline");
        database
            .set_account_batch_state("batch-2", "ACTIVE")
            .expect("manual resume");
        let batch = database
            .account_batch("batch-2")
            .expect("get")
            .expect("row");
        assert_eq!(batch.retry_at_ms, None);
    }

    #[test]
    fn merges_browser_placeholder_user_into_stable_identity() {
        let database = Database::open_in_memory().expect("database");
        let placeholder = database
            .upsert_user(
                "browser-123",
                Some("alice"),
                Some("Alice"),
                "2026-09-20T00:00:00Z",
            )
            .expect("placeholder");
        database
            .record_user_name(
                placeholder,
                "alice_old",
                Some("Alice"),
                "legacy",
                "2026-09-19T00:00:00Z",
            )
            .expect("legacy placeholder name");
        let stable = database
            .upsert_user("42", Some("alice"), Some("Alice"), "2026-09-21T00:00:00Z")
            .expect("stable");
        // a second identical observation from the stable path must be kept as-is
        // (only the placeholder fold de-duplicates)
        database
            .record_user_name(
                stable,
                "alice",
                Some("Alice"),
                "dom",
                "2026-09-21T00:00:00Z",
            )
            .expect("duplicate name");
        let tweet_row_id = database
            .insert_tweet_for_user(
                "123",
                "https://x.com/alice/status/123",
                "post",
                "",
                Some(placeholder),
                "2026-09-20T00:00:00Z",
            )
            .expect("tweet");
        database
            .merge_placeholder_user("browser-123", stable, "2026-09-22T00:00:00Z")
            .expect("merge");
        let linked: Option<i64> = database
            .connection
            .query_row(
                "SELECT user_id FROM tweets WHERE id = ?1",
                params![tweet_row_id],
                |row| row.get(0),
            )
            .expect("tweet user");
        assert_eq!(linked, Some(stable));
        let remaining: i64 = database
            .connection
            .query_row(
                "SELECT COUNT(*) FROM users WHERE x_user_id = 'browser-123'",
                [],
                |row| row.get(0),
            )
            .expect("placeholder removed");
        assert_eq!(remaining, 0);
        let names = database.list_user_names(stable).expect("names");
        let usernames: Vec<&str> = names.iter().map(|name| name.username.as_str()).collect();
        // stable keeps its own duplicate observations; the identical placeholder
        // observation is dropped while the unique one is folded in
        assert_eq!(usernames, ["alice", "alice", "alice_old"]);
        let user: UserSummary = database
            .connection
            .query_row(
                "SELECT x_user_id, stable_directory_name, first_seen_at, last_seen_at FROM users WHERE id = ?1",
                params![stable],
                |row| {
                    Ok(UserSummary {
                        user_id: row.get(0)?,
                        stable_directory_name: row.get(1)?,
                        first_seen_at: row.get(2)?,
                        last_seen_at: row.get(3)?,
                    })
                },
            )
            .expect("user summary");
        assert_eq!(user.first_seen_at, "2026-09-20T00:00:00Z");
        assert_eq!(user.last_seen_at, "2026-09-21T00:00:00Z");
        // unknown placeholders and self merges are no-ops
        database
            .merge_placeholder_user("browser-missing", stable, "2026-09-22T00:00:00Z")
            .expect("missing placeholder");
        database
            .merge_placeholder_user("42", stable, "2026-09-22T00:00:00Z")
            .expect("self merge");
        assert_eq!(database.list_user_names(stable).expect("names").len(), 3);
    }

    #[test]
    fn reports_archive_facts_for_completed_tweets_in_media_order() {
        let database = Database::open_in_memory().expect("database");
        let user_row_id = database
            .upsert_user("42", Some("alice"), Some("Alice"), "2026-09-20T00:00:00Z")
            .expect("user");
        let tweet_row_id = database
            .insert_tweet_for_user(
                "123",
                "https://x.com/alice/status/123",
                "post",
                "",
                Some(user_row_id),
                "2026-09-20T00:00:00Z",
            )
            .expect("tweet");
        assert!(
            database
                .tweet_archive_facts("123")
                .expect("pending")
                .is_none()
        );
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: "123".into(),
            url: "https://x.com/alice/status/123".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: Some("42".into()),
                username: Some("alice".into()),
                display_name: Some("Alice".into()),
            },
            created_at: None,
            text: "with media".into(),
            media: vec![
                xarchive_core::ArchiveMedia {
                    index: 1,
                    media_id: Some("m0".into()),
                    media_type: "video".into(),
                    file: "media/1.mp4".into(),
                    mime_type: Some("video/mp4".into()),
                    size_bytes: 10,
                    sha256: "a".into(),
                },
                xarchive_core::ArchiveMedia {
                    index: 2,
                    media_id: Some("m1".into()),
                    media_type: "photo".into(),
                    file: "media/2.jpg".into(),
                    mime_type: Some("image/jpeg".into()),
                    size_bytes: 20,
                    sha256: "b".into(),
                },
            ],
            archived_at: "2026-09-21T00:00:00Z".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        database
            .update_tweet_metadata(tweet_row_id, &metadata, "archive/123")
            .expect("metadata");
        // inserted out of order on purpose: facts must follow media_index
        for media in metadata.media.iter().rev() {
            database
                .insert_media(tweet_row_id, media, &metadata.archived_at)
                .expect("media");
        }
        let facts = database
            .tweet_archive_facts("123")
            .expect("facts")
            .expect("row");
        assert_eq!(facts.archive_directory, "archive/123");
        assert_eq!(
            facts.media_paths().collect::<Vec<_>>(),
            vec!["media/1.mp4", "media/2.jpg"]
        );
        // Media identity and size travel with the facts so completeness can be
        // decided without treating "the file exists" as success.
        assert_eq!(facts.media[0].media_index, 1);
        assert_eq!(facts.media[0].media_id.as_deref(), Some("m0"));
        assert_eq!(facts.media[0].media_type, "video");
        assert_eq!(facts.media[0].size_bytes, Some(10));
        assert_eq!(facts.media[0].sha256.as_deref(), Some("a"));
        assert_eq!(facts.media[1].media_index, 2);
        assert_eq!(facts.media[1].size_bytes, Some(20));
        assert!(
            database
                .tweet_archive_facts("456")
                .expect("missing")
                .is_none()
        );
    }

    // -------------------------------------------------------------------------
    // Telegram outbox and file_id cache (plan TG-04 / TG-05)
    // -------------------------------------------------------------------------

    const BOT_A: &str = "bot-identity-a";
    const BOT_B: &str = "bot-identity-b";
    const OUTBOX_NOW: &str = "2026-10-01T00:00:00Z";
    const OUTBOX_LEASE: &str = "2026-10-01T00:05:00Z";
    const OUTBOX_LATER: &str = "2026-10-01T01:00:00Z";
    const OUTBOX_EXPIRED: &str = "2026-10-01T00:06:00Z";

    fn outbox_entry(bot: &str, key: &str, plan_order: i64) -> NewOutboxEntry {
        NewOutboxEntry {
            bot_identity: bot.to_owned(),
            target_scope: "-1001:none".to_owned(),
            chat_id: "-1001".to_owned(),
            message_thread_id: None,
            idempotency_key: key.to_owned(),
            request_fingerprint: format!("fp-{key}"),
            message_kind: "media".to_owned(),
            config_version: 3,
            plan_version: 1,
            plan_order,
            tweet_id: None,
            media_reference: Some(format!("media/{key}.jpg")),
            content_sha256: Some(format!("sha-{key}")),
            payload_schema_version: 1,
            payload_json: format!("{{\"kind\":\"test\",\"key\":\"{key}\"}}"),
            created_at: OUTBOX_NOW.to_owned(),
        }
    }

    fn cache_key(bot: &str, kind: MediaKind, version: u32) -> FileCacheKey {
        FileCacheKey {
            bot_identity: bot.to_owned(),
            content_sha256: "sha-1".to_owned(),
            media_kind: kind,
            representation_version: version,
        }
    }

    #[test]
    fn telegram_outbox_migration_is_additive_and_isolates_legacy_rows() {
        let database = Database::open_in_memory().expect("database");
        // The 0002 contract keeps working unchanged.
        database
            .record_pending("-1001", "legacy", "metadata", OUTBOX_NOW)
            .expect("legacy pending");
        let new_tables: i64 = database
            .connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                   AND name IN ('telegram_outbox', 'telegram_file_cache')",
                [],
                |row| row.get(0),
            )
            .expect("new tables");
        assert_eq!(new_tables, 2);
        // A legacy row carries no bot identity, so the new bot sees nothing.
        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_NOW)
                .expect("due")
                .is_empty()
        );
        let id = database
            .enqueue_outbox(outbox_entry(BOT_A, "new", 1))
            .expect("enqueue");
        assert!(id > 0);
        let scope: String = database
            .connection
            .query_row(
                "SELECT target_scope FROM telegram_outbox WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("migrated target scope");
        let legacy_payload: Option<String> = database
            .connection
            .query_row(
                "SELECT payload_json FROM telegram_outbox WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("legacy payload is nullable");
        assert_eq!(scope, "-1001:none");
        assert_eq!(
            legacy_payload,
            Some(outbox_entry(BOT_A, "new", 1).payload_json)
        );
        let legacy_rows: i64 = database
            .connection
            .query_row("SELECT COUNT(*) FROM telegram_send_attempts", [], |row| {
                row.get(0)
            })
            .expect("legacy count");
        assert_eq!(legacy_rows, 1, "the legacy table is untouched");
    }

    #[test]
    fn outbox_enqueue_is_idempotent_and_plan_order_decides_the_order() {
        let database = Database::open_in_memory().expect("database");
        let first = database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        // Same bot, target and key with an identical payload is idempotent;
        // the stored plan position wins.
        let again = database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 9))
            .expect("repeat");
        assert_eq!(again, first);
        let second = database
            .enqueue_outbox(outbox_entry(BOT_A, "k2", 2))
            .expect("enqueue");
        assert_ne!(first, second);

        let due = database.list_due_outbox(BOT_A, OUTBOX_NOW).expect("due");
        assert_eq!(due.len(), 2);
        assert_eq!(due[0].idempotency_key, "k1");
        assert_eq!(due[1].idempotency_key, "k2");
        // The settings version an item was queued under travels with it.
        assert_eq!(due[0].config_version, 3);
        assert_eq!(due[0].state, OutboxState::Queued);
        assert!(!due[0].request_started);
    }

    #[test]
    fn outbox_identity_is_bot_and_topic_scoped_and_rejects_changed_payloads() {
        let database = Database::open_in_memory().expect("database");
        let first = database
            .enqueue_outbox(outbox_entry(BOT_A, "shared-key", 1))
            .expect("first enqueue");
        assert_eq!(
            database
                .enqueue_outbox(outbox_entry(BOT_A, "shared-key", 9))
                .expect("same intent and payload is idempotent"),
            first
        );

        let mut changed = outbox_entry(BOT_A, "shared-key", 1);
        changed.payload_json = "{\"kind\":\"changed\"}".to_owned();
        assert_eq!(
            database.enqueue_outbox(changed),
            Err(SendStateError::IdempotencyConflict)
        );

        let mut other_bot = outbox_entry(BOT_B, "shared-key", 1);
        assert_ne!(
            database
                .enqueue_outbox(other_bot.clone())
                .expect("same target and key for another bot is independent"),
            first
        );
        other_bot.bot_identity = BOT_A.to_owned();
        other_bot.target_scope = "-1001:topic-7".to_owned();
        other_bot.message_thread_id = Some(7);
        other_bot.chat_id = "-1001".to_owned();
        assert_ne!(
            database
                .enqueue_outbox(other_bot)
                .expect("different topic is a distinct send target"),
            first
        );
    }

    #[test]
    fn outbox_claim_is_exclusive_and_scoped_to_one_bot_identity() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        // Another bot never sees or claims this entry.
        assert!(
            database
                .claim_due_outbox(BOT_B, "claim-b", OUTBOX_NOW, OUTBOX_LEASE)
                .expect("claim")
                .is_none()
        );
        let claimed = database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        assert_eq!(claimed.state, OutboxState::InFlight);
        assert_eq!(claimed.attempt_count, 1);
        assert_eq!(claimed.claim_token.as_deref(), Some("claim-a"));
        assert_eq!(claimed.claim_expires_at.as_deref(), Some(OUTBOX_LEASE));
        // A racing second claim finds nothing while the lease is live.
        assert!(
            database
                .claim_due_outbox(BOT_A, "claim-a2", OUTBOX_NOW, OUTBOX_LEASE)
                .expect("claim")
                .is_none()
        );
        // A claimed entry is no longer "due".
        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_NOW)
                .expect("due")
                .is_empty()
        );
    }

    #[test]
    fn outbox_transitions_require_the_claim_that_owns_the_row() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        database
            .mark_request_started("claim-a", OUTBOX_NOW)
            .expect("started");

        // A foreign or expired claim can never write over the live one.
        for stale in [
            database.record_outbox_sent("claim-b", "1", None, OUTBOX_NOW),
            database.record_outbox_retry("claim-b", OUTBOX_LATER, None, "e", OUTBOX_NOW),
            database.record_outbox_unknown("claim-b", "lost", None, "e", OUTBOX_NOW),
            database.record_outbox_failed("claim-b", Some(403), "e", OUTBOX_NOW),
            database.mark_request_started("claim-b", OUTBOX_NOW),
        ] {
            assert_eq!(stale, Err(SendStateError::StaleClaim));
        }

        database
            .record_outbox_sent("claim-a", "77", Some(r#"[{"message_id":77}]"#), OUTBOX_NOW)
            .expect("sent");
        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_NOW)
                .expect("due")
                .is_empty()
        );
        // The claim is released by the transition, so a late write from the
        // finished worker is stale as well.
        assert_eq!(
            database.record_outbox_failed("claim-a", None, "late", OUTBOX_LATER),
            Err(SendStateError::StaleClaim)
        );
    }

    #[test]
    fn outbox_retry_waits_for_the_scheduled_time() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        database
            .record_outbox_retry(
                "claim-a",
                OUTBOX_LATER,
                Some(429),
                "Too Many Requests",
                OUTBOX_NOW,
            )
            .expect("retry");

        // Not due yet: the persisted retry_after delay is honoured.
        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_NOW)
                .expect("due")
                .is_empty()
        );
        assert!(
            database
                .claim_due_outbox(BOT_A, "claim-early", OUTBOX_NOW, OUTBOX_LEASE)
                .expect("claim")
                .is_none()
        );
        let due = database
            .list_due_outbox(BOT_A, OUTBOX_LATER)
            .expect("due later");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].state, OutboxState::RetryWait);
        assert_eq!(due[0].last_error_code, Some(429));
        assert_eq!(due[0].next_retry_at.as_deref(), Some(OUTBOX_LATER));

        let again = database
            .claim_due_outbox(BOT_A, "claim-b", OUTBOX_LATER, OUTBOX_EXPIRED)
            .expect("claim")
            .expect("claimed");
        assert_eq!(again.state, OutboxState::InFlight);
        assert_eq!(again.attempt_count, 2, "each attempt is counted");
    }

    #[test]
    fn outbox_unknown_is_never_picked_up_automatically() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        database
            .record_outbox_unknown("claim-a", "response_lost", None, "lost", OUTBOX_NOW)
            .expect("unknown");

        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_LATER)
                .expect("due")
                .is_empty()
        );
        assert!(
            database
                .claim_due_outbox(BOT_A, "claim-b", OUTBOX_LATER, OUTBOX_EXPIRED)
                .expect("claim")
                .is_none()
        );
        // Only a deliberate re-send after review may claim it again.
        let reviewed = database
            .claim_outbox(BOT_A, "k1", "claim-manual", OUTBOX_LATER, OUTBOX_EXPIRED)
            .expect("claim")
            .expect("claimed");
        assert_eq!(reviewed.state, OutboxState::InFlight);
        assert_eq!(reviewed.unknown_reason.as_deref(), Some("response_lost"));
    }

    #[test]
    fn outbox_plan_order_holds_back_the_rest_of_an_archive_until_its_predecessor_is_sent() {
        let database = Database::open_in_memory().expect("database");
        let tweet_row = database
            .insert_tweet("7", "https://x.com/i/status/7", "tweet", "hi", OUTBOX_NOW)
            .expect("tweet row");
        let mut first = outbox_entry(BOT_A, "tweet-7:metadata", 0);
        first.tweet_id = Some(tweet_row);
        database.enqueue_outbox(first).expect("enqueue first");
        let mut second = outbox_entry(BOT_A, "tweet-7:media:1", 1);
        second.tweet_id = Some(tweet_row);
        database.enqueue_outbox(second).expect("enqueue second");

        // Only the leading unit of the archive is claimable while it runs.
        let claimed = database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        assert_eq!(claimed.idempotency_key, "tweet-7:metadata");
        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_NOW)
                .expect("due")
                .is_empty(),
            "a claimed predecessor holds the rest of the archive back"
        );
        assert!(
            database
                .claim_due_outbox(BOT_A, "claim-b", OUTBOX_NOW, OUTBOX_LEASE)
                .expect("claim")
                .is_none()
        );

        database
            .record_outbox_sent("claim-a", "msg-1", None, OUTBOX_NOW)
            .expect("sent");

        // The predecessor reached SENT, so the next planned unit is free.
        let next = database
            .claim_due_outbox(BOT_A, "claim-b", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        assert_eq!(next.idempotency_key, "tweet-7:media:1");
        assert_eq!(next.plan_order, 1);
    }

    #[test]
    fn outbox_unit_that_cannot_be_sent_pauses_the_units_that_follow_it() {
        let database = Database::open_in_memory().expect("database");
        let tweet_row = database
            .insert_tweet("9", "https://x.com/i/status/9", "tweet", "hi", OUTBOX_NOW)
            .expect("tweet row");
        let mut first = outbox_entry(BOT_A, "tweet-9:media:1", 0);
        first.tweet_id = Some(tweet_row);
        database.enqueue_outbox(first).expect("enqueue first");
        let mut second = outbox_entry(BOT_A, "tweet-9:media:2", 1);
        second.tweet_id = Some(tweet_row);
        database.enqueue_outbox(second).expect("enqueue second");

        database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        database
            .record_outbox_failed("claim-a", Some(403), "forbidden", OUTBOX_NOW)
            .expect("fail");

        // A permanent failure is not SENT: the remaining units stay paused
        // instead of being sent in a different order than planned.
        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_LATER)
                .expect("due")
                .is_empty()
        );
        assert!(
            database
                .claim_due_outbox(BOT_A, "claim-b", OUTBOX_LATER, OUTBOX_LEASE)
                .expect("claim")
                .is_none()
        );
    }

    #[test]
    fn outbox_rows_without_a_tweet_are_independent_and_still_claim_in_plan_order() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k2", 1))
            .expect("enqueue second");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 0))
            .expect("enqueue first");

        // No archive to sequence, so both are due and plan_order wins.
        let due = database.list_due_outbox(BOT_A, OUTBOX_NOW).expect("due");
        assert_eq!(due.len(), 2);
        assert_eq!(due[0].idempotency_key, "k1");
        assert_eq!(due[1].idempotency_key, "k2");
    }

    #[test]
    fn outbox_lease_renewal_is_owner_only_and_moves_the_recovery_deadline() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");

        // A token that holds nothing cannot extend anything.
        assert!(
            !database
                .renew_outbox_claim("claim-a", OUTBOX_LATER, OUTBOX_NOW)
                .expect("renew")
        );

        database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        // Not the owner...
        assert!(
            !database
                .renew_outbox_claim("claim-b", OUTBOX_LATER, OUTBOX_NOW)
                .expect("renew")
        );
        // ...the owner can, while its lease is still live.
        assert!(
            !database
                .renew_outbox_claim("claim-a", OUTBOX_NOW, OUTBOX_NOW)
                .expect("reject past deadline")
        );
        // A delayed heartbeat cannot undo a newer extension.
        assert!(
            database
                .renew_outbox_claim("claim-a", OUTBOX_LEASE, OUTBOX_NOW)
                .expect("same deadline remains valid")
        );
        assert!(
            database
                .renew_outbox_claim("claim-a", OUTBOX_LATER, OUTBOX_NOW)
                .expect("renew")
        );

        // Without the renewal the lease would have expired at 00:05; the
        assert!(
            !database
                .renew_outbox_claim("claim-a", OUTBOX_LEASE, OUTBOX_NOW)
                .expect("reject shortened deadline")
        );
        // renewal pushed recovery out to OUTBOX_LATER.
        assert_eq!(
            database
                .recover_outbox_claims(OUTBOX_EXPIRED)
                .expect("recover"),
            0
        );
        assert_eq!(
            database
                .recover_outbox_claims(OUTBOX_LATER)
                .expect("recover"),
            1
        );
    }

    #[test]
    fn outbox_expired_lease_cannot_be_resurrected_by_its_former_owner() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");

        // The lease already elapsed, so the claim belongs to recovery now.
        assert!(matches!(
            database.mark_request_started("claim-a", OUTBOX_EXPIRED),
            Err(SendStateError::StaleClaim)
        ));
        assert!(matches!(
            database.record_outbox_sent("claim-a", "123", None, OUTBOX_EXPIRED),
            Err(SendStateError::StaleClaim)
        ));
        assert!(matches!(
            database.record_outbox_retry("claim-a", OUTBOX_LATER, None, "retry", OUTBOX_EXPIRED),
            Err(SendStateError::StaleClaim)
        ));
        assert!(matches!(
            database.record_outbox_unknown("claim-a", "unknown", None, "error", OUTBOX_EXPIRED),
            Err(SendStateError::StaleClaim)
        ));
        assert!(matches!(
            database.record_outbox_failed("claim-a", None, "error", OUTBOX_EXPIRED),
            Err(SendStateError::StaleClaim)
        ));
        assert!(matches!(
            database.record_outbox_cancelled("claim-a", OUTBOX_EXPIRED),
            Err(SendStateError::StaleClaim)
        ));
        assert!(
            !database
                .renew_outbox_claim("claim-a", OUTBOX_LATER, OUTBOX_EXPIRED)
                .expect("renew")
        );
        assert_eq!(
            database
                .recover_outbox_claims(OUTBOX_EXPIRED)
                .expect("recover"),
            1
        );
    }

    #[test]
    fn outbox_cancel_only_applies_before_the_send() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k2", 2))
            .expect("enqueue");
        assert!(
            database
                .cancel_outbox(BOT_A, "k1", OUTBOX_NOW)
                .expect("cancel")
        );
        // Idempotent: an already cancelled entry cannot be cancelled twice.
        assert!(
            !database
                .cancel_outbox(BOT_A, "k1", OUTBOX_NOW)
                .expect("repeat")
        );
        assert!(
            !database
                .cancel_outbox(BOT_A, "missing", OUTBOX_NOW)
                .expect("missing")
        );

        // A claimed entry may already be on the wire, so cancelling it must
        // fail and go through the UNKNOWN path instead.
        let claimed = database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        assert_eq!(claimed.idempotency_key, "k2");
        assert!(
            !database
                .cancel_outbox(BOT_A, "k2", OUTBOX_NOW)
                .expect("in flight")
        );
        // Another bot cannot cancel it either.
        assert!(
            !database
                .cancel_outbox(BOT_B, "k2", OUTBOX_NOW)
                .expect("other bot")
        );
    }

    #[test]
    fn outbox_lease_recovery_separates_pre_send_from_started_requests() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "pre", 1))
            .expect("enqueue");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "started", 2))
            .expect("enqueue");
        // The first claim crashes before the network attempt, the second after.
        database
            .claim_due_outbox(BOT_A, "claim-pre", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        database
            .claim_due_outbox(BOT_A, "claim-started", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        database
            .mark_request_started("claim-started", OUTBOX_NOW)
            .expect("started");

        // Nothing is resolved while the lease is still live.
        assert_eq!(
            database.recover_outbox_claims(OUTBOX_NOW).expect("recover"),
            0
        );
        assert_eq!(
            database
                .recover_outbox_claims(OUTBOX_EXPIRED)
                .expect("recover"),
            2
        );

        // The pre-send crash returns to RETRY_WAIT and is due again.
        let due = database
            .list_due_outbox(BOT_A, OUTBOX_EXPIRED)
            .expect("due");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].idempotency_key, "pre");
        assert_eq!(due[0].state, OutboxState::RetryWait);
        assert_eq!(due[0].attempt_count, 1);

        // The started request becomes UNKNOWN and keeps its reason; it is
        // never re-sent automatically.
        let reclaimed = database
            .claim_due_outbox(BOT_A, "claim-x", OUTBOX_EXPIRED, OUTBOX_EXPIRED)
            .expect("claim")
            .expect("claimed");
        assert_eq!(reclaimed.idempotency_key, "pre");
        assert_eq!(reclaimed.state, OutboxState::InFlight);
        // Nothing else is due afterwards: the started request stayed UNKNOWN.
        assert!(
            database
                .claim_due_outbox(BOT_A, "claim-y", OUTBOX_EXPIRED, OUTBOX_EXPIRED)
                .expect("claim")
                .is_none()
        );
        let reviewed = database
            .claim_outbox(
                BOT_A,
                "started",
                "claim-manual",
                OUTBOX_EXPIRED,
                OUTBOX_EXPIRED,
            )
            .expect("claim")
            .expect("claimed");
        assert_eq!(
            reviewed.unknown_reason.as_deref(),
            Some(UNKNOWN_REASON_LEASE_EXPIRED)
        );
    }

    #[test]
    fn file_id_cache_is_scoped_per_bot_kind_and_representation() {
        let database = Database::open_in_memory().expect("database");
        let cached = CachedFileId {
            file_id: "file-1".to_owned(),
            file_unique_id: "uniq-1".to_owned(),
            file_size: 42,
            confirmed_at: OUTBOX_NOW.to_owned(),
        };
        let key = cache_key(BOT_A, MediaKind::Photo, TELEGRAM_FILE_CACHE_VERSION);
        assert!(
            database.lookup_file_id(&key).expect("miss").is_none(),
            "an empty cache misses"
        );
        database.store_file_id(&key, &cached).expect("store");
        assert_eq!(
            database.lookup_file_id(&key).expect("hit").expect("entry"),
            cached
        );

        // Bot identity, media kind and representation version all scope an
        // entry: a token rotation or a kind change never reuses another id.
        assert!(
            database
                .lookup_file_id(&cache_key(
                    BOT_B,
                    MediaKind::Photo,
                    TELEGRAM_FILE_CACHE_VERSION
                ))
                .expect("other bot")
                .is_none()
        );
        assert!(
            database
                .lookup_file_id(&cache_key(
                    BOT_A,
                    MediaKind::Video,
                    TELEGRAM_FILE_CACHE_VERSION
                ))
                .expect("other kind")
                .is_none()
        );
        assert!(
            database
                .lookup_file_id(&cache_key(
                    BOT_A,
                    MediaKind::Photo,
                    TELEGRAM_FILE_CACHE_VERSION + 1
                ))
                .expect("other version")
                .is_none()
        );
    }

    #[test]
    fn file_id_cache_drop_and_refresh_touch_only_the_named_bot() {
        let database = Database::open_in_memory().expect("database");
        let key_a = cache_key(BOT_A, MediaKind::Photo, TELEGRAM_FILE_CACHE_VERSION);
        let key_b = cache_key(BOT_B, MediaKind::Photo, TELEGRAM_FILE_CACHE_VERSION);
        let cached = CachedFileId {
            file_id: "file-a".to_owned(),
            file_unique_id: "uniq-a".to_owned(),
            file_size: 1,
            confirmed_at: OUTBOX_NOW.to_owned(),
        };
        database.store_file_id(&key_a, &cached).expect("store a");
        database.store_file_id(&key_b, &cached).expect("store b");

        assert_eq!(database.drop_file_cache_for_bot(BOT_A).expect("drop"), 1);
        assert!(
            database.lookup_file_id(&key_a).expect("a").is_none(),
            "the named bot is purged"
        );
        assert!(
            database.lookup_file_id(&key_b).expect("b").is_some(),
            "another bot keeps its entries"
        );

        // A newer confirmed result replaces the older one for the same key.
        let refreshed = CachedFileId {
            file_id: "file-b2".to_owned(),
            file_unique_id: "uniq-b2".to_owned(),
            file_size: 2,
            confirmed_at: OUTBOX_LATER.to_owned(),
        };
        database.store_file_id(&key_b, &refreshed).expect("refresh");
        assert_eq!(
            database
                .lookup_file_id(&key_b)
                .expect("hit")
                .expect("entry"),
            refreshed
        );
    }

    #[test]
    fn a_claimed_entry_can_be_cancelled_before_its_request_starts() {
        let database = Database::open_in_memory().expect("database");
        database
            .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
            .expect("enqueue");
        database
            .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .expect("claimed");
        // Nothing was sent, so the driver can record a clean cancellation.
        database
            .record_outbox_cancelled("claim-a", OUTBOX_NOW)
            .expect("cancelled");
        assert!(
            database
                .list_due_outbox(BOT_A, OUTBOX_LATER)
                .expect("due")
                .is_empty(),
            "a cancelled entry never returns to the queue"
        );
        // A claim token that no longer owns the row cannot write again.
        assert_eq!(
            database.record_outbox_cancelled("claim-a", OUTBOX_LATER),
            Err(SendStateError::StaleClaim)
        );
    }

    #[test]
    fn a_tweets_outbox_rows_are_read_in_plan_order_for_one_bot() {
        let database = Database::open_in_memory().expect("database");
        let tweet = database
            .insert_tweet(
                "1961",
                "https://x.com/a/status/1961",
                "post",
                "text",
                OUTBOX_NOW,
            )
            .expect("tweet");
        for (key, order) in [("first", 2), ("second", 1)] {
            let mut entry = outbox_entry(BOT_A, key, order);
            entry.tweet_id = Some(tweet);
            database.enqueue_outbox(entry).expect("enqueue");
        }
        // Another bot's row for the same Tweet stays invisible.
        let mut other_bot = outbox_entry(BOT_B, "other", 0);
        other_bot.tweet_id = Some(tweet);
        database.enqueue_outbox(other_bot).expect("enqueue");

        let rows = database
            .list_outbox_for_tweet(BOT_A, tweet)
            .expect("rows for the task projection");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].idempotency_key, "second", "plan order decides");
        assert_eq!(rows[1].idempotency_key, "first");
        assert_eq!(rows[0].state, OutboxState::Queued);
        assert!(
            database
                .list_outbox_for_tweet(BOT_A, tweet + 1)
                .expect("other tweet")
                .is_empty()
        );
    }
}
