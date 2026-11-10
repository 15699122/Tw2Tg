use super::*;

#[test]
fn accepts_valid_extract_command_and_event() {
    let command = SidecarV2Command::extract(
        "request-1",
        "job-1",
        "https://x.com/alice/status/123",
        Some("edge".to_owned()),
        None,
    );
    assert_eq!(command.validate(), Ok(()));
    let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), sample_result());
    assert_eq!(event.validate(), Ok(()));
}

#[test]
fn requires_download_media_capability_for_handshake() {
    let mut without_download = Vec::new();
    for capability in REQUIRED_V2_CAPABILITIES {
        if capability != SidecarV2Capability::DownloadMedia {
            without_download.push(capability);
        }
    }
    assert!(!has_required_capabilities(&without_download));
    assert!(has_required_capabilities(&REQUIRED_V2_CAPABILITIES));
}
