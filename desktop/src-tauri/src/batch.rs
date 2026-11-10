//! Account batch discovery persistence and dispatch selection.
//!
//! The Sidecar `discover` command only streams Tweet candidates. Everything
//! after that is owned here: the filter schema persisted in
//! `archive_batches.filters_json`, durable candidate persistence, stable
//! identity binding, already-archived skip verification against committed
//! archive facts, and the bounded candidate list handed to the job executor.
//!
//! A batch decides *which* Tweets to archive. The archive completeness decision
//! stays with the single-Tweet commit path (`ArchiveService`), so a skip here
//! only means "this Tweet already has a committed directory containing every
//! recorded media file", never "the download is complete".
//!
//! The Desktop intake (Tauri commands and the dispatch worker) is wired in the
//! following batch, so the module keeps the executor-module convention of
//! allowing not-yet-called production code. Its behavior is covered by the
//! module tests below.
#![allow(dead_code)]

// Only `storage_error` remains in this module; the imports below are used by the
// regression tests that still live here.
use xarchive_storage::StorageError;

#[cfg(test)]
use crate::executor::CancellationToken;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Arc, Mutex as StdMutex};
#[cfg(test)]
use std::thread;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use xarchive_core::JobState;
#[cfg(test)]
use xarchive_sidecar_supervisor::SidecarSupervisor;
#[cfg(test)]
use xarchive_storage::{Database, FileStore};

mod discovery;
mod dispatch;
mod selection;
mod submission;

// Compatibility re-exports: discovery and selection moved to `batch/`
// submodules, but the parent（以及同级子模块）继续用 `crate::batch::<name>` 与
// `super::<name>` 访问。作用域与迁移前一致 (`pub(crate)` 或模块私有)，没有
// 放宽任何项。
#[allow(unused_imports)]
pub(crate) use discovery::discovery_error_code;
#[allow(unused_imports)]
pub(crate) use discovery::{
    DiscoverySummary, SKIP_AFTER_RANGE, SKIP_ALREADY_ARCHIVED, SKIP_BEFORE_RANGE,
    SKIP_BEYOND_LIMIT, SKIP_NO_MEDIA, SKIP_REPOST, candidate_record, discovery_request,
    persist_discovered_candidates, run_account_discovery,
};
#[allow(unused_imports)]
pub(crate) use selection::{BatchFilters, candidate_archive_request, select_dispatch_candidates};
// The submission contract stays available to the parent and to `dispatch` at its
// original private reach.
#[allow(unused_imports)]
pub(in crate::batch) use submission::{inherited_execution_spec, metadata_batch_settings};
// The coordinator entry points keep their original `pub(crate)` reach because
// `commands` and the runtime call them through `crate::batch::<name>`.
#[allow(unused_imports)]
pub(crate) use dispatch::{dispatch_batch_pass, spawn_account_batch, spawn_batch_dispatch};
// Reached only by this module's remaining regression tests, which still live in
// `batch.rs`. Scope stays inside `crate::batch`.
#[allow(unused_imports)]
pub(in crate::batch) use dispatch::{
    BatchCancellationGuard, BatchDispatchPass, reconcile_batch_candidates,
};

/// Keep a persisted error message bounded; the batch row is user-visible.
fn truncate_message(message: &str) -> String {
    message.chars().take(4000).collect()
}

fn storage_error(context: &str, error: StorageError) -> String {
    format!("{context}: {error}")
}

#[cfg(test)]
mod d1_batch_tests {
    use super::*;

    #[test]
    fn d1_batch_snapshot_dispatches_v4_and_refuses_future_or_unknown_fields() {
        let settings = crate::executor::MetadataOutputSettings {
            export_json: false,
            export_yaml: true,
            ..Default::default()
        };
        let raw = serde_json::json!({"schema_version":4,"settings":settings}).to_string();
        assert_eq!(metadata_batch_settings(&raw).unwrap(), Some(settings));
        let database = Database::open_in_memory().unwrap();
        let legacy = crate::executor::OutputSettings::default();
        database
            .create_account_batch_with_metadata_snapshot(
                xarchive_storage::CreateAccountBatch {
                    id: "d1-batch",
                    username: "alice",
                    profile_url: "https://x.com/alice",
                    browser: None,
                    profile: None,
                    filters_json: "{}",
                    output_settings: &legacy,
                    downloader_arguments_json: "{}",
                },
                &raw,
            )
            .unwrap();
        assert_eq!(
            database
                .account_batch_output_settings_json("d1-batch")
                .unwrap()
                .as_deref(),
            Some(raw.as_str())
        );
        assert!(metadata_batch_settings(r#"{"schema_version":5,"settings":{}}"#).is_err());
        assert!(
            metadata_batch_settings(r#"{"schema_version":4,"settings":{},"extra":true}"#).is_err()
        );
        assert!(
            metadata_batch_settings(
                &serde_json::to_string(&crate::executor::OutputSettings::default()).unwrap()
            )
            .unwrap()
            .is_none()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::production::{DiscoveryOutcome, DiscoveryRequest};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use xarchive_storage::BatchCandidateRecord;

    use xarchive_core::{ArchiveMedia, ArchiveMetadata};

    const NOW: &str = "2026-09-22T00:00:00Z";

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "xarchive-batch-test-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    fn candidate_full(
        tweet_id: &str,
        created_at: Option<&str>,
        media_count: u32,
        is_repost: bool,
    ) -> xarchive_protocol::DiscoveryCandidate {
        xarchive_protocol::DiscoveryCandidate {
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            created_at: created_at.map(str::to_owned),
            tweet_type: if is_repost {
                "retweet".to_owned()
            } else {
                "post".to_owned()
            },
            is_repost,
            has_media: media_count > 0,
            media_count,
            user_id: Some("42".to_owned()),
            username: Some("alice".to_owned()),
        }
    }

    fn outcome(candidates: Vec<xarchive_protocol::DiscoveryCandidate>) -> DiscoveryOutcome {
        DiscoveryOutcome {
            candidates_found: candidates.len() as u64,
            candidates,
        }
    }

    fn open_batch(database: &Database, id: &str) {
        let output_settings = crate::executor::OutputSettings::default();
        database
            .create_account_batch(xarchive_storage::CreateAccountBatch {
                id,
                username: "alice",
                profile_url: "https://x.com/alice",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &output_settings,
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            })
            .expect("batch");
    }

    #[test]
    fn child_execution_spec_inherits_persisted_batch_snapshot() {
        let request = candidate_archive_request(
            &BatchCandidateRecord {
                tweet_id: "12345".into(),
                url: "https://x.com/alice/status/12345".into(),
                created_at: None,
                tweet_type: "post".into(),
                is_repost: false,
                has_media: true,
                media_count: 1,
                user_id: Some("42".into()),
                username: Some("alice".into()),
                state: "PENDING".into(),
                job_id: None,
                error_code: None,
                error_message: None,
                skip_reason: None,
            },
            None,
            None,
        )
        .expect("candidate request");
        let snapshot = r#"{"naming_mode":"template","filename_template":"{tweet_id}","export_json":false,"export_text":true}"#;

        let args = r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#;
        let child_json = inherited_execution_spec(request, snapshot, args).expect("child spec");
        let child = crate::executor::decode_execution_spec(3, &child_json).expect("decode child");

        assert_eq!(child.request.tweet.tweet_id, "12345");
        assert_eq!(
            child.output_settings,
            crate::executor::OutputSettings {
                naming_mode: crate::executor::NamingMode::Template,
                filename_template: "{tweet_id}".into(),
                export_json: false,
                export_text: true,
            }
        );
        let d1_settings = crate::executor::MetadataOutputSettings {
            export_json: false,
            export_yaml: true,
            ..Default::default()
        };
        let d1_snapshot =
            serde_json::json!({"schema_version":4,"settings":d1_settings}).to_string();
        let d1_json = inherited_execution_spec(child.request.clone(), &d1_snapshot, args).unwrap();
        let d1_child = crate::executor::decode_execution_spec(4, &d1_json).unwrap();
        assert_eq!(d1_child.metadata_output, Some(d1_settings));
        assert!(!d1_child.output_settings.export_text);
        assert!(
            inherited_execution_spec(
                candidate_archive_request(
                    &BatchCandidateRecord {
                        tweet_id: "1".into(),
                        url: "https://x.com/alice/status/1".into(),
                        created_at: None,
                        tweet_type: "post".into(),
                        is_repost: false,
                        has_media: true,
                        media_count: 1,
                        user_id: Some("42".into()),
                        username: Some("alice".into()),
                        state: "PENDING".into(),
                        job_id: None,
                        error_code: None,
                        error_message: None,
                        skip_reason: None,
                    },
                    None,
                    None,
                )
                .expect("second candidate request"),
                "{broken",
                args
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn failed_batch_candidate_can_redispatch_through_persisted_scheduler() {
        let root = temp_root("retry-session");
        fs::create_dir_all(root.join("config")).expect("config");
        let database_path = root.join("config/archive.sqlite3");
        let database = Database::open(&database_path).expect("database");
        open_batch(&database, "batch-retry");
        persist_discovered_candidates(
            &database,
            "batch-retry",
            &outcome(vec![candidate_full("12345", None, 1, false)]),
        )
        .expect("candidate");
        let files = xarchive_storage::FileStore::new(&root).expect("files");
        let executor = crate::executor::JobExecutor::new();
        let service = crate::executor::ArchiveApplicationService::new(&executor);
        let mut previous_job = None;
        for _ in 0..2 {
            database
                .set_account_batch_state("batch-retry", "ACTIVE")
                .expect("active");
            let pass = dispatch_batch_pass(
                &database,
                &files,
                &service,
                &database_path,
                "batch-retry",
                1,
            )
            .expect("dispatch");
            assert_eq!(
                pass.submitted,
                1,
                "candidates: {:?}",
                database
                    .list_batch_candidates("batch-retry", None, 10)
                    .unwrap()
            );
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                reconcile_batch_candidates(&database, "batch-retry").expect("reconcile");
                let failed = database
                    .list_batch_candidates("batch-retry", Some("FAILED"), 10)
                    .expect("failed candidates");
                if let Some(candidate) = failed.first() {
                    let job_id = candidate.job_id.clone().expect("persisted job");
                    let summary = database.job_summary(&job_id).unwrap().unwrap();
                    assert_eq!(summary.state, JobState::Failed);
                    assert_eq!(
                        summary.last_error_code.as_deref(),
                        Some("EXECUTOR_WORKER_FAILED")
                    );
                    if let Some(previous) = previous_job.as_ref() {
                        assert_ne!(
                            previous, &job_id,
                            "retry dispatch creates a new terminal-job successor"
                        );
                    }
                    previous_job = Some(job_id);
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "scheduled failure must become durable"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            database
                .retry_failed_batch_candidates("batch-retry")
                .expect("reset failed candidate");
        }
        executor.shutdown().expect("shutdown");
        drop(database);
        let _ = fs::remove_dir_all(root);
    }

    fn selected_ids(candidates: &[BatchCandidateRecord]) -> Vec<String> {
        candidates
            .iter()
            .map(|candidate| candidate.tweet_id.clone())
            .collect()
    }

    #[cfg(unix)]
    #[test]
    fn failed_batch_retry_resumes_to_attempt_owned_success() {
        use crate::executor::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Factory(Arc<AtomicUsize>);
        struct Execution(bool);
        impl JobExecution for Execution {
            fn execute(
                &mut self,
                _: &JobSnapshot,
                _: &CancellationToken,
            ) -> Result<JobExecutionResult, JobExecutionError> {
                if !self.0 {
                    return Err(JobExecutionError {
                        error_code: "TEST_RETRY".into(),
                        error_message: "first dispatch".into(),
                        persistence_already_updated: false,
                    });
                }
                Ok(JobExecutionResult {
                    files_copied: 1,
                    archive_directory: "Tweets/retried".into(),
                    download_event_recorded: false,
                    state_already_updated: false,
                    pending_publication: None,
                })
            }
        }
        impl JobExecutionFactory for Factory {
            fn create(
                &self,
                _: &str,
                _: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                panic!("batch must bind an attempt");
            }
            fn create_for_attempt(
                &self,
                identity: &xarchive_workflow::AttemptIdentity,
                snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
                assert_eq!(identity.job_id(), snapshot.job_id);
                assert_eq!(identity.attempt(), 1);
                Ok(Box::new(Execution(
                    self.0.fetch_add(1, Ordering::SeqCst) > 0,
                )))
            }
        }
        let root = temp_root("retry-success");
        fs::create_dir_all(root.join("config")).unwrap();
        let path = root.join("config/archive.sqlite3");
        let database = Database::open(&path).unwrap();
        open_batch(&database, "retry-success");
        persist_discovered_candidates(
            &database,
            "retry-success",
            &outcome(vec![candidate_full("12345", None, 1, false)]),
        )
        .unwrap();
        let files = xarchive_storage::FileStore::new(&root).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = JobExecutor::with_capacity_and_factory(4, Arc::new(Factory(calls.clone())));
        let service = ArchiveApplicationService::new(&executor);
        let mut jobs = Vec::new();
        for expected in ["FAILED", "DONE"] {
            database
                .set_account_batch_state("retry-success", "ACTIVE")
                .unwrap();
            assert_eq!(
                dispatch_batch_pass(&database, &files, &service, &path, "retry-success", 1)
                    .unwrap()
                    .submitted,
                1
            );
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                reconcile_batch_candidates(&database, "retry-success").unwrap();
                let candidates = database
                    .list_batch_candidates("retry-success", Some(expected), 10)
                    .unwrap();
                if let Some(candidate) = candidates.first() {
                    jobs.push(candidate.job_id.clone().unwrap());
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "candidate must reach {expected}"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if expected == "FAILED" {
                database
                    .retry_failed_batch_candidates("retry-success")
                    .unwrap();
                database
                    .set_account_batch_state("retry-success", "PAUSED")
                    .unwrap();
                assert_eq!(
                    dispatch_batch_pass(&database, &files, &service, &path, "retry-success", 1)
                        .unwrap()
                        .submitted,
                    0
                );
            }
        }
        assert_ne!(jobs[0], jobs[1]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            database.job_summary(&jobs[1]).unwrap().unwrap().state,
            JobState::Complete
        );
        assert!(
            dispatch_batch_pass(&database, &files, &service, &path, "retry-success", 1)
                .unwrap()
                .completed
        );
        executor.shutdown().unwrap();
        drop(database);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn batch_cancellation_guard_keeps_newer_worker_generation() {
        let cancellations = Arc::new(StdMutex::new(HashMap::new()));
        let old_token = CancellationToken::new();
        let new_token = CancellationToken::new();
        cancellations
            .lock()
            .expect("registry")
            .insert("batch-1".to_owned(), new_token.clone());
        let guard = BatchCancellationGuard {
            cancellations: cancellations.clone(),
            batch_id: "batch-1".to_owned(),
            token: old_token,
        };
        drop(guard);
        let registry = cancellations.lock().expect("registry after guard");
        assert!(
            registry
                .get("batch-1")
                .expect("new token")
                .same_instance(&new_token)
        );
    }

    #[test]
    fn batch_filters_use_product_defaults_and_reject_unknown_fields() {
        let filters = BatchFilters::from_json("{}").expect("defaults");
        assert_eq!(filters, BatchFilters::default());
        assert!(!filters.include_reposts);
        assert!(!filters.include_without_media);
        assert_eq!(filters.limit, None);

        let explicit = BatchFilters::from_json(
            r#"{"include_reposts":true,"include_without_media":true,"since":"2026-09-01T00:00:00Z","until":"2026-09-30T00:00:00Z","limit":25}"#,
        )
        .expect("explicit filters");
        assert_eq!(
            BatchFilters::from_json(&explicit.to_json().expect("json")).expect("round trip"),
            explicit
        );
        assert!(BatchFilters::from_json(r#"{"unexpected":1}"#).is_err());
        assert!(BatchFilters::from_json("not-json").is_err());
    }

    #[test]
    fn persists_discovery_candidates_once_and_binds_stable_identity() {
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        let discovery = outcome(vec![
            candidate_full("100", Some("2026-09-20T00:00:00Z"), 2, false),
            candidate_full("101", Some("2026-09-21T00:00:00Z"), 0, false),
        ]);

        let summary =
            persist_discovered_candidates(&database, "batch-1", &discovery).expect("persist");
        assert_eq!(summary.candidates_found, 2);
        assert_eq!(summary.inserted, 2);
        assert_eq!(summary.user_id.as_deref(), Some("42"));
        assert_eq!(summary.username.as_deref(), Some("alice"));

        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.user_id.as_deref(), Some("42"));
        assert_eq!(batch.counts.total, 2);
        assert_eq!(batch.counts.pending, 2);
        assert_eq!(
            batch.discovery_state, "PENDING",
            "persisting candidates must not advance the discovery state"
        );

        let stored = database
            .list_batch_candidates("batch-1", None, 10)
            .expect("candidates");
        assert_eq!(stored[0].tweet_id, "100");
        assert_eq!(stored[0].media_count, 2);
        assert!(stored[0].has_media);
        assert!(!stored[0].is_repost);
        assert_eq!(stored[1].tweet_id, "101");
        assert!(!stored[1].has_media);

        // A re-run must not duplicate rows nor resurrect an already dispatched row.
        database
            .mark_batch_candidate(
                "batch-1",
                "100",
                "SUBMITTED",
                Some("job-100"),
                None,
                None,
                None,
            )
            .expect("submit");
        let rerun = persist_discovered_candidates(&database, "batch-1", &discovery).expect("rerun");
        assert_eq!(rerun.inserted, 0);
        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.counts.total, 2);
        assert_eq!(batch.counts.submitted, 1);
        let submitted = database
            .list_batch_candidates("batch-1", Some("SUBMITTED"), 10)
            .expect("submitted");
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].job_id.as_deref(), Some("job-100"));
    }

    /// Local Python is used exactly like the supervisor-crate tests do: a worker
    /// stub that streams protocol v2 lines. Without an interpreter the stream
    /// behavior stays covered by the supervisor crate, so the case is skipped
    /// rather than reported as a pass.
    const DISCOVERY_STUB_OK: &str = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] != 'discover':
        continue
    base = {'protocol_version': 2, 'job_id': command['job_id'], 'request_id': command['request_id']}
    print(json.dumps(dict(base, event='discovery_started')), flush=True)
    for tweet_id in ('100', '101'):
        candidate = {
            'tweet_id': tweet_id,
            'url': 'https://x.com/alice/status/' + tweet_id,
            'created_at': '2026-09-20T00:00:00Z',
            'tweet_type': 'post',
            'is_repost': False,
            'has_media': True,
            'media_count': 2,
            'user_id': '42',
            'username': 'alice',
        }
        print(json.dumps(dict(base, event='candidate', candidate=candidate)), flush=True)
    print(json.dumps(dict(base, event='discovery_completed', candidates_found=2)), flush=True)
    break
"#;

    const DISCOVERY_STUB_DELAYED: &str = r#"
import json, os, sys, time
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] != 'discover':
        continue
    base = {'protocol_version': 2, 'job_id': command['job_id'], 'request_id': command['request_id']}
    print(json.dumps(dict(base, event='discovery_started')), flush=True)
    candidate = {
        'tweet_id': '100',
        'url': 'https://x.com/alice/status/100',
        'created_at': '2026-09-20T00:00:00Z',
        'tweet_type': 'post',
        'is_repost': False,
        'has_media': True,
        'media_count': 2,
        'user_id': '42',
        'username': 'alice',
    }
    print(json.dumps(dict(base, event='candidate', candidate=candidate)), flush=True)
    deadline = time.monotonic() + 10
    while not os.path.exists(sys.argv[1]):
        if time.monotonic() >= deadline:
            raise RuntimeError('observer never acknowledged the persisted candidate')
        time.sleep(0.02)
    print(json.dumps(dict(base, event='discovery_completed', candidates_found=1)), flush=True)
    break
"#;

    const DISCOVERY_STUB_AUTH_FAILURE: &str = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] != 'discover':
        continue
    base = {'protocol_version': 2, 'job_id': command['job_id'], 'request_id': command['request_id']}
    print(json.dumps(dict(base, event='discovery_started')), flush=True)
    print(json.dumps(dict(base, event='failed', error_code='HTTP_401', error_message='login required')), flush=True)
    break
"#;

    fn python_stub(script: &str) -> Option<SidecarSupervisor> {
        let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_owned());
        SidecarSupervisor::spawn(&python, &["-c", script]).ok()
    }

    fn batch_request(database: &Database, batch_id: &str) -> DiscoveryRequest {
        discovery_request(&database.account_batch(batch_id).expect("get").expect("row"))
    }

    #[test]
    fn runs_discovery_and_persists_streamed_candidates() {
        let Some(mut supervisor) = python_stub(DISCOVERY_STUB_OK) else {
            return;
        };
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        let request = batch_request(&database, "batch-1");
        assert_eq!(request.request_id, "batch-discovery-batch-1");

        let summary = run_account_discovery(
            &database,
            &mut supervisor,
            &request,
            &CancellationToken::new(),
            Duration::from_secs(5),
        )
        .expect("discovery");
        assert_eq!(summary.candidates_found, 2);
        assert_eq!(summary.inserted, 2);
        assert_eq!(summary.user_id.as_deref(), Some("42"));

        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.discovery_state, "COMPLETED");
        assert_eq!(batch.last_error_code, None);
        assert_eq!(batch.user_id.as_deref(), Some("42"));
        assert_eq!(batch.counts.pending, 2);
        supervisor
            .shutdown()
            .expect("shutdown after batch assertions");
    }

    #[test]
    fn persists_each_discovery_candidate_before_completion() {
        let root = temp_root("stream-persist");
        std::fs::create_dir_all(&root).expect("root");
        let acknowledgement = root.join("candidate-observed");
        let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_owned());
        let Some(mut supervisor) = SidecarSupervisor::spawn(
            &python,
            &[
                "-c",
                DISCOVERY_STUB_DELAYED,
                acknowledgement.to_str().expect("ack path"),
            ],
        )
        .ok() else {
            let _ = std::fs::remove_dir_all(root);
            return;
        };
        let database_path = root.join("archive.sqlite3");
        let database = Database::open(&database_path).expect("database");
        open_batch(&database, "batch-1");
        let request = batch_request(&database, "batch-1");
        drop(database);

        let observer_path = database_path.clone();
        let observer_acknowledgement = acknowledgement.clone();
        let handle = std::thread::spawn(move || {
            let database = Database::open(&observer_path).expect("observer database");
            let deadline = std::time::Instant::now() + Duration::from_secs(8);
            loop {
                let candidates = database
                    .list_batch_candidates("batch-1", None, 10)
                    .expect("candidates");
                if candidates.len() == 1 {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "candidate was not streamed"
                );
                thread::sleep(Duration::from_millis(20));
            }
            let batch = database
                .account_batch("batch-1")
                .expect("batch")
                .expect("row");
            assert_eq!(batch.discovery_state, "RUNNING");
            std::fs::write(observer_acknowledgement, b"observed").expect("acknowledge candidate");
        });

        let database = Database::open(&database_path).expect("database");
        let summary = run_account_discovery(
            &database,
            &mut supervisor,
            &request,
            &CancellationToken::new(),
            Duration::from_secs(5),
        )
        .expect("discovery");
        assert_eq!(summary.inserted, 1);
        handle.join().expect("observer");
        supervisor
            .shutdown()
            .expect("shutdown after batch assertions");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn records_a_failed_discovery_on_the_batch() {
        let Some(mut supervisor) = python_stub(DISCOVERY_STUB_AUTH_FAILURE) else {
            return;
        };
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        let request = batch_request(&database, "batch-1");

        let error = run_account_discovery(
            &database,
            &mut supervisor,
            &request,
            &CancellationToken::new(),
            Duration::from_secs(5),
        )
        .expect_err("discovery must fail");
        assert_eq!(error, "HTTP_401: login required");

        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.discovery_state, "FAILED");
        assert_eq!(batch.last_error_code.as_deref(), Some("HTTP_401"));
        assert_eq!(
            batch.last_error_message.as_deref(),
            Some("HTTP_401: login required")
        );
        assert_eq!(batch.counts.total, 0);
        supervisor
            .shutdown()
            .expect("shutdown after batch assertions");
    }

    /// Persist the durable archive facts of one Tweet: user row, tweet row,
    /// committed directory, media row. `keep_media_file` controls whether the
    /// recorded media file still exists on disk after the commit.
    fn archive_tweet(
        database: &Database,
        files: &FileStore,
        tweet_id: &str,
        keep_media_file: bool,
    ) {
        let user_row_id = database
            .upsert_user("42", Some("alice"), Some("Alice"), NOW)
            .expect("user");
        let tweet_row_id = database
            .insert_tweet_for_user(
                tweet_id,
                &format!("https://x.com/alice/status/{tweet_id}"),
                "post",
                "archived text",
                Some(user_row_id),
                NOW,
            )
            .expect("tweet");
        let directory = format!("@alice - Alice [42]/tweet-{tweet_id}");
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            tweet_type: "post".to_owned(),
            author: xarchive_core::ArchiveAuthor {
                user_id: Some("42".to_owned()),
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
            },
            created_at: Some("2026-09-20T00:00:00Z".to_owned()),
            text: "archived text".to_owned(),
            media: vec![ArchiveMedia {
                index: 1,
                media_id: Some("m1".to_owned()),
                media_type: "photo".to_owned(),
                file: "media/1.jpg".to_owned(),
                mime_type: Some("image/jpeg".to_owned()),
                size_bytes: 4,
                sha256: "a".to_owned(),
            }],
            archived_at: NOW.to_owned(),
            reply_to: None,
            quoted_tweet: None,
        };
        database
            .update_tweet_metadata(tweet_row_id, &metadata, &directory)
            .expect("metadata");
        for media in &metadata.media {
            database
                .insert_media(tweet_row_id, media, NOW)
                .expect("media row");
        }
        if keep_media_file {
            let media_path = files
                .archive_path(&directory)
                .expect("archive path")
                .join("media/1.jpg");
            fs::create_dir_all(media_path.parent().expect("media dir")).expect("media dir");
            fs::write(&media_path, b"jpeg").expect("media file");
        }
    }

    #[test]
    fn skips_candidates_whose_archive_is_already_on_disk() {
        let root = temp_root("batch-skip");
        let files = FileStore::new(&root).expect("files");
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        persist_discovered_candidates(
            &database,
            "batch-1",
            &outcome(vec![
                candidate_full("100", Some("2026-09-20T00:00:00Z"), 1, false),
                candidate_full("101", Some("2026-09-21T00:00:00Z"), 1, false),
                candidate_full("102", Some("2026-09-22T00:00:00Z"), 1, false),
            ]),
        )
        .expect("persist");

        // 100: committed archive with its media file present.
        archive_tweet(&database, &files, "100", true);
        // 101: committed archive whose media file disappeared afterwards.
        archive_tweet(&database, &files, "101", false);
        // 102: no archive facts at all.

        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &BatchFilters::default(), 10)
                .expect("select");
        assert_eq!(selected_ids(&selected), ["102", "101"]);

        let counts = database.batch_candidate_counts("batch-1").expect("counts");
        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.pending, 2);

        let skipped = database
            .list_batch_candidates("batch-1", Some("SKIPPED"), 10)
            .expect("skipped");
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].tweet_id, "100");
        assert_eq!(
            skipped[0].skip_reason.as_deref(),
            Some(SKIP_ALREADY_ARCHIVED)
        );

        // A later round must not re-evaluate the skipped candidate.
        let second =
            select_dispatch_candidates(&database, &files, "batch-1", &BatchFilters::default(), 10)
                .expect("second select");
        assert_eq!(selected_ids(&second), ["102", "101"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn filters_and_batch_limit_record_every_exclusion_reason() {
        let root = temp_root("batch-filters");
        let files = FileStore::new(&root).expect("files");
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        persist_discovered_candidates(
            &database,
            "batch-1",
            &outcome(vec![
                candidate_full("200", Some("2026-09-20T00:00:00Z"), 0, false),
                candidate_full("201", Some("2026-09-21T00:00:00Z"), 1, true),
                candidate_full("202", Some("2026-09-22T00:00:00Z"), 1, false),
                candidate_full("203", Some("2026-09-23T00:00:00Z"), 1, false),
                candidate_full("204", Some("2026-09-24T00:00:00Z"), 1, false),
                candidate_full("205", Some("2026-09-25T00:00:00Z"), 1, false),
            ]),
        )
        .expect("persist");

        let filters = BatchFilters {
            limit: Some(2),
            ..BatchFilters::default()
        };
        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 10).expect("select");
        assert_eq!(selected_ids(&selected), ["205", "204"]);

        let counts = database.batch_candidate_counts("batch-1").expect("counts");
        assert_eq!(counts.skipped, 4);
        assert_eq!(counts.pending, 2);
        let skipped = database
            .list_batch_candidates("batch-1", Some("SKIPPED"), 10)
            .expect("skipped");
        let reasons = skipped
            .iter()
            .map(|candidate| (candidate.tweet_id.clone(), candidate.skip_reason.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            reasons,
            [
                ("200".to_owned(), Some(SKIP_NO_MEDIA.to_owned())),
                ("201".to_owned(), Some(SKIP_REPOST.to_owned())),
                ("202".to_owned(), Some(SKIP_BEYOND_LIMIT.to_owned())),
                ("203".to_owned(), Some(SKIP_BEYOND_LIMIT.to_owned())),
            ]
        );

        // The batch cap counts dispatched candidates, so submitting one of them
        // leaves exactly one slot for the next round.
        database
            .mark_batch_candidate(
                "batch-1",
                "205",
                "SUBMITTED",
                Some("job-205"),
                None,
                None,
                None,
            )
            .expect("submit");
        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 10).expect("second");
        assert_eq!(selected_ids(&selected), ["204"]);
        let counts = database.batch_candidate_counts("batch-1").expect("counts");
        assert_eq!(counts.skipped, 4);
        assert_eq!(counts.pending, 1);

        // The per-round bound only limits the returned list.
        let bounded =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 0).expect("bounded");
        assert!(bounded.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn date_filters_exclude_out_of_range_candidates_and_keep_unknown_dates() {
        let root = temp_root("batch-dates");
        let files = FileStore::new(&root).expect("files");
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        persist_discovered_candidates(
            &database,
            "batch-1",
            &outcome(vec![
                candidate_full("300", Some("2026-09-01T00:00:00Z"), 1, false),
                candidate_full("301", Some("2026-09-15T00:00:00Z"), 1, false),
                candidate_full("302", None, 1, false),
                candidate_full("303", Some("2026-09-25T00:00:00Z"), 1, false),
            ]),
        )
        .expect("persist");
        let filters = BatchFilters {
            since: Some("2026-09-10T00:00:00Z".to_owned()),
            until: Some("2026-09-20T00:00:00Z".to_owned()),
            ..BatchFilters::default()
        };
        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 10).expect("select");
        // `302` carries no timestamp: an unknown date is kept, never dropped.
        assert_eq!(selected_ids(&selected), ["301", "302"]);
        let skipped = database
            .list_batch_candidates("batch-1", Some("SKIPPED"), 10)
            .expect("skipped");
        let reasons = skipped
            .iter()
            .map(|candidate| (candidate.tweet_id.clone(), candidate.skip_reason.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            reasons,
            [
                ("300".to_owned(), Some(SKIP_BEFORE_RANGE.to_owned())),
                ("303".to_owned(), Some(SKIP_AFTER_RANGE.to_owned())),
            ]
        );
        let _ = fs::remove_dir_all(root);
    }
}
