use serde::{Deserialize, Serialize};

use crate::{DownloadError, TransferFile, TransferId, TransferState, TransferStatus};

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
        .map(|file| -> Result<TransferFile, DownloadError> {
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
