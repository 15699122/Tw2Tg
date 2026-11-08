use super::*;

#[test]
fn credential_generation_activation_is_atomic_and_fenced() {
    let root = temp_root();
    std::fs::create_dir_all(&root).expect("root");
    let path = root.join("credentials.sqlite3");
    let database = Database::open(&path).expect("database");
    assert!(
        database
            .prepare_telegram_credential_generation("token-hash", "secret/1", "t0")
            .is_err()
    );
    let first = database
        .prepare_telegram_credential_generation("telegram-bot:42", "secret/1", "t0")
        .expect("candidate");
    assert!(
        database
            .active_telegram_credential_generation()
            .unwrap()
            .is_none()
    );
    database
        .activate_telegram_credential_generation(first, None)
        .expect("activate");
    let second = database
        .prepare_telegram_credential_generation("telegram-bot:42", "secret/2", "t1")
        .expect("rotation");
    assert!(
        database
            .activate_telegram_credential_generation(second, None)
            .is_err()
    );
    assert_eq!(
        database
            .active_telegram_credential_generation()
            .unwrap()
            .unwrap()
            .0,
        first
    );
    database
        .activate_telegram_credential_generation(second, Some(first))
        .expect("rotate");
    assert!(
        database
            .activate_telegram_credential_generation(first, Some(second))
            .is_err()
    );
    drop(database);
    let database = Database::open(&path).expect("reopen");
    assert_eq!(
        database.active_telegram_credential_generation().unwrap(),
        Some((second, "telegram-bot:42".into(), "secret/2".into()))
    );
    drop(database);
    std::fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn stale_credential_activation_from_another_connection_cannot_replace_winner() {
    let root = temp_root();
    std::fs::create_dir_all(&root).expect("root");
    let path = root.join("activation.sqlite3");
    let first_connection = Database::open(&path).expect("first connection");
    let old = first_connection
        .prepare_telegram_credential_generation("telegram-bot:42", "secret/old", "t0")
        .expect("old candidate");
    first_connection
        .activate_telegram_credential_generation(old, None)
        .expect("initial activation");
    let second_connection = Database::open(&path).expect("second connection");
    let winner = first_connection
        .prepare_telegram_credential_generation("telegram-bot:42", "secret/winner", "t1")
        .expect("winner");
    let stale = second_connection
        .prepare_telegram_credential_generation("telegram-bot:43", "secret/stale", "t1")
        .expect("stale");
    first_connection
        .activate_telegram_credential_generation(winner, Some(old))
        .expect("winner activation");
    assert!(
        second_connection
            .activate_telegram_credential_generation(stale, Some(old))
            .is_err()
    );
    assert_eq!(
        second_connection
            .active_telegram_credential_generation()
            .expect("active"),
        Some((winner, "telegram-bot:42".into(), "secret/winner".into()))
    );
    drop(second_connection);
    drop(first_connection);
    std::fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn automatic_rotation_grants_only_existing_same_bot_rows() {
    let database = Database::open_in_memory().unwrap();
    let old = database
        .prepare_telegram_credential_generation("telegram-bot:42", "old", "t0")
        .unwrap();
    database
        .activate_telegram_credential_generation(old, None)
        .unwrap();
    let queued = database
        .enqueue_outbox(outbox_entry("telegram-bot:42", "queued", 0))
        .unwrap();
    let unknown = database
        .enqueue_outbox(outbox_entry("telegram-bot:42", "unknown", 0))
        .unwrap();
    database
        .execute_batch(&format!(
            "UPDATE telegram_outbox SET state = 'UNKNOWN' WHERE id = {unknown}"
        ))
        .unwrap();
    let other = database
        .enqueue_outbox(outbox_entry("telegram-bot:43", "other", 0))
        .unwrap();
    let next = database
        .prepare_telegram_credential_generation("telegram-bot:42", "next", "t1")
        .unwrap();
    assert_eq!(
        database
            .activate_telegram_credential_with_resume(next, Some(old), "automatic", "t2")
            .unwrap(),
        vec![queued]
    );
    assert!(database.telegram_resume_authorized(next, queued).unwrap());
    assert!(!database.telegram_resume_authorized(next, unknown).unwrap());
    assert!(!database.telegram_resume_authorized(next, other).unwrap());
    let future = database
        .enqueue_outbox(outbox_entry("telegram-bot:42", "future", 0))
        .unwrap();
    assert!(!database.telegram_resume_authorized(next, future).unwrap());
    let changed = database
        .prepare_telegram_credential_generation("telegram-bot:43", "changed", "t3")
        .unwrap();
    assert!(
        database
            .activate_telegram_credential_with_resume(changed, Some(next), "automatic", "t4")
            .unwrap()
            .is_empty()
    );
    assert!(!database.telegram_resume_authorized(changed, other).unwrap());
}
#[test]
fn resume_grants_are_exact_atomic_and_generation_scoped() {
    let database = Database::open_in_memory().expect("database");
    let generation = database
        .prepare_telegram_credential_generation("telegram-bot:42", "secret/1", "t0")
        .unwrap();
    database
        .activate_telegram_credential_generation(generation, None)
        .unwrap();
    let first = database
        .enqueue_outbox(outbox_entry("telegram-bot:42", "first", 0))
        .unwrap();
    let other_bot = database
        .enqueue_outbox(outbox_entry("telegram-bot:43", "other", 0))
        .unwrap();
    assert!(
        database
            .authorize_telegram_resume(generation, &[first, other_bot], "t1")
            .is_err()
    );
    assert!(
        !database
            .telegram_resume_authorized(generation, first)
            .unwrap()
    );
    database
        .authorize_telegram_resume(generation, &[first], "t1")
        .unwrap();
    database
        .authorize_telegram_resume(generation, &[first], "t2")
        .unwrap();
    assert!(
        database
            .telegram_resume_authorized(generation, first)
            .unwrap()
    );
    let future = database
        .enqueue_outbox(outbox_entry("telegram-bot:42", "future", 0))
        .unwrap();
    assert!(
        !database
            .telegram_resume_authorized(generation, future)
            .unwrap()
    );
    database
        .execute_batch(&format!(
            "UPDATE telegram_outbox SET state = 'UNKNOWN' WHERE id = {first}"
        ))
        .unwrap();
    assert!(
        !database
            .telegram_resume_authorized(generation, first)
            .unwrap()
    );
    assert!(
        database
            .authorize_telegram_resume(generation, &[first], "t3")
            .is_err()
    );
    let rotated = database
        .prepare_telegram_credential_generation("telegram-bot:42", "secret/2", "t3")
        .unwrap();
    database
        .activate_telegram_credential_generation(rotated, Some(generation))
        .unwrap();
    assert!(
        database
            .authorize_telegram_resume(generation, &[future], "t4")
            .is_err()
    );
    assert!(
        !database
            .telegram_resume_authorized(rotated, future)
            .unwrap()
    );
}
#[test]
fn rotation_after_request_start_recovers_unknown_and_never_auto_resends() {
    let database = Database::open_in_memory().unwrap();
    let bot = "telegram-bot:42";
    let row = database
        .enqueue_outbox(outbox_entry(bot, "started-rotation", 0))
        .unwrap();
    let old = database
        .prepare_telegram_credential_generation(bot, "secret/started-old", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(old, None)
        .unwrap();
    database
        .authorize_telegram_resume(old, &[row], OUTBOX_NOW)
        .unwrap();
    database
        .claim_due_outbox_for_generation(old, bot, "started-old", OUTBOX_NOW, OUTBOX_LEASE)
        .unwrap()
        .unwrap();
    database
        .mark_request_started("started-old", OUTBOX_NOW)
        .unwrap();
    let new = database
        .prepare_telegram_credential_generation(bot, "secret/started-new", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(new, Some(old))
        .unwrap();
    assert!(matches!(
        database.record_outbox_sent("started-old", "77", None, OUTBOX_NOW),
        Err(SendStateError::StaleClaim)
    ));
    assert_eq!(database.recover_outbox_claims(OUTBOX_LEASE).unwrap(), 1);
    assert_eq!(database.recover_outbox_claims(OUTBOX_LEASE).unwrap(), 0);
    let rows = database.list_outbox_for_tweet(bot, 0).unwrap();
    assert!(rows.is_empty());
    assert!(
        database
            .authorize_telegram_resume(new, &[row], OUTBOX_LEASE)
            .is_err()
    );
    assert!(
        database
            .claim_due_outbox_for_generation(new, bot, "started-new", OUTBOX_LEASE, OUTBOX_LATER)
            .unwrap()
            .is_none()
    );
    assert!(
        database
            .claim_outbox(
                bot,
                "started-rotation",
                "manual-new",
                OUTBOX_LEASE,
                OUTBOX_LATER
            )
            .unwrap()
            .is_none()
    );
    let state: String = database
        .connection
        .query_row(
            "SELECT state FROM telegram_outbox WHERE id = ?1",
            [row],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "UNKNOWN");
}
#[test]
fn explicit_unknown_review_requires_current_identity_and_duplicate_acknowledgement() {
    let database = Database::open_in_memory().unwrap();
    let mut entry = outbox_entry("telegram-bot:42", "review", 0);
    entry.payload_schema_version = 1;
    entry.payload_json = "{}".into();
    let row = database.enqueue_outbox(entry).unwrap();
    let generation = database
        .prepare_telegram_credential_generation("telegram-bot:42", "review-ref", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(generation, None)
        .unwrap();
    database
        .authorize_telegram_resume(generation, &[row], OUTBOX_NOW)
        .unwrap();
    database
        .claim_due_outbox_for_generation(
            generation,
            "telegram-bot:42",
            "review-claim",
            OUTBOX_NOW,
            OUTBOX_LEASE,
        )
        .unwrap()
        .unwrap();
    database
        .mark_request_started("review-claim", OUTBOX_NOW)
        .unwrap();
    database.recover_outbox_claims(OUTBOX_LEASE).unwrap();
    assert!(
        database
            .review_telegram_unknown_for_resend(generation, row, false, OUTBOX_LEASE)
            .is_err()
    );
    assert!(
        database
            .review_telegram_unknown_for_resend(generation + 1, row, true, OUTBOX_LEASE)
            .is_err()
    );
    assert!(
        database
            .claim_due_outbox_for_generation(
                generation,
                "telegram-bot:42",
                "auto",
                OUTBOX_LEASE,
                OUTBOX_LATER
            )
            .unwrap()
            .is_none()
    );
    database
        .review_telegram_unknown_for_resend(generation, row, true, OUTBOX_LEASE)
        .unwrap();
    assert!(
        database
            .review_telegram_unknown_for_resend(generation, row, true, OUTBOX_LEASE)
            .is_err()
    );
    assert!(
        database
            .telegram_resume_authorized(generation, row)
            .unwrap()
    );
    assert!(
        database
            .claim_due_outbox_for_generation(
                generation,
                "telegram-bot:42",
                "reviewed",
                OUTBOX_LEASE,
                OUTBOX_LATER
            )
            .unwrap()
            .is_some()
    );
}
#[test]
fn rotation_after_claim_fences_request_renewal_and_result_writes() {
    let database = Database::open_in_memory().unwrap();
    let bot = "telegram-bot:42";
    let row = database
        .enqueue_outbox(outbox_entry(bot, "rotation-after-claim", 0))
        .unwrap();
    let old = database
        .prepare_telegram_credential_generation(bot, "secret/claimed-old", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(old, None)
        .unwrap();
    database
        .authorize_telegram_resume(old, &[row], OUTBOX_NOW)
        .unwrap();
    database
        .claim_due_outbox_for_generation(old, bot, "claimed-old", OUTBOX_NOW, OUTBOX_LEASE)
        .unwrap()
        .unwrap();
    let new = database
        .prepare_telegram_credential_generation(bot, "secret/claimed-new", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(new, Some(old))
        .unwrap();
    assert!(matches!(
        database.mark_request_started("claimed-old", OUTBOX_NOW),
        Err(SendStateError::StaleClaim)
    ));
    assert!(
        !database
            .renew_outbox_claim("claimed-old", OUTBOX_LEASE, OUTBOX_NOW)
            .unwrap()
    );
    assert!(matches!(
        database.record_outbox_sent("claimed-old", "77", None, OUTBOX_NOW),
        Err(SendStateError::StaleClaim)
    ));
    database.recover_outbox_claims(OUTBOX_LEASE).unwrap();
    database
        .authorize_telegram_resume(new, &[row], OUTBOX_LEASE)
        .unwrap();
    database
        .claim_due_outbox_for_generation(
            new,
            bot,
            "claimed-new",
            OUTBOX_LEASE,
            "2026-10-02T00:00:00Z",
        )
        .unwrap()
        .unwrap();
    database
        .mark_request_started("claimed-new", OUTBOX_LEASE)
        .unwrap();
    assert!(matches!(
        database.record_outbox_sent("claimed-old", "77", None, OUTBOX_LEASE),
        Err(SendStateError::StaleClaim)
    ));
    database
        .record_outbox_sent("claimed-new", "78", None, OUTBOX_LEASE)
        .unwrap();
}
#[test]
fn generation_fenced_claim_rejects_retired_worker() {
    let database = Database::open_in_memory().unwrap();
    let bot = "telegram-bot:42";
    let row = database
        .enqueue_outbox(outbox_entry(bot, "fenced", 0))
        .unwrap();
    let old = database
        .prepare_telegram_credential_generation(bot, "secret/fence-old", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(old, None)
        .unwrap();
    let new = database
        .prepare_telegram_credential_generation(bot, "secret/fence-new", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(new, Some(old))
        .unwrap();
    database
        .authorize_telegram_resume(new, &[row], OUTBOX_NOW)
        .unwrap();
    assert!(
        database
            .claim_due_outbox_for_generation(old, bot, "old-worker", OUTBOX_NOW, OUTBOX_LEASE)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        database
            .claim_due_outbox_for_generation(new, bot, "new-worker", OUTBOX_NOW, OUTBOX_LEASE)
            .unwrap()
            .unwrap()
            .id,
        row
    );
}
#[test]
fn stable_bot_claim_requires_active_generation_row_grant() {
    let database = Database::open_in_memory().expect("database");
    let bot = "telegram-bot:42";
    let row = database
        .enqueue_outbox(outbox_entry(bot, "grant-claim", 0))
        .unwrap();
    assert!(
        database
            .claim_due_outbox(bot, "no-grant", OUTBOX_NOW, OUTBOX_LEASE)
            .unwrap()
            .is_none()
    );
    assert!(
        database
            .claim_outbox(
                bot,
                "grant-claim",
                "manual-no-grant",
                OUTBOX_NOW,
                OUTBOX_LEASE
            )
            .unwrap()
            .is_none()
    );
    let generation = database
        .prepare_telegram_credential_generation(bot, "secret/claim", OUTBOX_NOW)
        .unwrap();
    database
        .activate_telegram_credential_generation(generation, None)
        .unwrap();
    database
        .authorize_telegram_resume(generation, &[row], OUTBOX_NOW)
        .unwrap();
    assert_eq!(
        database
            .claim_due_outbox(bot, "granted", OUTBOX_NOW, OUTBOX_LEASE)
            .unwrap()
            .unwrap()
            .id,
        row
    );
}
#[test]
fn rotation_decision_preserves_policy_and_exact_candidates_across_reopen() {
    let root = temp_root();
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("rotation.sqlite3");
    let database = Database::open(&path).unwrap();
    let generation = database
        .prepare_telegram_credential_generation("telegram-bot:42", "secret/decision", "t0")
        .unwrap();
    database
        .activate_telegram_credential_generation(generation, None)
        .unwrap();
    assert!(
        database
            .record_telegram_rotation_decision(generation, "automatic", &[999999], "t0")
            .is_err()
    );
    assert!(
        database
            .telegram_rotation_decision(generation)
            .unwrap()
            .is_none()
    );
    database
        .record_telegram_rotation_decision(generation, "confirm", &[7, 8], "t1")
        .unwrap();
    database
        .record_telegram_rotation_decision(generation, "confirm", &[7, 8], "t2")
        .unwrap();
    assert!(
        database
            .record_telegram_rotation_decision(generation, "automatic", &[7, 8], "t3")
            .is_err()
    );
    assert!(
        database
            .record_telegram_rotation_decision(generation, "confirm", &[7, 8, 9], "t3")
            .is_err()
    );
    drop(database);
    let database = Database::open(&path).unwrap();
    assert_eq!(
        database.telegram_rotation_decision(generation).unwrap(),
        Some(("confirm".into(), vec![7, 8]))
    );
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn activation_and_rotation_decision_commit_together_or_not_at_all() {
    let database = Database::open_in_memory().unwrap();
    let old = database
        .prepare_telegram_credential_generation("telegram-bot:42", "old", "t0")
        .unwrap();
    database
        .activate_telegram_credential_generation(old, None)
        .unwrap();
    let candidate = database
        .prepare_telegram_credential_generation("telegram-bot:42", "new", "t1")
        .unwrap();
    // Force decision insertion to fail after the attempted state changes.
    database
        .record_telegram_rotation_decision(candidate, "confirm", &[], "t1")
        .unwrap();
    assert!(
        database
            .activate_telegram_credential_with_resume(candidate, Some(old), "automatic", "t2")
            .is_err()
    );
    assert_eq!(
        database
            .active_telegram_credential_generation()
            .unwrap()
            .unwrap()
            .0,
        old
    );
    let next = database
        .prepare_telegram_credential_generation("telegram-bot:42", "next", "t3")
        .unwrap();
    assert!(
        database
            .activate_telegram_credential_with_resume(next, Some(old), "automatic", "t4")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        database
            .active_telegram_credential_generation()
            .unwrap()
            .unwrap()
            .0,
        next
    );
    assert_eq!(
        database.telegram_rotation_decision(next).unwrap(),
        Some(("automatic".into(), vec![]))
    );
    assert!(
        database
            .activate_telegram_credential_with_resume(candidate, Some(old), "confirm", "t5")
            .is_err()
    );
}
