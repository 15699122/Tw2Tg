use super::*;

#[test]
fn a_fresh_config_defers_the_level_to_the_build_channel() {
    let config = AppConfig::default();
    // No stored level: the channel decides, so a pre-release starts at
    // debug and a stable release starts at info.
    assert_eq!(config.logging.level, None);
    assert_eq!(
        config.logging.effective_level(),
        crate::build_channel::channel_default_log_level()
    );
    assert_eq!(config.logging.max_files, 5);
}

#[test]
fn an_explicit_user_level_outranks_the_channel_default() {
    let mut config = AppConfig::default();
    config.logging.level = Some(LogLevel::Error);
    assert_eq!(config.logging.effective_level(), LogLevel::Error);
}

#[test]
fn a_stored_level_survives_a_save_and_load_cycle() {
    // An existing installation keeps its level across an upgrade instead of
    // being silently rewritten to the new channel default.
    let paths = PortablePaths::from_root("/tmp/xarchive-logging");
    let _ = fs::remove_file(&paths.config_file);
    let mut config = AppConfig::default();
    config.logging.level = Some(LogLevel::Warning);
    config.save(&paths).expect("save config");

    let (loaded, error) = AppConfig::load(&paths);
    assert_eq!(error, None);
    assert_eq!(loaded.logging.level, Some(LogLevel::Warning));
    assert_eq!(loaded.logging.effective_level(), LogLevel::Warning);
    let _ = fs::remove_file(&paths.config_file);
}

#[test]
fn a_config_without_a_level_follows_the_channel() {
    let paths = PortablePaths::from_root("/tmp/xarchive-logging-absent");
    let _ = fs::remove_file(&paths.config_file);
    let (loaded, error) = AppConfig::load(&paths);
    assert_eq!(error, None);
    assert_eq!(loaded.logging.level, None);
    assert_eq!(
        loaded.logging.effective_level(),
        crate::build_channel::channel_default_log_level()
    );
}

#[test]
fn validates_log_file_range() {
    let mut config = AppConfig::default();
    config.logging.max_files = 0;
    assert!(config.validate().is_err());
    config.logging.max_files = 100;
    assert!(config.validate().is_ok());
}
