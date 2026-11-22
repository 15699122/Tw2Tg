import { createHash } from "node:crypto";
import { copyFile, mkdir, readdir, readFile, stat, writeFile } from "node:fs/promises";
import { join, relative, resolve, sep } from "node:path";

export const OFFLINE_BUNDLE_MANIFEST_FILE = "offline-bundle-manifest.json";
export const OFFLINE_BUNDLE_CATALOG_FILE = "components/catalog.json";
export const OFFLINE_BUNDLE_RELEASE_MANIFEST_FILE = "release-manifest.json";
/** Matches `COMPONENT_CATALOG_SCHEMA_VERSION` in `desktop/src-tauri/src/components.rs`. */
export const EMBEDDED_CATALOG_SCHEMA_VERSION = 1;
export const EMBEDDED_CATALOG_PLATFORM = "windows";
export const EMBEDDED_CATALOG_ARCHITECTURE = "x64";

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

/** Walk an artifact tree in the order `components.rs::tree_digest` uses. */
async function collectArtifactFiles(root) {
  const files = [];
  async function walk(current) {
    for (const entry of await readdir(current, { withFileTypes: true })) {
      const absolute = join(current, entry.name);
      if (entry.isDirectory()) await walk(absolute);
      else if (entry.isFile()) {
        files.push({ relative: relative(root, absolute).split(sep).join("/"), absolute });
      } else {
        // A link or device inside an artifact is not something the Windows
        // installer may copy; refuse instead of silently resolving it.
        throw new Error(`unsupported filesystem entry in artifact: ${absolute}`);
      }
    }
  }
  await walk(root);
  // `tree_digest` sorts the UTF-8 bytes of the relative path, so compare encoded
  // buffers: JS string order is UTF-16 code units and disagrees outside ASCII.
  return files.sort((left, right) => Buffer.compare(Buffer.from(left.relative, "utf8"), Buffer.from(right.relative, "utf8")));
}

/**
 * SHA-256 over one artifact directory, identical to `tree_digest` in
 * `desktop/src-tauri/src/components.rs`: each file contributes its relative
 * path, a NUL separator, its little-endian u64 length and its bytes, in
 * relative-path order. The catalog the Windows installer verifies is therefore
 * the same value this script writes.
 */
export async function treeDigest(root) {
  const found = await collectArtifactFiles(root);
  const hasher = createHash("sha256");
  let sizeBytes = 0;
  for (const file of found) {
    const bytes = await readFile(file.absolute);
    sizeBytes += bytes.length;
    const length = Buffer.alloc(8);
    length.writeBigUInt64LE(BigInt(bytes.length));
    hasher.update(Buffer.from(file.relative, "utf8"));
    hasher.update(Buffer.from([0]));
    hasher.update(length);
    hasher.update(bytes);
  }
  return { sha256: hasher.digest("hex"), size_bytes: sizeBytes, files: found.map((file) => file.relative) };
}

/**
 * Derive one component's manifest fields from the artifacts on disk.
 *
 * The caller supplies identity and file lists only. The hash and size are
 * computed here, so a manifest cannot carry a hash that was not derived from
 * the bytes it describes. `required_files` and `license_files` are relative to
 * the component's artifact root, which is how `components.rs` resolves them.
 */
export async function describeOfflineBundleComponent(sourceRoot, input) {
  if (!input || typeof input !== "object") throw new Error("offline bundle component must be an object");
  const id = nonEmpty(input.id, "component id");
  if (!OFFLINE_COMPONENT_IDS.has(id)) throw new Error(`unsupported offline bundle component: ${id}`);
  const version = nonEmpty(input.version, `component ${id} version`);
  const artifact = validateBundlePath(input.artifact, `component ${id} artifact`);
  const absolute = resolve(sourceRoot, artifact);
  if (!(await stat(absolute)).isDirectory()) throw new Error(`component ${id} artifact must be a directory: ${artifact}`);
  const digest = await treeDigest(absolute);
  if (!Array.isArray(input.required_files) || input.required_files.length === 0) throw new Error(`component ${id} must list required_files`);
  if (!Array.isArray(input.license_files) || input.license_files.length === 0) throw new Error(`component ${id} must list license_files`);
  const required_files = input.required_files.map((file) => validateBundlePath(file, `component ${id} required file`));
  const license_files = input.license_files.map((file) => validateBundlePath(file, `component ${id} license file`));
  for (const file of [...required_files, ...license_files]) {
    if (!digest.files.includes(file)) throw new Error(`component ${id} lists a file the artifact does not contain: ${file}`);
  }
  return validateOfflineBundleComponent({
    id,
    version,
    artifact,
    sha256: digest.sha256,
    size_bytes: digest.size_bytes,
    required_files,
    license_files,
  });
}

/**
 * The embedded catalog `components.rs` validates, built from the same values as
 * the bundle manifest so the two descriptions cannot drift.
 */
export function createEmbeddedCatalog({ catalogVersion, components }) {
  nonEmpty(catalogVersion, "catalog_version");
  if (!Array.isArray(components) || components.length === 0) throw new Error("embedded catalog requires components");
  const seen = new Set();
  const entries = components.map((component) => {
    validateOfflineBundleComponent(component);
    if (seen.has(component.id)) throw new Error(`duplicate component ${component.id}`);
    seen.add(component.id);
    return {
      id: component.id,
      version: component.version,
      platform: EMBEDDED_CATALOG_PLATFORM,
      architecture: EMBEDDED_CATALOG_ARCHITECTURE,
      artifact: component.artifact,
      sha256: component.sha256,
      // `components.rs` rejects an artifact larger than this limit. A pinned
      // component has one exact size, so the exact size is the tightest limit.
      max_size_bytes: component.size_bytes,
      required_files: component.required_files,
      license_files: component.license_files,
      probe: null,
      protocol: null,
    };
  });
  return {
    schema_version: EMBEDDED_CATALOG_SCHEMA_VERSION,
    catalog_version: catalogVersion,
    components: entries,
  };
}

async function copyArtifactTree(source, target) {
  await mkdir(target, { recursive: true });
  for (const entry of await readdir(source, { withFileTypes: true })) {
    const from = join(source, entry.name);
    const to = join(target, entry.name);
    if (entry.isDirectory()) await copyArtifactTree(from, to);
    else if (entry.isFile()) await copyFile(from, to);
    else throw new Error(`unsupported filesystem entry in artifact: ${from}`);
  }
}

function assertDisjointArtifacts(components) {
  for (let index = 0; index < components.length; index += 1) {
    for (let other = index + 1; other < components.length; other += 1) {
      const left = components[index].artifact;
      const right = components[other].artifact;
      if (left === right || left.startsWith(`${right}/`) || right.startsWith(`${left}/`)) {
        throw new Error(`offline bundle artifacts must not overlap: ${left} and ${right}`);
      }
    }
  }
}

/**
 * Assemble a complete Offline Bundle from a release input directory.
 *
 * This is the cross-platform half of the U13 plan: it fixes the layout, derives
 * every hash and size from the bytes present, writes the embedded catalog and
 * the bundle manifest from one component list, and re-verifies the result. It
 * does not produce Windows artifacts, sign anything, download a `latest`, or
 * create runtime directories; those inputs and the release acceptance remain
 * Windows-owned.
 */
export async function assembleOfflineBundle({
  sourceRoot,
  outputRoot,
  releaseTag,
  catalogVersion,
  components,
  releaseManifest = OFFLINE_BUNDLE_RELEASE_MANIFEST_FILE,
}) {
  if (!Array.isArray(components) || components.length !== OFFLINE_COMPONENT_IDS.size) {
    throw new Error("offline bundle must list exactly the required components");
  }
  const described = [];
  for (const component of components) described.push(await describeOfflineBundleComponent(sourceRoot, component));
  assertDisjointArtifacts(described);
  const manifest = createOfflineBundleManifest({
    releaseTag,
    catalogVersion,
    components: described,
    releaseManifest,
    embeddedCatalog: OFFLINE_BUNDLE_CATALOG_FILE,
  });
  const catalog = createEmbeddedCatalog({ catalogVersion, components: described });
  const releaseSource = resolve(sourceRoot, validateBundlePath(releaseManifest, "release_manifest"));
  if (!(await stat(releaseSource)).isFile()) throw new Error(`release_manifest must be a file: ${releaseManifest}`);

  // Written once into a fresh directory: merging into an existing tree would
  // let a stale file survive a failed assembly.
  const destination = resolve(outputRoot);
  await mkdir(destination);
  for (const component of described) {
    await copyArtifactTree(resolve(sourceRoot, component.artifact), resolve(destination, component.artifact));
  }
  await copyFile(releaseSource, resolve(destination, releaseManifest));
  await mkdir(resolve(destination, "components"), { recursive: true });
  await writeFile(resolve(destination, OFFLINE_BUNDLE_CATALOG_FILE), `${JSON.stringify(catalog, null, 2)}\n`);
  await writeFile(resolve(destination, OFFLINE_BUNDLE_MANIFEST_FILE), `${JSON.stringify(manifest, null, 2)}\n`);
  await verifyOfflineBundle({ root: destination, manifest });
  return { manifest, catalog };
}

/**
 * Re-read an assembled bundle and prove its three descriptions agree: the
 * bundle manifest, the embedded catalog and the bytes actually written.
 */
export async function verifyOfflineBundle({ root, manifest }) {
  const validated = validateOfflineBundleManifest(manifest);
  const catalog = JSON.parse(await readFile(resolve(root, validated.embedded_catalog), "utf8"));
  if (catalog.schema_version !== EMBEDDED_CATALOG_SCHEMA_VERSION) {
    throw new Error("embedded catalog schema_version does not match components.rs");
  }
  if (catalog.catalog_version !== validated.catalog_version) {
    throw new Error("embedded catalog catalog_version does not match the bundle manifest");
  }
  const entries = new Map(catalog.components.map((component) => [component.id, component]));
  if (entries.size !== catalog.components.length) throw new Error("embedded catalog contains duplicate components");
  if (!(await stat(resolve(root, validated.release_manifest))).isFile()) {
    throw new Error(`release manifest is missing: ${validated.release_manifest}`);
  }
  for (const component of validated.components) {
    const entry = entries.get(component.id);
    if (!entry) throw new Error(`embedded catalog is missing component: ${component.id}`);
    for (const field of ["version", "artifact", "sha256"]) {
      if (entry[field] !== component[field]) {
        throw new Error(`embedded catalog ${component.id} ${field} does not match the bundle manifest`);
      }
    }
    if (entry.max_size_bytes !== component.size_bytes) {
      throw new Error(`embedded catalog ${component.id} max_size_bytes does not match the bundle size`);
    }
    const digest = await treeDigest(resolve(root, component.artifact));
    if (digest.sha256 !== component.sha256 || digest.size_bytes !== component.size_bytes) {
      throw new Error(`assembled artifact does not match its manifest: ${component.id}`);
    }
    for (const file of [...component.required_files, ...component.license_files]) {
      if (!digest.files.includes(file)) {
        throw new Error(`assembled artifact ${component.id} is missing ${file}`);
      }
    }
  }
  return { manifest: validated, catalog };
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