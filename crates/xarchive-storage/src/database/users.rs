use rusqlite::{OptionalExtension, params};

use crate::*;

impl Database {
    pub fn upsert_user(
        &self,
        user_id: &str,
        username: Option<&str>,
        display_name: Option<&str>,
        now: &str,
    ) -> Result<i64, StorageError> {
        if user_id.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "user_id must not be empty".into(),
            ));
        }
        let directory = xarchive_core::stable_user_directory_name(username, display_name, user_id);
        self.connection.execute(
            "INSERT INTO users (x_user_id, stable_directory_name, first_seen_at, last_seen_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?3, ?3, ?3) ON CONFLICT(x_user_id) DO UPDATE SET stable_directory_name = excluded.stable_directory_name, last_seen_at = excluded.last_seen_at, updated_at = excluded.updated_at",
            params![user_id, directory, now],
        )?;
        let row_id = self.connection.query_row(
            "SELECT id FROM users WHERE x_user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        if let Some(username) = username.filter(|value| !value.trim().is_empty()) {
            let unchanged = self.latest_user_name(row_id)?.is_some_and(|latest| {
                latest.username == username && latest.display_name.as_deref() == display_name
            });
            if !unchanged {
                self.record_user_name(row_id, username, display_name, "unknown", now)?;
            }
        }
        Ok(row_id)
    }

    /// Most recent username observation for a user, if any.
    fn latest_user_name(&self, user_row_id: i64) -> Result<Option<UserNameSummary>, StorageError> {
        Ok(self.list_user_names(user_row_id)?.into_iter().next())
    }

    pub fn record_user_name(
        &self,
        user_row_id: i64,
        username: &str,
        display_name: Option<&str>,
        source: &str,
        observed_at: &str,
    ) -> Result<(), StorageError> {
        if username.trim().is_empty() || source.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "username and source must not be empty".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO user_names (user_id, username, display_name, source, observed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user_row_id, username, display_name, source, observed_at],
        )?;

        Ok(())
    }

    pub fn list_user_names(&self, user_row_id: i64) -> Result<Vec<UserNameSummary>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT username, display_name, source, observed_at FROM user_names WHERE user_id = ?1 ORDER BY observed_at DESC, id DESC",
        )?;
        let rows = statement.query_map(params![user_row_id], |row| {
            Ok(UserNameSummary {
                username: row.get(0)?,
                display_name: row.get(1)?,
                source: row.get(2)?,
                observed_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    /// Latest user profile snapshot derived from `users` and `user_names`.
    ///
    /// The newest name observation (by `observed_at`) provides the current
    /// username/display name; the full history is embedded in profile files.
    pub fn user_profile(
        &self,
        x_user_id: &str,
    ) -> Result<Option<UserProfileSnapshot>, StorageError> {
        let row = self
            .connection
            .query_row(
                "SELECT id, x_user_id, stable_directory_name, first_seen_at, last_seen_at FROM users WHERE x_user_id = ?1",
                params![x_user_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((row_id, user_id, stable_directory_name, first_seen_at, last_seen_at)) = row
        else {
            return Ok(None);
        };
        let names = self.list_user_names(row_id)?;
        let (username, display_name) = match names.first() {
            Some(latest) => (Some(latest.username.clone()), latest.display_name.clone()),
            None => (None, None),
        };
        Ok(Some(UserProfileSnapshot {
            user_id,
            stable_directory_name,
            username,
            display_name,
            first_seen_at,
            last_seen_at,
            names,
        }))
    }

    /// Merge a browser-derived placeholder user into the stable identity.
    ///
    /// The browser path can only anchor a user as `browser-<tweet_id>` because
    /// the DOM payload carries no stable X user id. Once extraction returns the
    /// real id, this upgrade re-points every tweet linked to the placeholder,
    /// folds the name history into the stable row (without duplicates), folds
    /// the observation window, and removes the placeholder row. Calling this
    /// with an unknown placeholder id is a no-op.
    pub fn merge_placeholder_user(
        &self,
        placeholder_user_id: &str,
        target_row_id: i64,
        now: &str,
    ) -> Result<(), StorageError> {
        let placeholder_row: Option<i64> = self
            .connection
            .query_row(
                "SELECT id FROM users WHERE x_user_id = ?1",
                params![placeholder_user_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(placeholder_row) = placeholder_row else {
            return Ok(());
        };
        if placeholder_row == target_row_id {
            return Ok(());
        }
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "UPDATE tweets SET user_id = ?2, updated_at = ?3 WHERE user_id = ?1",
            params![placeholder_row, target_row_id, now],
        )?;
        transaction.execute(
            "INSERT INTO user_names (user_id, username, display_name, source, observed_at) \
             SELECT ?1, n.username, n.display_name, n.source, n.observed_at \
             FROM user_names n \
             WHERE n.user_id = ?2 \
               AND NOT EXISTS (\
                   SELECT 1 FROM user_names existing \
                   WHERE existing.user_id = ?1 \
                     AND existing.username = n.username \
                     AND ((existing.display_name IS NULL AND n.display_name IS NULL) \
                          OR existing.display_name = n.display_name))",
            params![target_row_id, placeholder_row],
        )?;
        transaction.execute(
            "UPDATE users SET \
                 first_seen_at = MIN(first_seen_at, (SELECT first_seen_at FROM users WHERE id = ?2)), \
                 last_seen_at = MAX(last_seen_at, (SELECT last_seen_at FROM users WHERE id = ?2)), \
                 updated_at = ?3 \
             WHERE id = ?1",
            params![target_row_id, placeholder_row, now],
        )?;
        transaction.execute("DELETE FROM users WHERE id = ?1", params![placeholder_row])?;
        transaction.commit()?;
        Ok(())
    }
}
