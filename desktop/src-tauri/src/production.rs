//! U7 production archive orchestration.
//!
//! This module owns the v2 extraction -> aria2 transfer boundary. Legacy v1
//! extraction/download code and the synchronous archive command were removed in
//! U8; `archive.rs` owns the executor context, staging verification, and the
//! `ArchiveService` commit.

use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use xarchive_download::{
    Aria2Supervisor, Aria2SupervisorConfig, Aria2TransferDriver, MediaTransferPlan,
    TransferControl, TransferDriver, TransferDriverConfig, TransferFailure, TransferFailureCode,
    TransferOutcome, media_transfer_plan,
};
use xarchive_protocol::{
    ExtractionMediaType, ExtractionResult, SidecarV2Command, SidecarV2EventType,
};
use xarchive_sidecar_supervisor::{SidecarSupervisor, SupervisorEvent};

use crate::archive::{SidecarArchiveResult, SidecarDownloadRequest};
use crate::executor::CancellationToken;

pub(crate) fn execute_v2_archive(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    aria2_program: Option<&str>,
    use_aria2: bool,
    network: &crate::executor::ExecutorNetworkConfig,
) -> Result<SidecarArchiveResult, String> {
    if use_aria2 {
        return execute_aria2_archive(supervisor, request, cancellation, aria2_program, network);
    }
    if aria2_program.is_some() {
        eprintln!(
            "warning: aria2 path is configured but optional downloads are off; aria2 will not start"
        );
    }
    execute_gallery_dl_archive(supervisor, request, cancellation, network)
}

fn execute_aria2_archive(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
) -> Result<SidecarArchiveResult, String> {
    let extraction = extract_v2(supervisor, request, cancellation)?;
    let staging_dir = &request.staging_dir;
    let initial_plan = media_transfer_plan(&extraction, staging_dir.display().to_string())
        .map_err(|error| format!("invalid extraction transfer plan: {error}"))?;

    let files = if initial_plan.items.is_empty() {
        Vec::new()
    } else {
        let (final_extraction, final_plan, outcomes) = transfer_with_one_refresh(
            supervisor,
            request,
            cancellation,
            extraction.clone(),
            initial_plan,
            aria2_program,
            network,
        )?;
        let converted = transfer_files(&final_plan, &outcomes, staging_dir)?;
        return Ok(SidecarArchiveResult {
            metadata: extraction_to_metadata(&final_extraction),
            files: converted,
        });
    };

    let metadata = extraction_to_metadata(&extraction);
    Ok(SidecarArchiveResult { metadata, files })
}

fn transfer_once(
    plan: &MediaTransferPlan,
    cancellation: &CancellationToken,
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
) -> Result<Vec<TransferOutcome>, TransferFailure> {
    let aria2_config = aria2_config(aria2_program, network).map_err(|error| TransferFailure {
        media_id: None,
        code: TransferFailureCode::Failed,
        message: error,
        aria2_error_code: None,
    })?;
    let mut aria2 = Aria2Supervisor::spawn(aria2_config).map_err(|error| TransferFailure {
        media_id: None,
        code: TransferFailureCode::Failed,
        message: format!("aria2 startup failed: {error}"),
        aria2_error_code: None,
    })?;
    let backend = aria2.backend().clone();
    let driver = Aria2TransferDriver::new(
        backend,
        TransferDriverConfig {
            poll_interval: std::time::Duration::from_millis(500),
            timeout: network.transfer_timeout,
        },
    );
    let control = TransferControl::new();
    let monitor_stop = Arc::new(AtomicBool::new(false));
    let monitor_stop_for_thread = monitor_stop.clone();
    let control_for_thread = control.clone();
    let cancellation_for_thread = cancellation.clone();
    let monitor = std::thread::spawn(move || {
        while !monitor_stop_for_thread.load(Ordering::Acquire) {
            if cancellation_for_thread.is_cancelled() {
                control_for_thread.cancel();
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    let result = driver.execute(plan, &control, &mut |_| {});
    monitor_stop.store(true, Ordering::Release);
    let _ = monitor.join();
    aria2.shutdown();
    result
}

fn transfer_with_one_refresh(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    initial_extraction: ExtractionResult,
    initial_plan: MediaTransferPlan,
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
) -> Result<(ExtractionResult, MediaTransferPlan, Vec<TransferOutcome>), String> {
    match transfer_once(&initial_plan, cancellation, aria2_program, network) {
        Ok(outcomes) => Ok((initial_extraction, initial_plan, outcomes)),
        Err(failure) if failure.code == TransferFailureCode::ExpiredUrl => {
            let refreshed = extract_v2(supervisor, request, cancellation)?;
            let refreshed_plan =
                media_transfer_plan(&refreshed, request.staging_dir.display().to_string())
                    .map_err(|error| format!("invalid refreshed transfer plan: {error}"))?;
            ensure_same_media_collection(&initial_plan, &refreshed_plan)?;
            let outcomes = transfer_once(&refreshed_plan, cancellation, aria2_program, network)
                .map_err(|failure| format_transfer_failure(&failure))?;
            Ok((refreshed, refreshed_plan, outcomes))
        }
        Err(failure) => Err(format_transfer_failure(&failure)),
    }
}

fn ensure_same_media_collection(
    before: &MediaTransferPlan,
    after: &MediaTransferPlan,
) -> Result<(), String> {
    let before_ids = before
        .items
        .iter()
        .map(|item| (&item.media_id, &item.filename))
        .collect::<std::collections::HashMap<_, _>>();
    let after_ids = after
        .items
        .iter()
        .map(|item| (&item.media_id, &item.filename))
        .collect::<std::collections::HashMap<_, _>>();
    if before_ids != after_ids {
        return Err(xarchive_download::EXTRACTION_RESULT_CHANGED.to_owned());
    }
    Ok(())
}

fn extract_v2(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
) -> Result<ExtractionResult, String> {
    let command = SidecarV2Command::extract(
        request.request_id.clone(),
        request.job_id.clone(),
        request.url.clone(),
        request.browser.clone(),
        request.profile.clone(),
    );
    supervisor
        .send_v2(&command)
        .map_err(|error| format!("failed to send v2 extraction command: {error}"))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(15 * 60);
    loop {
        if cancellation.is_cancelled() {
            let _ = supervisor.send_v2_cancel(
                format!("{}-cancel", request.request_id),
                request.job_id.clone(),
            );
            return Err("archive extraction cancelled".to_owned());
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err("sidecar extraction timed out".to_owned());
        }
        let event = supervisor
            .recv_timeout(remaining.min(Duration::from_millis(250)))
            .map_err(|error| format!("sidecar event failure: {error}"))?;
        let Some(event) = event else { continue };
        match event {
            SupervisorEvent::V2(event)
                if event.job_id == request.job_id
                    && event
                        .request_id
                        .as_deref()
                        .is_none_or(|id| id == request.request_id) =>
            {
                event
                    .validate()
                    .map_err(|error| format!("invalid v2 sidecar event: {error}"))?;
                match event.event {
                    SidecarV2EventType::Extracted => {
                        return event
                            .result
                            .ok_or_else(|| "extracted event did not include a result".to_owned());
                    }
                    SidecarV2EventType::Failed => {
                        return Err(format!(
                            "{}: {}",
                            event
                                .error_code
                                .unwrap_or_else(|| "SIDECAR_FAILED".to_owned()),
                            event
                                .error_message
                                .unwrap_or_else(|| "sidecar extraction failed".to_owned())
                        ));
                    }
                    SidecarV2EventType::Cancelled => {
                        return Err("archive extraction cancelled".to_owned());
                    }
                    SidecarV2EventType::ExtractionStarted
                    | SidecarV2EventType::Log
                    | SidecarV2EventType::Ready => {}
                    // Download events belong to the `download` path; accepting
                    // them here would silently drop a download result.
                    SidecarV2EventType::DownloadStarted | SidecarV2EventType::DownloadCompleted => {
                        return Err(format!(
                            "unexpected sidecar download event during extraction: {:?}",
                            event.event
                        ));
                    }
                    // Discovery events belong to the `discover` path; accepting
                    // them here would silently drop a candidate.
                    SidecarV2EventType::DiscoveryStarted
                    | SidecarV2EventType::Candidate
                    | SidecarV2EventType::DiscoveryCompleted => {
                        return Err(format!(
                            "unexpected sidecar discovery event during extraction: {:?}",
                            event.event
                        ));
                    }
                }
            }
            SupervisorEvent::Exited(result) => {
                return Err(format!("sidecar exited during extraction: {result:?}"));
            }
            SupervisorEvent::ProtocolError { message, .. } => {
                return Err(format!("sidecar protocol error: {message}"));
            }
            SupervisorEvent::V2(_) | SupervisorEvent::Stderr(_) => {}
        }
    }
}

/// Run one v2 `download` command and wait for `download_completed`.
///
/// This is the optional direct-download path (`use_aria2 = false`): gallery-dl
/// downloads the media itself, so no aria2 transfer plan is ever built.
fn download_v2(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
) -> Result<(xarchive_protocol::DownloadResult, ExtractionResult), String> {
    let command = SidecarV2Command::download(
        request.request_id.clone(),
        request.job_id.clone(),
        request.url.clone(),
        request.browser.clone(),
        request.profile.clone(),
        request.staging_dir.display().to_string(),
    );
    supervisor
        .send_v2(&command)
        .map_err(|error| format!("failed to send v2 download command: {error}"))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(30 * 60);
    loop {
        if cancellation.is_cancelled() {
            let _ = supervisor.send_v2_cancel(
                format!("{}-cancel", request.request_id),
                request.job_id.clone(),
            );
            // A rebuild-driven executor replacement and an explicit user cancel
            // both trip this token. Keep the message factual about the token
            // rather than claiming a user action, so triage can separate a
            // settings-change interruption from a deliberate cancel.
            return Err("archive download interrupted by executor cancellation".to_owned());
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err("sidecar download timed out".to_owned());
        }
        let event = supervisor
            .recv_timeout(remaining.min(Duration::from_millis(250)))
            .map_err(|error| format!("sidecar event failure: {error}"))?;
        let Some(event) = event else { continue };
        match event {
            SupervisorEvent::V2(mut event)
                if event.job_id == request.job_id
                    && event
                        .request_id
                        .as_deref()
                        .is_none_or(|id| id == request.request_id) =>
            {
                event
                    .validate()
                    .map_err(|error| format!("invalid v2 sidecar event: {error}"))?;
                match event.event {
                    SidecarV2EventType::DownloadCompleted => {
                        let download = event.download.take().ok_or_else(|| {
                            "download_completed event did not include a download result".to_owned()
                        })?;
                        let extraction = event.result.take().or_else(|| download.result.clone());
                        let extraction = extraction.ok_or_else(|| {
                            "download_completed event did not include an extraction result"
                                .to_owned()
                        })?;
                        if extraction.tweet_id != download.tweet_id {
                            return Err("download result tweet identity does not match extraction"
                                .to_owned());
                        }
                        return Ok((download, extraction));
                    }
                    SidecarV2EventType::Failed => {
                        return Err(format!(
                            "{}: {}",
                            event
                                .error_code
                                .unwrap_or_else(|| "SIDECAR_FAILED".to_owned()),
                            event
                                .error_message
                                .unwrap_or_else(|| "sidecar download failed".to_owned())
                        ));
                    }
                    SidecarV2EventType::Cancelled => {
                        // A worker-side cancel arrives here as well. It is a
                        // distinct source from the executor-token path above:
                        // the worker decided, versus our own shutdown.
                        return Err("archive download cancelled by worker".to_owned());
                    }
                    SidecarV2EventType::DownloadStarted
                    | SidecarV2EventType::Log
                    | SidecarV2EventType::Ready => {}
                    // Extraction and discovery events belong to their own paths.
                    SidecarV2EventType::ExtractionStarted
                    | SidecarV2EventType::Extracted
                    | SidecarV2EventType::DiscoveryStarted
                    | SidecarV2EventType::Candidate
                    | SidecarV2EventType::DiscoveryCompleted => {
                        return Err(format!(
                            "unexpected sidecar event during download: {:?}",
                            event.event
                        ));
                    }
                }
            }
            SupervisorEvent::Exited(result) => {
                return Err(format!("sidecar exited during download: {result:?}"));
            }
            SupervisorEvent::ProtocolError { message, .. } => {
                return Err(format!("sidecar protocol error: {message}"));
            }
            SupervisorEvent::V2(_) | SupervisorEvent::Stderr(_) => {}
        }
    }
}

/// Verify the reported gallery-dl files against the staging directory and
/// convert them into the durable commit contract.
///
/// The worker is a separate process, so every reported path is re-anchored
/// under staging, checked for regular-file shape, and compared against the
/// size the worker claimed. A tweet whose extraction lists media but produced
/// no file is rejected instead of being archived as metadata-only.
fn verify_downloaded_files(
    download: &xarchive_protocol::DownloadResult,
    extraction: &ExtractionResult,
    staging_dir: &Path,
) -> Result<Vec<xarchive_protocol::DownloadFile>, String> {
    if !extraction.media.is_empty() && download.media.is_empty() {
        return Err("download completed without any media files".to_owned());
    }

    let mut files = Vec::with_capacity(download.media.len());
    for item in &download.media {
        let path = staging_dir.join(&item.relative_path);
        let relative = path.strip_prefix(staging_dir).map_err(|_| {
            format!(
                "download path escaped staging directory: {}",
                path.display()
            )
        })?;
        if relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(format!("download path is unsafe: {}", relative.display()));
        }
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("download output is unavailable: {error}"))?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "download output is not a regular file: {}",
                path.display()
            ));
        }
        if metadata.len() != item.size_bytes {
            return Err(format!(
                "download size mismatch for {}: reported {} bytes, found {} bytes",
                item.relative_path,
                item.size_bytes,
                metadata.len()
            ));
        }
        files.push(xarchive_protocol::DownloadFile {
            relative_path: item.relative_path.clone(),
            size_bytes: metadata.len(),
            media_type: media_type_name(&item.media_type),
            mime_type: item.mime_type.clone(),
        });
    }
    Ok(files)
}

/// Optional direct-download archive path (`use_aria2 = false`).
///
/// gallery-dl downloads the media into the job staging directory itself; this
/// runner only validates the reported staging-relative files against disk and
/// converts them into the durable commit contract. No aria2 process starts.
fn execute_gallery_dl_archive(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    _network: &crate::executor::ExecutorNetworkConfig,
) -> Result<SidecarArchiveResult, String> {
    let (download, extraction) = download_v2(supervisor, request, cancellation)?;
    let files = verify_downloaded_files(&download, &extraction, &request.staging_dir)?;

    let metadata = extraction_to_metadata(&extraction);
    Ok(SidecarArchiveResult { metadata, files })
}

/// One account-discovery run identity.
pub(crate) struct DiscoveryRequest {
    pub(crate) batch_id: String,
    pub(crate) request_id: String,
    pub(crate) profile_url: String,
    pub(crate) browser: Option<String>,
    pub(crate) profile: Option<String>,
}

/// Durable discovery output: every accepted candidate plus the worker's count.
pub(crate) struct DiscoveryOutcome {
    pub(crate) candidates: Vec<xarchive_protocol::DiscoveryCandidate>,
    pub(crate) candidates_found: u64,
}

/// Run one v2 `discover` command and collect its candidate events.
///
/// Discovery is streamed: every `candidate` event is validated by
/// `SidecarV2Event::validate` before it is accepted, and the run ends on
/// `discovery_completed`. Candidate payloads never carry media transfer facts,
/// so the dispatcher re-extracts each Tweet when it archives it.
pub(crate) fn execute_v2_discovery(
    supervisor: &mut SidecarSupervisor,
    request: &DiscoveryRequest,
    cancellation: &CancellationToken,
    timeout: Duration,
    mut on_candidate: impl FnMut(&xarchive_protocol::DiscoveryCandidate) -> Result<(), String>,
) -> Result<DiscoveryOutcome, String> {
    supervisor
        .send_v2_discover(
            request.request_id.clone(),
            request.batch_id.clone(),
            request.profile_url.clone(),
            request.browser.clone(),
            request.profile.clone(),
        )
        .map_err(|error| format!("failed to send v2 discovery command: {error}"))?;

    let mut candidates = Vec::new();
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if cancellation.is_cancelled() {
            let _ = supervisor.send_v2_cancel(
                format!("{}-cancel", request.request_id),
                request.batch_id.clone(),
            );
            return Err("account discovery cancelled".to_owned());
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err("sidecar discovery timed out".to_owned());
        }
        let event = supervisor
            .recv_timeout(remaining.min(Duration::from_millis(250)))
            .map_err(|error| format!("sidecar event failure: {error}"))?;
        let Some(event) = event else { continue };
        match event {
            SupervisorEvent::V2(event)
                if event.job_id == request.batch_id
                    && event
                        .request_id
                        .as_deref()
                        .is_none_or(|id| id == request.request_id) =>
            {
                event
                    .validate()
                    .map_err(|error| format!("invalid v2 sidecar event: {error}"))?;
                match event.event {
                    SidecarV2EventType::DiscoveryStarted => {}
                    SidecarV2EventType::Candidate => {
                        let candidate = event.candidate.ok_or_else(|| {
                            "candidate event did not include a candidate payload".to_owned()
                        })?;
                        on_candidate(&candidate)?;
                        candidates.push(candidate);
                    }
                    SidecarV2EventType::DiscoveryCompleted => {
                        let candidates_found = event.candidates_found.ok_or_else(|| {
                            "discovery_completed event did not include a candidate count".to_owned()
                        })?;
                        return Ok(DiscoveryOutcome {
                            candidates,
                            candidates_found,
                        });
                    }
                    SidecarV2EventType::Failed => {
                        return Err(format!(
                            "{}: {}",
                            event
                                .error_code
                                .unwrap_or_else(|| "DISCOVERY_FAILED".to_owned()),
                            event
                                .error_message
                                .unwrap_or_else(|| "sidecar account discovery failed".to_owned())
                        ));
                    }
                    SidecarV2EventType::Cancelled => {
                        return Err("account discovery cancelled".to_owned());
                    }
                    SidecarV2EventType::Log | SidecarV2EventType::Ready => {}
                    SidecarV2EventType::ExtractionStarted | SidecarV2EventType::Extracted => {
                        return Err(format!(
                            "unexpected sidecar extraction event during discovery: {:?}",
                            event.event
                        ));
                    }
                    SidecarV2EventType::DownloadStarted | SidecarV2EventType::DownloadCompleted => {
                        return Err(format!(
                            "unexpected sidecar download event during discovery: {:?}",
                            event.event
                        ));
                    }
                }
            }
            SupervisorEvent::Exited(result) => {
                return Err(format!("sidecar exited during discovery: {result:?}"));
            }
            SupervisorEvent::ProtocolError { message, .. } => {
                return Err(format!("sidecar protocol error: {message}"));
            }
            SupervisorEvent::V2(_) | SupervisorEvent::Stderr(_) => {}
        }
    }
}

fn aria2_config(
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
) -> Result<Aria2SupervisorConfig, String> {
    let program = aria2_program
        .map(str::to_owned)
        .or_else(|| std::env::var("XARCHIVE_ARIA2_PROGRAM").ok())
        .ok_or_else(|| "ARIA2_NOT_CONFIGURED: aria2 executable is not configured".to_owned())?;
    let port = std::env::var("XARCHIVE_ARIA2_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|value| *value != 0)
        .unwrap_or_else(available_loopback_port);
    Aria2SupervisorConfig::new_with_random_secret(program, "127.0.0.1", port)
        .map(|config| {
            config
                .with_network(
                    network.aria2_connect_timeout,
                    network.aria2_idle_timeout,
                    network.aria2_max_tries,
                )
                .with_proxy_mode(network.proxy_mode)
                .with_proxy(network.proxy.clone())
        })
        .map_err(|error| format!("invalid aria2 configuration: {error}"))
}

fn available_loopback_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap_or(6800)
}

fn transfer_files(
    plan: &MediaTransferPlan,
    outcomes: &[xarchive_download::TransferOutcome],
    staging_dir: &Path,
) -> Result<Vec<xarchive_protocol::DownloadFile>, String> {
    let mut files = Vec::with_capacity(outcomes.len());
    for outcome in outcomes {
        let item = plan
            .items
            .iter()
            .find(|item| item.media_id == outcome.media_id)
            .ok_or_else(|| format!("transfer returned unknown media id: {}", outcome.media_id))?;
        let path = outcome
            .path
            .as_deref()
            .ok_or_else(|| format!("transfer has no output path: {}", outcome.media_id))?;
        let path = PathBuf::from(path);
        let relative = path.strip_prefix(staging_dir).map_err(|_| {
            format!(
                "transfer path escaped staging directory: {}",
                path.display()
            )
        })?;
        if relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(format!("transfer path is unsafe: {}", relative.display()));
        }
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("transfer output is unavailable: {error}"))?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "transfer output is not a regular file: {}",
                path.display()
            ));
        }
        files.push(xarchive_protocol::DownloadFile {
            relative_path: relative.to_string_lossy().to_string(),
            size_bytes: metadata.len(),
            media_type: item.media_type.clone(),
            mime_type: item.mime_type.clone(),
        });
    }
    Ok(files)
}

fn media_type_name(media_type: &ExtractionMediaType) -> String {
    match media_type {
        ExtractionMediaType::Photo => "photo",
        ExtractionMediaType::Video => "video",
        ExtractionMediaType::Unknown => "unknown",
    }
    .to_owned()
}

fn extraction_to_metadata(result: &ExtractionResult) -> serde_json::Value {
    serde_json::json!({
        "tweet_id": result.tweet_id,
        "url": result.url,
        "tweet_type": result.tweet_type,
        "text": result.text,
        "username": result.username,
        "display_name": result.display_name,
        "created_at": result.created_at,
        "user_id": result.user_id,
        "reply_to": result.reply_to,
        "quoted_tweet": result.quoted_tweet.as_ref().map(|quoted| serde_json::json!({
            "tweet_id": quoted.tweet_id,
            "url": quoted.url,
            "username": quoted.username,
            "display_name": quoted.display_name,
            "user_id": quoted.user_id,
            "text": quoted.text,
            "created_at": quoted.created_at,
            "tweet_type": quoted.tweet_type,
        })),
        "media": result.media.iter().map(|media| serde_json::json!({
            "media_id": media.media_id,
            "id": media.media_id,
            "type": media_type_name(&media.media_type),
        })).collect::<Vec<_>>(),
    })
}

fn format_transfer_failure(failure: &xarchive_download::TransferFailure) -> String {
    let code = match failure.code {
        TransferFailureCode::Cancelled => "TRANSFER_CANCELLED",
        TransferFailureCode::Interrupted => "TRANSFER_INTERRUPTED",
        TransferFailureCode::Timeout => "TRANSFER_TIMEOUT",
        TransferFailureCode::ExpiredUrl => "TRANSFER_EXPIRED_URL",
        TransferFailureCode::Failed => "TRANSFER_FAILED",
    };
    format!("{code}: {}", failure.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xarchive_core::ProxyMode;
    use xarchive_protocol::{DownloadResult, DownloadedMediaFile, ExtractionMediaItem};

    fn staging_extraction() -> ExtractionResult {
        ExtractionResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            tweet_type: "post".to_owned(),
            text: None,
            username: None,
            display_name: None,
            created_at: None,
            user_id: None,
            reply_to: None,
            quoted_tweet: None,
            media: vec![ExtractionMediaItem {
                index: 1,
                media_id: Some("media-01".to_owned()),
                media_type: ExtractionMediaType::Photo,
                url: "https://cdn.example/1.jpg".to_owned(),
                filename: "01.jpg".to_owned(),
                mime_type: Some("image/jpeg".to_owned()),
            }],
            request_headers: Vec::new(),
        }
    }

    fn staging_download(relative_path: &str, size_bytes: u64) -> DownloadResult {
        DownloadResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            media: vec![DownloadedMediaFile {
                index: 1,
                media_id: Some("media-01".to_owned()),
                media_type: ExtractionMediaType::Photo,
                filename: "01.jpg".to_owned(),
                relative_path: relative_path.to_owned(),
                size_bytes,
                mime_type: Some("image/jpeg".to_owned()),
            }],
            result: None,
        }
    }

    fn temp_staging(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xarchive-download-{}-{}-{name}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&root).expect("staging directory");
        root
    }

    #[test]
    fn verifies_reported_download_files_against_the_staging_directory() {
        let staging = temp_staging("verify");
        std::fs::write(staging.join("01.jpg"), b"jpeg-bytes").expect("media");
        let download = staging_download("01.jpg", b"jpeg-bytes".len() as u64);
        let files = verify_downloaded_files(&download, &staging_extraction(), &staging)
            .expect("verified files");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "01.jpg");
        assert_eq!(files[0].size_bytes, 10);
        assert_eq!(files[0].media_type, "photo");
        let _ = std::fs::remove_dir_all(&staging);
    }

    #[test]
    fn rejects_escaping_missing_mismatched_and_empty_downloads() {
        let staging = temp_staging("reject");
        let extraction = staging_extraction();

        // A reported path that leaves staging is refused even when the file
        // exists outside it. Protocol validation already rejects this shape;
        // this is the defense-in-depth check at the filesystem boundary.
        std::fs::write(staging.join("..").join("outside.jpg"), b"jpeg-bytes").ok();
        let escaping = staging_download("../outside.jpg", 10);
        let error =
            verify_downloaded_files(&escaping, &extraction, &staging).expect_err("escaping path");
        assert!(
            error.contains("escaped staging directory") || error.contains("path is unsafe"),
            "unexpected error: {error}"
        );

        // A missing file never becomes an archived media record.
        let missing = staging_download("missing.jpg", 10);
        assert!(
            verify_downloaded_files(&missing, &extraction, &staging)
                .expect_err("missing file")
                .contains("download output is unavailable")
        );

        // A worker-reported size that disagrees with disk is refused.
        std::fs::write(staging.join("01.jpg"), b"jpeg-bytes").expect("media");
        let mismatch = staging_download("01.jpg", 999);
        assert!(
            verify_downloaded_files(&mismatch, &extraction, &staging)
                .expect_err("size mismatch")
                .contains("download size mismatch")
        );

        // Media in extraction but no file at all is refused.
        let empty = DownloadResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            media: Vec::new(),
            result: None,
        };
        assert_eq!(
            verify_downloaded_files(&empty, &extraction, &staging)
                .expect_err("empty download")
                .as_str(),
            "download completed without any media files"
        );
        let _ = std::fs::remove_dir_all(&staging);
    }

    #[test]
    fn production_aria2_configuration_generates_a_fresh_secret_without_environment_setup() {
        let network =
            crate::executor::ExecutorNetworkConfig::from_seconds(60, 30, 5, 10, 2, 30, 60)
                .with_proxy(
                    ProxyMode::Manual,
                    Some("http://alice:s3cret@proxy.example:8080".to_owned()),
                );
        let config = aria2_config(Some("aria2c"), &network).expect("aria2 config");
        assert_eq!(config.rpc_secret.len(), 64);
        assert!(
            config
                .rpc_secret
                .chars()
                .all(|value| value.is_ascii_hexdigit())
        );
        assert!(!format!("{config:?}").contains("s3cret"));
    }

    #[test]
    fn the_aria2_configuration_carries_the_selected_mode() {
        let base =
            || crate::executor::ExecutorNetworkConfig::from_seconds(60, 30, 5, 10, 2, 30, 60);
        let manual = base().with_proxy(
            ProxyMode::Manual,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        let manual_config = aria2_config(Some("aria2c"), &manual).expect("manual config");
        assert_eq!(manual_config.proxy_mode, ProxyMode::Manual);
        assert!(
            manual_config
                .environment()
                .iter()
                .any(|(key, value)| key == "all_proxy" && value.contains("s3cret"))
        );
        assert!(manual_config.environment_remove().is_empty());

        let direct = base().with_proxy(
            ProxyMode::Direct,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        let direct_config = aria2_config(Some("aria2c"), &direct).expect("direct config");
        assert_eq!(direct_config.proxy_mode, ProxyMode::Direct);
        assert!(
            direct_config.environment().is_empty(),
            "Direct must not hand the stored proxy to aria2"
        );
        assert!(
            direct_config
                .environment_remove()
                .iter()
                .any(|key| key == "http_proxy")
        );

        let system = base().with_proxy(
            ProxyMode::System,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        let system_config = aria2_config(Some("aria2c"), &system).expect("system config");
        assert_eq!(system_config.proxy_mode, ProxyMode::System);
        assert!(
            system_config.environment().is_empty(),
            "System must not pin the stored manual value on aria2"
        );
    }

    #[test]
    fn converts_extraction_to_durable_metadata_without_headers_or_urls() {
        let result = ExtractionResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/a/status/123".to_owned(),
            tweet_type: "post".to_owned(),
            text: Some("hello".to_owned()),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            created_at: None,
            user_id: Some("9001".to_owned()),
            reply_to: Some("111".to_owned()),
            quoted_tweet: Some(xarchive_protocol::ExtractionQuotedTweet {
                tweet_id: "987".to_owned(),
                url: "https://x.com/bob/status/987".to_owned(),
                username: Some("bob".to_owned()),
                display_name: Some("Bob".to_owned()),
                user_id: Some("9002".to_owned()),
                text: Some("original".to_owned()),
                created_at: None,
                tweet_type: Some("post".to_owned()),
            }),
            media: vec![],
            request_headers: vec![xarchive_protocol::ExtractionRequestHeader {
                name: "Referer".to_owned(),
                value: "https://x.com/".to_owned(),
            }],
        };
        let metadata = extraction_to_metadata(&result);
        assert_eq!(metadata["tweet_id"], "123");
        assert_eq!(metadata["user_id"], "9001");
        assert_eq!(metadata["reply_to"], "111");
        assert_eq!(metadata["quoted_tweet"]["tweet_id"], "987");
        assert_eq!(metadata["quoted_tweet"]["user_id"], "9002");
        assert!(metadata.get("request_headers").is_none());
        assert!(metadata.get("signed_url").is_none());
    }

    #[test]
    fn rejects_changed_media_identity_or_filename_sets() {
        let before = MediaTransferPlan {
            items: vec![xarchive_download::TransferPlanItem {
                media_id: "m1".into(),
                url: "https://cdn.example/1.jpg".into(),
                filename: "01.jpg".into(),
                directory: "/tmp/staging".into(),
                headers: vec![],
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
            }],
        };
        let mut after = before.clone();
        after.items[0].filename = "02.jpg".into();
        assert_eq!(
            ensure_same_media_collection(&before, &after),
            Err(xarchive_download::EXTRACTION_RESULT_CHANGED.to_owned())
        );
    }
}
