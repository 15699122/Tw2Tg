# storage-archive-service

- **Path / entry:** `crates/xarchive-storage/src/archive_service.rs` plus child
  modules `archive_service/{legacy_completion,internal_v1_publication,internal_v1_recovery,v2_rename,telegram_intent}.rs`.
- **Responsibility:** commit one completed archive outcome — legacy completion,
  InternalV1 publication/recovery, V2 rename publication/recovery, or Telegram
  intent recovery — from an already-authorized attempt.
- **Owner:** Cross-platform Owner.
- **Depends on:** `Database`, `FileStore`, `build_archive_metadata`,
  `plan_archive_media_renames`, recovery manifests and the workflow permit type.
- **Depended on by:** the Desktop execution adapter, Storage recovery entry points
  and the Storage test modules.
- **Public contract:** unchanged. `ArchiveService` is still one type with one public
  surface; `lib.rs` re-exports `ArchivePreview`, `ArchiveRecoveryFailpoint`,
  `ArchiveService` and `SidecarArchiveRequest` from the same path. The inherent
  `impl` is distributed across child modules, which Rust permits without changing
  any signature or visibility.
- **Compatibility entry:** `mod archive_service;` and its `pub use` list in
  `lib.rs` are untouched.
- **State and ownership:** Storage consumes authorization; it never performs
  admission and never mints a permit. The PREPARED replay path takes a
  `PublicationPermit` and stays distinct from COMMITTED verification.
- **Failure behaviour:** unchanged. Transactions stay whole; CAS, attempt fencing
  and phase monotonicity live in the same method bodies as before.
- **Visibility:** seven cross-file helpers are `pub(super)` (reaching only
  `archive_service` itself), matching their pre-split reach as private `impl`
  methods. Nothing became crate-visible or public.
- **Tests:** `crates/xarchive-storage/src/tests/archive_service/` (48 tests),
  unchanged.
- **Gates to re-run:** Storage lib regression; Desktop lib regression.
- **Exceptions:** none.
- **Open questions:** the architecture baseline still records the pre-split line
  count; resealing requires separate authorization.
- **Related:** [storage tests](storage-tests.md),
  [storage archive recovery manifest](storage-archive-recovery-manifest.md),
  [storage telegram archive intents](storage-telegram-archive-intents.md).
