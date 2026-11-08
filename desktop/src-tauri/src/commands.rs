use crate::RuntimeState;
use xarchive_storage::Database;

pub(crate) mod app;
pub(crate) mod archive;
pub(crate) mod batches;
pub(crate) mod executor;
pub(crate) mod extension;
pub(crate) mod jobs;
pub(crate) mod logs;
pub(crate) mod records;
pub(crate) mod settings;
pub(crate) mod sidecar;

pub(crate) fn open_batch_database(state: &RuntimeState) -> Result<Database, String> {
    Database::open(state.executor.database_path()).map_err(|error| error.to_string())
}

pub(crate) mod telegram;

fn bounded_diagnostic(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

#[cfg(test)]
mod frontend_diagnostic_tests {
    use super::{bounded_diagnostic, jobs::redact_job_errors};
    use xarchive_core::JobState;
    use xarchive_storage::JobSummary;
    #[test]
    fn bounds_frontend_diagnostic_by_characters() {
        assert_eq!(bounded_diagnostic("abcdef", 3), "abc");
        assert_eq!(bounded_diagnostic("白屏诊断", 2), "白屏");
    }

    #[test]
    fn redacts_job_failure_urls_before_frontend_projection() {
        let mut jobs = vec![JobSummary {
            job_id: "job-1".to_owned(),
            tweet_id: "1".to_owned(),
            tweet_type: "post".to_owned(),
            state: JobState::Failed,
            created_at: "now".to_owned(),
            updated_at: "now".to_owned(),
            last_error_code: Some("ARCHIVE_DOWNLOAD_FAILED".to_owned()),
            last_error_message: Some("http://u:p@cdn.example/file?token=secret&id=1".to_owned()),
        }];
        redact_job_errors(&mut jobs, &[]);
        assert_eq!(
            jobs[0].last_error_message.as_deref(),
            Some("http://[REDACTED]@cdn.example/file?token=[REDACTED]&id=1")
        );
    }
}

pub(crate) fn parse_sidecar_args(raw: &str) -> Result<Vec<String>, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw)
        .map_err(|error| format!("XARCHIVE_SIDECAR_ARGS must be a JSON string array: {error}"))
}

pub(crate) fn refresh_sidecar_state(state: &mut RuntimeState) {
    let exited = state
        .sidecar
        .as_mut()
        .and_then(|supervisor| supervisor.poll_exit().ok().flatten());
    if let Some(status) = exited {
        state.sidecar.take();
        state.sidecar_error = Some(format!("sidecar exited: {status}"));
    }
}
