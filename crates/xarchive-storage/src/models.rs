use serde::{Deserialize, Serialize};
use xarchive_core::JobState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobSummary {
    pub job_id: String,
    pub tweet_id: String,
    pub tweet_type: String,
    pub state: JobState,
    pub created_at: String,
    pub updated_at: String,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct JobMetrics {
    pub total: u64,
    pub active: u64,
    pub completed: u64,
    pub failed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobDownloadMetrics {
    pub job_id: String,
    pub backend: Option<String>,
    pub download_started_at: Option<String>,
    pub download_finished_at: Option<String>,
    pub task_finished_at: Option<String>,
    pub downloaded_bytes: Option<u64>,
    pub download_duration_ms: Option<u64>,
    pub attempt_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PagedJobs {
    pub jobs: Vec<JobSummary>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
}

impl JobMetrics {
    pub fn error_rate_percent(self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.failed as f64 / self.total as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobEventRecord {
    pub event_type: String,
    pub payload_json: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserSummary {
    pub user_id: String,
    pub stable_directory_name: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserNameSummary {
    pub username: String,
    pub display_name: Option<String>,
    pub source: String,
    pub observed_at: String,
}

/// Direct reply/quote relationship columns for one archived tweet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweetRelationships {
    pub reply_to_tweet_id: Option<String>,
    pub quoted_tweet_id: Option<String>,
}

/// Committed archive directory plus relative media file names for one Tweet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweetArchiveFacts {
    pub archive_directory: String,
    /// One entry per recorded media row, ordered by `media_index`.
    pub media: Vec<ArchivedMediaFact>,
}

/// Durable archive-to-Telegram handoff record. A record is intentionally
/// separate from the outbox transaction; startup recovery advances it
/// idempotently until it is queued or explicitly skipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramArchiveIntentRecord {
    pub job_id: String,
    pub tweet_row_id: i64,
    pub archive_directory: String,
    pub state: String,
    pub metadata_text: Option<String>,
    pub media_json: String,
    pub bot_identity: Option<String>,
    pub chat_id: Option<String>,
    pub message_thread_id: Option<i64>,
    pub config_revision: Option<i64>,
    pub plan_version: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl TweetArchiveFacts {
    /// Recorded media file paths, in `media_index` order.
    pub fn media_paths(&self) -> impl Iterator<Item = &str> {
        self.media.iter().map(|media| media.relative_path.as_str())
    }
}

/// One recorded media row of an archived tweet.
///
/// Completeness is decided from these durable facts, never from a bare
/// "the file exists" check: identity and size are part of the row, and
/// `sha256` is available for callers that ask for digest verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchivedMediaFact {
    pub media_index: u32,
    pub relative_path: String,
    pub media_id: Option<String>,
    pub media_type: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
}

/// One `settings_meta` row exposed to callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingEntry {
    pub key: String,
    pub value_json: String,
    pub updated_at: String,
}

/// Current user profile state read from the database for profile files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserProfileSnapshot {
    pub user_id: String,
    pub stable_directory_name: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<UserNameSummary>,
}

/// Portable `profile.json` payload stored in the stable user directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfileFile {
    pub schema_version: u32,
    pub user_id: String,
    pub stable_directory_name: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<UserNameSummary>,
}
