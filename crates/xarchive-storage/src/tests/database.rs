use super::*;

#[test]
fn initializes_schema_and_creates_idempotent_job() {
    let database = Database::open_in_memory().expect("database");
    let tweet_id = database
        .insert_tweet("1", "https://x.com/a/status/1", "post", "text", "now")
        .expect("tweet");
    assert!(
        database
            .create_archive_job("job-1", tweet_id, "now")
            .expect("job")
    );
    assert!(
        !database
            .create_archive_job("job-2", tweet_id, "now")
            .expect("idempotent job")
    );
}
#[test]
fn exact_job_queries_are_not_limited_to_the_latest_one_hundred() {
    let database = Database::open_in_memory().expect("database");
    let first_tweet = database
        .insert_tweet("1", "https://x.com/a/status/1", "post", "", "t0")
        .expect("first tweet");
    assert!(
        database
            .create_archive_job("job-1", first_tweet, "t0")
            .expect("first job")
    );
    for index in 2..=105 {
        let tweet_id = index.to_string();
        let row_id = database
            .insert_tweet(
                &tweet_id,
                &format!("https://x.com/a/status/{tweet_id}"),
                "post",
                "",
                &format!("t{index}"),
            )
            .expect("tweet");
        assert!(
            database
                .create_archive_job(&format!("job-{index}"), row_id, &format!("t{index}"))
                .expect("job")
        );
    }
    assert_eq!(database.list_recent_jobs(100).expect("recent").len(), 100);
    assert_eq!(
        database
            .job_summary("job-1")
            .expect("exact summary")
            .expect("old job")
            .tweet_id,
        "1"
    );
    assert_eq!(
        database
            .active_job_for_tweet("1")
            .expect("active by tweet")
            .expect("old active job")
            .job_id,
        "job-1"
    );
    assert_eq!(
        database
            .list_recovery_candidate_jobs()
            .expect("recovery")
            .len(),
        105
    );
    assert!(database.job_summary("missing").expect("missing").is_none());
    assert!(
        database
            .active_job_for_tweet("missing")
            .expect("missing active")
            .is_none()
    );
    let page = database.list_jobs_page(100, 20).expect("old page");
    assert_eq!(page.total, 105);
    assert_eq!(page.jobs.len(), 5);
    assert_eq!(page.offset, 100);
    assert_eq!(page.limit, 20);
    assert_eq!(page.jobs.last().expect("last page row").job_id, "job-1");
    assert!(
        database
            .job_download_metrics("job-1")
            .expect("metrics")
            .is_none()
    );
}
#[test]
fn download_metrics_are_persisted_and_stale_attempts_are_fenced() {
    let database = Database::open_in_memory().expect("database");
    let tweet = database
        .insert_tweet("50", "https://x.com/a/status/50", "post", "", "t0")
        .expect("tweet");
    database
        .create_archive_job("metric-job", tweet, "t0")
        .expect("job");
    let metrics = JobDownloadMetrics {
        job_id: "metric-job".into(),
        backend: Some("aria2".into()),
        download_started_at: Some("t1".into()),
        download_finished_at: Some("t2".into()),
        task_finished_at: None,
        downloaded_bytes: Some(4096),
        download_duration_ms: Some(2000),
        attempt_count: 2,
    };
    database
        .record_download_metrics(&metrics, "t2")
        .expect("record metrics");
    let stale = JobDownloadMetrics {
        attempt_count: 1,
        downloaded_bytes: Some(1),
        ..metrics.clone()
    };
    database
        .record_download_metrics(&stale, "t3")
        .expect("stale result ignored");
    let stored = database
        .job_download_metrics("metric-job")
        .expect("read")
        .expect("stored metrics");
    assert_eq!(stored.attempt_count, 2);
    assert_eq!(stored.downloaded_bytes, Some(4096));
    assert_eq!(stored.download_duration_ms, Some(2000));
    assert_eq!(stored.backend.as_deref(), Some("aria2"));
}
#[test]
fn persists_user_name_history_and_stable_directory_name() {
    let database = Database::open_in_memory().expect("database");
    let user_id = database
        .upsert_user(
            "123",
            Some("alice"),
            Some("Alice / One"),
            "2026-09-08T00:00:00Z",
        )
        .expect("user");
    database
        .record_user_name(
            user_id,
            "alice_new",
            Some("Alice New"),
            "dom",
            "2026-09-08T00:01:00Z",
        )
        .expect("name history");
    let names = database.list_user_names(user_id).expect("names");
    assert_eq!(names.len(), 2);
    assert_eq!(names[0].username, "alice_new");
    assert_eq!(names[1].username, "alice");
    let user: UserSummary = database
            .connection
            .query_row(
                "SELECT x_user_id, stable_directory_name, first_seen_at, last_seen_at FROM users WHERE id = ?1",
                params![user_id],
                |row| {
                    Ok(UserSummary {
                        user_id: row.get(0)?,
                        stable_directory_name: row.get(1)?,
                        first_seen_at: row.get(2)?,
                        last_seen_at: row.get(3)?,
                    })
                },
            )
            .expect("user summary");
    assert_eq!(user.stable_directory_name, "@alice - Alice _ One [123]");
}
#[test]
fn does_not_duplicate_name_history_when_names_are_unchanged() {
    let database = Database::open_in_memory().expect("database");
    let user_id = database
        .upsert_user("123", Some("alice"), Some("Alice"), "t1")
        .expect("user");
    database
        .upsert_user("123", Some("alice"), Some("Alice"), "t2")
        .expect("unchanged upsert");
    database
        .record_user_name(user_id, "alice_new", Some("Alice New"), "dom", "t3")
        .expect("name change");
    database
        .upsert_user("123", Some("alice_new"), Some("Alice New"), "t4")
        .expect("unchanged upsert after change");
    let names = database.list_user_names(user_id).expect("names");
    let usernames: Vec<&str> = names.iter().map(|name| name.username.as_str()).collect();
    // There is also the case where display_name changes but username stays the same — it must still be recorded.
    assert_eq!(usernames, ["alice_new", "alice"]);
}
#[test]
fn persists_reply_and_quote_relationships() {
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/a/status/123", "quote", "", "now")
        .expect("tweet");
    let metadata = ArchiveMetadata {
        schema_version: 1,
        tweet_id: "123".into(),
        url: "https://x.com/a/status/123".into(),
        tweet_type: "quote".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: Some("alice".into()),
            display_name: Some("Alice".into()),
        },
        created_at: None,
        text: "quoting".into(),
        media: Vec::new(),
        archived_at: "2026-09-08T00:01:00Z".into(),
        reply_to: Some("111".into()),
        quoted_tweet: Some(xarchive_core::ArchiveQuotedTweet {
            tweet_id: "987".into(),
            url: "https://x.com/b/status/987".into(),
            username: Some("bob".into()),
            display_name: Some("Bob".into()),
            user_id: Some("9002".into()),
            text: Some("original".into()),
            created_at: None,
            tweet_type: Some("post".into()),
        }),
    };
    database
        .update_tweet_metadata(tweet_row_id, &metadata, "archive/123")
        .expect("metadata");
    let relationships = database
        .tweet_relationships("123")
        .expect("relationships")
        .expect("relationship row");
    assert_eq!(
        relationships,
        TweetRelationships {
            reply_to_tweet_id: Some("111".into()),
            quoted_tweet_id: Some("987".into()),
        }
    );
    assert_eq!(
        database.tweet_relationships("missing").expect("missing"),
        None
    );
}
#[test]
fn upgrades_existing_database_with_relationship_columns() {
    let root = temp_root();
    fs::create_dir_all(&root).expect("root");
    let database_path = root.join("archive.sqlite3");
    {
        let connection = rusqlite::Connection::open(&database_path).expect("raw database");
        connection
                .execute_batch(
                    "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
                )
                .expect("legacy version table");
        connection
            .execute_batch(include_str!("../../migrations/0001_initial.sql"))
            .expect("legacy schema");
        connection
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-09-08T00:00:00Z')",
                    [],
                )
                .expect("legacy initial migration version");
        connection
                .execute(
                    "INSERT INTO tweets (tweet_id, canonical_url, tweet_type, text, created_at, updated_at) VALUES ('123', 'https://x.com/a/status/123', 'post', '', 'now', 'now')",
                    [],
                )
                .expect("legacy tweet");
        connection
            .execute_batch(include_str!(
                "../../migrations/0002_telegram_send_state.sql"
            ))
            .expect("legacy telegram schema");
        connection
            .execute_batch(include_str!(
                "../../migrations/0003_quote_reply_relationships.sql"
            ))
            .expect("legacy relationships");
        connection
            .execute_batch(include_str!(
                "../../migrations/0004_archive_job_requests.sql"
            ))
            .expect("legacy execution request schema");
        connection
            .execute_batch(include_str!("../../migrations/0005_account_batches.sql"))
            .expect("legacy batch schema");
        connection
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (5, '2026-09-08T00:00:00Z')",
                    [],
                )
                .expect("legacy version");
        connection
                .execute(
                    "INSERT INTO archive_batches (id, username, profile_url, filters_json, state, discovery_state) VALUES ('legacy-batch', 'alice', 'https://x.com/alice', '{}', 'ACTIVE', 'PENDING')",
                    [],
                )
                .expect("legacy batch row");
    }
    let database = Database::open(&database_path).expect("upgraded database");
    let version: i64 = database
        .connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    // The legacy database is upgraded through the current migration set
    // (0001…0009, including the archive-intent recovery journal).
    assert_eq!(version, connection::MIGRATIONS.len() as i64);
    let relationships = database
        .tweet_relationships("123")
        .expect("relationships")
        .expect("legacy row");
    assert_eq!(
        relationships,
        TweetRelationships {
            reply_to_tweet_id: None,
            quoted_tweet_id: None,
        }
    );
    let legacy_batch_snapshot: String = database
        .connection
        .query_row(
            "SELECT output_settings_json FROM archive_batches LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("snapshot column exists after migration");
    assert_eq!(
        legacy_batch_snapshot,
        r#"{"naming_mode":"original","filename_template":"","export_json":true,"export_text":true}"#
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn attaches_tags_idempotently_and_lists_them_in_order() {
    let database = Database::open_in_memory().expect("database");
    let tweet_id = database
        .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
        .expect("tweet");
    let launch = database.upsert_tag("launch", "now").expect("tag");
    let person = database.upsert_tag("person", "now").expect("tag");
    assert_eq!(
        database.upsert_tag("launch", "later").expect("same tag"),
        launch
    );
    database.attach_tag(tweet_id, launch).expect("attach");
    database
        .attach_tag(tweet_id, launch)
        .expect("duplicate attach");
    database.attach_tag(tweet_id, person).expect("attach");
    assert_eq!(
        database.list_tweet_tags(tweet_id).expect("tags"),
        vec!["launch", "person"]
    );
}
#[test]
fn rejects_invalid_user_and_tag_inputs() {
    let database = Database::open_in_memory().expect("database");
    assert!(matches!(
        database.upsert_user("", None, None, "now"),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert!(matches!(
        database.upsert_tag("\n", "now"),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert!(matches!(
        database.record_user_name(999, "", None, "dom", "now"),
        Err(StorageError::InvalidMetadata(_))
    ));
}
#[test]
fn lists_recent_jobs_in_updated_order_and_preserves_errors() {
    let database = Database::open_in_memory().expect("database");
    let first_tweet = database
        .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
        .expect("first tweet");
    let second_tweet = database
        .insert_tweet("2", "https://x.com/b/status/2", "reply", "", "now")
        .expect("second tweet");
    database
        .create_archive_job("job-1", first_tweet, "2026-09-08T00:00:00Z")
        .expect("first job");
    database
        .create_archive_job("job-2", second_tweet, "2026-09-08T00:01:00Z")
        .expect("second job");
    let mut database = database;
    database
        .transition_job("job-2", JobState::Validating, "2026-09-08T00:02:00Z")
        .expect("transition");
    let jobs = database.list_recent_jobs(10).expect("jobs");
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].job_id, "job-2");
    assert_eq!(jobs[0].tweet_id, "2");
    assert_eq!(jobs[0].state, JobState::Validating);
    assert_eq!(jobs[1].job_id, "job-1");
    assert_eq!(database.job_history_index("job-2").unwrap(), Some(0));
    assert_eq!(database.job_history_index("job-1").unwrap(), Some(1));
    assert_eq!(database.job_history_index("missing").unwrap(), None);
    database
        .fail_job(
            "job-1",
            xarchive_core::JobState::Failed,
            "TRANSFER_FAILED",
            "http://u:p@cdn.example/file?token=secret&id=1",
            "now",
        )
        .expect("fail");
    let summary = database
        .job_summary("job-1")
        .expect("summary")
        .expect("job");
    assert_eq!(
        summary.last_error_message.as_deref(),
        Some("http://[REDACTED]@cdn.example/file?token=[REDACTED]&id=1")
    );
}
#[test]
fn recovery_candidate_query_includes_only_nonterminal_recovery_states() {
    let mut database = Database::open_in_memory().expect("database");
    let states = [
        JobState::Queued,
        JobState::Validating,
        JobState::MetadataReady,
        JobState::TgMetadataSending,
        JobState::TgMetadataSent,
        JobState::Downloading,
        JobState::Downloaded,
        JobState::TgMediaUploading,
        JobState::Interrupted,
        JobState::Complete,
        JobState::Failed,
        JobState::Cancelled,
        JobState::AuthRequired,
    ];
    for (index, state) in states.into_iter().enumerate() {
        let tweet_id = format!("candidate-{index}");
        let row = database
            .insert_tweet(
                &tweet_id,
                &format!("https://x.com/a/status/{tweet_id}"),
                "post",
                "",
                "now",
            )
            .expect("tweet");
        database
            .create_archive_job(&format!("job-{index}"), row, "now")
            .expect("job");
        match state {
            JobState::Queued => {}
            JobState::Failed => {
                database
                    .fail_job(
                        &format!("job-{index}"),
                        state,
                        "TEST",
                        "terminal fixture",
                        "now",
                    )
                    .expect("terminal state");
            }
            JobState::Complete => {
                database
                    .connection
                    .execute(
                        "UPDATE jobs SET state = 'COMPLETE' WHERE id = ?1",
                        [&format!("job-{index}")],
                    )
                    .expect("complete fixture");
            }
            JobState::Interrupted => {
                database
                    .connection
                    .execute(
                        "UPDATE jobs SET state = 'INTERRUPTED' WHERE id = ?1",
                        [&format!("job-{index}")],
                    )
                    .expect("interrupted fixture");
            }
            JobState::Cancelled => {
                database
                    .connection
                    .execute(
                        "UPDATE jobs SET state = 'CANCELLED' WHERE id = ?1",
                        [&format!("job-{index}")],
                    )
                    .expect("cancelled fixture");
            }
            JobState::Validating => database
                .transition_job(&format!("job-{index}"), state, "now")
                .expect("active state"),
            JobState::MetadataReady
            | JobState::TgMetadataSending
            | JobState::TgMetadataSent
            | JobState::Downloading
            | JobState::Downloaded
            | JobState::TgMediaUploading => {
                let job_id = format!("job-{index}");
                database
                    .transition_job(&job_id, JobState::Validating, "now")
                    .expect("validating state");
                for next in [
                    JobState::MetadataReady,
                    JobState::TgMetadataSending,
                    JobState::TgMetadataSent,
                    JobState::Downloading,
                    JobState::Downloaded,
                    JobState::TgMediaUploading,
                ] {
                    if next == state {
                        break;
                    }
                    database
                        .transition_job(&job_id, next, "now")
                        .expect("active transition");
                }
                database
                    .transition_job(&job_id, state, "now")
                    .expect("requested active state");
            }
            JobState::AuthRequired => {
                database
                    .connection
                    .execute(
                        "UPDATE jobs SET state = ?1 WHERE id = ?2",
                        params![state.as_str(), format!("job-{index}")],
                    )
                    .expect("terminal non-recovery fixture");
            }
        }
    }

    let mut candidates = database
        .list_recovery_candidate_jobs()
        .expect("recovery candidates")
        .into_iter()
        .map(|job| job.job_id)
        .collect::<Vec<_>>();
    candidates.sort();
    let mut expected = (0..9)
        .map(|index| format!("job-{index}"))
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(candidates, expected);
}
#[test]
fn locates_history_index_by_created_order_not_recent_update_order() {
    let database = Database::open_in_memory().expect("database");
    for (id, created_at) in [
        ("old", "2026-10-01T00:00:00Z"),
        ("new", "2026-10-02T00:00:00Z"),
    ] {
        let tweet = database
            .insert_tweet(
                id,
                &format!("https://x.com/a/status/{id}"),
                "post",
                "",
                created_at,
            )
            .expect("tweet");
        database
            .create_archive_job(&format!("job-{id}"), tweet, created_at)
            .expect("job");
    }
    let mut database = database;
    database
        .transition_job("job-old", JobState::Validating, "2026-10-03T00:00:00Z")
        .expect("update old job");

    assert_eq!(database.list_recent_jobs(2).unwrap()[0].job_id, "job-old");
    assert_eq!(
        database.list_jobs_page(0, 2).unwrap().jobs[0].job_id,
        "job-new"
    );
    assert_eq!(database.job_history_index("job-old").unwrap(), Some(1));
    assert_eq!(database.job_history_index("job-new").unwrap(), Some(0));
}
#[test]
fn aggregates_job_metrics_across_all_persisted_states() {
    let database = Database::open_in_memory().expect("database");
    assert_eq!(
        database.job_metrics().expect("empty metrics"),
        JobMetrics {
            total: 0,
            active: 0,
            completed: 0,
            failed: 0
        }
    );
    let tweet_ids = (1..=4)
        .map(|id| {
            database
                .insert_tweet(
                    &id.to_string(),
                    &format!("https://x.com/a/status/{id}"),
                    "post",
                    "",
                    "now",
                )
                .expect("tweet")
        })
        .collect::<Vec<_>>();
    let mut database = database;
    for (index, tweet_id) in tweet_ids.into_iter().enumerate() {
        database
            .create_archive_job(&format!("job-{index}"), tweet_id, "now")
            .expect("job");
    }
    database
        .transition_job("job-0", JobState::Validating, "later")
        .expect("active");
    database
        .transition_job("job-1", JobState::Validating, "later")
        .expect("active");
    database
        .transition_job("job-1", JobState::MetadataReady, "later")
        .expect("metadata");
    database
        .fail_job("job-2", JobState::Failed, "TEST", "failed", "later")
        .expect("failed");
    for next in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::TgMetadataSending,
        JobState::TgMetadataSent,
        JobState::Downloading,
        JobState::Downloaded,
        JobState::TgMediaUploading,
        JobState::Complete,
    ] {
        database
            .transition_job("job-3", next, "later")
            .expect("complete transition");
    }
    assert_eq!(
        database.job_metrics().expect("metrics"),
        JobMetrics {
            total: 4,
            active: 2,
            completed: 1,
            failed: 1
        }
    );
}
#[test]
fn reopens_persistent_database_without_reapplying_schema() {
    let root = temp_root();
    fs::create_dir_all(&root).expect("root");
    let database_path = root.join("archive.sqlite3");
    {
        let database = Database::open(&database_path).expect("first open");
        let tweet_id = database
            .insert_tweet("1", "https://x.com/a/status/1", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job("job-1", tweet_id, "now")
            .expect("job");
    }
    let reopened = Database::open(&database_path).expect("reopen");
    assert_eq!(
        reopened.job_state("job-1").expect("state"),
        JobState::Queued
    );
    let _ = fs::remove_dir_all(root);
}
