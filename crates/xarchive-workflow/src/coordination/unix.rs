//! Unix advisory-lock adapter.
//!
//! `flock` locks are associated with independently opened file descriptions, so
//! separate threads and coordinator instances contend while the kernel releases
//! ownership when the process exits, including
//! after `SIGKILL`. A successor needs no lease timeout or liveness probe.
//!
//! The lock file is never removed. Removing it would break exclusivity, because
//! a second process could create a fresh file at the same path and lock that
//! instead, leaving two owners of one logical resource. A leftover file is
//! harmless: it is an inert rendezvous point whose only purpose is to be locked.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use super::{CoordinationGuard, Coordinator, LockError, LockScope, lock_file_path};

/// Drop one advisory lock on a descriptor.
fn unlock(file: &File) -> std::io::Result<()> {
    rustix::fs::flock(file, rustix::fs::FlockOperation::Unlock).map_err(std::io::Error::from)
}

/// One held advisory lock. The lock lives as long as the open file description.
pub struct LinuxLockGuard {
    path: PathBuf,
    file: Option<File>,
}

impl std::fmt::Debug for LinuxLockGuard {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LinuxLockGuard")
            .field("path", &self.path)
            .field("held", &self.file.is_some())
            .finish()
    }
}

impl LinuxLockGuard {
    pub(super) fn new(path: PathBuf, file: File) -> Self {
        Self {
            path,
            file: Some(file),
        }
    }

    /// Path of the inert lock file backing this record.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl CoordinationGuard for LinuxLockGuard {
    fn release(mut self) -> Result<(), LockError> {
        // Closing the descriptor releases the lock. Unlocking first keeps the
        // release explicit so a failure can be reported rather than deferred to
        // the implicit close, whose result cannot be observed.
        let file = self.file.take().expect("guard released once");
        if let Err(error) = unlock(&file) {
            drop(file);
            return Err(LockError::from(error));
        }
        drop(file);
        Ok(())
    }
}

impl Drop for LinuxLockGuard {
    fn drop(&mut self) {
        // The kernel releases the lock when the open file description closes, so
        // a panic path still loses exclusivity. An unlock failure here cannot be
        // reported, which is why `release` exists for the normal path.
        drop(self.file.take());
    }
}

/// Holds one lock per scope, released together.
///
/// Public only because it appears in the [`LinuxGuard`] variant; construct it
/// through [`Coordinator::acquire_all`], not directly.
pub struct LinuxLockGroup {
    paths: Vec<PathBuf>,
    files: Vec<File>,
}

impl std::fmt::Debug for LinuxLockGroup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LinuxLockGroup")
            .field("paths", &self.paths)
            .field("held", &self.files.len())
            .finish()
    }
}

impl LinuxLockGroup {
    pub(super) fn new(paths: Vec<PathBuf>, files: Vec<File>) -> Self {
        Self { paths, files }
    }
}

impl CoordinationGuard for LinuxLockGroup {
    fn release(self) -> Result<(), LockError> {
        for file in &self.files {
            unlock(file)?;
        }
        drop(self);
        Ok(())
    }
}

impl Drop for LinuxLockGroup {
    fn drop(&mut self) {
        drop(self.files.drain(..));
        drop(self.paths.drain(..));
    }
}

/// One or more held `fcntl` records, released together.
///
/// A single enum keeps the `Coordinator` guard type unified: callers do not
/// pick a different type based on how many scopes they needed, and the batch
/// path cannot leak a partial set through a narrower guard.
#[derive(Debug)]
pub enum LinuxGuard {
    Single(LinuxLockGuard),
    Group(LinuxLockGroup),
}

impl LinuxGuard {
    fn single(path: PathBuf, file: File) -> Self {
        Self::Single(LinuxLockGuard::new(path, file))
    }

    fn group(paths: Vec<PathBuf>, files: Vec<File>) -> Self {
        Self::Group(LinuxLockGroup::new(paths, files))
    }
}

impl CoordinationGuard for LinuxGuard {
    fn release(self) -> Result<(), LockError> {
        match self {
            Self::Single(guard) => guard.release(),
            Self::Group(guard) => guard.release(),
        }
    }
}

/// Coordinates one workspace through advisory record locks in a lock directory.
pub struct LinuxCoordinator {
    lock_directory: PathBuf,
}

impl LinuxCoordinator {
    /// Create the lock directory if it does not exist yet.
    ///
    /// The directory must be stable for the life of the workspace: moving it
    /// would orphan records held against the old path and let a second process
    /// coordinate against a different directory while both touched one dataset.
    pub fn new(lock_directory: impl Into<PathBuf>) -> Result<Self, LockError> {
        let lock_directory = lock_directory.into();
        std::fs::create_dir_all(&lock_directory)?;
        Ok(Self { lock_directory })
    }

    /// Directory holding the lock files.
    pub fn lock_directory(&self) -> &Path {
        &self.lock_directory
    }

    /// Open one lock file without locking it.
    fn open_lock_file(path: &Path) -> Result<File, LockError> {
        // Read/write plus create: the file must be creatable by the first
        // process and openable by later ones. It is never truncated, because
        // truncating a file another process holds open would not release their
        // lock but would still mutate shared state.
        Ok(OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?)
    }

    /// Map a failed lock attempt onto the adapter's error taxonomy.
    ///
    /// Only the kernel's "conflicting record" reports become contention. A
    /// genuine I/O failure must not be mistaken for a busy peer, or the caller
    /// would retry a broken filesystem forever.
    fn classify(error: rustix::io::Errno) -> LockError {
        let error = std::io::Error::from(error);
        if matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::PermissionDenied
        ) {
            LockError::Contended
        } else {
            LockError::from(error)
        }
    }
}

impl Coordinator for LinuxCoordinator {
    type Guard = LinuxGuard;

    fn acquire(&self, scope: &LockScope) -> Result<Self::Guard, LockError> {
        let path = lock_file_path(&self.lock_directory, scope)?;
        let file = Self::open_lock_file(&path)?;
        match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(LinuxGuard::single(path, file)),
            Err(error) => {
                drop(file);
                Err(Self::classify(error))
            }
        }
    }

    fn acquire_all(&self, scopes: &[LockScope]) -> Result<Self::Guard, LockError> {
        // Sort by derived lock path so every process requests the same set in
        // the same order. Without a shared order, two processes holding one
        // scope each and waiting for the other's is a stable deadlock.
        let mut ordered: Vec<LockScope> = scopes.to_vec();
        ordered.sort_by_cached_key(|scope| {
            lock_file_path(&self.lock_directory, scope)
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        ordered.dedup();

        let mut paths = Vec::with_capacity(ordered.len());
        let mut files = Vec::with_capacity(ordered.len());
        for scope in &ordered {
            let path = lock_file_path(&self.lock_directory, scope)?;
            let file = Self::open_lock_file(&path)?;
            match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => {
                    paths.push(path);
                    files.push(file);
                }
                Err(error) => {
                    // Release everything already taken before reporting. Keeping
                    // a partial set would block other processes for scopes this
                    // one never actually needed.
                    drop(files);
                    drop(paths);
                    return Err(Self::classify(error));
                }
            }
        }

        Ok(LinuxGuard::group(paths, files))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::CoordinationGuard;

    fn scratch() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "xarchive-lock-group-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).expect("scratch directory");
        path
    }

    #[test]
    fn group_holds_all_members_until_group_drop() {
        let lock_dir = scratch();
        let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");
        let first = LockScope::Job("first".to_owned());
        let second = LockScope::Job("second".to_owned());
        let group = coordinator
            .acquire_all(&[first.clone(), second.clone()])
            .expect("lock group");

        assert!(matches!(
            coordinator.acquire(&first),
            Err(LockError::Contended)
        ));
        assert!(matches!(
            coordinator.acquire(&second),
            Err(LockError::Contended)
        ));
        drop(group);

        let first_guard = coordinator.acquire(&first).expect("first released");
        let second_guard = coordinator.acquire(&second).expect("second released");
        drop((first_guard, second_guard));
        let _ = std::fs::remove_dir_all(lock_dir);
    }

    #[test]
    fn group_explicit_release_unlocks_every_member() {
        let lock_dir = scratch();
        let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");
        let first = LockScope::Job("first".to_owned());
        let second = LockScope::Job("second".to_owned());
        coordinator
            .acquire_all(&[first.clone(), second.clone()])
            .expect("lock group")
            .release()
            .expect("release group");

        let first_guard = coordinator.acquire(&first).expect("first released");
        let second_guard = coordinator.acquire(&second).expect("second released");
        drop((first_guard, second_guard));
        let _ = std::fs::remove_dir_all(lock_dir);
    }
}

/// Linux atomic no-replace mover backed by `renameat2(RENAME_NOREPLACE)`.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default)]
pub struct LinuxNoReplaceMover;

impl super::NoReplaceMover for LinuxNoReplaceMover {
    fn move_no_replace(&self, source: &Path, destination: &Path) -> Result<(), super::MoveError> {
        let (source_parent, source_name) = split_parent_and_name(source)?;
        let (destination_parent, destination_name) = split_parent_and_name(destination)?;
        let source_directory = File::open(source_parent).map_err(|_| super::MoveError::Io)?;
        let destination_directory =
            File::open(destination_parent).map_err(|_| super::MoveError::Io)?;
        rustix::fs::renameat_with(
            &source_directory,
            source_name,
            &destination_directory,
            destination_name,
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(|error| match error {
            rustix::io::Errno::EXIST => super::MoveError::DestinationExists,
            rustix::io::Errno::NOENT | rustix::io::Errno::NOTDIR => super::MoveError::InvalidSource,
            rustix::io::Errno::NOSYS | rustix::io::Errno::INVAL | rustix::io::Errno::OPNOTSUPP => {
                super::MoveError::Unsupported
            }
            _ => super::MoveError::Io,
        })
    }
}

#[cfg(target_os = "linux")]
fn split_parent_and_name(path: &Path) -> Result<(&Path, &std::ffi::OsStr), super::MoveError> {
    let name = path.file_name().ok_or(super::MoveError::Unsupported)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    Ok((parent, name))
}
