use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_storage::{Database, FileStore};

use crate::executor::ExecutorRuntime;

pub(crate) const DEFAULT_ARCHIVE_ROOT: &str = "X-Archive";

pub struct RuntimeState {
    pub(crate) archive_root: PathBuf,
    pub(crate) database: Option<Database>,
    pub(crate) database_ready: bool,
    pub(crate) database_error: Option<String>,
    pub(crate) executor: ExecutorRuntime,
    pub(crate) sidecar: Option<SidecarSupervisor>,
    pub(crate) sidecar_error: Option<String>,
}

pub(crate) fn timestamp_marker() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

impl RuntimeState {
    pub(crate) fn initialize() -> Self {
        let archive_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(DEFAULT_ARCHIVE_ROOT);
        let database_path = archive_root.join("_database").join("archive.sqlite3");
        let database = (|| {
            FileStore::new(archive_root.clone()).ok()?;
            std::fs::create_dir_all(database_path.parent()?).ok()?;
            Database::open(&database_path).ok()
        })();
        let database_ready = database.is_some();
        let database_error = if database_ready {
            None
        } else {
            Some("failed to initialize archive database".to_owned())
        };

        Self {
            archive_root,
            database,
            database_ready,
            database_error,
            executor: ExecutorRuntime::new(database_path),
            sidecar: None,
            sidecar_error: None,
        }
    }
}

impl Drop for RuntimeState {
    fn drop(&mut self) {
        // Keep the executor worker lifetime bounded by the application
        // runtime. The synchronous archive fallback still owns its current
        // resources independently; this only shuts down the idle R1 worker
        // boundary during application teardown.
        let _ = self.executor.shutdown_in_place();
    }
}
