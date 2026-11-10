//! `archive_service` — preview coverage.

use super::*;

#[test]
fn attempt_archive_preview_uses_attempt_payload_not_legacy_staging() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job("job-1", tweet_row_id, "now")
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    fs::write(
        files
            .staging_dir("job-1")
            .expect("legacy staging")
            .join("01.jpg"),
        b"legacy payload",
    )
    .expect("legacy media");
    let attempt = files
        .create_attempt_staging_dir("job-1", 1)
        .expect("attempt staging");
    fs::write(attempt.join("01.jpg"), b"attempt payload").expect("attempt media");
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "01.jpg".into(),
        size_bytes: 0,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let raw = serde_json::json!({
        "tweet_id": "123",
        "media": [{"id": "media-1", "type": "photo"}]
    });
    let service = ArchiveService::new(database, files);

    let preview = service
        .preview_sidecar_archive_attempt(
            "job-1",
            1,
            "123",
            &raw,
            &sidecar_files,
            "2026-10-12T00:00:00Z",
            &BatchOutputSettings::default(),
        )
        .expect("attempt-scoped preview");

    assert_eq!(preview.metadata.media[0].size_bytes, 15);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn attempt_archive_preview_binds_metadata_to_transfer_relative_path_and_hash() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job("job-transfer", tweet_row_id, "now")
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    let attempt = files
        .create_attempt_staging_dir("job-transfer", 4)
        .expect("attempt staging");
    let transferred = b"verified transfer payload";
    fs::create_dir_all(attempt.join("media")).expect("transfer output directory");
    fs::write(attempt.join("media/01.jpg"), transferred).expect("transfer output");
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "media/01.jpg".into(),
        size_bytes: transferred.len() as u64,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let raw = serde_json::json!({
        "tweet_id": "123",
        "media": [{"id": "media-1", "type": "photo"}]
    });
    let service = ArchiveService::new(database, files);

    let preview = service
        .preview_sidecar_archive_attempt(
            "job-transfer",
            4,
            "123",
            &raw,
            &sidecar_files,
            "2026-10-14T00:00:00Z",
            &BatchOutputSettings::default(),
        )
        .expect("attempt-scoped transfer preview");

    assert_eq!(preview.metadata.media.len(), 1);
    assert_eq!(
        preview.metadata.media[0].size_bytes,
        transferred.len() as u64
    );
    assert_eq!(
        preview.metadata.media[0].sha256,
        FileStore::sha256(attempt.join("media/01.jpg")).expect("transfer hash")
    );
    assert!(
        service
            .preview_sidecar_archive(
                "job-transfer",
                "123",
                &raw,
                &sidecar_files,
                "2026-10-14T00:00:00Z",
                &BatchOutputSettings::default(),
            )
            .is_err(),
        "legacy Job staging must not resolve an attempt-relative transfer path"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn attempt_archive_preview_never_falls_back_to_legacy_staging() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job("job-1", tweet_row_id, "now")
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    fs::write(
        files
            .staging_dir("job-1")
            .expect("legacy staging")
            .join("01.jpg"),
        b"must not be used",
    )
    .expect("legacy media");
    let service = ArchiveService::new(database, files);
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "01.jpg".into(),
        size_bytes: 0,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];
    let raw = serde_json::json!({
        "tweet_id": "123",
        "media": [{"id": "media-1", "type": "photo"}]
    });

    assert!(
        service
            .preview_sidecar_archive_attempt(
                "job-1",
                1,
                "123",
                &raw,
                &sidecar_files,
                "2026-10-12T00:00:00Z",
                &BatchOutputSettings::default(),
            )
            .is_err()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn attempt_archive_preview_rejects_reported_path_escape() {
    let root = temp_root();
    let database = Database::open_in_memory().expect("database");
    let files = FileStore::new(&root).expect("files");
    let attempt = files
        .create_attempt_staging_dir("job-escape", 3)
        .expect("attempt staging");
    fs::write(root.join("escape.jpg"), b"outside attempt").expect("outside payload");
    let service = ArchiveService::new(database, files);
    let raw = serde_json::json!({"tweet_id": "123"});
    let sidecar_files = vec![xarchive_protocol::DownloadFile {
        relative_path: "../escape.jpg".into(),
        size_bytes: 14,
        media_type: "photo".into(),
        mime_type: Some("image/jpeg".into()),
    }];

    assert!(matches!(
        service.preview_sidecar_archive_attempt(
            "job-escape",
            3,
            "123",
            &raw,
            &sidecar_files,
            "2026-10-12T00:00:00Z",
            &BatchOutputSettings::default(),
        ),
        Err(StorageError::InvalidPath)
    ));
    assert_eq!(
        fs::read(root.join("escape.jpg")).expect("outside unchanged"),
        b"outside attempt"
    );
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(attempt);
}
