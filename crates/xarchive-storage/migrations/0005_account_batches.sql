-- Account batch download: durable batch headers and per-Tweet candidates.
-- Rust owns all writes; discovery persists candidates here before dispatch.
-- `batch_candidates.created_at` is the Tweet's own creation time (ordering key);
-- `inserted_at`/`updated_at` are the row's local bookkeeping timestamps.

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
        CHECK (discovery_state IN ('PENDING', 'RUNNING', 'COMPLETED', 'FAILED', 'CANCELLED')),
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

CREATE INDEX archive_batches_state_idx ON archive_batches(state, created_at);
CREATE INDEX batch_candidates_batch_state_idx ON batch_candidates(batch_id, state, created_at);
CREATE INDEX batch_candidates_job_idx ON batch_candidates(job_id);
