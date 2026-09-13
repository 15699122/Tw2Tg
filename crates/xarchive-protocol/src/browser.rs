use serde::{Deserialize, Serialize};

use crate::{PROTOCOL_VERSION, ProtocolError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum BrowserRequest {
    ArchiveRequest {
        protocol_version: u32,
        request_id: String,
        tweet: BrowserTweet,
    },
    QueryStatus {
        protocol_version: u32,
        request_id: String,
        tweet_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum BrowserResponse {
    ArchiveStatus {
        protocol_version: u32,
        request_id: String,
        tweet_id: String,
        job_id: Option<String>,
        state: String,
        progress: Option<serde_json::Value>,
    },
    Error {
        protocol_version: u32,
        request_id: Option<String>,
        error_code: String,
        error_message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTweet {
    pub tweet_id: String,
    pub url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default = "default_tweet_type")]
    pub tweet_type: String,
    #[serde(default)]
    pub reply_to: Option<String>,
    /// The tweet quoted by this tweet, if the DOM exposes enough nested data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quoted_tweet: Option<Box<BrowserTweet>>,
}

fn default_tweet_type() -> String {
    "post".to_owned()
}

fn validate_quoted_tweet(tweet: &BrowserTweet) -> Result<(), ProtocolError> {
    if tweet.tweet_id.is_empty() || !tweet.tweet_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(ProtocolError::InvalidTweetId);
    }
    if extract_tweet_id(&tweet.url) != Some(tweet.tweet_id.as_str()) {
        return Err(ProtocolError::InvalidTweetUrl);
    }
    if !matches!(tweet.tweet_type.as_str(), "post" | "reply" | "quote") {
        return Err(ProtocolError::InvalidTweetType);
    }
    if let Some(quoted) = &tweet.quoted_tweet {
        validate_quoted_tweet(quoted)?;
    }
    Ok(())
}

impl BrowserRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let (protocol_version, request_id) = match self {
            Self::ArchiveRequest {
                protocol_version,
                request_id,
                tweet,
            } => {
                if tweet.tweet_id.is_empty() || !tweet.tweet_id.chars().all(|c| c.is_ascii_digit())
                {
                    return Err(ProtocolError::InvalidTweetId);
                }
                if extract_tweet_id(&tweet.url) != Some(tweet.tweet_id.as_str()) {
                    return Err(ProtocolError::InvalidTweetUrl);
                }
                if !matches!(tweet.tweet_type.as_str(), "post" | "reply" | "quote") {
                    return Err(ProtocolError::InvalidTweetType);
                }
                if let Some(quoted) = &tweet.quoted_tweet {
                    validate_quoted_tweet(quoted)?;
                }
                (protocol_version, request_id)
            }
            Self::QueryStatus {
                protocol_version,
                request_id,
                tweet_ids,
            } => {
                if tweet_ids.is_empty()
                    || tweet_ids.len() > 100
                    || tweet_ids
                        .iter()
                        .any(|id| id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()))
                {
                    return Err(ProtocolError::InvalidTweetId);
                }
                (protocol_version, request_id)
            }
        };
        if *protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(*protocol_version));
        }
        if request_id.is_empty() || request_id.len() > 128 {
            return Err(ProtocolError::InvalidRequestId);
        }
        Ok(())
    }
}

/// Extract the status id from one supported canonical X URL shape.
pub fn extract_tweet_id(url: &str) -> Option<&str> {
    let (scheme, remainder) = url.split_once("://")?;
    if scheme != "https" {
        return None;
    }
    let (authority, path) = remainder.split_once('/')?;
    if authority != "x.com" && authority != "twitter.com" {
        return None;
    }
    if path.contains('?') || path.contains('#') {
        return None;
    }
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() != 3
        || segments[0].is_empty()
        || (segments[1] != "status" && segments[1] != "statuses")
        || segments[2].is_empty()
        || !segments[2].bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some(segments[2])
}
