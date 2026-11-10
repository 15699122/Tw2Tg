//! `archive_service` — v2 forward publish coverage.

use super::*;

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
