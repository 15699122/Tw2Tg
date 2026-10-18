//! Windows handle-based path pins. No reparse traversal or delete sharing.
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf, Prefix};

const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const BACKUP_SEMANTICS: u32 = 0x0200_0000;
const REPARSE_POINT: u32 = 0x400;

/// Pins every ancestor, preventing rename/replacement until this guard drops.
#[derive(Debug)]
pub struct PinnedPath {
    file: File,
    _parents: Vec<File>,
}

impl PinnedPath {
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Existing local directory, or create missing directories for lock storage.
    pub fn directory(path: &Path, create: bool) -> io::Result<Self> {
        Self::open(path, true, create, true)
    }

    /// A stable read-only media file; existing writers cannot share this handle.
    pub fn media(path: &Path) -> io::Result<Self> {
        Self::open(path, false, false, false)
    }

    pub(crate) fn lock_file(path: &Path) -> io::Result<Self> {
        Self::open(path, false, true, true)
    }

    fn open(path: &Path, directory: bool, create: bool, share_write: bool) -> io::Result<Self> {
        let absolute = std::path::absolute(path)?;
        let mut current = PathBuf::new();
        let mut pins = Vec::new();
        let parts: Vec<_> = absolute.components().collect();
        if !matches!(parts.first(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(io::Error::other(
                "only local Windows drive paths are supported",
            ));
        }
        for (index, part) in parts.iter().enumerate() {
            if matches!(part, Component::ParentDir | Component::CurDir) {
                return Err(io::Error::other("non-normal Windows path"));
            }
            if let Component::Normal(name) = part {
                if name.to_string_lossy().contains(':') {
                    return Err(io::Error::other("alternate data stream path rejected"));
                }
            }
            current.push(part.as_os_str());
            if matches!(part, Component::Prefix(_)) {
                continue;
            }
            let last = index + 1 == parts.len();
            let is_directory = !last || directory;
            if is_directory && create && !current.try_exists()? {
                match std::fs::create_dir(&current) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error),
                }
            }
            let mut options = OpenOptions::new();
            options
                .read(true)
                .share_mode(if last && !share_write { 1 } else { 3 })
                .custom_flags(OPEN_REPARSE_POINT | BACKUP_SEMANTICS);
            if last && !directory && create {
                options.write(true).create(true).truncate(false);
            }
            let file = options.open(&current)?;
            let metadata = file.metadata()?;
            if metadata.file_attributes() & REPARSE_POINT != 0
                || (is_directory && !metadata.is_dir())
                || (!is_directory && !metadata.is_file())
            {
                return Err(io::Error::other(
                    "reparse point or unexpected file type rejected",
                ));
            }
            pins.push(file);
        }
        let file = pins
            .pop()
            .ok_or_else(|| io::Error::other("path has no filesystem object"))?;
        Ok(Self {
            file,
            _parents: pins,
        })
    }
}
