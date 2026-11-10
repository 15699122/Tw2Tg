//! Contract-level suites: SecretStore, request/response models, endpoint
//! policy, bot identity and transport error mapping.

use super::*;

#[test]
fn stores_and_deletes_secrets_without_exposing_values() {
    let mut store = MemorySecretStore::default();
    store.set("telegram.bot_token", "123:secret").expect("set");
    assert_eq!(
        store.get("telegram.bot_token").expect("get"),
        Some("123:secret".into())
    );
    let token =
        BotToken::new(store.get("telegram.bot_token").expect("get").unwrap()).expect("token");
    assert_eq!(token.to_string(), "[REDACTED]");
    assert!(!format!("{token:?}").contains("123:secret"));
    assert!(!format!("{store:?}").contains("123:secret"));
    store.delete("telegram.bot_token").expect("delete");
    assert_eq!(store.get("telegram.bot_token").expect("get"), None);
}

#[test]
fn validates_message_boundaries() {
    let valid = SendMessageRequest {
        chat_id: "-100".into(),
        message_thread_id: Some(42),
        text: "hello".into(),
        disable_web_page_preview: true,
    };
    assert!(validate_message(&valid).is_ok());
    assert_eq!(
        validate_message(&SendMessageRequest {
            chat_id: "".into(),
            ..valid.clone()
        }),
        Err(TelegramError::InvalidChatId)
    );
    assert_eq!(
        validate_message(&SendMessageRequest {
            text: "".into(),
            ..valid
        }),
        Err(TelegramError::EmptyText)
    );
}

#[test]
fn sends_message_over_https_transport_contract() {
    let (endpoint, server) = fake_server(r#"{"ok":true}"#);
    let transport = ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
    let token = BotToken::new("123:secret").expect("token");
    let response = transport
        .send(
            &token,
            TelegramRequest::Message(SendMessageRequest {
                chat_id: "-100".into(),
                message_thread_id: Some(42),
                text: "hello".into(),
                disable_web_page_preview: true,
            }),
        )
        .expect("send");
    assert!(response.ok);
    let request = server.join().expect("server");
    assert!(request.starts_with("POST /bot123:secret/sendMessage HTTP/1.1"));
    assert!(request.contains(r#""chat_id":"-100""#));
    assert!(request.contains(r#""text":"hello""#));
    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("HTTP body");
    let json: serde_json::Value = serde_json::from_str(body).expect("JSON request");
    assert_eq!(json["message_thread_id"], 42);
}

#[test]
fn sends_photo_video_and_media_group_methods() {
    for (request, method) in [
        (
            TelegramRequest::Photo(SendPhotoRequest {
                chat_id: "-100".into(),
                message_thread_id: Some(42),
                photo: "photo-id".into(),
                caption: Some("caption".into()),
            }),
            "sendPhoto",
        ),
        (
            TelegramRequest::Video(SendVideoRequest {
                chat_id: "-100".into(),
                message_thread_id: Some(42),
                video: "video-id".into(),
                caption: None,
            }),
            "sendVideo",
        ),
        (
            TelegramRequest::MediaGroup(SendMediaGroupRequest {
                chat_id: "-100".into(),
                message_thread_id: Some(42),
                media: vec![MediaGroupItem {
                    media_type: "photo".into(),
                    media: "photo-id".into(),
                    caption: None,
                }],
            }),
            "sendMediaGroup",
        ),
    ] {
        let (endpoint, server) = fake_server(r#"{"ok":true}"#);
        let transport = ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
        let token = BotToken::new("123:secret").expect("token");
        transport.send(&token, request).expect("send");
        let request = server.join().expect("server");
        assert!(request.starts_with(&format!("POST /bot123:secret/{method} HTTP/1.1")));
    }
}

#[test]
fn maps_telegram_api_and_http_errors_without_exposing_token() {
    let (endpoint, server) =
        fake_server(r#"{"ok":false,"error_code":429,"description":"Too Many Requests"}"#);
    let transport = ReqwestTelegramTransport::with_test_endpoint(endpoint).expect("transport");
    let token = BotToken::new("123:secret").expect("token");
    let error = transport
        .send(
            &token,
            TelegramRequest::Message(SendMessageRequest {
                chat_id: "-100".into(),
                message_thread_id: None,
                text: "hello".into(),
                disable_web_page_preview: false,
            }),
        )
        .expect_err("api error");
    assert_eq!(
        error,
        TelegramError::Api {
            code: 429,
            description: "Too Many Requests".into(),
            parameters: None
        }
    );
    assert!(!error.to_string().contains("123:secret"));
    server.join().expect("server");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = format!("http://{}", listener.local_addr().expect("address"));
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .expect("write");
    });
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let error = transport
        .send(
            &token,
            TelegramRequest::Message(SendMessageRequest {
                chat_id: "-100".into(),
                message_thread_id: None,
                text: "hello".into(),
                disable_web_page_preview: false,
            }),
        )
        .expect_err("http error");
    assert_eq!(error, TelegramError::Transport("HTTP status 503".into()));
    server.join().expect("server");
}

#[test]
fn rejects_non_https_production_endpoints() {
    assert!(matches!(
        ReqwestTelegramTransport::with_endpoint("http://localhost:8080"),
        Err(TelegramError::InvalidEndpoint)
    ));
    assert!(ReqwestTelegramTransport::new().is_ok());
}

#[test]
fn a_resolved_proxy_actually_receives_the_request() {
    // A proxy receives an absolute-form request line. If the transport
    // discarded the proxy, the request would go straight to the target and
    // the proxy would never be contacted.
    let proxy_listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy");
    let proxy_address = proxy_listener.local_addr().expect("proxy address");
    let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let recorded = seen.clone();
    let proxy = thread::spawn(move || {
        let (mut stream, _) = proxy_listener.accept().expect("accept");
        let mut request = [0_u8; 2048];
        let read = stream.read(&mut request).unwrap_or(0);
        recorded
            .lock()
            .expect("record")
            .push_str(&String::from_utf8_lossy(&request[..read]));
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"ok\":true,\"x\":1}",
        );
    });

    let transport = ReqwestTelegramTransport::with_resolved_proxy(
        "https://api.telegram.org",
        std::time::Duration::from_secs(10),
        Some(format!("http://{proxy_address}")),
    )
    .expect("transport");
    // The call must fail at the proxy stage, which is enough to prove the
    // request was routed there rather than to the real API.
    let _ = transport.send(
        &BotToken::new("123456:ABCDEF").expect("token"),
        TelegramRequest::Message(SendMessageRequest {
            chat_id: "1".into(),
            message_thread_id: None,
            text: "hello".into(),
            disable_web_page_preview: false,
        }),
    );
    proxy.join().expect("proxy thread");
    let observed = seen.lock().expect("read").clone();
    assert!(
        observed.contains("api.telegram.org"),
        "the proxy must receive the request, got: {observed}"
    );
}

#[test]
fn a_disabled_proxy_still_builds_a_direct_client() {
    // `with_network(..., disable_proxy = true)` must still build; the
    // no_proxy path is what keeps a direct promise.
    assert!(
        ReqwestTelegramTransport::with_network(
            "https://api.telegram.org",
            std::time::Duration::from_secs(5),
            true
        )
        .is_ok()
    );
}

#[test]
fn a_malformed_proxy_is_rejected_rather_than_silently_ignored() {
    let result = ReqwestTelegramTransport::with_resolved_proxy(
        "https://api.telegram.org",
        std::time::Duration::from_secs(5),
        Some("not a url".to_owned()),
    );
    assert!(matches!(result, Err(TelegramError::Transport(_))));
}

#[test]
fn parses_result_message_id_from_bot_api_response() {
    let response: TelegramResponse =
        serde_json::from_str(r#"{"ok":true,"result":{"message_id":4242,"chat":{"id":-100}}}"#)
            .expect("response");
    assert_eq!(response.result_message_id(), Some("4242".into()));
    let plain: TelegramResponse = serde_json::from_str(r#"{"ok":true}"#).expect("plain");
    assert_eq!(plain.result_message_id(), None);
}

#[test]
fn accepts_cloud_and_loopback_local_endpoints() {
    let cloud = TelegramEndpoint::parse(EndpointMode::Cloud, "https://api.telegram.org")
        .expect("cloud endpoint");
    assert_eq!(cloud.base(), "https://api.telegram.org");
    assert_eq!(cloud.mode(), EndpointMode::Cloud);
    assert_eq!(TelegramEndpoint::cloud().base(), cloud.base());

    // A trailing slash is normalised away rather than rejected.
    let with_slash = TelegramEndpoint::parse(EndpointMode::Cloud, "https://api.telegram.org/")
        .expect("trailing slash");
    assert_eq!(with_slash.base(), "https://api.telegram.org");

    for base in [
        "http://127.0.0.1:8081",
        "http://localhost:8081",
        "http://[::1]:8081",
    ] {
        let local = TelegramEndpoint::parse(EndpointMode::Local, base).expect("loopback endpoint");
        assert_eq!(local.mode(), EndpointMode::Local);
        assert_eq!(local.base(), base);
    }

    assert_eq!(EndpointMode::parse("cloud"), Some(EndpointMode::Cloud));
    assert_eq!(EndpointMode::parse("local"), Some(EndpointMode::Local));
    assert_eq!(EndpointMode::parse("Local"), None);
    assert_eq!(EndpointMode::Cloud.as_str(), "cloud");
}

#[test]
fn rejects_endpoints_that_could_redirect_or_leak_the_token() {
    // Cloud must be HTTPS.
    assert_eq!(
        TelegramEndpoint::parse(EndpointMode::Cloud, "http://api.telegram.org"),
        Err(TelegramError::EndpointNotSecure)
    );
    // Local must be an explicit loopback HTTP address with a port.
    for base in [
        "http://192.168.1.5:8081",
        "http://10.0.0.1:8081",
        "http://127.0.0.1",
        "https://127.0.0.1:8081",
    ] {
        assert_eq!(
            TelegramEndpoint::parse(EndpointMode::Local, base),
            Err(TelegramError::EndpointNotLoopback),
            "expected rejection for {base}"
        );
    }
    // Embedded credentials, query, fragment and extra path are malformed in
    // either mode: they turn a token-bearing URL into another target.
    for base in [
        "https://user:pass@api.telegram.org",
        "https://api.telegram.org/?token=x",
        "https://api.telegram.org/#frag",
        "https://api.telegram.org/extra",
        "not a url",
    ] {
        assert_eq!(
            TelegramEndpoint::parse(EndpointMode::Cloud, base),
            Err(TelegramError::EndpointMalformed),
            "expected malformed for {base}"
        );
    }
}

#[test]
fn secret_store_error_variants_do_not_carry_the_secret() {
    let unavailable = SecretStoreError::Unavailable("credential manager offline".into());
    assert_eq!(
        unavailable.to_string(),
        "secret store unavailable: credential manager offline"
    );
    let denied = SecretStoreError::AccessDenied("policy blocked read".into());
    assert_eq!(
        denied.to_string(),
        "secret store access denied: policy blocked read"
    );
    assert_ne!(unavailable, denied);
}

#[test]
fn extracts_message_and_file_ids_from_success_results() {
    let album: TelegramResponse = serde_json::from_str(
        r#"{"ok":true,"result":[{"message_id":1,"photo":{"file_id":"p1"}},{"message_id":2,"photo":{"file_id":"p2"}}]}"#,
    )
    .expect("album response");
    assert_eq!(album.result_message_ids(), ["1", "2"]);
    assert_eq!(album.result_file_ids(), ["p1", "p2"]);

    let single: TelegramResponse =
        serde_json::from_str(r#"{"ok":true,"result":{"message_id":9,"video":{"file_id":"v1"}}}"#)
            .expect("single response");
    assert_eq!(single.result_message_ids(), ["9"]);
    assert_eq!(single.result_file_ids(), ["v1"]);
    assert_eq!(single.result_message_id(), Some("9".into()));

    let plain: TelegramResponse =
        serde_json::from_str(r#"{"ok":true,"result":true}"#).expect("plain response");
    assert!(plain.result_message_ids().is_empty());
    assert!(plain.result_file_ids().is_empty());
}

#[test]
fn verified_bot_identity_is_stable_across_credentials_and_rejects_non_bots() {
    struct IdentityTransport(serde_json::Value);
    impl TelegramTransport for IdentityTransport {
        fn send(
            &self,
            _token: &BotToken,
            request: TelegramRequest,
        ) -> Result<TelegramResponse, TelegramError> {
            assert_eq!(request, TelegramRequest::GetMe);
            serde_json::from_value(serde_json::json!({"ok":true,"result":self.0}))
                .map_err(json_error)
        }
    }
    let transport = IdentityTransport(serde_json::json!({"id":1234567890123_i64,"is_bot":true}));
    let first = BotToken::new("first-token").expect("token");
    let second = BotToken::new("second-token").expect("token");
    assert_eq!(
        verify_bot_identity(&transport, &first).expect("identity"),
        "telegram-bot:1234567890123"
    );
    assert_eq!(
        verify_bot_identity(&transport, &first).unwrap(),
        verify_bot_identity(&transport, &second).unwrap()
    );
    for result in [
        serde_json::json!({"id":1,"is_bot":false}),
        serde_json::json!({"id":0,"is_bot":true}),
        serde_json::json!({"id":-1,"is_bot":true}),
        serde_json::json!({"id":"1","is_bot":true}),
        serde_json::json!({"is_bot":true}),
    ] {
        assert!(verify_bot_identity(&IdentityTransport(result), &first).is_err());
    }
    let (method, payload) = request_payload(TelegramRequest::GetMe).expect("request");
    assert_eq!(method, "getMe");
    assert_eq!(payload, serde_json::json!({}));
    let (method, payload) = request_payload(TelegramRequest::GetChat {
        chat_id: "-100123".into(),
    })
    .unwrap();
    assert_eq!(method, "getChat");
    assert_eq!(payload, serde_json::json!({"chat_id":"-100123"}));
    assert!(request_payload(TelegramRequest::GetChat { chat_id: "".into() }).is_err());
    assert_eq!(
        request_payload(TelegramRequest::LogOut).unwrap().0,
        "logOut"
    );
    assert_eq!(request_payload(TelegramRequest::Close).unwrap().0, "close");
    assert_eq!(
        request_payload(TelegramRequest::DeleteWebhook).unwrap(),
        (
            "deleteWebhook",
            serde_json::json!({"drop_pending_updates": false})
        )
    );
}

#[test]
fn api_endpoint_contract_gates_local_http_production_transport() {
    // Local mode is the production consumer of the TG-01 contract: an
    // explicit loopback HTTP server is accepted...
    let local =
        TelegramEndpoint::parse(EndpointMode::Local, "http://127.0.0.1:8081/").expect("local");
    assert_eq!(local.base(), "http://127.0.0.1:8081");
    let transport =
        ReqwestTelegramTransport::with_api_endpoint(local, Duration::from_secs(5), None)
            .expect("local transport");
    assert!(format!("{transport:?}").contains("http://127.0.0.1:8081"));

    // ...while non-loopback HTTP, a port-less local target and local
    // HTTPS are refused by the contract itself, before any transport
    // exists, and the raw-string constructor still demands HTTPS.
    assert!(matches!(
        TelegramEndpoint::parse(EndpointMode::Local, "http://93.184.216.34:8080"),
        Err(TelegramError::EndpointNotLoopback)
    ));
    assert!(matches!(
        TelegramEndpoint::parse(EndpointMode::Local, "http://127.0.0.1"),
        Err(TelegramError::EndpointNotLoopback)
    ));
    assert!(matches!(
        TelegramEndpoint::parse(EndpointMode::Local, "https://127.0.0.1:8081"),
        Err(TelegramError::EndpointNotLoopback)
    ));
    assert!(matches!(
        TelegramEndpoint::parse(EndpointMode::Cloud, "http://api.telegram.org"),
        Err(TelegramError::EndpointNotSecure)
    ));

    let cloud = ReqwestTelegramTransport::with_api_endpoint(
        TelegramEndpoint::cloud(),
        Duration::from_secs(5),
        None,
    )
    .expect("cloud transport");
    assert!(format!("{cloud:?}").contains("https://api.telegram.org"));
}

#[test]
fn migration_source_and_target_apply_endpoint_specific_proxy_policy() {
    // A Cloud→Local migration builds two transports from the same
    // resolved proxy value. The local leg addresses the operator's own
    // loopback server, so a configured proxy must be dropped there and
    // kept for cloud; otherwise the token would travel through a proxy
    // that has no reason to see it, or the local call would leave the
    // machine.
    let proxy = Some("http://user:secret@127.0.0.1:3128".to_owned());
    let source = ReqwestTelegramTransport::with_api_endpoint(
        TelegramEndpoint::cloud(),
        Duration::from_secs(5),
        proxy.clone(),
    )
    .expect("cloud source transport");
    let target = ReqwestTelegramTransport::with_api_endpoint(
        TelegramEndpoint::parse(EndpointMode::Local, "http://127.0.0.1:8081/").expect("local"),
        Duration::from_secs(5),
        proxy.clone(),
    )
    .expect("local target transport");

    let cloud_debug = format!("{source:?}");
    let local_debug = format!("{target:?}");
    assert!(
        cloud_debug.contains("disable_proxy: false"),
        "cloud leg must keep the configured proxy: {cloud_debug}"
    );
    assert!(
        local_debug.contains("disable_proxy: true"),
        "local leg must pin a direct connection: {local_debug}"
    );
    // The local leg must not retain the proxy value at all, and neither
    // leg may expose proxy credentials through Debug.
    assert!(
        !local_debug.contains("3128") && !cloud_debug.contains("secret"),
        "proxy credentials must never reach Debug output: {cloud_debug} / {local_debug}"
    );
}

#[test]
fn bot_identity_is_stable_reversible_free_and_token_sized() {
    let token = BotToken::new("1234567:AAFakeSecretValue").expect("token");
    let identity = token.identity();
    assert_eq!(identity.len(), 64, "full SHA-256 hex");
    assert!(
        identity
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );
    assert_eq!(identity, token.identity(), "identity is stable");
    assert!(!identity.contains("1234567"));
    assert!(!identity.contains("AAFakeSecret"));
    let other = BotToken::new("9999999:OtherSecret").expect("token");
    assert_ne!(identity, other.identity(), "rotation isolates the bot");
    // Debug/Display stay redacted with the new method present.
    assert!(!format!("{token:?}").contains("AAFakeSecretValue"));
    assert_eq!(token.to_string(), "[REDACTED]");
}
