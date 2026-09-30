//! Media-spec completeness policy (P2-C).
//!
//! "Already archived" must never degrade into "a file with that name exists".
//! The decision is made from durable facts plus the bytes on disk:
//!
//! * the committed directory must exist;
//! * every recorded media row must exist on disk;
//! * when the caller knows how many media the Tweet should have (discovery
//!   metadata, `media_count`), the recorded row count must match it, so a
//!   partial download is never treated as complete;
//! * recorded sizes must match the file on disk, which catches truncated or
//!   replaced files without hashing gigabytes of video;
//! * SHA-256 verification is opt-in through
//!   [`ArchiveCompletenessOptions::verify_digest`] and is reserved for
//!   explicit verification flows, not for the hot dispatch path.
//!
//! A Tweet without media is complete when its directory exists, which keeps
//! text-only Tweets archivable without inventing a media expectation.

use std::path::{Component, Path, PathBuf};

use crate::{ArchivedMediaFact, FileStore, TweetArchiveFacts};

/// One reason an archive is not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletenessIssue {
    /// The committed archive directory is missing or is not a directory.
    MissingDirectory { archive_directory: String },
    /// A recorded media row has no usable archive-relative path.
    MediaPathInvalid { media_index: u32 },
    /// A recorded media file is absent from the committed directory.
    MissingMedia { relative_path: String },
    /// The file size does not match the recorded `size_bytes`.
    SizeMismatch {
        relative_path: String,
        expected: u64,
        actual: u64,
    },
    /// The recorded SHA-256 does not match the file on disk.
    DigestMismatch { relative_path: String },
    /// The expected media count is higher than the number of recorded rows.
    MediaNotRecorded { expected: u64, recorded: u64 },
}

/// Verdict for one Tweet archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveCompleteness {
    Complete,
    Incomplete(Vec<CompletenessIssue>),
}

impl ArchiveCompleteness {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    pub fn issues(&self) -> &[CompletenessIssue] {
        match self {
            Self::Complete => &[],
            Self::Incomplete(issues) => issues,
        }
    }
}

/// Options for one completeness decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArchiveCompletenessOptions {
    /// Re-hash every recorded media file and compare against the recorded
    /// SHA-256. Off by default because it reads the whole archive.
    pub verify_digest: bool,
}

impl ArchiveCompletenessOptions {
    /// Existence + size + expected-media-count verification (default).
    pub fn fast() -> Self {
        Self::default()
    }

    /// Adds SHA-256 verification on top of the default checks.
    pub fn with_digest_verification() -> Self {
        Self {
            verify_digest: true,
        }
    }
}

/// Join a persisted media path onto its archive directory without letting the
/// path escape that directory.
pub fn safe_media_path(directory: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(directory.join(relative))
}

/// Decide whether a Tweet's committed archive is complete.
///
/// `expected_media_count` is what the caller independently believes the Tweet
/// should hold (discovery `media_count`); `None` means "unknown", in which case
/// only the recorded media rows are verified.
pub fn evaluate_archive_completeness(
    files: &FileStore,
    facts: &TweetArchiveFacts,
    expected_media_count: Option<u64>,
    options: &ArchiveCompletenessOptions,
) -> ArchiveCompleteness {
    let mut issues = Vec::new();

    let directory = match files.archive_path(&facts.archive_directory) {
        Ok(directory) if directory.is_dir() => Some(directory),
        _ => {
            issues.push(CompletenessIssue::MissingDirectory {
                archive_directory: facts.archive_directory.clone(),
            });
            None
        }
    };

    if let Some(expected) = expected_media_count {
        let recorded = facts.media.len() as u64;
        if expected > recorded {
            issues.push(CompletenessIssue::MediaNotRecorded { expected, recorded });
        }
    }

    for media in &facts.media {
        inspect_media(&mut issues, directory.as_deref(), media, options);
    }

    if issues.is_empty() {
        ArchiveCompleteness::Complete
    } else {
        ArchiveCompleteness::Incomplete(issues)
    }
}

fn inspect_media(
    issues: &mut Vec<CompletenessIssue>,
    directory: Option<&Path>,
    media: &ArchivedMediaFact,
    options: &ArchiveCompletenessOptions,
) {
    let Some(directory) = directory else {
        return;
    };
    let Some(path) = safe_media_path(directory, &media.relative_path) else {
        issues.push(CompletenessIssue::MediaPathInvalid {
            media_index: media.media_index,
        });
        return;
    };
    let Ok(metadata) = std::fs::metadata(&path) else {
        issues.push(CompletenessIssue::MissingMedia {
            relative_path: media.relative_path.clone(),
        });
        return;
    };
    if !metadata.is_file() {
        issues.push(CompletenessIssue::MissingMedia {
            relative_path: media.relative_path.clone(),
        });
        return;
    }
    if let Some(expected) = media.size_bytes {
        let actual = metadata.len();
        if expected != actual {
            issues.push(CompletenessIssue::SizeMismatch {
                relative_path: media.relative_path.clone(),
                expected,
                actual,
            });
            return;
        }
    }
    if options.verify_digest
        && let Some(expected) = media.sha256.as_deref()
        && FileStore::sha256(&path).is_ok_and(|actual| actual != expected)
    {
        issues.push(CompletenessIssue::DigestMismatch {
            relative_path: media.relative_path.clone(),
        });
    }
}
