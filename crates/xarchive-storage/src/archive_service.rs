use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use xarchive_core::{ArchiveMetadata, JobState};

use crate::{
    ArchiveRecoveryContract, ArchiveRecoveryManifest, ArchiveRecoveryManifestV2,
    ArchiveRecoveryPhase, BatchOutputSettings, Database, FileStore, StorageError,
    TelegramArchiveIntentRecord, UserProfileFile, build_archive_metadata,
};
use xarchive_workflow::PublicationPermit;

/// Name of the durable rename plan written into an attempt staging tree before
/// any media file is renamed. It is an auxiliary recovery fact, not part of the
/// strict InternalV1 recovery manifest, so it never conflicts with
/// `#[serde(deny_unknown_fields)]` on `ArchiveRecoveryManifest`.
pub(crate) const RENAME_PLAN_FILE: &str = ".xarchive-rename-plan.json";

/// One durable rename fact recorded before mutating attempt staging. Used by
/// crash replay to converge the on-disk staging tree to the manifest's final
/// media paths without guessing the original names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingRename {
    pub source_path: String,
    pub final_path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

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
    pub output_settings: &'a crate::BatchOutputSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePreview {
    pub metadata: ArchiveMetadata,
    pub renames: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRecoveryFailpoint {
    AfterTemporaryRename(usize),
    AfterFinalRename(usize),
    AfterDirectoryCommit,
}

impl ArchiveService {
    pub fn new(database: Database, files: FileStore) -> Self {
        Self { database, files }
    }

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
        self.complete_sidecar_archive_with_failpoint(request, None)
    }

    pub fn complete_sidecar_archive_with_failpoint(
        &mut self,
        request: SidecarArchiveRequest<'_>,
        failpoint: Option<ArchiveRecoveryFailpoint>,
    ) -> Result<PathBuf, StorageError> {
        if let Some(failpoint) = failpoint {
            return Err(StorageError::InvalidState(format!(
                "archive failpoint {failpoint:?} is only implemented on the v2 publish path"
            )));
        }
        let preview = self.preview_sidecar_archive(
            request.job_id,
            request.expected_tweet_id,
            request.metadata,
            request.files,
            request.archived_at,
            request.output_settings,
        )?;
        let archive_metadata = preview.metadata;
        if self.database.archive_recovery_contract(request.job_id)?
            == Some(ArchiveRecoveryContract::InternalRenameV2.version())
        {
            return Err(StorageError::InvalidState(
                "v2 archives must remain fail-closed until lifecycle and file fencing are wired"
                    .into(),
            ));
        }
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

    /// Publish Sidecar output from the exact durable attempt authorized by the
    /// live runner. Unlike `complete_sidecar_archive`, this path never reads or
    /// writes the legacy Job-scoped staging tree.
    ///
    /// The rename plan is persisted into the attempt staging tree before any
    /// media file is renamed, so crash replay can converge the on-disk tree to
    /// the manifest's final media paths. The durable PREPARED journal is written
    /// before the renames execute; a crash after PREPARED but before or during
    /// renames is recovered by [`Self::recover_internal_archive_attempt`].
    /// Production entry point for runner-authorized InternalV1 publication.
    /// The permit remains borrowed through the full durable finalization.
    pub fn publish_internal_v1_attempt(
        &mut self,
        request: SidecarArchiveRequest<'_>,
        attempt_count: u32,
        permit: PublicationPermit<'_>,
    ) -> Result<PathBuf, StorageError> {
        self.complete_sidecar_archive_attempt_inner(
            request,
            attempt_count,
            None,
            permit.attempt(),
            permit.destination(),
        )
    }

    fn complete_sidecar_archive_attempt_inner(
        &mut self,
        request: SidecarArchiveRequest<'_>,
        attempt_count: u32,
        failpoint: Option<ArchiveRecoveryFailpoint>,
        attempt_identity: &xarchive_workflow::AttemptIdentity,
        permit_destination: &Path,
    ) -> Result<PathBuf, StorageError> {
        if attempt_count == 0
            || attempt_identity.job_id() != request.job_id
            || attempt_identity.attempt() != attempt_count
        {
            return Err(StorageError::InvalidState(
                "publication permit does not match the durable attempt".into(),
            ));
        }
        if self.database.archive_recovery_contract(request.job_id)?
            != Some(ArchiveRecoveryContract::InternalV1.version())
        {
            return Err(StorageError::InvalidState(
                "attempt-scoped Sidecar publication requires InternalV1".into(),
            ));
        }
        let destination = self.files.archive_path(request.final_directory)?;
        if permit_destination != destination {
            return Err(StorageError::InvalidState(
                "publication permit destination does not match archive destination".into(),
            ));
        }
        if self.database.archive_attempt(request.job_id)? != attempt_count {
            return Err(StorageError::InvalidState(
                "publication attempt is no longer current".into(),
            ));
        }
        let preview = self.preview_sidecar_archive_attempt(
            request.job_id,
            attempt_count,
            request.expected_tweet_id,
            request.metadata,
            request.files,
            request.archived_at,
            request.output_settings,
        )?;
        let ArchivePreview { metadata, renames } = preview;
        let staging = self
            .files
            .existing_attempt_staging_dir(request.job_id, attempt_count)?;
        // Persist the immutable rename plan before mutating the attempt staging
        // tree. This is an auxiliary recovery fact outside the strict V1
        // manifest, so it never conflicts with `#[serde(deny_unknown_fields)]`.
        let pending = self.build_pending_renames(&staging, &renames)?;
        fs::write(
            staging.join(RENAME_PLAN_FILE),
            serde_json::to_vec_pretty(&pending)?,
        )?;

        fs::write(
            staging.join("tweet.json"),
            serde_json::to_vec_pretty(&metadata)?,
        )?;
        let author = metadata.author.display_name.as_deref().unwrap_or("");
        let username = metadata.author.username.as_deref().unwrap_or("");
        fs::write(
            staging.join("tweet.txt"),
            format!("{author}\n@{username}\n\n{}\n", metadata.text),
        )?;
        let manifest = ArchiveRecoveryManifest::from_metadata(
            request.job_id,
            attempt_count,
            request.tweet_row_id,
            request.final_directory,
            &metadata,
            request.telegram_intent.map(|intent| intent.tweet_row_id),
        )?;
        fs::write(
            staging.join(".xarchive-recovery.json"),
            serde_json::to_vec(&manifest)?,
        )?;
        self.database
            .create_archive_recovery_manifest(&manifest, request.archived_at)?;
        let prepared = self
            .database
            .verify_archive_recovery_manifest(request.job_id)?;
        if prepared != manifest {
            return Err(StorageError::InvalidState(
                "prepared recovery manifest changed before attempt commit".into(),
            ));
        }

        // Execute the persisted renames now that PREPARED is durable. A crash
        // here is recovered by replaying the pending plan from disk.
        self.apply_pending_renames(&staging, &pending, failpoint)?;

        let committed = self.files.commit_attempt_staging(
            request.job_id,
            attempt_count,
            request.final_directory,
        )?;
        self.database.update_tweet_metadata(
            request.tweet_row_id,
            &metadata,
            &request.final_directory.to_string_lossy(),
        )?;
        self.refresh_author_profile(request.tweet_row_id, &metadata)?;
        for media in &metadata.media {
            self.database
                .insert_media(request.tweet_row_id, media, &metadata.archived_at)?;
        }
        self.database
            .recover_archive_rows_from_manifest(&manifest, request.archived_at)?;
        self.database
            .commit_archive_recovery_manifest(request.job_id, request.archived_at)?;
        if request.telegram_intent.is_some() {
            self.database.transition_telegram_archive_intent(
                request.tweet_row_id,
                "PREPARED",
                "ARCHIVED",
                request.archived_at,
            )?;
        }
        Ok(committed)
    }

    /// Build durable rename facts from the planned source/target pairs by
    /// hashing the current staging files. Identity renames are skipped.
    fn build_pending_renames(
        &self,
        staging: &Path,
        renames: &[(String, String)],
    ) -> Result<Vec<PendingRename>, StorageError> {
        let mut pending = Vec::new();
        for (source, target) in renames {
            if source == target {
                continue;
            }
            let source_path = FileStore::resolve_within(staging, Path::new(source))?;
            let target_path = FileStore::resolve_within(staging, Path::new(target))?;
            if target_path.exists() {
                return Err(StorageError::InvalidState(
                    "attempt archive rename destination already exists".into(),
                ));
            }
            let metadata = fs::symlink_metadata(&source_path)?;
            if !metadata.file_type().is_file() {
                return Err(StorageError::InvalidPath);
            }
            pending.push(PendingRename {
                source_path: source.clone(),
                final_path: target.clone(),
                size_bytes: metadata.len(),
                sha256: FileStore::sha256(&source_path)?,
            });
        }
        Ok(pending)
    }

    /// Apply persisted renames against the attempt staging tree. Already-complete
    /// edges (target present) are idempotent no-ops; missing sources fail closed.
    /// `failpoint` injects a crash after the rename at the given zero-based index
    /// so crash-window tests can prove replay convergence.
    fn apply_pending_renames(
        &self,
        staging: &Path,
        pending: &[PendingRename],
        failpoint: Option<ArchiveRecoveryFailpoint>,
    ) -> Result<(), StorageError> {
        for (index, rename) in pending.iter().enumerate() {
            let source = FileStore::resolve_within(staging, Path::new(&rename.source_path))?;
            let target = FileStore::resolve_within(staging, Path::new(&rename.final_path))?;
            match (source.is_file(), target.exists()) {
                (true, false) => {
                    fs::rename(&source, &target)?;
                    if matches!(
                        failpoint,
                        Some(ArchiveRecoveryFailpoint::AfterFinalRename(i)) if i == index
                    ) {
                        return Err(StorageError::InvalidState(
                            "injected v1 publish crash after rename".into(),
                        ));
                    }
                }
                (false, true) => {}
                (true, true) => {
                    return Err(StorageError::InvalidState(
                        "attempt archive rename destination already exists".into(),
                    ));
                }
                (false, false) => return Err(StorageError::InvalidPath),
            }
        }
        Ok(())
    }

    /// Compose the Storage half of the v2 forward publish pipeline.
    ///
    /// The caller has already completed writer drain and verified the
    /// attempt-owned payload; this method freezes the candidate plus manifest,
    /// persists them as `PREPARED`, performs the journaled no-replace
    /// two-phase rename, commits the attempt staging tree, and closes with
    /// the attempt-fenced transactional finalization. Every filesystem
    /// boundary is failpoint-injectable so crash-window tests can prove replay
    /// convergence through [`Self::recover_internal_rename_v2`].
    ///
    /// The `InternalRenameV2` enrollment gate stays: this method only runs
    /// for a job enrolled in that contract, and production jobs keep enrolling
    /// `InternalV1`, so v2 acceptance remains fail-closed until the P6 gate.
    ///
    /// Authorization: `permit` is a runner-minted, move-only token binding
    /// the attempt identity and destination directory. Storage rejects the
    /// publish when either does not match the manifest identity and
    /// `final_directory`.
    #[allow(dead_code)]
    pub(crate) fn publish_prepared_internal_rename_v2(
        &mut self,
        manifest: &ArchiveRecoveryManifestV2,
        metadata: &ArchiveMetadata,
        final_directory: &Path,
        now: &str,
        failpoint: Option<ArchiveRecoveryFailpoint>,
        permit: PublicationPermit<'_>,
    ) -> Result<PathBuf, StorageError> {
        self.publish_prepared_internal_rename_v2_with_exports(
            manifest,
            metadata,
            final_directory,
            now,
            failpoint,
            None,
            permit,
        )
    }

    /// D1 supplies already serialized exports of the same metadata DTO. Legacy
    /// callers retain JSON/TXT behavior. Recovery never reads optional exports.
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub(crate) fn publish_prepared_internal_rename_v2_with_exports(
        &mut self,
        manifest: &ArchiveRecoveryManifestV2,
        metadata: &ArchiveMetadata,
        final_directory: &Path,
        now: &str,
        failpoint: Option<ArchiveRecoveryFailpoint>,
        exports: Option<&[(&str, Vec<u8>)]>,
        permit: PublicationPermit<'_>,
    ) -> Result<PathBuf, StorageError> {
        self.publish_prepared_internal_rename_v2_core(
            manifest,
            metadata,
            final_directory,
            now,
            failpoint,
            exports,
            Some(permit),
        )
    }

    /// V2 unit tests exercise its private journal/replay mechanics without
    /// manufacturing production publication authority. This entry point is not
    /// compiled into production and cannot be called outside this crate.
    #[cfg(test)]
    pub(crate) fn publish_prepared_internal_rename_v2_for_test(
        &mut self,
        manifest: &ArchiveRecoveryManifestV2,
        metadata: &ArchiveMetadata,
        final_directory: &Path,
        now: &str,
        failpoint: Option<ArchiveRecoveryFailpoint>,
    ) -> Result<PathBuf, StorageError> {
        self.publish_prepared_internal_rename_v2_core(
            manifest,
            metadata,
            final_directory,
            now,
            failpoint,
            None,
            None,
        )
    }

    #[cfg(test)]
    pub(crate) fn publish_prepared_internal_rename_v2_with_exports_for_test(
        &mut self,
        manifest: &ArchiveRecoveryManifestV2,
        metadata: &ArchiveMetadata,
        final_directory: &Path,
        now: &str,
        failpoint: Option<ArchiveRecoveryFailpoint>,
        exports: Option<&[(&str, Vec<u8>)]>,
    ) -> Result<PathBuf, StorageError> {
        self.publish_prepared_internal_rename_v2_core(
            manifest,
            metadata,
            final_directory,
            now,
            failpoint,
            exports,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn publish_prepared_internal_rename_v2_core(
        &mut self,
        manifest: &ArchiveRecoveryManifestV2,
        metadata: &ArchiveMetadata,
        final_directory: &Path,
        now: &str,
        failpoint: Option<ArchiveRecoveryFailpoint>,
        exports: Option<&[(&str, Vec<u8>)]>,
        permit: Option<PublicationPermit<'_>>,
    ) -> Result<PathBuf, StorageError> {
        manifest.validate()?;
        if let Some(selection) = &manifest.metadata_output {
            let exports = exports.ok_or_else(|| {
                StorageError::InvalidMetadata(
                    "D1 exports must be serialized before publication".into(),
                )
            })?;
            if selection.export_yaml != exports.iter().any(|(name, _)| *name == "tweet.yaml") {
                return Err(StorageError::InvalidMetadata(
                    "D1 YAML selection mismatch".into(),
                ));
            }
        } else if exports.is_some() {
            return Err(StorageError::InvalidMetadata(
                "D1 exports require explicit recovery selection".into(),
            ));
        }
        if let Some(exports) = exports {
            let mut names = std::collections::HashSet::new();
            if manifest.export_text
                || exports.iter().any(|(name, _)| {
                    !matches!(*name, "tweet.json" | "tweet.yaml") || !names.insert(*name)
                })
                || manifest.export_json != names.contains("tweet.json")
            {
                return Err(StorageError::InvalidMetadata(
                    "invalid D1 metadata exports".into(),
                ));
            }
        }
        if self.database.archive_attempt(&manifest.job_id)? != manifest.attempt_count
            || self.database.archive_recovery_contract(&manifest.job_id)? != Some(2)
            || self.database.job_state(&manifest.job_id)?.is_terminal()
        {
            return Err(StorageError::InvalidState(
                "v2 publication identity is not current".into(),
            ));
        }
        if let Some(permit) = permit {
            if permit.attempt().job_id() != manifest.job_id
                || permit.attempt().attempt() != manifest.attempt_count
            {
                return Err(StorageError::InvalidState(
                    "publication permit does not match manifest attempt identity".into(),
                ));
            }
            let permit_destination = self.files.archive_path(final_directory)?;
            if permit.destination() != permit_destination {
                return Err(StorageError::InvalidState(
                    "publication permit destination does not match publication directory".into(),
                ));
            }
        }
        // Historical execution snapshots retain JSON/TXT semantics. Export
        // serialization and create-new writes precede PREPARED; replay uses the
        // durable candidate, never these optional user files. A prepared retry
        // must not attempt to recreate exports already fixed by that boundary.
        if self
            .database
            .archive_recovery_manifest_v2(&manifest.job_id)?
            .is_none()
        {
            if let Some(exports) = exports {
                for (name, bytes) in exports {
                    self.files.create_attempt_file(
                        &manifest.job_id,
                        manifest.attempt_count,
                        name,
                        bytes,
                    )?;
                }
            } else {
                let json = manifest
                    .export_json
                    .then(|| serde_json::to_vec_pretty(metadata))
                    .transpose()?;
                let text = manifest.export_text.then(|| {
                    format!(
                        "{}\n@{}\n\n{}\n",
                        metadata.author.display_name.as_deref().unwrap_or(""),
                        metadata.author.username.as_deref().unwrap_or(""),
                        metadata.text,
                    )
                });
                if let Some(json) = json {
                    self.files.create_attempt_file(
                        &manifest.job_id,
                        manifest.attempt_count,
                        "tweet.json",
                        &json,
                    )?;
                }
                if let Some(text) = text {
                    self.files.create_attempt_file(
                        &manifest.job_id,
                        manifest.attempt_count,
                        "tweet.txt",
                        text.as_bytes(),
                    )?;
                }
            }
        }
        self.database
            .prepare_archive_recovery_v2(manifest, metadata, now)?;
        let job_id = manifest.job_id.as_str();
        let attempt = manifest.attempt_count;
        let edges: Vec<crate::workflow_bridge::RenameEdge> = manifest
            .rename_plan
            .iter()
            .map(|plan| {
                crate::workflow_bridge::RenameEdge::new(
                    plan.source_path.as_str(),
                    plan.temporary_path.as_str(),
                    plan.final_path.as_str(),
                )
                .map_err(|_| StorageError::InvalidMetadata("invalid v2 rename plan edge".into()))
            })
            .collect::<Result<_, _>>()?;
        let fail_at_temporary = matches!(
            failpoint,
            Some(ArchiveRecoveryFailpoint::AfterTemporaryRename(_))
        );
        let fail_at_final = matches!(
            failpoint,
            Some(ArchiveRecoveryFailpoint::AfterFinalRename(_))
        );
        let fail_at_commit = matches!(
            failpoint,
            Some(ArchiveRecoveryFailpoint::AfterDirectoryCommit)
        );
        self.files.replay_attempt_rename_stage(
            job_id,
            attempt,
            &edges,
            &manifest.rename_plan,
            true,
        )?;
        if fail_at_temporary {
            return Err(StorageError::InvalidState(
                "injected v2 publish crash after temporary rename".into(),
            ));
        }
        self.database
            .transition_archive_recovery_v2(job_id, "PLANNED", "TEMPORARY", now)?;
        self.files.replay_attempt_rename_stage(
            job_id,
            attempt,
            &edges,
            &manifest.rename_plan,
            false,
        )?;
        if fail_at_final {
            return Err(StorageError::InvalidState(
                "injected v2 publish crash after final rename".into(),
            ));
        }
        self.database
            .transition_archive_recovery_v2(job_id, "TEMPORARY", "FINAL", now)?;
        self.files
            .commit_attempt_staging(job_id, attempt, final_directory)?;
        if fail_at_commit {
            return Err(StorageError::InvalidState(
                "injected v2 publish crash after directory commit".into(),
            ));
        }
        let candidate = self
            .database
            .archive_recovery_v2_candidate(job_id, attempt, manifest.tweet_row_id)?
            .ok_or_else(|| StorageError::InvalidState("v2 recovery candidate is missing".into()))?;
        self.database.finalize_archive_recovery_v2(
            job_id,
            attempt,
            manifest.tweet_row_id,
            &candidate,
            manifest,
            now,
        )?;
        self.files.archive_path(final_directory)
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
        let attempt_staging_exists = self
            .files
            .recovery_attempt_directory_exists(job_id, manifest.attempt_count)?;
        let legacy_staging_exists = self.files.recovery_directory_exists(job_id, "_staging")?;
        let staging_exists = self.files.recovery_directory_exists(job_id, "_staging")?;
        let final_exists = self
            .files
            .recovery_directory_exists(job_id, final_directory)?;
        if attempt_staging_exists {
            if legacy_staging_exists || final_exists {
                return Err(StorageError::InvalidState(
                    "ambiguous attempt-scoped InternalV1 recovery directories".into(),
                ));
            }
            return self.recover_internal_archive_attempt(job_id, tweet_row_id, final_directory);
        }
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

    /// Replay an InternalV1 journal against its immutable `attempt-N` staging
    /// tree. The session-bound caller must establish the live writer drain and
    /// destination authorization before invoking this mutating recovery path.
    pub fn recover_internal_archive_attempt(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
    ) -> Result<PathBuf, StorageError> {
        let manifest = self.database.verify_archive_recovery_manifest(job_id)?;
        if manifest.attempt_count == 0
            || manifest.tweet_row_id != tweet_row_id
            || manifest.archive_directory != final_directory.to_string_lossy().replace('\\', "/")
            || self.database.archive_attempt(job_id)? != manifest.attempt_count
            || self.database.archive_recovery_contract(job_id)?
                != Some(ArchiveRecoveryContract::InternalV1.version())
        {
            return Err(StorageError::InvalidMetadata(
                "attempt recovery identity does not match the live journal".into(),
            ));
        }
        let destination = self.files.archive_path(final_directory)?;
        let staging_exists = self
            .files
            .recovery_attempt_directory_exists(job_id, manifest.attempt_count)?;
        let final_exists = self
            .files
            .recovery_directory_exists(job_id, final_directory)?;
        let journal = self
            .database
            .archive_recovery_journal(job_id)?
            .ok_or_else(|| {
                StorageError::InvalidState("internal recovery journal missing".into())
            })?;
        if journal.phase == ArchiveRecoveryPhase::Committed {
            if staging_exists || !final_exists {
                return Err(StorageError::InvalidState(
                    "committed attempt recovery has an unexpected directory state".into(),
                ));
            }
            self.verify_internal_attempt_recovery_tree(&manifest, &destination)?;
            return Ok(destination);
        }
        if staging_exists == final_exists {
            return Err(StorageError::InvalidState(
                "attempt recovery requires exactly one staging or final directory".into(),
            ));
        }
        let directory = if staging_exists {
            self.files
                .existing_attempt_staging_dir(job_id, manifest.attempt_count)?
        } else {
            destination.clone()
        };
        // Replay persisted renames before verifying the manifest, so a crash
        // during publishing converges to the final media paths. The plan is an
        // auxiliary fact outside the strict V1 manifest; its absence means no
        // renames were ever planned for this attempt.
        if staging_exists {
            let plan_path = directory.join(RENAME_PLAN_FILE);
            if plan_path.is_file() {
                let pending: Vec<PendingRename> = serde_json::from_slice(&fs::read(&plan_path)?)?;
                self.apply_pending_renames(&directory, &pending, None)?;
            }
        }
        self.verify_internal_attempt_recovery_tree(&manifest, &directory)?;
        let result = if staging_exists {
            self.files
                .commit_attempt_staging(job_id, manifest.attempt_count, final_directory)?
        } else {
            destination
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

    fn verify_internal_attempt_recovery_tree(
        &self,
        manifest: &ArchiveRecoveryManifest,
        directory: &Path,
    ) -> Result<(), StorageError> {
        let internal_manifest_path =
            FileStore::resolve_within(&directory, Path::new(".xarchive-recovery.json"))?;
        let disk_manifest: ArchiveRecoveryManifest =
            serde_json::from_slice(&fs::read(internal_manifest_path)?)?;
        disk_manifest.validate()?;
        if &disk_manifest != manifest {
            return Err(StorageError::InvalidMetadata(
                "attempt recovery manifest does not match the journal".into(),
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
                    "attempt recovery media integrity mismatch".into(),
                ));
            }
        }
        Ok(())
    }

    /// Replay a PREPARED (or already-committed) v2 rename journal from actual
    /// on-disk state, then transactionally finalize the durable archive facts.
    ///
    /// This is the crash-recovery half of the two-phase rename pipeline: it
    /// re-derives progress from the filesystem rather than trusting an
    /// in-memory cursor, so an interruption at any rename boundary converges.
    ///
    /// Fail-closed by construction: the immutable manifest and its attempt-bound
    /// candidate are re-verified through [`Database::archive_recovery_manifest_v2`]
    /// (which rejects a missing/corrupt journal, an attempt that no longer matches
    /// the active Job, or a missing candidate); the caller's target must match
    /// the journal's identity; every rename edge is replayed only when its
    /// recorded size/hash matches the file actually present; and the durable
    /// facts commit only through the fenced [`Database::finalize_archive_recovery_v2`]
    /// transaction, which requires the FINAL prepared journal and rolls back on a
    /// stale attempt. A COMMITTED journal is read-only verified and never re-planned.
    pub fn recover_internal_rename_v2(
        &mut self,
        job_id: &str,
        tweet_row_id: i64,
        final_directory: &Path,
        now: &str,
    ) -> Result<PathBuf, StorageError> {
        let journal = self
            .database
            .archive_recovery_manifest_v2(job_id)?
            .ok_or_else(|| {
                StorageError::InvalidState("v2 archive recovery journal is missing".into())
            })?;
        if journal.tweet_row_id != tweet_row_id
            || journal.archive_directory != final_directory.to_string_lossy().replace('\\', "/")
        {
            return Err(StorageError::InvalidMetadata(
                "v2 recovery target does not match the journal".into(),
            ));
        }
        let manifest: ArchiveRecoveryManifestV2 = serde_json::from_str(&journal.manifest_json)?;
        manifest.validate()?;
        // A COMMITTED journal owns the archive: verify only, never re-plan or
        // re-rename. Returning a path alone is not media-integrity evidence.
        if journal.phase == ArchiveRecoveryPhase::Committed {
            let facts = self
                .database
                .tweet_archive_facts(&manifest.tweet_id)?
                .ok_or_else(|| {
                    StorageError::InvalidMetadata(
                        "committed v2 archive database facts missing".into(),
                    )
                })?;
            if facts.archive_directory != manifest.archive_directory
                || facts.media.len() != manifest.media.len()
                || manifest.media.iter().any(|expected| {
                    !facts.media.iter().any(|actual| {
                        actual.media_index == expected.media_index
                            && actual.relative_path == expected.relative_path
                            && actual.media_id == expected.media_id
                            && actual.media_type == expected.media_type
                            && actual.mime_type == expected.mime_type
                            && actual.size_bytes == Some(expected.size_bytes)
                            && actual.sha256.as_deref() == Some(expected.sha256.as_str())
                    })
                })
            {
                return Err(StorageError::InvalidMetadata(
                    "committed v2 archive database facts mismatch".into(),
                ));
            }
            let committed = self.files.archive_path(final_directory)?;
            for fact in &manifest.media {
                let path = FileStore::resolve_within(&committed, Path::new(&fact.relative_path))?;
                let metadata = fs::symlink_metadata(&path)?;
                if !metadata.file_type().is_file()
                    || crate::file_store::is_reparse_point(&metadata)
                    || metadata.len() != fact.size_bytes
                    || FileStore::sha256(&path)? != fact.sha256
                {
                    return Err(StorageError::InvalidMetadata(
                        "committed v2 archive media integrity mismatch".into(),
                    ));
                }
            }
            return Ok(committed);
        }
        let progress = journal.rename_progress.as_str();
        // A crash between the atomic directory commit and the finalize
        // transaction leaves the durable archive directory in place while the
        // journal is still PREPARED/FINAL and the attempt staging tree is gone.
        // Detect that committed state from the filesystem, verify the media
        // facts against the immutable manifest, and finalize without re-planning
        // or re-renaming. This is the only path that may skip the rename replay.
        let destination_already_committed = self
            .files
            .recovery_directory_exists(job_id, final_directory)?;
        if !destination_already_committed {
            // Build the immutable whole-phase edges once; the plan never changes
            // during replay, so an edge that does not match its identity is
            // corruption.
            let mut edges = Vec::with_capacity(manifest.rename_plan.len());
            for plan in &manifest.rename_plan {
                edges.push(
                    crate::workflow_bridge::RenameEdge::new(
                        &plan.source_path,
                        &plan.temporary_path,
                        &plan.final_path,
                    )
                    .map_err(|_| {
                        StorageError::InvalidMetadata("invalid v2 rename plan edge".into())
                    })?,
                );
            }
            // Replay each whole phase from actual on-disk state. A crash after
            // the files moved but before the journal advanced is idempotent: the
            // replay sees the completed destination and the phase transition then
            // lands.
            if progress == "PLANNED" {
                self.files.replay_attempt_rename_stage(
                    job_id,
                    journal.attempt_count,
                    &edges,
                    &manifest.rename_plan,
                    true,
                )?;
                self.database.transition_archive_recovery_v2(
                    job_id,
                    "PLANNED",
                    "TEMPORARY",
                    now,
                )?;
            }
            if matches!(progress, "PLANNED" | "TEMPORARY") {
                self.files.replay_attempt_rename_stage(
                    job_id,
                    journal.attempt_count,
                    &edges,
                    &manifest.rename_plan,
                    false,
                )?;
                self.database
                    .transition_archive_recovery_v2(job_id, "TEMPORARY", "FINAL", now)?;
            }
            // Both rename phases placed every media file at its final relative
            // path inside the attempt staging tree. Commit that tree to the
            // durable archive directory with an atomic no-replace move before
            // the database facts are finalized, so a crash cannot leave final
            // files at the staging location while the journal claims otherwise.
            self.files
                .commit_attempt_staging(job_id, journal.attempt_count, final_directory)?;
        } else {
            // The directory is already committed: verify every recorded media
            // fact against the file actually present before finalizing, so a
            // truncated or swapped payload fails closed instead of being
            // accepted as the committed archive.
            let committed = self.files.archive_path(final_directory)?;
            for fact in &manifest.media {
                let path = FileStore::resolve_within(&committed, Path::new(&fact.relative_path))?;
                let metadata = fs::symlink_metadata(&path)?;
                if !metadata.file_type().is_file()
                    || crate::file_store::is_reparse_point(&metadata)
                    || metadata.len() != fact.size_bytes
                    || FileStore::sha256(&path)? != fact.sha256
                {
                    return Err(StorageError::InvalidMetadata(
                        "committed v2 archive media integrity mismatch".into(),
                    ));
                }
            }
        }
        let candidate = self
            .database
            .archive_recovery_v2_candidate(job_id, journal.attempt_count, journal.tweet_row_id)?
            .ok_or_else(|| StorageError::InvalidState("v2 recovery candidate is missing".into()))?;
        self.database.finalize_archive_recovery_v2(
            job_id,
            journal.attempt_count,
            journal.tweet_row_id,
            &candidate,
            &manifest,
            now,
        )?;
        self.files.archive_path(final_directory)
    }
}
