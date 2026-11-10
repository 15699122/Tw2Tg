//! Submission, duplicate-tweet reuse and the application-service facade.

use super::*;

#[test]
fn submission_adapter_persists_the_captured_output_settings_snapshot() {
    let request = archive_request("12005");
    let settings = OutputSettings {
        naming_mode: NamingMode::Template,
        filename_template: "{username}_{tweet_id}".into(),
        export_json: false,
        export_text: true,
    };

    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_output_settings(&request, "now", settings.clone())
        .expect("prepare task with snapshot");
    let decoded = decode_execution_spec(prepared.schema_version, &prepared.request_json)
        .expect("decode persisted snapshot");

    assert_eq!(decoded.output_settings, settings);
    assert_eq!(decoded.request.tweet.tweet_id, "12005");
}

#[test]
fn v3_submission_uses_internal_recovery_for_storage_backed_archive_jobs() {
    let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
    let mut request = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12006"),
            "now",
            OutputSettings::default(),
            DownloaderArgumentSnapshot::historical(),
        )
        .expect("prepare v3 task");
    request.schema_version = 3;

    let submitted = persistence
        .create_or_reuse(&request)
        .expect("persist v3 job");
    let recovery_contract = persistence
        .database
        .archive_recovery_contract(&submitted.job.job_id)
        .expect("read recovery contract");

    assert_eq!(recovery_contract, Some(1));
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
fn application_service_delegates_submit_query_cancel_and_shutdown() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let submitted = service.submit(request("job-1", "tweet-1")).expect("submit");
    assert!(submitted.created);
    assert_eq!(service.query("job-1").unwrap(), submitted.job);
    assert_eq!(service.cancel("job-1").unwrap().state, JobState::Cancelled);
    service.shutdown().expect("shutdown");
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
    assert!(persistence.list_recovery_candidates().unwrap().is_empty());
}
