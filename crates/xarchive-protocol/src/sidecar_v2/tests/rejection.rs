use super::*;

#[test]
fn rejects_missing_capabilities() {
    let event = SidecarV2Event::ready(
        "system",
        Some("r1".to_owned()),
        vec![SidecarV2Capability::ExtractMedia],
    );
    assert_eq!(
        event.validate(),
        Err(ProtocolError::InvalidSidecarV2Capability)
    );
}

#[test]
fn rejects_identity_mismatch_and_cookie_headers() {
    let mut result = sample_result();
    result.tweet_id = "999".to_owned();
    let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
    assert_eq!(
        event.validate(),
        Err(ProtocolError::InvalidSidecarV2Identity)
    );

    let mut result = sample_result();
    result.request_headers = vec![ExtractionRequestHeader {
        name: "Cookie".to_owned(),
        value: "session=secret".to_owned(),
    }];
    let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
    assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));
}

#[test]
fn rejects_invalid_author_and_relationship_fields() {
    let mut result = sample_result();
    result.user_id = Some("not-numeric".to_owned());
    let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
    assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));

    let mut result = sample_result();
    result.reply_to = Some("12x".to_owned());
    let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
    assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));

    let mut result = sample_result();
    result.quoted_tweet.as_mut().expect("quoted").url = "https://x.com/bob/status/555".to_owned();
    let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
    assert_eq!(
        event.validate(),
        Err(ProtocolError::InvalidSidecarV2Identity)
    );

    let mut result = sample_result();
    result.quoted_tweet.as_mut().expect("quoted").user_id = Some("abc".to_owned());
    let event = SidecarV2Event::extracted("job-1", Some("r1".to_owned()), result);
    assert_eq!(event.validate(), Err(ProtocolError::InvalidSidecarV2Event));
}

#[test]
fn rejects_unknown_fields() {
    let input = "{\"protocol_version\":2,\"request_id\":\"r1\",\"cmd\":\"extract\",\"job_id\":\"j1\",\"url\":\"https://x.com/a/status/1\",\"executable\":\"x\"}";
    let parsed: Result<SidecarV2Command, _> = serde_json::from_str(input);
    assert!(parsed.is_err());
}
