//! Linux/Unix advisory-lock adapter.
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

use rustix::fs::OpenOptionsExt;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

use super::{CoordinationGuard, Coordinator, LockError, LockScope, lock_file_path};

type LocalLockRegistry = Arc<Mutex<HashMap<PathBuf, usize>>>;

fn local_lock_registry() -> LocalLockRegistry {
    static REGISTRY: OnceLock<LocalLockRegistry> = OnceLock::new();
    REGISTRY
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

/// Drop one advisory lock on a descriptor.
fn unlock(file: &File) -> std::io::Result<()> {
    rustix::fs::flock(file, rustix::fs::FlockOperation::Unlock).map_err(std::io::Error::from)
}

/// One held advisory lock. The lock lives as long as the open file description.
pub struct LinuxLockGuard {
    path: PathBuf,
    file: Option<File>,
    local_locks: Arc<Mutex<std::collections::HashMap<PathBuf, usize>>>,
    local_key: Option<PathBuf>,
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
    fn release_key(
        local_locks: &Arc<Mutex<std::collections::HashMap<PathBuf, usize>>>,
        key: &Path,
    ) {
        if let Ok(mut locks) = local_locks.lock() {
            locks.remove(key);
        }
    }

    pub(super) fn new(
        path: PathBuf,
        file: File,
        local_locks: Arc<Mutex<std::collections::HashMap<PathBuf, usize>>>,
        local_key: PathBuf,
    ) -> Self {
        Self {
            path,
            file: Some(file),
            local_locks,
            local_key: Some(local_key),
        }
    }

    fn release_local(&mut self) {
        if let Some(key) = self.local_key.take() {
            Self::release_key(&self.local_locks, &key);
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
            self.release_local();
            return Err(LockError::from(error));
        }
        drop(file);
        self.release_local();
        Ok(())
    }
}

impl Drop for LinuxLockGuard {
    fn drop(&mut self) {
        // The kernel releases the lock when the open file description closes, so
        // a panic path still loses exclusivity. An unlock failure here cannot be
        // reported, which is why `release` exists for the normal path.
        drop(self.file.take());
        self.release_local();
    }
}

/// Holds one lock per scope, released together.
///
/// Public only because it appears in the [`LinuxGuard`] variant; construct it
/// through [`Coordinator::acquire_all`], not directly.
pub struct LinuxLockGroup {
    paths: Vec<PathBuf>,
    files: Vec<File>,
    local_locks: Arc<Mutex<std::collections::HashMap<PathBuf, usize>>>,
    local_keys: Vec<PathBuf>,
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
    pub(super) fn new(
        paths: Vec<PathBuf>,
        files: Vec<File>,
        local_locks: Arc<Mutex<std::collections::HashMap<PathBuf, usize>>>,
        local_keys: Vec<PathBuf>,
    ) -> Self {
        Self {
            paths,
            files,
            local_locks,
            local_keys,
        }
    }

    fn release_local(&mut self) {
        for key in self.local_keys.drain(..) {
            LinuxLockGuard::release_key(&self.local_locks, &key);
        }
    }
}

impl CoordinationGuard for LinuxLockGroup {
    fn release(self) -> Result<(), LockError> {
        let mut first_error = None;
        for file in &self.files {
            if let Err(error) = unlock(file)
                && first_error.is_none()
            {
                first_error = Some(LockError::from(error));
            }
        }
        drop(self);
        first_error.map_or(Ok(()), Err)
    }
}

impl Drop for LinuxLockGroup {
    fn drop(&mut self) {
        drop(self.files.drain(..));
        drop(self.paths.drain(..));
        self.release_local();
    }
}

/// One or more held `flock` records, released together.
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
    fn single(
        path: PathBuf,
        file: File,
        local_locks: Arc<Mutex<std::collections::HashMap<PathBuf, usize>>>,
        local_key: PathBuf,
    ) -> Self {
        Self::Single(LinuxLockGuard::new(path, file, local_locks, local_key))
    }

    fn group(
        paths: Vec<PathBuf>,
        files: Vec<File>,
        local_locks: Arc<Mutex<std::collections::HashMap<PathBuf, usize>>>,
        local_keys: Vec<PathBuf>,
    ) -> Self {
        Self::Group(LinuxLockGroup::new(paths, files, local_locks, local_keys))
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
    local_locks: LocalLockRegistry,
}

impl LinuxCoordinator {
    fn reserve_local(&self, path: &Path) -> Result<PathBuf, LockError> {
        let mut locks = self.local_locks.lock().map_err(|_| LockError::Io)?;
        let lock_directory = std::fs::canonicalize(&self.lock_directory)
            .unwrap_or_else(|_| self.lock_directory.clone());
        let key = lock_directory.join(path.file_name().ok_or(LockError::InvalidName)?);
        if locks.contains_key(&key) {
            return Err(LockError::Contended);
        }
        locks.insert(key.clone(), 1);
        Ok(key)
    }

    fn release_local(&self, key: &Path) {
        if let Ok(mut locks) = self.local_locks.lock() {
            locks.remove(key);
        }
    }

    /// Create the lock directory if it does not exist yet.
    ///
    /// The directory must be stable for the life of the workspace: moving it
    /// would orphan records held against the old path and let a second process
    /// coordinate against a different directory while both touched one dataset.
    pub fn new(lock_directory: impl Into<PathBuf>) -> Result<Self, LockError> {
        let lock_directory = lock_directory.into();
        std::fs::create_dir_all(&lock_directory)?;
        Ok(Self {
            lock_directory,
            local_locks: local_lock_registry(),
        })
    }

    /// Directory holding the lock files.
    pub fn lock_directory(&self) -> &Path {
        &self.lock_directory
    }

    fn reject_symlink_lock_file(path: &Path) -> Result<(), LockError> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(LockError::Io),
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(LockError::Io),
        }
    }

    /// Open one lock file without following a pre-existing symlink.
    fn open_lock_file(path: &Path) -> Result<File, LockError> {
        // Read/write plus create: the file must be creatable by the first
        // process and openable by later ones. It is never truncated, because
        // truncating a file another process holds open would not release their
        // lock but would still mutate shared state.
        Self::reject_symlink_lock_file(path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(LockError::Io);
        }
        Ok(file)
    }

    /// Bound for absorbing unowned fork-to-exec lock residue.
    ///
    /// A child spawned while this process holds a lock descriptor inherits
    /// that descriptor: close-on-exec acts at `exec`, not at `fork`. If every
    /// local guard for the lock is released inside that fork-to-exec window,
    /// the child's inherited copy keeps the kernel lock alive for the
    /// remaining milliseconds even though no logical owner remains — the next
    /// `flock` on a fresh descriptor reports `EAGAIN` against nobody. The
    /// residue always ends when the child execs or exits, so [`Self::flock`]
    /// retries for at most this duration before reporting contention. A live
    /// owner holds the lock for as long as it runs, so genuine cross-process
    /// contention still surfaces after the window, and same-process
    /// contention is answered earlier and immediately by the local registry.
    const FORK_RESIDUE_ABSORPTION: std::time::Duration = std::time::Duration::from_millis(50);

    /// Take the kernel lock, absorbing unowned fork-to-exec residue.
    ///
    /// Only `EAGAIN` is retried: it is the one errno that can mean either a
    /// live owner or residue, and the bounded window separates them by
    /// duration. Any other failure (permission, I/O) is returned immediately —
    /// retrying a broken or inaccessible filesystem as if it were a busy peer
    /// would loop forever without ever succeeding.
    fn flock(file: &File) -> Result<(), rustix::io::Errno> {
        let deadline = std::time::Instant::now() + Self::FORK_RESIDUE_ABSORPTION;
        loop {
            match rustix::fs::flock(file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => return Ok(()),
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) if error == rustix::io::Errno::AGAIN => {
                    if std::time::Instant::now() >= deadline {
                        return Err(error);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Map a failed lock attempt onto the adapter's error taxonomy.
    ///
    /// Only the kernel's "conflicting record" report becomes contention: it is
    /// the single case that means another live owner holds the resource.
    /// Permission denial and any other I/O failure must map to
    /// [`LockError::Io`], because retrying a broken or inaccessible filesystem
    /// as if it were a busy peer would loop forever without ever succeeding.
    fn classify(error: rustix::io::Errno) -> LockError {
        let error = std::io::Error::from(error);
        if matches!(error.kind(), std::io::ErrorKind::WouldBlock) {
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
        let local_key = self.reserve_local(&path)?;
        let file = match Self::open_lock_file(&path) {
            Ok(file) => file,
            Err(error) => {
                self.release_local(&local_key);
                return Err(error);
            }
        };
        match Self::flock(&file) {
            Ok(()) => Ok(LinuxGuard::single(
                path,
                file,
                self.local_locks.clone(),
                local_key,
            )),
            Err(error) => {
                drop(file);
                self.release_local(&local_key);
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
        let mut local_keys: Vec<PathBuf> = Vec::with_capacity(ordered.len());
        for scope in &ordered {
            let path = lock_file_path(&self.lock_directory, scope)?;
            let local_key = match self.reserve_local(&path) {
                Ok(key) => key,
                Err(error) => {
                    drop(files);
                    for key in &local_keys {
                        self.release_local(key);
                    }
                    return Err(error);
                }
            };
            let file = match Self::open_lock_file(&path) {
                Ok(file) => file,
                Err(error) => {
                    drop(files);
                    for key in &local_keys {
                        self.release_local(key);
                    }
                    self.release_local(&local_key);
                    return Err(error);
                }
            };
            match Self::flock(&file) {
                Ok(()) => {
                    paths.push(path);
                    files.push(file);
                    local_keys.push(local_key);
                }
                Err(error) => {
                    // Release everything already taken before reporting. Keeping
                    // a partial set would block other processes for scopes this
                    // one never actually needed.
                    drop(files);
                    drop(paths);
                    self.release_local(&local_key);
                    for key in &local_keys {
                        self.release_local(key);
                    }
                    return Err(Self::classify(error));
                }
            }
        }

        Ok(LinuxGuard::group(
            paths,
            files,
            self.local_locks.clone(),
            local_keys,
        ))
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

    #[test]
    fn same_process_threads_and_coordinator_instances_contend() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let lock_dir = scratch();
        let first = LinuxCoordinator::new(&lock_dir).expect("first coordinator");
        let second = LinuxCoordinator::new(&lock_dir).expect("second coordinator");
        let scope = LockScope::Job("same-process".to_owned());
        let held = first.acquire(&scope).expect("first lock");
        assert!(matches!(second.acquire(&scope), Err(LockError::Contended)));

        let barrier = Arc::new(Barrier::new(2));
        let thread_dir = lock_dir.clone();
        let thread_scope = scope.clone();
        let thread_barrier = barrier.clone();
        let thread = thread::spawn(move || {
            let coordinator = LinuxCoordinator::new(thread_dir).expect("thread coordinator");
            thread_barrier.wait();
            coordinator.acquire(&thread_scope).err()
        });
        barrier.wait();
        assert_eq!(
            thread.join().expect("thread result"),
            Some(LockError::Contended)
        );
        drop(held);
        let released = second
            .acquire(&scope)
            .expect("lock released in same process");
        drop(released);
        let _ = std::fs::remove_dir_all(lock_dir);
    }

    #[test]
    fn partial_group_contention_releases_prior_local_reservations() {
        let lock_dir = scratch();
        let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");
        let held = coordinator
            .acquire(&LockScope::Job("second".to_owned()))
            .expect("held lock");
        let first = LockScope::Job("first".to_owned());
        let second = LockScope::Job("second".to_owned());
        assert!(matches!(
            coordinator.acquire_all(&[first.clone(), second]),
            Err(LockError::Contended)
        ));
        let acquired = coordinator.acquire(&first).expect("partial lock released");
        drop((acquired, held));
        let _ = std::fs::remove_dir_all(lock_dir);
    }

    #[test]
    fn group_guard_blocks_single_lock_until_group_drop() {
        let lock_dir = scratch();
        let first = LinuxCoordinator::new(&lock_dir).expect("first coordinator");
        let second = LinuxCoordinator::new(&lock_dir).expect("second coordinator");
        let scope = LockScope::Job("group-member".to_owned());
        let group = first
            .acquire_all(std::slice::from_ref(&scope))
            .expect("group");
        assert!(matches!(second.acquire(&scope), Err(LockError::Contended)));
        drop(group);
        let single = second.acquire(&scope).expect("released member");
        drop(single);
        let _ = std::fs::remove_dir_all(lock_dir);
    }

    #[cfg(unix)]
    #[test]
    fn lock_open_rejects_existing_symlink_target() {
        let lock_dir = scratch();
        let coordinator = LinuxCoordinator::new(&lock_dir).expect("coordinator");
        let scope = LockScope::Job("symlinked-lock".to_owned());
        let lock_path = lock_file_path(&lock_dir, &scope).expect("lock path");
        let outside = lock_dir.join("outside");
        std::fs::write(&outside, b"untouched").expect("outside file");
        std::os::unix::fs::symlink(&outside, &lock_path).expect("lock symlink");

        assert!(matches!(coordinator.acquire(&scope), Err(LockError::Io)));
        assert_eq!(
            std::fs::read(outside).expect("outside retained"),
            b"untouched"
        );
        let _ = std::fs::remove_dir_all(lock_dir);
    }

    #[test]
    fn only_would_block_classifies_as_contention() {
        assert_eq!(
            LinuxCoordinator::classify(rustix::io::Errno::AGAIN),
            LockError::Contended
        );
        assert_eq!(
            LinuxCoordinator::classify(rustix::io::Errno::ACCESS),
            LockError::Io
        );
        assert_eq!(
            LinuxCoordinator::classify(rustix::io::Errno::IO),
            LockError::Io
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_lock_directory_alias_shares_lock_identity() {
        let base = scratch();
        let real = base.join("locks");
        let alias = base.join("alias");
        std::fs::create_dir_all(&real).expect("real lock directory");
        std::os::unix::fs::symlink(&real, &alias).expect("alias symlink");

        let via_real = LinuxCoordinator::new(&real).expect("real coordinator");
        let via_alias = LinuxCoordinator::new(&alias).expect("alias coordinator");
        let scope = LockScope::Job("alias-identity".to_owned());

        let held = via_real.acquire(&scope).expect("held via real path");
        // The alias resolves to the same lock identity, so a second coordinator
        // must not believe it owns the resource just because it used a
        // different path spelling.
        assert!(matches!(
            via_alias.acquire(&scope),
            Err(LockError::Contended)
        ));
        drop(held);

        let released = via_alias.acquire(&scope).expect("released for alias");
        drop(released);
        let _ = std::fs::remove_dir_all(base);
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
