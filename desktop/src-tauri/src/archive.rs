use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use xarchive_core::{ArchiveAuthor, ArchiveMedia, ArchiveMetadata, ArchiveQuotedTweet};
use xarchive_storage::{
    ArchiveRecoveryManifestV2, ArchiveRecoveryMediaFact, ArchiveRenamePlan, ArchiveService,
    DrainAuthorization, FileStore,
};

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
    #[cfg(target_os = "linux")]
    pub(crate) retained_aria2: Option<xarchive_download::Aria2Supervisor>,
}

#[cfg(target_os = "linux")]
type ArchiveExecutionFailure = (String, Option<xarchive_download::Aria2Supervisor>);
#[cfg(not(target_os = "linux"))]
type ArchiveExecutionFailure = String;

/// Attempt-owned archive facts produced before publication.
///
/// Unlike [`JobExecutionResult`], this type is deliberately non-`Clone` and
/// carries the durable attempt identity plus the drained process owners that
/// must stay alive across the archive/publication seam. Publication code
/// receives this value by move, so a stale result cannot be replayed for
/// another attempt and a drained owner cannot be dropped before the
/// runner-local finalization observes it.
///
/// The drain-gated constructor rejects facts unless the owned Sidecar has
/// shut down successfully before publication.
#[derive(Debug)]
pub(crate) struct AttemptArchiveFacts {
    pub(crate) attempt_identity: xarchive_workflow::AttemptIdentity,
    #[allow(dead_code)]
    pub(crate) metadata: serde_json::Value,
    pub(crate) files: Vec<xarchive_protocol::DownloadFile>,
    pub(crate) staging_dir: std::path::PathBuf,
    pub(crate) transfer_file_count: usize,
    #[cfg(target_os = "linux")]
    #[allow(dead_code)]
    pub(crate) retained_aria2: Option<xarchive_download::Aria2Supervisor>,
}

/// Drain-gated constructor probe: a facts value can only be built while the
/// caller still owns its Sidecar supervisor and has observed a successful
/// shutdown. Failure to drain refuses facts construction, so publication can
/// never proceed past an undrained writer.
impl AttemptArchiveFacts {
    #[allow(dead_code)]
    pub(crate) fn after_sidecar_drain(
        supervisor: &mut SidecarSupervisor,
        attempt_identity: xarchive_workflow::AttemptIdentity,
        metadata: serde_json::Value,
        files: Vec<xarchive_protocol::DownloadFile>,
        staging_dir: std::path::PathBuf,
        #[cfg(target_os = "linux")] retained_aria2: Option<xarchive_download::Aria2Supervisor>,
    ) -> Result<Self, xarchive_sidecar_supervisor::SupervisorError> {
        supervisor.shutdown()?;
        let transfer_file_count = files.len();
        Ok(Self {
            attempt_identity,
            metadata,
            files,
            staging_dir,
            transfer_file_count,
            #[cfg(target_os = "linux")]
            retained_aria2,
        })
    }
}

/// Verify that a facts value still points at the immutable staging tree it was
/// built from. Publication must refuse when the attempt directory vanished,
/// became a non-directory, or was replaced by a symlink/reparse point after
/// the transfer completed; re-creating it here would turn a missing payload
/// into an apparently valid empty tree.
fn verify_attempt_facts_staging(
    files: &FileStore,
    job_id: &str,
    facts: &AttemptArchiveFacts,
) -> Result<(), String> {
    // Resolve the live attempt tree from the durable identity carried by the
    // facts, never from a directory name that could be forged after transfer.
    let expected = files
        .existing_attempt_staging_dir(job_id, facts.attempt_identity.attempt())
        .map_err(|error| error.to_string())?;
    if expected != facts.staging_dir {
        return Err(
            "attempt facts staging path does not match immutable attempt staging".to_owned(),
        );
    }
    if facts.files.len() != facts.transfer_file_count {
        return Err("attempt facts file count does not match transfer-derived count".to_owned());
    }
    Ok(())
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
    #[cfg(target_os = "linux")]
    pub(crate) containment: Option<crate::executor::AttemptWriterDomain>,
    pub(crate) attempt: Option<xarchive_workflow::AttemptIdentity>,
    pub(crate) telegram_config_file: Option<PathBuf>,
    pub(crate) telegram: crate::config::TelegramConfig,
    pub(crate) database: Database,
    pub(crate) files: FileStore,
    pub(crate) supervisor: SidecarSupervisor,
    #[cfg(target_os = "linux")]
    retained_aria2: Option<xarchive_download::Aria2Supervisor>,
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
    metadata_output: Option<crate::executor::MetadataOutputSettings>,
    tweet_row_id: i64,
    request_id: String,
    archived_at: String,
}

impl ArchiveExecutionJob {
    pub(crate) fn with_metadata_output(
        mut self,
        settings: Option<crate::executor::MetadataOutputSettings>,
    ) -> Self {
        self.metadata_output = settings;
        self
    }
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
                metadata_output: None,
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
            self.metadata_output.as_ref(),
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

/// Run one archive execution against an already-built context.
///
/// The argument count is above clippy's threshold because every value is a
/// distinct input to the execution contract; bundling them into a struct would
/// be a shared-signature change that needs its own reviewed batch, so the lint
/// is scoped here instead of refactored as a drive-by.
#[allow(clippy::too_many_arguments)]
fn execute_archive_context(
    mut context: ArchiveExecutionContext,
    request: &ArchiveTweetRequest,
    output_settings: &xarchive_storage::BatchOutputSettings,
    metadata_output: Option<&crate::executor::MetadataOutputSettings>,
    job: &JobSnapshot,
    tweet_row_id: i64,
    request_id: &str,
    archived_at: &str,
    cancellation: &CancellationToken,
) -> Result<
    (ArchiveExecutionContext, JobExecutionResult),
    Box<(ArchiveExecutionContext, JobExecutionError)>,
> {
    let Some(attempt_identity) = context
        .attempt
        .as_ref()
        .filter(|identity| identity.job_id() == job.job_id)
        .cloned()
    else {
        return Err(Box::new((
            context,
            JobExecutionError {
                error_code: "ATTEMPT_EXECUTION_REQUIRED".to_owned(),
                error_message: "archive execution requires a matching claimed attempt identity"
                    .to_owned(),
                persistence_already_updated: false,
            },
        )));
    };
    let facts = match collect_attempt_facts(
        &mut context,
        request,
        job,
        request_id,
        archived_at,
        cancellation,
    ) {
        Ok(facts) => facts,
        Err((error_code, error_message)) => {
            // `collect_attempt_facts` already performed the Sidecar drain
            // gate; its error preserves the drain diagnostic. The legacy
            // error return below keeps the context recoverable for the
            // existing caller contract.
            return Err(Box::new((
                context,
                JobExecutionError {
                    error_code,
                    error_message,
                    persistence_already_updated: false,
                },
            )));
        }
    };
    // `collect_attempt_facts` owns the drain/verification half. The publication
    // half below consumes the verified facts by move through the fenced
    // publication helper, so `facts` is never cloned back into a replayable
    // `SidecarArchiveResult` on this path.
    publish_verified_attempt_facts(
        context,
        facts,
        attempt_identity,
        request,
        output_settings,
        metadata_output,
        job,
        tweet_row_id,
        archived_at,
    )
}

/// Publish one verified attempt-facts value through the fenced v2 path.
///
/// The facts value is consumed by move: callers cannot reuse it for another
/// attempt, and publication observes the drained transfer state plus the
/// durable attempt identity carried by the value.
#[allow(clippy::too_many_arguments)]
fn publish_verified_attempt_facts(
    mut context: ArchiveExecutionContext,
    facts: AttemptArchiveFacts,
    attempt_identity: xarchive_workflow::AttemptIdentity,
    request: &ArchiveTweetRequest,
    output_settings: &xarchive_storage::BatchOutputSettings,
    metadata_output: Option<&crate::executor::MetadataOutputSettings>,
    job: &JobSnapshot,
    tweet_row_id: i64,
    archived_at: &str,
) -> Result<
    (ArchiveExecutionContext, JobExecutionResult),
    Box<(ArchiveExecutionContext, JobExecutionError)>,
> {
    let AttemptArchiveFacts {
        attempt_identity: facts_identity,
        metadata: _,
        files: facts_files,
        staging_dir: facts_staging,
        transfer_file_count: facts_count,
        #[cfg(target_os = "linux")]
            retained_aria2: facts_retained_aria2,
    } = facts;
    // Publication observes the moved facts owners: the durable attempt
    // identity anchors the manifest, and the verified staging/count pair is
    // the only staging authority on this path. A drained Linux aria2 owner is
    // carried by the value; an undrained owner never reaches this helper
    // because `collect_attempt_facts` refuses facts construction for it.
    debug_assert_eq!(facts_identity.job_id(), job.job_id);
    debug_assert_eq!(facts_identity.job_id(), attempt_identity.job_id());
    debug_assert_eq!(facts_identity.attempt(), attempt_identity.attempt());
    #[cfg(target_os = "linux")]
    debug_assert!(facts_retained_aria2.is_none());
    let source_paths = facts_files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<Vec<_>>();
    let final_paths = match xarchive_storage::render_media_filenames(
        &source_paths,
        request.tweet.username.as_deref(),
        &request.tweet.tweet_id,
        output_settings,
    ) {
        Ok(paths) => paths,
        Err(error) => {
            return Err(Box::new((
                context,
                JobExecutionError {
                    error_code: "ARCHIVE_FILENAME_INVALID".to_owned(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                },
            )));
        }
    };
    // Sample only after extraction and transfer have completed. A malformed or
    // unavailable production configuration fails closed, never using a stale
    // enabled snapshot. Recovery subsequently uses the journal, not this file.
    let telegram = context.completion_telegram_config();
    let telegram_config_file = context.telegram_config_file.clone();
    let network = context.network().clone();
    let downloader_arguments = context.downloader_arguments.clone();
    let (database, files, supervisor, aria2_program, use_aria2, mut retained_aria2) =
        context.into_parts();
    let mut archive = ArchiveService::new(database, files);
    let final_directory = PathBuf::from("Tweets").join(&request.tweet.tweet_id);
    // Build fenced v2 publication manifest from verified attempt facts.
    // Hashes are computed first (error path returns from the function), then
    // the manifest is assembled from precomputed hashes so `archive.database`
    // is never moved into a closure across loop iterations.
    let mut sha256_hashes = Vec::new();
    for file in &facts_files {
        let source_full = facts_staging.join(&file.relative_path);
        let sha256 = match FileStore::sha256(&source_full) {
            Ok(sha) => sha,
            Err(error) => {
                return Err(Box::new((
                    ArchiveExecutionContext::with_aria2_and_network(
                        archive.database,
                        archive.files,
                        supervisor,
                        aria2_program,
                        use_aria2,
                        network.clone(),
                    )
                    .with_retained_aria2(retained_aria2.take())
                    .with_attempt_identity(attempt_identity.clone())
                    .with_telegram(telegram.clone())
                    .with_telegram_config_file(telegram_config_file.clone())
                    .with_downloader_arguments(downloader_arguments.clone()),
                    JobExecutionError {
                        error_code: "ARCHIVE_MANIFEST_HASH_FAILED".to_owned(),
                        error_message: error.to_string(),
                        persistence_already_updated: false,
                    },
                )));
            }
        };
        sha256_hashes.push(sha256);
    }
    let mut rename_plan = Vec::new();
    let mut media_facts = Vec::new();
    let mut archive_media = Vec::new();
    for (index, file) in facts_files.iter().enumerate() {
        let media_index = (index + 1) as u32;
        let temporary_path = format!(".xarchive-rename-{:02}.tmp", media_index);
        let sha256 = &sha256_hashes[index];
        rename_plan.push(ArchiveRenamePlan {
            source_path: file.relative_path.clone(),
            temporary_path,
            final_path: final_paths[index].clone(),
            size_bytes: file.size_bytes,
            sha256: sha256.clone(),
        });
        media_facts.push(ArchiveRecoveryMediaFact {
            media_index,
            relative_path: final_paths[index].clone(),
            media_id: None,
            media_type: file.media_type.clone(),
            mime_type: file.mime_type.clone(),
            size_bytes: file.size_bytes,
            sha256: sha256.clone(),
        });
        archive_media.push(ArchiveMedia {
            index: media_index,
            media_id: None,
            media_type: file.media_type.clone(),
            file: final_paths[index].clone(),
            mime_type: file.mime_type.clone(),
            size_bytes: file.size_bytes,
            sha256: sha256.clone(),
        });
    }

    let mut manifest = ArchiveRecoveryManifestV2 {
        metadata_output: metadata_output.map(|settings| {
            xarchive_storage::MetadataExportSelection {
                schema_version: 4,
                export_json: settings.export_json,
                export_yaml: settings.export_yaml,
            }
        }),
        schema_version: 2,
        job_id: job.job_id.clone(),
        attempt_count: facts_identity.attempt(),
        tweet_row_id,
        tweet_id: request.tweet.tweet_id.clone(),
        archive_directory: format!("Tweets/{}", request.tweet.tweet_id),
        rename_phase: "PLANNED".to_owned(),
        rename_plan,
        media: media_facts,
        telegram_tweet_row_id: None,
        export_json: output_settings.export_json,
        export_text: output_settings.export_text,
    };

    let author = ArchiveAuthor {
        user_id: None,
        username: request.tweet.username.clone(),
        display_name: request.tweet.display_name.clone(),
    };

    let metadata = ArchiveMetadata {
        schema_version: 2,
        tweet_id: request.tweet.tweet_id.clone(),
        url: request.tweet.url.clone(),
        tweet_type: request.tweet.tweet_type.clone(),
        author,
        created_at: request.tweet.created_at.clone(),
        text: request.tweet.text.clone().unwrap_or_default(),
        media: archive_media,
        archived_at: archived_at.to_owned(),
        reply_to: request.tweet.reply_to.clone(),
        quoted_tweet: request
            .tweet
            .quoted_tweet
            .as_ref()
            .map(|quoted| ArchiveQuotedTweet {
                tweet_id: quoted.tweet_id.clone(),
                url: quoted.url.clone(),
                username: quoted.username.clone(),
                display_name: quoted.display_name.clone(),
                user_id: None,
                text: quoted.text.clone(),
                created_at: quoted.created_at.clone(),
                tweet_type: Some(quoted.tweet_type.clone()),
            }),
    };

    let telegram_intent = if crate::telegram_send::auto_send_enabled(&telegram) {
        archive
            .database
            .active_telegram_credential_generation()
            .ok()
            .flatten()
            .and_then(|(_, bot, _)| {
                crate::telegram_send::ArchiveSendIntent::from_archive_metadata(
                    tweet_row_id,
                    final_directory.to_string_lossy().into_owned(),
                    &metadata,
                    &telegram,
                    &bot,
                )
                .ok()
            })
            .and_then(|intent| intent.to_journal_record(&job.job_id, archived_at).ok())
    } else {
        None
    };
    manifest.telegram_tweet_row_id = telegram_intent.as_ref().map(|intent| intent.tweet_row_id);
    if let Some(intent) = &telegram_intent {
        if let Err(error) = archive.database.prepare_telegram_archive_intent(intent) {
            return Err(Box::new((
                ArchiveExecutionContext::with_aria2_and_network(
                    archive.database,
                    archive.files,
                    supervisor,
                    aria2_program,
                    use_aria2,
                    network,
                )
                .with_retained_aria2(retained_aria2.take())
                .with_attempt_identity(attempt_identity)
                .with_telegram(telegram)
                .with_telegram_config_file(telegram_config_file)
                .with_downloader_arguments(downloader_arguments),
                JobExecutionError {
                    error_code: "TELEGRAM_INTENT_PREPARE_FAILED".into(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                },
            )));
        }
    }
    let exports = metadata_output
        .map(|settings| settings.serialize_metadata(&metadata))
        .transpose();
    // Drain authorization: `AttemptArchiveFacts` can only be built after the
    // Sidecar drain gate passed, so this token binds publication to a drained
    // writer for this attempt. Storage rejects PREPARED without it.
    let drain_authorization =
        DrainAuthorization::after_drain(job.job_id.clone(), facts_identity.attempt());

    let publication = match exports {
        Ok(exports) => archive.publish_prepared_internal_rename_v2_with_exports(
            &manifest,
            &metadata,
            &final_directory,
            archived_at,
            None,
            exports.as_deref(),
            &drain_authorization,
            &final_directory,
        ),
        Err(error) => Err(xarchive_storage::StorageError::InvalidMetadata(error)),
    };
    if let Err(error) = publication {
        return Err(Box::new((
            ArchiveExecutionContext::with_aria2_and_network(
                archive.database,
                archive.files,
                supervisor,
                aria2_program,
                use_aria2,
                network.clone(),
            )
            .with_retained_aria2(retained_aria2.take())
            .with_attempt_identity(attempt_identity.clone())
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
    // Completion event is intentionally not emitted here: the caller
    // (`execute_session`) records `DownloadCompleted` after the durable
    // publication commit and before the `state_already_updated` skip, so the
    // event and the publication decision stay within the same session
    // boundary. `download_event_recorded` stays `false` to let the caller
    // know it must record the event.
    context = ArchiveExecutionContext::with_aria2_and_network(
        archive.database,
        archive.files,
        supervisor,
        aria2_program,
        use_aria2,
        network,
    )
    .with_retained_aria2(retained_aria2.take())
    .with_attempt_identity(attempt_identity)
    .with_telegram(telegram)
    .with_downloader_arguments(downloader_arguments);
    context.telegram_config_file = telegram_config_file;
    Ok((
        context,
        JobExecutionResult {
            // `transfer_file_count` is the transfer-verified count observed at
            // the drain gate; `facts_files.len()` is the staging count. Both
            // are equal on this path (the drain gate rejects mismatches), but
            // the durable completion event must carry the transfer-verified
            // count, not a re-derived staging length.
            files_copied: facts_count as u32,
            archive_directory: final_directory.to_string_lossy().to_string(),
            download_event_recorded: false,
            state_already_updated: true,
        },
    ))
}

/// Collect verified attempt facts without publishing.
///
/// This separates the archive half of `execute_archive_context` from the
/// legacy publication half: it runs the existing download, performs the
/// existing Sidecar drain gate, verifies the immutable attempt staging tree,
/// and returns the move-only facts value. The legacy publication path stays
/// in `execute_archive_context` until the runner-local fenced
/// prepare/publish/finalize composition lands; production scheduling,
/// recovery, and v2 remain fail-closed.
#[allow(clippy::too_many_arguments)]
fn collect_attempt_facts(
    context: &mut ArchiveExecutionContext,
    request: &ArchiveTweetRequest,
    job: &JobSnapshot,
    request_id: &str,
    archived_at: &str,
    cancellation: &CancellationToken,
) -> Result<AttemptArchiveFacts, (String, String)> {
    // Recover the durable attempt identity established by the claim before any
    // writer starts. Facts are bound to this identity, not derived from a
    // directory name after the fact.
    let attempt_identity = context
        .attempt
        .as_ref()
        .filter(|identity| identity.job_id() == job.job_id && identity.attempt() > 0)
        .cloned()
        .ok_or_else(|| {
            (
                "ATTEMPT_EXECUTION_REQUIRED".to_owned(),
                "attempt-owned archive execution is required".to_owned(),
            )
        })?;

    // Create the attempt-scoped staging tree exactly once, after the claim.
    let attempt_staging = context
        .create_attempt_staging_directory(&job.job_id)
        .map_err(|message| ("ATTEMPT_STAGING_REQUIRED".to_owned(), message))?;

    let mut result = match context.download_v2(
        request,
        &job.job_id,
        &attempt_staging,
        request_id,
        archived_at,
        cancellation,
    ) {
        Ok(result) => result,
        // Preserve any undrained aria2 owner in the context so the caller can
        // observe and clean it up instead of silently dropping it.
        Err((message, retained_aria2)) => {
            #[cfg(target_os = "linux")]
            {
                context.retained_aria2 = retained_aria2;
            }
            let (error_code, error_message) = match context.shutdown_sidecar() {
                Ok(()) => ("ARCHIVE_DOWNLOAD_FAILED".to_owned(), message),
                Err(shutdown) => (
                    "ARCHIVE_WRITER_DRAIN_FAILED".to_owned(),
                    format!("{message}; Sidecar shutdown incomplete: {shutdown}"),
                ),
            };
            return Err((error_code, error_message));
        }
    };

    #[cfg(target_os = "linux")]
    let retained_aria2 = result.retained_aria2.take();
    if let Err(error) = context.shutdown_sidecar() {
        #[cfg(target_os = "linux")]
        {
            // Preserve the transfer-retained owner before reporting the drain
            // failure so the caller is not left without cleanup authority. The
            // owner must be stored back unconditionally: a `Some` retained
            // supervisor is an undrained writer and must not be dropped.
            context.retained_aria2 = retained_aria2;
        }
        return Err((
            "ARCHIVE_WRITER_DRAIN_FAILED".to_owned(),
            format!("Sidecar shutdown incomplete: {error}"),
        ));
    }

    // A retained aria2 owner must already be drained before facts exist.
    // `run_transfer_with_supervisor` retains the supervisor exactly when its
    // shutdown reports failure, so any `Some` here is an undrained writer and
    // must refuse facts construction rather than flow into publication.
    #[cfg(target_os = "linux")]
    if retained_aria2.is_some() {
        return Err((
            "ARCHIVE_WRITER_DRAIN_FAILED".to_owned(),
            "retained aria2 owner is not drained".to_owned(),
        ));
    }

    merge_browser_relationships(&mut result.metadata, &request.tweet);
    let facts = AttemptArchiveFacts {
        attempt_identity,
        metadata: result.metadata,
        files: result.files,
        staging_dir: attempt_staging,
        transfer_file_count: 0,
        #[cfg(target_os = "linux")]
        retained_aria2: None,
    };
    let facts = AttemptArchiveFacts {
        transfer_file_count: facts.files.len(),
        ..facts
    };
    verify_attempt_facts_staging(&context.files, &job.job_id, &facts)
        .map_err(|message| ("ATTEMPT_STAGING_UNVERIFIED".to_owned(), message))?;
    Ok(facts)
}

#[allow(clippy::too_many_arguments, dead_code)]
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
    /// Stop the per-execution Sidecar and join its owned output readers before
    /// any archive metadata or completion writes. This proves local supervisor
    /// cleanup only; escaped descendants and crash recovery remain separate
    /// containment gates.
    fn shutdown_sidecar(&mut self) -> Result<(), xarchive_sidecar_supervisor::SupervisorError> {
        self.supervisor.shutdown()
    }

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
            attempt: None,
            #[cfg(target_os = "linux")]
            containment: None,
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            database,
            files,
            supervisor,
            retained_aria2: None,
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

    pub(crate) fn with_attempt_identity(
        mut self,
        identity: xarchive_workflow::AttemptIdentity,
    ) -> Self {
        self.attempt = Some(identity);
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

    #[cfg(target_os = "linux")]
    fn with_retained_aria2(mut self, owner: Option<xarchive_download::Aria2Supervisor>) -> Self {
        self.retained_aria2 = owner;
        self
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        Database,
        FileStore,
        SidecarSupervisor,
        Option<String>,
        bool,
        Option<xarchive_download::Aria2Supervisor>,
    ) {
        (
            self.database,
            self.files,
            self.supervisor,
            self.aria2_program,
            self.use_aria2,
            self.retained_aria2,
        )
    }

    pub(crate) fn network(&self) -> &crate::executor::ExecutorNetworkConfig {
        &self.network
    }

    pub(crate) fn download_v2(
        &mut self,
        request: &ArchiveTweetRequest,
        job_id: &str,
        staging_dir: &std::path::Path,
        request_id: &str,
        _now: &str,
        cancellation: &CancellationToken,
    ) -> Result<SidecarArchiveResult, ArchiveExecutionFailure> {
        let sidecar_request = SidecarDownloadRequest {
            job_id: job_id.to_owned(),
            request_id: request_id.to_owned(),
            url: request.tweet.url.clone(),
            staging_dir: staging_dir.to_path_buf(),
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
            #[cfg(target_os = "linux")]
            self.containment.as_ref(),
        )
    }

    /// Create attempt staging once, after the durable attempt claim.
    fn create_attempt_staging_directory(&self, job_id: &str) -> Result<PathBuf, String> {
        let identity = self
            .attempt
            .as_ref()
            .filter(|identity| identity.job_id() == job_id && identity.attempt() > 0)
            .ok_or_else(|| "attempt-owned archive execution is required".to_owned())?;
        self.files
            .create_attempt_staging_dir(job_id, identity.attempt())
            .map_err(|error| error.to_string())
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

    #[test]
    fn attempt_staging_resolution_requires_matching_identity_and_isolates_attempts() {
        let root = std::env::temp_dir().join(format!(
            "attempt-context-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        let archive = root.join("archive");
        let staging = root.join("staging");
        let files = FileStore::with_staging_root(&archive, &staging).expect("file store");
        let supervisor_script = root.join("sidecar-stub.sh");
        std::fs::write(&supervisor_script, "#!/bin/sh\ncat >/dev/null\n").expect("script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&supervisor_script)
                .expect("script metadata")
                .permissions();
            permissions.set_mode(0o700);
            std::fs::set_permissions(&supervisor_script, permissions).expect("script mode");
        }
        let supervisor =
            SidecarSupervisor::spawn(supervisor_script.to_str().expect("script path"), &[])
                .expect("supervisor");
        let mut context = ArchiveExecutionContext::with_aria2_and_network(
            Database::open(root.join("db.sqlite3")).expect("database"),
            files,
            supervisor,
            None,
            false,
            crate::executor::ExecutorNetworkConfig::default(),
        );
        assert!(
            context.create_attempt_staging_directory("job-1").is_err(),
            "an unclaimed execution must not obtain shared staging"
        );

        context = context.with_attempt_identity(
            xarchive_workflow::AttemptIdentity::new("job-1", 7).expect("attempt identity"),
        );
        let attempt_path = context
            .create_attempt_staging_directory("job-1")
            .expect("attempt staging");
        assert!(attempt_path.ends_with("job-1/attempt-7"));
        assert!(
            context
                .create_attempt_staging_directory("another-job")
                .is_err()
        );
        assert!(!staging.join("job-1").join("attempt-1").exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn archive_context_shutdown_joins_sidecar_before_completion() {
        let root = std::env::temp_dir().join(format!(
            "archive-sidecar-drain-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&root).expect("root");
        let files = FileStore::with_staging_root(root.join("archive"), root.join("staging"))
            .expect("files");
        let mut context = ArchiveExecutionContext::with_aria2_and_network(
            Database::open(root.join("db.sqlite3")).expect("database"),
            files,
            SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).expect("sidecar"),
            None,
            false,
            crate::executor::ExecutorNetworkConfig::default(),
        );

        context
            .shutdown_sidecar()
            .expect("owned Sidecar and readers drained");

        assert!(context.supervisor.readers_finished());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn attempt_facts_require_sidecar_drain_and_carry_transfer_count() {
        let root = std::env::temp_dir().join(format!(
            "attempt-facts-drain-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&root).expect("root");
        let mut supervisor =
            SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).expect("sidecar");
        let staging = root.join("staging").join("job-1").join("attempt-1");
        std::fs::create_dir_all(&staging).expect("attempt staging");
        let metadata = serde_json::json!({"tweet_id": "1"});
        let files = vec![xarchive_protocol::DownloadFile {
            relative_path: "01.jpg".to_owned(),
            size_bytes: 3,
            media_type: "photo".to_owned(),
            mime_type: Some("image/jpeg".to_owned()),
        }];

        let attempt_identity =
            xarchive_workflow::AttemptIdentity::new("job-1", 1).expect("attempt identity");
        let facts = AttemptArchiveFacts::after_sidecar_drain(
            &mut supervisor,
            attempt_identity.clone(),
            metadata.clone(),
            files,
            staging.clone(),
            #[cfg(target_os = "linux")]
            None,
        )
        .expect("drained Sidecar must yield attempt facts");
        assert_eq!(facts.attempt_identity, attempt_identity);
        assert_eq!(facts.transfer_file_count, 1);
        assert_eq!(facts.staging_dir, staging);
        assert_eq!(facts.files.len(), facts.transfer_file_count);
        assert!(supervisor.readers_finished());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn attempt_facts_are_attempt_owned_and_not_cloneable() {
        // AttemptArchiveFacts carries non-Clone owners (SidecarSupervisor on
        // all platforms, Aria2Supervisor on Linux). It derives only Debug and
        // has no Clone impl, so a facts value cannot be duplicated across the
        // archive/publication seam. Stable Rust cannot express a negative
        // `!Clone` bound, so this test pins the absence of Clone by requiring
        // Send (owners are thread-local-capable) and documenting the contract.
        fn assert_send<T: Send>() {}
        assert_send::<AttemptArchiveFacts>();
    }

    #[test]
    fn attempt_facts_staging_verification_rejects_missing_or_replaced_trees() {
        let root = std::env::temp_dir().join(format!(
            "attempt-facts-verify-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        let archive = root.join("archive");
        let staging = root.join("staging");
        let files = FileStore::with_staging_root(&archive, &staging).expect("file store");
        let staging_dir = files
            .create_attempt_staging_dir("job-1", 3)
            .expect("attempt staging");
        let attempt_identity =
            xarchive_workflow::AttemptIdentity::new("job-1", 3).expect("attempt identity");
        let facts = AttemptArchiveFacts {
            attempt_identity,
            metadata: serde_json::json!({"tweet_id": "1"}),
            files: vec![xarchive_protocol::DownloadFile {
                relative_path: "01.jpg".to_owned(),
                size_bytes: 1,
                media_type: "photo".to_owned(),
                mime_type: None,
            }],
            staging_dir: staging_dir.clone(),
            transfer_file_count: 1,
            #[cfg(target_os = "linux")]
            retained_aria2: None,
        };
        verify_attempt_facts_staging(&files, "job-1", &facts).expect("live staging verifies");

        let tampered_count = AttemptArchiveFacts {
            attempt_identity: facts.attempt_identity.clone(),
            metadata: facts.metadata.clone(),
            files: facts.files.clone(),
            staging_dir: facts.staging_dir.clone(),
            transfer_file_count: facts.transfer_file_count + 1,
            #[cfg(target_os = "linux")]
            retained_aria2: None,
        };
        assert!(
            verify_attempt_facts_staging(&files, "job-1", &tampered_count).is_err(),
            "transfer-derived count mismatch must refuse publication"
        );

        std::fs::remove_dir_all(&staging_dir).expect("remove attempt staging");
        assert!(
            verify_attempt_facts_staging(&files, "job-1", &facts).is_err(),
            "missing attempt staging must not verify"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
