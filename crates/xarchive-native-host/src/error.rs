use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMessagingError {
    Io(String),
    MessageTooLarge(usize),
    Truncated,
    InvalidUtf8,
    InvalidJson(String),
    ProtocolViolation(String),
}

impl std::fmt::Display for NativeMessagingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "native messaging I/O error: {error}"),
            Self::MessageTooLarge(size) => write!(
                formatter,
                "native messaging payload is too large: {size} bytes"
            ),
            Self::Truncated => formatter.write_str("native messaging payload is truncated"),
            Self::InvalidUtf8 => formatter.write_str("native messaging payload is not UTF-8"),
            Self::InvalidJson(error) => write!(
                formatter,
                "native messaging payload is invalid JSON: {error}"
            ),
            Self::ProtocolViolation(message) => {
                write!(formatter, "native messaging protocol violation: {message}")
            }
        }
    }
}

impl std::error::Error for NativeMessagingError {}

impl From<io::Error> for NativeMessagingError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
