//! Build a backend-neutral media transfer plan from a durable extraction
//! result.
//!
//! The plan is the only bridge between extraction and transfer: it carries
//! stable media identity, sanitized filenames and an allowlisted subset of
//! request headers. It never carries credentials and never describes
//! downloaded files.

use xarchive_protocol::{
    ExtractionMediaItem, ExtractionMediaType, ExtractionRequestHeader, ExtractionResult,
};

use crate::driver::{MediaTransferPlan, TransferPlanItem};
use crate::error::DownloadError;

/// Headers that may be forwarded from extraction into a media transfer.
/// Anything outside this allowlist is rejected instead of silently dropped.
pub const TRANSFER_HEADER_ALLOWLIST: [&str; 2] = ["accept", "referer"];

/// Build the aria2 transfer plan for one extraction result.
///
/// Rules:
/// - media items keep extraction order and unique stable identities;
/// - only allowlisted headers are forwarded, formatted as `Name: value`;
/// - filenames must be a single safe path component;
/// - an empty media list is valid (media-less tweets transfer nothing).
pub fn media_transfer_plan(
    result: &ExtractionResult,
    directory: impl Into<String>,
) -> Result<MediaTransferPlan, DownloadError> {
    let directory = directory.into();
    if directory.trim().is_empty() {
        return Err(DownloadError::InvalidTransferPlan(
            "transfer directory must not be empty".into(),
        ));
    }

    let headers = allowlisted_headers(&result.request_headers)?;
    let mut items = Vec::with_capacity(result.media.len());
    let mut seen = std::collections::HashSet::new();
    for media in &result.media {
        validate_media_item(media)?;
        let media_id = match &media.media_id {
            Some(id) if !id.trim().is_empty() => id.trim().to_owned(),
            _ => {
                return Err(DownloadError::InvalidTransferPlan(format!(
                    "media item {} has no stable media id",
                    media.index
                )));
            }
        };
        if !seen.insert(media_id.clone()) {
            return Err(DownloadError::InvalidTransferPlan(format!(
                "duplicate media id: {media_id}"
            )));
        }
        items.push(TransferPlanItem {
            media_id,
            url: media.url.clone(),
            filename: media.filename.clone(),
            directory: directory.clone(),
            headers: headers.clone(),
            media_type: match media.media_type {
                ExtractionMediaType::Photo => "photo",
                ExtractionMediaType::Video => "video",
                ExtractionMediaType::Unknown => "unknown",
            }
            .to_owned(),
            mime_type: media.mime_type.clone(),
        });
    }

    Ok(MediaTransferPlan { items })
}

fn validate_media_item(media: &ExtractionMediaItem) -> Result<(), DownloadError> {
    if !media.url.starts_with("http://") && !media.url.starts_with("https://") {
        return Err(DownloadError::InvalidTransferPlan(format!(
            "media item {} has a non-HTTP(S) URL",
            media.index
        )));
    }
    if media.filename.contains(['/', '\\'])
        || media.filename == "."
        || media.filename == ".."
        || media.filename.is_empty()
        || media.filename.bytes().any(|byte| byte < 0x20)
    {
        return Err(DownloadError::InvalidTransferPlan(format!(
            "media item {} has an unsafe filename: {:?}",
            media.index, media.filename
        )));
    }
    Ok(())
}

fn allowlisted_headers(headers: &[ExtractionRequestHeader]) -> Result<Vec<String>, DownloadError> {
    let mut formatted = Vec::new();
    for header in headers {
        let lowered = header.name.to_ascii_lowercase();
        if !TRANSFER_HEADER_ALLOWLIST.contains(&lowered.as_str()) {
            return Err(DownloadError::InvalidTransferPlan(format!(
                "request header is not allowlisted: {}",
                header.name
            )));
        }
        let name = header.name.trim();
        let value = header.value.trim();
        if name.is_empty() || value.is_empty() || value.contains('\n') || value.contains('\r') {
            return Err(DownloadError::InvalidTransferPlan(format!(
                "request header is malformed: {}",
                header.name
            )));
        }
        formatted.push(format!("{name}: {value}"));
    }
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use xarchive_protocol::{ExtractionMediaType, ExtractionRequestHeader};

    use super::*;

    fn extraction_result() -> ExtractionResult {
        ExtractionResult {
            tweet_id: "12345".into(),
            url: "https://x.com/user/status/12345".into(),
            tweet_type: "post".into(),
            text: None,
            username: None,
            display_name: None,
            created_at: None,
            user_id: None,
            reply_to: None,
            quoted_tweet: None,
            media: vec![
                xarchive_protocol::ExtractionMediaItem {
                    index: 1,
                    media_id: Some("media-1".into()),
                    media_type: ExtractionMediaType::Video,
                    url: "https://cdn.example/1.mp4".into(),
                    filename: "01.mp4".into(),
                    mime_type: Some("video/mp4".into()),
                },
                xarchive_protocol::ExtractionMediaItem {
                    index: 2,
                    media_id: Some("media-2".into()),
                    media_type: ExtractionMediaType::Photo,
                    url: "https://cdn.example/2.jpg".into(),
                    filename: "02.jpg".into(),
                    mime_type: None,
                },
            ],
            request_headers: vec![
                ExtractionRequestHeader {
                    name: "Referer".into(),
                    value: "https://x.com/".into(),
                },
                ExtractionRequestHeader {
                    name: "Accept".into(),
                    value: "*/*".into(),
                },
            ],
        }
    }

    #[test]
    fn builds_plan_with_allowlisted_headers() {
        let plan = media_transfer_plan(&extraction_result(), "/tmp/staging").expect("plan");
        assert_eq!(plan.items.len(), 2);
        assert_eq!(plan.items[0].media_id, "media-1");
        assert_eq!(plan.items[0].filename, "01.mp4");
        assert_eq!(plan.items[0].directory, "/tmp/staging");
        assert!(
            plan.items[0]
                .headers
                .iter()
                .all(|header| header.starts_with("Referer:") || header.starts_with("Accept:"))
        );
        plan.validate().expect("driver accepts the plan");
    }

    #[test]
    fn rejects_non_allowlisted_and_malformed_headers() {
        let mut result = extraction_result();
        result.request_headers.push(ExtractionRequestHeader {
            name: "Cookie".into(),
            value: "auth_token=secret".into(),
        });
        assert!(matches!(
            media_transfer_plan(&result, "/tmp/staging"),
            Err(DownloadError::InvalidTransferPlan(_))
        ));

        let mut result = extraction_result();
        result.request_headers[0].value = "https://x.com/\r\nX-Evil: 1".into();
        assert!(matches!(
            media_transfer_plan(&result, "/tmp/staging"),
            Err(DownloadError::InvalidTransferPlan(_))
        ));
    }

    #[test]
    fn rejects_missing_media_id_and_unsafe_filename() {
        let mut result = extraction_result();
        result.media[0].media_id = None;
        assert!(matches!(
            media_transfer_plan(&result, "/tmp/staging"),
            Err(DownloadError::InvalidTransferPlan(_))
        ));

        let mut result = extraction_result();
        result.media[1].filename = "../escape.jpg".into();
        assert!(matches!(
            media_transfer_plan(&result, "/tmp/staging"),
            Err(DownloadError::InvalidTransferPlan(_))
        ));
    }

    #[test]
    fn rejects_empty_directory_and_allows_media_less_result() {
        assert!(matches!(
            media_transfer_plan(&extraction_result(), "  "),
            Err(DownloadError::InvalidTransferPlan(_))
        ));

        let mut result = extraction_result();
        result.media.clear();
        let plan = media_transfer_plan(&result, "/tmp/staging").expect("media-less plan");
        assert!(plan.items.is_empty());
    }
}
