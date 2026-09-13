use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use crate::{StorageError, UserProfileFile};

pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("_staging"))?;
        Ok(Self { root })
    }

    pub fn staging_dir(&self, job_id: &str) -> Result<PathBuf, StorageError> {
        let path = self.safe_child(&Path::new("_staging").join(job_id))?;
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
        let staging = self.safe_child(&Path::new("_staging").join(job_id))?;
        if !staging.is_dir() {
            return Err(StorageError::InvalidPath);
        }
        let destination = self.safe_child(destination.as_ref())?;
        if destination.starts_with(self.root.join("_staging")) {
            return Err(StorageError::InvalidPath);
        }
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
        Ok(self.root.join(relative))
    }
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
