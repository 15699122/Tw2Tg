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
const ALLOWED_ORIGIN: &str = "chrome-extension://iaajefkoanbkleojofoadeakelihbjne/";

fn has_fixed_extension_origin(manifest: &Value) -> bool {
    manifest
        .get("allowed_origins")
        .and_then(Value::as_array)
        .is_some_and(|origins| origins.len() == 1 && origins[0].as_str() == Some(ALLOWED_ORIGIN))
}

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
    pub(crate) fn start_with_pairing(
        service: ArchiveApplicationService,
        database_path: PathBuf,
        endpoint: PathBuf,
        pairing: Option<Arc<crate::browser_pairing::PairingCoordinator>>,
        output_settings: xarchive_storage::BatchOutputSettings,
        use_aria2: bool,
        tool_compatibility: crate::executor::ToolCompatibility,
    ) -> Result<Self, String> {
        let sddl = U16CString::from_str("D:P(A;;GA;;;OW)")
            .map_err(|error| format!("invalid current-user Named Pipe ACL: {error}"))?;
        let security = SecurityDescriptor::deserialize(&sddl)
            .map_err(|error| format!("failed to create current-user Named Pipe ACL: {error}"))?;
        let listener = PipeListenerOptions::new()
            .path(endpoint.as_path())
            .accept_remote(false)
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
        let output_settings = Arc::new(output_settings);
        let tool_compatibility = Arc::new(tool_compatibility);
        let session = Arc::new(TransportSessionState::default());
        let session_for_thread = session.clone();
        let thread = std::thread::Builder::new()
            .name("xarchive-desktop-named-pipe".to_owned())
            .spawn(move || {
                let mut workers: Vec<std::thread::JoinHandle<()>> = Vec::new();
                while !stop_for_thread.load(Ordering::Relaxed) {
                    let mut index = 0;
                    while index < workers.len() {
                        if workers[index].is_finished() {
                            let _ = workers.swap_remove(index).join();
                        } else {
                            index += 1;
                        }
                    }
                    if workers.len() >= 32 {
                        std::thread::sleep(Duration::from_millis(25));
                        continue;
                    }
                    match listener.accept() {
                        Ok(stream) => {
                            let service = service.clone();
                            let database_path = database_path.clone();
                            let session = session_for_thread.clone();
                            let pairing = pairing.clone();
                            let stop = stop_for_thread.clone();
                            let output_settings = output_settings.clone();
                            let tool_compatibility = tool_compatibility.clone();
                            let use_aria2 = use_aria2;
                            if let Ok(worker) = std::thread::Builder::new()
                                .name("xarchive-named-pipe-request".to_owned())
                                .spawn(move || {
                                    session.connected_once.store(true, Ordering::Relaxed);
                                    session.active.fetch_add(1, Ordering::Relaxed);
                                    if let Ok(mut last) = session.last_request.lock() {
                                        *last = Some(Instant::now());
                                    }
                                    handle_pipe_connection(
                                        &service,
                                        &database_path,
                                        stream,
                                        pairing.as_deref(),
                                        stop,
                                        &output_settings,
                                        use_aria2,
                                        &tool_compatibility,
                                    );
                                    session.active.fetch_sub(1, Ordering::Relaxed);
                                })
                            {
                                workers.push(worker);
                            }
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(25));
                        }
                        Err(_) if stop_for_thread.load(Ordering::Relaxed) => break,
                        Err(_) => std::thread::sleep(Duration::from_millis(100)),
                    }
                }
                for worker in workers {
                    let _ = worker.join();
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
    stream: PipeStream<pipe_mode::Bytes, pipe_mode::Bytes>,
    pairing: Option<&crate::browser_pairing::PairingCoordinator>,
    stop: Arc<AtomicBool>,
    output_settings: &xarchive_storage::BatchOutputSettings,
    use_aria2: bool,
    tool_compatibility: &crate::executor::ToolCompatibility,
) {
    use xarchive_native_host::windows_pipe::{BoundedPipe, PIPE_TIMEOUT};
    use xarchive_native_host::{read_json, write_json};

    let Ok(mut stream) = BoundedPipe::new(stream, stop, Instant::now() + PIPE_TIMEOUT) else {
        return;
    };
    let payload = read_json::<_, Value>(&mut stream);
    if let Ok(Some(value)) = &payload
        && value.get("message_type").and_then(|field| field.as_str()) == Some("bootstrap")
    {
        let response =
            match serde_json::from_value::<xarchive_protocol::BrowserPairingRequest>(value.clone())
            {
                Ok(request) => match pairing {
                    Some(pairing) => pairing.bootstrap(request),
                    None => xarchive_protocol::BrowserPairingResponse::Error {
                        protocol_version: 1,
                        request_id: match request {
                            xarchive_protocol::BrowserPairingRequest::Bootstrap {
                                request_id,
                                ..
                            } => Some(request_id),
                        },
                        error_code: "DESKTOP_NOT_READY".into(),
                        error_message: "Desktop listener unavailable".into(),
                        retryable: true,
                    },
                },
                Err(_) => xarchive_protocol::BrowserPairingResponse::Error {
                    protocol_version: 1,
                    request_id: None,
                    error_code: "INVALID_REQUEST".into(),
                    error_message: "invalid bootstrap request".into(),
                    retryable: false,
                },
            };
        if write_json(&mut stream, &response).is_ok() {
            stream.finish_response();
        }
        return;
    }
    let request = payload.and_then(|value| {
        value
            .map(serde_json::from_value::<BrowserRequest>)
            .transpose()
            .map_err(|error| {
                xarchive_native_host::NativeMessagingError::InvalidJson(error.to_string())
            })
    });

    let response = match request {
        Ok(Some(request)) => match StorageJobPersistence::open(database_path) {
            Ok(mut persistence) => BrowserTransportAdapter::with_database_path(
                service.clone(),
                database_path.to_owned(),
            )
            .with_output_settings(output_settings.clone())
            .with_use_aria2(use_aria2)
            .with_tool_compatibility(tool_compatibility.clone())
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
    if write_json(&mut stream, &response).is_ok() {
        stream.finish_response();
    }
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

pub(crate) fn native_host_registered(portable_root: &Path) -> Result<bool, String> {
    let expected = registration_manifest_path()?;
    Ok(native_host_registered_at(
        portable_root,
        &expected,
        &RegKey::predef(HKEY_CURRENT_USER),
        &registry_keys(),
    ))
}

fn native_host_registered_at(
    portable_root: &Path,
    expected: &Path,
    current_user: &RegKey,
    keys: &[(&str, &str)],
) -> bool {
    let current_host = portable_root.join("native-host").join(HOST_EXECUTABLE_FILE);
    for (_, base) in keys {
        let key_path = format!(r"{base}\{HOST_NAME}");
        if let Ok(key) = current_user.open_subkey_with_flags(key_path, KEY_READ)
            && let Ok(value) = key.get_value::<String, _>("")
            && is_managed_manifest(&value, expected)
            && expected.is_file()
            && registered_manifest_is_usable(expected, &current_host)
        {
            return true;
        }
    }
    false
}

fn registered_manifest_is_usable(path: &Path, current_host: &Path) -> bool {
    let Ok(manifest) = read_host_manifest(path) else {
        return false;
    };
    manifest.get("name").and_then(Value::as_str) == Some(HOST_NAME)
        && manifest.get("type").and_then(Value::as_str) == Some("stdio")
        && has_fixed_extension_origin(&manifest)
        && manifest
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|host| {
                let (Ok(registered), Ok(current)) =
                    (std::fs::File::open(host), std::fs::File::open(current_host))
                else {
                    return false;
                };
                registered.metadata().is_ok_and(|m| m.is_file())
                    && current.metadata().is_ok_and(|m| m.is_file())
                    && matches!(
                        (fs_id::FileID::new(&registered), fs_id::FileID::new(&current)),
                        (Ok(registered_id), Ok(current_id)) if registered_id == current_id
                    )
            })
}

fn validate_registration_key(key: &RegKey, browser: &str, target: &Path) -> Result<(), String> {
    match key.get_value::<String, _>("") {
        Ok(existing) if !is_managed_manifest(&existing, target) => Err(format!(
            "refusing to overwrite an unknown {browser} Native Host registration: {existing}"
        )),
        Err(_) if key.enum_values().next().is_some() || key.enum_keys().next().is_some() => Err(
            format!("refusing to modify an unrecognized {browser} Native Host registry key"),
        ),
        _ => Ok(()),
    }
}

pub(crate) fn register_or_repair(portable_root: &Path) -> Result<(), String> {
    register_or_repair_at(
        portable_root,
        registration_manifest_path,
        &RegKey::predef(HKEY_CURRENT_USER),
        &registry_keys(),
    )
}

fn registration_lock(target: &Path) -> Result<std::fs::File, String> {
    let parent = target
        .parent()
        .ok_or_else(|| "invalid Native Host manifest path".to_owned())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create per-user Native Host directory: {error}"))?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(parent.join("registration.lock"))
        .map_err(|error| format!("failed to open Native Host registration lock: {error}"))?;
    lock.try_lock()
        .map_err(|error| format!("Native Host registration is busy or unavailable: {error}"))?;
    Ok(lock)
}

fn register_or_repair_at(
    portable_root: &Path,
    target_path: impl FnOnce() -> Result<PathBuf, String>,
    current_user: &RegKey,
    registry: &[(&str, &str)],
) -> Result<(), String> {
    register_or_repair_at_inner(portable_root, target_path, current_user, registry, || {})
}

fn register_or_repair_at_inner(
    portable_root: &Path,
    target_path: impl FnOnce() -> Result<PathBuf, String>,
    current_user: &RegKey,
    registry: &[(&str, &str)],
    before_activate: impl FnOnce(),
) -> Result<(), String> {
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
        || !has_fixed_extension_origin(&manifest)
    {
        return Err(
            "bundled Native Host manifest does not match XArchive's registration contract"
                .to_owned(),
        );
    }
    manifest["path"] = Value::String(host_executable.display().to_string());

    let target = target_path()?;
    let _registration_lock = registration_lock(&target)?;
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

    let mut keys = Vec::new();
    for (browser, base) in registry {
        let key_path = format!(r"{base}\{HOST_NAME}");
        if let Ok(key) = current_user.open_subkey_with_flags(&key_path, KEY_READ) {
            validate_registration_key(&key, browser, &target)?;
        }
        keys.push((browser, key_path));
    }

    // Acquire every writable key before changing the managed manifest. If a
    // browser key is denied, the existing installation keeps its old host.
    let mut writable_keys = Vec::new();
    for (browser, key_path) in keys {
        let (key, _) = current_user
            .create_subkey(&key_path)
            .map_err(|error| format!("failed to register Native Host for {browser}: {error}"))?;
        validate_registration_key(&key, browser, &target)?;
        writable_keys.push((browser, key));
    }

    let json = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    let temporary = target.with_extension("json.tmp");
    std::fs::write(&temporary, json)
        .map_err(|error| format!("failed to write Native Host manifest: {error}"))?;
    before_activate();
    std::fs::rename(&temporary, &target)
        .map_err(|error| format!("failed to activate Native Host manifest: {error}"))?;

    for (browser, key) in writable_keys {
        key.set_value("", &target.display().to_string())
            .map_err(|error| format!("failed to register Native Host for {browser}: {error}"))?;
    }
    Ok(())
}

pub(crate) fn unregister() -> Result<(), String> {
    let expected = registration_manifest_path()?;
    unregister_at(
        &expected,
        &RegKey::predef(HKEY_CURRENT_USER),
        &registry_keys(),
    )
}

fn unregister_at(
    expected: &Path,
    current_user: &RegKey,
    registry: &[(&str, &str)],
) -> Result<(), String> {
    let _registration_lock = registration_lock(expected)?;
    let mut registered_keys = Vec::new();
    for (browser, base) in registry {
        let key_path = format!(r"{base}\{HOST_NAME}");
        let key = match current_user.open_subkey_with_flags(&key_path, KEY_READ) {
            Ok(key) => key,
            Err(_) => continue,
        };
        let current = match key.get_value::<String, _>("") {
            Ok(value) => value,
            Err(_) => continue,
        };
        if !is_managed_manifest(&current, expected) {
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

    fn set_synthetic_key_write_denied(path: &str, denied: bool) -> std::io::Result<()> {
        use std::os::windows::process::CommandExt;
        let script = r#"
$ErrorActionPreference='Stop'
$rights=[Security.AccessControl.RegistryRights]::ReadPermissions -bor [Security.AccessControl.RegistryRights]::ChangePermissions
$key=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($env:XARCHIVE_TEST_KEY,[Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,$rights)
if($null -eq $key){throw 'test key missing'}
try {
 $acl=$key.GetAccessControl()
 $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User
 $rule=[Security.AccessControl.RegistryAccessRule]::new($sid,[Security.AccessControl.RegistryRights]::SetValue,[Security.AccessControl.InheritanceFlags]::None,[Security.AccessControl.PropagationFlags]::None,[Security.AccessControl.AccessControlType]::Deny)
 if($env:XARCHIVE_TEST_DENY -eq '1'){$acl.AddAccessRule($rule)}else{$acl.RemoveAccessRuleSpecific($rule)}
 $key.SetAccessControl($acl)
} finally { $key.Dispose() }
"#;
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("XARCHIVE_TEST_KEY", path)
            .env("XARCHIVE_TEST_DENY", if denied { "1" } else { "0" })
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "test ACL setup failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    #[test]
    fn registration_requires_the_fixed_extension_identity() {
        for origins in [
            serde_json::json!([]),
            serde_json::json!(["chrome-extension://other/"]),
            serde_json::json!([super::ALLOWED_ORIGIN, "chrome-extension://other/"]),
        ] {
            assert!(!super::has_fixed_extension_origin(
                &serde_json::json!({"allowed_origins":origins})
            ));
        }
        assert!(super::has_fixed_extension_origin(
            &serde_json::json!({"allowed_origins":[super::ALLOWED_ORIGIN]})
        ));
    }

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
    fn registration_from_another_portable_package_is_not_current() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("xarchive-host-identity-{unique}"));
        let previous = root.join("previous").join(super::HOST_EXECUTABLE_FILE);
        let current = root.join("current").join(super::HOST_EXECUTABLE_FILE);
        std::fs::create_dir_all(previous.parent().unwrap()).unwrap();
        std::fs::create_dir_all(current.parent().unwrap()).unwrap();
        std::fs::write(&previous, b"identical synthetic binary").unwrap();
        std::fs::write(&current, b"identical synthetic binary").unwrap();
        let manifest = root.join("host.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(&serde_json::json!({
                "name": super::HOST_NAME,
                "type": "stdio",
                "path": previous,
                "allowed_origins": [super::ALLOWED_ORIGIN],
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(!super::registered_manifest_is_usable(&manifest, &current));
        let mut contents: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        contents["path"] = serde_json::json!(current);
        std::fs::write(&manifest, serde_json::to_vec(&contents).unwrap()).unwrap();
        assert!(super::registered_manifest_is_usable(&manifest, &current));
        assert!(!super::registered_manifest_is_usable(
            &manifest,
            &root.join("missing.exe")
        ));
        let alias = PathBuf::from(current.to_str().unwrap().to_uppercase());
        assert!(super::registered_manifest_is_usable(&manifest, &alias));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn isolated_registry_register_repair_and_unregister() {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let key_name = format!(
            r"Software\XArchiveValidation-{}-{unique}",
            std::process::id()
        );
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (registry, disposition) = hkcu.create_subkey(&key_name).unwrap();
        assert_eq!(disposition, winreg::enums::REG_CREATED_NEW_KEY);
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.0);
            }
        }
        let _cleanup = Cleanup(key_name);
        let root = std::env::temp_dir().join(format!("xarchive-host-registry-{unique}"));
        let previous = root.join("previous");
        let current = root.join("current");
        for package in [&previous, &current] {
            let host_dir = package.join("native-host");
            std::fs::create_dir_all(&host_dir).unwrap();
            std::fs::write(host_dir.join(super::HOST_EXECUTABLE_FILE), b"synthetic").unwrap();
            std::fs::write(
                host_dir.join(super::HOST_MANIFEST_FILE),
                serde_json::to_vec(&serde_json::json!({
                    "name": super::HOST_NAME,
                    "type": "stdio",
                    "path": "staging-path",
                    "allowed_origins": [super::ALLOWED_ORIGIN],
                }))
                .unwrap(),
            )
            .unwrap();
        }
        let target = root.join("managed").join(super::HOST_MANIFEST_FILE);
        let keys = [("Edge", "Edge"), ("Chrome", "Chrome")];
        super::register_or_repair_at(&previous, || Ok(target.clone()), &registry, &keys).unwrap();
        for (_, base) in keys {
            let key = registry
                .open_subkey(format!(r"{base}\{}", super::HOST_NAME))
                .unwrap();
            assert_eq!(
                key.get_value::<String, _>("").unwrap(),
                target.display().to_string()
            );
        }
        assert!(super::native_host_registered_at(
            &previous, &target, &registry, &keys
        ));
        assert!(!super::native_host_registered_at(
            &current, &target, &registry, &keys
        ));

        // Pause a real repair after staging, then try another repair and unregister.
        let (a_ready_tx, a_ready_rx) = std::sync::mpsc::channel();
        let (a_go_tx, a_go_rx) = std::sync::mpsc::channel();
        let (a_result, b_result, unregister_result, child) = std::thread::scope(|scope| {
            let key_name = &_cleanup.0;
            let target_ref = &target;
            let current_ref = &current;
            let keys_ref = &keys;
            let a = scope.spawn(move || {
                let user = RegKey::predef(HKEY_CURRENT_USER)
                    .open_subkey_with_flags(
                        key_name,
                        winreg::enums::KEY_READ | winreg::enums::KEY_WRITE,
                    )
                    .unwrap();
                super::register_or_repair_at_inner(
                    current_ref,
                    || Ok(target_ref.clone()),
                    &user,
                    keys_ref,
                    || {
                        a_ready_tx.send(()).unwrap();
                        a_go_rx.recv().unwrap();
                    },
                )
            });
            a_ready_rx.recv().unwrap();
            let b_result =
                super::register_or_repair_at(&previous, || Ok(target.clone()), &registry, &keys);
            let unregister_result = super::unregister_at(&target, &registry, &keys);
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "windows_transport::tests::registration_lock_child",
                ])
                .env("XARCHIVE_TEST_REGISTRATION_LOCK", &target)
                .output()
                .unwrap();
            a_go_tx.send(()).unwrap();
            (a.join().unwrap(), b_result, unregister_result, child)
        });
        assert!(a_result.is_ok());
        assert!(b_result.is_err(), "concurrent repair must be refused");
        assert!(
            unregister_result.is_err(),
            "concurrent unregister must be refused"
        );
        assert!(
            child.status.success(),
            "separate process must observe the lock: {}",
            String::from_utf8_lossy(&child.stdout)
        );
        assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
        assert!(super::native_host_registered_at(
            &current, &target, &registry, &keys
        ));
        super::register_or_repair_at(&previous, || Ok(target.clone()), &registry, &keys).unwrap();

        let chrome_path = format!(r"Chrome\{}", super::HOST_NAME);
        let edge_path = format!(r"Edge\{}", super::HOST_NAME);
        struct RestoreAcl {
            key: String,
            active: bool,
        }
        impl Drop for RestoreAcl {
            fn drop(&mut self) {
                if self.active {
                    let _ = set_synthetic_key_write_denied(&self.key, false);
                }
            }
        }
        let acl_key = format!(r"{}\{}", _cleanup.0, chrome_path);
        let mut restore_acl = RestoreAcl {
            key: acl_key,
            active: true,
        };
        set_synthetic_key_write_denied(&restore_acl.key, true).unwrap();
        assert!(registry.open_subkey(&chrome_path).is_ok());
        assert_eq!(
            registry
                .open_subkey_with_flags(
                    &chrome_path,
                    winreg::enums::KEY_READ | winreg::enums::KEY_WRITE
                )
                .unwrap_err()
                .raw_os_error(),
            Some(5)
        );
        let manifest_before_denial = std::fs::read(&target).unwrap();
        let denied =
            super::register_or_repair_at(&current, || Ok(target.clone()), &registry, &keys);
        let manifest_after_denial = std::fs::read(&target).unwrap();
        set_synthetic_key_write_denied(&restore_acl.key, false).unwrap();
        restore_acl.active = false;
        assert!(denied.is_err(), "denied browser key must reject repair");
        assert_eq!(manifest_after_denial, manifest_before_denial);
        // A reader can allow writes while denying replacement. Repair must not
        // fall back to modifying the managed manifest in place in that case.
        use std::os::windows::fs::OpenOptionsExt;
        let reader = OpenOptions::new()
            .read(true)
            .share_mode(0x1 | 0x2) // FILE_SHARE_READ | FILE_SHARE_WRITE
            .open(&target)
            .unwrap();
        let before_locked_repair = std::fs::read(&target).unwrap();
        let locked_repair =
            super::register_or_repair_at(&current, || Ok(target.clone()), &registry, &keys);
        drop(reader);
        assert!(
            locked_repair.is_err(),
            "replacement-denied repair must fail"
        );
        assert_eq!(std::fs::read(&target).unwrap(), before_locked_repair);
        let chrome = registry
            .open_subkey_with_flags(
                &chrome_path,
                winreg::enums::KEY_READ | winreg::enums::KEY_WRITE,
            )
            .unwrap();
        chrome.set_value("", &"foreign-manifest").unwrap();
        let manifest_before = std::fs::read(&target).unwrap();
        assert!(
            super::register_or_repair_at(&current, || Ok(target.clone()), &registry, &keys)
                .is_err()
        );
        assert_eq!(std::fs::read(&target).unwrap(), manifest_before);
        assert!(super::unregister_at(&target, &registry, &keys).is_err());
        let edge = registry.open_subkey(&edge_path).unwrap();
        assert_eq!(
            edge.get_value::<String, _>("").unwrap(),
            target.display().to_string()
        );
        chrome.set_value("", &target.display().to_string()).unwrap();
        drop((chrome, edge));

        super::register_or_repair_at(&current, || Ok(target.clone()), &registry, &keys).unwrap();
        for (_, base) in keys {
            let key = registry
                .open_subkey(format!(r"{base}\{}", super::HOST_NAME))
                .unwrap();
            assert_eq!(
                key.get_value::<String, _>("").unwrap(),
                target.display().to_string()
            );
        }
        assert!(super::native_host_registered_at(
            &current, &target, &registry, &keys
        ));
        assert!(!super::native_host_registered_at(
            &previous, &target, &registry, &keys
        ));
        super::unregister_at(&target, &registry, &keys).unwrap();
        assert!(!super::native_host_registered_at(
            &current, &target, &registry, &keys
        ));
        for (_, base) in keys {
            let key = registry
                .open_subkey(format!(r"{base}\{}", super::HOST_NAME))
                .unwrap();
            assert!(key.get_value::<String, _>("").is_err());
        }
        drop(registry);
        hkcu.delete_subkey_all(&_cleanup.0).unwrap();
        assert!(hkcu.open_subkey(&_cleanup.0).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registration_lock_child() {
        if let Some(target) = std::env::var_os("XARCHIVE_TEST_REGISTRATION_LOCK") {
            assert!(super::registration_lock(Path::new(&target)).is_err());
        }
    }

    #[test]
    fn pipe_bootstrap_and_stalled_client_shutdown() {
        use std::sync::Arc;
        use std::time::{Duration, Instant};
        use xarchive_native_host::{read_json, write_json};
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("xarchive-bootstrap-{unique}"));
        std::fs::create_dir_all(&root).unwrap();
        let database = root.join("archive.sqlite3");
        let endpoint = PathBuf::from(format!(r"\\.\pipe\xarchive-bootstrap-{unique}"));
        let mut executor = crate::executor::ExecutorRuntime::new(database.clone());
        let pairing = Arc::new(crate::browser_pairing::PairingCoordinator::new(45678).unwrap());
        let server = DesktopTransportServer::start_with_pairing(
            executor.service(),
            database,
            endpoint.clone(),
            Some(pairing.clone()),
            xarchive_storage::BatchOutputSettings::default(),
            false,
            crate::executor::ToolCompatibility::default(),
        )
        .unwrap();
        assert!(
            DesktopTransportServer::start_with_pairing(
                executor.service(),
                root.join("other.sqlite3"),
                endpoint.clone(),
                None,
                xarchive_storage::BatchOutputSettings::default(),
                false,
                crate::executor::ToolCompatibility::default(),
            )
            .is_err()
        );
        {
            let mut client =
                xarchive_native_host::windows_pipe::BoundedPipe::connect(&endpoint).unwrap();
            write_json(&mut client, &serde_json::json!({"protocol_version":1,"message_type":"bootstrap","request_id":"win-bootstrap"})).unwrap();
            let response: xarchive_protocol::BrowserPairingResponse =
                read_json(&mut client).unwrap().unwrap();
            match response {
                xarchive_protocol::BrowserPairingResponse::Bootstrap {
                    ticket,
                    port,
                    request_id,
                    ..
                } => {
                    assert_eq!(port, 45678);
                    assert_eq!(request_id, "win-bootstrap");
                    assert!(pairing.consume(&ticket, crate::browser_pairing::EXTENSION_ORIGIN));
                    assert!(!pairing.consume(&ticket, crate::browser_pairing::EXTENSION_ORIGIN));
                }
                other => panic!("unexpected bootstrap response: {other:?}"),
            }
        }
        let _stalled = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&endpoint)
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let start = Instant::now();
        drop(server);
        assert!(start.elapsed() < Duration::from_secs(1));
        executor.shutdown_in_place().unwrap();
        std::fs::remove_dir_all(root).unwrap();
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
        let server = DesktopTransportServer::start_with_pairing(
            executor.service(),
            database_path,
            endpoint.clone(),
            None,
            xarchive_storage::BatchOutputSettings::default(),
            false,
            crate::executor::ToolCompatibility::default(),
        )
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
