use crate::{Database, StorageError};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashSet;
use std::path::{Component, Path};
use xarchive_core::ArchiveMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRecoveryContract {
    InternalV1,
    InternalRenameV2,
}

impl ArchiveRecoveryContract {
    pub const fn version(self) -> i64 {
        match self {
            Self::InternalV1 => 1,
            Self::InternalRenameV2 => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRecoveryPhase {
    Prepared,
    Committed,
}

impl ArchiveRecoveryPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "PREPARED",
            Self::Committed => "COMMITTED",
        }
    }
}

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

impl ArchiveRecoveryManifestV2 {
    pub fn validate(&self) -> Result<(), StorageError> {
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
        let mut sources = HashSet::new();
        let mut temporaries = HashSet::new();
        let mut finals = HashSet::new();
        let mut plans = std::collections::HashMap::new();
        for plan in &self.rename_plan {
            validate_recovery_path(&plan.source_path)?;
            validate_recovery_path(&plan.temporary_path)?;
            validate_recovery_path(&plan.final_path)?;
            let source = plan.source_path.to_lowercase();
            let temporary = plan.temporary_path.to_lowercase();
            let final_path = plan.final_path.to_lowercase();
            if !sources.insert(source)
                || !temporaries.insert(temporary)
                || !finals.insert(final_path)
                || plan.sha256.len() != 64
                || !plan.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                || plans.insert(plan.final_path.to_lowercase(), plan).is_some()
            {
                return Err(StorageError::InvalidMetadata(
                    "invalid v2 archive rename plan".into(),
                ));
            }
        }
        if temporaries
            .iter()
            .any(|path| sources.contains(path) || finals.contains(path))
            || sources.iter().any(|path| is_reserved_export_path(path))
            || temporaries.iter().any(|path| is_reserved_export_path(path))
            || finals.iter().any(|path| is_reserved_export_path(path))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveRecoveryJournalRecord {
    pub job_id: String,
    pub attempt_count: u32,
    pub tweet_row_id: i64,
    pub archive_directory: String,
    pub manifest_schema_version: u32,
    pub manifest_json: String,
    pub manifest_sha256: String,
    pub telegram_intent_required: bool,
    pub telegram_tweet_row_id: Option<i64>,
    pub phase: ArchiveRecoveryPhase,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveRecoveryManifestV2Record {
    pub job_id: String,
    pub attempt_count: u32,
    pub tweet_row_id: i64,
    pub archive_directory: String,
    pub manifest_json: String,
    pub manifest_sha256: String,
    pub phase: ArchiveRecoveryPhase,
    pub created_at: String,
    pub updated_at: String,
}

impl Database {
    pub fn archive_recovery_contract(&self, job_id: &str) -> Result<Option<i64>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT recovery_contract_version FROM jobs WHERE id = ?1",
                [job_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten())
    }

    pub fn create_archive_recovery_manifest_v2(
        &self,
        manifest: &ArchiveRecoveryManifestV2,
        now: &str,
    ) -> Result<ArchiveRecoveryManifestV2Record, StorageError> {
        manifest.validate()?;
        if self.archive_recovery_contract(&manifest.job_id)?
            != Some(ArchiveRecoveryContract::InternalRenameV2.version())
            || self.archive_attempt(&manifest.job_id)? != manifest.attempt_count
            || self.tweet_external_id(manifest.tweet_row_id)?.as_deref()
                != Some(manifest.tweet_id.as_str())
            || self
                .job_summary(&manifest.job_id)?
                .is_none_or(|job| job.tweet_id != manifest.tweet_id)
        {
            return Err(StorageError::InvalidState(
                "job is not enrolled in this v2 recovery attempt".into(),
            ));
        }
        if let Some(row_id) = manifest.telegram_tweet_row_id {
            let intent = self.telegram_archive_intent(row_id)?;
            if intent.as_ref().is_none_or(|intent| {
                intent.job_id != manifest.job_id
                    || intent.archive_directory != manifest.archive_directory
                    || intent.tweet_row_id != manifest.tweet_row_id
            }) {
                return Err(StorageError::InvalidState(
                    "matching Telegram intent must be persisted before v2 recovery manifest".into(),
                ));
            }
        }
        let manifest_json = serde_json::to_string(manifest)?;
        let digest = manifest.digest()?;
        self.connection.execute(
            "INSERT INTO archive_recovery_v2 (job_id, attempt_count, tweet_row_id, archive_directory, manifest_json, manifest_sha256, phase, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'PREPARED', ?7, ?7) ON CONFLICT(job_id) DO NOTHING",
            rusqlite::params![manifest.job_id, manifest.attempt_count, manifest.tweet_row_id, manifest.archive_directory, manifest_json, digest, now],
        )?;
        let stored = self
            .archive_recovery_manifest_v2(&manifest.job_id)?
            .ok_or_else(|| {
                StorageError::InvalidState("v2 recovery journal insert missing".into())
            })?;
        if stored.attempt_count != manifest.attempt_count
            || stored.manifest_json != manifest_json
            || stored.manifest_sha256 != digest
        {
            return Err(StorageError::InvalidState(
                "v2 recovery manifest already exists with different facts".into(),
            ));
        }
        Ok(stored)
    }

    pub fn archive_recovery_manifest_v2(
        &self,
        job_id: &str,
    ) -> Result<Option<ArchiveRecoveryManifestV2Record>, StorageError> {
        let record = self
            .connection
            .query_row(
                "SELECT job_id, attempt_count, tweet_row_id, archive_directory, manifest_json, manifest_sha256, phase, created_at, updated_at FROM archive_recovery_v2 WHERE job_id = ?1",
                [job_id],
                archive_recovery_v2_record_from_row,
            )
            .optional()?;
        let Some(record) = record else {
            return Ok(None);
        };
        let manifest: ArchiveRecoveryManifestV2 = serde_json::from_str(&record.manifest_json)?;
        manifest.validate()?;
        if manifest.job_id != record.job_id
            || manifest.attempt_count != record.attempt_count
            || manifest.tweet_row_id != record.tweet_row_id
            || manifest.archive_directory != record.archive_directory
            || manifest.digest()? != record.manifest_sha256
            || (record.phase == ArchiveRecoveryPhase::Committed && manifest.rename_phase != "FINAL")
            || self.archive_recovery_contract(job_id)?
                != Some(ArchiveRecoveryContract::InternalRenameV2.version())
            || self.archive_attempt(job_id)? != record.attempt_count
            || self.tweet_external_id(record.tweet_row_id)?.as_deref()
                != Some(manifest.tweet_id.as_str())
            || self
                .job_summary(job_id)?
                .is_none_or(|job| job.tweet_id != manifest.tweet_id)
        {
            return Err(StorageError::InvalidMetadata(
                "v2 recovery journal integrity or identity mismatch".into(),
            ));
        }
        if let Some(row_id) = manifest.telegram_tweet_row_id {
            let intent = self.telegram_archive_intent(row_id)?.ok_or_else(|| {
                StorageError::InvalidState("required v2 Telegram intent missing".into())
            })?;
            if intent.job_id != record.job_id
                || intent.archive_directory != record.archive_directory
                || intent.tweet_row_id != record.tweet_row_id
            {
                return Err(StorageError::InvalidMetadata(
                    "v2 Telegram intent identity mismatch".into(),
                ));
            }
        }
        Ok(Some(record))
    }

    pub fn transition_archive_recovery_v2(
        &self,
        job_id: &str,
        expected: &str,
        next: &str,
        now: &str,
    ) -> Result<ArchiveRecoveryManifestV2Record, StorageError> {
        let valid = matches!(
            (expected, next),
            ("PLANNED", "TEMPORARY") | ("TEMPORARY", "FINAL") | ("FINAL", "COMMITTED")
        );
        if !valid {
            return Err(StorageError::InvalidState(
                "invalid v2 recovery phase transition".into(),
            ));
        }
        let current = self
            .archive_recovery_manifest_v2(job_id)?
            .ok_or_else(|| StorageError::InvalidState("v2 recovery journal missing".into()))?;
        let mut manifest: ArchiveRecoveryManifestV2 = serde_json::from_str(&current.manifest_json)?;
        let stored_phase = if current.phase == ArchiveRecoveryPhase::Committed {
            "COMMITTED"
        } else {
            "PREPARED"
        };
        if stored_phase == "COMMITTED" && next == "COMMITTED" && manifest.rename_phase == "FINAL" {
            return Ok(current);
        }
        if stored_phase != "PREPARED" || manifest.rename_phase != expected {
            if stored_phase == "PREPARED" && manifest.rename_phase == next {
                return Ok(current);
            }
            return Err(StorageError::InvalidState(
                "v2 recovery phase compare-and-set rejected".into(),
            ));
        }
        let committed = next == "COMMITTED";
        manifest.rename_phase = if committed { "FINAL" } else { next }.to_owned();
        let json = serde_json::to_string(&manifest)?;
        let digest = manifest.digest()?;
        let changed = self.connection.execute(
            "UPDATE archive_recovery_v2 SET manifest_json = ?1, manifest_sha256 = ?2, phase = ?3, updated_at = ?4 WHERE job_id = ?5 AND phase = ?6",
            rusqlite::params![json, digest, if committed { "COMMITTED" } else { "PREPARED" }, now, job_id, "PREPARED"],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidState(
                "v2 recovery phase compare-and-set lost".into(),
            ));
        }
        self.archive_recovery_manifest_v2(job_id)?.ok_or_else(|| {
            StorageError::InvalidState("v2 recovery journal disappeared after transition".into())
        })
    }

    pub fn create_archive_recovery_manifest(
        &self,
        manifest: &ArchiveRecoveryManifest,
        now: &str,
    ) -> Result<(), StorageError> {
        manifest.validate()?;
        let json = serde_json::to_string(manifest)?;
        let digest = manifest.digest()?;
        if self.archive_recovery_contract(&manifest.job_id)?
            != Some(ArchiveRecoveryContract::InternalV1.version())
            || self.archive_attempt(&manifest.job_id)? != manifest.attempt_count
        {
            return Err(StorageError::InvalidState(
                "job is not enrolled in this recovery attempt".into(),
            ));
        }
        if manifest.telegram_tweet_row_id.is_some() {
            let intent = self.telegram_archive_intent(manifest.tweet_row_id)?;
            if intent.as_ref().is_none_or(|intent| {
                intent.job_id != manifest.job_id
                    || intent.archive_directory != manifest.archive_directory
            }) {
                return Err(StorageError::InvalidState(
                    "Telegram intent must be persisted before recovery manifest".into(),
                ));
            }
        }
        let (intent_required, intent_row_id) = (
            manifest.telegram_tweet_row_id.is_some(),
            manifest.telegram_tweet_row_id,
        );
        self.connection.execute(
            "INSERT INTO archive_recovery_journal (job_id, attempt_count, tweet_row_id, archive_directory, manifest_schema_version, manifest_json, manifest_sha256, telegram_intent_required, telegram_tweet_row_id, phase, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'PREPARED', ?10, ?10) ON CONFLICT(job_id) DO NOTHING",
            rusqlite::params![manifest.job_id, manifest.attempt_count, manifest.tweet_row_id, manifest.archive_directory, manifest.schema_version, json, digest, intent_required, intent_row_id, now],
        )?;
        let stored = self
            .archive_recovery_journal(&manifest.job_id)?
            .ok_or_else(|| StorageError::InvalidState("recovery journal insert missing".into()))?;
        if stored.manifest_json != json
            || stored.manifest_sha256 != digest
            || stored.attempt_count != manifest.attempt_count
        {
            return Err(StorageError::InvalidState(
                "recovery manifest already exists with different facts".into(),
            ));
        }
        Ok(())
    }

    pub fn archive_recovery_journal(
        &self,
        job_id: &str,
    ) -> Result<Option<ArchiveRecoveryJournalRecord>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT job_id, attempt_count, tweet_row_id, archive_directory, manifest_schema_version, manifest_json, manifest_sha256, telegram_intent_required, telegram_tweet_row_id, phase, created_at, updated_at FROM archive_recovery_journal WHERE job_id = ?1",
                [job_id],
                journal_from_row,
            )
            .optional()?)
    }

    pub fn verify_archive_recovery_manifest(
        &self,
        job_id: &str,
    ) -> Result<ArchiveRecoveryManifest, StorageError> {
        let record = self
            .archive_recovery_journal(job_id)?
            .ok_or_else(|| StorageError::InvalidState("internal recovery record missing".into()))?;
        let manifest: ArchiveRecoveryManifest = serde_json::from_str(&record.manifest_json)?;
        manifest.validate()?;
        if manifest.schema_version != record.manifest_schema_version
            || manifest.job_id != record.job_id
            || manifest.attempt_count != record.attempt_count
            || manifest.tweet_row_id != record.tweet_row_id
            || manifest.archive_directory != record.archive_directory
            || manifest.telegram_tweet_row_id != record.telegram_tweet_row_id
            || record.telegram_intent_required != record.telegram_tweet_row_id.is_some()
            || manifest.digest()? != record.manifest_sha256
            || self.tweet_external_id(manifest.tweet_row_id)?.as_deref() != Some(&manifest.tweet_id)
            || self
                .job_summary(job_id)?
                .is_none_or(|job| job.tweet_id != manifest.tweet_id)
        {
            return Err(StorageError::InvalidMetadata(
                "internal recovery record integrity or identity mismatch".into(),
            ));
        }
        if let Some(row_id) = manifest.telegram_tweet_row_id {
            let intent = self.telegram_archive_intent(row_id)?.ok_or_else(|| {
                StorageError::InvalidState("required Telegram archive intent missing".into())
            })?;
            if intent.job_id != manifest.job_id
                || intent.archive_directory != manifest.archive_directory
            {
                return Err(StorageError::InvalidMetadata(
                    "Telegram intent mismatch".into(),
                ));
            }
        }
        Ok(manifest)
    }

    pub fn list_prepared_archive_recovery_manifests(
        &self,
    ) -> Result<Vec<ArchiveRecoveryManifest>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT job_id FROM archive_recovery_journal WHERE phase = 'PREPARED' ORDER BY updated_at, job_id",
        )?;
        let jobs = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        jobs.iter()
            .map(|job| self.verify_archive_recovery_manifest(job))
            .collect()
    }

    pub fn commit_archive_recovery_manifest(
        &self,
        job_id: &str,
        now: &str,
    ) -> Result<(), StorageError> {
        let changed = self.connection.execute(
            "UPDATE archive_recovery_journal SET phase = 'COMMITTED', updated_at = ?2 WHERE job_id = ?1 AND phase = 'PREPARED'",
            rusqlite::params![job_id, now],
        )?;
        if changed == 0
            && self
                .archive_recovery_journal(job_id)?
                .is_none_or(|record| record.phase != ArchiveRecoveryPhase::Committed)
        {
            return Err(StorageError::InvalidState(
                "recovery journal transition rejected".into(),
            ));
        }
        Ok(())
    }
}

fn journal_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArchiveRecoveryJournalRecord> {
    let phase: String = row.get(9)?;
    let phase = match phase.as_str() {
        "PREPARED" => ArchiveRecoveryPhase::Prepared,
        "COMMITTED" => ArchiveRecoveryPhase::Committed,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(ArchiveRecoveryJournalRecord {
        job_id: row.get(0)?,
        attempt_count: row.get::<_, i64>(1)?.max(0) as u32,
        tweet_row_id: row.get(2)?,
        archive_directory: row.get(3)?,
        manifest_schema_version: row.get::<_, i64>(4)?.max(0) as u32,
        manifest_json: row.get(5)?,
        manifest_sha256: row.get(6)?,
        telegram_intent_required: row.get(7)?,
        telegram_tweet_row_id: row.get(8)?,
        phase,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn archive_recovery_v2_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ArchiveRecoveryManifestV2Record> {
    let phase: String = row.get(6)?;
    let phase = match phase.as_str() {
        "PREPARED" | "COMMITTED" => match phase.as_str() {
            "COMMITTED" => ArchiveRecoveryPhase::Committed,
            _ => ArchiveRecoveryPhase::Prepared,
        },
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(ArchiveRecoveryManifestV2Record {
        job_id: row.get(0)?,
        attempt_count: row.get::<_, i64>(1)?.max(0) as u32,
        tweet_row_id: row.get(2)?,
        archive_directory: row.get(3)?,
        manifest_json: row.get(4)?,
        manifest_sha256: row.get(5)?,
        phase,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ArchiveRecoveryManifest {
        ArchiveRecoveryManifest {
            schema_version: 1,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id: 1,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            media: vec![ArchiveRecoveryMediaFact {
                media_index: 1,
                relative_path: "media/01.jpg".into(),
                media_id: Some("media-1".into()),
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            telegram_tweet_row_id: None,
        }
    }

    #[test]
    fn manifest_is_strictly_versioned_and_contains_no_user_metadata() {
        let manifest = manifest();
        manifest.validate().expect("valid manifest");
        let json = serde_json::to_value(&manifest).expect("json");
        assert!(json.get("text").is_none());
        assert!(json.get("author").is_none());
        let mut invalid = manifest;
        invalid.schema_version = 2;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn rejects_traversal_duplicate_and_invalid_hash_media_facts() {
        let mut value = manifest();
        value.media[0].relative_path = "../outside.jpg".into();
        assert!(value.validate().is_err());
        let mut value = manifest();
        value.media[0].relative_path = r"..\outside.jpg".into();
        assert!(value.validate().is_err());
        let mut value = manifest();
        value.media.push(value.media[0].clone());
        assert!(value.validate().is_err());
        let mut value = manifest();
        value.media[0].relative_path = "media/File.jpg".into();
        value.media.push(ArchiveRecoveryMediaFact {
            relative_path: "media/file.jpg".into(),
            ..value.media[0].clone()
        });
        assert!(value.validate().is_err());
        let mut value = manifest();
        value.media[0].sha256 = "xyz".into();
        assert!(value.validate().is_err());
    }

    #[test]
    fn rejects_unknown_fields_in_manifest_wire_format() {
        let json = r#"{"schema_version":1,"job_id":"job-1","attempt_count":1,"tweet_row_id":1,"tweet_id":"123","archive_directory":"Tweets/123","media":[],"telegram_tweet_row_id":null,"text":"forbidden"}"#;
        assert!(serde_json::from_str::<ArchiveRecoveryManifest>(json).is_err());
    }

    #[test]
    fn v2_rename_plan_rejects_path_escape_and_duplicate_temporary_names() {
        let fact = ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "final.jpg".into(),
            media_id: None,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        };
        let base = ArchiveRecoveryManifestV2 {
            schema_version: 2,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id: 1,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            rename_phase: "PLANNED".into(),
            rename_plan: vec![ArchiveRenamePlan {
                source_path: "source.jpg".into(),
                temporary_path: ".xarchive-rename-01.tmp".into(),
                final_path: "final.jpg".into(),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            media: vec![fact],
            telegram_tweet_row_id: None,
            export_json: true,
            export_text: true,
        };
        base.validate().expect("valid v2 plan");
        let mut traversal = base.clone();
        traversal.rename_plan[0].temporary_path = "../outside".into();
        assert!(traversal.validate().is_err());
        let mut duplicate_temp = base;
        duplicate_temp.rename_plan.push(ArchiveRenamePlan {
            source_path: "other.jpg".into(),
            temporary_path: ".xarchive-rename-01.tmp".into(),
            final_path: "other-final.jpg".into(),
            size_bytes: 3,
            sha256: "a".repeat(64),
        });
        duplicate_temp.media.push(ArchiveRecoveryMediaFact {
            media_index: 2,
            relative_path: "other-final.jpg".into(),
            media_id: None,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        });
        assert!(duplicate_temp.validate().is_err());
    }

    #[test]
    fn v2_plan_rejects_cross_role_path_collisions_and_mismatched_media_facts() {
        let base = ArchiveRecoveryManifestV2 {
            schema_version: 2,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id: 1,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            rename_phase: "PLANNED".into(),
            rename_plan: vec![ArchiveRenamePlan {
                source_path: "source.jpg".into(),
                temporary_path: ".tmp.jpg".into(),
                final_path: "final.jpg".into(),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            media: vec![ArchiveRecoveryMediaFact {
                media_index: 1,
                relative_path: "final.jpg".into(),
                media_id: None,
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            telegram_tweet_row_id: None,
            export_json: true,
            export_text: true,
        };
        base.validate().expect("valid plan");

        let mut temp_collides_with_source = base.clone();
        temp_collides_with_source.rename_plan[0].temporary_path = "source.jpg".into();
        assert!(temp_collides_with_source.validate().is_err());

        let mut media_mismatch = base;
        media_mismatch.media[0].sha256 = "b".repeat(64);
        assert!(media_mismatch.validate().is_err());
    }

    #[test]
    fn v2_journal_is_idempotent_strict_and_has_monotonic_phase_transitions() {
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                2,
                "request-1",
                "{}",
                Some(ArchiveRecoveryContract::InternalRenameV2),
                "now",
            )
            .expect("job");
        database
            .begin_archive_attempt("job-1", "now")
            .expect("attempt");
        let media = ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "final.jpg".into(),
            media_id: None,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        };
        let manifest = ArchiveRecoveryManifestV2 {
            schema_version: 2,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            rename_phase: "PLANNED".into(),
            rename_plan: vec![ArchiveRenamePlan {
                source_path: "source.jpg".into(),
                temporary_path: ".xarchive-rename-01.tmp".into(),
                final_path: "final.jpg".into(),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            media: vec![media],
            telegram_tweet_row_id: None,
            export_json: true,
            export_text: true,
        };
        let first = database
            .create_archive_recovery_manifest_v2(&manifest, "now")
            .expect("persist v2 manifest");
        assert_eq!(first.phase, ArchiveRecoveryPhase::Prepared);
        assert_eq!(
            database
                .create_archive_recovery_manifest_v2(&manifest, "later")
                .expect("idempotent retry"),
            first
        );
        database
            .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "t1")
            .expect("temporary phase");
        database
            .transition_archive_recovery_v2("job-1", "TEMPORARY", "FINAL", "t2")
            .expect("final phase");
        database
            .transition_archive_recovery_v2("job-1", "FINAL", "COMMITTED", "t3")
            .expect("commit phase");
        let committed = database
            .archive_recovery_manifest_v2("job-1")
            .expect("read record")
            .expect("record");
        assert_eq!(committed.phase, ArchiveRecoveryPhase::Committed);
        assert!(
            database
                .transition_archive_recovery_v2("job-1", "PLANNED", "TEMPORARY", "t4")
                .is_err()
        );
        let mut conflicting = manifest;
        conflicting.rename_plan[0].final_path = "other.jpg".into();
        conflicting.media[0].relative_path = "other.jpg".into();
        assert!(
            database
                .create_archive_recovery_manifest_v2(&conflicting, "later")
                .is_err()
        );
    }

    #[test]
    fn recovery_rejects_missing_or_corrupt_internal_manifest_and_changed_media() {
        for damage in ["missing-manifest", "corrupt-manifest", "changed-media"] {
            let root = std::env::temp_dir().join(format!(
                "xarchive-recovery-reject-{}-{damage}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            let mut database = Database::open_in_memory().expect("database");
            let tweet_row_id = database
                .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
                .expect("tweet");
            database
                .create_archive_job_with_optional_recovery_contract(
                    "job-1",
                    tweet_row_id,
                    2,
                    "request-1",
                    "{}",
                    Some(ArchiveRecoveryContract::InternalV1),
                    "now",
                )
                .expect("job");
            database
                .begin_archive_attempt("job-1", "now")
                .expect("attempt");
            let files = crate::FileStore::new(&root).expect("files");
            let staging = files.staging_dir("job-1").expect("staging");
            std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
            let metadata = xarchive_core::ArchiveMedia {
                index: 1,
                media_id: None,
                media_type: "photo".into(),
                file: "01.jpg".into(),
                mime_type: None,
                size_bytes: 3,
                sha256: crate::FileStore::sha256(staging.join("01.jpg")).expect("hash"),
            };
            let manifest = ArchiveRecoveryManifest {
                schema_version: 1,
                job_id: "job-1".into(),
                attempt_count: 1,
                tweet_row_id,
                tweet_id: "123".into(),
                archive_directory: "Tweets/123".into(),
                media: vec![ArchiveRecoveryMediaFact {
                    media_index: metadata.index,
                    relative_path: metadata.file.clone(),
                    media_id: metadata.media_id.clone(),
                    media_type: metadata.media_type.clone(),
                    mime_type: metadata.mime_type.clone(),
                    size_bytes: metadata.size_bytes,
                    sha256: metadata.sha256.clone(),
                }],
                telegram_tweet_row_id: None,
            };
            database
                .create_archive_recovery_manifest(&manifest, "now")
                .expect("journal");
            if damage != "missing-manifest" {
                let contents = if damage == "corrupt-manifest" {
                    b"{}".to_vec()
                } else {
                    serde_json::to_vec(&manifest).expect("serialize")
                };
                std::fs::write(staging.join(".xarchive-recovery.json"), contents)
                    .expect("manifest file");
            }
            if damage == "changed-media" {
                std::fs::write(staging.join("01.jpg"), b"xyz").expect("replace media");
            }
            let mut service = crate::ArchiveService::new(database, files);
            assert!(
                service
                    .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
                    .is_err(),
                "damage kind should fail: {damage}"
            );
            assert_eq!(
                service.database.job_state("job-1").expect("state"),
                xarchive_core::JobState::Queued
            );
            assert!(!root.join("Tweets/123").exists());
            let _ = std::fs::remove_dir_all(root);
        }
    }

    #[test]
    fn unsupported_recovery_contract_fails_closed_without_export_fallback() {
        let root = std::env::temp_dir().join(format!(
            "xarchive-recovery-unknown-contract-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                2,
                "request-1",
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job");
        database
            .begin_archive_attempt("job-1", "now")
            .expect("attempt");
        database
            .connection
            .execute(
                "UPDATE jobs SET recovery_contract_version = 99 WHERE id = 'job-1'",
                [],
            )
            .expect("install unknown contract version");

        let files = crate::FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        std::fs::write(staging.join("tweet.json"), b"{}")
            .expect("legacy export exists to prove it is not used");
        std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
        let mut service = crate::ArchiveService::new(database, files);

        let error = service
            .recover_staging_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
            .expect_err("unknown contract must not recover through exported metadata");
        assert!(
            error
                .to_string()
                .contains("unsupported archive recovery contract")
        );
        assert_eq!(
            service
                .database
                .job_state("job-1")
                .expect("state unchanged"),
            xarchive_core::JobState::Queued
        );
        assert!(!root.join("Tweets/123").exists());
        assert!(staging.join("01.jpg").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    /// Intent linkage: a manifest that records a Telegram intent transitions it
    /// PREPARED -> ARCHIVED during recovery; a manifest whose recorded intent
    /// row is missing fails verification instead of guessing a target.
    #[test]
    fn internal_recovery_transitions_linked_intent_and_rejects_missing_intent() {
        let root = std::env::temp_dir().join(format!(
            "xarchive-recovery-intent-linkage-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                2,
                "request-1",
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job");
        database
            .begin_archive_attempt("job-1", "now")
            .expect("attempt");
        let intent = crate::TelegramArchiveIntentRecord {
            job_id: "job-1".into(),
            tweet_row_id,
            archive_directory: "Tweets/123".into(),
            state: "PREPARED".into(),
            metadata_text: None,
            media_json: "[]".into(),
            bot_identity: Some("bot".into()),
            chat_id: Some("-100".into()),
            message_thread_id: None,
            config_revision: Some(1),
            plan_version: 1,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        database
            .prepare_telegram_archive_intent(&intent)
            .expect("prepare intent");
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: "123".into(),
            url: "https://x.com/a/status/123".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: None,
                display_name: None,
            },
            created_at: None,
            text: "intent linkage".into(),
            media: vec![xarchive_core::ArchiveMedia {
                index: 1,
                media_id: Some("m1".into()),
                media_type: "photo".into(),
                file: "01.jpg".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            }],
            archived_at: "now".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        let manifest = ArchiveRecoveryManifest::from_metadata(
            "job-1",
            1,
            tweet_row_id,
            Path::new("Tweets/123"),
            &metadata,
            Some(tweet_row_id),
        )
        .expect("manifest with intent linkage");
        let files = crate::FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
        std::fs::write(
            staging.join(".xarchive-recovery.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("manifest file");
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("journal");

        let mut service = crate::ArchiveService::new(database, files);
        service
            .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
            .expect("recover with linked intent");
        assert_eq!(
            service
                .database
                .telegram_archive_intent(tweet_row_id)
                .expect("intent query")
                .expect("intent row")
                .state,
            "ARCHIVED"
        );

        // The journal linkage must keep verifying while its intent row exists...
        assert!(
            service
                .database
                .verify_archive_recovery_manifest("job-1")
                .is_ok(),
            "committed manifest still verifies"
        );
        // ...and fail explicitly once the recorded intent row is gone. The journal
        // FK normally prevents this state; disabling FK on this connection
        // simulates an externally altered/legacy database to prove verification
        // still fails closed instead of guessing an intent target.
        service
            .database
            .connection
            .execute("PRAGMA foreign_keys = OFF", [])
            .expect("disable fk for fixture");
        service
            .database
            .connection
            .execute("DELETE FROM telegram_archive_intents", [])
            .expect("remove intent");
        assert!(
            service
                .database
                .verify_archive_recovery_manifest("job-1")
                .is_err()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn crashing_between_manifest_writes_leaves_a_repeatable_journal_row() {
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                2,
                "request-1",
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job");
        database
            .begin_archive_attempt("job-1", "now")
            .expect("attempt");
        let manifest = ArchiveRecoveryManifest {
            schema_version: 1,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            media: vec![ArchiveRecoveryMediaFact {
                media_index: 1,
                relative_path: "01.jpg".into(),
                media_id: Some("m1".into()),
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            telegram_tweet_row_id: None,
        };
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("first prepared write");
        // Crash/retry between journal writes is a no-op: the original row survives.
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("repeat prepared write");
        assert_eq!(
            database
                .verify_archive_recovery_manifest("job-1")
                .expect("verify"),
            manifest
        );
        assert_eq!(
            database
                .archive_recovery_journal("job-1")
                .expect("journal")
                .expect("journal row")
                .phase,
            ArchiveRecoveryPhase::Prepared
        );
        // A conflicting manifest never displaces the prepared row.
        let mut conflicted = manifest.clone();
        conflicted.archive_directory = "Tweets/999".into();
        assert!(
            database
                .create_archive_recovery_manifest(&conflicted, "now")
                .is_err()
        );
        assert_eq!(
            database
                .verify_archive_recovery_manifest("job-1")
                .expect("original survives"),
            manifest
        );
    }

    #[test]
    fn post_rename_database_conflict_rolls_back_entire_recovery_transaction() {
        let root = std::env::temp_dir().join(format!(
            "xarchive-recovery-row-conflict-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                2,
                "request-1",
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job");
        database
            .begin_archive_attempt("job-1", "now")
            .expect("attempt");
        // A pre-existing media row with the same index but incompatible facts makes
        // the guarded recovery transaction fail after filesystem rename. The whole
        // transaction must roll back: no partial directory facts, no COMMITTED.
        database
            .insert_media(
                tweet_row_id,
                &xarchive_core::ArchiveMedia {
                    index: 1,
                    media_id: Some("other".into()),
                    media_type: "photo".into(),
                    file: "01.jpg".into(),
                    mime_type: Some("image/jpeg".into()),
                    size_bytes: 3,
                    sha256: "b".repeat(64),
                },
                "now",
            )
            .expect("conflicting media row");
        let manifest = ArchiveRecoveryManifest {
            schema_version: 1,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            media: vec![ArchiveRecoveryMediaFact {
                media_index: 1,
                relative_path: "01.jpg".into(),
                media_id: Some("m1".into()),
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                // sha256 of the fixture bytes b"abc" so media integrity passes and
                // the failure lands on the DB recovery boundary after rename.
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            }],
            telegram_tweet_row_id: None,
        };
        let files = crate::FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        std::fs::write(staging.join("01.jpg"), b"abc").expect("staging media");
        std::fs::write(
            staging.join(".xarchive-recovery.json"),
            serde_json::to_vec(&manifest).expect("manifest"),
        )
        .expect("manifest file");
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("journal");

        let mut service = crate::ArchiveService::new(database, files);
        assert!(
            service
                .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
                .is_err()
        );
        assert_eq!(
            service
                .database
                .archive_recovery_journal("job-1")
                .expect("journal")
                .expect("journal row")
                .phase,
            ArchiveRecoveryPhase::Prepared
        );
        assert!(
            service
                .database
                .tweet_archive_facts("123")
                .expect("facts query")
                .is_none()
        );
        assert_eq!(
            service.database.job_state("job-1").expect("job state"),
            xarchive_core::JobState::Queued
        );
        // The renamed directory stays inspectable so a later restart can retry or
        // fail closed on the same facts instead of losing the commit evidence.
        assert!(root.join("Tweets/123/01.jpg").is_file());
        assert!(!root.join("_staging/job-1").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn repeated_internal_recovery_is_idempotent_after_commit() {
        let root =
            std::env::temp_dir().join(format!("xarchive-recovery-replay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                2,
                "request-1",
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job");
        database
            .begin_archive_attempt("job-1", "now")
            .expect("attempt");
        let files = crate::FileStore::new(&root).expect("files");
        let staging = files.staging_dir("job-1").expect("staging");
        std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
        let manifest = ArchiveRecoveryManifest {
            schema_version: 1,
            job_id: "job-1".into(),
            attempt_count: 1,
            tweet_row_id,
            tweet_id: "123".into(),
            archive_directory: "Tweets/123".into(),
            media: vec![ArchiveRecoveryMediaFact {
                media_index: 1,
                relative_path: "01.jpg".into(),
                media_id: None,
                media_type: "photo".into(),
                mime_type: None,
                size_bytes: 3,
                sha256: crate::FileStore::sha256(staging.join("01.jpg")).expect("hash"),
            }],
            telegram_tweet_row_id: None,
        };
        std::fs::write(
            staging.join(".xarchive-recovery.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .unwrap();
        let mut service = crate::ArchiveService::new(database, files);
        service
            .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
            .unwrap();
        assert_eq!(
            service.database.job_state("job-1").unwrap(),
            xarchive_core::JobState::Downloaded
        );
        assert_eq!(
            service
                .database
                .archive_recovery_journal("job-1")
                .unwrap()
                .unwrap()
                .phase,
            ArchiveRecoveryPhase::Committed
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recovery_reconstructs_staging_and_final_only_from_internal_facts() {
        for final_only in [false, true] {
            let root = std::env::temp_dir().join(format!(
                "xarchive-recovery-{}-{}",
                std::process::id(),
                final_only
            ));
            let _ = std::fs::remove_dir_all(&root);
            let mut database = Database::open_in_memory().expect("database");
            let tweet_row_id = database
                .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
                .expect("tweet");
            database
                .create_archive_job_with_optional_recovery_contract(
                    "job-1",
                    tweet_row_id,
                    2,
                    "request-1",
                    "{}",
                    Some(ArchiveRecoveryContract::InternalV1),
                    "now",
                )
                .expect("job");
            database
                .begin_archive_attempt("job-1", "now")
                .expect("attempt");
            let files = crate::FileStore::new(&root).expect("file store");
            let staging = files.staging_dir("job-1").expect("staging");
            std::fs::write(staging.join("01.jpg"), b"abc").expect("media");
            let fact = ArchiveRecoveryMediaFact {
                media_index: 1,
                relative_path: "01.jpg".into(),
                media_id: Some("m1".into()),
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: crate::FileStore::sha256(staging.join("01.jpg")).expect("hash"),
            };
            let manifest = ArchiveRecoveryManifest {
                schema_version: 1,
                job_id: "job-1".into(),
                attempt_count: 1,
                tweet_row_id,
                tweet_id: "123".into(),
                archive_directory: "Tweets/123".into(),
                media: vec![fact],
                telegram_tweet_row_id: None,
            };
            let internal_manifest = serde_json::to_vec(&manifest).expect("serialize manifest");
            std::fs::write(staging.join(".xarchive-recovery.json"), internal_manifest)
                .expect("internal manifest file");
            database
                .create_archive_recovery_manifest(&manifest, "now")
                .expect("journal");
            if final_only {
                files
                    .commit_staging("job-1", Path::new("Tweets/123"))
                    .expect("rename");
            }
            let mut service = crate::ArchiveService::new(database, files);
            service
                .recover_internal_archive("job-1", tweet_row_id, Path::new("Tweets/123"))
                .expect("recover");
            assert_eq!(
                service.database.job_state("job-1").expect("job state"),
                xarchive_core::JobState::Downloaded
            );
            assert_eq!(
                service
                    .database
                    .tweet_archive_facts("123")
                    .expect("facts")
                    .expect("archive facts")
                    .media[0]
                    .sha256
                    .as_deref(),
                Some(manifest.media[0].sha256.as_str())
            );
            let _ = std::fs::remove_dir_all(&root);
        }
    }
}

#[cfg(test)]
mod database_tests {
    use super::*;

    #[test]
    fn manifest_journal_round_trips_and_rejects_corruption() {
        let mut database = Database::open_in_memory().expect("database");
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/a/status/123", "post", "", "now")
            .expect("tweet");
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                2,
                "request-1",
                "{}",
                Some(ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .expect("job");
        database
            .begin_archive_attempt("job-1", "now")
            .expect("attempt");
        let metadata = ArchiveMetadata {
            schema_version: 1,
            tweet_id: "123".into(),
            url: "https://x.com/a/status/123".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: None,
                display_name: None,
            },
            created_at: None,
            text: "must not be in manifest".into(),
            media: vec![xarchive_core::ArchiveMedia {
                index: 1,
                media_id: Some("m1".into()),
                media_type: "photo".into(),
                file: "01.jpg".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            archived_at: "now".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        let manifest = ArchiveRecoveryManifest::from_metadata(
            "job-1",
            1,
            tweet_row_id,
            Path::new("Tweets/123"),
            &metadata,
            None,
        )
        .expect("manifest");
        database
            .create_archive_recovery_manifest(&manifest, "now")
            .expect("journal");
        assert_eq!(
            database
                .verify_archive_recovery_manifest("job-1")
                .expect("verify"),
            manifest
        );
        assert!(
            !serde_json::to_string(&manifest)
                .expect("serialize")
                .contains("must not be in manifest")
        );
        database
            .connection
            .execute(
                "UPDATE archive_recovery_journal SET manifest_json = '{}' WHERE job_id = 'job-1'",
                [],
            )
            .expect("corrupt fixture");
        assert!(database.verify_archive_recovery_manifest("job-1").is_err());
    }
}
