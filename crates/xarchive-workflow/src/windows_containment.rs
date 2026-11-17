//! Attempt-bound Windows Job Object ownership.
//!
//! A keeper inherits a Job Object handle before writers can start. It keeps
//! the name discoverable after its creator exits. An absent name never proves
//! drain: the keeper itself may have died. Recovery must hold its session
//! scopes and prove that every descendant has exited; zero active processes
//! alone cannot mint drain evidence. This is a trusted-same-user boundary;
//! writer registration exists, but production drain and recovery remain gated.

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
        // Keep the workspace pinned while resolving its filesystem name, so
        // recovery derives the same identity from case and existing 8.3 aliases.
        // The pin also rejects reparse-point traversal before canonicalization.
        let _workspace_pin = crate::windows_filesystem::PinnedPath::directory(workspace, false)?;
        let workspace = workspace.canonicalize()?;
        if workspace.to_str().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "non-Unicode canonical workspace",
            ));
        }
        Ok(Self {
            workspace,
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

/// A Windows pipe whose closed writer is exposed as normal end-of-file.
pub struct WindowsPipeReader(firehazard::io::ReadPipe);

impl Read for WindowsPipeReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.0.read(buffer) {
            Err(error) if error.raw_os_error() == Some(109) => Ok(0), // ERROR_BROKEN_PIPE
            result => result,
        }
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
    keeper: Option<process::Information>,
    writer: Option<process::Information>,
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
            keeper: None,
            writer: None,
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
                keeper: None,
                writer: None,
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
    pub fn start_desktop_keeper(&mut self) -> io::Result<()> {
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
        self.keeper = Some(self.start_keeper(&executable, &["--xarchive-writer-keeper"])?);
        Ok(())
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
        let mut command = windows_command_line(executable, args)?;
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
        let mut command = windows_command_line(executable, args)?;
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

    /// Launch one registered writer with only its standard handles inherited.
    /// Admission is held closed to concurrent publication while CreateProcess
    /// atomically assigns the writer to this job.
    pub fn spawn_piped_with_environment(
        &mut self,
        executable: &Path,
        args: &[&str],
        set: &[(String, String)],
        remove: &[String],
    ) -> io::Result<(
        firehazard::io::WritePipe,
        WindowsPipeReader,
        WindowsPipeReader,
    )> {
        use std::os::windows::ffi::OsStrExt;
        if self.admission_closed
            || self.writer_started
            || !executable.is_absolute()
            || args
                .iter()
                .any(|arg| arg.encode_utf16().any(|unit| unit == 0))
        {
            return Err(io::Error::other(
                "writer admission closed or invalid command",
            ));
        }
        let keeper = self
            .keeper
            .as_ref()
            .ok_or_else(|| io::Error::other("keeper not started"))?;
        if !process::is_process_running(&keeper.process) {
            return Err(io::Error::other("keeper is not running"));
        }
        let (state, _) = self
            .production_registration
            .as_ref()
            .ok_or_else(|| io::Error::other("writer is not registered"))?;
        let state = state.clone();
        let admission = state
            .lock()
            .map_err(|_| io::Error::other("writer registry poisoned"))?;
        if admission.closed {
            return Err(io::Error::other("writer admission closed"));
        }
        let executable_name = widestring::U16CString::from_vec(
            executable.as_os_str().encode_wide().collect::<Vec<_>>(),
        )
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in executable"))?;
        let env = windows_environment(set, remove)?;
        let attrs = firehazard::security::Attributes::new(None, true);
        let (child_stdin, parent_stdin) =
            firehazard::io::create_pipe(Some(&attrs), 0).map_err(io::Error::from)?;
        let (parent_stdout, child_stdout) =
            firehazard::io::create_pipe(Some(&attrs), 0).map_err(io::Error::from)?;
        let (parent_stderr, child_stderr) =
            firehazard::io::create_pipe(Some(&attrs), 0).map_err(io::Error::from)?;
        let handles = [
            handle::Borrowed::from(&child_stdin),
            handle::Borrowed::from(&child_stdout),
            handle::Borrowed::from(&child_stderr),
        ];
        let jobs = [job::Handle::from(&self.job)];
        let attributes = [
            process::ThreadAttributeRef::job_list(&jobs),
            process::ThreadAttributeRef::handle_list(&handles),
        ];
        let list = process::ThreadAttributeList::try_from(attributes.as_slice())
            .map_err(io::Error::from)?;
        let mut startup = process::StartupInfoExW {
            attribute_list: Some(list),
            ..Default::default()
        };
        startup.startup_info.flags = 0x0000_0100; // STARTF_USESTDHANDLES
        startup.startup_info.std_input = Some(firehazard::io::ReadHandle::from(&child_stdin));
        startup.startup_info.std_output = Some(firehazard::io::WriteHandle::from(&child_stdout));
        startup.startup_info.std_error = Some(firehazard::io::WriteHandle::from(&child_stderr));
        let mut command = windows_command_line(executable, args)?;
        let writer = process::create_process_w(
            executable_name.as_ucstr(),
            Some(&mut command),
            None,
            None,
            true,
            process::EXTENDED_STARTUPINFO_PRESENT
                | process::CREATE_NO_WINDOW
                | process::CREATE_UNICODE_ENVIRONMENT,
            env.as_slice(),
            (),
            &startup,
        )
        .map_err(io::Error::from)?;
        self.writer = Some(writer);
        self.writer_started = true;
        drop(admission);
        drop(startup);
        drop((child_stdin, child_stdout, child_stderr));
        Ok((
            parent_stdin,
            WindowsPipeReader(parent_stdout),
            WindowsPipeReader(parent_stderr),
        ))
    }

    pub fn poll_writer_exit(&self) -> io::Result<Option<std::process::ExitStatus>> {
        use std::os::windows::process::ExitStatusExt;
        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| io::Error::other("no owned writer"))?;
        if process::is_process_running(&writer.process) {
            return Ok(None);
        }
        let code = process::get_exit_code_process(&writer.process).map_err(io::Error::from)?;
        if code == 259 {
            return Err(io::Error::other("writer exit state is ambiguous"));
        }
        Ok(Some(std::process::ExitStatus::from_raw(code)))
    }

    /// Terminate the job and join owned handles. This does not certify a
    /// complete descendant drain or mint publication authority.
    pub fn stop_and_drain(&mut self, timeout: Duration) -> io::Result<()> {
        self.admission_closed = true;
        job::terminate_job_object(&self.job, 1).map_err(io::Error::from)?;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid drain timeout"))?;
        loop {
            let accounting: job::object::BasicAccountingInformation =
                job::query_information_job_object(&self.job).map_err(io::Error::from)?;
            let owned_processes_exited =
                [&self.writer, &self.keeper]
                    .into_iter()
                    .flatten()
                    .all(|child| {
                        !process::is_process_running(&child.process)
                            && process::get_exit_code_process(&child.process)
                                .is_ok_and(|code| code != 259)
                    });
            if accounting.active_processes == 0 && owned_processes_exited {
                // Job accounting and the two owned process handles do not
                // certify that every descendant handle has signalled. Leave
                // the production registration undrained until that proof exists.
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

fn windows_command_line(executable: &Path, args: &[&str]) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let mut line = Vec::new();
    for argument in std::iter::once(executable.as_os_str().encode_wide().collect::<Vec<_>>()).chain(
        args.iter()
            .map(|arg| arg.encode_utf16().collect::<Vec<_>>()),
    ) {
        if argument.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "NUL in command line",
            ));
        }
        if !line.is_empty() {
            line.push(b' ' as u16);
        }
        line.push(b'"' as u16);
        let mut slashes = 0;
        for unit in argument {
            if unit == b'\\' as u16 {
                slashes += 1;
                continue;
            }
            if unit == b'"' as u16 {
                line.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2 + 1));
            } else {
                line.extend(std::iter::repeat_n(b'\\' as u16, slashes));
            }
            slashes = 0;
            line.push(unit);
        }
        line.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
        line.push(b'"' as u16);
    }
    line.push(0);
    Ok(line)
}

fn windows_environment(set: &[(String, String)], remove: &[String]) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    if set
        .iter()
        .any(|(key, value)| key.is_empty() || key.contains(['=', '\0']) || value.contains('\0'))
        || remove
            .iter()
            .any(|key| key.is_empty() || key.contains(['=', '\0']))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid environment variable",
        ));
    }
    let mut vars: Vec<_> = std::env::vars_os().collect();
    for (key, value) in set {
        vars.retain(|(existing, _)| !existing.to_string_lossy().eq_ignore_ascii_case(key));
        vars.push((key.into(), value.into()));
    }
    vars.retain(|(key, _)| {
        !remove
            .iter()
            .any(|removed| key.to_string_lossy().eq_ignore_ascii_case(removed))
    });
    vars.sort_by_cached_key(|(key, _)| key.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (key, value) in vars {
        block.extend(key.encode_wide());
        block.push('=' as u16);
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    if block.len() == 1 {
        block.push(0);
    }
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_piped_writer_refuses_publication_without_descendant_proof() {
        use std::io::BufRead;
        let root = std::env::temp_dir().join(format!("tw2tg-piped-writer-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let coordinator = crate::coordination::windows::WindowsCoordinator::new(&root).unwrap();
        let attempt = crate::AttemptIdentity::new("piped-job", 1).unwrap();
        let session =
            crate::ExecutionReservation::acquire(&coordinator, "piped-job", &root.join("archive"))
                .unwrap()
                .bind_existing_attempt(attempt.clone())
                .unwrap();
        let publication = crate::writer_lifecycle::PublicationOwner::for_session(&session);
        let registration = publication.registration();
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut job = WindowsWriterJob::create_recorded(&root, identity).unwrap();
        registration.register_windows(&mut job).unwrap();
        let desktop = std::env::var_os("TW2TG_DESKTOP_KEEPER_EXE")
            .expect("set Desktop keeper executable for this test");
        job.keeper = Some(
            job.start_keeper(Path::new(&desktop), &["--xarchive-writer-keeper"])
                .unwrap(),
        );
        let executable = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        let script = "[Console]::Out.WriteLine($env:TW2TG_PIPE_TEST); [Console]::Out.WriteLine([Console]::In.ReadLine()); Start-Sleep -Seconds 20";
        let (mut stdin, stdout, stderr) = job
            .spawn_piped_with_environment(
                &executable,
                &["-NoProfile", "-NonInteractive", "-Command", script],
                &[("TW2TG_PIPE_TEST".into(), "bound".into())],
                &[],
            )
            .unwrap();
        stdin.write_all(b"hello\n").unwrap();
        let mut stdout = std::io::BufReader::new(stdout);
        let mut first = String::new();
        let mut second = String::new();
        stdout.read_line(&mut first).unwrap();
        stdout.read_line(&mut second).unwrap();
        assert_eq!(first.trim(), "bound");
        assert_eq!(second.trim(), "hello");
        assert!(job.poll_writer_exit().unwrap().is_none());
        publication.close_admission().unwrap();
        assert_eq!(
            publication.authorize(&session).err(),
            Some(crate::writer_lifecycle::PublicationError::IncompleteDrain)
        );
        drop(stdin);
        job.stop_and_drain(Duration::from_secs(10)).unwrap();
        assert!(job.poll_writer_exit().unwrap().is_some());
        let mut eof = String::new();
        assert_eq!(stdout.read_line(&mut eof).unwrap(), 0);
        assert_eq!(
            publication.authorize(&session).err(),
            Some(crate::writer_lifecycle::PublicationError::IncompleteDrain)
        );
        drop((stdout, stderr, job, publication));
        drop(session);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn writer_identity_is_stable_across_workspace_case_aliases() {
        let base = std::env::var_os("TW2TG_WRITER_ID_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let root = base.join(format!("tw2tg-Identity-Alias-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let alias = PathBuf::from(root.to_str().unwrap().to_uppercase());
        assert_ne!(root, alias);
        let attempt = crate::AttemptIdentity::new("identity-alias", 1).unwrap();
        let original = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let alternate = WindowsWriterIdentity::new(&alias, &attempt, "sidecar").unwrap();
        let resolved =
            WindowsWriterIdentity::new(&root.canonicalize().unwrap(), &attempt, "sidecar").unwrap();
        assert_eq!(original, alternate);
        assert_eq!(original, resolved);
        assert_eq!(original.object_name(), alternate.object_name());
        let owner = WindowsWriterJob::create_recorded(&root, original).unwrap();
        let recovered = WindowsWriterJob::open_recorded(&root, alternate).unwrap();
        drop((owner, recovered));
        std::fs::remove_dir_all(root).unwrap();
    }

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
