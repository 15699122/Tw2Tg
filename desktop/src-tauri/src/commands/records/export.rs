//! Record export: batch and single-record serialization.
//!
//! Split out of `commands/records.rs` without changing any command name,
//! argument shape, serialized field or error string. The rules:
//!
//! * export enumerates every candidate — the UI page limit must never truncate
//!   an exchange document;
//! * a record carrying a private URL or a non-portable media path is refused
//!   instead of being silently rewritten, because rewriting it would break
//!   cross-format idempotency;
//! * both formats go through the same versioned core serializer.

use std::path::Path;

use xarchive_core::DownloadRecord;
use xarchive_storage::Database;

use super::serialize_persisted_record;

/// Serialize every candidate of one batch as a download or archive document.
pub(super) fn serialize_batch_record_export(
    database: &Database,
    batch_id: &str,
    format: &str,
    kind: Option<&str>,
    archive_root: Option<&str>,
) -> Result<String, String> {
    if !matches!(kind, None | Some("download") | Some("archive")) {
        return Err("record kind must be download or archive".into());
    }
    database
        .account_batch(batch_id)
        .map_err(|e| e.to_string())?
        .ok_or("batch not found")?;
    // No UI-page limit: export every candidate with canonical archived metadata.
    let candidates = database
        .list_batch_candidates(batch_id, None, u32::MAX)
        .map_err(|e| e.to_string())?;
    let mut downloads = Vec::new();
    let mut archives = Vec::new();
    for candidate in candidates {
        if database
            .merged_metadata_json(&candidate.tweet_id)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            continue;
        }
        let raw =
            serialize_persisted_record(database, &candidate.tweet_id, "json", kind, archive_root)?;
        if kind == Some("archive") {
            archives.extend(
                xarchive_core::ArchiveRecordsDocument::from_json(&raw)
                    .map_err(|e| e.to_string())?
                    .records,
            );
        } else {
            downloads.extend(
                xarchive_core::DownloadRecordsDocument::from_json(&raw)
                    .map_err(|e| e.to_string())?
                    .records,
            );
        }
    }
    if kind == Some("archive") {
        let document = xarchive_core::ArchiveRecordsDocument::new(archives);
        match format {
            "json" => document.to_json(),
            "yaml" => document.to_yaml(),
            _ => return Err("record format must be json or yaml".into()),
        }
        .map_err(|e| e.to_string())
    } else {
        let document = xarchive_core::DownloadRecordsDocument::new(downloads);
        match format {
            "json" => document.to_json(),
            "yaml" => document.to_yaml(),
            _ => return Err("record format must be json or yaml".into()),
        }
        .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
pub(super) fn serialize_record_export(raw: &str, format: &str) -> Result<String, String> {
    serialize_record_export_with_account(raw, format, None)
}

pub(super) fn serialize_record_export_with_account(
    raw: &str,
    format: &str,
    account: Option<String>,
) -> Result<String, String> {
    let metadata: xarchive_core::ArchiveMetadata =
        serde_json::from_str(raw).map_err(|e| e.to_string())?;
    // Refuse non-portable/private structural fields rather than silently changing
    // the canonical record and breaking cross-format idempotency.
    let safe_url = |url: &str| xarchive_core::redact(url, &[]) == url;
    if !safe_url(&metadata.url)
        || metadata
            .quoted_tweet
            .as_ref()
            .is_some_and(|quoted| !safe_url(&quoted.url))
        || metadata.media.iter().any(|media| {
            let path = Path::new(&media.file);
            path.is_absolute()
                || media.file.contains('\\')
                || media.file.contains(':')
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
        })
    {
        return Err(
            "record contains private URL or non-portable media path; export refused".into(),
        );
    }
    let account_handle = account
        .or_else(|| metadata.author.username.clone())
        .ok_or("record has no account handle")?;
    let document = xarchive_core::DownloadRecordsDocument::new(vec![DownloadRecord {
        account_handle,
        metadata,
    }]);
    match format {
        "json" => document.to_json(),
        "yaml" => document.to_yaml(),
        _ => return Err("record format must be json or yaml".into()),
    }
    .map_err(|e| e.to_string())
}
