use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{DEFAULT_LOG_MAX_FILES, LogLevel};

/// Maximum size of the active log file before it is rotated on append.
pub const MAX_LOG_FILE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum size of a single log line; longer messages are truncated.
pub const MAX_LOG_LINE_BYTES: usize = 16 * 1024;
/// Maximum number of bytes read from the tail when showing recent logs.
const MAX_LOG_READ_BYTES: usize = 512 * 1024;

/// Collapse newlines so one message cannot inject extra log lines.
fn sanitize(message: &str) -> String {
    message.replace(['\n', '\r'], " ")
}

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

    /// Append one log line, refusing to grow the active file past the size cap.
    ///
    /// A single oversized line is truncated so logging never fails the caller
    /// and never turns a pathological message into unbounded disk usage.
    pub fn append(&self, level: LogLevel, message: &str) -> Result<(), String> {
        if self.level == LogLevel::Silent || !enabled(self.level, level) {
            return Ok(());
        }
        let sanitized = sanitize(message);
        self.append_bounded(level, &sanitized)
    }

    fn append_bounded(&self, level: LogLevel, message: &str) -> Result<(), String> {
        use std::io::Write;
        let mut line = format!(
            "{} {} app: {message}\n",
            chrono_like_timestamp(),
            level.as_str()
        );
        if line.len() > MAX_LOG_LINE_BYTES {
            line.truncate(MAX_LOG_LINE_BYTES);
            line.push('\n');
        }
        // Check the current size before writing so a long session rotates
        // instead of growing without bound.
        if let Ok(size) = fs::metadata(&self.path).map(|metadata| metadata.len())
            && size as usize + line.len() > MAX_LOG_FILE_BYTES
        {
            self.rotate_active_file()?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        file.write_all(line.as_bytes())
            .map_err(|error| error.to_string())
    }

    /// Rename the active log so the next append starts a fresh file.
    fn rotate_active_file(&self) -> Result<(), String> {
        let directory = self
            .path
            .parent()
            .ok_or_else(|| "log file has no parent directory".to_owned())?;
        let stem = self
            .path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("xarchive-archive")
            .to_owned();
        let rotated = directory.join(format!("{stem}-oversize.log"));
        fs::rename(&self.path, &rotated).map_err(|error| error.to_string())?;
        fs::write(&self.path, format!("level={}\n", self.level.as_str()))
            .map_err(|error| error.to_string())?;
        rotate(directory, DEFAULT_LOG_MAX_FILES)
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
        // Read only the tail of the newest log so a large session cannot make
        // the UI log view allocate the whole file.
        let bytes = read_tail(&path)?;
        let content = String::from_utf8_lossy(&bytes);
        let mut lines = content.lines().map(str::to_owned).collect::<Vec<_>>();
        // The tail may begin mid-line; drop that possible fragment.
        if lines.len() > 1 {
            lines.remove(0);
        }
        let start = lines.len().saturating_sub(limit.max(1));
        Ok(lines[start..].to_vec())
    }
}

/// Read at most the last `MAX_LOG_READ_BYTES` bytes of a file.
fn read_tail(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let size = file.metadata().map_err(|error| error.to_string())?.len();
    if size <= MAX_LOG_READ_BYTES as u64 {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        return Ok(bytes);
    }
    file.seek(SeekFrom::Start(size - MAX_LOG_READ_BYTES as u64))
        .map_err(|error| error.to_string())?;
    let mut bytes = vec![0_u8; MAX_LOG_READ_BYTES];
    file.read_exact(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
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

    #[test]
    fn truncates_a_single_oversized_log_line() {
        let directory = std::env::temp_dir().join(format!(
            "xarchive-log-line-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        fs::create_dir_all(&directory).expect("log directory");
        let log = LogFile::open(&directory, LogLevel::Debug, 5).expect("open log");

        let oversized = "x".repeat(MAX_LOG_LINE_BYTES * 2);
        log.append(LogLevel::Error, &oversized).expect("append");

        let content = fs::read_to_string(&log.path).expect("read log");
        assert!(
            content.len() <= MAX_LOG_LINE_BYTES + 64,
            "log line was not bounded: {} bytes",
            content.len()
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn collapses_newlines_so_one_message_cannot_add_log_lines() {
        let directory = std::env::temp_dir().join(format!(
            "xarchive-log-newline-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        fs::create_dir_all(&directory).expect("log directory");
        let log = LogFile::open(&directory, LogLevel::Debug, 5).expect("open log");

        log.append(LogLevel::Error, "first\nsecond\r\nthird")
            .expect("append");

        let content = fs::read_to_string(&log.path).expect("read log");
        let application_lines = content.lines().filter(|line| line.contains("app:")).count();
        assert_eq!(
            application_lines, 1,
            "one message must produce one log line: {content}"
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn read_recent_bounds_reading_of_a_large_log() {
        let directory = std::env::temp_dir().join(format!(
            "xarchive-log-tail-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        fs::create_dir_all(&directory).expect("log directory");
        let path = directory.join("xarchive-1.log");
        let mut content = "filler-line\n".repeat(MAX_LOG_READ_BYTES / 12 + 10);
        content.push_str("keep-1\nkeep-2\nkeep-3\n");
        fs::write(&path, content).expect("write log");

        assert_eq!(
            LogFile::read_recent(&directory, 2).expect("recent logs"),
            vec!["keep-2", "keep-3"]
        );
        let _ = fs::remove_dir_all(directory);
    }
}
