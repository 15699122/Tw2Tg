//! Localised async streaming upload transport (plan TG-02).
//!
//! This module owns the upload half of the Bot API transport: layered upload
//! timeouts, ordered phase reporting, the pull-based streamed body, the album
//! watchdog and the per-media send call. It does not own the blocking control
//! client, the outbox state machine or any retry decision.
//!
//! The crate root re-exports every public item, so call sites are unchanged.

use futures_core::Stream;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use crate::contract::{BotToken, TelegramError, TelegramResponse};
use crate::formatting::MediaKind;
use crate::retry_policy::RequestProgress;
use crate::transport::ReqwestTelegramTransport;
use crate::transport::{TELEGRAM_MAX_RESPONSE_BYTES, validate_chat_id, validate_thread_id};
use crate::{
    CancellationToken, SendAttemptError, SendAttemptSuccess, TELEGRAM_ALBUM_MIN_ITEMS,
    TELEGRAM_MEDIA_GROUP_LIMIT,
};

// ---------------------------------------------------------------------------
// TG-02 — localised async streaming upload transport
// ---------------------------------------------------------------------------

/// Bytes read from disk per body-stream step.
///
/// The upload body is pull-based, so at most one chunk is in flight: memory
/// does not grow with the file size and a whole video is never read at once.
pub const TELEGRAM_UPLOAD_CHUNK_BYTES: usize = 64 * 1024;

/// Layered timeouts for a streaming upload (plan TG-02).
///
/// Each layer covers a different failure mode, so a slow-but-progressing
/// transfer is not killed by the control-request timeout while a dead server
/// is still detected:
///
/// - `connect`: TCP/TLS/DNS connection setup.
/// - `body_stall`: the longest allowed gap with no bytes produced while the
///   file is streaming. Once the body has finished, this layer no longer
///   applies — a slow *server* is covered by `server_processing`.
/// - `server_processing`: the wait from the last body byte being handed over
///   until the response arrives.
/// - `total`: optional overall deadline covering the whole send.
///
/// The transport's control-request timeout (30 s) never governs large sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadTimeouts {
    pub connect: Duration,
    pub body_stall: Duration,
    pub server_processing: Duration,
    pub total: Option<Duration>,
}

impl UploadTimeouts {
    /// Each fixed layer must be positive, so a misconfiguration fails loudly
    /// at validation time instead of firing instantly at runtime.
    pub(crate) fn validate(&self) -> Result<(), TelegramError> {
        if self.connect.is_zero() {
            return Err(TelegramError::InvalidUploadRequest(
                "connect timeout must be greater than zero".to_owned(),
            ));
        }
        if self.body_stall.is_zero() {
            return Err(TelegramError::InvalidUploadRequest(
                "body stall timeout must be greater than zero".to_owned(),
            ));
        }
        if self.server_processing.is_zero() {
            return Err(TelegramError::InvalidUploadRequest(
                "server processing timeout must be greater than zero".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Default for UploadTimeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            body_stall: Duration::from_secs(60),
            server_processing: Duration::from_secs(300),
            total: None,
        }
    }
}

/// One step of an upload, in emission order.
///
/// `Uploading { sent_bytes == total_bytes }` means only that the request body
/// reached the Bot API connection — it is never a "message sent" claim. Only
/// `Confirmed`, which follows a parsed `ok: true` result, means Telegram
/// accepted the message (plan TG-02 progress semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadStage {
    Queued,
    CheckingFile,
    Uploading { sent_bytes: u64, total_bytes: u64 },
    AwaitingResult,
    Confirmed,
}

/// One media file to stream to the Bot API as `multipart/form-data`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadRequest {
    pub chat_id: String,
    pub message_thread_id: Option<i64>,
    /// Selects the method (`sendPhoto`/`sendVideo`/`sendDocument`) and the
    /// multipart field name (`photo`/`video`/`document`).
    pub media_kind: MediaKind,
    pub file_path: PathBuf,
    /// Stable display name from [`stable_upload_file_name`]; never an
    /// absolute local path.
    pub file_name: String,
    pub mime_type: Option<String>,
    pub caption: Option<String>,
    /// Re-verification hint: when present, the file size must still match at
    /// send time. Every call re-opens the file from disk, so a rebuilt
    /// request re-reads and re-verifies it (plan TG-02).
    pub expected_size: Option<u64>,
}

pub(crate) fn validate_upload_request(request: &UploadRequest) -> Result<(), TelegramError> {
    validate_chat_id(&request.chat_id)?;
    validate_thread_id(request.message_thread_id)?;
    if request.file_name.trim().is_empty() {
        return Err(TelegramError::InvalidUploadRequest(
            "file name must not be empty".to_owned(),
        ));
    }
    // The name lands verbatim in a `Content-Disposition` header: quoting,
    // CR/LF and path separators would corrupt or redirect the part.
    if request
        .file_name
        .chars()
        .any(|character| matches!(character, '"' | '\r' | '\n' | '/' | '\\'))
    {
        return Err(TelegramError::InvalidUploadRequest(
            "file name contains unsupported characters".to_owned(),
        ));
    }
    if let Some(mime_type) = request.mime_type.as_deref() {
        let trimmed = mime_type.trim();
        if trimmed.is_empty() || !trimmed.contains('/') || trimmed.contains(['\r', '\n']) {
            return Err(TelegramError::InvalidUploadRequest(
                "content type must be a single-line MIME type".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Bot API method and multipart field name for a media kind.
pub(crate) fn upload_method(media_kind: MediaKind) -> (&'static str, &'static str) {
    match media_kind {
        MediaKind::Photo => ("sendPhoto", "photo"),
        MediaKind::Video => ("sendVideo", "video"),
        MediaKind::Document => ("sendDocument", "document"),
    }
}

/// Body phase used by the watchdog to pick the right layered deadline.
const UPLOAD_PHASE_PRE_BODY: u8 = 0;
const UPLOAD_PHASE_STREAMING: u8 = 1;
const UPLOAD_PHASE_FINISHED: u8 = 2;

/// Watchdog re-check interval. Short enough that a phase transition (for
/// example the body finishing while a long stall deadline is pending) is
/// noticed promptly, and cheap because it only reads two atomics.
const UPLOAD_WATCHDOG_TICK: Duration = Duration::from_millis(25);

/// Progress state shared between the body stream and the watchdog.
/// Bytes accumulated by the shared body progress sink.
pub(crate) struct UploadTracker {
    started: Instant,
    phase: AtomicU8,
    /// Milliseconds since `started` of the last body chunk (or of start).
    last_progress_ms: AtomicU64,
    /// Milliseconds since `started` when the body finished; `u64::MAX` while
    /// the body has not finished.
    finished_at_ms: AtomicU64,
    /// Cumulative body bytes and the expected total, reported as progress.
    bytes_sent: AtomicU64,
    total_bytes: AtomicU64,
}

impl UploadTracker {
    pub(crate) fn new() -> Self {
        Self {
            started: Instant::now(),
            phase: AtomicU8::new(UPLOAD_PHASE_PRE_BODY),
            last_progress_ms: AtomicU64::new(0),
            finished_at_ms: AtomicU64::new(u64::MAX),
            bytes_sent: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
        }
    }

    /// Record the expected body size once it is known (the file was opened
    /// and verified). Only the total changes; no progress is implied.
    fn set_total_bytes(&self, total: u64) {
        self.total_bytes.store(total, Ordering::Release);
    }

    /// The body produced bytes: the stall clock restarts and streaming is
    /// considered started.
    pub(crate) fn record_body_progress(&self, bytes: u64) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.last_progress_ms.store(elapsed, Ordering::Release);
        self.bytes_sent.fetch_add(bytes, Ordering::AcqRel);
        if self.phase.load(Ordering::Acquire) == UPLOAD_PHASE_PRE_BODY {
            // Only the body streams perform this transition, so a benign
            // check-then-set is enough; the atomics provide visibility.
            self.phase.store(UPLOAD_PHASE_STREAMING, Ordering::Release);
        }
    }

    /// The whole body was handed to the transport: the stall rule stops and
    /// the server-processing clock starts.
    pub(crate) fn mark_body_finished(&self) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.finished_at_ms.store(elapsed, Ordering::Release);
        self.phase.store(UPLOAD_PHASE_FINISHED, Ordering::Release);
    }

    fn phase(&self) -> u8 {
        self.phase.load(Ordering::Acquire)
    }

    fn last_progress(&self) -> Duration {
        Duration::from_millis(self.last_progress_ms.load(Ordering::Acquire))
    }

    fn finished_at(&self) -> Duration {
        let value = self.finished_at_ms.load(Ordering::Acquire);
        if value == u64::MAX {
            self.started.elapsed()
        } else {
            Duration::from_millis(value)
        }
    }

    fn snapshot(&self) -> (u64, u64) {
        (
            self.bytes_sent.load(Ordering::Acquire),
            self.total_bytes.load(Ordering::Acquire),
        )
    }
}

/// Receives body progress from the tracked streams: bytes produced, and the
/// end of one item's body. A single-file send reports the end of the whole
/// request; an album reports it per item and finishes when the last one ends.
trait BodyProgressSink: Send + Sync {
    fn on_chunk(&self, bytes: u64);
    fn on_item_finished(&self);
    /// Cumulative `(sent_bytes, total_bytes)` for `UploadStage::Uploading`.
    fn snapshot(&self) -> (u64, u64);
}

impl BodyProgressSink for UploadTracker {
    fn on_chunk(&self, bytes: u64) {
        self.record_body_progress(bytes);
    }

    fn on_item_finished(&self) {
        self.mark_body_finished();
    }

    fn snapshot(&self) -> (u64, u64) {
        UploadTracker::snapshot(self)
    }
}

/// Aggregate progress of an album body: the request is only finished when
/// every item's body has been handed over.
struct AlbumProgress {
    started: Instant,
    total_items: u64,
    total_bytes: u64,
    finished_items: AtomicU64,
    bytes_sent: AtomicU64,
    /// Milliseconds since `started` of the last chunk, and of the last
    /// finished item, so the stall and server-processing rules work exactly as
    /// they do for a single-file body.
    last_progress_ms: AtomicU64,
    last_finish_ms: AtomicU64,
}

impl AlbumProgress {
    fn new(total_items: u64, total_bytes: u64) -> Self {
        Self {
            started: Instant::now(),
            total_items,
            total_bytes,
            finished_items: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            last_progress_ms: AtomicU64::new(0),
            last_finish_ms: AtomicU64::new(u64::MAX),
        }
    }

    fn phase(&self) -> u8 {
        if self.finished_items.load(Ordering::Acquire) >= self.total_items {
            UPLOAD_PHASE_FINISHED
        } else if self.bytes_sent.load(Ordering::Acquire) > 0 {
            UPLOAD_PHASE_STREAMING
        } else {
            UPLOAD_PHASE_PRE_BODY
        }
    }

    fn last_progress(&self) -> Duration {
        Duration::from_millis(self.last_progress_ms.load(Ordering::Acquire))
    }

    fn last_finish(&self) -> Duration {
        let value = self.last_finish_ms.load(Ordering::Acquire);
        if value == u64::MAX {
            self.started.elapsed()
        } else {
            Duration::from_millis(value)
        }
    }
}

impl BodyProgressSink for AlbumProgress {
    fn on_chunk(&self, bytes: u64) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.last_progress_ms.store(elapsed, Ordering::Release);
        self.bytes_sent.fetch_add(bytes, Ordering::AcqRel);
    }

    fn on_item_finished(&self) {
        let elapsed = self.started.elapsed().as_millis() as u64;
        self.last_finish_ms.store(elapsed, Ordering::Release);
        self.finished_items.fetch_add(1, Ordering::AcqRel);
    }

    fn snapshot(&self) -> (u64, u64) {
        (self.bytes_sent.load(Ordering::Acquire), self.total_bytes)
    }
}

/// The album counterpart of [`upload_watchdog`]: the same layered deadlines,
/// driven by the aggregate album phase.
async fn album_watchdog(progress: Arc<AlbumProgress>, timeouts: UploadTimeouts) -> TelegramError {
    loop {
        let elapsed = progress.started.elapsed();
        let mut wake: Option<Duration> = None;
        match progress.phase() {
            UPLOAD_PHASE_FINISHED => {
                let deadline = progress.last_finish() + timeouts.server_processing;
                if elapsed >= deadline {
                    return TelegramError::UploadServerTimeout;
                }
                wake = Some(deadline);
            }
            UPLOAD_PHASE_STREAMING => {
                let deadline = progress.last_progress() + timeouts.body_stall;
                if elapsed >= deadline {
                    return TelegramError::UploadBodyStalled;
                }
                wake = Some(deadline);
            }
            _ => {}
        }
        if let Some(total) = timeouts.total {
            if elapsed >= total {
                return TelegramError::UploadDeadlineExceeded;
            }
            wake = Some(wake.map_or(total, |existing| existing.min(total)));
        }
        let sleep_for = match wake {
            Some(deadline) => deadline.saturating_sub(elapsed).min(UPLOAD_WATCHDOG_TICK),
            None => UPLOAD_WATCHDOG_TICK,
        };
        tokio::time::sleep(sleep_for).await;
    }
}

/// One item of an album body: either a file to stream, or an already cached
/// `file_id` Telegram can reuse for this bot (plan TG-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlbumItemUpload {
    File(Box<UploadRequest>),
    CachedFileId {
        media_kind: MediaKind,
        file_id: String,
        caption: Option<String>,
    },
}

/// Streams a whole album (`sendMediaGroup`) as `multipart/form-data`.
///
/// Every file part is length-known and pull-based, so an album of videos is
/// never buffered whole. An item whose `file_id` is already cached for this
/// bot is sent as that value instead of being uploaded again.
#[allow(clippy::too_many_arguments)]
pub async fn send_media_group_attempt<P>(
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    chat_id: &str,
    message_thread_id: Option<i64>,
    items: &[AlbumItemUpload],
    timeouts: &UploadTimeouts,
    cancellation: &CancellationToken,
    on_progress: P,
) -> Result<SendAttemptSuccess, SendAttemptError>
where
    P: FnMut(UploadStage) + Send + 'static,
{
    if chat_id.trim().is_empty() {
        return Err(SendAttemptError::new(
            TelegramError::InvalidChatId,
            RequestProgress::NotSent,
        ));
    }
    if !(TELEGRAM_ALBUM_MIN_ITEMS..=TELEGRAM_MEDIA_GROUP_LIMIT).contains(&items.len()) {
        // One item is a single-media send, and more than the limit is refused
        // before any request is built.
        return Err(SendAttemptError::new(
            TelegramError::InvalidUploadRequest(format!(
                "an album needs {TELEGRAM_ALBUM_MIN_ITEMS}..={TELEGRAM_MEDIA_GROUP_LIMIT} items, got {}",
                items.len()
            )),
            RequestProgress::NotSent,
        ));
    }
    let progress: Arc<Mutex<UploadProgressCallback>> = Arc::new(Mutex::new(Box::new({
        let mut moved = on_progress;
        move |stage: UploadStage| moved(stage)
    })));
    emit_upload_progress(&progress, UploadStage::Queued);
    emit_upload_progress(&progress, UploadStage::CheckingFile);

    // Prepare every file before anything reaches the wire: an album must never
    // be half-sent because a later file turned out to be unreadable.
    let mut prepared: Vec<Option<(tokio::fs::File, UploadRequest, u64)>> =
        Vec::with_capacity(items.len());
    let mut total_bytes = 0_u64;
    for item in items {
        match item {
            AlbumItemUpload::CachedFileId { file_id, .. } => {
                if file_id.trim().is_empty() {
                    return Err(SendAttemptError::new(
                        TelegramError::InvalidUploadRequest(
                            "cached file id must not be empty".to_owned(),
                        ),
                        RequestProgress::NotSent,
                    ));
                }
                prepared.push(None);
            }
            AlbumItemUpload::File(request) => {
                validate_upload_request(request)
                    .map_err(|error| SendAttemptError::new(error, RequestProgress::NotSent))?;
                let file = tokio::fs::File::open(&request.file_path)
                    .await
                    .map_err(|error| {
                        SendAttemptError::new(
                            TelegramError::FileUnreadable(format!(
                                "{}: {error}",
                                request.file_name
                            )),
                            RequestProgress::NotSent,
                        )
                    })?;
                let metadata = file.metadata().await.map_err(|error| {
                    SendAttemptError::new(
                        TelegramError::FileUnreadable(format!("{}: {error}", request.file_name)),
                        RequestProgress::NotSent,
                    )
                })?;
                if metadata.is_dir() {
                    return Err(SendAttemptError::new(
                        TelegramError::FileUnreadable(format!(
                            "{}: path is a directory",
                            request.file_name
                        )),
                        RequestProgress::NotSent,
                    ));
                }
                let size = metadata.len();
                if let Some(expected_size) = request.expected_size
                    && expected_size != size
                {
                    return Err(SendAttemptError::new(
                        TelegramError::FileChanged(format!(
                            "expected {expected_size} bytes, found {size}"
                        )),
                        RequestProgress::NotSent,
                    ));
                }
                total_bytes = total_bytes.saturating_add(size);
                prepared.push(Some((file, (**request).clone(), size)));
            }
        }
    }

    let album_progress = Arc::new(AlbumProgress::new(items.len() as u64, total_bytes));
    let mut media = Vec::with_capacity(items.len());
    let mut form = reqwest::multipart::Form::new().text("chat_id", chat_id.to_owned());
    for (index, (item, entry)) in items.iter().zip(prepared.iter()).enumerate() {
        match (item, entry) {
            (
                AlbumItemUpload::CachedFileId {
                    media_kind,
                    file_id,
                    caption,
                },
                _,
            ) => {
                let mut media_item = serde_json::json!({
                    "type": media_kind.api_media_type(),
                    "media": file_id,
                });
                if let Some(caption) = caption {
                    media_item["caption"] = serde_json::Value::String(caption.clone());
                }
                media.push(media_item);
            }
            (AlbumItemUpload::File(_), Some((file, request, size))) => {
                let reader = file.try_clone().await.map_err(|error| {
                    SendAttemptError::new(
                        TelegramError::FileUnreadable(format!("{}: {error}", request.file_name)),
                        RequestProgress::NotSent,
                    )
                })?;
                let body = TrackedUploadStream {
                    inner: tokio_util::io::ReaderStream::with_capacity(
                        reader,
                        TELEGRAM_UPLOAD_CHUNK_BYTES,
                    ),
                    progress_sink: album_progress.clone(),
                    progress: Arc::clone(&progress),
                    finished: false,
                };
                let mut part = reqwest::multipart::Part::stream_with_length(
                    reqwest::Body::wrap_stream(body),
                    *size,
                )
                .file_name(request.file_name.clone());
                if let Some(mime_type) = request.mime_type.as_deref() {
                    part = part.mime_str(mime_type).map_err(|_| {
                        SendAttemptError::new(
                            TelegramError::InvalidUploadRequest(
                                "content type could not be parsed".to_owned(),
                            ),
                            RequestProgress::NotSent,
                        )
                    })?;
                }
                let media_field = format!("media_{index}");
                let mut media_item = serde_json::json!({
                    "type": request.media_kind.api_media_type(),
                    "media": format!("attach://{media_field}"),
                });
                if let Some(caption) = request.caption.as_deref() {
                    media_item["caption"] = serde_json::Value::String(caption.to_owned());
                }
                media.push(media_item);
                form = form.part(media_field, part);
            }
            _ => {
                return Err(SendAttemptError::new(
                    TelegramError::InvalidUploadRequest("album item was not prepared".to_owned()),
                    RequestProgress::NotSent,
                ));
            }
        }
    }

    form = form.text("media", serde_json::Value::Array(media).to_string());
    if let Some(thread_id) = message_thread_id {
        form = form.text("message_thread_id", thread_id.to_string());
    }

    let client = transport
        .upload_client(timeouts)
        .map_err(|error| SendAttemptError::new(error, RequestProgress::NotSent))?;
    let url = transport.method_url(token, "sendMediaGroup");
    let upload = async {
        let response = client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(map_upload_send_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(TelegramError::Transport(format!(
                "HTTP status {}",
                status.as_u16()
            )));
        }
        read_bounded_upload_response(response).await
    };
    let outcome: Result<TelegramResponse, TelegramError> = tokio::select! {
        result = async {
            tokio::select! {
                result = upload => result,
                error = album_watchdog(Arc::clone(&album_progress), timeouts.clone()) => Err(error),
            }
        } => result,
        () = cancellation.cancelled() => Err(TelegramError::UploadCancelled),
    };
    // The phase is read after the send, so a failure reports what was really
    // known about the request rather than a guess.
    let phase = album_progress.phase();
    let response = match outcome {
        Ok(response) => {
            emit_upload_progress(&progress, UploadStage::AwaitingResult);
            emit_upload_progress(&progress, UploadStage::Confirmed);
            response
        }
        Err(error) => return Err(SendAttemptError::new(error, progress_from_phase(phase))),
    };

    let message_ids = response.result_message_ids();
    let telegram_message_id = message_ids.first().cloned().ok_or_else(|| {
        SendAttemptError::new(
            TelegramError::Transport(
                "Telegram confirmed the request but returned no message id".to_owned(),
            ),
            RequestProgress::Sent,
        )
    })?;
    Ok(SendAttemptSuccess {
        telegram_message_id,
        results_json: Some(
            serde_json::json!({
                "message_ids": message_ids,
                "file_ids": response.result_file_ids(),
            })
            .to_string(),
        ),
    })
}

/// Waits until the first layered deadline fires and classifies it exactly.
///
/// Cancellation is handled by the caller's `select!`, so this future only
/// ever returns timeout classifications. Before the body starts streaming
/// only the overall deadline applies — connection setup is bounded by the
/// client's connect timeout instead.
pub(crate) async fn upload_watchdog(
    tracker: Arc<UploadTracker>,
    timeouts: UploadTimeouts,
) -> TelegramError {
    loop {
        let elapsed = tracker.started.elapsed();
        let mut wake: Option<Duration> = None;
        match tracker.phase() {
            UPLOAD_PHASE_STREAMING => {
                let deadline = tracker.last_progress() + timeouts.body_stall;
                if elapsed >= deadline {
                    return TelegramError::UploadBodyStalled;
                }
                wake = Some(deadline);
            }
            UPLOAD_PHASE_FINISHED => {
                let deadline = tracker.finished_at() + timeouts.server_processing;
                if elapsed >= deadline {
                    return TelegramError::UploadServerTimeout;
                }
                wake = Some(deadline);
            }
            _ => {}
        }
        if let Some(total) = timeouts.total {
            if elapsed >= total {
                return TelegramError::UploadDeadlineExceeded;
            }
            wake = Some(wake.map_or(total, |existing| existing.min(total)));
        }
        // Never sleep past a phase transition: a long stall deadline must not
        // hide an earlier server-processing or overall deadline.
        let sleep_for = match wake {
            Some(deadline) => deadline.saturating_sub(elapsed).min(UPLOAD_WATCHDOG_TICK),
            None => UPLOAD_WATCHDOG_TICK,
        };
        tokio::time::sleep(sleep_for).await;
    }
}

type UploadProgressCallback = Box<dyn FnMut(UploadStage) + Send>;

fn emit_upload_progress(progress: &Mutex<UploadProgressCallback>, stage: UploadStage) {
    let mut callback = progress.lock().expect("upload progress callback");
    callback(stage);
}

/// Wraps the file reader so the transport observes byte-level progress (the
/// stall clock) and can report the `Uploading`/`AwaitingResult` stages.
///
/// The wrapper is pull-based: exactly one chunk is read per poll, so memory
/// stays bounded regardless of file size.
struct TrackedUploadStream {
    inner: tokio_util::io::ReaderStream<tokio::fs::File>,
    progress_sink: Arc<dyn BodyProgressSink>,
    progress: Arc<Mutex<UploadProgressCallback>>,
    finished: bool,
}

impl Stream for TrackedUploadStream {
    type Item = std::io::Result<bytes::Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(context) {
            Poll::Ready(Some(Ok(bytes))) => {
                this.progress_sink.on_chunk(bytes.len() as u64);
                let (sent_bytes, total_bytes) = this.progress_sink.snapshot();
                emit_upload_progress(
                    &this.progress,
                    UploadStage::Uploading {
                        sent_bytes,
                        total_bytes,
                    },
                );
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => {
                if !this.finished {
                    this.finished = true;
                    this.progress_sink.on_item_finished();
                    emit_upload_progress(&this.progress, UploadStage::AwaitingResult);
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Maps a `send()` failure without ever formatting the `reqwest::Error`
/// itself, because its `Display` includes the token-bearing URL.
fn map_upload_send_error(error: reqwest::Error) -> TelegramError {
    if error.is_timeout() {
        // The upload client only configures a connect timeout, so a reqwest
        // timeout here is connection setup rather than the transfer.
        TelegramError::UploadConnectTimeout
    } else if error.is_body() {
        // The media stream failed mid-flight (file removed or truncated
        // after it was verified).
        TelegramError::FileUnreadable("media stream ended early".to_owned())
    } else {
        TelegramError::Transport("upload request failed".to_owned())
    }
}

/// Reads a Telegram response under the size bound, then parses and checks it
/// (`ok: true`, not just the HTTP status).
async fn read_bounded_upload_response(
    mut response: reqwest::Response,
) -> Result<TelegramResponse, TelegramError> {
    if let Some(length) = response.content_length()
        && length > TELEGRAM_MAX_RESPONSE_BYTES
    {
        return Err(TelegramError::ResponseTooLarge);
    }
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| TelegramError::Transport("upload response read failed".to_owned()))?
    {
        if body.len() as u64 + chunk.len() as u64 > TELEGRAM_MAX_RESPONSE_BYTES {
            return Err(TelegramError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    let parsed: TelegramResponse = serde_json::from_slice(&body)
        .map_err(|_| TelegramError::Transport("invalid Telegram JSON response".to_owned()))?;
    if !parsed.ok {
        return Err(TelegramError::Api {
            code: parsed.error_code.unwrap_or(0),
            description: parsed
                .description
                .unwrap_or_else(|| "Telegram API request failed".to_owned()),
            parameters: parsed.parameters,
        });
    }
    Ok(parsed)
}

/// Maps a body tracker phase to what is known about the request when it ends.
///
/// This is the only honest source for that value: guessing it would either
/// re-send an accepted message or treat an unsent one as lost (plan TG-04).
fn progress_from_phase(phase: u8) -> RequestProgress {
    match phase {
        UPLOAD_PHASE_FINISHED => RequestProgress::Sent,
        UPLOAD_PHASE_STREAMING => RequestProgress::Partial,
        _ => RequestProgress::NotSent,
    }
}

impl ReqwestTelegramTransport {
    /// The async client used for uploads. Built per upload so the layered
    /// connect timeout belongs to this send, not to the control-request
    /// client. Redirects are disabled: the URL carries the token.
    fn upload_client(&self, timeouts: &UploadTimeouts) -> Result<reqwest::Client, TelegramError> {
        let mut builder = reqwest::Client::builder()
            .connect_timeout(timeouts.connect)
            .redirect(reqwest::redirect::Policy::none());
        if self.disable_proxy {
            builder = builder.no_proxy();
        } else if let Some(proxy) = self.proxy.as_deref() {
            let proxy = reqwest::Proxy::all(proxy).map_err(|_| {
                TelegramError::Transport("failed to apply the configured proxy".to_owned())
            })?;
            builder = builder.proxy(proxy);
        }
        builder
            .build()
            .map_err(|_| TelegramError::Transport("failed to build HTTP client".to_owned()))
    }

    /// Streams one media file to the Bot API over `multipart/form-data`
    /// (plan TG-02: localised async upload transport).
    ///
    /// The file is re-opened from disk, verified against `expected_size`,
    /// read in bounded chunks and handed to reqwest as a length-known part;
    /// it is never read into memory as a whole. The caller races this future
    /// against `cancellation` with `tokio::select!`; the future itself
    /// settles `UploadCancelled` when the token fires, and a request that was
    /// already sent must not be auto-retried — the caller reports `UNKNOWN`
    /// instead (plan TG-02 cancellation semantics).
    ///
    /// `on_progress` receives ordered [`UploadStage`] events. `Confirmed` is
    /// emitted only after a parsed `ok: true` result, so "uploaded" is never
    /// presented as "sent" before Telegram confirms it.
    async fn send_upload_outcome<P>(
        &self,
        token: &BotToken,
        request: &UploadRequest,
        timeouts: &UploadTimeouts,
        cancellation: &CancellationToken,
        mut on_progress: P,
    ) -> (Result<TelegramResponse, TelegramError>, RequestProgress)
    where
        P: FnMut(UploadStage) + Send + 'static,
    {
        // The tracker lives outside the send so the caller can learn what is
        // known about the request once the attempt ends.
        let tracker = Arc::new(UploadTracker::new());
        let outcome: Result<TelegramResponse, TelegramError> = async {
        on_progress(UploadStage::Queued);
        validate_upload_request(request)?;
        timeouts.validate()?;
        if cancellation.is_cancelled() {
            return Err(TelegramError::UploadCancelled);
        }

        on_progress(UploadStage::CheckingFile);
        let file = tokio::fs::File::open(&request.file_path)
            .await
            .map_err(|error| {
                TelegramError::FileUnreadable(format!("{}: {error}", request.file_name))
            })?;
        let metadata = file.metadata().await.map_err(|error| {
            TelegramError::FileUnreadable(format!("{}: {error}", request.file_name))
        })?;
        if metadata.is_dir() {
            return Err(TelegramError::FileUnreadable(format!(
                "{}: path is a directory",
                request.file_name
            )));
        }
        let total_bytes = metadata.len();
        if let Some(expected_size) = request.expected_size
            && expected_size != total_bytes
        {
            return Err(TelegramError::FileChanged(format!(
                "expected {expected_size} bytes, found {total_bytes}"
            )));
        }

        let (method, field) = upload_method(request.media_kind);
        tracker.set_total_bytes(total_bytes);
        // The caller's callback is moved behind the shared handle the body
        // stream and the final `Confirmed` event both use.
        let progress: Arc<Mutex<UploadProgressCallback>> = Arc::new(Mutex::new(Box::new({
            let mut moved = on_progress;
            move |stage: UploadStage| moved(stage)
        })));
        let body = TrackedUploadStream {
            inner: tokio_util::io::ReaderStream::with_capacity(file, TELEGRAM_UPLOAD_CHUNK_BYTES),
            progress_sink: tracker.clone(),
            progress: Arc::clone(&progress),
            finished: false,
        };
        let mut part = reqwest::multipart::Part::stream_with_length(
            reqwest::Body::wrap_stream(body),
            total_bytes,
        )
        .file_name(request.file_name.clone());
        if let Some(mime_type) = request.mime_type.as_deref() {
            part = part.mime_str(mime_type).map_err(|_| {
                TelegramError::InvalidUploadRequest("content type could not be parsed".to_owned())
            })?;
        }
        let mut form = reqwest::multipart::Form::new()
            .text("chat_id", request.chat_id.clone())
            .part(field, part);
        if let Some(caption) = request.caption.as_deref() {
            form = form.text("caption", caption.to_owned());
        }
        if let Some(thread_id) = request.message_thread_id {
            form = form.text("message_thread_id", thread_id.to_string());
        }

        let client = self.upload_client(timeouts)?;
        let url = self.method_url(token, method);
        let upload = async {
            let response = client
                .post(url)
                .multipart(form)
                .send()
                .await
                .map_err(map_upload_send_error)?;
            let status = response.status();
            if !status.is_success() {
                return Err(TelegramError::Transport(format!(
                    "HTTP status {}",
                    status.as_u16()
                )));
            }
            read_bounded_upload_response(response).await
        };
        // Layers: the send itself, the watchdog's classification of the first
        // deadline, and the caller's cancellation token.
        let outcome: Result<TelegramResponse, TelegramError> = tokio::select! {
            result = async {
                tokio::select! {
                    result = upload => result,
                    error = upload_watchdog(Arc::clone(&tracker), timeouts.clone()) => Err(error),
                }
            } => result,
            () = cancellation.cancelled() => Err(TelegramError::UploadCancelled),
        };
        match outcome {
                Ok(response) => {
                    emit_upload_progress(&progress, UploadStage::Confirmed);
                    Ok(response)
                }
                Err(error) => Err(error),
            }
        }
        .await;
        (outcome, progress_from_phase(tracker.phase()))
    }

    /// Streams one media file to the Bot API over `multipart/form-data`
    /// (plan TG-02) and returns the parsed response.
    ///
    /// Use [`ReqwestTelegramTransport::send_upload_attempt`] when the send is
    /// driven by the outbox: that variant also reports what is known about the
    /// request when it fails.
    pub async fn send_upload<P>(
        &self,
        token: &BotToken,
        request: &UploadRequest,
        timeouts: &UploadTimeouts,
        cancellation: &CancellationToken,
        on_progress: P,
    ) -> Result<TelegramResponse, TelegramError>
    where
        P: FnMut(UploadStage) + Send + 'static,
    {
        self.send_upload_outcome(token, request, timeouts, cancellation, on_progress)
            .await
            .0
    }

    /// One media attempt as the outbox driver needs it.
    ///
    /// On success the confirmed message id (and any per-item file ids) are
    /// extracted; on failure the request progress comes from the body tracker,
    /// so [`run_claimed_attempt`] never has to guess it.
    pub async fn send_upload_attempt<P>(
        &self,
        token: &BotToken,
        request: &UploadRequest,
        timeouts: &UploadTimeouts,
        cancellation: &CancellationToken,
        on_progress: P,
    ) -> Result<SendAttemptSuccess, SendAttemptError>
    where
        P: FnMut(UploadStage) + Send + 'static,
    {
        let (outcome, progress) = self
            .send_upload_outcome(token, request, timeouts, cancellation, on_progress)
            .await;
        let response = outcome.map_err(|error| SendAttemptError::new(error, progress))?;
        let telegram_message_id = response.result_message_id().ok_or_else(|| {
            // `ok: true` without a message id cannot confirm which message was
            // created, so the outcome stays unknown instead of being guessed.
            SendAttemptError::new(
                TelegramError::Transport(
                    "Telegram confirmed the request but returned no message id".to_owned(),
                ),
                RequestProgress::Sent,
            )
        })?;
        let file_ids = response.result_file_ids();
        let results_json = if file_ids.is_empty() {
            None
        } else {
            Some(
                serde_json::json!({
                    "message_ids": response.result_message_ids(),
                    "file_ids": file_ids,
                })
                .to_string(),
            )
        };
        Ok(SendAttemptSuccess {
            telegram_message_id,
            results_json,
        })
    }
}
