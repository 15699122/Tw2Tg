//! Typed protocol v2 extraction contract shared by Rust and Python.
//!
//! The extraction result is durable metadata only: stable tweet and media
//! identity, sanitized text/profile fields, safe filenames, media transfer
//! types, and an allowlisted subset of HTTP headers. Signed URLs, cookie
//! jars, browser credential paths, filesystem roots, and aria2 transfer
//! state are absent: they belong to a session transfer plan.
use serde::{Deserialize, Serialize};

use crate::{ProtocolError, SIDECAR_PROTOCOL_VERSION, extract_tweet_id};

/// Capabilities advertised by the v2 `hello -> ready` handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarV2Capability {
    ExtractMedia,
    CancelActiveExtraction,
    StructuredMediaPlan,
}

/// Required capabilities for every production v2 worker.
pub const REQUIRED_V2_CAPABILITIES: [SidecarV2Capability; 3] = [
    SidecarV2Capability::ExtractMedia,
    SidecarV2Capability::CancelActiveExtraction,
    SidecarV2Capability::StructuredMediaPlan,
];

/// v2 worker commands. There is intentionally no v1 `download` fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarV2CommandType {
    Hello,
    Extract,
    Cancel,
    Shutdown,
}

/// Typed v2 worker events. `extracted` carries a durable extraction result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarV2EventType {
    Ready,
    ExtractionStarted,
    Extracted,
    Cancelled,
    Failed,
    Log,
}

/// Structured error payload reused by `failed` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidecarV2Error {
    pub error_code: String,
    pub error_message: String,
}

impl SidecarV2Error {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error_code: Sanitizer::redacted_code(code.into()),
            error_message: Sanitizer::bounded_message(message.into()),
        }
    }
}

/// One media reference produced by extraction-only work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionMediaItem {
    pub index: u32,
    pub media_id: Option<String>,
    pub media_type: ExtractionMediaType,
    pub url: String,
    pub filename: String,
    pub mime_type: Option<String>,
}

/// Media transfer classification used by the future aria2-only driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMediaType {
    Photo,
    Video,
    Unknown,
}

/// Request-scoped HTTP headers forwarded into a later media transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionRequestHeader {
    pub name: String,
    pub value: String,
}

/// Durable metadata returned by one v2 `extract` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionResult {
    pub tweet_id: String,
    pub url: String,
    pub tweet_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub media: Vec<ExtractionMediaItem>,
    #[serde(default)]
    pub request_headers: Vec<ExtractionRequestHeader>,
}

/// A v2 command. Unknown JSON fields are rejected at the schema boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidecarV2Command {
    pub protocol_version: u32,
    pub request_id: String,
    pub cmd: SidecarV2CommandType,
    pub job_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

/// A v2 event. Unknown JSON fields are rejected at the schema boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidecarV2Event {
    pub protocol_version: u32,
    pub event: SidecarV2EventType,
    pub job_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<SidecarV2Capability>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ExtractionResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SidecarV2Command {
    pub fn hello(request_id: impl Into<String>) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: SidecarV2CommandType::Hello,
            job_id: "system".to_owned(),
            url: None,
            browser: None,
            profile: None,
        }
    }

    pub fn extract(
        request_id: impl Into<String>,
        job_id: impl Into<String>,
        url: impl Into<String>,
        browser: Option<String>,
        profile: Option<String>,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: SidecarV2CommandType::Extract,
            job_id: job_id.into(),
            url: Some(url.into()),
            browser,
            profile,
        }
    }

    /// Validate framing plus command-specific identity rules.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)?;
        validate_request_id(&self.request_id)?;
        validate_job_id(&self.job_id)?;

        match self.cmd {
            SidecarV2CommandType::Hello | SidecarV2CommandType::Shutdown => {
                if self.url.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Command);
                }
            }
            SidecarV2CommandType::Extract => {
                let url = self.url.as_deref().ok_or(ProtocolError::InvalidTweetUrl)?;
                extract_tweet_id(url).ok_or(ProtocolError::InvalidTweetUrl)?;
                if self.job_id == "system" {
                    return Err(ProtocolError::InvalidSidecarV2Identity);
                }
            }
            SidecarV2CommandType::Cancel => {
                if self.job_id.is_empty() || self.job_id.len() > 128 {
                    return Err(ProtocolError::InvalidSidecarV2Identity);
                }
            }
        }

        if let Some(browser) = &self.browser
            && (browser.is_empty() || browser.len() > 64)
        {
            return Err(ProtocolError::InvalidSidecarV2Command);
        }
        if let Some(profile) = &self.profile
            && (profile.is_empty() || profile.len() > 128)
        {
            return Err(ProtocolError::InvalidSidecarV2Command);
        }
        Ok(())
    }
}

impl SidecarV2Event {
    pub fn ready(
        job_id: impl Into<String>,
        request_id: Option<String>,
        capabilities: Vec<SidecarV2Capability>,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::Ready,
            job_id: job_id.into(),
            request_id,
            capabilities: Some(capabilities),
            result: None,
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn extraction_started(job_id: impl Into<String>, request_id: Option<String>) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::ExtractionStarted,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result: None,
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn extracted(
        job_id: impl Into<String>,
        request_id: Option<String>,
        result: ExtractionResult,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::Extracted,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result: Some(result),
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn failed(
        job_id: impl Into<String>,
        request_id: Option<String>,
        error: SidecarV2Error,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::Failed,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result: None,
            error_code: Some(error.error_code),
            error_message: Some(error.error_message),
            level: None,
            message: None,
        }
    }

    /// Validate framing plus event-specific payload rules.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)?;
        if let Some(request_id) = &self.request_id {
            validate_request_id(request_id)?;
        }
        validate_job_id(&self.job_id)?;

        match self.event {
            SidecarV2EventType::Ready => {
                let capabilities = self
                    .capabilities
                    .as_deref()
                    .ok_or(ProtocolError::InvalidSidecarV2Capability)?;
                if !has_required_capabilities(capabilities) {
                    return Err(ProtocolError::InvalidSidecarV2Capability);
                }
                reject_success_error_fields(self)?;
            }
            SidecarV2EventType::ExtractionStarted
            | SidecarV2EventType::Cancelled
            | SidecarV2EventType::Log => {
                if self.result.is_some() || self.capabilities.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Event);
                }
                if self.event == SidecarV2EventType::Log {
                    match self.message.as_deref() {
                        Some(message) if !message.is_empty() && message.len() <= 4000 => {}
                        _ => return Err(ProtocolError::InvalidSidecarV2Event),
                    }
                    if let Some(level) = &self.level
                        && !matches!(level.as_str(), "debug" | "info" | "warning" | "error")
                    {
                        return Err(ProtocolError::InvalidSidecarV2Event);
                    }
                } else {
                    reject_success_error_fields(self)?;
                }
            }
            SidecarV2EventType::Extracted => {
                let result = self
                    .result
                    .as_ref()
                    .ok_or(ProtocolError::InvalidSidecarV2Event)?;
                result.validate()?;
                reject_success_error_fields(self)?;
            }
            SidecarV2EventType::Failed => {
                if self.result.is_some() || self.capabilities.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Event);
                }
                match (&self.error_code, &self.error_message) {
                    (Some(code), Some(message))
                        if !code.trim().is_empty()
                            && code.len() <= 64
                            && !message.trim().is_empty()
                            && message.len() <= 500 => {}
                    _ => return Err(ProtocolError::InvalidSidecarV2Event),
                }
            }
        }
        Ok(())
    }
}

impl ExtractionResult {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.tweet_id.is_empty()
            || !self.tweet_id.bytes().all(|byte| byte.is_ascii_digit())
            || self.tweet_id.len() > 32
        {
            return Err(ProtocolError::InvalidTweetId);
        }
        if extract_tweet_id(&self.url) != Some(self.tweet_id.as_str()) {
            return Err(ProtocolError::InvalidSidecarV2Identity);
        }
        if !matches!(self.tweet_type.as_str(), "post" | "reply" | "quote") {
            return Err(ProtocolError::InvalidTweetType);
        }
        if self.media.len() > 32 {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }

        let mut expected_index = 1_u32;
        for item in &self.media {
            if item.index != expected_index {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            expected_index = expected_index.saturating_add(1);
            if item.url.is_empty()
                || item.url.len() > 2048
                || !(item.url.starts_with("https://") || item.url.starts_with("http://"))
            {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            if !is_safe_filename(&item.filename) {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            if looks_like_secret_value(&item.url) {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
        }

        if self.request_headers.len() > 8 {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }
        for header in &self.request_headers {
            if !is_allowlisted_header_name(&header.name) {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            if header.value.is_empty() || header.value.len() > 2048 {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            if looks_like_secret_value(&header.value) {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
        }
        Ok(())
    }
}

pub fn has_required_capabilities(capabilities: &[SidecarV2Capability]) -> bool {
    REQUIRED_V2_CAPABILITIES
        .iter()
        .all(|required| capabilities.contains(required))
}

fn validate_protocol_version(version: u32) -> Result<(), ProtocolError> {
    if version != SIDECAR_PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedSidecarV2Version(version));
    }
    Ok(())
}

fn validate_request_id(request_id: &str) -> Result<(), ProtocolError> {
    if request_id.is_empty() || request_id.len() > 128 {
        return Err(ProtocolError::InvalidRequestId);
    }
    Ok(())
}

fn validate_job_id(job_id: &str) -> Result<(), ProtocolError> {
    if job_id.is_empty() || job_id.len() > 128 {
        return Err(ProtocolError::InvalidSidecarV2Identity);
    }
    Ok(())
}

fn reject_success_error_fields(event: &SidecarV2Event) -> Result<(), ProtocolError> {
    if event.error_code.is_some() || event.error_message.is_some() {
        return Err(ProtocolError::InvalidSidecarV2Event);
    }
    Ok(())
}

fn is_safe_filename(filename: &str) -> bool {
    if filename.is_empty() || filename.len() > 128 {
        return false;
    }
    if filename == "." || filename == ".." {
        return false;
    }
    if filename.contains(['/', '\\', '\0']) {
        return false;
    }
    if filename.starts_with('.') {
        return false;
    }
    if filename.contains("..") {
        return false;
    }
    true
}

fn is_allowlisted_header_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "referer" | "origin" | "user-agent" | "accept" | "accept-language"
    )
}

struct Sanitizer;

impl Sanitizer {
    fn redacted_code(code: String) -> String {
        let trimmed = code.trim();
        if trimmed.is_empty() || trimmed.len() > 64 {
            return "SIDECAR_INTERNAL_ERROR".to_owned();
        }
        if trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            trimmed.to_owned()
        } else {
            "SIDECAR_INTERNAL_ERROR".to_owned()
        }
    }

    fn bounded_message(message: String) -> String {
        let trimmed = message.trim();
        if trimmed.is_empty() {
            return "sidecar extraction failed".to_owned();
        }
        let mut bounded = trimmed.chars().take(500).collect::<String>();
        if looks_like_secret_value(&bounded) {
            bounded = "sidecar extraction failed".to_owned();
        }
        bounded
    }
}

fn looks_like_secret_value(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    for marker in ["cookie", "authorization", "bearer ", "secret", "token="] {
        if lower.contains(marker) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result() -> ExtractionResult {
        ExtractionResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            tweet_type: "post".to_owned(),
            text: Some("hello".to_owned()),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            created_at: Some("2026-09-08T09:00:00Z".to_owned()),
            media: vec![ExtractionMediaItem {
                index: 1,
                media_id: Some("m1".to_owned()),
                media_type: ExtractionMediaType::Photo,
                url: "https://cdn.example/1.jpg".to_owned(),
                filename: "01.jpg".to_owned(),
                mime_type: Some("image/jpeg".to_owned()),
            }],
            request_headers: vec![ExtractionRequestHeader {
                name: "Referer".to_owned(),
                value: "https://x.com/".to_owned(),
            }],
        }
    }

    #[test]
    fn accepts_valid_extract_command_and_event() {
        let command = SidecarV2Command::extract(
            "request-1",
            "job-1",
            "https://x.com/alice/status/123",
            Some("edge".to_owned()),
            None,
        );
        assert_eq!(command.validate(), Ok(()));
        let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), sample_result());
        assert_eq!(event.validate(), Ok(()));
    }

    #[test]
    fn rejects_missing_capabilities() {
        let event = SidecarV2Event::ready(
            "system",
            Some("r1".to_owned()),
            vec![SidecarV2Capability::ExtractMedia],
        );
        assert_eq!(
            event.validate(),
            Err(ProtocolError::InvalidSidecarV2Capability)
        );
    }

    #[test]
    fn rejects_identity_mismatch_and_cookie_headers() {
        let mut result = sample_result();
        result.tweet_id = "999".to_owned();
        let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
        assert_eq!(
            event.validate(),
            Err(ProtocolError::InvalidSidecarV2Identity)
        );

        let mut result = sample_result();
        result.request_headers = vec![ExtractionRequestHeader {
            name: "Cookie".to_owned(),
            value: "session=secret".to_owned(),
        }];
        let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
        assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));
    }

    #[test]
    fn rejects_unknown_fields() {
        let input = "{\"protocol_version\":2,\"request_id\":\"r1\",\"cmd\":\"extract\",\"job_id\":\"j1\",\"url\":\"https://x.com/a/status/1\",\"executable\":\"x\"}";
        let parsed: Result<SidecarV2Command, _> = serde_json::from_str(input);
        assert!(parsed.is_err());
    }
}
