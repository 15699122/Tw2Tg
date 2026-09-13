use std::fs;
use std::io;
use std::path::{Component, Path};

use xarchive_core::{ArchiveMetadata, ArchiveQuotedTweet};

use crate::{FileStore, StorageError};

/// Build portable metadata from the variable-shaped gallery-dl payload.
pub fn build_archive_metadata(
    expected_tweet_id: &str,
    raw: &serde_json::Value,
    files: &[xarchive_protocol::DownloadFile],
    staging_dir: &Path,
    archived_at: &str,
) -> Result<ArchiveMetadata, StorageError> {
    let object = raw
        .as_object()
        .ok_or_else(|| StorageError::InvalidMetadata("metadata must be an object".into()))?;
    let string_value = |keys: &[&str]| {
        keys.iter().find_map(|key| {
            object.get(*key).and_then(|value| match value {
                serde_json::Value::String(value) => Some(value.clone()),
                serde_json::Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
        })
    };
    let tweet_id = string_value(&["tweet_id", "status_id", "id"]).ok_or_else(|| {
        StorageError::InvalidMetadata("tweet_id is missing or not a string".into())
    })?;
    xarchive_core::TweetId::new(tweet_id.clone())
        .map_err(|_| StorageError::InvalidMetadata("tweet_id must be numeric".into()))?;
    if tweet_id != expected_tweet_id {
        return Err(StorageError::InvalidMetadata(
            "sidecar tweet_id does not match the archive request".into(),
        ));
    }

    let media_values = object
        .get("media")
        .or_else(|| object.get("items"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut media = Vec::with_capacity(files.len());
    for (position, file) in files.iter().enumerate() {
        let relative = Path::new(&file.relative_path);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(StorageError::InvalidPath);
        }
        let path = staging_dir.join(relative);
        let file_metadata = fs::symlink_metadata(&path)?;
        if !file_metadata.file_type().is_file()
            || crate::file_store::is_reparse_point(&file_metadata)
        {
            return Err(StorageError::InvalidPath);
        }
        if !path.is_file() {
            return Err(StorageError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("sidecar file is missing: {}", file.relative_path),
            )));
        }
        let size_bytes = fs::metadata(&path)?.len();
        let sha256 = FileStore::sha256(&path)?;
        let raw_media = media_values
            .get(position)
            .and_then(serde_json::Value::as_object);
        let media_id = raw_media
            .and_then(|value| value.get("media_id").or_else(|| value.get("id")))
            .map(|value| value.to_string().trim_matches('"').to_owned());
        media.push(xarchive_core::ArchiveMedia {
            index: position as u32 + 1,
            media_id,
            media_type: file.media_type.clone(),
            file: file.relative_path.clone(),
            mime_type: file.mime_type.clone(),
            size_bytes,
            sha256,
        });
    }

    let quoted_tweet = object
        .get("quoted_tweet")
        .or_else(|| object.get("quoted_status"))
        .and_then(serde_json::Value::as_object)
        .and_then(|quoted_object| {
            let quoted_id = quoted_object
                .get("tweet_id")
                .or_else(|| quoted_object.get("status_id"))
                .or_else(|| quoted_object.get("id"))
                .and_then(|value| match value {
                    serde_json::Value::String(value) => Some(value.clone()),
                    serde_json::Value::Number(value) => Some(value.to_string()),
                    _ => None,
                })?;
            xarchive_core::TweetId::new(quoted_id.clone()).ok()?;
            Some(ArchiveQuotedTweet {
                tweet_id: quoted_id,
                url: quoted_object
                    .get("url")
                    .or_else(|| quoted_object.get("tweet_url"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                username: quoted_object
                    .get("username")
                    .or_else(|| quoted_object.get("author_username"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                display_name: quoted_object
                    .get("display_name")
                    .or_else(|| quoted_object.get("author_name"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                text: quoted_object
                    .get("text")
                    .or_else(|| quoted_object.get("description"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                created_at: quoted_object
                    .get("created_at")
                    .or_else(|| quoted_object.get("date"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                tweet_type: quoted_object
                    .get("tweet_type")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            })
        });

    Ok(ArchiveMetadata {
        schema_version: 1,
        tweet_id,
        url: string_value(&["url", "tweet_url"]).unwrap_or_default(),
        tweet_type: string_value(&["tweet_type", "type"]).unwrap_or_else(|| "post".into()),
        author: xarchive_core::ArchiveAuthor {
            user_id: string_value(&["user_id", "author_id"]),
            username: string_value(&["username", "author_username", "user"]),
            display_name: string_value(&["display_name", "author_name"]),
        },
        created_at: string_value(&["created_at", "date", "timestamp"]),
        text: string_value(&["text", "description"]).unwrap_or_default(),
        media,
        archived_at: archived_at.to_owned(),
        reply_to: string_value(&[
            "in_reply_to_status_id_str",
            "in_reply_to_status_id",
            "in_reply_to",
            "reply_to_tweet_id",
            "reply_to",
        ])
        .filter(|value| xarchive_core::TweetId::new(value.clone()).is_ok()),
        quoted_tweet,
    })
}
