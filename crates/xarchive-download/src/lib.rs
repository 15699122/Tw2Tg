//! Download transport abstractions, aria2 JSON-RPC models, and a small
//! dependency-free HTTP client for a loopback aria2 endpoint.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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

#[derive(Clone)]
pub struct Aria2HttpClient {
    host: String,
    port: u16,
    rpc_secret: String,
    timeout: Duration,
}

impl std::fmt::Debug for Aria2HttpClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Aria2HttpClient")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("rpc_secret", &"[REDACTED]")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Aria2HttpClient {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        rpc_secret: impl Into<String>,
    ) -> Result<Self, DownloadError> {
        let host = host.into();
        let rpc_secret = rpc_secret.into();
        if host.is_empty() || rpc_secret.is_empty() {
            return Err(DownloadError::InvalidClientConfiguration);
        }
        Ok(Self {
            host,
            port,
            rpc_secret,
            timeout: Duration::from_secs(15),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn call(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, DownloadError> {
        let address = format!("{}:{}", self.host, self.port);
        let mut stream = address
            .to_socket_addrs()
            .map_err(|error| DownloadError::Http(error.to_string()))?
            .next()
            .ok_or_else(|| DownloadError::Http("aria2 endpoint has no address".to_owned()))
            .and_then(|address| {
                TcpStream::connect_timeout(&address, self.timeout)
                    .map_err(|error| DownloadError::Http(error.to_string()))
            })?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|error| DownloadError::Http(error.to_string()))?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|error| DownloadError::Http(error.to_string()))?;
        let body =
            serde_json::to_vec(&request).map_err(|error| DownloadError::Http(error.to_string()))?;
        write!(stream, "POST /jsonrpc HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", self.host, body.len())
            .map_err(|error| DownloadError::Http(error.to_string()))?;
        stream
            .write_all(&body)
            .map_err(|error| DownloadError::Http(error.to_string()))?;
        stream
            .flush()
            .map_err(|error| DownloadError::Http(error.to_string()))?;
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|error| DownloadError::Http(error.to_string()))?;
        let separator = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| DownloadError::Http("invalid HTTP response".to_owned()))?;
        let headers = String::from_utf8_lossy(&response[..separator]);
        let status = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| DownloadError::Http("invalid HTTP status".to_owned()))?;
        if status != 200 {
            return Err(DownloadError::HttpStatus(status));
        }
        serde_json::from_slice(&response[separator + 4..])
            .map_err(|error| DownloadError::Http(error.to_string()))
    }

    pub fn get_version(&self) -> Result<String, DownloadError> {
        let response = self.call(get_version_rpc_request("version", &self.rpc_secret)?)?;
        if let Some(error) = response.error {
            return Err(DownloadError::Rpc(error));
        }
        response
            .result
            .and_then(|value| {
                value
                    .get("version")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or(DownloadError::InvalidRpcResult)
    }
}

impl DownloadBackend for Aria2HttpClient {
    fn add_uri(&self, request: AddUriRequest) -> Result<TransferId, DownloadError> {
        parse_add_uri_response(self.call(add_uri_rpc_request(
            "add-uri",
            &self.rpc_secret,
            &request,
        )?)?)
    }

    fn status(&self, id: &TransferId) -> Result<TransferStatus, DownloadError> {
        parse_status_response(self.call(tell_status_rpc_request(
            "status",
            &self.rpc_secret,
            id,
        )?)?)
    }

    fn pause(&self, id: &TransferId) -> Result<TransferId, DownloadError> {
        parse_add_uri_response(self.call(pause_rpc_request("pause", &self.rpc_secret, id)?)?)
    }

    fn resume(&self, id: &TransferId) -> Result<TransferId, DownloadError> {
        parse_add_uri_response(self.call(unpause_rpc_request("resume", &self.rpc_secret, id)?)?)
    }

    fn cancel(&self, id: &TransferId) -> Result<TransferId, DownloadError> {
        parse_add_uri_response(self.call(remove_rpc_request("cancel", &self.rpc_secret, id)?)?)
    }
}

#[derive(Clone)]
pub struct Aria2SupervisorConfig {
    pub program: String,
    pub host: String,
    pub port: u16,
    pub rpc_secret: String,
    pub startup_timeout: Duration,
    pub request_timeout: Duration,
}

impl std::fmt::Debug for Aria2SupervisorConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Aria2SupervisorConfig")
            .field("program", &self.program)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("rpc_secret", &"[REDACTED]")
            .field("startup_timeout", &self.startup_timeout)
            .field("request_timeout", &self.request_timeout)
            .finish()
    }
}

impl Aria2SupervisorConfig {
    pub fn new(
        program: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        rpc_secret: impl Into<String>,
    ) -> Result<Self, DownloadError> {
        let config = Self {
            program: program.into(),
            host: host.into(),
            port,
            rpc_secret: rpc_secret.into(),
            startup_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(15),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    fn validate(&self) -> Result<(), DownloadError> {
        if self.program.trim().is_empty()
            || self.host.trim().is_empty()
            || self.rpc_secret.is_empty()
            || self.port == 0
            || self.startup_timeout.is_zero()
            || self.request_timeout.is_zero()
        {
            return Err(DownloadError::InvalidSupervisorConfiguration);
        }
        Ok(())
    }

    fn command_args(&self) -> Vec<String> {
        vec![
            "--enable-rpc=true".into(),
            "--rpc-listen-all=false".into(),
            format!("--rpc-listen-port={}", self.port),
            format!("--rpc-secret={}", self.rpc_secret),
            "--quiet=true".into(),
        ]
    }
}

pub struct Aria2Supervisor {
    child: Option<Child>,
    client: Aria2HttpClient,
}

impl std::fmt::Debug for Aria2Supervisor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Aria2Supervisor")
            .field("running", &self.child.is_some())
            .field("client", &self.client)
            .finish()
    }
}

impl Aria2Supervisor {
    pub fn spawn(config: Aria2SupervisorConfig) -> Result<Self, DownloadError> {
        config.validate()?;
        let client = Aria2HttpClient::new(&config.host, config.port, &config.rpc_secret)?
            .with_timeout(config.request_timeout);
        let mut child = Command::new(&config.program)
            .args(config.command_args())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| DownloadError::Process(error.to_string()))?;
        if let Err(error) = wait_for_aria2(&mut child, &client, config.startup_timeout) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(Self {
            child: Some(child),
            client,
        })
    }

    pub fn backend(&self) -> &Aria2HttpClient {
        &self.client
    }

    pub fn get_version(&self) -> Result<String, DownloadError> {
        self.client.get_version()
    }

    pub fn is_running(&mut self) -> Result<bool, DownloadError> {
        let child = self
            .child
            .as_mut()
            .ok_or(DownloadError::SupervisorNotRunning)?;
        child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|error| DownloadError::Process(error.to_string()))
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<bool, DownloadError> {
        let deadline = Instant::now() + timeout;
        loop {
            let child = self
                .child
                .as_mut()
                .ok_or(DownloadError::SupervisorNotRunning)?;
            if child
                .try_wait()
                .map_err(|error| DownloadError::Process(error.to_string()))?
                .is_some()
            {
                self.child.take();
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Aria2Supervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn wait_for_aria2(
    child: &mut Child,
    client: &Aria2HttpClient,
    timeout: Duration,
) -> Result<(), DownloadError> {
    let deadline = Instant::now() + timeout;
    loop {
        if child
            .try_wait()
            .map_err(|error| DownloadError::Process(error.to_string()))?
            .is_some()
        {
            return Err(DownloadError::Process(
                "aria2 exited before its RPC endpoint became ready".to_owned(),
            ));
        }
        match client.get_version() {
            Ok(_) => return Ok(()),
            Err(_error) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => {
                return Err(DownloadError::StartupTimeout {
                    last_error: error.to_string(),
                });
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

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

    fn fake_server(response: String) -> (u16, thread::JoinHandle<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fake aria2 server");
        let port = listener.local_addr().expect("server address").port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept client");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            let body_length = loop {
                let count = stream.read(&mut buffer).expect("read request");
                if count == 0 {
                    break 0;
                }
                request.extend_from_slice(&buffer[..count]);
                if let Some(separator) = request.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&request[..separator]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            (name.eq_ignore_ascii_case("content-length"))
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    let body_start = separator + 4;
                    while request.len() - body_start < length {
                        let count = stream.read(&mut buffer).expect("read request body");
                        if count == 0 {
                            break;
                        }
                        request.extend_from_slice(&buffer[..count]);
                    }
                    break body_start;
                }
            };
            stream
                .write_all(response.as_bytes())
                .expect("write fake response");
            String::from_utf8(request[body_length..].to_vec()).expect("JSON request")
        });
        (port, handle)
    }

    #[test]
    fn sends_add_uri_over_loopback_http() {
        let body = r#"{"jsonrpc":"2.0","id":"add-uri","result":"gid-1"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let (port, server) = fake_server(response);
        let client = Aria2HttpClient::new("127.0.0.1", port, "secret").expect("client");
        let transfer = client
            .add_uri(AddUriRequest {
                url: "https://cdn.example/file.jpg".into(),
                directory: "/tmp/staging/job-1".into(),
                filename: "01.jpg".into(),
                headers: vec![],
            })
            .expect("add URI");
        let request: JsonRpcRequest =
            serde_json::from_str(&server.join().expect("server thread")).expect("captured request");
        assert_eq!(transfer.as_str(), "gid-1");
        assert_eq!(request.method, "aria2.addUri");
        assert_eq!(request.params[0], "token:secret");
    }

    #[test]
    fn parses_status_from_loopback_http() {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            include_str!("../../../shared/protocol-schema/fixtures/aria2-status-response.json")
                .len(),
            include_str!("../../../shared/protocol-schema/fixtures/aria2-status-response.json")
        );
        let (port, server) = fake_server(response);
        let client = Aria2HttpClient::new("127.0.0.1", port, "secret").expect("client");
        let transfer_id = TransferId::new("0123456789abcdef").expect("transfer ID");
        let status = client.status(&transfer_id).expect("status");
        let request: JsonRpcRequest =
            serde_json::from_str(&server.join().expect("server thread")).expect("captured request");
        assert_eq!(status.state, TransferState::Active);
        assert_eq!(status.completed_bytes, 2048);
        assert_eq!(request.method, "aria2.tellStatus");
    }

    #[test]
    fn maps_http_and_rpc_failures() {
        let (port, server) =
            fake_server("HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n".to_owned());
        let client = Aria2HttpClient::new("127.0.0.1", port, "secret").expect("client");
        let error = client.get_version().expect_err("HTTP failure");
        assert!(matches!(error, DownloadError::HttpStatus(503)));
        server.join().expect("server thread");

        let body =
            r#"{"jsonrpc":"2.0","id":"version","error":{"code":1,"message":"unauthorized"}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let (port, server) = fake_server(response);
        let client = Aria2HttpClient::new("127.0.0.1", port, "secret").expect("client");
        let error = client.get_version().expect_err("RPC failure");
        assert!(matches!(
            error,
            DownloadError::Rpc(JsonRpcError { code: 1, .. })
        ));
        server.join().expect("server thread");
    }

    #[test]
    fn validates_supervisor_configuration_and_redacts_secret() {
        let config = Aria2SupervisorConfig::new("aria2c", "127.0.0.1", 6800, "rpc-secret")
            .expect("configuration");
        assert_eq!(
            config.command_args(),
            vec![
                "--enable-rpc=true",
                "--rpc-listen-all=false",
                "--rpc-listen-port=6800",
                "--rpc-secret=rpc-secret",
                "--quiet=true",
            ]
        );
        assert!(!format!("{config:?}").contains("rpc-secret"));

        assert!(matches!(
            Aria2SupervisorConfig::new("", "127.0.0.1", 6800, "rpc-secret"),
            Err(DownloadError::InvalidSupervisorConfiguration)
        ));
        assert!(matches!(
            Aria2SupervisorConfig::new("aria2c", "127.0.0.1", 0, "rpc-secret"),
            Err(DownloadError::InvalidSupervisorConfiguration)
        ));
    }

    #[test]
    fn maps_process_spawn_failure_without_exposing_secret() {
        let config = Aria2SupervisorConfig::new(
            "/definitely/missing/aria2c",
            "127.0.0.1",
            6800,
            "rpc-secret",
        )
        .expect("configuration");
        let error = Aria2Supervisor::spawn(config).expect_err("missing process");
        assert!(matches!(error, DownloadError::Process(_)));
        assert!(!error.to_string().contains("rpc-secret"));
    }
}
