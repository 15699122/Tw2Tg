use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use crate::{StorageError, UserProfileFile};

pub struct FileStore {
    archive_root: PathBuf,
    staging_root: PathBuf,
}

impl FileStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Self::with_staging_root(root.clone(), root.join("_staging"))
    }

    pub fn with_staging_root(
        archive_root: impl Into<PathBuf>,
        staging_root: impl Into<PathBuf>,
    ) -> Result<Self, StorageError> {
        let archive_root = archive_root.into();
        let staging_root = staging_root.into();
        fs::create_dir_all(&archive_root)?;
        fs::create_dir_all(&staging_root)?;
        Ok(Self {
            archive_root,
            staging_root,
        })
    }

    pub fn staging_dir(&self, job_id: &str) -> Result<PathBuf, StorageError> {
        let path = self.safe_staging_child(job_id)?;
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn write_json<T: serde::Serialize>(
        &self,
        relative: impl AsRef<Path>,
        value: &T,
    ) -> Result<PathBuf, StorageError> {
        let path = self.safe_child(relative.as_ref())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_vec_pretty(value)?)?;
        Ok(path)
    }

    pub fn write_text(
        &self,
        relative: impl AsRef<Path>,
        content: &str,
    ) -> Result<PathBuf, StorageError> {
        let path = self.safe_child(relative.as_ref())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        Ok(path)
    }

    /// Resolve the `profile.json` path inside the stable user directory.
    pub fn user_profile_path(&self, stable_directory_name: &str) -> Result<PathBuf, StorageError> {
        if stable_directory_name.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "stable directory name must not be empty".into(),
            ));
        }
        self.safe_child(
            &Path::new("Users")
                .join(stable_directory_name)
                .join("profile.json"),
        )
    }

    /// Write or refresh the portable `profile.json` in the user directory.
    pub fn write_user_profile(&self, profile: &UserProfileFile) -> Result<PathBuf, StorageError> {
        let path = self.user_profile_path(&profile.stable_directory_name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_vec_pretty(profile)?)?;
        Ok(path)
    }

    pub fn sha256(path: impl AsRef<Path>) -> Result<String, StorageError> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn commit_staging(
        &self,
        job_id: &str,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, StorageError> {
        let staging = self.safe_staging_child(job_id)?;
        if !staging.is_dir() {
            return Err(StorageError::InvalidPath);
        }
        let destination = self.safe_child(destination.as_ref())?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            return Err(StorageError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "destination already exists",
            )));
        }
        fs::rename(&staging, &destination)?;
        Ok(destination)
    }

    /// Return whether a job's staging directory or final archive directory is
    /// present without creating either directory. Recovery must inspect the
    /// filesystem as-is; calling `staging_dir` would create a false positive.
    pub fn recovery_directory_exists(
        &self,
        job_id: &str,
        relative: impl AsRef<Path>,
    ) -> Result<bool, StorageError> {
        let relative = relative.as_ref();
        if relative == Path::new("_staging") {
            return Ok(self.safe_staging_child(job_id)?.is_dir());
        }
        Ok(self.safe_child(relative)?.is_dir())
    }

    /// Resolve a committed archive-relative path under the archive root.
    ///
    /// The batch dispatcher verifies that a committed archive directory and its
    /// recorded media files still exist before it skips an already-archived
    /// candidate. Rejecting absolute paths and `..` keeps a persisted
    /// `archive_directory` value from escaping the archive root.
    pub fn archive_path(&self, relative: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
        self.safe_child(relative.as_ref())
    }

    fn safe_child(&self, relative: &Path) -> Result<PathBuf, StorageError> {
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(StorageError::InvalidPath);
        }
        Ok(self.archive_root.join(relative))
    }

    fn safe_staging_child(&self, job_id: &str) -> Result<PathBuf, StorageError> {
        // Staging directories are keyed by a single opaque job ID, not by a
        // caller-supplied relative path. Keep that boundary explicit on every
        // platform before adapting the name to the host filesystem.
        if job_id.is_empty() || job_id.contains('/') || job_id.contains('\\') {
            return Err(StorageError::InvalidPath);
        }
        let relative = Path::new(job_id);
        if relative.components().count() != 1
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(StorageError::InvalidPath);
        }

        #[cfg(windows)]
        let component = windows_safe_component(job_id);
        #[cfg(not(windows))]
        let component = job_id.to_owned();

        Ok(self.staging_root.join(component))
    }
}

/// Map opaque staging IDs to valid Windows filename components. Percent is
/// escaped too, making the mapping unambiguous while leaving ordinary IDs
/// unchanged. Persisted job IDs and archive metadata retain their original
/// value; only the staging path uses this representation.
#[cfg(windows)]
fn windows_safe_component(value: &str) -> String {
    let trailing_start = value.trim_end_matches([' ', '.']).len();
    let base = value
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.']);
    let reserved_device_name = matches!(
        base.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );

    let mut encoded = String::with_capacity(value.len());
    for (index, character) in value.char_indices() {
        let invalid = character.is_ascii()
            && (character <= '\u{1f}'
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
                || character == '%');
        let trailing_dot_or_space = index >= trailing_start && matches!(character, '.' | ' ');
        let reserved_prefix = reserved_device_name && index == 0;
        if invalid || trailing_dot_or_space || reserved_prefix {
            for byte in character.to_string().bytes() {
                use std::fmt::Write as _;
                write!(&mut encoded, "%{byte:02X}").expect("writing to String cannot fail");
            }
        } else {
            encoded.push(character);
        }
    }
    encoded
}

#[cfg(unix)]
pub(crate) fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
pub(crate) fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
