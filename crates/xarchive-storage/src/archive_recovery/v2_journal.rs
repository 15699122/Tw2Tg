//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl Database {
    pub fn prepare_archive_recovery_v2(
        &self,
        manifest: &ArchiveRecoveryManifestV2,
        metadata: &ArchiveMetadata,
        now: &str,
    ) -> Result<ArchiveRecoveryManifestV2Record, StorageError> {
        manifest.validate()?;
        let candidate_json = crate::recovery_candidate::encode(metadata)?;
        let candidate_digest = format!("{:x}", sha2::Sha256::digest(candidate_json.as_bytes()));
        if metadata.tweet_id != manifest.tweet_id
            || metadata.media.len() != manifest.media.len()
            || manifest.media.iter().any(|fact| {
                !metadata.media.iter().any(|media| {
                    media.index == fact.media_index
                        && media.file == fact.relative_path
                        && media.media_id == fact.media_id
                        && media.media_type == fact.media_type
                        && media.mime_type == fact.mime_type
                        && media.size_bytes == fact.size_bytes
                        && media.sha256 == fact.sha256
                })
            })
        {
            return Err(StorageError::InvalidState(
                "v2 candidate must exactly match immutable manifest media facts".into(),
            ));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let identity: Option<(i64, i64, String)> = transaction
            .query_row(
                "SELECT recovery_contract_version, attempt_count, tweets.tweet_id FROM jobs JOIN tweets ON tweets.id = jobs.tweet_id WHERE jobs.id = ?1 AND tweets.id = ?2",
                rusqlite::params![manifest.job_id, manifest.tweet_row_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if identity
            != Some((
                2,
                i64::from(manifest.attempt_count),
                manifest.tweet_id.clone(),
            ))
        {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "job is not enrolled in this v2 recovery attempt".into(),
            ));
        }
        let current_job: (String, String) = transaction.query_row(
            "SELECT jobs.state, tweets.tweet_id FROM jobs JOIN tweets ON tweets.id = jobs.tweet_id WHERE jobs.id = ?1 AND jobs.tweet_id = ?2",
            rusqlite::params![manifest.job_id, manifest.tweet_row_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if current_job.1 != manifest.tweet_id {
            transaction.rollback()?;
            return Err(StorageError::InvalidMetadata(
                "v2 Tweet identity mismatch".into(),
            ));
        }
        let current_state = xarchive_core::JobState::parse(&current_job.0)
            .map_err(|_| StorageError::InvalidState("invalid v2 Job state".into()))?;
        if !current_state.can_transition_to(xarchive_core::JobState::Downloaded)
            && current_state != xarchive_core::JobState::Downloaded
        {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "v2 archive cannot be prepared from a terminal Job state".into(),
            ));
        }
        if let Some(row_id) = manifest.telegram_tweet_row_id {
            let intent: Option<(String, i64, String)> = transaction.query_row(
                "SELECT job_id, tweet_id, archive_directory FROM telegram_archive_intents WHERE tweet_id = ?1",
                [row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?;
            if intent.is_none_or(|intent| {
                intent.0 != manifest.job_id
                    || intent.1 != manifest.tweet_row_id
                    || intent.2 != manifest.archive_directory
            }) {
                transaction.rollback()?;
                return Err(StorageError::InvalidState(
                    "matching Telegram intent must be persisted before v2 recovery manifest".into(),
                ));
            }
        }
        let manifest_json = serde_json::to_string(manifest)?;
        let digest = manifest.digest()?;
        transaction.execute(
            "INSERT INTO archive_recovery_v2_candidate (job_id, attempt_count, tweet_row_id, metadata_json, metadata_sha256, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) ON CONFLICT(job_id) DO NOTHING",
            rusqlite::params![manifest.job_id, manifest.attempt_count, manifest.tweet_row_id, candidate_json, candidate_digest, now],
        )?;
        let candidate: (i64, i64, String, String) = transaction.query_row(
            "SELECT attempt_count, tweet_row_id, metadata_json, metadata_sha256 FROM archive_recovery_v2_candidate WHERE job_id = ?1",
            [&manifest.job_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        if candidate
            != (
                i64::from(manifest.attempt_count),
                manifest.tweet_row_id,
                candidate_json,
                candidate_digest,
            )
        {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "v2 candidate conflicts with immutable attempt".into(),
            ));
        }
        transaction.execute(
            "INSERT INTO archive_recovery_v2 (job_id, attempt_count, tweet_row_id, archive_directory, manifest_json, manifest_sha256, phase, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'PREPARED', ?7, ?7) ON CONFLICT(job_id) DO NOTHING",
            rusqlite::params![manifest.job_id, manifest.attempt_count, manifest.tweet_row_id, manifest.archive_directory, manifest_json, digest, now],
        )?;
        let stored_json: String = transaction.query_row(
            "SELECT manifest_json FROM archive_recovery_v2 WHERE job_id = ?1",
            [&manifest.job_id],
            |row| row.get(0),
        )?;
        if stored_json != manifest_json {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "v2 recovery plan conflicts with existing attempt".into(),
            ));
        }
        transaction.commit()?;
        self.archive_recovery_manifest_v2(&manifest.job_id)?
            .ok_or_else(|| StorageError::InvalidState("v2 recovery journal disappeared".into()))
    }

    pub fn archive_recovery_v2_candidate(
        &self,
        job_id: &str,
        attempt_count: u32,
        tweet_row_id: i64,
    ) -> Result<Option<ArchiveMetadata>, StorageError> {
        let stored: Option<(i64, i64, String, String)> = self.connection.query_row(
            "SELECT attempt_count, tweet_row_id, metadata_json, metadata_sha256 FROM archive_recovery_v2_candidate WHERE job_id = ?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional()?;
        let Some((stored_attempt, stored_tweet, json, digest)) = stored else {
            return Ok(None);
        };
        if stored_attempt != i64::from(attempt_count)
            || stored_tweet != tweet_row_id
            || format!("{:x}", sha2::Sha256::digest(json.as_bytes())) != digest
        {
            return Err(StorageError::InvalidMetadata(
                "v2 metadata candidate integrity or attempt mismatch".into(),
            ));
        }
        let metadata = crate::recovery_candidate::decode(&json)?;
        if metadata.media.iter().any(|media| media.validate().is_err()) {
            return Err(StorageError::InvalidMetadata(
                "v2 metadata candidate has invalid media index".into(),
            ));
        }
        Ok(Some(metadata))
    }

    pub fn archive_recovery_manifest_v2(
        &self,
        job_id: &str,
    ) -> Result<Option<ArchiveRecoveryManifestV2Record>, StorageError> {
        let record = self
            .connection
            .query_row(
                "SELECT job_id, attempt_count, tweet_row_id, archive_directory, manifest_json, manifest_sha256, rename_progress, phase, created_at, updated_at FROM archive_recovery_v2 WHERE job_id = ?1",
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
            || manifest.rename_phase != "PLANNED"
            || !matches!(
                record.rename_progress.as_str(),
                "PLANNED" | "TEMPORARY" | "FINAL"
            )
            || (record.phase == ArchiveRecoveryPhase::Committed
                && record.rename_progress != "FINAL")
            || self.archive_recovery_contract(job_id)?
                != Some(ArchiveRecoveryContract::InternalRenameV2.version())
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
        if self.archive_attempt(job_id)? != record.attempt_count {
            return Err(StorageError::InvalidMetadata(
                "v2 journal attempt does not match the active Job attempt".into(),
            ));
        }
        if record.phase == ArchiveRecoveryPhase::Prepared {
            let candidate = self
                .archive_recovery_v2_candidate(
                    &record.job_id,
                    record.attempt_count,
                    record.tweet_row_id,
                )?
                .ok_or_else(|| {
                    StorageError::InvalidMetadata("v2 recovery candidate is missing".into())
                })?;
            // Reject incompatible business facts before replay can rename or
            // publish files, not only in the final database transaction.
            if candidate.media.len() != manifest.media.len()
                || manifest.media.iter().any(|fact| {
                    !candidate.media.iter().any(|media| {
                        media.index == fact.media_index
                            && media.file == fact.relative_path
                            && media.media_id == fact.media_id
                            && media.media_type == fact.media_type
                            && media.mime_type == fact.mime_type
                            && media.size_bytes == fact.size_bytes
                            && media.sha256 == fact.sha256
                    })
                })
            {
                return Err(StorageError::InvalidMetadata(
                    "v2 recovery candidate media facts mismatch".into(),
                ));
            }
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
        let _: ArchiveRecoveryManifestV2 = serde_json::from_str(&current.manifest_json)?;
        let stored_phase = if current.phase == ArchiveRecoveryPhase::Committed {
            "COMMITTED"
        } else {
            "PREPARED"
        };
        if stored_phase == "COMMITTED" && next == "COMMITTED" && current.rename_progress == "FINAL"
        {
            return Ok(current);
        }
        if stored_phase != "PREPARED" || current.rename_progress != expected {
            if stored_phase == "PREPARED" && current.rename_progress == next {
                return Ok(current);
            }
            return Err(StorageError::InvalidState(
                "v2 recovery phase compare-and-set rejected".into(),
            ));
        }
        let committed = next == "COMMITTED";
        let (rename_progress, phase) = if committed {
            ("FINAL", "COMMITTED")
        } else {
            (next, "PREPARED")
        };
        let changed = self.connection.execute(
            "UPDATE archive_recovery_v2 SET rename_progress = ?1, phase = ?2, updated_at = ?3 WHERE job_id = ?4 AND phase = 'PREPARED' AND rename_progress = ?5",
            rusqlite::params![rename_progress, phase, now, job_id, expected],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidState(
                "v2 recovery phase compare-and-set lost".into(),
            ));
        }
        // The manifest JSON is the immutable plan: deserialization above already
        // validated its shape, and only the progress columns advance. A row
        // whose stored digest no longer matches is corruption, not a silent
        // re-plan.
        self.archive_recovery_manifest_v2(job_id)?.ok_or_else(|| {
            StorageError::InvalidState("v2 recovery journal disappeared after transition".into())
        })
    }
}
