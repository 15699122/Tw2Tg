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
        let path = self.safe_staging_child(Path::new(job_id))?;
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
        let staging = self.safe_staging_child(Path::new(job_id))?;
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
            return Ok(self.safe_staging_child(Path::new(job_id))?.is_dir());
        }
        Ok(self.safe_child(relative)?.is_dir())
    }

    /// Resolve a relative path under `root` and refuse any intermediate link.
    ///
    /// Lexical checks alone are not enough: an intermediate directory can be a
    /// symlink (or a Windows reparse point / junction) that points outside the
    /// root, so every existing component below the root is inspected.
    fn resolve_within(root: &Path, relative: &Path) -> Result<PathBuf, StorageError> {
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
        let mut current = root.to_path_buf();
        for component in relative.components() {
            current.push(component);
            // A component that already exists as a link escapes the root, so
            // reject it. A missing component is fine: the caller creates it.
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if is_reparse_point(&metadata) {
                        return Err(StorageError::InvalidPath);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => break,
                Err(error) => return Err(StorageError::Io(error)),
            }
        }
        Ok(root.join(relative))
    }

    fn safe_child(&self, relative: &Path) -> Result<PathBuf, StorageError> {
        Self::resolve_within(&self.archive_root, relative)
    }

    fn safe_staging_child(&self, relative: &Path) -> Result<PathBuf, StorageError> {
        Self::resolve_within(&self.staging_root, relative)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "xarchive-file-store-{label}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    #[test]
    fn rejects_parent_and_absolute_paths() {
        let root = temp_root("lexical");
        let store = FileStore::new(&root).expect("file store");

        assert!(matches!(
            store.write_text(Path::new("../escape.txt"), "x"),
            Err(StorageError::InvalidPath)
        ));
        assert!(matches!(
            store.write_text(Path::new("/absolute.txt"), "x"),
            Err(StorageError::InvalidPath)
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn allows_a_missing_intermediate_directory() {
        let root = temp_root("missing");
        let store = FileStore::new(&root).expect("file store");

        store
            .write_text(Path::new("new/nested/file.txt"), "ok")
            .expect("write creates the missing directories");
        assert!(root.join("new/nested/file.txt").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    /// ENG-03: an intermediate directory that is a symlink (or, on Windows, a
    /// reparse point / junction) must not allow writes outside the root.
    #[cfg(unix)]
    #[test]
    fn rejects_an_intermediate_symlink_that_points_outside_the_root() {
        use std::os::unix::fs::symlink;

        let root = temp_root("symlink");
        let outside = temp_root("symlink-outside");
        fs::write(outside.join("secret.txt"), "sensitive").expect("outside file");

        let store = FileStore::new(&root).expect("file store");
        symlink(&outside, root.join("escape")).expect("create symlink");

        let result = store.write_text(Path::new("escape/payload.txt"), "x");
        assert!(
            matches!(result, Err(StorageError::InvalidPath)),
            "intermediate symlink must be rejected, got {result:?}"
        );
        assert!(
            !outside.join("payload.txt").exists(),
            "write escaped the archive root"
        );

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_final_symlink_component() {
        use std::os::unix::fs::symlink;

        let root = temp_root("final-symlink");
        let outside = temp_root("final-symlink-outside");
        fs::write(outside.join("target.txt"), "sensitive").expect("outside file");

        let store = FileStore::new(&root).expect("file store");
        symlink(outside.join("target.txt"), root.join("link.txt")).expect("symlink");

        let result = store.write_text(Path::new("link.txt"), "overwritten");
        assert!(
            matches!(result, Err(StorageError::InvalidPath)),
            "final symlink must be rejected, got {result:?}"
        );
        assert_eq!(
            fs::read_to_string(outside.join("target.txt")).expect("read"),
            "sensitive"
        );

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(outside);
    }
}
