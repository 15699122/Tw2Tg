# protocol-sidecar-v2

- **Path / entry:** `crates/xarchive-protocol/src/sidecar_v2.rs` plus test modules
  `sidecar_v2/tests/{extract,discovery,download,rejection,fixtures}.rs`.
- **Responsibility:** define and validate the v2 Sidecar command/event contract
  between the Desktop sidecar supervisor and the extraction process.
- **Owner:** Cross-platform Owner.
- **Depends on:** `serde`, the shared JSONL fixtures under
  `shared/protocol-schema/fixtures/`.
- **Depended on by:** Desktop sidecar supervision and the sidecar binary.
- **Public contract:** unchanged. Every command/event type, field, validation rule
  and serde attribute is untouched.
- **Tests:** 14 tests moved into five groups (extract 2, discovery 1, download 6,
  rejection 4, fixtures 1). `host_absolute_staging_dir`, `sample_result` and
  `sample_download` remain in the parent `mod tests` as shared helpers. The only
  body edit is the `include_str!` depth for the shared fixtures, which had to grow
  from `../../../` to `../../../../../` after the file moved two directories
  deeper.
- **Gates to re-run:** protocol lib regression (31/0/0).
- **Exceptions:** none.
- **Open questions:** the fixtures test is the cross-language contract guard; it
  must keep reading the real shared JSONL files rather than copies.
