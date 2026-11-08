//! Sidecar lifecycle commands: start and stop the protocol-v2 worker.
//!
//! Starting spawns the supervised worker and performs the v2 handshake; stopping
//! sends `shutdown`, closes stdin, waits briefly and then escalates. Command
//! names, argument shapes and every returned status string are the registered IPC
//! contract and must not change here.

use std::sync::Mutex;
use std::time::Duration;

use tauri::State;
use xarchive_sidecar_supervisor::SidecarSupervisor;

use crate::RuntimeState;
use crate::commands::parse_sidecar_args;

fn sidecar_configuration() -> Result<(String, Vec<String>), String> {
    if let Ok(program) = std::env::var("XARCHIVE_SIDECAR_PROGRAM") {
        if program.trim().is_empty() {
            return Err("XARCHIVE_SIDECAR_PROGRAM is empty".to_owned());
        }
        let args = parse_sidecar_args(&std::env::var("XARCHIVE_SIDECAR_ARGS").unwrap_or_default())?;
        return Ok((program, args));
    }
    let paths = crate::portable::PortablePaths::from_root(crate::portable::portable_root());
    let (config, _) = crate::config::AppConfig::load(&paths);
    let worker = crate::portable::resolve_config_path(&paths.root, &config.sidecar.worker);
    let gallery = crate::portable::resolve_config_path(&paths.root, &config.sidecar.gallery_dl);
    let (program, args) = if worker.is_file() {
        (
            worker.display().to_string(),
            vec!["--gallery-dl".to_owned(), gallery.display().to_string()],
        )
    } else {
        return Err("XArchive Sidecar worker was not found".to_owned());
    };
    if program.trim().is_empty() {
        return Err("XARCHIVE_SIDECAR_PROGRAM is empty".to_owned());
    }
    Ok((program, args))
}

#[tauri::command]
pub(crate) fn start_sidecar(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let (program, mut args) = sidecar_configuration()?;
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    if state.sidecar.is_some() {
        return Ok("ready".to_owned());
    }
    state.sidecar_error = None;
    args.extend(state.config.network.sidecar_args());
    let env = state.config.network.sidecar_env();
    // A `Direct` mode also has to drop the proxy variables the launching shell
    // exported, otherwise the Sidecar re-introduces the proxy the user turned
    // off through gallery-dl.
    let env_remove = state.config.network.sidecar_env_remove();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    state.debug(
        "sidecar",
        &format!(
            "starting program={} args={} proxy_mode={} env={} env_remove={:?}",
            program,
            arg_refs.join(" "),
            state.config.network.proxy_mode.as_str(),
            env.len(),
            env_remove,
        ),
    );
    match SidecarSupervisor::spawn_ready_v2_with_environment(
        &program,
        &arg_refs,
        &xarchive_sidecar_supervisor::ChildProcessEnvironment::new(env, env_remove),
        Duration::from_secs(5),
    ) {
        Ok(supervisor) => {
            state.sidecar = Some(supervisor);
            state.debug("sidecar", "handshake ready");
            Ok("ready".to_owned())
        }
        Err(error) => {
            let message = error.to_string();
            state.sidecar_error = Some(message.clone());
            state.error("sidecar", &format!("start failed: {message}"));
            Err(message)
        }
    }
}

#[tauri::command]
pub(crate) fn stop_sidecar(state: State<'_, Mutex<RuntimeState>>) -> Result<String, String> {
    let mut state = state
        .lock()
        .map_err(|_| "runtime state lock poisoned".to_owned())?;
    if let Some(mut supervisor) = state.sidecar.take() {
        let _ = supervisor.send_v2_shutdown("desktop-shutdown");
        supervisor.close_stdin();
        let shutdown_result = match supervisor.wait_for_exit(Duration::from_secs(1)) {
            Ok(true) => Ok(()),
            Ok(false) => supervisor.shutdown(),
            Err(wait_error) => match supervisor.shutdown() {
                Ok(()) => Err(wait_error),
                Err(shutdown_error) => Err(shutdown_error),
            },
        };
        if let Err(error) = shutdown_result {
            let message = error.to_string();
            state.sidecar = Some(supervisor);
            state.sidecar_error = Some(message.clone());
            state.error("sidecar", &message);
            return Err(message);
        }
        state.sidecar_error = None;
        Ok("stopped".to_owned())
    } else {
        Ok("stopped".to_owned())
    }
}
