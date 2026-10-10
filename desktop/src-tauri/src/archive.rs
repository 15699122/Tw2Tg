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
    /// The task's validated downloader-argument snapshot. The gallery-dl
    /// direct-download path forwards `gallery_dl`; the aria2 path forwards
    /// `aria2`. Both were validated by the Rust argument-policy authority when
    /// the task snapshot was accepted and are never resampled or rewritten.
    pub(crate) downloader_arguments: crate::executor::DownloaderArgumentSnapshot,
    pub(crate) gallery_dl_program: Option<String>,
}

/// Resources required by one archive execution.
///
/// The context is independent from Tauri State and is created by the
/// production executor factory from the persisted execution spec.
pub(crate) struct ArchiveExecutionContext {
    pub(crate) telegram_config_file: Option<PathBuf>,
    pub(crate) telegram: crate::config::TelegramConfig,
    pub(crate) database: Database,
    pub(crate) files: FileStore,
    pub(crate) supervisor: SidecarSupervisor,
    pub(crate) aria2_program: Option<String>,
    pub(crate) use_aria2: bool,
    pub(crate) network: crate::executor::ExecutorNetworkConfig,
    pub(crate) downloader_arguments: crate::executor::DownloaderArgumentSnapshot,
    pub(crate) gallery_dl_program: Option<String>,
}

/// Adapter that connects one State-independent archive resource bundle to the
/// executor's single-Job execution port.
pub(crate) struct ArchiveExecutionJob {
    context: Arc<Mutex<Option<ArchiveExecutionContext>>>,
    request: ArchiveTweetRequest,
    output_settings: xarchive_storage::BatchOutputSettings,
    tweet_row_id: i64,
    request_id: String,
    archived_at: String,
}

impl ArchiveExecutionJob {
    // The production executor factory owns one resource context per Job.
    pub(crate) fn new(
        context: ArchiveExecutionContext,
        request: ArchiveTweetRequest,
        output_settings: xarchive_storage::BatchOutputSettings,
        tweet_row_id: i64,
        request_id: String,
        archived_at: String,
    ) -> (Self, Arc<Mutex<Option<ArchiveExecutionContext>>>) {
        let context = Arc::new(Mutex::new(Some(context)));
        (
            Self {
                context: context.clone(),
                request,
                output_settings,
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
            &self.output_settings,
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
    output_settings: &xarchive_storage::BatchOutputSettings,
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
    // Sample only after extraction and transfer have completed. A malformed or
    // unavailable production configuration fails closed, never using a stale
    // enabled snapshot. Recovery subsequently uses the journal, not this file.
    let telegram = context.completion_telegram_config();
    let telegram_config_file = context.telegram_config_file.clone();
    let network = context.network().clone();
    let downloader_arguments = context.downloader_arguments.clone();
    let (database, files, supervisor, aria2_program, use_aria2) = context.into_parts();
    let mut archive = ArchiveService::new(database, files);
    let final_directory = PathBuf::from("Tweets").join(&request.tweet.tweet_id);
    let telegram_intent = capture_completed_intent(
        &archive.database,
        &archive.files,
        &job.job_id,
        tweet_row_id,
        &request.tweet.tweet_id,
        &result,
        &final_directory,
        archived_at,
        &telegram,
    );
    if let Err(error) = archive.complete_sidecar_archive(SidecarArchiveRequest {
        job_id: &job.job_id,
        tweet_row_id,
        expected_tweet_id: &request.tweet.tweet_id,
        metadata: &result.metadata,
        files: &result.files,
        final_directory: &final_directory,
        archived_at,
        telegram_intent: telegram_intent.as_ref(),
        output_settings,
    }) {
        return Err(Box::new((
            ArchiveExecutionContext::with_aria2_and_network(
                archive.database,
                archive.files,
                supervisor,
                aria2_program,
                use_aria2,
                network.clone(),
            )
            .with_telegram(telegram.clone())
            .with_telegram_config_file(telegram_config_file.clone())
            .with_downloader_arguments(downloader_arguments.clone()),
            JobExecutionError {
                error_code: "ARCHIVE_COMMIT_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            },
        )));
    }
    if telegram_intent.is_some() {
        // A planning failure must not undo a successful local archive. The
        // ARCHIVED journal remains available to startup reconciliation.
        let _ = crate::telegram_send::queue_persisted_archive_sends(
            &archive.database,
            tweet_row_id,
            archived_at,
        );
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
                use_aria2,
                network.clone(),
            )
            .with_telegram(telegram.clone())
            .with_telegram_config_file(telegram_config_file.clone())
            .with_downloader_arguments(downloader_arguments.clone()),
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
        use_aria2,
        network,
    )
    .with_telegram(telegram)
    .with_downloader_arguments(downloader_arguments);
    context.telegram_config_file = telegram_config_file;
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn capture_completed_intent(
    database: &Database,
    files: &FileStore,
    job_id: &str,
    tweet_row_id: i64,
    tweet_id: &str,
    result: &SidecarArchiveResult,
    directory: &std::path::Path,
    now: &str,
    config: &crate::config::TelegramConfig,
) -> Option<xarchive_storage::TelegramArchiveIntentRecord> {
    if !crate::telegram_send::auto_send_enabled(config) {
        return None;
    }
    let (_, bot, _) = database.active_telegram_credential_generation().ok()??;
    let metadata = xarchive_storage::build_archive_metadata(
        tweet_id,
        &result.metadata,
        &result.files,
        &files.staging_dir(job_id).ok()?,
        now,
    )
    .ok()?;
    crate::telegram_send::ArchiveSendIntent::from_archive_metadata(
        tweet_row_id,
        directory.to_str()?.to_owned(),
        &metadata,
        config,
        &bot,
    )
    .ok()?
    .to_journal_record(job_id, now)
    .ok()
}

fn sample_completion_config(
    path: Option<&std::path::Path>,
    fallback: &crate::config::TelegramConfig,
) -> crate::config::TelegramConfig {
    match path {
        None => fallback.clone(),
        Some(path) => std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_yaml::from_str::<crate::config::AppConfig>(&raw).ok())
            .filter(|config| config.telegram.validate().is_ok())
            .map(|config| config.telegram)
            .unwrap_or_default(),
    }
}

impl ArchiveExecutionContext {
    fn with_telegram_config_file(mut self, path: Option<PathBuf>) -> Self {
        self.telegram_config_file = path;
        self
    }
    fn completion_telegram_config(&self) -> crate::config::TelegramConfig {
        sample_completion_config(self.telegram_config_file.as_deref(), &self.telegram)
    }

    pub(crate) fn with_telegram(mut self, config: crate::config::TelegramConfig) -> Self {
        self.telegram = config;
        self
    }
    pub(crate) fn with_aria2_and_network(
        database: Database,
        files: FileStore,
        supervisor: SidecarSupervisor,
        aria2_program: Option<String>,
        use_aria2: bool,
        network: crate::executor::ExecutorNetworkConfig,
    ) -> Self {
        Self {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            database,
            files,
            supervisor,
            aria2_program,
            use_aria2,
            network,
            downloader_arguments: crate::executor::DownloaderArgumentSnapshot::historical(),
            gallery_dl_program: None,
        }
    }

    pub(crate) fn with_gallery_dl_program(mut self, program: Option<String>) -> Self {
        self.gallery_dl_program = program;
        self
    }

    /// Attach the task's validated downloader-argument snapshot. Set by the
    /// production factory when it creates the context from the decoded spec, so
    /// the gallery-dl and aria2 paths forward exactly the arguments that were
    /// accepted for this task.
    pub(crate) fn with_downloader_arguments(
        mut self,
        downloader_arguments: crate::executor::DownloaderArgumentSnapshot,
    ) -> Self {
        self.downloader_arguments = downloader_arguments;
        self
    }

    pub(crate) fn into_parts(
        self,
    ) -> (Database, FileStore, SidecarSupervisor, Option<String>, bool) {
        (
            self.database,
            self.files,
            self.supervisor,
            self.aria2_program,
            self.use_aria2,
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
            downloader_arguments: self.downloader_arguments.clone(),
            gallery_dl_program: self.gallery_dl_program.clone(),
        };
        crate::production::execute_v2_archive(
            &mut self.supervisor,
            &sidecar_request,
            cancellation,
            self.aria2_program.as_deref(),
            self.use_aria2,
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

#[cfg(test)]
mod completion_tests {
    use super::*;

    #[test]
    fn completion_samples_latest_persisted_settings_and_fails_closed() {
        let root = std::env::temp_dir().join(format!(
            "tg-completion-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        let paths = crate::portable::PortablePaths::from_root(&root);
        let mut config = crate::config::AppConfig::default();
        config.telegram.enabled = true;
        config.telegram.auto_send_on_archive = true;
        config.telegram.chat_id = "old-target".into();
        let stale = config.telegram.clone();
        config.telegram.chat_id = "new-target".into();
        config.telegram.revision = 19;
        config.save(&paths).unwrap();
        let sampled = sample_completion_config(Some(&paths.config_file), &stale);
        assert_eq!(sampled.chat_id, "new-target");
        assert_eq!(sampled.revision, 19);
        config.telegram.enabled = false;
        config.save(&paths).unwrap();
        assert!(!sample_completion_config(Some(&paths.config_file), &stale).enabled);
        std::fs::write(&paths.config_file, "broken: [").unwrap();
        assert!(!sample_completion_config(Some(&paths.config_file), &stale).enabled);
        std::fs::remove_file(&paths.config_file).unwrap();
        assert!(!sample_completion_config(Some(&paths.config_file), &stale).enabled);
        std::fs::remove_dir_all(root).unwrap();
    }
}
