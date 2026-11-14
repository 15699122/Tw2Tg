//! Durable archive execution-specification decoding.
//!
//! These types and decoders own one responsibility: turn a persisted execution
//! specification into validated business inputs, refusing anything this build
//! cannot interpret faithfully. They perform no I/O and hold no session, owner,
//! permit, or writer state.
//!
//! Compatibility note: `crate::executor` re-exports every public item here, so
//! existing call sites and serialized payloads are unchanged by the move.

use xarchive_storage::{BatchOutputSettings as OutputSettings, Database};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSpecV2 {
    pub request: crate::ArchiveTweetRequest,
    pub output_settings: OutputSettings,
}

/// Exact packaged tool versions a v3 argument snapshot was accepted against.
///
/// Tool compatibility is part of the immutable snapshot, not a lookup of the
/// currently installed tool: a persisted snapshot must fail on a different tool
/// version instead of silently re-interpreting saved arguments. A `None` entry
/// means the snapshot carries no arguments for that engine.
#[derive(Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCompatibility {
    pub aria2: Option<String>,
    pub gallery_dl: Option<String>,
}

impl std::fmt::Debug for ToolCompatibility {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ToolCompatibility")
            .field("aria2_identity_present", &self.aria2.is_some())
            .field("gallery_dl_identity_present", &self.gallery_dl.is_some())
            .finish()
    }
}

/// Typed downloader-argument snapshot carried by a v3 execution spec.
///
/// The two arrays are the user's own arguments per engine. They are stored as
/// validated argv elements and never tokenized, escaped, or rewritten here.
/// `policy_version` records which Rust argument policy accepted the snapshot so
/// a snapshot written under a later policy is refused rather than reinterpreted.
#[derive(Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloaderArgumentSnapshot {
    pub gallery_dl: Vec<String>,
    pub aria2: Vec<String>,
    pub policy_version: u32,
    pub tool_compatibility: ToolCompatibility,
}

impl std::fmt::Debug for DownloaderArgumentSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DownloaderArgumentSnapshot")
            .field("gallery_dl_argument_count", &self.gallery_dl.len())
            .field("aria2_argument_count", &self.aria2.len())
            .field("policy_version", &self.policy_version)
            .field("tool_compatibility", &self.tool_compatibility)
            .finish()
    }
}

impl DownloaderArgumentSnapshot {
    /// The snapshot used by every historical v1/v2 execution spec.
    ///
    /// Historical tasks carry no user arguments and must never be resampled
    /// from current settings, so they decode to an empty snapshot rather than
    /// to today's arguments.
    pub fn historical() -> Self {
        Self {
            policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSpecV3 {
    pub request: crate::ArchiveTweetRequest,
    pub output_settings: OutputSettings,
    pub downloader_arguments: DownloaderArgumentSnapshot,
}

/// D1 output contract. This is deliberately not the historical TXT snapshot.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataOutputSettings {
    pub naming_mode: xarchive_storage::BatchNamingMode,
    pub filename_template: String,
    pub export_json: bool,
    pub export_yaml: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSpecV4 {
    pub request: crate::ArchiveTweetRequest,
    pub output_settings: MetadataOutputSettings,
    pub downloader_arguments: DownloaderArgumentSnapshot,
}

impl Default for MetadataOutputSettings {
    fn default() -> Self {
        Self {
            naming_mode: xarchive_storage::BatchNamingMode::Original,
            filename_template: String::new(),
            export_json: true,
            export_yaml: true,
        }
    }
}

// Test-only switch that forces `MetadataOutputSettings::serialize_metadata` to
// fail. Thread-local so parallel tests cannot contaminate each other, and
// compiled out of production builds.
#[cfg(test)]
thread_local! {
    static METADATA_EXPORT_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Test-only guard. Restores the previous value on drop, including on panic.
#[cfg(test)]
pub struct MetadataExportFailureGuard {
    previous: bool,
}

#[cfg(test)]
impl MetadataExportFailureGuard {
    /// Force the next serializations on this thread to fail until the guard is
    /// dropped.
    pub fn arm() -> Self {
        let previous = METADATA_EXPORT_FAILURE.with(|failure| failure.replace(true));
        Self { previous }
    }
}

#[cfg(test)]
impl Drop for MetadataExportFailureGuard {
    fn drop(&mut self) {
        METADATA_EXPORT_FAILURE.with(|failure| failure.set(self.previous));
    }
}

impl MetadataOutputSettings {
    pub fn legacy_naming(&self) -> OutputSettings {
        OutputSettings {
            naming_mode: self.naming_mode,
            filename_template: self.filename_template.clone(),
            export_json: self.export_json,
            export_text: false,
        }
    }

    pub fn read(database: &Database) -> Result<Option<Self>, String> {
        database
            .get_setting("download.metadata_output.v4")
            .map_err(|e| e.to_string())?
            .map(|raw| {
                serde_json::from_str(&raw).map_err(|e| format!("invalid D1 output settings: {e}"))
            })
            .transpose()
    }
    /// Serialize the same explicit DTO before entering the publication phase.
    /// No files are mutated if either requested serializer fails.
    ///
    /// Called from `PendingArchivePublication::finalize`, which passes the
    /// result to the Storage publish entry point as `metadata_exports`.
    pub fn serialize_metadata<T: serde::Serialize>(
        &self,
        metadata: &T,
    ) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
        // Test-only failure injection. This is compiled out of production
        // builds and exists so the production finalizer path can be exercised
        // with a serializer failure instead of only argued about in source
        // order. It never changes the successful serialization result.
        #[cfg(test)]
        if METADATA_EXPORT_FAILURE.with(|failure| failure.get()) {
            return Err("injected metadata export failure".to_owned());
        }
        let mut exports = Vec::new();
        if self.export_json {
            exports.push((
                "tweet.json",
                serde_json::to_vec_pretty(metadata).map_err(|e| e.to_string())?,
            ));
        }
        if self.export_yaml {
            exports.push((
                "tweet.yaml",
                serde_yaml::to_string(metadata)
                    .map_err(|e| e.to_string())?
                    .into_bytes(),
            ));
        }
        Ok(exports)
    }
}

#[cfg(test)]
mod d1_contract_tests {
    #[test]
    fn v4_dispatch_is_explicit_and_never_reinterprets_txt() {
        let request = serde_json::json!({"tweet":{"tweet_id":"123","url":"https://x.com/a/status/123","tweet_type":"post"},"browser":null,"profile":null});
        let raw = serde_json::json!({"request":request,"output_settings":super::MetadataOutputSettings::default(),"downloader_arguments":super::DownloaderArgumentSnapshot::historical()}).to_string();
        let decoded = super::decode_execution_spec(4, &raw).unwrap();
        assert_eq!(
            decoded.metadata_output,
            Some(super::MetadataOutputSettings::default())
        );
        assert!(!decoded.output_settings.export_text);
        assert!(super::decode_execution_spec(3, &raw).is_err());
        assert!(super::decode_execution_spec(5, &raw).is_err());
    }
    #[test]
    fn d1_json_yaml_share_one_dto_for_all_switch_combinations() {
        let metadata = serde_json::json!({"schema_version":2,"text":"引号: \"正文\"\n下一行","media":[{"file":"01.jpg","size_bytes":15}],"author":null});
        for export_json in [false, true] {
            for export_yaml in [false, true] {
                let settings = super::MetadataOutputSettings {
                    export_json,
                    export_yaml,
                    ..Default::default()
                };
                let exports = settings.serialize_metadata(&metadata).unwrap();
                assert_eq!(
                    exports.len(),
                    usize::from(export_json) + usize::from(export_yaml)
                );
                for (name, bytes) in exports {
                    let parsed: serde_json::Value = match name {
                        "tweet.json" => serde_json::from_slice(&bytes).unwrap(),
                        "tweet.yaml" => serde_yaml::from_slice(&bytes).unwrap(),
                        _ => panic!("unexpected metadata export"),
                    };
                    assert_eq!(parsed, metadata);
                }
            }
        }
    }
    #[test]
    fn d1_snapshot_preserves_independent_formats_and_refuses_legacy_text() {
        for export_json in [false, true] {
            for export_yaml in [false, true] {
                let settings = super::MetadataOutputSettings {
                    export_json,
                    export_yaml,
                    ..Default::default()
                };
                let value = serde_json::to_value(&settings).unwrap();
                assert!(value.get("export_text").is_none());
                assert_eq!(
                    serde_json::from_value::<super::MetadataOutputSettings>(value).unwrap(),
                    settings
                );
            }
        }
        assert!(serde_json::from_str::<super::MetadataOutputSettings>(r#"{"naming_mode":"original","filename_template":"","export_json":true,"export_text":true}"#).is_err());
    }
}

#[derive(Debug, Clone)]
pub struct DecodedExecutionSpec {
    pub metadata_output: Option<MetadataOutputSettings>,
    pub request: crate::ArchiveTweetRequest,
    pub output_settings: OutputSettings,
    pub downloader_arguments: DownloaderArgumentSnapshot,
}

// The `InvalidJson` payload is part of this type's contract: callers construct it
// with the serde message and the derived `Debug` prints it. `Display`
// deliberately stays generic, and the dead-code lint ignores derived impls, so
// the payload is annotated rather than dropped or leaked into user-facing text.
#[allow(dead_code)]
#[derive(Debug)]
pub enum ExecutionSpecError {
    UnsupportedVersion(u32),
    InvalidJson(String),
    /// A persisted v3 snapshot whose argument-policy version does not match the
    /// Rust policy in this build. Rejected instead of reinterpreted.
    DownloaderArgumentPolicy(String),
}

impl std::fmt::Display for ExecutionSpecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "unsupported archive execution spec version: {version}"
                )
            }
            Self::InvalidJson(_) => formatter.write_str("invalid archive execution spec JSON"),
            Self::DownloaderArgumentPolicy(message) => {
                write!(formatter, "archive downloader-argument policy: {message}")
            }
        }
    }
}

pub fn decode_execution_spec(
    schema_version: u32,
    request_json: &str,
) -> Result<DecodedExecutionSpec, ExecutionSpecError> {
    match schema_version {
        1 => serde_json::from_str(request_json)
            .map(|request| DecodedExecutionSpec {
                metadata_output: None,
                request,
                output_settings: OutputSettings::default(),
                downloader_arguments: DownloaderArgumentSnapshot::historical(),
            })
            .map_err(|error| ExecutionSpecError::InvalidJson(error.to_string())),
        2 => serde_json::from_str::<ExecutionSpecV2>(request_json)
            .map(|spec| DecodedExecutionSpec {
                metadata_output: None,
                request: spec.request,
                output_settings: spec.output_settings,
                downloader_arguments: DownloaderArgumentSnapshot::historical(),
            })
            .map_err(|error| ExecutionSpecError::InvalidJson(error.to_string())),
        3 => {
            let spec = serde_json::from_str::<ExecutionSpecV3>(request_json)
                .map_err(|error| ExecutionSpecError::InvalidJson(error.to_string()))?;
            validate_downloader_argument_snapshot(&spec.downloader_arguments)?;
            Ok(DecodedExecutionSpec {
                metadata_output: None,
                request: spec.request,
                output_settings: spec.output_settings,
                downloader_arguments: spec.downloader_arguments,
            })
        }
        4 => {
            let spec = serde_json::from_str::<ExecutionSpecV4>(request_json)
                .map_err(|error| ExecutionSpecError::InvalidJson(error.to_string()))?;
            validate_downloader_argument_snapshot(&spec.downloader_arguments)?;
            Ok(DecodedExecutionSpec {
                output_settings: OutputSettings {
                    naming_mode: spec.output_settings.naming_mode,
                    filename_template: spec.output_settings.filename_template.clone(),
                    export_json: spec.output_settings.export_json,
                    export_text: false,
                },
                metadata_output: Some(spec.output_settings),
                request: spec.request,
                downloader_arguments: spec.downloader_arguments,
            })
        }
        version => Err(ExecutionSpecError::UnsupportedVersion(version)),
    }
}

/// Validate a persisted v3 downloader-argument snapshot.
///
/// The argument-policy version is dispatched independently of the
/// execution-spec and recovery-contract versions. A snapshot written under a
/// different policy version is refused so a later allowlist/value-policy change
/// can never silently reinterpret previously saved arguments.
pub fn validate_downloader_argument_snapshot(
    snapshot: &DownloaderArgumentSnapshot,
) -> Result<(), ExecutionSpecError> {
    let current = xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION;
    if snapshot.policy_version != current {
        return Err(ExecutionSpecError::DownloaderArgumentPolicy(format!(
            "persisted argument policy version mismatch (current {current})"
        )));
    }
    xarchive_download::downloader_args::validate_gallery_dl_args(&snapshot.gallery_dl)
        .map_err(|error| ExecutionSpecError::DownloaderArgumentPolicy(error.to_string()))?;
    xarchive_download::downloader_args::validate_aria2_args(&snapshot.aria2)
        .map_err(|error| ExecutionSpecError::DownloaderArgumentPolicy(error.to_string()))?;
    if !snapshot.gallery_dl.is_empty() && snapshot.tool_compatibility.gallery_dl.is_none() {
        return Err(ExecutionSpecError::DownloaderArgumentPolicy(
            "gallery-dl tool compatibility identity is required".to_owned(),
        ));
    }
    if !snapshot.aria2.is_empty() && snapshot.tool_compatibility.aria2.is_none() {
        return Err(ExecutionSpecError::DownloaderArgumentPolicy(
            "aria2 tool compatibility identity is required".to_owned(),
        ));
    }
    Ok(())
}
