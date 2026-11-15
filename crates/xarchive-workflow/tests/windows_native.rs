#![cfg(windows)]
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use xarchive_workflow::{
    coordination::{
        CoordinationGuard, Coordinator, LockError, LockScope, MoveError, NoReplaceMover,
        windows::{WindowsCoordinator, WindowsNoReplaceMover},
    },
    maintenance::WorkspaceGuard,
    windows_filesystem::PinnedPath,
};

fn fixture(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::var_os("TW2TG_WINDOWS_FIXTURE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = base.join(format!(
        "tw2tg-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn shared_readers_close_admission_drain_cancel_and_reopen() {
    let root = fixture("maintenance");
    let a = WorkspaceGuard::try_enter(&root).unwrap();
    let b = WorkspaceGuard::try_enter(&root).unwrap();
    let polls = std::cell::Cell::new(0);
    assert!(
        WorkspaceGuard::drain(&root, || {
            polls.set(polls.get() + 1);
            if polls.get() == 2 {
                assert!(matches!(
                    WorkspaceGuard::try_enter(&root),
                    Err(LockError::Contended)
                ));
            }
            polls.get() > 3
        })
        .is_err()
    );
    let c = WorkspaceGuard::try_enter(&root).unwrap();
    drop((a, b, c));
    let exclusive = WorkspaceGuard::drain(&root, || false).unwrap();
    assert!(matches!(
        WorkspaceGuard::try_enter(&root),
        Err(LockError::Contended)
    ));
    assert!(WorkspaceGuard::drain(&root, || true).is_err());
    drop(exclusive);
    drop(WorkspaceGuard::try_enter(&root).unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coordinator_scope_contention_and_partial_rollback() {
    let root = fixture("scopes");
    let one = WindowsCoordinator::new(&root).unwrap();
    let two = WindowsCoordinator::new(&root).unwrap();
    let a = LockScope::Job("a".into());
    let z = LockScope::Job("z".into());
    let held = one.acquire(&z).unwrap();
    assert!(matches!(
        two.acquire_all(&[a.clone(), z.clone()]),
        Err(LockError::Contended)
    ));
    two.acquire(&a).unwrap().release().unwrap();
    assert!(matches!(two.acquire(&z), Err(LockError::Contended)));
    drop(held);
    two.acquire_all(&[a.clone(), z, a])
        .unwrap()
        .release()
        .unwrap();
    let destination = LockScope::Destination(root.join("archive"));
    let held = one.acquire(&destination).unwrap();
    let alias = LockScope::Destination(root.join("ARCHIVE"));
    assert!(matches!(two.acquire(&alias), Err(LockError::Contended)));
    drop((held, one, two));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn evidence_handles_prevent_write_and_directory_replacement() {
    let root = fixture("pins");
    fs::write(root.join("media.bin"), b"original").unwrap();
    let pin = PinnedPath::media(&root.join("media.bin")).unwrap();
    assert!(fs::write(root.join("media.bin"), b"changed").is_err());
    assert!(fs::rename(&root, root.with_extension("moved")).is_err());
    assert!(fs::remove_file(root.join("media.bin")).is_err());
    drop(pin);
    fs::write(root.join("media.bin"), b"changed").unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn no_replace_preserves_existing_destination() {
    let root = fixture("publish");
    let staging = root.join("staging");
    let destination = root.join("archive");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("archive.json"), b"new").unwrap();
    fs::write(&destination, b"existing").unwrap();
    assert!(
        WindowsNoReplaceMover
            .move_no_replace(&staging, &destination)
            .is_err()
    );
    assert_eq!(fs::read(&destination).unwrap(), b"existing");
    assert!(staging.exists());
    fs::remove_file(&destination).unwrap();
    WindowsNoReplaceMover
        .move_no_replace(&staging, &destination)
        .unwrap();
    assert_eq!(fs::read(destination.join("archive.json")).unwrap(), b"new");
    assert_eq!(
        WindowsNoReplaceMover.move_no_replace(&staging, &destination),
        Err(MoveError::InvalidSource)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn no_replace_publishes_beyond_max_path() {
    use std::os::windows::ffi::OsStrExt;

    let root = fixture("long-path");
    let mut parent = root.clone();
    for _ in 0..6 {
        parent.push("archive-publication-long-path-component-12345678");
    }
    let staging = parent.join("staging");
    let destination = parent.join("published");
    assert!(destination.as_os_str().encode_wide().count() > 260);
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join("tweet.json"), b"export").unwrap();
    WindowsNoReplaceMover
        .move_no_replace(&staging, &destination)
        .unwrap();
    assert_eq!(fs::read(destination.join("tweet.json")).unwrap(), b"export");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn junction_ancestors_and_leaf_lock_reparse_are_rejected() {
    use std::os::windows::process::CommandExt;
    let root = fixture("junction");
    let target = root.join("real");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("file"), b"safe").unwrap();
    let alias = root.join("alias");
    let output = std::process::Command::new("cmd.exe")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&alias)
        .arg(&target)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction fixture unavailable: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(PinnedPath::directory(&alias, false).is_err());
    assert!(PinnedPath::media(&alias.join("file")).is_err());
    assert!(WindowsCoordinator::new(&alias).is_err());
    assert!(WorkspaceGuard::try_enter(&alias).is_err());
    let coordinator = WindowsCoordinator::new(&root).unwrap();
    fs::rename(&alias, root.join("alias.lock")).unwrap();
    assert!(matches!(
        coordinator.acquire(&LockScope::Job("alias".into())),
        Err(LockError::Io)
    ));
    drop(coordinator);
    fs::remove_dir(root.join("alias.lock")).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn owner_process() {
    let Some(path) = std::env::var_os("TW2TG_WINDOWS_LOCK_CHILD") else {
        return;
    };
    let root = PathBuf::from(path);
    let _guard = WorkspaceGuard::drain(&root, || false).unwrap();
    let _scope = WindowsCoordinator::new(&root)
        .unwrap()
        .acquire(&LockScope::Destination(root.join("archive")))
        .unwrap();
    fs::write(root.join("ready"), b"ready").unwrap();
    loop {
        std::thread::park();
    }
}

#[test]
fn killed_process_releases_both_scope_and_admission() {
    let root = fixture("crash");
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "owner_process", "--nocapture"])
        .env("TW2TG_WINDOWS_LOCK_CHILD", &root)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("ready").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = root.join("ready").exists();
    let admission_blocked = matches!(WorkspaceGuard::try_enter(&root), Err(LockError::Contended));
    let coordinator = WindowsCoordinator::new(&root).unwrap();
    let destination = LockScope::Destination(root.join("ARCHIVE"));
    let destination_blocked =
        matches!(coordinator.acquire(&destination), Err(LockError::Contended));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(
        ready && admission_blocked && destination_blocked,
        "child must own the real locks before termination"
    );
    drop(WorkspaceGuard::try_enter(&root).unwrap());
    coordinator
        .acquire(&destination)
        .unwrap()
        .release()
        .unwrap();
    drop(coordinator);
    fs::remove_dir_all(root).unwrap();
}
