#[cfg(all(test, target_os = "linux"))]
pub(crate) fn assert_transfer_fixture_success(
    plan: &MediaTransferPlan,
    program: &str,
    domain: &crate::executor::AttemptWriterDomain,
) {
    let network = crate::executor::ExecutorNetworkConfig {
        proxy_mode: xarchive_core::ProxyMode::Direct,
        transfer_timeout: Duration::from_secs(5),
        aria2_connect_timeout: Duration::from_secs(2),
        aria2_idle_timeout: Duration::from_secs(2),
        aria2_max_tries: 1,
        ..Default::default()
    };
    let outcomes = transfer_once(
        plan,
        &CancellationToken::default(),
        Some(program),
        &network,
        &[],
        Some(domain),
        0,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "production transfer fixture: {}",
            failure.diagnostic.message
        )
    });
    assert_eq!(outcomes.len(), 1);
    let mut expired = plan.clone();
    expired.items[0].url = "http://127.0.0.1/expired".into();
    expired.items[0].filename = "expired.jpg".into();
    let failure = match transfer_once(
        &expired,
        &CancellationToken::default(),
        Some(program),
        &network,
        &[],
        Some(domain),
        1,
    ) {
        Ok(_) => panic!("expired transfer must fail"),
        Err(failure) => failure,
    };
    assert_eq!(failure.diagnostic.code, TransferFailureCode::ExpiredUrl);
    assert!(
        failure.retained_supervisor.is_none(),
        "drain must complete before refresh eligibility"
    );
    assert!(transfer_failure_enters_refresh(&failure));
    // A nested empty cgroup makes group removal fail after writer drain. This
    // exercises a real retained supervisor, without inventing startup failure.
    let group = domain.provisioning.delegated_parent.join(
        domain
            .identity
            .clone()
            .with_writer("aria2-0")
            .unwrap()
            .group_name(),
    );
    let blocker = group.join("retry-blocker");
    let blocker_for_thread = blocker.clone();
    let monitor = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if blocker_for_thread
                .parent()
                .unwrap()
                .join("cgroup.procs")
                .exists()
            {
                std::fs::create_dir(&blocker_for_thread).unwrap();
                return;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    let mut failure = match transfer_once(
        plan,
        &CancellationToken::default(),
        Some(program),
        &network,
        &[],
        Some(domain),
        0,
    ) {
        Ok(_) => panic!("nested group must retain cleanup owner"),
        Err(failure) => failure,
    };
    monitor.join().unwrap();
    assert_eq!(
        failure.diagnostic.code,
        TransferFailureCode::ShutdownIncomplete
    );
    assert!(failure.retained_supervisor.is_some());
    assert!(!transfer_failure_enters_refresh(&failure));
    let retained_address = failure.retained_supervisor.as_ref().unwrap() as *const Aria2Supervisor;
    assert!(retry_retained_drain(&mut failure).is_err());
    assert_eq!(
        failure.retained_supervisor.as_ref().unwrap() as *const Aria2Supervisor,
        retained_address
    );
    std::fs::remove_dir(blocker).unwrap();
    assert!(retry_retained_drain(&mut failure).unwrap());
    assert!(failure.retained_supervisor.is_none());
    assert!(!group.exists());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let mut cancelled_plan = plan.clone();
    cancelled_plan.items[0].filename = "cancelled.jpg".into();
    let failure = match transfer_once(
        &cancelled_plan,
        &cancelled,
        Some(program),
        &network,
        &[],
        Some(domain),
        0,
    ) {
        Ok(_) => panic!("cancelled transfer must fail"),
        Err(failure) => failure,
    };
    assert!(failure.retained_supervisor.is_none());
    assert!(!transfer_failure_enters_refresh(&failure));
    let sidecar_script = r#"import sys,json
c=json.loads(sys.stdin.readline())
print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
c=json.loads(sys.stdin.readline())
r={'tweet_id':'123','url':c['url'],'tweet_type':'post','media':[{'index':1,'media_id':'media-1','media_type':'photo','url':'http://127.0.0.1/fresh','filename':'refreshed.jpg'}]}
print(json.dumps({'protocol_version':2,'event':'extracted','job_id':c['job_id'],'request_id':c['request_id'],'result':r}),flush=True)
sys.stdin.read()
"#;
    for scenario in ["success", "changed", "expired"] {
        let script = match scenario {
            "changed" => {
                sidecar_script.replace("'media_id':'media-1'", "'media_id':'changed-media'")
            }
            "expired" => {
                sidecar_script.replace("http://127.0.0.1/fresh", "http://127.0.0.1/expired")
            }
            _ => sidecar_script.to_owned(),
        };
        let mut scenario_domain = domain.clone();
        scenario_domain.identity = xarchive_workflow::linux_containment::AttemptIdentity::new(
            &plan.items[0].directory,
            &format!("refresh-{scenario}"),
            1,
        )
        .unwrap();
        let owner =
            crate::executor::recover_and_create_writer_group(&scenario_domain, "sidecar").unwrap();
        let mut sidecar = SidecarSupervisor::spawn_contained_with_environment(
            owner,
            &domain.provisioning.python,
            &domain.provisioning.launcher,
            Path::new("/usr/bin/python3"),
            &["-u", "-c", &script],
            &xarchive_sidecar_supervisor::ChildProcessEnvironment::default(),
        )
        .unwrap()
        .require_ready_v2(Duration::from_secs(3))
        .unwrap();
        let extraction: ExtractionResult = serde_json::from_value(serde_json::json!({
            "tweet_id":"123", "url":"https://x.com/a/status/123", "tweet_type":"post",
            "media":[{"index":1,"media_id":"media-1","media_type":"photo",
                      "url":"http://127.0.0.1/expired","filename":"refreshed.jpg"}]
        }))
        .unwrap();
        let request = SidecarDownloadRequest {
            job_id: "transfer-job".into(),
            request_id: "refresh-request".into(),
            url: extraction.url.clone(),
            staging_dir: PathBuf::from(&plan.items[0].directory),
            browser: None,
            profile: None,
            downloader_arguments: crate::executor::DownloaderArgumentSnapshot::historical(),
            gallery_dl_program: None,
        };
        let initial =
            media_transfer_plan(&extraction, request.staging_dir.to_string_lossy()).unwrap();
        let result = transfer_with_one_refresh(
            &mut sidecar,
            &request,
            &CancellationToken::default(),
            (extraction, initial),
            Some(program),
            &network,
            Some(&scenario_domain),
        );
        if scenario == "success" {
            let result = result.unwrap_or_else(|(message, _)| panic!("refresh fixture: {message}"));
            assert_eq!(result.2.len(), 1);
            assert!(result.3.is_none());
            assert_eq!(
                std::fs::read(request.staging_dir.join("refreshed.jpg")).unwrap(),
                b"fixture-transfer"
            );
        } else {
            let (message, owner) = match result {
                Ok(_) => panic!("{scenario} must fail"),
                Err(error) => error,
            };
            assert!(owner.is_none());
            assert!(
                message.contains(if scenario == "changed" {
                    "EXTRACTION_RESULT_CHANGED"
                } else {
                    "TRANSFER_EXPIRED_URL"
                }),
                "{message}"
            );
        }
        sidecar.shutdown().unwrap();
    }
}
