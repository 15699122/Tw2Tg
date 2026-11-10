//! Sidecar V2 extraction and download operations.
//!
//! Split out of `production.rs` without changing behaviour. The rules:
//!
//! * the extraction path accepts only extraction events; a discovery or download
//!   event on this stream is a protocol violation, not something to ignore;
//! * the download path is the non-aria2 route: it consumes `download` events and
//!   then verifies the staging tree against the requested media set;
//! * verification checks the staging-relative path, that each entry is a regular
//!   file, and the recorded byte count, so a partially written or rewritten file is
//!   refused rather than archived;
//! * a cancellation observed mid-stream ends the operation instead of leaving the
//!   supervisor in an ambiguous state.

use std::path::{Component, Path};
use std::time::Duration;

use xarchive_protocol::{ExtractionResult, SidecarV2Command, SidecarV2EventType};
use xarchive_sidecar_supervisor::{SidecarSupervisor, SupervisorEvent};

use crate::archive::SidecarDownloadRequest;
use crate::executor::CancellationToken;

use super::media_type_name;

pub(crate) fn extract_v2(
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
pub(crate) fn download_v2(
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
        request.downloader_arguments.gallery_dl.clone(),
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
pub(crate) fn verify_downloaded_files(
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
