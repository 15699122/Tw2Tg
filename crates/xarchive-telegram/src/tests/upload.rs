//! Upload suites: request validation, layered deadlines, streaming progress
//! and media-group assembly.

use super::*;

#[test]
fn single_upload_contains_exactly_one_topic_field() {
    let path = temp_upload_file("single-topic", b"image-bytes");
    let (address, server) = upload_http_server(
        None,
        br#"{"ok":true,"result":{"message_id":7,"photo":[{"file_id":"p7"}]}}"#.to_vec(),
    );
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let request = album_upload_request(&path, "topic.jpg");
    let outcome = block_on(transport.send_upload(
        &token,
        &request,
        &UploadTimeouts::default(),
        &CancellationToken::new(),
        |_| {},
    ));
    assert!(outcome.expect("upload").ok);
    let received = server.join().expect("server");
    let parsed = parse_multipart_request(&received).unwrap_or_else(|error| {
        panic!(
            "{error}; raw request: {}",
            String::from_utf8_lossy(&received)
        )
    });
    assert_eq!(parsed.fields["message_thread_id"], "42");
    assert_eq!(
        String::from_utf8_lossy(&received)
            .matches("name=\"message_thread_id\"")
            .count(),
        1
    );
    let _ = std::fs::remove_file(path);
}

fn sample_upload_request() -> UploadRequest {
    UploadRequest {
        chat_id: "-100123".into(),
        message_thread_id: Some(42),
        media_kind: MediaKind::Photo,
        file_path: std::path::PathBuf::from("/tmp/x.jpg"),
        file_name: "x_1_ab.jpg".into(),
        mime_type: Some("image/jpeg".into()),
        caption: Some("#tag".into()),
        expected_size: Some(10),
    }
}

#[test]
fn validates_upload_request_boundaries() {
    assert!(validate_upload_request(&sample_upload_request()).is_ok());

    let empty_chat = UploadRequest {
        chat_id: " ".into(),
        ..sample_upload_request()
    };
    assert_eq!(
        validate_upload_request(&empty_chat),
        Err(TelegramError::InvalidChatId)
    );

    // The name lands in a Content-Disposition header: emptiness and
    // quoting/newline/path characters must all be rejected.
    for bad_name in [
        "",
        "  ",
        "with/slash.jpg",
        "with\"quote.jpg",
        "line\nbreak.jpg",
        "back\\slash.jpg",
    ] {
        let request = UploadRequest {
            file_name: bad_name.into(),
            ..sample_upload_request()
        };
        assert!(
            matches!(
                validate_upload_request(&request),
                Err(TelegramError::InvalidUploadRequest(_))
            ),
            "{bad_name:?} must be rejected"
        );
    }

    for bad_mime in ["", "image", "image/png\rX: y"] {
        let request = UploadRequest {
            mime_type: Some(bad_mime.into()),
            ..sample_upload_request()
        };
        assert!(
            matches!(
                validate_upload_request(&request),
                Err(TelegramError::InvalidUploadRequest(_))
            ),
            "{bad_mime:?} must be rejected"
        );
    }
    let no_mime = UploadRequest {
        mime_type: None,
        ..sample_upload_request()
    };
    assert!(validate_upload_request(&no_mime).is_ok());
}

#[test]
fn validates_upload_timeout_layers() {
    assert!(UploadTimeouts::default().validate().is_ok());
    let zero = Duration::ZERO;
    let broken = [
        UploadTimeouts {
            connect: zero,
            ..UploadTimeouts::default()
        },
        UploadTimeouts {
            body_stall: zero,
            ..UploadTimeouts::default()
        },
        UploadTimeouts {
            server_processing: zero,
            ..UploadTimeouts::default()
        },
    ];
    for timeouts in broken {
        assert!(
            matches!(
                timeouts.validate(),
                Err(TelegramError::InvalidUploadRequest(_))
            ),
            "{timeouts:?} must be rejected"
        );
    }
    // The overall deadline is optional; omitting it stays valid.
    let no_total = UploadTimeouts {
        total: None,
        ..UploadTimeouts::default()
    };
    assert!(no_total.validate().is_ok());
}

#[test]
fn maps_media_kinds_to_upload_methods_and_fields() {
    assert_eq!(upload_method(MediaKind::Photo), ("sendPhoto", "photo"));
    assert_eq!(upload_method(MediaKind::Video), ("sendVideo", "video"));
    assert_eq!(
        upload_method(MediaKind::Document),
        ("sendDocument", "document")
    );
}

// ---------------------------------------------------------------
// TG-02 upload test helpers
// ---------------------------------------------------------------

fn temp_upload_file(tag: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xarchive-tg-upload-{tag}-{}.bin",
        std::process::id()
    ));
    std::fs::write(&path, bytes).expect("write temp upload file");
    path
}

struct ParsedMultipartFile {
    file_name: String,
    bytes: Vec<u8>,
}

struct ParsedMultipartRequest {
    fields: std::collections::HashMap<String, String>,
    files: std::collections::HashMap<String, ParsedMultipartFile>,
}

fn parse_multipart_request(request: &[u8]) -> Result<ParsedMultipartRequest, String> {
    let header_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "request headers are incomplete".to_owned())?;
    let headers = std::str::from_utf8(&request[..header_end]).map_err(|error| error.to_string())?;
    let content_type = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-type")
                .then_some(value.trim())
        })
        .ok_or_else(|| "missing content-type".to_owned())?;
    let boundary = content_type
        .split(';')
        .map(str::trim)
        .find_map(|parameter| parameter.strip_prefix("boundary="))
        .ok_or_else(|| "missing multipart boundary".to_owned())?;
    let marker = format!("--{boundary}").into_bytes();
    let body = &request[header_end + 4..];
    let mut fields = std::collections::HashMap::new();
    let mut files = std::collections::HashMap::new();

    let mut cursor = 0;
    while let Some(relative_start) = body[cursor..]
        .windows(marker.len())
        .position(|window| window == marker)
    {
        let part_start = cursor + relative_start + marker.len();
        if body.get(part_start..part_start + 2) == Some(b"--") {
            break;
        }
        let part_start = part_start
            .checked_add(2)
            .ok_or_else(|| "invalid multipart boundary offset".to_owned())?;
        let header_offset = body[part_start..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| "multipart part headers are incomplete".to_owned())?;
        let part_headers = std::str::from_utf8(&body[part_start..part_start + header_offset])
            .map_err(|error| error.to_string())?;
        let content_start = part_start + header_offset + 4;
        let next = body[content_start..]
            .windows(marker.len() + 2)
            .position(|window| window.starts_with(b"\r\n") && window[2..] == *marker)
            .map(|offset| content_start + offset)
            .ok_or_else(|| "multipart part terminator is missing".to_owned())?;
        let content = &body[content_start..next];
        let disposition = part_headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-disposition")
                    .then_some(value.trim())
            })
            .ok_or_else(|| "missing content-disposition".to_owned())?;
        let parameter = |name: &str| -> Option<String> {
            let prefix = format!("{name}=\"");
            disposition.split(';').map(str::trim).find_map(|item| {
                let (key, value) = item.split_once('=')?;
                if key.eq_ignore_ascii_case(name) {
                    return Some(value.trim().trim_matches('"').to_owned());
                }
                item.strip_prefix(&prefix)
                    .map(|value| value.trim_end_matches('"').to_owned())
            })
        };
        let name = parameter("name").ok_or_else(|| "missing part name".to_owned())?;
        if let Some(file_name) = parameter("filename") {
            files.insert(
                name,
                ParsedMultipartFile {
                    file_name,
                    bytes: content.to_vec(),
                },
            );
        } else {
            fields.insert(
                name,
                String::from_utf8(content.to_vec()).map_err(|error| error.to_string())?,
            );
        }
        cursor = next + 2;
    }
    Ok(ParsedMultipartRequest { fields, files })
}

/// Serves exactly one request and answers with a 200 JSON response of
/// `body`. `claimed_length` overrides the announced `Content-Length`
/// (which may exceed the real body) so the size bound can be tested.
fn upload_http_server(
    claimed_length: Option<u64>,
    body: Vec<u8>,
) -> (String, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind upload server");
    let address = format!("http://{}", listener.local_addr().expect("address"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept upload request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let received = read_http_request(&mut stream);
        let length = claimed_length.unwrap_or(body.len() as u64);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
        // Keep the socket open briefly so the client can read the answer.
        thread::sleep(Duration::from_millis(50));
        received
    });
    (address, handle)
}

#[tokio::test]
async fn watchdog_classifies_the_first_layered_deadline() {
    let layer = Duration::from_millis(40);
    let slow = Duration::from_secs(60);
    let base = UploadTimeouts {
        connect: Duration::from_secs(5),
        body_stall: slow,
        server_processing: slow,
        total: None,
    };

    // Streaming with no byte progress for `body_stall` -> body stall.
    let streaming = Arc::new(UploadTracker::new());
    streaming.record_body_progress(1);
    let error = upload_watchdog(
        Arc::clone(&streaming),
        UploadTimeouts {
            body_stall: layer,
            ..base.clone()
        },
    )
    .await;
    assert_eq!(error, TelegramError::UploadBodyStalled);

    // Body finished, response slower than `server_processing` -> server timeout.
    let finished = Arc::new(UploadTracker::new());
    finished.mark_body_finished();
    let error = upload_watchdog(
        finished,
        UploadTimeouts {
            server_processing: layer,
            ..base.clone()
        },
    )
    .await;
    assert_eq!(error, TelegramError::UploadServerTimeout);

    // Before the body starts, only the overall deadline applies.
    let queued = Arc::new(UploadTracker::new());
    let error = upload_watchdog(
        queued,
        UploadTimeouts {
            total: Some(layer),
            ..base
        },
    )
    .await;
    assert_eq!(error, TelegramError::UploadDeadlineExceeded);
}

#[test]
fn upload_short_circuits_before_the_network_when_cancelled() {
    let transport =
        ReqwestTelegramTransport::with_test_endpoint("http://127.0.0.1:9").expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&stages);
    let error = block_on(transport.send_upload(
        &token,
        &sample_upload_request(),
        &UploadTimeouts::default(),
        &cancellation,
        move |stage| observed.lock().expect("stages").push(stage),
    ))
    .expect_err("cancelled upload must not run");
    assert_eq!(error, TelegramError::UploadCancelled);
    // Only the queued stage was reported: no file check, no upload, and
    // no fabricated progress.
    assert_eq!(*stages.lock().expect("stages"), vec![UploadStage::Queued]);
}

#[test]
fn upload_reports_missing_and_changed_files_before_any_network() {
    let transport =
        ReqwestTelegramTransport::with_test_endpoint("http://127.0.0.1:9").expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let cancellation = CancellationToken::new();

    let mut missing = sample_upload_request();
    missing.file_path = std::env::temp_dir().join("xarchive-tg-missing-upload-file.bin");
    let error = block_on(transport.send_upload(
        &token,
        &missing,
        &UploadTimeouts::default(),
        &cancellation,
        |_| {},
    ))
    .expect_err("missing file must fail");
    assert!(matches!(error, TelegramError::FileUnreadable(_)));

    let path = temp_upload_file("changed", b"1234");
    let mut changed = sample_upload_request();
    changed.file_path = path.clone();
    changed.expected_size = Some(999);
    let error = block_on(transport.send_upload(
        &token,
        &changed,
        &UploadTimeouts::default(),
        &cancellation,
        |_| {},
    ))
    .expect_err("size mismatch must fail");
    assert!(matches!(error, TelegramError::FileChanged(_)));
    let _ = std::fs::remove_file(path);
}

#[test]
fn send_upload_streams_the_file_and_reports_ordered_progress() {
    let payload: Vec<u8> = (0..100_000u32).map(|index| (index % 251) as u8).collect();
    let path = temp_upload_file("stream", &payload);
    let expected_size = payload.len() as u64;

    let response_body = br#"{"ok":true,"result":{"message_id":77,"chat":{"id":-100},"photo":{"file_id":"photo-file-1"}}}"#
        .to_vec();
    let (address, server) = upload_http_server(None, response_body);
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let cancellation = CancellationToken::new();

    let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&stages);
    let mut request = sample_upload_request();
    request.file_path = path.clone();
    request.expected_size = Some(expected_size);

    let response = block_on(transport.send_upload(
        &token,
        &request,
        &UploadTimeouts::default(),
        &cancellation,
        move |stage| observed.lock().expect("stages").push(stage),
    ))
    .expect("upload succeeds");

    // The success result carries message and media ids, not just a status.
    assert_eq!(response.result_message_id(), Some("77".into()));
    assert_eq!(response.result_file_ids(), ["photo-file-1"]);

    let received = server.join().expect("server thread");
    let received = String::from_utf8_lossy(&received).to_ascii_lowercase();
    assert!(received.contains("/bot1234:test/sendphoto"));
    assert!(received.contains("content-type: multipart/form-data"));
    assert!(received.contains("name=\"photo\""));
    assert!(
        received.contains("x_1_ab.jpg"),
        "file name reaches Telegram"
    );
    assert!(received.contains("content-length:"), "length-known body");

    // Progress order: queued -> checking -> uploading* -> awaiting -> confirmed.
    let stages = stages.lock().expect("stages").clone();
    assert_eq!(stages.first(), Some(&UploadStage::Queued));
    assert_eq!(stages.get(1), Some(&UploadStage::CheckingFile));
    let uploads: Vec<&UploadStage> = stages
        .iter()
        .filter(|stage| matches!(stage, UploadStage::Uploading { .. }))
        .collect();
    assert!(!uploads.is_empty());
    assert_eq!(
        uploads.last().copied(),
        Some(&UploadStage::Uploading {
            sent_bytes: expected_size,
            total_bytes: expected_size,
        })
    );
    assert_eq!(
        stages.get(stages.len() - 2),
        Some(&UploadStage::AwaitingResult)
    );
    assert_eq!(stages.last(), Some(&UploadStage::Confirmed));
    // Confirmed follows AwaitingResult: "body sent" is never "sent".
    let awaiting = stages
        .iter()
        .position(|stage| *stage == UploadStage::AwaitingResult)
        .expect("awaiting stage");
    let confirmed = stages
        .iter()
        .position(|stage| *stage == UploadStage::Confirmed)
        .expect("confirmed stage");
    assert!(awaiting < confirmed);
    let _ = std::fs::remove_file(path);
}

#[test]
fn send_upload_rejects_an_oversized_response_before_reading_it() {
    let path = temp_upload_file("oversize", b"tiny");
    // Announce more than the bound without ever sending the body: the
    // header check must refuse the response before it is read.
    let (address, server) = upload_http_server(Some(TELEGRAM_MAX_RESPONSE_BYTES + 1), Vec::new());
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let cancellation = CancellationToken::new();

    let mut request = sample_upload_request();
    request.file_path = path.clone();
    request.expected_size = None;
    let error = block_on(transport.send_upload(
        &token,
        &request,
        &UploadTimeouts::default(),
        &cancellation,
        |_| {},
    ))
    .expect_err("oversized response must be refused");
    assert_eq!(error, TelegramError::ResponseTooLarge);

    let _ = server.join();
    let _ = std::fs::remove_file(path);
}

#[test]
fn send_upload_classifies_a_bot_api_error_result() {
    let path = temp_upload_file("apierror", b"payload");
    let body = br#"{"ok":false,"error_code":400,"description":"chat not found"}"#.to_vec();
    let (address, server) = upload_http_server(None, body);
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let cancellation = CancellationToken::new();

    let mut request = sample_upload_request();
    request.file_path = path.clone();
    request.expected_size = None;
    let error = block_on(transport.send_upload(
        &token,
        &request,
        &UploadTimeouts::default(),
        &cancellation,
        |_| {},
    ))
    .expect_err("ok:false must surface as an API error");
    assert_eq!(
        error,
        TelegramError::Api {
            code: 400,
            description: "chat not found".into(),
            parameters: None,
        }
    );

    let _ = server.join();
    let _ = std::fs::remove_file(path);
}

#[test]
fn send_upload_attempt_reports_confirmed_ids_and_honest_progress() {
    let payload = b"media-bytes";
    let path = temp_upload_file("attempt", payload);
    let expected_size = payload.len() as u64;

    // Success: the confirmed message id and per-item file ids are extracted.
    let body = br#"{"ok":true,"result":{"message_id":77,"photo":{"file_id":"p1"}}}"#.to_vec();
    let (address, server) = upload_http_server(None, body);
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let mut request = sample_upload_request();
    request.file_path = path.clone();
    request.expected_size = Some(expected_size);
    let success = block_on(transport.send_upload_attempt(
        &token,
        &request,
        &UploadTimeouts::default(),
        &CancellationToken::new(),
        |_| {},
    ))
    .expect("attempt");
    assert_eq!(success.telegram_message_id, "77");
    let results = success.results_json.expect("file results");
    assert!(results.contains("p1"), "{results}");
    assert!(results.contains("77"), "{results}");
    let _ = server.join();

    // A Bot API rejection arrives after the body was fully sent, so the
    // progress must say so instead of looking unsent.
    let body = br#"{"ok":false,"error_code":400,"description":"chat not found"}"#.to_vec();
    let (address, server) = upload_http_server(None, body);
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let failure = block_on(transport.send_upload_attempt(
        &token,
        &request,
        &UploadTimeouts::default(),
        &CancellationToken::new(),
        |_| {},
    ))
    .expect_err("api error");
    assert_eq!(failure.progress, RequestProgress::Sent);
    assert!(matches!(
        failure.error,
        TelegramError::Api { code: 400, .. }
    ));
    let _ = server.join();
    let _ = std::fs::remove_file(path);
}

fn album_upload_request(path: &std::path::Path, file_name: &str) -> UploadRequest {
    UploadRequest {
        chat_id: "-1001".to_owned(),
        message_thread_id: Some(42),
        media_kind: MediaKind::Photo,
        file_path: path.to_path_buf(),
        file_name: file_name.to_owned(),
        mime_type: Some("image/jpeg".to_owned()),
        caption: None,
        expected_size: std::fs::metadata(path).map(|meta| meta.len()).ok(),
    }
}

#[test]
fn an_album_streams_every_file_and_reuses_a_cached_file_id() {
    let first = temp_upload_file("album-a", b"first-item-bytes");
    let second = temp_upload_file("album-b", b"second-item-bytes");
    let body = br#"{"ok":true,"result":[{"message_id":11,"photo":[{"file_id":"a1"}]},{"message_id":12,"photo":[{"file_id":"a2"}]}]}"#
        .to_vec();
    let (address, server) = upload_http_server(None, body);
    let transport = ReqwestTelegramTransport::with_test_endpoint(address).expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");

    let items = vec![
        AlbumItemUpload::File(Box::new(album_upload_request(&first, "album-a.jpg"))),
        // The second item reuses an identifier already cached for this bot.
        AlbumItemUpload::CachedFileId {
            media_kind: MediaKind::Photo,
            file_id: "cached-file-id".to_owned(),
            caption: Some("cached caption".to_owned()),
        },
    ];
    let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&stages);
    let success = block_on(send_media_group_attempt(
        &transport,
        &token,
        "-1001",
        Some(42),
        &items,
        &UploadTimeouts::default(),
        &CancellationToken::new(),
        move |stage| observed.lock().expect("stages").push(stage),
    ))
    .expect("album attempt");
    assert_eq!(success.telegram_message_id, "11", "first confirmed message");
    let results = success.results_json.expect("album results");
    let parsed_results: serde_json::Value =
        serde_json::from_str(&results).expect("structured album results");
    assert_eq!(
        parsed_results["message_ids"],
        serde_json::json!(["11", "12"])
    );
    assert_eq!(parsed_results["file_ids"], serde_json::json!(["a1", "a2"]));

    let received = server.join().expect("server");
    let request_text = String::from_utf8_lossy(&received).to_ascii_lowercase();
    assert!(request_text.contains("/bot1234:test/sendmediagroup"));
    let request = parse_multipart_request(&received).unwrap_or_else(|error| {
        panic!(
            "{error}; raw request: {}",
            String::from_utf8_lossy(&received)
        )
    });
    assert_eq!(request.fields["chat_id"], "-1001");
    assert_eq!(request.fields["message_thread_id"], "42");
    let media: serde_json::Value =
        serde_json::from_str(&request.fields["media"]).expect("media JSON field");
    assert_eq!(media.as_array().expect("media array").len(), 2);
    assert_eq!(media[0]["media"], "attach://media_0");
    assert_eq!(media[1]["media"], "cached-file-id");
    assert_eq!(media[1]["caption"], "cached caption");
    assert_eq!(request.files["media_0"].file_name, "album-a.jpg");
    assert_eq!(request.files["media_0"].bytes, b"first-item-bytes");

    // Progress is reported for the whole album, ending in Confirmed.
    let stages = stages.lock().expect("stages").clone();
    assert_eq!(stages.first(), Some(&UploadStage::Queued));
    assert_eq!(stages.last(), Some(&UploadStage::Confirmed));
    assert!(
        stages.iter().any(
            |stage| matches!(stage, UploadStage::Uploading { sent_bytes, .. } if *sent_bytes > 0)
        ),
        "the album body reported progress: {stages:?}"
    );

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
}

#[test]
fn an_album_outside_the_legal_range_is_refused_before_any_request() {
    let path = temp_upload_file("album-range", b"bytes");
    let transport =
        ReqwestTelegramTransport::with_test_endpoint("http://127.0.0.1:9").expect("transport");
    let token = BotToken::new("1234:TEST").expect("token");
    let request = album_upload_request(&path, "album.jpg");

    // One item is a single-media send, not an album.
    let single = vec![AlbumItemUpload::File(Box::new(request.clone()))];
    let error = block_on(send_media_group_attempt(
        &transport,
        &token,
        "-1001",
        None,
        &single,
        &UploadTimeouts::default(),
        &CancellationToken::new(),
        |_| {},
    ))
    .expect_err("one item is not an album");
    assert!(matches!(
        error.error,
        TelegramError::InvalidUploadRequest(_)
    ));
    assert_eq!(error.progress, RequestProgress::NotSent);

    // More than the album limit is refused as well.
    let too_many = (0..11)
        .map(|_| AlbumItemUpload::File(Box::new(request.clone())))
        .collect::<Vec<_>>();
    let error = block_on(send_media_group_attempt(
        &transport,
        &token,
        "-1001",
        None,
        &too_many,
        &UploadTimeouts::default(),
        &CancellationToken::new(),
        |_| {},
    ))
    .expect_err("eleven items is not an album");
    assert!(matches!(
        error.error,
        TelegramError::InvalidUploadRequest(_)
    ));

    // An item that cannot be read fails the album before any bytes move.
    let missing = album_upload_request(
        std::path::Path::new("/nonexistent/xarchive-tg-missing-album-item.jpg"),
        "missing.jpg",
    );
    let pair = vec![
        AlbumItemUpload::File(Box::new(request.clone())),
        AlbumItemUpload::File(Box::new(missing)),
    ];
    let error = block_on(send_media_group_attempt(
        &transport,
        &token,
        "-1001",
        None,
        &pair,
        &UploadTimeouts::default(),
        &CancellationToken::new(),
        |_| {},
    ))
    .expect_err("an unreadable item must fail the album");
    assert!(matches!(error.error, TelegramError::FileUnreadable(_)));
    assert_eq!(error.progress, RequestProgress::NotSent);
    let _ = std::fs::remove_file(path);
}
