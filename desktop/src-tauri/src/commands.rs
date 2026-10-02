use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, State};
use xarchive_sidecar_supervisor::SidecarSupervisor;

use crate::archive::upsert_browser_user;
use crate::batch::{BatchFilters, spawn_account_batch, spawn_batch_dispatch};
use crate::components::ComponentBootstrapStatus;
use crate::config::{LogLevel, MAX_LOG_MAX_FILES, MIN_LOG_MAX_FILES, ProxyMode};
use crate::executor::CancellationToken;
use crate::executor::{ArchiveJobSubmissionAdapter, ExecutorError, JobSnapshot};
use crate::portable::system_download_archive_directory;
use crate::proxy::ProxyHttpClient;
use crate::websocket_transport::WebSocketDiagnosticSnapshot;
use crate::{ArchiveTweetRequest, RuntimeState};
use xarchive_core::ProxyDecision;
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

fn redact_job_errors(jobs: &mut [xarchive_storage::JobSummary], secrets: &[String]) {
    for job in jobs {
        job.last_error_message = job
            .last_error_message
            .as_deref()
            .map(|message| crate::logging::redact_line(message, secrets));
    }
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

#[cfg(test)]
mod frontend_diagnostic_tests {
    use super::{bounded_diagnostic, redact_job_errors};
    use xarchive_core::JobState;
    use xarchive_storage::JobSummary;
    #[test]
    fn bounds_frontend_diagnostic_by_characters() {
        assert_eq!(bounded_diagnostic("abcdef", 3), "abc");
        assert_eq!(bounded_diagnostic("白屏诊断", 2), "白屏");
    }

    #[test]
    fn redacts_job_failure_urls_before_frontend_projection() {
        let mut jobs = vec![JobSummary {
            job_id: "job-1".to_owned(),
            tweet_id: "1".to_owned(),
            tweet_type: "post".to_owned(),
            state: JobState::Failed,
            created_at: "now".to_owned(),
            updated_at: "now".to_owned(),
            last_error_code: Some("ARCHIVE_DOWNLOAD_FAILED".to_owned()),
            last_error_message: Some("http://u:p@cdn.example/file?token=secret&id=1".to_owned()),
        }];
        redact_job_errors(&mut jobs, &[]);
        assert_eq!(
            jobs[0].last_error_message.as_deref(),
            Some("http://[REDACTED]@cdn.example/file?token=[REDACTED]&id=1")
        );
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

/// The proxy settings the Settings page reads and writes.
///
/// The stored value is never returned. `proxy` comes back empty unless the user
/// is editing it, so the frontend can never read back a stored credential.
#[derive(Debug, Serialize)]
pub struct NetworkSettings {
    pub proxy_mode: String,
    /// Empty when nothing is stored, or when a value is stored but not in
    /// effect. The UI shows the redacted summary from `proxy_summary`.
    pub proxy: String,
    pub proxy_summary: Option<String>,
    pub proxy_configured: bool,
    pub proxy_active: bool,
    pub supported_modes: Vec<String>,
    pub system_proxy_note: String,
    pub system_proxy_supported: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSettingsInput {
    pub proxy_mode: String,
    #[serde(default)]
    pub proxy: Option<String>,
}

#[tauri::command]
pub(crate) fn get_network_settings(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<NetworkSettings, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    Ok(network_settings(&state))
}

fn network_settings(state: &RuntimeState) -> NetworkSettings {
    let diagnostics = state.config.network.diagnostics();
    NetworkSettings {
        proxy_mode: diagnostics.proxy_mode,
        proxy: String::new(),
        proxy_summary: diagnostics.proxy,
        proxy_configured: diagnostics.proxy_configured,
        proxy_active: diagnostics.proxy_active,
        supported_modes: vec![
            ProxyMode::System.as_str().to_owned(),
            ProxyMode::Direct.as_str().to_owned(),
            ProxyMode::Manual.as_str().to_owned(),
        ],
        system_proxy_note: system_proxy_note(),
        system_proxy_supported: system_proxy_supported(),
    }
}

/// What `System` actually resolves to on this platform.
///
/// The Settings page shows this rather than promising platform-wide system proxy
/// support, because the resolver only reads the process environment until the
/// Windows native resolver lands.
fn system_proxy_note() -> String {
    if cfg!(target_os = "windows") {
        "System currently follows the environment and the Windows registry manual proxy. \
         PAC and WPAD are resolved per URL once the native resolver is enabled."
            .to_owned()
    } else {
        "System follows the proxy environment variables of the launching process.".to_owned()
    }
}

/// Whether this platform can resolve a system proxy beyond the environment.
fn system_proxy_supported() -> bool {
    // Batch B replaces this with the WinHTTP resolver's capability. Reporting
    // false today keeps the Settings page from overstating what is delivered.
    false
}

#[tauri::command]
pub(crate) fn save_network_settings(
    state: State<'_, Mutex<RuntimeState>>,
    settings: NetworkSettingsInput,
) -> Result<NetworkSettings, String> {
    let mode = ProxyMode::parse(&settings.proxy_mode)?;
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let requested = settings
        .proxy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    match mode {
        // Switching away from `Manual` keeps the stored value, so returning to
        // `Manual` later does not require the user to retype a credential.
        ProxyMode::Manual => {
            let value = requested
                .or_else(|| state.config.network.normalized_proxy())
                .ok_or_else(|| "a proxy address is required when the mode is manual".to_owned())?;
            state.config.network.proxy = Some(value);
        }
        _ => {
            if let Some(value) = requested {
                state.config.network.proxy = Some(value);
            }
        }
    }
    state.config.network.proxy_mode = mode;
    // The mode is now explicit, so a later load must not re-run the migration.
    state.config.network.proxy_mode_declared = true;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    // Only the redacted summary may be logged: the configured value itself can
    // carry credentials and must never reach the log file.
    state.debug(
        "network",
        &format!(
            "applying mode={} summary={:?}",
            state.config.network.proxy_mode.as_str(),
            state.config.network.diagnostics(),
        ),
    );
    state.config.save(&paths)?;
    // A saved mode that the runtime then refuses to apply must not be reported
    // as success, so this failure surfaces instead of leaving the old route
    // active behind a "saved" message.
    apply_network_config(&mut state)?;
    // The redaction list now has to cover the proxy that was just stored, so the
    // log file is rebuilt the same way the logging settings command rebuilds it.
    state.log_file = crate::logging::LogFile::open(
        &state.logs_root,
        state.config.logging.effective_level(),
        state.config.logging.max_files,
    )
    .map(|log| log.with_secrets(state.config.log_secrets()))
    .ok();
    Ok(network_settings(&state))
}

#[derive(Debug, Serialize)]
pub struct ProxyDiagnostic {
    pub mode: String,
    pub url: String,
    /// `proxy`, `direct`, `failed`, or `unsupported`.
    pub route: String,
    /// The redacted proxy. Never contains credentials.
    pub proxy: Option<String>,
    pub message: String,
}

/// Resolve the route for one URL without sending a request.
///
/// This is a diagnostic, not a connectivity test: it reports what the resolver
/// decided, so the Settings page can show a real result rather than a claim.
/// Resolution runs on a worker thread because a Windows PAC or WPAD lookup can
/// block, and the UI thread must stay responsive.
#[tauri::command]
pub(crate) fn inspect_proxy_route(
    state: State<'_, Mutex<RuntimeState>>,
    url: String,
) -> Result<ProxyDiagnostic, String> {
    let target = url.trim().to_owned();
    if target.is_empty() {
        return Err("a URL is required".to_owned());
    }
    let client = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        ProxyHttpClient::from_config(&state.config.network)
    };
    let resolved_url = target.clone();
    let mode = client.mode();
    // Resolution is moved off the calling thread because a platform resolver may
    // block. The UI thread must never wait on a PAC or WPAD lookup.
    let decision = std::thread::spawn(move || client.decision_for(&resolved_url))
        .join()
        .map_err(|_| "proxy resolution thread panicked".to_owned())?;

    Ok(ProxyDiagnostic {
        mode: mode.as_str().to_owned(),
        url: target,
        route: route_name(&decision).to_owned(),
        proxy: decision.redacted_proxy(),
        message: route_message(&decision),
    })
}

fn route_name(decision: &ProxyDecision) -> &'static str {
    match decision {
        ProxyDecision::Direct => "direct",
        ProxyDecision::Proxy(_) => "proxy",
        ProxyDecision::ResolutionFailed(_) => "failed",
        ProxyDecision::Unsupported(_) => "unsupported",
    }
}

fn route_message(decision: &ProxyDecision) -> String {
    match decision {
        ProxyDecision::Direct => "This destination connects without a proxy.".to_owned(),
        ProxyDecision::Proxy(_) => {
            "This destination connects through the resolved proxy.".to_owned()
        }
        ProxyDecision::ResolutionFailed(reason) => {
            format!("The system proxy could not be resolved: {reason}. No request was sent.")
        }
        ProxyDecision::Unsupported(reason) => {
            format!("The selected mode cannot be applied: {reason}")
        }
    }
}

/// Push a saved network configuration into the running runtime.
///
/// The executor holds a copy of the configuration, so a mode change only takes
/// effect once this runs. Reusing `replace_executor` keeps the Browser transport
/// on the same service generation instead of pointing it at a closed executor.
fn apply_network_config(state: &mut RuntimeState) -> Result<(), String> {
    let database_path = state.executor.database_path().to_owned();
    let config = crate::runtime::executor_config(
        &state.portable_root,
        &state.config,
        database_path,
        state.cache_root.join("staging"),
        state.download_root.clone(),
    );
    state.replace_executor(config)
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
    pub websocket_port: Option<u16>,
    pub websocket_runtime_instance_id: Option<String>,
    pub websocket_authenticated: bool,
    pub websocket_connection: String,
    pub websocket_diagnostics: WebSocketDiagnosticSnapshot,
    pub websocket_error: Option<String>,
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
        logging_level: state.config.logging.effective_level().as_str().to_owned(),
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
    let (program, mut args) = sidecar_configuration()?;
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    if state.sidecar.is_some() {
        return Ok("ready".to_owned());
    }
    state.sidecar_error = None;
    args.extend(state.config.network.sidecar_args());
    let env = state.config.network.sidecar_env();
    // A `Direct` mode also has to drop the proxy variables the launching shell
    // exported, otherwise the Sidecar re-introduces the proxy the user turned
    // off through gallery-dl.
    let env_remove = state.config.network.sidecar_env_remove();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    state.debug(
        "sidecar",
        &format!(
            "starting program={} args={} proxy_mode={} env={} env_remove={:?}",
            program,
            arg_refs.join(" "),
            state.config.network.proxy_mode.as_str(),
            env.len(),
            env_remove,
        ),
    );
    match SidecarSupervisor::spawn_ready_v2_with_environment(
        &program,
        &arg_refs,
        &xarchive_sidecar_supervisor::ChildProcessEnvironment::new(env, env_remove),
        Duration::from_secs(5),
    ) {
        Ok(supervisor) => {
            state.sidecar = Some(supervisor);
            state.debug("sidecar", "handshake ready");
            Ok("ready".to_owned())
        }
        Err(error) => {
            let message = error.to_string();
            state.sidecar_error = Some(message.clone());
            state.error("sidecar", &format!("start failed: {message}"));
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

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateAccountBatchRequest {
    pub username: String,
    pub profile_url: String,
    pub browser: Option<String>,
    pub profile: Option<String>,
    #[serde(default)]
    pub filters: BatchFilters,
}

fn open_batch_database(state: &RuntimeState) -> Result<Database, String> {
    Database::open(state.executor.database_path()).map_err(|error| error.to_string())
}

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
    database
        .create_account_batch(
            &id,
            &username,
            request.profile_url.trim(),
            request.browser.as_deref(),
            request.profile.as_deref(),
            &filters_json,
        )
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
    let (
        websocket_port,
        websocket_runtime_instance_id,
        websocket_authenticated,
        websocket_connection,
        websocket_diagnostics,
    ) = state
        .websocket_server
        .as_ref()
        .map(|server| {
            (
                Some(server.port()),
                Some(server.runtime_instance_id().to_owned()),
                server.session.authenticated(),
                server.session.browser_connection().to_owned(),
                server.session.diagnostic_snapshot(),
            )
        })
        .unwrap_or_else(|| {
            (
                None,
                None,
                false,
                "not_started".to_owned(),
                crate::websocket_transport::WebSocketDiagnosticSnapshot {
                    accepted: 0,
                    handshake_failed: 0,
                    auth_read_failed: 0,
                    auth_received: 0,
                    auth_succeeded: 0,
                    auth_failed: 0,
                    auth_response_failed: 0,
                    close_before_auth: 0,
                    close_after_auth: 0,
                    last_request_age_seconds: None,
                },
            )
        });
    Ok(ExtensionStatus {
        files_ready,
        directory: directory.display().to_string(),
        browser_connection,
        native_host,
        message,
        source: state.config.extension.source.clone(),
        version: state.config.extension.version.clone(),
        websocket_port,
        websocket_runtime_instance_id,
        websocket_authenticated,
        websocket_connection,
        websocket_diagnostics,
        websocket_error: state.websocket_error.clone(),
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
    // The user picked a level, so it is stored as an explicit choice and keeps
    // winning over the build channel default on every later start.
    state.config.logging.level = Some(settings.logging_level);
    state.config.logging.max_files = settings.max_log_files;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.debug(
        "logging",
        &format!(
            "user selected level={} (channel={} default={}) max_files={}",
            settings.logging_level.as_str(),
            crate::build_channel::release_channel().as_str(),
            crate::build_channel::channel_default_log_level().as_str(),
            settings.max_log_files,
        ),
    );
    state.config.save(&paths)?;
    state.log_file = crate::logging::LogFile::open(
        &state.logs_root,
        settings.logging_level,
        settings.max_log_files,
    )
    .map(|log| log.with_secrets(state.config.log_secrets()))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn network_config(mode: ProxyMode, proxy: Option<&str>) -> crate::config::NetworkConfig {
        crate::config::NetworkConfig {
            proxy_mode: mode,
            proxy_mode_declared: true,
            proxy: proxy.map(str::to_owned),
            ..crate::config::NetworkConfig::default()
        }
    }

    fn settings_for(config: &crate::config::NetworkConfig) -> NetworkSettings {
        let diagnostics = config.diagnostics();
        NetworkSettings {
            proxy_mode: diagnostics.proxy_mode,
            proxy: String::new(),
            proxy_summary: diagnostics.proxy,
            proxy_configured: diagnostics.proxy_configured,
            proxy_active: diagnostics.proxy_active,
            supported_modes: vec![
                "system".to_owned(),
                "direct".to_owned(),
                "manual".to_owned(),
            ],
            system_proxy_note: system_proxy_note(),
            system_proxy_supported: system_proxy_supported(),
        }
    }

    #[test]
    fn the_settings_view_never_returns_the_stored_credential() {
        let config = network_config(
            ProxyMode::Manual,
            Some("http://alice:s3cret@proxy.example:8080"),
        );
        let settings = settings_for(&config);
        assert!(
            settings.proxy.is_empty(),
            "the stored value must never be returned to the frontend"
        );
        assert_eq!(
            settings.proxy_summary.as_deref(),
            Some("http://[REDACTED]@proxy.example:8080")
        );
        assert!(settings.proxy_configured);
        assert!(settings.proxy_active);
        let rendered = serde_json::to_string(&settings).expect("settings");
        assert!(
            !rendered.contains("s3cret"),
            "credentials leaked to the frontend: {rendered}"
        );
    }

    #[test]
    fn an_inactive_value_is_reported_as_stored_but_not_in_effect() {
        for mode in [ProxyMode::System, ProxyMode::Direct] {
            let config = network_config(mode, Some("http://alice:s3cret@proxy.example:8080"));
            let settings = settings_for(&config);
            assert!(
                settings.proxy_configured,
                "{mode:?} must still report a stored value"
            );
            assert!(
                !settings.proxy_active,
                "{mode:?} must not claim the value is in effect"
            );
            assert!(
                settings.proxy_summary.is_none(),
                "{mode:?} must not show a proxy summary"
            );
        }
    }

    #[test]
    fn the_settings_view_does_not_overstate_system_proxy_support() {
        assert!(
            !system_proxy_supported(),
            "the native resolver is Batch B; claiming support now would be false"
        );
        assert!(!system_proxy_note().is_empty());
    }

    #[test]
    fn a_route_is_described_without_exposing_credentials() {
        let decision = ProxyDecision::Proxy("http://alice:s3cret@proxy.example:8080".to_owned());
        assert_eq!(route_name(&decision), "proxy");
        let message = route_message(&decision);
        assert!(!message.contains("s3cret"));
        assert!(!message.contains("proxy.example"));

        assert_eq!(route_name(&ProxyDecision::Direct), "direct");
        assert_eq!(
            route_name(&ProxyDecision::ResolutionFailed("x".to_owned())),
            "failed"
        );
        assert_eq!(
            route_name(&ProxyDecision::Unsupported("y".to_owned())),
            "unsupported"
        );
    }

    #[test]
    fn a_failed_resolution_is_reported_as_a_failure_and_not_as_direct() {
        let message = route_message(&ProxyDecision::ResolutionFailed(
            "WPAD discovery timed out".to_owned(),
        ));
        assert!(message.contains("WPAD discovery timed out"));
        assert!(
            message.contains("No request was sent"),
            "a failure must state that nothing was sent: {message}"
        );
    }

    #[test]
    fn an_unsupported_mode_is_reported_as_unsupported() {
        let message = route_message(&ProxyDecision::Unsupported(
            "manual mode is selected but no proxy is configured".to_owned(),
        ));
        assert!(message.contains("cannot be applied"));
        assert!(!message.contains("s3cret"));
    }
}
