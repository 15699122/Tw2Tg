use super::*;

#[test]
fn attempt_rename_recovery_is_idempotent_and_checks_identity() {
    let root = std::env::temp_dir().join(format!("xarchive-file-store-v2-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let files = FileStore::new(&root).expect("file store");
    let staging = files
        .create_attempt_staging_dir("job-v2", 1)
        .expect("attempt staging");
    let source = staging.join("source.jpg");
    fs::write(&source, b"media").expect("write source");
    let digest = FileStore::sha256(&source).expect("hash");

    let plan = crate::ArchiveRenamePlan {
        source_path: "source.jpg".into(),
        temporary_path: ".temp.jpg".into(),
        final_path: "final.jpg".into(),
        size_bytes: 5,
        sha256: digest.clone(),
    };
    files
        .recover_attempt_rename("job-v2", 1, &plan, true)
        .expect("first move");
    files
        .recover_attempt_rename("job-v2", 1, &plan, true)
        .expect("replay completed move");
    assert!(!source.exists());
    assert_eq!(
        fs::read(staging.join(".temp.jpg")).expect("destination"),
        b"media"
    );

    assert!(
        files
            .recover_attempt_rename(
                "job-v2",
                1,
                &crate::ArchiveRenamePlan {
                    source_path: "source.jpg".into(),
                    temporary_path: ".temp.jpg".into(),
                    final_path: "final.jpg".into(),
                    size_bytes: 4,
                    sha256: digest.clone(),
                },
                false
            )
            .is_err()
    );
    assert!(!staging.join("final.jpg").exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn attempt_rename_recovery_rejects_conflicts_and_missing_edges() {
    let root = std::env::temp_dir().join(format!(
        "xarchive-file-store-v2-conflict-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let files = FileStore::new(&root).expect("file store");
    let attempt = files
        .create_attempt_staging_dir("job-v2", 1)
        .expect("attempt staging");
    fs::write(attempt.join("source.jpg"), b"media").expect("source");
    fs::write(attempt.join("temporary.jpg"), b"media").expect("conflicting destination");
    let digest = FileStore::sha256(attempt.join("source.jpg")).expect("hash");
    let identity = crate::ArchiveRenamePlan {
        source_path: "source.jpg".into(),
        temporary_path: "temporary.jpg".into(),
        final_path: "final.jpg".into(),
        size_bytes: 5,
        sha256: digest,
    };

    assert!(
        files
            .recover_attempt_rename("job-v2", 1, &identity, true)
            .is_err()
    );
    assert!(
        files
            .recover_attempt_rename(
                "job-v2",
                1,
                &crate::ArchiveRenamePlan {
                    source_path: "missing.jpg".into(),
                    temporary_path: "missing-temporary.jpg".into(),
                    final_path: "missing-final.jpg".into(),
                    size_bytes: 5,
                    sha256: "a".repeat(64),
                },
                true
            )
            .is_err()
    );
    assert_eq!(
        fs::read(attempt.join("source.jpg")).expect("source remains"),
        b"media"
    );
    assert_eq!(
        fs::read(attempt.join("temporary.jpg")).expect("destination remains"),
        b"media"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn allows_a_missing_intermediate_directory() {
    let root = temp_root("missing");
    let store = FileStore::new(&root).expect("file store");

    store
        .write_text(Path::new("new/nested/file.txt"), "ok")
        .expect("write creates the missing directories");
    assert!(root.join("new/nested/file.txt").is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn attempt_staging_isolates_concurrent_attempts() {
    let root = temp_root("attempt-isolation");
    let store = FileStore::new(&root).expect("file store");

    let first = store
        .create_attempt_staging_dir("job-1", 1)
        .expect("attempt 1 staging");
    let second = store
        .create_attempt_staging_dir("job-1", 2)
        .expect("attempt 2 staging");
    assert_ne!(first, second);
    fs::write(first.join("stale.bin"), b"stale").expect("stale payload");
    assert!(!second.join("stale.bin").exists());
    assert!(store.existing_attempt_staging_dir("job-1", 1).is_ok());
    assert!(store.existing_attempt_staging_dir("job-1", 3).is_err());
    assert!(store.create_attempt_staging_dir("job-1", 0).is_err());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn fresh_attempt_staging_creation_never_reuses_an_existing_attempt() {
    let root = temp_root("fresh-attempt");
    let store = FileStore::new(&root).expect("file store");
    let first = store
        .create_attempt_staging_dir("job-1", 1)
        .expect("first attempt directory");
    fs::write(first.join("partial.bin"), b"partial").expect("partial output");

    let duplicate = store.create_attempt_staging_dir("job-1", 1);
    assert!(matches!(
        duplicate,
        Err(StorageError::Io(ref error)) if error.kind() == io::ErrorKind::AlreadyExists
    ));
    assert_eq!(
        fs::read(first.join("partial.bin")).expect("original partial output remains"),
        b"partial"
    );

    let retry = store
        .create_attempt_staging_dir("job-1", 2)
        .expect("new durable attempt gets a fresh directory");
    assert_ne!(first, retry);
    assert!(!retry.join("partial.bin").exists());
    assert!(store.create_attempt_staging_dir("job-1", 0).is_err());
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn fresh_attempt_staging_rejects_a_symlinked_job_parent() {
    use std::os::unix::fs::symlink;

    let root = temp_root("attempt-parent-symlink");
    let store = FileStore::new(&root).expect("file store");
    let outside = temp_root("attempt-parent-outside");
    fs::create_dir_all(&outside).expect("outside directory");
    symlink(&outside, root.join("_staging/job-linked")).expect("job parent symlink");

    assert!(matches!(
        store.create_attempt_staging_dir("job-linked", 1),
        Err(StorageError::InvalidPath)
    ));
    assert!(!outside.join("attempt-1").exists());

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn fresh_attempt_staging_rejects_a_symlinked_staging_root() {
    use std::os::unix::fs::symlink;

    let root = temp_root("attempt-root-symlink");
    let archive = root.join("archive");
    let staging_link = root.join("staging-link");
    let outside = temp_root("attempt-root-outside");
    fs::create_dir_all(&archive).expect("archive directory");
    fs::create_dir_all(&outside).expect("outside directory");
    fs::create_dir(&staging_link).expect("initial staging root");
    let store = FileStore::with_staging_root(&archive, &staging_link).expect("file store");
    fs::remove_dir(&staging_link).expect("remove initial staging root");
    symlink(&outside, &staging_link).expect("staging root symlink");

    assert!(matches!(
        store.create_attempt_staging_dir("job-1", 1),
        Err(StorageError::InvalidPath)
    ));
    assert!(!outside.join("job-1/attempt-1").exists());

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn attempt_file_creation_never_overwrites_existing_files_or_links() {
    let root = temp_root("attempt-create-only");
    let store = FileStore::new(&root).expect("file store");
    let staging = store
        .create_attempt_staging_dir("job-1", 1)
        .expect("attempt");
    let path = store
        .create_attempt_file("job-1", 1, "tweet.json", br#"{"safe":true}"#)
        .expect("first create");
    assert_eq!(fs::read(&path).expect("created bytes"), br#"{"safe":true}"#);
    assert!(
        store
            .create_attempt_file("job-1", 1, "tweet.json", b"overwrite")
            .is_err()
    );
    assert_eq!(
        fs::read(&path).expect("original bytes retained"),
        br#"{"safe":true}"#
    );

    #[cfg(unix)]
    {
        let outside = temp_root("attempt-create-outside");
        fs::create_dir_all(&outside).expect("outside");
        let target = outside.join("target.json");
        fs::write(&target, b"untouched").expect("target");
        std::os::unix::fs::symlink(&target, staging.join("linked.json")).expect("symlink");
        assert!(
            store
                .create_attempt_file("job-1", 1, "linked.json", b"attack")
                .is_err()
        );
        assert_eq!(fs::read(&target).expect("target retained"), b"untouched");
        let _ = fs::remove_dir_all(outside);
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn replay_attempt_stage_moves_pending_edges_atomically() {
    let root = temp_root("attempt-replay");
    let store = FileStore::new(&root).expect("file store");
    let staging = store
        .create_attempt_staging_dir("job-2", 1)
        .expect("attempt staging");
    fs::write(staging.join("source.bin"), b"payload").expect("source");
    let hash = FileStore::sha256(staging.join("source.bin")).expect("source hash");
    let identity = crate::ArchiveRenamePlan {
        source_path: "source.bin".into(),
        temporary_path: "moved.bin".into(),
        final_path: "final.bin".into(),
        size_bytes: 7,
        sha256: hash,
    };
    let edge = crate::workflow_bridge::RenameEdge::new(
        &identity.source_path,
        &identity.temporary_path,
        &identity.final_path,
    )
    .expect("valid edge");
    store
        .replay_attempt_rename_stage(
            "job-2",
            1,
            std::slice::from_ref(&edge),
            std::slice::from_ref(&identity),
            true,
        )
        .expect("pending move");
    assert!(staging.join("moved.bin").is_file());
    assert_eq!(
        fs::read(staging.join("moved.bin")).expect("moved intact"),
        b"payload"
    );
    store
        .replay_attempt_rename_stage(
            "job-2",
            1,
            std::slice::from_ref(&edge),
            std::slice::from_ref(&identity),
            true,
        )
        .expect("completed move replays by verified identity");
    let conflicting = crate::ArchiveRenamePlan {
        sha256: "0".repeat(64),
        ..identity.clone()
    };
    let conflict_edge = crate::workflow_bridge::RenameEdge::new(
        &conflicting.source_path,
        &conflicting.temporary_path,
        &conflicting.final_path,
    )
    .expect("valid conflicting edge");
    assert!(
        store
            .replay_attempt_rename_stage("job-2", 1, &[conflict_edge], &[conflicting], true)
            .is_err()
    );
    let absent_identity = crate::ArchiveRenamePlan {
        source_path: "absent.bin".into(),
        temporary_path: "temporary.bin".into(),
        final_path: "final.bin".into(),
        size_bytes: 1,
        sha256: "0".repeat(64),
    };
    let absent_edge = crate::workflow_bridge::RenameEdge::new(
        &absent_identity.source_path,
        &absent_identity.temporary_path,
        &absent_identity.final_path,
    )
    .expect("valid absent edge");
    assert!(
        store
            .replay_attempt_rename_stage("job-2", 1, &[absent_edge], &[absent_identity], true,)
            .is_err()
    );

    let mismatch = crate::workflow_bridge::RenameEdge::new(
        "other-source.bin",
        &identity.temporary_path,
        &identity.final_path,
    )
    .expect("individually valid edge");
    assert!(
        store
            .replay_attempt_rename_stage(
                "job-2",
                1,
                &[mismatch],
                std::slice::from_ref(&identity),
                true,
            )
            .is_err()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let outside = temp_root("attempt-replay-outside");
        fs::write(outside.join("payload.bin"), b"payload").expect("outside payload");
        let linked_identity = crate::ArchiveRenamePlan {
            source_path: "link.bin".into(),
            temporary_path: "linked-temp.bin".into(),
            final_path: "linked-final.bin".into(),
            size_bytes: 7,
            sha256: identity.sha256.clone(),
        };
        let linked_edge = crate::workflow_bridge::RenameEdge::new(
            &linked_identity.source_path,
            &linked_identity.temporary_path,
            &linked_identity.final_path,
        )
        .expect("valid symlink edge");
        symlink(outside.join("payload.bin"), staging.join("link.bin"))
            .expect("create source symlink");
        assert!(
            store
                .replay_attempt_rename_stage("job-2", 1, &[linked_edge], &[linked_identity], true,)
                .is_err()
        );
        let _ = fs::remove_dir_all(outside);
    }
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn existing_attempt_staging_rejects_symlinked_ancestors() {
    use std::os::unix::fs::symlink;

    let root = temp_root("existing-attempt-ancestors");
    let archive = root.join("archive");
    let staging = root.join("staging");
    let outside = root.join("outside");
    fs::create_dir_all(&archive).expect("archive");
    fs::create_dir_all(&outside).expect("outside");

    // First reject replacement of the staging root itself.
    let store = FileStore::with_staging_root(&archive, &staging).expect("store");
    let attempt = store
        .create_attempt_staging_dir("job-1", 1)
        .expect("attempt");
    fs::write(attempt.join("media.jpg"), b"media").expect("media");
    fs::remove_dir_all(&staging).expect("remove staging root");
    symlink(&outside, &staging).expect("staging root symlink");
    assert!(matches!(
        store.existing_attempt_staging_dir("job-1", 1),
        Err(StorageError::InvalidPath)
    ));
    fs::remove_file(&staging).expect("remove staging symlink");

    // Then reject a replaced job ancestor even when the attempt directory
    // exists outside under the same textual path.
    fs::create_dir(&staging).expect("restore staging root");
    let outside_job = outside.join("job-1");
    fs::create_dir(&outside_job).expect("outside job");
    fs::create_dir(outside_job.join("attempt-1")).expect("outside attempt");
    symlink(&outside_job, staging.join("job-1")).expect("job symlink");
    assert!(matches!(
        store.existing_attempt_staging_dir("job-1", 1),
        Err(StorageError::InvalidPath)
    ));
    let _ = fs::remove_dir_all(root);
}
