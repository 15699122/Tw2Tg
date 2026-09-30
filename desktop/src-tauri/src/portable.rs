use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortablePaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub database_file: PathBuf,
    pub cache_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub downloads_cache_dir: PathBuf,
    pub runtime_cache_dir: PathBuf,
    pub download_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub sidecar_dir: PathBuf,
    pub gallery_dl_dir: PathBuf,
    pub aria2_dir: PathBuf,
    pub extension_dir: PathBuf,
}

impl PortablePaths {
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let config_dir = root.join("config");
        let cache_dir = root.join("cache");
        let sidecar_dir = root.join("sidecar");
        Self {
            config_file: config_dir.join("config.yaml"),
            database_file: config_dir.join("archive.sqlite3"),
            staging_dir: cache_dir.join("staging"),
            downloads_cache_dir: cache_dir.join("downloads"),
            runtime_cache_dir: cache_dir.join("runtime"),
            gallery_dl_dir: sidecar_dir.join("gallery-dl"),
            aria2_dir: sidecar_dir.join("aria2"),
            config_dir,
            cache_dir,
            download_dir: root.join("download"),
            logs_dir: root.join("logs"),
            extension_dir: root.join("extension"),
            sidecar_dir,
            root,
        }
    }

    pub fn ensure_runtime_dirs(&self) -> std::io::Result<()> {
        for directory in [
            &self.config_dir,
            &self.cache_dir,
            &self.staging_dir,
            &self.downloads_cache_dir,
            &self.runtime_cache_dir,
            &self.logs_dir,
            &self.sidecar_dir,
        ] {
            fs::create_dir_all(directory)?;
        }
        Ok(())
    }

    pub fn resolve_download_directory(&self, configured: Option<&str>) -> PathBuf {
        configured
            .map(|value| resolve_config_path(&self.root, value))
            .unwrap_or_else(|| self.download_dir.clone())
    }
}

pub fn portable_root() -> PathBuf {
    if let Ok(root) = env::var("XARCHIVE_PORTABLE_ROOT")
        && !root.trim().is_empty()
    {
        return PathBuf::from(root);
    }
    if let Ok(exe) = env::current_exe()
        && let Some(parent) = exe.parent()
    {
        return parent.to_path_buf();
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn resolve_config_path(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        return normalize_path(&path);
    }
    normalize_path(&root.join(path))
}

/// Normalize a path without touching the filesystem.
///
/// `canonicalize` is intentionally not used here because portable paths may
/// not exist yet. This also keeps symlink/reparse-point handling inside the
/// storage safety boundary instead of resolving it during configuration load.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(value) => normalized.push(value),
        }
    }
    normalized
}

/// Name of the archive directory created inside the resolved system download
/// directory.
pub const ARCHIVE_DIRECTORY_NAME: &str = "XArchive";

/// Resolve the archive directory for the current user's system download
/// directory.
///
/// The platform query itself lives in [`crate::platform`], because Windows must
/// ask the Shell for the registered Downloads known folder instead of joining
/// `%USERPROFILE%` and `Downloads`. Keeping the join here makes the product
/// behavior (an `XArchive` subdirectory) testable without a platform query.
pub fn system_download_archive_directory(app: &tauri::AppHandle) -> Option<PathBuf> {
    system_download_directory(app).map(archive_directory_in)
}

/// Append the archive directory name to an already resolved download directory.
pub fn archive_directory_in(download_root: PathBuf) -> PathBuf {
    download_root.join(ARCHIVE_DIRECTORY_NAME)
}

/// Query the current user's system download directory.
fn system_download_directory(app: &tauri::AppHandle) -> Option<PathBuf> {
    crate::platform::system_download_directory(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_portable_layout_without_telegram_directory() {
        let paths = PortablePaths::from_root("/tmp/xarchive");
        assert_eq!(
            paths.config_file,
            PathBuf::from("/tmp/xarchive/config/config.yaml")
        );
        assert_eq!(
            paths.staging_dir,
            PathBuf::from("/tmp/xarchive/cache/staging")
        );
        assert_eq!(paths.logs_dir, PathBuf::from("/tmp/xarchive/logs"));
        assert!(!paths.root.join("telegram").exists());
    }

    #[test]
    fn resolves_relative_config_paths_against_portable_root() {
        let root = Path::new("/tmp/xarchive");
        assert_eq!(
            resolve_config_path(root, "./download"),
            PathBuf::from("/tmp/xarchive/download")
        );
        assert_eq!(
            resolve_config_path(root, "/data/download"),
            PathBuf::from("/data/download")
        );
    }

    #[test]
    fn normalizes_nested_relative_components_without_filesystem_access() {
        let root = Path::new("/tmp/xarchive");
        assert_eq!(
            resolve_config_path(root, "./cache/../logs"),
            PathBuf::from("/tmp/xarchive/logs")
        );
        assert_eq!(
            resolve_config_path(root, "./sidecar/gallery-dl/"),
            PathBuf::from("/tmp/xarchive/sidecar/gallery-dl")
        );
    }

    #[test]
    fn appends_archive_directory_to_a_resolved_download_directory() {
        // The download directory comes from the platform known-folder query, so
        // the archive location has to follow whatever location was resolved,
        // including a redirected volume or a non-ASCII path. Path::join is used
        // instead of a literal separator so the expectation stays valid on both
        // the Windows and the non-Windows separator.
        for download_root in ["/data/Downloads", r"D:\下载", r"\\NAS\User\Downloads"] {
            let resolved = PathBuf::from(download_root);
            assert_eq!(
                archive_directory_in(resolved.clone()),
                resolved.join(ARCHIVE_DIRECTORY_NAME)
            );
        }
        assert_eq!(ARCHIVE_DIRECTORY_NAME, "XArchive");
    }
}
