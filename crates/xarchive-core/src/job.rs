//! Archive job state machine.

/// Persisted state of an archive job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobState {
    Queued,
    Validating,
    MetadataReady,
    TgMetadataSending,
    TgMetadataSent,
    Downloading,
    Downloaded,
    TgMediaUploading,
    Complete,
    Interrupted,
    AuthRequired,
    Failed,
    Cancelled,
}

impl JobState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::Validating => "VALIDATING",
            Self::MetadataReady => "METADATA_READY",
            Self::TgMetadataSending => "TG_METADATA_SENDING",
            Self::TgMetadataSent => "TG_METADATA_SENT",
            Self::Downloading => "DOWNLOADING",
            Self::Downloaded => "DOWNLOADED",
            Self::TgMediaUploading => "TG_MEDIA_UPLOADING",
            Self::Complete => "COMPLETE",
            Self::Interrupted => "INTERRUPTED",
            Self::AuthRequired => "AUTH_REQUIRED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn parse(value: &str) -> Result<Self, JobStateParseError> {
        match value {
            "QUEUED" => Ok(Self::Queued),
            "VALIDATING" => Ok(Self::Validating),
            "METADATA_READY" => Ok(Self::MetadataReady),
            "TG_METADATA_SENDING" => Ok(Self::TgMetadataSending),
            "TG_METADATA_SENT" => Ok(Self::TgMetadataSent),
            "DOWNLOADING" => Ok(Self::Downloading),
            "DOWNLOADED" => Ok(Self::Downloaded),
            "TG_MEDIA_UPLOADING" => Ok(Self::TgMediaUploading),
            "COMPLETE" => Ok(Self::Complete),
            "INTERRUPTED" => Ok(Self::Interrupted),
            "AUTH_REQUIRED" => Ok(Self::AuthRequired),
            "FAILED" => Ok(Self::Failed),
            "CANCELLED" => Ok(Self::Cancelled),
            _ => Err(JobStateParseError),
        }
    }

    /// Returns whether the job may still be claimed by a worker.
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Queued
                | Self::Validating
                | Self::MetadataReady
                | Self::TgMetadataSending
                | Self::TgMetadataSent
                | Self::Downloading
                | Self::Downloaded
                | Self::TgMediaUploading
        )
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Cancelled)
    }

    /// Validate a state transition before it is persisted.
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Validating)
                | (Self::Queued, Self::Cancelled)
                | (Self::Validating, Self::MetadataReady)
                | (Self::Validating, Self::AuthRequired)
                | (Self::Validating, Self::Failed)
                | (Self::MetadataReady, Self::TgMetadataSending)
                | (Self::MetadataReady, Self::Downloading)
                | (Self::MetadataReady, Self::Cancelled)
                | (Self::TgMetadataSending, Self::TgMetadataSent)
                | (Self::TgMetadataSending, Self::Failed)
                | (Self::TgMetadataSending, Self::Interrupted)
                | (Self::TgMetadataSent, Self::Downloading)
                | (Self::TgMetadataSent, Self::Failed)
                | (Self::TgMetadataSent, Self::Interrupted)
                | (Self::Downloading, Self::Downloaded)
                | (Self::Downloading, Self::AuthRequired)
                | (Self::Downloading, Self::Failed)
                | (Self::Downloading, Self::Interrupted)
                | (Self::Downloading, Self::Cancelled)
                | (Self::Downloaded, Self::TgMediaUploading)
                | (Self::Downloaded, Self::Complete)
                | (Self::Downloaded, Self::Failed)
                | (Self::TgMediaUploading, Self::Complete)
                | (Self::TgMediaUploading, Self::Failed)
                | (Self::TgMediaUploading, Self::Interrupted)
                | (Self::Interrupted, Self::Validating)
                | (Self::Interrupted, Self::Downloading)
                | (Self::Interrupted, Self::Cancelled)
                | (Self::AuthRequired, Self::Validating)
                | (Self::AuthRequired, Self::Cancelled)
                | (Self::Failed, Self::Validating)
                | (Self::Failed, Self::Downloading)
                | (Self::Failed, Self::TgMediaUploading)
                | (Self::Failed, Self::Cancelled)
        )
    }

    pub fn transition_to(self, next: Self) -> Result<Self, JobStateError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(JobStateError {
                from: self,
                to: next,
            })
        }
    }
}

pub const fn is_active_state(state: JobState) -> bool {
    state.is_active()
}

pub const fn is_terminal_state(state: JobState) -> bool {
    state.is_terminal()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobStateError {
    pub from: JobState,
    pub to: JobState,
}

impl std::fmt::Display for JobStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "invalid job transition: {:?} -> {:?}",
            self.from, self.to
        )
    }
}

impl std::error::Error for JobStateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobStateParseError;

impl std::fmt::Display for JobStateParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("unknown persisted job state")
    }
}

impl std::error::Error for JobStateParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_archive_flow() {
        let states = [
            JobState::Queued,
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
            JobState::Downloaded,
            JobState::Complete,
        ];

        for pair in states.windows(2) {
            assert_eq!(pair[0].transition_to(pair[1]), Ok(pair[1]));
        }
    }

    #[test]
    fn permits_retry_without_recreating_the_job() {
        assert_eq!(
            JobState::Failed.transition_to(JobState::Downloading),
            Ok(JobState::Downloading)
        );
        assert_eq!(
            JobState::Interrupted.transition_to(JobState::Validating),
            Ok(JobState::Validating)
        );
    }

    #[test]
    fn rejects_skipping_required_steps() {
        let error = JobState::Queued
            .transition_to(JobState::Complete)
            .expect_err("queued cannot be complete");
        assert_eq!(error.from, JobState::Queued);
        assert_eq!(error.to, JobState::Complete);
    }

    #[test]
    fn classifies_active_and_terminal_states() {
        assert!(JobState::Downloading.is_active());
        assert!(!JobState::Failed.is_active());
        assert!(JobState::Complete.is_terminal());
        assert!(!JobState::Interrupted.is_terminal());
    }
}
