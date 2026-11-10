use super::*;

#[test]
fn validates_discovery_command_and_candidate_events() {
    let command = SidecarV2Command::discover(
        "r1",
        "batch-1",
        "https://x.com/alice",
        Some("edge".to_owned()),
        None,
    );
    assert_eq!(command.validate(), Ok(()));

    let mut bad = SidecarV2Command::discover("r1", "batch-1", "https://example.com/x", None, None);
    assert_eq!(bad.validate(), Err(ProtocolError::InvalidTweetUrl));
    bad = SidecarV2Command::discover("r1", "system", "https://x.com/alice", None, None);
    assert_eq!(bad.validate(), Err(ProtocolError::InvalidSidecarV2Identity));

    let candidate = DiscoveryCandidate {
        tweet_id: "123".to_owned(),
        url: "https://x.com/alice/status/123".to_owned(),
        created_at: Some("2026-09-01T00:00:00Z".to_owned()),
        tweet_type: "post".to_owned(),
        is_repost: false,
        has_media: true,
        media_count: 2,
        user_id: Some("9001".to_owned()),
        username: Some("alice".to_owned()),
    };
    let started = SidecarV2Event::discovery_started("batch-1", Some("r1".to_owned()));
    assert_eq!(started.validate(), Ok(()));
    let event = SidecarV2Event::candidate("batch-1", Some("r1".to_owned()), candidate);
    assert_eq!(event.validate(), Ok(()));
    let completed = SidecarV2Event::discovery_completed("batch-1", Some("r1".to_owned()), 42);
    assert_eq!(completed.validate(), Ok(()));

    // Candidate payload must not ride on unrelated events.
    let mut smuggled = SidecarV2Event::discovery_started("batch-1", Some("r1".to_owned()));
    smuggled.candidate = Some(DiscoveryCandidate {
        tweet_id: "123".to_owned(),
        url: "https://x.com/alice/status/123".to_owned(),
        created_at: None,
        tweet_type: "post".to_owned(),
        is_repost: false,
        has_media: false,
        media_count: 0,
        user_id: None,
        username: None,
    });
    assert_eq!(
        smuggled.validate(),
        Err(ProtocolError::InvalidSidecarV2Event)
    );

    // Invalid candidate identity is rejected.
    let mut invalid = SidecarV2Event::candidate(
        "batch-1",
        Some("r1".to_owned()),
        DiscoveryCandidate {
            tweet_id: "12x".to_owned(),
            ..event.candidate.expect("candidate")
        },
    );
    invalid.candidate.as_mut().expect("candidate").tweet_id = "12x".to_owned();
    assert!(invalid.validate().is_err());
}
