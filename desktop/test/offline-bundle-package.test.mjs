import test from "node:test";
import assert from "node:assert/strict";
import {
  OFFLINE_COMPONENT_IDS,
  createOfflineBundleManifest,
  validateBundlePath,
  validateOfflineBundleManifest,
} from "../scripts/offline-bundle-package.mjs";

const components = [...OFFLINE_COMPONENT_IDS].map((id, index) => ({
  id,
  version: `1.0.${index}`,
  artifact: `${id}/artifact.bin`,
  sha256: "ab".repeat(32),
  size_bytes: 100 + index,
  required_files: [`${id}/artifact.bin`],
  license_files: [`${id}/LICENSE.txt`],
}));

test("offline bundle paths are relative and cannot escape the root", () => {
  assert.equal(validateBundlePath("sidecar/worker.exe"), "sidecar/worker.exe");
  assert.throws(() => validateBundlePath("../worker.exe"), /escape/);
  assert.throws(() => validateBundlePath("C:\\worker.exe"), /relative/);
  assert.throws(() => validateBundlePath("/worker.exe"), /relative/);
});

test("creates an offline bundle manifest with complete component parity", () => {
  const manifest = createOfflineBundleManifest({
    releaseTag: "v0.2.0-pre.2",
    catalogVersion: "2026-09-20-u13",
    components,
  });
  assert.equal(manifest.package_type, "offline-bundle");
  assert.equal(manifest.platform, "windows-x64");
  assert.equal(manifest.components.length, 6);
  assert.deepEqual(manifest.parity, {
    release_manifest: "release-manifest.json",
    catalog_version: "2026-09-20-u13",
  });
});

test("rejects incomplete, duplicated, or non-parity bundle manifests", () => {
  const manifest = createOfflineBundleManifest({
    releaseTag: "v0.2.0-pre.2",
    catalogVersion: "2026-09-20-u13",
    components,
  });
  assert.throws(() => validateOfflineBundleManifest({ ...manifest, components: components.slice(1) }), /exactly the required/);
  assert.throws(() => validateOfflineBundleManifest({ ...manifest, components: [components[0], ...components.slice(0, 5)] }), /duplicate/);
  assert.throws(() => validateOfflineBundleManifest({ ...manifest, parity: { release_manifest: "other.json", catalog_version: manifest.catalog_version } }), /parity/);
  assert.throws(() => validateOfflineBundleManifest({ ...manifest, runtime_directories: ["config"] }), /runtime directory/);
});
