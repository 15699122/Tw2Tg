use rusqlite::{OptionalExtension, params};

use crate::*;

use xarchive_core::ArchiveMetadata;

impl Database {
    pub fn insert_tweet(
        &self,
        tweet_id: &str,
        canonical_url: &str,
        tweet_type: &str,
        text: &str,
        now: &str,
    ) -> Result<i64, StorageError> {
        self.insert_tweet_for_user(tweet_id, canonical_url, tweet_type, text, None, now)
    }

    pub fn insert_tweet_for_user(
        &self,
        tweet_id: &str,
        canonical_url: &str,
        tweet_type: &str,
        text: &str,
        user_row_id: Option<i64>,
        now: &str,
    ) -> Result<i64, StorageError> {
        self.connection.execute(
            "INSERT INTO tweets (tweet_id, canonical_url, tweet_type, text, user_id, created_at, updated_at)\n             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)\n             ON CONFLICT(tweet_id) DO UPDATE SET canonical_url = excluded.canonical_url, user_id = COALESCE(excluded.user_id, tweets.user_id), updated_at = excluded.updated_at",
            params![tweet_id, canonical_url, tweet_type, text, user_row_id, now],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM tweets WHERE tweet_id = ?1",
            params![tweet_id],
            |row| row.get(0),
        )?)
    }

    pub fn set_tweet_user(&self, tweet_row_id: i64, user_row_id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE tweets SET user_id = ?1, updated_at = updated_at WHERE id = ?2",
            params![user_row_id, tweet_row_id],
        )?;
        Ok(())
    }

    pub fn update_tweet_metadata(
        &self,
        tweet_row_id: i64,
        metadata: &ArchiveMetadata,
        archive_directory: &str,
    ) -> Result<(), StorageError> {
        let quoted_tweet_id = metadata
            .quoted_tweet
            .as_ref()
            .and_then(|quoted| xarchive_core::TweetId::new(quoted.tweet_id.clone()).ok())
            .map(|id| id.as_str().to_owned());
        let reply_to_tweet_id = metadata
            .reply_to
            .as_ref()
            .and_then(|reply_to| xarchive_core::TweetId::new(reply_to.clone()).ok())
            .map(|id| id.as_str().to_owned());
        self.connection.execute(
            "UPDATE tweets SET text = ?1, merged_metadata_json = ?2, archive_directory = ?3, archived_at = ?4, reply_to_tweet_id = ?5, quoted_tweet_id = ?6, updated_at = ?4 WHERE id = ?7",
            params![
                metadata.text,
                serde_json::to_string(metadata)?,
                archive_directory,
                metadata.archived_at,
                reply_to_tweet_id,
                quoted_tweet_id,
                tweet_row_id
            ],
        )?;
        Ok(())
    }

    pub fn tweet_relationships(
        &self,
        tweet_id: &str,
    ) -> Result<Option<TweetRelationships>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT reply_to_tweet_id, quoted_tweet_id FROM tweets WHERE tweet_id = ?1",
                params![tweet_id],
                |row| {
                    Ok(TweetRelationships {
                        reply_to_tweet_id: row.get(0)?,
                        quoted_tweet_id: row.get(1)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn insert_media(
        &self,
        tweet_row_id: i64,
        media: &xarchive_core::ArchiveMedia,
        now: &str,
    ) -> Result<i64, StorageError> {
        self.connection.execute(
            "INSERT INTO media (tweet_id, media_index, x_media_id, media_type, relative_path, mime_type, size_bytes, sha256, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9) ON CONFLICT(tweet_id, media_index) DO UPDATE SET relative_path = excluded.relative_path, mime_type = excluded.mime_type, size_bytes = excluded.size_bytes, sha256 = excluded.sha256, updated_at = excluded.updated_at",
            params![
                tweet_row_id,
                media.index,
                media.media_id,
                media.media_type,
                media.file,
                media.mime_type,
                media.size_bytes,
                media.sha256,
                now
            ],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM media WHERE tweet_id = ?1 AND media_index = ?2",
            params![tweet_row_id, media.index],
            |row| row.get(0),
        )?)
    }
}
