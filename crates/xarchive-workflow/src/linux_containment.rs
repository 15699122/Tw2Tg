//! Linux trusted-tool cgroup owner. Not an adversarial same-user sandbox.
//!
//! The owner holds the exclusively created group's directory descriptor through
//! explicit cleanup, so admission control and descriptor-relative kill/drain
//! stay inside one boundary the caller cannot interleave on its own schedule.
//! The owner retains and reaps its launcher before final drain and supports
//! explicit removal after drain. Crash-orphan recovery is not yet implemented.
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

impl LinuxContainment {
    pub fn create(parent: &Path, name: &str) -> io::Result<Self> {
        use rustix::fs::{Mode, OFlags, mkdirat, open, openat};
        let path = group_path(parent, name)?;
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
        mkdirat(&directory, name, Mode::RUSR | Mode::WUSR | Mode::XUSR)?;
        let group = openat(&directory, name, flags, Mode::empty())?;
        Ok(Self {
            directory: File::from(group),
            path,
            admission_closed: false,
            launch_issued: false,
            child: None,
            parent: File::from(directory),
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
            self.child = None;
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
                self.drained = true;
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
}
