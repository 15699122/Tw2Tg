//! Job read-model commands: recent jobs, paged jobs, history index, download
//! metrics and aggregate metrics.
//!
//! These read the persisted read-model directly and never take the executor
//! service handle, so they are deliberately separate from `commands::executor`.
//! Command names, argument shapes, serialized fields and error strings are the
//! registered IPC contract and must not change here.

use std::sync::Mutex;

use serde::Deserialize;
use tauri::State;
use xarchive_storage::Database;

use crate::RuntimeState;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ListJobsPageRequest {
    pub offset: u32,
    pub limit: u32,
}

pub(crate) fn redact_job_errors(jobs: &mut [xarchive_storage::JobSummary], secrets: &[String]) {
    for job in jobs {
        job.last_error_message = job
            .last_error_message
            .as_deref()
            .map(|message| crate::logging::redact_line(message, secrets));
    }
}

#[tauri::command]
pub(crate) fn list_jobs(
    state: State<'_, Mutex<RuntimeState>>,
    limit: Option<u32>,
) -> Result<Vec<xarchive_storage::JobSummary>, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let mut jobs = match state.database.as_ref() {
        Some(database) => database
            .list_recent_jobs(limit.unwrap_or(20))
            .map_err(|error| error.to_string())?,
        None => Database::open(state.executor.database_path())
            .map_err(|error| error.to_string())?
            .list_recent_jobs(limit.unwrap_or(20))
            .map_err(|error| error.to_string())?,
    };
    redact_job_errors(&mut jobs, &state.config.log_secrets());
    Ok(jobs)
}

#[tauri::command]
pub(crate) fn list_jobs_page(
    state: State<'_, Mutex<RuntimeState>>,
    request: ListJobsPageRequest,
) -> Result<xarchive_storage::PagedJobs, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let mut page = match state.database.as_ref() {
        Some(database) => database.list_jobs_page(request.offset, request.limit),
        None => Database::open(state.executor.database_path())
            .and_then(|db| db.list_jobs_page(request.offset, request.limit)),
    }
    .map_err(|error| error.to_string())?;
    redact_job_errors(&mut page.jobs, &state.config.log_secrets());
    Ok(page)
}

#[tauri::command]
pub(crate) fn get_job_history_index(
    state: State<'_, Mutex<RuntimeState>>,
    job_id: String,
) -> Result<Option<u32>, String> {
    if job_id.trim().is_empty() || job_id.len() > 200 {
        return Err("invalid job id".to_owned());
    }
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    match state.database.as_ref() {
        Some(database) => database.job_history_index(&job_id),
        None => Database::open(state.executor.database_path())
            .and_then(|db| db.job_history_index(&job_id)),
    }
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_job_download_metrics(
    state: State<'_, Mutex<RuntimeState>>,
    job_id: String,
) -> Result<Option<xarchive_storage::JobDownloadMetrics>, String> {
    if job_id.trim().is_empty() || job_id.len() > 200 {
        return Err("invalid job id".to_owned());
    }
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    match state.database.as_ref() {
        Some(database) => database.job_download_metrics(&job_id),
        None => Database::open(state.executor.database_path())
            .and_then(|db| db.job_download_metrics(&job_id)),
    }
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_job_metrics(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<xarchive_storage::JobMetrics, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    match state.database.as_ref() {
        Some(database) => database.job_metrics().map_err(|error| error.to_string()),
        None => Database::open(state.executor.database_path())
            .map_err(|error| error.to_string())?
            .job_metrics()
            .map_err(|error| error.to_string()),
    }
}
