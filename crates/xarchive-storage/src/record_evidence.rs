//! F3 record-evidence binding and accept/preserve/reject preview.
//!
//! Pure pre-import evaluation for one record against a user-selected archive
//! root. `final = archive_root.join(record.relative_path)` is resolved through
//! the same containment contract Storage uses everywhere else: absolute paths
//! and `..` escapes are rejected, and every existing link below the root is
//! rejected, so a record can never point preview — let alone a later apply —
//! outside the selected root. This module creates no directories and writes no
//! durable state; the maintenance-guarded single-transaction apply lives in a
//! later batch.

use std::path::{Component, Path};

use xarchive_core::{ArchiveRecord, DownloadRecord, RecordIdentity};

/// Outcome of evaluating one incoming record's on-disk evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordEvidenceVerdict {
    /// Download-record media evidence re-verified (all media regular files with
    /// matching size and SHA-256), or a record with no media constraints.
    Accept,
    /// Kept back with a human-readable reason; nothing durable changed.
    Preserve { reason: String },
}

/// Evaluate one media entry: resolve `archive_root.join(file)` under the
/// containment contract, then require a regular file whose size and SHA-256
/// match the record. `check` abstracts the on-disk probe so unit tests run
/// without touching the real filesystem.
fn media_path(root: &Path, file: &str) -> Result<std::path::PathBuf, String> {
    let relative = Path::new(file);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || file.contains('\\')
        || file.contains(':')
    {
        return Err(format!(
            "record media path escapes the archive root: {file}"
        ));
    }
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "record media path escapes the archive root: {file}"
        ));
    }
    let mut resolved = root.to_path_buf();
    for component in relative.components() {
        resolved.push(component);
        match std::fs::symlink_metadata(&resolved) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("record media path contains a symlink: {file}"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(resolved)
}

/// Preview one download record: every media entry with a non-empty recorded
/// hash must exist as a regular file with matching size and SHA-256.
pub fn preview_download_record<R: Fn(&Path) -> Result<(bool, u64, String), String>>(
    record: &DownloadRecord,
    archive_root: &Path,
    probe: R,
) -> RecordEvidenceVerdict {
    for media in &record.metadata.media {
        let path = match media_path(archive_root, &media.file) {
            Ok(path) => path,
            Err(reason) => return RecordEvidenceVerdict::Preserve { reason },
        };
        let (is_file, size, digest) = match probe(&path) {
            Ok(facts) => facts,
            Err(reason) => return RecordEvidenceVerdict::Preserve { reason },
        };
        if !is_file {
            return RecordEvidenceVerdict::Preserve {
                reason: format!("media file is not a regular file: {}", media.file),
            };
        }
        if size != media.size_bytes {
            return RecordEvidenceVerdict::Preserve {
                reason: format!(
                    "media size mismatch for {}: record {}, on-disk {}",
                    media.file, media.size_bytes, size
                ),
            };
        }
        if media.sha256.trim().is_empty() {
            return RecordEvidenceVerdict::Preserve {
                reason: format!("media record has no digest: {}", media.file),
            };
        }
        if !constant_time_eq(&digest, &media.sha256) {
            return RecordEvidenceVerdict::Preserve {
                reason: format!("media digest mismatch for {}", media.file),
            };
        }
    }
    RecordEvidenceVerdict::Accept
}

/// Resolve P1's existing archive directory below the selected root.
/// Media paths remain relative to this directory, not to the selected root.
pub fn archive_record_directory(
    record: &ArchiveRecord,
    archive_root: &Path,
) -> Result<std::path::PathBuf, String> {
    let directory = media_path(archive_root, &record.final_path)?;
    let metadata = std::fs::symlink_metadata(&directory).map_err(|error| error.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("archive final path must be an existing non-symlink directory".into());
    }
    Ok(directory)
}

/// Preview one archive record using the approved P1 directory-relative mapping.
pub fn preview_archive_record<R: Fn(&Path) -> Result<(bool, u64, String), String>>(
    record: &ArchiveRecord,
    archive_root: &Path,
    probe: R,
) -> RecordEvidenceVerdict {
    let directory = match archive_record_directory(record, archive_root) {
        Ok(directory) => directory,
        Err(reason) => {
            return RecordEvidenceVerdict::Preserve {
                reason: format!("archive final path rejected: {reason}"),
            };
        }
    };
    preview_download_record(&record.as_download_record(), &directory, probe)
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.as_bytes()
        .iter()
        .zip(right.as_bytes())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

/// Build the per-identity preview over a whole document.
pub fn preview_download_records<R: Fn(&Path) -> Result<(bool, u64, String), String>>(
    records: &[DownloadRecord],
    archive_root: &Path,
    probe: R,
) -> Vec<(RecordIdentity, RecordEvidenceVerdict)> {
    records
        .iter()
        .map(|record| {
            (
                record.identity(),
                preview_download_record(record, archive_root, &probe),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use xarchive_core::{ArchiveAuthor, ArchiveMedia, ArchiveMetadata};

    fn record_with_media(
        handle: &str,
        id: &str,
        media: Vec<(String, u64, String)>,
    ) -> DownloadRecord {
        DownloadRecord {
            account_handle: handle.into(),
            metadata: ArchiveMetadata {
                schema_version: 1,
                tweet_id: id.into(),
                url: format!("https://x.test/i/status/{id}"),
                tweet_type: "original".into(),
                author: ArchiveAuthor {
                    user_id: None,
                    username: Some(handle.into()),
                    display_name: None,
                },
                created_at: None,
                text: "t".into(),
                media: media
                    .into_iter()
                    .enumerate()
                    .map(|(index, (file, size_bytes, sha256))| ArchiveMedia {
                        index: (index + 1) as u32,
                        media_id: None,
                        media_type: "photo".into(),
                        file,
                        mime_type: None,
                        size_bytes,
                        sha256,
                    })
                    .collect(),
                archived_at: "2026-01-01T00:00:00Z".into(),
                reply_to: None,
                quoted_tweet: None,
            },
        }
    }

    #[test]
    fn archive_media_resolve_below_final_directory_and_require_it_to_exist() {
        let root = std::env::temp_dir().join(format!(
            "tw2tg-p1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("Tweets/222")).unwrap();
        let download = record_with_media(
            "alice",
            "222",
            vec![("media/a.jpg".into(), 7, "digest".into())],
        );
        let mut record = ArchiveRecord {
            account_handle: download.account_handle,
            metadata: download.metadata,
            final_path: "Tweets/222".into(),
        };
        assert_eq!(
            preview_archive_record(&record, &root, |path| {
                assert_eq!(path, root.join("Tweets/222/media/a.jpg"));
                Ok((true, 7, "digest".into()))
            }),
            RecordEvidenceVerdict::Accept
        );
        record.metadata.media.clear();
        for invalid in ["missing", "../escape", "C:\\archive", "Tweets\\222"] {
            record.final_path = invalid.into();
            assert!(matches!(
                preview_archive_record(&record, &root, |_| {
                    panic!("invalid directory must fail before probing media")
                }),
                RecordEvidenceVerdict::Preserve { .. }
            ));
        }
        record.final_path = ".".into();
        assert_eq!(
            preview_archive_record(&record, &root, |_| unreachable!()),
            RecordEvidenceVerdict::Accept
        );
        std::fs::write(root.join("file"), b"not a directory").unwrap();
        record.final_path = "file".into();
        assert!(matches!(
            preview_archive_record(&record, &root, |_| unreachable!()),
            RecordEvidenceVerdict::Preserve { .. }
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn archive_directory_symlink_is_rejected_without_media() {
        let root = std::env::temp_dir().join(format!("tw2tg-p1-link-{}", std::process::id()));
        std::fs::create_dir_all(root.join("real")).unwrap();
        std::os::unix::fs::symlink("real", root.join("link")).unwrap();
        let download = record_with_media("alice", "223", vec![]);
        let record = ArchiveRecord {
            account_handle: download.account_handle,
            metadata: download.metadata,
            final_path: "link".into(),
        };
        assert!(archive_record_directory(&record, &root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn matching_evidence_is_accepted_and_mismatches_are_preserved() {
        let record = record_with_media("alice", "1", vec![("media/a.jpg".into(), 10, "d1".into())]);
        let root = Path::new("/archive");
        let probe = |_: &Path| Ok((true, 10u64, "d1".to_owned()));

        assert_eq!(
            preview_download_record(&record, root, probe),
            RecordEvidenceVerdict::Accept
        );

        let size_mismatch = |_: &Path| Ok((true, 11u64, "d1".to_owned()));
        assert!(matches!(
            preview_download_record(&record, root, size_mismatch),
            RecordEvidenceVerdict::Preserve { .. }
        ));

        let digest_mismatch = |_: &Path| Ok((true, 10u64, "d2".to_owned()));
        assert!(matches!(
            preview_download_record(&record, root, digest_mismatch),
            RecordEvidenceVerdict::Preserve { .. }
        ));
    }

    #[test]
    fn escapes_are_preserved_and_source_absolute_paths_never_leak_in() {
        let evil = record_with_media("alice", "2", vec![("../../etc/x".into(), 1, "d".into())]);
        let root = Path::new("/archive");
        let probe = |_: &Path| Ok((true, 1u64, "d".to_owned()));
        match preview_download_record(&evil, root, probe) {
            RecordEvidenceVerdict::Preserve { reason } => assert!(reason.contains("escapes")),
            RecordEvidenceVerdict::Accept => panic!("escape must not be accepted"),
        }

        // Even a probe that would map the attacker's absolute path to a real
        // file cannot be reached: the lexical rejection happens first, so the
        // probe is never consulted for an absolute record path.
        let absolute = record_with_media(
            "alice",
            "3",
            vec![("/attacker/abs.jpg".into(), 1, "d".into())],
        );
        let unreachable = |_: &Path| -> Result<(bool, u64, String), String> {
            panic!("probe must not run for an escaping path");
        };
        assert!(matches!(
            preview_download_record(&absolute, root, unreachable),
            RecordEvidenceVerdict::Preserve { .. }
        ));
    }

    #[test]
    fn probe_errors_preserve_without_mutation() {
        let record = record_with_media("alice", "4", vec![("media/b.jpg".into(), 5, "d".into())]);
        let root = Path::new("/archive");
        let failing = |_: &Path| -> Result<(bool, u64, String), String> {
            Err("disk probe unavailable".into())
        };
        assert!(matches!(
            preview_download_record(&record, root, failing),
            RecordEvidenceVerdict::Preserve { .. }
        ));
    }
}
