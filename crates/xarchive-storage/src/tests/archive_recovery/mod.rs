//! Archive recovery dispatch, attempt claims, journal replay and crash windows.
//!
//! The crash-window test below re-executes this test binary through
//! `current_exe()` with a hardcoded `--exact` selector naming this module, so it
//! stays in `mod.rs` and the selector keeps selecting exactly one test.

mod cancellation;
mod claims;
mod dispatch;
mod v1_journal;
mod v2_committed;
mod v2_finalization;

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
