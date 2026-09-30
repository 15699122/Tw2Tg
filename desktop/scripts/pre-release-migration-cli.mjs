//! Migration planner for the pre-release series renumbering.
//!
//! The planner is deliberately the only place that decides *what* happens to
//! each frozen asset. It reads the frozen ledger, validates that the source is
//! a retired `-pre.N` release and the target is a publishable `-preN` tag, and
//! emits an explicit per-asset plan. The Windows migration workflow only
//! executes that plan, so no naming or policy decision is duplicated in YAML.
//!
//! Actions:
//! - `copy`: the source file is republished byte-identically under the new name
//!   (nothing inside it refers to the old tag).
//! - `rebuild_metadata`: archive payload files are preserved byte-for-byte while
//!   the tag-bearing metadata inside the archive is regenerated.
//! - `rearchive_container`: an Extension ZIP is extracted and re-archived as a
//!   real 7z container with identical contents.
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  assertPublishableReleaseTag,
  isLegacyPreReleaseTag,
  parseReleaseAssetName,
  releaseAssetNames,
} from "./release-assets.mjs";

export const MIGRATION_SCHEMA_VERSION = 1;
export const MIGRATION_ACTIONS = ["copy", "rebuild_metadata", "rearchive_container"];
export const MIGRATION_MODES = ["repackage", "artifact"];

/// Kinds whose payload files are preserved and whose tag metadata is rebuilt.
const METADATA_REBUILD_KINDS = new Set(["repository_dependencies", "full"]);
/// Kinds republished byte-identically under a new name.
const BYTE_COPY_KINDS = new Set(["executable", "archive"]);

export function readMigrationLedger(json) {
  if (json === null || typeof json !== "object" || Array.isArray(json)) {
    throw new Error("migration ledger must be an object");
  }
  if (json.schema_version !== MIGRATION_SCHEMA_VERSION) {
    throw new Error(`migration ledger schema_version must be ${MIGRATION_SCHEMA_VERSION}`);
  }
  if (!Array.isArray(json.entries) || json.entries.length === 0) {
    throw new Error("migration ledger must contain at least one entry");
  }
  return json;
}

function entryAssets(entry) {
  const assets = entry.expected_assets;
  if (!Array.isArray(assets) || assets.length === 0) {
    throw new Error(`ledger entry ${entry.source_tag} must freeze at least one asset`);
  }
  return assets.map((asset) => {
    if (typeof asset.name !== "string" || !Number.isInteger(asset.size_bytes) || asset.size_bytes <= 0) {
      throw new Error(`ledger asset ${asset?.name} must carry a positive integer size_bytes`);
    }
    if (typeof asset.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(asset.sha256)) {
      throw new Error(`ledger asset ${asset.name} must carry a 64-character hex SHA-256`);
    }
    return asset;
  });
}

function classifyEra(release, auxiliary, sourceTag) {
  const kinds = new Set(release.map((asset) => asset.kind));
  const carriedNames = new Set([...release.map((asset) => asset.name), ...auxiliary]);
  const hasContract =
    carriedNames.has(`XArchive-${sourceTag}-release-manifest.json`) &&
    carriedNames.has(`SHA256SUMS-${sourceTag}.txt`);
  if (kinds.size === 2 && kinds.has("executable") && kinds.has("archive")) {
    return { era: "two-asset", regenerateReleaseManifest: false };
  }
  if (
    kinds.size === 4 &&
    kinds.has("executable") &&
    kinds.has("archive") &&
    kinds.has("repository_dependencies") &&
    kinds.has("full")
  ) {
    return { era: "four-asset", regenerateReleaseManifest: hasContract };
  }
  if (kinds.size === 5 && kinds.has("extension")) {
    const extension = release.find((asset) => asset.kind === "extension");
    const container = extension.name.endsWith(".zip") ? "extension-zip" : "extension-7z";
    return { era: `five-asset-${container}`, regenerateReleaseManifest: hasContract };
  }
  throw new Error(
    `unsupported frozen asset set for ${sourceTag}: ${[...carriedNames].sort().join(", ")}`,
  );
}

function tryParseAssetName(name) {
  try {
    return parseReleaseAssetName(name);
  } catch {
    return null;
  }
}

/// The v0.2.0-pre.7 era shipped a real ZIP Extension package, which the current
/// contract no longer names. It is still an Extension release asset and must be
/// migrated (re-archived) rather than treated as an auxiliary file.
const LEGACY_EXTENSION_ZIP_PATTERN = /^XArchive-(v\d+\.\d+\.\d+(?:-pre\.?\d+)?)-extension\.zip$/;

function splitLedgerAssets(assets, sourceTag) {
  const release = [];
  const auxiliary = [];
  for (const asset of assets) {
    const parsed = tryParseAssetName(asset.name);
    if (parsed === null) {
      const legacyZip = LEGACY_EXTENSION_ZIP_PATTERN.exec(asset.name);
      if (legacyZip) {
        if (legacyZip[1] !== sourceTag) {
          throw new Error(`ledger asset ${asset.name} does not belong to ${sourceTag}`);
        }
        release.push({ ...asset, kind: "extension" });
        continue;
      }
      auxiliary.push(asset.name);
      continue;
    }
    if (parsed.tag !== sourceTag) {
      throw new Error(`ledger asset ${asset.name} does not belong to ${sourceTag}`);
    }
    release.push({ ...asset, kind: parsed.kind });
  }
  return { release, auxiliary };
}

export function planMigration(ledger, targetTag) {
  assertPublishableReleaseTag(targetTag);
  const matches = ledger.entries.filter((entry) => entry.target_tag === targetTag);
  if (matches.length !== 1) {
    throw new Error(`ledger must contain exactly one entry for ${targetTag}, found ${matches.length}`);
  }
  const entry = matches[0];
  if (!isLegacyPreReleaseTag(entry.source_tag)) {
    throw new Error(`migration source ${entry.source_tag} must be a retired -pre.N tag`);
  }
  if (typeof entry.source_sha !== "string" || !/^[0-9a-f]{40}$/.test(entry.source_sha)) {
    throw new Error(`ledger entry ${entry.source_tag} must freeze the tag commit SHA`);
  }
  if (!MIGRATION_MODES.includes(entry.mode)) {
    throw new Error(`ledger entry ${entry.source_tag} has unsupported mode ${entry.mode}`);
  }

  const assets = entryAssets(entry);
  const { release, auxiliary } = splitLedgerAssets(assets, entry.source_tag);
  const targetNames = releaseAssetNames(targetTag);

  // What the source material is: either the old Release itself, or the isolated
  // verified package set that was built from the frozen commit.
  let sourceRef;
  let sourceExpectations;
  let eraRelease;
  let eraAuxiliary;
  let eraTag = entry.source_tag;
  if (entry.mode === "repackage") {
    sourceRef = { type: "release", tag: entry.source_tag, release_id: entry.source_release_id };
    sourceExpectations = assets.map((asset) => ({
      name: asset.name,
      size_bytes: asset.size_bytes,
      sha256: asset.sha256,
    }));
    eraRelease = release;
    eraAuxiliary = auxiliary;
  } else {
    const artifact = entry.artifact;
    if (artifact === null || typeof artifact !== "object" || Array.isArray(artifact)) {
      throw new Error(`artifact-mode source ${entry.source_tag} must describe its artifact`);
    }
    if (!Number.isInteger(artifact.run_id) || artifact.run_id <= 0) {
      throw new Error(`artifact ${artifact.name} must record a numeric run_id`);
    }
    for (const field of ["name", "package_dir", "tag"]) {
      if (typeof artifact[field] !== "string" || artifact[field].trim() === "") {
        throw new Error(`artifact of ${entry.source_tag} must record ${field}`);
      }
    }
    if (artifact.tag !== entry.source_tag) {
      throw new Error(
        `artifact tag ${artifact.tag} must equal the frozen source tag ${entry.source_tag}`,
      );
    }
    if (
      typeof artifact.expected_executable_sha256 !== "string" ||
      !/^[0-9a-f]{64}$/.test(artifact.expected_executable_sha256)
    ) {
      throw new Error(`artifact ${artifact.name} must pin the executable SHA-256`);
    }
    eraTag = artifact.tag;
    const artifactNames = releaseAssetNames(eraTag);
    eraRelease = [
      artifactNames.executable,
      artifactNames.archive,
      artifactNames.repository_dependencies,
      artifactNames.full,
      artifactNames.extension,
    ].map((name) => ({ name, kind: parseReleaseAssetName(name).kind }));
    eraAuxiliary = [`XArchive-${eraTag}-release-manifest.json`, `SHA256SUMS-${eraTag}.txt`];
    sourceRef = {
      type: "artifact",
      run_id: artifact.run_id,
      name: artifact.name,
      package_dir: artifact.package_dir,
      tag: artifact.tag,
      expected_executable_sha256: artifact.expected_executable_sha256,
    };
    // The build bundle also carries its own release-manifest and SHA256SUMS
    // checksum files. They are pinned source expectations too (the workflow
    // cross-checks their contents and the executable hash), but they are NOT
    // republishing inputs: they carry the frozen source tag, so they stay in
    // eraAuxiliary for era classification only.
    sourceExpectations = eraRelease
      .map((asset) => ({
        name: asset.name,
        size_bytes: null,
        sha256: asset.kind === "executable" ? artifact.expected_executable_sha256 : null,
      }))
      .concat(eraAuxiliary.map((name) => ({ name, size_bytes: null, sha256: null })));
  }

  const { era, regenerateReleaseManifest } = classifyEra(eraRelease, eraAuxiliary, eraTag);
  const planned = eraRelease
    .map((asset) => {
      const targetName =
        asset.kind === "extension" ? targetNames.extension : targetNames[asset.kind];
      if (!targetName) {
        throw new Error(`target contract has no asset for kind ${asset.kind}`);
      }
      if (BYTE_COPY_KINDS.has(asset.kind)) {
        return { kind: asset.kind, source_name: asset.name, target_name: targetName, action: "copy" };
      }
      if (METADATA_REBUILD_KINDS.has(asset.kind)) {
        return {
          kind: asset.kind,
          source_name: asset.name,
          target_name: targetName,
          action: "rebuild_metadata",
        };
      }
      if (asset.kind === "extension") {
        return {
          kind: asset.kind,
          source_name: asset.name,
          target_name: targetName,
          action: asset.name.endsWith(".zip") ? "rearchive_container" : "copy",
        };
      }
      throw new Error(`no migration action for ${asset.name}`);
    })
    .sort((left, right) => left.kind.localeCompare(right.kind));

  for (const action of planned.map((asset) => asset.action)) {
    if (!MIGRATION_ACTIONS.includes(action)) {
      throw new Error(`unknown migration action ${action}`);
    }
  }
  return {
    schema_version: MIGRATION_SCHEMA_VERSION,
    source_tag: entry.source_tag,
    target_tag: targetTag,
    source_sha: entry.source_sha,
    mode: entry.mode,
    era,
    source_ref: sourceRef,
    source_expectations: sourceExpectations,
    frozen_release_assets: assets,
    regenerate_release_manifest: regenerateReleaseManifest,
    carried_findings: entry.carried_findings ?? [],
    assets: planned,
  };
}

function parseArguments(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) {
      throw new Error(`Unexpected argument: ${token}`);
    }
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`Missing value for ${token}`);
    }
    options[token.slice(2)] = value;
    index += 1;
  }
  return options;
}

function requireOption(options, name) {
  const value = options[name];
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`Missing required option --${name}`);
  }
  return value;
}

export async function runPreReleaseMigrationCommand(argv) {
  const options = parseArguments(argv);
  const ledgerPath = resolve(requireOption(options, "ledger"));
  const targetTag = requireOption(options, "target");
  const ledger = readMigrationLedger(JSON.parse(await readFile(ledgerPath, "utf8")));
  const plan = planMigration(ledger, targetTag);
  if (options.output) {
    await writeFile(resolve(options.output), `${JSON.stringify(plan, null, 2)}\n`);
  }
  return {
    plan,
    message: `migration plan: ${plan.source_tag} -> ${plan.target_tag} (${plan.mode}, ${plan.era}) with ${plan.assets.length} target assets\n`,
  };
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  runPreReleaseMigrationCommand(process.argv.slice(2))
    .then(({ message }) => process.stdout.write(message))
    .catch((error) => {
      process.stderr.write(`pre-release-migration: ${error.message}\n`);
      process.exitCode = 1;
    });
}

