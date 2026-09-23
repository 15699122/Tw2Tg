//! Versioned cross-process protocol primitives.

mod browser;
mod error;
mod jsonl;
mod media;
mod sidecar_v2;

pub const PROTOCOL_VERSION: u32 = 1;
pub const BROWSER_PROTOCOL_VERSION: u32 = PROTOCOL_VERSION;
pub const SIDECAR_PROTOCOL_VERSION: u32 = 2;

/// Environment variable that overrides the Desktop transport endpoint.
///
/// Both the Desktop runtime and the Native Host read this variable. It is a
/// diagnostic/experimental override; production wiring uses each platform's
/// default endpoint.
pub const PIPE_ENDPOINT_ENV: &str = "XARCHIVE_PIPE_ENDPOINT";

/// Default Windows Named Pipe endpoint shared by the Desktop transport
/// server and the Native Host client.
///
/// The Windows Owner implements the Desktop listener against this contract
/// (roadmap E5); the Native Host connects here when [`PIPE_ENDPOINT_ENV`] is
/// unset.
pub const WINDOWS_PIPE_ENDPOINT: &str = r"\\.\pipe\xarchive-v1";

pub use browser::{
    BrowserArchiveStatus, BrowserRequest, BrowserResponse, BrowserTweet, extract_tweet_id,
};
pub use error::ProtocolError;
pub use jsonl::{decode_json_line, encode_json_line, read_json_lines, write_json_line};
pub use media::DownloadFile;
pub use sidecar_v2::{
    ExtractionMediaItem, ExtractionMediaType, ExtractionRequestHeader, ExtractionResult,
    REQUIRED_V2_CAPABILITIES, SidecarV2Capability, SidecarV2Command, SidecarV2CommandType,
    SidecarV2Error, SidecarV2Event, SidecarV2EventType, has_required_capabilities,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolEnvelope {
    pub protocol_version: u32,
    pub request_id: String,
}

impl ProtocolEnvelope {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_current_protocol_envelope() {
        let envelope = ProtocolEnvelope::new("request-1");
        assert_eq!(envelope.protocol_version, 1);
        assert_eq!(envelope.request_id, "request-1");
    }

    #[test]
    fn pins_shared_transport_endpoint_contract() {
        assert_eq!(PIPE_ENDPOINT_ENV, "XARCHIVE_PIPE_ENDPOINT");
        assert_eq!(WINDOWS_PIPE_ENDPOINT, r"\\.\pipe\xarchive-v1");
    }

    #[test]
    fn round_trips_durable_download_file_facts() {
        let file = DownloadFile {
            relative_path: "01.jpg".into(),
            size_bytes: 5,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
        };
        let line = encode_json_line(&file).expect("encode JSONL");
        let decoded: DownloadFile = decode_json_line(line.trim_end()).expect("decode JSONL");
        assert_eq!(decoded, file);
    }

    #[test]
    fn decodes_multiple_download_file_lines_without_session_fields() {
        let input = concat!(
            "{\"relative_path\":\"01.jpg\",\"size_bytes\":5,\"media_type\":\"photo\",\"mime_type\":\"image/jpeg\"}\n",
            "{\"relative_path\":\"02.mp4\",\"size_bytes\":9,\"media_type\":\"video\"}\n"
        );
        let files: Vec<DownloadFile> = read_json_lines(std::io::Cursor::new(input))
            .collect::<Result<_, _>>()
            .expect("read files");
        assert_eq!(files[0].relative_path, "01.jpg");
        assert_eq!(files[0].mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(files[1].mime_type, None);
    }

    #[test]
    fn validates_browser_archive_request() {
        let request: BrowserRequest = serde_json::from_str(
            r#"{"message_type":"archive_request","protocol_version":1,"request_id":"r1","tweet":{"tweet_id":"123","url":"https://x.com/alice/status/123","tweet_type":"post"}}"#,
        )
        .expect("browser request");
        request.validate().expect("valid browser request");
    }

    #[test]
    fn validates_browser_archive_request_with_quoted_tweet() {
        let request: BrowserRequest = serde_json::from_str(
            r#"{"message_type":"archive_request","protocol_version":1,"request_id":"r1","tweet":{"tweet_id":"123","url":"https://x.com/alice/status/123","tweet_type":"quote","quoted_tweet":{"tweet_id":"987","url":"https://x.com/bob/status/987","tweet_type":"post","username":"bob","text":"original"}}}"#,
        )
        .expect("browser request with quoted tweet");
        request
            .validate()
            .expect("valid browser request with quote");

        let quoted = match &request {
            BrowserRequest::ArchiveRequest { tweet, .. } => &tweet.quoted_tweet,
            _ => panic!("wrong variant"),
        };
        let quoted = quoted.as_deref().expect("quoted tweet present");
        assert_eq!(quoted.tweet_id, "987");
        assert_eq!(quoted.username.as_deref(), Some("bob"));
    }

    #[test]
    fn rejects_invalid_nested_quoted_tweet() {
        let request: BrowserRequest = serde_json::from_str(
            r#"{"message_type":"archive_request","protocol_version":1,"request_id":"r1","tweet":{"tweet_id":"123","url":"https://x.com/alice/status/123","tweet_type":"quote","quoted_tweet":{"tweet_id":"abc","url":"https://x.com/bob/status/987","tweet_type":"post"}}}"#,
        )
        .expect("browser request");
        assert_eq!(request.validate(), Err(ProtocolError::InvalidTweetId));
    }

    #[test]
    fn round_trips_quoted_tweet_without_losing_nested_data() {
        let quoted = BrowserTweet {
            tweet_id: "987".into(),
            url: "https://x.com/bob/status/987".into(),
            username: Some("bob".into()),
            display_name: Some("Bob".into()),
            text: Some("original".into()),
            created_at: Some("2026-09-08T09:00:00Z".into()),
            tweet_type: "post".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        let tweet = BrowserTweet {
            tweet_id: "123".into(),
            url: "https://x.com/alice/status/123".into(),
            username: Some("alice".into()),
            display_name: Some("Alice".into()),
            text: Some("quoting".into()),
            created_at: Some("2026-09-08T10:00:00Z".into()),
            tweet_type: "quote".into(),
            reply_to: None,
            quoted_tweet: Some(Box::new(quoted)),
        };
        let json = serde_json::to_string(&tweet).expect("tweet JSON");
        assert!(json.contains("\"quoted_tweet\""));
        let decoded: BrowserTweet = serde_json::from_str(&json).expect("decoded");
        assert_eq!(decoded, tweet);
    }

    #[test]
    fn rejects_browser_request_with_wrong_url() {
        let request: BrowserRequest = serde_json::from_str(
            r#"{"message_type":"archive_request","protocol_version":1,"request_id":"r1","tweet":{"tweet_id":"123","url":"https://example.com/status/123"}}"#,
        )
        .expect("browser request");
        assert_eq!(request.validate(), Err(ProtocolError::InvalidTweetUrl));
    }

    #[test]
    fn round_trips_browser_error_response() {
        let response = BrowserResponse::Error {
            protocol_version: PROTOCOL_VERSION,
            request_id: Some("r1".into()),
            error_code: "NATIVE_PIPE_UNAVAILABLE".into(),
            error_message: "Named Pipe forwarding is not configured".into(),
            retryable: true,
        };
        let json = serde_json::to_string(&response).expect("response JSON");
        let decoded: BrowserResponse = serde_json::from_str(&json).expect("response");
        assert_eq!(decoded, response);
    }

    #[test]
    fn round_trips_browser_status_batch_response() {
        let response = BrowserResponse::ArchiveStatusBatch {
            protocol_version: PROTOCOL_VERSION,
            request_id: "status-1".into(),
            statuses: vec![
                BrowserArchiveStatus {
                    tweet_id: "123".into(),
                    job_id: Some("job-123".into()),
                    state: "COMPLETE".into(),
                    progress: None,
                },
                BrowserArchiveStatus {
                    tweet_id: "456".into(),
                    job_id: None,
                    state: "NOT_ARCHIVED".into(),
                    progress: None,
                },
            ],
        };
        let json = serde_json::to_string(&response).expect("response JSON");
        let decoded: BrowserResponse = serde_json::from_str(&json).expect("response");
        assert_eq!(decoded, response);
    }

    #[test]
    fn rejects_unknown_browser_fields() {
        let request: Result<BrowserRequest, _> = serde_json::from_str(
            r#"{"message_type":"query_status","protocol_version":1,"request_id":"r1","tweet_ids":["123"],"unexpected":true}"#,
        );
        assert!(request.is_err());

        let tweet: Result<BrowserTweet, _> = serde_json::from_str(
            r#"{"tweet_id":"123","url":"https://x.com/a/status/123","unexpected":true}"#,
        );
        assert!(tweet.is_err());
    }

    #[test]
    fn rejects_browser_request_boundaries() {
        let invalid_type: BrowserRequest = serde_json::from_str(
            r#"{"message_type":"archive_request","protocol_version":1,"request_id":"r1","tweet":{"tweet_id":"123","url":"https://x.com/alice/status/123","tweet_type":"thread"}}"#,
        )
        .expect("browser request");
        assert_eq!(
            invalid_type.validate(),
            Err(ProtocolError::InvalidTweetType)
        );

        let invalid_path: BrowserRequest = serde_json::from_str(
            r#"{"message_type":"archive_request","protocol_version":1,"request_id":"r1","tweet":{"tweet_id":"123","url":"https://x.com/alice/likes/123"}}"#,
        )
        .expect("browser request");
        assert_eq!(invalid_path.validate(), Err(ProtocolError::InvalidTweetUrl));

        let too_many = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: (0..101).map(|id| id.to_string()).collect(),
        };
        assert_eq!(too_many.validate(), Err(ProtocolError::InvalidTweetId));

        let invalid_request_id = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r".repeat(129),
            tweet_ids: vec!["1".into()],
        };
        assert_eq!(
            invalid_request_id.validate(),
            Err(ProtocolError::InvalidRequestId)
        );
    }
}
