//! Windows NTFS coordinator, preserving the shared scope/admission contract.
use super::{
    CoordinationGuard, Coordinator, LockError, LockScope, MoveError, NoReplaceMover, lock_file_path,
};
use crate::windows_filesystem::PinnedPath;
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

fn destination_lock_identity(destination: &Path) -> Result<(PathBuf, Vec<PinnedPath>), LockError> {
    if destination.as_os_str().is_empty()
        || destination
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(LockError::InvalidName);
    }
    let absolute = std::path::absolute(destination)?;
    let mut ancestor = absolute.parent().ok_or(LockError::InvalidName)?;
    loop {
        match ancestor.symlink_metadata() {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ancestor = ancestor.parent().ok_or(LockError::InvalidName)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    let mut pins = vec![PinnedPath::directory(ancestor, false)?];
    let suffix = absolute
        .strip_prefix(ancestor)
        .map_err(|_| LockError::InvalidName)?;
    if suffix.components().any(|part| {
        !matches!(part, Component::Normal(name) if name.to_str().is_some_and(|name|
            !name.contains(':') && !name.ends_with(['.', ' '])))
    }) {
        return Err(LockError::InvalidName);
    }
    // Resolve an existing leaf as well as its ancestors: NTFS can expose the
    // same directory through a long name and an 8.3 alias. Pin and reject
    // reparse points before resolving it. A not-yet-created leaf keeps the
    // canonical parent plus the requested name.
    let canonical = match absolute.symlink_metadata() {
        Ok(metadata) => {
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(LockError::InvalidName);
            }
            let leaf = if metadata.is_dir() {
                PinnedPath::directory(&absolute, false)?
            } else if metadata.is_file() {
                PinnedPath::media(&absolute)?
            } else {
                return Err(LockError::InvalidName);
            };
            pins.push(leaf);
            absolute.canonicalize()?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ancestor.canonicalize()?.join(suffix)
        }
        Err(error) => return Err(error.into()),
    };
    let text = canonical.to_str().ok_or(LockError::InvalidName)?;
    Ok((PathBuf::from(text.replace('/', "\\").to_uppercase()), pins))
}

pub struct WindowsNoReplaceMover;

impl NoReplaceMover for WindowsNoReplaceMover {
    fn move_no_replace(&self, source: &Path, destination: &Path) -> Result<(), MoveError> {
        let source_parent = source.parent().ok_or(MoveError::InvalidSource)?;
        let destination_parent = destination.parent().ok_or(MoveError::Io)?;
        let _source_pin = PinnedPath::directory(source_parent, false).map_err(|_| MoveError::Io)?;
        let _destination_pin =
            PinnedPath::directory(destination_parent, false).map_err(|_| MoveError::Io)?;
        let source = source_parent
            .canonicalize()
            .map_err(|_| MoveError::Io)?
            .join(source.file_name().ok_or(MoveError::InvalidSource)?);
        let destination = destination_parent
            .canonicalize()
            .map_err(|_| MoveError::Io)?
            .join(destination.file_name().ok_or(MoveError::Io)?);
        let metadata = source.symlink_metadata().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                MoveError::InvalidSource
            } else {
                MoveError::Io
            }
        })?;
        if metadata.file_attributes() & 0x400 != 0 || !(metadata.is_dir() || metadata.is_file()) {
            return Err(MoveError::InvalidSource);
        }
        atomicwrites::move_atomic(&source, &destination).map_err(|error| match error.kind() {
            std::io::ErrorKind::AlreadyExists => MoveError::DestinationExists,
            _ => MoveError::Io,
        })
    }
}

pub struct WindowsCoordinator {
    directory: PathBuf,
    _pin: PinnedPath,
}
pub struct WindowsGuard {
    files: Vec<PinnedPath>,
    // Preserve the identity used for each destination for the whole lock.
    _destination_pins: Vec<PinnedPath>,
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
        let mut destination_pins = Vec::new();
        let mut paths = scopes
            .iter()
            .map(|scope| match scope {
                LockScope::Destination(path) => {
                    let (identity, pins) = destination_lock_identity(path)?;
                    destination_pins.extend(pins);
                    lock_file_path(&self.directory, &LockScope::Destination(identity))
                }
                _ => lock_file_path(&self.directory, scope),
            })
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
        Ok(WindowsGuard {
            files,
            _destination_pins: destination_pins,
        })
    }
}
