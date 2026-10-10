//! Cross-process coordination for one archive data workspace.
//!
//! The confirmed contract allows several application processes on one host to
//! work concurrently on the same workspace: different jobs may proceed in
//! parallel, while operations on the same job, or on the same final
//! destination, must be mutually exclusive.
//!
//! Two design points matter more than the locking call itself.
//!
//! First, the lock must be released by the kernel when its owner dies. A lease
//! with a timeout cannot satisfy this: an owner that is still alive but wedged,
//! or a clock that moved backwards, would produce either a wrongful takeover or
//! A kernel-held advisory lock disappears with the
//! process, so a crashed owner cannot keep a successor out and cannot resume
//! writing after a successor takes over.
//!
//! Second, lock resource names are derived, never accepted. A job id or
//! destination path reaching this module unchecked could address a lock file
//! outside the workspace, so names are canonicalised and length-bounded here
//! before they become part of a filesystem path.

use std::fmt;
use std::path::{Path, PathBuf};

/// Linux/Unix advisory-lock adapter.
///
/// Kernel-enforced advisory locks are released when a process exits, for any
/// reason. A crashed
/// owner therefore cannot keep a successor out and cannot resume writing after a
/// successor takes over, with no lease timeout and no liveness probe.
#[cfg(unix)]
pub mod unix;

/// Windows has no adapter yet.
///
/// Windows file locking is not a portable variant of the Unix mechanism:
/// mandatory locking, share-mode semantics and behaviour across reparse points
/// differ, and the correctness claim depends on NTFS specifics. Rather than
/// silently degrading to an unsynchronised coordinator, the module is absent
/// and callers must not compile a Windows coordination path from these types.
/// The Windows Owner implements and validates the NTFS adapter.
#[cfg(not(unix))]
pub mod unix {
    //! Deliberately empty: see the parent module's Windows note.
}

/// One mutually exclusive resource inside the workspace.
///
/// Job scope serialises execution, recovery and retry for one attempt; two
/// processes must never mutate the same job's files at once. Destination scope
/// serialises the final commit of one archive directory, because two different
/// jobs can otherwise resolve to the same final path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockScope {
    /// Execution, recovery and retry for one job.
    Job(String),
    /// The final commit of one archive destination path.
    Destination(PathBuf),
}

/// Why a lock could not be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockError {
    /// The resource is held by another live owner.
    Contended,
    /// The resource name cannot be used as a lock identity.
    InvalidName,
    /// The lock directory or lock file could not be created or opened.
    Io,
}

impl fmt::Display for LockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contended => {
                formatter.write_str("archive coordination lock is held by another owner")
            }
            Self::InvalidName => formatter.write_str("invalid archive coordination lock name"),
            Self::Io => formatter.write_str("archive coordination lock file could not be opened"),
        }
    }
}

impl std::error::Error for LockError {}

impl From<std::io::Error> for LockError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}

/// Owns one advisory lock record for as long as it is alive.
///
/// Dropping the guard releases the lock explicitly. On a process crash the
/// kernel releases the same record, which is what makes a crashed owner safe to
/// supersede.
pub trait CoordinationGuard {
    /// Release the record now instead of waiting for the drop.
    fn release(self) -> Result<(), LockError>;
}

/// Acquires and releases cross-process advisory lock records.
///
/// The trait exists so the platform adapter is a swappable implementation
/// detail. Callers must treat `acquire` as the only authority on exclusivity:
/// holding a guard is the sole evidence that mutation is permitted, and a guard
/// must never be reconstructed from a cached value.
pub trait Coordinator {
    /// The concrete guard this coordinator hands out.
    type Guard: CoordinationGuard;

    /// Take the lock for one scope without blocking.
    ///
    /// Returns [`LockError::Contended`] when another live owner holds it. The
    /// caller must not retry in a tight loop: contention is a scheduling
    /// decision that belongs to the caller, not to the adapter.
    fn acquire(&self, scope: &LockScope) -> Result<Self::Guard, LockError>;

    /// Acquire several scopes in one canonical order.
    ///
    /// Ordering is fixed by this function rather than by the caller, so two
    /// processes that need the same pair of scopes cannot deadlock by requesting
    /// them in opposite orders.
    fn acquire_all(&self, scopes: &[LockScope]) -> Result<Self::Guard, LockError>;
}

/// Canonical, bounded form of a lock resource name.
///
/// A name is accepted only if it is short, free of path separators and control
/// characters, and not a reserved relative component. Everything else is
/// rejected instead of sanitised, because a silently rewritten name could
/// address a different resource than the caller intended.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LockName(String);

/// Longest accepted resource name; keeps lock paths well inside filesystem
/// limits even with a nested lock directory.
pub const MAX_LOCK_NAME_LEN: usize = 128;

impl LockName {
    /// Validate and canonicalise one resource name.
    pub fn new(raw: &str) -> Result<Self, LockError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_LOCK_NAME_LEN {
            return Err(LockError::InvalidName);
        }
        if trimmed.contains('\0')
            || trimmed.chars().any(char::is_control)
            || matches!(trimmed, "." | "..")
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains(':')
        {
            return Err(LockError::InvalidName);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LockName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Derives the stable lock file path for one scope.
///
/// The name is a digest of the scope rather than the scope itself: a
/// destination path contains separators that cannot appear in a single file
/// name, and a fixed-length digest keeps unrelated scopes from colliding.
pub fn lock_file_path(lock_directory: &Path, scope: &LockScope) -> Result<PathBuf, LockError> {
    let name = match scope {
        LockScope::Job(job_id) => LockName::new(job_id)?.as_str().to_owned(),
        LockScope::Destination(destination) => digest_name(destination)?,
    };
    Ok(lock_directory.join(format!("{name}.lock")))
}

/// A short, filename-safe digest for names that cannot be used verbatim.
fn digest_name(raw: impl AsRef<Path>) -> Result<String, LockError> {
    let text = raw.as_ref().to_string_lossy();
    if text.trim().is_empty() {
        return Err(LockError::InvalidName);
    }
    // FNV-1a: enough to separate real destination paths, with no dependency.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Ok(format!("d{hash:016x}"))
}

/// One filesystem move that must never replace an existing destination.
///
/// POSIX `rename` silently replaces; the C1 contract needs the opposite.
/// The platform adapter either performs an atomic no-replace move or
/// reports [`MoveError::Unsupported`] so the caller fails closed instead
/// of silently degrading to a replacing rename.
pub trait NoReplaceMover {
    /// Atomically move `source` to `destination`, failing when the
    /// destination already exists.
    fn move_no_replace(
        &self,
        source: &std::path::Path,
        destination: &std::path::Path,
    ) -> Result<(), MoveError>;
}

/// Why an atomic no-replace move failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// The destination already exists; nothing was moved.
    DestinationExists,
    /// The source is missing or not a regular file; nothing was moved.
    InvalidSource,
    /// The platform or filesystem cannot provide atomic no-replace moves.
    Unsupported,
    /// An I/O failure other than the cases above.
    Io,
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DestinationExists => formatter.write_str("move destination already exists"),
            Self::InvalidSource => formatter.write_str("move source is missing or invalid"),
            Self::Unsupported => {
                formatter.write_str("atomic no-replace move is not supported here")
            }
            Self::Io => formatter.write_str("atomic move failed"),
        }
    }
}

impl std::error::Error for MoveError {}

/// One validated edge of a two-phase rename plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenameEdge {
    source: std::path::PathBuf,
    temporary: std::path::PathBuf,
    final_path: std::path::PathBuf,
}

impl RenameEdge {
    /// Validate one edge: all three roles are relative, non-empty and
    /// pairwise distinct, and none escapes its staging root.
    pub fn new(source: &str, temporary: &str, final_path: &str) -> Result<Self, LockError> {
        for role in [source, temporary, final_path] {
            if role.is_empty()
                || role.len() > 512
                || role.contains('\0')
                || role.starts_with('/')
                || role.starts_with('\\')
            {
                return Err(LockError::InvalidName);
            }
            let path = std::path::Path::new(role);
            if path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                return Err(LockError::InvalidName);
            }
        }
        if source == temporary || temporary == final_path || source == final_path {
            return Err(LockError::InvalidName);
        }
        Ok(Self {
            source: std::path::PathBuf::from(source),
            temporary: std::path::PathBuf::from(temporary),
            final_path: std::path::PathBuf::from(final_path),
        })
    }

    /// Whole-phase order: every source moves to temporary first.
    pub fn temporary_edge(&self) -> (&std::path::Path, &std::path::Path) {
        (&self.source, &self.temporary)
    }

    /// The immutable source path in this rename plan.
    pub fn source_path(&self) -> &std::path::Path {
        &self.source
    }

    /// The immutable temporary path in this rename plan.
    pub fn temporary_path(&self) -> &std::path::Path {
        &self.temporary
    }

    /// The immutable final path in this rename plan.
    pub fn final_path(&self) -> &std::path::Path {
        &self.final_path
    }

    /// Whole-phase order: every temporary moves to final second.
    pub fn final_edge(&self) -> (&std::path::Path, &std::path::Path) {
        (&self.temporary, &self.final_path)
    }
}

/// Replay one whole rename stage from actual on-disk state.
///
/// For each edge the mover runs only when the source exists and the
/// destination does not. An already-completed edge (destination present)
/// is an idempotent success. This low-level helper establishes only path
/// state; Storage replay must additionally verify each file's recorded size
/// and digest before accepting an edge as complete.
pub fn replay_rename_stage<M: NoReplaceMover>(
    mover: &M,
    staging_root: &std::path::Path,
    edges: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<(), MoveError> {
    for (source, destination) in edges {
        let source = staging_root.join(source);
        let destination = staging_root.join(destination);
        let source_exists = source.is_file();
        let destination_exists = destination.exists();
        match (source_exists, destination_exists) {
            (true, false) => mover.move_no_replace(&source, &destination)?,
            (false, true) => {}
            (true, true) => return Err(MoveError::DestinationExists),
            (false, false) => return Err(MoveError::InvalidSource),
        }
    }
    Ok(())
}
