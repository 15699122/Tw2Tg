import test from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { componentPlan, createManifest, packageDirectories, validatePackageType } from "../scripts/portable-package.mjs";

test("portable package type accepts only full and core", () => {
  assert.equal(validatePackageType("full"), "full");
  assert.equal(validatePackageType("core"), "core");
  assert.throws(() => validatePackageType("lite"), /must be full or core/);
});

test("Full package includes gallery-dl directory while Core does not", () => {
  assert.ok(packageDirectories("full").includes("sidecar/gallery-dl"));
  assert.ok(!packageDirectories("core").includes("sidecar/gallery-dl"));
  assert.ok(packageDirectories("core").includes("sidecar/xarchive-downloader"));
  assert.ok(packageDirectories("full").includes("native-host"));
  assert.ok(!packageDirectories("core").includes("native-host"));
});

test("component plan requires worker, keeps aria2 optional, bundles gallery-dl only in Full", () => {
  const full = componentPlan("/project", "/output", "full");
  const core = componentPlan("/project", "/output", "core");
  assert.equal(full.find(([source]) => source.endsWith("xarchive-downloader"))[2], "required");
  assert.equal(full.find(([source]) => source.endsWith("aria2"))[2], "optional");
  assert.equal(full.find(([source]) => source.endsWith("gallery-dl"))[2], "required");
  assert.equal(core.find(([source]) => source.endsWith("xarchive-downloader"))[2], "required");
  assert.equal(core.find(([source]) => source.endsWith("aria2"))[2], "optional");
  assert.equal(core.find(([source]) => source.endsWith("gallery-dl"))[2], "excluded");
});

test("component plan excludes gallery-dl from Core even if source directory exists", () => {
  const core = componentPlan("/project", "/output", "core");
  const entry = core.find(([source]) => source.endsWith("gallery-dl"));
  assert.equal(entry[2], "excluded");
  // Build script must skip excluded components regardless of source existence.
  const galleryDlSuffix = join("sidecar", "gallery-dl");
  assert.equal(entry[0].endsWith(galleryDlSuffix), true);
  assert.equal(entry[1].endsWith(galleryDlSuffix), true);
});

test("manifest explicitly describes Full/Core component boundaries", () => {
  const full = createManifest("full");
  const core = createManifest("core");
  assert.equal(full.sidecar.gallery_dl, "sidecar/gallery-dl/gallery-dl.exe");
  assert.equal(full.extension.bundled, true);
  assert.equal(core.sidecar.gallery_dl, null);
  assert.equal(core.sidecar.gallery_dl_bundled, false);
  assert.equal(core.extension.bundled, false);
  assert.equal(core.extension.user_importable, false);
  assert.equal(full.native_host, null);
});