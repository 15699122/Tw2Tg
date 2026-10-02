//! Listener-scoped, in-memory browser credentials. Never expose this store in diagnostics.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use xarchive_protocol::{
    BROWSER_PAIRING_PROTOCOL_VERSION, BrowserPairingRequest, BrowserPairingResponse,
};

pub(crate) const EXTENSION_ORIGIN: &str = "chrome-extension://iaajefkoanbkleojofoadeakelihbjne";
const TTL: Duration = Duration::from_secs(30);
const CAPACITY: usize = 64;

pub(crate) struct PairingCoordinator {
    port: u16,
    runtime_instance_id: String,
    tickets: Mutex<Tickets>,
}
#[derive(Default)]
struct Tickets {
    issued: HashMap<String, Instant>,
    last_issue: Option<Instant>,
    stopped: bool,
}
fn random_id() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "pairing entropy unavailable".to_owned())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
impl PairingCoordinator {
    pub(crate) fn new(port: u16) -> Result<Self, String> {
        Ok(Self {
            port,
            runtime_instance_id: random_id()?,
            tickets: Mutex::new(Tickets::default()),
        })
    }
    pub(crate) fn runtime_instance_id(&self) -> &str {
        &self.runtime_instance_id
    }
    pub(crate) fn bootstrap(&self, request: BrowserPairingRequest) -> BrowserPairingResponse {
        let BrowserPairingRequest::Bootstrap { request_id, .. } = &request;
        let failure = |code: &str, retryable| BrowserPairingResponse::Error {
            protocol_version: BROWSER_PAIRING_PROTOCOL_VERSION,
            request_id: Some(request_id.clone()),
            error_code: code.to_owned(),
            error_message: "Desktop bootstrap unavailable".to_owned(),
            retryable,
        };
        if request.validate().is_err() {
            return failure("INVALID_REQUEST", false);
        }
        let Ok(mut store) = self.tickets.lock() else {
            return failure("DESKTOP_NOT_READY", true);
        };
        if store.stopped {
            return failure("DESKTOP_NOT_READY", true);
        }
        let now = Instant::now();
        store.issued.retain(|_, expires| *expires > now);
        if store.issued.len() >= CAPACITY
            || store
                .last_issue
                .is_some_and(|last| now.duration_since(last) < Duration::from_millis(100))
        {
            return failure("BOOTSTRAP_RATE_LIMIT", true);
        }
        let Ok(ticket) = random_id() else {
            return failure("DESKTOP_NOT_READY", true);
        };
        store.issued.insert(ticket.clone(), now + TTL);
        store.last_issue = Some(now);
        BrowserPairingResponse::Bootstrap {
            protocol_version: BROWSER_PAIRING_PROTOCOL_VERSION,
            request_id: request_id.clone(),
            host: "127.0.0.1".to_owned(),
            port: self.port,
            path: "/".to_owned(),
            runtime_instance_id: self.runtime_instance_id.clone(),
            ticket,
            expires_in_ms: TTL.as_millis() as u32,
        }
    }
    pub(crate) fn consume(&self, ticket: &str, origin: &str) -> bool {
        if origin != EXTENSION_ORIGIN {
            return false;
        }
        let Ok(mut store) = self.tickets.lock() else {
            return false;
        };
        !store.stopped
            && store
                .issued
                .remove(ticket)
                .is_some_and(|expires| expires > Instant::now())
    }
    pub(crate) fn stop(&self) {
        if let Ok(mut store) = self.tickets.lock() {
            store.stopped = true;
            store.issued.clear();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    fn request() -> BrowserPairingRequest {
        BrowserPairingRequest::Bootstrap {
            protocol_version: 1,
            request_id: "r1".into(),
        }
    }
    fn ticket(coordinator: &PairingCoordinator) -> String {
        let response = coordinator.bootstrap(request());
        assert_eq!(response.validate(), Ok(()));
        let BrowserPairingResponse::Bootstrap { ticket, .. } = response else {
            panic!("bootstrap");
        };
        ticket
    }
    #[test]
    fn origin_expiry_replay_and_generation_are_enforced() {
        let first = PairingCoordinator::new(43127).unwrap();
        let second = PairingCoordinator::new(43128).unwrap();
        assert_ne!(first.runtime_instance_id(), second.runtime_instance_id());
        let value = ticket(&first);
        assert!(!first.consume(&value, "https://x.com"));
        assert!(!second.consume(&value, EXTENSION_ORIGIN));
        assert!(first.consume(&value, EXTENSION_ORIGIN));
        assert!(!first.consume(&value, EXTENSION_ORIGIN));
        first
            .tickets
            .lock()
            .unwrap()
            .issued
            .insert("expired".into(), Instant::now() - Duration::from_millis(1));
        assert!(!first.consume("expired", EXTENSION_ORIGIN));
    }
    #[test]
    fn concurrent_consumers_have_exactly_one_winner() {
        let coordinator = Arc::new(PairingCoordinator::new(43127).unwrap());
        let value = ticket(&coordinator);
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let coordinator = coordinator.clone();
                let value = value.clone();
                std::thread::spawn(move || coordinator.consume(&value, EXTENSION_ORIGIN))
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .filter_map(|thread| thread.join().ok())
                .filter(|valid| *valid)
                .count(),
            1
        );
    }
    #[test]
    fn shutdown_and_issue_limits_fail_closed() {
        let coordinator = PairingCoordinator::new(43127).unwrap();
        let value = ticket(&coordinator);
        assert!(matches!(
            coordinator.bootstrap(request()),
            BrowserPairingResponse::Error { .. }
        ));
        coordinator.stop();
        assert!(!coordinator.consume(&value, EXTENSION_ORIGIN));
        assert!(matches!(
            coordinator.bootstrap(request()),
            BrowserPairingResponse::Error { .. }
        ));
        let other = PairingCoordinator::new(43127).unwrap();
        let mut store = other.tickets.lock().unwrap();
        for index in 0..CAPACITY {
            store.issued.insert(index.to_string(), Instant::now() + TTL);
        }
        drop(store);
        assert!(matches!(
            other.bootstrap(request()),
            BrowserPairingResponse::Error { .. }
        ));
    }
}
