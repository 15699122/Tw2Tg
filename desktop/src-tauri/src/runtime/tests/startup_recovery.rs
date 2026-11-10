use super::*;

#[test]
fn recovery_refusal_is_visible_in_application_logs_without_raw_errors() {
    let root = std::env::temp_dir().join(format!(
        "rt-log-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let state = RuntimeState::initialize_at(root.clone());
    state.record_recovery_results(vec![
        ("job-refused".into(), Err("private-token-and-path".into())),
        ("job-contended".into(), Ok(false)),
    ]);
    let lines = LogFile::read_recent(&state.logs_root, 500)
        .unwrap()
        .join("\n");
    assert!(lines.contains("recovery refused job=job-refused"));
    assert!(lines.contains("durable attempt retained"));
    assert!(lines.contains("recovery deferred by contention job=job-contended"));
    assert!(!lines.contains("private-token-and-path"));
    drop(state);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fatal_startup_recovery_error_propagates_without_logging_raw_details() {
    let root = std::env::temp_dir().join(format!(
        "rt-recovery-enumeration-{}-{}",
        std::process::id(),
        timestamp_marker()
    ));
    let state = RuntimeState::initialize_at(root.clone());
    let error = "private-db-path-and-token".to_owned();
    assert_eq!(
        state.record_startup_recovery_report(crate::executor::StartupRecoveryReport {
            outcomes: Vec::new(),
            fatal_error: Some(error.clone()),
        }),
        Err(error)
    );
    let lines = LogFile::read_recent(&state.logs_root, 500)
        .unwrap()
        .join("\n");
    assert!(lines.contains("startup recovery enumeration unavailable"));
    assert!(!lines.contains("private-db-path-and-token"));
    drop(state);
    std::fs::remove_dir_all(root).unwrap();
}
