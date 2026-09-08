//! Versioned cross-process protocol primitives.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum BrowserRequest {
    ArchiveRequest {
        protocol_version: u32,
        request_id: String,
        tweet: BrowserTweet,
    },
    QueryStatus {
        protocol_version: u32,
        request_id: String,
        tweet_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum BrowserResponse {
    ArchiveStatus {
        protocol_version: u32,
        request_id: String,
        tweet_id: String,
        job_id: Option<String>,
        state: String,
        progress: Option<serde_json::Value>,
    },
    Error {
        protocol_version: u32,
        request_id: Option<String>,
        error_code: String,
        error_message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTweet {
    pub tweet_id: String,
    pub url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default = "default_tweet_type")]
    pub tweet_type: String,
    #[serde(default)]
    pub reply_to: Option<String>,
}

fn default_tweet_type() -> String {
    "post".to_owned()
}

impl BrowserRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let (protocol_version, request_id) = match self {
            Self::ArchiveRequest {
                protocol_version,
                request_id,
                tweet,
            } => {
                if tweet.tweet_id.is_empty() || !tweet.tweet_id.chars().all(|c| c.is_ascii_digit())
                {
                    return Err(ProtocolError::InvalidTweetId);
                }
                if !tweet.url.starts_with("https://x.com/")
                    && !tweet.url.starts_with("https://twitter.com/")
                {
                    return Err(ProtocolError::InvalidTweetUrl);
                }
                if !tweet.url.contains("/status/") && !tweet.url.contains("/statuses/") {
                    return Err(ProtocolError::InvalidTweetUrl);
                }
                if !matches!(tweet.tweet_type.as_str(), "post" | "reply" | "quote") {
                    return Err(ProtocolError::InvalidTweetType);
                }
                (protocol_version, request_id)
            }
            Self::QueryStatus {
                protocol_version,
                request_id,
                tweet_ids,
            } => {
                if tweet_ids.is_empty()
                    || tweet_ids.len() > 100
                    || tweet_ids
                        .iter()
                        .any(|id| id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()))
                {
                    return Err(ProtocolError::InvalidTweetId);
                }
                (protocol_version, request_id)
            }
        };
        if *protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(*protocol_version));
        }
        if request_id.is_empty() || request_id.len() > 128 {
            return Err(ProtocolError::InvalidRequestId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidTweetId,
    InvalidTweetUrl,
    UnsupportedVersion(u32),
    InvalidRequestId,
    InvalidTweetType,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTweetId => {
                formatter.write_str("tweet_id must be a non-empty decimal string")
            }
            Self::InvalidTweetUrl => formatter.write_str("tweet URL must use x.com or twitter.com"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version: {version}")
            }
            Self::InvalidRequestId => formatter.write_str("request_id must be 1-128 characters"),
            Self::InvalidTweetType => {
                formatter.write_str("tweet_type must be post, reply, or quote")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    ArchiveRequest,
    QueryStatus,
    ArchiveStatus,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarCommandType {
    Hello,
    Download,
    Cancel,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadEventType {
    Ready,
    Started,
    Metadata,
    Progress,
    File,
    Complete,
    Failed,
    Log,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarCommand {
    pub protocol_version: u32,
    pub request_id: String,
    pub cmd: SidecarCommandType,
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staging_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadEvent {
    pub protocol_version: u32,
    pub event: DownloadEventType,
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<DownloadFile>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadFile {
    pub relative_path: String,
    pub size_bytes: u64,
    pub media_type: String,
    #[serde(default)]
    pub mime_type: Option<String>,
}

pub fn encode_json_line<T: Serialize>(value: &T) -> serde_json::Result<String> {
    serde_json::to_string(value)
}

pub fn decode_json_line<T: for<'de> Deserialize<'de>>(line: &str) -> serde_json::Result<T> {
    serde_json::from_str(line)
}

pub fn write_json_line<T: Serialize, W: Write>(writer: &mut W, value: &T) -> io::Result<()> {
    let encoded = encode_json_line(value).map_err(io::Error::other)?;
    writer.write_all(encoded.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

pub fn read_json_lines<R: BufRead, T: for<'de> Deserialize<'de>>(
    reader: R,
) -> impl Iterator<Item = serde_json::Result<T>> {
    reader.lines().map(|line| match line {
        Ok(line) => decode_json_line(&line),
        Err(error) => Err(serde_json::Error::io(error)),
    })
}

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
