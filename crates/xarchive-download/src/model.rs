use crate::{AddUriRequest, DownloadError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferId(String);

impl TransferId {
    pub fn new(value: impl Into<String>) -> Result<Self, DownloadError> {
        let value = value.into();
        if value.is_empty() {
            return Err(DownloadError::InvalidTransferId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    Waiting,
    Active,
    Paused,
    Complete,
    Error,
    Removed,
}

impl TransferState {
    pub fn from_aria2(value: &str) -> Result<Self, DownloadError> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "complete" => Ok(Self::Complete),
            "error" => Ok(Self::Error),
            "removed" => Ok(Self::Removed),
            other => Err(DownloadError::UnknownState(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferStatus {
    pub id: TransferId,
    pub state: TransferState,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
    pub download_speed: u64,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub files: Vec<TransferFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferFile {
    pub path: String,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub trait DownloadBackend {
    fn add_uri(&self, request: AddUriRequest) -> Result<TransferId, DownloadError>;
    fn status(&self, id: &TransferId) -> Result<TransferStatus, DownloadError>;
    fn pause(&self, id: &TransferId) -> Result<TransferId, DownloadError>;
    fn resume(&self, id: &TransferId) -> Result<TransferId, DownloadError>;
    fn cancel(&self, id: &TransferId) -> Result<TransferId, DownloadError>;
}

/// The backend selected for a completed download attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadRoute {
    GalleryDl,
    Aria2,
}

/// A stable, backend-neutral result returned by the download router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadResult {
    pub route: DownloadRoute,
    pub transfer_id: Option<TransferId>,
}
