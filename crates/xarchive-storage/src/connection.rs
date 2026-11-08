//! Connection opening and migration assembly for the archive database.
//!
//! This module owns exactly two things: how a SQLite connection is opened, and
//! the ordered migration list applied to it. It contains no schema logic of its
//! own — the SQL lives in `migrations/` and the order below is the contract.
//! Changing the order, or inserting a migration, is a schema decision and needs
//! its own review; this module must not acquire query or domain behaviour.

use rusqlite::Connection;
use std::path::Path;

use crate::StorageError;

pub(crate) const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_initial.sql"),
    include_str!("../migrations/0002_telegram_send_state.sql"),
    include_str!("../migrations/0003_quote_reply_relationships.sql"),
    include_str!("../migrations/0004_archive_job_requests.sql"),
    include_str!("../migrations/0005_account_batches.sql"),
    include_str!("../migrations/0006_batch_discovery_paused.sql"),
    include_str!("../migrations/0007_telegram_outbox.sql"),
    include_str!("../migrations/0008_telegram_outbox_snapshot.sql"),
    include_str!("../migrations/0009_telegram_archive_intents.sql"),
    include_str!("../migrations/0010_telegram_credential_generations.sql"),
    include_str!("../migrations/0011_telegram_resume_authorizations.sql"),
    include_str!("../migrations/0012_telegram_rotation_decisions.sql"),
    include_str!("../migrations/0013_telegram_claim_generation.sql"),
    include_str!("../migrations/0014_download_task_metrics.sql"),
    include_str!("../migrations/0015_batch_output_snapshot.sql"),
    include_str!("../migrations/0016_archive_recovery_journal.sql"),
    include_str!("../migrations/0017_archive_recovery_v2.sql"),
    include_str!("../migrations/0018_archive_recovery_v2_candidate.sql"),
    include_str!("../migrations/0019_batch_downloader_arguments.sql"),
    include_str!("../migrations/0020_record_exchange_ledger.sql"),
];

/// Open a database file and bring it up to the current schema version.
pub(crate) fn open(path: impl AsRef<Path>) -> Result<Connection, StorageError> {
    let connection = Connection::open(path)?;
    prepare(connection)
}

/// Open an in-memory database for tests and ephemeral callers.
pub(crate) fn open_in_memory() -> Result<Connection, StorageError> {
    prepare(Connection::open_in_memory()?)
}

/// Apply the connection pragmas and every migration this build knows about.
///
/// Each migration runs in its own transaction and records its version, so a
/// reopen skips the versions already applied and a failed migration leaves the
/// database at the last applied version rather than a partial one.
pub(crate) fn prepare(connection: Connection) -> Result<Connection, StorageError> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;",
    )?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
    )?;
    let current_version: i64 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    for (index, migration) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        if current_version >= version {
            continue;
        }
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(migration)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [version],
        )?;
        transaction.commit()?;
    }
    Ok(connection)
}
