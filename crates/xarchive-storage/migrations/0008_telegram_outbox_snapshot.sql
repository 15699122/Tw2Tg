-- Telegram outbox durable payload and identity scope (TG-06).
-- Preserve historical rows. Their absent payload snapshot explicitly means
-- they cannot be safely resumed until they are manually replanned.

ALTER TABLE telegram_outbox RENAME TO telegram_outbox_legacy;

CREATE TABLE telegram_outbox (
    id INTEGER PRIMARY KEY,
    bot_identity TEXT NOT NULL,
    target_scope TEXT NOT NULL,
    chat_id TEXT NOT NULL,
    message_thread_id INTEGER,
    idempotency_key TEXT NOT NULL,
    request_fingerprint TEXT NOT NULL,
    message_kind TEXT NOT NULL,
    config_version INTEGER NOT NULL DEFAULT 1,
    plan_version INTEGER NOT NULL DEFAULT 1,
    plan_order INTEGER NOT NULL DEFAULT 0,
    tweet_id INTEGER REFERENCES tweets(id) ON DELETE CASCADE,
    media_reference TEXT,
    content_sha256 TEXT,
    payload_schema_version INTEGER,
    payload_json TEXT,
    state TEXT NOT NULL CHECK (state IN (
        'QUEUED', 'IN_FLIGHT', 'SENT',
        'RETRY_WAIT', 'FAILED_PERMANENT', 'UNKNOWN', 'CANCELLED'
    )),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    claim_token TEXT,
    claim_expires_at TEXT,
    request_started INTEGER NOT NULL DEFAULT 0 CHECK (request_started IN (0, 1)),
    next_retry_at TEXT,
    telegram_message_id TEXT,
    results_json TEXT,
    last_error_code INTEGER,
    last_error_message TEXT,
    unknown_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(bot_identity, target_scope, idempotency_key)
);

INSERT INTO telegram_outbox (
    id, bot_identity, target_scope, chat_id, message_thread_id, idempotency_key,
    request_fingerprint, message_kind, config_version, plan_version, plan_order,
    tweet_id, media_reference, content_sha256, payload_schema_version, payload_json,
    state, attempt_count, claim_token, claim_expires_at, request_started,
    next_retry_at, telegram_message_id, results_json, last_error_code,
    last_error_message, unknown_reason, created_at, updated_at
)
SELECT
    id, bot_identity,
    chat_id || ':' || COALESCE(CAST(message_thread_id AS TEXT), 'none'),
    chat_id, message_thread_id, idempotency_key, request_fingerprint, message_kind,
    config_version, plan_version, plan_order, tweet_id, media_reference, content_sha256,
    NULL, NULL, state, attempt_count, claim_token, claim_expires_at, request_started,
    next_retry_at, telegram_message_id, results_json, last_error_code,
    last_error_message, unknown_reason, created_at, updated_at
FROM telegram_outbox_legacy;

DROP TABLE telegram_outbox_legacy;

CREATE INDEX telegram_outbox_due_idx
    ON telegram_outbox(bot_identity, state, next_retry_at, plan_order);

CREATE INDEX telegram_outbox_claim_idx
    ON telegram_outbox(state, claim_expires_at);