use std::io::{stdin, stdout};
use xarchive_native_host::{
    PIPE_ENDPOINT_ENV, error_response, forward_request, read_json, request_id, write_json,
};
use xarchive_protocol::BrowserRequest;

fn main() {
    let mut input = stdin().lock();
    let mut output = stdout().lock();
    loop {
        match read_json::<_, serde_json::Value>(&mut input) {
            Ok(None) => break,
            Ok(Some(payload)) => {
                if payload.get("message_type").and_then(|value| value.as_str()) == Some("bootstrap")
                {
                    let response = forward_control(payload, std::env::args().nth(1).as_deref());
                    if write_json(&mut output, &response).is_err() {
                        break;
                    }
                    continue;
                }
                let request: BrowserRequest = match serde_json::from_value(payload) {
                    Ok(request) => request,
                    Err(error) => {
                        if write_json(
                            &mut output,
                            &error_response(None, "INVALID_MESSAGE", error.to_string()),
                        )
                        .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                };
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
    let Some(endpoint) = std::env::var_os(PIPE_ENDPOINT_ENV).or_else(default_desktop_endpoint)
    else {
        return error_response(
            request_id(request),
            "NATIVE_PIPE_UNAVAILABLE",
            "Named Pipe forwarding is not configured",
        );
    };
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let mut transport = match UnixStream::connect(endpoint) {
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
            Err(error) => {
                error_response(request_id(request), "NATIVE_PIPE_ERROR", error.to_string())
            }
        }
    }

    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
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
            Err(error) => {
                error_response(request_id(request), "NATIVE_PIPE_ERROR", error.to_string())
            }
        }
    }
}

/// Platform default used when `XARCHIVE_PIPE_ENDPOINT` is unset.
///
/// Windows falls back to the shared default pipe name so Desktop and Native
/// Host agree without environment wiring. Unix keeps requiring the variable
/// because the socket path depends on Desktop's portable root.
#[cfg(windows)]
fn default_desktop_endpoint() -> Option<std::ffi::OsString> {
    Some(std::ffi::OsString::from(
        xarchive_protocol::WINDOWS_PIPE_ENDPOINT,
    ))
}

#[cfg(unix)]
fn default_desktop_endpoint() -> Option<std::ffi::OsString> {
    None
}

fn forward_control(
    payload: serde_json::Value,
    origin: Option<&str>,
) -> xarchive_protocol::BrowserPairingResponse {
    use xarchive_protocol::{BrowserPairingRequest, BrowserPairingResponse};
    let request = serde_json::from_value::<BrowserPairingRequest>(payload);
    let request_id = request.as_ref().ok().map(|request| match request {
        BrowserPairingRequest::Bootstrap { request_id, .. } => request_id.clone(),
    });
    let failure = |code: &str, retryable| BrowserPairingResponse::Error {
        protocol_version: 1,
        request_id: request_id.clone(),
        error_code: code.into(),
        error_message: "Desktop bootstrap unavailable".into(),
        retryable,
    };
    let Ok(request) = request else {
        return failure("INVALID_REQUEST", false);
    };
    if request.validate().is_err() {
        return failure("INVALID_REQUEST", false);
    }
    if origin != Some("chrome-extension://iaajefkoanbkleojofoadeakelihbjne/") {
        return failure("ORIGIN_REJECTED", false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        use std::time::Duration;
        let Some(endpoint) = std::env::var_os(PIPE_ENDPOINT_ENV) else {
            return failure("NATIVE_PIPE_UNAVAILABLE", true);
        };
        let Ok(mut transport) = UnixStream::connect(endpoint) else {
            return failure("DESKTOP_NOT_READY", true);
        };
        if transport
            .set_read_timeout(Some(Duration::from_secs(3)))
            .is_err()
            || transport
                .set_write_timeout(Some(Duration::from_secs(3)))
                .is_err()
        {
            return failure("NATIVE_PIPE_ERROR", true);
        }
        xarchive_native_host::forward_bootstrap(&mut transport, request)
            .unwrap_or_else(|_| failure("NATIVE_PIPE_ERROR", true))
    }
    #[cfg(windows)]
    {
        // Windows Owner wires the existing Named Pipe client after shared handoff.
        let _ = request;
        failure("BOOTSTRAP_NOT_IMPLEMENTED", false)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_entry_rejects_untrusted_origin_and_business_envelopes() {
        let bootstrap =
            serde_json::json!({"protocol_version":1,"message_type":"bootstrap","request_id":"r1"});
        for origin in [
            None,
            Some("https://x.com"),
            Some("chrome-extension://other/"),
        ] {
            let response = forward_control(bootstrap.clone(), origin);
            assert!(
                matches!(response, xarchive_protocol::BrowserPairingResponse::Error { error_code, retryable: false, .. } if error_code == "ORIGIN_REJECTED")
            );
        }
        let response = forward_control(
            serde_json::json!({"message_type":"query_status"}),
            Some("chrome-extension://iaajefkoanbkleojofoadeakelihbjne/"),
        );
        assert!(
            matches!(response, xarchive_protocol::BrowserPairingResponse::Error { error_code, .. } if error_code == "INVALID_REQUEST")
        );
    }
}
