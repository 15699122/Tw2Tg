//! Archive and portable-setup commands: archive root, download setup, archive
//! directory selection, gallery-dl path and the archive/clipboard folder actions.
//!
//! Command names, argument shapes, serialized fields and error strings are the
//! registered IPC contract and must not change here.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, State};
use xarchive_storage::Database;

use crate::RuntimeState;
use crate::commands::app::{AppStatus, GalleryDlInstallation, PortableSetup, app_status};
use crate::portable::system_download_archive_directory;

/// Validate a user-picked archive directory without touching the filesystem.
///
/// The archive directory is the target every job commits into, so the settings
/// page must not be able to point it at the portable root itself, at a file, or
/// at a relative path that would silently resolve somewhere else.
fn validate_archive_directory(portable_root: &Path, selected: &Path) -> Result<(), String> {
    if !selected.is_absolute() {
        return Err("归档目录必须是绝对路径。".to_owned());
    }
    if selected == portable_root {
        return Err("归档目录不能是程序所在目录。".to_owned());
    }
    if selected.is_file() {
        return Err("归档目录已存在同名文件。".to_owned());
    }
    Ok(())
}

fn gallery_dl_version(path: &Path) -> Result<String, String> {
    if !path.is_file() {
        return Err("gallery-dl 文件不存在".to_owned());
    }
    let mut command = std::process::Command::new(path);
    crate::platform::hide_console_window(&mut command);
    let output = command
        .arg("--version")
        .output()
        .map_err(|error| format!("无法启动 gallery-dl：{error}"))?;
    if !output.status.success() {
        return Err(format!("gallery-dl 返回状态 {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace()
        .find(|value| {
            value
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        })
        .map(str::to_owned)
        .ok_or_else(|| "无法从 gallery-dl 输出中识别版本".to_owned())
}

#[tauri::command]
pub(crate) fn get_archive_root(state: State<'_, Mutex<RuntimeState>>) -> String {
    state
        .lock()
        .expect("runtime state lock poisoned")
        .download_root
        .display()
        .to_string()
}

#[tauri::command]
pub(crate) fn get_portable_setup(
    app: AppHandle,
    state: State<'_, Mutex<RuntimeState>>,
) -> PortableSetup {
    let state = state.lock().expect("runtime state lock poisoned");
    PortableSetup {
        portable_root: state.portable_root.display().to_string(),
        download_root: state.download_root.display().to_string(),
        system_download_root: system_download_archive_directory(&app)
            .map(|path| path.display().to_string()),
        required: state.download_setup_required,
    }
}

#[tauri::command]
pub(crate) fn complete_download_setup(
    app: AppHandle,
    state: State<'_, Mutex<RuntimeState>>,
    choice: String,
) -> Result<PortableSetup, String> {
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let selected = match choice.as_str() {
        "portable" => state.portable_root.join("download"),
        "system_downloads" => system_download_archive_directory(&app)
            .ok_or_else(|| "system Downloads directory is unavailable".to_owned())?,
        _ => return Err("download setup choice must be portable or system_downloads".to_owned()),
    };
    std::fs::create_dir_all(&selected)
        .map_err(|error| format!("failed to create download directory: {error}"))?;
    let staging_root = state.cache_root.join("staging");
    std::fs::create_dir_all(&staging_root)
        .map_err(|error| format!("failed to create cache directory: {error}"))?;
    state.config.download.mode = choice;
    state.config.download.directory = selected
        .strip_prefix(&state.portable_root)
        .map(|path| format!("./{}", path.to_string_lossy().replace('\\', "/")))
        .unwrap_or_else(|_| selected.to_string_lossy().to_string());
    state.config.download.initialized = true;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.config.save(&paths)?;
    let database_path = state.config.database_path(&paths);
    std::fs::create_dir_all(
        database_path
            .parent()
            .ok_or_else(|| "invalid database path".to_owned())?,
    )
    .map_err(|error| error.to_string())?;
    state.database = Some(Database::open(&database_path).map_err(|error| error.to_string())?);
    state.database_ready = true;
    state.database_error = None;
    state.download_root = selected;
    state.download_setup_required = false;
    let executor_config = crate::runtime::executor_config(
        &paths.root,
        &state.config,
        database_path,
        staging_root,
        state.download_root.clone(),
    );
    state.replace_executor(executor_config)?;
    let system_download_root =
        system_download_archive_directory(&app).map(|path| path.display().to_string());
    Ok(PortableSetup {
        portable_root: state.portable_root.display().to_string(),
        download_root: state.download_root.display().to_string(),
        system_download_root,
        required: false,
    })
}

/// Persist a new archive directory and rebuild the executor around it.
///
/// The staging root, database, and logs stay where they are: only the archive
/// commit target changes. This mirrors `complete_download_setup` so a later
/// launch restores the same directory from `config.yaml`.
#[tauri::command]
pub(crate) fn set_archive_directory(
    state: State<'_, Mutex<RuntimeState>>,
    directory: String,
) -> Result<AppStatus, String> {
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let selected = crate::portable::normalize_path(Path::new(directory.trim()));
    validate_archive_directory(&state.portable_root, &selected)?;
    std::fs::create_dir_all(&selected).map_err(|error| format!("无法创建归档目录：{error}"))?;
    state.config.download.mode = "custom".to_owned();
    state.config.download.directory = selected
        .strip_prefix(&state.portable_root)
        .map(|path| format!("./{}", path.to_string_lossy().replace('\\', "/")))
        .unwrap_or_else(|_| selected.to_string_lossy().to_string());
    state.config.download.initialized = true;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.debug(
        "storage",
        &format!("archive directory changed to {}", selected.display()),
    );
    state.config.save(&paths)?;
    state.download_root = selected.clone();
    state.download_setup_required = false;
    let executor_config = crate::runtime::executor_config(
        &paths.root,
        &state.config,
        state.config.database_path(&paths),
        state.cache_root.join("staging"),
        selected,
    );
    state.replace_executor(executor_config)?;
    Ok(app_status(&state))
}

#[tauri::command]
pub(crate) fn validate_gallery_dl_path(path: String) -> Result<GalleryDlInstallation, String> {
    let candidate = PathBuf::from(path.trim());
    match gallery_dl_version(&candidate) {
        Ok(version) => Ok(GalleryDlInstallation {
            found: true,
            version: Some(version),
            path: Some(candidate.display().to_string()),
            error: None,
        }),
        Err(error) => Ok(GalleryDlInstallation {
            found: false,
            version: None,
            path: Some(candidate.display().to_string()),
            error: Some(error),
        }),
    }
}

#[tauri::command]
pub(crate) fn save_gallery_dl_path(
    state: State<'_, Mutex<RuntimeState>>,
    path: String,
) -> Result<GalleryDlInstallation, String> {
    let validated = validate_gallery_dl_path(path.clone())?;
    if !validated.found {
        return Err(validated
            .error
            .clone()
            .unwrap_or_else(|| "gallery-dl 路径无效".to_owned()));
    }
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    state.config.sidecar.gallery_dl = validated.path.clone().unwrap_or(path);
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.config.save(&paths)?;
    Ok(validated)
}

#[tauri::command]
pub(crate) fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text.as_str()))
        .map_err(|error| format!("failed to copy to clipboard: {error}"))
}

#[tauri::command]
pub(crate) fn open_archive_folder(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let path = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?
        .download_root
        .clone();
    let mut command = crate::platform::open_path_command(&path);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open archive folder: {error}"))
}

#[cfg(test)]
mod archive_directory_tests {
    use super::validate_archive_directory;
    use std::path::Path;

    #[test]
    fn accepts_an_absolute_directory_outside_the_portable_root() {
        let portable_root = std::env::temp_dir().join("xarchive-app");
        assert!(
            validate_archive_directory(
                &portable_root,
                &std::env::temp_dir().join("xarchive-output")
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_relative_and_portable_root_targets() {
        let portable_root = std::env::temp_dir().join("xarchive-app");
        assert!(validate_archive_directory(&portable_root, Path::new("download")).is_err());
        assert!(validate_archive_directory(&portable_root, &portable_root).is_err());
    }

    #[test]
    fn rejects_an_existing_file_target() {
        let directory = std::env::temp_dir().join(format!(
            "xarchive-archive-directory-test-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&directory).expect("temp directory");
        let file = directory.join("archive");
        std::fs::write(&file, b"not a directory").expect("temp file");
        assert!(validate_archive_directory(Path::new("/tmp/portable"), &file).is_err());
        let _ = std::fs::remove_dir_all(&directory);
    }
}
