-- Durable boundary between filesystem archive completion and Telegram outbox
-- planning. This journal is recoverable but is not a cross-resource transaction.
CREATE TABLE telegram_archive_intents (
    tweet_id INTEGER PRIMARY KEY REFERENCES tweets(id) ON DELETE CASCADE,
    job_id TEXT NOT NULL UNIQUE REFERENCES jobs(id) ON DELETE CASCADE,
    archive_directory TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('PREPARED', 'ARCHIVED', 'QUEUED', 'SKIPPED')),
    metadata_text TEXT,
    media_json TEXT NOT NULL,
    bot_identity TEXT,
    chat_id TEXT,
    message_thread_id INTEGER,
    config_revision INTEGER,
    plan_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (state = 'SKIPPED' AND bot_identity IS NULL AND chat_id IS NULL)
        OR (state <> 'SKIPPED' AND bot_identity IS NOT NULL AND chat_id IS NOT NULL)
    )
);

CREATE INDEX telegram_archive_intents_recovery_idx
    ON telegram_archive_intents(state, updated_at);