use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use crate::{StorageError, UserProfileFile};

pub struct FileStore {
    archive_root: PathBuf,
    staging_root: PathBuf,
    // `Send` because a `FileStore` is moved across runner and worker threads;
    // every platform mover adapter is a stateless/unit or `Send` handle.
    mover: Box<dyn crate::workflow_bridge::NoReplaceMover + Send>,
}

#[cfg(not(unix))]
struct UnsupportedNoReplaceMover;

#[cfg(not(unix))]
impl crate::workflow_bridge::NoReplaceMover for UnsupportedNoReplaceMover {
    fn move_no_replace(
        &self,
        _source: &Path,
        _destination: &Path,
    ) -> Result<(), crate::workflow_bridge::MoveError> {
        Err(crate::workflow_bridge::MoveError::Unsupported)
    }
}

#[cfg(unix)]
struct NativeNoReplaceMover;

#[cfg(unix)]
impl crate::workflow_bridge::NoReplaceMover for NativeNoReplaceMover {
    fn move_no_replace(
        &self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), crate::workflow_bridge::MoveError> {
        crate::workflow_bridge::NoReplaceMover::move_no_replace(
            &crate::workflow_bridge::LinuxNoReplaceMover,
            source,
            destination,
        )
    }
}

impl FileStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Self::with_staging_root(root.clone(), root.join("_staging"))
    }

    /// Construct a store with an explicit atomic no-replace platform adapter.
    /// Production composition roots must inject the adapter they have validated;
    /// the default constructors remain fail-closed off Unix.
    pub fn with_no_replace_mover<M>(
        archive_root: impl Into<PathBuf>,
        staging_root: impl Into<PathBuf>,
        mover: M,
    ) -> Result<Self, StorageError>
    where
        M: crate::workflow_bridge::NoReplaceMover + Send + 'static,
    {
        #[cfg(unix)]
        {
            Self::build(archive_root, staging_root, Box::new(mover))
        }
        #[cfg(not(unix))]
        {
            Self::build(archive_root, staging_root, Box::new(mover))
        }
    }

    fn build(
        archive_root: impl Into<PathBuf>,
        staging_root: impl Into<PathBuf>,
        mover: Box<dyn crate::workflow_bridge::NoReplaceMover + Send>,
    ) -> Result<Self, StorageError> {
        let archive_root = archive_root.into();
        let staging_root = staging_root.into();
        fs::create_dir_all(&archive_root)?;
        fs::create_dir_all(&staging_root)?;
        Ok(Self {
            archive_root,
            staging_root,
            mover,
        })
    }

    pub fn with_staging_root(
        archive_root: impl Into<PathBuf>,
        staging_root: impl Into<PathBuf>,
    ) -> Result<Self, StorageError> {
        #[cfg(unix)]
        {
            Self::build(archive_root, staging_root, Box::new(NativeNoReplaceMover))
        }
        #[cfg(not(unix))]
        {
            Self::build(
                archive_root,
                staging_root,
                Box::new(UnsupportedNoReplaceMover),
            )
        }
    }

    pub fn staging_dir(&self, job_id: &str) -> Result<PathBuf, StorageError> {
        let path = self.safe_staging_child(job_id)?;
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Attempt-isolated staging directory for callers that already hold the
    /// attempt's authorization.
    ///
    /// Each archive attempt gets its own staging tree (`job_id/attempt-N`)
    /// so a stale worker can at most keep writing its own attempt directory
    /// and can never observe or mutate the directory of a newer attempt.
    /// Recovery reuses the recorded attempt's directory; only a retry that
    /// has revoked the old attempt's commit rights creates a new one. The
    /// legacy `job_id`-level `staging_dir` stays for pre-C1 callers and must
    /// not be used by the v2 pipeline.
    ///
    /// The directory is created when missing. Callers that must not silently
    /// reuse an existing attempt tree (in particular, creation of a fresh
    /// attempt staging area at claim time) must use
    /// [`Self::create_attempt_staging_dir`] instead.
    #[deprecated(
        since = "0.1.0",
        note = "use create_attempt_staging_dir for fresh attempt creation or existing_attempt_staging_dir for recovery; do not claim a fresh attempt through this helper"
    )]
    pub fn attempt_staging_dir(
        &self,
        job_id: &str,
        attempt_count: u32,
    ) -> Result<PathBuf, StorageError> {
        if attempt_count == 0 {
            return Err(StorageError::InvalidPath);
        }
        let job = self.safe_staging_child(job_id)?;
        fs::create_dir_all(&job)?;
        let name = format!("attempt-{attempt_count}");
        let path = Self::resolve_within(&job, Path::new(&name))?;
        if path != job.join(&name) {
            return Err(StorageError::InvalidPath);
        }
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Create an attempt-scoped staging tree only when it does not already
    /// exist. A retry must never reuse files left by an earlier invocation of
    /// the same durable attempt.
    pub fn create_attempt_staging_dir(
        &self,
        job_id: &str,
        attempt_count: u32,
    ) -> Result<PathBuf, StorageError> {
        let staging_metadata = fs::symlink_metadata(&self.staging_root)?;
        if !staging_metadata.is_dir() || is_reparse_point(&staging_metadata) {
            return Err(StorageError::InvalidPath);
        }
        let path = self.attempt_staging_path(job_id, attempt_count)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            let metadata = fs::symlink_metadata(parent)?;
            if !metadata.is_dir() || is_reparse_point(&metadata) {
                return Err(StorageError::InvalidPath);
            }
        }
        match fs::create_dir(&path) {
            Ok(()) => Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(StorageError::Io(error))
            }
            Err(error) => Err(StorageError::Io(error)),
        }
    }

    /// Resolve an existing attempt staging directory without creating it.
    /// Recovery must not turn a missing payload into an apparently valid
    /// empty staging tree.
    pub fn existing_attempt_staging_dir(
        &self,
        job_id: &str,
        attempt_count: u32,
    ) -> Result<PathBuf, StorageError> {
        let path = self.attempt_staging_path(job_id, attempt_count)?;
        let staging_metadata = fs::symlink_metadata(&self.staging_root)?;
        if !staging_metadata.is_dir() || is_reparse_point(&staging_metadata) {
            return Err(StorageError::InvalidPath);
        }
        let job_directory = self.safe_staging_child(job_id)?;
        let job_metadata = fs::symlink_metadata(&job_directory)?;
        if !job_metadata.is_dir() || is_reparse_point(&job_metadata) {
            return Err(StorageError::InvalidPath);
        }
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !is_reparse_point(&metadata) => Ok(path),
            Ok(_) => Err(StorageError::InvalidPath),
            Err(error) => Err(StorageError::Io(error)),
        }
    }

    fn attempt_staging_path(
        &self,
        job_id: &str,
        attempt_count: u32,
    ) -> Result<PathBuf, StorageError> {
        if attempt_count == 0 {
            return Err(StorageError::InvalidPath);
        }
        let job = self.safe_staging_child(job_id)?;
        let name = format!("attempt-{attempt_count}");
        let path = Self::resolve_within(&job, Path::new(&name))?;
        if path != job.join(&name) {
            return Err(StorageError::InvalidPath);
        }
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
        self.mover
            .move_no_replace(&source, &destination)
            .map_err(crate::workflow_bridge::bridge_move_error)
    }

    /// Replay one whole v2 rename stage inside an attempt staging tree.
    ///
    /// `edges` and `identities` describe the same immutable plan in whole-phase
    /// order. Each pending edge moves with the injected platform atomic
    /// no-replace operation. Both the existing source
    /// and completed destination are checked against the recorded size/hash;
    /// missing files, links, I/O errors and identity conflicts fail closed.
    pub fn replay_attempt_rename_stage(
        &self,
        job_id: &str,
        attempt_count: u32,
        edges: &[crate::workflow_bridge::RenameEdge],
        identities: &[crate::ArchiveRenamePlan],
        temporary_stage: bool,
    ) -> Result<(), StorageError> {
        if edges.len() != identities.len() {
            return Err(StorageError::InvalidMetadata(
                "rename edge and identity counts differ".into(),
            ));
        }
        let staging = self.existing_attempt_staging_dir(job_id, attempt_count)?;
        for (edge, identity) in edges.iter().zip(identities) {
            let (source, destination) = if temporary_stage {
                edge.temporary_edge()
            } else {
                edge.final_edge()
            };
            let (expected_source, expected_destination) = if temporary_stage {
                (
                    Path::new(&identity.source_path),
                    Path::new(&identity.temporary_path),
                )
            } else {
                (
                    Path::new(&identity.temporary_path),
                    Path::new(&identity.final_path),
                )
            };
            if expected_source != source || expected_destination != destination {
                return Err(StorageError::InvalidMetadata(
                    "rename edge does not match its immutable identity plan".into(),
                ));
            }
            let source_path = Self::resolve_within(&staging, source)?;
            let destination_path = Self::resolve_within(&staging, destination)?;
            let source_exists =
                verified_regular_file(&source_path, identity.size_bytes, &identity.sha256)?;
            let destination_exists =
                verified_regular_file(&destination_path, identity.size_bytes, &identity.sha256)?;
            match (source_exists, destination_exists) {
                (true, false) => {
                    self.mover
                        .move_no_replace(&source_path, &destination_path)
                        .map_err(crate::workflow_bridge::bridge_move_error)?;
                }
                (false, true) => {}
                (true, true) => {
                    return Err(StorageError::Io(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "rename destination already exists",
                    )));
                }
                (false, false) => return Err(StorageError::InvalidPath),
            }
        }
        Ok(())
    }

    pub fn recover_attempt_rename(
        &self,
        job_id: &str,
        attempt_count: u32,
        plan: &crate::ArchiveRenamePlan,
        temporary_stage: bool,
    ) -> Result<(), StorageError> {
        let staging = self.existing_attempt_staging_dir(job_id, attempt_count)?;
        let (source, destination) = if temporary_stage {
            (&plan.source_path, &plan.temporary_path)
        } else {
            (&plan.temporary_path, &plan.final_path)
        };
        self.recover_file_within(&staging, source, destination, plan.size_bytes, &plan.sha256)
    }

    fn recover_file_within(
        &self,
        root: &Path,
        source: &str,
        destination: &str,
        size: u64,
        sha: &str,
    ) -> Result<(), StorageError> {
        let source_path = Self::resolve_within(root, Path::new(source))?;
        let destination_path = Self::resolve_within(root, Path::new(destination))?;
        let source_exists = verified_regular_file(&source_path, size, sha)?;
        let destination_exists = verified_regular_file(&destination_path, size, sha)?;
        match (source_exists, destination_exists) {
            (true, false) => {
                self.mover
                    .move_no_replace(&source_path, &destination_path)
                    .map_err(crate::workflow_bridge::bridge_move_error)?;
                Ok(())
            }
            (false, true) => Ok(()),
            (true, true) => Err(StorageError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "both rename endpoints exist",
            ))),
            (false, false) => Err(StorageError::InvalidPath),
        }
    }

    pub fn write_attempt_json<T: serde::Serialize>(
        &self,
        job_id: &str,
        attempt: u32,
        relative: &str,
        value: &T,
    ) -> Result<PathBuf, StorageError> {
        let root = self.existing_attempt_staging_dir(job_id, attempt)?;
        self.write_file_within(&root, relative, &serde_json::to_vec_pretty(value)?)
    }

    pub fn write_attempt_text(
        &self,
        job_id: &str,
        attempt: u32,
        relative: &str,
        value: &str,
    ) -> Result<PathBuf, StorageError> {
        let root = self.existing_attempt_staging_dir(job_id, attempt)?;
        self.write_file_within(&root, relative, value.as_bytes())
    }

    /// Create a new file without following links or replacing an existing
    /// entry. A worker must not be able to overwrite a file after PREPARED.
    pub fn create_attempt_file(
        &self,
        job_id: &str,
        attempt: u32,
        relative: &str,
        contents: &[u8],
    ) -> Result<PathBuf, StorageError> {
        let root = self.existing_attempt_staging_dir(job_id, attempt)?;
        self.write_file_within(&root, relative, contents)
    }

    fn write_file_within(
        &self,
        root: &Path,
        relative: &str,
        contents: &[u8],
    ) -> Result<PathBuf, StorageError> {
        let path = Self::resolve_within(root, Path::new(relative))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(0x0002_0000); // O_NOFOLLOW on Unix targets
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
        }
        use std::io::Write as _;
        let mut file = options.open(&path)?;
        file.write_all(contents)?;
        file.sync_all()?;
        Ok(path)
    }

    pub fn commit_attempt_staging(
        &self,
        job_id: &str,
        attempt: u32,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, StorageError> {
        let staging = self.existing_attempt_staging_dir(job_id, attempt)?;
        let destination = self.safe_child(destination.as_ref())?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        self.mover
            .move_no_replace(&staging, &destination)
            .map_err(crate::workflow_bridge::bridge_move_error)?;
        Ok(destination)
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
        self.mover
            .move_no_replace(&staging, &destination)
            .map_err(crate::workflow_bridge::bridge_move_error)?;
        Ok(destination)
    }

    pub fn commit_recovered_staging(
        &self,
        job_id: &str,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, StorageError> {
        let staging = self.existing_staging_dir(job_id)?;
        let destination = self.safe_child(destination.as_ref())?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::symlink_metadata(&destination) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.mover
                    .move_no_replace(&staging, &destination)
                    .map_err(crate::workflow_bridge::bridge_move_error)?;
                Ok(destination)
            }
            Ok(metadata) if metadata.is_dir() && !is_reparse_point(&metadata) => {
                Err(StorageError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "archive destination already exists",
                )))
            }
            Ok(_) => Err(StorageError::InvalidPath),
            Err(error) => Err(StorageError::Io(error)),
        }
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

fn verified_regular_file(path: &Path, size_bytes: u64, sha256: &str) -> Result<bool, StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() || is_reparse_point(&metadata) {
                return Err(StorageError::InvalidPath);
            }
            if metadata.len() != size_bytes || FileStore::sha256(path)? != sha256 {
                return Err(StorageError::InvalidMetadata(
                    "recovery file size or hash mismatch".into(),
                ));
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(StorageError::Io(error)),
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingMover(Arc<AtomicUsize>);

    impl crate::workflow_bridge::NoReplaceMover for CountingMover {
        fn move_no_replace(
            &self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), crate::workflow_bridge::MoveError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            crate::workflow_bridge::NoReplaceMover::move_no_replace(
                &crate::workflow_bridge::LinuxNoReplaceMover,
                source,
                destination,
            )
        }
    }

    struct UnsupportedMover;

    impl crate::workflow_bridge::NoReplaceMover for UnsupportedMover {
        fn move_no_replace(
            &self,
            _source: &Path,
            _destination: &Path,
        ) -> Result<(), crate::workflow_bridge::MoveError> {
            Err(crate::workflow_bridge::MoveError::Unsupported)
        }
    }

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
    fn attempt_recovery_uses_injected_mover() {
        let root = temp_root("injected-mover-recovery");
        let calls = Arc::new(AtomicUsize::new(0));
        let store = FileStore::with_no_replace_mover(
            root.join("archive"),
            root.join("staging"),
            CountingMover(calls.clone()),
        )
        .expect("file store");
        let staging = store
            .create_attempt_staging_dir("job", 1)
            .expect("attempt staging");
        let source = staging.join("source.bin");
        fs::write(&source, b"payload").expect("source");
        let plan = crate::ArchiveRenamePlan {
            source_path: "source.bin".into(),
            temporary_path: "temporary.bin".into(),
            final_path: "final.bin".into(),
            size_bytes: 7,
            sha256: FileStore::sha256(&source).expect("hash"),
        };

        store
            .recover_attempt_rename("job", 1, &plan, true)
            .expect("injected move succeeds");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        store
            .recover_attempt_rename("job", 1, &plan, false)
            .expect("second phase uses injected mover");
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn attempt_directory_commit_uses_injected_mover() {
        let root = temp_root("injected-mover-directory");
        let calls = Arc::new(AtomicUsize::new(0));
        let store = FileStore::with_no_replace_mover(
            root.join("archive"),
            root.join("staging"),
            CountingMover(calls.clone()),
        )
        .expect("file store");
        store
            .create_attempt_staging_dir("job", 1)
            .expect("attempt staging");

        let committed = store
            .commit_attempt_staging("job", 1, "Tweets/123")
            .expect("directory commit");
        assert!(committed.is_dir());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        store
            .create_attempt_staging_dir("legacy", 1)
            .expect("legacy attempt staging");
        // The job-level staging API is retained for old recovery records; its
        // publication still must use the same no-replace adapter.
        let legacy = store.staging_root.join("legacy");
        fs::create_dir_all(&legacy).expect("legacy staging");
        store
            .commit_staging("legacy", "Tweets/legacy")
            .expect("legacy commit");
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        fs::create_dir_all(store.staging_root.join("recovered")).expect("recovery staging");
        store
            .commit_recovered_staging("recovered", "Tweets/recovered")
            .expect("recovered commit");
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        let legacy_staging = store.staging_dir("legacy-file").expect("legacy staging");
        fs::write(legacy_staging.join("source.bin"), b"legacy").expect("legacy source");
        store
            .rename_staged_file("legacy-file", "source.bin", "destination.bin")
            .expect("legacy staged rename");
        assert_eq!(calls.load(Ordering::SeqCst), 4);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn attempt_rename_replay_uses_injected_mover() {
        let root = temp_root("injected-mover-replay");
        let calls = Arc::new(AtomicUsize::new(0));
        let store = FileStore::with_no_replace_mover(
            root.join("archive"),
            root.join("staging"),
            CountingMover(calls.clone()),
        )
        .expect("file store");
        let staging = store
            .create_attempt_staging_dir("job", 1)
            .expect("attempt staging");
        let source = staging.join("source.bin");
        fs::write(&source, b"payload").expect("source");
        let identity = crate::ArchiveRenamePlan {
            source_path: "source.bin".into(),
            temporary_path: "temporary.bin".into(),
            final_path: "final.bin".into(),
            size_bytes: 7,
            sha256: FileStore::sha256(&source).expect("hash"),
        };
        let edge = crate::workflow_bridge::RenameEdge::new(
            &identity.source_path,
            &identity.temporary_path,
            &identity.final_path,
        )
        .expect("valid edge");

        store
            .replay_attempt_rename_stage("job", 1, &[edge], &[identity], true)
            .expect("injected replay move");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unsupported_injected_mover_fails_closed_without_mutating_staging() {
        let root = temp_root("unsupported-injected-mover");
        let store = FileStore::with_no_replace_mover(
            root.join("archive"),
            root.join("staging"),
            UnsupportedMover,
        )
        .expect("file store");
        let staging = store
            .create_attempt_staging_dir("job", 1)
            .expect("attempt staging");
        let source = staging.join("source.bin");
        fs::write(&source, b"payload").expect("source");
        let plan = crate::ArchiveRenamePlan {
            source_path: "source.bin".into(),
            temporary_path: "temporary.bin".into(),
            final_path: "final.bin".into(),
            size_bytes: 7,
            sha256: FileStore::sha256(&source).expect("hash"),
        };

        assert!(matches!(
            store.recover_attempt_rename("job", 1, &plan, true),
            Err(StorageError::InvalidState(_))
        ));
        assert_eq!(fs::read(&source).expect("source unchanged"), b"payload");
        assert!(!staging.join("temporary.bin").exists());
        assert!(matches!(
            store.commit_attempt_staging("job", 1, "Tweets/123"),
            Err(StorageError::InvalidState(_))
        ));
        assert!(
            staging.is_dir(),
            "unsupported directory move must retain staging"
        );

        let _ = fs::remove_dir_all(root);
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
    fn attempt_rename_recovery_is_idempotent_and_checks_identity() {
        let root =
            std::env::temp_dir().join(format!("xarchive-file-store-v2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let files = FileStore::new(&root).expect("file store");
        let staging = files
            .create_attempt_staging_dir("job-v2", 1)
            .expect("attempt staging");
        let source = staging.join("source.jpg");
        fs::write(&source, b"media").expect("write source");
        let digest = FileStore::sha256(&source).expect("hash");

        let plan = crate::ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".temp.jpg".into(),
            final_path: "final.jpg".into(),
            size_bytes: 5,
            sha256: digest.clone(),
        };
        files
            .recover_attempt_rename("job-v2", 1, &plan, true)
            .expect("first move");
        files
            .recover_attempt_rename("job-v2", 1, &plan, true)
            .expect("replay completed move");
        assert!(!source.exists());
        assert_eq!(
            fs::read(staging.join(".temp.jpg")).expect("destination"),
            b"media"
        );

        assert!(
            files
                .recover_attempt_rename(
                    "job-v2",
                    1,
                    &crate::ArchiveRenamePlan {
                        source_path: "source.jpg".into(),
                        temporary_path: ".temp.jpg".into(),
                        final_path: "final.jpg".into(),
                        size_bytes: 4,
                        sha256: digest.clone(),
                    },
                    false
                )
                .is_err()
        );
        assert!(!staging.join("final.jpg").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn attempt_rename_recovery_rejects_conflicts_and_missing_edges() {
        let root = std::env::temp_dir().join(format!(
            "xarchive-file-store-v2-conflict-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let files = FileStore::new(&root).expect("file store");
        let attempt = files
            .create_attempt_staging_dir("job-v2", 1)
            .expect("attempt staging");
        fs::write(attempt.join("source.jpg"), b"media").expect("source");
        fs::write(attempt.join("temporary.jpg"), b"media").expect("conflicting destination");
        let digest = FileStore::sha256(attempt.join("source.jpg")).expect("hash");
        let identity = crate::ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: "temporary.jpg".into(),
            final_path: "final.jpg".into(),
            size_bytes: 5,
            sha256: digest,
        };

        assert!(
            files
                .recover_attempt_rename("job-v2", 1, &identity, true)
                .is_err()
        );
        assert!(
            files
                .recover_attempt_rename(
                    "job-v2",
                    1,
                    &crate::ArchiveRenamePlan {
                        source_path: "missing.jpg".into(),
                        temporary_path: "missing-temporary.jpg".into(),
                        final_path: "missing-final.jpg".into(),
                        size_bytes: 5,
                        sha256: "a".repeat(64),
                    },
                    true
                )
                .is_err()
        );
        assert_eq!(
            fs::read(attempt.join("source.jpg")).expect("source remains"),
            b"media"
        );
        assert_eq!(
            fs::read(attempt.join("temporary.jpg")).expect("destination remains"),
            b"media"
        );
        let _ = fs::remove_dir_all(root);
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

    #[test]
    fn attempt_staging_isolates_concurrent_attempts() {
        let root = temp_root("attempt-isolation");
        let store = FileStore::new(&root).expect("file store");

        let first = store
            .create_attempt_staging_dir("job-1", 1)
            .expect("attempt 1 staging");
        let second = store
            .create_attempt_staging_dir("job-1", 2)
            .expect("attempt 2 staging");
        assert_ne!(first, second);
        fs::write(first.join("stale.bin"), b"stale").expect("stale payload");
        assert!(!second.join("stale.bin").exists());
        assert!(store.existing_attempt_staging_dir("job-1", 1).is_ok());
        assert!(store.existing_attempt_staging_dir("job-1", 3).is_err());
        assert!(store.create_attempt_staging_dir("job-1", 0).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fresh_attempt_staging_creation_never_reuses_an_existing_attempt() {
        let root = temp_root("fresh-attempt");
        let store = FileStore::new(&root).expect("file store");
        let first = store
            .create_attempt_staging_dir("job-1", 1)
            .expect("first attempt directory");
        fs::write(first.join("partial.bin"), b"partial").expect("partial output");

        let duplicate = store.create_attempt_staging_dir("job-1", 1);
        assert!(matches!(
            duplicate,
            Err(StorageError::Io(ref error)) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(
            fs::read(first.join("partial.bin")).expect("original partial output remains"),
            b"partial"
        );

        let retry = store
            .create_attempt_staging_dir("job-1", 2)
            .expect("new durable attempt gets a fresh directory");
        assert_ne!(first, retry);
        assert!(!retry.join("partial.bin").exists());
        assert!(store.create_attempt_staging_dir("job-1", 0).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn fresh_attempt_staging_rejects_a_symlinked_job_parent() {
        use std::os::unix::fs::symlink;

        let root = temp_root("attempt-parent-symlink");
        let store = FileStore::new(&root).expect("file store");
        let outside = temp_root("attempt-parent-outside");
        fs::create_dir_all(&outside).expect("outside directory");
        symlink(&outside, root.join("_staging/job-linked")).expect("job parent symlink");

        assert!(matches!(
            store.create_attempt_staging_dir("job-linked", 1),
            Err(StorageError::InvalidPath)
        ));
        assert!(!outside.join("attempt-1").exists());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[test]
    fn fresh_attempt_staging_rejects_a_symlinked_staging_root() {
        use std::os::unix::fs::symlink;

        let root = temp_root("attempt-root-symlink");
        let archive = root.join("archive");
        let staging_link = root.join("staging-link");
        let outside = temp_root("attempt-root-outside");
        fs::create_dir_all(&archive).expect("archive directory");
        fs::create_dir_all(&outside).expect("outside directory");
        fs::create_dir(&staging_link).expect("initial staging root");
        let store = FileStore::with_staging_root(&archive, &staging_link).expect("file store");
        fs::remove_dir(&staging_link).expect("remove initial staging root");
        symlink(&outside, &staging_link).expect("staging root symlink");

        assert!(matches!(
            store.create_attempt_staging_dir("job-1", 1),
            Err(StorageError::InvalidPath)
        ));
        assert!(!outside.join("job-1/attempt-1").exists());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn attempt_file_creation_never_overwrites_existing_files_or_links() {
        let root = temp_root("attempt-create-only");
        let store = FileStore::new(&root).expect("file store");
        let staging = store
            .create_attempt_staging_dir("job-1", 1)
            .expect("attempt");
        let path = store
            .create_attempt_file("job-1", 1, "tweet.json", br#"{"safe":true}"#)
            .expect("first create");
        assert_eq!(fs::read(&path).expect("created bytes"), br#"{"safe":true}"#);
        assert!(
            store
                .create_attempt_file("job-1", 1, "tweet.json", b"overwrite")
                .is_err()
        );
        assert_eq!(
            fs::read(&path).expect("original bytes retained"),
            br#"{"safe":true}"#
        );

        #[cfg(unix)]
        {
            let outside = temp_root("attempt-create-outside");
            fs::create_dir_all(&outside).expect("outside");
            let target = outside.join("target.json");
            fs::write(&target, b"untouched").expect("target");
            std::os::unix::fs::symlink(&target, staging.join("linked.json")).expect("symlink");
            assert!(
                store
                    .create_attempt_file("job-1", 1, "linked.json", b"attack")
                    .is_err()
            );
            assert_eq!(fs::read(&target).expect("target retained"), b"untouched");
            let _ = fs::remove_dir_all(outside);
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn replay_attempt_stage_moves_pending_edges_atomically() {
        let root = temp_root("attempt-replay");
        let store = FileStore::new(&root).expect("file store");
        let staging = store
            .create_attempt_staging_dir("job-2", 1)
            .expect("attempt staging");
        fs::write(staging.join("source.bin"), b"payload").expect("source");
        let hash = FileStore::sha256(staging.join("source.bin")).expect("source hash");
        let identity = crate::ArchiveRenamePlan {
            source_path: "source.bin".into(),
            temporary_path: "moved.bin".into(),
            final_path: "final.bin".into(),
            size_bytes: 7,
            sha256: hash,
        };
        let edge = crate::workflow_bridge::RenameEdge::new(
            &identity.source_path,
            &identity.temporary_path,
            &identity.final_path,
        )
        .expect("valid edge");
        store
            .replay_attempt_rename_stage(
                "job-2",
                1,
                std::slice::from_ref(&edge),
                std::slice::from_ref(&identity),
                true,
            )
            .expect("pending move");
        assert!(staging.join("moved.bin").is_file());
        assert_eq!(
            fs::read(staging.join("moved.bin")).expect("moved intact"),
            b"payload"
        );
        store
            .replay_attempt_rename_stage(
                "job-2",
                1,
                std::slice::from_ref(&edge),
                std::slice::from_ref(&identity),
                true,
            )
            .expect("completed move replays by verified identity");
        let conflicting = crate::ArchiveRenamePlan {
            sha256: "0".repeat(64),
            ..identity.clone()
        };
        let conflict_edge = crate::workflow_bridge::RenameEdge::new(
            &conflicting.source_path,
            &conflicting.temporary_path,
            &conflicting.final_path,
        )
        .expect("valid conflicting edge");
        assert!(
            store
                .replay_attempt_rename_stage("job-2", 1, &[conflict_edge], &[conflicting], true)
                .is_err()
        );
        let absent_identity = crate::ArchiveRenamePlan {
            source_path: "absent.bin".into(),
            temporary_path: "temporary.bin".into(),
            final_path: "final.bin".into(),
            size_bytes: 1,
            sha256: "0".repeat(64),
        };
        let absent_edge = crate::workflow_bridge::RenameEdge::new(
            &absent_identity.source_path,
            &absent_identity.temporary_path,
            &absent_identity.final_path,
        )
        .expect("valid absent edge");
        assert!(
            store
                .replay_attempt_rename_stage("job-2", 1, &[absent_edge], &[absent_identity], true,)
                .is_err()
        );

        let mismatch = crate::workflow_bridge::RenameEdge::new(
            "other-source.bin",
            &identity.temporary_path,
            &identity.final_path,
        )
        .expect("individually valid edge");
        assert!(
            store
                .replay_attempt_rename_stage(
                    "job-2",
                    1,
                    &[mismatch],
                    std::slice::from_ref(&identity),
                    true,
                )
                .is_err()
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let outside = temp_root("attempt-replay-outside");
            fs::write(outside.join("payload.bin"), b"payload").expect("outside payload");
            let linked_identity = crate::ArchiveRenamePlan {
                source_path: "link.bin".into(),
                temporary_path: "linked-temp.bin".into(),
                final_path: "linked-final.bin".into(),
                size_bytes: 7,
                sha256: identity.sha256.clone(),
            };
            let linked_edge = crate::workflow_bridge::RenameEdge::new(
                &linked_identity.source_path,
                &linked_identity.temporary_path,
                &linked_identity.final_path,
            )
            .expect("valid symlink edge");
            symlink(outside.join("payload.bin"), staging.join("link.bin"))
                .expect("create source symlink");
            assert!(
                store
                    .replay_attempt_rename_stage(
                        "job-2",
                        1,
                        &[linked_edge],
                        &[linked_identity],
                        true,
                    )
                    .is_err()
            );
            let _ = fs::remove_dir_all(outside);
        }
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
    fn existing_attempt_staging_rejects_symlinked_ancestors() {
        use std::os::unix::fs::symlink;

        let root = temp_root("existing-attempt-ancestors");
        let archive = root.join("archive");
        let staging = root.join("staging");
        let outside = root.join("outside");
        fs::create_dir_all(&archive).expect("archive");
        fs::create_dir_all(&outside).expect("outside");

        // First reject replacement of the staging root itself.
        let store = FileStore::with_staging_root(&archive, &staging).expect("store");
        let attempt = store
            .create_attempt_staging_dir("job-1", 1)
            .expect("attempt");
        fs::write(attempt.join("media.jpg"), b"media").expect("media");
        fs::remove_dir_all(&staging).expect("remove staging root");
        symlink(&outside, &staging).expect("staging root symlink");
        assert!(matches!(
            store.existing_attempt_staging_dir("job-1", 1),
            Err(StorageError::InvalidPath)
        ));
        fs::remove_file(&staging).expect("remove staging symlink");

        // Then reject a replaced job ancestor even when the attempt directory
        // exists outside under the same textual path.
        fs::create_dir(&staging).expect("restore staging root");
        let outside_job = outside.join("job-1");
        fs::create_dir(&outside_job).expect("outside job");
        fs::create_dir(outside_job.join("attempt-1")).expect("outside attempt");
        symlink(&outside_job, staging.join("job-1")).expect("job symlink");
        assert!(matches!(
            store.existing_attempt_staging_dir("job-1", 1),
            Err(StorageError::InvalidPath)
        ));
        let _ = fs::remove_dir_all(root);
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
