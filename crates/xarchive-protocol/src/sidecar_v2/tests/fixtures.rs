use super::*;

#[test]
fn shared_fixtures_deserialize_and_validate() {
    // The shared fixtures are the cross-language contract reference, so a
    // schema or capability change that forgets them must fail here instead
    // of drifting silently against the Rust types.
    for line in
        include_str!("../../../../../shared/protocol-schema/fixtures/sidecar-v2-command.jsonl")
            .lines()
            .filter(|line| !line.trim().is_empty())
    {
        let mut command: SidecarV2Command =
            serde_json::from_str(line).expect("fixture command parses");
        if let Some(staging_dir) = command.staging_dir.as_mut() {
            // The shared JSONL uses POSIX examples; instantiate only those
            // known fixture paths for the host, preserving relative-path
            // rejection without pinning a specific job id.
            assert!(
                staging_dir.starts_with("/tmp/xarchive-staging/"),
                "fixture staging_dir is a known POSIX example: {staging_dir}"
            );
            *staging_dir = host_absolute_staging_dir();
        }
        assert_eq!(command.validate(), Ok(()), "fixture command validates");
    }
    for line in
        include_str!("../../../../../shared/protocol-schema/fixtures/sidecar-v2-events.jsonl")
            .lines()
            .filter(|line| !line.trim().is_empty())
    {
        let event: SidecarV2Event = serde_json::from_str(line).expect("fixture event parses");
        assert_eq!(event.validate(), Ok(()), "fixture event validates");
    }
}
