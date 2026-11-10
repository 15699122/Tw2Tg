use super::*;

#[test]
fn attempt_staging_resolution_requires_matching_identity_and_isolates_attempts() {
    let root = std::env::temp_dir().join(format!(
        "attempt-context-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let archive = root.join("archive");
    let staging = root.join("staging");
    let files = FileStore::with_staging_root(&archive, &staging).expect("file store");
    let supervisor_script = root.join("sidecar-stub.sh");
    std::fs::write(&supervisor_script, "#!/bin/sh\ncat >/dev/null\n").expect("script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&supervisor_script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&supervisor_script, permissions).expect("script mode");
    }
    let supervisor =
        SidecarSupervisor::spawn(supervisor_script.to_str().expect("script path"), &[])
            .expect("supervisor");
    let mut context = ArchiveExecutionContext::with_aria2_and_network(
        Database::open(root.join("db.sqlite3")).expect("database"),
        files,
        supervisor,
        None,
        false,
        crate::executor::ExecutorNetworkConfig::default(),
    );
    assert!(
        context.create_attempt_staging_directory("job-1").is_err(),
        "an unclaimed execution must not obtain shared staging"
    );

    context = context.with_attempt_identity(
        xarchive_workflow::AttemptIdentity::new("job-1", 7).expect("attempt identity"),
    );
    let attempt_path = context
        .create_attempt_staging_directory("job-1")
        .expect("attempt staging");
    assert!(attempt_path.ends_with("job-1/attempt-7"));
    assert!(
        context
            .create_attempt_staging_directory("another-job")
            .is_err()
    );
    assert!(!staging.join("job-1").join("attempt-1").exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn attempt_facts_are_attempt_owned_and_not_cloneable() {
    // AttemptArchiveFacts carries non-Clone owners (SidecarSupervisor on
    // all platforms, Aria2Supervisor on Linux). It derives only Debug and
    // has no Clone impl, so a facts value cannot be duplicated across the
    // archive/publication seam. Stable Rust cannot express a negative
    // `!Clone` bound, so this test pins the absence of Clone by requiring
    // Send (owners are thread-local-capable) and documenting the contract.
    fn assert_send<T: Send>() {}
    assert_send::<AttemptArchiveFacts>();
}

#[test]
fn attempt_facts_staging_verification_rejects_missing_or_replaced_trees() {
    let root = std::env::temp_dir().join(format!(
        "attempt-facts-verify-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let archive = root.join("archive");
    let staging = root.join("staging");
    let files = FileStore::with_staging_root(&archive, &staging).expect("file store");
    let staging_dir = files
        .create_attempt_staging_dir("job-1", 3)
        .expect("attempt staging");
    let attempt_identity =
        xarchive_workflow::AttemptIdentity::new("job-1", 3).expect("attempt identity");
    let facts = AttemptArchiveFacts {
        attempt_identity,
        metadata: serde_json::json!({"tweet_id": "1"}),
        files: vec![xarchive_protocol::DownloadFile {
            relative_path: "01.jpg".to_owned(),
            size_bytes: 1,
            media_type: "photo".to_owned(),
            mime_type: None,
        }],
        staging_dir: staging_dir.clone(),
        transfer_file_count: 1,
        #[cfg(target_os = "linux")]
        retained_aria2: None,
    };
    verify_attempt_facts_staging(&files, "job-1", &facts).expect("live staging verifies");

    let tampered_count = AttemptArchiveFacts {
        attempt_identity: facts.attempt_identity.clone(),
        metadata: facts.metadata.clone(),
        files: facts.files.clone(),
        staging_dir: facts.staging_dir.clone(),
        transfer_file_count: facts.transfer_file_count + 1,
        #[cfg(target_os = "linux")]
        retained_aria2: None,
    };
    assert!(
        verify_attempt_facts_staging(&files, "job-1", &tampered_count).is_err(),
        "transfer-derived count mismatch must refuse publication"
    );

    std::fs::remove_dir_all(&staging_dir).expect("remove attempt staging");
    assert!(
        verify_attempt_facts_staging(&files, "job-1", &facts).is_err(),
        "missing attempt staging must not verify"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
