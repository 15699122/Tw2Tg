//! Storage regression suites, grouped by the responsibility they cover.
//!
//! Each group owns one subject and runs against the real SQLite schema; the
//! shared fixtures and imports below are the only items they inherit.

mod archive_recovery;
mod archive_recovery_manifest;
mod archive_service;
mod database;
mod database_telegram;
mod telegram_credentials;
mod telegram_outbox;

use super::*;
use rusqlite::params;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xarchive_core::{ArchiveMetadata, JobState};
use xarchive_telegram::{
    CachedFileId, FileCacheKey, FileIdCacheStore, MediaKind, NewOutboxEntry, OutboxState,
    SendState, SendStateError, SendStateStore, SentSendRecord, TELEGRAM_FILE_CACHE_VERSION,
    TelegramOutboxStore, UNKNOWN_REASON_LEASE_EXPIRED,
};

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "xarchive-storage-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
fn check_archive_intent_recovery(rename_before_restart: bool) {
    let root = temp_root();
    fs::create_dir_all(&root).expect("root");
    let database_path = root.join("recovery.sqlite3");
    let mut database = Database::open(&database_path).expect("database");
    let tweet_row_id = database
        .insert_tweet(
            "intent-1",
            "https://x.com/a/status/intent-1",
            "post",
            "",
            "t0",
        )
        .expect("tweet");
    database
        .create_archive_job("intent-job", tweet_row_id, "t0")
        .expect("job");
    let files = FileStore::new(&root).expect("files");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        database
            .transition_job("intent-job", state, "t0")
            .expect("advance job");
    }
    let staging = files.staging_dir("intent-job").expect("staging");
    let metadata = ArchiveMetadata {
        schema_version: 1,
        tweet_id: "intent-1".into(),
        url: "https://x.com/a/status/intent-1".into(),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: None,
            display_name: None,
        },
        created_at: None,
        text: "recover me".into(),
        media: Vec::new(),
        archived_at: "t1".into(),
        reply_to: None,
        quoted_tweet: None,
    };
    fs::write(
        staging.join("tweet.json"),
        serde_json::to_vec(&metadata).expect("json"),
    )
    .expect("metadata");
    fs::write(staging.join("tweet.txt"), "recover me").expect("text");
    let intent = TelegramArchiveIntentRecord {
        job_id: "intent-job".into(),
        tweet_row_id,
        archive_directory: "Tweets/intent-1".into(),
        state: "PREPARED".into(),
        metadata_text: Some("recover me".into()),
        media_json: "[]".into(),
        bot_identity: Some("bot-fingerprint".into()),
        chat_id: Some("-100".into()),
        message_thread_id: Some(17),
        config_revision: Some(3),
        plan_version: 1,
        created_at: "t1".into(),
        updated_at: "t1".into(),
    };
    database
        .prepare_telegram_archive_intent(&intent)
        .expect("prepare");

    if rename_before_restart {
        // Interruption after rename but before SQLite archive facts.
        files
            .commit_staging("intent-job", "Tweets/intent-1")
            .expect("rename");
    }
    drop(database);
    let database = Database::open(&database_path).expect("reopen database");
    assert_eq!(
        database
            .telegram_archive_intent(tweet_row_id)
            .expect("load after restart"),
        Some(intent.clone())
    );
    let mut service = ArchiveService::new(database, files);
    assert_eq!(
        service
            .recover_telegram_archive_intent(&intent, "t2")
            .expect("recover"),
        "ARCHIVED"
    );
    assert!(root.join("Tweets/intent-1/tweet.json").is_file());
    assert!(!root.join("_staging/intent-job").exists());
    service
        .database
        .prepare_telegram_archive_intent(&intent)
        .expect("repeat original intent after recovery");
    assert_eq!(
        service.database.job_state("intent-job").expect("job state"),
        JobState::Downloaded
    );
    let stored = service
        .database
        .telegram_archive_intent(tweet_row_id)
        .expect("load")
        .expect("record");
    assert_eq!(stored.state, "ARCHIVED");
    assert_eq!(stored.chat_id.as_deref(), Some("-100"));
    assert_eq!(stored.message_thread_id, Some(17));
    // An untrusted/mismatched final record must not become ARCHIVED.
    fs::write(root.join("Tweets/intent-1/tweet.json"), b"{}").expect("damage metadata");
    assert!(
        service
            .recover_telegram_archive_intent(&intent, "t3")
            .is_err()
    );
    assert_eq!(
        service
            .recover_telegram_archive_intent(&stored, "t3")
            .expect("repeat"),
        "ARCHIVED"
    );
    assert_eq!(
        service
            .database
            .list_recoverable_telegram_archive_intents()
            .expect("list")
            .len(),
        1
    );
    drop(service);
    let database = Database::open(&database_path).expect("second restart");
    assert_eq!(
        database
            .telegram_archive_intent(tweet_row_id)
            .expect("persisted progress")
            .expect("intent")
            .state,
        "ARCHIVED"
    );
    drop(database);
    let _ = fs::remove_dir_all(root);
}
/// Build an enrolled v2 job plus its attempt staging tree and the matching
/// manifest/candidate pair, without persisting the journal yet. The
/// forward-publish tests drive `publish_prepared_internal_rename_v2`,
/// which freezes PREPARED itself, so they need the unprepared inputs.
#[allow(clippy::type_complexity)]
fn unprepared_v2_publish_inputs(
    root: &std::path::Path,
    job_id: &str,
) -> (
    Database,
    FileStore,
    i64,
    ArchiveRecoveryManifestV2,
    ArchiveMetadata,
) {
    fs::create_dir_all(root).expect("fixture root");
    let mut database = Database::open(root.join("archive.sqlite3")).expect("database");
    let tweet_row_id = database
        .insert_tweet("123", "https://x.com/alice/status/123", "post", "", "now")
        .expect("tweet");
    database
        .create_archive_job_with_optional_recovery_contract(
            job_id,
            tweet_row_id,
            1,
            "request-1",
            "{}",
            Some(ArchiveRecoveryContract::InternalRenameV2),
            "now",
        )
        .expect("job");
    let attempt = database
        .begin_archive_attempt(job_id, "now")
        .expect("attempt");
    for state in [
        xarchive_core::JobState::Validating,
        xarchive_core::JobState::MetadataReady,
        xarchive_core::JobState::Downloading,
    ] {
        database
            .transition_job(job_id, state, "now")
            .expect("active state");
    }
    let payload = b"recoverable media payload";
    let files = FileStore::new(root).expect("files");
    let staging = files
        .create_attempt_staging_dir(job_id, attempt)
        .expect("attempt staging");
    fs::write(staging.join("source.jpg"), payload).expect("source media");
    let sha = FileStore::sha256(staging.join("source.jpg")).expect("hash");
    let manifest = ArchiveRecoveryManifestV2 {
        metadata_output: None,
        schema_version: 2,
        job_id: job_id.into(),
        attempt_count: attempt,
        tweet_row_id,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        rename_phase: "PLANNED".into(),
        rename_plan: vec![ArchiveRenamePlan {
            source_path: "source.jpg".into(),
            temporary_path: ".rename-01.tmp".into(),
            final_path: "final.jpg".into(),
            size_bytes: payload.len() as u64,
            sha256: sha.clone(),
        }],
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "final.jpg".into(),
            media_id: None,
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: payload.len() as u64,
            sha256: sha.clone(),
        }],
        telegram_tweet_row_id: None,
        export_json: true,
        export_text: true,
    };
    let candidate = ArchiveMetadata {
        schema_version: 1,
        tweet_id: "123".into(),
        url: "https://x.com/alice/status/123".into(),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: None,
            display_name: None,
        },
        created_at: None,
        text: "recoverable".into(),
        media: vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: None,
            media_type: "photo".into(),
            file: "final.jpg".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: payload.len() as u64,
            sha256: sha,
        }],
        archived_at: "now".into(),
        reply_to: None,
        quoted_tweet: None,
    };
    (database, files, tweet_row_id, manifest, candidate)
}
/// Build a PREPARED v2 rename journal on **on-disk SQLite** plus its
/// attempt-bound candidate and attempt staging tree, returning the
/// identifiers recovery needs. On-disk storage lets a crash-window test
/// drop the service and reopen it, proving the journal and candidate
/// survive a process restart rather than living only in memory.
fn prepared_v2_rename_fixture(root: &std::path::Path, job_id: &str) -> (Database, FileStore, i64) {
    let (database, files, tweet_row_id, manifest, candidate) =
        unprepared_v2_publish_inputs(root, job_id);
    database
        .prepare_archive_recovery_v2(&manifest, &candidate, "now")
        .expect("prepared journal");
    (database, files, tweet_row_id)
}
fn new_candidate(tweet_id: &str, state: &str) -> NewBatchCandidate {
    NewBatchCandidate {
        tweet_id: tweet_id.into(),
        url: format!("https://x.com/alice/status/{tweet_id}"),
        created_at: Some("2026-09-20T00:00:00Z".into()),
        tweet_type: "post".into(),
        is_repost: false,
        has_media: true,
        media_count: 2,
        user_id: Some("42".into()),
        username: Some("alice".into()),
        state: state.into(),
        skip_reason: None,
    }
}
fn insert_alice_batch(database: &Database, id: &str) {
    database
            .create_account_batch(CreateAccountBatch {
                id,
                username: "alice",
                profile_url: "https://x.com/alice",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &BatchOutputSettings::default(),
                downloader_arguments_json: r#"{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}"#,
            })
            .expect("batch");
}
const BOT_A: &str = "bot-identity-a";
const BOT_B: &str = "bot-identity-b";
const OUTBOX_NOW: &str = "2026-10-01T00:00:00Z";
const OUTBOX_LEASE: &str = "2026-10-01T00:05:00Z";
const OUTBOX_LATER: &str = "2026-10-01T01:00:00Z";
const OUTBOX_EXPIRED: &str = "2026-10-01T00:06:00Z";
fn outbox_entry(bot: &str, key: &str, plan_order: i64) -> NewOutboxEntry {
    NewOutboxEntry {
        bot_identity: bot.to_owned(),
        target_scope: "-1001:none".to_owned(),
        chat_id: "-1001".to_owned(),
        message_thread_id: None,
        idempotency_key: key.to_owned(),
        request_fingerprint: format!("fp-{key}"),
        message_kind: "media".to_owned(),
        config_version: 3,
        plan_version: 1,
        plan_order,
        tweet_id: None,
        media_reference: Some(format!("media/{key}.jpg")),
        content_sha256: Some(format!("sha-{key}")),
        payload_schema_version: 1,
        payload_json: format!("{{\"kind\":\"test\",\"key\":\"{key}\"}}"),
        created_at: OUTBOX_NOW.to_owned(),
    }
}
fn cache_key(bot: &str, kind: MediaKind, version: u32) -> FileCacheKey {
    FileCacheKey {
        bot_identity: bot.to_owned(),
        content_sha256: "sha-1".to_owned(),
        media_kind: kind,
        representation_version: version,
    }
}

fn test_candidate_metadata(
    tweet_id: &str,
    media: Vec<xarchive_core::ArchiveMedia>,
) -> ArchiveMetadata {
    ArchiveMetadata {
        schema_version: 1,
        tweet_id: tweet_id.into(),
        url: format!("https://x.com/a/status/{tweet_id}"),
        tweet_type: "post".into(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: None,
            display_name: None,
        },
        created_at: None,
        text: "recovery candidate".into(),
        media,
        archived_at: "now".into(),
        reply_to: None,
        quoted_tweet: None,
    }
}

fn manifest(job_id: &str, attempt_count: u32) -> ArchiveRecoveryManifest {
    ArchiveRecoveryManifest {
        schema_version: 1,
        job_id: job_id.into(),
        attempt_count,
        tweet_row_id: 1,
        tweet_id: "123".into(),
        archive_directory: "Tweets/123".into(),
        media: vec![ArchiveRecoveryMediaFact {
            media_index: 1,
            relative_path: "media/01.jpg".into(),
            media_id: Some("media-1".into()),
            media_type: "photo".into(),
            mime_type: Some("image/jpeg".into()),
            size_bytes: 3,
            sha256: "a".repeat(64),
        }],
        telegram_tweet_row_id: None,
    }
}
