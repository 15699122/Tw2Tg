//! Windows NTFS coordinator, preserving the shared scope/admission contract.
use super::{CoordinationGuard, Coordinator, LockError, LockScope, lock_file_path};
use crate::windows_filesystem::PinnedPath;
use std::path::{Path, PathBuf};

pub struct WindowsCoordinator {
    directory: PathBuf,
    _pin: PinnedPath,
}
pub struct WindowsGuard {
    files: Vec<PinnedPath>,
}

impl WindowsCoordinator {
    pub fn new(directory: impl Into<PathBuf>) -> Result<Self, LockError> {
        let directory = directory.into();
        let pin = PinnedPath::directory(&directory, true)?;
        Ok(Self {
            directory,
            _pin: pin,
        })
    }
    pub fn lock_directory(&self) -> &Path {
        &self.directory
    }
}

impl CoordinationGuard for WindowsGuard {
    fn release(self) -> Result<(), LockError> {
        for file in &self.files {
            file.file().unlock().map_err(|_| LockError::Io)?;
        }
        Ok(())
    }
}

impl Coordinator for WindowsCoordinator {
    type Guard = WindowsGuard;
    fn acquire(&self, scope: &LockScope) -> Result<Self::Guard, LockError> {
        self.acquire_all(std::slice::from_ref(scope))
    }
    fn acquire_all(&self, scopes: &[LockScope]) -> Result<Self::Guard, LockError> {
        let mut paths = scopes
            .iter()
            .map(|s| lock_file_path(&self.directory, s))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        paths.dedup();
        let mut files = Vec::new();
        for path in paths {
            let file = PinnedPath::lock_file(&path)?;
            file.file().try_lock().map_err(|error| match error {
                std::fs::TryLockError::WouldBlock => LockError::Contended,
                std::fs::TryLockError::Error(_) => LockError::Io,
            })?;
            files.push(file);
        }
        Ok(WindowsGuard { files })
    }
}
