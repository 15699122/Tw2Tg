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
}

impl ArchiveRecoveryContract {
    pub const fn version(self) -> i64 {
        match self {
            Self::InternalV1 => 1,
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
