use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::ArchiveTweetRequest;
use crate::executor::{
    CancellationToken, JobExecution, JobExecutionError, JobExecutionResult, JobSnapshot,
};
use xarchive_storage::{ArchiveService, BatchOutputSettings, FileStore};

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

/// Move-only pending archive publication: owns the Storage half and carries all
/// facts needed to call the crash-safe
/// [`xarchive_storage::ArchiveService::publish_internal_v1_attempt`]. The
/// value is consumed by `finalize` so publication facts can never be replayed.
/// CROSS_PLATFORM_CHANGE_REQUIRED (ADR-015 clause 8): the runner receives the
/// same Storage/process owners without transferring a borrowed session.
pub struct PendingArchivePublication {
    service: Box<xarchive_storage::ArchiveService>,
    attempt_identity: xarchive_workflow::AttemptIdentity,
    job_id: String,
    tweet_row_id: i64,
    expected_tweet_id: String,
    metadata: serde_json::Value,
    files: Vec<xarchive_protocol::DownloadFile>,
    final_directory: std::path::PathBuf,
    archived_at: String,
    output_settings: BatchOutputSettings,
    /// Frozen D1 selection for this attempt. `None` means the job predates v4
    /// settings and keeps the historical unconditional export behaviour.
    metadata_output: Option<crate::executor::MetadataOutputSettings>,
    files_copied: u32,
    telegram: crate::config::TelegramConfig,
    finalized: bool,
    _sidecar_owner: SidecarSupervisor,
    #[cfg(target_os = "linux")]
    _retained_aria2: Option<xarchive_download::Aria2Supervisor>,
}

impl PendingArchivePublication {
    /// Freeze verified attempt facts and transfer the exact execution-owned
    /// Storage and process handles into the runner's move-only pending value.
    fn from_verified_facts(
        context: ArchiveExecutionContext,
        facts: AttemptArchiveFacts,
        request: &ArchiveTweetRequest,
        job: &JobSnapshot,
        output_settings: BatchOutputSettings,
        tweet_row_id: i64,
        archived_at: &str,
        metadata_output: Option<crate::executor::MetadataOutputSettings>,
    ) -> Result<Self, (ArchiveExecutionContext, JobExecutionError)> {
        let AttemptArchiveFacts {
            attempt_identity,
            metadata,
            files,
            staging_dir: _,
            transfer_file_count,
            #[cfg(target_os = "linux")]
                retained_aria2: facts_retained_aria2,
        } = facts;
        #[cfg(target_os = "linux")]
        if facts_retained_aria2.is_some() {
            return Err((
                context,
                JobExecutionError {
                    error_code: "ARCHIVE_WRITER_DRAIN_FAILED".to_owned(),
                    error_message: "retained aria2 owner is not drained".to_owned(),
                    persistence_already_updated: false,
                },
            ));
        }
        if job.job_id != attempt_identity.job_id()
            || job.tweet_id != request.tweet.tweet_id
            || attempt_identity.attempt() == 0
        {
            return Err((
                context,
                JobExecutionError {
                    error_code: "PUBLICATION_IDENTITY_MISMATCH".to_owned(),
                    error_message: "archive request does not match the claimed attempt".to_owned(),
                    persistence_already_updated: false,
                },
            ));
        }
        let telegram = context.completion_telegram_config();
        let (database, file_store, supervisor, _aria2_program, _use_aria2, retained_aria2) =
            context.into_parts();
        let service = Box::new(ArchiveService::new(database, file_store));
        let final_directory = PathBuf::from("Tweets").join(&request.tweet.tweet_id);
        Ok(Self {
            service,
            attempt_identity,
            job_id: job.job_id.clone(),
            tweet_row_id,
            expected_tweet_id: request.tweet.tweet_id.clone(),
            metadata,
            files,
            final_directory,
            archived_at: archived_at.to_owned(),
            output_settings,
            metadata_output,
            files_copied: transfer_file_count as u32,
            telegram,
            finalized: false,
            _sidecar_owner: supervisor,
            #[cfg(target_os = "linux")]
            _retained_aria2: retained_aria2,
        })
    }

    /// Bridges the runner-owned execution to the crash-safe storage commit.
    ///
    /// Consumes the pending publication exactly once through the same Storage
    /// owner while the runner still holds the live session and writer owner.
    pub(crate) fn finalize<C: xarchive_workflow::coordination::Coordinator>(
        &mut self,
        session: &xarchive_workflow::ExecutionSession<'_, C>,
        publication_owner: &xarchive_workflow::PublicationOwner,
        cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        if std::mem::replace(&mut self.finalized, true) {
            return Err(JobExecutionError {
                error_code: "PUBLICATION_ALREADY_FINALIZED".to_owned(),
                error_message: "pending publication may only be finalized once".to_owned(),
                persistence_already_updated: false,
            });
        }
        if cancellation.is_cancelled() {
            return Err(JobExecutionError {
                error_code: "TRANSFER_CANCELLED".to_owned(),
                error_message: "archive publication was cancelled before PREPARED".to_owned(),
                persistence_already_updated: false,
            });
        }
        if session.job_id() != self.job_id
            || session.attempt().job_id() != self.job_id
            || session.attempt() != &self.attempt_identity
            || session.destination()
                != self
                    .service
                    .files
                    .archive_path(&self.final_directory)
                    .map_err(|error| JobExecutionError {
                        error_code: "ARCHIVE_DESTINATION_INVALID".to_owned(),
                        error_message: error.to_string(),
                        persistence_already_updated: false,
                    })?
        {
            return Err(JobExecutionError {
                error_code: "PUBLICATION_IDENTITY_MISMATCH".to_owned(),
                error_message: "pending archive does not match the live session".to_owned(),
                persistence_already_updated: false,
            });
        }
        publication_owner
            .close_admission()
            .map_err(|error| JobExecutionError {
                error_code: "PUBLICATION_DRAIN_INCOMPLETE".to_owned(),
                error_message: format!("writer admission could not be closed: {error:?}"),
                persistence_already_updated: false,
            })?;
        let permit = match publication_owner.authorize(session) {
            Ok(permit) => permit,
            Err(error) => {
                return Err(match error {
                    xarchive_workflow::PublicationError::AttemptMismatch => JobExecutionError {
                        error_code: "PUBLICATION_ATTEMPT_MISMATCH".to_owned(),
                        error_message: "publication attempt mismatch".to_owned(),
                        persistence_already_updated: false,
                    },
                    xarchive_workflow::PublicationError::DestinationMismatch => JobExecutionError {
                        error_code: "PUBLICATION_DESTINATION_MISMATCH".to_owned(),
                        error_message: "publication destination mismatch".to_owned(),
                        persistence_already_updated: false,
                    },
                    xarchive_workflow::PublicationError::IncompleteDrain => JobExecutionError {
                        error_code: "PUBLICATION_INCOMPLETE_DRAIN".to_owned(),
                        error_message: "publication incomplete drain".to_owned(),
                        persistence_already_updated: false,
                    },
                    xarchive_workflow::PublicationError::SessionMismatch => JobExecutionError {
                        error_code: "PUBLICATION_SESSION_MISMATCH".to_owned(),
                        error_message: "publication session mismatch".to_owned(),
                        persistence_already_updated: false,
                    },
                    xarchive_workflow::PublicationError::AlreadyPublished => JobExecutionError {
                        error_code: "PUBLICATION_ALREADY_PUBLISHED".to_owned(),
                        error_message: "publication already published".to_owned(),
                        persistence_already_updated: false,
                    },
                    _ => JobExecutionError {
                        error_code: "PUBLICATION_AUTHORIZATION_REJECTED".to_owned(),
                        error_message: "publication authorization rejected".to_owned(),
                        persistence_already_updated: false,
                    },
                });
            }
        };
        let attempt_count = session.attempt().attempt();
        let preview = self
            .service
            .preview_sidecar_archive_attempt(
                &self.job_id,
                attempt_count,
                &self.expected_tweet_id,
                &self.metadata,
                &self.files,
                &self.archived_at,
                &self.output_settings,
            )
            .map_err(|error| JobExecutionError {
                error_code: "ARCHIVE_PREVIEW_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            })?;
        let telegram_intent = if crate::telegram_send::auto_send_enabled(&self.telegram) {
            self.service
                .database
                .active_telegram_credential_generation()
                .ok()
                .flatten()
                .and_then(|(_, bot, _)| {
                    crate::telegram_send::ArchiveSendIntent::from_archive_metadata(
                        self.tweet_row_id,
                        self.final_directory.to_string_lossy().into_owned(),
                        &preview.metadata,
                        &self.telegram,
                        &bot,
                    )
                    .ok()
                })
                .and_then(|intent| {
                    intent
                        .to_journal_record(&self.job_id, &self.archived_at)
                        .ok()
                })
        } else {
            None
        };
        // D1: serialize the selected user-facing formats from the same canonical
        // DTO that the internal recovery manifest is built from. A serialization
        // failure here aborts before any file mutation or PREPARED journal, so a
        // half-written export can never be published.
        let metadata_exports = match self.metadata_output.as_ref() {
            None => None,
            Some(settings) => Some(settings.serialize_metadata(&preview.metadata).map_err(
                |error| JobExecutionError {
                    error_code: "ARCHIVE_METADATA_EXPORT_FAILED".to_owned(),
                    error_message: error,
                    persistence_already_updated: false,
                },
            )?),
        };
        let request = xarchive_storage::SidecarArchiveRequest {
            job_id: &self.job_id,
            tweet_row_id: self.tweet_row_id,
            expected_tweet_id: &self.expected_tweet_id,
            metadata: &self.metadata,
            files: &self.files,
            final_directory: &self.final_directory,
            archived_at: &self.archived_at,
            telegram_intent: telegram_intent.as_ref(),
            output_settings: &self.output_settings,
            metadata_exports: metadata_exports.as_deref(),
        };
        let archive_directory = self.final_directory.to_string_lossy().to_string();
        if let Some(intent) = &telegram_intent {
            self.service
                .database
                .prepare_telegram_archive_intent(intent)
                .map_err(|error| JobExecutionError {
                    error_code: "TELEGRAM_INTENT_PREPARE_FAILED".to_owned(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                })?;
        }
        self.service
            .publish_internal_v1_attempt(request, attempt_count, permit)
            .map(|_| JobExecutionResult {
                files_copied: self.files_copied,
                archive_directory,
                download_event_recorded: false,
                state_already_updated: false,
                pending_publication: None,
            })
            .map_err(|error| JobExecutionError {
                error_code: "ARCHIVE_PUBLISH_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            })
    }
}

impl std::fmt::Debug for PendingArchivePublication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingArchivePublication")
            .field("attempt_identity", &self.attempt_identity)
            .field("job_id", &self.job_id)
            .field("tweet_row_id", &self.tweet_row_id)
            .field("expected_tweet_id", &self.expected_tweet_id)
            .field("metadata", &self.metadata)
            .field("files", &self.files)
            .field("final_directory", &self.final_directory)
            .field("archived_at", &self.archived_at)
            .field("output_settings", &self.output_settings)
            .field("files_copied", &self.files_copied)
            .field("telegram", &self.telegram)
            .field("finalized", &self.finalized)
            .field("finalized", &self.finalized)
            .finish()
    }
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
    pub(crate) publication_registration: Option<xarchive_workflow::WriterRegistration>,
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
                if let Some(context) = context {
                    if let Ok(mut stored) = self.context.lock() {
                        *stored = Some(context);
                    }
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
    _output_settings: &xarchive_storage::BatchOutputSettings,
    metadata_output: Option<&crate::executor::MetadataOutputSettings>,
    job: &JobSnapshot,
    _tweet_row_id: i64,
    request_id: &str,
    archived_at: &str,
    cancellation: &CancellationToken,
) -> Result<
    (Option<ArchiveExecutionContext>, JobExecutionResult),
    Box<(ArchiveExecutionContext, JobExecutionError)>,
> {
    let Some(_attempt_identity) = context
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
    let pending = match PendingArchivePublication::from_verified_facts(
        context,
        facts,
        request,
        job,
        _output_settings.clone(),
        _tweet_row_id,
        archived_at,
        metadata_output.cloned(),
    ) {
        Ok(pending) => pending,
        Err((context, error)) => return Err(Box::new((context, error))),
    };
    Ok((
        None,
        JobExecutionResult {
            files_copied: 0,
            archive_directory: PathBuf::from("Tweets")
                .join(&request.tweet.tweet_id)
                .to_string_lossy()
                .to_string(),
            download_event_recorded: false,
            state_already_updated: false,
            pending_publication: Some(Box::new(pending)),
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

    fn completion_telegram_config(&self) -> crate::config::TelegramConfig {
        sample_completion_config(self.telegram_config_file.as_deref(), &self.telegram)
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
            publication_registration: None,
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

    mod attempt_staging;
    mod completion_settings;
    mod writer_drain;
}
