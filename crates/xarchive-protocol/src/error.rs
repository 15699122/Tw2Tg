#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidTweetId,
    InvalidTweetUrl,
    UnsupportedVersion(u32),
    InvalidRequestId,
    InvalidTweetType,
    InvalidSidecarV2Command,
    InvalidSidecarV2Event,
    InvalidSidecarV2Capability,
    InvalidSidecarV2Identity,
    UnsupportedSidecarV2Version(u32),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTweetId => {
                formatter.write_str("tweet_id must be a non-empty decimal string")
            }
            Self::InvalidTweetUrl => formatter.write_str("tweet URL must use x.com or twitter.com"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version: {version}")
            }
            Self::InvalidRequestId => formatter.write_str("request_id must be 1-128 characters"),
            Self::InvalidTweetType => {
                formatter.write_str("tweet_type must be post, reply, or quote")
            }
            Self::InvalidSidecarV2Command => formatter.write_str("invalid sidecar v2 command"),
            Self::InvalidSidecarV2Event => formatter.write_str("invalid sidecar v2 event"),
            Self::InvalidSidecarV2Capability => {
                formatter.write_str("sidecar v2 worker is missing required capabilities")
            }
            Self::InvalidSidecarV2Identity => {
                formatter.write_str("sidecar v2 request/job identity mismatch")
            }
            Self::UnsupportedSidecarV2Version(version) => {
                write!(formatter, "unsupported sidecar protocol version: {version}")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}
