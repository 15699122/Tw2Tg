//! Record import evidence: the filesystem probe and the post-drain recheck.
//!
//! Split out of `commands/records.rs` without changing any command name,
//! argument shape, serialized field or error string. The rules:
//!
//! * the probe reports size and SHA-256 of a regular file, and reports "not a
//!   regular file" or "not found" instead of failing, so a preview can carry a
//!   verdict per record;
//! * the recheck runs over the whole accepted selection after the maintenance
//!   drain: a valid prefix is never imported while another selected record has
//!   invalid or changed evidence.

use std::fs::File;
use std::io::copy;
use std::path::Path;

use sha2::{Digest, Sha256};
use xarchive_core::DownloadRecord;
use xarchive_storage::record_evidence::RecordEvidenceVerdict;
use xarchive_storage::record_import::AcceptedRecord;

/// Filesystem probe for `preview_record_import`: resolves media regular-file
/// facts (size and SHA-256) under the caller-selected archive root.
pub(super) fn record_import_media_probe(path: &Path) -> Result<(bool, u64, String), String> {
    match File::open(path) {
        Ok(mut file) => {
            let metadata = file.metadata().map_err(|error| error.to_string())?;
            if !metadata.is_file() {
                return Ok((false, 0, "media path is not a regular file".into()));
            }
            let mut hasher = Sha256::new();
            copy(&mut file, &mut hasher).map_err(|error| error.to_string())?;
            let digest = format!("{:x}", hasher.finalize());
            Ok((true, metadata.len(), digest))
        }
        Err(_) => Ok((false, 0, "media file not found".into())),
    }
}

/// Recheck the entire selection after maintenance drain; never import a valid
/// prefix when another selected record has invalid or changed evidence.
pub(super) fn verify_accepted_media_evidence(
    records: &[DownloadRecord],
    accepted: &[AcceptedRecord],
    archive_root: &str,
) -> Result<Vec<AcceptedRecord>, String> {
    if accepted.is_empty() {
        return Err("accepted set is empty".into());
    }
    let mut seen = std::collections::HashSet::new();
    for item in accepted {
        if !seen.insert(item.index) {
            return Err(format!("duplicate accepted-set index {}", item.index));
        }
        let record = records
            .get(item.index)
            .ok_or_else(|| format!("accepted-set index {} is out of range", item.index))?;
        if let RecordEvidenceVerdict::Preserve { reason } =
            xarchive_storage::record_evidence::preview_download_record(
                record,
                Path::new(archive_root),
                record_import_media_probe,
            )
        {
            return Err(format!("accepted-set index {}: {reason}", item.index));
        }
    }
    Ok(accepted.to_vec())
}
