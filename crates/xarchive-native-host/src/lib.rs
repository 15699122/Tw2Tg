//! Cross-platform Chromium Native Messaging framing.
//!
//! The Native Host owns browser framing and the request/response forwarding
//! boundary. The configured endpoint is opened by the binary; on Windows it is
//! expected to be a Named Pipe path such as `\\.\pipe\xarchive-v1`.

use std::io::{self, Read, Write};
use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
pub const PIPE_ENDPOINT_ENV: &str = "XARCHIVE_PIPE_ENDPOINT";

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

pub fn read_payload<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, NativeMessagingError> {
    let mut length_bytes = [0_u8; 4];
    let mut read = 0;
    while read < length_bytes.len() {
        let count = reader.read(&mut length_bytes[read..])?;
        if count == 0 {
            if read == 0 {
                return Ok(None);
            }
            return Err(NativeMessagingError::Truncated);
        }
        read += count;
    }
    let length = u32::from_le_bytes(length_bytes) as usize;
    if length > MAX_MESSAGE_BYTES {
        return Err(NativeMessagingError::MessageTooLarge(length));
    }
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            NativeMessagingError::Truncated
        } else {
            NativeMessagingError::Io(error.to_string())
        }
    })?;
    Ok(Some(payload))
}

pub fn write_payload<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), NativeMessagingError> {
    if payload.len() > MAX_MESSAGE_BYTES {
        return Err(NativeMessagingError::MessageTooLarge(payload.len()));
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| NativeMessagingError::MessageTooLarge(payload.len()))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

pub fn read_json<R: Read, T: for<'de> serde::Deserialize<'de>>(
    reader: &mut R,
) -> Result<Option<T>, NativeMessagingError> {
    let Some(payload) = read_payload(reader)? else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&payload).map_err(|_| NativeMessagingError::InvalidUtf8)?;
    serde_json::from_str(text)
        .map(Some)
        .map_err(|error| NativeMessagingError::InvalidJson(error.to_string()))
}

pub fn write_json<W: Write, T: serde::Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), NativeMessagingError> {
    let payload = serde_json::to_vec(value)
        .map_err(|error| NativeMessagingError::InvalidJson(error.to_string()))?;
    write_payload(writer, &payload)
}

/// Forward one validated browser request over an already-open transport.
///
/// Keeping transport opening outside this function makes the protocol flow
/// testable on Linux and leaves Windows-specific Named Pipe ACL/connection
/// policy in the executable boundary.
pub fn forward_request<T: Read + Write>(
    transport: &mut T,
    request: BrowserRequest,
) -> Result<BrowserResponse, NativeMessagingError> {
    let expected_request_id = request_id(&request).ok_or_else(|| {
        NativeMessagingError::ProtocolViolation("request has no request_id".to_owned())
    })?;
    request
        .validate()
        .map_err(|error| NativeMessagingError::InvalidJson(error.to_string()))?;
    write_json(transport, &request)?;
    let response: BrowserResponse = read_json(transport)?
        .ok_or_else(|| NativeMessagingError::Io("transport closed".to_owned()))?;
    validate_response(&response, &expected_request_id)?;
    Ok(response)
}

fn validate_response(
    response: &BrowserResponse,
    expected_request_id: &str,
) -> Result<(), NativeMessagingError> {
    match response {
        BrowserResponse::ArchiveStatus {
            protocol_version,
            request_id,
            ..
        } => {
            if *protocol_version != PROTOCOL_VERSION {
                return Err(NativeMessagingError::ProtocolViolation(format!(
                    "unsupported response protocol version: {protocol_version}"
                )));
            }
            if request_id != expected_request_id {
                return Err(NativeMessagingError::ProtocolViolation(
                    "response request_id does not match request".to_owned(),
                ));
            }
        }
        BrowserResponse::Error {
            protocol_version,
            request_id,
            ..
        } => {
            if *protocol_version != PROTOCOL_VERSION {
                return Err(NativeMessagingError::ProtocolViolation(format!(
                    "unsupported response protocol version: {protocol_version}"
                )));
            }
            if request_id.as_deref() != Some(expected_request_id) {
                return Err(NativeMessagingError::ProtocolViolation(
                    "response request_id does not match request".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

pub fn error_response(
    request_id: Option<String>,
    error_code: &str,
    error_message: impl Into<String>,
) -> BrowserResponse {
    BrowserResponse::Error {
        protocol_version: PROTOCOL_VERSION,
        request_id,
        error_code: error_code.to_owned(),
        error_message: error_message.into(),
    }
}

pub fn request_id(request: &BrowserRequest) -> Option<String> {
    match request {
        BrowserRequest::ArchiveRequest { request_id, .. }
        | BrowserRequest::QueryStatus { request_id, .. } => Some(request_id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct Duplex {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl Read for Duplex {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.input.read(buffer)
        }
    }

    impl Write for Duplex {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn round_trips_length_prefixed_payload() {
        let mut encoded = Vec::new();
        write_payload(&mut encoded, br#"{"ok":true}"#).expect("write payload");
        assert_eq!(&encoded[..4], &(11_u32.to_le_bytes()));
        assert_eq!(
            read_payload(&mut Cursor::new(encoded)).expect("read payload"),
            Some(br#"{"ok":true}"#.to_vec())
        );
    }

    #[test]
    fn accepts_clean_eof_between_messages() {
        assert_eq!(
            read_payload(&mut Cursor::new(Vec::<u8>::new())).expect("EOF"),
            None
        );
    }

    #[test]
    fn rejects_truncated_payload() {
        let mut input = Cursor::new(
            5_u32
                .to_le_bytes()
                .into_iter()
                .chain(b"hi".iter().copied())
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            read_payload(&mut input),
            Err(NativeMessagingError::Truncated)
        );
    }

    #[test]
    fn rejects_payload_above_limit() {
        let input = (MAX_MESSAGE_BYTES as u32 + 1).to_le_bytes();
        assert_eq!(
            read_payload(&mut Cursor::new(input)),
            Err(NativeMessagingError::MessageTooLarge(MAX_MESSAGE_BYTES + 1))
        );
    }

    #[test]
    fn forwards_valid_request_and_preserves_response() {
        let response = BrowserResponse::ArchiveStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_id: "123".into(),
            job_id: Some("job-1".into()),
            state: "QUEUED".into(),
            progress: None,
        };
        let mut encoded_response = Vec::new();
        write_json(&mut encoded_response, &response).expect("response");
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["123".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(encoded_response),
            output: Vec::new(),
        };

        assert_eq!(
            forward_request(&mut transport, request.clone()).expect("forward"),
            response
        );
        let mut output = Cursor::new(transport.output);
        let forwarded: BrowserRequest = read_json(&mut output)
            .expect("forwarded request")
            .expect("request present");
        assert_eq!(forwarded, request);
    }

    #[test]
    fn rejects_invalid_request_before_writing_to_transport() {
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["not-numeric".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(Vec::new()),
            output: Vec::new(),
        };

        assert!(matches!(
            forward_request(&mut transport, request),
            Err(NativeMessagingError::InvalidJson(_))
        ));
        assert!(transport.output.is_empty());
    }

    #[test]
    fn rejects_response_with_a_different_request_id() {
        let response = BrowserResponse::ArchiveStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "other-request".into(),
            tweet_id: "123".into(),
            job_id: None,
            state: "QUEUED".into(),
            progress: None,
        };
        let mut encoded_response = Vec::new();
        write_json(&mut encoded_response, &response).expect("response");
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["123".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(encoded_response),
            output: Vec::new(),
        };

        assert!(matches!(
            forward_request(&mut transport, request),
            Err(NativeMessagingError::ProtocolViolation(message))
                if message.contains("request_id")
        ));
    }

    #[test]
    fn rejects_response_with_an_unsupported_protocol_version() {
        let response = BrowserResponse::Error {
            protocol_version: PROTOCOL_VERSION + 1,
            request_id: Some("r1".into()),
            error_code: "ERROR".into(),
            error_message: "unsupported".into(),
        };
        let mut encoded_response = Vec::new();
        write_json(&mut encoded_response, &response).expect("response");
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["123".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(encoded_response),
            output: Vec::new(),
        };

        assert!(matches!(
            forward_request(&mut transport, request),
            Err(NativeMessagingError::ProtocolViolation(message))
                if message.contains("protocol version")
        ));
    }
}
