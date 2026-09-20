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
) -> Result<Vec<TransferOutcome>, TransferFailure> {
    let aria2_config = aria2_config(aria2_program).map_err(|error| TransferFailure {
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
    let driver = Aria2TransferDriver::new(backend, TransferDriverConfig::default());
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
) -> Result<(ExtractionResult, MediaTransferPlan, Vec<TransferOutcome>), String> {
    match transfer_once(&initial_plan, cancellation, aria2_program) {
        Ok(outcomes) => Ok((initial_extraction, initial_plan, outcomes)),
        Err(failure) if failure.code == TransferFailureCode::ExpiredUrl => {
            let refreshed = extract_v2(supervisor, request, cancellation)?;
            let refreshed_plan =
                media_transfer_plan(&refreshed, request.staging_dir.display().to_string())
                    .map_err(|error| format!("invalid refreshed transfer plan: {error}"))?;
            ensure_same_media_collection(&initial_plan, &refreshed_plan)?;
            let outcomes = transfer_once(&refreshed_plan, cancellation, aria2_program)
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

fn aria2_config(aria2_program: Option<&str>) -> Result<Aria2SupervisorConfig, String> {
    let program = aria2_program
        .map(str::to_owned)
        .or_else(|| std::env::var("XARCHIVE_ARIA2_PROGRAM").ok())
        .ok_or_else(|| "ARIA2_NOT_CONFIGURED: aria2 executable is not configured".to_owned())?;
    let port = std::env::var("XARCHIVE_ARIA2_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(6800);
    let secret = std::env::var("XARCHIVE_ARIA2_RPC_SECRET")
        .map_err(|_| "ARIA2_NOT_CONFIGURED: XARCHIVE_ARIA2_RPC_SECRET is missing".to_owned())?;
    Aria2SupervisorConfig::new(program, "127.0.0.1", port, secret)
        .map_err(|error| format!("invalid aria2 configuration: {error}"))
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
            media: vec![],
            request_headers: vec![xarchive_protocol::ExtractionRequestHeader {
                name: "Referer".to_owned(),
                value: "https://x.com/".to_owned(),
            }],
        };
        let metadata = extraction_to_metadata(&result);
        assert_eq!(metadata["tweet_id"], "123");
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
