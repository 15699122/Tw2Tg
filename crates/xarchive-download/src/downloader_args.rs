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
    "auto-file-renaming",
    "conditional-get",
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
        let parsed: Vec<String> = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                // Narrow fallback: flat whitespace tokenizer, but reject any
                // token that carries shell semantics.
                let tokens: Vec<String> = trimmed.split_whitespace().map(String::from).collect();
                if tokens.iter().any(|t| {
                    t.contains(['|', '>', '<', '&', ';', '`', '$'])
                        || t.starts_with("&&")
                        || t.starts_with("||")
                }) {
                    return Err(DownloadError::DownloaderArgs(
                        "shell metacharacters are not allowed".into(),
                    ));
                }
                return Ok(Self { args: tokens });
            }
        };
        for token in &parsed {
            if token.contains(['|', '>', '<', '&', ';', '`', '$'])
                || token.starts_with("&&")
                || token.starts_with("||")
            {
                return Err(DownloadError::DownloaderArgs(
                    "shell metacharacters are not allowed".into(),
                ));
            }
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
    for token in &parsed.args {
        if *token == "--" {
            // Terminator: stop option parsing. Any token after `--` belongs
            // to the tool, so we stop allowlisting here.
            argv.push("--".into());
            break;
        }
        if let Some(arg) = token.strip_prefix("--") {
            if !allowlist.contains(&arg) {
                return AllowlistCheck {
                    argv: Vec::new(),
                    reason: Some(RejectReason::ProtectedOption(arg.to_owned())),
                };
            }
            argv.push(token.clone());
        } else if let Some(arg) = token.strip_prefix('-') {
            if !allowlist.contains(&arg) {
                return AllowlistCheck {
                    argv: Vec::new(),
                    reason: Some(RejectReason::ProtectedOption(arg.to_owned())),
                };
            }
            argv.push(token.clone());
        } else {
            // Plain argument (URL, filename): order is preserved and is not
            // a shell metacharacter because the parser above already
            // rejected '|', '>', '<', '&', ';'.
            argv.push(token.clone());
        }
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
        assert!(error.to_string().contains("not allowed"));
    }

    #[test]
    fn rejects_pipe_and_ampersand_in_json_array() {
        let error = crate::downloader_args::DownloaderArgs::parse(r#"["foo|bar"]"#)
            .expect_err("expected rejection");
        assert!(error.to_string().contains("not allowed"));
    }

    #[test]
    fn builds_aria2_argv_for_allowed_options() {
        let allowlist = DOWNLOADER_ARG_ALLOWLIST;
        let check = build_aria2_argv(r#"["--out","a.mp4","--max-tries","3"]"#, allowlist);
        assert!(check.is_ok());
        assert_eq!(check.argv, vec!["--out", "a.mp4", "--max-tries", "3"]);
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
    fn stops_at_double_dash() {
        let check = build_aria2_argv(
            r#"["--out","x.mp4","--","--bad"]"#,
            DOWNLOADER_ARG_ALLOWLIST,
        );
        assert!(check.is_ok());
        assert_eq!(check.argv, vec!["--out", "x.mp4", "--"]);
    }
}
