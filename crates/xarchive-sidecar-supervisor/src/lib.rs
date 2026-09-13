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
    DownloadEventType, PROTOCOL_VERSION, SidecarCommand, SidecarCommandType, write_json_line,
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
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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

    pub fn send(&mut self, command: &SidecarCommand) -> Result<(), SupervisorError> {
        let stdin = self.stdin.as_mut().ok_or(SupervisorError::NotRunning)?;
        write_json_line(stdin, command).map_err(SupervisorError::Send)
    }

    /// Spawn a sidecar and require the protocol `hello → ready` handshake.
    pub fn spawn_ready(
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<Self, SupervisorError> {
        let mut supervisor = Self::spawn(program, args)?;
        let hello = SidecarCommand {
            protocol_version: PROTOCOL_VERSION,
            request_id: "desktop-hello".to_owned(),
            cmd: SidecarCommandType::Hello,
            job_id: "system".to_owned(),
            url: None,
            staging_dir: None,
            browser: None,
            profile: None,
        };
        supervisor.send(&hello)?;

        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                supervisor.shutdown();
                return Err(SupervisorError::HandshakeTimeout);
            }
            match supervisor.recv_timeout(remaining)? {
                Some(SupervisorEvent::Download(event))
                    if event.event == DownloadEventType::Ready =>
                {
                    return Ok(supervisor);
                }
                Some(SupervisorEvent::Download(event))
                    if event.event == DownloadEventType::Failed =>
                {
                    let message = event
                        .error_message
                        .unwrap_or_else(|| "sidecar rejected hello".to_owned());
                    supervisor.shutdown();
                    return Err(SupervisorError::HandshakeFailed(message));
                }
                Some(SupervisorEvent::Exited(result)) => {
                    supervisor.shutdown();
                    return Err(SupervisorError::HandshakeFailed(format!(
                        "sidecar exited: {result:?}"
                    )));
                }
                Some(SupervisorEvent::ProtocolError { message, .. }) => {
                    supervisor.shutdown();
                    return Err(SupervisorError::HandshakeFailed(message));
                }
                Some(SupervisorEvent::Stderr(_)) | Some(SupervisorEvent::Download(_)) => {}
                None => {}
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
            let _ = child.kill();
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
    use xarchive_protocol::{DownloadEventType, PROTOCOL_VERSION};

    #[test]
    fn parses_download_event_from_json() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader(
            "{\"protocol_version\":1,\"event\":\"started\",\"job_id\":\"job-1\"}\n".as_bytes(),
            sender,
        );

        match receiver.recv().expect("event") {
            SupervisorEvent::Download(event) => {
                assert_eq!(event.protocol_version, PROTOCOL_VERSION);
                assert_eq!(event.event, DownloadEventType::Started);
                assert_eq!(event.job_id, "job-1");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn reports_invalid_json_without_stopping_reader() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader(
            "not-json\n{\"protocol_version\":1,\"event\":\"complete\",\"job_id\":\"job-1\"}\n"
                .as_bytes(),
            sender,
        );

        assert!(matches!(
            receiver.recv().expect("protocol error"),
            SupervisorEvent::ProtocolError { .. }
        ));
        assert!(matches!(
            receiver.recv().expect("download event"),
            SupervisorEvent::Download(_)
        ));
    }

    #[test]
    fn communicates_with_a_real_python_worker_when_available() {
        let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
        let script = r#"
import sys
for line in sys.stdin:
    command = __import__('json').loads(line)
    if command['cmd'] == 'hello':
        print(__import__('json').dumps({'protocol_version': 1, 'event': 'ready', 'job_id': command['job_id'], 'request_id': command['request_id']}), flush=True)
    elif command['cmd'] == 'shutdown':
        break
    else:
        print(__import__('json').dumps({'protocol_version': 1, 'event': 'started', 'job_id': command['job_id'], 'request_id': command['request_id']}), flush=True)
        print(__import__('json').dumps({'protocol_version': 1, 'event': 'complete', 'job_id': command['job_id'], 'request_id': command['request_id'], 'files': []}), flush=True)
"#;
        let mut supervisor = match SidecarSupervisor::spawn(&python, &["-c", script]) {
            Ok(supervisor) => supervisor,
            Err(SupervisorError::Spawn(_)) => return,
            Err(error) => panic!("unexpected supervisor error: {error}"),
        };

        let hello = SidecarCommand {
            protocol_version: PROTOCOL_VERSION,
            request_id: "request-1".into(),
            cmd: xarchive_protocol::SidecarCommandType::Hello,
            job_id: "system".into(),
            url: None,
            staging_dir: None,
            browser: None,
            profile: None,
        };
        supervisor.send(&hello).expect("send hello");
        assert!(matches!(
            supervisor.recv_timeout(Duration::from_secs(2)).expect("ready event"),
            Some(SupervisorEvent::Download(event)) if event.event == DownloadEventType::Ready
        ));

        let download = SidecarCommand {
            protocol_version: PROTOCOL_VERSION,
            request_id: "request-2".into(),
            cmd: xarchive_protocol::SidecarCommandType::Download,
            job_id: "job-1".into(),
            url: Some("https://example.invalid/status/1".into()),
            staging_dir: Some("/tmp/job-1".into()),
            browser: None,
            profile: None,
        };
        supervisor.send(&download).expect("send download");
        assert!(matches!(
            supervisor.recv_timeout(Duration::from_secs(2)).expect("started event"),
            Some(SupervisorEvent::Download(event)) if event.event == DownloadEventType::Started
        ));
        assert!(matches!(
            supervisor.recv_timeout(Duration::from_secs(2)).expect("complete event"),
            Some(SupervisorEvent::Download(event)) if event.event == DownloadEventType::Complete
        ));

        let shutdown = SidecarCommand {
            protocol_version: PROTOCOL_VERSION,
            request_id: "request-3".into(),
            cmd: xarchive_protocol::SidecarCommandType::Shutdown,
            job_id: "system".into(),
            url: None,
            staging_dir: None,
            browser: None,
            profile: None,
        };
        supervisor.send(&shutdown).expect("send shutdown");
        for _ in 0..20 {
            if !supervisor.is_running().expect("poll sidecar") {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("python worker did not exit after shutdown");
    }

    #[test]
    fn spawn_ready_completes_the_hello_handshake() {
        let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
        let script = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] == 'hello':
        print(json.dumps({'protocol_version': 1, 'event': 'ready', 'job_id': 'system', 'request_id': command['request_id']}), flush=True)
    elif command['cmd'] == 'shutdown':
        break
"#;
        let mut supervisor =
            SidecarSupervisor::spawn_ready(&python, &["-c", script], Duration::from_secs(2))
                .expect("hello handshake");
        assert!(supervisor.is_running().expect("sidecar running"));
        supervisor.shutdown();
    }
}
