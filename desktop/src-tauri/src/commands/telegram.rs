//! Tauri commands for the Telegram delivery surface: task/job projections,
//! resume and review decisions, sender lifecycle, credential replacement and
//! endpoint migration.
//!
//! These are thin adapters over `RuntimeState` and the storage/telegram crates.
//! They own no delivery logic; projection wording, classification and transport
//! live in `xarchive-telegram`. Command names, argument shapes and error strings
//! are the registered IPC contract and must not change here.

use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;
use xarchive_storage::Database;

use crate::RuntimeState;

#[derive(Serialize)]
pub(crate) struct TelegramTaskProjection {
    id: i64,
    bot_identity: String,
    idempotency_key: String,
    message_link: Option<String>,
    reason: Option<String>,
    state: String,
    label: &'static str,
    progress: Option<String>,
}

fn telegram_progress_label(
    state: &RuntimeState,
    row: &xarchive_telegram::OutboxEntry,
) -> Option<String> {
    if row.state != xarchive_telegram::OutboxState::InFlight {
        return None;
    }
    state
        .telegram_progress
        .lock()
        .ok()?
        .get(&row.idempotency_key)
        .map(|stage| match stage {
            xarchive_telegram::UploadStage::Queued => "等待发送".into(),
            xarchive_telegram::UploadStage::CheckingFile => "核验文件".into(),
            xarchive_telegram::UploadStage::Uploading {
                sent_bytes,
                total_bytes,
            } => format!("请求体进度 {sent_bytes}/{total_bytes} 字节（非接收确认）"),
            xarchive_telegram::UploadStage::AwaitingResult => "等待 Bot API 结果".into(),
            xarchive_telegram::UploadStage::Confirmed => "Bot API 已确认（非客户端已读）".into(),
        })
}

#[tauri::command]
pub(crate) fn get_telegram_task_state(
    state: State<'_, Mutex<RuntimeState>>,
    bot_identity: String,
    tweet_row_id: i64,
) -> Result<Vec<TelegramTaskProjection>, String> {
    let state = state.lock().map_err(|_| "runtime unavailable")?;
    let database = state.database.as_ref().ok_or("database unavailable")?;
    database
        .list_outbox_for_tweet(&bot_identity, tweet_row_id)
        .map(|rows| {
            rows.into_iter()
                .map(|row| TelegramTaskProjection {
                    progress: telegram_progress_label(&state, &row),
                    id: row.id,
                    reason: row
                        .unknown_reason
                        .clone()
                        .or_else(|| row.last_error_code.map(|code| format!("API {code}"))),
                    bot_identity: row.bot_identity,
                    idempotency_key: row.idempotency_key,
                    message_link: row
                        .telegram_message_id
                        .as_deref()
                        .and_then(|id| xarchive_telegram::message_link(&row.chat_id, id)),
                    state: row.state.as_str().to_owned(),
                    label: xarchive_telegram::outbox_projection(row.state).label(),
                })
                .collect()
        })
        .map_err(|_| "telegram task state unavailable".into())
}

#[tauri::command]
pub(crate) fn get_telegram_job_state(
    state: State<'_, Mutex<RuntimeState>>,
    tweet_id: String,
) -> Result<Vec<TelegramTaskProjection>, String> {
    let state = state.lock().map_err(|_| "runtime unavailable")?;
    let database = state.database.as_ref().ok_or("database unavailable")?;
    let row_id = database
        .tweet_row_id(&tweet_id)
        .map_err(|_| "tweet unavailable")?;
    database
        .list_all_outbox_for_tweet(row_id)
        .map(|rows| {
            rows.into_iter()
                .map(|row| TelegramTaskProjection {
                    progress: telegram_progress_label(&state, &row),
                    id: row.id,
                    reason: row
                        .unknown_reason
                        .clone()
                        .or_else(|| row.last_error_code.map(|code| format!("API {code}"))),
                    bot_identity: row.bot_identity,
                    idempotency_key: row.idempotency_key,
                    message_link: row
                        .telegram_message_id
                        .as_deref()
                        .and_then(|id| xarchive_telegram::message_link(&row.chat_id, id)),
                    state: row.state.as_str().into(),
                    label: xarchive_telegram::outbox_projection(row.state).label(),
                })
                .collect()
        })
        .map_err(|_| "telegram state unavailable".into())
}

#[tauri::command]
pub(crate) fn confirm_telegram_resume(
    state: State<'_, Mutex<RuntimeState>>,
    generation: i64,
    outbox_ids: Vec<i64>,
) -> Result<(), String> {
    let state = state.lock().map_err(|_| "runtime unavailable")?;
    let database = state.database.as_ref().ok_or("database unavailable")?;
    let (_, candidates) = database
        .telegram_rotation_decision(generation)
        .map_err(|_| "rotation decision unavailable")?
        .ok_or("rotation decision missing")?;
    if outbox_ids.is_empty() || outbox_ids.iter().any(|id| !candidates.contains(id)) {
        return Err("resume request is outside captured rotation candidates".into());
    }
    database
        .authorize_telegram_resume(generation, &outbox_ids, &crate::clock::now_iso())
        .map_err(|_| "resume authorization rejected".to_owned())?;
    if let Some(worker) = state.telegram_worker.as_ref() {
        worker.wake();
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn review_telegram_unknown(
    state: State<'_, Mutex<RuntimeState>>,
    generation: i64,
    outbox_id: i64,
    confirmed: bool,
) -> Result<(), String> {
    let state = state.lock().map_err(|_| "runtime unavailable")?;
    state
        .database
        .as_ref()
        .ok_or("database unavailable")?
        .review_telegram_unknown_for_resend(
            generation,
            outbox_id,
            confirmed,
            &crate::clock::now_iso(),
        )
        .map_err(|_| "review target or credential changed; refresh before reviewing".to_owned())?;
    if let Some(worker) = &state.telegram_worker {
        worker.wake();
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn stop_telegram_sender(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "runtime unavailable")?;
    if state.stop_telegram_worker(Duration::from_secs(2)) {
        Ok(())
    } else {
        Err("telegram worker still stopping".into())
    }
}

#[tauri::command]
pub(crate) fn save_telegram_settings(
    state: State<'_, Mutex<RuntimeState>>,
    mut settings: crate::config::TelegramConfig,
) -> Result<i64, String> {
    let mut state = state.lock().map_err(|_| "runtime unavailable")?;
    if state.config.telegram.migration_pending {
        return Err("complete the explicit endpoint migration before saving settings".into());
    }
    settings.migration_pending = false;
    if settings.endpoint_mode != state.config.telegram.endpoint_mode
        && state.database.as_ref().is_some_and(|database| {
            database
                .active_telegram_credential_generation()
                .ok()
                .flatten()
                .is_some()
        })
    {
        return Err("active bot endpoint migration requires explicit server logout/close; delete local credential only after completing migration".into());
    }
    settings.validate()?;
    // Clients cannot attest capability or choose the next revision.
    settings.capability = None;
    settings.revision = state.config.telegram.revision;
    settings.record_settings_change();
    if !state.stop_telegram_worker(Duration::from_secs(2)) {
        return Err("telegram worker still stopping".into());
    }
    let paths = crate::portable::PortablePaths::from_root(&state.portable_root);
    let mut candidate = state.config.clone();
    candidate.telegram = settings;
    candidate.save(&paths)?;
    state.config = candidate;
    state.restart_telegram_sender()?;
    Ok(state.config.telegram.revision)
}

#[tauri::command]
pub(crate) async fn inspect_telegram_connection(
    state: State<'_, Mutex<RuntimeState>>,
    operation: String,
    confirmed: bool,
) -> Result<String, String> {
    let (store, reference, config, proxy) = {
        let state = state.lock().map_err(|_| "runtime unavailable")?;
        let (_, _, reference) = state
            .database
            .as_ref()
            .ok_or("database unavailable")?
            .active_telegram_credential_generation()
            .map_err(|_| "credential unavailable")?
            .ok_or("credential unavailable")?;
        (
            state
                .telegram_secrets
                .clone()
                .ok_or("platform credential provider unavailable")?,
            reference,
            state.config.telegram.clone(),
            state.telegram_proxy()?,
        )
    };
    let request = crate::telegram_control::diagnostic_request(&operation, confirmed, &config)?;
    tauri::async_runtime::spawn_blocking(move || {
        use xarchive_telegram::{SecretStore, TelegramTransport};
        let token = xarchive_telegram::BotToken::new(store.get(&reference).map_err(|_| "credential unavailable")?.ok_or("credential unavailable")?).map_err(|_| "credential invalid")?;
        let transport = xarchive_telegram::ReqwestTelegramTransport::with_api_endpoint(config.endpoint()?, Duration::from_secs(config.connect_timeout_seconds), proxy).map_err(|_| "transport unavailable")?;
        transport.send(&token, request).map_err(|_| "connection check failed; test message outcome may be unknown; do not automatically repeat")?;
        Ok("Bot API confirmed; client receipt not judged".into())
    }).await.map_err(|_| "connection task unavailable")?
}

#[tauri::command]
pub(crate) async fn migrate_telegram_endpoint(
    state: State<'_, Mutex<RuntimeState>>,
    mut settings: crate::config::TelegramConfig,
    confirmed: bool,
) -> Result<String, String> {
    if !confirmed {
        return Err("explicit migration confirmation required".into());
    }
    settings.validate()?;
    settings.endpoint()?;
    let (store, active, source, proxy, revision) = {
        let mut state = state.lock().map_err(|_| "runtime unavailable")?;
        if state.telegram_migration_busy {
            return Err("migration already running".into());
        }
        if settings.endpoint_mode == state.config.telegram.endpoint_mode
            && settings.api_base == state.config.telegram.api_base
        {
            return Err("select a different endpoint".into());
        }
        let active = state
            .database
            .as_ref()
            .ok_or("database unavailable")?
            .active_telegram_credential_generation()
            .map_err(|_| "credential unavailable")?
            .ok_or("credential unavailable")?;
        let store = state
            .telegram_secrets
            .clone()
            .ok_or("credential provider unavailable")?;
        let proxy = state.telegram_proxy()?;
        if !state.stop_telegram_worker(Duration::from_secs(2)) {
            return Err("sender still stopping".into());
        }
        let source = state.config.telegram.clone();
        let mut paused = state.config.clone();
        paused.telegram.migration_pending = true;
        paused.telegram.enabled = false;
        paused.telegram.record_settings_change();
        paused.save(&crate::portable::PortablePaths::from_root(
            &state.portable_root,
        ))?;
        state.config = paused;
        state.telegram_migration_busy = true;
        (store, active, source, proxy, state.config.telegram.revision)
    };
    let target = settings.clone();
    let expected = active.1.clone();
    let reference = active.2.clone();
    let migration_result = tauri::async_runtime::spawn_blocking(move || {
        use xarchive_telegram::SecretStore;
        let token = xarchive_telegram::BotToken::new(
            store
                .get(&reference)
                .map_err(|_| "credential unavailable")?
                .ok_or("credential unavailable")?,
        )
        .map_err(|_| "credential invalid")?;
        let transport = |config: &crate::config::TelegramConfig| {
            xarchive_telegram::ReqwestTelegramTransport::with_api_endpoint(
                config.endpoint()?,
                Duration::from_secs(config.connect_timeout_seconds),
                proxy.clone(),
            )
            .map_err(|_| "transport unavailable".to_owned())
        };
        crate::telegram_control::migrate(
            &transport(&source)?,
            &transport(&target)?,
            &token,
            source.endpoint_mode,
            &expected,
            true,
        )
    })
    .await
    .map_err(|_| "migration task unavailable; sender stays paused".to_owned());
    let mut state = state.lock().map_err(|_| "runtime unavailable")?;
    state.telegram_migration_busy = false;
    migration_result??;
    let current = state
        .database
        .as_ref()
        .ok_or("database unavailable")?
        .active_telegram_credential_generation()
        .map_err(|_| "credential unavailable")?;
    if current.as_ref() != Some(&active) || state.config.telegram.revision != revision {
        return Err("configuration or credential changed; sender stays paused".into());
    }
    settings.migration_pending = false;
    settings.enabled = false;
    settings.revision = revision;
    settings.record_settings_change();
    let mut candidate = state.config.clone();
    candidate.telegram = settings;
    candidate.save(&crate::portable::PortablePaths::from_root(
        &state.portable_root,
    ))?;
    state.config = candidate;
    Ok("Endpoint verified and saved; sending stays disabled until explicitly enabled".into())
}

#[derive(Serialize)]
pub(crate) struct TelegramRuntimeProjection {
    settings: crate::config::TelegramSettings,
    provider_available: bool,
    sender_running: bool,
    sender_error: Option<String>,
    generation: Option<i64>,
    bot_identity: Option<String>,
    resume_candidates: Vec<i64>,
}

#[tauri::command]
pub(crate) fn get_telegram_settings(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<TelegramRuntimeProjection, String> {
    use xarchive_telegram::SecretStore;
    let state = state.lock().map_err(|_| "runtime unavailable")?;
    let active = state
        .database
        .as_ref()
        .ok_or("database unavailable")?
        .active_telegram_credential_generation()
        .map_err(|_| "credential state unavailable")?;
    let present = match (&state.telegram_secrets, &active) {
        (Some(store), Some((_, _, reference))) => store
            .get(reference)
            .map_err(|_| "platform credential access failed")?
            .is_some(),
        _ => false,
    };
    let candidates = match active.as_ref() {
        Some((generation, _, _)) => state
            .database
            .as_ref()
            .unwrap()
            .telegram_rotation_decision(*generation)
            .map_err(|_| "rotation state unavailable")?
            .filter(|(policy, _)| policy == "confirm")
            .map(|(_, ids)| ids)
            .unwrap_or_default(),
        None => vec![],
    };
    Ok(TelegramRuntimeProjection {
        settings: state.config.telegram.settings(present),
        provider_available: state.telegram_secrets.is_some(),
        sender_running: state
            .telegram_worker
            .as_ref()
            .is_some_and(|worker| worker.is_running()),
        sender_error: state.telegram_sender_error.clone().or_else(|| {
            state
                .telegram_batch_error
                .lock()
                .ok()
                .and_then(|error| error.clone())
        }),
        generation: active.as_ref().map(|value| value.0),
        bot_identity: active.map(|value| value.1),
        resume_candidates: candidates,
    })
}

#[tauri::command]
pub(crate) fn start_telegram_sender(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    state
        .lock()
        .map_err(|_| "runtime unavailable")?
        .restart_telegram_sender()
}

#[tauri::command]
pub(crate) async fn replace_telegram_credential(
    state: State<'_, Mutex<RuntimeState>>,
    token: String,
) -> Result<i64, String> {
    let (mut store, database_path, config, proxy, expected) = {
        let state = state.lock().map_err(|_| "runtime unavailable")?;
        let store = state
            .telegram_secrets
            .clone()
            .ok_or("platform credential provider unavailable")?;
        let expected = state
            .database
            .as_ref()
            .ok_or("database unavailable")?
            .active_telegram_credential_generation()
            .map_err(|_| "credential state unavailable")?
            .map(|value| value.0);
        (
            store,
            state.executor.database_path().to_owned(),
            state.config.telegram.clone(),
            state.telegram_proxy()?,
            expected,
        )
    };
    let generation = tauri::async_runtime::spawn_blocking(move || {
        let database =
            Database::open(database_path).map_err(|_| "credential database unavailable")?;
        let transport = xarchive_telegram::ReqwestTelegramTransport::with_api_endpoint(
            config.endpoint()?,
            Duration::from_secs(config.connect_timeout_seconds.max(1)),
            proxy,
        )
        .map_err(|_| "credential transport unavailable")?;
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| "credential reference generation failed")?;
        let reference = format!("xarchive/telegram/{:032x}", u128::from_be_bytes(nonce));
        crate::telegram_send::activate_verified_credential_with_policy(
            &database,
            &mut store,
            &transport,
            &token,
            &reference,
            expected,
            config.credential_rotation_resume_policy,
            &crate::clock::now_iso(),
        )
    })
    .await
    .map_err(|_| "credential task unavailable")??;
    state
        .lock()
        .map_err(|_| "runtime unavailable")?
        .restart_telegram_sender()?;
    Ok(generation)
}

#[tauri::command]
pub(crate) fn delete_telegram_credential(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<(), String> {
    state
        .lock()
        .map_err(|_| "runtime unavailable")?
        .delete_telegram_secret()
}

#[tauri::command]
pub(crate) fn cancel_telegram_send(
    state: State<'_, Mutex<RuntimeState>>,
    bot_identity: String,
    idempotency_key: String,
) -> Result<bool, String> {
    use xarchive_telegram::TelegramOutboxStore;
    let state = state.lock().map_err(|_| "runtime unavailable")?;
    state
        .database
        .as_ref()
        .ok_or("database unavailable")?
        .cancel_outbox(&bot_identity, &idempotency_key, &crate::clock::now_iso())
        .map_err(|_| "telegram cancellation unavailable".into())
}
