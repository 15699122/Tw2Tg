//! Unix advisory-lock adapter.
//!
//! `fcntl` record locks are used rather than `flock` because the guarantee that
//! matters is per-process: a record belongs to a process, so the kernel drops
//! every record the process holds when it exits, for any reason, including
//! `SIGKILL`. A successor therefore acquires the lock as soon as the dead owner
//! is gone, with no timeout and no liveness probe.
//!
//! The lock file is never removed. Removing it would break exclusivity, because
//! a second process could create a fresh file at the same path and lock that
//! instead, leaving two owners of one logical resource. A leftover file is
//! harmless: it is an inert rendezvous point whose only purpose is to be locked.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use super::{CoordinationGuard, Coordinator, LockError, LockScope, lock_file_path};

/// Drop one record lock on a descriptor, without blocking.
fn unlock(file: &File) -> std::io::Result<()> {
    rustix::fs::fcntl_lock(file, rustix::fs::FlockOperation::NonBlockingUnlock)
        .map_err(std::io::Error::from)
}

/// One held `fcntl` record lock. The lock lives as long as the descriptor.
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
        // Closing the descriptor releases the record. Unlocking first keeps the
        // release explicit so a failure can be reported rather than deferred to
        // the implicit close, whose result cannot be observed.
        let file = self.file.take().expect("guard released once");
        unlock(&file)?;
        drop(file);
        Ok(())
    }
}

impl Drop for LinuxLockGuard {
    fn drop(&mut self) {
        // The kernel releases the record when the last descriptor closes, so a
        // panic path still loses exclusivity. An unlock failure here cannot be
        // reported, which is why `release` exists for the normal path.
        drop(self.file.take());
    }
}

/// Holds one record per scope, released together.
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
    fn release(mut self) -> Result<(), LockError> {
        for file in self.files.drain(..) {
            unlock(&file)?;
        }
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
        match rustix::fs::fcntl_lock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
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
            match rustix::fs::fcntl_lock(
                &file,
                rustix::fs::FlockOperation::NonBlockingLockExclusive,
            ) {
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
