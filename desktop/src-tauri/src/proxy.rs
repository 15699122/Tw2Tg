//! Outbound proxy resolution and HTTP client construction.
//!
//! One module owns every outbound decision so the Settings page, the Telegram
//! transport, and the aria2 release download cannot disagree about the route.
//!
//! Three rules shape the design:
//!
//! * Resolution must not run on the UI thread. Windows PAC/WPAD discovery can
//!   block for seconds, so callers run [`ProxyResolver::resolve`] off the UI
//!   thread and report the result back.
//! * A client must not be rebuilt per request. [`ProxyHttpClient`] caches one
//!   client per resolved route.
//! * `System` may only claim what the platform can actually deliver. A resolver
//!   that cannot honor the policy reports a failure instead of connecting
//!   direct.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use xarchive_core::{
    ProxyBypass, ProxyCandidate, ProxyDecision, ProxyMode, ProxyResolution, ProxySource,
};

/// Resolve a destination URL into a routing decision.
///
/// A resolver that cannot honor the requested policy returns
/// [`ProxyDecision::ResolutionFailed`] or [`ProxyDecision::Unsupported`], never
/// [`ProxyDecision::Direct`], so a policy failure cannot masquerade as success.
pub(crate) trait ProxyResolver: Send + Sync {
    fn resolve(&self, url: &str) -> ProxyDecision;

    /// The ordered candidate list for a destination.
    ///
    /// The default collapses the single-route decision so a resolver that only
    /// has one route keeps working without reimplementing this method.
    fn resolve_ordered(&self, url: &str) -> ProxyResolution {
        into_resolution(self.resolve(url))
    }

    /// A redacted description of the configuration this resolver will use.
    fn describe(&self) -> SystemProxyDescription {
        SystemProxyDescription::default()
    }

    /// The monotonically increasing configuration revision, used to invalidate
    /// cached clients when the operating system changes its proxy settings.
    fn generation(&self) -> u64 {
        0
    }
}

/// A redacted summary of the system proxy configuration, for the settings page.
///
/// Every field is display-safe. A field is `None` when the platform cannot
/// report it, which is deliberately different from reporting "disabled".
#[derive(Clone, Debug, Default, serde::Serialize)]
pub(crate) struct SystemProxyDescription {
    /// Which resolver backs `System`, so the settings page never claims PAC
    /// support the platform cannot deliver.
    pub backend: String,
    /// Whether the process environment supplies a proxy.
    pub environment_proxy: bool,
    /// The effective `no_proxy` entries, lowercased.
    pub bypass: Vec<String>,
    /// Whether the OS asked for automatic discovery (WPAD).
    pub auto_detect: bool,
    /// The configured PAC URL, redacted.
    pub pac_url: Option<String>,
    /// PAC/WPAD source state reported by the platform.
    pub pac_state: Option<String>,
    /// Static system proxy endpoints, redacted.
    pub static_proxies: Vec<String>,
    /// The OS proxy bypass list, redacted.
    pub system_bypass: Option<String>,
    /// The resolver's configuration revision.
    pub generation: u64,
}

/// Whether a child process can honor the active system policy on its own.
///
/// gallery-dl and aria2 read proxy environment variables once at start, so they
/// can only follow a policy that has one answer for every host. A PAC or WPAD
/// policy has a different answer per host, and letting the child "discover the
/// platform proxy" from the environment would silently bypass it. This is why
/// `System` mode is refused for those sources instead of quietly going direct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChildProxyCoverage {
    /// The child inherits the environment and can follow the policy itself.
    InheritsEnvironment,
    /// The policy is per-URL, so the child cannot evaluate it. The caller must
    /// refuse the operation rather than let the child go direct.
    RequiresCallerRouting,
    /// `System` is not the active mode; the mode itself is absolute.
    NotApplicable,
}

/// The destination used to ask the platform resolver which policy is active.
///
/// A long-lived child fetches many hosts, so this probe only establishes *which*
/// policy applies; the per-request route is resolved where the URL is known.
pub(crate) const SIDECAR_PROXY_PROBE_URL: &str = "https://x.com/";

/// The resolver used when the platform offers nothing beyond the process
/// environment.
///
/// This is honest about its limits: it honors `NO_PROXY` and distinguishes the
/// HTTP and HTTPS variables, but it cannot know about a registry, a PAC URL, or
/// WPAD. Windows uses the official OS resolver instead; see
/// `system_proxy_resolver.rs`.
pub(crate) struct EnvironmentProxyResolver;

impl EnvironmentProxyResolver {
    /// The candidate for one URL, or `None` when the environment asks for a
    /// direct connection to it.
    fn candidate_for(&self, url: &str) -> Option<ProxyCandidate> {
        let (scheme, host, port) = split_destination(url)?;
        let bypass = ProxyBypass::parse(&first_env(&["NO_PROXY", "no_proxy"]).unwrap_or_default());
        if bypass.matches(&host, port) {
            return None;
        }
        // Scheme matters: sending HTTPS traffic to an `HTTP_PROXY` that was only
        // meant for plain HTTP silently widens what the operator configured.
        let (primary, secondary) = if scheme == "http" {
            ("HTTP_PROXY", "http_proxy")
        } else {
            ("HTTPS_PROXY", "https_proxy")
        };
        first_env(&[primary, secondary])
            .or_else(|| first_env(&["ALL_PROXY", "all_proxy"]))
            .map(|value| ProxyCandidate::Http(with_default_scheme(&value)))
            .filter(|candidate| {
                !candidate
                    .endpoint()
                    .is_some_and(|endpoint| endpoint.trim().is_empty())
            })
    }
}

impl ProxyResolver for EnvironmentProxyResolver {
    fn resolve(&self, url: &str) -> ProxyDecision {
        match self.candidate_for(url) {
            // No environment proxy for this URL is a real answer: the
            // environment asked for a direct connection, not a failure.
            None => ProxyDecision::Direct,
            Some(candidate) => {
                ProxyDecision::Proxy(candidate.endpoint().unwrap_or_default().to_owned())
            }
        }
    }

    fn resolve_ordered(&self, url: &str) -> ProxyResolution {
        match self.candidate_for(url) {
            None => ProxyResolution {
                candidates: vec![ProxyCandidate::Direct],
                source: ProxySource::Environment,
                reason: None,
            },
            Some(candidate) => ProxyResolution {
                candidates: vec![candidate],
                source: ProxySource::Environment,
                reason: None,
            },
        }
    }

    fn describe(&self) -> SystemProxyDescription {
        let proxy = first_env(&["HTTPS_PROXY", "https_proxy"])
            .or_else(|| first_env(&["HTTP_PROXY", "http_proxy"]))
            .or_else(|| first_env(&["ALL_PROXY", "all_proxy"]));
        SystemProxyDescription {
            backend: "environment".to_owned(),
            environment_proxy: proxy.is_some(),
            bypass: ProxyBypass::parse(&first_env(&["NO_PROXY", "no_proxy"]).unwrap_or_default())
                .entries()
                .to_vec(),
            auto_detect: false,
            pac_url: None,
            pac_state: Some("unsupported".to_owned()),
            static_proxies: proxy
                .map(|value| ProxyCandidate::Http(with_default_scheme(&value)).redacted())
                .into_iter()
                .collect(),
            system_bypass: first_env(&["NO_PROXY", "no_proxy"]),
            generation: 0,
        }
    }
}

/// Lift a single-route decision into an ordered resolution, for resolvers that
/// only produce one route.
fn into_resolution(decision: ProxyDecision) -> ProxyResolution {
    let source = ProxySource::Environment;
    let (candidates, reason) = match decision {
        ProxyDecision::Direct => (vec![ProxyCandidate::Direct], None),
        ProxyDecision::Proxy(value) => (vec![ProxyCandidate::Http(value)], None),
        // A refused decision stays refused; it must not become a direct route,
        // and the original cause is kept so the error can name it.
        ProxyDecision::ResolutionFailed(reason) => (Vec::new(), Some(reason)),
        ProxyDecision::Unsupported(reason) => (Vec::new(), Some(reason)),
    };
    ProxyResolution {
        candidates,
        source,
        reason,
    }
}

/// Whether this build can route through a SOCKS proxy.
///
/// The `socks` feature of reqwest is enabled in the manifest, so a PAC result
/// naming SOCKS is usable here rather than a reason to refuse the request.
const TRANSPORT_SUPPORTS_SOCKS: bool = true;

/// The first non-empty value among the given environment names.
fn first_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

/// Split a URL into `(scheme, host, port)` without pulling in a URL parser.
///
/// The resolver runs for every outbound request, so this stays small and only
/// extracts what routing needs.
fn split_destination(url: &str) -> Option<(String, String, u16)> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme = scheme.trim().to_ascii_lowercase();
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // Strip any userinfo; credentials in a destination are not our business.
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let default_port = match scheme.as_str() {
        "http" | "ws" => 80,
        "https" | "wss" => 443,
        _ => return None,
    };
    // A bracketed IPv6 literal keeps its brackets for the bypass matcher.
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port_text)) if !host.ends_with(']') => {
            (host.to_owned(), port_text.parse::<u16>().ok()?)
        }
        _ => (authority.to_owned(), default_port),
    };
    (!host.is_empty()).then_some((scheme, host, port))
}

/// Give a bare `host:port` a scheme so reqwest and the child processes agree.
fn with_default_scheme(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.contains("://") {
        return trimmed.to_owned();
    }
    if trimmed.starts_with("socks5:") || trimmed.starts_with("socks4:") {
        return format!("socks5://{trimmed}");
    }
    format!("http://{trimmed}")
}

/// The resolver this platform provides for the `System` mode.
///
/// On Windows this is the official OS resolver, which understands the static
/// system proxy, bypass lists, PAC, and WPAD. Elsewhere the process
/// environment is genuinely all the platform offers.
pub(crate) fn platform_resolver() -> Box<dyn ProxyResolver> {
    #[cfg(windows)]
    {
        crate::system_proxy_resolver::SystemProxyResolver::new()
    }
    #[cfg(not(windows))]
    {
        Box::new(EnvironmentProxyResolver)
    }
}

/// The ordered route for a destination, given the configured mode.
pub(crate) fn resolve(
    mode: ProxyMode,
    manual_proxy: Option<String>,
    resolver: &dyn ProxyResolver,
    url: &str,
) -> ProxyResolution {
    match mode {
        ProxyMode::Direct => ProxyResolution::explicit_direct(),
        ProxyMode::Manual => match manual_proxy
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        {
            Some(value) => ProxyResolution::manual(value),
            // `Manual` with no value cannot be honored, and inventing a route
            // here would silently bypass the user's policy.
            None => ProxyResolution::unresolved(ProxySource::Manual),
        },
        ProxyMode::System => resolver.resolve_ordered(url),
    }
}

/// The user-facing reason a decision must not proceed.
fn failure_reason(decision: &ProxyDecision) -> Option<String> {
    match decision {
        ProxyDecision::ResolutionFailed(reason) => Some(format!(
            "system proxy resolution failed ({reason}); the request was not sent. Choose Manual or Direct instead."
        )),
        ProxyDecision::Unsupported(reason) => {
            Some(format!("proxy mode is not supported here ({reason})"))
        }
        _ => None,
    }
}

/// Build one client for a candidate.
///
/// `Direct` calls `no_proxy()` because reqwest would otherwise fall back to the
/// environment. A proxy sets the value explicitly, which also disables reqwest's
/// own matcher so an ordered PAC list is not reinterpreted a second time.
fn build_client_for_candidate(
    candidate: &ProxyCandidate,
    timeout: Duration,
) -> Result<reqwest::blocking::Client, String> {
    let builder = reqwest::blocking::Client::builder().timeout(timeout);
    let builder = match candidate {
        ProxyCandidate::Direct => builder.no_proxy(),
        proxy => {
            let endpoint = normalize_candidate(proxy)
                .ok_or_else(|| format!("{} has no proxy endpoint", proxy.redacted()))?;
            if endpoint.is_empty() {
                return Err(format!("{} has an empty proxy endpoint", proxy.redacted()));
            }
            let proxy = reqwest::Proxy::all(endpoint.clone())
                .map_err(|error| format!("failed to apply the proxy {endpoint}: {error}"))?;
            builder.proxy(proxy)
        }
    };
    builder
        .build()
        .map_err(|error| format!("failed to create download client: {error}"))
}

/// Give a PAC proxy endpoint a scheme reqwest accepts.
///
/// PAC returns bare `host:port`, which reqwest rejects. The candidate kind is
/// what decides the scheme: a `Socks` candidate becomes `socks5://` because
/// reqwest tells SOCKS and HTTP apart by scheme, and inferring it from the
/// hostname would misroute an ordinary proxy that happens to be named
/// `socks.example`.
pub(crate) fn normalize_candidate(candidate: &ProxyCandidate) -> Option<String> {
    match candidate {
        ProxyCandidate::Direct => None,
        ProxyCandidate::Http(endpoint) => Some(with_http_scheme(endpoint)),
        ProxyCandidate::Socks(endpoint) => Some(format!("socks5://{}", authority(endpoint))),
    }
}

/// The `host:port` authority, without any scheme or path.
fn authority(value: &str) -> &str {
    let trimmed = value.trim();
    let without_scheme = trimmed.split_once("://").map_or(trimmed, |(_, rest)| rest);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim_start_matches("socks5:")
        .trim_start_matches("socks4:")
        .trim()
}

/// Give a value an HTTP scheme, preserving an explicit scheme it already has.
///
/// A stored manual value may already be `socks5://…`; flattening that to HTTP
/// would silently route a SOCKS proxy through an HTTP CONNECT.
fn with_http_scheme(value: &str) -> String {
    let trimmed = value.trim();
    if let Some((scheme, rest)) = trimmed.split_once("://") {
        let scheme = scheme.trim().to_ascii_lowercase();
        if !rest.trim().is_empty() {
            return format!("{scheme}://{}", rest.trim());
        }
        return String::new();
    }
    let authority = authority(trimmed);
    if authority.is_empty() {
        return String::new();
    }
    format!("http://{authority}")
}

/// A blocking HTTP client set that applies the configured mode.
///
/// One instance is created when the configuration is applied and is then shared
/// by every boundary, so a mode change cannot leave one boundary on the old
/// route.
pub(crate) struct ProxyHttpClient {
    resolver: Box<dyn ProxyResolver>,
    mode: ProxyMode,
    manual_proxy: Option<String>,
    clients: Mutex<HashMap<String, reqwest::blocking::Client>>,
    timeout: Duration,
}

impl ProxyHttpClient {
    pub(crate) fn new(
        mode: ProxyMode,
        manual_proxy: Option<String>,
        timeout: Duration,
        resolver: Box<dyn ProxyResolver>,
    ) -> Self {
        Self {
            resolver,
            mode,
            manual_proxy,
            clients: Mutex::new(HashMap::new()),
            timeout,
        }
    }

    pub(crate) fn mode(&self) -> ProxyMode {
        self.mode
    }

    /// Build the client set for a saved configuration.
    ///
    /// The platform resolver is chosen here so the Windows build can swap in the
    /// WinHTTP-backed implementation without any caller changing.
    pub(crate) fn from_config(network: &crate::config::NetworkConfig) -> Self {
        let timeout = std::time::Duration::from_secs(network.transfer_timeout_seconds.max(1));
        Self::new(
            network.proxy_mode,
            network.normalized_proxy(),
            timeout,
            platform_resolver(),
        )
    }

    /// The ordered route that would be applied to `url`.
    ///
    /// This is what a diagnostic command reports. It performs no I/O beyond the
    /// resolver, so a caller must still keep it off the UI thread.
    pub(crate) fn resolution_for(&self, url: &str) -> ProxyResolution {
        resolve(
            self.mode,
            self.manual_proxy.clone(),
            self.resolver.as_ref(),
            url,
        )
    }

    /// A redacted description of the system configuration behind `System`.
    pub(crate) fn describe_system(&self) -> SystemProxyDescription {
        self.resolver.describe()
    }

    /// Whether a child process can honor the configured mode by itself.
    ///
    /// `Direct` and `Manual` are absolute, so a child follows them from the
    /// environment alone. `System` depends on what the platform actually
    /// reports, which is why it is inspected rather than assumed.
    pub(crate) fn child_proxy_coverage(&self, url: &str) -> ChildProxyCoverage {
        if self.mode != ProxyMode::System {
            return ChildProxyCoverage::NotApplicable;
        }
        let resolution = self.resolution_for(url);
        // An unresolved policy is not a coverage answer; the operation is
        // refused separately, so it is not silently reported as inheritable.
        if resolution.is_empty() {
            return ChildProxyCoverage::RequiresCallerRouting;
        }
        if resolution.source.is_per_url() {
            ChildProxyCoverage::RequiresCallerRouting
        } else {
            ChildProxyCoverage::InheritsEnvironment
        }
    }

    /// Refuse an operation that a child process would silently bypass.
    ///
    /// The message names the actual limitation instead of telling the user to
    /// retry, because retrying cannot make the child evaluate a PAC script.
    pub(crate) fn require_child_routable(&self, url: &str, transport: &str) -> Result<(), String> {
        match self.child_proxy_coverage(url) {
            ChildProxyCoverage::RequiresCallerRouting => {
                let resolution = self.resolution_for(url);
                if resolution.is_empty() {
                    return Err(format!(
                        "the {} proxy policy could not be resolved, so {transport} was not started",
                        resolution.source.as_str()
                    ));
                }
                Err(format!(
                    "the {} proxy policy returns a different route per host, and {transport} \
                     reads proxy settings only once, so it cannot follow it. Choose Manual or \
                     Direct in the proxy settings, or start a job whose target is a single host.",
                    resolution.source.as_str()
                ))
            }
            _ => Ok(()),
        }
    }

    /// Clients for every candidate in an ordered resolution, in PAC order.
    ///
    /// Callers that can retry a request that provably never reached the server
    /// walk this list. Callers that cannot prove that must use
    /// [`ProxyHttpClient::client_for`] instead, so a retry cannot duplicate a
    /// side effect.
    pub(crate) fn candidates_for(
        &self,
        url: &str,
    ) -> Result<Vec<(reqwest::blocking::Client, ProxyCandidate)>, String> {
        let resolution = self.resolution_for(url);
        if resolution.is_empty() {
            return Err(
                failure_reason(&resolution.to_decision()).unwrap_or_else(|| {
                    format!(
                        "the {} proxy policy resolved to no route; the request was not sent",
                        resolution.source.as_str()
                    )
                }),
            );
        }
        // A candidate kind this transport cannot use is dropped; if nothing
        // survives, the request fails rather than quietly going direct.
        let candidates = resolution
            .candidates_for(TRANSPORT_SUPPORTS_SOCKS)
            .ok_or_else(|| {
            format!(
                "the {} proxy policy only returned proxy kinds this build cannot use; the request was not sent",
                resolution.source.as_str()
            )
        })?;
        let timeout = self.timeout;
        let mut clients = self
            .clients
            .lock()
            .map_err(|_| "proxy client cache poisoned".to_owned())?;
        candidates
            .into_iter()
            .map(|candidate| {
                let key = format!(
                    "{}|g{}",
                    match &candidate {
                        ProxyCandidate::Direct => "direct".to_owned(),
                        other => format!("proxy:{}", other.endpoint().unwrap_or_default()),
                    },
                    self.resolver.generation()
                );
                let client = match clients.get(&key) {
                    Some(client) => client.clone(),
                    None => {
                        let client = build_client_for_candidate(&candidate, timeout)?;
                        clients.insert(key, client.clone());
                        client
                    }
                };
                Ok((client, candidate))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A resolver that answers from a fixed table, so routing can be asserted
    /// without a real proxy, a real PAC file, or a real Windows policy.
    struct StaticResolver(ProxyDecision);

    impl ProxyResolver for StaticResolver {
        fn resolve(&self, _url: &str) -> ProxyDecision {
            self.0.clone()
        }
    }

    fn static_resolver(decision: ProxyDecision) -> Box<dyn ProxyResolver> {
        Box::new(StaticResolver(decision))
    }

    /// A resolver that always fails to produce a route, standing in for a PAC
    /// policy that could not be resolved.
    struct UnresolvedResolver;

    impl ProxyResolver for UnresolvedResolver {
        fn resolve(&self, _url: &str) -> ProxyDecision {
            ProxyDecision::ResolutionFailed("the test policy has no route".to_owned())
        }
    }

    /// A resolver returning a fixed ordered list, standing in for a PAC result.
    struct OrderedResolver(Vec<ProxyCandidate>);

    impl ProxyResolver for OrderedResolver {
        fn resolve(&self, _url: &str) -> ProxyDecision {
            ProxyResolution {
                candidates: self.0.clone(),
                source: ProxySource::Pac,
                reason: None,
            }
            .to_decision()
        }
        fn resolve_ordered(&self, _url: &str) -> ProxyResolution {
            ProxyResolution {
                candidates: self.0.clone(),
                source: ProxySource::Pac,
                reason: None,
            }
        }
    }

    fn ordered_resolver(candidates: Vec<ProxyCandidate>) -> Box<dyn ProxyResolver> {
        Box::new(OrderedResolver(candidates))
    }

    #[test]
    fn direct_never_consults_the_resolver() {
        let decision = resolve(
            ProxyMode::Direct,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            &*static_resolver(ProxyDecision::Proxy("http://p:1".to_owned())),
            "https://example.invalid/",
        )
        .to_decision();
        assert_eq!(decision, ProxyDecision::Direct);
    }

    #[test]
    fn manual_never_consults_the_resolver() {
        let decision = resolve(
            ProxyMode::Manual,
            Some("http://proxy.example:8080".to_owned()),
            &*static_resolver(ProxyDecision::Direct),
            "https://example.invalid/",
        )
        .to_decision();
        assert_eq!(
            decision,
            ProxyDecision::Proxy("http://proxy.example:8080".to_owned())
        );
    }

    #[test]
    fn manual_without_a_value_is_unsupported_rather_than_direct() {
        for manual in [None, Some("   ".to_owned())] {
            let expected = format!("{manual:?}");
            let decision = resolve(
                ProxyMode::Manual,
                manual,
                &*static_resolver(ProxyDecision::Direct),
                "https://example.invalid/",
            );
            let collapsed = decision.to_decision();
            assert!(collapsed.is_failure(), "{expected} must not become direct");
            assert!(!collapsed.is_direct());
        }
    }

    #[test]
    fn system_delegates_to_the_platform_resolver() {
        let decision = resolve(
            ProxyMode::System,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            &*static_resolver(ProxyDecision::Proxy("http://p:1".to_owned())),
            "https://example.invalid/",
        )
        .to_decision();
        assert_eq!(
            decision,
            ProxyDecision::Proxy("http://p:1".to_owned()),
            "System must not pin the stored manual value"
        );
    }

    #[test]
    fn a_system_resolution_failure_blocks_the_request() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            static_resolver(ProxyDecision::ResolutionFailed(
                "PAC unreachable".to_owned(),
            )),
        );
        let error = client
            .candidates_for("https://example.invalid/")
            .expect_err("a failed resolution must not produce a client");
        assert!(error.contains("PAC unreachable"), "unclear: {error}");
        assert!(
            error.contains("the request was not sent"),
            "the error must state the request was refused: {error}"
        );
    }

    #[test]
    fn an_unsupported_mode_blocks_the_request() {
        let client = ProxyHttpClient::new(
            ProxyMode::Manual,
            None,
            Duration::from_secs(5),
            static_resolver(ProxyDecision::Direct),
        );
        assert!(client.candidates_for("https://example.invalid/").is_err());
    }

    #[test]
    fn one_route_builds_one_client_and_is_reused() {
        let client = ProxyHttpClient::new(
            ProxyMode::Manual,
            Some("http://proxy.example:8080".to_owned()),
            Duration::from_secs(5),
            static_resolver(ProxyDecision::Direct),
        );
        client
            .candidates_for("https://first.invalid/")
            .expect("first");
        client
            .candidates_for("https://second.invalid/")
            .expect("second");
        let cached = client.clients.lock().expect("cache").len();
        assert_eq!(cached, 1, "one route must not build a second client");
    }

    #[test]
    fn two_routes_are_cached_separately() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            ordered_resolver(vec![
                ProxyCandidate::Http("first.example:8080".to_owned()),
                ProxyCandidate::Http("second.example:8080".to_owned()),
            ]),
        );
        let first = client
            .candidates_for("https://example.invalid/")
            .expect("candidates");
        assert_eq!(first.len(), 2, "both proxies are part of one resolution");
        assert_eq!(client.clients.lock().expect("cache").len(), 2);
        // Resolving again must reuse both, not build four clients.
        client
            .candidates_for("https://example.invalid/")
            .expect("candidates");
        assert_eq!(client.clients.lock().expect("cache").len(), 2);
    }

    #[test]
    fn a_direct_candidate_is_cached_apart_from_a_proxy() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            ordered_resolver(vec![
                ProxyCandidate::Http("proxy.example:8080".to_owned()),
                ProxyCandidate::Direct,
            ]),
        );
        client
            .candidates_for("https://example.invalid/")
            .expect("candidates");
        assert_eq!(
            client.clients.lock().expect("cache").len(),
            2,
            "direct and proxy must never share a cached client"
        );
    }

    #[test]
    fn a_decision_rendering_never_carries_the_configured_credentials() {
        let client = ProxyHttpClient::new(
            ProxyMode::Manual,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            Duration::from_secs(5),
            static_resolver(ProxyDecision::Direct),
        );
        let redacted = client
            .resolution_for("https://example.invalid/")
            .to_decision()
            .redacted_proxy()
            .expect("redacted");
        assert!(
            !redacted.contains("s3cret"),
            "credentials must not survive redaction: {redacted}"
        );
    }

    #[test]
    fn the_environment_resolver_reports_a_route_or_direct() {
        let decision = EnvironmentProxyResolver.resolve("https://example.invalid/");
        assert!(
            decision.is_direct() || decision.proxy().is_some(),
            "the environment resolver must produce a usable decision"
        );
    }

    #[test]
    fn every_reachable_candidate_builds_a_client() {
        for candidate in [
            ProxyCandidate::Direct,
            ProxyCandidate::Http("http://proxy.example:8080".to_owned()),
            ProxyCandidate::Socks("s.example:1080".to_owned()),
        ] {
            assert!(
                build_client_for_candidate(&candidate, Duration::from_secs(5)).is_ok(),
                "{candidate:?} must build"
            );
        }
    }

    #[test]
    fn an_unresolved_system_policy_refuses_to_build_a_client() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            Box::new(UnresolvedResolver),
        );
        let error = client
            .candidates_for("https://example.invalid/")
            .expect_err("an unresolved policy must not build a client");
        assert!(error.contains("resolution failed"), "unclear: {error}");
        assert!(client.candidates_for("https://example.invalid/").is_err());
    }

    #[test]
    fn a_manual_mode_without_a_value_refuses_instead_of_going_direct() {
        let client = ProxyHttpClient::new(
            ProxyMode::Manual,
            None,
            Duration::from_secs(5),
            static_resolver(ProxyDecision::Direct),
        );
        // The resolver would say direct, but `Manual` is absolute: an absent
        // value must not quietly fall through to the environment's answer.
        assert!(client.candidates_for("https://example.invalid/").is_err());
    }

    #[test]
    fn an_ordered_list_is_returned_in_priority_order() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            ordered_resolver(vec![
                ProxyCandidate::Http("first.example:8080".to_owned()),
                ProxyCandidate::Direct,
            ]),
        );
        let candidates: Vec<String> = client
            .candidates_for("https://example.invalid/")
            .expect("candidates")
            .into_iter()
            .map(|(_, candidate)| candidate.redacted())
            .collect();
        assert_eq!(candidates, ["PROXY first.example:8080", "DIRECT"]);
    }

    #[test]
    fn a_socks_candidate_builds_a_client_because_the_transport_supports_it() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            ordered_resolver(vec![ProxyCandidate::Socks("s.example:1080".to_owned())]),
        );
        // reqwest's `socks` feature is enabled, so a PAC result naming SOCKS is
        // usable. Refusing it would break a valid corporate policy.
        let candidates = client
            .candidates_for("https://example.invalid/")
            .expect("a SOCKS candidate is supported by this build");
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            normalize_candidate(&candidates[0].1).as_deref(),
            Some("socks5://s.example:1080"),
            "the SOCKS kind must reach reqwest as a socks5 scheme"
        );
    }

    #[test]
    fn a_socks_endpoint_named_like_one_is_still_treated_as_socks() {
        // The kind comes from PAC, not from the hostname.
        assert_eq!(
            normalize_candidate(&ProxyCandidate::Socks("socks.example:1080".to_owned())).as_deref(),
            Some("socks5://socks.example:1080")
        );
        assert_eq!(
            normalize_candidate(&ProxyCandidate::Http("socks.example:1080".to_owned())).as_deref(),
            Some("http://socks.example:1080")
        );
        assert_eq!(normalize_candidate(&ProxyCandidate::Direct), None);
    }

    #[test]
    fn a_generation_change_drops_the_cached_client() {
        struct CountingResolver {
            generation: std::sync::atomic::AtomicU64,
        }
        impl ProxyResolver for CountingResolver {
            fn resolve(&self, _url: &str) -> ProxyDecision {
                ProxyDecision::Direct
            }
            fn generation(&self) -> u64 {
                self.generation.load(std::sync::atomic::Ordering::SeqCst)
            }
        }
        let resolver = std::sync::Arc::new(CountingResolver {
            generation: std::sync::atomic::AtomicU64::new(1),
        });
        struct Shared(std::sync::Arc<CountingResolver>);
        impl ProxyResolver for Shared {
            fn resolve(&self, url: &str) -> ProxyDecision {
                self.0.resolve(url)
            }
            fn generation(&self) -> u64 {
                self.0.generation()
            }
        }
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            Box::new(Shared(resolver.clone())),
        );
        client
            .candidates_for("https://first.invalid/")
            .expect("first");
        client
            .candidates_for("https://second.invalid/")
            .expect("second");
        assert_eq!(client.clients.lock().expect("cache").len(), 1);
        // A system proxy change must not leave the old route cached.
        resolver
            .generation
            .store(2, std::sync::atomic::Ordering::SeqCst);
        client
            .candidates_for("https://third.invalid/")
            .expect("third");
        assert_eq!(
            client.clients.lock().expect("cache").len(),
            2,
            "a new configuration generation must build a new client"
        );
    }

    #[test]
    fn a_socks_candidate_survives_the_route_that_a_transport_uses() {
        // Regression: reading the route through the single-value decision loses
        // the SOCKS kind, and re-normalizing the string turns a SOCKS proxy into
        // an HTTP one, which silently misroutes the request.
        let resolution = ProxyResolution {
            candidates: vec![ProxyCandidate::Socks("s.example:1080".to_owned())],
            source: ProxySource::Pac,
            reason: None,
        };
        let endpoint = normalize_candidate(resolution.primary().expect("primary"))
            .expect("a socks candidate has an endpoint");
        assert_eq!(endpoint, "socks5://s.example:1080");
        assert!(
            !endpoint.starts_with("http://"),
            "a SOCKS policy must not be re-normalized as HTTP: {endpoint}"
        );
    }

    #[test]
    fn a_direct_candidate_stays_direct_for_a_transport() {
        let resolution = ProxyResolution {
            candidates: vec![ProxyCandidate::Direct],
            source: ProxySource::Pac,
            reason: None,
        };
        assert_eq!(
            normalize_candidate(resolution.primary().expect("primary")),
            None,
            "Direct must reach a transport as 'no proxy', not as a URL"
        );
    }

    #[test]
    fn a_destination_splits_into_scheme_host_and_port() {
        assert_eq!(
            split_destination("https://api.example:8443/v1"),
            Some(("https".to_owned(), "api.example".to_owned(), 8443))
        );
        assert_eq!(
            split_destination("http://example.invalid"),
            Some(("http".to_owned(), "example.invalid".to_owned(), 80))
        );
        assert_eq!(
            split_destination("wss://example.invalid/socket"),
            Some(("wss".to_owned(), "example.invalid".to_owned(), 443))
        );
        // Userinfo must never become part of the host.
        assert_eq!(
            split_destination("https://user:pw@example.invalid/x"),
            Some(("https".to_owned(), "example.invalid".to_owned(), 443))
        );
        assert_eq!(split_destination("not-a-url"), None);
        assert_eq!(split_destination("ftp://example.invalid"), None);
    }

    /// A resolver reporting a per-URL source, standing in for PAC or WPAD.
    struct PerUrlResolver;

    impl ProxyResolver for PerUrlResolver {
        fn resolve(&self, _url: &str) -> ProxyDecision {
            ProxyDecision::Proxy("a.example:8080".to_owned())
        }
        fn resolve_ordered(&self, _url: &str) -> ProxyResolution {
            ProxyResolution {
                candidates: vec![ProxyCandidate::Http("a.example:8080".to_owned())],
                source: ProxySource::Pac,
                reason: None,
            }
        }
    }

    #[test]
    fn a_per_url_policy_cannot_be_handed_to_a_child_process() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            Box::new(PerUrlResolver),
        );
        assert_eq!(
            client.child_proxy_coverage("https://x.com/"),
            ChildProxyCoverage::RequiresCallerRouting
        );
        // The guard is what stops gallery-dl from silently going direct.
        let error = client
            .require_child_routable("https://x.com/", "the Sidecar worker")
            .expect_err("a PAC policy must not be inherited by a child");
        assert!(
            error.contains("different route per host"),
            "unclear: {error}"
        );
        assert!(
            error.contains("Manual or Direct"),
            "must be actionable: {error}"
        );
    }

    #[test]
    fn a_static_environment_policy_is_inheritable() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            Box::new(EnvironmentProxyResolver),
        );
        // The environment has no per-URL behaviour, so a child can follow it.
        assert_eq!(
            client.child_proxy_coverage("https://x.com/"),
            ChildProxyCoverage::InheritsEnvironment
        );
        assert!(
            client
                .require_child_routable("https://x.com/", "the Sidecar worker")
                .is_ok()
        );
    }

    #[test]
    fn an_absolute_mode_never_depends_on_child_coverage() {
        for mode in [ProxyMode::Direct, ProxyMode::Manual] {
            let client = ProxyHttpClient::new(
                mode,
                Some("http://proxy.example:8080".to_owned()),
                Duration::from_secs(5),
                Box::new(PerUrlResolver),
            );
            assert_eq!(
                client.child_proxy_coverage("https://x.com/"),
                ChildProxyCoverage::NotApplicable
            );
            assert!(
                client
                    .require_child_routable("https://x.com/", "the Sidecar worker")
                    .is_ok()
            );
        }
    }

    #[test]
    fn an_unresolved_system_policy_blocks_the_child_with_its_own_cause() {
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(5),
            Box::new(UnresolvedResolver),
        );
        let error = client
            .require_child_routable("https://x.com/", "the Sidecar worker")
            .expect_err("an unresolved policy must not start a child");
        assert!(
            error.contains("could not be resolved") && error.contains("not started"),
            "unclear: {error}"
        );
    }

    #[test]
    fn a_per_url_source_is_reported_by_the_shared_contract() {
        assert!(ProxySource::Pac.is_per_url());
        assert!(ProxySource::Wpad.is_per_url());
        assert!(!ProxySource::SystemStatic.is_per_url());
        assert!(!ProxySource::Environment.is_per_url());
    }
}
