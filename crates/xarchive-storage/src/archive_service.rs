use std::fs;
use std::path::{Path, PathBuf};

use xarchive_core::{ArchiveMetadata, JobState};

use crate::{Database, FileStore, StorageError, UserProfileFile, build_archive_metadata};

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
        fs::write(&json_path, serde_json::to_vec_pretty(metadata)?)?;
        let text = format!(
            "{}\n@{}\n\n{}\n",
            metadata.author.display_name.as_deref().unwrap_or(""),
            metadata.author.username.as_deref().unwrap_or(""),
            metadata.text
        );
        fs::write(staging.join("tweet.txt"), text)?;

        let committed = self.files.commit_staging(job_id, final_directory)?;
        let relative_directory = final_directory.to_string_lossy().to_string();
        self.database
            .update_tweet_metadata(tweet_row_id, metadata, &relative_directory)?;
        self.refresh_author_profile(tweet_row_id, metadata)?;
        for media in &metadata.media {
            self.database
                .insert_media(tweet_row_id, media, &metadata.archived_at)?;
        }
        self.database
            .transition_job(job_id, JobState::Downloaded, &metadata.archived_at)?;
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
        self.complete_local_archive(
            request.job_id,
            request.tweet_row_id,
            &archive_metadata,
            request.final_directory,
        )
    }
}
