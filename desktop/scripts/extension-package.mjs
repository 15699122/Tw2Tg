import { readFile, readdir, writeFile } from "node:fs/promises";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { assertExtensionIdentityParity } from "./extension-identity.mjs";
import {
  isExcludedExtensionPackagePath,
  normalizeExtensionPackagePath,
  releaseAssetNames,
  validateExtensionPackageInventory,
  validateReleaseTag,
} from "./release-assets.mjs";

export const EXTENSION_PACKAGE_SCHEMA_VERSION = 1;

/// Collect the loadable Extension file inventory, skipping dev-only paths.
export async function collectExtensionPackageFiles(extensionDirectory) {
  const root = resolve(extensionDirectory);
  const files = [];
  async function walk(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    for (const entry of entries) {
      const absolute = join(directory, entry.name);
      const relativePath = normalizeExtensionPackagePath(
        relative(root, absolute).split(sep).join("/"),
      );
      if (isExcludedExtensionPackagePath(relativePath)) {
        continue;
      }
      if (entry.isDirectory()) {
        await walk(absolute);
        continue;
      }
      if (!entry.isFile()) {
        throw new Error(`extension package contains an unsupported entry: ${relativePath}`);
      }
      files.push(relativePath);
    }
  }
  await walk(root);
  return files.sort();
}

export async function readExtensionManifestFile(extensionDirectory) {
  const manifestPath = join(resolve(extensionDirectory), "manifest.json");
  return JSON.parse(await readFile(manifestPath, "utf8"));
}

export async function buildExtensionPackageMetadata({ extensionDirectory, extensionId, releaseTag }) {
  validateReleaseTag(releaseTag);
  const manifest = await readExtensionManifestFile(extensionDirectory);
  const derivedExtensionId = assertExtensionIdentityParity({
    manifestKey: manifest.key,
    expectedExtensionId: extensionId,
  });
  const files = validateExtensionPackageInventory({
    files: await collectExtensionPackageFiles(extensionDirectory),
    manifest,
    expectedExtensionId: extensionId,
  });
  return {
    schema_version: EXTENSION_PACKAGE_SCHEMA_VERSION,
    release_tag: releaseTag,
    asset: releaseAssetNames(releaseTag).extension,
    extension_id: derivedExtensionId,
    extension_version: manifest.version,
    file_count: files.length,
    files,
  };
}

export function validateExtensionPackageMetadata(metadata) {
  if (metadata === null || typeof metadata !== "object") {
    throw new Error("extension package metadata must be an object");
  }
  if (metadata.schema_version !== EXTENSION_PACKAGE_SCHEMA_VERSION) {
    throw new Error(
      `extension package metadata schema_version must be ${EXTENSION_PACKAGE_SCHEMA_VERSION}`,
    );
  }
  validateReleaseTag(metadata.release_tag);
  if (metadata.asset !== releaseAssetNames(metadata.release_tag).extension) {
    throw new Error("extension package metadata asset name does not match its release tag");
  }
  validateExtensionPackageInventory({
    files: metadata.files,
    manifest: { manifest_version: 3, version: metadata.extension_version },
  });
  return metadata;
}

function parseArguments(argv) {
  const [command, ...rest] = argv;
  const options = {};
  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index];
    if (!token.startsWith("--")) {
      throw new Error(`Unexpected argument: ${token}`);
    }
    const value = rest[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`Missing value for ${token}`);
    }
    options[token.slice(2)] = value;
    index += 1;
  }
  return { command, options };
}

function requireOption(options, name) {
  const value = options[name];
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`Missing required option --${name}`);
  }
  return value;
}

export async function runExtensionPackageCommand(argv) {
  const { command, options } = parseArguments(argv);
  const extensionDirectory = resolve(options["extension-dir"] ?? "extension");

  if (command === "plan") {
    const metadata = await buildExtensionPackageMetadata({
      extensionDirectory,
      extensionId: requireOption(options, "extension-id"),
      releaseTag: requireOption(options, "tag"),
    });
    const output = resolve(requireOption(options, "output"));
    await writeFile(output, `${JSON.stringify(metadata, null, 2)}\n`);
    return {
      metadata,
      message: `Extension package plan: ${metadata.file_count} files for ${metadata.asset}\n  output: ${output}\n`,
    };
  }

  if (command === "verify") {
    const metadata = validateExtensionPackageMetadata(
      JSON.parse(await readFile(resolve(requireOption(options, "metadata")), "utf8")),
    );
    const manifest = await readExtensionManifestFile(extensionDirectory);
    const actual = validateExtensionPackageInventory({
      files: await collectExtensionPackageFiles(extensionDirectory),
      manifest,
      expectedExtensionId: metadata.extension_id,
    });
    const missing = metadata.files.filter((file) => !actual.includes(file));
    const unexpected = actual.filter((file) => !metadata.files.includes(file));
    if (missing.length > 0 || unexpected.length > 0) {
      throw new Error(
        `extension package inventory mismatch (missing: ${missing.join(", ") || "none"}; unexpected: ${unexpected.join(", ") || "none"})`,
      );
    }
    return {
      metadata,
      message: `Extension package verified: ${actual.length} files match ${metadata.asset}\n`,
    };
  }

  throw new Error(`Unknown command: ${command ?? "(none)"} (expected plan or verify)`);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  runExtensionPackageCommand(process.argv.slice(2))
    .then(({ message }) => {
      process.stdout.write(message);
    })
    .catch((error) => {
      process.stderr.write(`extension-package: ${error.message}\n`);
      process.exitCode = 1;
    });
}
