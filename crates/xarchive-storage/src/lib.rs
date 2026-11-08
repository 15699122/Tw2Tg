//! SQLite persistence and local file storage for the Desktop application.

mod archive_recovery;
mod archive_recovery_manifest;
mod archive_service;
mod database {
    pub mod batches;
    pub mod jobs;
    pub mod settings;
    pub mod tags;
    pub mod telegram;
    pub mod telegram_outbox;
    pub mod tweets;
    pub mod users;
}
mod archive_completeness;
mod connection;
mod error;
mod file_store;
mod metadata;
mod models;
mod naming;
pub mod record_evidence;
pub mod record_import;
mod recovery_candidate;
mod telegram_archive_intents;
mod telegram_credentials;
pub mod workflow_bridge;

pub use archive_completeness::{
    ArchiveCompleteness, ArchiveCompletenessOptions, CompletenessIssue,
    evaluate_archive_completeness, safe_media_path,
};
pub use archive_recovery::{
    ArchiveCancellationDecision, ArchiveRecoveryContract, ArchiveRecoveryDispatch,
    ArchiveRecoveryJournalRecord, ArchiveRecoveryManifestV2Record, ArchiveRecoveryPhase,
};
pub use archive_recovery_manifest::{
    ArchiveRecoveryManifest, ArchiveRecoveryManifestV2, ArchiveRecoveryMediaFact,
    ArchiveRenamePlan, MetadataExportSelection,
};
pub use archive_service::{
    ArchivePreview, ArchiveRecoveryFailpoint, ArchiveService, SidecarArchiveRequest,
};
pub use database::batches::{
    AccountBatchSummary, BatchCandidateRecord, BatchCounts, BatchNamingMode, BatchOutputSettings,
    CreateAccountBatch, NewBatchCandidate,
};
pub use error::StorageError;
pub use file_store::FileStore;
pub use metadata::build_archive_metadata;
pub use models::{
    ArchivedMediaFact, JobDownloadMetrics, JobEventRecord, JobMetrics, JobSummary, PagedJobs,
    SettingEntry, TelegramArchiveIntentRecord, TweetArchiveFacts, TweetRelationships,
    UserNameSummary, UserProfileFile, UserProfileSnapshot, UserSummary,
};
pub use naming::{
    plan_archive_media_renames, render_archive_metadata_paths, render_media_filenames,
};

use rusqlite::Connection;
use std::path::Path;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Ok(Self {
            connection: connection::open(path)?,
        })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        Ok(Self {
            connection: connection::open_in_memory()?,
        })
    }

    /// Execute a narrow database operation for crate consumers that need a
    /// transactional recovery/state repair unsupported by the public API.
    pub fn execute_batch(&self, sql: &str) -> Result<(), StorageError> {
        self.connection.execute_batch(sql)?;
        Ok(())
    }

    /// Test-only access to the underlying connection for starting transactions.
    #[cfg(test)]
    pub(crate) fn connection_for_test(&self) -> &Connection {
        &self.connection
    }
}

#[cfg(test)]
mod tests;
