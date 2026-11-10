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
