import test from "node:test";
import assert from "node:assert/strict";
import {
  parseReleaseAssetName,
  releaseAssetNames,
  validateReleaseManifest,
  validateReleaseTag,
} from "../scripts/release-assets.mjs";

function manifest(tag = "v0.1.1-pre.4") {
  return {
    schema_version: 1,
    tag,
    platform: "windows-x64",
    catalog_version: "2026-09-20-u11",
    assets: [
      {
        name: `XArchive-${tag}-windows-x64.exe`,
        sha256: "9e".repeat(32),
        size_bytes: 1024,
      },
      {
        name: `XArchive-${tag}-windows-x64.7z`,
        sha256: "ab".repeat(32),
        size_bytes: 2048,
      },
    ],
    licenses: [{ component: "XArchive", file: "THIRD_PARTY_NOTICES.md" }],
  };
}

test("release tags and asset names stay versioned and Windows-scoped", () => {
  assert.equal(validateReleaseTag("v0.1.1-pre.4"), "v0.1.1-pre.4");
  assert.throws(() => validateReleaseTag("latest"), /release tag/);
  assert.deepEqual(releaseAssetNames("v0.1.1-pre.4"), {
    executable: "XArchive-v0.1.1-pre.4-windows-x64.exe",
    archive: "XArchive-v0.1.1-pre.4-windows-x64.7z",
  });
  assert.deepEqual(parseReleaseAssetName("XArchive-v0.1.1-pre.4-windows-x64.exe"), {
    tag: "v0.1.1-pre.4",
    kind: "executable",
  });
  assert.deepEqual(parseReleaseAssetName("XArchive-v0.1.1-pre.4-windows-x64.7z"), {
    tag: "v0.1.1-pre.4",
    kind: "archive",
  });
  assert.throws(() => parseReleaseAssetName("XArchive-latest-windows-x64.exe"), /not a versioned/);
});

test("release manifest requires versioned assets, hashes, sizes, and licenses", () => {
  assert.equal(validateReleaseManifest(manifest()).tag, "v0.1.1-pre.4");
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
