//! SQLite persistence and local file storage for the Desktop application.

use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use xarchive_core::JobState;

const MIGRATION: &str = include_str!("../../../desktop/src-tauri/migrations/0001_initial.sql");

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Io(io::Error),
    Json(serde_json::Error),
    InvalidPath,
    InvalidState(String),
    InvalidTransition(xarchive_core::JobStateError),
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
        connection.execute_batch(MIGRATION)?;
        Ok(Self { connection })
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
}

pub struct FileStore {
    root: PathBuf,
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
}
