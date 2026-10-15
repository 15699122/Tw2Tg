#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;
use xarchive_workflow::linux_containment::LinuxContainment;

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
