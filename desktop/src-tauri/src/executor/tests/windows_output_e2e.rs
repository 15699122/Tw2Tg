//! Windows output acceptance E2E tests (W4):
//! - D1 four export combinations (`export_json` x `export_yaml`) via v4 frozen spec
//! - Export-disabled (`false`/`false`) `InternalV1` startup recovery
//! - Long archive path (>260 chars) publication under contained Windows production runner

use super::*;
use std::path::PathBuf;

use super::windows_containment_e2e::{windows_desktop_keeper_exe, windows_python_exe};

fn run_windows_v4_export_case(
    root: PathBuf,
    archive_root: PathBuf,
    tweet_id: &str,
    export_json: bool,
    export_yaml: bool,
) {
    let keeper_exe = windows_desktop_keeper_exe();
    let python_exe = windows_python_exe();
    let config_dir = root.join("config");
    let database_path = config_dir.join("archive.sqlite3");
    let staging_root = config_dir.join("_staging");
    std::fs::create_dir_all(&archive_root).unwrap();
    std::fs::create_dir_all(&staging_root).unwrap();

    let script = format!(
        concat!(
            "import sys,json,pathlib\n",
            "c=json.loads(sys.stdin.readline())\n",
            "print(json.dumps({{'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}}),flush=True)\n",
            "c=json.loads(sys.stdin.readline())\n",
            "p=pathlib.Path(c['staging_dir']); p.mkdir(parents=True,exist_ok=True); (p/'media.jpg').write_bytes(b'fixture-media')\n",
            "r={{'tweet_id':{tweet_id:?},'url':c['url'],'tweet_type':'post','media':[]}}\n",
            "d={{'tweet_id':{tweet_id:?},'url':c['url'],'media':[{{'index':1,'media_type':'photo','filename':'media.jpg','relative_path':'media.jpg','size_bytes':13}}],'result':r}}\n",
            "print(json.dumps({{'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':d,'result':r}}),flush=True)\n",
            "sys.stdin.read()\n"
        ),
        tweet_id = tweet_id
    );

    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: archive_root.clone(),
        staging_root: staging_root.clone(),
        database_path: database_path.clone(),
        sidecar_program: Some(python_exe.to_string_lossy().into_owned()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    };
    let factory =
        ProductionExecutionFactory::new(config.clone()).with_windows_keeper_executable(keeper_exe);
    let runtime = ExecutorRuntime::with_production_factory(config, factory);

    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_metadata_settings(
            &archive_request(tweet_id),
            &format!("win-d1-exports-{tweet_id}"),
            MetadataOutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                export_json,
                export_yaml,
            },
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let job_id = prepared.job_id.clone();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    runtime
        .service()
        .submit_persisted(&mut persistence, prepared)
        .unwrap();
    let _ = runtime.recover_startup().unwrap().join().unwrap();
    wait_for_persisted_state(&database_path, &job_id, JobState::Complete);

    let final_dir = archive_root.join("Tweets").join(tweet_id);
    let canonical_final_dir = std::fs::canonicalize(&final_dir).unwrap_or_else(|error| {
        panic!(
            "{tweet_id}: final archive dir {} must exist: {error}",
            final_dir.display()
        )
    });
    let media_file = canonical_final_dir.join(format!("{tweet_id}_01.jpg"));
    assert!(
        media_file.is_file(),
        "{tweet_id}: media must be published at {}",
        media_file.display()
    );
    let json_file = canonical_final_dir.join("tweet.json");
    let yaml_file = canonical_final_dir.join("tweet.yaml");
    let txt_file = canonical_final_dir.join("tweet.txt");
    assert_eq!(
        json_file.exists(),
        export_json,
        "{tweet_id}: tweet.json must match frozen export_json={export_json}"
    );
    assert_eq!(
        yaml_file.exists(),
        export_yaml,
        "{tweet_id}: tweet.yaml must match frozen export_yaml={export_yaml}"
    );
    assert!(
        !txt_file.exists(),
        "{tweet_id}: a v4 spec must never emit legacy tweet.txt"
    );

    let database = Database::open(&database_path).unwrap();
    assert_eq!(database.job_state(&job_id).unwrap(), JobState::Complete);
    assert_eq!(
        database
            .archive_recovery_journal(&job_id)
            .unwrap()
            .unwrap()
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Committed,
        "{tweet_id}: recovery journal must converge to Committed"
    );
    let record_root = crate::executor::containment_record_root(&staging_root).unwrap();
    let remaining_records = if record_root.exists() {
        std::fs::read_dir(&record_root).unwrap().count()
    } else {
        0
    };
    assert_eq!(
        remaining_records, 0,
        "{tweet_id}: contained runner must clean up drained containment records"
    );

    drop(database);
    drop(persistence);
    drop(runtime);
}

#[test]
fn windows_production_runner_writes_exactly_the_frozen_v4_export_selection() {
    for (tweet_id, export_json, export_yaml) in [
        ("12192", false, false),
        ("12193", true, false),
        ("12194", false, true),
        ("12195", true, true),
    ] {
        let root =
            std::env::temp_dir().join(format!("tw2tg-win-w4-d1-{tweet_id}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let archive_root = root.join("archive");
        run_windows_v4_export_case(
            root.clone(),
            archive_root,
            tweet_id,
            export_json,
            export_yaml,
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}

#[test]
fn windows_internal_recovery_succeeds_with_exports_disabled() {
    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-w4-no-exports-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let config_dir = root.join("config");
    let database_path = config_dir.join("archive.sqlite3");
    let staging_root = config_dir.join("_staging");
    std::fs::create_dir_all(&config_dir).unwrap();

    let tweet_id = "12197";
    let mut database = Database::open(&database_path).unwrap();
    let tweet_row_id = database
        .insert_tweet(
            tweet_id,
            &format!("https://x.com/test/status/{tweet_id}"),
            "post",
            "",
            "now",
        )
        .unwrap();
    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_metadata_settings(
            &archive_request(tweet_id),
            "win-w4-no-exports-req",
            MetadataOutputSettings {
                naming_mode: NamingMode::Original,
                filename_template: String::new(),
                export_json: false,
                export_yaml: false,
            },
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let job_id = prepared.job_id.as_str();
    database
        .create_archive_job_with_optional_recovery_contract(
            job_id,
            tweet_row_id,
            prepared.schema_version,
            &prepared.request_id,
            &prepared.request_json,
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .unwrap();
    database.begin_archive_attempt(job_id, "now").unwrap();
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        database.transition_job(job_id, state, "now").unwrap();
    }

    let files = FileStore::with_staging_root(&root, &staging_root).unwrap();
    let staging = files.create_attempt_staging_dir(job_id, 1).unwrap();
    std::fs::write(staging.join("01.jpg"), b"abc").unwrap();
    let sha256 = FileStore::sha256(staging.join("01.jpg")).unwrap();
    let final_directory = format!("Tweets/{tweet_id}");
    let metadata = xarchive_core::ArchiveMetadata {
        schema_version: 1,
        tweet_id: tweet_id.to_owned(),
        url: format!("https://x.com/test/status/{tweet_id}"),
        tweet_type: "post".to_owned(),
        author: xarchive_core::ArchiveAuthor {
            user_id: None,
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
        },
        created_at: None,
        text: "exports disabled recovery".to_owned(),
        media: vec![xarchive_core::ArchiveMedia {
            index: 1,
            media_id: Some("m1".to_owned()),
            media_type: "photo".to_owned(),
            file: "01.jpg".to_owned(),
            mime_type: Some("image/jpeg".to_owned()),
            size_bytes: 3,
            sha256,
        }],
        archived_at: "now".to_owned(),
        reply_to: None,
        quoted_tweet: None,
    };
    let manifest = xarchive_storage::ArchiveRecoveryManifest::from_metadata(
        job_id,
        1,
        tweet_row_id,
        Path::new(&final_directory),
        &metadata,
        None,
    )
    .unwrap();
    std::fs::write(
        staging.join(".xarchive-recovery.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    database
        .create_archive_recovery_manifest(&manifest, "now")
        .unwrap();
    database
        .transition_job(job_id, JobState::Downloaded, "now")
        .unwrap();
    drop(database);

    let runtime = ExecutorRuntime::new(database_path.clone());
    let recovery = runtime.recover_startup().unwrap().join().unwrap();
    assert_eq!(
        recovery.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.to_owned(),
            status: StartupRecoveryStatus::Recovered,
        }],
        "refusals: {:?}",
        recovery.refusals
    );
    let final_dir = root.join(format!("Tweets/{tweet_id}"));
    assert!(final_dir.join("01.jpg").is_file());
    assert!(!final_dir.join("tweet.json").exists());
    assert!(!final_dir.join("tweet.yaml").exists());
    assert!(!final_dir.join("tweet.txt").exists());
    let reopened = Database::open(&database_path).unwrap();
    assert_eq!(reopened.job_state(job_id).unwrap(), JobState::Complete);
    assert_eq!(
        reopened
            .archive_recovery_journal(job_id)
            .unwrap()
            .unwrap()
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Committed
    );
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn windows_production_runner_publishes_under_long_archive_path() {
    let root = std::env::temp_dir().join(format!(
        "tw2tg-win-w4-longpath-{}-{}",
        std::process::id(),
        crate::runtime::timestamp_marker()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let mut archive_root = root.join("archive");
    for idx in 0..6 {
        archive_root = archive_root.join(format!("w4_long_archive_segment_{idx}_1234567890123456"));
    }
    let tweet_id = "12196";
    let expected_media = archive_root.join(format!("Tweets/{tweet_id}/{tweet_id}_01.jpg"));
    assert!(
        expected_media.as_os_str().len() > 260,
        "expected long path > 260 chars, got {}",
        expected_media.as_os_str().len()
    );
    run_windows_v4_export_case(root.clone(), archive_root, tweet_id, true, true);
    std::fs::remove_dir_all(&root).unwrap();
}
