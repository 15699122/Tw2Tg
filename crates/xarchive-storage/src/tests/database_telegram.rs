use super::*;

#[test]
fn telegram_send_state_round_trip() {
    let database = Database::open_in_memory().expect("database");
    assert!(
        database
            .find_sent("-100", "tweet-1:metadata")
            .expect("find")
            .is_none()
    );
    database
        .record_pending("-100", "tweet-1:metadata", "metadata", "now-1")
        .expect("pending");
    assert!(
        database
            .find_sent("-100", "tweet-1:metadata")
            .expect("find")
            .is_none()
    );
    database
        .record_sent(
            "-100",
            "tweet-1:metadata",
            "4242",
            Some("file-id-1"),
            "now-2",
        )
        .expect("sent");
    let sent = database
        .find_sent("-100", "tweet-1:metadata")
        .expect("find")
        .expect("sent record");
    assert_eq!(
        sent,
        SentSendRecord {
            chat_id: "-100".into(),
            idempotency_key: "tweet-1:metadata".into(),
            message_kind: "metadata".into(),
            telegram_message_id: "4242".into(),
            telegram_file_id: Some("file-id-1".into()),
        }
    );
    assert!(database.list_unsent().expect("unsent").is_empty());
}
#[test]
fn telegram_send_state_counts_retries_and_lists_unsent() {
    let database = Database::open_in_memory().expect("database");
    database
        .record_pending("-100", "tweet-1:media:1", "media", "t1")
        .expect("pending");
    database
        .record_failed(
            "-100",
            "tweet-1:media:1",
            Some(429),
            "Too Many Requests",
            "t2",
        )
        .expect("failed");
    database
        .record_pending("-100", "tweet-1:media:1", "media", "t3")
        .expect("pending again");
    let unsent = database.list_unsent().expect("unsent");
    assert_eq!(unsent.len(), 1);
    assert_eq!(unsent[0].state, SendState::Pending);
    assert_eq!(unsent[0].attempt_count, 2);
    assert_eq!(unsent[0].message_kind, "media");
    database
        .record_failed("-100", "tweet-1:media:1", None, "HTTP status 503", "t4")
        .expect("failed again");
    database
        .record_pending("-200", "tweet-1:metadata", "metadata", "t5")
        .expect("other chat pending");
    let unsent = database.list_unsent().expect("unsent");
    assert_eq!(unsent.len(), 2);
    assert_eq!(unsent[0].chat_id, "-100");
    assert_eq!(unsent[0].attempt_count, 2);
    assert_eq!(unsent[1].chat_id, "-200");
    database
        .record_sent("-100", "tweet-1:media:1", "77", None, "t6")
        .expect("sent");
    let unsent = database.list_unsent().expect("unsent");
    assert_eq!(unsent.len(), 1);
    assert_eq!(unsent[0].chat_id, "-200");
}
#[test]
fn telegram_send_state_sent_update_requires_existing_record() {
    let database = Database::open_in_memory().expect("database");
    assert_eq!(
        database
            .record_sent("-100", "missing", "1", None, "now")
            .expect_err("not found"),
        SendStateError::NotFound
    );
    assert_eq!(
        database
            .record_failed("-100", "missing", Some(400), "bad", "now")
            .expect_err("not found"),
        SendStateError::NotFound
    );
}
#[test]
fn set_and_get_setting_round_trips() {
    let database = Database::open_in_memory().expect("database");
    assert!(database.get_setting("ui.theme").expect("get").is_none());
    database
        .set_setting("ui.theme", "\"dark\"", "2026-09-12T00:00:00Z")
        .expect("set");
    assert_eq!(
        database.get_setting("ui.theme").expect("get"),
        Some("\"dark\"".to_owned())
    );
    database
        .set_setting("ui.theme", "\"light\"", "2026-09-12T00:01:00Z")
        .expect("update");
    assert_eq!(
        database.get_setting("ui.theme").expect("get"),
        Some("\"light\"".to_owned())
    );
}
#[test]
fn output_settings_persist_as_a_typed_setting_and_reject_unknown_fields() {
    let database = Database::open_in_memory().expect("database");
    let settings = BatchOutputSettings {
        naming_mode: BatchNamingMode::Template,
        filename_template: "{username}_{tweet_id}".into(),
        export_json: false,
        export_text: true,
    };
    let encoded = serde_json::to_string(&settings).expect("encode settings");
    database
        .set_setting("download.output_settings", &encoded, "now")
        .expect("persist settings");
    let persisted = database
        .get_setting("download.output_settings")
        .expect("read settings")
        .expect("saved settings");
    assert_eq!(
        serde_json::from_str::<BatchOutputSettings>(&persisted).expect("typed settings"),
        settings
    );
    assert!(serde_json::from_str::<BatchOutputSettings>(
            r#"{"naming_mode":"original","filename_template":"","export_json":true,"export_text":true,"extra":1}"#
        )
        .is_err());
}
#[test]
fn list_and_delete_settings() {
    let database = Database::open_in_memory().expect("database");
    assert!(database.list_settings().expect("list").is_empty());
    database.set_setting("ui.a", "\"1\"", "now").expect("set a");
    database
        .set_setting("download.b", "\"2\"", "now")
        .expect("set b");
    let settings = database.list_settings().expect("list");
    assert_eq!(settings.len(), 2);
    assert_eq!(settings[0].key, "download.b");
    assert_eq!(settings[1].key, "ui.a");
    assert!(database.delete_setting("ui.a").expect("delete"));
    assert!(!database.delete_setting("ui.a").expect("delete again"));
    assert_eq!(database.list_settings().expect("list").len(), 1);
}
#[test]
fn rejects_invalid_setting_inputs() {
    let database = Database::open_in_memory().expect("database");
    assert!(matches!(
        database.set_setting("", "\"v\"", "now"),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert!(matches!(
        database.set_setting("telegram.bot_token", "\"secret\"", "now"),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert!(matches!(
        database.set_setting("ui.k", "not-json", "now"),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert!(matches!(
        database.get_setting(""),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert!(matches!(
        database.delete_setting("telegram.bot_token"),
        Err(StorageError::InvalidMetadata(_))
    ));
}
#[test]
fn rejects_sidecar_metadata_for_a_different_tweet() {
    let root = temp_root();
    let files = FileStore::new(&root).expect("files");
    let staging = files.staging_dir("job-1").expect("staging");
    let raw = serde_json::json!({"tweet_id": "456"});
    assert!(matches!(
        build_archive_metadata("123", &raw, &[], &staging, "now"),
        Err(StorageError::InvalidMetadata(message))
            if message.contains("does not match")
    ));
    let _ = fs::remove_dir_all(root);
}
#[test]
fn creates_account_batch_with_pending_discovery_and_zero_counts() {
    let database = Database::open_in_memory().expect("database");
    insert_alice_batch(&database, "batch-1");
    let batch = database
        .account_batch("batch-1")
        .expect("get")
        .expect("row");
    assert_eq!(batch.state, "ACTIVE");
    assert_eq!(batch.discovery_state, "PENDING");
    assert_eq!(batch.counts, BatchCounts::default());
    assert_eq!(batch.username, "alice");
    assert_eq!(
        database.account_batch_state("batch-1").expect("state"),
        Some("ACTIVE".into())
    );
    assert!(
        database
            .account_batch("missing")
            .expect("missing")
            .is_none()
    );
    database
        .set_account_batch_discovery_state("batch-1", "RUNNING")
        .expect("discovery state");
    database
        .resolve_account_batch_identity("batch-1", Some("42"), Some("alice_real"))
        .expect("identity");
    let batch = database
        .account_batch("batch-1")
        .expect("get")
        .expect("row");
    assert_eq!(batch.discovery_state, "RUNNING");
    assert_eq!(batch.user_id.as_deref(), Some("42"));
    assert_eq!(batch.username, "alice_real");
    assert!(matches!(
            database.create_account_batch(CreateAccountBatch {
                id: "",
                username: "alice",
                profile_url: "https://x.com/alice",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &BatchOutputSettings::default(),
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            }),
            Err(StorageError::InvalidMetadata(_))
        ));
    assert!(matches!(
        database.set_account_batch_state("missing", "COMPLETED"),
        Err(StorageError::InvalidMetadata(_))
    ));
}
#[test]
fn lists_account_batches_newest_first_and_honours_limit() {
    let database = Database::open_in_memory().expect("database");
    insert_alice_batch(&database, "batch-1");
    database
            .create_account_batch(CreateAccountBatch {
                id: "batch-2",
                username: "bob",
                profile_url: "https://x.com/bob",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &BatchOutputSettings::default(),
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            })
            .expect("second batch");
    let batches = database.list_account_batches(10).expect("list");
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].id, "batch-2");
    assert_eq!(batches[1].id, "batch-1");
    assert_eq!(database.list_account_batches(1).expect("limited").len(), 1);
}
#[test]
fn account_batch_output_snapshot_round_trips_without_normalization() {
    let database = Database::open_in_memory().expect("database");
    let snapshot = r#"{"naming_mode":"template","filename_template":"{tweet_id}","export_json":false,"export_text":true}"#;
    database
            .create_account_batch(CreateAccountBatch {
                id: "snapshot-batch",
                username: "alice",
                profile_url: "https://x.com/alice",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &serde_json::from_str(snapshot).expect("typed snapshot"),
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            })
            .expect("create batch");
    let batch = database
        .account_batch("snapshot-batch")
        .expect("read batch")
        .expect("batch row");
    assert_eq!(batch.output_settings_json, snapshot);
}
#[test]
fn rejects_unknown_fields_when_deserializing_batch_output_settings() {
    let invalid: Result<BatchOutputSettings, _> = serde_json::from_str(
        r#"{"naming_mode":"original","filename_template":"","export_json":true,"export_text":true,"extra":1}"#,
    );
    assert!(invalid.is_err());
}
#[test]
fn rejects_invalid_persisted_batch_output_settings_before_dispatch() {
    let database = Database::open_in_memory().expect("database");
    insert_alice_batch(&database, "invalid-snapshot-batch");
    database
            .connection
            .execute(
                "UPDATE archive_batches SET output_settings_json = ?1 WHERE id = ?2",
                params![r#"{"naming_mode":"future","filename_template":"","export_json":true,"export_text":true}"#, "invalid-snapshot-batch"],
            )
            .expect("corrupt snapshot fixture");
    let persisted = database
        .account_batch_output_settings_json("invalid-snapshot-batch")
        .expect("read snapshot")
        .expect("batch snapshot");
    assert!(serde_json::from_str::<BatchOutputSettings>(&persisted).is_err());
}
#[test]
fn archive_job_and_execution_spec_are_created_atomically_and_reuse_preserves_spec() {
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("908", "https://x.com/a/status/908", "post", "", "now")
        .expect("tweet");
    let request_json = r#"{"request":"first"}"#;
    assert!(
        database
            .create_archive_job_with_request(
                "atomic-job",
                tweet_row_id,
                2,
                "request-first",
                request_json,
                "now",
            )
            .expect("create job and spec")
    );
    assert_eq!(
        database
            .archive_job_request("atomic-job")
            .expect("read spec"),
        Some((2, "request-first".into(), request_json.into()))
    );
    assert!(
        !database
            .create_archive_job_with_request(
                "replacement-job",
                tweet_row_id,
                2,
                "request-second",
                r#"{"request":"second"}"#,
                "later",
            )
            .expect("reuse active job")
    );
    assert!(
        database
            .job_summary("replacement-job")
            .expect("replacement query")
            .is_none()
    );
    assert_eq!(
        database
            .archive_job_request("atomic-job")
            .expect("original spec"),
        Some((2, "request-first".into(), request_json.into()))
    );
}
#[test]
fn finds_active_archive_job_before_request_side_effects() {
    let database = Database::open_in_memory().expect("database");
    let user_id = database
        .upsert_user("42", Some("before"), Some("Before"), "now")
        .expect("user");
    let tweet_row_id = database
        .insert_tweet_for_user(
            "909",
            "https://x.com/before/status/909",
            "post",
            "before text",
            Some(user_id),
            "now",
        )
        .expect("tweet");
    database
        .create_archive_job_with_request(
            "active-job",
            tweet_row_id,
            2,
            "active-request",
            r#"{"stored":true}"#,
            "now",
        )
        .expect("job/spec");

    let active = database
        .active_archive_job_for_tweet("909")
        .expect("query")
        .expect("active job");
    assert_eq!(active.job_id, "active-job");
    assert_eq!(
        database.archive_job_request("active-job").expect("spec"),
        Some((2, "active-request".into(), r#"{"stored":true}"#.into()))
    );
    let tweet_text: String = database
        .connection
        .query_row(
            "SELECT text FROM tweets WHERE id = ?1",
            [tweet_row_id],
            |row| row.get(0),
        )
        .expect("stored text");
    assert_eq!(tweet_text, "before text");
}
#[test]
fn inserts_batch_candidates_once_and_filters_by_state() {
    let database = Database::open_in_memory().expect("database");
    insert_alice_batch(&database, "batch-1");
    let candidates = vec![
        new_candidate("100", "PENDING"),
        new_candidate("101", "PENDING"),
    ];
    assert_eq!(
        database
            .insert_batch_candidates("batch-1", &candidates)
            .expect("insert"),
        2
    );
    assert_eq!(
        database
            .insert_batch_candidates("batch-1", &candidates)
            .expect("reinsert"),
        0
    );
    let listed = database
        .list_batch_candidates("batch-1", None, 10)
        .expect("list");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].tweet_id, "100");
    assert_eq!(listed[0].media_count, 2);
    assert!(listed[0].has_media);
    assert!(!listed[0].is_repost);
    assert_eq!(listed[0].state, "PENDING");
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
        .expect("mark submitted");
    assert_eq!(
        database.batch_candidate_counts("batch-1").expect("counts"),
        BatchCounts {
            total: 2,
            pending: 1,
            submitted: 1,
            ..BatchCounts::default()
        }
    );
    let submitted = database
        .list_batch_candidates("batch-1", Some("SUBMITTED"), 10)
        .expect("submitted");
    assert_eq!(submitted.len(), 1);
    assert_eq!(submitted[0].job_id.as_deref(), Some("job-100"));
    assert!(matches!(
        database.mark_batch_candidate("batch-1", "999", "DONE", None, None, None, None),
        Err(StorageError::InvalidMetadata(_))
    ));
    assert!(matches!(
        database.mark_batch_candidate("batch-1", "101", "BOGUS", None, None, None, None),
        Err(StorageError::InvalidMetadata(_))
    ));
}
#[test]
fn retries_failures_and_cancels_only_undispatched_candidates() {
    let database = Database::open_in_memory().expect("database");
    insert_alice_batch(&database, "batch-1");
    let candidates = (0..3)
        .map(|index| new_candidate(&format!("10{index}"), "PENDING"))
        .collect::<Vec<_>>();
    database
        .insert_batch_candidates("batch-1", &candidates)
        .expect("insert");
    database
        .mark_batch_candidate(
            "batch-1",
            "100",
            "FAILED",
            None,
            Some("HTTP_429"),
            Some("rate limited"),
            None,
        )
        .expect("failed");
    database
        .mark_batch_candidate(
            "batch-1",
            "101",
            "SUBMITTED",
            Some("job-101"),
            None,
            None,
            None,
        )
        .expect("submitted");
    assert_eq!(
        database
            .retry_failed_batch_candidates("batch-1")
            .expect("retry"),
        1
    );
    assert!(
        database
            .list_batch_candidates("batch-1", Some("FAILED"), 10)
            .expect("failed list")
            .is_empty()
    );
    let retried = database
        .list_batch_candidates("batch-1", Some("PENDING"), 10)
        .expect("pending list");
    assert_eq!(retried.len(), 2);
    assert!(
        retried
            .iter()
            .all(|candidate| candidate.error_code.is_none() && candidate.job_id.is_none())
    );
    // cancel only stops future dispatch: submitted jobs keep running
    assert_eq!(
        database
            .cancel_pending_batch_candidates("batch-1")
            .expect("cancel"),
        2
    );
    let counts = database.batch_candidate_counts("batch-1").expect("counts");
    assert_eq!(counts.cancelled, 2);
    assert_eq!(counts.submitted, 1);
    assert_eq!(counts.pending, 0);
    assert_eq!(
        database
            .cancel_pending_batch_candidates("batch-1")
            .expect("cancel again"),
        0
    );
}
#[test]
fn upgrades_v5_account_batches_without_losing_candidates_or_foreign_keys() {
    let root = temp_root();
    fs::create_dir_all(&root).expect("root");
    let database_path = root.join("archive.sqlite3");
    {
        let connection = rusqlite::Connection::open(&database_path).expect("raw database");
        connection
            .execute_batch(include_str!("../../migrations/0001_initial.sql"))
            .expect("base schema");
        connection
            .execute_batch(include_str!("../../migrations/0005_account_batches.sql"))
            .expect("v5 batch schema");
        connection
                .execute(
                    "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
                    [],
                )
                .expect("versions");
        for version in 1..=5 {
            connection
                    .execute(
                        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, '2026-09-24T00:00:00Z')",
                        [version],
                    )
                    .expect("version row");
        }
        connection
                .execute(
                    "INSERT INTO archive_batches (id, username, profile_url, filters_json, state, discovery_state) VALUES ('batch-old', 'alice', 'https://x.com/alice', '{}', 'ACTIVE', 'RUNNING')",
                    [],
                )
                .expect("batch");
        connection
                .execute(
                    "INSERT INTO batch_candidates (batch_id, tweet_id, url, tweet_type, is_repost, has_media, media_count, state) VALUES ('batch-old', '123', 'https://x.com/alice/status/123', 'post', 0, 1, 1, 'PENDING')",
                    [],
                )
                .expect("candidate");
    }
    let mut database = Database::open(&database_path).expect("upgrade");
    let candidates = database
        .list_batch_candidates("batch-old", Some("PENDING"), 10)
        .expect("candidates");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].tweet_id, "123");
    let paused = database
        .pause_account_batch("batch-old")
        .expect("paused after migration");
    assert_eq!(paused.discovery_state, "PAUSED");
    let foreign_key_errors = database
        .connection
        .prepare("PRAGMA foreign_key_check")
        .expect("fk check")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("fk rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("fk values");
    assert!(foreign_key_errors.is_empty());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn atomic_batch_control_keeps_discovery_state_consistent() {
    let mut database = Database::open_in_memory().expect("database");
    database
            .create_account_batch(CreateAccountBatch {
                id: "batch-pause",
                username: "alice",
                profile_url: "https://x.com/alice",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &BatchOutputSettings::default(),
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            })
            .expect("pause batch");
    database
        .set_account_batch_discovery_state("batch-pause", "RUNNING")
        .expect("running");
    let paused = database
        .pause_account_batch("batch-pause")
        .expect("pause transaction");
    assert_eq!(paused.state, "PAUSED");
    assert_eq!(paused.discovery_state, "PAUSED");

    database
            .create_account_batch(CreateAccountBatch {
                id: "batch-cancel",
                username: "bob",
                profile_url: "https://x.com/bob",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &BatchOutputSettings::default(),
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            })
            .expect("cancel batch");
    database
        .insert_batch_candidates(
            "batch-cancel",
            &[NewBatchCandidate {
                tweet_id: "1".to_owned(),
                url: "https://x.com/bob/status/1".to_owned(),
                created_at: None,
                tweet_type: "post".to_owned(),
                is_repost: false,
                has_media: true,
                media_count: 1,
                user_id: Some("2".to_owned()),
                username: Some("bob".to_owned()),
                state: "PENDING".to_owned(),
                skip_reason: None,
            }],
        )
        .expect("candidate");
    database
        .set_account_batch_discovery_state("batch-cancel", "RUNNING")
        .expect("running");
    let cancelled = database
        .cancel_account_batch("batch-cancel")
        .expect("cancel transaction");
    assert_eq!(cancelled.state, "CANCELLED");
    assert_eq!(cancelled.discovery_state, "CANCELLED");
    assert_eq!(cancelled.counts.cancelled, 1);
    assert!(database.pause_account_batch("batch-cancel").is_err());
}
#[test]
fn pauses_batches_with_bounded_retry_and_resumes_after_backoff() {
    let database = Database::open_in_memory().expect("database");
    insert_alice_batch(&database, "batch-1");
    database
            .create_account_batch(CreateAccountBatch {
                id: "batch-2",
                username: "bob",
                profile_url: "https://x.com/bob",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &BatchOutputSettings::default(),
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            })
            .expect("second batch");
    database
        .set_account_batch_state("batch-1", "PAUSED")
        .expect("pause");
    database
        .set_account_batch_error("batch-1", Some("HTTP_429"), Some("rate limited"))
        .expect("error");
    database
        .set_account_batch_retry_at("batch-1", Some(2_000))
        .expect("retry at");
    // auth failures have no retry deadline and therefore wait for the user
    database
        .set_account_batch_state("batch-2", "PAUSED")
        .expect("pause auth batch");
    let batch = database
        .account_batch("batch-1")
        .expect("get")
        .expect("row");
    assert_eq!(batch.retry_at_ms, Some(2_000));
    assert_eq!(batch.last_error_code.as_deref(), Some("HTTP_429"));
    assert_eq!(
        database
            .resume_rate_limited_batches(1_999)
            .expect("too early"),
        0
    );
    assert_eq!(database.resume_rate_limited_batches(2_000).expect("due"), 1);
    let batch = database
        .account_batch("batch-1")
        .expect("get")
        .expect("row");
    assert_eq!(batch.state, "ACTIVE");
    assert_eq!(batch.retry_at_ms, None);
    assert_eq!(
        database.account_batch_state("batch-2").expect("state"),
        Some("PAUSED".into())
    );
    // an active transition clears any stale pause deadline
    database
        .set_account_batch_retry_at("batch-2", Some(9_000))
        .expect("stale deadline");
    database
        .set_account_batch_state("batch-2", "ACTIVE")
        .expect("manual resume");
    let batch = database
        .account_batch("batch-2")
        .expect("get")
        .expect("row");
    assert_eq!(batch.retry_at_ms, None);
}
#[test]
fn merges_browser_placeholder_user_into_stable_identity() {
    let database = Database::open_in_memory().expect("database");
    let placeholder = database
        .upsert_user(
            "browser-123",
            Some("alice"),
            Some("Alice"),
            "2026-09-20T00:00:00Z",
        )
        .expect("placeholder");
    database
        .record_user_name(
            placeholder,
            "alice_old",
            Some("Alice"),
            "legacy",
            "2026-09-19T00:00:00Z",
        )
        .expect("legacy placeholder name");
    let stable = database
        .upsert_user("42", Some("alice"), Some("Alice"), "2026-09-21T00:00:00Z")
        .expect("stable");
    // a second identical observation from the stable path must be kept as-is
    // (only the placeholder fold de-duplicates)
    database
        .record_user_name(
            stable,
            "alice",
            Some("Alice"),
            "dom",
            "2026-09-21T00:00:00Z",
        )
        .expect("duplicate name");
    let tweet_row_id = database
        .insert_tweet_for_user(
            "123",
            "https://x.com/alice/status/123",
            "post",
            "",
            Some(placeholder),
            "2026-09-20T00:00:00Z",
        )
        .expect("tweet");
    database
        .merge_placeholder_user("browser-123", stable, "2026-09-22T00:00:00Z")
        .expect("merge");
    let linked: Option<i64> = database
        .connection
        .query_row(
            "SELECT user_id FROM tweets WHERE id = ?1",
            params![tweet_row_id],
            |row| row.get(0),
        )
        .expect("tweet user");
    assert_eq!(linked, Some(stable));
    let remaining: i64 = database
        .connection
        .query_row(
            "SELECT COUNT(*) FROM users WHERE x_user_id = 'browser-123'",
            [],
            |row| row.get(0),
        )
        .expect("placeholder removed");
    assert_eq!(remaining, 0);
    let names = database.list_user_names(stable).expect("names");
    let usernames: Vec<&str> = names.iter().map(|name| name.username.as_str()).collect();
    // stable keeps its own duplicate observations; the identical placeholder
    // observation is dropped while the unique one is folded in
    assert_eq!(usernames, ["alice", "alice", "alice_old"]);
    let user: UserSummary = database
            .connection
            .query_row(
                "SELECT x_user_id, stable_directory_name, first_seen_at, last_seen_at FROM users WHERE id = ?1",
                params![stable],
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
    assert_eq!(user.first_seen_at, "2026-09-20T00:00:00Z");
    assert_eq!(user.last_seen_at, "2026-09-21T00:00:00Z");
    // unknown placeholders and self merges are no-ops
    database
        .merge_placeholder_user("browser-missing", stable, "2026-09-22T00:00:00Z")
        .expect("missing placeholder");
    database
        .merge_placeholder_user("42", stable, "2026-09-22T00:00:00Z")
        .expect("self merge");
    assert_eq!(database.list_user_names(stable).expect("names").len(), 3);
}
#[test]
fn reports_archive_facts_for_completed_tweets_in_media_order() {
    let database = Database::open_in_memory().expect("database");
    let user_row_id = database
        .upsert_user("42", Some("alice"), Some("Alice"), "2026-09-20T00:00:00Z")
        .expect("user");
    let tweet_row_id = database
        .insert_tweet_for_user(
            "123",
            "https://x.com/alice/status/123",
            "post",
            "",
            Some(user_row_id),
            "2026-09-20T00:00:00Z",
        )
        .expect("tweet");
    assert!(
        database
            .tweet_archive_facts("123")
            .expect("pending")
            .is_none()
    );
    let metadata = ArchiveMetadata {
        schema_version: 1,
        tweet_id: "123".into(),
        url: "https://x.com/alice/status/123".into(),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: Some("42".into()),
            username: Some("alice".into()),
            display_name: Some("Alice".into()),
        },
        created_at: None,
        text: "with media".into(),
        media: vec![
            xarchive_core::ArchiveMedia {
                index: 1,
                media_id: Some("m0".into()),
                media_type: "video".into(),
                file: "media/1.mp4".into(),
                mime_type: Some("video/mp4".into()),
                size_bytes: 10,
                sha256: "a".into(),
            },
            xarchive_core::ArchiveMedia {
                index: 2,
                media_id: Some("m1".into()),
                media_type: "photo".into(),
                file: "media/2.jpg".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 20,
                sha256: "b".into(),
            },
        ],
        archived_at: "2026-09-21T00:00:00Z".into(),
        reply_to: None,
        quoted_tweet: None,
    };
    database
        .update_tweet_metadata(tweet_row_id, &metadata, "archive/123")
        .expect("metadata");
    // inserted out of order on purpose: facts must follow media_index
    for media in metadata.media.iter().rev() {
        database
            .insert_media(tweet_row_id, media, &metadata.archived_at)
            .expect("media");
    }
    let facts = database
        .tweet_archive_facts("123")
        .expect("facts")
        .expect("row");
    assert_eq!(facts.archive_directory, "archive/123");
    assert_eq!(
        facts.media_paths().collect::<Vec<_>>(),
        vec!["media/1.mp4", "media/2.jpg"]
    );
    // Media identity and size travel with the facts so completeness can be
    // decided without treating "the file exists" as success.
    assert_eq!(facts.media[0].media_index, 1);
    assert_eq!(facts.media[0].media_id.as_deref(), Some("m0"));
    assert_eq!(facts.media[0].media_type, "video");
    assert_eq!(facts.media[0].size_bytes, Some(10));
    assert_eq!(facts.media[0].sha256.as_deref(), Some("a"));
    assert_eq!(facts.media[1].media_index, 2);
    assert_eq!(facts.media[1].size_bytes, Some(20));
    assert!(
        database
            .tweet_archive_facts("456")
            .expect("missing")
            .is_none()
    );
}

// -------------------------------------------------------------------------
// Telegram outbox and file_id cache (plan TG-04 / TG-05)
// -------------------------------------------------------------------------
