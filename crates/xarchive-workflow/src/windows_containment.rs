//! Attempt-bound Windows Job Object ownership.
//!
//! A keeper inherits a Job Object handle before writers can start. It keeps
//! the name discoverable after its creator exits. An absent name never proves
//! drain: the keeper itself may have died. Recovery must hold its session
//! scopes, open the object and observe zero active processes before issuing
//! any evidence. This is a trusted-same-user boundary; the adapter is not yet
//! wired to writer registration or production recovery.

use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use firehazard::{access, handle, job, process};
use sha2::{Digest, Sha256};

/// The durable identity of one attempt's writer role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsWriterIdentity {
    workspace: PathBuf,
    attempt: crate::AttemptIdentity,
    role: &'static str,
}

impl WindowsWriterIdentity {
    pub fn new(
        workspace: &Path,
        attempt: &crate::AttemptIdentity,
        role: &'static str,
    ) -> io::Result<Self> {
        if !workspace.is_absolute()
            || workspace
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            || workspace.to_str().is_none()
            || !matches!(role, "sidecar" | "aria2-0" | "aria2-1")
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid Windows writer identity",
            ));
        }
        Ok(Self {
            workspace: workspace.to_path_buf(),
            attempt: attempt.clone(),
            role,
        })
    }

    fn object_name(&self) -> widestring::U16CString {
        let mut hash = Sha256::new();
        for value in [
            self.workspace
                .to_str()
                .expect("validated workspace")
                .as_bytes(),
            self.attempt.job_id().as_bytes(),
            &self.attempt.attempt().to_le_bytes(),
            self.role.as_bytes(),
        ] {
            hash.update((value.len() as u64).to_le_bytes());
            hash.update(value);
        }
        widestring::U16CString::from_str(format!("Global\\XArchive-writer-{:x}", hash.finalize()))
            .expect("generated name has no NUL")
    }
}

/// A uniquely created or recovered Job Object.
pub struct WindowsWriterJob {
    job: job::OwnedHandle,
    identity: WindowsWriterIdentity,
    admission_closed: bool,
    keeper_started: bool,
}

impl WindowsWriterJob {
    /// Refuse to adopt a job that already exists under this attempt identity.
    pub fn create(identity: WindowsWriterIdentity) -> io::Result<Self> {
        let name = identity.object_name();
        let job = job::create_job_object_w(None, name.as_ucstr()).map_err(io::Error::from)?;
        // CreateJobObjectW reports a successful open of an existing name
        // through the thread's last-error value.
        if u32::from(firehazard::Error::get_last()) == 183 {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "writer job already exists",
            ));
        }
        let mut limits = job::object::ExtendedLimitInformation::default();
        limits.basic_limit_information.limit_flags = job::object::limit::KILL_ON_JOB_CLOSE;
        job::set_information_job_object(&job, limits).map_err(io::Error::from)?;
        Ok(Self {
            job,
            identity,
            admission_closed: false,
            keeper_started: false,
        })
    }

    /// Reopen the exact job after the previous owner has exited.
    pub fn open(identity: WindowsWriterIdentity) -> io::Result<Option<Self>> {
        let name = identity.object_name();
        match job::open_job_object_w(access::GENERIC_ALL, false, name.as_ucstr()) {
            Ok(job) => Ok(Some(Self {
                job,
                identity,
                admission_closed: true,
                keeper_started: true,
            })),
            Err(error) if u32::from(error) == 2 => Ok(None),
            Err(error) => Err(io::Error::from(error)),
        }
    }

    pub fn identity(&self) -> &WindowsWriterIdentity {
        &self.identity
    }

    /// Start the keeper mode of the current Desktop executable.
    pub fn start_desktop_keeper(&mut self) -> io::Result<process::Information> {
        let executable = std::env::current_exe()?;
        self.start_keeper(&executable, &["--xarchive-writer-keeper"])
    }

    /// Start a keeper atomically inside the job with an inherited handle to it.
    /// The caller must retain this process and only launch a writer once it is
    /// running. Failure to reopen the name is never evidence of a drain.
    fn start_keeper(
        &mut self,
        executable: &Path,
        args: &[&str],
    ) -> io::Result<process::Information> {
        if self.admission_closed || self.keeper_started || !executable.is_absolute() {
            return Err(io::Error::other(
                "keeper admission closed or invalid executable",
            ));
        }
        self.keeper_started = true;
        use std::os::windows::ffi::OsStrExt;
        let executable_name = widestring::U16CString::from_vec(
            executable.as_os_str().encode_wide().collect::<Vec<_>>(),
        )
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in executable"))?;
        let inheritable =
            handle::duplicate_handle_local_same_access(&self.job, true).map_err(io::Error::from)?;
        let inherited = [handle::Borrowed::from(&inheritable)];
        let jobs = [job::Handle::from(&self.job)];
        let attributes = [
            process::ThreadAttributeRef::job_list(&jobs),
            process::ThreadAttributeRef::handle_list(&inherited),
        ];
        let list = process::ThreadAttributeList::try_from(attributes.as_slice())
            .map_err(io::Error::from)?;
        let startup = process::StartupInfoExW {
            attribute_list: Some(list),
            ..Default::default()
        };
        let mut command = process::argv_to_command_line_0(executable, args);
        process::create_process_w(
            executable_name.as_ucstr(),
            Some(&mut command),
            None,
            None,
            true,
            process::EXTENDED_STARTUPINFO_PRESENT | process::CREATE_NO_WINDOW,
            process::environment::Inherit,
            (),
            &startup,
        )
        .map_err(io::Error::from)
    }

    #[cfg(test)]
    fn spawn_test_writer(
        &self,
        executable: &Path,
        args: &[&str],
    ) -> io::Result<process::Information> {
        use std::os::windows::ffi::OsStrExt;
        let executable_name = widestring::U16CString::from_vec(
            executable.as_os_str().encode_wide().collect::<Vec<_>>(),
        )
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in executable"))?;
        let jobs = [job::Handle::from(&self.job)];
        let attributes = [process::ThreadAttributeRef::job_list(&jobs)];
        let list = process::ThreadAttributeList::try_from(attributes.as_slice())
            .map_err(io::Error::from)?;
        let startup = process::StartupInfoExW {
            attribute_list: Some(list),
            ..Default::default()
        };
        let mut command = process::argv_to_command_line_0(executable, args);
        process::create_process_w(
            executable_name.as_ucstr(),
            Some(&mut command),
            None,
            None,
            false,
            process::EXTENDED_STARTUPINFO_PRESENT | process::CREATE_NO_WINDOW,
            process::environment::Inherit,
            (),
            &startup,
        )
        .map_err(io::Error::from)
    }

    /// Terminate every member and wait until the kernel reports none remain.
    pub fn stop_and_drain(&mut self, timeout: Duration) -> io::Result<()> {
        self.admission_closed = true;
        job::terminate_job_object(&self.job, 1).map_err(io::Error::from)?;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid drain timeout"))?;
        loop {
            let accounting: job::object::BasicAccountingInformation =
                job::query_information_job_object(&self.job).map_err(io::Error::from)?;
            if accounting.active_processes == 0 {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "writer job drain timed out",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_owner_probe() {
        let Some(root) = std::env::var_os("TW2TG_KEEPER_PROBE_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let attempt = crate::AttemptIdentity::new("crash-owner-probe", 1).unwrap();
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut job = WindowsWriterJob::create(identity).unwrap();
        let executable = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        let keeper = if let Some(desktop) = std::env::var_os("TW2TG_DESKTOP_KEEPER_EXE") {
            job.start_keeper(Path::new(&desktop), &["--xarchive-writer-keeper"])
        } else {
            job.start_keeper(
                &executable,
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Start-Sleep -Seconds 20",
                ],
            )
        }
        .unwrap();
        assert_eq!(
            firehazard::process::get_exit_code_process(&keeper.process).unwrap(),
            259
        );
        let writer = job
            .spawn_test_writer(
                &executable,
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Start-Sleep -Seconds 20",
                ],
            )
            .unwrap();
        assert_eq!(
            firehazard::process::get_exit_code_process(&writer.process).unwrap(),
            259
        );
        std::fs::write(root.join("ready"), b"ready").unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn successor_opens_and_drains_job_after_owner_is_killed() {
        let root = std::env::temp_dir().join(format!("tw2tg-job-crash-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let mut owner = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "windows_containment::tests::crash_owner_probe"])
            .env("TW2TG_KEEPER_PROBE_ROOT", &root)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("ready").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let ready = root.join("ready").exists();
        owner.kill().unwrap();
        owner.wait().unwrap();
        assert!(ready, "owner never established keeper");
        let attempt = crate::AttemptIdentity::new("crash-owner-probe", 1).unwrap();
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut successor = WindowsWriterJob::open(identity)
            .unwrap()
            .expect("keeper lost after owner crash");
        let before: job::object::BasicAccountingInformation =
            job::query_information_job_object(&successor.job).unwrap();
        assert!(before.active_processes >= 2);
        successor.stop_and_drain(Duration::from_secs(10)).unwrap();
        let after: job::object::BasicAccountingInformation =
            job::query_information_job_object(&successor.job).unwrap();
        assert_eq!(after.active_processes, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeper_retains_job_after_owner_drops_and_successor_drains() {
        let workspace = std::env::current_dir().unwrap();
        let attempt =
            crate::AttemptIdentity::new(format!("keeper-probe-{}", std::process::id()), 1).unwrap();
        let identity = WindowsWriterIdentity::new(&workspace, &attempt, "sidecar").unwrap();
        let mut owner = WindowsWriterJob::create(identity.clone()).unwrap();
        assert!(WindowsWriterJob::create(identity.clone()).is_err());
        let executable = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        let keeper = owner
            .start_keeper(
                &executable,
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Start-Sleep -Seconds 20",
                ],
            )
            .unwrap();
        assert_eq!(
            firehazard::process::get_exit_code_process(&keeper.process).unwrap(),
            259
        );
        drop(owner);
        let mut successor = WindowsWriterJob::open(identity)
            .unwrap()
            .expect("keeper must retain job");
        assert!(
            successor
                .start_keeper(&executable, &["-Command", "exit"])
                .is_err()
        );
        let before: job::object::BasicAccountingInformation =
            job::query_information_job_object(&successor.job).unwrap();
        assert!(before.active_processes >= 1);
        successor.stop_and_drain(Duration::from_secs(10)).unwrap();
        let after: job::object::BasicAccountingInformation =
            job::query_information_job_object(&successor.job).unwrap();
        assert_eq!(after.active_processes, 0);
        assert_ne!(
            firehazard::process::get_exit_code_process(&keeper.process).unwrap(),
            259
        );
    }
}
