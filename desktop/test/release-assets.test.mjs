import test from "node:test";
import assert from "node:assert/strict";
import {
  assertPublishableReleaseTag,
  isLegacyPreReleaseTag,
  parseReleaseAssetName,
  releaseAssetNames,
  validateReleaseManifest,
  validateReleaseTag,
} from "../scripts/release-assets.mjs";

const tag = "v0.2.0-pre6";

function manifest(releaseTag = tag) {
  const names = releaseAssetNames(releaseTag);
  return {
    schema_version: 1,
    tag,
    platform: "windows-x64",
    catalog_version: "2026-09-20-u11",
    assets: [
      { name: names.executable, sha256: "9e".repeat(32), size_bytes: 1024 },
      { name: names.archive, sha256: "ab".repeat(32), size_bytes: 2048 },
      { name: names.repository_dependencies, sha256: "cd".repeat(32), size_bytes: 3072 },
      { name: names.full, sha256: "ef".repeat(32), size_bytes: 4096 },
      { name: names.extension, sha256: "12".repeat(32), size_bytes: 512 },
    ],
    licenses: [{ component: "XArchive", file: "THIRD_PARTY_NOTICES.md" }],
  };
}

test("release tags and asset names stay versioned and Windows-scoped", () => {
  assert.equal(validateReleaseTag(tag), tag);
  assert.equal(assertPublishableReleaseTag(tag), tag);
  assert.throws(() => validateReleaseTag("latest"), /release tag/);
  assert.deepEqual(releaseAssetNames(tag), {
    executable: "XArchive-v0.2.0-pre6-windows-x64.exe",
    archive: "XArchive-v0.2.0-pre6-windows-x64.7z",
    repository_dependencies:
      "XArchive-v0.2.0-pre6-windows-x64-repository-dependencies.7z",
    full: "XArchive-v0.2.0-pre6-windows-x64-full.7z",
    extension: "XArchive-v0.2.0-pre6-extension.7z",
  });
  assert.deepEqual(parseReleaseAssetName("XArchive-v0.2.0-pre6-windows-x64.exe"), {
    tag,
    kind: "executable",
  });
  assert.deepEqual(parseReleaseAssetName("XArchive-v0.2.0-pre6-windows-x64.7z"), {
    tag,
    kind: "archive",
  });
  assert.deepEqual(
    parseReleaseAssetName("XArchive-v0.2.0-pre6-windows-x64-repository-dependencies.7z"),
    { tag, kind: "repository_dependencies" },
  );
  assert.deepEqual(parseReleaseAssetName("XArchive-v0.2.0-pre6-windows-x64-full.7z"), {
    tag,
    kind: "full",
  });
  assert.deepEqual(parseReleaseAssetName("XArchive-v0.2.0-pre6-extension.7z"), {
    tag,
    kind: "extension",
  });
  // v0.2.0 ships every archive as a real 7z container, so the historical ZIP
  // name is no longer part of the contract and must not be accepted.
  assert.throws(
    () => parseReleaseAssetName("XArchive-v0.2.0-pre6-extension.zip"),
    /not a versioned/,
  );
  assert.throws(() => parseReleaseAssetName("XArchive-latest-windows-x64.exe"), /not a versioned/);
});

test("legacy -pre.N numbering stays readable and never publishable", () => {
  assert.equal(validateReleaseTag("v0.1.1-pre.4"), "v0.1.1-pre.4");
  assert.equal(isLegacyPreReleaseTag("v0.1.1-pre.4"), true);
  assert.equal(isLegacyPreReleaseTag(tag), false);
  assert.equal(isLegacyPreReleaseTag("v0.2.0"), false);
  assert.throws(() => assertPublishableReleaseTag("v0.1.1-pre.4"), /read-only/);
  // Frozen evidence keeps parsing so migration ledgers can name old assets.
  assert.deepEqual(parseReleaseAssetName("XArchive-v0.1.1-pre.4-windows-x64.exe"), {
    tag: "v0.1.1-pre.4",
    kind: "executable",
  });
});

test("release manifest requires versioned assets, hashes, sizes, and licenses", () => {
  assert.equal(validateReleaseManifest(manifest()).tag, tag);
  assert.throws(() => validateReleaseManifest({ ...manifest(), tag: "latest" }), /release tag/);
  assert.throws(
    () =>
      validateReleaseManifest({
        ...manifest(),
        assets: [manifest().assets[0]],
      }),
    /both the executable and the archive/,
  );
  assert.throws(
    () =>
      validateReleaseManifest({
        ...manifest(),
        assets: manifest().assets.map((asset) => ({ ...asset, sha256: "zz" })),
      }),
    /SHA-256/,
  );
  assert.throws(
    () => validateReleaseManifest({ ...manifest(), licenses: [] }),
    /at least one license/,
  );
  assert.throws(
    () =>
      validateReleaseManifest({
        ...manifest(),
        licenses: [{ component: "XArchive", file: "../escape.txt" }],
      }),
    /must not escape/,
  );
});
