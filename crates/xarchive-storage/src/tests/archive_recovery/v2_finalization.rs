//! `archive_recovery` — v2 finalization coverage.

use super::*;

#[test]
fn v2_finalization_commits_tweet_media_job_intent_and_journal_atomically() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "t0")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            1,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
            "t0",
        )
        .expect("job");
    let attempt = database
        .begin_archive_attempt("job-1", "t1")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-1", state, "t1")
            .expect("active job state");
    }
    let media = xarchive_core::ArchiveMedia {
        index: 1,
        media_id: Some("media-1".into()),
        media_type: "photo".into(),
        file: "final.jpg".into(),
        mime_type: Some("image/jpeg".into()),
        size_bytes: 7,
        sha256: "a".repeat(64),
    };
    let metadata = xarchive_core::ArchiveMetadata {
        schema_version: 1,
        tweet_id: "123".into(),
        url: "https://x.com/a/status/123".into(),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: Some("a".into()),
            display_name: None,
        },
        created_at: None,
        text: "text".into(),
        media: vec![media],
        archived_at: "t2".into(),
        reply_to: None,
        quoted_tweet: None,
    };
    let manifest = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: "job-1".into(),
        attempt_count: attempt,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".xarchive-rename-00.tmp".into(),
            final_path: "final.jpg".into(),
            size_bytes: 7,
            sha256: "a".repeat(64),
        }],
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "final.jpg".into(),
            media_id: Some("media-1".into()),
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 7,
            sha256: "a".repeat(64),
        }],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    let candidate = test_candidate_metadata(
        "123",
        vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: Some("media-1".into()),
            media_type: "photo".into(),
            file: "final.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 7,
            sha256: "a".repeat(64),
        }],
    );
    database
        .prepare_archive_recovery_v2(&manifest, &candidate, "t1")
        .expect("prepare");
    database
        .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "t2")
        .expect("temporary");
    database
        .transition_archive_recovery_v2("job-1", "TEMPORARY", "FINAL", "t2")
        .expect("final");
    database
        .finalize_archive_recovery_v2("job-1", attempt, tweet_row_id, &metadata, &manifest, "t3")
        .expect("atomic finalization");

    let record = database
        .archive_recovery_manifest_v2("job-1")
        .expect("journal")
        .expect("record");
    assert_eq!(record.phase, ArchiveRecoveryPhase::Committed);
    assert_eq!(
        database.job_state("job-1").expect("job state"),
        xarchive_core::JobState::Downloaded
    );
    let facts = database
        .tweet_archive_facts("123")
        .expect("facts")
        .expect("archive facts");
    assert_eq!(facts.archive_directory, "Tweets/123");
    assert_eq!(facts.media[0].relative_path, "final.jpg");
}

#[test]
fn v2_finalization_rejects_stale_attempt_and_rolls_back_archive_facts() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "t0")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            1,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
            "t0",
        )
        .expect("job");
    let attempt = database
        .begin_archive_attempt("job-1", "t1")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-1", state, "t1")
            .expect("active state");
    }
    let media = xarchive_core::ArchiveMedia {
        index: 1,
        media_id: None,
        media_type: "photo".into(),
        file: "final.jpg".into(),
        mime_type: None,
        size_bytes: 1,
        sha256: "a".repeat(64),
    };
    let metadata = xarchive_core::ArchiveMetadata {
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
        text: "new".into(),
        media: vec![media],
        archived_at: "t2".into(),
        reply_to: None,
        quoted_tweet: None,
    };
    let manifest = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: "job-1".into(),
        attempt_count: attempt,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".temp.jpg".into(),
            final_path: "final.jpg".into(),
            size_bytes: 1,
            sha256: "a".repeat(64),
        }],
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "final.jpg".into(),
            media_id: None,
            media_type: "photo".into(),
            mime_type: None,
            size_bytes: 1,
            sha256: "a".repeat(64),
        }],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    let candidate = test_candidate_metadata(
        "123",
        vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: None,
            media_type: "photo".into(),
            file: "final.jpg".into(),
            mime_type: None,
            size_bytes: 1,
            sha256: "a".repeat(64),
        }],
    );
    database
        .prepare_archive_recovery_v2(&manifest, &candidate, "t1")
        .expect("prepare");
    database
        .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "t2")
        .expect("temporary");
    database
        .transition_archive_recovery_v2("job-1", "TEMPORARY", "FINAL", "t2")
        .expect("final");

    // Simulate a competing retry after journal preparation. The stale
    // attempt must not leave Tweet/media changes or commit the journal.
    database
        .execute_batch("UPDATE jobs SET attempt_count = attempt_count + 1 WHERE id = 'job-1'")
        .expect("advance attempt");
    assert!(
        database
            .finalize_archive_recovery_v2(
                "job-1",
                attempt,
                tweet_row_id,
                &metadata,
                &manifest,
                "t3",
            )
            .is_err()
    );
    assert_eq!(database.tweet_archive_facts("123").expect("facts"), None);
    let phase: String = database
        .connection
        .query_row(
            "SELECT phase FROM archive_recovery_v2 WHERE job_id = 'job-1'",
            [],
            |row| row.get(0),
        )
        .expect("journal phase");
    assert_eq!(phase, "PREPARED");
}

/// The legacy no-permit entry point refuses linked-intent recovery without
/// mutating the PREPARED journal, intent, or staging tree.
#[test]
fn legacy_recovery_refuses_linked_intent_without_mutation() {
    let root = std::env::temp_dir().join(format!(
        "xarchive-recovery-intent-linkage-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            2,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("job");
    database
        .begin_archive_attempt("job-1", "now")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-1", state, "now")
            .expect("active state");
    }
    let intent = crate::TelegramArchiveIntentRecord {
        job_id: "job-1".into(),
        tweet_row_id,
        archive_directory: "Tweets/123".into(),
        state: "PREPARED".into(),
        metadata_text: None,
        media_json: "[]".into(),
        bot_identity: Some("bot".into()),
        chat_id: Some("-100".into()),
        message_thread_id: None,
        config_revision: Some(1),
        plan_version: 1,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    database
        .prepare_telegram_archive_intent(&intent)
        .expect("prepare intent");
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
        text: "intent linkage".into(),
        media: vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: Some("m1".into()),
            media_type: "photo".into(),
            file: "01.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        }],
        archived_at: "now".into(),
        reply_to: None,
        quoted_tweet: None,
    };
    let manifest = ArchiveRecoveryManifest::from_metadata(
        "job-1",
        1,
        tweet_row_id,
        Path::new("Tweets/123"),
        &metadata,
        Some(tweet_row_id),
    )
    .expect("manifest with intent linkage");
    let files = crate::FileStore::new(&root).expect("files");
    let staging = files.staging_dir("job-1").expect("staging");
    std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
    std::fs::write(
        staging.join(".xarchive-recovery.json"),
        serde_json::to_vec(&manifest).expect("serialize manifest"),
    )
    .expect("manifest file");
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("journal");

    let mut service = crate::ArchiveService::new(database, files);
    assert!(
        service
            .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
            .is_err(),
        "legacy recovery must not bypass Workflow authorization"
    );
    assert_eq!(
        service
            .database
            .archive_recovery_journal("job-1")
            .expect("journal")
            .expect("journal row")
            .phase,
        ArchiveRecoveryPhase::Prepared,
    );
    assert_eq!(
        service
            .database
            .telegram_archive_intent(tweet_row_id)
            .expect("intent query")
            .expect("intent row")
            .state,
        "PREPARED",
    );
    assert!(root.join("_staging/job-1/01.jpg").is_file());
    assert!(
        service
            .database
            .verify_archive_recovery_manifest("job-1")
            .is_ok(),
        "stored manifest remains verifiable without replay authorization"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn crashing_between_manifest_writes_leaves_a_repeatable_journal_row() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            2,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("job");
    database
        .begin_archive_attempt("job-1", "now")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-1", state, "now")
            .expect("active state");
    }
    let manifest = ArchiveRecoveryManifest {
        schema_version: 1,
        job_id: "job-1".into(),
        attempt_count: 1,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "01.jpg".into(),
            media_id: Some("m1".into()),
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        telegram_tweet_row_id: None,
    };
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("first prepared write");
    // Crash/retry between journal writes is a no-op: the original row survives.
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("repeat prepared write");
    assert_eq!(
        database
            .verify_archive_recovery_manifest("job-1")
            .expect("verify"),
        manifest
    );
    assert_eq!(
        database
            .archive_recovery_journal("job-1")
            .expect("journal")
            .expect("journal row")
            .phase,
        ArchiveRecoveryPhase::Prepared
    );
    // A conflicting manifest never displaces the prepared row.
    let mut conflicted = manifest.clone();
    conflicted.archive_directory = "Tweets/999".into();
    assert!(
        database
            .create_archive_recovery_manifest(&conflicted, "now")
            .is_err()
    );
    assert_eq!(
        database
            .verify_archive_recovery_manifest("job-1")
            .expect("original survives"),
        manifest
    );
}

#[test]
fn post_rename_database_conflict_rolls_back_entire_recovery_transaction() {
    let root = std::env::temp_dir().join(format!(
        "xarchive-recovery-row-conflict-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            2,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("job");
    database
        .begin_archive_attempt("job-1", "now")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-1", state, "now")
            .expect("active state");
    }
    // A pre-existing media row with the same index but incompatible facts makes
    // authorized recovery fail. This legacy Storage entry point must refuse
    // before changing either the journal or filesystem.
    database
        .insert_media(
            tweet_row_id,
            &xarchive_core::ArchiveMedia {
                index: 1,
                media_id: Some("other".into()),
                media_type: "photo".into(),
                file: "01.jpg".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "b".repeat(64),
            },
            "now",
        )
        .expect("conflicting media row");
    let manifest = ArchiveRecoveryManifest {
        schema_version: 1,
        job_id: "job-1".into(),
        attempt_count: 1,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "01.jpg".into(),
            media_id: Some("m1".into()),
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            // sha256 of the fixture bytes b"abc" so media integrity passes and
            // the failure lands on the DB recovery boundary after rename.
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        }],
        telegram_tweet_row_id: None,
    };
    let files = crate::FileStore::new(&root).expect("files");
    let staging = files.staging_dir("job-1").expect("staging");
    std::fs::write(staging.join("01.jpg"), b"abc").expect("staging media");
    std::fs::write(
        staging.join(".xarchive-recovery.json"),
        serde_json::to_vec(&manifest).expect("manifest"),
    )
    .expect("manifest file");
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("journal");

    let mut service = crate::ArchiveService::new(database, files);
    assert!(
        service
            .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
            .is_err()
    );
    assert_eq!(
        service
            .database
            .archive_recovery_journal("job-1")
            .expect("journal")
            .expect("journal row")
            .phase,
        ArchiveRecoveryPhase::Prepared
    );
    assert!(
        service
            .database
            .tweet_archive_facts("123")
            .expect("facts query")
            .is_none()
    );
    assert_eq!(
        service.database.job_state("job-1").expect("job state"),
        xarchive_core::JobState::Downloading
    );
    // The renamed directory stays inspectable so a later restart can retry or
    // fail closed on the same facts instead of losing the commit evidence.
    assert!(root.join("_staging/job-1/01.jpg").is_file());
    assert!(!root.join("Tweets/123").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn legacy_internal_recovery_refuses_without_mutating_attempt() {
    let root = std::env::temp_dir().join(format!(
        "xarchive-legacy-recovery-refusal-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let database_path = root.join("database.sqlite3");
    std::fs::create_dir_all(&root).expect("fixture root");
    let mut database = Database::open(&database_path).expect("database");
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
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("InternalV1 job");
    let files = FileStore::with_staging_root(&root.join("archive"), &root.join("staging"))
        .expect("file store");
    let staging = files.staging_dir("job-1").expect("staging");
    std::fs::write(staging.join("01.jpg"), b"abc").expect("media");

    let mut service = ArchiveService::new(database, files);
    assert!(
        service
            .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
            .is_err()
    );
    assert_eq!(
        service.database.archive_attempt("job-1").expect("attempt"),
        0
    );
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        xarchive_core::JobState::Queued
    );
    assert!(!root.join("archive/Tweets/123").exists());
    assert!(root.join("staging/job-1/01.jpg").is_file());
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}
