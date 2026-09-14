use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;
use xarchive_sidecar_supervisor::SidecarSupervisor;

use crate::archive::upsert_browser_user;
use crate::executor::{ArchiveJobSubmissionAdapter, ExecutorError, JobSnapshot};
use crate::{ArchiveTweetRequest, RuntimeState};
use xarchive_storage::Database;

#[derive(Debug, Serialize)]
pub struct ExecutorSubmitResponse {
    pub job_id: String,
    pub tweet_id: String,
    pub state: String,
    pub created: bool,
}

#[derive(Debug, Serialize)]
pub struct ExecutorJobResponse {
    pub job_id: String,
    pub tweet_id: String,
    pub state: String,
}

fn executor_error(error: ExecutorError) -> String {
    error.to_string()
}

fn job_response(snapshot: JobSnapshot) -> ExecutorJobResponse {
    ExecutorJobResponse {
        job_id: snapshot.job_id,
        tweet_id: snapshot.tweet_id,
        state: snapshot.state.as_str().to_owned(),
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
    pub sidecar_error: Option<String>,
    pub executor: String,
}

#[tauri::command]
pub(crate) fn get_app_status(state: State<'_, Mutex<RuntimeState>>) -> AppStatus {
    let mut state = state.lock().expect("runtime state lock poisoned");
    refresh_sidecar_state(&mut state);
    AppStatus {
        app_name: "XArchive",
        app_version: env!("CARGO_PKG_VERSION"),
        sidecar: sidecar_status(&state),
        database: if state.database_ready {
            "ready".to_owned()
        } else {
            "error".to_owned()
        },
        platform: std::env::consts::OS,
        archive_root: state.archive_root.display().to_string(),
        database_error: state.database_error.clone(),
        sidecar_error: state.sidecar_error.clone(),
        executor: if state.executor.is_running() {
            "ready".to_owned()
        } else {
            "stopped".to_owned()
        },
    }
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

fn sidecar_configuration() -> Result<(String, Vec<String>), String> {
    let program = std::env::var("XARCHIVE_SIDECAR_PROGRAM")
        .map_err(|_| "XARCHIVE_SIDECAR_PROGRAM is not configured".to_owned())?;
    if program.trim().is_empty() {
        return Err("XARCHIVE_SIDECAR_PROGRAM is empty".to_owned());
    }
    let args = parse_sidecar_args(&std::env::var("XARCHIVE_SIDECAR_ARGS").unwrap_or_default())?;
    Ok((program, args))
}

pub(crate) fn parse_sidecar_args(raw: &str) -> Result<Vec<String>, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw)
        .map_err(|error| format!("XARCHIVE_SIDECAR_ARGS must be a JSON string array: {error}"))
}

pub(crate) fn refresh_sidecar_state(state: &mut RuntimeState) {
    let exited = state
        .sidecar
        .as_mut()
        .and_then(|supervisor| supervisor.poll_exit().ok().flatten());
    if let Some(status) = exited {
        state.sidecar.take();
        state.sidecar_error = Some(format!("sidecar exited: {status}"));
    }
}

#[tauri::command]
pub(crate) fn start_sidecar(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let (program, args) = sidecar_configuration()?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    if state.sidecar.is_some() {
        return Ok("ready".to_owned());
    }
    state.sidecar_error = None;
    match SidecarSupervisor::spawn_ready(&program, &arg_refs, Duration::from_secs(5)) {
        Ok(supervisor) => {
            state.sidecar = Some(supervisor);
            Ok("ready".to_owned())
        }
        Err(error) => {
            let message = error.to_string();
            state.sidecar_error = Some(message.clone());
            Err(message)
        }
    }
}

#[tauri::command]
pub(crate) fn stop_sidecar(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    if let Some(mut supervisor) = state.sidecar.take() {
        let shutdown = xarchive_protocol::SidecarCommand {
            protocol_version: xarchive_protocol::PROTOCOL_VERSION,
            request_id: "desktop-shutdown".to_owned(),
            cmd: xarchive_protocol::SidecarCommandType::Shutdown,
            job_id: "system".to_owned(),
            url: None,
            staging_dir: None,
            browser: None,
            profile: None,
        };
        let _ = supervisor.send(&shutdown);
        supervisor.close_stdin();
        if !supervisor
            .wait_for_exit(Duration::from_secs(1))
            .unwrap_or(false)
        {
            supervisor.shutdown();
        }
        state.sidecar_error = None;
        Ok("stopped".to_owned())
    } else {
        Ok("stopped".to_owned())
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
    let database = state
        .database
        .as_ref()
        .ok_or_else(|| "archive database is not initialized".to_owned())?;
    database
        .list_recent_jobs(limit.unwrap_or(20))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_archive_root(state: State<'_, Mutex<RuntimeState>>) -> String {
    state
        .lock()
        .expect("runtime state lock poisoned")
        .archive_root
        .display()
        .to_string()
}

#[tauri::command]
pub(crate) fn open_archive_folder(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let path = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?
        .archive_root
        .clone();
    let mut command = crate::platform::open_path_command(&path);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open archive folder: {error}"))
}

#[tauri::command]
pub(crate) fn get_runtime_health(state: State<'_, Mutex<RuntimeState>>) -> bool {
    state
        .lock()
        .expect("runtime state lock poisoned")
        .database_ready
}

/// Submit and execute one archive Job through the real executor worker.
///
/// RuntimeState is used only to lease the SidecarSupervisor and to obtain
/// immutable paths/handles. SQLite, FileStore, Sidecar and ArchiveService I/O
/// happen after the state lock has been released.
#[tauri::command]
pub(crate) fn submit_executor_job(
    state: State<'_, Mutex<RuntimeState>>,
    request: ArchiveTweetRequest,
) -> Result<ExecutorSubmitResponse, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
        )
    };
    let timestamp = crate::runtime::timestamp_marker();
    let mut database = match Database::open(&database_path) {
        Ok(database) => database,
        Err(error) => return Err(error.to_string()),
    };
    let now = crate::runtime::timestamp_marker();
    let tweet_row_id = match database.insert_tweet(
        &request.tweet.tweet_id,
        &request.tweet.url,
        &request.tweet.tweet_type,
        request.tweet.text.as_deref().unwrap_or_default(),
        &now,
    ) {
        Ok(tweet_row_id) => tweet_row_id,
        Err(error) => return Err(error.to_string()),
    };
    if let Err(error) = upsert_browser_user(&mut database, tweet_row_id, &request.tweet, &now) {
        return Err(error.to_string());
    }
    let mut persistence = crate::executor::StorageJobPersistence::open(database_path.clone())?;
    let result = match ArchiveJobSubmissionAdapter.submit(
        &service,
        &mut persistence,
        &request,
        &timestamp,
    ) {
        Ok(result) => result,
        Err(error) => return Err(executor_error(error)),
    };
    if !result.created {
        return Ok(ExecutorSubmitResponse {
            job_id: result.job.job_id,
            tweet_id: result.job.tweet_id,
            state: result.job.state.as_str().to_owned(),
            created: false,
        });
    }
    drop(database);
    let snapshot = service
        .execute_persisted_from_factory(&mut persistence, &result.job.job_id)
        .map_err(executor_error)?;
    Ok(ExecutorSubmitResponse {
        job_id: snapshot.job_id,
        tweet_id: snapshot.tweet_id,
        state: snapshot.state.as_str().to_owned(),
        created: true,
    })
}

#[tauri::command]
pub(crate) fn query_executor_job(
    state: State<'_, Mutex<RuntimeState>>,
    job_id: String,
) -> Result<ExecutorJobResponse, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
        )
    };
    let persistence = crate::executor::StorageJobPersistence::open(database_path)?;
    service
        .query_persisted(&persistence, &job_id)
        .map(job_response)
        .map_err(executor_error)
}

#[tauri::command]
pub(crate) fn cancel_executor_job(
    state: State<'_, Mutex<RuntimeState>>,
    job_id: String,
) -> Result<ExecutorJobResponse, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
        )
    };
    let mut persistence = crate::executor::StorageJobPersistence::open(database_path)?;
    service
        .cancel_persisted(&mut persistence, &job_id)
        .map(job_response)
        .map_err(executor_error)
}

#[tauri::command]
pub(crate) fn shutdown_executor(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
        )
    };
    let mut persistence = crate::executor::StorageJobPersistence::open(database_path)?;
    service
        .interrupt_persisted(&mut persistence)
        .map_err(executor_error)?;

    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    state
        .executor
        .shutdown_in_place()
        .map(|()| "stopped".to_owned())
        .map_err(executor_error)
}
