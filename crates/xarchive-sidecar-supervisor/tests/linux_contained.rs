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
fn contained_supervisor_drop_drains_and_removes_writer_domain() {
    // V1-06: Drop is not evidence of termination unless it actually drains. A
    // contained supervisor whose owner is dropped must leave no live writer and
    // no surviving domain, otherwise an untracked writer could continue after
    // the attempt's owners are released.
    let (owner, group, launcher) = delegated_owner("v1-drop");

    let supervisor = {
        let mut supervisor = SidecarSupervisor::spawn_contained(
            owner,
            Path::new("/usr/bin/python3"),
            &launcher,
            Path::new("/usr/bin/python3"),
            &["-u", "-c", "import sys; sys.stdin.read()"],
        )
        .unwrap();
        assert!(supervisor.is_running().unwrap());
        assert!(group.exists(), "the writer domain starts populated");
        supervisor
    };
    // Dropping the last owner must shut the writer down, not leak it.
    drop(supervisor);

    assert!(
        !group.exists(),
        "Drop must remove the writer domain instead of leaving an untracked writer"
    );
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
fn contained_internal_v1_cancellation_drains_writer_domain() {
    // V1-08: Sidecar cancellation inside a real registered writer domain. The
    // existing cancellation test runs a bare process, so it cannot show that
    // cancelling also leaves the containment domain drained and removable.

    let (owner, group, launcher) = delegated_owner("v1-cancel");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../desktop/src-tauri/tests/fixtures/internal_v1_worker.py");
    assert!(fixture.is_file(), "internal V1 worker fixture is required");
    let mut supervisor = SidecarSupervisor::spawn_contained(
        owner,
        Path::new("/usr/bin/python3"),
        &launcher,
        Path::new("/usr/bin/python3"),
        &["-u", fixture.to_str().unwrap()],
    )
    .unwrap()
    .require_ready_v2(Duration::from_secs(5))
    .unwrap();
    assert!(supervisor.is_running().unwrap());

    let staging =
        std::env::temp_dir().join(format!("tw2tg-v1-cancel-staging-{}", std::process::id()));
    std::fs::create_dir_all(&staging).unwrap();
    supervisor
        .send_v2_download(
            "req-1",
            "job-cancel",
            "https://x.com/a/status/123",
            None,
            None,
            staging.to_string_lossy(),
            Vec::new(),
        )
        .expect("download starts inside the registered domain");
    // Give the fixture a moment to enter its extraction loop before cancelling;
    // the assertion below does not depend on parsing event text.
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        supervisor.is_running().unwrap(),
        "the worker must still be running inside the domain"
    );

    supervisor
        .send_v2_cancel("job-cancel", "req-cancel")
        .expect("cancel is delivered to the contained worker");

    // A completed drain proves the cancellation did not leave an escaped writer.
    supervisor
        .shutdown()
        .expect("cancelled worker must drain cleanly");
    assert!(
        supervisor.readers_finished(),
        "owned pipe readers must join after cancellation"
    );
    assert!(
        !group.exists(),
        "cancellation must leave no registered writer domain behind"
    );
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
