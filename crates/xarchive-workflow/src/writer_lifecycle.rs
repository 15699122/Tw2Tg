//! Explicit writer lifetime contract. Drop is not evidence of termination.
//! Adapters must confirm all descendants have exited, not just the parent.

use crate::AttemptIdentity;

/// Producer-owned writer roles inside one attempt.
///
/// This is the single shared source of truth for the writer domains a
/// production attempt may create. Successor recovery enumerates exactly
/// [`WriterRole::ALL`] and every production spawn names one of these variants,
/// so the recovery set and the spawn sites cannot drift apart by editing two
/// independent string lists. The string form is the stable domain label handed
/// to a platform containment adapter; it is not itself a recovery credential.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum WriterRole {
    Sidecar,
    Aria2Primary,
    Aria2Refresh,
}

impl WriterRole {
    /// Every writer role one attempt may create, in stable recovery order.
    pub const ALL: [WriterRole; 3] = [
        WriterRole::Sidecar,
        WriterRole::Aria2Primary,
        WriterRole::Aria2Refresh,
    ];

    /// Stable domain label for this role.
    pub fn as_str(self) -> &'static str {
        match self {
            WriterRole::Sidecar => "sidecar",
            WriterRole::Aria2Primary => "aria2-0",
            WriterRole::Aria2Refresh => "aria2-1",
        }
    }
}

impl std::fmt::Display for WriterRole {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
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
    fn writer_roles_are_unique_and_stable_across_recovery() {
        // Successor recovery enumerates exactly `WriterRole::ALL`, so the set
        // must have no duplicates and the labels must be the stable domain names
        // that spawn sites already hand to the platform containment adapter.
        let mut seen = std::collections::HashSet::new();
        for role in WriterRole::ALL {
            assert!(seen.insert(role.as_str()), "duplicate writer role label");
        }
        assert_eq!(seen.len(), WriterRole::ALL.len());
        assert_eq!(WriterRole::Sidecar.as_str(), "sidecar");
        assert_eq!(WriterRole::Aria2Primary.as_str(), "aria2-0");
        assert_eq!(WriterRole::Aria2Refresh.as_str(), "aria2-1");
    }

    #[test]
    fn writer_role_display_matches_its_stable_label() {
        for role in WriterRole::ALL {
            assert_eq!(role.to_string(), role.as_str());
        }
    }
}
