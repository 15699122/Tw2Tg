//! SQLite persistence and local file storage for the Desktop application.

mod archive_service;
mod database {
    pub mod jobs;
    pub mod settings;
    pub mod tags;
    pub mod telegram;
    pub mod tweets;
    pub mod users;
}
mod error;
mod file_store;
mod metadata;
mod models;

pub use archive_service::{ArchiveService, SidecarArchiveRequest};
pub use error::StorageError;
pub use file_store::FileStore;
pub use metadata::build_archive_metadata;
pub use models::{
    JobSummary, SettingEntry, TweetRelationships, UserNameSummary, UserProfileFile,
    UserProfileSnapshot, UserSummary,
};

use rusqlite::Connection;
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_telegram_send_state.sql"),
    include_str!("../migrations/0003_quote_reply_relationships.sql"),
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
