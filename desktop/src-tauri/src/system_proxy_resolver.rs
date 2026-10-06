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
//!   That fall-through is deliberately *not* honored as a route: an
//!   unevaluable script would otherwise send corporate traffic out unproxied.
//!   The resolver reports configuration state separately, so this adapter uses
//!   that state to tell a failed script from a deliberate `DIRECT` and refuses
//!   only the former.
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
        PacSourceState::Disabled => "disabled".to_owned(),
        PacSourceState::Unsupported => "unsupported".to_owned(),
        PacSourceState::Unconfigured => "unconfigured".to_owned(),
        PacSourceState::NotFound => "not-found".to_owned(),
        PacSourceState::Available => "available".to_owned(),
        PacSourceState::ErrorDiscovery => "error-discovery".to_owned(),
        PacSourceState::ErrorDownload => "error-download".to_owned(),
        other => return format!("{other:?}").to_ascii_lowercase(),
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

/// The environment only overrides the OS for a valid proxy for this scheme.
fn environment_applies(scheme: &str, http: bool, https: bool, all: bool) -> bool {
    match scheme {
        "http" | "ws" => http || all,
        "https" | "wss" => https || all,
        _ => all,
    }
}

enum PolicyRoute<'a> {
    None,
    Evaluate {
        source_url: &'a str,
        source: ProxySource,
    },
    Failed {
        source: ProxySource,
        state: String,
    },
}

fn route_for_config(config: &ProxyConfig) -> PolicyRoute<'_> {
    policy_route(
        config.pac_url.as_deref(),
        config.auto_detect,
        config
            .pac
            .as_ref()
            .map(|pac| (pac.url.as_str(), pac.source)),
        config.configured_pac.state,
        config.wpad_dhcp.state,
        config.wpad_dns.state,
    )
}

/// Configuration presence and loaded scripts are different facts. The official
/// snapshot retains PAC/WPAD errors after the loaded `pac` becomes None.
fn policy_route<'a>(
    configured_url: Option<&'a str>,
    auto_detect: bool,
    loaded: Option<(&'a str, PacScriptSource)>,
    configured_state: PacSourceState,
    dhcp_state: PacSourceState,
    dns_state: PacSourceState,
) -> PolicyRoute<'a> {
    if let Some((source_url, source)) = loaded {
        return PolicyRoute::Evaluate {
            source_url,
            source: match source {
                PacScriptSource::WpadDns | PacScriptSource::WpadDhcp => ProxySource::Wpad,
                _ => ProxySource::Pac,
            },
        };
    }
    if let Some(source_url) = configured_url {
        let state = pac_state_name(configured_state);
        if crate::proxy::pac_fallback_must_fail_closed(true, &state) {
            return PolicyRoute::Failed {
                source: ProxySource::Pac,
                state,
            };
        }
        // Even Available only proves a script was loaded, never evaluated.
        return PolicyRoute::Evaluate {
            source_url,
            source: ProxySource::Pac,
        };
    }
    if auto_detect {
        for status in [dhcp_state, dns_state] {
            let state = pac_state_name(status);
            if crate::proxy::pac_fallback_must_fail_closed(true, &state) {
                return PolicyRoute::Failed {
                    source: ProxySource::Wpad,
                    state,
                };
            }
        }
    }
    PolicyRoute::None
}

fn policy_failure(source: ProxySource, state: &str) -> ProxyResolution {
    ProxyResolution::unresolved_because(
        source,
        format!(
            "the PAC/WPAD policy could not be evaluated ({state}); no fallback route was accepted"
        ),
    )
}

fn map_candidates(
    result: os_proxy_resolver::Result<Vec<ProxyKind>>,
    source: ProxySource,
) -> ProxyResolution {
    match result {
        Ok(kinds) if !kinds.is_empty() => ProxyResolution {
            candidates: convert(kinds),
            source,
            reason: None,
        },
        _ => policy_failure(source, "resolution-error"),
    }
}

/// Use WinHTTP's explicit-source API, which exposes an evaluation failure.
/// resolve_proxy hides that failure by falling through to static/DIRECT.
fn evaluate_policy(
    resolver: &OfficialResolver,
    source_url: &str,
    destination: &url::Url,
    source: ProxySource,
) -> ProxyResolution {
    match resolver.evaluate_pac_source(source_url, destination) {
        Ok(kinds) => map_candidates(Ok(kinds), source),
        // Never propagate the upstream error: it may include PAC credentials.
        Err(_) => policy_failure(source, "evaluation-error"),
    }
}

/// The Windows `System` resolver.
pub(crate) struct SystemProxyResolver;

impl SystemProxyResolver {
    pub(crate) fn new() -> Box<dyn ProxyResolver> {
        Box::new(Self)
    }
}

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
        let config = shared().read_proxy_config();
        let valid = |setting: &Option<os_proxy_resolver::EnvironmentVariableStatus>| {
            setting.as_ref().is_some_and(|value| value.error.is_none())
        };
        if environment_applies(
            parsed.scheme(),
            valid(&config.environment.http_proxy),
            valid(&config.environment.https_proxy),
            valid(&config.environment.all_proxy),
        ) {
            // Let the official resolver apply its own NO_PROXY matching. An
            // environment DIRECT answer is not a failed OS policy fallback.
            return map_candidates(shared().resolve_proxy(&parsed), ProxySource::Environment);
        }
        match route_for_config(&config) {
            PolicyRoute::Evaluate { source_url, source } => {
                evaluate_policy(shared(), source_url, &parsed, source)
            }
            PolicyRoute::Failed { source, state } => policy_failure(source, &state),
            PolicyRoute::None => {
                map_candidates(shared().resolve_proxy(&parsed), source_for(&config))
            }
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
                .map(|pac| pac.url.as_str())
                .or(config.pac_url.as_deref())
                .map(redact_url_credentials),
            pac_state: Some(match route_for_config(&config) {
                PolicyRoute::Failed { state, .. } => state,
                PolicyRoute::Evaluate { .. } => {
                    pac_state_name(match config.pac.as_ref().map(|pac| pac.source) {
                        Some(PacScriptSource::WpadDhcp) => config.wpad_dhcp.state,
                        Some(PacScriptSource::WpadDns) => config.wpad_dns.state,
                        _ => config.configured_pac.state,
                    })
                }
                PolicyRoute::None => pac_state_name(config.configured_pac.state),
            }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::Duration,
    };

    #[test]
    fn unloaded_configured_pac_failure_is_not_direct() {
        let route = policy_route(
            Some("http://fixture/policy.pac"),
            false,
            None,
            PacSourceState::ErrorDownload,
            PacSourceState::Disabled,
            PacSourceState::Disabled,
        );
        let PolicyRoute::Failed { source, state } = route else {
            panic!("failed policy must not fall through")
        };
        assert_eq!(source, ProxySource::Pac);
        assert_eq!(state, "error-download");
        assert!(policy_failure(source, &state).candidates.is_empty());
    }

    #[test]
    fn wpad_failure_and_loaded_source_precedence() {
        for state in [
            PacSourceState::ErrorDiscovery,
            PacSourceState::ErrorDownload,
            PacSourceState::NotFound,
        ] {
            assert!(matches!(
                policy_route(
                    None,
                    true,
                    None,
                    PacSourceState::Unconfigured,
                    PacSourceState::Unsupported,
                    state
                ),
                PolicyRoute::Failed {
                    source: ProxySource::Wpad,
                    ..
                }
            ));
        }
        assert!(matches!(
            policy_route(
                Some("http://configured/pac"),
                true,
                Some(("http://wpad/pac", PacScriptSource::WpadDns)),
                PacSourceState::ErrorDownload,
                PacSourceState::Unsupported,
                PacSourceState::Available
            ),
            PolicyRoute::Evaluate {
                source: ProxySource::Wpad,
                source_url: "http://wpad/pac"
            }
        ));
        assert!(matches!(
            policy_route(
                None,
                false,
                None,
                PacSourceState::Unconfigured,
                PacSourceState::Disabled,
                PacSourceState::Disabled
            ),
            PolicyRoute::None
        ));
    }

    #[test]
    fn environment_override_matches_destination_scheme() {
        assert!(environment_applies("http", true, false, false));
        assert!(!environment_applies("https", true, false, false));
        assert!(environment_applies("wss", false, true, false));
        assert!(environment_applies("https", false, false, true));
    }

    struct PacServer {
        url: String,
        stop: Arc<AtomicBool>,
        worker: Option<thread::JoinHandle<()>>,
    }
    impl PacServer {
        fn new(body: &'static str, status: u16) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/policy.pac", listener.local_addr().unwrap());
            listener.set_nonblocking(true).unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let shutdown = stop.clone();
            let worker = thread::spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(1)))
                                .unwrap();
                            let mut request = [0u8; 4096];
                            let _ = stream.read(&mut request);
                            let response = format!(
                                "HTTP/1.1 {status} fixture\r\nContent-Type: application/x-ns-proxy-autoconfig\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            );
                            let _ = stream.write_all(response.as_bytes());
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5))
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                url,
                stop,
                worker: Some(worker),
            }
        }
    }
    impl Drop for PacServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            self.worker.take().unwrap().join().unwrap();
        }
    }

    #[test]
    fn native_winhttp_distinguishes_direct_from_pac_failures() {
        let mut options = ResolverOptions::default();
        options.pac_timeout = Duration::from_secs(2);
        options.pac_fetch_timeout = Duration::from_secs(2);
        let resolver = OfficialResolver::with_options(options);
        let destination = url::Url::parse("https://fixture.invalid/resource").unwrap();
        let direct = PacServer::new(
            "function FindProxyForURL(url, host) { return 'DIRECT'; }",
            200,
        );
        let result = evaluate_policy(&resolver, &direct.url, &destination, ProxySource::Pac);
        assert_eq!(result.candidates, vec![ProxyCandidate::Direct]);
        assert!(result.reason.is_none());
        let invalid = PacServer::new(
            "function FindProxyForURL(url, host) { throw new Error('fixture'); }",
            200,
        );
        let missing = PacServer::new("missing", 404);
        for server in [&invalid, &missing] {
            let result = evaluate_policy(&resolver, &server.url, &destination, ProxySource::Pac);
            assert!(
                result.candidates.is_empty(),
                "PAC failure must not become DIRECT"
            );
            assert_eq!(result.source, ProxySource::Pac);
            assert!(result.reason.unwrap().contains("evaluation-error"));
        }
    }
    struct ExplicitPacResolver {
        resolver: OfficialResolver,
        source_url: String,
    }
    impl ProxyResolver for ExplicitPacResolver {
        fn resolve(&self, url: &str) -> ProxyDecision {
            self.resolve_ordered(url).to_decision()
        }
        fn resolve_ordered(&self, url: &str) -> ProxyResolution {
            evaluate_policy(
                &self.resolver,
                &self.source_url,
                &url::Url::parse(url).unwrap(),
                ProxySource::Pac,
            )
        }
    }

    #[test]
    fn native_pac_failure_blocks_http_client_and_child_start() {
        use crate::proxy::ProxyHttpClient;
        use xarchive_core::ProxyMode;
        let target = "https://fixture.invalid/payload".to_owned();
        let invalid = PacServer::new(
            "function FindProxyForURL(url, host) { throw new Error('fixture'); }",
            200,
        );
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(2),
            Box::new(ExplicitPacResolver {
                resolver: OfficialResolver::with_options(ResolverOptions::default()),
                source_url: invalid.url.clone(),
            }),
        );
        assert!(client.candidates_for(&target).is_err());
        assert!(
            client
                .require_child_routable(&target, "fixture-child")
                .is_err()
        );
        // The positive control performs real loopback HTTP through the same client boundary.
        let payload = PacServer::new("fixture payload", 200);
        let direct = PacServer::new(
            "function FindProxyForURL(url, host) { return 'DIRECT'; }",
            200,
        );
        let client = ProxyHttpClient::new(
            ProxyMode::System,
            None,
            Duration::from_secs(2),
            Box::new(ExplicitPacResolver {
                resolver: OfficialResolver::with_options(ResolverOptions::default()),
                source_url: direct.url.clone(),
            }),
        );
        let candidates = client.candidates_for(&payload.url).unwrap();
        assert_eq!(candidates[0].1, ProxyCandidate::Direct);
        assert_eq!(
            candidates[0]
                .0
                .get(&payload.url)
                .send()
                .unwrap()
                .text()
                .unwrap(),
            "fixture payload"
        );
    }
}
