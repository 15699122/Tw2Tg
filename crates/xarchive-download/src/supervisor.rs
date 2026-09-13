use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::{Aria2HttpClient, DownloadError};

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
        vec![
            "--enable-rpc=true".into(),
            "--rpc-listen-all=false".into(),
            format!("--rpc-listen-port={}", self.port),
            format!("--rpc-secret={}", self.rpc_secret),
            "--quiet=true".into(),
        ]
    }
}

pub struct Aria2Supervisor {
    child: Option<Child>,
    client: Aria2HttpClient,
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
    pub fn spawn(config: Aria2SupervisorConfig) -> Result<Self, DownloadError> {
        config.validate()?;
        let client = Aria2HttpClient::new(&config.host, config.port, &config.rpc_secret)?
            .with_timeout(config.request_timeout);
        let mut child = Command::new(&config.program)
            .args(config.command_args())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| DownloadError::Process(error.to_string()))?;
        if let Err(error) = wait_for_aria2(&mut child, &client, config.startup_timeout) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(Self {
            child: Some(child),
            client,
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
