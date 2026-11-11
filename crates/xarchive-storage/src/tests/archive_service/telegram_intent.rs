//! `archive_service` — telegram intent coverage.

use super::*;

#[test]
fn archive_intent_journal_recovers_staging_after_database_reopen() {
    check_archive_intent_recovery(false);
}

#[test]
fn archive_intent_journal_recovers_renamed_archive_after_database_reopen() {
    check_archive_intent_recovery(true);
}

#[test]
fn internal_v1_telegram_intent_refuses_recovery_without_runner_permit() {
    let root = temp_root();
    fs::create_dir_all(&root).expect("root");
    let database_path = root.join("recovery.sqlite3");
    let database = Database::open(&database_path).expect("database");
    let tweet_row_id = database
        .insert_tweet(
            "intent-v1",
            "https://x.com/a/status/intent-v1",
            "post",
            "",
            "t0",
        )
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            "intent-v1-job",
            tweet_row_id,
            1,
            "request",
            "{}",
            Some(ArchiveRecoveryContract::InternalV1),
            "t0",
        )
        .expect("InternalV1 job");
    let files = FileStore::new(&root).expect("files");
    for (name, contents) in [("01.jpg", &b"payload"[..]), ("tweet.json", &b"{}"[..])] {
        fs::write(
            files
                .staging_dir("intent-v1-job")
                .expect("staging")
                .join(name),
            contents,
        )
        .expect("staging file");
    }
    let intent = TelegramArchiveIntentRecord {
        job_id: "intent-v1-job".into(),
        tweet_row_id,
        archive_directory: "Tweets/intent-v1".into(),
        state: "PREPARED".into(),
        metadata_text: None,
        media_json: "[]".into(),
        bot_identity: Some("bot".into()),
        chat_id: Some("-1".into()),
        message_thread_id: None,
        config_revision: None,
        plan_version: 1,
        created_at: "t0".into(),
        updated_at: "t0".into(),
    };
    database
        .prepare_telegram_archive_intent(&intent)
        .expect("intent");
    let mut service = ArchiveService::new(database, files);
    let error = service
        .recover_telegram_archive_intent(&intent, "t1")
        .expect_err("runner authorization required");
    assert!(
        error
            .to_string()
            .contains("runner-owned Workflow authorization")
    );
    assert_eq!(
        service
            .database
            .telegram_archive_intent(tweet_row_id)
            .expect("intent query"),
        Some(intent)
    );
    assert_eq!(
        service.database.job_state("intent-v1-job").expect("state"),
        JobState::Queued
    );
    assert!(root.join("_staging/intent-v1-job/01.jpg").is_file());
    assert!(!root.join("Tweets/intent-v1").exists());
    drop(service);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn archive_intent_journal_rejects_changed_facts_and_invalid_transitions() {
    let database = Database::open_in_memory().expect("database");
    let tweet = database
        .insert_tweet("intent-2", "url", "post", "", "t0")
        .expect("tweet");
    database
        .create_archive_job("intent-job-2", tweet, "t0")
        .expect("job");
    let intent = TelegramArchiveIntentRecord {
        job_id: "intent-job-2".into(),
        tweet_row_id: tweet,
        archive_directory: "Tweets/intent-2".into(),
        state: "PREPARED".into(),
        metadata_text: None,
        media_json: "[]".into(),
        bot_identity: Some("bot".into()),
        chat_id: Some("-100".into()),
        message_thread_id: None,
        config_revision: Some(1),
        plan_version: 1,
        created_at: "t0".into(),
        updated_at: "t0".into(),
    };
    database
        .prepare_telegram_archive_intent(&intent)
        .expect("prepare");
    let mut changed = intent.clone();
    changed.chat_id = Some("-200".into());
    assert!(database.prepare_telegram_archive_intent(&changed).is_err());
    assert!(
        database
            .transition_telegram_archive_intent(tweet, "PREPARED", "QUEUED", "t1")
            .is_err()
    );
    assert!(
        database
            .transition_telegram_archive_intent(tweet, "PREPARED", "SKIPPED", "t1")
            .is_err()
    );
    assert!(
        database
            .transition_telegram_archive_intent(tweet, "PREPARED", "ARCHIVED", "t1")
            .expect("transition")
    );
    assert!(
        !database
            .transition_telegram_archive_intent(tweet, "PREPARED", "ARCHIVED", "t2")
            .expect("repeat transition")
    );
    let mut entry = outbox_entry("bot", "journal-plan", 0);
    entry.tweet_id = Some(tweet);
    entry.chat_id = "-100".into();
    entry.target_scope = "-100:none".into();
    entry.config_version = 1;
    let mut wrong = entry.clone();
    wrong.bot_identity = "other-bot".into();
    wrong.idempotency_key = "wrong-plan".into();
    assert!(
        database
            .enqueue_archived_intent(tweet, vec![entry.clone(), wrong], "t3")
            .is_err()
    );
    assert!(
        database
            .list_outbox_for_tweet("bot", tweet)
            .expect("rolled back rows")
            .is_empty()
    );
    assert_eq!(
        database
            .telegram_archive_intent(tweet)
            .expect("journal")
            .expect("intent")
            .state,
        "ARCHIVED"
    );
    let ids = database
        .enqueue_archived_intent(tweet, vec![entry.clone()], "t4")
        .expect("materialize");
    assert_eq!(
        database
            .telegram_archive_intent(tweet)
            .expect("journal")
            .expect("intent")
            .state,
        "QUEUED"
    );
    assert_eq!(
        database
            .enqueue_archived_intent(tweet, vec![entry], "t5")
            .expect("replay"),
        ids
    );
}

#[test]
fn outbox_batch_rolls_back_on_conflict_and_replays_idempotently() {
    let database = Database::open_in_memory().expect("database");
    let original = outbox_entry(BOT_A, "batch-existing", 1);
    database.enqueue_outbox(original.clone()).expect("existing");
    let fresh = outbox_entry(BOT_A, "batch-new", 0);
    let mut conflict = original.clone();
    conflict.request_fingerprint = "different".into();
    assert!(
        database
            .enqueue_outbox_batch(vec![fresh.clone(), conflict])
            .is_err()
    );
    assert_eq!(
        database
            .list_due_outbox(BOT_A, OUTBOX_NOW)
            .expect("due")
            .len(),
        1
    );
    let ids = database
        .enqueue_outbox_batch(vec![fresh.clone(), original.clone()])
        .expect("batch");
    assert_eq!(
        database
            .enqueue_outbox_batch(vec![fresh, original])
            .expect("replay"),
        ids
    );
    assert_eq!(
        database
            .list_due_outbox(BOT_A, OUTBOX_NOW)
            .expect("due")
            .len(),
        2
    );
}
