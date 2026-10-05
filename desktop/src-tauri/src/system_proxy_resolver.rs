//! Windows system proxy resolution built on Microsoft's OS proxy resolver.
//!
//! This is the `System` mode implementation for Windows. It resolves the
//! current user's static proxy, bypass list, PAC, and WPAD the way the operating
//! system does, which is the only way to honor an enterprise policy that routes
//! different hosts differently.
//!
//! Two properties of that resolver shape this adapter:
//!
//! * It returns an *ordered* list. `PROXY a; DIRECT` means "try `a`, then go
//!   direct", so the order is preserved and handed to callers rather than
//!   collapsed.
//! * PAC/WPAD failures fall through to the next layer and can end at `DIRECT`.
//!   The resolver reports configuration state separately, so this adapter
//!   surfaces that state to the settings page instead of presenting a fallback
//!   as a deliberate policy.
//!
//! The resolver is expensive to construct and keeps a change watcher alive, so
//! one process-wide instance is shared.

use std::sync::OnceLock;

use os_proxy_resolver::{
    PacScriptSource, PacSourceState, PlatformProxyConfig, ProxyConfig, ProxyKind,
    ProxyResolver as OfficialResolver, ResolverOptions,
};
use xarchive_core::{
    ProxyBypass, ProxyCandidate, ProxyDecision, ProxyResolution, ProxySource,
    redact_url_credentials,
};

use crate::proxy::{ProxyResolver, SystemProxyDescription};

/// The process-wide resolver, shared because it owns a change watcher.
fn shared() -> &'static OfficialResolver {
    static RESOLVER: OnceLock<OfficialResolver> = OnceLock::new();
    RESOLVER.get_or_init(|| {
        let mut options = ResolverOptions::default();
        // PAC resolution can block for seconds on a slow network. Resolution
        // runs off the UI thread, but a bounded timeout keeps a wedged PAC
        // script from stalling a request indefinitely.
        options.pac_timeout = std::time::Duration::from_secs(5);
        options.resolution_ttl = std::time::Duration::from_secs(30);
        OfficialResolver::with_options(options)
    })
}

/// Convert the official list into this application's candidate list.
///
/// PAC endpoints arrive as bare `host:port`, which [`ProxyCandidate`] keeps
/// verbatim; each transport adds the scheme it needs.
fn convert(kinds: Vec<ProxyKind>) -> Vec<ProxyCandidate> {
    kinds
        .into_iter()
        .map(|kind| match kind {
            ProxyKind::Direct => ProxyCandidate::Direct,
            ProxyKind::Http(endpoint) => ProxyCandidate::Http(endpoint),
            ProxyKind::Socks(endpoint) => ProxyCandidate::Socks(endpoint),
        })
        .collect()
}

/// Which layer of the resolver supplied the winning candidate.
///
/// The resolver applies its documented precedence — environment variables, then
/// the OS configuration, then `DIRECT` — so reporting the source lets the
/// settings page distinguish an inherited variable from the OS policy.
fn source_for(config: &ProxyConfig) -> ProxySource {
    if config.environment.http_proxy.is_some()
        || config.environment.https_proxy.is_some()
        || config.environment.all_proxy.is_some()
    {
        return ProxySource::Environment;
    }
    match config.pac.as_ref().map(|pac| pac.source) {
        Some(PacScriptSource::WpadDns) | Some(PacScriptSource::WpadDhcp) => ProxySource::Wpad,
        Some(PacScriptSource::Configured) => ProxySource::Pac,
        None if config.static_rules.is_some() => ProxySource::SystemStatic,
        _ => ProxySource::Environment,
    }
}

/// The name this resolver reports for a PAC source state.
fn pac_state_name(state: PacSourceState) -> String {
    // `PacSourceState` is `#[non_exhaustive]`, so the match keeps an arm for
    // unknown variants rather than assuming the set is closed.
    match state {
        PacSourceState::Disabled => "disabled",
        PacSourceState::Unsupported => "unsupported",
        PacSourceState::Unconfigured => "unconfigured",
        PacSourceState::NotFound => "not-found",
        PacSourceState::Available => "available",
        PacSourceState::ErrorDiscovery => "error-discovery",
        PacSourceState::ErrorDownload => "error-download",
        other => format!("{other:?}").to_ascii_lowercase(),
    }
}

/// The name this resolver reports for a static rule candidate.
fn static_candidate_name(candidate: &ProxyKind) -> String {
    match candidate {
        ProxyKind::Direct => "DIRECT".to_owned(),
        ProxyKind::Socks(endpoint) => format!("SOCKS {}", redact_url_credentials(endpoint)),
        ProxyKind::Http(endpoint) => format!("PROXY {}", redact_url_credentials(endpoint)),
    }
}

/// The Windows `System` resolver.
pub(crate) struct SystemProxyResolver;

impl ProxyResolver for SystemProxyResolver {
    fn resolve(&self, url: &str) -> ProxyDecision {
        self.resolve_ordered(url).to_decision()
    }

    fn resolve_ordered(&self, url: &str) -> ProxyResolution {
        let Ok(parsed) = url::Url::parse(url) else {
            // An unparseable destination cannot be resolved per URL. Reporting
            // this as unresolved keeps a malformed URL from going direct.
            return ProxyResolution::unresolved(ProxySource::SystemStatic);
        };
        match shared().resolve_proxy(&parsed) {
            // The resolver's documented behavior is to fall through to `DIRECT`
            // when PAC/WPAD cannot be resolved. That is the operating system's
            // own policy, so it is honored here, and `describe` reports the PAC
            // state separately so the settings page can still show that the
            // configured script was never evaluated.
            Ok(kinds) => {
                let candidates = convert(kinds);
                if candidates.is_empty() {
                    ProxyResolution::unresolved(source_for(&shared().read_proxy_config()))
                } else {
                    ProxyResolution {
                        candidates,
                        source: source_for(&shared().read_proxy_config()),
                    }
                }
            }
            // A platform error means the resolver could not answer at all. That
            // must not become a direct route.
            Err(_) => ProxyResolution::unresolved(ProxySource::SystemStatic),
        }
    }

    fn describe(&self) -> SystemProxyDescription {
        let config = shared().read_proxy_config();
        let static_proxies = config
            .static_rules
            .iter()
            .flat_map(|rules| {
                [
                    rules.http.as_ref(),
                    rules.https.as_ref(),
                    rules.socks.as_ref(),
                ]
                .into_iter()
                .flatten()
            })
            .map(static_candidate_name)
            .collect();
        SystemProxyDescription {
            backend: "windows-os".to_owned(),
            environment_proxy: config.environment.http_proxy.is_some()
                || config.environment.https_proxy.is_some()
                || config.environment.all_proxy.is_some(),
            bypass: config
                .environment
                .no_proxy
                .as_ref()
                .map(|status| ProxyBypass::parse(&status.value).entries().to_vec())
                .unwrap_or_default(),
            auto_detect: config.auto_detect,
            pac_url: config
                .pac
                .as_ref()
                .map(|pac| redact_url_credentials(&pac.url)),
            pac_state: Some(pac_state_name(config.configured_pac.state)),
            static_proxies,
            system_bypass: match config.platform.as_ref() {
                Some(PlatformProxyConfig::Windows(windows)) => windows
                    .proxy_bypass
                    .as_ref()
                    .map(|bypass| redact_url_credentials(bypass)),
                _ => None,
            },
            generation: shared().config_generation(),
        }
    }

    fn generation(&self) -> u64 {
        shared().config_generation()
    }
}
