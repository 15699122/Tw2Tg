use super::*;

#[cfg(unix)]
#[test]
fn replacing_executor_restarts_transport_on_the_new_service_generation() {
    use xarchive_native_host::{read_json, write_json};
    use xarchive_protocol::{BrowserRequest, BrowserResponse, BrowserTweet, PROTOCOL_VERSION};

    let root = std::env::temp_dir().join(format!(
        "xarchive-runtime-replace-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let mut state = RuntimeState::initialize_at(root.clone());
    let old_service = state.executor.service();
    let config = state.executor.config().clone();
    state
        .replace_executor(config)
        .expect("replace executor and transport");

    assert_eq!(
        old_service.query("old-job").unwrap_err(),
        crate::executor::ExecutorError::Closed
    );

    let endpoint = crate::transport::transport_endpoint(&root);
    let mut stream = std::os::unix::net::UnixStream::connect(&endpoint).expect("connect socket");
    let request = BrowserRequest::ArchiveRequest {
        protocol_version: PROTOCOL_VERSION,
        request_id: "after-replace".to_owned(),
        tweet: BrowserTweet {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            text: Some("archive".to_owned()),
            created_at: Some("2026-09-24T00:00:00Z".to_owned()),
            tweet_type: "post".to_owned(),
            reply_to: None,
            quoted_tweet: None,
        },
    };
    write_json(&mut stream, &request).expect("write request");
    let response: BrowserResponse = read_json(&mut stream)
        .expect("read response")
        .expect("browser response");
    assert!(matches!(
        response,
        BrowserResponse::ArchiveStatus {
            request_id,
            state,
            ..
        } if request_id == "after-replace" && state == "QUEUED"
    ));
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn executor_config_preserves_gallery_dl_and_network_arguments() {
    let root = std::path::Path::new("portable root");
    let mut config = AppConfig::default();
    config.network.discovery_timeout_seconds = 123;
    let executor = executor_config(
        root,
        &config,
        PathBuf::from("config/archive.sqlite3"),
        PathBuf::from("cache/staging"),
        PathBuf::from("download"),
    );
    let gallery = crate::portable::resolve_config_path(root, &config.sidecar.gallery_dl)
        .display()
        .to_string();
    let expected_gallery = ["--gallery-dl".to_owned(), gallery];
    let gallery_index = executor
        .sidecar_args
        .windows(2)
        .position(|pair| pair == expected_gallery)
        .expect("portable gallery-dl arguments");
    let timeout_index = executor
        .sidecar_args
        .windows(2)
        .position(|pair| pair == ["--discovery-timeout-seconds", "123"])
        .expect("network discovery timeout");
    assert!(gallery_index < timeout_index);
}
