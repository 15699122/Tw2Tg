use super::*;
use rusqlite::OptionalExtension;

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
#[test]
fn internal_v1_telegram_intent_refuses_recovery_without_runner_permit() {
    let root = temp_root();
    fs::create_dir_all(&root).expect("root");
    let database_path = root.join("recovery.sqlite3");
    let mut database = Database::open(&database_path).expect("database");
    let tweet_row_id = database
        .insert_tweet(
            "intent-v1",
            "https://x.com/a/status/intent-v1",
            "post",
            "",
            "t0",
        )
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "intent-v1-job",
            tweet_row_id,
            1,
            "request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "t0",
        )
        .expect("InternalV1 job");
    let files = FileStore::new(&root).expect("files");
    for (name, contents) in [("01.jpg", &b"payload"[..]), ("tweet.json", &b"{}"[..])] {
        fs::write(
            files
                .staging_dir("intent-v1-job")
                .expect("staging")
                .join(name),
            contents,
        )
        .expect("staging file");
    }
    let intent = TelegramArchiveIntentRecord {
        job_id: "intent-v1-job".into(),
        tweet_row_id,
        archive_directory: "Tweets/intent-v1".into(),
        state: "PREPARED".into(),
        metadata_text: None,
        media_json: "[]".into(),
        bot_identity: Some("bot".into()),
        chat_id: Some("-1".into()),
        message_thread_id: None,
        config_revision: None,
        plan_version: 1,
        created_at: "t0".into(),
        updated_at: "t0".into(),
    };
    database
        .prepare_telegram_archive_intent(&intent)
        .expect("intent");
    let mut service = ArchiveService::new(database, files);
    let error = service
        .recover_telegram_archive_intent(&intent, "t1")
        .expect_err("runner authorization required");
    assert!(
        error
            .to_string()
            .contains("runner-owned Workflow authorization")
    );
    assert_eq!(
        service
            .database
            .telegram_archive_intent(tweet_row_id)
            .expect("intent query"),
        Some(intent)
    );
    assert_eq!(
        service.database.job_state("intent-v1-job").expect("state"),
        JobState::Queued
    );
    assert!(root.join("_staging/intent-v1-job/01.jpg").is_file());
    assert!(!root.join("Tweets/intent-v1").exists());
    drop(service);
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
#[test]
fn attempt_archive_preview_uses_attempt_payload_not_legacy_staging() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job("job-1", tweet_row_id, "now")
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    fs::write(
        files
            .staging_dir("job-1")
            .expect("legacy staging")
            .join("01.jpg"),
        b"legacy payload",
    )
    .expect("legacy media");
    let attempt = files
        .create_attempt_staging_dir("job-1", 1)
        .expect("attempt staging");
    fs::write(attempt.join("01.jpg"), b"attempt payload").expect("attempt media");
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "01.jpg".into(),
        size_bytes: 0,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let raw = serde_json::json!({
        "tweet_id": "123",
        "media": [{"id": "media-1", "type": "photo"}]
    });
    let service = ArchiveService::new(database, files);

    let preview = service
        .preview_sidecar_archive_attempt(
            "job-1",
            1,
            "123",
            &raw,
            &sidecar_files,
            "2026-10-12T00:00:00Z",
            &BatchOutputSettings::default(),
        )
        .expect("attempt-scoped preview");

    assert_eq!(preview.metadata.media[0].size_bytes, 15);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn attempt_archive_preview_binds_metadata_to_transfer_relative_path_and_hash() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job("job-transfer", tweet_row_id, "now")
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    let attempt = files
        .create_attempt_staging_dir("job-transfer", 4)
        .expect("attempt staging");
    let transferred = b"verified transfer payload";
    fs::create_dir_all(attempt.join("media")).expect("transfer output directory");
    fs::write(attempt.join("media/01.jpg"), transferred).expect("transfer output");
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "media/01.jpg".into(),
        size_bytes: transferred.len() as u64,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let raw = serde_json::json!({
        "tweet_id": "123",
        "media": [{"id": "media-1", "type": "photo"}]
    });
    let service = ArchiveService::new(database, files);

    let preview = service
        .preview_sidecar_archive_attempt(
            "job-transfer",
            4,
            "123",
            &raw,
            &sidecar_files,
            "2026-10-14T00:00:00Z",
            &BatchOutputSettings::default(),
        )
        .expect("attempt-scoped transfer preview");

    assert_eq!(preview.metadata.media.len(), 1);
    assert_eq!(
        preview.metadata.media[0].size_bytes,
        transferred.len() as u64
    );
    assert_eq!(
        preview.metadata.media[0].sha256,
        FileStore::sha256(attempt.join("media/01.jpg")).expect("transfer hash")
    );
    assert!(
        service
            .preview_sidecar_archive(
                "job-transfer",
                "123",
                &raw,
                &sidecar_files,
                "2026-10-14T00:00:00Z",
                &BatchOutputSettings::default(),
            )
            .is_err(),
        "legacy Job staging must not resolve an attempt-relative transfer path"
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn attempt_archive_preview_never_falls_back_to_legacy_staging() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job("job-1", tweet_row_id, "now")
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    fs::write(
        files
            .staging_dir("job-1")
            .expect("legacy staging")
            .join("01.jpg"),
        b"must not be used",
    )
    .expect("legacy media");
    let service = ArchiveService::new(database, files);
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "01.jpg".into(),
        size_bytes: 0,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let raw = serde_json::json!({
        "tweet_id": "123",
        "media": [{"id": "media-1", "type": "photo"}]
    });

    assert!(
        service
            .preview_sidecar_archive_attempt(
                "job-1",
                1,
                "123",
                &raw,
                &sidecar_files,
                "2026-10-12T00:00:00Z",
                &BatchOutputSettings::default(),
            )
            .is_err()
    );

    let _ = fs::remove_dir_all(root);
}
#[test]
fn attempt_archive_preview_rejects_reported_path_escape() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let files = FileStore::new(&root).expect("files");
    let attempt = files
        .create_attempt_staging_dir("job-escape", 3)
        .expect("attempt staging");
    fs::write(root.join("escape.jpg"), b"outside attempt").expect("outside payload");
    let service = ArchiveService::new(database, files);
    let raw = serde_json::json!({"tweet_id": "123"});
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "../escape.jpg".into(),
        size_bytes: 14,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];

    assert!(matches!(
        service.preview_sidecar_archive_attempt(
            "job-escape",
            3,
            "123",
            &raw,
            &sidecar_files,
            "2026-10-12T00:00:00Z",
            &BatchOutputSettings::default(),
        ),
        Err(StorageError::InvalidPath)
    ));
    assert_eq!(
        fs::read(root.join("escape.jpg")).expect("outside unchanged"),
        b"outside attempt"
    );
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(attempt);
}

#[test]
fn v2_candidate_contract_preserves_legacy_business_payload_and_rejects_extensions() {
    let root = temp_root();
    let (database, _, _, manifest, mut metadata) = unprepared_v2_publish_inputs(&root, "job-1");
    metadata.author.display_name = Some("Alice".into());
    metadata.reply_to = Some("100".into());
    metadata.quoted_tweet = Some(xarchive_core::ArchiveQuotedTweet {
        tweet_id: "99".into(),
        url: "https://twitter.com/bob/status/99".into(),
        username: Some("bob".into()),
        display_name: None,
        user_id: None,
        text: Some("quoted text".into()),
        created_at: None,
        tweet_type: None,
    });
    let legacy = serde_json::to_string(&metadata).unwrap();
    assert_eq!(
        crate::recovery_candidate::encode(&metadata).unwrap(),
        legacy
    );
    assert_eq!(
        crate::recovery_candidate::decode(&legacy).unwrap(),
        metadata
    );
    let mut production_metadata = metadata.clone();
    production_metadata.schema_version = 2;
    assert_eq!(
        crate::recovery_candidate::decode(
            &crate::recovery_candidate::encode(&production_metadata).unwrap()
        )
        .unwrap(),
        production_metadata
    );
    production_metadata.schema_version = 3;
    assert!(crate::recovery_candidate::encode(&production_metadata).is_err());
    database
        .prepare_archive_recovery_v2(&manifest, &metadata, "now")
        .unwrap();
    assert_eq!(
        database
            .archive_recovery_v2_candidate("job-1", manifest.attempt_count, manifest.tweet_row_id)
            .unwrap(),
        Some(metadata)
    );
    for pointer in ["", "/author", "/media/0", "/quoted_tweet"] {
        let mut value: serde_json::Value = serde_json::from_str(&legacy).unwrap();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                "request_headers".into(),
                serde_json::json!({"Authorization": "secret"}),
            );
        assert!(crate::recovery_candidate::decode(&value.to_string()).is_err());
    }
    let mut unsafe_quote: serde_json::Value = serde_json::from_str(&legacy).unwrap();
    unsafe_quote["quoted_tweet"]["url"] =
        serde_json::json!("https://twitter.com/bob/status/99?signature=secret");
    assert!(crate::recovery_candidate::decode(&unsafe_quote.to_string()).is_err());
    // An old stored row with an otherwise valid digest must not have its
    // unsupported fields silently discarded or be rewritten on read.
    let mut extended: serde_json::Value = serde_json::from_str(&legacy).unwrap();
    extended["cookies"] = serde_json::json!("secret");
    let extended = extended.to_string();
    use sha2::Digest;
    let digest = format!("{:x}", sha2::Sha256::digest(extended.as_bytes()));
    database.connection.execute(
            "UPDATE archive_recovery_v2_candidate SET metadata_json = ?1, metadata_sha256 = ?2 WHERE job_id = 'job-1'",
            rusqlite::params![extended, digest],
        ).unwrap();
    assert!(
        database
            .archive_recovery_v2_candidate("job-1", manifest.attempt_count, manifest.tweet_row_id)
            .is_err()
    );
    let retained: String = database
        .connection
        .query_row(
            "SELECT metadata_json FROM archive_recovery_v2_candidate WHERE job_id = 'job-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, extended);
    drop(database);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn v2_candidate_contract_refuses_unsafe_urls_before_any_prepared_writes() {
    let root = temp_root();
    let (database, _, _, manifest, metadata) = unprepared_v2_publish_inputs(&root, "job-1");
    for url in [
        "https://x.com/alice/status/123?token=secret",
        "https://user:secret@x.com/alice/status/123",
        "https://video.twimg.com/123?signature=secret",
        "https://x.com/alice/status/999",
        "https://x.com.evil/alice/status/123",
    ] {
        let mut unsafe_metadata = metadata.clone();
        unsafe_metadata.url = url.into();
        assert!(
            database
                .prepare_archive_recovery_v2(&manifest, &unsafe_metadata, "now")
                .is_err()
        );
        let counts: (i64, i64) = database.connection.query_row(
                "SELECT (SELECT COUNT(*) FROM archive_recovery_v2), (SELECT COUNT(*) FROM archive_recovery_v2_candidate)",
                [], |row| Ok((row.get(0)?, row.get(1)?)),
            ).unwrap();
        assert_eq!(counts, (0, 0));
    }
    drop(database);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn v2_prepared_candidate_failures_preserve_journal_and_staging() {
    use sha2::Digest;
    for mode in ["missing", "corrupt", "unsupported", "identity", "media"] {
        let root = temp_root();
        let (database, files, tweet) = prepared_v2_rename_fixture(&root, "job-1");
        let journal_snapshot = |database: &Database| -> (String, String, String) {
            database.connection.query_row(
                    "SELECT manifest_json, phase, rename_progress FROM archive_recovery_v2 WHERE job_id = 'job-1'",
                    [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                ).unwrap()
        };
        let before = journal_snapshot(&database);
        let staging = files.existing_attempt_staging_dir("job-1", 1).unwrap();
        let original_bytes = fs::read(staging.join("source.jpg")).unwrap();
        let candidate: String = database
            .connection
            .query_row(
                "SELECT metadata_json FROM archive_recovery_v2_candidate WHERE job_id = 'job-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        if mode == "missing" {
            database
                .execute_batch("DELETE FROM archive_recovery_v2_candidate WHERE job_id = 'job-1'")
                .unwrap();
        } else {
            let replacement = if mode == "corrupt" {
                "{".to_owned()
            } else {
                let mut value: serde_json::Value = serde_json::from_str(&candidate).unwrap();
                match mode {
                    "unsupported" => value["schema_version"] = serde_json::json!(999),
                    "identity" => value["tweet_id"] = serde_json::json!("999"),
                    "media" => value["media"][0]["file"] = serde_json::json!("other.jpg"),
                    _ => unreachable!(),
                }
                value.to_string()
            };
            // A matching digest must not bypass shape or manifest binding.
            database.connection.execute(
                    "UPDATE archive_recovery_v2_candidate SET metadata_json = ?1, metadata_sha256 = ?2 WHERE job_id = 'job-1'",
                    rusqlite::params![replacement, format!("{:x}", sha2::Sha256::digest(replacement.as_bytes()))],
                ).unwrap();
        }
        let stored: Option<String> = database
            .connection
            .query_row(
                "SELECT metadata_json FROM archive_recovery_v2_candidate WHERE job_id = 'job-1'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        let mut service = ArchiveService::new(database, files);
        assert!(
            service
                .recover_internal_rename_v2("job-1", tweet, Path::new("Tweets/123"), "later")
                .is_err(),
            "{mode}"
        );
        assert_eq!(journal_snapshot(&service.database), before, "{mode}");
        assert_eq!(
            fs::read(staging.join("source.jpg")).unwrap(),
            original_bytes,
            "{mode}"
        );
        assert!(!root.join("Tweets/123").exists(), "{mode}");
        let after: Option<String> = service
            .database
            .connection
            .query_row(
                "SELECT metadata_json FROM archive_recovery_v2_candidate WHERE job_id = 'job-1'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(after, stored, "{mode}");
        assert!(
            service
                .database
                .tweet_archive_facts("123")
                .unwrap()
                .is_none(),
            "{mode}"
        );
        drop(service);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn v2_rename_recovery_replays_planned_to_committed_from_disk() {
    let root = temp_root();
    let (database, files, tweet_row_id) = prepared_v2_rename_fixture(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    let destination = service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
        .expect("planned replay converges");
    assert_eq!(destination, root.join("Tweets/123"));
    assert_eq!(
        fs::read(destination.join("final.jpg")).expect("final media"),
        b"recoverable media payload"
    );
    assert!(!destination.join("source.jpg").exists());
    assert!(!destination.join(".rename-01.tmp").exists());
    let journal = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .expect("journal")
        .expect("record");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Committed);
    assert_eq!(journal.rename_progress, "FINAL");
    let before_claims = service.database.list_events_for_job("job-1", 100).unwrap();
    assert!(
        service
            .database
            .begin_archive_attempt("job-1", "forbidden")
            .is_err()
    );
    assert!(
        service
            .database
            .claim_archive_attempt(
                "job-1",
                journal.attempt_count,
                JobState::Downloaded,
                JobState::TgMediaUploading,
                "forbidden",
            )
            .is_err()
    );
    assert!(
        service
            .database
            .claim_archive_job(
                "job-1",
                journal.attempt_count,
                JobState::Downloaded,
                JobState::TgMediaUploading,
                "forbidden",
            )
            .is_err()
    );
    assert_eq!(
        service.database.archive_attempt("job-1").unwrap(),
        journal.attempt_count
    );
    assert_eq!(
        service.database.list_events_for_job("job-1", 100).unwrap(),
        before_claims
    );
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        JobState::Downloaded
    );
    let facts = service
        .database
        .tweet_archive_facts("123")
        .expect("facts")
        .expect("archive facts");
    assert_eq!(facts.archive_directory, "Tweets/123");
    assert_eq!(facts.media[0].relative_path, "final.jpg");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_is_idempotent_across_repeated_replays() {
    let root = temp_root();
    let (database, files, tweet_row_id) = prepared_v2_rename_fixture(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
        .expect("first replay");
    // A second replay after COMMITTED is read-only verified and converges to
    // the same committed directory without re-planning or re-renaming.
    let destination = service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later-2")
        .expect("committed replay is idempotent");
    assert_eq!(destination, root.join("Tweets/123"));
    assert_eq!(
        fs::read(destination.join("final.jpg")).expect("final media"),
        b"recoverable media payload"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_preserves_an_unchanged_media_filename() {
    let root = temp_root();
    let (database, files, _tweet_row_id, mut manifest, candidate) =
        unprepared_v2_publish_inputs(&root, "job-unchanged-name");
    manifest.rename_plan[0].final_path = manifest.rename_plan[0].source_path.clone();
    manifest.media[0].relative_path = manifest.rename_plan[0].source_path.clone();
    let mut candidate = candidate;
    candidate.media[0].file = manifest.rename_plan[0].source_path.clone();
    let mut service = ArchiveService::new(database, files);
    let staging = service
        .files
        .existing_attempt_staging_dir("job-unchanged-name", 1)
        .expect("attempt staging");
    let source_before = staging.join(&manifest.rename_plan[0].source_path);

    manifest.validate().expect("unchanged name plan validates");
    let archive = service
        .publish_prepared_internal_rename_v2_for_test(
            &manifest,
            &candidate,
            Path::new("Tweets/123"),
            "now",
            None,
        )
        .expect("two-phase publish returns media to its original name");

    let source_after = archive.join(&manifest.rename_plan[0].source_path);
    assert!(!source_before.exists(), "attempt staging is committed");
    assert_eq!(
        fs::read(source_after).expect("unchanged final media"),
        b"recoverable media payload"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_manifest_rejects_media_path_without_matching_rename_plan() {
    let root = temp_root();
    let (_database, _files, _tweet_row_id, mut manifest, _candidate) =
        unprepared_v2_publish_inputs(&root, "job-unchanged-mismatch");
    manifest.media[0].relative_path = "unplanned.jpg".into();
    assert!(manifest.validate().is_err());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_unchanged_filename_recovery_converges_across_temporary_phase_crash() {
    let root = temp_root();
    let (database, files, tweet_row_id, mut manifest, mut candidate) =
        unprepared_v2_publish_inputs(&root, "job-unchanged-replay");
    manifest.rename_plan[0].final_path = manifest.rename_plan[0].source_path.clone();
    manifest.media[0].relative_path = manifest.rename_plan[0].source_path.clone();
    candidate.media[0].file = manifest.rename_plan[0].source_path.clone();
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                Some(ArchiveRecoveryFailpoint::AfterTemporaryRename(1)),
            )
            .is_err()
    );

    let archive = service
        .recover_internal_rename_v2(
            "job-unchanged-replay",
            tweet_row_id,
            Path::new("Tweets/123"),
            "later",
        )
        .expect("replay completes the second edge and finalizes");
    assert_eq!(
        fs::read(archive.join(&manifest.rename_plan[0].source_path))
            .expect("replayed unchanged media"),
        b"recoverable media payload"
    );
    assert!(!archive.join(".rename-01.tmp").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_converges_when_files_moved_but_journal_lagged() {
    // Simulate a crash after the whole-phase filesystem rename completed but
    // before the journal advanced past PLANNED. The replay must observe the
    // moved temporary files and still advance the journal to COMMITTED.
    let root = temp_root();
    let (database, files, tweet_row_id) = prepared_v2_rename_fixture(&root, "job-1");
    let staging = files
        .existing_attempt_staging_dir("job-1", 1)
        .expect("attempt staging");
    fs::rename(staging.join("source.jpg"), staging.join(".rename-01.tmp"))
        .expect("simulate completed temporary rename");
    let mut service = ArchiveService::new(database, files);
    let destination = service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
        .expect("lagging journal converges");
    assert_eq!(
        fs::read(destination.join("final.jpg")).expect("final media"),
        b"recoverable media payload"
    );
    let journal = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .expect("journal")
        .expect("record");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Committed);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_rejects_a_mismatched_target_without_finalizing() {
    let root = temp_root();
    let (database, files, tweet_row_id) = prepared_v2_rename_fixture(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/other"), "later")
            .is_err()
    );
    // A rejected target must not finalize: the journal stays PREPARED and no
    // archive facts are written.
    let journal = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .expect("journal")
        .expect("record");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Prepared);
    assert_eq!(
        service.database.tweet_archive_facts("123").expect("facts"),
        None
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_fails_closed_on_a_stale_attempt() {
    let root = temp_root();
    let (database, files, tweet_row_id) = prepared_v2_rename_fixture(&root, "job-1");
    // A competing retry advances the durable attempt after the journal was
    // prepared; recovery must refuse rather than finalize a stale attempt.
    database
        .execute_batch("UPDATE jobs SET attempt_count = attempt_count + 1 WHERE id = 'job-1'")
        .expect("advance attempt");
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
            .is_err()
    );
    assert_eq!(
        service.database.tweet_archive_facts("123").expect("facts"),
        None
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_fails_closed_when_the_journal_is_missing() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            1,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
            "now",
        )
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    let mut service = ArchiveService::new(database, files);
    assert!(matches!(
        service.recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later"),
        Err(StorageError::InvalidState(_))
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_finalizes_when_committed_directory_exists_but_journal_lagged() {
    // Simulate a crash after the atomic directory commit (attempt staging
    // moved to Tweets/123) but before the finalize transaction. On replay
    // the durable directory exists while the journal is still PREPARED/FINAL
    // and the attempt staging tree is gone; recovery must verify the
    // committed media and finalize without re-planning or re-renaming.
    let root = temp_root();
    let (database, files, tweet_row_id) = prepared_v2_rename_fixture(&root, "job-1");
    let staging = files
        .existing_attempt_staging_dir("job-1", 1)
        .expect("attempt staging");
    // Advance the journal to FINAL exactly as the production rename phases
    // would, then place the final media and commit the staging tree. This is
    // the real crash window: directory committed, journal FINAL/PREPARED,
    // finalize transaction not yet run.
    database
        .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "now")
        .expect("temporary");
    database
        .transition_archive_recovery_v2("job-1", "TEMPORARY", "FINAL", "now")
        .expect("final");
    fs::write(staging.join("final.jpg"), b"recoverable media payload").expect("place final media");
    files
        .commit_attempt_staging("job-1", 1, Path::new("Tweets/123"))
        .expect("simulate committed directory");
    // Restart: drop the original handles and reopen both the durable
    // journal and the file store from disk, proving recovery does not rely
    // on any in-process state left over from the crashed run.
    drop(database);
    drop(files);
    let database = Database::open(root.join("archive.sqlite3")).expect("reopen database");
    let files = FileStore::new(&root).expect("reopen files");
    let mut service = ArchiveService::new(database, files);
    let destination = service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
        .expect("committed-but-unfinalized replay converges");
    assert_eq!(destination, root.join("Tweets/123"));
    assert_eq!(
        fs::read(destination.join("final.jpg")).expect("final media"),
        b"recoverable media payload"
    );
    let journal = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .expect("journal")
        .expect("record");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Committed);
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        JobState::Downloaded
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_rename_recovery_rejects_a_committed_directory_with_swapped_media() {
    // A committed directory whose media no longer matches the immutable
    // manifest facts must fail closed rather than finalize corrupt facts.
    let root = temp_root();
    let (database, files, tweet_row_id) = prepared_v2_rename_fixture(&root, "job-1");
    let staging = files
        .existing_attempt_staging_dir("job-1", 1)
        .expect("attempt staging");
    database
        .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "now")
        .expect("temporary");
    database
        .transition_archive_recovery_v2("job-1", "TEMPORARY", "FINAL", "now")
        .expect("final");
    fs::write(staging.join("final.jpg"), b"tampered payload").expect("place swapped media");
    files
        .commit_attempt_staging("job-1", 1, Path::new("Tweets/123"))
        .expect("simulate committed directory");
    let mut service = ArchiveService::new(database, files);
    assert!(matches!(
        service.recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later"),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert_eq!(
        service.database.tweet_archive_facts("123").expect("facts"),
        None
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_download_event_failure_rolls_back_publication_facts_and_replays_once() {
    let root = temp_root();
    let (database, files, tweet, manifest, candidate) =
        unprepared_v2_publish_inputs(&root, "job-1");
    database.execute_batch("CREATE TRIGGER reject_download BEFORE INSERT ON events WHEN NEW.event_type = 'DOWNLOAD_COMPLETED' BEGIN SELECT RAISE(ABORT, 'fixture event failure'); END;").unwrap();
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                None,
            )
            .is_err()
    );
    assert_eq!(
        service.database.job_state("job-1").unwrap(),
        JobState::Downloading
    );
    assert!(
        service
            .database
            .tweet_archive_facts("123")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        service
            .database
            .archive_recovery_manifest_v2("job-1")
            .unwrap()
            .unwrap()
            .phase,
        ArchiveRecoveryPhase::Prepared
    );
    assert!(
        service
            .database
            .list_events_for_job("job-1", 100)
            .unwrap()
            .iter()
            .all(|event| event.event_type != "DOWNLOAD_COMPLETED")
    );
    service
        .database
        .execute_batch("DROP TRIGGER reject_download")
        .unwrap();
    assert_eq!(
        service
            .database
            .archive_recovery_v2_candidate("job-1", manifest.attempt_count, manifest.tweet_row_id)
            .unwrap(),
        Some(candidate.clone()),
        "failed finalization retains the exact prepared business candidate"
    );
    for _ in 0..2 {
        service
            .recover_internal_rename_v2("job-1", tweet, Path::new("Tweets/123"), "later")
            .unwrap();
    }
    assert!(
        service
            .database
            .archive_recovery_v2_candidate("job-1", manifest.attempt_count, manifest.tweet_row_id)
            .unwrap()
            .is_none()
    );
    let before_terminal = service.database.list_events_for_job("job-1", 100).unwrap();
    let committed = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .unwrap();
    let facts = service.database.tweet_archive_facts("123").unwrap();
    service.database.execute_batch("CREATE TRIGGER reject_terminal BEFORE INSERT ON events WHEN NEW.event_type = 'JOB_COMPLETED' BEGIN SELECT RAISE(ABORT, 'fixture terminal failure'); END;").unwrap();
    assert!(
        service
            .database
            .complete_terminal_for_attempt(
                "job-1",
                manifest.attempt_count,
                "Tweets/123",
                1,
                "failed-terminal"
            )
            .is_err()
    );
    assert_eq!(
        service.database.job_state("job-1").unwrap(),
        JobState::Downloaded
    );
    assert_eq!(
        service.database.list_events_for_job("job-1", 100).unwrap(),
        before_terminal
    );
    assert_eq!(
        service
            .database
            .archive_recovery_manifest_v2("job-1")
            .unwrap(),
        committed
    );
    assert_eq!(service.database.tweet_archive_facts("123").unwrap(), facts);
    // Reopen durable state, just as startup does after the failed terminal
    // transaction. COMMITTED verification must not add another download.
    service
        .database
        .execute_batch("DROP TRIGGER reject_terminal")
        .unwrap();
    drop(service);
    let mut service = ArchiveService::new(
        Database::open(root.join("archive.sqlite3")).unwrap(),
        FileStore::new(&root).unwrap(),
    );
    service
        .recover_internal_rename_v2("job-1", tweet, Path::new("Tweets/123"), "restart")
        .unwrap();
    assert_eq!(
        service.database.list_events_for_job("job-1", 100).unwrap(),
        before_terminal
    );
    service
        .database
        .complete_terminal_for_attempt("job-1", manifest.attempt_count, "Tweets/123", 1, "done")
        .unwrap();
    let events = service.database.list_events_for_job("job-1", 100).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "DOWNLOAD_COMPLETED")
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "JOB_COMPLETED")
            .count(),
        1
    );
    // Historical v2 COMMITTED rows predate atomic download events. Keep
    // those readable and backfill only when Desktop completion is pending.
    service.database.execute_batch("DELETE FROM events WHERE job_id = 'job-1' AND (event_type IN ('DOWNLOAD_COMPLETED','JOB_COMPLETED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state = 'COMPLETE')); UPDATE jobs SET state = 'DOWNLOADED' WHERE id = 'job-1';").unwrap();
    assert!(
        service
            .database
            .complete_terminal_for_attempt(
                "job-1",
                manifest.attempt_count,
                "Tweets/123",
                1,
                "historical"
            )
            .unwrap()
    );
    assert!(
        !service
            .database
            .complete_terminal_for_attempt(
                "job-1",
                manifest.attempt_count,
                "Tweets/123",
                1,
                "repeat"
            )
            .unwrap()
    );
    let events = service.database.list_events_for_job("job-1", 100).unwrap();
    for kind in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == kind)
                .count(),
            1
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_export_collision_refuses_before_prepared_or_publication() {
    let root = temp_root();
    let (database, files, _, manifest, candidate) = unprepared_v2_publish_inputs(&root, "job-1");
    let staging = files
        .existing_attempt_staging_dir("job-1", manifest.attempt_count)
        .unwrap();
    fs::write(staging.join("tweet.json"), b"untrusted existing export").unwrap();
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                None,
            )
            .is_err()
    );
    assert!(
        service
            .database
            .archive_recovery_manifest_v2("job-1")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fs::read(staging.join("tweet.json")).unwrap(),
        b"untrusted existing export"
    );
    assert!(staging.join("source.jpg").is_file());
    assert!(!root.join("Tweets/123").exists());
    assert_eq!(
        service.database.job_state("job-1").unwrap(),
        JobState::Downloading
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn v2_historical_export_switches_do_not_control_internal_recovery() {
    for (json, text) in [(false, false), (false, true), (true, false), (true, true)] {
        let root = temp_root();
        let (database, files, tweet, mut manifest, candidate) =
            unprepared_v2_publish_inputs(&root, "job-1");
        manifest.export_json = json;
        manifest.export_text = text;
        let mut service = ArchiveService::new(database, files);
        assert!(
            service
                .publish_prepared_internal_rename_v2_for_test(
                    &manifest,
                    &candidate,
                    Path::new("Tweets/123"),
                    "now",
                    Some(ArchiveRecoveryFailpoint::AfterTemporaryRename(0)),
                )
                .is_err()
        );
        assert_eq!(
            service
                .database
                .archive_recovery_v2_candidate(
                    "job-1",
                    manifest.attempt_count,
                    manifest.tweet_row_id
                )
                .unwrap(),
            Some(candidate.clone()),
            "export switches never suppress the prepared business candidate"
        );
        let destination = service
            .recover_internal_rename_v2("job-1", tweet, Path::new("Tweets/123"), "retry")
            .unwrap();
        assert!(
            service
                .database
                .archive_recovery_v2_candidate(
                    "job-1",
                    manifest.attempt_count,
                    manifest.tweet_row_id
                )
                .unwrap()
                .is_none()
        );
        assert_eq!(destination.join("tweet.json").is_file(), json);
        assert_eq!(destination.join("tweet.txt").is_file(), text);
        assert!(!destination.join("tweet.yaml").exists());
        assert_eq!(
            service.database.job_state("job-1").unwrap(),
            JobState::Downloaded
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn v2_forward_publish_composes_prepare_rename_commit_and_fenced_finalize() {
    let root = temp_root();
    let (database, files, _, manifest, candidate) = unprepared_v2_publish_inputs(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    let destination = service
        .publish_prepared_internal_rename_v2_for_test(
            &manifest,
            &candidate,
            Path::new("Tweets/123"),
            "now",
            None,
        )
        .expect("forward publish converges");
    assert_eq!(destination, root.join("Tweets/123"));
    assert_eq!(
        fs::read(destination.join("final.jpg")).expect("final media"),
        b"recoverable media payload"
    );
    assert!(!destination.join("source.jpg").exists());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(destination.join("tweet.json")).unwrap()
        )
        .unwrap(),
        serde_json::to_value(&candidate).unwrap()
    );
    assert!(destination.join("tweet.txt").is_file());
    assert!(!destination.join(".rename-01.tmp").exists());
    let journal = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .expect("journal")
        .expect("record");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Committed);
    assert_eq!(journal.rename_progress, "FINAL");
    assert!(
        service
            .database
            .archive_recovery_v2_candidate("job-1", manifest.attempt_count, manifest.tweet_row_id)
            .expect("candidate lookup")
            .is_none(),
        "atomic finalization must retire the temporary business metadata copy"
    );
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        JobState::Downloaded
    );
    let facts = service
        .database
        .tweet_archive_facts("123")
        .expect("facts")
        .expect("archive facts");
    assert_eq!(facts.archive_directory, "Tweets/123");
    assert_eq!(facts.media[0].relative_path, "final.jpg");
    // Three fixture transitions plus DOWNLOADING->DOWNLOADED and the
    // download event committed atomically with archive facts/journal.
    assert_eq!(
        service.database.count_job_events("job-1").expect("events"),
        5
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn d1_optional_exports_publish_and_recover_without_user_files() {
    for export_json in [false, true] {
        for export_yaml in [false, true] {
            for crash in [false, true] {
                let root = temp_root();
                let (database, files, row_id, mut manifest, candidate) =
                    unprepared_v2_publish_inputs(&root, "job-1");
                manifest.export_json = export_json;
                manifest.export_text = false;
                manifest.metadata_output = Some(MetadataExportSelection {
                    schema_version: 4,
                    export_json,
                    export_yaml,
                });
                let bytes = serde_json::to_vec_pretty(&candidate).unwrap();
                let mut exports = Vec::new();
                if export_json {
                    exports.push(("tweet.json", bytes.clone()));
                }
                // JSON is also valid YAML; actual YAML serialization is covered
                // at the Desktop adapter, not duplicated in Storage.
                if export_yaml {
                    exports.push(("tweet.yaml", bytes.clone()));
                }
                let mut service = ArchiveService::new(database, files);
                let result = service.publish_prepared_internal_rename_v2_with_exports_for_test(
                    &manifest,
                    &candidate,
                    Path::new("Tweets/123"),
                    "now",
                    crash.then_some(ArchiveRecoveryFailpoint::AfterTemporaryRename(1)),
                    Some(&exports),
                );
                if !crash {
                    let destination = result.unwrap();
                    assert_eq!(destination.join("tweet.json").exists(), export_json);
                    assert_eq!(destination.join("tweet.yaml").exists(), export_yaml);
                    assert!(!destination.join("tweet.txt").exists());
                    fs::remove_dir_all(root).unwrap();
                    continue;
                }
                assert!(result.is_err());
                // Recovery must use internal facts even if all user exports
                // disappeared after PREPARED.
                let staging = service
                    .files
                    .existing_attempt_staging_dir("job-1", manifest.attempt_count)
                    .unwrap();
                for (name, _) in &exports {
                    assert_eq!(fs::read(staging.join(name)).unwrap(), bytes);
                    fs::remove_file(staging.join(name)).unwrap();
                }
                let destination = service
                    .recover_internal_rename_v2("job-1", row_id, Path::new("Tweets/123"), "later")
                    .unwrap();
                assert!(destination.join("final.jpg").is_file());
                assert!(!destination.join("tweet.txt").exists());
                assert_eq!(
                    service
                        .database
                        .archive_recovery_manifest_v2("job-1")
                        .unwrap()
                        .unwrap()
                        .phase,
                    ArchiveRecoveryPhase::Committed
                );
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
}

#[test]
fn v2_forward_publish_crash_windows_converge_through_recovery_replay() {
    const ROOT_ENV: &str = "TW2TG_P5_PUBLISH_CRASH_ROOT";
    const WINDOW_ENV: &str = "TW2TG_P5_PUBLISH_CRASH_WINDOW";
    if let Some(root) = std::env::var_os(ROOT_ENV) {
        let root = std::path::PathBuf::from(root);
        let window = std::env::var(WINDOW_ENV).unwrap();
        let failpoint = match window.as_str() {
            "temporary" => Some(ArchiveRecoveryFailpoint::AfterTemporaryRename(0)),
            "final" => Some(ArchiveRecoveryFailpoint::AfterFinalRename(0)),
            "directory" => Some(ArchiveRecoveryFailpoint::AfterDirectoryCommit),
            "unprepared" | "prepared" | "committed" => None,
            other => panic!("unknown crash window {other}"),
        };
        let (database, files, _, manifest, candidate) =
            unprepared_v2_publish_inputs(&root, "job-1");
        let mut service = ArchiveService::new(database, files);
        if window == "unprepared" {
            // Candidate bytes exist but no journal authorizes publication.
        } else if window == "prepared" {
            service
                .database
                .prepare_archive_recovery_v2(&manifest, &candidate, "now")
                .expect("persist PREPARED before crash");
        } else {
            let result = service.publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                failpoint,
            );
            if window == "committed" {
                result.expect("finalize before crash");
            } else {
                let error = result.expect_err("publish must stop at requested failpoint");
                assert!(error.to_string().contains("injected v2 publish crash"));
            }
        }
        fs::write(root.join("crash.ready"), manifest.tweet_row_id.to_string())
            .expect("record reached failpoint");
        // Do not run ArchiveService/SQLite destructors: recovery must use
        // only the files and transactions durable at the production seam.
        std::process::abort();
    }
    for window in [
        "unprepared",
        "prepared",
        "temporary",
        "final",
        "directory",
        "committed",
    ] {
        let root = temp_root();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::archive_service::v2_forward_publish_crash_windows_converge_through_recovery_replay",
                "--nocapture",
            ])
            .env(ROOT_ENV, &root)
            .env(WINDOW_ENV, window)
            .output()
            .expect("run isolated publish owner");
        assert!(!output.status.success(), "publish owner must crash");
        let tweet_row_id: i64 = fs::read_to_string(root.join("crash.ready"))
            .unwrap_or_else(|error| {
                panic!(
                    "{window}: failpoint was not reached: {error}; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
            })
            .parse()
            .unwrap();
        let database = Database::open(root.join("archive.sqlite3")).expect("reopen");
        let files = FileStore::new(&root).expect("reopen files");
        let mut service = ArchiveService::new(database, files);
        if window == "unprepared" {
            assert!(
                service
                    .recover_internal_rename_v2(
                        "job-1",
                        tweet_row_id,
                        Path::new("Tweets/123"),
                        "later"
                    )
                    .is_err()
            );
            assert!(!root.join("Tweets/123").exists());
            assert_eq!(
                service.database.job_state("job-1").expect("state"),
                JobState::Downloading
            );
            drop(service);
            let _ = fs::remove_dir_all(root);
            continue;
        }
        let destination = service
            .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
            .expect("crash window replays to committed");
        assert_eq!(destination, root.join("Tweets/123"));
        assert_eq!(
            fs::read(destination.join("final.jpg")).expect("final media"),
            b"recoverable media payload"
        );
        assert_eq!(
            service
                .database
                .archive_recovery_manifest_v2("job-1")
                .expect("journal")
                .expect("record")
                .phase,
            ArchiveRecoveryPhase::Committed
        );
        let event_count = service.database.count_job_events("job-1").expect("events");
        assert_eq!(
            service
                .recover_internal_rename_v2(
                    "job-1",
                    tweet_row_id,
                    Path::new("Tweets/123"),
                    "repeat"
                )
                .expect("repeat replay"),
            destination
        );
        assert_eq!(
            service.database.count_job_events("job-1").expect("events"),
            event_count
        );
        assert_eq!(
            fs::read(destination.join("final.jpg")).unwrap(),
            b"recoverable media payload"
        );
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn v2_committed_replay_rejects_swapped_bytes_without_mutation() {
    let root = temp_root();
    let (database, files, _, manifest, candidate) = unprepared_v2_publish_inputs(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    let destination = service
        .publish_prepared_internal_rename_v2_for_test(
            &manifest,
            &candidate,
            Path::new("Tweets/123"),
            "now",
            None,
        )
        .unwrap();
    let media = destination.join(&manifest.media[0].relative_path);
    fs::write(&media, vec![b'x'; manifest.media[0].size_bytes as usize]).unwrap();
    let before = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .unwrap()
        .unwrap();
    assert!(
        service
            .recover_internal_rename_v2(
                "job-1",
                manifest.tweet_row_id,
                Path::new("Tweets/123"),
                "later"
            )
            .is_err()
    );
    let after = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .unwrap()
        .unwrap();
    assert_eq!(before.manifest_json, after.manifest_json);
    assert_eq!(after.phase, ArchiveRecoveryPhase::Committed);
    assert_eq!(
        fs::read(media).unwrap(),
        vec![b'x'; manifest.media[0].size_bytes as usize]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_committed_replay_rejects_missing_database_facts() {
    let root = temp_root();
    let (database, files, _, manifest, candidate) = unprepared_v2_publish_inputs(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    service
        .publish_prepared_internal_rename_v2_for_test(
            &manifest,
            &candidate,
            Path::new("Tweets/123"),
            "now",
            None,
        )
        .unwrap();
    service
        .database
        .connection
        .execute(
            "DELETE FROM media WHERE tweet_id = ?1",
            [manifest.tweet_row_id],
        )
        .unwrap();
    assert!(
        service
            .recover_internal_rename_v2(
                "job-1",
                manifest.tweet_row_id,
                Path::new("Tweets/123"),
                "later"
            )
            .is_err()
    );
    assert_eq!(
        service
            .database
            .archive_recovery_manifest_v2("job-1")
            .unwrap()
            .unwrap()
            .phase,
        ArchiveRecoveryPhase::Committed
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_committed_replay_rejects_changed_database_facts_without_mutation() {
    for assignment in [
        "media_index = 2",
        "relative_path = 'other.jpg'",
        "x_media_id = 'other-media'",
        "media_type = 'video'",
        "mime_type = NULL",
        "size_bytes = NULL",
        "sha256 = NULL",
    ] {
        let root = temp_root();
        let (database, files, _, manifest, candidate) =
            unprepared_v2_publish_inputs(&root, "job-1");
        let mut service = ArchiveService::new(database, files);
        let destination = service
            .publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                None,
            )
            .unwrap();
        service
            .database
            .connection
            .execute(
                &format!("UPDATE media SET {assignment} WHERE tweet_id = ?1"),
                [manifest.tweet_row_id],
            )
            .unwrap();
        let before_facts = service.database.tweet_archive_facts("123").unwrap();
        let before_journal = service
            .database
            .archive_recovery_manifest_v2("job-1")
            .unwrap()
            .unwrap();
        let before_events = service.database.list_events_for_job("job-1", 100).unwrap();
        let media = destination.join(&manifest.media[0].relative_path);
        let before_bytes = fs::read(&media).unwrap();
        let error = service
            .recover_internal_rename_v2(
                "job-1",
                manifest.tweet_row_id,
                Path::new("Tweets/123"),
                "later",
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("database facts mismatch"),
            "{assignment}: {error}"
        );
        assert_eq!(
            service.database.tweet_archive_facts("123").unwrap(),
            before_facts,
            "{assignment}"
        );
        let after_journal = service
            .database
            .archive_recovery_manifest_v2("job-1")
            .unwrap()
            .unwrap();
        assert_eq!(after_journal.manifest_json, before_journal.manifest_json);
        assert_eq!(after_journal.phase, ArchiveRecoveryPhase::Committed);
        assert_eq!(
            service.database.list_events_for_job("job-1", 100).unwrap(),
            before_events
        );
        assert_eq!(fs::read(media).unwrap(), before_bytes);
        drop(service);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn v2_forward_publish_rejects_destination_collision_without_finalizing() {
    let root = temp_root();
    let (database, files, _, manifest, candidate) = unprepared_v2_publish_inputs(&root, "job-1");
    fs::create_dir_all(root.join("Tweets/123")).expect("occupy destination");
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                None,
            )
            .is_err(),
        "no-replace commit must refuse an occupied destination"
    );
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        JobState::Downloading
    );
    assert_eq!(
        service.database.tweet_archive_facts("123").expect("facts"),
        None
    );
    // Only the 3 fixture transitions exist: no DOWNLOADING->DOWNLOADED
    // terminal event was written.
    assert_eq!(
        service.database.count_job_events("job-1").expect("events"),
        3
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_forward_publish_fails_closed_on_a_stale_attempt() {
    let root = temp_root();
    let (database, files, _, manifest, candidate) = unprepared_v2_publish_inputs(&root, "job-1");
    database
        .execute_batch("UPDATE jobs SET attempt_count = attempt_count + 1 WHERE id = 'job-1'")
        .expect("advance attempt");
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                None,
            )
            .is_err(),
        "a stale attempt identity must not publish"
    );
    assert_eq!(
        service.database.tweet_archive_facts("123").expect("facts"),
        None
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_forward_publish_keeps_prepared_commit_winning_over_cancellation() {
    let root = temp_root();
    let (database, files, _, manifest, candidate) = unprepared_v2_publish_inputs(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    service
        .publish_prepared_internal_rename_v2_for_test(
            &manifest,
            &candidate,
            Path::new("Tweets/123"),
            "now",
            None,
        )
        .expect("publish commits");
    assert_eq!(
        service
            .database
            .decide_archive_cancellation("job-1", "later")
            .expect("arbitration"),
        ArchiveCancellationDecision::CommitWins
    );
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        JobState::Downloaded
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_forward_publish_rejects_a_rolled_back_candidate_without_finalizing() {
    let root = temp_root();
    let (database, files, _, manifest, mut candidate) =
        unprepared_v2_publish_inputs(&root, "job-1");
    candidate.media[0].file = "other.jpg".into();
    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                None,
            )
            .is_err()
    );
    assert_eq!(
        service
            .database
            .archive_recovery_manifest_v2("job-1")
            .expect("journal query"),
        None
    );
    assert_eq!(
        service.database.tweet_archive_facts("123").expect("facts"),
        None
    );
    assert!(!root.join("Tweets/123").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn v2_forward_publish_duplicate_completion_replays_read_only() {
    let root = temp_root();
    let (database, files, tweet_row_id, manifest, candidate) =
        unprepared_v2_publish_inputs(&root, "job-1");
    let mut service = ArchiveService::new(database, files);
    service
        .publish_prepared_internal_rename_v2_for_test(
            &manifest,
            &candidate,
            Path::new("Tweets/123"),
            "now",
            None,
        )
        .expect("first publish");
    let destination = service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
        .expect("duplicate completion converges read-only");
    assert_eq!(destination, root.join("Tweets/123"));
    assert_eq!(
        fs::read(destination.join("final.jpg")).expect("final media"),
        b"recoverable media payload"
    );
    let _ = fs::remove_dir_all(root);
}
