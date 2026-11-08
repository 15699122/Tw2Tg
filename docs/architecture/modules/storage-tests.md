## storage-tests

- **Path / entry:** `crates/xarchive-storage/src/tests/` — `mod.rs` holds the
  shared imports, fixtures and submodule declarations; each submodule holds one
  subject's regression suites.
- **Responsibility:** Keep the Storage regression matrix next to the subject it
  covers instead of in one inline module.
- **Owner:** Cross-platform
- **Depends on:** `super::*` (the crate root) and the shared fixtures in
  `mod.rs`.
- **Depended on by:** nothing at runtime; compiled only under `#[cfg(test)]`.
- **Public contract:** none — test-only. Test names keep their leaf identity, so
  a targeted filter still selects the same suite; the module path gains the group
  name (`tests::<group>::<name>`).
- **Compatibility entry:** `#[cfg(test)] mod tests;` in `lib.rs` replaces the
  former inline module.
- **State and ownership:** each suite creates its own temporary root, database
  and file store, exactly as before the move.
- **Failure behaviour:** unchanged. A suite that re-executes itself for a crash
  window passes the fully qualified name, so the child runs the same test.
- **Tests:** groups `archive_recovery` (22), `archive_recovery_manifest` (9),
  `archive_service` (48), `database` (14), `database_telegram` (22),
  `telegram_credentials` (11), `telegram_outbox` (18). The two
  `archive_recovery*` groups split the former single 31-suite module: manifest
  contracts, rename plans and verification on one side, attempt claims, journal
  replay and crash windows on the other. Their shared fixtures
  (`test_candidate_metadata`, `manifest`) live in `mod.rs`.
- **Gates to re-run:** `cargo test -p xarchive-storage --lib` — the accepted
  evidence is an unchanged total of 178 passing suites.
- **Exceptions:** none; every file stays under the 3000-line library limit.
- **Open questions:** none.
