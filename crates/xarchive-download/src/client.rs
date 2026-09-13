use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::rpc::{
    JsonRpcRequest, JsonRpcResponse, add_uri_rpc_request, get_version_rpc_request,
    parse_add_uri_response, parse_status_response, pause_rpc_request, remove_rpc_request,
    tell_status_rpc_request, unpause_rpc_request,
};
use crate::{AddUriRequest, DownloadBackend, DownloadError, TransferId, TransferStatus};

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
