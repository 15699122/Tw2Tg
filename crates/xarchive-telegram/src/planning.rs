//! Telegram send planning: turn archived content into the deterministic units
//! of Telegram work the outbox will transmit (plan TG-03/TG-04).
//!
//! This module is pure. It reads no clock, performs no I/O and holds no outbox,
//! lease or transport state; it only decides how content is grouped and how the
//! persisted payload snapshot is shaped.
//!
//! The crate root re-exports every public item, so call sites are unchanged.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

use crate::delivery::SendStateError;
use crate::formatting::MediaKind;
use crate::outbox::NewOutboxEntry;
use crate::{TELEGRAM_ALBUM_MIN_ITEMS, TELEGRAM_MEDIA_GROUP_LIMIT, sha256_hex};

/// One media item of a planned send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedMediaItem {
    pub media_kind: MediaKind,
    /// Archive-root-relative path, not an absolute machine-local location.
    pub file_path: PathBuf,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub caption: Option<String>,
    /// SHA-256 already computed while archiving. It is reused as the `file_id`
    /// cache key (plan TG-05) instead of hashing the file again.
    pub content_sha256: Option<String>,
    pub size_bytes: Option<u64>,
}

/// What one outbox entry transmits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SendPayload {
    Message {
        chat_id: String,
        message_thread_id: Option<i64>,
        text: String,
    },
    Media {
        chat_id: String,
        message_thread_id: Option<i64>,
        items: Vec<PlannedMediaItem>,
    },
}

/// A deterministic unit of Telegram work: one text message, one album or one
/// standalone media item (plan TG-03/TG-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedSend {
    /// Stable across restarts, so the same planned content is never queued twice.
    pub idempotency_key: String,
    /// `message`, `media_album` or `media_single`.
    pub message_kind: String,
    pub payload: SendPayload,
}

impl PlannedSend {
    /// Deterministic content fingerprint stored as `request_fingerprint`: a
    /// changed plan can then never reuse a result recorded for other content.
    pub fn fingerprint(&self) -> String {
        // File paths are runtime locations, not part of the logical payload.
        // Hash the portable media facts so moving an archive does not create a
        // different send intent.
        let canonical = match &self.payload {
            SendPayload::Message {
                chat_id,
                message_thread_id,
                text,
            } => serde_json::json!({
                "kind": "message",
                "chat_id": chat_id,
                "message_thread_id": message_thread_id,
                "text": text,
            })
            .to_string(),
            SendPayload::Media {
                chat_id,
                message_thread_id,
                items,
            } => {
                let items: Vec<_> = items
                    .iter()
                    .map(|item| {
                        serde_json::json!({
                            "media_kind": item.media_kind,
                            "file_name": item.file_name,
                            "mime_type": item.mime_type,
                            "caption": item.caption,
                            "content_sha256": item.content_sha256,
                            "size_bytes": item.size_bytes,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "kind": "media",
                    "chat_id": chat_id,
                    "message_thread_id": message_thread_id,
                    "items": items,
                })
                .to_string()
            }
        };
        sha256_hex(&[self.message_kind.as_bytes(), b"\x1f", canonical.as_bytes()])
    }

    /// The outbox row this plan becomes. A versioned payload snapshot is stored
    /// with archive-root-relative media references so restart recovery never
    /// depends on the current settings.
    ///
    /// `archive_directory` is the Tweet's committed archive directory. Every
    /// media reference must be an archive-relative path that stays inside it;
    /// otherwise the plan is rejected with `InvalidPayloadPath` rather than
    /// persisting an unsafe or machine-local snapshot.
    ///
    /// A text message references no file, so it is unaffected.
    #[allow(clippy::too_many_arguments)]
    pub fn to_outbox_entry(
        &self,
        bot_identity: &str,
        archive_directory: &str,
        tweet_id: Option<i64>,
        config_version: i64,
        plan_version: i64,
        plan_order: i64,
        created_at: &str,
    ) -> Result<NewOutboxEntry, SendStateError> {
        if let SendPayload::Media { items, .. } = &self.payload {
            let directory = Path::new(archive_directory);
            if !is_safe_archive_relative_path(directory) {
                return Err(SendStateError::InvalidPayloadPath);
            }
            for item in items {
                // `starts_with` is component-wise, so `archive-x` can never
                // masquerade as a child of `archive`, and the equality guard
                // requires a file *inside* the directory, not the directory.
                if !is_safe_archive_relative_path(&item.file_path)
                    || !item.file_path.starts_with(directory)
                    || item.file_path.as_path() == directory
                {
                    return Err(SendStateError::InvalidPayloadPath);
                }
            }
        }
        let payload_json = serde_json::to_string(&self.payload)
            .map_err(|error| SendStateError::Store(error.to_string()))?;
        let target_scope = match &self.payload {
            SendPayload::Message {
                chat_id,
                message_thread_id,
                ..
            }
            | SendPayload::Media {
                chat_id,
                message_thread_id,
                ..
            } => format!(
                "{chat_id}:{}",
                message_thread_id.map_or_else(|| "none".to_owned(), |id| id.to_string())
            ),
        };
        let (chat_id, message_thread_id, media_reference, content_sha256) = match &self.payload {
            SendPayload::Message {
                chat_id,
                message_thread_id,
                ..
            } => (chat_id.clone(), *message_thread_id, None, None),
            SendPayload::Media {
                chat_id,
                message_thread_id,
                items,
            } => (
                chat_id.clone(),
                *message_thread_id,
                Some(
                    items
                        .iter()
                        .map(|item| item.file_path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(";"),
                ),
                items.first().and_then(|item| item.content_sha256.clone()),
            ),
        };
        Ok(NewOutboxEntry {
            bot_identity: bot_identity.to_owned(),
            target_scope,
            chat_id,
            message_thread_id,
            idempotency_key: self.idempotency_key.clone(),
            request_fingerprint: self.fingerprint(),
            message_kind: self.message_kind.clone(),
            config_version,
            plan_version,
            plan_order,
            tweet_id,
            media_reference,
            content_sha256,
            payload_schema_version: 1,
            payload_json,
            created_at: created_at.to_owned(),
        })
    }
}

fn is_safe_archive_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// One text message as a planned send.
pub fn plan_text_send(
    chat_id: impl Into<String>,
    message_thread_id: Option<i64>,
    text: impl Into<String>,
    idempotency_key: impl Into<String>,
) -> PlannedSend {
    PlannedSend {
        idempotency_key: idempotency_key.into(),
        message_kind: "message".to_owned(),
        payload: SendPayload::Message {
            chat_id: chat_id.into(),
            message_thread_id,
            text: text.into(),
        },
    }
}

/// Turn an ordered media list into sendable units: albums of 2–10 items, with a
/// trailing group of one promoted to a standalone send (plan TG-03, mirroring
/// [`media_send_plan`] for the richer media items).
pub fn plan_media_sends(
    chat_id: impl Into<String>,
    message_thread_id: Option<i64>,
    items: Vec<PlannedMediaItem>,
    idempotency_prefix: &str,
) -> Vec<PlannedSend> {
    let chat_id = chat_id.into();
    let total = items.len();
    let mut units = Vec::new();
    let mut index = 0;
    while index < total {
        // Documents may only form document albums; photos and videos may mix.
        // Split contiguous compatible runs rather than reordering the archive.
        let document = items[index].media_kind == MediaKind::Document;
        let remaining = items[index..]
            .iter()
            .take_while(|item| (item.media_kind == MediaKind::Document) == document)
            .count();
        let take = if remaining >= TELEGRAM_ALBUM_MIN_ITEMS {
            remaining.min(TELEGRAM_MEDIA_GROUP_LIMIT)
        } else {
            1
        };
        units.push(PlannedSend {
            idempotency_key: format!("{idempotency_prefix}:{index:02}"),
            message_kind: if take == 1 {
                "media_single".to_owned()
            } else {
                "media_album".to_owned()
            },
            payload: SendPayload::Media {
                chat_id: chat_id.clone(),
                message_thread_id,
                items: items[index..index + take].to_vec(),
            },
        });
        index += take;
    }
    units
}

// ---------------------------------------------------------------------------
