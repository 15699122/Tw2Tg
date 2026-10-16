#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use xarchive_workflow::linux_containment::{AttemptIdentity, LinuxContainment, RecoveryOutcome};

#[test]
fn invalid_roots_are_rejected_without_creation() {
    assert!(LinuxContainment::create(Path::new("/tmp"), "probe").is_err());
    assert!(LinuxContainment::create(Path::new("/sys/fs/cgroup"), "../probe").is_err());
}
/// Run explicitly inside systemd-run --user -p Delegate=yes.
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn real_owner_launches_and_drains() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let name = format!("tw2tg-rust-owner-{}", std::process::id());
    let mut owner = LinuxContainment::create(&parent, &name).unwrap();
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    assert!(owner.remove_group().is_err());
    let (stdin, stdout, stderr) = owner
        .spawn_piped(
            Path::new("/usr/bin/python3"),
            &launcher,
            Path::new("/usr/bin/python3"),
            &["-u", "-c", "import pathlib,sys; print(pathlib.Path('/proc/self/cgroup').read_text().strip(), flush=True); sys.stdin.read()"],
        )
        .unwrap();
    let mut membership_line = String::new();
    BufReader::new(stdout)
        .read_line(&mut membership_line)
        .unwrap();
    assert_eq!(membership_line.trim(), format!("0::{relative}/{name}"));
    drop(stdin);
    drop(stderr);
    owner.stop_and_drain(Duration::from_secs(5)).unwrap();
    assert!(
        owner
            .spawn(
                Path::new("/usr/bin/python3"),
                &launcher,
                Path::new("/usr/bin/python3")
            )
            .is_err()
    );
    owner.remove_group().unwrap();
    owner.remove_group().unwrap();
    assert!(!parent.join(name).exists());
}

/// Unique scratch directory for record-root fixtures.
fn scratch_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tw2tg-r4a-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// A cgroup parent that passes path validation but does not exist, so group
/// creation fails after the record step without touching the real hierarchy.
fn absent_delegate() -> PathBuf {
    Path::new("/sys/fs/cgroup").join(format!("tw2tg-r4a-absent-{}", std::process::id()))
}

#[test]
fn attempt_identity_names_are_deterministic_and_distinct() {
    let identity = AttemptIdentity::new("/ws/r4a", "job", 1).unwrap();
    let same = AttemptIdentity::new("/ws/r4a", "job", 1).unwrap();
    assert_eq!(identity.group_name(), same.group_name());
    let name = identity.group_name();
    assert!(name.starts_with("tw2tg-att-"));
    assert!(!name.contains(['/', '\0']));
    assert_ne!(name, ".");
    assert_ne!(name, "..");
    assert_ne!(
        name,
        AttemptIdentity::new("/ws/r4a", "job", 2)
            .unwrap()
            .group_name()
    );
    assert_ne!(
        name,
        AttemptIdentity::new("/ws/r4a", "job2", 1)
            .unwrap()
            .group_name()
    );
    assert_ne!(
        name,
        AttemptIdentity::new("/ws/r4b", "job", 1)
            .unwrap()
            .group_name()
    );
    assert!(AttemptIdentity::new("relative/ws", "job", 1).is_err());
    assert!(AttemptIdentity::new("/ws/r4a", "", 1).is_err());
    assert!(AttemptIdentity::new("/ws\nr4a", "job", 1).is_err());
    assert!(AttemptIdentity::new("/ws\0r4a", "job", 1).is_err());
}

#[test]
fn recorded_creation_persists_the_record_before_group_creation() {
    let record_root = scratch_root("record");
    let identity = AttemptIdentity::new("/ws/r4a", "job-record", 7).unwrap();
    let parent = absent_delegate();
    // The group step fails (parent chain absent), but the record must already
    // be durable: record-before-mkdir ordering.
    assert!(LinuxContainment::create_recorded(&record_root, &identity, &parent).is_err());
    let record = std::fs::read_to_string(identity.record_path(&record_root)).unwrap();
    assert!(record.starts_with("tw2tg-containment-record-v1\n"));
    assert!(record.contains("workspace=/ws/r4a\n"));
    assert!(record.contains("job=job-record\n"));
    assert!(record.contains("attempt=7\n"));
    assert!(record.contains(&format!("parent={}\n", parent.display())));
    assert!(record.contains(&format!("name={}\n", identity.group_name())));
    std::fs::remove_dir_all(&record_root).unwrap();
}

#[test]
fn recovery_without_a_record_reports_no_record() {
    let record_root = scratch_root("norecord");
    let identity = AttemptIdentity::new("/ws/r4a", "job-none", 1).unwrap();
    let outcome =
        LinuxContainment::recover_attempt(&record_root, &identity, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome, RecoveryOutcome::NoRecord);
    // A record root that never existed is also NoRecord, not an error.
    let missing = record_root.join("never-created");
    let outcome =
        LinuxContainment::recover_attempt(&missing, &identity, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome, RecoveryOutcome::NoRecord);
    std::fs::remove_dir_all(&record_root).unwrap();
}

#[test]
fn recovery_fails_closed_on_tampered_records() {
    let record_root = scratch_root("tamper");
    let identity = AttemptIdentity::new("/ws/r4a", "job-tamper", 3).unwrap();
    let parent = absent_delegate();
    assert!(LinuxContainment::create_recorded(&record_root, &identity, &parent).is_err());
    let path = identity.record_path(&record_root);
    let original = std::fs::read_to_string(&path).unwrap();

    // Tampering with the derived name binding must not be trusted.
    let tampered = original.replace(
        &format!("name={}", identity.group_name()),
        "name=tw2tg-att-0000000000000000",
    );
    assert_ne!(tampered, original);
    std::fs::write(&path, &tampered).unwrap();
    let error = LinuxContainment::recover_attempt(&record_root, &identity, Duration::from_secs(1))
        .unwrap_err();
    assert_ne!(error.kind(), std::io::ErrorKind::NotFound);

    // Tampering with the persisted parent must not escape path validation.
    let tampered = original.replace(&format!("parent={}", parent.display()), "parent=/tmp");
    assert_ne!(tampered, original);
    std::fs::write(&path, &tampered).unwrap();
    let error = LinuxContainment::recover_attempt(&record_root, &identity, Duration::from_secs(1))
        .unwrap_err();
    assert_ne!(error.kind(), std::io::ErrorKind::NotFound);

    // Garbage content is rejected instead of treated as absence.
    std::fs::write(&path, "not a containment record").unwrap();
    let error = LinuxContainment::recover_attempt(&record_root, &identity, Duration::from_secs(1))
        .unwrap_err();
    assert_ne!(error.kind(), std::io::ErrorKind::NotFound);
    std::fs::remove_dir_all(&record_root).unwrap();
}

#[test]
fn recovery_of_record_without_group_reports_already_absent() {
    let record_root = scratch_root("absent");
    let identity = AttemptIdentity::new("/ws/r4a", "job-absent", 2).unwrap();
    let parent = absent_delegate();
    // Record step succeeds, group step fails: the crash-equivalent state of a
    // parent that died between record and mkdir.
    assert!(LinuxContainment::create_recorded(&record_root, &identity, &parent).is_err());
    for _ in 0..2 {
        let outcome =
            LinuxContainment::recover_attempt(&record_root, &identity, Duration::from_secs(1))
                .unwrap();
        assert_eq!(outcome, RecoveryOutcome::AlreadyAbsent);
    }
    // The record is retained so the invariant stays provable across retries.
    assert!(identity.record_path(&record_root).exists());
    std::fs::remove_dir_all(&record_root).unwrap();
}

/// Run explicitly inside systemd-run --user -p Delegate=yes.
///
/// Simulates a SIGKILLed owner by dropping the recorded owner without drain
/// while a descendant is scheduled to write a file late, then proves the
/// successor attaches with admission closed, kills and drains the domain,
/// observes no late write, and sees `AlreadyAbsent` on repeat recovery.
#[test]
#[ignore = "requires an explicitly delegated disposable cgroup"]
fn successor_recovers_crashed_owner_domain() {
    let record_root = scratch_root("orphan");
    let identity = AttemptIdentity::new(
        format!("/ws/r4a-orphan-{}", std::process::id()),
        "job-orphan",
        1,
    )
    .unwrap();
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let late_file = record_root.join("late-write.txt");
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let late = late_file.to_str().unwrap().to_owned();
    let delay = "2".to_owned();

    // Crashed owner: recorded creation, then a descendant scheduled to write
    // late; the owner handle is dropped without any kill or drain.
    let mut crashed = LinuxContainment::create_recorded(&record_root, &identity, &parent).unwrap();
    let (stdin, stdout, stderr) = crashed
        .spawn_piped(
            Path::new("/usr/bin/python3"),
            &launcher,
            Path::new("/usr/bin/python3"),
            &[
                "-u",
                "-c",
                "import sys,time; print('ready',flush=True); time.sleep(float(sys.argv[1])); open(sys.argv[2],'w').write('late')",
                delay.as_str(),
                late.as_str(),
            ],
        )
        .unwrap();
    let mut ready = String::new();
    BufReader::new(stdout).read_line(&mut ready).unwrap();
    assert_eq!(ready.trim(), "ready");
    drop(stdin);
    drop(crashed); // no kill, no drain: the descendant keeps running in the group

    // A successor attach permanently refuses new launches.
    let mut attached = LinuxContainment::attach(&parent, &identity.group_name()).unwrap();
    assert!(
        attached
            .spawn(
                Path::new("/usr/bin/python3"),
                &launcher,
                Path::new("/usr/bin/python3")
            )
            .is_err()
    );
    drop(attached);
    drop(stderr);

    let outcome =
        LinuxContainment::recover_attempt(&record_root, &identity, Duration::from_secs(5)).unwrap();
    assert_eq!(outcome, RecoveryOutcome::Drained);
    assert!(!parent.join(identity.group_name()).exists());

    // Outlive the descendant's write deadline: a surviving writer would have
    // produced the file by now.
    std::thread::sleep(Duration::from_millis(2600));
    assert!(!late_file.exists());

    // Repeat recovery stays idempotent and the record is retained.
    let outcome =
        LinuxContainment::recover_attempt(&record_root, &identity, Duration::from_secs(5)).unwrap();
    assert_eq!(outcome, RecoveryOutcome::AlreadyAbsent);
    assert!(identity.record_path(&record_root).exists());
    std::fs::remove_dir_all(&record_root).unwrap();
}
