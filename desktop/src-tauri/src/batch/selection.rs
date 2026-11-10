//! Dispatch selection: which Tweets of a batch may run next.
//!
//! Split out of `batch.rs` without changing behaviour. The decision order is
//! part of the contract and must not move:
//!
//! 1. a present committed archive is skipped as `ALREADY_ARCHIVED`;
//! 2. a candidate the filters exclude is skipped with its filter reason;
//! 3. everything else is approved, bounded per dispatch round and per batch.
//!
//! Skipping is durable and ordered. A skipped candidate leaves the PENDING set,
//! so a later round never re-evaluates it and the batch counts explain the
//! outcome. The batch cap keeps the newest candidates, and a candidate without
//! a `created_at` is never dropped by a date range: unknown data must not
//! silently vanish.
//!
//! The already-archived check uses the completeness policy, never bare file
//! existence: the recorded media-row count is compared with the discovery
//! `media_count`, every recorded file must exist in the committed directory,
//! and recorded sizes must match. A digest check is intentionally not part of
//! dispatch; SHA-256 verification runs separately on explicit request.

use std::path::{Component, PathBuf};

use serde::{Deserialize, Serialize};

use xarchive_storage::{
    ArchiveCompletenessOptions, BatchCandidateRecord, BatchCounts, Database, FileStore,
    TweetArchiveFacts, evaluate_archive_completeness,
};

use super::{
    SKIP_AFTER_RANGE, SKIP_ALREADY_ARCHIVED, SKIP_BEFORE_RANGE, SKIP_BEYOND_LIMIT, SKIP_NO_MEDIA,
    SKIP_REPOST, storage_error,
};

/// How many PENDING candidates one selection call inspects.
const SELECTION_SCAN_LIMIT: u32 = 512;

/// User-visible discovery filters, persisted as `archive_batches.filters_json`.
///
/// The default matches the approved product scope: only the account's own
/// media Tweets, with no explicit cap.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
        if (!candidate.has_media || candidate.media_count == 0) && !self.include_without_media {
            return Some(SKIP_NO_MEDIA);
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
    Ok(
        evaluate_archive_completeness(files, &facts, expected, &ArchiveCompletenessOptions::fast())
            .is_complete(),
    )
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
