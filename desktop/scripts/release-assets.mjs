//! Versioned release-asset manifest contract (U11, Linux scope).
//!
//! This module only defines *naming and manifest validation* for Windows x64
//! release assets. It never downloads, builds, signs, or uploads artifacts:
//! real `.exe` / `.7z` files, their SHA-256 values, and GitHub Release
//! publication are produced on Windows runners / CI (see the Windows queue).
//! The embedded component catalog stays the single source of truth for what
//! the Desktop is allowed to activate; this manifest describes the matching
//! GitHub Release assets so reviewers can compare names, versions, hashes,
//! sizes, and licenses without trusting a dynamic `latest` lookup.

export const RELEASE_SCHEMA_VERSION = 1;
export const RELEASE_PLATFORM = "windows-x64";

const TAG_PATTERN = /^v\d+\.\d+\.\d+(-pre\.\d+)?$/;
const HEX64_PATTERN = /^[0-9a-fA-F]{64}$/;

function assertNonEmptyString(value, field) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${field} must be a non-empty string`);
  }
  return value;
}

export function validateReleaseTag(tag) {
  assertNonEmptyString(tag, "tag");
  if (!TAG_PATTERN.test(tag)) {
    throw new Error(`release tag must match vMAJOR.MINOR.PATCH[-pre.N], got: ${tag}`);
  }
  return tag;
}

export function releaseAssetNames(tag) {
  validateReleaseTag(tag);
  return {
    executable: `XArchive-${tag}-windows-x64.exe`,
    archive: `XArchive-${tag}-windows-x64.7z`,
    repository_dependencies: `XArchive-${tag}-windows-x64-repository-dependencies.7z`,
    full: `XArchive-${tag}-windows-x64-full.7z`,
    extension: `XArchive-${tag}-extension.zip`,
  };
}

/// Asset kinds the release manifest must describe before publishing.
export const REQUIRED_RELEASE_ASSET_KINDS = [
  "executable",
  "archive",
  "repository_dependencies",
  "full",
  "extension",
];

const ASSET_NAME_PATTERNS = [
  [/^XArchive-(v\d+\.\d+\.\d+(?:-pre\.\d+)?)-windows-x64\.exe$/, "executable"],
  [/^XArchive-(v\d+\.\d+\.\d+(?:-pre\.\d+)?)-windows-x64\.7z$/, "archive"],
  [
    /^XArchive-(v\d+\.\d+\.\d+(?:-pre\.\d+)?)-windows-x64-repository-dependencies\.7z$/,
    "repository_dependencies",
  ],
  [/^XArchive-(v\d+\.\d+\.\d+(?:-pre\.\d+)?)-windows-x64-full\.7z$/, "full"],
  [/^XArchive-(v\d+\.\d+\.\d+(?:-pre\.\d+)?)-extension\.zip$/, "extension"],
];

export function parseReleaseAssetName(name) {
  assertNonEmptyString(name, "asset name");
  for (const [pattern, kind] of ASSET_NAME_PATTERNS) {
    const match = pattern.exec(name);
    if (match) {
      return { tag: match[1], kind };
    }
  }
  throw new Error(`release asset name is not a versioned XArchive asset, got: ${name}`);
}

/// Files the loadable Extension ZIP must always contain, relative to its root.
export const EXTENSION_PACKAGE_REQUIRED_FILES = [
  "manifest.json",
  "src/background.js",
  "src/content-core.js",
  "src/content.js",
];

// Paths which must never be shipped inside the loadable Extension ZIP: test
// suites, dependency trees, build caches, local secrets, and signing material.
const EXCLUDED_EXTENSION_PACKAGE_PATTERNS = [
  /^tests?\//i,
  /^node_modules\//i,
  /^dist\//i,
  /^\.vite\//i,
  /^coverage\//i,
  /^secrets?\//i,
  /^\.git\//i,
  /^cache\//i,
  /^logs?\//i,
  /(^|\/)package(-lock)?\.json$/i,
  /(^|\/)readme\.md$/i,
  /\.(pem|key|p12|pfx|sops\.json)$/i,
  /\.env(\..*)?$/i,
  /\.log$/i,
  /\.(sqlite3?|db)$/i,
];

export function normalizeExtensionPackagePath(value) {
  assertNonEmptyString(value, "extension package path");
  const normalized = value.replace(/\\/g, "/").replace(/^\.\//, "");
  if (normalized.startsWith("/") || /^[a-zA-Z]:/.test(normalized)) {
    throw new Error(`extension package path must be relative, got: ${value}`);
  }
  if (normalized.split("/").includes("..")) {
    throw new Error(`extension package path must not escape its root, got: ${value}`);
  }
  return normalized;
}

export function isExcludedExtensionPackagePath(value) {
  const normalized = normalizeExtensionPackagePath(value);
  return EXCLUDED_EXTENSION_PACKAGE_PATTERNS.some((pattern) => pattern.test(normalized));
}

export function validateExtensionPackageInventory({ files, manifest, expectedExtensionId }) {
  if (!Array.isArray(files) || files.length === 0) {
    throw new Error("extension package inventory must list at least one file");
  }
  const normalizedFiles = [];
  const seen = new Set();
  for (const file of files) {
    const normalized = normalizeExtensionPackagePath(file);
    if (isExcludedExtensionPackagePath(normalized)) {
      throw new Error(`extension package must not contain ${normalized}`);
    }
    if (seen.has(normalized)) {
      throw new Error(`duplicate extension package file: ${normalized}`);
    }
    seen.add(normalized);
    normalizedFiles.push(normalized);
  }
  for (const required of EXTENSION_PACKAGE_REQUIRED_FILES) {
    if (!seen.has(required)) {
      throw new Error(`extension package is missing required file: ${required}`);
    }
  }
  if (manifest === null || typeof manifest !== "object") {
    throw new Error("extension package requires the Extension manifest object");
  }
  if (manifest.manifest_version !== 3) {
    throw new Error("extension package manifest_version must be 3");
  }
  assertNonEmptyString(manifest.version, "extension version");
  if (expectedExtensionId !== undefined) {
    assertNonEmptyString(expectedExtensionId, "expected Extension ID");
    assertNonEmptyString(manifest.key, "extension manifest public key");
  }
  return normalizedFiles.sort();
}


function validateRelativeLicensePath(value) {
  assertNonEmptyString(value, "license file");
  if (value.startsWith("/") || value.startsWith("\\") || /^[a-zA-Z]:/.test(value)) {
    throw new Error(`license file must be a relative path, got: ${value}`);
  }
  if (value.split(/[\\/]/).includes("..")) {
    throw new Error(`license file must not escape its directory, got: ${value}`);
  }
  return value;
}

export function validateReleaseManifest(manifest) {
  if (manifest === null || typeof manifest !== "object" || Array.isArray(manifest)) {
    throw new Error("release manifest must be an object");
  }
  if (manifest.schema_version !== RELEASE_SCHEMA_VERSION) {
    throw new Error(`release manifest schema_version must be ${RELEASE_SCHEMA_VERSION}`);
  }
  validateReleaseTag(manifest.tag);
  if (manifest.platform !== RELEASE_PLATFORM) {
    throw new Error(`release manifest platform must be ${RELEASE_PLATFORM}`);
  }
  assertNonEmptyString(manifest.catalog_version, "catalog_version");

  if (!Array.isArray(manifest.assets) || manifest.assets.length === 0) {
    throw new Error("release manifest must list at least one asset");
  }
  const seenNames = new Set();
  const seenKinds = new Set();
  for (const asset of manifest.assets) {
    if (asset === null || typeof asset !== "object") {
      throw new Error("release manifest asset must be an object");
    }
    const parsed = parseReleaseAssetName(asset.name);
    if (parsed.tag !== manifest.tag) {
      throw new Error(`asset ${asset.name} does not match release tag ${manifest.tag}`);
    }
    if (seenNames.has(asset.name)) {
      throw new Error(`duplicate release asset: ${asset.name}`);
    }
    seenNames.add(asset.name);
    seenKinds.add(parsed.kind);
    if (typeof asset.sha256 !== "string" || !HEX64_PATTERN.test(asset.sha256)) {
      throw new Error(`asset ${asset.name} must carry a 64-character hex SHA-256`);
    }
    if (!Number.isInteger(asset.size_bytes) || asset.size_bytes <= 0) {
      throw new Error(`asset ${asset.name} must carry a positive integer size_bytes`);
    }
  }
  if (!seenKinds.has("executable") || !seenKinds.has("archive")) {
    throw new Error("release manifest must contain both the executable and the archive asset");
  }
  const missingKinds = REQUIRED_RELEASE_ASSET_KINDS.filter((kind) => !seenKinds.has(kind));
  if (missingKinds.length > 0) {
    throw new Error(
      `release manifest is missing required release assets: ${missingKinds.join(", ")}`,
    );
  }

  if (!Array.isArray(manifest.licenses) || manifest.licenses.length === 0) {
    throw new Error("release manifest must list at least one license entry");
  }
  for (const entry of manifest.licenses) {
    if (entry === null || typeof entry !== "object") {
      throw new Error("release manifest license entry must be an object");
    }
    assertNonEmptyString(entry.component, "license component");
    validateRelativeLicensePath(entry.file);
  }
  return manifest;
}
