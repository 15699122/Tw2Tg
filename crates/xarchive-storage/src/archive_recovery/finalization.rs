//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl Database {
    /// Atomically persist all database facts for a v2 archive commit. Filesystem
    /// verification and directory placement must be completed by the caller
    /// before this transaction is invoked. COMMITTED is written last in the
    /// same SQLite transaction as the Tweet/media/Job/intent updates.
    pub fn finalize_archive_recovery_v2(
        &mut self,
        job_id: &str,
        attempt_count: u32,
        tweet_row_id: i64,
        metadata: &xarchive_core::ArchiveMetadata,
        manifest: &ArchiveRecoveryManifestV2,
        now: &str,
    ) -> Result<(), StorageError> {
        manifest.validate()?;
        if metadata.media.iter().any(|media| media.validate().is_err())
            || manifest.media.len() != metadata.media.len()
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
            return Err(StorageError::InvalidMetadata(
                "v2 metadata and manifest media facts do not match".into(),
            ));
        }
        if manifest.job_id != job_id
            || manifest.attempt_count != attempt_count
            || manifest.tweet_row_id != tweet_row_id
            || manifest.tweet_id != metadata.tweet_id
            || manifest.archive_directory != format!("Tweets/{}", metadata.tweet_id)
        {
            return Err(StorageError::InvalidMetadata(
                "v2 finalization identity mismatch".into(),
            ));
        }
        let manifest_json = serde_json::to_string(manifest)?;
        let transaction = self.connection.transaction()?;
        let journal: (i64, String, String, String) = transaction.query_row(
            "SELECT attempt_count, phase, rename_progress, manifest_json FROM archive_recovery_v2 WHERE job_id = ?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        if journal.0 != i64::from(attempt_count)
            || journal.1 != "PREPARED"
            || journal.2 != "FINAL"
            || journal.3 != manifest_json
        {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "v2 finalization requires the matching FINAL prepared journal".into(),
            ));
        }
        let fenced_job: Option<String> = transaction
            .query_row(
                "SELECT jobs.state FROM jobs JOIN tweets ON tweets.id = jobs.tweet_id WHERE jobs.id = ?1 AND jobs.attempt_count = ?2 AND jobs.tweet_id = ?3 AND tweets.tweet_id = ?4",
                rusqlite::params![job_id, attempt_count, tweet_row_id, manifest.tweet_id],
                |row| row.get(0),
            )
            .optional()?;
        let fenced_state = fenced_job
            .ok_or_else(|| StorageError::InvalidState("stale v2 archive attempt".into()))?;
        if !matches!(
            fenced_state.as_str(),
            "QUEUED" | "VALIDATING" | "METADATA_READY" | "DOWNLOADING" | "DOWNLOADED"
        ) {
            transaction.rollback()?;
            return Err(StorageError::InvalidState(
                "v2 Job state is not eligible for finalization".into(),
            ));
        }
        let current_job_state = xarchive_core::JobState::parse(&fenced_state)
            .map_err(|_| StorageError::InvalidState("invalid v2 Job state".into()))?;
        let mut next_job_state = current_job_state;
        transaction.execute(
            "UPDATE tweets SET text = ?1, merged_metadata_json = ?2, archive_directory = ?3, archived_at = ?4, reply_to_tweet_id = ?5, quoted_tweet_id = ?6, updated_at = ?4 WHERE id = ?7",
            rusqlite::params![metadata.text, serde_json::to_string(metadata)?, manifest.archive_directory, metadata.archived_at, metadata.reply_to.as_ref().and_then(|id| xarchive_core::TweetId::new(id.clone()).ok()).map(|id| id.as_str().to_owned()), metadata.quoted_tweet.as_ref().and_then(|quoted| xarchive_core::TweetId::new(quoted.tweet_id.clone()).ok()).map(|id| id.as_str().to_owned()), tweet_row_id],
        )?;
        for fact in &manifest.media {
            transaction.execute(
                "INSERT INTO media (tweet_id, media_index, x_media_id, media_type, relative_path, mime_type, size_bytes, sha256, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9) ON CONFLICT(tweet_id, media_index) DO UPDATE SET x_media_id = excluded.x_media_id, media_type = excluded.media_type, relative_path = excluded.relative_path, mime_type = excluded.mime_type, size_bytes = excluded.size_bytes, sha256 = excluded.sha256, updated_at = excluded.updated_at",
                rusqlite::params![tweet_row_id, fact.media_index, fact.media_id, fact.media_type, fact.relative_path, fact.mime_type, fact.size_bytes, fact.sha256, now],
            )?;
        }
        while next_job_state != xarchive_core::JobState::Downloaded {
            let state = match next_job_state {
                xarchive_core::JobState::Queued => xarchive_core::JobState::Validating,
                xarchive_core::JobState::Validating => xarchive_core::JobState::MetadataReady,
                xarchive_core::JobState::MetadataReady => xarchive_core::JobState::Downloading,
                xarchive_core::JobState::Downloading => xarchive_core::JobState::Downloaded,
                _ => {
                    return Err(StorageError::InvalidTransition(
                        next_job_state
                            .transition_to(xarchive_core::JobState::Downloaded)
                            .expect_err("invalid v2 Job transition"),
                    ));
                }
            };
            let changed = transaction.execute(
                "UPDATE jobs SET state = ?1, started_at = CASE WHEN ?1 = 'DOWNLOADING' AND started_at IS NULL THEN ?2 ELSE started_at END, updated_at = ?2 WHERE id = ?3 AND attempt_count = ?4 AND state = ?5",
                rusqlite::params![state.as_str(), now, job_id, attempt_count, next_job_state.as_str()],
            )?;
            if changed != 1 {
                return Err(StorageError::InvalidState(
                    "v2 Job transition was fenced".into(),
                ));
            }
            transaction.execute(
                "INSERT INTO events (job_id, event_type, previous_state, new_state, created_at) VALUES (?1, 'JOB_STATE_CHANGED', ?2, ?3, ?4)",
                rusqlite::params![job_id, next_job_state.as_str(), state.as_str(), now],
            )?;
            next_job_state = state;
        }
        if let Some(intent_id) = manifest.telegram_tweet_row_id {
            let intent_changed = transaction.execute(
                "UPDATE telegram_archive_intents SET state = 'ARCHIVED', archive_directory = ?1, updated_at = ?2 WHERE tweet_id = ?3 AND job_id = ?4 AND state = 'PREPARED'",
                rusqlite::params![manifest.archive_directory, now, intent_id, job_id],
            )?;
            if intent_changed != 1 {
                return Err(StorageError::InvalidState(
                    "v2 Telegram intent finalization mismatch".into(),
                ));
            }
        }
        let committed = transaction.execute(
            "UPDATE archive_recovery_v2 SET phase = 'COMMITTED', updated_at = ?1 WHERE job_id = ?2 AND attempt_count = ?3 AND phase = 'PREPARED' AND rename_progress = 'FINAL'",
            rusqlite::params![now, job_id, attempt_count],
        )?;
        if committed != 1 {
            return Err(StorageError::InvalidState(
                "v2 journal finalization compare-and-set failed".into(),
            ));
        }
        // CROSS_PLATFORM_CHANGE_REQUIRED (ADR-015/P4): download success is a
        // fact of this publication, not a later best-effort Desktop write.
        // Roll it back together with media, intent, state and journal facts.
        let files_copied = u32::try_from(manifest.media.len())
            .map_err(|_| StorageError::InvalidMetadata("too many archive media files".into()))?;
        let payload =
            serde_json::to_string(&xarchive_core::JobEvent::DownloadCompleted { files_copied })?;
        transaction.execute(
            "INSERT INTO events (job_id, event_type, payload_json, created_at) VALUES (?1, 'DOWNLOAD_COMPLETED', ?2, ?3)",
            rusqlite::params![job_id, payload, now],
        )?;
        // The candidate exists only to bridge PREPARED to durable business
        // facts. Retire it atomically: rollback must retain recovery authority,
        // while COMMITTED replay verifies journal/media/business facts only.
        transaction.execute(
            "DELETE FROM archive_recovery_v2_candidate WHERE job_id = ?1 AND attempt_count = ?2 AND tweet_row_id = ?3",
            rusqlite::params![job_id, attempt_count, tweet_row_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn recover_internal_v1_rows_and_commit_manifest(
        &self,
        manifest: &ArchiveRecoveryManifest,
        now: &str,
    ) -> Result<(), StorageError> {
        manifest.validate()?;
        let transaction = self.connection.unchecked_transaction()?;
        let job: Option<(i64, Option<i64>, String, i64)> = transaction
            .query_row(
                "SELECT attempt_count, recovery_contract_version, state, tweet_id FROM jobs WHERE id = ?1",
                [&manifest.job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let journal: Option<(i64, i64, String, i64, String, String, String)> = transaction
            .query_row(
                "SELECT attempt_count, tweet_row_id, archive_directory, manifest_schema_version, manifest_json, manifest_sha256, phase FROM archive_recovery_journal WHERE job_id = ?1",
                [&manifest.job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .optional()?;
        let Some((attempt_count, contract, state, tweet_row_id)) = job else {
            return Err(StorageError::InvalidState(
                "InternalV1 recovery job is missing".into(),
            ));
        };
        let Some((
            journal_attempt,
            journal_tweet_row_id,
            archive_directory,
            schema_version,
            json,
            digest,
            phase,
        )) = journal
        else {
            return Err(StorageError::InvalidState(
                "InternalV1 recovery journal is missing".into(),
            ));
        };
        if attempt_count != i64::from(manifest.attempt_count)
            || contract != Some(i64::from(ArchiveRecoveryContract::InternalV1.version()))
            || !matches!(state.as_str(), "DOWNLOADING" | "DOWNLOADED")
            || tweet_row_id != manifest.tweet_row_id
            || journal_attempt != i64::from(manifest.attempt_count)
            || journal_tweet_row_id != manifest.tweet_row_id
            || archive_directory != manifest.archive_directory
            || schema_version != i64::from(manifest.schema_version)
            || json != serde_json::to_string(manifest)?
            || digest != manifest.digest()?
            || phase != ArchiveRecoveryPhase::Prepared.as_str()
        {
            return Err(StorageError::InvalidState(
                "InternalV1 recovery identity is not current and PREPARED".into(),
            ));
        }
        // CROSS_PLATFORM_CHANGE_REQUIRED: archive facts, terminal download
        // transition/events, and journal phase must share one atomic boundary.
        Database::recover_archive_rows_from_manifest_in_transaction(&transaction, manifest, now)?;
        let changed = transaction.execute(
            "UPDATE archive_recovery_journal SET phase = 'COMMITTED', updated_at = ?1 WHERE job_id = ?2 AND attempt_count = ?3 AND phase = 'PREPARED'",
            rusqlite::params![now, manifest.job_id, manifest.attempt_count],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidState(
                "InternalV1 journal finalization compare-and-set failed".into(),
            ));
        }
        transaction.commit()?;
        Ok(())
    }
}
