//! `archive_service` — v2 rename recovery coverage.

use super::*;

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
