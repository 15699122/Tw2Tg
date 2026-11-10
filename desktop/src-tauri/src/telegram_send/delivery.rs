//! Delivery of one claimed payload, and the confirmed-file cache around it.
//!
//! Split out of `telegram_send.rs` without changing behaviour or the public
//! path: the parent re-exports these items, so `crate::telegram_send::<name>`
//! keeps resolving.
//!
//! Ordering and ownership rules that must not move:
//!
//! * progress stages are published per payload and confirm only after the
//!   transport accepted the request;
//! * a text send and a media send report the same stage contract;
//! * the file cache is populated only from a confirmed success, and evicted
//!   only when Telegram explicitly answers that an identifier is dead;
//! * a payload that can no longer be resolved is a plan needing correction, not
//!   a silent skip and not a blind retry.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use xarchive_storage::Database;

use xarchive_telegram::{
    AlbumItemUpload, BotToken, CachedFileId, CancellationToken, FileCacheKey, FileIdCacheStore,
    MediaKind, PlannedMediaItem, RequestProgress, ReqwestTelegramTransport, SendAttemptError,
    SendAttemptSuccess, SendMessageRequest, SendPayload, SendPhotoRequest, SendVideoRequest,
    TELEGRAM_FILE_CACHE_VERSION, TelegramError, TelegramRequest, TelegramTransport, UploadRequest,
    UploadStage, UploadTimeouts, is_invalid_file_id_error,
};

use crate::config::TelegramConfig;

/// The upload timeouts this configuration asks for.
pub(in crate::telegram_send) fn upload_timeouts(config: &TelegramConfig) -> UploadTimeouts {
    let processing = Duration::from_secs(config.upload_processing_timeout_seconds.max(1));
    UploadTimeouts {
        connect: Duration::from_secs(config.connect_timeout_seconds.max(1)),
        body_stall: processing,
        server_processing: processing,
        total: None,
    }
}

/// Perform one claimed entry's request.
pub(in crate::telegram_send) async fn execute_payload(
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
pub(in crate::telegram_send) fn cache_key(
    bot_identity: &str,
    item: &PlannedMediaItem,
) -> Option<FileCacheKey> {
    item.content_sha256.as_ref().map(|sha256| FileCacheKey {
        bot_identity: bot_identity.to_owned(),
        content_sha256: sha256.clone(),
        media_kind: item.media_kind,
        representation_version: TELEGRAM_FILE_CACHE_VERSION,
    })
}

/// The cached identifier of a media item, if this bot already has one.
pub(in crate::telegram_send) fn cached_file_id(
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
pub(in crate::telegram_send) fn store_confirmed_files(
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
pub(in crate::telegram_send) fn evict_rejected_files(
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
