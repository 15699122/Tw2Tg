-- Initial business schema. Rust owns all writes to this database.

PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    x_user_id TEXT NOT NULL UNIQUE,
    stable_directory_name TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE user_names (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username TEXT NOT NULL,
    display_name TEXT,
    source TEXT NOT NULL,
    observed_at TEXT NOT NULL
);

CREATE TABLE tweets (
    id INTEGER PRIMARY KEY,
    tweet_id TEXT NOT NULL UNIQUE,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    canonical_url TEXT NOT NULL,
    tweet_type TEXT NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    x_created_at TEXT,
    dom_metadata_json TEXT,
    extractor_metadata_json TEXT,
    merged_metadata_json TEXT,
    metadata_schema_version INTEGER NOT NULL DEFAULT 1,
    archive_directory TEXT,
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    tweet_id INTEGER NOT NULL REFERENCES tweets(id) ON DELETE CASCADE,
    job_type TEXT NOT NULL,
    state TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    last_error_code TEXT,
    last_error_message TEXT,
    next_retry_at TEXT,
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX jobs_one_active_archive_per_tweet
    ON jobs(tweet_id)
    WHERE job_type = 'archive'
      AND state IN (
          'QUEUED', 'VALIDATING', 'METADATA_READY',
          'TG_METADATA_SENDING', 'TG_METADATA_SENT',
          'DOWNLOADING', 'DOWNLOADED', 'TG_MEDIA_UPLOADING'
      );

CREATE TABLE media (
    id INTEGER PRIMARY KEY,
    tweet_id INTEGER NOT NULL REFERENCES tweets(id) ON DELETE CASCADE,
    media_index INTEGER NOT NULL CHECK (media_index > 0),
    x_media_id TEXT,
    media_type TEXT NOT NULL,
    relative_path TEXT,
    mime_type TEXT,
    size_bytes INTEGER CHECK (size_bytes >= 0),
    sha256 TEXT,
    duplicate_of_media_id INTEGER REFERENCES media(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(tweet_id, media_index)
);

CREATE TABLE transfers (
    id INTEGER PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    media_id INTEGER REFERENCES media(id) ON DELETE SET NULL,
    backend TEXT NOT NULL CHECK (backend IN ('gallery_dl', 'aria2')),
    backend_task_id TEXT,
    state TEXT NOT NULL,
    completed_bytes INTEGER NOT NULL DEFAULT 0 CHECK (completed_bytes >= 0),
    total_bytes INTEGER CHECK (total_bytes >= 0),
    started_at TEXT,
    finished_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    job_id TEXT REFERENCES jobs(id) ON DELETE SET NULL,
    tweet_id INTEGER REFERENCES tweets(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    previous_state TEXT,
    new_state TEXT,
    payload_json TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE telegram_messages (
    id INTEGER PRIMARY KEY,
    tweet_id INTEGER NOT NULL REFERENCES tweets(id) ON DELETE CASCADE,
    media_id INTEGER REFERENCES media(id) ON DELETE SET NULL,
    chat_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    message_kind TEXT NOT NULL,
    telegram_file_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(chat_id, message_id)
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE tweet_tags (
    tweet_id INTEGER NOT NULL REFERENCES tweets(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY(tweet_id, tag_id)
);

CREATE TABLE settings_meta (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX user_names_user_observed_idx ON user_names(user_id, observed_at DESC);
CREATE INDEX tweets_user_created_idx ON tweets(user_id, x_created_at DESC);
CREATE INDEX jobs_state_updated_idx ON jobs(state, updated_at DESC);
CREATE INDEX media_sha256_idx ON media(sha256);
CREATE INDEX events_job_created_idx ON events(job_id, created_at DESC);