//! Containment, real-tool publication and the internal-v1 publication boundary.

use super::*;

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn production_factory_contained_handshake_and_drop_remove_writer_group() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let database_path = temporary_database_path("contained-factory");
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let root = database_path.parent().unwrap().parent().unwrap();
    let marker = root.join("membership");
    let database = Database::open(&database_path).unwrap();
    let tweet = database
        .insert_tweet(
            "12090",
            "https://x.com/alice/status/12090",
            "post",
            "",
            "now",
        )
        .unwrap();
    let json = serde_json::to_string(&ExecutionSpecV2 {
        request: archive_request("12090"),
        output_settings: OutputSettings::default(),
    })
    .unwrap();
    database
        .create_archive_job_with_optional_recovery_contract(
            "contained-job",
            tweet,
            2,
            "contained-request",
            &json,
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .unwrap();
    let script = "import pathlib,sys,json; pathlib.Path(sys.argv[1]).write_text(pathlib.Path('/proc/self/cgroup').read_text()); c=json.loads(sys.stdin.readline()); print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True); sys.stdin.read()";
    let factory = ProductionExecutionFactory::new(ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec![
            "-u".into(),
            "-c".into(),
            script.into(),
            marker.to_string_lossy().into_owned(),
        ],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    })
    .with_linux_containment(LinuxExecutionContainment {
        delegated_parent: parent.clone(),
        python: PathBuf::from("/usr/bin/python3"),
        launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
    });
    let execution = factory
        .create_for_attempt(
            &AttemptIdentity::new("contained-job", 1).unwrap(),
            &JobSnapshot {
                job_id: "contained-job".into(),
                tweet_id: "12090".into(),
                state: JobState::Downloading,
            },
        )
        .unwrap();
    let group_membership = std::fs::read_to_string(&marker).unwrap();
    let group_relative = group_membership.trim().strip_prefix("0::").unwrap();
    assert!(group_relative.starts_with(&format!("{relative}/tw2tg-att-")));
    let group = Path::new("/sys/fs/cgroup").join(group_relative.trim_start_matches('/'));
    assert!(group.exists());
    drop(execution);
    assert!(!group.exists());
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}

/// The uncontained production spawn path must retain a *real* failed-start
/// owner when cleanup genuinely cannot complete, and the outcome boundary
/// must surface the abort instead of dropping it. The fixture forks a
/// setsid descendant that keeps both pipes open, so every shutdown attempt
/// inside the startup handshake fails while the direct child is already
/// gone — no cgroup delegation required.
#[cfg(target_os = "linux")]
#[test]
fn production_factory_retains_a_real_stuck_cleanup_owner_and_resolve_aborts() {
    let database_path = temporary_database_path("stuck-cleanup");
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    let database = Database::open(&database_path).unwrap();
    let tweet = database
        .insert_tweet(
            "12095",
            "https://x.com/alice/status/12095",
            "post",
            "",
            "now",
        )
        .unwrap();
    let json = serde_json::to_string(&ExecutionSpecV2 {
        request: archive_request("12095"),
        output_settings: OutputSettings::default(),
    })
    .unwrap();
    database
        .create_archive_job_with_optional_recovery_contract(
            "stuck-cleanup-job",
            tweet,
            2,
            "stuck-cleanup-request",
            &json,
            Some(xarchive_storage::ArchiveRecoveryContract::InternalV1),
            "now",
        )
        .unwrap();
    // A setsid descendant inherits stdout/stderr and survives the process
    // group kill; the invalid protocol line fails the handshake fast.
    let script = "import subprocess,sys; subprocess.Popen([sys.executable,'-c','import time; time.sleep(8)'],start_new_session=True); print('this is not a v2 protocol line',flush=True)";
    let factory = ProductionExecutionFactory::new(ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script.into()],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    });
    let error = match factory.create_for_attempt(
        &AttemptIdentity::new("stuck-cleanup-job", 1).unwrap(),
        &JobSnapshot {
            job_id: "stuck-cleanup-job".into(),
            tweet_id: "12095".into(),
            state: JobState::Downloading,
        },
    ) {
        Err(error) => error,
        Ok(_) => panic!("stuck pipe cleanup must retain a pending owner"),
    };
    let owner = match error {
        AttemptCreationError::StartupCleanupPending(owner) => owner,
        other => panic!("expected a retained startup-cleanup owner, got {other:?}"),
    };
    assert!(
        matches!(
            owner.startup_error(),
            xarchive_sidecar_supervisor::SupervisorError::Send(_)
                | xarchive_sidecar_supervisor::SupervisorError::HandshakeFailed(_)
                | xarchive_sidecar_supervisor::SupervisorError::ShutdownIncomplete(_)
                | xarchive_sidecar_supervisor::SupervisorError::HandshakeTimeout
        ),
        "unexpected real startup error: {:?}",
        owner.startup_error()
    );
    // The outcome boundary retries the same owner; the descendant still
    // holds both pipes, so the retry must fail into the abort report
    // instead of silently dropping the owner.
    let outcome = SessionRunOutcome::StartupCleanupPending {
        job_id: "stuck-cleanup-job".to_owned(),
        decision: Box::new(Ok(SessionRunOutcome::Finished(JobSnapshot {
            job_id: "stuck-cleanup-job".into(),
            tweet_id: "12095".into(),
            state: JobState::Failed,
        }))),
        owner,
    };
    let error = resolve_session_run(outcome)
        .expect_err("a genuinely stuck cleanup must abort with its owner reported");
    assert!(matches!(
        error,
        ExecutorError::Execution { error_code, .. }
            if error_code == "SIDECAR_STARTUP_CLEANUP_INCOMPLETE"
    ));
    drop(database);
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn production_real_tools_publish_local_media() {
    use std::time::Duration;
    let gallery = std::env::var("TW2TG_TEST_GALLERY").expect("real gallery-dl");
    let media_url = std::env::var("TW2TG_TEST_MEDIA_URL").expect("local fixture URL");
    assert!(media_url.starts_with("http://127.0.0.1:"));
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let mut engine_metadata = Vec::new();
    for use_aria2 in [false, true] {
        let aria2 = if use_aria2 {
            Some(std::env::var("TW2TG_TEST_ARIA2").expect("real aria2 for aria2 engine"))
        } else {
            None
        };
        for d1 in [false, true] {
            let database_path = temporary_database_path(if use_aria2 {
                "real-publish-aria2"
            } else {
                "real-publish-gallery"
            });
            std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
            let root = database_path.parent().unwrap().parent().unwrap();
            // Fixture controls extraction and redirects only the network input
            // to loopback; real DownloadRunner and gallery-dl perform download,
            // info-json parsing and file scan. No fake tool writes media bytes.
            let script = format!(
                r#"import sys,json
sys.path.insert(0,{source:?})
from dataclasses import replace
from xarchive_downloader import worker_v2 as w
from xarchive_downloader.extraction import DownloadRunner,ExtractionConfig
real=DownloadRunner.run
def local(self,url,staging_dir,**kwargs):
 self.config=replace(self.config,executable_args=('-o','extractor.directlink.keywords='+json.dumps({{'tweet_id':12091,'num':1,'tweet_url':'https://x.com/fixture/status/12091','text':'real-tool text','media':[{{'index':1,'media_id':'local-media','type':'photo','url':{media_url:?},'filename':'01.jpg','mime_type':'image/jpeg'}}]}})))
 self._runner.config=self.config
 return real(self,{media_url:?},staging_dir,**kwargs)
DownloadRunner.run=local
class Extraction:
 def __init__(self,*a,**kw): pass
 def run(self,url,*a,**kw):
  from xarchive_downloader.protocol_v2 import ExtractionResult,ExtractionMediaItem
  return ExtractionResult(tweet_id='12091',url='https://x.com/fixture/status/12091',tweet_type='post',text='real-tool text',media=(ExtractionMediaItem(index=1,media_id='local-media',media_type='photo',url={media_url:?},filename='01.jpg',mime_type='image/jpeg'),))
w.ExtractionRunner=Extraction
w.run_v2_worker(gallery_dl_executable={gallery:?},proxy_mode='direct',timeout_seconds=15)
"#,
                source = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../sidecar/src")
                    .to_str()
                    .unwrap()
            );
            let runtime = ExecutorRuntime::with_linux_containment(
                ExecutorConfig {
                    telegram_config_file: None,
                    telegram: crate::config::TelegramConfig::default(),
                    archive_root: root.join("archive"),
                    staging_root: root.join("staging"),
                    database_path: database_path.clone(),
                    sidecar_program: Some("/usr/bin/python3".into()),
                    sidecar_args: vec!["-u".into(), "-c".into(), script],
                    gallery_dl_program: Some(gallery.clone()),
                    aria2_program: aria2.clone(),
                    use_aria2,
                    network: ExecutorNetworkConfig {
                        proxy_mode: ProxyMode::Direct,
                        transfer_timeout: Duration::from_secs(15),
                        aria2_connect_timeout: Duration::from_secs(3),
                        aria2_idle_timeout: Duration::from_secs(5),
                        aria2_max_tries: 1,
                        ..Default::default()
                    },
                },
                LinuxExecutionContainment {
                    delegated_parent: parent.clone(),
                    python: PathBuf::from("/usr/bin/python3"),
                    launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
                },
            );
            let settings = OutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                ..OutputSettings::default()
            };
            let preview = xarchive_storage::render_media_filenames(
                &["01.jpg".into()],
                None,
                "12091",
                &settings,
            )
            .unwrap();
            let prepared = if d1 {
                ArchiveJobSubmissionAdapter
                    .prepare_with_metadata_settings(
                        &archive_request("12091"),
                        "real-publish",
                        MetadataOutputSettings {
                            naming_mode: settings.naming_mode,
                            filename_template: settings.filename_template.clone(),
                            ..Default::default()
                        },
                        DownloaderArgumentSnapshot::historical(),
                    )
                    .unwrap()
            } else {
                ArchiveJobSubmissionAdapter
                    .prepare_with_downloader_settings(
                        &archive_request("12091"),
                        "real-publish",
                        settings,
                        DownloaderArgumentSnapshot::historical(),
                    )
                    .unwrap()
            };
            let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
            let job_id = prepared.job_id.clone();
            runtime
                .service()
                .submit_persisted(&mut persistence, prepared)
                .unwrap();
            assert_eq!(
                runtime.recover_startup().unwrap().join().unwrap().outcomes,
                vec![StartupRecoveryOutcome {
                    job_id: job_id.clone(),
                    status: StartupRecoveryStatus::Recovered
                }]
            );
            wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
            let final_dir = root.join("archive/Tweets/12091");
            assert_eq!(
                std::fs::read(final_dir.join(&preview[0])).unwrap(),
                b"real-tool-media"
            );
            let metadata: serde_json::Value =
                serde_json::from_slice(&std::fs::read(final_dir.join("tweet.json")).unwrap())
                    .unwrap();
            assert_eq!(metadata["media"][0]["file"], preview[0]);
            if d1 {
                let yaml: serde_json::Value =
                    serde_yaml::from_slice(&std::fs::read(final_dir.join("tweet.yaml")).unwrap())
                        .unwrap();
                assert_eq!(yaml, metadata);
                assert!(!final_dir.join("tweet.txt").exists());
            } else {
                assert!(final_dir.join("tweet.txt").is_file());
                assert!(!final_dir.join("tweet.yaml").exists());
            }
            let mut comparable = metadata;
            // Separate jobs have different publication times by definition.
            assert!(!comparable["archived_at"].as_str().unwrap().is_empty());
            comparable.as_object_mut().unwrap().remove("archived_at");
            engine_metadata.push(comparable);
            let database = Database::open(&database_path).unwrap();
            assert_eq!(
                database
                    .tweet_archive_facts("12091")
                    .unwrap()
                    .unwrap()
                    .media[0]
                    .relative_path,
                preview[0]
            );
            assert_eq!(
                database
                    .archive_recovery_manifest_v2(&job_id)
                    .unwrap()
                    .unwrap()
                    .phase,
                xarchive_storage::ArchiveRecoveryPhase::Committed
            );
            drop(database);
            drop(runtime);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    for metadata in &engine_metadata[1..] {
        assert_eq!(
            &engine_metadata[0], metadata,
            "both engines and snapshot versions must publish the same business metadata"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires explicitly delegated disposable cgroup and real aria2"]
fn production_real_aria2_drains_recorded_writer() {
    let program = std::env::var("TW2TG_TEST_ARIA2").expect("explicit real aria2 executable");
    let url = std::env::var("TW2TG_TEST_MEDIA_URL").expect("explicit local media URL");
    assert!(url.starts_with("http://127.0.0.1:"));
    let database_path = temporary_database_path("real-aria2");
    let root = database_path.parent().unwrap().parent().unwrap();
    std::fs::create_dir_all(root).unwrap();
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let coordinator = LinuxCoordinator::new(root.join("session-locks")).unwrap();
    let session_attempt = xarchive_workflow::AttemptIdentity::new("real-transfer", 1).unwrap();
    let session = ExecutionReservation::acquire(
        &coordinator,
        "real-transfer",
        &root.join("archive/Tweets/real-transfer"),
    )
    .unwrap()
    .claim(|| xarchive_workflow::ClaimOutcome::Claimed(session_attempt))
    .unwrap();
    let publication_owner = xarchive_workflow::PublicationOwner::for_session(&session);
    let domain = AttemptWriterDomain {
        registration: Some(publication_owner.registration()),
        provisioning: LinuxExecutionContainment {
            delegated_parent: parent.clone(),
            python: PathBuf::from("/usr/bin/python3"),
            launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        },
        record_root: root.join("records"),
        identity: xarchive_workflow::linux_containment::AttemptIdentity::new(
            root.to_str().unwrap(),
            "real-transfer",
            1,
        )
        .unwrap(),
    };
    let mut sidecar_owner = recover_and_create_writer_group(&domain, "sidecar").unwrap();
    sidecar_owner
        .spawn_silent_with_environment(
            &domain.provisioning.python,
            &domain.provisioning.launcher,
            Path::new("/bin/true"),
            &[],
            &[],
            &[],
        )
        .unwrap();
    sidecar_owner
        .stop_and_drain(std::time::Duration::from_secs(5))
        .unwrap();
    sidecar_owner.remove_group().unwrap();
    let plan = xarchive_download::MediaTransferPlan {
        items: vec![xarchive_download::TransferPlanItem {
            media_id: "real-media-1".into(),
            url,
            filename: "media.jpg".into(),
            directory: root.to_string_lossy().into_owned(),
            headers: vec![],
            media_type: "photo".into(),
            mime_type: None,
        }],
    };
    crate::production::assert_real_transfer(&plan, &program, &domain);
    assert_eq!(
        std::fs::read(root.join("media.jpg")).unwrap(),
        b"real-tool-media"
    );
    publication_owner.close_admission().unwrap();
    let permit = publication_owner.authorize(&session).unwrap();
    assert_eq!(permit.attempt().attempt(), 1);
    drop(permit);
    drop(session);
    drop(coordinator);
    assert!(
        !std::fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("tw2tg-att-"))
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires explicitly delegated disposable cgroup and real aria2"]
fn production_real_aria2_transfer_and_refresh_drain_recorded_writer() {
    use std::io::{Read, Write};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let database_path = temporary_database_path("transfer-rpc");
    let root = database_path.parent().unwrap().parent().unwrap();
    std::fs::create_dir_all(root).unwrap();
    let program = std::env::var("TW2TG_TEST_ARIA2").expect("explicit real aria2 executable path");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let fixture_port = listener.local_addr().unwrap().port();
    let fixture_stop = Arc::new(AtomicBool::new(false));
    let fixture_stop_thread = fixture_stop.clone();
    let fixture_server = std::thread::spawn(move || {
        while !fixture_stop_thread.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 2048];
                    let size = stream.read(&mut request).unwrap_or(0);
                    let request = String::from_utf8_lossy(&request[..size]);
                    let expired = request.starts_with("GET /expired ");
                    let (status, body) = if expired {
                        (403, &b"expired"[..])
                    } else {
                        (200, &b"fixture-transfer"[..])
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    ).unwrap();
                    stream.write_all(body).unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => panic!("fixture server accept failed: {error}"),
            }
        }
    });
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let domain = AttemptWriterDomain {
        registration: None,
        provisioning: LinuxExecutionContainment {
            delegated_parent: parent.clone(),
            python: PathBuf::from("/usr/bin/python3"),
            launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        },
        record_root: root.join("records"),
        identity: xarchive_workflow::linux_containment::AttemptIdentity::new(
            root.to_str().unwrap(),
            "transfer-job",
            1,
        )
        .unwrap(),
    };
    let plan = xarchive_download::MediaTransferPlan {
        items: vec![xarchive_download::TransferPlanItem {
            media_id: "media-1".into(),
            url: format!("http://127.0.0.1:{fixture_port}/fixture"),
            filename: "media.jpg".into(),
            directory: root.to_string_lossy().into_owned(),
            headers: vec![],
            media_type: "photo".into(),
            mime_type: None,
        }],
    };
    crate::production::assert_transfer_fixture_success(
        &plan,
        &program,
        &domain,
        &format!("http://127.0.0.1:{}", fixture_port),
    );
    assert_eq!(
        std::fs::read(root.join("media.jpg")).unwrap(),
        b"fixture-transfer"
    );
    assert!(
        !std::fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("tw2tg-att-"))
    );
    std::thread::sleep(std::time::Duration::from_secs(3));
    assert!(
        !std::fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".late")),
        "setsid descendants must be drained before their write deadline"
    );
    fixture_stop.store(true, Ordering::Release);
    fixture_server.join().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "test-only v2 acceptance is deferred; production v2 stays fail-closed until C1"]
fn test_only_v2_scheduler_fixture_publishes_and_recovers_protocol_bytes() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let database_path = temporary_database_path("production-publish");
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let descriptor_path = database_path.with_extension("launch.json");
    let launcher_copy = database_path.with_extension("launcher.py");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        &launcher_copy,
    )
    .unwrap();
    std::fs::write(
        &descriptor_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "delegated_parent": Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/')),
            "python": "/usr/bin/python3",
            "launcher": launcher_copy,
        }))
        .unwrap(),
    )
    .unwrap();
    let provisioning = LinuxExecutionContainment::from_launch_arguments([
        std::ffi::OsString::from("--linux-execution-descriptor"),
        descriptor_path.clone().into_os_string(),
    ])
    .unwrap()
    .unwrap();
    let runtime_root = std::env::temp_dir().join(format!("tw2tg-rt-{}", std::process::id()));
    let mut state = crate::runtime::RuntimeState::initialize_with_containment(
        runtime_root.clone(),
        Some(provisioning.clone()),
    );
    assert_eq!(
        state.executor.linux_containment().unwrap().delegated_parent,
        provisioning.delegated_parent
    );
    let replacement = state.executor.config().clone();
    state.replace_executor(replacement).unwrap();
    assert_eq!(
        state.executor.linux_containment().unwrap().launcher,
        provisioning.launcher
    );
    // Revalidation must refuse before closing the old executor or transport.
    let original_config = state.executor.config().clone();
    let launcher_bytes = std::fs::read(&launcher_copy).unwrap();
    std::fs::remove_file(&launcher_copy).unwrap();
    let mut rejected_config = original_config.clone();
    rejected_config.archive_root = runtime_root.join("must-not-be-installed");
    assert!(state.replace_executor(rejected_config).is_err());
    assert_eq!(
        state.executor.config().archive_root,
        original_config.archive_root
    );
    assert!(state.executor.recover_startup().unwrap().join().is_ok());
    std::fs::write(&launcher_copy, launcher_bytes).unwrap();
    state.replace_executor(original_config).unwrap();
    drop(state);
    std::fs::remove_dir_all(runtime_root).unwrap();
    assert!(
        LinuxExecutionContainment::from_launch_arguments([
            std::ffi::OsString::from("--linux-execution-descriptor"),
            descriptor_path.clone().into_os_string(),
            std::ffi::OsString::from("--linux-execution-descriptor"),
            descriptor_path.into_os_string(),
        ])
        .is_err()
    );
    let root = database_path.parent().unwrap().parent().unwrap();
    let script = r#"import sys,json,pathlib
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
c=json.loads(sys.stdin.readline())
p=pathlib.Path(c['staging_dir']); p.mkdir(parents=True,exist_ok=True); (p/'media.jpg').write_bytes(b'fixture-media')
r={'tweet_id':'12091','url':c['url'],'tweet_type':'post','media':[]}
d={'tweet_id':'12091','url':c['url'],'media':[{'index':1,'media_type':'photo','filename':'media.jpg','relative_path':'media.jpg','size_bytes':13}],'result':r}
print(json.dumps({'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':d,'result':r}),flush=True)
sys.stdin.read()
"#;
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script.into()],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    };
    let runtime = ExecutorRuntime::with_linux_containment(config.clone(), provisioning.clone());
    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "production-publish-job",
            OutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                ..OutputSettings::default()
            },
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    let job_id = prepared.job_id.clone();
    let mut v2_service = runtime.service();
    v2_service.v2_enrollment = true;
    v2_service
        .submit_persisted(&mut persistence, prepared)
        .unwrap();
    // Simulate process exit after durable test-only v2 enrollment but before
    // scheduling. The explicit fixture flag exercises recovery without
    // enabling the production runtime's v2 gate.
    assert_eq!(persistence.current_attempt(&job_id).unwrap(), 0);
    let report = runtime
        .recover_startup_with_v2(true)
        .unwrap()
        .join()
        .unwrap();
    // Diagnostic only: a refused startup recovery must report its recorded
    // cause instead of only the reason-free `Refused` outcome.
    if !report.refusals().is_empty() {
        panic!(
            "v2 startup recovery refused: {:?}",
            report
                .refusals()
                .iter()
                .map(|refusal| format!("{}: {}", refusal.job_id, refusal.reason))
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        report.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.clone(),
            status: StartupRecoveryStatus::Recovered
        }]
    );
    // A pre-PREPARED interrupted attempt must drain its predecessor and
    // claim a successor, not stay permanently gated on a missing journal.
    let interrupted = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "production-interrupted-job",
            OutputSettings::default(),
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    // Use a separate workspace: same destination in the completed job must
    // never be overwritten by this recovery probe.
    let interrupted_path = root.join("interrupted/config/archive.sqlite3");
    std::fs::create_dir_all(interrupted_path.parent().unwrap()).unwrap();
    let mut interrupted_config = runtime.config().clone();
    interrupted_config.database_path = interrupted_path.clone();
    interrupted_config.archive_root = root.join("interrupted/archive");
    interrupted_config.staging_root = root.join("interrupted/staging");
    let interrupted_runtime = ExecutorRuntime::with_linux_containment(
        interrupted_config,
        runtime.containment.clone().unwrap(),
    );
    let mut interrupted_persistence = StorageJobPersistence::open(&interrupted_path).unwrap();
    let interrupted_id = interrupted.job_id.clone();
    let mut interrupted_v2_service = interrupted_runtime.service();
    interrupted_v2_service.v2_enrollment = true;
    interrupted_v2_service
        .submit_persisted(&mut interrupted_persistence, interrupted)
        .unwrap();
    interrupted_persistence
        .begin_attempt(&interrupted_id)
        .unwrap();
    interrupted_persistence
        .persist_state(&JobSnapshot {
            job_id: interrupted_id.clone(),
            tweet_id: "12091".into(),
            state: JobState::Interrupted,
        })
        .unwrap();
    assert_eq!(
        interrupted_runtime
            .recover_startup_with_v2(true)
            .unwrap()
            .join()
            .unwrap()
            .outcomes,
        vec![StartupRecoveryOutcome {
            job_id: interrupted_id.clone(),
            status: StartupRecoveryStatus::Recovered
        }]
    );
    assert_eq!(
        interrupted_persistence
            .current_attempt(&interrupted_id)
            .unwrap(),
        2
    );
    assert_eq!(
        interrupted_persistence
            .snapshot(&interrupted_id)
            .unwrap()
            .state,
        JobState::Complete
    );
    drop(interrupted_runtime);
    wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
    let tweet = Database::open(&database_path)
        .unwrap()
        .tweet_row_id("12091")
        .unwrap();
    assert_eq!(
        std::fs::read(root.join("archive/Tweets/12091/12091_01.jpg")).unwrap(),
        b"fixture-media"
    );
    let events = persistence.stored_events(&job_id).unwrap();
    for expected in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
        assert_eq!(
            events.iter().filter(|(kind, _)| kind == expected).count(),
            1
        );
    }
    let database = Database::open(&database_path).unwrap();
    assert_eq!(
        database
            .archive_recovery_manifest_v2(&job_id)
            .unwrap()
            .unwrap()
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Committed
    );
    let files = FileStore::with_staging_root(root.join("archive"), root.join("staging")).unwrap();
    let mut archive = xarchive_storage::ArchiveService::new(database, files);
    for _ in 0..2 {
        archive
            .recover_internal_rename_v2(&job_id, tweet, Path::new("Tweets/12091"), "later")
            .unwrap();
    }
    assert_eq!(persistence.stored_events(&job_id).unwrap(), events);
    assert!(
        !std::fs::read_dir(Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/')))
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("tw2tg-att-"))
    );
    drop(archive);
    // Re-open a valid COMMITTED fixture as Downloaded to exercise the
    // current startup gate, without enabling replay in production.
    let database = Database::open(&database_path).unwrap();
    database
        .execute_batch(&format!(
            "UPDATE jobs SET state = 'DOWNLOADED' WHERE id = '{job_id}'"
        ))
        .unwrap();
    // An unprovisioned runtime still preserves the historical rejection.
    runtime
        .recover_startup_with_v2(false)
        .unwrap()
        .join()
        .unwrap();
    wait_for_persisted_state(&database_path, &job_id, JobState::Failed);
    assert_eq!(
        persistence
            .stored_job_summary(&job_id)
            .unwrap()
            .last_error_code
            .as_deref(),
        Some("ARCHIVE_RECOVERY_V2_NOT_READY")
    );
    assert_eq!(
        persistence
            .stored_events(&job_id)
            .unwrap()
            .iter()
            .filter(|(kind, _)| kind == "JOB_COMPLETED")
            .count(),
        1
    );
    // Model interruption after Storage COMMITTED but before the Desktop
    // terminal transaction: that transaction and its events never existed.
    database
        .execute_batch(&format!(
            "DELETE FROM events WHERE job_id = '{job_id}' AND (event_type IN ('JOB_COMPLETED', 'JOB_FAILED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state IN ('COMPLETE','FAILED'))); UPDATE jobs SET state = 'DOWNLOADED', last_error_code = NULL, finished_at = NULL WHERE id = '{job_id}'"
        ))
        .unwrap();
    let attempt = database.archive_attempt(&job_id).unwrap();
    let before_recovery = persistence.stored_events(&job_id).unwrap();
    let mut v2_service = runtime.service();
    v2_service.v2_enrollment = true;
    let peer = ProductionCoordinator::open(&database_path).unwrap();
    let held =
        ExecutionReservation::acquire(&peer, &job_id, &root.join("archive/Tweets/12091")).unwrap();
    assert!(
        !recover_v2_on_runner(
            &v2_service,
            database_path.clone(),
            root.join("archive"),
            root.join("staging"),
            runtime.containment.clone(),
            job_id.clone(),
            "12091".into(),
        )
        .unwrap()
    );
    assert_eq!(persistence.stored_events(&job_id).unwrap(), before_recovery);
    drop(held);
    for replay in 0..2 {
        assert!(
            recover_v2_on_runner(
                &v2_service,
                database_path.clone(),
                root.join("archive"),
                root.join("staging"),
                runtime.containment.clone(),
                job_id.clone(),
                "12091".into(),
            )
            .unwrap()
        );
        if replay == 0 {
            assert_eq!(
                persistence.snapshot(&job_id).unwrap().state,
                JobState::Complete
            );
        }
    }
    assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
    let after_recovery = persistence.stored_events(&job_id).unwrap();
    assert_eq!(
        after_recovery
            .iter()
            .filter(|(kind, _)| kind == "JOB_COMPLETED")
            .count(),
        1
    );
    std::fs::write(
        root.join("archive/Tweets/12091/12091_01.jpg"),
        b"fixture-wrong",
    )
    .unwrap();
    assert!(
        recover_v2_on_runner(
            &v2_service,
            database_path.clone(),
            root.join("archive"),
            root.join("staging"),
            runtime.containment.clone(),
            job_id.clone(),
            "12091".into(),
        )
        .is_err()
    );
    assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
    assert_eq!(persistence.stored_events(&job_id).unwrap(), after_recovery);
    std::fs::write(
        root.join("archive/Tweets/12091/12091_01.jpg"),
        b"fixture-media",
    )
    .unwrap();
    assert!(
        recover_v2_on_runner(
            &v2_service,
            database_path.clone(),
            root.join("archive"),
            root.join("staging"),
            None,
            job_id.clone(),
            "12091".into(),
        )
        .is_err()
    );
    assert_eq!(persistence.stored_events(&job_id).unwrap(), after_recovery);
    // Model the durable directory-commit crash window: bytes already moved,
    // but the journal's finalization transaction has not yet committed.
    // Undo candidate retirement too: PREPARED still owns this exact business
    // metadata, whereas the successful transaction retired its copy.
    let candidate: xarchive_core::ArchiveMetadata = serde_json::from_slice(
        &std::fs::read(root.join("archive/Tweets/12091/tweet.json")).unwrap(),
    )
    .unwrap();
    let candidate_json = serde_json::to_string(&candidate).unwrap();
    let candidate_digest = {
        use sha2::Digest;
        format!("{:x}", sha2::Sha256::digest(candidate_json.as_bytes()))
    };
    database.execute_batch(&format!(
        "INSERT INTO archive_recovery_v2_candidate (job_id, attempt_count, tweet_row_id, metadata_json, metadata_sha256, created_at, updated_at) SELECT '{job_id}', {attempt}, {tweet}, merged_metadata_json, '{candidate_digest}', 'fixture', 'fixture' FROM tweets WHERE id = {tweet}"
    )).unwrap();
    database
        .execute_batch(&format!(
            "DELETE FROM events WHERE job_id = '{job_id}' AND (event_type IN ('JOB_COMPLETED','DOWNLOAD_COMPLETED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state IN ('COMPLETE','DOWNLOADED'))); UPDATE archive_recovery_v2 SET phase = 'PREPARED' WHERE job_id = '{job_id}'; UPDATE jobs SET state = 'DOWNLOADED', finished_at = NULL WHERE id = '{job_id}'"
        ))
        .unwrap();
    let recovery = runtime
        .recover_startup_with_v2(true)
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(
        recovery.outcomes,
        vec![StartupRecoveryOutcome {
            job_id: job_id.clone(),
            status: StartupRecoveryStatus::Recovered
        }]
    );
    for _ in 0..100 {
        if database
            .archive_recovery_manifest_v2(&job_id)
            .unwrap()
            .unwrap()
            .phase
            == xarchive_storage::ArchiveRecoveryPhase::Committed
        {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        database
            .archive_recovery_manifest_v2(&job_id)
            .unwrap()
            .unwrap()
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Committed
    );
    assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
    wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
    // Replay each durable rename phase through startup, not just Storage's
    // helper. Restore the original source/temporary mapping in the attempt
    // staging tree without re-rendering settings during recovery.
    for progress in ["PLANNED", "TEMPORARY", "FINAL"] {
        database.execute_batch(&format!(
            "INSERT INTO archive_recovery_v2_candidate (job_id, attempt_count, tweet_row_id, metadata_json, metadata_sha256, created_at, updated_at) SELECT '{job_id}', {attempt}, {tweet}, merged_metadata_json, '{candidate_digest}', 'fixture', 'fixture' FROM tweets WHERE id = {tweet}"
        )).unwrap();
        let files =
            FileStore::with_staging_root(root.join("archive"), root.join("staging")).unwrap();
        let staging = files.create_attempt_staging_dir(&job_id, attempt).unwrap();
        std::fs::remove_dir(&staging).unwrap();
        std::fs::rename(root.join("archive/Tweets/12091"), &staging).unwrap();
        if progress != "FINAL" {
            let journal = database
                .archive_recovery_manifest_v2(&job_id)
                .unwrap()
                .unwrap();
            let manifest: xarchive_storage::ArchiveRecoveryManifestV2 =
                serde_json::from_str(&journal.manifest_json).unwrap();
            for edge in manifest.rename_plan {
                std::fs::rename(
                    staging.join(edge.final_path),
                    staging.join(if progress == "TEMPORARY" {
                        edge.temporary_path
                    } else {
                        edge.source_path
                    }),
                )
                .unwrap();
            }
        }
        database.execute_batch(&format!(
            "DELETE FROM events WHERE job_id = '{job_id}' AND (event_type IN ('JOB_COMPLETED','DOWNLOAD_COMPLETED') OR (event_type = 'JOB_STATE_CHANGED' AND new_state IN ('COMPLETE','DOWNLOADED'))); UPDATE archive_recovery_v2 SET phase = 'PREPARED', rename_progress = '{progress}' WHERE job_id = '{job_id}'; UPDATE jobs SET state = 'DOWNLOADED', finished_at = NULL WHERE id = '{job_id}'"
        )).unwrap();
        let recovery = runtime
            .recover_startup_with_v2(true)
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            recovery.outcomes,
            vec![StartupRecoveryOutcome {
                job_id: job_id.clone(),
                status: StartupRecoveryStatus::Recovered
            }]
        );
        wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
        assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
        assert_eq!(
            std::fs::read(root.join("archive/Tweets/12091/12091_01.jpg")).unwrap(),
            b"fixture-media"
        );
    }
    let completed_events = persistence.stored_events(&job_id).unwrap();
    for kind in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
        assert_eq!(
            completed_events
                .iter()
                .filter(|(event, _)| event == kind)
                .count(),
            1
        );
    }
    database
        .execute_batch(&format!(
            "UPDATE archive_recovery_v2 SET manifest_json = '{{' WHERE job_id = '{job_id}'"
        ))
        .unwrap();
    assert!(
        recover_v2_on_runner(
            &v2_service,
            database_path.clone(),
            root.join("archive"),
            root.join("staging"),
            runtime.containment.clone(),
            job_id.clone(),
            "12091".into(),
        )
        .is_err()
    );
    assert_eq!(
        persistence.stored_events(&job_id).unwrap(),
        completed_events
    );
    // A published state without a journal cannot start a successor.
    database.execute_batch(&format!(
        "DELETE FROM archive_recovery_v2 WHERE job_id = '{job_id}'; UPDATE jobs SET state = 'DOWNLOADED' WHERE id = '{job_id}'"
    )).unwrap();
    let recovery = runtime
        .recover_startup_with_v2(true)
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(recovery.outcomes.len(), 1);
    assert!(recovery.outcomes[0].status == StartupRecoveryStatus::Refused);
    assert_eq!(database.archive_attempt(&job_id).unwrap(), attempt);
    assert_eq!(
        persistence.snapshot(&job_id).unwrap().state,
        JobState::Downloaded
    );
    assert_eq!(
        persistence.stored_events(&job_id).unwrap(),
        completed_events
    );
    database
        .execute_batch(&format!(
            "UPDATE jobs SET state = 'DOWNLOADED' WHERE id = '{job_id}'"
        ))
        .unwrap();
    let before_rejection = persistence.stored_events(&job_id).unwrap();
    let rejected = runtime
        .recover_startup_with_v2(true)
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(rejected.outcomes.len(), 1);
    assert_eq!(rejected.outcomes[0].job_id, job_id);
    assert!(rejected.outcomes[0].status == StartupRecoveryStatus::Refused);
    assert_eq!(
        persistence.snapshot(&job_id).unwrap().state,
        JobState::Downloaded
    );
    assert_eq!(
        persistence.stored_events(&job_id).unwrap(),
        before_rejection
    );
    database
        .execute_batch(&format!(
            "DELETE FROM archive_recovery_v2 WHERE job_id = '{job_id}'"
        ))
        .unwrap();
    assert!(
        recover_v2_on_runner(
            &runtime.service(),
            database_path.clone(),
            root.join("archive"),
            root.join("staging"),
            runtime.containment.clone(),
            job_id.clone(),
            "12091".into(),
        )
        .is_err()
    );
    assert_eq!(
        persistence.stored_events(&job_id).unwrap(),
        completed_events
    );
    drop(database);
    runtime.executor.shutdown().unwrap();
    drop(persistence);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_factory_carries_the_v4_d1_export_selection_into_archive_execution() {
    // D1 is only meaningful if the frozen v4 selection survives decoding into
    // the execution object that later builds `PendingArchivePublication`.
    let decoded = decode_execution_spec(
        4,
        &serde_json::to_string(&ExecutionSpecV4 {
            request: archive_request("123"),
            output_settings: MetadataOutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                export_json: false,
                export_yaml: true,
            },
            downloader_arguments: DownloaderArgumentSnapshot::historical(),
        })
        .unwrap(),
    )
    .expect("v4 spec");
    let settings = decoded.metadata_output.expect("v4 carries a D1 selection");
    assert!(
        !settings.export_json,
        "json must follow the frozen selection"
    );
    assert!(
        settings.export_yaml,
        "yaml must follow the frozen selection"
    );
    assert_eq!(settings.naming_mode, NamingMode::Template);
    let exports = settings
        .serialize_metadata(&serde_json::json!({"tweet_id": "123"}))
        .expect("serialize");
    assert_eq!(exports.len(), 1, "only yaml was selected");
    assert_eq!(exports[0].0, "tweet.yaml");
}

#[test]
fn production_factory_carries_v2_output_snapshot_into_archive_execution() {
    let decoded = decode_execution_spec(
        2,
        &serde_json::to_string(&ExecutionSpecV2 {
            request: archive_request("123"),
            output_settings: OutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                export_json: false,
                export_text: true,
            },
        })
        .unwrap(),
    )
    .expect("v2 spec");
    assert_eq!(
        decoded.output_settings.filename_template,
        "{tweet_id}_{index}"
    );
    assert!(!decoded.output_settings.export_json);
    assert_eq!(decoded.output_settings.naming_mode, NamingMode::Template);
}

/// D1 acceptance on the **real** production chain, not a decoded spec: each of
/// the four export combinations is persisted as a v4 execution spec, executed by
/// the contained production factory, finalized by
/// `PendingArchivePublication::finalize`, and verified on the files that actually
/// landed in the archive directory.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn production_runner_writes_exactly_the_frozen_v4_export_selection() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let delegated_parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    // Each case uses its own tweet id, database and archive root: the desktop
    // submission adapter derives `request_id` from the tweet id, so reusing one
    // id would collide on the persisted request instead of testing exports.
    for (tweet_id, export_json, export_yaml) in [
        ("12092", false, false),
        ("12093", true, false),
        ("12094", false, true),
        ("12095", true, true),
    ] {
        run_frozen_v4_export_case(&delegated_parent, tweet_id, export_json, export_yaml);
    }
    assert!(
        !std::fs::read_dir(&delegated_parent)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("tw2tg-att-")),
        "the runner must drain every registered writer domain"
    );
}

/// Run one frozen v4 export combination through the contained production chain.
#[cfg(target_os = "linux")]
fn run_frozen_v4_export_case(
    delegated_parent: &Path,
    tweet_id: &str,
    export_json: bool,
    export_yaml: bool,
) {
    let database_path = temporary_database_path(&format!("runner-d1-exports-{tweet_id}"));
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let descriptor_path = database_path.with_extension("launch.json");
    let launcher_copy = database_path.with_extension("launcher.py");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        &launcher_copy,
    )
    .unwrap();
    std::fs::write(
        &descriptor_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "delegated_parent": delegated_parent,
            "python": "/usr/bin/python3",
            "launcher": launcher_copy,
        }))
        .unwrap(),
    )
    .unwrap();
    let provisioning = LinuxExecutionContainment::from_launch_arguments([
        std::ffi::OsString::from("--linux-execution-descriptor"),
        descriptor_path.into_os_string(),
    ])
    .unwrap()
    .unwrap();
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    let script = format!(
        r#"import sys,json,pathlib
c=json.loads(sys.stdin.readline())
print(json.dumps({{'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}}),flush=True)
c=json.loads(sys.stdin.readline())
p=pathlib.Path(c['staging_dir']); p.mkdir(parents=True,exist_ok=True); (p/'media.jpg').write_bytes(b'fixture-media')
r={{'tweet_id':{tweet_id:?},'url':c['url'],'tweet_type':'post','media':[]}}
d={{'tweet_id':{tweet_id:?},'url':c['url'],'media':[{{'index':1,'media_type':'photo','filename':'media.jpg','relative_path':'media.jpg','size_bytes':13}}],'result':r}}
print(json.dumps({{'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':d,'result':r}}),flush=True)
sys.stdin.read()
"#
    );
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    };
    let runtime = ExecutorRuntime::with_linux_containment(config, provisioning);
    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_metadata_settings(
            &archive_request(tweet_id),
            &format!("runner-d1-exports-{tweet_id}"),
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
    let final_dir = root.join(format!("archive/Tweets/{tweet_id}"));
    assert!(
        final_dir.join(format!("{tweet_id}_01.jpg")).is_file(),
        "{tweet_id}: media must be published"
    );
    assert_eq!(
        final_dir.join("tweet.json").exists(),
        export_json,
        "{tweet_id}: tweet.json must follow the frozen selection"
    );
    assert_eq!(
        final_dir.join("tweet.yaml").exists(),
        export_yaml,
        "{tweet_id}: tweet.yaml must follow the frozen selection"
    );
    assert!(
        !final_dir.join("tweet.txt").exists(),
        "{tweet_id}: a v4 spec must never emit the legacy text export"
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
        "{tweet_id}: recovery must converge independently of exports"
    );
    drop(database);
    drop(persistence);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn production_runner_rejects_uncontained_internal_v1_writer_before_publish() {
    let database_path = temporary_database_path("runner-v1-uncontained");
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec![
            "-u".into(),
            "-c".into(),
            r#"import sys,json
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':{'tweet_id':'12091','url':c['url'],'media':[]},'result':{'tweet_id':'12091','url':c['url'],'tweet_type':'post','media':[]}}),flush=True)
sys.stdin.read()"#
                .into(),
        ],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    };
    let factory = ProductionExecutionFactory::new(config.clone());
    let runtime = ExecutorRuntime::with_production_factory(config, factory, None);
    let submission = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "runner-v1-uncontained-job",
            OutputSettings::default(),
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let job_id = submission.job_id.clone();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    runtime
        .service()
        .submit_persisted(&mut persistence, submission)
        .unwrap();

    runtime.recover_startup().unwrap().join().unwrap();
    wait_for_persisted_state(&database_path, &job_id, JobState::Failed);

    let database = Database::open(&database_path).unwrap();
    assert!(
        database
            .archive_recovery_journal(&job_id)
            .unwrap()
            .is_none()
    );
    assert!(!root.join("archive/Tweets/12091").exists());
    let failure = persistence.stored_job_summary(&job_id).unwrap();
    assert_eq!(
        failure.last_error_code.as_deref(),
        Some("EXECUTOR_WORKER_FAILED")
    );
    assert!(
        failure
            .last_error_message
            .as_deref()
            .is_some_and(|message| { message.contains("PUBLICATION_INCOMPLETE_DRAIN") })
    );

    drop(runtime);
    drop(database);
    drop(persistence);
    std::fs::remove_dir_all(root).unwrap();
}

/// V1-03 at the real runner boundary: after the attempt staging tree is
/// committed to its final directory but before archive metadata converges,
/// the publication database write fails. The runner must not report a
/// terminal success, and the durable facts must let startup recovery
/// converge the attempt once without duplicating events.
///
/// The failure is injected with a SQLite trigger, so production entry
/// points, session authorization, and permit ownership are unchanged.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn production_runner_post_commit_database_failure_recovers_without_false_success() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let database_path = temporary_database_path("runner-v1-post-commit-failure");
    let root = database_path.parent().unwrap().parent().unwrap().to_owned();
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let descriptor_path = database_path.with_extension("launch.json");
    let launcher_copy = database_path.with_extension("launcher.py");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        &launcher_copy,
    )
    .unwrap();
    let delegated_parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    std::fs::write(
        &descriptor_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "delegated_parent": delegated_parent,
            "python": "/usr/bin/python3",
            "launcher": launcher_copy,
        }))
        .unwrap(),
    )
    .unwrap();
    let provisioning = LinuxExecutionContainment::from_launch_arguments([
        std::ffi::OsString::from("--linux-execution-descriptor"),
        descriptor_path.into_os_string(),
    ])
    .unwrap()
    .unwrap();
    let script = r#"import sys,json,pathlib
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
c=json.loads(sys.stdin.readline())
p=pathlib.Path(c['staging_dir']); p.mkdir(parents=True,exist_ok=True); (p/'media.jpg').write_bytes(b'fixture-media')
r={'tweet_id':'12091','url':c['url'],'tweet_type':'post','media':[]}
d={'tweet_id':'12091','url':c['url'],'media':[{'index':1,'media_type':'photo','filename':'media.jpg','relative_path':'media.jpg','size_bytes':13}],'result':r}
print(json.dumps({'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':d,'result':r}),flush=True)
sys.stdin.read()
"#;
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script.into()],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    };
    let runtime = ExecutorRuntime::with_linux_containment(config.clone(), provisioning.clone());

    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "runner-v1-post-commit-failure-job",
            OutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                ..OutputSettings::default()
            },
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    let job_id = prepared.job_id.clone();
    runtime
        .service()
        .submit_persisted(&mut persistence, prepared)
        .unwrap();

    // Fail the archive metadata convergence write, which happens after the
    // staging tree has already been renamed into its final directory.
    Database::open(&database_path)
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fail_v1_metadata \
             BEFORE UPDATE OF archive_directory ON tweets \
             BEGIN SELECT RAISE(ABORT, 'injected post-commit metadata failure'); END;",
        )
        .unwrap();

    let report = runtime.recover_startup().unwrap().join().unwrap();
    assert!(
        persistence
            .stored_events(&job_id)
            .unwrap()
            .iter()
            .all(|(kind, _)| kind != "JOB_COMPLETED"),
        "a post-commit database failure must not record a false terminal success"
    );
    assert_ne!(
        persistence.snapshot(&job_id).unwrap().state,
        JobState::Complete,
        "the attempt must not be reported complete while convergence failed: {report:?}"
    );
    let failed_database = Database::open(&database_path).unwrap();
    assert_eq!(
        failed_database
            .archive_recovery_journal(&job_id)
            .unwrap()
            .expect("the PREPARED journal is the durable recovery fact")
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Prepared,
        "the journal must remain PREPARED so replay can converge it"
    );
    assert!(
        root.join("archive/Tweets/12091/12091_01.jpg").is_file(),
        "the file commit happened before the injected database failure"
    );
    drop(failed_database);
    drop(runtime);

    // Recovery with the fault removed. PREPARED replay is now authorized
    // for DOWNLOADING state under the full identity/drain/permit fences, so
    // startup converges the attempt to COMMITTED/COMPLETE exactly once.
    Database::open(&database_path)
        .unwrap()
        .execute_batch("DROP TRIGGER fail_v1_metadata;")
        .unwrap();
    let recovered_runtime =
        ExecutorRuntime::with_linux_containment(config.clone(), provisioning.clone());
    let recovery_report = recovered_runtime.recover_startup().unwrap().join().unwrap();
    let after_recovery = Database::open(&database_path).unwrap();
    let journal_after = after_recovery
        .archive_recovery_journal(&job_id)
        .unwrap()
        .map(|record| record.phase);
    let state_after = after_recovery.job_state(&job_id).unwrap();
    drop(after_recovery);

    // After convergence, exactly one legitimate JOB_COMPLETED is recorded
    // (success is not false once recovery converges); no duplicate in the
    // repeated scan below.
    let completed_after_first = persistence
        .stored_events(&job_id)
        .unwrap()
        .iter()
        .filter(|(kind, _)| kind == "JOB_COMPLETED")
        .count();
    assert!(
        completed_after_first == 1,
        "converged recovery must record exactly one JOB_COMPLETED, not {completed_after_first}"
    );
    assert_eq!(
        state_after,
        JobState::Complete,
        "recovery must converge DOWNLOADING+PREPARED to Complete"
    );
    assert_eq!(
        journal_after,
        Some(xarchive_storage::ArchiveRecoveryPhase::Committed),
        "recovery must converge PREPARED journal to COMMITTED"
    );
    assert!(
        recovery_report
            .outcomes()
            .iter()
            .any(|outcome| outcome.job_id == job_id
                && outcome.status == StartupRecoveryStatus::Recovered),
        "startup must recover DOWNLOADING+PREPARED rather than refuse: {recovery_report:?}"
    );

    // Production-faithful integrity validation: the same committed terminal
    // verification the COMMITTED recovery path uses must accept the
    // converged tree. A damaged archive or a mismatched manifest therefore
    // cannot be reported as "already converged" by a repeated scan.
    let manifest = Database::open(&database_path)
        .unwrap()
        .verify_archive_recovery_manifest(&job_id)
        .expect("the converged journal must carry a valid manifest");
    assert_eq!(
        manifest.attempt_count, 1,
        "the converged manifest must describe attempt 1"
    );
    let final_directory = Path::new("Tweets").join("12091");
    let converged_attempt =
        xarchive_workflow::AttemptIdentity::new(&job_id, manifest.attempt_count).unwrap();
    ArchiveService::new(
        Database::open(&database_path).unwrap(),
        FileStore::with_staging_root(root.join("archive"), root.join("staging")).unwrap(),
    )
    .verify_committed_terminal_internal_v1(
        &job_id,
        manifest.tweet_row_id,
        &final_directory,
        &converged_attempt,
    )
    .expect("a converged committed archive must pass terminal integrity verification");

    // Idempotence: a repeated recovery scan must not re-execute or
    // duplicate completion.
    let again_runtime =
        ExecutorRuntime::with_linux_containment(config.clone(), provisioning.clone());
    let again_report = again_runtime.recover_startup().unwrap().join().unwrap();
    let again_completed = persistence
        .stored_events(&job_id)
        .unwrap()
        .iter()
        .filter(|(kind, _)| kind == "JOB_COMPLETED")
        .count();
    assert_eq!(
        again_completed, completed_after_first,
        "repeated recovery must not duplicate JOB_COMPLETED"
    );
    assert!(
        !again_report
            .outcomes()
            .iter()
            .any(|outcome| outcome.job_id == job_id
                && outcome.status == StartupRecoveryStatus::Recovered),
        "repeated recovery must not execute CONVERGED attempt again: {again_report:?}"
    );
    drop(again_runtime);

    drop(recovered_runtime);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn production_runner_finalizes_internal_v1_archive_after_registered_writer_drain() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let database_path = temporary_database_path("runner-v1-finalize");
    // The test derives its coordinator from this fresh temporary DB root;
    // never inspect, create, or remove the repository's historical lock.
    let test_root = database_path.parent().unwrap().parent().unwrap().to_owned();
    assert!(test_root.starts_with(std::env::temp_dir()));
    assert!(
        !database_path.exists(),
        "temporary fixture DB must be fresh"
    );
    std::fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let descriptor_path = database_path.with_extension("launch.json");
    let launcher_copy = database_path.with_extension("launcher.py");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        &launcher_copy,
    )
    .unwrap();
    let delegated_parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    std::fs::write(
        &descriptor_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "delegated_parent": delegated_parent,
            "python": "/usr/bin/python3",
            "launcher": launcher_copy,
        }))
        .unwrap(),
    )
    .unwrap();
    let provisioning = LinuxExecutionContainment::from_launch_arguments([
        std::ffi::OsString::from("--linux-execution-descriptor"),
        descriptor_path.into_os_string(),
    ])
    .unwrap()
    .unwrap();
    let root = &test_root;
    let script = r#"import sys,json,pathlib
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
c=json.loads(sys.stdin.readline())
p=pathlib.Path(c['staging_dir']); p.mkdir(parents=True,exist_ok=True); (p/'media.jpg').write_bytes(b'fixture-media')
r={'tweet_id':'12091','url':c['url'],'tweet_type':'post','media':[]}
d={'tweet_id':'12091','url':c['url'],'media':[{'index':1,'media_type':'photo','filename':'media.jpg','relative_path':'media.jpg','size_bytes':13}],'result':r}
print(json.dumps({'protocol_version':2,'event':'download_completed','job_id':c['job_id'],'request_id':c['request_id'],'download':d,'result':r}),flush=True)
sys.stdin.read()
"#;
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script.into()],
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            ..Default::default()
        },
    };
    let runtime = ExecutorRuntime::with_linux_containment(config.clone(), provisioning.clone());
    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "runner-v1-finalize-job",
            OutputSettings {
                naming_mode: NamingMode::Template,
                filename_template: "{tweet_id}_{index}".into(),
                ..OutputSettings::default()
            },
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let mut persistence = StorageJobPersistence::open(&database_path).unwrap();
    let job_id = prepared.job_id.clone();
    runtime
        .service()
        .submit_persisted(&mut persistence, prepared)
        .unwrap();
    assert_eq!(
        Database::open(&database_path)
            .unwrap()
            .archive_recovery_contract(&job_id)
            .unwrap(),
        Some(xarchive_storage::ArchiveRecoveryContract::InternalV1.version()),
        "Linux containment does not enroll production v2",
    );
    let _startup_results = runtime.recover_startup().unwrap().join().unwrap();
    wait_for_persisted_state(&database_path, &job_id, JobState::Complete);
    assert_eq!(
        persistence.snapshot(&job_id).unwrap().state,
        JobState::Complete,
    );
    assert_eq!(
        std::fs::read(root.join("archive/Tweets/12091/12091_01.jpg")).unwrap(),
        b"fixture-media",
    );
    let events = persistence.stored_events(&job_id).unwrap();
    for kind in ["DOWNLOAD_COMPLETED", "JOB_COMPLETED"] {
        assert_eq!(events.iter().filter(|(event, _)| event == kind).count(), 1);
    }
    let database = Database::open(&database_path).unwrap();
    assert_eq!(
        database
            .archive_recovery_journal(&job_id)
            .unwrap()
            .unwrap()
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Committed,
    );
    assert!(
        !std::fs::read_dir(&delegated_parent)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("tw2tg-att-")),
        "the runner must retain and drain every registered writer domain"
    );
    drop(runtime);

    // Local supervisor shutdown without a containment-confirmed writer
    // registration is not publication authority. The same production
    // call chain must fail closed before PREPARED or a final directory.
    let uncontained_path = root.join("uncontained/config/archive.sqlite3");
    std::fs::create_dir_all(uncontained_path.parent().unwrap()).unwrap();
    let mut uncontained_config = config.clone();
    uncontained_config.database_path = uncontained_path.clone();
    uncontained_config.archive_root = root.join("uncontained/archive");
    uncontained_config.staging_root = root.join("uncontained/staging");
    let uncontained_factory = ProductionExecutionFactory::new(uncontained_config.clone());
    let uncontained_runtime =
        ExecutorRuntime::with_production_factory(uncontained_config, uncontained_factory, None);
    let uncontained_submission = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "runner-v1-uncontained-job",
            OutputSettings::default(),
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let uncontained_id = uncontained_submission.job_id.clone();
    let mut uncontained_persistence = StorageJobPersistence::open(&uncontained_path).unwrap();
    uncontained_runtime
        .service()
        .submit_persisted(&mut uncontained_persistence, uncontained_submission)
        .unwrap();
    let _ = uncontained_runtime
        .recover_startup()
        .unwrap()
        .join()
        .unwrap();
    wait_for_persisted_state(&uncontained_path, &uncontained_id, JobState::Failed);
    let uncontained_database = Database::open(&uncontained_path).unwrap();
    assert!(
        uncontained_database
            .archive_recovery_journal(&uncontained_id)
            .unwrap()
            .is_none()
    );
    assert!(!root.join("uncontained/archive/Tweets/12091").exists());
    let failure = uncontained_persistence
        .stored_job_summary(&uncontained_id)
        .unwrap();
    assert_eq!(
        failure.last_error_code.as_deref(),
        Some("EXECUTOR_WORKER_FAILED")
    );
    assert!(
        failure
            .last_error_message
            .as_deref()
            .is_some_and(|message| message.contains("PUBLICATION_INCOMPLETE_DRAIN")),
        "uncontained writer must be rejected by the same runner finalizer: {failure:?}"
    );
    drop(uncontained_runtime);

    // A destination collision after PREPARED belongs to InternalV1 replay;
    // the runner must retain the attempt as Downloading, not convert it to
    // a fresh FAILED result or emit terminal events.
    let collision_path = root.join("collision/config/archive.sqlite3");
    std::fs::create_dir_all(collision_path.parent().unwrap()).unwrap();
    let mut collision_config = config;
    collision_config.database_path = collision_path.clone();
    collision_config.archive_root = root.join("collision/archive");
    collision_config.staging_root = root.join("collision/staging");
    let collision_directory = collision_config.archive_root.join("Tweets/12091");
    std::fs::create_dir_all(&collision_directory).unwrap();
    std::fs::write(collision_directory.join("owner.marker"), b"preexisting").unwrap();
    let collision_runtime = ExecutorRuntime::with_linux_containment(collision_config, provisioning);
    let collision_submission = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "runner-v1-prepared-collision-job",
            OutputSettings::default(),
            DownloaderArgumentSnapshot::historical(),
        )
        .unwrap();
    let collision_id = collision_submission.job_id.clone();
    let mut collision_persistence = StorageJobPersistence::open(&collision_path).unwrap();
    collision_runtime
        .service()
        .submit_persisted(&mut collision_persistence, collision_submission)
        .unwrap();
    let _ = collision_runtime.recover_startup().unwrap().join().unwrap();
    let collision_database = Database::open(&collision_path).unwrap();
    assert_eq!(
        collision_database.job_state(&collision_id).unwrap(),
        JobState::Downloading
    );
    assert_eq!(
        collision_database
            .archive_recovery_journal(&collision_id)
            .unwrap()
            .unwrap()
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Prepared,
    );
    assert_eq!(
        std::fs::read(collision_directory.join("owner.marker")).unwrap(),
        b"preexisting",
    );
    assert!(
        collision_persistence
            .stored_events(&collision_id)
            .unwrap()
            .iter()
            .all(|(kind, _)| kind != "DOWNLOAD_COMPLETED" && kind != "JOB_COMPLETED"),
    );
    drop(collision_runtime);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires explicitly delegated disposable cgroup and real aria2"]
fn production_runner_registers_and_drains_real_aria2_before_internal_v1_publish() {
    use std::io::{Read, Write};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    let aria2 = std::env::var("TW2TG_TEST_ARIA2").expect("explicit real aria2 executable");
    let database_path = temporary_database_path("runner-v1-real-aria2");
    let root = database_path.parent().unwrap().parent().unwrap();
    std::fs::create_dir_all(root).unwrap();
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let delegated_parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let descriptor_path = database_path.with_extension("launch.json");
    let launcher_copy = database_path.with_extension("launcher.py");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        &launcher_copy,
    )
    .unwrap();
    std::fs::write(
        &descriptor_path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "delegated_parent": delegated_parent,
            "python": "/usr/bin/python3",
            "launcher": launcher_copy,
        }))
        .unwrap(),
    )
    .unwrap();
    let provisioning = LinuxExecutionContainment::from_launch_arguments([
        std::ffi::OsString::from("--linux-execution-descriptor"),
        descriptor_path.into_os_string(),
    ])
    .unwrap()
    .unwrap();
    let delegated_parent = provisioning.delegated_parent.clone();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let fixture_port = listener.local_addr().unwrap().port();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let server = std::thread::spawn(move || {
        while !stop_thread.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 2048];
                    let _size = stream.read(&mut request).unwrap_or(0);
                    let body = b"fixture-transfer";
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .unwrap();
                    stream.write_all(body).unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => panic!("fixture HTTP server failed: {error}"),
            }
        }
    });
    let script = format!(
        r#"import sys,json
c=json.loads(sys.stdin.readline())
print(json.dumps({{'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}}),flush=True)
c=json.loads(sys.stdin.readline())
r={{'tweet_id':'12091','url':c['url'],'tweet_type':'post','media':[{{'index':1,'media_id':'media-1','media_type':'photo','url':'http://127.0.0.1:{fixture_port}/fresh','filename':'media.jpg','mime_type':'image/jpeg'}}]}}
print(json.dumps({{'protocol_version':2,'event':'extracted','job_id':c['job_id'],'request_id':c['request_id'],'result':r}}),flush=True)
sys.stdin.read()
"#
    );
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: root.join("staging"),
        database_path: database_path.clone(),
        sidecar_program: Some("/usr/bin/python3".into()),
        gallery_dl_program: None,
        sidecar_args: vec!["-u".into(), "-c".into(), script],
        aria2_program: Some(aria2),
        use_aria2: true,
        network: ExecutorNetworkConfig {
            proxy_mode: ProxyMode::Direct,
            aria2_max_tries: 1,
            ..Default::default()
        },
    };
    let runtime = ExecutorRuntime::with_linux_containment(config, provisioning);
    let prepared = ArchiveJobSubmissionAdapter
        .prepare_with_downloader_settings(
            &archive_request("12091"),
            "runner-v1-real-aria2-job",
            OutputSettings::default(),
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
    assert_eq!(
        std::fs::read(root.join("archive/Tweets/12091/12091_01.jpg")).unwrap(),
        b"fixture-transfer",
    );
    let database = Database::open(&database_path).unwrap();
    assert_eq!(
        database.archive_recovery_contract(&job_id).unwrap(),
        Some(xarchive_storage::ArchiveRecoveryContract::InternalV1.version()),
    );
    assert_eq!(
        database
            .archive_recovery_journal(&job_id)
            .unwrap()
            .unwrap()
            .phase,
        xarchive_storage::ArchiveRecoveryPhase::Committed,
    );
    assert!(
        !std::fs::read_dir(&delegated_parent)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("tw2tg-att-")),
        "both registered Sidecar and aria2 writer groups must be drained and removed",
    );
    stop.store(true, Ordering::Release);
    server.join().unwrap();
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn linux_launch_descriptor_is_explicit_bounded_and_fail_closed() {
    let parse = |arguments: Vec<std::ffi::OsString>| {
        LinuxExecutionContainment::from_launch_arguments(arguments)
    };
    assert!(parse(vec![]).unwrap().is_none());
    assert!(parse(vec!["--linux-execution-descriptor".into()]).is_err());
    assert!(
        parse(vec![
            "--linux-execution-descriptor".into(),
            "relative.json".into()
        ])
        .is_err()
    );
    let path = temporary_database_path("launch-descriptor");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let link = path.with_extension("symlink");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(
        parse(vec![
            "--linux-execution-descriptor".into(),
            link.into_os_string()
        ])
        .is_err()
    );
    for bytes in [b"{".to_vec(), vec![b' '; 16 * 1024 + 1], br#"{"schema_version":2,"delegated_parent":"/sys/fs/cgroup/x","python":"/usr/bin/python3","launcher":"/missing"}"#.to_vec()] {
        std::fs::write(&path, bytes).unwrap();
        assert!(parse(vec!["--linux-execution-descriptor".into(), path.clone().into_os_string()]).is_err());
    }
    for bytes in [
        br#"{"schema_version":1,"delegated_parent":"/sys/fs/cgroup/x","python":"/usr/bin/python3","launcher":"/missing","unknown":true}"#.as_slice(),
        br#"{"schema_version":1,"schema_version":1,"delegated_parent":"/sys/fs/cgroup/x","python":"/usr/bin/python3","launcher":"/missing"}"#.as_slice(),
        br#"{"schema_version":1,"delegated_parent":"/sys/fs/cgroup/x","python":"relative","launcher":"/missing"}"#.as_slice(),
        br#"{"schema_version":1}"#.as_slice(),
    ] {
        std::fs::write(&path, bytes).unwrap();
        assert!(parse(vec!["--linux-execution-descriptor".into(), path.clone().into_os_string()]).is_err());
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}
