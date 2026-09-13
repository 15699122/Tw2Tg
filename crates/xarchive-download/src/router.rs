use crate::model::{DownloadResult, DownloadRoute};
use crate::{AddUriRequest, DownloadError, TransferId};

/// Error information returned by the Python gallery-dl adapter.
///
/// The router deliberately receives only the stable error code and message
/// from the sidecar protocol instead of depending on Python implementation
/// details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryDlFailure {
    pub code: String,
    pub message: String,
}

impl GalleryDlFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    fn can_fallback_to_aria2(&self) -> bool {
        self.code == "EXTRACT_OR_DOWNLOAD_FAILED"
    }
}

/// The final error after the router has exhausted the applicable backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadRouterError {
    GalleryDl(GalleryDlFailure),
    Aria2(DownloadError),
    GalleryDlThenAria2 {
        gallery: GalleryDlFailure,
        aria2: DownloadError,
    },
    Aria2NotConfigured,
}

impl std::fmt::Display for DownloadRouterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GalleryDl(error) => {
                write!(formatter, "gallery-dl {}: {}", error.code, error.message)
            }
            Self::Aria2(error) => write!(formatter, "aria2 download failed: {error}"),
            Self::GalleryDlThenAria2 { gallery, aria2 } => write!(
                formatter,
                "gallery-dl {}: {}; aria2 download failed: {aria2}",
                gallery.code, gallery.message
            ),
            Self::Aria2NotConfigured => formatter.write_str("aria2 fallback is not configured"),
        }
    }
}

impl std::error::Error for DownloadRouterError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadRouterConfig {
    pub allow_aria2_fallback: bool,
}

impl Default for DownloadRouterConfig {
    fn default() -> Self {
        Self {
            allow_aria2_fallback: true,
        }
    }
}

/// Business routing policy for media downloads.
///
/// gallery-dl remains the default extractor and downloader. aria2 is an
/// explicit fallback only: callers must enable it and provide an `AddUri`
/// request produced from a fresh, authenticated media URL. The router does
/// not manufacture or cache URLs, so a caller can re-extract a URL before
/// invoking the aria2 closure.
#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadRouter {
    config: DownloadRouterConfig,
}

impl DownloadRouter {
    pub const fn new(config: DownloadRouterConfig) -> Self {
        Self { config }
    }

    pub const fn config(self) -> DownloadRouterConfig {
        self.config
    }

    pub fn execute<G, A>(
        &self,
        gallery_download: G,
        aria2_request: Option<AddUriRequest>,
        aria2_download: A,
    ) -> Result<DownloadResult, DownloadRouterError>
    where
        G: FnOnce() -> Result<(), GalleryDlFailure>,
        A: FnOnce(AddUriRequest) -> Result<TransferId, DownloadError>,
    {
        match gallery_download() {
            Ok(()) => Ok(DownloadResult {
                route: DownloadRoute::GalleryDl,
                transfer_id: None,
            }),
            Err(gallery) if gallery.can_fallback_to_aria2() && self.config.allow_aria2_fallback => {
                let request = aria2_request.ok_or(DownloadRouterError::Aria2NotConfigured)?;
                match aria2_download(request) {
                    Ok(transfer_id) => Ok(DownloadResult {
                        route: DownloadRoute::Aria2,
                        transfer_id: Some(transfer_id),
                    }),
                    Err(aria2) => Err(DownloadRouterError::GalleryDlThenAria2 { gallery, aria2 }),
                }
            }
            Err(gallery) => Err(DownloadRouterError::GalleryDl(gallery)),
        }
    }
}
