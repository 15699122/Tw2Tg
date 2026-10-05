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
    DownloadMedia,
    CancelActiveExtraction,
    StructuredMediaPlan,
    AccountDiscovery,
}

/// Required capabilities for every production v2 worker.
pub const REQUIRED_V2_CAPABILITIES: [SidecarV2Capability; 5] = [
    SidecarV2Capability::ExtractMedia,
    SidecarV2Capability::DownloadMedia,
    SidecarV2Capability::CancelActiveExtraction,
    SidecarV2Capability::StructuredMediaPlan,
    SidecarV2Capability::AccountDiscovery,
];

/// v2 worker commands. There is intentionally no v1 `download` fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarV2CommandType {
    Hello,
    Extract,
    Download,
    Discover,
    Cancel,
    Shutdown,
}

/// Typed v2 worker events. `extracted` carries a durable extraction result;
/// `candidate` carries one account-discovery candidate; `download_completed`
/// carries a durable full-download result written under the job staging
/// directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarV2EventType {
    Ready,
    ExtractionStarted,
    Extracted,
    DownloadStarted,
    DownloadCompleted,
    DiscoveryStarted,
    Candidate,
    DiscoveryCompleted,
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

/// One downloaded media file recorded by a gallery-dl full download.
///
/// The path stays relative to the job staging root: supervisor and archive
/// code re-anchor it before touching disk. Downloads never return absolute
/// paths, parent traversal, signed URLs or other session transfer state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadedMediaFile {
    pub index: u32,
    pub media_id: Option<String>,
    pub media_type: ExtractionMediaType,
    pub filename: String,
    pub relative_path: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Durable result produced by one gallery-dl full download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadResult {
    pub tweet_id: String,
    pub url: String,
    pub media: Vec<DownloadedMediaFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ExtractionResult>,
}

/// Quoted-tweet reference carried by one durable extraction result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionQuotedTweet {
    pub tweet_id: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tweet_type: Option<String>,
}

/// One tweet candidate produced by account discovery.
///
/// Discovery never yields media transfer facts: it only identifies durable
/// Tweet candidates (stable id, canonical-ish URL, creation time, media
/// presence) so Rust can persist, filter, and dispatch archive jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryCandidate {
    pub tweet_id: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub tweet_type: String,
    pub is_repost: bool,
    pub has_media: bool,
    pub media_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
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
    /// Stable numeric X user id of the tweet author, when the extractor
    /// exposes one. Never a username or a browser-derived placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Numeric id of the tweet this tweet replies to, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// Quoted-tweet reference, when the extractor exposes enough data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quoted_tweet: Option<ExtractionQuotedTweet>,
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
    /// Absolute job staging directory. Only `download` may carry it: the
    /// worker writes media bytes there directly, so both sides must agree on
    /// one directory. Extraction-only commands never receive it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging_dir: Option<String>,
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
    pub candidate: Option<DiscoveryCandidate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<DownloadResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates_found: Option<u64>,
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
            staging_dir: None,
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
            staging_dir: None,
        }
    }

    pub fn download(
        request_id: impl Into<String>,
        job_id: impl Into<String>,
        url: impl Into<String>,
        browser: Option<String>,
        profile: Option<String>,
        staging_dir: impl Into<String>,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: SidecarV2CommandType::Download,
            job_id: job_id.into(),
            url: Some(url.into()),
            browser,
            profile,
            staging_dir: Some(staging_dir.into()),
        }
    }

    pub fn discover(
        request_id: impl Into<String>,
        job_id: impl Into<String>,
        profile_url: impl Into<String>,
        browser: Option<String>,
        profile: Option<String>,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: SidecarV2CommandType::Discover,
            job_id: job_id.into(),
            url: Some(profile_url.into()),
            browser,
            profile,
            staging_dir: None,
        }
    }

    /// Validate framing plus command-specific identity rules.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)?;
        validate_request_id(&self.request_id)?;
        validate_job_id(&self.job_id)?;

        match self.cmd {
            SidecarV2CommandType::Hello | SidecarV2CommandType::Shutdown => {
                if self.url.is_some() || self.staging_dir.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Command);
                }
            }
            SidecarV2CommandType::Extract => {
                let url = self.url.as_deref().ok_or(ProtocolError::InvalidTweetUrl)?;
                extract_tweet_id(url).ok_or(ProtocolError::InvalidTweetUrl)?;
                if self.job_id == "system" {
                    return Err(ProtocolError::InvalidSidecarV2Identity);
                }
                if self.staging_dir.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Command);
                }
            }
            SidecarV2CommandType::Download => {
                let url = self.url.as_deref().ok_or(ProtocolError::InvalidTweetUrl)?;
                extract_tweet_id(url).ok_or(ProtocolError::InvalidTweetUrl)?;
                if self.job_id == "system" {
                    return Err(ProtocolError::InvalidSidecarV2Identity);
                }
                // The worker writes bytes directly, so the command must name
                // the absolute staging directory both sides agree on.
                let staging = self
                    .staging_dir
                    .as_deref()
                    .ok_or(ProtocolError::InvalidSidecarV2Command)?;
                let staging_path = std::path::Path::new(staging);
                if staging.is_empty()
                    || staging.len() > 4096
                    || !staging_path.is_absolute()
                    || staging.contains('\0')
                {
                    return Err(ProtocolError::InvalidSidecarV2Command);
                }
            }
            SidecarV2CommandType::Discover => {
                let profile_url = self.url.as_deref().ok_or(ProtocolError::InvalidTweetUrl)?;
                if !is_profile_url(profile_url) {
                    return Err(ProtocolError::InvalidTweetUrl);
                }
                if self.job_id == "system" {
                    return Err(ProtocolError::InvalidSidecarV2Identity);
                }
                if self.staging_dir.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Command);
                }
            }
            SidecarV2CommandType::Cancel => {
                if self.job_id.is_empty() || self.job_id.len() > 128 {
                    return Err(ProtocolError::InvalidSidecarV2Identity);
                }
                if self.staging_dir.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Command);
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
            download: None,
            candidate: None,
            candidates_found: None,
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
            download: None,
            candidate: None,
            candidates_found: None,
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn discovery_started(job_id: impl Into<String>, request_id: Option<String>) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::DiscoveryStarted,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result: None,
            download: None,
            candidate: None,
            candidates_found: None,
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn candidate(
        job_id: impl Into<String>,
        request_id: Option<String>,
        candidate: DiscoveryCandidate,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::Candidate,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result: None,
            download: None,
            candidate: Some(candidate),
            candidates_found: None,
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn discovery_completed(
        job_id: impl Into<String>,
        request_id: Option<String>,
        candidates_found: u64,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::DiscoveryCompleted,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result: None,
            download: None,
            candidate: None,
            candidates_found: Some(candidates_found),
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
            download: None,
            candidate: None,
            candidates_found: None,
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn download_started(job_id: impl Into<String>, request_id: Option<String>) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::DownloadStarted,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result: None,
            download: None,
            candidate: None,
            candidates_found: None,
            error_code: None,
            error_message: None,
            level: None,
            message: None,
        }
    }

    pub fn download_completed(
        job_id: impl Into<String>,
        request_id: Option<String>,
        download: DownloadResult,
        result: Option<ExtractionResult>,
    ) -> Self {
        Self {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            event: SidecarV2EventType::DownloadCompleted,
            job_id: job_id.into(),
            request_id,
            capabilities: None,
            result,
            download: Some(download),
            candidate: None,
            candidates_found: None,
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
            download: None,
            candidate: None,
            candidates_found: None,
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
        // New discovery fields must not ride on unrelated event types.
        if self.event != SidecarV2EventType::Candidate && self.candidate.is_some() {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }
        if self.event != SidecarV2EventType::DiscoveryCompleted && self.candidates_found.is_some() {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }
        if self.event != SidecarV2EventType::DownloadCompleted && self.download.is_some() {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }

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
            | SidecarV2EventType::DiscoveryStarted
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
            SidecarV2EventType::DownloadStarted => {
                if self.result.is_some() || self.capabilities.is_some() || self.download.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Event);
                }
                reject_success_error_fields(self)?;
            }
            SidecarV2EventType::DownloadCompleted => {
                let download = self
                    .download
                    .as_ref()
                    .ok_or(ProtocolError::InvalidSidecarV2Event)?;
                download.validate()?;
                if let Some(result) = self.result.as_ref() {
                    result.validate()?;
                }
                reject_success_error_fields(self)?;
            }
            SidecarV2EventType::Candidate => {
                let candidate = self
                    .candidate
                    .as_ref()
                    .ok_or(ProtocolError::InvalidSidecarV2Event)?;
                candidate.validate()?;
                if self.result.is_some() || self.capabilities.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Event);
                }
                reject_success_error_fields(self)?;
            }
            SidecarV2EventType::DiscoveryCompleted => {
                match self.candidates_found {
                    Some(count) if count <= 1_000_000 => {}
                    _ => return Err(ProtocolError::InvalidSidecarV2Event),
                }
                if self.result.is_some() || self.capabilities.is_some() {
                    return Err(ProtocolError::InvalidSidecarV2Event);
                }
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
        if let Some(user_id) = &self.user_id {
            validate_numeric_id(user_id).map_err(|_| ProtocolError::InvalidSidecarV2Event)?;
        }
        if let Some(reply_to) = &self.reply_to {
            validate_numeric_id(reply_to).map_err(|_| ProtocolError::InvalidSidecarV2Event)?;
        }
        if let Some(quoted) = &self.quoted_tweet {
            validate_extraction_quoted_tweet(quoted)?;
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

impl DownloadResult {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_numeric_id(&self.tweet_id).map_err(|_| ProtocolError::InvalidTweetId)?;
        if extract_tweet_id(&self.url) != Some(self.tweet_id.as_str()) {
            return Err(ProtocolError::InvalidSidecarV2Identity);
        }
        // A tweet without media legitimately downloads zero files; the archive
        // runner cross-checks emptiness against the extraction media list.
        if self.media.len() > 64 {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }

        let mut expected_index = 1_u32;
        for file in &self.media {
            if file.index != expected_index {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            expected_index = expected_index.saturating_add(1);
            if !is_safe_filename(&file.filename) {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            validate_relative_media_path(&file.relative_path)?;
            if let Some(media_id) = &file.media_id
                && !is_safe_media_identity(media_id)
            {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            if file.size_bytes == 0 || file.size_bytes > 4 * 1024 * 1024 * 1024 {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
            if let Some(mime_type) = &file.mime_type
                && (mime_type.is_empty() || mime_type.len() > 128)
            {
                return Err(ProtocolError::InvalidSidecarV2Event);
            }
        }

        if let Some(result) = &self.result {
            if result.tweet_id != self.tweet_id {
                return Err(ProtocolError::InvalidSidecarV2Identity);
            }
            result.validate()?;
        }
        Ok(())
    }
}

impl DiscoveryCandidate {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_numeric_id(&self.tweet_id)?;
        if self.url.is_empty()
            || self.url.len() > 2048
            || !(self.url.starts_with("https://") || self.url.starts_with("http://"))
        {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }
        if !self.url.contains(&self.tweet_id) {
            return Err(ProtocolError::InvalidSidecarV2Identity);
        }
        if !matches!(
            self.tweet_type.as_str(),
            "post" | "reply" | "quote" | "retweet"
        ) {
            return Err(ProtocolError::InvalidTweetType);
        }
        if self.media_count > 64 {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }
        if let Some(user_id) = &self.user_id {
            validate_numeric_id(user_id).map_err(|_| ProtocolError::InvalidSidecarV2Event)?;
        }
        if let Some(username) = &self.username
            && (username.is_empty() || username.len() > 64)
        {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }
        Ok(())
    }
}

fn validate_extraction_quoted_tweet(quoted: &ExtractionQuotedTweet) -> Result<(), ProtocolError> {
    validate_numeric_id(&quoted.tweet_id)?;
    if quoted.url.is_empty()
        || quoted.url.len() > 2048
        || !(quoted.url.starts_with("https://") || quoted.url.starts_with("http://"))
    {
        return Err(ProtocolError::InvalidSidecarV2Event);
    }
    if !quoted.url.contains(&quoted.tweet_id) {
        return Err(ProtocolError::InvalidSidecarV2Identity);
    }
    if let Some(url_tweet_id) = extract_tweet_id(&quoted.url)
        && url_tweet_id != quoted.tweet_id
    {
        return Err(ProtocolError::InvalidSidecarV2Identity);
    }
    if let Some(user_id) = &quoted.user_id {
        validate_numeric_id(user_id).map_err(|_| ProtocolError::InvalidSidecarV2Event)?;
    }
    if let Some(tweet_type) = &quoted.tweet_type
        && !matches!(tweet_type.as_str(), "post" | "reply" | "quote")
    {
        return Err(ProtocolError::InvalidTweetType);
    }
    Ok(())
}

fn validate_numeric_id(value: &str) -> Result<(), ProtocolError> {
    if value.is_empty() || value.len() > 32 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ProtocolError::InvalidTweetId);
    }
    Ok(())
}

/// Validate an account profile URL (`https://x.com/<username>`).
fn is_profile_url(url: &str) -> bool {
    for prefix in ["https://x.com/", "https://twitter.com/"] {
        if let Some(rest) = url.strip_prefix(prefix) {
            let rest = rest.trim_end_matches('/');
            return !rest.is_empty()
                && rest.len() <= 32
                && !rest.contains(['?', '#', '/'])
                && rest.chars().all(|character| {
                    character.is_ascii_alphanumeric() || character == '_' || character == '-'
                });
        }
    }
    false
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

/// Validate a media identity token.
///
/// Media identity is not a numeric X user or tweet id: gallery-dl's stable
/// identity can be a media key or a URL file-name segment, so this only
/// enforces the same bounded, traversal-free, secret-free shape the extraction
/// media item already requires.
fn is_safe_media_identity(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 || looks_like_secret_value(value) {
        return false;
    }
    !value.contains(['/', '\\', '\0'])
}

/// Validate a staging-relative media path: forward-slash separated, no
/// absolute roots, no drive letters, no parent traversal, bounded depth.
fn validate_relative_media_path(path: &str) -> Result<(), ProtocolError> {
    if path.is_empty() || path.len() > 512 {
        return Err(ProtocolError::InvalidSidecarV2Event);
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(ProtocolError::InvalidSidecarV2Event);
    }
    if path.contains('\\') || path.contains('\0') {
        return Err(ProtocolError::InvalidSidecarV2Event);
    }
    if path.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && path[1..].starts_with(':') {
        return Err(ProtocolError::InvalidSidecarV2Event);
    }
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() > 8 {
        return Err(ProtocolError::InvalidSidecarV2Event);
    }
    for segment in segments {
        if !is_safe_filename(segment) {
            return Err(ProtocolError::InvalidSidecarV2Event);
        }
    }
    Ok(())
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

    /// A staging directory that is absolute *on the platform running the test*.
    ///
    /// `Path::is_absolute` follows the host rules, so a hard-coded POSIX path is
    /// not absolute on Windows and the fixture would assert the wrong reason for
    /// rejection. Derived from the host temp directory so the fixture stays valid
    /// on every target.
    fn host_absolute_staging_dir() -> String {
        let path = std::env::temp_dir().join("job-1");
        assert!(
            path.is_absolute(),
            "the fixture must be an absolute path on this platform"
        );
        path.to_string_lossy().into_owned()
    }

    fn sample_result() -> ExtractionResult {
        ExtractionResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            tweet_type: "post".to_owned(),
            text: Some("hello".to_owned()),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            created_at: Some("2026-09-08T09:00:00Z".to_owned()),
            user_id: Some("9000".to_owned()),
            reply_to: Some("111".to_owned()),
            quoted_tweet: Some(ExtractionQuotedTweet {
                tweet_id: "987".to_owned(),
                url: "https://x.com/bob/status/987".to_owned(),
                username: Some("bob".to_owned()),
                display_name: Some("Bob".to_owned()),
                user_id: Some("9002".to_owned()),
                text: Some("original".to_owned()),
                created_at: None,
                tweet_type: Some("post".to_owned()),
            }),
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
    fn rejects_invalid_author_and_relationship_fields() {
        let mut result = sample_result();
        result.user_id = Some("not-numeric".to_owned());
        let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
        assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));

        let mut result = sample_result();
        result.reply_to = Some("12x".to_owned());
        let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
        assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));

        let mut result = sample_result();
        result.quoted_tweet.as_mut().expect("quoted").url =
            "https://x.com/bob/status/555".to_owned();
        let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
        assert_eq!(
            event.validate(),
            Err(ProtocolError::InvalidSidecarV2Identity)
        );

        let mut result = sample_result();
        result.quoted_tweet.as_mut().expect("quoted").user_id = Some("abc".to_owned());
        let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
        assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));
    }

    #[test]
    fn validates_discovery_command_and_candidate_events() {
        let command = SidecarV2Command::discover(
            "r1",
            "batch-1",
            "https://x.com/alice",
            Some("edge".to_owned()),
            None,
        );
        assert_eq!(command.validate(), Ok(()));

        let mut bad =
            SidecarV2Command::discover("r1", "batch-1", "https://example.com/x", None, None);
        assert_eq!(bad.validate(), Err(ProtocolError::InvalidTweetUrl));
        bad = SidecarV2Command::discover("r1", "system", "https://x.com/alice", None, None);
        assert_eq!(bad.validate(), Err(ProtocolError::InvalidSidecarV2Identity));

        let candidate = DiscoveryCandidate {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            created_at: Some("2026-09-01T00:00:00Z".to_owned()),
            tweet_type: "post".to_owned(),
            is_repost: false,
            has_media: true,
            media_count: 2,
            user_id: Some("9001".to_owned()),
            username: Some("alice".to_owned()),
        };
        let started = SidecarV2Event::discovery_started("batch-1", Some("r1".to_owned()));
        assert_eq!(started.validate(), Ok(()));
        let event = SidecarV2Event::candidate("batch-1", Some("r1".to_owned()), candidate);
        assert_eq!(event.validate(), Ok(()));
        let completed = SidecarV2Event::discovery_completed("batch-1", Some("r1".to_owned()), 42);
        assert_eq!(completed.validate(), Ok(()));

        // Candidate payload must not ride on unrelated events.
        let mut smuggled = SidecarV2Event::discovery_started("batch-1", Some("r1".to_owned()));
        smuggled.candidate = Some(DiscoveryCandidate {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            created_at: None,
            tweet_type: "post".to_owned(),
            is_repost: false,
            has_media: false,
            media_count: 0,
            user_id: None,
            username: None,
        });
        assert_eq!(
            smuggled.validate(),
            Err(ProtocolError::InvalidSidecarV2Event)
        );

        // Invalid candidate identity is rejected.
        let mut invalid = SidecarV2Event::candidate(
            "batch-1",
            Some("r1".to_owned()),
            DiscoveryCandidate {
                tweet_id: "12x".to_owned(),
                ..event.candidate.expect("candidate")
            },
        );
        invalid.candidate.as_mut().expect("candidate").tweet_id = "12x".to_owned();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn rejects_unknown_fields() {
        let input = "{\"protocol_version\":2,\"request_id\":\"r1\",\"cmd\":\"extract\",\"job_id\":\"j1\",\"url\":\"https://x.com/a/status/1\",\"executable\":\"x\"}";
        let parsed: Result<SidecarV2Command, _> = serde_json::from_str(input);
        assert!(parsed.is_err());
    }

    fn sample_download() -> DownloadResult {
        DownloadResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            media: vec![DownloadedMediaFile {
                index: 1,
                media_id: Some("media-01".to_owned()),
                media_type: ExtractionMediaType::Photo,
                filename: "01.jpg".to_owned(),
                relative_path: "01.jpg".to_owned(),
                size_bytes: 2048,
                mime_type: Some("image/jpeg".to_owned()),
            }],
            result: Some(sample_result()),
        }
    }

    #[test]
    fn validates_download_command_and_events() {
        let staging = std::env::temp_dir().join("xarchive-staging-job-1");
        let command = SidecarV2Command::download(
            "r1",
            "job-1",
            "https://x.com/alice/status/123",
            None,
            None,
            staging.display().to_string(),
        );
        assert_eq!(command.validate(), Ok(()));

        let started = SidecarV2Event::download_started("job-1", Some("r1".to_owned()));
        assert_eq!(started.validate(), Ok(()));
        let completed = SidecarV2Event::download_completed(
            "job-1",
            Some("r1".to_owned()),
            sample_download(),
            Some(sample_result()),
        );
        assert_eq!(completed.validate(), Ok(()));
    }

    #[test]
    fn download_command_requires_absolute_staging_dir() {
        // The positive case matters as much as the negative one: a validator that
        // rejected every staging directory would still satisfy every assertion
        // below and silently break all downloads.
        let accepted = SidecarV2Command::download(
            "r1",
            "job-1",
            "https://x.com/alice/status/123",
            None,
            None,
            host_absolute_staging_dir(),
        );
        assert_eq!(
            accepted.validate(),
            Ok(()),
            "a host-absolute staging directory must be accepted"
        );

        let mut command = SidecarV2Command::download(
            "r1",
            "job-1",
            "https://x.com/alice/status/123",
            None,
            None,
            "relative/job-1".to_owned(),
        );
        assert_eq!(
            command.validate(),
            Err(ProtocolError::InvalidSidecarV2Command)
        );
        command.staging_dir = None;
        assert_eq!(
            command.validate(),
            Err(ProtocolError::InvalidSidecarV2Command)
        );
        // Extraction-only commands never carry a staging directory.
        let mut extract =
            SidecarV2Command::extract("r1", "job-1", "https://x.com/alice/status/123", None, None);
        extract.staging_dir = Some(host_absolute_staging_dir());
        assert_eq!(
            extract.validate(),
            Err(ProtocolError::InvalidSidecarV2Command)
        );
    }

    #[test]
    fn rejects_unsafe_download_payloads_and_event_riding() {
        let mut escaping = sample_download();
        escaping.media[0].relative_path = "../escape.jpg".to_owned();
        let event = SidecarV2Event::download_completed(
            "job-1",
            Some("r1".to_owned()),
            escaping,
            Some(sample_result()),
        );
        assert!(event.validate().is_err());

        let mut absolute = sample_download();
        absolute.media[0].relative_path = "/abs.jpg".to_owned();
        assert!(absolute.validate().is_err());

        let mut traversal = sample_download();
        traversal.media[0].relative_path = "sub/../01.jpg".to_owned();
        assert!(traversal.validate().is_err());

        let mut empty = sample_download();
        empty.media[0].size_bytes = 0;
        assert!(empty.validate().is_err());

        // The download payload must not ride on an unrelated event.
        let mut smuggled = SidecarV2Event::download_started("job-1", Some("r1".to_owned()));
        smuggled.download = Some(sample_download());
        assert_eq!(
            smuggled.validate(),
            Err(ProtocolError::InvalidSidecarV2Event)
        );

        // download_completed requires a download payload.
        let mut missing = SidecarV2Event::download_completed(
            "job-1",
            Some("r1".to_owned()),
            DownloadResult {
                tweet_id: "123".to_owned(),
                url: "https://x.com/alice/status/123".to_owned(),
                media: Vec::new(),
                result: Some(sample_result()),
            },
            None,
        );
        missing.download = None;
        assert_eq!(
            missing.validate(),
            Err(ProtocolError::InvalidSidecarV2Event)
        );
    }

    #[test]
    fn shared_fixtures_deserialize_and_validate() {
        // The shared fixtures are the cross-language contract reference, so a
        // schema or capability change that forgets them must fail here instead
        // of drifting silently against the Rust types.
        for line in
            include_str!("../../../shared/protocol-schema/fixtures/sidecar-v2-command.jsonl")
                .lines()
                .filter(|line| !line.trim().is_empty())
        {
            let mut command: SidecarV2Command =
                serde_json::from_str(line).expect("fixture command parses");
            if let Some(staging_dir) = command.staging_dir.as_mut() {
                // The shared JSONL uses a POSIX example; instantiate only that
                // known fixture path for the host, preserving relative-path rejection.
                assert_eq!(staging_dir, "/tmp/xarchive-staging/job-1");
                *staging_dir = host_absolute_staging_dir();
            }
            assert_eq!(command.validate(), Ok(()), "fixture command validates");
        }
        for line in include_str!("../../../shared/protocol-schema/fixtures/sidecar-v2-events.jsonl")
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let event: SidecarV2Event = serde_json::from_str(line).expect("fixture event parses");
            assert_eq!(event.validate(), Ok(()), "fixture event validates");
        }
    }

    #[test]
    fn requires_download_media_capability_for_handshake() {
        let mut without_download = Vec::new();
        for capability in REQUIRED_V2_CAPABILITIES {
            if capability != SidecarV2Capability::DownloadMedia {
                without_download.push(capability);
            }
        }
        assert!(!has_required_capabilities(&without_download));
        assert!(has_required_capabilities(&REQUIRED_V2_CAPABILITIES));
    }
}
