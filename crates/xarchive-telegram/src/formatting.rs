//! Pure message formatting and media send planning.
//!
//! Everything in this module is a deterministic function of its arguments: it
//! reads no credential, holds no outbox lease, performs no I/O and starts no
//! request. Timeouts, cancellation, credential rotation and uncertain-send
//! outcomes stay in the transport and outbox code, so a caller can reuse the
//! planning helpers without inheriting any delivery state.
//!
//! The crate root re-exports every public item here, so existing call sites and
//! serialized payloads are unchanged.

use serde::{Deserialize, Serialize};

use crate::{
    MediaGroupItem, TELEGRAM_ALBUM_MIN_ITEMS, TELEGRAM_MEDIA_GROUP_LIMIT, TELEGRAM_TEXT_LIMIT,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataInput<'a> {
    pub tweet_id: &'a str,
    pub url: &'a str,
    pub username: Option<&'a str>,
    pub display_name: Option<&'a str>,
    pub text: &'a str,
    pub tags: &'a [&'a str],
}

pub fn format_metadata(input: &MetadataInput<'_>) -> String {
    let author = match (input.display_name, input.username) {
        (Some(name), Some(username)) => format!("{name} (@{username})"),
        (Some(name), None) => name.to_owned(),
        (None, Some(username)) => format!("@{username}"),
        (None, None) => "Unknown author".to_owned(),
    };
    let mut output = format!(
        "{author}\nhttps://x.com/i/status/{}\n\n{}",
        input.tweet_id,
        input.text.trim()
    );
    if !input.tags.is_empty() {
        output.push_str("\n\n");
        output.push_str(
            &input
                .tags
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    if !input.url.is_empty() && !output.contains(input.url) {
        output.push('\n');
        output.push_str(input.url);
    }
    output
}

pub fn split_text(text: &str, limit: usize) -> Vec<String> {
    if limit == 0 || text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(limit)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

pub fn split_for_telegram(text: &str) -> Vec<String> {
    split_text(text, TELEGRAM_TEXT_LIMIT)
}

/// How a media item is classified for delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Photo,
    Video,
    Document,
}

/// Delivery intent for a media item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadMode {
    /// Browse in the client: photo/video/album methods. No byte-identical
    /// download promise.
    #[default]
    Display,
    /// Preserve the exact bytes: a file message, hash-verified on download.
    OriginalFile,
}

impl MediaKind {
    pub fn api_media_type(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Video => "video",
            Self::Document => "document",
        }
    }
}

/// One unit of a send plan: either an album or a standalone item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaGroupSend {
    Album(Vec<MediaGroupItem>),
    Single(MediaGroupItem),
}

/// Classify a media item for display purposes.
///
/// The declared MIME type wins, with the file extension as a fallback.
/// Anything unrecognised becomes a `Document`, which is the safe choice: it
/// never claims the client can render or play the file inline.
pub fn classify_media(mime_type: Option<&str>, file_name: &str) -> MediaKind {
    let mime = mime_type.unwrap_or("").trim().to_ascii_lowercase();
    // `image/gif` is an animation, not a still photo, so it falls through to
    // `Document` rather than being claimed as a photo.
    if mime.starts_with("image/") && mime != "image/gif" {
        return MediaKind::Photo;
    }
    if mime.starts_with("video/") {
        return MediaKind::Video;
    }
    let extension = normalized_extension(file_name);
    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" => MediaKind::Photo,
        "mp4" | "mov" | "m4v" | "webm" => MediaKind::Video,
        _ => MediaKind::Document,
    }
}

/// Build a deterministic, collision-resistant upload file name (TG-03).
///
/// Shape: `x_<tweet_id>_<media_index>_<hash_prefix>.<ext>`. It is never a
/// generic name such as `video.mp4`, never contains an absolute local path, and
/// never renames the archived file on disk.
pub fn stable_upload_file_name(
    tweet_id: &str,
    media_index: u32,
    content_hash_hex: &str,
    original_file_name: &str,
) -> String {
    let tweet = sanitize_token(tweet_id, 32);
    let mut hash_prefix: String = content_hash_hex
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(12)
        .collect();
    hash_prefix = hash_prefix.to_ascii_lowercase();
    if hash_prefix.is_empty() {
        hash_prefix = "nohash".to_owned();
    }
    let extension = normalized_extension(original_file_name);
    format!("x_{tweet}_{media_index:02}_{hash_prefix}.{extension}")
}

fn normalized_extension(file_name: &str) -> String {
    let raw = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or("");
    let cleaned: String = raw
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect();
    let cleaned = cleaned.to_ascii_lowercase();
    if cleaned.is_empty() {
        "bin".to_owned()
    } else {
        cleaned
    }
}

fn sanitize_token(value: &str, max: usize) -> String {
    let cleaned: String = value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(max)
        .collect();
    if cleaned.is_empty() {
        "unknown".to_owned()
    } else {
        cleaned
    }
}

/// Group items into sendable units.
///
/// [`media_groups`] chunks by the album maximum, which can leave a trailing
/// group of one item. Because an album requires 2–10 items, any such group is
/// promoted to a standalone single send instead of an invalid one-item album.
pub fn media_send_plan(items: Vec<MediaGroupItem>) -> Vec<MediaGroupSend> {
    media_groups(items)
        .into_iter()
        .map(|mut group| {
            if group.len() < TELEGRAM_ALBUM_MIN_ITEMS
                && let Some(single) = group.pop()
            {
                return MediaGroupSend::Single(single);
            }
            MediaGroupSend::Album(group)
        })
        .collect()
}

pub fn media_groups(items: Vec<MediaGroupItem>) -> Vec<Vec<MediaGroupItem>> {
    items
        .chunks(TELEGRAM_MEDIA_GROUP_LIMIT)
        .map(|group| group.to_vec())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_metadata_and_continuations() {
        let input = MetadataInput {
            tweet_id: "123",
            url: "https://x.com/alice/status/123",
            username: Some("alice"),
            display_name: Some("Alice"),
            text: "hello",
            tags: &["person", "launch"],
        };
        let text = format_metadata(&input);
        assert!(text.contains("Alice (@alice)"));
        assert!(text.contains("#person #launch"));
        assert_eq!(split_text("你好世界", 2), vec!["你好", "世界"]);
    }

    fn photo_item(index: usize) -> MediaGroupItem {
        MediaGroupItem {
            media_type: "photo".into(),
            media: format!("file-{index}"),
            caption: None,
        }
    }

    #[test]
    fn groups_media_in_telegram_limit() {
        let items = (0..21)
            .map(|index| MediaGroupItem {
                media_type: "photo".into(),
                media: format!("file-{index}"),
                caption: None,
            })
            .collect();
        let groups = media_groups(items);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 10);
        assert_eq!(groups[2].len(), 1);
    }

    #[test]
    fn classifies_media_by_mime_then_extension() {
        assert_eq!(
            classify_media(Some("image/jpeg"), "a.jpg"),
            MediaKind::Photo
        );
        assert_eq!(classify_media(Some("IMAGE/PNG"), "a.png"), MediaKind::Photo);
        assert_eq!(classify_media(Some("video/mp4"), "a.mp4"), MediaKind::Video);
        // A GIF is an animation, not a still photo; document is the safe claim.
        assert_eq!(
            classify_media(Some("image/gif"), "a.gif"),
            MediaKind::Document
        );
        // The extension is only a fallback when the MIME type is absent/unknown.
        assert_eq!(classify_media(None, "PHOTO.PNG"), MediaKind::Photo);
        assert_eq!(classify_media(None, "clip.MOV"), MediaKind::Video);
        assert_eq!(
            classify_media(Some("application/octet-stream"), "archive.zip"),
            MediaKind::Document
        );
        assert_eq!(classify_media(None, "noext"), MediaKind::Document);
    }

    #[test]
    fn builds_stable_upload_file_names() {
        let name = stable_upload_file_name(
            "1234567890123456789",
            2,
            "A1B2C3D4E5F60708112233",
            "IMG_0001.JPG",
        );
        assert_eq!(name, "x_1234567890123456789_02_a1b2c3d4e5f6.jpg");

        // Deterministic: identical inputs give an identical name.
        assert_eq!(
            name,
            stable_upload_file_name(
                "1234567890123456789",
                2,
                "A1B2C3D4E5F60708112233",
                "IMG_0001.JPG"
            )
        );
        // Two-digit index keeps names sortable and distinct.
        assert!(stable_upload_file_name("9", 12, "aa", "a.mp4").contains("_12_"));
        // Never a generic name and never a local path fragment.
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
        assert!(!name.contains(':'));
        assert_ne!(name, "video.mp4");
        // A missing or non-hex hash still yields a usable, distinct name.
        assert!(stable_upload_file_name("9", 1, "", "a.mp4").contains("nohash"));
        assert!(stable_upload_file_name("9", 1, "zzz", "a.mp4").contains("nohash"));
        // Path-hostile inputs are sanitised, not passed through.
        assert_eq!(
            stable_upload_file_name("../../etc/passwd", 1, "ab", "../../a.mp4"),
            "x_etcpasswd_01_ab.mp4"
        );
        // A missing extension falls back to `bin` rather than an empty suffix.
        assert!(stable_upload_file_name("9", 1, "ab", "noext").ends_with(".bin"));
    }

    #[test]
    fn promotes_a_trailing_single_item_out_of_an_album() {
        assert!(media_send_plan(vec![]).is_empty());

        assert_eq!(
            media_send_plan(vec![photo_item(0)]),
            vec![MediaGroupSend::Single(photo_item(0))]
        );

        let pair = media_send_plan((0..2).map(photo_item).collect());
        assert!(matches!(&pair[..], [MediaGroupSend::Album(items)] if items.len() == 2));

        // Whole multiples of ten never yield a single-item album.
        for total in [10usize, 20] {
            let plan = media_send_plan((0..total).map(photo_item).collect());
            assert!(
                plan.iter()
                    .all(|group| matches!(group, MediaGroupSend::Album(_))),
                "{total} items must not produce a single-item album"
            );
        }

        // 11 and 21 leave a trailing remainder of one: it becomes a Single,
        // never an invalid one-item album.
        for total in [11usize, 21] {
            let plan = media_send_plan((0..total).map(photo_item).collect());
            assert_eq!(plan.len(), if total == 11 { 2 } else { 3 });
            assert!(matches!(plan.last(), Some(MediaGroupSend::Single(_))));
            assert!(
                plan[..plan.len() - 1].iter().all(
                    |group| matches!(group, MediaGroupSend::Album(items) if items.len() == 10)
                )
            );
        }
    }
}
