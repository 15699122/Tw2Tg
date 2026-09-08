//! Versioned cross-process protocol primitives.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

pub const PROTOCOL_VERSION: u32 = 1;

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
}
