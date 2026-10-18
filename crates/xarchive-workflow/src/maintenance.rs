//! Workspace maintenance turnstile. Lock files must never be removed/replaced.
//! Independently opened descriptors serialize threads as well as processes.
use crate::coordination::{LockError, unix::LinuxCoordinator};
use std::fs::File;
use std::path::Path;

/// Active reader or drained exclusive maintenance ownership; Drop releases it.
#[derive(Debug)]
pub struct WorkspaceGuard {
    _active: File,
    _turnstile: Option<File>,
}

/// Exclusive ownership proof, never constructible from an active reader.
#[derive(Debug)]
pub struct MaintenanceGuard {
    _guard: WorkspaceGuard,
}

fn lock(file: &File, shared: bool) -> Result<(), LockError> {
    use rustix::fs::{FlockOperation, flock};
    flock(
        file,
        if shared {
            FlockOperation::NonBlockingLockShared
        } else {
            FlockOperation::NonBlockingLockExclusive
        },
    )
    .map_err(|error| {
        if error == rustix::io::Errno::AGAIN {
            LockError::Contended
        } else {
            LockError::Io
        }
    })
}

impl WorkspaceGuard {
    /// Fail fast without durable mutation when maintenance has closed admission.
    pub fn try_enter(directory: &Path) -> Result<Self, LockError> {
        let turnstile =
            LinuxCoordinator::open_lock_file(&directory.join("workspace-admission.lock"))?;
        lock(&turnstile, false)?;
        let active = LinuxCoordinator::open_lock_file(&directory.join("workspace-active.lock"))?;
        lock(&active, true)?;
        drop(turnstile);
        Ok(Self {
            _active: active,
            _turnstile: None,
        })
    }

    /// Close admission before draining. Cancellation drops all held descriptors.
    /// Contended also represents cancellation, with no persistent closed state.
    pub fn drain(
        directory: &Path,
        cancelled: impl Fn() -> bool,
    ) -> Result<MaintenanceGuard, LockError> {
        let turnstile =
            LinuxCoordinator::open_lock_file(&directory.join("workspace-admission.lock"))?;
        let wait = |file: &File| -> Result<(), LockError> {
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
        let active = LinuxCoordinator::open_lock_file(&directory.join("workspace-active.lock"))?;
        wait(&active)?;
        Ok(MaintenanceGuard {
            _guard: Self {
                _active: active,
                _turnstile: Some(turnstile),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_owner() {
        let Some(directory) = std::env::var_os("TW2TG_MAINTENANCE_TEST_DIRECTORY") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        let _guard = WorkspaceGuard::drain(&directory, || false).unwrap();
        std::fs::write(directory.join("ready"), b"ready").unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn killed_process_releases_maintenance() {
        let directory =
            std::env::temp_dir().join(format!("tw2tg-maintenance-process-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "maintenance::tests::process_owner",
                "--nocapture",
            ])
            .env("TW2TG_MAINTENANCE_TEST_DIRECTORY", &directory)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !directory.join("ready").exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let ready = directory.join("ready").exists();
        let contended = matches!(
            WorkspaceGuard::try_enter(&directory),
            Err(LockError::Contended)
        );
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(ready && contended, "child must own maintenance before kill");
        assert!(WorkspaceGuard::try_enter(&directory).is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn shared_readers_drain_and_cancel_release_admission() {
        let directory =
            std::env::temp_dir().join(format!("tw2tg-maintenance-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let first = WorkspaceGuard::try_enter(&directory).unwrap();
        let second = WorkspaceGuard::try_enter(&directory).unwrap();
        let polls = std::cell::Cell::new(0);
        assert!(
            WorkspaceGuard::drain(&directory, || {
                polls.set(polls.get() + 1);
                if polls.get() == 2 {
                    assert!(matches!(
                        WorkspaceGuard::try_enter(&directory),
                        Err(LockError::Contended)
                    ));
                }
                polls.get() > 2
            })
            .is_err()
        );
        drop((first, second));
        let maintenance = WorkspaceGuard::drain(&directory, || false).unwrap();
        assert!(matches!(
            WorkspaceGuard::try_enter(&directory),
            Err(LockError::Contended)
        ));
        drop(maintenance);
        assert!(WorkspaceGuard::try_enter(&directory).is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
