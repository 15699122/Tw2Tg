//! Archive recovery manifest contracts: the persisted media/rename facts, the V1
//! and V2 manifest shapes, their validation and their digest.
//!
//! This module is pure. It performs no database I/O, opens no transaction and
//! holds no journal state; it only defines and validates the immutable payload
//! that the journal persists. The SQL, phase transitions and transaction
//! boundaries stay in `archive_recovery`.
//!
//! The crate root re-exports every public item, so call sites are unchanged.

use crate::StorageError;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashSet;
use std::path::{Component, Path};
use xarchive_core::ArchiveMetadata;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecoveryMediaFact {
    pub media_index: u32,
    pub relative_path: String,
    pub media_id: Option<String>,
    pub media_type: String,
    pub mime_type: Option<String>,
    pub size_bytes: u64,
    pub sha256: String,
}

/// Minimum identity and verified media facts; never contains user metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecoveryManifest {
    pub schema_version: u32,
    pub job_id: String,
    pub attempt_count: u32,
    pub tweet_row_id: i64,
    pub tweet_id: String,
    pub archive_directory: String,
    pub media: Vec<ArchiveRecoveryMediaFact>,
    pub telegram_tweet_row_id: Option<i64>,
}

/// Recoverable two-phase media path plan used when C1 naming is enabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRenamePlan {
    pub source_path: String,
    pub temporary_path: String,
    pub final_path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecoveryManifestV2 {
    /// Absent in historical manifests; presence explicitly selects D1 exports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_output: Option<MetadataExportSelection>,
    pub schema_version: u32,
    pub job_id: String,
    pub attempt_count: u32,
    pub tweet_row_id: i64,
    pub tweet_id: String,
    pub archive_directory: String,
    pub rename_phase: String,
    pub rename_plan: Vec<ArchiveRenamePlan>,
    pub media: Vec<ArchiveRecoveryMediaFact>,
    pub telegram_tweet_row_id: Option<i64>,
    pub export_json: bool,
    pub export_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataExportSelection {
    pub schema_version: u32,
    pub export_json: bool,
    pub export_yaml: bool,
}

impl ArchiveRecoveryManifestV2 {
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.metadata_output.as_ref().is_some_and(|settings| {
            settings.schema_version != 4
                || settings.export_json != self.export_json
                || self.export_text
        }) {
            return Err(StorageError::InvalidMetadata(
                "invalid D1 recovery output selection".into(),
            ));
        }
        for media in &self.media {
            if media.media_index == 0 {
                return Err(StorageError::InvalidMetadata(
                    "media index must be one-based".into(),
                ));
            }
        }
        if self.schema_version != 2
            || self.job_id.is_empty()
            || self.attempt_count == 0
            || self.tweet_row_id <= 0
            || xarchive_core::TweetId::new(self.tweet_id.clone()).is_err()
            || self.archive_directory != format!("Tweets/{}", self.tweet_id)
            || !matches!(
                self.rename_phase.as_str(),
                "PLANNED" | "TEMPORARY" | "FINAL"
            )
            || self
                .telegram_tweet_row_id
                .is_some_and(|id| id != self.tweet_row_id)
        {
            return Err(StorageError::InvalidMetadata(
                "invalid v2 archive recovery identity".into(),
            ));
        }
        let mut temporaries = HashSet::new();
        let mut finals = HashSet::new();
        let mut plans = std::collections::HashMap::new();
        let mut endpoints = HashSet::new();
        for plan in &self.rename_plan {
            validate_recovery_path(&plan.source_path)?;
            validate_recovery_path(&plan.temporary_path)?;
            validate_recovery_path(&plan.final_path)?;
            if plan.source_path.eq_ignore_ascii_case(&plan.temporary_path)
                || plan.temporary_path.eq_ignore_ascii_case(&plan.final_path)
            {
                return Err(StorageError::InvalidMetadata(
                    "v2 temporary paths must differ from both rename endpoints".into(),
                ));
            }
            let source = plan.source_path.to_lowercase();
            let temporary = plan.temporary_path.to_lowercase();
            let final_path = plan.final_path.to_lowercase();
            if !temporaries.insert(temporary)
                || !finals.insert(final_path.clone())
                || (endpoints.contains(&source) && source != final_path)
                || if source != final_path {
                    !endpoints.insert(source.clone()) || !endpoints.insert(final_path.clone())
                } else {
                    !endpoints.insert(source.clone())
                }
                || is_reserved_export_path(&source)
                || is_reserved_export_path(&plan.final_path)
                || plan.sha256.len() != 64
                || !plan.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                || plans.insert(plan.final_path.to_lowercase(), plan).is_some()
            {
                return Err(StorageError::InvalidMetadata(
                    "invalid v2 archive rename plan".into(),
                ));
            }
        }
        if temporaries.iter().any(|path| endpoints.contains(path))
            || temporaries.iter().any(|path| is_reserved_export_path(path))
        {
            return Err(StorageError::InvalidMetadata(
                "v2 temporary paths must be disjoint from source and final paths".into(),
            ));
        }
        let mut indexes = HashSet::new();
        let mut media_paths = HashSet::new();
        for media in &self.media {
            validate_recovery_path(&media.relative_path)?;
            if !indexes.insert(media.media_index)
                || !media_paths.insert(media.relative_path.to_lowercase())
                || media.sha256.len() != 64
                || !media.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(StorageError::InvalidMetadata(
                    "invalid v2 archive media facts".into(),
                ));
            }
            let plan = plans
                .get(&media.relative_path.to_lowercase())
                .ok_or_else(|| {
                    StorageError::InvalidMetadata("v2 media has no rename plan".into())
                })?;
            if plan.size_bytes != media.size_bytes || plan.sha256 != media.sha256 {
                return Err(StorageError::InvalidMetadata(
                    "v2 rename plan does not match media identity".into(),
                ));
            }
        }
        if self.rename_plan.len() != self.media.len() {
            return Err(StorageError::InvalidMetadata(
                "v2 rename plan/media count mismatch".into(),
            ));
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, StorageError> {
        Ok(format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(self)?)
        ))
    }
}

fn is_reserved_export_path(path: &str) -> bool {
    [
        "tweet.json",
        "tweet.txt",
        "tweet.yaml",
        ".xarchive-recovery.json",
        ".xarchive-recovery-v2.json",
    ]
    .iter()
    .any(|reserved| path.eq_ignore_ascii_case(reserved))
}

fn validate_recovery_path(value: &str) -> Result<(), StorageError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(StorageError::InvalidMetadata(
            "unsafe archive recovery path".into(),
        ));
    }
    Ok(())
}

impl ArchiveRecoveryManifest {
    pub fn validate(&self) -> Result<(), StorageError> {
        let archive_parts = self.archive_directory.split('/').collect::<Vec<_>>();
        if self.schema_version != 1
            || self.job_id.is_empty()
            || self.attempt_count == 0
            || self.tweet_row_id <= 0
            || xarchive_core::TweetId::new(self.tweet_id.clone()).is_err()
            || !self.archive_directory.starts_with("Tweets/")
            || self.archive_directory.contains('\\')
            || self.archive_directory.contains(':')
            || archive_parts.len() != 2
            || archive_parts[0] != "Tweets"
            || archive_parts[1] != self.tweet_id
            || Path::new(&self.archive_directory).components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
            || self
                .telegram_tweet_row_id
                .is_some_and(|id| id != self.tweet_row_id)
        {
            return Err(StorageError::InvalidMetadata(
                "invalid archive recovery manifest identity".into(),
            ));
        }
        let (mut indexes, mut paths) = (HashSet::new(), HashSet::new());
        for media in &self.media {
            if media.media_index == 0 {
                return Err(StorageError::InvalidMetadata(
                    "media index must be one-based".into(),
                ));
            }
            let path = Path::new(&media.relative_path);
            let windows_path = media.relative_path.replace('\\', "/");
            if media.relative_path.is_empty()
                || media.relative_path.contains('\\')
                || media.relative_path.contains(':')
                || path.is_absolute()
                || path.components().any(|part| {
                    matches!(
                        part,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
                || !indexes.insert(media.media_index)
                || !paths.insert(windows_path.to_lowercase())
                || media.sha256.len() != 64
                || !media.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(StorageError::InvalidMetadata(
                    "invalid archive recovery media facts".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn from_metadata(
        job_id: &str,
        attempt_count: u32,
        tweet_row_id: i64,
        archive_directory: &Path,
        metadata: &ArchiveMetadata,
        telegram_tweet_row_id: Option<i64>,
    ) -> Result<Self, StorageError> {
        let manifest = Self {
            schema_version: 1,
            job_id: job_id.to_owned(),
            attempt_count,
            tweet_row_id,
            tweet_id: metadata.tweet_id.clone(),
            archive_directory: archive_directory.to_string_lossy().replace('\\', "/"),
            media: metadata
                .media
                .iter()
                .map(|item| ArchiveRecoveryMediaFact {
                    media_index: item.index,
                    relative_path: item.file.clone(),
                    media_id: item.media_id.clone(),
                    media_type: item.media_type.clone(),
                    mime_type: item.mime_type.clone(),
                    size_bytes: item.size_bytes,
                    sha256: item.sha256.clone(),
                })
                .collect(),
            telegram_tweet_row_id,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn digest(&self) -> Result<String, StorageError> {
        Ok(format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(self)?)
        ))
    }
}
