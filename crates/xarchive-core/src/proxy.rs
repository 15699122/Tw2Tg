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

/// One hop in an ordered proxy resolution, mirroring PAC semantics.
///
/// A PAC result such as `"PROXY a:8080; SOCKS b:1080; DIRECT"` becomes
/// `[Http, Socks, Direct]`. Consumers try candidates in order, so the order
/// this enum is carried in is part of the contract, not an implementation
/// detail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProxyCandidate {
    /// Connect without a proxy. As a *later* candidate this is the PAC
    /// script's own fallback, which is a legitimate policy outcome.
    Direct,
    /// An HTTP proxy as `host:port`. May carry credentials and must be treated
    /// as a secret everywhere except the transport that consumes it.
    Http(String),
    /// A SOCKS proxy as `host:port`, without the SOCKS4/SOCKS5 distinction
    /// because PAC does not preserve it. May carry credentials.
    Socks(String),
}

impl ProxyCandidate {
    /// The proxy endpoint without its kind, if this candidate is a proxy.
    pub fn endpoint(&self) -> Option<&str> {
        match self {
            Self::Direct => None,
            Self::Http(value) | Self::Socks(value) => Some(value.as_str()),
        }
    }

    /// The label this candidate uses in settings and diagnostics.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Direct => "DIRECT",
            Self::Http(_) => "PROXY",
            Self::Socks(_) => "SOCKS",
        }
    }

    /// A form that is safe to display or log. Credentials never survive this.
    pub fn redacted(&self) -> String {
        match self.endpoint() {
            None => "DIRECT".to_owned(),
            Some(endpoint) => format!(
                "{} {}",
                self.kind_label(),
                crate::redact_url_credentials(endpoint)
            ),
        }
    }

    /// Whether this candidate can be consumed by the given transport.
    ///
    /// The transports do not all speak every PAC token, so an unsupported
    /// candidate is reported rather than silently downgraded to direct.
    pub fn supported_by(&self, supports_socks: bool) -> bool {
        match self {
            Self::Direct | Self::Http(_) => true,
            Self::Socks(_) => supports_socks,
        }
    }
}

/// Where an ordered resolution came from.
///
/// This is reported to the user because "the system proxy" and "a proxy
/// environment variable this process happened to inherit" look identical in a
/// settings page but are not the same policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProxySource {
    /// The user explicitly turned the proxy off.
    ExplicitDirect,
    /// The user configured a manual proxy value.
    Manual,
    /// Proxy environment variables inherited by this process.
    Environment,
    /// A static proxy configured by the operating system.
    SystemStatic,
    /// A proxy auto-configuration script supplied by the operating system.
    Pac,
    /// A proxy auto-configuration script discovered over WPAD.
    Wpad,
}

impl ProxySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitDirect => "explicit-direct",
            Self::Manual => "manual",
            Self::Environment => "environment",
            Self::SystemStatic => "system-static",
            Self::Pac => "pac",
            Self::Wpad => "wpad",
        }
    }

    /// Whether this source is the operating system rather than this process.
    pub fn is_system(self) -> bool {
        matches!(self, Self::SystemStatic | Self::Pac | Self::Wpad)
    }

    /// Whether this source can only be answered per destination URL.
    ///
    /// A child process reads proxy environment variables once, so it cannot
    /// evaluate such a policy for the many hosts a single run touches. Leaving
    /// it to discover "the platform proxy" by itself would silently bypass the
    /// policy, which is why callers gate on this instead.
    pub fn is_per_url(self) -> bool {
        matches!(self, Self::Pac | Self::Wpad)
    }
}

/// An ordered, per-URL routing result.
///
/// An empty candidate list is a policy failure, not a direct route. Callers
/// must distinguish "the policy says connect without a proxy" from "the policy
/// could not be resolved", because the second one must not quietly bypass the
/// user's or their employer's proxy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProxyResolution {
    pub candidates: Vec<ProxyCandidate>,
    pub source: ProxySource,
    /// Why the policy could not be resolved, when it could not.
    ///
    /// This is kept so a refusal can still name its cause instead of degrading
    /// into a generic "no route" message.
    pub reason: Option<String>,
}

impl ProxyResolution {
    /// A single explicit direct route.
    pub fn explicit_direct() -> Self {
        Self {
            candidates: vec![ProxyCandidate::Direct],
            source: ProxySource::ExplicitDirect,
            reason: None,
        }
    }

    /// The one manual proxy the user configured.
    pub fn manual(proxy: String) -> Self {
        Self {
            candidates: vec![ProxyCandidate::Http(proxy)],
            source: ProxySource::Manual,
            reason: None,
        }
    }

    /// A resolution that carries no usable route. Callers must surface it as a
    /// failure rather than connecting directly.
    pub fn unresolved(source: ProxySource) -> Self {
        Self {
            candidates: Vec::new(),
            source,
            reason: None,
        }
    }

    /// An unresolved route that carries the cause of the failure.
    pub fn unresolved_because(source: ProxySource, reason: impl Into<String>) -> Self {
        Self {
            candidates: Vec::new(),
            source,
            reason: Some(reason.into()),
        }
    }

    /// Whether this resolution carries no usable route at all.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// The candidate to try first, if any.
    pub fn primary(&self) -> Option<&ProxyCandidate> {
        self.candidates.first()
    }

    /// Collapse the list to the single-route decision older callers expect.
    ///
    /// Only the first candidate is representable, so this is used where a
    /// transport cannot act on an ordered list. A transport that can fall back
    /// should keep the list instead.
    pub fn to_decision(&self) -> ProxyDecision {
        match self.candidates.first() {
            None if self.reason.is_some() => {
                ProxyDecision::ResolutionFailed(self.reason.clone().unwrap_or_default())
            }
            None => ProxyDecision::ResolutionFailed(format!(
                "the {} proxy policy resolved to no route",
                self.source.as_str()
            )),
            Some(ProxyCandidate::Direct) => ProxyDecision::Direct,
            Some(ProxyCandidate::Http(value)) | Some(ProxyCandidate::Socks(value)) => {
                ProxyDecision::Proxy(value.clone())
            }
        }
    }

    /// Every candidate in a form that is safe to display or log.
    pub fn redacted_candidates(&self) -> Vec<String> {
        self.candidates
            .iter()
            .map(ProxyCandidate::redacted)
            .collect()
    }

    /// Drop candidates this transport cannot use.
    ///
    /// Returns `None` when nothing survives, so an all-SOCKS policy on a
    /// transport without SOCKS support fails loudly instead of going direct.
    pub fn candidates_for(&self, supports_socks: bool) -> Option<Vec<ProxyCandidate>> {
        let usable: Vec<ProxyCandidate> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.supported_by(supports_socks))
            .cloned()
            .collect();
        (!usable.is_empty()).then_some(usable)
    }
}

/// A `no_proxy` / bypass list, matched the way the common runtimes match it.
///
/// The Windows adapter uses the resolver's own per-URL matching rather than
/// this type; it exists for the environment configuration Linux already reads.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProxyBypass {
    entries: Vec<String>,
}

impl ProxyBypass {
    /// Parse a `no_proxy` value. Empty entries are dropped.
    pub fn parse(value: &str) -> Self {
        Self {
            entries: value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_ascii_lowercase)
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether this host and port bypass the proxy.
    ///
    /// Supports a bare host, `host:port`, `.domain`, `*.domain` and `*`.
    /// `.domain` matches the domain and its subdomains, `*.domain` matches only
    /// subdomains, and a bare hostname matches itself. A port-qualified entry
    /// only matches that port. A bare hostname is compared exactly, so an entry
    /// naming a parent domain does not silently widen to its subdomains.
    pub fn matches(&self, host: &str, port: u16) -> bool {
        let host = host
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_ascii_lowercase();
        if host.is_empty() {
            return false;
        }
        if self.entries.iter().any(|entry| entry == "*") {
            return true;
        }
        self.entries
            .iter()
            .any(|entry| match entry.rsplit_once(':') {
                Some((pattern, port_text)) => {
                    pattern.matches_host(&host)
                        && port_text
                            .parse::<u16>()
                            .is_ok_and(|candidate| candidate == port)
                }
                None => entry.matches_host(&host),
            })
    }

    /// The entries, for a settings summary.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

trait HostPattern {
    fn matches_host(&self, host: &str) -> bool;
}

impl HostPattern for str {
    fn matches_host(&self, host: &str) -> bool {
        // `*.domain` is the spelling used in real `NO_PROXY` values and in
        // wildcard-style bypass lists, and it means subdomains only. `.domain`
        // means the domain *and* its subdomains, which is the historical
        // `NO_PROXY` convention, so the two must be kept distinct rather than
        // normalized to the same rule.
        if let Some(domain) = self.strip_prefix("*.") {
            return !domain.is_empty() && host.ends_with(&format!(".{domain}"));
        }
        match self.strip_prefix('.') {
            // A leading dot matches the domain itself and its subdomains, which
            // is what `.example.com` means.
            Some(domain) => host == domain || host.ends_with(&format!(".{domain}")),
            None => host == self,
        }
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

    #[test]
    fn an_ordered_candidate_list_keeps_its_order_and_labels() {
        let resolution = ProxyResolution {
            candidates: vec![
                ProxyCandidate::Http("a.example:8080".to_owned()),
                ProxyCandidate::Socks("b.example:1080".to_owned()),
                ProxyCandidate::Direct,
            ],
            source: ProxySource::Pac,
            reason: None,
        };
        assert_eq!(
            resolution.redacted_candidates(),
            ["PROXY a.example:8080", "SOCKS b.example:1080", "DIRECT"]
        );
        assert_eq!(
            resolution.primary(),
            Some(&ProxyCandidate::Http("a.example:8080".to_owned()))
        );
    }

    #[test]
    fn candidate_redaction_keeps_the_kind_and_drops_credentials() {
        let candidate = ProxyCandidate::Http("http://alice:s3cret@proxy.example:8080".to_owned());
        let redacted = candidate.redacted();
        assert!(!redacted.contains("s3cret"), "leaked: {redacted}");
        assert!(redacted.starts_with("PROXY "), "lost the label: {redacted}");
        assert!(redacted.contains("proxy.example"));
    }

    #[test]
    fn an_unresolved_policy_never_becomes_direct() {
        let resolution = ProxyResolution::unresolved(ProxySource::Pac);
        assert!(resolution.is_empty());
        let decision = resolution.to_decision();
        assert!(
            !decision.is_direct(),
            "a PAC failure must not masquerade as a direct route"
        );
        assert!(decision.is_failure());
    }

    #[test]
    fn a_pac_direct_fallback_is_a_real_direct_route() {
        // This is the distinction the whole type exists to preserve: PAC said
        // DIRECT, so this is policy, not a failure. Collapsing to a single
        // route is still correct here because DIRECT is the only candidate.
        let resolution = ProxyResolution {
            candidates: vec![ProxyCandidate::Direct],
            source: ProxySource::Pac,
            reason: None,
        };
        let decision = resolution.to_decision();
        assert!(decision.is_direct());
        assert!(!decision.is_failure());
    }

    #[test]
    fn collapsing_an_ordered_list_takes_the_first_candidate_not_the_last() {
        // `PROXY a; DIRECT` must collapse to the proxy: the single-route view
        // cannot express the fallback, and the proxy is what PAC prefers.
        let resolution = ProxyResolution {
            candidates: vec![
                ProxyCandidate::Http("a.example:8080".to_owned()),
                ProxyCandidate::Direct,
            ],
            source: ProxySource::Pac,
            reason: None,
        };
        let decision = resolution.to_decision();
        assert_eq!(decision.proxy(), Some("a.example:8080"));
        assert!(!decision.is_failure());
        assert_eq!(
            resolution.candidates.len(),
            2,
            "the ordered list is preserved"
        );
    }

    #[test]
    fn an_unsupported_candidate_is_dropped_not_downgraded() {
        let socks_only = ProxyResolution {
            candidates: vec![ProxyCandidate::Socks("b.example:1080".to_owned())],
            source: ProxySource::Pac,
            reason: None,
        };
        assert_eq!(
            socks_only.candidates_for(false),
            None,
            "a SOCKS-only policy must fail on a transport without SOCKS"
        );
        assert!(
            socks_only
                .candidates_for(true)
                .is_some_and(|candidates| candidates.len() == 1)
        );
    }

    #[test]
    fn filtering_keeps_a_direct_fallback_after_dropping_socks() {
        let resolution = ProxyResolution {
            candidates: vec![
                ProxyCandidate::Socks("b.example:1080".to_owned()),
                ProxyCandidate::Direct,
            ],
            source: ProxySource::Pac,
            reason: None,
        };
        let usable = resolution
            .candidates_for(false)
            .expect("the direct fallback survives");
        assert_eq!(usable, vec![ProxyCandidate::Direct]);
    }

    #[test]
    fn bypass_matches_hosts_ports_and_wildcards() {
        let bypass = ProxyBypass::parse("localhost, .internal.example,10.0.0.1:8443");
        assert!(bypass.matches("localhost", 3000));
        assert!(bypass.matches("api.internal.example", 443));
        assert!(bypass.matches("internal.example", 443));
        assert!(bypass.matches("10.0.0.1", 8443));
        assert!(!bypass.matches("10.0.0.1", 443), "the port must matter");
        assert!(!bypass.matches("evil-internal.example", 443));
        assert!(!bypass.matches("example.com", 443));
        assert!(
            bypass.matches("LOCALHOST", 3000),
            "host matching is case-insensitive"
        );
    }

    #[test]
    fn a_star_prefix_bypass_entry_matches_subdomains() {
        // `*.example.com` is the spelling users type into `NO_PROXY` and into
        // the Windows bypass list, and the doc comment promises it. Without the
        // star being stripped it degrades to a literal comparison against the
        // string "*.example.com" and silently matches nothing.
        let bypass = ProxyBypass::parse("*.example.com");
        assert!(bypass.matches("a.example.com", 443));
        assert!(bypass.matches("deep.nested.example.com", 443));
        // `*.` in a certificate sense covers subdomains only, not the apex.
        assert!(
            !bypass.matches("example.com", 443),
            "a star entry covers subdomains, not the apex domain"
        );
        assert!(!bypass.matches("notexample.com", 443));
        assert!(!bypass.matches("example.com.evil.test", 443));
    }

    #[test]
    fn a_dot_and_star_bypass_entry_agree() {
        // Both spellings appear in real `NO_PROXY` values, so they must not
        // disagree about the apex host.
        assert!(ProxyBypass::parse(".example.com").matches("example.com", 443));
        assert!(ProxyBypass::parse("*.example.com").matches("a.example.com", 443));
        assert!(ProxyBypass::parse(".example.com").matches("a.example.com", 443));
    }

    #[test]
    fn a_star_alone_bypasses_everything() {
        let bypass = ProxyBypass::parse("*");
        assert!(bypass.matches("anything.test", 443));
        assert!(bypass.matches("10.0.0.1", 8443));
    }

    #[test]
    fn an_empty_bypass_matches_nothing() {
        let bypass = ProxyBypass::default();
        assert!(bypass.is_empty());
        assert!(!bypass.matches("example.com", 443));
    }

    #[test]
    fn source_labels_separate_the_environment_from_the_os() {
        assert!(ProxySource::Pac.is_system());
        assert!(ProxySource::Wpad.is_system());
        assert!(ProxySource::SystemStatic.is_system());
        assert!(
            !ProxySource::Environment.is_system(),
            "an inherited variable is not the operating system policy"
        );
        assert!(!ProxySource::Manual.is_system());
    }
}
