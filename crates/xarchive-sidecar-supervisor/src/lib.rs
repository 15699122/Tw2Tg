//! Process supervision for the Python JSONL Sidecar.
//!
//! This crate intentionally owns only process and stream mechanics. Archive
//! state, retries, persistence, and the decision that a job is complete stay
//! in the Desktop application.

use std::io;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::thread;
use std::time::Duration;

use xarchive_protocol::{
    SIDECAR_PROTOCOL_VERSION, SidecarV2Command, SidecarV2EventType, write_json_line,
};

mod error;
mod events;
mod readers;

pub use error::SupervisorError;
pub use events::SupervisorEvent;

pub struct SidecarSupervisor {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    events: Receiver<SupervisorEvent>,
}
impl SidecarSupervisor {
    pub fn spawn(program: &str, args: &[&str]) -> Result<Self, SupervisorError> {
        Self::spawn_with_env(program, args, &[])
    }

    pub fn spawn_with_env(
        program: &str,
        args: &[&str],
        env: &[(String, String)],
    ) -> Result<Self, SupervisorError> {
        let mut command = Command::new(program);
        hide_console_window(&mut command);
        command
            .args(args)
            .envs(env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        configure_process_group(&mut command);
        let mut child = command.spawn().map_err(SupervisorError::Spawn)?;
        let stdin = child.stdin.take().ok_or_else(|| {
            SupervisorError::Spawn(io::Error::other("sidecar stdin was not piped"))
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            SupervisorError::Spawn(io::Error::other("sidecar stdout was not piped"))
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            SupervisorError::Spawn(io::Error::other("sidecar stderr was not piped"))
        })?;

        let (sender, events) = mpsc::channel();
        readers::spawn_stdout_reader(stdout, sender.clone());
        readers::spawn_stderr_reader(stderr, sender);

        Ok(Self {
            child: Some(child),
            stdin: Some(stdin),
            events,
        })
    }

    pub fn send_v2(&mut self, command: &SidecarV2Command) -> Result<(), SupervisorError> {
        let stdin = self.stdin.as_mut().ok_or(SupervisorError::NotRunning)?;
        write_json_line(stdin, command).map_err(SupervisorError::Send)
    }

    pub fn send_v2_cancel(
        &mut self,
        request_id: impl Into<String>,
        job_id: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        self.send_v2(&SidecarV2Command {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: xarchive_protocol::SidecarV2CommandType::Cancel,
            job_id: job_id.into(),
            url: None,
            browser: None,
            profile: None,
        })
    }

    pub fn send_v2_shutdown(
        &mut self,
        request_id: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        self.send_v2(&SidecarV2Command {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: xarchive_protocol::SidecarV2CommandType::Shutdown,
            job_id: "system".to_owned(),
            url: None,
            browser: None,
            profile: None,
        })
    }

    /// Request account discovery for one profile URL.
    ///
    /// The batch id travels as the v2 `job_id` so every candidate event of this
    /// discovery run is fenced to one batch without a separate correlation
    /// channel.
    pub fn send_v2_discover(
        &mut self,
        request_id: impl Into<String>,
        batch_id: impl Into<String>,
        profile_url: impl Into<String>,
        browser: Option<String>,
        profile: Option<String>,
    ) -> Result<(), SupervisorError> {
        self.send_v2(&SidecarV2Command::discover(
            request_id,
            batch_id,
            profile_url,
            browser,
            profile,
        ))
    }

    /// Spawn a sidecar and require the capability-bearing `hello -> ready`
    /// handshake.
    ///
    /// The stdout reader only accepts protocol v2 events, so a worker that
    /// answers with a legacy protocol line fails the handshake instead of being
    /// accepted as a fallback.
    pub fn spawn_ready_v2(
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<Self, SupervisorError> {
        Self::spawn_ready_v2_with_env(program, args, &[], timeout)
    }

    pub fn spawn_ready_v2_with_env(
        program: &str,
        args: &[&str],
        env: &[(String, String)],
        timeout: Duration,
    ) -> Result<Self, SupervisorError> {
        let mut supervisor = Self::spawn_with_env(program, args, env)?;
        supervisor.send_v2(&xarchive_protocol::SidecarV2Command::hello(
            "desktop-hello-v2",
        ))?;
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                supervisor.shutdown();
                return Err(SupervisorError::HandshakeTimeout);
            }
            match supervisor.recv_timeout(remaining)? {
                Some(SupervisorEvent::V2(event)) if event.event == SidecarV2EventType::Ready => {
                    event
                        .validate()
                        .map_err(|error| SupervisorError::HandshakeFailed(error.to_string()))?;
                    if event.protocol_version != SIDECAR_PROTOCOL_VERSION
                        || event.job_id != "system"
                        || event.request_id.as_deref() != Some("desktop-hello-v2")
                        || !xarchive_protocol::has_required_capabilities(
                            event.capabilities.as_deref().unwrap_or(&[]),
                        )
                    {
                        supervisor.shutdown();
                        return Err(SupervisorError::HandshakeFailed(
                            "invalid v2 ready identity or capabilities".to_owned(),
                        ));
                    }
                    return Ok(supervisor);
                }
                Some(SupervisorEvent::Exited(result)) => {
                    return Err(SupervisorError::HandshakeFailed(format!(
                        "sidecar exited during v2 handshake: {result:?}"
                    )));
                }
                Some(SupervisorEvent::ProtocolError { message, .. }) => {
                    supervisor.shutdown();
                    return Err(SupervisorError::HandshakeFailed(message));
                }
                Some(_) | None => {}
            }
        }
    }

    pub fn try_recv(&self) -> Result<Option<SupervisorEvent>, SupervisorError> {
        match self.events.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(SupervisorError::NotRunning),
        }
    }

    /// Wait for one event without blocking indefinitely.
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<SupervisorEvent>, SupervisorError> {
        match self.events.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(SupervisorError::NotRunning),
        }
    }

    pub fn is_running(&mut self) -> Result<bool, SupervisorError> {
        Ok(self.poll_exit()?.is_none())
    }

    /// Check whether the child has exited without blocking the Desktop event loop.
    pub fn poll_exit(&mut self) -> Result<Option<std::process::ExitStatus>, SupervisorError> {
        let child = self.child.as_mut().ok_or(SupervisorError::NotRunning)?;
        child.try_wait().map_err(SupervisorError::Spawn)
    }

    pub fn close_stdin(&mut self) {
        self.stdin.take();
    }

    pub fn shutdown(&mut self) {
        self.close_stdin();
        if let Some(mut child) = self.child.take() {
            terminate_process_tree(&mut child);
            let _ = child.wait();
        }
    }

    /// Wait briefly for a protocol shutdown before using the forceful fallback.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<bool, SupervisorError> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if self.poll_exit()?.is_some() {
                self.child.take();
                return Ok(true);
            }
            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn hide_console_window(command: &mut Command) {
    #[cfg(not(target_os = "windows"))]
    let _ = command;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
}

fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // The Sidecar and every child it starts must be isolated from the
        // Desktop process group so cancellation cannot leave gallery-dl or a
        // helper process writing into a released staging directory.
        command.process_group(0);
    }
    #[cfg(not(unix))]
    let _ = command;
}

fn terminate_process_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;

        let process_group = Pid::from_raw(-(child.id() as i32));
        // Best effort: the direct child is still killed below if the group is
        // already gone or the platform refuses the signal.
        let _ = kill(process_group, Signal::SIGTERM);
        thread::sleep(Duration::from_millis(25));
        let _ = kill(process_group, Signal::SIGKILL);
    }
    let _ = child.kill();
}

impl Drop for SidecarSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;

    #[test]
    fn parses_v2_event_from_json() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader(
            "{\"protocol_version\":2,\"event\":\"extraction_started\",\"job_id\":\"job-1\",\"request_id\":\"request-1\"}\n".as_bytes(),
            sender,
        );

        match receiver.recv().expect("v2 event") {
            SupervisorEvent::V2(event) => {
                assert_eq!(event.protocol_version, 2);
                assert_eq!(
                    event.event,
                    xarchive_protocol::SidecarV2EventType::ExtractionStarted
                );
                assert_eq!(event.job_id, "job-1");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn rejects_legacy_protocol_line_as_protocol_error() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader(
            "{\"protocol_version\":1,\"event\":\"ready\",\"job_id\":\"system\"}\n".as_bytes(),
            sender,
        );

        match receiver.recv().expect("protocol error") {
            SupervisorEvent::ProtocolError { line, message } => {
                assert!(message.contains("unsupported sidecar protocol version"));
                assert!(line.contains("\"protocol_version\":1"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn reports_invalid_json_without_stopping_reader() {
        let (sender, receiver) = mpsc::channel();
        let input = concat!(
            "not-json\n",
            "{\"protocol_version\":2,\"event\":\"extraction_started\",\"job_id\":\"job-1\",\"request_id\":\"request-1\"}\n"
        );
        readers::spawn_stdout_reader(input.as_bytes(), sender);

        assert!(matches!(
            receiver.recv().expect("protocol error"),
            SupervisorEvent::ProtocolError { .. }
        ));
        assert!(matches!(
            receiver.recv().expect("v2 event"),
            SupervisorEvent::V2(_)
        ));
    }

    #[test]
    fn reports_missing_protocol_version() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader("{\"event\":\"ready\"}\n".as_bytes(), sender);

        match receiver.recv().expect("protocol error") {
            SupervisorEvent::ProtocolError { message, .. } => {
                assert!(message.contains("missing protocol_version"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn spawn_ready_v2_completes_the_capability_handshake() {
        let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
        let script = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] == 'hello':
        print(json.dumps({'protocol_version': 2, 'event': 'ready', 'job_id': 'system', 'request_id': command['request_id'], 'capabilities': ['extract_media', 'cancel_active_extraction', 'structured_media_plan', 'account_discovery']}), flush=True)
    elif command['cmd'] == 'shutdown':
        break
"#;
        let mut supervisor = match SidecarSupervisor::spawn_ready_v2(
            &python,
            &["-c", script],
            Duration::from_secs(5),
        ) {
            Ok(supervisor) => supervisor,
            Err(SupervisorError::Spawn(_)) => return,
            Err(error) => panic!("unexpected supervisor error: {error}"),
        };

        assert!(supervisor.is_running().expect("sidecar running"));
        supervisor
            .send_v2_shutdown("desktop-shutdown")
            .expect("send shutdown");
        supervisor.close_stdin();
        assert!(
            supervisor
                .wait_for_exit(Duration::from_secs(2))
                .expect("wait for exit")
        );
    }

    #[test]
    fn spawn_ready_v2_rejects_worker_without_required_capabilities() {
        let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
        let script = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] == 'hello':
        print(json.dumps({'protocol_version': 2, 'event': 'ready', 'job_id': 'system', 'request_id': command['request_id'], 'capabilities': ['extract_media']}), flush=True)
    elif command['cmd'] == 'shutdown':
        break
"#;
        match SidecarSupervisor::spawn_ready_v2(&python, &["-c", script], Duration::from_secs(5)) {
            Ok(mut supervisor) => {
                supervisor.shutdown();
                panic!("worker without required capabilities must be rejected");
            }
            Err(SupervisorError::Spawn(_)) => {}
            Err(SupervisorError::HandshakeFailed(message)) => {
                assert!(
                    message.contains("capabilities"),
                    "unexpected message: {message}"
                );
            }
            Err(error) => panic!("unexpected supervisor error: {error}"),
        }
    }
}
