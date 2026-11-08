//! Production execution factory: immutable spec decoding, attempt-owned
//! execution creation, and the fail-closed unconfigured fallback.
//!
//! One responsibility: turning a persisted Job snapshot into an executable
//! `JobExecution` bound to the claimed attempt. It performs no admission,
//! claim, drain, authorization or finalization, so moving it cannot reorder
//! ADR-015.

use xarchive_core::ChildEnvironment;
use xarchive_sidecar_supervisor::{ChildProcessEnvironment, SidecarSupervisor};
use xarchive_storage::Database;
use xarchive_workflow::AttemptIdentity;

use crate::execution_spec::{DecodedExecutionSpec, decode_execution_spec};
use crate::executor::validate_execution_spec_identity;

use crate::executor::{
    AttemptCreationError, ExecutorConfig, ExecutorError, JobExecution, JobExecutionFactory,
    JobSnapshot,
};
#[cfg(target_os = "linux")]
use crate::executor::{
    AttemptWriterDomain, LinuxExecutionContainment, attempt_writer_base, containment_record_root,
    recover_and_create_writer_group, recover_predecessor_writer_domains,
};

pub struct ProductionExecutionFactory {
    config: ExecutorConfig,
    #[cfg(target_os = "linux")]
    containment: Option<LinuxExecutionContainment>,
}

fn decode_job_execution_spec(
    schema_version: u32,
    request_json: &str,
    job_tweet_id: &str,
) -> Result<DecodedExecutionSpec, ExecutorError> {
    let decoded = decode_execution_spec(schema_version, request_json)
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
    validate_execution_spec_identity(decoded, job_tweet_id)
}

impl ProductionExecutionFactory {
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            config,
            #[cfg(target_os = "linux")]
            containment: None,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn with_linux_containment(mut self, containment: LinuxExecutionContainment) -> Self {
        self.containment = Some(containment);
        self
    }
}

impl JobExecutionFactory for ProductionExecutionFactory {
    fn create(
        &self,
        job_id: &str,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        let _ = (job_id, snapshot);
        Err(ExecutorError::Execution {
            error_code: "ATTEMPT_EXECUTION_REQUIRED".to_owned(),
            error_message: "production archive execution requires a claimed attempt identity"
                .to_owned(),
            persistence_already_updated: false,
        })
    }

    fn create_registered_attempt(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
        registration: xarchive_workflow::WriterRegistration,
    ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
        if identity.job_id() != snapshot.job_id {
            return Err(ExecutorError::Persistence("attempt/job mismatch".into()).into());
        }
        self.create_attempt_execution(identity, snapshot, Some(registration))
    }

    fn create_for_attempt(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
        if identity.job_id() != snapshot.job_id.as_str() {
            return Err(ExecutorError::Execution {
                error_code: "ATTEMPT_JOB_MISMATCH".to_owned(),
                error_message: "claimed attempt does not match the execution Job".to_owned(),
                persistence_already_updated: false,
            }
            .into());
        }
        if identity.attempt() == 0 {
            return Err(ExecutorError::Execution {
                error_code: "INVALID_ATTEMPT_IDENTITY".to_owned(),
                error_message: "archive attempt number must be positive".to_owned(),
                persistence_already_updated: false,
            }
            .into());
        }
        self.create_attempt_execution(identity, snapshot, None)
    }
}

impl ProductionExecutionFactory {
    fn create_attempt_execution(
        &self,
        identity: &AttemptIdentity,
        snapshot: &JobSnapshot,
        registration: Option<xarchive_workflow::WriterRegistration>,
    ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
        let job_id = identity.job_id();
        let database = Database::open(&self.config.database_path)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let (schema_version, request_id, request_json) = database
            .archive_job_request(job_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?
            .ok_or_else(|| {
                ExecutorError::Persistence("archive execution spec is missing".to_owned())
            })?;
        let decoded = decode_job_execution_spec(schema_version, &request_json, &snapshot.tweet_id)?;
        let request = decoded.request;
        let output_settings = decoded.output_settings;
        let tweet_row_id = database
            .tweet_row_id(&request.tweet.tweet_id)
            .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let files = xarchive_storage::FileStore::with_staging_root(
            self.config.archive_root.clone(),
            self.config.staging_root.clone(),
        )
        .map_err(|error| ExecutorError::Persistence(error.to_string()))?;
        let program =
            self.config
                .sidecar_program
                .as_deref()
                .ok_or_else(|| ExecutorError::Execution {
                    error_code: "SIDECAR_NOT_CONFIGURED".to_owned(),
                    error_message: "XARCHIVE_SIDECAR_PROGRAM is not configured".to_owned(),
                    persistence_already_updated: false,
                })?;
        let args = self
            .config
            .sidecar_args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        // The Sidecar is told the mode as well as the value, and a `Direct`
        // mode additionally strips the proxy variables it would otherwise
        // inherit from the launching shell.
        //
        // `System` is only safe to hand over as an inherited environment when
        // the platform policy has one answer for every host. A PAC or WPAD
        // policy does not, and the child would silently bypass it, so the job
        // is refused here instead of running unprotected.
        let network = crate::proxy::ProxyHttpClient::new(
            self.config.network.proxy_mode,
            self.config.network.proxy.clone(),
            self.config.network.transfer_timeout,
            crate::proxy::platform_resolver(),
        );
        network
            .require_child_routable(crate::proxy::SIDECAR_PROXY_PROBE_URL, "the Sidecar worker")
            .map_err(|error| ExecutorError::Execution {
                error_code: "PROXY_POLICY_NOT_APPLICABLE".to_owned(),
                error_message: error,
                persistence_already_updated: false,
            })?;
        let environment = ChildEnvironment::for_mode(
            self.config.network.proxy_mode,
            self.config.network.proxy.clone(),
        );
        let mut env = vec![(
            "XARCHIVE_PROXY_MODE".to_owned(),
            self.config.network.proxy_mode.as_str().to_owned(),
        )];
        env.extend(environment.set);
        let environment = ChildProcessEnvironment::new(env, environment.remove);
        #[cfg(target_os = "linux")]
        let attempt_domain =
            self.containment
                .as_ref()
                .map(
                    |provisioning| -> Result<AttemptWriterDomain, ExecutorError> {
                        let record_root = containment_record_root(&self.config.staging_root)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: "STAGING_ROOT_NOT_ABSOLUTE".to_owned(),
                                error_message: error.to_string(),
                                persistence_already_updated: false,
                            })?;
                        let base = attempt_writer_base(&self.config.staging_root, identity)
                            .map_err(|error| ExecutorError::Execution {
                                error_code: "ATTEMPT_CONTAINMENT_IDENTITY_INVALID".to_owned(),
                                error_message: error.to_string(),
                                persistence_already_updated: false,
                            })?;
                        // Successor admission is fail-closed: a predecessor writer
                        // domain may only be admitted as NoRecord, AlreadyAbsent or
                        // Drained; anything else must refuse this attempt.
                        recover_predecessor_writer_domains(&record_root, &base).map_err(
                            |error| ExecutorError::Execution {
                                error_code: "WRITER_DOMAIN_RECOVERY_FAILED".to_owned(),
                                error_message: error.to_string(),
                                persistence_already_updated: false,
                            },
                        )?;
                        Ok(AttemptWriterDomain {
                            registration: registration.clone(),
                            provisioning: provisioning.clone(),
                            record_root,
                            identity: base,
                        })
                    },
                )
                .transpose()?;
        #[cfg(target_os = "linux")]
        let contained = attempt_domain.as_ref().map(|domain| {
            // The attempt-bound recorded group replaces the retired
            // per-process unique name: the deterministic identity plus the
            // recover-before-create protocol prevents accidental reuse of a
            // crashed earlier attempt's domain. Removal remains
            // supervisor-owned.
            let owner = recover_and_create_writer_group(domain, "sidecar")
                .map_err(xarchive_sidecar_supervisor::SupervisorError::Spawn)?;
            SidecarSupervisor::spawn_contained_with_environment(
                owner,
                &domain.provisioning.python,
                &domain.provisioning.launcher,
                std::path::Path::new(program),
                &args,
                &environment,
            )?
            .require_ready_v2(std::time::Duration::from_secs(5))
        });
        #[cfg(target_os = "linux")]
        let supervisor = match contained {
            Some(result) => result,
            None => SidecarSupervisor::spawn_ready_v2_with_environment(
                program,
                &args,
                &environment,
                std::time::Duration::from_secs(5),
            ),
        };
        #[cfg(not(target_os = "linux"))]
        let supervisor = SidecarSupervisor::spawn_ready_v2_with_environment(
            program,
            &args,
            &environment,
            std::time::Duration::from_secs(5),
        );
        let supervisor = supervisor.map_err(|error| match error {
            xarchive_sidecar_supervisor::SupervisorError::StartupCleanupIncomplete(owner) => {
                AttemptCreationError::StartupCleanupPending(owner)
            }
            error => AttemptCreationError::Failed(ExecutorError::Execution {
                error_code: "SIDECAR_START_FAILED".to_owned(),
                error_message: error.to_string(),
                persistence_already_updated: false,
            }),
        })?;
        let mut context = crate::archive::ArchiveExecutionContext::with_aria2_and_network(
            database,
            files,
            supervisor,
            self.config.aria2_program.clone(),
            self.config.use_aria2,
            self.config.network.clone(),
        )
        .with_attempt_identity(identity.clone())
        .with_downloader_arguments(decoded.downloader_arguments.clone())
        .with_gallery_dl_program(self.config.gallery_dl_program.clone());
        context.publication_registration = registration;
        context.telegram = self.config.telegram.clone();
        #[cfg(target_os = "linux")]
        {
            context.containment = attempt_domain;
        }
        context.telegram_config_file = self.config.telegram_config_file.clone();
        let (execution, _lease) = crate::archive::ArchiveExecutionJob::new(
            context,
            request,
            output_settings,
            tweet_row_id,
            request_id,
            crate::runtime::timestamp_marker(),
        );
        Ok(Box::new(
            execution.with_metadata_output(decoded.metadata_output),
        ))
    }
}

#[cfg(test)]
mod production_factory_attempt_tests {
    use std::path::PathBuf;

    use xarchive_core::JobState;

    use crate::executor::ExecutorNetworkConfig;

    use super::*;

    #[test]
    fn production_factory_rejects_unfenced_legacy_execution_creation() {
        let factory = ProductionExecutionFactory::new(ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root: PathBuf::from("archive"),
            staging_root: PathBuf::from("staging"),
            database_path: PathBuf::from("db.sqlite3"),
            sidecar_program: None,
            gallery_dl_program: None,
            sidecar_args: Vec::new(),
            aria2_program: None,
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        });
        let result = factory.create(
            "job-1",
            &JobSnapshot {
                job_id: "job-1".into(),
                tweet_id: "tweet-1".into(),
                state: JobState::Downloading,
            },
        );
        let error = match result {
            Ok(_) => panic!("legacy creation unexpectedly succeeded without attempt fence"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            ExecutorError::Execution { error_code, .. }
                if error_code == "ATTEMPT_EXECUTION_REQUIRED"
        ));
    }
}

pub(super) struct UnconfiguredExecutionFactory;

impl JobExecutionFactory for UnconfiguredExecutionFactory {
    fn create(
        &self,
        _job_id: &str,
        _snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        Err(ExecutorError::Execution {
            error_code: "EXECUTOR_FACTORY_UNAVAILABLE".to_owned(),
            error_message: "production execution factory is not configured".to_owned(),
            persistence_already_updated: false,
        })
    }
}
