//! Credential generation and rotation-resume policy.
//!
//! Split out of `telegram_send.rs` without changing any public path: the parent
//! module re-exports these items, so `crate::telegram_send::<name>` still
//! resolves for existing callers.
//!
//! Two invariants live here and must not move:
//!
//! * a rotation decision is captured atomically with the candidate snapshot, so
//!   a replay can never reauthorize rows that have since advanced or been
//!   cancelled;
//! * a failed activation deletes the newly written secret, leaving at most an
//!   inactive orphan and never sending authority.

use xarchive_storage::Database;
use xarchive_telegram::{BotToken, TelegramTransport};

/// Apply rotation policy to an exact queue snapshot. Confirmation mode only
/// returns candidates; callers must explicitly grant them later.
pub fn apply_credential_resume_policy(
    database: &Database,
    generation: i64,
    previous_bot_identity: Option<&str>,
    policy: crate::config::CredentialRotationResumePolicy,
    now: &str,
) -> Result<Vec<i64>, String> {
    let active = database
        .active_telegram_credential_generation()
        .map_err(|_| "credential state unavailable".to_owned())?
        .filter(|active| active.0 == generation)
        .ok_or("credential generation changed")?;
    if previous_bot_identity != Some(active.1.as_str()) {
        return Ok(Vec::new());
    }
    let (captured_policy, candidates) = match database
        .telegram_rotation_decision(generation)
        .map_err(|_| "rotation decision unavailable".to_owned())?
    {
        Some(decision) => decision,
        None => {
            let candidates = database
                .telegram_resume_candidates(generation)
                .map_err(|_| "resume candidates unavailable".to_owned())?;
            let captured_policy = match policy {
                crate::config::CredentialRotationResumePolicy::Automatic => "automatic",
                crate::config::CredentialRotationResumePolicy::Confirm => "confirm",
            };
            database
                .record_telegram_rotation_decision(generation, captured_policy, &candidates, now)
                .map_err(|_| "rotation decision persistence failed".to_owned())?;
            (captured_policy.to_owned(), candidates)
        }
    };
    // Automatic grants are committed atomically with the captured decision.
    // Replays never reauthorize rows that have since advanced or been cancelled.
    debug_assert!(matches!(captured_policy.as_str(), "automatic" | "confirm"));
    Ok(candidates)
}

/// Activate a verified candidate through an isolated SecretStore reference.
/// Failed activation never overwrites the active generation's stored token.
pub fn activate_verified_credential(
    database: &Database,
    store: &mut dyn xarchive_telegram::SecretStore,
    transport: &dyn TelegramTransport,
    candidate: &str,
    secret_reference: &str,
    expected_active: Option<i64>,
    now: &str,
) -> Result<i64, String> {
    activate_verified_credential_with_policy(
        database,
        store,
        transport,
        candidate,
        secret_reference,
        expected_active,
        crate::config::CredentialRotationResumePolicy::Automatic,
        now,
    )
}

/// Activate using the explicitly selected policy, captured in the same database
/// transaction as the generation change and eligible queue snapshot.
#[allow(clippy::too_many_arguments)]
pub fn activate_verified_credential_with_policy(
    database: &Database,
    store: &mut dyn xarchive_telegram::SecretStore,
    transport: &dyn TelegramTransport,
    candidate: &str,
    secret_reference: &str,
    expected_active: Option<i64>,
    policy: crate::config::CredentialRotationResumePolicy,
    now: &str,
) -> Result<i64, String> {
    let token =
        BotToken::new(candidate.trim()).map_err(|_| "invalid candidate token".to_owned())?;
    let identity = xarchive_telegram::verify_bot_identity(transport, &token)
        .map_err(|_| "candidate identity verification failed".to_owned())?;
    let generation = database
        .prepare_telegram_credential_generation(&identity, secret_reference, now)
        .map_err(|_| "credential candidate registration failed".to_owned())?;
    store
        .set(secret_reference, candidate.trim())
        .map_err(|_| "candidate secret storage failed".to_owned())?;
    if database
        .activate_telegram_credential_with_resume(
            generation,
            expected_active,
            match policy {
                crate::config::CredentialRotationResumePolicy::Automatic => "automatic",
                crate::config::CredentialRotationResumePolicy::Confirm => "confirm",
            },
            now,
        )
        .is_err()
    {
        // A failed delete leaves only an inactive orphan, never sending authority.
        let _ = store.delete(secret_reference);
        return Err("credential activation changed or failed".into());
    }
    Ok(generation)
}
