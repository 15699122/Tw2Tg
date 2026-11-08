//! Settings commands: network and proxy, theme, downloader arguments, output and
//! metadata output, application logging and the aria2/sidecar tool paths.
//!
//! The network view never returns a stored proxy credential, and the output and
//! downloader settings readers are shared with the batch and executor domains.
//! Command names, argument shapes, serialized fields and error strings are the
//! registered IPC contract and must not change here.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;
use xarchive_core::ProxyDecision;
use xarchive_storage::Database;

use crate::RuntimeState;
use crate::commands::app::{AppStatus, app_status};
use crate::commands::open_batch_database;
use crate::config::{LogLevel, MAX_LOG_MAX_FILES, MIN_LOG_MAX_FILES, ProxyMode};
use crate::proxy::ProxyHttpClient;

/// The proxy settings the Settings page reads and writes.
///
/// The stored value is never returned. `proxy` comes back empty unless the user
/// is editing it, so the frontend can never read back a stored credential.
#[derive(Debug, Serialize)]
pub struct NetworkSettings {
    pub proxy_mode: String,
    /// Empty when nothing is stored, or when a value is stored but not in
    /// effect. The UI shows the redacted summary from `proxy_summary`.
    pub proxy: String,
    pub proxy_summary: Option<String>,
    pub proxy_configured: bool,
    pub proxy_active: bool,
    pub supported_modes: Vec<String>,
    pub system_proxy_note: String,
    pub system_proxy_supported: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSettingsInput {
    pub proxy_mode: String,
    #[serde(default)]
    pub proxy: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProxyDiagnostic {
    pub mode: String,
    pub url: String,
    /// `proxy`, `direct`, `failed`, or `unsupported`.
    pub route: String,
    /// The redacted proxy. Never contains credentials.
    pub proxy: Option<String>,
    pub message: String,
    /// Which layer produced the route: `environment`, `system-static`, `pac`,
    /// `wpad`, `manual`, or `explicit-direct`.
    pub source: String,
    /// Every candidate in priority order, redacted. A PAC result such as
    /// `PROXY a; DIRECT` produces more than one entry.
    pub candidates: Vec<String>,
    /// The redacted system configuration behind `System` mode.
    pub system: ProxySystemSummary,
}

/// The display-safe system proxy configuration.
#[derive(Debug, serde::Serialize)]
pub struct ProxySystemSummary {
    /// Which resolver backs `System` mode.
    pub backend: String,
    /// Whether an environment proxy is set for this process.
    pub environment_proxy: bool,
    /// The effective `no_proxy` entries.
    pub bypass: Vec<String>,
    /// Whether the OS asked for automatic discovery.
    pub auto_detect: bool,
    /// The redacted PAC URL, when one is configured.
    pub pac_url: Option<String>,
    /// PAC/WPAD source state.
    pub pac_state: Option<String>,
    /// Redacted static system proxy entries.
    pub static_proxies: Vec<String>,
    /// The OS proxy bypass list.
    pub system_bypass: Option<String>,
    /// The resolver configuration revision; a change invalidates cached routes.
    pub generation: u64,
    /// Whether a child process (gallery-dl, aria2) can follow the current
    /// policy on its own, or the caller must route each request itself.
    pub child_coverage: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OutputSettingsInput {
    pub settings: xarchive_storage::BatchOutputSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationSettingsInput {
    pub logging_level: LogLevel,
    pub max_log_files: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DownloaderSettingsForTransport {
    #[serde(default = "empty_argument_array")]
    pub(crate) gallery_dl_json: String,
    #[serde(default = "empty_argument_array")]
    pub(crate) aria2_json: String,
}

const OUTPUT_SETTINGS_KEY: &str = "download.output_settings";

const THEME_SETTINGS_KEY: &str = "ui.theme";

pub(crate) type DownloaderSettings = DownloaderSettingsForTransport;

impl ThemePreference {
    fn parse(value: &str) -> Result<Self, String> {
        serde_json::from_str(value)
            .map_err(|error| format!("invalid persisted theme preference: {error}"))
    }
}

fn network_settings(state: &RuntimeState) -> NetworkSettings {
    let diagnostics = state.config.network.diagnostics();
    NetworkSettings {
        proxy_mode: diagnostics.proxy_mode,
        proxy: String::new(),
        proxy_summary: diagnostics.proxy,
        proxy_configured: diagnostics.proxy_configured,
        proxy_active: diagnostics.proxy_active,
        supported_modes: vec![
            ProxyMode::System.as_str().to_owned(),
            ProxyMode::Direct.as_str().to_owned(),
            ProxyMode::Manual.as_str().to_owned(),
        ],
        system_proxy_note: system_proxy_note(),
        system_proxy_supported: system_proxy_supported(),
    }
}

/// What `System` actually resolves to on this platform.
///
/// The Settings page shows this rather than promising platform-wide system proxy
/// support, because the resolver only reads the process environment until the
/// Windows native resolver lands.
fn system_proxy_note() -> String {
    if cfg!(target_os = "windows") {
        "System follows the Windows static proxy, bypass list, PAC and WPAD the way the \
         operating system does, resolving each URL against the current policy. A PAC or \
         WPAD policy is not handed to gallery-dl or aria2, because those read the proxy \
         environment only once and cannot route per URL; such a job is refused instead \
         of running without the policy."
            .to_owned()
    } else {
        "System follows the proxy environment variables of the launching process.".to_owned()
    }
}

/// Whether this platform can resolve a system proxy beyond the environment.
///
/// Windows resolves through the native OS resolver, including per-URL PAC and
/// WPAD. Other platforms read the launching process's environment, which has no
/// equivalent, so they report false and the page does not overstate it.
fn system_proxy_supported() -> bool {
    cfg!(target_os = "windows")
}

/// Push a saved network configuration into the running runtime.
///
/// The executor holds a copy of the configuration, so a mode change only takes
/// effect once this runs. Reusing `replace_executor` keeps the Browser transport
/// on the same service generation instead of pointing it at a closed executor.
fn apply_network_config(state: &mut RuntimeState) -> Result<(), String> {
    let database_path = state.executor.database_path().to_owned();
    let config = crate::runtime::executor_config(
        &state.portable_root,
        &state.config,
        database_path,
        state.cache_root.join("staging"),
        state.download_root.clone(),
    );
    state.replace_executor(config)
}

/// The display name for a child-process coverage answer.
fn child_coverage_name(coverage: crate::proxy::ChildProxyCoverage) -> String {
    match coverage {
        crate::proxy::ChildProxyCoverage::InheritsEnvironment => "inherits-environment".to_owned(),
        crate::proxy::ChildProxyCoverage::RequiresCallerRouting => {
            "requires-caller-routing".to_owned()
        }
        crate::proxy::ChildProxyCoverage::NotApplicable => "not-applicable".to_owned(),
    }
}

fn route_name(decision: &ProxyDecision) -> &'static str {
    match decision {
        ProxyDecision::Direct => "direct",
        ProxyDecision::Proxy(_) => "proxy",
        ProxyDecision::ResolutionFailed(_) => "failed",
        ProxyDecision::Unsupported(_) => "unsupported",
    }
}

fn route_message(decision: &ProxyDecision) -> String {
    match decision {
        ProxyDecision::Direct => "This destination connects without a proxy.".to_owned(),
        ProxyDecision::Proxy(_) => {
            "This destination connects through the resolved proxy.".to_owned()
        }
        ProxyDecision::ResolutionFailed(reason) => {
            format!("The system proxy could not be resolved: {reason}. No request was sent.")
        }
        ProxyDecision::Unsupported(reason) => {
            format!("The selected mode cannot be applied: {reason}")
        }
    }
}

pub(crate) fn read_output_settings(
    database: &Database,
) -> Result<xarchive_storage::BatchOutputSettings, String> {
    match database
        .get_setting(OUTPUT_SETTINGS_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(value) => serde_json::from_str(&value)
            .map_err(|error| format!("invalid persisted output settings: {error}")),
        None => Ok(xarchive_storage::BatchOutputSettings::default()),
    }
}

pub(crate) fn validate_output_settings(
    settings: &xarchive_storage::BatchOutputSettings,
) -> Result<(), String> {
    const MAX_TEMPLATE_BYTES: usize = 512;
    if settings.filename_template.len() > MAX_TEMPLATE_BYTES
        || settings.filename_template.chars().any(char::is_control)
    {
        return Err("filename template is too long or contains control characters".into());
    }
    if matches!(
        settings.naming_mode,
        xarchive_storage::BatchNamingMode::Template
    ) && settings.filename_template.trim().is_empty()
    {
        return Err("filename template is required in template mode".into());
    }
    Ok(())
}

pub(crate) fn read_downloader_settings(database: &Database) -> Result<DownloaderSettings, String> {
    let raw = database
        .get_setting("download.downloader_arguments")
        .map_err(|error| error.to_string())?;
    let settings = match raw {
        Some(raw) => serde_json::from_str::<DownloaderSettings>(&raw)
            .map_err(|error| format!("invalid persisted downloader settings: {error}"))?,
        None => DownloaderSettings::default(),
    };
    validate_downloader_settings(&settings)?;
    Ok(settings)
}

pub(crate) fn validate_downloader_settings(settings: &DownloaderSettings) -> Result<(), String> {
    for (engine, raw) in [
        ("gallery-dl", &settings.gallery_dl_json),
        ("aria2", &settings.aria2_json),
    ] {
        let args: Vec<String> = serde_json::from_str(raw)
            .map_err(|_| format!("{engine} arguments must be a JSON string array"))?;
        let validation = if engine == "gallery-dl" {
            xarchive_download::downloader_args::validate_gallery_dl_args(&args)
        } else {
            xarchive_download::downloader_args::validate_aria2_args(&args)
        };
        validation.map_err(|error| format!("{engine} arguments rejected: {error}"))?;
    }
    Ok(())
}

pub(crate) fn snapshot_downloader_settings(
    settings: &DownloaderSettings,
    use_aria2: bool,
    tool_compatibility: crate::executor::ToolCompatibility,
) -> Result<crate::executor::DownloaderArgumentSnapshot, String> {
    validate_downloader_settings(settings)?;
    let gallery_dl = serde_json::from_str::<Vec<String>>(&settings.gallery_dl_json)
        .map_err(|_| "gallery-dl arguments must be a JSON string array".to_owned())?;
    let aria2 = serde_json::from_str::<Vec<String>>(&settings.aria2_json)
        .map_err(|_| "aria2 arguments must be a JSON string array".to_owned())?;
    let mut snapshot = crate::executor::DownloaderArgumentSnapshot {
        gallery_dl,
        aria2,
        policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
        tool_compatibility,
    };
    // Only the selected backend can receive task arguments. Tool identity is
    // bound at execution time to the resolved executable, not a package pin.
    if use_aria2 {
        snapshot.gallery_dl.clear();
    } else {
        snapshot.aria2.clear();
    }
    Ok(snapshot)
}

pub(crate) fn validate_transport_downloader_settings(
    settings: DownloaderSettingsForTransport,
    use_aria2: bool,
    compatibility: crate::executor::ToolCompatibility,
) -> Result<crate::executor::DownloaderArgumentSnapshot, String> {
    snapshot_downloader_settings(&settings, use_aria2, compatibility)
}

pub(crate) fn empty_argument_array() -> String {
    "[]".to_owned()
}

#[tauri::command]
pub(crate) fn get_network_settings(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<NetworkSettings, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    Ok(network_settings(&state))
}

#[tauri::command]
pub(crate) fn save_network_settings(
    state: State<'_, Mutex<RuntimeState>>,
    settings: NetworkSettingsInput,
) -> Result<NetworkSettings, String> {
    let mode = ProxyMode::parse(&settings.proxy_mode)?;
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let requested = settings
        .proxy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    match mode {
        // Switching away from `Manual` keeps the stored value, so returning to
        // `Manual` later does not require the user to retype a credential.
        ProxyMode::Manual => {
            let value = requested
                .or_else(|| state.config.network.normalized_proxy())
                .ok_or_else(|| "a proxy address is required when the mode is manual".to_owned())?;
            state.config.network.proxy = Some(value);
        }
        _ => {
            if let Some(value) = requested {
                state.config.network.proxy = Some(value);
            }
        }
    }
    state.config.network.proxy_mode = mode;
    // The mode is now explicit, so a later load must not re-run the migration.
    state.config.network.proxy_mode_declared = true;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    // Only the redacted summary may be logged: the configured value itself can
    // carry credentials and must never reach the log file.
    state.debug(
        "network",
        &format!(
            "applying mode={} summary={:?}",
            state.config.network.proxy_mode.as_str(),
            state.config.network.diagnostics(),
        ),
    );
    state.config.save(&paths)?;
    // A saved mode that the runtime then refuses to apply must not be reported
    // as success, so this failure surfaces instead of leaving the old route
    // active behind a "saved" message.
    apply_network_config(&mut state)?;
    // The redaction list now has to cover the proxy that was just stored, so the
    // log file is rebuilt the same way the logging settings command rebuilds it.
    state.log_file = crate::logging::LogFile::open(
        &state.logs_root,
        state.config.logging.effective_level(),
        state.config.logging.max_files,
    )
    .map(|log| log.with_secrets(state.config.log_secrets()))
    .ok();
    Ok(network_settings(&state))
}

/// Resolve the route for one URL without sending a request.
///
/// This is a diagnostic, not a connectivity test: it reports what the resolver
/// decided, so the Settings page can show a real result rather than a claim.
/// Resolution runs on a worker thread because a Windows PAC or WPAD lookup can
/// block, and the UI thread must stay responsive.
#[tauri::command]
pub(crate) fn inspect_proxy_route(
    state: State<'_, Mutex<RuntimeState>>,
    url: String,
) -> Result<ProxyDiagnostic, String> {
    let target = url.trim().to_owned();
    if target.is_empty() {
        return Err("a URL is required".to_owned());
    }
    let client = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        ProxyHttpClient::from_config(&state.config.network)
    };
    let resolved_url = target.clone();
    let mode = client.mode();
    // Resolution is moved off the calling thread because a platform resolver may
    // block. The UI thread must never wait on a PAC or WPAD lookup.
    let coverage_url = target.clone();
    let (resolution, system, child_coverage) = std::thread::spawn(move || {
        (
            client.resolution_for(&resolved_url),
            client.describe_system(),
            child_coverage_name(client.child_proxy_coverage(&coverage_url)),
        )
    })
    .join()
    .map_err(|_| "proxy resolution thread panicked".to_owned())?;

    let decision = resolution.to_decision();
    Ok(ProxyDiagnostic {
        mode: mode.as_str().to_owned(),
        url: target,
        route: route_name(&decision).to_owned(),
        proxy: decision.redacted_proxy(),
        message: route_message(&decision),
        source: resolution.source.as_str().to_owned(),
        candidates: resolution.redacted_candidates(),
        system: ProxySystemSummary {
            backend: system.backend,
            environment_proxy: system.environment_proxy,
            bypass: system.bypass,
            auto_detect: system.auto_detect,
            pac_url: system.pac_url,
            pac_state: system.pac_state,
            static_proxies: system.static_proxies,
            system_bypass: system.system_bypass,
            generation: system.generation,
            child_coverage,
        },
    })
}

/// Report the system proxy configuration without resolving any URL.
///
/// Reading the OS configuration may evaluate a PAC script, so this runs off
/// the calling thread for the same reason as [`inspect_proxy_route`].
#[tauri::command]
pub(crate) fn get_system_proxy_status(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ProxySystemSummary, String> {
    let client = {
        let state = state
            .lock()
            .map_err(|_| "runtime state lock poisoned".to_owned())?;
        ProxyHttpClient::from_config(&state.config.network)
    };
    let probe = crate::proxy::SIDECAR_PROXY_PROBE_URL.to_owned();
    let (system, child_coverage) = std::thread::spawn(move || {
        (
            client.describe_system(),
            child_coverage_name(client.child_proxy_coverage(&probe)),
        )
    })
    .join()
    .map_err(|_| "proxy status thread panicked".to_owned())?;
    Ok(ProxySystemSummary {
        backend: system.backend,
        environment_proxy: system.environment_proxy,
        bypass: system.bypass,
        auto_detect: system.auto_detect,
        pac_url: system.pac_url,
        pac_state: system.pac_state,
        static_proxies: system.static_proxies,
        system_bypass: system.system_bypass,
        generation: system.generation,
        child_coverage,
    })
}

#[tauri::command]
pub(crate) fn get_theme_preference(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<ThemePreference, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let database = open_batch_database(&state)?;
    match database
        .get_setting(THEME_SETTINGS_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(value) => ThemePreference::parse(&value),
        None => Ok(ThemePreference::System),
    }
}

#[tauri::command]
pub(crate) fn save_theme_preference(
    state: State<'_, Mutex<RuntimeState>>,
    theme: ThemePreference,
) -> Result<ThemePreference, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let database = open_batch_database(&state)?;
    let value = serde_json::to_string(&theme).map_err(|error| error.to_string())?;
    database
        .set_setting(THEME_SETTINGS_KEY, &value, &crate::clock::now_iso())
        .map_err(|error| error.to_string())?;
    Ok(theme)
}

#[tauri::command]
pub(crate) fn get_downloader_argument_settings(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<serde_json::Value, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let settings = read_downloader_settings(&open_batch_database(&state)?)?;
    serde_json::from_str(&serde_json::to_string(&settings).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn save_downloader_argument_settings(
    state: State<'_, Mutex<RuntimeState>>,
    settings: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let settings: DownloaderSettings = serde_json::from_value(settings)
        .map_err(|error| format!("invalid downloader settings: {error}"))?;
    validate_downloader_settings(&settings)?;
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let database = open_batch_database(&state)?;
    let value = serde_json::to_string(&settings).map_err(|error| error.to_string())?;
    database
        .set_setting(
            "download.downloader_arguments",
            &value,
            &crate::clock::now_iso(),
        )
        .map_err(|error| error.to_string())?;
    serde_json::to_value(settings).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_output_settings(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<xarchive_storage::BatchOutputSettings, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let database = open_batch_database(&state)?;
    read_output_settings(&database)
}

#[tauri::command]
pub(crate) fn get_metadata_output_settings(
    state: State<'_, Mutex<RuntimeState>>,
) -> Result<crate::executor::MetadataOutputSettings, String> {
    let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
    let database = open_batch_database(&state)?;
    Ok(crate::executor::MetadataOutputSettings::read(&database)?.unwrap_or_default())
}

#[tauri::command]
pub(crate) fn save_metadata_output_settings(
    state: State<'_, Mutex<RuntimeState>>,
    settings: crate::executor::MetadataOutputSettings,
) -> Result<crate::executor::MetadataOutputSettings, String> {
    validate_output_settings(&settings.legacy_naming())?;
    let state = state.lock().map_err(|_| "runtime state lock poisoned")?;
    let database = open_batch_database(&state)?;
    database
        .set_setting(
            "download.metadata_output.v4",
            &serde_json::to_string(&settings).map_err(|e| e.to_string())?,
            &crate::clock::now_iso(),
        )
        .map_err(|e| e.to_string())?;
    Ok(settings)
}

/// F3 accepted-set import under the workspace maintenance barrier.
///
/// Linux-only: the turnstile/active lock stops new production sessions while
/// existing ones drain, so the single-transaction import cannot race a writer.
/// Any commit error or conflicting duplicate rolls back the entire accepted set.
#[tauri::command]
pub(crate) fn save_output_settings(
    state: State<'_, Mutex<RuntimeState>>,
    settings: OutputSettingsInput,
) -> Result<xarchive_storage::BatchOutputSettings, String> {
    validate_output_settings(&settings.settings)?;
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let database = open_batch_database(&state)?;
    let value = serde_json::to_string(&settings.settings).map_err(|error| error.to_string())?;
    database
        .set_setting(OUTPUT_SETTINGS_KEY, &value, &crate::clock::now_iso())
        .map_err(|error| error.to_string())?;
    Ok(settings.settings)
}

#[tauri::command]
pub(crate) fn save_application_settings(
    state: State<'_, Mutex<RuntimeState>>,
    settings: ApplicationSettingsInput,
) -> Result<AppStatus, String> {
    if !(MIN_LOG_MAX_FILES..=MAX_LOG_MAX_FILES).contains(&settings.max_log_files) {
        return Err(format!(
            "max_log_files must be between {MIN_LOG_MAX_FILES} and {MAX_LOG_MAX_FILES}"
        ));
    }
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    // The user picked a level, so it is stored as an explicit choice and keeps
    // winning over the build channel default on every later start.
    state.config.logging.level = Some(settings.logging_level);
    state.config.logging.max_files = settings.max_log_files;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.debug(
        "logging",
        &format!(
            "user selected level={} (channel={} default={}) max_files={}",
            settings.logging_level.as_str(),
            crate::build_channel::release_channel().as_str(),
            crate::build_channel::channel_default_log_level().as_str(),
            settings.max_log_files,
        ),
    );
    state.config.save(&paths)?;
    state.log_file = crate::logging::LogFile::open(
        &state.logs_root,
        settings.logging_level,
        settings.max_log_files,
    )
    .map(|log| log.with_secrets(state.config.log_secrets()))
    .ok();
    Ok(app_status(&state))
}

/// Persist the optional aria2 transfer choice and push it into the executor.
///
/// The executor holds a copy of the configuration, so toggling only takes
/// effect once `apply_network_config` rebuilds it. gallery-dl performs the byte
/// download whenever the flag is off, so this never depends on aria2 running.
#[tauri::command]
pub(crate) fn set_use_aria2(
    state: State<'_, Mutex<RuntimeState>>,
    use_aria2: bool,
) -> Result<AppStatus, String> {
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let changed = state.config.download.use_aria2 != use_aria2;
    state.config.download.use_aria2 = use_aria2;
    if changed {
        state.debug(
            "download",
            &format!("use_aria2={use_aria2} transfer backend updated"),
        );
    }
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.config.save(&paths)?;
    if changed {
        apply_network_config(&mut state)?;
    }
    Ok(app_status(&state))
}

#[tauri::command]
pub(crate) fn get_sidecar_path(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    Ok(
        crate::portable::resolve_config_path(&paths.root, &state.config.sidecar.gallery_dl)
            .display()
            .to_string(),
    )
}

#[tauri::command]
pub(crate) fn save_aria2_path(
    state: State<'_, Mutex<RuntimeState>>,
    path: String,
) -> Result<crate::aria2::Aria2Installation, String> {
    let validated = crate::aria2::validate_aria2_path(path.clone());
    if !validated.found {
        return Err(validated
            .error
            .unwrap_or_else(|| "the given path is not a working aria2c executable".to_owned()));
    }
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    state.config.sidecar.aria2 = validated.path.clone().unwrap_or(path);
    let paths = crate::portable::PortablePaths::from_root(state.portable_root.clone());
    state.config.save(&paths)?;
    Ok(validated)
}

#[cfg(test)]
mod output_settings_tests {
    use super::{ThemePreference, read_output_settings, validate_output_settings};
    use xarchive_storage::{BatchNamingMode, BatchOutputSettings, Database};

    #[test]
    fn missing_output_settings_use_legacy_compatible_defaults() {
        let database = Database::open_in_memory().expect("database");
        assert_eq!(
            read_output_settings(&database).expect("defaults"),
            BatchOutputSettings::default()
        );
    }

    #[test]
    fn corrupt_persisted_output_settings_fail_closed() {
        let database = Database::open_in_memory().expect("database");
        database
            .set_setting("download.output_settings", r#"{"extra":true}"#, "now")
            .expect("store malformed typed value");
        assert!(
            read_output_settings(&database)
                .expect_err("invalid settings must be rejected")
                .contains("invalid persisted output settings")
        );
    }

    #[test]
    fn output_settings_validation_rejects_empty_template_and_control_characters() {
        let empty = BatchOutputSettings {
            naming_mode: BatchNamingMode::Template,
            ..BatchOutputSettings::default()
        };
        assert!(validate_output_settings(&empty).is_err());
        let controls = BatchOutputSettings {
            filename_template: "bad\nname".into(),
            ..BatchOutputSettings::default()
        };
        assert!(validate_output_settings(&controls).is_err());
    }

    #[test]
    fn theme_preference_accepts_only_the_versioned_ui_choices() {
        assert_eq!(
            ThemePreference::parse("\"system\""),
            Ok(ThemePreference::System)
        );
        assert_eq!(
            ThemePreference::parse("\"light\""),
            Ok(ThemePreference::Light)
        );
        assert_eq!(
            ThemePreference::parse("\"dark\""),
            Ok(ThemePreference::Dark)
        );
        assert!(ThemePreference::parse("\"sepia\"").is_err());
        assert!(ThemePreference::parse("{\"theme\":\"dark\"}").is_err());
    }

    #[test]
    fn theme_preference_persists_in_settings_meta_and_defaults_to_system() {
        let database = Database::open_in_memory().expect("database");
        let initial = database.get_setting("ui.theme").expect("read setting");
        assert!(initial.is_none());
        database
            .set_setting("ui.theme", "\"dark\"", "now")
            .expect("persist theme");
        assert_eq!(
            ThemePreference::parse(
                &database
                    .get_setting("ui.theme")
                    .expect("read saved theme")
                    .unwrap()
            ),
            Ok(ThemePreference::Dark)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network_config(mode: ProxyMode, proxy: Option<&str>) -> crate::config::NetworkConfig {
        crate::config::NetworkConfig {
            proxy_mode: mode,
            proxy_mode_declared: true,
            proxy: proxy.map(str::to_owned),
            ..crate::config::NetworkConfig::default()
        }
    }

    fn settings_for(config: &crate::config::NetworkConfig) -> NetworkSettings {
        let diagnostics = config.diagnostics();
        NetworkSettings {
            proxy_mode: diagnostics.proxy_mode,
            proxy: String::new(),
            proxy_summary: diagnostics.proxy,
            proxy_configured: diagnostics.proxy_configured,
            proxy_active: diagnostics.proxy_active,
            supported_modes: vec![
                "system".to_owned(),
                "direct".to_owned(),
                "manual".to_owned(),
            ],
            system_proxy_note: system_proxy_note(),
            system_proxy_supported: system_proxy_supported(),
        }
    }

    #[test]
    fn the_settings_view_never_returns_the_stored_credential() {
        let config = network_config(
            ProxyMode::Manual,
            Some("http://alice:s3cret@proxy.example:8080"),
        );
        let settings = settings_for(&config);
        assert!(
            settings.proxy.is_empty(),
            "the stored value must never be returned to the frontend"
        );
        assert_eq!(
            settings.proxy_summary.as_deref(),
            Some("http://[REDACTED]@proxy.example:8080")
        );
        assert!(settings.proxy_configured);
        assert!(settings.proxy_active);
        let rendered = serde_json::to_string(&settings).expect("settings");
        assert!(
            !rendered.contains("s3cret"),
            "credentials leaked to the frontend: {rendered}"
        );
    }

    #[test]
    fn an_inactive_value_is_reported_as_stored_but_not_in_effect() {
        for mode in [ProxyMode::System, ProxyMode::Direct] {
            let config = network_config(mode, Some("http://alice:s3cret@proxy.example:8080"));
            let settings = settings_for(&config);
            assert!(
                settings.proxy_configured,
                "{mode:?} must still report a stored value"
            );
            assert!(
                !settings.proxy_active,
                "{mode:?} must not claim the value is in effect"
            );
            assert!(
                settings.proxy_summary.is_none(),
                "{mode:?} must not show a proxy summary"
            );
        }
    }

    #[test]
    fn the_settings_view_does_not_overstate_system_proxy_support() {
        // Only Windows resolves beyond the launching environment. Asserting the
        // invariant rather than a literal keeps the test meaningful on a host
        // that *can* support it, and a non-Windows build must still report false
        // so the page does not promise PAC/WPAD it cannot deliver.
        assert_eq!(
            system_proxy_supported(),
            cfg!(target_os = "windows"),
            "support must follow the platform, not a hard-coded value"
        );
        assert!(!system_proxy_note().is_empty());
        if !system_proxy_supported() {
            assert!(
                !system_proxy_note().contains("PAC"),
                "a platform without a native resolver must not mention PAC support"
            );
        }
    }

    #[test]
    fn a_route_is_described_without_exposing_credentials() {
        let decision = ProxyDecision::Proxy("http://alice:s3cret@proxy.example:8080".to_owned());
        assert_eq!(route_name(&decision), "proxy");
        let message = route_message(&decision);
        assert!(!message.contains("s3cret"));
        assert!(!message.contains("proxy.example"));

        assert_eq!(route_name(&ProxyDecision::Direct), "direct");
        assert_eq!(
            route_name(&ProxyDecision::ResolutionFailed("x".to_owned())),
            "failed"
        );
        assert_eq!(
            route_name(&ProxyDecision::Unsupported("y".to_owned())),
            "unsupported"
        );
    }

    #[test]
    fn a_failed_resolution_is_reported_as_a_failure_and_not_as_direct() {
        let message = route_message(&ProxyDecision::ResolutionFailed(
            "WPAD discovery timed out".to_owned(),
        ));
        assert!(message.contains("WPAD discovery timed out"));
        assert!(
            message.contains("No request was sent"),
            "a failure must state that nothing was sent: {message}"
        );
    }

    #[test]
    fn an_unsupported_mode_is_reported_as_unsupported() {
        let message = route_message(&ProxyDecision::Unsupported(
            "manual mode is selected but no proxy is configured".to_owned(),
        ));
        assert!(message.contains("cannot be applied"));
        assert!(!message.contains("s3cret"));
    }
}
