PRAGMA foreign_keys = ON;

-- Telegram send-state persistence and idempotent re-send (M2).
-- tweet_id is nullable because the cross-platform SendStateStore contract
-- does not carry the storage-layer tweet row id.

CREATE TABLE telegram_send_attempts (
    id INTEGER PRIMARY KEY,
    tweet_id INTEGER REFERENCES tweets(id) ON DELETE CASCADE,
    chat_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    message_kind TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('PENDING', 'SENT', 'FAILED')),
    attempt_count INTEGER NOT NULL DEFAULT 1 CHECK (attempt_count >= 1),
    telegram_message_id TEXT,
    telegram_file_id TEXT,
    last_error_code TEXT,
    last_error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(chat_id, idempotency_key)
);

CREATE INDEX telegram_send_attempts_state_idx
    ON telegram_send_attempts(state, updated_at DESC);