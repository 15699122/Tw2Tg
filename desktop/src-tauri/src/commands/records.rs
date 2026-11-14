//! Record and configuration exchange commands: export, preview, apply and import.
//!
//! The record path revalidates the exchange document, probes local media evidence
//! and commits through Storage; the configuration path exchanges only non-secret
//! settings. Command names, argument shapes, serialized fields and error strings
//! are the registered IPC contract and must not change here.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use std::fs;
use tauri::State;
use xarchive_core::DownloadRecord;

#[cfg(test)]
use sha2::{Digest, Sha256};
use xarchive_storage::Database;
use xarchive_storage::record_evidence::RecordEvidenceVerdict;
use xarchive_storage::record_import::{AcceptedRecord, RecordImportOutcome};
use xarchive_workflow::maintenance::WorkspaceGuard;

use crate::RuntimeState;
use crate::executor::CancellationToken;

mod export;
mod import_document;
mod import_evidence;

#[cfg(test)]
use export::serialize_record_export;
use export::{serialize_batch_record_export, serialize_record_export_with_account};
use import_document::{RecordImportDocument, record_import_revision, record_import_root};
pub(crate) use import_document::{
    RecordImportOutcomeResponse, RecordImportPreviewEntry, RecordImportPreviewResponse,
};
use import_evidence::{record_import_media_probe, verify_accepted_media_evidence};

/// Redacted preview projection. Never carries candidate path values or secrets
/// to the frontend; only the derived revision, restart flag and preserved notes.
#[derive(Serialize)]
pub(crate) struct ConfigurationExchangePreviewResponse {
    pub revision: String,
    pub restart_required: bool,
    pub preserved: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ConfigurationExchangeApplyResponse {
    pub revision: String,
    pub restart_required: bool,
    pub preserved: Vec<String>,
}

impl From<RecordImportOutcome> for RecordImportOutcomeResponse {
    fn from(outcome: RecordImportOutcome) -> Self {
        match outcome {
            RecordImportOutcome::Committed {
                accepted,
                identical,
            } => RecordImportOutcomeResponse {
                status: "Committed".to_owned(),
                accepted,
                identical,
                identities: Vec::new(),
                failures: Vec::new(),
            },
            RecordImportOutcome::Conflicts { identities } => RecordImportOutcomeResponse {
                status: "Conflicts".to_owned(),
                accepted: 0,
                identical: 0,
                identities,
                failures: Vec::new(),
            },
            RecordImportOutcome::RolledBack { failures } => RecordImportOutcomeResponse {
                status: "RolledBack".to_owned(),
                accepted: 0,
                identical: 0,
                identities: Vec::new(),
                failures,
            },
        }
    }
}

struct RecordImportWait;
impl Drop for RecordImportWait {
    fn drop(&mut self) {
        if let Ok(mut token) = RECORD_IMPORT_CANCELLATION.lock() {
            *token = None;
        }
    }
}

static RECORD_IMPORT_CANCELLATION: Mutex<Option<CancellationToken>> = Mutex::new(None);

fn serialize_persisted_record(
    database: &Database,
    tweet_id: &str,
    format: &str,
    kind: Option<&str>,
    archive_root: Option<&str>,
) -> Result<String, String> {
    let raw = database
        .merged_metadata_json(tweet_id)
        .map_err(|e| e.to_string())?
        .ok_or("job has no canonical archived metadata")?;
    let exchange_kind = match kind.unwrap_or("download") {
        "download" => xarchive_core::DOWNLOAD_RECORDS_KIND,
        "archive" => xarchive_core::ARCHIVE_RECORDS_KIND,
        _ => return Err("record kind must be download or archive".into()),
    };
    let handle = database
        .exchange_account_handle(exchange_kind, tweet_id)
        .map_err(|e| e.to_string())?;
    let download_json = serialize_record_export_with_account(&raw, "json", handle)?;
    let mut download_document = xarchive_core::DownloadRecordsDocument::from_json(&download_json)
        .map_err(|e| e.to_string())?;
    match kind.unwrap_or("download") {
        "download" => match format {
            "json" => download_document.to_json(),
            "yaml" => download_document.to_yaml(),
            _ => return Err("record format must be json or yaml".into()),
        }
        .map_err(|e| e.to_string()),
        "archive" => {
            let root = record_import_root(archive_root.unwrap_or(""))?;
            let facts = database
                .tweet_archive_facts(tweet_id)
                .map_err(|e| e.to_string())?
                .ok_or("tweet has no published archive facts")?;
            let directory =
                std::fs::canonicalize(&facts.archive_directory).map_err(|e| e.to_string())?;
            let relative = directory
                .strip_prefix(&root)
                .map_err(|_| "archive is outside export root")?;
            let final_path = if relative.as_os_str().is_empty() {
                ".".to_owned()
            } else {
                relative
                    .to_str()
                    .ok_or("archive path must be UTF-8")?
                    .to_owned()
            };
            let download = download_document.records.remove(0);
            let record = xarchive_core::ArchiveRecord {
                account_handle: download.account_handle,
                metadata: download.metadata,
                final_path,
            };
            if facts.media.len() != record.metadata.media.len()
                || record.metadata.media.iter().any(|media| {
                    !facts.media.iter().any(|fact| {
                        fact.media_index == media.index
                            && fact.media_id == media.media_id
                            && fact.media_type == media.media_type
                            && fact.mime_type == media.mime_type
                            && fact.relative_path == media.file
                            && fact.size_bytes == Some(media.size_bytes)
                            && fact.sha256.as_ref() == Some(&media.sha256)
                    })
                })
            {
                return Err("published media facts differ from canonical metadata".into());
            }
            if let RecordEvidenceVerdict::Preserve { reason } =
                xarchive_storage::record_evidence::preview_archive_record(
                    &record,
                    &root,
                    record_import_media_probe,
                )
            {
                return Err(reason);
            }
            let document = xarchive_core::ArchiveRecordsDocument::new(vec![record]);
            match format {
                "json" => document.to_json(),
                "yaml" => document.to_yaml(),
                _ => return Err("record format must be json or yaml".into()),
            }
            .map_err(|e| e.to_string())
        }
        _ => Err("record kind must be download or archive".into()),
    }
}

#[tauri::command]
pub(crate) fn export_configuration_exchange(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<String, String> {
    let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
    crate::config::ConfigurationExchange::export(&state.config)
}

#[tauri::command]
pub(crate) fn preview_configuration_exchange(
    state: State<'_, Mutex<RuntimeState>>,
    document: String,
) -> Result<ConfigurationExchangePreviewResponse, String> {
    let exchange = crate::config::ConfigurationExchange::parse(&document)?;
    let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    let bytes = fs::read(&paths.config_file).ok();
    let preview = exchange.preview(&state.config, &paths.root, bytes.as_deref())?;
    Ok(ConfigurationExchangePreviewResponse {
        revision: preview.revision,
        restart_required: preview.restart_required,
        preserved: preview.preserved,
    })
}

#[tauri::command]
pub(crate) fn cancel_configuration_exchange() -> Result<(), String> {
    if let Some(token) = CONFIGURATION_EXCHANGE_CANCELLATION
        .lock()
        .map_err(|_| "configuration exchange cancellation lock poisoned")?
        .as_ref()
    {
        token.cancel();
    }
    Ok(())
}

/// Releases the in-flight marker on every exit path, including a panic.
struct ConfigurationExchangeWait;
impl Drop for ConfigurationExchangeWait {
    fn drop(&mut self) {
        if let Ok(mut token) = CONFIGURATION_EXCHANGE_CANCELLATION.lock() {
            *token = None;
        }
    }
}

static CONFIGURATION_EXCHANGE_CANCELLATION: Mutex<Option<CancellationToken>> = Mutex::new(None);

/// How long a configuration apply may wait for running sessions to drain.
#[cfg(target_os = "linux")]
const CONFIGURATION_EXCHANGE_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

/// Wait for exclusive workspace maintenance, classifying why it ended.
///
/// `WorkspaceGuard::drain` reports cancellation and genuine contention with the
/// same `LockError::Contended`, because the workflow layer must not know about
/// business cancellation. The mapping therefore happens here, where the token
/// and the deadline are both visible:
///
/// - a cancel observed by this wait is reported as `cancelled`, so the UI can
///   say "nothing was written" instead of showing a lock failure;
/// - a real I/O failure is never reported as cancellation;
/// - a genuine timeout is distinct from both.
///
/// The classification is a last observation, not a claim that no cancel can
/// arrive later. A cancel that lands after the drain succeeded is handled by the
/// commit-acceptance check inside `persist_preview`.
#[cfg(target_os = "linux")]
fn exchange_apply_wait(
    lock_dir: &std::path::Path,
    cancellation: &CancellationToken,
) -> Result<xarchive_workflow::maintenance::MaintenanceGuard, String> {
    let deadline = std::time::Instant::now() + CONFIGURATION_EXCHANGE_WAIT;
    match xarchive_workflow::maintenance::WorkspaceGuard::drain(lock_dir, || {
        cancellation.is_cancelled() || std::time::Instant::now() >= deadline
    }) {
        Ok(guard) => Ok(guard),
        Err(xarchive_workflow::coordination::LockError::Io) => {
            Err("workspace maintenance unavailable: cannot open the coordination lock".into())
        }
        Err(_) if cancellation.is_cancelled() => Err(
            "configuration exchange apply cancelled while waiting for workspace maintenance".into(),
        ),
        Err(_) if std::time::Instant::now() >= deadline => {
            Err("configuration exchange apply timed out waiting for workspace maintenance".into())
        }
        // `WorkspaceGuard::drain` only polls, so it cannot block on a holder
        // that never releases without either the deadline or the cancel firing.
        Err(error) => Err(format!("workspace maintenance unavailable: {error}")),
    }
}

/// Persist the preview under an already-held maintenance guard.
///
/// This is the write half of the apply transaction, separated from IPC plumbing
/// so the tests exercise the same code the command runs. The caller must hold the
/// `RuntimeState` lock, which is what makes the runtime swap and the durable
/// rename one acceptance point for this process.
#[cfg(target_os = "linux")]
fn exchange_apply_persist(
    exchange: &crate::config::ConfigurationExchange,
    expected_revision: &str,
    local: &mut crate::config::AppConfig,
    paths: &crate::portable::PortablePaths,
    maintenance: &xarchive_workflow::maintenance::MaintenanceGuard,
    cancellation: &CancellationToken,
) -> Result<crate::config::ConfigurationExchangePreview, String> {
    exchange.persist_preview(
        local,
        paths,
        expected_revision,
        maintenance,
        Some(cancellation),
    )
}

/// Apply a validated exchange under the workspace maintenance barrier.
///
/// Linux-only: the turnstile/active lock is what stops new production sessions
/// while existing ones drain, so config persistence cannot race a writer. The
/// wait is cancellable through `cancel_configuration_exchange`; a cancel observed
/// before the write's acceptance point leaves the durable and runtime
/// configuration untouched.
///
/// The boundary is **commit acceptance**, not the rename: `persist_preview`
/// re-reads the durable bytes, the expected revision and the cancellation token
/// immediately before it creates the temporary file. A cancel that lands after
/// that read is not rolled back — it reports the applied revision instead of a
/// cancelled result, which is the approved contract.
///
/// This is an `async` command on purpose. A synchronous `#[tauri::command]` is
/// executed inline on the IPC thread (`tauri` `Webview::on_message` ->
/// `run_invoke_handler` -> `body_blocking`), so a blocking drain wait would
/// occupy that thread and the cancel command could never run. The wait now
/// happens on a blocking worker instead.
#[tauri::command]
pub(crate) async fn apply_configuration_exchange(
    state: State<'_, Mutex<RuntimeState>>,
    document: String,
    expected_revision: String,
) -> Result<ConfigurationExchangeApplyResponse, String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (state, document, expected_revision);
        return Err("configuration exchange apply is not implemented on this platform".into());
    }
    #[cfg(target_os = "linux")]
    {
        let exchange = crate::config::ConfigurationExchange::parse(&document)?;
        let cancellation = CancellationToken::new();
        {
            let mut active = CONFIGURATION_EXCHANGE_CANCELLATION
                .lock()
                .map_err(|_| "configuration exchange cancellation lock poisoned")?;
            if active.is_some() {
                return Err("another configuration exchange apply is active".into());
            }
            *active = Some(cancellation.clone());
        }
        let _wait = ConfigurationExchangeWait;
        let (portable_root, database_path) = {
            let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
            (
                state.portable_root.clone(),
                state.executor.database_path().to_owned(),
            )
        };
        let paths = crate::portable::PortablePaths::from_root(portable_root);
        let lock_dir = database_path
            .parent()
            .ok_or("database path has no parent")?
            .join("locks");
        let wait_token = cancellation.clone();
        // Run the blocking drain on a worker so the IPC thread stays free to
        // deliver `cancel_configuration_exchange`.
        let maintenance = tauri::async_runtime::spawn_blocking(move || {
            exchange_apply_wait(&lock_dir, &wait_token)
        })
        .await
        .map_err(|error| format!("configuration exchange apply task failed: {error}"))??;
        let mut state = state.lock().map_err(|_| "runtime state lock poisoned")?;
        // Commit acceptance: the runtime swap and the durable rename share this
        // `RuntimeState` lock, and `persist_preview` re-reads durable state, the
        // expected revision and the cancellation token immediately before it
        // creates the temporary file. A cancel observed before that point writes
        // nothing; a cancel after it is never reported as cancelled.
        let preview = exchange_apply_persist(
            &exchange,
            &expected_revision,
            &mut state.config,
            &paths,
            &maintenance,
            &cancellation,
        )?;
        drop(maintenance);
        Ok(ConfigurationExchangeApplyResponse {
            revision: preview.revision,
            restart_required: preview.restart_required,
            preserved: preview.preserved,
        })
    }
}

/// Export persisted canonical metadata for a chosen job, never runtime config.
#[tauri::command]
pub(crate) fn export_record_exchange(
    state: State<'_, Mutex<RuntimeState>>,
    job_id: String,
    format: String,
    kind: Option<String>,
    archive_root: Option<String>,
    tweet_id: Option<String>,
    batch_id: Option<String>,
) -> Result<String, String> {
    let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
    let database = state.database.as_ref().ok_or("database unavailable")?;
    if !job_id.trim().is_empty() && tweet_id.as_ref().is_some_and(|id| !id.trim().is_empty()) {
        return Err("choose exactly one export scope: job, tweet or batch".into());
    }
    if let Some(batch_id) = batch_id.filter(|id| !id.trim().is_empty()) {
        if !job_id.trim().is_empty() || tweet_id.as_ref().is_some_and(|id| !id.trim().is_empty()) {
            return Err("choose exactly one export scope: job, tweet or batch".into());
        }
        return serialize_batch_record_export(
            database,
            &batch_id,
            &format,
            kind.as_deref(),
            archive_root.as_deref(),
        );
    }
    let tweet_id = match tweet_id.filter(|id| !id.trim().is_empty()) {
        Some(id) => id,
        None => {
            database
                .job_summary(&job_id)
                .map_err(|e| e.to_string())?
                .ok_or("job not found")?
                .tweet_id
        }
    };
    serialize_persisted_record(
        database,
        &tweet_id,
        &format,
        kind.as_deref(),
        archive_root.as_deref(),
    )
}

/// Pure F3 preview: re-checks every accepted record's media evidence against
/// the archive root without writing any durable state.
#[tauri::command]
pub(crate) fn preview_record_import(
    state: State<'_, Mutex<RuntimeState>>,
    records_json: String,
    archive_root: String,
) -> Result<RecordImportPreviewResponse, String> {
    let document = RecordImportDocument::parse(&records_json)?;
    let root = record_import_root(&archive_root)?;
    let records = document.records();
    let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
    let database = state.database.as_ref().ok_or("database unavailable")?;
    let verdicts = document.evidence(&root);
    let entries = records.iter().map(DownloadRecord::identity).zip(verdicts);
    Ok(RecordImportPreviewResponse {
        revision: record_import_revision(&records_json, &root),
        entries: entries
            .into_iter()
            .enumerate()
            .map(|(index, (identity, verdict))| {
                let conflict = if matches!(verdict, RecordEvidenceVerdict::Accept) {
                    match &document {
                        RecordImportDocument::Download(document) => database
                            .download_record_conflict(
                                &document.records[index],
                                root.to_str().ok_or("root must be UTF-8")?,
                            ),
                        RecordImportDocument::Archive(document) => {
                            database.archive_record_conflict(&document.records[index], &root)
                        }
                    }
                    .map_err(|e| e.to_string())?
                } else {
                    None
                };
                let verdict = match conflict {
                    Some(reason) => RecordEvidenceVerdict::Preserve { reason },
                    None => verdict,
                };
                let (verdict_str, reason) = match verdict {
                    RecordEvidenceVerdict::Accept => ("accept".to_owned(), None),
                    RecordEvidenceVerdict::Preserve { reason } => {
                        ("preserve".to_owned(), Some(reason))
                    }
                };
                Ok(RecordImportPreviewEntry {
                    identity: format!("{}/{}", identity.account_handle, identity.tweet_id),
                    verdict: verdict_str,
                    reason,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

#[tauri::command]
pub(crate) fn cancel_record_import() -> Result<(), String> {
    if let Some(token) = RECORD_IMPORT_CANCELLATION
        .lock()
        .map_err(|_| "import cancellation lock poisoned")?
        .as_ref()
    {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn accept_record_import(
    state: State<'_, Mutex<RuntimeState>>,
    records_json: String,
    accepted_json: String,
    tweet_id: String,
    archive_directory: String,
    expected_revision: String,
) -> Result<RecordImportOutcomeResponse, String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (
            state,
            records_json,
            accepted_json,
            tweet_id,
            archive_directory,
            expected_revision,
        );
        return Err("record import accept is not implemented on this platform".into());
    }
    #[cfg(target_os = "linux")]
    {
        let cancellation = CancellationToken::new();
        {
            let mut active = RECORD_IMPORT_CANCELLATION
                .lock()
                .map_err(|_| "import cancellation lock poisoned")?;
            if active.is_some() {
                return Err("another record import is active".into());
            }
            *active = Some(cancellation.clone());
        }
        let _wait = RecordImportWait;
        let now = crate::clock::now_iso();
        // Kept for command compatibility; tweet identities come from each record.
        let _ = tweet_id;
        let document = RecordImportDocument::parse(&records_json)?;
        let records = document.records();
        let accepted: Vec<AcceptedRecord> = serde_json::from_str(&accepted_json)
            .map_err(|error| format!("invalid accepted: {error}"))?;

        let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
        let database = state.database.as_ref().ok_or("database unavailable")?;

        let lock_dir = state
            .executor
            .database_path()
            .parent()
            .ok_or("database path has no parent")?
            .join("locks");
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        let _maintenance = WorkspaceGuard::drain(&lock_dir, || {
            cancellation.is_cancelled() || std::time::Instant::now() >= deadline
        })
        .map_err(|error| format!("workspace maintenance unavailable: {error}"))?;

        let root = record_import_root(&archive_directory)?;
        if record_import_revision(&records_json, &root) != expected_revision {
            return Err("record import preview is stale; preview again".into());
        }
        let verdicts = document.evidence(&root);
        for item in &accepted {
            if let Some(RecordEvidenceVerdict::Preserve { reason }) = verdicts.get(item.index) {
                return Err(format!("accepted-set index {}: {reason}", item.index));
            }
        }
        let archive_directory = root.to_str().ok_or("archive root must be UTF-8")?;
        // Validate the entire selection independently of record kind. Archive
        // evidence was resolved using P1 above; do not probe it relative to root.
        if accepted.is_empty() {
            return Err("accepted selection must not be empty".into());
        }
        let mut seen = std::collections::HashSet::new();
        for item in &accepted {
            if item.index >= records.len() || !seen.insert(item.index) {
                return Err("accepted selection has duplicate or out-of-range indices".into());
            }
        }
        let verified_accepted = match &document {
            RecordImportDocument::Download(_) => {
                verify_accepted_media_evidence(&records, &accepted, archive_directory)
                    .map_err(|error| format!("media evidence verification failed: {error}"))?
            }
            RecordImportDocument::Archive(_) => accepted,
        };

        if cancellation.is_cancelled() {
            return Err("record import cancelled before commit".into());
        }

        let outcome = match &document {
            RecordImportDocument::Download(_) => database.import_accepted_records(
                &records,
                &verified_accepted,
                &now,
                archive_directory,
            ),
            RecordImportDocument::Archive(document) => database.import_accepted_archive_records(
                &document.records,
                &verified_accepted,
                &now,
                &root,
            ),
        }
        .map_err(|error| format!("import failed: {error}"))?;

        Ok(RecordImportOutcomeResponse::from(outcome))
    }
}

#[cfg(test)]
mod record_import_acceptance_tests {
    use super::*;

    #[test]
    fn cancellation_command_signals_waiter_and_drop_clears_registration() {
        let token = CancellationToken::new();
        *RECORD_IMPORT_CANCELLATION.lock().unwrap() = Some(token.clone());
        let registration = RecordImportWait;
        cancel_record_import().unwrap();
        assert!(token.is_cancelled());
        drop(registration);
        assert!(RECORD_IMPORT_CANCELLATION.lock().unwrap().is_none());
    }

    #[test]
    fn production_parser_accepts_both_formats_and_refuses_unversioned_input() {
        let document = xarchive_core::DownloadRecordsDocument::new(vec![]);
        for raw in [document.to_json().unwrap(), document.to_yaml().unwrap()] {
            assert!(
                RecordImportDocument::parse(&raw)
                    .unwrap()
                    .records()
                    .is_empty()
            );
        }
        let archive = xarchive_core::ArchiveRecordsDocument::new(vec![]);
        for raw in [archive.to_json().unwrap(), archive.to_yaml().unwrap()] {
            assert!(matches!(
                RecordImportDocument::parse(&raw),
                Ok(RecordImportDocument::Archive(_))
            ));
        }
        for raw in [
            "[]",
            r#"{"schema_version":99,"kind":"tw2tg.download-records","records":[]}"#,
            r#"{"schema_version":1,"kind":"tw2tg.download-records","records":[],"token":"secret"}"#,
        ] {
            assert!(RecordImportDocument::parse(raw).is_err());
        }
    }

    #[test]
    fn batch_exports_all_available_records_in_both_formats() {
        let database = Database::open_in_memory().unwrap();
        let settings = xarchive_storage::BatchOutputSettings::default();
        database
            .create_account_batch(xarchive_storage::CreateAccountBatch {
                id: "exchange-batch",
                username: "alice",
                profile_url: "https://x.test/alice",
                browser: None,
                profile: None,
                filters_json: "{}",
                output_settings: &settings,
                downloader_arguments_json: "{}",
            })
            .unwrap();
        let records: Vec<DownloadRecord> = ["951", "952"]
            .into_iter()
            .map(|id| {
                serde_json::from_value(serde_json::json!({
            "account_handle": "collector", "metadata": {
                "schema_version": 1, "tweet_id": id, "url": format!("https://x.test/i/status/{id}"),
                "tweet_type": "original", "author": {"username": "alice"},
                "text": "batch", "archived_at": "2026-10-18T00:00:00Z", "media": []
            }
        })).unwrap()
            })
            .collect();
        database
            .import_accepted_records(
                &records,
                &[AcceptedRecord { index: 0 }, AcceptedRecord { index: 1 }],
                "now",
                "/archive",
            )
            .unwrap();
        let candidates: Vec<_> = ["951", "952", "953"]
            .into_iter()
            .map(|id| xarchive_storage::NewBatchCandidate {
                tweet_id: id.into(),
                url: format!("https://x.test/i/status/{id}"),
                created_at: None,
                tweet_type: "original".into(),
                is_repost: false,
                has_media: false,
                media_count: 0,
                user_id: None,
                username: Some("alice".into()),
                state: "DONE".into(),
                skip_reason: None,
            })
            .collect();
        database
            .insert_batch_candidates("exchange-batch", &candidates)
            .unwrap();
        let json =
            serialize_batch_record_export(&database, "exchange-batch", "json", None, None).unwrap();
        let yaml =
            serialize_batch_record_export(&database, "exchange-batch", "yaml", None, None).unwrap();
        assert_eq!(
            xarchive_core::DownloadRecordsDocument::from_json(&json)
                .unwrap()
                .records,
            records
        );
        assert_eq!(
            xarchive_core::DownloadRecordsDocument::from_yaml(&yaml)
                .unwrap()
                .records,
            records
        );
        assert!(serialize_batch_record_export(&database, "missing", "json", None, None).is_err());
        assert!(
            serialize_batch_record_export(
                &database,
                "exchange-batch",
                "json",
                Some("invalid"),
                None
            )
            .is_err()
        );
        let root = std::env::temp_dir().join(format!("tw2tg-batch-export-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let archives: Vec<_> = records
            .iter()
            .map(|record| xarchive_core::ArchiveRecord {
                account_handle: record.account_handle.clone(),
                metadata: record.metadata.clone(),
                final_path: ".".into(),
            })
            .collect();
        database
            .import_accepted_archive_records(
                &archives,
                &[AcceptedRecord { index: 0 }, AcceptedRecord { index: 1 }],
                "now",
                &root,
            )
            .unwrap();
        let root_string = root.to_str().unwrap();
        for format in ["json", "yaml"] {
            let raw = serialize_batch_record_export(
                &database,
                "exchange-batch",
                format,
                Some("archive"),
                Some(root_string),
            )
            .unwrap();
            let parsed = if format == "json" {
                xarchive_core::ArchiveRecordsDocument::from_json(&raw)
            } else {
                xarchive_core::ArchiveRecordsDocument::from_yaml(&raw)
            }
            .unwrap();
            assert_eq!(parsed.records, archives);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_parser_uses_final_directory_and_rechecks_changed_media() {
        let root =
            std::env::temp_dir().join(format!("record-archive-command-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Tweets/901")).unwrap();
        std::fs::write(root.join("Tweets/901/a.bin"), b"original").unwrap();
        let record: xarchive_core::ArchiveRecord = serde_json::from_value(serde_json::json!({
            "account_handle": "collector", "final_path": "Tweets/901",
            "metadata": {
                "schema_version": 1, "tweet_id": "901", "url": "https://x.test/i/status/901",
                "tweet_type": "original", "author": {"user_id": "1", "username": "alice"},
                "text": "one", "archived_at": "2026-10-18T00:00:00Z",
                "media": [{"index": 1, "media_type": "photo", "file": "a.bin",
                    "size_bytes": 8, "sha256": format!("{:x}", Sha256::digest(b"original"))}]
            }
        }))
        .unwrap();
        let document = xarchive_core::ArchiveRecordsDocument::new(vec![record]);
        let database = Database::open_in_memory().unwrap();
        for raw in [document.to_json().unwrap(), document.to_yaml().unwrap()] {
            let parsed = RecordImportDocument::parse(&raw).unwrap();
            assert_eq!(parsed.evidence(&root), vec![RecordEvidenceVerdict::Accept]);
            let RecordImportDocument::Archive(parsed) = parsed else {
                panic!("kind lost")
            };
            database
                .import_accepted_archive_records(
                    &parsed.records,
                    &[AcceptedRecord { index: 0 }],
                    "now",
                    &root,
                )
                .unwrap();
        }
        assert_eq!(
            database
                .exchange_account_handle(xarchive_core::ARCHIVE_RECORDS_KIND, "901")
                .unwrap()
                .as_deref(),
            Some("collector")
        );
        std::fs::write(root.join("Tweets/901/a.bin"), b"modified").unwrap();
        assert!(matches!(
            RecordImportDocument::parse(&document.to_json().unwrap())
                .unwrap()
                .evidence(&root)
                .as_slice(),
            [RecordEvidenceVerdict::Preserve { .. }]
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn root_and_input_revision_changes_invalidate_preview() {
        let root = std::env::temp_dir().join(format!("record-root-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let canonical = record_import_root(root.to_str().unwrap()).unwrap();
        let old = record_import_revision("first", &canonical);
        assert_ne!(old, record_import_revision("second", &canonical));
        assert!(record_import_root("").is_err());
        let moved = root.with_extension("old");
        std::fs::rename(&root, &moved).unwrap();
        std::fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        assert_ne!(old, record_import_revision("first", &canonical));
        std::fs::remove_dir(root).unwrap();
        std::fs::remove_dir(moved).unwrap();
    }

    #[test]
    fn changed_media_rejects_the_whole_selection() {
        let root = std::env::temp_dir().join(format!("record-import-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("media.bin");
        std::fs::write(&path, b"original").unwrap();
        let record: DownloadRecord = serde_json::from_value(serde_json::json!({
            "account_handle": "alice",
            "metadata": {
                "schema_version": 1, "tweet_id": "101", "url": "https://x.test/i/status/101",
                "tweet_type": "original", "author": {"user_id": "1", "username": "alice"},
                "text": "one", "archived_at": "2026-10-18T00:00:00Z",
                "media": [{"index": 1, "media_type": "photo", "file": "media.bin",
                    "size_bytes": 8, "sha256": format!("{:x}", Sha256::digest(b"original"))}]
            }
        }))
        .unwrap();
        let records = vec![record.clone(), record];
        let raw = serde_json::to_string(&records[0].metadata).unwrap();
        let json = serialize_record_export(&raw, "json").unwrap();
        let yaml = serialize_record_export(&raw, "yaml").unwrap();
        assert_eq!(
            RecordImportDocument::parse(&json).unwrap().records(),
            RecordImportDocument::parse(&yaml).unwrap().records()
        );
        assert!(serialize_record_export(&raw, "zip").is_err());
        let mut private = records[0].metadata.clone();
        private.url = "https://user:password@example.test/?token=private".into();
        assert!(
            serialize_record_export(&serde_json::to_string(&private).unwrap(), "json").is_err()
        );
        private.url = records[0].metadata.url.clone();
        private.media[0].file = "/source/private/file.jpg".into();
        assert!(
            serialize_record_export(&serde_json::to_string(&private).unwrap(), "yaml").is_err()
        );
        let database = Database::open_in_memory().unwrap();
        let one = [AcceptedRecord { index: 0 }];
        let imported = RecordImportDocument::parse(&json).unwrap().records();
        database
            .import_accepted_records(&imported, &one, "now", root.to_str().unwrap())
            .unwrap();
        let imported_yaml = RecordImportDocument::parse(&yaml).unwrap().records();
        assert_eq!(
            database
                .import_accepted_records(&imported_yaml, &one, "later", root.to_str().unwrap())
                .unwrap(),
            RecordImportOutcome::Committed {
                accepted: 0,
                identical: 1
            }
        );
        let selected = [AcceptedRecord { index: 0 }, AcceptedRecord { index: 1 }];
        let root_str = root.to_str().unwrap();
        assert!(verify_accepted_media_evidence(&records, &selected, root_str).is_ok());
        assert!(
            verify_accepted_media_evidence(&records, &[selected[0], selected[0]], root_str)
                .unwrap_err()
                .contains("duplicate")
        );
        std::fs::write(&path, b"modified").unwrap();
        assert!(verify_accepted_media_evidence(&records, &selected, root_str).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(verify_accepted_media_evidence(&records, &selected, root_str).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/passwd", &path).unwrap();
            assert!(
                verify_accepted_media_evidence(&records, &selected, root_str)
                    .unwrap_err()
                    .contains("symlink")
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_selections_fail_before_import() {
        assert!(verify_accepted_media_evidence(&[], &[], "/archive").is_err());
        assert!(
            verify_accepted_media_evidence(&[], &[AcceptedRecord { index: 0 }], "/archive")
                .unwrap_err()
                .contains("out of range")
        );
    }

    #[test]
    fn outcomes_have_flat_serialized_shapes() {
        let cases = [
            RecordImportOutcome::Committed {
                accepted: 2,
                identical: 1,
            },
            RecordImportOutcome::Conflicts {
                identities: vec!["alice / 1".into()],
            },
            RecordImportOutcome::RolledBack {
                failures: vec![xarchive_storage::record_import::RecordImportCommitFailure {
                    index: 3,
                    reason: "failed".into(),
                }],
            },
        ];
        for (outcome, status) in cases
            .into_iter()
            .zip(["Committed", "Conflicts", "RolledBack"])
        {
            let value = serde_json::to_value(RecordImportOutcomeResponse::from(outcome)).unwrap();
            assert_eq!(value["status"], status);
            assert!(value["accepted"].is_number());
            assert!(value["identical"].is_number());
            assert!(value["identities"].is_array());
            assert!(value["failures"].is_array());
        }
    }
}
#[cfg(test)]
mod configuration_exchange_cancellation_tests {
    use super::*;

    /// The cancellation slot is process-global, so tests that install a token
    /// must not interleave. Without this ordering, one test's `Drop` clears
    /// another test's token and a passing run becomes a scheduling accident.
    static EXCHANGE_SLOT_TEST_ORDER: Mutex<()> = Mutex::new(());

    fn hold_slot(token: &CancellationToken) -> std::sync::MutexGuard<'static, ()> {
        let order = EXCHANGE_SLOT_TEST_ORDER
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut active = CONFIGURATION_EXCHANGE_CANCELLATION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert!(
            active.is_none(),
            "a previous test leaked the in-flight cancellation slot"
        );
        *active = Some(token.clone());
        drop(active);
        order
    }

    /// The apply wait must be interruptible by the registered cancel command.
    /// This drives the real static token that `apply_configuration_exchange`
    /// installs, so a regression that stops forwarding cancellation fails here.
    #[test]
    fn cancel_command_interrupts_an_in_flight_configuration_exchange() {
        let token = CancellationToken::new();
        let _order = hold_slot(&token);
        let wait = ConfigurationExchangeWait;
        assert!(!token.is_cancelled());
        cancel_configuration_exchange().expect("cancel command");
        assert!(
            token.is_cancelled(),
            "the cancel command must flip the in-flight token"
        );
        drop(wait);
        assert!(
            CONFIGURATION_EXCHANGE_CANCELLATION
                .lock()
                .unwrap()
                .is_none(),
            "dropping the wait marker must clear the in-flight slot"
        );
    }

    /// Cancellation must actually reach a blocked apply. `apply` is an `async`
    /// command whose blocking drain runs on a `spawn_blocking` worker, so a
    /// cancel delivered from another thread must be observed by that wait
    /// instead of being queued behind it. This test holds the workspace
    /// admission lock, starts a drain that can only finish by cancellation, and
    /// asserts the cancel path unblocks it.
    #[cfg(target_os = "linux")]
    #[test]
    fn cancel_command_reaches_a_blocked_maintenance_drain() {
        let token = CancellationToken::new();
        let _order = hold_slot(&token);
        let root = std::env::temp_dir().join(format!(
            "tw2tg-f2-drain-cancel-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        let locks = root.join("locks");
        std::fs::create_dir_all(&locks).unwrap();
        // An active reader keeps `drain` blocked, exactly like a running writer.
        let active_reader = WorkspaceGuard::try_enter(&locks).expect("active reader");
        let _wait = ConfigurationExchangeWait;

        let wait_token = token.clone();
        let lock_dir = locks.clone();
        let drain = std::thread::spawn(move || exchange_apply_wait(&lock_dir, &wait_token));
        // Give the drain time to actually block on the contended lock.
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(
            !drain.is_finished(),
            "the drain must be blocked while an active reader holds the workspace"
        );
        cancel_configuration_exchange().expect("cancel command");
        let error = drain
            .join()
            .expect("drain thread")
            .expect_err("a cancelled drain must not report maintenance ownership");
        // A cancelled wait must not be presented to the user as a lock failure.
        assert!(
            error.contains("cancelled"),
            "the cancelled wait must be classified as cancellation: {error}"
        );
        assert!(
            !error.contains("workspace maintenance unavailable"),
            "a cancel is not a maintenance failure: {error}"
        );

        drop(active_reader);
        // Admission must be open again: cancellation dropped every descriptor.
        WorkspaceGuard::try_enter(&locks).expect("admission reopened after cancel");
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Production apply flow, through the same two functions the command calls:
    /// wait for maintenance, then persist under the commit-acceptance check.
    /// The earlier stage had no test at this level — a token flip and a direct
    /// `persist_preview` call are not evidence that an apply refuses to write.
    #[cfg(target_os = "linux")]
    #[test]
    fn apply_flow_cancelled_after_the_wait_writes_nothing_and_still_applies_later() {
        let token = CancellationToken::new();
        let _order = hold_slot(&token);
        let _wait = ConfigurationExchangeWait;
        let root = std::env::temp_dir().join(format!(
            "tw2tg-f2-flow-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        let paths = crate::portable::PortablePaths::from_root(root.clone());
        std::fs::create_dir_all(&paths.config_dir).unwrap();
        let locks = paths.config_dir.join("locks");
        std::fs::create_dir_all(&locks).unwrap();
        let document = crate::config::ConfigurationExchange::parse(
            r#"{"schema_version":1,"kind":"tw2tg.configuration","settings":{"log_max_files":11}}"#,
        )
        .unwrap();
        let mut local = crate::config::AppConfig::default();

        // Uncontended wait succeeds; the cancel then arrives before the commit
        // acceptance point, which is precisely the window nothing tested.
        let maintenance = exchange_apply_wait(&locks, &token).expect("uncontended wait");
        cancel_configuration_exchange().expect("cancel command");
        let error = match exchange_apply_persist(
            &document,
            "missing",
            &mut local,
            &paths,
            &maintenance,
            &token,
        ) {
            Ok(_) => panic!("a cancel before the write must refuse it"),
            Err(error) => error,
        };
        assert!(
            error.contains("cancelled before writing"),
            "unexpected error: {error}"
        );
        assert!(
            !paths.config_file.exists(),
            "a cancelled apply must not create the durable configuration"
        );
        assert_eq!(
            local.logging.max_files,
            crate::config::AppConfig::default().logging.max_files,
            "a cancelled apply must not mutate the runtime configuration"
        );
        drop(maintenance);

        // The same document still applies once nothing is cancelled.
        let fresh = CancellationToken::new();
        {
            let mut active = CONFIGURATION_EXCHANGE_CANCELLATION.lock().unwrap();
            *active = Some(fresh.clone());
        }
        let maintenance = exchange_apply_wait(&locks, &fresh).expect("second wait");
        let preview = exchange_apply_persist(
            &document,
            "missing",
            &mut local,
            &paths,
            &maintenance,
            &fresh,
        )
        .expect("an uncancelled apply must succeed");
        drop(maintenance);
        assert!(!preview.revision.is_empty());
        assert_eq!(local.logging.max_files, 11);
        assert!(paths.config_file.exists());

        std::fs::remove_dir_all(root).unwrap();
    }
}
