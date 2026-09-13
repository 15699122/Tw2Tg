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
