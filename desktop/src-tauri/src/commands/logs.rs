//! Application-log commands: frontend diagnostics, log reading and log folder
//! opening.
//!
//! `log_frontend_event` is a redaction boundary: the configured log secrets are
//! removed before anything reaches disk. Command names, argument shapes and
//! error strings are the registered IPC contract and must not change here.

use serde::Deserialize;
use std::sync::Mutex;
use tauri::State;

use crate::RuntimeState;
use crate::commands::bounded_diagnostic;

#[derive(Debug, Clone, Deserialize)]
pub struct FrontendDiagnosticEvent {
    pub event: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub source: String,
}

#[tauri::command]
pub(crate) fn log_frontend_event(
    state: State<'_, Mutex<RuntimeState>>,
    event: FrontendDiagnosticEvent,
) -> Result<(), String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let event_name = bounded_diagnostic(&event.event, 64);
    if event_name.is_empty() {
        return Err("frontend diagnostic event is empty".to_owned());
    }
    // Browser-visible diagnostics are a P2-A redaction boundary: the same
    // configured secrets as the log file apply before anything reaches disk.
    let detail = crate::logging::redact_line(
        &format!(
            "frontend event={} state={} message={} context={} source={}",
            event_name,
            bounded_diagnostic(&event.state, 64),
            bounded_diagnostic(&event.message, 512).replace('\n', " "),
            bounded_diagnostic(&event.context, 2048).replace('\n', " "),
            bounded_diagnostic(&event.source, 2048).replace('\n', " "),
        ),
        &state.config.log_secrets(),
    );
    state
        .log_file
        .as_ref()
        .ok_or_else(|| "application log is unavailable".to_owned())?
        .append(crate::config::LogLevel::Info, &detail)
}

#[tauri::command]
pub(crate) fn read_application_logs(
    state: State<'_, Mutex<RuntimeState>>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    crate::logging::LogFile::read_recent(&state.logs_root, limit.unwrap_or(500))
}

#[tauri::command]
pub(crate) fn open_log_folder(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    crate::platform::open_path_command(&state.logs_root)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open log folder: {error}"))
}
