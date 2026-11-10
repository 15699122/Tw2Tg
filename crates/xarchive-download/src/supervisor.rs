use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::{Aria2HttpClient, DownloadError};
use xarchive_core::{ChildEnvironment, ProxyMode};

/// ENG-13: the aria2 RPC secret must never appear in the child process argv
/// (visible via Task Manager / `wmic process get CommandLine` on Windows).
/// The secret is instead written to a short-lived config file with owner-only
/// permissions and passed via `--conf-path`. The file is removed on
/// shutdown/drop. Windows ACL hardening of the temp file is validated in
/// WQ-ENG-12; on Windows the file inherits the caller's temp-dir ACL.
static SECRET_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Aria2SupervisorConfig {
    pub program: String,
    pub host: String,
    pub port: u16,
    pub rpc_secret: String,
    pub secret_file_path: Option<PathBuf>,
    pub startup_timeout: Duration,
    pub request_timeout: Duration,
    /// Per-connection connect timeout (`aria2c --connect-timeout`).
    pub connect_timeout: Duration,
    /// Per-connection idle timeout (`aria2c --timeout`).
    pub idle_timeout: Duration,
    /// Attempts per media URL (`aria2c --max-tries`).
    pub max_tries: u32,
    /// Optional HTTP/SOCKS proxy. Carries credentials when the network requires
    /// authentication, so it is passed to aria2 through the environment rather
    /// than the command line and never appears in `Debug` output.
    ///
    /// Only meaningful for [`ProxyMode::Manual`]; other modes ignore it.
    pub proxy: Option<String>,
    /// How aria2's own traffic is routed.
    pub proxy_mode: ProxyMode,
    /// Additional validated aria2 command-line options supplied per archive
    /// task. These are the user's own retry/timeout/concurrency knobs, already
    /// vetted by the Rust downloader-argument policy authority; they are
    /// appended after the application-owned options so a user value wins over an
    /// application default for an option both set, while every protected option
    /// (output identity, proxy, credentials, RPC/secret, hooks, reporting) is
    /// rejected upstream and can never appear here. The vector carries only
    /// `--option value` pairs with numeric values, so it holds no secret and is
    /// safe to show in diagnostics.
    pub user_args: Vec<String>,
}

impl std::fmt::Debug for Aria2SupervisorConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Aria2SupervisorConfig")
            .field("program", &self.program)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("rpc_secret", &"[REDACTED]")
            .field("startup_timeout", &self.startup_timeout)
            .field("request_timeout", &self.request_timeout)
            .field("connect_timeout", &self.connect_timeout)
            .field("idle_timeout", &self.idle_timeout)
            .field("max_tries", &self.max_tries)
            .field("proxy_mode", &self.proxy_mode)
            .field("proxy", &self.proxy.as_ref().map(|_| "[REDACTED]"))
            .field("user_argument_count", &self.user_args.len())
            .finish()
    }
}

impl Aria2SupervisorConfig {
    pub fn new(
        program: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        rpc_secret: impl Into<String>,
    ) -> Result<Self, DownloadError> {
        Self::build(program, host, port, rpc_secret)
    }

    /// Build a loopback-only configuration with a fresh cryptographically random
    /// RPC secret. The secret belongs to one aria2 child process and is never
    /// persisted or logged by the supervisor.
    pub fn new_with_random_secret(
        program: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Result<Self, DownloadError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| DownloadError::MissingRpcSecret)?;
        let mut rpc_secret = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            write!(&mut rpc_secret, "{byte:02x}").expect("writing into String cannot fail");
        }
        Self::build(program, host, port, rpc_secret)
    }

    fn build(
        program: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        rpc_secret: impl Into<String>,
    ) -> Result<Self, DownloadError> {
        let config = Self {
            program: program.into(),
            host: host.into(),
            port,
            rpc_secret: rpc_secret.into(),
            secret_file_path: None,
            startup_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(15),
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(60),
            max_tries: 3,
            proxy: None,
            proxy_mode: ProxyMode::System,
            user_args: Vec::new(),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    pub fn with_secret_file_path(mut self, path: PathBuf) -> Self {
        self.secret_file_path = Some(path);
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Apply the unified network timeouts and retry budget (P2-A).
    pub fn with_network(
        mut self,
        connect_timeout: Duration,
        idle_timeout: Duration,
        max_tries: u32,
    ) -> Self {
        self.connect_timeout = connect_timeout;
        self.idle_timeout = idle_timeout;
        self.max_tries = max_tries;
        self
    }

    /// Apply the configured proxy. The value is delivered to aria2 as an
    /// environment variable, never as a command-line argument.
    pub fn with_proxy(mut self, proxy: Option<impl Into<String>>) -> Self {
        self.proxy = proxy
            .map(Into::into)
            .map(|proxy| proxy.trim().to_owned())
            .filter(|proxy| !proxy.is_empty());
        self
    }

    /// Apply the routing mode.
    ///
    /// aria2 has no per-URL proxy resolution, so `System` is expressed by
    /// leaving its inherited environment untouched. `Direct` actively removes the
    /// inherited proxy variables, which is the only way to keep a `Direct`
    /// promise when the launching shell exported them.
    pub fn with_proxy_mode(mut self, mode: ProxyMode) -> Self {
        self.proxy_mode = mode;
        self
    }

    /// Attach the user's validated per-task aria2 options.
    ///
    /// The vector must already have passed the Rust downloader-argument policy
    /// authority (allowlist + value policy + protected-option rejection). This
    /// method does not re-validate; it only records them for
    /// [`Self::command_args`]. They are appended after the application-owned
    /// options so a user value overrides an application default for a shared
    /// option, and a protected option can never reach here because it was
    /// rejected before the snapshot was accepted.
    pub fn with_user_args(mut self, user_args: Vec<String>) -> Self {
        self.user_args = user_args;
        self
    }

    fn validate(&self) -> Result<(), DownloadError> {
        if self.program.trim().is_empty()
            || self.host.trim().is_empty()
            || self.rpc_secret.is_empty()
            || self.port == 0
            || self.startup_timeout.is_zero()
            || self.request_timeout.is_zero()
            || self.connect_timeout.is_zero()
            || self.idle_timeout.is_zero()
            || self.max_tries == 0
        {
            return Err(DownloadError::InvalidSupervisorConfiguration);
        }
        if self
            .proxy
            .as_deref()
            .is_some_and(|proxy| proxy.chars().any(char::is_whitespace))
        {
            return Err(DownloadError::InvalidSupervisorConfiguration);
        }
        Ok(())
    }

    pub(crate) fn command_args(&self) -> Vec<String> {
        // ENG-13: deliberately no `--rpc-secret` here; it would expose the
        // secret in the child process command line. The secret travels via
        // `--conf-path` (see `write_secret_file`).
        let mut args = vec![
            "--enable-rpc=true".into(),
            "--rpc-listen-all=false".into(),
            format!("--rpc-listen-port={}", self.port),
            format!("--connect-timeout={}", self.connect_timeout.as_secs()),
            format!("--timeout={}", self.idle_timeout.as_secs()),
            format!("--max-tries={}", self.max_tries),
            "--retry-wait=1".into(),
            "--quiet=true".into(),
        ];
        // User options come last so a user value overrides an application
        // default for an option both set (e.g. the user's --max-tries wins over
        // the network default). Every protected option was rejected upstream,
        // so nothing here can touch output identity, proxy, credentials, RPC,
        // hooks, or reporting. The RPC `--conf-path` is still appended by the
        // spawner after this vector, keeping the secret off the argv.
        args.extend(self.user_args.iter().cloned());
        args
    }

    /// Environment variables handed to the aria2 child process.
    ///
    /// aria2 falls back to `http_proxy`/`https_proxy`/`all_proxy` when the
    /// matching command-line option is absent, which keeps proxy credentials out
    /// of the process command line and out of any command-echo diagnostics.
    pub fn environment(&self) -> Vec<(String, String)> {
        ChildEnvironment::for_mode(self.proxy_mode, self.proxy.clone()).set
    }

    /// Environment variables the aria2 child process must not inherit.
    ///
    /// `Direct` needs these removals: aria2 reads them at startup, so leaving
    /// them in place would silently re-enable the proxy the user turned off.
    pub fn environment_remove(&self) -> Vec<String> {
        ChildEnvironment::for_mode(self.proxy_mode, self.proxy.clone()).remove
    }

    /// Write the RPC secret to an owner-only temp config file, returning its
    /// path. The file contains only `rpc-secret=<secret>\n`.
    pub(crate) fn write_secret_file(&self) -> Result<PathBuf, DownloadError> {
        self.validate()?;
        let mut path = self
            .secret_file_path
            .clone()
            .unwrap_or_else(std::env::temp_dir);
        if path.is_dir() {
            let unique = SECRET_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
            path.push(format!(
                "xarchive-aria2-secret-{}-{unique}.conf",
                std::process::id()
            ));
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .map_err(|error| DownloadError::Process(error.to_string()))?;
        file.write_all(format!("rpc-secret={}\n", self.rpc_secret).as_bytes())
            .map_err(|error| DownloadError::Process(error.to_string()))?;
        // ENG-13: force the secret bytes to stable storage before spawning
        // aria2; otherwise the child could read a partially written file.
        file.sync_data().map_err(|error| {
            let _ = std::fs::remove_file(&path);
            DownloadError::Process(error.to_string())
        })?;
        Ok(path)
    }
}

pub struct Aria2Supervisor {
    child: Option<Child>,
    #[cfg(target_os = "linux")]
    containment: Option<xarchive_workflow::linux_containment::LinuxContainment>,
    client: Aria2HttpClient,
    secret_file: Option<PathBuf>,
}

/// Startup failure that retains the supervisor when cleanup is incomplete.
/// Callers must keep the owner and retry cleanup before any successor writer
/// or publication is admitted.
pub struct Aria2StartupFailure {
    pub error: DownloadError,
    pub owner: Option<Aria2Supervisor>,
}

impl Aria2StartupFailure {
    /// Retry cleanup on the same retained supervisor. `Ok(false)` means
    /// startup failed but its containment was already fully retired.
    pub fn retry_cleanup(&mut self) -> Result<bool, DownloadError> {
        let Some(owner) = self.owner.as_mut() else {
            return Ok(false);
        };
        owner.shutdown()?;
        self.owner = None;
        Ok(true)
    }
}

impl std::fmt::Debug for Aria2StartupFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Aria2StartupFailure")
            .field("error", &self.error)
            .field("owner_retained", &self.owner.is_some())
            .finish()
    }
}

impl std::fmt::Debug for Aria2Supervisor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Aria2Supervisor")
            .field("running", &self.child.is_some())
            .field("client", &self.client)
            .finish()
    }
}

impl Aria2Supervisor {
    /// CROSS_PLATFORM_CHANGE_REQUIRED: opt-in Linux trusted-tool owner.
    #[cfg(target_os = "linux")]
    #[deprecated(note = "use spawn_contained_owned to retain failed startup cleanup owners")]
    pub fn spawn_contained(
        config: Aria2SupervisorConfig,
        owner: xarchive_workflow::linux_containment::LinuxContainment,
        python: &std::path::Path,
        launcher: &std::path::Path,
    ) -> Result<Self, DownloadError> {
        Self::spawn_contained_owned(config, owner, python, launcher)
            .map_err(|failure| failure.error)
    }

    /// Owned startup form for attempt coordinators which must retain an
    /// incompletely cleaned writer owner after any startup failure.
    #[cfg(target_os = "linux")]
    pub fn spawn_contained_owned(
        config: Aria2SupervisorConfig,
        owner: xarchive_workflow::linux_containment::LinuxContainment,
        python: &std::path::Path,
        launcher: &std::path::Path,
    ) -> Result<Self, Aria2StartupFailure> {
        // The group already exists when this entry point is called. Even
        // pre-spawn validation failures must explicitly retire its owner.
        let initialization = (|| {
            config.validate()?;
            let client = Aria2HttpClient::new(&config.host, config.port, &config.rpc_secret)?
                .with_timeout(config.request_timeout);
            let secret = config.write_secret_file()?;
            Ok::<_, DownloadError>((client, secret))
        })();
        let (client, secret) = match initialization {
            Ok(initialized) => initialized,
            Err(error) => {
                let client = Aria2HttpClient::new(&config.host, config.port, &config.rpc_secret)
                    .unwrap_or_else(|_| {
                        Aria2HttpClient::new("127.0.0.1", 1, "cleanup-owner")
                            .expect("static cleanup client configuration")
                    });
                let mut supervisor = Self {
                    child: None,
                    containment: Some(owner),
                    client,
                    secret_file: None,
                };
                return match supervisor.shutdown() {
                    Ok(()) => Err(Aria2StartupFailure { error, owner: None }),
                    Err(cleanup) => Err(Aria2StartupFailure {
                        error: DownloadError::Process(format!(
                            "initialize: {error}; cleanup: {cleanup}"
                        )),
                        owner: Some(supervisor),
                    }),
                };
            }
        };
        let mut supervisor = Self {
            child: None,
            containment: Some(owner),
            client,
            secret_file: Some(secret.clone()),
        };
        let result = (|| {
            let mut args = config.command_args();
            args.push(format!("--conf-path={}", secret.to_string_lossy()));
            let mut remove = config.environment_remove();
            if matches!(config.proxy_mode, ProxyMode::Direct) {
                remove.extend(
                    std::env::vars_os()
                        .map(|(key, _)| key.to_string_lossy().into_owned())
                        .filter(|key| xarchive_core::is_proxy_environment_key(key)),
                );
            }
            supervisor
                .containment
                .as_mut()
                .expect("owned containment")
                .spawn_silent_with_environment(
                    python,
                    launcher,
                    std::path::Path::new(&config.program),
                    &args,
                    &config.environment(),
                    &remove,
                )
                .map_err(|error| DownloadError::Process(error.to_string()))?;
            let deadline = Instant::now() + config.startup_timeout;
            loop {
                if !supervisor.is_running()? {
                    return Err(DownloadError::SupervisorNotRunning);
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(DownloadError::Process(
                        "contained aria2 startup timeout".into(),
                    ));
                }
                let startup_client =
                    Aria2HttpClient::new(&config.host, config.port, &config.rpc_secret)?
                        .with_timeout(config.request_timeout.min(remaining));
                if startup_client.get_version().is_ok() {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })();
        match result {
            Ok(()) => Ok(supervisor),
            Err(error) => match supervisor.shutdown() {
                Ok(()) => Err(Aria2StartupFailure { error, owner: None }),
                Err(cleanup) => Err(Aria2StartupFailure {
                    error: DownloadError::Process(format!("start: {error}; cleanup: {cleanup}")),
                    owner: Some(supervisor),
                }),
            },
        }
    }
    #[cfg(test)]
    pub(crate) fn with_secret_file_for_test(client: Aria2HttpClient, secret_file: PathBuf) -> Self {
        Self {
            child: None,
            #[cfg(target_os = "linux")]
            containment: None,
            client,
            secret_file: Some(secret_file),
        }
    }

    pub fn spawn(config: Aria2SupervisorConfig) -> Result<Self, DownloadError> {
        config.validate()?;
        let client = Aria2HttpClient::new(&config.host, config.port, &config.rpc_secret)?
            .with_timeout(config.request_timeout);
        // ENG-13: write the secret to a short-lived config file BEFORE spawn
        // so a spawn failure never leaves the secret file behind.
        let secret_file = config.write_secret_file()?;
        let mut command = Command::new(&config.program);
        hide_console_window(&mut command);
        configure_process_group(&mut command);
        for (key, value) in config.environment() {
            command.env(key, value);
        }
        // `Direct` must also drop what the launching shell exported; setting an
        // empty value would not stop aria2 from treating a proxy as configured.
        // The case-insensitive sweep matters on Windows, where a mixed spelling
        // such as `Http_Proxy` would otherwise survive.
        for key in config.environment_remove() {
            command.env_remove(&key);
        }
        if matches!(config.proxy_mode, ProxyMode::Direct) {
            for key in std::env::vars_os().map(|(key, _)| key) {
                let name = key.to_string_lossy().into_owned();
                if xarchive_core::is_proxy_environment_key(&name) {
                    command.env_remove(&name);
                }
            }
        }
        let mut child = command
            .args(config.command_args())
            .arg(format!(
                "--conf-path={}",
                secret_file.to_string_lossy().as_ref()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                let _ = std::fs::remove_file(&secret_file);
                DownloadError::Process(error.to_string())
            })?;
        if let Err(error) = wait_for_aria2(&mut child, &client, config.startup_timeout) {
            terminate_process_tree(&mut child);
            let _ = child.wait();
            let _ = std::fs::remove_file(&secret_file);
            return Err(error);
        }
        Ok(Self {
            child: Some(child),
            #[cfg(target_os = "linux")]
            containment: None,
            client,
            secret_file: Some(secret_file),
        })
    }

    pub fn backend(&self) -> &Aria2HttpClient {
        &self.client
    }

    pub fn get_version(&self) -> Result<String, DownloadError> {
        self.client.get_version()
    }

    pub fn is_running(&mut self) -> Result<bool, DownloadError> {
        #[cfg(target_os = "linux")]
        if let Some(owner) = self.containment.as_mut() {
            let running = owner
                .poll_exit()
                .map_err(|error| DownloadError::Process(error.to_string()))?
                .is_none();
            if !running {
                self.shutdown()?;
            }
            return Ok(running);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(false);
        };
        let running = child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|error| DownloadError::Process(error.to_string()))?;
        if !running {
            // Polling is also a terminal observation: callers need not follow
            // it with wait_for_exit to stop group descendants or remove secrets.
            self.shutdown()?;
        }
        Ok(running)
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<bool, DownloadError> {
        #[cfg(target_os = "linux")]
        if self.containment.is_some() {
            let deadline = Instant::now() + timeout;
            loop {
                if !self.is_running()? {
                    return Ok(true);
                }
                if Instant::now() >= deadline {
                    return Ok(false);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if self.child.is_none() {
            self.shutdown()?;
            return Ok(true);
        }
        let deadline = Instant::now() + timeout;
        loop {
            let child = self
                .child
                .as_mut()
                .ok_or(DownloadError::SupervisorNotRunning)?;
            if child
                .try_wait()
                .map_err(|error| DownloadError::Process(error.to_string()))?
                .is_some()
            {
                self.shutdown()?;
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Stop aria2 and remove its short-lived secret file.
    ///
    /// This reports the direct child and secret-file cleanup outcome; it does
    /// not prove escaped descendants have exited.
    pub fn shutdown(&mut self) -> Result<(), DownloadError> {
        let mut failures = Vec::new();
        #[cfg(target_os = "linux")]
        if let Some(owner) = self.containment.as_mut() {
            match owner
                .stop_and_drain(Duration::from_secs(5))
                .and_then(|()| owner.remove_group())
            {
                Ok(()) => self.containment = None,
                Err(error) => failures.push(format!("aria2 containment cleanup: {error}")),
            }
        }
        // Do not relinquish the direct Child while containment still reports
        // incomplete drain/removal. The retained supervisor is the retry handle
        // for the whole process owner, not merely for the secret-file cleanup.
        #[cfg(target_os = "linux")]
        if self.containment.is_some() {
            return Err(DownloadError::Process(failures.join("; ")));
        }
        if self.child.is_some() {
            // Terminate the process group while the Child handle remains owned;
            // `wait` below is the only operation that can release that handle.
            if let Some(child) = self.child.as_mut() {
                terminate_process_tree(child);
            }
        }
        if let Some(child) = self.child.as_mut() {
            terminate_process_tree(child);
            match child.wait() {
                Ok(_) => self.child = None,
                Err(error) => failures.push(format!("wait for aria2 process: {error}")),
            }
        }
        if self.child.is_some() {
            // Keep the child handle available for an explicit subsequent retry.
            if failures.is_empty() {
                failures.push("aria2 child remains owned after shutdown".into());
            }
        }
        if self.child.is_none() {
            // ENG-13: remove the short-lived secret file only after process
            // teardown succeeds so a retry never loses the owner's credential.
            if let Some(path) = self.secret_file.take()
                && let Err(error) = std::fs::remove_file(&path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                failures.push(format!("remove aria2 secret file: {error}"));
                self.secret_file = Some(path);
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(DownloadError::Process(failures.join("; ")))
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

// CROSS_PLATFORM_CHANGE_REQUIRED: Unix writers are isolated from the Desktop
// process group and their descendants are stopped on every terminal path.
fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
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
        let group = Pid::from_raw(-(child.id() as i32));
        let _ = kill(group, Signal::SIGTERM);
        std::thread::sleep(Duration::from_millis(25));
        let _ = kill(group, Signal::SIGKILL);
    }
    let _ = child.kill();
}

impl Drop for Aria2Supervisor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn wait_for_aria2(
    child: &mut Child,
    client: &Aria2HttpClient,
    timeout: Duration,
) -> Result<(), DownloadError> {
    let deadline = Instant::now() + timeout;
    loop {
        if child
            .try_wait()
            .map_err(|error| DownloadError::Process(error.to_string()))?
            .is_some()
        {
            return Err(DownloadError::Process(
                "aria2 exited before its RPC endpoint became ready".to_owned(),
            ));
        }
        match client.get_version() {
            Ok(_) => return Ok(()),
            Err(_error) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => {
                return Err(DownloadError::StartupTimeout {
                    last_error: error.to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    mod command_line;
    mod debug_redaction;
    mod network_proxy;
    mod secret_lifecycle;

    fn supervisor_with_secret_file(
        client: Aria2HttpClient,
        secret_file: PathBuf,
    ) -> Aria2Supervisor {
        Aria2Supervisor {
            child: None,
            #[cfg(target_os = "linux")]
            containment: None,
            client,
            secret_file: Some(secret_file),
        }
    }

    fn config() -> Aria2SupervisorConfig {
        Aria2SupervisorConfig::new("aria2c", "127.0.0.1", 6800, "rpc-secret-value")
            .expect("valid supervisor config")
    }
}
