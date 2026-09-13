mod archive;
mod aria2;
mod commands;

use archive::{
    SidecarArchiveResult, SidecarDownloadRequest, merge_browser_relationships,
    run_sidecar_download, upsert_browser_user,
};
use aria2::{detect_aria2, download_aria2, list_aria2_releases};
use commands::{
    get_app_status, get_archive_root, get_runtime_health, list_jobs, open_archive_folder,
    start_sidecar, stop_sidecar,
};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use xarchive_core::{JobEvent, JobState};
use xarchive_download::DownloadRouter;
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
    commands::refresh_sidecar_state(&mut state);
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
    use crate::archive::merge_browser_relationships;
    use crate::aria2::{selected_aria2_release, sha256_hex};

    #[test]
    fn runtime_state_uses_a_dedicated_archive_directory() {
        let state = RuntimeState::initialize();
        assert!(state.archive_root.ends_with(DEFAULT_ARCHIVE_ROOT));
    }

    #[test]
    fn parses_sidecar_args_as_json_without_splitting_paths() {
        let args =
            commands::parse_sidecar_args(r#"["-m","xarchive_downloader","C:\\Work Dir\\staging"]"#)
                .expect("valid sidecar args");
        assert_eq!(args[2], r#"C:\Work Dir\staging"#);
    }

    #[test]
    fn rejects_non_array_sidecar_args() {
        let error = commands::parse_sidecar_args("--verbose").expect_err("invalid args");
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
