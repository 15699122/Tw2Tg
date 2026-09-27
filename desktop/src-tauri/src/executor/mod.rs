//! R1 job-executor command/state model and production control boundary.
//!
//! The module owns the bounded control worker, single active runner,
//! persistence ports, recovery contracts, execution-spec fencing, and the
//! `ExecutorRuntime` resource boundary used by RuntimeState. Production
//! execution loads immutable request specs by Job ID and creates its own
//! Database/FileStore/Sidecar context; the synchronous archive command remains
//! an explicit fallback.
//!
//! ENG-15 split the former single 4449-line file by reason-to-change without
//! changing behaviour, and the flattened re-exports keep every existing
//! `crate::executor::*` path working:
//!
//! - [`model`]: requests, snapshots, events, errors, and the persistence and
//!   execution ports the siblings implement.
//! - [`persistence`]: in-memory and storage-backed `JobPersistence`
//!   adapters, plus `ExecutorRuntime`.
//! - [`service`]: the application service and the worker/runner control loops.
//! - [`runtime`]: executor handles and the drop/shutdown boundary.
#![allow(dead_code)]

mod model;
mod persistence;
mod runtime;
mod service;

pub(crate) use model::DEFAULT_QUEUE_CAPACITY;
pub use model::*;
pub use persistence::*;
// `JobExecutor`/`JobExecutorHandle` are reached through this flattened path by
// the `transport` unit tests, so the library target alone cannot see the use.
// Keeping the re-export preserves the pre-split `crate::executor::*` surface.
#[allow(unused_imports)]
pub use runtime::*;
pub use service::*;

#[cfg(test)]
mod tests;
