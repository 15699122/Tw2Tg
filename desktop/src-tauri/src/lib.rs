use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};
use xarchive_download::{DownloadRouter, GalleryDlFailure};
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{ArchiveService, Database, FileStore, JobSummary};

const DEFAULT_ARCHIVE_ROOT: &str = "X-Archive";

#[derive(Debug, Clone, Serialize)]
pub struct Aria2Release {
    pub version: &'static str,
    pub asset_name: &'static str,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Aria2Installation {
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Aria2DownloadResult {
    pub version: String,
    pub path: String,
    pub sha256: String,
}

const ARIA2_RELEASES: &[Aria2Release] = &[
    Aria2Release {
        version: "1.37.0",
        asset_name: "aria2-1.37.0-win-64bit-build1.zip",
        sha256: "67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288",
    },
    Aria2Release {
        version: "1.36.0",
        asset_name: "aria2-1.36.0-win-64bit-build1.zip",
        sha256: "C82DF5415125B438D72443923FEA7F5F9FDA1F326D0DBFAC6AAB16D58DBB7BF0",
    },
];

fn aria2_executable_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["aria2c.exe", "aria2c"]
    } else {
        &["aria2c"]
    }
}

fn executable_version(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("failed to run aria2c: {error}"))?;
    if !output.status.success() {
        return Err(format!("aria2c exited with status {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| {
            line.split_whitespace()
                .find(|part| {
                    part.chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_digit())
                })
                .map(str::to_owned)
        })
        .ok_or_else(|| "aria2c version was not found in --version output".to_owned())
}

fn candidate_aria2_paths(extra_directories: &[PathBuf]) -> Vec<(PathBuf, &'static str)> {
    let mut candidates = Vec::new();
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(directory) = current_exe.parent()
    {
        for name in aria2_executable_names() {
            candidates.push((directory.join(name), "program_directory"));
            candidates.push((directory.join("bin").join(name), "program_bin"));
        }
    }
    for directory in extra_directories {
        for name in aria2_executable_names() {
            candidates.push((directory.join(name), "program_data"));
            candidates.push((directory.join("bin").join(name), "program_data_bin"));
        }
        if let Ok(versions) = fs::read_dir(directory) {
            for version in versions.flatten() {
                let version_directory = version.path();
                if !version_directory.is_dir() {
                    continue;
                }
                for name in aria2_executable_names() {
                    candidates.push((version_directory.join(name), "program_data_version"));
                    candidates.push((
                        version_directory.join("bin").join(name),
                        "program_data_version_bin",
                    ));
                }
                if let Ok(build_directories) = fs::read_dir(&version_directory) {
                    for build in build_directories.flatten() {
                        if !build.path().is_dir() {
                            continue;
                        }
                        for name in aria2_executable_names() {
                            candidates.push((build.path().join(name), "program_data_build"));
                        }
                    }
                }
            }
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            for name in aria2_executable_names() {
                candidates.push((directory.join(name), "path"));
            }
        }
    }
    candidates
}

fn detect_aria2_installation(extra_directories: &[PathBuf]) -> Aria2Installation {
    for (path, source) in candidate_aria2_paths(extra_directories) {
        if !path.is_file() {
            continue;
        }
        match executable_version(&path) {
            Ok(version) => {
                return Aria2Installation {
                    found: true,
                    version: Some(version),
                    path: Some(path.display().to_string()),
                    source: Some(source.to_owned()),
                    error: None,
                };
            }
            Err(error) => {
                return Aria2Installation {
                    found: false,
                    version: None,
                    path: Some(path.display().to_string()),
                    source: Some(source.to_owned()),
                    error: Some(error),
                };
            }
        }
    }
    Aria2Installation {
        found: false,
        version: None,
        path: None,
        source: None,
        error: None,
    }
}

#[tauri::command]
fn detect_aria2(app: AppHandle) -> Aria2Installation {
    let app_data = app.path().app_data_dir().ok();
    let extra_directories = app_data
        .into_iter()
        .map(|path| path.join("tools").join("aria2"))
        .collect::<Vec<_>>();
    detect_aria2_installation(&extra_directories)
}

#[tauri::command]
fn list_aria2_releases() -> Vec<Aria2Release> {
    ARIA2_RELEASES.to_vec()
}

fn selected_aria2_release(version: &str) -> Result<&'static Aria2Release, String> {
    ARIA2_RELEASES
        .iter()
        .find(|release| release.version == version)
        .ok_or_else(|| "unsupported aria2 version".to_owned())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[tauri::command]
fn download_aria2(app: AppHandle, version: String) -> Result<Aria2DownloadResult, String> {
    if !cfg!(target_os = "windows") {
        return Err("aria2 Windows x64 downloads are only supported on Windows".to_owned());
    }
    let release = selected_aria2_release(&version)?;
    let url = format!(
        "https://github.com/aria2/aria2/releases/download/release-{}/{}",
        release.version, release.asset_name
    );
    let response = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| "failed to create download client".to_owned())?
        .get(url)
        .send()
        .map_err(|_| "failed to download aria2 release".to_owned())?;
    if !response.status().is_success() {
        return Err(format!(
            "aria2 release download returned HTTP {}",
            response.status()
        ));
    }
    let bytes = response
        .bytes()
        .map_err(|_| "failed to read aria2 release".to_owned())?;
    let actual_sha256 = sha256_hex(&bytes);
    if !actual_sha256.eq_ignore_ascii_case(release.sha256) {
        return Err("aria2 release SHA-256 mismatch".to_owned());
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to locate application data directory: {error}"))?;
    let install_root = app_data.join("tools").join("aria2").join(release.version);
    fs::create_dir_all(&install_root)
        .map_err(|error| format!("failed to create aria2 install directory: {error}"))?;
    let archive_path = install_root.join(release.asset_name);
    fs::write(&archive_path, &bytes)
        .map_err(|error| format!("failed to save aria2 release: {error}"))?;

    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force",
            &archive_path.to_string_lossy(),
            &install_root.to_string_lossy(),
        ])
        .status()
        .map_err(|error| format!("failed to extract aria2 release: {error}"))?;
    if !status.success() {
        return Err(format!(
            "aria2 release extraction failed with status {status}"
        ));
    }
    let executable = install_root
        .join(format!("aria2-{}-win-64bit-build1", release.version))
        .join("aria2c.exe");
    if !executable.is_file() {
        return Err("aria2 archive did not contain the expected aria2c.exe".to_owned());
    }
    let _ = fs::remove_file(&archive_path);
    Ok(Aria2DownloadResult {
        version: release.version.to_owned(),
        path: executable.display().to_string(),
        sha256: actual_sha256,
    })
}

pub struct RuntimeState {
    archive_root: PathBuf,
    database: Option<Database>,
    database_ready: bool,
    database_error: Option<String>,
    sidecar: Option<SidecarSupervisor>,
    sidecar_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveTweetRequest {
    pub tweet: xarchive_protocol::BrowserTweet,
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
}

fn timestamp_marker() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

struct SidecarArchiveResult {
    metadata: serde_json::Value,
    files: Vec<xarchive_protocol::DownloadFile>,
}

fn run_sidecar_download(
    supervisor: &mut SidecarSupervisor,
    job_id: &str,
    request_id: &str,
    url: &str,
    staging_dir: &Path,
    executable: Option<&str>,
    browser: Option<&str>,
    profile: Option<&str>,
) -> Result<SidecarArchiveResult, GalleryDlFailure> {
    let command = xarchive_protocol::SidecarCommand {
        protocol_version: xarchive_protocol::PROTOCOL_VERSION,
        request_id: request_id.to_owned(),
        cmd: xarchive_protocol::SidecarCommandType::Download,
        job_id: job_id.to_owned(),
        url: Some(url.to_owned()),
        staging_dir: Some(staging_dir.display().to_string()),
        executable: executable.map(str::to_owned),
        browser: browser.map(str::to_owned),
        profile: profile.map(str::to_owned),
    };
    let mut metadata = None;
    let mut files = None;
    supervisor
        .send(&command)
        .map_err(|error| GalleryDlFailure::new("SIDECAR_INTERNAL_ERROR", error.to_string()))?;
    let deadline = std::time::Instant::now() + Duration::from_secs(15 * 60);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(GalleryDlFailure::new(
                "DOWNLOAD_TIMEOUT",
                "sidecar download timed out",
            ));
        }
        let event = supervisor
            .recv_timeout(remaining.min(Duration::from_millis(250)))
            .map_err(|error| GalleryDlFailure::new("SIDECAR_INTERNAL_ERROR", error.to_string()))?;
        let Some(event) = event else {
            continue;
        };
        match event {
            xarchive_sidecar_supervisor::SupervisorEvent::Download(event)
                if event.job_id == job_id
                    && event
                        .request_id
                        .as_deref()
                        .is_none_or(|id| id == request_id) =>
            {
                match event.event {
                    xarchive_protocol::DownloadEventType::Metadata => {
                        metadata = event.data;
                    }
                    xarchive_protocol::DownloadEventType::File => {
                        let Some(path) = event.path else { continue };
                        files
                            .get_or_insert_with(Vec::new)
                            .push(xarchive_protocol::DownloadFile {
                                relative_path: path,
                                size_bytes: event.size_bytes.unwrap_or_default(),
                                media_type: event
                                    .media_type
                                    .unwrap_or_else(|| "unknown".to_owned()),
                                mime_type: event.mime_type,
                            });
                    }
                    xarchive_protocol::DownloadEventType::Complete => {
                        return Ok(SidecarArchiveResult {
                            metadata: metadata.ok_or_else(|| {
                                GalleryDlFailure::new(
                                    "METADATA_MISSING",
                                    "sidecar completed without metadata",
                                )
                            })?,
                            files: event.files.or(files).unwrap_or_default(),
                        });
                    }
                    xarchive_protocol::DownloadEventType::Failed => {
                        return Err(GalleryDlFailure::new(
                            event
                                .error_code
                                .unwrap_or_else(|| "EXTRACT_OR_DOWNLOAD_FAILED".to_owned()),
                            event
                                .error_message
                                .unwrap_or_else(|| "sidecar download failed".to_owned()),
                        ));
                    }
                    _ => {}
                }
            }
            xarchive_sidecar_supervisor::SupervisorEvent::Exited(result) => {
                return Err(GalleryDlFailure::new(
                    "SIDECAR_INTERNAL_ERROR",
                    format!("sidecar exited during download: {result:?}"),
                ));
            }
            xarchive_sidecar_supervisor::SupervisorEvent::ProtocolError { message, .. } => {
                return Err(GalleryDlFailure::new("SIDECAR_INTERNAL_ERROR", message));
            }
            xarchive_sidecar_supervisor::SupervisorEvent::Stderr(_)
            | xarchive_sidecar_supervisor::SupervisorEvent::Download(_) => {}
        }
    }
}

impl RuntimeState {
    fn initialize() -> Self {
        let archive_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(DEFAULT_ARCHIVE_ROOT);
        let database_path = archive_root.join("_database").join("archive.sqlite3");
        let database = (|| {
            FileStore::new(archive_root.clone()).ok()?;
            std::fs::create_dir_all(database_path.parent()?).ok()?;
            Database::open(&database_path).ok()
        })();
        let database_ready = database.is_some();
        let database_error = if database_ready {
            None
        } else {
            Some("failed to initialize archive database".to_owned())
        };

        Self {
            archive_root,
            database,
            database_ready,
            database_error,
            sidecar: None,
            sidecar_error: None,
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
    pub sidecar_error: Option<String>,
}

#[tauri::command]
fn get_app_status(state: State<'_, Mutex<RuntimeState>>) -> AppStatus {
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

fn parse_sidecar_args(raw: &str) -> Result<Vec<String>, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw)
        .map_err(|error| format!("XARCHIVE_SIDECAR_ARGS must be a JSON string array: {error}"))
}

fn refresh_sidecar_state(state: &mut RuntimeState) {
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
fn start_sidecar(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
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
fn stop_sidecar(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
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
            executable: None,
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
fn archive_tweet(
    state: State<'_, Mutex<RuntimeState>>,
    request: ArchiveTweetRequest,
) -> Result<JobSummary, String> {
    xarchive_protocol::BrowserRequest::ArchiveRequest {
        protocol_version: xarchive_protocol::PROTOCOL_VERSION,
        request_id: "archive-validation".to_owned(),
        tweet: request.tweet.clone(),
    }
    .validate()
    .map_err(|error| error.to_string())?;
    let now = timestamp_marker();
    let job_id = format!("archive-{}-{now}", request.tweet.tweet_id);
    let request_id = format!("desktop-{job_id}");

    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    refresh_sidecar_state(&mut state);
    let database = state
        .database
        .take()
        .ok_or_else(|| "archive database is not initialized".to_owned())?;
    let mut supervisor = match state.sidecar.take() {
        Some(supervisor) => supervisor,
        None => {
            state.database = Some(database);
            return Err("sidecar is not running".to_owned());
        }
    };

    // Insert tweet and create job
    let tweet_row_id = database
        .insert_tweet(
            &request.tweet.tweet_id,
            &request.tweet.url,
            &request.tweet.tweet_type,
            request.tweet.text.as_deref().unwrap_or_default(),
            &now,
        )
        .map_err(|error| error.to_string())?;
    let created = database
        .create_archive_job(&job_id, tweet_row_id, &now)
        .map_err(|error| error.to_string())?;
    if !created {
        // Job already exists - find the existing active job
        let existing = database
            .list_recent_jobs(100)
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|job| job.tweet_id == request.tweet.tweet_id && job.state.is_active())
            .ok_or_else(|| "active archive job already exists but could not be found".to_owned())?;
        // Return existing job summary but keep database for state
        state.database = Some(database);
        state.sidecar = Some(supervisor);
        return Ok(existing);
    }

    // Create staging directory and download
    let files = FileStore::new(state.archive_root.clone()).map_err(|error| error.to_string())?;
    let staging_dir = files
        .staging_dir(&job_id)
        .map_err(|error| error.to_string())?;
    let router = DownloadRouter::default();
    let mut sidecar_result: Option<SidecarArchiveResult> = None;
    let _download = router.execute(
        || {
            sidecar_result = Some(run_sidecar_download(
                &mut supervisor,
                &job_id,
                &request_id,
                &request.tweet.url,
                &staging_dir,
                request.executable.as_deref(),
                request.browser.as_deref(),
                request.profile.as_deref(),
            )?);
            Ok(())
        },
        None,
        |_request| Err(xarchive_download::DownloadError::InvalidAddUriRequest),
    );
    let archive_result = sidecar_result
        .take()
        .expect("sidecar download completed but result was lost");

    // Archive the results
    let mut archive = ArchiveService::new(database, files);
    let final_directory = PathBuf::from("archives").join(&request.tweet.tweet_id);
    archive
        .complete_sidecar_archive(
            &job_id,
            tweet_row_id,
            &archive_result.metadata,
            &archive_result.files,
            &final_directory,
            &now,
        )
        .map_err(|error| error.to_string())?;

    // Get the final job summary
    let job = archive
        .database
        .list_recent_jobs(100)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|job| job.job_id == job_id)
        .ok_or_else(|| "completed archive job could not be found".to_owned())?;

    state.database = Some(archive.database);
    state.sidecar = Some(supervisor);
    Ok(job)
}

#[tauri::command]
fn list_jobs(
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
fn get_archive_root(state: State<'_, Mutex<RuntimeState>>) -> String {
    state
        .lock()
        .expect("runtime state lock poisoned")
        .archive_root
        .display()
        .to_string()
}

#[tauri::command]
fn open_archive_folder(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let path = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?
        .archive_root
        .clone();
    let mut command = if cfg!(target_os = "windows") {
        let mut command = std::process::Command::new("explorer");
        command.arg(&path);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = std::process::Command::new("open");
        command.arg(&path);
        command
    } else {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(&path);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open archive folder: {error}"))
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
            get_runtime_health,
            start_sidecar,
            stop_sidecar,
            archive_tweet,
            list_jobs,
            open_archive_folder,
            detect_aria2,
            list_aria2_releases,
            download_aria2
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

    #[test]
    fn parses_sidecar_args_as_json_without_splitting_paths() {
        let args = parse_sidecar_args(r#"["-m","xarchive_downloader","C:\\Work Dir\\staging"]"#)
            .expect("valid sidecar args");
        assert_eq!(args[2], r#"C:\Work Dir\staging"#);
    }

    #[test]
    fn rejects_non_array_sidecar_args() {
        let error = parse_sidecar_args("--verbose").expect_err("invalid args");
        assert!(error.contains("JSON string array"));
    }

    #[test]
    fn exposes_verified_aria2_releases_and_sha256() {
        let releases = list_aria2_releases();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].version, "1.37.0");
        assert_eq!(
            releases[0].sha256,
            "67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288"
        );
        assert_eq!(
            sha256_hex(b"xarchive"),
            "5bb7d0a5d35ed6fb314afcc4931bc18b03262864f955ad678fb662e2239214fa"
        );
    }

    #[test]
    fn rejects_unsupported_aria2_versions() {
        assert!(selected_aria2_release("9.99.9").is_err());
    }
}
