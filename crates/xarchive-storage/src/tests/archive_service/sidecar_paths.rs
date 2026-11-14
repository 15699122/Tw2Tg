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
            metadata_exports: None,
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

/// Publishing with every user-facing export disabled, then replaying the
/// durable PREPARED journal, must converge without any user export file. This
/// is the D1 contract that recovery never depends on optional exports.
#[test]
fn prepared_replay_converges_when_all_user_exports_are_disabled() {
    let root = temp_root();
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet(
            "424243",
            "https://x.com/alice/status/424243",
            "post",
            "",
            "now",
        )
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "replay-job",
            tweet_row_id,
            1,
            "replay-request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("job");
    let attempt = database
        .claim_archive_attempt(
            "replay-job",
            0,
            JobState::Queued,
            JobState::Validating,
            "now",
        )
        .expect("claim")
        .expect("claim winner");
    for state in [JobState::MetadataReady, JobState::Downloading] {
        database
            .transition_job("replay-job", state, "now")
            .expect("state");
    }
    let files =
        FileStore::with_staging_root(root.join("archive"), root.join("staging")).expect("files");
    let staging = files
        .create_attempt_staging_dir("replay-job", attempt)
        .expect("attempt staging");
    std::fs::write(staging.join("01.jpg"), b"replay media").expect("media");
    let raw = serde_json::json!({
        "tweet_id": "424243",
        "tweet_url": "https://x.com/alice/status/424243",
        "username": "alice",
        "display_name": "Alice",
        "text": "no exports at all",
        "media": [{"id": "media-1", "type": "photo"}]
    });
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "01.jpg".into(),
        size_bytes: 12,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let mut service = ArchiveService::new(database, files);
    // Both formats off: an empty selection, not `None`.
    let exports: Vec<(&str, Vec<u8>)> = Vec::new();
    let destination = service
        .complete_sidecar_archive_attempt_for_test(
            SidecarArchiveRequest {
                job_id: "replay-job",
                tweet_row_id,
                expected_tweet_id: "424243",
                metadata: &raw,
                files: &sidecar_files,
                final_directory: Path::new("Tweets/424243"),
                archived_at: "2026-11-14T00:00:00Z",
                telegram_intent: None,
                output_settings: &BatchOutputSettings::default(),
                metadata_exports: Some(&exports),
            },
            attempt,
            None,
        )
        .expect("publish with no exports");
    // No user-facing file at all, but the internal recovery facts exist.
    assert!(!destination.join("tweet.json").exists());
    assert!(!destination.join("tweet.yaml").exists());
    assert!(!destination.join("tweet.txt").exists());
    let manifest = service
        .database
        .verify_archive_recovery_manifest("replay-job")
        .expect("internal manifest survives without exports");
    assert_eq!(manifest.attempt_count, attempt);
    // The committed tree verifies from internal facts alone.
    let attempt_identity =
        xarchive_workflow::AttemptIdentity::new("replay-job", attempt).expect("identity");
    service
        .verify_committed_internal_v1(
            "replay-job",
            tweet_row_id,
            Path::new("Tweets/424243"),
            &attempt_identity,
        )
        .expect("committed verification must not read user exports");
    let _ = fs::remove_dir_all(root);
}

/// selection decides exactly which user-facing files are written. `None` keeps
/// the historical `tweet.json` + `tweet.txt` behaviour; `Some` writes only the
/// supplied formats, including none at all, and publication still converges.
#[cfg(test)]
#[test]
fn internal_v1_attempt_publish_writes_exactly_the_selected_metadata_exports() {
    for (export_json, export_yaml) in [(false, false), (true, false), (false, true), (true, true)] {
        let root = temp_root();
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet(
                "424242",
                "https://x.com/alice/status/424242",
                "post",
                "",
                "now",
            )
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "d1-job",
                tweet_row_id,
                4,
                "d1-request",
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job");
        let attempt = database
            .begin_archive_attempt("d1-job", "now")
            .expect("attempt");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
        ] {
            database
                .transition_job("d1-job", state, "now")
                .expect("state");
        }
        let files = FileStore::with_staging_root(root.join("archive"), root.join("staging"))
            .expect("files");
        let staging = files
            .create_attempt_staging_dir("d1-job", attempt)
            .expect("attempt staging");
        fs::write(staging.join("01.jpg"), b"d1 media").expect("media");
        let raw = serde_json::json!({
            "tweet_id": "424242",
            "tweet_url": "https://x.com/alice/status/424242",
            "username": "alice",
            "display_name": "Alice",
            "text": "d1 export selection",
            "media": [{"id": "media-1", "type": "photo"}]
        });
        let sidecar_files = vec![xarchive_protocol::DownloadFile {
            relative_path: "01.jpg".into(),
            size_bytes: 8,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
        }];
        let mut service = ArchiveService::new(database, files);
        let preview = service
            .preview_sidecar_archive_attempt(
                "d1-job",
                attempt,
                "424242",
                &raw,
                &sidecar_files,
                "2026-11-14T00:00:00Z",
                &BatchOutputSettings::default(),
            )
            .expect("preview");
        let mut exports: Vec<(&str, Vec<u8>)> = Vec::new();
        if export_json {
            exports.push((
                "tweet.json",
                serde_json::to_vec_pretty(&preview.metadata).expect("json"),
            ));
        }
        if export_yaml {
            exports.push((
                "tweet.yaml",
                b"tweet_id: 424242
"
                .to_vec(),
            ));
        }
        let destination = service
            .complete_sidecar_archive_attempt_for_test(
                SidecarArchiveRequest {
                    job_id: "d1-job",
                    tweet_row_id,
                    expected_tweet_id: "424242",
                    metadata: &raw,
                    files: &sidecar_files,
                    final_directory: Path::new("Tweets/424242"),
                    archived_at: "2026-11-14T00:00:00Z",
                    telegram_intent: None,
                    output_settings: &BatchOutputSettings::default(),
                    metadata_exports: Some(&exports),
                },
                attempt,
                None,
            )
            .expect("attempt-scoped publish with explicit exports");
        assert_eq!(
            destination.join("tweet.json").is_file(),
            export_json,
            "tweet.json presence must follow the selection"
        );
        assert_eq!(
            destination.join("tweet.yaml").is_file(),
            export_yaml,
            "tweet.yaml presence must follow the selection"
        );
        assert!(!destination.join("tweet.txt").exists());
        assert_eq!(
            service.database.job_state("d1-job").expect("state"),
            JobState::Downloaded
        );
        // Recovery must not depend on the user-facing exports: the durable
        // PREPARED journal and its internal manifest are gone only after the
        // journal reached COMMITTED, which this published case already proves.
        assert_eq!(
            service
                .database
                .archive_recovery_journal("d1-job")
                .expect("journal")
                .map(|journal| journal.phase),
            Some(ArchiveRecoveryPhase::Committed),
            "publication must commit the internal journal regardless of exports"
        );
        let _ = fs::remove_dir_all(root);
    }
}
