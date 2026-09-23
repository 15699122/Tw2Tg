use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;
use xarchive_sidecar_supervisor::SidecarSupervisor;

use crate::archive::upsert_browser_user;
use crate::components::ComponentBootstrapStatus;
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

fn bounded_diagnostic(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
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
    let detail = format!(
        "frontend event={} state={} message={} context={} source={}",
        event_name,
        bounded_diagnostic(&event.state, 64),
        bounded_diagnostic(&event.message, 512).replace('\n', " "),
        bounded_diagnostic(&event.context, 2048).replace('\n', " "),
        bounded_diagnostic(&event.source, 2048).replace('\n', " "),
    );
    state
        .log_file
        .as_ref()
        .ok_or_else(|| "application log is unavailable".to_owned())?
        .append(crate::config::LogLevel::Info, &detail)
}

#[cfg(test)]
mod frontend_diagnostic_tests {
    use super::bounded_diagnostic;

    #[test]
    fn bounds_frontend_diagnostic_by_characters() {
        assert_eq!(bounded_diagnostic("abcdef", 3), "abc");
        assert_eq!(bounded_diagnostic("白屏诊断", 2), "白屏");
    }
}

#[derive(Debug, Serialize)]
pub struct PortableSetup {
    pub portable_root: String,
    pub download_root: String,
    pub system_download_root: Option<String>,
    pub required: bool,
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
    pub source: String,
    pub version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GalleryDlInstallation {
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>,
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
    let worker = crate::portable::resolve_config_path(&paths.root, &config.sidecar.worker);
    let gallery = crate::portable::resolve_config_path(&paths.root, &config.sidecar.gallery_dl);
    let (program, args) = if worker.is_file() {
        (
            worker.display().to_string(),
            vec!["--gallery-dl".to_owned(), gallery.display().to_string()],
        )
    } else {
        return Err("XArchive Sidecar worker was not found".to_owned());
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
    match SidecarSupervisor::spawn_ready_v2(&program, &arg_refs, Duration::from_secs(5)) {
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
        let _ = supervisor.send_v2_shutdown("desktop-shutdown");
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
    let database = match state.database.as_ref() {
        Some(database) => database
            .list_recent_jobs(limit.unwrap_or(20))
            .map_err(|error| error.to_string())?,
        None => Database::open(state.executor.database_path())
            .map_err(|error| error.to_string())?
            .list_recent_jobs(limit.unwrap_or(20))
            .map_err(|error| error.to_string())?,
    };
    Ok(database)
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
            sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok().or_else(|| {
                let worker =
                    crate::portable::resolve_config_path(&paths.root, &state.config.sidecar.worker);
                worker.is_file().then(|| worker.display().to_string())
            }),
            sidecar_args: crate::runtime::configured_sidecar_args(&paths.root, &state.config),
            aria2_program: Some(std::env::var("XARCHIVE_ARIA2_PROGRAM").ok().unwrap_or_else(
                || {
                    crate::portable::resolve_config_path(&paths.root, &state.config.sidecar.aria2)
                        .display()
                        .to_string()
                },
            )),
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

fn copy_directory_contents(source: &Path, target: &Path) -> Result<(), String> {
    let metadata =
        fs::metadata(source).map_err(|error| format!("无法读取 Extension 目录：{error}"))?;
    if !metadata.is_dir() {
        return Err("Extension 导入路径必须是目录".to_owned());
    }
    fs::create_dir_all(target).map_err(|error| format!("无法创建临时 Extension 目录：{error}"))?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let destination = target.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            copy_directory_contents(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination).map_err(|error| error.to_string())?;
        } else {
            return Err("Extension 目录包含不支持的特殊文件".to_owned());
        }
    }
    Ok(())
}

fn validate_extension_directory(directory: &Path) -> Result<(String, String), String> {
    let manifest_path = directory.join("manifest.json");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("无法读取 manifest.json：{error}"))?;
    let value: serde_json::Value = serde_json::from_str(&manifest)
        .map_err(|error| format!("manifest.json 格式错误：{error}"))?;
    if value
        .get("manifest_version")
        .and_then(|value| value.as_u64())
        != Some(3)
        || value.get("name").and_then(|value| value.as_str()).is_none()
        || value
            .get("version")
            .and_then(|value| value.as_str())
            .is_none()
    {
        return Err("Extension manifest 必须包含 Manifest V3、name 和 version".to_owned());
    }
    for required in ["src/background.js", "src/content.js"] {
        if !directory.join(required).is_file() {
            return Err(format!("Extension 缺少必需文件：{required}"));
        }
    }
    Ok((
        value["version"].as_str().unwrap_or_default().to_owned(),
        sha256_directory_marker(directory)?,
    ))
}

fn sha256_directory_marker(directory: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(directory, directory, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for (relative, path) in files {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(fs::read(path).map_err(|error| error.to_string())?);
        hasher.update([0]);
    }
    Ok(format!("{0:x}", hasher.finalize()))
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
            files.push((relative.to_string_lossy().replace('\\', "/"), path));
        }
    }
    Ok(())
}

fn extension_status_from_state(state: &RuntimeState) -> Result<ExtensionStatus, String> {
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    let directory =
        crate::portable::resolve_config_path(&paths.root, &state.config.extension.directory);
    let files_ready = [
        directory.join("manifest.json"),
        directory.join("src").join("background.js"),
        directory.join("src").join("content.js"),
    ]
    .iter()
    .all(|path| path.is_file());
    #[cfg(windows)]
    let (browser_connection, native_host, message) = {
        let host_executable = paths
            .root
            .join("native-host")
            .join("xarchive-native-host.exe");
        let host_registered = crate::windows_transport::native_host_registered()?;
        let browser_connection = state
            .transport_server
            .as_ref()
            .map(|server| server.session.browser_connection().to_owned())
            .unwrap_or_else(|| "error".to_owned());
        let native_host = if !host_executable.is_file() {
            "missing"
        } else if host_registered {
            "registered"
        } else {
            "not_registered"
        };
        let message = if !files_ready {
            "未找到完整的 Extension 文件，请导入本地目录。".to_owned()
        } else if !host_executable.is_file() {
            "Extension 文件已就绪；当前包缺少 Native Host，请使用 Full package。".to_owned()
        } else if !host_registered {
            "Extension 和 Native Host 文件已就绪；Native Host 尚未注册到当前用户的 Edge/Chrome。"
                .to_owned()
        } else if let Some(error) = state.transport_error.as_deref() {
            format!("Native Host 已注册，但 Desktop Named Pipe 服务未启动：{error}")
        } else if browser_connection == "connected" {
            "最近 30 秒内收到 Native Host 请求；Desktop transport 正常。".to_owned()
        } else if browser_connection == "disconnected" {
            "曾收到 Native Host 请求；目前没有活动请求，请检查浏览器扩展或重启 Desktop。".to_owned()
        } else {
            "文件已就绪且 Native Host 已注册；Desktop 尚未观察到浏览器连接。".to_owned()
        };
        (browser_connection, native_host.to_owned(), message)
    };
    #[cfg(not(windows))]
    let (browser_connection, native_host, message) = (
        if files_ready {
            "not_loaded".to_owned()
        } else {
            "missing".to_owned()
        },
        if !files_ready {
            "missing".to_owned()
        } else {
            "not_available".to_owned()
        },
        if files_ready {
            "扩展文件已就绪；浏览器尚未加载，Native Host 注册和连接需要在目标环境验证。".to_owned()
        } else {
            "未找到完整的 Extension 文件，请导入本地目录。".to_owned()
        },
    );
    Ok(ExtensionStatus {
        files_ready,
        directory: directory.display().to_string(),
        browser_connection,
        native_host,
        message,
        source: state.config.extension.source.clone(),
        version: state.config.extension.version.clone(),
    })
}

#[tauri::command]
pub(crate) fn register_native_host(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ExtensionStatus, String> {
    #[cfg(windows)]
    {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        crate::windows_transport::register_or_repair(&state.portable_root)?;
        extension_status_from_state(&state)
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        Err("Native Host registration is only available on Windows".to_owned())
    }
}

#[tauri::command]
pub(crate) fn unregister_native_host(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ExtensionStatus, String> {
    #[cfg(windows)]
    {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        crate::windows_transport::unregister()?;
        extension_status_from_state(&state)
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        Err("Native Host registration is only available on Windows".to_owned())
    }
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
pub(crate) fn import_extension_directory(
    state: State<'_, Mutex<RuntimeState>>,
    source: String,
) -> Result<ExtensionStatus, String> {
    let source = PathBuf::from(source.trim());
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    let target =
        crate::portable::resolve_config_path(&paths.root, &state.config.extension.directory);
    if source == target {
        return Err("导入源不能是 XArchive 当前 Extension 目录".to_owned());
    }
    let temporary = paths.cache_dir.join(format!(
        "extension-import-{}",
        crate::runtime::timestamp_marker()
    ));
    let _ = fs::remove_dir_all(&temporary);
    copy_directory_contents(&source, &temporary)?;
    let (version, sha256) = validate_extension_directory(&temporary)?;
    let backup = paths.cache_dir.join("extension-backup");
    let _ = fs::remove_dir_all(&backup);
    if target.exists() {
        fs::rename(&target, &backup).map_err(|error| format!("无法备份现有 Extension：{error}"))?;
    }
    if let Err(error) = fs::rename(&temporary, &target) {
        if backup.exists() {
            let _ = fs::rename(&backup, &target);
        }
        return Err(format!("无法安装 Extension：{error}"));
    }
    let _ = fs::remove_dir_all(&backup);
    state.config.extension.source = "imported".to_owned();
    state.config.extension.version = Some(version);
    state.config.extension.sha256 = Some(sha256);
    state.config.save(&paths)?;
    extension_status_from_state(&state)
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
    extension_status_from_state(&state)
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

/// Submit one archive Job and schedule it on the real executor worker.
///
/// RuntimeState is used only to obtain immutable paths and the executor
/// handle. The command returns the initial Job snapshot immediately; the
/// executor opens its own persistence/resource context for long-running
/// SQLite, FileStore, Sidecar and ArchiveService I/O.
#[tauri::command]
pub(crate) fn submit_executor_job(
    state: State<'_, Mutex<RuntimeState>>,
    request: ArchiveTweetRequest,
) -> Result<ExecutorSubmitResponse, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        if state.download_setup_required {
            return Err("download directory setup is required before archiving".to_owned());
        }
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
    let prepared = ArchiveJobSubmissionAdapter
        .prepare(&request, &timestamp)
        .map_err(executor_error)?;
    let result = match service.submit_and_schedule_persisted(
        &mut persistence,
        prepared,
        database_path.clone(),
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
    Ok(ExecutorSubmitResponse {
        job_id: result.job.job_id,
        tweet_id: result.job.tweet_id,
        state: result.job.state.as_str().to_owned(),
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
