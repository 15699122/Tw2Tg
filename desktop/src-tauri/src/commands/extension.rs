//! Extension domain commands: status projection, directory import and Native
//! Host registration.
//!
//! These are thin adapters over `RuntimeState`, the portable path resolver and
//! the extension/native-host platform layer. Command names, argument shapes,
//! serialized fields and error strings are the registered IPC contract and must
//! not change here.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

use crate::RuntimeState;
use crate::websocket_transport::WebSocketDiagnosticSnapshot;

#[derive(Debug, Serialize)]
pub struct ExtensionStatus {
    pub files_ready: bool,
    pub directory: String,
    pub browser_connection: String,
    pub native_host: String,
    pub message: String,
    pub source: String,
    pub version: Option<String>,
    pub websocket_port: Option<u16>,
    pub websocket_runtime_instance_id: Option<String>,
    pub websocket_authenticated: bool,
    pub websocket_connection: String,
    pub websocket_diagnostics: WebSocketDiagnosticSnapshot,
    pub websocket_error: Option<String>,
}

fn extension_status_from_state(state: &RuntimeState) -> Result<ExtensionStatus, String> {
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    let directory =
        crate::portable::resolve_config_path(&paths.root, &state.config.extension.directory);
    let files_ready = [
        directory.join("manifest.json"),
        directory.join("src").join("background.js"),
        directory.join("src").join("content.js"),
    ]
    .iter()
    .all(|path| path.is_file());
    #[cfg(windows)]
    let (browser_connection, native_host, message) = {
        let host_executable = paths
            .root
            .join("native-host")
            .join("xarchive-native-host.exe");
        let host_registered = crate::windows_transport::native_host_registered()?;
        let browser_connection = state
            .transport_server
            .as_ref()
            .map(|server| server.session.browser_connection().to_owned())
            .unwrap_or_else(|| "error".to_owned());
        let native_host = if !host_executable.is_file() {
            "missing"
        } else if host_registered {
            "registered"
        } else {
            "not_registered"
        };
        let message = if !files_ready {
            "未找到完整的 Extension 文件，请导入本地目录。".to_owned()
        } else if !host_executable.is_file() {
            "Extension 文件已就绪；当前包缺少 Native Host，请使用 Full package。".to_owned()
        } else if !host_registered {
            "Extension 和 Native Host 文件已就绪；Native Host 尚未注册到当前用户的 Edge/Chrome。"
                .to_owned()
        } else if let Some(error) = state.transport_error.as_deref() {
            format!("Native Host 已注册，但 Desktop Named Pipe 服务未启动：{error}")
        } else if browser_connection == "connected" {
            // `browser_connection` combines two live sources: an active
            // WebSocket socket, or a Named Pipe request inside its 30-second
            // freshness window. Either one means the browser path was recently
            // alive; the panel's `websocket_*` fields say which one.
            "收到浏览器活动请求，Desktop transport 正常；具体通道见下方 WebSocket 状态。".to_owned()
        } else if browser_connection == "disconnected" {
            "曾收到浏览器请求；目前没有活动连接，请检查浏览器扩展或重启 Desktop。".to_owned()
        } else {
            "文件已就绪且 Native Host 已注册；Desktop 尚未观察到浏览器连接。".to_owned()
        };
        (browser_connection, native_host.to_owned(), message)
    };
    #[cfg(not(windows))]
    let (browser_connection, native_host, message) = (
        if files_ready {
            "not_loaded".to_owned()
        } else {
            "missing".to_owned()
        },
        if !files_ready {
            "missing".to_owned()
        } else {
            "not_available".to_owned()
        },
        if files_ready {
            "扩展文件已就绪；浏览器尚未加载，Native Host 注册和连接需要在目标环境验证。".to_owned()
        } else {
            "未找到完整的 Extension 文件，请导入本地目录。".to_owned()
        },
    );
    let (
        websocket_port,
        websocket_runtime_instance_id,
        websocket_authenticated,
        websocket_connection,
        websocket_diagnostics,
    ) = state
        .websocket_server
        .as_ref()
        .map(|server| {
            (
                Some(server.port()),
                Some(server.runtime_instance_id().to_owned()),
                server.session.authenticated(),
                server.session.browser_connection().to_owned(),
                server.session.diagnostic_snapshot(),
            )
        })
        .unwrap_or_else(|| {
            (
                None,
                None,
                false,
                "not_started".to_owned(),
                crate::websocket_transport::WebSocketDiagnosticSnapshot {
                    accepted: 0,
                    handshake_failed: 0,
                    auth_read_failed: 0,
                    auth_received: 0,
                    auth_succeeded: 0,
                    auth_failed: 0,
                    auth_response_failed: 0,
                    close_before_auth: 0,
                    close_after_auth: 0,
                    last_request_age_seconds: None,
                },
            )
        });
    Ok(ExtensionStatus {
        files_ready,
        directory: directory.display().to_string(),
        browser_connection,
        native_host,
        message,
        source: state.config.extension.source.clone(),
        version: state.config.extension.version.clone(),
        websocket_port,
        websocket_runtime_instance_id,
        websocket_authenticated,
        websocket_connection,
        websocket_diagnostics,
        websocket_error: state.websocket_error.clone(),
    })
}

fn validate_extension_directory(directory: &Path) -> Result<(String, String), String> {
    let manifest_path = directory.join("manifest.json");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("无法读取 manifest.json：{error}"))?;
    let value: serde_json::Value = serde_json::from_str(&manifest)
        .map_err(|error| format!("manifest.json 格式错误：{error}"))?;
    if value
        .get("manifest_version")
        .and_then(|value| value.as_u64())
        != Some(3)
        || value.get("name").and_then(|value| value.as_str()).is_none()
        || value
            .get("version")
            .and_then(|value| value.as_str())
            .is_none()
    {
        return Err("Extension manifest 必须包含 Manifest V3、name 和 version".to_owned());
    }
    for required in ["src/background.js", "src/content.js"] {
        if !directory.join(required).is_file() {
            return Err(format!("Extension 缺少必需文件：{required}"));
        }
    }
    Ok((
        value["version"].as_str().unwrap_or_default().to_owned(),
        sha256_directory_marker(directory)?,
    ))
}

fn sha256_directory_marker(directory: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(directory, directory, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for (relative, path) in files {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(fs::read(path).map_err(|error| error.to_string())?);
        hasher.update([0]);
    }
    Ok(format!("{0:x}", hasher.finalize()))
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
            files.push((relative.to_string_lossy().replace('\\', "/"), path));
        }
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, target: &Path) -> Result<(), String> {
    let metadata =
        fs::metadata(source).map_err(|error| format!("无法读取 Extension 目录：{error}"))?;
    if !metadata.is_dir() {
        return Err("Extension 导入路径必须是目录".to_owned());
    }
    fs::create_dir_all(target).map_err(|error| format!("无法创建临时 Extension 目录：{error}"))?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let destination = target.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            copy_directory_contents(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination).map_err(|error| error.to_string())?;
        } else {
            return Err("Extension 目录包含不支持的特殊文件".to_owned());
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn register_native_host(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ExtensionStatus, String> {
    #[cfg(windows)]
    {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        crate::windows_transport::register_or_repair(&state.portable_root)?;
        extension_status_from_state(&state)
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        Err("Native Host registration is only available on Windows".to_owned())
    }
}

#[tauri::command]
pub(crate) fn unregister_native_host(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ExtensionStatus, String> {
    #[cfg(windows)]
    {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        crate::windows_transport::unregister()?;
        extension_status_from_state(&state)
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        Err("Native Host registration is only available on Windows".to_owned())
    }
}

#[tauri::command]
pub(crate) fn import_extension_directory(
    state: State<'_, Mutex<RuntimeState>>,
    source: String,
) -> Result<ExtensionStatus, String> {
    let source = PathBuf::from(source.trim());
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    let target =
        crate::portable::resolve_config_path(&paths.root, &state.config.extension.directory);
    if source == target {
        return Err("导入源不能是 XArchive 当前 Extension 目录".to_owned());
    }
    let temporary = paths.cache_dir.join(format!(
        "extension-import-{}",
        crate::runtime::timestamp_marker()
    ));
    let _ = fs::remove_dir_all(&temporary);
    copy_directory_contents(&source, &temporary)?;
    let (version, sha256) = validate_extension_directory(&temporary)?;
    let backup = paths.cache_dir.join("extension-backup");
    let _ = fs::remove_dir_all(&backup);
    if target.exists() {
        fs::rename(&target, &backup).map_err(|error| format!("无法备份现有 Extension：{error}"))?;
    }
    if let Err(error) = fs::rename(&temporary, &target) {
        if backup.exists() {
            let _ = fs::rename(&backup, &target);
        }
        return Err(format!("无法安装 Extension：{error}"));
    }
    let _ = fs::remove_dir_all(&backup);
    state.config.extension.source = "imported".to_owned();
    state.config.extension.version = Some(version);
    state.config.extension.sha256 = Some(sha256);
    state.config.save(&paths)?;
    extension_status_from_state(&state)
}

#[tauri::command]
pub(crate) fn get_extension_status(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ExtensionStatus, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    extension_status_from_state(&state)
}

#[tauri::command]
pub(crate) fn open_extension_folder(state: State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let path = get_extension_status(state)?.directory;
    let mut command = crate::platform::open_path_command(std::path::Path::new(&path));
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open extension folder: {error}"))
}
