use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::{Aria2HttpClient, DownloadError};

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
    pub startup_timeout: Duration,
    pub request_timeout: Duration,
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
        let config = Self {
            program: program.into(),
            host: host.into(),
            port,
            rpc_secret: rpc_secret.into(),
            startup_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(15),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    fn validate(&self) -> Result<(), DownloadError> {
        if self.program.trim().is_empty()
            || self.host.trim().is_empty()
            || self.rpc_secret.is_empty()
            || self.port == 0
            || self.startup_timeout.is_zero()
            || self.request_timeout.is_zero()
        {
            return Err(DownloadError::InvalidSupervisorConfiguration);
        }
        Ok(())
    }

    pub(crate) fn command_args(&self) -> Vec<String> {
        // ENG-13: deliberately no `--rpc-secret` here; it would expose the
        // secret in the child process command line. The secret travels via
        // `--conf-path` (see `write_secret_file`).
        vec![
            "--enable-rpc=true".into(),
            "--rpc-listen-all=false".into(),
            format!("--rpc-listen-port={}", self.port),
            "--quiet=true".into(),
        ]
    }

    /// Write the RPC secret to an owner-only temp config file, returning its
    /// path. The file contains only `rpc-secret=<secret>\n`.
    pub(crate) fn write_secret_file(&self) -> Result<PathBuf, DownloadError> {
        self.validate()?;
        let mut path = std::env::temp_dir();
        let unique = SECRET_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        path.push(format!(
            "xarchive-aria2-secret-{}-{unique}.conf",
            std::process::id()
        ));
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
    client: Aria2HttpClient,
    secret_file: Option<PathBuf>,
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
    #[cfg(test)]
    pub(crate) fn with_secret_file_for_test(client: Aria2HttpClient, secret_file: PathBuf) -> Self {
        Self {
            child: None,
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
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&secret_file);
            return Err(error);
        }
        Ok(Self {
            child: Some(child),
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
        let child = self
            .child
            .as_mut()
            .ok_or(DownloadError::SupervisorNotRunning)?;
        child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|error| DownloadError::Process(error.to_string()))
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<bool, DownloadError> {
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
                self.child.take();
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        // ENG-13: remove the short-lived secret file; the secret must not
        // outlive the supervised process.
        if let Some(path) = self.secret_file.take() {
            let _ = std::fs::remove_file(path);
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

impl Drop for Aria2Supervisor {
    fn drop(&mut self) {
        self.shutdown();
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
