//! Startup recovery reporting and journal replay under runner admission.
//! Original authorization, integrity verification and terminal ordering are preserved.

#[cfg(target_os = "linux")]
use super::{
    ArchiveApplicationService, ExecutorError, JobPersistence, LinuxExecutionContainment,
    ProductionCoordinator, SessionExecutionBoundary, SessionRunOutcome, StorageJobPersistence,
    containment_record_root, recover_writer_domain_set, recover_writer_domain_set_with_evidence,
    resolve_session_run,
};
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use xarchive_core::JobState;
#[cfg(target_os = "linux")]
use xarchive_storage::{ArchiveService, Database, FileStore};
#[cfg(target_os = "linux")]
use xarchive_workflow::{
    AttemptIdentity, ClaimOutcome, ExecutionReservation, RecoveryJournalIdentity, SessionError,
    authorize_prepared_recovery,
};

/// Internal startup-recovery result. Fatal enumeration/setup failures are kept
/// distinct from per-job refusals and contention; raw details are not logged.
///
/// `refusals` carries the per-job reason text for every outcome classified as
/// `Refused`. The public outcome enum deliberately stays reason-free, so the
/// diagnostic text cannot leak into an IPC contract, but a refused recovery is
/// no longer undiagnosable.
#[derive(Debug, Default)]
pub(crate) struct StartupRecoveryReport {
    pub(crate) outcomes: Vec<StartupRecoveryOutcome>,
    pub(crate) fatal_error: Option<String>,
    pub(crate) refusals: Vec<StartupRecoveryRefusal>,
}

/// Why one job's startup recovery was classified `Refused`.
///
/// This is internal diagnostics for a fail-closed decision, not an
/// authorization record: it never grants publication, never replaces an
/// attempt fence and never proves a writer was drained.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StartupRecoveryRefusal {
    pub(crate) job_id: String,
    pub(crate) reason: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StartupRecoveryOutcome {
    pub(crate) job_id: String,
    pub(crate) status: StartupRecoveryStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartupRecoveryStatus {
    Recovered,
    DeferredByContention,
    Refused,
}

impl StartupRecoveryReport {
    pub(super) fn failed(error: String) -> Self {
        Self {
            outcomes: Vec::new(),
            fatal_error: Some(error),
            refusals: Vec::new(),
        }
    }

    pub(crate) fn outcomes(&self) -> &[StartupRecoveryOutcome] {
        &self.outcomes
    }

    /// Per-job `Refused` reasons, in the same order as `outcomes`.
    pub(crate) fn refusals(&self) -> &[StartupRecoveryRefusal] {
        &self.refusals
    }
}

/// Replay the original journaled attempt under runner admission and live scopes.
/// `false` means contention: leave the durable job untouched for a later retry.
/// This never starts a writer or claims a successor attempt.
#[cfg(target_os = "linux")]
pub(super) fn recover_internal_v1_on_runner(
    service: &ArchiveApplicationService,
    database_path: PathBuf,
    archive_root: PathBuf,
    staging_root: PathBuf,
    job_id: String,
    tweet_id: String,
) -> Result<bool, ExecutorError> {
    if !service.executor.runner_try_admit() {
        return Ok(false);
    }
    let admission = service.executor.owned_runner_admission();
    let recovery_service = service.clone();
    service.executor.run_operation(admission, move || {
        let recover = || -> Result<bool, String> {
            let coordinator =
                ProductionCoordinator::open(&database_path).map_err(|e| e.to_string())?;
            let relative = PathBuf::from("Tweets").join(&tweet_id);
            let destination = archive_root.join(&relative);
            let reservation =
                match ExecutionReservation::acquire(&coordinator, &job_id, &destination) {
                    Ok(value) => value,
                    Err(SessionError::Contended) => return Ok(false),
                    Err(error) => return Err(error.to_string()),
                };
            let database = Database::open(&database_path).map_err(|e| e.to_string())?;
            let (phase, recorded_attempt, tweet_row_id, recorded_directory, recorded_state) =
                database
                    .internal_v1_recovery_identity(&job_id)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "InternalV1 recovery contract disappeared".to_owned())?;
            let identity = (
                phase.ok_or_else(|| {
                    "InternalV1 journal is missing for recovery candidate".to_owned()
                })?,
                recorded_attempt,
                tweet_row_id,
                recorded_directory,
            );
            let attempt = database
                .archive_attempt(&job_id)
                .map_err(|e| e.to_string())?;
            if identity.1 != attempt || identity.3 != relative.to_string_lossy() {
                return Err("InternalV1 journal attempt or destination mismatch".into());
            }
            let initial_state_ok = match identity.0 {
                xarchive_storage::ArchiveRecoveryPhase::Prepared => {
                    matches!(recorded_state.as_str(), "DOWNLOADING" | "DOWNLOADED")
                }
                xarchive_storage::ArchiveRecoveryPhase::Committed => {
                    matches!(recorded_state.as_str(), "DOWNLOADED" | "COMPLETE")
                }
            };
            if !initial_state_ok {
                return Err(
                    "startup recovery dispatch reached a non-published InternalV1 job".into(),
                );
            }
            let session = reservation
                .bind_existing_attempt_checked(
                    AttemptIdentity::new(&job_id, attempt).map_err(|e| e.to_string())?,
                    |bound| bound.attempt() == identity.1,
                )
                .map_err(|e| e.to_string())?;
            let record_root = containment_record_root(&staging_root).map_err(|e| e.to_string())?;
            let evidence = recover_writer_domain_set_with_evidence(
                &record_root,
                session.recovery_drain_binding(&staging_root),
            )
            .map_err(|e| e.to_string())?;
            let rechecked = Database::open(&database_path).map_err(|e| e.to_string())?;
            let (
                current_phase,
                current_attempt,
                current_tweet_row,
                current_directory,
                current_state,
            ) = rechecked
                .internal_v1_recovery_identity(&job_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    "InternalV1 recovery journal disappeared after writer drain".to_owned()
                })?;
            let current_identity = (
                current_phase.ok_or_else(|| {
                    "InternalV1 journal disappeared after writer drain".to_owned()
                })?,
                current_attempt,
                current_tweet_row,
                current_directory,
            );
            if current_identity != identity
                || rechecked
                    .archive_attempt(&job_id)
                    .map_err(|e| e.to_string())?
                    != attempt
                || rechecked
                    .job_state(&job_id)
                    .map_err(|e| e.to_string())?
                    .as_str()
                    != current_state
                || {
                    let drain_state_ok = match identity.0 {
                        xarchive_storage::ArchiveRecoveryPhase::Prepared => {
                            matches!(current_state.as_str(), "DOWNLOADING" | "DOWNLOADED")
                        }
                        xarchive_storage::ArchiveRecoveryPhase::Committed => {
                            matches!(current_state.as_str(), "DOWNLOADED" | "COMPLETE")
                        }
                    };
                    !drain_state_ok
                }
            {
                return Err("InternalV1 identity changed during writer drain".into());
            }
            // A terminal journal plus terminal state is a no-op, but only after
            // the committed recovery authorization and the Storage integrity
            // verification have run. Idempotence must never replace validation:
            // the decision to skip convergence is made inside the COMMITTED arm
            // below, after `authorize_committed_recovery` and
            // `verify_committed_terminal_internal_v1` have both succeeded.
            let mut archive = ArchiveService::new(
                Database::open(&database_path).map_err(|e| e.to_string())?,
                FileStore::with_staging_root(&archive_root, &staging_root)
                    .map_err(|e| e.to_string())?,
            );
            let final_path = match identity.0 {
                xarchive_storage::ArchiveRecoveryPhase::Prepared => {
                    let permit = authorize_prepared_recovery(
                        &session,
                        evidence,
                        RecoveryJournalIdentity {
                            attempt: session.attempt(),
                            destination: &archive_root.join(&relative),
                            journal_directory: &relative,
                            workspace: &staging_root,
                            phase: xarchive_workflow::RecoveryJournalPhase::Prepared,
                        },
                    )
                    .map_err(|e| format!("recovery authorization rejected: {e:?}"))?;
                    let (permit_attempt, _) = permit.replay_identity();
                    if permit_attempt != *session.attempt()
                        || permit.destination() != session.destination()
                    {
                        return Err("recovery permit identity changed".into());
                    }
                    let final_path = archive
                        .replay_prepared_internal_v1(
                            &job_id,
                            identity.2,
                            &relative,
                            &permit_attempt,
                            permit,
                        )
                        .map_err(|e| e.to_string())?;
                    archive
                        .converge_committed_internal_v1(
                            &job_id,
                            &permit_attempt,
                            &relative,
                            &StorageJobPersistence::now(),
                        )
                        .map_err(|e| e.to_string())?;
                    final_path
                }
                xarchive_storage::ArchiveRecoveryPhase::Committed => {
                    xarchive_workflow::authorize_committed_recovery(
                        &session,
                        evidence,
                        RecoveryJournalIdentity {
                            attempt: session.attempt(),
                            destination: &archive_root.join(&relative),
                            journal_directory: &relative,
                            workspace: &staging_root,
                            phase: xarchive_workflow::RecoveryJournalPhase::Committed,
                        },
                    )
                    .map_err(|e| format!("committed recovery authorization rejected: {e:?}"))?;
                    let final_path = archive
                        .verify_committed_terminal_internal_v1(
                            &job_id,
                            identity.2,
                            &relative,
                            session.attempt(),
                        )
                        .map_err(|e| e.to_string())?;
                    // Only now that the committed tree, its manifest and the
                    // durable identity have all been validated may an already
                    // COMPLETE attempt be treated as a no-op. A damaged or
                    // mismatched committed archive fails above instead of being
                    // reported as "already converged".
                    if current_state == "COMPLETE" {
                        return Ok(false);
                    }
                    archive
                        .converge_committed_internal_v1(
                            &job_id,
                            session.attempt(),
                            &relative,
                            &StorageJobPersistence::now(),
                        )
                        .map_err(|e| e.to_string())?;
                    final_path
                }
            };
            let _ = final_path;
            drop(session);
            let _ = recovery_service;
            Ok(true)
        };
        recover().map_err(ExecutorError::Persistence)
    })
}

#[cfg(target_os = "linux")]
pub(super) fn recover_v2_on_runner(
    service: &ArchiveApplicationService,
    database_path: PathBuf,
    archive_root: PathBuf,
    staging_root: PathBuf,
    containment: Option<LinuxExecutionContainment>,
    job_id: String,
    tweet_id: String,
) -> Result<bool, ExecutorError> {
    if !service.executor.runner_try_admit() {
        return Ok(false);
    }
    let admission = service.executor.owned_runner_admission();
    let recovery_service = service.clone();
    service.executor.run_operation(admission, move || {
        let recover = || -> Result<bool, String> {
            let coordinator =
                ProductionCoordinator::open(&database_path).map_err(|error| error.to_string())?;
            let relative = PathBuf::from("Tweets").join(&tweet_id);
            let destination = archive_root.join(&relative);
            let reservation =
                match ExecutionReservation::acquire(&coordinator, &job_id, &destination) {
                    Ok(reservation) => reservation,
                    Err(SessionError::Contended) => return Ok(false),
                    Err(error) => return Err(error.to_string()),
                };
            let database = Database::open(&database_path).map_err(|error| error.to_string())?;
            let journal = database
                .archive_recovery_manifest_v2(&job_id)
                .map_err(|error| error.to_string())?;
            let attempt = database
                .archive_attempt(&job_id)
                .map_err(|error| error.to_string())?;
            // Before PREPARED, drain the old attempt before admitting a
            // successor. Published states still require the durable journal.
            let state = database
                .job_state(&job_id)
                .map_err(|error| error.to_string())?;
            if journal.is_none()
                && matches!(
                    state,
                    JobState::Queued
                        | JobState::Validating
                        | JobState::MetadataReady
                        | JobState::Downloading
                        | JobState::Interrupted
                )
            {
                if containment.is_none() {
                    return Err("v2 recovery requires provisioned writer containment".into());
                }
                let record_root =
                    containment_record_root(&staging_root).map_err(|error| error.to_string())?;
                recover_writer_domain_set(&record_root, &staging_root, &job_id, u64::from(attempt))
                    .map_err(|error| error.to_string())?;
                drop(reservation);
                let persistence = StorageJobPersistence::open(&database_path)?;
                let mut boundary = SessionExecutionBoundary::new(
                    recovery_service.clone(),
                    persistence,
                    coordinator,
                    archive_root.clone(),
                );
                return boundary
                    .execute_session(&job_id)
                    .and_then(resolve_session_run)
                    .map_err(|error| error.to_string())
                    .and_then(|outcome| match outcome {
                        SessionRunOutcome::Finished(snapshot)
                            if snapshot.state == JobState::Complete =>
                        {
                            Ok(true)
                        }
                        SessionRunOutcome::Reschedulable { .. } => Ok(false),
                        SessionRunOutcome::Finished(snapshot) => Err(format!(
                            "startup execution did not complete; retained job is {:?}, inspect it",
                            snapshot.state
                        )),
                        // Defensive only: `resolve_session_run` above consumed
                        // the pending owner, so no live owner remains here.
                        SessionRunOutcome::StartupCleanupPending { job_id, .. } => {
                            Err(format!("unresolved startup cleanup decision for {job_id}").into())
                        }
                    });
            }
            let journal =
                journal.ok_or_else(|| "v2 archive recovery journal is missing".to_owned())?;
            if attempt != journal.attempt_count {
                return Err("v2 recovery attempt does not match the journal".into());
            }
            // Claim here binds the existing durable identity under the scopes;
            // it deliberately does not begin/increment an attempt or change state.
            let session = reservation
                .claim(|| match AttemptIdentity::new(&job_id, attempt) {
                    Ok(identity) => ClaimOutcome::Claimed(identity),
                    Err(_) => ClaimOutcome::Failed,
                })
                .map_err(|error| error.to_string())?;
            if containment.is_none() {
                return Err("v2 recovery requires provisioned writer containment".into());
            }
            let record_root =
                containment_record_root(&staging_root).map_err(|error| error.to_string())?;
            recover_writer_domain_set(&record_root, &staging_root, &job_id, u64::from(attempt))
                .map_err(|error| error.to_string())?;
            let tweet_row_id = database
                .tweet_row_id(&tweet_id)
                .map_err(|error| error.to_string())?;
            let files = FileStore::with_staging_root(&archive_root, &staging_root)
                .map_err(|error| error.to_string())?;
            let mut archive = xarchive_storage::ArchiveService::new(database, files);
            archive
                .recover_internal_rename_v2(
                    &job_id,
                    tweet_row_id,
                    &relative,
                    &StorageJobPersistence::now(),
                )
                .map_err(|error| error.to_string())?;
            // Storage owns journal/media finalization; Desktop owns the final
            // attempt-fenced Job/event transaction, still under this session.
            let mut persistence = StorageJobPersistence::open(&database_path)?;
            if persistence
                .snapshot(&job_id)
                .map_err(|error| error.to_string())?
                .state
                == JobState::Downloaded
            {
                persistence
                    .complete_terminal_for_attempt(
                        &job_id,
                        attempt,
                        &relative.to_string_lossy(),
                        u32::try_from(
                            serde_json::from_str::<xarchive_storage::ArchiveRecoveryManifestV2>(
                                &journal.manifest_json,
                            )
                            .map_err(|error| error.to_string())?
                            .media
                            .len(),
                        )
                        .map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
            }
            drop(session);
            Ok(true)
        };
        recover().map_err(ExecutorError::Persistence)
    })
}
