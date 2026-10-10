//! Bridge between shared workflow ports and Storage errors.
//!
//! `xarchive-workflow` owns coordination dependencies; this module maps its
//! platform move errors onto [`StorageError`] and re-exports the mover
//! so `FileStore` can use the Unix adapter without Storage depending on
//! platform details directly.

use crate::StorageError;

pub use xarchive_workflow::coordination::{
    MoveError, NoReplaceMover, RenameEdge, replay_rename_stage,
};

pub use xarchive_workflow::{AttemptIdentity, CommitPhase, WorkflowError, attempt_may_mutate};

pub struct LinuxNoReplaceMover;

impl NoReplaceMover for LinuxNoReplaceMover {
    fn move_no_replace(
        &self,
        source: &std::path::Path,
        destination: &std::path::Path,
    ) -> Result<(), MoveError> {
        #[cfg(target_os = "linux")]
        {
            xarchive_workflow::coordination::unix::LinuxNoReplaceMover
                .move_no_replace(source, destination)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (source, destination);
            Err(MoveError::Unsupported)
        }
    }
}

/// Map a workflow move failure onto the Storage error taxonomy without
/// losing the fail-closed distinction between conflicts, missing sources
/// and unsupported platforms.
pub fn bridge_move_error(error: MoveError) -> StorageError {
    match error {
        MoveError::DestinationExists => StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "rename destination already exists",
        )),
        MoveError::InvalidSource => StorageError::InvalidPath,
        MoveError::Unsupported => StorageError::InvalidState(
            "atomic no-replace rename requires the platform adapter".into(),
        ),
        MoveError::Io => StorageError::Io(std::io::Error::other("atomic move failed")),
    }
}
