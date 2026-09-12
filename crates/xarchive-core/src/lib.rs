//! Domain primitives shared by the Desktop application and future tooling.

mod job;
mod reliability;
mod tags;

pub use job::{JobEvent, JobState, JobStateError, is_active_state, is_terminal_state};
pub use reliability::{ErrorClass, RetryDecision, RetryPolicy, decide_retry};
pub use tags::{TagInput, TagRule, evaluate_tags};

use serde::{Deserialize, Serialize};

/// Portable metadata written beside the original media files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    pub schema_version: u32,
    pub tweet_id: String,
    pub url: String,
    pub tweet_type: String,
    pub author: ArchiveAuthor,
    pub created_at: Option<String>,
    pub text: String,
    pub media: Vec<ArchiveMedia>,
    pub archived_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quoted_tweet: Option<ArchiveQuotedTweet>,
}

/// Reference data for a tweet quoted by the archived tweet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveQuotedTweet {
    pub tweet_id: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tweet_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveAuthor {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
}

pub fn stable_user_directory_name(
    username: Option<&str>,
    display_name: Option<&str>,
    user_id: &str,
) -> String {
    let handle = username
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown");
    let name = display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Unknown");
    format!(
        "@{} - {} [{}]",
        sanitize_component(handle),
        sanitize_component(name),
        sanitize_component(user_id)
    )
}

fn sanitize_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        let allowed = !matches!(
            character,
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
        ) && !character.is_control();
        output.push(if allowed { character } else { '_' });
    }
    let trimmed = output.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "_".into()
    } else {
        trimmed.chars().take(120).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveMedia {
    pub index: u32,
    pub media_id: Option<String>,
    pub media_type: String,
    pub file: String,
    pub mime_type: Option<String>,
    pub size_bytes: u64,
    pub sha256: String,
}

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

    #[test]
    fn creates_stable_windows_safe_user_directory_names() {
        assert_eq!(
            stable_user_directory_name(Some("alice"), Some("Alice / One"), "123"),
            "@alice - Alice _ One [123]"
        );
        assert_eq!(
            stable_user_directory_name(None, None, "456"),
            "@unknown - Unknown [456]"
        );
    }
}
