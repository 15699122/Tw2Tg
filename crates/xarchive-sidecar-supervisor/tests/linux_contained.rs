#![cfg(target_os = "linux")]

use std::path::Path;
use std::time::Duration;
use xarchive_sidecar_supervisor::ChildProcessEnvironment;
use xarchive_sidecar_supervisor::SidecarSupervisor;
use xarchive_workflow::linux_containment::LinuxContainment;

fn delegated_owner(label: &str) -> (LinuxContainment, std::path::PathBuf, std::path::PathBuf) {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let group = parent.join(format!("tw2tg-{label}-{}", std::process::id()));
    let owner =
        LinuxContainment::create(&parent, group.file_name().unwrap().to_str().unwrap()).unwrap();
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    (owner, group, launcher)
}

#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn contained_valid_handshake_and_graceful_exit() {
    let (owner, group, launcher) = delegated_owner("ready");
    let script = "import json,sys; c=json.loads(sys.stdin.readline()); print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True); c=json.loads(sys.stdin.readline()); assert c['cmd']=='shutdown'";
    let mut supervisor = SidecarSupervisor::spawn_contained(
        owner,
        Path::new("/usr/bin/python3"),
        &launcher,
        Path::new("/usr/bin/python3"),
        &["-u", "-c", script],
    )
    .unwrap()
    .require_ready_v2(Duration::from_secs(5))
    .unwrap();
    assert!(supervisor.is_running().unwrap());
    supervisor.send_v2_shutdown("stop").unwrap();
    assert!(supervisor.wait_for_exit(Duration::from_secs(5)).unwrap());
    assert!(!group.exists());
}

#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn contained_timeout_kills_setsid_descendant() {
    let (owner, group, launcher) = delegated_owner("timeout");
    let marker = std::env::temp_dir().join(format!("tw2tg-descendant-{}", std::process::id()));
    let ready = marker.with_extension("ready");
    let descendant = "import pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text('ready'); time.sleep(1); pathlib.Path(sys.argv[2]).write_text('late'); time.sleep(20)";
    let script = "import subprocess,sys,pathlib,time; subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2],sys.argv[3]],start_new_session=True); deadline=time.monotonic()+5\nwhile not pathlib.Path(sys.argv[2]).exists():\n if time.monotonic()>deadline: raise RuntimeError('not ready')\n time.sleep(.01)\nprint('{\"protocol_version\":2,\"event\":\"log\",\"job_id\":\"system\",\"message\":\"descendant ready\"}',flush=True); sys.stdin.read()";
    let supervisor = SidecarSupervisor::spawn_contained(
        owner,
        Path::new("/usr/bin/python3"),
        &launcher,
        Path::new("/usr/bin/python3"),
        &[
            "-u",
            "-c",
            script,
            descendant,
            ready.to_str().unwrap(),
            marker.to_str().unwrap(),
        ],
    )
    .unwrap();
    // The fixture emits a protocol line only after its escaped process-group
    // descendant is running. cgroup membership survives setsid.
    assert!(
        supervisor
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    assert!(ready.exists());
    assert!(matches!(
        supervisor.require_ready_v2(Duration::from_millis(50)),
        Err(xarchive_sidecar_supervisor::SupervisorError::HandshakeTimeout)
    ));
    assert!(!group.exists());
    std::thread::sleep(Duration::from_millis(1200));
    assert!(!marker.exists());
    std::fs::remove_file(ready).unwrap();
}

#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn contained_supervisor_reaps_and_removes_group() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let name = format!("tw2tg-sidecar-{}", std::process::id());
    let owner = LinuxContainment::create(&parent, &name).unwrap();
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let mut supervisor = SidecarSupervisor::spawn_contained(
        owner,
        Path::new("/usr/bin/python3"),
        &launcher,
        Path::new("/usr/bin/python3"),
        &["-u", "-c", "import sys; sys.stdin.read()"],
    )
    .unwrap();
    supervisor.shutdown().unwrap();
    assert!(supervisor.readers_finished());
    assert!(!parent.join(name).exists());
    supervisor.shutdown().unwrap();
    assert!(supervisor.wait_for_exit(Duration::from_secs(1)).unwrap());
}

#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn contained_handshake_rejection_cleans_group_and_applies_environment() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let name = format!("tw2tg-sidecar-reject-{}", std::process::id());
    let owner = LinuxContainment::create(&parent, &name).unwrap();
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let script = "import os,json,sys; assert os.environ['TW2TG_SET']=='value with spaces'; assert 'TW2TG_REMOVE' not in os.environ; c=json.loads(sys.stdin.readline()); print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media']}),flush=True); sys.stdin.read()";
    let environment = ChildProcessEnvironment::new(
        vec![
            ("TW2TG_SET".into(), "value with spaces".into()),
            ("TW2TG_REMOVE".into(), "must disappear".into()),
        ],
        vec!["TW2TG_REMOVE".into()],
    );
    let supervisor = SidecarSupervisor::spawn_contained_with_environment(
        owner,
        Path::new("/usr/bin/python3"),
        &launcher,
        Path::new("/usr/bin/python3"),
        &["-u", "-c", script],
        &environment,
    )
    .unwrap();
    assert!(
        matches!(supervisor.require_ready_v2(Duration::from_secs(5)), Err(xarchive_sidecar_supervisor::SupervisorError::HandshakeFailed(message)) if message.contains("capabilities"))
    );
    assert!(!parent.join(name).exists());
}
