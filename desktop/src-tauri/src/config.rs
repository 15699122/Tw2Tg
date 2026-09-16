use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::portable::{PortablePaths, resolve_config_path};

pub const DEFAULT_LOG_MAX_FILES: usize = 5;
pub const MIN_LOG_MAX_FILES: usize = 1;
pub const MAX_LOG_MAX_FILES: usize = 100;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warning,
    #[default]
    Info,
    Debug,
    Silent,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Silent => "silent",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_logs_directory")]
    pub directory: String,
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_logs_directory() -> String {
    "./logs".to_owned()
}
fn default_max_files() -> usize {
    DEFAULT_LOG_MAX_FILES
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StorageConfig {
    #[serde(default = "default_database_path")]
    pub database: String,
    #[serde(default = "default_cache_path")]
    pub cache: String,
}

fn default_database_path() -> String {
    "./config/archive.sqlite3".to_owned()
}
fn default_cache_path() -> String {
    "./cache".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DownloadConfig {
    #[serde(default = "default_download_mode")]
    pub mode: String,
    #[serde(default = "default_download_path")]
    pub directory: String,
    #[serde(default)]
    pub initialized: bool,
}

fn default_download_mode() -> String {
    "portable".to_owned()
}
fn default_download_path() -> String {
    "./download".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SidecarConfig {
    #[serde(default = "default_gallery_dl_path")]
    pub gallery_dl: String,
    #[serde(default = "default_aria2_path")]
    pub aria2: String,
}

fn default_gallery_dl_path() -> String {
    "./sidecar/gallery-dl/gallery-dl.exe".to_owned()
}
fn default_aria2_path() -> String {
    "./sidecar/aria2/aria2c.exe".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExtensionConfig {
    #[serde(default = "default_extension_path")]
    pub directory: String,
}

fn default_extension_path() -> String {
    "./extension".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub download: DownloadConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub sidecar: SidecarConfig,
    #[serde(default)]
    pub extension: ExtensionConfig,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database: default_database_path(),
            cache: default_cache_path(),
        }
    }
}
impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            mode: default_download_mode(),
            directory: default_download_path(),
            initialized: false,
        }
    }
}
impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            directory: default_logs_directory(),
            level: default_log_level(),
            max_files: DEFAULT_LOG_MAX_FILES,
        }
    }
}

#[cfg(debug_assertions)]
fn default_log_level() -> LogLevel {
    LogLevel::Debug
}

#[cfg(not(debug_assertions))]
fn default_log_level() -> LogLevel {
    LogLevel::Info
}
impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            gallery_dl: default_gallery_dl_path(),
            aria2: default_aria2_path(),
        }
    }
}
impl Default for ExtensionConfig {
    fn default() -> Self {
        Self {
            directory: default_extension_path(),
        }
    }
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            storage: StorageConfig::default(),
            download: DownloadConfig::default(),
            logging: LoggingConfig::default(),
            sidecar: SidecarConfig::default(),
            extension: ExtensionConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn validate(&mut self) -> Result<(), String> {
        if !(MIN_LOG_MAX_FILES..=MAX_LOG_MAX_FILES).contains(&self.logging.max_files) {
            return Err(format!(
                "logging.max_files must be between {MIN_LOG_MAX_FILES} and {MAX_LOG_MAX_FILES}"
            ));
        }
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported config schema_version {}",
                self.schema_version
            ));
        }
        Ok(())
    }

    pub fn database_path(&self, paths: &PortablePaths) -> PathBuf {
        resolve_config_path(&paths.root, &self.storage.database)
    }
    pub fn cache_path(&self, paths: &PortablePaths) -> PathBuf {
        resolve_config_path(&paths.root, &self.storage.cache)
    }
    pub fn download_path(&self, paths: &PortablePaths) -> PathBuf {
        paths.resolve_download_directory(Some(&self.download.directory))
    }
    pub fn logs_path(&self, paths: &PortablePaths) -> PathBuf {
        resolve_config_path(&paths.root, &self.logging.directory)
    }

    pub fn load(paths: &PortablePaths) -> (Self, Option<String>) {
        let Ok(raw) = fs::read_to_string(&paths.config_file) else {
            return (Self::default(), None);
        };
        match serde_yaml::from_str::<Self>(&raw) {
            Ok(mut config) => match config.validate() {
                Ok(()) => (config, None),
                Err(error) => (Self::default(), Some(error)),
            },
            Err(error) => (
                Self::default(),
                Some(format!("invalid config.yaml: {error}")),
            ),
        }
    }

    pub fn save(&mut self, paths: &PortablePaths) -> Result<(), String> {
        self.validate()?;
        fs::create_dir_all(&paths.config_dir).map_err(|error| error.to_string())?;
        let content = serde_yaml::to_string(self).map_err(|error| error.to_string())?;
        let temporary = paths.config_file.with_extension("yaml.tmp");
        fs::write(&temporary, content).map_err(|error| error.to_string())?;
        fs::rename(&temporary, &paths.config_file).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable::PortablePaths;

    #[test]
    fn defaults_to_info_and_five_log_files() {
        let config = AppConfig::default();
        #[cfg(debug_assertions)]
        assert_eq!(config.logging.level, LogLevel::Debug);
        #[cfg(not(debug_assertions))]
        assert_eq!(config.logging.level, LogLevel::Info);
        assert_eq!(config.logging.max_files, 5);
    }

    #[test]
    fn validates_log_file_range() {
        let mut config = AppConfig::default();
        config.logging.max_files = 0;
        assert!(config.validate().is_err());
        config.logging.max_files = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn resolves_database_and_download_paths() {
        let paths = PortablePaths::from_root("/tmp/xarchive");
        let config = AppConfig::default();
        assert_eq!(
            config.database_path(&paths),
            PathBuf::from("/tmp/xarchive/./config/archive.sqlite3")
        );
        assert_eq!(
            config.download_path(&paths),
            PathBuf::from("/tmp/xarchive/./download")
        );
    }
}
