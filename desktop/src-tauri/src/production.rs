//! U7 production archive orchestration.
//!
//! This module owns the v2 extraction -> aria2 transfer boundary. Legacy v1
//! extraction/download code and the synchronous archive command were removed in
//! U8; `archive.rs` owns the executor context, staging verification, and the
//! `ArchiveService` commit.

#[cfg(test)]
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::time::Duration;
use xarchive_download::Aria2Supervisor;
#[cfg(test)]
use xarchive_download::{
    MediaTransferPlan, TransferFailure, TransferFailureCode, media_transfer_plan,
};
#[cfg(test)]
use xarchive_protocol::ExtractionMediaType;
#[cfg(test)]
use xarchive_protocol::ExtractionResult;
use xarchive_sidecar_supervisor::SidecarSupervisor;

use crate::archive::{SidecarArchiveResult, SidecarDownloadRequest};
use crate::executor::CancellationToken;

mod discovery;
mod result_mapping;
mod sidecar_operations;
mod tool_compatibility;
mod transfer;

// Compatibility re-exports: the tool-compatibility check moved to
// `production/tool_compatibility.rs`, but the parent and its tests keep calling
// `revalidate_tool_compatibility` / `resolved_tool_version` unqualified.
#[allow(unused_imports)]
pub(crate) use discovery::{DiscoveryOutcome, DiscoveryRequest, execute_v2_discovery};
#[allow(unused_imports)]
pub(crate) use result_mapping::{execute_gallery_dl_archive, media_type_name};
#[allow(unused_imports)]
pub(crate) use sidecar_operations::{download_v2, extract_v2, verify_downloaded_files};
#[allow(unused_imports)]
use transfer::extraction_to_metadata;
// Re-exported at the original `production::*` paths. The transfer fixture is
// `include!`-expanded into this module, so it reaches these names unqualified;
// that is why the whole surface is bridged rather than narrowed per caller.
#[allow(unused_imports)]
use tool_compatibility::executable_version;
pub(crate) use tool_compatibility::{resolved_tool_version, revalidate_tool_compatibility};
#[allow(unused_imports)]
pub(crate) use transfer::{
    TransferAttemptFailure, aria2_config, ensure_same_media_collection, execute_aria2_archive,
    format_transfer_failure, retry_retained_drain, retry_retained_drain_with,
    run_transfer_with_supervisor, transfer_failure_allows_refresh, transfer_failure_enters_refresh,
    transfer_once, transfer_with_one_refresh,
};

pub(crate) fn execute_v2_archive(
    supervisor: &mut SidecarSupervisor,
    request: &SidecarDownloadRequest,
    cancellation: &CancellationToken,
    aria2_program: Option<&str>,
    use_aria2: bool,
    network: &crate::executor::ExecutorNetworkConfig,
    #[cfg(target_os = "linux")] containment: Option<&crate::executor::AttemptWriterDomain>,
) -> Result<SidecarArchiveResult, (String, Option<Aria2Supervisor>)> {
    revalidate_tool_compatibility(request, aria2_program, use_aria2)
        .map_err(|message| (message, None))?;
    if use_aria2 {
        return execute_aria2_archive(
            supervisor,
            request,
            cancellation,
            aria2_program,
            network,
            #[cfg(target_os = "linux")]
            containment,
        );
    }
    if aria2_program.is_some() {
        eprintln!(
            "warning: aria2 path is configured but optional downloads are off; aria2 will not start"
        );
    }
    execute_gallery_dl_archive(supervisor, request, cancellation, network)
        .map_err(|message| (message, None))
}

#[cfg(test)]
mod e3_argument_tests {
    use super::*;

    #[test]
    fn transfer_containment_failure_never_falls_back_to_plain_spawn() {
        let domain = crate::executor::AttemptWriterDomain {
            registration: None,
            provisioning: crate::executor::LinuxExecutionContainment {
                delegated_parent: std::path::PathBuf::from("/tmp/not-a-cgroup"),
                python: std::path::PathBuf::from("/usr/bin/python3"),
                launcher: std::path::PathBuf::from("/missing-launcher.py"),
            },
            record_root: std::env::temp_dir().join("tw2tg-e3-record-root"),
            identity: xarchive_workflow::linux_containment::AttemptIdentity::new(
                "/ws/e3", "job-e3", 1,
            )
            .unwrap(),
        };
        let failure = transfer_once(
            &MediaTransferPlan::default(),
            &CancellationToken::default(),
            Some("/usr/bin/false"),
            &crate::executor::ExecutorNetworkConfig::default(),
            &[],
            Some(&domain),
            0,
        )
        .expect_err("invalid containment must reject startup");
        assert_eq!(failure.diagnostic.code, TransferFailureCode::Failed);
        assert!(
            failure
                .diagnostic
                .message
                .contains("invalid dedicated cgroup path")
        );
        assert!(failure.retained_supervisor.is_none());
    }

    #[test]
    fn gallery_dl_snapshot_rejects_before_any_download_process_is_spawned() {
        let request = SidecarDownloadRequest {
            job_id: "job-1".into(),
            request_id: "request-1".into(),
            url: "https://x.com/status/1".into(),
            staging_dir: std::env::temp_dir(),
            browser: None,
            profile: None,
            downloader_arguments: crate::executor::DownloaderArgumentSnapshot {
                gallery_dl: vec!["--unreviewed-option".into()],
                aria2: Vec::new(),
                policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
                tool_compatibility: crate::executor::ToolCompatibility {
                    aria2: None,
                    gallery_dl: Some("1.0.0".into()),
                },
            },
            gallery_dl_program: None,
        };
        assert!(revalidate_tool_compatibility(&request, None, false).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn version_probe_uses_resolved_executable_and_parses_reported_version() {
        use std::os::unix::fs::PermissionsExt;
        let directory =
            std::env::temp_dir().join(format!("xarchive-version-probe-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temp directory");
        let executable = directory.join("fake-downloader");
        std::fs::write(&executable, "#!/bin/sh\nprintf 'fake 7.4.2\\n'\n").expect("write fake");
        let mut permissions = std::fs::metadata(&executable)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&executable, permissions).expect("chmod");
        assert_eq!(
            resolved_tool_version(executable.to_str().unwrap(), &["--version"]).unwrap(),
            "7.4.2"
        );
        std::fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn version_probe_failure_is_diagnostic_but_does_not_echo_arguments() {
        assert_eq!(
            resolved_tool_version("/missing/fake", &["--version"]).unwrap_err(),
            "downloader version probe failed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn empty_snapshot_does_not_probe_or_spawn_gallery_dl() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "xarchive-tool-mismatch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("create fixture directory");
        let marker = directory.join("download-started");
        let executable = directory.join("fake-gallery-dl");
        std::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nprintf 'invoked:%s\\n' \"$1\" >> '{}'\nif [ \"$1\" = \"--version\" ]; then echo 'fake 7.4.2'; exit 0; fi\ntouch '{}'\n",
                marker.display(),
                marker.display()
            ),
        )
        .expect("write fake executable");
        let mut permissions = std::fs::metadata(&executable)
            .expect("fake executable metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&executable, permissions).expect("make executable");
        let request = SidecarDownloadRequest {
            job_id: "job-1".into(),
            request_id: "request-1".into(),
            url: "https://x.com/status/1".into(),
            staging_dir: directory.clone(),
            browser: None,
            profile: None,
            downloader_arguments: crate::executor::DownloaderArgumentSnapshot {
                gallery_dl: Vec::new(),
                aria2: Vec::new(),
                policy_version: xarchive_download::downloader_args::ARGUMENT_POLICY_VERSION,
                tool_compatibility: crate::executor::ToolCompatibility {
                    aria2: None,
                    gallery_dl: None,
                },
            },
            gallery_dl_program: Some(executable.to_string_lossy().into_owned()),
        };

        revalidate_tool_compatibility(&request, None, false)
            .expect("empty gallery-dl snapshot requires no executable probe");
        assert!(
            !marker.exists(),
            "an empty snapshot must not probe the tool"
        );
        std::fs::remove_dir_all(directory).expect("remove fixture directory");
    }
}

#[cfg(all(test, target_os = "linux"))]
include!("production_transfer_fixture.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use xarchive_core::ProxyMode;
    use xarchive_protocol::{DownloadResult, DownloadedMediaFile, ExtractionMediaItem};

    fn staging_extraction() -> ExtractionResult {
        ExtractionResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            tweet_type: "post".to_owned(),
            text: None,
            username: None,
            display_name: None,
            created_at: None,
            user_id: None,
            reply_to: None,
            quoted_tweet: None,
            media: vec![ExtractionMediaItem {
                index: 1,
                media_id: Some("media-01".to_owned()),
                media_type: ExtractionMediaType::Photo,
                url: "https://cdn.example/1.jpg".to_owned(),
                filename: "01.jpg".to_owned(),
                mime_type: Some("image/jpeg".to_owned()),
            }],
            request_headers: Vec::new(),
        }
    }

    fn staging_download(relative_path: &str, size_bytes: u64) -> DownloadResult {
        DownloadResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            media: vec![DownloadedMediaFile {
                index: 1,
                media_id: Some("media-01".to_owned()),
                media_type: ExtractionMediaType::Photo,
                filename: "01.jpg".to_owned(),
                relative_path: relative_path.to_owned(),
                size_bytes,
                mime_type: Some("image/jpeg".to_owned()),
            }],
            result: None,
        }
    }

    /// Per-test root directory. Escaping-path assertions write deliberately
    /// outside `staging`, so the parent must be exclusive to this test run
    /// instead of the shared system temporary directory.
    fn temp_case_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xarchive-download-{}-{}-{name}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        ));
        std::fs::create_dir_all(&root).expect("case root");
        root
    }

    fn temp_staging(name: &str) -> PathBuf {
        let staging = temp_case_root(name).join("staging");
        std::fs::create_dir_all(&staging).expect("staging directory");
        staging
    }

    fn remove_case_root(staging: &Path) {
        let root = staging.parent().unwrap_or(staging);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn verifies_reported_download_files_against_the_staging_directory() {
        let staging = temp_staging("verify");
        std::fs::write(staging.join("01.jpg"), b"jpeg-bytes").expect("media");
        let download = staging_download("01.jpg", b"jpeg-bytes".len() as u64);
        let files = verify_downloaded_files(&download, &staging_extraction(), &staging)
            .expect("verified files");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "01.jpg");
        assert_eq!(files[0].size_bytes, 10);
        assert_eq!(files[0].media_type, "photo");
        remove_case_root(&staging);
    }

    #[test]
    fn rejects_escaping_missing_mismatched_and_empty_downloads() {
        let staging = temp_staging("reject");
        let extraction = staging_extraction();

        // A reported path that leaves staging is refused even when the file
        // exists outside it. Protocol validation already rejects this shape;
        // this is the defense-in-depth check at the filesystem boundary.
        std::fs::write(staging.join("..").join("outside.jpg"), b"jpeg-bytes").ok();
        let escaping = staging_download("../outside.jpg", 10);
        let error =
            verify_downloaded_files(&escaping, &extraction, &staging).expect_err("escaping path");
        assert!(
            error.contains("escaped staging directory") || error.contains("path is unsafe"),
            "unexpected error: {error}"
        );

        // A missing file never becomes an archived media record.
        let missing = staging_download("missing.jpg", 10);
        assert!(
            verify_downloaded_files(&missing, &extraction, &staging)
                .expect_err("missing file")
                .contains("download output is unavailable")
        );

        // A worker-reported size that disagrees with disk is refused.
        std::fs::write(staging.join("01.jpg"), b"jpeg-bytes").expect("media");
        let mismatch = staging_download("01.jpg", 999);
        assert!(
            verify_downloaded_files(&mismatch, &extraction, &staging)
                .expect_err("size mismatch")
                .contains("download size mismatch")
        );

        // Media in extraction but no file at all is refused.
        let empty = DownloadResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/alice/status/123".to_owned(),
            media: Vec::new(),
            result: None,
        };
        assert_eq!(
            verify_downloaded_files(&empty, &extraction, &staging)
                .expect_err("empty download")
                .as_str(),
            "download completed without any media files"
        );
        remove_case_root(&staging);
    }

    #[test]
    fn production_aria2_configuration_generates_a_fresh_secret_without_environment_setup() {
        let network =
            crate::executor::ExecutorNetworkConfig::from_seconds(60, 30, 5, 10, 2, 30, 60)
                .with_proxy(
                    ProxyMode::Manual,
                    Some("http://alice:s3cret@proxy.example:8080".to_owned()),
                );
        let config = aria2_config(Some("aria2c"), &network, &[]).expect("aria2 config");
        assert_eq!(config.rpc_secret.len(), 64);
        assert!(
            config
                .rpc_secret
                .chars()
                .all(|value| value.is_ascii_hexdigit())
        );
        assert!(!format!("{config:?}").contains("s3cret"));
    }

    #[test]
    fn aria2_config_forwards_validated_user_args_into_the_spawned_config() {
        // Batch E: the aria2 path must forward the task's validated aria2
        // options into the spawned aria2 configuration. These were allowlist /
        // value vetted by the Rust argument-policy authority when the snapshot
        // was accepted; the config carries them verbatim for command_args to
        // append after the application-owned options.
        let network =
            crate::executor::ExecutorNetworkConfig::from_seconds(60, 30, 5, 10, 2, 30, 60);
        let user_args = vec!["--max-tries".to_owned(), "7".to_owned()];
        let config = aria2_config(Some("aria2c"), &network, &user_args)
            .expect("aria2 config with user args");
        assert_eq!(config.user_args, user_args);
        // A protected option never reaches here; it was rejected upstream. The
        // argv append itself (application options then user options last) is
        // unit-tested in the download crate where command_args is visible.
        assert!(
            !config
                .user_args
                .iter()
                .any(|a| a.contains("--out") || a.contains("--dir"))
        );
    }

    #[test]
    fn the_aria2_configuration_carries_the_selected_mode() {
        let base =
            || crate::executor::ExecutorNetworkConfig::from_seconds(60, 30, 5, 10, 2, 30, 60);
        let manual = base().with_proxy(
            ProxyMode::Manual,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        let manual_config = aria2_config(Some("aria2c"), &manual, &[]).expect("manual config");
        assert_eq!(manual_config.proxy_mode, ProxyMode::Manual);
        assert!(
            manual_config
                .environment()
                .iter()
                .any(|(key, value)| key == "all_proxy" && value.contains("s3cret"))
        );
        assert!(manual_config.environment_remove().is_empty());

        let direct = base().with_proxy(
            ProxyMode::Direct,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        let direct_config = aria2_config(Some("aria2c"), &direct, &[]).expect("direct config");
        assert_eq!(direct_config.proxy_mode, ProxyMode::Direct);
        assert!(
            direct_config.environment().is_empty(),
            "Direct must not hand the stored proxy to aria2"
        );
        assert!(
            direct_config
                .environment_remove()
                .iter()
                .any(|key| key == "http_proxy")
        );

        let system = base().with_proxy(
            ProxyMode::System,
            Some("http://alice:s3cret@proxy.example:8080".to_owned()),
        );
        let system_config = aria2_config(Some("aria2c"), &system, &[]).expect("system config");
        assert_eq!(system_config.proxy_mode, ProxyMode::System);
        assert!(
            system_config.environment().is_empty(),
            "System must not pin the stored manual value on aria2"
        );
    }

    #[test]
    fn converts_extraction_to_durable_metadata_without_headers_or_urls() {
        let result = ExtractionResult {
            tweet_id: "123".to_owned(),
            url: "https://x.com/a/status/123".to_owned(),
            tweet_type: "post".to_owned(),
            text: Some("hello".to_owned()),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            created_at: None,
            user_id: Some("9001".to_owned()),
            reply_to: Some("111".to_owned()),
            quoted_tweet: Some(xarchive_protocol::ExtractionQuotedTweet {
                tweet_id: "987".to_owned(),
                url: "https://x.com/bob/status/987".to_owned(),
                username: Some("bob".to_owned()),
                display_name: Some("Bob".to_owned()),
                user_id: Some("9002".to_owned()),
                text: Some("original".to_owned()),
                created_at: None,
                tweet_type: Some("post".to_owned()),
            }),
            media: vec![],
            request_headers: vec![xarchive_protocol::ExtractionRequestHeader {
                name: "Referer".to_owned(),
                value: "https://x.com/".to_owned(),
            }],
        };
        let metadata = extraction_to_metadata(&result);
        assert_eq!(metadata["tweet_id"], "123");
        assert_eq!(metadata["user_id"], "9001");
        assert_eq!(metadata["reply_to"], "111");
        assert_eq!(metadata["quoted_tweet"]["tweet_id"], "987");
        assert_eq!(metadata["quoted_tweet"]["user_id"], "9002");
        assert!(metadata.get("request_headers").is_none());
        assert!(metadata.get("signed_url").is_none());
    }

    #[test]
    fn rejects_changed_media_identity_or_filename_sets() {
        let before = MediaTransferPlan {
            items: vec![xarchive_download::TransferPlanItem {
                media_id: "m1".into(),
                url: "https://cdn.example/1.jpg".into(),
                filename: "01.jpg".into(),
                directory: "/tmp/staging".into(),
                headers: vec![],
                media_type: "photo".into(),
                mime_type: Some("image/jpeg".into()),
            }],
        };
        let mut after = before.clone();
        after.items[0].filename = "02.jpg".into();
        assert_eq!(
            ensure_same_media_collection(&before, &after),
            Err(xarchive_download::EXTRACTION_RESULT_CHANGED.to_owned())
        );
    }

    #[test]
    fn transfer_failure_enters_refresh_expired_url_without_retained() {
        let failure = TransferAttemptFailure {
            diagnostic: TransferFailure {
                media_id: Some("media-1".to_owned()),
                code: TransferFailureCode::ExpiredUrl,
                message: "url expired".to_owned(),
                aria2_error_code: None,
            },
            retained_supervisor: None,
        };
        assert!(transfer_failure_enters_refresh(&failure));
    }

    /// Non-expired failures never enter refresh, regardless of retained state.
    #[test]
    fn transfer_failure_enters_refresh_other_code() {
        let failure = TransferAttemptFailure {
            diagnostic: TransferFailure {
                media_id: None,
                code: TransferFailureCode::Failed,
                message: "generic failure".to_owned(),
                aria2_error_code: None,
            },
            retained_supervisor: None,
        };
        assert!(!transfer_failure_enters_refresh(&failure));
    }
    /// `retry_retained_drain` with no retained owner returns `Ok(false)` and
    /// does not attempt cleanup.
    #[test]
    fn retry_retained_drain_no_owner_returns_false() {
        let mut failure = TransferAttemptFailure {
            diagnostic: TransferFailure {
                media_id: None,
                code: TransferFailureCode::Failed,
                message: "generic failure".to_owned(),
                aria2_error_code: None,
            },
            retained_supervisor: None,
        };
        let result = retry_retained_drain(&mut failure);
        assert!(result.is_ok());
        assert!(!result.unwrap());
        assert!(failure.retained_supervisor.is_none());
    }

    #[test]
    fn retained_drain_failure_preserves_owner_and_blocks_refresh() {
        let mut owner = Some(String::from("aria2-owner-0"));
        let mut shutdown_calls = 0;
        let result = retry_retained_drain_with(&mut owner, |owner| {
            shutdown_calls += 1;
            assert_eq!(owner, "aria2-owner-0");
            Err::<(), _>("injected drain failure")
        });

        assert_eq!(result, Err("injected drain failure"));
        assert_eq!(shutdown_calls, 1);
        assert_eq!(owner.as_deref(), Some("aria2-owner-0"));
        assert!(
            !transfer_failure_allows_refresh(TransferFailureCode::ExpiredUrl, owner.is_some()),
            "refresh policy must reject an expired URL while its owner is retained"
        );
    }

    #[test]
    fn retained_drain_success_releases_owner_only_after_shutdown() {
        let mut owner = Some(String::from("aria2-owner-0"));
        let mut shutdown_calls = 0;
        let result = retry_retained_drain_with(&mut owner, |owner| {
            shutdown_calls += 1;
            assert_eq!(owner, "aria2-owner-0");
            Ok::<(), &str>(())
        });

        assert_eq!(result, Ok(true));
        assert_eq!(shutdown_calls, 1);
        assert!(owner.is_none());
    }

    #[test]
    fn expired_url_refresh_policy_rejects_retained_owner() {
        assert!(!transfer_failure_allows_refresh(
            TransferFailureCode::ExpiredUrl,
            true
        ));
    }
}
