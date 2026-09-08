//! Download transport abstractions and aria2 JSON-RPC protocol models.
//!
//! This crate does not start aria2 or perform HTTP requests yet. It keeps the
//! protocol and state mapping testable before adding a process supervisor.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferId(String);

impl TransferId {
    pub fn new(value: impl Into<String>) -> Result<Self, DownloadError> {
        let value = value.into();
        if value.is_empty() {
            return Err(DownloadError::InvalidTransferId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    Waiting,
    Active,
    Paused,
    Complete,
    Error,
    Removed,
}

impl TransferState {
    pub fn from_aria2(value: &str) -> Result<Self, DownloadError> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "complete" => Ok(Self::Complete),
            "error" => Ok(Self::Error),
            "removed" => Ok(Self::Removed),
            other => Err(DownloadError::UnknownState(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferStatus {
    pub id: TransferId,
    pub state: TransferState,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
    pub download_speed: u64,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub files: Vec<TransferFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferFile {
    pub path: String,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub trait DownloadBackend {
    fn add_uri(&self, request: AddUriRequest) -> Result<TransferId, DownloadError>;
    fn status(&self, id: &TransferId) -> Result<TransferStatus, DownloadError>;
    fn pause(&self, id: &TransferId) -> Result<TransferId, DownloadError>;
    fn resume(&self, id: &TransferId) -> Result<TransferId, DownloadError>;
    fn cancel(&self, id: &TransferId) -> Result<TransferId, DownloadError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddUriRequest {
    pub url: String,
    pub directory: String,
    pub filename: String,
    pub headers: Vec<String>,
}

impl AddUriRequest {
    pub fn validate(&self) -> Result<(), DownloadError> {
        if self.url.is_empty() || self.directory.is_empty() || self.filename.is_empty() {
            return Err(DownloadError::InvalidAddUriRequest);
        }
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(DownloadError::UnsupportedUrl);
        }
        if self.filename.contains(['/', '\\']) || self.filename == "." || self.filename == ".." {
            return Err(DownloadError::InvalidFilename);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Vec<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(
        id: impl Into<String>,
        method: impl Into<String>,
        params: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

pub fn add_uri_rpc_request(
    id: impl Into<String>,
    rpc_secret: &str,
    request: &AddUriRequest,
) -> Result<JsonRpcRequest, DownloadError> {
    request.validate()?;
    if rpc_secret.is_empty() {
        return Err(DownloadError::MissingRpcSecret);
    }
    let options = serde_json::json!({
        "dir": request.directory,
        "out": request.filename,
        "header": request.headers,
        "continue": "true"
    });
    Ok(JsonRpcRequest::new(
        id,
        "aria2.addUri",
        vec![
            serde_json::Value::String(format!("token:{rpc_secret}")),
            serde_json::json!([request.url]),
            options,
        ],
    ))
}

pub fn tell_status_rpc_request(
    id: impl Into<String>,
    rpc_secret: &str,
    transfer_id: &TransferId,
) -> Result<JsonRpcRequest, DownloadError> {
    Ok(JsonRpcRequest::new(
        id,
        "aria2.tellStatus",
        authenticated_params(rpc_secret, serde_json::json!(transfer_id.as_str()))?,
    ))
}

pub fn pause_rpc_request(
    id: impl Into<String>,
    rpc_secret: &str,
    transfer_id: &TransferId,
) -> Result<JsonRpcRequest, DownloadError> {
    Ok(JsonRpcRequest::new(
        id,
        "aria2.pause",
        authenticated_params(rpc_secret, serde_json::json!(transfer_id.as_str()))?,
    ))
}

pub fn unpause_rpc_request(
    id: impl Into<String>,
    rpc_secret: &str,
    transfer_id: &TransferId,
) -> Result<JsonRpcRequest, DownloadError> {
    Ok(JsonRpcRequest::new(
        id,
        "aria2.unpause",
        authenticated_params(rpc_secret, serde_json::json!(transfer_id.as_str()))?,
    ))
}

pub fn remove_rpc_request(
    id: impl Into<String>,
    rpc_secret: &str,
    transfer_id: &TransferId,
) -> Result<JsonRpcRequest, DownloadError> {
    Ok(JsonRpcRequest::new(
        id,
        "aria2.remove",
        authenticated_params(rpc_secret, serde_json::json!(transfer_id.as_str()))?,
    ))
}

pub fn get_version_rpc_request(
    id: impl Into<String>,
    rpc_secret: &str,
) -> Result<JsonRpcRequest, DownloadError> {
    Ok(JsonRpcRequest::new(
        id,
        "aria2.getVersion",
        vec![serde_json::Value::String(format!("token:{rpc_secret}"))],
    ))
}

fn authenticated_params(
    rpc_secret: &str,
    value: serde_json::Value,
) -> Result<Vec<serde_json::Value>, DownloadError> {
    if rpc_secret.is_empty() {
        return Err(DownloadError::MissingRpcSecret);
    }
    Ok(vec![
        serde_json::Value::String(format!("token:{rpc_secret}")),
        value,
    ])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: String,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

pub fn parse_add_uri_response(response: JsonRpcResponse) -> Result<TransferId, DownloadError> {
    if let Some(error) = response.error {
        return Err(DownloadError::Rpc(error));
    }
    let result = response.result.ok_or(DownloadError::MissingRpcResult)?;
    let id = result.as_str().ok_or(DownloadError::InvalidRpcResult)?;
    TransferId::new(id)
}

pub fn parse_status_response(response: JsonRpcResponse) -> Result<TransferStatus, DownloadError> {
    if let Some(error) = response.error {
        return Err(DownloadError::Rpc(error));
    }
    let result = response.result.ok_or(DownloadError::MissingRpcResult)?;
    let raw: Aria2Status =
        serde_json::from_value(result).map_err(|_| DownloadError::InvalidRpcResult)?;
    let id = TransferId::new(raw.gid)?;
    let state = TransferState::from_aria2(&raw.status)?;
    let files = raw
        .files
        .into_iter()
        .map(|file| {
            Ok(TransferFile {
                path: file.path,
                completed_bytes: parse_byte_count(&file.completed_length)?,
                total_bytes: parse_optional_byte_count(&file.length)?,
            })
        })
        .collect::<Result<Vec<_>, DownloadError>>()?;
    Ok(TransferStatus {
        id,
        state,
        completed_bytes: parse_byte_count(&raw.completed_length)?,
        total_bytes: parse_optional_byte_count(&raw.total_length)?,
        download_speed: parse_byte_count(&raw.download_speed)?,
        error_code: raw.error_code,
        error_message: raw.error_message,
        files,
    })
}

#[derive(Debug, Deserialize)]
struct Aria2Status {
    gid: String,
    status: String,
    #[serde(rename = "completedLength")]
    completed_length: String,
    #[serde(rename = "totalLength")]
    total_length: String,
    #[serde(rename = "downloadSpeed")]
    download_speed: String,
    #[serde(default)]
    #[serde(rename = "errorCode")]
    error_code: Option<String>,
    #[serde(default)]
    #[serde(rename = "errorMessage")]
    error_message: Option<String>,
    #[serde(default)]
    files: Vec<Aria2File>,
}

#[derive(Debug, Deserialize)]
struct Aria2File {
    path: String,
    #[serde(rename = "completedLength")]
    completed_length: String,
    length: String,
}

fn parse_byte_count(value: &str) -> Result<u64, DownloadError> {
    value
        .parse::<u64>()
        .map_err(|_| DownloadError::InvalidByteCount(value.into()))
}

fn parse_optional_byte_count(value: &str) -> Result<Option<u64>, DownloadError> {
    if value == "0" {
        Ok(None)
    } else {
        Ok(Some(parse_byte_count(value)?))
    }
}

#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_authenticated_add_uri_request() {
        let request = AddUriRequest {
            url: "https://cdn.example/video.mp4".into(),
            directory: "/tmp/staging/job-1".into(),
            filename: "01.mp4".into(),
            headers: vec!["Referer: https://x.com/".into()],
        };
        let rpc = add_uri_rpc_request("1", "secret", &request).expect("RPC request");
        assert_eq!(rpc.method, "aria2.addUri");
        assert_eq!(rpc.params[0], "token:secret");
        assert_eq!(rpc.params[2]["out"], "01.mp4");
    }

    #[test]
    fn rejects_unsafe_filename_and_missing_secret() {
        let request = AddUriRequest {
            url: "https://cdn.example/file.jpg".into(),
            directory: "/tmp/staging".into(),
            filename: "../escape.jpg".into(),
            headers: vec![],
        };
        assert!(matches!(
            add_uri_rpc_request("1", "secret", &request),
            Err(DownloadError::InvalidFilename)
        ));
        let valid = AddUriRequest {
            filename: "01.jpg".into(),
            ..request
        };
        assert!(matches!(
            add_uri_rpc_request("1", "", &valid),
            Err(DownloadError::MissingRpcSecret)
        ));
    }

    #[test]
    fn maps_status_and_parses_byte_counts() {
        let response: JsonRpcResponse = serde_json::from_str(include_str!(
            "../../../shared/protocol-schema/fixtures/aria2-status-response.json"
        ))
        .expect("fixture");
        let status = parse_status_response(response).expect("status");
        assert_eq!(status.id.as_str(), "0123456789abcdef");
        assert_eq!(status.state, TransferState::Active);
        assert_eq!(status.completed_bytes, 2048);
        assert_eq!(status.total_bytes, Some(4096));
        assert_eq!(status.files[0].path, "/tmp/staging/job-1/01.jpg");
    }

    #[test]
    fn maps_rpc_error() {
        let response: JsonRpcResponse = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":"1","error":{"code":1,"message":"unauthorized"}}"#,
        )
        .expect("response");
        assert!(matches!(
            parse_add_uri_response(response),
            Err(DownloadError::Rpc(_))
        ));
    }

    #[test]
    fn builds_authenticated_control_requests() {
        let transfer_id = TransferId::new("0123456789abcdef").expect("transfer ID");
        let status = tell_status_rpc_request("1", "secret", &transfer_id).expect("status");
        assert_eq!(status.method, "aria2.tellStatus");
        assert_eq!(status.params[0], "token:secret");
        assert_eq!(status.params[1], "0123456789abcdef");

        let version = get_version_rpc_request("2", "secret").expect("version");
        assert_eq!(version.method, "aria2.getVersion");
        assert_eq!(version.params, vec![serde_json::json!("token:secret")]);
    }
}
