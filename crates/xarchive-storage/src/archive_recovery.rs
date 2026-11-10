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

// Recovery database capability is split by complete operation. The public
// `Database` methods and the module path are unchanged.
mod dispatch;
mod finalization;
mod v1_journal;
mod v2_journal;

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

impl Database {}

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
