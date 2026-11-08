# desktop-commands-app

- **Path / entry:** `desktop/src-tauri/src/commands/app.rs`; reached through
  `crate::commands::app`, declared by `commands.rs` as `pub(crate) mod app;`.
- **Responsibility:** The startup/runtime projection — application status, component
  bootstrap status and the runtime health probe.
- **Owner:** Cross-platform.
- **Depends on:** `crate::RuntimeState`, `crate::components::ComponentManager`,
  `crate::portable::resolve_config_path`, `crate::commands::refresh_sidecar_state`,
  and `crate::batch::BatchFilters` for the re-homed
  `CreateAccountBatchRequest`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`
  (`commands::app::<name>`), plus `commands/archive.rs` and
  `commands/settings.rs`, which import `AppStatus` and `app_status` from here.
- **Public contract:** the 3 registered command names, `AppStatus`'s serialized
  fields (`app_name`, `app_version`, `sidecar`, `database`, `platform`,
  `archive_root`, `gallery_dl_path`, `database_error`, `sidecar_error`, `executor`,
  `download_setup_required`, `logs_root`, `logging_level`, `max_log_files`,
  `use_aria2`), `PortableSetup`, `GalleryDlInstallation`,
  `CreateAccountBatchRequest` and every status string (`"ready"`, `"error"`,
  `"stopped"`, `"failed"`, `"not_configured"`). Unchanged by the move.
- **State and ownership:** reads `RuntimeState` only. `get_app_status` polls the
  Sidecar supervisor for exit before projecting, so a dead worker is reported as
  `failed` rather than `ready`.
- **Failure behaviour:** `get_app_status` and `get_runtime_health` are infallible
  projections and never invent a state; a poisoned lock fails closed at the
  platform boundary rather than resuming with partial data.
- **Code moved, not rewritten:** `AppStatus`, `PortableSetup`,
  `GalleryDlInstallation`, `CreateAccountBatchRequest`, `sidecar_status`,
  `app_status`, `get_component_bootstrap_status`, `get_app_status`,
  `get_runtime_health`.
- **Why `CreateAccountBatchRequest` lives here:** it was the last type left in
  `commands.rs`; it belongs to the batch request surface but depends on
  `BatchFilters` and executor metadata settings, so it is grouped with the other
  shared projection types. `commands/batches.rs` imports it from here.
- **Tests:** covered by the Desktop lib suite.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`.
- **Exceptions:** none; 140 lines.
- **Open questions:** none. With this module, `commands.rs` holds no
  `#[tauri::command]` and remains only as the module root plus the two shared
  helpers (`open_batch_database`, `parse_sidecar_args`, `refresh_sidecar_state`,
  `bounded_diagnostic`).
