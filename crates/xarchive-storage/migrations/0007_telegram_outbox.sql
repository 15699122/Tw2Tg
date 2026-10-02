-- Telegram outbox and bot-isolated file_id cache (plan TG-04 / TG-05).
--
-- Additive by design: 0002's telegram_send_attempts keeps serving the older
-- idempotent-send contract and is not modified. Rows written before a bot
-- identity existed stay there and are therefore excluded from this outbox
-- and from the file_id cache.

CREATE TABLE telegram_outbox (
    id INTEGER PRIMARY KEY,
    -- One-way fingerprint of the bot token, never the token itself.
    bot_identity TEXT NOT NULL,
    chat_id TEXT NOT NULL,
    message_thread_id INTEGER,
    idempotency_key TEXT NOT NULL,
    request_fingerprint TEXT NOT NULL,
    message_kind TEXT NOT NULL,
    -- Settings version this entry was queued under; later config edits apply
    -- only to new items (plan TG-06).
    config_version INTEGER NOT NULL DEFAULT 1,
    plan_version INTEGER NOT NULL DEFAULT 1,
    plan_order INTEGER NOT NULL DEFAULT 0,
    tweet_id INTEGER REFERENCES tweets(id) ON DELETE CASCADE,
    media_reference TEXT,
    content_sha256 TEXT,
    state TEXT NOT NULL CHECK (state IN (
        'QUEUED', 'IN_FLIGHT', 'SENT',
        'RETRY_WAIT', 'FAILED_PERMANENT', 'UNKNOWN', 'CANCELLED'
    )),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    claim_token TEXT,
    claim_expires_at TEXT,
    -- 0 until the network attempt starts; from then a lost lease can only
    -- resolve to UNKNOWN, never to an automatic re-send.
    request_started INTEGER NOT NULL DEFAULT 0 CHECK (request_started IN (0, 1)),
    next_retry_at TEXT,
    telegram_message_id TEXT,
    -- Per-item album results: JSON array of {message_id, file_id}.
    results_json TEXT,
    last_error_code INTEGER,
    -- Caller-redacted: never a token or a credential-bearing URL.
    last_error_message TEXT,
    unknown_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(chat_id, idempotency_key)
);

CREATE INDEX telegram_outbox_due_idx
    ON telegram_outbox(bot_identity, state, next_retry_at, plan_order);

CREATE INDEX telegram_outbox_claim_idx
    ON telegram_outbox(state, claim_expires_at);

CREATE TABLE telegram_file_cache (
    id INTEGER PRIMARY KEY,
    bot_identity TEXT NOT NULL,
    content_sha256 TEXT NOT NULL,
    media_kind TEXT NOT NULL,
    representation_version INTEGER NOT NULL,
    file_id TEXT NOT NULL,
    -- Identification only, never a send parameter.
    file_unique_id TEXT NOT NULL,
    file_size INTEGER NOT NULL CHECK (file_size >= 0),
    confirmed_at TEXT NOT NULL,
    UNIQUE(bot_identity, content_sha256, media_kind, representation_version)
);