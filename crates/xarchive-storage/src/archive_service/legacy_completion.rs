//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl ArchiveService {
    pub fn preview_sidecar_archive(
        &self,
        job_id: &str,
        expected_tweet_id: &str,
        metadata: &serde_json::Value,
        files: &[xarchive_protocol::DownloadFile],
        archived_at: &str,
        output_settings: &BatchOutputSettings,
    ) -> Result<ArchivePreview, StorageError> {
        let staging = self.files.staging_dir(job_id)?;
        let metadata =
            build_archive_metadata(expected_tweet_id, metadata, files, &staging, archived_at)?;
        let (metadata, renames) = crate::plan_archive_media_renames(
            &metadata,
            metadata.author.username.as_deref(),
            output_settings,
        )?;
        Ok(ArchivePreview { metadata, renames })
    }

    /// Preview an archive using only the existing staging tree for one durable
    /// attempt. Unlike the legacy preview, this never creates or falls back to
    /// the Job-scoped staging directory.
    pub fn preview_sidecar_archive_attempt(
        &self,
        job_id: &str,
        attempt_count: u32,
        expected_tweet_id: &str,
        metadata: &serde_json::Value,
        files: &[xarchive_protocol::DownloadFile],
        archived_at: &str,
        output_settings: &BatchOutputSettings,
    ) -> Result<ArchivePreview, StorageError> {
        let staging = self
            .files
            .existing_attempt_staging_dir(job_id, attempt_count)?;
        let metadata =
            build_archive_metadata(expected_tweet_id, metadata, files, &staging, archived_at)?;
        let (metadata, renames) = crate::plan_archive_media_renames(
            &metadata,
            metadata.author.username.as_deref(),
            output_settings,
        )?;
        Ok(ArchivePreview { metadata, renames })
    }

    /// Commit a completed Sidecar result as one local archive operation.
    ///
    /// The caller must provide a staging directory containing already
    /// validated files. This service writes portable metadata, registers
    /// media hashes, then advances the job only after the directory commit.
    pub fn complete_local_archive(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        metadata: &ArchiveMetadata,
        final_directory: &Path,
    ) -> Result<PathBuf, StorageError> {
        if self.database.archive_recovery_contract(job_id)?
            == Some(ArchiveRecoveryContract::InternalRenameV2.version())
        {
            return Err(StorageError::InvalidState(
                "v2 archives must be committed through the recoverable rename pipeline".into(),
            ));
        }
        let state = self.database.job_state(job_id)?;
        if state == JobState::Queued {
            self.database
                .transition_job(job_id, JobState::Validating, &metadata.archived_at)?;
            self.database
                .transition_job(job_id, JobState::MetadataReady, &metadata.archived_at)?;
            self.database
                .transition_job(job_id, JobState::Downloading, &metadata.archived_at)?;
        }

        let staging = self.files.staging_dir(job_id)?;
        let json_path = staging.join("tweet.json");
        let use_internal_recovery = self.database.archive_recovery_contract(job_id)?
            == Some(ArchiveRecoveryContract::InternalV1.version());
        if !use_internal_recovery || !json_path.exists() {
            fs::write(&json_path, serde_json::to_vec_pretty(metadata)?)?;
        }
        let text = format!(
            "{}\n@{}\n\n{}\n",
            metadata.author.display_name.as_deref().unwrap_or(""),
            metadata.author.username.as_deref().unwrap_or(""),
            metadata.text
        );
        let text_path = staging.join("tweet.txt");
        if !use_internal_recovery || !text_path.exists() {
            fs::write(text_path, text)?;
        }

        if use_internal_recovery && self.database.archive_recovery_journal(job_id)?.is_none() {
            let manifest = ArchiveRecoveryManifest::from_metadata(
                job_id,
                self.database.archive_attempt(job_id)?,
                tweet_row_id,
                final_directory,
                metadata,
                self.database
                    .telegram_archive_intent(tweet_row_id)?
                    .filter(|intent| intent.job_id == job_id)
                    .map(|intent| intent.tweet_row_id),
            )?;
            fs::write(
                staging.join(".xarchive-recovery.json"),
                serde_json::to_vec(&manifest)?,
            )?;
            self.database
                .create_archive_recovery_manifest(&manifest, &metadata.archived_at)?;
        }

        let committed = self.files.commit_staging(job_id, final_directory)?;
        // Archive destinations are defined relative to the archive root.
        // Persist the validated committed path, never a machine-local root.
        let relative_directory = final_directory.to_string_lossy().to_string();
        self.database
            .update_tweet_metadata(tweet_row_id, metadata, &relative_directory)?;
        self.refresh_author_profile(tweet_row_id, metadata)?;
        for media in &metadata.media {
            self.database
                .insert_media(tweet_row_id, media, &metadata.archived_at)?;
        }
        if self.database.job_state(job_id)? != JobState::Downloaded {
            self.database
                .transition_job(job_id, JobState::Downloaded, &metadata.archived_at)?;
        }
        if self.database.archive_recovery_contract(job_id)?
            == Some(ArchiveRecoveryContract::InternalV1.version())
        {
            self.database
                .commit_archive_recovery_manifest(job_id, &metadata.archived_at)?;
        }
        Ok(committed)
    }
}
