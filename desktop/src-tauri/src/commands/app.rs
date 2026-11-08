//! App status commands: the runtime projection shown on startup and in Settings,
//! component bootstrap status and the runtime health probe.
//!
//! `AppStatus` and `CreateAccountBatchRequest` stay public in this module because
//! the settings, archive and batch domains also use them. Command names, argument
//! shapes, serialized fields and error strings are the registered IPC contract and
//! must not change here.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::RuntimeState;
use crate::batch::BatchFilters;
use crate::commands::refresh_sidecar_state;
use crate::components::ComponentBootstrapStatus;

#[derive(Debug, Serialize)]
pub struct AppStatus {
    pub app_name: &'static str,
    pub app_version: &'static str,
    pub sidecar: String,
    pub database: String,
    pub platform: &'static str,
    pub archive_root: String,
    pub gallery_dl_path: Option<String>,
    pub database_error: Option<String>,
    pub sidecar_error: Option<String>,
    pub executor: String,
    pub download_setup_required: bool,
    pub logs_root: String,
    pub logging_level: String,
    pub max_log_files: usize,
    /// Whether the optional aria2 transfer backend is enabled. When false,
    /// gallery-dl downloads the media bytes itself and aria2 never starts.
    pub use_aria2: bool,
}

#[derive(Debug, Serialize)]
pub struct PortableSetup {
    pub portable_root: String,
    pub download_root: String,
    pub system_download_root: Option<String>,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct GalleryDlInstallation {
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateAccountBatchRequest {
    #[serde(default)]
    pub metadata_output: Option<crate::executor::MetadataOutputSettings>,
    pub username: String,
    pub profile_url: String,
    pub browser: Option<String>,
    pub profile: Option<String>,
    #[serde(default)]
    pub filters: BatchFilters,
    #[serde(default)]
    pub output_settings: Option<xarchive_storage::BatchOutputSettings>,
}

fn sidecar_status(state: &RuntimeState) -> String {
    if state.sidecar.is_some() {
        "ready".to_owned()
    } else if state.sidecar_error.is_some() {
        "failed".to_owned()
    } else {
        "not_configured".to_owned()
    }
}

pub(crate) fn app_status(state: &RuntimeState) -> AppStatus {
    AppStatus {
        app_name: "XArchive",
        app_version: env!("CARGO_PKG_VERSION"),
        sidecar: sidecar_status(state),
        database: if state.database_ready {
            "ready".to_owned()
        } else {
            "error".to_owned()
        },
        platform: std::env::consts::OS,
        archive_root: state.download_root.display().to_string(),
        gallery_dl_path: Some(
            crate::portable::resolve_config_path(
                &state.portable_root,
                &state.config.sidecar.gallery_dl,
            )
            .display()
            .to_string(),
        ),
        database_error: state.database_error.clone(),
        sidecar_error: state.sidecar_error.clone(),
        executor: if state.executor.is_running() {
            "ready".to_owned()
        } else {
            "stopped".to_owned()
        },
        download_setup_required: state.download_setup_required,
        logs_root: state.logs_root.display().to_string(),
        logging_level: state.config.logging.effective_level().as_str().to_owned(),
        max_log_files: state.config.logging.max_files,
        use_aria2: state.config.download.use_aria2,
    }
}

#[tauri::command]
pub(crate) fn get_component_bootstrap_status(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ComponentBootstrapStatus, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    crate::components::ComponentManager::embedded(state.portable_root.join("components"))
        .map(|manager| manager.bootstrap_status())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_app_status(state: State<'_, Mutex<RuntimeState>>) -> AppStatus {
    let mut state = state.lock().expect("runtime state lock poisoned");
    refresh_sidecar_state(&mut state);
    app_status(&state)
}

#[tauri::command]
pub(crate) fn get_runtime_health(state: State<'_, Mutex<RuntimeState>>) -> bool {
    state
        .lock()
        .expect("runtime state lock poisoned")
        .database_ready
}
