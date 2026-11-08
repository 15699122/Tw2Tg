//! Archive recovery dispatch, attempt claims, journal replay and crash windows.

use super::*;
use crate::{ArchiveService, FileStore};
use sha2::Digest;
use xarchive_core::JobState;

fn test_candidate_metadata(
    tweet_id: &str,
    media: Vec<xarchive_core::ArchiveMedia>,
) -> ArchiveMetadata {
    ArchiveMetadata {
        schema_version: 1,
        tweet_id: tweet_id.into(),
        url: format!("https://x.com/a/status/{tweet_id}"),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: None,
            display_name: None,
        },
        created_at: None,
        text: "recovery candidate".into(),
        media,
        archived_at: "now".into(),
        reply_to: None,
        quoted_tweet: None,
    }
}
fn manifest(job_id: &str, attempt_count: u32) -> ArchiveRecoveryManifest {
    ArchiveRecoveryManifest {
        schema_version: 1,
        job_id: job_id.into(),
        attempt_count,
        tweet_row_id: 1,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "media/01.jpg".into(),
            media_id: Some("media-1".into()),
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        telegram_tweet_row_id: None,
    }
}
/// Recover one crashed V1 window through the real lock-protected
/// production PREPARED replay and prove convergence plus idempotence.
fn replay_crashed_v1_window(root: &Path, window: &str) {
    let workflow_attempt =
        xarchive_workflow::AttemptIdentity::new("job-1", 1).expect("workflow attempt");
    let destination = root.join("archive/Tweets/123");
    let coordinator =
        xarchive_workflow::coordination::unix::LinuxCoordinator::new(root.join("locks"))
            .expect("coordinator");
    let session =
        xarchive_workflow::ExecutionReservation::acquire(&coordinator, "job-1", &destination)
            .expect("recovery locks")
            .bind_existing_attempt_checked(workflow_attempt.clone(), |bound| bound.attempt() == 1)
            .expect("bind the crashed attempt");
    let staging_root = root.join("staging");
    let evidence =
        xarchive_workflow::linux_containment::LinuxContainment::recover_production_attempt(
            &root.join("containment-records"),
            session.recovery_drain_binding(&staging_root),
            std::time::Duration::from_secs(1),
        )
        .expect("crashed writer domains are recoverable");
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
    .expect("authorize replay under both locks");

    let mut service = ArchiveService::new(
        Database::open(root.join("database.sqlite3")).expect("reopen database"),
        FileStore::with_staging_root(&root.join("archive"), &staging_root)
            .expect("reopen file store"),
    );
    let events_before = service
        .database
        .count_job_events("job-1")
        .expect("events before replay");
    if window == "committed" {
        // The publish completed before the abort, so nothing needs replay:
        // the durable facts must already be converged and the journal
        // COMMITTED. Startup treats this as a read-only verification plus
        // attempt-fenced terminal convergence.
        assert!(
            service
                .database
                .tweet_archive_facts("123")
                .expect("archive facts")
                .is_some(),
            "committed: the completed publish already recorded archive facts"
        );
        assert_eq!(
            service
                .database
                .archive_recovery_journal("job-1")
                .expect("journal")
                .expect("record")
                .phase,
            ArchiveRecoveryPhase::Committed,
            "committed: the journal was already COMMITTED before the crash"
        );
        assert_eq!(
            std::fs::read(destination.join("123_01.jpg")).expect("final media"),
            b"img",
            "committed: the final media is durable"
        );
        assert_attempt_fenced_terminal_convergence(&mut service, &workflow_attempt, window);
        return;
    }
    assert!(
        service
            .database
            .tweet_archive_facts("123")
            .expect("archive facts before replay")
            .is_none(),
        "{window}: the crashed window must not have converged archive facts already"
    );

    let replayed = service
        .replay_prepared_internal_v1(
            "job-1",
            1,
            Path::new("Tweets/123"),
            &workflow_attempt,
            permit,
        )
        .unwrap_or_else(|error| {
            panic!("{window}: PREPARED replay must converge after a crash, got {error}")
        });
    assert_eq!(
        replayed, destination,
        "{window}: replay returns the final path"
    );
    assert_eq!(
        std::fs::read(destination.join("123_01.jpg")).expect("final media"),
        b"img",
        "{window}: the committed media bytes are the transfer bytes"
    );
    assert_eq!(
        service
            .database
            .archive_recovery_journal("job-1")
            .expect("journal")
            .expect("record")
            .phase,
        ArchiveRecoveryPhase::Committed,
        "{window}: the journal converges to COMMITTED"
    );

    assert!(
        service
            .converge_committed_internal_v1(
                "job-1",
                &workflow_attempt,
                Path::new("Tweets/123"),
                "recovered",
            )
            .expect("terminal convergence"),
        "{window}: the crashed attempt is completed once"
    );
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        xarchive_core::JobState::Complete
    );
    let events_after = service
        .database
        .count_job_events("job-1")
        .expect("events after convergence");
    assert!(
        events_after > events_before,
        "{window}: the terminal transition is recorded"
    );
    repeat_convergence_is_idempotent(&mut service, &workflow_attempt, window);
}
/// A second terminal convergence must be a no-op and must not duplicate
/// completion events.
fn repeat_convergence_is_idempotent(
    service: &mut ArchiveService,
    workflow_attempt: &xarchive_workflow::AttemptIdentity,
    window: &str,
) {
    assert!(
        !service
            .converge_committed_internal_v1(
                "job-1",
                workflow_attempt,
                Path::new("Tweets/123"),
                "again",
            )
            .expect("idempotent convergence"),
        "{window}: repeated convergence must be a no-op"
    );
    assert_eq!(
        service.database.job_state("job-1").expect("state"),
        xarchive_core::JobState::Complete,
        "{window}: repeated convergence keeps the terminal state"
    );
}
/// Terminal convergence must be attempt-fenced and repeat-safe: a second
/// run is a no-op and never duplicates completion events.
fn assert_attempt_fenced_terminal_convergence(
    service: &mut ArchiveService,
    workflow_attempt: &xarchive_workflow::AttemptIdentity,
    window: &str,
) {
    assert!(
        service
            .converge_committed_internal_v1(
                "job-1",
                workflow_attempt,
                Path::new("Tweets/123"),
                "after-replay",
            )
            .expect("attempt-fenced terminal convergence"),
        "{window}: the committed attempt is completed once"
    );
    repeat_convergence_is_idempotent(service, workflow_attempt, window);
}
/// Drive one V1 publish crash window and abort the process at that durable
/// boundary. Called only inside the isolated child of
/// [`v1_publish_crash_windows_converge_through_production_prepared_replay`].
fn run_v1_crash_window_child(root: &Path, window: &str) -> ! {
    use crate::{ArchiveRecoveryFailpoint, BatchOutputSettings, SidecarArchiveRequest};
    let mut database = Database::open(root.join("database.sqlite3")).expect("child database");
    let files = FileStore::with_staging_root(&root.join("archive"), &root.join("staging"))
        .expect("child file store");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
        .expect("child tweet");
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
        .expect("child InternalV1 job");
    let attempt = database
        .claim_archive_attempt(
            "job-1",
            0,
            xarchive_core::JobState::Queued,
            xarchive_core::JobState::Validating,
            "now",
        )
        .expect("child claim")
        .expect("child winner");
    assert_eq!(attempt, 1);
    let staging = files
        .create_attempt_staging_dir("job-1", attempt)
        .expect("child attempt staging");
    std::fs::create_dir_all(staging.join("media")).expect("child media directory");
    std::fs::write(staging.join("media/01.jpg"), b"img").expect("child media payload");
    for state in [
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job("job-1", state, "now")
            .expect("child job advance");
    }

    let metadata = serde_json::json!({
        "tweet_id": "123",
        "url": "https://x.com/a/status/123",
        "tweet_type": "post",
        "text": "fixture text",
        "media": [],
    });
    let downloads = vec![xarchive_protocol::DownloadFile {
        relative_path: "media/01.jpg".into(),
        size_bytes: 3,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let output_settings = BatchOutputSettings {
        naming_mode: crate::BatchNamingMode::Template,
        filename_template: "{tweet_id}_{index}".into(),
        ..BatchOutputSettings::default()
    };
    let archived_at = "now";
    let mut service = ArchiveService::new(database, files);
    let request = SidecarArchiveRequest {
        job_id: "job-1",
        tweet_row_id,
        expected_tweet_id: "123",
        metadata: &metadata,
        files: &downloads,
        final_directory: Path::new("Tweets/123"),
        archived_at,
        telegram_intent: None,
        output_settings: &output_settings,
    };

    if window == "prepared" {
        // Persist PREPARED and stop before any rename executes.
        let preview = service
            .preview_sidecar_archive_attempt(
                "job-1",
                attempt,
                "123",
                &metadata,
                &downloads,
                archived_at,
                &output_settings,
            )
            .expect("child preview");
        let manifest = ArchiveRecoveryManifest::from_metadata(
            "job-1",
            attempt,
            tweet_row_id,
            Path::new("Tweets/123"),
            &preview.metadata,
            None,
        )
        .expect("child manifest");
        // Match the production write order: the durable rename plan lands
        // in staging before the renames run, so crash replay can converge
        // the tree without guessing original names.
        let pending: Vec<crate::archive_service::PendingRename> = preview
            .renames
            .iter()
            .filter(|(source, target)| source != target)
            .map(|(source, target)| {
                let path = staging.join(source);
                crate::archive_service::PendingRename {
                    source_path: source.clone(),
                    final_path: target.clone(),
                    size_bytes: std::fs::metadata(&path)
                        .expect("child rename source metadata")
                        .len(),
                    sha256: FileStore::sha256(&path).expect("child rename source digest"),
                }
            })
            .collect();
        std::fs::write(
            staging.join(crate::archive_service::RENAME_PLAN_FILE),
            serde_json::to_vec_pretty(&pending).expect("child rename plan JSON"),
        )
        .expect("child disk rename plan");
        std::fs::write(
            staging.join(".xarchive-recovery.json"),
            serde_json::to_vec(&manifest).expect("child manifest JSON"),
        )
        .expect("child disk manifest");
        service
            .database
            .create_archive_recovery_manifest(&manifest, archived_at)
            .expect("child PREPARED journal");
    } else {
        let failpoint = match window {
            "rename" => Some(ArchiveRecoveryFailpoint::AfterFinalRename(0)),
            "directory" => Some(ArchiveRecoveryFailpoint::AfterDirectoryCommit),
            "committed" => None,
            other => panic!("unknown v1 crash window {other}"),
        };
        let result = service.complete_sidecar_archive_attempt_for_test(request, attempt, failpoint);
        if window == "committed" {
            result.expect("child publishes before abort");
        } else {
            let error = result.expect_err("child must stop at the requested window");
            assert!(
                error.to_string().contains("injected v1 publish crash"),
                "unexpected window error: {error}"
            );
        }
    }
    std::fs::write(root.join("crash.ready"), "1").expect("child reached failpoint");
    // Skip ArchiveService/SQLite destructors: recovery must rely only on the
    // files and transactions that were durable at that seam.
    std::process::abort();
}

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
#[test]
fn v1_publish_crash_windows_converge_through_production_prepared_replay() {
    // V1-04 requires process-crash evidence, not a returned error. Each
    // publish window runs in an isolated child that reaches the requested
    // durable boundary and then aborts without running ArchiveService or
    // SQLite destructors, so recovery only sees what was durable at the
    // production seam.
    const ROOT_ENV: &str = "TW2TG_V1_PUBLISH_CRASH_ROOT";
    const WINDOW_ENV: &str = "TW2TG_V1_PUBLISH_CRASH_WINDOW";
    if let Some(root) = std::env::var_os(ROOT_ENV) {
        let root = std::path::PathBuf::from(root);
        let window = std::env::var(WINDOW_ENV).expect("crash window");
        run_v1_crash_window_child(&root, &window);
    }

    for window in ["prepared", "rename", "directory", "committed"] {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "xarchive-v1-crash-window-{}-{}-{}",
            window,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).expect("crash window root");
        let output = std::process::Command::new(std::env::current_exe().expect("child exe"))
                .args([
                    "--exact",
                    "tests::archive_recovery::v1_publish_crash_windows_converge_through_production_prepared_replay",
                    "--nocapture",
                ])
                .env(ROOT_ENV, &root)
                .env(WINDOW_ENV, window)
                .output()
                .expect("run isolated v1 publish owner");
        assert!(
            !output.status.success(),
            "{window}: the publish owner must abort, not return"
        );
        assert!(
            root.join("crash.ready").is_file(),
            "{window}: failpoint was not reached; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        replay_crashed_v1_window(&root, window);
        let _ = std::fs::remove_dir_all(&root);
    }
}
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
