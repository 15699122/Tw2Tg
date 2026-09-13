#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidTweetId,
    InvalidTweetUrl,
    UnsupportedVersion(u32),
    InvalidRequestId,
    InvalidTweetType,
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
        }
    }
}

impl std::error::Error for ProtocolError {}
