//! Durable media facts shared with the local archive commit path.

use serde::{Deserialize, Serialize};

/// One verified media file that the Desktop commit path writes into the final
/// archive directory.
///
/// The value is produced by the Desktop after it validates the aria2 staging
/// output (relative path, size, reparse status); it is not a Sidecar event
/// payload and therefore carries no session-only data such as signed URLs,
/// request headers, transfer IDs, or refresh state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadFile {
    pub relative_path: String,
    pub size_bytes: u64,
    pub media_type: String,
    #[serde(default)]
    pub mime_type: Option<String>,
}
