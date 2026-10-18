CREATE TABLE record_exchange_ledger (
    schema_version INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('tw2tg.download-records', 'tw2tg.archive-records')),
    account_handle TEXT NOT NULL,
    tweet_id TEXT NOT NULL REFERENCES tweets(tweet_id) ON DELETE CASCADE,
    canonical_record_json TEXT NOT NULL,
    archive_directory TEXT NOT NULL,
    accepted_at TEXT NOT NULL,
    PRIMARY KEY (schema_version, kind, account_handle, tweet_id)
);