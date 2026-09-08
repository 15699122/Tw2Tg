use std::io::{stdin, stdout};
use xarchive_native_host::{read_json, write_json};
use xarchive_protocol::{BrowserRequest, BrowserResponse};

fn main() {
    let mut input = stdin().lock();
    let mut output = stdout().lock();
    loop {
        match read_json::<_, BrowserRequest>(&mut input) {
            Ok(None) => break,
            Ok(Some(request)) => {
                if let Err(error) = request.validate() {
                    let response = BrowserResponse::Error {
                        protocol_version: xarchive_protocol::PROTOCOL_VERSION,
                        request_id: request_id(&request),
                        error_code: "INVALID_REQUEST".into(),
                        error_message: error.to_string(),
                    };
                    if let Err(write_error) = write_json(&mut output, &response) {
                        eprintln!("failed to write Native Messaging error: {write_error}");
                        break;
                    }
                    continue;
                }
                let response = BrowserResponse::Error {
                    protocol_version: xarchive_protocol::PROTOCOL_VERSION,
                    request_id: request_id(&request),
                    error_code: "NATIVE_PIPE_UNAVAILABLE".into(),
                    error_message: "Named Pipe forwarding is not configured".into(),
                };
                if let Err(write_error) = write_json(&mut output, &response) {
                    eprintln!("failed to write Native Messaging response: {write_error}");
                    break;
                }
            }
            Err(error) => {
                let response = BrowserResponse::Error {
                    protocol_version: xarchive_protocol::PROTOCOL_VERSION,
                    request_id: None,
                    error_code: "INVALID_MESSAGE".into(),
                    error_message: error.to_string(),
                };
                if let Err(write_error) = write_json(&mut output, &response) {
                    eprintln!("failed to write Native Messaging error: {write_error}");
                    break;
                }
            }
        }
    }
}

fn request_id(request: &BrowserRequest) -> Option<String> {
    match request {
        BrowserRequest::ArchiveRequest { request_id, .. }
        | BrowserRequest::QueryStatus { request_id, .. } => Some(request_id.clone()),
    }
}
