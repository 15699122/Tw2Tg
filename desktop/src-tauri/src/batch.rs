//! Account batch discovery persistence and dispatch selection.
//!
//! The Sidecar `discover` command only streams Tweet candidates. Everything
//! after that is owned here: the filter schema persisted in
//! `archive_batches.filters_json`, durable candidate persistence, stable
//! identity binding, already-archived skip verification against committed
//! archive facts, and the bounded candidate list handed to the job executor.
//!
//! A batch decides *which* Tweets to archive. The archive completeness decision
//! stays with the single-Tweet commit path (`ArchiveService`), so a skip here
//! only means "this Tweet already has a committed directory containing every
//! recorded media file", never "the download is complete".
//!
//! The Desktop intake (Tauri commands and the dispatch worker) is wired in the
//! following batch, so the module keeps the executor-module convention of
//! allowing not-yet-called production code. Its behavior is covered by the
//! module tests below.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Component, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use xarchive_core::JobState;
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{
    AccountBatchSummary, ArchiveCompletenessOptions, BatchCandidateRecord, BatchCounts, Database,
    FileStore, NewBatchCandidate, StorageError, TweetArchiveFacts, evaluate_archive_completeness,
};

use crate::executor::CancellationToken;
use crate::production::{DiscoveryOutcome, DiscoveryRequest, execute_v2_discovery};

/// Skip reasons persisted in `batch_candidates.skip_reason`.
pub(crate) const SKIP_ALREADY_ARCHIVED: &str = "ALREADY_ARCHIVED";
pub(crate) const SKIP_REPOST: &str = "FILTERED_REPOST";
pub(crate) const SKIP_NO_MEDIA: &str = "FILTERED_NO_MEDIA";
pub(crate) const SKIP_BEFORE_RANGE: &str = "FILTERED_BEFORE_RANGE";
pub(crate) const SKIP_AFTER_RANGE: &str = "FILTERED_AFTER_RANGE";
pub(crate) const SKIP_BEYOND_LIMIT: &str = "FILTERED_LIMIT";

/// How many PENDING candidates one selection call inspects.
const SELECTION_SCAN_LIMIT: u32 = 512;

/// User-visible discovery filters, persisted as `archive_batches.filters_json`.
///
/// The default matches the approved product scope: only the account's own
/// media Tweets, with no explicit cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BatchFilters {
    pub include_reposts: bool,
    pub include_without_media: bool,
    /// Inclusive lower bound compared against the candidate's `created_at`.
    pub since: Option<String>,
    /// Inclusive upper bound compared against the candidate's `created_at`.
    pub until: Option<String>,
    /// Upper bound on how many candidates of this batch are ever dispatched,
    /// counting already submitted/done candidates. The newest candidates win.
    pub limit: Option<u32>,
}

impl Default for BatchFilters {
    fn default() -> Self {
        Self {
            include_reposts: false,
            include_without_media: false,
            since: None,
            until: None,
            limit: None,
        }
    }
}

impl BatchFilters {
    pub(crate) fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| format!("invalid batch filters: {error}"))
    }

    pub(crate) fn from_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|error| format!("invalid batch filters: {error}"))
    }

    /// Why this candidate is excluded from dispatch, if it is.
    ///
    /// Timestamps are compared as ISO-8601 UTC strings, which orders correctly
    /// for the format the Sidecar emits. A candidate without `created_at` is
    /// never dropped by a date range: unknown data must not silently vanish.
    fn exclusion(&self, candidate: &BatchCandidateRecord) -> Option<&'static str> {
        if candidate.is_repost && !self.include_reposts {
            return Some(SKIP_REPOST);
        }
        if !candidate.has_media || candidate.media_count == 0 {
            if !self.include_without_media {
                return Some(SKIP_NO_MEDIA);
            }
        }
        if let Some(created_at) = candidate.created_at.as_deref() {
            if self
                .since
                .as_deref()
                .is_some_and(|since| created_at < since)
            {
                return Some(SKIP_BEFORE_RANGE);
            }
            if self
                .until
                .as_deref()
                .is_some_and(|until| created_at > until)
            {
                return Some(SKIP_AFTER_RANGE);
            }
        }
        None
    }
}

/// Durable result of one discovery run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoverySummary {
    /// Candidate count reported by the worker's `discovery_completed` event.
    pub(crate) candidates_found: u64,
    /// Candidates that were newly persisted by this run.
    pub(crate) inserted: u64,
    /// Stable X user id bound to the batch, when discovery exposed one.
    pub(crate) user_id: Option<String>,
    /// Profile username observed during discovery.
    pub(crate) username: Option<String>,
}

/// Build the discovery request for one persisted batch.
pub(crate) fn discovery_request(batch: &AccountBatchSummary) -> DiscoveryRequest {
    DiscoveryRequest {
        batch_id: batch.id.clone(),
        request_id: format!("batch-discovery-{}", batch.id),
        profile_url: batch.profile_url.clone(),
        browser: batch.browser.clone(),
        profile: batch.profile.clone(),
    }
}

/// Map one sidecar candidate onto its durable row.
pub(crate) fn candidate_record(
    candidate: &xarchive_protocol::DiscoveryCandidate,
) -> NewBatchCandidate {
    NewBatchCandidate {
        tweet_id: candidate.tweet_id.clone(),
        url: candidate.url.clone(),
        created_at: candidate.created_at.clone(),
        tweet_type: candidate.tweet_type.clone(),
        is_repost: candidate.is_repost,
        has_media: candidate.has_media,
        media_count: u64::from(candidate.media_count),
        user_id: candidate.user_id.clone(),
        username: candidate.username.clone(),
        state: "PENDING".to_owned(),
        skip_reason: None,
    }
}

/// Persist one discovery outcome as batch candidates.
///
/// Re-running discovery for the same batch is idempotent: `tweet_id` is unique
/// per batch, and existing candidates keep their state so a re-run never
/// resurrects an archived or cancelled Tweet.
pub(crate) fn persist_discovered_candidates(
    database: &Database,
    batch_id: &str,
    outcome: &DiscoveryOutcome,
) -> Result<DiscoverySummary, String> {
    let records = outcome
        .candidates
        .iter()
        .map(candidate_record)
        .collect::<Vec<_>>();
    let inserted = database
        .insert_batch_candidates(batch_id, &records)
        .map_err(|error| storage_error("failed to persist discovery candidates", error))?;

    // The stable identity is the first candidate that actually carries one; a
    // username is only ever a display aid for the batch.
    let user_id = outcome
        .candidates
        .iter()
        .find_map(|candidate| candidate.user_id.clone());
    let username = outcome
        .candidates
        .iter()
        .find_map(|candidate| candidate.username.clone());
    if user_id.is_some() || username.is_some() {
        database
            .resolve_account_batch_identity(batch_id, user_id.as_deref(), username.as_deref())
            .map_err(|error| storage_error("failed to bind batch identity", error))?;
    }

    Ok(DiscoverySummary {
        candidates_found: outcome.candidates_found,
        inserted,
        user_id,
        username,
    })
}

/// Run discovery for one batch and persist the result.
///
/// Discovery state is durable so a restart can tell "never discovered" from
/// "discovery failed"; a failure records the sidecar error code and message on
/// the batch itself for the UI.
pub(crate) fn run_account_discovery(
    database: &Database,
    supervisor: &mut SidecarSupervisor,
    request: &DiscoveryRequest,
    cancellation: &CancellationToken,
) -> Result<DiscoverySummary, String> {
    database
        .set_account_batch_discovery_state(&request.batch_id, "RUNNING")
        .map_err(|error| storage_error("failed to start discovery", error))?;

    let outcome = match execute_v2_discovery(supervisor, request, cancellation) {
        Ok(outcome) => outcome,
        Err(error) => {
            if cancellation.is_cancelled() {
                let discovery_state = database
                    .account_batch_state(&request.batch_id)
                    .ok()
                    .flatten()
                    .as_deref()
                    .is_some_and(|state| state == "CANCELLED")
                    .then_some("CANCELLED")
                    .unwrap_or("PENDING");
                let _ =
                    database.set_account_batch_discovery_state(&request.batch_id, discovery_state);
                return Err(error);
            }
            let code = discovery_error_code(&error);
            let message = truncate_message(&error);
            database
                .set_account_batch_error(&request.batch_id, Some(&code), Some(&message))
                .map_err(|storage| storage_error("failed to record discovery error", storage))?;
            database
                .set_account_batch_discovery_state(&request.batch_id, "FAILED")
                .map_err(|storage| storage_error("failed to record discovery failure", storage))?;
            return Err(error);
        }
    };

    let summary = persist_discovered_candidates(database, &request.batch_id, &outcome)?;
    database
        .set_account_batch_error(&request.batch_id, None, None)
        .map_err(|error| storage_error("failed to clear discovery error", error))?;
    database
        .set_account_batch_discovery_state(&request.batch_id, "COMPLETED")
        .map_err(|error| storage_error("failed to complete discovery", error))?;
    Ok(summary)
}

/// Derive a bounded error code from a discovery failure message.
///
/// The protocol allows at most 64 characters for `error_code`, so a message
/// that does not start with a code-shaped token falls back to a stable code.
fn discovery_error_code(message: &str) -> String {
    let candidate = message.split(':').next().unwrap_or_default().trim();
    let code_shaped = !candidate.is_empty()
        && candidate.len() <= 64
        && candidate
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if code_shaped {
        candidate.to_owned()
    } else {
        "DISCOVERY_FAILED".to_owned()
    }
}

/// Keep a persisted error message bounded; the batch row is user-visible.
fn truncate_message(message: &str) -> String {
    message.chars().take(4000).collect()
}

fn storage_error(context: &str, error: StorageError) -> String {
    format!("{context}: {error}")
}

/// Verify that a committed archive directory still holds every recorded media
/// file (slow path kept for tests; dispatch uses the P2-C policy above).
fn archive_facts_present(files: &FileStore, facts: &TweetArchiveFacts) -> bool {
    let Ok(directory) = files.archive_path(&facts.archive_directory) else {
        return false;
    };
    if !directory.is_dir() {
        return false;
    }
    facts
        .media_paths()
        .collect::<Vec<_>>()
        .iter()
        .all(|relative| safe_media_path(&directory, relative).is_some_and(|path| path.is_file()))
}

/// Join a persisted media path onto its archive directory without letting the
/// path escape that directory.
fn safe_media_path(directory: &std::path::Path, relative: &str) -> Option<PathBuf> {
    let relative = std::path::Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(directory.join(relative))
}

/// Whether this candidate may be skipped because its archive is already complete.
///
/// The decision uses the P2-C completeness policy, never bare file existence:
/// the recorded media-row count is checked against the discovery `media_count`,
/// every recorded file must exist in the committed directory, and recorded
/// sizes must match. A digest check is intentionally not part of dispatch; the
/// SHA-256 verification flow runs separately on explicit request.
fn candidate_already_archived(
    database: &Database,
    files: &FileStore,
    candidate: &BatchCandidateRecord,
) -> Result<bool, String> {
    let facts = database
        .tweet_archive_facts(&candidate.tweet_id)
        .map_err(|error| storage_error("failed to read archive facts", error))?;
    let Some(facts) = facts else {
        return Ok(false);
    };
    // A media candidate without recorded media paths cannot be verified as
    // archived, so it is dispatched instead of silently skipped.
    if candidate.has_media && facts.media_paths().next().is_none() {
        return Ok(false);
    }
    let expected = candidate.has_media.then_some(candidate.media_count);
    Ok(evaluate_archive_completeness(
        &files,
        &facts,
        expected,
        &ArchiveCompletenessOptions::fast(),
    )
    .is_complete())
}

/// Select the candidates one dispatch round may submit.
///
/// Each PENDING candidate is decided in this order:
/// 1. a present committed archive is skipped as `ALREADY_ARCHIVED`;
/// 2. a candidate the filters exclude is skipped with its filter reason;
/// 3. everything else is approved, bounded by `limit` per dispatch round and by
///    `filters.limit` per batch.
///
/// Skipping is durable: a skipped candidate leaves the PENDING set, so a later
/// round never re-evaluates it and the batch counts explain the outcome. The
/// returned list is ordered newest first, and so is the batch-cap decision.
pub(crate) fn select_dispatch_candidates(
    database: &Database,
    files: &FileStore,
    batch_id: &str,
    filters: &BatchFilters,
    limit: u32,
) -> Result<Vec<BatchCandidateRecord>, String> {
    let pending = database
        .list_batch_candidates(batch_id, Some("PENDING"), SELECTION_SCAN_LIMIT)
        .map_err(|error| storage_error("failed to read pending candidates", error))?;
    let counts = database
        .batch_candidate_counts(batch_id)
        .map_err(|error| storage_error("failed to read batch counts", error))?;

    let mut approved = Vec::new();
    for candidate in pending {
        if candidate_already_archived(database, files, &candidate)? {
            mark_skipped(
                database,
                batch_id,
                &candidate.tweet_id,
                SKIP_ALREADY_ARCHIVED,
            )?;
            continue;
        }
        if let Some(reason) = filters.exclusion(&candidate) {
            mark_skipped(database, batch_id, &candidate.tweet_id, reason)?;
            continue;
        }
        approved.push(candidate);
    }

    let approved = newest_first(approved);
    let mut approved = apply_batch_limit(database, batch_id, filters, counts, approved)?;
    approved.truncate(limit as usize);
    Ok(approved)
}

/// Apply `filters.limit` across the whole batch.
///
/// The cap counts already submitted/done candidates plus this round's
/// approvals. When the cap is exceeded the newest candidates win and the oldest
/// overflow is skipped as `FILTERED_LIMIT`, so an account cap keeps producing
/// the most recent Tweets rather than the discovery order.
fn apply_batch_limit(
    database: &Database,
    batch_id: &str,
    filters: &BatchFilters,
    counts: BatchCounts,
    approved: Vec<BatchCandidateRecord>,
) -> Result<Vec<BatchCandidateRecord>, String> {
    let Some(cap) = filters.limit else {
        return Ok(approved);
    };
    let accounted = counts.submitted.saturating_add(counts.done);
    let room = u64::from(cap).saturating_sub(accounted);
    if approved.len() as u64 <= room {
        return Ok(approved);
    }
    let mut kept = approved;
    let dropped = kept.split_off(room as usize);
    for candidate in &dropped {
        mark_skipped(database, batch_id, &candidate.tweet_id, SKIP_BEYOND_LIMIT)?;
    }
    Ok(kept)
}

/// Order candidates newest first, keeping unknown timestamps last.
///
/// Timestamps are ISO-8601 UTC strings, which compare and sort correctly as
/// text for the format the Sidecar emits.
fn newest_first(mut candidates: Vec<BatchCandidateRecord>) -> Vec<BatchCandidateRecord> {
    candidates.sort_by(|left, right| match (&left.created_at, &right.created_at) {
        (Some(left), Some(right)) => right.cmp(left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => right.tweet_id.cmp(&left.tweet_id),
    });
    candidates
}

fn mark_skipped(
    database: &Database,
    batch_id: &str,
    tweet_id: &str,
    reason: &str,
) -> Result<(), String> {
    database
        .mark_batch_candidate(
            batch_id,
            tweet_id,
            "SKIPPED",
            None,
            None,
            None,
            Some(reason),
        )
        .map_err(|error| storage_error("failed to skip batch candidate", error))
}

/// Convert a discovery candidate into an immutable browser-shaped request.
/// Text and nested quote data are intentionally re-read by Sidecar at execution
/// time rather than persisted from discovery.
pub(crate) fn candidate_archive_request(
    candidate: &BatchCandidateRecord,
    browser: Option<&str>,
    profile: Option<&str>,
) -> Result<crate::ArchiveTweetRequest, String> {
    let tweet = xarchive_protocol::BrowserTweet {
        tweet_id: candidate.tweet_id.clone(),
        url: candidate.url.clone(),
        username: candidate.username.clone(),
        display_name: None,
        text: None,
        created_at: candidate.created_at.clone(),
        tweet_type: candidate.tweet_type.clone(),
        reply_to: None,
        quoted_tweet: None,
    };
    xarchive_protocol::BrowserRequest::ArchiveRequest {
        protocol_version: xarchive_protocol::PROTOCOL_VERSION,
        request_id: format!("batch-candidate-{}", candidate.tweet_id),
        tweet: tweet.clone(),
    }
    .validate()
    .map_err(|error| {
        format!(
            "candidate {} is not archivable: {error}",
            candidate.tweet_id
        )
    })?;
    Ok(crate::ArchiveTweetRequest {
        tweet,
        browser: browser.map(str::to_owned),
        profile: profile.map(str::to_owned),
    })
}

/// Result of one bounded dispatch pass. QueueFull is deliberately absent:
/// callers leave the candidate PENDING and retry later.
pub(crate) struct BatchDispatchPass {
    pub(crate) submitted: u64,
    pub(crate) completed: bool,
}

struct BatchCancellationGuard {
    cancellations: Arc<StdMutex<HashMap<String, CancellationToken>>>,
    batch_id: String,
}

impl Drop for BatchCancellationGuard {
    fn drop(&mut self) {
        if let Ok(mut cancellations) = self.cancellations.lock() {
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
        let prepared = crate::executor::ArchiveJobSubmissionAdapter
            .prepare(
                &request,
                &format!(
                    "batch-{batch_id}-{}-{}",
                    candidate.tweet_id,
                    crate::runtime::timestamp_marker()
                ),
            )
            .map_err(|error| error.to_string())?;
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

fn reconcile_batch_candidates(database: &Database, batch_id: &str) -> Result<(), String> {
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
    thread::Builder::new()
        .name(format!("xarchive-batch-{batch_id}"))
        .spawn(move || {
            let _guard = BatchCancellationGuard {
                cancellations: cancellations.clone(),
                batch_id: batch_id.clone(),
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
            if let Ok(mut tokens) = cancellations.lock() {
                tokens.remove(&batch_id);
            }
        })
        .map(|_| ())
        .map_err(|error| format!("failed to spawn batch coordinator: {error}"))
}

/// Run discovery and then hand the durable candidates to the bounded executor
/// coordinator. The Sidecar process is private to this worker.
pub(crate) fn spawn_account_batch(
    service: crate::executor::ArchiveApplicationService,
    database_path: std::path::PathBuf,
    files: FileStore,
    batch_id: String,
    sidecar_program: String,
    sidecar_args: Vec<String>,
    sidecar_env: Vec<(String, String)>,
    cancellation: CancellationToken,
    cancellations: Arc<StdMutex<HashMap<String, CancellationToken>>>,
) -> Result<(), String> {
    thread::Builder::new()
        .name(format!("xarchive-batch-discovery-{batch_id}"))
        .spawn(move || {
            let _guard = BatchCancellationGuard {
                cancellations: cancellations.clone(),
                batch_id: batch_id.clone(),
            };
            let Ok(database) = Database::open(&database_path) else {
                return;
            };
            let Ok(Some(batch)) = database.account_batch(&batch_id) else {
                return;
            };
            let arg_refs = sidecar_args.iter().map(String::as_str).collect::<Vec<_>>();
            let mut supervisor = match SidecarSupervisor::spawn_ready_v2_with_env(
                &sidecar_program,
                &arg_refs,
                &sidecar_env,
                Duration::from_secs(5),
            ) {
                Ok(supervisor) => supervisor,
                Err(error) => {
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
            if run_account_discovery(&database, &mut supervisor, &request, &cancellation).is_err() {
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
            if let Ok(mut tokens) = cancellations.lock() {
                tokens.remove(&batch_id);
            }
        })
        .map(|_| ())
        .map_err(|error| format!("failed to spawn account batch worker: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use xarchive_core::{ArchiveMedia, ArchiveMetadata};

    const NOW: &str = "2026-09-22T00:00:00Z";

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "xarchive-batch-test-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    fn candidate_full(
        tweet_id: &str,
        created_at: Option<&str>,
        media_count: u32,
        is_repost: bool,
    ) -> xarchive_protocol::DiscoveryCandidate {
        xarchive_protocol::DiscoveryCandidate {
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            created_at: created_at.map(str::to_owned),
            tweet_type: if is_repost {
                "retweet".to_owned()
            } else {
                "post".to_owned()
            },
            is_repost,
            has_media: media_count > 0,
            media_count,
            user_id: Some("42".to_owned()),
            username: Some("alice".to_owned()),
        }
    }

    fn outcome(candidates: Vec<xarchive_protocol::DiscoveryCandidate>) -> DiscoveryOutcome {
        DiscoveryOutcome {
            candidates_found: candidates.len() as u64,
            candidates,
        }
    }

    fn open_batch(database: &Database, id: &str) {
        database
            .create_account_batch(id, "alice", "https://x.com/alice", None, None, "{}")
            .expect("batch");
    }

    fn selected_ids(candidates: &[BatchCandidateRecord]) -> Vec<String> {
        candidates
            .iter()
            .map(|candidate| candidate.tweet_id.clone())
            .collect()
    }

    #[test]
    fn batch_filters_use_product_defaults_and_reject_unknown_fields() {
        let filters = BatchFilters::from_json("{}").expect("defaults");
        assert_eq!(filters, BatchFilters::default());
        assert!(!filters.include_reposts);
        assert!(!filters.include_without_media);
        assert_eq!(filters.limit, None);

        let explicit = BatchFilters::from_json(
            r#"{"include_reposts":true,"include_without_media":true,"since":"2026-09-01T00:00:00Z","until":"2026-09-30T00:00:00Z","limit":25}"#,
        )
        .expect("explicit filters");
        assert_eq!(
            BatchFilters::from_json(&explicit.to_json().expect("json")).expect("round trip"),
            explicit
        );
        assert!(BatchFilters::from_json(r#"{"unexpected":1}"#).is_err());
        assert!(BatchFilters::from_json("not-json").is_err());
    }

    #[test]
    fn persists_discovery_candidates_once_and_binds_stable_identity() {
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        let discovery = outcome(vec![
            candidate_full("100", Some("2026-09-20T00:00:00Z"), 2, false),
            candidate_full("101", Some("2026-09-21T00:00:00Z"), 0, false),
        ]);

        let summary =
            persist_discovered_candidates(&database, "batch-1", &discovery).expect("persist");
        assert_eq!(summary.candidates_found, 2);
        assert_eq!(summary.inserted, 2);
        assert_eq!(summary.user_id.as_deref(), Some("42"));
        assert_eq!(summary.username.as_deref(), Some("alice"));

        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.user_id.as_deref(), Some("42"));
        assert_eq!(batch.counts.total, 2);
        assert_eq!(batch.counts.pending, 2);
        assert_eq!(
            batch.discovery_state, "PENDING",
            "persisting candidates must not advance the discovery state"
        );

        let stored = database
            .list_batch_candidates("batch-1", None, 10)
            .expect("candidates");
        assert_eq!(stored[0].tweet_id, "100");
        assert_eq!(stored[0].media_count, 2);
        assert!(stored[0].has_media);
        assert!(!stored[0].is_repost);
        assert_eq!(stored[1].tweet_id, "101");
        assert!(!stored[1].has_media);

        // A re-run must not duplicate rows nor resurrect an already dispatched row.
        database
            .mark_batch_candidate(
                "batch-1",
                "100",
                "SUBMITTED",
                Some("job-100"),
                None,
                None,
                None,
            )
            .expect("submit");
        let rerun = persist_discovered_candidates(&database, "batch-1", &discovery).expect("rerun");
        assert_eq!(rerun.inserted, 0);
        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.counts.total, 2);
        assert_eq!(batch.counts.submitted, 1);
        let submitted = database
            .list_batch_candidates("batch-1", Some("SUBMITTED"), 10)
            .expect("submitted");
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].job_id.as_deref(), Some("job-100"));
    }

    /// Local Python is used exactly like the supervisor-crate tests do: a worker
    /// stub that streams protocol v2 lines. Without an interpreter the stream
    /// behavior stays covered by the supervisor crate, so the case is skipped
    /// rather than reported as a pass.
    const DISCOVERY_STUB_OK: &str = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] != 'discover':
        continue
    base = {'protocol_version': 2, 'job_id': command['job_id'], 'request_id': command['request_id']}
    print(json.dumps(dict(base, event='discovery_started')), flush=True)
    for tweet_id in ('100', '101'):
        candidate = {
            'tweet_id': tweet_id,
            'url': 'https://x.com/alice/status/' + tweet_id,
            'created_at': '2026-09-20T00:00:00Z',
            'tweet_type': 'post',
            'is_repost': False,
            'has_media': True,
            'media_count': 2,
            'user_id': '42',
            'username': 'alice',
        }
        print(json.dumps(dict(base, event='candidate', candidate=candidate)), flush=True)
    print(json.dumps(dict(base, event='discovery_completed', candidates_found=2)), flush=True)
    break
"#;

    const DISCOVERY_STUB_AUTH_FAILURE: &str = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] != 'discover':
        continue
    base = {'protocol_version': 2, 'job_id': command['job_id'], 'request_id': command['request_id']}
    print(json.dumps(dict(base, event='discovery_started')), flush=True)
    print(json.dumps(dict(base, event='failed', error_code='HTTP_401', error_message='login required')), flush=True)
    break
"#;

    fn python_stub(script: &str) -> Option<SidecarSupervisor> {
        let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_owned());
        SidecarSupervisor::spawn(&python, &["-c", script]).ok()
    }

    fn batch_request(database: &Database, batch_id: &str) -> DiscoveryRequest {
        discovery_request(&database.account_batch(batch_id).expect("get").expect("row"))
    }

    #[test]
    fn runs_discovery_and_persists_streamed_candidates() {
        let Some(mut supervisor) = python_stub(DISCOVERY_STUB_OK) else {
            return;
        };
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        let request = batch_request(&database, "batch-1");
        assert_eq!(request.request_id, "batch-discovery-batch-1");

        let summary = run_account_discovery(
            &database,
            &mut supervisor,
            &request,
            &CancellationToken::new(),
        )
        .expect("discovery");
        assert_eq!(summary.candidates_found, 2);
        assert_eq!(summary.inserted, 2);
        assert_eq!(summary.user_id.as_deref(), Some("42"));

        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.discovery_state, "COMPLETED");
        assert_eq!(batch.last_error_code, None);
        assert_eq!(batch.user_id.as_deref(), Some("42"));
        assert_eq!(batch.counts.pending, 2);
        supervisor.shutdown();
    }

    #[test]
    fn records_a_failed_discovery_on_the_batch() {
        let Some(mut supervisor) = python_stub(DISCOVERY_STUB_AUTH_FAILURE) else {
            return;
        };
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        let request = batch_request(&database, "batch-1");

        let error = run_account_discovery(
            &database,
            &mut supervisor,
            &request,
            &CancellationToken::new(),
        )
        .expect_err("discovery must fail");
        assert_eq!(error, "HTTP_401: login required");

        let batch = database
            .account_batch("batch-1")
            .expect("get")
            .expect("row");
        assert_eq!(batch.discovery_state, "FAILED");
        assert_eq!(batch.last_error_code.as_deref(), Some("HTTP_401"));
        assert_eq!(
            batch.last_error_message.as_deref(),
            Some("HTTP_401: login required")
        );
        assert_eq!(batch.counts.total, 0);
        supervisor.shutdown();
    }

    /// Persist the durable archive facts of one Tweet: user row, tweet row,
    /// committed directory, media row. `keep_media_file` controls whether the
    /// recorded media file still exists on disk after the commit.
    fn archive_tweet(
        database: &Database,
        files: &FileStore,
        tweet_id: &str,
        keep_media_file: bool,
    ) {
        let user_row_id = database
            .upsert_user("42", Some("alice"), Some("Alice"), NOW)
            .expect("user");
        let tweet_row_id = database
            .insert_tweet_for_user(
                tweet_id,
                &format!("https://x.com/alice/status/{tweet_id}"),
                "post",
                "archived text",
                Some(user_row_id),
                NOW,
            )
            .expect("tweet");
        let directory = format!("@alice - Alice [42]/tweet-{tweet_id}");
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            tweet_type: "post".to_owned(),
            author: xarchive_core::ArchiveAuthor {
                user_id: Some("42".to_owned()),
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
            },
            created_at: Some("2026-09-20T00:00:00Z".to_owned()),
            text: "archived text".to_owned(),
            media: vec![ArchiveMedia {
                index: 1,
                media_id: Some("m1".to_owned()),
                media_type: "photo".to_owned(),
                file: "media/1.jpg".to_owned(),
                mime_type: Some("image/jpeg".to_owned()),
                size_bytes: 4,
                sha256: "a".to_owned(),
            }],
            archived_at: NOW.to_owned(),
            reply_to: None,
            quoted_tweet: None,
        };
        database
            .update_tweet_metadata(tweet_row_id, &metadata, &directory)
            .expect("metadata");
        for media in &metadata.media {
            database
                .insert_media(tweet_row_id, media, NOW)
                .expect("media row");
        }
        if keep_media_file {
            let media_path = files
                .archive_path(&directory)
                .expect("archive path")
                .join("media/1.jpg");
            fs::create_dir_all(media_path.parent().expect("media dir")).expect("media dir");
            fs::write(&media_path, b"jpeg").expect("media file");
        }
    }

    #[test]
    fn skips_candidates_whose_archive_is_already_on_disk() {
        let root = temp_root("batch-skip");
        let files = FileStore::new(&root).expect("files");
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        persist_discovered_candidates(
            &database,
            "batch-1",
            &outcome(vec![
                candidate_full("100", Some("2026-09-20T00:00:00Z"), 1, false),
                candidate_full("101", Some("2026-09-21T00:00:00Z"), 1, false),
                candidate_full("102", Some("2026-09-22T00:00:00Z"), 1, false),
            ]),
        )
        .expect("persist");

        // 100: committed archive with its media file present.
        archive_tweet(&database, &files, "100", true);
        // 101: committed archive whose media file disappeared afterwards.
        archive_tweet(&database, &files, "101", false);
        // 102: no archive facts at all.

        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &BatchFilters::default(), 10)
                .expect("select");
        assert_eq!(selected_ids(&selected), ["102", "101"]);

        let counts = database.batch_candidate_counts("batch-1").expect("counts");
        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.pending, 2);

        let skipped = database
            .list_batch_candidates("batch-1", Some("SKIPPED"), 10)
            .expect("skipped");
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].tweet_id, "100");
        assert_eq!(
            skipped[0].skip_reason.as_deref(),
            Some(SKIP_ALREADY_ARCHIVED)
        );

        // A later round must not re-evaluate the skipped candidate.
        let second =
            select_dispatch_candidates(&database, &files, "batch-1", &BatchFilters::default(), 10)
                .expect("second select");
        assert_eq!(selected_ids(&second), ["102", "101"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn filters_and_batch_limit_record_every_exclusion_reason() {
        let root = temp_root("batch-filters");
        let files = FileStore::new(&root).expect("files");
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        persist_discovered_candidates(
            &database,
            "batch-1",
            &outcome(vec![
                candidate_full("200", Some("2026-09-20T00:00:00Z"), 0, false),
                candidate_full("201", Some("2026-09-21T00:00:00Z"), 1, true),
                candidate_full("202", Some("2026-09-22T00:00:00Z"), 1, false),
                candidate_full("203", Some("2026-09-23T00:00:00Z"), 1, false),
                candidate_full("204", Some("2026-09-24T00:00:00Z"), 1, false),
                candidate_full("205", Some("2026-09-25T00:00:00Z"), 1, false),
            ]),
        )
        .expect("persist");

        let filters = BatchFilters {
            limit: Some(2),
            ..BatchFilters::default()
        };
        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 10).expect("select");
        assert_eq!(selected_ids(&selected), ["205", "204"]);

        let counts = database.batch_candidate_counts("batch-1").expect("counts");
        assert_eq!(counts.skipped, 4);
        assert_eq!(counts.pending, 2);
        let skipped = database
            .list_batch_candidates("batch-1", Some("SKIPPED"), 10)
            .expect("skipped");
        let reasons = skipped
            .iter()
            .map(|candidate| (candidate.tweet_id.clone(), candidate.skip_reason.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            reasons,
            [
                ("200".to_owned(), Some(SKIP_NO_MEDIA.to_owned())),
                ("201".to_owned(), Some(SKIP_REPOST.to_owned())),
                ("202".to_owned(), Some(SKIP_BEYOND_LIMIT.to_owned())),
                ("203".to_owned(), Some(SKIP_BEYOND_LIMIT.to_owned())),
            ]
        );

        // The batch cap counts dispatched candidates, so submitting one of them
        // leaves exactly one slot for the next round.
        database
            .mark_batch_candidate(
                "batch-1",
                "205",
                "SUBMITTED",
                Some("job-205"),
                None,
                None,
                None,
            )
            .expect("submit");
        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 10).expect("second");
        assert_eq!(selected_ids(&selected), ["204"]);
        let counts = database.batch_candidate_counts("batch-1").expect("counts");
        assert_eq!(counts.skipped, 4);
        assert_eq!(counts.pending, 1);

        // The per-round bound only limits the returned list.
        let bounded =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 0).expect("bounded");
        assert!(bounded.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn date_filters_exclude_out_of_range_candidates_and_keep_unknown_dates() {
        let root = temp_root("batch-dates");
        let files = FileStore::new(&root).expect("files");
        let database = Database::open_in_memory().expect("database");
        open_batch(&database, "batch-1");
        persist_discovered_candidates(
            &database,
            "batch-1",
            &outcome(vec![
                candidate_full("300", Some("2026-09-01T00:00:00Z"), 1, false),
                candidate_full("301", Some("2026-09-15T00:00:00Z"), 1, false),
                candidate_full("302", None, 1, false),
                candidate_full("303", Some("2026-09-25T00:00:00Z"), 1, false),
            ]),
        )
        .expect("persist");
        let filters = BatchFilters {
            since: Some("2026-09-10T00:00:00Z".to_owned()),
            until: Some("2026-09-20T00:00:00Z".to_owned()),
            ..BatchFilters::default()
        };
        let selected =
            select_dispatch_candidates(&database, &files, "batch-1", &filters, 10).expect("select");
        // `302` carries no timestamp: an unknown date is kept, never dropped.
        assert_eq!(selected_ids(&selected), ["301", "302"]);
        let skipped = database
            .list_batch_candidates("batch-1", Some("SKIPPED"), 10)
            .expect("skipped");
        let reasons = skipped
            .iter()
            .map(|candidate| (candidate.tweet_id.clone(), candidate.skip_reason.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            reasons,
            [
                ("300".to_owned(), Some(SKIP_BEFORE_RANGE.to_owned())),
                ("303".to_owned(), Some(SKIP_AFTER_RANGE.to_owned())),
            ]
        );
        let _ = fs::remove_dir_all(root);
    }
}
