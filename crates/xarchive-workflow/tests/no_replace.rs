//! Atomic no-replace move tests (Unix).
//!
//! These prove the Linux `renameat2(RENAME_NOREPLACE)` adapter moves
//! exactly once, refuses to replace, and reports missing sources.
//! Windows NTFS behaviour stays with the Windows Owner and its native
//! adapter.

#![cfg(unix)]

use std::path::PathBuf;

use xarchive_workflow::coordination::{
    MoveError, NoReplaceMover, replay_rename_stage, unix::LinuxNoReplaceMover,
};

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "xarchive-mover-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

#[test]
fn moves_exactly_once_and_refuses_to_replace() {
    let root = scratch("once");
    let mover = LinuxNoReplaceMover;
    std::fs::write(root.join("a.bin"), b"payload").expect("source");

    mover
        .move_no_replace(&root.join("a.bin"), &root.join("b.bin"))
        .expect("first move");
    assert!(!root.join("a.bin").exists());
    assert_eq!(
        std::fs::read(root.join("b.bin")).expect("destination"),
        b"payload"
    );

    std::fs::write(root.join("c.bin"), b"other").expect("second source");
    assert_eq!(
        mover.move_no_replace(&root.join("c.bin"), &root.join("b.bin")),
        Err(MoveError::DestinationExists)
    );
    assert_eq!(
        std::fs::read(root.join("b.bin")).expect("destination intact"),
        b"payload",
        "a refused move must not replace the destination"
    );
    assert_eq!(
        mover.move_no_replace(&root.join("missing.bin"), &root.join("d.bin")),
        Err(MoveError::InvalidSource)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn replay_moves_pending_edges_and_accepts_completed_ones() {
    let root = scratch("replay");
    let mover = LinuxNoReplaceMover;
    std::fs::write(root.join("one.bin"), b"1").expect("source");
    std::fs::write(root.join("two-done.bin"), b"2").expect("completed");

    replay_rename_stage(
        &mover,
        &root,
        &[
            (PathBuf::from("one.bin"), PathBuf::from("one-moved.bin")),
            (
                PathBuf::from("two-missing.bin"),
                PathBuf::from("two-done.bin"),
            ),
        ],
    )
    .expect("pending moves, completed edges accepted");
    assert!(root.join("one-moved.bin").is_file());
    assert_eq!(
        std::fs::read(root.join("two-done.bin")).expect("completed intact"),
        b"2"
    );

    assert_eq!(
        replay_rename_stage(
            &mover,
            &root,
            &[(PathBuf::from("x.bin"), PathBuf::from("x-moved.bin"))],
        ),
        Err(MoveError::InvalidSource)
    );
    assert_eq!(
        replay_rename_stage(
            &mover,
            &root,
            &[(
                PathBuf::from("one-moved.bin"),
                PathBuf::from("two-done.bin")
            )],
        ),
        Err(MoveError::DestinationExists)
    );
    let _ = std::fs::remove_dir_all(root);
}
