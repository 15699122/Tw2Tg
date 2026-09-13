mod aria2;

use aria2::{detect_aria2, download_aria2, list_aria2_releases};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::State;
use xarchive_core::{JobEvent, JobState};
use xarchive_download::{DownloadRouter, GalleryDlFailure};
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{ArchiveService, Database, FileStore, JobSummary, SidecarArchiveRequest};

const DEFAULT_ARCHIVE_ROOT: &str = "X-Archive";

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

/// Persist the browser-supplied author link for an archived tweet.
///
/// Browser payloads do not carry a stable numeric X user id, so the tweet id
/// is used as the directory anchor. When no username is present there is no
/// durable user row to link, and `Ok(None)` preserves the existing nullable
/// `tweets.user_id` behavior.
fn upsert_browser_user(
    database: &mut Database,
    tweet_row_id: i64,
    tweet: &xarchive_protocol::BrowserTweet,
    now: &str,
) -> Result<Option<i64>, xarchive_storage::StorageError> {
    let username = tweet
        .username
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let Some(username) = username else {
        return Ok(None);
    };
    let user_row_id = database.upsert_user(
        &format!("browser-{}", tweet.tweet_id),
        Some(username),
        tweet.display_name.as_deref(),
        now,
    )?;
    database.set_tweet_user(tweet_row_id, user_row_id)?;
    Ok(Some(user_row_id))
}

struct SidecarArchiveResult {
    metadata: serde_json::Value,
    files: Vec<xarchive_protocol::DownloadFile>,
}

struct SidecarDownloadRequest {
    job_id: String,
    request_id: String,
    url: String,
    staging_dir: PathBuf,
    browser: Option<String>,
    profile: Option<String>,
}

/// Merge browser-DOM relationship data into the Sidecar metadata payload.
///
/// gallery-dl's info.json rarely carries structured reply/quote references,
/// while the Browser extension extracts them directly from the DOM. The
/// browser-supplied data only fills gaps in the Sidecar payload and never
/// overwrites metadata the Sidecar already provided. A Sidecar `tweet_type`
/// of `post` is upgraded to the browser-observed `reply`/`quote` because the
/// DOM social context is the more reliable signal for the tweet kind.
fn merge_browser_relationships(
    metadata: &mut serde_json::Value,
    tweet: &xarchive_protocol::BrowserTweet,
) {
    let Some(object) = metadata.as_object_mut() else {
        return;
    };
    if let Some(reply_to) = tweet
        .reply_to
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let missing = match object.get("reply_to") {
            None | Some(serde_json::Value::Null) => true,
            Some(serde_json::Value::String(value)) => value.trim().is_empty(),
            _ => false,
        };
        if missing {
            object.insert(
                "reply_to".to_owned(),
                serde_json::Value::String(reply_to.to_owned()),
            );
        }
    }
    if let Some(quoted) = &tweet.quoted_tweet {
        let missing = !matches!(
            object.get("quoted_tweet"),
            Some(serde_json::Value::Object(_))
        );
        if missing {
            object.insert(
                "quoted_tweet".to_owned(),
                serde_json::json!({
                    "tweet_id": quoted.tweet_id,
                    "url": quoted.url,
                    "username": quoted.username,
                    "display_name": quoted.display_name,
                    "text": quoted.text,
                    "created_at": quoted.created_at,
                    "tweet_type": quoted.tweet_type,
                }),
            );
        }
    }
    if object.get("tweet_type").and_then(serde_json::Value::as_str) == Some("post")
        && tweet.tweet_type != "post"
    {
        object.insert(
            "tweet_type".to_owned(),
            serde_json::Value::String(tweet.tweet_type.clone()),
        );
    }
}

fn run_sidecar_download(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
) -> Result<SidecarArchiveResult, GalleryDlFailure> {
    let command = xarchive_protocol::SidecarCommand {
        protocol_version: xarchive_protocol::PROTOCOL_VERSION,
        request_id: request.request_id.clone(),
        cmd: xarchive_protocol::SidecarCommandType::Download,
        job_id: request.job_id.clone(),
        url: Some(request.url.clone()),
        staging_dir: Some(request.staging_dir.display().to_string()),
        browser: request.browser.clone(),
        profile: request.profile.clone(),
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
                if event.job_id == request.job_id
                    && event
                        .request_id
                        .as_deref()
                        .is_none_or(|id| id == request.request_id) =>
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
                        let error_code = event
                            .error_code
                            .as_deref()
                            .unwrap_or("EXTRACT_OR_DOWNLOAD_FAILED");
                        return Err(GalleryDlFailure::new(
                            error_code,
                            safe_sidecar_error_message(error_code),
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
            xarchive_sidecar_supervisor::SupervisorEvent::ProtocolError { .. } => {
                return Err(GalleryDlFailure::new(
                    "SIDECAR_INTERNAL_ERROR",
                    "sidecar protocol error",
                ));
            }
            xarchive_sidecar_supervisor::SupervisorEvent::Stderr(_)
            | xarchive_sidecar_supervisor::SupervisorEvent::Download(_) => {}
        }
    }
}

fn safe_sidecar_error_message(code: &str) -> &'static str {
    match code {
        "AUTH_REQUIRED" => "X authentication is required",
        "RATE_LIMITED" => "X temporarily rate-limited the request",
        "TWEET_NOT_FOUND" => "the requested X post was not found",
        "DOWNLOAD_TIMEOUT" => "the download timed out",
        "METADATA_MISSING" => "the downloader did not return metadata",
        "INVALID_DOWNLOAD_COMMAND" => "the downloader command was invalid",
        "SIDECAR_DEPENDENCY_MISSING" => "the downloader dependency is unavailable",
        "SIDECAR_INTERNAL_ERROR" => "the downloader encountered an internal error",
        _ => "the X post could not be archived",
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
    let mut database = state
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
    let _browser_user_row_id =
        upsert_browser_user(&mut database, tweet_row_id, &request.tweet, &now)
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

    database
        .record_event(
            &job_id,
            &JobEvent::Created {
                job_id: job_id.clone(),
            },
            &now,
        )
        .map_err(|error| error.to_string())?;

    // Create staging directory and download
    let files = FileStore::new(state.archive_root.clone()).map_err(|error| error.to_string())?;
    let staging_dir = files
        .staging_dir(&job_id)
        .map_err(|error| error.to_string())?;
    let sidecar_request = SidecarDownloadRequest {
        job_id: job_id.clone(),
        request_id: request_id.clone(),
        url: request.tweet.url.clone(),
        staging_dir,
        browser: request.browser.clone(),
        profile: request.profile.clone(),
    };
    let router = DownloadRouter::default();
    database
        .record_event(
            &job_id,
            &JobEvent::DownloadStarted {
                backend: "gallery-dl".to_owned(),
            },
            &now,
        )
        .map_err(|error| error.to_string())?;
    let mut sidecar_result: Option<SidecarArchiveResult> = None;
    let download = router.execute(
        || {
            sidecar_result = Some(run_sidecar_download(&mut supervisor, &sidecar_request)?);
            Ok(())
        },
        None,
        |_request| Err(xarchive_download::DownloadError::InvalidAddUriRequest),
    );
    let archive_result = match download {
        Ok(_) => sidecar_result
            .take()
            .ok_or_else(|| "sidecar download completed but result was lost".to_owned())?,
        Err(error) => {
            let (next_state, error_code) = match &error {
                xarchive_download::DownloadRouterError::GalleryDl(gallery) => {
                    let next_state = if gallery.code == "AUTH_REQUIRED" {
                        JobState::AuthRequired
                    } else {
                        JobState::Failed
                    };
                    (next_state, gallery.code.as_str())
                }
                xarchive_download::DownloadRouterError::GalleryDlThenAria2 { gallery, .. } => {
                    let next_state = if gallery.code == "AUTH_REQUIRED" {
                        JobState::AuthRequired
                    } else {
                        JobState::Failed
                    };
                    (next_state, gallery.code.as_str())
                }
                xarchive_download::DownloadRouterError::Aria2NotConfigured
                | xarchive_download::DownloadRouterError::Aria2(_) => {
                    (JobState::Failed, "ARIA2_FALLBACK_FAILED")
                }
            };
            let message = error.to_string();
            database
                .record_event(
                    &job_id,
                    &JobEvent::DownloadFailed {
                        error_code: error_code.to_owned(),
                        error_message: message.clone(),
                    },
                    &now,
                )
                .map_err(|record_error| record_error.to_string())?;
            database
                .fail_job(&job_id, next_state, error_code, &message, &now)
                .map_err(|failure_error| failure_error.to_string())?;
            state.database = Some(database);
            state.sidecar = Some(supervisor);
            return Err(message);
        }
    };

    // Archive the results
    let mut archive_result = archive_result;
    merge_browser_relationships(&mut archive_result.metadata, &request.tweet);
    let mut archive = ArchiveService::new(database, files);
    let final_directory = PathBuf::from("archives").join(&request.tweet.tweet_id);
    archive
        .complete_sidecar_archive(SidecarArchiveRequest {
            job_id: &job_id,
            tweet_row_id,
            expected_tweet_id: &request.tweet.tweet_id,
            metadata: &archive_result.metadata,
            files: &archive_result.files,
            final_directory: &final_directory,
            archived_at: &now,
        })
        .map_err(|error| error.to_string())?;

    archive
        .database
        .record_event(
            &job_id,
            &JobEvent::DownloadCompleted {
                files_copied: archive_result.files.len() as u32,
            },
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
    use crate::aria2::{selected_aria2_release, sha256_hex};

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

    fn browser_tweet(tweet_type: &str) -> xarchive_protocol::BrowserTweet {
        xarchive_protocol::BrowserTweet {
            tweet_id: "123".into(),
            url: "https://x.com/alice/status/123".into(),
            username: Some("alice".into()),
            display_name: Some("Alice".into()),
            text: Some("quoting".into()),
            created_at: Some("2026-09-12T00:00:00Z".into()),
            tweet_type: tweet_type.into(),
            reply_to: Some("111".into()),
            quoted_tweet: Some(
                xarchive_protocol::BrowserTweet {
                    tweet_id: "987".into(),
                    url: "https://x.com/bob/status/987".into(),
                    username: Some("bob".into()),
                    display_name: Some("Bob".into()),
                    text: Some("original".into()),
                    created_at: Some("2026-09-11T00:00:00Z".into()),
                    tweet_type: "post".into(),
                    reply_to: None,
                    quoted_tweet: None,
                }
                .into(),
            ),
        }
    }

    #[test]
    fn merges_browser_relationships_into_sidecar_metadata_gaps() {
        let mut metadata = serde_json::json!({
            "tweet_id": "123",
            "url": "https://x.com/alice/status/123",
            "tweet_type": "post",
            "text": "quoting"
        });
        let tweet = browser_tweet("quote");
        merge_browser_relationships(&mut metadata, &tweet);
        assert_eq!(metadata["reply_to"], "111");
        assert_eq!(metadata["quoted_tweet"]["tweet_id"], "987");
        assert_eq!(metadata["quoted_tweet"]["username"], "bob");
        assert_eq!(metadata["tweet_type"], "quote");
    }

    #[test]
    fn preserves_sidecar_provided_relationship_data() {
        let mut metadata = serde_json::json!({
            "tweet_id": "123",
            "reply_to": "222",
            "tweet_type": "quote",
            "quoted_tweet": {"tweet_id": "555", "url": "https://x.com/carol/status/555"}
        });
        let tweet = browser_tweet("quote");
        merge_browser_relationships(&mut metadata, &tweet);
        assert_eq!(metadata["reply_to"], "222");
        assert_eq!(metadata["quoted_tweet"]["tweet_id"], "555");
        assert_eq!(metadata["tweet_type"], "quote");
    }

    #[test]
    fn ignores_non_object_sidecar_metadata() {
        let mut metadata = serde_json::Value::Null;
        let tweet = browser_tweet("quote");
        merge_browser_relationships(&mut metadata, &tweet);
        assert_eq!(metadata, serde_json::Value::Null);
    }
}
