//! Bounded, coalescing wakeups for a dedicated Telegram sender thread.
//! The injected batch owns its database and platform credential dependencies.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) struct WorkerStopSignal(Arc<AtomicBool>);

impl WorkerStopSignal {
    pub(crate) fn is_stopping(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub(crate) struct TelegramWorker {
    wake: mpsc::SyncSender<()>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl TelegramWorker {
    pub(crate) fn is_running(&self) -> bool {
        !self.stopping.load(Ordering::Acquire)
            && self
                .thread
                .as_ref()
                .is_some_and(|thread| !thread.is_finished())
    }
    /// Build thread-local SQLite and transport dependencies before installing
    /// the worker. The platform supplies a real SecretStore; no fallback exists.
    #[allow(clippy::too_many_arguments)] // Explicit thread-owned dependencies and observation sink.
    pub(crate) fn start_sender<S, E>(
        interval: Duration,
        database_path: std::path::PathBuf,
        archive_root: std::path::PathBuf,
        config: crate::config::TelegramConfig,
        cloud_proxy: Option<String>,
        secrets: S,
        progress: Arc<
            std::sync::Mutex<std::collections::HashMap<String, xarchive_telegram::UploadStage>>,
        >,
        mut report: E,
    ) -> Result<Self, String>
    where
        S: xarchive_telegram::SecretStore + Send + 'static,
        E: FnMut(Result<crate::telegram_send::SendRunSummary, String>) + Send + 'static,
    {
        let endpoint = config.endpoint()?;
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let mut dependencies = None;
        let worker = Self::start_cooperative(interval, move |stop| {
            if dependencies.is_none() {
                let initialized = (|| {
                    let database = xarchive_storage::Database::open(&database_path)
                        .map_err(|_| "sender database unavailable".to_owned())?;
                    let files = xarchive_storage::FileStore::new(&archive_root)
                        .map_err(|_| "sender archive unavailable".to_owned())?;
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|_| "sender runtime unavailable".to_owned())?;
                    let transport = xarchive_telegram::ReqwestTelegramTransport::with_api_endpoint(
                        endpoint.clone(),
                        Duration::from_secs(config.connect_timeout_seconds.max(1)),
                        cloud_proxy.clone(),
                    )
                    .map_err(|_| "sender transport unavailable".to_owned())?;
                    Ok((database, files, runtime, transport))
                })();
                match initialized {
                    Ok(value) => {
                        dependencies = Some(value);
                        let _ = ready_tx.send(Ok(()));
                    }
                    Err(error) => {
                        let _ = ready_tx.try_send(Err(error));
                        return;
                    }
                }
            }
            let (database, files, runtime, transport) = dependencies.as_ref().unwrap();
            report(
                runtime.block_on(crate::telegram_send::run_active_archived_sends_cooperative(
                    database,
                    files,
                    transport,
                    &secrets,
                    &config,
                    Some(stop),
                    |key, stage| {
                        if let Ok(mut states) = progress.lock() {
                            states.insert(key.to_owned(), stage);
                        }
                    },
                )),
            );
        })
        .map_err(|_| "sender thread unavailable".to_owned())?;
        worker.wake();
        match ready_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(worker),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("sender initialization timed out".into()),
        }
    }

    pub(crate) fn start<F>(interval: Duration, mut batch: F) -> std::io::Result<Self>
    where
        F: FnMut() + Send + 'static,
    {
        Self::start_cooperative(interval, move |_| batch())
    }

    pub(crate) fn start_cooperative<F>(interval: Duration, mut batch: F) -> std::io::Result<Self>
    where
        F: FnMut(&WorkerStopSignal) + Send + 'static,
    {
        if interval.is_zero() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "worker interval must be positive",
            ));
        }
        let (wake, receiver) = mpsc::sync_channel(1);
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopping);
        let signal = WorkerStopSignal(Arc::clone(&stopping));
        let thread = thread::Builder::new()
            .name("telegram-sender".into())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    match receiver.recv_timeout(interval) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {
                            if !stop.load(Ordering::Acquire) {
                                batch(&signal);
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            wake,
            stopping,
            thread: Some(thread),
        })
    }

    pub(crate) fn wake(&self) {
        // Full means a wake is already pending; never block the GUI caller.
        let _ = self.wake.try_send(());
    }

    /// Stop scheduling new batches. An in-flight batch must implement its own
    /// bounded cancellation; dropping this handle never blocks on network I/O.
    pub(crate) fn stop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        self.wake();
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished)
            && let Some(thread) = self.thread.take()
        {
            let _ = thread.join();
        }
    }

    /// True means the thread terminated (including a panic), not that a remote
    /// request was cancelled or confirmed. False preserves the handle for retry.
    pub(crate) fn stop_and_wait(&mut self, timeout: Duration) -> bool {
        self.stop();
        let start = Instant::now();
        while self
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            if start.elapsed() >= timeout {
                return false;
            }
            thread::sleep(Duration::from_millis(1).min(timeout.saturating_sub(start.elapsed())));
        }
        self.stop();
        true
    }
}

impl Drop for TelegramWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sender_factory_runs_disabled_batch_without_credentials() {
        struct ForbiddenSecrets;
        impl xarchive_telegram::SecretStore for ForbiddenSecrets {
            fn get(&self, _: &str) -> Result<Option<String>, xarchive_telegram::SecretStoreError> {
                panic!("disabled sender must not access credentials");
            }
            fn set(&mut self, _: &str, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("read only");
            }
            fn delete(&mut self, _: &str) -> Result<(), xarchive_telegram::SecretStoreError> {
                panic!("read only");
            }
        }
        let root = std::env::temp_dir().join(format!(
            "tg-factory-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut worker = TelegramWorker::start_sender(
            Duration::from_secs(60),
            root.join("archive.sqlite3"),
            root.join("archive"),
            crate::config::TelegramConfig::default(),
            None,
            ForbiddenSecrets,
            Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            move |result| {
                tx.send(result.map(|summary| summary.claimed)).unwrap();
            },
        )
        .unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap().unwrap(), 0);
        assert!(worker.stop_and_wait(Duration::from_secs(2)));
        assert!(root.join("archive.sqlite3").is_file());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wake_runs_batch_and_stop_prevents_more_batches() {
        let (sent, received) = mpsc::channel();
        let mut worker = TelegramWorker::start(Duration::from_secs(60), move || {
            sent.send(()).unwrap();
        })
        .unwrap();
        worker.wake();
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.stop();
        worker.wake();
        assert!(received.recv_timeout(Duration::from_millis(30)).is_err());
    }

    #[test]
    fn cooperative_batch_observes_stop_and_joins() {
        let (started, received) = mpsc::channel();
        let mut worker = TelegramWorker::start_cooperative(Duration::from_secs(60), move |stop| {
            started.send(()).unwrap();
            while !stop.is_stopping() {
                thread::sleep(Duration::from_millis(1));
            }
        })
        .unwrap();
        worker.wake();
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(worker.stop_and_wait(Duration::from_secs(2)));
        assert!(worker.thread.is_none());
    }

    #[test]
    fn wait_timeout_keeps_handle_until_blocked_batch_exits() {
        let (started, received) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let mut worker = TelegramWorker::start(Duration::from_secs(60), move || {
            started.send(()).unwrap();
            blocked.recv().unwrap();
        })
        .unwrap();
        worker.wake();
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!worker.stop_and_wait(Duration::ZERO));
        assert!(worker.thread.is_some());
        release.send(()).unwrap();
        assert!(worker.stop_and_wait(Duration::from_secs(2)));
    }

    #[test]
    fn zero_interval_is_rejected() {
        assert!(TelegramWorker::start(Duration::ZERO, || {}).is_err());
    }
}
