use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use xarchive_core::JobEvent;
use xarchive_storage::{ArchiveService, FileStore, SidecarArchiveRequest};

use crate::ArchiveTweetRequest;
use crate::executor::{
    CancellationToken, JobExecution, JobExecutionError, JobExecutionResult, JobSnapshot,
};

use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::Database;

/// Persist the browser-supplied author link for an archived tweet.
///
/// Browser payloads do not carry a stable numeric X user id, so the tweet id is
/// used as the directory anchor. When no username is present there is no
/// durable user row to link, and `Ok(None)` preserves the existing nullable
/// `tweets.user_id` behavior. Once Sidecar extraction returns a stable
/// `user_id`, `ArchiveService::refresh_author_profile` merges this temporary
/// `browser-<tweet_id>` placeholder into the stable identity (P1-C).
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
/// The context is independent from Tauri State and is created by the
/// production executor factory from the persisted execution spec.
pub(crate) struct ArchiveExecutionContext {
    pub(crate) database: Database,
    pub(crate) files: FileStore,
    pub(crate) supervisor: SidecarSupervisor,
    pub(crate) aria2_program: Option<String>,
    pub(crate) network: crate::executor::ExecutorNetworkConfig,
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
    // The production executor factory owns one resource context per Job.
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
        match context.download_v2(request, &job.job_id, request_id, archived_at, cancellation) {
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
    let network = context.network().clone();
    let (database, files, supervisor, aria2_program) = context.into_parts();
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
            ArchiveExecutionContext::with_aria2_and_network(
                archive.database,
                archive.files,
                supervisor,
                aria2_program,
                network.clone(),
            ),
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
            ArchiveExecutionContext::with_aria2_and_network(
                archive.database,
                archive.files,
                supervisor,
                aria2_program,
                network.clone(),
            ),
            JobExecutionError {
                error_code: "ARCHIVE_EVENT_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            },
        )));
    }
    context = ArchiveExecutionContext::with_aria2_and_network(
        archive.database,
        archive.files,
        supervisor,
        aria2_program,
        network,
    );
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
    pub(crate) fn with_aria2_and_network(
        database: Database,
        files: FileStore,
        supervisor: SidecarSupervisor,
        aria2_program: Option<String>,
        network: crate::executor::ExecutorNetworkConfig,
    ) -> Self {
        Self {
            database,
            files,
            supervisor,
            aria2_program,
            network,
        }
    }

    pub(crate) fn into_parts(self) -> (Database, FileStore, SidecarSupervisor, Option<String>) {
        (
            self.database,
            self.files,
            self.supervisor,
            self.aria2_program,
        )
    }

    pub(crate) fn network(&self) -> &crate::executor::ExecutorNetworkConfig {
        &self.network
    }

    pub(crate) fn download_v2(
        &mut self,
        request: &ArchiveTweetRequest,
        job_id: &str,
        request_id: &str,
        _now: &str,
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
        crate::production::execute_v2_archive(
            &mut self.supervisor,
            &sidecar_request,
            cancellation,
            self.aria2_program.as_deref(),
            &self.network,
        )
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
