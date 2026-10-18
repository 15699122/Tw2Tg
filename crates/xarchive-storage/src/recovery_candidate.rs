//! Explicit business-candidate boundary, independent of portable DTO evolution.
use crate::StorageError;
use xarchive_core::ArchiveMetadata;

fn fields(value: &serde_json::Value, allowed: &[&str]) -> Result<(), StorageError> {
    let object = value.as_object().ok_or_else(invalid)?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> StorageError {
    StorageError::InvalidMetadata("unsupported or unsafe recovery business candidate".into())
}

fn status_url(url: &str, tweet_id: &str) -> bool {
    if url.is_empty() {
        return true;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let parts: Vec<_> = rest.split('/').collect();
    matches!(parts.as_slice(), [host, user, "status", id]
        if matches!(*host, "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com")
        && !user.is_empty()
        && user.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && *id == tweet_id
        && id.bytes().all(|byte| byte.is_ascii_digit()))
}

pub(crate) fn decode(json: &str) -> Result<ArchiveMetadata, StorageError> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    fields(
        &value,
        &[
            "schema_version",
            "tweet_id",
            "url",
            "tweet_type",
            "author",
            "created_at",
            "text",
            "media",
            "archived_at",
            "reply_to",
            "quoted_tweet",
        ],
    )?;
    fields(&value["author"], &["user_id", "username", "display_name"])?;
    for media in value["media"].as_array().ok_or_else(invalid)? {
        fields(
            media,
            &[
                "index",
                "media_id",
                "media_type",
                "file",
                "mime_type",
                "size_bytes",
                "sha256",
            ],
        )?;
    }
    if let Some(quoted) = value.get("quoted_tweet").filter(|value| !value.is_null()) {
        fields(
            quoted,
            &[
                "tweet_id",
                "url",
                "username",
                "display_name",
                "user_id",
                "text",
                "created_at",
                "tweet_type",
            ],
        )?;
    }
    let metadata: ArchiveMetadata = serde_json::from_value(value)?;
    if !matches!(metadata.schema_version, 1 | 2)
        || xarchive_core::TweetId::new(metadata.tweet_id.clone()).is_err()
        || !status_url(&metadata.url, &metadata.tweet_id)
        || metadata
            .reply_to
            .as_ref()
            .is_some_and(|id| xarchive_core::TweetId::new(id.clone()).is_err())
        || metadata.media.iter().any(|media| media.validate().is_err())
        || metadata.quoted_tweet.as_ref().is_some_and(|quoted| {
            xarchive_core::TweetId::new(quoted.tweet_id.clone()).is_err()
                || !status_url(&quoted.url, &quoted.tweet_id)
        })
    {
        return Err(invalid());
    }
    Ok(metadata)
}

pub(crate) fn encode(metadata: &ArchiveMetadata) -> Result<String, StorageError> {
    let json = serde_json::to_string(metadata)?;
    // Fail closed if the portable DTO grows: every new field needs an explicit
    // recovery-necessity review rather than silently expanding durable storage.
    decode(&json)?;
    Ok(json)
}
