use super::*;

#[test]
fn network_defaults_are_bounded_and_follow_the_system_proxy() {
    let network = NetworkConfig::default();
    assert_eq!(network.proxy_mode, ProxyMode::System);
    assert!(network.proxy.is_none());
    assert!(network.normalized_proxy().is_none());
    assert!(network.active_manual_proxy().is_none());
    // The default inherits, so a child must be left alone and the Sidecar
    // is told the mode instead of being handed a value.
    assert_eq!(
        network.sidecar_env(),
        vec![("XARCHIVE_PROXY_MODE".to_owned(), "system".to_owned())]
    );
    assert!(network.sidecar_env_remove().is_empty());
    assert_eq!(network.extraction_timeout_seconds, 300);
    assert_eq!(network.discovery_timeout_seconds, 600);
    assert_eq!(network.aria2_connect_timeout_seconds, 30);
    assert_eq!(network.aria2_idle_timeout_seconds, 60);
    assert_eq!(network.aria2_max_tries, 3);
    assert_eq!(network.transfer_timeout_seconds, 1800);
    assert_eq!(network.telegram_timeout_seconds, 30);
    assert!(network.validate().is_ok());
}

#[test]
fn network_proxy_credentials_never_appear_in_diagnostics() {
    let network = NetworkConfig {
        proxy_mode: ProxyMode::Manual,
        proxy: Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        ..NetworkConfig::default()
    };
    assert!(network.validate().is_ok());
    assert_eq!(
        network.redacted_proxy().as_deref(),
        Some("http://[REDACTED]@proxy.example:8080")
    );
    let diagnostics = serde_json::to_string(&network.diagnostics()).expect("diagnostics");
    assert!(!diagnostics.contains("s3cret"));
    assert!(diagnostics.contains("proxy.example"));
    assert!(diagnostics.contains(r#""proxy_mode":"manual""#));
    assert_eq!(
        network.secrets(),
        vec!["http://alice:s3cret@proxy.example:8080"]
    );
    assert_eq!(
        network
            .sidecar_env()
            .into_iter()
            .filter(|(key, _)| key == "XARCHIVE_PROXY")
            .collect::<Vec<_>>(),
        vec![(
            "XARCHIVE_PROXY".to_owned(),
            "http://alice:s3cret@proxy.example:8080".to_owned()
        )]
    );
    // The proxy is never part of the process command line.
    assert!(
        !network
            .sidecar_args()
            .iter()
            .any(|argument| argument.contains("s3cret"))
    );
    assert_eq!(
        network.sidecar_args(),
        vec![
            "--timeout-seconds",
            "300",
            "--discovery-timeout-seconds",
            "600"
        ]
    );
}

#[test]
fn rejects_malformed_proxy_and_out_of_range_network_values() {
    let mut network = NetworkConfig {
        proxy_mode: ProxyMode::Manual,
        ..NetworkConfig::default()
    };
    for proxy in [
        "http://alice:s3cret@proxy example:8080",
        "file:///etc/passwd",
        "http://",
        "http://@",
    ] {
        network.proxy = Some(proxy.to_owned());
        assert!(
            network.validate().is_err(),
            "proxy must be rejected: {proxy}"
        );
    }
    network.proxy = Some("socks5://127.0.0.1:1080".to_owned());
    assert!(network.validate().is_ok());

    network.extraction_timeout_seconds = 0;
    assert!(network.validate().is_err());
    network.extraction_timeout_seconds = default_extraction_timeout_seconds();
    network.aria2_max_tries = 0;
    assert!(network.validate().is_err());
    network.aria2_max_tries = MAX_ARIA2_MAX_TRIES + 1;
    assert!(network.validate().is_err());
}

#[test]
fn app_config_validation_checks_the_network_section() {
    let mut config = AppConfig::default();
    config.network.proxy_mode = ProxyMode::Manual;
    config.network.proxy = Some("ftp://proxy.example:21".to_owned());
    assert!(config.validate().is_ok());
    config.network.transfer_timeout_seconds = 0;
    assert!(config.validate().is_err());
}

#[test]
fn direct_removes_the_inherited_proxy_environment_of_child_processes() {
    let network = NetworkConfig {
        proxy_mode: ProxyMode::Direct,
        proxy: Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        ..NetworkConfig::default()
    };
    assert!(network.validate().is_ok());
    assert_eq!(
        network.sidecar_env(),
        vec![("XARCHIVE_PROXY_MODE".to_owned(), "direct".to_owned())]
    );
    let removed = network.sidecar_env_remove();
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
fn only_the_manual_mode_hands_its_value_to_a_client_or_child() {
    let manual = NetworkConfig {
        proxy_mode: ProxyMode::Manual,
        proxy: Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        ..NetworkConfig::default()
    };
    assert!(manual.active_manual_proxy().is_some());

    for mode in [ProxyMode::System, ProxyMode::Direct] {
        let inactive = NetworkConfig {
            proxy_mode: mode,
            proxy: manual.proxy.clone(),
            ..NetworkConfig::default()
        };
        assert!(
            inactive.active_manual_proxy().is_none(),
            "{mode:?} must not use a stored value"
        );
        assert!(
            inactive.redacted_proxy().is_none(),
            "{mode:?} must not report a proxy as active"
        );
        assert!(
            !inactive
                .sidecar_env()
                .iter()
                .any(|(key, _)| key == "XARCHIVE_PROXY"),
            "{mode:?} must not pass a stored value to the Sidecar"
        );
        let diagnostics = inactive.diagnostics();
        assert!(
            diagnostics.proxy_configured,
            "{mode:?} must still report that a value is stored"
        );
        assert!(
            !diagnostics.proxy_active,
            "{mode:?} must report that the stored value is not in effect"
        );
    }
}

#[test]
fn manual_requires_a_value_and_blank_values_do_not_count() {
    let mut network = NetworkConfig {
        proxy_mode: ProxyMode::Manual,
        ..NetworkConfig::default()
    };
    assert!(network.validate().is_err());
    network.proxy = Some("   ".to_owned());
    assert!(network.validate().is_err());
    network.proxy = Some("http://proxy.example:8080".to_owned());
    assert!(network.validate().is_ok());
}

#[test]
fn the_mode_survives_a_save_and_load_round_trip() {
    let paths = crate::portable::PortablePaths::from_root(
        std::env::temp_dir().join(format!("xarchive-config-{}", std::process::id())),
    );
    let _ = std::fs::remove_dir_all(&paths.config_dir);
    for mode in [ProxyMode::System, ProxyMode::Direct, ProxyMode::Manual] {
        let mut config = AppConfig::default();
        config.network.proxy_mode = mode;
        config.network.proxy = Some("http://alice:s3cret@proxy.example:8080".to_owned());
        config.save(&paths).expect("save");

        let (loaded, error) = AppConfig::load(&paths);
        assert_eq!(error, None, "{mode:?} must load without an error");
        assert_eq!(
            loaded.network.proxy_mode, mode,
            "{mode:?} must survive a restart"
        );
        assert_eq!(
            loaded.network.normalized_proxy().as_deref(),
            Some("http://alice:s3cret@proxy.example:8080")
        );
    }
    let _ = std::fs::remove_dir_all(&paths.config_dir);
}

#[test]
fn a_configuration_without_a_proxy_loads_as_system() {
    let network = load_document("network:\n  extraction_timeout_seconds: 120\n").network;
    assert_eq!(network.proxy_mode, ProxyMode::System);
    assert!(network.active_manual_proxy().is_none());
    assert_eq!(network.extraction_timeout_seconds, 120);
    assert!(network.validate().is_ok());
}

#[test]
fn an_explicit_system_mode_is_not_migrated_away_from_a_stored_value() {
    let network = load_document(
        "network:\n  proxy_mode: system\n  proxy: http://alice:s3cret@proxy.example:8080\n",
    )
    .network;
    assert_eq!(
        network.proxy_mode,
        ProxyMode::System,
        "a deliberate system choice must survive the migration"
    );
    assert!(network.active_manual_proxy().is_none());
}

#[test]
fn an_explicit_mode_is_never_overwritten_by_the_migration() {
    for mode in [ProxyMode::System, ProxyMode::Direct, ProxyMode::Manual] {
        let mut network = NetworkConfig {
            proxy_mode: mode,
            proxy_mode_declared: true,
            proxy: Some("http://proxy.example:8080".to_owned()),
            ..NetworkConfig::default()
        };
        network.apply_legacy_migration();
        assert_eq!(
            network.proxy_mode, mode,
            "the migration must not change an explicit mode"
        );
    }
}

#[test]
fn a_configuration_without_a_mode_loads_as_manual_and_keeps_its_value() {
    // This is the exact shape written before the mode existed.
    let network =
        load_document("network:\n  proxy: http://alice:s3cret@proxy.example:8080\n").network;
    assert_eq!(network.proxy_mode, ProxyMode::Manual);
    assert_eq!(
        network.active_manual_proxy().as_deref(),
        Some("http://alice:s3cret@proxy.example:8080")
    );
    assert!(network.validate().is_ok());
    assert_eq!(
        network.redacted_proxy().as_deref(),
        Some("http://[REDACTED]@proxy.example:8080")
    );
}
