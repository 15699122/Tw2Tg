use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::Sender;

use xarchive_protocol::{SIDECAR_PROTOCOL_VERSION, SidecarV2Event};

use super::SupervisorEvent;

pub fn spawn_stdout_reader(stdout: impl Read + Send + 'static, sender: Sender<SupervisorEvent>) {
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    let decoded = decode_stdout_line(&line);
                    match decoded {
                        Ok(event) => {
                            if sender.send(event).is_err() {
                                break;
                            }
                        }
                        Err(error) => {
                            if sender
                                .send(SupervisorEvent::ProtocolError {
                                    line,
                                    message: error,
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
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

/// Decode one Sidecar stdout line as a protocol v2 event.
///
/// The supervisor only accepts the current protocol. A line that advertises any
/// other version, or that is not a valid v2 event, is reported as a protocol
/// error instead of being coerced into another shape.
fn decode_stdout_line(line: &str) -> Result<SupervisorEvent, String> {
    let value: serde_json::Value = serde_json::from_str(line).map_err(|error| error.to_string())?;
    match value
        .get("protocol_version")
        .and_then(|value| value.as_u64())
    {
        Some(version) if version == u64::from(SIDECAR_PROTOCOL_VERSION) => {}
        Some(version) => {
            return Err(format!(
                "unsupported sidecar protocol version in stdout event: {version}"
            ));
        }
        None => return Err("sidecar event is missing protocol_version".to_owned()),
    }
    serde_json::from_value::<SidecarV2Event>(value)
        .map(|event| SupervisorEvent::V2(Box::new(event)))
        .map_err(|error| error.to_string())
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
