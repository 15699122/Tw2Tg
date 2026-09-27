//! Executor regression tests.
//!
//! ENG-15: kept as one suite so the existing coverage keeps proving the same
//! behaviour after the module split.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use xarchive_core::{JobEvent, JobState};
use xarchive_storage::JobSummary;

use super::*;

fn request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
    ArchiveJobRequest {
        job_id: job_id.to_owned(),
        tweet_id: tweet_id.to_owned(),
        request_id: format!("test-request-{job_id}"),
        request_json: "{}".to_owned(),
    }
}

fn archive_request(tweet_id: &str) -> crate::ArchiveTweetRequest {
    crate::ArchiveTweetRequest {
        tweet: xarchive_protocol::BrowserTweet {
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            text: Some("archive me".to_owned()),
            created_at: Some("2026-09-12T00:00:00Z".to_owned()),
            tweet_type: "post".to_owned(),
            reply_to: None,
            quoted_tweet: None,
        },
        browser: None,
        profile: None,
    }
}

fn insert_downloaded_job<P: JobPersistence>(persistence: &mut P, job_id: &str, tweet_id: &str) {
    persistence
        .create_or_reuse(&request(job_id, tweet_id))
        .expect("create job");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
        JobState::Downloaded,
    ] {
        persistence
            .persist_state(&JobSnapshot {
                job_id: job_id.into(),
                tweet_id: tweet_id.into(),
                state,
            })
            .expect("advance job state");
    }
}

fn temporary_database_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "xarchive-executor-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ))
        .join("config")
        .join("archive.sqlite3")
}

fn wait_for_persisted_state(database_path: &Path, job_id: &str, expected: JobState) -> JobSnapshot {
    for _ in 0..100 {
        if let Ok(persistence) = StorageJobPersistence::open(database_path)
            && let Ok(snapshot) = persistence.snapshot(job_id)
            && snapshot.state == expected
        {
            return snapshot;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("Job {job_id} did not reach {expected:?}");
}

#[derive(Debug)]
struct FakeJobExecution {
    result: Result<JobExecutionResult, JobExecutionError>,
    calls: Vec<String>,
}

impl FakeJobExecution {
    fn success(files_copied: u32, archive_directory: &str) -> Self {
        Self {
            result: Ok(JobExecutionResult {
                files_copied,
                archive_directory: archive_directory.to_owned(),
                download_event_recorded: false,
                state_already_updated: false,
            }),
            calls: Vec::new(),
        }
    }

    fn failure(error_code: &str, error_message: &str) -> Self {
        Self {
            result: Err(JobExecutionError {
                error_code: error_code.to_owned(),
                error_message: error_message.to_owned(),
                persistence_already_updated: false,
            }),
            calls: Vec::new(),
        }
    }
}

impl JobExecution for FakeJobExecution {
    fn execute(
        &mut self,
        job: &JobSnapshot,
        _cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        self.calls.push(job.job_id.clone());
        self.result.clone()
    }
}

struct CancellationAwareExecution;

impl JobExecution for CancellationAwareExecution {
    fn execute(
        &mut self,
        _job: &JobSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        while !cancellation.is_cancelled() {
            thread::sleep(std::time::Duration::from_millis(2));
        }
        Ok(JobExecutionResult {
            files_copied: 1,
            archive_directory: "archives/cancelled-late-result".to_owned(),
            download_event_recorded: false,
            state_already_updated: false,
        })
    }
}

#[test]
fn submits_and_reuses_an_active_job_for_duplicate_tweets() {
    let executor = JobExecutor::new();
    let handle = executor.handle();
    let first = handle.submit(request("job-1", "tweet-1")).unwrap();
    let second = handle.submit(request("job-2", "tweet-1")).unwrap();

    assert!(first.created);
    assert!(!second.created);
    assert_eq!(second.job.job_id, "job-1");
    assert_eq!(second.job.state, JobState::Queued);
}

#[test]
fn cancel_is_idempotent_and_does_not_change_terminal_state() {
    let executor = JobExecutor::new();
    let handle = executor.handle();
    handle.submit(request("job-1", "tweet-1")).unwrap();

    let cancelled = handle.cancel("job-1").unwrap();
    assert_eq!(cancelled.state, JobState::Interrupted);
    assert_eq!(handle.cancel("job-1").unwrap(), cancelled);
}

#[test]
fn running_cancel_signals_execution_and_fences_late_success() {
    let executor = Arc::new(JobExecutor::new());
    let handle = executor.handle();
    handle
        .submit(request("job-cancel-running", "tweet-cancel-running"))
        .unwrap();
    let worker_handle = handle.clone();
    let execution = thread::spawn(move || {
        worker_handle
            .execute("job-cancel-running", Box::new(CancellationAwareExecution))
            .expect("execution response")
    });

    thread::sleep(std::time::Duration::from_millis(10));
    let cancelled = handle.cancel("job-cancel-running").expect("cancel");
    let late_result = execution.join().expect("execution thread");

    assert_eq!(cancelled.state, JobState::Interrupted);
    assert_eq!(
        late_result.archive_directory,
        "archives/cancelled-late-result"
    );
    assert_eq!(
        handle.snapshot("job-cancel-running").unwrap().state,
        JobState::Interrupted
    );
    drop(executor);
}

#[test]
fn shutdown_interrupts_active_jobs_and_rejects_later_commands() {
    let executor = JobExecutor::new();
    let handle = executor.handle();
    handle.submit(request("job-1", "tweet-1")).unwrap();
    handle.submit(request("job-2", "tweet-2")).unwrap();

    executor.shutdown().unwrap();
    assert_eq!(handle.snapshot("job-1").unwrap_err(), ExecutorError::Closed);
}

#[test]
fn concurrent_submissions_keep_one_active_worker_per_tweet() {
    let executor = Arc::new(JobExecutor::with_capacity(16));
    let mut workers = Vec::new();
    for index in 0..8 {
        let executor = Arc::clone(&executor);
        workers.push(thread::spawn(move || {
            executor
                .handle()
                .submit(request(&format!("job-{index}"), "tweet-1"))
                .unwrap()
        }));
    }
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.created).count(), 1);
}

#[test]
fn start_preserves_job_state_machine_boundaries() {
    let executor = JobExecutor::new();
    let handle = executor.handle();
    handle.submit(request("job-1", "tweet-1")).unwrap();
    assert_eq!(handle.start("job-1").unwrap().state, JobState::Validating);
    assert_eq!(
        handle.start("job-1").unwrap_err(),
        ExecutorError::InvalidTransition {
            from: JobState::Validating,
            to: JobState::Validating,
        }
    );
}

#[test]
fn application_service_delegates_submit_query_cancel_and_shutdown() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let submitted = service.submit(request("job-1", "tweet-1")).expect("submit");
    assert!(submitted.created);
    assert_eq!(service.query("job-1").unwrap(), submitted.job);
    assert_eq!(
        service.cancel("job-1").unwrap().state,
        JobState::Interrupted
    );
    service.shutdown().expect("shutdown");
}

#[test]
fn recovery_skips_terminal_and_duplicate_jobs_and_restarts_interrupted_jobs() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    service.submit(request("job-1", "tweet-1")).unwrap();

    let recovered = service
        .recover(vec![
            JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Interrupted,
            },
            JobSnapshot {
                job_id: "job-2".into(),
                tweet_id: "tweet-2".into(),
                state: JobState::Interrupted,
            },
            JobSnapshot {
                job_id: "job-3".into(),
                tweet_id: "tweet-3".into(),
                state: JobState::Complete,
            },
        ])
        .unwrap();

    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].job_id, "job-2");
    assert_eq!(recovered[0].state, JobState::Validating);
    assert_eq!(
        service.query("job-3"),
        Err(ExecutorError::UnknownJob("job-3".into()))
    );
}

#[test]
fn persisted_submit_reuses_database_like_active_job_semantics() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();

    let first = service
        .submit_persisted(&mut persistence, request("job-1", "tweet-1"))
        .unwrap();
    let duplicate = service
        .submit_persisted(&mut persistence, request("job-2", "tweet-1"))
        .unwrap();

    assert!(first.created);
    assert!(!duplicate.created);
    assert_eq!(duplicate.job.job_id, "job-1");
    assert_eq!(
        service.query_persisted(&persistence, "job-1").unwrap(),
        first.job
    );
    assert_eq!(
        persistence.events("job-1").unwrap(),
        vec![
            ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            },
            ExecutorEvent::Reused {
                job_id: "job-1".into(),
            },
        ]
    );
}

#[test]
fn execute_persisted_records_successful_lifecycle_and_completion_order() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-execute", "tweet-execute"))
        .unwrap();
    let mut execution = FakeJobExecution::success(3, "archives/tweet-execute");

    let completed = service
        .execute_persisted(&mut persistence, &mut execution, "job-execute")
        .unwrap();

    assert_eq!(completed.state, JobState::Complete);
    assert_eq!(execution.calls, vec!["job-execute"]);
    assert_eq!(
        persistence.events("job-execute").unwrap(),
        vec![
            ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Validating,
            },
            ExecutorEvent::StateChanged {
                from: JobState::Validating,
                to: JobState::MetadataReady,
            },
            ExecutorEvent::StateChanged {
                from: JobState::MetadataReady,
                to: JobState::Downloading,
            },
            ExecutorEvent::StateChanged {
                from: JobState::Downloading,
                to: JobState::Downloaded,
            },
            ExecutorEvent::DownloadCompleted { files_copied: 3 },
            ExecutorEvent::Completed {
                archive_directory: "archives/tweet-execute".into(),
            },
        ]
    );
}

#[test]
fn execute_persisted_records_failure_without_polluting_other_jobs() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-failed", "tweet-failed"))
        .unwrap();
    persistence
        .create_or_reuse(&request("job-other", "tweet-other"))
        .unwrap();
    let mut execution = FakeJobExecution::failure("AUTH_REQUIRED", "login required");

    let failed = service
        .execute_persisted(&mut persistence, &mut execution, "job-failed")
        .unwrap();

    assert_eq!(failed.state, JobState::Failed);
    assert_eq!(execution.calls, vec!["job-failed"]);
    assert_eq!(
        persistence.snapshot("job-other").unwrap().state,
        JobState::Queued
    );
    assert!(matches!(
        persistence.events("job-failed").unwrap().last(),
        Some(ExecutorEvent::DownloadFailed { error_code, .. })
            if error_code == "AUTH_REQUIRED"
    ));
}

#[test]
fn execute_persisted_skips_terminal_jobs_without_calling_execution_port() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-terminal", "tweet-terminal"))
        .unwrap();
    insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Complete,
        })
        .unwrap();
    let before = persistence.events("job-terminal").unwrap();
    let mut execution = FakeJobExecution::success(1, "archives/terminal");

    let result = service
        .execute_persisted(&mut persistence, &mut execution, "job-terminal")
        .unwrap();

    assert_eq!(result.state, JobState::Complete);
    assert!(execution.calls.is_empty());
    assert_eq!(persistence.events("job-terminal").unwrap(), before);
}

#[test]
fn persisted_submit_marks_created_job_failed_when_executor_is_closed() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    executor.shutdown().unwrap();

    let error = service
        .submit_persisted(&mut persistence, request("job-closed", "tweet-closed"))
        .expect_err("closed executor");

    assert_eq!(error, ExecutorError::Closed);
    assert_eq!(
        persistence.snapshot("job-closed").unwrap().state,
        JobState::Failed
    );
    assert!(persistence.list_recovery_candidates().is_empty());
    assert!(matches!(
        persistence.events("job-closed").unwrap().as_slice(),
        [ExecutorEvent::DownloadFailed { error_code, .. }] if error_code == "EXECUTOR_UNAVAILABLE"
    ));
}

#[test]
fn submit_and_schedule_persists_background_executor_failure() {
    let database_path = temporary_database_path("background-failure");
    std::fs::create_dir_all(database_path.parent().expect("database parent")).expect("parent");
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);

    let submitted = service
        .submit_and_schedule_persisted(
            &mut persistence,
            request("job-background-failure", "tweet-background-failure"),
            database_path.clone(),
        )
        .expect("submit and schedule");

    assert!(submitted.created);
    assert_eq!(submitted.job.state, JobState::Queued);

    let failed =
        wait_for_persisted_state(&database_path, "job-background-failure", JobState::Failed);
    assert_eq!(failed.tweet_id, "tweet-background-failure");

    let persisted = StorageJobPersistence::open(&database_path).expect("reopen database");
    assert!(matches!(
        persisted
            .stored_events("job-background-failure")
            .expect("events")
            .last(),
        Some((event_type, payload))
            if event_type == "DOWNLOAD_FAILED"
                && payload.as_deref().is_some_and(|payload| {
                    payload.contains("EXECUTOR_WORKER_FAILED")
                })
    ));

    let _ = std::fs::remove_dir_all(database_path.parent().unwrap().parent().unwrap());
}

#[test]
fn submit_and_schedule_marks_job_failed_when_database_path_is_invalid() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    let invalid_database_path = std::env::temp_dir()
        .join(format!("xarchive-missing-parent-{}", std::process::id()))
        .join("missing")
        .join("archive.sqlite3");

    let error = service
        .submit_and_schedule_persisted(
            &mut persistence,
            request("job-invalid-database", "tweet-invalid-database"),
            invalid_database_path,
        )
        .expect_err("invalid database path");

    assert!(matches!(
        error,
        ExecutorError::Execution { error_code, .. }
            if error_code == "EXECUTOR_SCHEDULE_FAILED"
    ));
    assert_eq!(
        persistence
            .snapshot("job-invalid-database")
            .expect("snapshot")
            .state,
        JobState::Failed
    );
    assert!(matches!(
        persistence.events("job-invalid-database").expect("events").last(),
        Some(ExecutorEvent::DownloadFailed { error_code, .. })
            if error_code == "EXECUTOR_SCHEDULE_FAILED"
    ));
}

#[test]
fn archive_submission_adapter_preserves_sync_job_identity_and_reuse_contract() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let adapter = ArchiveJobSubmissionAdapter;
    let mut persistence = InMemoryJobPersistence::default();
    let request = archive_request("123");

    let first = adapter
        .submit(&service, &mut persistence, &request, "2026-09-13T00:00:00Z")
        .expect("submit");
    let duplicate = adapter
        .submit(&service, &mut persistence, &request, "2026-09-13T00:00:01Z")
        .expect("reuse");

    assert!(first.created);
    assert_eq!(first.job.job_id, "archive-123-2026-09-13T00:00:00Z");
    assert!(!duplicate.created);
    assert_eq!(duplicate.job, first.job);
    assert_eq!(
        adapter
            .query(&service, &persistence, &first.job.job_id)
            .expect("query"),
        first.job
    );
}

#[test]
fn archive_submission_adapter_rejects_invalid_browser_requests_before_submit() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let adapter = ArchiveJobSubmissionAdapter;
    let mut persistence = InMemoryJobPersistence::default();
    let mut request = archive_request("123");
    request.tweet.tweet_id = "not-a-tweet-id".to_owned();
    request.tweet.url = "https://example.com/not-x".to_owned();

    let error = adapter
        .submit(&service, &mut persistence, &request, "2026-09-13T00:00:00Z")
        .expect_err("invalid request");
    assert!(matches!(error, ExecutorError::Persistence(message) if message.contains("tweet_id")));
    assert!(persistence.list_recovery_candidates().is_empty());
}

#[test]
fn persisted_recovery_updates_only_recoverable_candidates() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .create_or_reuse(&request("job-2", "tweet-2"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-2".into(),
            tweet_id: "tweet-2".into(),
            state: JobState::Interrupted,
        })
        .unwrap();

    let recovered = service.recover_persisted(&mut persistence).unwrap();
    assert_eq!(recovered.len(), 2);
    assert!(
        recovered
            .iter()
            .all(|job| job.state == JobState::Validating)
    );
    assert_eq!(
        service
            .query_persisted(&persistence, "job-2")
            .unwrap()
            .state,
        JobState::Validating
    );
    assert_eq!(
        persistence.events("job-2").unwrap(),
        vec![ExecutorEvent::Recovered {
            from: JobState::Interrupted,
            to: JobState::Validating,
        }]
    );
}

#[test]
fn persisted_cancel_preserves_queued_to_interrupted_transition_and_is_idempotent() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    executor
        .handle()
        .submit(request("job-1", "tweet-1"))
        .unwrap();

    let cancelled = service.cancel_persisted(&mut persistence, "job-1").unwrap();
    assert_eq!(cancelled.state, JobState::Interrupted);
    assert_eq!(persistence.snapshot("job-1").unwrap(), cancelled);
    assert_eq!(
        persistence.events("job-1").unwrap(),
        vec![ExecutorEvent::StateChanged {
            from: JobState::Queued,
            to: JobState::Interrupted,
        }]
    );

    assert_eq!(
        service.cancel_persisted(&mut persistence, "job-1").unwrap(),
        cancelled
    );
    assert_eq!(persistence.events("job-1").unwrap().len(), 1);
}

#[test]
fn persisted_cancel_does_not_change_terminal_job_or_record_an_event() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Complete,
        })
        .unwrap();
    let before = persistence.events("job-1").unwrap();

    let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
    assert_eq!(result.state, JobState::Complete);
    assert_eq!(persistence.events("job-1").unwrap(), before);
}

#[test]
fn persisted_shutdown_interrupts_active_jobs_once_and_skips_terminal_jobs() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-active", "tweet-active"))
        .unwrap();
    persistence
        .create_or_reuse(&request("job-terminal", "tweet-terminal"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Complete,
        })
        .unwrap();
    let terminal_events = persistence.events("job-terminal").unwrap();

    service.shutdown_persisted(&mut persistence).unwrap();

    assert_eq!(
        persistence.snapshot("job-active").unwrap().state,
        JobState::Interrupted
    );
    assert_eq!(
        persistence.events("job-active").unwrap(),
        vec![
            ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Interrupted,
            },
            ExecutorEvent::ShutdownInterrupted {
                from: JobState::Queued,
                to: JobState::Interrupted,
            },
        ]
    );
    assert_eq!(persistence.events("job-terminal").unwrap(), terminal_events);
}

#[test]
fn interrupt_persisted_is_separate_from_worker_shutdown() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    service
        .submit(request("job-active", "tweet-active"))
        .unwrap();
    persistence
        .create_or_reuse(&request("job-active", "tweet-active"))
        .unwrap();

    service.interrupt_persisted(&mut persistence).unwrap();

    assert_eq!(
        persistence.snapshot("job-active").unwrap().state,
        JobState::Interrupted
    );
    assert!(service.query("job-active").is_ok());
    service.shutdown().unwrap();
}

#[test]
fn persisted_completion_requires_downloaded_state_and_records_completion_once() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();

    let completed = service
        .complete_persisted(&mut persistence, "job-1", "archives/123")
        .unwrap();
    assert_eq!(completed.state, JobState::Complete);
    assert_eq!(persistence.snapshot("job-1").unwrap(), completed);
    assert_eq!(
        persistence.events("job-1").unwrap().last(),
        Some(&ExecutorEvent::Completed {
            archive_directory: "archives/123".into(),
        })
    );

    let events_before_retry = persistence.events("job-1").unwrap();
    assert_eq!(
        service
            .complete_persisted(&mut persistence, "job-1", "archives/123")
            .unwrap(),
        completed
    );
    assert_eq!(persistence.events("job-1").unwrap(), events_before_retry);
}

#[test]
fn persisted_completion_rejects_queued_job_without_writing_state_or_event() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    let before = persistence.events("job-1").unwrap();

    assert_eq!(
        service.complete_persisted(&mut persistence, "job-1", "archives/123"),
        Err(ExecutorError::InvalidTransition {
            from: JobState::Queued,
            to: JobState::Complete,
        })
    );
    assert_eq!(
        persistence.snapshot("job-1").unwrap().state,
        JobState::Queued
    );
    assert_eq!(persistence.events("job-1").unwrap(), before);
}

#[test]
fn commit_recovery_decision_distinguishes_resume_complete_failure_and_skip() {
    let resume = CommitRecoveryFacts {
        job_id: "job-resume".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Present,
    };
    assert_eq!(
        resume.decide("archives/123"),
        CommitRecoveryDecision::ResumeStaging
    );

    let complete = CommitRecoveryFacts {
        job_id: "job-complete".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    };
    assert_eq!(
        complete.decide("archives/123"),
        CommitRecoveryDecision::Complete {
            archive_directory: "archives/123".into(),
        }
    );

    let incomplete = CommitRecoveryFacts {
        job_id: "job-incomplete".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Missing,
    };
    assert!(matches!(
        incomplete.decide("archives/123"),
        CommitRecoveryDecision::MarkFailed { error_code, .. }
            if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
    ));

    let already_complete = CommitRecoveryFacts {
        job_id: "job-done".into(),
        state: JobState::Complete,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    };
    assert_eq!(
        already_complete.decide("archives/123"),
        CommitRecoveryDecision::Skip
    );
}

#[test]
fn commit_recovery_decision_detects_missing_archive_for_complete_job() {
    let facts = CommitRecoveryFacts {
        job_id: "job-done".into(),
        state: JobState::Complete,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Missing,
    };
    assert!(matches!(
        facts.decide("archives/123"),
        CommitRecoveryDecision::MarkFailed { error_code, error_message }
            if error_code == "ARCHIVE_COMMIT_MISSING"
                && error_message.contains("job-done")
    ));
}

#[test]
fn commit_recovery_does_not_rewrite_unrelated_job_states() {
    let facts = CommitRecoveryFacts {
        job_id: "job-active".into(),
        state: JobState::Downloading,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Present,
    };
    assert_eq!(
        facts.decide("archives/123"),
        CommitRecoveryDecision::NotApplicable
    );
}

#[test]
fn apply_commit_recovery_resumes_staging_without_fabricating_completion() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    let before = persistence.events("job-1").unwrap();
    let facts = CommitRecoveryFacts {
        job_id: "job-1".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Present,
    };

    let result = service
        .apply_commit_recovery(&mut persistence, &facts, "archives/123")
        .unwrap();
    assert_eq!(result.state, JobState::Downloaded);
    assert_eq!(persistence.events("job-1").unwrap(), before);
}

#[test]
fn apply_commit_recovery_completes_existing_final_directory() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    let facts = CommitRecoveryFacts {
        job_id: "job-1".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    };

    let result = service
        .apply_commit_recovery(&mut persistence, &facts, "archives/123")
        .unwrap();
    assert_eq!(result.state, JobState::Complete);
    assert!(matches!(
        persistence.events("job-1").unwrap().last(),
        Some(ExecutorEvent::Completed { archive_directory })
            if archive_directory == "archives/123"
    ));
}

#[test]
fn apply_commit_recovery_marks_missing_commit_as_failed_and_records_error() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    let facts = CommitRecoveryFacts {
        job_id: "job-1".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Missing,
    };

    let result = service
        .apply_commit_recovery(&mut persistence, &facts, "archives/123")
        .unwrap();
    assert_eq!(result.state, JobState::Failed);
    let summary = persistence.stored_job_summary("job-1").unwrap();
    assert_eq!(
        summary.last_error_code.as_deref(),
        Some("ARCHIVE_COMMIT_INCOMPLETE")
    );
    assert!(matches!(
        persistence.events("job-1").unwrap().last(),
        Some(ExecutorEvent::DownloadFailed { error_code, .. })
            if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
    ));
}

#[test]
fn apply_commit_recovery_skips_complete_job_without_duplicate_event() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Complete,
        })
        .unwrap();
    let before = persistence.events("job-1").unwrap();
    let facts = CommitRecoveryFacts {
        job_id: "job-1".into(),
        state: JobState::Complete,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    };

    let result = service
        .apply_commit_recovery(&mut persistence, &facts, "archives/123")
        .unwrap();
    assert_eq!(result.state, JobState::Complete);
    assert_eq!(persistence.events("job-1").unwrap(), before);
}

#[test]
fn recover_commit_persisted_uses_injected_directory_facts_and_matches_snapshot() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();

    let mut provider = InMemoryCommitRecoveryFactsProvider::default();
    provider.insert(CommitRecoveryFacts {
        job_id: "job-1".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    });

    let result = service
        .recover_commit_persisted(&mut persistence, &provider, "job-1", "archives/123")
        .unwrap();
    assert_eq!(result.state, JobState::Complete);
}

#[test]
fn recover_commit_persisted_rejects_facts_that_do_not_match_snapshot() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();

    let mut provider = InMemoryCommitRecoveryFactsProvider::default();
    provider.insert(CommitRecoveryFacts {
        job_id: "job-1".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    });

    let error = service
        .recover_commit_persisted(&mut persistence, &provider, "job-1", "archives/123")
        .expect_err("mismatched facts");
    assert!(
        matches!(error, ExecutorError::Persistence(message) if message.contains("do not match"))
    );
    assert_eq!(
        persistence.snapshot("job-1").unwrap().state,
        JobState::Queued
    );
}

#[test]
fn storage_recovery_facts_preserve_job_summary_state_and_injected_directories() {
    let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();

    let facts = persistence
        .recovery_facts(
            "job-1",
            CommitRecoveryDirectory::Present,
            CommitRecoveryDirectory::Missing,
        )
        .unwrap();
    assert_eq!(facts.job_id, "job-1");
    assert_eq!(facts.state, JobState::Downloaded);
    assert_eq!(facts.final_directory, CommitRecoveryDirectory::Present);
    assert_eq!(facts.staging_directory, CommitRecoveryDirectory::Missing);
}

#[test]
fn recover_commits_persisted_keeps_results_sorted_and_isolates_fact_errors() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    for job_id in ["job-b", "job-a"] {
        persistence
            .create_or_reuse(&request(job_id, &format!("tweet-{job_id}")))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: job_id.into(),
                tweet_id: format!("tweet-{job_id}"),
                state: JobState::Downloaded,
            })
            .unwrap();
    }

    let mut provider = InMemoryCommitRecoveryFactsProvider::default();
    provider.insert(CommitRecoveryFacts {
        job_id: "job-a".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    });

    let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
    assert_eq!(
        results
            .iter()
            .map(|result| result.job_id.as_str())
            .collect::<Vec<_>>(),
        vec!["job-a", "job-b"]
    );
    assert_eq!(
        results[0].result.as_ref().unwrap().state,
        JobState::Complete
    );
    assert!(matches!(
        &results[1].result,
        Err(ExecutorError::UnknownJob(job_id)) if job_id == "job-b"
    ));
    assert_eq!(
        persistence.snapshot("job-a").unwrap().state,
        JobState::Complete
    );
    assert_eq!(
        persistence.snapshot("job-b").unwrap().state,
        JobState::Downloaded
    );
}

#[test]
fn recover_commits_persisted_skips_terminal_jobs_from_candidate_scan() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-complete", "tweet-complete"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-complete".into(),
            tweet_id: "tweet-complete".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-complete".into(),
            tweet_id: "tweet-complete".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-complete".into(),
            tweet_id: "tweet-complete".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-complete".into(),
            tweet_id: "tweet-complete".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-complete".into(),
            tweet_id: "tweet-complete".into(),
            state: JobState::Complete,
        })
        .unwrap();

    let provider = InMemoryCommitRecoveryFactsProvider::default();
    let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
    assert!(results.is_empty());
}

#[test]
fn recover_commits_persisted_mixes_actions_and_preserves_each_job_event_order() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    insert_downloaded_job(&mut persistence, "job-resume", "tweet-resume");
    insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
    insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
    insert_downloaded_job(&mut persistence, "job-missing", "tweet-missing");
    insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Complete,
        })
        .unwrap();

    let mut provider = InMemoryCommitRecoveryFactsProvider::default();
    provider.insert(CommitRecoveryFacts {
        job_id: "job-resume".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Present,
    });
    provider.insert(CommitRecoveryFacts {
        job_id: "job-complete".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    });
    provider.insert(CommitRecoveryFacts {
        job_id: "job-failed".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Missing,
    });
    provider.insert(CommitRecoveryFacts {
        job_id: "job-terminal".into(),
        state: JobState::Complete,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    });

    let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
    assert_eq!(
        results
            .iter()
            .map(|result| result.job_id.as_str())
            .collect::<Vec<_>>(),
        vec!["job-complete", "job-failed", "job-missing", "job-resume"]
    );
    assert_eq!(
        results[0].result.as_ref().unwrap().state,
        JobState::Complete
    );
    assert_eq!(results[1].result.as_ref().unwrap().state, JobState::Failed);
    assert!(matches!(
        &results[2].result,
        Err(ExecutorError::UnknownJob(job_id)) if job_id == "job-missing"
    ));
    assert_eq!(
        results[3].result.as_ref().unwrap().state,
        JobState::Downloaded
    );

    assert!(matches!(
        persistence.events("job-complete").unwrap().last(),
        Some(ExecutorEvent::Completed { archive_directory })
            if archive_directory == "archives"
    ));
    assert!(matches!(
        persistence.events("job-failed").unwrap().last(),
        Some(ExecutorEvent::DownloadFailed { error_code, .. })
            if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
    ));
    assert!(persistence.events("job-resume").unwrap().is_empty());
    assert_eq!(
        persistence.snapshot("job-terminal").unwrap().state,
        JobState::Complete
    );
}

#[test]
fn storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
    insert_downloaded_job(&mut persistence, "job-resume", "tweet-resume");
    insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
    insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
    insert_downloaded_job(&mut persistence, "job-missing", "tweet-missing");
    insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Complete,
        })
        .unwrap();

    let mut provider = InMemoryCommitRecoveryFactsProvider::default();
    provider.insert(CommitRecoveryFacts {
        job_id: "job-resume".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Present,
    });
    provider.insert(CommitRecoveryFacts {
        job_id: "job-complete".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    });
    provider.insert(CommitRecoveryFacts {
        job_id: "job-failed".into(),
        state: JobState::Downloaded,
        final_directory: CommitRecoveryDirectory::Missing,
        staging_directory: CommitRecoveryDirectory::Missing,
    });
    provider.insert(CommitRecoveryFacts {
        job_id: "job-terminal".into(),
        state: JobState::Complete,
        final_directory: CommitRecoveryDirectory::Present,
        staging_directory: CommitRecoveryDirectory::Missing,
    });

    let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
    assert_eq!(
        results
            .iter()
            .map(|result| result.job_id.as_str())
            .collect::<Vec<_>>(),
        vec!["job-complete", "job-failed", "job-missing", "job-resume"]
    );

    let complete_events = persistence.stored_events("job-complete").unwrap();
    assert_eq!(complete_events.last().unwrap().0, "JOB_COMPLETED");
    assert_eq!(
        persistence.snapshot("job-complete").unwrap().state,
        JobState::Complete
    );

    let failed_events = persistence.stored_events("job-failed").unwrap();
    assert_eq!(failed_events.last().unwrap().0, "DOWNLOAD_FAILED");
    let failed_summary = persistence.stored_job_summary("job-failed").unwrap();
    assert_eq!(failed_summary.state, JobState::Failed);
    assert_eq!(
        failed_summary.last_error_code.as_deref(),
        Some("ARCHIVE_COMMIT_INCOMPLETE")
    );
    assert!(
        failed_summary
            .last_error_message
            .as_deref()
            .is_some_and(|message| message.contains("job-failed"))
    );

    assert!(persistence.stored_events("job-resume").unwrap().len() >= 4);
    assert_eq!(
        persistence.snapshot("job-resume").unwrap().state,
        JobState::Downloaded
    );
    let missing_events = persistence.stored_events("job-missing").unwrap();
    assert!(missing_events.len() >= 4);
    assert!(
        missing_events
            .iter()
            .all(|(event_type, _)| event_type == "JOB_STATE_CHANGED")
    );
    assert_eq!(
        persistence.snapshot("job-missing").unwrap().state,
        JobState::Downloaded
    );
    assert_eq!(
        persistence.snapshot("job-terminal").unwrap().state,
        JobState::Complete
    );

    let terminal_events = persistence.stored_events("job-terminal").unwrap();
    assert!(terminal_events.len() >= 5);
    assert!(
        terminal_events
            .iter()
            .all(|(event_type, _)| event_type != "JOB_COMPLETED")
    );
}

#[test]
fn storage_completion_persists_state_then_job_completed_event() {
    let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    persistence
        .record_event(
            "job-1",
            ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            },
        )
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::MetadataReady,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloading,
        })
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Downloaded,
        })
        .unwrap();

    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    service
        .complete_persisted(&mut persistence, "job-1", "archives/123")
        .unwrap();
    let events = persistence.stored_events("job-1").unwrap();
    assert_eq!(events.last().unwrap().0, "JOB_COMPLETED");
    assert_eq!(
        persistence.snapshot("job-1").unwrap().state,
        JobState::Complete
    );
}

#[test]
fn storage_persistence_adapter_matches_job_repository_contract() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    let first = persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create");
    let duplicate = persistence
        .create_or_reuse(&request("job-2", "tweet-1"))
        .expect("reuse");

    assert!(first.created);
    assert!(!duplicate.created);
    assert_eq!(duplicate.job.job_id, "job-1");
    assert_eq!(
        persistence.snapshot("job-1").expect("snapshot").state,
        JobState::Queued
    );

    persistence
        .record_event(
            "job-1",
            ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            },
        )
        .expect("event");
    assert_eq!(
        persistence.events("job-1").expect("events"),
        vec![ExecutorEvent::Submitted {
            job_id: "job-1".into(),
        }]
    );
}

#[test]
fn database_factory_opens_a_job_context_without_runtime_state() {
    let factory = InMemoryJobDatabaseFactory;
    let mut persistence = StorageJobPersistence::from_factory(&factory, "job-1").unwrap();
    let result = persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .unwrap();
    assert!(result.created);
    assert_eq!(
        persistence.snapshot("job-1").unwrap().state,
        JobState::Queued
    );
}

#[test]
fn storage_persistence_preserves_state_transition_and_created_event() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create");
    persistence
        .record_event(
            "job-1",
            ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            },
        )
        .expect("created event");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .expect("transition");
    persistence
        .record_event(
            "job-1",
            ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Validating,
            },
        )
        .expect("state event");

    let events = persistence.stored_events("job-1").expect("stored events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].0, "JOB_CREATED");
    assert_eq!(events[1].0, "JOB_STATE_CHANGED");
    assert!(events[0].1.is_some());
    assert!(events[1].1.is_none());
    assert_eq!(
        persistence.snapshot("job-1").expect("snapshot").state,
        JobState::Validating
    );
}

#[test]
fn executor_lifecycle_events_match_existing_archive_job_event_types() {
    let cases = [
        (
            ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            },
            "JOB_CREATED",
        ),
        (
            ExecutorEvent::DownloadStarted {
                backend: "gallery-dl".into(),
            },
            "DOWNLOAD_STARTED",
        ),
        (
            ExecutorEvent::DownloadCompleted { files_copied: 2 },
            "DOWNLOAD_COMPLETED",
        ),
        (
            ExecutorEvent::DownloadFailed {
                error_code: "DOWNLOAD_FAILED".into(),
                error_message: "network error".into(),
            },
            "DOWNLOAD_FAILED",
        ),
        (
            ExecutorEvent::Completed {
                archive_directory: "archives/123".into(),
            },
            "JOB_COMPLETED",
        ),
    ];

    for (event, expected_type) in cases {
        let job_event = event.to_job_event("job-1").expect("mapped event");
        assert_eq!(job_event.event_type(), expected_type);
    }
    assert!(
        ExecutorEvent::Reused {
            job_id: "job-1".into()
        }
        .to_job_event("job-1")
        .is_none()
    );
}

#[test]
fn storage_persistence_does_not_duplicate_transactional_state_events() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create");
    persistence
        .record_event(
            "job-1",
            ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            },
        )
        .expect("created event");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Validating,
        })
        .expect("transition");
    persistence
        .record_event(
            "job-1",
            ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Validating,
            },
        )
        .expect("lifecycle event");

    let events = persistence.stored_events("job-1").expect("events");
    assert_eq!(
        events
            .iter()
            .map(|(event_type, _)| event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["JOB_CREATED", "JOB_STATE_CHANGED"]
    );
}

#[test]
fn job_summary_projection_preserves_executor_fields_without_synthesizing_storage_fields() {
    let summary = JobSummary {
        job_id: "job-1".into(),
        tweet_id: "tweet-1".into(),
        tweet_type: "quote".into(),
        state: JobState::Failed,
        created_at: "2026-09-13T00:00:00Z".into(),
        updated_at: "2026-09-13T00:01:00Z".into(),
        last_error_code: Some("DOWNLOAD_FAILED".into()),
        last_error_message: Some("network error".into()),
    };

    assert_eq!(
        JobSnapshot::from_job_summary(&summary),
        JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            state: JobState::Failed,
        }
    );
    assert_eq!(summary.tweet_type, "quote");
    assert_eq!(summary.created_at, "2026-09-13T00:00:00Z");
    assert_eq!(summary.updated_at, "2026-09-13T00:01:00Z");
    assert_eq!(summary.last_error_code.as_deref(), Some("DOWNLOAD_FAILED"));
    assert_eq!(summary.last_error_message.as_deref(), Some("network error"));
}

#[test]
fn storage_summary_and_event_queries_preserve_error_and_nullable_payload_contracts() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    persistence
        .create_or_reuse(&request("job-1", "tweet-1"))
        .expect("create");
    persistence
        .database
        .fail_job(
            "job-1",
            JobState::Failed,
            "DOWNLOAD_FAILED",
            "network error",
            "2026-09-13T00:01:00Z",
        )
        .expect("fail");

    let summary = persistence.stored_job_summary("job-1").expect("summary");
    assert_eq!(summary.state, JobState::Failed);
    assert_eq!(summary.last_error_code.as_deref(), Some("DOWNLOAD_FAILED"));
    assert_eq!(summary.last_error_message.as_deref(), Some("network error"));

    let events = persistence.stored_events("job-1").expect("events");
    assert_eq!(events[0].0, "JOB_STATE_CHANGED");
    assert!(events[0].1.is_none());
}

#[test]
fn storage_persistence_recovery_scans_active_jobs_and_skips_terminal_jobs() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");

    persistence
        .create_or_reuse(&request("job-queued", "tweet-queued"))
        .expect("queued job");
    persistence
        .create_or_reuse(&request("job-interrupted", "tweet-interrupted"))
        .expect("interrupted job");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-interrupted".into(),
            tweet_id: "tweet-interrupted".into(),
            state: JobState::Interrupted,
        })
        .expect("interrupt job");
    persistence
        .create_or_reuse(&request("job-failed", "tweet-failed"))
        .expect("failed job");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-failed".into(),
            tweet_id: "tweet-failed".into(),
            state: JobState::Validating,
        })
        .expect("validate failed job");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-failed".into(),
            tweet_id: "tweet-failed".into(),
            state: JobState::Failed,
        })
        .expect("fail job");

    let candidates = persistence.list_recovery_candidates();
    assert_eq!(candidates.len(), 2);
    assert!(candidates.iter().all(|job| job.job_id != "job-failed"));

    let recovered = service.recover_persisted(&mut persistence).unwrap();
    assert_eq!(recovered.len(), 2);
    assert!(
        recovered
            .iter()
            .all(|job| job.state == JobState::Validating)
    );
    assert_eq!(
        persistence.snapshot("job-queued").unwrap().state,
        JobState::Validating
    );
    assert_eq!(
        persistence.snapshot("job-interrupted").unwrap().state,
        JobState::Validating
    );
    assert_eq!(
        persistence.snapshot("job-failed").unwrap().state,
        JobState::Failed
    );
    let interrupted_events = persistence.stored_events("job-interrupted").unwrap();
    assert_eq!(
        interrupted_events
            .iter()
            .map(|(event_type, _)| event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["JOB_STATE_CHANGED", "JOB_STATE_CHANGED"]
    );
}

#[test]
fn persisted_recovery_records_the_actual_source_state_for_queued_and_interrupted_jobs() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-queued", "tweet-queued"))
        .unwrap();
    persistence
        .create_or_reuse(&request("job-interrupted", "tweet-interrupted"))
        .unwrap();
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-interrupted".into(),
            tweet_id: "tweet-interrupted".into(),
            state: JobState::Interrupted,
        })
        .unwrap();

    service.recover_persisted(&mut persistence).unwrap();
    assert_eq!(
        persistence.events("job-queued").unwrap(),
        vec![ExecutorEvent::Recovered {
            from: JobState::Queued,
            to: JobState::Validating,
        }]
    );
    assert_eq!(
        persistence.events("job-interrupted").unwrap(),
        vec![ExecutorEvent::Recovered {
            from: JobState::Interrupted,
            to: JobState::Validating,
        }]
    );
}

#[test]
fn executor_events_preserve_state_transition_order() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    service.submit(request("job-1", "tweet-1")).unwrap();
    service.cancel("job-1").unwrap();

    assert_eq!(
        service.events("job-1").unwrap(),
        vec![
            ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            },
            ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Interrupted,
            },
        ]
    );
}

#[test]
fn sidecar_crash_maps_to_safe_failure_and_core_job_event() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    service.submit(request("job-1", "tweet-1")).unwrap();
    service.executor.start("job-1").unwrap();
    let snapshot = service
        .executor
        .sidecar_crashed("job-1", "worker exited")
        .unwrap();

    assert_eq!(snapshot.state, JobState::Failed);
    let events = service.events("job-1").unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        ExecutorEvent::SidecarCrashed { error_message } if error_message == "worker exited"
    )));
    assert_eq!(
        events.last().and_then(|event| event.to_job_event("job-1")),
        Some(JobEvent::DownloadFailed {
            error_code: "SIDECAR_INTERNAL_ERROR".into(),
            error_message: "worker exited".into(),
        })
    );
}
