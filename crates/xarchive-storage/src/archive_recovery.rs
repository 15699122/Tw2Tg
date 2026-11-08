use crate::archive_recovery_manifest::{ArchiveRecoveryManifest, ArchiveRecoveryManifestV2};
use crate::{Database, StorageError};
use rusqlite::OptionalExtension;
use sha2::Digest;
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

/// Classification of the durable archive recovery facts used at startup.
/// Unknown versions are preserved as data so callers can fail closed without
/// accidentally treating a future contract as a legacy job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRecoveryDispatch {
    Legacy,
    InternalV1,
    InternalRenameV2,
    Unknown(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRecoveryPhase {
    Prepared,
    Committed,
}

/// Outcome of the atomic cancellation decision taken against the durable
/// archive commit boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveCancellationDecision {
    /// The job was transitioned to `CANCELLED` with its state-change event in
    /// the same transaction.
    Cancelled,
    /// A PREPARED/COMMITTED journal owns the job: the archive commit wins and
    /// no cancellation state was written.
    CommitWins,
    /// The job was already terminal; nothing was written.
    AlreadyDecided,
}

impl ArchiveRecoveryPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "PREPARED",
            Self::Committed => "COMMITTED",
        }
    }
}

impl Database {
    pub fn internal_v1_recovery_job_ids(&self) -> Result<Vec<String>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT jobs.id FROM jobs WHERE jobs.recovery_contract_version = 1 OR EXISTS (SELECT 1 FROM archive_recovery_journal j WHERE j.job_id = jobs.id) ORDER BY jobs.updated_at, jobs.id",
        )?;
        Ok(statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn declared_recovery_contract_job_ids(&self) -> Result<Vec<String>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id FROM jobs WHERE recovery_contract_version IS NOT NULL ORDER BY updated_at, id",
        )?;
        Ok(statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn internal_v1_recovery_identity(
        &self,
        job_id: &str,
    ) -> Result<Option<(Option<ArchiveRecoveryPhase>, u32, i64, String, String)>, StorageError>
    {
        let contract = self.archive_recovery_contract(job_id)?;
        if contract != Some(1) {
            if contract.is_some() || self.archive_recovery_journal(job_id)?.is_some() {
                return Err(StorageError::InvalidMetadata(
                    "recovery contract and journal disagree".into(),
                ));
            }
            return Ok(None);
        }
        let attempt = self.archive_attempt(job_id)?;
        let state = self.job_state(job_id)?.as_str().to_owned();
        match self.archive_recovery_journal(job_id)? {
            Some(journal) => Ok(Some((
                Some(journal.phase),
                journal.attempt_count,
                journal.tweet_row_id,
                journal.archive_directory,
                state,
            ))),
            None => Ok(Some((None, attempt, 0, String::new(), state))),
        }
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
    /// Authoritative mutable rename progress tracked in the
    /// `archive_recovery_v2.rename_progress` column. The JSON manifest is the
    /// immutable plan: its embedded `rename_phase` is always `PLANNED` and is
    /// never rewritten by phase transitions.
    pub rename_progress: String,
    pub phase: ArchiveRecoveryPhase,
    pub created_at: String,
    pub updated_at: String,
}

impl Database {
    /// Select recovery or normal resume from durable internal facts and the
    /// declared contract. A missing V1 journal is valid only before the
    /// download begins; a DOWNLOADING row must use the dedicated fenced
    /// predecessor/successor takeover, never ordinary resume.
    /// CROSS_PLATFORM_CHANGE_REQUIRED (ADR-015): distinguish pre-PREPARED V1
    /// execution from a downloaded job whose required journal is missing.
    pub fn startup_recovery_dispatch(
        &self,
        job_id: &str,
    ) -> Result<ArchiveRecoveryDispatch, StorageError> {
        let contract = self.archive_recovery_contract(job_id)?;
        let (v1, has_v2) = match contract {
            Some(1) => (self.archive_recovery_journal(job_id)?, None),
            Some(2) => (None, self.v2_journal_state(job_id)?),
            Some(_) => (None, None),
            None => {
                let v1 = self.archive_recovery_journal(job_id)?;
                let v2 = self.v2_journal_state(job_id)?;
                if v2.is_some() {
                    return Err(StorageError::InvalidMetadata(
                        "archive recovery journal exists without a declared contract".into(),
                    ));
                }
                (v1, v2)
            }
        };
        match contract {
            Some(1) if v1.is_some() && has_v2.is_none() => Ok(ArchiveRecoveryDispatch::InternalV1),
            Some(1) if v1.is_none() && has_v2.is_none() => {
                let state = self.job_state(job_id)?;
                if matches!(
                    state,
                    xarchive_core::JobState::Queued
                        | xarchive_core::JobState::Validating
                        | xarchive_core::JobState::MetadataReady
                        | xarchive_core::JobState::Interrupted
                        | xarchive_core::JobState::Downloading
                ) {
                    Ok(ArchiveRecoveryDispatch::InternalV1)
                } else {
                    Err(StorageError::InvalidMetadata(
                        "declared InternalV1 archive recovery journal is missing after download"
                            .into(),
                    ))
                }
            }
            Some(1) => Err(StorageError::InvalidMetadata(
                "declared InternalV1 archive recovery journal is missing".into(),
            )),
            Some(2) if has_v2.is_some() && v1.is_none() => {
                Ok(ArchiveRecoveryDispatch::InternalRenameV2)
            }
            Some(2) => Err(StorageError::InvalidMetadata(
                "declared InternalRenameV2 archive recovery journal is missing".into(),
            )),
            Some(version) => Ok(ArchiveRecoveryDispatch::Unknown(version)),
            None if v1.is_some() => Err(StorageError::InvalidMetadata(
                "archive recovery journal exists without a declared contract".into(),
            )),
            None => Ok(ArchiveRecoveryDispatch::Legacy),
        }
    }
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

    fn v2_journal_state(&self, job_id: &str) -> Result<Option<ArchiveRecoveryPhase>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT phase FROM archive_recovery_v2 WHERE job_id = ?1",
                [job_id],
                |row| {
                    let phase: String = row.get(0)?;
                    match phase.as_str() {
                        "PREPARED" => Ok(ArchiveRecoveryPhase::Prepared),
                        "COMMITTED" => Ok(ArchiveRecoveryPhase::Committed),
                        _ => Err(rusqlite::Error::InvalidQuery),
                    }
                },
            )
            .optional()?)
    }

    /// Whether a durable archive commit boundary already owns this job.
    ///
    /// Both journal generations count: a PREPARED or COMMITTED row in either
    /// table means the archive commit has claimed the job, and cancellation
    /// must never write a cancelled state over it. The read is advisory — the
    /// authoritative check happens inside
    /// [`Self::decide_archive_cancellation`]'s transaction.
    pub fn archive_commit_started(&self, job_id: &str) -> Result<bool, StorageError> {
        if self.v2_journal_state(job_id)?.is_some() {
            return Ok(true);
        }
        let v1: Option<String> = self
            .connection
            .query_row(
                "SELECT phase FROM archive_recovery_journal WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(v1.is_some())
    }

    /// Atomically decide and persist cancellation against the commit boundary.
    ///
    /// One transaction re-checks both journals, validates the state
    /// transition, and writes the cancelled state together with its
    /// state-change event. A prepare committing concurrently can therefore
    /// never be overwritten by `CANCELLED`, and `CANCELLED` can never be
    /// written over a PREPARED journal: whichever side commits first wins and
    /// the other observes it here.
    pub fn decide_archive_cancellation(
        &self,
        job_id: &str,
        now: &str,
    ) -> Result<ArchiveCancellationDecision, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let v2: Option<String> = transaction
            .query_row(
                "SELECT phase FROM archive_recovery_v2 WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .optional()?;
        let v1: Option<String> = transaction
            .query_row(
                "SELECT phase FROM archive_recovery_journal WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .optional()?;
        if v2.is_some() || v1.is_some() {
            // No writes: dropping the transaction rolls back.
            return Ok(ArchiveCancellationDecision::CommitWins);
        }
        let current: String =
            transaction.query_row("SELECT state FROM jobs WHERE id = ?1", [job_id], |row| {
                row.get(0)
            })?;
        let current = xarchive_core::JobState::parse(&current)
            .map_err(|_| StorageError::InvalidState(current))?;
        if current.is_terminal() {
            return Ok(ArchiveCancellationDecision::AlreadyDecided);
        }
        current
            .transition_to(xarchive_core::JobState::Cancelled)
            .map_err(StorageError::InvalidTransition)?;
        // Mirrors Database::transition_job for a cancellation, inside the same
        // transaction as the boundary check.
        transaction.execute(
            "UPDATE jobs SET state = 'CANCELLED', finished_at = ?1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, job_id],
        )?;
        transaction.execute(
            "INSERT INTO events (job_id, event_type, previous_state, new_state, created_at) VALUES (?1, 'JOB_STATE_CHANGED', ?2, 'CANCELLED', ?3)",
            rusqlite::params![job_id, current.as_str(), now],
        )?;
        transaction.commit()?;
        Ok(ArchiveCancellationDecision::Cancelled)
    }

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

    /// List every InternalV1 recovery journal, including COMMITTED records
    /// whose durable job terminal transition may still need convergence.

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
    let rename_progress: String = row.get(6)?;
    let phase: String = row.get(7)?;
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
        rename_progress,
        phase,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
