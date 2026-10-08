//! Shared filename renderer for archive media output.

use crate::{BatchNamingMode, BatchOutputSettings, StorageError};
use std::collections::HashSet;

const RESERVED_DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];
const INTERNAL_ARCHIVE_FILES: &[&str] = &["tweet.json", "tweet.txt"];
const MAX_FILENAME_BYTES: usize = 255;

fn invalid(message: impl Into<String>) -> StorageError {
    StorageError::InvalidMetadata(message.into())
}

fn render_stem(
    template: &str,
    username: Option<&str>,
    tweet_id: &str,
    index: u32,
) -> Result<String, StorageError> {
    let mut output = String::with_capacity(template.len() + 16);
    let mut chars = template.chars();
    while let Some(character) = chars.next() {
        match character {
            '{' => {
                let mut token = String::new();
                let mut closed = false;
                for character in chars.by_ref() {
                    if character == '}' {
                        closed = true;
                        break;
                    }
                    token.push(character);
                }
                if !closed {
                    return Err(invalid("filename template has an unterminated token"));
                }
                match token.as_str() {
                    "username" => output.push_str(
                        username
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .ok_or_else(|| invalid("filename template requires {username}"))?,
                    ),
                    "tweet_id" => output.push_str(tweet_id),
                    "index" => output.push_str(&format!("{index:02}")),
                    other => {
                        return Err(invalid(format!(
                            "unknown filename template token: {{{other}}}"
                        )));
                    }
                }
            }
            '}' => {
                return Err(invalid(
                    "filename template contains an unmatched closing brace",
                ));
            }
            character => output.push(character),
        }
    }
    Ok(output)
}

fn extension_of(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => extension,
        _ => "",
    }
}

fn validate_component(name: &str) -> Result<(), StorageError> {
    if name.is_empty() || name.len() > MAX_FILENAME_BYTES || matches!(name, "." | "..") {
        return Err(invalid(
            "rendered filename is empty, traversal, or too long",
        ));
    }
    if name.ends_with('.')
        || name.ends_with(' ')
        || name.chars().any(|ch| {
            ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
    {
        return Err(invalid("rendered filename contains disallowed characters"));
    }
    let device = name.split('.').next().unwrap_or_default();
    if RESERVED_DEVICE_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(device))
    {
        return Err(invalid(format!(
            "rendered filename uses reserved device name {device}"
        )));
    }
    Ok(())
}

/// Return final media-relative paths using the output settings snapshot.
pub fn render_media_filenames(
    original_relative_paths: &[String],
    username: Option<&str>,
    tweet_id: &str,
    settings: &BatchOutputSettings,
) -> Result<Vec<String>, StorageError> {
    if matches!(settings.naming_mode, BatchNamingMode::Original) {
        let mut seen = HashSet::with_capacity(original_relative_paths.len());
        for path in original_relative_paths {
            validate_relative_media_path(path)?;
            if !seen.insert(path.to_lowercase()) {
                return Err(invalid(format!("media path {path} is not unique")));
            }
        }
        return Ok(original_relative_paths.to_vec());
    }
    if settings.filename_template.trim().is_empty() {
        return Err(invalid("filename template is required in template mode"));
    }
    let mut rendered = Vec::with_capacity(original_relative_paths.len());
    let mut seen = HashSet::with_capacity(original_relative_paths.len());
    for (position, original) in original_relative_paths.iter().enumerate() {
        let stem = render_stem(
            &settings.filename_template,
            username,
            tweet_id,
            position as u32 + 1,
        )?;
        let extension = extension_of(original);
        let name = if extension.is_empty() {
            stem
        } else {
            format!("{stem}.{extension}")
        };
        validate_component(&name)?;
        if INTERNAL_ARCHIVE_FILES
            .iter()
            .any(|internal| internal.eq_ignore_ascii_case(&name))
            || name.eq_ignore_ascii_case(".xarchive-recovery.json")
        {
            return Err(invalid(format!(
                "rendered filename collides with internal file {name}"
            )));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(invalid(format!("rendered filename {name} is not unique")));
        }
        rendered.push(name);
    }
    Ok(rendered)
}

fn validate_relative_media_path(path: &str) -> Result<(), StorageError> {
    let components = path.split('/').collect::<Vec<_>>();
    if path.is_empty()
        || path.contains('\\')
        || path.starts_with('/')
        || path.contains(':')
        || components
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err(invalid(format!(
            "media path {path} is not a safe relative path"
        )));
    }
    for component in components {
        validate_component(component)?;
    }
    Ok(())
}

/// Apply the shared path rendering to a metadata copy without touching any files.
pub fn render_archive_metadata_paths(
    metadata: &xarchive_core::ArchiveMetadata,
    username: Option<&str>,
    settings: &BatchOutputSettings,
) -> Result<xarchive_core::ArchiveMetadata, StorageError> {
    let original = metadata
        .media
        .iter()
        .map(|media| media.file.clone())
        .collect::<Vec<_>>();
    let rendered = render_media_filenames(&original, username, &metadata.tweet_id, settings)?;
    let mut output = metadata.clone();
    for (media, path) in output.media.iter_mut().zip(rendered) {
        media.file = path;
    }
    Ok(output)
}

/// Return a renamed metadata copy and the source-to-destination mapping used by
/// the archive commit. Paths are flat filenames by contract.
pub fn plan_archive_media_renames(
    metadata: &xarchive_core::ArchiveMetadata,
    username: Option<&str>,
    settings: &BatchOutputSettings,
) -> Result<(xarchive_core::ArchiveMetadata, Vec<(String, String)>), StorageError> {
    let output = render_archive_metadata_paths(metadata, username, settings)?;
    let mapping = metadata
        .media
        .iter()
        .zip(&output.media)
        .map(|(source, destination)| (source.file.clone(), destination.file.clone()))
        .collect();
    Ok((output, mapping))
}

#[cfg(test)]
mod tests {
    use super::{
        plan_archive_media_renames, render_archive_metadata_paths, render_media_filenames,
    };
    use crate::{BatchNamingMode, BatchOutputSettings, StorageError};

    fn template_settings(template: &str) -> BatchOutputSettings {
        BatchOutputSettings {
            naming_mode: BatchNamingMode::Template,
            filename_template: template.into(),
            ..BatchOutputSettings::default()
        }
    }

    #[test]
    fn renders_template_with_source_extensions_and_one_based_indices() {
        let names = render_media_filenames(
            &["original/a.PNG".into(), "b".into()],
            Some("alice"),
            "123",
            &template_settings("{username}_{tweet_id}_{index}"),
        )
        .expect("render names");
        assert_eq!(names, ["alice_123_01.PNG", "alice_123_02"]);
    }

    #[test]
    fn rejects_missing_username_and_unknown_tokens() {
        let paths = ["photo.jpg".into()];
        assert!(matches!(
            render_media_filenames(
                &paths,
                None,
                "123",
                &template_settings("{username}_{tweet_id}")
            ),
            Err(StorageError::InvalidMetadata(_))
        ));
        assert!(matches!(
            render_media_filenames(
                &paths,
                Some("alice"),
                "123",
                &template_settings("{unknown}")
            ),
            Err(StorageError::InvalidMetadata(_))
        ));
    }

    #[test]
    fn rejects_reserved_names_internal_collisions_and_case_insensitive_duplicates() {
        for (template, source) in [("CON", "a.jpg"), ("tweet", "a.json")] {
            assert!(
                render_media_filenames(
                    &[source.into()],
                    Some("alice"),
                    "123",
                    &template_settings(template)
                )
                .is_err()
            );
        }
        assert!(
            render_media_filenames(
                &["a.jpg".into(), "b.jpg".into()],
                Some("alice"),
                "123",
                &template_settings("{tweet_id}")
            )
            .is_err()
        );
        assert!(
            render_media_filenames(
                &["a.jpg".into(), "b.JPG".into()],
                Some("alice"),
                "123",
                &template_settings("Photo")
            )
            .is_err()
        );
    }

    #[test]
    fn original_mode_preserves_paths_without_rewriting_them() {
        let paths = ["nested/original.jpg".into()];
        assert_eq!(
            render_media_filenames(&paths, None, "123", &BatchOutputSettings::default()).unwrap(),
            paths
        );
    }

    #[test]
    fn original_mode_rejects_unsafe_or_colliding_paths() {
        for paths in [
            vec!["../outside.jpg".into()],
            vec!["nested//photo.jpg".into()],
            vec!["A.jpg".into(), "a.JPG".into()],
        ] {
            assert!(
                render_media_filenames(&paths, None, "123", &BatchOutputSettings::default())
                    .is_err()
            );
        }
    }

    #[test]
    fn metadata_path_renderer_changes_only_media_paths() {
        let metadata = xarchive_core::ArchiveMetadata {
            schema_version: 1,
            tweet_id: "123".into(),
            url: "https://example.test/123".into(),
            tweet_type: "post".into(),
            author: xarchive_core::ArchiveAuthor {
                user_id: None,
                username: Some("alice".into()),
                display_name: Some("Alice".into()),
            },
            created_at: Some("now".into()),
            text: "private text".into(),
            media: vec![xarchive_core::ArchiveMedia {
                index: 1,
                media_id: None,
                media_type: "image".into(),
                file: "original.jpg".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 3,
                sha256: "a".repeat(64),
            }],
            archived_at: "now".into(),
            reply_to: None,
            quoted_tweet: None,
        };
        let output = render_archive_metadata_paths(
            &metadata,
            Some("alice"),
            &template_settings("{tweet_id}_{index}"),
        )
        .unwrap();
        assert_eq!(output.media[0].file, "123_01.jpg");
        assert_eq!(output.text, metadata.text);
        assert_eq!(output.author, metadata.author);
        assert_eq!(output.media[0].sha256, metadata.media[0].sha256);
        let (planned, mapping) = plan_archive_media_renames(
            &metadata,
            Some("alice"),
            &template_settings("{tweet_id}_{index}"),
        )
        .unwrap();
        assert_eq!(planned, output);
        assert_eq!(mapping, [("original.jpg".into(), "123_01.jpg".into())]);
    }
}
