-- Minimal rich archive facts needed to finish a PREPARED v2 archive after restart.
-- The candidate is attempt-bound, secret-free, and intentionally separate from
-- user-controlled JSON/TXT exports and the immutable rename manifest.
CREATE TABLE archive_recovery_v2_candidate (
    job_id TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
    attempt_count INTEGER NOT NULL CHECK (attempt_count > 0),
    tweet_row_id INTEGER NOT NULL REFERENCES tweets(id) ON DELETE CASCADE,
    metadata_json TEXT NOT NULL,
    metadata_sha256 TEXT NOT NULL CHECK (length(metadata_sha256) = 64),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);