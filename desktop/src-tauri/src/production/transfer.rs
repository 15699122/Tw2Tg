//! aria2 transfer: the retained-owner, drain and single-refresh path.
//!
//! Split out of `production.rs` without changing behaviour. The invariants Plan §3
//! names explicitly, and why each matters:
//!
//! * a containment failure never falls back to a plain spawn. Rejecting startup
//!   fails the attempt; falling back would run the downloader outside the owned
//!   cgroup and silently lose the drain guarantee.
//!
//! * the supervisor is retained across a refreshable failure. `TransferAttemptFailure`
//!   carries `retained_supervisor` so the caller can retry the *same* owned owner
//!   instead of spawning a second one.
//!
//! * the drain happens once per retained owner. `retry_retained_drain` re-drains the
//!   owner it is given and never acquires a new one.
//!
//! * the media collection identity is checked, so a retry cannot mix the media set
//!   of one attempt with the files of another.
//!
//! * exactly one refresh is attempted. A second refresh is a different attempt, not
//!   a loop inside this function.

use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[cfg(target_os = "linux")]
use xarchive_download::Aria2StartupFailure;
use xarchive_download::{
    Aria2Supervisor, Aria2SupervisorConfig, Aria2TransferDriver, MediaTransferPlan,
    TransferControl, TransferDriver, TransferDriverConfig, TransferFailure, TransferFailureCode,
    TransferOutcome, media_transfer_plan,
};
use xarchive_protocol::ExtractionResult;
use xarchive_sidecar_supervisor::SidecarSupervisor;

use crate::archive::{SidecarArchiveResult, SidecarDownloadRequest};
use crate::executor::CancellationToken;

use super::media_type_name;
use super::sidecar_operations::extract_v2;

pub(super) fn execute_aria2_archive(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
    #[cfg(target_os = "linux")] containment: Option<&crate::executor::AttemptWriterDomain>,
) -> Result<SidecarArchiveResult, (String, Option<Aria2Supervisor>)> {
    let extraction =
        extract_v2(supervisor, request, cancellation).map_err(|message| (message, None))?;
    let staging_dir = &request.staging_dir;
    let initial_plan = media_transfer_plan(&extraction, staging_dir.display().to_string())
        .map_err(|error| (format!("invalid extraction transfer plan: {error}"), None))?;

    let files = if initial_plan.items.is_empty() {
        Vec::new()
    } else {
        let (final_extraction, final_plan, outcomes, retained_aria2) = transfer_with_one_refresh(
            supervisor,
            request,
            cancellation,
            (extraction.clone(), initial_plan),
            aria2_program,
            network,
            #[cfg(target_os = "linux")]
            containment,
        )?;
        let converted = match transfer_files(&final_plan, &outcomes, staging_dir) {
            Ok(converted) => converted,
            Err(message) => return Err((message, retained_aria2)),
        };
        return Ok(SidecarArchiveResult {
            metadata: extraction_to_metadata(&final_extraction),
            files: converted,
            #[cfg(target_os = "linux")]
            retained_aria2,
        });
    };

    let metadata = extraction_to_metadata(&extraction);
    Ok(SidecarArchiveResult {
        metadata,
        files,
        #[cfg(target_os = "linux")]
        retained_aria2: None,
    })
}

#[cfg(target_os = "linux")]
pub(super) fn transfer_once(
    plan: &MediaTransferPlan,
    cancellation: &CancellationToken,
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
    user_args: &[String],
    #[cfg(target_os = "linux")] containment: Option<&crate::executor::AttemptWriterDomain>,
    #[cfg(target_os = "linux")] writer_slot: u8,
) -> Result<Vec<TransferOutcome>, TransferAttemptFailure> {
    let aria2_config = aria2_config(aria2_program, network, user_args)
        .map_err(|error| TransferFailure {
            media_id: None,
            code: TransferFailureCode::Failed,
            message: error,
            aria2_error_code: None,
        })
        .map_err(TransferAttemptFailure::diagnostic)?;
    let startup = match containment {
        Some(domain) => {
            // The attempt-bound recorded domain plus the
            // recover-before-create protocol replaces the retired
            // per-process unique group name. The refresh transfer uses a
            // distinct writer slot, so successor recovery enumeration
            // stays exact.
            let owner = crate::executor::recover_and_create_writer_group(
                domain,
                &format!("aria2-{writer_slot}"),
            )
            .map_err(|error| {
                TransferAttemptFailure::diagnostic(TransferFailure {
                    media_id: None,
                    code: TransferFailureCode::Failed,
                    message: error.to_string(),
                    aria2_error_code: None,
                })
            })?;
            Aria2Supervisor::spawn_contained_owned(
                aria2_config,
                owner,
                &domain.provisioning.python,
                &domain.provisioning.launcher,
            )
            .map_err(TransferAttemptFailure::from_startup)
        }
        None => Aria2Supervisor::spawn(aria2_config).map_err(|error| {
            TransferAttemptFailure::diagnostic(TransferFailure {
                media_id: None,
                code: TransferFailureCode::Failed,
                message: format!("aria2 startup failed: {error}"),
                aria2_error_code: None,
            })
        }),
    };
    let aria2 = startup?;
    run_transfer_with_supervisor(plan, cancellation, network, aria2)
}

#[cfg(not(target_os = "linux"))]
pub(super) fn transfer_once(
    plan: &MediaTransferPlan,
    cancellation: &CancellationToken,
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
    user_args: &[String],
) -> Result<Vec<TransferOutcome>, TransferFailure> {
    let aria2_config =
        aria2_config(aria2_program, network, user_args).map_err(|error| TransferFailure {
            media_id: None,
            code: TransferFailureCode::Failed,
            message: error,
            aria2_error_code: None,
        })?;
    let aria2 = Aria2Supervisor::spawn(aria2_config).map_err(|error| TransferFailure {
        media_id: None,
        code: TransferFailureCode::Failed,
        message: format!("aria2 startup failed: {error}"),
        aria2_error_code: None,
    })?;
    run_transfer_with_supervisor(plan, cancellation, network, aria2)
}

pub(super) fn run_transfer_with_supervisor(
    plan: &MediaTransferPlan,
    cancellation: &CancellationToken,
    network: &crate::executor::ExecutorNetworkConfig,
    mut aria2: Aria2Supervisor,
) -> Result<Vec<TransferOutcome>, TransferAttemptFailure> {
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
    let monitor_result = monitor.join();
    let shutdown_result = aria2.shutdown();
    match (result, monitor_result, shutdown_result) {
        (Ok(outcomes), Ok(()), Ok(())) => Ok(outcomes),
        (Err(mut failure), monitor, shutdown) => {
            let mut shutdown_failed = false;
            if monitor.is_err() {
                failure.code = TransferFailureCode::ShutdownIncomplete;
                shutdown_failed = true;
                failure
                    .message
                    .push_str("; aria2 cancellation monitor panicked");
            }
            if let Err(error) = shutdown {
                failure.code = TransferFailureCode::ShutdownIncomplete;
                shutdown_failed = true;
                failure
                    .message
                    .push_str(&format!("; aria2 shutdown incomplete: {error}"));
            }
            if shutdown_failed {
                return Err(TransferAttemptFailure {
                    diagnostic: failure,
                    retained_supervisor: Some(aria2),
                });
            }
            return Err(TransferAttemptFailure::diagnostic(failure));
        }
        (Ok(_), Err(_), _) => {
            return Err(TransferAttemptFailure {
                diagnostic: TransferFailure {
                    media_id: None,
                    code: TransferFailureCode::ShutdownIncomplete,
                    message: "aria2 cancellation monitor panicked during shutdown".to_owned(),
                    aria2_error_code: None,
                },
                retained_supervisor: Some(aria2),
            });
        }
        (Ok(_), Ok(()), Err(error)) => {
            return Err(TransferAttemptFailure {
                diagnostic: TransferFailure {
                    media_id: None,
                    code: TransferFailureCode::ShutdownIncomplete,
                    message: format!("aria2 shutdown incomplete: {error}"),
                    aria2_error_code: None,
                },
                retained_supervisor: Some(aria2),
            });
        }
    }
}

pub(super) fn transfer_with_one_refresh(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    initial: (ExtractionResult, MediaTransferPlan),
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
    #[cfg(target_os = "linux")] containment: Option<&crate::executor::AttemptWriterDomain>,
) -> Result<
    (
        ExtractionResult,
        MediaTransferPlan,
        Vec<TransferOutcome>,
        Option<Aria2Supervisor>,
    ),
    (String, Option<Aria2Supervisor>),
> {
    let (initial_extraction, initial_plan) = initial;
    let user_args = request.downloader_arguments.aria2.clone();
    match transfer_once(
        &initial_plan,
        cancellation,
        aria2_program,
        network,
        &user_args,
        #[cfg(target_os = "linux")]
        containment,
        #[cfg(target_os = "linux")]
        0,
    ) {
        Ok(outcomes) => Ok((initial_extraction, initial_plan, outcomes, None)),
        Err(mut failure) if transfer_failure_enters_refresh(&failure) => {
            // Refresh and its successor transfer stay behind the retained
            // owner's successful drain. A failed retry returns the same owner.
            if let Err(error) = retry_retained_drain(&mut failure) {
                let retained = failure.retained_supervisor.take();
                return Err((
                    format!(
                        "{}; retained aria2 owner drain retry failed: {error}",
                        format_transfer_failure(&failure.diagnostic)
                    ),
                    retained,
                ));
            }
            let refreshed =
                extract_v2(supervisor, request, cancellation).map_err(|message| (message, None))?;
            let refreshed_plan =
                media_transfer_plan(&refreshed, request.staging_dir.display().to_string())
                    .map_err(|error| (format!("invalid refreshed transfer plan: {error}"), None))?;
            ensure_same_media_collection(&initial_plan, &refreshed_plan)
                .map_err(|error| (error, None))?;
            let outcomes = transfer_once(
                &refreshed_plan,
                cancellation,
                aria2_program,
                network,
                &user_args,
                #[cfg(target_os = "linux")]
                containment,
                #[cfg(target_os = "linux")]
                1,
            )
            .map_err(TransferAttemptFailure::into_archive_error)?;
            Ok((refreshed, refreshed_plan, outcomes, None))
        }
        #[cfg(target_os = "linux")]
        Err(failure) => {
            let (message, retained) = failure.into_archive_error();
            Err((message, retained))
        }
        #[cfg(not(target_os = "linux"))]
        Err(failure) => Err((format_transfer_failure(&failure), None)),
    }
}

pub(super) fn ensure_same_media_collection(
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

#[cfg(target_os = "linux")]
pub(super) fn transfer_failure_enters_refresh(failure: &TransferAttemptFailure) -> bool {
    transfer_failure_allows_refresh(
        failure.diagnostic.code,
        failure.retained_supervisor.is_some(),
    )
}

#[cfg(target_os = "linux")]
pub(super) fn transfer_failure_allows_refresh(
    code: TransferFailureCode,
    has_retained_owner: bool,
) -> bool {
    code == TransferFailureCode::ExpiredUrl && !has_retained_owner
}

#[cfg(target_os = "linux")]
pub(super) fn retry_retained_drain_with<T, E>(
    owner: &mut Option<T>,
    shutdown: impl FnOnce(&mut T) -> Result<(), E>,
) -> Result<bool, E> {
    let Some(retained) = owner.as_mut() else {
        return Ok(false);
    };
    shutdown(retained)?;
    *owner = None;
    Ok(true)
}

#[cfg(target_os = "linux")]
pub(super) fn retry_retained_drain(
    failure: &mut TransferAttemptFailure,
) -> Result<bool, xarchive_download::DownloadError> {
    retry_retained_drain_with(&mut failure.retained_supervisor, Aria2Supervisor::shutdown)
}

#[cfg(target_os = "linux")]
pub(super) struct TransferAttemptFailure {
    pub(in crate::production) diagnostic: TransferFailure,
    pub(in crate::production) retained_supervisor: Option<Aria2Supervisor>,
}

#[cfg(target_os = "linux")]
impl TransferAttemptFailure {
    fn into_archive_error(self) -> (String, Option<Aria2Supervisor>) {
        (
            format_transfer_failure(&self.diagnostic),
            self.retained_supervisor,
        )
    }

    fn from_startup(failure: Aria2StartupFailure) -> Self {
        Self {
            diagnostic: TransferFailure {
                media_id: None,
                code: TransferFailureCode::Failed,
                message: format!("aria2 startup failed: {}", failure.error),
                aria2_error_code: None,
            },
            retained_supervisor: failure.owner,
        }
    }

    fn diagnostic(failure: TransferFailure) -> Self {
        Self {
            diagnostic: failure,
            retained_supervisor: None,
        }
    }
}

pub(super) fn aria2_config(
    aria2_program: Option<&str>,
    network: &crate::executor::ExecutorNetworkConfig,
    user_args: &[String],
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
                .with_user_args(user_args.to_vec())
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

pub(super) fn extraction_to_metadata(result: &ExtractionResult) -> serde_json::Value {
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

pub(super) fn format_transfer_failure(failure: &xarchive_download::TransferFailure) -> String {
    let code = match failure.code {
        TransferFailureCode::Cancelled => "TRANSFER_CANCELLED",
        TransferFailureCode::Interrupted => "TRANSFER_INTERRUPTED",
        TransferFailureCode::Timeout => "TRANSFER_TIMEOUT",
        TransferFailureCode::ExpiredUrl => "TRANSFER_EXPIRED_URL",
        TransferFailureCode::Failed => "TRANSFER_FAILED",
        TransferFailureCode::ShutdownIncomplete => "TRANSFER_SHUTDOWN_INCOMPLETE",
    };
    format!("{code}: {}", failure.message)
}
