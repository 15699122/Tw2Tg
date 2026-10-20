use std::io;

#[derive(Debug)]
pub enum SupervisorError {
    Spawn(io::Error),
    NotRunning,
    Send(io::Error),
    HandshakeTimeout,
    HandshakeFailed(String),
    ShutdownIncomplete(String),
    StartupCleanupIncomplete(StartupFailureOwner),
}

/// A failed startup whose original supervisor remains available for cleanup retry.
///
/// The owner is intentionally move-only. A failed cleanup returns this same
/// value so callers cannot accidentally replace it with a reconstructed handle.
pub struct StartupFailureOwner {
    supervisor: crate::SidecarSupervisor,
    startup_error: Box<SupervisorError>,
}

impl StartupFailureOwner {
    /// Wrap a supervisor whose startup cleanup has not completed yet.
    ///
    /// The owner is move-only and must be consumed by
    /// [`StartupFailureOwner::retry_cleanup`]; dropping it is not a cleanup
    /// proof. Callers outside this crate use this to forward a retained
    /// failure owner across a boundary (for example the factory/session
    /// boundary) that cannot observe this crate's internal failure paths.
    pub fn new(supervisor: crate::SidecarSupervisor, startup_error: SupervisorError) -> Self {
        Self {
            supervisor,
            startup_error: Box::new(startup_error),
        }
    }

    /// Retry cleanup on the same supervisor. On failure, ownership is returned
    /// intact; on success, the original startup error is returned.
    pub fn retry_cleanup(mut self) -> Result<SupervisorError, Self> {
        match self.supervisor.shutdown() {
            Ok(()) => Ok(*self.startup_error),
            Err(_) => Err(self),
        }
    }

    /// The startup failure this owner was retained for, without consuming the
    /// owner.
    pub fn startup_error(&self) -> &SupervisorError {
        &self.startup_error
    }
}

impl std::fmt::Debug for StartupFailureOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StartupFailureOwner")
            .field("startup_error", &self.startup_error)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(error) => write!(formatter, "failed to spawn sidecar: {error}"),
            Self::NotRunning => formatter.write_str("sidecar is not running"),
            Self::Send(error) => write!(formatter, "failed to send sidecar command: {error}"),
            Self::HandshakeTimeout => formatter.write_str("sidecar v2 hello handshake timed out"),
            Self::HandshakeFailed(message) => {
                write!(formatter, "sidecar v2 hello handshake failed: {message}")
            }
            Self::ShutdownIncomplete(message) => {
                write!(formatter, "sidecar shutdown incomplete: {message}")
            }
            Self::StartupCleanupIncomplete(owner) => write!(
                formatter,
                "sidecar startup cleanup incomplete; retryable owner retained: {owner:?}"
            ),
        }
    }
}

impl std::error::Error for SupervisorError {}
