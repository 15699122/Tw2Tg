//! Test-only cross-process lock probe.
//!
//! The integration test spawns this binary as a genuine second OS process,
//! which is the only way to exercise `fcntl` record-lock exclusivity:
//! two acquisitions in one process never contend because records are
//! per-process. The probe acquires one scope, prints `READY`, then either
//! holds until stdin closes (`hold`) or exits without releasing (`crash`).
//!
//! The `session` mode acquires a real [`ExecutionSession`] over one job and
//! destination, so cross-process tests prove the session contract rather than
//! only the raw adapter: a live session in one process must make the same job
//! contended in another, and the session's guard must be released with the
//! process.
//!
//! It is built only with the `__lock_probe` feature and is never linked
//! into production code.
//!
//! Unix-only in substance, because the coordinator it drives is the Unix
//! adapter. Every item is individually `cfg(unix)` with a refusing `main` stub
//! for other targets, and that split is load-bearing rather than cosmetic: the
//! binary is declared with `required-features`, so a file-level `#![cfg(unix)]`
//! would remove `main` entirely and fail to link, while an ungated body would
//! fail on an import that cannot resolve. The Windows Owner validates NTFS
//! session behaviour with the native adapter instead.

#[cfg(unix)]
use std::io::Read as _;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::ExitCode;

#[cfg(unix)]
use xarchive_workflow::coordination::{Coordinator, LockScope, unix::LinuxCoordinator};
#[cfg(unix)]
use xarchive_workflow::{AttemptIdentity, ExecutionSession};

#[cfg(unix)]
fn usage() -> ExitCode {
    eprintln!(
        "usage: xarchive-lock-probe <hold|crash> <lock-dir> <job|dest> <value>\n       \
         xarchive-lock-probe <hold|crash> session <lock-dir> <job-id> <destination> <attempt>"
    );
    ExitCode::from(2)
}

/// What the probe holds for the lifetime of its run.
///
/// Both variants exist so the `session` mode exercises the real contract:
/// holding an `ExecutionSession` is what authorizes mutation, so a probe that
/// only held a raw guard would not prove that a live session blocks a peer.
#[cfg(unix)]
enum Held {
    /// One raw adapter guard. Held only so the lock stays taken; the legacy
    /// `job`/`dest` probe modes use it to prove adapter-level exclusivity,
    /// while `session` is the mode that proves the contract itself.
    #[allow(dead_code)]
    Guard(<LinuxCoordinator as Coordinator>::Guard),
    Session(ExecutionSession<'static, LinuxCoordinator>),
}

#[cfg(unix)]
impl Held {
    /// What this probe actually holds, for the test to assert against.
    ///
    /// Reporting the acquired identity is evidence, not decoration: it proves
    /// the second process established the session the test asked for, so a
    /// contention result cannot be an artifact of the probe holding something
    /// else entirely.
    fn describe(&self) -> String {
        match self {
            Self::Guard(_) => "guard:single".to_owned(),
            Self::Session(session) => format!(
                "session:job={}:attempt={}",
                session.job_id(),
                session.attempt().attempt()
            ),
        }
    }
}

#[cfg(unix)]
fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(mode) = args.next() else {
        return usage();
    };
    if mode != "hold" && mode != "crash" {
        return usage();
    }
    let Some(second) = args.next() else {
        return usage();
    };

    let held = if second == "session" {
        let (Some(lock_dir), Some(job_id), Some(destination), Some(attempt)) =
            (args.next(), args.next(), args.next(), args.next())
        else {
            return usage();
        };
        if args.next().is_some() {
            return usage();
        }
        let Ok(attempt) = attempt.parse::<u32>() else {
            return usage();
        };
        // Leaked so the session's borrow of the coordinator is `'static`:
        // the probe runs to exit, and a leak here cannot outlive the process.
        let coordinator: &'static LinuxCoordinator =
            Box::leak(Box::new(match LinuxCoordinator::new(lock_dir) {
                Ok(coordinator) => coordinator,
                Err(error) => {
                    eprintln!("lock-probe: cannot open lock directory: {error}");
                    return ExitCode::from(3);
                }
            }));
        let identity = match AttemptIdentity::new(job_id, attempt) {
            Ok(identity) => identity,
            Err(error) => {
                eprintln!("lock-probe: invalid attempt identity: {error}");
                return ExitCode::from(5);
            }
        };
        match ExecutionSession::begin(coordinator, identity, Path::new(&destination)) {
            Ok(session) => Held::Session(session),
            Err(error) => {
                eprintln!("lock-probe: cannot acquire session: {error}");
                return ExitCode::from(4);
            }
        }
    } else {
        let Some(kind) = args.next() else {
            return usage();
        };
        let lock_dir = second;
        let Some(value) = args.next() else {
            return usage();
        };
        if args.next().is_some() {
            return usage();
        }
        let scope = match kind.as_str() {
            "job" => LockScope::Job(value),
            "dest" => LockScope::Destination(PathBuf::from(value)),
            _ => return usage(),
        };
        let coordinator = match LinuxCoordinator::new(lock_dir) {
            Ok(coordinator) => coordinator,
            Err(error) => {
                eprintln!("lock-probe: cannot open lock directory: {error}");
                return ExitCode::from(3);
            }
        };
        match coordinator.acquire(&scope) {
            Ok(guard) => Held::Guard(guard),
            Err(error) => {
                eprintln!("lock-probe: cannot acquire lock: {error}");
                return ExitCode::from(4);
            }
        }
    };

    println!("READY");
    println!("HELD {}", held.describe());
    match mode.as_str() {
        "hold" => {
            let mut stdin = std::io::stdin().lock();
            let mut sink = Vec::new();
            let _ = stdin.read_to_end(&mut sink);
            drop(held);
            ExitCode::SUCCESS
        }
        _ => std::process::exit(7),
    }
}

/// Non-Unix stub so the declared binary still links where its adapter cannot.
///
/// The probe drives the Unix coordinator, so it has nothing to prove on another
/// platform. Refusing to run is the honest behaviour: a probe that silently
/// succeeded without holding anything would make a contention test pass for the
/// wrong reason. The Windows Owner validates NTFS session behaviour with the
/// native adapter instead.
#[cfg(not(unix))]
fn main() {
    eprintln!("xarchive-lock-probe: the Unix coordination adapter is unavailable here");
    std::process::exit(9);
}
