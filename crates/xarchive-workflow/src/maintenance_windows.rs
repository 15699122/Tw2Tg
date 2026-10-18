//! NTFS maintenance turnstile using OS-released, independently opened locks.
use crate::coordination::LockError;
use crate::windows_filesystem::PinnedPath;
use std::path::Path;

#[derive(Debug)]
pub struct WorkspaceGuard {
    _active: PinnedPath,
    _turnstile: Option<PinnedPath>,
}

#[derive(Debug)]
pub struct MaintenanceGuard {
    _guard: WorkspaceGuard,
}

fn lock(file: &PinnedPath, shared: bool) -> Result<(), LockError> {
    let result = if shared {
        file.file().try_lock_shared()
    } else {
        file.file().try_lock()
    };
    result.map_err(|error| match error {
        std::fs::TryLockError::WouldBlock => LockError::Contended,
        std::fs::TryLockError::Error(_) => LockError::Io,
    })
}

impl WorkspaceGuard {
    pub fn try_enter(directory: &Path) -> Result<Self, LockError> {
        let turnstile = PinnedPath::lock_file(&directory.join("workspace-admission.lock"))?;
        lock(&turnstile, false)?;
        let active = PinnedPath::lock_file(&directory.join("workspace-active.lock"))?;
        lock(&active, true)?;
        drop(turnstile);
        Ok(Self {
            _active: active,
            _turnstile: None,
        })
    }

    pub fn drain(
        directory: &Path,
        cancelled: impl Fn() -> bool,
    ) -> Result<MaintenanceGuard, LockError> {
        let turnstile = PinnedPath::lock_file(&directory.join("workspace-admission.lock"))?;
        let wait = |file: &PinnedPath| -> Result<(), LockError> {
            loop {
                if cancelled() {
                    return Err(LockError::Contended);
                }
                match lock(file, false) {
                    Err(LockError::Contended) => {
                        std::thread::sleep(std::time::Duration::from_millis(10))
                    }
                    result => return result,
                }
            }
        };
        wait(&turnstile)?;
        let active = PinnedPath::lock_file(&directory.join("workspace-active.lock"))?;
        wait(&active)?;
        Ok(MaintenanceGuard {
            _guard: WorkspaceGuard {
                _active: active,
                _turnstile: Some(turnstile),
            },
        })
    }
}
