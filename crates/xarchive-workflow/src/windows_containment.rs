//! Attempt-bound Windows Job Object ownership.
//!
//! A keeper inherits a Job Object handle before writers can start. It keeps
//! the name discoverable after its creator exits. An absent name never proves
//! drain: the keeper itself may have died. Recovery must hold its session
//! scopes, open the object and observe zero active processes before issuing
//! any evidence. This is a trusted-same-user boundary; the adapter is not yet
//! wired to writer registration or production recovery.

use std::io::{self, Read, Write};
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
    fn record_bytes(&self) -> Vec<u8> {
        format!(
            "xarchive-writer-job-v1\n{}\n",
            self.object_name().to_string_lossy()
        )
        .into_bytes()
    }

    fn record_path(&self, root: &Path) -> PathBuf {
        let name = self.object_name().to_string_lossy();
        root.join(format!(
            "{}.record",
            name.strip_prefix("Global\\")
                .expect("generated global job name")
        ))
    }
}

/// A uniquely created or recovered Job Object.
pub struct WindowsWriterJob {
    job: job::OwnedHandle,
    identity: WindowsWriterIdentity,
    admission_closed: bool,
    keeper_started: bool,
    recorded: bool,
    writer_started: bool,
    production_registration: Option<(
        std::sync::Arc<std::sync::Mutex<crate::writer_lifecycle::ProductionWriters>>,
        String,
    )>,
}

impl WindowsWriterJob {
    /// Persist the exact identity before creating any process domain.
    /// A partial record is retained and fails closed during recovery.
    pub fn create_recorded(
        record_root: &Path,
        identity: WindowsWriterIdentity,
    ) -> io::Result<Self> {
        use std::os::windows::fs::OpenOptionsExt;
        let _root_pin = crate::windows_filesystem::PinnedPath::directory(record_root, true)?;
        let path = identity.record_path(record_root);
        let mut record = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(0)
            .custom_flags(0x8000_0000)
            .open(&path)?;
        record.write_all(&identity.record_bytes())?;
        record.sync_all()?;
        drop(record);
        let mut owner = Self::create(identity)?;
        owner.recorded = true;
        Ok(owner)
    }

    /// Open only the job named by a matching, complete record. An absent job
    /// is a refusal: it does not prove the writer process tree has drained.
    pub fn open_recorded(record_root: &Path, identity: WindowsWriterIdentity) -> io::Result<Self> {
        let _root_pin = crate::windows_filesystem::PinnedPath::directory(record_root, false)?;
        let pin = crate::windows_filesystem::PinnedPath::media(&identity.record_path(record_root))?;
        let mut bytes = Vec::new();
        pin.file().take(1025).read_to_end(&mut bytes)?;
        if bytes != identity.record_bytes() {
            return Err(io::Error::other("invalid Windows writer record"));
        }
        Self::open(identity)?.ok_or_else(|| {
            io::Error::other("recorded writer job is absent; drain cannot be proven")
        })
    }

    /// Refuse to adopt a job that already exists under this attempt identity.
    fn create(identity: WindowsWriterIdentity) -> io::Result<Self> {
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
            recorded: false,
            writer_started: false,
            production_registration: None,
        })
    }

    /// Reopen the exact job after the previous owner has exited.
    fn open(identity: WindowsWriterIdentity) -> io::Result<Option<Self>> {
        let name = identity.object_name();
        match job::open_job_object_w(access::GENERIC_ALL, false, name.as_ucstr()) {
            Ok(job) => Ok(Some(Self {
                job,
                identity,
                admission_closed: true,
                keeper_started: true,
                recorded: true,
                writer_started: false,
                production_registration: None,
            })),
            Err(error) if u32::from(error) == 2 => Ok(None),
            Err(error) => Err(io::Error::from(error)),
        }
    }

    pub fn identity(&self) -> &WindowsWriterIdentity {
        &self.identity
    }

    pub(crate) fn matches_unstarted_recorded(&self, attempt: &crate::AttemptIdentity) -> bool {
        self.recorded
            && !self.admission_closed
            && !self.keeper_started
            && self.production_registration.is_none()
            && &self.identity.attempt == attempt
    }

    pub(crate) fn writer_role(&self) -> &'static str {
        self.identity.role
    }

    pub(crate) fn register_production(
        &mut self,
        state: std::sync::Arc<std::sync::Mutex<crate::writer_lifecycle::ProductionWriters>>,
        role: String,
    ) {
        self.production_registration = Some((state, role));
    }

    /// Start the keeper mode of the current Desktop executable.
    pub fn start_desktop_keeper(&mut self) -> io::Result<process::Information> {
        let (state, _) = self
            .production_registration
            .as_ref()
            .ok_or_else(|| io::Error::other("writer is not registered"))?;
        if state
            .lock()
            .map_err(|_| io::Error::other("writer registry poisoned"))?
            .closed
        {
            return Err(io::Error::other("writer admission closed"));
        }
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
                if self.writer_started {
                    if let Some((state, role)) = &self.production_registration {
                        state
                            .lock()
                            .map_err(|_| io::Error::other("writer registry poisoned"))?
                            .mark_drained(role)
                            .map_err(|_| io::Error::other("writer drain registration rejected"))?;
                    }
                }
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
    fn recorded_registration_rejects_mismatch_and_unstarted_drain() {
        let root =
            std::env::temp_dir().join(format!("tw2tg-job-registration-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let coordinator = crate::coordination::windows::WindowsCoordinator::new(&root).unwrap();
        let attempt = crate::AttemptIdentity::new("registered-job", 1).unwrap();
        let session = crate::ExecutionReservation::acquire(
            &coordinator,
            "registered-job",
            &root.join("archive"),
        )
        .unwrap()
        .bind_existing_attempt(attempt.clone())
        .unwrap();
        let publication = crate::writer_lifecycle::PublicationOwner::for_session(&session);
        let registration = publication.registration();
        let foreign = WindowsWriterIdentity::new(
            &root,
            &crate::AttemptIdentity::new("foreign", 1).unwrap(),
            "sidecar",
        )
        .unwrap();
        let mut foreign_job = WindowsWriterJob::create_recorded(&root, foreign).unwrap();
        assert!(registration.register_windows(&mut foreign_job).is_err());
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut job = WindowsWriterJob::create_recorded(&root, identity).unwrap();
        registration.register_windows(&mut job).unwrap();
        assert!(registration.register_windows(&mut job).is_err());
        publication.close_admission().unwrap();
        assert!(job.start_desktop_keeper().is_err());
        job.stop_and_drain(Duration::from_secs(5)).unwrap();
        assert_eq!(
            publication.authorize(&session).err(),
            Some(crate::writer_lifecycle::PublicationError::IncompleteDrain)
        );
        drop((job, foreign_job, publication));
        drop(session);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn records_are_exclusive_bound_and_absence_never_proves_drain() {
        let root = std::env::temp_dir().join(format!("tw2tg-job-record-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let attempt = crate::AttemptIdentity::new("recorded-job", 1).unwrap();
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let owner = WindowsWriterJob::create_recorded(&root, identity.clone()).unwrap();
        assert!(WindowsWriterJob::create_recorded(&root, identity.clone()).is_err());
        let mut recovery = WindowsWriterJob::open_recorded(&root, identity.clone()).unwrap();
        recovery.stop_and_drain(Duration::from_secs(5)).unwrap();
        drop(recovery);
        drop(owner);
        assert!(WindowsWriterJob::open_recorded(&root, identity.clone()).is_err());
        std::fs::write(identity.record_path(&root), b"partial").unwrap();
        assert!(WindowsWriterJob::open_recorded(&root, identity).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn crash_owner_probe() {
        let Some(root) = std::env::var_os("TW2TG_KEEPER_PROBE_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let attempt = crate::AttemptIdentity::new("crash-owner-probe", 1).unwrap();
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut job = WindowsWriterJob::create_recorded(&root, identity).unwrap();
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
        let mut successor = WindowsWriterJob::open_recorded(&root, identity)
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
