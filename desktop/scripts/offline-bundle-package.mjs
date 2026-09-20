export const OFFLINE_BUNDLE_SCHEMA_VERSION = 1;
export const OFFLINE_BUNDLE_PACKAGE_TYPE = "offline-bundle";
export const OFFLINE_BUNDLE_PLATFORM = "windows-x64";

export const OFFLINE_COMPONENT_IDS = new Set([
  "desktop",
  "worker",
  "native-host",
  "extension",
  "gallery-dl",
  "aria2",
]);

export const FORBIDDEN_RUNTIME_PATHS = new Set([
  "config",
  "cache",
  "download",
  "logs",
]);

const VERSION_PATTERN = /^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const SHA256_PATTERN = /^[0-9a-fA-F]{64}$/;

function nonEmpty(value, field) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${field} must be a non-empty string`);
  }
  return value;
}

export function validateBundlePath(value, field = "path") {
  nonEmpty(value, field);
  if (value.startsWith("/") || value.startsWith("\\") || /^[A-Za-z]:/.test(value)) {
    throw new Error(`${field} must be relative`);
  }
  const parts = value.split(/[\\/]/).filter(Boolean);
  if (parts.includes("..")) {
    throw new Error(`${field} must not escape the bundle root`);
  }
  if (parts.length === 0) {
    throw new Error(`${field} must not be empty`);
  }
  return parts.join("/");
}

export function validateOfflineBundleComponent(component) {
  if (!component || typeof component !== "object" || Array.isArray(component)) {
    throw new Error("offline bundle component must be an object");
  }
  nonEmpty(component.id, "component id");
  if (!OFFLINE_COMPONENT_IDS.has(component.id)) {
    throw new Error(`unsupported offline bundle component: ${component.id}`);
  }
  nonEmpty(component.version, `component ${component.id} version`);
  validateBundlePath(component.artifact, `component ${component.id} artifact`);
  if (!SHA256_PATTERN.test(component.sha256 || "")) {
    throw new Error(`component ${component.id} must carry a SHA-256`);
  }
  if (!Number.isInteger(component.size_bytes) || component.size_bytes <= 0) {
    throw new Error(`component ${component.id} must carry a positive size_bytes`);
  }
  if (!Array.isArray(component.required_files) || component.required_files.length === 0) {
    throw new Error(`component ${component.id} must list required_files`);
  }
  component.required_files.forEach((file) => validateBundlePath(file, `component ${component.id} required file`));
  if (!Array.isArray(component.license_files) || component.license_files.length === 0) {
    throw new Error(`component ${component.id} must list license_files`);
  }
  component.license_files.forEach((file) => validateBundlePath(file, `component ${component.id} license file`));
  return component;
}

export function validateOfflineBundleManifest(manifest) {
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) {
    throw new Error("offline bundle manifest must be an object");
  }
  if (manifest.schema_version !== OFFLINE_BUNDLE_SCHEMA_VERSION) {
    throw new Error(`offline bundle schema_version must be ${OFFLINE_BUNDLE_SCHEMA_VERSION}`);
  }
  if (manifest.package_type !== OFFLINE_BUNDLE_PACKAGE_TYPE) {
    throw new Error(`offline bundle package_type must be ${OFFLINE_BUNDLE_PACKAGE_TYPE}`);
  }
  if (manifest.platform !== OFFLINE_BUNDLE_PLATFORM) {
    throw new Error(`offline bundle platform must be ${OFFLINE_BUNDLE_PLATFORM}`);
  }
  if (!VERSION_PATTERN.test(manifest.release_tag || "")) {
    throw new Error("offline bundle release_tag must be versioned");
  }
  nonEmpty(manifest.catalog_version, "catalog_version");
  nonEmpty(manifest.release_manifest, "release_manifest");
  nonEmpty(manifest.embedded_catalog, "embedded_catalog");

  if (!Array.isArray(manifest.components) || manifest.components.length !== OFFLINE_COMPONENT_IDS.size) {
    throw new Error("offline bundle must list exactly the required components");
  }
  const ids = new Set();
  for (const component of manifest.components) {
    validateOfflineBundleComponent(component);
    if (ids.has(component.id)) throw new Error(`duplicate offline bundle component: ${component.id}`);
    ids.add(component.id);
  }
  for (const id of OFFLINE_COMPONENT_IDS) {
    if (!ids.has(id)) throw new Error(`offline bundle is missing component: ${id}`);
  }

  const runtimePaths = manifest.runtime_directories || [];
  if (!Array.isArray(runtimePaths)) throw new Error("runtime_directories must be an array");
  for (const path of runtimePaths) {
    const normalized = validateBundlePath(path, "runtime directory");
    if (FORBIDDEN_RUNTIME_PATHS.has(normalized)) {
      throw new Error(`offline bundle must not pre-create runtime directory: ${normalized}`);
    }
  }

  if (!manifest.parity || manifest.parity.release_manifest !== manifest.release_manifest || manifest.parity.catalog_version !== manifest.catalog_version) {
    throw new Error("offline bundle parity must match release_manifest and catalog_version");
  }
  return manifest;
}

export function createOfflineBundleManifest({ releaseTag, catalogVersion, components, releaseManifest = "release-manifest.json", embeddedCatalog = "components/catalog.json" }) {
  return validateOfflineBundleManifest({
    schema_version: OFFLINE_BUNDLE_SCHEMA_VERSION,
    package_type: OFFLINE_BUNDLE_PACKAGE_TYPE,
    platform: OFFLINE_BUNDLE_PLATFORM,
    release_tag: releaseTag,
    catalog_version: catalogVersion,
    release_manifest: releaseManifest,
    embedded_catalog: embeddedCatalog,
    components,
    runtime_directories: [],
    parity: {
      release_manifest: releaseManifest,
      catalog_version: catalogVersion,
    },
  });
}