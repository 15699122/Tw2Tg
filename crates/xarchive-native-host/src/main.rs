use std::fs::OpenOptions;
use std::io::{stdin, stdout};
use xarchive_native_host::{
    PIPE_ENDPOINT_ENV, error_response, forward_request, read_json, request_id, write_json,
};
use xarchive_protocol::BrowserRequest;

fn main() {
    let mut input = stdin().lock();
    let mut output = stdout().lock();
    loop {
        match read_json::<_, BrowserRequest>(&mut input) {
            Ok(None) => break,
            Ok(Some(request)) => {
                let response = match request.validate() {
                    Err(error) => {
                        error_response(request_id(&request), "INVALID_REQUEST", error.to_string())
                    }
                    Ok(()) => forward_to_desktop(&request),
                };
                if let Err(write_error) = write_json(&mut output, &response) {
                    eprintln!("failed to write Native Messaging response: {write_error}");
                    break;
                }
            }
            Err(error) => {
                let response = error_response(None, "INVALID_MESSAGE", error.to_string());
                if let Err(write_error) = write_json(&mut output, &response) {
                    eprintln!("failed to write Native Messaging error: {write_error}");
                    break;
                }
            }
        }
    }
}

fn forward_to_desktop(request: &BrowserRequest) -> xarchive_protocol::BrowserResponse {
    let Some(endpoint) = std::env::var_os(PIPE_ENDPOINT_ENV) else {
        return error_response(
            request_id(request),
            "NATIVE_PIPE_UNAVAILABLE",
            "Named Pipe forwarding is not configured",
        );
    };
    let mut transport = match OpenOptions::new().read(true).write(true).open(endpoint) {
        Ok(transport) => transport,
        Err(error) => {
            return error_response(
                request_id(request),
                "NATIVE_PIPE_ERROR",
                format!("failed to connect to Desktop transport: {error}"),
            );
        }
    };
    match forward_request(&mut transport, request.clone()) {
        Ok(response) => response,
        Err(error) => error_response(request_id(request), "NATIVE_PIPE_ERROR", error.to_string()),
    }
}
