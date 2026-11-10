//! `archive_recovery` — dispatch coverage.

use super::*;

#[test]
fn startup_dispatch_routes_internal_v1_and_fails_closed_after_downloaded() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "legacy",
            tweet_row_id,
            1,
            "legacy-request",
            "{}",
            None,
            "now",
        )
        .expect("legacy job");
    assert_eq!(
        database
            .startup_recovery_dispatch("legacy")
            .expect("legacy"),
        ArchiveRecoveryDispatch::Legacy
    );

    let v1_tweet_row_id = database
        .insert_tweet("124", "https://x.com/a/status/124", "post", "", "now")
        .expect("v1 tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "v1",
            v1_tweet_row_id,
            1,
            "v1-request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("v1 job");
    assert_eq!(
        database.startup_recovery_dispatch("v1").unwrap(),
        ArchiveRecoveryDispatch::InternalV1,
        "a queued InternalV1 job with its durable spec may enter normal execution",
    );
    for (state_index, state) in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
        xarchive_core::JobState::Interrupted,
    ]
    .into_iter()
    .enumerate()
    {
        let job_id = format!("v1-no-journal-{state_index}");
        let tweet_id = format!("v1-no-journal-tweet-{state_index}");
        let row_id = database
            .insert_tweet(
                &tweet_id,
                &format!("https://x.com/a/status/{tweet_id}"),
                "post",
                "",
                "now",
            )
            .expect("matrix tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                &job_id,
                row_id,
                1,
                &format!("request-{job_id}"),
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("matrix job");
        if state != xarchive_core::JobState::Queued {
            database
                .begin_archive_attempt(&job_id, "now")
                .expect("matrix attempt");
            for step in [
                xarchive_core::JobState::Validating,
                xarchive_core::JobState::MetadataReady,
                xarchive_core::JobState::Downloading,
            ] {
                if state == step {
                    break;
                }
                database
                    .transition_job(&job_id, step, "now")
                    .expect("matrix transition");
            }
            if state == xarchive_core::JobState::Interrupted {
                database
                    .connection
                    .execute(
                        "UPDATE jobs SET state = 'INTERRUPTED' WHERE id = ?1",
                        [&job_id],
                    )
                    .expect("interrupted fixture");
            } else if state != xarchive_core::JobState::Validating
                && state != xarchive_core::JobState::MetadataReady
                && state != xarchive_core::JobState::Downloading
            {
                database
                    .transition_job(&job_id, state, "now")
                    .expect("requested matrix state");
            }
        }
        assert_eq!(
            database.startup_recovery_dispatch(&job_id).unwrap(),
            ArchiveRecoveryDispatch::InternalV1,
            "pre-download state {state:?} without journal should use session execution"
        );
    }
    assert!(
        database
            .create_archive_recovery_manifest(
                &ArchiveRecoveryManifest {
                    job_id: "v1".into(),
                    ..manifest("v1", 1)
                },
                "now"
            )
            .is_err(),
        "attempt fencing rejects a journal before the attempt begins"
    );
    let no_journal_downloading = database
        .insert_tweet("125", "https://x.com/a/status/125", "post", "", "now")
        .expect("downloading tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "no-journal-downloading",
            no_journal_downloading,
            1,
            "no-journal-downloading-request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("downloading v1 job");
    database
        .begin_archive_attempt("no-journal-downloading", "now")
        .expect("downloading attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("no-journal-downloading", state, "now")
            .expect("downloading state");
    }
    assert_eq!(
        database
            .startup_recovery_dispatch("no-journal-downloading")
            .expect("no-journal downloading is an explicit takeover candidate"),
        ArchiveRecoveryDispatch::InternalV1
    );
    database
        .begin_archive_attempt("v1", "now")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("v1", state, "now")
            .expect("active state");
    }
    let mut v1_base_manifest = manifest("v1", 1);
    v1_base_manifest.tweet_id = "124".into();
    v1_base_manifest.tweet_row_id = v1_tweet_row_id;
    v1_base_manifest.archive_directory = "Tweets/124".into();
    let v1_manifest = ArchiveRecoveryManifest {
        job_id: "v1".into(),
        ..v1_base_manifest
    };
    database
        .create_archive_recovery_manifest(&v1_manifest, "now")
        .expect("v1 journal");
    assert_eq!(
        database
            .startup_recovery_dispatch("v1")
            .expect("v1 dispatch"),
        ArchiveRecoveryDispatch::InternalV1
    );

    let downloaded_tweet = database
        .insert_tweet("126", "https://x.com/a/status/126", "post", "", "now")
        .expect("downloaded tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "downloaded-v1",
            downloaded_tweet,
            1,
            "downloaded-v1-request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("downloaded v1 job");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
        xarchive_core::JobState::Downloaded,
    ] {
        database
            .transition_job("downloaded-v1", state, "now")
            .expect("downloaded state");
    }
    assert!(database.startup_recovery_dispatch("downloaded-v1").is_err());
    assert_eq!(
        database
            .job_state("downloaded-v1")
            .expect("startup dispatch is read-only"),
        xarchive_core::JobState::Downloaded,
        "dispatch must not silently alter durable admission or job state"
    );

    let changed = database
        .connection
        .execute(
            "UPDATE jobs SET recovery_contract_version = 99 WHERE id = 'legacy'",
            [],
        )
        .expect("future version");
    assert_eq!(changed, 1, "future contract fixture must be updated");
    assert_eq!(
        database
            .startup_recovery_dispatch("legacy")
            .expect("future dispatch"),
        ArchiveRecoveryDispatch::Unknown(99)
    );
}

#[test]
fn prepared_replay_is_atomic_under_sql_fault_and_terminal_convergence_is_idempotent() {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "xarchive-v1-replay-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    let archive_root = root.join("archive");
    let staging_root = root.join("staging");
    std::fs::create_dir_all(&root).expect("fixture root");
    let database_path = root.join("database.sqlite3");
    let result = (|| {
        let mut database = Database::open(&database_path).expect("database");
        let files = FileStore::with_staging_root(&archive_root, &staging_root)
            .expect("create isolated archive and staging roots");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet row");
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
        let attempt = database
            .claim_archive_attempt(
                "job-1",
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "now",
            )
            .expect("claim")
            .expect("winner");
        assert_eq!(attempt, 1);
        let attempt_staging = files
            .create_attempt_staging_dir("job-1", 1)
            .expect("fresh attempt staging");
        std::fs::create_dir_all(attempt_staging.join("media")).expect("media directory");
        for state in [
            xarchive_core::JobState::MetadataReady,
            xarchive_core::JobState::Downloading,
        ] {
            database
                .transition_job("job-1", state, "now")
                .expect("advance job");
        }

        let media = ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "media/01.jpg".into(),
            media_id: Some("media-1".into()),
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: format!("{:x}", sha2::Sha256::digest(b"img")),
        };
        std::fs::write(attempt_staging.join("media/01.jpg"), b"img").expect("media payload");
        let manifest = ArchiveRecoveryManifest {
            schema_version: 1,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            media: vec![media],
            telegram_tweet_row_id: None,
        };
        std::fs::write(
            attempt_staging.join(".xarchive-recovery.json"),
            serde_json::to_vec(&manifest).expect("manifest JSON"),
        )
        .expect("disk manifest");
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("PREPARED journal");
        drop(database);

        let workflow_attempt =
            xarchive_workflow::AttemptIdentity::new("job-1", 1).expect("workflow attempt");
        assert_ne!(
            workflow_attempt.job_id(),
            "123",
            "job identity and tweet identity are distinct"
        );
        let coordinator =
            xarchive_workflow::coordination::unix::LinuxCoordinator::new(root.join("locks"))
                .expect("coordinator");
        let destination = archive_root.join("Tweets/123");
        let session =
            xarchive_workflow::ExecutionReservation::acquire(&coordinator, "job-1", &destination)
                .expect("recovery locks")
                .bind_existing_attempt_checked(workflow_attempt.clone(), |bound| {
                    bound.attempt() == attempt
                })
                .expect("bind persisted attempt");
        let evidence =
            xarchive_workflow::linux_containment::LinuxContainment::recover_production_attempt(
                &root.join("containment-records"),
                session.recovery_drain_binding(&staging_root),
                std::time::Duration::from_secs(1),
            )
            .expect("no-record writer domains are safe to recover");
        assert_eq!(session.destination(), destination);
        assert_eq!(evidence.workspace(), staging_root);
        let permit = xarchive_workflow::authorize_prepared_recovery(
            &session,
            evidence,
            xarchive_workflow::RecoveryJournalIdentity::new(
                &workflow_attempt,
                &destination,
                Path::new("Tweets/123"),
                &staging_root,
                xarchive_workflow::RecoveryJournalPhase::Prepared,
            ),
        )
        .expect("authorize under both locks");

        let mut archive = ArchiveService::new(
            Database::open(&database_path).expect("reopen database"),
            files,
        );
        let archive_facts_before_failure = archive
            .database
            .tweet_archive_facts("123")
            .expect("archive facts before injected failure");
        archive
                .database
                .execute_batch("CREATE TRIGGER fail_v1_commit BEFORE UPDATE ON archive_recovery_journal WHEN NEW.job_id = 'job-1' AND NEW.phase = 'COMMITTED' BEGIN SELECT RAISE(ABORT, 'injected InternalV1 commit failure'); END;")
                .expect("install scoped post-directory-commit failure trigger");
        assert!(
            archive
                .replay_prepared_internal_v1(
                    "job-1",
                    tweet_row_id,
                    Path::new("Tweets/123"),
                    &workflow_attempt,
                    permit,
                )
                .is_err(),
            "the journal commit trigger must fail replay after publication"
        );
        let prepared_after_failure = archive
            .database
            .archive_recovery_journal("job-1")
            .expect("journal after injected failure")
            .expect("PREPARED record survives transaction rollback");
        assert_eq!(prepared_after_failure.phase, ArchiveRecoveryPhase::Prepared);
        assert!(destination.join("media/01.jpg").is_file());
        assert_eq!(
            archive
                .database
                .tweet_archive_facts("123")
                .expect("archive facts remain readable after trigger failure"),
            archive_facts_before_failure,
            "journal transaction failure must not partially commit archive facts"
        );
        assert_eq!(
            archive
                .database
                .archive_attempt("job-1")
                .expect("attempt after failure"),
            1,
            "failed replay must not advance the durable attempt"
        );
        assert_eq!(
            archive
                .database
                .job_state("job-1")
                .expect("state after failure"),
            xarchive_core::JobState::Downloading,
            "failed replay must roll back state and event writes with journal"
        );

        archive
            .database
            .execute_batch("DROP TRIGGER fail_v1_commit;")
            .expect("remove scoped failure trigger for replay");
        let retry_evidence =
            xarchive_workflow::linux_containment::LinuxContainment::recover_production_attempt(
                &root.join("containment-records"),
                session.recovery_drain_binding(&staging_root),
                std::time::Duration::from_secs(1),
            )
            .expect("retry re-verifies the recorded writer domains");
        let retry_permit = xarchive_workflow::authorize_prepared_recovery(
            &session,
            retry_evidence,
            xarchive_workflow::RecoveryJournalIdentity::new(
                &workflow_attempt,
                &destination,
                Path::new("Tweets/123"),
                &staging_root,
                xarchive_workflow::RecoveryJournalPhase::Prepared,
            ),
        )
        .expect("retry obtains a fresh permit for the same journaled attempt");
        let final_path = archive
            .replay_prepared_internal_v1(
                "job-1",
                tweet_row_id,
                Path::new("Tweets/123"),
                &workflow_attempt,
                retry_permit,
            )
            .expect("PREPARED replay converges after the database fault is removed");
        assert_eq!(final_path, destination);
        assert_eq!(
            archive
                .database
                .archive_recovery_journal("job-1")
                .expect("journal")
                .expect("record")
                .phase,
            ArchiveRecoveryPhase::Committed
        );
        assert_eq!(
            archive
                .database
                .job_state("job-1")
                .expect("replay leaves convergence to caller"),
            xarchive_core::JobState::Downloaded
        );

        assert!(
            archive
                .converge_committed_internal_v1(
                    "job-1",
                    &workflow_attempt,
                    Path::new("Tweets/123"),
                    "recovered",
                )
                .expect("terminal convergence")
        );
        assert_eq!(
            archive.database.job_state("job-1").expect("complete"),
            xarchive_core::JobState::Complete
        );
        let events_after_completion = archive
            .database
            .count_job_events("job-1")
            .expect("completion events");
        assert!(
            !archive
                .converge_committed_internal_v1(
                    "job-1",
                    &workflow_attempt,
                    Path::new("Tweets/123"),
                    "again",
                )
                .expect("idempotent terminal convergence")
        );
        assert_eq!(
            archive
                .database
                .count_job_events("job-1")
                .expect("stable events"),
            events_after_completion
        );
        assert_eq!(
            archive
                .database
                .archive_recovery_journal("job-1")
                .expect("journal")
                .expect("record")
                .phase,
            ArchiveRecoveryPhase::Committed
        );
        Ok::<(), Box<dyn std::error::Error>>(())
    })();
    let _ = std::fs::remove_dir_all(&root);
    result.expect("isolated PREPARED replay and terminal convergence");
}
