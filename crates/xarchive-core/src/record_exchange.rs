//! F3 canonical record-exchange model.
//!
//! One in-memory record model is shared by JSON and YAML. Both formats are two
//! serializations of the same model, so a JSON->model->YAML conversion preserves
//! every record field and both import paths run one validation pipeline. The
//! tweet payload reuses [`ArchiveMetadata`], the exact canonical DTO already
//! written to `tweet.json`, so no field is redefined or reinterpreted here.

use serde::{Deserialize, Serialize};

use crate::ArchiveMetadata;

const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

pub const DOWNLOAD_RECORDS_KIND: &str = "tw2tg.download-records";
pub const ARCHIVE_RECORDS_KIND: &str = "tw2tg.archive-records";
pub const RECORD_SCHEMA_VERSION: u32 = 1;

/// Stable idempotency key for a record across formats.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordIdentity {
    pub account_handle: String,
    pub tweet_id: String,
}

/// Canonical download record: the portable tweet metadata plus the account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadRecord {
    pub account_handle: String,
    pub metadata: ArchiveMetadata,
}

/// Canonical archive record: a download record plus the published final path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecord {
    pub account_handle: String,
    pub metadata: ArchiveMetadata,
    pub final_path: String,
}

impl DownloadRecord {
    pub fn identity(&self) -> RecordIdentity {
        RecordIdentity {
            account_handle: self.account_handle.clone(),
            tweet_id: self.metadata.tweet_id.clone(),
        }
    }
}

impl ArchiveRecord {
    pub fn identity(&self) -> RecordIdentity {
        RecordIdentity {
            account_handle: self.account_handle.clone(),
            tweet_id: self.metadata.tweet_id.clone(),
        }
    }

    pub fn as_download_record(&self) -> DownloadRecord {
        DownloadRecord {
            account_handle: self.account_handle.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordExchangeError {
    TooLarge,
    InvalidSchema {
        expected: u32,
        found: u32,
    },
    InvalidKind {
        expected: &'static str,
        found: String,
    },
    EmptyIdentity {
        index: usize,
    },
    DuplicateIdentity {
        identity: RecordIdentity,
    },
    Parse(String),
}

impl std::fmt::Display for RecordExchangeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("record exchange document exceeds 16 MiB"),
            Self::InvalidSchema { expected, found } => {
                write!(
                    formatter,
                    "expected schema_version {expected}, found {found}"
                )
            }
            Self::InvalidKind { expected, found } => {
                write!(formatter, "expected kind {expected}, found {found}")
            }
            Self::EmptyIdentity { index } => {
                write!(formatter, "record {index} has an empty identity")
            }
            Self::DuplicateIdentity { identity } => write!(
                formatter,
                "duplicate record identity {} / {}",
                identity.account_handle, identity.tweet_id
            ),
            Self::Parse(error) => write!(formatter, "invalid record exchange document: {error}"),
        }
    }
}

impl std::error::Error for RecordExchangeError {}

/// Strict versioned envelope for download records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadRecordsDocument {
    pub schema_version: u32,
    pub kind: String,
    pub records: Vec<DownloadRecord>,
}

/// Strict versioned envelope for archive records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecordsDocument {
    pub schema_version: u32,
    pub kind: String,
    pub records: Vec<ArchiveRecord>,
}

macro_rules! impl_document {
    ($doc:ty, $record:ty, $kind:expr) => {
        impl $doc {
            pub fn new(records: Vec<$record>) -> Self {
                Self {
                    schema_version: RECORD_SCHEMA_VERSION,
                    kind: $kind.into(),
                    records,
                }
            }

            fn validate(&self) -> Result<(), RecordExchangeError> {
                validate_envelope(self.schema_version, &self.kind, $kind)?;
                validate_identities(self.records.iter().map(|record| record.identity()))
            }

            pub fn to_json(&self) -> Result<String, RecordExchangeError> {
                serde_json::to_string_pretty(self)
                    .map_err(|error| RecordExchangeError::Parse(error.to_string()))
            }

            pub fn to_yaml(&self) -> Result<String, RecordExchangeError> {
                serde_yaml::to_string(self)
                    .map_err(|error| RecordExchangeError::Parse(error.to_string()))
            }

            pub fn from_json(raw: &str) -> Result<Self, RecordExchangeError> {
                bound(raw)?;
                let value: StrictValue = serde_json::from_str(raw)
                    .map_err(|error| RecordExchangeError::Parse(error.to_string()))?;
                validate_metadata_fields(&value.0)?;
                let document: Self = serde_json::from_str(raw)
                    .map_err(|error| RecordExchangeError::Parse(error.to_string()))?;
                document.validate()?;
                Ok(document)
            }

            pub fn from_yaml(raw: &str) -> Result<Self, RecordExchangeError> {
                bound(raw)?;
                let value: StrictValue = serde_yaml::from_str(raw)
                    .map_err(|error| RecordExchangeError::Parse(error.to_string()))?;
                validate_metadata_fields(&value.0)?;
                let document: Self = serde_yaml::from_str(raw)
                    .map_err(|error| RecordExchangeError::Parse(error.to_string()))?;
                document.validate()?;
                Ok(document)
            }
        }
    };
}

impl_document!(
    DownloadRecordsDocument,
    DownloadRecord,
    DOWNLOAD_RECORDS_KIND
);
impl_document!(ArchiveRecordsDocument, ArchiveRecord, ARCHIVE_RECORDS_KIND);

fn bound(raw: &str) -> Result<(), RecordExchangeError> {
    if raw.len() > MAX_DOCUMENT_BYTES {
        return Err(RecordExchangeError::TooLarge);
    }
    Ok(())
}

/// Recursive duplicate-key rejection, including optional nested metadata keys.
struct StrictValue(serde_json::Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a document without duplicate mapping keys")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!("duplicate field: {key}")));
                    }
                    values.insert(key, map.next_value::<StrictValue>()?.0);
                }
                Ok(StrictValue(values.into()))
            }
            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                mut seq: S,
            ) -> Result<Self::Value, S::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<StrictValue>()? {
                    values.push(value.0);
                }
                Ok(StrictValue(values.into()))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Ok(StrictValue(
                    serde_json::Number::from_f64(value)
                        .ok_or_else(|| E::custom("non-finite number"))?
                        .into(),
                ))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(serde_json::Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                self.visit_unit()
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// Enforce the exchange boundary without tightening legacy metadata readers.
fn validate_metadata_fields(document: &serde_json::Value) -> Result<(), RecordExchangeError> {
    fn fields(value: &serde_json::Value, allowed: &[&str]) -> Result<(), RecordExchangeError> {
        if let Some(object) = value.as_object() {
            for key in object.keys() {
                if !allowed.contains(&key.as_str()) {
                    return Err(RecordExchangeError::Parse(format!(
                        "unknown metadata field: {key}"
                    )));
                }
            }
        }
        Ok(())
    }
    if let Some(records) = document.get("records").and_then(|v| v.as_array()) {
        for record in records {
            let metadata = &record["metadata"];
            fields(
                metadata,
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
            fields(
                &metadata["author"],
                &["user_id", "username", "display_name"],
            )?;
            fields(
                &metadata["quoted_tweet"],
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
            if let Some(media) = metadata["media"].as_array() {
                for item in media {
                    fields(
                        item,
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
            }
        }
    }
    Ok(())
}

fn validate_envelope(
    schema_version: u32,
    kind: &str,
    expected_kind: &'static str,
) -> Result<(), RecordExchangeError> {
    if schema_version != RECORD_SCHEMA_VERSION {
        return Err(RecordExchangeError::InvalidSchema {
            expected: RECORD_SCHEMA_VERSION,
            found: schema_version,
        });
    }
    if kind != expected_kind {
        return Err(RecordExchangeError::InvalidKind {
            expected: expected_kind,
            found: kind.into(),
        });
    }
    Ok(())
}

fn validate_identities(
    identities: impl Iterator<Item = RecordIdentity>,
) -> Result<(), RecordExchangeError> {
    let mut seen = std::collections::HashSet::new();
    for (index, identity) in identities.enumerate() {
        if identity.account_handle.trim().is_empty() || identity.tweet_id.trim().is_empty() {
            return Err(RecordExchangeError::EmptyIdentity { index });
        }
        if !seen.insert(identity.clone()) {
            return Err(RecordExchangeError::DuplicateIdentity { identity });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchiveAuthor, ArchiveMedia, ArchiveQuotedTweet};

    fn sample_metadata(tweet_id: &str) -> ArchiveMetadata {
        ArchiveMetadata {
            schema_version: 1,
            tweet_id: tweet_id.into(),
            url: format!("https://x.test/i/status/{tweet_id}"),
            tweet_type: "original".into(),
            author: ArchiveAuthor {
                user_id: Some("42".into()),
                username: Some("alice".into()),
                display_name: Some("Alice".into()),
            },
            created_at: Some("2026-01-01T00:00:00Z".into()),
            text: "hello".into(),
            media: vec![ArchiveMedia {
                index: 1,
                media_id: Some("m1".into()),
                media_type: "photo".into(),
                file: "media/1.jpg".into(),
                mime_type: Some("image/jpeg".into()),
                size_bytes: 1234,
                sha256: "abc".into(),
            }],
            archived_at: "2026-01-02T00:00:00Z".into(),
            reply_to: None,
            quoted_tweet: Some(ArchiveQuotedTweet {
                tweet_id: "9".into(),
                url: "https://x.test/i/status/9".into(),
                username: None,
                display_name: None,
                user_id: None,
                text: None,
                created_at: None,
                tweet_type: None,
            }),
        }
    }

    fn sample_download() -> DownloadRecord {
        DownloadRecord {
            account_handle: "alice".into(),
            metadata: sample_metadata("111"),
        }
    }

    #[test]
    fn download_records_round_trip_json_and_yaml_to_same_model() {
        let document = DownloadRecordsDocument::new(vec![sample_download()]);
        let json = document.to_json().unwrap();
        let yaml = document.to_yaml().unwrap();

        let from_json = DownloadRecordsDocument::from_json(&json).unwrap();
        let from_yaml = DownloadRecordsDocument::from_yaml(&yaml).unwrap();

        assert_eq!(from_json, document);
        assert_eq!(from_yaml, document);

        // Cross-format: re-serializing either parsed model to the other format
        // yields identical canonical bytes, proving field preservation.
        assert_eq!(from_yaml.to_json().unwrap(), from_json.to_json().unwrap());
        assert_eq!(from_json.to_yaml().unwrap(), from_yaml.to_yaml().unwrap());
    }

    #[test]
    fn archive_records_round_trip_and_preserve_final_path() {
        let record = ArchiveRecord {
            account_handle: "alice".into(),
            metadata: sample_metadata("222"),
            final_path: "Tweets/222".into(),
        };
        let document = ArchiveRecordsDocument::new(vec![record.clone()]);
        let from_json = ArchiveRecordsDocument::from_json(&document.to_json().unwrap()).unwrap();
        let from_yaml = ArchiveRecordsDocument::from_yaml(&document.to_yaml().unwrap()).unwrap();
        assert_eq!(from_json, document);
        assert_eq!(from_yaml, document);
        assert_eq!(from_json.records[0].final_path, "Tweets/222");
        assert_eq!(record.as_download_record().identity(), record.identity());
    }

    #[test]
    fn rejects_unknown_kind_schema_and_duplicate_keys() {
        let document = DownloadRecordsDocument::new(vec![sample_download()]);
        let raw = document.to_json().unwrap();

        let wrong_kind = raw.replace(DOWNLOAD_RECORDS_KIND, "tw2tg.other");
        assert!(matches!(
            DownloadRecordsDocument::from_json(&wrong_kind),
            Err(RecordExchangeError::InvalidKind { .. })
        ));

        let wrong_schema = raw.replace(
            &format!("\"schema_version\": {}", RECORD_SCHEMA_VERSION),
            "\"schema_version\": 2",
        );
        assert!(matches!(
            DownloadRecordsDocument::from_json(&wrong_schema),
            Err(RecordExchangeError::InvalidSchema { .. })
        ));

        let unknown = raw.replacen('{', "{\"extra\":true,", 1);
        assert!(DownloadRecordsDocument::from_json(&unknown).is_err());

        let duplicate = raw.replacen(
            &format!("\"schema_version\": {}", RECORD_SCHEMA_VERSION),
            &format!(
                "\"schema_version\": {},\"schema_version\": {}",
                RECORD_SCHEMA_VERSION, RECORD_SCHEMA_VERSION
            ),
            1,
        );
        assert!(DownloadRecordsDocument::from_json(&duplicate).is_err());
        let nested_duplicate = raw.replacen("\"username\":", "\"username\":null,\"username\":", 1);
        assert!(DownloadRecordsDocument::from_json(&nested_duplicate).is_err());
        let yaml = document.to_yaml().unwrap();
        let duplicate_yaml = yaml.replacen("username:", "username: null\n      username:", 1);
        assert!(DownloadRecordsDocument::from_yaml(&duplicate_yaml).is_err());
        let mut nested = serde_json::to_value(&document).unwrap();
        nested["records"][0]["metadata"]["token"] = "private".into();
        assert!(DownloadRecordsDocument::from_json(&nested.to_string()).is_err());
        assert!(
            DownloadRecordsDocument::from_yaml(&serde_yaml::to_string(&nested).unwrap()).is_err()
        );
    }

    #[test]
    fn duplicate_identities_are_rejected_and_identity_is_derived() {
        let record = sample_download();
        let duplicate = record.clone();
        let raw = serde_json::to_string_pretty(&DownloadRecordsDocument {
            schema_version: RECORD_SCHEMA_VERSION,
            kind: DOWNLOAD_RECORDS_KIND.into(),
            records: vec![record, duplicate],
        })
        .unwrap();
        assert!(matches!(
            DownloadRecordsDocument::from_json(&raw),
            Err(RecordExchangeError::DuplicateIdentity { .. })
        ));

        let mut changed = sample_download();
        changed.metadata.tweet_id = "999".into();
        assert_ne!(changed.identity(), sample_download().identity());
    }

    #[test]
    fn archive_and_download_kinds_are_distinct() {
        // A structurally valid archive document with a replaced kind must be
        // rejected precisely as an invalid kind (not a generic parse failure).
        let archive = ArchiveRecordsDocument::new(vec![ArchiveRecord {
            account_handle: "alice".into(),
            metadata: sample_metadata("333"),
            final_path: "Tweets/333".into(),
        }]);
        let wrong_kind = archive
            .to_json()
            .unwrap()
            .replace(ARCHIVE_RECORDS_KIND, "tw2tg.other");
        assert!(matches!(
            ArchiveRecordsDocument::from_json(&wrong_kind),
            Err(RecordExchangeError::InvalidKind { .. })
        ));

        // A download document parsed as an archive document is still rejected
        // (its records lack the required archive-only `final_path` field),
        // which proves the two envelopes are not interchangeable.
        let download = DownloadRecordsDocument::new(vec![sample_download()]);
        assert!(ArchiveRecordsDocument::from_json(&download.to_json().unwrap()).is_err());
    }
}
