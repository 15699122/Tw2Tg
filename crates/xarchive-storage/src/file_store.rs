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

    pub fn rename_staged_directory(
        &self,
        job_id: &str,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, StorageError> {
        self.commit_staging(job_id, destination)
    }

    /// Resolve an existing staging directory without creating it. Recovery must
    /// not turn a missing payload into an apparently valid empty staging tree.
    pub fn existing_staging_dir(&self, job_id: &str) -> Result<PathBuf, StorageError> {
        let path = self.safe_staging_child(job_id)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !is_reparse_point(&metadata) => Ok(path),
            Ok(_) => Err(StorageError::InvalidPath),
            Err(error) => Err(StorageError::Io(error)),
        }
    }

    /// Rename one already-validated regular file without replacing a target.
    pub fn rename_staged_file(
        &self,
        job_id: &str,
        source: &str,
        destination: &str,
    ) -> Result<(), StorageError> {
        let staging = self.existing_staging_dir(job_id)?;
        let source = Self::resolve_within(&staging, Path::new(source))?;
        let destination = Self::resolve_within(&staging, Path::new(destination))?;
        let source_metadata = fs::symlink_metadata(&source)?;
        if !source_metadata.file_type().is_file() || is_reparse_point(&source_metadata) {
            return Err(StorageError::InvalidPath);
        }
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                return Err(StorageError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "rename destination already exists",
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(StorageError::Io(error)),
        }
        fs::rename(source, destination)?;
        Ok(())
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

    /// Resolve a relative path under `root` and refuse any intermediate link.
    ///
    /// Lexical checks alone are not enough: an intermediate directory can be a
    /// symlink (or a Windows reparse point / junction) that points outside the
    /// root, so every existing component below the root is inspected.
    ///
    /// `pub(crate)` so `build_archive_metadata` applies the same containment
    /// contract when it resolves sidecar file paths under staging (WQ-ENG-03).
    pub(crate) fn resolve_within(root: &Path, relative: &Path) -> Result<PathBuf, StorageError> {
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

        // ENG-03: also refuse an existing intermediate link between the staging
        // root and the job directory, so a redirected `staging_root` cannot move
        // job data outside the archive tree.
        Self::resolve_within(&self.staging_root, Path::new(&component))
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
