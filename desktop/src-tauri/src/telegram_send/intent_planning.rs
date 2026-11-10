//! Archive send intent: immutable planning facts and their idempotent queueing.
//!
//! Split out of `telegram_send.rs` without changing behaviour or the public
//! path: the parent re-exports these items, so `crate::telegram_send::<name>`
//! keeps resolving. The invariants that keep this module honest:
//!
//! * a plan is derived from captured facts, never from current settings, so a
//!   configuration edit cannot redirect already-queued work;
//! * replaying the same intent is idempotent and cannot duplicate rows;
//! * a damaged journal is isolated per record instead of blocking unrelated
//!   archives.

use xarchive_storage::Database;

use xarchive_telegram::{
    MediaKind, PlannedMediaItem, PlannedSend, plan_media_sends, plan_text_send,
};

use crate::config::TelegramConfig;

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

/// Build the ordered outbox entries of one immutable intent.
///
/// Visible only inside the `telegram_send` subtree (`pub(in ...)`, not `pub`):
/// after the P5.2 extraction the parent module's regression tests still drive
/// this planner directly, and a sibling private item is not reachable from a
/// child module. No crate-level or external exposure is added.
pub(in crate::telegram_send) fn archive_plan_entries(
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
