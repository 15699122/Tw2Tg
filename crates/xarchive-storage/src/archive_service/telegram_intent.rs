//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl ArchiveService {
    /// Resume a journaled archive intent after a crash between SQLite and the
    /// filesystem commit. `PREPARED` is finalized only when the destination or
    /// staging payload still exists; `ARCHIVED` is returned for the caller to
    /// idempotently materialize its outbox rows. This operation never guesses
    /// current Telegram settings.
    pub fn recover_telegram_archive_intent(
        &mut self,
        intent: &TelegramArchiveIntentRecord,
        now: &str,
    ) -> Result<String, StorageError> {
        let stored = self
            .database
            .telegram_archive_intent(intent.tweet_row_id)?
            .ok_or_else(|| StorageError::InvalidState("archive intent is not persisted".into()))?;
        let mut comparable = stored.clone();
        comparable.state = intent.state.clone();
        comparable.updated_at = intent.updated_at.clone();
        if comparable != *intent {
            return Err(StorageError::InvalidState(
                "archive recovery intent mismatch".into(),
            ));
        }
        if self.database.archive_recovery_contract(&intent.job_id)?
            == Some(ArchiveRecoveryContract::InternalV1.version())
            && intent.state == "PREPARED"
        {
            return Err(StorageError::InvalidState(
                "InternalV1 intent recovery requires runner-owned Workflow authorization".into(),
            ));
        }
        match intent.state.as_str() {
            "PREPARED" => {
                let final_exists = self
                    .files
                    .recovery_directory_exists(&intent.job_id, &intent.archive_directory)?;
                let staging_exists = self
                    .files
                    .recovery_directory_exists(&intent.job_id, "_staging")?;
                if final_exists {
                    if staging_exists {
                        return Err(StorageError::InvalidState(
                            "both staging and final archive directories exist".into(),
                        ));
                    }
                    let directory = self.files.archive_path(&intent.archive_directory)?;
                    let metadata = self.validate_recovery_metadata(intent, &directory)?;
                    if self.database.job_state(&intent.job_id)?
                        != xarchive_core::JobState::Downloaded
                    {
                        self.finish_committed_archive(
                            &intent.job_id,
                            intent.tweet_row_id,
                            &metadata,
                            Path::new(&intent.archive_directory),
                        )?;
                    }
                    self.database.transition_telegram_archive_intent(
                        intent.tweet_row_id,
                        "PREPARED",
                        "ARCHIVED",
                        now,
                    )?;
                    Ok("ARCHIVED".to_owned())
                } else if staging_exists {
                    let staging = self.files.staging_dir(&intent.job_id)?;
                    let metadata = self.validate_recovery_metadata(intent, &staging)?;
                    let committed = self
                        .files
                        .commit_staging(&intent.job_id, Path::new(&intent.archive_directory))?;
                    debug_assert!(committed.is_dir());
                    self.finish_committed_archive(
                        &intent.job_id,
                        intent.tweet_row_id,
                        &metadata,
                        Path::new(&intent.archive_directory),
                    )?;
                    self.database.transition_telegram_archive_intent(
                        intent.tweet_row_id,
                        "PREPARED",
                        "ARCHIVED",
                        now,
                    )?;
                    Ok("ARCHIVED".to_owned())
                } else {
                    Err(StorageError::InvalidState(
                        "prepared Telegram archive intent has neither staging nor final directory"
                            .into(),
                    ))
                }
            }
            "ARCHIVED" => Ok("ARCHIVED".to_owned()),
            "QUEUED" | "SKIPPED" => Ok(intent.state.clone()),
            _ => Err(StorageError::InvalidState(
                "unknown Telegram archive intent state".into(),
            )),
        }
    }
}
