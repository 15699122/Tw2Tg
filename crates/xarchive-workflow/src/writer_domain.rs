//! Attempt-bound writer authorization and its Linux filesystem adapter.
//!
//! This boundary binds paths to one attempt staging root. It is not an OS
//! process-tree containment mechanism: callers must not treat a valid path or
//! an adapter operation as proof that a downloader or escaped descendant has
//! exited. Publication remains a separate, fenced workflow.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::AttemptIdentity;
use crate::coordination::Coordinator;
use crate::{ExecutionSession, SessionError};

/// A path binding borrowed together with its live Job/destination session.
///
/// This cannot outlive either value. It is not process containment, and paths
/// obtained from it are not durable authorization tokens for later I/O.
///
/// A bound writer cannot be used after consuming its session:
///
/// ```compile_fail,E0505
/// use xarchive_workflow::{ExecutionSession, WriterDomain};
/// use xarchive_workflow::coordination::Coordinator;
/// fn release_early<C: Coordinator>(domain: &WriterDomain, session: ExecutionSession<'_, C>) {
///     let writer = domain.bind_session(&session).unwrap();
///     drop(session);
///     let _ = writer.resolve_relative("payload");
/// }
/// ```
///
/// Nor can it escape the domain borrow:
///
/// ```compile_fail,E0505
/// use xarchive_workflow::{ExecutionSession, WriterDomain};
/// use xarchive_workflow::coordination::Coordinator;
/// fn discard_domain<C: Coordinator>(domain: WriterDomain, session: &ExecutionSession<'_, C>) {
///     let writer = domain.bind_session(session).unwrap();
///     drop(domain);
///     let _ = writer.resolve_relative("payload");
/// }
/// ```
///
/// The bound capability is deliberately not cloneable:
///
/// ```compile_fail,E0277
/// use xarchive_workflow::SessionWriterDomain;
/// use xarchive_workflow::coordination::Coordinator;
/// fn requires_clone<T: Clone>(_: &T) {}
/// fn duplicate<C: Coordinator>(writer: &SessionWriterDomain<'_, '_, C>) {
///     requires_clone(writer);
/// }
/// ```
///
/// Ordinary operations within both borrows remain supported:
///
/// ```
/// use xarchive_workflow::{ExecutionSession, WriterDomain, WriterDomainError, WriterPath};
/// use xarchive_workflow::coordination::Coordinator;
/// fn resolve<C: Coordinator>(domain: &WriterDomain, session: &ExecutionSession<'_, C>)
///     -> Result<WriterPath, WriterDomainError>
/// {
///     domain.bind_session(session).map_err(|_| WriterDomainError::StaleAttempt)?
///         .resolve_relative("payload")
/// }
/// ```
pub struct SessionWriterDomain<'a, 'coordinator, C: Coordinator> {
    domain: &'a WriterDomain,
    session: &'a ExecutionSession<'coordinator, C>,
}

impl<C: Coordinator> SessionWriterDomain<'_, '_, C> {
    /// Pin the directory currently at this domain's root, without exposing its
    /// descriptor. Reject a device/inode change since adapter observation.
    /// This snapshot fence is not resistant to inode reuse after deletion.
    #[cfg(unix)]
    pub fn pin(&self) -> Result<PinnedSessionWriter<'_, '_, '_, C>, WriterDomainError> {
        self.resolve_relative("pin-check")?;
        let directory = unix::open_root(self.domain.staging_root())?;
        if Some(unix::directory_identity(&directory)?) != self.domain.directory_identity {
            return Err(WriterDomainError::InvalidRoot);
        }
        Ok(PinnedSessionWriter {
            writer: self,
            directory,
        })
    }

    /// Create and sync a new file and its parent while borrowing the live session.
    ///
    /// All directories must already exist. Every component is opened relative
    /// to a retained directory descriptor with NOFOLLOW; no writable handle
    /// escapes this operation. A failed write can leave a partial create-new
    /// file, which must not be silently reused. This does not stop external
    /// processes or guarantee namespace stability against directory renames.
    /// Parent sync failure is an error even if file contents were synced. This
    /// does not sync newly created ancestors or authorize publication.
    #[cfg(unix)]
    pub fn create_new_file(
        &self,
        relative: impl AsRef<Path>,
        contents: &[u8],
    ) -> Result<(), WriterDomainError> {
        // Reject untrusted input before opening the root, including when the
        // root has disappeared or its identity no longer matches.
        self.resolve_relative(relative.as_ref())?;
        self.pin()?.create_new_file(relative, contents)
    }

    pub fn resolve_relative(
        &self,
        relative: impl AsRef<Path>,
    ) -> Result<WriterPath, WriterDomainError> {
        // Keep the live-session check at the operation boundary rather than
        // exposing a cloneable "authorized" domain.
        self.session
            .authorize(self.domain.identity())
            .map_err(|_| WriterDomainError::StaleAttempt)?;
        self.domain.authorize_relative(relative)
    }
}

/// Session-borrowed Unix root descriptor. No Clone or raw handle access.
/// Root rename/replacement cannot redirect writes after pinning. Descendant
/// directories are still resolved per operation and may be renamed/replaced;
/// this is not filesystem isolation, process containment or publication proof.
///
/// The pinned writer cannot survive consuming its borrowed writer:
///
/// ```compile_fail,E0505
/// use xarchive_workflow::SessionWriterDomain;
/// use xarchive_workflow::coordination::Coordinator;
/// fn release<C: Coordinator>(writer: SessionWriterDomain<'_, '_, C>) {
///     let pinned = writer.pin().unwrap();
///     drop(writer);
///     let _ = pinned.create_new_file("payload", b"data");
/// }
/// ```
///
/// Nor does pinning create a cloneable capability:
///
/// ```compile_fail,E0277
/// use xarchive_workflow::writer_domain::PinnedSessionWriter;
/// use xarchive_workflow::coordination::Coordinator;
/// fn needs_clone<T: Clone>(_: &T) {}
/// fn duplicate<C: Coordinator>(pinned: &PinnedSessionWriter<'_, '_, '_, C>) {
///     needs_clone(pinned);
/// }
/// ```
///
/// ```
/// # #[cfg(unix)]
/// # fn example() {
/// use xarchive_workflow::SessionWriterDomain;
/// use xarchive_workflow::coordination::Coordinator;
/// fn write<C: Coordinator>(writer: &SessionWriterDomain<'_, '_, C>) {
///     let pinned = writer.pin().unwrap();
///     pinned.create_new_file("payload", b"data").unwrap();
/// }
/// # }
/// ```
#[cfg(unix)]
pub struct PinnedSessionWriter<'writer, 'domain, 'coordinator, C: Coordinator> {
    writer: &'writer SessionWriterDomain<'domain, 'coordinator, C>,
    directory: std::fs::File,
}

#[cfg(unix)]
impl<C: Coordinator> PinnedSessionWriter<'_, '_, '_, C> {
    pub fn create_new_file(
        &self,
        relative: impl AsRef<Path>,
        contents: &[u8],
    ) -> Result<(), WriterDomainError> {
        self.writer.resolve_relative(relative.as_ref())?;
        unix::write_at_root(&self.directory, relative.as_ref(), contents)
    }
}

/// A validated path under a writer domain's attempt staging root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterPath(PathBuf);

impl WriterPath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Errors from attempt-bound writer path authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterDomainError {
    InvalidRoot,
    InvalidRelativePath,
    StaleAttempt,
    Io,
}

impl fmt::Display for WriterDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io => formatter.write_str("attempt file creation or synchronization failed"),
            Self::StaleAttempt => formatter.write_str("writer domain belongs to another attempt"),
            Self::InvalidRoot => formatter.write_str("invalid writer-domain staging root"),
            Self::InvalidRelativePath => {
                formatter.write_str("writer path must be a safe relative path")
            }
        }
    }
}

impl std::error::Error for WriterDomainError {}

/// Path binding for exactly one attempt, not a live session authorization.
///
/// Adapter constructors accept an identity, not an ExecutionSession. This value
/// therefore proves neither scope ownership nor permission to write or publish.
/// Filesystem checks are snapshots and do not prevent concurrent link swaps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterDomain {
    identity: AttemptIdentity,
    staging_root: PathBuf,
    #[cfg(unix)]
    directory_identity: Option<(u64, u64)>,
}

impl WriterDomain {
    /// Bind path resolution to the held session, rejecting foreign or stale
    /// attempts. This does not extend the session's lifetime or transfer it.
    pub fn bind_session<'a, 'coordinator, C: Coordinator>(
        &'a self,
        session: &'a ExecutionSession<'coordinator, C>,
    ) -> Result<SessionWriterDomain<'a, 'coordinator, C>, SessionError> {
        session.authorize(&self.identity)?;
        Ok(SessionWriterDomain {
            domain: self,
            session,
        })
    }
    pub fn identity(&self) -> &AttemptIdentity {
        &self.identity
    }

    pub fn staging_root(&self) -> &Path {
        &self.staging_root
    }

    /// Resolve a downloader-reported relative path beneath this attempt.
    /// Absolute paths, parent traversal, prefixes, and empty paths are refused.
    /// This is lexical resolution only; links and concurrent replacement require
    /// descriptor-relative filesystem operations at the eventual I/O boundary.
    pub fn authorize_relative(
        &self,
        relative: impl AsRef<Path>,
    ) -> Result<WriterPath, WriterDomainError> {
        let relative = relative.as_ref();
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(WriterDomainError::InvalidRelativePath);
        }

        let path = self.staging_root.join(relative);
        if !path.starts_with(&self.staging_root) {
            return Err(WriterDomainError::InvalidRelativePath);
        }
        Ok(WriterPath(path))
    }
}

/// Linux/Unix adapter for creating a fresh attempt writer domain.
#[cfg(unix)]
pub mod unix {
    use std::fs;
    use std::io;
    use std::path::{Component, Path, PathBuf};

    use super::{WriterDomain, WriterDomainError};
    use crate::AttemptIdentity;
    use crate::coordination::LockName;

    pub(super) fn directory_identity(file: &fs::File) -> Result<(u64, u64), WriterDomainError> {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata().map_err(|_| WriterDomainError::Io)?;
        Ok((metadata.dev(), metadata.ino()))
    }

    #[cfg(test)]
    pub(super) fn write_new_file(
        root: &Path,
        relative: &Path,
        contents: &[u8],
    ) -> Result<(), WriterDomainError> {
        write_new_file_with_sync(root, relative, contents, fs::File::sync_all)
    }

    // Private seam for deterministic sync-failure tests; production always
    // uses File::sync_all and callers cannot weaken the durability operation.
    #[cfg(test)]
    fn write_new_file_with_sync(
        root: &Path,
        relative: &Path,
        contents: &[u8],
        mut sync: impl FnMut(&fs::File) -> io::Result<()>,
    ) -> Result<(), WriterDomainError> {
        let directory = open_root(root)?;
        write_at_root_with_sync(&directory, relative, contents, &mut sync)
    }

    pub(super) fn open_root(root: &Path) -> Result<fs::File, WriterDomainError> {
        use rustix::fs::{Mode, OFlags, open, openat};
        let directory_flags =
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        if !root.is_absolute() {
            return Err(WriterDomainError::InvalidRoot);
        }
        let mut directory =
            open("/", directory_flags, Mode::empty()).map_err(|_| WriterDomainError::Io)?;
        // Opening the whole absolute root at once would follow ancestor links.
        for component in root.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => {
                    directory = openat(&directory, name, directory_flags, Mode::empty())
                        .map_err(|_| WriterDomainError::Io)?;
                }
                _ => return Err(WriterDomainError::InvalidRoot),
            }
        }
        Ok(fs::File::from(directory))
    }

    pub(super) fn write_at_root(
        directory: &fs::File,
        relative: &Path,
        contents: &[u8],
    ) -> Result<(), WriterDomainError> {
        write_at_root_with_sync(directory, relative, contents, fs::File::sync_all)
    }

    fn write_at_root_with_sync(
        directory: &fs::File,
        relative: &Path,
        contents: &[u8],
        mut sync: impl FnMut(&fs::File) -> io::Result<()>,
    ) -> Result<(), WriterDomainError> {
        use rustix::fs::{Mode, OFlags, openat};
        use std::io::Write;
        let directory_flags =
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut directory = directory.try_clone().map_err(|_| WriterDomainError::Io)?;
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(WriterDomainError::InvalidRelativePath);
            };
            if components.peek().is_some() {
                directory = fs::File::from(
                    openat(&directory, name, directory_flags, Mode::empty())
                        .map_err(|_| WriterDomainError::Io)?,
                );
            } else {
                let descriptor = openat(
                    &directory,
                    name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(|_| WriterDomainError::Io)?;
                let mut file = fs::File::from(descriptor);
                file.write_all(contents)
                    .map_err(|_| WriterDomainError::Io)?;
                sync(&file).map_err(|_| WriterDomainError::Io)?;
                sync(&directory).map_err(|_| WriterDomainError::Io)?;
                return Ok(());
            }
        }
        Err(WriterDomainError::InvalidRelativePath)
    }

    fn validate_inputs(root: &Path, identity: &AttemptIdentity) -> Result<(), WriterDomainError> {
        if !root.is_absolute()
            || root.components().any(|c| {
                !matches!(
                    c,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
            || LockName::new(identity.job_id()).is_err()
            || identity.job_id().trim() != identity.job_id()
        {
            return Err(WriterDomainError::InvalidRoot);
        }
        Ok(())
    }

    /// Create a fresh `<staging_root>/<job>/attempt-N` directory and bind it
    /// to `identity`. Existing attempt directories are never reused.
    /// Traversed directories and their parents are synced before success,
    /// including ancestors left by a previous failed creation.
    /// Sync failure can leave partial directories; an existing attempt remains
    /// refused on retry. This is not a power-loss or namespace isolation proof.
    pub fn create_attempt_domain(
        staging_root: impl AsRef<Path>,
        identity: AttemptIdentity,
    ) -> Result<WriterDomain, WriterDomainError> {
        create_attempt_domain_with_sync(staging_root.as_ref(), identity, fs::File::sync_all)
    }

    // Private failure-injection seam; the public adapter always syncs for real.
    fn create_attempt_domain_with_sync(
        staging_root: &Path,
        identity: AttemptIdentity,
        mut sync: impl FnMut(&fs::File) -> io::Result<()>,
    ) -> Result<WriterDomain, WriterDomainError> {
        validate_inputs(staging_root, &identity)?;
        use rustix::fs::{Mode, OFlags, mkdirat, open, openat};
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mode = Mode::RUSR | Mode::WUSR | Mode::XUSR;
        let mut directory = fs::File::from(
            open("/", flags, Mode::empty()).map_err(|_| WriterDomainError::InvalidRoot)?,
        );
        // Never resolve a multi-component creation path through the namespace.
        // Each mkdir and subsequent open uses the retained parent descriptor.
        for name in staging_root
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => Some(name),
                _ => None,
            })
            .chain(std::iter::once(std::ffi::OsStr::new(identity.job_id())))
        {
            match mkdirat(&directory, name, mode) {
                Ok(()) => {}
                Err(error) if error == rustix::io::Errno::EXIST => {}
                Err(_) => return Err(WriterDomainError::InvalidRoot),
            }
            let child = fs::File::from(
                openat(&directory, name, flags, Mode::empty())
                    .map_err(|_| WriterDomainError::InvalidRoot)?,
            );
            // EXIST does not prove durability: a previous call may have
            // created this ancestor and failed before syncing its parent.
            sync(&child).map_err(|_| WriterDomainError::Io)?;
            sync(&directory).map_err(|_| WriterDomainError::Io)?;
            directory = child;
        }
        let attempt_name = format!("attempt-{}", identity.attempt());
        mkdirat(&directory, &attempt_name, mode).map_err(|_| WriterDomainError::InvalidRoot)?;
        // Refuse a link replacement before returning the path binding. This
        // does not promise namespace stability after descriptors are dropped.
        let attempt_directory = fs::File::from(
            openat(&directory, &attempt_name, flags, Mode::empty())
                .map_err(|_| WriterDomainError::InvalidRoot)?,
        );
        sync(&attempt_directory).map_err(|_| WriterDomainError::Io)?;
        sync(&directory).map_err(|_| WriterDomainError::Io)?;
        let observed_identity = directory_identity(&attempt_directory)?;
        let attempt_root = staging_root.join(identity.job_id()).join(attempt_name);
        Ok(WriterDomain {
            identity,
            staging_root: attempt_root,
            directory_identity: Some(observed_identity),
        })
    }

    /// Resolve an existing attempt domain for inspection/recovery without
    /// creating a missing directory or weakening the fresh-creation rule.
    /// Descriptors protect traversal only; the returned path is not pinned
    /// against subsequent namespace replacement and grants no recovery permit.
    pub fn existing_attempt_domain(
        staging_root: impl AsRef<Path>,
        identity: AttemptIdentity,
    ) -> Result<WriterDomain, WriterDomainError> {
        validate_inputs(staging_root.as_ref(), &identity)?;
        let attempt_root: PathBuf = staging_root
            .as_ref()
            .join(identity.job_id())
            .join(format!("attempt-{}", identity.attempt()));
        use rustix::fs::{Mode, OFlags, open, openat};
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut directory =
            open("/", flags, Mode::empty()).map_err(|_| WriterDomainError::InvalidRoot)?;
        for component in attempt_root.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => {
                    directory = openat(&directory, name, flags, Mode::empty())
                        .map_err(|_| WriterDomainError::InvalidRoot)?;
                }
                _ => return Err(WriterDomainError::InvalidRoot),
            }
        }
        Ok(WriterDomain {
            identity,
            staging_root: attempt_root,
            directory_identity: Some(directory_identity(&fs::File::from(directory))?),
        })
    }
    #[cfg(test)]
    mod sync_tests {
        use super::*;

        #[test]
        fn directory_sync_failures_stop_creation_and_refuse_attempt_reuse() {
            // Two syncs each for staging, job, and attempt.
            for fail_at in 1..=6 {
                let root = super::super::tests::temp_root(&format!("directory-sync-{fail_at}"));
                let _ = fs::remove_dir_all(&root);
                fs::create_dir_all(&root).expect("fixture parent");
                let staging = root.join("staging");
                let identity = AttemptIdentity::new("job", 1).expect("identity");
                let mut expected = Vec::new();
                let mut ancestor = PathBuf::from("/");
                for component in root.components() {
                    if let Component::Normal(name) = component {
                        let parent = ancestor.clone();
                        ancestor.push(name);
                        expected.extend([ancestor.clone(), parent]);
                    }
                }
                let prefix = expected.len();
                expected.extend([
                    staging.clone(),
                    root.clone(),
                    staging.join("job"),
                    staging.clone(),
                    staging.join("job/attempt-1"),
                    staging.join("job"),
                ]);
                let mut calls = 0;
                let result = create_attempt_domain_with_sync(&staging, identity.clone(), |file| {
                    assert_eq!(
                        directory_identity(file).unwrap(),
                        directory_identity(&fs::File::open(&expected[calls])?).unwrap()
                    );
                    calls += 1;
                    if calls == prefix + fail_at {
                        Err(io::Error::other("injected directory sync failure"))
                    } else {
                        file.sync_all()
                    }
                });
                assert!(matches!(result, Err(WriterDomainError::Io)));
                assert_eq!(calls, prefix + fail_at, "stop at the first error");
                assert!(staging.is_dir());
                assert_eq!(staging.join("job").exists(), fail_at >= 3);
                assert_eq!(staging.join("job/attempt-1").exists(), fail_at >= 5);
                let mut retry_calls = 0;
                let retry = create_attempt_domain_with_sync(&staging, identity, |file| {
                    assert_eq!(
                        directory_identity(file).unwrap(),
                        directory_identity(&fs::File::open(&expected[retry_calls])?).unwrap()
                    );
                    retry_calls += 1;
                    file.sync_all()
                });
                if fail_at >= 5 {
                    assert!(matches!(retry, Err(WriterDomainError::InvalidRoot)));
                    assert_eq!(retry_calls, prefix + 4);
                } else {
                    retry.expect("partial ancestors permit fresh attempt retry");
                    assert_eq!(retry_calls, expected.len());
                }
                fs::remove_dir_all(root).expect("cleanup");
            }
        }

        #[test]
        fn sync_failures_refuse_success_and_preserve_create_new_fence() {
            for fail_at in [1, 2] {
                let root = super::super::tests::temp_root(&format!("sync-failure-{fail_at}"));
                let _ = fs::remove_dir_all(&root);
                fs::create_dir_all(&root).expect("root");
                let mut calls = 0;
                let result =
                    write_new_file_with_sync(&root, Path::new("payload"), b"original", |file| {
                        calls += 1;
                        assert_eq!(file.metadata()?.is_dir(), calls == 2);
                        if calls == fail_at {
                            Err(io::Error::other("injected sync failure"))
                        } else {
                            file.sync_all()
                        }
                    });
                assert_eq!(result, Err(WriterDomainError::Io));
                assert_eq!(calls, fail_at, "stop at first sync error");
                assert_eq!(fs::read(root.join("payload")).unwrap(), b"original");
                assert_eq!(
                    write_new_file(&root, Path::new("payload"), b"replacement"),
                    Err(WriterDomainError::Io)
                );
                assert_eq!(fs::read(root.join("payload")).unwrap(), b"original");
                fs::remove_dir_all(root).expect("cleanup");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_binding_rejects_stale_and_foreign_attempts() {
        use crate::coordination::{CoordinationGuard, LockError, LockScope};
        use crate::{ClaimOutcome, ExecutionReservation};
        struct Probe;
        struct Guard;
        impl CoordinationGuard for Guard {
            fn release(self) -> Result<(), LockError> {
                Ok(())
            }
        }
        impl Coordinator for Probe {
            type Guard = Guard;
            fn acquire(&self, _: &LockScope) -> Result<Guard, LockError> {
                Ok(Guard)
            }
            fn acquire_all(&self, _: &[LockScope]) -> Result<Guard, LockError> {
                Ok(Guard)
            }
        }
        let coordinator = Probe;
        let identity = AttemptIdentity::new("job", 2).expect("identity");
        let session = ExecutionReservation::acquire(&coordinator, "job", Path::new("final"))
            .expect("reservation")
            .claim(|| ClaimOutcome::Claimed(identity.clone()))
            .expect("session");
        let current = WriterDomain {
            identity,
            staging_root: PathBuf::from("staging/job/attempt-2"),
            #[cfg(unix)]
            directory_identity: None,
        };
        let bound = current.bind_session(&session).expect("matching session");
        #[cfg(unix)]
        {
            let root = temp_root("session-write");
            let _ = std::fs::remove_dir_all(&root);
            let domain = unix::create_attempt_domain(&root, session.attempt().clone())
                .expect("fresh domain");
            let writer = domain.bind_session(&session).expect("held session");
            writer
                .create_new_file("metadata.json", b"{}")
                .expect("write");
            assert_eq!(
                std::fs::read(domain.staging_root().join("metadata.json")).unwrap(),
                b"{}"
            );
            assert_eq!(
                writer.create_new_file("metadata.json", b"bad"),
                Err(WriterDomainError::Io)
            );
            assert_eq!(
                writer.create_new_file("../escape", b"bad"),
                Err(WriterDomainError::InvalidRelativePath)
            );
            std::fs::create_dir(domain.staging_root().join("nested")).expect("nested directory");
            writer
                .create_new_file("nested/empty", b"")
                .expect("sync empty file and nested parent");
            assert_eq!(
                std::fs::read(domain.staging_root().join("nested/empty")).unwrap(),
                b""
            );
            writer
                .create_new_file("nested/data", b"nested payload")
                .expect("sync nested payload and parent");
            assert_eq!(
                std::fs::read(domain.staging_root().join("nested/data")).unwrap(),
                b"nested payload"
            );
            let pinned = writer.pin().expect("pin held root");
            let moved = root.join("moved-attempt");
            std::fs::rename(domain.staging_root(), &moved).expect("move root");
            std::fs::create_dir(domain.staging_root()).expect("replacement root");
            assert!(matches!(writer.pin(), Err(WriterDomainError::InvalidRoot)));
            for invalid in ["../escape", "/absolute", ""] {
                assert_eq!(
                    writer.create_new_file(invalid, b"bad"),
                    Err(WriterDomainError::InvalidRelativePath)
                );
            }
            assert_eq!(
                writer.create_new_file("replacement-write", b"bad"),
                Err(WriterDomainError::InvalidRoot)
            );
            assert!(!domain.staging_root().join("replacement-write").exists());
            pinned
                .create_new_file("pinned", b"original root")
                .expect("pinned write");
            assert_eq!(
                std::fs::read(moved.join("pinned")).unwrap(),
                b"original root"
            );
            assert!(!domain.staging_root().join("pinned").exists());
            assert_eq!(
                pinned.create_new_file("../escape", b"bad"),
                Err(WriterDomainError::InvalidRelativePath)
            );
            assert_eq!(
                pinned.create_new_file("pinned", b"bad"),
                Err(WriterDomainError::Io)
            );
            let outside = root.join("outside");
            std::fs::create_dir(&outside).unwrap();
            std::os::unix::fs::symlink(&outside, moved.join("escape")).unwrap();
            assert_eq!(
                pinned.create_new_file("escape/payload", b"bad"),
                Err(WriterDomainError::Io)
            );
            std::os::unix::fs::symlink(outside.join("payload"), moved.join("linked")).unwrap();
            assert_eq!(
                pinned.create_new_file("linked", b"bad"),
                Err(WriterDomainError::Io)
            );
            assert!(!outside.join("payload").exists());
            assert_eq!(
                pinned.create_new_file("missing/payload", b"bad"),
                Err(WriterDomainError::Io)
            );
            assert!(!moved.join("missing").exists());
            drop(pinned);
            std::fs::remove_dir_all(root).expect("cleanup");
        }
        assert_eq!(
            bound.resolve_relative("01.jpg").expect("path").as_path(),
            Path::new("staging/job/attempt-2/01.jpg")
        );
        assert_eq!(
            bound.resolve_relative("../escape"),
            Err(WriterDomainError::InvalidRelativePath)
        );
        for identity in [
            AttemptIdentity::new("job", 1).unwrap(),
            AttemptIdentity::new("foreign", 2).unwrap(),
        ] {
            let domain = WriterDomain {
                identity,
                staging_root: PathBuf::from("staging"),
                #[cfg(unix)]
                directory_identity: None,
            };
            assert!(matches!(
                domain.bind_session(&session),
                Err(SessionError::StaleAttempt)
            ));
        }
    }

    pub(super) fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "xarchive-writer-domain-{label}-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ))
    }

    #[cfg(unix)]
    #[test]
    fn linux_domain_binds_attempt_and_rejects_reuse() {
        let root = temp_root("create");
        let _ = std::fs::remove_dir_all(&root);
        let identity = AttemptIdentity::new("job-1", 3).expect("identity");
        let domain = unix::create_attempt_domain(&root, identity.clone()).expect("domain");
        assert_eq!(domain.identity(), &identity);
        assert!(domain.staging_root().ends_with("job-1/attempt-3"));
        assert_eq!(
            unix::create_attempt_domain(&root, identity),
            Err(WriterDomainError::InvalidRoot),
            "fresh creation must not reuse an existing attempt"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn authorized_paths_are_confined_and_reject_escape_forms() {
        let domain = WriterDomain {
            identity: AttemptIdentity::new("job-1", 2).expect("identity"),
            staging_root: PathBuf::from("/tmp/staging/job-1/attempt-2"),
            #[cfg(unix)]
            directory_identity: None,
        };
        assert_eq!(
            domain
                .authorize_relative("media/01.jpg")
                .expect("safe path")
                .as_path(),
            Path::new("/tmp/staging/job-1/attempt-2/media/01.jpg")
        );
        for unsafe_path in [
            "",
            "../other/file",
            "media/../../other",
            "/tmp/outside",
            ".",
        ] {
            assert_eq!(
                domain.authorize_relative(unsafe_path),
                Err(WriterDomainError::InvalidRelativePath),
                "accepted unsafe path {unsafe_path:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn failed_fresh_create_can_retry_without_reusing_partial_attempt() {
        let root = temp_root("retry");
        let _ = std::fs::remove_dir_all(&root);
        let identity = AttemptIdentity::new("job-2", 1).expect("identity");
        let job_root = root.join("job-2");
        std::fs::create_dir_all(job_root.join("attempt-1")).expect("preexisting partial");
        assert_eq!(
            unix::create_attempt_domain(&root, identity.clone()),
            Err(WriterDomainError::InvalidRoot)
        );
        std::fs::remove_dir_all(job_root.join("attempt-1")).expect("remove fixture");
        let domain = unix::create_attempt_domain(&root, identity.clone()).expect("retry");
        assert_eq!(domain.identity(), &identity);
        let recovered = unix::existing_attempt_domain(&root, identity).expect("idempotent lookup");
        assert_eq!(recovered, domain);
        let _ = std::fs::remove_dir_all(root);
    }
    #[cfg(unix)]
    #[test]
    fn rejects_unsafe_job_ids_before_creating_any_directory() {
        let root = temp_root("unsafe");
        let _ = std::fs::remove_dir_all(&root);
        for job in [
            "../escape",
            "/absolute",
            "nested/job",
            "back\\slash",
            " spaced ",
        ] {
            let identity = AttemptIdentity::new(job, 1).expect("identity is not a path validator");
            assert_eq!(
                unix::create_attempt_domain(&root, identity.clone()),
                Err(WriterDomainError::InvalidRoot)
            );
            assert_eq!(
                unix::existing_attempt_domain(&root, identity),
                Err(WriterDomainError::InvalidRoot)
            );
            assert!(!root.exists());
        }
    }

    #[cfg(unix)]
    #[test]
    fn recovery_traversal_refuses_links_files_and_missing_attempt_without_creation() {
        use std::os::unix::fs::symlink;
        let root = temp_root("recovery-traversal");
        let _ = std::fs::remove_dir_all(&root);
        let identity = AttemptIdentity::new("job", 4).unwrap();
        let staging = root.join("staging");
        let domain = unix::create_attempt_domain(&staging, identity.clone()).unwrap();
        assert_eq!(
            unix::existing_attempt_domain(&staging, identity.clone()).unwrap(),
            domain
        );
        symlink(&staging, root.join("alias")).unwrap();
        assert_eq!(
            unix::existing_attempt_domain(root.join("alias"), identity.clone()),
            Err(WriterDomainError::InvalidRoot)
        );
        std::fs::remove_dir(domain.staging_root()).unwrap();
        assert_eq!(
            unix::existing_attempt_domain(&staging, identity.clone()),
            Err(WriterDomainError::InvalidRoot)
        );
        assert!(!domain.staging_root().exists());
        std::fs::write(domain.staging_root(), b"not a directory").unwrap();
        assert_eq!(
            unix::existing_attempt_domain(&staging, identity.clone()),
            Err(WriterDomainError::InvalidRoot)
        );
        std::fs::remove_file(domain.staging_root()).unwrap();
        std::fs::create_dir(root.join("outside")).unwrap();
        symlink(root.join("outside"), domain.staging_root()).unwrap();
        assert_eq!(
            unix::existing_attempt_domain(&staging, identity),
            Err(WriterDomainError::InvalidRoot)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn recovery_rejects_linked_job_ancestor_and_missing_payload() {
        use std::os::unix::fs::symlink;
        let root = temp_root("links");
        let _ = std::fs::remove_dir_all(&root);
        let identity = AttemptIdentity::new("job", 1).expect("identity");
        std::fs::create_dir_all(root.join("outside/attempt-1")).expect("fixture");
        symlink(root.join("outside"), root.join("job")).expect("link");
        assert_eq!(
            unix::existing_attempt_domain(&root, identity.clone()),
            Err(WriterDomainError::InvalidRoot)
        );
        assert_eq!(
            unix::create_attempt_domain(&root, identity.clone()),
            Err(WriterDomainError::InvalidRoot)
        );
        std::fs::remove_file(root.join("job")).expect("remove link");
        assert_eq!(
            unix::existing_attempt_domain(&root, identity),
            Err(WriterDomainError::InvalidRoot)
        );
        assert!(!root.join("job").exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn fresh_domain_refuses_linked_staging_ancestor_without_creating_outside() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = temp_root("fresh-ancestor");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("outside")).expect("outside");
        symlink(root.join("outside"), root.join("alias")).expect("ancestor link");
        let identity = AttemptIdentity::new("job", 1).unwrap();
        assert_eq!(
            unix::create_attempt_domain(root.join("alias/staging"), identity.clone()),
            Err(WriterDomainError::InvalidRoot)
        );
        assert!(!root.join("outside/staging").exists());
        let domain = unix::create_attempt_domain(root.join("safe/staging"), identity)
            .expect("nested fresh domain");
        assert_eq!(
            std::fs::metadata(domain.staging_root())
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0,
            "new attempt must not grant group or other permissions"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn descriptor_relative_creation_refuses_links_and_overwrite() {
        use std::os::unix::fs::symlink;
        let root = temp_root("descriptor");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("attempt/media")).expect("directories");
        std::fs::create_dir_all(root.join("outside")).expect("outside");
        let attempt = root.join("attempt");
        unix::write_new_file(&attempt, Path::new("media/file"), b"original").expect("create");
        assert_eq!(
            unix::write_new_file(&attempt, Path::new("media/file"), b"replacement"),
            Err(WriterDomainError::Io)
        );
        assert_eq!(
            std::fs::read(attempt.join("media/file")).unwrap(),
            b"original"
        );
        symlink(root.join("outside"), attempt.join("escape")).expect("directory link");
        assert_eq!(
            unix::write_new_file(&attempt, Path::new("escape/file"), b"bad"),
            Err(WriterDomainError::Io)
        );
        symlink(root.join("outside/file"), attempt.join("linked-file")).expect("file link");
        assert_eq!(
            unix::write_new_file(&attempt, Path::new("linked-file"), b"bad"),
            Err(WriterDomainError::Io)
        );
        symlink(&attempt, root.join("alias")).expect("root link");
        assert_eq!(
            unix::write_new_file(&root.join("alias"), Path::new("new"), b"bad"),
            Err(WriterDomainError::Io)
        );
        assert!(!root.join("outside/file").exists());
        assert!(!attempt.join("new").exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
