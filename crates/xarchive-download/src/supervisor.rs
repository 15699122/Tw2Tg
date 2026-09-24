use std::fmt::Write as _;
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
    /// Per-connection connect timeout (`aria2c --connect-timeout`).
    pub connect_timeout: Duration,
    /// Per-connection idle timeout (`aria2c --timeout`).
    pub idle_timeout: Duration,
    /// Attempts per media URL (`aria2c --max-tries`).
    pub max_tries: u32,
    /// Optional HTTP/SOCKS proxy. Carries credentials when the network requires
    /// authentication, so it is passed to aria2 through the environment rather
    /// than the command line and never appears in `Debug` output.
    pub proxy: Option<String>,
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
            .field("proxy", &self.proxy.as_ref().map(|_| "[REDACTED]"))
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
            startup_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(15),
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(60),
            max_tries: 3,
            proxy: None,
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
        vec![
            "--enable-rpc=true".into(),
            "--rpc-listen-all=false".into(),
            format!("--rpc-listen-port={}", self.port),
            format!("--rpc-secret={}", self.rpc_secret),
            format!("--connect-timeout={}", self.connect_timeout.as_secs()),
            format!("--timeout={}", self.idle_timeout.as_secs()),
            format!("--max-tries={}", self.max_tries),
            "--retry-wait=1".into(),
            "--quiet=true".into(),
        ]
    }

    /// Environment variables handed to the aria2 child process.
    ///
    /// aria2 falls back to `http_proxy`/`https_proxy`/`all_proxy` when the
    /// matching command-line option is absent, which keeps proxy credentials out
    /// of the process command line and out of any command-echo diagnostics.
    pub(crate) fn environment(&self) -> Vec<(String, String)> {
        let Some(proxy) = self.proxy.clone() else {
            return Vec::new();
        };
        vec![
            ("all_proxy".to_owned(), proxy.clone()),
            ("http_proxy".to_owned(), proxy.clone()),
            ("https_proxy".to_owned(), proxy),
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
        let mut command = Command::new(&config.program);
        hide_console_window(&mut command);
        for (key, value) in config.environment() {
            command.env(key, value);
        }
        let mut child = command
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

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Aria2SupervisorConfig {
        Aria2SupervisorConfig::new("aria2c", "127.0.0.1", 6800, "rpc-secret-value")
            .expect("valid supervisor config")
    }

    #[test]
    fn random_rpc_secrets_are_fresh_hex_values() {
        let first = Aria2SupervisorConfig::new_with_random_secret("aria2c", "127.0.0.1", 6800)
            .expect("first random configuration");
        let second = Aria2SupervisorConfig::new_with_random_secret("aria2c", "127.0.0.1", 6800)
            .expect("second random configuration");
        assert_eq!(first.rpc_secret.len(), 64);
        assert!(
            first
                .rpc_secret
                .chars()
                .all(|value| value.is_ascii_hexdigit())
        );
        assert_ne!(first.rpc_secret, second.rpc_secret);
        assert!(!format!("{first:?}").contains(&first.rpc_secret));
    }

    #[test]
    fn defaults_apply_a_bounded_network_budget() {
        let config = config();
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(60));
        assert_eq!(config.max_tries, 3);
        assert!(config.proxy.is_none());
        assert!(config.environment().is_empty());
    }

    #[test]
    fn unified_network_settings_reach_the_command_line_without_the_proxy() {
        let config = config()
            .with_network(Duration::from_secs(12), Duration::from_secs(45), 5)
            .with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
        let args = config.command_args();
        assert!(args.contains(&"--connect-timeout=12".to_owned()));
        assert!(args.contains(&"--timeout=45".to_owned()));
        assert!(args.contains(&"--max-tries=5".to_owned()));
        assert!(
            !args
                .iter()
                .any(|argument| argument.contains("s3cret") || argument.contains("proxy.example")),
            "proxy credentials must not be passed on the command line"
        );
    }

    #[test]
    fn proxy_reaches_aria2_through_the_environment_and_never_through_debug() {
        let config = config().with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
        let environment = config.environment();
        assert_eq!(environment.len(), 3);
        for (_, value) in &environment {
            assert_eq!(value, "http://alice:s3cret@proxy.example:8080");
        }
        let keys = environment
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["all_proxy", "http_proxy", "https_proxy"]);
        let debug = format!("{config:?}");
        assert!(!debug.contains("s3cret"));
        assert!(!debug.contains("proxy.example"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn blank_or_whitespace_proxies_are_ignored_and_malformed_ones_rejected() {
        assert!(config().with_proxy(Some("   ")).proxy.is_none());
        let malformed = config().with_proxy(Some("http://alice:s3cret@proxy example:8080"));
        assert!(matches!(
            malformed.validate(),
            Err(DownloadError::InvalidSupervisorConfiguration)
        ));
    }
}
