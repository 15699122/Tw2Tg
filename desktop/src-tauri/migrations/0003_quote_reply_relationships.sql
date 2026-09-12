-- Quote/Reply relationship modeling for archived tweets (M5).
-- These columns are nullable reference links derived from the merged
-- metadata written by update_tweet_metadata. They exist so the archive
-- library and future GUI can query direct reply/quote relationships without
-- parsing the whole merged_metadata_json payload.

ALTER TABLE tweets ADD COLUMN reply_to_tweet_id TEXT;
ALTER TABLE tweets ADD COLUMN quoted_tweet_id TEXT;