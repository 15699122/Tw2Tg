//! Test-only cross-process lock probe.
//!
//! The integration test spawns this binary as a genuine second OS process,
//! which is the only way to exercise `fcntl` record-lock exclusivity:
//! two acquisitions in one process never contend because records are
//! per-process. The probe acquires one scope, prints `READY`, then either
//! holds until stdin closes (`hold`) or exits without releasing (`crash`).
//! It is built only with the `__lock_probe` feature and is never linked
//! into production code.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use xarchive_workflow::coordination::{Coordinator, LockScope, unix::LinuxCoordinator};

fn usage() -> ExitCode {
    eprintln!("usage: xarchive-lock-probe <hold|crash> <lock-dir> <job|dest> <value>");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(mode), Some(lock_dir), Some(kind), Some(value)) =
        (args.next(), args.next(), args.next(), args.next())
    else {
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
    let guard = match coordinator.acquire(&scope) {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("lock-probe: cannot acquire lock: {error}");
            return ExitCode::from(4);
        }
    };
    println!("READY");
    match mode.as_str() {
        "hold" => {
            let mut stdin = std::io::stdin().lock();
            let mut sink = Vec::new();
            let _ = stdin.read_to_end(&mut sink);
            drop(guard);
            ExitCode::SUCCESS
        }
        "crash" => std::process::exit(7),
        _ => usage(),
    }
}
