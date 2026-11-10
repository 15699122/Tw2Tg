//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl Database {
    pub fn create_archive_recovery_manifest(
        &self,
        manifest: &ArchiveRecoveryManifest,
        now: &str,
    ) -> Result<(), StorageError> {
        manifest.validate()?;
        let json = serde_json::to_string(manifest)?;
        let digest = manifest.digest()?;
        let transaction = self.connection.unchecked_transaction()?;
        let identity: Option<(Option<i64>, i64, String)> = transaction
            .query_row(
                "SELECT recovery_contract_version, attempt_count, state FROM jobs WHERE id = ?1",
                [&manifest.job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if identity
            != Some((
                Some(i64::from(ArchiveRecoveryContract::InternalV1.version())),
                i64::from(manifest.attempt_count),
                "DOWNLOADING".to_owned(),
            ))
        {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "job is not active in this InternalV1 recovery attempt".into(),
            ));
        }
        if manifest.telegram_tweet_row_id.is_some() {
            let intent = transaction
                .query_row(
                    "SELECT job_id, archive_directory FROM telegram_archive_intents WHERE tweet_id = ?1",
                    [manifest.tweet_row_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            if intent.as_ref().is_none_or(|intent| {
                intent.0 != manifest.job_id || intent.1 != manifest.archive_directory
            }) {
                transaction.rollback()?;
                return Err(StorageError::InvalidState(
                    "Telegram intent must be persisted before recovery manifest".into(),
                ));
            }
        }
        let (intent_required, intent_row_id) = (
            manifest.telegram_tweet_row_id.is_some(),
            manifest.telegram_tweet_row_id,
        );
        transaction.execute(
            "INSERT INTO archive_recovery_journal (job_id, attempt_count, tweet_row_id, archive_directory, manifest_schema_version, manifest_json, manifest_sha256, telegram_intent_required, telegram_tweet_row_id, phase, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'PREPARED', ?10, ?10) ON CONFLICT(job_id) DO NOTHING",
            rusqlite::params![manifest.job_id, manifest.attempt_count, manifest.tweet_row_id, manifest.archive_directory, manifest.schema_version, json, digest, intent_required, intent_row_id, now],
        )?;
        let stored: Option<(String, String, i64)> = transaction
            .query_row(
                "SELECT manifest_json, manifest_sha256, attempt_count FROM archive_recovery_journal WHERE job_id = ?1",
                [&manifest.job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let stored = stored
            .ok_or_else(|| StorageError::InvalidState("recovery journal insert missing".into()))?;
        if stored.0 != json || stored.1 != digest || stored.2 != i64::from(manifest.attempt_count) {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "recovery manifest already exists with different facts".into(),
            ));
        }
        transaction.commit()?;
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

    pub fn list_internal_v1_recovery_jobs(&self) -> Result<Vec<String>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT job_id FROM archive_recovery_journal ORDER BY updated_at, job_id")?;
        Ok(statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn internal_v1_recovery_tweet_row_id(&self, job_id: &str) -> Result<i64, StorageError> {
        self.connection
            .query_row(
                "SELECT tweet_row_id FROM archive_recovery_journal WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .map_err(StorageError::from)
    }

    pub fn commit_archive_recovery_manifest(
        &self,
        job_id: &str,
        now: &str,
    ) -> Result<(), StorageError> {
        let changed = self.connection.execute(
            "UPDATE archive_recovery_journal SET phase = 'COMMITTED', updated_at = ?1 WHERE job_id = ?2 AND phase = 'PREPARED'",
            rusqlite::params![now, job_id],
        )?;
        if changed == 1
            || self
                .archive_recovery_journal(job_id)?
                .is_some_and(|record| record.phase == ArchiveRecoveryPhase::Committed)
        {
            Ok(())
        } else {
            Err(StorageError::InvalidState(
                "archive recovery journal is not prepared".into(),
            ))
        }
    }
}
