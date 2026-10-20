//! R1 job-executor command/state model and production control boundary.
//!
//! The module owns the bounded control worker, single active runner,
//! persistence ports, recovery contracts, execution-spec fencing, and the
//! `ExecutorRuntime` resource boundary used by RuntimeState. Production
//! execution loads immutable request specs by Job ID and creates its own
//! Database/FileStore/Sidecar context; there is no synchronous archive command
//! fallback.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use xarchive_core::{ChildEnvironment, JobEvent, JobState, ProxyMode};

use xarchive_sidecar_supervisor::{ChildProcessEnvironment, SidecarSupervisor};
#[cfg(test)]
pub use xarchive_storage::BatchNamingMode as NamingMode;
pub use xarchive_storage::BatchOutputSettings as OutputSettings;
use xarchive_storage::{ArchiveRecoveryDispatch, ArchiveService, Database, FileStore, JobSummary};
use xarchive_workflow::coordination::Coordinator;
#[cfg(unix)]
use xarchive_workflow::coordination::unix::{LinuxCoordinator, LinuxGuard};
#[cfg(unix)]
use xarchive_workflow::coordination::{LockError, LockScope};
use xarchive_workflow::{AttemptIdentity, ClaimOutcome, ExecutionReservation, SessionError};

const DEFAULT_QUEUE_CAPACITY: usize = 32;
/// Capacity of the single-active-runner channel. Runner admission must be
/// checked before any durable attempt/state claim so a saturated runner never
/// converts an accepted Job into a persisted `FAILED`.
const RUNNER_CAPACITY: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveJobRequest {
    pub job_id: String,
    pub tweet_id: String,
    pub request_id: String,
    pub request_json: String,
    pub schema_version: u32,
}

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
    pub fn serialize_metadata<T: serde::Serialize>(
        &self,
        metadata: &T,
    ) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
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

fn validate_execution_spec_identity(
    decoded: DecodedExecutionSpec,
    job_tweet_id: &str,
) -> Result<DecodedExecutionSpec, ExecutorError> {
    if decoded.request.tweet.tweet_id != job_tweet_id {
        return Err(ExecutorError::Persistence(
            "archive execution spec tweet identity does not match Job".to_owned(),
        ));
    }
    Ok(decoded)
}

/// Adapter that validates a browser archive request and derives the durable
/// executor Job identity from it. It performs request validation and Job
/// identity derivation only; it does not own I/O.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArchiveJobSubmissionAdapter;

impl ArchiveJobSubmissionAdapter {
    pub fn prepare_with_metadata_settings(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
        output_settings: MetadataOutputSettings,
        downloader_arguments: DownloaderArgumentSnapshot,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        let mut prepared = self.prepare(request, timestamp)?;
        validate_downloader_argument_snapshot(&downloader_arguments)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        prepared.request_json = serde_json::to_string(&ExecutionSpecV4 {
            request: request.clone(),
            output_settings,
            downloader_arguments,
        })
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        prepared.schema_version = 4;
        Ok(prepared)
    }
    pub fn prepare_with_output_settings(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
        output_settings: OutputSettings,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        let mut prepared = self.prepare(request, timestamp)?;
        prepared.request_json = serde_json::to_string(&ExecutionSpecV2 {
            request: request.clone(),
            output_settings,
        })
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        Ok(prepared)
    }

    pub fn prepare_with_downloader_settings(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
        output_settings: OutputSettings,
        downloader_arguments: DownloaderArgumentSnapshot,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        validate_downloader_argument_snapshot(&downloader_arguments)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        xarchive_protocol::BrowserRequest::ArchiveRequest {
            protocol_version: xarchive_protocol::PROTOCOL_VERSION,
            request_id: "archive-validation".to_owned(),
            tweet: request.tweet.clone(),
        }
        .validate()
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let spec = ExecutionSpecV3 {
            request: request.clone(),
            output_settings,
            downloader_arguments,
        };
        Ok(ArchiveJobRequest {
            job_id: format!("archive-{}-{timestamp}", request.tweet.tweet_id),
            tweet_id: request.tweet.tweet_id.clone(),
            request_id: format!("desktop-archive-{}", request.tweet.tweet_id),
            request_json: serde_json::to_string(&spec)
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
            schema_version: 3,
        })
    }

    pub fn prepare(
        &self,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
    ) -> Result<ArchiveJobRequest, ExecutorError> {
        xarchive_protocol::BrowserRequest::ArchiveRequest {
            protocol_version: xarchive_protocol::PROTOCOL_VERSION,
            request_id: "archive-validation".to_owned(),
            tweet: request.tweet.clone(),
        }
        .validate()
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;

        Ok(ArchiveJobRequest {
            job_id: format!("archive-{}-{timestamp}", request.tweet.tweet_id),
            tweet_id: request.tweet.tweet_id.clone(),
            request_id: format!("desktop-archive-{}", request.tweet.tweet_id),
            request_json: serde_json::to_string(&ExecutionSpecV2 {
                request: request.clone(),
                output_settings: OutputSettings::default(),
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
            schema_version: 2,
        })
    }

    pub fn submit<P: JobPersistence>(
        &self,
        service: &ArchiveApplicationService,
        persistence: &mut P,
        request: &crate::ArchiveTweetRequest,
        timestamp: &str,
    ) -> Result<SubmitResult, ExecutorError> {
        let job = self.prepare(request, timestamp)?;
        service.submit_persisted(persistence, job)
    }

    pub fn query<P: JobPersistence>(
        &self,
        service: &ArchiveApplicationService,
        persistence: &P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        service.query_persisted(persistence, job_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSnapshot {
    pub job_id: String,
    pub tweet_id: String,
    pub state: JobState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitRecoveryDirectory {
    Missing,
    Present,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecoveryFacts {
    pub job_id: String,
    pub state: JobState,
    pub final_directory: CommitRecoveryDirectory,
    pub staging_directory: CommitRecoveryDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitRecoveryDecision {
    ResumeStaging,
    Complete {
        archive_directory: String,
    },
    MarkFailed {
        error_code: String,
        error_message: String,
    },
    Skip,
    NotApplicable,
}

pub trait CommitRecoveryFactsProvider {
    fn facts_for(&self, job_id: &str) -> Result<CommitRecoveryFacts, ExecutorError>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommitRecoveryBatchResult {
    pub job_id: String,
    pub result: Result<JobSnapshot, ExecutorError>,
}

#[derive(Debug, Default)]
pub struct InMemoryCommitRecoveryFactsProvider {
    facts: HashMap<String, CommitRecoveryFacts>,
}

impl InMemoryCommitRecoveryFactsProvider {
    pub fn insert(&mut self, facts: CommitRecoveryFacts) {
        self.facts.insert(facts.job_id.clone(), facts);
    }
}

impl CommitRecoveryFactsProvider for InMemoryCommitRecoveryFactsProvider {
    fn facts_for(&self, job_id: &str) -> Result<CommitRecoveryFacts, ExecutorError> {
        self.facts
            .get(job_id)
            .cloned()
            .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))
    }
}

impl CommitRecoveryFacts {
    pub fn decide(&self, archive_directory: &str) -> CommitRecoveryDecision {
        match (self.state, self.final_directory, self.staging_directory) {
            (
                JobState::Downloaded,
                CommitRecoveryDirectory::Missing,
                CommitRecoveryDirectory::Present,
            ) => CommitRecoveryDecision::ResumeStaging,
            (JobState::Downloaded, CommitRecoveryDirectory::Present, _) => {
                CommitRecoveryDecision::Complete {
                    archive_directory: archive_directory.to_owned(),
                }
            }
            (
                JobState::Downloaded,
                CommitRecoveryDirectory::Missing,
                CommitRecoveryDirectory::Missing,
            ) => CommitRecoveryDecision::MarkFailed {
                error_code: "ARCHIVE_COMMIT_INCOMPLETE".to_owned(),
                error_message: format!(
                    "job {} is DOWNLOADED but has neither final archive nor staging directory",
                    self.job_id
                ),
            },
            (JobState::Complete, CommitRecoveryDirectory::Present, _) => {
                CommitRecoveryDecision::Skip
            }
            (JobState::Complete, CommitRecoveryDirectory::Missing, _) => {
                CommitRecoveryDecision::MarkFailed {
                    error_code: "ARCHIVE_COMMIT_MISSING".to_owned(),
                    error_message: format!(
                        "job {} is COMPLETE but its final archive directory is missing",
                        self.job_id
                    ),
                }
            }
            _ => CommitRecoveryDecision::NotApplicable,
        }
    }
}

impl JobSnapshot {
    /// Project only executor-owned fields from the storage-facing JobSummary.
    /// Timestamps, tweet type, and persisted error details remain query-layer
    /// data and are intentionally not synthesized by the executor.
    pub fn from_job_summary(summary: &JobSummary) -> Self {
        Self {
            job_id: summary.job_id.clone(),
            tweet_id: summary.tweet_id.clone(),
            state: summary.state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitResult {
    pub job: JobSnapshot,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecutionResult {
    pub files_copied: u32,
    pub archive_directory: String,
    pub download_event_recorded: bool,
    pub state_already_updated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecutionError {
    pub error_code: String,
    pub error_message: String,
    pub persistence_already_updated: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

pub trait JobExecution: Send {
    fn execute(
        &mut self,
        job: &JobSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError>;
}

pub trait JobExecutionFactory: Send + Sync {
    fn create(
        &self,
        job_id: &str,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError>;

    /// CROSS_PLATFORM_CHANGE_REQUIRED: session execution must explicitly bind
    /// its writer to the claimed attempt. Never fall back to a Job-only writer.
    fn create_for_attempt(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        let _ = (identity, snapshot);
        Err(ExecutorError::Execution {
            error_code: "ATTEMPT_EXECUTION_NOT_CONFIGURED".to_owned(),
            error_message: "factory does not support attempt-owned execution".to_owned(),
            persistence_already_updated: false,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ExecutorConfig {
    pub telegram_config_file: Option<PathBuf>,
    pub telegram: crate::config::TelegramConfig,
    pub archive_root: PathBuf,
    pub staging_root: PathBuf,
    pub database_path: PathBuf,
    pub sidecar_program: Option<String>,
    pub gallery_dl_program: Option<String>,
    pub sidecar_args: Vec<String>,
    pub aria2_program: Option<String>,
    /// Whether this execution may spawn aria2. When false, gallery-dl
    /// performs byte downloads and aria2 must remain untouched.
    pub use_aria2: bool,
    pub network: ExecutorNetworkConfig,
}

/// Network values carried into production execution.
///
/// Timeouts and retry budgets stay numeric here; the optional proxy may carry
/// credentials, so it is kept off `Debug` output and off process command lines
/// (aria2 and the Sidecar receive it through the environment instead).
#[derive(Clone, Default)]
pub struct ExecutorNetworkConfig {
    pub transfer_timeout: std::time::Duration,
    pub telegram_timeout: std::time::Duration,
    /// The routing mode. It travels with the proxy because `System` and `Direct`
    /// are decisions, not values: a stored proxy is inert unless the mode says
    /// otherwise.
    pub proxy_mode: ProxyMode,
    pub proxy: Option<String>,
    pub aria2_connect_timeout: std::time::Duration,
    pub aria2_idle_timeout: std::time::Duration,
    pub aria2_max_tries: u32,
    pub extraction_timeout_secs: f64,
    pub discovery_timeout_secs: f64,
}

impl std::fmt::Debug for ExecutorNetworkConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExecutorNetworkConfig")
            .field("transfer_timeout", &self.transfer_timeout)
            .field("telegram_timeout", &self.telegram_timeout)
            .field("proxy_mode", &self.proxy_mode)
            .field("proxy", &self.proxy.as_ref().map(|_| "[REDACTED]"))
            .field("aria2_connect_timeout", &self.aria2_connect_timeout)
            .field("aria2_idle_timeout", &self.aria2_idle_timeout)
            .field("aria2_max_tries", &self.aria2_max_tries)
            .field("extraction_timeout_secs", &self.extraction_timeout_secs)
            .field("discovery_timeout_secs", &self.discovery_timeout_secs)
            .finish()
    }
}

impl ExecutorNetworkConfig {
    pub(crate) fn from_seconds(
        transfer_timeout_seconds: u64,
        telegram_timeout_seconds: u64,
        aria2_connect_timeout_seconds: u64,
        aria2_idle_timeout_seconds: u64,
        aria2_max_tries: u32,
        extraction_timeout_seconds: u64,
        discovery_timeout_seconds: u64,
    ) -> Self {
        Self {
            transfer_timeout: std::time::Duration::from_secs(transfer_timeout_seconds.max(1)),
            telegram_timeout: std::time::Duration::from_secs(telegram_timeout_seconds.max(1)),
            proxy_mode: ProxyMode::System,
            proxy: None,
            aria2_connect_timeout: std::time::Duration::from_secs(
                aria2_connect_timeout_seconds.max(1),
            ),
            aria2_idle_timeout: std::time::Duration::from_secs(aria2_idle_timeout_seconds.max(1)),
            aria2_max_tries: aria2_max_tries.max(1),
            extraction_timeout_secs: extraction_timeout_seconds.max(1) as f64,
            discovery_timeout_secs: discovery_timeout_seconds.max(1) as f64,
        }
    }

    /// Apply the routing mode and the configured value together.
    ///
    /// The value is normalized but kept even for `System` and `Direct`, because
    /// a later switch back to `Manual` must not silently lose the setting. Only
    /// the boundary decides whether a stored value is in effect.
    pub(crate) fn with_proxy(mut self, mode: ProxyMode, proxy: Option<String>) -> Self {
        self.proxy_mode = mode;
        self.proxy = proxy
            .map(|proxy| proxy.trim().to_owned())
            .filter(|proxy| !proxy.is_empty());
        self
    }

    /// The proxy value that is actually in effect for this mode.
    pub(crate) fn active_proxy(&self) -> Option<String> {
        match self.proxy_mode {
            ProxyMode::Manual => self.proxy.clone(),
            ProxyMode::System | ProxyMode::Direct => None,
        }
    }
}

pub struct ProductionExecutionFactory {
    config: ExecutorConfig,
    #[cfg(target_os = "linux")]
    containment: Option<LinuxExecutionContainment>,
}

/// Explicit provisioning, never inferred from inherited environment variables.
/// CROSS_PLATFORM_CHANGE_REQUIRED: Linux opt-in production launch contract.
#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct LinuxExecutionContainment {
    pub delegated_parent: PathBuf,
    pub python: PathBuf,
    pub launcher: PathBuf,
}

#[cfg(target_os = "linux")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LinuxLaunchDescriptor {
    schema_version: u32,
    delegated_parent: PathBuf,
    python: PathBuf,
    launcher: PathBuf,
}

#[cfg(target_os = "linux")]
impl LinuxExecutionContainment {
    /// Explicit trusted-launcher input; never read inherited environment.
    /// CROSS_PLATFORM_CHANGE_REQUIRED: application provisioning contract.
    pub(crate) fn from_launch_arguments(
        arguments: impl IntoIterator<Item = std::ffi::OsString>,
    ) -> Result<Option<Self>, String> {
        use std::io::Read;
        let mut arguments = arguments.into_iter();
        let mut descriptor = None;
        while let Some(argument) = arguments.next() {
            if argument != "--linux-execution-descriptor" {
                continue;
            }
            if descriptor.is_some() {
                return Err("duplicate Linux execution descriptor".into());
            }
            let path = PathBuf::from(
                arguments
                    .next()
                    .ok_or("missing Linux execution descriptor")?,
            );
            if !path.is_absolute() {
                return Err("Linux execution descriptor path must be absolute".into());
            }
            // Trusted launch input must still never block startup on a FIFO or
            // device. Symlinked descriptors are rejected; Python may remain a
            // deployment-managed symlink (as on normal Linux installations).
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|_| "cannot inspect Linux execution descriptor")?;
            if !metadata.file_type().is_file() {
                return Err("Linux execution descriptor must be a regular file".into());
            }
            let file =
                std::fs::File::open(path).map_err(|_| "cannot open Linux execution descriptor")?;
            let mut bytes = Vec::new();
            file.take(16 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "cannot read Linux execution descriptor")?;
            if bytes.len() > 16 * 1024 {
                return Err("Linux execution descriptor exceeds size limit".into());
            }
            let value: LinuxLaunchDescriptor =
                serde_json::from_slice(&bytes).map_err(|_| "invalid Linux execution descriptor")?;
            if value.schema_version != 1 {
                return Err("unsupported Linux execution descriptor version".into());
            }
            let containment = Self {
                delegated_parent: value.delegated_parent,
                python: value.python,
                launcher: value.launcher,
            };
            containment.validate_launch_paths()?;
            descriptor = Some(containment);
        }
        Ok(descriptor)
    }

    pub(crate) fn validate_launch_paths(&self) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        for path in [&self.delegated_parent, &self.python, &self.launcher] {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(
                    "Linux execution paths must be absolute without parent traversal".into(),
                );
            }
        }
        if self.delegated_parent == Path::new("/sys/fs/cgroup")
            || !self.delegated_parent.starts_with("/sys/fs/cgroup")
            || !self.delegated_parent.join("cgroup.events").is_file()
            || !self.delegated_parent.join("cgroup.procs").is_file()
        {
            return Err("Linux execution parent must be a dedicated cgroup v2 directory".into());
        }
        let python =
            std::fs::metadata(&self.python).map_err(|_| "Linux execution Python unavailable")?;
        if !python.is_file() || python.permissions().mode() & 0o111 == 0 || !self.launcher.is_file()
        {
            return Err("Linux execution Python/launcher unavailable".into());
        }
        // Verify delegation with an exclusively created disposable group. The
        // existing adapter rejects symlink traversal and never adopts a peer's
        // group. No target process is started by this capability probe.
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| "Linux execution probe entropy unavailable")?;
        let name = format!("tw2tg-provision-{:032x}", u128::from_le_bytes(nonce));
        let mut probe = xarchive_workflow::linux_containment::LinuxContainment::create(
            &self.delegated_parent,
            &name,
        )
        .map_err(|_| "Linux execution cgroup delegation unavailable")?;
        probe
            .stop_and_drain(std::time::Duration::from_secs(2))
            .and_then(|_| probe.remove_group())
            .map_err(|_| "Linux execution cgroup probe cleanup failed")?;
        Ok(())
    }
}

/// Attempt-bound writer-domain provisioning for one production execution.
///
/// `CROSS_PLATFORM_CHANGE_REQUIRED` (ADR-015 clause 8): R4b replaces
/// per-process unique cgroup names with the recorded attempt-bound
/// containment protocol, so a successor can discover and drain a crashed
/// attempt's writer domains without PID, timestamp or fixed-name inference.
#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct AttemptWriterDomain {
    pub provisioning: LinuxExecutionContainment,
    pub record_root: PathBuf,
    pub identity: xarchive_workflow::linux_containment::AttemptIdentity,
}

/// Stable workspace-scoped record root beside the staging root.
///
/// The record root lives outside the staging tree so attempt-staging cleanup
/// can never delete a record whose writer domain still exists: record
/// absence must keep proving that the protocol never created a group.
#[cfg(target_os = "linux")]
pub(crate) fn containment_record_root(staging_root: &Path) -> std::io::Result<PathBuf> {
    if !staging_root.is_absolute() {
        return Err(std::io::Error::other(
            "attempt containment requires an absolute staging root",
        ));
    }
    let parent = staging_root
        .parent()
        .ok_or_else(|| std::io::Error::other("staging root has no parent directory"))?;
    Ok(parent.join("tw2tg-containment-records"))
}

/// Attempt-bound containment identity for one writer domain.
#[cfg(target_os = "linux")]
pub(crate) fn attempt_writer_base(
    staging_root: &Path,
    identity: &AttemptIdentity,
) -> std::io::Result<xarchive_workflow::linux_containment::AttemptIdentity> {
    xarchive_workflow::linux_containment::AttemptIdentity::new(
        staging_root,
        identity.job_id(),
        u64::from(identity.attempt()),
    )
}

/// Writer roles whose recorded domains one attempt may create.
///
/// Successor recovery enumerates exactly this set; keep it in sync with the
/// production spawn sites.
#[cfg(target_os = "linux")]
pub(crate) const ATTEMPT_WRITER_ROLES: [&str; 3] = ["sidecar", "aria2-0", "aria2-1"];

/// Drain every recorded writer domain of one attempt.
///
/// Only the three safe outcomes admit; any error fails closed.
#[cfg(target_os = "linux")]
pub(crate) fn recover_writer_domain_set(
    record_root: &Path,
    workspace: &Path,
    job_id: &str,
    attempt: u64,
) -> std::io::Result<()> {
    use xarchive_workflow::linux_containment::{LinuxContainment, RecoveryOutcome};
    if attempt == 0 {
        return Ok(());
    }
    for writer in ATTEMPT_WRITER_ROLES {
        let domain =
            xarchive_workflow::linux_containment::AttemptIdentity::new(workspace, job_id, attempt)?
                .with_writer(writer)?;
        match LinuxContainment::recover_attempt(
            record_root,
            &domain,
            std::time::Duration::from_secs(10),
        )? {
            RecoveryOutcome::NoRecord
            | RecoveryOutcome::AlreadyAbsent
            | RecoveryOutcome::Drained => {}
        }
    }
    Ok(())
}

/// Drain a crashed predecessor attempt's writer domains before a successor
/// admission. Only the three safe outcomes admit; any error fails closed.
#[cfg(target_os = "linux")]
pub(crate) fn recover_predecessor_writer_domains(
    record_root: &Path,
    base: &xarchive_workflow::linux_containment::AttemptIdentity,
) -> std::io::Result<()> {
    if base.attempt() <= 1 {
        return Ok(());
    }
    recover_writer_domain_set(
        record_root,
        base.workspace(),
        base.job_id(),
        base.attempt() - 1,
    )
}

/// Recover this attempt's writer domain, then create its recorded group.
///
/// Successor admission happens only on the three safe outcomes; any recovery
/// error fails closed instead of falling back to an unrecorded group.
#[cfg(target_os = "linux")]
pub(crate) fn recover_and_create_writer_group(
    domain: &AttemptWriterDomain,
    writer: &str,
) -> std::io::Result<xarchive_workflow::linux_containment::LinuxContainment> {
    use xarchive_workflow::linux_containment::{LinuxContainment, RecoveryOutcome};
    let identity = domain.identity.clone().with_writer(writer)?;
    match LinuxContainment::recover_attempt(
        &domain.record_root,
        &identity,
        std::time::Duration::from_secs(10),
    )? {
        RecoveryOutcome::NoRecord | RecoveryOutcome::AlreadyAbsent | RecoveryOutcome::Drained => {}
    }
    LinuxContainment::create_recorded(
        &domain.record_root,
        &identity,
        &domain.provisioning.delegated_parent,
    )
}

#[cfg(all(test, target_os = "linux"))]
mod attempt_writer_domain_tests {
    use super::*;

    #[test]
    fn record_root_requires_an_absolute_staging_root() {
        assert!(containment_record_root(Path::new("staging")).is_err());
        let root = containment_record_root(Path::new("/data/_staging")).unwrap();
        assert!(root.starts_with("/data"));
        assert!(root.ends_with("tw2tg-containment-records"));
    }

    #[test]
    fn predecessor_recovery_skips_the_first_attempt() {
        let base = xarchive_workflow::linux_containment::AttemptIdentity::new("/ws/r4b", "job", 1)
            .unwrap();
        recover_predecessor_writer_domains(Path::new("/nonexistent-record-root"), &base).unwrap();
    }

    #[test]
    fn predecessor_recovery_fails_closed_on_a_malformed_record() {
        let record_root = std::env::temp_dir().join(format!(
            "tw2tg-r4b-predecessor-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&record_root).unwrap();
        let base = xarchive_workflow::linux_containment::AttemptIdentity::new("/ws/r4b", "job", 2)
            .unwrap();
        let previous =
            xarchive_workflow::linux_containment::AttemptIdentity::new("/ws/r4b", "job", 1)
                .unwrap()
                .with_writer("sidecar")
                .unwrap();
        // A malformed predecessor record must stop the successor instead of
        // being treated as absence.
        std::fs::write(
            previous.record_path(&record_root),
            "not a containment record\n",
        )
        .unwrap();
        let error = recover_predecessor_writer_domains(&record_root, &base).unwrap_err();
        assert_ne!(error.kind(), std::io::ErrorKind::NotFound);
        std::fs::remove_dir_all(&record_root).unwrap();
    }
}

fn decode_job_execution_spec(
    schema_version: u32,
    request_json: &str,
    job_tweet_id: &str,
) -> Result<DecodedExecutionSpec, ExecutorError> {
    let decoded = decode_execution_spec(schema_version, request_json)
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
    validate_execution_spec_identity(decoded, job_tweet_id)
}

impl ProductionExecutionFactory {
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            config,
            #[cfg(target_os = "linux")]
            containment: None,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn with_linux_containment(mut self, containment: LinuxExecutionContainment) -> Self {
        self.containment = Some(containment);
        self
    }
}

impl JobExecutionFactory for ProductionExecutionFactory {
    fn create(
        &self,
        job_id: &str,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        let _ = (job_id, snapshot);
        Err(ExecutorError::Execution {
            error_code: "ATTEMPT_EXECUTION_REQUIRED".to_owned(),
            error_message: "production archive execution requires a claimed attempt identity"
                .to_owned(),
            persistence_already_updated: false,
        })
    }

    fn create_for_attempt(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        if identity.job_id() != snapshot.job_id.as_str() {
            return Err(ExecutorError::Execution {
                error_code: "ATTEMPT_JOB_MISMATCH".to_owned(),
                error_message: "claimed attempt does not match the execution Job".to_owned(),
                persistence_already_updated: false,
            });
        }
        if identity.attempt() == 0 {
            return Err(ExecutorError::Execution {
                error_code: "INVALID_ATTEMPT_IDENTITY".to_owned(),
                error_message: "archive attempt number must be positive".to_owned(),
                persistence_already_updated: false,
            });
        }
        self.create_attempt_execution(identity, snapshot)
    }
}

impl ProductionExecutionFactory {
    fn create_attempt_execution(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        let job_id = identity.job_id();
        let database = Database::open(&self.config.database_path)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let (schema_version, request_id, request_json) = database
            .archive_job_request(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .ok_or_else(|| {
                ExecutorError::Persistence("archive execution spec is missing".to_owned())
            })?;
        let decoded = decode_job_execution_spec(schema_version, &request_json, &snapshot.tweet_id)?;
        let request = decoded.request;
        let output_settings = decoded.output_settings;
        let tweet_row_id = database
            .tweet_row_id(&request.tweet.tweet_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let files = xarchive_storage::FileStore::with_staging_root(
            self.config.archive_root.clone(),
            self.config.staging_root.clone(),
        )
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let program =
            self.config
                .sidecar_program
                .as_deref()
                .ok_or_else(|| ExecutorError::Execution {
                    error_code: "SIDECAR_NOT_CONFIGURED".to_owned(),
                    error_message: "XARCHIVE_SIDECAR_PROGRAM is not configured".to_owned(),
                    persistence_already_updated: false,
                })?;
        let args = self
            .config
            .sidecar_args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        // The Sidecar is told the mode as well as the value, and a `Direct`
        // mode additionally strips the proxy variables it would otherwise
        // inherit from the launching shell.
        //
        // `System` is only safe to hand over as an inherited environment when
        // the platform policy has one answer for every host. A PAC or WPAD
        // policy does not, and the child would silently bypass it, so the job
        // is refused here instead of running unprotected.
        let network = crate::proxy::ProxyHttpClient::new(
            self.config.network.proxy_mode,
            self.config.network.proxy.clone(),
            self.config.network.transfer_timeout,
            crate::proxy::platform_resolver(),
        );
        network
            .require_child_routable(crate::proxy::SIDECAR_PROXY_PROBE_URL, "the Sidecar worker")
            .map_err(|error| ExecutorError::Execution {
                error_code: "PROXY_POLICY_NOT_APPLICABLE".to_owned(),
                error_message: error,
                persistence_already_updated: false,
            })?;
        let environment = ChildEnvironment::for_mode(
            self.config.network.proxy_mode,
            self.config.network.proxy.clone(),
        );
        let mut env = vec![(
            "XARCHIVE_PROXY_MODE".to_owned(),
            self.config.network.proxy_mode.as_str().to_owned(),
        )];
        env.extend(environment.set);
        let environment = ChildProcessEnvironment::new(env, environment.remove);
        #[cfg(target_os = "linux")]
        let attempt_domain =
            self.containment
                .as_ref()
                .map(
                    |provisioning| -> Result<AttemptWriterDomain, ExecutorError> {
                        let record_root = containment_record_root(&self.config.staging_root)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: "STAGING_ROOT_NOT_ABSOLUTE".to_owned(),
                                error_message: error.to_string(),
                                persistence_already_updated: false,
                            })?;
                        let base = attempt_writer_base(&self.config.staging_root, identity)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: "ATTEMPT_CONTAINMENT_IDENTITY_INVALID".to_owned(),
                                error_message: error.to_string(),
                                persistence_already_updated: false,
                            })?;
                        // Successor admission is fail-closed: a predecessor writer
                        // domain may only be admitted as NoRecord, AlreadyAbsent or
                        // Drained; anything else must refuse this attempt.
                        recover_predecessor_writer_domains(&record_root, &base).map_err(
                            |error| ExecutorError::Execution {
                                error_code: "WRITER_DOMAIN_RECOVERY_FAILED".to_owned(),
                                error_message: error.to_string(),
                                persistence_already_updated: false,
                            },
                        )?;
                        Ok(AttemptWriterDomain {
                            provisioning: provisioning.clone(),
                            record_root,
                            identity: base,
                        })
                    },
                )
                .transpose()?;
        #[cfg(target_os = "linux")]
        let contained = attempt_domain.as_ref().map(|domain| {
            // The attempt-bound recorded group replaces the retired
            // per-process unique name: the deterministic identity plus the
            // recover-before-create protocol prevents accidental reuse of a
            // crashed earlier attempt's domain. Removal remains
            // supervisor-owned.
            let owner = recover_and_create_writer_group(domain, "sidecar")
                .map_err(xarchive_sidecar_supervisor::SupervisorError::Spawn)?;
            SidecarSupervisor::spawn_contained_with_environment(
                owner,
                &domain.provisioning.python,
                &domain.provisioning.launcher,
                std::path::Path::new(program),
                &args,
                &environment,
            )?
            .require_ready_v2(std::time::Duration::from_secs(5))
        });
        #[cfg(target_os = "linux")]
        let supervisor = match contained {
            Some(result) => result,
            None => SidecarSupervisor::spawn_ready_v2_with_environment(
                program,
                &args,
                &environment,
                std::time::Duration::from_secs(5),
            ),
        };
        #[cfg(not(target_os = "linux"))]
        let supervisor = SidecarSupervisor::spawn_ready_v2_with_environment(
            program,
            &args,
            &environment,
            std::time::Duration::from_secs(5),
        );
        let supervisor = supervisor.map_err(|error| ExecutorError::Execution {
            error_code: "SIDECAR_START_FAILED".to_owned(),
            error_message: error.to_string(),
            persistence_already_updated: false,
        })?;
        let mut context = crate::archive::ArchiveExecutionContext::with_aria2_and_network(
            database,
            files,
            supervisor,
            self.config.aria2_program.clone(),
            self.config.use_aria2,
            self.config.network.clone(),
        )
        .with_attempt_identity(identity.clone())
        .with_downloader_arguments(decoded.downloader_arguments.clone())
        .with_gallery_dl_program(self.config.gallery_dl_program.clone());
        context.telegram = self.config.telegram.clone();
        #[cfg(target_os = "linux")]
        {
            context.containment = attempt_domain;
        }
        context.telegram_config_file = self.config.telegram_config_file.clone();
        let (execution, _lease) = crate::archive::ArchiveExecutionJob::new(
            context,
            request,
            output_settings,
            tweet_row_id,
            request_id,
            crate::runtime::timestamp_marker(),
        );
        Ok(Box::new(
            execution.with_metadata_output(decoded.metadata_output),
        ))
    }
}

#[cfg(test)]
mod production_factory_attempt_tests {
    use super::*;

    #[test]
    fn production_factory_rejects_unfenced_legacy_execution_creation() {
        let factory = ProductionExecutionFactory::new(ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root: PathBuf::from("archive"),
            staging_root: PathBuf::from("staging"),
            database_path: PathBuf::from("db.sqlite3"),
            sidecar_program: None,
            gallery_dl_program: None,
            sidecar_args: Vec::new(),
            aria2_program: None,
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        });
        let result = factory.create(
            "job-1",
            &JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            },
        );
        let error = match result {
            Ok(_) => panic!("legacy creation unexpectedly succeeded without attempt fence"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            ExecutorError::Execution { error_code, .. }
                if error_code == "ATTEMPT_EXECUTION_REQUIRED"
        ));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorEvent {
    Submitted {
        job_id: String,
    },
    Reused {
        job_id: String,
    },
    StateChanged {
        from: JobState,
        to: JobState,
    },
    Recovered {
        from: JobState,
        to: JobState,
    },
    ShutdownInterrupted {
        from: JobState,
        to: JobState,
    },
    DownloadStarted {
        backend: String,
    },
    DownloadCompleted {
        files_copied: u32,
    },
    DownloadFailed {
        error_code: String,
        error_message: String,
    },
    Completed {
        archive_directory: String,
    },
    SidecarCrashed {
        error_message: String,
    },
}

impl ExecutorEvent {
    pub fn to_job_event(&self, job_id: &str) -> Option<JobEvent> {
        match self {
            Self::Submitted { .. } => Some(JobEvent::Created {
                job_id: job_id.to_owned(),
            }),
            Self::Reused { .. } => None,
            Self::StateChanged { from, to } | Self::Recovered { from, to } => {
                Some(JobEvent::StateChanged {
                    from: *from,
                    to: *to,
                })
            }
            Self::ShutdownInterrupted { from, to } => Some(JobEvent::StateChanged {
                from: *from,
                to: *to,
            }),
            Self::DownloadStarted { backend } => Some(JobEvent::DownloadStarted {
                backend: backend.clone(),
            }),
            Self::DownloadCompleted { files_copied } => Some(JobEvent::DownloadCompleted {
                files_copied: *files_copied,
            }),
            Self::DownloadFailed {
                error_code,
                error_message,
            } => Some(JobEvent::DownloadFailed {
                error_code: error_code.clone(),
                error_message: error_message.clone(),
            }),
            Self::Completed { archive_directory } => Some(JobEvent::Completed {
                archive_directory: archive_directory.clone(),
            }),
            Self::SidecarCrashed { error_message } => Some(JobEvent::DownloadFailed {
                error_code: "SIDECAR_INTERNAL_ERROR".to_owned(),
                error_message: error_message.clone(),
            }),
        }
    }
}

pub trait JobPersistence {
    fn configure_v2_enrollment(&mut self, _enabled: bool) {}
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError>;
    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError>;
    fn list_recovery_candidates(&self) -> Vec<JobSnapshot>;
    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError>;
    fn fail(
        &mut self,
        job_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError>;
    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError>;
    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError>;

    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError>;
    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError>;
    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError>;
    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError>;
    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError>;

    fn has_execution_spec(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(true)
    }

    /// Persist one state transition only while `attempt` still owns the Job.
    ///
    /// `expected` is the durable state the caller believes it is leaving. The
    /// write is a single conditional update of
    /// `(state = expected, attempt_count = attempt)`; it returns `false` when
    /// the row already moved (a newer attempt, a cancellation, or any other
    /// writer won), meaning this caller lost the arbitration and must stop
    /// writing states and events. The default implementation has no
    /// attempt-identity knowledge and always writes, matching in-memory and
    /// stub persistence whose callers do not claim attempts.
    fn persist_state_for_attempt(
        &mut self,
        job_id: &str,
        _attempt: u32,
        _expected: JobState,
        next: JobState,
    ) -> Result<bool, ExecutorError> {
        let mut snapshot = self.snapshot(job_id)?;
        snapshot.state = next;
        self.persist_state(&snapshot)?;
        Ok(true)
    }

    /// Atomically complete a job from `Downloaded` to `Complete`, recording
    /// `DownloadCompleted` and `Completed` events in the same transaction,
    /// fenced by `attempt`. Returns `Ok(false)` (idempotent no-op) when the
    /// job is already `Complete`. Returns `Err` when the attempt no longer
    /// owns the job or the job is in a state other than `Downloaded` or
    /// `Complete`.
    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError>;

    /// Whether a durable archive commit boundary (a PREPARED/COMMITTED
    /// recovery journal) already owns this job.
    ///
    /// The default implementation has no journal, so cancellation is never
    /// refused. Storage-backed persistence overrides this with the real
    /// journal check.
    fn archive_commit_started(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(false)
    }

    /// Atomically persist the cancellation decision against any durable
    /// archive commit boundary.
    ///
    /// The default implementation has no journal knowledge and persists
    /// unconditionally, matching pre-v2 behaviour. Storage-backed
    /// persistence re-checks the journal inside the same transaction that
    /// writes the cancelled state, so a PREPARED commit is never overwritten.
    fn persist_cancellation(&mut self, job_id: &str) -> Result<CancelArbitration, ExecutorError> {
        let mut snapshot = self.snapshot(job_id)?;
        snapshot.state = JobState::Cancelled;
        self.persist_state(&snapshot)?;
        Ok(CancelArbitration::Cancelled)
    }

    /// Resolve the most recently updated Job for a Tweet, when one exists.
    ///
    /// Default returns `None`; storage-backed adapters override this so the
    /// browser `query_status` boundary can answer by Tweet ID.
    fn snapshot_for_tweet(&self, _tweet_id: &str) -> Result<Option<JobSnapshot>, ExecutorError> {
        Ok(None)
    }
}

/// Outcome of the durable cancellation decision taken by
/// [`JobPersistence::persist_cancellation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelArbitration {
    /// The job is durably cancelled and its state-change event is written.
    Cancelled,
    /// A durable PREPARED/COMMITTED archive boundary exists: the commit wins
    /// and no cancellation state was written.
    CommitWins,
    /// The job was already terminal; nothing was written.
    AlreadyDecided,
}

/// Connection factory contract for isolated executor Job contexts.
pub trait JobDatabaseFactory {
    fn open_job_database(&self, job_id: &str) -> Result<Database, ExecutorError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct InMemoryJobDatabaseFactory;

impl JobDatabaseFactory for InMemoryJobDatabaseFactory {
    fn open_job_database(&self, _job_id: &str) -> Result<Database, ExecutorError> {
        Database::open_in_memory().map_err(|error| ExecutorError::Persistence(error.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct InMemoryJobPersistence {
    jobs: HashMap<String, JobSnapshot>,
    events: HashMap<String, Vec<ExecutorEvent>>,
    attempts: HashMap<String, u32>,
}

impl JobPersistence for InMemoryJobPersistence {
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        if let Some(existing) = self
            .jobs
            .values()
            .find(|job| job.tweet_id == request.tweet_id && job.state.is_active())
        {
            return Ok(SubmitResult {
                job: existing.clone(),
                created: false,
            });
        }
        let snapshot = JobSnapshot {
            job_id: request.job_id.clone(),
            tweet_id: request.tweet_id.clone(),
            state: JobState::Queued,
        };
        self.jobs.insert(request.job_id.clone(), snapshot.clone());
        Ok(SubmitResult {
            job: snapshot,
            created: true,
        })
    }

    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.jobs
            .get(job_id)
            .cloned()
            .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))
    }

    fn list_recovery_candidates(&self) -> Vec<JobSnapshot> {
        self.jobs
            .values()
            .filter(|job| job.state.is_active() || job.state == JobState::Interrupted)
            .cloned()
            .collect()
    }

    fn snapshot_for_tweet(&self, tweet_id: &str) -> Result<Option<JobSnapshot>, ExecutorError> {
        let mut matches = self
            .jobs
            .values()
            .filter(|job| job.tweet_id == tweet_id)
            .collect::<Vec<_>>();
        matches.sort_by_key(|job| job.job_id.clone());
        Ok(matches.pop().cloned())
    }

    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
        let stored = self
            .jobs
            .get_mut(&snapshot.job_id)
            .ok_or_else(|| ExecutorError::UnknownJob(snapshot.job_id.clone()))?;
        stored.state = snapshot.state;
        Ok(())
    }

    fn fail(
        &mut self,
        job_id: &str,
        _error_code: &str,
        _error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let mut snapshot = self.snapshot(job_id)?;
        if snapshot.state == JobState::Queued {
            let validating = JobSnapshot {
                state: JobState::Validating,
                ..snapshot.clone()
            };
            self.persist_state(&validating)?;
            snapshot = validating;
        }
        if snapshot.state != JobState::Failed {
            let next = snapshot
                .state
                .transition_to(JobState::Failed)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: snapshot.state,
                    to: JobState::Failed,
                })?;
            let updated = JobSnapshot {
                state: next,
                ..snapshot
            };
            self.persist_state(&updated)?;
            return Ok(updated);
        }
        Ok(snapshot)
    }

    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError> {
        if !self.jobs.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(event);
        Ok(())
    }

    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        if !self.jobs.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        Ok(self.events.get(job_id).cloned().unwrap_or_default())
    }

    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError> {
        self.record_event(job_id, ExecutorEvent::StateChanged { from, to })
    }

    fn has_execution_spec(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(true)
    }

    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        if !self.jobs.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        let attempt = self.attempts.entry(job_id.to_owned()).or_default();
        *attempt = attempt.saturating_add(1);
        Ok(*attempt)
    }

    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
        Ok(self.attempts.get(job_id).copied().unwrap_or_default())
    }

    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError> {
        let current = self.snapshot(job_id)?;
        if current.state != from || self.current_attempt(job_id)? != expected_attempt {
            return Ok(None);
        }
        let attempt = self.begin_attempt(job_id)?;
        if !self.persist_state_for_attempt(job_id, attempt, from, to)? {
            return Ok(None);
        }
        Ok(Some(attempt))
    }

    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        Ok(self.attempts.get(job_id).copied().unwrap_or_default() == attempt)
    }

    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError> {
        let snapshot = self.snapshot(job_id)?;
        if snapshot.state == JobState::Complete {
            return Ok(snapshot);
        }
        if snapshot.state != JobState::Downloaded {
            return Err(ExecutorError::Persistence(
                "cannot complete job in non-Downloaded state".into(),
            ));
        }
        if !self.attempt_is_current(job_id, attempt)? {
            return Err(ExecutorError::Persistence("stale attempt".into()));
        }
        let next = snapshot
            .state
            .transition_to(JobState::Complete)
            .map_err(|_| ExecutorError::InvalidTransition {
                from: snapshot.state,
                to: JobState::Complete,
            })?;
        let updated = JobSnapshot {
            state: next,
            ..snapshot.clone()
        };
        self.persist_state(&updated)?;
        self.record_event(job_id, ExecutorEvent::DownloadCompleted { files_copied })?;
        self.record_event(
            job_id,
            ExecutorEvent::Completed {
                archive_directory: archive_directory.to_owned(),
            },
        )?;
        Ok(updated)
    }
}

/// SQLite persistence context used by one executor/application operation.
///
/// The connection is intentionally owned by the operation context rather than
/// by `RuntimeState`, so opening or using it never requires holding the global
/// runtime mutex across worker I/O.
pub struct StorageJobPersistence {
    database: Database,
    tweet_rows: HashMap<String, i64>,
    events: HashMap<String, Vec<ExecutorEvent>>,
    v2_enrollment: bool,
}

impl StorageJobPersistence {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        Self::from_database(Database::open(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    pub fn open_in_memory() -> Result<Self, String> {
        Self::from_database(Database::open_in_memory().map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    pub fn from_factory<F: JobDatabaseFactory>(
        factory: &F,
        job_id: &str,
    ) -> Result<Self, ExecutorError> {
        Self::from_database(factory.open_job_database(job_id)?)
    }

    fn from_database(database: Database) -> Result<Self, ExecutorError> {
        Ok(Self {
            database,
            tweet_rows: HashMap::new(),
            events: HashMap::new(),
            v2_enrollment: false,
        })
    }

    fn now() -> String {
        // Production persistence must record the real event time. Tests that
        // need a deterministic timestamp pass it explicitly (for example into
        // `submit`) instead of relying on a shared fixed value.
        crate::clock::now_iso()
    }

    fn remember_tweet(&mut self, tweet_id: &str) -> Result<i64, ExecutorError> {
        if let Some(row_id) = self.tweet_rows.get(tweet_id) {
            return Ok(*row_id);
        }
        let row_id = match self.database.tweet_row_id(tweet_id) {
            Ok(row_id) => row_id,
            Err(_) => self
                .database
                .insert_tweet(
                    tweet_id,
                    &format!("https://x.com/test/status/{tweet_id}"),
                    "post",
                    "",
                    &Self::now(),
                )
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
        };
        self.tweet_rows.insert(tweet_id.to_owned(), row_id);
        Ok(row_id)
    }

    fn stored_events(&self, job_id: &str) -> Result<Vec<(String, Option<String>)>, ExecutorError> {
        self.database
            .list_events_for_job(job_id, 100)
            .map(|events| {
                events
                    .into_iter()
                    .rev()
                    .map(|event| (event.event_type, event.payload_json))
                    .collect()
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn stored_job_summary(&self, job_id: &str) -> Result<JobSummary, ExecutorError> {
        self.database
            .job_summary(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))
    }

    pub fn recovery_facts(
        &self,
        job_id: &str,
        final_directory: CommitRecoveryDirectory,
        staging_directory: CommitRecoveryDirectory,
    ) -> Result<CommitRecoveryFacts, ExecutorError> {
        let summary = self.stored_job_summary(job_id)?;
        Ok(CommitRecoveryFacts {
            job_id: summary.job_id,
            state: summary.state,
            final_directory,
            staging_directory,
        })
    }
}

/// Runtime-owned executor configuration and lifecycle boundary for the R1 worker.
/// Production runner resources are created from immutable Job execution specs.
pub struct ExecutorRuntime {
    executor: JobExecutor,
    database_path: PathBuf,
    config: ExecutorConfig,
    #[cfg(target_os = "linux")]
    containment: Option<LinuxExecutionContainment>,
}

impl ExecutorRuntime {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        let database_path = database_path.into();
        let archive_root = database_path
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("."))
            .to_owned();
        let config = ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root,
            staging_root: database_path
                .parent()
                .map(|path| path.join("_staging"))
                .unwrap_or_else(|| PathBuf::from("_staging")),
            database_path: database_path.clone(),
            sidecar_program: std::env::var("XARCHIVE_SIDECAR_PROGRAM").ok(),
            gallery_dl_program: std::env::var("XARCHIVE_GALLERY_DL_PROGRAM").ok(),
            sidecar_args: std::env::var("XARCHIVE_SIDECAR_ARGS")
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
            aria2_program: std::env::var("XARCHIVE_ARIA2_PROGRAM").ok(),
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        };
        Self::with_config(config)
    }

    pub fn with_config(config: ExecutorConfig) -> Self {
        let factory = ProductionExecutionFactory::new(config.clone());
        Self::with_production_factory(
            config,
            factory,
            #[cfg(target_os = "linux")]
            None,
        )
    }

    /// Explicit Linux runtime provisioning. No environment inference or
    /// uncontained fallback is performed. Provisioned Linux pairs v2 enrollment
    /// with public startup recovery; existing job contracts are not rewritten.
    /// CROSS_PLATFORM_CHANGE_REQUIRED: runtime launch configuration boundary.
    #[cfg(target_os = "linux")]
    pub fn with_linux_containment(
        config: ExecutorConfig,
        containment: LinuxExecutionContainment,
    ) -> Self {
        let factory = ProductionExecutionFactory::new(config.clone())
            .with_linux_containment(containment.clone());
        Self::with_production_factory(config, factory, Some(containment))
    }

    fn with_production_factory(
        config: ExecutorConfig,
        factory: ProductionExecutionFactory,
        #[cfg(target_os = "linux")] containment: Option<LinuxExecutionContainment>,
    ) -> Self {
        Self {
            executor: JobExecutor::with_capacity_and_factory(
                DEFAULT_QUEUE_CAPACITY,
                Arc::new(factory),
            ),
            database_path: config.database_path.clone(),
            config,
            #[cfg(target_os = "linux")]
            containment,
        }
    }

    pub fn service(&self) -> ArchiveApplicationService {
        let service = ArchiveApplicationService::new(&self.executor);
        #[cfg(target_os = "linux")]
        let service = ArchiveApplicationService {
            v2_enrollment: self.containment.is_some(),
            ..service
        };
        service
    }

    pub fn open_persistence(&self) -> Result<StorageJobPersistence, String> {
        StorageJobPersistence::open(&self.database_path)
    }

    /// Build an owned production session boundary for a single persisted Job.
    ///
    /// The lock records live beside the durable database, under a stable
    /// workspace identity shared by every process using this configured
    /// workspace. This does not opt the Job into v2 publication or recovery.
    #[cfg(unix)]
    pub(crate) fn production_session_boundary(
        &self,
    ) -> Result<SessionExecutionBoundary<StorageJobPersistence, ProductionCoordinator>, ExecutorError>
    {
        let persistence = StorageJobPersistence::open(&self.database_path)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let coordinator = ProductionCoordinator::open(&self.database_path)?;
        Ok(SessionExecutionBoundary::new(
            self.service(),
            persistence,
            coordinator,
            self.config.archive_root.clone(),
        ))
    }

    /// Execute one persisted Job through the owned `SessionExecutionBoundary`.
    ///
    /// Routes through `SessionExecutionBoundary::execute_on_runner`, which
    /// performs runner admission → Job/destination scope acquisition → durable
    /// claim under the guard → factory creation → runner execution →
    /// attempt-fenced terminal writes and finalization → session release →
    /// admission release. The live session is retained through finalization,
    /// so no detached result-only seam can be mistaken for lifecycle progress.
    ///
    /// Production scheduling remains fail-closed: this does not switch the
    /// legacy `execute_persisted_from_factory` path until B4/B5 acceptance
    /// gates pass. It is available for callers that opt into the owned
    /// session boundary (e.g. startup recovery via `recover_startup`).
    #[cfg(unix)]
    pub(crate) fn execute_persisted_on_runner(
        &self,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let boundary = self.production_session_boundary()?;
        let outcome = boundary.execute_on_runner(job_id.to_owned())?;
        match outcome {
            SessionRunOutcome::Finished(snapshot) => Ok(snapshot),
            SessionRunOutcome::Reschedulable { job_id, .. } => Err(ExecutorError::Execution {
                error_code: "EXECUTION_RESCHEDULABLE".to_owned(),
                error_message: format!("job {job_id} is reschedulable"),
                persistence_already_updated: false,
            }),
        }
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn linux_containment(&self) -> Option<LinuxExecutionContainment> {
        self.containment.clone()
    }

    pub fn aria2_program(&self) -> Option<&str> {
        self.config.aria2_program.as_deref()
    }

    pub fn gallery_dl_program(&self) -> Option<&str> {
        self.config.gallery_dl_program.as_deref()
    }

    pub fn recover_startup(
        &self,
    ) -> Result<thread::JoinHandle<Vec<(String, Result<bool, String>)>>, ExecutorError> {
        #[cfg(target_os = "linux")]
        let enabled = self.containment.is_some();
        #[cfg(not(target_os = "linux"))]
        let enabled = false;
        self.recover_startup_with_v2(enabled)
    }

    // Explicit provisioning selects Linux v2; other platforms/unprovisioned
    // runtimes keep historical gates. Never retry as a fresh attempt.
    fn recover_startup_with_v2(
        &self,
        enable_v2: bool,
    ) -> Result<thread::JoinHandle<Vec<(String, Result<bool, String>)>>, ExecutorError> {
        let database_path = self.database_path.clone();
        let archive_root = self.config.archive_root.clone();
        let staging_root = self.config.staging_root.clone();
        #[cfg(target_os = "linux")]
        let containment = self.containment.clone();
        let service = self.service();
        thread::Builder::new()
            .name("xarchive-startup-recovery".to_owned())
            .spawn(move || {
                // Admission precedes candidate inspection and writer-domain drain.
                // Retain this reader across the entire recovery operation.
                #[cfg(target_os = "linux")]
                let _workspace = match ProductionCoordinator::open(&database_path) {
                    Ok(coordinator) => coordinator,
                    Err(error) => return vec![(String::new(), Err(error.to_string()))],
                };
                let mut v2_results = Vec::new();
                let mut persistence = match StorageJobPersistence::open(&database_path) {
                    Ok(persistence) => persistence,
                    Err(error) => return vec![(String::new(), Err(error))],
                };
                let candidates = persistence.list_recovery_candidates();
                for candidate in candidates {
                    // Gated v2 validates journals only after taking scopes. In
                    // particular corrupt journals must not reach the historical
                    // unfenced failure path below.
                    #[cfg(target_os = "linux")]
                    if enable_v2 && matches!(
                        Database::open(&database_path).and_then(|database| database.archive_recovery_contract(&candidate.job_id)),
                        Ok(Some(2))
                    ) {
                        let result = recover_v2_on_runner(
                            &service, database_path.clone(), archive_root.clone(),
                            staging_root.clone(), containment.clone(),
                            candidate.job_id.clone(), candidate.tweet_id.clone(),
                        );
                        v2_results.push((candidate.job_id.clone(), result.map_err(|error| error.to_string())));
                        continue;
                    }
                    // R4b-2: before any recovery decision, drain the crashed
                    // attempt's recorded writer domains when containment is
                    // provisioned. Only NoRecord/AlreadyAbsent/Drained admit;
                    // anything else fails the job closed. Re-dispatched jobs
                    // inherit the same drain at claim time, so this stays
                    // idempotent for them.
                    #[cfg(target_os = "linux")]
                    if containment.is_some() {
                        let drained: Result<(), String> = (|| {
                            let record_root = containment_record_root(&staging_root)
                                .map_err(|error| error.to_string())?;
                            let attempt = Database::open(&database_path)
                                .and_then(|database| database.archive_attempt(&candidate.job_id))
                                .map_err(|error| error.to_string())?;
                            recover_writer_domain_set(
                                &record_root,
                                &staging_root,
                                &candidate.job_id,
                                u64::from(attempt),
                            )
                            .map_err(|error| error.to_string())
                        })();
                        if let Err(error) = drained {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                "WRITER_DOMAIN_RECOVERY_FAILED",
                                &error,
                            );
                            continue;
                        }
                    }
                    let recovery_contract = match Database::open(&database_path)
                        .and_then(|database| database.startup_recovery_dispatch(&candidate.job_id))
                    {
                        Ok(contract) => contract,
                        Err(error) => {
                            let error_code = match error {
                                xarchive_storage::StorageError::InvalidMetadata(_) => {
                                    "ARCHIVE_COMMIT_RECOVERY_FAILED"
                                }
                                _ => "ARCHIVE_RECOVERY_DATABASE_ERROR",
                            };
                            let _ = persistence.fail(&candidate.job_id, error_code, &error.to_string());
                            continue;
                        }
                    };
                    if let ArchiveRecoveryDispatch::Unknown(_) = recovery_contract {
                        let _ = persistence.fail(&candidate.job_id, "ARCHIVE_RECOVERY_UNSUPPORTED_VERSION", "unsupported internal archive recovery contract version");
                        continue;
                    }

                    let has_execution_spec =
                        persistence.has_execution_spec(&candidate.job_id).unwrap_or(false);
                    if !has_execution_spec && recovery_contract != ArchiveRecoveryDispatch::Legacy {
                        let _ = persistence.fail(
                            &candidate.job_id,
                            "EXECUTION_SPEC_MISSING",
                            "archive execution spec is missing; recovery cannot reconstruct the request",
                        );
                        continue;
                    }

                    if recovery_contract == ArchiveRecoveryDispatch::InternalRenameV2 {
                        #[cfg(not(target_os = "linux"))]
                        let _ = enable_v2;
                        // V2 execution and finalization remain gated until the
                        // service-level transaction/replay contract is enabled.
                        // Do not restart an incomplete v2 job as a fresh run.
                        let _ = persistence.fail(
                            &candidate.job_id,
                            "ARCHIVE_RECOVERY_V2_NOT_READY",
                            "v2 archive recovery is not yet enabled",
                        );
                        continue;
                    }

                    if candidate.state != JobState::Downloaded
                        && recovery_contract != ArchiveRecoveryDispatch::InternalV1
                    {
                        if !has_execution_spec {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                "ARCHIVE_RECOVERY_LEGACY_INCOMPLETE",
                                "legacy archive has no execution spec or committed recovery facts",
                            );
                            continue;
                        }
                        // Route through the owned SessionExecutionBoundary so the
                        // session is retained through drain and fenced finalization
                        // (A4). The boundary owns persistence and coordinator, so
                        // they are moved into the runner operation; no caller-side
                        // claim or legacy dispatch remains. Open a fresh
                        // persistence for the boundary so the loop's `persistence`
                        // remains available to later recovery branches.
                        let boundary_persistence = match StorageJobPersistence::open(&database_path) {
                            Ok(p) => p,
                            Err(_) => continue,
                        };
                        let coordinator = match ProductionCoordinator::open(&database_path) {
                            Ok(c) => c,
                            Err(_) => continue,
                        };
                        let boundary = SessionExecutionBoundary::new(
                            service.clone(),
                            boundary_persistence,
                            coordinator,
                            archive_root.clone(),
                        );
                        match boundary.execute_on_runner(candidate.job_id.clone()) {
                            Ok(SessionRunOutcome::Finished(_)) => continue,
                            Ok(SessionRunOutcome::Reschedulable { .. }) => continue,
                            Err(_) => continue,
                        }
                    }

                    if recovery_contract == ArchiveRecoveryDispatch::InternalV1 {
                        let final_directory = PathBuf::from("Tweets").join(&candidate.tweet_id);
                        let database = match Database::open(&database_path) {
                            Ok(database) => database,
                            Err(error) => {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "ARCHIVE_RECOVERY_DATABASE_ERROR",
                                    &error.to_string(),
                                );
                                continue;
                            }
                        };
                        let tweet_row_id = match database.tweet_row_id(&candidate.tweet_id) {
                            Ok(row_id) => row_id,
                            Err(error) => {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "ARCHIVE_COMMIT_RECOVERY_FAILED",
                                    &error.to_string(),
                                );
                                continue;
                            }
                        };
                        let files = match FileStore::with_staging_root(&archive_root, &staging_root) {
                            Ok(files) => files,
                            Err(error) => {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "ARCHIVE_RECOVERY_FILESYSTEM_ERROR",
                                    &error.to_string(),
                                );
                                continue;
                            }
                        };
                        let mut archive = ArchiveService::new(database, files);
                        match archive.recover_internal_archive(
                            &candidate.job_id,
                            tweet_row_id,
                            &final_directory,
                        ) {
                            Ok(_) if candidate.state == JobState::Downloaded => {
                                let _ = service.complete_persisted(
                                    &mut persistence,
                                    &candidate.job_id,
                                    &final_directory.to_string_lossy(),
                                );
                            }
                            Ok(_) => {}
                            Err(error) => {
                                let _ = persistence.fail(
                                    &candidate.job_id,
                                    "ARCHIVE_COMMIT_RECOVERY_FAILED",
                                    &error.to_string(),
                                );
                            }
                        }
                        continue;
                    }

                    let final_directory = PathBuf::from("Tweets").join(&candidate.tweet_id);
                    let files = match FileStore::with_staging_root(&archive_root, &staging_root) {
                        Ok(files) => files,
                        Err(error) => {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                "ARCHIVE_RECOVERY_FILESYSTEM_ERROR",
                                &error.to_string(),
                            );
                            continue;
                        }
                    };
                    let facts = CommitRecoveryFacts {
                        job_id: candidate.job_id.clone(),
                        state: candidate.state,
                        final_directory: if files
                            .recovery_directory_exists(&candidate.job_id, &final_directory)
                            .unwrap_or(false)
                        {
                            CommitRecoveryDirectory::Present
                        } else {
                            CommitRecoveryDirectory::Missing
                        },
                        staging_directory: if files
                            .recovery_directory_exists(&candidate.job_id, Path::new("_staging"))
                            .unwrap_or(false)
                        {
                            CommitRecoveryDirectory::Present
                        } else {
                            CommitRecoveryDirectory::Missing
                        },
                    };

                    match facts.decide(&final_directory.to_string_lossy()) {
                        CommitRecoveryDecision::ResumeStaging => {
                            let database = match Database::open(&database_path) {
                                Ok(database) => database,
                                Err(error) => {
                                    let _ = persistence.fail(
                                        &candidate.job_id,
                                        "ARCHIVE_RECOVERY_DATABASE_ERROR",
                                        &error.to_string(),
                                    );
                                    continue;
                                }
                            };
                            let tweet_row_id = match database.tweet_row_id(&candidate.tweet_id) {
                                Ok(row_id) => row_id,
                                Err(error) => {
                                    let _ = persistence.fail(
                                        &candidate.job_id,
                                        "ARCHIVE_RECOVERY_DATABASE_ERROR",
                                        &error.to_string(),
                                    );
                                    continue;
                                }
                            };
                            let mut archive = ArchiveService::new(database, files);
                            match archive.recover_staging_archive(
                                &candidate.job_id,
                                tweet_row_id,
                                &final_directory,
                            ) {
                                Ok(_) => {
                                    let _ = service.complete_persisted(
                                        &mut persistence,
                                        &candidate.job_id,
                                        &final_directory.to_string_lossy(),
                                    );
                                }
                                Err(error) => {
                                    let _ = persistence.fail(
                                        &candidate.job_id,
                                        "ARCHIVE_COMMIT_RECOVERY_FAILED",
                                        &error.to_string(),
                                    );
                                }
                            }
                        }
                        CommitRecoveryDecision::Complete { archive_directory } => {
                            let _ = service.complete_persisted(
                                &mut persistence,
                                &candidate.job_id,
                                &archive_directory,
                            );
                        }
                        CommitRecoveryDecision::MarkFailed {
                            error_code,
                            error_message,
                        } => {
                            let _ = persistence.fail(
                                &candidate.job_id,
                                &error_code,
                                &error_message,
                            );
                        }
                        CommitRecoveryDecision::Skip | CommitRecoveryDecision::NotApplicable => {}
                    }
                }
                v2_results
            })
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    pub fn is_running(&self) -> bool {
        self.executor.is_running()
    }

    pub fn shutdown(self) -> Result<(), ExecutorError> {
        self.executor.shutdown()
    }

    pub fn shutdown_in_place(&mut self) -> Result<(), ExecutorError> {
        self.executor.shutdown_in_place()
    }
}

/// Production coordinator backed by the Linux advisory-lock adapter.
///
/// Lives beside the durable database under a stable workspace lock
/// directory. The kernel releases held records when the owner process
/// exits (including SIGKILL), so a crashed owner cannot keep a successor
/// out. Windows has no adapter here: the Windows Owner implements and
/// validates the NTFS coordinator separately.
#[cfg(unix)]
pub(crate) struct ProductionCoordinator {
    inner: LinuxCoordinator,
    #[cfg(target_os = "linux")]
    _workspace: xarchive_workflow::maintenance::WorkspaceGuard,
}

#[cfg(unix)]
impl ProductionCoordinator {
    /// Open a production coordinator whose lock files live beside the
    /// durable database.
    pub(crate) fn open(database_path: &Path) -> Result<Self, ExecutorError> {
        let lock_dir = database_path
            .parent()
            .ok_or_else(|| ExecutorError::Execution {
                error_code: "NO_DATABASE_PARENT".to_owned(),
                error_message: "database path has no parent directory for coordination locks"
                    .to_owned(),
                persistence_already_updated: false,
            })?
            .join("locks");
        let inner = LinuxCoordinator::new(lock_dir).map_err(|error| ExecutorError::Execution {
            error_code: "COORDINATOR_OPEN_FAILED".to_owned(),
            error_message: format!("failed to open production coordinator: {error}"),
            persistence_already_updated: false,
        })?;
        #[cfg(target_os = "linux")]
        let workspace =
            xarchive_workflow::maintenance::WorkspaceGuard::try_enter(inner.lock_directory())
                .map_err(|error| ExecutorError::Execution {
                    error_code: "WORKSPACE_ADMISSION_UNAVAILABLE".to_owned(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                })?;
        Ok(Self {
            inner,
            #[cfg(target_os = "linux")]
            _workspace: workspace,
        })
    }
}

#[cfg(unix)]
impl Coordinator for ProductionCoordinator {
    type Guard = LinuxGuard;

    fn acquire(&self, scope: &LockScope) -> Result<Self::Guard, LockError> {
        self.inner.acquire(scope)
    }

    fn acquire_all(&self, scopes: &[LockScope]) -> Result<Self::Guard, LockError> {
        self.inner.acquire_all(scopes)
    }
}

/// Replay the original journaled attempt under runner admission and live scopes.
/// `false` means contention: leave the durable job untouched for a later retry.
/// This never starts a writer or claims a successor attempt.
#[cfg(target_os = "linux")]
fn recover_v2_on_runner(
    service: &ArchiveApplicationService,
    database_path: PathBuf,
    archive_root: PathBuf,
    staging_root: PathBuf,
    containment: Option<LinuxExecutionContainment>,
    job_id: String,
    tweet_id: String,
) -> Result<bool, ExecutorError> {
    if !service.executor.runner_try_admit() {
        return Ok(false);
    }
    let admission = service.executor.owned_runner_admission();
    let recovery_service = service.clone();
    service.executor.run_operation(admission, move || {
        let recover = || -> Result<bool, String> {
            let coordinator =
                ProductionCoordinator::open(&database_path).map_err(|error| error.to_string())?;
            let relative = PathBuf::from("Tweets").join(&tweet_id);
            let destination = archive_root.join(&relative);
            let reservation =
                match ExecutionReservation::acquire(&coordinator, &job_id, &destination) {
                    Ok(reservation) => reservation,
                    Err(SessionError::Contended) => return Ok(false),
                    Err(error) => return Err(error.to_string()),
                };
            let database = Database::open(&database_path).map_err(|error| error.to_string())?;
            let journal = database
                .archive_recovery_manifest_v2(&job_id)
                .map_err(|error| error.to_string())?;
            let attempt = database
                .archive_attempt(&job_id)
                .map_err(|error| error.to_string())?;
            // Before PREPARED, drain the old attempt before admitting a
            // successor. Published states still require the durable journal.
            let state = database
                .job_state(&job_id)
                .map_err(|error| error.to_string())?;
            if journal.is_none()
                && matches!(
                    state,
                    JobState::Queued
                        | JobState::Validating
                        | JobState::MetadataReady
                        | JobState::Downloading
                        | JobState::Interrupted
                )
            {
                if containment.is_none() {
                    return Err("v2 recovery requires provisioned writer containment".into());
                }
                let record_root =
                    containment_record_root(&staging_root).map_err(|error| error.to_string())?;
                recover_writer_domain_set(&record_root, &staging_root, &job_id, u64::from(attempt))
                    .map_err(|error| error.to_string())?;
                drop(reservation);
                let persistence = StorageJobPersistence::open(&database_path)?;
                let mut boundary = SessionExecutionBoundary::new(
                    recovery_service.clone(),
                    persistence,
                    coordinator,
                    archive_root.clone(),
                );
                return boundary
                    .execute_session(&job_id)
                    .map_err(|error| error.to_string())
                    .and_then(|outcome| match outcome {
                        SessionRunOutcome::Finished(snapshot)
                            if snapshot.state == JobState::Complete =>
                        {
                            Ok(true)
                        }
                        SessionRunOutcome::Reschedulable { .. } => Ok(false),
                        SessionRunOutcome::Finished(_) => {
                            Err("startup execution did not complete; inspect retained job".into())
                        }
                    });
            }
            let journal =
                journal.ok_or_else(|| "v2 archive recovery journal is missing".to_owned())?;
            if attempt != journal.attempt_count {
                return Err("v2 recovery attempt does not match the journal".into());
            }
            // Claim here binds the existing durable identity under the scopes;
            // it deliberately does not begin/increment an attempt or change state.
            let session = reservation
                .claim(|| match AttemptIdentity::new(&job_id, attempt) {
                    Ok(identity) => ClaimOutcome::Claimed(identity),
                    Err(_) => ClaimOutcome::Failed,
                })
                .map_err(|error| error.to_string())?;
            if containment.is_none() {
                return Err("v2 recovery requires provisioned writer containment".into());
            }
            let record_root =
                containment_record_root(&staging_root).map_err(|error| error.to_string())?;
            recover_writer_domain_set(&record_root, &staging_root, &job_id, u64::from(attempt))
                .map_err(|error| error.to_string())?;
            let tweet_row_id = database
                .tweet_row_id(&tweet_id)
                .map_err(|error| error.to_string())?;
            let files = FileStore::with_staging_root(&archive_root, &staging_root)
                .map_err(|error| error.to_string())?;
            let mut archive = xarchive_storage::ArchiveService::new(database, files);
            archive
                .recover_internal_rename_v2(
                    &job_id,
                    tweet_row_id,
                    &relative,
                    &StorageJobPersistence::now(),
                )
                .map_err(|error| error.to_string())?;
            // Storage owns journal/media finalization; Desktop owns the final
            // attempt-fenced Job/event transaction, still under this session.
            let mut persistence = StorageJobPersistence::open(&database_path)?;
            if persistence
                .snapshot(&job_id)
                .map_err(|error| error.to_string())?
                .state
                == JobState::Downloaded
            {
                persistence
                    .complete_terminal_for_attempt(
                        &job_id,
                        attempt,
                        &relative.to_string_lossy(),
                        u32::try_from(
                            serde_json::from_str::<xarchive_storage::ArchiveRecoveryManifestV2>(
                                &journal.manifest_json,
                            )
                            .map_err(|error| error.to_string())?
                            .media
                            .len(),
                        )
                        .map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
            }
            drop(session);
            Ok(true)
        };
        recover().map_err(ExecutorError::Persistence)
    })
}

pub(crate) fn state_sort_priority(state: &JobState) -> u8 {
    match state {
        JobState::Complete => 7,
        JobState::Downloaded => 6,
        JobState::Downloading => 5,
        JobState::TgMediaUploading => 5,
        JobState::TgMetadataSent => 4,
        JobState::TgMetadataSending => 4,
        JobState::MetadataReady => 4,
        JobState::Validating => 3,
        JobState::Queued => 2,
        JobState::Interrupted => 1,
        JobState::AuthRequired => 1,
        JobState::Failed | JobState::Cancelled => 0,
    }
}

impl JobPersistence for StorageJobPersistence {
    fn configure_v2_enrollment(&mut self, enabled: bool) {
        self.v2_enrollment = enabled;
    }
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        let tweet_row_id = self.remember_tweet(&request.tweet_id)?;
        let recovery_contract =
            matches!(request.schema_version, 2..=4).then_some(if self.v2_enrollment {
                xarchive_storage::ArchiveRecoveryContract::InternalRenameV2
            } else {
                xarchive_storage::ArchiveRecoveryContract::InternalV1
            });
        let created = self
            .database
            .create_archive_job_with_optional_recovery_contract(
                &request.job_id,
                tweet_row_id,
                request.schema_version,
                &request.request_id,
                &request.request_json,
                recovery_contract,
                &Self::now(),
            )
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if created {
            let summary = self.stored_job_summary(&request.job_id)?;
            return Ok(SubmitResult {
                job: JobSnapshot::from_job_summary(&summary),
                created: true,
            });
        }
        let job = self
            .database
            .active_job_for_tweet(&request.tweet_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .ok_or_else(|| ExecutorError::UnknownJob(request.job_id.clone()))?;
        Ok(SubmitResult {
            job: JobSnapshot::from_job_summary(&job),
            created: false,
        })
    }

    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        let summary = self.stored_job_summary(job_id)?;
        Ok(JobSnapshot::from_job_summary(&summary))
    }

    fn has_execution_spec(&self, job_id: &str) -> Result<bool, ExecutorError> {
        self.database
            .archive_job_request(job_id)
            .map(|request| request.is_some())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn list_recovery_candidates(&self) -> Vec<JobSnapshot> {
        self.database
            .list_recovery_candidate_jobs()
            .unwrap_or_default()
            .into_iter()
            .map(|job| JobSnapshot::from_job_summary(&job))
            .collect()
    }

    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
        let current = self
            .database
            .job_state(&snapshot.job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if current != snapshot.state {
            self.database
                .transition_job(&snapshot.job_id, snapshot.state, &Self::now())
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        }
        Ok(())
    }

    /// Attempt-fenced state transition backed by one conditional SQL update.
    ///
    /// `persist_state_if_attempt` is the existing atomic primitive: the update
    /// only applies while the row is still in `expected` with
    /// `attempt_count = attempt`, and it writes the state-change event inside
    /// the same transaction. Returning `false` therefore means a newer
    /// attempt, a durable cancellation, or another writer won the race.
    fn persist_state_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        expected: JobState,
        next: JobState,
    ) -> Result<bool, ExecutorError> {
        self.database
            .persist_state_if_attempt(job_id, attempt, expected, next, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn fail(
        &mut self,
        job_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.database
            .fail_job(
                job_id,
                JobState::Failed,
                error_code,
                error_message,
                &Self::now(),
            )
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        self.snapshot(job_id)
    }

    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError> {
        let job_event = event.to_job_event(job_id);
        if let Some(job_event) = job_event {
            // Database::transition_job persists StateChanged atomically with
            // the state update. Do not insert a duplicate event here.
            if !matches!(
                event,
                ExecutorEvent::StateChanged { .. }
                    | ExecutorEvent::Recovered { .. }
                    | ExecutorEvent::ShutdownInterrupted { .. }
            ) {
                self.database
                    .record_event(job_id, &job_event, &Self::now())
                    .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
            }
        }
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(event);
        Ok(())
    }

    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        if !self.events.contains_key(job_id) {
            return Err(ExecutorError::UnknownJob(job_id.to_owned()));
        }
        Ok(self.events.get(job_id).cloned().unwrap_or_default())
    }

    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError> {
        let applied = self
            .database
            .complete_terminal_for_attempt(
                job_id,
                attempt,
                archive_directory,
                files_copied,
                &Self::now(),
            )
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        if !applied {
            // Already Complete (idempotent) or attempt lost: return current snapshot.
            return self.snapshot(job_id);
        }
        // Mirror into the in-memory event cache so `events()` stays consistent.
        let snapshot = self.snapshot(job_id)?;
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(ExecutorEvent::DownloadCompleted { files_copied });
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(ExecutorEvent::Completed {
                archive_directory: archive_directory.to_owned(),
            });
        Ok(snapshot)
    }

    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        self.database
            .begin_archive_attempt(job_id, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
        self.database
            .archive_attempt(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError> {
        self.database
            .claim_archive_attempt(job_id, expected_attempt, from, to, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError> {
        self.events
            .entry(job_id.to_owned())
            .or_default()
            .push(ExecutorEvent::StateChanged { from, to });
        Ok(())
    }

    fn archive_commit_started(&self, job_id: &str) -> Result<bool, ExecutorError> {
        self.database
            .archive_commit_started(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn persist_cancellation(&mut self, job_id: &str) -> Result<CancelArbitration, ExecutorError> {
        let decision = self
            .database
            .decide_archive_cancellation(job_id, &Self::now())
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        Ok(match decision {
            xarchive_storage::ArchiveCancellationDecision::Cancelled => {
                CancelArbitration::Cancelled
            }
            xarchive_storage::ArchiveCancellationDecision::CommitWins => {
                CancelArbitration::CommitWins
            }
            xarchive_storage::ArchiveCancellationDecision::AlreadyDecided => {
                CancelArbitration::AlreadyDecided
            }
        })
    }

    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        self.database
            .archive_attempt(job_id)
            .map(|current| current == attempt)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))
    }

    fn snapshot_for_tweet(&self, tweet_id: &str) -> Result<Option<JobSnapshot>, ExecutorError> {
        Ok(self
            .database
            .active_job_for_tweet(tweet_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .map(|job| JobSnapshot::from_job_summary(&job)))
    }
}

#[derive(Clone)]
pub struct ArchiveApplicationService {
    executor: JobExecutorHandle,
    v2_enrollment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    Closed,
    QueueFull,
    ResponseClosed,
    UnknownJob(String),
    InvalidTransition {
        from: JobState,
        to: JobState,
    },
    Persistence(String),
    Execution {
        error_code: String,
        error_message: String,
        persistence_already_updated: bool,
    },
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("job executor is closed"),
            Self::QueueFull => formatter.write_str("job executor command queue is full"),
            Self::ResponseClosed => formatter.write_str("job executor response channel is closed"),
            Self::UnknownJob(job_id) => write!(formatter, "unknown job: {job_id}"),
            Self::InvalidTransition { from, to } => {
                write!(formatter, "invalid executor transition: {from:?} -> {to:?}")
            }
            Self::Persistence(message) => {
                write!(formatter, "executor persistence error: {message}")
            }
            Self::Execution {
                error_code,
                error_message,
                persistence_already_updated: _,
            } => write!(
                formatter,
                "job execution error [{error_code}]: {error_message}"
            ),
        }
    }
}

impl std::error::Error for ExecutorError {}

#[derive(Debug)]
struct JobRecord {
    snapshot: JobSnapshot,
    events: Vec<ExecutorEvent>,
    cancellation: CancellationToken,
}

#[derive(Clone, Default)]
struct CancellationRegistry(Arc<Mutex<HashMap<String, CancellationToken>>>);

impl CancellationRegistry {
    fn token(&self, job_id: &str) -> Result<CancellationToken, ExecutorError> {
        self.0
            .lock()
            .map_err(|_| ExecutorError::Execution {
                error_code: "CANCELLATION_REGISTRY_POISONED".to_owned(),
                error_message: "job cancellation registry was poisoned".to_owned(),
                persistence_already_updated: false,
            })
            .map(|mut tokens| tokens.entry(job_id.to_owned()).or_default().clone())
    }
}

enum Command {
    Submit {
        request: ArchiveJobRequest,
        response: mpsc::Sender<Result<SubmitResult, ExecutorError>>,
    },
    Start {
        job_id: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Cancel {
        job_id: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Snapshot {
        job_id: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Shutdown {
        response: mpsc::Sender<Result<(), ExecutorError>>,
    },
    Recover {
        jobs: Vec<JobSnapshot>,
        response: mpsc::Sender<Result<Vec<JobSnapshot>, ExecutorError>>,
    },
    Events {
        job_id: String,
        response: mpsc::Sender<Result<Vec<ExecutorEvent>, ExecutorError>>,
    },
    SidecarCrashed {
        job_id: String,
        error_message: String,
        response: mpsc::Sender<Result<JobSnapshot, ExecutorError>>,
    },
    Execute {
        job_id: String,
        execution: Box<dyn JobExecution>,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
    RunJob {
        job_id: String,
        snapshot: JobSnapshot,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
}

enum RunnerCommand {
    /// Run one owned lifecycle operation inline on the runner. The closure
    /// deliberately has no access to a dispatch handle, preventing recursive
    /// submission to the runner that is currently executing it.
    Operation {
        operation: Box<dyn RunnerOperation>,
    },
    Execute {
        job_id: String,
        snapshot: JobSnapshot,
        execution: Box<dyn JobExecution>,
        cancellation: CancellationToken,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
    RunJob {
        job_id: String,
        snapshot: JobSnapshot,
        cancellation: CancellationToken,
        response: mpsc::Sender<Result<JobExecutionResult, ExecutorError>>,
    },
    Shutdown,
}

trait RunnerOperation: Send {
    fn run(self: Box<Self>);
}

struct RunnerOperationTask<F, T> {
    operation: Option<Box<F>>,
    response: mpsc::Sender<Result<T, ExecutorError>>,
    admission: Option<OwnedRunnerAdmission>,
}

impl<F, T> RunnerOperation for RunnerOperationTask<F, T>
where
    F: FnOnce() -> Result<T, ExecutorError> + Send + 'static,
    T: Send + 'static,
{
    fn run(mut self: Box<Self>) {
        let operation = self.operation.take().expect("one-shot runner operation");
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
        // Drop the boxed FnOnce explicitly: closure environments are not
        // guaranteed to be destroyed before a caller's response is observed.
        drop(self.operation.take());
        let result = match outcome {
            Ok(result) => result,
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("runner operation panicked")
                    .to_owned();
                // Destroy the panic payload before publishing the error or
                // releasing admission; panic payloads may own lifecycle data.
                drop(payload);
                Err(ExecutorError::Execution {
                    error_code: "RUNNER_OPERATION_PANICKED".to_owned(),
                    error_message: message,
                    persistence_already_updated: false,
                })
            }
        };
        // The boxed closure and any panic payload are gone before admission is
        // released. Release before replying so observing the result also
        // observes completed admission cleanup.
        drop(self.admission.take());
        let _ = self.response.send(result);
    }
}

/// Owned counterpart to `RunnerReservation`: it can cross the runner's
/// `'static` queue and releases admission only after the inline operation and
/// all values captured by that operation have been dropped.
struct OwnedRunnerAdmission(Arc<AtomicUsize>);

impl Drop for OwnedRunnerAdmission {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct JobExecutorHandle {
    sender: SyncSender<Command>,
    runner_sender: SyncSender<RunnerCommand>,
    factory: Arc<dyn JobExecutionFactory>,
    /// Tracks how many callers currently hold a runner slot. Mirrors the
    /// capacity of `runner_sender` so admission can be decided *before* a
    /// durable attempt/state claim instead of after it.
    runner_in_flight: Arc<AtomicUsize>,
    cancellations: CancellationRegistry,
}

pub struct JobExecutor {
    handle: JobExecutorHandle,
    worker: Option<JoinHandle<()>>,
    runner: Option<JoinHandle<()>>,
}

impl JobExecutor {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_QUEUE_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_factory(capacity, Arc::new(UnconfiguredExecutionFactory))
    }

    pub fn with_capacity_and_factory(
        capacity: usize,
        factory: Arc<dyn JobExecutionFactory>,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
        let (runner_sender, runner_receiver) = mpsc::sync_channel(RUNNER_CAPACITY);
        let handle = JobExecutorHandle {
            sender,
            runner_sender: runner_sender.clone(),
            factory: factory.clone(),
            runner_in_flight: Arc::new(AtomicUsize::new(0)),
            cancellations: CancellationRegistry::default(),
        };
        let runner = thread::Builder::new()
            .name("xarchive-job-runner".to_owned())
            .spawn(move || run_runner(runner_receiver, factory))
            .expect("job runner must spawn");
        let worker_cancellations = handle.cancellations.clone();
        let worker = thread::Builder::new()
            .name("xarchive-job-executor".to_owned())
            .spawn(move || run_worker(receiver, runner_sender, worker_cancellations))
            .expect("job executor worker must spawn");
        Self {
            handle,
            worker: Some(worker),
            runner: Some(runner),
        }
    }

    pub fn handle(&self) -> JobExecutorHandle {
        self.handle.clone()
    }

    pub fn is_running(&self) -> bool {
        self.worker.is_some()
    }

    pub fn shutdown(mut self) -> Result<(), ExecutorError> {
        let result = self.handle.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = self.handle.runner_sender.send(RunnerCommand::Shutdown);
        if let Some(runner) = self.runner.take() {
            let _ = runner.join();
        }
        result
    }

    pub fn shutdown_in_place(&mut self) -> Result<(), ExecutorError> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let result = self.handle.shutdown();
        let _ = worker.join();
        let _ = self.handle.runner_sender.send(RunnerCommand::Shutdown);
        if let Some(runner) = self.runner.take() {
            let _ = runner.join();
        }
        result
    }
}

struct UnconfiguredExecutionFactory;

impl JobExecutionFactory for UnconfiguredExecutionFactory {
    fn create(
        &self,
        _job_id: &str,
        _snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        Err(ExecutorError::Execution {
            error_code: "EXECUTOR_FACTORY_UNAVAILABLE".to_owned(),
            error_message: "production execution factory is not configured".to_owned(),
            persistence_already_updated: false,
        })
    }
}

impl Drop for JobExecutor {
    fn drop(&mut self) {
        if self.worker.is_some() {
            let _ = self.handle.shutdown();
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
            let _ = self.handle.runner_sender.send(RunnerCommand::Shutdown);
            if let Some(runner) = self.runner.take() {
                let _ = runner.join();
            }
        }
    }
}

impl JobExecutorHandle {
    fn cancellation_for(&self, job_id: &str) -> Result<CancellationToken, ExecutorError> {
        self.cancellations.token(job_id)
    }

    /// Reserve a runner slot before a durable attempt/state claim.
    ///
    /// Returns `false` when the single-active runner is already saturated. A
    /// caller that cannot reserve must leave the Job queued and reschedulable
    /// rather than claiming an attempt and later persisting a failure.
    fn runner_try_admit(&self) -> bool {
        self.runner_in_flight
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |in_flight| {
                (in_flight < RUNNER_CAPACITY).then_some(in_flight + 1)
            })
            .is_ok()
    }

    /// Release a previously reserved runner slot.
    fn runner_release(&self) {
        self.runner_in_flight.fetch_sub(1, Ordering::AcqRel);
    }

    /// Convert an already-reserved slot into an owned token suitable for a
    /// single inline runner operation. The caller must not release the slot
    /// separately after this succeeds.
    fn owned_runner_admission(&self) -> OwnedRunnerAdmission {
        OwnedRunnerAdmission(self.runner_in_flight.clone())
    }

    /// Submit one `'static` operation to run directly on the existing runner
    /// thread, with its admission token owned by that task. The task cannot
    /// recursively submit another runner command because it receives no handle.
    fn run_operation<T: Send + 'static>(
        &self,
        admission: OwnedRunnerAdmission,
        operation: impl FnOnce() -> Result<T, ExecutorError> + Send + 'static,
    ) -> Result<T, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        let task: Box<dyn RunnerOperation> = Box::new(RunnerOperationTask {
            operation: Some(Box::new(operation)),
            response: response_sender,
            admission: Some(admission),
        });
        if let Err(error) = self
            .runner_sender
            .try_send(RunnerCommand::Operation { operation: task })
        {
            return match error {
                TrySendError::Full(RunnerCommand::Operation { operation }) => {
                    drop(operation);
                    Err(ExecutorError::QueueFull)
                }
                TrySendError::Disconnected(RunnerCommand::Operation { operation }) => {
                    drop(operation);
                    Err(ExecutorError::Closed)
                }
                _ => unreachable!("only an operation command was sent"),
            };
        }
        let result = response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?;
        result
    }

    pub fn submit(&self, request: ArchiveJobRequest) -> Result<SubmitResult, ExecutorError> {
        self.request(|response| Command::Submit { request, response })
    }

    pub fn start(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::Start {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn cancel(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::Cancel {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::Snapshot {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn shutdown(&self) -> Result<(), ExecutorError> {
        self.request(|response| Command::Shutdown { response })
    }

    pub fn recover(&self, jobs: Vec<JobSnapshot>) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.request(|response| Command::Recover { jobs, response })
    }

    pub fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        self.request(|response| Command::Events {
            job_id: job_id.to_owned(),
            response,
        })
    }

    pub fn sidecar_crashed(
        &self,
        job_id: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.request(|response| Command::SidecarCrashed {
            job_id: job_id.to_owned(),
            error_message: error_message.to_owned(),
            response,
        })
    }

    pub fn execute(
        &self,
        job_id: &str,
        execution: Box<dyn JobExecution>,
    ) -> Result<JobExecutionResult, ExecutorError> {
        self.request(|response| Command::Execute {
            job_id: job_id.to_owned(),
            execution,
            response,
        })
    }

    /// Dispatch an execution object directly to the single-active runner,
    /// carrying the caller's *owned* running snapshot.
    ///
    /// This is the dispatch path for the runner-owned session boundary, which
    /// owns its persistence context and therefore cannot depend on the
    /// worker's `jobs` map: a borrowed `JobPersistence` cannot cross the
    /// runner's `'static` queue, so the snapshot must be supplied by the
    /// caller rather than looked up by the worker. The caller holds a
    /// [`RunnerReservation`] for the whole call, so the single runner slot is
    /// guaranteed available and `try_send` cannot spuriously report
    /// [`ExecutorError::QueueFull`]. Mid-execution cancellation is a later
    /// boundary concern (B4/B5); the token is never signalled here because the
    /// boundary fences every terminal write on the durable attempt and reports
    /// a durable `Interrupted`/newer-attempt decision after the run.
    pub(crate) fn execute_owned(
        &self,
        job_id: &str,
        snapshot: JobSnapshot,
        execution: Box<dyn JobExecution>,
    ) -> Result<JobExecutionResult, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        self.runner_sender
            .try_send(RunnerCommand::Execute {
                job_id: job_id.to_owned(),
                snapshot,
                execution,
                cancellation: CancellationToken::new(),
                response: response_sender,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => ExecutorError::QueueFull,
                TrySendError::Disconnected(_) => ExecutorError::Closed,
            })?;
        response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?
    }

    pub fn run_job(
        &self,
        job_id: &str,
        snapshot: JobSnapshot,
    ) -> Result<JobExecutionResult, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        self.sender
            .try_send(Command::RunJob {
                job_id: job_id.to_owned(),
                snapshot,
                response: response_sender,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => ExecutorError::QueueFull,
                TrySendError::Disconnected(_) => ExecutorError::Closed,
            })?;
        response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?
    }

    fn request<T>(
        &self,
        build: impl FnOnce(mpsc::Sender<Result<T, ExecutorError>>) -> Command,
    ) -> Result<T, ExecutorError> {
        let (response_sender, response_receiver) = mpsc::channel();
        self.sender
            .try_send(build(response_sender))
            .map_err(|error| match error {
                TrySendError::Full(_) => ExecutorError::QueueFull,
                TrySendError::Disconnected(_) => ExecutorError::Closed,
            })?;
        response_receiver
            .recv()
            .map_err(|_| ExecutorError::ResponseClosed)?
    }
}

impl ArchiveApplicationService {
    pub fn new(executor: &JobExecutor) -> Self {
        Self {
            executor: executor.handle(),
            v2_enrollment: false,
        }
    }

    pub fn submit(&self, request: ArchiveJobRequest) -> Result<SubmitResult, ExecutorError> {
        self.executor.submit(request)
    }

    pub fn query(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.executor.snapshot(job_id)
    }

    pub fn cancel(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.executor.cancel(job_id)
    }

    pub fn recover(&self, jobs: Vec<JobSnapshot>) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.executor.recover(jobs)
    }

    pub fn shutdown(&self) -> Result<(), ExecutorError> {
        self.executor.shutdown()
    }

    pub fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        self.executor.events(job_id)
    }

    pub fn run_job(
        &self,
        job_id: &str,
        snapshot: JobSnapshot,
    ) -> Result<JobExecutionResult, ExecutorError> {
        self.executor.run_job(job_id, snapshot)
    }

    pub fn submit_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request: ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        persistence.configure_v2_enrollment(self.v2_enrollment);
        let result = persistence.create_or_reuse(&request)?;
        if result.created {
            if let Err(error) = self.executor.submit(request) {
                if matches!(error, ExecutorError::Closed) {
                    let failure_message = error.to_string();
                    persistence.fail(
                        &result.job.job_id,
                        "EXECUTOR_UNAVAILABLE",
                        &failure_message,
                    )?;
                    persistence.record_event(
                        &result.job.job_id,
                        ExecutorEvent::DownloadFailed {
                            error_code: "EXECUTOR_UNAVAILABLE".to_owned(),
                            error_message: failure_message,
                        },
                    )?;
                }
                // QueueFull is normal bounded backpressure. The durable Job stays
                // QUEUED so a later batch pass can retry the same submission;
                // only a closed executor is an unavailable/failed condition.
                return Err(error);
            }
            persistence.record_event(
                &result.job.job_id,
                ExecutorEvent::Submitted {
                    job_id: result.job.job_id.clone(),
                },
            )?;
        } else {
            persistence.record_event(
                &result.job.job_id,
                ExecutorEvent::Reused {
                    job_id: result.job.job_id.clone(),
                },
            )?;
        }
        Ok(result)
    }

    /// Submit a persisted Job and schedule its production execution without
    /// waiting for Sidecar, filesystem, or commit I/O to finish.
    ///
    /// The caller owns the short-lived persistence transaction. The scheduled
    /// execution opens its own SQLite context and re-loads the immutable
    /// execution spec by Job ID, so transport and Tauri command callers can
    /// return the initial Job snapshot immediately.
    pub fn submit_and_schedule_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request: ArchiveJobRequest,
        database_path: PathBuf,
    ) -> Result<SubmitResult, ExecutorError> {
        let result = self.submit_persisted(persistence, request)?;
        if result.created
            && let Err(error) = self.schedule_persisted(database_path, result.job.job_id.clone())
        {
            // Do not disguise an orchestration/storage failure as a download
            // failure. QueueFull remains non-failing bounded backpressure in
            // submit_persisted. A detached-thread setup error has no retry
            // owner yet, so persist its established explicit failure result.
            let failure_message = error.to_string();
            persistence.fail(
                &result.job.job_id,
                "EXECUTOR_SCHEDULE_FAILED",
                &failure_message,
            )?;
            persistence.record_event(
                &result.job.job_id,
                ExecutorEvent::DownloadFailed {
                    error_code: "EXECUTOR_SCHEDULE_FAILED".to_owned(),
                    error_message: failure_message,
                },
            )?;
            return Err(error);
        }
        Ok(result)
    }

    /// Schedule one persisted Job on a detached orchestration thread.
    ///
    /// The thread only coordinates persistence and the existing bounded
    /// executor/runner. Long-running archive I/O remains owned by the
    /// executor's execution port and never runs under RuntimeState's mutex.
    pub fn schedule_persisted(
        &self,
        database_path: PathBuf,
        job_id: String,
    ) -> Result<(), ExecutorError> {
        // Fail before detaching the thread when the target database cannot be
        // opened. This is an orchestration failure, distinct from bounded queue
        // rejection (handled before any execution claim).
        StorageJobPersistence::open(&database_path).map_err(|error| ExecutorError::Execution {
            error_code: "EXECUTOR_SCHEDULE_FAILED".to_owned(),
            error_message: format!("failed to open persistence for Job {job_id}: {error}"),
            persistence_already_updated: false,
        })?;

        let service = self.clone();
        thread::Builder::new()
            .name(format!("xarchive-job-schedule-{job_id}"))
            .spawn(move || {
                #[cfg(unix)]
                {
                    // Route ordinary submission through the owned SessionExecutionBoundary
                    // (A4): runner admission → scopes → durable claim → factory → execute
                    // → fenced finalize → session release → admission release.
                    let Ok(persistence) = StorageJobPersistence::open(&database_path) else {
                        return;
                    };
                    let Ok(coordinator) = ProductionCoordinator::open(&database_path) else {
                        return;
                    };
                    let archive_root = database_path
                        .parent()
                        .and_then(Path::parent)
                        .unwrap_or(Path::new("."))
                        .to_owned();
                    let boundary = SessionExecutionBoundary::new(
                        service,
                        persistence,
                        coordinator,
                        archive_root,
                    );
                    let _ = boundary.execute_on_runner(job_id);
                }
                #[cfg(not(unix))]
                {
                    let Ok(mut persistence) = StorageJobPersistence::open(&database_path) else {
                        return;
                    };
                    let _ = service.execute_persisted_from_factory(&mut persistence, &job_id);
                }
            })
            .map(|_| ())
            .map_err(|error| ExecutorError::Execution {
                error_code: "EXECUTOR_SCHEDULE_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            })
    }

    pub fn query_persisted<P: JobPersistence>(
        &self,
        persistence: &P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        persistence.snapshot(job_id)
    }

    pub fn execute_persisted<P, E>(
        &self,
        persistence: &mut P,
        execution: &mut E,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError>
    where
        P: JobPersistence,
        E: JobExecution,
    {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() {
            return Ok(current);
        }
        if current.state == JobState::Interrupted {
            persistence.persist_state(&JobSnapshot {
                state: JobState::Validating,
                ..current.clone()
            })?;
        }

        let mut running = current.clone();
        let required_states = match running.state {
            JobState::Queued => vec![
                JobState::Validating,
                JobState::MetadataReady,
                JobState::Downloading,
            ],
            JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
            JobState::MetadataReady => vec![JobState::Downloading],
            JobState::Downloading => Vec::new(),
            _ => Vec::new(),
        };
        for next in required_states {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            running.state = next;
            persistence.persist_state(&running)?;
            persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }

        let cancellation = CancellationToken::new();
        match execution.execute(&running, &cancellation) {
            Ok(result) => {
                let current = persistence.snapshot(job_id)?;
                if current.state == JobState::Interrupted || cancellation.is_cancelled() {
                    return Ok(current);
                }
                if result.state_already_updated && current.state == JobState::Complete {
                    return Ok(current);
                }
                let downloaded = JobSnapshot {
                    state: JobState::Downloaded,
                    ..running
                };
                if !result.state_already_updated || current.state != JobState::Downloaded {
                    persistence.persist_state(&downloaded)?;
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::StateChanged {
                            from: current.state,
                            to: JobState::Downloaded,
                        },
                    )?;
                }
                if !result.download_event_recorded {
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::DownloadCompleted {
                            files_copied: result.files_copied,
                        },
                    )?;
                }
                self.complete_persisted(persistence, job_id, &result.archive_directory)
            }
            Err(error) => {
                persistence.fail(job_id, &error.error_code, &error.error_message)?;
                persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: error.error_code,
                        error_message: error.error_message,
                    },
                )?;
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn execute_persisted_on_worker<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
        execution: Box<dyn JobExecution>,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() || current.state == JobState::Interrupted {
            return Ok(current);
        }
        // Runner admission must precede the durable attempt/state claim. When
        // the single-active runner is saturated we leave the Job queued and
        // reschedulable: no attempt is incremented and no FAILED is persisted.
        let _reservation = if self.executor.runner_try_admit() {
            self.executor.owned_runner_admission()
        } else {
            return Err(ExecutorError::QueueFull);
        };
        let mut running = current.clone();
        let mut transitions = match running.state {
            JobState::Queued => vec![
                JobState::Validating,
                JobState::MetadataReady,
                JobState::Downloading,
            ],
            JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
            JobState::MetadataReady => vec![JobState::Downloading],
            JobState::Downloading => Vec::new(),
            _ => Vec::new(),
        }
        .into_iter();
        // B2: atomically claim the first running transition together with the
        // attempt increment. A losing claimant returns the winner's snapshot
        // without side effects. A Job already in its running state has no
        // transition to claim, so a plain attempt begin provides the identity
        // used by later fenced writes.
        let attempt = if let Some(first) = transitions.next() {
            let expected_attempt = persistence.current_attempt(job_id)?;
            match persistence.claim_attempt(job_id, expected_attempt, current.state, first)? {
                Some(attempt) => {
                    running.state = first;
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::StateChanged {
                            from: current.state,
                            to: first,
                        },
                    )?;
                    attempt
                }
                None => return persistence.snapshot(job_id),
            }
        } else {
            persistence.begin_attempt(job_id)?
        };
        for next in transitions {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            if !persistence.persist_state_for_attempt(job_id, attempt, previous, next)? {
                // A newer attempt, a durable cancellation, or any other writer
                // already moved the row. This attempt lost the arbitration, so
                // it must stop writing states and events and report the state
                // the winning writer actually left behind.
                return persistence.snapshot(job_id);
            }
            running.state = next;
            persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }

        let run_result = self.executor.execute(job_id, execution);
        // Release the runner slot once the runner interaction is complete. The
        // post-run persistence/archive work below does not occupy the runner.
        drop(_reservation);
        match run_result {
            Ok(result) => {
                let current = persistence.snapshot(job_id)?;
                if current.state == JobState::Interrupted {
                    return Ok(current);
                }
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(current);
                }
                if result.state_already_updated && current.state == JobState::Complete {
                    return Ok(current);
                }
                // Fence the terminal download write on this attempt too: a
                // concurrent cancellation or newer attempt must not be
                // overwritten by `DOWNLOADING -> DOWNLOADED`.
                if current.state != JobState::Downloaded
                    && !persistence.persist_state_for_attempt(
                        job_id,
                        attempt,
                        JobState::Downloading,
                        JobState::Downloaded,
                    )?
                {
                    return persistence.snapshot(job_id);
                }
                persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: JobState::Downloading,
                        to: JobState::Downloaded,
                    },
                )?;
                if !result.download_event_recorded {
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::DownloadCompleted {
                            files_copied: result.files_copied,
                        },
                    )?;
                }
                self.complete_persisted(persistence, job_id, &result.archive_directory)
            }
            Err(error) => {
                if persistence.snapshot(job_id)?.state == JobState::Interrupted {
                    return persistence.snapshot(job_id);
                }
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return persistence.snapshot(job_id);
                }
                if let ExecutorError::Execution {
                    persistence_already_updated: true,
                    ..
                } = &error
                {
                    return persistence.snapshot(job_id);
                }
                let message = error.to_string();
                persistence.fail(job_id, "EXECUTOR_WORKER_FAILED", &message)?;
                persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTOR_WORKER_FAILED".to_owned(),
                        error_message: message,
                    },
                )?;
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn execute_persisted_from_factory<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() || current.state == JobState::Interrupted {
            return Ok(current);
        }
        // Runner admission must precede the durable attempt/state claim. When
        // the single-active runner is saturated we leave the Job queued and
        // reschedulable: no attempt is incremented and no FAILED is persisted.
        let _reservation = if self.executor.runner_try_admit() {
            self.executor.owned_runner_admission()
        } else {
            return Err(ExecutorError::QueueFull);
        };
        let mut running = current.clone();
        let mut transitions = match running.state {
            JobState::Queued => vec![
                JobState::Validating,
                JobState::MetadataReady,
                JobState::Downloading,
            ],
            JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
            JobState::MetadataReady => vec![JobState::Downloading],
            JobState::Downloading => Vec::new(),
            _ => Vec::new(),
        }
        .into_iter();
        // B2: atomically claim the first running transition together with the
        // attempt increment. A losing claimant returns the winner's snapshot
        // without side effects. A Job already in its running state has no
        // transition to claim, so a plain attempt begin provides the identity
        // used by later fenced writes.
        let attempt = if let Some(first) = transitions.next() {
            let expected_attempt = persistence.current_attempt(job_id)?;
            match persistence.claim_attempt(job_id, expected_attempt, current.state, first)? {
                Some(attempt) => {
                    running.state = first;
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::StateChanged {
                            from: current.state,
                            to: first,
                        },
                    )?;
                    attempt
                }
                None => return persistence.snapshot(job_id),
            }
        } else {
            persistence.begin_attempt(job_id)?
        };
        for next in transitions {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            if !persistence.persist_state_for_attempt(job_id, attempt, previous, next)? {
                // A newer attempt, a durable cancellation, or any other writer
                // already moved the row. This attempt lost the arbitration, so
                // it must stop writing states and events and report the state
                // the winning writer actually left behind.
                return persistence.snapshot(job_id);
            }
            running.state = next;
            persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }

        let run_result = self.executor.run_job(job_id, running.clone());
        // Release the runner slot once the runner interaction is complete. The
        // post-run persistence/archive work below does not occupy the runner,
        // so the next Job may start downloading while this one finalizes.
        drop(_reservation);
        match run_result {
            Ok(result) => {
                let current = persistence.snapshot(job_id)?;
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(current);
                }
                if result.state_already_updated && current.state == JobState::Complete {
                    return Ok(current);
                }
                // Fence the terminal download write on this attempt too: a
                // concurrent cancellation or newer attempt must not be
                // overwritten by `DOWNLOADING -> DOWNLOADED`.
                if current.state != JobState::Downloaded
                    && !persistence.persist_state_for_attempt(
                        job_id,
                        attempt,
                        JobState::Downloading,
                        JobState::Downloaded,
                    )?
                {
                    return persistence.snapshot(job_id);
                }
                persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: current.state,
                        to: JobState::Downloaded,
                    },
                )?;
                if !result.download_event_recorded {
                    persistence.record_event(
                        job_id,
                        ExecutorEvent::DownloadCompleted {
                            files_copied: result.files_copied,
                        },
                    )?;
                }
                self.complete_persisted(persistence, job_id, &result.archive_directory)
            }
            Err(error) => {
                if !persistence.attempt_is_current(job_id, attempt)? {
                    return persistence.snapshot(job_id);
                }
                if let ExecutorError::Execution {
                    persistence_already_updated: true,
                    ..
                } = &error
                {
                    return persistence.snapshot(job_id);
                }
                let message = error.to_string();
                persistence.fail(job_id, "EXECUTOR_WORKER_FAILED", &message)?;
                persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTOR_WORKER_FAILED".to_owned(),
                        error_message: message,
                    },
                )?;
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn recover_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
    ) -> Result<Vec<JobSnapshot>, ExecutorError> {
        let candidates = persistence.list_recovery_candidates();
        for candidate in &candidates {
            if !persistence.has_execution_spec(&candidate.job_id)? {
                persistence.fail(
                    &candidate.job_id,
                    "EXECUTION_SPEC_MISSING",
                    "archive execution spec is missing; recovery cannot reconstruct the request",
                )?;
                persistence.record_event(
                    &candidate.job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTION_SPEC_MISSING".to_owned(),
                        error_message: "archive execution spec is missing; recovery cannot reconstruct the request".to_owned(),
                    },
                )?;
            }
        }
        let previous_states = candidates
            .iter()
            .map(|snapshot| (snapshot.job_id.clone(), snapshot.state))
            .collect::<HashMap<_, _>>();
        let recovered = self.executor.recover(candidates)?;
        for snapshot in &recovered {
            persistence.persist_state(snapshot)?;
            persistence.record_event(
                &snapshot.job_id,
                ExecutorEvent::Recovered {
                    from: previous_states
                        .get(&snapshot.job_id)
                        .copied()
                        .unwrap_or(JobState::Interrupted),
                    to: snapshot.state,
                },
            )?;
        }
        Ok(recovered)
    }

    pub fn cancel_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state.is_terminal() || current.state == JobState::Interrupted {
            return Ok(current);
        }
        // A durable PREPARED/COMMITTED archive boundary means the archive
        // commit owns the job: refuse the cancellation without stopping the
        // writer, so the commit can finish.
        if persistence.archive_commit_started(job_id)? {
            return Ok(current);
        }
        // Stop the writer before the durable decision so nothing keeps writing
        // once cancellation is decided. The decision itself re-checks the
        // commit boundary inside its transaction, so a prepare that landed in
        // the meantime is never overwritten by a cancelled state.
        let stopped = self.executor.cancel(job_id)?;
        match persistence.persist_cancellation(job_id)? {
            CancelArbitration::Cancelled => {
                persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: current.state,
                        to: JobState::Cancelled,
                    },
                )?;
                Ok(stopped)
            }
            // The commit boundary won the race. The durable journal keeps the
            // job non-cancelled and replay completes the commit; no cancelled
            // state or event is written.
            CancelArbitration::CommitWins | CancelArbitration::AlreadyDecided => {
                persistence.snapshot(job_id)
            }
        }
    }

    pub fn interrupt_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
    ) -> Result<(), ExecutorError> {
        let active_jobs = persistence
            .list_recovery_candidates()
            .into_iter()
            .filter(|snapshot| snapshot.state.is_active())
            .collect::<Vec<_>>();
        for current in active_jobs {
            let interrupted = JobSnapshot {
                job_id: current.job_id.clone(),
                tweet_id: current.tweet_id.clone(),
                state: JobState::Interrupted,
            };
            persistence.persist_state(&interrupted)?;
            persistence.record_event(
                &current.job_id,
                ExecutorEvent::StateChanged {
                    from: current.state,
                    to: JobState::Interrupted,
                },
            )?;
            persistence.record_event(
                &current.job_id,
                ExecutorEvent::ShutdownInterrupted {
                    from: current.state,
                    to: JobState::Interrupted,
                },
            )?;
        }
        Ok(())
    }

    pub fn shutdown_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
    ) -> Result<(), ExecutorError> {
        self.interrupt_persisted(persistence)?;
        self.executor.shutdown()
    }

    pub fn complete_persisted<P: JobPersistence>(
        &self,
        persistence: &mut P,
        job_id: &str,
        archive_directory: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        let current = persistence.snapshot(job_id)?;
        if current.state == JobState::Complete {
            return Ok(current);
        }
        current
            .state
            .transition_to(JobState::Complete)
            .map_err(|_| ExecutorError::InvalidTransition {
                from: current.state,
                to: JobState::Complete,
            })?;
        let completed = JobSnapshot {
            job_id: current.job_id,
            tweet_id: current.tweet_id,
            state: JobState::Complete,
        };
        persistence.persist_state(&completed)?;
        persistence.record_event(
            job_id,
            ExecutorEvent::Completed {
                archive_directory: archive_directory.to_owned(),
            },
        )?;
        Ok(completed)
    }

    pub fn apply_commit_recovery<P: JobPersistence>(
        &self,
        persistence: &mut P,
        facts: &CommitRecoveryFacts,
        archive_directory: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        match facts.decide(archive_directory) {
            CommitRecoveryDecision::ResumeStaging
            | CommitRecoveryDecision::Skip
            | CommitRecoveryDecision::NotApplicable => persistence.snapshot(&facts.job_id),
            CommitRecoveryDecision::Complete { archive_directory } => {
                self.complete_persisted(persistence, &facts.job_id, &archive_directory)
            }
            CommitRecoveryDecision::MarkFailed {
                error_code,
                error_message,
            } => {
                let failed = persistence.fail(&facts.job_id, &error_code, &error_message)?;
                persistence.record_event(
                    &facts.job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code,
                        error_message,
                    },
                )?;
                Ok(failed)
            }
        }
    }

    pub fn recover_commit_persisted<P, F>(
        &self,
        persistence: &mut P,
        facts_provider: &F,
        job_id: &str,
        archive_directory: &str,
    ) -> Result<JobSnapshot, ExecutorError>
    where
        P: JobPersistence,
        F: CommitRecoveryFactsProvider,
    {
        let snapshot = persistence.snapshot(job_id)?;
        let facts = facts_provider.facts_for(job_id)?;
        if facts.state != snapshot.state || facts.job_id != snapshot.job_id {
            return Err(ExecutorError::Persistence(
                "commit recovery facts do not match persisted Job snapshot".to_owned(),
            ));
        }
        self.apply_commit_recovery(persistence, &facts, archive_directory)
    }

    pub fn recover_commits_persisted<P, F>(
        &self,
        persistence: &mut P,
        facts_provider: &F,
        archive_directory: &str,
    ) -> Vec<CommitRecoveryBatchResult>
    where
        P: JobPersistence,
        F: CommitRecoveryFactsProvider,
    {
        let mut candidates = persistence.list_recovery_candidates();
        candidates.sort_by(|left, right| left.job_id.cmp(&right.job_id));
        candidates
            .into_iter()
            .map(|candidate| {
                let job_id = candidate.job_id.clone();
                let result = self.recover_commit_persisted(
                    persistence,
                    facts_provider,
                    &job_id,
                    archive_directory,
                );
                CommitRecoveryBatchResult { job_id, result }
            })
            .collect()
    }
}

/// Outcome of one [`SessionExecutionBoundary::execute_session`] run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionRunOutcome {
    /// The attempt reached a durable decision while its session was alive.
    /// Besides terminal states this includes snapshots that lost a later
    /// arbitration (a newer attempt or a durable cancellation), which report
    /// the winning writer's state instead of overwriting it.
    Finished(JobSnapshot),
    /// The Job was not executed because a live peer owns the Job or
    /// destination scope, or won the durable claim race. Nothing is
    /// persisted as a failure; the Job stays reschedulable.
    Reschedulable {
        job_id: String,
        reason: SessionRescheduleReason,
    },
}

/// Why a session run was reschedulable instead of executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionRescheduleReason {
    /// Another live owner holds the Job or destination scope.
    Contended,
    /// Another writer won the durable claim; this caller has no attempt.
    ClaimLost,
}

/// Runner-owned execution boundary: an owned persistence context combined
/// with an owned coordinator so one attempt's `ExecutionSession` can span
/// the whole lifecycle. (`CROSS_PLATFORM_CHANGE_REQUIRED` shared contract.)
///
/// This is the owned persistence/coordinator/session boundary the C1-A
/// checkpoint identified as the missing integration surface:
///
/// - The persistence context is *owned* by the boundary, so it can be moved
///   into the scheduler's `'static` thread together with the coordinator,
///   instead of being borrowed by the scheduling caller (a borrowed
///   `JobPersistence` cannot cross the runner's `'static` queue).
/// - The coordinator is owned and generic over the [`Coordinator`] port, so
///   the same boundary compiles on Windows once the Windows Owner implements
///   the NTFS adapter; no platform `cfg` removes the session.
/// - [`SessionExecutionBoundary::execute_on_runner`] reserves runner capacity
///   and moves the stable boundary owners into one inline operation; that
///   operation invokes [`SessionExecutionBoundary::execute_session`] directly
///   on the runner thread.
/// - `execute_session` acquires the Job *and* destination scopes,
///   runs the durable attempt claim under that guard, creates the execution
///   object only *after* the claim, and holds the resulting session — and the
///   runner reservation — through the runner interaction, the fenced
///   terminal writes and archive finalization. The session is dropped last,
///   so the scopes are released only after finalization completes.
/// - Contention and a lost claim map to [`SessionRunOutcome::Reschedulable`]
///   and never persist a `FAILED`; runner saturation returns
///   [`ExecutorError::QueueFull`] before any attempt is claimed.
///
/// Unix persisted scheduling uses this boundary. Provisioned Linux execution
/// retains attempt-owned writers through drain, no-replace publication and
/// fenced finalization. Other platform gates remain independently enforced.
pub(crate) struct SessionExecutionBoundary<P, C> {
    service: ArchiveApplicationService,
    persistence: P,
    coordinator: C,
    archive_root: PathBuf,
}

/// The running-state advance table shared by the persisted execution paths.
fn running_transitions(state: JobState) -> std::vec::IntoIter<JobState> {
    match state {
        JobState::Queued | JobState::Interrupted => vec![
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
        ],
        JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
        JobState::MetadataReady => vec![JobState::Downloading],
        JobState::Downloading => Vec::new(),
        _ => Vec::new(),
    }
    .into_iter()
}

impl<P: JobPersistence, C: Coordinator> SessionExecutionBoundary<P, C> {
    /// Build a boundary that owns its persistence context, its coordinator,
    /// and the application service whose runner executes the work.
    pub(crate) fn new(
        service: ArchiveApplicationService,
        persistence: P,
        coordinator: C,
        archive_root: PathBuf,
    ) -> Self {
        Self {
            service,
            persistence,
            coordinator,
            archive_root,
        }
    }

    /// Transfer the stable boundary owners into one inline runner operation.
    /// The session is borrowed only after the closure reaches the runner, so no
    /// borrowed guard or coordinator reference crosses the `'static` queue.
    pub(crate) fn execute_on_runner(
        self,
        job_id: String,
    ) -> Result<SessionRunOutcome, ExecutorError>
    where
        P: Send + 'static,
        C: Send + 'static,
    {
        if !self.service.executor.runner_try_admit() {
            return Err(ExecutorError::QueueFull);
        }
        let admission = self.service.executor.owned_runner_admission();
        let runner = self.service.executor.clone();
        runner.run_operation(admission, move || {
            let mut boundary = self;
            boundary.execute_session(&job_id)
        })
    }

    /// The final archive directory this boundary binds a session to for
    /// `tweet_id`: `<archive_root>/Tweets/<tweet_id>`.
    fn destination_for(&self, tweet_id: &str) -> PathBuf {
        self.archive_root.join("Tweets").join(tweet_id)
    }

    /// Run one Job under a runner-owned session.
    ///
    /// Ordering contract, enforced by construction: runner admission in
    /// `execute_on_runner` → Job/destination scope acquisition → durable claim under the guard →
    /// factory creation → runner execution → attempt-fenced terminal writes
    /// and finalization → session release. Contention or a lost claim
    /// release both scopes without persisting anything.
    fn execute_session(&mut self, job_id: &str) -> Result<SessionRunOutcome, ExecutorError> {
        let current = self.persistence.snapshot(job_id)?;
        if current.state.is_terminal()
            || (current.state == JobState::Interrupted && !self.service.v2_enrollment)
        {
            return Ok(SessionRunOutcome::Finished(current));
        }
        let destination = self.destination_for(&current.tweet_id);
        let reservation =
            match ExecutionReservation::acquire(&self.coordinator, job_id, &destination) {
                Ok(reservation) => reservation,
                Err(SessionError::Contended) => {
                    return Ok(SessionRunOutcome::Reschedulable {
                        job_id: job_id.to_owned(),
                        reason: SessionRescheduleReason::Contended,
                    });
                }
                Err(error) => return Err(session_establishment_error(job_id, error)),
            };
        // Compute the first running transition before claiming; the claim
        // transaction itself re-validates state and attempt atomically, so
        // this value is only the transition this caller intends to claim.
        let mut transitions = running_transitions(current.state);
        let first = transitions.next();
        // The durable claim runs *under the held guard*; the callback form
        // makes the lock-then-claim order impossible to invert.
        let session = match reservation.claim(|| {
            let expected_attempt = match self.persistence.current_attempt(job_id) {
                Ok(attempt) => attempt,
                Err(_) => return ClaimOutcome::Failed,
            };
            let identity = match first {
                Some(to) => {
                    match self.persistence.claim_attempt(
                        job_id,
                        expected_attempt,
                        current.state,
                        to,
                    ) {
                        Ok(Some(attempt)) => attempt,
                        Ok(None) => return ClaimOutcome::Lost,
                        Err(_) => return ClaimOutcome::Failed,
                    }
                }
                // A Job already in its running state has no transition to
                // claim; a plain attempt begin provides the identity used by
                // later fenced writes.
                None => match self.persistence.begin_attempt(job_id) {
                    Ok(attempt) => attempt,
                    Err(_) => return ClaimOutcome::Failed,
                },
            };
            match AttemptIdentity::new(job_id, identity) {
                Ok(identity) => ClaimOutcome::Claimed(identity),
                Err(_) => ClaimOutcome::Failed,
            }
        }) {
            Ok(session) => session,
            Err(error @ (SessionError::Contended | SessionError::ClaimLost)) => {
                let reason = match error {
                    SessionError::Contended => SessionRescheduleReason::Contended,
                    _ => SessionRescheduleReason::ClaimLost,
                };
                return Ok(SessionRunOutcome::Reschedulable {
                    job_id: job_id.to_owned(),
                    reason,
                });
            }
            Err(error) => return Err(session_establishment_error(job_id, error)),
        };
        let attempt = session.attempt().attempt();
        let mut running = current.clone();
        if let Some(to) = first {
            // The claim transaction already wrote the state; record its
            // event, then apply the remaining advances attempt-fenced.
            self.persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: current.state,
                    to,
                },
            )?;
            running.state = to;
        }
        for next in transitions {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            if !self
                .persistence
                .persist_state_for_attempt(job_id, attempt, previous, next)?
            {
                // A newer attempt, a durable cancellation, or any other
                // writer already moved the row; this attempt must stop
                // writing and report the winning state.
                return Ok(SessionRunOutcome::Finished(
                    self.persistence.snapshot(job_id)?,
                ));
            }
            running.state = next;
            self.persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }
        // The execution factory and runner remain in the object-safe production
        // seam; the session stays live through publication and finalization.
        // We are already executing on the single runner thread. Invoke the
        // owned execution directly; dispatching via execute_owned here would
        // enqueue behind this operation and self-deadlock. `execution` is
        // intentionally kept alive past the completion decision below so the
        // attempt-owned owners (Sidecar, aria2, Rust writers) are not dropped
        // before the session has finished finalizing.
        let mut execution: Option<Box<dyn JobExecution>> = None;
        let run_result: Result<JobExecutionResult, ExecutorError> = match self
            .service
            .executor
            .factory
            .create_for_attempt(session.attempt(), &running)
        {
            Ok(exec) => {
                execution = Some(exec);
                execution
                    .as_mut()
                    .unwrap()
                    .execute(&running, &self.service.executor.cancellation_for(job_id)?)
                    .map_err(|error| ExecutorError::Execution {
                        error_code: error.error_code,
                        error_message: error.error_message,
                        persistence_already_updated: error.persistence_already_updated,
                    })
            }
            Err(error) => Err(error),
        };
        let outcome = match run_result {
            Ok(result) => {
                let current = self.persistence.snapshot(job_id)?;
                if current.state == JobState::Interrupted {
                    return Ok(SessionRunOutcome::Finished(current));
                }
                if !self.persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(SessionRunOutcome::Finished(current));
                }
                // Fence the terminal download write on this attempt too.
                if current.state != JobState::Downloaded
                    && !self.persistence.persist_state_for_attempt(
                        job_id,
                        attempt,
                        JobState::Downloading,
                        JobState::Downloaded,
                    )?
                {
                    return Ok(SessionRunOutcome::Finished(
                        self.persistence.snapshot(job_id)?,
                    ));
                }
                self.persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: JobState::Downloading,
                        to: JobState::Downloaded,
                    },
                )?;
                // Atomically record DownloadCompleted + Complete + Completed
                // in one transaction, fenced by attempt. Idempotent: if the
                // job is already Complete, no writes occur; if the attempt no
                // longer owns the job, the transition is refused.
                let completed = self.persistence.complete_terminal_for_attempt(
                    job_id,
                    attempt,
                    &result.archive_directory,
                    result.files_copied,
                )?;
                SessionRunOutcome::Finished(completed)
            }
            Err(error) => {
                if !self.persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(SessionRunOutcome::Finished(
                        self.persistence.snapshot(job_id)?,
                    ));
                }
                if let ExecutorError::Execution {
                    persistence_already_updated: true,
                    ..
                } = &error
                {
                    return Ok(SessionRunOutcome::Finished(
                        self.persistence.snapshot(job_id)?,
                    ));
                }
                // Publication can fail after PREPARED (rename, directory move,
                // or DB finalization). The journal owns that attempt: retain
                // its candidate state for startup replay instead of hiding it
                // behind FAILED, which is not a startup recovery candidate.
                if self.persistence.archive_commit_started(job_id)? {
                    return Err(error);
                }
                let message = error.to_string();
                self.persistence
                    .fail(job_id, "EXECUTOR_WORKER_FAILED", &message)?;
                self.persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTOR_WORKER_FAILED".to_owned(),
                        error_message: message,
                    },
                )?;
                SessionRunOutcome::Finished(self.persistence.snapshot(job_id)?)
            }
        };
        // `execution` is deliberately held past the completion decision above
        // so the attempt-owned owners (Sidecar, aria2, Rust writers) are not
        // dropped before the session has finished finalizing.
        let _ = execution;
        // The borrowed session releases its guard before an enclosing owned
        // runner operation drops the boundary's coordinator and admission.
        Ok(outcome)
    }
}

/// Map a session-establishment failure that is neither contention nor a lost
/// claim onto an executor error. These are refusal paths: no `FAILED` is
/// persisted, because the Job itself did not fail to execute — it never
/// obtained a legitimate attempt.
fn session_establishment_error(job_id: &str, error: SessionError) -> ExecutorError {
    ExecutorError::Execution {
        error_code: "EXECUTION_SESSION_FAILED".to_owned(),
        error_message: format!("could not establish execution session for {job_id}: {error}"),
        persistence_already_updated: false,
    }
}

fn run_runner(receiver: Receiver<RunnerCommand>, factory: Arc<dyn JobExecutionFactory>) {
    while let Ok(command) = receiver.recv() {
        match command {
            RunnerCommand::Operation { operation } => operation.run(),
            RunnerCommand::Execute {
                snapshot,
                mut execution,
                cancellation,
                response,
                ..
            } => {
                let result = execution
                    .execute(&snapshot, &cancellation)
                    .map_err(|error| ExecutorError::Execution {
                        error_code: error.error_code,
                        error_message: error.error_message,
                        persistence_already_updated: error.persistence_already_updated,
                    });
                // The execution owns its process supervisors and session-bound
                // resources. Tear them down before the caller can proceed to
                // finalization or release its attempt authorization.
                drop(execution);
                let _ = response.send(result);
            }
            RunnerCommand::RunJob {
                job_id,
                snapshot,
                cancellation,
                response,
            } => {
                // The scheduling caller (`execute_persisted_from_factory`)
                // already reserved the single runner slot and claimed the
                // attempt before dispatching here. This arm runs *on* the
                // runner thread, so it must invoke the factory and execution
                // directly: re-entering `run_operation` would enqueue to this
                // same runner and self-deadlock, and acquiring a second
                // admission would double-release the caller's reservation.
                let result = factory
                    .create(&job_id, &snapshot)
                    .and_then(|mut execution| {
                        let result = execution
                            .execute(&snapshot, &cancellation)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: error.error_code,
                                error_message: error.error_message,
                                persistence_already_updated: error.persistence_already_updated,
                            });
                        // The execution owns its process supervisors and
                        // session-bound resources; tear them down before the
                        // caller can proceed to finalization or release its
                        // attempt authorization.
                        drop(execution);
                        result
                    });
                let _ = response.send(result);
            }
            RunnerCommand::Shutdown => break,
        }
    }
}

fn run_worker(
    receiver: Receiver<Command>,
    runner_sender: SyncSender<RunnerCommand>,
    cancellations: CancellationRegistry,
) {
    let mut jobs = HashMap::<String, JobRecord>::new();
    while let Ok(command) = receiver.recv() {
        let shutdown = matches!(command, Command::Shutdown { .. });
        handle_command(command, &mut jobs, &runner_sender, &cancellations);
        if shutdown {
            break;
        }
    }
}

fn handle_command(
    command: Command,
    jobs: &mut HashMap<String, JobRecord>,
    runner_sender: &SyncSender<RunnerCommand>,
    cancellations: &CancellationRegistry,
) {
    match command {
        Command::Submit { request, response } => {
            let existing = jobs.values().find(|record| {
                record.snapshot.tweet_id == request.tweet_id && record.snapshot.state.is_active()
            });
            if let Some(existing) = existing {
                let job = existing.snapshot.clone();
                let job_id = job.job_id.clone();
                if let Some(record) = jobs.get_mut(&job_id) {
                    record.events.push(ExecutorEvent::Reused {
                        job_id: job_id.clone(),
                    });
                }
                let _ = response.send(Ok(SubmitResult {
                    job,
                    created: false,
                }));
                return;
            }
            let snapshot = JobSnapshot {
                job_id: request.job_id.clone(),
                tweet_id: request.tweet_id,
                state: JobState::Queued,
            };
            let cancellation = CancellationToken::new();
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            if let Ok(mut tokens) = cancellations.0.lock() {
                tokens.insert(snapshot.job_id.clone(), cancellation.clone());
            }
            jobs.insert(
                snapshot.job_id.clone(),
                JobRecord {
                    snapshot: snapshot.clone(),
                    events: vec![ExecutorEvent::Submitted {
                        job_id: snapshot.job_id.clone(),
                    }],
                    cancellation,
                },
            );
            let _ = response.send(Ok(SubmitResult {
                job: snapshot,
                created: true,
            }));
        }
        Command::Start { job_id, response } => {
            let result = transition_job(jobs, &job_id, JobState::Validating);
            let _ = response.send(result);
        }
        Command::Cancel { job_id, response } => {
            if let Some(record) = jobs.get(&job_id) {
                record.cancellation.cancel();
            }
            if let Ok(tokens) = cancellations.0.lock()
                && let Some(token) = tokens.get(&job_id)
            {
                token.cancel();
            }
            if let Ok(tokens) = cancellations.0.lock()
                && let Some(token) = tokens.get(&job_id)
            {
                token.cancel();
            }
            let result = cancel_job(jobs, &job_id);
            let _ = response.send(result);
        }
        Command::Snapshot { job_id, response } => {
            let result = jobs
                .get(&job_id)
                .map(|record| record.snapshot.clone())
                .ok_or(ExecutorError::UnknownJob(job_id));
            let _ = response.send(result);
        }
        Command::Events { job_id, response } => {
            let result = jobs
                .get(&job_id)
                .map(|record| record.events.clone())
                .ok_or(ExecutorError::UnknownJob(job_id));
            let _ = response.send(result);
        }
        Command::Shutdown { response } => {
            for record in jobs.values_mut() {
                if record.snapshot.state.is_active() {
                    record.cancellation.cancel();
                    let from = record.snapshot.state;
                    record.snapshot.state = JobState::Interrupted;
                    record.events.push(ExecutorEvent::StateChanged {
                        from,
                        to: JobState::Interrupted,
                    });
                    record.events.push(ExecutorEvent::ShutdownInterrupted {
                        from,
                        to: JobState::Interrupted,
                    });
                }
            }
            let _ = response.send(Ok(()));
        }
        Command::Recover {
            jobs: candidates,
            response,
        } => {
            let mut recovered = Vec::new();
            for candidate in candidates {
                if candidate.state.is_terminal() || jobs.contains_key(&candidate.job_id) {
                    continue;
                }
                let from_state = candidate.state;
                let mut snapshot = candidate;
                if snapshot.state == JobState::Queued || snapshot.state == JobState::Interrupted {
                    snapshot.state = JobState::Validating;
                }
                jobs.insert(
                    snapshot.job_id.clone(),
                    JobRecord {
                        snapshot: snapshot.clone(),
                        events: vec![ExecutorEvent::Recovered {
                            from: from_state,
                            to: snapshot.state,
                        }],
                        cancellation: CancellationToken::new(),
                    },
                );
                recovered.push(snapshot);
            }
            let _ = response.send(Ok(recovered));
        }
        Command::SidecarCrashed {
            job_id,
            error_message,
            response,
        } => {
            let result = sidecar_crash(jobs, &job_id, error_message);
            let _ = response.send(result);
        }
        Command::Execute {
            job_id,
            execution,
            response,
        } => {
            let snapshot = jobs
                .get(&job_id)
                .map(|record| record.snapshot.clone())
                .ok_or_else(|| ExecutorError::UnknownJob(job_id.clone()));
            match snapshot {
                Ok(snapshot) => {
                    let cancellation = jobs
                        .get(&job_id)
                        .map(|record| record.cancellation.clone())
                        .unwrap_or_default();
                    let result = runner_sender.try_send(RunnerCommand::Execute {
                        job_id,
                        snapshot,
                        execution,
                        cancellation,
                        response,
                    });
                    if let Err(error) = result {
                        let (response, executor_error) = match error {
                            TrySendError::Full(RunnerCommand::Execute { response, .. }) => {
                                (response, ExecutorError::QueueFull)
                            }
                            TrySendError::Disconnected(RunnerCommand::Execute {
                                response, ..
                            }) => (response, ExecutorError::Closed),
                            TrySendError::Full(RunnerCommand::Operation { .. })
                            | TrySendError::Disconnected(RunnerCommand::Operation { .. }) => {
                                unreachable!("inline operations bypass the executor worker")
                            }
                            TrySendError::Full(RunnerCommand::RunJob { response, .. }) => {
                                (response, ExecutorError::QueueFull)
                            }
                            TrySendError::Disconnected(RunnerCommand::RunJob {
                                response, ..
                            }) => (response, ExecutorError::Closed),
                            TrySendError::Full(RunnerCommand::Shutdown)
                            | TrySendError::Disconnected(RunnerCommand::Shutdown) => {
                                unreachable!("shutdown is only sent by the executor owner")
                            }
                        };
                        let _ = response.send(Err(executor_error));
                    }
                }
                Err(error) => {
                    let _ = response.send(Err(error));
                }
            }
        }
        Command::RunJob {
            job_id,
            snapshot,
            response,
        } => {
            let cancellation = jobs
                .get(&job_id)
                .map(|record| record.cancellation.clone())
                .unwrap_or_default();
            let result = runner_sender.try_send(RunnerCommand::RunJob {
                job_id,
                snapshot,
                cancellation,
                response,
            });
            if let Err(error) = result {
                let (response, executor_error) = match error {
                    TrySendError::Full(RunnerCommand::RunJob { response, .. }) => {
                        (response, ExecutorError::QueueFull)
                    }
                    TrySendError::Disconnected(RunnerCommand::RunJob { response, .. }) => {
                        (response, ExecutorError::Closed)
                    }
                    TrySendError::Full(RunnerCommand::Execute { response, .. })
                    | TrySendError::Disconnected(RunnerCommand::Execute { response, .. }) => {
                        (response, ExecutorError::Closed)
                    }
                    TrySendError::Full(RunnerCommand::Operation { .. })
                    | TrySendError::Disconnected(RunnerCommand::Operation { .. }) => {
                        unreachable!("inline operations bypass the executor worker")
                    }
                    TrySendError::Full(RunnerCommand::Shutdown)
                    | TrySendError::Disconnected(RunnerCommand::Shutdown) => {
                        unreachable!("shutdown is only sent by the executor owner")
                    }
                };
                let _ = response.send(Err(executor_error));
            }
        }
    }
}

fn transition_job(
    jobs: &mut HashMap<String, JobRecord>,
    job_id: &str,
    next: JobState,
) -> Result<JobSnapshot, ExecutorError> {
    let record = jobs
        .get_mut(job_id)
        .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))?;
    let current = record.snapshot.state;
    current
        .transition_to(next)
        .map_err(|_| ExecutorError::InvalidTransition {
            from: current,
            to: next,
        })?;
    record.snapshot.state = next;
    record.events.push(ExecutorEvent::StateChanged {
        from: current,
        to: next,
    });
    Ok(record.snapshot.clone())
}

fn cancel_job(
    jobs: &mut HashMap<String, JobRecord>,
    job_id: &str,
) -> Result<JobSnapshot, ExecutorError> {
    let record = jobs
        .get_mut(job_id)
        .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))?;
    if record.snapshot.state.is_terminal() {
        return Ok(record.snapshot.clone());
    }
    let current = record.snapshot.state;
    if current == JobState::Cancelled || current == JobState::Interrupted {
        return Ok(record.snapshot.clone());
    }
    current
        .transition_to(JobState::Cancelled)
        .map_err(|_| ExecutorError::InvalidTransition {
            from: current,
            to: JobState::Cancelled,
        })?;
    record.snapshot.state = JobState::Cancelled;
    record.events.push(ExecutorEvent::StateChanged {
        from: current,
        to: JobState::Cancelled,
    });
    Ok(record.snapshot.clone())
}

fn sidecar_crash(
    jobs: &mut HashMap<String, JobRecord>,
    job_id: &str,
    error_message: String,
) -> Result<JobSnapshot, ExecutorError> {
    let record = jobs
        .get_mut(job_id)
        .ok_or_else(|| ExecutorError::UnknownJob(job_id.to_owned()))?;
    let current = record.snapshot.state;
    let next = if current.can_transition_to(JobState::Failed) {
        JobState::Failed
    } else if current.can_transition_to(JobState::Interrupted) {
        JobState::Interrupted
    } else {
        return Err(ExecutorError::InvalidTransition {
            from: current,
            to: JobState::Failed,
        });
    };
    record.snapshot.state = next;
    record.events.push(ExecutorEvent::StateChanged {
        from: current,
        to: next,
    });
    record
        .events
        .push(ExecutorEvent::SidecarCrashed { error_message });
    Ok(record.snapshot.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::mpsc::Sender;
    use std::thread;

    fn request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
        ArchiveJobRequest {
            job_id: job_id.to_owned(),
            tweet_id: tweet_id.to_owned(),
            request_id: format!("test-request-{job_id}"),
            request_json: "{}".to_owned(),
            schema_version: 1,
        }
    }

    fn archive_request(tweet_id: &str) -> crate::ArchiveTweetRequest {
        crate::ArchiveTweetRequest {
            tweet: xarchive_protocol::BrowserTweet {
                tweet_id: tweet_id.to_owned(),
                url: format!("https://x.com/alice/status/{tweet_id}"),
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
                text: Some("archive me".to_owned()),
                created_at: Some("2026-09-12T00:00:00Z".to_owned()),
                tweet_type: "post".to_owned(),
                reply_to: None,
                quoted_tweet: None,
            },
            browser: None,
            profile: None,
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires explicitly delegated disposable cgroup"]
    fn production_factory_contained_handshake_and_drop_remove_writer_group() {
        let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
        let relative = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .unwrap();
        let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
        let database_path = temporary_database_path("contained-factory");
        std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
        let root = database_path.parent().unwrap().parent().unwrap();
        let marker = root.join("membership");
        let database = Database::open(&database_path).unwrap();
        let tweet = database
            .insert_tweet(
                "12090",
                "https://x.com/alice/status/12090",
                "post",
                "",
                "now",
            )
            .unwrap();
        let json = serde_json::to_string(&ExecutionSpecV2 {
            request: archive_request("12090"),
            output_settings: OutputSettings::default(),
        })
        .unwrap();
        database
            .create_archive_job_with_optional_recovery_contract(
                "contained-job",
                tweet,
                2,
                "contained-request",
                &json,
                Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .unwrap();
        let script = "import pathlib,sys,json; pathlib.Path(sys.argv[1]).write_text(pathlib.Path('/proc/self/cgroup').read_text()); c=json.loads(sys.stdin.readline()); print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True); sys.stdin.read()";
        let factory = ProductionExecutionFactory::new(ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root: root.join("archive"),
            staging_root: root.join("staging"),
            database_path: database_path.clone(),
            sidecar_program: Some("/usr/bin/python3".into()),
            gallery_dl_program: None,
            sidecar_args: vec![
                "-u".into(),
                "-c".into(),
                script.into(),
                marker.to_string_lossy().into_owned(),
            ],
            aria2_program: None,
            use_aria2: false,
            network: ExecutorNetworkConfig {
                proxy_mode: ProxyMode::Direct,
                ..Default::default()
            },
        })
        .with_linux_containment(LinuxExecutionContainment {
            delegated_parent: parent.clone(),
            python: PathBuf::from("/usr/bin/python3"),
            launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        });
        let execution = factory
            .create_for_attempt(
                &AttemptIdentity::new("contained-job", 1).unwrap(),
                &JobSnapshot {
                    job_id: "contained-job".into(),
                    tweet_id: "12090".into(),
                    state: JobState::Downloading,
                },
            )
            .unwrap();
        let group_membership = std::fs::read_to_string(&marker).unwrap();
        let group_relative = group_membership.trim().strip_prefix("0::").unwrap();
        assert!(group_relative.starts_with(&format!("{relative}/tw2tg-att-")));
        let group = Path::new("/sys/fs/cgroup").join(group_relative.trim_start_matches('/'));
        assert!(group.exists());
        drop(execution);
        assert!(!group.exists());
        drop(database);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn execution_spec_v1_decodes_historical_request_json_without_rewriting() {
        let request = archive_request("12001");
        let json = serde_json::to_string(&request).expect("serialize historical request");

        let decoded = decode_execution_spec(1, &json).expect("decode v1");

        assert_eq!(decoded.request.tweet.tweet_id, request.tweet.tweet_id);
        assert_eq!(decoded.request.browser, request.browser);
        assert_eq!(decoded.request.profile, request.profile);
        assert_eq!(decoded.output_settings, OutputSettings::default());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires explicitly delegated disposable cgroup"]
    fn production_real_tools_publish_local_media() {
        use std::time::Duration;
        let aria2 = std::env::var("TW2TG_TEST_ARIA2").expect("real aria2");
        let gallery = std::env::var("TW2TG_TEST_GALLERY").expect("real gallery-dl");
        let media_url = std::env::var("TW2TG_TEST_MEDIA_URL").expect("local fixture URL");
        assert!(media_url.starts_with("http://127.0.0.1:"));
        let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
        let relative = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .unwrap();
        let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
        let mut engine_metadata = Vec::new();
        for use_aria2 in [false, true] {
            for d1 in [false, true] {
                let database_path = temporary_database_path(if use_aria2 {
                    "real-publish-aria2"
                } else {
                    "real-publish-gallery"
                });
                std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
                let root = database_path.parent().unwrap().parent().unwrap();
                // Fixture controls extraction and redirects only the network input
                // to loopback; real DownloadRunner and gallery-dl perform download,
                // info-json parsing and file scan. No fake tool writes media bytes.
                let script = format!(
                    r#"import sys,json
sys.path.insert(0,{source:?})
from dataclasses import replace
from xarchive_downloader import worker_v2 as w
from xarchive_downloader.extraction import DownloadRunner,ExtractionConfig
real=DownloadRunner.run
def local(self,url,staging_dir,**kwargs):
 self.config=replace(self.config,executable_args=('-o','extractor.directlink.keywords='+json.dumps({{'tweet_id':12091,'num':1,'tweet_url':'https://x.com/fixture/status/12091','text':'real-tool text','media':[{{'index':1,'media_id':'local-media','type':'photo','url':{media_url:?},'filename':'01.jpg','mime_type':'image/jpeg'}}]}})))
 self._runner.config=self.config
 return real(self,{media_url:?},staging_dir,**kwargs)
DownloadRunner.run=local
class Extraction:
 def __init__(self,*a,**kw): pass
 def run(self,url,*a,**kw):
  from xarchive_downloader.protocol_v2 import ExtractionResult,ExtractionMediaItem
  return ExtractionResult(tweet_id='12091',url='https://x.com/fixture/status/12091',tweet_type='post',text='real-tool text',media=(ExtractionMediaItem(index=1,media_id='local-media',media_type='photo',url={media_url:?},filename='01.jpg',mime_type='image/jpeg'),))
w.ExtractionRunner=Extraction
w.run_v2_worker(gallery_dl_executable={gallery:?},proxy_mode='direct',timeout_seconds=15)
"#,
                    source = Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../sidecar/src")
                        .to_str()
                        .unwrap()
                );
                let runtime = ExecutorRuntime::with_linux_containment(
                    ExecutorConfig {
                        telegram_config_file: None,
                        telegram: crate::config::TelegramConfig::default(),
                        archive_root: root.join("archive"),
                        staging_root: root.join("staging"),
                        database_path: database_path.clone(),
                        sidecar_program: Some("/usr/bin/python3".into()),
                        sidecar_args: vec!["-u".into(), "-c".into(), script],
                        gallery_dl_program: Some(gallery.clone()),
                        aria2_program: Some(aria2.clone()),
                        use_aria2,
                        network: ExecutorNetworkConfig {
                            proxy_mode: ProxyMode::Direct,
                            transfer_timeout: Duration::from_secs(15),
                            aria2_connect_timeout: Duration::from_secs(3),
                            aria2_idle_timeout: Duration::from_secs(5),
                            aria2_max_tries: 1,
                            ..Default::default()
                        },
                    },
                    LinuxExecutionContainment {
                        delegated_parent: parent.clone(),
                        python: PathBuf::from("/usr/bin/python3"),
                        launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
                    },
                );
                let settings = OutputSettings {
                    naming_mode: NamingMode::Template,
                    filename_template: "{tweet_id}_{index}".into(),
                    ..OutputSettings::default()
                };
                let preview = xarchive_storage::render_media_filenames(
                    &["01.jpg".into()],
                    None,
                    "12091",
                    &settings,
                )
                .unwrap();
                let prepared = if d1 {
                    ArchiveJobSubmissionAdapter
                        .prepare_with_metadata_settings(
                            &archive_request("12091"),
                            "real-publish",
                            MetadataOutputSettings {
                                naming_mode: settings.naming_mode,
                                filename_template: settings.filename_template.clone(),
                                ..Default::default()
                            },
                            DownloaderArgumentSnapshot::historical(),
                        )
                        .unwrap()
                } else {
                    ArchiveJobSubmissionAdapter
                        .prepare_with_downloader_settings(
                            &archive_request("12091"),
                            "real-publish",
                            settings,
                            DownloaderArgumentSnapshot::historical(),
                        )
                        .unwrap()
                };
                let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
                let job_id = prepared.job_id.clone();
                runtime
                    .service()
                    .submit_persisted(&mut persistence, prepared)
                    .unwrap();
                assert_eq!(
                    runtime.recover_startup().unwrap().join().unwrap(),
                    vec![(job_id.clone(), Ok(true))]
                );
                wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
                let final_dir = root.join("archive/Tweets/12091");
                assert_eq!(
                    std::fs::read(final_dir.join(&preview[0])).unwrap(),
                    b"real-tool-media"
                );
                let metadata: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(final_dir.join("tweet.json")).unwrap())
                        .unwrap();
                assert_eq!(metadata["media"][0]["file"], preview[0]);
                if d1 {
                    let yaml: serde_json::Value = serde_yaml::from_slice(
                        &std::fs::read(final_dir.join("tweet.yaml")).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(yaml, metadata);
                    assert!(!final_dir.join("tweet.txt").exists());
                } else {
                    assert!(final_dir.join("tweet.txt").is_file());
                    assert!(!final_dir.join("tweet.yaml").exists());
                }
                let mut comparable = metadata;
                // Separate jobs have different publication times by definition.
                assert!(!comparable["archived_at"].as_str().unwrap().is_empty());
                comparable.as_object_mut().unwrap().remove("archived_at");
                engine_metadata.push(comparable);
                let database = Database::open(&database_path).unwrap();
                assert_eq!(
                    database
                        .tweet_archive_facts("12091")
                        .unwrap()
                        .unwrap()
                        .media[0]
                        .relative_path,
                    preview[0]
                );
                assert_eq!(
                    database
                        .archive_recovery_manifest_v2(&job_id)
                        .unwrap()
                        .unwrap()
                        .phase,
                    xarchive_storage::ArchiveRecoveryPhase::Committed
                );
                drop(database);
                drop(runtime);
                std::fs::remove_dir_all(root).unwrap();
            }
        }
        for metadata in &engine_metadata[1..] {
            assert_eq!(
                &engine_metadata[0], metadata,
                "both engines and snapshot versions must publish the same business metadata"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires explicitly delegated disposable cgroup and real aria2"]
    fn production_real_aria2_drains_recorded_writer() {
        let program = std::env::var("TW2TG_TEST_ARIA2").expect("explicit real aria2 executable");
        let url = std::env::var("TW2TG_TEST_MEDIA_URL").expect("explicit local media URL");
        assert!(url.starts_with("http://127.0.0.1:"));
        let database_path = temporary_database_path("real-aria2");
        let root = database_path.parent().unwrap().parent().unwrap();
        std::fs::create_dir_all(root).unwrap();
        let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
        let relative = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .unwrap();
        let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
        let domain = AttemptWriterDomain {
            provisioning: LinuxExecutionContainment {
                delegated_parent: parent.clone(),
                python: PathBuf::from("/usr/bin/python3"),
                launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
            },
            record_root: root.join("records"),
            identity: xarchive_workflow::linux_containment::AttemptIdentity::new(
                root.to_str().unwrap(),
                "real-transfer",
                1,
            )
            .unwrap(),
        };
        let plan = xarchive_download::MediaTransferPlan {
            items: vec![xarchive_download::TransferPlanItem {
                media_id: "real-media-1".into(),
                url,
                filename: "media.jpg".into(),
                directory: root.to_string_lossy().into_owned(),
                headers: vec![],
                media_type: "photo".into(),
                mime_type: None,
            }],
        };
        crate::production::assert_real_transfer(&plan, &program, &domain);
        assert_eq!(
            std::fs::read(root.join("media.jpg")).unwrap(),
            b"real-tool-media"
        );
        assert!(
            !std::fs::read_dir(parent)
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("tw2tg-att-"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires explicitly delegated disposable cgroup"]
    fn production_transfer_rpc_fixture_drains_recorded_writer() {
        use std::os::unix::fs::PermissionsExt;
        let database_path = temporary_database_path("transfer-rpc");
        let root = database_path.parent().unwrap().parent().unwrap();
        std::fs::create_dir_all(root).unwrap();
        let program = root.join("aria2-fixture");
        std::fs::write(&program, include_str!("../tests/fixtures/aria2_rpc.py")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
        let relative = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .unwrap();
        let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
        let domain = AttemptWriterDomain {
            provisioning: LinuxExecutionContainment {
                delegated_parent: parent.clone(),
                python: PathBuf::from("/usr/bin/python3"),
                launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
            },
            record_root: root.join("records"),
            identity: xarchive_workflow::linux_containment::AttemptIdentity::new(
                root.to_str().unwrap(),
                "transfer-job",
                1,
            )
            .unwrap(),
        };
        let plan = xarchive_download::MediaTransferPlan {
            items: vec![xarchive_download::TransferPlanItem {
                media_id: "media-1".into(),
                url: "http://127.0.0.1/fixture".into(),
                filename: "media.jpg".into(),
                directory: root.to_string_lossy().into_owned(),
                headers: vec![],
                media_type: "photo".into(),
                mime_type: None,
            }],
        };
        crate::production::assert_transfer_fixture_success(
            &plan,
            program.to_str().unwrap(),
            &domain,
        );
        assert_eq!(
            std::fs::read(root.join("media.jpg")).unwrap(),
            b"fixture-transfer"
        );
        assert!(
            !std::fs::read_dir(parent)
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("tw2tg-att-"))
        );
        std::thread::sleep(std::time::Duration::from_secs(3));
        assert!(
            !std::fs::read_dir(root)
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry.file_name().to_string_lossy().ends_with(".late")),
            "setsid descendants must be drained before their write deadline"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires explicitly delegated disposable cgroup"]
    fn production_scheduler_publishes_protocol_fixture_bytes() {
        let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
        let relative = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .unwrap();
        let database_path = temporary_database_path("production-publish");
        std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
        let descriptor_path = database_path.with_extension("launch.json");
        let launcher_copy = database_path.with_extension("launcher.py");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
            &launcher_copy,
        )
        .unwrap();
        std::fs::write(&descriptor_path, serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "delegated_parent": Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/')),
            "python": "/usr/bin/python3",
            "launcher": launcher_copy,
        })).unwrap()).unwrap();
        let provisioning = LinuxExecutionContainment::from_launch_arguments([
            std::ffi::OsString::from("--linux-execution-descriptor"),
            descriptor_path.clone().into_os_string(),
        ])
        .unwrap()
        .unwrap();
        let runtime_root = std::env::temp_dir().join(format!("tw2tg-rt-{}", std::process::id()));
        let mut state = crate::runtime::RuntimeState::initialize_with_containment(
            runtime_root.clone(),
            Some(provisioning.clone()),
        );
        assert_eq!(
            state.executor.linux_containment().unwrap().delegated_parent,
            provisioning.delegated_parent
        );
        let replacement = state.executor.config().clone();
        state.replace_executor(replacement).unwrap();
        assert_eq!(
            state.executor.linux_containment().unwrap().launcher,
            provisioning.launcher
        );
        // Revalidation must refuse before closing the old executor or transport.
        let original_config = state.executor.config().clone();
        let launcher_bytes = std::fs::read(&launcher_copy).unwrap();
        std::fs::remove_file(&launcher_copy).unwrap();
        let mut rejected_config = original_config.clone();
        rejected_config.archive_root = runtime_root.join("must-not-be-installed");
        assert!(state.replace_executor(rejected_config).is_err());
        assert_eq!(
            state.executor.config().archive_root,
            original_config.archive_root
        );
        assert!(state.executor.recover_startup().unwrap().join().is_ok());
        std::fs::write(&launcher_copy, launcher_bytes).unwrap();
        state.replace_executor(original_config).unwrap();
        drop(state);
        std::fs::remove_dir_all(runtime_root).unwrap();
        assert!(
            LinuxExecutionContainment::from_launch_arguments([
                std::ffi::OsString::from("--linux-execution-descriptor"),
                descriptor_path.clone().into_os_string(),
                std::ffi::OsString::from("--linux-execution-descriptor"),
                descriptor_path.into_os_string(),
            ])
            .is_err()
        );
        let root = database_path.parent().unwrap().parent().unwrap();
        let script = r#"import sys,json,pathlib
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
c=json.loads(sys.stdin.readline())
p=pathlib.Path(c['staging_dir']); p.mkdir(parents=True,exist_ok=True); (p/'media.jpg').write_bytes(b'fixture-media')
r={'tweet_id':'12091','url':c['url'],'tweet_type':'post','media':[]}
d={'tweet_id':'12091','url':c['url'],'media':[{'index':1,'media_type':'photo','filename':'media.jpg','relative_path':'media.jpg','size_bytes':13}],'result':r}
print(json.dumps({'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':d,'result':r}),flush=True)
sys.stdin.read()
"#;
        let runtime = ExecutorRuntime::with_linux_containment(
            ExecutorConfig {
                telegram_config_file: None,
                telegram: crate::config::TelegramConfig::default(),
                archive_root: root.join("archive"),
                staging_root: root.join("staging"),
                database_path: database_path.clone(),
                sidecar_program: Some("/usr/bin/python3".into()),
                gallery_dl_program: None,
                sidecar_args: vec!["-u".into(), "-c".into(), script.into()],
                aria2_program: None,
                use_aria2: false,
                network: ExecutorNetworkConfig {
                    proxy_mode: ProxyMode::Direct,
                    ..Default::default()
                },
            },
            provisioning,
        );
        let prepared = ArchiveJobSubmissionAdapter
            .prepare_with_downloader_settings(
                &archive_request("12091"),
                "production-publish-job",
                OutputSettings {
                    naming_mode: NamingMode::Template,
                    filename_template: "{tweet_id}_{index}".into(),
                    ..OutputSettings::default()
                },
                DownloaderArgumentSnapshot::historical(),
            )
            .unwrap();
        let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
        let job_id = prepared.job_id.clone();
        runtime
            .service()
            .submit_persisted(&mut persistence, prepared)
            .unwrap();
        // Simulate process exit after durable enrollment but before scheduling.
        // Public startup must execute this unstarted job, not require PREPARED.
        assert_eq!(persistence.current_attempt(&job_id).unwrap(), 0);
        assert_eq!(
            runtime.recover_startup().unwrap().join().unwrap(),
            vec![(job_id.clone(), Ok(true))]
        );
        // A pre-PREPARED interrupted attempt must drain its predecessor and
        // claim a successor, not stay permanently gated on a missing journal.
        let interrupted = ArchiveJobSubmissionAdapter
            .prepare_with_downloader_settings(
                &archive_request("12091"),
                "production-interrupted-job",
                OutputSettings::default(),
                DownloaderArgumentSnapshot::historical(),
            )
            .unwrap();
        // Use a separate workspace: same destination in the completed job must
        // never be overwritten by this recovery probe.
        let interrupted_path = root.join("interrupted/config/archive.sqlite3");
        std::fs::create_dir_all(interrupted_path.parent().unwrap()).unwrap();
        let mut interrupted_config = runtime.config().clone();
        interrupted_config.database_path = interrupted_path.clone();
        interrupted_config.archive_root = root.join("interrupted/archive");
        interrupted_config.staging_root = root.join("interrupted/staging");
        let interrupted_runtime = ExecutorRuntime::with_linux_containment(
            interrupted_config,
            runtime.containment.clone().unwrap(),
        );
        let mut interrupted_persistence = StorageJobPersistence::open(&interrupted_path).unwrap();
        let interrupted_id = interrupted.job_id.clone();
        interrupted_runtime
            .service()
            .submit_persisted(&mut interrupted_persistence, interrupted)
            .unwrap();
        interrupted_persistence
            .begin_attempt(&interrupted_id)
            .unwrap();
        interrupted_persistence
            .persist_state(&JobSnapshot {
                job_id: interrupted_id.clone(),
                tweet_id: "12091".into(),
                state: JobState::Interrupted,
            })
            .unwrap();
        assert_eq!(
            interrupted_runtime
                .recover_startup()
                .unwrap()
                .join()
                .unwrap(),
            vec![(interrupted_id.clone(), Ok(true))]
        );
        assert_eq!(
            interrupted_persistence
                .current_attempt(&interrupted_id)
                .unwrap(),
            2
        );
        assert_eq!(
            interrupted_persistence
                .snapshot(&interrupted_id)
                .unwrap()
                .state,
            JobState::Complete
        );
        drop(interrupted_runtime);
        wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
        let tweet = Database::open(&database_path)
            .unwrap()
            .tweet_row_id("12091")
            .unwrap();
        assert_eq!(
            std::fs::read(root.join("archive/Tweets/12091/12091_01.jpg")).unwrap(),
            b"fixture-media"
        );
        let events = persistence.stored_events(&job_id).unwrap();
        for expected in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
            assert_eq!(
                events.iter().filter(|(kind, _)| kind == expected).count(),
                1
            );
        }
        let database = Database::open(&database_path).unwrap();
        assert_eq!(
            database
                .archive_recovery_manifest_v2(&job_id)
                .unwrap()
                .unwrap()
                .phase,
            xarchive_storage::ArchiveRecoveryPhase::Committed
        );
        let files =
            FileStore::with_staging_root(root.join("archive"), root.join("staging")).unwrap();
        let mut archive = xarchive_storage::ArchiveService::new(database, files);
        for _ in 0..2 {
            archive
                .recover_internal_rename_v2(&job_id, tweet, Path::new("Tweets/12091"), "later")
                .unwrap();
        }
        assert_eq!(persistence.stored_events(&job_id).unwrap(), events);
        assert!(
            !std::fs::read_dir(Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/')))
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("tw2tg-att-"))
        );
        drop(archive);
        // Re-open a valid COMMITTED fixture as Downloaded to exercise the
        // current startup gate, without enabling replay in production.
        let database = Database::open(&database_path).unwrap();
        database
            .execute_batch(&format!(
                "UPDATE jobs SET state = 'DOWNLOADED' WHERE id = '{job_id}'"
            ))
            .unwrap();
        // An unprovisioned runtime still preserves the historical rejection.
        runtime
            .recover_startup_with_v2(false)
            .unwrap()
            .join()
            .unwrap();
        wait_for_persisted_state(&database_path, &job_id, JobState::Failed);
        assert_eq!(
            persistence
                .stored_job_summary(&job_id)
                .unwrap()
                .last_error_code
                .as_deref(),
            Some("ARCHIVE_RECOVERY_V2_NOT_READY")
        );
        assert_eq!(
            persistence
                .stored_events(&job_id)
                .unwrap()
                .iter()
                .filter(|(kind, _)| kind == "JOB_COMPLETED")
                .count(),
            1
        );
        // Model interruption after Storage COMMITTED but before the Desktop
        // terminal transaction: that transaction and its events never existed.
        database
            .execute_batch(&format!(
                "DELETE FROM events WHERE job_id = '{job_id}' AND (event_type IN ('JOB_COMPLETED', 'JOB_FAILED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state IN ('COMPLETE','FAILED'))); UPDATE jobs SET state = 'DOWNLOADED', last_error_code = NULL, finished_at = NULL WHERE id = '{job_id}'"
            ))
            .unwrap();
        let attempt = database.archive_attempt(&job_id).unwrap();
        let before_recovery = persistence.stored_events(&job_id).unwrap();
        let peer = ProductionCoordinator::open(&database_path).unwrap();
        let held =
            ExecutionReservation::acquire(&peer, &job_id, &root.join("archive/Tweets/12091"))
                .unwrap();
        assert!(
            !recover_v2_on_runner(
                &runtime.service(),
                database_path.clone(),
                root.join("archive"),
                root.join("staging"),
                runtime.containment.clone(),
                job_id.clone(),
                "12091".into(),
            )
            .unwrap()
        );
        assert_eq!(persistence.stored_events(&job_id).unwrap(), before_recovery);
        drop(held);
        for replay in 0..2 {
            assert!(
                recover_v2_on_runner(
                    &runtime.service(),
                    database_path.clone(),
                    root.join("archive"),
                    root.join("staging"),
                    runtime.containment.clone(),
                    job_id.clone(),
                    "12091".into(),
                )
                .unwrap()
            );
            if replay == 0 {
                assert_eq!(
                    persistence.snapshot(&job_id).unwrap().state,
                    JobState::Complete
                );
            }
        }
        assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
        let after_recovery = persistence.stored_events(&job_id).unwrap();
        assert_eq!(
            after_recovery
                .iter()
                .filter(|(kind, _)| kind == "JOB_COMPLETED")
                .count(),
            1
        );
        std::fs::write(
            root.join("archive/Tweets/12091/12091_01.jpg"),
            b"fixture-wrong",
        )
        .unwrap();
        assert!(
            recover_v2_on_runner(
                &runtime.service(),
                database_path.clone(),
                root.join("archive"),
                root.join("staging"),
                runtime.containment.clone(),
                job_id.clone(),
                "12091".into(),
            )
            .is_err()
        );
        assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
        assert_eq!(persistence.stored_events(&job_id).unwrap(), after_recovery);
        std::fs::write(
            root.join("archive/Tweets/12091/12091_01.jpg"),
            b"fixture-media",
        )
        .unwrap();
        assert!(
            recover_v2_on_runner(
                &runtime.service(),
                database_path.clone(),
                root.join("archive"),
                root.join("staging"),
                None,
                job_id.clone(),
                "12091".into(),
            )
            .is_err()
        );
        assert_eq!(persistence.stored_events(&job_id).unwrap(), after_recovery);
        // Model the durable directory-commit crash window: bytes already moved,
        // but the journal's finalization transaction has not yet committed.
        // Undo candidate retirement too: PREPARED still owns this exact business
        // metadata, whereas the successful transaction retired its copy.
        let candidate: xarchive_core::ArchiveMetadata = serde_json::from_slice(
            &std::fs::read(root.join("archive/Tweets/12091/tweet.json")).unwrap(),
        )
        .unwrap();
        let candidate_json = serde_json::to_string(&candidate).unwrap();
        let candidate_digest = {
            use sha2::Digest;
            format!("{:x}", sha2::Sha256::digest(candidate_json.as_bytes()))
        };
        database.execute_batch(&format!(
            "INSERT INTO archive_recovery_v2_candidate (job_id, attempt_count, tweet_row_id, metadata_json, metadata_sha256, created_at, updated_at) SELECT '{job_id}', {attempt}, {tweet}, merged_metadata_json, '{candidate_digest}', 'fixture', 'fixture' FROM tweets WHERE id = {tweet}"
        )).unwrap();
        database
            .execute_batch(&format!(
                "DELETE FROM events WHERE job_id = '{job_id}' AND (event_type IN ('JOB_COMPLETED','DOWNLOAD_COMPLETED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state IN ('COMPLETE','DOWNLOADED'))); UPDATE archive_recovery_v2 SET phase = 'PREPARED' WHERE job_id = '{job_id}'; UPDATE jobs SET state = 'DOWNLOADED', finished_at = NULL WHERE id = '{job_id}'"
            ))
            .unwrap();
        let recovery = runtime.recover_startup().unwrap().join().unwrap();
        assert_eq!(recovery, vec![(job_id.clone(), Ok(true))]);
        for _ in 0..100 {
            if database
                .archive_recovery_manifest_v2(&job_id)
                .unwrap()
                .unwrap()
                .phase
                == xarchive_storage::ArchiveRecoveryPhase::Committed
            {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(
            database
                .archive_recovery_manifest_v2(&job_id)
                .unwrap()
                .unwrap()
                .phase,
            xarchive_storage::ArchiveRecoveryPhase::Committed
        );
        assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
        wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
        // Replay each durable rename phase through startup, not just Storage's
        // helper. Restore the original source/temporary mapping in the attempt
        // staging tree without re-rendering settings during recovery.
        for progress in ["PLANNED", "TEMPORARY", "FINAL"] {
            database.execute_batch(&format!(
                "INSERT INTO archive_recovery_v2_candidate (job_id, attempt_count, tweet_row_id, metadata_json, metadata_sha256, created_at, updated_at) SELECT '{job_id}', {attempt}, {tweet}, merged_metadata_json, '{candidate_digest}', 'fixture', 'fixture' FROM tweets WHERE id = {tweet}"
            )).unwrap();
            let files =
                FileStore::with_staging_root(root.join("archive"), root.join("staging")).unwrap();
            let staging = files.create_attempt_staging_dir(&job_id, attempt).unwrap();
            std::fs::remove_dir(&staging).unwrap();
            std::fs::rename(root.join("archive/Tweets/12091"), &staging).unwrap();
            if progress != "FINAL" {
                let journal = database
                    .archive_recovery_manifest_v2(&job_id)
                    .unwrap()
                    .unwrap();
                let manifest: xarchive_storage::ArchiveRecoveryManifestV2 =
                    serde_json::from_str(&journal.manifest_json).unwrap();
                for edge in manifest.rename_plan {
                    std::fs::rename(
                        staging.join(edge.final_path),
                        staging.join(if progress == "TEMPORARY" {
                            edge.temporary_path
                        } else {
                            edge.source_path
                        }),
                    )
                    .unwrap();
                }
            }
            database.execute_batch(&format!(
                "DELETE FROM events WHERE job_id = '{job_id}' AND (event_type IN ('JOB_COMPLETED','DOWNLOAD_COMPLETED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state IN ('COMPLETE','DOWNLOADED'))); UPDATE archive_recovery_v2 SET phase = 'PREPARED', rename_progress = '{progress}' WHERE job_id = '{job_id}'; UPDATE jobs SET state = 'DOWNLOADED', finished_at = NULL WHERE id = '{job_id}'"
            )).unwrap();
            let recovery = runtime
                .recover_startup_with_v2(true)
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(recovery, vec![(job_id.clone(), Ok(true))]);
            wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
            assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
            assert_eq!(
                std::fs::read(root.join("archive/Tweets/12091/12091_01.jpg")).unwrap(),
                b"fixture-media"
            );
        }
        let completed_events = persistence.stored_events(&job_id).unwrap();
        for kind in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
            assert_eq!(
                completed_events
                    .iter()
                    .filter(|(event, _)| event == kind)
                    .count(),
                1
            );
        }
        database
            .execute_batch(&format!(
                "UPDATE archive_recovery_v2 SET manifest_json = '{{' WHERE job_id = '{job_id}'"
            ))
            .unwrap();
        assert!(
            recover_v2_on_runner(
                &runtime.service(),
                database_path.clone(),
                root.join("archive"),
                root.join("staging"),
                runtime.containment.clone(),
                job_id.clone(),
                "12091".into(),
            )
            .is_err()
        );
        assert_eq!(
            persistence.stored_events(&job_id).unwrap(),
            completed_events
        );
        // A published state without a journal cannot start a successor.
        database.execute_batch(&format!(
            "DELETE FROM archive_recovery_v2 WHERE job_id = '{job_id}'; UPDATE jobs SET state = 'DOWNLOADED' WHERE id = '{job_id}'"
        )).unwrap();
        let missing_results = runtime.recover_startup().unwrap().join().unwrap();
        assert_eq!(missing_results.len(), 1);
        assert!(missing_results[0].1.is_err());
        assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
        assert_eq!(
            persistence.snapshot(&job_id).unwrap().state,
            JobState::Downloaded
        );
        assert_eq!(
            persistence.stored_events(&job_id).unwrap(),
            completed_events
        );
        database
            .execute_batch(&format!(
                "UPDATE jobs SET state = 'DOWNLOADED' WHERE id = '{job_id}'"
            ))
            .unwrap();
        let before_rejection = persistence.stored_events(&job_id).unwrap();
        let rejected = runtime
            .recover_startup_with_v2(true)
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].0, job_id);
        assert!(rejected[0].1.is_err());
        assert_eq!(
            persistence.snapshot(&job_id).unwrap().state,
            JobState::Downloaded
        );
        assert_eq!(
            persistence.stored_events(&job_id).unwrap(),
            before_rejection
        );
        database
            .execute_batch(&format!(
                "DELETE FROM archive_recovery_v2 WHERE job_id = '{job_id}'"
            ))
            .unwrap();
        assert!(
            recover_v2_on_runner(
                &runtime.service(),
                database_path.clone(),
                root.join("archive"),
                root.join("staging"),
                runtime.containment.clone(),
                job_id.clone(),
                "12091".into(),
            )
            .is_err()
        );
        assert_eq!(
            persistence.stored_events(&job_id).unwrap(),
            completed_events
        );
        drop(database);
        runtime.executor.shutdown().unwrap();
        drop(persistence);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn v2_execution_settings_are_preserved_by_decoding() {
        let settings = OutputSettings {
            naming_mode: NamingMode::Template,
            filename_template: "{tweet_id}_{index}".to_owned(),
            export_json: false,
            export_text: true,
        };
        let json = serde_json::to_string(&ExecutionSpecV2 {
            request: archive_request("12002"),
            output_settings: settings.clone(),
        })
        .expect("serialize v2");

        let decoded = decode_execution_spec(2, &json).expect("decode v2");

        assert_eq!(decoded.output_settings, settings);
    }

    #[test]
    fn execution_spec_v1_rejects_invalid_json_and_v2_rejects_request_identity_mismatch() {
        assert!(matches!(
            decode_execution_spec(1, "{"),
            Err(ExecutionSpecError::InvalidJson(_))
        ));
        let request = archive_request("12004");
        let json = serde_json::to_string(&ExecutionSpecV2 {
            request,
            output_settings: OutputSettings::default(),
        })
        .expect("serialize v2");
        let decoded = decode_execution_spec(2, &json).expect("decode v2");
        let error = validate_execution_spec_identity(decoded, "12003")
            .expect_err("mismatched identity must fail before process creation");
        assert!(error.to_string().contains("identity does not match Job"));
    }

    #[test]
    fn execution_spec_v2_decodes_request_and_immutable_output_snapshot() {
        let request = archive_request("12002");
        let envelope = ExecutionSpecV2 {
            request: request.clone(),
            output_settings: OutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{username}_{tweet_id}".to_owned(),
                export_json: false,
                export_text: true,
            },
        };
        let json = serde_json::to_string(&envelope).expect("serialize v2");

        let decoded = decode_execution_spec(2, &json).expect("decode v2");

        assert_eq!(decoded.request.tweet.tweet_id, request.tweet.tweet_id);
        assert_eq!(decoded.request.browser, request.browser);
        assert_eq!(decoded.request.profile, request.profile);
        assert_eq!(decoded.output_settings, envelope.output_settings);
    }

    #[test]
    fn submission_adapter_persists_the_captured_output_settings_snapshot() {
        let request = archive_request("12005");
        let settings = OutputSettings {
            naming_mode: NamingMode::Template,
            filename_template: "{username}_{tweet_id}".into(),
            export_json: false,
            export_text: true,
        };

        let prepared = ArchiveJobSubmissionAdapter
            .prepare_with_output_settings(&request, "now", settings.clone())
            .expect("prepare task with snapshot");
        let decoded = decode_execution_spec(prepared.schema_version, &prepared.request_json)
            .expect("decode persisted snapshot");

        assert_eq!(decoded.output_settings, settings);
        assert_eq!(decoded.request.tweet.tweet_id, "12005");
    }

    #[test]
    fn v3_submission_uses_internal_recovery_for_storage_backed_archive_jobs() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        let mut request = ArchiveJobSubmissionAdapter
            .prepare_with_downloader_settings(
                &archive_request("12006"),
                "now",
                OutputSettings::default(),
                DownloaderArgumentSnapshot::historical(),
            )
            .expect("prepare v3 task");
        request.schema_version = 3;

        let submitted = persistence
            .create_or_reuse(&request)
            .expect("persist v3 job");
        let recovery_contract = persistence
            .database
            .archive_recovery_contract(&submitted.job.job_id)
            .expect("read recovery contract");

        assert_eq!(recovery_contract, Some(1));
    }

    #[test]
    fn execution_spec_rejects_unknown_version_explicitly() {
        assert!(matches!(
            decode_execution_spec(77, "{}"),
            Err(ExecutionSpecError::UnsupportedVersion(77))
        ));
    }

    fn v3_spec(
        downloader_arguments: DownloaderArgumentSnapshot,
    ) -> (String, crate::ArchiveTweetRequest) {
        let request = archive_request("13001");
        let json = serde_json::to_string(&ExecutionSpecV3 {
            request: request.clone(),
            output_settings: OutputSettings::default(),
            downloader_arguments,
        })
        .expect("serialize v3");
        (json, request)
    }

    #[test]
    fn execution_spec_v3_decodes_request_output_and_downloader_argument_snapshot() {
        let settings = OutputSettings {
            naming_mode: NamingMode::Template,
            filename_template: "{username}_{tweet_id}".to_owned(),
            export_json: false,
            export_text: true,
        };
        let request = archive_request("13002");
        let snapshot = DownloaderArgumentSnapshot {
            gallery_dl: Vec::new(),
            aria2: vec!["--max-connection-per-server".to_owned(), "4".to_owned()],
            policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
            tool_compatibility: ToolCompatibility {
                aria2: Some("1.37.0".to_owned()),
                gallery_dl: Some("1.28.5".to_owned()),
            },
        };
        let json = serde_json::to_string(&ExecutionSpecV3 {
            request: request.clone(),
            output_settings: settings.clone(),
            downloader_arguments: snapshot.clone(),
        })
        .expect("serialize v3");

        let decoded = decode_execution_spec(3, &json).expect("decode v3");

        assert_eq!(decoded.request.tweet.tweet_id, request.tweet.tweet_id);
        assert_eq!(decoded.output_settings, settings);
        assert_eq!(decoded.downloader_arguments, snapshot);
    }

    #[test]
    fn execution_spec_v1_and_v2_decode_to_empty_historical_argument_snapshot() {
        // Historical tasks carry no user arguments and must not be resampled
        // from current settings. They decode to an empty snapshot rather than
        // to today's arguments.
        let v1_json = serde_json::to_string(&archive_request("13003")).expect("serialize v1");
        let decoded_v1 = decode_execution_spec(1, &v1_json).expect("decode v1");
        assert_eq!(
            decoded_v1.downloader_arguments,
            DownloaderArgumentSnapshot::historical()
        );
        assert!(decoded_v1.downloader_arguments.gallery_dl.is_empty());
        assert!(decoded_v1.downloader_arguments.aria2.is_empty());

        let v2_json = serde_json::to_string(&ExecutionSpecV2 {
            request: archive_request("13003"),
            output_settings: OutputSettings::default(),
        })
        .expect("serialize v2");
        let decoded_v2 = decode_execution_spec(2, &v2_json).expect("decode v2");
        assert_eq!(
            decoded_v2.downloader_arguments,
            DownloaderArgumentSnapshot::historical()
        );
        assert!(decoded_v2.downloader_arguments.gallery_dl.is_empty());
        assert!(decoded_v2.downloader_arguments.aria2.is_empty());
    }

    #[test]
    fn execution_spec_v3_refuses_a_mismatched_argument_policy_version() {
        // A snapshot written under a different policy version is refused rather
        // than reinterpreted, so a later allowlist/value-policy change can never
        // silently reinterpret previously saved arguments.
        let snapshot = DownloaderArgumentSnapshot {
            gallery_dl: vec!["--unreviewed-option".to_owned()],
            aria2: Vec::new(),
            policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION + 1,
            tool_compatibility: ToolCompatibility::default(),
        };
        let (json, _) = v3_spec(snapshot);

        let error = decode_execution_spec(3, &json)
            .expect_err("a foreign argument-policy version must be refused");

        assert!(matches!(
            error,
            ExecutionSpecError::DownloaderArgumentPolicy(_)
        ));
        assert!(error.to_string().contains("argument policy version"));
    }

    #[test]
    fn execution_spec_v3_rejects_invalid_json_explicitly() {
        assert!(matches!(
            decode_execution_spec(3, "{"),
            Err(ExecutionSpecError::InvalidJson(_))
        ));
    }

    #[test]
    fn execution_spec_v3_rejects_unknown_fields_in_the_argument_snapshot() {
        // The snapshot is deny_unknown_fields, so an unexpected key is refused
        // at decode time instead of being silently dropped.
        let raw = format!(
            "{{\"request\":{},\"output_settings\":{{}},\"downloader_arguments\":{{\"gallery_dl\":[],\"aria2\":[],\"policy_version\":{},\"tool_compatibility\":{{}},\"unexpected\":1}}}}",
            serde_json::to_string(&archive_request("13004")).expect("serialize request"),
            xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION
        );

        let error =
            decode_execution_spec(3, &raw).expect_err("an unknown snapshot field must be refused");

        assert!(matches!(error, ExecutionSpecError::InvalidJson(_)));
    }

    #[test]
    fn v3_argument_snapshot_round_trips_through_json_without_rewriting_argv() {
        // Stored argv elements are validated string arrays; they round-trip
        // byte-for-byte and are never tokenized, escaped, or rewritten.
        let snapshot = DownloaderArgumentSnapshot {
            gallery_dl: Vec::new(),
            aria2: vec!["--max-connection-per-server".to_owned(), "8".to_owned()],
            policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
            tool_compatibility: ToolCompatibility {
                aria2: Some("1.37.0".to_owned()),
                gallery_dl: None,
            },
        };
        let (json, _) = v3_spec(snapshot.clone());

        let decoded = decode_execution_spec(3, &json).expect("decode v3");

        assert_eq!(decoded.downloader_arguments.gallery_dl, snapshot.gallery_dl);
        assert_eq!(decoded.downloader_arguments.aria2, snapshot.aria2);
        assert_eq!(decoded.downloader_arguments, snapshot);
    }

    #[test]
    fn downloader_argument_diagnostics_do_not_disclose_values_or_tool_versions() {
        let snapshot = DownloaderArgumentSnapshot {
            gallery_dl: Vec::new(),
            aria2: vec!["--max-tries".into(), "secret-value".into()],
            policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
            tool_compatibility: ToolCompatibility {
                aria2: Some("1.37.0-private-build".into()),
                gallery_dl: None,
            },
        };
        let diagnostic = format!("{snapshot:?}");
        assert!(!diagnostic.contains("secret-value"));
        assert!(!diagnostic.contains("1.37.0-private-build"));
        assert!(diagnostic.contains("aria2_argument_count"));
    }

    fn insert_downloaded_job<P: JobPersistence>(persistence: &mut P, job_id: &str, tweet_id: &str) {
        persistence
            .create_or_reuse(&request(job_id, tweet_id))
            .expect("create job");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
            JobState::Downloaded,
        ] {
            persistence
                .persist_state(&JobSnapshot {
                    job_id: job_id.into(),
                    tweet_id: tweet_id.into(),
                    state,
                })
                .expect("advance job state");
        }
    }

    fn v2_request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
        ArchiveJobRequest {
            job_id: job_id.to_owned(),
            tweet_id: tweet_id.to_owned(),
            request_id: format!("test-request-{job_id}"),
            request_json: serde_json::to_string(&ExecutionSpecV2 {
                request: archive_request(tweet_id),
                output_settings: OutputSettings::default(),
            })
            .expect("serialize v2 execution spec"),
            schema_version: 2,
        }
    }

    fn insert_v2_downloaded_job(
        persistence: &mut StorageJobPersistence,
        job_id: &str,
        tweet_id: &str,
    ) {
        persistence
            .create_or_reuse(&v2_request(job_id, tweet_id))
            .expect("create v2 job");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
            JobState::Downloaded,
        ] {
            persistence
                .persist_state(&JobSnapshot {
                    job_id: job_id.into(),
                    tweet_id: tweet_id.into(),
                    state,
                })
                .expect("advance v2 job state");
        }
    }

    fn temporary_database_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "xarchive-executor-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system clock")
                    .as_nanos()
            ))
            .join("config")
            .join("archive.sqlite3")
    }

    fn wait_for_persisted_state(
        database_path: &Path,
        job_id: &str,
        expected: JobState,
    ) -> JobSnapshot {
        for _ in 0..100 {
            if let Ok(persistence) = StorageJobPersistence::open(database_path)
                && let Ok(snapshot) = persistence.snapshot(job_id)
                && snapshot.state == expected
            {
                return snapshot;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("Job {job_id} did not reach {expected:?}");
    }

    #[derive(Debug)]
    struct FakeJobExecution {
        result: Result<JobExecutionResult, JobExecutionError>,
        calls: Vec<String>,
    }

    impl FakeJobExecution {
        fn success(files_copied: u32, archive_directory: &str) -> Self {
            Self {
                result: Ok(JobExecutionResult {
                    files_copied,
                    archive_directory: archive_directory.to_owned(),
                    download_event_recorded: false,
                    state_already_updated: false,
                }),
                calls: Vec::new(),
            }
        }

        fn failure(error_code: &str, error_message: &str) -> Self {
            Self {
                result: Err(JobExecutionError {
                    error_code: error_code.to_owned(),
                    error_message: error_message.to_owned(),
                    persistence_already_updated: false,
                }),
                calls: Vec::new(),
            }
        }
    }

    impl JobExecution for FakeJobExecution {
        fn execute(
            &mut self,
            job: &JobSnapshot,
            _cancellation: &CancellationToken,
        ) -> Result<JobExecutionResult, JobExecutionError> {
            self.calls.push(job.job_id.clone());
            self.result.clone()
        }
    }

    struct CancellationAwareExecution;

    impl JobExecution for CancellationAwareExecution {
        fn execute(
            &mut self,
            _job: &JobSnapshot,
            cancellation: &CancellationToken,
        ) -> Result<JobExecutionResult, JobExecutionError> {
            while !cancellation.is_cancelled() {
                thread::sleep(std::time::Duration::from_millis(2));
            }
            Ok(JobExecutionResult {
                files_copied: 1,
                archive_directory: "archives/cancelled-late-result".to_owned(),
                download_event_recorded: false,
                state_already_updated: false,
            })
        }
    }

    struct DropNotifiesExecution {
        dropped: Sender<()>,
    }

    impl JobExecution for DropNotifiesExecution {
        fn execute(
            &mut self,
            _job: &JobSnapshot,
            _cancellation: &CancellationToken,
        ) -> Result<JobExecutionResult, JobExecutionError> {
            Ok(JobExecutionResult {
                files_copied: 0,
                archive_directory: "archives/drop-probe".to_owned(),
                download_event_recorded: false,
                state_already_updated: false,
            })
        }
    }

    impl Drop for DropNotifiesExecution {
        fn drop(&mut self) {
            let _ = self.dropped.send(());
        }
    }

    #[test]
    fn execute_reply_waits_until_execution_resources_are_dropped() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        handle
            .submit(request("job-drop-order", "tweet-drop-order"))
            .unwrap();
        let (dropped_sender, dropped_receiver) = mpsc::channel();
        let result = handle
            .execute(
                "job-drop-order",
                Box::new(DropNotifiesExecution {
                    dropped: dropped_sender,
                }),
            )
            .expect("execution result");

        assert_eq!(result.archive_directory, "archives/drop-probe");
        dropped_receiver
            .try_recv()
            .expect("execution must be torn down before the caller receives its result");
        drop(executor);
    }

    #[test]
    fn submits_and_reuses_an_active_job_for_duplicate_tweets() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        let first = handle.submit(request("job-1", "tweet-1")).unwrap();
        let second = handle.submit(request("job-2", "tweet-1")).unwrap();

        assert!(first.created);
        assert!(!second.created);
        assert_eq!(second.job.job_id, "job-1");
        assert_eq!(second.job.state, JobState::Queued);
    }

    #[test]
    fn cancel_is_idempotent_and_does_not_change_terminal_state() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        handle.submit(request("job-1", "tweet-1")).unwrap();

        let cancelled = handle.cancel("job-1").unwrap();
        assert_eq!(cancelled.state, JobState::Cancelled);
        assert_eq!(handle.cancel("job-1").unwrap(), cancelled);
    }

    #[test]
    fn running_cancel_signals_execution_and_fences_late_success() {
        let executor = Arc::new(JobExecutor::new());
        let handle = executor.handle();
        handle
            .submit(request("job-cancel-running", "tweet-cancel-running"))
            .unwrap();
        let worker_handle = handle.clone();
        let execution = thread::spawn(move || {
            worker_handle
                .execute("job-cancel-running", Box::new(CancellationAwareExecution))
                .expect("execution response")
        });

        thread::sleep(std::time::Duration::from_millis(10));
        let cancelled = handle.cancel("job-cancel-running").expect("cancel");
        let late_result = execution.join().expect("execution thread");

        assert_eq!(cancelled.state, JobState::Cancelled);
        assert_eq!(
            late_result.archive_directory,
            "archives/cancelled-late-result"
        );
        assert_eq!(
            handle.snapshot("job-cancel-running").unwrap().state,
            JobState::Cancelled
        );
        drop(executor);
    }

    #[test]
    fn shutdown_interrupts_active_jobs_and_rejects_later_commands() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        handle.submit(request("job-1", "tweet-1")).unwrap();
        handle.submit(request("job-2", "tweet-2")).unwrap();

        executor.shutdown().unwrap();
        assert_eq!(handle.snapshot("job-1").unwrap_err(), ExecutorError::Closed);
    }

    #[test]
    fn concurrent_submissions_keep_one_active_worker_per_tweet() {
        let executor = Arc::new(JobExecutor::with_capacity(16));
        let mut workers = Vec::new();
        for index in 0..8 {
            let executor = Arc::clone(&executor);
            workers.push(thread::spawn(move || {
                executor
                    .handle()
                    .submit(request(&format!("job-{index}"), "tweet-1"))
                    .unwrap()
            }));
        }
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.created).count(), 1);
    }

    #[test]
    fn start_preserves_job_state_machine_boundaries() {
        let executor = JobExecutor::new();
        let handle = executor.handle();
        handle.submit(request("job-1", "tweet-1")).unwrap();
        assert_eq!(handle.start("job-1").unwrap().state, JobState::Validating);
        assert_eq!(
            handle.start("job-1").unwrap_err(),
            ExecutorError::InvalidTransition {
                from: JobState::Validating,
                to: JobState::Validating,
            }
        );
    }

    #[test]
    fn application_service_delegates_submit_query_cancel_and_shutdown() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let submitted = service.submit(request("job-1", "tweet-1")).expect("submit");
        assert!(submitted.created);
        assert_eq!(service.query("job-1").unwrap(), submitted.job);
        assert_eq!(service.cancel("job-1").unwrap().state, JobState::Cancelled);
        service.shutdown().expect("shutdown");
    }

    #[test]
    fn recovery_skips_terminal_and_duplicate_jobs_and_restarts_interrupted_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service.submit(request("job-1", "tweet-1")).unwrap();

        let recovered = service
            .recover(vec![
                JobSnapshot {
                    job_id: "job-1".into(),
                    tweet_id: "tweet-1".into(),
                    state: JobState::Interrupted,
                },
                JobSnapshot {
                    job_id: "job-2".into(),
                    tweet_id: "tweet-2".into(),
                    state: JobState::Interrupted,
                },
                JobSnapshot {
                    job_id: "job-3".into(),
                    tweet_id: "tweet-3".into(),
                    state: JobState::Complete,
                },
            ])
            .unwrap();

        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].job_id, "job-2");
        assert_eq!(recovered[0].state, JobState::Validating);
        assert_eq!(
            service.query("job-3"),
            Err(ExecutorError::UnknownJob("job-3".into()))
        );
    }

    #[test]
    fn persisted_submit_reuses_database_like_active_job_semantics() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();

        let first = service
            .submit_persisted(&mut persistence, request("job-1", "tweet-1"))
            .unwrap();
        let duplicate = service
            .submit_persisted(&mut persistence, request("job-2", "tweet-1"))
            .unwrap();

        assert!(first.created);
        assert!(!duplicate.created);
        assert_eq!(duplicate.job.job_id, "job-1");
        assert_eq!(
            service.query_persisted(&persistence, "job-1").unwrap(),
            first.job
        );
        assert_eq!(
            persistence.events("job-1").unwrap(),
            vec![
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
                ExecutorEvent::Reused {
                    job_id: "job-1".into(),
                },
            ]
        );
    }

    #[test]
    fn execute_persisted_records_successful_lifecycle_and_completion_order() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-execute", "tweet-execute"))
            .unwrap();
        let mut execution = FakeJobExecution::success(3, "archives/tweet-execute");

        let completed = service
            .execute_persisted(&mut persistence, &mut execution, "job-execute")
            .unwrap();

        assert_eq!(completed.state, JobState::Complete);
        assert_eq!(execution.calls, vec!["job-execute"]);
        assert_eq!(
            persistence.events("job-execute").unwrap(),
            vec![
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Validating,
                },
                ExecutorEvent::StateChanged {
                    from: JobState::Validating,
                    to: JobState::MetadataReady,
                },
                ExecutorEvent::StateChanged {
                    from: JobState::MetadataReady,
                    to: JobState::Downloading,
                },
                ExecutorEvent::StateChanged {
                    from: JobState::Downloading,
                    to: JobState::Downloaded,
                },
                ExecutorEvent::DownloadCompleted { files_copied: 3 },
                ExecutorEvent::Completed {
                    archive_directory: "archives/tweet-execute".into(),
                },
            ]
        );
    }

    #[test]
    fn execute_persisted_records_failure_without_polluting_other_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-failed", "tweet-failed"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-other", "tweet-other"))
            .unwrap();
        let mut execution = FakeJobExecution::failure("AUTH_REQUIRED", "login required");

        let failed = service
            .execute_persisted(&mut persistence, &mut execution, "job-failed")
            .unwrap();

        assert_eq!(failed.state, JobState::Failed);
        assert_eq!(execution.calls, vec!["job-failed"]);
        assert_eq!(
            persistence.snapshot("job-other").unwrap().state,
            JobState::Queued
        );
        assert!(matches!(
            persistence.events("job-failed").unwrap().last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "AUTH_REQUIRED"
        ));
    }

    #[test]
    fn stale_attempt_cannot_overwrite_a_state_written_by_a_newer_attempt() {
        let database_path = temporary_database_path("stale-attempt-fence");
        std::fs::create_dir_all(database_path.parent().expect("config dir")).expect("parent");
        let job_id = "job-stale-attempt";
        let tweet_id = "tweet-stale-attempt";

        {
            let mut persistence =
                StorageJobPersistence::open(&database_path).expect("open persistence");
            persistence
                .create_or_reuse(&v2_request(job_id, tweet_id))
                .expect("create v2 job");
        }

        // The stale attempt claims attempt 1 and advances to DOWNLOADING.
        let mut stale = StorageJobPersistence::open(&database_path).expect("open stale attempt");
        let stale_attempt = stale.begin_attempt(job_id).expect("claim stale attempt");
        assert_eq!(stale_attempt, 1);
        for (expected, next) in [
            (JobState::Queued, JobState::Validating),
            (JobState::Validating, JobState::MetadataReady),
            (JobState::MetadataReady, JobState::Downloading),
        ] {
            assert!(
                stale
                    .persist_state_for_attempt(job_id, stale_attempt, expected, next)
                    .expect("stale attempt writes while it still owns the job"),
                "stale attempt should still own the job at {expected:?} -> {next:?}"
            );
        }

        // A newer attempt claims the same Job and durably fails it.
        let mut winner = StorageJobPersistence::open(&database_path).expect("open winner attempt");
        let winner_attempt = winner.begin_attempt(job_id).expect("claim winner attempt");
        assert_eq!(winner_attempt, 2);
        winner
            .fail(job_id, "CANCELLED_BY_USER", "durably cancelled")
            .expect("durable cancellation");
        drop(winner);

        // The stale attempt must not write DOWNLOADED over the winning state.
        let advanced = stale
            .persist_state_for_attempt(
                job_id,
                stale_attempt,
                JobState::Downloading,
                JobState::Downloaded,
            )
            .expect("fenced write reports a loss instead of failing");
        assert!(!advanced, "stale attempt must lose the arbitration");
        assert_eq!(
            stale.snapshot(job_id).expect("snapshot").state,
            JobState::Failed,
            "the winning attempt's durable state must survive"
        );

        let _ = std::fs::remove_dir_all(database_path.parent().expect("root"));
    }

    /// Execution that blocks the single-active runner until `release` is set,
    /// used to deterministically saturate the runner channel.
    struct BlockingUntilReleased {
        release: Arc<AtomicBool>,
        started: Arc<AtomicBool>,
    }

    impl JobExecution for BlockingUntilReleased {
        fn execute(
            &mut self,
            _job: &JobSnapshot,
            _cancellation: &CancellationToken,
        ) -> Result<JobExecutionResult, JobExecutionError> {
            self.started.store(true, Ordering::SeqCst);
            while !self.release.load(Ordering::SeqCst) {
                thread::sleep(std::time::Duration::from_millis(2));
            }
            Ok(JobExecutionResult {
                files_copied: 1,
                archive_directory: "archives/n1-blocker".to_owned(),
                download_event_recorded: false,
                state_already_updated: false,
            })
        }
    }

    /// Panic-safety net that releases the blocker's runner slot if a later
    /// assertion panics, so the blocked thread can never hang the test binary.
    struct ReleaseOnDrop(Arc<AtomicBool>);

    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// Factory that blocks the blocker Job and fails any other Job once it is
    /// admitted, so the re-dispatch assertion does not depend on archive
    /// finalization.
    struct N1AdmissionFactory {
        release: Arc<AtomicBool>,
        started: Arc<AtomicBool>,
    }

    impl JobExecutionFactory for N1AdmissionFactory {
        fn create(
            &self,
            job_id: &str,
            _snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            if job_id == "job-blocker" {
                Ok(Box::new(BlockingUntilReleased {
                    release: self.release.clone(),
                    started: self.started.clone(),
                }))
            } else {
                Ok(Box::new(FakeJobExecution::failure(
                    "TEST_EXEC_FAIL",
                    "execution ran after admission",
                )))
            }
        }
    }

    /// B1 admission regression: a saturated runner must reject a queued Job
    /// without incrementing an attempt or persisting FAILED, and the Job must
    /// stay reschedulable once the runner frees up.
    #[test]
    fn saturated_runner_rejection_is_non_failing_and_reschedulable() {
        let release = Arc::new(AtomicBool::new(false));
        let started = Arc::new(AtomicBool::new(false));
        let factory = Arc::new(N1AdmissionFactory {
            release: release.clone(),
            started: started.clone(),
        });
        let executor = JobExecutor::with_capacity_and_factory(DEFAULT_QUEUE_CAPACITY, factory);
        let service = ArchiveApplicationService::new(&executor);

        let database_path = temporary_database_path("n1-admission");
        std::fs::create_dir_all(database_path.parent().expect("config dir")).expect("parent");
        {
            let mut setup = StorageJobPersistence::open(&database_path).expect("setup persistence");
            setup
                .create_or_reuse(&request("job-blocker", "tweet-blocker"))
                .expect("create blocker");
            setup
                .create_or_reuse(&request("job-second", "tweet-second"))
                .expect("create second");
        }

        // Occupy the single runner slot with a blocking Job on its own thread.
        let blocker_service = service.clone();
        let blocker_db = database_path.clone();
        let blocker = thread::spawn(move || {
            let mut persistence =
                StorageJobPersistence::open(&blocker_db).expect("blocker persistence");
            // The blocker only needs to occupy the runner slot; its final
            // archive result is irrelevant to the admission assertion.
            let _ = blocker_service.execute_persisted_from_factory(&mut persistence, "job-blocker");
        });
        while !started.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(2));
        }
        // From here on the blocker occupies the runner slot. If any assertion
        // below panics, this guard still releases the slot so the blocker
        // thread unblocks and the test binary cannot hang.
        let _release_guard = ReleaseOnDrop(release.clone());

        // The saturated runner must reject the second Job before any durable
        // claim: no attempt increment and no FAILED.
        let mut main_persistence =
            StorageJobPersistence::open(&database_path).expect("main persistence");
        let rejected = service.execute_persisted_from_factory(&mut main_persistence, "job-second");
        assert!(
            matches!(rejected, Err(ExecutorError::QueueFull)),
            "a saturated runner must report QueueFull, got {rejected:?}"
        );
        let second = main_persistence
            .snapshot("job-second")
            .expect("second snapshot");
        assert_eq!(
            second.state,
            JobState::Queued,
            "a QueueFull rejection must leave the Job queued, not failed"
        );
        assert_eq!(
            main_persistence
                .current_attempt("job-second")
                .expect("attempt"),
            0,
            "a QueueFull rejection must not increment the durable attempt"
        );

        // Release the blocker; the runner frees its slot. A drop guard also
        // releases it if any assertion above panics, so the blocker thread can
        // never hang the test binary.
        release.store(true, Ordering::SeqCst);
        blocker.join().expect("blocker thread");

        // The previously rejected Job is now admitted and runs (failing only
        // because the test factory returns a failure execution).
        let rerun = service.execute_persisted_from_factory(&mut main_persistence, "job-second");
        assert!(
            rerun.is_ok(),
            "the rescheduled Job must be admitted once the runner frees up, got {rerun:?}"
        );
        assert_eq!(
            main_persistence
                .current_attempt("job-second")
                .expect("attempt"),
            1,
            "an admitted Job claims exactly one attempt"
        );

        let _ = std::fs::remove_dir_all(database_path.parent().expect("root"));
    }

    #[test]
    fn execute_persisted_skips_terminal_jobs_without_calling_execution_port() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-terminal", "tweet-terminal"))
            .unwrap();
        insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let before = persistence.events("job-terminal").unwrap();
        let mut execution = FakeJobExecution::success(1, "archives/terminal");

        let result = service
            .execute_persisted(&mut persistence, &mut execution, "job-terminal")
            .unwrap();

        assert_eq!(result.state, JobState::Complete);
        assert!(execution.calls.is_empty());
        assert_eq!(persistence.events("job-terminal").unwrap(), before);
    }

    #[test]
    fn persisted_submit_marks_created_job_failed_when_executor_is_closed() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        executor.shutdown().unwrap();

        let error = service
            .submit_persisted(&mut persistence, request("job-closed", "tweet-closed"))
            .expect_err("closed executor");

        assert_eq!(error, ExecutorError::Closed);
        assert_eq!(
            persistence.snapshot("job-closed").unwrap().state,
            JobState::Failed
        );
        assert!(persistence.list_recovery_candidates().is_empty());
        assert!(matches!(
            persistence.events("job-closed").unwrap().as_slice(),
            [ExecutorEvent::DownloadFailed { error_code, .. }] if error_code == "EXECUTOR_UNAVAILABLE"
        ));
    }

    #[test]
    fn submit_and_schedule_persists_background_executor_failure() {
        let database_path = temporary_database_path("background-failure");
        std::fs::create_dir_all(database_path.parent().expect("database parent")).expect("parent");
        let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);

        let submitted = service
            .submit_and_schedule_persisted(
                &mut persistence,
                request("job-background-failure", "tweet-background-failure"),
                database_path.clone(),
            )
            .expect("submit and schedule");

        assert!(submitted.created);
        assert_eq!(submitted.job.state, JobState::Queued);

        let failed =
            wait_for_persisted_state(&database_path, "job-background-failure", JobState::Failed);
        assert_eq!(failed.tweet_id, "tweet-background-failure");

        let persisted = StorageJobPersistence::open(&database_path).expect("reopen database");
        assert!(matches!(
            persisted
                .stored_events("job-background-failure")
                .expect("events")
                .last(),
            Some((event_type, payload))
                if event_type == "DOWNLOAD_FAILED"
                    && payload.as_deref().is_some_and(|payload| {
                        payload.contains("EXECUTOR_WORKER_FAILED")
                    })
        ));

        let _ = std::fs::remove_dir_all(database_path.parent().unwrap().parent().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn persisted_scheduler_binds_attempt_before_successful_completion() {
        struct AttemptFactory;
        impl JobExecutionFactory for AttemptFactory {
            fn create(
                &self,
                _: &str,
                _: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                panic!("scheduler must use attempt-owned creation");
            }
            fn create_for_attempt(
                &self,
                identity: &AttemptIdentity,
                snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                assert_eq!(identity.job_id(), snapshot.job_id);
                assert_eq!(identity.attempt(), 1);
                assert_eq!(snapshot.state, JobState::Downloading);
                Ok(Box::new(FakeJobExecution::success(1, "Tweets/scheduled")))
            }
        }
        let database_path = temporary_database_path("scheduled-success");
        std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
        let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
        let executor = JobExecutor::new();
        let mut handle = executor.handle();
        handle.factory = Arc::new(AttemptFactory);
        let service = ArchiveApplicationService {
            executor: handle,
            v2_enrollment: false,
        };
        service
            .submit_and_schedule_persisted(
                &mut persistence,
                request("job-scheduled", "tweet-scheduled"),
                database_path.clone(),
            )
            .expect("schedule");
        wait_for_persisted_state(&database_path, "job-scheduled", JobState::Complete);
        assert_eq!(persistence.current_attempt("job-scheduled").unwrap(), 1);
        let events = persistence.stored_events("job-scheduled").unwrap();
        for expected in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
            assert_eq!(
                events.iter().filter(|(kind, _)| kind == expected).count(),
                1
            );
        }
        executor.shutdown().unwrap();
        drop(persistence);
        let _ = std::fs::remove_dir_all(database_path.parent().unwrap().parent().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn startup_redispatch_binds_attempt_before_successful_completion() {
        struct RecoveryFactory;
        impl JobExecutionFactory for RecoveryFactory {
            fn create(
                &self,
                _: &str,
                _: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                panic!("startup recovery must use attempt-owned creation");
            }
            fn create_for_attempt(
                &self,
                identity: &AttemptIdentity,
                snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                assert_eq!(identity.job_id(), snapshot.job_id);
                assert_eq!(identity.attempt(), 1);
                Ok(Box::new(FakeJobExecution::success(1, "Tweets/recovered")))
            }
        }
        let database_path = temporary_database_path("startup-success");
        std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
        let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
        persistence
            .create_or_reuse(&request("job-recovered", "tweet-recovered"))
            .unwrap();
        let mut runtime = ExecutorRuntime::new(database_path.clone());
        runtime.executor = JobExecutor::with_capacity_and_factory(
            DEFAULT_QUEUE_CAPACITY,
            Arc::new(RecoveryFactory),
        );
        runtime.recover_startup().expect("startup redispatch");
        wait_for_persisted_state(&database_path, "job-recovered", JobState::Complete);
        assert_eq!(persistence.current_attempt("job-recovered").unwrap(), 1);
        let events = persistence.stored_events("job-recovered").unwrap();
        for expected in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
            assert_eq!(
                events.iter().filter(|(kind, _)| kind == expected).count(),
                1
            );
        }
        runtime.executor.shutdown().unwrap();
        drop(persistence);
        let _ = std::fs::remove_dir_all(database_path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn submit_and_schedule_marks_job_failed_when_database_path_is_invalid() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        let invalid_database_path = std::env::temp_dir()
            .join(format!("xarchive-missing-parent-{}", std::process::id()))
            .join("missing")
            .join("archive.sqlite3");

        let error = service
            .submit_and_schedule_persisted(
                &mut persistence,
                request("job-invalid-database", "tweet-invalid-database"),
                invalid_database_path,
            )
            .expect_err("invalid database path");

        assert!(matches!(
            error,
            ExecutorError::Execution { error_code, .. }
                if error_code == "EXECUTOR_SCHEDULE_FAILED"
        ));
        assert_eq!(
            persistence
                .snapshot("job-invalid-database")
                .expect("snapshot")
                .state,
            JobState::Failed
        );
        assert!(matches!(
            persistence.events("job-invalid-database").expect("events").last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "EXECUTOR_SCHEDULE_FAILED"
        ));
    }

    /// D0 acceptance: an InternalV1 job whose journal record is absent must
    /// fail closed on startup recovery instead of re-running the download or
    /// fabricating a completion from whatever files happen to exist.
    #[test]
    fn startup_recovery_fails_closed_when_internal_manifest_is_missing() {
        let database_path = temporary_database_path("startup-missing-manifest");
        let root = database_path.parent().unwrap().parent().unwrap().to_owned();
        std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
        let job_id = "job-startup-missing-manifest";
        let tweet_id = "900001";
        let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
        insert_v2_downloaded_job(&mut persistence, job_id, tweet_id);
        drop(persistence);

        let runtime = ExecutorRuntime::new(database_path.clone());
        runtime.recover_startup().expect("startup recovery spawns");

        let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
        assert_eq!(failed.tweet_id, tweet_id);
        let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
        assert_eq!(
            reopened
                .stored_job_summary(job_id)
                .expect("summary")
                .last_error_code
                .as_deref(),
            Some("ARCHIVE_COMMIT_RECOVERY_FAILED")
        );
        assert!(
            !reopened
                .stored_events(job_id)
                .expect("events")
                .iter()
                .any(|(event_type, _)| event_type == "JOB_COMPLETED")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// R4b-2: a malformed containment record for the crashed attempt fails
    /// the candidate closed before any recovery decision is taken.
    #[cfg(target_os = "linux")]
    #[test]
    fn startup_recovery_drains_crashed_writer_domains_before_decisions() {
        let database_path = temporary_database_path("startup-writer-drain");
        let root = database_path.parent().unwrap().parent().unwrap().to_owned();
        std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
        let job_id = "job-startup-writer-drain";
        let tweet_id = "900041";
        let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
        insert_v2_downloaded_job(&mut persistence, job_id, tweet_id);
        drop(persistence);
        let mut database = Database::open(&database_path).expect("database");
        database
            .begin_archive_attempt(job_id, "startup-recovery-test")
            .expect("begin attempt");
        drop(database);
        let staging_root = database_path.parent().unwrap().join("_staging");
        let record_root = database_path
            .parent()
            .unwrap()
            .join("tw2tg-containment-records");
        let malformed =
            xarchive_workflow::linux_containment::AttemptIdentity::new(&staging_root, job_id, 1)
                .unwrap()
                .with_writer("sidecar")
                .unwrap();
        std::fs::create_dir_all(&record_root).expect("record root");
        std::fs::write(
            malformed.record_path(&record_root),
            "not a containment record\n",
        )
        .expect("malformed record");

        let config = ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root: root.join("archive"),
            staging_root,
            database_path: database_path.clone(),
            sidecar_program: None,
            gallery_dl_program: None,
            sidecar_args: Vec::new(),
            aria2_program: None,
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        };
        let runtime = ExecutorRuntime::with_linux_containment(
            config,
            LinuxExecutionContainment {
                delegated_parent: std::path::PathBuf::from("/sys/fs/cgroup"),
                python: std::path::PathBuf::from("/usr/bin/python3"),
                launcher: std::path::PathBuf::from("/missing-launcher.py"),
            },
        );
        runtime.recover_startup().expect("startup recovery spawns");

        let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
        assert_eq!(failed.tweet_id, tweet_id);
        let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
        assert_eq!(
            reopened
                .stored_job_summary(job_id)
                .expect("summary")
                .last_error_code
                .as_deref(),
            Some("WRITER_DOMAIN_RECOVERY_FAILED")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// P5 integration: a killed startup-recovery owner leaves a recorded
    /// writer domain and a live descendant. A fresh runtime must drain that
    /// domain before making its normal recovery decision.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires systemd-run --user --property=Delegate=yes disposable cgroup"]
    fn startup_recovery_drains_live_descendant_after_owner_sigkill() {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        const WORKER_ENV: &str = "TW2TG_P5_CRASH_WORKER";
        const PARENT_ENV: &str = "TW2TG_P5_DELEGATED_PARENT";
        const ROOT_ENV: &str = "TW2TG_P5_FIXTURE_ROOT";

        if std::env::var_os(WORKER_ENV).is_some() {
            let parent = PathBuf::from(std::env::var_os(PARENT_ENV).expect("delegated parent"));
            let root = PathBuf::from(std::env::var_os(ROOT_ENV).expect("fixture root"));
            let database_path = root.join("jobs.sqlite3");
            let staging_root = root.join("staging");
            let record_root = root.join("tw2tg-containment-records");
            std::fs::create_dir_all(&staging_root).expect("staging root");
            std::fs::create_dir_all(&record_root).expect("record root");
            let identity = xarchive_workflow::linux_containment::AttemptIdentity::new(
                &staging_root,
                "job-p5-crash-owner",
                1,
            )
            .unwrap()
            .with_writer("sidecar")
            .unwrap();
            let mut owner =
                xarchive_workflow::linux_containment::LinuxContainment::create_recorded(
                    &record_root,
                    &identity,
                    &parent,
                )
                .expect("create recorded writer group");

            let marker = root.join("late-write-marker");
            owner.spawn_silent_with_environment(
                Path::new("/usr/bin/python3"),
                &Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
                Path::new("/usr/bin/python3"),
                &["-c".into(), "import os,sys,time; from pathlib import Path; Path(sys.argv[1]+'.ready').write_text(str(os.getpid())); time.sleep(8); Path(sys.argv[1]).write_text('late')".into(), marker.to_string_lossy().into_owned()],
                &[], &[],
            ).expect("launch contained late writer");
            let child_ready = marker.with_extension("ready");
            let ready_deadline = Instant::now() + Duration::from_secs(3);
            while !child_ready.exists() {
                assert!(Instant::now() < ready_deadline, "descendant must start");
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(owner.poll_exit().expect("poll descendant").is_none());

            let mut database = StorageJobPersistence::open(&database_path).expect("database");
            insert_v2_downloaded_job(&mut database, "job-p5-crash-owner", "900051");
            drop(database);
            Database::open(&database_path)
                .expect("database")
                .begin_archive_attempt("job-p5-crash-owner", "p5-crash-test")
                .expect("begin archive attempt");

            std::fs::write(
                root.join("owner.ready"),
                owner.path().as_os_str().as_encoded_bytes(),
            )
            .expect("persist readiness");
            use std::io::Write as _;
            std::io::stdout().flush().expect("flush readiness");
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }

        let membership = std::fs::read_to_string("/proc/self/cgroup").expect("cgroup membership");
        let relative = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .expect("unified cgroup membership");
        let delegated_parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
        assert!(
            delegated_parent.join("cgroup.procs").metadata().is_ok(),
            "test must run inside an explicitly delegated cgroup"
        );
        let fixture_database = temporary_database_path("p5-crash-recovery");
        let root = fixture_database.parent().unwrap().to_owned();
        std::fs::create_dir_all(&root).expect("fixture root");

        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .arg("--exact")
            .arg("executor::tests::startup_recovery_drains_live_descendant_after_owner_sigkill")
            .arg("--nocapture")
            .arg("--ignored")
            .env(WORKER_ENV, "1")
            .env(PARENT_ENV, &delegated_parent)
            .env(ROOT_ENV, &root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn crash owner worker");
        let ready_deadline = Instant::now() + Duration::from_secs(6);
        while !root.join("owner.ready").exists() {
            if let Some(status) = child.try_wait().expect("poll owner") {
                let output = child.wait_with_output().expect("owner output");
                panic!(
                    "owner exited {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            if Instant::now() >= ready_deadline {
                child.kill().expect("kill unready owner");
                let output = child.wait_with_output().expect("owner output");
                panic!(
                    "owner readiness timed out: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let group_path = PathBuf::from(std::fs::read_to_string(root.join("owner.ready")).unwrap());
        assert!(
            std::fs::read_to_string(group_path.join("cgroup.events"))
                .expect("live writer domain")
                .lines()
                .any(|line| line == "populated 1"),
            "crash fixture must have a live contained writer"
        );
        let status = Command::new("/bin/kill")
            .args(["-KILL", &child.id().to_string()])
            .status()
            .expect("SIGKILL owner");
        assert!(status.success(), "SIGKILL command succeeded");
        let killed = child.wait().expect("reap killed owner");
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(killed.signal(), Some(9), "owner must die from SIGKILL");
        assert!(
            std::fs::read_to_string(group_path.join("cgroup.events"))
                .expect("orphan writer domain")
                .lines()
                .any(|line| line == "populated 1"),
            "writer must survive owner death until successor drain"
        );

        let database_path = root.join("jobs.sqlite3");
        let staging_root = root.join("staging");
        let config = ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root: root.join("archive"),
            staging_root: staging_root.clone(),
            database_path: database_path.clone(),
            sidecar_program: None,
            gallery_dl_program: None,
            sidecar_args: Vec::new(),
            aria2_program: None,
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        };
        let runtime = ExecutorRuntime::with_linux_containment(
            config,
            LinuxExecutionContainment {
                delegated_parent,
                python: PathBuf::from("/usr/bin/python3"),
                launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
            },
        );
        runtime
            .recover_startup()
            .expect("startup recovery dispatch");

        let deadline = Instant::now() + Duration::from_secs(3);
        while group_path.exists() {
            assert!(
                Instant::now() < deadline,
                "recovery must remove drained group"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_secs(9));
        assert!(
            !root.join("late-write-marker").exists(),
            "descendant must not write after successor recovery"
        );
        let failed =
            wait_for_persisted_state(&database_path, "job-p5-crash-owner", JobState::Failed);
        assert_eq!(failed.tweet_id, "900051");
        assert!(
            !group_path.exists(),
            "recorded owner group removed after drain"
        );
        runtime.recover_startup().expect("repeat startup recovery");
        let replayed =
            wait_for_persisted_state(&database_path, "job-p5-crash-owner", JobState::Failed);
        assert_eq!(replayed.tweet_id, failed.tweet_id);
        assert!(!root.join("late-write-marker").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// D0 acceptance: a non-NULL, unsupported recovery contract version fails
    /// explicitly before any state re-execution is attempted.
    #[test]
    fn startup_recovery_fails_closed_on_unknown_contract_version() {
        let database_path = temporary_database_path("startup-unknown-version");
        let root = database_path.parent().unwrap().parent().unwrap().to_owned();
        std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
        let job_id = "job-startup-unknown-version";
        let tweet_id = "900002";
        let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
        insert_v2_downloaded_job(&mut persistence, job_id, tweet_id);
        drop(persistence);

        let database = Database::open(&database_path).expect("database");
        database
            .execute_batch(&format!(
                "UPDATE jobs SET recovery_contract_version = 99 WHERE id = '{job_id}'"
            ))
            .expect("install unknown contract version");
        drop(database);

        let runtime = ExecutorRuntime::new(database_path.clone());
        runtime.recover_startup().expect("startup recovery spawns");

        let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
        assert_eq!(failed.tweet_id, tweet_id);
        let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
        assert_eq!(
            reopened
                .stored_job_summary(job_id)
                .expect("summary")
                .last_error_code
                .as_deref(),
            Some("ARCHIVE_RECOVERY_UNSUPPORTED_VERSION")
        );
        assert!(
            !reopened
                .stored_events(job_id)
                .expect("events")
                .iter()
                .any(|(event_type, _)| event_type == "JOB_COMPLETED")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn production_factory_carries_v2_output_snapshot_into_archive_execution() {
        let decoded = decode_execution_spec(
            2,
            &serde_json::to_string(&ExecutionSpecV2 {
                request: archive_request("123"),
                output_settings: OutputSettings {
                    naming_mode: NamingMode::Template,
                    filename_template: "{tweet_id}_{index}".into(),
                    export_json: false,
                    export_text: true,
                },
            })
            .unwrap(),
        )
        .expect("v2 spec");
        assert_eq!(
            decoded.output_settings.filename_template,
            "{tweet_id}_{index}"
        );
        assert!(!decoded.output_settings.export_json);
        assert_eq!(decoded.output_settings.naming_mode, NamingMode::Template);
    }

    #[test]
    fn startup_v2_missing_journal_remains_fail_closed() {
        let path = temporary_database_path("startup-v2-gate");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut persistence = StorageJobPersistence::open(&path).unwrap();
        insert_v2_downloaded_job(&mut persistence, "v2-gated", "900003");
        let database = Database::open(&path).unwrap();
        database
            .execute_batch("UPDATE jobs SET recovery_contract_version = 2 WHERE id = 'v2-gated'")
            .unwrap();
        let runtime = ExecutorRuntime::new(path.clone());
        let before = persistence.stored_events("v2-gated").unwrap();
        for _ in 0..2 {
            let results = runtime
                .recover_startup_with_v2(true)
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].0, "v2-gated");
            assert!(results[0].1.is_err());
            assert_eq!(
                persistence.snapshot("v2-gated").unwrap().state,
                JobState::Downloaded
            );
            assert_eq!(persistence.stored_events("v2-gated").unwrap(), before);
        }
        runtime.recover_startup().unwrap();
        wait_for_persisted_state(&path, "v2-gated", JobState::Failed);
        assert_eq!(
            persistence
                .stored_job_summary("v2-gated")
                .unwrap()
                .last_error_code
                .as_deref(),
            Some("ARCHIVE_COMMIT_RECOVERY_FAILED")
        );
        assert!(
            !persistence
                .stored_events("v2-gated")
                .unwrap()
                .iter()
                .any(|(kind, _)| kind == "JOB_COMPLETED")
        );
        runtime.executor.shutdown().unwrap();
        drop(database);
        drop(persistence);
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_launch_descriptor_is_explicit_bounded_and_fail_closed() {
        let parse = |arguments: Vec<std::ffi::OsString>| {
            LinuxExecutionContainment::from_launch_arguments(arguments)
        };
        assert!(parse(vec![]).unwrap().is_none());
        assert!(parse(vec!["--linux-execution-descriptor".into()]).is_err());
        assert!(
            parse(vec![
                "--linux-execution-descriptor".into(),
                "relative.json".into()
            ])
            .is_err()
        );
        let path = temporary_database_path("launch-descriptor");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let link = path.with_extension("symlink");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(
            parse(vec![
                "--linux-execution-descriptor".into(),
                link.into_os_string()
            ])
            .is_err()
        );
        for bytes in [b"{".to_vec(), vec![b' '; 16 * 1024 + 1], br#"{"schema_version":2,"delegated_parent":"/sys/fs/cgroup/x","python":"/usr/bin/python3","launcher":"/missing"}"#.to_vec()] {
            std::fs::write(&path, bytes).unwrap();
            assert!(parse(vec!["--linux-execution-descriptor".into(), path.clone().into_os_string()]).is_err());
        }
        for bytes in [
            br#"{"schema_version":1,"delegated_parent":"/sys/fs/cgroup/x","python":"/usr/bin/python3","launcher":"/missing","unknown":true}"#.as_slice(),
            br#"{"schema_version":1,"schema_version":1,"delegated_parent":"/sys/fs/cgroup/x","python":"/usr/bin/python3","launcher":"/missing"}"#.as_slice(),
            br#"{"schema_version":1,"delegated_parent":"/sys/fs/cgroup/x","python":"relative","launcher":"/missing"}"#.as_slice(),
            br#"{"schema_version":1}"#.as_slice(),
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(parse(vec!["--linux-execution-descriptor".into(), path.clone().into_os_string()]).is_err());
        }
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    /// A legacy job without an execution spec and without archive recovery
    /// facts is classified as incomplete, not as a new-contract spec failure.
    #[test]
    fn startup_recovery_classifies_legacy_without_recovery_facts_as_incomplete() {
        let database_path = temporary_database_path("startup-missing-spec");
        let root = database_path.parent().unwrap().parent().unwrap().to_owned();
        std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
        let job_id = "job-startup-missing-spec";
        let tweet_id = "900003";

        let database = Database::open(&database_path).expect("database");
        let tweet_row_id = database
            .insert_tweet(
                tweet_id,
                &format!("https://x.com/test/status/{tweet_id}"),
                "post",
                "",
                "now",
            )
            .expect("tweet");
        database
            .create_archive_job(job_id, tweet_row_id, "now")
            .expect("job without execution spec");
        drop(database);

        let runtime = ExecutorRuntime::new(database_path.clone());
        runtime.recover_startup().expect("startup recovery spawns");

        let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
        assert_eq!(failed.tweet_id, tweet_id);
        let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
        assert_eq!(
            reopened
                .stored_job_summary(job_id)
                .expect("summary")
                .last_error_code
                .as_deref(),
            Some("ARCHIVE_RECOVERY_LEGACY_INCOMPLETE")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn startup_recovery_classifies_legacy_without_execution_spec_consistently() {
        let database_path = temporary_database_path("startup-legacy-missing-spec");
        let root = database_path.parent().unwrap().parent().unwrap().to_owned();
        std::fs::create_dir_all(database_path.parent().unwrap()).expect("parent");
        let job_id = "job-startup-legacy-no-spec";
        let tweet_id = "900005";
        let database = Database::open(&database_path).expect("database");
        let tweet_row_id = database
            .insert_tweet(
                tweet_id,
                &format!("https://x.com/test/status/{tweet_id}"),
                "post",
                "",
                "now",
            )
            .expect("tweet");
        database
            .create_archive_job(job_id, tweet_row_id, "now")
            .expect("legacy job without execution spec");
        drop(database);

        let runtime = ExecutorRuntime::new(database_path.clone());
        runtime.recover_startup().expect("startup recovery spawns");
        let failed = wait_for_persisted_state(&database_path, job_id, JobState::Failed);
        assert_eq!(failed.tweet_id, tweet_id);
        let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
        assert_eq!(
            reopened
                .stored_job_summary(job_id)
                .expect("summary")
                .last_error_code
                .as_deref(),
            Some("ARCHIVE_RECOVERY_LEGACY_INCOMPLETE")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// The happy path through the production startup boundary: a valid
    /// InternalV1 journal + on-disk manifest reconstructs the archive, completes
    /// the job exactly once, and a repeated startup leaves it untouched.
    #[test]
    fn startup_recovery_completes_valid_internal_archive_and_repeats_idempotently() {
        let database_path = temporary_database_path("startup-valid-manifest");
        let root = database_path.parent().unwrap().parent().unwrap().to_owned();
        let config_dir = database_path.parent().unwrap().to_owned();
        std::fs::create_dir_all(&config_dir).expect("parent");
        let job_id = "job-startup-valid";
        let tweet_id = "900004";
        let final_directory = format!("Tweets/{tweet_id}");

        let mut database = Database::open(&database_path).expect("database");
        let tweet_row_id = database
            .insert_tweet(
                tweet_id,
                &format!("https://x.com/test/status/{tweet_id}"),
                "post",
                "",
                "now",
            )
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                job_id,
                tweet_row_id,
                2,
                &format!("test-request-{job_id}"),
                &serde_json::to_string(&ExecutionSpecV2 {
                    request: archive_request(tweet_id),
                    output_settings: OutputSettings::default(),
                })
                .expect("v2 spec"),
                Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job with contract");
        database
            .begin_archive_attempt(job_id, "now")
            .expect("attempt");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
            JobState::Downloaded,
        ] {
            database
                .transition_job(job_id, state, "now")
                .expect("advance state");
        }

        let files =
            FileStore::with_staging_root(&root, config_dir.join("_staging")).expect("file store");
        let staging = files.staging_dir(job_id).expect("staging");
        std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
        let sha256 = FileStore::sha256(staging.join("01.jpg")).expect("hash");
        let metadata = xarchive_core::ArchiveMetadata {
            schema_version: 1,
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/test/status/{tweet_id}"),
            tweet_type: "post".to_owned(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
            },
            created_at: None,
            text: "startup recovery".to_owned(),
            media: vec![xarchive_core::ArchiveMedia {
                index: 1,
                media_id: Some("m1".to_owned()),
                media_type: "photo".to_owned(),
                file: "01.jpg".to_owned(),
                mime_type: Some("image/jpeg".to_owned()),
                size_bytes: 3,
                sha256,
            }],
            archived_at: "now".to_owned(),
            reply_to: None,
            quoted_tweet: None,
        };
        let manifest = xarchive_storage::ArchiveRecoveryManifest::from_metadata(
            job_id,
            1,
            tweet_row_id,
            Path::new(&final_directory),
            &metadata,
            None,
        )
        .expect("manifest");
        std::fs::write(
            staging.join(".xarchive-recovery.json"),
            serde_json::to_vec(&manifest).expect("manifest json"),
        )
        .expect("manifest file");
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("journal");
        drop(database);

        let runtime = ExecutorRuntime::new(database_path.clone());
        runtime.recover_startup().expect("first startup recovery");
        let completed = wait_for_persisted_state(&database_path, job_id, JobState::Complete);
        assert_eq!(completed.tweet_id, tweet_id);
        assert!(root.join(format!("{final_directory}/01.jpg")).is_file());
        assert!(!config_dir.join("_staging").join(job_id).exists());

        let database = Database::open(&database_path).expect("reopen");
        let journal = database
            .archive_recovery_journal(job_id)
            .expect("journal")
            .expect("journal row");
        assert_eq!(
            journal.phase,
            xarchive_storage::ArchiveRecoveryPhase::Committed
        );
        drop(database);

        // Repeated startup: terminal jobs are skipped, completion stays single-shot.
        runtime.recover_startup().expect("second startup recovery");
        thread::sleep(std::time::Duration::from_millis(300));
        let reopened = StorageJobPersistence::open(&database_path).expect("reopen");
        assert_eq!(
            reopened.snapshot(job_id).expect("snapshot").state,
            JobState::Complete
        );
        let completion_events = reopened
            .stored_events(job_id)
            .expect("events")
            .iter()
            .filter(|(event_type, _)| event_type == "JOB_COMPLETED")
            .count();
        assert_eq!(completion_events, 1, "completion must stay single-shot");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn archive_submission_adapter_preserves_sync_job_identity_and_reuse_contract() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let adapter = ArchiveJobSubmissionAdapter;
        let mut persistence = InMemoryJobPersistence::default();
        let request = archive_request("123");

        let first = adapter
            .submit(&service, &mut persistence, &request, "2026-09-13T00:00:00Z")
            .expect("submit");
        let duplicate = adapter
            .submit(&service, &mut persistence, &request, "2026-09-13T00:00:01Z")
            .expect("reuse");

        assert!(first.created);
        assert_eq!(first.job.job_id, "archive-123-2026-09-13T00:00:00Z");
        assert!(!duplicate.created);
        assert_eq!(duplicate.job, first.job);
        assert_eq!(
            adapter
                .query(&service, &persistence, &first.job.job_id)
                .expect("query"),
            first.job
        );
    }

    #[test]
    fn archive_submission_adapter_rejects_invalid_browser_requests_before_submit() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let adapter = ArchiveJobSubmissionAdapter;
        let mut persistence = InMemoryJobPersistence::default();
        let mut request = archive_request("123");
        request.tweet.tweet_id = "not-a-tweet-id".to_owned();
        request.tweet.url = "https://example.com/not-x".to_owned();

        let error = adapter
            .submit(&service, &mut persistence, &request, "2026-09-13T00:00:00Z")
            .expect_err("invalid request");
        assert!(
            matches!(error, ExecutorError::Persistence(message) if message.contains("tweet_id"))
        );
        assert!(persistence.list_recovery_candidates().is_empty());
    }

    #[test]
    fn persisted_recovery_updates_only_recoverable_candidates() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-2", "tweet-2"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-2".into(),
                tweet_id: "tweet-2".into(),
                state: JobState::Interrupted,
            })
            .unwrap();

        let recovered = service.recover_persisted(&mut persistence).unwrap();
        assert_eq!(recovered.len(), 2);
        assert!(
            recovered
                .iter()
                .all(|job| job.state == JobState::Validating)
        );
        assert_eq!(
            service
                .query_persisted(&persistence, "job-2")
                .unwrap()
                .state,
            JobState::Validating
        );
        assert_eq!(
            persistence.events("job-2").unwrap(),
            vec![ExecutorEvent::Recovered {
                from: JobState::Interrupted,
                to: JobState::Validating,
            }]
        );
    }

    #[test]
    fn persisted_cancel_transitions_queued_to_cancelled_and_is_idempotent() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        executor
            .handle()
            .submit(request("job-1", "tweet-1"))
            .unwrap();

        let cancelled = service.cancel_persisted(&mut persistence, "job-1").unwrap();
        assert_eq!(cancelled.state, JobState::Cancelled);
        assert_eq!(persistence.snapshot("job-1").unwrap(), cancelled);
        assert_eq!(
            persistence.events("job-1").unwrap(),
            vec![ExecutorEvent::StateChanged {
                from: JobState::Queued,
                to: JobState::Cancelled,
            }]
        );

        assert_eq!(
            service.cancel_persisted(&mut persistence, "job-1").unwrap(),
            cancelled
        );
        assert_eq!(persistence.events("job-1").unwrap().len(), 1);
    }

    #[test]
    fn persisted_cancel_does_not_change_terminal_job_or_record_an_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let before = persistence.events("job-1").unwrap();

        let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
        assert_eq!(result.state, JobState::Complete);
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    /// Test persistence that reports a durable archive commit boundary, either
    /// at the pre-check (like a already-PREPARED journal observed before the
    /// writer stops) or only at decision time (like a prepare landing between
    /// the check and the decision).
    #[derive(Default)]
    struct CommitBoundaryPersistence {
        inner: InMemoryJobPersistence,
        report_at_precheck: bool,
        report_at_decision: bool,
    }

    #[cfg(unix)]
    #[test]
    fn session_execution_error_preserves_prepared_job_for_startup_replay() {
        let path = temporary_database_path("prepared-error");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let runtime = ExecutorRuntime::new(path.clone());
        let mut persistence = CommitBoundaryPersistence {
            report_at_precheck: true,
            ..Default::default()
        };
        persistence
            .create_or_reuse(&request("job-prepared", "123"))
            .unwrap();
        let coordinator = ProductionCoordinator::open(&path).unwrap();
        let service = runtime.service();
        let mut boundary = SessionExecutionBoundary::new(
            service,
            persistence,
            coordinator,
            path.parent().unwrap().to_owned(),
        );
        // The unavailable production factory fails after the durable claim.
        assert!(boundary.execute_session("job-prepared").is_err());
        assert_eq!(
            boundary.persistence.snapshot("job-prepared").unwrap().state,
            JobState::Downloading
        );
        assert!(
            !boundary
                .persistence
                .events("job-prepared")
                .unwrap()
                .iter()
                .any(|event| matches!(event, ExecutorEvent::DownloadFailed { .. }))
        );
        drop(boundary);
        drop(runtime);
        std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn maintenance_blocks_production_and_startup_recovery_before_mutation() {
        let path = temporary_database_path("maintenance-admission");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let runtime = ExecutorRuntime::new(path.clone());
        let coordinator = ProductionCoordinator::open(&path).unwrap();
        let locks = coordinator.inner.lock_directory().to_owned();
        drop(coordinator);
        let maintenance =
            xarchive_workflow::maintenance::WorkspaceGuard::drain(&locks, || false).unwrap();
        assert!(runtime.production_session_boundary().is_err());
        let results = runtime.recover_startup().unwrap().join().unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.is_err());
        drop(maintenance);
        assert!(runtime.production_session_boundary().is_ok());
        drop(runtime);
        std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
    }

    /// A live production session (holding an active shared guard) must block the
    /// maintenance drain until it finalizes; only then does drain complete. This
    /// reconciles the apply path against a real session, not a raw guard.
    #[cfg(target_os = "linux")]
    #[test]
    fn live_production_session_blocks_maintenance_drain_until_finalize() {
        let path = temporary_database_path("live-writer-drain");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let session = ProductionCoordinator::open(&path).unwrap();
        let locks = session.inner.lock_directory().to_owned();

        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let drain_locks = locks.clone();
        let drain_thread = std::thread::spawn(move || {
            // Signal that drain is about to block, then wait (cancellable) up to
            // a generous deadline for the live session to release.
            ready_tx.send(()).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let guard = xarchive_workflow::maintenance::WorkspaceGuard::drain(&drain_locks, || {
                std::time::Instant::now() >= deadline
            });
            done_tx.send(guard.is_ok()).unwrap();
        });
        ready_rx.recv().unwrap();
        // Give the drain thread time to close admission and start waiting.
        std::thread::sleep(std::time::Duration::from_millis(200));
        // The live session still holds active shared, so drain cannot have completed.
        assert!(
            done_rx.try_recv().is_err(),
            "drain must not complete while a live session holds active shared"
        );
        // New production admission is now refused because the turnstile is held.
        assert!(
            ProductionCoordinator::open(&path).is_err(),
            "turnstile must refuse new admission during maintenance drain"
        );
        // Finalize the live session: release its active shared guard.
        drop(session);
        // Drain now completes within the deadline.
        let completed = done_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("drain must complete after the live session finalizes");
        assert!(
            completed,
            "drain should acquire active exclusive once drained"
        );
        drain_thread.join().unwrap();
        std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
    }

    impl JobPersistence for CommitBoundaryPersistence {
        fn create_or_reuse(
            &mut self,
            request: &ArchiveJobRequest,
        ) -> Result<SubmitResult, ExecutorError> {
            self.inner.create_or_reuse(request)
        }
        fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
            self.inner.snapshot(job_id)
        }
        fn list_recovery_candidates(&self) -> Vec<JobSnapshot> {
            self.inner.list_recovery_candidates()
        }
        fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
            self.inner.persist_state(snapshot)
        }
        fn fail(
            &mut self,
            job_id: &str,
            error_code: &str,
            error_message: &str,
        ) -> Result<JobSnapshot, ExecutorError> {
            self.inner.fail(job_id, error_code, error_message)
        }
        fn record_event(
            &mut self,
            job_id: &str,
            event: ExecutorEvent,
        ) -> Result<(), ExecutorError> {
            self.inner.record_event(job_id, event)
        }
        fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
            self.inner.events(job_id)
        }
        fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
            self.inner.begin_attempt(job_id)
        }
        fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
            self.inner.current_attempt(job_id)
        }
        fn claim_attempt(
            &mut self,
            job_id: &str,
            expected_attempt: u32,
            from: JobState,
            to: JobState,
        ) -> Result<Option<u32>, ExecutorError> {
            self.inner.claim_attempt(job_id, expected_attempt, from, to)
        }
        fn record_claimed_attempt_state(
            &mut self,
            job_id: &str,
            from: JobState,
            to: JobState,
        ) -> Result<(), ExecutorError> {
            self.inner.record_claimed_attempt_state(job_id, from, to)
        }
        fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
            self.inner.attempt_is_current(job_id, attempt)
        }
        fn complete_terminal_for_attempt(
            &mut self,
            job_id: &str,
            attempt: u32,
            archive_directory: &str,
            files_copied: u32,
        ) -> Result<JobSnapshot, ExecutorError> {
            self.inner.complete_terminal_for_attempt(
                job_id,
                attempt,
                archive_directory,
                files_copied,
            )
        }
        fn archive_commit_started(&self, _job_id: &str) -> Result<bool, ExecutorError> {
            Ok(self.report_at_precheck)
        }
        fn persist_cancellation(
            &mut self,
            job_id: &str,
        ) -> Result<CancelArbitration, ExecutorError> {
            if self.report_at_decision {
                return Ok(CancelArbitration::CommitWins);
            }
            self.inner.persist_cancellation(job_id)
        }
    }

    /// When a durable archive commit boundary exists before cancellation is
    /// attempted, the commit wins: the writer keeps running, no cancelled
    /// state is persisted, and no state-change event is recorded.
    #[test]
    fn persisted_cancel_refuses_before_stopping_when_commit_started() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = CommitBoundaryPersistence {
            inner: InMemoryJobPersistence::default(),
            report_at_precheck: true,
            report_at_decision: false,
        };
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        executor
            .handle()
            .submit(request("job-1", "tweet-1"))
            .unwrap();

        let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
        assert_eq!(result.state, JobState::Queued);
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued
        );
        assert!(
            persistence
                .events("job-1")
                .unwrap()
                .iter()
                .all(|event| !matches!(event, ExecutorEvent::StateChanged { .. })),
            "a refused cancellation records no state change"
        );
        // The writer was never stopped.
        assert_eq!(
            executor.handle().snapshot("job-1").unwrap().state,
            JobState::Queued
        );
    }

    /// A prepare that lands between the pre-check and the decision still wins:
    /// the decision path reports CommitWins and no cancelled state or event is
    /// written over the journal.
    #[test]
    fn persisted_cancel_commit_wins_race_and_writes_nothing() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = CommitBoundaryPersistence {
            inner: InMemoryJobPersistence::default(),
            report_at_precheck: false,
            report_at_decision: true,
        };
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        executor
            .handle()
            .submit(request("job-1", "tweet-1"))
            .unwrap();

        let result = service.cancel_persisted(&mut persistence, "job-1").unwrap();
        assert_eq!(result.state, JobState::Queued);
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued,
            "the durable job stays non-cancelled when the commit wins"
        );
        assert!(
            persistence
                .events("job-1")
                .unwrap()
                .iter()
                .all(|event| !matches!(event, ExecutorEvent::StateChanged { .. })),
            "no cancelled state-change event is written when the commit wins"
        );
    }

    #[test]
    fn persisted_shutdown_interrupts_active_jobs_once_and_skips_terminal_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-active", "tweet-active"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-terminal", "tweet-terminal"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let terminal_events = persistence.events("job-terminal").unwrap();

        service.shutdown_persisted(&mut persistence).unwrap();

        assert_eq!(
            persistence.snapshot("job-active").unwrap().state,
            JobState::Interrupted
        );
        assert_eq!(
            persistence.events("job-active").unwrap(),
            vec![
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Interrupted,
                },
                ExecutorEvent::ShutdownInterrupted {
                    from: JobState::Queued,
                    to: JobState::Interrupted,
                },
            ]
        );
        assert_eq!(persistence.events("job-terminal").unwrap(), terminal_events);
    }

    #[test]
    fn interrupt_persisted_is_separate_from_worker_shutdown() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        service
            .submit(request("job-active", "tweet-active"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-active", "tweet-active"))
            .unwrap();

        service.interrupt_persisted(&mut persistence).unwrap();

        assert_eq!(
            persistence.snapshot("job-active").unwrap().state,
            JobState::Interrupted
        );
        assert!(service.query("job-active").is_ok());
        service.shutdown().unwrap();
    }

    #[test]
    fn persisted_completion_requires_downloaded_state_and_records_completion_once() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let completed = service
            .complete_persisted(&mut persistence, "job-1", "archives/123")
            .unwrap();
        assert_eq!(completed.state, JobState::Complete);
        assert_eq!(persistence.snapshot("job-1").unwrap(), completed);
        assert_eq!(
            persistence.events("job-1").unwrap().last(),
            Some(&ExecutorEvent::Completed {
                archive_directory: "archives/123".into(),
            })
        );

        let events_before_retry = persistence.events("job-1").unwrap();
        assert_eq!(
            service
                .complete_persisted(&mut persistence, "job-1", "archives/123")
                .unwrap(),
            completed
        );
        assert_eq!(persistence.events("job-1").unwrap(), events_before_retry);
    }

    #[test]
    fn persisted_completion_rejects_queued_job_without_writing_state_or_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        let before = persistence.events("job-1").unwrap();

        assert_eq!(
            service.complete_persisted(&mut persistence, "job-1", "archives/123"),
            Err(ExecutorError::InvalidTransition {
                from: JobState::Queued,
                to: JobState::Complete,
            })
        );
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued
        );
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    #[test]
    fn commit_recovery_decision_distinguishes_resume_complete_failure_and_skip() {
        let resume = CommitRecoveryFacts {
            job_id: "job-resume".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        };
        assert_eq!(
            resume.decide("archives/123"),
            CommitRecoveryDecision::ResumeStaging
        );

        let complete = CommitRecoveryFacts {
            job_id: "job-complete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert_eq!(
            complete.decide("archives/123"),
            CommitRecoveryDecision::Complete {
                archive_directory: "archives/123".into(),
            }
        );

        let incomplete = CommitRecoveryFacts {
            job_id: "job-incomplete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert!(matches!(
            incomplete.decide("archives/123"),
            CommitRecoveryDecision::MarkFailed { error_code, .. }
                if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
        ));

        let already_complete = CommitRecoveryFacts {
            job_id: "job-done".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert_eq!(
            already_complete.decide("archives/123"),
            CommitRecoveryDecision::Skip
        );
    }

    #[test]
    fn commit_recovery_decision_detects_missing_archive_for_complete_job() {
        let facts = CommitRecoveryFacts {
            job_id: "job-done".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        };
        assert!(matches!(
            facts.decide("archives/123"),
            CommitRecoveryDecision::MarkFailed { error_code, error_message }
                if error_code == "ARCHIVE_COMMIT_MISSING"
                    && error_message.contains("job-done")
        ));
    }

    #[test]
    fn commit_recovery_does_not_rewrite_unrelated_job_states() {
        let facts = CommitRecoveryFacts {
            job_id: "job-active".into(),
            state: JobState::Downloading,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Present,
        };
        assert_eq!(
            facts.decide("archives/123"),
            CommitRecoveryDecision::NotApplicable
        );
    }

    #[test]
    fn apply_commit_recovery_resumes_staging_without_fabricating_completion() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        let before = persistence.events("job-1").unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Downloaded);
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    #[test]
    fn apply_commit_recovery_completes_existing_final_directory() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Complete);
        assert!(matches!(
            persistence.events("job-1").unwrap().last(),
            Some(ExecutorEvent::Completed { archive_directory })
                if archive_directory == "archives/123"
        ));
    }

    #[test]
    fn apply_commit_recovery_marks_missing_commit_as_failed_and_records_error() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Failed);
        let summary = persistence.stored_job_summary("job-1").unwrap();
        assert_eq!(
            summary.last_error_code.as_deref(),
            Some("ARCHIVE_COMMIT_INCOMPLETE")
        );
        assert!(matches!(
            persistence.events("job-1").unwrap().last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
        ));
    }

    #[test]
    fn apply_commit_recovery_skips_complete_job_without_duplicate_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Complete,
            })
            .unwrap();
        let before = persistence.events("job-1").unwrap();
        let facts = CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        };

        let result = service
            .apply_commit_recovery(&mut persistence, &facts, "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Complete);
        assert_eq!(persistence.events("job-1").unwrap(), before);
    }

    #[test]
    fn recover_commit_persisted_uses_injected_directory_facts_and_matches_snapshot() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let result = service
            .recover_commit_persisted(&mut persistence, &provider, "job-1", "archives/123")
            .unwrap();
        assert_eq!(result.state, JobState::Complete);
    }

    #[test]
    fn recover_commit_persisted_rejects_facts_that_do_not_match_snapshot() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-1".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let error = service
            .recover_commit_persisted(&mut persistence, &provider, "job-1", "archives/123")
            .expect_err("mismatched facts");
        assert!(
            matches!(error, ExecutorError::Persistence(message) if message.contains("do not match"))
        );
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued
        );
    }

    #[test]
    fn storage_recovery_facts_preserve_job_summary_state_and_injected_directories() {
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let facts = persistence
            .recovery_facts(
                "job-1",
                CommitRecoveryDirectory::Present,
                CommitRecoveryDirectory::Missing,
            )
            .unwrap();
        assert_eq!(facts.job_id, "job-1");
        assert_eq!(facts.state, JobState::Downloaded);
        assert_eq!(facts.final_directory, CommitRecoveryDirectory::Present);
        assert_eq!(facts.staging_directory, CommitRecoveryDirectory::Missing);
    }

    #[test]
    fn recover_commits_persisted_keeps_results_sorted_and_isolates_fact_errors() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        for job_id in ["job-b", "job-a"] {
            persistence
                .create_or_reuse(&request(job_id, &format!("tweet-{job_id}")))
                .unwrap();
            persistence
                .persist_state(&JobSnapshot {
                    job_id: job_id.into(),
                    tweet_id: format!("tweet-{job_id}"),
                    state: JobState::Downloaded,
                })
                .unwrap();
        }

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-a".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert_eq!(
            results
                .iter()
                .map(|result| result.job_id.as_str())
                .collect::<Vec<_>>(),
            vec!["job-a", "job-b"]
        );
        assert_eq!(
            results[0].result.as_ref().unwrap().state,
            JobState::Complete
        );
        assert!(matches!(
            &results[1].result,
            Err(ExecutorError::UnknownJob(job_id)) if job_id == "job-b"
        ));
        assert_eq!(
            persistence.snapshot("job-a").unwrap().state,
            JobState::Complete
        );
        assert_eq!(
            persistence.snapshot("job-b").unwrap().state,
            JobState::Downloaded
        );
    }

    #[test]
    fn recover_commits_persisted_skips_terminal_jobs_from_candidate_scan() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-complete", "tweet-complete"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Downloaded,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-complete".into(),
                tweet_id: "tweet-complete".into(),
                state: JobState::Complete,
            })
            .unwrap();

        let provider = InMemoryCommitRecoveryFactsProvider::default();
        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert!(results.is_empty());
    }

    #[test]
    fn recover_commits_persisted_mixes_actions_and_preserves_each_job_event_order() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        insert_downloaded_job(&mut persistence, "job-resume", "tweet-resume");
        insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
        insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
        insert_downloaded_job(&mut persistence, "job-missing", "tweet-missing");
        insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-resume".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-complete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-failed".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-terminal".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert_eq!(
            results
                .iter()
                .map(|result| result.job_id.as_str())
                .collect::<Vec<_>>(),
            vec!["job-complete", "job-failed", "job-missing", "job-resume"]
        );
        assert_eq!(
            results[0].result.as_ref().unwrap().state,
            JobState::Complete
        );
        assert_eq!(results[1].result.as_ref().unwrap().state, JobState::Failed);
        assert!(matches!(
            &results[2].result,
            Err(ExecutorError::UnknownJob(job_id)) if job_id == "job-missing"
        ));
        assert_eq!(
            results[3].result.as_ref().unwrap().state,
            JobState::Downloaded
        );

        assert!(matches!(
            persistence.events("job-complete").unwrap().last(),
            Some(ExecutorEvent::Completed { archive_directory })
                if archive_directory == "archives"
        ));
        assert!(matches!(
            persistence.events("job-failed").unwrap().last(),
            Some(ExecutorEvent::DownloadFailed { error_code, .. })
                if error_code == "ARCHIVE_COMMIT_INCOMPLETE"
        ));
        assert!(persistence.events("job-resume").unwrap().is_empty());
        assert_eq!(
            persistence.snapshot("job-terminal").unwrap().state,
            JobState::Complete
        );
    }

    #[test]
    fn storage_recovery_persists_mixed_actions_and_event_order_without_cross_job_pollution() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        insert_downloaded_job(&mut persistence, "job-resume", "tweet-resume");
        insert_downloaded_job(&mut persistence, "job-complete", "tweet-complete");
        insert_downloaded_job(&mut persistence, "job-failed", "tweet-failed");
        insert_downloaded_job(&mut persistence, "job-missing", "tweet-missing");
        insert_downloaded_job(&mut persistence, "job-terminal", "tweet-terminal");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-terminal".into(),
                tweet_id: "tweet-terminal".into(),
                state: JobState::Complete,
            })
            .unwrap();

        let mut provider = InMemoryCommitRecoveryFactsProvider::default();
        provider.insert(CommitRecoveryFacts {
            job_id: "job-resume".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Present,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-complete".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-failed".into(),
            state: JobState::Downloaded,
            final_directory: CommitRecoveryDirectory::Missing,
            staging_directory: CommitRecoveryDirectory::Missing,
        });
        provider.insert(CommitRecoveryFacts {
            job_id: "job-terminal".into(),
            state: JobState::Complete,
            final_directory: CommitRecoveryDirectory::Present,
            staging_directory: CommitRecoveryDirectory::Missing,
        });

        let results = service.recover_commits_persisted(&mut persistence, &provider, "archives");
        assert_eq!(
            results
                .iter()
                .map(|result| result.job_id.as_str())
                .collect::<Vec<_>>(),
            vec!["job-complete", "job-failed", "job-missing", "job-resume"]
        );

        let complete_events = persistence.stored_events("job-complete").unwrap();
        assert_eq!(complete_events.last().unwrap().0, "JOB_COMPLETED");
        assert_eq!(
            persistence.snapshot("job-complete").unwrap().state,
            JobState::Complete
        );

        let failed_events = persistence.stored_events("job-failed").unwrap();
        assert_eq!(failed_events.last().unwrap().0, "DOWNLOAD_FAILED");
        let failed_summary = persistence.stored_job_summary("job-failed").unwrap();
        assert_eq!(failed_summary.state, JobState::Failed);
        assert_eq!(
            failed_summary.last_error_code.as_deref(),
            Some("ARCHIVE_COMMIT_INCOMPLETE")
        );
        assert!(
            failed_summary
                .last_error_message
                .as_deref()
                .is_some_and(|message| message.contains("job-failed"))
        );

        assert!(persistence.stored_events("job-resume").unwrap().len() >= 4);
        assert_eq!(
            persistence.snapshot("job-resume").unwrap().state,
            JobState::Downloaded
        );
        let missing_events = persistence.stored_events("job-missing").unwrap();
        assert!(missing_events.len() >= 4);
        assert!(
            missing_events
                .iter()
                .all(|(event_type, _)| event_type == "JOB_STATE_CHANGED")
        );
        assert_eq!(
            persistence.snapshot("job-missing").unwrap().state,
            JobState::Downloaded
        );
        assert_eq!(
            persistence.snapshot("job-terminal").unwrap().state,
            JobState::Complete
        );

        let terminal_events = persistence.stored_events("job-terminal").unwrap();
        assert!(terminal_events.len() >= 5);
        assert!(
            terminal_events
                .iter()
                .all(|(event_type, _)| event_type != "JOB_COMPLETED")
        );
    }

    #[test]
    fn storage_completion_persists_state_then_job_completed_event() {
        let mut persistence = StorageJobPersistence::open_in_memory().unwrap();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::MetadataReady,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            })
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloaded,
            })
            .unwrap();

        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service
            .complete_persisted(&mut persistence, "job-1", "archives/123")
            .unwrap();
        let events = persistence.stored_events("job-1").unwrap();
        assert_eq!(events.last().unwrap().0, "JOB_COMPLETED");
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Complete
        );
    }

    #[test]
    fn storage_persistence_adapter_matches_job_repository_contract() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        let first = persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        let duplicate = persistence
            .create_or_reuse(&request("job-2", "tweet-1"))
            .expect("reuse");

        assert!(first.created);
        assert!(!duplicate.created);
        assert_eq!(duplicate.job.job_id, "job-1");
        assert_eq!(
            persistence.snapshot("job-1").expect("snapshot").state,
            JobState::Queued
        );

        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .expect("event");
        assert_eq!(
            persistence.events("job-1").expect("events"),
            vec![ExecutorEvent::Submitted {
                job_id: "job-1".into(),
            }]
        );
    }

    #[test]
    fn database_factory_opens_a_job_context_without_runtime_state() {
        let factory = InMemoryJobDatabaseFactory;
        let mut persistence = StorageJobPersistence::from_factory(&factory, "job-1").unwrap();
        let result = persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .unwrap();
        assert!(result.created);
        assert_eq!(
            persistence.snapshot("job-1").unwrap().state,
            JobState::Queued
        );
    }

    #[test]
    fn storage_persistence_preserves_state_transition_and_created_event() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .expect("created event");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .expect("transition");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Validating,
                },
            )
            .expect("state event");

        let events = persistence.stored_events("job-1").expect("stored events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "JOB_CREATED");
        assert_eq!(events[1].0, "JOB_STATE_CHANGED");
        assert!(events[0].1.is_some());
        assert!(events[1].1.is_none());
        assert_eq!(
            persistence.snapshot("job-1").expect("snapshot").state,
            JobState::Validating
        );
    }

    #[test]
    fn executor_lifecycle_events_match_existing_archive_job_event_types() {
        let cases = [
            (
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
                "JOB_CREATED",
            ),
            (
                ExecutorEvent::DownloadStarted {
                    backend: "gallery-dl".into(),
                },
                "DOWNLOAD_STARTED",
            ),
            (
                ExecutorEvent::DownloadCompleted { files_copied: 2 },
                "DOWNLOAD_COMPLETED",
            ),
            (
                ExecutorEvent::DownloadFailed {
                    error_code: "DOWNLOAD_FAILED".into(),
                    error_message: "network error".into(),
                },
                "DOWNLOAD_FAILED",
            ),
            (
                ExecutorEvent::Completed {
                    archive_directory: "archives/123".into(),
                },
                "JOB_COMPLETED",
            ),
        ];

        for (event, expected_type) in cases {
            let job_event = event.to_job_event("job-1").expect("mapped event");
            assert_eq!(job_event.event_type(), expected_type);
        }
        assert!(
            ExecutorEvent::Reused {
                job_id: "job-1".into()
            }
            .to_job_event("job-1")
            .is_none()
        );
    }

    #[test]
    fn storage_persistence_does_not_duplicate_transactional_state_events() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
            )
            .expect("created event");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Validating,
            })
            .expect("transition");
        persistence
            .record_event(
                "job-1",
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Validating,
                },
            )
            .expect("lifecycle event");

        let events = persistence.stored_events("job-1").expect("events");
        assert_eq!(
            events
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            vec!["JOB_CREATED", "JOB_STATE_CHANGED"]
        );
    }

    #[test]
    fn job_summary_projection_preserves_executor_fields_without_synthesizing_storage_fields() {
        let summary = JobSummary {
            job_id: "job-1".into(),
            tweet_id: "tweet-1".into(),
            tweet_type: "quote".into(),
            state: JobState::Failed,
            created_at: "2026-09-13T00:00:00Z".into(),
            updated_at: "2026-09-13T00:01:00Z".into(),
            last_error_code: Some("DOWNLOAD_FAILED".into()),
            last_error_message: Some("network error".into()),
        };

        assert_eq!(
            JobSnapshot::from_job_summary(&summary),
            JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Failed,
            }
        );
        assert_eq!(summary.tweet_type, "quote");
        assert_eq!(summary.created_at, "2026-09-13T00:00:00Z");
        assert_eq!(summary.updated_at, "2026-09-13T00:01:00Z");
        assert_eq!(summary.last_error_code.as_deref(), Some("DOWNLOAD_FAILED"));
        assert_eq!(summary.last_error_message.as_deref(), Some("network error"));
    }

    #[test]
    fn storage_summary_and_event_queries_preserve_error_and_nullable_payload_contracts() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create");
        persistence
            .database
            .fail_job(
                "job-1",
                JobState::Failed,
                "DOWNLOAD_FAILED",
                "network error",
                "2026-09-13T00:01:00Z",
            )
            .expect("fail");

        let summary = persistence.stored_job_summary("job-1").expect("summary");
        assert_eq!(summary.state, JobState::Failed);
        assert_eq!(summary.last_error_code.as_deref(), Some("DOWNLOAD_FAILED"));
        assert_eq!(summary.last_error_message.as_deref(), Some("network error"));

        let events = persistence.stored_events("job-1").expect("events");
        assert_eq!(events[0].0, "JOB_STATE_CHANGED");
        assert!(events[0].1.is_none());
    }

    #[test]
    fn storage_persistence_recovery_scans_active_jobs_and_skips_terminal_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");

        persistence
            .create_or_reuse(&request("job-queued", "tweet-queued"))
            .expect("queued job");
        persistence
            .create_or_reuse(&request("job-interrupted", "tweet-interrupted"))
            .expect("interrupted job");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-interrupted".into(),
                tweet_id: "tweet-interrupted".into(),
                state: JobState::Interrupted,
            })
            .expect("interrupt job");
        persistence
            .create_or_reuse(&request("job-failed", "tweet-failed"))
            .expect("failed job");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-failed".into(),
                tweet_id: "tweet-failed".into(),
                state: JobState::Validating,
            })
            .expect("validate failed job");
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-failed".into(),
                tweet_id: "tweet-failed".into(),
                state: JobState::Failed,
            })
            .expect("fail job");

        let candidates = persistence.list_recovery_candidates();
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|job| job.job_id != "job-failed"));

        let recovered = service.recover_persisted(&mut persistence).unwrap();
        assert_eq!(recovered.len(), 2);
        assert!(
            recovered
                .iter()
                .all(|job| job.state == JobState::Validating)
        );
        assert_eq!(
            persistence.snapshot("job-queued").unwrap().state,
            JobState::Validating
        );
        assert_eq!(
            persistence.snapshot("job-interrupted").unwrap().state,
            JobState::Validating
        );
        assert_eq!(
            persistence.snapshot("job-failed").unwrap().state,
            JobState::Failed
        );
        let interrupted_events = persistence.stored_events("job-interrupted").unwrap();
        assert_eq!(
            interrupted_events
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            vec!["JOB_STATE_CHANGED", "JOB_STATE_CHANGED"]
        );
    }

    #[test]
    fn persisted_recovery_records_the_actual_source_state_for_queued_and_interrupted_jobs() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-queued", "tweet-queued"))
            .unwrap();
        persistence
            .create_or_reuse(&request("job-interrupted", "tweet-interrupted"))
            .unwrap();
        persistence
            .persist_state(&JobSnapshot {
                job_id: "job-interrupted".into(),
                tweet_id: "tweet-interrupted".into(),
                state: JobState::Interrupted,
            })
            .unwrap();

        service.recover_persisted(&mut persistence).unwrap();
        assert_eq!(
            persistence.events("job-queued").unwrap(),
            vec![ExecutorEvent::Recovered {
                from: JobState::Queued,
                to: JobState::Validating,
            }]
        );
        assert_eq!(
            persistence.events("job-interrupted").unwrap(),
            vec![ExecutorEvent::Recovered {
                from: JobState::Interrupted,
                to: JobState::Validating,
            }]
        );
    }

    #[test]
    fn executor_events_preserve_state_transition_order() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service.submit(request("job-1", "tweet-1")).unwrap();
        service.cancel("job-1").unwrap();

        assert_eq!(
            service.events("job-1").unwrap(),
            vec![
                ExecutorEvent::Submitted {
                    job_id: "job-1".into(),
                },
                ExecutorEvent::StateChanged {
                    from: JobState::Queued,
                    to: JobState::Cancelled,
                },
            ]
        );
    }

    #[test]
    fn sidecar_crash_maps_to_safe_failure_and_core_job_event() {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        service.submit(request("job-1", "tweet-1")).unwrap();
        service.executor.start("job-1").unwrap();
        let snapshot = service
            .executor
            .sidecar_crashed("job-1", "worker exited")
            .unwrap();

        assert_eq!(snapshot.state, JobState::Failed);
        let events = service.events("job-1").unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutorEvent::SidecarCrashed { error_message } if error_message == "worker exited"
        )));
        assert_eq!(
            events.last().and_then(|event| event.to_job_event("job-1")),
            Some(JobEvent::DownloadFailed {
                error_code: "SIDECAR_INTERNAL_ERROR".into(),
                error_message: "worker exited".into(),
            })
        );
    }
}

/// Linux-only: the boundary tests exercise the real Unix coordination
/// adapter, because the point of the boundary is cross-process-equivalent
/// scope exclusivity. The boundary itself stays generic and compiles on
/// every target; only these probes are gated.
#[cfg(all(test, unix))]
mod session_boundary_tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::mpsc::{Receiver, Sender};
    use std::thread;
    use xarchive_workflow::coordination::LockScope;
    use xarchive_workflow::coordination::unix::LinuxCoordinator;

    fn scratch(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "tg-session-boundary-{}-{}-{}",
            label,
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&directory).expect("scratch directory");
        directory
    }

    fn request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
        ArchiveJobRequest {
            job_id: job_id.to_owned(),
            tweet_id: tweet_id.to_owned(),
            request_id: format!("session-request-{job_id}"),
            request_json: "{}".to_owned(),
            schema_version: 1,
        }
    }

    /// One-shot start/release channel pair a blocking probe execution waits on.
    struct ExecutionGate {
        started: Sender<()>,
        release: Receiver<()>,
    }

    struct ProbeExecution {
        lock_dir: PathBuf,
        job_id: String,
        execute_scope_contended: Arc<Mutex<Vec<bool>>>,
        started: Option<Sender<()>>,
        release: Option<Receiver<()>>,
        failure: Option<(String, String)>,
        state_already_updated: bool,
    }

    impl JobExecution for ProbeExecution {
        fn execute(
            &mut self,
            job: &JobSnapshot,
            _cancellation: &CancellationToken,
        ) -> Result<JobExecutionResult, JobExecutionError> {
            if let Some(started) = self.started.take() {
                let _ = started.send(());
            }
            if let Some(release) = self.release.take() {
                let _ = release.recv();
            }
            if let Some((error_code, error_message)) = self.failure.take() {
                return Err(JobExecutionError {
                    error_code,
                    error_message,
                    persistence_already_updated: false,
                });
            }
            // While the execution object runs (writers and publication are
            // inside it in production), a peer must see the Job scope as
            // contended: the session guard is held by the boundary thread.
            let peer =
                LinuxCoordinator::new(&self.lock_dir).map_err(|error| JobExecutionError {
                    error_code: "PROBE_COORDINATOR".to_owned(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                })?;
            let contended = matches!(
                peer.acquire(&LockScope::Job(self.job_id.clone())),
                Err(xarchive_workflow::coordination::LockError::Contended)
            ) && matches!(
                peer.acquire(&LockScope::Destination(
                    PathBuf::from("/archive/Tweets").join(&job.tweet_id)
                )),
                Err(xarchive_workflow::coordination::LockError::Contended)
            );
            self.execute_scope_contended
                .lock()
                .expect("probe contention log")
                .push(contended);
            Ok(JobExecutionResult {
                files_copied: 1,
                archive_directory: format!("Tweets/{}", job.tweet_id),
                state_already_updated: self.state_already_updated,
                download_event_recorded: false,
            })
        }
    }

    struct ProbeFactory {
        lock_dir: PathBuf,
        create_scope_contended: Arc<Mutex<Vec<bool>>>,
        execute_scope_contended: Arc<Mutex<Vec<bool>>>,
        gate: Mutex<Option<ExecutionGate>>,
    }

    impl ProbeFactory {
        fn new(lock_dir: PathBuf) -> Self {
            Self {
                lock_dir,
                create_scope_contended: Arc::new(Mutex::new(Vec::new())),
                execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                gate: Mutex::new(None),
            }
        }
    }

    impl JobExecutionFactory for ProbeFactory {
        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            assert_eq!(identity.job_id(), snapshot.job_id);
            assert_eq!(identity.attempt(), 1, "factory sees the claimed attempt");
            self.create(identity.job_id(), snapshot)
        }

        fn create(
            &self,
            job_id: &str,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, ExecutorError> {
            // Factory creation must happen after the durable claim, inside
            // the live session: a peer must already see the Job scope held.
            let peer = LinuxCoordinator::new(&self.lock_dir).map_err(|error| {
                ExecutorError::Execution {
                    error_code: "PROBE_COORDINATOR".to_owned(),
                    error_message: error.to_string(),
                    persistence_already_updated: false,
                }
            })?;
            self.create_scope_contended
                .lock()
                .expect("probe contention log")
                .push(
                    matches!(
                        peer.acquire(&LockScope::Job(job_id.to_owned())),
                        Err(xarchive_workflow::coordination::LockError::Contended)
                    ) && matches!(
                        peer.acquire(&LockScope::Destination(
                            PathBuf::from("/archive/Tweets").join(&snapshot.tweet_id)
                        )),
                        Err(xarchive_workflow::coordination::LockError::Contended)
                    ),
                );
            let gate = self.gate.lock().expect("probe gate").take();
            let (started, release) = match gate {
                Some(gate) => (Some(gate.started), Some(gate.release)),
                None => (None, None),
            };
            Ok(Box::new(ProbeExecution {
                lock_dir: self.lock_dir.clone(),
                job_id: job_id.to_owned(),
                execute_scope_contended: self.execute_scope_contended.clone(),
                started,
                release,
                failure: None,
                state_already_updated: false,
            }))
        }
    }

    /// In-memory persistence that logs, at every durable state write,
    /// whether a peer coordinator sees the Job scope as contended, and that
    /// can force the durable claim to report a lost race.
    struct BoundaryProbePersistence {
        inner: InMemoryJobPersistence,
        lock_dir: PathBuf,
        persist_scope_contended: Arc<Mutex<Vec<(JobState, bool)>>>,
        force_claim_lost: bool,
    }

    impl BoundaryProbePersistence {
        fn new(lock_dir: PathBuf) -> Self {
            Self {
                inner: InMemoryJobPersistence::default(),
                lock_dir,
                persist_scope_contended: Arc::new(Mutex::new(Vec::new())),
                force_claim_lost: false,
            }
        }
    }

    impl JobPersistence for BoundaryProbePersistence {
        fn create_or_reuse(
            &mut self,
            request: &ArchiveJobRequest,
        ) -> Result<SubmitResult, ExecutorError> {
            self.inner.create_or_reuse(request)
        }
        fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
            self.inner.snapshot(job_id)
        }
        fn list_recovery_candidates(&self) -> Vec<JobSnapshot> {
            self.inner.list_recovery_candidates()
        }
        fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
            let peer = LinuxCoordinator::new(&self.lock_dir)
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
            let contended = matches!(
                peer.acquire(&LockScope::Job(snapshot.job_id.clone())),
                Err(xarchive_workflow::coordination::LockError::Contended)
            ) && matches!(
                peer.acquire(&LockScope::Destination(
                    PathBuf::from("/archive/Tweets").join(&snapshot.tweet_id)
                )),
                Err(xarchive_workflow::coordination::LockError::Contended)
            );
            self.persist_scope_contended
                .lock()
                .expect("probe contention log")
                .push((snapshot.state, contended));
            self.inner.persist_state(snapshot)
        }
        fn fail(
            &mut self,
            job_id: &str,
            error_code: &str,
            error_message: &str,
        ) -> Result<JobSnapshot, ExecutorError> {
            self.inner.fail(job_id, error_code, error_message)
        }
        fn record_event(
            &mut self,
            job_id: &str,
            event: ExecutorEvent,
        ) -> Result<(), ExecutorError> {
            self.inner.record_event(job_id, event)
        }
        fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
            self.inner.events(job_id)
        }
        fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
            self.inner.begin_attempt(job_id)
        }
        fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
            self.inner.current_attempt(job_id)
        }
        fn claim_attempt(
            &mut self,
            job_id: &str,
            expected_attempt: u32,
            from: JobState,
            to: JobState,
        ) -> Result<Option<u32>, ExecutorError> {
            if self.force_claim_lost {
                return Ok(None);
            }
            self.inner.claim_attempt(job_id, expected_attempt, from, to)
        }
        fn record_claimed_attempt_state(
            &mut self,
            job_id: &str,
            from: JobState,
            to: JobState,
        ) -> Result<(), ExecutorError> {
            self.inner.record_claimed_attempt_state(job_id, from, to)
        }
        fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
            self.inner.attempt_is_current(job_id, attempt)
        }
        fn complete_terminal_for_attempt(
            &mut self,
            job_id: &str,
            attempt: u32,
            archive_directory: &str,
            files_copied: u32,
        ) -> Result<JobSnapshot, ExecutorError> {
            let snapshot = self.inner.snapshot(job_id)?;
            if snapshot.state == JobState::Complete {
                return Ok(snapshot);
            }
            if snapshot.state != JobState::Downloaded {
                return Err(ExecutorError::Persistence(
                    "cannot complete job in non-Downloaded state".into(),
                ));
            }
            if !self.inner.attempt_is_current(job_id, attempt)? {
                return Err(ExecutorError::Persistence("stale attempt".into()));
            }
            let next = snapshot
                .state
                .transition_to(JobState::Complete)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: snapshot.state,
                    to: JobState::Complete,
                })?;
            let updated = JobSnapshot {
                state: next,
                ..snapshot.clone()
            };
            // Write through `Self::persist_state` so the contention probe
            // logs the final Complete write under the held Job scope.
            self.persist_state(&updated)?;
            self.inner
                .record_event(job_id, ExecutorEvent::DownloadCompleted { files_copied })?;
            self.inner.record_event(
                job_id,
                ExecutorEvent::Completed {
                    archive_directory: archive_directory.to_owned(),
                },
            )?;
            Ok(updated)
        }
    }

    /// Compile-time claim: the boundary must be `Send` so it can be moved
    /// into the scheduler's `'static` thread together with its owned
    /// persistence context and coordinator.
    #[test]
    fn the_boundary_is_send_so_it_can_move_into_the_scheduler_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<SessionExecutionBoundary<InMemoryJobPersistence, LinuxCoordinator>>();
    }

    #[test]
    fn factory_failure_finalizes_the_claim_and_releases_admission_and_scopes() {
        let root = scratch("factory-failure");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity(4);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");
        let mut boundary = SessionExecutionBoundary::new(
            ArchiveApplicationService::new(&executor),
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let outcome = boundary
            .execute_session("job-1")
            .expect("failure finalized");
        assert!(
            matches!(outcome, SessionRunOutcome::Finished(ref job) if job.state == JobState::Failed)
        );
        assert_eq!(
            boundary
                .persistence
                .current_attempt("job-1")
                .expect("attempt"),
            1
        );
        assert!(
            boundary
                .persistence
                .events("job-1")
                .expect("events")
                .iter()
                .any(
                    |event| matches!(event, ExecutorEvent::DownloadFailed { error_message, .. }
                if error_message.contains("ATTEMPT_EXECUTION_NOT_CONFIGURED"))
                )
        );
        let peer = LinuxCoordinator::new(&lock_dir).expect("peer");
        let reservation =
            ExecutionReservation::acquire(&peer, "job-1", Path::new("/archive/Tweets/tweet-1"))
                .expect("failed factory released scopes");
        drop(reservation);
        assert!(
            boundary.service.executor.runner_try_admit(),
            "runner admission released"
        );
        boundary.service.executor.runner_release();
        drop(boundary);
        drop(executor);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn archive_writer_drain_failure_is_terminal_and_never_completes_archive() {
        struct DrainFailureFactory;
        impl JobExecutionFactory for DrainFailureFactory {
            fn create_for_attempt(
                &self,
                identity: &AttemptIdentity,
                snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                assert_eq!(identity.job_id(), snapshot.job_id);
                assert_eq!(identity.attempt(), 1);
                self.create(&snapshot.job_id, snapshot)
            }

            fn create(
                &self,
                job_id: &str,
                _snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                Ok(Box::new(ProbeExecution {
                    lock_dir: PathBuf::new(),
                    job_id: job_id.to_owned(),
                    execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                    started: None,
                    release: None,
                    failure: Some((
                        "ARCHIVE_WRITER_DRAIN_FAILED".to_owned(),
                        "Sidecar shutdown incomplete: child still running".to_owned(),
                    )),
                    state_already_updated: false,
                }))
            }
        }

        let root = scratch("writer-drain-failure");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity(4);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");
        let mut handle = executor.handle();
        handle.factory = Arc::new(DrainFailureFactory);
        let mut boundary = SessionExecutionBoundary::new(
            ArchiveApplicationService {
                executor: handle,
                v2_enrollment: false,
            },
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );

        let outcome = boundary.execute_session("job-1").expect("terminal result");
        let SessionRunOutcome::Finished(snapshot) = outcome else {
            panic!("drain failure must be terminal, not rescheduled: {outcome:?}");
        };
        assert_eq!(snapshot.state, JobState::Failed);
        let events = boundary.persistence.events("job-1").expect("events");
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutorEvent::DownloadFailed { error_code, error_message }
                if error_code == "EXECUTOR_WORKER_FAILED"
                    && error_message.contains("job execution error [ARCHIVE_WRITER_DRAIN_FAILED]")
                    && error_message.contains("Sidecar shutdown incomplete")
        )), "drain failure must be terminal and preserve diagnostic detail: {events:?}");
        assert!(
            !events.iter().any(|event| matches!(
                event,
                ExecutorEvent::DownloadCompleted { .. } | ExecutorEvent::Completed { .. }
            )),
            "drain failure must never enter completion: {events:?}"
        );
        assert_eq!(
            boundary
                .persistence
                .current_attempt("job-1")
                .expect("attempt"),
            1
        );
        drop(boundary);
        drop(executor);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stale_attempt_success_records_no_completion_event_or_final_state() {
        struct StaleSuccessFactory;
        impl JobExecutionFactory for StaleSuccessFactory {
            fn create_for_attempt(
                &self,
                identity: &AttemptIdentity,
                snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                assert_eq!(identity.attempt(), 1);
                self.create(&snapshot.job_id, snapshot)
            }

            fn create(
                &self,
                job_id: &str,
                _snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                Ok(Box::new(ProbeExecution {
                    lock_dir: PathBuf::new(),
                    job_id: job_id.to_owned(),
                    execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                    started: None,
                    release: None,
                    failure: None,
                    state_already_updated: false,
                }))
            }
        }

        struct StaleAfterExecutePersistence {
            inner: InMemoryJobPersistence,
            job_id: String,
        }
        impl JobPersistence for StaleAfterExecutePersistence {
            fn create_or_reuse(
                &mut self,
                request: &ArchiveJobRequest,
            ) -> Result<SubmitResult, ExecutorError> {
                self.inner.create_or_reuse(request)
            }
            fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
                self.inner.snapshot(job_id)
            }
            fn list_recovery_candidates(&self) -> Vec<JobSnapshot> {
                self.inner.list_recovery_candidates()
            }
            fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
                self.inner.persist_state(snapshot)
            }
            fn fail(
                &mut self,
                job_id: &str,
                error_code: &str,
                error_message: &str,
            ) -> Result<JobSnapshot, ExecutorError> {
                self.inner.fail(job_id, error_code, error_message)
            }
            fn record_event(
                &mut self,
                job_id: &str,
                event: ExecutorEvent,
            ) -> Result<(), ExecutorError> {
                self.inner.record_event(job_id, event)
            }
            fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
                self.inner.events(job_id)
            }
            fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
                self.inner.begin_attempt(job_id)
            }
            fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
                self.inner.current_attempt(job_id)
            }
            fn claim_attempt(
                &mut self,
                job_id: &str,
                expected_attempt: u32,
                from: JobState,
                to: JobState,
            ) -> Result<Option<u32>, ExecutorError> {
                self.inner.claim_attempt(job_id, expected_attempt, from, to)
            }
            fn record_claimed_attempt_state(
                &mut self,
                job_id: &str,
                from: JobState,
                to: JobState,
            ) -> Result<(), ExecutorError> {
                self.inner.record_claimed_attempt_state(job_id, from, to)
            }
            fn attempt_is_current(
                &self,
                job_id: &str,
                attempt: u32,
            ) -> Result<bool, ExecutorError> {
                // The attempt was current while its execution ran; a newer
                // attempt won before terminal writes, so the stale result must
                // be suppressed without persisting completion.
                if job_id == self.job_id && attempt == 1 {
                    return Ok(false);
                }
                self.inner.attempt_is_current(job_id, attempt)
            }
            fn complete_terminal_for_attempt(
                &mut self,
                job_id: &str,
                attempt: u32,
                archive_directory: &str,
                files_copied: u32,
            ) -> Result<JobSnapshot, ExecutorError> {
                self.inner.complete_terminal_for_attempt(
                    job_id,
                    attempt,
                    archive_directory,
                    files_copied,
                )
            }
        }

        let root = scratch("stale-success-suppression");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity(4);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");
        let mut handle = executor.handle();
        handle.factory = Arc::new(StaleSuccessFactory);
        let stale_persistence = StaleAfterExecutePersistence {
            inner: persistence,
            job_id: "job-1".to_owned(),
        };
        let mut boundary = SessionExecutionBoundary::new(
            ArchiveApplicationService {
                executor: handle,
                v2_enrollment: false,
            },
            stale_persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let outcome = boundary.execute_session("job-1").expect("stale suppressed");
        let SessionRunOutcome::Finished(snapshot) = outcome else {
            panic!("stale success must not reschedule: {outcome:?}");
        };
        assert_ne!(snapshot.state, JobState::Complete);
        let events = boundary.persistence.inner.events("job-1").expect("events");
        assert!(
            !events.iter().any(|event| matches!(
                event,
                ExecutorEvent::DownloadCompleted { .. } | ExecutorEvent::Completed { .. }
            )),
            "stale success must record no completion: {events:?}"
        );
        drop(boundary);
        drop(executor);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn fenced_publication_records_download_completed_and_skips_duplicate_complete() {
        /// Execution reports `state_already_updated: true`, meaning fenced
        /// publication already committed inside `execute_archive_context`. The
        /// session must record `DownloadCompleted` (best-effort, after the
        /// durable publication commit) but must not call `complete_persisted`
        /// again, avoiding a double publication and a duplicate `Completed`.
        struct FencedFactory;
        impl JobExecutionFactory for FencedFactory {
            fn create_for_attempt(
                &self,
                identity: &AttemptIdentity,
                snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                assert_eq!(identity.job_id(), snapshot.job_id);
                assert_eq!(identity.attempt(), 1, "factory sees the claimed attempt");
                self.create(&snapshot.job_id, snapshot)
            }
            fn create(
                &self,
                job_id: &str,
                _snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                Ok(Box::new(ProbeExecution {
                    lock_dir: PathBuf::new(),
                    job_id: job_id.to_owned(),
                    execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                    started: None,
                    release: None,
                    failure: None,
                    state_already_updated: true,
                }))
            }
        }

        let root = scratch("fenced-publication");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity(4);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");
        let mut handle = executor.handle();
        handle.factory = Arc::new(FencedFactory);
        let mut boundary = SessionExecutionBoundary::new(
            ArchiveApplicationService {
                executor: handle,
                v2_enrollment: false,
            },
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let outcome = boundary.execute_session("job-1").expect("terminal result");
        let SessionRunOutcome::Finished(snapshot) = outcome else {
            panic!("fenced publication must be terminal: {outcome:?}");
        };
        assert_eq!(
            snapshot.state,
            JobState::Complete,
            "complete_persisted must advance state even when state_already_updated"
        );
        let events = boundary.persistence.events("job-1").expect("events");
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutorEvent::DownloadCompleted { .. })),
            "DownloadCompleted must be recorded after fenced publication: {events:?}"
        );
        let completed_events: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, ExecutorEvent::Completed { .. }))
            .collect();
        assert_eq!(
            completed_events.len(),
            1,
            "Completed must be recorded exactly once (no duplicate): {events:?}"
        );
        assert_eq!(
            boundary
                .persistence
                .current_attempt("job-1")
                .expect("attempt"),
            1
        );
        drop(boundary);
        drop(executor);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn non_fenced_execution_records_download_completed_and_completes() {
        /// Execution reports `state_already_updated: false`, meaning no fenced
        /// publication has committed. The session must record `DownloadCompleted`
        /// and then call `complete_persisted`, advancing the Job to `Complete`.
        struct NonFencedFactory;
        impl JobExecutionFactory for NonFencedFactory {
            fn create_for_attempt(
                &self,
                identity: &AttemptIdentity,
                snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                assert_eq!(identity.job_id(), snapshot.job_id);
                assert_eq!(identity.attempt(), 1, "factory sees the claimed attempt");
                self.create(&snapshot.job_id, snapshot)
            }
            fn create(
                &self,
                job_id: &str,
                _snapshot: &JobSnapshot,
            ) -> Result<Box<dyn JobExecution>, ExecutorError> {
                Ok(Box::new(ProbeExecution {
                    lock_dir: PathBuf::new(),
                    job_id: job_id.to_owned(),
                    execute_scope_contended: Arc::new(Mutex::new(Vec::new())),
                    started: None,
                    release: None,
                    failure: None,
                    state_already_updated: false,
                }))
            }
        }

        let root = scratch("non-fenced-completion");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity(4);
        let mut persistence = InMemoryJobPersistence::default();
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");
        let mut handle = executor.handle();
        handle.factory = Arc::new(NonFencedFactory);
        let mut boundary = SessionExecutionBoundary::new(
            ArchiveApplicationService {
                executor: handle,
                v2_enrollment: false,
            },
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let outcome = boundary.execute_session("job-1").expect("terminal result");
        let SessionRunOutcome::Finished(snapshot) = outcome else {
            panic!("non-fenced execution must be terminal: {outcome:?}");
        };
        assert_eq!(
            snapshot.state,
            JobState::Complete,
            "complete_persisted must advance state when not state_already_updated"
        );
        let events = boundary.persistence.events("job-1").expect("events");
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutorEvent::DownloadCompleted { .. })),
            "DownloadCompleted must be recorded before complete_persisted: {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutorEvent::Completed { .. })),
            "Completed must be recorded after complete_persisted: {events:?}"
        );
        assert_eq!(
            boundary
                .persistence
                .current_attempt("job-1")
                .expect("attempt"),
            1
        );
        drop(boundary);
        drop(executor);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn inline_runner_operation_is_static_runs_on_runner_and_drops_before_admission() {
        use std::sync::atomic::AtomicUsize;

        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<Box<dyn FnOnce() + Send + 'static>>();

        let executor = JobExecutor::with_capacity(4);
        let handle = executor.handle();
        assert!(handle.runner_try_admit());
        let admission = handle.owned_runner_admission();
        let observed_thread = Arc::new(Mutex::new(None::<thread::ThreadId>));
        let operation_thread = observed_thread.clone();
        let captured_dropped = Arc::new(AtomicUsize::new(0));
        let captured_counter = captured_dropped.clone();
        let local_dropped = Arc::new(AtomicUsize::new(0));
        let local_counter = local_dropped.clone();
        let operation_handle = handle.clone();
        struct CapturedDropProbe(Arc<AtomicUsize>);
        impl Drop for CapturedDropProbe {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::AcqRel);
            }
        }
        let capture = CapturedDropProbe(captured_counter);
        let result = handle
            .run_operation(admission, move || {
                let _capture = capture;
                *operation_thread.lock().expect("thread observation") =
                    Some(thread::current().id());
                struct DropProbe(Arc<AtomicUsize>);
                impl Drop for DropProbe {
                    fn drop(&mut self) {
                        self.0.fetch_add(1, Ordering::AcqRel);
                    }
                }
                let probe = DropProbe(local_counter);
                let in_flight = operation_handle.runner_in_flight.load(Ordering::Acquire);
                drop(probe);
                Ok((in_flight, thread::current().name().map(str::to_owned)))
            })
            .expect("runner operation");

        assert_eq!(result.0, 1, "admission remains held during the task");
        assert_eq!(result.1.as_deref(), Some("xarchive-job-runner"));
        assert_eq!(local_dropped.load(Ordering::Acquire), 1);
        assert_eq!(
            captured_dropped.load(Ordering::Acquire),
            1,
            "FnOnce captures must be destroyed before the result is returned"
        );
        assert_ne!(
            *observed_thread.lock().expect("thread observation"),
            Some(thread::current().id()),
            "operation must execute on the runner, not the caller"
        );
        assert_eq!(
            handle.runner_in_flight.load(Ordering::Acquire),
            0,
            "admission releases after operation completion"
        );
        drop(handle);
        executor.shutdown().expect("shutdown");
    }

    #[test]
    fn inline_runner_operation_error_drops_capture_before_releasing_admission() {
        use std::sync::atomic::AtomicUsize;

        let executor = JobExecutor::with_capacity(4);
        let handle = executor.handle();
        assert!(handle.runner_try_admit());
        let admission = handle.owned_runner_admission();
        let in_flight_during_drop = Arc::new(AtomicUsize::new(usize::MAX));
        let observed = in_flight_during_drop.clone();
        let operation_handle = handle.clone();
        struct OwnerProbe {
            counter: Arc<AtomicUsize>,
            admission: Arc<AtomicUsize>,
        }
        impl Drop for OwnerProbe {
            fn drop(&mut self) {
                self.counter
                    .store(self.admission.load(Ordering::Acquire), Ordering::Release);
            }
        }
        let captured_owner = OwnerProbe {
            counter: observed,
            admission: operation_handle.runner_in_flight.clone(),
        };
        let result = handle.run_operation(admission, move || {
            let _owner = captured_owner;
            Err::<(), _>(ExecutorError::Persistence("probe error".to_owned()))
        });
        assert!(matches!(
            result,
            Err(ExecutorError::Persistence(message)) if message == "probe error"
        ));
        assert_eq!(in_flight_during_drop.load(Ordering::Acquire), 1);
        assert_eq!(handle.runner_in_flight.load(Ordering::Acquire), 0);
        drop(handle);
        executor.shutdown().expect("shutdown");
    }

    #[test]
    fn session_boundary_execute_on_runner_runs_session_on_named_runner() {
        let root = scratch("boundary-on-runner");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity_and_factory(
            4,
            Arc::new(ProbeFactory::new(lock_dir.clone())),
        );
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
        persistence
            .inner
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create job");
        let boundary = SessionExecutionBoundary::new(
            service,
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );

        let outcome = boundary
            .execute_on_runner("job-1".to_owned())
            .expect("runner-owned boundary");
        assert!(matches!(
            outcome,
            SessionRunOutcome::Finished(JobSnapshot {
                state: JobState::Complete,
                ..
            })
        ));
        assert_eq!(
            executor.handle().runner_in_flight.load(Ordering::Acquire),
            0,
            "runner admission releases after session finalization"
        );
        executor.shutdown().expect("shutdown");
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn inline_runner_operation_drops_owner_before_admission_on_success() {
        let executor = JobExecutor::with_capacity(4);
        let handle = executor.handle();
        assert!(handle.runner_try_admit());
        let admission = handle.owned_runner_admission();
        let observed = Arc::new(AtomicUsize::new(usize::MAX));
        let counter = observed.clone();
        let in_flight = handle.runner_in_flight.clone();
        struct Owner(Arc<AtomicUsize>, Arc<AtomicUsize>);
        impl Drop for Owner {
            fn drop(&mut self) {
                self.0
                    .store(self.1.load(Ordering::Acquire), Ordering::Release);
            }
        }
        let owner = Owner(counter, in_flight);
        handle
            .run_operation(admission, move || {
                let _owner = owner;
                Ok(())
            })
            .expect("runner operation");
        assert_eq!(observed.load(Ordering::Acquire), 1);
        assert_eq!(handle.runner_in_flight.load(Ordering::Acquire), 0);
        executor.shutdown().expect("shutdown");
    }

    #[test]
    fn inline_runner_operation_panic_closes_reply_and_releases_admission() {
        let executor = JobExecutor::with_capacity(4);
        let handle = executor.handle();
        assert!(handle.runner_try_admit());
        let admission = handle.owned_runner_admission();
        let caller_handle = handle.clone();
        let panic_owner_dropped = Arc::new(AtomicBool::new(false));
        let owner_flag = panic_owner_dropped.clone();
        let caller = thread::spawn(move || {
            caller_handle.run_operation(admission, move || -> Result<(), ExecutorError> {
                struct PanicOwner(Arc<AtomicBool>);
                impl Drop for PanicOwner {
                    fn drop(&mut self) {
                        self.0.store(true, Ordering::Release);
                    }
                }
                let _owner = PanicOwner(owner_flag);
                panic!("runner operation panic probe");
            })
        });
        let result = caller.join().expect("caller thread survives runner panic");
        assert!(matches!(
            result,
            Err(ExecutorError::Execution {
                error_code,
                error_message,
                ..
            }) if error_code == "RUNNER_OPERATION_PANICKED"
                && error_message == "runner operation panic probe"
        ));
        assert!(panic_owner_dropped.load(Ordering::Acquire));
        assert_eq!(handle.runner_in_flight.load(Ordering::Acquire), 0);
        assert!(
            handle.runner_try_admit(),
            "runner remains usable after task panic"
        );
        let admission = handle.owned_runner_admission();
        assert_eq!(
            handle
                .run_operation(admission, || Ok::<_, ExecutorError>(()))
                .unwrap(),
            ()
        );
        drop(handle);
        executor.shutdown().expect("shutdown");
    }

    #[test]
    fn a_session_holds_both_scopes_across_execution_and_finalization() {
        let root = scratch("happy");
        let lock_dir = root.join("locks");
        let factory = ProbeFactory::new(lock_dir.clone());
        let create_contended = factory.create_scope_contended.clone();
        let execute_contended = factory.execute_scope_contended.clone();
        let executor = JobExecutor::with_capacity_and_factory(4, Arc::new(factory));
        let service = ArchiveApplicationService::new(&executor);

        let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
        persistence
            .inner
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create job");
        let persist_contended = persistence.persist_scope_contended.clone();

        let mut boundary = SessionExecutionBoundary::new(
            service,
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let outcome = boundary.execute_session("job-1").expect("session run");
        let SessionRunOutcome::Finished(snapshot) = outcome else {
            panic!("expected a finished session run, got {outcome:?}");
        };
        assert_eq!(snapshot.state, JobState::Complete);

        // Every durable state write — including the final `Complete`
        // finalization write — happened while a peer saw the Job scope held.
        let persisted = persist_contended.lock().expect("probe log");
        assert!(!persisted.is_empty(), "the run must persist durable state");
        assert!(
            persisted.iter().all(|(_, contended)| *contended),
            "every state write must occur under the held Job scope: {persisted:?}"
        );
        assert!(
            persisted
                .iter()
                .any(|(state, contended)| *state == JobState::Complete && *contended),
            "finalization must occur under the held Job scope: {persisted:?}"
        );
        drop(persisted);
        // The execution object observed the guard, and the factory was
        // invoked only after the claim established the session.
        let executed = execute_contended.lock().expect("probe log");
        assert!(
            !executed.is_empty() && executed.iter().all(|contended| *contended),
            "the execution object must run under the held Job scope: {executed:?}"
        );
        drop(executed);
        let created = create_contended.lock().expect("probe log");
        assert!(
            !created.is_empty() && created.iter().all(|contended| *contended),
            "factory creation must happen under the held Job scope: {created:?}"
        );
        drop(created);
        let events = boundary.persistence.events("job-1").expect("events");
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ExecutorEvent::DownloadFailed { .. })),
            "a successful run must not record a failure event: {events:?}"
        );
        // After the run both scopes are released for a successor.
        drop(boundary);
        let peer = LinuxCoordinator::new(&lock_dir).expect("peer coordinator");
        ExecutionReservation::acquire(&peer, "job-1", Path::new("/archive/Tweets/tweet-1"))
            .expect("scopes released after the session run");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_live_peer_makes_the_run_reschedulable_without_any_durable_write() {
        let root = scratch("contended");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity_and_factory(
            4,
            Arc::new(ProbeFactory::new(lock_dir.clone())),
        );
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
        persistence
            .inner
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create job");
        let persist_contended = persistence.persist_scope_contended.clone();

        let peer_coordinator = LinuxCoordinator::new(&lock_dir).expect("peer coordinator");
        let peer_guard = peer_coordinator
            .acquire(&LockScope::Job("job-1".to_owned()))
            .expect("peer holds the job scope");

        let mut boundary = SessionExecutionBoundary::new(
            service,
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let outcome = boundary
            .execute_session("job-1")
            .expect("contention is an outcome, not an error");
        assert_eq!(
            outcome,
            SessionRunOutcome::Reschedulable {
                job_id: "job-1".to_owned(),
                reason: SessionRescheduleReason::Contended,
            }
        );
        let snapshot = boundary.persistence.snapshot("job-1").expect("snapshot");
        assert_eq!(
            snapshot.state,
            JobState::Queued,
            "a contended run must not advance the Job"
        );
        assert_eq!(
            boundary
                .persistence
                .current_attempt("job-1")
                .expect("attempt"),
            0,
            "a contended run must not claim an attempt"
        );
        assert!(
            boundary
                .persistence
                .events("job-1")
                .expect("events")
                .is_empty(),
            "a contended run must not persist events"
        );
        assert!(
            persist_contended.lock().expect("probe log").is_empty(),
            "a contended run must not reach any durable state write"
        );
        drop(peer_guard);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_lost_claim_is_reschedulable_releases_scopes_and_persists_no_failure() {
        let root = scratch("lost-claim");
        let lock_dir = root.join("locks");
        let executor = JobExecutor::with_capacity_and_factory(
            4,
            Arc::new(ProbeFactory::new(lock_dir.clone())),
        );
        let service = ArchiveApplicationService::new(&executor);
        let mut persistence = BoundaryProbePersistence::new(lock_dir.clone());
        persistence.force_claim_lost = true;
        persistence
            .inner
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create job");

        let mut boundary = SessionExecutionBoundary::new(
            service,
            persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        let outcome = boundary
            .execute_session("job-1")
            .expect("a lost claim is an outcome, not an error");
        assert_eq!(
            outcome,
            SessionRunOutcome::Reschedulable {
                job_id: "job-1".to_owned(),
                reason: SessionRescheduleReason::ClaimLost,
            }
        );
        let snapshot = boundary.persistence.snapshot("job-1").expect("snapshot");
        assert_eq!(
            snapshot.state,
            JobState::Queued,
            "a lost claim must not advance the Job"
        );
        assert!(
            boundary
                .persistence
                .events("job-1")
                .expect("events")
                .is_empty(),
            "a lost claim must not persist a failure or any event"
        );
        // The failed claim must leave no scope held: a successor can claim.
        drop(boundary);
        let peer = LinuxCoordinator::new(&lock_dir).expect("peer coordinator");
        ExecutionReservation::acquire(&peer, "job-1", Path::new("/archive/Tweets/tweet-1"))
            .expect("scopes released after the lost claim");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_saturated_runner_is_rejected_before_any_claim() {
        let root = scratch("saturation");
        let lock_dir = root.join("locks");
        let factory = ProbeFactory::new(lock_dir.clone());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        *factory.gate.lock().expect("probe gate") = Some(ExecutionGate {
            started: started_tx,
            release: release_rx,
        });
        let executor = JobExecutor::with_capacity_and_factory(4, Arc::new(factory));
        let service = ArchiveApplicationService::new(&executor);

        let mut running_persistence = BoundaryProbePersistence::new(lock_dir.clone());
        running_persistence
            .inner
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("create running job");
        let mut running_boundary = SessionExecutionBoundary::new(
            service.clone(),
            running_persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        assert!(executor.handle().runner_try_admit());
        let reservation = executor.handle().owned_runner_admission();
        let runner = thread::spawn(move || {
            let _reservation = reservation;
            running_boundary.execute_session("job-1")
        });
        started_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("job-1 execution started");

        // While job-1 still owns the single runner slot, a second boundary must
        // refuse completely: no attempt claimed, no persistence read, no
        // durable write.
        let queued_job2 = {
            let mut waiting_persistence = BoundaryProbePersistence::new(lock_dir.clone());
            waiting_persistence
                .inner
                .create_or_reuse(&request("job-2", "tweet-2"))
                .expect("create waiting job");
            let waiting_boundary = SessionExecutionBoundary::new(
                service.clone(),
                waiting_persistence,
                LinuxCoordinator::new(&lock_dir).expect("coordinator"),
                PathBuf::from("/archive"),
            );
            let first_snapshot = waiting_boundary
                .persistence
                .snapshot("job-2")
                .expect("snapshot");
            let first_current_attempt = waiting_boundary
                .persistence
                .current_attempt("job-2")
                .expect("attempt");
            let first_events = waiting_boundary
                .persistence
                .events("job-2")
                .expect("events");
            let rejected = waiting_boundary.execute_on_runner("job-2".to_owned());
            assert_eq!(
                rejected.expect_err("a saturated runner must refuse before claiming"),
                ExecutorError::QueueFull
            );
            assert_eq!(first_snapshot.state, JobState::Queued);
            assert_eq!(first_current_attempt, 0);
            assert!(first_events.is_empty());
            first_snapshot
        };

        // Release the runner, then run the same saturated-boundary command on a
        // fresh ownership: it must complete job-2 through the runner operation.
        release_tx.send(()).expect("release job-1");
        let finished = runner.join().expect("job-1 thread").expect("job-1 run");
        assert!(
            matches!(
                finished,
                SessionRunOutcome::Finished(ref snapshot)
                    if snapshot.state == JobState::Complete
            ),
            "job-1 must complete once the runner is released: {finished:?}"
        );

        let mut waiting_persistence = BoundaryProbePersistence::new(lock_dir.clone());
        waiting_persistence
            .inner
            .jobs
            .insert("job-2".to_owned(), queued_job2);
        let waiting_boundary = SessionExecutionBoundary::new(
            service,
            waiting_persistence,
            LinuxCoordinator::new(&lock_dir).expect("coordinator"),
            PathBuf::from("/archive"),
        );
        assert!(executor.handle().runner_try_admit());
        let reservation = executor.handle().owned_runner_admission();
        let runner = thread::spawn(move || {
            let _reservation = reservation;
            let mut boundary = waiting_boundary;
            let outcome = boundary.execute_session("job-2");
            (boundary, outcome)
        });
        let (waiting_boundary, completed) = runner.join().expect("job-2 thread");
        assert!(
            matches!(completed, Ok(SessionRunOutcome::Finished(ref out))
            if out.state == JobState::Complete && out.job_id == "job-2"),
            "job-2 must complete after the runner is released: {completed:?}"
        );
        let final_snapshot = waiting_boundary
            .persistence
            .snapshot("job-2")
            .expect("snapshot");
        assert_eq!(final_snapshot.state, JobState::Complete);
        let final_current_attempt = waiting_boundary
            .persistence
            .current_attempt("job-2")
            .expect("attempt");
        assert_eq!(final_current_attempt, 1);
        let final_events = waiting_boundary
            .persistence
            .events("job-2")
            .expect("events");
        assert!(!final_events.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `complete_terminal_for_attempt` writes `Complete` state, `DownloadCompleted`,
    /// and `Completed` event in one persistence transaction.
    #[test]
    fn complete_terminal_for_attempt_is_atomic() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");

        let attempt = persistence.current_attempt("job-1").expect("attempt");
        assert!(
            persistence
                .persist_state_for_attempt("job-1", attempt, JobState::Queued, JobState::Validating)
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt,
                    JobState::Validating,
                    JobState::MetadataReady
                )
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt,
                    JobState::MetadataReady,
                    JobState::Downloading
                )
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt,
                    JobState::Downloading,
                    JobState::Downloaded
                )
                .expect("transition"),
        );
        assert_eq!(
            persistence.snapshot("job-1").expect("snapshot").state,
            JobState::Downloaded
        );

        let completed = persistence
            .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
            .expect("complete");
        assert_eq!(completed.state, JobState::Complete);

        let events = persistence.events("job-1").expect("events");
        let download_completed: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ExecutorEvent::DownloadCompleted { .. }))
            .collect();
        let completed_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ExecutorEvent::Completed { .. }))
            .collect();
        assert_eq!(download_completed.len(), 1, "exactly one DownloadCompleted");
        assert_eq!(completed_events.len(), 1, "exactly one Completed");
    }

    /// `complete_terminal_for_attempt` is idempotent: re-completing an already
    /// `Complete` job writes nothing and returns the current snapshot.
    #[test]
    fn complete_terminal_for_attempt_is_idempotent() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");

        let attempt = persistence.current_attempt("job-1").expect("attempt");
        assert!(
            persistence
                .persist_state_for_attempt("job-1", attempt, JobState::Queued, JobState::Validating)
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt,
                    JobState::Validating,
                    JobState::MetadataReady
                )
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt,
                    JobState::MetadataReady,
                    JobState::Downloading
                )
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt,
                    JobState::Downloading,
                    JobState::Downloaded
                )
                .expect("transition"),
        );

        let first = persistence
            .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
            .expect("first complete");
        assert_eq!(first.state, JobState::Complete);
        let events_after_first = persistence.events("job-1").expect("events");
        let completed_count = events_after_first
            .iter()
            .filter(|e| matches!(e, ExecutorEvent::Completed { .. }))
            .count();
        assert_eq!(completed_count, 1, "one Completed after first completion");

        let second = persistence
            .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
            .expect("second complete");
        assert_eq!(second.state, JobState::Complete);
        let events_after_second = persistence.events("job-1").expect("events");
        let completed_count_2 = events_after_second
            .iter()
            .filter(|e| matches!(e, ExecutorEvent::Completed { .. }))
            .count();
        assert_eq!(
            completed_count_2, 1,
            "still one Completed after idempotent re-completion: {events_after_second:?}"
        );
    }

    /// `complete_terminal_for_attempt` rejects a stale attempt: a newer attempt
    /// must not be able to complete an older job's terminal state.
    #[test]
    fn complete_terminal_for_attempt_rejects_stale_attempt() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");

        let attempt1 = persistence.current_attempt("job-1").expect("attempt");
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt1,
                    JobState::Queued,
                    JobState::Validating
                )
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt1,
                    JobState::Validating,
                    JobState::MetadataReady
                )
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt1,
                    JobState::MetadataReady,
                    JobState::Downloading
                )
                .expect("transition"),
        );
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt1,
                    JobState::Downloading,
                    JobState::Downloaded
                )
                .expect("transition"),
        );

        // A newer attempt claims the job and moves state away from Downloaded.
        let attempt2 = attempt1 + 1;
        persistence.begin_attempt("job-1").expect("begin attempt 2");
        assert!(
            persistence
                .persist_state_for_attempt(
                    "job-1",
                    attempt2,
                    JobState::Downloaded,
                    JobState::Failed
                )
                .expect("transition"),
        );

        let err = persistence
            .complete_terminal_for_attempt("job-1", attempt1, "Tweets/12345", 3)
            .unwrap_err();
        assert!(
            matches!(err, ExecutorError::Persistence(_)),
            "stale attempt must be rejected: {err:?}"
        );
        assert_eq!(
            persistence.snapshot("job-1").expect("snapshot").state,
            JobState::Failed,
            "stale attempt must not change state"
        );
    }

    /// `complete_terminal_for_attempt` fails closed when the job is not in
    /// `Downloaded` state (e.g. `Queued`, `Failed`, `Cancelled`).
    #[test]
    fn complete_terminal_for_attempt_requires_downloaded_state() {
        let mut persistence = StorageJobPersistence::open_in_memory().expect("storage");
        persistence
            .create_or_reuse(&request("job-1", "tweet-1"))
            .expect("job");

        let attempt = persistence.current_attempt("job-1").expect("attempt");
        // Job is still Queued; attempting to complete must fail.
        let err = persistence
            .complete_terminal_for_attempt("job-1", attempt, "Tweets/12345", 3)
            .unwrap_err();
        assert!(
            matches!(err, ExecutorError::Persistence(_)),
            "non-Downloaded state must be rejected: {err:?}"
        );
        assert_eq!(
            persistence.snapshot("job-1").expect("snapshot").state,
            JobState::Queued,
            "state must not change on rejected completion"
        );
    }
}
