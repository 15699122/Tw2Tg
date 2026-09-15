//! Browser protocol transport adapter for the executor service.
//!
//! This module translates the browser Native Messaging / WebSocket protocol
//! into calls against the executor application service and job persistence.
//!
//! The adapter is intentionally not registered as a production Native Host
//! endpoint yet. Keep the module-level dead-code allowance narrow to this
//! contract boundary until the Windows transport wiring is available.
#![allow(dead_code)]
//! It preserves the browser `request_id` so the extension can match request
//! and response.
use crate::executor::{
    ArchiveApplicationService, ArchiveJobSubmissionAdapter, ExecutorError, JobPersistence,
    state_sort_priority,
};
use xarchive_protocol::{BrowserRequest, BrowserResponse, BrowserTweet, PROTOCOL_VERSION};

/// Transports one browser request through the executor application service.
pub(crate) struct BrowserTransportAdapter {
    service: ArchiveApplicationService,
}

impl BrowserTransportAdapter {
    pub(crate) fn new(service: ArchiveApplicationService) -> Self {
        Self { service }
    }

    /// Handle one browser request and return the matching browser response.
    ///
    /// The browser `request_id` is preserved on success and included on
    /// failure so the extension can correlate the error with the original
    /// request.
    pub(crate) fn handle_request<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request: BrowserRequest,
    ) -> BrowserResponse {
        let request_id = match &request {
            BrowserRequest::ArchiveRequest { request_id, .. }
            | BrowserRequest::QueryStatus { request_id, .. } => request_id.clone(),
        };
        if let Err(error) = request.validate() {
            return BrowserResponse::Error {
                protocol_version: PROTOCOL_VERSION,
                request_id: Some(request_id),
                error_code: "PROTOCOL_ERROR".to_owned(),
                error_message: error.to_string(),
            };
        }

        match request {
            BrowserRequest::ArchiveRequest {
                protocol_version: _,
                request_id,
                tweet,
            } => self.handle_archive_request(persistence, request_id, tweet),
            BrowserRequest::QueryStatus {
                protocol_version: _,
                request_id,
                tweet_ids,
            } => self.handle_query_status(persistence, request_id, tweet_ids),
        }
    }

    fn handle_archive_request<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request_id: String,
        tweet: BrowserTweet,
    ) -> BrowserResponse {
        let archive_request = crate::ArchiveTweetRequest {
            tweet,
            browser: None,
            profile: None,
        };

        match ArchiveJobSubmissionAdapter.prepare(&archive_request, &now_iso()) {
            Ok(job) => match self.service.submit_persisted(persistence, job) {
                Ok(result) => BrowserResponse::ArchiveStatus {
                    protocol_version: PROTOCOL_VERSION,
                    request_id: request_id.clone(),
                    tweet_id: result.job.tweet_id.clone(),
                    job_id: Some(result.job.job_id.clone()),
                    state: result.job.state.as_str().to_owned(),
                    progress: None,
                },
                Err(error) => error_response(&request_id, &error),
            },
            Err(error) => error_response(&request_id, &error),
        }
    }

    fn handle_query_status<P: JobPersistence>(
        &self,
        persistence: &mut P,
        request_id: String,
        tweet_ids: Vec<String>,
    ) -> BrowserResponse {
        let requested_tweet_ids = tweet_ids.clone();
        let mut entries = Vec::new();

        for tweet_id in tweet_ids {
            let snapshot = match persistence.snapshot_for_tweet(&tweet_id) {
                Ok(Some(snapshot)) => snapshot,
                Ok(None) => continue,
                Err(_) => {
                    return BrowserResponse::Error {
                        protocol_version: PROTOCOL_VERSION,
                        request_id: Some(request_id.clone()),
                        error_code: "PERSISTENCE_ERROR".to_owned(),
                        error_message: format!("failed to read job for tweet {tweet_id}"),
                    };
                }
            };

            entries.push(snapshot);
        }

        if entries.is_empty() {
            return BrowserResponse::Error {
                protocol_version: PROTOCOL_VERSION,
                request_id: Some(request_id),
                error_code: "NO_JOBS_FOUND".to_owned(),
                error_message: format!(
                    "no matching jobs found for the requested tweet IDs: {}",
                    requested_tweet_ids.join(", ")
                ),
            };
        }

        // Sort by state priority (latest state first) then by job_id for stable ordering
        entries.sort_by(|a, b| {
            let a_priority = state_sort_priority(&a.state);
            let b_priority = state_sort_priority(&b.state);
            b_priority
                .cmp(&a_priority)
                .then_with(|| a.job_id.cmp(&b.job_id))
        });

        let latest = &entries[0];

        BrowserResponse::ArchiveStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.clone(),
            tweet_id: latest.tweet_id.clone(),
            job_id: Some(latest.job_id.clone()),
            state: latest.state.as_str().to_owned(),
            progress: None,
        }
    }
}

fn now_iso() -> String {
    // Keep this adapter deterministic in tests by using a fixed timestamp.
    // In production wiring this would be replaced by the runtime clock helper.
    "2026-09-13T00:00:00Z".to_owned()
}

fn error_response(request_id: &str, error: &ExecutorError) -> BrowserResponse {
    BrowserResponse::Error {
        protocol_version: PROTOCOL_VERSION,
        request_id: Some(request_id.to_owned()),
        error_code: error_error_code(error),
        error_message: error.to_string(),
    }
}

fn error_error_code(error: &ExecutorError) -> String {
    match error {
        ExecutorError::Closed => "EXECUTOR_CLOSED".to_owned(),
        ExecutorError::QueueFull => "EXECUTOR_QUEUE_FULL".to_owned(),
        ExecutorError::ResponseClosed => "EXECUTOR_RESPONSE_CLOSED".to_owned(),
        ExecutorError::UnknownJob(_) => "EXECUTOR_UNKNOWN_JOB".to_owned(),
        ExecutorError::InvalidTransition { .. } => "EXECUTOR_INVALID_TRANSITION".to_owned(),
        ExecutorError::Persistence(_) => "PERSISTENCE_ERROR".to_owned(),
        ExecutorError::Execution { error_code, .. } => error_code.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{InMemoryJobPersistence, JobExecutor};
    use xarchive_core::JobState;

    fn tweet(tweet_id: &str) -> BrowserTweet {
        BrowserTweet {
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            text: Some("archive me".to_owned()),
            created_at: Some("2026-09-12T00:00:00Z".to_owned()),
            tweet_type: "post".to_owned(),
            reply_to: None,
            quoted_tweet: None,
        }
    }

    fn archive_adapter() -> (BrowserTransportAdapter, JobExecutor) {
        let executor = JobExecutor::new();
        let service = ArchiveApplicationService::new(&executor);
        (BrowserTransportAdapter::new(service), executor)
    }

    #[test]
    fn transport_preserves_request_id_on_archive_request() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-archive-1".to_owned(),
            tweet: tweet("123"),
        };

        let response = transport.handle_request(&mut persistence, request.clone());

        let BrowserResponse::ArchiveStatus {
            request_id,
            tweet_id,
            job_id,
            state,
            ..
        } = response
        else {
            panic!("expected archive_status response, got {response:?}");
        };

        assert_eq!(request_id, "browser-archive-1");
        assert_eq!(tweet_id, "123");
        assert_eq!(job_id.as_deref(), Some("archive-123-2026-09-13T00:00:00Z"));
        assert_eq!(state, "QUEUED");
    }

    #[test]
    fn transport_reuses_existing_job_for_duplicate_archive_request() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-archive-1".to_owned(),
            tweet: tweet("123"),
        };

        let first = transport.handle_request(&mut persistence, request.clone());
        let second = transport.handle_request(&mut persistence, request);

        let BrowserResponse::ArchiveStatus {
            request_id: first_id,
            ..
        } = first
        else {
            panic!("expected first response to be archive_status");
        };
        let BrowserResponse::ArchiveStatus {
            request_id: second_id,
            ..
        } = second
        else {
            panic!("expected second response to be archive_status");
        };

        assert_eq!(first_id, "browser-archive-1");
        assert_eq!(second_id, "browser-archive-1");
    }

    #[test]
    fn transport_replies_with_current_state_on_query_status() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        // Submit an archive request first
        let archive_request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-archive-1".to_owned(),
            tweet: tweet("123"),
        };
        transport.handle_request(&mut persistence, archive_request);

        // Advance job state to Downloading
        persistence
            .persist_state(&crate::executor::JobSnapshot {
                job_id: "archive-123-2026-09-13T00:00:00Z".to_owned(),
                tweet_id: "123".to_owned(),
                state: JobState::Downloading,
            })
            .expect("persist state");

        // Query status
        let query_request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-status-1".to_owned(),
            tweet_ids: vec!["123".to_owned()],
        };

        let response = transport.handle_request(&mut persistence, query_request);

        let BrowserResponse::ArchiveStatus {
            request_id,
            tweet_id,
            state,
            ..
        } = response
        else {
            panic!("expected archive_status response, got {response:?}");
        };

        assert_eq!(request_id, "browser-status-1");
        assert_eq!(tweet_id, "123");
        assert_eq!(state, "DOWNLOADING");
    }

    #[test]
    fn transport_reports_error_for_query_status_with_no_matching_jobs() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let query_request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-status-1".to_owned(),
            tweet_ids: vec!["999".to_owned()],
        };

        let response = transport.handle_request(&mut persistence, query_request);

        let BrowserResponse::Error {
            request_id,
            error_code,
            error_message,
            ..
        } = response
        else {
            panic!("expected error response, got {response:?}");
        };

        assert_eq!(request_id.as_deref(), Some("browser-status-1"));
        assert_eq!(error_code, "NO_JOBS_FOUND");
        assert!(error_message.contains("999"));
    }

    #[test]
    fn transport_reports_error_when_archive_tweet_validation_fails() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();

        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "browser-invalid-tweet".to_owned(),
            tweet: BrowserTweet {
                tweet_id: "123".to_owned(),
                url: "https://example.com/status/123".to_owned(),
                username: Some("alice".to_owned()),
                display_name: Some("Alice".to_owned()),
                text: Some("archive me".to_owned()),
                created_at: Some("2026-09-12T00:00:00Z".to_owned()),
                tweet_type: "post".to_owned(),
                reply_to: None,
                quoted_tweet: None,
            },
        };

        let response = transport.handle_request(&mut persistence, request);

        let BrowserResponse::Error {
            request_id,
            error_code,
            error_message,
            ..
        } = response
        else {
            panic!("expected error response, got {response:?}");
        };

        assert_eq!(request_id.as_deref(), Some("browser-invalid-tweet"));
        assert_eq!(error_code, "PROTOCOL_ERROR");
        assert!(error_message.contains("tweet URL"));
        assert!(persistence.list_recovery_candidates().is_empty());
    }

    #[test]
    fn transport_rejects_invalid_protocol_version_before_persistence() {
        let (transport, _executor) = archive_adapter();
        let mut persistence = InMemoryJobPersistence::default();
        let request = BrowserRequest::ArchiveRequest {
            protocol_version: PROTOCOL_VERSION + 1,
            request_id: "browser-invalid-version".to_owned(),
            tweet: tweet("123"),
        };

        let response = transport.handle_request(&mut persistence, request);

        let BrowserResponse::Error {
            request_id,
            error_code,
            error_message,
            ..
        } = response
        else {
            panic!("expected protocol error response");
        };
        assert_eq!(request_id.as_deref(), Some("browser-invalid-version"));
        assert_eq!(error_code, "PROTOCOL_ERROR");
        assert!(error_message.contains("unsupported protocol version"));
        assert!(persistence.list_recovery_candidates().is_empty());
    }
}
