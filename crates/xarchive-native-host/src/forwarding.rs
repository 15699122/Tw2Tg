use std::io::{Read, Write};

use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

use super::{NativeMessagingError, framing};

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
    framing::write_json(transport, &request)?;
    let response: BrowserResponse = framing::read_json(transport)?
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
