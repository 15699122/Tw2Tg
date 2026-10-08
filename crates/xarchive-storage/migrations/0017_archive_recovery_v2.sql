-- Isolate rename-capable recovery facts from the strict InternalV1 journal.
CREATE TABLE archive_recovery_v2 (
    job_id TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
    attempt_count INTEGER NOT NULL CHECK (attempt_count > 0),
    tweet_row_id INTEGER NOT NULL REFERENCES tweets(id) ON DELETE CASCADE,
    archive_directory TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL CHECK (length(manifest_sha256) = 64),
    phase TEXT NOT NULL CHECK (phase IN ('PREPARED', 'COMMITTED')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX archive_recovery_v2_phase_idx
    ON archive_recovery_v2(phase, updated_at);