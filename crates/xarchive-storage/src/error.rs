use std::io;

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Io(io::Error),
    Json(serde_json::Error),
    InvalidPath,
    InvalidState(String),
    InvalidTransition(xarchive_core::JobStateError),
    InvalidMetadata(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "SQLite error: {error}"),
            Self::Io(error) => write!(formatter, "file error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::InvalidPath => formatter.write_str("path is outside the archive root"),
            Self::InvalidState(value) => write!(formatter, "invalid persisted job state: {value}"),
            Self::InvalidTransition(error) => write!(formatter, "{error}"),
            Self::InvalidMetadata(message) => {
                write!(formatter, "invalid archive metadata: {message}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<io::Error> for StorageError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
