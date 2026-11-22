import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  OFFLINE_BUNDLE_CATALOG_FILE,
  OFFLINE_COMPONENT_IDS,
  assembleOfflineBundle,
  createEmbeddedCatalog,
  createOfflineBundleManifest,
  describeOfflineBundleComponent,
  treeDigest,
  validateBundlePath,
  validateOfflineBundleManifest,
  verifyOfflineBundle,
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

async function syntheticRelease(root) {
  for (const [index, id] of [...OFFLINE_COMPONENT_IDS].entries()) {
    await mkdir(join(root, id, "payload"), { recursive: true });
    await writeFile(join(root, id, "payload", "artifact.bin"), Buffer.from(`artifact-${id}-${index}`));
    await writeFile(join(root, id, "LICENSE.txt"), `license-${id}`);
  }
  await writeFile(join(root, "release-manifest.json"), JSON.stringify({ release: "v0.2.0-pre.2" }));
  return [...OFFLINE_COMPONENT_IDS].map((id, index) => ({
    id,
    version: `1.0.${index}`,
    artifact: id,
    required_files: ["payload/artifact.bin"],
    license_files: ["LICENSE.txt"],
  }));
}

test("tree digest matches the components.rs ordering and framing", async () => {
  const root = await mkdtemp(join(tmpdir(), "tw2tg-tree-digest-"));
  try {
    await mkdir(join(root, "nested"));
    await writeFile(join(root, "b.bin"), Buffer.from([2]));
    await writeFile(join(root, "a.bin"), Buffer.from([1]));
    await writeFile(join(root, "nested", "c.bin"), Buffer.from([3, 3]));
    const digest = await treeDigest(root);
    // Independent re-derivation of the same contract: relative path, NUL,
    // little-endian u64 length, bytes, in UTF-8 path order.
    const expected = createHash("sha256");
    for (const [name, bytes] of [
      [Buffer.from("a.bin"), Buffer.from([1])],
      [Buffer.from("b.bin"), Buffer.from([2])],
      [Buffer.from("nested/c.bin"), Buffer.from([3, 3])],
    ]) {
      const length = Buffer.alloc(8);
      length.writeBigUInt64LE(BigInt(bytes.length));
      expected.update(Buffer.concat([name, Buffer.from([0]), length, bytes]));
    }
    assert.equal(digest.sha256, expected.digest("hex"));
    assert.equal(digest.size_bytes, 4);
    assert.deepEqual(digest.files, ["a.bin", "b.bin", "nested/c.bin"]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("component descriptions derive the hash and must resolve inside the artifact", async () => {
  const root = await mkdtemp(join(tmpdir(), "tw2tg-component-"));
  try {
    await mkdir(join(root, "worker", "payload"), { recursive: true });
    await writeFile(join(root, "worker", "payload", "worker.exe"), "executable");
    await writeFile(join(root, "worker", "LICENSE.txt"), "license");
    const described = await describeOfflineBundleComponent(root, {
      id: "worker",
      version: "1.0.0",
      artifact: "worker",
      required_files: ["payload/worker.exe"],
      license_files: ["LICENSE.txt"],
    });
    assert.match(described.sha256, /^[0-9a-f]{64}$/);
    assert.equal(described.size_bytes, "executable".length + "license".length);
    await assert.rejects(
      describeOfflineBundleComponent(root, {
        id: "worker",
        version: "1.0.0",
        artifact: "worker",
        required_files: ["payload/missing.exe"],
        license_files: ["LICENSE.txt"],
      }),
      /does not contain/
    );
    await assert.rejects(
      describeOfflineBundleComponent(root, {
        id: "unknown",
        version: "1.0.0",
        artifact: "worker",
        required_files: ["payload/worker.exe"],
        license_files: ["LICENSE.txt"],
      }),
      /unsupported/
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test("assembles an offline bundle whose manifest, catalog and bytes agree", async () => {
  const source = await mkdtemp(join(tmpdir(), "tw2tg-bundle-source-"));
  const parent = await mkdtemp(join(tmpdir(), "tw2tg-bundle-output-"));
  const output = join(parent, "bundle");
  try {
    const components = await syntheticRelease(source);
    const { manifest, catalog } = await assembleOfflineBundle({
      sourceRoot: source,
      outputRoot: output,
      releaseTag: "v0.2.0-pre.2",
      catalogVersion: "2026-11-22-u13",
      components,
    });
    assert.equal(manifest.package_type, "offline-bundle");
    assert.equal(manifest.components.length, OFFLINE_COMPONENT_IDS.size);
    const written = JSON.parse(await readFile(join(output, OFFLINE_BUNDLE_CATALOG_FILE), "utf8"));
    assert.deepEqual(written, catalog);
    for (const component of manifest.components) {
      assert.equal(component.sha256.length, 64);
      const entry = catalog.components.find((candidate) => candidate.id === component.id);
      assert.equal(entry.platform, "windows");
      assert.equal(entry.architecture, "x64");
      assert.equal(entry.max_size_bytes, component.size_bytes);
      assert.equal(entry.sha256, component.sha256);
      assert.equal(entry.required_files[0], component.required_files[0]);
    }
    await verifyOfflineBundle({ root: output, manifest });
    // One changed byte is rejected: the catalog describes the bytes on disk.
    await writeFile(join(output, manifest.components[0].artifact, "payload", "artifact.bin"), "tampered");
    await assert.rejects(verifyOfflineBundle({ root: output, manifest }), /does not match its manifest/);
  } finally {
    await rm(source, { recursive: true, force: true });
    await rm(parent, { recursive: true, force: true });
  }
});

test("assembly refuses an incomplete set, an existing output root and overlapping artifacts", async () => {
  const source = await mkdtemp(join(tmpdir(), "tw2tg-bundle-refuse-"));
  const parent = await mkdtemp(join(tmpdir(), "tw2tg-bundle-refuse-output-"));
  try {
    const components = await syntheticRelease(source);
    const common = { sourceRoot: source, releaseTag: "v0.2.0-pre.2", catalogVersion: "2026-11-22-u13" };
    await assert.rejects(
      assembleOfflineBundle({ ...common, outputRoot: join(parent, "incomplete"), components: components.slice(1) }),
      /exactly the required/
    );
    await mkdir(join(parent, "existing"));
    await assert.rejects(
      assembleOfflineBundle({ ...common, outputRoot: join(parent, "existing"), components }),
      /EEXIST/
    );
    const overlapping = components.map((component, index) =>
      index === 0 ? { ...component, artifact: "worker" } : component
    );
    await assert.rejects(
      assembleOfflineBundle({ ...common, outputRoot: join(parent, "overlap"), components: overlapping }),
      /must not overlap/
    );
  } finally {
    await rm(source, { recursive: true, force: true });
    await rm(parent, { recursive: true, force: true });
  }
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
