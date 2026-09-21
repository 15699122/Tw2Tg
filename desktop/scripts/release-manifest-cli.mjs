import { createHash } from "node:crypto";
import { readFile, stat, writeFile } from "node:fs/promises";
import { basename, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  RELEASE_PLATFORM,
  releaseAssetNames,
  validateReleaseManifest,
  validateReleaseTag,
} from "./release-assets.mjs";

const ASSET_OPTIONS = [
  ["executable", "executable"],
  ["archive", "archive"],
  ["repository-dependencies", "repository_dependencies"],
  ["full", "full"],
  ["extension", "extension"],
];

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

async function hashFile(path) {
  const contents = await readFile(path);
  return {
    sha256: createHash("sha256").update(contents).digest("hex"),
    size_bytes: (await stat(path)).size,
  };
}

function buildSha256Sums(manifest) {
  return `${manifest.assets
    .map((asset) => `${asset.sha256}  ${asset.name}`)
    .join("\n")}\n`;
}

export async function buildReleaseManifest({
  tag,
  assetPaths,
  sourceSha,
  catalogVersion = "unreleased",
  extensionId,
}) {
  validateReleaseTag(tag);
  const names = releaseAssetNames(tag);
  const assets = [];

  for (const [kind, path] of Object.entries(assetPaths)) {
    if (!names[kind]) {
      throw new Error(`unknown release asset kind: ${kind}`);
    }
    const resolvedPath = resolve(path);
    const file = await hashFile(resolvedPath);
    const expectedName = names[kind];
    if (basename(resolvedPath) !== expectedName) {
      throw new Error(
        `asset path for ${kind} must end with ${expectedName}, got ${basename(resolvedPath)}`,
      );
    }
    assets.push({ name: expectedName, ...file });
  }

  assets.sort((left, right) => left.name.localeCompare(right.name));
  const manifest = {
    schema_version: 1,
    tag,
    platform: RELEASE_PLATFORM,
    catalog_version: catalogVersion,
    ...(sourceSha ? { source_sha: sourceSha } : {}),
    ...(extensionId ? { extension_id: extensionId } : {}),
    assets,
    licenses: [
      { component: "XArchive", file: "LICENSE" },
      { component: "Third-party dependencies", file: "THIRD_PARTY_NOTICES.md" },
    ],
  };
  validateReleaseManifest(manifest);
  return manifest;
}

export async function runReleaseManifestCommand(argv) {
  const options = parseArguments(argv);
  const tag = requireOption(options, "tag");
  const assetPaths = {};
  for (const [optionName, kind] of ASSET_OPTIONS) {
    assetPaths[kind] = requireOption(options, optionName);
  }

  const manifest = await buildReleaseManifest({
    tag,
    assetPaths,
    sourceSha: options["source-sha"],
    catalogVersion: options["catalog-version"] ?? "unreleased",
    extensionId: options["extension-id"],
  });
  const output = resolve(requireOption(options, "output"));
  const sumsOutput = resolve(requireOption(options, "sha256sums"));
  await writeFile(output, `${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(sumsOutput, buildSha256Sums(manifest));
  return {
    manifest,
    output,
    sumsOutput,
    message: `Release manifest verified: ${manifest.assets.length} assets for ${manifest.tag}\n`,
  };
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  runReleaseManifestCommand(process.argv.slice(2))
    .then(({ message }) => process.stdout.write(message))
    .catch((error) => {
      process.stderr.write(`release-manifest: ${error.message}\n`);
      process.exitCode = 1;
    });
}