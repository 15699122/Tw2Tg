use std::io;

use xarchive_protocol::SidecarV2Event;

#[derive(Debug)]
pub enum SupervisorEvent {
    V2(Box<SidecarV2Event>),
    Stderr(String),
    ProtocolError { line: String, message: String },
    Exited(io::Result<std::process::ExitStatus>),
}
