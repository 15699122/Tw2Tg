//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl ArchiveService {
    /// Recover a commit that reached `DOWNLOADED` after the staging payload
    /// was written but before the final rename completed. The portable
    /// metadata is the immutable local commit record, so recovery can rebuild
    /// the normal commit path without contacting the Sidecar.
    pub fn recover_staging_archive(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
    ) -> Result<PathBuf, StorageError> {
        match self.database.archive_recovery_contract(job_id)? {
            #[allow(deprecated)]
            Some(version) if version == ArchiveRecoveryContract::InternalV1.version() => {
                self.recover_internal_archive(job_id, tweet_row_id, final_directory)
            }
            Some(version) if version == ArchiveRecoveryContract::InternalRenameV2.version() => {
                self.recover_internal_rename_v2(job_id, tweet_row_id, final_directory, "recovered")
            }
            Some(_) => Err(StorageError::InvalidState(
                "unsupported archive recovery contract version".into(),
            )),
            None => {
                let staging = self.files.staging_dir(job_id)?;
                let metadata: ArchiveMetadata =
                    serde_json::from_slice(&fs::read(staging.join("tweet.json"))?)?;
                self.complete_local_archive(job_id, tweet_row_id, &metadata, final_directory)
            }
        }
    }

    /// Legacy migration-only InternalV1 entry point. It fails closed; production
    /// recovery must pass through Workflow authorization.
    pub fn recover_internal_archive(
        &mut self,
        _job_id: &str,
        _tweet_row_id: i64,
        _final_directory: &Path,
    ) -> Result<PathBuf, StorageError> {
        Err(StorageError::InvalidState(
            "InternalV1 recovery requires a Workflow permit".into(),
        ))
    }

    /// Replay an InternalV1 PREPARED journal against its immutable `attempt-N`
    /// staging tree. The permit-consuming entry point below is the production
    /// authorization boundary.
    #[deprecated(note = "use replay_prepared_internal_v1 with a Workflow permit")]
    pub fn recover_internal_archive_attempt(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
    ) -> Result<PathBuf, StorageError> {
        let manifest = self.database.verify_archive_recovery_manifest(job_id)?;
        let journal = self
            .database
            .archive_recovery_journal(job_id)?
            .ok_or_else(|| {
                StorageError::InvalidState("internal recovery journal missing".into())
            })?;
        if manifest.attempt_count == 0
            || manifest.tweet_row_id != tweet_row_id
            || manifest.archive_directory != final_directory.to_string_lossy().replace('\\', "/")
            || self.database.archive_attempt(job_id)? != manifest.attempt_count
            || self.database.archive_recovery_contract(job_id)?
                != Some(ArchiveRecoveryContract::InternalV1.version())
            || journal.phase != ArchiveRecoveryPhase::Prepared
            || self
                .files
                .recovery_directory_exists(job_id, &Path::new("_staging").join(job_id))?
        {
            return Err(StorageError::InvalidMetadata(
                "attempt recovery identity does not match the live journal".into(),
            ));
        }
        let destination = self.files.archive_path(final_directory)?;
        let staging_exists = self
            .files
            .recovery_attempt_directory_exists(job_id, manifest.attempt_count)?;
        let legacy_staging_exists = self.files.recovery_directory_exists(job_id, "_staging")?;
        let final_exists = self
            .files
            .recovery_directory_exists(job_id, final_directory)?;
        if staging_exists {
            if legacy_staging_exists || final_exists {
                return Err(StorageError::InvalidState(
                    "ambiguous attempt-scoped InternalV1 recovery directories".into(),
                ));
            }
            return Err(StorageError::InvalidState(
                "attempt-scoped InternalV1 replay requires a Workflow permit".into(),
            ));
        }
        if legacy_staging_exists == final_exists {
            return Err(StorageError::InvalidState(
                "ambiguous legacy InternalV1 recovery directories".into(),
            ));
        }
        let directory = if legacy_staging_exists {
            self.files.staging_dir(job_id)?
        } else {
            destination.clone()
        };
        // Replay persisted renames before verifying the manifest, so a crash
        // during publishing converges to the final media paths. The plan is an
        // auxiliary fact outside the strict V1 manifest; its absence means no
        // renames were ever planned for this attempt.
        if legacy_staging_exists {
            let plan_path = directory.join(RENAME_PLAN_FILE);
            if plan_path.is_file() {
                let pending: Vec<PendingRename> = serde_json::from_slice(&fs::read(&plan_path)?)?;
                self.apply_pending_renames(&directory, &pending, None)?;
            }
        }
        let result = if legacy_staging_exists {
            self.files.commit_staging(job_id, final_directory)?
        } else {
            destination
        };
        self.database
            .recover_internal_v1_rows_and_commit_manifest(&manifest, "recovered")?;
        if manifest.telegram_tweet_row_id.is_some() {
            let _ = self.database.transition_telegram_archive_intent(
                tweet_row_id,
                "PREPARED",
                "ARCHIVED",
                "recovered",
            )?;
        }
        Ok(result)
    }

    /// Validate a COMMITTED InternalV1 journal and its final tree without
    /// replaying renames, moving directories, or reconstructing archive rows.
    pub fn verify_committed_internal_v1(
        &self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
        attempt: &xarchive_workflow::AttemptIdentity,
    ) -> Result<PathBuf, StorageError> {
        let manifest = self.database.verify_archive_recovery_manifest(job_id)?;
        let journal = self
            .database
            .archive_recovery_journal(job_id)?
            .ok_or_else(|| {
                StorageError::InvalidState("internal recovery journal missing".into())
            })?;
        if journal.phase != ArchiveRecoveryPhase::Committed
            || manifest.job_id != attempt.job_id()
            || manifest.attempt_count != attempt.attempt()
            || manifest.tweet_row_id != tweet_row_id
            || manifest.archive_directory != final_directory.to_string_lossy().replace('\\', "/")
            || self.database.archive_attempt(job_id)? != attempt.attempt()
            || self.database.archive_recovery_contract(job_id)?
                != Some(ArchiveRecoveryContract::InternalV1.version())
        {
            return Err(StorageError::InvalidMetadata(
                "committed recovery identity mismatch".into(),
            ));
        }
        let legacy_staging = PathBuf::from("_staging").join(job_id);
        if self
            .files
            .recovery_directory_exists(job_id, &legacy_staging)?
        {
            return Err(StorageError::InvalidState(
                "committed recovery has an ambiguous legacy staging directory".into(),
            ));
        }
        let destination = self.files.archive_path(final_directory)?;
        if self
            .files
            .recovery_attempt_directory_exists(job_id, attempt.attempt())?
            || !self
                .files
                .recovery_directory_exists(job_id, final_directory)?
        {
            return Err(StorageError::InvalidState(
                "committed recovery tree is not final".into(),
            ));
        }
        self.verify_internal_attempt_recovery_tree(&manifest, &destination)?;
        Ok(destination)
    }

    pub fn replay_committed_terminal_internal_v1(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
        attempt: &xarchive_workflow::AttemptIdentity,
        now: &str,
    ) -> Result<PathBuf, StorageError> {
        let destination =
            self.verify_committed_internal_v1(job_id, tweet_row_id, final_directory, attempt)?;
        self.converge_committed_internal_v1(job_id, attempt, final_directory, now)?;
        Ok(destination)
    }

    pub fn verify_committed_terminal_internal_v1(
        &self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
        attempt: &xarchive_workflow::AttemptIdentity,
    ) -> Result<PathBuf, StorageError> {
        self.verify_committed_internal_v1(job_id, tweet_row_id, final_directory, attempt)
    }

    /// Replay an InternalV1 PREPARED journal only when the runner supplies its
    /// one-shot, session-bound recovery permit. COMMITTED verification remains
    /// a separate read-only path.
    pub fn replay_prepared_internal_v1<C: xarchive_workflow::coordination::Coordinator>(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
        attempt: &xarchive_workflow::AttemptIdentity,
        permit: xarchive_workflow::PreparedRecoveryPermit<'_, '_, C>,
    ) -> Result<PathBuf, StorageError> {
        let manifest = self.database.verify_archive_recovery_manifest(job_id)?;
        let destination = self.files.archive_path(final_directory)?;
        if manifest.job_id != attempt.job_id()
            || manifest.attempt_count != attempt.attempt()
            || manifest.tweet_row_id != tweet_row_id
            || manifest.archive_directory != final_directory.to_string_lossy().replace('\\', "/")
            || self.database.archive_attempt(job_id)? != attempt.attempt()
            || self.database.archive_recovery_contract(job_id)?
                != Some(ArchiveRecoveryContract::InternalV1.version())
        {
            return Err(StorageError::InvalidMetadata(
                "prepared replay identity does not match the durable journal".into(),
            ));
        }
        let journal = self
            .database
            .archive_recovery_journal(job_id)?
            .ok_or_else(|| {
                StorageError::InvalidState("internal recovery journal missing".into())
            })?;
        if journal.phase != ArchiveRecoveryPhase::Prepared {
            return Err(StorageError::InvalidState(
                "permit-consuming replay accepts PREPARED journals only".into(),
            ));
        }
        let legacy_staging = PathBuf::from("_staging").join(job_id);
        if self
            .files
            .recovery_directory_exists(job_id, &legacy_staging)?
        {
            return Err(StorageError::InvalidState(
                "attempt-scoped prepared replay has an ambiguous legacy staging directory".into(),
            ));
        }
        permit
            .consume(attempt, &destination, final_directory)
            .map_err(|error| {
                StorageError::InvalidState(format!("recovery permit rejected: {error:?}"))
            })?;
        let staging_exists = self
            .files
            .recovery_attempt_directory_exists(job_id, attempt.attempt())?;
        let final_exists = self
            .files
            .recovery_directory_exists(job_id, final_directory)?;
        if staging_exists == final_exists {
            return Err(StorageError::InvalidState(
                "prepared replay requires exactly one attempt staging or final directory".into(),
            ));
        }
        let attempt_staging = if staging_exists {
            self.files
                .existing_attempt_staging_dir(job_id, attempt.attempt())?
        } else {
            destination.clone()
        };
        if staging_exists {
            let plan_path = attempt_staging.join(RENAME_PLAN_FILE);
            if plan_path.is_file() {
                let pending: Vec<PendingRename> = serde_json::from_slice(&fs::read(&plan_path)?)?;
                self.apply_pending_renames(&attempt_staging, &pending, None)?;
            }
        }
        let manifest = self.database.verify_archive_recovery_manifest(job_id)?;
        self.verify_internal_attempt_recovery_tree(&manifest, &attempt_staging)?;
        let result = if staging_exists {
            self.files
                .commit_attempt_staging(job_id, attempt.attempt(), final_directory)?
        } else {
            destination
        };
        self.database
            .recover_internal_v1_rows_and_commit_manifest(&manifest, "recovered")?;
        if manifest.telegram_tweet_row_id.is_some() {
            let _ = self.database.transition_telegram_archive_intent(
                tweet_row_id,
                "PREPARED",
                "ARCHIVED",
                "recovered",
            )?;
        }
        Ok(result)
    }

    /// Close the durable job terminal state after an already COMMITTED
    /// InternalV1 journal has been independently verified against its final
    /// tree. This does not replay storage or rewrite archive rows.
    pub fn converge_committed_internal_v1(
        &mut self,
        job_id: &str,
        attempt: &xarchive_workflow::AttemptIdentity,
        archive_directory: &Path,
        now: &str,
    ) -> Result<bool, StorageError> {
        let journal = self
            .database
            .archive_recovery_journal(job_id)?
            .ok_or_else(|| {
                StorageError::InvalidState("internal recovery journal missing".into())
            })?;
        let manifest = self.database.verify_archive_recovery_manifest(job_id)?;
        if journal.phase != ArchiveRecoveryPhase::Committed
            || manifest.job_id != attempt.job_id()
            || manifest.attempt_count != attempt.attempt()
            || manifest.archive_directory != archive_directory.to_string_lossy().replace('\\', "/")
            || self.database.archive_recovery_contract(job_id)?
                != Some(ArchiveRecoveryContract::InternalV1.version())
        {
            return Err(StorageError::InvalidMetadata(
                "committed terminal convergence identity mismatch".into(),
            ));
        }
        let _ = self.verify_committed_internal_v1(
            job_id,
            manifest.tweet_row_id,
            archive_directory,
            attempt,
        )?;
        self.database.complete_terminal_for_attempt(
            job_id,
            attempt.attempt(),
            &manifest.archive_directory,
            u32::try_from(manifest.media.len()).map_err(|_| {
                StorageError::InvalidState("media count exceeds completion range".into())
            })?,
            now,
        )
    }

    pub(super) fn verify_internal_attempt_recovery_tree(
        &self,
        manifest: &ArchiveRecoveryManifest,
        directory: &Path,
    ) -> Result<(), StorageError> {
        let manifest_path =
            FileStore::resolve_within(directory, Path::new(".xarchive-recovery.json"))?;
        let disk_manifest: ArchiveRecoveryManifest =
            serde_json::from_slice(&fs::read(manifest_path)?)?;
        disk_manifest.validate()?;
        if &disk_manifest != manifest {
            return Err(StorageError::InvalidMetadata(
                "attempt recovery manifest mismatch".into(),
            ));
        }
        for fact in &manifest.media {
            let path = FileStore::resolve_within(directory, Path::new(&fact.relative_path))?;
            let metadata = fs::symlink_metadata(&path)?;
            if !metadata.file_type().is_file()
                || crate::file_store::is_reparse_point(&metadata)
                || metadata.len() != fact.size_bytes
                || FileStore::sha256(&path)? != fact.sha256
            {
                return Err(StorageError::InvalidMetadata(
                    "attempt recovery media integrity mismatch".into(),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_recovery_metadata(
        &self,
        intent: &TelegramArchiveIntentRecord,
        directory: &Path,
    ) -> Result<ArchiveMetadata, StorageError> {
        let metadata_path = FileStore::resolve_within(directory, Path::new("tweet.json"))?;
        let metadata: ArchiveMetadata = serde_json::from_slice(&fs::read(metadata_path)?)?;
        let job = self.database.job_summary(&intent.job_id)?;
        let tweet = self.database.tweet_external_id(intent.tweet_row_id)?;
        if metadata.schema_version != 1
            || tweet.as_deref() != Some(metadata.tweet_id.as_str())
            || job.as_ref().map(|job| job.tweet_id.as_str()) != Some(metadata.tweet_id.as_str())
        {
            return Err(StorageError::InvalidMetadata(
                "archive recovery identity mismatch".into(),
            ));
        }
        let text = FileStore::resolve_within(directory, Path::new("tweet.txt"))?;
        if !text.is_file() {
            return Err(StorageError::InvalidMetadata(
                "archive recovery text is missing".into(),
            ));
        }
        for media in &metadata.media {
            let path = FileStore::resolve_within(directory, Path::new(&media.file))?;
            if fs::metadata(&path)?.len() != media.size_bytes
                || FileStore::sha256(&path)? != media.sha256
            {
                return Err(StorageError::InvalidMetadata(
                    "archive recovery media integrity mismatch".into(),
                ));
            }
        }
        Ok(metadata)
    }

    pub(super) fn finish_committed_archive(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        metadata: &xarchive_core::ArchiveMetadata,
        final_directory: &Path,
    ) -> Result<(), StorageError> {
        self.database.update_tweet_metadata(
            tweet_row_id,
            metadata,
            &final_directory.to_string_lossy(),
        )?;
        self.refresh_author_profile(tweet_row_id, metadata)?;
        for media in &metadata.media {
            self.database
                .insert_media(tweet_row_id, media, &metadata.archived_at)?;
        }
        if self.database.job_state(job_id)? != xarchive_core::JobState::Downloaded {
            self.database.transition_job(
                job_id,
                xarchive_core::JobState::Downloaded,
                &metadata.archived_at,
            )?;
        }
        Ok(())
    }
}
