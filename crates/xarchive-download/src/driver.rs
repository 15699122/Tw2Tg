//! Backend-neutral media transfer driver with an aria2-only implementation.
//!
//! The driver consumes a [`MediaTransferPlan`] produced from an extraction
//! result and drives every media item through the [`DownloadBackend`] to a
//! terminal state. It never falls back to another extractor and never
//! performs extraction itself.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::error::DownloadError;
use crate::model::{DownloadBackend, TransferId, TransferState};
use crate::rpc::AddUriRequest;

/// One media item to transfer, derived from an extraction result media plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferPlanItem {
    pub media_id: String,
    pub url: String,
    pub filename: String,
    pub directory: String,
    pub headers: Vec<String>,
}

/// The ordered set of media items to transfer for one archive job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MediaTransferPlan {
    pub items: Vec<TransferPlanItem>,
}

impl MediaTransferPlan {
    pub fn validate(&self) -> Result<(), DownloadError> {
        if self.items.is_empty() {
            return Err(DownloadError::InvalidTransferPlan(
                "plan has no media items".into(),
            ));
        }
        let mut seen = HashSet::new();
        for item in &self.items {
            if item.media_id.trim().is_empty() {
                return Err(DownloadError::InvalidTransferPlan(
                    "media id must not be empty".into(),
                ));
            }
            if !seen.insert(item.media_id.as_str()) {
                return Err(DownloadError::InvalidTransferPlan(format!(
                    "duplicate media id: {}",
                    item.media_id
                )));
            }
            if !(item.url.starts_with("https://") || item.url.starts_with("http://")) {
                return Err(DownloadError::UnsupportedUrl);
            }
            if item.filename.is_empty()
                || item.filename == "."
                || item.filename == ".."
                || item.filename.contains('/')
                || item.filename.contains('\\')
            {
                return Err(DownloadError::InvalidFilename);
            }
            if item.directory.is_empty() {
                return Err(DownloadError::InvalidTransferPlan(
                    "directory must not be empty".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Cooperative stop signals shared between the caller and the driver.
#[derive(Clone, Default)]
pub struct TransferControl {
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

impl TransferControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

/// Progress snapshot emitted while transfers are running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferProgress {
    pub media_id: String,
    pub state: TransferState,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
}

/// Successful terminal result for one media item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferOutcome {
    pub media_id: String,
    pub transfer_id: TransferId,
    pub path: Option<String>,
    pub total_bytes: Option<u64>,
}

/// Stable failure codes reported by the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferFailureCode {
    Cancelled,
    Interrupted,
    Timeout,
    ExpiredUrl,
    Failed,
}

impl TransferFailureCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cancelled => "TRANSFER_CANCELLED",
            Self::Interrupted => "TRANSFER_INTERRUPTED",
            Self::Timeout => "TRANSFER_TIMEOUT",
            Self::ExpiredUrl => "TRANSFER_EXPIRED_URL",
            Self::Failed => "TRANSFER_FAILED",
        }
    }
}

/// Terminal failure of a plan execution. `media_id` identifies the item that
/// failed, when the failure is attributable to one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferFailure {
    pub media_id: Option<String>,
    pub code: TransferFailureCode,
    pub message: String,
    pub aria2_error_code: Option<String>,
}

impl TransferFailure {
    fn failed(media_id: Option<String>, message: impl Into<String>) -> Self {
        Self {
            media_id,
            code: TransferFailureCode::Failed,
            message: message.into(),
            aria2_error_code: None,
        }
    }

    fn from_code(code: TransferFailureCode, media_id: Option<String>) -> Self {
        Self {
            media_id,
            code,
            message: code.as_str().to_owned(),
            aria2_error_code: None,
        }
    }

    fn expired_url(media_id: String, code: String, message: String) -> Self {
        Self {
            media_id: Some(media_id),
            code: TransferFailureCode::ExpiredUrl,
            message,
            aria2_error_code: Some(code),
        }
    }
}

impl std::fmt::Display for TransferFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.media_id {
            Some(media_id) => write!(
                formatter,
                "{}: {}: {}",
                self.code.as_str(),
                media_id,
                self.message
            ),
            None => write!(formatter, "{}: {}", self.code.as_str(), self.message),
        }
    }
}

/// Drives a [`MediaTransferPlan`] to terminal states.
pub trait TransferDriver {
    fn execute(
        &self,
        plan: &MediaTransferPlan,
        control: &TransferControl,
        on_progress: &mut dyn FnMut(TransferProgress),
    ) -> Result<Vec<TransferOutcome>, TransferFailure>;
}

/// Tunables for [`Aria2TransferDriver`].
#[derive(Debug, Clone, Copy)]
pub struct TransferDriverConfig {
    pub poll_interval: Duration,
    pub timeout: Duration,
}

impl Default for TransferDriverConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(500),
            timeout: Duration::from_secs(1800),
        }
    }
}

struct PendingItem {
    item: TransferPlanItem,
    transfer_id: TransferId,
    last_completed_bytes: u64,
    last_path: Option<String>,
    path_hint: String,
}

/// aria2-only transfer driver over any [`DownloadBackend`].
pub struct Aria2TransferDriver<B: DownloadBackend> {
    backend: B,
    config: TransferDriverConfig,
}

impl<B: DownloadBackend> Aria2TransferDriver<B> {
    pub fn new(backend: B, config: TransferDriverConfig) -> Self {
        Self { backend, config }
    }

    fn stop_failure(&self, control: &TransferControl) -> TransferFailure {
        if control.is_cancelled() {
            TransferFailure::from_code(TransferFailureCode::Cancelled, None)
        } else {
            TransferFailure::from_code(TransferFailureCode::Interrupted, None)
        }
    }

    /// Best-effort abandon: cancel every pending transfer and remove the
    /// aria2 control file plus the partial download of each known path.
    fn abandon_pending<'a, I>(&self, pending: I)
    where
        I: IntoIterator<Item = &'a PendingItem>,
    {
        for entry in pending {
            let _ = self.backend.cancel(&entry.transfer_id);
            let path = entry
                .last_path
                .as_deref()
                .unwrap_or(entry.path_hint.as_str());
            cleanup_partial_files(path);
        }
    }
}

impl<B: DownloadBackend> TransferDriver for Aria2TransferDriver<B> {
    fn execute(
        &self,
        plan: &MediaTransferPlan,
        control: &TransferControl,
        on_progress: &mut dyn FnMut(TransferProgress),
    ) -> Result<Vec<TransferOutcome>, TransferFailure> {
        plan.validate()
            .map_err(|error| TransferFailure::failed(None, error.to_string()))?;

        let deadline = Instant::now() + self.config.timeout;
        let mut pending: Vec<PendingItem> = Vec::new();
        let mut completed: Vec<TransferOutcome> = Vec::new();

        for item in &plan.items {
            if control.is_cancelled() || control.is_shutdown() {
                self.abandon_pending(&pending);
                return Err(self.stop_failure(control));
            }
            let request = AddUriRequest {
                url: item.url.clone(),
                directory: item.directory.clone(),
                filename: item.filename.clone(),
                headers: item.headers.clone(),
            };
            let transfer_id = match self.backend.add_uri(request) {
                Ok(transfer_id) => transfer_id,
                Err(error) => {
                    let failure =
                        TransferFailure::failed(Some(item.media_id.clone()), error.to_string());
                    self.abandon_pending(&pending);
                    return Err(failure);
                }
            };
            pending.push(PendingItem {
                item: item.clone(),
                transfer_id,
                last_completed_bytes: 0,
                last_path: None,
                path_hint: format!("{}/{}", item.directory.trim_end_matches('/'), item.filename),
            });
        }

        loop {
            if control.is_cancelled() || control.is_shutdown() {
                self.abandon_pending(&pending);
                return Err(self.stop_failure(control));
            }
            if Instant::now() >= deadline {
                self.abandon_pending(&pending);
                return Err(TransferFailure::from_code(
                    TransferFailureCode::Timeout,
                    None,
                ));
            }
            let mut still_pending: Vec<PendingItem> = Vec::new();
            for mut entry in pending.drain(..) {
                let status = match self.backend.status(&entry.transfer_id) {
                    Ok(status) => status,
                    Err(error) => {
                        let media_id = entry.item.media_id.clone();
                        self.abandon_pending(&still_pending);
                        self.abandon_pending(std::slice::from_ref(&entry));
                        return Err(TransferFailure::failed(Some(media_id), error.to_string()));
                    }
                };
                match status.state {
                    TransferState::Complete => {
                        // aria2 removes the control file itself on success.
                        completed.push(TransferOutcome {
                            media_id: entry.item.media_id.clone(),
                            transfer_id: entry.transfer_id.clone(),
                            path: status.files.first().map(|file| file.path.clone()),
                            total_bytes: status.total_bytes,
                        });
                    }
                    TransferState::Error => {
                        let message = status
                            .error_message
                            .clone()
                            .unwrap_or_else(|| "aria2 reported an error state".into());
                        let media_id = entry.item.media_id.clone();
                        entry.last_path = status.files.first().map(|file| file.path.clone());
                        self.abandon_pending(&still_pending);
                        self.abandon_pending(std::slice::from_ref(&entry));
                        if is_expired_url_error(
                            status.error_code.as_deref(),
                            status.error_message.as_deref(),
                        ) {
                            return Err(TransferFailure::expired_url(
                                media_id,
                                status.error_code.unwrap_or_else(|| "403".into()),
                                message,
                            ));
                        }
                        return Err(TransferFailure::failed(Some(media_id), message));
                    }
                    TransferState::Removed => {
                        let media_id = entry.item.media_id.clone();
                        entry.last_path = status.files.first().map(|file| file.path.clone());
                        self.abandon_pending(&still_pending);
                        self.abandon_pending(std::slice::from_ref(&entry));
                        return Err(TransferFailure::failed(
                            Some(media_id),
                            "aria2 transfer was removed before completion",
                        ));
                    }
                    TransferState::Waiting | TransferState::Active | TransferState::Paused => {
                        if status.completed_bytes < entry.last_completed_bytes {
                            let media_id = entry.item.media_id.clone();
                            self.abandon_pending(&still_pending);
                            self.abandon_pending(std::slice::from_ref(&entry));
                            return Err(TransferFailure::failed(
                                Some(media_id),
                                "transfer progress regressed",
                            ));
                        }
                        on_progress(TransferProgress {
                            media_id: entry.item.media_id.clone(),
                            state: status.state,
                            completed_bytes: status.completed_bytes,
                            total_bytes: status.total_bytes,
                        });
                        entry.last_completed_bytes = status.completed_bytes;
                        entry.last_path = status.files.first().map(|file| file.path.clone());
                        still_pending.push(entry);
                    }
                }
            }
            if still_pending.is_empty() {
                return Ok(order_by_plan(plan, completed));
            }
            pending = still_pending;
            std::thread::sleep(self.config.poll_interval);
        }
    }
}

fn order_by_plan(
    plan: &MediaTransferPlan,
    completed: Vec<TransferOutcome>,
) -> Vec<TransferOutcome> {
    let mut by_media_id: HashMap<String, TransferOutcome> = completed
        .into_iter()
        .map(|outcome| (outcome.media_id.clone(), outcome))
        .collect();
    plan.items
        .iter()
        .filter_map(|item| by_media_id.remove(&item.media_id))
        .collect()
}

/// Best-effort removal of the aria2 control file and the partial download.
fn cleanup_partial_files(path: &str) {
    let _ = std::fs::remove_file(format!("{path}.aria2"));
    let _ = std::fs::remove_file(path);
}

fn is_expired_url_error(code: Option<&str>, message: Option<&str>) -> bool {
    let code = code.unwrap_or_default().to_ascii_lowercase();
    let message = message.unwrap_or_default().to_ascii_lowercase();
    code == "403"
        || code == "401"
        || code.contains("forbidden")
        || message.contains("403")
        || message.contains("forbidden")
        || message.contains("expired")
        || message.contains("signature")
        || message.contains("access denied")
}
