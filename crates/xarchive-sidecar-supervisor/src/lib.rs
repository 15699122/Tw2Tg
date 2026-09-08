//! Process supervision for the Python JSONL Sidecar.
//!
//! This crate intentionally owns only process and stream mechanics. Archive
//! state, retries, persistence, and the decision that a job is complete stay
//! in the Desktop application.

use std::io::{self, BufRead, BufReader};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use xarchive_protocol::{DownloadEvent, SidecarCommand, write_json_line};

#[derive(Debug)]
pub enum SupervisorEvent {
    Download(Box<DownloadEvent>),
    Stderr(String),
    ProtocolError { line: String, message: String },
    Exited(io::Result<std::process::ExitStatus>),
}

#[derive(Debug)]
pub enum SupervisorError {
    Spawn(io::Error),
    NotRunning,
    Send(io::Error),
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(error) => write!(formatter, "failed to spawn sidecar: {error}"),
            Self::NotRunning => formatter.write_str("sidecar is not running"),
            Self::Send(error) => write!(formatter, "failed to send sidecar command: {error}"),
        }
    }
}

impl std::error::Error for SupervisorError {}

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
        spawn_stdout_reader(stdout, sender.clone());
        spawn_stderr_reader(stderr, sender);

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
}

impl Drop for SidecarSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_stdout_reader(
    stdout: impl std::io::Read + Send + 'static,
    sender: Sender<SupervisorEvent>,
) {
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => match serde_json::from_str::<DownloadEvent>(&line) {
                    Ok(event) => {
                        if sender
                            .send(SupervisorEvent::Download(Box::new(event)))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        if sender
                            .send(SupervisorEvent::ProtocolError {
                                line,
                                message: error.to_string(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                },
                Err(error) => {
                    let _ = sender.send(SupervisorEvent::ProtocolError {
                        line: String::new(),
                        message: error.to_string(),
                    });
                    break;
                }
            }
        }
    });
}

fn spawn_stderr_reader(
    stderr: impl std::io::Read + Send + 'static,
    sender: Sender<SupervisorEvent>,
) {
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            match line {
                Ok(line) => {
                    if sender.send(SupervisorEvent::Stderr(line)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(SupervisorEvent::Stderr(error.to_string()));
                    break;
                }
            }
        }
    });
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
        spawn_stdout_reader(
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
        spawn_stdout_reader(
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
}
