# desktop-commands-archive

- **Path / entry:** `desktop/src-tauri/src/commands/archive.rs`; reached through
  `crate::commands::archive`, declared by `commands.rs` as
  `pub(crate) mod archive;`.
- **Responsibility:** Archive root and portable download setup — archive directory
  selection and validation, first-run download setup completion, the gallery-dl
  executable path, and the archive/clipboard folder actions.
- **Owner:** Cross-platform.
- **Depends on:** `crate::RuntimeState`, `crate::portable::PortablePaths` and
  `system_download_archive_directory`, `crate::platform::open_path_command`,
  `xarchive_storage::Database`, and the parent's `AppStatus`, `app_status` and
  `GalleryDlInstallation`.
- **Depended on by:** `desktop/src-tauri/src/lib.rs::generate_handler!`, naming
  `commands::archive::<name>`.
- **Public contract:** the 8 registered command names, `PortableSetup`'s serialized
  fields, `GalleryDlInstallation`'s fields, the argument names and every error
  string (including the Chinese-language messages). Unchanged by the move.
- **Safety boundaries preserved:** `validate_archive_directory` rejects a path that
  is not an absolute child of the portable root, a symlink escape, a relative path
  and a non-directory; `complete_download_setup` and `set_archive_directory` only
  write after that validation.
- **State and ownership:** writes `state.config` and persists it through the
  portable config; `get_*` commands only project state.
- **Failure behaviour:** fail closed with a redacted `String` error; an invalid
  directory leaves the existing configuration untouched.
- **Tests:** `archive_directory_tests` moved with the module and still asserts the
  absolute/contained/directory requirements, including the symlink and
  non-directory refusals.
- **Gates to re-run:** `cargo test -p xarchive-desktop --lib`; `cargo check -p
  xarchive-desktop --lib`.
- **Exceptions:** none; 282 lines.
- **Open questions:** none.
