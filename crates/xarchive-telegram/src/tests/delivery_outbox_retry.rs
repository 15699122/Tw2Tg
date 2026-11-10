//! Delivery and outbox suites: idempotent send, retry classification and
//! the claim-fenced attempt driver.

use super::*;

#[test]
fn a_control_request_with_a_lost_response_is_not_safe_to_retry() {
    let (endpoint, server) = fake_server("not-json");
    let transport = ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
    let token = BotToken::new("123:secret").expect("token");
    let error = transport
        .send(
            &token,
            TelegramRequest::Message(SendMessageRequest {
                chat_id: "-100".into(),
                message_thread_id: Some(42),
                text: "send once".into(),
                disable_web_page_preview: true,
            }),
        )
        .expect_err("invalid response after request is sent");
    let request = server.join().expect("server");
    assert!(request.contains("send once"));
    assert_eq!(
        classify_send_failure(&error, RequestProgress::Sent),
        SendFailure::Unknown
    );
}

#[derive(Default)]
struct MemorySendStateStore {
    records: std::sync::Mutex<std::collections::HashMap<(String, String), MemorySendRecord>>,
}

#[derive(Clone)]
struct MemorySendRecord {
    message_kind: String,
    state: SendState,
    attempt_count: u32,
    telegram_message_id: Option<String>,
    telegram_file_id: Option<String>,
}

impl SendStateStore for MemorySendStateStore {
    fn find_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<SentSendRecord>, SendStateError> {
        let records = self.records.lock().expect("lock");
        Ok(records
            .get(&(chat_id.to_owned(), idempotency_key.to_owned()))
            .filter(|record| record.state == SendState::Sent)
            .map(|record| SentSendRecord {
                chat_id: chat_id.to_owned(),
                idempotency_key: idempotency_key.to_owned(),
                message_kind: record.message_kind.clone(),
                telegram_message_id: record.telegram_message_id.clone().expect("sent id"),
                telegram_file_id: record.telegram_file_id.clone(),
            }))
    }

    fn record_pending(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        message_kind: &str,
        _updated_at: &str,
    ) -> Result<(), SendStateError> {
        let mut records = self.records.lock().expect("lock");
        let record = records
            .entry((chat_id.to_owned(), idempotency_key.to_owned()))
            .or_insert_with(|| MemorySendRecord {
                message_kind: message_kind.to_owned(),
                state: SendState::Pending,
                attempt_count: 0,
                telegram_message_id: None,
                telegram_file_id: None,
            });
        record.state = SendState::Pending;
        record.attempt_count += 1;
        Ok(())
    }

    fn record_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        telegram_message_id: &str,
        telegram_file_id: Option<&str>,
        _updated_at: &str,
    ) -> Result<(), SendStateError> {
        let mut records = self.records.lock().expect("lock");
        let record = records
            .get_mut(&(chat_id.to_owned(), idempotency_key.to_owned()))
            .ok_or(SendStateError::NotFound)?;
        record.state = SendState::Sent;
        record.telegram_message_id = Some(telegram_message_id.to_owned());
        record.telegram_file_id = telegram_file_id.map(str::to_owned);
        Ok(())
    }

    fn record_failed(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        _error_code: Option<i64>,
        _error_message: &str,
        _updated_at: &str,
    ) -> Result<(), SendStateError> {
        let mut records = self.records.lock().expect("lock");
        let record = records
            .get_mut(&(chat_id.to_owned(), idempotency_key.to_owned()))
            .ok_or(SendStateError::NotFound)?;
        record.state = SendState::Failed;
        Ok(())
    }

    fn list_unsent(&self) -> Result<Vec<PendingSendRecord>, SendStateError> {
        let records = self.records.lock().expect("lock");
        let mut pending: Vec<PendingSendRecord> = records
            .iter()
            .filter(|(_, record)| record.state != SendState::Sent)
            .map(|((chat_id, idempotency_key), record)| PendingSendRecord {
                chat_id: chat_id.clone(),
                idempotency_key: idempotency_key.clone(),
                message_kind: record.message_kind.clone(),
                state: record.state,
                attempt_count: record.attempt_count,
            })
            .collect();
        pending.sort_by(|left, right| left.idempotency_key.cmp(&right.idempotency_key));
        Ok(pending)
    }
}

#[test]
fn idempotent_send_skips_transport_when_already_sent() {
    let store = MemorySendStateStore::default();
    store
        .record_pending("-100", "tweet-1:metadata", "metadata", "now")
        .expect("pending");
    store
        .record_sent("-100", "tweet-1:metadata", "77", Some("file-id-1"), "now")
        .expect("sent");
    let outcome = send_idempotently(
        &store,
        "-100",
        "tweet-1:metadata",
        "metadata",
        "now",
        || panic!("deliver must not run for already-sent messages"),
    )
    .expect("outcome");
    assert_eq!(
        outcome,
        IdempotentSendOutcome::AlreadySent {
            telegram_message_id: "77".into()
        }
    );
}

#[test]
fn idempotent_send_records_delivered_state_and_stays_stable() {
    let store = MemorySendStateStore::default();
    let outcome = send_idempotently(&store, "-100", "tweet-1:media:1", "media", "now", || {
        Ok(DeliveredSend {
            telegram_message_id: "42".into(),
            telegram_file_id: Some("photo-file-id".into()),
        })
    })
    .expect("outcome");
    assert_eq!(
        outcome,
        IdempotentSendOutcome::Delivered(DeliveredSend {
            telegram_message_id: "42".into(),
            telegram_file_id: Some("photo-file-id".into()),
        })
    );
    let sent = store
        .find_sent("-100", "tweet-1:media:1")
        .expect("find")
        .expect("sent record");
    assert_eq!(sent.telegram_message_id, "42");
    assert_eq!(sent.telegram_file_id.as_deref(), Some("photo-file-id"));
    assert!(store.list_unsent().expect("unsent").is_empty());
    let outcome = send_idempotently(&store, "-100", "tweet-1:media:1", "media", "now", || {
        panic!("deliver must not run twice for the same key")
    })
    .expect("outcome");
    assert_eq!(
        outcome,
        IdempotentSendOutcome::AlreadySent {
            telegram_message_id: "42".into()
        }
    );
}

#[test]
fn idempotent_send_persists_failure_and_retries_with_same_key() {
    let store = MemorySendStateStore::default();
    let error = send_idempotently(
        &store,
        "-100",
        "tweet-2:metadata",
        "metadata",
        "now",
        || {
            Err(TelegramError::Api {
                code: 429,
                description: "Too Many Requests".into(),
                parameters: None,
            })
        },
    )
    .expect_err("first attempt fails");
    assert_eq!(
        error,
        IdempotentSendError::Telegram(TelegramError::Api {
            code: 429,
            description: "Too Many Requests".into(),
            parameters: None,
        })
    );
    let unsent = store.list_unsent().expect("unsent");
    assert_eq!(unsent.len(), 1);
    assert_eq!(unsent[0].state, SendState::Failed);
    assert_eq!(unsent[0].attempt_count, 1);
    assert_eq!(unsent[0].message_kind, "metadata");

    let outcome = send_idempotently(
        &store,
        "-100",
        "tweet-2:metadata",
        "metadata",
        "now",
        || {
            Ok(DeliveredSend {
                telegram_message_id: "9001".into(),
                telegram_file_id: None,
            })
        },
    )
    .expect("retry succeeds");
    assert_eq!(
        outcome,
        IdempotentSendOutcome::Delivered(DeliveredSend {
            telegram_message_id: "9001".into(),
            telegram_file_id: None,
        })
    );
    assert!(store.list_unsent().expect("unsent").is_empty());
    let sent = store
        .find_sent("-100", "tweet-2:metadata")
        .expect("find")
        .expect("sent record");
    assert_eq!(sent.telegram_message_id, "9001");
}

#[test]
fn classifies_send_failures_into_the_retry_policy() {
    let rate_limited = TelegramError::Api {
        code: 429,
        description: "Too Many Requests: retry after 7".into(),
        parameters: Some(serde_json::json!({ "retry_after": 7 })),
    };
    assert_eq!(
        classify_send_failure(&rate_limited, RequestProgress::NotSent),
        SendFailure::Retry {
            retry_after: Some(Duration::from_secs(7))
        }
    );
    let no_parameters = TelegramError::Api {
        code: 429,
        description: "Too Many Requests".into(),
        parameters: None,
    };
    assert_eq!(
        classify_send_failure(&no_parameters, RequestProgress::Sent),
        SendFailure::Retry {
            retry_after: Some(TELEGRAM_DEFAULT_RETRY_AFTER)
        }
    );

    for code in [401_i64, 403] {
        let error = TelegramError::Api {
            code,
            description: "unauthorized".into(),
            parameters: None,
        };
        assert_eq!(
            classify_send_failure(&error, RequestProgress::NotSent),
            SendFailure::Permanent,
            "auth failure {code} must stop"
        );
    }
    let target = TelegramError::Api {
        code: 400,
        description: "Bad Request: chat not found".into(),
        parameters: None,
    };
    assert_eq!(
        classify_send_failure(&target, RequestProgress::Sent),
        SendFailure::Permanent
    );
    let media = TelegramError::Api {
        code: 400,
        description: "Bad Request: PHOTO_INVALID_DIMENSIONS".into(),
        parameters: None,
    };
    assert_eq!(
        classify_send_failure(&media, RequestProgress::NotSent),
        SendFailure::MediaCorrection
    );
    let server = TelegramError::Api {
        code: 500,
        description: "Internal Server Error".into(),
        parameters: None,
    };
    assert_eq!(
        classify_send_failure(&server, RequestProgress::NotSent),
        SendFailure::Retry { retry_after: None }
    );

    // Response-loss and server-timeout rows are UNKNOWN regardless of
    // progress: never an automatic re-send.
    for error in [
        TelegramError::ResponseLost,
        TelegramError::UploadServerTimeout,
        TelegramError::ResponseTooLarge,
    ] {
        for progress in [
            RequestProgress::NotSent,
            RequestProgress::Partial,
            RequestProgress::Sent,
        ] {
            assert_eq!(
                classify_send_failure(&error, progress),
                SendFailure::Unknown
            );
        }
    }

    // Transport rows depend on whether the full request went out.
    let transport = TelegramError::Transport("boom".into());
    assert_eq!(
        classify_send_failure(&transport, RequestProgress::NotSent),
        SendFailure::Retry { retry_after: None }
    );
    assert_eq!(
        classify_send_failure(&transport, RequestProgress::Sent),
        SendFailure::Unknown
    );
    assert_eq!(
        classify_send_failure(&TelegramError::UploadBodyStalled, RequestProgress::Partial),
        SendFailure::Retry { retry_after: None }
    );
    assert_eq!(
        classify_send_failure(
            &TelegramError::UploadConnectTimeout,
            RequestProgress::NotSent
        ),
        SendFailure::Retry { retry_after: None }
    );
    assert_eq!(
        classify_send_failure(
            &TelegramError::UploadDeadlineExceeded,
            RequestProgress::Sent
        ),
        SendFailure::Unknown
    );

    // Cancellation records whether it raced a sent request.
    assert_eq!(
        classify_send_failure(&TelegramError::UploadCancelled, RequestProgress::NotSent),
        SendFailure::Cancelled { after_send: false }
    );
    assert_eq!(
        classify_send_failure(&TelegramError::UploadCancelled, RequestProgress::Sent),
        SendFailure::Cancelled { after_send: true }
    );

    // Config faults are permanent; input faults need correction.
    assert_eq!(
        classify_send_failure(&TelegramError::InvalidEndpoint, RequestProgress::NotSent),
        SendFailure::Permanent
    );
    assert_eq!(
        classify_send_failure(
            &TelegramError::FileChanged("size".into()),
            RequestProgress::NotSent
        ),
        SendFailure::MediaCorrection
    );
}

#[test]
fn bounded_backoff_grows_then_caps() {
    assert_eq!(bounded_backoff(0), Duration::from_secs(5));
    assert_eq!(bounded_backoff(1), Duration::from_secs(10));
    assert_eq!(bounded_backoff(2), Duration::from_secs(20));
    assert!(bounded_backoff(4) > bounded_backoff(2));
    assert_eq!(bounded_backoff(20), Duration::from_secs(30 * 60));
    assert_eq!(bounded_backoff(u32::MAX), Duration::from_secs(30 * 60));
}

#[test]
fn outbox_states_roundtrip_and_gate_automatic_sending() {
    for state in [
        OutboxState::Queued,
        OutboxState::InFlight,
        OutboxState::Sent,
        OutboxState::RetryWait,
        OutboxState::FailedPermanent,
        OutboxState::Unknown,
        OutboxState::Cancelled,
    ] {
        assert_eq!(OutboxState::parse(state.as_str()), Some(state));
    }
    assert_eq!(OutboxState::parse("WEIRD"), None);
    // Only queued/retry entries may be picked up automatically; UNKNOWN
    // requires human review and must never be re-sent by default.
    assert!(OutboxState::Queued.auto_sendable());
    assert!(OutboxState::RetryWait.auto_sendable());
    assert!(!OutboxState::Unknown.auto_sendable());
    assert!(!OutboxState::InFlight.auto_sendable());
    assert!(!OutboxState::Sent.auto_sendable());
    assert!(OutboxState::Sent.is_terminal());
    assert!(OutboxState::FailedPermanent.is_terminal());
    assert!(OutboxState::Cancelled.is_terminal());
    assert!(!OutboxState::Unknown.is_terminal());
}

#[test]
fn failed_attempts_map_onto_the_next_durable_transition() {
    // A rate limit uses the server's own delay; anything else without one
    // uses bounded backoff from the attempt number.
    let rate_limited = SendFailure::Retry {
        retry_after: Some(Duration::from_secs(42)),
    };
    assert_eq!(
        decide_outbox_transition(&rate_limited, 3),
        OutboxDecision::RetryAfter(Duration::from_secs(42))
    );
    let backoff = SendFailure::Retry { retry_after: None };
    assert_eq!(
        decide_outbox_transition(&backoff, 0),
        OutboxDecision::RetryAfter(bounded_backoff(0))
    );
    assert_eq!(
        decide_outbox_transition(&backoff, 2),
        OutboxDecision::RetryAfter(bounded_backoff(2))
    );

    // Response loss and a cancel that raced a sent request both become
    // UNKNOWN, with distinct reasons so the review UI can explain them.
    assert_eq!(
        decide_outbox_transition(&SendFailure::Unknown, 0),
        OutboxDecision::Unknown(UNKNOWN_REASON_RESPONSE_LOST)
    );
    assert_eq!(
        decide_outbox_transition(&SendFailure::Cancelled { after_send: true }, 0),
        OutboxDecision::Unknown(UNKNOWN_REASON_CANCELLED_AFTER_SEND)
    );
    assert_eq!(
        decide_outbox_transition(&SendFailure::Cancelled { after_send: false }, 0),
        OutboxDecision::Cancelled
    );

    assert_eq!(
        decide_outbox_transition(&SendFailure::Permanent, 0),
        OutboxDecision::FailedPermanent
    );
    assert_eq!(
        decide_outbox_transition(&SendFailure::MediaCorrection, 0),
        OutboxDecision::NeedsMediaCorrection
    );
    // Only a plain retry ever schedules another automatic attempt.
    for failure in [
        SendFailure::Unknown,
        SendFailure::Permanent,
        SendFailure::MediaCorrection,
        SendFailure::Cancelled { after_send: true },
        SendFailure::Cancelled { after_send: false },
    ] {
        assert!(
            !matches!(
                decide_outbox_transition(&failure, 0),
                OutboxDecision::RetryAfter(_)
            ),
            "{failure:?} must not schedule an automatic retry"
        );
    }
}

// ---------------------------------------------------------------------
// Attempt driver and send plans
// ---------------------------------------------------------------------

/// In-memory `TelegramOutboxStore` for driver tests: it records every
/// transition the driver persists so the protocol can be asserted without
/// a database.
#[derive(Default)]
struct RecordingOutbox {
    state: Mutex<RecordingOutboxState>,
}

#[derive(Default)]
struct RecordingOutboxState {
    writes: Vec<String>,
    fail_with: Option<SendStateError>,
}

impl RecordingOutbox {
    fn writes(&self) -> Vec<String> {
        self.state.lock().expect("lock").writes.clone()
    }

    fn fail_next_with(&self, error: SendStateError) {
        self.state.lock().expect("lock").fail_with = Some(error);
    }

    fn record(&self, entry: String) -> Result<(), SendStateError> {
        let mut state = self.state.lock().expect("lock");
        if let Some(error) = state.fail_with.take() {
            return Err(error);
        }
        state.writes.push(entry);
        Ok(())
    }
}

impl TelegramOutboxStore for RecordingOutbox {
    fn enqueue_outbox(&self, _entry: NewOutboxEntry) -> Result<i64, SendStateError> {
        Ok(1)
    }

    fn claim_due_outbox(
        &self,
        _bot_identity: &str,
        _claim_token: &str,
        _now: &str,
        _lease_until: &str,
    ) -> Result<Option<OutboxEntry>, SendStateError> {
        Ok(None)
    }

    fn claim_outbox(
        &self,
        _bot_identity: &str,
        _idempotency_key: &str,
        _claim_token: &str,
        _now: &str,
        _lease_until: &str,
    ) -> Result<Option<OutboxEntry>, SendStateError> {
        Ok(None)
    }

    fn mark_request_started(&self, claim_token: &str, _now: &str) -> Result<(), SendStateError> {
        assert_eq!(claim_token, "claim-1");
        self.record("started".to_owned())
    }

    fn renew_outbox_claim(
        &self,
        _claim_token: &str,
        _lease_until: &str,
        _now: &str,
    ) -> Result<bool, SendStateError> {
        Ok(true)
    }

    fn record_outbox_sent(
        &self,
        _claim_token: &str,
        telegram_message_id: &str,
        results_json: Option<&str>,
        _now: &str,
    ) -> Result<(), SendStateError> {
        self.record(format!("sent:{telegram_message_id}:{results_json:?}"))
    }

    fn record_outbox_retry(
        &self,
        _claim_token: &str,
        next_retry_at: &str,
        error_code: Option<i64>,
        error_message: &str,
        _now: &str,
    ) -> Result<(), SendStateError> {
        self.record(format!(
            "retry:{next_retry_at}:{error_code:?}:{error_message}"
        ))
    }

    fn record_outbox_unknown(
        &self,
        _claim_token: &str,
        reason: &str,
        error_code: Option<i64>,
        error_message: &str,
        _now: &str,
    ) -> Result<(), SendStateError> {
        self.record(format!("unknown:{reason}:{error_code:?}:{error_message}"))
    }

    fn record_outbox_failed(
        &self,
        _claim_token: &str,
        error_code: Option<i64>,
        error_message: &str,
        _now: &str,
    ) -> Result<(), SendStateError> {
        self.record(format!("failed:{error_code:?}:{error_message}"))
    }

    fn record_outbox_cancelled(
        &self,
        _claim_token: &str,
        _now: &str,
    ) -> Result<(), SendStateError> {
        self.record("cancelled".to_owned())
    }

    fn cancel_outbox(
        &self,
        _bot_identity: &str,
        _idempotency_key: &str,
        _now: &str,
    ) -> Result<bool, SendStateError> {
        Ok(false)
    }

    fn recover_outbox_claims(&self, _now: &str) -> Result<u64, SendStateError> {
        Ok(0)
    }

    fn list_due_outbox(
        &self,
        _bot_identity: &str,
        _now: &str,
    ) -> Result<Vec<OutboxEntry>, SendStateError> {
        Ok(Vec::new())
    }
}

fn schedule_label(delay: Duration) -> String {
    format!("due+{}s", delay.as_secs())
}

#[test]
fn clock_aware_attempt_reads_time_before_and_after_io() {
    let store = RecordingOutbox::default();
    let reads = std::cell::Cell::new(0);
    block_on(run_claimed_attempt_with_clock(
        &store,
        "claim-1",
        0,
        || {
            reads.set(reads.get() + 1);
            format!("time-{}", reads.get())
        },
        schedule_label,
        async {
            assert_eq!(reads.get(), 1);
            Ok(SendAttemptSuccess {
                telegram_message_id: "88".into(),
                results_json: None,
            })
        },
    ))
    .unwrap();
    assert_eq!(reads.get(), 2);
    assert_eq!(store.writes()[0], "started");
}

#[test]
fn attempt_driver_fences_before_sending_and_records_a_confirmed_result() {
    let store = RecordingOutbox::default();
    let success = block_on(run_claimed_attempt(
        &store,
        "claim-1",
        0,
        "2026-10-01T00:00:00Z",
        schedule_label,
        async {
            Ok(SendAttemptSuccess {
                telegram_message_id: "77".to_owned(),
                results_json: Some("{\"file_ids\":[\"p1\"]}".to_owned()),
            })
        },
    ))
    .expect("attempt");
    assert_eq!(success.telegram_message_id, "77");
    // The fence is written before anything else, so a crash after it can
    // only ever resolve to UNKNOWN, never to a silent re-send.
    let writes = store.writes();
    assert_eq!(writes[0], "started");
    assert!(writes[1].starts_with("sent:77:"));
}

#[test]
fn attempt_driver_persists_the_retry_policy_of_each_failure_class() {
    // Rate limit: the server's own delay wins over the backoff.
    let store = RecordingOutbox::default();
    let error = block_on(run_claimed_attempt(
        &store,
        "claim-1",
        2,
        "2026-10-01T00:00:00Z",
        schedule_label,
        async {
            Err(SendAttemptError::new(
                TelegramError::Api {
                    code: 429,
                    description: "Too Many Requests".to_owned(),
                    parameters: Some(serde_json::json!({ "retry_after": 7 })),
                },
                RequestProgress::Sent,
            ))
        },
    ))
    .expect_err("failed attempt");
    assert_eq!(
        error,
        RunAttemptError::Failed {
            classification: SendFailure::Retry {
                retry_after: Some(Duration::from_secs(7))
            },
            transition: OutboxDecision::RetryAfter(Duration::from_secs(7)),
            message: "Telegram API error 429: Too Many Requests".to_owned(),
        }
    );
    assert_eq!(
        store.writes(),
        vec![
            "started".to_owned(),
            "retry:due+7s:Some(429):Telegram API error 429: Too Many Requests".to_owned()
        ]
    );

    // Response loss: UNKNOWN with its reason, never a scheduled re-send.
    let store = RecordingOutbox::default();
    let error = block_on(run_claimed_attempt(
        &store,
        "claim-1",
        0,
        "2026-10-01T00:00:00Z",
        schedule_label,
        async {
            Err(SendAttemptError::new(
                TelegramError::ResponseLost,
                RequestProgress::Sent,
            ))
        },
    ))
    .expect_err("failed attempt");
    assert!(matches!(
        error,
        RunAttemptError::Failed {
            transition: OutboxDecision::Unknown(UNKNOWN_REASON_RESPONSE_LOST),
            ..
        }
    ));
    assert!(
        store.writes()[1].starts_with("unknown:response_lost:"),
        "{:?}",
        store.writes()
    );

    // Auth failure: permanent, so no automatic attempt is scheduled.
    let store = RecordingOutbox::default();
    let error = block_on(run_claimed_attempt(
        &store,
        "claim-1",
        0,
        "2026-10-01T00:00:00Z",
        schedule_label,
        async {
            Err(SendAttemptError::new(
                TelegramError::Api {
                    code: 401,
                    description: "Unauthorized".to_owned(),
                    parameters: None,
                },
                RequestProgress::Sent,
            ))
        },
    ))
    .expect_err("failed attempt");
    assert!(matches!(
        error,
        RunAttemptError::Failed {
            transition: OutboxDecision::FailedPermanent,
            ..
        }
    ));
    assert!(
        store.writes()[1].starts_with("failed:Some(401):"),
        "{:?}",
        store.writes()
    );

    // Cancel before the send: a clean cancel with no uncertainty.
    let store = RecordingOutbox::default();
    let error = block_on(run_claimed_attempt(
        &store,
        "claim-1",
        0,
        "2026-10-01T00:00:00Z",
        schedule_label,
        async {
            Err(SendAttemptError::new(
                TelegramError::UploadCancelled,
                RequestProgress::NotSent,
            ))
        },
    ))
    .expect_err("cancelled attempt");
    assert!(matches!(
        error,
        RunAttemptError::Failed {
            transition: OutboxDecision::Cancelled,
            ..
        }
    ));
    assert_eq!(store.writes(), vec!["started", "cancelled"]);

    // Media-parameter error: leaves the automatic queue for re-planning.
    let store = RecordingOutbox::default();
    let error = block_on(run_claimed_attempt(
        &store,
        "claim-1",
        0,
        "2026-10-01T00:00:00Z",
        schedule_label,
        async {
            Err(SendAttemptError::new(
                TelegramError::Api {
                    code: 400,
                    description: "Bad Request: PHOTO_INVALID_DIMENSIONS".to_owned(),
                    parameters: None,
                },
                RequestProgress::Sent,
            ))
        },
    ))
    .expect_err("failed attempt");
    assert!(matches!(
        error,
        RunAttemptError::Failed {
            transition: OutboxDecision::NeedsMediaCorrection,
            ..
        }
    ));
    assert!(
        store.writes()[1].contains("PHOTO_INVALID_DIMENSIONS"),
        "{:?}",
        store.writes()
    );
}

#[test]
fn attempt_driver_drops_a_lost_claim_without_sending_or_writing() {
    let store = RecordingOutbox::default();
    store.fail_next_with(SendStateError::StaleClaim);
    let sent = std::sync::atomic::AtomicBool::new(false);
    let error = block_on(run_claimed_attempt(
        &store,
        "claim-1",
        0,
        "2026-10-01T00:00:00Z",
        schedule_label,
        async {
            sent.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(SendAttemptSuccess {
                telegram_message_id: "77".to_owned(),
                results_json: None,
            })
        },
    ))
    .expect_err("stale claim");
    assert_eq!(error, RunAttemptError::StaleClaim);
    assert!(
        !sent.load(std::sync::atomic::Ordering::SeqCst),
        "a lost claim must stop before the request is sent"
    );
    assert!(store.writes().is_empty(), "no transition may be written");
}
