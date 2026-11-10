//! `archive_recovery` — v2 committed coverage.

use super::*;

#[test]
fn recover_internal_rename_v2_committed_is_read_only() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
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
    database
        .begin_archive_attempt("job-1", "now")
        .expect("attempt");
    database
        .transition_job("job-1", JobState::Validating, "now")
        .expect("validate");
    database
        .transition_job("job-1", JobState::MetadataReady, "now")
        .expect("metadata ready");
    database
        .transition_job("job-1", JobState::Downloading, "now")
        .expect("downloading");

    let media = ArchiveRecoveryMediaFact {
        media_index: 1,
        relative_path: "final.jpg".into(),
        media_id: None,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
        size_bytes: 3,
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
    };
    let manifest = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: "job-1".into(),
        attempt_count: 1,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".xarchive-rename-01.tmp".into(),
            final_path: "final.jpg".into(),
            size_bytes: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        }],
        media: vec![media.clone()],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    let metadata = ArchiveMetadata {
        schema_version: 1,
        tweet_id: "123".into(),
        url: "https://x.com/a/status/123".into(),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: None,
            display_name: None,
        },
        created_at: None,
        text: "recovery candidate".into(),
        media: vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: None,
            media_type: "photo".into(),
            file: "final.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        }],
        archived_at: "now".into(),
        reply_to: None,
        quoted_tweet: None,
    };

    let _ = database
        .prepare_archive_recovery_v2(&manifest, &metadata, "now")
        .expect("prepare v2");
    database
        .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "t1")
        .expect("temporary phase");
    database
        .transition_archive_recovery_v2("job-1", "TEMPORARY", "FINAL", "t2")
        .expect("final phase");
    database
        .finalize_archive_recovery_v2("job-1", 1, tweet_row_id, &metadata, &manifest, "t3")
        .expect("commit facts and phase atomically");

    let root = std::env::temp_dir().join(format!(
        "recover-committed-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("root");
    let archive_dir = root.join("archive");
    let staging_dir = root.join("staging");
    let files = FileStore::with_staging_root(&archive_dir, &staging_dir).expect("file store");
    let committed_dir = archive_dir.join("Tweets").join("123");
    std::fs::create_dir_all(&committed_dir).expect("committed dir");
    std::fs::write(committed_dir.join("final.jpg"), b"abc").expect("committed file");

    let mut service = ArchiveService::new(database, files);
    let result = service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "now")
        .expect("committed recovery is read-only");
    assert_eq!(result, archive_dir.join("Tweets/123"));

    let journal = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .expect("read journal")
        .expect("journal");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Committed);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn recover_internal_rename_v2_rejects_identity_mismatch() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    let wrong_tweet_row_id = database
        .insert_tweet("999", "https://x.com/a/status/999", "post", "", "now")
        .expect("wrong tweet");
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
    database
        .begin_archive_attempt("job-1", "now")
        .expect("attempt");
    database
        .transition_job("job-1", JobState::Validating, "now")
        .expect("validate");
    database
        .transition_job("job-1", JobState::MetadataReady, "now")
        .expect("metadata ready");
    database
        .transition_job("job-1", JobState::Downloading, "now")
        .expect("downloading");

    let media = ArchiveRecoveryMediaFact {
        media_index: 1,
        relative_path: "final.jpg".into(),
        media_id: None,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
        size_bytes: 3,
        sha256: "a".repeat(64),
    };
    let manifest = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: "job-1".into(),
        attempt_count: 1,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".xarchive-rename-01.tmp".into(),
            final_path: "final.jpg".into(),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        media: vec![media.clone()],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    let metadata = ArchiveMetadata {
        schema_version: 1,
        tweet_id: "123".into(),
        url: "https://x.com/a/status/123".into(),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: None,
            display_name: None,
        },
        created_at: None,
        text: "recovery candidate".into(),
        media: vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: None,
            media_type: "photo".into(),
            file: "final.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        archived_at: "now".into(),
        reply_to: None,
        quoted_tweet: None,
    };

    let _ = database
        .prepare_archive_recovery_v2(&manifest, &metadata, "now")
        .expect("prepare v2");

    let root = std::env::temp_dir().join(format!(
        "recover-mismatch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("root");
    let archive_dir = root.join("archive");
    let staging_dir = root.join("staging");
    let files = FileStore::with_staging_root(&archive_dir, &staging_dir).expect("file store");

    let mut service = ArchiveService::new(database, files);
    let err = service
        .recover_internal_rename_v2("job-1", wrong_tweet_row_id, Path::new("Tweets/123"), "now")
        .expect_err("mismatched tweet_row_id must fail");
    assert!(
        err.to_string()
            .contains("v2 recovery target does not match the journal")
    );
    let err = service
        .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/999"), "now")
        .expect_err("mismatched archive directory must fail");
    assert!(
        err.to_string()
            .contains("v2 recovery target does not match the journal")
    );

    let journal = service
        .database
        .archive_recovery_manifest_v2("job-1")
        .expect("read journal")
        .expect("journal");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Prepared);
    std::fs::remove_dir_all(root).expect("cleanup");
}
