use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{DEFAULT_LOG_MAX_FILES, LogLevel};

pub struct LogFile {
    pub path: PathBuf,
    pub level: LogLevel,
}

impl LogFile {
    pub fn open(directory: &Path, level: LogLevel, max_files: usize) -> Result<Self, String> {
        if level == LogLevel::Silent {
            return Ok(Self {
                path: directory.to_path_buf(),
                level,
            });
        }
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or(0);
        let path = directory.join(format!("xarchive-{timestamp}.log"));
        fs::write(&path, format!("level={}\n", level.as_str()))
            .map_err(|error| error.to_string())?;
        rotate(directory, max_files)?;
        Ok(Self { path, level })
    }

    pub fn append(&self, level: LogLevel, message: &str) -> Result<(), String> {
        if self.level == LogLevel::Silent || !enabled(self.level, level) {
            return Ok(());
        }
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        writeln!(file, "{} {}", level.as_str(), message.replace('\n', " "))
            .map_err(|error| error.to_string())
    }
}

pub fn enabled(configured: LogLevel, message: LogLevel) -> bool {
    match configured {
        LogLevel::Silent => false,
        LogLevel::Error => message == LogLevel::Error,
        LogLevel::Warning => matches!(message, LogLevel::Error | LogLevel::Warning),
        LogLevel::Info => matches!(
            message,
            LogLevel::Error | LogLevel::Warning | LogLevel::Info
        ),
        LogLevel::Debug => true,
    }
}

pub fn rotate(directory: &Path, max_files: usize) -> Result<(), String> {
    let max_files = if max_files == 0 {
        DEFAULT_LOG_MAX_FILES
    } else {
        max_files
    };
    let mut files = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|v| v.to_str()) == Some("log")
                && path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| v.starts_with("xarchive-")))
            .then_some(path)
        })
        .collect::<Vec<_>>();
    files.sort();
    while files.len() > max_files {
        let path = files.remove(0);
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_log_level() {
        assert!(enabled(LogLevel::Debug, LogLevel::Error));
        assert!(enabled(LogLevel::Info, LogLevel::Info));
        assert!(!enabled(LogLevel::Info, LogLevel::Debug));
        assert!(!enabled(LogLevel::Silent, LogLevel::Error));
    }
}
