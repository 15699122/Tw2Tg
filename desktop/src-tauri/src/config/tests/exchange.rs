use super::*;

#[test]
fn configuration_exchange_is_strict_and_never_exports_secret_categories() {
    let config = AppConfig::default();
    let raw = ConfigurationExchange::export(&config).unwrap();
    let document = ConfigurationExchange::parse(&raw).unwrap();
    assert_eq!(document.schema_version, 1);
    for forbidden in [
        "telegram",
        "extension",
        "proxy",
        "token",
        "database",
        "cache",
        "initialized",
    ] {
        assert!(!raw.contains(forbidden), "excluded category: {forbidden}");
    }
    assert!(
        ConfigurationExchange::parse(
            r#"{"schema_version":2,"kind":"tw2tg.configuration","settings":{}}"#
        )
        .is_err()
    );
    assert!(
        ConfigurationExchange::parse(
            r#"{"schema_version":1,"kind":"tw2tg.configuration","settings":{"token":"secret"}}"#
        )
        .is_err()
    );
    assert!(
        ConfigurationExchange::parse(
            r#"{"schema_version":1,"schema_version":1,"kind":"tw2tg.configuration","settings":{}}"#
        )
        .is_err()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn configuration_exchange_atomic_persistence_rejects_stale_and_preserves_runtime_on_failure() {
    let root = std::env::temp_dir().join(format!("tw2tg-f2-persist-{}", std::process::id()));
    let paths = PortablePaths::from_root(root.clone());
    fs::create_dir_all(&paths.config_dir).unwrap();
    let locks = paths.config_dir.join("locks");
    fs::create_dir_all(&locks).unwrap();
    let maintenance =
        xarchive_workflow::maintenance::WorkspaceGuard::drain(&locks, || false).unwrap();
    let mut local = AppConfig::default();
    let document = ConfigurationExchange::parse(
        r#"{"schema_version":1,"kind":"tw2tg.configuration","settings":{"log_max_files":7}}"#,
    )
    .unwrap();
    assert!(
        document
            .persist_preview(&mut local, &paths, "wrong", &maintenance)
            .is_err()
    );
    assert!(!paths.config_file.exists());
    document
        .persist_preview(&mut local, &paths, "missing", &maintenance)
        .unwrap();
    assert_eq!(local.logging.max_files, 7);
    let bytes = fs::read(&paths.config_file).unwrap();
    let revision = document
        .preview(&local, &root, Some(&bytes))
        .unwrap()
        .revision;
    fs::write(&paths.config_file, b"changed").unwrap();
    assert!(
        document
            .persist_preview(&mut local, &paths, &revision, &maintenance)
            .is_err()
    );
    assert_eq!(fs::read(&paths.config_file).unwrap(), b"changed");
    fs::remove_file(&paths.config_file).unwrap();
    fs::remove_dir_all(&paths.config_dir).unwrap();
    assert!(
        document
            .persist_preview(&mut local, &paths, "missing", &maintenance)
            .is_err()
    );
    assert_eq!(local.logging.max_files, 7);
    drop(maintenance);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn configuration_exchange_preview_preserves_missing_paths_and_binds_exact_bytes() {
    let local = AppConfig::default();
    let document = ConfigurationExchange::parse(r#"{"schema_version":1,"kind":"tw2tg.configuration","settings":{"archive_directory":"/definitely-missing-tw2tg-f2","use_aria2":true}}"#).unwrap();
    let preview = document
        .preview(&local, std::path::Path::new("/tmp"), Some(b"one\n"))
        .unwrap();
    assert_eq!(
        preview.candidate.download.directory,
        local.download.directory
    );
    assert_eq!(preview.preserved.len(), 1);
    assert!(preview.restart_required);
    assert!(preview.candidate.download.use_aria2);
    let changed = document
        .preview(&local, std::path::Path::new("/tmp"), Some(b"one"))
        .unwrap();
    assert_ne!(preview.revision, changed.revision);
    let invalid = ConfigurationExchange::parse(
        r#"{"schema_version":1,"kind":"tw2tg.configuration","settings":{"log_max_files":0}}"#,
    )
    .unwrap();
    assert!(
        invalid
            .preview(&local, std::path::Path::new("/tmp"), None)
            .is_err()
    );
}
