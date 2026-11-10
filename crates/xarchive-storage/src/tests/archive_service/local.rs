//! `archive_service` — local coverage.

use super::*;

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
    let stable = xarchive_core::stable_user_directory_name(Some("alice"), Some("Alice"), "user-1");
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
