use std::path::PathBuf;
use std::time::Duration;

use xarchive_download::GalleryDlFailure;
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::Database;

/// Persist the browser-supplied author link for an archived tweet.
///
/// Browser payloads do not carry a stable numeric X user id, so the tweet id
/// is used as the directory anchor. When no username is present there is no
/// durable user row to link, and `Ok(None)` preserves the existing nullable
/// `tweets.user_id` behavior.
pub(crate) fn upsert_browser_user(
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

pub(crate) struct SidecarArchiveResult {
    pub(crate) metadata: serde_json::Value,
    pub(crate) files: Vec<xarchive_protocol::DownloadFile>,
}

pub(crate) struct SidecarDownloadRequest {
    pub(crate) job_id: String,
    pub(crate) request_id: String,
    pub(crate) url: String,
    pub(crate) staging_dir: PathBuf,
    pub(crate) browser: Option<String>,
    pub(crate) profile: Option<String>,
}

/// Merge browser-DOM relationship data into the Sidecar metadata payload.
///
/// gallery-dl's info.json rarely carries structured reply/quote references,
/// while the Browser extension extracts them directly from the DOM. The
/// browser-supplied data only fills gaps in the Sidecar payload and never
/// overwrites metadata the Sidecar already provided. A Sidecar `tweet_type`
/// of `post` is upgraded to the browser-observed `reply`/`quote` because the
/// DOM social context is the more reliable signal for the tweet kind.
pub(crate) fn merge_browser_relationships(
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

pub(crate) fn run_sidecar_download(
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
