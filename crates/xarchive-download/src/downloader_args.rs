//! Downloader argument handling for Batch E (downloader arguments).
//!
//! This module provides the shared, non-Windows contract for:
//! - user-provided downloader argument text,
//! - backend-parsed/validated argv,
//! - allowlist validation,
//! - aria2 command construction.
//!
//! It deliberately never invokes a shell and never exposes pipes or
//! redirection.

use crate::DownloadError;

/// Controlled allowlist of aria2 options that the application may expose to
/// the user. Options outside this set are rejected rather than silently
/// dropped.
pub const DOWNLOADER_ARG_ALLOWLIST: &[&str] = &[
    // Metadata/behavior flags that must stay enabled for archiving.
    "max-tries",
    "retry-wait",
    "timeout",
    // Transport/proxy flags that stay consistent with the shared proxy policy.
    // Environment-variable and indirect config-file routes are explicitly
    // excluded.
    "all-proxy",
    "http-proxy",
    "https-proxy",
    "ftp-proxy",
    // High-level network knobs that do not change artifact intent.
    "out",
    "out-ext",
];

/// A token that is never legal in user downloader arguments: shell
/// metacharacters that would give the user shell/gearbox semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// Pipe, redirection, variable expansion, or command chaining.
    ShellMeta,
    /// An option that replaces application-controlled input/output/protocol.
    ProtectedOption(String),
    /// An option that disables required metadata.
    MetadataOff(String),
    /// An option that bypasses the shared proxy policy.
    ProxyBypass(String),
}

/// Result of an allowlist check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowlistCheck {
    /// The validated argv that we hand to the backend.
    pub argv: Vec<String>,
    /// Reason if the check failed.
    pub reason: Option<RejectReason>,
}

impl AllowlistCheck {
    /// Whether the check succeeded.
    pub fn is_ok(&self) -> bool {
        self.reason.is_none()
    }
}

/// A user-facing parse of downloader argument text into argv.
///
/// The input is intentionally narrow. We accept a JSON array of strings and
/// reject shell metacharacters. Ordinary unknown flags are reported by the
/// tool and are never silently dropped here; this function rejects anything
/// that would need shell semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloaderArgs {
    pub args: Vec<String>,
}

impl DownloaderArgs {
    pub fn parse(args_text: &str) -> Result<Self, DownloadError> {
        let trimmed = args_text.trim();
        if trimmed.is_empty() {
            return Ok(Self { args: Vec::new() });
        }
        let parsed: Vec<String> = serde_json::from_str(trimmed).map_err(|error| {
            DownloadError::DownloaderArgs(format!("arguments must be a JSON string array: {error}"))
        })?;
        if parsed.iter().any(|token| {
            token.is_empty()
                || token.chars().any(char::is_control)
                || token.contains(['|', '>', '<', '&', ';', '`', '$'])
                || token.starts_with('@')
        }) {
            return Err(DownloadError::DownloaderArgs(
                "empty arguments, control characters, and shell metacharacters are not allowed"
                    .into(),
            ));
        }
        Ok(Self { args: parsed })
    }
}

/// Build aria2 argv from user text plus the allowlist, without invoking a
/// shell. The returned vector only contains options that are on the
/// allowlist; everything else is rejected.
pub fn build_aria2_argv(user_text: &str, allowlist: &[&str]) -> AllowlistCheck {
    let parsed = match DownloaderArgs::parse(user_text) {
        Ok(parsed) => parsed,
        Err(_) => {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::ShellMeta),
            };
        }
    };
    let mut argv = Vec::new();
    let mut index = 0;
    while index < parsed.args.len() {
        let token = &parsed.args[index];
        let Some(option) = token.strip_prefix("--") else {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::ProtectedOption(token.clone())),
            };
        };
        if option.is_empty() || option.contains('=') || !allowlist.contains(&option) {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::ProtectedOption(option.to_owned())),
            };
        }
        let Some(value) = parsed.args.get(index + 1) else {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::ProtectedOption(format!(
                    "{option} requires a value"
                ))),
            };
        };
        if value.starts_with('-') {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::ProtectedOption(format!(
                    "invalid value for {option}"
                ))),
            };
        }
        let valid_value = match option {
            "timeout" | "retry-wait" => value
                .parse::<u64>()
                .is_ok_and(|number| (1..=600).contains(&number)),
            "max-tries" => value
                .parse::<u32>()
                .is_ok_and(|number| (1..=100).contains(&number)),
            "all-proxy" | "http-proxy" | "https-proxy" | "ftp-proxy" => {
                let lower = value.to_ascii_lowercase();
                !lower.starts_with("file:")
                    && !lower.starts_with("http://")
                    && !lower.starts_with("https://")
                    && !lower.starts_with("socks")
            }
            "out" => {
                !value.is_empty()
                    && !value.contains(['/', '\\', ':'])
                    && value != "."
                    && value != ".."
            }
            "out-ext" => {
                !value.is_empty()
                    && value.len() <= 32
                    && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
            }
            _ => false,
        };
        if !valid_value {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::ProtectedOption(format!(
                    "invalid value for {option}"
                ))),
            };
        }
        argv.push(token.clone());
        argv.push(value.clone());
        index += 2;
    }
    AllowlistCheck { argv, reason: None }
}

#[cfg(test)]
mod tests {
    use crate::downloader_args::{DOWNLOADER_ARG_ALLOWLIST, RejectReason, build_aria2_argv};

    #[test]
    fn parses_json_array() {
        let args = crate::downloader_args::DownloaderArgs::parse(r#"["--timeout","10"]"#).unwrap();
        assert_eq!(args.args, vec!["--timeout", "10"]);
    }

    #[test]
    fn parses_empty_string() {
        let args = crate::downloader_args::DownloaderArgs::parse("").unwrap();
        assert!(args.args.is_empty());
    }

    #[test]
    fn rejects_shell_metacharacters() {
        let error = crate::downloader_args::DownloaderArgs::parse("foo | bar")
            .expect_err("expected rejection");
        assert!(error.to_string().contains("JSON string array"));
    }

    #[test]
    fn rejects_pipe_and_ampersand_in_json_array() {
        let error = crate::downloader_args::DownloaderArgs::parse(r#"["foo|bar"]"#)
            .expect_err("expected rejection");
        assert!(error.to_string().contains("shell metacharacters"));
    }

    #[test]
    fn builds_aria2_argv_for_allowed_options() {
        let allowlist = DOWNLOADER_ARG_ALLOWLIST;
        let check = build_aria2_argv(r#"["--max-tries","3","--timeout","60"]"#, allowlist);
        assert!(check.is_ok());
        assert_eq!(check.argv, vec!["--max-tries", "3", "--timeout", "60"]);
    }

    #[test]
    fn rejects_protected_option() {
        let check = build_aria2_argv(r#"["--enable-something"]"#, DOWNLOADER_ARG_ALLOWLIST);
        assert!(!check.is_ok());
        assert_eq!(
            check.reason,
            Some(RejectReason::ProtectedOption("enable-something".into()))
        );
    }

    #[test]
    fn rejects_double_dash_and_attached_short_or_long_options() {
        let check = build_aria2_argv(
            r#"["--max-tries","3","--","--config-path=x"]"#,
            DOWNLOADER_ARG_ALLOWLIST,
        );
        assert!(!check.is_ok());
        for input in [
            r#"["--out=x.mp4"]"#,
            r#"["-out","x.mp4"]"#,
            r#"["--timeout"]"#,
            r#"["--timeout","--config-path=x"]"#,
        ] {
            assert!(
                !build_aria2_argv(input, DOWNLOADER_ARG_ALLOWLIST).is_ok(),
                "accepted {input}"
            );
        }
    }

    #[test]
    fn requires_json_array_without_whitespace_fallback() {
        assert!(crate::downloader_args::DownloaderArgs::parse("--timeout 10").is_err());
        assert!(crate::downloader_args::DownloaderArgs::parse(r#"["--timeout","10"]"#).is_ok());
    }

    #[test]
    fn validates_allowed_option_value_types() {
        for input in [
            r#"["--timeout","0"]"#,
            r#"["--timeout","many"]"#,
            r#"["--all-proxy","file:///tmp/proxy"]"#,
            r#"["--out","../outside.mp4"]"#,
            r#"["--out-ext","mp4;hook"]"#,
        ] {
            assert!(
                !build_aria2_argv(input, DOWNLOADER_ARG_ALLOWLIST).is_ok(),
                "accepted {input}"
            );
        }
        assert!(
            build_aria2_argv(
                r#"["--timeout","30","--max-tries","3"]"#,
                DOWNLOADER_ARG_ALLOWLIST,
            )
            .is_ok()
        );
    }
}
