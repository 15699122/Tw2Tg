use super::*;

#[cfg(unix)]
#[test]
fn archive_context_shutdown_joins_sidecar_before_completion() {
    let root = std::env::temp_dir().join(format!(
        "archive-sidecar-drain-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    std::fs::create_dir_all(&root).expect("root");
    let files =
        FileStore::with_staging_root(root.join("archive"), root.join("staging")).expect("files");
    let mut context = ArchiveExecutionContext::with_aria2_and_network(
        Database::open(root.join("db.sqlite3")).expect("database"),
        files,
        SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).expect("sidecar"),
        None,
        false,
        crate::executor::ExecutorNetworkConfig::default(),
    );

    context
        .shutdown_sidecar()
        .expect("owned Sidecar and readers drained");

    assert!(context.supervisor.readers_finished());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn attempt_facts_require_sidecar_drain_and_carry_transfer_count() {
    let root = std::env::temp_dir().join(format!(
        "attempt-facts-drain-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    std::fs::create_dir_all(&root).expect("root");
    let mut supervisor =
        SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).expect("sidecar");
    let staging = root.join("staging").join("job-1").join("attempt-1");
    std::fs::create_dir_all(&staging).expect("attempt staging");
    let metadata = serde_json::json!({"tweet_id": "1"});
    let files = vec![xarchive_protocol::DownloadFile {
        relative_path: "01.jpg".to_owned(),
        size_bytes: 3,
        media_type: "photo".to_owned(),
        mime_type: Some("image/jpeg".to_owned()),
    }];

    let attempt_identity =
        xarchive_workflow::AttemptIdentity::new("job-1", 1).expect("attempt identity");
    let facts = AttemptArchiveFacts::after_sidecar_drain(
        &mut supervisor,
        attempt_identity.clone(),
        metadata.clone(),
        files,
        staging.clone(),
        #[cfg(target_os = "linux")]
        None,
    )
    .expect("drained Sidecar must yield attempt facts");
    assert_eq!(facts.attempt_identity, attempt_identity);
    assert_eq!(facts.transfer_file_count, 1);
    assert_eq!(facts.staging_dir, staging);
    assert_eq!(facts.files.len(), facts.transfer_file_count);
    assert!(supervisor.readers_finished());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal() {
    use xarchive_workflow::coordination::unix::LinuxCoordinator;

    for (cancelled, mismatched_attempt) in [(true, false), (false, false), (false, true)] {
        let root = std::env::temp_dir().join(format!(
            "pending-finalizer-{}-{}-{cancelled}-{mismatched_attempt}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let database_path = root.join("archive.sqlite3");
        let mut database = Database::open(&database_path).unwrap();
        let tweet_row_id = database
            .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
            .unwrap();
        database
            .create_archive_job_with_optional_recovery_contract(
                "job-1",
                tweet_row_id,
                4,
                "pending-request",
                "{}",
                Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
                "now",
            )
            .unwrap();
        let attempt = database.begin_archive_attempt("job-1", "now").unwrap();
        for state in [
            xarchive_core::JobState::Validating,
            xarchive_core::JobState::MetadataReady,
            xarchive_core::JobState::Downloading,
        ] {
            database.transition_job("job-1", state, "now").unwrap();
        }
        let archive_root = root.join("archive");
        let staging_root = root.join("staging");
        let files = FileStore::with_staging_root(&archive_root, &staging_root).unwrap();
        let staging = files.create_attempt_staging_dir("job-1", attempt).unwrap();
        let supervisor = SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).unwrap();
        let identity = xarchive_workflow::AttemptIdentity::new("job-1", attempt).unwrap();
        let mut context = ArchiveExecutionContext::with_aria2_and_network(
            database,
            files,
            supervisor,
            None,
            false,
            crate::executor::ExecutorNetworkConfig::default(),
        )
        .with_attempt_identity(identity.clone());
        let facts = AttemptArchiveFacts::after_sidecar_drain(
            &mut context.supervisor,
            identity.clone(),
            serde_json::json!({"tweet_id":"123","media":[]}),
            Vec::new(),
            staging,
            #[cfg(target_os = "linux")]
            None,
        )
        .unwrap();
        let request = ArchiveTweetRequest {
            tweet: xarchive_protocol::BrowserTweet {
                tweet_id: "123".into(),
                url: "https://x.com/alice/status/123".into(),
                username: Some("alice".into()),
                display_name: Some("Alice".into()),
                text: None,
                created_at: None,
                tweet_type: "post".into(),
                reply_to: None,
                quoted_tweet: None,
            },
            browser: None,
            profile: None,
        };
        let job = JobSnapshot {
            job_id: "job-1".into(),
            tweet_id: "123".into(),
            state: xarchive_core::JobState::Downloading,
        };
        let mut pending = Box::new(
            PendingArchivePublication::from_verified_facts(
                context,
                facts,
                &request,
                &job,
                BatchOutputSettings::default(),
                tweet_row_id,
                "now",
            )
            .unwrap_or_else(|(_, error)| panic!("pending construction failed: {error:?}")),
        );
        let service_address = &*pending.service as *const ArchiveService;
        let supervisor_address = &pending._sidecar_owner as *const SidecarSupervisor;
        let coordinator = LinuxCoordinator::new(root.join("locks")).unwrap();
        let destination = archive_root.join("Tweets/123");
        let session =
            xarchive_workflow::ExecutionReservation::acquire(&coordinator, "job-1", &destination)
                .unwrap()
                .claim(|| {
                    xarchive_workflow::ClaimOutcome::Claimed(if mismatched_attempt {
                        xarchive_workflow::AttemptIdentity::new("job-1", 2).unwrap()
                    } else {
                        identity
                    })
                })
                .unwrap();
        let publication_owner = xarchive_workflow::PublicationOwner::for_session(&session);
        let cancellation = CancellationToken::default();
        if cancelled {
            cancellation.cancel();
        }

        let error = pending
            .finalize(&session, &publication_owner, &cancellation)
            .unwrap_err();
        assert_eq!(
            error.error_code,
            if cancelled {
                "TRANSFER_CANCELLED"
            } else if mismatched_attempt {
                "PUBLICATION_IDENTITY_MISMATCH"
            } else {
                "PUBLICATION_INCOMPLETE_DRAIN"
            }
        );
        assert!(pending.finalized);
        assert_eq!(&*pending.service as *const ArchiveService, service_address);
        assert_eq!(
            &pending._sidecar_owner as *const SidecarSupervisor,
            supervisor_address
        );
        assert!(
            pending
                .service
                .database
                .archive_recovery_journal("job-1")
                .unwrap()
                .is_none()
        );
        assert!(!destination.exists());

        drop(pending);
        drop(session);
        drop(coordinator);
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// V1-02b: a genuinely registered writer whose drain is incomplete must be
/// refused at the real authorization boundary.
///
/// Unlike `pending_finalizer_cancel_and_incomplete_drain_keep_owner_and_create_no_journal`,
/// this registers a real production `sidecar` role through
/// `WriterRegistration::register_linux` inside a real delegated cgroup, then
/// makes the drain fail by keeping a process alive in the group. The registry
/// therefore reports a registered-but-undrained writer, and authorization
/// must fail closed.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn registered_writer_with_incomplete_drain_is_refused_by_authorization() {
    use std::time::Duration;
    use xarchive_workflow::coordination::unix::LinuxCoordinator;
    use xarchive_workflow::linux_containment::{
        AttemptIdentity as AdapterIdentity, LinuxContainment,
    };

    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let delegated_parent =
        std::path::Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));

    let root = std::env::temp_dir().join(format!(
        "v1-02b-registered-undrained-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let database_path = root.join("archive.sqlite3");
    let staging_root = root.join("staging");
    let archive_root = root.join("archive");
    std::fs::create_dir_all(&staging_root).unwrap();
    std::fs::create_dir_all(&archive_root).unwrap();

    let mut database = Database::open(&database_path).unwrap();
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .unwrap();
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            4,
            "registered-undrained-request",
            "{}",
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .unwrap();
    let attempt = database.begin_archive_attempt("job-1", "now").unwrap();
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database.transition_job("job-1", state, "now").unwrap();
    }
    drop(database);

    // Establish the real session and its writer-set owner first: the
    // registration handle can only come from a live session.
    let coordinator = LinuxCoordinator::new(root.join("locks")).unwrap();
    let destination = archive_root.join("Tweets/123");
    let identity = xarchive_workflow::AttemptIdentity::new("job-1", attempt).unwrap();
    let session =
        xarchive_workflow::ExecutionReservation::acquire(&coordinator, "job-1", &destination)
            .unwrap()
            .claim(|| xarchive_workflow::ClaimOutcome::Claimed(identity.clone()))
            .unwrap();
    let publication_owner = xarchive_workflow::PublicationOwner::for_session(&session);
    let registration = publication_owner.registration();

    // Create a real recorded writer domain inside the delegated cgroup and
    // register the production `sidecar` role on it.
    let record_root = root.join("containment-records");
    std::fs::create_dir_all(&record_root).unwrap();
    let adapter_identity = AdapterIdentity::new(
        staging_root.to_string_lossy().as_ref(),
        "job-1",
        u64::from(attempt),
    )
    .unwrap()
    .with_writer("sidecar")
    .unwrap();
    let mut owner =
        LinuxContainment::create_recorded(&record_root, &adapter_identity, &delegated_parent)
            .expect("create the recorded writer group in the delegated cgroup");
    registration
        .register_linux(&mut owner, &adapter_identity)
        .expect("register the production sidecar role");

    // Keep a process alive inside the group so the drain cannot reach
    // `populated 0` and therefore never marks the role drained.
    let launcher = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let (_stdin, _stdout, _stderr) = owner
        .spawn_piped_with_environment(
            std::path::Path::new("/usr/bin/python3"),
            &launcher,
            std::path::Path::new("/usr/bin/sleep"),
            &["30"],
            &[],
            &[],
        )
        .expect("launch a long-lived member inside the registered writer group");

    // Close admission first, before any drain. Production finalize closes
    // admission; doing it here isolates the drain condition from the
    // admission condition for the authorization check below.
    publication_owner
        .close_admission()
        .expect("close writer admission before drain");

    // With admission closed but the writer still undrained, authorization
    // must refuse. The sole remaining cause is the incomplete drain.
    assert_eq!(
        publication_owner.authorize(&session).err(),
        Some(xarchive_workflow::PublicationError::IncompleteDrain),
        "closed admission plus undrained writer must not authorize publication"
    );

    // `stop_and_drain` is the only thing that can mark the role drained. Run
    // it for real against the populated group and confirm the transition.
    owner
        .stop_and_drain(Duration::from_secs(10))
        .expect("a real drain must succeed for a killable group");
    let _ = owner.remove_group();

    // With admission closed and the writer fully drained, authorization
    // succeeds: the refusal above was caused by the incomplete drain alone.
    assert!(
        publication_owner.authorize(&session).is_ok(),
        "a fully drained registered writer with closed admission must authorize publication"
    );

    // No PREPARED journal and no final directory may exist for the refused
    // attempt.
    let database = Database::open(&database_path).unwrap();
    assert!(
        database
            .archive_recovery_journal("job-1")
            .unwrap()
            .is_none(),
        "a refused publication must not create a PREPARED journal"
    );
    assert!(
        !destination.exists(),
        "a refused publication must not create a final directory"
    );
    drop(database);

    drop(owner);
    drop(session);
    drop(coordinator);
    let _ = std::fs::remove_dir_all(root);
}

/// V1-02b production acceptance: a *registered* writer whose drain is
/// incomplete must be refused by the real `PendingArchivePublication` ->
/// `PublicationOwner::authorize` boundary, and the same owner must authorize
/// once that writer has really drained.
///
/// The refusal is therefore isolated to the incomplete drain of a genuinely
/// registered writer, and it is observed at the production finalizer rather
/// than at a direct `authorize` call.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn registered_undrained_writer_is_refused_by_production_finalizer() {
    use std::time::Duration;
    use xarchive_workflow::coordination::unix::LinuxCoordinator;
    use xarchive_workflow::linux_containment::{
        AttemptIdentity as AdapterIdentity, LinuxContainment,
    };

    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let delegated_parent =
        std::path::Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));

    let root = std::env::temp_dir().join(format!(
        "v1-02b-finalizer-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let database_path = root.join("archive.sqlite3");
    let archive_root = root.join("archive");
    let staging_root = root.join("staging");
    std::fs::create_dir_all(&archive_root).unwrap();
    std::fs::create_dir_all(&staging_root).unwrap();

    let mut database = Database::open(&database_path).unwrap();
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .unwrap();
    database
        .create_archive_job_with_optional_recovery_contract(
            "job-1",
            tweet_row_id,
            4,
            "v1-02b-request",
            "{}",
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .unwrap();
    let attempt = database.begin_archive_attempt("job-1", "now").unwrap();
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database.transition_job("job-1", state, "now").unwrap();
    }

    let files = FileStore::with_staging_root(&archive_root, &staging_root).unwrap();
    let staging = files.create_attempt_staging_dir("job-1", attempt).unwrap();
    let supervisor = SidecarSupervisor::spawn("/bin/sh", &["-c", "cat >/dev/null"]).unwrap();
    let identity = xarchive_workflow::AttemptIdentity::new("job-1", attempt).unwrap();
    let mut context = ArchiveExecutionContext::with_aria2_and_network(
        database,
        files,
        supervisor,
        None,
        false,
        crate::executor::ExecutorNetworkConfig::default(),
    )
    .with_attempt_identity(identity.clone());
    let facts = AttemptArchiveFacts::after_sidecar_drain(
        &mut context.supervisor,
        identity.clone(),
        serde_json::json!({"tweet_id":"123","media":[]}),
        Vec::new(),
        staging,
        #[cfg(target_os = "linux")]
        None,
    )
    .unwrap();

    let request = ArchiveTweetRequest {
        tweet: xarchive_protocol::BrowserTweet {
            tweet_id: "123".into(),
            url: "https://x.com/alice/status/123".into(),
            username: Some("alice".into()),
            display_name: Some("Alice".into()),
            text: None,
            created_at: None,
            tweet_type: "post".into(),
            reply_to: None,
            quoted_tweet: None,
        },
        browser: None,
        profile: None,
    };
    let job = JobSnapshot {
        job_id: "job-1".into(),
        tweet_id: "123".into(),
        state: xarchive_core::JobState::Downloading,
    };
    let mut pending = Box::new(
        PendingArchivePublication::from_verified_facts(
            context,
            facts,
            &request,
            &job,
            BatchOutputSettings::default(),
            tweet_row_id,
            "now",
        )
        .unwrap_or_else(|(_, error)| panic!("pending construction failed: {error:?}")),
    );
    let service_address = &*pending.service as *const ArchiveService;
    let supervisor_address = &pending._sidecar_owner as *const SidecarSupervisor;

    let coordinator = LinuxCoordinator::new(root.join("locks")).unwrap();
    let destination = archive_root.join("Tweets/123");
    let session =
        xarchive_workflow::ExecutionReservation::acquire(&coordinator, "job-1", &destination)
            .unwrap()
            .claim(|| xarchive_workflow::ClaimOutcome::Claimed(identity))
            .unwrap();
    let publication_owner = xarchive_workflow::PublicationOwner::for_session(&session);
    let registration = publication_owner.registration();

    // A real recorded writer domain whose production `sidecar` role is
    // registered, plus a live member process so the drain cannot observe
    // `populated 0`.
    let record_root = root.join("containment-records");
    std::fs::create_dir_all(&record_root).unwrap();
    let adapter_identity = AdapterIdentity::new(
        staging_root.to_string_lossy().as_ref(),
        "job-1",
        u64::from(attempt),
    )
    .unwrap()
    .with_writer("sidecar")
    .unwrap();
    let mut owner =
        LinuxContainment::create_recorded(&record_root, &adapter_identity, &delegated_parent)
            .expect("create the recorded writer group in the delegated cgroup");
    registration
        .register_linux(&mut owner, &adapter_identity)
        .expect("register the production sidecar role");
    let launcher = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let (_stdin, _stdout, _stderr) = owner
        .spawn_piped_with_environment(
            std::path::Path::new("/usr/bin/python3"),
            &launcher,
            std::path::Path::new("/usr/bin/sleep"),
            &["30"],
            &[],
            &[],
        )
        .expect("launch a long-lived member inside the registered writer group");

    // Production finalizer with a registered writer that has not drained.
    let cancellation = CancellationToken::default();
    let error = pending
        .finalize(&session, &publication_owner, &cancellation)
        .unwrap_err();
    assert_eq!(
        error.error_code, "PUBLICATION_INCOMPLETE_DRAIN",
        "the production finalizer must refuse a registered, undrained writer"
    );
    assert!(pending.finalized, "the pending publication is one-shot");
    assert_eq!(&*pending.service as *const ArchiveService, service_address);
    assert_eq!(
        &pending._sidecar_owner as *const SidecarSupervisor,
        supervisor_address
    );
    assert!(
        pending
            .service
            .database
            .archive_recovery_journal("job-1")
            .unwrap()
            .is_none(),
        "a refused publication must not create a PREPARED journal"
    );
    assert!(
        !destination.exists(),
        "a refused publication must not create a final directory"
    );

    // Isolate the cause: drain for real, then the same owner must authorize.
    owner
        .stop_and_drain(Duration::from_secs(10))
        .expect("a real drain must succeed for a killable group");
    let _ = owner.remove_group();
    assert!(
        publication_owner.authorize(&session).is_ok(),
        "after the registered writer drains for real the same owner must authorize, \
         so the refusal above was caused by the incomplete drain alone"
    );

    drop(pending);
    drop(session);
    drop(coordinator);
    let _ = std::fs::remove_dir_all(root);
}
