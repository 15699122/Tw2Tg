//! The batch dispatch coordinator and its worker lifetime.
//!
//! Split out of `batch.rs` without changing behaviour. The rules:
//!
//! * one pass submits at most `limit` candidates; a full queue stops the pass
//!   and leaves the remaining candidates PENDING, so a later round retries them
//!   instead of losing them;
//! * a submit failure is recorded per candidate and never retried implicitly;
//! * the cancellation token is unregistered by the guard's `Drop`, and a failed
//!   thread spawn unregisters it too, so a dead registration cannot block a
//!   later attempt;
//! * a batch whose state is no longer ACTIVE is never dispatched; a worker that
//!   starts late re-reads the durable state instead of trusting its launch
//!   arguments.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::Duration;

use xarchive_core::JobState;
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{Database, FileStore};

use crate::executor::CancellationToken;

use super::{
    BatchFilters, candidate_archive_request, discovery_request, inherited_execution_spec,
    metadata_batch_settings, run_account_discovery, select_dispatch_candidates, storage_error,
    truncate_message,
};

/// Result of one bounded dispatch pass. QueueFull is deliberately absent:
/// callers leave the candidate PENDING and retry later.
pub(crate) struct BatchDispatchPass {
    pub(crate) submitted: u64,
    pub(crate) completed: bool,
}

pub(in crate::batch) struct BatchCancellationGuard {
    pub(in crate::batch) cancellations: Arc<StdMutex<HashMap<String, CancellationToken>>>,
    pub(in crate::batch) batch_id: String,
    pub(in crate::batch) token: CancellationToken,
}

impl Drop for BatchCancellationGuard {
    fn drop(&mut self) {
        if let Ok(mut cancellations) = self.cancellations.lock()
            && cancellations
                .get(&self.batch_id)
                .is_some_and(|current| current.same_instance(&self.token))
        {
            cancellations.remove(&self.batch_id);
        }
    }
}

pub(crate) fn dispatch_batch_pass(
    database: &Database,
    files: &FileStore,
    service: &crate::executor::ArchiveApplicationService,
    database_path: &std::path::Path,
    batch_id: &str,
    limit: u32,
) -> Result<BatchDispatchPass, String> {
    let state = database
        .account_batch_state(batch_id)
        .map_err(|error| storage_error("failed to read batch state", error))?;
    if state.as_deref() != Some("ACTIVE") {
        return Ok(BatchDispatchPass {
            submitted: 0,
            completed: true,
        });
    }
    let output_settings_json = database
        .account_batch_output_settings_json(batch_id)
        .map_err(|error| storage_error("failed to read batch output snapshot", error))?
        .ok_or_else(|| format!("unknown batch: {batch_id}"))?;
    let downloader_arguments_json = database
        .account_batch_downloader_arguments_json(batch_id)
        .map_err(|error| storage_error("failed to read batch downloader snapshot", error))?
        .ok_or_else(|| format!("unknown batch: {batch_id}"))?;
    // Validate even when no candidate is pending so a corrupt immutable
    // snapshot cannot be silently carried through a resumed batch.
    let metadata_settings = metadata_batch_settings(&output_settings_json)?;
    if metadata_settings.is_none() {
        let _: crate::executor::OutputSettings = serde_json::from_str(&output_settings_json)
            .map_err(|error| format!("invalid persisted batch output settings: {error}"))?;
    }
    reconcile_batch_candidates(database, batch_id)?;
    let batch = database
        .account_batch(batch_id)
        .map_err(|error| storage_error("failed to read batch", error))?
        .ok_or_else(|| format!("unknown batch: {batch_id}"))?;
    let filters = BatchFilters::from_json(&batch.filters_json)?;
    let pending = select_dispatch_candidates(database, files, batch_id, &filters, limit)?;
    let mut submitted = 0;
    for candidate in pending {
        let request = candidate_archive_request(
            &candidate,
            batch.browser.as_deref(),
            batch.profile.as_deref(),
        )?;
        let mut prepared = crate::executor::ArchiveJobSubmissionAdapter
            .prepare(
                &request,
                &format!(
                    "batch-{batch_id}-{}-{}",
                    candidate.tweet_id,
                    crate::runtime::timestamp_marker()
                ),
            )
            .map_err(|error| error.to_string())?;
        let request_json =
            inherited_execution_spec(request, &output_settings_json, &downloader_arguments_json)?;
        prepared.schema_version = if metadata_settings.is_some() { 4 } else { 3 };
        prepared.request_json = request_json;
        // Each redispatch is a new request, including retries of the same
        // candidate after its previous Job reached a terminal state.
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| "batch request entropy unavailable".to_owned())?;
        let nonce = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        prepared.request_id = format!("batch-{batch_id}-{}-{nonce}", candidate.tweet_id);
        let mut persistence = crate::executor::StorageJobPersistence::open(database_path)
            .map_err(|error| format!("failed to open executor persistence: {error}"))?;
        match service.submit_and_schedule_persisted(
            &mut persistence,
            prepared,
            database_path.to_owned(),
        ) {
            Ok(result) => {
                database
                    .mark_batch_candidate(
                        batch_id,
                        &candidate.tweet_id,
                        "SUBMITTED",
                        Some(&result.job.job_id),
                        None,
                        None,
                        None,
                    )
                    .map_err(|error| storage_error("failed to mark submitted candidate", error))?;
                submitted += 1;
            }
            Err(crate::executor::ExecutorError::QueueFull) => break,
            Err(error) => {
                let message = truncate_message(&error.to_string());
                database
                    .mark_batch_candidate(
                        batch_id,
                        &candidate.tweet_id,
                        "FAILED",
                        None,
                        Some("BATCH_DISPATCH_FAILED"),
                        Some(&message),
                        None,
                    )
                    .map_err(|storage| {
                        storage_error("failed to record dispatch failure", storage)
                    })?;
            }
        }
    }
    reconcile_batch_candidates(database, batch_id)?;
    let counts = database
        .batch_candidate_counts(batch_id)
        .map_err(|error| storage_error("failed to count batch candidates", error))?;
    let completed = counts.pending == 0 && counts.submitted == 0;
    if completed {
        database
            .set_account_batch_state(batch_id, "COMPLETED")
            .map_err(|error| storage_error("failed to complete batch", error))?;
    }
    Ok(BatchDispatchPass {
        submitted,
        completed,
    })
}

pub(in crate::batch) fn reconcile_batch_candidates(
    database: &Database,
    batch_id: &str,
) -> Result<(), String> {
    let submitted = database
        .list_batch_candidates(batch_id, Some("SUBMITTED"), 100)
        .map_err(|error| storage_error("failed to list submitted candidates", error))?;
    for candidate in submitted {
        let Some(job_id) = candidate.job_id.as_deref() else {
            database
                .mark_batch_candidate(
                    batch_id,
                    &candidate.tweet_id,
                    "FAILED",
                    None,
                    Some("BATCH_JOB_ID_MISSING"),
                    Some("submitted candidate has no executor job id"),
                    None,
                )
                .map_err(|error| storage_error("failed to reconcile missing job id", error))?;
            continue;
        };
        let Some(job) = database
            .job_summary(job_id)
            .map_err(|error| storage_error("failed to read candidate job", error))?
        else {
            database
                .mark_batch_candidate(
                    batch_id,
                    &candidate.tweet_id,
                    "FAILED",
                    Some(job_id),
                    Some("BATCH_JOB_MISSING"),
                    Some("executor job no longer exists"),
                    None,
                )
                .map_err(|error| storage_error("failed to reconcile missing job", error))?;
            continue;
        };
        match job.state {
            JobState::Complete => database
                .mark_batch_candidate(
                    batch_id,
                    &candidate.tweet_id,
                    "DONE",
                    Some(job_id),
                    None,
                    None,
                    None,
                )
                .map_err(|error| storage_error("failed to mark completed candidate", error))?,
            JobState::Failed | JobState::Cancelled => database
                .mark_batch_candidate(
                    batch_id,
                    &candidate.tweet_id,
                    "FAILED",
                    Some(job_id),
                    job.last_error_code.as_deref(),
                    job.last_error_message.as_deref(),
                    None,
                )
                .map_err(|error| storage_error("failed to mark failed candidate", error))?,
            JobState::AuthRequired => {
                database
                    .set_account_batch_state(batch_id, "PAUSED")
                    .map_err(|error| storage_error("failed to pause batch for auth", error))?;
            }
            _ => {}
        }
    }
    Ok(())
}
/// Start the detached batch coordinator. Commands return immediately; all
/// progress is durable and a restart can safely resume the same batch.
pub(crate) fn spawn_batch_dispatch(
    service: crate::executor::ArchiveApplicationService,
    database_path: std::path::PathBuf,
    files: FileStore,
    batch_id: String,
    cancellations: Arc<StdMutex<HashMap<String, CancellationToken>>>,
) -> Result<(), String> {
    let token = CancellationToken::new();
    let mut registered = cancellations
        .lock()
        .map_err(|_| "batch cancellation registry poisoned".to_owned())?;
    if registered.contains_key(&batch_id) {
        return Err("batch worker is still stopping".to_owned());
    }
    registered.insert(batch_id.clone(), token.clone());
    drop(registered);
    let cleanup_cancellations = cancellations.clone();
    let cleanup_batch_id = batch_id.clone();
    let cleanup_token = token.clone();
    thread::Builder::new()
        .name(format!("xarchive-batch-{batch_id}"))
        .spawn(move || {
            let _guard = BatchCancellationGuard {
                cancellations: cancellations.clone(),
                batch_id: batch_id.clone(),
                token: token.clone(),
            };
            let Ok(database) = Database::open(&database_path) else {
                return;
            };
            loop {
                match dispatch_batch_pass(
                    &database,
                    &files,
                    &service,
                    &database_path,
                    &batch_id,
                    16,
                ) {
                    Ok(pass) if pass.completed => break,
                    Ok(_) => thread::sleep(Duration::from_millis(250)),
                    Err(_) => break,
                }
            }
        })
        .map(|_| ())
        .map_err(|error| {
            if let Ok(mut tokens) = cleanup_cancellations.lock()
                && tokens
                    .get(&cleanup_batch_id)
                    .is_some_and(|current| current.same_instance(&cleanup_token))
            {
                tokens.remove(&cleanup_batch_id);
            }
            format!("failed to spawn batch coordinator: {error}")
        })
}

/// Run discovery and then hand the durable candidates to the bounded executor
/// coordinator. The Sidecar process is private to this worker.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_account_batch(
    service: crate::executor::ArchiveApplicationService,
    database_path: std::path::PathBuf,
    files: FileStore,
    batch_id: String,
    sidecar_program: String,
    sidecar_args: Vec<String>,
    sidecar_env: Vec<(String, String)>,
    sidecar_env_remove: Vec<String>,
    discovery_timeout: Duration,
    cancellation: CancellationToken,
    cancellations: Arc<StdMutex<HashMap<String, CancellationToken>>>,
) -> Result<(), String> {
    let mut registered = cancellations
        .lock()
        .map_err(|_| "batch cancellation registry poisoned".to_owned())?;
    if registered.contains_key(&batch_id) {
        return Err("batch worker is still stopping".to_owned());
    }
    registered.insert(batch_id.clone(), cancellation.clone());
    drop(registered);
    let cleanup_cancellations = cancellations.clone();
    let cleanup_batch_id = batch_id.clone();
    let cleanup_token = cancellation.clone();
    thread::Builder::new()
        .name(format!("xarchive-batch-discovery-{batch_id}"))
        .spawn(move || {
            let _guard = BatchCancellationGuard {
                cancellations: cancellations.clone(),
                batch_id: batch_id.clone(),
                token: cancellation.clone(),
            };
            let Ok(database) = Database::open(&database_path) else {
                return;
            };
            let Ok(Some(batch)) = database.account_batch(&batch_id) else {
                return;
            };
            if batch.state != "ACTIVE" {
                return;
            };
            let arg_refs = sidecar_args.iter().map(String::as_str).collect::<Vec<_>>();
            // The removals travel with the additions so a `Direct` batch worker
            // cannot inherit a proxy from the launching shell.
            let mut supervisor = match SidecarSupervisor::spawn_ready_v2_with_environment(
                &sidecar_program,
                &arg_refs,
                &xarchive_sidecar_supervisor::ChildProcessEnvironment::new(
                    sidecar_env,
                    sidecar_env_remove,
                ),
                Duration::from_secs(5),
            ) {
                Ok(supervisor) => supervisor,
                Err(error) => {
                    let stopped_state = database
                        .account_batch_state(&batch_id)
                        .ok()
                        .flatten()
                        .filter(|state| matches!(state.as_str(), "PAUSED" | "CANCELLED"));
                    if cancellation.is_cancelled() || stopped_state.is_some() {
                        return;
                    }
                    let _ = database.set_account_batch_error(
                        &batch_id,
                        Some("SIDECAR_START_FAILED"),
                        Some(&truncate_message(&error.to_string())),
                    );
                    let _ = database.set_account_batch_discovery_state(&batch_id, "FAILED");
                    return;
                }
            };
            let request = discovery_request(&batch);
            if run_account_discovery(
                &database,
                &mut supervisor,
                &request,
                &cancellation,
                discovery_timeout,
            )
            .is_err()
            {
                return;
            }
            drop(supervisor);
            loop {
                match dispatch_batch_pass(
                    &database,
                    &files,
                    &service,
                    &database_path,
                    &batch_id,
                    16,
                ) {
                    Ok(pass) if pass.completed => break,
                    Ok(_) => thread::sleep(Duration::from_millis(250)),
                    Err(_) => break,
                }
            }
        })
        .map(|_| ())
        .map_err(|error| {
            if let Ok(mut tokens) = cleanup_cancellations.lock()
                && tokens
                    .get(&cleanup_batch_id)
                    .is_some_and(|current| current.same_instance(&cleanup_token))
            {
                tokens.remove(&cleanup_batch_id);
            }
            format!("failed to spawn account batch worker: {error}")
        })
}
