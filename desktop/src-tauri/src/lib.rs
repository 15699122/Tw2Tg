mod archive;
mod aria2;
mod batch;
mod commands;
mod components;
mod config;
mod executor;
mod logging;
mod platform;
mod portable;
mod production;
mod runtime;
pub(crate) mod transport;
#[cfg(windows)]
mod windows_transport;

use aria2::{detect_aria2, download_aria2, list_aria2_releases, validate_aria2_path};
use commands::{
    cancel_account_batch, cancel_executor_job, complete_download_setup, copy_text_to_clipboard,
    create_account_batch, get_account_batch, get_app_status, get_archive_root,
    get_component_bootstrap_status, get_extension_status, get_job_metrics, get_portable_setup,
    get_runtime_health, get_sidecar_path, import_extension_directory,
    list_account_batch_candidates, list_account_batches, list_jobs, log_frontend_event,
    open_archive_folder, open_extension_folder, open_log_folder, pause_account_batch,
    query_executor_job, read_application_logs, register_native_host, resume_account_batch,
    retry_account_batch, save_application_settings, save_aria2_path, save_gallery_dl_path,
    shutdown_executor, start_sidecar, stop_sidecar, submit_executor_job, unregister_native_host,
    validate_gallery_dl_path,
};
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
    let runtime_state = Mutex::new(RuntimeState::initialize());
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
            get_app_status,
            get_archive_root,
            get_portable_setup,
            get_component_bootstrap_status,
            complete_download_setup,
            save_application_settings,
            get_runtime_health,
            get_extension_status,
            create_account_batch,
            list_account_batches,
            get_account_batch,
            list_account_batch_candidates,
            pause_account_batch,
            resume_account_batch,
            cancel_account_batch,
            retry_account_batch,
            register_native_host,
            unregister_native_host,
            start_sidecar,
            stop_sidecar,
            list_jobs,
            get_job_metrics,
            log_frontend_event,
            submit_executor_job,
            query_executor_job,
            cancel_executor_job,
            shutdown_executor,
            open_archive_folder,
            open_extension_folder,
            read_application_logs,
            open_log_folder,
            detect_aria2,
            list_aria2_releases,
            download_aria2,
            validate_aria2_path,
            save_aria2_path,
            validate_gallery_dl_path,
            save_gallery_dl_path,
            import_extension_directory,
            get_sidecar_path,
            copy_text_to_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running XArchive desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::merge_browser_relationships;
    use crate::aria2::{selected_aria2_release, sha256_hex};
    use crate::commands::AppStatus;

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
            database_error: state.database_error.clone(),
            sidecar_error: state.sidecar_error.clone(),
            executor: if state.executor.is_running() {
                "ready".to_owned()
            } else {
                "stopped".to_owned()
            },
            download_setup_required: state.download_setup_required,
            logs_root: state.logs_root.display().to_string(),
            logging_level: state.config.logging.level.as_str().to_owned(),
            max_log_files: state.config.logging.max_files,
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
