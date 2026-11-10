use super::*;

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
