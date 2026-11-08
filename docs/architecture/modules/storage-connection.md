## storage-connection

- **Path / entry:** `crates/xarchive-storage/src/connection.rs`; reached through
  `Database::open` / `Database::open_in_memory` in `crates/xarchive-storage/src/lib.rs`.
- **Responsibility:** Open a SQLite connection and apply the ordered migration
  list.
- **Owner:** Cross-platform
- **Depends on:** `rusqlite`, `crate::StorageError`, and the SQL files in
  `crates/xarchive-storage/migrations/` (via `include_str!`).
- **Depended on by:** `Database::open` and `Database::open_in_memory`.
- **Public contract:** `pub(crate) fn open`, `pub(crate) fn open_in_memory`,
  `pub(crate) fn prepare`, `pub(crate) const MIGRATIONS`. `Database::open` keeps
  its public signature, its `StorageError` mapping and its behaviour of applying
  only the versions not yet recorded in `schema_migrations`.
- **Compatibility entry:** `Database::open` / `Database::open_in_memory` are
  unchanged for every caller.
- **State and ownership:** holds no application state. It returns an owned
  `rusqlite::Connection`; the `Database` wrapper in `lib.rs` remains the only
  owner.
- **Failure behaviour:** fail closed. A migration failure rolls back its own
  transaction and leaves the database at the last applied version; a pragma or
  connection failure returns `StorageError` instead of a partially configured
  handle.
- **Tests:** `tests::database::reopens_persistent_database_without_reapplying_schema`,
  `tests::database::initializes_schema_and_creates_idempotent_job`,
  `tests::database::upgrades_existing_database_with_relationship_columns`,
  `tests::database_telegram::telegram_outbox_migration_is_additive_and_isolates_legacy_rows`
  (all exercise the real migration list).
- **Gates to re-run:** `cargo test -p xarchive-storage --lib`; any gate that
  reopens a database (V1 replay and crash-window suites).
- **Exceptions:** none.
- **Open questions:** none.
