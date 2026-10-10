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
        assert_eq!(config.proxy_mode, ProxyMode::System);
        // The default follows the system proxy, so aria2 keeps whatever it
        // would have discovered on its own.
        assert!(config.environment().is_empty());
        assert!(config.environment_remove().is_empty());
    }

    #[test]
    fn unified_network_settings_reach_the_command_line_without_the_proxy() {
        let config = config()
            .with_network(Duration::from_secs(12), Duration::from_secs(45), 5)
            .with_proxy_mode(ProxyMode::Manual)
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
    fn user_args_are_appended_after_the_application_options() {
        // Batch E: validated per-task user options are appended after the
        // application-owned options so a user value wins over an application
        // default for a shared option. They are already allowlist/value vetted
        // upstream; a protected option never reaches this point.
        let config = config()
            .with_network(Duration::from_secs(30), Duration::from_secs(60), 3)
            .with_user_args(vec![
                "--max-tries".to_owned(),
                "9".to_owned(),
                "--split".to_owned(),
                "4".to_owned(),
            ]);
        let args = config.command_args();
        // The user's --max-tries overrides the application default of 3 because
        // it appears later on the argv; both are present, the last one wins.
        assert_eq!(
            args.iter()
                .filter(|a| a.as_str() == "--max-tries=3")
                .count(),
            1
        );
        assert!(args.contains(&"--max-tries".to_owned()));
        assert!(args.contains(&"9".to_owned()));
        assert!(args.contains(&"--split".to_owned()));
        assert!(args.contains(&"4".to_owned()));
        // The RPC secret is never placed on the argv by user args.
        assert!(!args.iter().any(|a| a.contains("rpc-secret")));
        // The application-owned output/proxy/RPC options are still intact and
        // the user options are last.
        assert_eq!(args.last(), Some(&"4".to_owned()));
    }

    #[test]
    fn command_args_without_user_args_matches_the_previous_shape() {
        // No user args means the argv is unchanged from before Batch E wiring.
        let args = config().command_args();
        assert!(args.contains(&"--connect-timeout=30".to_owned()));
        assert!(args.contains(&"--timeout=60".to_owned()));
        assert!(args.contains(&"--max-tries=3".to_owned()));
        assert_eq!(args.last(), Some(&"--quiet=true".to_owned()));
    }

    #[test]
    fn proxy_reaches_aria2_through_the_environment_and_never_through_debug() {
        let config = config()
            .with_proxy_mode(ProxyMode::Manual)
            .with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
        let environment = config.environment();
        assert!(!environment.is_empty());
        for (_, value) in &environment {
            assert_eq!(value, "http://alice:s3cret@proxy.example:8080");
        }
        // aria2 reads the lowercase trio; the uppercase spellings and the
        // application-specific variable are added for the other children that
        // share this environment.
        for key in ["all_proxy", "http_proxy", "https_proxy"] {
            assert!(
                environment.iter().any(|(name, _)| name == key),
                "aria2 requires {key}, got {environment:?}"
            );
        }
        assert!(config.environment_remove().is_empty());
        let debug = format!("{config:?}");
        assert!(!debug.contains("s3cret"));
        assert!(!debug.contains("proxy.example"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn direct_strips_the_inherited_proxy_environment() {
        let config = config()
            .with_proxy_mode(ProxyMode::Direct)
            .with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
        assert!(
            config.environment().is_empty(),
            "Direct must not hand a stored proxy to aria2"
        );
        let removed = config.environment_remove();
        for key in [
            "http_proxy",
            "https_proxy",
            "all_proxy",
            "HTTP_PROXY",
            "HTTPS_PROXY",
        ] {
            assert!(
                removed.iter().any(|name| name == key),
                "Direct must remove {key}, got {removed:?}"
            );
        }
    }

    #[test]
    fn system_leaves_the_inherited_environment_for_aria2() {
        let config = config().with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
        assert_eq!(config.proxy_mode, ProxyMode::System);
        assert!(
            config.environment().is_empty(),
            "System must not pin the stored manual value on aria2"
        );
        assert!(
            config.environment_remove().is_empty(),
            "System must not strip what aria2 would use on its own"
        );
    }

    #[test]
    fn blank_or_whitespace_proxies_are_ignored_and_malformed_ones_rejected() {
        assert!(
            config()
                .with_proxy_mode(ProxyMode::Manual)
                .with_proxy(Some("   "))
                .proxy
                .is_none()
        );
        let malformed = config()
            .with_proxy_mode(ProxyMode::Manual)
            .with_proxy(Some("http://alice:s3cret@proxy example:8080"));
        assert!(matches!(
            malformed.validate(),
            Err(DownloadError::InvalidSupervisorConfiguration)
        ));
    }
}
