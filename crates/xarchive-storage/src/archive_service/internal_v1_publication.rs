//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl ArchiveService {
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

    /// Register the archive author and refresh their portable profile file.
    ///
    /// Tweets without a resolvable `user_id` skip registration silently:
    /// no user row is created and no profile file is written.
    pub(super) fn refresh_author_profile(
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

    /// V1 unit tests exercise the attempt-scoped publication mechanics and
    /// their crash windows without manufacturing a production publication
    /// permit. This entry point is not compiled into production builds and
    /// cannot be called outside this crate. It mirrors
    /// [`Self::publish_prepared_internal_rename_v2_for_test`]; every check the
    /// production path performs before `complete_sidecar_archive_attempt_inner`
    /// still runs here, so a test cannot use it to bypass attempt, contract,
    /// destination, or current-attempt validation.
    #[cfg(test)]
    pub(crate) fn complete_sidecar_archive_attempt_for_test(
        &mut self,
        request: SidecarArchiveRequest<'_>,
        attempt_count: u32,
        failpoint: Option<ArchiveRecoveryFailpoint>,
    ) -> Result<PathBuf, StorageError> {
        let attempt_identity =
            xarchive_workflow::AttemptIdentity::new(request.job_id, attempt_count)
                .map_err(|error| StorageError::InvalidState(error.to_string()))?;
        let destination = self.files.archive_path(request.final_directory)?;
        self.complete_sidecar_archive_attempt_inner(
            request,
            attempt_count,
            failpoint,
            &attempt_identity,
            &destination,
        )
    }

    pub(super) fn complete_sidecar_archive_attempt_inner(
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
        // The committed-directory/database-not-converged boundary is a real
        // crash window: the rename is durable but no archive facts or journal
        // convergence exists yet. Replay must converge it idempotently.
        if matches!(
            failpoint,
            Some(ArchiveRecoveryFailpoint::AfterDirectoryCommit)
        ) {
            return Err(StorageError::InvalidState(
                "injected v1 publish crash after directory commit".into(),
            ));
        }
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
    pub(super) fn build_pending_renames(
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
    pub(super) fn apply_pending_renames(
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
}
