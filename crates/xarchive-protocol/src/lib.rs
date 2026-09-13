//! Versioned cross-process protocol primitives.

mod browser;
mod error;
mod jsonl;
mod sidecar;

pub const PROTOCOL_VERSION: u32 = 1;

pub use browser::{BrowserRequest, BrowserResponse, BrowserTweet, extract_tweet_id};
pub use error::ProtocolError;
pub use jsonl::{decode_json_line, encode_json_line, read_json_lines, write_json_line};
pub use sidecar::{
    DownloadEvent, DownloadEventType, DownloadFile, MessageType, SidecarCommand, SidecarCommandType,
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
    fn round_trips_sidecar_command_as_jsonl() {
        let command = SidecarCommand {
            protocol_version: PROTOCOL_VERSION,
            request_id: "request-1".into(),
            cmd: SidecarCommandType::Download,
            job_id: "job-1".into(),
            url: Some("https://x.com/example/status/1".into()),
            staging_dir: Some("/tmp/staging/job-1".into()),
            browser: None,
            profile: None,
        };
        let mut output = Vec::new();
        write_json_line(&mut output, &command).expect("write JSONL");
        let decoded: Vec<SidecarCommand> = read_json_lines(std::io::Cursor::new(output))
            .collect::<Result<_, _>>()
            .expect("read JSONL");
        assert_eq!(decoded, vec![command]);
    }

    #[test]
    fn decodes_file_and_complete_events() {
        let input = concat!(
            "{\"protocol_version\":1,\"event\":\"file\",\"job_id\":\"job-1\",\"path\":\"01.jpg\",\"size_bytes\":5,\"media_type\":\"photo\",\"mime_type\":\"image/jpeg\"}\n",
            "{\"protocol_version\":1,\"event\":\"complete\",\"job_id\":\"job-1\",\"files\":[{\"relative_path\":\"01.jpg\",\"size_bytes\":5,\"media_type\":\"photo\",\"mime_type\":\"image/jpeg\"}]}\n"
        );
        let events: Vec<DownloadEvent> = read_json_lines(std::io::Cursor::new(input))
            .collect::<Result<_, _>>()
            .expect("read events");
        assert_eq!(events[0].path.as_deref(), Some("01.jpg"));
        assert_eq!(events[0].size_bytes, Some(5));
        assert_eq!(events[1].files.as_ref().expect("files").len(), 1);
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
        };
        let json = serde_json::to_string(&response).expect("response JSON");
        let decoded: BrowserResponse = serde_json::from_str(&json).expect("response");
        assert_eq!(decoded, response);
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
