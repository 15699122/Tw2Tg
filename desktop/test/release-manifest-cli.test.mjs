import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { buildReleaseManifest, runReleaseManifestCommand } from "../scripts/release-manifest-cli.mjs";

const tag = "v0.2.0-pre.7";
const names = {
  executable: `XArchive-${tag}-windows-x64.exe`,
  archive: `XArchive-${tag}-windows-x64.7z`,
  repository_dependencies: `XArchive-${tag}-windows-x64-repository-dependencies.7z`,
  full: `XArchive-${tag}-windows-x64-full.7z`,
  extension: `XArchive-${tag}-extension.zip`,
};

async function withFixture(callback) {
  const directory = await mkdtemp(join(tmpdir(), "xarchive-release-manifest-"));
  try {
    return await callback(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

async function createAssets(directory) {
  const paths = {};
  for (const [kind, name] of Object.entries(names)) {
    const path = join(directory, name);
    await writeFile(path, `${kind}\n`);
    paths[kind] = path;
  }
  return paths;
}

test("builds a validated manifest and deterministic SHA256SUMS content", async () => {
  await withFixture(async (directory) => {
    const assetPaths = await createAssets(directory);
    const manifest = await buildReleaseManifest({
      tag,
      assetPaths,
      sourceSha: "a".repeat(40),
      extensionId: "iaajefkoanbkleojofoadeakelihbjne",
    });
    assert.equal(manifest.assets.length, 5);
    assert.equal(manifest.source_sha, "a".repeat(40));
    assert.equal(manifest.extension_id, "iaajefkoanbkleojofoadeakelihbjne");
    assert.equal(manifest.assets[0].name, names.extension);
  });
});

test("CLI writes JSON manifest and SHA256SUMS only after validating all assets", async () => {
  await withFixture(async (directory) => {
    const assetPaths = await createAssets(directory);
    const output = join(directory, "manifest.json");
    const sumsOutput = join(directory, "SHA256SUMS");
    await runReleaseManifestCommand([
      "--tag", tag,
      "--executable", assetPaths.executable,
      "--archive", assetPaths.archive,
      "--repository-dependencies", assetPaths.repository_dependencies,
      "--full", assetPaths.full,
      "--extension", assetPaths.extension,
      "--source-sha", "b".repeat(40),
      "--output", output,
      "--sha256sums", sumsOutput,
    ]);
    const manifest = JSON.parse(await readFile(output, "utf8"));
    const sums = await readFile(sumsOutput, "utf8");
    assert.equal(manifest.assets.length, 5);
    assert.match(sums, new RegExp(`  ${names.extension.replaceAll(".", "\\.")}\\n`));
    assert.equal(sums.split("\n").filter(Boolean).length, 5);
  });
});

test("rejects an asset whose file name does not match the release tag", async () => {
  await withFixture(async (directory) => {
    const assetPaths = await createAssets(directory);
    const wrongPath = join(directory, "wrong-extension.zip");
    await writeFile(wrongPath, "wrong\n");
    await assert.rejects(
      buildReleaseManifest({ tag, assetPaths: { ...assetPaths, extension: wrongPath } }),
      /must end with XArchive-v0\.2\.0-pre\.7-extension\.zip/,
    );
  });
});