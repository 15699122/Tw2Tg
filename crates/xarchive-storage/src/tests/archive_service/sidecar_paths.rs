//! `archive_service` — sidecar paths coverage.

use super::*;

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
    let preview = service
        .preview_sidecar_archive(
            "job-1",
            "123",
            &raw,
            &sidecar_files,
            "2026-09-08T00:01:00Z",
            &BatchOutputSettings {
                naming_mode: BatchNamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                ..BatchOutputSettings::default()
            },
        )
        .expect("preview");
    assert_eq!(preview.metadata.media[0].file, "123_01.jpg");
    assert_eq!(preview.renames, [("01.jpg".into(), "123_01.jpg".into())]);
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
            output_settings: &BatchOutputSettings::default(),
        })
        .expect("sidecar archive");
    assert!(destination.join("01.jpg").is_file());
    assert!(destination.join("tweet.json").is_file());
    let saved: xarchive_core::ArchiveMetadata =
        serde_json::from_slice(&fs::read(destination.join("tweet.json")).expect("metadata"))
            .expect("parse metadata");
    assert_eq!(saved.media[0].file, "01.jpg");
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
