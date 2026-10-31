//! Explicit writer lifetime contract. Drop is not evidence of termination.
//! Adapters must confirm all descendants have exited, not just the parent.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::AttemptIdentity;

/// Error when minting or validating a publication permit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationError {
    /// The drained writers' attempt does not match the session's attempt.
    AttemptMismatch,
    /// The permit's destination does not match the publication directory.
    DestinationMismatch,
    IncompleteDrain,
    SessionMismatch,
    AlreadyPublished,
}

/// Platform adapter owning an exclusively assigned writer domain.
pub trait WriterContainment {
    type Error;

    /// Start the target only after the containment boundary is established.
    fn start(&mut self) -> Result<(), Self::Error>;

    /// Block new starts, stop writers, and confirm the entire domain is empty.
    /// Must also work after a partially failed start.
    fn stop_and_drain(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterState {
    Ready,
    Starting,
    Running,
    Stopping,
    Drained,
}

#[derive(Debug)]
pub enum WriterLifecycleError<E> {
    InvalidState,
    Adapter(E),
}

/// Non-cloneable evidence tied to the owner borrow, not a publication permit.
pub struct DrainedWriters<'a> {
    identity: &'a AttemptIdentity,
}

impl DrainedWriters<'_> {
    pub fn identity(&self) -> &AttemptIdentity {
        self.identity
    }
}

/// Move-only publication authority borrowing both the live session and its
/// exact writer-set owner. Only `PublicationOwner::authorize` can construct it.
#[derive(Debug)]
pub struct PublicationPermit<'a> {
    attempt: &'a AttemptIdentity,
    destination: &'a Path,
    _owner: &'a PublicationOwner,
}

impl PublicationPermit<'_> {
    pub fn attempt(&self) -> &AttemptIdentity {
        self.attempt
    }

    pub fn destination(&self) -> &Path {
        self.destination
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProductionWriters {
    pub(crate) closed: bool,
    published: bool,
    pub(crate) roles: std::collections::BTreeMap<String, bool>,
}

impl ProductionWriters {
    pub(crate) fn mark_drained(&mut self, role: &str) -> Result<(), PublicationError> {
        let drained = self
            .roles
            .get_mut(role)
            .ok_or(PublicationError::IncompleteDrain)?;
        *drained = true;
        Ok(())
    }
}

/// Fixed production writer-set owner, created only from a live session before
/// factory startup. Registration is Linux-adapter-specific, not a user trait.
#[derive(Debug)]
pub struct PublicationOwner {
    source: Arc<()>,
    attempt: AttemptIdentity,
    state: Arc<Mutex<ProductionWriters>>,
}

/// Factory capability: registers concrete kernel-contained writers but cannot
/// mint publication authorization. Clones share permanently closed admission.
#[derive(Clone)]
pub struct WriterRegistration {
    source: Arc<()>,
    attempt: AttemptIdentity,
    state: Arc<Mutex<ProductionWriters>>,
}

impl PublicationOwner {
    pub fn for_session<C: crate::coordination::Coordinator>(
        session: &crate::ExecutionSession<'_, C>,
    ) -> Self {
        Self {
            source: session.publication_source.clone(),
            attempt: session.attempt().clone(),
            state: Arc::new(Mutex::new(ProductionWriters::default())),
        }
    }
    pub fn registration(&self) -> WriterRegistration {
        WriterRegistration {
            source: self.source.clone(),
            attempt: self.attempt.clone(),
            state: self.state.clone(),
        }
    }
    pub fn close_admission(&self) -> Result<(), PublicationError> {
        self.state.lock().map_err(|_| PublicationError::IncompleteDrain)?.closed = true;
        Ok(())
    }
    pub fn authorize<'a, C: crate::coordination::Coordinator>(
        &'a self, session: &'a crate::ExecutionSession<'_, C>,
    ) -> Result<PublicationPermit<'a>, PublicationError> {
        if !Arc::ptr_eq(&self.source, &session.publication_source) {
            return Err(PublicationError::SessionMismatch);
        }
        if &self.attempt != session.attempt() {
            return Err(PublicationError::AttemptMismatch);
        }
        let mut state = self.state.lock().map_err(|_| PublicationError::IncompleteDrain)?;
        if state.published { return Err(PublicationError::AlreadyPublished); }
        if !state.closed
            || state.roles.get("sidecar") != Some(&true)
            || state.roles.values().any(|drained| !drained)
        {
            return Err(PublicationError::IncompleteDrain);
        }
        state.published = true;
        Ok(PublicationPermit { attempt: session.attempt(), destination: session.destination(), _owner: self })
    }
}

impl WriterRegistration {
    pub fn same_owner(&self, owner: &PublicationOwner) -> bool {
        Arc::ptr_eq(&self.source, &owner.source) && Arc::ptr_eq(&self.state, &owner.state)
    }

    #[cfg(target_os = "linux")]
    pub fn register_linux(
        &self,
        containment: &mut crate::linux_containment::LinuxContainment,
        identity: &crate::linux_containment::AttemptIdentity,
    ) -> std::io::Result<()> {
        if identity.job_id() != self.attempt.job_id()
            || identity.attempt() != u64::from(self.attempt.attempt())
        {
            return Err(std::io::Error::other("writer registration attempt mismatch"));
        }
        let role = identity.writer();
        if !matches!(role, "sidecar" | "aria2-0" | "aria2-1") {
            return Err(std::io::Error::other("unknown production writer role"));
        }
        if !containment.matches_recorded(identity) {
            return Err(std::io::Error::other(
                "writer registration requires an unstarted recorded owner",
            ));
        }
        let mut state = self.state.lock().map_err(|_| std::io::Error::other("writer registry poisoned"))?;
        if state.closed || state.roles.contains_key(role) {
            return Err(std::io::Error::other(
                "writer admission closed or duplicate role",
            ));
        }
        state.roles.insert(role.to_owned(), false);
        containment.register_production(self.state.clone(), role.to_owned());
        Ok(())
    }
}

/// Single-owner, single-start lifecycle. No adapter or handle escapes.
pub struct WriterLifecycle<C> {
    identity: AttemptIdentity,
    containment: C,
    state: WriterState,
}

impl<C: WriterContainment> WriterLifecycle<C> {
    pub fn new(identity: AttemptIdentity, containment: C) -> Self {
        Self {
            identity,
            containment,
            state: WriterState::Ready,
        }
    }

    pub fn state(&self) -> WriterState {
        self.state
    }

    pub fn start(&mut self) -> Result<(), WriterLifecycleError<C::Error>> {
        if self.state != WriterState::Ready {
            return Err(WriterLifecycleError::InvalidState);
        }
        self.state = WriterState::Starting;
        self.containment
            .start()
            .map_err(WriterLifecycleError::Adapter)?;
        self.state = WriterState::Running;
        Ok(())
    }

    /// Retry failed cleanup, but never reopen admission after cleanup begins.
    pub fn stop_and_drain(&mut self) -> Result<DrainedWriters<'_>, WriterLifecycleError<C::Error>> {
        if self.state != WriterState::Drained {
            self.state = WriterState::Stopping;
            self.containment
                .stop_and_drain()
                .map_err(WriterLifecycleError::Adapter)?;
            self.state = WriterState::Drained;
        }
        Ok(DrainedWriters {
            identity: &self.identity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        fail_start: bool,
        fail_stop: bool,
    }
    impl WriterContainment for Fake {
        type Error = ();
        fn start(&mut self) -> Result<(), ()> {
            if self.fail_start { Err(()) } else { Ok(()) }
        }
        fn stop_and_drain(&mut self) -> Result<(), ()> {
            if std::mem::take(&mut self.fail_stop) {
                Err(())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn failed_start_requires_cleanup_and_cannot_restart() {
        let identity = AttemptIdentity::new("job", 1).unwrap();
        let mut owner = WriterLifecycle::new(
            identity.clone(),
            Fake {
                fail_start: true,
                fail_stop: false,
            },
        );
        assert!(owner.start().is_err());
        assert_eq!(owner.state(), WriterState::Starting);
        assert!(matches!(
            owner.start(),
            Err(WriterLifecycleError::InvalidState)
        ));
        assert_eq!(owner.stop_and_drain().unwrap().identity(), &identity);
        assert_eq!(owner.state(), WriterState::Drained);
    }

    #[test]
    fn failed_drain_never_proves_exit_and_retry_keeps_admission_closed() {
        let mut owner = WriterLifecycle::new(
            AttemptIdentity::new("job", 2).unwrap(),
            Fake {
                fail_start: false,
                fail_stop: true,
            },
        );
        owner.start().unwrap();
        assert!(owner.start().is_err());
        assert!(owner.stop_and_drain().is_err());
        assert_eq!(owner.state(), WriterState::Stopping);
        assert!(owner.start().is_err());
        owner.stop_and_drain().unwrap();
        owner.stop_and_drain().unwrap();
        assert_eq!(owner.state(), WriterState::Drained);
        assert!(owner.start().is_err());
    }

    #[test]
    fn production_drain_update_requires_a_registered_role() {
        let mut writers = ProductionWriters::default();
        writers.roles.insert("sidecar".to_owned(), false);
        writers.mark_drained("sidecar").unwrap();
        assert_eq!(writers.roles.get("sidecar"), Some(&true));
        assert_eq!(
            writers.mark_drained("unknown"),
            Err(PublicationError::IncompleteDrain)
        );
    }

    #[test]
    fn admission_close_does_not_erase_a_later_verified_drain() {
        let mut writers = ProductionWriters::default();
        writers.roles.insert("sidecar".to_owned(), false);
        writers.closed = true;
        writers.mark_drained("sidecar").unwrap();
        assert!(writers.closed);
        assert_eq!(writers.roles.get("sidecar"), Some(&true));
    }
}
