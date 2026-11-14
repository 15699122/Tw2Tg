mod archive;
mod aria2;
mod batch;
mod browser_pairing;
mod build_channel;
mod clock;
mod commands;
mod components;
mod config;
mod execution_spec;
mod executor;
mod logging;
mod platform;
mod portable;
mod production;
mod proxy;
mod runtime;
#[cfg(windows)]
mod system_proxy_resolver;
mod telegram_control;
mod telegram_send;
#[allow(dead_code)] // Wired after platform credential-provider injection.
mod telegram_worker;
pub(crate) mod transport;
mod websocket_transport;
#[cfg(windows)]
mod windows_credentials;
#[cfg(windows)]
mod windows_transport;

use aria2::{detect_aria2, download_aria2, list_aria2_releases, validate_aria2_path};
use runtime::RuntimeState;
use serde::Deserialize;
use std::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct ArchiveTweetRequest {
    pub tweet: xarchive_protocol::BrowserTweet,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    #[cfg(windows)]
    let context = {
        let mut context = context;
        // Tauri's Windows ICO decoder takes the first entry (16 px), unlike
        // Explorer's size-aware resource selection. Avoid enlarging that bitmap
        // for the taskbar and Alt+Tab; keep the multi-size ICO for EXE resources.
        context.set_default_window_icon(Some(tauri::include_image!("icons/128x128@2x.png")));
        context
    };
    let state = match RuntimeState::try_initialize() {
        Ok(state) => state,
        Err(error) => {
            eprintln!("Application startup refused: {error}");
            eprintln!(
                "Ask the trusted launcher administrator to repair the execution descriptor and delegated cgroup/Python/launcher provisioning, then restart. No uncontained fallback was started."
            );
            std::process::exit(2);
        }
    };
    #[cfg(windows)]
    let mut state = state;
    #[cfg(windows)]
    if let Err(error) =
        state.install_telegram_secrets(Box::new(windows_credentials::WindowsCredentialStore))
    {
        state.telegram_sender_error = Some(error);
    }
    let runtime_state = Mutex::new(state);
    let builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    let builder = builder.plugin(
        tauri_plugin_mcp_bridge::Builder::new()
            .bind_address("127.0.0.1")
            .build(),
    );

    #[cfg(feature = "wdio-e2e")]
    let builder = builder.plugin(tauri_plugin_wdio::init());

    builder
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(runtime_state)
        .invoke_handler(tauri::generate_handler![
            commands::telegram::get_telegram_task_state,
            commands::telegram::get_telegram_job_state,
            commands::telegram::confirm_telegram_resume,
            commands::telegram::review_telegram_unknown,
            commands::telegram::stop_telegram_sender,
            commands::telegram::save_telegram_settings,
            commands::telegram::cancel_telegram_send,
            commands::telegram::get_telegram_settings,
            commands::telegram::inspect_telegram_connection,
            commands::telegram::migrate_telegram_endpoint,
            commands::telegram::start_telegram_sender,
            commands::telegram::replace_telegram_credential,
            commands::telegram::delete_telegram_credential,
            commands::app::get_app_status,
            commands::archive::get_archive_root,
            commands::archive::get_portable_setup,
            commands::app::get_component_bootstrap_status,
            commands::archive::complete_download_setup,
            commands::archive::set_archive_directory,
            commands::settings::set_use_aria2,
            commands::settings::save_application_settings,
            commands::settings::get_output_settings,
            commands::settings::get_metadata_output_settings,
            commands::settings::save_metadata_output_settings,
            commands::records::export_configuration_exchange,
            commands::records::preview_configuration_exchange,
            commands::records::apply_configuration_exchange,
            commands::settings::save_output_settings,
            commands::settings::get_theme_preference,
            commands::settings::save_theme_preference,
            commands::settings::get_downloader_argument_settings,
            commands::settings::save_downloader_argument_settings,
            commands::app::get_runtime_health,
            commands::extension::get_extension_status,
            commands::batches::create_account_batch,
            commands::batches::list_account_batches,
            commands::batches::get_account_batch,
            commands::batches::list_account_batch_candidates,
            commands::batches::pause_account_batch,
            commands::batches::resume_account_batch,
            commands::batches::cancel_account_batch,
            commands::batches::retry_account_batch,
            commands::extension::register_native_host,
            commands::extension::unregister_native_host,
            commands::sidecar::start_sidecar,
            commands::sidecar::stop_sidecar,
            commands::jobs::list_jobs,
            commands::jobs::list_jobs_page,
            commands::jobs::get_job_history_index,
            commands::jobs::get_job_download_metrics,
            commands::jobs::get_job_metrics,
            commands::logs::log_frontend_event,
            commands::executor::submit_executor_job,
            commands::executor::query_executor_job,
            commands::executor::cancel_executor_job,
            commands::executor::shutdown_executor,
            commands::archive::open_archive_folder,
            commands::extension::open_extension_folder,
            commands::logs::read_application_logs,
            commands::logs::open_log_folder,
            detect_aria2,
            list_aria2_releases,
            download_aria2,
            validate_aria2_path,
            commands::settings::save_aria2_path,
            commands::archive::validate_gallery_dl_path,
            commands::archive::save_gallery_dl_path,
            commands::extension::import_extension_directory,
            commands::settings::get_sidecar_path,
            commands::archive::copy_text_to_clipboard,
            commands::settings::get_network_settings,
            commands::settings::save_network_settings,
            commands::settings::inspect_proxy_route,
            commands::settings::get_system_proxy_status,
            commands::records::preview_record_import,
            commands::records::accept_record_import,
            commands::records::export_record_exchange,
            commands::records::cancel_record_import,
            commands::records::cancel_configuration_exchange,
        ])
        .run(context)
        .expect("error while running XArchive desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::merge_browser_relationships;
    use crate::aria2::{selected_aria2_release, sha256_hex};
    use crate::commands::app::AppStatus;

    #[test]
    fn runtime_state_uses_a_dedicated_archive_directory() {
        let state = RuntimeState::initialize();
        assert!(state.database_error.is_none() || state.download_setup_required);
        assert!(state.executor.is_running());
    }

    #[test]
    fn initializes_database_before_download_setup() {
        let root = std::env::temp_dir().join(format!(
            "xarchive-test-db-init-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        let mut state = RuntimeState::initialize_at(root.clone());
        assert!(state.download_setup_required, "fresh root requires setup");
        assert!(
            state.database_ready,
            "database must initialize even before download setup"
        );
        assert!(state.database_error.is_none());
        let jobs = state
            .database
            .as_ref()
            .expect("database")
            .list_recent_jobs(20)
            .expect("job list works before download setup");
        assert!(jobs.is_empty());
        state
            .executor
            .shutdown_in_place()
            .expect("executor shutdown");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn app_status_reports_executor_lifecycle_without_changing_archive_state() {
        let state = RuntimeState::initialize();
        let status = AppStatus {
            app_name: "XArchive",
            app_version: env!("CARGO_PKG_VERSION"),
            sidecar: "not_configured".to_owned(),
            database: if state.database_ready {
                "ready".to_owned()
            } else {
                "error".to_owned()
            },
            platform: std::env::consts::OS,
            archive_root: state.download_root.display().to_string(),
            gallery_dl_path: Some(
                crate::portable::resolve_config_path(
                    &state.portable_root,
                    &state.config.sidecar.gallery_dl,
                )
                .display()
                .to_string(),
            ),
            database_error: state.database_error.clone(),
            sidecar_error: state.sidecar_error.clone(),
            executor: if state.executor.is_running() {
                "ready".to_owned()
            } else {
                "stopped".to_owned()
            },
            download_setup_required: state.download_setup_required,
            logs_root: state.logs_root.display().to_string(),
            logging_level: state.config.logging.effective_level().as_str().to_owned(),
            max_log_files: state.config.logging.max_files,
            use_aria2: state.config.download.use_aria2,
        };
        assert_eq!(status.executor, "ready");
        assert!(
            status.archive_root.ends_with("download")
                || status.archive_root.ends_with("Downloads/XArchive")
        );
    }

    #[test]
    fn parses_sidecar_args_as_json_without_splitting_paths() {
        let args =
            commands::parse_sidecar_args(r#"["-m","xarchive_downloader","C:\\Work Dir\\staging"]"#)
                .expect("valid sidecar args");
        assert_eq!(args[2], r#"C:\Work Dir\staging"#);
    }

    #[test]
    fn rejects_non_array_sidecar_args() {
        let error = commands::parse_sidecar_args("--verbose").expect_err("invalid args");
        assert!(error.contains("JSON string array"));
    }

    #[test]
    fn exposes_verified_aria2_releases_and_sha256() {
        let releases = list_aria2_releases();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].version, "1.37.0");
        assert_eq!(
            releases[0].sha256,
            "67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288"
        );
        assert_eq!(
            sha256_hex(b"xarchive"),
            "5bb7d0a5d35ed6fb314afcc4931bc18b03262864f955ad678fb662e2239214fa"
        );
    }

    #[test]
    fn rejects_unsupported_aria2_versions() {
        assert!(selected_aria2_release("9.99.9").is_err());
    }

    #[test]
    fn latest_aria2_release_is_the_newest_allowlist_entry() {
        let latest = crate::aria2::latest_aria2_release();
        assert_eq!(latest.version, "1.37.0");
        let releases = list_aria2_releases();
        assert_eq!(
            releases.first().map(|release| release.version),
            Some(latest.version)
        );
    }

    fn browser_tweet(tweet_type: &str) -> xarchive_protocol::BrowserTweet {
        xarchive_protocol::BrowserTweet {
            tweet_id: "123".into(),
            url: "https://x.com/alice/status/123".into(),
            username: Some("alice".into()),
            display_name: Some("Alice".into()),
            text: Some("quoting".into()),
            created_at: Some("2026-09-12T00:00:00Z".into()),
            tweet_type: tweet_type.into(),
            reply_to: Some("111".into()),
            quoted_tweet: Some(
                xarchive_protocol::BrowserTweet {
                    tweet_id: "987".into(),
                    url: "https://x.com/bob/status/987".into(),
                    username: Some("bob".into()),
                    display_name: Some("Bob".into()),
                    text: Some("original".into()),
                    created_at: Some("2026-09-11T00:00:00Z".into()),
                    tweet_type: "post".into(),
                    reply_to: None,
                    quoted_tweet: None,
                }
                .into(),
            ),
        }
    }

    #[test]
    fn merges_browser_relationships_into_sidecar_metadata_gaps() {
        let mut metadata = serde_json::json!({
            "tweet_id": "123",
            "url": "https://x.com/alice/status/123",
            "tweet_type": "post",
            "text": "quoting"
        });
        let tweet = browser_tweet("quote");
        merge_browser_relationships(&mut metadata, &tweet);
        assert_eq!(metadata["reply_to"], "111");
        assert_eq!(metadata["quoted_tweet"]["tweet_id"], "987");
        assert_eq!(metadata["quoted_tweet"]["username"], "bob");
        assert_eq!(metadata["tweet_type"], "quote");
    }

    #[test]
    fn preserves_sidecar_provided_relationship_data() {
        let mut metadata = serde_json::json!({
            "tweet_id": "123",
            "reply_to": "222",
            "tweet_type": "quote",
            "quoted_tweet": {"tweet_id": "555", "url": "https://x.com/carol/status/555"}
        });
        let tweet = browser_tweet("quote");
        merge_browser_relationships(&mut metadata, &tweet);
        assert_eq!(metadata["reply_to"], "222");
        assert_eq!(metadata["quoted_tweet"]["tweet_id"], "555");
        assert_eq!(metadata["tweet_type"], "quote");
    }

    #[test]
    fn ignores_non_object_sidecar_metadata() {
        let mut metadata = serde_json::Value::Null;
        let tweet = browser_tweet("quote");
        merge_browser_relationships(&mut metadata, &tweet);
        assert_eq!(metadata, serde_json::Value::Null);
    }
}
