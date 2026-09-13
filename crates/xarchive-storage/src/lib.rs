//! SQLite persistence and local file storage for the Desktop application.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use xarchive_core::{ArchiveMetadata, ArchiveQuotedTweet, JobEvent, JobState};
use xarchive_telegram::{
    PendingSendRecord, SendState, SendStateError, SendStateStore, SentSendRecord,
};

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_telegram_send_state.sql"),
    include_str!("../migrations/0003_quote_reply_relationships.sql"),
];

const MAX_SETTING_BYTES: usize = 16 * 1024;

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Io(io::Error),
    Json(serde_json::Error),
    InvalidPath,
    InvalidState(String),
    InvalidTransition(xarchive_core::JobStateError),
    InvalidMetadata(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "SQLite error: {error}"),
            Self::Io(error) => write!(formatter, "file error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::InvalidPath => formatter.write_str("path is outside the archive root"),
            Self::InvalidState(value) => write!(formatter, "invalid persisted job state: {value}"),
            Self::InvalidTransition(error) => write!(formatter, "{error}"),
            Self::InvalidMetadata(message) => {
                write!(formatter, "invalid archive metadata: {message}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<io::Error> for StorageError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub struct Database {
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobSummary {
    pub job_id: String,
    pub tweet_id: String,
    pub tweet_type: String,
    pub state: JobState,
    pub created_at: String,
    pub updated_at: String,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserSummary {
    pub user_id: String,
    pub stable_directory_name: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserNameSummary {
    pub username: String,
    pub display_name: Option<String>,
    pub source: String,
    pub observed_at: String,
}

/// Direct reply/quote relationship columns for one archived tweet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweetRelationships {
    pub reply_to_tweet_id: Option<String>,
    pub quoted_tweet_id: Option<String>,
}

/// One `settings_meta` row exposed to callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingEntry {
    pub key: String,
    pub value_json: String,
    pub updated_at: String,
}

/// Current user profile state read from the database for profile files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserProfileSnapshot {
    pub user_id: String,
    pub stable_directory_name: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<UserNameSummary>,
}

/// Portable `profile.json` payload stored in the stable user directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfileFile {
    pub schema_version: u32,
    pub user_id: String,
    pub stable_directory_name: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<UserNameSummary>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
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

    pub fn upsert_user(
        &self,
        user_id: &str,
        username: Option<&str>,
        display_name: Option<&str>,
        now: &str,
    ) -> Result<i64, StorageError> {
        if user_id.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "user_id must not be empty".into(),
            ));
        }
        let directory = xarchive_core::stable_user_directory_name(username, display_name, user_id);
        self.connection.execute(
            "INSERT INTO users (x_user_id, stable_directory_name, first_seen_at, last_seen_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?3, ?3, ?3) ON CONFLICT(x_user_id) DO UPDATE SET stable_directory_name = excluded.stable_directory_name, last_seen_at = excluded.last_seen_at, updated_at = excluded.updated_at",
            params![user_id, directory, now],
        )?;
        let row_id = self.connection.query_row(
            "SELECT id FROM users WHERE x_user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        if let Some(username) = username.filter(|value| !value.trim().is_empty()) {
            let unchanged = self.latest_user_name(row_id)?.is_some_and(|latest| {
                latest.username == username && latest.display_name.as_deref() == display_name
            });
            if !unchanged {
                self.record_user_name(row_id, username, display_name, "unknown", now)?;
            }
        }
        Ok(row_id)
    }

    /// Most recent username observation for a user, if any.
    fn latest_user_name(&self, user_row_id: i64) -> Result<Option<UserNameSummary>, StorageError> {
        Ok(self.list_user_names(user_row_id)?.into_iter().next())
    }

    pub fn record_user_name(
        &self,
        user_row_id: i64,
        username: &str,
        display_name: Option<&str>,
        source: &str,
        observed_at: &str,
    ) -> Result<(), StorageError> {
        if username.trim().is_empty() || source.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "username and source must not be empty".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO user_names (user_id, username, display_name, source, observed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user_row_id, username, display_name, source, observed_at],
        )?;

        Ok(())
    }

    pub fn list_user_names(&self, user_row_id: i64) -> Result<Vec<UserNameSummary>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT username, display_name, source, observed_at FROM user_names WHERE user_id = ?1 ORDER BY observed_at DESC, id DESC",
        )?;
        let rows = statement.query_map(params![user_row_id], |row| {
            Ok(UserNameSummary {
                username: row.get(0)?,
                display_name: row.get(1)?,
                source: row.get(2)?,
                observed_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    /// Latest user profile snapshot derived from `users` and `user_names`.
    ///
    /// The newest name observation (by `observed_at`) provides the current
    /// username/display name; the full history is embedded in profile files.
    pub fn user_profile(
        &self,
        x_user_id: &str,
    ) -> Result<Option<UserProfileSnapshot>, StorageError> {
        let row = self
            .connection
            .query_row(
                "SELECT id, x_user_id, stable_directory_name, first_seen_at, last_seen_at FROM users WHERE x_user_id = ?1",
                params![x_user_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((row_id, user_id, stable_directory_name, first_seen_at, last_seen_at)) = row
        else {
            return Ok(None);
        };
        let names = self.list_user_names(row_id)?;
        let (username, display_name) = match names.first() {
            Some(latest) => (Some(latest.username.clone()), latest.display_name.clone()),
            None => (None, None),
        };
        Ok(Some(UserProfileSnapshot {
            user_id,
            stable_directory_name,
            username,
            display_name,
            first_seen_at,
            last_seen_at,
            names,
        }))
    }

    /// Read one setting value by key, or `None` if the key does not exist.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let key = key.trim();
        if !is_allowed_setting_key(key) {
            return Err(StorageError::InvalidMetadata(
                "setting key must not be empty".into(),
            ));
        }
        Ok(self
            .connection
            .query_row(
                "SELECT value_json FROM settings_meta WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?)
    }

    /// Insert or update a setting value.
    pub fn set_setting(&self, key: &str, value_json: &str, now: &str) -> Result<(), StorageError> {
        let key = key.trim();
        if key.is_empty()
            || key.len() > 256
            || !(key.starts_with("ui.") || key.starts_with("download."))
            || key.chars().any(char::is_control)
        {
            return Err(StorageError::InvalidMetadata(
                "setting key is invalid".into(),
            ));
        }
        if value_json.trim().is_empty()
            || value_json.len() > MAX_SETTING_BYTES
            || serde_json::from_str::<serde_json::Value>(value_json).is_err()
        {
            return Err(StorageError::InvalidMetadata(
                "value_json must be valid JSON and within the size limit".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO settings_meta (key, value_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
            params![key, value_json, now],
        )?;
        Ok(())
    }

    /// List all settings ordered by key.
    pub fn list_settings(&self) -> Result<Vec<SettingEntry>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT key, value_json, updated_at FROM settings_meta ORDER BY key ASC")?;
        let rows = statement.query_map([], |row| {
            Ok(SettingEntry {
                key: row.get(0)?,
                value_json: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    /// Delete a setting by key. Returns `true` if a row was removed.
    pub fn delete_setting(&self, key: &str) -> Result<bool, StorageError> {
        let key = key.trim();
        if !is_allowed_setting_key(key) {
            return Err(StorageError::InvalidMetadata(
                "setting key must not be empty".into(),
            ));
        }
        let changed = self
            .connection
            .execute("DELETE FROM settings_meta WHERE key = ?1", params![key])?;
        Ok(changed > 0)
    }

    pub fn upsert_tag(&self, name: &str, created_at: &str) -> Result<i64, StorageError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err(StorageError::InvalidMetadata("tag name is invalid".into()));
        }
        self.connection.execute(
            "INSERT INTO tags (name, created_at) VALUES (?1, ?2) ON CONFLICT(name) DO NOTHING",
            params![name, created_at],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?)
    }

    pub fn attach_tag(&self, tweet_row_id: i64, tag_id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO tweet_tags (tweet_id, tag_id) VALUES (?1, ?2) ON CONFLICT(tweet_id, tag_id) DO NOTHING",
            params![tweet_row_id, tag_id],
        )?;
        Ok(())
    }

    pub fn list_tweet_tags(&self, tweet_row_id: i64) -> Result<Vec<String>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT tags.name FROM tags JOIN tweet_tags ON tweet_tags.tag_id = tags.id WHERE tweet_tags.tweet_id = ?1 ORDER BY tags.name ASC",
        )?;
        let rows = statement.query_map(params![tweet_row_id], |row| row.get(0))?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(StorageError::from)
    }

    pub fn insert_tweet(
        &self,
        tweet_id: &str,
        canonical_url: &str,
        tweet_type: &str,
        text: &str,
        now: &str,
    ) -> Result<i64, StorageError> {
        self.insert_tweet_for_user(tweet_id, canonical_url, tweet_type, text, None, now)
    }

    pub fn insert_tweet_for_user(
        &self,
        tweet_id: &str,
        canonical_url: &str,
        tweet_type: &str,
        text: &str,
        user_row_id: Option<i64>,
        now: &str,
    ) -> Result<i64, StorageError> {
        self.connection.execute(
            "INSERT INTO tweets (tweet_id, canonical_url, tweet_type, text, user_id, created_at, updated_at)\n             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)\n             ON CONFLICT(tweet_id) DO UPDATE SET canonical_url = excluded.canonical_url, user_id = COALESCE(excluded.user_id, tweets.user_id), updated_at = excluded.updated_at",
            params![tweet_id, canonical_url, tweet_type, text, user_row_id, now],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM tweets WHERE tweet_id = ?1",
            params![tweet_id],
            |row| row.get(0),
        )?)
    }

    pub fn set_tweet_user(&self, tweet_row_id: i64, user_row_id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE tweets SET user_id = ?1, updated_at = updated_at WHERE id = ?2",
            params![user_row_id, tweet_row_id],
        )?;
        Ok(())
    }

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

    pub fn update_tweet_metadata(
        &self,
        tweet_row_id: i64,
        metadata: &ArchiveMetadata,
        archive_directory: &str,
    ) -> Result<(), StorageError> {
        let quoted_tweet_id = metadata
            .quoted_tweet
            .as_ref()
            .and_then(|quoted| xarchive_core::TweetId::new(quoted.tweet_id.clone()).ok())
            .map(|id| id.as_str().to_owned());
        let reply_to_tweet_id = metadata
            .reply_to
            .as_ref()
            .and_then(|reply_to| xarchive_core::TweetId::new(reply_to.clone()).ok())
            .map(|id| id.as_str().to_owned());
        self.connection.execute(
            "UPDATE tweets SET text = ?1, merged_metadata_json = ?2, archive_directory = ?3, archived_at = ?4, reply_to_tweet_id = ?5, quoted_tweet_id = ?6, updated_at = ?4 WHERE id = ?7",
            params![
                metadata.text,
                serde_json::to_string(metadata)?,
                archive_directory,
                metadata.archived_at,
                reply_to_tweet_id,
                quoted_tweet_id,
                tweet_row_id
            ],
        )?;
        Ok(())
    }

    pub fn tweet_relationships(
        &self,
        tweet_id: &str,
    ) -> Result<Option<TweetRelationships>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT reply_to_tweet_id, quoted_tweet_id FROM tweets WHERE tweet_id = ?1",
                params![tweet_id],
                |row| {
                    Ok(TweetRelationships {
                        reply_to_tweet_id: row.get(0)?,
                        quoted_tweet_id: row.get(1)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn insert_media(
        &self,
        tweet_row_id: i64,
        media: &xarchive_core::ArchiveMedia,
        now: &str,
    ) -> Result<i64, StorageError> {
        self.connection.execute(
            "INSERT INTO media (tweet_id, media_index, x_media_id, media_type, relative_path, mime_type, size_bytes, sha256, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9) ON CONFLICT(tweet_id, media_index) DO UPDATE SET relative_path = excluded.relative_path, mime_type = excluded.mime_type, size_bytes = excluded.size_bytes, sha256 = excluded.sha256, updated_at = excluded.updated_at",
            params![
                tweet_row_id,
                media.index,
                media.media_id,
                media.media_type,
                media.file,
                media.mime_type,
                media.size_bytes,
                media.sha256,
                now
            ],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM media WHERE tweet_id = ?1 AND media_index = ?2",
            params![tweet_row_id, media.index],
            |row| row.get(0),
        )?)
    }
}

fn is_allowed_setting_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 256
        && (key.starts_with("ui.") || key.starts_with("download."))
        && !key.chars().any(char::is_control)
}

fn send_state_error(error: rusqlite::Error) -> SendStateError {
    SendStateError::Store(error.to_string())
}

impl SendStateStore for Database {
    fn find_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<SentSendRecord>, SendStateError> {
        self.connection
            .query_row(
                "SELECT chat_id, idempotency_key, message_kind, telegram_message_id, telegram_file_id \
                 FROM telegram_send_attempts \
                 WHERE chat_id = ?1 AND idempotency_key = ?2 AND state = 'SENT'",
                params![chat_id, idempotency_key],
                |row| {
                    Ok(SentSendRecord {
                        chat_id: row.get(0)?,
                        idempotency_key: row.get(1)?,
                        message_kind: row.get(2)?,
                        telegram_message_id: row.get(3)?,
                        telegram_file_id: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(send_state_error)
    }

    fn record_pending(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        message_kind: &str,
        updated_at: &str,
    ) -> Result<(), SendStateError> {
        let changed = self
            .connection
            .execute(
                "INSERT INTO telegram_send_attempts \
                     (chat_id, idempotency_key, message_kind, state, attempt_count, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, 'PENDING', 1, ?4, ?4) \
                 ON CONFLICT(chat_id, idempotency_key) DO UPDATE SET \
                     state = 'PENDING', \
                     attempt_count = attempt_count + 1, \
                     telegram_message_id = NULL, \
                     telegram_file_id = NULL, \
                     last_error_code = NULL, \
                     last_error_message = NULL, \
                     updated_at = excluded.updated_at",
                params![chat_id, idempotency_key, message_kind, updated_at],
            )
            .map_err(send_state_error)?;
        if changed == 0 {
            return Err(SendStateError::Store(
                "telegram_send_attempts insert did not apply".to_owned(),
            ));
        }
        Ok(())
    }

    fn record_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        telegram_message_id: &str,
        telegram_file_id: Option<&str>,
        updated_at: &str,
    ) -> Result<(), SendStateError> {
        let changed = self
            .connection
            .execute(
                "UPDATE telegram_send_attempts \
                 SET state = 'SENT', telegram_message_id = ?3, telegram_file_id = ?4, \
                     last_error_code = NULL, last_error_message = NULL, updated_at = ?5 \
                 WHERE chat_id = ?1 AND idempotency_key = ?2",
                params![
                    chat_id,
                    idempotency_key,
                    telegram_message_id,
                    telegram_file_id,
                    updated_at
                ],
            )
            .map_err(send_state_error)?;
        if changed == 0 {
            return Err(SendStateError::NotFound);
        }
        Ok(())
    }

    fn record_failed(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        error_code: Option<i64>,
        error_message: &str,
        updated_at: &str,
    ) -> Result<(), SendStateError> {
        let changed = self
            .connection
            .execute(
                "UPDATE telegram_send_attempts \
                 SET state = 'FAILED', last_error_code = ?3, last_error_message = ?4, updated_at = ?5 \
                 WHERE chat_id = ?1 AND idempotency_key = ?2",
                params![
                    chat_id,
                    idempotency_key,
                    error_code.map(|code| code.to_string()),
                    error_message,
                    updated_at
                ],
            )
            .map_err(send_state_error)?;
        if changed == 0 {
            return Err(SendStateError::NotFound);
        }
        Ok(())
    }

    fn list_unsent(&self) -> Result<Vec<PendingSendRecord>, SendStateError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT chat_id, idempotency_key, message_kind, state, attempt_count \
                 FROM telegram_send_attempts \
                 WHERE state IN ('PENDING', 'FAILED') \
                 ORDER BY updated_at ASC, id ASC",
            )
            .map_err(send_state_error)?;
        let rows = statement
            .query_map([], |row| {
                let state_value: String = row.get(3)?;
                let attempt_count: i64 = row.get(4)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    state_value,
                    attempt_count,
                ))
            })
            .map_err(send_state_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(send_state_error)?;
        rows.into_iter()
            .map(
                |(chat_id, idempotency_key, message_kind, state_value, attempt_count)| {
                    let state = SendState::parse(&state_value).ok_or_else(|| {
                        SendStateError::Store(format!("invalid send state: {state_value}"))
                    })?;
                    Ok(PendingSendRecord {
                        chat_id,
                        idempotency_key,
                        message_kind,
                        state,
                        attempt_count: attempt_count.clamp(0, i64::from(u32::MAX)) as u32,
                    })
                },
            )
            .collect()
    }
}

pub struct FileStore {
    root: PathBuf,
}

pub struct ArchiveService {
    pub database: Database,
    pub files: FileStore,
}

/// Untrusted Sidecar output plus the request identity needed to commit it.
/// Keeping the operation context together avoids a wide positional API and
/// makes the identity binding explicit at the archive boundary.
pub struct SidecarArchiveRequest<'a> {
    pub job_id: &'a str,
    pub tweet_row_id: i64,
    pub expected_tweet_id: &'a str,
    pub metadata: &'a serde_json::Value,
    pub files: &'a [xarchive_protocol::DownloadFile],
    pub final_directory: &'a Path,
    pub archived_at: &'a str,
}

impl ArchiveService {
    pub fn new(database: Database, files: FileStore) -> Self {
        Self { database, files }
    }

    /// Commit a completed Sidecar result as one local archive operation.
    ///
    /// The caller must provide a staging directory containing already
    /// validated files. This service writes portable metadata, registers
    /// media hashes, then advances the job only after the directory commit.
    pub fn complete_local_archive(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        metadata: &ArchiveMetadata,
        final_directory: &Path,
    ) -> Result<PathBuf, StorageError> {
        let state = self.database.job_state(job_id)?;
        if state == JobState::Queued {
            self.database
                .transition_job(job_id, JobState::Validating, &metadata.archived_at)?;
            self.database
                .transition_job(job_id, JobState::MetadataReady, &metadata.archived_at)?;
            self.database
                .transition_job(job_id, JobState::Downloading, &metadata.archived_at)?;
        }

        let staging = self.files.staging_dir(job_id)?;
        let json_path = staging.join("tweet.json");
        fs::write(&json_path, serde_json::to_vec_pretty(metadata)?)?;
        let text = format!(
            "{}\n@{}\n\n{}\n",
            metadata.author.display_name.as_deref().unwrap_or(""),
            metadata.author.username.as_deref().unwrap_or(""),
            metadata.text
        );
        fs::write(staging.join("tweet.txt"), text)?;

        let committed = self.files.commit_staging(job_id, final_directory)?;
        let relative_directory = final_directory.to_string_lossy().to_string();
        self.database
            .update_tweet_metadata(tweet_row_id, metadata, &relative_directory)?;
        self.refresh_author_profile(tweet_row_id, metadata)?;
        for media in &metadata.media {
            self.database
                .insert_media(tweet_row_id, media, &metadata.archived_at)?;
        }
        self.database
            .transition_job(job_id, JobState::Downloaded, &metadata.archived_at)?;
        Ok(committed)
    }

    /// Register the archive author and refresh their portable profile file.
    ///
    /// Tweets without a resolvable `user_id` skip registration silently:
    /// no user row is created and no profile file is written.
    fn refresh_author_profile(
        &mut self,
        tweet_row_id: i64,
        metadata: &ArchiveMetadata,
    ) -> Result<(), StorageError> {
        let Some(user_id) = metadata
            .author
            .user_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
        else {
            return Ok(());
        };
        let user_row_id = self.database.upsert_user(
            &user_id,
            metadata.author.username.as_deref(),
            metadata.author.display_name.as_deref(),
            &metadata.archived_at,
        )?;
        self.database.set_tweet_user(tweet_row_id, user_row_id)?;
        let Some(snapshot) = self.database.user_profile(&user_id)? else {
            return Ok(());
        };
        self.files.write_user_profile(&UserProfileFile {
            schema_version: 1,
            user_id: snapshot.user_id,
            stable_directory_name: snapshot.stable_directory_name,
            username: snapshot.username,
            display_name: snapshot.display_name,
            first_seen_at: snapshot.first_seen_at,
            last_seen_at: snapshot.last_seen_at,
            updated_at: metadata.archived_at.clone(),
            names: snapshot.names,
        })?;
        Ok(())
    }

    /// Convert a Sidecar result into trusted local metadata and commit it.
    ///
    /// Sidecar-reported paths and sizes are treated as untrusted hints. Rust
    /// resolves each path below the job staging directory, reads the actual
    /// file size, and computes the SHA-256 before writing the database record.
    pub fn complete_sidecar_archive(
        &mut self,
        request: SidecarArchiveRequest<'_>,
    ) -> Result<PathBuf, StorageError> {
        let staging = self.files.staging_dir(request.job_id)?;
        let archive_metadata = build_archive_metadata(
            request.expected_tweet_id,
            request.metadata,
            request.files,
            &staging,
            request.archived_at,
        )?;
        self.complete_local_archive(
            request.job_id,
            request.tweet_row_id,
            &archive_metadata,
            request.final_directory,
        )
    }
}

/// Build portable metadata from the variable-shaped gallery-dl payload.
pub fn build_archive_metadata(
    expected_tweet_id: &str,
    raw: &serde_json::Value,
    files: &[xarchive_protocol::DownloadFile],
    staging_dir: &Path,
    archived_at: &str,
) -> Result<ArchiveMetadata, StorageError> {
    let object = raw
        .as_object()
        .ok_or_else(|| StorageError::InvalidMetadata("metadata must be an object".into()))?;
    let string_value = |keys: &[&str]| {
        keys.iter().find_map(|key| {
            object.get(*key).and_then(|value| match value {
                serde_json::Value::String(value) => Some(value.clone()),
                serde_json::Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
        })
    };
    let tweet_id = string_value(&["tweet_id", "status_id", "id"]).ok_or_else(|| {
        StorageError::InvalidMetadata("tweet_id is missing or not a string".into())
    })?;
    xarchive_core::TweetId::new(tweet_id.clone())
        .map_err(|_| StorageError::InvalidMetadata("tweet_id must be numeric".into()))?;
    if tweet_id != expected_tweet_id {
        return Err(StorageError::InvalidMetadata(
            "sidecar tweet_id does not match the archive request".into(),
        ));
    }

    let media_values = object
        .get("media")
        .or_else(|| object.get("items"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut media = Vec::with_capacity(files.len());
    for (position, file) in files.iter().enumerate() {
        let relative = Path::new(&file.relative_path);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(StorageError::InvalidPath);
        }
        let path = staging_dir.join(relative);
        let file_metadata = fs::symlink_metadata(&path)?;
        if !file_metadata.file_type().is_file() || is_reparse_point(&file_metadata) {
            return Err(StorageError::InvalidPath);
        }
        if !path.is_file() {
            return Err(StorageError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("sidecar file is missing: {}", file.relative_path),
            )));
        }
        let size_bytes = fs::metadata(&path)?.len();
        let sha256 = FileStore::sha256(&path)?;
        let raw_media = media_values
            .get(position)
            .and_then(serde_json::Value::as_object);
        let media_id = raw_media
            .and_then(|value| value.get("media_id").or_else(|| value.get("id")))
            .map(|value| value.to_string().trim_matches('"').to_owned());
        media.push(xarchive_core::ArchiveMedia {
            index: position as u32 + 1,
            media_id,
            media_type: file.media_type.clone(),
            file: file.relative_path.clone(),
            mime_type: file.mime_type.clone(),
            size_bytes,
            sha256,
        });
    }

    let quoted_tweet = object
        .get("quoted_tweet")
        .or_else(|| object.get("quoted_status"))
        .and_then(serde_json::Value::as_object)
        .and_then(|quoted_object| {
            let quoted_id = quoted_object
                .get("tweet_id")
                .or_else(|| quoted_object.get("status_id"))
                .or_else(|| quoted_object.get("id"))
                .and_then(|value| match value {
                    serde_json::Value::String(value) => Some(value.clone()),
                    serde_json::Value::Number(value) => Some(value.to_string()),
                    _ => None,
                })?;
            xarchive_core::TweetId::new(quoted_id.clone()).ok()?;
            Some(ArchiveQuotedTweet {
                tweet_id: quoted_id,
                url: quoted_object
                    .get("url")
                    .or_else(|| quoted_object.get("tweet_url"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                username: quoted_object
                    .get("username")
                    .or_else(|| quoted_object.get("author_username"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                display_name: quoted_object
                    .get("display_name")
                    .or_else(|| quoted_object.get("author_name"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                text: quoted_object
                    .get("text")
                    .or_else(|| quoted_object.get("description"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                created_at: quoted_object
                    .get("created_at")
                    .or_else(|| quoted_object.get("date"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                tweet_type: quoted_object
                    .get("tweet_type")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            })
        });

    Ok(ArchiveMetadata {
        schema_version: 1,
        tweet_id,
        url: string_value(&["url", "tweet_url"]).unwrap_or_default(),
        tweet_type: string_value(&["tweet_type", "type"]).unwrap_or_else(|| "post".into()),
        author: xarchive_core::ArchiveAuthor {
            user_id: string_value(&["user_id", "author_id"]),
            username: string_value(&["username", "author_username", "user"]),
            display_name: string_value(&["display_name", "author_name"]),
        },
        created_at: string_value(&["created_at", "date", "timestamp"]),
        text: string_value(&["text", "description"]).unwrap_or_default(),
        media,
        archived_at: archived_at.to_owned(),
        reply_to: string_value(&[
            "in_reply_to_status_id_str",
            "in_reply_to_status_id",
            "in_reply_to",
            "reply_to_tweet_id",
            "reply_to",
        ])
        .filter(|value| xarchive_core::TweetId::new(value.clone()).is_ok()),
        quoted_tweet,
    })
}

impl FileStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("_staging"))?;
        Ok(Self { root })
    }

    pub fn staging_dir(&self, job_id: &str) -> Result<PathBuf, StorageError> {
        let path = self.safe_child(&Path::new("_staging").join(job_id))?;
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn write_json<T: serde::Serialize>(
        &self,
        relative: impl AsRef<Path>,
        value: &T,
    ) -> Result<PathBuf, StorageError> {
        let path = self.safe_child(relative.as_ref())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_vec_pretty(value)?)?;
        Ok(path)
    }

    pub fn write_text(
        &self,
        relative: impl AsRef<Path>,
        content: &str,
    ) -> Result<PathBuf, StorageError> {
        let path = self.safe_child(relative.as_ref())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        Ok(path)
    }

    /// Resolve the `profile.json` path inside the stable user directory.
    pub fn user_profile_path(&self, stable_directory_name: &str) -> Result<PathBuf, StorageError> {
        if stable_directory_name.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "stable directory name must not be empty".into(),
            ));
        }
        self.safe_child(
            &Path::new("Users")
                .join(stable_directory_name)
                .join("profile.json"),
        )
    }

    /// Write or refresh the portable `profile.json` in the user directory.
    pub fn write_user_profile(&self, profile: &UserProfileFile) -> Result<PathBuf, StorageError> {
        let path = self.user_profile_path(&profile.stable_directory_name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_vec_pretty(profile)?)?;
        Ok(path)
    }

    pub fn sha256(path: impl AsRef<Path>) -> Result<String, StorageError> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn commit_staging(
        &self,
        job_id: &str,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, StorageError> {
        let staging = self.safe_child(&Path::new("_staging").join(job_id))?;
        if !staging.is_dir() {
            return Err(StorageError::InvalidPath);
        }
        let destination = self.safe_child(destination.as_ref())?;
        if destination.starts_with(self.root.join("_staging")) {
            return Err(StorageError::InvalidPath);
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            return Err(StorageError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "destination already exists",
            )));
        }
        fs::rename(&staging, &destination)?;
        Ok(destination)
    }

    fn safe_child(&self, relative: &Path) -> Result<PathBuf, StorageError> {
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(StorageError::InvalidPath);
        }
        Ok(self.root.join(relative))
    }
}

#[cfg(unix)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(any(unix, windows)))]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        assert_eq!(version, 3);
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
}
