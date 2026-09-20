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
  };
}

export function parseReleaseAssetName(name) {
  assertNonEmptyString(name, "asset name");
  const match = /^XArchive-(v\d+\.\d+\.\d+(?:-pre\.\d+)?)-windows-x64\.(exe|7z)$/.exec(name);
  if (!match) {
    throw new Error(`release asset name is not a versioned Windows x64 asset, got: ${name}`);
  }
  return { tag: match[1], kind: match[2] === "exe" ? "executable" : "archive" };
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
