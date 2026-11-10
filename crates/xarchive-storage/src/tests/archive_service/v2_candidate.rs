//! `archive_service` — v2 candidate coverage.

use super::*;

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
