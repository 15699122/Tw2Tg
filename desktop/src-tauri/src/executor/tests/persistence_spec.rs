//! Durable persistence adapters and the versioned execution-spec contract.

use super::*;

#[test]
fn execution_spec_v1_decodes_historical_request_json_without_rewriting() {
    let request = archive_request("12001");
    let json = serde_json::to_string(&request).expect("serialize historical request");

    let decoded = decode_execution_spec(1, &json).expect("decode v1");

    assert_eq!(decoded.request.tweet.tweet_id, request.tweet.tweet_id);
    assert_eq!(decoded.request.browser, request.browser);
    assert_eq!(decoded.request.profile, request.profile);
    assert_eq!(decoded.output_settings, OutputSettings::default());
}

#[test]
fn v2_execution_settings_are_preserved_by_decoding() {
    let settings = OutputSettings {
        naming_mode: NamingMode::Template,
        filename_template: "{tweet_id}_{index}".to_owned(),
        export_json: false,
        export_text: true,
    };
    let json = serde_json::to_string(&ExecutionSpecV2 {
        request: archive_request("12002"),
        output_settings: settings.clone(),
    })
    .expect("serialize v2");

    let decoded = decode_execution_spec(2, &json).expect("decode v2");

    assert_eq!(decoded.output_settings, settings);
}

#[test]
fn execution_spec_v1_rejects_invalid_json_and_v2_rejects_request_identity_mismatch() {
    assert!(matches!(
        decode_execution_spec(1, "{"),
        Err(ExecutionSpecError::InvalidJson(_))
    ));
    let request = archive_request("12004");
    let json = serde_json::to_string(&ExecutionSpecV2 {
        request,
        output_settings: OutputSettings::default(),
    })
    .expect("serialize v2");
    let decoded = decode_execution_spec(2, &json).expect("decode v2");
    let error = validate_execution_spec_identity(decoded, "12003")
        .expect_err("mismatched identity must fail before process creation");
    assert!(error.to_string().contains("identity does not match Job"));
}

#[test]
fn execution_spec_v2_decodes_request_and_immutable_output_snapshot() {
    let request = archive_request("12002");
    let envelope = ExecutionSpecV2 {
        request: request.clone(),
        output_settings: OutputSettings {
            naming_mode: NamingMode::Template,
            filename_template: "{username}_{tweet_id}".to_owned(),
            export_json: false,
            export_text: true,
        },
    };
    let json = serde_json::to_string(&envelope).expect("serialize v2");

    let decoded = decode_execution_spec(2, &json).expect("decode v2");

    assert_eq!(decoded.request.tweet.tweet_id, request.tweet.tweet_id);
    assert_eq!(decoded.request.browser, request.browser);
    assert_eq!(decoded.request.profile, request.profile);
    assert_eq!(decoded.output_settings, envelope.output_settings);
}

#[test]
fn execution_spec_rejects_unknown_version_explicitly() {
    assert!(matches!(
        decode_execution_spec(77, "{}"),
        Err(ExecutionSpecError::UnsupportedVersion(77))
    ));
}

#[test]
fn execution_spec_v3_decodes_request_output_and_downloader_argument_snapshot() {
    let settings = OutputSettings {
        naming_mode: NamingMode::Template,
        filename_template: "{username}_{tweet_id}".to_owned(),
        export_json: false,
        export_text: true,
    };
    let request = archive_request("13002");
    let snapshot = DownloaderArgumentSnapshot {
        gallery_dl: Vec::new(),
        aria2: vec!["--max-connection-per-server".to_owned(), "4".to_owned()],
        policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
        tool_compatibility: ToolCompatibility {
            aria2: Some("1.37.0".to_owned()),
            gallery_dl: Some("1.28.5".to_owned()),
        },
    };
    let json = serde_json::to_string(&ExecutionSpecV3 {
        request: request.clone(),
        output_settings: settings.clone(),
        downloader_arguments: snapshot.clone(),
    })
    .expect("serialize v3");

    let decoded = decode_execution_spec(3, &json).expect("decode v3");

    assert_eq!(decoded.request.tweet.tweet_id, request.tweet.tweet_id);
    assert_eq!(decoded.output_settings, settings);
    assert_eq!(decoded.downloader_arguments, snapshot);
}

#[test]
fn execution_spec_v1_and_v2_decode_to_empty_historical_argument_snapshot() {
    // Historical tasks carry no user arguments and must not be resampled
    // from current settings. They decode to an empty snapshot rather than
    // to today's arguments.
    let v1_json = serde_json::to_string(&archive_request("13003")).expect("serialize v1");
    let decoded_v1 = decode_execution_spec(1, &v1_json).expect("decode v1");
    assert_eq!(
        decoded_v1.downloader_arguments,
        DownloaderArgumentSnapshot::historical()
    );
    assert!(decoded_v1.downloader_arguments.gallery_dl.is_empty());
    assert!(decoded_v1.downloader_arguments.aria2.is_empty());

    let v2_json = serde_json::to_string(&ExecutionSpecV2 {
        request: archive_request("13003"),
        output_settings: OutputSettings::default(),
    })
    .expect("serialize v2");
    let decoded_v2 = decode_execution_spec(2, &v2_json).expect("decode v2");
    assert_eq!(
        decoded_v2.downloader_arguments,
        DownloaderArgumentSnapshot::historical()
    );
    assert!(decoded_v2.downloader_arguments.gallery_dl.is_empty());
    assert!(decoded_v2.downloader_arguments.aria2.is_empty());
}

#[test]
fn execution_spec_v3_refuses_a_mismatched_argument_policy_version() {
    // A snapshot written under a different policy version is refused rather
    // than reinterpreted, so a later allowlist/value-policy change can never
    // silently reinterpret previously saved arguments.
    let snapshot = DownloaderArgumentSnapshot {
        gallery_dl: vec!["--unreviewed-option".to_owned()],
        aria2: Vec::new(),
        policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION + 1,
        tool_compatibility: ToolCompatibility::default(),
    };
    let (json, _) = v3_spec(snapshot);

    let error = decode_execution_spec(3, &json)
        .expect_err("a foreign argument-policy version must be refused");

    assert!(matches!(
        error,
        ExecutionSpecError::DownloaderArgumentPolicy(_)
    ));
    assert!(error.to_string().contains("argument policy version"));
}

#[test]
fn execution_spec_v3_rejects_invalid_json_explicitly() {
    assert!(matches!(
        decode_execution_spec(3, "{"),
        Err(ExecutionSpecError::InvalidJson(_))
    ));
}

#[test]
fn execution_spec_v3_rejects_unknown_fields_in_the_argument_snapshot() {
    // The snapshot is deny_unknown_fields, so an unexpected key is refused
    // at decode time instead of being silently dropped.
    let raw = format!(
        "{{\"request\":{},\"output_settings\":{{}},\"downloader_arguments\":{{\"gallery_dl\":[],\"aria2\":[],\"policy_version\":{},\"tool_compatibility\":{{}},\"unexpected\":1}}}}",
        serde_json::to_string(&archive_request("13004")).expect("serialize request"),
        xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION
    );

    let error =
        decode_execution_spec(3, &raw).expect_err("an unknown snapshot field must be refused");

    assert!(matches!(error, ExecutionSpecError::InvalidJson(_)));
}

#[test]
fn v3_argument_snapshot_round_trips_through_json_without_rewriting_argv() {
    // Stored argv elements are validated string arrays; they round-trip
    // byte-for-byte and are never tokenized, escaped, or rewritten.
    let snapshot = DownloaderArgumentSnapshot {
        gallery_dl: Vec::new(),
        aria2: vec!["--max-connection-per-server".to_owned(), "8".to_owned()],
        policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
        tool_compatibility: ToolCompatibility {
            aria2: Some("1.37.0".to_owned()),
            gallery_dl: None,
        },
    };
    let (json, _) = v3_spec(snapshot.clone());

    let decoded = decode_execution_spec(3, &json).expect("decode v3");

    assert_eq!(decoded.downloader_arguments.gallery_dl, snapshot.gallery_dl);
    assert_eq!(decoded.downloader_arguments.aria2, snapshot.aria2);
    assert_eq!(decoded.downloader_arguments, snapshot);
}

#[test]
fn downloader_argument_diagnostics_do_not_disclose_values_or_tool_versions() {
    let snapshot = DownloaderArgumentSnapshot {
        gallery_dl: Vec::new(),
        aria2: vec!["--max-tries".into(), "secret-value".into()],
        policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
        tool_compatibility: ToolCompatibility {
            aria2: Some("1.37.0-private-build".into()),
            gallery_dl: None,
        },
    };
    let diagnostic = format!("{snapshot:?}");
    assert!(!diagnostic.contains("secret-value"));
    assert!(!diagnostic.contains("1.37.0-private-build"));
    assert!(diagnostic.contains("aria2_argument_count"));
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
fn stale_attempt_cannot_overwrite_a_state_written_by_a_newer_attempt() {
    let database_path = temporary_database_path("stale-attempt-fence");
    std::fs::create_dir_all(database_path.parent().expect("config dir")).expect("parent");
    let job_id = "job-stale-attempt";
    let tweet_id = "tweet-stale-attempt";

    {
        let mut persistence =
            StorageJobPersistence::open(&database_path).expect("open persistence");
        persistence
            .create_or_reuse(&v2_request(job_id, tweet_id))
            .expect("create v2 job");
    }

    // The stale attempt claims attempt 1 and advances to DOWNLOADING.
    let mut stale = StorageJobPersistence::open(&database_path).expect("open stale attempt");
    let stale_attempt = stale.begin_attempt(job_id).expect("claim stale attempt");
    assert_eq!(stale_attempt, 1);
    for (expected, next) in [
        (JobState::Queued, JobState::Validating),
        (JobState::Validating, JobState::MetadataReady),
        (JobState::MetadataReady, JobState::Downloading),
    ] {
        assert!(
            stale
                .persist_state_for_attempt(job_id, stale_attempt, expected, next)
                .expect("stale attempt writes while it still owns the job"),
            "stale attempt should still own the job at {expected:?} -> {next:?}"
        );
    }

    // A newer attempt claims the same Job and durably fails it.
    let mut winner = StorageJobPersistence::open(&database_path).expect("open winner attempt");
    let winner_attempt = winner.begin_attempt(job_id).expect("claim winner attempt");
    assert_eq!(winner_attempt, 2);
    winner
        .fail(job_id, "CANCELLED_BY_USER", "durably cancelled")
        .expect("durable cancellation");
    drop(winner);

    // The stale attempt must not write DOWNLOADED over the winning state.
    let advanced = stale
        .persist_state_for_attempt(
            job_id,
            stale_attempt,
            JobState::Downloading,
            JobState::Downloaded,
        )
        .expect("fenced write reports a loss instead of failing");
    assert!(!advanced, "stale attempt must lose the arbitration");
    assert_eq!(
        stale.snapshot(job_id).expect("snapshot").state,
        JobState::Failed,
        "the winning attempt's durable state must survive"
    );

    let _ = std::fs::remove_dir_all(database_path.parent().expect("root"));
}

#[test]
fn execute_persisted_skips_terminal_jobs_without_calling_execution_port() {
    let executor = JobExecutor::new();
    let service = ArchiveApplicationService::new(&executor);
    let mut persistence = InMemoryJobPersistence::default();
    persistence
        .create_or_reuse(&request("job-terminal", "tweet-terminal"))
        .unwrap();
    complete_job_for_fixture(&mut persistence, "job-terminal", "tweet-terminal");
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
    assert!(persistence.list_recovery_candidates().unwrap().is_empty());
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

#[cfg(unix)]
#[test]
fn session_execution_error_preserves_prepared_job_for_startup_replay() {
    let path = temporary_database_path("prepared-error");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let runtime = ExecutorRuntime::new(path.clone());
    let mut persistence = CommitBoundaryPersistence {
        report_at_precheck: true,
        ..Default::default()
    };
    persistence
        .create_or_reuse(&request("job-prepared", "123"))
        .unwrap();
    let coordinator = ProductionCoordinator::open(&path).unwrap();
    let service = runtime.service();
    let mut boundary = SessionExecutionBoundary::new(
        service,
        persistence,
        coordinator,
        path.parent().unwrap().to_owned(),
    );
    // The unavailable production factory fails after the durable claim.
    assert!(boundary.execute_session("job-prepared").is_err());
    assert_eq!(
        boundary.persistence.snapshot("job-prepared").unwrap().state,
        JobState::Downloading
    );
    assert!(
        !boundary
            .persistence
            .events("job-prepared")
            .unwrap()
            .iter()
            .any(|event| matches!(event, ExecutorEvent::DownloadFailed { .. }))
    );
    drop(boundary);
    drop(runtime);
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
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
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-resume".into(),
            tweet_id: "tweet-resume".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-complete".into(),
            tweet_id: "tweet-complete".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-failed".into(),
            tweet_id: "tweet-failed".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
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
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-resume".into(),
            tweet_id: "tweet-resume".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-complete".into(),
            tweet_id: "tweet-complete".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-failed".into(),
            tweet_id: "tweet-failed".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    insert_downloaded_job(&mut persistence, "job-missing", "tweet-missing");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-missing".into(),
            tweet_id: "tweet-missing".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
    persistence
        .persist_state(&JobSnapshot {
            job_id: "job-terminal".into(),
            tweet_id: "tweet-terminal".into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .complete_terminal_for_attempt("job-terminal", 0, "archives/terminal", 0)
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
    assert_eq!(
        terminal_events
            .iter()
            .filter(|(event_type, _)| event_type == "JOB_COMPLETED")
            .count(),
        1
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

    let candidates = persistence.list_recovery_candidates().unwrap();
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
