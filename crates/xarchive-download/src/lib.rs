//! Download transport abstractions and aria2 integration.

mod client;
mod error;
mod model;
mod router;
mod rpc;
mod supervisor;

pub use client::Aria2HttpClient;
pub use error::DownloadError;
pub use model::{DownloadBackend, TransferFile, TransferId, TransferState, TransferStatus};
pub use model::{DownloadResult, DownloadRoute};
pub use router::{DownloadRouter, DownloadRouterConfig, DownloadRouterError, GalleryDlFailure};
pub use rpc::{
    AddUriRequest, JsonRpcError, JsonRpcRequest, JsonRpcResponse, add_uri_rpc_request,
    get_version_rpc_request, parse_add_uri_response, parse_status_response, pause_rpc_request,
    remove_rpc_request, tell_status_rpc_request, unpause_rpc_request,
};
pub use supervisor::{Aria2Supervisor, Aria2SupervisorConfig};

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

    fn router_request() -> AddUriRequest {
        AddUriRequest {
            url: "https://cdn.example/file.jpg".into(),
            directory: "/tmp/staging/job-1".into(),
            filename: "01.jpg".into(),
            headers: vec![],
        }
    }

    #[test]
    fn router_uses_gallery_dl_by_default() {
        let router = DownloadRouter::default();
        let mut aria2_called = false;
        let result = router
            .execute(
                || Ok(()),
                Some(router_request()),
                |_| {
                    aria2_called = true;
                    Ok(TransferId::new("gid-1").expect("transfer ID"))
                },
            )
            .expect("gallery-dl result");

        assert_eq!(result.route, DownloadRoute::GalleryDl);
        assert_eq!(result.transfer_id, None);
        assert!(!aria2_called);
    }

    #[test]
    fn router_falls_back_to_aria2_for_download_failure() {
        let router = DownloadRouter::default();
        let result = router
            .execute(
                || {
                    Err(GalleryDlFailure::new(
                        "EXTRACT_OR_DOWNLOAD_FAILED",
                        "media URL returned HTTP 403",
                    ))
                },
                Some(router_request()),
                |request| {
                    assert_eq!(request.filename, "01.jpg");
                    Ok(TransferId::new("gid-1").expect("transfer ID"))
                },
            )
            .expect("aria2 fallback");

        assert_eq!(result.route, DownloadRoute::Aria2);
        assert_eq!(result.transfer_id.expect("transfer ID").as_str(), "gid-1");
    }

    #[test]
    fn router_does_not_fallback_for_authentication_failure() {
        let router = DownloadRouter::default();
        let result = router.execute(
            || Err(GalleryDlFailure::new("AUTH_REQUIRED", "login required")),
            Some(router_request()),
            |_| panic!("authentication failure must not invoke aria2"),
        );

        assert_eq!(
            result,
            Err(DownloadRouterError::GalleryDl(GalleryDlFailure::new(
                "AUTH_REQUIRED",
                "login required"
            )))
        );
    }

    #[test]
    fn router_reports_missing_aria2_configuration() {
        let router = DownloadRouter::default();
        let result = router.execute(
            || Err(GalleryDlFailure::new("EXTRACT_OR_DOWNLOAD_FAILED", "403")),
            None,
            |_| panic!("missing request must not invoke aria2"),
        );

        assert_eq!(result, Err(DownloadRouterError::Aria2NotConfigured));
    }

    #[test]
    fn router_can_disable_aria2_fallback() {
        let router = DownloadRouter::new(DownloadRouterConfig {
            allow_aria2_fallback: false,
        });
        let result = router.execute(
            || Err(GalleryDlFailure::new("EXTRACT_OR_DOWNLOAD_FAILED", "403")),
            Some(router_request()),
            |_| panic!("disabled fallback must not invoke aria2"),
        );

        assert_eq!(
            result,
            Err(DownloadRouterError::GalleryDl(GalleryDlFailure::new(
                "EXTRACT_OR_DOWNLOAD_FAILED",
                "403"
            )))
        );
    }

    #[test]
    fn router_preserves_both_failures() {
        let router = DownloadRouter::default();
        let result = router.execute(
            || Err(GalleryDlFailure::new("EXTRACT_OR_DOWNLOAD_FAILED", "403")),
            Some(router_request()),
            |_| Err(DownloadError::HttpStatus(503)),
        );

        assert_eq!(
            result,
            Err(DownloadRouterError::GalleryDlThenAria2 {
                gallery: GalleryDlFailure::new("EXTRACT_OR_DOWNLOAD_FAILED", "403"),
                aria2: DownloadError::HttpStatus(503),
            })
        );
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
