use std::io;

use xarchive_protocol::DownloadEvent;

#[derive(Debug)]
pub enum SupervisorEvent {
    Download(Box<DownloadEvent>),
    Stderr(String),
    ProtocolError { line: String, message: String },
    Exited(io::Result<std::process::ExitStatus>),
}
