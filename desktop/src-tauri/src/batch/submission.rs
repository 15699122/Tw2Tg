//! Batch submission contract: what a submitted job inherits, immutably.
//!
//! Split out of `batch.rs` without changing behaviour. The rules:
//!
//! * the output settings and downloader arguments captured with the batch are
//!   authoritative; current settings never redirect an already-created batch;
//! * a v4 metadata snapshot yields `ExecutionSpecV4`, anything else decodes as
//!   the legacy `ExecutionSpecV3`;
//! * an unsupported snapshot version fails closed rather than silently
//!   downgrading to a shape the caller did not intend.

pub(in crate::batch) fn inherited_execution_spec(
    request: crate::ArchiveTweetRequest,
    output_settings_json: &str,
    downloader_arguments_json: &str,
) -> Result<String, String> {
    if let Some(settings) = metadata_batch_settings(output_settings_json)? {
        let downloader_arguments = serde_json::from_str(downloader_arguments_json)
            .map_err(|e| format!("invalid batch downloader snapshot: {e}"))?;
        crate::executor::validate_downloader_argument_snapshot(&downloader_arguments)
            .map_err(|e| e.to_string())?;
        return serde_json::to_string(&crate::executor::ExecutionSpecV4 {
            request,
            output_settings: settings,
            downloader_arguments,
        })
        .map_err(|e| e.to_string());
    }
    let output_settings: crate::executor::OutputSettings =
        serde_json::from_str(output_settings_json)
            .map_err(|error| format!("invalid persisted batch output settings: {error}"))?;
    let downloader_arguments: crate::executor::DownloaderArgumentSnapshot =
        serde_json::from_str(downloader_arguments_json)
            .map_err(|error| format!("invalid persisted batch downloader arguments: {error}"))?;
    crate::executor::validate_downloader_argument_snapshot(&downloader_arguments)
        .map_err(|error| format!("invalid persisted batch downloader arguments: {error}"))?;
    serde_json::to_string(&crate::executor::ExecutionSpecV3 {
        request,
        output_settings,
        downloader_arguments,
    })
    .map_err(|error| format!("failed to encode batch execution spec: {error}"))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataBatchSnapshot {
    schema_version: u32,
    settings: crate::executor::MetadataOutputSettings,
}

pub(in crate::batch) fn metadata_batch_settings(
    raw: &str,
) -> Result<Option<crate::executor::MetadataOutputSettings>, String> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if value.get("schema_version").is_none() {
        return Ok(None);
    }
    let snapshot: MetadataBatchSnapshot =
        serde_json::from_value(value).map_err(|e| e.to_string())?;
    if snapshot.schema_version != 4 {
        return Err("unsupported batch output snapshot version".into());
    }
    Ok(Some(snapshot.settings))
}
