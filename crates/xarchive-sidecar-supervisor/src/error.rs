use std::io;

#[derive(Debug)]
pub enum SupervisorError {
    Spawn(io::Error),
    NotRunning,
    Send(io::Error),
    HandshakeTimeout,
    HandshakeFailed(String),
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(error) => write!(formatter, "failed to spawn sidecar: {error}"),
            Self::NotRunning => formatter.write_str("sidecar is not running"),
            Self::Send(error) => write!(formatter, "failed to send sidecar command: {error}"),
            Self::HandshakeTimeout => formatter.write_str("sidecar hello handshake timed out"),
            Self::HandshakeFailed(message) => {
                write!(formatter, "sidecar hello handshake failed: {message}")
            }
        }
    }
}

impl std::error::Error for SupervisorError {}
