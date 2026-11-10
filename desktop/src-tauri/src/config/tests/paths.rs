use super::*;

#[test]
fn resolves_database_and_download_paths() {
    let paths = PortablePaths::from_root("/tmp/xarchive");
    let config = AppConfig::default();
    assert_eq!(
        config.database_path(&paths),
        PathBuf::from("/tmp/xarchive/config/archive.sqlite3")
    );
    assert_eq!(
        config.download_path(&paths),
        PathBuf::from("/tmp/xarchive/download")
    );
    assert_eq!(
        config.logs_path(&paths),
        PathBuf::from("/tmp/xarchive/logs")
    );
}

#[test]
fn separates_worker_gallery_dl_and_extension_defaults() {
    let config = AppConfig::default();
    assert_ne!(config.sidecar.worker, config.sidecar.gallery_dl);
    assert!(config.sidecar.worker.ends_with("xarchive-downloader.exe"));
    assert!(config.sidecar.gallery_dl.ends_with("gallery-dl.exe"));
    assert_eq!(config.extension.directory, "./extension");
    assert_eq!(config.extension.source, "bundled");
}

#[test]
fn worker_default_path_matches_portable_layout() {
    // Locks the contract between PyInstaller one-dir output,
    // windows-worker-artifact.yml and build-portable-windows.mjs:
    // worker must live at sidecar/xarchive-downloader/xarchive-downloader.exe
    let config = AppConfig::default();
    assert_eq!(
        config.sidecar.worker,
        "./sidecar/xarchive-downloader/xarchive-downloader.exe"
    );
}
