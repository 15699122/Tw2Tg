//! Shared outbound proxy routing contract.
//!
//! The Desktop application, the Sidecar, and aria2 all need the same answer to
//! "should this request go through a proxy?". A single optional string cannot
//! express that, because a user needs to distinguish an inherited environment
//! proxy from a deliberate configuration and from an explicit opt out.
//!
//! The three states and the decision mapping live here so the platform layers,
//! the child-process rules, and the settings surface cannot drift apart.

use serde::{Deserialize, Serialize};

/// How outbound network traffic is routed.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// Follow the platform system proxy configuration.
    ///
    /// This is not automatically the full Windows semantics. A platform that
    /// supports PAC or WPAD resolution must supply a resolver; a platform that
    /// only has a static system proxy uses that.
    #[default]
    System,
    /// Never use a proxy, including one inherited from the process environment.
    Direct,
    /// Use the explicitly configured proxy value.
    Manual,
}

impl ProxyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Direct => "direct",
            Self::Manual => "manual",
        }
    }

    /// Parse a user- or configuration-supplied value.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "system" | "" => Ok(Self::System),
            "direct" | "none" | "off" => Ok(Self::Direct),
            "manual" | "custom" => Ok(Self::Manual),
            other => Err(format!("unsupported proxy mode '{other}'")),
        }
    }

    /// Whether this mode can ever use a proxy.
    pub fn allows_proxy(self) -> bool {
        !matches!(self, Self::Direct)
    }
}

/// The routing decision for one destination.
///
/// `ResolutionFailed` and `Unsupported` are deliberately distinct from
/// `Direct`. A platform that cannot honor the configured system policy must not
/// quietly connect directly, because that silently bypasses the policy the user
/// or their employer relies on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProxyDecision {
    /// Connect without a proxy.
    Direct,
    /// Connect through this proxy. The value may carry credentials and must be
    /// treated as a secret everywhere except the transport that uses it.
    Proxy(String),
    /// The system policy exists but could not be resolved. This is an error.
    ResolutionFailed(String),
    /// The platform cannot honor the requested mode at all.
    Unsupported(String),
}

impl ProxyDecision {
    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Self::ResolutionFailed(_) | Self::Unsupported(_))
    }

    /// The proxy value, if one was resolved. Credentials may be present.
    pub fn proxy(&self) -> Option<&str> {
        match self {
            Self::Proxy(value) => Some(value.as_str()),
            _ => None,
        }
    }

    /// The proxy value in a form that is safe to display or log.
    pub fn redacted_proxy(&self) -> Option<String> {
        self.proxy().map(crate::redact_url_credentials)
    }
}

/// Environment variables a child process would read as a proxy source.
///
/// A `Direct` parent must remove these, otherwise the child re-introduces the
/// very proxy the user turned off. Both the lowercase and uppercase spellings
/// are listed because aria2, gallery-dl, and Python tooling disagree on which
/// they honor.
pub const PROXY_ENVIRONMENT_KEYS: [&str; 12] = [
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "ftp_proxy",
    "no_proxy",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "FTP_PROXY",
    "NO_PROXY",
    "XARCHIVE_PROXY",
    "XARCHIVE_PROXY_MODE",
];

/// Whether a variable name is a proxy source, ignoring case.
///
/// Windows environment variable names are case-insensitive, so a `Direct` mode
/// that removed only the exact spellings above would leave `Http_Proxy` in place
/// and silently re-enable the proxy in the child process.
pub fn is_proxy_environment_key(name: &str) -> bool {
    PROXY_ENVIRONMENT_KEYS
        .iter()
        .any(|key| key.eq_ignore_ascii_case(name))
}

/// What a caller must do to apply a mode to a child process environment.
///
/// The caller applies this to an explicit child environment. The application
/// never mutates its own process environment, which keeps the rule testable and
/// keeps a mode change from leaking into unrelated children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProxyRequirement {
    /// Remove every key in [`PROXY_ENVIRONMENT_KEYS`]; the child must connect
    /// directly.
    ClearProxyEnvironment,
    /// Leave the inherited environment alone so the child can discover the
    /// platform proxy itself.
    Inherit,
    /// Set the proxy variables to this value. It may carry credentials and must
    /// not be placed on a command line.
    Set(String),
}

/// Translate a mode plus a configured value into a child-process requirement.
pub fn proxy_requirement(mode: ProxyMode, manual_proxy: Option<String>) -> ProxyRequirement {
    match mode {
        ProxyMode::Direct => ProxyRequirement::ClearProxyEnvironment,
        ProxyMode::System => ProxyRequirement::Inherit,
        ProxyMode::Manual => match manual_proxy
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        {
            Some(value) => ProxyRequirement::Set(value),
            // A `Manual` mode with no value cannot be honored. Clearing is the
            // only honest outcome, and the configuration layer rejects this
            // combination before it reaches a child.
            None => ProxyRequirement::ClearProxyEnvironment,
        },
    }
}

/// The proxy variables to set on a child environment for a requirement.
///
/// An empty result means the child inherits its environment unchanged, which is
/// what `System` requires; the caller removes [`PROXY_ENVIRONMENT_KEYS`]
/// separately for `Direct`.
pub fn child_proxy_environment(
    mode: ProxyMode,
    manual_proxy: Option<String>,
) -> Vec<(String, String)> {
    match proxy_requirement(mode, manual_proxy) {
        ProxyRequirement::ClearProxyEnvironment | ProxyRequirement::Inherit => Vec::new(),
        ProxyRequirement::Set(value) => {
            let mut environment = vec![("XARCHIVE_PROXY".to_owned(), value.clone())];
            for key in [
                "all_proxy",
                "http_proxy",
                "https_proxy",
                "ALL_PROXY",
                "HTTP_PROXY",
                "HTTPS_PROXY",
            ] {
                environment.push((key.to_owned(), value.clone()));
            }
            environment
        }
    }
}

/// The variables a child process needs in order to honor the configured mode.
///
/// A caller passes `set` to `Command::envs` and `remove` to `Command::env_remove`.
/// Returning removals instead of relying on an unset variable is what makes
/// `Direct` survive a launching shell that exported proxy settings.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChildEnvironment {
    pub set: Vec<(String, String)>,
    pub remove: Vec<String>,
}

impl ChildEnvironment {
    /// The environment for a mode, honoring `System` by leaving the inherited
    /// variables untouched.
    pub fn for_mode(mode: ProxyMode, manual_proxy: Option<String>) -> Self {
        match proxy_requirement(mode, manual_proxy) {
            ProxyRequirement::ClearProxyEnvironment => Self {
                set: Vec::new(),
                remove: PROXY_ENVIRONMENT_KEYS
                    .iter()
                    .map(|key| (*key).to_owned())
                    .collect(),
            },
            ProxyRequirement::Inherit => Self::default(),
            ProxyRequirement::Set(value) => Self {
                set: child_proxy_environment(mode, Some(value)),
                remove: Vec::new(),
            },
        }
    }

    /// Whether the child inherits proxy settings from this process.
    pub fn inherits(&self) -> bool {
        self.set.is_empty() && self.remove.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_removes_every_proxy_variable_from_the_child() {
        let environment = ChildEnvironment::for_mode(ProxyMode::Direct, None);
        assert!(environment.set.is_empty());
        for key in PROXY_ENVIRONMENT_KEYS {
            assert!(
                environment.remove.iter().any(|name| name == key),
                "Direct must remove {key} from the child environment"
            );
        }
    }

    #[test]
    fn system_leaves_the_child_environment_alone() {
        let environment =
            ChildEnvironment::for_mode(ProxyMode::System, Some("http://p:1".to_owned()));
        assert!(
            environment.inherits(),
            "System must let the child discover the platform proxy itself"
        );
    }

    #[test]
    fn manual_sets_the_proxy_and_removes_nothing() {
        let environment = ChildEnvironment::for_mode(
            ProxyMode::Manual,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        assert!(environment.remove.is_empty());
        assert!(
            environment
                .set
                .iter()
                .any(|(key, value)| key == "XARCHIVE_PROXY" && value.contains("s3cret"))
        );
    }

    #[test]
    fn direct_still_removes_the_proxy_when_a_value_is_stored() {
        let environment = ChildEnvironment::for_mode(
            ProxyMode::Direct,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        assert!(
            environment.set.is_empty(),
            "a stored value must not leak into a Direct child"
        );
        assert!(!environment.remove.is_empty());
    }

    #[test]
    fn direct_clears_the_child_environment() {
        let requirement = proxy_requirement(ProxyMode::Direct, Some("http://p:1".to_owned()));
        assert_eq!(
            requirement,
            ProxyRequirement::ClearProxyEnvironment,
            "Direct must not hand a stored value to a child"
        );
        assert!(child_proxy_environment(ProxyMode::Direct, None).is_empty());
    }

    #[test]
    fn system_inherits_instead_of_setting_a_value() {
        assert_eq!(
            proxy_requirement(ProxyMode::System, None),
            ProxyRequirement::Inherit
        );
        assert!(
            child_proxy_environment(ProxyMode::System, Some("http://p:1".to_owned())).is_empty(),
            "System must not pin a static value on a child"
        );
    }

    #[test]
    fn manual_sets_every_spelling_the_tooling_reads() {
        let environment = child_proxy_environment(
            ProxyMode::Manual,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        for key in [
            "all_proxy",
            "http_proxy",
            "https_proxy",
            "HTTP_PROXY",
            "HTTPS_PROXY",
        ] {
            assert!(
                environment
                    .iter()
                    .any(|(name, value)| name == key && value.contains("proxy.example")),
                "child environment is missing {key}"
            );
        }
    }

    #[test]
    fn manual_without_a_value_cannot_silently_inherit() {
        assert_eq!(
            proxy_requirement(ProxyMode::Manual, None),
            ProxyRequirement::ClearProxyEnvironment
        );
        assert_eq!(
            proxy_requirement(ProxyMode::Manual, Some("   ".to_owned())),
            ProxyRequirement::ClearProxyEnvironment
        );
    }

    #[test]
    fn every_proxy_environment_key_is_listed_in_both_spellings() {
        for key in ["http_proxy", "https_proxy", "all_proxy"] {
            assert!(PROXY_ENVIRONMENT_KEYS.contains(&key));
            let upper = key.to_ascii_uppercase();
            assert!(PROXY_ENVIRONMENT_KEYS.contains(&upper.as_str()));
        }
        assert!(PROXY_ENVIRONMENT_KEYS.contains(&"XARCHIVE_PROXY"));
    }

    #[test]
    fn proxy_variable_matching_ignores_case() {
        // Windows environment names are case-insensitive, so a `Direct` removal
        // that matched exactly would leave `Http_Proxy` behind.
        for name in ["HTTP_PROXY", "http_proxy", "Http_Proxy", "hTtP_pRoXy"] {
            assert!(is_proxy_environment_key(name), "{name} must be recognized");
        }
        assert!(!is_proxy_environment_key("PATH"));
        assert!(!is_proxy_environment_key("HOME"));
        assert!(!is_proxy_environment_key("XARCHIVE_SIDECAR_PROGRAM"));
    }

    #[test]
    fn a_resolution_failure_is_not_reported_as_direct() {
        let failure = ProxyDecision::ResolutionFailed("PAC unreachable".to_owned());
        assert!(!failure.is_direct());
        assert!(failure.is_failure());
        assert_eq!(failure.redacted_proxy(), None);
    }

    #[test]
    fn decision_modes_parse_and_render() {
        assert_eq!(ProxyMode::parse("SYSTEM"), Ok(ProxyMode::System));
        assert_eq!(ProxyMode::parse(" direct "), Ok(ProxyMode::Direct));
        assert_eq!(ProxyMode::parse("Manual"), Ok(ProxyMode::Manual));
        assert!(ProxyMode::parse("pac").is_err());
        assert_eq!(ProxyMode::Direct.as_str(), "direct");
        assert!(!ProxyMode::Direct.allows_proxy());
        assert!(ProxyMode::System.allows_proxy());
    }

    #[test]
    fn a_decision_redacts_credentials() {
        let decision = ProxyDecision::Proxy("http://alice:s3cret@proxy.example:8080".to_owned());
        let redacted = decision.redacted_proxy().expect("redacted");
        assert!(
            !redacted.contains("s3cret"),
            "credentials leaked: {redacted}"
        );
        assert!(!decision.is_failure());
    }
}
