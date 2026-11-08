//! Executor job commands: submit, query, cancel and shutdown.
//!
//! These adapters hand a Job to the real executor service and return only a
//! snapshot. They never own the long-running persistence/resource context; the
//! executor opens its own. Command names, argument shapes, serialized fields and
//! error strings are the registered IPC contract and must not change here.

use std::sync::Mutex;

use serde::Serialize;
use tauri::State;
use xarchive_storage::Database;

use crate::archive::upsert_browser_user;
use crate::commands::settings::{
    read_downloader_settings, read_output_settings, snapshot_downloader_settings,
};
use crate::executor::{ArchiveJobSubmissionAdapter, ExecutorError, JobSnapshot};
use crate::{ArchiveTweetRequest, RuntimeState};

#[derive(Debug, Serialize)]
pub struct ExecutorSubmitResponse {
    pub job_id: String,
    pub tweet_id: String,
    pub state: String,
    pub created: bool,
}

#[derive(Debug, Serialize)]
pub struct ExecutorJobResponse {
    pub job_id: String,
    pub tweet_id: String,
    pub state: String,
}

fn executor_error(error: ExecutorError) -> String {
    error.to_string()
}

fn job_response(snapshot: JobSnapshot) -> ExecutorJobResponse {
    ExecutorJobResponse {
        job_id: snapshot.job_id,
        tweet_id: snapshot.tweet_id,
        state: snapshot.state.as_str().to_owned(),
    }
}

/// Submit one archive Job and schedule it on the real executor worker.
///
/// RuntimeState is used only to obtain immutable paths and the executor
/// handle. The command returns the initial Job snapshot immediately; the
/// executor opens its own persistence/resource context for long-running
/// SQLite, FileStore, Sidecar and ArchiveService I/O.
#[tauri::command]
pub(crate) fn submit_executor_job(
    state: State<'_, Mutex<RuntimeState>>,
    request: ArchiveTweetRequest,
) -> Result<ExecutorSubmitResponse, String> {
    let (service, database_path, use_aria2, tool_compatibility) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        if state.download_setup_required {
            return Err("download directory setup is required before archiving".to_owned());
        }
        let use_aria2 = state.config.download.use_aria2;
        let tool_compatibility = if use_aria2 {
            crate::executor::ToolCompatibility {
                aria2: state
                    .executor
                    .aria2_program()
                    .map(|program| {
                        crate::production::resolved_tool_version(program, &["--version"])
                    })
                    .transpose()?,
                gallery_dl: None,
            }
        } else {
            crate::executor::ToolCompatibility {
                aria2: None,
                gallery_dl: state
                    .executor
                    .gallery_dl_program()
                    .map(|program| {
                        crate::production::resolved_tool_version(program, &["--version"])
                    })
                    .transpose()?,
            }
        };
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
            use_aria2,
            tool_compatibility,
        )
    };
    let timestamp = crate::runtime::timestamp_marker();
    let mut database = match Database::open(&database_path) {
        Ok(database) => database,
        Err(error) => return Err(error.to_string()),
    };
    if let Some(active) = database
        .active_archive_job_for_tweet(&request.tweet.tweet_id)
        .map_err(|error| error.to_string())?
    {
        return Ok(ExecutorSubmitResponse {
            job_id: active.job_id,
            tweet_id: request.tweet.tweet_id,
            state: active.state.as_str().to_owned(),
            created: false,
        });
    }
    let now = crate::runtime::timestamp_marker();
    let tweet_row_id = match database.insert_tweet(
        &request.tweet.tweet_id,
        &request.tweet.url,
        &request.tweet.tweet_type,
        request.tweet.text.as_deref().unwrap_or_default(),
        &now,
    ) {
        Ok(tweet_row_id) => tweet_row_id,
        Err(error) => return Err(error.to_string()),
    };
    if let Err(error) = upsert_browser_user(&mut database, tweet_row_id, &request.tweet, &now) {
        return Err(error.to_string());
    }
    let mut persistence = crate::executor::StorageJobPersistence::open(database_path.clone())?;
    let output_settings = read_output_settings(&database)?;
    let downloader_settings = read_downloader_settings(&database)?;
    let downloader_arguments =
        snapshot_downloader_settings(&downloader_settings, use_aria2, tool_compatibility)?;
    let metadata_output = crate::executor::MetadataOutputSettings::read(&database)?;
    let prepared = if let Some(settings) = metadata_output {
        ArchiveJobSubmissionAdapter.prepare_with_metadata_settings(
            &request,
            &timestamp,
            settings,
            downloader_arguments,
        )
    } else {
        ArchiveJobSubmissionAdapter.prepare_with_downloader_settings(
            &request,
            &timestamp,
            output_settings,
            downloader_arguments,
        )
    }
    .map_err(executor_error)?;
    let result = match service.submit_and_schedule_persisted(
        &mut persistence,
        prepared,
        database_path.clone(),
    ) {
        Ok(result) => result,
        Err(error) => return Err(executor_error(error)),
    };
    if !result.created {
        return Ok(ExecutorSubmitResponse {
            job_id: result.job.job_id,
            tweet_id: result.job.tweet_id,
            state: result.job.state.as_str().to_owned(),
            created: false,
        });
    }
    drop(database);
    Ok(ExecutorSubmitResponse {
        job_id: result.job.job_id,
        tweet_id: result.job.tweet_id,
        state: result.job.state.as_str().to_owned(),
        created: true,
    })
}

#[tauri::command]
pub(crate) fn query_executor_job(
    state: State<'_, Mutex<RuntimeState>>,
    job_id: String,
) -> Result<ExecutorJobResponse, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
        )
    };
    let persistence = crate::executor::StorageJobPersistence::open(database_path)?;
    service
        .query_persisted(&persistence, &job_id)
        .map(job_response)
        .map_err(executor_error)
}

#[tauri::command]
pub(crate) fn cancel_executor_job(
    state: State<'_, Mutex<RuntimeState>>,
    job_id: String,
) -> Result<ExecutorJobResponse, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
        )
    };
    let mut persistence = crate::executor::StorageJobPersistence::open(database_path)?;
    service
        .cancel_persisted(&mut persistence, &job_id)
        .map(job_response)
        .map_err(executor_error)
}

#[tauri::command]
pub(crate) fn shutdown_executor(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let (service, database_path) = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        (
            state.executor.service(),
            state.executor.database_path().to_owned(),
        )
    };
    let mut persistence = crate::executor::StorageJobPersistence::open(database_path)?;
    service
        .interrupt_persisted(&mut persistence)
        .map_err(executor_error)?;

    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    state
        .executor
        .shutdown_in_place()
        .map(|()| "stopped".to_owned())
        .map_err(executor_error)
}
