//! Linux attempt-bound writer containment and drain recovery.
//!
//! One responsibility: how a production execution owns a cgroup writer domain,
//! and how a successor discovers and drains the domains a crashed attempt left.
//!
//! Nothing here admits, claims, authorizes or finalizes: the module provides the
//! containment primitives that the executor calls at those points, so moving it
//! cannot reorder ADR-015. Every item is `target_os = "linux"` only.

use std::path::{Path, PathBuf};

use xarchive_workflow::AttemptIdentity;
use xarchive_workflow::linux_containment::RecoveryDrainBinding;
use xarchive_workflow::writer_lifecycle::VerifiedRecoveryDrain;

/// Explicit provisioning, never inferred from inherited environment variables.
/// CROSS_PLATFORM_CHANGE_REQUIRED: Linux opt-in production launch contract.
#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct LinuxExecutionContainment {
    pub delegated_parent: PathBuf,
    pub python: PathBuf,
    pub launcher: PathBuf,
}

#[cfg(target_os = "linux")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LinuxLaunchDescriptor {
    schema_version: u32,
    delegated_parent: PathBuf,
    python: PathBuf,
    launcher: PathBuf,
}

#[cfg(target_os = "linux")]
impl LinuxExecutionContainment {
    /// Explicit trusted-launcher input; never read inherited environment.
    /// CROSS_PLATFORM_CHANGE_REQUIRED: application provisioning contract.
    pub(crate) fn from_launch_arguments(
        arguments: impl IntoIterator<Item = std::ffi::OsString>,
    ) -> Result<Option<Self>, String> {
        use std::io::Read;
        let mut arguments = arguments.into_iter();
        let mut descriptor = None;
        while let Some(argument) = arguments.next() {
            if argument != "--linux-execution-descriptor" {
                continue;
            }
            if descriptor.is_some() {
                return Err("duplicate Linux execution descriptor".into());
            }
            let path = PathBuf::from(
                arguments
                    .next()
                    .ok_or("missing Linux execution descriptor")?,
            );
            if !path.is_absolute() {
                return Err("Linux execution descriptor path must be absolute".into());
            }
            // Trusted launch input must still never block startup on a FIFO or
            // device. Symlinked descriptors are rejected; Python may remain a
            // deployment-managed symlink (as on normal Linux installations).
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|_| "cannot inspect Linux execution descriptor")?;
            if !metadata.file_type().is_file() {
                return Err("Linux execution descriptor must be a regular file".into());
            }
            let file =
                std::fs::File::open(path).map_err(|_| "cannot open Linux execution descriptor")?;
            let mut bytes = Vec::new();
            file.take(16 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "cannot read Linux execution descriptor")?;
            if bytes.len() > 16 * 1024 {
                return Err("Linux execution descriptor exceeds size limit".into());
            }
            let value: LinuxLaunchDescriptor =
                serde_json::from_slice(&bytes).map_err(|_| "invalid Linux execution descriptor")?;
            if value.schema_version != 1 {
                return Err("unsupported Linux execution descriptor version".into());
            }
            let containment = Self {
                delegated_parent: value.delegated_parent,
                python: value.python,
                launcher: value.launcher,
            };
            containment.validate_launch_paths()?;
            descriptor = Some(containment);
        }
        Ok(descriptor)
    }

    pub(crate) fn validate_launch_paths(&self) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        for path in [&self.delegated_parent, &self.python, &self.launcher] {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(
                    "Linux execution paths must be absolute without parent traversal".into(),
                );
            }
        }
        if self.delegated_parent == Path::new("/sys/fs/cgroup")
            || !self.delegated_parent.starts_with("/sys/fs/cgroup")
            || !self.delegated_parent.join("cgroup.events").is_file()
            || !self.delegated_parent.join("cgroup.procs").is_file()
        {
            return Err("Linux execution parent must be a dedicated cgroup v2 directory".into());
        }
        let python =
            std::fs::metadata(&self.python).map_err(|_| "Linux execution Python unavailable")?;
        if !python.is_file() || python.permissions().mode() & 0o111 == 0 || !self.launcher.is_file()
        {
            return Err("Linux execution Python/launcher unavailable".into());
        }
        // Verify delegation with an exclusively created disposable group. The
        // existing adapter rejects symlink traversal and never adopts a peer's
        // group. No target process is started by this capability probe.
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| "Linux execution probe entropy unavailable")?;
        let name = format!("tw2tg-provision-{:032x}", u128::from_le_bytes(nonce));
        let mut probe = xarchive_workflow::linux_containment::LinuxContainment::create(
            &self.delegated_parent,
            &name,
        )
        .map_err(|_| "Linux execution cgroup delegation unavailable")?;
        probe
            .stop_and_drain(std::time::Duration::from_secs(2))
            .and_then(|_| probe.remove_group())
            .map_err(|_| "Linux execution cgroup probe cleanup failed")?;
        Ok(())
    }
}

/// Attempt-bound writer-domain provisioning for one production execution.
///
/// `CROSS_PLATFORM_CHANGE_REQUIRED` (ADR-015 clause 8): R4b replaces
/// per-process unique cgroup names with the recorded attempt-bound
/// containment protocol, so a successor can discover and drain a crashed
/// attempt's writer domains without PID, timestamp or fixed-name inference.
#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct AttemptWriterDomain {
    pub registration: Option<xarchive_workflow::WriterRegistration>,
    pub provisioning: LinuxExecutionContainment,
    pub record_root: PathBuf,
    pub identity: xarchive_workflow::linux_containment::AttemptIdentity,
}

/// Stable workspace-scoped record root beside the staging root.
///
/// The record root lives outside the staging tree so attempt-staging cleanup
/// can never delete a record whose writer domain still exists: record
/// absence must keep proving that the protocol never created a group.
#[cfg(target_os = "linux")]
pub(crate) fn containment_record_root(staging_root: &Path) -> std::io::Result<PathBuf> {
    if !staging_root.is_absolute() {
        return Err(std::io::Error::other(
            "attempt containment requires an absolute staging root",
        ));
    }
    let parent = staging_root
        .parent()
        .ok_or_else(|| std::io::Error::other("staging root has no parent directory"))?;
    Ok(parent.join("tw2tg-containment-records"))
}

/// Attempt-bound containment identity for one writer domain.
#[cfg(target_os = "linux")]
pub(crate) fn attempt_writer_base(
    staging_root: &Path,
    identity: &AttemptIdentity,
) -> std::io::Result<xarchive_workflow::linux_containment::AttemptIdentity> {
    xarchive_workflow::linux_containment::AttemptIdentity::new(
        staging_root,
        identity.job_id(),
        u64::from(identity.attempt()),
    )
}

/// Writer roles whose recorded domains one attempt may create.
///
/// Successor recovery enumerates exactly this set; keep it in sync with the
/// production spawn sites.
#[cfg(target_os = "linux")]
pub(crate) const ATTEMPT_WRITER_ROLES: [&str; 3] = ["sidecar", "aria2-0", "aria2-1"];

/// Drain every recorded writer domain of one attempt.
///
/// Only the three safe outcomes admit; any error fails closed.
#[cfg(target_os = "linux")]
pub(crate) fn recover_writer_domain_set(
    record_root: &Path,
    workspace: &Path,
    job_id: &str,
    attempt: u64,
) -> std::io::Result<()> {
    use xarchive_workflow::linux_containment::{LinuxContainment, RecoveryOutcome};
    if attempt == 0 {
        return Ok(());
    }
    for writer in ATTEMPT_WRITER_ROLES {
        let domain =
            xarchive_workflow::linux_containment::AttemptIdentity::new(workspace, job_id, attempt)?
                .with_writer(writer)?;
        match LinuxContainment::recover_attempt(
            record_root,
            &domain,
            std::time::Duration::from_secs(10),
        )? {
            RecoveryOutcome::NoRecord
            | RecoveryOutcome::AlreadyAbsent
            | RecoveryOutcome::Drained => {}
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn recover_writer_domain_set_with_evidence(
    record_root: &Path,
    binding: RecoveryDrainBinding,
) -> std::io::Result<VerifiedRecoveryDrain> {
    let drained =
        xarchive_workflow::linux_containment::LinuxContainment::recover_production_attempt(
            record_root,
            binding,
            std::time::Duration::from_secs(10),
        )?;
    Ok(drained)
}

/// Drain a crashed predecessor attempt's writer domains before a successor
/// admission. Only the three safe outcomes admit; any error fails closed.
#[cfg(target_os = "linux")]
pub(crate) fn recover_predecessor_writer_domains(
    record_root: &Path,
    base: &xarchive_workflow::linux_containment::AttemptIdentity,
) -> std::io::Result<()> {
    if base.attempt() <= 1 {
        return Ok(());
    }
    recover_writer_domain_set(
        record_root,
        base.workspace(),
        base.job_id(),
        base.attempt() - 1,
    )
}

/// Recover this attempt's writer domain, then create its recorded group.
///
/// Successor admission happens only on the three safe outcomes; any recovery
/// error fails closed instead of falling back to an unrecorded group.
#[cfg(target_os = "linux")]
pub(crate) fn recover_and_create_writer_group(
    domain: &AttemptWriterDomain,
    writer: &str,
) -> std::io::Result<xarchive_workflow::linux_containment::LinuxContainment> {
    use xarchive_workflow::linux_containment::{LinuxContainment, RecoveryOutcome};
    let identity = domain.identity.clone().with_writer(writer)?;
    match LinuxContainment::recover_attempt(
        &domain.record_root,
        &identity,
        std::time::Duration::from_secs(10),
    )? {
        RecoveryOutcome::NoRecord | RecoveryOutcome::AlreadyAbsent | RecoveryOutcome::Drained => {}
    }
    let mut owner = LinuxContainment::create_recorded(
        &domain.record_root,
        &identity,
        &domain.provisioning.delegated_parent,
    )?;
    if let Some(registration) = &domain.registration {
        registration.register_linux(&mut owner, &identity)?;
    }
    Ok(owner)
}

#[cfg(all(test, target_os = "linux"))]
mod attempt_writer_domain_tests {
    use super::*;

    #[test]
    fn record_root_requires_an_absolute_staging_root() {
        assert!(containment_record_root(Path::new("staging")).is_err());
        let root = containment_record_root(Path::new("/data/_staging")).unwrap();
        assert!(root.starts_with("/data"));
        assert!(root.ends_with("tw2tg-containment-records"));
    }

    #[test]
    fn predecessor_recovery_skips_the_first_attempt() {
        let base = xarchive_workflow::linux_containment::AttemptIdentity::new("/ws/r4b", "job", 1)
            .unwrap();
        recover_predecessor_writer_domains(Path::new("/nonexistent-record-root"), &base).unwrap();
    }

    #[test]
    fn predecessor_recovery_fails_closed_on_a_malformed_record() {
        let record_root = std::env::temp_dir().join(format!(
            "tw2tg-r4b-predecessor-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&record_root).unwrap();
        let base = xarchive_workflow::linux_containment::AttemptIdentity::new("/ws/r4b", "job", 2)
            .unwrap();
        let previous =
            xarchive_workflow::linux_containment::AttemptIdentity::new("/ws/r4b", "job", 1)
                .unwrap()
                .with_writer("sidecar")
                .unwrap();
        // A malformed predecessor record must stop the successor instead of
        // being treated as absence.
        std::fs::write(
            previous.record_path(&record_root),
            "not a containment record\n",
        )
        .unwrap();
        let error = recover_predecessor_writer_domains(&record_root, &base).unwrap_err();
        assert_ne!(error.kind(), std::io::ErrorKind::NotFound);
        std::fs::remove_dir_all(&record_root).unwrap();
    }
}
