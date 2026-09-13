use rusqlite::params;

use crate::*;

impl Database {
    pub fn upsert_tag(&self, name: &str, created_at: &str) -> Result<i64, StorageError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err(StorageError::InvalidMetadata("tag name is invalid".into()));
        }
        self.connection.execute(
            "INSERT INTO tags (name, created_at) VALUES (?1, ?2) ON CONFLICT(name) DO NOTHING",
            params![name, created_at],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?)
    }

    pub fn attach_tag(&self, tweet_row_id: i64, tag_id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO tweet_tags (tweet_id, tag_id) VALUES (?1, ?2) ON CONFLICT(tweet_id, tag_id) DO NOTHING",
            params![tweet_row_id, tag_id],
        )?;
        Ok(())
    }

    pub fn list_tweet_tags(&self, tweet_row_id: i64) -> Result<Vec<String>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT tags.name FROM tags JOIN tweet_tags ON tweet_tags.tag_id = tags.id WHERE tweet_tags.tweet_id = ?1 ORDER BY tags.name ASC",
        )?;
        let rows = statement.query_map(params![tweet_row_id], |row| row.get(0))?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(StorageError::from)
    }
}
