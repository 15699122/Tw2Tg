//! Sidecar V2 discovery: request/outcome types and the event-stream consumer.
//!
//! Split out of `production.rs` without changing behaviour. The rules:
//!
//! * every candidate event is validated through `SidecarV2Event::validate` before
//!   it is accepted, so a malformed or unexpected event fails the discovery instead
//!   of being silently skipped;
//! * only `discovery_started`/`candidate`/`discovery_completed` are expected on this
//!   path; an extraction or download event is a protocol violation here;
//! * the outcome reports what the worker said it found, so the caller can tell an
//!   empty result from a truncated stream.

use std::time::Duration;

use xarchive_protocol::SidecarV2EventType;
use xarchive_sidecar_supervisor::{SidecarSupervisor, SupervisorEvent};

use crate::executor::CancellationToken;

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
