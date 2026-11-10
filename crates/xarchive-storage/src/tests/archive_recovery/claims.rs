//! `archive_recovery` — claims coverage.

use super::*;

#[test]
fn beginning_archive_attempt_rejects_terminal_jobs_without_incrementing_attempt() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");

    for (job_id, request_id, terminal_state) in [
        (
            "complete-job",
            "complete-request",
            xarchive_core::JobState::Complete,
        ),
        (
            "cancelled-job",
            "cancelled-request",
            xarchive_core::JobState::Cancelled,
        ),
    ] {
        database
            .create_archive_job_with_optional_recovery_contract(
                job_id,
                tweet_row_id,
                1,
                request_id,
                "{}",
                None,
                "now",
            )
            .expect("job");
        database
            .connection
            .execute(
                "UPDATE jobs SET state = ?1, attempt_count = 4 WHERE id = ?2",
                rusqlite::params![terminal_state.as_str(), job_id],
            )
            .expect("set terminal state");

        assert!(matches!(
            database.begin_archive_attempt(job_id, "later"),
            Err(StorageError::InvalidState(_))
        ));
        assert_eq!(
            database.archive_attempt(job_id).expect("attempt count"),
            4,
            "a terminal job must not be reactivated or have its attempt incremented"
        );
    }
}

#[test]
fn beginning_archive_attempt_fails_explicitly_on_overflow() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "overflow-job",
            tweet_row_id,
            1,
            "overflow-request",
            "{}",
            None,
            "now",
        )
        .expect("overflow job");
    database
        .connection
        .execute(
            "UPDATE jobs SET attempt_count = ?1 WHERE id = 'overflow-job'",
            [i64::from(u32::MAX)],
        )
        .expect("saturate attempt count");
    assert!(matches!(
        database.begin_archive_attempt("overflow-job", "later"),
        Err(StorageError::InvalidState(_))
    ));
    assert_eq!(
        database
            .archive_attempt("overflow-job")
            .expect("attempt count"),
        u32::MAX,
        "overflow must fail without wrapping the counter"
    );
}

#[test]
fn beginning_archive_attempt_refuses_to_bypass_a_prepared_journal() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "prepared-job",
            tweet_row_id,
            2,
            "prepared-request",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
            "now",
        )
        .expect("job");
    database
        .connection
        .execute(
            "UPDATE jobs SET attempt_count = 1, state = 'DOWNLOADING' WHERE id = 'prepared-job'",
            [],
        )
        .expect("active attempt");
    database
            .connection
            .execute(
                "INSERT INTO archive_recovery_v2 (job_id, attempt_count, tweet_row_id, archive_directory, manifest_json, manifest_sha256, rename_progress, phase, created_at, updated_at) VALUES ('prepared-job', 1, ?1, 'Tweets/123', '{}', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'PLANNED', 'PREPARED', 'now', 'now')",
                [tweet_row_id],
            )
            .expect("prepared journal");
    assert!(matches!(
        database.begin_archive_attempt("prepared-job", "later"),
        Err(StorageError::InvalidState(_))
    ));
    assert_eq!(
        database
            .archive_attempt("prepared-job")
            .expect("attempt count"),
        1,
        "a prepared journal must be recovered, not bypassed"
    );
}

#[test]
fn claiming_an_archive_job_is_atomic_against_state_and_attempt() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "claim-job",
            tweet_row_id,
            1,
            "claim-request",
            "{}",
            None,
            "now",
        )
        .expect("job");
    let events_before = database.count_job_events("claim-job").expect("events");
    assert!(
        database
            .claim_archive_job(
                "claim-job",
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "t1",
            )
            .expect("claim"),
        "the first claim wins the conditional state transition"
    );
    // A loser racing with a stale view observes the move and backs off.
    assert!(
        !database
            .claim_archive_job(
                "claim-job",
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "t2",
            )
            .expect("stale claim"),
        "a second claim against the same expected state must lose"
    );
    assert_eq!(
        database.job_state("claim-job").expect("state"),
        xarchive_core::JobState::Validating
    );
    assert_eq!(
        database.count_job_events("claim-job").expect("events"),
        events_before + 1,
        "exactly one claim writes the state-change event"
    );
}

#[test]
fn claiming_archive_attempt_atomically_advances_attempt_and_state_once() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("124", "https://x.com/a/status/124", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "attempt-claim-job",
            tweet_row_id,
            1,
            "attempt-claim-request",
            "{}",
            None,
            "now",
        )
        .expect("job");
    let events_before = database
        .count_job_events("attempt-claim-job")
        .expect("events");

    assert_eq!(
        database
            .claim_archive_attempt(
                "attempt-claim-job",
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "claimed",
            )
            .expect("claim"),
        Some(1)
    );
    assert_eq!(
        database
            .claim_archive_attempt(
                "attempt-claim-job",
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "stale",
            )
            .expect("stale claim"),
        None
    );
    assert_eq!(
        database
            .archive_attempt("attempt-claim-job")
            .expect("attempt"),
        1
    );
    assert_eq!(
        database.job_state("attempt-claim-job").expect("state"),
        xarchive_core::JobState::Validating
    );
    assert_eq!(
        database
            .count_job_events("attempt-claim-job")
            .expect("events"),
        events_before + 1
    );
}

#[test]
fn internal_v1_successor_claim_is_predecessor_cas_and_preserves_downloading() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet(
            "successor-123",
            "https://x.com/a/status/123",
            "post",
            "",
            "now",
        )
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "successor-job",
            tweet_row_id,
            1,
            "successor-request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("InternalV1 job");
    assert_eq!(
        database
            .claim_archive_attempt(
                "successor-job",
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "claim",
            )
            .expect("initial claim"),
        Some(1)
    );
    database
        .transition_job(
            "successor-job",
            xarchive_core::JobState::MetadataReady,
            "metadata",
        )
        .expect("metadata state");
    database
        .transition_job(
            "successor-job",
            xarchive_core::JobState::Downloading,
            "download",
        )
        .expect("downloading state");
    let events_before = database.count_job_events("successor-job").expect("events");

    assert_eq!(
        database
            .claim_internal_v1_successor("successor-job", 0, "stale")
            .expect("stale predecessor loses"),
        None
    );
    assert_eq!(database.archive_attempt("successor-job").unwrap(), 1);
    assert_eq!(
        database.job_state("successor-job").unwrap(),
        xarchive_core::JobState::Downloading
    );
    assert_eq!(
        database.count_job_events("successor-job").unwrap(),
        events_before
    );

    assert_eq!(
        database
            .claim_internal_v1_successor("successor-job", 1, "successor")
            .expect("successor CAS"),
        Some(2)
    );
    assert_eq!(database.archive_attempt("successor-job").unwrap(), 2);
    assert_eq!(
        database.job_state("successor-job").unwrap(),
        xarchive_core::JobState::Downloading
    );
    assert_eq!(
        database.count_job_events("successor-job").unwrap(),
        events_before,
        "attempt takeover must not synthesize a Job state transition"
    );
    assert_eq!(
        database
            .claim_internal_v1_successor("successor-job", 1, "duplicate")
            .expect("duplicate predecessor loses"),
        None
    );
    assert_eq!(database.archive_attempt("successor-job").unwrap(), 2);
}

#[test]
fn internal_v1_successor_claim_refuses_journaled_or_non_v1_jobs() {
    let mut database = Database::open_in_memory().expect("database");
    for (job_id, tweet_id, contract_request_id, contract) in [
        (
            "successor-journaled",
            "123",
            "successor-journaled-request",
            Some(ArchiveRecoveryContract::InternalV1),
        ),
        ("successor-legacy", "124", "successor-legacy-request", None),
    ] {
        let tweet_row_id = database
            .insert_tweet(tweet_id, "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                job_id,
                tweet_row_id,
                1,
                contract_request_id,
                "{}",
                contract,
                "now",
            )
            .expect("job");
        database
            .claim_archive_attempt(
                job_id,
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "claim",
            )
            .expect("claim");
        database
            .transition_job(job_id, xarchive_core::JobState::MetadataReady, "metadata")
            .expect("metadata");
        database
            .transition_job(job_id, xarchive_core::JobState::Downloading, "download")
            .expect("downloading");
    }
    let manifest = ArchiveRecoveryManifest {
        schema_version: 1,
        job_id: "successor-journaled".into(),
        attempt_count: 1,
        tweet_row_id: database.tweet_row_id("123").unwrap(),
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        media: Vec::new(),
        telegram_tweet_row_id: None,
    };
    database
        .create_archive_recovery_manifest(&manifest, "prepared")
        .expect("PREPARED journal");

    for job_id in ["successor-journaled", "successor-legacy"] {
        let events_before = database.count_job_events(job_id).unwrap();
        assert_eq!(
            database
                .claim_internal_v1_successor(job_id, 1, "refused")
                .expect("ineligible successor is a CAS loss"),
            None
        );
        assert_eq!(database.archive_attempt(job_id).unwrap(), 1);
        assert_eq!(
            database.job_state(job_id).unwrap(),
            xarchive_core::JobState::Downloading
        );
        assert_eq!(database.count_job_events(job_id).unwrap(), events_before);
    }
}

#[test]
fn claiming_archive_attempt_rejects_overflow_and_prepared_recovery() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("125", "https://x.com/a/status/125", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "overflow-claim-job",
            tweet_row_id,
            1,
            "overflow-claim-request",
            "{}",
            None,
            "now",
        )
        .expect("job");
    database
        .execute_batch("UPDATE jobs SET attempt_count = 4294967295 WHERE id = 'overflow-claim-job'")
        .expect("set maximum attempt");
    assert!(matches!(
        database.claim_archive_attempt(
            "overflow-claim-job",
            u32::MAX,
            xarchive_core::JobState::Queued,
            xarchive_core::JobState::Validating,
            "overflow",
        ),
        Err(StorageError::InvalidState(_))
    ));
    assert_eq!(
        database
            .archive_attempt("overflow-claim-job")
            .expect("attempt"),
        u32::MAX
    );

    let prepared_tweet_row_id = database
        .insert_tweet("126", "https://x.com/a/status/126", "post", "", "now")
        .expect("prepared tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "prepared-claim-job",
            prepared_tweet_row_id,
            1,
            "prepared-claim-request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("prepared job");
    let manifest = ArchiveRecoveryManifest {
        job_id: "prepared-claim-job".into(),
        attempt_count: 1,
        tweet_row_id: prepared_tweet_row_id,
        archive_directory: "Tweets/126".into(),
        tweet_id: "126".into(),
        media: Vec::new(),
        telegram_tweet_row_id: None,
        schema_version: 1,
    };
    assert_eq!(
        database
            .claim_archive_attempt(
                "prepared-claim-job",
                0,
                xarchive_core::JobState::Queued,
                xarchive_core::JobState::Validating,
                "now",
            )
            .expect("claim")
            .expect("attempt"),
        1
    );
    for state in [
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("prepared-claim-job", state, "now")
            .expect("active state");
    }
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("prepared journal");
    assert!(matches!(
        database.claim_archive_attempt(
            "prepared-claim-job",
            1,
            xarchive_core::JobState::Downloading,
            xarchive_core::JobState::Failed,
            "retry",
        ),
        Err(StorageError::InvalidState(_))
    ));
    assert_eq!(
        database
            .archive_attempt("prepared-claim-job")
            .expect("attempt"),
        1
    );
    assert_eq!(
        database.job_state("prepared-claim-job").expect("state"),
        xarchive_core::JobState::Downloading
    );
}
