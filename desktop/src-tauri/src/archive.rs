use std::path::PathBuf;
use std::time::Duration;

use std::sync::{Arc, Mutex};
use xarchive_download::{DownloadRouter, GalleryDlFailure};

use tauri::State;
use xarchive_core::{JobEvent, JobState};
use xarchive_storage::{ArchiveService, FileStore, JobSummary, SidecarArchiveRequest};

use crate::ArchiveTweetRequest;
use crate::commands;
use crate::executor::{
    ArchiveJobSubmissionAdapter, CancellationToken, JobExecution, JobExecutionError,
    JobExecutionResult, JobSnapshot,
};
use crate::runtime::{RuntimeState, timestamp_marker};

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

/// Resources required by one archive execution.
///
/// The context is independent from Tauri State and is created either by the
/// production executor factory or by the explicit synchronous fallback.
pub(crate) struct ArchiveExecutionContext {
    pub(crate) database: Database,
    pub(crate) files: FileStore,
    pub(crate) supervisor: SidecarSupervisor,
}

/// Adapter that connects one State-independent archive resource bundle to the
/// executor's single-Job execution port.
pub(crate) struct ArchiveExecutionJob {
    context: Arc<Mutex<Option<ArchiveExecutionContext>>>,
    request: ArchiveTweetRequest,
    tweet_row_id: i64,
    request_id: String,
    archived_at: String,
}

impl ArchiveExecutionJob {
    // The production executor factory and the synchronous fallback both use
    // this adapter with independent resource contexts.
    #[allow(dead_code)]
    pub(crate) fn new(
        context: ArchiveExecutionContext,
        request: ArchiveTweetRequest,
        tweet_row_id: i64,
        request_id: String,
        archived_at: String,
    ) -> (Self, Arc<Mutex<Option<ArchiveExecutionContext>>>) {
        let context = Arc::new(Mutex::new(Some(context)));
        (
            Self {
                context: context.clone(),
                request,
                tweet_row_id,
                request_id,
                archived_at,
            },
            context,
        )
    }
}

impl JobExecution for ArchiveExecutionJob {
    fn execute(
        &mut self,
        job: &JobSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        if job.tweet_id != self.request.tweet.tweet_id {
            return Err(JobExecutionError {
                error_code: "JOB_IDENTITY_MISMATCH".to_owned(),
                error_message: "executor Job tweet identity does not match archive request"
                    .to_owned(),
                persistence_already_updated: false,
            });
        }
        let context = self
            .context
            .lock()
            .map_err(|_| JobExecutionError {
                error_code: "EXECUTION_CONTEXT_POISONED".to_owned(),
                error_message: "archive execution context lock was poisoned".to_owned(),
                persistence_already_updated: false,
            })?
            .take()
            .ok_or_else(|| JobExecutionError {
                error_code: "EXECUTION_ALREADY_CONSUMED".to_owned(),
                error_message: "archive execution context was already consumed".to_owned(),
                persistence_already_updated: false,
            })?;
        match execute_archive_context(
            context,
            &self.request,
            job,
            self.tweet_row_id,
            &self.request_id,
            &self.archived_at,
            cancellation,
        ) {
            Ok((context, result)) => {
                if let Ok(mut stored) = self.context.lock() {
                    *stored = Some(context);
                }
                Ok(result)
            }
            Err(payload) => {
                let (context, error) = *payload;
                if let Ok(mut stored) = self.context.lock() {
                    *stored = Some(context);
                }
                Err(error)
            }
        }
    }
}

fn execute_archive_context(
    mut context: ArchiveExecutionContext,
    request: &ArchiveTweetRequest,
    job: &JobSnapshot,
    tweet_row_id: i64,
    request_id: &str,
    archived_at: &str,
    cancellation: &CancellationToken,
) -> Result<
    (ArchiveExecutionContext, JobExecutionResult),
    Box<(ArchiveExecutionContext, JobExecutionError)>,
> {
    let mut result =
        match context.download_sidecar(request, &job.job_id, request_id, archived_at, cancellation)
        {
            Ok(result) => result,
            Err(message) => {
                return Err(Box::new((
                    context,
                    JobExecutionError {
                        error_code: "ARCHIVE_DOWNLOAD_FAILED".to_owned(),
                        error_message: message,
                        persistence_already_updated: false,
                    },
                )));
            }
        };
    merge_browser_relationships(&mut result.metadata, &request.tweet);
    let (database, files, supervisor) = context.into_parts();
    let mut archive = ArchiveService::new(database, files);
    let final_directory = PathBuf::from("Tweets").join(&request.tweet.tweet_id);
    if let Err(error) = archive.complete_sidecar_archive(SidecarArchiveRequest {
        job_id: &job.job_id,
        tweet_row_id,
        expected_tweet_id: &request.tweet.tweet_id,
        metadata: &result.metadata,
        files: &result.files,
        final_directory: &final_directory,
        archived_at,
    }) {
        return Err(Box::new((
            ArchiveExecutionContext::new(archive.database, archive.files, supervisor),
            JobExecutionError {
                error_code: "ARCHIVE_COMMIT_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            },
        )));
    }
    if let Err(error) = archive.database.record_event(
        &job.job_id,
        &JobEvent::DownloadCompleted {
            files_copied: result.files.len() as u32,
        },
        archived_at,
    ) {
        return Err(Box::new((
            ArchiveExecutionContext::new(archive.database, archive.files, supervisor),
            JobExecutionError {
                error_code: "ARCHIVE_EVENT_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            },
        )));
    }
    context = ArchiveExecutionContext::new(archive.database, archive.files, supervisor);
    Ok((
        context,
        JobExecutionResult {
            files_copied: result.files.len() as u32,
            archive_directory: final_directory.to_string_lossy().to_string(),
            download_event_recorded: true,
            state_already_updated: true,
        },
    ))
}

impl ArchiveExecutionContext {
    pub(crate) fn new(database: Database, files: FileStore, supervisor: SidecarSupervisor) -> Self {
        Self {
            database,
            files,
            supervisor,
        }
    }

    pub(crate) fn into_parts(self) -> (Database, FileStore, SidecarSupervisor) {
        (self.database, self.files, self.supervisor)
    }

    pub(crate) fn download_sidecar(
        &mut self,
        request: &ArchiveTweetRequest,
        job_id: &str,
        request_id: &str,
        now: &str,
        cancellation: &CancellationToken,
    ) -> Result<SidecarArchiveResult, String> {
        let staging_dir = self
            .files
            .staging_dir(job_id)
            .map_err(|error| error.to_string())?;
        let sidecar_request = SidecarDownloadRequest {
            job_id: job_id.to_owned(),
            request_id: request_id.to_owned(),
            url: request.tweet.url.clone(),
            staging_dir,
            browser: request.browser.clone(),
            profile: request.profile.clone(),
        };
        let router = DownloadRouter::default();
        self.database
            .record_event(
                job_id,
                &JobEvent::DownloadStarted {
                    backend: "gallery-dl".to_owned(),
                },
                now,
            )
            .map_err(|error| error.to_string())?;
        let mut sidecar_result: Option<SidecarArchiveResult> = None;
        let download = router.execute(
            || {
                sidecar_result = Some(run_sidecar_download(
                    &mut self.supervisor,
                    &sidecar_request,
                    cancellation,
                )?);
                Ok(())
            },
            None,
            |_request| Err(xarchive_download::DownloadError::InvalidAddUriRequest),
        );
        match download {
            Ok(_) => sidecar_result
                .take()
                .ok_or_else(|| "sidecar download completed but result was lost".to_owned()),
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
                    xarchive_download::DownloadRouterError::GalleryDlThenAria2 {
                        gallery, ..
                    } => {
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
                self.database
                    .record_event(
                        job_id,
                        &JobEvent::DownloadFailed {
                            error_code: error_code.to_owned(),
                            error_message: message.clone(),
                        },
                        now,
                    )
                    .map_err(|record_error| record_error.to_string())?;
                self.database
                    .fail_job(job_id, next_state, error_code, &message, now)
                    .map_err(|failure_error| failure_error.to_string())?;
                Err(message)
            }
        }
    }
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
    cancellation: &CancellationToken,
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
        if cancellation.is_cancelled() {
            let cancel = xarchive_protocol::SidecarCommand {
                protocol_version: xarchive_protocol::PROTOCOL_VERSION,
                request_id: request.request_id.clone(),
                cmd: xarchive_protocol::SidecarCommandType::Cancel,
                job_id: request.job_id.clone(),
                url: None,
                staging_dir: None,
                browser: None,
                profile: None,
            };
            let _ = supervisor.send(&cancel);
            supervisor.shutdown();
            return Err(GalleryDlFailure::new(
                "CANCELLED",
                "archive download cancelled",
            ));
        }
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

#[tauri::command]
pub(crate) fn archive_tweet(
    state: State<'_, Mutex<RuntimeState>>,
    request: ArchiveTweetRequest,
) -> Result<JobSummary, String> {
    let now = timestamp_marker();
    // Keep the synchronous path as the controlled fallback, but share request
    // validation and Job identity derivation with the R1 executor boundary.
    // The adapter performs no I/O and therefore does not change the fallback's
    // lock or resource ownership semantics.
    let prepared = ArchiveJobSubmissionAdapter
        .prepare(&request, &now)
        .map_err(|error| error.to_string())?;
    let job_id = prepared.job_id;
    let request_id = format!("desktop-{job_id}");

    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    if state.download_setup_required {
        return Err("download directory setup is required before archiving".to_owned());
    }
    commands::refresh_sidecar_state(&mut state);
    let mut database = match state.database.take() {
        Some(database) => database,
        None => Database::open(state.executor.database_path())
            .map_err(|error| format!("failed to open archive database: {error}"))?,
    };
    let supervisor = match state.sidecar.take() {
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

    // Create the resource bundle and execute the Sidecar download through the
    // State-independent context. The executor worker will own this context in
    // a later integration step; the synchronous fallback remains unchanged.
    let files =
        FileStore::with_staging_root(state.download_root.clone(), state.staging_root.clone())
            .map_err(|error| error.to_string())?;
    let mut context = ArchiveExecutionContext::new(database, files, supervisor);
    let cancellation = CancellationToken::new();
    let archive_result =
        match context.download_sidecar(&request, &job_id, &request_id, &now, &cancellation) {
            Ok(result) => result,
            Err(message) => {
                let (database, _files, supervisor) = context.into_parts();
                state.database = Some(database);
                state.sidecar = Some(supervisor);
                return Err(message);
            }
        };
    let (database, files, supervisor) = context.into_parts();

    // Archive the results
    let mut archive_result = archive_result;
    merge_browser_relationships(&mut archive_result.metadata, &request.tweet);
    let mut archive = ArchiveService::new(database, files);
    let final_directory = PathBuf::from("Tweets").join(&request.tweet.tweet_id);
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
