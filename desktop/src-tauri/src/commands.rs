use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;
use xarchive_sidecar_supervisor::SidecarSupervisor;

use crate::archive::upsert_browser_user;
use crate::config::{LogLevel, MAX_LOG_MAX_FILES, MIN_LOG_MAX_FILES};
use crate::executor::{ArchiveJobSubmissionAdapter, ExecutorError, JobSnapshot};
use crate::portable::system_download_archive_directory;
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
    pub download_setup_required: bool,
    pub logs_root: String,
    pub logging_level: String,
    pub max_log_files: usize,
}

#[derive(Debug, Serialize)]
pub struct PortableSetup {
    pub portable_root: String,
    pub download_root: String,
    pub system_download_root: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationSettingsInput {
    pub logging_level: LogLevel,
    pub max_log_files: usize,
}

#[derive(Debug, Serialize)]
pub struct ExtensionStatus {
    pub files_ready: bool,
    pub directory: String,
    pub browser_connection: String,
    pub native_host: String,
    pub message: String,
}

#[tauri::command]
pub(crate) fn get_app_status(state: State<'_, Mutex<RuntimeState>>) -> AppStatus {
    let mut state = state.lock().expect("runtime state lock poisoned");
    refresh_sidecar_state(&mut state);
    app_status(&state)
}

fn app_status(state: &RuntimeState) -> AppStatus {
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
        database_error: state.database_error.clone(),
        sidecar_error: state.sidecar_error.clone(),
        executor: if state.executor.is_running() {
            "ready".to_owned()
        } else {
            "stopped".to_owned()
        },
        download_setup_required: state.download_setup_required,
        logs_root: state.logs_root.display().to_string(),
        logging_level: state.config.logging.level.as_str().to_owned(),
        max_log_files: state.config.logging.max_files,
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
    if let Ok(program) = std::env::var("XARCHIVE_SIDECAR_PROGRAM") {
        if program.trim().is_empty() {
            return Err("XARCHIVE_SIDECAR_PROGRAM is empty".to_owned());
        }
        let args = parse_sidecar_args(&std::env::var("XARCHIVE_SIDECAR_ARGS").unwrap_or_default())?;
        return Ok((program, args));
    }
    let paths = crate::portable::PortablePaths::from_root(crate::portable::portable_root());
    let (config, _) = crate::config::AppConfig::load(&paths);
    let configured = crate::portable::resolve_config_path(&paths.root, &config.sidecar.gallery_dl);
    let (program, args) = if configured.is_file() {
        (configured.display().to_string(), Vec::new())
    } else {
        return Err(
            "XARCHIVE_SIDECAR_PROGRAM is not configured and bundled gallery-dl was not found"
                .to_owned(),
        );
    };
    if program.trim().is_empty() {
        return Err("XARCHIVE_SIDECAR_PROGRAM is empty".to_owned());
    }
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
        .download_root
        .display()
        .to_string()
}

#[tauri::command]
pub(crate) fn get_portable_setup(state: State<'_, Mutex<RuntimeState>>) -> PortableSetup {
    let state = state.lock().expect("runtime state lock poisoned");
    PortableSetup {
        portable_root: state.portable_root.display().to_string(),
        download_root: state.download_root.display().to_string(),
        system_download_root: system_download_archive_directory()
            .map(|path| path.display().to_string()),
        required: state.download_setup_required,
    }
}

#[tauri::command]
pub(crate) fn complete_download_setup(
    state: State<'_, Mutex<RuntimeState>>,
    choice: String,
) -> Result<PortableSetup, String> {
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let selected = match choice.as_str() {
        "portable" => state.portable_root.join("download"),
        "system_downloads" => system_download_archive_directory()
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
    state
        .executor
        .shutdown_in_place()
        .map_err(|error| error.to_string())?;
    state.executor =
        crate::executor::ExecutorRuntime::with_config(crate::executor::ExecutorConfig {
            archive_root: state.download_root.clone(),
            staging_root,
            database_path,
            sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok(),
            sidecar_args: std::env::var("XARCHIVE_SIDECAR_ARGS")
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
        });
    let system_download_root =
        system_download_archive_directory().map(|path| path.display().to_string());
    Ok(PortableSetup {
        portable_root: state.portable_root.display().to_string(),
        download_root: state.download_root.display().to_string(),
        system_download_root,
        required: false,
    })
}

#[tauri::command]
pub(crate) fn save_application_settings(
    state: State<'_, Mutex<RuntimeState>>,
    settings: ApplicationSettingsInput,
) -> Result<AppStatus, String> {
    if !(MIN_LOG_MAX_FILES..=MAX_LOG_MAX_FILES).contains(&settings.max_log_files) {
        return Err(format!(
            "max_log_files must be between {MIN_LOG_MAX_FILES} and {MAX_LOG_MAX_FILES}"
        ));
    }
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    state.config.logging.level = settings.logging_level;
    state.config.logging.max_files = settings.max_log_files;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.config.save(&paths)?;
    state.log_file = crate::logging::LogFile::open(
        &state.logs_root,
        settings.logging_level,
        settings.max_log_files,
    )
    .ok();
    Ok(app_status(&state))
}

#[tauri::command]
pub(crate) fn get_sidecar_path(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    Ok(
        crate::portable::resolve_config_path(&paths.root, &state.config.sidecar.gallery_dl)
            .display()
            .to_string(),
    )
}

#[tauri::command]
pub(crate) fn save_aria2_path(
    state: State<'_, Mutex<RuntimeState>>,
    path: String,
) -> Result<crate::aria2::Aria2Installation, String> {
    let validated = crate::aria2::validate_aria2_path(path.clone());
    if !validated.found {
        return Err(validated
            .error
            .unwrap_or_else(|| "the given path is not a working aria2c executable".to_owned()));
    }
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    state.config.sidecar.aria2 = validated.path.clone().unwrap_or(path);
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

#[tauri::command]
pub(crate) fn get_extension_status(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ExtensionStatus, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    let directory =
        crate::portable::resolve_config_path(&paths.root, &state.config.extension.directory);
    let required_files = [
        directory.join("manifest.json"),
        directory.join("src").join("background.js"),
        directory.join("src").join("content.js"),
    ];
    let files_ready = required_files.iter().all(|path| path.is_file());
    Ok(ExtensionStatus {
        files_ready,
        directory: directory.display().to_string(),
        browser_connection: "unknown".to_owned(),
        native_host: if cfg!(windows) {
            "not_verified".to_owned()
        } else {
            "not_available_on_linux".to_owned()
        },
        message: if files_ready {
            "扩展文件已就绪；浏览器加载和 Native Host 连接需要在目标浏览器中验证。".to_owned()
        } else {
            "未找到完整的 Extension 文件，请检查便携目录中的 extension 文件夹。".to_owned()
        },
    })
}

#[tauri::command]
pub(crate) fn open_extension_folder(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let path = get_extension_status(state)?.directory;
    let mut command = crate::platform::open_path_command(std::path::Path::new(&path));
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open extension folder: {error}"))
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
