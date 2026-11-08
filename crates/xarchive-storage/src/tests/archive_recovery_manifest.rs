//! Manifest contracts, rename plans and verification for archive recovery.

use super::*;
use sha2::Digest;

#[test]
fn manifest_is_strictly_versioned_and_contains_no_user_metadata() {
    let manifest = manifest("job-1", 1);
    manifest.validate().expect("valid manifest");
    let json = serde_json::to_value(&manifest).expect("json");
    assert!(json.get("text").is_none());
    assert!(json.get("author").is_none());
    let mut invalid = manifest;
    invalid.schema_version = 2;
    assert!(invalid.validate().is_err());
}
#[test]
fn rejects_traversal_duplicate_and_invalid_hash_media_facts() {
    let mut value = manifest("job-1", 1);
    value.media[0].relative_path = "../outside.jpg".into();
    assert!(value.validate().is_err());
    let mut value = manifest("job-1", 1);
    value.media[0].relative_path = r"..\outside.jpg".into();
    assert!(value.validate().is_err());
    let mut value = manifest("job-1", 1);
    value.media.push(value.media[0].clone());
    assert!(value.validate().is_err());
    let mut value = manifest("job-1", 1);
    value.media[0].relative_path = "media/File.jpg".into();
    value.media.push(ArchiveRecoveryMediaFact {
        relative_path: "media/file.jpg".into(),
        ..value.media[0].clone()
    });
    assert!(value.validate().is_err());
    let mut value = manifest("job-1", 1);
    value.media[0].sha256 = "xyz".into();
    assert!(value.validate().is_err());
}
#[test]
fn rejects_unknown_fields_in_manifest_wire_format() {
    let json = r#"{"schema_version":1,"job_id":"job-1","attempt_count":1,"tweet_row_id":1,"tweet_id":"123","archive_directory":"Tweets/123","media":[],"telegram_tweet_row_id":null,"text":"forbidden"}"#;
    assert!(serde_json::from_str::<ArchiveRecoveryManifest>(json).is_err());
}
#[test]
fn v2_rename_plan_rejects_path_escape_and_duplicate_temporary_names() {
    let fact = ArchiveRecoveryMediaFact {
        media_index: 1,
        relative_path: "final.jpg".into(),
        media_id: None,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
        size_bytes: 3,
        sha256: "a".repeat(64),
    };
    let base = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: "job-1".into(),
        attempt_count: 1,
        tweet_row_id: 1,
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
        media: vec![fact],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    base.validate().expect("valid v2 plan");
    let historical = serde_json::to_string(&base).unwrap();
    assert!(!historical.contains("metadata_output"));
    let restored: ArchiveRecoveryManifestV2 = serde_json::from_str(&historical).unwrap();
    assert_eq!(restored.digest().unwrap(), base.digest().unwrap());
    let mut d1 = base.clone();
    d1.export_text = false;
    d1.metadata_output = Some(MetadataExportSelection {
        schema_version: 4,
        export_json: true,
        export_yaml: false,
    });
    d1.validate().unwrap();
    let mut unsupported = d1.clone();
    unsupported.metadata_output.as_mut().unwrap().schema_version = 5;
    assert!(unsupported.validate().is_err());
    d1.export_text = true;
    assert!(d1.validate().is_err());
    let mut traversal = base.clone();
    traversal.rename_plan[0].temporary_path = "../outside".into();
    assert!(traversal.validate().is_err());
    let mut unchanged_name = base.clone();
    unchanged_name.rename_plan[0].source_path = "final.jpg".into();
    unchanged_name
        .validate()
        .expect("source and final may match via a distinct temporary path");
    let mut duplicate_temp = base.clone();
    duplicate_temp.rename_plan.push(ArchiveRenamePlan {
        source_path: "other.jpg".into(),
        temporary_path: ".xarchive-rename-01.tmp".into(),
        final_path: "other-final.jpg".into(),
        size_bytes: 3,
        sha256: "a".repeat(64),
    });
    duplicate_temp.media.push(ArchiveRecoveryMediaFact {
        media_index: 2,
        relative_path: "other-final.jpg".into(),
        media_id: None,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
        size_bytes: 3,
        sha256: "a".repeat(64),
    });
    assert!(duplicate_temp.validate().is_err());
    base.validate().expect("valid plan remains valid");
}
#[test]
fn v2_plan_rejects_cross_role_path_collisions_and_mismatched_media_facts() {
    let base = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: "job-1".into(),
        attempt_count: 1,
        tweet_row_id: 1,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".tmp.jpg".into(),
            final_path: "final.jpg".into(),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "final.jpg".into(),
            media_id: None,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    base.validate().expect("valid plan");

    let mut temp_collides_with_source = base.clone();
    temp_collides_with_source.rename_plan[0].temporary_path = "final.jpg".into();
    assert!(temp_collides_with_source.validate().is_err());

    let mut media_mismatch = base;
    media_mismatch.media[0].sha256 = "b".repeat(64);
    assert!(media_mismatch.validate().is_err());

    let mut case_alias = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: "job-2".into(),
        attempt_count: 1,
        tweet_row_id: 2,
        tweet_id: "456".into(),
        archive_directory: "Tweets/456".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![
            ArchiveRenamePlan {
                source_path: "source.jpg".into(),
                temporary_path: ".temp-1".into(),
                final_path: "final.jpg".into(),
                size_bytes: 3,
                sha256: "a".repeat(64),
            },
            ArchiveRenamePlan {
                source_path: "other.jpg".into(),
                temporary_path: ".temp-2".into(),
                final_path: "SOURCE.JPG".into(),
                size_bytes: 3,
                sha256: "a".repeat(64),
            },
        ],
        media: vec![
            ArchiveRecoveryMediaFact {
                media_index: 1,
                relative_path: "final.jpg".into(),
                media_id: None,
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "a".repeat(64),
            },
            ArchiveRecoveryMediaFact {
                media_index: 2,
                relative_path: "SOURCE.JPG".into(),
                media_id: None,
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "a".repeat(64),
            },
        ],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    assert!(case_alias.validate().is_err());
    case_alias.rename_plan[1].final_path = "other-final.jpg".into();
    case_alias.media[1].relative_path = "other-final.jpg".into();
    case_alias
        .validate()
        .expect("disjoint rename endpoint identities validate");
}
#[test]
fn v2_journal_verification_requires_intact_attempt_bound_candidate() {
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
    let attempt = 1;
    database
        .connection
        .execute(
            "UPDATE jobs SET attempt_count = ?1, state = 'DOWNLOADING' WHERE id = 'job-1'",
            [attempt],
        )
        .expect("establish active attempt state");
    let media_fact = ArchiveRecoveryMediaFact {
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
        attempt_count: attempt,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".rename.tmp".into(),
            final_path: "final.jpg".into(),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        media: vec![media_fact],
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
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
    );
    database
        .prepare_archive_recovery_v2(&manifest, &candidate, "now")
        .expect("prepare");
    assert!(
        database
            .archive_recovery_manifest_v2("job-1")
            .expect("verify intact journal")
            .is_some()
    );

    database
        .connection
        .execute(
            "UPDATE archive_recovery_v2_candidate SET metadata_json = '{}'
                 WHERE job_id = 'job-1'",
            [],
        )
        .expect("corrupt candidate payload");
    assert!(database.archive_recovery_manifest_v2("job-1").is_err());

    database
        .connection
        .execute(
            "UPDATE archive_recovery_v2_candidate SET metadata_json = ?1,
                 metadata_sha256 = ?2 WHERE job_id = 'job-1'",
            rusqlite::params![
                serde_json::to_string(&candidate).expect("candidate JSON"),
                format!(
                    "{:x}",
                    sha2::Sha256::digest(
                        serde_json::to_string(&candidate)
                            .expect("candidate JSON")
                            .as_bytes()
                    )
                )
            ],
        )
        .expect("restore intact candidate");
    database
        .connection
        .execute(
            "UPDATE archive_recovery_v2_candidate SET attempt_count = attempt_count + 1
                 WHERE job_id = 'job-1'",
            [],
        )
        .expect("corrupt candidate attempt");
    assert!(database.archive_recovery_manifest_v2("job-1").is_err());

    database
        .connection
        .execute(
            "DELETE FROM archive_recovery_v2_candidate WHERE job_id = 'job-1'",
            [],
        )
        .expect("remove candidate");
    assert!(database.archive_recovery_manifest_v2("job-1").is_err());
}
#[test]
fn recovery_rejects_missing_or_corrupt_internal_manifest_and_changed_media() {
    for damage in ["missing-manifest", "corrupt-manifest", "changed-media"] {
        let root = std::env::temp_dir().join(format!(
            "xarchive-recovery-reject-{}-{damage}",
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
        let files = crate::FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
        let metadata = xarchive_core::ArchiveMedia {
            index: 1,
            media_id: None,
            media_type: "photo".into(),
            file: "01.jpg".into(),
            mime_type: None,
            size_bytes: 3,
            sha256: crate::FileStore::sha256(staging.join("01.jpg")).expect("hash"),
        };
        let manifest = ArchiveRecoveryManifest {
            schema_version: 1,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            media: vec![ArchiveRecoveryMediaFact {
                media_index: metadata.index,
                relative_path: metadata.file.clone(),
                media_id: metadata.media_id.clone(),
                media_type: metadata.media_type.clone(),
                mime_type: metadata.mime_type.clone(),
                size_bytes: metadata.size_bytes,
                sha256: metadata.sha256.clone(),
            }],
            telegram_tweet_row_id: None,
        };
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("journal");
        if damage != "missing-manifest" {
            let contents = if damage == "corrupt-manifest" {
                b"{}".to_vec()
            } else {
                serde_json::to_vec(&manifest).expect("serialize")
            };
            std::fs::write(staging.join(".xarchive-recovery.json"), contents)
                .expect("manifest file");
        }
        if damage == "changed-media" {
            std::fs::write(staging.join("01.jpg"), b"xyz").expect("replace media");
        }
        let mut service = crate::ArchiveService::new(database, files);
        assert!(
            service
                .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
                .is_err(),
            "damage kind should fail: {damage}"
        );
        assert_eq!(
            service.database.job_state("job-1").expect("state"),
            xarchive_core::JobState::Downloading
        );
        assert!(!root.join("Tweets/123").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
#[test]
fn unsupported_recovery_contract_fails_closed_without_export_fallback() {
    let root = std::env::temp_dir().join(format!(
        "xarchive-recovery-unknown-contract-{}",
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
    database
        .connection
        .execute(
            "UPDATE jobs SET recovery_contract_version = 99 WHERE id = 'job-1'",
            [],
        )
        .expect("install unknown contract version");

    let files = crate::FileStore::new(&root).expect("files");
    let staging = files.staging_dir("job-1").expect("staging");
    std::fs::write(staging.join("tweet.json"), b"{}")
        .expect("legacy export exists to prove it is not used");
    std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
    let mut service = crate::ArchiveService::new(database, files);

    let error = service
        .recover_staging_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
        .expect_err("unknown contract must not recover through exported metadata");
    assert!(
        error
            .to_string()
            .contains("unsupported archive recovery contract")
    );
    assert_eq!(
        service
            .database
            .job_state("job-1")
            .expect("state unchanged"),
        xarchive_core::JobState::Queued
    );
    assert!(!root.join("Tweets/123").exists());
    assert!(staging.join("01.jpg").exists());
    let _ = std::fs::remove_dir_all(root);
}
#[test]
fn manifest_journal_round_trips_and_rejects_corruption() {
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
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        database
            .transition_job("job-1", state, "now")
            .expect("active state");
    }
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
        text: "must not be in manifest".into(),
        media: vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: Some("m1".into()),
            media_type: "photo".into(),
            file: "01.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
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
        None,
    )
    .expect("manifest");
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("journal");
    assert_eq!(
        database
            .verify_archive_recovery_manifest("job-1")
            .expect("verify"),
        manifest
    );
    assert!(
        !serde_json::to_string(&manifest)
            .expect("serialize")
            .contains("must not be in manifest")
    );
    database
        .connection
        .execute(
            "UPDATE archive_recovery_journal SET manifest_json = '{}' WHERE job_id = 'job-1'",
            [],
        )
        .expect("corrupt fixture");
    assert!(database.verify_archive_recovery_manifest("job-1").is_err());
}
