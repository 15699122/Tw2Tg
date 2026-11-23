//! Windows NTFS coordinator, preserving the shared scope/admission contract.
use super::{
    CoordinationGuard, Coordinator, LockError, LockScope, MoveError, NoReplaceMover, lock_file_path,
};
use crate::windows_filesystem::PinnedPath;
use std::collections::BTreeMap;
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LockMode {
    Shared,
    Exclusive,
}

struct LockRequest {
    path: PathBuf,
    mode: LockMode,
}

struct DestinationLocks {
    requests: Vec<LockRequest>,
    pins: Vec<PinnedPath>,
}

/// Applies Windows 1:1 case folding (`$UpCase` semantics) so distinct Unicode
/// names like `straße` and `STRASSE` do not falsely collide while true 1:1
/// NTFS/ReFS aliases still match.
fn upcase_windows_char(ch: char) -> char {
    match ch {
        '/' => '\\',
        '\u{1f80}'..='\u{1f87}' | '\u{1f90}'..='\u{1f97}' | '\u{1fa0}'..='\u{1fa7}' => {
            char::from_u32(u32::from(ch) + 8).unwrap_or(ch)
        }
        '\u{1fb3}' => '\u{1fbc}',
        '\u{1fc3}' => '\u{1fcc}',
        '\u{1ff3}' => '\u{1ffc}',
        _ => {
            let mut upper = ch.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(single), None) => single,
                _ => ch,
            }
        }
    }
}

fn upcase_windows_path(text: &str) -> String {
    text.chars().map(upcase_windows_char).collect()
}

fn is_valid_83_byte(byte: u8) -> bool {
    byte.is_ascii_uppercase()
        || byte.is_ascii_digit()
        || matches!(
            byte,
            b'$' | b'%'
                | b'\''
                | b'-'
                | b'_'
                | b'@'
                | b'~'
                | b'`'
                | b'!'
                | b'('
                | b')'
                | b'{'
                | b'}'
                | b'^'
                | b'#'
                | b'&'
        )
}

fn is_valid_83_name(upcased_leaf: &str) -> Option<&str> {
    let (base, ext) = match upcased_leaf.split_once('.') {
        Some((base, ext)) => {
            if ext.contains('.') {
                return None;
            }
            (base, Some(ext))
        }
        None => (upcased_leaf, None),
    };
    if base.is_empty() || base.len() > 8 || !base.bytes().all(is_valid_83_byte) {
        return None;
    }
    if ext.is_some_and(|ext| ext.is_empty() || ext.len() > 3 || !ext.bytes().all(is_valid_83_byte))
    {
        return None;
    }
    Some(base)
}

/// Coordinates pre-creation 8.3 tilde aliases (`Exclusive`) against non-8.3
/// long names sharing the same 6-character NTFS stem (`Shared`).
fn classify_precreation_83_stem(upcased_leaf: &str) -> Option<(String, LockMode)> {
    if let Some(base) = is_valid_83_name(upcased_leaf) {
        let (stem, digits) = base.rsplit_once('~')?;
        if !stem.is_empty()
            && stem.len() <= 6
            && !digits.is_empty()
            && digits.len() <= 6
            && digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Some((stem.to_owned(), LockMode::Exclusive));
        }
        return None;
    }
    let base_source = match upcased_leaf.rsplit_once('.') {
        Some((base, _)) if !base.is_empty() => base,
        _ => upcased_leaf,
    };
    let stem: String = base_source
        .bytes()
        .filter(|&byte| byte != b' ' && byte != b'.' && is_valid_83_byte(byte))
        .take(6)
        .map(char::from)
        .collect();
    if stem.is_empty() {
        None
    } else {
        Some((stem, LockMode::Shared))
    }
}

fn push_83_stem_request(
    requests: &mut Vec<LockRequest>,
    lock_directory: &Path,
    upcased_parent: &str,
    upcased_leaf: &str,
) -> Result<(), LockError> {
    if let Some((stem, mode)) = classify_precreation_83_stem(upcased_leaf) {
        let stem_scope = PathBuf::from(format!("8.3-stem:{upcased_parent}\\{stem}"));
        requests.push(LockRequest {
            path: lock_file_path(lock_directory, &LockScope::Destination(stem_scope))?,
            mode,
        });
    }
    Ok(())
}

fn unpinned_parent_lock_path(lock_directory: &Path) -> PathBuf {
    // Subdirectory path cannot collide with `LockScope::Job` (`/` and `\` rejected)
    // or `LockScope::Destination` (`d<16hex>.lock` at the root).
    lock_directory.join("__win").join("unpinned-parent.lock")
}

fn destination_locks(
    lock_directory: &Path,
    destination: &Path,
) -> Result<DestinationLocks, LockError> {
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
    let mut requests = Vec::new();
    let has_missing_parent;
    match absolute.symlink_metadata() {
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
            let canonical = absolute.canonicalize()?;
            let canonical_text = canonical.to_str().ok_or(LockError::InvalidName)?;
            let upcased_canonical = upcase_windows_path(canonical_text);
            requests.push(LockRequest {
                path: lock_file_path(
                    lock_directory,
                    &LockScope::Destination(PathBuf::from(&upcased_canonical)),
                )?,
                mode: LockMode::Exclusive,
            });
            if let (Some(parent), Some(leaf)) = (canonical.parent(), canonical.file_name()) {
                let parent_str = parent.to_str().ok_or(LockError::InvalidName)?;
                let leaf_str = leaf.to_str().ok_or(LockError::InvalidName)?;
                push_83_stem_request(
                    &mut requests,
                    lock_directory,
                    &upcase_windows_path(parent_str),
                    &upcase_windows_path(leaf_str),
                )?;
            }
            has_missing_parent = false;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent_canonical = ancestor.canonicalize()?;
            let normalized_target = suffix
                .components()
                .fold(parent_canonical.clone(), |acc, part| {
                    acc.join(part.as_os_str())
                });
            let target_text = normalized_target.to_str().ok_or(LockError::InvalidName)?;
            let upcased_target = upcase_windows_path(target_text);
            requests.push(LockRequest {
                path: lock_file_path(
                    lock_directory,
                    &LockScope::Destination(PathBuf::from(upcased_target)),
                )?,
                mode: LockMode::Exclusive,
            });
            has_missing_parent = suffix.components().count() > 1;
            if !has_missing_parent {
                let leaf_name = suffix
                    .components()
                    .next()
                    .and_then(|part| part.as_os_str().to_str())
                    .ok_or(LockError::InvalidName)?;
                let upcased_parent =
                    upcase_windows_path(parent_canonical.to_str().ok_or(LockError::InvalidName)?);
                let upcased_leaf = upcase_windows_path(leaf_name);
                push_83_stem_request(
                    &mut requests,
                    lock_directory,
                    &upcased_parent,
                    &upcased_leaf,
                )?;
            }
        }
        Err(error) => return Err(error.into()),
    }
    requests.push(LockRequest {
        path: unpinned_parent_lock_path(lock_directory),
        mode: if has_missing_parent {
            LockMode::Exclusive
        } else {
            LockMode::Shared
        },
    });
    Ok(DestinationLocks { requests, pins })
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
        let mut requests = BTreeMap::<PathBuf, LockMode>::new();
        let mut record_request = |path: PathBuf, mode: LockMode| {
            requests
                .entry(path)
                .and_modify(|existing| {
                    if mode > *existing {
                        *existing = mode;
                    }
                })
                .or_insert(mode);
        };
        for scope in scopes {
            match scope {
                LockScope::Destination(path) => {
                    let locks = destination_locks(&self.directory, path)?;
                    destination_pins.extend(locks.pins);
                    for request in locks.requests {
                        record_request(request.path, request.mode);
                    }
                }
                _ => {
                    let path = lock_file_path(&self.directory, scope)?;
                    record_request(path, LockMode::Exclusive);
                }
            }
        }
        let mut files = Vec::with_capacity(requests.len());
        for (path, mode) in requests {
            let file = PinnedPath::lock_file(&path)?;
            let attempt = match mode {
                LockMode::Exclusive => file.file().try_lock(),
                LockMode::Shared => file.file().try_lock_shared(),
            };
            attempt.map_err(|error| match error {
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
