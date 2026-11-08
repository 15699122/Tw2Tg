//! Attempt-owned session execution and finalization boundary.
//! Admission, scope, claim, drain, authorization and terminal ordering stay intact.

use super::{
    ArchiveApplicationService, AttemptCreationError, CancelArbitration, ExecutorError,
    ExecutorEvent, JobExecution, JobExecutionResult, JobPersistence, JobSnapshot,
};
#[cfg(target_os = "linux")]
use super::{
    LinuxExecutionContainment, containment_record_root, recover_writer_domain_set_with_evidence,
};
use std::path::PathBuf;
use xarchive_core::JobState;
use xarchive_workflow::coordination::Coordinator;
use xarchive_workflow::{AttemptIdentity, ClaimOutcome, ExecutionReservation, SessionError};

/// Outcome of one [`SessionExecutionBoundary::execute_session`] run.
///
/// The move-only startup-cleanup variant below means this enum no longer
/// derives `Clone`/`PartialEq`: outcomes are consumed per variant, and a
/// pending owner is consumed through [`resolve_session_run`].
#[derive(Debug)]
pub(crate) enum SessionRunOutcome {
    /// The attempt reached a durable decision while its session was alive.
    /// Besides terminal states this includes snapshots that lost a later
    /// arbitration (a newer attempt or a durable cancellation), which report
    /// the winning writer's state instead of overwriting it.
    Finished(JobSnapshot),
    /// The Job was not executed because a live peer owns the Job or
    /// destination scope, or won the durable claim race. Nothing is
    /// persisted as a failure; the Job stays reschedulable.
    Reschedulable {
        job_id: String,
        reason: SessionRescheduleReason,
    },
    /// CROSS_PLATFORM_CHANGE_REQUIRED: the admitted attempt reached its
    /// durable decision (or a refused decision), but Sidecar startup cleanup
    /// failed and the move-only owner still needs a same-owner retry. Consume
    /// this through [`resolve_session_run`], which retries cleanup on the
    /// original supervisor and then returns `decision`; a failed retry
    /// surfaces `SIDECAR_STARTUP_CLEANUP_INCOMPLETE` with the retained
    /// startup error instead of silently dropping the owner.
    StartupCleanupPending {
        job_id: String,
        /// The value this run would have returned once the owner is resolved.
        decision: Box<Result<SessionRunOutcome, ExecutorError>>,
        owner: xarchive_sidecar_supervisor::StartupFailureOwner,
    },
}

/// Why a session run was reschedulable instead of executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionRescheduleReason {
    /// Another live owner holds the Job or destination scope.
    Contended,
    /// Another writer won the durable claim; this caller has no attempt.
    ClaimLost,
}

/// Runner-owned execution boundary: an owned persistence context combined
/// with an owned coordinator so one attempt's `ExecutionSession` can span
/// the whole lifecycle. (`CROSS_PLATFORM_CHANGE_REQUIRED` shared contract.)
///
/// This is the owned persistence/coordinator/session boundary the C1-A
/// checkpoint identified as the missing integration surface:
///
/// - The persistence context is *owned* by the boundary, so it can be moved
///   into the scheduler's `'static` thread together with the coordinator,
///   instead of being borrowed by the scheduling caller (a borrowed
///   `JobPersistence` cannot cross the runner's `'static` queue).
/// - The coordinator is owned and generic over the [`Coordinator`] port, so
///   the same boundary compiles on Windows once the Windows Owner implements
///   the NTFS adapter; no platform `cfg` removes the session.
/// - [`SessionExecutionBoundary::execute_on_runner`] reserves runner capacity
///   and moves the stable boundary owners into one inline operation; that
///   operation invokes [`SessionExecutionBoundary::execute_session`] directly
///   on the runner thread.
/// - `execute_session` acquires the Job *and* destination scopes,
///   runs the durable attempt claim under that guard, creates the execution
///   object only *after* the claim, and holds the resulting session — and the
///   runner reservation — through the runner interaction, the fenced
///   terminal writes and archive finalization. The session is dropped last,
///   so the scopes are released only after finalization completes.
/// - Contention and a lost claim map to [`SessionRunOutcome::Reschedulable`]
///   and never persist a `FAILED`; runner saturation returns
///   [`ExecutorError::QueueFull`] before any attempt is claimed.
///
/// Unix persisted scheduling uses this boundary. Provisioned Linux execution
/// retains attempt-owned writers through drain, no-replace publication and
/// fenced finalization. Other platform gates remain independently enforced.
pub(crate) struct SessionExecutionBoundary<P, C> {
    pub(super) service: ArchiveApplicationService,
    pub(super) persistence: P,
    coordinator: C,
    archive_root: PathBuf,
    #[cfg(target_os = "linux")]
    recovery_containment: Option<(LinuxExecutionContainment, PathBuf)>,
}

/// The running-state advance table shared by the persisted execution paths.
fn running_transitions(state: JobState) -> std::vec::IntoIter<JobState> {
    match state {
        JobState::Queued | JobState::Interrupted => vec![
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
        ],
        JobState::Validating => vec![JobState::MetadataReady, JobState::Downloading],
        JobState::MetadataReady => vec![JobState::Downloading],
        JobState::Downloading => Vec::new(),
        _ => Vec::new(),
    }
    .into_iter()
}

impl<P: JobPersistence, C: Coordinator> SessionExecutionBoundary<P, C> {
    /// Build a boundary that owns its persistence context, its coordinator,
    /// and the application service whose runner executes the work.
    pub(crate) fn new(
        service: ArchiveApplicationService,
        persistence: P,
        coordinator: C,
        archive_root: PathBuf,
    ) -> Self {
        Self {
            service,
            persistence,
            coordinator,
            archive_root,
            #[cfg(target_os = "linux")]
            recovery_containment: None,
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn with_recovery_containment(
        mut self,
        containment: LinuxExecutionContainment,
        staging_root: PathBuf,
    ) -> Self {
        self.recovery_containment = Some((containment, staging_root));
        self
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn with_optional_recovery_containment(
        mut self,
        containment: Option<LinuxExecutionContainment>,
        staging_root: PathBuf,
    ) -> Self {
        self.recovery_containment = containment.map(|value| (value, staging_root));
        self
    }

    /// Transfer the stable boundary owners into one inline runner operation.
    /// The session is borrowed only after the closure reaches the runner, so no
    /// borrowed guard or coordinator reference crosses the `'static` queue.
    pub(crate) fn execute_on_runner(
        self,
        job_id: String,
    ) -> Result<SessionRunOutcome, ExecutorError>
    where
        P: Send + 'static,
        C: Send + 'static,
    {
        if !self.service.executor.runner_try_admit() {
            return Err(ExecutorError::QueueFull);
        }
        let admission = self.service.executor.owned_runner_admission();
        let runner = self.service.executor.clone();
        runner.run_operation(admission, move || {
            let mut boundary = self;
            boundary.execute_session(&job_id)
        })
    }

    /// The final archive directory this boundary binds a session to for
    /// `tweet_id`: `<archive_root>/Tweets/<tweet_id>`.
    fn destination_for(&self, tweet_id: &str) -> PathBuf {
        self.archive_root.join("Tweets").join(tweet_id)
    }

    /// Run one Job under a runner-owned session.
    ///
    /// Ordering contract, enforced by construction: runner admission in
    /// `execute_on_runner` → Job/destination scope acquisition → durable claim under the guard →
    /// factory creation → runner execution → attempt-fenced terminal writes
    /// and finalization → session release. Contention or a lost claim
    /// release both scopes without persisting anything.
    pub(super) fn execute_session(
        &mut self,
        job_id: &str,
    ) -> Result<SessionRunOutcome, ExecutorError> {
        let current = self.persistence.snapshot(job_id)?;
        if current.state.is_terminal()
            || (current.state == JobState::Interrupted && !self.service.v2_enrollment)
        {
            return Ok(SessionRunOutcome::Finished(current));
        }
        let destination = self.destination_for(&current.tweet_id);
        let reservation =
            match ExecutionReservation::acquire(&self.coordinator, job_id, &destination) {
                Ok(reservation) => reservation,
                Err(SessionError::Contended) => {
                    return Ok(SessionRunOutcome::Reschedulable {
                        job_id: job_id.to_owned(),
                        reason: SessionRescheduleReason::Contended,
                    });
                }
                Err(error) => return Err(session_establishment_error(job_id, error)),
            };
        let mut transitions = running_transitions(current.state);
        let first = transitions.next();
        // InternalV1 DOWNLOADING without a journal is a crashed predecessor,
        // not an ordinary same-state retry. Retain runner admission and both
        // scopes while the adapter drains, Storage CASes the successor, and
        // Workflow consumes the exact predecessor evidence.
        #[cfg(target_os = "linux")]
        let session = if current.state == JobState::Downloading
            && self.persistence.is_internal_v1_contract(job_id)?
        {
            if !matches!(
                self.persistence.internal_v1_attempt_phase(job_id)?,
                Some(None)
            ) {
                return Err(ExecutorError::Execution {
                    error_code: "INTERNAL_V1_RECOVERY_REQUIRED".to_owned(),
                    error_message: "journaled InternalV1 attempt requires startup recovery"
                        .to_owned(),
                    persistence_already_updated: false,
                });
            }
            let Some((_, staging_root)) = self.recovery_containment.as_ref() else {
                return Err(ExecutorError::Execution {
                    error_code: "WRITER_DOMAIN_RECOVERY_FAILED".to_owned(),
                    error_message: "InternalV1 predecessor drain is not provisioned".to_owned(),
                    persistence_already_updated: false,
                });
            };
            let predecessor_attempt = self.persistence.current_attempt(job_id)?;
            if predecessor_attempt == 0 {
                return Err(ExecutorError::Execution {
                    error_code: "INTERNAL_V1_RECOVERY_REQUIRED".to_owned(),
                    error_message: "InternalV1 DOWNLOADING predecessor has no durable attempt"
                        .to_owned(),
                    persistence_already_updated: false,
                });
            }
            let predecessor = reservation
                .bind_existing_attempt_checked(
                    AttemptIdentity::new(job_id, predecessor_attempt)
                        .map_err(|error| ExecutorError::Persistence(error.to_string()))?,
                    |identity| identity.job_id() == job_id,
                )
                .map_err(|error| session_establishment_error(job_id, error))?;
            let record_root = containment_record_root(staging_root)
                .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
            let evidence = recover_writer_domain_set_with_evidence(
                &record_root,
                predecessor.recovery_drain_binding(staging_root),
            )
            .map_err(|error| ExecutorError::Execution {
                error_code: "WRITER_DOMAIN_RECOVERY_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            })?;
            predecessor
                .take_over_drained_predecessor(evidence, |identity| {
                    self.persistence
                        .claim_internal_v1_successor(job_id, identity.attempt())
                        .map(|successor| {
                            successor.and_then(|attempt| AttemptIdentity::new(job_id, attempt).ok())
                        })
                        .map_err(|_| SessionError::ClaimRejected)
                })
                .map_err(|error| session_establishment_error(job_id, error))?
        } else {
            // Compute the first running transition before claiming; the claim
            // transaction itself re-validates state and attempt atomically.
            // The durable claim runs under the held scope guard.
            let session = match reservation.claim(|| {
                let expected_attempt = match self.persistence.current_attempt(job_id) {
                    Ok(attempt) => attempt,
                    Err(_) => return ClaimOutcome::Failed,
                };
                let identity = match first {
                    Some(to) => match self.persistence.claim_attempt(
                        job_id,
                        expected_attempt,
                        current.state,
                        to,
                    ) {
                        Ok(Some(attempt)) => attempt,
                        Ok(None) => return ClaimOutcome::Lost,
                        Err(_) => return ClaimOutcome::Failed,
                    },
                    None => match self.persistence.begin_attempt(job_id) {
                        Ok(attempt) => attempt,
                        Err(_) => return ClaimOutcome::Failed,
                    },
                };
                match AttemptIdentity::new(job_id, identity) {
                    Ok(identity) => ClaimOutcome::Claimed(identity),
                    Err(_) => ClaimOutcome::Failed,
                }
            }) {
                Ok(session) => session,
                Err(error @ (SessionError::Contended | SessionError::ClaimLost)) => {
                    let reason = match error {
                        SessionError::Contended => SessionRescheduleReason::Contended,
                        _ => SessionRescheduleReason::ClaimLost,
                    };
                    return Ok(SessionRunOutcome::Reschedulable {
                        job_id: job_id.to_owned(),
                        reason,
                    });
                }
                Err(error) => return Err(session_establishment_error(job_id, error)),
            };
            session
        };
        let publication_owner = xarchive_workflow::PublicationOwner::for_session(&session);
        let attempt = session.attempt().attempt();
        let mut running = current.clone();
        if let Some(to) = first {
            // The claim transaction already wrote the state; record its
            // event, then apply the remaining advances attempt-fenced.
            self.persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: current.state,
                    to,
                },
            )?;
            running.state = to;
        }
        for next in transitions {
            let previous = running.state;
            previous
                .transition_to(next)
                .map_err(|_| ExecutorError::InvalidTransition {
                    from: previous,
                    to: next,
                })?;
            if !self
                .persistence
                .persist_state_for_attempt(job_id, attempt, previous, next)?
            {
                // A newer attempt, a durable cancellation, or any other
                // writer already moved the row; this attempt must stop
                // writing and report the winning state.
                return Ok(SessionRunOutcome::Finished(
                    self.persistence.snapshot(job_id)?,
                ));
            }
            running.state = next;
            self.persistence.record_event(
                job_id,
                ExecutorEvent::StateChanged {
                    from: previous,
                    to: next,
                },
            )?;
        }
        // The execution factory and runner remain in the object-safe production
        // seam; the session stays live through publication and finalization.
        // We are already executing on the single runner thread. Invoke the
        // owned execution directly; dispatching via execute_owned here would
        // enqueue behind this operation and self-deadlock. `execution` is
        // intentionally kept alive past the completion decision below so the
        // attempt-owned owners (Sidecar, aria2, Rust writers) are not dropped
        // before the session has finished finalizing.
        let mut execution: Option<Box<dyn JobExecution>> = None;
        // Keep the exact Storage/process owners alive through attempt-fenced
        // terminal writes, even after runner-local publication has returned.
        let mut finalized_publication: Option<Box<crate::archive::PendingArchivePublication>> =
            None;
        // A retained startup-cleanup owner stays with this admitted attempt on
        // every exit path below: explicit exits defer it through the pending
        // outcome, and an unexpected `?` error still attempts the same-owner
        // cleanup (best effort, never a proof) when this guard drops.
        let mut retained_startup_owner = DeferredStartupCleanup { owner: None };
        let cancellation = self.service.executor.cancellation_for(job_id)?;
        let run_result: Result<JobExecutionResult, ExecutorError> =
            match self.service.executor.factory.create_registered_attempt(
                session.attempt(),
                &running,
                publication_owner.registration(),
            ) {
                Ok(exec) => {
                    execution = Some(exec);
                    execution
                        .as_mut()
                        .unwrap()
                        .execute(&running, &cancellation)
                        .map_err(|error| ExecutorError::Execution {
                            error_code: error.error_code,
                            error_message: error.error_message,
                            persistence_already_updated: error.persistence_already_updated,
                        })
                }
                Err(AttemptCreationError::Failed(error)) => Err(error),
                Err(AttemptCreationError::StartupCleanupPending(owner)) => {
                    let startup_error = owner.startup_error().to_string();
                    retained_startup_owner.owner = Some(owner);
                    Err(ExecutorError::Execution {
                        error_code: "SIDECAR_START_FAILED".to_owned(),
                        error_message: startup_error,
                        persistence_already_updated: false,
                    })
                }
            };
        let run_result =
            run_result.and_then(|mut result| match result.pending_publication.take() {
                Some(pending) => {
                    finalized_publication = Some(pending);
                    let finalized = finalized_publication
                        .as_mut()
                        .expect("pending publication retained by runner")
                        .finalize(&session, &publication_owner, &cancellation)
                        .map_err(|error| ExecutorError::Execution {
                            error_code: error.error_code,
                            error_message: error.error_message,
                            persistence_already_updated: error.persistence_already_updated,
                        })?;
                    Ok(finalized)
                }
                None => Ok(result),
            });
        let outcome = match run_result {
            Ok(result) => {
                let current = self.persistence.snapshot(job_id)?;
                if current.state == JobState::Interrupted {
                    return Ok(SessionRunOutcome::Finished(current));
                }
                if !self.persistence.attempt_is_current(job_id, attempt)? {
                    return Ok(SessionRunOutcome::Finished(current));
                }
                // Fence the terminal download write on this attempt too.
                if current.state != JobState::Downloaded
                    && !self.persistence.persist_state_for_attempt(
                        job_id,
                        attempt,
                        JobState::Downloading,
                        JobState::Downloaded,
                    )?
                {
                    return Ok(SessionRunOutcome::Finished(
                        self.persistence.snapshot(job_id)?,
                    ));
                }
                self.persistence.record_event(
                    job_id,
                    ExecutorEvent::StateChanged {
                        from: JobState::Downloading,
                        to: JobState::Downloaded,
                    },
                )?;
                // Atomically record DownloadCompleted + Complete + Completed
                // in one transaction, fenced by attempt. Idempotent: if the
                // job is already Complete, no writes occur; if the attempt no
                // longer owns the job, the transition is refused.
                let completed = self.persistence.complete_terminal_for_attempt(
                    job_id,
                    attempt,
                    &result.archive_directory,
                    result.files_copied,
                )?;
                SessionRunOutcome::Finished(completed)
            }
            Err(error) => {
                let current = self.persistence.snapshot(job_id)?;
                if cancellation.is_cancelled() && !current.state.is_terminal() {
                    let decision = self.persistence.persist_cancellation(job_id)?;
                    match decision {
                        CancelArbitration::Cancelled => {
                            self.persistence.record_event(
                                job_id,
                                ExecutorEvent::StateChanged {
                                    from: current.state,
                                    to: JobState::Cancelled,
                                },
                            )?;
                            return retained_startup_owner.defer(
                                job_id,
                                Ok(SessionRunOutcome::Finished(
                                    self.persistence.snapshot(job_id)?,
                                )),
                            );
                        }
                        CancelArbitration::AlreadyDecided => {
                            return retained_startup_owner.defer(
                                job_id,
                                Ok(SessionRunOutcome::Finished(
                                    self.persistence.snapshot(job_id)?,
                                )),
                            );
                        }
                        CancelArbitration::CommitWins => {}
                    }
                }
                if matches!(current.state, JobState::Cancelled | JobState::Interrupted) {
                    return retained_startup_owner
                        .defer(job_id, Ok(SessionRunOutcome::Finished(current)));
                }
                let current = self.persistence.snapshot(job_id)?;
                if matches!(current.state, JobState::Cancelled | JobState::Interrupted) {
                    return retained_startup_owner
                        .defer(job_id, Ok(SessionRunOutcome::Finished(current)));
                }
                if !self.persistence.attempt_is_current(job_id, attempt)? {
                    return retained_startup_owner.defer(
                        job_id,
                        Ok(SessionRunOutcome::Finished(
                            self.persistence.snapshot(job_id)?,
                        )),
                    );
                }
                if let ExecutorError::Execution {
                    persistence_already_updated: true,
                    ..
                } = &error
                {
                    return retained_startup_owner.defer(
                        job_id,
                        Ok(SessionRunOutcome::Finished(
                            self.persistence.snapshot(job_id)?,
                        )),
                    );
                }
                // Publication can fail after PREPARED (rename, directory move,
                // or DB finalization). The journal owns that attempt: retain
                // its candidate state for startup replay instead of hiding it
                // behind FAILED, which is not a startup recovery candidate.
                if self.persistence.archive_commit_started(job_id)? {
                    return retained_startup_owner.defer(job_id, Err(error));
                }
                let message = error.to_string();
                self.persistence
                    .fail(job_id, "EXECUTOR_WORKER_FAILED", &message)?;
                self.persistence.record_event(
                    job_id,
                    ExecutorEvent::DownloadFailed {
                        error_code: "EXECUTOR_WORKER_FAILED".to_owned(),
                        error_message: message,
                    },
                )?;
                SessionRunOutcome::Finished(self.persistence.snapshot(job_id)?)
            }
        };
        // `execution` is deliberately held past the completion decision above
        // so the attempt-owned owners (Sidecar, aria2, Rust writers) are not
        // dropped before the session has finished finalizing.
        let _ = (execution, finalized_publication);
        // The borrowed session releases its guard before an enclosing owned
        // runner operation drops the boundary's coordinator and admission.
        // The retained startup owner (if any) crosses this boundary here with
        // the decision it belongs to.
        retained_startup_owner.defer(job_id, Ok(outcome))
    }
}

/// Keeps a retained startup-cleanup owner alive for the rest of one
/// `execute_session` run.
///
/// Explicit exits call [`Self::defer`] so the owner crosses the outcome
/// boundary move-only together with the decision it belongs to. Any other
/// exit (a persistence `?` error) drops the guard, which still attempts the
/// same-owner cleanup as a best-effort fallback — a Drop attempt is never a
/// cleanup proof, only damage control.
struct DeferredStartupCleanup {
    owner: Option<xarchive_sidecar_supervisor::StartupFailureOwner>,
}

impl DeferredStartupCleanup {
    /// Consume the guard. With no retained owner this is the identity of
    /// `decision`; with an owner it wraps `decision` into
    /// [`SessionRunOutcome::StartupCleanupPending`] for
    /// [`resolve_session_run`].
    fn defer(
        mut self,
        job_id: &str,
        decision: Result<SessionRunOutcome, ExecutorError>,
    ) -> Result<SessionRunOutcome, ExecutorError> {
        let Some(owner) = self.owner.take() else {
            return decision;
        };
        Ok(SessionRunOutcome::StartupCleanupPending {
            job_id: job_id.to_owned(),
            decision: Box::new(decision),
            owner,
        })
    }
}

impl Drop for DeferredStartupCleanup {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            let _ = owner.retry_cleanup();
        }
    }
}

/// Consume a pending startup-cleanup owner with a same-owner retry and
/// return the retained decision.
///
/// This is the one sanctioned crossing point for the move-only owner: a
/// successful retry proves local cleanup on the original supervisor before
/// the durable decision is observed; a failed retry reports
/// `SIDECAR_STARTUP_CLEANUP_INCOMPLETE` with the retained startup error
/// instead of silently dropping the owner, while the decision behind it
/// stays exactly as `execute_session` recorded it.
pub(crate) fn resolve_session_run(
    outcome: SessionRunOutcome,
) -> Result<SessionRunOutcome, ExecutorError> {
    match outcome {
        SessionRunOutcome::StartupCleanupPending {
            job_id,
            decision,
            owner,
        } => match owner.retry_cleanup() {
            Ok(_startup_error) => *decision,
            Err(owner) => {
                let persistence_already_updated =
                    matches!(decision.as_ref(), Ok(SessionRunOutcome::Finished(_)));
                Err(ExecutorError::Execution {
                    error_code: "SIDECAR_STARTUP_CLEANUP_INCOMPLETE".to_owned(),
                    error_message: format!(
                        "sidecar startup cleanup for {job_id} incomplete after a \
                         same-owner retry: {owner:?}"
                    ),
                    persistence_already_updated,
                })
            }
        },
        other => Ok(other),
    }
}

/// Map a session-establishment failure that is neither contention nor a lost
/// claim onto an executor error. These are refusal paths: no `FAILED` is
/// persisted, because the Job itself did not fail to execute — it never
/// obtained a legitimate attempt.
fn session_establishment_error(job_id: &str, error: SessionError) -> ExecutorError {
    ExecutorError::Execution {
        error_code: "EXECUTION_SESSION_FAILED".to_owned(),
        error_message: format!("could not establish execution session for {job_id}: {error}"),
        persistence_already_updated: false,
    }
}
