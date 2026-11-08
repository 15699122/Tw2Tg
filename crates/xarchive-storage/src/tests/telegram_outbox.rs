use super::*;

#[test]
fn telegram_outbox_migration_is_additive_and_isolates_legacy_rows() {
    let database = Database::open_in_memory().expect("database");
    // The 0002 contract keeps working unchanged.
    database
        .record_pending("-1001", "legacy", "metadata", OUTBOX_NOW)
        .expect("legacy pending");
    let new_tables: i64 = database
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                   AND name IN ('telegram_outbox', 'telegram_file_cache')",
            [],
            |row| row.get(0),
        )
        .expect("new tables");
    assert_eq!(new_tables, 2);
    // A legacy row carries no bot identity, so the new bot sees nothing.
    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_NOW)
            .expect("due")
            .is_empty()
    );
    let id = database
        .enqueue_outbox(outbox_entry(BOT_A, "new", 1))
        .expect("enqueue");
    assert!(id > 0);
    let scope: String = database
        .connection
        .query_row(
            "SELECT target_scope FROM telegram_outbox WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .expect("migrated target scope");
    let legacy_payload: Option<String> = database
        .connection
        .query_row(
            "SELECT payload_json FROM telegram_outbox WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .expect("legacy payload is nullable");
    assert_eq!(scope, "-1001:none");
    assert_eq!(
        legacy_payload,
        Some(outbox_entry(BOT_A, "new", 1).payload_json)
    );
    let legacy_rows: i64 = database
        .connection
        .query_row("SELECT COUNT(*) FROM telegram_send_attempts", [], |row| {
            row.get(0)
        })
        .expect("legacy count");
    assert_eq!(legacy_rows, 1, "the legacy table is untouched");
}
#[test]
fn outbox_enqueue_is_idempotent_and_plan_order_decides_the_order() {
    let database = Database::open_in_memory().expect("database");
    let first = database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    // Same bot, target and key with an identical payload is idempotent;
    // the stored plan position wins.
    let again = database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 9))
        .expect("repeat");
    assert_eq!(again, first);
    let second = database
        .enqueue_outbox(outbox_entry(BOT_A, "k2", 2))
        .expect("enqueue");
    assert_ne!(first, second);

    let due = database.list_due_outbox(BOT_A, OUTBOX_NOW).expect("due");
    assert_eq!(due.len(), 2);
    assert_eq!(due[0].idempotency_key, "k1");
    assert_eq!(due[1].idempotency_key, "k2");
    // The settings version an item was queued under travels with it.
    assert_eq!(due[0].config_version, 3);
    assert_eq!(due[0].state, OutboxState::Queued);
    assert!(!due[0].request_started);
}
#[test]
fn outbox_identity_is_bot_and_topic_scoped_and_rejects_changed_payloads() {
    let database = Database::open_in_memory().expect("database");
    let first = database
        .enqueue_outbox(outbox_entry(BOT_A, "shared-key", 1))
        .expect("first enqueue");
    assert_eq!(
        database
            .enqueue_outbox(outbox_entry(BOT_A, "shared-key", 9))
            .expect("same intent and payload is idempotent"),
        first
    );

    let mut changed = outbox_entry(BOT_A, "shared-key", 1);
    changed.payload_json = "{\"kind\":\"changed\"}".to_owned();
    assert_eq!(
        database.enqueue_outbox(changed),
        Err(SendStateError::IdempotencyConflict)
    );

    let mut other_bot = outbox_entry(BOT_B, "shared-key", 1);
    assert_ne!(
        database
            .enqueue_outbox(other_bot.clone())
            .expect("same target and key for another bot is independent"),
        first
    );
    other_bot.bot_identity = BOT_A.to_owned();
    other_bot.target_scope = "-1001:topic-7".to_owned();
    other_bot.message_thread_id = Some(7);
    other_bot.chat_id = "-1001".to_owned();
    assert_ne!(
        database
            .enqueue_outbox(other_bot)
            .expect("different topic is a distinct send target"),
        first
    );
}
#[test]
fn outbox_claim_is_exclusive_and_scoped_to_one_bot_identity() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    // Another bot never sees or claims this entry.
    assert!(
        database
            .claim_due_outbox(BOT_B, "claim-b", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .is_none()
    );
    let claimed = database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    assert_eq!(claimed.state, OutboxState::InFlight);
    assert_eq!(claimed.attempt_count, 1);
    assert_eq!(claimed.claim_token.as_deref(), Some("claim-a"));
    assert_eq!(claimed.claim_expires_at.as_deref(), Some(OUTBOX_LEASE));
    // A racing second claim finds nothing while the lease is live.
    assert!(
        database
            .claim_due_outbox(BOT_A, "claim-a2", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .is_none()
    );
    // A claimed entry is no longer "due".
    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_NOW)
            .expect("due")
            .is_empty()
    );
}
#[test]
fn outbox_transitions_require_the_claim_that_owns_the_row() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    database
        .mark_request_started("claim-a", OUTBOX_NOW)
        .expect("started");

    // A foreign or expired claim can never write over the live one.
    for stale in [
        database.record_outbox_sent("claim-b", "1", None, OUTBOX_NOW),
        database.record_outbox_retry("claim-b", OUTBOX_LATER, None, "e", OUTBOX_NOW),
        database.record_outbox_unknown("claim-b", "lost", None, "e", OUTBOX_NOW),
        database.record_outbox_failed("claim-b", Some(403), "e", OUTBOX_NOW),
        database.mark_request_started("claim-b", OUTBOX_NOW),
    ] {
        assert_eq!(stale, Err(SendStateError::StaleClaim));
    }

    database
        .record_outbox_sent("claim-a", "77", Some(r#"[{"message_id":77}]"#), OUTBOX_NOW)
        .expect("sent");
    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_NOW)
            .expect("due")
            .is_empty()
    );
    // The claim is released by the transition, so a late write from the
    // finished worker is stale as well.
    assert_eq!(
        database.record_outbox_failed("claim-a", None, "late", OUTBOX_LATER),
        Err(SendStateError::StaleClaim)
    );
}
#[test]
fn outbox_retry_waits_for_the_scheduled_time() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    database
        .record_outbox_retry(
            "claim-a",
            OUTBOX_LATER,
            Some(429),
            "Too Many Requests",
            OUTBOX_NOW,
        )
        .expect("retry");

    // Not due yet: the persisted retry_after delay is honoured.
    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_NOW)
            .expect("due")
            .is_empty()
    );
    assert!(
        database
            .claim_due_outbox(BOT_A, "claim-early", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .is_none()
    );
    let due = database
        .list_due_outbox(BOT_A, OUTBOX_LATER)
        .expect("due later");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].state, OutboxState::RetryWait);
    assert_eq!(due[0].last_error_code, Some(429));
    assert_eq!(due[0].next_retry_at.as_deref(), Some(OUTBOX_LATER));

    let again = database
        .claim_due_outbox(BOT_A, "claim-b", OUTBOX_LATER, OUTBOX_EXPIRED)
        .expect("claim")
        .expect("claimed");
    assert_eq!(again.state, OutboxState::InFlight);
    assert_eq!(again.attempt_count, 2, "each attempt is counted");
}
#[test]
fn outbox_unknown_is_never_picked_up_automatically() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    database
        .record_outbox_unknown("claim-a", "response_lost", None, "lost", OUTBOX_NOW)
        .expect("unknown");

    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_LATER)
            .expect("due")
            .is_empty()
    );
    assert!(
        database
            .claim_due_outbox(BOT_A, "claim-b", OUTBOX_LATER, OUTBOX_EXPIRED)
            .expect("claim")
            .is_none()
    );
    // Only a deliberate re-send after review may claim it again.
    let reviewed = database
        .claim_outbox(BOT_A, "k1", "claim-manual", OUTBOX_LATER, OUTBOX_EXPIRED)
        .expect("claim")
        .expect("claimed");
    assert_eq!(reviewed.state, OutboxState::InFlight);
    assert_eq!(reviewed.unknown_reason.as_deref(), Some("response_lost"));
}
#[test]
fn outbox_plan_order_holds_back_the_rest_of_an_archive_until_its_predecessor_is_sent() {
    let database = Database::open_in_memory().expect("database");
    let tweet_row = database
        .insert_tweet("7", "https://x.com/i/status/7", "tweet", "hi", OUTBOX_NOW)
        .expect("tweet row");
    let mut first = outbox_entry(BOT_A, "tweet-7:metadata", 0);
    first.tweet_id = Some(tweet_row);
    database.enqueue_outbox(first).expect("enqueue first");
    let mut second = outbox_entry(BOT_A, "tweet-7:media:1", 1);
    second.tweet_id = Some(tweet_row);
    database.enqueue_outbox(second).expect("enqueue second");

    // Only the leading unit of the archive is claimable while it runs.
    let claimed = database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    assert_eq!(claimed.idempotency_key, "tweet-7:metadata");
    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_NOW)
            .expect("due")
            .is_empty(),
        "a claimed predecessor holds the rest of the archive back"
    );
    assert!(
        database
            .claim_due_outbox(BOT_A, "claim-b", OUTBOX_NOW, OUTBOX_LEASE)
            .expect("claim")
            .is_none()
    );

    database
        .record_outbox_sent("claim-a", "msg-1", None, OUTBOX_NOW)
        .expect("sent");

    // The predecessor reached SENT, so the next planned unit is free.
    let next = database
        .claim_due_outbox(BOT_A, "claim-b", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    assert_eq!(next.idempotency_key, "tweet-7:media:1");
    assert_eq!(next.plan_order, 1);
}
#[test]
fn outbox_unit_that_cannot_be_sent_pauses_the_units_that_follow_it() {
    let database = Database::open_in_memory().expect("database");
    let tweet_row = database
        .insert_tweet("9", "https://x.com/i/status/9", "tweet", "hi", OUTBOX_NOW)
        .expect("tweet row");
    let mut first = outbox_entry(BOT_A, "tweet-9:media:1", 0);
    first.tweet_id = Some(tweet_row);
    database.enqueue_outbox(first).expect("enqueue first");
    let mut second = outbox_entry(BOT_A, "tweet-9:media:2", 1);
    second.tweet_id = Some(tweet_row);
    database.enqueue_outbox(second).expect("enqueue second");

    database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    database
        .record_outbox_failed("claim-a", Some(403), "forbidden", OUTBOX_NOW)
        .expect("fail");

    // A permanent failure is not SENT: the remaining units stay paused
    // instead of being sent in a different order than planned.
    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_LATER)
            .expect("due")
            .is_empty()
    );
    assert!(
        database
            .claim_due_outbox(BOT_A, "claim-b", OUTBOX_LATER, OUTBOX_LEASE)
            .expect("claim")
            .is_none()
    );
}
#[test]
fn outbox_rows_without_a_tweet_are_independent_and_still_claim_in_plan_order() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k2", 1))
        .expect("enqueue second");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 0))
        .expect("enqueue first");

    // No archive to sequence, so both are due and plan_order wins.
    let due = database.list_due_outbox(BOT_A, OUTBOX_NOW).expect("due");
    assert_eq!(due.len(), 2);
    assert_eq!(due[0].idempotency_key, "k1");
    assert_eq!(due[1].idempotency_key, "k2");
}
#[test]
fn outbox_lease_renewal_is_owner_only_and_moves_the_recovery_deadline() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");

    // A token that holds nothing cannot extend anything.
    assert!(
        !database
            .renew_outbox_claim("claim-a", OUTBOX_LATER, OUTBOX_NOW)
            .expect("renew")
    );

    database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    // Not the owner...
    assert!(
        !database
            .renew_outbox_claim("claim-b", OUTBOX_LATER, OUTBOX_NOW)
            .expect("renew")
    );
    // ...the owner can, while its lease is still live.
    assert!(
        !database
            .renew_outbox_claim("claim-a", OUTBOX_NOW, OUTBOX_NOW)
            .expect("reject past deadline")
    );
    // A delayed heartbeat cannot undo a newer extension.
    assert!(
        database
            .renew_outbox_claim("claim-a", OUTBOX_LEASE, OUTBOX_NOW)
            .expect("same deadline remains valid")
    );
    assert!(
        database
            .renew_outbox_claim("claim-a", OUTBOX_LATER, OUTBOX_NOW)
            .expect("renew")
    );

    // Without the renewal the lease would have expired at 00:05; the
    assert!(
        !database
            .renew_outbox_claim("claim-a", OUTBOX_LEASE, OUTBOX_NOW)
            .expect("reject shortened deadline")
    );
    // renewal pushed recovery out to OUTBOX_LATER.
    assert_eq!(
        database
            .recover_outbox_claims(OUTBOX_EXPIRED)
            .expect("recover"),
        0
    );
    assert_eq!(
        database
            .recover_outbox_claims(OUTBOX_LATER)
            .expect("recover"),
        1
    );
}
#[test]
fn outbox_expired_lease_cannot_be_resurrected_by_its_former_owner() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");

    // The lease already elapsed, so the claim belongs to recovery now.
    assert!(matches!(
        database.mark_request_started("claim-a", OUTBOX_EXPIRED),
        Err(SendStateError::StaleClaim)
    ));
    assert!(matches!(
        database.record_outbox_sent("claim-a", "123", None, OUTBOX_EXPIRED),
        Err(SendStateError::StaleClaim)
    ));
    assert!(matches!(
        database.record_outbox_retry("claim-a", OUTBOX_LATER, None, "retry", OUTBOX_EXPIRED),
        Err(SendStateError::StaleClaim)
    ));
    assert!(matches!(
        database.record_outbox_unknown("claim-a", "unknown", None, "error", OUTBOX_EXPIRED),
        Err(SendStateError::StaleClaim)
    ));
    assert!(matches!(
        database.record_outbox_failed("claim-a", None, "error", OUTBOX_EXPIRED),
        Err(SendStateError::StaleClaim)
    ));
    assert!(matches!(
        database.record_outbox_cancelled("claim-a", OUTBOX_EXPIRED),
        Err(SendStateError::StaleClaim)
    ));
    assert!(
        !database
            .renew_outbox_claim("claim-a", OUTBOX_LATER, OUTBOX_EXPIRED)
            .expect("renew")
    );
    assert_eq!(
        database
            .recover_outbox_claims(OUTBOX_EXPIRED)
            .expect("recover"),
        1
    );
}
#[test]
fn outbox_cancel_only_applies_before_the_send() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k2", 2))
        .expect("enqueue");
    assert!(
        database
            .cancel_outbox(BOT_A, "k1", OUTBOX_NOW)
            .expect("cancel")
    );
    // Idempotent: an already cancelled entry cannot be cancelled twice.
    assert!(
        !database
            .cancel_outbox(BOT_A, "k1", OUTBOX_NOW)
            .expect("repeat")
    );
    assert!(
        !database
            .cancel_outbox(BOT_A, "missing", OUTBOX_NOW)
            .expect("missing")
    );

    // A claimed entry may already be on the wire, so cancelling it must
    // fail and go through the UNKNOWN path instead.
    let claimed = database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    assert_eq!(claimed.idempotency_key, "k2");
    assert!(
        !database
            .cancel_outbox(BOT_A, "k2", OUTBOX_NOW)
            .expect("in flight")
    );
    // Another bot cannot cancel it either.
    assert!(
        !database
            .cancel_outbox(BOT_B, "k2", OUTBOX_NOW)
            .expect("other bot")
    );
}
#[test]
fn outbox_lease_recovery_separates_pre_send_from_started_requests() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "pre", 1))
        .expect("enqueue");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "started", 2))
        .expect("enqueue");
    // The first claim crashes before the network attempt, the second after.
    database
        .claim_due_outbox(BOT_A, "claim-pre", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    database
        .claim_due_outbox(BOT_A, "claim-started", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    database
        .mark_request_started("claim-started", OUTBOX_NOW)
        .expect("started");

    // Nothing is resolved while the lease is still live.
    assert_eq!(
        database.recover_outbox_claims(OUTBOX_NOW).expect("recover"),
        0
    );
    assert_eq!(
        database
            .recover_outbox_claims(OUTBOX_EXPIRED)
            .expect("recover"),
        2
    );

    // The pre-send crash returns to RETRY_WAIT and is due again.
    let due = database
        .list_due_outbox(BOT_A, OUTBOX_EXPIRED)
        .expect("due");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].idempotency_key, "pre");
    assert_eq!(due[0].state, OutboxState::RetryWait);
    assert_eq!(due[0].attempt_count, 1);

    // The started request becomes UNKNOWN and keeps its reason; it is
    // never re-sent automatically.
    let reclaimed = database
        .claim_due_outbox(BOT_A, "claim-x", OUTBOX_EXPIRED, OUTBOX_EXPIRED)
        .expect("claim")
        .expect("claimed");
    assert_eq!(reclaimed.idempotency_key, "pre");
    assert_eq!(reclaimed.state, OutboxState::InFlight);
    // Nothing else is due afterwards: the started request stayed UNKNOWN.
    assert!(
        database
            .claim_due_outbox(BOT_A, "claim-y", OUTBOX_EXPIRED, OUTBOX_EXPIRED)
            .expect("claim")
            .is_none()
    );
    let reviewed = database
        .claim_outbox(
            BOT_A,
            "started",
            "claim-manual",
            OUTBOX_EXPIRED,
            OUTBOX_EXPIRED,
        )
        .expect("claim")
        .expect("claimed");
    assert_eq!(
        reviewed.unknown_reason.as_deref(),
        Some(UNKNOWN_REASON_LEASE_EXPIRED)
    );
}
#[test]
fn file_id_cache_is_scoped_per_bot_kind_and_representation() {
    let database = Database::open_in_memory().expect("database");
    let cached = CachedFileId {
        file_id: "file-1".to_owned(),
        file_unique_id: "uniq-1".to_owned(),
        file_size: 42,
        confirmed_at: OUTBOX_NOW.to_owned(),
    };
    let key = cache_key(BOT_A, MediaKind::Photo, TELEGRAM_FILE_CACHE_VERSION);
    assert!(
        database.lookup_file_id(&key).expect("miss").is_none(),
        "an empty cache misses"
    );
    database.store_file_id(&key, &cached).expect("store");
    assert_eq!(
        database.lookup_file_id(&key).expect("hit").expect("entry"),
        cached
    );

    // Bot identity, media kind and representation version all scope an
    // entry: a token rotation or a kind change never reuses another id.
    assert!(
        database
            .lookup_file_id(&cache_key(
                BOT_B,
                MediaKind::Photo,
                TELEGRAM_FILE_CACHE_VERSION
            ))
            .expect("other bot")
            .is_none()
    );
    assert!(
        database
            .lookup_file_id(&cache_key(
                BOT_A,
                MediaKind::Video,
                TELEGRAM_FILE_CACHE_VERSION
            ))
            .expect("other kind")
            .is_none()
    );
    assert!(
        database
            .lookup_file_id(&cache_key(
                BOT_A,
                MediaKind::Photo,
                TELEGRAM_FILE_CACHE_VERSION + 1
            ))
            .expect("other version")
            .is_none()
    );
}
#[test]
fn file_id_cache_drop_and_refresh_touch_only_the_named_bot() {
    let database = Database::open_in_memory().expect("database");
    let key_a = cache_key(BOT_A, MediaKind::Photo, TELEGRAM_FILE_CACHE_VERSION);
    let key_b = cache_key(BOT_B, MediaKind::Photo, TELEGRAM_FILE_CACHE_VERSION);
    let cached = CachedFileId {
        file_id: "file-a".to_owned(),
        file_unique_id: "uniq-a".to_owned(),
        file_size: 1,
        confirmed_at: OUTBOX_NOW.to_owned(),
    };
    database.store_file_id(&key_a, &cached).expect("store a");
    database.store_file_id(&key_b, &cached).expect("store b");

    assert_eq!(database.drop_file_cache_for_bot(BOT_A).expect("drop"), 1);
    assert!(
        database.lookup_file_id(&key_a).expect("a").is_none(),
        "the named bot is purged"
    );
    assert!(
        database.lookup_file_id(&key_b).expect("b").is_some(),
        "another bot keeps its entries"
    );

    // A newer confirmed result replaces the older one for the same key.
    let refreshed = CachedFileId {
        file_id: "file-b2".to_owned(),
        file_unique_id: "uniq-b2".to_owned(),
        file_size: 2,
        confirmed_at: OUTBOX_LATER.to_owned(),
    };
    database.store_file_id(&key_b, &refreshed).expect("refresh");
    assert_eq!(
        database
            .lookup_file_id(&key_b)
            .expect("hit")
            .expect("entry"),
        refreshed
    );
}
#[test]
fn a_claimed_entry_can_be_cancelled_before_its_request_starts() {
    let database = Database::open_in_memory().expect("database");
    database
        .enqueue_outbox(outbox_entry(BOT_A, "k1", 1))
        .expect("enqueue");
    database
        .claim_due_outbox(BOT_A, "claim-a", OUTBOX_NOW, OUTBOX_LEASE)
        .expect("claim")
        .expect("claimed");
    // Nothing was sent, so the driver can record a clean cancellation.
    database
        .record_outbox_cancelled("claim-a", OUTBOX_NOW)
        .expect("cancelled");
    assert!(
        database
            .list_due_outbox(BOT_A, OUTBOX_LATER)
            .expect("due")
            .is_empty(),
        "a cancelled entry never returns to the queue"
    );
    // A claim token that no longer owns the row cannot write again.
    assert_eq!(
        database.record_outbox_cancelled("claim-a", OUTBOX_LATER),
        Err(SendStateError::StaleClaim)
    );
}
#[test]
fn a_tweets_outbox_rows_are_read_in_plan_order_for_one_bot() {
    let database = Database::open_in_memory().expect("database");
    let tweet = database
        .insert_tweet(
            "1961",
            "https://x.com/a/status/1961",
            "post",
            "text",
            OUTBOX_NOW,
        )
        .expect("tweet");
    for (key, order) in [("first", 2), ("second", 1)] {
        let mut entry = outbox_entry(BOT_A, key, order);
        entry.tweet_id = Some(tweet);
        database.enqueue_outbox(entry).expect("enqueue");
    }
    // Another bot's row for the same Tweet stays invisible.
    let mut other_bot = outbox_entry(BOT_B, "other", 0);
    other_bot.tweet_id = Some(tweet);
    database.enqueue_outbox(other_bot).expect("enqueue");

    let rows = database
        .list_outbox_for_tweet(BOT_A, tweet)
        .expect("rows for the task projection");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].idempotency_key, "second", "plan order decides");
    assert_eq!(rows[1].idempotency_key, "first");
    assert_eq!(rows[0].state, OutboxState::Queued);
    assert!(
        database
            .list_outbox_for_tweet(BOT_A, tweet + 1)
            .expect("other tweet")
            .is_empty()
    );
}
