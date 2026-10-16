//! Linux trusted-tool cgroup owner. Not an adversarial same-user sandbox.
//!
//! The owner holds the exclusively created group's directory descriptor through
//! explicit cleanup, so admission control and descriptor-relative kill/drain
//! stay inside one boundary the caller cannot interleave on its own schedule.
//! The owner retains and reaps its launcher before final drain and supports
//! explicit removal after drain.
//!
//! Crash-orphan recovery follows a record-before-mkdir protocol: a successor
//! derives the deterministic group name from the attempt identity, verifies
//! the containment record that was atomically persisted before the group
//! existed, attaches the leftover group with admission permanently closed,
//! and drains and removes it before successor admission. Production wiring of
//! that protocol (R4b) is not part of this adapter.
//! This is a Linux-only adapter behind the platform-neutral
//! [`crate::writer_lifecycle`] contract; that contract, not this file, owns the
//! shared lifecycle meaning.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

/// Only the kernel-owned hierarchy may host a dedicated writer group.
const CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// Holds the exclusively created group's descriptor through explicit cleanup.
///
/// `admission_closed` permanently rejects further launches once draining
/// begins; `launch_issued` enforces the single-start boundary so a completed or
/// partially failed start cannot be silently repeated.
pub struct LinuxContainment {
    directory: File,
    path: PathBuf,
    admission_closed: bool,
    launch_issued: bool,
    child: Option<Child>,
    parent: File,
    name: String,
    drained: bool,
    removed: bool,
}

/// Stable identity of one attempt's writer domain.
///
/// The workspace must be the caller's stable absolute staging-root path. It
/// is never canonicalized or resolved here, so creation and recovery must
/// pass the identical spelling: the derived group name and record file bind
/// to that spelling, the job id, the attempt number and the optional writer
/// role only — never to a PID, a timestamp or a fixed cgroup name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttemptIdentity {
    workspace: PathBuf,
    job_id: String,
    attempt: u64,
    writer: String,
}

impl AttemptIdentity {
    pub fn new(
        workspace: impl Into<PathBuf>,
        job_id: impl Into<String>,
        attempt: u64,
    ) -> io::Result<Self> {
        let workspace = workspace.into();
        let job_id = job_id.into();
        let workspace_text = workspace
            .to_str()
            .ok_or_else(|| io::Error::other("workspace path is not valid UTF-8"))?;
        if !workspace.is_absolute()
            || job_id.is_empty()
            || workspace_text.contains(['\n', '\0'])
            || job_id.contains(['\n', '\0'])
        {
            return Err(io::Error::other("invalid attempt identity"));
        }
        Ok(Self {
            workspace,
            job_id,
            attempt,
            writer: String::new(),
        })
    }

    /// Bind this identity to one writer role inside the attempt.
    ///
    /// Each writer role gets its own recorded domain so draining one role
    /// never kills another role's writers. Production uses the fixed role
    /// set `sidecar`, `aria2-0` and `aria2-1`, which successor recovery
    /// enumerates exactly.
    pub fn with_writer(self, writer: impl Into<String>) -> io::Result<Self> {
        let writer = writer.into();
        if writer.is_empty() || writer.contains(['\n', '\0']) {
            return Err(io::Error::other("invalid writer role"));
        }
        Ok(Self { writer, ..self })
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn attempt(&self) -> u64 {
        self.attempt
    }

    /// Writer role bound to this identity; empty when unbound.
    pub fn writer(&self) -> &str {
        &self.writer
    }

    /// Deterministic group name for this identity.
    ///
    /// Length-prefixed FNV-1a 64 over workspace, job id, attempt and writer
    /// role keeps the name unique per identity while staying reproducible by
    /// a successor that knows only the persisted workspace state.
    pub fn group_name(&self) -> String {
        let workspace = self
            .workspace
            .to_str()
            .expect("workspace UTF-8 is validated by AttemptIdentity::new");
        let mut bytes = Vec::with_capacity(workspace.len() + self.job_id.len() + 16);
        bytes.extend_from_slice(&(workspace.len() as u64).to_le_bytes());
        bytes.extend_from_slice(workspace.as_bytes());
        bytes.extend_from_slice(&(self.job_id.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.job_id.as_bytes());
        bytes.extend_from_slice(&self.attempt.to_le_bytes());
        bytes.extend_from_slice(&(self.writer.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.writer.as_bytes());
        format!("tw2tg-att-{hash:016x}", hash = fnv1a64(&bytes))
    }

    /// Stable record path for this identity under `record_root`.
    pub fn record_path(&self, record_root: &Path) -> PathBuf {
        record_root.join(format!("{}.record", self.group_name()))
    }
}

/// Result of successor-side recovery of one attempt's writer domain.
///
/// Each returned variant proves that no live descendant of the recorded
/// writer domain remains; any error instead must reject successor admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryOutcome {
    /// No record exists, so the record-before-mkdir protocol never created a
    /// group for this identity.
    NoRecord,
    /// The recorded group no longer exists. The kernel removes a cgroup only
    /// after it empties, so nothing from that domain survives.
    AlreadyAbsent,
    /// The group was attached, killed, observed drained and removed.
    Drained,
}

/// Validate the dedicated group path before any filesystem access.
fn group_path(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let root = Path::new(CGROUP_ROOT);
    if !parent.is_absolute()
        || parent == root
        || !parent.starts_with(root)
        || parent
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        || name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\0'])
    {
        return Err(io::Error::other("invalid dedicated cgroup path"));
    }
    Ok(parent.join(name))
}

/// Open the cgroup parent chain of `parent` below the kernel-owned root.
fn open_parent_chain(parent: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags, open, openat};
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let root = open(Path::new(CGROUP_ROOT), flags, Mode::empty())?;
    let mut directory = root;
    for component in parent
        .strip_prefix(CGROUP_ROOT)
        .map_err(io::Error::other)?
        .components()
    {
        if let Component::Normal(component) = component {
            directory = openat(&directory, component, flags, Mode::empty())?;
        }
    }
    Ok(File::from(directory))
}

/// Marker line of a persisted containment record.
const RECORD_MAGIC: &str = "tw2tg-containment-record-v2";

/// Upper bound for one persisted containment record.
const RECORD_LIMIT: usize = 16 * 1024;

/// Parsed content of one record file. Every field is verified against the
/// requested [`AttemptIdentity`] before recovery trusts it.
struct ContainmentRecord {
    workspace: String,
    job_id: String,
    writer: String,
    attempt: u64,
    parent: PathBuf,
    name: String,
}

/// FNV-1a 64 used only for deterministic group-name derivation.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Reject field content that cannot round-trip through the line-based record.
fn record_value<'a>(field: &str, value: &'a str) -> io::Result<&'a str> {
    if value.contains(['\n', '\0']) {
        return Err(io::Error::other(format!(
            "invalid containment record {field} value"
        )));
    }
    Ok(value)
}

/// fsync the temporary record, rename it into place, then fsync the root.
fn publish_record(
    file: &mut File,
    body: &[u8],
    tmp_path: &Path,
    final_path: &Path,
    record_root: &Path,
) -> io::Result<()> {
    file.write_all(body)?;
    file.sync_all()?;
    std::fs::rename(tmp_path, final_path)?;
    File::open(record_root)?.sync_all()
}

/// Atomically persist the record before the group is created.
///
/// The bytes are deterministic for one identity, so republishing restores the
/// authoritative content instead of changing any binding. A crash before the
/// rename leaves no record; combined with the mkdir step running only after a
/// successful publish, "no record implies no group was ever created" stays
/// provable.
fn write_record(
    record_root: &Path,
    identity: &AttemptIdentity,
    parent: &Path,
    name: &str,
) -> io::Result<()> {
    // Validate before persisting anything: an invalid request never leaves
    // a record behind.
    group_path(parent, name)?;
    let workspace = record_value(
        "workspace",
        identity
            .workspace()
            .to_str()
            .ok_or_else(|| io::Error::other("workspace path is not valid UTF-8"))?,
    )?;
    let job_id = record_value("job", identity.job_id())?;
    let parent = record_value(
        "parent",
        parent
            .to_str()
            .ok_or_else(|| io::Error::other("cgroup parent path is not valid UTF-8"))?,
    )?;
    let attempt = identity.attempt();
    let writer = record_value("writer", identity.writer())?;
    let body = format!(
        "{RECORD_MAGIC}\nworkspace={workspace}\njob={job_id}\nwriter={writer}\n\
         attempt={attempt}\nparent={parent}\nname={name}\n"
    );
    std::fs::create_dir_all(record_root)?;
    let final_path = identity.record_path(record_root);
    let mut suffix = 0u32;
    let (mut file, tmp_path) = loop {
        let candidate = record_root.join(format!(".{name}.{}.{}.tmp", std::process::id(), suffix));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (file, candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && suffix < 32 => {
                suffix += 1;
            }
            Err(error) => return Err(error),
        }
    };
    let published = publish_record(
        &mut file,
        body.as_bytes(),
        &tmp_path,
        &final_path,
        record_root,
    );
    drop(file);
    if let Err(error) = published {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(error);
    }
    Ok(())
}

/// Load and verify the persisted record for `identity`.
///
/// `Ok(None)` means no record file exists. Any structural, path or identity
/// mismatch fails closed instead of guessing.
fn read_record(
    record_root: &Path,
    identity: &AttemptIdentity,
) -> io::Result<Option<ContainmentRecord>> {
    let bytes = match std::fs::read(identity.record_path(record_root)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if bytes.len() > RECORD_LIMIT {
        return Err(io::Error::other("oversized containment record"));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| io::Error::other("containment record is not UTF-8"))?;
    let record = parse_record(text)?;
    let workspace = identity
        .workspace()
        .to_str()
        .ok_or_else(|| io::Error::other("workspace path is not valid UTF-8"))?;
    if record.workspace != workspace
        || record.job_id != identity.job_id()
        || record.writer != identity.writer()
        || record.attempt != identity.attempt()
        || record.name != identity.group_name()
    {
        return Err(io::Error::other(
            "containment record does not match the attempt identity",
        ));
    }
    // Re-enforce the path constraints on the persisted parent/name pair.
    group_path(&record.parent, &record.name)?;
    Ok(Some(record))
}

/// Strictly parse the fixed seven-line record layout.
fn parse_record(text: &str) -> io::Result<ContainmentRecord> {
    fn field<'lines>(
        lines: &mut impl Iterator<Item = &'lines str>,
        key: &str,
    ) -> io::Result<&'lines str> {
        let line = lines
            .next()
            .ok_or_else(|| io::Error::other("truncated containment record"))?;
        line.strip_prefix(key)
            .ok_or_else(|| io::Error::other("invalid containment record field"))
    }
    let mut lines = text.lines();
    if lines.next() != Some(RECORD_MAGIC) {
        return Err(io::Error::other("invalid containment record marker"));
    }
    let workspace = field(&mut lines, "workspace=")?;
    let job_id = field(&mut lines, "job=")?;
    let writer = field(&mut lines, "writer=")?;
    let attempt = field(&mut lines, "attempt=")?;
    let parent = field(&mut lines, "parent=")?;
    let name = field(&mut lines, "name=")?;
    if lines.next().is_some() {
        return Err(io::Error::other("trailing containment record content"));
    }
    Ok(ContainmentRecord {
        workspace: record_value("workspace", workspace)?.to_owned(),
        job_id: record_value("job", job_id)?.to_owned(),
        writer: record_value("writer", writer)?.to_owned(),
        attempt: attempt
            .parse::<u64>()
            .map_err(|_| io::Error::other("invalid containment record attempt"))?,
        parent: record_value("parent", parent)?.to_owned().into(),
        name: record_value("name", name)?.to_owned(),
    })
}

impl LinuxContainment {
    pub fn create(parent: &Path, name: &str) -> io::Result<Self> {
        use rustix::fs::{Mode, OFlags, mkdirat, openat};
        let path = group_path(parent, name)?;
        let directory = open_parent_chain(parent)?;
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        mkdirat(&directory, name, Mode::RUSR | Mode::WUSR | Mode::XUSR)?;
        let group = openat(&directory, name, flags, Mode::empty())?;
        Ok(Self {
            directory: File::from(group),
            path,
            admission_closed: false,
            launch_issued: false,
            child: None,
            parent: directory,
            name: name.to_owned(),
            drained: false,
            removed: false,
        })
    }

    /// Two-phase recorded creation: persist the containment record first, then
    /// create the group.
    ///
    /// Because the record is durable before `mkdir` runs, an absent record
    /// proves this protocol never created a group for `identity`, and a
    /// record without a group is the recoverable crash window between the two
    /// phases. Records are retained after recovery so that invariant stays
    /// provable across successor retries; this adapter does not delete
    /// records, and unremoved domains are never deleted. Workspace-level
    /// record cleanup beyond that is the caller's responsibility.
    pub fn create_recorded(
        record_root: &Path,
        identity: &AttemptIdentity,
        parent: &Path,
    ) -> io::Result<Self> {
        let name = identity.group_name();
        group_path(parent, &name)?;
        write_record(record_root, identity, parent, &name)?;
        Self::create(parent, &name)
    }

    /// Open an already-created group left behind by a crashed owner.
    ///
    /// The attached owner permanently refuses launches: both admission flags
    /// are set before the descriptor is returned, so a successor may only
    /// drain the recorded domain and never admit new work through it. A
    /// missing group surfaces as [`io::ErrorKind::NotFound`] so recovery can
    /// report [`RecoveryOutcome::AlreadyAbsent`].
    pub fn attach(parent: &Path, name: &str) -> io::Result<Self> {
        use rustix::fs::{Mode, OFlags, openat};
        let path = group_path(parent, name)?;
        let directory = open_parent_chain(parent)?;
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let group = match openat(&directory, name, flags, Mode::empty()) {
            Ok(group) => group,
            Err(error) if error == rustix::io::Errno::NOENT => {
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            directory: File::from(group),
            path,
            admission_closed: true,
            launch_issued: true,
            child: None,
            parent: directory,
            name: name.to_owned(),
            drained: false,
            removed: false,
        })
    }

    /// Absolute path of the exclusively created group.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn poll_exit(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        self.child
            .as_mut()
            .ok_or_else(|| io::Error::other("no owned child"))?
            .try_wait()
    }

    /// Construct one launcher command. This is internal to the adapter; the
    /// owner must not call `Command::new` and `spawn` around it. Use
    /// [`Self::spawn`] so start/reap stays inside the cgroup boundary.
    fn command(&mut self, python: &Path, launcher: &Path, target: &Path) -> io::Result<Command> {
        if self.admission_closed
            || self.launch_issued
            || !python.is_absolute()
            || !launcher.is_absolute()
            || !target.is_absolute()
        {
            return Err(io::Error::other(
                "invalid containment launch state or executable",
            ));
        }
        let mut command = Command::new(python);
        self.launch_issued = true;
        command.arg(launcher).arg(&self.path).arg("--").arg(target);
        Ok(command)
    }

    /// Spawn the launcher inside the already-created cgroup in one step.
    ///
    /// Retains the launcher so a not-yet-joined process cannot survive drain.
    pub fn spawn(&mut self, python: &Path, launcher: &Path, target: &Path) -> io::Result<()> {
        let mut command = self.command(python, launcher, target)?;
        self.child = Some(command.spawn()?);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_silent_with_environment(
        &mut self,
        python: &Path,
        launcher: &Path,
        target: &Path,
        args: &[String],
        set: &[(String, String)],
        remove: &[String],
    ) -> io::Result<()> {
        let mut command = self.command(python, launcher, target)?;
        command
            .args(args)
            .envs(set.iter().map(|(key, value)| (key, value)));
        for key in remove {
            command.env_remove(key);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        self.child = Some(command.spawn()?);
        Ok(())
    }

    /// Starts a protocol worker with owned process lifetime but transferable
    /// pipes. Pipe ownership does not permit another launch or process reaping.
    pub fn spawn_piped(
        &mut self,
        python: &Path,
        launcher: &Path,
        target: &Path,
        args: &[&str],
    ) -> io::Result<(ChildStdin, ChildStdout, ChildStderr)> {
        self.spawn_piped_with_environment(python, launcher, target, args, &[], &[])
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_piped_with_environment(
        &mut self,
        python: &Path,
        launcher: &Path,
        target: &Path,
        args: &[&str],
        set: &[(String, String)],
        remove: &[String],
    ) -> io::Result<(ChildStdin, ChildStdout, ChildStderr)> {
        let mut command = self.command(python, launcher, target)?;
        command.envs(set.iter().map(|(key, value)| (key, value)));
        for key in remove {
            command.env_remove(key);
        }
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        self.child = Some(command.spawn()?);
        let child = self.child.as_mut().expect("child retained after spawn");
        Ok((
            child
                .stdin
                .take()
                .ok_or_else(|| io::Error::other("missing stdin"))?,
            child
                .stdout
                .take()
                .ok_or_else(|| io::Error::other("missing stdout"))?,
            child
                .stderr
                .take()
                .ok_or_else(|| io::Error::other("missing stderr"))?,
        ))
    }

    /// Remove only after a successful drain; failure leaves ownership intact.
    pub fn remove_group(&mut self) -> io::Result<()> {
        if self.removed {
            return Ok(());
        }
        if !self.drained {
            return Err(io::Error::other("group has not drained"));
        }
        rustix::fs::unlinkat(
            &self.parent,
            self.name.as_str(),
            rustix::fs::AtFlags::REMOVEDIR,
        )?;
        self.removed = true;
        Ok(())
    }

    /// Close admission and reap the launcher before killing remaining members.
    /// A timeout or wait error retains the child for an explicit retry.
    pub fn stop_and_drain(&mut self, timeout: Duration) -> io::Result<()> {
        use rustix::fs::{Mode, OFlags, openat};
        self.admission_closed = true;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::other("invalid drain timeout"))?;
        if let Some(child) = self.child.as_mut() {
            if child.try_wait()?.is_none() {
                child.kill()?;
                loop {
                    if child.try_wait()?.is_some() {
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "launcher reap timeout",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
        let fd = openat(
            &self.directory,
            "cgroup.kill",
            OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        File::from(fd).write_all(b"1")?;
        loop {
            let fd = openat(
                &self.directory,
                "cgroup.events",
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?;
            let mut events = String::new();
            File::from(fd).take(4097).read_to_string(&mut events)?;
            let states: Vec<_> = events
                .lines()
                .filter(|line| line.starts_with("populated "))
                .collect();
            if events.len() > 4096 {
                return Err(io::Error::other("oversized cgroup events"));
            }
            if states == ["populated 0"] {
                self.child = None;
                self.drained = true;
                // Keep the launcher handle across every fallible cgroup
                // operation. A failed kill/event read or timeout must leave
                // the complete owner available for an explicit retry.
                self.child = None;
                return Ok(());
            }
            if states != ["populated 1"] {
                return Err(io::Error::other("invalid cgroup populated state"));
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "cgroup drain timeout",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Successor-side recovery of one crashed attempt's writer domain.
    ///
    /// Loads the record persisted by [`Self::create_recorded`], verifies it
    /// against `identity` (any mismatch fails closed), attaches the recorded
    /// group with admission permanently closed, then reuses
    /// [`Self::stop_and_drain`] and [`Self::remove_group`].
    ///
    /// Every returned [`RecoveryOutcome`] proves that the recorded writer
    /// domain has no live descendant; any `Err` must reject successor
    /// admission rather than assume the domain is quiet.
    pub fn recover_attempt(
        record_root: &Path,
        identity: &AttemptIdentity,
        timeout: Duration,
    ) -> io::Result<RecoveryOutcome> {
        let record = match read_record(record_root, identity)? {
            Some(record) => record,
            None => return Ok(RecoveryOutcome::NoRecord),
        };
        let mut owner = match Self::attach(&record.parent, &record.name) {
            Ok(owner) => owner,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(RecoveryOutcome::AlreadyAbsent);
            }
            Err(error) => return Err(error),
        };
        owner.stop_and_drain(timeout)?;
        owner.remove_group()?;
        Ok(RecoveryOutcome::Drained)
    }
}
