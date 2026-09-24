//! SQLite persistence and local file storage for the Desktop application.

mod archive_service;
mod database {
    pub mod batches;
    pub mod jobs;
    pub mod settings;
    pub mod tags;
    pub mod telegram;
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
    ArchivedMediaFact, JobEventRecord, JobMetrics, JobSummary, SettingEntry, TweetArchiveFacts,
    TweetRelationships, UserNameSummary, UserProfileFile, UserProfileSnapshot, UserSummary,
};

use rusqlite::Connection;
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_telegram_send_state.sql"),
    include_str!("../migrations/0003_quote_reply_relationships.sql"),
    include_str!("../migrations/0004_archive_job_requests.sql"),
    include_str!("../migrations/0005_account_batches.sql"),
    include_str!("../migrations/0006_batch_discovery_paused.sql"),
];

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
    use xarchive_telegram::{SendState, SendStateError, SendStateStore, SentSendRecord};

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
        assert_eq!(version, 6);
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
}
