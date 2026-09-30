//! Secret redaction for diagnostics, logs, persisted state and browser-visible
//! messages (P2-A).
//!
//! Network configuration may legitimately carry credentials: an HTTP proxy is
//! commonly written as `http://user:password@host:port`. The child processes
//! that perform extraction and transfer need those credentials to work, but the
//! values must never reach SQLite, ordinary log files, or messages that travel
//! back to the browser extension.
//!
//! Every helper here is total: redaction never fails, never truncates unrelated
//! text, and leaves text without secrets byte-for-byte unchanged.

/// Replacement text written in place of a redacted secret.
pub const REDACTED: &str = "[REDACTED]";

/// Query parameter names whose value must never be written down.
const SENSITIVE_QUERY_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "auth",
    "key",
    "passwd",
    "password",
    "rpc_secret",
    "secret",
    "token",
    "access_token",
];

/// Mask the userinfo part of a URL-shaped value.
///
/// `http://user:password@host:8080/path` becomes
/// `http://[REDACTED]@host:8080/path`; scheme, host, port and path stay legible
/// so a misconfigured proxy can still be diagnosed. Values without a scheme, or
/// without userinfo, are returned unchanged.
pub fn redact_url_credentials(value: &str) -> String {
    let Some(scheme_end) = value.find("://") else {
        return value.to_owned();
    };
    let authority_start = scheme_end + 3;
    let rest = &value[authority_start..];
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let Some(at) = authority.rfind('@') else {
        return value.to_owned();
    };
    let mut redacted = String::with_capacity(value.len());
    redacted.push_str(&value[..authority_start]);
    redacted.push_str(REDACTED);
    redacted.push_str(&rest[at..]);
    redacted
}

/// Mask the values of sensitive query parameters.
///
/// `https://x/status?token=abc&id=1` becomes
/// `https://x/status?token=[REDACTED]&id=1`. Unknown parameters are untouched,
/// so unrelated diagnostics stay readable.
pub fn redact_query_secrets(value: &str) -> String {
    let Some(index) = value.find(['?', '&']) else {
        return value.to_owned();
    };
    let mut redacted = String::with_capacity(value.len());
    redacted.push_str(&value[..index]);
    let mut remaining = &value[index..];
    while !remaining.is_empty() {
        let separator = remaining.chars().next().expect("separator is present");
        redacted.push(separator);
        remaining = &remaining[separator.len_utf8()..];
        let end = remaining.find(['?', '&']).unwrap_or(remaining.len());
        let (segment, tail) = remaining.split_at(end);
        redacted.push_str(&redact_query_segment(segment));
        remaining = tail;
    }
    redacted
}

/// Replace every literal occurrence of each configured secret.
///
/// Empty and whitespace-only entries are ignored so a blank configuration
/// cannot erase the text it is meant to protect.
pub fn redact_literals(text: &str, secrets: &[String]) -> String {
    let mut redacted = text.to_owned();
    for secret in secrets.iter().filter(|secret| !secret.trim().is_empty()) {
        if redacted.contains(secret.as_str()) {
            redacted = redacted.replace(secret.as_str(), REDACTED);
        }
    }
    redacted
}

/// Redact configured literals plus URL credentials and sensitive query values.
///
/// This is the entry point for log lines, frontend diagnostics, and any other
/// human-readable sink.
pub fn redact(text: &str, secrets: &[String]) -> String {
    let literals = redact_literals(text, secrets);
    redact_query_secrets(&redact_url_credentials(&literals))
}

fn redact_query_segment(segment: &str) -> String {
    let Some((key, _value)) = segment.split_once('=') else {
        return segment.to_owned();
    };
    if is_sensitive_query_key(key) {
        format!("{key}={REDACTED}")
    } else {
        segment.to_owned()
    }
}

fn is_sensitive_query_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase().replace('-', "_");
    SENSITIVE_QUERY_KEYS.contains(&normalized.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_proxy_credentials_and_keeps_the_endpoint_readable() {
        assert_eq!(
            redact_url_credentials("http://alice:s3cret@proxy.example:8080"),
            "http://[REDACTED]@proxy.example:8080"
        );
        assert_eq!(
            redact_url_credentials("socks5://alice:s3cret@127.0.0.1:1080/path?x=1"),
            "socks5://[REDACTED]@127.0.0.1:1080/path?x=1"
        );
    }

    #[test]
    fn leaves_values_without_userinfo_or_scheme_unchanged() {
        for value in [
            "proxy.example:8080",
            "http://proxy.example:8080/health",
            "not a url at all",
        ] {
            assert_eq!(redact_url_credentials(value), value);
        }
    }

    #[test]
    fn masks_sensitive_query_values_only() {
        assert_eq!(
            redact_query_secrets("https://x.com/a?token=abc&id=1"),
            "https://x.com/a?token=[REDACTED]&id=1"
        );
        assert_eq!(
            redact_query_secrets("https://x.com/a?rpc-secret=abc&RPC_SECRET=def&keep=1"),
            "https://x.com/a?rpc-secret=[REDACTED]&RPC_SECRET=[REDACTED]&keep=1"
        );
        assert_eq!(redact_query_secrets("plain text"), "plain text");
    }

    #[test]
    fn replaces_configured_literals_and_ignores_blanks() {
        let secrets = vec!["user:pass".to_owned(), "   ".to_owned(), String::new()];
        let redacted = redact_literals("proxy=user:pass timeout=30", &secrets);
        assert_eq!(redacted, "proxy=[REDACTED] timeout=30");
        assert!(!redacted.contains("user:pass"));
    }

    #[test]
    fn full_redaction_covers_literals_credentials_and_query_values() {
        let proxy = "http://alice:s3cret@proxy.example:8080".to_owned();
        let text = format!("using {proxy} for https://x.com/a?token=abc");
        let redacted = redact(&text, std::slice::from_ref(&proxy));
        assert_eq!(
            redacted,
            "using [REDACTED] for https://x.com/a?token=[REDACTED]"
        );
        assert!(!redacted.contains("s3cret"));
        assert!(!redacted.contains("token=abc"));
    }
}
