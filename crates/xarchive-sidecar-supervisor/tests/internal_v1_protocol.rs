//! Non-containment protocol controls for the InternalV1 Sidecar fixture.
//!
//! These tests need no cgroup delegation: they exercise the production JSONL
//! command/event contract against the real in-repo fixture, so a fixture or
//! protocol regression is detected before any delegated containment run.
//!
//! They do not replace containment, drain, or registered-writer acceptance.
//! Cancellation here proves command targeting and the reported result; it does
//! not prove that a writer domain was drained or removed.

use std::path::Path;
use std::time::Duration;

use xarchive_protocol::SidecarV2EventType;
use xarchive_sidecar_supervisor::{SidecarSupervisor, SupervisorEvent};

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../desktop/src-tauri/tests/fixtures/internal_v1_worker.py")
}

fn spawn_fixture() -> SidecarSupervisor {
    let fixture = fixture_path();
    assert!(fixture.is_file(), "internal V1 worker fixture is required");
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_owned());
    SidecarSupervisor::spawn_ready_v2(
        &python,
        &["-u", fixture.to_str().unwrap()],
        Duration::from_secs(10),
    )
    .expect("fixture must start and complete the v2 handshake")
}

fn next_v2(supervisor: &SidecarSupervisor, context: &str) -> xarchive_protocol::SidecarV2Event {
    match supervisor
        .recv_timeout(Duration::from_secs(10))
        .unwrap_or_else(|error| panic!("{context}: receive failed: {error}"))
        .unwrap_or_else(|| panic!("{context}: timed out waiting for a v2 event"))
    {
        SupervisorEvent::V2(event) => *event,
        SupervisorEvent::Exited(status) => panic!("{context}: worker exited: {status:?}"),
        SupervisorEvent::Stderr(line) => {
            panic!("{context}: unexpected stderr line instead of an event: {line}")
        }
        SupervisorEvent::ProtocolError { line, message } => {
            panic!("{context}: unparsable protocol line {line:?}: {message}")
        }
    }
}

/// A cancel addressed to a different job must not cancel the running transfer,
/// and a cancel addressed to the running job must report the production
/// `INTERRUPTED` result for that exact job.
#[test]
fn cancel_targets_only_the_active_download() {
    let mut supervisor = spawn_fixture();
    let staging =
        std::env::temp_dir().join(format!("tw2tg-v1-protocol-staging-{}", std::process::id()));
    std::fs::create_dir_all(&staging).expect("create staging directory");

    supervisor
        .send_v2_download(
            "req-download",
            "job-active",
            "https://x.com/a/status/123",
            None,
            None,
            staging.to_string_lossy(),
            Vec::new(),
        )
        .expect("download command is accepted");

    let started = next_v2(&supervisor, "download_started");
    assert_eq!(started.event, SidecarV2EventType::DownloadStarted);
    assert_eq!(started.job_id, "job-active");
    assert_eq!(started.request_id.as_deref(), Some("req-download"));

    // Negative control: the wrong job id must be rejected, not acted on.
    supervisor
        .send_v2_cancel("req-wrong", "job-unrelated")
        .expect("unrelated cancel is delivered");
    let rejected = next_v2(&supervisor, "unrelated cancel");
    assert_eq!(rejected.event, SidecarV2EventType::Failed);
    assert_eq!(rejected.job_id, "job-unrelated");
    assert_eq!(rejected.error_code.as_deref(), Some("CANCELLED"));
    assert_eq!(
        rejected.error_message.as_deref(),
        Some("no matching extraction is running"),
        "an unrelated cancel must report that no matching work is running"
    );
    assert!(
        supervisor.is_running().expect("worker state is readable"),
        "an unrelated cancel must not stop the worker"
    );

    // The real cancellation of the active transfer.
    supervisor
        .send_v2_cancel("req-download", "job-active")
        .expect("cancel for the active job is delivered");
    let cancelled = next_v2(&supervisor, "active cancel");
    assert_eq!(
        cancelled.event,
        SidecarV2EventType::Failed,
        "cancelling the active download must report a failure result"
    );
    assert_eq!(
        cancelled.job_id, "job-active",
        "the cancellation result must belong to the cancelled job, not to the \
         unrelated job that was rejected above"
    );
    assert_eq!(cancelled.error_code.as_deref(), Some("INTERRUPTED"));
    assert_eq!(
        cancelled.request_id.as_deref(),
        Some("req-download"),
        "the cancellation result must bind to the cancelled request"
    );

    supervisor.shutdown().expect("worker drains cleanly");
    assert!(
        supervisor.readers_finished(),
        "owned pipe readers must join after shutdown"
    );
    let _ = std::fs::remove_dir_all(&staging);
}

/// The fixture must not emit events outside the production protocol enum, and
/// shutdown must end the process without an acknowledgement event.
#[test]
fn shutdown_exits_without_an_out_of_protocol_acknowledgement() {
    let mut supervisor = spawn_fixture();
    supervisor
        .send_v2_shutdown("req-shutdown")
        .expect("shutdown command is delivered");
    assert!(
        supervisor
            .wait_for_exit(Duration::from_secs(10))
            .expect("wait for fixture exit"),
        "the worker must exit after shutdown"
    );
    // Any leftover protocol line would surface here as a typed event or a
    // ProtocolError; only clean channel completion is acceptable.
    loop {
        let event = match supervisor.recv_timeout(Duration::from_millis(100)) {
            Ok(Some(event)) => event,
            // Readers have finished and every sender is dropped: no event was
            // left over, which is the contract this test asserts.
            Ok(None) => break,
            Err(xarchive_sidecar_supervisor::SupervisorError::NotRunning) => break,
            Err(error) => panic!("event channel failed: {error}"),
        };
        match event {
            SupervisorEvent::V2(event) => panic!("unexpected event after shutdown: {event:?}"),
            SupervisorEvent::ProtocolError { line, message } => {
                panic!("out-of-protocol line after shutdown: {line:?} ({message})")
            }
            SupervisorEvent::Stderr(_) | SupervisorEvent::Exited(_) => {}
        }
    }
    assert!(
        supervisor.readers_finished(),
        "owned pipe readers must join after shutdown"
    );
}
