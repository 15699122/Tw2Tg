use crate::rpc::JsonRpcError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    InvalidTransferId,
    InvalidAddUriRequest,
    UnsupportedUrl,
    InvalidFilename,
    MissingRpcSecret,
    UnknownState(String),
    MissingRpcResult,
    InvalidRpcResult,
    InvalidByteCount(String),
    Rpc(JsonRpcError),
    InvalidClientConfiguration,
    InvalidSupervisorConfiguration,
    Process(String),
    SupervisorNotRunning,
    StartupTimeout { last_error: String },
    Http(String),
    HttpStatus(u16),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransferId => formatter.write_str("transfer ID must not be empty"),
            Self::InvalidAddUriRequest => formatter.write_str("addUri request is incomplete"),
            Self::UnsupportedUrl => formatter.write_str("only HTTP(S) URLs are supported"),
            Self::InvalidFilename => {
                formatter.write_str("filename must be a single safe path component")
            }
            Self::MissingRpcSecret => formatter.write_str("aria2 RPC secret is required"),
            Self::UnknownState(value) => write!(formatter, "unknown aria2 state: {value}"),
            Self::MissingRpcResult => formatter.write_str("aria2 response has no result"),
            Self::InvalidRpcResult => formatter.write_str("aria2 response result is invalid"),
            Self::InvalidByteCount(value) => write!(formatter, "invalid byte count: {value}"),
            Self::Rpc(error) => write!(
                formatter,
                "aria2 RPC error {}: {}",
                error.code, error.message
            ),
            Self::InvalidClientConfiguration => {
                formatter.write_str("aria2 client configuration is invalid")
            }
            Self::InvalidSupervisorConfiguration => {
                formatter.write_str("aria2 supervisor configuration is invalid")
            }
            Self::Process(error) => write!(formatter, "aria2 process error: {error}"),
            Self::SupervisorNotRunning => formatter.write_str("aria2 supervisor is not running"),
            Self::StartupTimeout { last_error } => {
                write!(formatter, "aria2 startup timed out: {last_error}")
            }
            Self::Http(error) => write!(formatter, "aria2 HTTP error: {error}"),
            Self::HttpStatus(status) => write!(formatter, "aria2 HTTP status: {status}"),
        }
    }
}

impl std::error::Error for DownloadError {}

impl From<JsonRpcError> for DownloadError {
    fn from(value: JsonRpcError) -> Self {
        Self::Rpc(value)
    }
}

impl From<DownloadError> for String {
    fn from(value: DownloadError) -> Self {
        value.to_string()
    }
}
