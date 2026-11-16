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
    fs::create_dir(root.join("archive")).unwrap();
    assert!(matches!(two.acquire(&alias), Err(LockError::Contended)));
    drop((held, one, two));
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires an existing local NTFS directory with a distinct 8.3 alias"]
fn destination_long_and_short_name_contend() {
    let long = PathBuf::from(std::env::var_os("TW2TG_ALIAS_LONG").expect("long path required"));
    let short = PathBuf::from(std::env::var_os("TW2TG_ALIAS_SHORT").expect("short path required"));
    assert_ne!(long, short);
    assert_eq!(
        fs::canonicalize(&long).unwrap(),
        fs::canonicalize(&short).unwrap()
    );
    let root = fixture("short-name-lock");
    let one = WindowsCoordinator::new(&root).unwrap();
    let two = WindowsCoordinator::new(&root).unwrap();
    let held = one.acquire(&LockScope::Destination(long)).unwrap();
    assert!(matches!(
        two.acquire(&LockScope::Destination(short)),
        Err(LockError::Contended)
    ));
    drop((held, one, two));
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "known unresolved alias-creation race; requires a disposable 8.3 fixture"]
fn destination_short_name_before_creation_contends_after_creation() {
    let long = PathBuf::from(std::env::var_os("TW2TG_ALIAS_LONG").expect("long path required"));
    let short = PathBuf::from(std::env::var_os("TW2TG_ALIAS_SHORT").expect("short path required"));
    let parent = long.parent().expect("fixture parent required");
    assert_eq!(long.file_name().unwrap(), "Long Destination Alias Fixture");
    assert_eq!(
        fs::read(parent.join("w2-fixture.marker")).unwrap(),
        b"w2-alias-fixture"
    );
    assert_eq!(
        fs::canonicalize(parent).unwrap(),
        fs::canonicalize(short.parent().unwrap()).unwrap()
    );
    assert!(!long.exists() && !short.exists());
    let root = fixture("future-short-name-lock");
    let one = WindowsCoordinator::new(&root).unwrap();
    let two = WindowsCoordinator::new(&root).unwrap();
    let held = one.acquire(&LockScope::Destination(short.clone())).unwrap();
    fs::create_dir(&long).unwrap();
    assert_eq!(
        fs::canonicalize(&long).unwrap(),
        fs::canonicalize(&short).unwrap()
    );
    assert!(matches!(
        two.acquire(&LockScope::Destination(long)),
        Err(LockError::Contended)
    ));
    drop((held, one, two));
    fs::remove_dir_all(root).unwrap();
}

/// Diagnostic only: two initially distinct names can become aliases while
/// both guards are held. This must not be mistaken for lock acceptance.
#[test]
#[ignore = "diagnostic requires 8.3 name generation on a disposable volume"]
fn destination_two_precreation_guards_can_become_aliases() {
    let root = fixture("future-two-aliases");
    let locks = root.join("locks");
    let long = root.join("Long Destination Alias Fixture");
    let short = root.join("LONGDE~1");
    assert!(!long.exists() && !short.exists());
    let one = WindowsCoordinator::new(&locks).unwrap();
    let two = WindowsCoordinator::new(&locks).unwrap();
    let first = one.acquire(&LockScope::Destination(short.clone())).unwrap();
    let second = two.acquire(&LockScope::Destination(long.clone())).unwrap();
    fs::create_dir(&long).unwrap();
    assert!(
        short.exists(),
        "fixture volume did not create expected 8.3 alias"
    );
    assert_eq!(
        fs::canonicalize(&long).unwrap(),
        fs::canonicalize(&short).unwrap()
    );
    println!(
        "two destination guards remained live after names became aliases: {}",
        root.display()
    );
    drop((first, second, one, two));
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
fn no_replace_classifies_existing_file_and_directory_destinations() {
    let root = fixture("destination-conflicts");
    for (source_is_dir, destination_is_dir) in
        [(false, false), (false, true), (true, false), (true, true)]
    {
        let source = root.join("source");
        let destination = root.join("destination");
        if source_is_dir {
            fs::create_dir(&source).unwrap();
            fs::write(source.join("source.bin"), b"source").unwrap();
        } else {
            fs::write(&source, b"source").unwrap();
        }
        if destination_is_dir {
            fs::create_dir(&destination).unwrap();
            fs::write(destination.join("existing.bin"), b"existing").unwrap();
        } else {
            fs::write(&destination, b"existing").unwrap();
        }
        assert_eq!(
            WindowsNoReplaceMover.move_no_replace(&source, &destination),
            Err(MoveError::DestinationExists),
            "source_is_dir={source_is_dir} destination_is_dir={destination_is_dir}"
        );
        assert_eq!(
            fs::read(if destination_is_dir {
                destination.join("existing.bin")
            } else {
                destination.clone()
            })
            .unwrap(),
            b"existing"
        );
        if source_is_dir {
            fs::remove_dir_all(source).unwrap();
        } else {
            fs::remove_file(source).unwrap();
        }
        if destination_is_dir {
            fs::remove_dir_all(destination).unwrap();
        } else {
            fs::remove_file(destination).unwrap();
        }
    }
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
fn active_workspace_holder() {
    let Some(path) = std::env::var_os("TW2TG_ACTIVE_WORKSPACE_HOLDER") else {
        return;
    };
    let root = PathBuf::from(path);
    let _guard = WorkspaceGuard::try_enter(&root).unwrap();
    let ready = if std::env::var_os("TW2TG_ACTIVE_HOLDER_TIMEOUT").is_some() {
        "active-holder-timeout.ready"
    } else if std::env::var_os("TW2TG_ACTIVE_HOLDER_SECOND").is_some() {
        "active-holder-second.ready"
    } else {
        "active-holder.ready"
    };
    fs::write(root.join(ready), b"ready").unwrap();
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

#[test]
fn competing_publish_worker() {
    let Some(root) = std::env::var_os("TW2TG_PUBLISH_RACE_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let id = std::env::var("TW2TG_PUBLISH_RACE_ID").unwrap();
    let staging = root.join(format!("staging-{id}"));
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join("identity"), id.as_bytes()).unwrap();
    fs::write(root.join(format!("ready-{id}")), b"ready").unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !root.join("go").exists() {
        assert!(Instant::now() < deadline, "race start timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    let result = WindowsNoReplaceMover.move_no_replace(&staging, &root.join("archive"));
    assert!(
        matches!(result, Ok(()) | Err(MoveError::DestinationExists)),
        "unexpected publication failure: {result:?}"
    );
    fs::write(
        root.join(format!("result-{id}")),
        if result.is_ok() { "win" } else { "lost" },
    )
    .unwrap();
}

#[test]
fn independent_processes_cannot_replace_the_same_published_directory() {
    let root = fixture("publish-race");
    let mut children: Vec<_> = ["a", "b"]
        .iter()
        .map(|id| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "competing_publish_worker", "--nocapture"])
                .env("TW2TG_PUBLISH_RACE_ROOT", &root)
                .env("TW2TG_PUBLISH_RACE_ID", id)
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(root.join("ready-a").exists() && root.join("ready-b").exists())
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    let ready = root.join("ready-a").exists() && root.join("ready-b").exists();
    fs::write(root.join("go"), b"go").unwrap();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    assert!(ready, "both independent publishers must reach the race");
    let a = fs::read(root.join("result-a")).unwrap();
    let b = fs::read(root.join("result-b")).unwrap();
    assert!((a == b"win" && b == b"lost") || (a == b"lost" && b == b"win"));
    let winner = if a == b"win" { "a" } else { "b" };
    let loser = if winner == "a" { "b" } else { "a" };
    assert_eq!(
        fs::read(root.join("archive").join("identity")).unwrap(),
        winner.as_bytes()
    );
    assert_eq!(
        fs::read(root.join(format!("staging-{loser}")).join("identity")).unwrap(),
        loser.as_bytes()
    );
    fs::remove_dir_all(root).unwrap();
}
