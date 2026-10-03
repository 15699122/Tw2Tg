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
    plan_media_sends, plan_text_send, run_claimed_attempt,
};

use crate::clock;
use crate::config::TelegramConfig;

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
    config.enabled && config.auto_send_on_archive && !config.chat_id.trim().is_empty()
}

/// Queue the planned sends of one finished archive and return their row ids.
///
/// Metadata text is queued first, then media units in order, so the target
/// receives them in the planned sequence. Queueing is idempotent per
/// `idempotency_key`, so a repeated call cannot duplicate work.
///
/// The arguments are the finished archive's own facts; grouping them into a
/// struct would only rename them.
#[allow(clippy::too_many_arguments)]
pub fn queue_archive_sends(
    database: &Database,
    config: &TelegramConfig,
    bot_identity: &str,
    tweet_id: i64,
    archive_directory: &str,
    metadata_text: Option<&str>,
    media: Vec<PlannedMediaItem>,
    plan_version: i64,
    now: &str,
) -> Result<Vec<i64>, String> {
    let mut plans: Vec<PlannedSend> = Vec::new();
    if let Some(text) = metadata_text.filter(|text| !text.trim().is_empty()) {
        plans.push(plan_text_send(
            config.chat_id.clone(),
            config.message_thread_id,
            text,
            format!("tweet-{tweet_id}:metadata"),
        ));
    }
    plans.extend(plan_media_sends(
        config.chat_id.clone(),
        config.message_thread_id,
        media,
        &format!("tweet-{tweet_id}:media"),
    ));
    let mut ids = Vec::with_capacity(plans.len());
    for (plan_order, plan) in plans.iter().enumerate() {
        let entry = plan
            .to_outbox_entry(
                bot_identity,
                archive_directory,
                Some(tweet_id),
                config.revision,
                plan_version,
                plan_order as i64,
                now,
            )
            .map_err(|error| error.to_string())?;
        ids.push(
            database
                .enqueue_outbox(entry)
                .map_err(|error| error.to_string())?,
        );
    }
    Ok(ids)
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
    mut on_progress: P,
) -> Result<SendRunSummary, String>
where
    R: Fn(&OutboxEntry) -> Option<SendPayload>,
    P: FnMut(&str, UploadStage),
{
    let mut summary = SendRunSummary::default();
    while (summary.claimed as usize) < MAX_SENDS_PER_RUN {
        let claim_token = claim_token(bot_identity, summary.claimed);
        let lease_until = clock::timestamp_after(now, CLAIM_LEASE);
        let Some(entry) = database
            .claim_due_outbox(bot_identity, &claim_token, now, &lease_until)
            .map_err(|error| error.to_string())?
        else {
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
        let outcome = run_claimed_attempt(
            database,
            &claim_token,
            attempt,
            now,
            |delay| clock::timestamp_after(now, delay),
            execute_payload(
                database,
                transport,
                token,
                config,
                bot_identity,
                payload.clone(),
                collected,
            ),
        )
        .await;
        for stage in stages.lock().expect("stages").iter().cloned() {
            on_progress(&key, stage);
        }

        match outcome {
            Ok(success) => {
                summary.sent += 1;
                // Confirmed results are the only thing that may populate the cache.
                if let Some(payload) = payload {
                    store_confirmed_files(database, bot_identity, &payload, &success, now)?;
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
        SendPayload::Message { chat_id, text, .. } => {
            let entry_chat_id = chat_id.clone();
            let request = TelegramRequest::Message(SendMessageRequest {
                chat_id,
                text,
                disable_web_page_preview: false,
            });
            // A text send is small: it runs on the blocking pool so no runtime
            // worker thread is parked.
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
            confirm_cached_send(response, &entry_chat_id)
        }
        SendPayload::Media { chat_id, items, .. } if items.len() == 1 => {
            send_single_media(
                database,
                transport,
                token,
                &chat_id,
                bot_identity,
                &items[0],
                &timeouts,
                &cancellation,
                stages,
            )
            .await
        }
        SendPayload::Media { chat_id, items, .. } => {
            let mut uploads = Vec::with_capacity(items.len());
            for item in &items {
                uploads.push(album_upload(database, bot_identity, &chat_id, item)?);
            }
            xarchive_telegram::send_media_group_attempt(
                transport,
                token,
                &chat_id,
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
    })? && let Some(request) = cached_media_request(chat_id, item, &cached.file_id)
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
    item: &PlannedMediaItem,
) -> Result<AlbumItemUpload, SendAttemptError> {
    if let Some(cached) = cached_file_id(database, bot_identity, item).map_err(|_| {
        SendAttemptError::new(
            TelegramError::Transport("file_id cache read failed".to_owned()),
            RequestProgress::NotSent,
        )
    })? {
        return Ok(AlbumItemUpload::CachedFileId(cached.file_id));
    }
    Ok(AlbumItemUpload::File(Box::new(UploadRequest {
        chat_id: chat_id.to_owned(),
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
    item: &PlannedMediaItem,
    file_id: &str,
) -> Option<TelegramRequest> {
    match item.media_kind {
        MediaKind::Photo => Some(TelegramRequest::Photo(SendPhotoRequest {
            chat_id: chat_id.to_owned(),
            photo: file_id.to_owned(),
            caption: item.caption.clone(),
        })),
        MediaKind::Video => Some(TelegramRequest::Video(SendVideoRequest {
            chat_id: chat_id.to_owned(),
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
    fn queueing_an_archive_is_idempotent_and_ordered() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let media = vec![
            photo_plan("first", "sha-first"),
            photo_plan("second", "sha-second"),
        ];
        let ids = queue_archive_sends(
            &database,
            &config,
            "bot-a",
            tweet_id,
            "archive",
            Some("caption"),
            media.clone(),
            7,
            "2026-10-01T00:00:00Z",
        )
        .expect("queue");
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
        let repeated = queue_archive_sends(
            &database,
            &config,
            "bot-a",
            tweet_id,
            "archive",
            Some("caption"),
            media,
            7,
            "2026-10-01T00:05:00Z",
        )
        .expect("requeue");
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
        let ids = queue_archive_sends(
            &database,
            &configured(),
            "bot-a",
            tweet_id,
            "archive",
            Some("   "),
            Vec::new(),
            1,
            "2026-10-01T00:00:00Z",
        )
        .expect("queue");
        assert!(ids.is_empty());
        assert!(
            database
                .list_outbox_for_tweet("bot-a", tweet_id)
                .expect("rows")
                .is_empty()
        );
    }
    #[test]
    fn a_due_message_is_sent_and_confirmed() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let ids = queue_archive_sends(
            &database,
            &config,
            "bot-a",
            tweet_id,
            "archive",
            Some("caption"),
            Vec::new(),
            1,
            "2026-10-01T00:00:00Z",
        )
        .expect("queue");
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
    fn an_unknown_entry_is_never_claimed_automatically() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let ids = queue_archive_sends(
            &database,
            &config,
            "bot-a",
            tweet_id,
            "archive",
            Some("caption"),
            Vec::new(),
            1,
            "2026-10-01T00:00:00Z",
        )
        .expect("queue");

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
        queue_archive_sends(
            &database,
            &configured(),
            "bot-a",
            tweet_id,
            "archive",
            Some("caption"),
            Vec::new(),
            1,
            "2026-10-01T00:00:00Z",
        )
        .expect("queue");
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
    fn a_cached_photo_is_sent_by_identifier_without_a_body() {
        let database = Database::open_in_memory().expect("database");
        let tweet_id = tweet_row(&database, "queue");
        let config = configured();
        let item = photo_plan("cached", "sha-cached");
        let ids = queue_archive_sends(
            &database,
            &config,
            "bot-a",
            tweet_id,
            "archive",
            None,
            vec![item.clone()],
            1,
            "2026-10-01T00:00:00Z",
        )
        .expect("queue");
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
        let ids = queue_archive_sends(
            &database,
            &config,
            "bot-a",
            tweet_id,
            "archive",
            None,
            vec![photo_plan("gone", "sha-gone")],
            1,
            "2026-10-01T00:00:00Z",
        )
        .expect("queue");
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
