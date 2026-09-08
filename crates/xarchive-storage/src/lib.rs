//! SQLite persistence and local file storage for the Desktop application.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use xarchive_core::{ArchiveMetadata, JobState};

const MIGRATION: &str = include_str!("../../../desktop/src-tauri/migrations/0001_initial.sql");

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserNameSummary {
    pub username: String,
    pub display_name: Option<String>,
    pub source: String,
    pub observed_at: String,
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
        if current_version < 1 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(MIGRATION)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                [],
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
            self.record_user_name(row_id, username, display_name, "unknown", now)?;
        }
        Ok(row_id)
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
        self.connection.execute(
            "INSERT INTO tweets (tweet_id, canonical_url, tweet_type, text, created_at, updated_at)\n             VALUES (?1, ?2, ?3, ?4, ?5, ?5)\n             ON CONFLICT(tweet_id) DO UPDATE SET canonical_url = excluded.canonical_url, updated_at = excluded.updated_at",
            params![tweet_id, canonical_url, tweet_type, text, now],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM tweets WHERE tweet_id = ?1",
            params![tweet_id],
            |row| row.get(0),
        )?)
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

    pub fn count_job_events(&self, job_id: &str) -> Result<i64, StorageError> {
        Ok(self.connection.query_row(
            "SELECT COUNT(*) FROM events WHERE job_id = ?1",
            params![job_id],
            |row| row.get(0),
        )?)
    }

    pub fn update_tweet_metadata(
        &self,
        tweet_row_id: i64,
        metadata: &ArchiveMetadata,
        archive_directory: &str,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE tweets SET text = ?1, merged_metadata_json = ?2, archive_directory = ?3, archived_at = ?4, updated_at = ?4 WHERE id = ?5",
            params![
                metadata.text,
                serde_json::to_string(metadata)?,
                archive_directory,
                metadata.archived_at,
                tweet_row_id
            ],
        )?;
        Ok(())
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

pub struct FileStore {
    root: PathBuf,
}

pub struct ArchiveService {
    pub database: Database,
    pub files: FileStore,
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
        for media in &metadata.media {
            self.database
                .insert_media(tweet_row_id, media, &metadata.archived_at)?;
        }
        self.database
            .transition_job(job_id, JobState::Downloaded, &metadata.archived_at)?;
        Ok(committed)
    }

    /// Convert a Sidecar result into trusted local metadata and commit it.
    ///
    /// Sidecar-reported paths and sizes are treated as untrusted hints. Rust
    /// resolves each path below the job staging directory, reads the actual
    /// file size, and computes the SHA-256 before writing the database record.
    pub fn complete_sidecar_archive(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        metadata: &serde_json::Value,
        files: &[xarchive_protocol::DownloadFile],
        final_directory: &Path,
        archived_at: &str,
    ) -> Result<PathBuf, StorageError> {
        let staging = self.files.staging_dir(job_id)?;
        let archive_metadata = build_archive_metadata(metadata, files, &staging, archived_at)?;
        self.complete_local_archive(job_id, tweet_row_id, &archive_metadata, final_directory)
    }
}

/// Build portable metadata from the variable-shaped gallery-dl payload.
pub fn build_archive_metadata(
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
            build_archive_metadata(&raw, &sidecar_files, &staging, "now").expect("metadata");
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
            build_archive_metadata(&raw, &sidecar_files, &staging, "now"),
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
        let metadata = build_archive_metadata(&raw, &[], &staging, "now").expect("metadata");
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
            .complete_sidecar_archive(
                "job-1",
                tweet_row_id,
                &raw,
                &sidecar_files,
                Path::new("Users/alice/2026/09/123"),
                "2026-09-08T00:01:00Z",
            )
            .expect("sidecar archive");
        assert!(destination.join("01.jpg").is_file());
        assert!(destination.join("tweet.json").is_file());
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
}
