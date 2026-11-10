use super::*;

#[test]
fn validates_download_command_and_events() {
    let staging = std::env::temp_dir().join("xarchive-staging-job-1");
    let command = SidecarV2Command::download(
        "r1",
        "job-1",
        "https://x.com/alice/status/123",
        None,
        None,
        staging.display().to_string(),
        Vec::new(),
    );
    assert_eq!(command.validate(), Ok(()));

    let started = SidecarV2Event::download_started("job-1", Some("r1".to_owned()));
    assert_eq!(started.validate(), Ok(()));
    let completed = SidecarV2Event::download_completed(
        "job-1",
        Some("r1".to_owned()),
        sample_download(),
        Some(sample_result()),
    );
    assert_eq!(completed.validate(), Ok(()));
}

#[test]
fn download_user_args_are_bounded_and_only_allowed_on_download() {
    let staging = host_absolute_staging_dir();
    let mut command = SidecarV2Command::download(
        "r1",
        "job-1",
        "https://x.com/alice/status/123",
        None,
        None,
        staging.clone(),
        vec!["--limit-rate".into(), "2M".into(), "包含空格的值".into()],
    );
    assert_eq!(command.validate(), Ok(()));
    assert_eq!(
        serde_json::from_str::<SidecarV2Command>(
            &serde_json::to_string(&command).expect("serialize command")
        )
        .expect("round-trip command")
        .user_args,
        command.user_args
    );

    command.user_args = vec!["".into()];
    assert_eq!(
        command.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );
    command.user_args = vec!["bad\narg".into()];
    assert_eq!(
        command.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );
    command.user_args = vec!["x".repeat(4097)];
    assert_eq!(
        command.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );
    command.user_args = vec!["x".into(); 129];
    assert_eq!(
        command.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );

    let mut extract =
        SidecarV2Command::extract("r2", "job-1", "https://x.com/alice/status/123", None, None);
    extract.user_args = vec!["--limit-rate".into()];
    assert_eq!(
        extract.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );
}

#[test]
fn download_command_matches_shared_user_args_json_fixture() {
    let command = SidecarV2Command::download(
        "fixture-request",
        "fixture-job",
        "https://x.com/alice/status/123",
        None,
        None,
        host_absolute_staging_dir(),
        vec![
            "--limit-rate".into(),
            "2M".into(),
            "value with spaces".into(),
        ],
    );
    let value = serde_json::to_value(command).expect("serialize download command");
    assert_eq!(
        value["user_args"],
        serde_json::json!(["--limit-rate", "2M", "value with spaces"])
    );
    assert_eq!(value["cmd"], "download");
}

#[test]
fn historical_download_command_without_user_args_still_deserializes() {
    let command = serde_json::json!({
        "protocol_version": 2,
        "request_id": "legacy-request",
        "cmd": "download",
        "job_id": "legacy-job",
        "url": "https://x.com/alice/status/123",
        "staging_dir": host_absolute_staging_dir(),
    });
    let parsed: SidecarV2Command =
        serde_json::from_value(command).expect("historical command deserializes");
    assert!(parsed.user_args.is_empty());
    assert_eq!(parsed.validate(), Ok(()));
}

#[test]
fn download_command_requires_absolute_staging_dir() {
    // The positive case matters as much as the negative one: a validator that
    // rejected every staging directory would still satisfy every assertion
    // below and silently break all downloads.
    let accepted = SidecarV2Command::download(
        "r1",
        "job-1",
        "https://x.com/alice/status/123",
        None,
        None,
        host_absolute_staging_dir(),
        vec!["--write-info-json".into()],
    );
    assert_eq!(
        accepted.validate(),
        Ok(()),
        "a host-absolute staging directory must be accepted"
    );

    let mut command = SidecarV2Command::download(
        "r1",
        "job-1",
        "https://x.com/alice/status/123",
        None,
        None,
        "relative/job-1".to_owned(),
        Vec::new(),
    );
    assert_eq!(
        command.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );
    command.staging_dir = None;
    assert_eq!(
        command.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );
    // Extraction-only commands never carry a staging directory.
    let mut extract =
        SidecarV2Command::extract("r1", "job-1", "https://x.com/alice/status/123", None, None);
    extract.staging_dir = Some(host_absolute_staging_dir());
    assert_eq!(
        extract.validate(),
        Err(ProtocolError::InvalidSidecarV2Command)
    );
}

#[test]
fn rejects_unsafe_download_payloads_and_event_riding() {
    let mut escaping = sample_download();
    escaping.media[0].relative_path = "../escape.jpg".to_owned();
    let event = SidecarV2Event::download_completed(
        "job-1",
        Some("r1".to_owned()),
        escaping,
        Some(sample_result()),
    );
    assert!(event.validate().is_err());

    let mut absolute = sample_download();
    absolute.media[0].relative_path = "/abs.jpg".to_owned();
    assert!(absolute.validate().is_err());

    let mut traversal = sample_download();
    traversal.media[0].relative_path = "sub/../01.jpg".to_owned();
    assert!(traversal.validate().is_err());

    let mut empty = sample_download();
    empty.media[0].size_bytes = 0;
    assert!(empty.validate().is_err());

    // The download payload must not ride on an unrelated event.
    let mut smuggled = SidecarV2Event::download_started("job-1", Some("r1".to_owned()));
    smuggled.download = Some(sample_download());
    assert_eq!(
        smuggled.validate(),
        Err(ProtocolError::InvalidSidecarV2Event)
    );

    // download_completed requires a download payload.
    let mut missing = SidecarV2Event::download_completed(
        "job-1",
        Some("r1".to_owned()),
        DownloadResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            media: Vec::new(),
            result: Some(sample_result()),
        },
        None,
    );
    missing.download = None;
    assert_eq!(
        missing.validate(),
        Err(ProtocolError::InvalidSidecarV2Event)
    );
}
