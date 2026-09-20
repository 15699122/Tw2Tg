//! Embedded component catalog and local component activation.
//!
//! U9 deliberately keeps acquisition outside this module. A caller supplies a
//! local artifact directory obtained from a trusted release/Offline Bundle;
//! this module verifies it against the fixed catalog contract before activation.
//! No dynamic `latest` lookup, unsigned remote manifest, archive extraction, or
//! platform-specific process management is performed here.
#![allow(dead_code)]

// The public manager surface is intentionally prepared for U10 Bootstrap
// wiring. U9 validates it through module tests before the Setup Wizard owns the
// runtime calls, so unused API items are expected in this unit.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component as PathComponent, Path, PathBuf};

pub const COMPONENT_CATALOG_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentSpec {
    pub id: String,
    pub version: String,
    pub platform: String,
    pub architecture: String,
    pub artifact: String,
    pub sha256: String,
    pub max_size_bytes: u64,
    pub required_files: Vec<String>,
    pub license_files: Vec<String>,
    pub probe: Option<String>,
    pub protocol: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EmbeddedComponentCatalog {
    pub schema_version: u32,
    pub catalog_version: String,
    pub components: Vec<ComponentSpec>,
}

/// The first embedded catalog is intentionally empty until U11 publishes
/// versioned release assets and their hashes. The schema and manager are live;
/// an empty catalog prevents the application from activating an unverified
/// placeholder artifact instead of silently weakening integrity checks.
pub fn embedded_catalog() -> EmbeddedComponentCatalog {
    EmbeddedComponentCatalog {
        schema_version: COMPONENT_CATALOG_SCHEMA_VERSION,
        catalog_version: "2026-09-20-u9".to_owned(),
        components: Vec::new(),
    }
}

#[derive(Debug)]
pub enum ComponentError {
    Io(io::Error),
    InvalidCatalog(String),
    ComponentNotFound(String),
    InvalidArtifact(String),
    IntegrityMismatch(String),
    Activation(String),
    RollbackUnavailable,
}

impl std::fmt::Display for ComponentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "component I/O failed: {error}"),
            Self::InvalidCatalog(message) => {
                write!(formatter, "invalid component catalog: {message}")
            }
            Self::ComponentNotFound(id) => {
                write!(formatter, "component is not in the embedded catalog: {id}")
            }
            Self::InvalidArtifact(message) => {
                write!(formatter, "invalid component artifact: {message}")
            }
            Self::IntegrityMismatch(message) => {
                write!(formatter, "component integrity mismatch: {message}")
            }
            Self::Activation(message) => {
                write!(formatter, "component activation failed: {message}")
            }
            Self::RollbackUnavailable => formatter.write_str("component rollback is unavailable"),
        }
    }
}

impl std::error::Error for ComponentError {}

impl From<io::Error> for ComponentError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Debug)]
pub struct ComponentManager {
    root: PathBuf,
    catalog: EmbeddedComponentCatalog,
}

impl ComponentManager {
    pub fn new(
        root: impl Into<PathBuf>,
        catalog: EmbeddedComponentCatalog,
    ) -> Result<Self, ComponentError> {
        validate_catalog(&catalog)?;
        Ok(Self {
            root: root.into(),
            catalog,
        })
    }

    pub fn embedded(root: impl Into<PathBuf>) -> Result<Self, ComponentError> {
        Self::new(root, embedded_catalog())
    }

    pub fn catalog(&self) -> &EmbeddedComponentCatalog {
        &self.catalog
    }

    pub fn component_root(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    pub fn validate_artifact(&self, id: &str, artifact_root: &Path) -> Result<(), ComponentError> {
        let spec = self
            .catalog
            .components
            .iter()
            .find(|component| component.id == id)
            .ok_or_else(|| ComponentError::ComponentNotFound(id.to_owned()))?;
        validate_artifact(spec, artifact_root)
    }

    /// Verify and atomically activate a local artifact directory.
    pub fn install_from_dir(&self, id: &str, artifact_root: &Path) -> Result<(), ComponentError> {
        self.validate_artifact(id, artifact_root)?;
        let spec = self
            .catalog
            .components
            .iter()
            .find(|component| component.id == id)
            .ok_or_else(|| ComponentError::ComponentNotFound(id.to_owned()))?;
        let component_root = self.component_root(id);
        fs::create_dir_all(&component_root)?;
        let staging = component_root.join(format!("{}.part", spec.version));
        let activated = component_root.join(&spec.version);
        remove_path(&staging)?;
        copy_tree(artifact_root, &staging)?;
        self.validate_artifact(id, &staging)?;
        remove_path(&activated)?;
        fs::rename(&staging, &activated)
            .map_err(|error| ComponentError::Activation(error.to_string()))?;
        self.write_current(id, &spec.version)?;
        Ok(())
    }

    pub fn active_version(&self, id: &str) -> Result<Option<String>, ComponentError> {
        let current = self.component_root(id).join("current");
        match fs::read_to_string(current) {
            Ok(version) => Ok(Some(version.trim().to_owned())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn rollback(&self, id: &str) -> Result<(), ComponentError> {
        let root = self.component_root(id);
        let previous = root.join("previous");
        let version = fs::read_to_string(&previous).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                ComponentError::RollbackUnavailable
            } else {
                ComponentError::Io(error)
            }
        })?;
        let version = version.trim();
        if version.is_empty() || !root.join(version).is_dir() {
            return Err(ComponentError::RollbackUnavailable);
        }
        self.write_current(id, version)
    }

    fn write_current(&self, id: &str, version: &str) -> Result<(), ComponentError> {
        let root = self.component_root(id);
        fs::create_dir_all(&root)?;
        let current = root.join("current");
        if let Ok(old) = fs::read_to_string(&current) {
            let previous = root.join("previous.part");
            fs::write(&previous, old.trim())?;
            fs::rename(previous, root.join("previous"))
                .map_err(|error| ComponentError::Activation(error.to_string()))?;
        }
        let marker = root.join("current.part");
        let mut file = fs::File::create(&marker)?;
        file.write_all(version.as_bytes())?;
        file.sync_all()?;
        fs::rename(marker, current)
            .map_err(|error| ComponentError::Activation(error.to_string()))?;
        Ok(())
    }
}

pub fn validate_catalog(catalog: &EmbeddedComponentCatalog) -> Result<(), ComponentError> {
    if catalog.schema_version != COMPONENT_CATALOG_SCHEMA_VERSION {
        return Err(ComponentError::InvalidCatalog(format!(
            "unsupported schema version {}",
            catalog.schema_version
        )));
    }
    if catalog.catalog_version.trim().is_empty() {
        return Err(ComponentError::InvalidCatalog(
            "catalog_version is empty".to_owned(),
        ));
    }
    let mut ids = BTreeMap::new();
    for spec in &catalog.components {
        if spec.id.trim().is_empty() || spec.version.trim().is_empty() {
            return Err(ComponentError::InvalidCatalog(
                "component id/version is empty".to_owned(),
            ));
        }
        if ids.insert(&spec.id, ()).is_some() {
            return Err(ComponentError::InvalidCatalog(format!(
                "duplicate component {}",
                spec.id
            )));
        }
        validate_relative_paths(spec, &spec.required_files)?;
        validate_relative_paths(spec, &spec.license_files)?;
        if let Some(probe) = &spec.probe {
            validate_relative_path(probe).map_err(ComponentError::InvalidCatalog)?;
        }
        if !is_sha256(&spec.sha256) {
            return Err(ComponentError::InvalidCatalog(format!(
                "component {} has an invalid SHA-256",
                spec.id
            )));
        }
    }
    Ok(())
}

fn validate_artifact(spec: &ComponentSpec, artifact_root: &Path) -> Result<(), ComponentError> {
    if !artifact_root.is_dir() {
        return Err(ComponentError::InvalidArtifact(
            "artifact root is not a directory".to_owned(),
        ));
    }
    let (digest, size) = tree_digest(artifact_root)?;
    if size > spec.max_size_bytes {
        return Err(ComponentError::IntegrityMismatch(format!(
            "size {size} exceeds limit {}",
            spec.max_size_bytes
        )));
    }
    if digest != spec.sha256 {
        return Err(ComponentError::IntegrityMismatch(format!(
            "SHA-256 {digest} does not match catalog value"
        )));
    }
    for relative in spec.required_files.iter().chain(spec.license_files.iter()) {
        if !artifact_root.join(relative).is_file() {
            return Err(ComponentError::InvalidArtifact(format!(
                "required file is missing: {relative}"
            )));
        }
    }
    if let Some(probe) = &spec.probe {
        let path = artifact_root.join(probe);
        if !path.is_file() {
            return Err(ComponentError::InvalidArtifact(format!(
                "probe is missing: {probe}"
            )));
        }
        if fs::metadata(path)?.len() == 0 {
            return Err(ComponentError::InvalidArtifact(format!(
                "probe is empty: {probe}"
            )));
        }
    }
    Ok(())
}

fn validate_relative_paths(spec: &ComponentSpec, paths: &[String]) -> Result<(), ComponentError> {
    for path in paths {
        validate_relative_path(path).map_err(|error| {
            ComponentError::InvalidCatalog(format!("component {}: {error}", spec.id))
        })?;
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() {
        return Err(format!("unsafe relative path: {value}"));
    }
    for component in path.components() {
        if matches!(
            component,
            PathComponent::ParentDir | PathComponent::RootDir | PathComponent::Prefix(_)
        ) {
            return Err(format!("unsafe relative path: {value}"));
        }
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn tree_digest(root: &Path) -> Result<(String, u64), ComponentError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    let mut total = 0;
    for (relative, path) in files {
        let mut file = fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        total += bytes.len() as u64;
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok((format!("{:x}", hasher.finalize()), total))
}

fn collect_files(
    root: &Path,
    current: &Path,
    output: &mut Vec<(String, PathBuf)>,
) -> Result<(), ComponentError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_files(root, &path, output)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| ComponentError::InvalidArtifact(error.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            validate_relative_path(&relative).map_err(ComponentError::InvalidArtifact)?;
            output.push((relative, path));
        } else {
            return Err(ComponentError::InvalidArtifact(format!(
                "unsupported filesystem entry: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), ComponentError> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_tree(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(source_path, target_path)?;
        } else {
            return Err(ComponentError::InvalidArtifact(
                "symlink or special file is not allowed".to_owned(),
            ));
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), ComponentError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("xarchive-u9-{name}-{suffix}"))
    }

    fn fixture_spec(root: &Path, version: &str) -> ComponentSpec {
        let artifact = root.join("artifact");
        fs::create_dir_all(&artifact).expect("artifact");
        fs::write(artifact.join("worker.bin"), b"worker").expect("worker");
        fs::write(artifact.join("LICENSE.txt"), b"MIT").expect("license");
        let (sha256, size) = tree_digest(&artifact).expect("digest");
        ComponentSpec {
            id: "worker".to_owned(),
            version: version.to_owned(),
            platform: "linux".to_owned(),
            architecture: "x86_64".to_owned(),
            artifact: "worker.bin".to_owned(),
            sha256,
            max_size_bytes: size + 1,
            required_files: vec!["worker.bin".to_owned()],
            license_files: vec!["LICENSE.txt".to_owned()],
            probe: Some("worker.bin".to_owned()),
            protocol: Some("sidecar-v2".to_owned()),
        }
    }

    #[test]
    fn embedded_catalog_is_fixed_and_does_not_enable_unverified_artifacts() {
        let catalog = embedded_catalog();
        assert_eq!(catalog.schema_version, COMPONENT_CATALOG_SCHEMA_VERSION);
        assert_eq!(catalog.catalog_version, "2026-09-20-u9");
        assert!(catalog.components.is_empty());
    }

    #[test]
    fn validates_catalog_paths_and_hashes() {
        let root = temp_root("catalog");
        let spec = fixture_spec(&root, "1.0.0");
        let catalog = EmbeddedComponentCatalog {
            schema_version: COMPONENT_CATALOG_SCHEMA_VERSION,
            catalog_version: "test".to_owned(),
            components: vec![spec.clone()],
        };
        validate_catalog(&catalog).expect("catalog");
        let manager = ComponentManager::new(root.join("components"), catalog).expect("manager");
        manager
            .validate_artifact("worker", &root.join("artifact"))
            .expect("artifact");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_traversal_and_hash_mismatch() {
        let root = temp_root("reject");
        let mut spec = fixture_spec(&root, "1.0.0");
        spec.required_files = vec!["../escape".to_owned()];
        let catalog = EmbeddedComponentCatalog {
            schema_version: COMPONENT_CATALOG_SCHEMA_VERSION,
            catalog_version: "test".to_owned(),
            components: vec![spec],
        };
        assert!(matches!(
            validate_catalog(&catalog),
            Err(ComponentError::InvalidCatalog(_))
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn installs_atomically_and_rolls_back_to_previous_version() {
        let root = temp_root("activation");
        let artifact = root.join("artifact");
        let mut first = fixture_spec(&root, "1.0.0");
        let manager_root = root.join("components");
        let manager = ComponentManager::new(
            manager_root.clone(),
            EmbeddedComponentCatalog {
                schema_version: COMPONENT_CATALOG_SCHEMA_VERSION,
                catalog_version: "test".to_owned(),
                components: vec![first.clone()],
            },
        )
        .expect("manager");
        manager
            .install_from_dir("worker", &artifact)
            .expect("first install");
        assert_eq!(
            manager.active_version("worker").expect("active"),
            Some("1.0.0".to_owned())
        );

        fs::write(artifact.join("worker.bin"), b"worker-v2").expect("update");
        let (sha256, size) = tree_digest(&artifact).expect("digest");
        first.version = "2.0.0".to_owned();
        first.sha256 = sha256;
        first.max_size_bytes = size + 1;
        let manager = ComponentManager::new(
            manager_root,
            EmbeddedComponentCatalog {
                schema_version: COMPONENT_CATALOG_SCHEMA_VERSION,
                catalog_version: "test".to_owned(),
                components: vec![first],
            },
        )
        .expect("manager v2");
        manager
            .install_from_dir("worker", &artifact)
            .expect("second install");
        assert_eq!(
            manager.active_version("worker").expect("active"),
            Some("2.0.0".to_owned())
        );
        manager.rollback("worker").expect("rollback");
        assert_eq!(
            manager.active_version("worker").expect("active"),
            Some("1.0.0".to_owned())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
