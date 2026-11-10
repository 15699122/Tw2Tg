//! `archive_recovery` — v1 journal coverage.

use super::*;

#[test]
fn v2_journal_is_idempotent_strict_and_has_monotonic_phase_transitions() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            2,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
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
            .expect("active job state");
    }
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
        media: vec![media],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    let metadata = test_candidate_metadata(
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
    let first = database
        .prepare_archive_recovery_v2(&manifest, &metadata, "now")
        .expect("persist v2 manifest");
    assert_eq!(first.phase, ArchiveRecoveryPhase::Prepared);
    assert_eq!(
        database
            .prepare_archive_recovery_v2(&manifest, &metadata, "later")
            .expect("idempotent retry"),
        first
    );
    database
        .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "t1")
        .expect("temporary phase");
    database
        .transition_archive_recovery_v2("job-1", "TEMPORARY", "FINAL", "t2")
        .expect("final phase");
    database
        .transition_archive_recovery_v2("job-1", "FINAL", "COMMITTED", "t3")
        .expect("commit phase");
    let committed = database
        .archive_recovery_manifest_v2("job-1")
        .expect("read record")
        .expect("record");
    assert_eq!(committed.phase, ArchiveRecoveryPhase::Committed);
    assert!(
        database
            .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "t4")
            .is_err()
    );
    let mut conflicting = manifest;
    conflicting.rename_plan[0].final_path = "other.jpg".into();
    conflicting.media[0].relative_path = "other.jpg".into();
    assert!(
        database
            .prepare_archive_recovery_v2(&conflicting, &metadata, "later")
            .is_err()
    );
}

/// Cancellation is arbitrated against the durable commit boundary: it wins
/// before PREPARED, and a PREPARED/COMMITTED journal always wins over a
/// cancelled state. The state and its event land in one transaction.
/// V1-05: cancellation at the PREPARED arbitration boundary.
///
/// The existing arbitration test exercises the InternalRenameV2 journal. This
/// one proves the InternalV1 branch specifically: once a PREPARED journal
/// exists, cancellation must lose to the commit so recovery cannot be
/// diverted into publishing the wrong attempt, and no cancellation writes
/// may occur.
#[test]
fn internal_v1_prepared_journal_wins_cancellation_arbitration() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-v1-cancel",
            tweet_row_id,
            1,
            "request-v1-cancel",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .expect("job");
    let attempt = database
        .begin_archive_attempt("job-v1-cancel", "now")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-v1-cancel", state, "now")
            .expect("active state");
    }
    let manifest = ArchiveRecoveryManifest {
        schema_version: 1,
        job_id: "job-v1-cancel".into(),
        attempt_count: attempt,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        media: Vec::new(),
        telegram_tweet_row_id: None,
    };
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .expect("PREPARED journal");

    // Before PREPARED existed, cancellation would win; the journal now makes
    // the commit authoritative and must write nothing.
    let events_before = database.count_job_events("job-v1-cancel").expect("events");
    assert_eq!(
        database
            .decide_archive_cancellation("job-v1-cancel", "t1")
            .expect("arbitration"),
        ArchiveCancellationDecision::CommitWins,
        "a PREPARED InternalV1 journal must win cancellation"
    );
    assert_eq!(
        database.count_job_events("job-v1-cancel").expect("events"),
        events_before,
        "a commit-wins decision must not write a cancellation event"
    );
    assert_eq!(
        database.job_state("job-v1-cancel").expect("state"),
        xarchive_core::JobState::Downloading,
        "the attempt must stay recoverable, not become Cancelled"
    );
    assert_eq!(
        database.archive_attempt("job-v1-cancel").expect("attempt"),
        attempt,
        "arbitration must not advance or reset the durable attempt"
    );
    assert_eq!(
        database
            .archive_recovery_journal("job-v1-cancel")
            .expect("journal")
            .expect("record")
            .phase,
        ArchiveRecoveryPhase::Prepared,
        "the PREPARED journal remains the recovery authority"
    );
    // Repeating the decision is stable and still writes nothing.
    assert_eq!(
        database
            .decide_archive_cancellation("job-v1-cancel", "t2")
            .expect("repeat arbitration"),
        ArchiveCancellationDecision::CommitWins
    );
    assert_eq!(
        database.count_job_events("job-v1-cancel").expect("events"),
        events_before
    );
}
