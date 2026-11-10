use super::*;

#[test]
fn defaults_apply_a_bounded_network_budget() {
    let config = config();
    assert_eq!(config.connect_timeout, Duration::from_secs(30));
    assert_eq!(config.idle_timeout, Duration::from_secs(60));
    assert_eq!(config.max_tries, 3);
    assert!(config.proxy.is_none());
    assert_eq!(config.proxy_mode, ProxyMode::System);
    // The default follows the system proxy, so aria2 keeps whatever it
    // would have discovered on its own.
    assert!(config.environment().is_empty());
    assert!(config.environment_remove().is_empty());
}

#[test]
fn unified_network_settings_reach_the_command_line_without_the_proxy() {
    let config = config()
        .with_network(Duration::from_secs(12), Duration::from_secs(45), 5)
        .with_proxy_mode(ProxyMode::Manual)
        .with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
    let args = config.command_args();
    assert!(args.contains(&"--connect-timeout=12".to_owned()));
    assert!(args.contains(&"--timeout=45".to_owned()));
    assert!(args.contains(&"--max-tries=5".to_owned()));
    assert!(
        !args
            .iter()
            .any(|argument| argument.contains("s3cret") || argument.contains("proxy.example")),
        "proxy credentials must not be passed on the command line"
    );
}

#[test]
fn proxy_reaches_aria2_through_the_environment_and_never_through_debug() {
    let config = config()
        .with_proxy_mode(ProxyMode::Manual)
        .with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
    let environment = config.environment();
    assert!(!environment.is_empty());
    for (_, value) in &environment {
        assert_eq!(value, "http://alice:s3cret@proxy.example:8080");
    }
    // aria2 reads the lowercase trio; the uppercase spellings and the
    // application-specific variable are added for the other children that
    // share this environment.
    for key in ["all_proxy", "http_proxy", "https_proxy"] {
        assert!(
            environment.iter().any(|(name, _)| name == key),
            "aria2 requires {key}, got {environment:?}"
        );
    }
    assert!(config.environment_remove().is_empty());
    let debug = format!("{config:?}");
    assert!(!debug.contains("s3cret"));
    assert!(!debug.contains("proxy.example"));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn direct_strips_the_inherited_proxy_environment() {
    let config = config()
        .with_proxy_mode(ProxyMode::Direct)
        .with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
    assert!(
        config.environment().is_empty(),
        "Direct must not hand a stored proxy to aria2"
    );
    let removed = config.environment_remove();
    for key in [
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
    ] {
        assert!(
            removed.iter().any(|name| name == key),
            "Direct must remove {key}, got {removed:?}"
        );
    }
}

#[test]
fn system_leaves_the_inherited_environment_for_aria2() {
    let config = config().with_proxy(Some("http://alice:s3cret@proxy.example:8080"));
    assert_eq!(config.proxy_mode, ProxyMode::System);
    assert!(
        config.environment().is_empty(),
        "System must not pin the stored manual value on aria2"
    );
    assert!(
        config.environment_remove().is_empty(),
        "System must not strip what aria2 would use on its own"
    );
}

#[test]
fn blank_or_whitespace_proxies_are_ignored_and_malformed_ones_rejected() {
    assert!(
        config()
            .with_proxy_mode(ProxyMode::Manual)
            .with_proxy(Some("   "))
            .proxy
            .is_none()
    );
    let malformed = config()
        .with_proxy_mode(ProxyMode::Manual)
        .with_proxy(Some("http://alice:s3cret@proxy example:8080"));
    assert!(matches!(
        malformed.validate(),
        Err(DownloadError::InvalidSupervisorConfiguration)
    ));
}
