//! Durable batch discovery: candidate persistence and run-state recording.
//!
//! Split out of `batch.rs` without changing behaviour. The invariants that
//! keep discovery honest:
//!
//! * re-running discovery for the same batch is idempotent — `tweet_id` is
//!   unique per batch and existing rows keep their state, so a re-run never
//!   resurrects an archived or cancelled Tweet;
//! * the stable identity is the first candidate that actually carries a user
//!   id; a username is only ever a display aid;
//! * a cancelled or externally stopped batch records PAUSED/CANCELLED instead
//!   of a failure, so a restart can tell the two apart;
//! * persisted error codes and messages stay bounded because the batch row is
//!   user-visible.

use std::time::Duration;

use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{AccountBatchSummary, Database, NewBatchCandidate};

use crate::executor::CancellationToken;
use crate::production::{DiscoveryOutcome, DiscoveryRequest, execute_v2_discovery};

use super::{storage_error, truncate_message};

pub(crate) const SKIP_ALREADY_ARCHIVED: &str = "ALREADY_ARCHIVED";
pub(crate) const SKIP_REPOST: &str = "FILTERED_REPOST";
pub(crate) const SKIP_NO_MEDIA: &str = "FILTERED_NO_MEDIA";
pub(crate) const SKIP_BEFORE_RANGE: &str = "FILTERED_BEFORE_RANGE";
pub(crate) const SKIP_AFTER_RANGE: &str = "FILTERED_AFTER_RANGE";
pub(crate) const SKIP_BEYOND_LIMIT: &str = "FILTERED_LIMIT";

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
    timeout: Duration,
) -> Result<DiscoverySummary, String> {
    if cancellation.is_cancelled() {
        let discovery_state = database
            .account_batch_state(&request.batch_id)
            .ok()
            .flatten()
            .as_deref()
            .and_then(|state| match state {
                "PAUSED" => Some("PAUSED"),
                "CANCELLED" => Some("CANCELLED"),
                _ => None,
            })
            .unwrap_or("PENDING");
        database
            .set_account_batch_discovery_state(&request.batch_id, discovery_state)
            .map_err(|error| {
                storage_error("failed to preserve cancelled discovery state", error)
            })?;
        return Err("account discovery cancelled before start".to_owned());
    }
    database
        .set_account_batch_discovery_state(&request.batch_id, "RUNNING")
        .map_err(|error| storage_error("failed to start discovery", error))?;

    let mut inserted = 0_u64;
    let mut discovered_user_id = None;
    let mut discovered_username = None;
    let outcome =
        match execute_v2_discovery(supervisor, request, cancellation, timeout, |candidate| {
            let record = candidate_record(candidate);
            inserted += database
                .insert_batch_candidates(&request.batch_id, std::slice::from_ref(&record))
                .map_err(|error| storage_error("failed to persist discovered candidate", error))?;
            if discovered_user_id.is_none() {
                discovered_user_id = candidate.user_id.clone();
            }
            if discovered_username.is_none() {
                discovered_username = candidate.username.clone();
            }
            if candidate.user_id.is_some() || candidate.username.is_some() {
                database
                    .resolve_account_batch_identity(
                        &request.batch_id,
                        candidate.user_id.as_deref(),
                        candidate.username.as_deref(),
                    )
                    .map_err(|error| storage_error("failed to bind batch identity", error))?;
            }
            Ok(())
        }) {
            Ok(outcome) => outcome,
            Err(error) => {
                let stopped_state = database
                    .account_batch_state(&request.batch_id)
                    .ok()
                    .flatten()
                    .and_then(|state| match state.as_str() {
                        "PAUSED" => Some("PAUSED"),
                        "CANCELLED" => Some("CANCELLED"),
                        _ => None,
                    });
                if cancellation.is_cancelled() || stopped_state.is_some() {
                    let discovery_state = stopped_state.unwrap_or("PENDING");
                    let _ = database
                        .set_account_batch_discovery_state(&request.batch_id, discovery_state);
                    return Err(error);
                }
                let code = discovery_error_code(&error);
                let message = truncate_message(&error);
                database
                    .set_account_batch_error(&request.batch_id, Some(&code), Some(&message))
                    .map_err(|storage| {
                        storage_error("failed to record discovery error", storage)
                    })?;
                database
                    .set_account_batch_discovery_state(&request.batch_id, "FAILED")
                    .map_err(|storage| {
                        storage_error("failed to record discovery failure", storage)
                    })?;
                return Err(error);
            }
        };

    let batch_state = database
        .account_batch_state(&request.batch_id)
        .map_err(|error| storage_error("failed to read batch state after discovery", error))?;
    if batch_state.as_deref() != Some("ACTIVE") {
        let discovery_state = match batch_state.as_deref() {
            Some("PAUSED") => "PAUSED",
            Some("CANCELLED") => "CANCELLED",
            _ => "PENDING",
        };
        database
            .set_account_batch_discovery_state(&request.batch_id, discovery_state)
            .map_err(|error| storage_error("failed to preserve stopped discovery state", error))?;
        return Err("account discovery stopped before completion".to_owned());
    }
    database
        .set_account_batch_error(&request.batch_id, None, None)
        .map_err(|error| storage_error("failed to clear discovery error", error))?;
    database
        .set_account_batch_discovery_state(&request.batch_id, "COMPLETED")
        .map_err(|error| storage_error("failed to complete discovery", error))?;
    Ok(DiscoverySummary {
        candidates_found: outcome.candidates_found,
        inserted,
        user_id: discovered_user_id,
        username: discovered_username,
    })
}

/// Derive a bounded error code from a discovery failure message.
///
/// The protocol allows at most 64 characters for `error_code`, so a message
/// that does not start with a code-shaped token falls back to a stable code.
pub(crate) fn discovery_error_code(message: &str) -> String {
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
