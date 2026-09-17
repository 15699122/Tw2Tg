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
        let timestamp = chrono_like_timestamp();
        writeln!(
            file,
            "{timestamp} {} app: {}",
            level.as_str(),
            message.replace('\n', " ")
        )
        .map_err(|error| error.to_string())
    }

    pub fn read_recent(directory: &Path, limit: usize) -> Result<Vec<String>, String> {
        let mut files = fs::read_dir(directory)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                (path.extension().and_then(|value| value.to_str()) == Some("log")
                    && path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.starts_with("xarchive-")))
                .then_some(path)
            })
            .collect::<Vec<_>>();
        files.sort();
        let path = files
            .pop()
            .ok_or_else(|| "no application log file exists".to_owned())?;
        let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
        let lines = content.lines().map(str::to_owned).collect::<Vec<_>>();
        let start = lines.len().saturating_sub(limit.max(1));
        Ok(lines[start..].to_vec())
    }
}

fn chrono_like_timestamp() -> String {
    // Keep the log format dependency-free. Milliseconds since epoch still sort
    // lexically and are converted to a readable time by the UI when possible.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
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

    #[test]
    fn reads_only_the_newest_bounded_log_lines() {
        let directory = std::env::temp_dir().join(format!(
            "xarchive-log-read-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        fs::create_dir_all(&directory).expect("log directory");
        fs::write(
            directory.join("xarchive-1.log"),
            "old\nkeep-1\nkeep-2\nkeep-3\n",
        )
        .expect("log file");
        assert_eq!(
            LogFile::read_recent(&directory, 2).expect("recent logs"),
            vec!["keep-2", "keep-3"]
        );
        let _ = fs::remove_dir_all(directory);
    }
}
