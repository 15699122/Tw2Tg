use super::*;

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
fn the_environment_resolver_reports_a_route_or_direct() {
    let decision = EnvironmentProxyResolver.resolve("https://example.invalid/");
    assert!(
        decision.is_direct() || decision.proxy().is_some(),
        "the environment resolver must produce a usable decision"
    );
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
