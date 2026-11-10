//! `archive_recovery` — cancellation coverage.

use super::*;

#[test]
fn cancellation_arbitrates_against_the_archive_commit_boundary() {
    let mut database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-cancel",
            tweet_row_id,
            2,
            "request-cancel",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
            "now",
        )
        .expect("job");
    database
        .begin_archive_attempt("job-cancel", "now")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-cancel", state, "now")
            .expect("active job state");
    }

    // Before any journal exists, cancellation wins and is durable.
    assert!(
        !database
            .archive_commit_started("job-cancel")
            .expect("commit probe")
    );
    let events_before = database
        .count_job_events("job-cancel")
        .expect("event count");
    assert_eq!(
        database
            .decide_archive_cancellation("job-cancel", "t1")
            .expect("cancel decision"),
        ArchiveCancellationDecision::Cancelled
    );
    assert_eq!(
        database.job_state("job-cancel").expect("state"),
        xarchive_core::JobState::Cancelled
    );
    assert_eq!(
        database
            .count_job_events("job-cancel")
            .expect("event count"),
        events_before + 1,
        "the cancellation writes exactly its state-change event"
    );
    // Repeating the decision on a terminal job writes nothing new.
    assert_eq!(
        database
            .decide_archive_cancellation("job-cancel", "t2")
            .expect("idempotent decision"),
        ArchiveCancellationDecision::AlreadyDecided
    );
    assert_eq!(
        database
            .count_job_events("job-cancel")
            .expect("event count"),
        events_before + 1
    );

    // A second job prepares an archive; from that point the commit wins.
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-prepared",
            tweet_row_id,
            2,
            "request-prepared",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
            "now",
        )
        .expect("second job");
    database
        .begin_archive_attempt("job-prepared", "now")
        .expect("attempt");
    database
        .transition_job("job-prepared", xarchive_core::JobState::Validating, "now")
        .expect("active state");
    database
        .transition_job(
            "job-prepared",
            xarchive_core::JobState::MetadataReady,
            "now",
        )
        .expect("active state");
    database
        .transition_job("job-prepared", xarchive_core::JobState::Downloading, "now")
        .expect("active state");
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
        job_id: "job-prepared".into(),
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
    database
        .prepare_archive_recovery_v2(&manifest, &metadata, "now")
        .expect("prepared journal");

    assert!(
        database
            .archive_commit_started("job-prepared")
            .expect("commit probe")
    );
    assert_eq!(
        database
            .decide_archive_cancellation("job-prepared", "t3")
            .expect("cancel decision"),
        ArchiveCancellationDecision::CommitWins
    );
    assert_eq!(
        database.job_state("job-prepared").expect("state"),
        xarchive_core::JobState::Downloading,
        "the commit boundary leaves the job untouched"
    );
    let journal = database
        .archive_recovery_manifest_v2("job-prepared")
        .expect("journal")
        .expect("record");
    assert_eq!(journal.phase, ArchiveRecoveryPhase::Prepared);
}
