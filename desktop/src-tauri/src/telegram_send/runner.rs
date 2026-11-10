//! The bounded claim loop that executes queued sends.
//!
//! Split out of `telegram_send.rs` without changing behaviour or the public
//! path: the parent re-exports these items, so `crate::telegram_send::<name>`
//! keeps resolving.
//!
//! Invariants that must not move:
//!
//! * one run claims at most `MAX_SENDS_PER_RUN` entries, so a large backlog
//!   cannot starve the application;
//! * every durable boundary re-reads the clock instead of reusing one reading;
//! * a claim is renewed before completion is polled, and a lost renewal aborts
//!   the attempt instead of letting it finish unowned;
//! * a stopped worker claims nothing and reads no secret;
//! * the credential generation is re-checked after the secret is fetched, so a
//!   rotation cannot send with a token that is no longer active.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use xarchive_storage::{Database, FileStore};

use xarchive_telegram::{
    BotToken, OutboxEntry, ReqwestTelegramTransport, RunAttemptError, SendFailure, SendPayload,
    TelegramOutboxStore, UploadStage,
};

use crate::clock;
use crate::config::TelegramConfig;

use super::delivery::{evict_rejected_files, execute_payload, store_confirmed_files};
use super::{reconcile_archived_send_intents, recover_expired_claims, resolve_archived_payload};

/// Bounded batch: one run must not starve the application when a large backlog
/// exists. The next scheduled run continues where this one stopped.
const MAX_SENDS_PER_RUN: usize = 10;

/// Claim lease: long enough for a slow upload, short enough that a crashed
/// worker's entry becomes recoverable without manual action.
const CLAIM_LEASE: Duration = Duration::from_secs(300);

/// What one queue run did, for logs and task events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendRunSummary {
    pub claimed: u32,
    pub sent: u32,
    /// A retry was scheduled (`RETRY_WAIT`).
    pub deferred: u32,
    /// The outcome is unknown and waits for review.
    pub unknown: u32,
    /// A permanent failure or one that needs a corrected plan.
    pub failed: u32,
}

/// Execute the due queue once.
///
/// Persisted version-1 snapshots are authoritative; media references are
/// resolved by `resolve`. Historical NULL snapshots are not reconstructed.
/// `None` means the archive no longer has the payload and is recorded as a
/// plan needing correction instead of being silently skipped. `on_progress` receives each entry's
/// upload stages. A run stops after [`MAX_SENDS_PER_RUN`] entries or when
/// nothing is due; `UNKNOWN` entries are never picked up automatically.
///
/// The parameters stay separate rather than bundled into a context struct: the
/// sender needs the store, the transport, the settings and the clock reading
/// independently, and a wrapper would only rename them.
#[allow(clippy::too_many_arguments)]
pub async fn run_due_sends<R, P>(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    now: &str,
    resolve: R,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    R: Fn(&OutboxEntry) -> Option<SendPayload>,
    P: FnMut(&str, UploadStage),
{
    run_due_sends_with_clock(
        database,
        transport,
        token,
        config,
        bot_identity,
        || now.to_owned(),
        resolve,
        on_progress,
    )
    .await
}

/// Queue runner with a fresh time reading at every durable boundary.
#[allow(clippy::too_many_arguments)]
pub async fn run_due_sends_with_clock<R, P, C>(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    current_time: C,
    resolve: R,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    R: Fn(&OutboxEntry) -> Option<SendPayload>,
    P: FnMut(&str, UploadStage),
    C: Fn() -> String,
{
    run_due_sends_with_generation(
        database,
        transport,
        token,
        config,
        bot_identity,
        None,
        None,
        current_time,
        resolve,
        on_progress,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_due_sends_with_generation<R, P, C>(
    database: &Database,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    generation: Option<i64>,
    stop: Option<&crate::telegram_worker::WorkerStopSignal>,
    current_time: C,
    resolve: R,
    mut on_progress: P,
) -> Result<SendRunSummary, String>
where
    R: Fn(&OutboxEntry) -> Option<SendPayload>,
    P: FnMut(&str, UploadStage),
    C: Fn() -> String,
{
    let mut summary = SendRunSummary::default();
    while (summary.claimed as usize) < MAX_SENDS_PER_RUN {
        if stop.is_some_and(|signal| signal.is_stopping()) {
            break;
        }
        let captured_time = current_time();
        let now = captured_time.as_str();
        let claim_token = claim_token(bot_identity, summary.claimed);
        let lease_until = clock::timestamp_after(now, CLAIM_LEASE);
        let claimed = match generation {
            Some(generation) => database.claim_due_outbox_for_generation(
                generation,
                bot_identity,
                &claim_token,
                now,
                &lease_until,
            ),
            None => database.claim_due_outbox(bot_identity, &claim_token, now, &lease_until),
        };
        let Some(entry) = claimed.map_err(|error| error.to_string())? else {
            break;
        };
        summary.claimed += 1;
        let key = entry.idempotency_key.clone();
        let attempt = entry.attempt_count.saturating_sub(1);
        let payload = if entry.payload_schema_version.is_some() {
            entry_payload(&entry).and_then(|payload| match payload {
                SendPayload::Media { .. } => resolve(&entry),
                payload => Some(payload),
            })
        } else {
            None
        };

        // The transport progress callback must be `Send + 'static`, so stages
        // are collected per entry and replayed to the caller afterwards.
        let stages: Arc<Mutex<Vec<UploadStage>>> = Arc::new(Mutex::new(Vec::new()));
        let collected = Arc::clone(&stages);
        let execution = execute_payload(
            database,
            transport,
            token,
            config,
            bot_identity,
            payload.clone(),
            collected,
        );
        let attempt_future = xarchive_telegram::run_claimed_attempt_with_clock(
            database,
            &claim_token,
            attempt,
            &current_time,
            |delay| clock::timestamp_after(&current_time(), delay),
            execution,
        );
        let guarded = with_claim_heartbeat(database, &claim_token, &current_time, attempt_future);
        tokio::pin!(guarded);
        let mut progress_tick = tokio::time::interval(Duration::from_millis(100));
        let outcome = loop {
            tokio::select! {
                result = &mut guarded => break result,
                _ = progress_tick.tick() => {
                    for stage in std::mem::take(&mut *stages.lock().expect("stages")) {
                        on_progress(&key, stage);
                    }
                }
            }
        };
        for stage in std::mem::take(&mut *stages.lock().expect("stages")) {
            on_progress(&key, stage);
        }

        match outcome {
            Ok(success) => {
                summary.sent += 1;
                // Confirmed results are the only thing that may populate the cache.
                if let Some(payload) = payload {
                    store_confirmed_files(
                        database,
                        bot_identity,
                        &payload,
                        &success,
                        &current_time(),
                    )?;
                }
            }
            Err(RunAttemptError::Failed { classification, .. }) => match classification {
                SendFailure::Unknown => summary.unknown += 1,
                SendFailure::Permanent => summary.failed += 1,
                SendFailure::MediaCorrection => {
                    summary.failed += 1;
                    // Only an explicit "identifier is dead" answer may evict the
                    // cache entry; every other failure keeps it.
                    if let Some(payload) = payload {
                        evict_rejected_files(database, bot_identity, &payload)?;
                    }
                }
                _ => summary.deferred += 1,
            },
            Err(RunAttemptError::StaleClaim) => {
                // Another worker owns the row; its outcome is authoritative.
                summary.deferred += 1;
            }
            Err(RunAttemptError::Store(error)) => return Err(error.to_string()),
        }
    }
    Ok(summary)
}

pub(in crate::telegram_send) async fn with_claim_heartbeat<F, C>(
    database: &Database,
    claim: &str,
    clock: &C,
    attempt: F,
) -> Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>
where
    F: std::future::Future<Output = Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>>,
    C: Fn() -> String,
{
    with_claim_heartbeat_interval(database, claim, clock, attempt, Duration::from_secs(60)).await
}

pub(in crate::telegram_send) async fn with_claim_heartbeat_interval<F, C>(
    database: &Database,
    claim: &str,
    clock: &C,
    attempt: F,
    interval: Duration,
) -> Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>
where
    F: std::future::Future<Output = Result<xarchive_telegram::SendAttemptSuccess, RunAttemptError>>,
    C: Fn() -> String,
{
    let mut attempt = std::pin::pin!(attempt);
    let mut heartbeat = Box::pin(tokio::time::sleep(interval));
    std::future::poll_fn(|context| {
        use std::future::Future;
        // Renew before polling completion when both become ready together.
        if heartbeat.as_mut().poll(context).is_ready() {
            let now = clock();
            let deadline = clock::timestamp_after(&now, CLAIM_LEASE);
            match database.renew_outbox_claim(claim, &deadline, &now) {
                Ok(true) => heartbeat
                    .as_mut()
                    .reset(tokio::time::Instant::now() + interval),
                Ok(false) => return std::task::Poll::Ready(Err(RunAttemptError::StaleClaim)),
                Err(error) => return std::task::Poll::Ready(Err(RunAttemptError::Store(error))),
            }
        }
        attempt.as_mut().poll(context)
    })
    .await
}

pub(in crate::telegram_send) fn entry_payload(entry: &OutboxEntry) -> Option<SendPayload> {
    if entry.payload_schema_version != Some(1) {
        return None;
    }
    serde_json::from_str(entry.payload_json.as_deref()?).ok()
}

/// Execute persisted snapshots using the archive store as the only media-path
/// resolver. The generic runner remains available for focused transport tests.
#[allow(clippy::too_many_arguments)]
pub async fn run_due_archived_sends<P>(
    database: &Database,
    files: &FileStore,
    transport: &ReqwestTelegramTransport,
    token: &BotToken,
    config: &TelegramConfig,
    bot_identity: &str,
    now: &str,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    P: FnMut(&str, UploadStage),
{
    run_due_sends(
        database,
        transport,
        token,
        config,
        bot_identity,
        now,
        |entry| resolve_archived_payload(database, entry, files),
        on_progress,
    )
    .await
}

/// Execute one bounded production batch using only the active credential's
/// secret reference. Disabled sending never reads secrets or claims work.
pub async fn run_active_archived_sends<P>(
    database: &Database,
    files: &FileStore,
    transport: &ReqwestTelegramTransport,
    secrets: &dyn xarchive_telegram::SecretStore,
    config: &TelegramConfig,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    P: FnMut(&str, UploadStage),
{
    run_active_archived_sends_cooperative(
        database,
        files,
        transport,
        secrets,
        config,
        None,
        on_progress,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_active_archived_sends_cooperative<P>(
    database: &Database,
    files: &FileStore,
    transport: &ReqwestTelegramTransport,
    secrets: &dyn xarchive_telegram::SecretStore,
    config: &TelegramConfig,
    stop: Option<&crate::telegram_worker::WorkerStopSignal>,
    on_progress: P,
) -> Result<SendRunSummary, String>
where
    P: FnMut(&str, UploadStage),
{
    if stop.is_some_and(|signal| signal.is_stopping()) {
        return Ok(SendRunSummary::default());
    }
    if !config.enabled || config.migration_pending {
        return Ok(SendRunSummary::default());
    }
    recover_expired_claims(database, &clock::now_iso())?;
    // Reconcile completed journals before claiming. A damaged journal remains
    // isolated; it cannot prevent unrelated archives from being sent.
    let _ = reconcile_archived_send_intents(database, &clock::now_iso())?;
    let (generation, identity, reference) = database
        .active_telegram_credential_generation()
        .map_err(|_| "active credential unavailable".to_owned())?
        .ok_or("active credential missing")?;
    let secret = secrets
        .get(&reference)
        .map_err(|_| "secret store unavailable".to_owned())?
        .ok_or("active secret missing")?;
    let token = BotToken::new(secret).map_err(|_| "active secret invalid".to_owned())?;
    if database
        .active_telegram_credential_generation()
        .map_err(|_| "active credential unavailable".to_owned())?
        != Some((generation, identity.clone(), reference))
    {
        return Err("credential generation changed during secret retrieval".into());
    }
    run_due_sends_with_generation(
        database,
        transport,
        &token,
        config,
        &identity,
        Some(generation),
        stop,
        clock::now_iso,
        |entry| resolve_archived_payload(database, entry, files),
        on_progress,
    )
    .await
}

/// A claim token unique per run, bot and index.
pub(in crate::telegram_send) fn claim_token(bot_identity: &str, index: u32) -> String {
    let prefix: String = bot_identity.chars().take(8).collect();
    format!(
        "{}-{prefix}-{index}-{}",
        clock::now_iso(),
        std::process::id()
    )
}
