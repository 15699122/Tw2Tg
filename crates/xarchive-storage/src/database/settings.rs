use rusqlite::{OptionalExtension, params};

use crate::{Database, SettingEntry, StorageError};

const MAX_SETTING_BYTES: usize = 16 * 1024;

fn is_allowed_setting_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 256
        && (key.starts_with("ui.") || key.starts_with("download."))
        && !key.chars().any(char::is_control)
}

impl Database {
    /// Read one setting value by key, or `None` if the key does not exist.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let key = key.trim();
        if !is_allowed_setting_key(key) {
            return Err(StorageError::InvalidMetadata(
                "setting key must not be empty".into(),
            ));
        }
        Ok(self
            .connection
            .query_row(
                "SELECT value_json FROM settings_meta WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?)
    }

    /// Insert or update a setting value.
    pub fn set_setting(&self, key: &str, value_json: &str, now: &str) -> Result<(), StorageError> {
        let key = key.trim();
        if key.is_empty()
            || key.len() > 256
            || !(key.starts_with("ui.") || key.starts_with("download."))
            || key.chars().any(char::is_control)
        {
            return Err(StorageError::InvalidMetadata(
                "setting key is invalid".into(),
            ));
        }
        if value_json.trim().is_empty()
            || value_json.len() > MAX_SETTING_BYTES
            || serde_json::from_str::<serde_json::Value>(value_json).is_err()
        {
            return Err(StorageError::InvalidMetadata(
                "value_json must be valid JSON and within the size limit".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO settings_meta (key, value_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
            params![key, value_json, now],
        )?;
        Ok(())
    }

    /// List all settings ordered by key.
    pub fn list_settings(&self) -> Result<Vec<SettingEntry>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT key, value_json, updated_at FROM settings_meta ORDER BY key ASC")?;
        let rows = statement.query_map([], |row| {
            Ok(SettingEntry {
                key: row.get(0)?,
                value_json: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    /// Delete a setting by key. Returns `true` if a row was removed.
    pub fn delete_setting(&self, key: &str) -> Result<bool, StorageError> {
        let key = key.trim();
        if !is_allowed_setting_key(key) {
            return Err(StorageError::InvalidMetadata(
                "setting key must not be empty".into(),
            ));
        }
        let changed = self
            .connection
            .execute("DELETE FROM settings_meta WHERE key = ?1", params![key])?;
        Ok(changed > 0)
    }
}
