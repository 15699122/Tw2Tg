//! Process supervision for the Python JSONL Sidecar.
//!
//! This crate intentionally owns only process and stream mechanics. Archive
//! state, retries, persistence, and the decision that a job is complete stay
//! in the Desktop application.

use std::io::{self, BufRead, BufReader};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use xarchive_protocol::{DownloadEvent, SidecarCommand, write_json_line};

#[derive(Debug)]
pub enum SupervisorEvent {
    Download(DownloadEvent),
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
                        if sender.send(SupervisorEvent::Download(event)).is_err() {
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
}
