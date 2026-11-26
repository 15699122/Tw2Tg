//! Result mapping and the direct-download archive path.
//!
//! Split out of `production.rs` without changing behaviour. The rules:
//!
//! * the direct-download path (`use_aria2 = false`) starts no aria2 process: it
//!   validates the staging-relative files the worker reported and converts them
//!   into the durable commit contract, nothing more;
//! * `media_type_name` is the single mapping from the protocol's media type to the
//!   persisted string, shared by the extraction and download paths so the two can
//!   never disagree;
//! * the mapping is total: an unrecognised type becomes `unknown` rather than
//!   failing the archive, because the media facts, not this label, decide identity.

use xarchive_protocol::ExtractionMediaType;
#[cfg(test)]
use xarchive_sidecar_supervisor::SidecarSupervisor;

#[cfg(test)]
use crate::archive::{SidecarArchiveResult, SidecarDownloadRequest};
#[cfg(test)]
use crate::executor::CancellationToken;

#[cfg(test)]
use super::sidecar_operations::{download_v2, verify_downloaded_files};
#[cfg(test)]
use super::transfer::extraction_to_metadata;

/// Test-only direct-download archive fixture path (`use_aria2 = false`).
/// Production builds converge to `aria2-only` and do not compile this path.
#[cfg(test)]
pub(super) fn execute_gallery_dl_archive(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    _network: &crate::executor::ExecutorNetworkConfig,
) -> Result<SidecarArchiveResult, String> {
    let (download, extraction) = download_v2(supervisor, request, cancellation)?;
    let files = verify_downloaded_files(&download, &extraction, &request.staging_dir)?;

    let metadata = extraction_to_metadata(&extraction);
    Ok(SidecarArchiveResult {
        metadata,
        files,
        #[cfg(any(target_os = "linux", windows))]
        retained_aria2: None,
    })
}

pub(super) fn media_type_name(media_type: &ExtractionMediaType) -> String {
    match media_type {
        ExtractionMediaType::Photo => "photo",
        ExtractionMediaType::Video => "video",
        ExtractionMediaType::Unknown => "unknown",
    }
    .to_owned()
}
