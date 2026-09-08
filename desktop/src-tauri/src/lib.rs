use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use xarchive_storage::{Database, FileStore};

const DEFAULT_ARCHIVE_ROOT: &str = "X-Archive";

pub struct RuntimeState {
    archive_root: PathBuf,
    database_ready: bool,
    database_error: Option<String>,
}

impl RuntimeState {
    fn initialize() -> Self {
        let archive_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(DEFAULT_ARCHIVE_ROOT);
        let database_path = archive_root.join("_database").join("archive.sqlite3");
        let database_ready = (|| {
            FileStore::new(archive_root.clone()).ok()?;
            std::fs::create_dir_all(database_path.parent()?).ok()?;
            Database::open(&database_path).ok()?;
            Some(())
        })()
        .is_some();
        let database_error = if database_ready {
            None
        } else {
            Some("failed to initialize archive database".to_owned())
        };

        Self {
            archive_root,
            database_ready,
            database_error,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AppStatus {
    pub app_name: &'static str,
    pub app_version: &'static str,
    pub sidecar: String,
    pub database: String,
    pub platform: &'static str,
    pub archive_root: String,
    pub database_error: Option<String>,
}

#[tauri::command]
fn get_app_status(state: State<'_, Mutex<RuntimeState>>) -> AppStatus {
    let state = state.lock().expect("runtime state lock poisoned");
    AppStatus {
        app_name: "XArchive",
        app_version: env!("CARGO_PKG_VERSION"),
        sidecar: "not_configured".to_owned(),
        database: if state.database_ready {
            "ready".to_owned()
        } else {
            "error".to_owned()
        },
        platform: std::env::consts::OS,
        archive_root: state.archive_root.display().to_string(),
        database_error: state.database_error.clone(),
    }
}

#[tauri::command]
fn get_archive_root(state: State<'_, Mutex<RuntimeState>>) -> String {
    state
        .lock()
        .expect("runtime state lock poisoned")
        .archive_root
        .display()
        .to_string()
}

#[tauri::command]
fn get_runtime_health(state: State<'_, Mutex<RuntimeState>>) -> bool {
    state
        .lock()
        .expect("runtime state lock poisoned")
        .database_ready
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime_state = Mutex::new(RuntimeState::initialize());
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(runtime_state)
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_archive_root,
            get_runtime_health
        ])
        .run(tauri::generate_context!())
        .expect("error while running XArchive desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_state_uses_a_dedicated_archive_directory() {
        let state = RuntimeState::initialize();
        assert!(state.archive_root.ends_with(DEFAULT_ARCHIVE_ROOT));
    }
}
