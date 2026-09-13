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
}
