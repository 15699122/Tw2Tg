//! Telegram sending core (plan TG-06 shared part).
//!
//! The Desktop runtime decides *when* this runs; this module owns *how* a
//! finished archive becomes queued sends and how due entries are executed.
//! Automatic sending stays off until the user enables it (plan §2.1), so the
//! archive path calls [`queue_archive_sends`] only behind
//! [`auto_send_enabled`].
//!
//! An outbox row stores archive-relative media paths. The desktop resolves
//! those paths against recorded archive facts before any transport request.

#![allow(dead_code)] // Consumed by the Telegram settings/send surface (Batch B).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use xarchive_storage::{Database, FileStore};

use xarchive_telegram::{
    AlbumItemUpload, BotToken, CachedFileId, CancellationToken, FileCacheKey, FileIdCacheStore,
    MediaKind, OutboxEntry, PlannedMediaItem, PlannedSend, RequestProgress,
    ReqwestTelegramTransport, RunAttemptError, SendAttemptError, SendAttemptSuccess, SendFailure,
    SendMessageRequest, SendPayload, SendPhotoRequest, SendVideoRequest,
    TELEGRAM_FILE_CACHE_VERSION, TelegramError, TelegramOutboxStore, TelegramRequest,
    TelegramTransport, UploadRequest, UploadStage, UploadTimeouts, is_invalid_file_id_error,
    plan_media_sends, plan_text_send,
};

use crate::clock;
use crate::config::TelegramConfig;

/// Apply rotation policy to an exact queue snapshot. Confirmation mode only
/// returns candidates; callers must explicitly grant them later.
pub fn apply_credential_resume_policy(
    database: &Database,
    generation: i64,
    previous_bot_identity: Option<&str>,
    policy: crate::config::CredentialRotationResumePolicy,
    now: &str,
) -> Result<Vec<i64>, String> {
    let active = database
        .active_telegram_credential_generation()
        .map_err(|_| "credential state unavailable".to_owned())?
        .filter(|active| active.0 == generation)
        .ok_or("credential generation changed")?;
    if previous_bot_identity != Some(active.1.as_str()) {
        return Ok(Vec::new());
    }
    let (captured_policy, candidates) = match database
        .telegram_rotation_decision(generation)
        .map_err(|_| "rotation decision unavailable".to_owned())?
    {
        Some(decision) => decision,
        None => {
            let candidates = database
                .telegram_resume_candidates(generation)
                .map_err(|_| "resume candidates unavailable".to_owned())?;
            let captured_policy = match policy {
                crate::config::CredentialRotationResumePolicy::Automatic => "automatic",
                crate::config::CredentialRotationResumePolicy::Confirm => "confirm",
            };
            database
                .record_telegram_rotation_decision(generation, captured_policy, &candidates, now)
                .map_err(|_| "rotation decision persistence failed".to_owned())?;
            (captured_policy.to_owned(), candidates)
        }
    };
    // Automatic grants are committed atomically with the captured decision.
    // Replays never reauthorize rows that have since advanced or been cancelled.
    debug_assert!(matches!(captured_policy.as_str(), "automatic" | "confirm"));
    Ok(candidates)
}

/// Activate a verified candidate through an isolated SecretStore reference.
/// Failed activation never overwrites the active generation's stored token.
pub fn activate_verified_credential(
    database: &Database,
    store: &mut dyn xarchive_telegram::SecretStore,
    transport: &dyn TelegramTransport,
    candidate: &str,
    secret_reference: &str,
    expected_active: Option<i64>,
    now: &str,
) -> Result<i64, String> {
    activate_verified_credential_with_policy(
        database,
        store,
        transport,
        candidate,
        secret_reference,
        expected_active,
        crate::config::CredentialRotationResumePolicy::Automatic,
        now,
    )
}

/// Activate using the explicitly selected policy, captured in the same database
/// transaction as the generation change and eligible queue snapshot.
#[allow(clippy::too_many_arguments)]
pub fn activate_verified_credential_with_policy(
    database: &Database,
    store: &mut dyn xarchive_telegram::SecretStore,
    transport: &dyn TelegramTransport,
    candidate: &str,
    secret_reference: &str,
    expected_active: Option<i64>,
    policy: crate::config::CredentialRotationResumePolicy,
    now: &str,
) -> Result<i64, String> {
    let token =
        BotToken::new(candidate.trim()).map_err(|_| "invalid candidate token".to_owned())?;
    let identity = xarchive_telegram::verify_bot_identity(transport, &token)
        .map_err(|_| "candidate identity verification failed".to_owned())?;
    let generation = database
        .prepare_telegram_credential_generation(&identity, secret_reference, now)
        .map_err(|_| "credential candidate registration failed".to_owned())?;
    store
        .set(secret_reference, candidate.trim())
        .map_err(|_| "candidate secret storage failed".to_owned())?;
    if database
        .activate_telegram_credential_with_resume(
            generation,
            expected_active,
            match policy {
                crate::config::CredentialRotationResumePolicy::Automatic => "automatic",
                crate::config::CredentialRotationResumePolicy::Confirm => "confirm",
            },
            now,
        )
        .is_err()
    {
        // A failed delete leaves only an inactive orphan, never sending authority.
        let _ = store.delete(secret_reference);
        return Err("credential activation changed or failed".into());
    }
    Ok(generation)
}

/// Bounded batch: one run must not starve the application when a large backlog
/// exists. The next scheduled run continues where this one stopped.
const MAX_SENDS_PER_RUN: usize = 10;

/// Claim lease: long enough for a slow upload, short enough that a crashed
/// worker's entry becomes recoverable without manual action.
const CLAIM_LEASE: Duration = Duration::from_secs(300);

/// What one queue run did, for logs and task events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendRunSummary {
    pub claimed: u32,
    pub sent: u32,
    /// A retry was scheduled (`RETRY_WAIT`).
    pub deferred: u32,
    /// The outcome is unknown and waits for review.
    pub unknown: u32,
    /// A permanent failure or one that needs a corrected plan.
    pub failed: u32,
}

/// Whether automatic sending may queue work after an archive.
///
/// Both switches must be on and a target must exist: automatic sending is
/// never implied by merely having Telegram configured.
pub fn auto_send_enabled(config: &TelegramConfig) -> bool {
    !config.migration_pending
        && config.enabled
        && config.auto_send_on_archive
        && !config.chat_id.trim().is_empty()
}

/// Finished archive facts and planning metadata. This value is also the input
/// to the same idempotent planner used by a caller that can reconstruct it
/// after restart.
#[derive(Debug, Clone)]
pub struct ArchiveSendIntent {
    pub tweet_row_id: i64,
    pub archive_directory: String,
    pub metadata_text: Option<String>,
    pub media: Vec<PlannedMediaItem>,
    pub plan_version: i64,
    /// Bot/config scope captured when the intent is prepared; it contains no
    /// token and prevents current settings from redirecting an existing plan.
    pub bot_identity: String,
    pub chat_id: String,
    pub message_thread_id: Option<i64>,
    pub config_revision: i64,
}

impl ArchiveSendIntent {
    /// Journal the exact captured plan before filesystem commit. Lifecycle
    /// timestamps do not participate in the immutable intent identity.
    pub fn to_journal_record(
        &self,
        job_id: &str,
        now: &str,
    ) -> Result<xarchive_storage::TelegramArchiveIntentRecord, String> {
        if job_id.trim().is_empty()
            || !safe_relative_path(std::path::Path::new(&self.archive_directory))
            || !matches!(self.plan_version, 1..=3)
            || self.bot_identity.trim().is_empty()
            || self.chat_id.trim().is_empty()
        {
            return Err("invalid captured archive intent".into());
        }
        Ok(xarchive_storage::TelegramArchiveIntentRecord {
            job_id: job_id.to_owned(),
            tweet_row_id: self.tweet_row_id,
            archive_directory: self.archive_directory.clone(),
            state: "PREPARED".into(),
            metadata_text: self.metadata_text.clone(),
            media_json: serde_json::to_string(&self.media)
                .map_err(|_| "archive media snapshot serialization failed".to_owned())?,
            bot_identity: Some(self.bot_identity.clone()),
            chat_id: Some(self.chat_id.clone()),
            message_thread_id: self.message_thread_id,
            config_revision: Some(self.config_revision),
            plan_version: self.plan_version,
            created_at: now.to_owned(),
            updated_at: now.to_owned(),
        })
    }

    /// Map trusted archive facts without hashing staging paths or reading
    /// mutable settings during replay. FileStore verifies these facts at send.
    pub fn from_archive_metadata(
        tweet_row_id: i64,
        archive_directory: String,
        metadata: &xarchive_core::ArchiveMetadata,
        config: &TelegramConfig,
        bot_identity: &str,
    ) -> Result<Self, String> {
        let directory = std::path::Path::new(&archive_directory);
        if !safe_relative_path(directory) {
            return Err("invalid archive directory".into());
        }
        let mut items = metadata.media.iter().collect::<Vec<_>>();
        items.sort_by_key(|item| item.index);
        let mut indices = std::collections::HashSet::new();
        let mut paths = std::collections::HashSet::new();
        let mut media = Vec::with_capacity(items.len());
        for item in items {
            let relative = std::path::Path::new(&item.file);
            if !safe_relative_path(relative)
                || !indices.insert(item.index)
                || !paths.insert(item.file.clone())
            {
                return Err("invalid or duplicate archive media reference".into());
            }
            let name = relative
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("invalid archive media filename")?;
            media.push(PlannedMediaItem {
                media_kind: if config.upload_mode == xarchive_telegram::UploadMode::OriginalFile {
                    MediaKind::Document
                } else {
                    match item.media_type.as_str() {
                        "photo" | "image" => MediaKind::Photo,
                        "video" | "animated_gif" => MediaKind::Video,
                        _ => MediaKind::Document,
                    }
                },
                file_path: directory.join(relative),
                file_name: name.to_owned(),
                mime_type: item.mime_type.clone(),
                caption: None,
                content_sha256: Some(item.sha256.clone()),
                size_bytes: Some(item.size_bytes),
            });
        }
        let text = xarchive_telegram::format_metadata(&xarchive_telegram::MetadataInput {
            tweet_id: &metadata.tweet_id,
            url: &metadata.url,
            username: metadata.author.username.as_deref(),
            display_name: metadata.author.display_name.as_deref(),
            text: &metadata.text,
            tags: &[],
        });
        Self::from_verified_config(
            tweet_row_id,
            archive_directory,
            Some(text),
            media,
            config,
            bot_identity,
        )
    }

    /// Capture immutable, non-secret queue facts after the caller has verified
    /// the token and obtained its stable bot identity.
    pub fn from_verified_config(
        tweet_row_id: i64,
        archive_directory: String,
        metadata_text: Option<String>,
        media: Vec<PlannedMediaItem>,
        config: &TelegramConfig,
        bot_identity: &str,
    ) -> Result<Self, String> {
        if bot_identity.trim().is_empty() {
            return Err("verified Telegram bot identity must not be empty".to_owned());
        }
        if config.chat_id.trim().is_empty() {
            return Err("Telegram target chat must not be empty".to_owned());
        }
        Ok(Self {
            tweet_row_id,
            archive_directory,
            metadata_text,
            media,
            plan_version: 3,
            bot_identity: bot_identity.to_owned(),
            chat_id: config.chat_id.clone(),
            message_thread_id: config.message_thread_id,
            config_revision: config.revision,
        })
    }
}

fn safe_relative_path(path: &std::path::Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
        && !path.to_string_lossy().contains(['\\', ':'])
}

/// Queue the planned sends of one immutable archive intent and return row ids.
/// Metadata text is queued first, then media units in order. Replaying the same
/// intent is idempotent and cannot redirect it to mutable current settings.
pub fn queue_archive_sends(
    database: &Database,
    intent: &ArchiveSendIntent,
    now: &str,
) -> Result<Vec<i64>, String> {
    database
        .enqueue_outbox_batch(archive_plan_entries(intent, now)?)
        .map_err(|error| error.to_string())
}

pub type ArchivedIntentQueueResult = (i64, Result<Vec<i64>, String>);

/// Reconcile archived intents independently: one damaged snapshot must not prevent
/// unrelated archives from being planned. PREPARED records require file recovery
/// first and are deliberately not consumed here. This performs no network I/O.
pub fn reconcile_archived_send_intents(
    database: &Database,
    now: &str,
) -> Result<Vec<ArchivedIntentQueueResult>, String> {
    let records = database
        .list_recoverable_telegram_archive_intents()
        .map_err(|error| error.to_string())?;
    Ok(records
        .into_iter()
        .filter(|record| record.state == "ARCHIVED")
        .map(|record| {
            (
                record.tweet_row_id,
                queue_persisted_archive_sends(database, record.tweet_row_id, now),
            )
        })
        .collect())
}

/// Recover a complete plan exclusively from the journal, never current settings.
pub fn queue_persisted_archive_sends(
    database: &Database,
    tweet_id: i64,
    now: &str,
) -> Result<Vec<i64>, String> {
    let record = database
        .telegram_archive_intent(tweet_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "archive intent missing".to_owned())?;
    if !matches!(record.plan_version, 1..=3) {
        return Err("unsupported archive intent plan version".into());
    }
    let intent = ArchiveSendIntent {
        tweet_row_id: record.tweet_row_id,
        archive_directory: record.archive_directory,
        metadata_text: record.metadata_text,
        media: serde_json::from_str(&record.media_json)
            .map_err(|_| "invalid archive intent media snapshot".to_owned())?,
        plan_version: record.plan_version,
        bot_identity: record
            .bot_identity
            .filter(|value| !value.trim().is_empty())
            .ok_or("archive intent bot missing")?,
        chat_id: record
            .chat_id
            .filter(|value| !value.trim().is_empty())
            .ok_or("archive intent chat missing")?,
        message_thread_id: record.message_thread_id,
        config_revision: record
            .config_revision
            .ok_or("archive intent revision missing")?,
    };
    database
        .enqueue_archived_intent(tweet_id, archive_plan_entries(&intent, now)?, now)
        .map_err(|error| error.to_string())
}

fn archive_plan_entries(
    intent: &ArchiveSendIntent,
    now: &str,
) -> Result<Vec<xarchive_telegram::NewOutboxEntry>, String> {
    if !matches!(intent.plan_version, 1..=3) {
        return Err("unsupported archive intent plan version".into());
    }
    let tweet_id = intent.tweet_row_id;
    let mut plans: Vec<PlannedSend> = Vec::new();
    let mut media = intent.media.clone();
    let merged_caption = intent.plan_version == 3
        && intent.metadata_text.as_deref().is_some_and(|text| {
            !text.trim().is_empty()
                && text.chars().count() <= xarchive_telegram::TELEGRAM_CAPTION_LIMIT
        })
        && media.first().is_some_and(|item| item.caption.is_none());
    if merged_caption {
        media[0].caption = intent.metadata_text.clone();
    }
    if let Some(text) = intent
        .metadata_text
        .as_deref()
        .filter(|text| !text.trim().is_empty() && !merged_caption)
    {
        let chunks = if intent.plan_version == 1 {
            vec![text.to_owned()]
        } else {
            xarchive_telegram::split_for_telegram(text)
        };
        for (index, chunk) in chunks.iter().enumerate() {
            let key = if chunks.len() == 1 {
                format!("tweet-{tweet_id}:metadata")
            } else {
                format!("tweet-{tweet_id}:metadata:{index}")
            };
            plans.push(plan_text_send(
                intent.chat_id.clone(),
                intent.message_thread_id,
                chunk,
                key,
            ));
        }
    }
    plans.extend(plan_media_sends(
        intent.chat_id.clone(),
        intent.message_thread_id,
        media,
        &format!("tweet-{tweet_id}:media"),
    ));
    let mut entries = Vec::with_capacity(plans.len());
    for (plan_order, plan) in plans.iter().enumerate() {
        let entry = plan
            .to_outbox_entry(
                &intent.bot_identity,
                &intent.archive_directory,
                Some(tweet_id),
                intent.config_revision,
                intent.plan_version,
                plan_order as i64,
                now,
            )
            .map_err(|error| error.to_string())?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Rebuild a send payload exclusively from the immutable persisted snapshot.
/// Current settings are intentionally not consulted, preventing config edits
/// from redirecting queued work.
pub fn restore_payload_from_entry(entry: &OutboxEntry) -> Option<SendPayload> {
    if entry.payload_schema_version != Some(1) {
        return None;
    }
    serde_json::from_str(entry.payload_json.as_deref()?).ok()
}

/// Resolve persisted archive-relative media references against the tweet's
/// recorded archive directory. FileStore rejects traversal and existing links.
pub fn resolve_archived_payload(
    database: &Database,
    entry: &OutboxEntry,
    files: &xarchive_storage::FileStore,
) -> Option<SendPayload> {
    let mut payload = restore_payload_from_entry(entry)?;
    let SendPayload::Media { items, .. } = &mut payload else {
        return Some(payload);
    };
    let tweet_id = entry.tweet_id?;
    let external_id = database.tweet_external_id(tweet_id).ok()??;
    let facts = database.tweet_archive_facts(&external_id).ok()??;
    // The committed archive directory is the only authority for the on-disk
    // location; the snapshot only supplies the archive-relative selection key.
    let archive_directory = files.archive_path(&facts.archive_directory).ok()?;
    for item in items {
        // A snapshot path is accepted only when it names a recorded media fact
        // of this tweet. Reconstructing the archive-root-relative path from the
        // committed directory rejects both traversal and a stale directory.
        let fact = facts.media.iter().find(|fact| {
            std::path::Path::new(&facts.archive_directory).join(&fact.relative_path)
                == item.file_path
        })?;
        let resolved = files.archive_path(&item.file_path).ok()?;
        if !resolved.starts_with(&archive_directory) || !resolved.is_file() {
            return None;
        }
        if let Some(expected) = fact.sha256.as_deref()
            && xarchive_storage::FileStore::sha256(&resolved)
                .ok()?
                .as_str()
                != expected
        {
            return None;
        }
        if let Some(expected) = item.content_sha256.as_deref()
            && xarchive_storage::FileStore::sha256(&resolved)
                .ok()?
                .as_str()
                != expected
        {
            return None;
        }
        if let Some(expected) = fact.size_bytes
            && std::fs::metadata(&resolved).ok()?.len() != expected
        {
            return None;
        }
        item.file_path = resolved;
    }
    Some(payload)
}

/// Resolve claims left behind by a crashed worker.
///
/// A claim that expired before its request started returns to the retry queue;
/// one that expired after the request started becomes `UNKNOWN` and waits for
/// human review (plan TG-04).
pub fn recover_expired_claims(database: &Database, now: &str) -> Result<u64, String> {
    database
        .recover_outbox_claims(now)
        .map_err(|error| error.to_string())
}

/// Execute the due queue once.
///
/// Persisted version-1 snapshots are authoritative; media references are
/// resolved by `resolve`. Historical NULL snapshots are not reconstructed.
/// `None` means the archive no longer has the payload and is recorded as a
/// plan needing correction instead of being silently skipped. `on_progress` receives each entry's
/// upload stages. A run stops after [`MAX_SENDS_PER_RUN`] entries or when
/// nothing is due; `UNKNOWN` entries are never picked up automatically.
///
/// The parameters stay separate rather than bundled into a context struct: the
/// sender needs the store, the transport, the settings and the clock reading
/// independently, and a wrapper would only rename them.
#[allow(clippy::too_many_arguments)]
pub async fn run_due_sends<R, P>(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    now: &str,
    resolve: R,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    R: Fn(&OutboxEntry) -> Option<SendPayload>,
    P: FnMut(&str, UploadStage),
{
    run_due_sends_with_clock(
        database,
        transport,
        token,
        config,
        bot_identity,
        || now.to_owned(),
        resolve,
        on_progress,
    )
    .await
}

/// Queue runner with a fresh time reading at every durable boundary.
#[allow(clippy::too_many_arguments)]
pub async fn run_due_sends_with_clock<R, P, C>(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    current_time: C,
    resolve: R,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    R: Fn(&OutboxEntry) -> Option<SendPayload>,
    P: FnMut(&str, UploadStage),
    C: Fn() -> String,
{
    run_due_sends_with_generation(
        database,
        transport,
        token,
        config,
        bot_identity,
        None,
        None,
        current_time,
        resolve,
        on_progress,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_due_sends_with_generation<R, P, C>(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    generation: Option<i64>,
    stop: Option<&crate::telegram_worker::WorkerStopSignal>,
    current_time: C,
    resolve: R,
    mut on_progress: P,
) -> Result<SendRunSummary, String>
where
    R: Fn(&OutboxEntry) -> Option<SendPayload>,
    P: FnMut(&str, UploadStage),
    C: Fn() -> String,
{
    let mut summary = SendRunSummary::default();
    while (summary.claimed as usize) < MAX_SENDS_PER_RUN {
        if stop.is_some_and(|signal| signal.is_stopping()) {
            break;
        }
        let captured_time = current_time();
        let now = captured_time.as_str();
        let claim_token = claim_token(bot_identity, summary.claimed);
        let lease_until = clock::timestamp_after(now, CLAIM_LEASE);
        let claimed = match generation {
            Some(generation) => database.claim_due_outbox_for_generation(
                generation,
                bot_identity,
                &claim_token,
                now,
                &lease_until,
            ),
            None => database.claim_due_outbox(bot_identity, &claim_token, now, &lease_until),
        };
        let Some(entry) = claimed.map_err(|error| error.to_string())? else {
            break;
        };
        summary.claimed += 1;
        let key = entry.idempotency_key.clone();
        let attempt = entry.attempt_count.saturating_sub(1);
        let payload = if entry.payload_schema_version.is_some() {
            entry_payload(&entry).and_then(|payload| match payload {
                SendPayload::Media { .. } => resolve(&entry),
                payload => Some(payload),
            })
        } else {
            None
        };

        // The transport progress callback must be `Send + 'static`, so stages
        // are collected per entry and replayed to the caller afterwards.
        let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
        let collected = Arc::clone(&stages);
        let execution = execute_payload(
            database,
            transport,
            token,
            config,
            bot_identity,
            payload.clone(),
            collected,
        );
        let attempt_future = xarchive_telegram::run_claimed_attempt_with_clock(
            database,
            &claim_token,
            attempt,
            &current_time,
            |delay| clock::timestamp_after(&current_time(), delay),
            execution,
        );
        let guarded = with_claim_heartbeat(database, &claim_token, &current_time, attempt_future);
        tokio::pin!(guarded);
        let mut progress_tick = tokio::time::interval(Duration::from_millis(100));
        let outcome = loop {
            tokio::select! {
                result = &mut guarded => break result,
                _ = progress_tick.tick() => {
                    for stage in std::mem::take(&mut *stages.lock().expect("stages")) {
                        on_progress(&key, stage);
                    }
                }
            }
        };
        for stage in std::mem::take(&mut *stages.lock().expect("stages")) {
            on_progress(&key, stage);
        }

        match outcome {
            Ok(success) => {
                summary.sent += 1;
                // Confirmed results are the only thing that may populate the cache.
                if let Some(payload) = payload {
                    store_confirmed_files(
                        database,
                        bot_identity,
                        &payload,
                        &success,
                        &current_time(),
                    )?;
                }
            }
            Err(RunAttemptError::Failed { classification, .. }) => match classification {
                SendFailure::Unknown => summary.unknown += 1,
                SendFailure::Permanent => summary.failed += 1,
                SendFailure::MediaCorrection => {
                    summary.failed += 1;
                    // Only an explicit "identifier is dead" answer may evict the
                    // cache entry; every other failure keeps it.
                    if let Some(payload) = payload {
                        evict_rejected_files(database, bot_identity, &payload)?;
                    }
                }
                _ => summary.deferred += 1,
            },
            Err(RunAttemptError::StaleClaim) => {
                // Another worker owns the row; its outcome is authoritative.
                summary.deferred += 1;
            }
            Err(RunAttemptError::Store(error)) => return Err(error.to_string()),
        }
    }
    Ok(summary)
}

async fn with_claim_heartbeat<F, C>(
    database: &Database,
    claim: &str,
    clock: &C,
    attempt: F,
) -> Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>
where
    F: std::future::Future<Output = Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>>,
    C: Fn() -> String,
{
    with_claim_heartbeat_interval(database, claim, clock, attempt, Duration::from_secs(60)).await
}

async fn with_claim_heartbeat_interval<F, C>(
    database: &Database,
    claim: &str,
    clock: &C,
    attempt: F,
    interval: Duration,
) -> Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>
where
    F: std::future::Future<Output = Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>>,
    C: Fn() -> String,
{
    let mut attempt = std::pin::pin!(attempt);
    let mut heartbeat = Box::pin(tokio::time::sleep(interval));
    std::future::poll_fn(|context| {
        use std::future::Future;
        // Renew before polling completion when both become ready together.
        if heartbeat.as_mut().poll(context).is_ready() {
            let now = clock();
            let deadline = clock::timestamp_after(&now, CLAIM_LEASE);
            match database.renew_outbox_claim(claim, &deadline, &now) {
                Ok(true) => heartbeat
                    .as_mut()
                    .reset(tokio::time::Instant::now() + interval),
                Ok(false) => return std::task::Poll::Ready(Err(RunAttemptError::StaleClaim)),
                Err(error) => return std::task::Poll::Ready(Err(RunAttemptError::Store(error))),
            }
        }
        attempt.as_mut().poll(context)
    })
    .await
}

fn entry_payload(entry: &OutboxEntry) -> Option<SendPayload> {
    if entry.payload_schema_version != Some(1) {
        return None;
    }
    serde_json::from_str(entry.payload_json.as_deref()?).ok()
}

/// Execute persisted snapshots using the archive store as the only media-path
/// resolver. The generic runner remains available for focused transport tests.
#[allow(clippy::too_many_arguments)]
pub async fn run_due_archived_sends<P>(
    database: &Database,
    files: &FileStore,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    now: &str,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    P: FnMut(&str, UploadStage),
{
    run_due_sends(
        database,
        transport,
        token,
        config,
        bot_identity,
        now,
        |entry| resolve_archived_payload(database, entry, files),
        on_progress,
    )
    .await
}

/// Execute one bounded production batch using only the active credential's
/// secret reference. Disabled sending never reads secrets or claims work.
pub async fn run_active_archived_sends<P>(
    database: &Database,
    files: &FileStore,
    transport: &ReqwestTelegramTransport,
    secrets: &dyn xarchive_telegram::SecretStore,
    config: &TelegramConfig,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    P: FnMut(&str, UploadStage),
{
    run_active_archived_sends_cooperative(
        database,
        files,
        transport,
        secrets,
        config,
        None,
        on_progress,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_active_archived_sends_cooperative<P>(
    database: &Database,
    files: &FileStore,
    transport: &ReqwestTelegramTransport,
    secrets: &dyn xarchive_telegram::SecretStore,
    config: &TelegramConfig,
    stop: Option<&crate::telegram_worker::WorkerStopSignal>,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    P: FnMut(&str, UploadStage),
{
    if stop.is_some_and(|signal| signal.is_stopping()) {
        return Ok(SendRunSummary::default());
    }
    if !config.enabled || config.migration_pending {
        return Ok(SendRunSummary::default());
    }
    recover_expired_claims(database, &clock::now_iso())?;
    // Reconcile completed journals before claiming. A damaged journal remains
    // isolated; it cannot prevent unrelated archives from being sent.
    let _ = reconcile_archived_send_intents(database, &clock::now_iso())?;
    let (generation, identity, reference) = database
        .active_telegram_credential_generation()
        .map_err(|_| "active credential unavailable".to_owned())?
        .ok_or("active credential missing")?;
    let secret = secrets
        .get(&reference)
        .map_err(|_| "secret store unavailable".to_owned())?
        .ok_or("active secret missing")?;
    let token = BotToken::new(secret).map_err(|_| "active secret invalid".to_owned())?;
    if database
        .active_telegram_credential_generation()
        .map_err(|_| "active credential unavailable".to_owned())?
        != Some((generation, identity.clone(), reference))
    {
        return Err("credential generation changed during secret retrieval".into());
    }
    run_due_sends_with_generation(
        database,
        transport,
        &token,
        config,
        &identity,
        Some(generation),
        stop,
        clock::now_iso,
        |entry| resolve_archived_payload(database, entry, files),
        on_progress,
    )
    .await
}

/// The upload timeouts this configuration asks for.
fn upload_timeouts(config: &TelegramConfig) -> UploadTimeouts {
    let processing = Duration::from_secs(config.upload_processing_timeout_seconds.max(1));
    UploadTimeouts {
        connect: Duration::from_secs(config.connect_timeout_seconds.max(1)),
        body_stall: processing,
        server_processing: processing,
        total: None,
    }
}

/// A claim token unique per run, bot and index.
fn claim_token(bot_identity: &str, index: u32) -> String {
    let prefix: String = bot_identity.chars().take(8).collect();
    format!(
        "{}-{prefix}-{index}-{}",
        clock::now_iso(),
        std::process::id()
    )
}

/// Perform one claimed entry's request.
async fn execute_payload(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    payload: Option<SendPayload>,
    stages: Arc<Mutex<Vec<UploadStage>>>,
) -> Result<SendAttemptSuccess, SendAttemptError> {
    let Some(payload) = payload else {
        // The archive no longer has this payload: it needs a new plan, not a
        // silent skip or a blind retry.
        return Err(SendAttemptError::new(
            TelegramError::InvalidUploadRequest(
                "the planned payload is no longer available".to_owned(),
            ),
            RequestProgress::NotSent,
        ));
    };
    let timeouts = upload_timeouts(config);
    let cancellation = CancellationToken::new();
    match payload {
        SendPayload::Message {
            chat_id,
            message_thread_id,
            text,
            ..
        } => {
            let entry_chat_id = chat_id.clone();
            let request = TelegramRequest::Message(SendMessageRequest {
                chat_id,
                message_thread_id,
                text,
                disable_web_page_preview: false,
            });
            // A text send is small: it runs on the blocking pool so no runtime
            // worker thread is parked. It still reports the same stage contract as
            // media so the task projection is not silently empty for text-only
            // archives.
            if let Ok(mut stages) = stages.lock() {
                stages.push(UploadStage::Queued);
                stages.push(UploadStage::AwaitingResult);
            }
            let transport = transport.clone();
            let token = token.clone();
            let response = tokio::task::spawn_blocking(move || transport.send(&token, request))
                .await
                .map_err(|error| {
                    SendAttemptError::new(
                        TelegramError::Transport(format!("sender worker failed: {error}")),
                        RequestProgress::NotSent,
                    )
                })?
                .map_err(|error| {
                    let progress = control_progress(&error);
                    SendAttemptError::new(error, progress)
                })?;
            let confirmed = confirm_cached_send(response, &entry_chat_id);
            if confirmed.is_ok()
                && let Ok(mut stages) = stages.lock()
            {
                stages.push(UploadStage::Confirmed);
            }
            confirmed
        }
        SendPayload::Media {
            chat_id,
            message_thread_id,
            items,
        } if items.len() == 1 => {
            send_single_media(
                database,
                transport,
                token,
                &chat_id,
                message_thread_id,
                bot_identity,
                &items[0],
                &timeouts,
                &cancellation,
                stages,
            )
            .await
        }
        SendPayload::Media {
            chat_id,
            message_thread_id,
            items,
        } => {
            let mut uploads = Vec::with_capacity(items.len());
            for item in &items {
                uploads.push(album_upload(
                    database,
                    bot_identity,
                    &chat_id,
                    message_thread_id,
                    item,
                )?);
            }
            xarchive_telegram::send_media_group_attempt(
                transport,
                token,
                &chat_id,
                message_thread_id,
                &uploads,
                &timeouts,
                &cancellation,
                move |stage| stages.lock().expect("stages").push(stage),
            )
            .await
        }
    }
}

/// Send one media item, reusing a cached `file_id` when the bot has one.
#[allow(clippy::too_many_arguments)]
async fn send_single_media(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    chat_id: &str,
    message_thread_id: Option<i64>,
    bot_identity: &str,
    item: &PlannedMediaItem,
    timeouts: &UploadTimeouts,
    cancellation: &CancellationToken,
    stages: Arc<Mutex<Vec<UploadStage>>>,
) -> Result<SendAttemptSuccess, SendAttemptError> {
    if let Some(cached) = cached_file_id(database, bot_identity, item).map_err(|_| {
        SendAttemptError::new(
            TelegramError::Transport("file_id cache read failed".to_owned()),
            RequestProgress::NotSent,
        )
    })? && let Some(request) =
        cached_media_request(chat_id, message_thread_id, item, &cached.file_id)
    {
        let transport = transport.clone();
        let token = token.clone();
        let response = tokio::task::spawn_blocking(move || transport.send(&token, request))
            .await
            .map_err(|error| {
                SendAttemptError::new(
                    TelegramError::Transport(format!("sender worker failed: {error}")),
                    RequestProgress::NotSent,
                )
            })?
            .map_err(|error| {
                let progress = control_progress(&error);
                SendAttemptError::new(error, progress)
            })?;
        return confirm_cached_send(response, chat_id);
    }
    // A document has no cached-identifier control method, so it is always
    // re-uploaded; a cached photo or video is sent by identifier instead.
    let request = UploadRequest {
        chat_id: chat_id.to_owned(),
        message_thread_id,
        media_kind: item.media_kind,
        file_path: item.file_path.clone(),
        file_name: item.file_name.clone(),
        mime_type: item.mime_type.clone(),
        caption: item.caption.clone(),
        expected_size: item.size_bytes,
    };
    transport
        .send_upload_attempt(token, &request, timeouts, cancellation, move |stage| {
            stages.lock().expect("stages").push(stage)
        })
        .await
}

/// The album body item: a cached identifier when available, else the file.
fn album_upload(
    database: &Database,
    bot_identity: &str,
    chat_id: &str,
    message_thread_id: Option<i64>,
    item: &PlannedMediaItem,
) -> Result<AlbumItemUpload, SendAttemptError> {
    if let Some(cached) = cached_file_id(database, bot_identity, item).map_err(|_| {
        SendAttemptError::new(
            TelegramError::Transport("file_id cache read failed".to_owned()),
            RequestProgress::NotSent,
        )
    })? {
        return Ok(AlbumItemUpload::CachedFileId {
            media_kind: item.media_kind,
            file_id: cached.file_id,
            caption: item.caption.clone(),
        });
    }
    Ok(AlbumItemUpload::File(Box::new(UploadRequest {
        chat_id: chat_id.to_owned(),
        message_thread_id,
        media_kind: item.media_kind,
        file_path: item.file_path.clone(),
        file_name: item.file_name.clone(),
        mime_type: item.mime_type.clone(),
        caption: item.caption.clone(),
        expected_size: item.size_bytes,
    })))
}

/// A cached identifier is sent through the matching JSON method.
///
/// Only photos and videos have a JSON control form; a cached document has no
/// such method, so the caller uploads the file again.
fn cached_media_request(
    chat_id: &str,
    message_thread_id: Option<i64>,
    item: &PlannedMediaItem,
    file_id: &str,
) -> Option<TelegramRequest> {
    match item.media_kind {
        MediaKind::Photo => Some(TelegramRequest::Photo(SendPhotoRequest {
            chat_id: chat_id.to_owned(),
            message_thread_id,
            photo: file_id.to_owned(),
            caption: item.caption.clone(),
        })),
        MediaKind::Video => Some(TelegramRequest::Video(SendVideoRequest {
            chat_id: chat_id.to_owned(),
            message_thread_id,
            video: file_id.to_owned(),
            caption: item.caption.clone(),
        })),
        MediaKind::Document => None,
    }
}

/// Turn a confirmed cached send into the attempt result shape.
fn confirm_cached_send(
    response: xarchive_telegram::TelegramResponse,
    chat_id: &str,
) -> Result<SendAttemptSuccess, SendAttemptError> {
    let telegram_message_id = response.result_message_id().ok_or_else(|| {
        SendAttemptError::new(
            TelegramError::Transport(format!(
                "Telegram confirmed the send to {chat_id} but returned no message id"
            )),
            RequestProgress::Sent,
        )
    })?;
    let file_ids = response.result_file_ids();
    Ok(SendAttemptSuccess {
        telegram_message_id,
        results_json: (!file_ids.is_empty())
            .then(|| serde_json::json!({ "file_ids": file_ids }).to_string()),
    })
}

/// How much is known about a control-path request when it failed.
fn control_progress(error: &TelegramError) -> RequestProgress {
    match error {
        TelegramError::Transport(_) => RequestProgress::NotSent,
        _ => RequestProgress::Sent,
    }
}

/// The cache key of a media item, when its content hash is known.
fn cache_key(bot_identity: &str, item: &PlannedMediaItem) -> Option<FileCacheKey> {
    item.content_sha256.as_ref().map(|sha256| FileCacheKey {
        bot_identity: bot_identity.to_owned(),
        content_sha256: sha256.clone(),
        media_kind: item.media_kind,
        representation_version: TELEGRAM_FILE_CACHE_VERSION,
    })
}

/// The cached identifier of a media item, if this bot already has one.
fn cached_file_id(
    database: &Database,
    bot_identity: &str,
    item: &PlannedMediaItem,
) -> Result<Option<CachedFileId>, String> {
    let Some(key) = cache_key(bot_identity, item) else {
        return Ok(None);
    };
    database
        .lookup_file_id(&key)
        .map_err(|error| error.to_string())
}

/// Store the confirmed identifiers of a payload, item by item.
fn store_confirmed_files(
    database: &Database,
    bot_identity: &str,
    payload: &SendPayload,
    success: &SendAttemptSuccess,
    now: &str,
) -> Result<(), String> {
    let SendPayload::Media { items, .. } = payload else {
        return Ok(());
    };
    for (item, file_id) in items.iter().zip(confirmed_file_ids(success)) {
        let Some(key) = cache_key(bot_identity, item) else {
            continue;
        };
        let value = CachedFileId {
            file_id,
            file_unique_id: String::new(),
            file_size: item.size_bytes.unwrap_or_default(),
            confirmed_at: now.to_owned(),
        };
        database
            .store_file_id(&key, &value)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Evict cache entries Telegram explicitly rejected.
fn evict_rejected_files(
    database: &Database,
    bot_identity: &str,
    payload: &SendPayload,
) -> Result<(), String> {
    let SendPayload::Media { items, .. } = payload else {
        return Ok(());
    };
    for item in items {
        let Some(key) = cache_key(bot_identity, item) else {
            continue;
        };
        database
            .delete_file_id(&key)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// The identifiers a confirmed response reported, in item order.
fn confirmed_file_ids(success: &SendAttemptSuccess) -> Vec<String> {
    success
        .results_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|value| {
            value
                .get("file_ids")
                .and_then(|ids| serde_json::from_value(ids.clone()).ok())
        })
        .unwrap_or_default()
}

/// Whether a failure is Telegram's explicit "identifier is dead" answer.
pub fn invalid_file_id(error: &TelegramError) -> bool {
    is_invalid_file_id_error(error)
}
#[cfg(test)]
mod tests {
    #[test]
    fn concurrent_journal_materialization_has_one_plan_and_one_authority_set() {
        use super::*;
        let root = std::env::temp_dir().join(format!(
            "tg-concurrent-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("archive.sqlite3");
        let database = Database::open(&path).unwrap();
        let generation = database
            .prepare_telegram_credential_generation("telegram-bot:321", "native-ref", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(generation, None)
            .unwrap();
        let tweet = tweet_row(&database, "concurrent-plan");
        database
            .create_archive_job("concurrent-job", tweet, "t0")
            .unwrap();
        let config = TelegramConfig {
            enabled: true,
            auto_send_on_archive: true,
            chat_id: "-100123".into(),
            ..Default::default()
        };
        let intent = ArchiveSendIntent::from_verified_config(
            tweet,
            "Tweets/concurrent-plan".into(),
            Some("hello".into()),
            vec![],
            &config,
            "telegram-bot:321",
        )
        .unwrap();
        database
            .prepare_telegram_archive_intent(
                &intent.to_journal_record("concurrent-job", "t0").unwrap(),
            )
            .unwrap();
        database
            .transition_telegram_archive_intent(tweet, "PREPARED", "ARCHIVED", "t1")
            .unwrap();
        drop(database);
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let database = Database::open(path).unwrap();
                    barrier.wait();
                    queue_persisted_archive_sends(&database, tweet, "t2")
                })
            })
            .collect();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        // SQLite may reject a competing deferred writer. Retrying is safe and
        // must converge to the same identity without partial materialization.
        assert!(results.iter().any(Result::is_ok));
        let database = Database::open(&path).unwrap();
        let ids = queue_persisted_archive_sends(&database, tweet, "t3").unwrap();
        assert_eq!(ids.len(), 1);
        for result in results.into_iter().flatten() {
            assert_eq!(result, ids);
        }
        assert!(
            database
                .telegram_resume_authorized(generation, ids[0])
                .unwrap()
        );
        assert_eq!(database.list_all_outbox_for_tweet(tweet).unwrap().len(), 1);
        drop(database);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn journal_restart_sends_once_and_api_failure_preserves_local_archive() {
        use super::*;
        for failure in [false, true] {
            let root = std::env::temp_dir().join(format!(
                "123456-{failure}-{}-{}",
                std::process::id(),
                crate::runtime::timestamp_marker()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let path = root.join("archive.sqlite3");
            let database = Database::open(&path).unwrap();
            let tweet = database
                .insert_tweet(
                    "123456",
                    "https://x.com/a/status/123456",
                    "post",
                    "archived",
                    "t0",
                )
                .unwrap();
            database
                .create_archive_job("chain-job", tweet, "t0")
                .unwrap();
            let generation = database
                .prepare_telegram_credential_generation("telegram-bot:123", "native-ref", "t0")
                .unwrap();
            database
                .activate_telegram_credential_generation(generation, None)
                .unwrap();
            let raw = serde_json::json!({"tweet_id":"123456", "tweet_url":"https://x.com/a/status/123456", "text":"hello", "media":[]});
            let files = FileStore::new(root.join("archive")).unwrap();
            let result = crate::archive::SidecarArchiveResult {
                metadata: raw.clone(),
                files: vec![],
            };
            let config = TelegramConfig {
                enabled: true,
                auto_send_on_archive: true,
                chat_id: "-100123".into(),
                ..Default::default()
            };
            let intent = crate::archive::capture_completed_intent(
                &database,
                &files,
                "chain-job",
                tweet,
                "123456",
                &result,
                std::path::Path::new("Tweets/123456"),
                "2026-10-04T00:00:00Z",
                &config,
            )
            .unwrap();
            let mut service = xarchive_storage::ArchiveService::new(database, files);
            service
                .complete_sidecar_archive(xarchive_storage::SidecarArchiveRequest {
                    job_id: "chain-job",
                    tweet_row_id: tweet,
                    expected_tweet_id: "123456",
                    metadata: &raw,
                    files: &[],
                    final_directory: std::path::Path::new("Tweets/123456"),
                    archived_at: "2026-10-04T00:00:00Z",
                    telegram_intent: Some(&intent),
                })
                .unwrap();
            drop(service);
            let database = Database::open(&path).unwrap();
            let files = FileStore::new(root.join("archive")).unwrap();
            let body = if failure {
                r#"{"ok":false,"error_code":403,"description":"forbidden"}"#
            } else {
                r#"{"ok":true,"result":{"message_id":42,"chat":{"id":-100123}}}"#
            };
            let (endpoint, server) = confirmed_server(body);
            let mut sender_config = config;
            sender_config.endpoint_mode = xarchive_telegram::EndpointMode::Local;
            sender_config.api_base = endpoint;
            let transport = ReqwestTelegramTransport::with_api_endpoint(
                sender_config.endpoint().unwrap(),
                Duration::from_secs(2),
                None,
            )
            .unwrap();
            let mut secrets = xarchive_telegram::MemorySecretStore::default();
            xarchive_telegram::SecretStore::set(&mut secrets, "native-ref", "123:fixture").unwrap();
            let summary = block_on(run_active_archived_sends(
                &database,
                &files,
                &transport,
                &secrets,
                &sender_config,
                |_, _| {},
            ))
            .unwrap();
            assert_eq!(summary.claimed, 1);
            let request = server.join().unwrap();
            assert!(!request.is_empty());
            let rows = database.list_all_outbox_for_tweet(tweet).unwrap();
            assert_eq!(
                rows[0].state,
                if failure {
                    OutboxState::FailedPermanent
                } else {
                    OutboxState::Sent
                }
            );
            assert_eq!(
                database.job_state("chain-job").unwrap(),
                xarchive_core::JobState::Downloaded
            );
            assert!(root.join("archive/Tweets/123456/tweet.json").is_file());
            assert_eq!(
                block_on(run_active_archived_sends(
                    &database,
                    &files,
                    &transport,
                    &secrets,
                    &sender_config,
                    |_, _| {}
                ))
                .unwrap()
                .claimed,
                0
            );
            drop(database);
            drop(files);
            drop(transport);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn new_journal_rows_are_authorized_but_replay_never_grants_after_rotation() {
        use super::*;
        let database = Database::open_in_memory().unwrap();
        let first = database
            .prepare_telegram_credential_generation("telegram-bot:123", "first", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(first, None)
            .unwrap();
        let tweet_id = tweet_row(&database, "new-authority");
        database
            .create_archive_job("new-authority-job", tweet_id, "t0")
            .unwrap();
        let config = TelegramConfig {
            enabled: true,
            auto_send_on_archive: true,
            chat_id: "-100123".into(),
            ..Default::default()
        };
        let intent = ArchiveSendIntent::from_verified_config(
            tweet_id,
            "Tweets/new-authority".into(),
            Some("message".into()),
            vec![],
            &config,
            "telegram-bot:123",
        )
        .unwrap();
        database
            .prepare_telegram_archive_intent(
                &intent.to_journal_record("new-authority-job", "t0").unwrap(),
            )
            .unwrap();
        database
            .transition_telegram_archive_intent(tweet_id, "PREPARED", "ARCHIVED", "t1")
            .unwrap();
        let ids = queue_persisted_archive_sends(&database, tweet_id, "t2").unwrap();
        assert!(database.telegram_resume_authorized(first, ids[0]).unwrap());
        let second = database
            .prepare_telegram_credential_generation("telegram-bot:123", "second", "t3")
            .unwrap();
        database
            .activate_telegram_credential_generation(second, Some(first))
            .unwrap();
        assert_eq!(
            queue_persisted_archive_sends(&database, tweet_id, "t4").unwrap(),
            ids
        );
        assert!(!database.telegram_resume_authorized(second, ids[0]).unwrap());
    }
    use super::*;

    use std::io::{Read as _, Write as _};

    use xarchive_storage::{Database, FileStore};
    use xarchive_telegram::OutboxState;

    /// A one-shot loopback server answering with `body`, so the sender path can
    /// be exercised without touching the real Bot API.
    fn confirmed_server(body: &'static str) -> (String, std::thread::JoinHandle<Vec<u8>>) {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
        let address = format!("http://{}", listener.local_addr().expect("local address"));
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accepted a request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("read timeout");
            let mut received = Vec::new();
            let mut buffer = [0_u8; 4096];
            while let Ok(count) = stream.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                received.extend_from_slice(&buffer[..count]);
                if received.len() >= expected_request_length(&received) {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            received
        });
        (address, handle)
    }

    /// Total request length when the body length is known from the headers.
    fn expected_request_length(received: &[u8]) -> usize {
        let text = String::from_utf8_lossy(received);
        let Some(headers) = text.split("\r\n\r\n").next() else {
            return usize::MAX;
        };
        let lower = headers.to_ascii_lowercase();
        match lower.lines().find_map(|line| {
            line.strip_prefix("content-length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        }) {
            Some(length) => text
                .find("\r\n\r\n")
                .map_or(usize::MAX, |index| index + 4 + length),
            // A chunked body is read until the client closes its side.
            None => usize::MAX,
        }
    }

    fn configured() -> TelegramConfig {
        TelegramConfig {
            enabled: true,
            auto_send_on_archive: true,
            chat_id: "-100777".to_owned(),
            ..TelegramConfig::default()
        }
    }

    #[test]
    fn candidate_activation_preserves_active_secret_on_stale_generation() {
        use xarchive_telegram::{MemorySecretStore, SecretStore, TelegramResponse};
        struct VerifiedTransport;
        impl TelegramTransport for VerifiedTransport {
            fn send(
                &self,
                _: &BotToken,
                request: TelegramRequest,
            ) -> Result<TelegramResponse, TelegramError> {
                assert_eq!(request, TelegramRequest::GetMe);
                Ok(serde_json::from_value(
                    serde_json::json!({"ok":true,"result":{"id":42,"is_bot":true}}),
                )
                .expect("response"))
            }
        }
        let database = Database::open_in_memory().expect("database");
        let mut store = MemorySecretStore::default();
        let first = activate_verified_credential(
            &database,
            &mut store,
            &VerifiedTransport,
            "old-token",
            "secret/old",
            None,
            "t0",
        )
        .expect("activate");
        assert!(
            activate_verified_credential(
                &database,
                &mut store,
                &VerifiedTransport,
                "new-token",
                "secret/stale",
                None,
                "t1"
            )
            .is_err()
        );
        assert_eq!(store.get("secret/old").unwrap(), Some("old-token".into()));
        assert_eq!(store.get("secret/stale").unwrap(), None);
        assert_eq!(
            database
                .active_telegram_credential_generation()
                .unwrap()
                .unwrap()
                .0,
            first
        );
        let intent = ArchiveSendIntent {
            tweet_row_id: tweet_row(&database, "rotation-policy"),
            archive_directory: "archive".into(),
            metadata_text: Some("message".into()),
            media: Vec::new(),
            plan_version: 1,
            bot_identity: "telegram-bot:42".into(),
            chat_id: "-100".into(),
            message_thread_id: None,
            config_revision: 1,
        };
        let ids = queue_archive_sends(&database, &intent, "t2").expect("queue");
        use crate::config::CredentialRotationResumePolicy;
        let second = activate_verified_credential_with_policy(
            &database,
            &mut store,
            &VerifiedTransport,
            "new-token",
            "secret/new",
            Some(first),
            CredentialRotationResumePolicy::Confirm,
            "t2",
        )
        .expect("rotate");
        assert_ne!(first, second);
        assert_eq!(store.get("secret/new").unwrap(), Some("new-token".into()));
        assert!(
            apply_credential_resume_policy(
                &database,
                second,
                Some("telegram-bot:43"),
                CredentialRotationResumePolicy::Automatic,
                "t3"
            )
            .unwrap()
            .is_empty()
        );
        assert_eq!(
            apply_credential_resume_policy(
                &database,
                second,
                Some("telegram-bot:42"),
                CredentialRotationResumePolicy::Confirm,
                "t3"
            )
            .unwrap(),
            ids
        );
        assert!(!database.telegram_resume_authorized(second, ids[0]).unwrap());
        assert_eq!(
            apply_credential_resume_policy(
                &database,
                second,
                Some("telegram-bot:42"),
                CredentialRotationResumePolicy::Automatic,
                "t4"
            )
            .unwrap(),
            ids
        );
        // Changing the setting cannot retroactively authorize a captured
        // confirmation decision. An explicit confirmation is required.
        assert!(!database.telegram_resume_authorized(second, ids[0]).unwrap());
        database
            .authorize_telegram_resume(second, &ids, "t5")
            .unwrap();
        assert!(database.telegram_resume_authorized(second, ids[0]).unwrap());
        assert_eq!(
            database
                .active_telegram_credential_generation()
                .unwrap()
                .unwrap()
                .0,
            second
        );
    }

    #[test]
    fn caption_layout_is_versioned_and_preserves_existing_media_text() {
        let database = Database::open_in_memory().unwrap();
        let tweet = tweet_row(&database, "caption-layout");
        let mut item = photo_plan("photo", "sha");
        item.caption = None;
        let mut intent = test_intent(&configured(), tweet, "archive", Some("正文"), vec![item]);
        intent.plan_version = 2;
        assert_eq!(archive_plan_entries(&intent, "t0").unwrap().len(), 2);
        intent.plan_version = 3;
        let entries = archive_plan_entries(&intent, "t0").unwrap();
        assert_eq!(entries.len(), 1);
        let payload: SendPayload = serde_json::from_str(&entries[0].payload_json).unwrap();
        match payload {
            SendPayload::Media { items, .. } => {
                assert_eq!(items[0].caption.as_deref(), Some("正文"))
            }
            _ => panic!("expected combined media"),
        }
        intent.media[0].caption = Some("original caption".into());
        assert_eq!(archive_plan_entries(&intent, "t0").unwrap().len(), 2);
        intent.media[0].caption = None;
        intent.metadata_text = Some("字".repeat(xarchive_telegram::TELEGRAM_CAPTION_LIMIT + 1));
        assert_eq!(archive_plan_entries(&intent, "t0").unwrap().len(), 2);
    }

    #[test]
    fn long_archive_text_is_split_into_stable_ordered_units() {
        let database = Database::open_in_memory().unwrap();
        let tweet_id = tweet_row(&database, "long-text");
        let text = "字".repeat(xarchive_telegram::TELEGRAM_TEXT_LIMIT + 1);
        let mut intent = test_intent(&configured(), tweet_id, "archive", Some(&text), vec![]);
        let legacy = archive_plan_entries(&intent, "t0").unwrap();
        assert_eq!(legacy.len(), 1);
        assert_eq!(
            legacy[0].idempotency_key,
            format!("tweet-{tweet_id}:metadata")
        );
        intent.plan_version = 2;
        let entries = archive_plan_entries(&intent, "t0").unwrap();
        assert_eq!(entries.len(), 2);
        for (index, entry) in entries.iter().enumerate() {
            assert_eq!(entry.plan_order, index as i64);
            assert_eq!(
                entry.idempotency_key,
                format!("tweet-{tweet_id}:metadata:{index}")
            );
        }
        let ids = queue_archive_sends(&database, &intent, "t0").unwrap();
        assert_eq!(queue_archive_sends(&database, &intent, "t1").unwrap(), ids);
        let rows = database.list_outbox_for_tweet("bot-a", tweet_id).unwrap();
        let restored: String = rows
            .iter()
            .map(|row| match restore_payload_from_entry(row).unwrap() {
                SendPayload::Message { text, .. } => {
                    assert!(text.chars().count() <= xarchive_telegram::TELEGRAM_TEXT_LIMIT);
                    text
                }
                _ => panic!("expected text"),
            })
            .collect();
        assert_eq!(restored, text);
    }

    #[test]
    fn persisted_archive_plan_rejects_corrupt_and_unknown_snapshots_without_progress() {
        for (label, media_json, plan_version) in [
            ("invalid-json", "{", 1),
            ("invalid-shape", "{}", 1),
            ("unknown-version", "[]", 99),
        ] {
            let database = Database::open_in_memory().expect("database");
            let tweet_id = tweet_row(&database, label);
            database
                .create_archive_job(label, tweet_id, "t0")
                .expect("job");
            let record = xarchive_storage::TelegramArchiveIntentRecord {
                job_id: label.into(),
                tweet_row_id: tweet_id,
                archive_directory: "archive".into(),
                state: "PREPARED".into(),
                metadata_text: Some("message".into()),
                media_json: media_json.into(),
                bot_identity: Some("bot".into()),
                chat_id: Some("-100".into()),
                message_thread_id: None,
                config_revision: Some(1),
                plan_version,
                created_at: "t0".into(),
                updated_at: "t0".into(),
            };
            database
                .prepare_telegram_archive_intent(&record)
                .expect("prepare");
            database
                .transition_telegram_archive_intent(tweet_id, "PREPARED", "ARCHIVED", "t1")
                .expect("archive");
            assert!(
                queue_persisted_archive_sends(&database, tweet_id, "t2").is_err(),
                "{label}"
            );
            assert!(
                database
                    .list_outbox_for_tweet("bot", tweet_id)
                    .expect("rows")
                    .is_empty(),
                "{label}"
            );
            let stored = database
                .telegram_archive_intent(tweet_id)
                .expect("journal")
                .expect("intent");
            assert_eq!(stored.state, "ARCHIVED", "{label}");
            assert_eq!(stored.updated_at, "t1", "{label}");
            let results = reconcile_archived_send_intents(&database, "t3").expect("scan");
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].0, tweet_id);
            assert!(results[0].1.is_err());
        }
    }

    #[test]
    fn persisted_archive_plan_is_atomic_and_replayable() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "persisted-plan");
        database
            .create_archive_job("persisted-job", tweet_id, "t0")
            .expect("job");
        let record = xarchive_storage::TelegramArchiveIntentRecord {
            job_id: "persisted-job".into(),
            tweet_row_id: tweet_id,
            archive_directory: "archive".into(),
            state: "PREPARED".into(),
            metadata_text: Some("captured message".into()),
            media_json: "[]".into(),
            bot_identity: Some("original-bot".into()),
            chat_id: Some("-100-original".into()),
            message_thread_id: Some(42),
            config_revision: Some(7),
            plan_version: 1,
            created_at: "t0".into(),
            updated_at: "t0".into(),
        };
        database
            .prepare_telegram_archive_intent(&record)
            .expect("prepare");
        assert!(queue_persisted_archive_sends(&database, tweet_id, "t1").is_err());
        assert!(
            reconcile_archived_send_intents(&database, "t1")
                .expect("scan")
                .is_empty()
        );
        assert!(
            database
                .list_outbox_for_tweet("original-bot", tweet_id)
                .expect("rows")
                .is_empty()
        );
        database
            .transition_telegram_archive_intent(tweet_id, "PREPARED", "ARCHIVED", "t1")
            .expect("archive");
        let ids = queue_persisted_archive_sends(&database, tweet_id, "t2").expect("queue");
        assert_eq!(ids.len(), 1);
        assert!(
            reconcile_archived_send_intents(&database, "t2")
                .expect("scan")
                .is_empty()
        );
        assert_eq!(
            queue_persisted_archive_sends(&database, tweet_id, "t3").expect("replay"),
            ids
        );
        let rows = database
            .list_outbox_for_tweet("original-bot", tweet_id)
            .expect("rows");
        assert_eq!(rows[0].chat_id, "-100-original");
        assert_eq!(rows[0].message_thread_id, Some(42));
        assert_eq!(rows[0].config_version, 7);
        assert_eq!(
            database
                .telegram_archive_intent(tweet_id)
                .expect("journal")
                .expect("intent")
                .state,
            "QUEUED"
        );
    }

    #[tokio::test]
    async fn heartbeat_renews_a_pending_attempt_before_its_original_deadline() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "heartbeat");
        let config = configured();
        queue_archive_sends(
            &database,
            &test_intent(&config, tweet_id, "archive", Some("message"), vec![]),
            "2026-10-04T00:00:00Z",
        )
        .expect("queue");
        database
            .claim_due_outbox(
                "bot-a",
                "heartbeat-claim",
                "2026-10-04T00:00:00Z",
                "2026-10-04T00:00:02Z",
            )
            .expect("claim")
            .expect("row");
        let attempt = async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Err(RunAttemptError::StaleClaim)
        };
        let _ = with_claim_heartbeat_interval(
            &database,
            "heartbeat-claim",
            &|| "2026-10-04T00:00:01Z".to_owned(),
            attempt,
            Duration::from_millis(1),
        )
        .await;
        let row = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows")
            .remove(0);
        assert_eq!(
            row.claim_expires_at.as_deref(),
            Some("2026-10-04T00:05:01Z")
        );
        assert_eq!(row.state, xarchive_telegram::OutboxState::InFlight);
    }

    #[tokio::test]
    async fn heartbeat_loss_drops_pending_attempt_without_polling_it_to_completion() {
        let database = Database::open_in_memory().expect("database");
        let completed = std::sync::atomic::AtomicBool::new(false);
        let attempt = async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            completed.store(true, std::sync::atomic::Ordering::SeqCst);
            Err(RunAttemptError::StaleClaim)
        };
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            with_claim_heartbeat_interval(
                &database,
                "missing-claim",
                &|| "2026-10-04T00:00:00Z".to_owned(),
                attempt,
                Duration::from_millis(1),
            ),
        )
        .await
        .expect("heartbeat must stop pending attempt");
        assert!(matches!(result, Err(RunAttemptError::StaleClaim)));
        assert!(!completed.load(std::sync::atomic::Ordering::SeqCst));
    }

    fn test_intent(
        config: &TelegramConfig,
        tweet_id: i64,
        archive_directory: &str,
        metadata_text: Option<&str>,
        media: Vec<PlannedMediaItem>,
    ) -> ArchiveSendIntent {
        ArchiveSendIntent {
            tweet_row_id: tweet_id,
            archive_directory: archive_directory.to_owned(),
            metadata_text: metadata_text.map(str::to_owned),
            media,
            plan_version: 1,
            bot_identity: "bot-a".to_owned(),
            chat_id: config.chat_id.clone(),
            message_thread_id: config.message_thread_id,
            config_revision: config.revision,
        }
    }

    fn topic_intent(
        config: &TelegramConfig,
        tweet_id: i64,
        media: Vec<PlannedMediaItem>,
    ) -> ArchiveSendIntent {
        ArchiveSendIntent {
            tweet_row_id: tweet_id,
            archive_directory: "archive".to_owned(),
            metadata_text: Some("topic message".to_owned()),
            media,
            plan_version: 1,
            bot_identity: "bot-a".to_owned(),
            chat_id: config.chat_id.clone(),
            message_thread_id: Some(42),
            config_revision: config.revision,
        }
    }

    /// A tweet row an outbox entry can reference, matching its row id.
    ///
    /// The outbox keeps a foreign key to `tweets(id)`, so a test must own the
    /// tweet whose sends it queues.
    fn tweet_row(database: &Database, label: &str) -> i64 {
        database
            .insert_tweet(
                &format!("tg-{label}"),
                &format!("https://x.com/a/status/tg-{label}"),
                "post",
                "archived",
                "2026-10-01T00:00:00Z",
            )
            .expect("tweet row")
    }

    /// Runs a future to completion.
    ///
    /// A blocking Bot API client must be constructed outside any runtime
    /// context, so tests drive the async sender from a runtime of their own
    /// and drop it afterwards.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(future)
    }

    fn photo_plan(name: &str, sha: &str) -> PlannedMediaItem {
        PlannedMediaItem {
            media_kind: MediaKind::Photo,
            file_path: format!("archive/media/{name}.jpg").into(),
            file_name: format!("{name}.jpg"),
            mime_type: Some("image/jpeg".to_owned()),
            caption: None,
            content_sha256: Some(sha.to_owned()),
            size_bytes: Some(9),
        }
    }

    fn outbox_entry(
        database: &Database,
        tweet_id: i64,
        sha: &str,
        archive_directory: &str,
        media_path: &str,
    ) -> OutboxEntry {
        let plan = plan_media_sends(
            "-100777",
            None,
            vec![PlannedMediaItem {
                file_path: media_path.into(),
                ..photo_plan("first", sha)
            }],
            &format!("tweet-{tweet_id}:media"),
        )
        .remove(0);
        let entry = plan
            .to_outbox_entry(
                "bot-a",
                archive_directory,
                Some(tweet_id),
                1,
                1,
                0,
                "2026-10-01T00:00:00Z",
            )
            .expect("snapshot");
        database.enqueue_outbox(entry).expect("persist snapshot");
        database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("outbox rows")
            .into_iter()
            .next()
            .expect("entry")
    }

    #[test]
    fn archived_snapshot_resolves_against_archive_facts_and_rejects_unsafe_paths() {
        let root = std::env::temp_dir().join(format!("tg-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let media_dir = root.join("archive/media");
        std::fs::create_dir_all(&media_dir).expect("archive directory");
        let media_path = media_dir.join("first.jpg");
        std::fs::write(&media_path, b"image").expect("media");
        let digest = FileStore::sha256(&media_path).expect("digest");
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "restore");
        database
            .update_tweet_metadata(
                tweet_id,
                &xarchive_core::ArchiveMetadata {
                    schema_version: 1,
                    tweet_id: "tg-restore".into(),
                    url: "https://x.com/a/status/tg-restore".into(),
                    tweet_type: "post".into(),
                    author: xarchive_core::ArchiveAuthor {
                        user_id: None,
                        username: None,
                        display_name: None,
                    },
                    created_at: None,
                    text: String::new(),
                    media: Vec::new(),
                    archived_at: "2026-10-01T00:00:00Z".into(),
                    reply_to: None,
                    quoted_tweet: None,
                },
                "archive",
            )
            .expect("archive metadata");
        database
            .insert_media(
                tweet_id,
                &xarchive_core::ArchiveMedia {
                    index: 1,
                    media_id: Some("first".into()),
                    media_type: "photo".into(),
                    file: "media/first.jpg".into(),
                    mime_type: Some("image/jpeg".into()),
                    size_bytes: 5,
                    sha256: digest.clone(),
                },
                "2026-10-01T00:00:00Z",
            )
            .expect("media fact");
        let entry = outbox_entry(
            &database,
            tweet_id,
            &digest,
            "archive",
            "archive/media/first.jpg",
        );
        let files = FileStore::new(&root).expect("file store");
        assert_eq!(
            entry.media_reference.as_deref(),
            Some("archive/media/first.jpg")
        );
        let restored = resolve_archived_payload(&database, &entry, &files).expect("resolved");
        let SendPayload::Media { items, .. } = restored else {
            panic!("media payload expected");
        };
        assert_eq!(items[0].file_path, media_path);

        let mut unsafe_entry = entry.clone();
        unsafe_entry.payload_json = unsafe_entry
            .payload_json
            .as_deref()
            .expect("snapshot JSON")
            .replace("media/first.jpg", "../../outside.jpg")
            .into();
        assert!(resolve_archived_payload(&database, &unsafe_entry, &files).is_none());

        #[cfg(unix)]
        {
            let outside = root.with_extension("outside");
            std::fs::create_dir_all(&outside).expect("outside");
            std::fs::write(outside.join("linked.jpg"), b"image").expect("outside file");
            std::os::unix::fs::symlink(&outside, media_dir.join("linked")).expect("symlink");
            let mut linked_entry = entry.clone();
            let mut linked_payload: SendPayload =
                serde_json::from_str(entry.payload_json.as_deref().unwrap()).unwrap();
            if let SendPayload::Media { items, .. } = &mut linked_payload {
                items[0].file_path = "archive/media/linked/linked.jpg".into();
            }
            linked_entry.payload_json = Some(serde_json::to_string(&linked_payload).unwrap());
            let linked_fact = xarchive_core::ArchiveMedia {
                index: 2,
                media_id: Some("linked".into()),
                media_type: "photo".into(),
                file: "media/linked/linked.jpg".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 5,
                sha256: digest.clone(),
            };
            database
                .insert_media(tweet_id, &linked_fact, "2026-10-01T00:00:00Z")
                .expect("linked media fact");
            assert!(resolve_archived_payload(&database, &linked_entry, &files).is_none());
            let _ = std::fs::remove_dir_all(outside);
        }

        let traversal_database = Database::open_in_memory().expect("traversal database");
        let traversal_tweet = tweet_row(&traversal_database, "traversal");
        traversal_database
            .update_tweet_metadata(
                traversal_tweet,
                &xarchive_core::ArchiveMetadata {
                    schema_version: 1,
                    tweet_id: "tg-traversal".into(),
                    url: "https://x.com/a/status/tg-traversal".into(),
                    tweet_type: "post".into(),
                    author: xarchive_core::ArchiveAuthor {
                        user_id: None,
                        username: None,
                        display_name: None,
                    },
                    created_at: None,
                    text: String::new(),
                    media: Vec::new(),
                    archived_at: "2026-10-01T00:00:00Z".into(),
                    reply_to: None,
                    quoted_tweet: None,
                },
                "../escape",
            )
            .expect("set unsafe archive fact");
        // The snapshot was queued while the committed directory was still safe;
        // the persisted fact is now stale and unsafe, and recovery must reject
        // it instead of resolving media outside the archive root.
        let traversal_entry = outbox_entry(
            &traversal_database,
            traversal_tweet,
            &digest,
            "archive",
            "archive/media/first.jpg",
        );
        assert!(resolve_archived_payload(&traversal_database, &traversal_entry, &files).is_none());

        let unarchived = tweet_row(&database, "unarchived");
        let missing_entry = outbox_entry(
            &database,
            unarchived,
            &digest,
            "archive",
            "archive/media/first.jpg",
        );
        assert!(resolve_archived_payload(&database, &missing_entry, &files).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_null_snapshot_is_not_reconstructed_from_current_settings() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "legacy-snapshot");
        let config = configured();
        let plan = plan_text_send(
            config.chat_id.clone(),
            None,
            "original text",
            format!("tweet-{tweet_id}:metadata"),
        )
        .to_outbox_entry(
            "bot-a",
            "Tweets/legacy-snapshot",
            Some(tweet_id),
            config.revision,
            1,
            0,
            "2026-10-01T00:00:00Z",
        )
        .expect("outbox plan");
        let id = database.enqueue_outbox(plan).expect("enqueue");
        database
            .execute_batch(&format!(
                "UPDATE telegram_outbox SET payload_schema_version = NULL, payload_json = NULL WHERE id = {id}"
            ))
            .expect("simulate legacy row");
        let entry = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("outbox")
            .remove(0);
        assert!(entry_payload(&entry).is_none());
        assert!(restore_payload_from_entry(&entry).is_none());
    }

    #[test]
    fn automatic_sending_requires_the_switch_and_a_target() {
        let mut config = configured();
        assert!(auto_send_enabled(&config));
        config.auto_send_on_archive = false;
        assert!(!auto_send_enabled(&config));
        config.auto_send_on_archive = true;
        config.chat_id = String::new();
        assert!(!auto_send_enabled(&config));
        config.chat_id = "-100777".to_owned();
        config.enabled = false;
        assert!(!auto_send_enabled(&config));
    }

    #[test]
    fn a_pending_endpoint_migration_suspends_automatic_sending() {
        let mut config = configured();
        config.migration_pending = true;
        assert!(!auto_send_enabled(&config));
        // The pause survives re-enabling both switches; it is not a UI toggle.
        config.enabled = true;
        config.auto_send_on_archive = true;
        assert!(!auto_send_enabled(&config));
        config.migration_pending = false;
        assert!(auto_send_enabled(&config));
    }

    #[test]
    fn queueing_an_archive_is_idempotent_and_ordered() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let media = vec![
            photo_plan("first", "sha-first"),
            photo_plan("second", "sha-second"),
        ];
        let intent = ArchiveSendIntent {
            tweet_row_id: tweet_id,
            archive_directory: "archive".to_owned(),
            metadata_text: Some("caption".to_owned()),
            media: media.clone(),
            plan_version: 1,
            bot_identity: "bot-a".to_owned(),
            chat_id: config.chat_id.clone(),
            message_thread_id: config.message_thread_id,
            config_revision: config.revision,
        };
        let ids = queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");
        assert_eq!(ids.len(), 2, "one message plus one two-item album");

        let rows = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows");
        let kinds: Vec<&str> = rows.iter().map(|row| row.message_kind.as_str()).collect();
        assert_eq!(kinds, ["message", "media_album"]);
        let orders: Vec<i64> = rows.iter().map(|row| row.plan_order).collect();
        assert_eq!(orders, [0, 1], "plan order is the send order");
        assert!(
            rows.iter().all(|row| row.config_version == config.revision),
            "the settings revision the entry was queued under is recorded"
        );

        // Re-queueing the same plan must not duplicate work.
        let repeated =
            queue_archive_sends(&database, &intent, "2026-10-01T00:05:00Z").expect("requeue");
        assert_eq!(repeated, ids, "the same rows are returned");
        assert_eq!(
            database
                .list_outbox_for_tweet("bot-a", tweet_id)
                .expect("rows")
                .len(),
            2
        );
    }

    #[test]
    fn an_empty_plan_queues_nothing() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "empty");
        let intent = ArchiveSendIntent {
            tweet_row_id: tweet_id,
            archive_directory: "archive".to_owned(),
            metadata_text: Some("   ".to_owned()),
            media: Vec::new(),
            plan_version: 1,
            bot_identity: "bot-a".to_owned(),
            chat_id: "-100777".to_owned(),
            message_thread_id: None,
            config_revision: 1,
        };
        let ids = queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");
        assert!(ids.is_empty());
        assert!(
            database
                .list_outbox_for_tweet("bot-a", tweet_id)
                .expect("rows")
                .is_empty()
        );
    }
    #[test]
    fn archive_metadata_mapping_orders_media_and_rejects_unsafe_references() {
        let mut metadata = xarchive_core::ArchiveMetadata {
            schema_version: 1,
            tweet_id: "123".into(),
            url: "https://x.com/a/status/123".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: Some("a".into()),
                display_name: None,
            },
            created_at: None,
            text: "正文".into(),
            archived_at: "2026-10-04T00:00:00Z".into(),
            reply_to: None,
            quoted_tweet: None,
            media: vec![
                xarchive_core::ArchiveMedia {
                    index: 1,
                    media_id: None,
                    media_type: "video".into(),
                    file: "media/1.mp4".into(),
                    mime_type: Some("video/mp4".into()),
                    size_bytes: 4,
                    sha256: "a".repeat(64),
                },
                xarchive_core::ArchiveMedia {
                    index: 0,
                    media_id: None,
                    media_type: "photo".into(),
                    file: "media/0.jpg".into(),
                    mime_type: Some("image/jpeg".into()),
                    size_bytes: 3,
                    sha256: "b".repeat(64),
                },
            ],
        };
        let capture = |metadata: &xarchive_core::ArchiveMetadata| {
            ArchiveSendIntent::from_archive_metadata(
                1,
                "Tweets/123".into(),
                metadata,
                &configured(),
                "telegram-bot:42",
            )
        };
        let intent = capture(&metadata).unwrap();
        let record = intent
            .to_journal_record("archive-job", "2026-10-04T00:00:00Z")
            .unwrap();
        assert_eq!(record.state, "PREPARED");
        assert_eq!(record.bot_identity.as_deref(), Some("telegram-bot:42"));
        assert_eq!(record.plan_version, intent.plan_version);
        assert_eq!(
            serde_json::from_str::<Vec<PlannedMediaItem>>(&record.media_json).unwrap(),
            intent.media
        );
        assert!(
            intent
                .to_journal_record("", "2026-10-04T00:00:00Z")
                .is_err()
        );
        assert_eq!(
            intent.media[0].file_path,
            std::path::PathBuf::from("Tweets/123/media/0.jpg")
        );
        assert_eq!(intent.media[0].media_kind, MediaKind::Photo);
        let mut original_config = configured();
        original_config.upload_mode = xarchive_telegram::UploadMode::OriginalFile;
        let original = ArchiveSendIntent::from_archive_metadata(
            1,
            "Tweets/123".into(),
            &metadata,
            &original_config,
            "telegram-bot:42",
        )
        .unwrap();
        assert!(
            original
                .media
                .iter()
                .all(|item| item.media_kind == MediaKind::Document)
        );
        assert!(intent.metadata_text.unwrap().contains("正文"));
        metadata.media[0].file = "../escape.mp4".into();
        assert!(capture(&metadata).is_err());
        metadata.media[0].file = "media/1.mp4".into();
        metadata.media[0].index = 0;
        assert!(capture(&metadata).is_err());
    }

    #[test]
    fn stopped_active_sender_never_reads_credentials_or_connects() {
        struct ForbiddenSecrets;
        impl xarchive_telegram::SecretStore for ForbiddenSecrets {
            fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
                panic!("stopped sender must not read credentials");
            }
            fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("sender must not write credentials");
            }
            fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("sender must not delete credentials");
            }
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let mut worker = crate::telegram_worker::TelegramWorker::start_cooperative(
            Duration::from_secs(60),
            move |signal| {
                tx.send(signal.clone()).unwrap();
            },
        )
        .unwrap();
        worker.wake();
        let signal = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(worker.stop_and_wait(Duration::from_secs(2)));
        let database = Database::open_in_memory().unwrap();
        let root = std::env::temp_dir().join(format!("tg-stopped-{}", std::process::id()));
        let files = FileStore::new(&root).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let transport = ReqwestTelegramTransport::with_test_endpoint(format!(
            "http://{}",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let summary = block_on(run_active_archived_sends_cooperative(
            &database,
            &files,
            &transport,
            &ForbiddenSecrets,
            &configured(),
            Some(&signal),
            |_, _| panic!("stopped sender must not report progress"),
        ))
        .unwrap();
        assert_eq!(summary.claimed, 0);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_sender_rejects_rotation_during_secret_retrieval() {
        struct RotatingSecrets<'a> {
            database: &'a Database,
            previous: i64,
        }
        impl xarchive_telegram::SecretStore for RotatingSecrets<'_> {
            fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
                let replacement = self
                    .database
                    .prepare_telegram_credential_generation(
                        "telegram-bot:42",
                        "secret/replacement",
                        "t1",
                    )
                    .unwrap();
                self.database
                    .activate_telegram_credential_generation(replacement, Some(self.previous))
                    .unwrap();
                Ok(Some("123456:old-secret".into()))
            }
            fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("read only");
            }
            fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("read only");
            }
        }
        let database = Database::open_in_memory().unwrap();
        let previous = database
            .prepare_telegram_credential_generation("telegram-bot:42", "secret/old", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(previous, None)
            .unwrap();
        let root = std::env::temp_dir().join(format!("tg-rotation-gate-{}", std::process::id()));
        let files = FileStore::new(&root).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let transport = ReqwestTelegramTransport::with_test_endpoint(format!(
            "http://{}",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let error = block_on(run_active_archived_sends(
            &database,
            &files,
            &transport,
            &RotatingSecrets {
                database: &database,
                previous,
            },
            &configured(),
            |_, _| {},
        ))
        .expect_err("rotation must reject old secret");
        assert_eq!(
            error,
            "credential generation changed during secret retrieval"
        );
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_sender_disabled_and_missing_credential_never_access_secrets_or_network() {
        struct ForbiddenSecrets;
        impl xarchive_telegram::SecretStore for ForbiddenSecrets {
            fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
                panic!("secret access forbidden before credential prerequisites");
            }
            fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("sender cannot write secrets");
            }
            fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("sender cannot delete secrets");
            }
        }
        let root = std::env::temp_dir().join(format!("tg-active-gate-{}", std::process::id()));
        let files = FileStore::new(&root).expect("files");
        let database = Database::open_in_memory().expect("database");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
        listener.set_nonblocking(true).expect("nonblocking");
        let transport = ReqwestTelegramTransport::with_test_endpoint(format!(
            "http://{}",
            listener.local_addr().unwrap()
        ))
        .expect("transport");
        let mut config = configured();
        config.enabled = false;
        let result = block_on(run_active_archived_sends(
            &database,
            &files,
            &transport,
            &ForbiddenSecrets,
            &config,
            |_, _| {},
        ))
        .expect("disabled");
        assert_eq!(result.claimed, 0);
        config.enabled = true;
        config.migration_pending = true;
        let paused = block_on(run_active_archived_sends(
            &database,
            &files,
            &transport,
            &ForbiddenSecrets,
            &config,
            |_, _| {},
        ))
        .expect("paused");
        assert_eq!(paused.claimed, 0);
        config.migration_pending = false;
        let error = block_on(run_active_archived_sends(
            &database,
            &files,
            &transport,
            &ForbiddenSecrets,
            &config,
            |_, _| {},
        ))
        .expect_err("missing credential");
        assert_eq!(error, "active credential missing");
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_due_message_is_sent_and_confirmed() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let intent = test_intent(&config, tweet_id, "archive", Some("caption"), Vec::new());
        let ids = queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");
        let (address, server) = confirmed_server(r#"{"ok":true,"result":{"message_id":77}}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let payload = SendPayload::Message {
            chat_id: config.chat_id.clone(),
            message_thread_id: config.message_thread_id,
            text: "caption".to_owned(),
        };
        let summary = block_on(run_due_sends(
            &database,
            &transport,
            &BotToken::new("1234:TEST").expect("token"),
            &config,
            "bot-a",
            "2026-10-01T00:00:01Z",
            |entry| (entry.id == ids[0]).then(|| payload.clone()),
            |_, _| {},
        ))
        .expect("run");
        assert_eq!(summary.claimed, 1);
        assert_eq!(summary.sent, 1);

        let request = String::from_utf8_lossy(&server.join().expect("server")).to_ascii_lowercase();
        assert!(request.contains("sendmessage"), "{request}");
        assert!(request.contains("caption"), "{request}");

        let rows = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows");
        assert_eq!(rows[0].state, OutboxState::Sent);
        assert_eq!(rows[0].telegram_message_id.as_deref(), Some("77"));
    }

    #[test]
    fn a_claimed_row_reports_terminal_progress_stages_only() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "progress");
        let config = configured();
        let intent = test_intent(&config, tweet_id, "archive", Some("caption"), Vec::new());
        queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");
        let (address, server) = confirmed_server(r#"{"ok":true,"result":{"message_id":78}}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let payload = SendPayload::Message {
            chat_id: config.chat_id.clone(),
            message_thread_id: config.message_thread_id,
            text: "caption".to_owned(),
        };
        let reported = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = std::rc::Rc::clone(&reported);
        block_on(run_due_sends(
            &database,
            &transport,
            &BotToken::new("1234:TEST").expect("token"),
            &config,
            "bot-a",
            "2026-10-01T00:00:01Z",
            |_| Some(payload.clone()),
            move |key, stage| sink.borrow_mut().push((key.to_owned(), stage)),
        ))
        .expect("run");
        let _ = server.join().expect("server");
        let stages = reported.borrow();
        assert!(!stages.is_empty(), "no progress stage was reported");
        // Every stage belongs to the claimed archive unit and ends confirmed.
        assert!(stages.iter().all(|(key, _)| !key.is_empty()));
        assert!(
            matches!(stages.last().expect("stage").1, UploadStage::Confirmed),
            "{stages:?}"
        );
        // A text unit has no byte stream, so it must not invent upload progress.
        assert!(
            !stages
                .iter()
                .any(|(_, stage)| matches!(stage, UploadStage::Uploading { .. })),
            "{stages:?}"
        );
        // A text unit reports queued, awaiting and confirmed in order.
        let reported: Vec<&str> = stages
            .iter()
            .map(|(_, stage)| match stage {
                UploadStage::Queued => "queued",
                UploadStage::AwaitingResult => "awaiting",
                UploadStage::Confirmed => "confirmed",
                UploadStage::CheckingFile => "checking",
                UploadStage::Uploading { .. } => "uploading",
            })
            .collect();
        assert_eq!(reported, vec!["queued", "awaiting", "confirmed"]);
    }

    #[test]
    fn text_payload_sends_to_its_persisted_topic() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "topic-text");
        let config = configured();
        let intent = topic_intent(&config, tweet_id, Vec::new());
        let ids = queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z")
            .expect("queue topic message");
        let (address, server) = confirmed_server(r#"{"ok":true,"result":{"message_id":78}}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let payload = SendPayload::Message {
            chat_id: config.chat_id.clone(),
            message_thread_id: Some(42),
            text: "topic message".to_owned(),
        };

        let summary = block_on(run_due_sends(
            &database,
            &transport,
            &BotToken::new("1234:TEST").expect("token"),
            &config,
            "bot-a",
            "2026-10-01T00:00:01Z",
            |entry| (entry.id == ids[0]).then(|| payload.clone()),
            |_, _| {},
        ))
        .expect("send");
        assert_eq!(summary.sent, 1);

        let request = String::from_utf8_lossy(&server.join().expect("server")).to_ascii_lowercase();
        assert!(request.contains("message_thread_id"), "{request}");
        assert!(request.contains("42"), "{request}");
    }

    #[test]
    fn an_unknown_entry_is_never_claimed_automatically() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let intent = test_intent(&config, tweet_id, "archive", Some("caption"), Vec::new());
        let ids = queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");

        // A worker claimed the entry, started the request, then died.
        let metadata_key = format!("tweet-{tweet_id}:metadata");
        let claimed = database
            .claim_outbox(
                "bot-a",
                &metadata_key,
                "claim-1",
                "2026-10-01T00:00:01Z",
                "2026-10-01T00:05:01Z",
            )
            .expect("claim");
        assert!(
            claimed.is_some(),
            "the deliberate re-send path can claim it"
        );
        database
            .mark_request_started("claim-1", "2026-10-01T00:00:02Z")
            .expect("started");

        let recovered =
            recover_expired_claims(&database, "2026-10-01T00:10:00Z").expect("recovered");
        assert_eq!(recovered, 1);
        let rows = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows");
        assert_eq!(rows[0].state, OutboxState::Unknown);

        // The automatic run must not touch it, even when it is due.
        let (address, _server) = confirmed_server(r#"{"ok":true,"result":{"message_id":5}}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let summary = block_on(run_due_sends(
            &database,
            &transport,
            &BotToken::new("1234:TEST").expect("token"),
            &config,
            "bot-a",
            "2026-10-01T00:10:00Z",
            |_| None,
            |_, _| {},
        ))
        .expect("run");
        assert_eq!(summary.claimed, 0, "UNKNOWN waits for a human decision");
        let rows = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows");
        assert_eq!(rows[0].state, OutboxState::Unknown);
        assert_eq!(rows[0].id, ids[0]);
    }

    #[test]
    fn a_claim_that_expired_before_the_request_retries_instead() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "crash");
        let config = configured();
        let intent = test_intent(&config, tweet_id, "archive", Some("caption"), Vec::new());
        queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");
        let metadata_key = format!("tweet-{tweet_id}:metadata");
        database
            .claim_outbox(
                "bot-a",
                &metadata_key,
                "claim-1",
                "2026-10-01T00:00:01Z",
                "2026-10-01T00:05:01Z",
            )
            .expect("claim");
        let recovered =
            recover_expired_claims(&database, "2026-10-01T00:10:00Z").expect("recovered");
        assert_eq!(recovered, 1);
        let rows = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows");
        assert_eq!(
            rows[0].state,
            OutboxState::RetryWait,
            "nothing was sent, so the entry stays retryable"
        );
    }
    #[test]
    fn confirmed_cache_remains_scoped_to_original_bot_after_rotation() {
        let database = Database::open_in_memory().unwrap();
        let original = "telegram-bot:42";
        let replacement = "telegram-bot:43";
        let old = database
            .prepare_telegram_credential_generation(original, "secret/cache-old", "t0")
            .unwrap();
        database
            .activate_telegram_credential_generation(old, None)
            .unwrap();
        let new = database
            .prepare_telegram_credential_generation(replacement, "secret/cache-new", "t1")
            .unwrap();
        database
            .activate_telegram_credential_generation(new, Some(old))
            .unwrap();
        let item = photo_plan("rotation-cache", "sha-rotation-cache");
        let payload = SendPayload::Media {
            chat_id: "-100".into(),
            message_thread_id: None,
            items: vec![item.clone()],
        };
        let success = SendAttemptSuccess {
            telegram_message_id: "77".into(),
            results_json: Some(r#"{"file_ids":["original-file"]}"#.into()),
        };
        store_confirmed_files(&database, original, &payload, &success, "t2").unwrap();
        assert_eq!(
            cached_file_id(&database, original, &item)
                .unwrap()
                .map(|value| value.file_id),
            Some("original-file".into())
        );
        assert_eq!(cached_file_id(&database, replacement, &item).unwrap(), None);
    }

    #[test]
    fn a_cached_photo_is_sent_by_identifier_without_a_body() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let item = photo_plan("cached", "sha-cached");
        let intent = test_intent(&config, tweet_id, "archive", None, vec![item.clone()]);
        let ids = queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");
        database
            .store_file_id(
                &cache_key("bot-a", &item).expect("cache key"),
                &CachedFileId {
                    file_id: "cached-photo-id".to_owned(),
                    file_unique_id: "unique-1".to_owned(),
                    file_size: 9,
                    confirmed_at: "2026-10-01T00:00:00Z".to_owned(),
                },
            )
            .expect("cached");

        let (address, server) = confirmed_server(
            r#"{"ok":true,"result":{"message_id":31,"photo":{"file_id":"cached-photo-id"}}}"#,
        );
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let payload = SendPayload::Media {
            chat_id: config.chat_id.clone(),
            message_thread_id: config.message_thread_id,
            items: vec![item],
        };
        let summary = block_on(run_due_sends(
            &database,
            &transport,
            &BotToken::new("1234:TEST").expect("token"),
            &config,
            "bot-a",
            "2026-10-01T00:00:01Z",
            |entry| (entry.id == ids[0]).then(|| payload.clone()),
            |_, _| {},
        ))
        .expect("run");
        assert_eq!(summary.sent, 1);

        let request = String::from_utf8_lossy(&server.join().expect("server")).to_ascii_lowercase();
        assert!(request.contains("sendphoto"), "{request}");
        assert!(
            request.contains("cached-photo-id"),
            "the cached identifier was sent: {request}"
        );
        assert!(
            !request.contains("multipart/form-data"),
            "a cached identifier needs no upload body: {request}"
        );
        let rows = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows");
        assert_eq!(rows[0].state, OutboxState::Sent);
        assert_eq!(rows[0].telegram_message_id.as_deref(), Some("31"));
    }

    #[test]
    fn a_media_plan_without_a_resolvable_payload_needs_a_new_plan() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let intent = test_intent(
            &config,
            tweet_id,
            "archive",
            None,
            vec![photo_plan("gone", "sha-gone")],
        );
        let ids = queue_archive_sends(&database, &intent, "2026-10-01T00:00:00Z").expect("queue");
        let (address, _server) = confirmed_server(r#"{"ok":true,"result":{"message_id":9}}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
        let summary = block_on(run_due_sends(
            &database,
            &transport,
            &BotToken::new("1234:TEST").expect("token"),
            &config,
            "bot-a",
            "2026-10-01T00:00:01Z",
            |_| None,
            |_, _| {},
        ))
        .expect("run");
        assert_eq!(summary.failed, 1, "a missing payload is a plan problem");
        let rows = database
            .list_outbox_for_tweet("bot-a", tweet_id)
            .expect("rows");
        assert_eq!(rows[0].id, ids[0]);
        assert_eq!(rows[0].state, OutboxState::FailedPermanent);
        assert!(
            rows[0]
                .last_error_message
                .as_deref()
                .is_some_and(|message| message.contains("no longer available")),
            "the reason is recorded: {:?}",
            rows[0].last_error_message
        );
    }
}
