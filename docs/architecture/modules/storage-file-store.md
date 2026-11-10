# storage-file-store

- **Path / entry:** `crates/xarchive-storage/src/file_store.rs` plus test modules
  `file_store/tests/{path_safety,try_staging}.rs`.
- **Responsibility:** own the on-disk attempt staging layout: create attempt
  directories, place files without clobbering, verify path safety against symlinks
  and traversal, and recover or roll forward pending edges atomically.
- **Owner:** Cross-platform Owner.
- **Depends on:** the filesystem only; no database access.
- **Depended on by:** archive publication and recovery paths that need a staged
  attempt tree.
- **Public contract:** unchanged. No function signature, error variant or path
  rule changed.
- **Tests:** 13 tests moved into path safety (3) and try staging (10); `temp_root`
  stays in the parent `mod tests`. Every symlink and traversal assertion is
  byte-identical to before.
- **Gates to re-run:** storage lib regression (178/0/0).
- **Exceptions:** none.
- **Open questions:** behaviour on Windows reparse points is Windows-owned and was
  not exercised here.
