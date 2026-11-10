use super::*;

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
