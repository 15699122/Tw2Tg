//! Archive service archive, preview, intent journal and V2 publication coverage.
//!
//! The crash-window test below re-executes this test binary through
//! `current_exe()` with a hardcoded `--exact` selector naming this module, so it
//! stays in `mod.rs` and the selector keeps selecting exactly one test.

mod local;
mod preview;
mod sidecar_paths;
mod telegram_intent;
mod v2_candidate;
mod v2_forward_publish;
mod v2_rename_recovery;

use rusqlite::OptionalExtension;

use super::*;

#[test]
fn v2_forward_publish_crash_windows_converge_through_recovery_replay() {
    const ROOT_ENV: &str = "TW2TG_P5_PUBLISH_CRASH_ROOT";
    const WINDOW_ENV: &str = "TW2TG_P5_PUBLISH_CRASH_WINDOW";
    if let Some(root) = std::env::var_os(ROOT_ENV) {
        let root = std::path::PathBuf::from(root);
        let window = std::env::var(WINDOW_ENV).unwrap();
        let failpoint = match window.as_str() {
            "temporary" => Some(ArchiveRecoveryFailpoint::AfterTemporaryRename(0)),
            "final" => Some(ArchiveRecoveryFailpoint::AfterFinalRename(0)),
            "directory" => Some(ArchiveRecoveryFailpoint::AfterDirectoryCommit),
            "unprepared" | "prepared" | "committed" => None,
            other => panic!("unknown crash window {other}"),
        };
        let (database, files, _, manifest, candidate) =
            unprepared_v2_publish_inputs(&root, "job-1");
        let mut service = ArchiveService::new(database, files);
        if window == "unprepared" {
            // Candidate bytes exist but no journal authorizes publication.
        } else if window == "prepared" {
            service
                .database
                .prepare_archive_recovery_v2(&manifest, &candidate, "now")
                .expect("persist PREPARED before crash");
        } else {
            let result = service.publish_prepared_internal_rename_v2_for_test(
                &manifest,
                &candidate,
                Path::new("Tweets/123"),
                "now",
                failpoint,
            );
            if window == "committed" {
                result.expect("finalize before crash");
            } else {
                let error = result.expect_err("publish must stop at requested failpoint");
                assert!(error.to_string().contains("injected v2 publish crash"));
            }
        }
        fs::write(root.join("crash.ready"), manifest.tweet_row_id.to_string())
            .expect("record reached failpoint");
        // Do not run ArchiveService/SQLite destructors: recovery must use
        // only the files and transactions durable at the production seam.
        std::process::abort();
    }
    for window in [
        "unprepared",
        "prepared",
        "temporary",
        "final",
        "directory",
        "committed",
    ] {
        let root = temp_root();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::archive_service::v2_forward_publish_crash_windows_converge_through_recovery_replay",
                "--nocapture",
            ])
            .env(ROOT_ENV, &root)
            .env(WINDOW_ENV, window)
            .output()
            .expect("run isolated publish owner");
        assert!(!output.status.success(), "publish owner must crash");
        let tweet_row_id: i64 = fs::read_to_string(root.join("crash.ready"))
            .unwrap_or_else(|error| {
                panic!(
                    "{window}: failpoint was not reached: {error}; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
            })
            .parse()
            .unwrap();
        let database = Database::open(root.join("archive.sqlite3")).expect("reopen");
        let files = FileStore::new(&root).expect("reopen files");
        let mut service = ArchiveService::new(database, files);
        if window == "unprepared" {
            assert!(
                service
                    .recover_internal_rename_v2(
                        "job-1",
                        tweet_row_id,
                        Path::new("Tweets/123"),
                        "later"
                    )
                    .is_err()
            );
            assert!(!root.join("Tweets/123").exists());
            assert_eq!(
                service.database.job_state("job-1").expect("state"),
                JobState::Downloading
            );
            drop(service);
            let _ = fs::remove_dir_all(root);
            continue;
        }
        let destination = service
            .recover_internal_rename_v2("job-1", tweet_row_id, Path::new("Tweets/123"), "later")
            .expect("crash window replays to committed");
        assert_eq!(destination, root.join("Tweets/123"));
        assert_eq!(
            fs::read(destination.join("final.jpg")).expect("final media"),
            b"recoverable media payload"
        );
        assert_eq!(
            service
                .database
                .archive_recovery_manifest_v2("job-1")
                .expect("journal")
                .expect("record")
                .phase,
            ArchiveRecoveryPhase::Committed
        );
        let event_count = service.database.count_job_events("job-1").expect("events");
        assert_eq!(
            service
                .recover_internal_rename_v2(
                    "job-1",
                    tweet_row_id,
                    Path::new("Tweets/123"),
                    "repeat"
                )
                .expect("repeat replay"),
            destination
        );
        assert_eq!(
            service.database.count_job_events("job-1").expect("events"),
            event_count
        );
        assert_eq!(
            fs::read(destination.join("final.jpg")).unwrap(),
            b"recoverable media payload"
        );
        let _ = fs::remove_dir_all(root);
    }
}
