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

/// Private adapter-minted proof that the complete fixed writer-role set for an
/// attempt has been recovered and drained.
#[derive(Debug)]
pub struct VerifiedAttemptDrain {
    identity: crate::AttemptIdentity,
    workspace: PathBuf,
    operation: std::sync::Arc<()>,
    roles: [&'static str; 3],
    _private: (),
}

/// Opaque, session-derived identity for one controlled recovery drain over the
/// production writer-role set. It cannot be constructed by Desktop callers.
#[derive(Debug)]
pub struct RecoveryDrainBinding {
    identity: crate::AttemptIdentity,
    workspace: PathBuf,
    operation: std::sync::Arc<()>,
    roles: [&'static str; 3],
}

impl RecoveryDrainBinding {
    pub(crate) fn for_session<C: crate::coordination::Coordinator>(
        session: &crate::ExecutionSession<'_, C>,
        workspace: &Path,
    ) -> Self {
        Self {
            identity: session.attempt().clone(),
            workspace: workspace.to_path_buf(),
            operation: session.publication_source.clone(),
            roles: ["sidecar", "aria2-0", "aria2-1"],
        }
    }
}

impl VerifiedAttemptDrain {
    pub fn identity(&self) -> &crate::AttemptIdentity {
        &self.identity
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub(crate) fn matches_binding(&self, binding: &RecoveryDrainBinding) -> bool {
        self.identity == binding.identity
            && self.workspace == binding.workspace
            && std::sync::Arc::ptr_eq(&self.operation, &binding.operation)
            && self.roles == binding.roles
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
    drained: bool,
    iocp: Option<xarchive_win_job_iocp::JobCompletionPort>,
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
            drained: false,
            iocp: None,
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
                drained: false,
                iocp: None,
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
        self.start_keeper_under_admission(|job| {
            let executable = std::env::current_exe()?;
            job.keeper = Some(job.start_keeper(&executable, &["--xarchive-writer-keeper"])?);
            Ok(())
        })
    }

    fn start_keeper_under_admission(
        &mut self,
        start: impl FnOnce(&mut Self) -> io::Result<()>,
    ) -> io::Result<()> {
        let (state, _) = self
            .production_registration
            .as_ref()
            .ok_or_else(|| io::Error::other("writer is not registered"))?;
        let state = state.clone();
        // Linearize keeper creation with closing the shared admission gate.
        let admission = state
            .lock()
            .map_err(|_| io::Error::other("writer registry poisoned"))?;
        if admission.closed {
            return Err(io::Error::other("writer admission closed"));
        }
        let result = start(self);
        drop(admission);
        result
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
}

struct PreparedWriterSpawn {
    executable_name: widestring::U16CString,
    env: Vec<u16>,
    command: Vec<u16>,
    state: std::sync::Arc<std::sync::Mutex<crate::writer_lifecycle::ProductionWriters>>,
}

impl WindowsWriterJob {
    fn prepare_writer_spawn(
        &self,
        executable: &Path,
        args: &[&str],
        set: &[(String, String)],
        remove: &[String],
    ) -> io::Result<PreparedWriterSpawn> {
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
        let executable_name = widestring::U16CString::from_vec(
            executable.as_os_str().encode_wide().collect::<Vec<_>>(),
        )
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in executable"))?;
        let env = windows_environment(set, remove)?;
        let command = windows_command_line(executable, args)?;
        Ok(PreparedWriterSpawn {
            executable_name,
            env,
            command,
            state,
        })
    }

    /// Launch one registered writer without inherited pipes, atomically
    /// assigned to this job with console output suppressed.
    pub fn spawn_silent_with_environment(
        &mut self,
        executable: &Path,
        args: &[&str],
        set: &[(String, String)],
        remove: &[String],
    ) -> io::Result<()> {
        let mut prepared = self.prepare_writer_spawn(executable, args, set, remove)?;
        let admission = prepared
            .state
            .lock()
            .map_err(|_| io::Error::other("writer registry poisoned"))?;
        if admission.closed {
            return Err(io::Error::other("writer admission closed"));
        }
        let jobs = [job::Handle::from(&self.job)];
        let attributes = [process::ThreadAttributeRef::job_list(&jobs)];
        let list = process::ThreadAttributeList::try_from(attributes.as_slice())
            .map_err(io::Error::from)?;
        let startup = process::StartupInfoExW {
            attribute_list: Some(list),
            ..Default::default()
        };
        let writer = process::create_process_w(
            prepared.executable_name.as_ucstr(),
            Some(&mut prepared.command),
            None,
            None,
            false,
            process::EXTENDED_STARTUPINFO_PRESENT
                | process::CREATE_NO_WINDOW
                | process::CREATE_UNICODE_ENVIRONMENT,
            prepared.env.as_slice(),
            (),
            &startup,
        )
        .map_err(io::Error::from)?;
        self.writer = Some(writer);
        self.writer_started = true;
        drop(admission);
        drop(startup);
        Ok(())
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
        let mut prepared = self.prepare_writer_spawn(executable, args, set, remove)?;
        let admission = prepared
            .state
            .lock()
            .map_err(|_| io::Error::other("writer registry poisoned"))?;
        if admission.closed {
            return Err(io::Error::other("writer admission closed"));
        }
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
        let writer = process::create_process_w(
            prepared.executable_name.as_ucstr(),
            Some(&mut prepared.command),
            None,
            None,
            true,
            process::EXTENDED_STARTUPINFO_PRESENT
                | process::CREATE_NO_WINDOW
                | process::CREATE_UNICODE_ENVIRONMENT,
            prepared.env.as_slice(),
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

    fn mark_registered_drained(&mut self) -> io::Result<()> {
        if let Some((state, role)) = &self.production_registration {
            let mut writers = state
                .lock()
                .map_err(|_| io::Error::other("writer registry poisoned"))?;
            writers.mark_drained(role).map_err(|error| {
                io::Error::other(format!("writer drain registration rejected: {error:?}"))
            })?;
        }
        Ok(())
    }

    /// Terminate the job, wait for kernel IOCP `JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO`,
    /// verify empty process-id list and owned handles, and mark registered role drained.
    pub fn stop_and_drain(&mut self, timeout: Duration) -> io::Result<()> {
        self.admission_closed = true;
        if self.drained {
            return Ok(());
        }
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid drain timeout"))?;
        let initial: job::object::BasicAccountingInformation =
            job::query_information_job_object(&self.job).map_err(io::Error::from)?;
        if !self.writer_started
            && self.keeper.is_none()
            && initial.total_processes == 0
            && initial.active_processes == 0
        {
            job::terminate_job_object(&self.job, 1).map_err(io::Error::from)?;
            let verified: job::object::BasicAccountingInformation =
                job::query_information_job_object(&self.job).map_err(io::Error::from)?;
            if verified.total_processes != 0 || verified.active_processes != 0 {
                return Err(io::Error::other(
                    "unexpected process observed in unstarted writer job",
                ));
            }
            self.drained = true;
            self.mark_registered_drained()?;
            return Ok(());
        }
        if self.iocp.is_none() {
            self.iocp = Some(xarchive_win_job_iocp::JobCompletionPort::attach(&self.job)?);
        }
        job::terminate_job_object(&self.job, 1).map_err(io::Error::from)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        self.iocp
            .as_ref()
            .expect("completion port attached before terminate")
            .wait_active_process_zero(remaining)?;
        loop {
            let accounting: job::object::BasicAccountingInformation =
                job::query_information_job_object(&self.job).map_err(io::Error::from)?;
            let pids: job::object::BasicProcessIdList =
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
            if accounting.active_processes == 0
                && pids.number_of_assigned_processes == 0
                && pids.process_id_list.is_empty()
                && owned_processes_exited
            {
                self.drained = true;
                self.mark_registered_drained()?;
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

    /// Recover all fixed production writer roles, minting drain proof only after
    /// each recorded Job Object is reopened and drained via IOCP.
    pub fn recover_production_attempt(
        record_root: &Path,
        binding: RecoveryDrainBinding,
        timeout: Duration,
    ) -> io::Result<crate::writer_lifecycle::VerifiedRecoveryDrain> {
        for role in binding.roles {
            let identity = WindowsWriterIdentity::new(&binding.workspace, &binding.identity, role)?;
            match Self::open_recorded(record_root, identity) {
                Ok(mut job) => job.stop_and_drain(timeout)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(
            crate::writer_lifecycle::VerifiedRecoveryDrain::from_windows(VerifiedAttemptDrain {
                identity: binding.identity,
                workspace: binding.workspace,
                operation: binding.operation,
                roles: binding.roles,
                _private: (),
            }),
        )
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
    fn closing_admission_waits_for_keeper_creation() {
        use std::sync::mpsc;
        let root =
            std::env::temp_dir().join(format!("tw2tg-keeper-admission-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let coordinator = crate::coordination::windows::WindowsCoordinator::new(&root).unwrap();
        let attempt = crate::AttemptIdentity::new("keeper-admission", 1).unwrap();
        let session = crate::ExecutionReservation::acquire(
            &coordinator,
            "keeper-admission",
            &root.join("archive"),
        )
        .unwrap()
        .bind_existing_attempt(attempt.clone())
        .unwrap();
        let publication = crate::writer_lifecycle::PublicationOwner::for_session(&session);
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut job = WindowsWriterJob::create_recorded(&root, identity).unwrap();
        publication
            .registration()
            .register_windows(&mut job)
            .unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (attempted_tx, attempted_rx) = mpsc::channel();
        let (closed_tx, closed_rx) = mpsc::channel();
        std::thread::scope(|scope| {
            let job_ref = &mut job;
            let starter = scope.spawn(move || {
                job_ref.start_keeper_under_admission(|_| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
            });
            entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let closer = scope.spawn(|| {
                attempted_tx.send(()).unwrap();
                let result = publication.close_admission();
                closed_tx.send(()).unwrap();
                result
            });
            attempted_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let closed_while_starting = closed_rx.recv_timeout(Duration::from_millis(250));
            release_tx.send(()).unwrap();
            starter.join().unwrap().unwrap();
            closer.join().unwrap().unwrap();
            assert!(matches!(
                closed_while_starting,
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
        });
        assert!(
            job.start_keeper_under_admission(|_| panic!("closed admission started keeper"))
                .is_err()
        );
        drop((job, publication));
        drop(session);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }

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
        let permit = publication
            .authorize(&session)
            .expect("IOCP drain certifies descendant exit and unlocks publication");
        assert_eq!(permit.attempt(), &attempt);
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
        assert_eq!(
            publication.authorize(&session).err(),
            Some(crate::writer_lifecycle::PublicationError::IncompleteDrain)
        );
        job.stop_and_drain(Duration::from_secs(5)).unwrap();
        assert!(
            publication.authorize(&session).is_ok(),
            "unstarted job verified zero-process drain should authorize"
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

    fn start_test_keeper(job: &mut WindowsWriterJob, executable: &Path) -> io::Result<()> {
        let keeper = if let Some(desktop) = std::env::var_os("TW2TG_DESKTOP_KEEPER_EXE") {
            job.start_keeper(Path::new(&desktop), &["--xarchive-writer-keeper"])?
        } else {
            job.start_keeper(
                executable,
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Start-Sleep -Seconds 30",
                ],
            )?
        };
        job.keeper = Some(keeper);
        Ok(())
    }

    #[test]
    fn orphaned_grandchild_blocks_publication_until_iocp_drain_completes() {
        use std::io::BufRead;
        let root = std::env::temp_dir().join(format!("tw2tg-grandchild-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let coordinator = crate::coordination::windows::WindowsCoordinator::new(&root).unwrap();
        let attempt = crate::AttemptIdentity::new("grandchild-job", 1).unwrap();
        let session = crate::ExecutionReservation::acquire(
            &coordinator,
            "grandchild-job",
            &root.join("archive"),
        )
        .unwrap()
        .bind_existing_attempt(attempt.clone())
        .unwrap();
        let publication = crate::writer_lifecycle::PublicationOwner::for_session(&session);
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut job = WindowsWriterJob::create_recorded(&root, identity).unwrap();
        publication
            .registration()
            .register_windows(&mut job)
            .unwrap();
        let executable = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        start_test_keeper(&mut job, &executable).unwrap();
        let script = concat!(
            "$p = Start-Process -FilePath $PSHOME\\powershell.exe ",
            "-ArgumentList '-NoProfile','-NonInteractive','-Command','Start-Sleep -Seconds 30' ",
            "-PassThru -WindowStyle Hidden; ",
            "[Console]::Out.WriteLine($p.Id); [Console]::Out.Flush()"
        );
        let (_stdin, stdout, _stderr) = job
            .spawn_piped_with_environment(
                &executable,
                &["-NoProfile", "-NonInteractive", "-Command", script],
                &[],
                &[],
            )
            .unwrap();
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let grandchild_pid: usize = line.trim().parse().expect("grandchild PID");

        let deadline = Instant::now() + Duration::from_secs(10);
        while job.poll_writer_exit().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            job.poll_writer_exit().unwrap().is_some(),
            "direct writer must exit while grandchild stays alive"
        );
        let pids: job::object::BasicProcessIdList =
            job::query_information_job_object(&job.job).unwrap();
        assert!(
            pids.process_id_list.contains(&grandchild_pid),
            "orphaned grandchild PID {grandchild_pid} must remain contained in job: {:?}",
            pids.process_id_list
        );
        publication.close_admission().unwrap();
        assert_eq!(
            publication.authorize(&session).err(),
            Some(crate::writer_lifecycle::PublicationError::IncompleteDrain)
        );

        job.stop_and_drain(Duration::from_secs(10)).unwrap();
        let after: job::object::BasicProcessIdList =
            job::query_information_job_object(&job.job).unwrap();
        assert!(after.process_id_list.is_empty());
        assert!(
            publication.authorize(&session).is_ok(),
            "IOCP drain must authorize publication after terminating grandchild"
        );
        drop((reader, job, publication));
        drop(session);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn job_rejects_create_breakaway_from_job() {
        use std::io::BufRead;
        let root = std::env::temp_dir().join(format!("tw2tg-breakaway-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let coordinator = crate::coordination::windows::WindowsCoordinator::new(&root).unwrap();
        let attempt = crate::AttemptIdentity::new("breakaway-job", 1).unwrap();
        let session = crate::ExecutionReservation::acquire(
            &coordinator,
            "breakaway-job",
            &root.join("archive"),
        )
        .unwrap()
        .bind_existing_attempt(attempt.clone())
        .unwrap();
        let publication = crate::writer_lifecycle::PublicationOwner::for_session(&session);
        let identity = WindowsWriterIdentity::new(&root, &attempt, "sidecar").unwrap();
        let mut job = WindowsWriterJob::create_recorded(&root, identity).unwrap();
        publication
            .registration()
            .register_windows(&mut job)
            .unwrap();
        let executable = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        start_test_keeper(&mut job, &executable).unwrap();
        let script = r#"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class WinProc {
    [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
    public struct STARTUPINFO { public int cb; public string a; public string b; public string c; public int d; public int e; public int f; public int g; public int h; public int i; public int j; public int k; public short l; public short m; public IntPtr n; public IntPtr o; public IntPtr p; public IntPtr q; }
    [StructLayout(LayoutKind.Sequential)]
    public struct PROCESS_INFORMATION { public IntPtr hProcess; public IntPtr hThread; public int dwProcessId; public int dwThreadId; }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern bool CreateProcessW(string app, string cmd, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string dir, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern uint WaitForSingleObject(IntPtr handle, uint ms);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool CloseHandle(IntPtr handle);
    public static string Probe(string exe) {
        STARTUPINFO si = new STARTUPINFO();
        si.cb = Marshal.SizeOf(si);
        PROCESS_INFORMATION pi1;
        bool ok1 = CreateProcessW(exe, "cmd.exe /c exit 0", IntPtr.Zero, IntPtr.Zero, false, 0x08000000, IntPtr.Zero, null, ref si, out pi1);
        int err1 = ok1 ? 0 : Marshal.GetLastWin32Error();
        if (ok1) { WaitForSingleObject(pi1.hProcess, 5000); CloseHandle(pi1.hProcess); CloseHandle(pi1.hThread); }
        PROCESS_INFORMATION pi2;
        bool ok2 = CreateProcessW(exe, "cmd.exe /c exit 0", IntPtr.Zero, IntPtr.Zero, false, 0x09000000, IntPtr.Zero, null, ref si, out pi2);
        int err2 = ok2 ? 0 : Marshal.GetLastWin32Error();
        if (ok2) { CloseHandle(pi2.hProcess); CloseHandle(pi2.hThread); }
        return ok1 + ":" + err1 + "|" + ok2 + ":" + err2;
    }
}
'@
[Console]::Out.WriteLine([WinProc]::Probe($env:ComSpec))
[Console]::Out.Flush()
"#;
        let (_stdin, stdout, stderr) = job
            .spawn_piped_with_environment(
                &executable,
                &["-NoProfile", "-NonInteractive", "-Command", script],
                &[],
                &[],
            )
            .unwrap();
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            let mut err_reader = std::io::BufReader::new(stderr);
            let mut err_text = String::new();
            std::io::Read::read_to_string(&mut err_reader, &mut err_text).unwrap();
            panic!("breakaway probe produced empty stdout; stderr: {err_text}");
        }
        assert_eq!(
            line.trim(),
            "True:0|False:5",
            "expected normal child spawn to succeed and CREATE_BREAKAWAY_FROM_JOB to fail with ERROR_ACCESS_DENIED (5)"
        );
        publication.close_admission().unwrap();
        job.stop_and_drain(Duration::from_secs(10)).unwrap();
        assert!(publication.authorize(&session).is_ok());
        drop((reader, job, publication));
        drop(session);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recover_production_attempt_mints_verified_drain_after_crash() {
        let root =
            std::env::temp_dir().join(format!("tw2tg-recover-attempt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
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

        let coordinator = crate::coordination::windows::WindowsCoordinator::new(&root).unwrap();
        let attempt = crate::AttemptIdentity::new("crash-owner-probe", 1).unwrap();
        let destination = root.join("archive").join("123");
        let session =
            crate::ExecutionReservation::acquire(&coordinator, "crash-owner-probe", &destination)
                .unwrap()
                .bind_existing_attempt(attempt.clone())
                .unwrap();
        let binding = session.recovery_drain_binding(&root);
        let evidence =
            WindowsWriterJob::recover_production_attempt(&root, binding, Duration::from_secs(10))
                .expect("recovery must drain leftover job via IOCP");

        let journal = crate::writer_lifecycle::RecoveryJournalIdentity::new(
            &attempt,
            &destination,
            Path::new("Tweets/123"),
            &root,
            crate::writer_lifecycle::RecoveryJournalPhase::Prepared,
        );
        let permit =
            crate::writer_lifecycle::authorize_prepared_recovery(&session, evidence, journal)
                .expect("verified Windows drain must authorize prepared recovery");
        assert_eq!(permit.destination(), destination.as_path());
        drop(permit);
        drop(session);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }
}
