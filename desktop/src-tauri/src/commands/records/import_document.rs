//! Record import document parsing and preview.
//!
//! Split out of `commands/records.rs` without changing any command name,
//! argument shape, serialized field or error string. The rules:
//!
//! * both serialized forms enter the same strict, versioned core parser, and an
//!   unversioned document is refused rather than guessed;
//! * a preview resolves evidence for every record, so the UI never shows a
//!   partially verified selection;
//! * the revision binds the raw text and the root identity, so a changed file or
//!   a changed directory invalidates an earlier preview instead of silently
//!   reusing it.

use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};
use xarchive_core::DownloadRecord;
use xarchive_storage::record_evidence::{RecordEvidenceVerdict, preview_download_records};

use super::record_import_media_probe;

/// Flat JSON response for F3 record-import acceptance. Matches the
/// frontend's expected shape: a flat `status` with all outcome fields at
/// the top level, so the caller does not need to discriminate on enum
/// variant names.
#[derive(Serialize)]
pub(crate) struct RecordImportOutcomeResponse {
    pub status: String,
    pub accepted: usize,
    pub identical: usize,
    pub identities: Vec<String>,
    pub failures: Vec<xarchive_storage::record_import::RecordImportCommitFailure>,
}

/// Per-record preview entry for the F3 import accept surface.
#[derive(Serialize)]
pub(crate) struct RecordImportPreviewEntry {
    pub identity: String,
    pub verdict: String,
    pub reason: Option<String>,
}

/// Whole-document preview response for `preview_record_import`.
#[derive(Serialize)]
pub(crate) struct RecordImportPreviewResponse {
    pub entries: Vec<RecordImportPreviewEntry>,
    pub revision: String,
}

/// Both serialized forms enter the same strict, versioned core parser.
pub(super) enum RecordImportDocument {
    Download(xarchive_core::DownloadRecordsDocument),
    Archive(xarchive_core::ArchiveRecordsDocument),
}

impl RecordImportDocument {
    pub(super) fn parse(raw: &str) -> Result<Self, String> {
        let json = raw.trim_start().starts_with('{');
        let download = if json {
            xarchive_core::DownloadRecordsDocument::from_json(raw)
        } else {
            xarchive_core::DownloadRecordsDocument::from_yaml(raw)
        };
        match download {
            Ok(document) => Ok(Self::Download(document)),
            Err(download_error) => {
                let archive = if json {
                    xarchive_core::ArchiveRecordsDocument::from_json(raw)
                } else {
                    xarchive_core::ArchiveRecordsDocument::from_yaml(raw)
                };
                archive.map(Self::Archive).map_err(|archive_error| {
                    format!("invalid record document: {download_error}; {archive_error}")
                })
            }
        }
    }

    pub(super) fn records(&self) -> Vec<DownloadRecord> {
        match self {
            Self::Download(document) => document.records.clone(),
            Self::Archive(document) => document
                .records
                .iter()
                .map(|r| r.as_download_record())
                .collect(),
        }
    }

    pub(super) fn evidence(&self, root: &Path) -> Vec<RecordEvidenceVerdict> {
        match self {
            Self::Download(document) => {
                preview_download_records(&document.records, root, record_import_media_probe)
                    .into_iter()
                    .map(|(_, verdict)| verdict)
                    .collect()
            }
            Self::Archive(document) => document
                .records
                .iter()
                .map(|record| {
                    xarchive_storage::record_evidence::preview_archive_record(
                        record,
                        root,
                        record_import_media_probe,
                    )
                })
                .collect(),
        }
    }
}

pub(super) fn record_import_root(root: &str) -> Result<std::path::PathBuf, String> {
    if root.trim().is_empty() {
        return Err("archive root must not be empty".into());
    }
    let root = std::fs::canonicalize(root).map_err(|error| format!("archive root: {error}"))?;
    if !root.is_dir() {
        return Err("archive root is not a directory".into());
    }
    Ok(root)
}

pub(super) fn record_import_revision(raw: &str, root: &Path) -> String {
    let mut digest = Sha256::new();
    digest.update(raw.as_bytes());
    digest.update([0]);
    digest.update(root.as_os_str().as_encoded_bytes());
    #[cfg(unix)]
    if let Ok(metadata) = std::fs::metadata(root) {
        use std::os::unix::fs::MetadataExt;
        digest.update(metadata.dev().to_le_bytes());
        digest.update(metadata.ino().to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}
