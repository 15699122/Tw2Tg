//! Downloader argument handling for Batch E (downloader arguments).
//!
//! This module is the **sole Rust authority** for user downloader-option
//! policy. It defines:
//! - the versioned argument policy ([`ARGUMENT_POLICY_VERSION`]),
//! - the narrow, reviewed allowlist of user-exposable aria2 options,
//! - the application-owned (protected) options that users may never override,
//! - JSON-string-array parsing that preserves argv boundaries (no shell),
//! - allowlist/value-policy validation, and
//! - aria2 argv construction from validated user options.
//!
//! It deliberately never invokes a shell and never exposes pipes or
//! redirection. Options outside the allowlist are rejected before any
//! downloader process is spawned; nothing is silently dropped or rewritten.
//! Python/Sidecar retains only protocol-shape and argv-boundary checks and
//! must not maintain a duplicate allowlist or value policy.

use crate::DownloadError;

/// Version of the user downloader-argument policy defined in this module.
///
/// This is independent of the execution-spec version and the recovery-contract
/// version; each is dispatched explicitly. Bump this when the allowlist, the
/// protected set, or any value policy changes so persisted v3 snapshots can be
/// interpreted against the policy that accepted them.
pub const ARGUMENT_POLICY_VERSION: u32 = 1;

/// Controlled allowlist of aria2 options that the application may expose to
/// the user. Options outside this set are rejected rather than silently
/// dropped.
///
/// The set is intentionally narrower than aria2's full CLI surface: it admits
/// only reviewed retry/timeout/concurrency knobs that do **not** change the
/// archived artifact's input, output identity, proxy policy, credentials,
/// configuration loading, hooks, or reporting. Every entry has an explicit
/// value type and range enforced in [`build_aria2_argv`].
pub const DOWNLOADER_ARG_ALLOWLIST: &[&str] = &[
    // Retry/timeout behavior. These never change artifact intent.
    "max-tries",
    "retry-wait",
    "timeout",
    "connect-timeout",
    // Concurrency knobs. Bounded to aria2's supported range.
    "max-connection-per-server",
    "split",
    "max-file-not-found",
];

/// Application-owned aria2 options that a user must never override. These
/// control input/output identity, proxy policy, credentials, configuration
/// loading, RPC/daemon, hooks, or reporting. They are listed explicitly so a
/// rejected override reports an accurate, auditable reason instead of a
/// generic "unknown option" message.
///
/// Membership here never *permits* an option; the allowlist in
/// [`DOWNLOADER_ARG_ALLOWLIST`] is the only source of permitted options. This
/// set only classifies the rejection reason.
pub const PROTECTED_ARIA2_OPTIONS: &[&str] = &[
    // Output / input identity (application owns the archive destination).
    "dir",
    "out",
    "out-ext",
    "input-file",
    "index-out",
    "save-session",
    "auto-file-renaming",
    "allow-overwrite",
    // Configuration loading and other indirect override routes.
    "conf-path",
    "no-conf",
    // Proxy policy (application owns the shared proxy decision).
    "all-proxy",
    "all-proxy-passwd",
    "all-proxy-user",
    "http-proxy",
    "https-proxy",
    "ftp-proxy",
    "no-proxy",
    "proxy-method",
    // Credentials.
    "http-user",
    "http-passwd",
    "ftp-user",
    "ftp-passwd",
    "header",
    // RPC / daemonization.
    "enable-rpc",
    "rpc-listen-port",
    "rpc-secret",
    "daemon",
    // Hooks, checksum, dry-run, and reporting/metadata overrides.
    "on-bt-download-complete",
    "on-download-complete",
    "on-download-error",
    "on-download-start",
    "on-download-pause",
    "on-download-stop",
    "checksum",
    "dry-run",
    "show-files",
    "download-result",
    "log",
    "log-level",
];

/// A token that is never legal in user downloader arguments: shell
/// metacharacters that would give the user shell/gearbox semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// Pipe, redirection, variable expansion, or command chaining.
    ShellMeta,
    /// The input was not a JSON string array.
    NotJsonArray,
    /// An application-owned option that the user may not override (input/
    /// output/proxy/credential/config/RPC/hook/reporting).
    ProtectedOption(String),
    /// An option that is neither allowlisted nor a known protected option.
    UnknownOption(String),
    /// A rejected value for an otherwise allowlisted option (wrong type, out
    /// of range, or a forbidden form).
    InvalidValue(String),
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

/// Validate and preserve a gallery-dl argv array using the shared Rust policy.
/// No gallery-dl options are enabled until a safe option/version contract is
/// reviewed. An empty array remains valid.
pub fn validate_gallery_dl_args(args: &[String]) -> Result<Vec<String>, DownloadError> {
    validate_json_argv(args)?;
    if !args.is_empty() {
        return Err(DownloadError::DownloaderArgs(
            "gallery-dl user options are not enabled by the current policy".into(),
        ));
    }
    Ok(args.to_vec())
}

fn validate_json_argv(args: &[String]) -> Result<(), DownloadError> {
    if args.len() > 128
        || args.iter().any(|token| {
            token.is_empty()
                || token.len() > 4096
                || token.chars().any(char::is_control)
                || token.contains(['|', '>', '<', '&', ';', '`', '$'])
                || token.starts_with('@')
        })
    {
        return Err(DownloadError::DownloaderArgs(
            "invalid or unsafe downloader argument array".into(),
        ));
    }
    Ok(())
}

/// Revalidate a persisted aria2 argv array with this build's policy.
pub fn validate_aria2_args(args: &[String]) -> Result<Vec<String>, DownloadError> {
    validate_json_argv(args)?;
    let encoded = serde_json::to_string(args)
        .map_err(|error| DownloadError::DownloaderArgs(error.to_string()))?;
    let checked = build_aria2_argv(&encoded, DOWNLOADER_ARG_ALLOWLIST);
    match checked.reason {
        Some(reason) => Err(DownloadError::DownloaderArgs(reject_reason_summary(
            &reason,
        ))),
        None => Ok(checked.argv),
    }
}

fn reject_reason_summary(reason: &RejectReason) -> String {
    match reason {
        RejectReason::ShellMeta => "shell-like argument content is not allowed".into(),
        RejectReason::NotJsonArray => "arguments must be a JSON string array".into(),
        RejectReason::ProtectedOption(_) => "application-owned option is not allowed".into(),
        RejectReason::UnknownOption(_) => "option is not allowlisted".into(),
        RejectReason::InvalidValue(_) => "argument form or value is invalid".into(),
    }
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
///
/// Classification is stable and auditable so a rejected override reports an
/// accurate reason instead of a generic message:
/// - [`RejectReason::NotJsonArray`] when the text is not a JSON string array,
/// - [`RejectReason::ShellMeta`] for empty tokens, control characters, `@file`
///   indirection, or shell metacharacters,
/// - [`RejectReason::ProtectedOption`] for an application-owned option the user
///   may never override (input/output/proxy/credential/config/RPC/hook/report),
/// - [`RejectReason::UnknownOption`] for a well-formed `--name` that is neither
///   protected nor allowlisted (never silently dropped),
/// - [`RejectReason::InvalidValue`] for a malformed argument form (short option,
///   attached `--name=value`, the `--` terminator, a bare value) or a value that
///   fails the per-option policy.
pub fn build_aria2_argv(user_text: &str, allowlist: &[&str]) -> AllowlistCheck {
    let trimmed = user_text.trim();
    if trimmed.is_empty() {
        return AllowlistCheck {
            argv: Vec::new(),
            reason: None,
        };
    }
    let parsed: Vec<String> = match serde_json::from_str(trimmed) {
        Ok(parsed) => parsed,
        Err(_) => {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::NotJsonArray),
            };
        }
    };
    if parsed.iter().any(|token| {
        token.is_empty()
            || token.chars().any(char::is_control)
            || token.contains(['|', '>', '<', '&', ';', '`', '$'])
            || token.starts_with('@')
    }) {
        return AllowlistCheck {
            argv: Vec::new(),
            reason: Some(RejectReason::ShellMeta),
        };
    }

    let mut argv = Vec::new();
    let mut index = 0;
    while index < parsed.len() {
        let token = &parsed[index];
        // The argv terminator, attached `--name=value`, short options and bare
        // values are all unsupported user-argument forms: reject them rather
        // than guess an interpretation.
        if token == "--" || !token.starts_with("--") {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::InvalidValue(format!(
                    "unsupported argument form: {token}"
                ))),
            };
        }
        let option = &token[2..];
        if option.is_empty() || option.contains('=') {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::InvalidValue(format!(
                    "unsupported argument form: {token}"
                ))),
            };
        }
        if PROTECTED_ARIA2_OPTIONS.contains(&option) {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::ProtectedOption(option.to_owned())),
            };
        }
        if !allowlist.contains(&option) {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::UnknownOption(option.to_owned())),
            };
        }
        let Some(value) = parsed.get(index + 1) else {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::InvalidValue(format!(
                    "{option} requires a value"
                ))),
            };
        };
        if value.starts_with('-') {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::InvalidValue(format!(
                    "invalid value for {option}"
                ))),
            };
        }
        if !downloader_option_value_is_allowed(option, value) {
            return AllowlistCheck {
                argv: Vec::new(),
                reason: Some(RejectReason::InvalidValue(format!(
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

/// Per-option value policy for the allowlisted aria2 options.
///
/// Ranges are application policy bounds for the pinned aria2 version, kept
/// conservative so a user option cannot request a behavior the archive runner
/// does not expect. Every allowlisted option must appear here; an unhandled
/// allowlisted option is a policy bug and returns `false` (rejected).
fn downloader_option_value_is_allowed(option: &str, value: &str) -> bool {
    let in_range = |min: u64, max: u64| -> bool {
        value
            .parse::<u64>()
            .is_ok_and(|number| (min..=max).contains(&number))
    };
    match option {
        "max-tries" => in_range(1, 100),
        "retry-wait" => in_range(0, 600),
        "timeout" => in_range(1, 600),
        "connect-timeout" => in_range(1, 600),
        "max-connection-per-server" => in_range(1, 16),
        "split" => in_range(1, 16),
        "max-file-not-found" => in_range(0, 100),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::downloader_args::{
        DOWNLOADER_ARG_ALLOWLIST, PROTECTED_ARIA2_OPTIONS, RejectReason, build_aria2_argv,
        validate_aria2_args, validate_gallery_dl_args,
    };

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
        // `--dir` is application-owned: the archive runner supplies the
        // destination. Rejecting it with a protected classification (rather
        // than a generic unknown-option error) is the contract.
        let check = build_aria2_argv(r#"["--dir","/tmp/elsewhere"]"#, DOWNLOADER_ARG_ALLOWLIST);
        assert!(!check.is_ok());
        assert_eq!(
            check.reason,
            Some(RejectReason::ProtectedOption("dir".into()))
        );
        // A well-formed option that is neither allowlisted nor protected is
        // reported as unknown, never silently dropped.
        let unknown = build_aria2_argv(r#"["--enable-something"]"#, DOWNLOADER_ARG_ALLOWLIST);
        assert!(!unknown.is_ok());
        assert_eq!(
            unknown.reason,
            Some(RejectReason::UnknownOption("enable-something".into()))
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

    #[test]
    fn rejects_application_owned_output_and_proxy_overrides() {
        // Batch E contract: the application owns output identity and proxy
        // policy, so these must be refused before any aria2 process starts.
        // The pre-integration prototype accepted them; that was a contract gap
        // and is now fixed. These are acceptance assertions for the Rust
        // argument-policy authority.
        for (input, option) in [
            (r#"["--out","replacement.bin"]"#, "out"),
            (r#"["--out-ext","mp4"]"#, "out-ext"),
            (r#"["--all-proxy","127.0.0.1:8080"]"#, "all-proxy"),
            (r#"["--http-proxy","127.0.0.1:8080"]"#, "http-proxy"),
            (r#"["--no-proxy","localhost"]"#, "no-proxy"),
            (r#"["--http-user","user"]"#, "http-user"),
            (r#"["--http-passwd","pass"]"#, "http-passwd"),
            (r#"["--conf-path","/tmp/evil.conf"]"#, "conf-path"),
            (r#"["--save-session","/tmp/session.txt"]"#, "save-session"),
            (r#"["--enable-rpc"]"#, "enable-rpc"),
            (r#"["--rpc-secret","token"]"#, "rpc-secret"),
            (
                r#"["--on-download-complete","/bin/sh"]"#,
                "on-download-complete",
            ),
            (r#"["--checksum","md5=deadbeef"]"#, "checksum"),
            (r#"["--log","/tmp/aria2.log"]"#, "log"),
        ] {
            let check = build_aria2_argv(input, DOWNLOADER_ARG_ALLOWLIST);
            assert!(!check.is_ok(), "accepted {input}");
            assert_eq!(
                check.reason,
                Some(RejectReason::ProtectedOption(option.into())),
                "wrong classification for {input}"
            );
        }
    }

    #[test]
    fn every_allowlisted_option_has_a_validated_value_range() {
        // An allowlisted option without a value policy would fall through to
        // the reject branch and be unusable. Assert each one actually accepts
        // an in-range value, so a future allowlist addition without a policy
        // fails here instead of silently breaking the option.
        for (option, value) in [
            ("max-tries", "3"),
            ("retry-wait", "30"),
            ("timeout", "60"),
            ("connect-timeout", "15"),
            ("max-connection-per-server", "4"),
            ("split", "8"),
            ("max-file-not-found", "2"),
        ] {
            let input = format!(r#"["--{option}","{value}"]"#);
            let check = build_aria2_argv(&input, DOWNLOADER_ARG_ALLOWLIST);
            assert!(
                check.is_ok(),
                "rejected allowed option {option}: {:?}",
                check.reason
            );
            assert_eq!(check.argv, vec![format!("--{option}"), value.to_owned()]);
        }
    }

    #[test]
    fn rejects_out_of_range_values_for_allowlisted_options() {
        for input in [
            r#"["--max-tries","0"]"#,
            r#"["--max-tries","101"]"#,
            r#"["--timeout","0"]"#,
            r#"["--retry-wait","601"]"#,
            r#"["--connect-timeout","601"]"#,
            r#"["--split","17"]"#,
            r#"["--max-connection-per-server","17"]"#,
            r#"["--max-file-not-found","101"]"#,
            r#"["--timeout","-5"]"#,
            r#"["--timeout","1.5"]"#,
        ] {
            let check = build_aria2_argv(input, DOWNLOADER_ARG_ALLOWLIST);
            assert!(!check.is_ok(), "accepted {input}");
            assert_eq!(
                check
                    .reason
                    .as_ref()
                    .map(|reason| matches!(reason, RejectReason::InvalidValue(_))),
                Some(true),
                "expected InvalidValue for {input}, got {:?}",
                check.reason
            );
        }
    }

    #[test]
    fn rejects_gallery_dl_options_until_a_reviewed_allowlist_exists() {
        assert!(validate_gallery_dl_args(&[]).unwrap().is_empty());
        let error = validate_gallery_dl_args(&["--write-metadata".to_owned()])
            .expect_err("unreviewed gallery-dl options fail closed");
        assert!(error.to_string().contains("not enabled"));
    }

    #[test]
    fn aria2_policy_errors_do_not_echo_unknown_options_or_values() {
        let error = validate_aria2_args(&[
            "--unknown-sensitive-option".to_owned(),
            "private-value".to_owned(),
        ])
        .expect_err("unknown option rejected");
        assert!(!error.to_string().contains("unknown-sensitive-option"));
        assert!(!error.to_string().contains("private-value"));
    }

    #[test]
    fn preserves_argument_order_and_string_boundaries() {
        let check = build_aria2_argv(
            r#"["--split","16","--max-tries","1","--timeout","600"]"#,
            DOWNLOADER_ARG_ALLOWLIST,
        );
        assert!(check.is_ok());
        assert_eq!(
            check.argv,
            vec!["--split", "16", "--max-tries", "1", "--timeout", "600"]
        );
    }

    #[test]
    fn allows_repeated_non_exclusive_options() {
        // `retry-wait` and the timeout knobs are repeatable in aria2; the last
        // value wins in the tool. The policy allows repetition because no
        // allowlisted option can change artifact intent.
        let check = build_aria2_argv(
            r#"["--timeout","30","--timeout","60"]"#,
            DOWNLOADER_ARG_ALLOWLIST,
        );
        assert!(check.is_ok(), "{:?}", check.reason);
        assert_eq!(check.argv, vec!["--timeout", "30", "--timeout", "60"]);
    }

    #[test]
    fn rejects_unicode_and_escaped_shell_indirection() {
        for input in [
            r#"["\u0007bell"]"#,
            r#"["--timeout","\u000760"]"#,
            r#"["@/etc/passwd"]"#,
            r#"["--timeout","$(whoami)"]"#,
            r#"["--timeout","60;rm -rf /"]"#,
        ] {
            assert!(
                !build_aria2_argv(input, DOWNLOADER_ARG_ALLOWLIST).is_ok(),
                "accepted {input}"
            );
        }
    }

    #[test]
    fn policy_sets_have_no_duplicate_entries() {
        // The allowlist and the protected set are audited policy data. A
        // duplicate entry is a contract-data defect even though membership
        // checks still behave correctly, so lock both sets against it.
        let mut allowlist = DOWNLOADER_ARG_ALLOWLIST.to_vec();
        allowlist.sort_unstable();
        allowlist.dedup();
        assert_eq!(allowlist.len(), DOWNLOADER_ARG_ALLOWLIST.len());

        let mut protected = PROTECTED_ARIA2_OPTIONS.to_vec();
        protected.sort_unstable();
        protected.dedup();
        assert_eq!(protected.len(), PROTECTED_ARIA2_OPTIONS.len());
    }

    #[test]
    fn allowlist_and_protected_sets_are_disjoint() {
        // An option can never be both user-exposable and application-owned.
        // Membership in the protected set only classifies a rejection, so an
        // overlap would make the protected classification unreachable.
        for option in DOWNLOADER_ARG_ALLOWLIST {
            assert!(
                !PROTECTED_ARIA2_OPTIONS.contains(option),
                "option {option} is both allowlisted and protected"
            );
        }
    }

    #[test]
    fn empty_and_missing_values_are_rejected_not_defaulted() {
        let missing = build_aria2_argv(r#"["--timeout"]"#, DOWNLOADER_ARG_ALLOWLIST);
        assert!(!missing.is_ok(), "{:?}", missing.reason);
        assert_eq!(
            missing.reason,
            Some(RejectReason::InvalidValue(
                "timeout requires a value".into()
            ))
        );
        let empty = build_aria2_argv(r#"["--timeout",""]"#, DOWNLOADER_ARG_ALLOWLIST);
        assert!(
            !empty.is_ok(),
            "an empty value must not be treated as an omitted option"
        );
        // An empty token is a shell-meta class violation: there is no value to
        // substitute a default into, so the whole submission is rejected.
        assert_eq!(empty.reason, Some(RejectReason::ShellMeta));
    }
}
