//! Tauri commands for account batches: creation, listing, inspection,
//! pause/resume, cancellation and retry.
//!
//! These are thin adapters over `RuntimeState`, the batch scheduler and the
//! storage batch API. Command names, argument shapes and error strings are the
//! registered IPC contract and must not change here.

use std::sync::Mutex;
use std::time::Duration;
use tauri::State;

use crate::RuntimeState;
use crate::batch::{spawn_account_batch, spawn_batch_dispatch};
use crate::commands::app::CreateAccountBatchRequest;
use crate::commands::open_batch_database;
use crate::commands::settings::{
    read_downloader_settings, read_output_settings, snapshot_downloader_settings,
    validate_output_settings,
};
use crate::executor::CancellationToken;

fn spawn_batch_from_state(state: &RuntimeState, batch_id: &str) -> Result<(), String> {
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    let program = crate::portable::resolve_config_path(&paths.root, &state.config.sidecar.worker);
    if !program.is_file() {
        return Err("XArchive Sidecar worker was not found".to_owned());
    }
    let files = xarchive_storage::FileStore::with_staging_root(
        state.download_root.clone(),
        state.cache_root.join("staging"),
    )
    .map_err(|error| error.to_string())?;
    let cancellation = CancellationToken::new();
    let cancellations = state.batch_cancellations.clone();
    spawn_account_batch(
        state.executor.service(),
        state.executor.database_path().to_owned(),
        files,
        batch_id.to_owned(),
        program.display().to_string(),
        crate::runtime::sidecar_runtime_args(&paths.root, &state.config),
        state.config.network.sidecar_env(),
        state.config.network.sidecar_env_remove(),
        Duration::from_secs(state.config.network.discovery_timeout_seconds.max(1)),
        cancellation,
        cancellations,
    )
}

#[tauri::command]
pub(crate) fn create_account_batch(
    state: State<'_, Mutex<RuntimeState>>,
    request: CreateAccountBatchRequest,
) -> Result<xarchive_storage::AccountBatchSummary, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    if state.download_setup_required {
        return Err("download directory setup is required before account archiving".to_owned());
    }
    let username = request.username.trim().trim_start_matches('@').to_owned();
    if username.is_empty() || request.profile_url.trim().is_empty() {
        return Err("username and profile_url are required".to_owned());
    }
    let filters_json = request.filters.to_json()?;
    let id = format!("batch-{}", crate::runtime::timestamp_marker());
    let database = open_batch_database(&state)?;
    let output_settings = match request.output_settings {
        Some(settings) => settings,
        None => read_output_settings(&database)?,
    };
    validate_output_settings(&output_settings)?;
    let tool_compatibility = if state.config.download.use_aria2 {
        crate::executor::ToolCompatibility {
            aria2: state
                .executor
                .aria2_program()
                .map(|program| crate::production::resolved_tool_version(program, &["--version"]))
                .transpose()?,
            gallery_dl: None,
        }
    } else {
        crate::executor::ToolCompatibility {
            aria2: None,
            gallery_dl: state
                .executor
                .gallery_dl_program()
                .map(|program| crate::production::resolved_tool_version(program, &["--version"]))
                .transpose()?,
        }
    };
    let downloader_settings = read_downloader_settings(&database)?;
    let downloader_arguments = snapshot_downloader_settings(
        &downloader_settings,
        state.config.download.use_aria2,
        tool_compatibility,
    )?;
    let downloader_arguments_json =
        serde_json::to_string(&downloader_arguments).map_err(|error| error.to_string())?;
    let batch = xarchive_storage::CreateAccountBatch {
        id: &id,
        username: &username,
        profile_url: request.profile_url.trim(),
        browser: request.browser.as_deref(),
        profile: request.profile.as_deref(),
        filters_json: &filters_json,
        output_settings: &output_settings,
        downloader_arguments_json: &downloader_arguments_json,
    };
    let metadata_output = request
        .metadata_output
        .or(crate::executor::MetadataOutputSettings::read(&database)?);
    if let Some(settings) = metadata_output {
        validate_output_settings(&settings.legacy_naming())?;
        let snapshot = serde_json::json!({"schema_version":4,"settings":settings}).to_string();
        database.create_account_batch_with_metadata_snapshot(batch, &snapshot)
    } else {
        database.create_account_batch(batch)
    }
    .map_err(|error| error.to_string())?;
    spawn_batch_from_state(&state, &id)?;
    database
        .account_batch(&id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "created batch could not be read".to_owned())
}

#[tauri::command]
pub(crate) fn list_account_batches(
    state: State<'_, Mutex<RuntimeState>>,
    limit: Option<u32>,
) -> Result<Vec<xarchive_storage::AccountBatchSummary>, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    open_batch_database(&state)?
        .list_account_batches(limit.unwrap_or(20).clamp(1, 100))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_account_batch(
    state: State<'_, Mutex<RuntimeState>>,
    batch_id: String,
) -> Result<Option<xarchive_storage::AccountBatchSummary>, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    open_batch_database(&state)?
        .account_batch(&batch_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn list_account_batch_candidates(
    state: State<'_, Mutex<RuntimeState>>,
    batch_id: String,
    candidate_state: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<xarchive_storage::BatchCandidateRecord>, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    open_batch_database(&state)?
        .list_batch_candidates(
            &batch_id,
            candidate_state.as_deref(),
            limit.unwrap_or(100).clamp(1, 500),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn pause_account_batch(
    state: State<'_, Mutex<RuntimeState>>,
    batch_id: String,
) -> Result<xarchive_storage::AccountBatchSummary, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let mut database = open_batch_database(&state)?;
    let paused = database
        .pause_account_batch(&batch_id)
        .map_err(|error| error.to_string())?;
    let tokens = state
        .batch_cancellations
        .lock()
        .map_err(|_| "batch cancellation registry poisoned".to_owned())?;
    if let Some(token) = tokens.get(&batch_id) {
        token.cancel();
    }
    Ok(paused)
}

#[tauri::command]
pub(crate) fn resume_account_batch(
    state: State<'_, Mutex<RuntimeState>>,
    batch_id: String,
) -> Result<xarchive_storage::AccountBatchSummary, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let database = open_batch_database(&state)?;
    let batch = database
        .account_batch(&batch_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "unknown batch".to_owned())?;
    if batch.state != "PAUSED" {
        return Err("only paused batches can be resumed".to_owned());
    }
    if state
        .batch_cancellations
        .lock()
        .map_err(|_| "batch cancellation registry poisoned".to_owned())?
        .contains_key(&batch_id)
    {
        return Err("batch worker is still stopping".to_owned());
    }
    database
        .set_account_batch_state(&batch_id, "ACTIVE")
        .map_err(|error| error.to_string())?;
    let discovery_complete = batch.discovery_state == "COMPLETED";
    let spawn_result = if discovery_complete {
        let files = xarchive_storage::FileStore::with_staging_root(
            state.download_root.clone(),
            state.cache_root.join("staging"),
        )
        .map_err(|error| error.to_string());
        files.and_then(|files| {
            crate::batch::spawn_batch_dispatch(
                state.executor.service(),
                state.executor.database_path().to_owned(),
                files,
                batch_id.clone(),
                state.batch_cancellations.clone(),
            )
        })
    } else {
        spawn_batch_from_state(&state, &batch_id)
    };
    if let Err(error) = spawn_result {
        let _ = database.set_account_batch_state(&batch_id, "PAUSED");
        return Err(error);
    }
    database
        .account_batch(&batch_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "unknown batch".to_owned())
}

#[tauri::command]
pub(crate) fn cancel_account_batch(
    state: State<'_, Mutex<RuntimeState>>,
    batch_id: String,
) -> Result<xarchive_storage::AccountBatchSummary, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let mut database = open_batch_database(&state)?;
    let cancelled = database
        .cancel_account_batch(&batch_id)
        .map_err(|error| error.to_string())?;
    let tokens = state
        .batch_cancellations
        .lock()
        .map_err(|_| "batch cancellation registry poisoned".to_owned())?;
    if let Some(token) = tokens.get(&batch_id) {
        token.cancel();
    }
    Ok(cancelled)
}

#[tauri::command]
pub(crate) fn retry_account_batch(
    state: State<'_, Mutex<RuntimeState>>,
    batch_id: String,
) -> Result<xarchive_storage::AccountBatchSummary, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let database = open_batch_database(&state)?;
    let batch = database
        .account_batch(&batch_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "unknown batch".to_owned())?;
    if !matches!(batch.state.as_str(), "PAUSED" | "FAILED") {
        return Err("only paused or failed batches can be retried".to_owned());
    }
    if state
        .batch_cancellations
        .lock()
        .map_err(|_| "batch cancellation registry poisoned".to_owned())?
        .contains_key(&batch_id)
    {
        return Err("batch worker is still stopping".to_owned());
    }
    database
        .retry_failed_batch_candidates(&batch_id)
        .map_err(|error| error.to_string())?;
    database
        .set_account_batch_state(&batch_id, "ACTIVE")
        .map_err(|error| error.to_string())?;
    let discovery_state = batch.discovery_state.clone();
    let spawn_result = if discovery_state == "FAILED" {
        database
            .set_account_batch_discovery_state(&batch_id, "PENDING")
            .map_err(|error| error.to_string())?;
        spawn_batch_from_state(&state, &batch_id)
    } else {
        let files = xarchive_storage::FileStore::with_staging_root(
            state.download_root.clone(),
            state.cache_root.join("staging"),
        )
        .map_err(|error| error.to_string());
        files.and_then(|files| {
            spawn_batch_dispatch(
                state.executor.service(),
                state.executor.database_path().to_owned(),
                files,
                batch_id.clone(),
                state.batch_cancellations.clone(),
            )
        })
    };
    if let Err(error) = spawn_result {
        let _ = database.set_account_batch_state(&batch_id, "FAILED");
        if discovery_state == "FAILED" {
            let _ = database.set_account_batch_discovery_state(&batch_id, "FAILED");
        }
        return Err(error);
    }
    database
        .account_batch(&batch_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "unknown batch".to_owned())
}
