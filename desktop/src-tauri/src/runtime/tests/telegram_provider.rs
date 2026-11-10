use super::*;

#[test]
fn provider_lifecycle_fails_closed_and_deletion_revokes_before_native_failure() {
    struct FailingDelete;
    impl xarchive_telegram::SecretStore for FailingDelete {
        fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
            Ok(None)
        }
        fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
            Ok(())
        }
        fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
            Err(xarchive_telegram::SecretStoreError::Unavailable(
                "sensitive-native-detail".into(),
            ))
        }
    }
    let root = std::env::temp_dir().join(format!(
        "tg-provider-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let mut state = RuntimeState::initialize_at(root.clone());
    state.config.telegram.enabled = true;
    assert_eq!(
        state.restart_telegram_sender().unwrap_err(),
        "platform credential provider unavailable"
    );
    assert!(state.telegram_worker.is_none());
    state.config.telegram.enabled = false;
    state
        .install_telegram_secrets(Box::new(FailingDelete))
        .unwrap();
    let database = state.database.as_ref().unwrap();
    let generation = database
        .prepare_telegram_credential_generation("telegram-bot:123", "native-ref", "t0")
        .unwrap();
    database
        .activate_telegram_credential_generation(generation, None)
        .unwrap();
    assert_eq!(
        state.delete_telegram_secret().unwrap_err(),
        "inactive credential cleanup failed"
    );
    assert!(
        state
            .database
            .as_ref()
            .unwrap()
            .active_telegram_credential_generation()
            .unwrap()
            .is_none()
    );
    state.telegram_secrets.as_mut().unwrap();
    drop(state);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn injected_provider_starts_and_restarts_exactly_one_worker_with_loopback_direct() {
    let root = std::env::temp_dir().join(format!(
        "tg-restart-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let mut state = RuntimeState::initialize_at(root.clone());
    state.download_setup_required = false;
    state.config.telegram.enabled = true;
    state.config.telegram.endpoint_mode = xarchive_telegram::EndpointMode::Local;
    state.config.telegram.api_base = "http://127.0.0.1:9".into();
    state.config.network.proxy_mode = crate::config::ProxyMode::Manual;
    state.config.network.proxy = Some("http://127.0.0.1:1".into());
    assert_eq!(state.telegram_proxy().unwrap(), None);
    state
        .install_telegram_secrets(Box::new(xarchive_telegram::MemorySecretStore::default()))
        .unwrap();
    assert!(state.telegram_worker.is_some());
    state.restart_telegram_sender().unwrap();
    assert!(state.telegram_worker.is_some());
    state.config.telegram.enabled = false;
    state.restart_telegram_sender().unwrap();
    assert!(state.telegram_worker.is_none());
    drop(state);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn disabled_runtime_does_not_start_sender_or_read_credentials() {
    struct ForbiddenSecrets;
    impl xarchive_telegram::SecretStore for ForbiddenSecrets {
        fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
            panic!("disabled runtime must not read credentials");
        }
        fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
            panic!("disabled runtime must not write credentials");
        }
        fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
            panic!("disabled runtime must not delete credentials");
        }
    }
    let root = std::env::temp_dir().join(format!(
        "telegram-runtime-disabled-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let mut state = RuntimeState::initialize_at(root.clone());
    assert!(!state.config.telegram.enabled);
    state
        .start_telegram_sender(ForbiddenSecrets, None, |_| panic!("no batch expected"))
        .unwrap();
    assert!(state.telegram_worker.is_none());
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_refuses_replacement_until_inflight_worker_exits() {
    let root = std::env::temp_dir().join(format!(
        "telegram-runtime-replace-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let mut state = RuntimeState::initialize_at(root.clone());
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    state
        .install_telegram_worker(std::time::Duration::from_secs(60), move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        })
        .unwrap();
    state.telegram_worker.as_ref().unwrap().wake();
    started_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert!(!state.stop_telegram_worker(std::time::Duration::ZERO));
    assert!(state.telegram_worker.is_some());
    let (replacement_tx, replacement_rx) = std::sync::mpsc::channel();
    assert_eq!(
        state
            .install_telegram_worker(std::time::Duration::from_secs(60), move |_| {
                replacement_tx.send(()).unwrap();
            })
            .unwrap_err(),
        "telegram worker still stopping"
    );
    assert!(state.telegram_worker.is_some());
    assert!(replacement_rx.try_recv().is_err());
    release_tx.send(()).unwrap();
    assert!(state.stop_telegram_worker(std::time::Duration::from_secs(2)));
    let (tx, rx) = std::sync::mpsc::channel();
    state
        .install_telegram_worker(std::time::Duration::from_secs(60), move |_| {
            tx.send(()).unwrap();
        })
        .unwrap();
    state.telegram_worker.as_ref().unwrap().wake();
    rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
    assert!(state.stop_telegram_worker(std::time::Duration::from_secs(2)));
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_owns_and_stops_injected_telegram_worker() {
    let root = std::env::temp_dir().join(format!(
        "telegram-runtime-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let mut state = RuntimeState::initialize_at(root.clone());
    let (tx, rx) = std::sync::mpsc::channel();
    state
        .install_telegram_worker(std::time::Duration::from_secs(60), move |signal| {
            assert!(!signal.is_stopping());
            tx.send(()).unwrap();
        })
        .unwrap();
    state.telegram_worker.as_ref().unwrap().wake();
    rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
    assert!(state.stop_telegram_worker(std::time::Duration::from_secs(2)));
    assert!(state.telegram_worker.is_none());
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}
