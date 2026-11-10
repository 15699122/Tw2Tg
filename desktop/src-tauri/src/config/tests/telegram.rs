use super::*;

#[test]
fn download_config_defaults_to_gallery_dl_direct_download() {
    // A config written before this flag existed must load without it, and
    // must not silently reintroduce the aria2 dependency.
    let config: DownloadConfig = serde_json::from_str("{}").unwrap();
    assert!(!config.use_aria2);
    assert!(!DownloadConfig::default().use_aria2);

    let enabled: DownloadConfig = serde_json::from_str(r#"{"use_aria2": true}"#).unwrap();
    assert!(enabled.use_aria2);
}

#[test]
fn telegram_rotation_defaults_do_not_enable_sending() {
    let config: TelegramConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(
        config.credential_rotation_resume_policy,
        CredentialRotationResumePolicy::Automatic
    );
    assert!(!config.enabled);
    assert!(!config.auto_send_on_archive);
    assert_eq!(
        config.settings(false).credential_rotation_resume_policy,
        CredentialRotationResumePolicy::Automatic
    );
}

#[test]
fn telegram_rotation_confirmation_survives_serialization() {
    let config = TelegramConfig {
        credential_rotation_resume_policy: CredentialRotationResumePolicy::Confirm,
        ..TelegramConfig::default()
    };
    let loaded: TelegramConfig =
        serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
    assert_eq!(
        loaded.credential_rotation_resume_policy,
        CredentialRotationResumePolicy::Confirm
    );
    assert_eq!(
        loaded.settings(false).credential_rotation_resume_policy,
        CredentialRotationResumePolicy::Confirm
    );
    assert!(
        serde_json::from_str::<TelegramConfig>(
            r#"{"credential_rotation_resume_policy":"invalid"}"#
        )
        .is_err()
    );
}

#[test]
fn token_replacement_verifies_before_mutating_secret_store() {
    use xarchive_telegram::{MemorySecretStore, SecretStore};

    let config = TelegramConfig::default();
    let mut store = MemorySecretStore::default();
    store
        .set(TELEGRAM_BOT_TOKEN_KEY, "old-token")
        .expect("seed existing token");

    let error = config
        .replace_bot_token(&mut store, " new-token ", |_| {
            Err("token verification failed".to_owned())
        })
        .expect_err("invalid candidate must be rejected");
    assert_eq!(error, "token verification failed");
    assert_eq!(
        store.get(TELEGRAM_BOT_TOKEN_KEY).expect("read old token"),
        Some("old-token".to_owned())
    );

    let identity = config
        .replace_bot_token(&mut store, " new-token ", |_| Ok("bot-identity".to_owned()))
        .expect("verified token replacement");
    assert_eq!(identity, "bot-identity");
    assert_eq!(
        store.get(TELEGRAM_BOT_TOKEN_KEY).expect("read new token"),
        Some("new-token".to_owned())
    );
}

#[test]
fn token_delete_removes_only_the_secret_store_value() {
    use xarchive_telegram::{MemorySecretStore, SecretStore};

    let config = TelegramConfig::default();
    let mut store = MemorySecretStore::default();
    store
        .set(TELEGRAM_BOT_TOKEN_KEY, "token")
        .expect("seed token");

    config
        .delete_bot_token(&mut store)
        .expect("delete stored token");
    assert_eq!(store.get(TELEGRAM_BOT_TOKEN_KEY).expect("read token"), None);
    assert!(!config.settings(false).bot_token_present);
}

#[test]
fn a_pending_telegram_migration_survives_a_configuration_round_trip() {
    let mut telegram = TelegramConfig {
        enabled: true,
        auto_send_on_archive: true,
        chat_id: "-100777".to_owned(),
        ..TelegramConfig::default()
    };
    assert!(!telegram.migration_pending);
    telegram.migration_pending = true;
    let yaml = serde_yaml::to_string(&telegram).expect("serialize");
    let restored: TelegramConfig = serde_yaml::from_str(&yaml).expect("deserialize");
    assert!(restored.migration_pending);
    assert!(
        restored.enabled,
        "the stored intent is preserved for review"
    );
    // A configuration file without the field must not resume sending.
    let legacy: TelegramConfig =
        serde_yaml::from_str("enabled: true\nauto_send_on_archive: true\n").expect("legacy");
    assert!(!legacy.migration_pending);
}

#[test]
fn the_settings_projection_reports_the_pause_without_exposing_a_token() {
    let mut telegram = TelegramConfig {
        enabled: true,
        chat_id: "-100777".to_owned(),
        ..TelegramConfig::default()
    };
    telegram.migration_pending = true;
    let view = serde_json::to_string(&telegram.settings(true)).expect("projection");
    assert!(view.contains("\"migration_pending\":true"));
    assert!(view.contains("\"bot_token_present\":true"), "{view}");
    // Presence is a boolean fact; no secret material may appear.
    assert!(!view.contains("1234:TEST"), "{view}");
    assert!(!view.contains("\"bot_token\":"), "{view}");
}

#[test]
fn a_document_without_telegram_keys_loads_disabled() {
    // A configuration written before Telegram settings existed must keep
    // loading, and it must load as disabled (plan TG-01).
    let config: AppConfig =
        serde_yaml::from_str("schema_version: 1\nlogging:\n  max_files: 5\n").expect("legacy");
    assert!(!config.telegram.enabled);
    assert!(!config.telegram.auto_send_on_archive);
    assert_eq!(config.telegram.api_base, "https://api.telegram.org");
    assert!(config.telegram.capability.is_none());
    assert!(config.telegram.validate().is_ok());
    let mut config = config;
    assert!(config.validate().is_ok());
}

#[test]
fn an_enabled_telegram_section_must_be_complete_and_secure() {
    let mut config = AppConfig::default();
    // Enabled without a target is rejected.
    config.telegram.enabled = true;
    assert!(config.validate().is_err());
    config.telegram.chat_id = "-1001234567890".to_owned();
    assert!(config.validate().is_ok());

    // Cloud stays HTTPS-only; the local mode accepts explicit loopback HTTP.
    config.telegram.api_base = "http://api.telegram.org".to_owned();
    assert!(config.validate().is_err());
    config.telegram.endpoint_mode = EndpointMode::Local;
    config.telegram.api_base = "http://127.0.0.1:8081".to_owned();
    assert!(config.validate().is_ok());
    config.telegram.api_base = "http://93.184.216.34:8080".to_owned();
    assert!(config.validate().is_err(), "cleartext remote is refused");
    config.telegram.api_base = "http://127.0.0.1".to_owned();
    assert!(config.validate().is_err(), "a port is required");

    // Timeout bounds are shared with the rest of the network settings.
    config.telegram.api_base = "http://127.0.0.1:8081".to_owned();
    config.telegram.connect_timeout_seconds = 0;
    assert!(config.validate().is_err());
    config.telegram.connect_timeout_seconds = 10;
    config.telegram.upload_processing_timeout_seconds = MAX_NETWORK_TIMEOUT_SECONDS + 1;
    assert!(config.validate().is_err());
}

#[test]
fn a_settings_change_advances_the_revision_and_drops_the_capability_record() {
    let mut telegram = TelegramConfig {
        enabled: true,
        chat_id: "-1001234567890".to_owned(),
        capability: Some(VerifiedTelegramCapability {
            endpoint_mode: EndpointMode::Cloud,
            api_base: "https://api.telegram.org".to_owned(),
            verified_at: "2026-10-01T00:00:00Z".to_owned(),
            server_version: Some("8.0".to_owned()),
            max_upload_bytes: Some(50 * 1024 * 1024),
        }),
        revision: 4,
        ..TelegramConfig::default()
    };
    assert!(telegram.capability_matches());

    // A trailing slash is the same endpoint, not a change.
    telegram.api_base = "https://api.telegram.org/".to_owned();
    assert!(telegram.capability_matches());
    // Switching to the local server invalidates the cloud record.
    telegram.endpoint_mode = EndpointMode::Local;
    assert!(!telegram.capability_matches());
    assert!(!telegram.settings(false).capability_verified);

    telegram.record_settings_change();
    assert_eq!(telegram.revision, 5, "queued items bind to a new revision");
    assert!(telegram.capability.is_none());
}

#[test]
fn telegram_configuration_never_carries_the_bot_token() {
    use xarchive_telegram::MemorySecretStore;
    let telegram = TelegramConfig {
        enabled: true,
        chat_id: "-1001234567890".to_owned(),
        ..TelegramConfig::default()
    };
    let mut store = MemorySecretStore::default();
    assert!(!telegram.bot_token_present(&mut store).expect("absent"));
    store
        .set(TELEGRAM_BOT_TOKEN_KEY, "123:secret")
        .expect("store");
    assert!(telegram.bot_token_present(&mut store).expect("present"));

    // Neither the persisted document nor the frontend projection carries
    // the secret itself.
    let document = serde_yaml::to_string(&telegram).expect("serialize");
    assert!(!document.contains("123:secret"));
    assert!(!document.to_lowercase().contains("token"));
    let projection = telegram.settings(true);
    let payload = serde_json::to_string(&projection).expect("settings json");
    assert!(!payload.contains("123:secret"));
    assert!(payload.contains("\"bot_token_present\":true"));
}
