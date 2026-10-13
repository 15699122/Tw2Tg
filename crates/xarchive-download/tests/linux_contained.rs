#![cfg(target_os = "linux")]

use std::path::Path;
use std::time::Duration;
use xarchive_download::{
    AddUriRequest, Aria2Supervisor, Aria2SupervisorConfig, DownloadBackend, TransferState,
};
use xarchive_workflow::linux_containment::LinuxContainment;

#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn contained_rpc_ready_stops_and_removes_group() {
    use std::os::unix::fs::PermissionsExt;
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let name = format!("tw2tg-aria-ready-{}", std::process::id());
    let owner = LinuxContainment::create(&parent, &name).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let fixture = std::env::temp_dir().join(format!("tw2tg-rpc-{}.py", std::process::id()));
    let script = format!(
        r#"#!/usr/bin/python3
import http.server, json, pathlib
assert '/{name}' in open('/proc/self/cgroup').read()
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        assert request['params'][0] == 'token:test-secret'
        method = request['method']
        if method == 'aria2.getVersion':
            result = {{'version':'fixture'}}
        elif method == 'aria2.addUri':
            options = request['params'][2]
            self.server.payload = pathlib.Path(options['dir']) / options['out']
            self.server.payload.write_bytes(b'fixture-transfer')
            result = 'fixture-gid'
        elif method == 'aria2.tellStatus':
            result = {{'gid':'fixture-gid','status':'complete','completedLength':'16','totalLength':'16','downloadSpeed':'0','files':[{{'path':str(self.server.payload),'length':'16','completedLength':'16'}}]}}
        else:
            raise AssertionError(method)
        response = json.dumps({{'jsonrpc':'2.0','id':request['id'],'result':result}}).encode()
        self.send_response(200)
        self.send_header('Content-Length', str(len(response)))
        self.end_headers()
        self.wfile.write(response)
    def log_message(self, *args): pass
http.server.HTTPServer(('127.0.0.1', {port}), Handler).serve_forever()
"#
    );
    std::fs::write(&fixture, script).unwrap();
    std::fs::set_permissions(&fixture, std::fs::Permissions::from_mode(0o700)).unwrap();
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let config =
        Aria2SupervisorConfig::new(fixture.to_string_lossy(), "127.0.0.1", port, "test-secret")
            .unwrap()
            .with_startup_timeout(Duration::from_secs(3));
    let mut supervisor =
        Aria2Supervisor::spawn_contained(config, owner, Path::new("/usr/bin/python3"), &launcher)
            .unwrap();
    assert!(supervisor.is_running().unwrap());
    let payload = fixture.with_extension("payload");
    let id = supervisor
        .backend()
        .add_uri(AddUriRequest {
            url: "http://127.0.0.1/fixture".into(),
            directory: payload.parent().unwrap().to_string_lossy().into_owned(),
            filename: payload.file_name().unwrap().to_string_lossy().into_owned(),
            headers: Vec::new(),
        })
        .unwrap();
    let status = supervisor.backend().status(&id).unwrap();
    assert_eq!(status.state, TransferState::Complete);
    assert_eq!(status.completed_bytes, 16);
    assert_eq!(std::fs::read(&payload).unwrap(), b"fixture-transfer");
    supervisor.shutdown().unwrap();
    supervisor.shutdown().unwrap();
    assert!(!parent.join(name).exists());
    std::fs::remove_file(fixture).unwrap();
    std::fs::remove_file(payload).unwrap();
}

#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn contained_invalid_configuration_cleans_unlaunched_group() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let name = format!("tw2tg-aria-invalid-{}", std::process::id());
    let owner = LinuxContainment::create(&parent, &name).unwrap();
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let config = Aria2SupervisorConfig::new("/usr/bin/false", "127.0.0.1", 1, "test-secret")
        .unwrap()
        .with_startup_timeout(Duration::ZERO);
    assert!(
        Aria2Supervisor::spawn_contained(config, owner, Path::new("/usr/bin/python3"), &launcher)
            .is_err()
    );
    assert!(!parent.join(name).exists());
}

#[test]
#[ignore = "requires explicitly delegated disposable cgroup"]
fn contained_startup_exit_cleans_group() {
    let membership = std::fs::read_to_string("/proc/self/cgroup").unwrap();
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap();
    let parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let name = format!("tw2tg-aria-exit-{}", std::process::id());
    let owner = LinuxContainment::create(&parent, &name).unwrap();
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py");
    let config = Aria2SupervisorConfig::new("/usr/bin/false", "127.0.0.1", 1, "test-secret")
        .unwrap()
        .with_startup_timeout(Duration::from_secs(1));
    let started = std::time::Instant::now();
    assert!(
        Aria2Supervisor::spawn_contained(config, owner, Path::new("/usr/bin/python3"), &launcher)
            .is_err()
    );
    assert!(!parent.join(name).exists());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "startup must not inherit the 15s transfer RPC timeout"
    );
}
