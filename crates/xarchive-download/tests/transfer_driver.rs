//! Integration tests for the aria2-only transfer driver.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use xarchive_download::{
    AddUriRequest, Aria2TransferDriver, DownloadBackend, MediaTransferPlan, TransferControl,
    TransferDriver, TransferDriverConfig, TransferFailureCode, TransferFile, TransferId,
    TransferProgress, TransferState, TransferStatus,
};

#[derive(Default)]
struct FakeBackend {
    added: Mutex<Vec<AddUriRequest>>,
    removed: Mutex<Vec<String>>,
    next_gid: AtomicU32,
    statuses: Mutex<HashMap<String, VecDeque<TransferStatus>>>,
    fail_add_from: Option<u32>,
    /// State returned once a gid's scripted queue is exhausted.
    /// `Complete` by default so happy-path tests finish; set to `Active`
    /// for timeout tests where the transfer must never finish.
    default_state: Option<TransferState>,
}

impl FakeBackend {
    fn script(&self, gid: &str, statuses: Vec<TransferStatus>) {
        self.statuses
            .lock()
            .unwrap()
            .insert(gid.to_owned(), statuses.into());
    }

    fn plan() -> MediaTransferPlan {
        MediaTransferPlan {
            items: vec![
                xarchive_download::TransferPlanItem {
                    media_id: "media-1".into(),
                    url: "https://cdn.example/1.mp4".into(),
                    filename: "01.mp4".into(),
                    directory: "/tmp/staging".into(),
                    headers: vec!["Referer: https://x.com/".into()],
                    media_type: "video".into(),
                    mime_type: Some("video/mp4".into()),
                },
                xarchive_download::TransferPlanItem {
                    media_id: "media-2".into(),
                    url: "https://cdn.example/2.jpg".into(),
                    filename: "02.jpg".into(),
                    directory: "/tmp/staging".into(),
                    headers: vec![],
                    media_type: "photo".into(),
                    mime_type: Some("image/jpeg".into()),
                },
            ],
        }
    }

    fn config() -> TransferDriverConfig {
        TransferDriverConfig {
            poll_interval: Duration::ZERO,
            timeout: Duration::from_secs(5),
        }
    }

    fn status(gid: &str, state: TransferState, completed: u64) -> TransferStatus {
        TransferStatus {
            id: TransferId::new(gid).unwrap(),
            state,
            completed_bytes: completed,
            total_bytes: Some(1024),
            download_speed: 0,
            error_code: None,
            error_message: None,
            files: vec![TransferFile {
                path: format!("/tmp/staging/{gid}"),
                completed_bytes: completed,
                total_bytes: Some(1024),
            }],
        }
    }
}

impl DownloadBackend for FakeBackend {
    fn add_uri(
        &self,
        request: AddUriRequest,
    ) -> Result<TransferId, xarchive_download::DownloadError> {
        request.validate()?;
        let count = self.next_gid.fetch_add(1, Ordering::SeqCst) + 1;
        if self.fail_add_from.is_some_and(|from| count >= from) {
            return Err(xarchive_download::DownloadError::InvalidAddUriRequest);
        }
        self.added.lock().unwrap().push(request);
        let gid = format!("gid-{count:02}");
        self.statuses
            .lock()
            .unwrap()
            .entry(gid.clone())
            .or_default();
        Ok(TransferId::new(gid).unwrap())
    }

    fn status(&self, id: &TransferId) -> Result<TransferStatus, xarchive_download::DownloadError> {
        let mut map = self.statuses.lock().unwrap();
        let queue = map
            .get_mut(id.as_str())
            .ok_or(xarchive_download::DownloadError::InvalidTransferId)?;
        if let Some(status) = queue.pop_front() {
            return Ok(status);
        }
        let state = self.default_state.unwrap_or(TransferState::Complete);
        Ok(Self::status(id.as_str(), state, 1024))
    }

    fn pause(&self, _id: &TransferId) -> Result<TransferId, xarchive_download::DownloadError> {
        Err(xarchive_download::DownloadError::SupervisorNotRunning)
    }

    fn resume(&self, _id: &TransferId) -> Result<TransferId, xarchive_download::DownloadError> {
        Err(xarchive_download::DownloadError::SupervisorNotRunning)
    }

    fn cancel(&self, id: &TransferId) -> Result<TransferId, xarchive_download::DownloadError> {
        self.removed.lock().unwrap().push(id.as_str().to_owned());
        Ok(id.clone())
    }
}

#[test]
fn completes_plan_in_plan_order_with_progress() {
    let backend = FakeBackend::default();
    backend.script(
        "gid-01",
        vec![FakeBackend::status("gid-01", TransferState::Active, 512)],
    );

    let driver = Aria2TransferDriver::new(backend, FakeBackend::config());
    let mut progress = Vec::new();
    let outcomes = driver
        .execute(
            &FakeBackend::plan(),
            &TransferControl::new(),
            &mut |event| progress.push(event),
        )
        .expect("transfer");

    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0].media_id, "media-1");
    assert_eq!(outcomes[1].media_id, "media-2");
    assert!(outcomes.iter().all(|outcome| outcome.path.is_some()));
    assert!(
        progress
            .iter()
            .any(|event: &TransferProgress| event.completed_bytes == 512)
    );
}

#[test]
fn cancel_abandons_pending_and_cleans_partials() {
    let staging = std::env::temp_dir().join(format!("xarchive-u5-driver-{}", std::process::id()));
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join("01.mp4"), b"partial").unwrap();

    let backend = FakeBackend::default();
    backend.script(
        "gid-01",
        vec![TransferStatus {
            files: vec![TransferFile {
                path: staging.join("01.mp4").to_string_lossy().to_string(),
                completed_bytes: 7,
                total_bytes: Some(1024),
            }],
            ..FakeBackend::status("gid-01", TransferState::Active, 7)
        }],
    );

    let control = TransferControl::new();
    let cancel_control = control.clone();
    let driver = Aria2TransferDriver::new(backend, FakeBackend::config());
    let failure = driver
        .execute(&FakeBackend::plan(), &control, &mut |_| {
            cancel_control.cancel();
        })
        .expect_err("cancelled");

    assert_eq!(failure.code, TransferFailureCode::Cancelled);
    assert!(!staging.join("01.mp4").exists(), "partial removed");
    let _ = std::fs::remove_dir_all(&staging);
}

#[test]
fn shutdown_maps_to_interrupted() {
    let backend = FakeBackend::default();
    let control = TransferControl::new();
    control.request_shutdown();
    let driver = Aria2TransferDriver::new(backend, FakeBackend::config());
    let failure = driver
        .execute(&FakeBackend::plan(), &control, &mut |_| {})
        .expect_err("interrupted");
    assert_eq!(failure.code, TransferFailureCode::Interrupted);
}

#[test]
fn error_state_fails_with_media_id_and_abandons_rest() {
    let backend = FakeBackend::default();
    backend.script(
        "gid-01",
        vec![TransferStatus {
            error_code: Some("ERROR_403".into()),
            error_message: Some("forbidden".into()),
            ..FakeBackend::status("gid-01", TransferState::Error, 0)
        }],
    );
    let driver = Aria2TransferDriver::new(backend, FakeBackend::config());
    let failure = driver
        .execute(&FakeBackend::plan(), &TransferControl::new(), &mut |_| {})
        .expect_err("failed");
    assert_eq!(failure.code, TransferFailureCode::ExpiredUrl);
    assert_eq!(failure.media_id.as_deref(), Some("media-1"));
    assert!(failure.message.contains("forbidden"));
}

#[test]
fn add_uri_failure_abandons_submitted_transfers() {
    let backend = FakeBackend {
        fail_add_from: Some(2),
        ..FakeBackend::default()
    };
    let driver = Aria2TransferDriver::new(backend, FakeBackend::config());
    let failure = driver
        .execute(&FakeBackend::plan(), &TransferControl::new(), &mut |_| {})
        .expect_err("failed submission");
    assert_eq!(failure.media_id.as_deref(), Some("media-2"));
}

#[test]
fn progress_regression_fails_the_plan() {
    let backend = FakeBackend::default();
    backend.script(
        "gid-01",
        vec![
            FakeBackend::status("gid-01", TransferState::Active, 800),
            FakeBackend::status("gid-01", TransferState::Active, 100),
        ],
    );
    let driver = Aria2TransferDriver::new(backend, FakeBackend::config());
    let failure = driver
        .execute(&FakeBackend::plan(), &TransferControl::new(), &mut |_| {})
        .expect_err("regressed");
    assert_eq!(failure.code, TransferFailureCode::Failed);
    assert!(failure.message.contains("regressed"));
}

#[test]
fn timeout_maps_to_transfer_timeout() {
    let backend = FakeBackend {
        default_state: Some(TransferState::Active),
        ..FakeBackend::default()
    };
    backend.script(
        "gid-01",
        vec![FakeBackend::status("gid-01", TransferState::Active, 0)],
    );
    let config = TransferDriverConfig {
        poll_interval: Duration::from_millis(1),
        timeout: Duration::from_millis(20),
    };
    let driver = Aria2TransferDriver::new(backend, config);
    let failure = driver
        .execute(&FakeBackend::plan(), &TransferControl::new(), &mut |_| {})
        .expect_err("timeout");
    assert_eq!(failure.code, TransferFailureCode::Timeout);
}
