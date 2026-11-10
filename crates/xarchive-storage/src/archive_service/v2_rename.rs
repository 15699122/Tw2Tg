//! `ArchiveService` — see the parent module for the type and its contract.
//!
//! The inherent `impl` is split by complete operation so each file holds one
//! archival outcome. Signatures, visibility and bodies are unchanged; only the
//! file that defines them moved.

use super::*;

impl ArchiveService {
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
}
