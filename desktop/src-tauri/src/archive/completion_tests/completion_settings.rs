use super::*;

#[test]
fn completion_samples_latest_persisted_settings_and_fails_closed() {
    let root = std::env::temp_dir().join(format!(
        "tg-completion-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let paths = crate::portable::PortablePaths::from_root(&root);
    let mut config = crate::config::AppConfig::default();
    config.telegram.enabled = true;
    config.telegram.auto_send_on_archive = true;
    config.telegram.chat_id = "old-target".into();
    let stale = config.telegram.clone();
    config.telegram.chat_id = "new-target".into();
    config.telegram.revision = 19;
    config.save(&paths).unwrap();
    let sampled = sample_completion_config(Some(&paths.config_file), &stale);
    assert_eq!(sampled.chat_id, "new-target");
    assert_eq!(sampled.revision, 19);
    config.telegram.enabled = false;
    config.save(&paths).unwrap();
    assert!(!sample_completion_config(Some(&paths.config_file), &stale).enabled);
    std::fs::write(&paths.config_file, "broken: [").unwrap();
    assert!(!sample_completion_config(Some(&paths.config_file), &stale).enabled);
    std::fs::remove_file(&paths.config_file).unwrap();
    assert!(!sample_completion_config(Some(&paths.config_file), &stale).enabled);
    std::fs::remove_dir_all(root).unwrap();
}
