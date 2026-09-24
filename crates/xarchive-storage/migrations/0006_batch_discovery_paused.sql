-- Allow account discovery to expose PAUSED independently from an ACTIVE batch.
-- SQLite cannot alter CHECK constraints in place, so rebuild both related tables
-- while preserving the candidate foreign key and all durable rows.

DROP INDEX archive_batches_state_idx;
DROP INDEX batch_candidates_batch_state_idx;
DROP INDEX batch_candidates_job_idx;
ALTER TABLE batch_candidates RENAME TO batch_candidates_v5;
ALTER TABLE archive_batches RENAME TO archive_batches_v5;

CREATE TABLE archive_batches (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    profile_url TEXT NOT NULL,
    user_id TEXT,
    browser TEXT,
    profile TEXT,
    filters_json TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('ACTIVE', 'PAUSED', 'CANCELLED', 'COMPLETED', 'FAILED')),
    discovery_state TEXT NOT NULL
        CHECK (discovery_state IN ('PENDING', 'RUNNING', 'PAUSED', 'COMPLETED', 'FAILED', 'CANCELLED')),
    last_error_code TEXT,
    last_error_message TEXT,
    retry_at_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE batch_candidates (
    id INTEGER PRIMARY KEY,
    batch_id TEXT NOT NULL REFERENCES archive_batches(id) ON DELETE CASCADE,
    tweet_id TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at TEXT,
    tweet_type TEXT NOT NULL DEFAULT 'post',
    is_repost INTEGER NOT NULL DEFAULT 0 CHECK (is_repost IN (0, 1)),
    has_media INTEGER NOT NULL DEFAULT 0 CHECK (has_media IN (0, 1)),
    media_count INTEGER NOT NULL DEFAULT 0 CHECK (media_count >= 0),
    user_id TEXT,
    username TEXT,
    state TEXT NOT NULL
        CHECK (state IN ('PENDING', 'SUBMITTED', 'DONE', 'FAILED', 'SKIPPED', 'CANCELLED')),
    job_id TEXT,
    error_code TEXT,
    error_message TEXT,
    skip_reason TEXT,
    inserted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(batch_id, tweet_id)
);

INSERT INTO archive_batches (
    id, username, profile_url, user_id, browser, profile, filters_json,
    state, discovery_state, last_error_code, last_error_message, retry_at_ms,
    created_at, updated_at
)
SELECT id, username, profile_url, user_id, browser, profile, filters_json,
       state, discovery_state, last_error_code, last_error_message, retry_at_ms,
       created_at, updated_at
FROM archive_batches_v5;

INSERT INTO batch_candidates (
    id, batch_id, tweet_id, url, created_at, tweet_type, is_repost, has_media,
    media_count, user_id, username, state, job_id, error_code, error_message,
    skip_reason, inserted_at, updated_at
)
SELECT id, batch_id, tweet_id, url, created_at, tweet_type, is_repost, has_media,
       media_count, user_id, username, state, job_id, error_code, error_message,
       skip_reason, inserted_at, updated_at
FROM batch_candidates_v5;

DROP TABLE batch_candidates_v5;
DROP TABLE archive_batches_v5;

CREATE INDEX archive_batches_state_idx ON archive_batches(state, created_at);
CREATE INDEX batch_candidates_batch_state_idx ON batch_candidates(batch_id, state, created_at);
CREATE INDEX batch_candidates_job_idx ON batch_candidates(job_id);
