# desktop-production

- **Path / entry:** `desktop/src-tauri/src/production.rs` plus child modules
  `production/{tool_compatibility,discovery,sidecar_operations,transfer,result_mapping}.rs`.
- **Responsibility:** orchestrate the v2 extraction → transfer boundary that turns a
  Sidecar extraction result into a durable archive commit input.
- **Owner:** Cross-platform Owner.
- **Depends on:** `xarchive_download` (aria2 supervisor, transfer driver, planning),
  `xarchive_protocol` (v2 commands/events), `xarchive_sidecar_supervisor`.
- **Depended on by:** `archive.rs` execution adapter; `batch.rs` discovery.
- **Public contract:** unchanged. `execute_v2_archive` remains the single
  composition entry in the parent and routes on `use_aria2`.
- **Compatibility entry:** `mod production;` is crate-private; the parent bridges
  the moved surface with `pub(crate) use` so original paths keep resolving.
- **State and ownership:** the aria2 supervisor is retained across a refreshable
  failure (`TransferAttemptFailure::retained_supervisor`) and re-drained by the same
  owner; `ensure_same_media_collection` keeps the media set identity.
- **Failure behaviour:** a containment failure fails the attempt and **never** falls
  back to a plain spawn; exactly one URL refresh is attempted; a protocol violation
  on the wrong event stream is an error, not a silent skip.
- **Constraint:** `production_transfer_fixture.rs` is `include!`-expanded into the
  `production` module and was neither moved nor edited, so the fixture keeps calling
  the same unqualified names.
- **Tests:** `production::*` (18 tests, 0 ignored) plus the included fixture.
- **Gates to re-run:** production module regression; Desktop lib regression. Real
  aria2 download acceptance remains deferred and is **not** claimed by this split.
- **Exceptions:** none.
- **Related:** [approved plan](../../development/internal-v1-completion-and-refactor-plan.md).
