use std::fs;
use std::path::{Path, PathBuf};

use xarchive_core::{ArchiveMetadata, JobState};

use crate::{
    ArchiveRecoveryContract, ArchiveRecoveryManifest, Database, FileStore, StorageError,
    TelegramArchiveIntentRecord, UserProfileFile, build_archive_metadata,
};

pub struct ArchiveService {
    pub database: Database,
    pub files: FileStore,
}

/// Untrusted Sidecar output plus the request identity needed to commit it.
/// Keeping the operation context together avoids a wide positional API and
/// makes the identity binding explicit at the archive boundary.
pub struct SidecarArchiveRequest<'a> {
    pub job_id: &'a str,
    pub tweet_row_id: i64,
    pub expected_tweet_id: &'a str,
    pub metadata: &'a serde_json::Value,
    pub files: &'a [xarchive_protocol::DownloadFile],
    pub final_directory: &'a Path,
    pub archived_at: &'a str,
    /// Captured, non-secret Telegram queue facts. `None` does not create a
    /// journal record and must never be interpreted as permission to send.
    pub telegram_intent: Option<&'a TelegramArchiveIntentRecord>,
}

impl ArchiveService {
    pub fn new(database: Database, files: FileStore) -> Self {
        Self { database, files }
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

    /// Register the archive author and refresh their portable profile file.
    ///
    /// Tweets without a resolvable `user_id` skip registration silently:
    /// no user row is created and no profile file is written.
    fn refresh_author_profile(
        &mut self,
        tweet_row_id: i64,
        metadata: &ArchiveMetadata,
    ) -> Result<(), StorageError> {
        let Some(user_id) = metadata
            .author
            .user_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
        else {
            return Ok(());
        };
        let user_row_id = self.database.upsert_user(
            &user_id,
            metadata.author.username.as_deref(),
            metadata.author.display_name.as_deref(),
            &metadata.archived_at,
        )?;
        // P1-C: the browser path anchored this tweet under a temporary
        // `browser-<tweet_id>` identity; with the stable id now available the
        // placeholder (and every other tweet linked to it) is upgraded in
        // place so the author aggregates under one durable user.
        let placeholder = format!("browser-{}", metadata.tweet_id);
        self.database
            .merge_placeholder_user(&placeholder, user_row_id, &metadata.archived_at)?;
        self.database.set_tweet_user(tweet_row_id, user_row_id)?;
        let Some(snapshot) = self.database.user_profile(&user_id)? else {
            return Ok(());
        };
        self.files.write_user_profile(&UserProfileFile {
            schema_version: 1,
            user_id: snapshot.user_id,
            stable_directory_name: snapshot.stable_directory_name,
            username: snapshot.username,
            display_name: snapshot.display_name,
            first_seen_at: snapshot.first_seen_at,
            last_seen_at: snapshot.last_seen_at,
            updated_at: metadata.archived_at.clone(),
            names: snapshot.names,
        })?;
        Ok(())
    }

    /// Convert a Sidecar result into trusted local metadata and commit it.
    ///
    /// Sidecar-reported paths and sizes are treated as untrusted hints. Rust
    /// resolves each path below the job staging directory, reads the actual
    /// file size, and computes the SHA-256 before writing the database record.
    pub fn complete_sidecar_archive(
        &mut self,
        request: SidecarArchiveRequest<'_>,
    ) -> Result<PathBuf, StorageError> {
        let staging = self.files.staging_dir(request.job_id)?;
        let archive_metadata = build_archive_metadata(
            request.expected_tweet_id,
            request.metadata,
            request.files,
            &staging,
            request.archived_at,
        )?;
        let staging = self.files.staging_dir(request.job_id)?;
        // Stage portable metadata before recording intent so its contents and
        // the files to be renamed are fixed before the recovery marker commits.
        fs::write(
            staging.join("tweet.json"),
            serde_json::to_vec_pretty(&archive_metadata)?,
        )?;
        let author = archive_metadata
            .author
            .display_name
            .as_deref()
            .unwrap_or("");
        let username = archive_metadata.author.username.as_deref().unwrap_or("");
        fs::write(
            staging.join("tweet.txt"),
            format!("{author}\n@{username}\n\n{}\n", archive_metadata.text),
        )?;
        if let Some(intent) = request.telegram_intent {
            self.database.prepare_telegram_archive_intent(intent)?;
        }
        let destination = self.complete_local_archive(
            request.job_id,
            request.tweet_row_id,
            &archive_metadata,
            request.final_directory,
        )?;
        if request.telegram_intent.is_some() {
            self.database.transition_telegram_archive_intent(
                request.tweet_row_id,
                "PREPARED",
                "ARCHIVED",
                request.archived_at,
            )?;
        }
        Ok(destination)
    }

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
            self.recover_internal_archive(
                &intent.job_id,
                intent.tweet_row_id,
                Path::new(&intent.archive_directory),
            )?;
            return Ok("ARCHIVED".to_owned());
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

    fn finish_committed_archive(
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

    fn validate_recovery_metadata(
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
            Some(version) if version == ArchiveRecoveryContract::InternalV1.version() => {
                self.recover_internal_archive(job_id, tweet_row_id, final_directory)
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

    /// Reconstruct only fields proven by the internal manifest and verified
    /// media. Rich Tweet metadata is intentionally not inferred from exports.
    pub fn recover_internal_archive(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
    ) -> Result<PathBuf, StorageError> {
        let manifest = self.database.verify_archive_recovery_manifest(job_id)?;
        if manifest.tweet_row_id != tweet_row_id
            || manifest.archive_directory != final_directory.to_string_lossy().replace('\\', "/")
        {
            return Err(StorageError::InvalidMetadata(
                "recovery target does not match internal manifest".into(),
            ));
        }
        let staging_exists = self.files.recovery_directory_exists(job_id, "_staging")?;
        let final_exists = self
            .files
            .recovery_directory_exists(job_id, final_directory)?;
        if staging_exists == final_exists {
            return Err(StorageError::InvalidState(
                "recovery requires exactly one of staging or final archive".into(),
            ));
        }
        let directory = if staging_exists {
            self.files.staging_dir(job_id)?
        } else {
            self.files.archive_path(final_directory)?
        };
        let internal_manifest_path =
            FileStore::resolve_within(&directory, Path::new(".xarchive-recovery.json"))?;
        let disk_manifest: ArchiveRecoveryManifest =
            serde_json::from_slice(&fs::read(internal_manifest_path)?)?;
        disk_manifest.validate()?;
        if disk_manifest != manifest {
            return Err(StorageError::InvalidMetadata(
                "on-disk recovery manifest does not match the journal".into(),
            ));
        }
        for fact in &manifest.media {
            let path = FileStore::resolve_within(&directory, Path::new(&fact.relative_path))?;
            let metadata = fs::symlink_metadata(&path)?;
            if !metadata.file_type().is_file()
                || crate::file_store::is_reparse_point(&metadata)
                || metadata.len() != fact.size_bytes
                || FileStore::sha256(&path)? != fact.sha256
            {
                return Err(StorageError::InvalidMetadata(
                    "internal recovery media integrity mismatch".into(),
                ));
            }
        }
        let result = if staging_exists {
            self.files.commit_staging(job_id, final_directory)?
        } else {
            self.files.archive_path(final_directory)?
        };
        self.database
            .recover_archive_rows_from_manifest(&manifest, "recovered")?;
        self.database
            .commit_archive_recovery_manifest(job_id, "recovered")?;
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
}
