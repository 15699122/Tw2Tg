//! One-shot URL refresh orchestration for aria2 transfers.

use std::collections::HashMap;

use crate::{MediaTransferPlan, TransferFailure, TransferFailureCode, TransferOutcome};

/// Stable error returned when refreshed extraction no longer describes the
/// same media collection.
pub const EXTRACTION_RESULT_CHANGED: &str = "EXTRACTION_RESULT_CHANGED";

/// Result of a refresh-aware transfer attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshTransferResult {
    pub outcomes: Vec<TransferOutcome>,
    pub refreshed: bool,
}

/// Coordinates extraction refresh and transfer retry without depending on
/// Tauri, SQLite, Sidecar process management, or a concrete aria2 client.
pub struct RefreshCoordinator<Extract, Transfer> {
    extract: Extract,
    transfer: Transfer,
}

impl<Extract, Transfer> RefreshCoordinator<Extract, Transfer> {
    pub fn new(extract: Extract, transfer: Transfer) -> Self {
        Self { extract, transfer }
    }
}

impl<Extract, Transfer, E> RefreshCoordinator<Extract, Transfer>
where
    Extract: FnMut(&MediaTransferPlan) -> Result<MediaTransferPlan, E>,
    Transfer: FnMut(&MediaTransferPlan) -> Result<Vec<TransferOutcome>, TransferFailure>,
    E: std::fmt::Display,
{
    /// Execute once and refresh at most once for an expired/forbidden URL.
    pub fn execute(
        &mut self,
        initial: &MediaTransferPlan,
    ) -> Result<RefreshTransferResult, TransferFailure> {
        let first = (self.transfer)(initial);
        let failure = match first {
            Ok(outcomes) => {
                return Ok(RefreshTransferResult {
                    outcomes,
                    refreshed: false,
                });
            }
            Err(failure) => failure,
        };

        if failure.code != TransferFailureCode::ExpiredUrl {
            return Err(failure);
        }

        let refreshed = (self.extract)(initial).map_err(|error| TransferFailure {
            media_id: failure.media_id.clone(),
            code: TransferFailureCode::Failed,
            message: format!("refresh extraction failed: {error}"),
            aria2_error_code: failure.aria2_error_code.clone(),
        })?;
        ensure_same_media_collection(initial, &refreshed).map_err(|message| TransferFailure {
            media_id: failure.media_id.clone(),
            code: TransferFailureCode::Failed,
            message,
            aria2_error_code: None,
        })?;

        let outcomes = (self.transfer)(&refreshed)?;
        Ok(RefreshTransferResult {
            outcomes,
            refreshed: true,
        })
    }
}

fn ensure_same_media_collection(
    before: &MediaTransferPlan,
    after: &MediaTransferPlan,
) -> Result<(), String> {
    let before_ids: HashMap<&str, &str> = before
        .items
        .iter()
        .map(|item| (item.media_id.as_str(), item.filename.as_str()))
        .collect();
    let after_ids: HashMap<&str, &str> = after
        .items
        .iter()
        .map(|item| (item.media_id.as_str(), item.filename.as_str()))
        .collect();
    if before_ids != after_ids {
        return Err(EXTRACTION_RESULT_CHANGED.to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TransferFailureCode, TransferId, TransferPlanItem};

    fn plan(ids: &[(&str, &str)]) -> MediaTransferPlan {
        MediaTransferPlan {
            items: ids
                .iter()
                .map(|(id, filename)| TransferPlanItem {
                    media_id: (*id).into(),
                    url: format!("https://cdn.example/{filename}"),
                    filename: (*filename).into(),
                    directory: "/tmp/staging".into(),
                    headers: vec![],
                    media_type: "photo".into(),
                    mime_type: None,
                })
                .collect(),
        }
    }

    fn expired() -> TransferFailure {
        TransferFailure {
            media_id: Some("media-1".into()),
            code: TransferFailureCode::ExpiredUrl,
            message: "forbidden".into(),
            aria2_error_code: Some("403".into()),
        }
    }

    fn outcome() -> Vec<TransferOutcome> {
        vec![TransferOutcome {
            media_id: "media-1".into(),
            transfer_id: TransferId::new("gid-2").unwrap(),
            path: Some("/tmp/staging/01.jpg".into()),
            total_bytes: Some(10),
        }]
    }

    #[test]
    fn refreshes_once_and_retries_with_new_plan() {
        let initial = plan(&[("media-1", "01.jpg")]);
        let refreshed = plan(&[("media-1", "01.jpg")]);
        let mut extract_calls = 0;
        let mut transfer_calls = 0;
        let mut coordinator = RefreshCoordinator::new(
            |_: &MediaTransferPlan| {
                extract_calls += 1;
                Ok::<_, &str>(refreshed.clone())
            },
            |_: &MediaTransferPlan| {
                transfer_calls += 1;
                if transfer_calls == 1 {
                    Err(expired())
                } else {
                    Ok(outcome())
                }
            },
        );
        let result = coordinator.execute(&initial).expect("refreshed transfer");
        assert!(result.refreshed);
        assert_eq!(extract_calls, 1);
        assert_eq!(transfer_calls, 2);
    }

    #[test]
    fn rejects_changed_media_collection() {
        let initial = plan(&[("media-1", "01.jpg")]);
        let changed = plan(&[("media-2", "02.jpg")]);
        let mut coordinator = RefreshCoordinator::new(
            move |_: &MediaTransferPlan| Ok::<_, &str>(changed.clone()),
            |_: &MediaTransferPlan| Err(expired()),
        );
        let failure = coordinator.execute(&initial).expect_err("changed result");
        assert_eq!(failure.message, EXTRACTION_RESULT_CHANGED);
    }

    #[test]
    fn does_not_refresh_non_expired_failure() {
        let initial = plan(&[("media-1", "01.jpg")]);
        let mut extract_calls = 0;
        let mut coordinator = RefreshCoordinator::new(
            |_: &MediaTransferPlan| {
                extract_calls += 1;
                Ok::<_, &str>(initial.clone())
            },
            |_: &MediaTransferPlan| {
                Err(TransferFailure {
                    media_id: Some("media-1".into()),
                    code: TransferFailureCode::Failed,
                    message: "disk full".into(),
                    aria2_error_code: None,
                })
            },
        );
        let failure = coordinator.execute(&initial).expect_err("non-refreshable");
        assert_eq!(failure.message, "disk full");
        assert_eq!(extract_calls, 0);
    }
}
