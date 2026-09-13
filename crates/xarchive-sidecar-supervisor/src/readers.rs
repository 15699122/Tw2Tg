use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::Sender;

use xarchive_protocol::DownloadEvent;

use super::SupervisorEvent;

pub fn spawn_stdout_reader(stdout: impl Read + Send + 'static, sender: Sender<SupervisorEvent>) {
    std::thread::spawn(move || {
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

pub fn spawn_stderr_reader(stderr: impl Read + Send + 'static, sender: Sender<SupervisorEvent>) {
    std::thread::spawn(move || {
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
