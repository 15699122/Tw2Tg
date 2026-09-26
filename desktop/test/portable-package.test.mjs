import test from "node:test";
import assert from "node:assert/strict";
import { join, parse, resolve, sep } from "node:path";
import {
  componentPlan,
  createManifest,
  filterPackageFiles,
  packageDirectories,
  validatePackageType,
  validatePortableOutputDir,
} from "../scripts/portable-package.mjs";

test("portable package type accepts only full and core", () => {
  assert.equal(validatePackageType("full"), "full");
  assert.equal(validatePackageType("core"), "core");
  assert.throws(() => validatePackageType("lite"), /must be full or core/);
});

test("Full package includes gallery-dl directory while Core does not", () => {
  assert.ok(packageDirectories("full").includes("sidecar/gallery-dl"));
  assert.ok(!packageDirectories("core").includes("sidecar/gallery-dl"));
  assert.ok(packageDirectories("core").includes("sidecar/xarchive-downloader"));
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

// ENG-01: the packaging script removes the output directory recursively, so a
// misconfigured PORTABLE_OUTPUT_DIR must fail before any delete happens.
const projectRoot = resolve(sep, "workspace", "Tw2Tg");
const deepProjectRoot = resolve(sep, "data", "builds", "agent", "Tw2Tg");
const homeDir = resolve(sep, "home", "operator");

test("portable output accepts the default dist-portable namespace", () => {
  assert.equal(
    validatePortableOutputDir("dist-portable/XArchive", { projectRoot, homeDir }),
    resolve(projectRoot, "dist-portable", "XArchive"),
  );
  assert.equal(
    validatePortableOutputDir(join("dist-portable", "nested", "pkg"), { projectRoot, homeDir }),
    resolve(projectRoot, "dist-portable", "nested", "pkg"),
  );
});

test("portable output refuses the project root and its ancestors", () => {
  assert.throws(() => validatePortableOutputDir(".", { projectRoot, homeDir }), /project root/);
  assert.throws(
    () => validatePortableOutputDir(projectRoot, { projectRoot, homeDir }),
    /project root/,
  );
  // `deepProjectRoot` has four segments, so `..` stays a real ancestor
  // directory instead of collapsing onto the filesystem root.
  assert.throws(
    () => validatePortableOutputDir(join("..", ".."), { projectRoot: deepProjectRoot, homeDir }),
    /ancestor of the project root/,
  );
  assert.throws(
    () => validatePortableOutputDir(join("..", "..", ".."), { projectRoot: deepProjectRoot, homeDir }),
    /ancestor of the project root/,
  );
});

test("portable output refuses a filesystem root", () => {
  assert.throws(
    () => validatePortableOutputDir(parse(projectRoot).root, { projectRoot, homeDir }),
    /filesystem root/,
  );
});

test("portable output refuses the user home directory and its ancestors", () => {
  assert.throws(
    () => validatePortableOutputDir(homeDir, { projectRoot, homeDir }),
    /user home directory/,
  );
  assert.throws(
    () => validatePortableOutputDir(resolve(sep, "home"), { projectRoot, homeDir }),
    /user home directory/,
  );
});

test("project-relative portable output must stay inside the packaging namespace", () => {
  assert.throws(() => validatePortableOutputDir("build", { projectRoot, homeDir }), /dist-portable/);
  assert.throws(
    () => validatePortableOutputDir(join("desktop", "dist"), { projectRoot, homeDir }),
    /dist-portable/,
  );
});

test("portable output rejects empty values and requires a project root", () => {
  assert.throws(() => validatePortableOutputDir("   ", { projectRoot, homeDir }), /non-empty/);
  assert.throws(() => validatePortableOutputDir(undefined, { projectRoot, homeDir }), /non-empty/);
  assert.throws(() => validatePortableOutputDir("dist-portable/XArchive", {}), /requires a projectRoot/);
});

test("portable output allows an explicit absolute directory outside protected roots", () => {
  const external = resolve(sep, "srv", "xarchive-packages", "XArchive");
  assert.equal(validatePortableOutputDir(external, { projectRoot, homeDir }), external);
});

  assert.equal(core.extension.user_importable, false);
});
// ENG-09: a recursive copy does not honour .gitignore, so local secrets,
// databases, logs, and caches must be filtered out explicitly.
test("package file filter drops local secrets, databases, logs, and caches", () => {
  const files = [
    "manifest.json",
    "xarchive-desktop.exe",
    ".env",
    ".env.local",
    "config/archive.sqlite3",
    "config/archive.sqlite3-wal",
    "logs/xarchive-1.log",
    "__pycache__/module.cpython-312.pyc",
    "nested/__pycache__/module.cpython-312.pyc",
    "sidecar/worker.pyc",
    ".pytest_cache/CACHEDIR.TAG",
    "node_modules/pkg/index.js",
    "target/release/artifact.bin",
    "test-artifacts/session.png",
    ".DS_Store",
    "Thumbs.db",
  ];
  const kept = filterPackageFiles(files);
  assert.deepEqual(kept, ["manifest.json", "xarchive-desktop.exe"]);
});

test("package file filter keeps legitimate nested component files", () => {
  const files = [
    "manifest.json",
    "src/content.js",
    "src/nested/deep/value.json",
    "sidecar/xarchive-downloader/xarchive-downloader.exe",
    "sidecar/gallery-dl/gallery-dl.exe",
  ];
  assert.deepEqual(filterPackageFiles(files), files);
});
