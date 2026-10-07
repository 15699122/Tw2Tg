-- Marks jobs whose archive recovery must use independent internal facts.
-- Existing jobs remain NULL and continue through the legacy recovery path.
ALTER TABLE jobs ADD COLUMN recovery_contract_version INTEGER
    CHECK (recovery_contract_version IS NULL OR recovery_contract_version > 0);

CREATE TABLE archive_recovery_journal (
    job_id TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
    attempt_count INTEGER NOT NULL CHECK (attempt_count > 0),
    tweet_row_id INTEGER NOT NULL REFERENCES tweets(id) ON DELETE CASCADE,
    archive_directory TEXT NOT NULL,
    manifest_schema_version INTEGER NOT NULL CHECK (manifest_schema_version > 0),
    manifest_json TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL CHECK (length(manifest_sha256) = 64),
    telegram_intent_required INTEGER NOT NULL CHECK (telegram_intent_required IN (0, 1)),
    telegram_tweet_row_id INTEGER REFERENCES telegram_archive_intents(tweet_id),
    phase TEXT NOT NULL CHECK (phase IN ('PREPARED', 'COMMITTED')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((telegram_intent_required = 1 AND telegram_tweet_row_id = tweet_row_id)
        OR (telegram_intent_required = 0 AND telegram_tweet_row_id IS NULL))
);

CREATE INDEX archive_recovery_journal_phase_idx
    ON archive_recovery_journal(phase, updated_at);