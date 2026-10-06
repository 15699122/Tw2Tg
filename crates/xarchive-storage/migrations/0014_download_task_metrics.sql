CREATE TABLE download_task_metrics (
    job_id TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
    backend TEXT CHECK (backend IN ('gallery_dl', 'aria2')),
    download_started_at TEXT,
    download_finished_at TEXT,
    task_finished_at TEXT,
    downloaded_bytes INTEGER CHECK (downloaded_bytes >= 0),
    download_duration_ms INTEGER CHECK (download_duration_ms >= 0),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    updated_at TEXT NOT NULL
);