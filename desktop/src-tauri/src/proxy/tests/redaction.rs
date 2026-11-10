use super::*;

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
