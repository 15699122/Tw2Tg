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
    /// Optional user-facing metadata exports, already serialized from the same
    /// canonical DTO. `None` keeps the historical behaviour (write `tweet.json`
    /// and `tweet.txt` unconditionally). `Some` writes exactly the supplied
    /// files, so the D1 switch matrix can suppress either or both formats.
    ///
    /// Attempt-scoped recovery never reads these files: it verifies the
    /// `.xarchive-recovery.json` manifest and the media facts instead, so a job
    /// whose exports are disabled still recovers.
    pub metadata_exports: Option<&'a [(&'a str, Vec<u8>)]>,
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

// The inherent `impl` is split by complete archival operation. The type, its
// public surface and the module path are unchanged; each file owns one outcome.
mod internal_v1_publication;
mod internal_v1_recovery;
mod legacy_completion;
mod telegram_intent;
mod v2_rename;

impl ArchiveService {
    pub fn new(database: Database, files: FileStore) -> Self {
        Self { database, files }
    }
}
