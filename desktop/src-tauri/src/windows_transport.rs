//! Windows Named Pipe transport and per-user Native Messaging registration.
//! This module is deliberately Windows-only; the browser protocol stays shared.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use interprocess::os::windows::named_pipe::PipeStream;
use interprocess::os::windows::{
    named_pipe::{PipeListenerOptions, pipe_mode},
    security_descriptor::SecurityDescriptor,
};
use serde_json::Value;
use widestring::U16CString;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

use crate::executor::{ArchiveApplicationService, StorageJobPersistence};
use crate::transport::BrowserTransportAdapter;

const EDGE_HOSTS_KEY: &str = r"Software\Microsoft\Edge\NativeMessagingHosts";
const CHROME_HOSTS_KEY: &str = r"Software\Google\Chrome\NativeMessagingHosts";
const HOST_NAME: &str = "com.tw2tg.xarchive";
const HOST_MANIFEST_FILE: &str = "com.tw2tg.xarchive.json";
const HOST_EXECUTABLE_FILE: &str = "xarchive-native-host.exe";

#[derive(Default)]
pub(crate) struct TransportSessionState {
    active: AtomicUsize,
    connected_once: AtomicBool,
    last_request: Mutex<Option<Instant>>,
}

impl TransportSessionState {
    pub(crate) fn browser_connection(&self) -> &'static str {
        if self.active.load(Ordering::Relaxed) > 0
            || self
                .last_request
                .lock()
                .ok()
                .and_then(|last| *last)
                .is_some_and(|last| last.elapsed() < Duration::from_secs(30))
        {
            "connected"
        } else if self.connected_once.load(Ordering::Relaxed) {
            "disconnected"
        } else {
            "not_loaded"
        }
    }
}

pub(crate) fn transport_endpoint() -> PathBuf {
    std::env::var_os(xarchive_protocol::PIPE_ENDPOINT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(xarchive_protocol::WINDOWS_PIPE_ENDPOINT))
}

pub(crate) struct DesktopTransportServer {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub(crate) session: Arc<TransportSessionState>,
}

impl DesktopTransportServer {
    pub(crate) fn start(
        service: ArchiveApplicationService,
        database_path: PathBuf,
        endpoint: PathBuf,
    ) -> Result<Self, String> {
        let sddl = U16CString::from_str("D:P(A;;GA;;;OW)")
            .map_err(|error| format!("invalid current-user Named Pipe ACL: {error}"))?;
        let security = SecurityDescriptor::deserialize(&sddl)
            .map_err(|error| format!("failed to create current-user Named Pipe ACL: {error}"))?;
        let listener = PipeListenerOptions::new()
            .path(endpoint.as_path())
            .security_descriptor(Some(security))
            .create_duplex::<pipe_mode::Bytes>()
            .map_err(|error| {
                format!(
                    "failed to listen on Named Pipe {}: {error}",
                    endpoint.display()
                )
            })?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("failed to configure Named Pipe listener: {error}"))?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let session = Arc::new(TransportSessionState::default());
        let session_for_thread = session.clone();
        let thread = std::thread::Builder::new()
            .name("xarchive-desktop-named-pipe".to_owned())
            .spawn(move || {
                while !stop_for_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok(stream) => {
                            let service = service.clone();
                            let database_path = database_path.clone();
                            let session = session_for_thread.clone();
                            let _ = std::thread::Builder::new()
                                .name("xarchive-named-pipe-request".to_owned())
                                .spawn(move || {
                                    session.connected_once.store(true, Ordering::Relaxed);
                                    session.active.fetch_add(1, Ordering::Relaxed);
                                    if let Ok(mut last) = session.last_request.lock() {
                                        *last = Some(Instant::now());
                                    }
                                    handle_pipe_connection(&service, &database_path, stream);
                                    session.active.fetch_sub(1, Ordering::Relaxed);
                                });
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(25));
                        }
                        Err(_) if stop_for_thread.load(Ordering::Relaxed) => break,
                        Err(_) => std::thread::sleep(Duration::from_millis(100)),
                    }
                }
            })
            .map_err(|error| format!("failed to start Named Pipe listener: {error}"))?;
        Ok(Self {
            stop,
            thread: Some(thread),
            session,
        })
    }
}

impl Drop for DesktopTransportServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle_pipe_connection(
    service: &ArchiveApplicationService,
    database_path: &Path,
    mut stream: PipeStream<pipe_mode::Bytes, pipe_mode::Bytes>,
) {
    use xarchive_native_host::{read_json, write_json};

    let response = match read_json::<_, BrowserRequest>(&mut stream) {
        Ok(Some(request)) => match StorageJobPersistence::open(database_path) {
            Ok(mut persistence) => BrowserTransportAdapter::with_database_path(
                service.clone(),
                database_path.to_owned(),
            )
            .handle_request(&mut persistence, request),
            Err(error) => BrowserResponse::Error {
                protocol_version: PROTOCOL_VERSION,
                request_id: None,
                error_code: "PERSISTENCE_ERROR".to_owned(),
                error_message: error,
                retryable: true,
            },
        },
        Ok(None) => return,
        Err(error) => BrowserResponse::Error {
            protocol_version: PROTOCOL_VERSION,
            request_id: None,
            error_code: "INVALID_MESSAGE".to_owned(),
            error_message: error.to_string(),
            retryable: false,
        },
    };
    let _ = write_json(&mut stream, &response);
}

fn registry_keys() -> [(&'static str, &'static str); 2] {
    [("Edge", EDGE_HOSTS_KEY), ("Chrome", CHROME_HOSTS_KEY)]
}

fn registration_manifest_path() -> Result<PathBuf, String> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "LOCALAPPDATA is unavailable for the current user".to_owned())?;
    Ok(PathBuf::from(local_app_data)
        .join("XArchive")
        .join("native-host")
        .join(HOST_MANIFEST_FILE))
}

fn read_host_manifest(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "failed to read Native Host manifest {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid Native Host manifest: {error}"))
}

fn is_managed_manifest(path: &str, expected: &Path) -> bool {
    Path::new(path)
        .to_string_lossy()
        .eq_ignore_ascii_case(&expected.to_string_lossy())
}

pub(crate) fn native_host_registered() -> Result<bool, String> {
    let expected = registration_manifest_path()?;
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    for (_, base) in registry_keys() {
        let key_path = format!(r"{base}\{HOST_NAME}");
        if let Ok(key) = current_user.open_subkey_with_flags(key_path, KEY_READ) {
            if let Ok(value) = key.get_value::<String, _>("")
                && is_managed_manifest(&value, &expected)
                && expected.is_file()
                && registered_manifest_is_usable(&expected)
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn registered_manifest_is_usable(path: &Path) -> bool {
    let Ok(manifest) = read_host_manifest(path) else {
        return false;
    };
    manifest.get("name").and_then(Value::as_str) == Some(HOST_NAME)
        && manifest.get("type").and_then(Value::as_str) == Some("stdio")
        && manifest
            .get("allowed_origins")
            .and_then(Value::as_array)
            .is_some_and(|origins| !origins.is_empty())
        && manifest
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|host| Path::new(host).is_file())
}

pub(crate) fn register_or_repair(portable_root: &Path) -> Result<(), String> {
    let source_manifest = portable_root.join("native-host").join(HOST_MANIFEST_FILE);
    let host_executable = portable_root.join("native-host").join(HOST_EXECUTABLE_FILE);
    if !host_executable.is_file() {
        return Err(format!(
            "Full package Native Host executable is missing: {}",
            host_executable.display()
        ));
    }
    let mut manifest = read_host_manifest(&source_manifest)?;
    if manifest.get("name").and_then(Value::as_str) != Some(HOST_NAME)
        || manifest.get("type").and_then(Value::as_str) != Some("stdio")
        || manifest
            .get("allowed_origins")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    {
        return Err(
            "bundled Native Host manifest does not match XArchive's registration contract"
                .to_owned(),
        );
    }
    manifest["path"] = Value::String(host_executable.display().to_string());

    let target = registration_manifest_path()?;
    let managed_root = target
        .parent()
        .ok_or_else(|| "invalid Native Host manifest path".to_owned())?;
    std::fs::create_dir_all(managed_root)
        .map_err(|error| format!("failed to create per-user Native Host directory: {error}"))?;
    if target.exists() {
        let current = read_host_manifest(&target)?;
        if current.get("name").and_then(Value::as_str) != Some(HOST_NAME)
            || current.get("allowed_origins") != manifest.get("allowed_origins")
        {
            return Err(format!(
                "refusing to replace an unknown manifest at {}",
                target.display()
            ));
        }
    }

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let mut keys = Vec::new();
    for (browser, base) in registry_keys() {
        let key_path = format!(r"{base}\{HOST_NAME}");
        if let Ok(key) = current_user.open_subkey_with_flags(&key_path, KEY_READ) {
            match key.get_value::<String, _>("") {
                Ok(existing) => {
                    if !is_managed_manifest(&existing, &target) {
                        return Err(format!(
                            "refusing to overwrite an unknown {browser} Native Host registration: {existing}"
                        ));
                    }
                }
                Err(_)
                    if key.enum_values().next().is_some() || key.enum_keys().next().is_some() =>
                {
                    return Err(format!(
                        "refusing to modify an unrecognized {browser} Native Host registry key"
                    ));
                }
                Err(_) => {}
            }
        }
        keys.push((browser, key_path));
    }

    let json = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    let temporary = target.with_extension("json.tmp");
    std::fs::write(&temporary, json)
        .map_err(|error| format!("failed to write Native Host manifest: {error}"))?;
    std::fs::rename(&temporary, &target)
        .or_else(|_| {
            std::fs::copy(&temporary, &target)
                .map(|_| ())
                .and_then(|()| std::fs::remove_file(&temporary))
        })
        .map_err(|error| format!("failed to activate Native Host manifest: {error}"))?;

    for (browser, key_path) in keys {
        let (key, _) = current_user
            .create_subkey(&key_path)
            .map_err(|error| format!("failed to register Native Host for {browser}: {error}"))?;
        key.set_value("", &target.display().to_string())
            .map_err(|error| format!("failed to register Native Host for {browser}: {error}"))?;
    }
    Ok(())
}

pub(crate) fn unregister() -> Result<(), String> {
    let expected = registration_manifest_path()?;
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let mut registered_keys = Vec::new();
    for (browser, base) in registry_keys() {
        let key_path = format!(r"{base}\{HOST_NAME}");
        let key = match current_user.open_subkey_with_flags(&key_path, KEY_READ) {
            Ok(key) => key,
            Err(_) => continue,
        };
        let current = match key.get_value::<String, _>("") {
            Ok(value) => value,
            Err(_) => continue,
        };
        if !is_managed_manifest(&current, &expected) {
            return Err(format!(
                "refusing to remove an unknown {browser} Native Host registration: {current}"
            ));
        }
        registered_keys.push((browser, key_path));
    }
    for (browser, key_path) in registered_keys {
        let key = current_user
            .open_subkey_with_flags(key_path, KEY_READ | KEY_WRITE)
            .map_err(|error| {
                format!("failed to open {browser} Native Host registration: {error}")
            })?;
        key.delete_value("")
            .map_err(|error| format!("failed to unregister Native Host from {browser}: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DesktopTransportServer, is_managed_manifest};
    use std::{
        fs::OpenOptions,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };
    use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

    #[test]
    fn accepts_only_the_xarchive_managed_manifest_path() {
        assert!(is_managed_manifest(
            r"C:\Users\Ada\AppData\Local\XArchive\native-host\com.tw2tg.xarchive.json",
            Path::new(r"c:\users\ada\appdata\local\xarchive\native-host\com.tw2tg.xarchive.json")
        ));
        assert!(!is_managed_manifest(
            r"C:\Other\host.json",
            Path::new(r"C:\Users\Ada\AppData\Local\XArchive\native-host\com.tw2tg.xarchive.json")
        ));
    }

    #[test]
    fn named_pipe_serves_repeated_native_host_framed_requests() {
        use xarchive_native_host::{read_json, write_json};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "xarchive-pipe-test-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create test root");
        let database_path = root.join("archive.sqlite3");
        let endpoint = PathBuf::from(format!(
            r"\\.\pipe\xarchive-e5-test-{}-{unique}",
            std::process::id()
        ));
        let mut executor = crate::executor::ExecutorRuntime::new(database_path.clone());
        let server =
            DesktopTransportServer::start(executor.service(), database_path, endpoint.clone())
                .expect("start secured pipe server");

        for index in 0..2 {
            let mut client = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&endpoint)
                .expect("connect as current user");
            let request = BrowserRequest::QueryStatus {
                protocol_version: PROTOCOL_VERSION,
                request_id: format!("pipe-request-{index}"),
                tweet_ids: vec!["123".to_owned()],
            };
            write_json(&mut client, &request).expect("write framed request");
            let response: BrowserResponse = read_json(&mut client)
                .expect("read framed response")
                .expect("response is present");
            assert!(
                matches!(response, BrowserResponse::ArchiveStatusBatch { request_id, .. } if request_id == format!("pipe-request-{index}"))
            );
        }
        assert_eq!(server.session.browser_connection(), "connected");
        drop(server);
        executor.shutdown_in_place().expect("stop executor");
        std::fs::remove_dir_all(root).expect("remove test root");
    }
}
