//! Process supervision for the Python JSONL Sidecar.
//!
//! This crate intentionally owns only process and stream mechanics. Archive
//! state, retries, persistence, and the decision that a job is complete stay
//! in the Desktop application.

use std::io;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::thread;
use std::time::Duration;

use xarchive_protocol::{
    SIDECAR_PROTOCOL_VERSION, SidecarV2Command, SidecarV2EventType, write_json_line,
};

mod error;
mod events;
mod readers;

pub use error::SupervisorError;
pub use events::SupervisorEvent;

/// The environment changes a child process needs before it starts.
///
/// Removals matter as much as additions: a process that inherits `HTTP_PROXY`
/// cannot be made direct by setting an empty value, and some tools treat an
/// empty proxy variable as a configured one.
#[derive(Clone, Debug, Default)]
pub struct ChildProcessEnvironment {
    pub set: Vec<(String, String)>,
    pub remove: Vec<String>,
}

impl ChildProcessEnvironment {
    pub fn new(set: Vec<(String, String)>, remove: Vec<String>) -> Self {
        Self { set, remove }
    }
}

impl From<&[(String, String)]> for ChildProcessEnvironment {
    fn from(set: &[(String, String)]) -> Self {
        Self {
            set: set.to_vec(),
            remove: Vec::new(),
        }
    }
}

/// Startup failure that retains the supervisor owner when cleanup is
/// incomplete.
///
/// Mirrors the aria2 `Aria2StartupFailure` contract. Callers must keep the
/// returned owner and retry cleanup before admitting any successor writer or
/// publication. The owner is cleared only after a successful shutdown; a `None`
/// owner means startup failed but its containment was already fully retired.
pub struct SidecarStartupFailure {
    pub error: SupervisorError,
    pub owner: Option<SidecarSupervisor>,
}

impl SidecarStartupFailure {
    /// Retry cleanup on the same retained owner. `Ok(false)` means startup
    /// failed but its containment was already fully retired.
    pub fn retry_cleanup(&mut self) -> Result<bool, SupervisorError> {
        let Some(owner) = self.owner.as_mut() else {
            return Ok(false);
        };
        owner.shutdown()?;
        self.owner = None;
        Ok(true)
    }
}

impl std::fmt::Debug for SidecarStartupFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SidecarStartupFailure")
            .field("error", &self.error)
            .field("owner_retained", &self.owner.is_some())
            .finish()
    }
}

pub struct SidecarSupervisor {
    child: Option<Child>,
    #[cfg(target_os = "linux")]
    containment: Option<xarchive_workflow::linux_containment::LinuxContainment>,
    stdin: Option<ChildStdin>,
    events: Receiver<SupervisorEvent>,
    readers: Vec<thread::JoinHandle<()>>,
}
impl SidecarSupervisor {
    /// CROSS_PLATFORM_CHANGE_REQUIRED: opt-in trusted-tool containment.
    /// The caller supplies an exclusively owned group; shutdown retains it
    /// until kill/drain and group removal succeed.
    #[cfg(target_os = "linux")]
    pub fn spawn_contained(
        owner: xarchive_workflow::linux_containment::LinuxContainment,
        python: &std::path::Path,
        launcher: &std::path::Path,
        program: &std::path::Path,
        args: &[&str],
    ) -> Result<Self, SidecarStartupFailure> {
        Self::spawn_contained_with_environment(
            owner,
            python,
            launcher,
            program,
            args,
            &ChildProcessEnvironment::default(),
        )
    }

    /// Build a cleanup-only supervisor around a retained containment owner.
    ///
    /// Used when a contained spawn fails before the real supervisor is
    /// constructed: the writer domain still needs an owner that can retry
    /// kill/drain and group removal, and that owner must not be dropped into a
    /// plain error value.
    #[cfg(target_os = "linux")]
    fn containment_shell(owner: xarchive_workflow::linux_containment::LinuxContainment) -> Self {
        let (_sender, events) = mpsc::channel();
        Self {
            child: None,
            containment: Some(owner),
            stdin: None,
            events,
            readers: Vec::new(),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn spawn_contained_with_environment(
        mut owner: xarchive_workflow::linux_containment::LinuxContainment,
        python: &std::path::Path,
        launcher: &std::path::Path,
        program: &std::path::Path,
        args: &[&str],
        environment: &ChildProcessEnvironment,
    ) -> Result<Self, SidecarStartupFailure> {
        let mut remove = environment.remove.clone();
        if !remove.is_empty() {
            remove.extend(
                std::env::vars_os()
                    .map(|(key, _)| key.to_string_lossy().into_owned())
                    .filter(|key| xarchive_core::is_proxy_environment_key(key)),
            );
        }
        let pipes = owner.spawn_piped_with_environment(
            python,
            launcher,
            program,
            args,
            &environment.set,
            &remove,
        );
        let (stdin, stdout, stderr) = match pipes {
            Ok(pipes) => pipes,
            Err(error) => {
                // The domain was created but no supervisor was built yet. Wrap
                // the retained owner so a failed cleanup keeps the recovery
                // handle instead of dropping it into a plain error string.
                let mut shell = Self::containment_shell(owner);
                return Err(match shell.shutdown() {
                    Ok(()) => SidecarStartupFailure {
                        error: SupervisorError::Spawn(error),
                        owner: None,
                    },
                    Err(cleanup) => SidecarStartupFailure {
                        error: SupervisorError::ShutdownIncomplete(format!(
                            "start: {error}; cleanup: {cleanup}"
                        )),
                        owner: Some(shell),
                    },
                });
            }
        };
        let (sender, events) = mpsc::channel();
        let readers = vec![
            readers::spawn_stdout_reader(stdout, sender.clone()),
            readers::spawn_stderr_reader(stderr, sender),
        ];
        Ok(Self {
            child: None,
            containment: Some(owner),
            stdin: Some(stdin),
            events,
            readers,
        })
    }
    pub fn spawn(program: &str, args: &[&str]) -> Result<Self, SupervisorError> {
        Self::spawn_with_env(program, args, &[])
    }

    pub fn spawn_with_env(
        program: &str,
        args: &[&str],
        env: &[(String, String)],
    ) -> Result<Self, SupervisorError> {
        Self::spawn_with_environment(program, args, &env.into())
    }

    /// Spawn with explicit additions and removals.
    ///
    /// Removals are applied after additions, so a key listed in both is removed.
    /// That ordering keeps a "must not be proxied" guarantee from being undone
    /// by an inherited or duplicated setting.
    pub fn spawn_with_environment(
        program: &str,
        args: &[&str],
        environment: &ChildProcessEnvironment,
    ) -> Result<Self, SupervisorError> {
        let mut command = Command::new(program);
        hide_console_window(&mut command);
        command.args(args).envs(
            environment
                .set
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
        for key in &environment.remove {
            command.env_remove(key);
        }
        // Windows environment names are case-insensitive, so a mixed spelling
        // inherited from the launching shell would survive the exact-match
        // removals above and re-enable the proxy in the child. This runs only
        // when a removal was requested; `System` must keep what it inherited.
        if !environment.remove.is_empty() {
            for key in std::env::vars_os().map(|(key, _)| key) {
                let name = key.to_string_lossy().into_owned();
                if xarchive_core::is_proxy_environment_key(&name) {
                    command.env_remove(&name);
                }
            }
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        configure_process_group(&mut command);
        let mut child = command.spawn().map_err(SupervisorError::Spawn)?;
        let stdin = child.stdin.take().ok_or_else(|| {
            SupervisorError::Spawn(io::Error::other("sidecar stdin was not piped"))
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            SupervisorError::Spawn(io::Error::other("sidecar stdout was not piped"))
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            SupervisorError::Spawn(io::Error::other("sidecar stderr was not piped"))
        })?;

        let (sender, events) = mpsc::channel();
        let readers = vec![
            readers::spawn_stdout_reader(stdout, sender.clone()),
            readers::spawn_stderr_reader(stderr, sender),
        ];

        Ok(Self {
            child: Some(child),
            #[cfg(target_os = "linux")]
            containment: None,
            stdin: Some(stdin),
            events,
            readers,
        })
    }

    pub fn send_v2(&mut self, command: &SidecarV2Command) -> Result<(), SupervisorError> {
        let stdin = self.stdin.as_mut().ok_or(SupervisorError::NotRunning)?;
        write_json_line(stdin, command).map_err(SupervisorError::Send)
    }

    pub fn send_v2_cancel(
        &mut self,
        request_id: impl Into<String>,
        job_id: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        self.send_v2(&SidecarV2Command {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: xarchive_protocol::SidecarV2CommandType::Cancel,
            job_id: job_id.into(),
            url: None,
            browser: None,
            profile: None,
            staging_dir: None,
            user_args: Vec::new(),
        })
    }

    pub fn send_v2_shutdown(
        &mut self,
        request_id: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        self.send_v2(&SidecarV2Command {
            protocol_version: SIDECAR_PROTOCOL_VERSION,
            request_id: request_id.into(),
            cmd: xarchive_protocol::SidecarV2CommandType::Shutdown,
            job_id: "system".to_owned(),
            url: None,
            browser: None,
            profile: None,
            staging_dir: None,
            user_args: Vec::new(),
        })
    }

    /// Request account discovery for one profile URL.
    ///
    /// The batch id travels as the v2 `job_id` so every candidate event of this
    /// discovery run is fenced to one batch without a separate correlation
    /// channel.
    pub fn send_v2_discover(
        &mut self,
        request_id: impl Into<String>,
        batch_id: impl Into<String>,
        profile_url: impl Into<String>,
        browser: Option<String>,
        profile: Option<String>,
    ) -> Result<(), SupervisorError> {
        self.send_v2(&SidecarV2Command::discover(
            request_id,
            batch_id,
            profile_url,
            browser,
            profile,
        ))
    }

    /// Request a gallery-dl download with task-scoped user argv entries.
    /// Policy validation belongs to the Rust task/settings boundary; this
    /// method only carries the already-validated snapshot across the protocol.
    #[allow(clippy::too_many_arguments)] // Mirrors SidecarV2Command::download's own fields one-for-one.
    pub fn send_v2_download(
        &mut self,
        request_id: impl Into<String>,
        job_id: impl Into<String>,
        url: impl Into<String>,
        browser: Option<String>,
        profile: Option<String>,
        staging_dir: impl Into<String>,
        user_args: Vec<String>,
    ) -> Result<(), SupervisorError> {
        self.send_v2(&SidecarV2Command::download(
            request_id,
            job_id,
            url,
            browser,
            profile,
            staging_dir,
            user_args,
        ))
    }

    /// Spawn a sidecar and require the capability-bearing `hello -> ready`
    /// handshake.
    ///
    /// The stdout reader only accepts protocol v2 events, so a worker that
    /// answers with a legacy protocol line fails the handshake instead of being
    /// accepted as a fallback.
    pub fn spawn_ready_v2(
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<Self, SupervisorError> {
        Self::spawn_ready_v2_with_env(program, args, &[], timeout)
    }

    pub fn spawn_ready_v2_with_env(
        program: &str,
        args: &[&str],
        env: &[(String, String)],
        timeout: Duration,
    ) -> Result<Self, SupervisorError> {
        Self::spawn_ready_v2_with_environment(program, args, &env.into(), timeout)
    }

    /// Spawn with explicit environment additions and removals, then require the
    /// capability-bearing handshake.
    pub fn spawn_ready_v2_with_environment(
        program: &str,
        args: &[&str],
        environment: &ChildProcessEnvironment,
        timeout: Duration,
    ) -> Result<Self, SupervisorError> {
        let supervisor = Self::spawn_with_environment(program, args, environment)?;
        supervisor.require_ready_v2(timeout)
    }

    /// Reuses the identical handshake and explicit cleanup for contained workers.
    ///
    /// This is the legacy error surface: a failed cleanup drops the owner. Use
    /// [`Self::require_ready_v2_owned`] for the production contained path, where
    /// a retained owner must be retried before any successor writer.
    pub fn require_ready_v2(self, timeout: Duration) -> Result<Self, SupervisorError> {
        match self.require_ready_v2_owned(timeout) {
            Ok(supervisor) => Ok(supervisor),
            Err(failure) => Err(failure.error),
        }
    }

    /// Handshake that retains the supervisor owner when cleanup is incomplete.
    ///
    /// A failed handshake must not drop the containment owner: the caller keeps
    /// the returned owner and retries cleanup before admitting a successor
    /// writer or publication. [`SidecarStartupFailure::retry_cleanup`] clears
    /// the owner only after a successful shutdown.
    pub fn require_ready_v2_owned(
        mut self,
        timeout: Duration,
    ) -> Result<Self, SidecarStartupFailure> {
        match self.handshake_v2(timeout) {
            Ok(()) => Ok(self),
            Err(error) => match self.shutdown() {
                Ok(()) => Err(SidecarStartupFailure { error, owner: None }),
                Err(cleanup) => Err(SidecarStartupFailure {
                    error: SupervisorError::ShutdownIncomplete(format!(
                        "handshake: {error}; cleanup: {cleanup}"
                    )),
                    owner: Some(self),
                }),
            },
        }
    }

    fn handshake_v2(&mut self, timeout: Duration) -> Result<(), SupervisorError> {
        let supervisor = self;
        supervisor.send_v2(&xarchive_protocol::SidecarV2Command::hello(
            "desktop-hello-v2",
        ))?;
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return match supervisor.shutdown() {
                    Ok(()) => Err(SupervisorError::HandshakeTimeout),
                    Err(error) => Err(error),
                };
            }
            match supervisor.recv_timeout(remaining)? {
                Some(SupervisorEvent::V2(event)) if event.event == SidecarV2EventType::Ready => {
                    event
                        .validate()
                        .map_err(|error| SupervisorError::HandshakeFailed(error.to_string()))?;
                    if event.protocol_version != SIDECAR_PROTOCOL_VERSION
                        || event.job_id != "system"
                        || event.request_id.as_deref() != Some("desktop-hello-v2")
                        || !xarchive_protocol::has_required_capabilities(
                            event.capabilities.as_deref().unwrap_or(&[]),
                        )
                    {
                        return match supervisor.shutdown() {
                            Ok(()) => Err(SupervisorError::HandshakeFailed(
                                "invalid v2 ready identity or capabilities".to_owned(),
                            )),
                            Err(error) => Err(error),
                        };
                    }
                    return Ok(());
                }
                Some(SupervisorEvent::Exited(result)) => {
                    return Err(SupervisorError::HandshakeFailed(format!(
                        "sidecar exited during v2 handshake: {result:?}"
                    )));
                }
                Some(SupervisorEvent::ProtocolError { message, .. }) => {
                    return match supervisor.shutdown() {
                        Ok(()) => Err(SupervisorError::HandshakeFailed(message)),
                        Err(error) => Err(error),
                    };
                }
                Some(_) | None => {}
            }
        }
    }

    pub fn try_recv(&self) -> Result<Option<SupervisorEvent>, SupervisorError> {
        match self.events.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(SupervisorError::NotRunning),
        }
    }

    /// Wait for one event without blocking indefinitely.
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<SupervisorEvent>, SupervisorError> {
        match self.events.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(SupervisorError::NotRunning),
        }
    }

    pub fn is_running(&mut self) -> Result<bool, SupervisorError> {
        Ok(self.poll_exit()?.is_none())
    }

    /// Check whether the child has exited without blocking the Desktop event loop.
    pub fn poll_exit(&mut self) -> Result<Option<std::process::ExitStatus>, SupervisorError> {
        #[cfg(target_os = "linux")]
        if let Some(owner) = self.containment.as_mut() {
            return owner.poll_exit().map_err(SupervisorError::Spawn);
        }
        let child = self.child.as_mut().ok_or(SupervisorError::NotRunning)?;
        child.try_wait().map_err(SupervisorError::Spawn)
    }

    pub fn close_stdin(&mut self) {
        self.stdin.take();
    }

    /// Stop the process and join owned output readers within the shutdown bound.
    ///
    /// This reports local process/reader cleanup only; it does not prove that
    /// descendants escaped from the process group have exited.
    pub fn shutdown(&mut self) -> Result<(), SupervisorError> {
        self.close_stdin();
        let mut failures = Vec::new();
        #[cfg(target_os = "linux")]
        if let Some(owner) = self.containment.as_mut() {
            match owner
                .stop_and_drain(Duration::from_secs(5))
                .and_then(|()| owner.remove_group())
            {
                Ok(()) => self.containment = None,
                Err(error) => failures.push(format!("containment cleanup: {error}")),
            }
        }
        if let Some(mut child) = self.child.take() {
            terminate_process_tree(&mut child);
            if let Err(error) = child.wait() {
                failures.push(format!("wait for Sidecar process: {error}"));
            }
        }
        // Readers remain owned even when a descendant retains a pipe. Never
        // block shutdown indefinitely on a writer outside the process group.
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while self.readers.iter().any(|reader| !reader.is_finished())
            && std::time::Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(10));
        }
        let mut pending = Vec::new();
        for reader in self.readers.drain(..) {
            if reader.is_finished() {
                if reader.join().is_err() {
                    failures.push("Sidecar output reader panicked".to_owned());
                }
            } else {
                pending.push(reader);
            }
        }
        self.readers = pending;
        if !self.readers.is_empty() {
            failures.push("output readers are still running".to_owned());
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(SupervisorError::ShutdownIncomplete(failures.join("; ")))
        }
    }

    /// A stopped direct child alone is insufficient: pipe readers may still
    /// be held open by descendant writers. This is not process-tree proof.
    pub fn readers_finished(&self) -> bool {
        self.readers.iter().all(|reader| reader.is_finished())
    }

    /// Wait briefly for a protocol shutdown before using the forceful fallback.
    /// `true` requires both direct-child exit and completed pipe readers, but
    /// does not authorize publication or prove escaped descendants have exited.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<bool, SupervisorError> {
        let has_process = self.child.is_some();
        #[cfg(target_os = "linux")]
        let has_process = has_process || self.containment.is_some();
        if !has_process {
            return self.shutdown().map(|()| true);
        }
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if self.poll_exit()?.is_some() {
                // Preserve the process-group identity until descendants have
                // been signalled, even when the group leader has already exited.
                return self.shutdown().map(|()| true);
            }
            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn hide_console_window(command: &mut Command) {
    #[cfg(not(target_os = "windows"))]
    let _ = command;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
}

fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // The Sidecar and every child it starts must be isolated from the
        // Desktop process group so cancellation cannot leave gallery-dl or a
        // helper process writing into a released staging directory.
        command.process_group(0);
    }
    #[cfg(not(unix))]
    let _ = command;
}

fn terminate_process_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;

        let process_group = Pid::from_raw(-(child.id() as i32));
        // Best effort: the direct child is still killed below if the group is
        // already gone or the platform refuses the signal.
        let _ = kill(process_group, Signal::SIGTERM);
        thread::sleep(Duration::from_millis(25));
        let _ = kill(process_group, Signal::SIGKILL);
    }
    let _ = child.kill();
}

impl Drop for SidecarSupervisor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;

    #[cfg(unix)]
    #[test]
    fn shutdown_joins_owned_pipe_readers_and_is_idempotent() {
        let mut supervisor =
            SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).expect("shell fixture");
        assert_eq!(supervisor.readers.len(), 2);
        supervisor.shutdown().expect("shutdown joins readers");
        assert!(supervisor.child.is_none());
        assert!(supervisor.readers.is_empty(), "both readers joined");
        assert!(supervisor.readers_finished());
        supervisor.shutdown().expect("repeat shutdown");
    }

    #[cfg(unix)]
    #[test]
    fn exit_wait_does_not_report_success_with_an_unfinished_reader() {
        let mut supervisor =
            SidecarSupervisor::spawn("/bin/sh", &["-c", "exit 0"]).expect("shell fixture");
        let (release, blocked) = mpsc::channel();
        supervisor.readers.push(thread::spawn(move || {
            let _ = blocked.recv();
        }));
        assert!(matches!(
            supervisor.wait_for_exit(Duration::from_secs(2)),
            Err(SupervisorError::ShutdownIncomplete(message))
                if message.contains("output readers are still running")
        ));
        assert!(supervisor.child.is_none());
        assert!(!supervisor.readers_finished());
        release.send(()).expect("release reader");
        assert!(
            supervisor
                .wait_for_exit(Duration::from_secs(2))
                .expect("retry wait")
        );
        assert!(supervisor.readers.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn shutdown_reports_reader_join_panic() {
        let mut supervisor =
            SidecarSupervisor::spawn("/bin/sh", &["-c", "exit 0"]).expect("shell fixture");
        let _ = supervisor
            .child
            .as_mut()
            .expect("child")
            .wait()
            .expect("wait");
        supervisor
            .readers
            .push(thread::spawn(|| panic!("reader fixture panic")));
        assert!(matches!(
            supervisor.shutdown(),
            Err(SupervisorError::ShutdownIncomplete(message))
                if message.contains("reader panicked")
        ));
        assert!(supervisor.readers.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn exit_wait_timeout_preserves_the_running_child() {
        let mut supervisor =
            SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).expect("shell fixture");
        assert!(!supervisor.wait_for_exit(Duration::ZERO).expect("timeout"));
        assert!(supervisor.child.is_some());
        supervisor.shutdown().expect("shutdown after timeout");
        assert!(supervisor.wait_for_exit(Duration::ZERO).expect("stopped"));
    }

    #[cfg(unix)]
    #[test]
    fn parent_exit_still_closes_descendant_owned_pipes() {
        let mut supervisor = SidecarSupervisor::spawn("/bin/sh", &["-c", "sleep 30 & exit 0"])
            .expect("parent and descendant fixture");
        assert!(
            supervisor
                .wait_for_exit(Duration::from_secs(2))
                .expect("parent exits")
        );
        assert!(supervisor.child.is_none());
        assert!(
            supervisor.readers.is_empty(),
            "descendant pipes closed and readers joined"
        );
    }

    #[test]
    fn parses_v2_event_from_json() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader(
            "{\"protocol_version\":2,\"event\":\"extraction_started\",\"job_id\":\"job-1\",\"request_id\":\"request-1\"}\n".as_bytes(),
            sender,
        );

        match receiver.recv().expect("v2 event") {
            SupervisorEvent::V2(event) => {
                assert_eq!(event.protocol_version, 2);
                assert_eq!(
                    event.event,
                    xarchive_protocol::SidecarV2EventType::ExtractionStarted
                );
                assert_eq!(event.job_id, "job-1");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn rejects_legacy_protocol_line_as_protocol_error() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader(
            "{\"protocol_version\":1,\"event\":\"ready\",\"job_id\":\"system\"}\n".as_bytes(),
            sender,
        );

        match receiver.recv().expect("protocol error") {
            SupervisorEvent::ProtocolError { line, message } => {
                assert!(message.contains("unsupported sidecar protocol version"));
                assert!(line.contains("\"protocol_version\":1"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn reports_invalid_json_without_stopping_reader() {
        let (sender, receiver) = mpsc::channel();
        let input = concat!(
            "not-json\n",
            "{\"protocol_version\":2,\"event\":\"extraction_started\",\"job_id\":\"job-1\",\"request_id\":\"request-1\"}\n"
        );
        readers::spawn_stdout_reader(input.as_bytes(), sender);

        assert!(matches!(
            receiver.recv().expect("protocol error"),
            SupervisorEvent::ProtocolError { .. }
        ));
        assert!(matches!(
            receiver.recv().expect("v2 event"),
            SupervisorEvent::V2(_)
        ));
    }

    #[test]
    fn reports_missing_protocol_version() {
        let (sender, receiver) = mpsc::channel();
        readers::spawn_stdout_reader("{\"event\":\"ready\"}\n".as_bytes(), sender);

        match receiver.recv().expect("protocol error") {
            SupervisorEvent::ProtocolError { message, .. } => {
                assert!(message.contains("missing protocol_version"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn spawn_ready_v2_completes_the_capability_handshake() {
        let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
        let script = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] == 'hello':
        print(json.dumps({'protocol_version': 2, 'event': 'ready', 'job_id': 'system', 'request_id': command['request_id'], 'capabilities': ['extract_media', 'download_media', 'cancel_active_extraction', 'structured_media_plan', 'account_discovery']}), flush=True)
    elif command['cmd'] == 'shutdown':
        break
"#;
        let mut supervisor = match SidecarSupervisor::spawn_ready_v2(
            &python,
            &["-c", script],
            Duration::from_secs(5),
        ) {
            Ok(supervisor) => supervisor,
            Err(SupervisorError::Spawn(_)) => return,
            Err(error) => panic!("unexpected supervisor error: {error}"),
        };

        assert!(supervisor.is_running().expect("sidecar running"));
        supervisor
            .send_v2_shutdown("desktop-shutdown")
            .expect("send shutdown");
        supervisor.close_stdin();
        assert!(
            supervisor
                .wait_for_exit(Duration::from_secs(2))
                .expect("wait for exit")
        );
    }

    #[test]
    fn spawn_ready_v2_rejects_worker_without_required_capabilities() {
        let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
        let script = r#"
import json, sys
for line in sys.stdin:
    command = json.loads(line)
    if command['cmd'] == 'hello':
        print(json.dumps({'protocol_version': 2, 'event': 'ready', 'job_id': 'system', 'request_id': command['request_id'], 'capabilities': ['extract_media']}), flush=True)
    elif command['cmd'] == 'shutdown':
        break
"#;
        match SidecarSupervisor::spawn_ready_v2(&python, &["-c", script], Duration::from_secs(5)) {
            Ok(mut supervisor) => {
                supervisor.shutdown().expect("shutdown rejected worker");
                panic!("worker without required capabilities must be rejected");
            }
            Err(SupervisorError::Spawn(_)) => {}
            Err(SupervisorError::HandshakeFailed(message)) => {
                assert!(
                    message.contains("capabilities"),
                    "unexpected message: {message}"
                );
            }
            Err(error) => panic!("unexpected supervisor error: {error}"),
        }
    }
}
