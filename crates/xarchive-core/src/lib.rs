//! Domain primitives shared by the Desktop application and future tooling.

mod job;

pub use job::{JobState, JobStateError, is_active_state, is_terminal_state};

/// Stable identifier for an X post.
///
/// X identifiers are represented as strings so browser, JSON, JavaScript,
/// Python, and Rust never lose precision on large integers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TweetId(String);

impl TweetId {
    pub fn new(value: impl Into<String>) -> Result<Self, TweetIdError> {
        let value = value.into();
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(TweetIdError);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TweetIdError;

impl std::fmt::Display for TweetIdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("tweet_id must be a non-empty decimal string")
    }
}

impl std::error::Error for TweetIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_large_tweet_ids_as_strings() {
        let id = TweetId::new("1961234567890123456").expect("valid ID");
        assert_eq!(id.as_str(), "1961234567890123456");
    }

    #[test]
    fn rejects_non_decimal_ids() {
        assert!(TweetId::new("abc").is_err());
        assert!(TweetId::new("").is_err());
    }
}
