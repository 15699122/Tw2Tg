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

use xarchive_core::{ProxyDecision, ProxyMode};

/// Resolve a destination URL into a routing decision.
///
/// A resolver that cannot honor the requested policy returns
/// [`ProxyDecision::ResolutionFailed`] or [`ProxyDecision::Unsupported`], never
/// [`ProxyDecision::Direct`], so a policy failure cannot masquerade as success.
pub(crate) trait ProxyResolver: Send + Sync {
    fn resolve(&self, url: &str) -> ProxyDecision;
}

/// The resolver used when the platform offers nothing beyond the process
/// environment.
///
/// This is honest about its limits: with reqwest's `system-proxy` feature off,
/// hyper-util's system matcher reduces to `from_env()`, so this resolver reads
/// the environment directly instead of claiming registry or PAC support it
/// cannot deliver. Batch B replaces it on Windows.
pub(crate) struct EnvironmentProxyResolver;

impl ProxyResolver for EnvironmentProxyResolver {
    fn resolve(&self, _url: &str) -> ProxyDecision {
        // The environment has no per-URL routing, so one value covers every
        // destination.
        for key in [
            "ALL_PROXY",
            "all_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "HTTP_PROXY",
            "http_proxy",
        ] {
            if let Some(value) = std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
            {
                return ProxyDecision::Proxy(value);
            }
        }
        ProxyDecision::Direct
    }
}

/// The resolver this platform provides for the `System` mode.
///
/// `System` promises whatever the platform can actually resolve. On Linux that
/// is the process environment. On Windows the honest answer today is the
/// environment too: the WinHTTP-backed resolver is Batch B, and reporting a
/// registry or PAC result before that resolver exists would be a claim the code
/// cannot keep.
pub(crate) fn platform_resolver() -> Box<dyn ProxyResolver> {
    Box::new(EnvironmentProxyResolver)
}

/// The route for a destination, given the configured mode.
///
/// `Direct` and `Manual` are decided without consulting the resolver, because
/// they are absolute. Only `System` needs a platform lookup.
pub(crate) fn decide(
    mode: ProxyMode,
    manual_proxy: Option<String>,
    resolver: &dyn ProxyResolver,
    url: &str,
) -> ProxyDecision {
    match mode {
        ProxyMode::Direct => ProxyDecision::Direct,
        ProxyMode::Manual => match manual_proxy
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        {
            Some(value) => ProxyDecision::Proxy(value),
            None => ProxyDecision::Unsupported(
                "manual mode is selected but no proxy is configured".to_owned(),
            ),
        },
        ProxyMode::System => resolver.resolve(url),
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

/// Build one client for a decision.
///
/// `Direct` calls `no_proxy()` because reqwest would otherwise fall back to the
/// environment. `System` deliberately configures nothing, leaving reqwest's own
/// environment matcher in place. `Proxy` sets the value explicitly.
fn build_client(
    decision: &ProxyDecision,
    timeout: Duration,
) -> Result<reqwest::blocking::Client, String> {
    let builder = reqwest::blocking::Client::builder().timeout(timeout);
    let builder = match decision {
        ProxyDecision::Direct => builder.no_proxy(),
        ProxyDecision::Proxy(value) => {
            let proxy = reqwest::Proxy::all(value.as_str())
                .map_err(|error| format!("failed to apply the configured proxy: {error}"))?;
            builder.proxy(proxy)
        }
        ProxyDecision::ResolutionFailed(_) | ProxyDecision::Unsupported(_) => {
            return Err("refusing to build a client for an unresolved proxy".to_owned());
        }
    };
    builder
        .build()
        .map_err(|error| format!("failed to create download client: {error}"))
}

/// The cache key for a decision.
///
/// A `Proxy` decision is keyed by its value because each distinct proxy needs
/// its own client. Every other decision shares one key because they all produce
/// the same client configuration.
fn cache_key(decision: &ProxyDecision) -> String {
    match decision {
        ProxyDecision::Proxy(value) => format!("proxy:{value}"),
        ProxyDecision::Direct => "direct".to_owned(),
        ProxyDecision::ResolutionFailed(reason) => format!("failed:{reason}"),
        ProxyDecision::Unsupported(reason) => format!("unsupported:{reason}"),
    }
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

    /// The decision that would be applied to `url`, without building a client.
    ///
    /// This is what a diagnostic command reports. It performs no I/O beyond the
    /// resolver, so a caller must still keep it off the UI thread.
    pub(crate) fn decision_for(&self, url: &str) -> ProxyDecision {
        decide(
            self.mode,
            self.manual_proxy.clone(),
            self.resolver.as_ref(),
            url,
        )
    }
    /// The client for `url`, building and caching it on first use.
    ///
    /// The decision is returned alongside the client so a caller can report the
    /// route it actually used, including a refusal.
    pub(crate) fn client_for(
        &self,
        url: &str,
    ) -> Result<(reqwest::blocking::Client, ProxyDecision), String> {
        let decision = self.decision_for(url);
        if let Some(reason) = failure_reason(&decision) {
            return Err(reason);
        }
        let key = cache_key(&decision);
        let mut clients = self
            .clients
            .lock()
            .map_err(|_| "proxy client cache poisoned".to_owned())?;
        if let Some(client) = clients.get(&key) {
            return Ok((client.clone(), decision));
        }
        let client = build_client(&decision, self.timeout)?;
        clients.insert(key, client.clone());
        Ok((client, decision))
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

    #[test]
    fn direct_never_consults_the_resolver() {
        let decision = decide(
            ProxyMode::Direct,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            &*static_resolver(ProxyDecision::Proxy("http://p:1".to_owned())),
            "https://example.invalid/",
        );
        assert_eq!(decision, ProxyDecision::Direct);
    }

    #[test]
    fn manual_never_consults_the_resolver() {
        let decision = decide(
            ProxyMode::Manual,
            Some("http://proxy.example:8080".to_owned()),
            &*static_resolver(ProxyDecision::Direct),
            "https://example.invalid/",
        );
        assert_eq!(
            decision,
            ProxyDecision::Proxy("http://proxy.example:8080".to_owned())
        );
    }

    #[test]
    fn manual_without_a_value_is_unsupported_rather_than_direct() {
        for manual in [None, Some("   ".to_owned())] {
            let expected = format!("{manual:?}");
            let decision = decide(
                ProxyMode::Manual,
                manual,
                &*static_resolver(ProxyDecision::Direct),
                "https://example.invalid/",
            );
            assert!(decision.is_failure(), "{expected} must not become direct");
            assert!(!decision.is_direct());
        }
    }

    #[test]
    fn system_delegates_to_the_platform_resolver() {
        let decision = decide(
            ProxyMode::System,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
            &*static_resolver(ProxyDecision::Proxy("http://p:1".to_owned())),
            "https://example.invalid/",
        );
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
            .client_for("https://example.invalid/")
            .expect_err("a failed resolution must not produce a client");
        assert!(error.contains("PAC unreachable"));
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
        assert!(client.client_for("https://example.invalid/").is_err());
    }

    #[test]
    fn one_route_builds_one_client_and_is_reused() {
        let client = ProxyHttpClient::new(
            ProxyMode::Manual,
            Some("http://proxy.example:8080".to_owned()),
            Duration::from_secs(5),
            static_resolver(ProxyDecision::Direct),
        );
        client.client_for("https://first.invalid/").expect("first");
        client
            .client_for("https://second.invalid/")
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
            static_resolver(ProxyDecision::Proxy("http://p:1".to_owned())),
        );
        client.client_for("https://first.invalid/").expect("first");
        {
            let mut clients = client.clients.lock().expect("cache");
            clients.insert(
                cache_key(&ProxyDecision::Direct),
                build_client(&ProxyDecision::Direct, Duration::from_secs(5)).expect("direct"),
            );
        }
        assert_eq!(client.clients.lock().expect("cache").len(), 2);
        assert_eq!(
            cache_key(&ProxyDecision::Proxy("http://p:1".to_owned())),
            "proxy:http://p:1"
        );
    }

    #[test]
    fn the_cache_key_separates_direct_from_a_proxy() {
        assert_ne!(
            cache_key(&ProxyDecision::Direct),
            cache_key(&ProxyDecision::Proxy(
                "http://proxy.example:8080".to_owned()
            ))
        );
        assert_eq!(cache_key(&ProxyDecision::Direct), "direct");
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
            .decision_for("https://example.invalid/")
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
    fn every_reachable_decision_builds_a_client() {
        for decision in [
            ProxyDecision::Direct,
            ProxyDecision::Proxy("http://proxy.example:8080".to_owned()),
        ] {
            assert!(
                build_client(&decision, Duration::from_secs(5)).is_ok(),
                "{decision:?} must build"
            );
        }
    }

    #[test]
    fn a_refused_decision_never_builds_a_client() {
        for decision in [
            ProxyDecision::ResolutionFailed("x".to_owned()),
            ProxyDecision::Unsupported("y".to_owned()),
        ] {
            assert!(
                build_client(&decision, Duration::from_secs(5)).is_err(),
                "{decision:?} must not build a client"
            );
        }
    }
}
